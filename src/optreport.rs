//! `qbc --opt-report` — a source-level findings report.
//!
//! Deliberately NOT a classical compiler-optimization report. qbc emits
//! straightforward Rust and hands it to `rustc`/LLVM, which already does
//! constant folding, dead-branch elimination, and the like on every build
//! regardless of whether this report exists — reporting those back would
//! be trivia qbc itself does nothing with, not an actionable lever. What
//! IS genuinely additive is anything about the ORIGINAL `.bas` SOURCE that
//! rustc never sees at all: dead labels, arrays that are declared but
//! never resized, and conditions that are constant in the source itself.
//!
//! Standalone analysis mode, but — unlike `--compatibility` — runs AFTER
//! `analyzer::analyze()`, since it needs the resolved global symbol table
//! and CONST values that only exist on `AnalyzedProgram`.

use std::collections::{HashMap, HashSet};
use crate::analyzer::AnalyzedProgram;
use crate::parser::{BinOp, Expr, LValue, QbType, Stmt, UnOp};

#[derive(Debug, Clone)]
pub struct OptReport {
    /// Shared/global variables → their declared type, sorted by name. Only
    /// covers `DIM SHARED`/promoted globals (what the analyzer's symbol
    /// table tracks) — NOT every local inside every SUB/FUNCTION.
    pub variables: Vec<(String, String)>,
    /// Named labels defined but never targeted by GOTO/GOSUB/ON…GOTO/
    /// ON…GOSUB/ON ERROR GOTO/ON KEY|TIMER GOSUB/RESUME/RESTORE.
    pub unreachable_labels: Vec<String>,
    /// Arrays (DIM'd with at least one dimension) that are never REDIM'd.
    pub never_resized_arrays: Vec<String>,
    /// Total DATA element count.
    pub data_count: usize,
    /// `IF`/`ELSEIF` conditions built entirely from literals/CONSTs, with
    /// the constant truth value they fold to.
    pub constant_branches: Vec<(String, bool)>,
}

pub fn analyze(prog: &AnalyzedProgram) -> OptReport {
    let mut variables: Vec<(String, String)> = prog.global_scope.symbols.values()
        .map(|s| (s.name.clone(), display_type(&s.ty, s.dims)))
        .collect();
    variables.sort();

    let unreachable_labels = find_unreachable_labels(prog);
    let never_resized_arrays = find_never_resized_arrays(prog);
    let data_count = prog.data_store.len();
    let constant_branches = find_constant_branches(prog);

    OptReport { variables, unreachable_labels, never_resized_arrays, data_count, constant_branches }
}

fn display_type(ty: &QbType, dims: usize) -> String {
    let base = match ty {
        QbType::Integer | QbType::Single | QbType::Double => "f64",
        QbType::String => "String",
        QbType::UserType(name) => return format!("{name} (TYPE){}", if dims > 0 { "()" } else { "" }),
    };
    if dims > 0 { format!("{base}()") } else { base.to_string() }
}

// ── Statement walking (self-contained — see compat.rs for the same
//    deliberate non-reuse rationale: keeps each report module independent) ──

fn walk_program(prog: &AnalyzedProgram, f: &mut dyn FnMut(&Stmt)) {
    walk_stmts(&prog.main_body, f);
    for s in &prog.subs { walk_stmts(&s.body, f); }
    for func in &prog.functions { walk_stmts(&func.body, f); }
}

fn walk_stmts(stmts: &[Stmt], f: &mut dyn FnMut(&Stmt)) {
    for s in stmts {
        f(s);
        match s {
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                walk_stmts(then_body, f);
                for (_, body) in elseif_branches { walk_stmts(body, f); }
                if let Some(body) = else_body { walk_stmts(body, f); }
            }
            Stmt::For { body, .. } => walk_stmts(body, f),
            Stmt::While { body, .. } => walk_stmts(body, f),
            Stmt::Do { body, .. } => walk_stmts(body, f),
            Stmt::Select { cases, default, .. } => {
                for c in cases { walk_stmts(&c.body, f); }
                if let Some(body) = default { walk_stmts(body, f); }
            }
            Stmt::Block(body) => walk_stmts(body, f),
            _ => {}
        }
    }
}

// ── Unreachable labels ───────────────────────────────────────────────────────

