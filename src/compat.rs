//! `qbc --compatibility` — a dialect-fidelity audit.
//!
//! Estimates how well a `.bas` file, AS WRITTEN, would load unmodified in
//! three real DOS-era interpreters (QBasic 1.1, QuickBASIC 4.5, GW-BASIC).
//! This is deliberately NOT about what qbc itself accepts — qbc already
//! accepts a broad superset of all three dialects. Every rule here is
//! grounded in a specific, documented incident from this project's own
//! history of hand-porting `basic-src/` programs to run under real DOS
//! QBasic (see CLAUDE.md's "QB1.1 DOS compatibility" changelog sections).
//!
//! Standalone analysis mode: only needs the raw source bytes, the token
//! stream, and the parsed `Program` AST — never the analyzer or emitter,
//! so the result can never depend on qbc's own (more permissive) internals.

use crate::lexer::{Spanned, Token};
use crate::parser::{Expr, LValue, Program, RetTySyntax, Stmt, VarDecl};

/// Real QBasic/QuickBASIC reserved words and built-in function names that
/// are legal identifiers as far as qbc's lexer is concerned (none of these
/// are qbc keywords — confirmed against `lexer::keyword()`) but collide
/// with a real interpreter's own reserved namespace. Grounded in the
/// `pos`/INVADERS.BAS and `fNum`/DEF FN-prefix incidents (CLAUDE.md
/// Common Pitfalls #14). `STEP` is deliberately excluded: it IS a qbc
/// lexer keyword, so a program using it as an identifier fails to parse
/// before `--compatibility` ever runs — the collision is structurally
/// unreachable here, not something this rule needs to catch.
const RESERVED_WORDS: &[&str] = &[
    "POS", "TIMER", "NAME", "SEEK", "FLUSH", "LOCK", "UNLOCK",
    "MKDIR", "RMDIR", "CHDIR", "KILL",
];

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StmtStatus { Supported, Advisory, Flagged }