fn find_unreachable_labels(prog: &AnalyzedProgram) -> Vec<String> {
    let mut defined: Vec<String> = Vec::new();
    let mut used: HashSet<String> = HashSet::new();

    walk_program(prog, &mut |s| match s {
        Stmt::Label(n) => defined.push(n.clone()),
        Stmt::Goto(n) | Stmt::Gosub(n) => { used.insert(n.to_lowercase()); }
        Stmt::OnGoto { labels, .. } => { for l in labels { used.insert(l.to_lowercase()); } }
        Stmt::OnError { label } => { used.insert(label.to_lowercase()); }
        Stmt::OnKeyGosub { target, .. } | Stmt::OnTimerGosub { target, .. } => {
            used.insert(target.to_lowercase());
        }
        Stmt::Resume { label: Some(l), .. } => { used.insert(l.to_lowercase()); }
        Stmt::Restore(Some(l)) => { used.insert(l.to_lowercase()); }
        _ => {}
    });

    // Purely-numeric line-number labels are excluded: in a GOTO-heavy
    // line-numbered program, a label can be reached by ordinary fall-
    // through from the statement above it with no explicit jump anywhere
    // — real reachability there needs control-flow analysis this report
    // doesn't attempt, so flagging them would risk false positives on
    // completely normal line-numbered code. Named labels don't have that
    // fallthrough ambiguity in practice — they exist specifically to be
    // jump targets, so "defined, never targeted" is a reliable signal.
    defined.into_iter()
        .filter(|n| !n.chars().all(|c| c.is_ascii_digit()))
        .filter(|n| !used.contains(&n.to_lowercase()))
        .collect()
}

// ── Never-resized arrays ─────────────────────────────────────────────────────

fn find_never_resized_arrays(prog: &AnalyzedProgram) -> Vec<String> {
    let mut dimmed: Vec<String> = Vec::new();
    let mut redimmed: HashSet<String> = HashSet::new();
    // A name can legally be DIM'd more than once across different SUB/
    // FUNCTION scopes (each local to its own procedure) — dedup by name so
    // the report doesn't repeat the same finding once per occurrence.
    let mut seen: HashSet<String> = HashSet::new();

    walk_program(prog, &mut |s| match s {
        Stmt::Dim(d) if !d.dims.is_empty() => {
            if seen.insert(d.name.to_lowercase()) { dimmed.push(d.name.clone()); }
        }
        Stmt::ReDim(d) => { redimmed.insert(d.name.to_lowercase()); }
        _ => {}
    });

    dimmed.into_iter()
        .filter(|n| !redimmed.contains(&n.to_lowercase()))
        .collect()
}

// ── Constant-condition branches ──────────────────────────────────────────────

fn find_constant_branches(prog: &AnalyzedProgram) -> Vec<(String, bool)> {
    let mut consts: HashMap<String, f64> = prog.consts.iter()
        .map(|(n, v)| (n.clone(), *v)).collect();
    // str_consts don't participate in numeric folding, but their presence
    // in the table would wrongly make a `CONST S$ = "x"` name look
    // "unfoldable" via the wrong branch — no action needed, fold_const_local
    // simply won't find them in `consts` and correctly bails out (None).
    let _ = &prog.str_consts;

    let mut out = Vec::new();
    walk_program(prog, &mut |s| {
        if let Stmt::If { cond, .. } = s {
            if let Some(v) = fold_const_local(cond, &mut consts) {
                out.push((describe_cond(cond), v != 0.0));
            }
        }
    });
    out
}

/// A best-effort QB-like rendering of a condition expression, for display
/// only (not re-parsed, not used for anything semantic).
fn describe_cond(e: &Expr) -> String {
    match e {
        Expr::IntLit(n) => n.to_string(),
        Expr::FloatLit(f) => f.to_string(),
        Expr::StrLit(s) => format!("\"{s}\""),
        Expr::Var(LValue::Scalar { name, .. }) => name.clone(),
        Expr::UnOp { op, operand } => {
            let sym = match op { UnOp::Neg => "-", UnOp::Not => "NOT " };
            format!("{sym}{}", describe_cond(operand))
        }
        Expr::BinOp { op, lhs, rhs } => {
            let sym = match op {
                BinOp::Add => "+", BinOp::Sub => "-", BinOp::Mul => "*", BinOp::Div => "/",
                BinOp::IntDiv => "\\", BinOp::Pow => "^", BinOp::Mod => "MOD",
                BinOp::Eq => "=", BinOp::Ne => "<>", BinOp::Lt => "<", BinOp::Le => "<=",
                BinOp::Gt => ">", BinOp::Ge => ">=", BinOp::And => "AND", BinOp::Or => "OR",
                BinOp::Xor => "XOR", BinOp::Eqv => "EQV", BinOp::Imp => "IMP",
            };
            format!("{} {sym} {}", describe_cond(lhs), describe_cond(rhs))
        }
        _ => "<expr>".to_string(),
    }
}