#[derive(Debug, Clone)]
pub struct TargetScore {
    pub name: &'static str,
    pub score: f64,
    pub violations: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct CompatReport {
    /// Statement kinds actually present in the file, in first-seen order.
    pub statements: Vec<(&'static str, StmtStatus)>,
    /// Rule-8 advisories (hardware/timing-dependent constructs) — never
    /// scored against any target, just surfaced.
    pub advisories: Vec<String>,
    /// Fixed order: QBasic 1.1, QuickBASIC 4.5, GW-BASIC.
    pub targets: Vec<TargetScore>,
    /// Best-fit target name (highest score; ties broken QB1.1 > QB4.5 >
    /// GW-BASIC).
    pub detected: &'static str,
    pub detected_score: f64,
}

/// Points deducted per violation *instance* (not per rule), floored at 0.
/// A flat, simple, per-instance scheme — chosen so a handful of issues is
/// visible in the score without one violation being catastrophic.
const POINTS_PER_VIOLATION: f64 = 3.0;

pub fn audit(source: &str, raw_bytes: &[u8], tokens: &[Spanned], ast: &Program) -> CompatReport {
    let mut statements = StmtTable::new();
    let mut advisories: Vec<String> = Vec::new();

    // Applicability: (QB1.1, QB4.5, GW-BASIC) violation lists, built up by
    // each rule below and folded into scores at the end.
    let mut qb11: Vec<String> = Vec::new();
    let mut qb45: Vec<String> = Vec::new();
    let mut gwbasic: Vec<String> = Vec::new();

    // ── Statement-kind presence (drives the Statements table) ──────────────
    walk_program(ast, &mut |s| statements.note(s));

    // ── Rule 1: underscore inside an identifier ─────────────────────────────
    // QB1.1: illegal. QB4.5/GW-BASIC: legal. Dedup by lowercase name — a
    // variable referenced many times is one thing to rename, not many.
    {
        let mut seen = std::collections::HashSet::new();
        for sp in tokens {
            if let Some(name) = ident_text(&sp.token) {
                if name.contains('_') && seen.insert(name.to_lowercase()) {
                    qb11.push(format!("Underscore in identifier `{name}` (illegal in QB1.1)"));
                }
            }
        }
    }

    // ── Rule 2: reserved-word / FN-prefix identifier collision ─────────────
    // All three dialects: these are real interpreter built-ins/reserved
    // words everywhere, even though every documented incident happened to
    // surface under QB1.1.
    {
        let mut names: Vec<String> = Vec::new();
        collect_var_names(ast, &mut names);
        let mut seen = std::collections::HashSet::new();
        for name in names {
            let upper = name.to_uppercase();
            let is_reserved = RESERVED_WORDS.contains(&upper.as_str());
            let is_fn_prefix = upper.starts_with("FN") && upper.len() > 2;
            if (is_reserved || is_fn_prefix) && seen.insert(upper.clone()) {
                let msg = format!("Variable name `{name}` collides with a reserved word/builtin");
                qb11.push(msg.clone());
                qb45.push(msg.clone());
                gwbasic.push(msg);
            }
        }
    }

    // ── Rule 3: FUNCTION Foo() AS <type> instead of sigil form ──────────────
    // QB1.1: illegal (sigil required). QB4.5: legal. GW-BASIC: N/A (no
    // FUNCTION at all — excluded, not counted either way).
    for f in &ast.functions {
        if f.ret_ty_written == RetTySyntax::AsClause {
            qb11.push(format!(
                "FUNCTION {}() declares its return type with AS (requires sigil form in QB1.1)",
                f.name
            ));
            statements.flag("FUNCTION");
        }
    }

    // ── Rule 4: `_` end-of-line continuation ────────────────────────────────
    // QB4.5-only feature. QB1.1/GW-BASIC: illegal. The lexer fully consumes
    // this construct (zero tokens emitted), so it's only visible in the raw
    // source — a per-physical-line scan for a standalone trailing `_`.
    for (i, line) in source.lines().enumerate() {
        if line_ends_with_bare_underscore(line) {
            let n = i + 1;
            qb11.push(format!("Line {n}: `_` end-of-line continuation (QB4.5-only)"));
            gwbasic.push(format!("Line {n}: `_` end-of-line continuation (QB4.5-only)"));
        }
    }

    // ── Rule 5: ON ERROR GOTO targets a label inside a SUB/FUNCTION ─────────
    // QB1.1: illegal ("Label not defined" — ON ERROR GOTO can only target a
    // module-level label). QB4.5: legal (per-procedure local error
    // trapping). GW-BASIC: N/A (no SUB/FUNCTION at all).
    {
        let mut local_labels = std::collections::HashSet::new();
        for s in &ast.subs {
            walk_stmts(&s.body, &mut |st| {
                if let Stmt::Label(n) = st { local_labels.insert(n.to_lowercase()); }
            });
        }
        for f in &ast.functions {
            walk_stmts(&f.body, &mut |st| {
                if let Stmt::Label(n) = st { local_labels.insert(n.to_lowercase()); }
            });
        }
        walk_stmts(&ast.main_body, &mut |st| {
            if let Stmt::OnError { label } = st {
                if label != "0" && local_labels.contains(&label.to_lowercase()) {
                    qb11.push(format!(
                        "ON ERROR GOTO {label} targets a label defined inside a SUB/FUNCTION"
                    ));
                    statements.flag("ON ERROR");
                }
            }
        });
    }

    // ── Rule 6: DIM inside a GOSUB-target routine that can re-execute ───────
    // All three: real interpreters raise "Duplicate definition" on re-entry.
    // Matches the farkle.bas fix — flags ANY Dim found inside a GOSUB
    // routine's statement run, regardless of provable call-count, since
    // that's exactly the scope of the historical fix.
    {
        let gosub_targets: std::collections::HashSet<String> = {
            let mut set = std::collections::HashSet::new();
            walk_stmts(&ast.main_body, &mut |st| {
                if let Stmt::Gosub(label) = st { set.insert(label.to_lowercase()); }
            });
            set
        };
        let mut in_gosub_routine = false;
        for st in &ast.main_body {
            match st {
                Stmt::Label(name) => {
                    in_gosub_routine = gosub_targets.contains(&name.to_lowercase());
                }
                Stmt::Return => in_gosub_routine = false,
                _ if in_gosub_routine => {
                    walk_stmts(std::slice::from_ref(st), &mut |inner| {
                        if let Stmt::Dim(d) = inner {
                            let msg = format!(
                                "DIM {} sits inside a GOSUB routine (re-entry raises \"Duplicate definition\")",
                                d.name
                            );
                            qb11.push(msg.clone());
                            qb45.push(msg.clone());
                            gwbasic.push(msg);
                            statements.flag("DIM");
                        }
                    });
                }
                _ => {}
            }
        }
    }

    // ── Rule 7: LF-only line endings (file-level, at most one instance) ────
    // All three: real DOS interpreters require CRLF.
    if raw_bytes.windows(2).any(|w| w == [b'\r', b'\n']) {
        // Has at least some CRLF pairs — likely already DOS-encoded; still
        // check for stray bare LFs (a partially-converted file).
        if has_bare_lf(raw_bytes) {
            let msg = "File mixes bare LF and CRLF line endings (DOS requires CRLF throughout)".to_string();
            qb11.push(msg.clone()); qb45.push(msg.clone()); gwbasic.push(msg);
        }
    } else if raw_bytes.contains(&b'\n') {
        let msg = "File uses LF-only line endings (DOS QBasic/GW-BASIC require CRLF)".to_string();
        qb11.push(msg.clone()); qb45.push(msg.clone()); gwbasic.push(msg);
    }

    // ── Rule 8: hardware/timing-dependent constructs (advisory only) ───────
    walk_stmts(&ast.main_body, &mut |st| note_advisory(st, &mut advisories, &mut statements));
    for s in &ast.subs { walk_stmts(&s.body, &mut |st| note_advisory(st, &mut advisories, &mut statements)); }
    for f in &ast.functions { walk_stmts(&f.body, &mut |st| note_advisory(st, &mut advisories, &mut statements)); }
    advisories.sort();
    advisories.dedup();

    let targets = vec![
        score_target("QBasic 1.1", qb11),
        score_target("QuickBASIC 4.5", qb45),
        score_target("GW-BASIC", gwbasic),
    ];

    let (detected, detected_score) = best_fit(&targets);

    CompatReport {
        statements: statements.into_ordered(),
        advisories,
        targets,
        detected,
        detected_score,
    }
}

fn score_target(name: &'static str, violations: Vec<String>) -> TargetScore {
    let score = (100.0 - POINTS_PER_VIOLATION * violations.len() as f64).max(0.0);
    TargetScore { name, score, violations }
}

/// Best-fit heuristic: highest score wins; ties broken QB1.1 > QB4.5 >
/// GW-BASIC (targets is always given in that fixed order, so a stable
/// max-by-index scan naturally implements the tiebreak).
fn best_fit(targets: &[TargetScore]) -> (&'static str, f64) {
    let mut best = &targets[0];
    for t in &targets[1..] {
        if t.score > best.score { best = t; }
    }
    (best.name, best.score)
}

// ── Statement-kind table ─────────────────────────────────────────────────────

struct StmtTable {
    order: Vec<&'static str>,
    status: std::collections::HashMap<&'static str, StmtStatus>,
}

impl StmtTable {
    fn new() -> Self { Self { order: Vec::new(), status: std::collections::HashMap::new() } }

    fn note(&mut self, s: &Stmt) {
        if let Some(name) = stmt_name(s) {
            self.status.entry(name).or_insert_with(|| { self.order.push(name); StmtStatus::Supported });
        }
    }

    fn flag(&mut self, name: &'static str) {
        self.status.entry(name).and_modify(|v| *v = StmtStatus::Flagged)
            .or_insert_with(|| { self.order.push(name); StmtStatus::Flagged });
    }

    fn advise(&mut self, name: &'static str) {
        self.status.entry(name).and_modify(|v| if *v == StmtStatus::Supported { *v = StmtStatus::Advisory })
            .or_insert_with(|| { self.order.push(name); StmtStatus::Advisory });
    }

    fn into_ordered(self) -> Vec<(&'static str, StmtStatus)> {
        self.order.into_iter().map(|n| (n, self.status[n])).collect()
    }
}

fn stmt_name(s: &Stmt) -> Option<&'static str> {
    Some(match s {
        Stmt::Print { .. } => "PRINT",
        Stmt::PrintUsing { .. } | Stmt::PrintFileUsing { .. } => "PRINT USING",
        Stmt::Input { .. } | Stmt::InputFile { .. } => "INPUT",
        Stmt::Line { .. } => "LINE",
        Stmt::Circle { .. } => "CIRCLE",
        Stmt::Play(_) => "PLAY",
        Stmt::Call { name, .. } if name.eq_ignore_ascii_case("chain") => "CHAIN",
        Stmt::SharedDecl(_) => "SHARED",
        Stmt::Dim(_) | Stmt::ReDim(_) => "DIM",
        Stmt::OnError { .. } => "ON ERROR",
        Stmt::Wait { .. } => "WAIT",
        Stmt::Out { .. } => "OUT",
        Stmt::DefSeg(_) => "DEF SEG",
        Stmt::PaletteUsing(_) => "PALETTE USING",
        Stmt::Poke { .. } => "POKE",
        _ => return None,
    })
}

fn note_advisory(s: &Stmt, out: &mut Vec<String>, statements: &mut StmtTable) {
    match s {
        Stmt::Wait { port, .. } => {
            out.push(format!("Uses WAIT {}", port_text(port)));
            statements.advise("WAIT");
        }
        Stmt::Out { port, .. } => {
            out.push(format!("Uses OUT port {}", port_text(port)));
            statements.advise("OUT");
        }
        Stmt::DefSeg(_) => {
            out.push("Uses DEF SEG (direct memory access)".to_string());
            statements.advise("DEF SEG");
        }
        Stmt::Poke { .. } => {
            out.push("Uses POKE (direct memory access)".to_string());
            statements.advise("POKE");
        }
        Stmt::PaletteUsing(_) => {
            out.push("Uses PALETTE USING (bulk palette remap — undocumented on some real hardware)".to_string());
            statements.advise("PALETTE USING");
        }
        _ => {}
    }
}

fn port_text(e: &Expr) -> String {
    match e {
        Expr::IntLit(n) => format!("&H{n:X}"),
        _ => "<computed port>".to_string(),
    }
}

// ── AST walkers ───────────────────────────────────────────────────────────────

fn walk_program(prog: &Program, f: &mut dyn FnMut(&Stmt)) {
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

fn collect_var_names(prog: &Program, out: &mut Vec<String>) {
    fn from_decl(d: &VarDecl, out: &mut Vec<String>) { out.push(d.name.clone()); }
    fn from_params(params: &[VarDecl], out: &mut Vec<String>) {
        for p in params { from_decl(p, out); }
    }
    fn from_lvalue(lv: &LValue, out: &mut Vec<String>) {
        match lv {
            LValue::Scalar { name, .. } | LValue::Index { name, .. } => out.push(name.clone()),
            LValue::Field { base, .. } | LValue::FieldIndex { base, .. } => from_lvalue(base, out),
        }
    }

    walk_program(prog, &mut |s| match s {
        Stmt::Dim(d) | Stmt::ReDim(d) => from_decl(d, out),
        Stmt::Let { var, .. } => from_lvalue(var, out),
        _ => {}
    });
    for s in &prog.subs { from_params(&s.params, out); }
    for func in &prog.functions { from_params(&func.params, out); }
}

fn ident_text(t: &Token) -> Option<&str> {
    match t {
        Token::Ident(s) | Token::IdentStr(s) | Token::IdentInt(s) |
        Token::IdentSng(s) | Token::IdentDbl(s) => Some(s),
        _ => None,
    }
}

/// True if `line`, after trimming trailing whitespace/CR, ends with a
/// standalone `_` "word" — i.e. the character before it (if any) is not
/// part of an identifier, matching the lexer's own `word == "_"` check for
/// the QB4.5 continuation construct (`src/lexer.rs`). Deliberately does
/// NOT try to detect whether the trailing `_` sits inside an open string
/// literal on the same line — a rare enough edge case that a false
/// positive here is an acceptable, documented approximation for an
/// advisory-adjacent audit tool.
fn line_ends_with_bare_underscore(line: &str) -> bool {
    let trimmed = line.trim_end_matches(['\r', ' ', '\t']);
    if !trimmed.ends_with('_') { return false; }
    match trimmed.as_bytes().get(trimmed.len().wrapping_sub(2)) {
        None => true,
        Some(c) => !(c.is_ascii_alphanumeric() || *c == b'_'),
    }
}

fn has_bare_lf(raw_bytes: &[u8]) -> bool {
    raw_bytes.iter().enumerate().any(|(i, &b)| {
        b == b'\n' && (i == 0 || raw_bytes[i - 1] != b'\r')
    })
}

// ── Report rendering ─────────────────────────────────────────────────────────

pub fn render(r: &CompatReport) -> String {
    let mut out = String::new();
    out.push_str("QBasic Compatibility Report\n");
    out.push_str("===========================\n");
    out.push_str(&format!("Dialect detected: {}\n\n", r.detected));

    out.push_str("Statements\n");
    out.push_str("----------\n");
    if r.statements.is_empty() {
        out.push_str("(none)\n");
    } else {
        for (name, status) in &r.statements {
            let label = match status {
                StmtStatus::Supported => "Supported",
                StmtStatus::Advisory  => "Supported (see potential issues)",
                StmtStatus::Flagged   => "Flagged (see potential issues)",
            };
            out.push_str(&format!("{name:<12}{label}\n"));
        }
    }
    out.push('\n');

    out.push_str("Potential issues\n");
    out.push_str("----------------\n");
    if r.advisories.is_empty() {
        out.push_str("(none)\n");
    } else {
        for a in &r.advisories { out.push_str(&format!("\u{26A0} {a}\n")); }
    }
    out.push('\n');

    out.push_str(&format!("Compatibility score: {:.1}%\n", r.detected_score));
    out.push_str("Target:\n");
    for t in &r.targets {
        out.push_str(&format!("  {:<16} {:.1}%\n", t.name, t.score));
        for v in &t.violations { out.push_str(&format!("    - {v}\n")); }
    }

    out
}

#[cfg(test)]
mod compat_tests {
    use super::*;
    use crate::{lexer, parser};

    fn run(src: &str) -> CompatReport {
        let tokens = lexer::tokenize(src).expect("lex");
        let ast = parser::parse(tokens.clone()).expect("parse");
        audit(src, src.as_bytes(), &tokens, &ast)
    }

    fn score_of<'a>(r: &'a CompatReport, name: &str) -> f64 {
        r.targets.iter().find(|t| t.name == name).unwrap().score
    }

    #[test]
    fn clean_program_scores_100_everywhere() {
        let r = run("PRINT \"HELLO\"\r\nA = 1\r\nPRINT A\r\n");
        assert_eq!(score_of(&r, "QBasic 1.1"), 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
        assert_eq!(score_of(&r, "GW-BASIC"), 100.0);
        assert!(r.advisories.is_empty());
        assert_eq!(r.detected, "QBasic 1.1");
    }

    #[test]
    fn underscore_identifier_docks_qb11_only() {
        let r = run("DIM my_var AS INTEGER\r\nmy_var = 1\r\nPRINT my_var\r\n");
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
        assert_eq!(score_of(&r, "GW-BASIC"), 100.0);
    }

    #[test]
    fn reserved_word_variable_docks_all_three() {
        let r = run("DIM pos AS INTEGER\r\npos = 1\r\nPRINT pos\r\n");
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert!(score_of(&r, "QuickBASIC 4.5") < 100.0);
        assert!(score_of(&r, "GW-BASIC") < 100.0);
    }

    #[test]
    fn function_as_clause_docks_qb11_only() {
        let r = run("FUNCTION Foo () AS INTEGER\r\nFoo = 1\r\nEND FUNCTION\r\nPRINT Foo()\r\n");
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
    }

    #[test]
    fn function_sigil_form_is_clean() {
        let r = run("FUNCTION Foo% ()\r\nFoo = 1\r\nEND FUNCTION\r\nPRINT Foo()\r\n");
        assert_eq!(score_of(&r, "QBasic 1.1"), 100.0);
    }

    #[test]
    fn underscore_continuation_docks_qb11_and_gwbasic_not_qb45() {
        let r = run("PRINT \"A\"; _\r\n\"B\"\r\n");
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
        assert!(score_of(&r, "GW-BASIC") < 100.0);
    }

    #[test]
    fn onerror_goto_sub_local_label_docks_qb11_only() {
        // The violation: a MODULE-level ON ERROR GOTO targeting a label
        // that only exists inside a SUB body (unreachable across procedure
        // boundaries in real QB1.1). A SUB's own local ON ERROR GOTO
        // targeting a label within that SAME SUB is normal and legal —
        // not what this rule flags.
        let r = run(
            "ON ERROR GOTO Handler\r\n\
             CALL Foo\r\n\
             SUB Foo\r\n\
             Handler:\r\n\
             END SUB\r\n"
        );
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
    }

    #[test]
    fn dim_inside_gosub_routine_docks_all_three() {
        let r = run(
            "GOSUB DoIt\r\n\
             END\r\n\
             DoIt:\r\n\
             DIM x AS INTEGER\r\n\
             x = 1\r\n\
             RETURN\r\n"
        );
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert!(score_of(&r, "QuickBASIC 4.5") < 100.0);
        assert!(score_of(&r, "GW-BASIC") < 100.0);
    }

    #[test]
    fn lf_only_line_endings_dock_all_three() {
        let r = run("PRINT \"HI\"\nA = 1\nPRINT A\n");
        assert!(score_of(&r, "QBasic 1.1") < 100.0);
        assert!(score_of(&r, "QuickBASIC 4.5") < 100.0);
        assert!(score_of(&r, "GW-BASIC") < 100.0);
    }

    #[test]
    fn hardware_ports_are_advisory_not_scored() {
        let r = run("WAIT &H3DA, 8\r\nOUT &H3C8, 0\r\n");
        assert_eq!(score_of(&r, "QBasic 1.1"), 100.0);
        assert_eq!(score_of(&r, "QuickBASIC 4.5"), 100.0);
        assert_eq!(score_of(&r, "GW-BASIC"), 100.0);
        assert!(!r.advisories.is_empty());
    }

    #[test]
    fn detected_can_be_quickbasic_45_for_qb45_only_construct() {
        let r = run("PRINT \"A\"; _\r\n\"B\"\r\n");
        assert_eq!(r.detected, "QuickBASIC 4.5");
    }

    #[test]
    fn render_does_not_panic_and_contains_header() {
        let r = run("PRINT \"HELLO\"\r\n");
        let text = render(&r);
        assert!(text.starts_with("QBasic Compatibility Report"));
        assert!(text.contains("Compatibility score"));
    }
}