/// Constant-fold a condition expression using only literals and CONST
/// lookups — returns None the moment any real variable, array, or function
/// call is involved, which is exactly the "not statically constant" case.
/// Broader than the analyzer's own `fold_const` (which only needs to
/// support CONST *declarations* and so skips comparison operators
/// entirely): this one also folds Eq/Ne/Lt/Le/Gt/Ge/Xor/Eqv/Imp, using the
/// same -1.0/0.0 QB boolean convention as the runtime's `qb_from_bool`.
fn fold_const_local(e: &Expr, consts: &mut HashMap<String, f64>) -> Option<f64> {
    match e {
        Expr::IntLit(n) => Some(*n as f64),
        Expr::FloatLit(f) => Some(*f),
        Expr::Var(LValue::Scalar { name, .. }) => consts.get(&name.to_uppercase()).copied(),
        Expr::UnOp { op, operand } => {
            let v = fold_const_local(operand, consts)?;
            Some(match op {
                UnOp::Neg => -v,
                UnOp::Not => if v == 0.0 { -1.0 } else { 0.0 },
            })
        }
        Expr::BinOp { op, lhs, rhs } => {
            let l = fold_const_local(lhs, consts)?;
            let r = fold_const_local(rhs, consts)?;
            let b = |v: bool| if v { -1.0 } else { 0.0 };
            Some(match op {
                BinOp::Add => l + r,
                BinOp::Sub => l - r,
                BinOp::Mul => l * r,
                BinOp::Div => { if r == 0.0 { return None; } l / r }
                BinOp::IntDiv => { if r as i64 == 0 { return None; } (l as i64 / r as i64) as f64 }
                BinOp::Mod => { if r as i64 == 0 { return None; } (l as i64 % r as i64) as f64 }
                BinOp::Pow => l.powf(r),
                BinOp::Eq => b(l == r), BinOp::Ne => b(l != r),
                BinOp::Lt => b(l < r),  BinOp::Le => b(l <= r),
                BinOp::Gt => b(l > r),  BinOp::Ge => b(l >= r),
                BinOp::And => ((l as i64) & (r as i64)) as f64,
                BinOp::Or  => ((l as i64) | (r as i64)) as f64,
                BinOp::Xor => ((l as i64) ^ (r as i64)) as f64,
                BinOp::Eqv => (!((l as i64) ^ (r as i64))) as f64,
                BinOp::Imp => ((!(l as i64)) | (r as i64)) as f64,
            })
        }
        _ => None,
    }
}

// ── Report rendering ─────────────────────────────────────────────────────────

pub fn render(r: &OptReport) -> String {
    let mut out = String::new();
    out.push_str("Optimization Report\n");
    out.push_str("====================\n");
    out.push_str("(source-level findings only — rustc already handles constant\n");
    out.push_str(" folding, dead-branch elimination, etc. at build time)\n\n");

    out.push_str("Variables (shared/global):\n");
    if r.variables.is_empty() {
        out.push_str("  (none)\n");
    } else {
        let width = r.variables.iter().map(|(n, _)| n.len()).max().unwrap_or(0) + 2;
        for (name, ty) in &r.variables {
            out.push_str(&format!("  {name:<width$}{ty}\n"));
        }
    }
    out.push('\n');

    out.push_str("Detected:\n");
    out.push_str(&format!("  {} unreachable label(s)\n", r.unreachable_labels.len()));
    out.push_str(&format!("  {} array(s) never resized\n", r.never_resized_arrays.len()));
    out.push_str(&format!("  {} constant branch condition(s)\n", r.constant_branches.len()));
    out.push_str(&format!("  DATA table: {} constant value(s)\n", r.data_count));
    out.push('\n');

    out.push_str("Findings:\n");
    let mut any = false;
    for l in &r.unreachable_labels {
        out.push_str(&format!("  - Label {l} is never GOTO'd/GOSUB'd/RESTOREd (dead code)\n"));
        any = true;
    }
    for a in &r.never_resized_arrays {
        out.push_str(&format!("  - Array {a}() is DIM'd but never REDIM'd\n"));
        any = true;
    }
    for (cond, val) in &r.constant_branches {
        out.push_str(&format!(
            "  - IF {cond} is always {} (dead branch in source)\n",
            if *val { "TRUE" } else { "FALSE" }
        ));
        any = true;
    }
    if !any { out.push_str("  (none)\n"); }

    out
}

#[cfg(test)]
mod optreport_tests {
    use super::*;

    fn run(src: &str) -> OptReport {
        let tokens = crate::lexer::tokenize(src).expect("lex");
        let ast = crate::parser::parse(tokens).expect("parse");
        let prog = crate::analyzer::analyze(ast).expect("analyze");
        analyze(&prog)
    }

    #[test]
    fn clean_program_has_no_findings() {
        let r = run("DIM SHARED X AS INTEGER\r\nX = 1\r\nPRINT X\r\n");
        assert!(r.unreachable_labels.is_empty());
        assert!(r.never_resized_arrays.is_empty());
        assert!(r.constant_branches.is_empty());
        assert_eq!(r.data_count, 0);
    }

    #[test]
    fn shared_variable_appears_in_table() {
        let r = run("DIM SHARED X AS INTEGER\r\nX = 1\r\nPRINT X\r\n");
        assert!(r.variables.iter().any(|(n, t)| n == "X" && t == "f64"));
    }

    #[test]
    fn unreachable_named_label_is_flagged() {
        let r = run("PRINT 1\r\nEND\r\nDeadCode:\r\nPRINT 2\r\nRETURN\r\n");
        assert_eq!(r.unreachable_labels, vec!["DeadCode".to_string()]);
    }

    #[test]
    fn gosub_target_label_is_not_flagged() {
        let r = run("GOSUB DoIt\r\nEND\r\nDoIt:\r\nPRINT 2\r\nRETURN\r\n");
        assert!(r.unreachable_labels.is_empty());
    }

    #[test]
    fn restore_target_label_is_not_flagged() {
        let r = run(
            "RESTORE Nums\r\nEND\r\nNums:\r\nDATA 1,2,3\r\n"
        );
        assert!(r.unreachable_labels.is_empty());
    }

    #[test]
    fn numeric_labels_are_never_flagged() {
        let r = run("10 PRINT 1\r\n20 PRINT 2\r\n");
        assert!(r.unreachable_labels.is_empty());
    }

    #[test]
    fn never_resized_array_is_flagged() {
        let r = run("DIM A(10) AS INTEGER\r\nA(1) = 1\r\nPRINT A(1)\r\n");
        assert_eq!(r.never_resized_arrays, vec!["A".to_string()]);
    }

    #[test]
    fn same_named_array_in_two_subs_is_reported_once() {
        let r = run(
            "CALL A\r\nCALL B\r\nSUB A\r\nDIM X(5) AS INTEGER\r\nEND SUB\r\nSUB B\r\nDIM X(5) AS INTEGER\r\nEND SUB\r\n"
        );
        assert_eq!(r.never_resized_arrays, vec!["X".to_string()]);
    }

    #[test]
    fn redimmed_array_is_not_flagged() {
        let r = run("DIM A(10) AS INTEGER\r\nREDIM A(20) AS INTEGER\r\nPRINT A(1)\r\n");
        assert!(r.never_resized_arrays.is_empty());
    }

    #[test]
    fn data_count_is_correct() {
        let r = run("DATA 1,2,3,4,5\r\nEND\r\n");
        assert_eq!(r.data_count, 5);
    }

    #[test]
    fn constant_true_branch_is_flagged() {
        let r = run("CONST N = 1\r\nIF N = 1 THEN\r\nPRINT \"always\"\r\nEND IF\r\n");
        assert_eq!(r.constant_branches.len(), 1);
        assert!(r.constant_branches[0].1);
    }

    #[test]
    fn constant_false_branch_is_flagged() {
        let r = run("IF 1 = 2 THEN\r\nPRINT \"never\"\r\nEND IF\r\n");
        assert_eq!(r.constant_branches.len(), 1);
        assert!(!r.constant_branches[0].1);
    }

    #[test]
    fn variable_dependent_branch_is_not_flagged() {
        let r = run("DIM SHARED X AS INTEGER\r\nX = 1\r\nIF X = 1 THEN\r\nPRINT \"maybe\"\r\nEND IF\r\n");
        assert!(r.constant_branches.is_empty());
    }

    #[test]
    fn render_does_not_panic_and_contains_header() {
        let r = run("PRINT \"HELLO\"\r\n");
        let text = render(&r);
        assert!(text.starts_with("Optimization Report"));
        assert!(text.contains("Findings"));
    }
}
