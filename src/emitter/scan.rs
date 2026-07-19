use crate::parser::*;
use crate::analyzer::AnalyzedProgram;
use std::collections::{HashMap, HashSet};
use super::helpers::*;

// ── Local variable collection ─────────────────────────────────────────────────

pub(super) fn collect_locals(stmts: &[Stmt], exclude: &HashSet<String>) -> Vec<(String, QbType)> {
    let mut result: Vec<(String, QbType)> = Vec::new();
    let mut added: HashSet<String> = HashSet::new();

    fn push(name: &str, ty: &QbType,
            result: &mut Vec<(String, QbType)>,
            added:  &mut HashSet<String>,
            exclude: &HashSet<String>) {
        // Use rust_ident_typed so parser-stripped $ sigils get _s appended.
        let rname = rust_ident_typed(name, ty);
        if !exclude.contains(&rname) && !added.contains(&rname) {
            added.insert(rname.clone());
            result.push((rname, ty.clone()));
        }
    }

    /// Recursively scan an expression for scalar variable references.
    fn scan_expr(expr: &Expr,
                 result: &mut Vec<(String, QbType)>,
                 added:  &mut HashSet<String>,
                 exclude: &HashSet<String>) {
        match expr {
            Expr::Var(LValue::Scalar { name, ty }) => {
                push(name, ty, result, added, exclude);
            }
            Expr::Var(LValue::Index { indices, .. }) => {
                for e in indices { scan_expr(e, result, added, exclude); }
            }
            Expr::Var(LValue::Field { base, .. }) |
            Expr::Var(LValue::FieldIndex { base, .. }) => {
                scan_expr(&Expr::Var(*base.clone()), result, added, exclude);
            }
            Expr::BinOp { lhs, rhs, .. } => {
                scan_expr(lhs, result, added, exclude);
                scan_expr(rhs, result, added, exclude);
            }
            Expr::UnOp { operand, .. } => scan_expr(operand, result, added, exclude),
            Expr::Call { args, .. } => {
                for a in args { scan_expr(a, result, added, exclude); }
            }
            Expr::Point { x, y } => {
                scan_expr(x, result, added, exclude);
                scan_expr(y, result, added, exclude);
            }
            Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        }
    }

    /// Pre-mark FOR-loop counter variables as already-added so collect_locals
    /// won't re-declare them (emit_stmt handles their declaration inline).

    fn visit(stmts: &[Stmt], result: &mut Vec<(String, QbType)>,
             added: &mut HashSet<String>, exclude: &HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Let { var, expr } => {
                    match var {
                        LValue::Scalar { name, ty } => {
                            push(name, ty, result, added, exclude);
                        }
                        LValue::Field { base, field } => {
                            // Flattened TYPE field: arr(i).Field → arr__field: Vec<f64>
                            if let LValue::Index { name, .. } = base.as_ref() {
                                let flat = format!("{}__{}", rust_ident(name), field.to_lowercase());
                                if !exclude.contains(&flat) && !added.contains(&flat) {
                                    added.insert(flat.clone());
                                    result.push((flat, QbType::UserType("vec_f64".into())));
                                }
                            }
                        }
                        // The assignment TARGET's index expressions are reads
                        // too — a variable referenced ONLY there (e.g.
                        // `BR(f(L3)) = …` with L3 never assigned) must still
                        // be declared. (Found by the differential fuzzer.)
                        LValue::FieldIndex { indices, .. } => {
                            for e in indices { scan_expr(e, result, added, exclude); }
                        }
                        LValue::Index { indices, .. } => {
                            for e in indices { scan_expr(e, result, added, exclude); }
                        }
                    }
                    scan_expr(expr, result, added, exclude);
                }
                Stmt::Dim(decl) | Stmt::ReDim(decl) => {
                    // Scan dimension expressions (e.g. DIM SHARED LBan&(x) → need x declared)
                    for e in &decl.dims { scan_expr(e, result, added, exclude); }
                    // REDIM local array declarations are emitted inline by emit_redim()
                    // (it tracks its own `redim_declared` set). Don't duplicate here.
                    // But scalar DIM (e.g. from FIELD) must be hoisted as locals.
                    if decl.dims.is_empty() && !decl.shared {
                        push(&decl.name, &decl.ty, result, added, exclude);
                    }
                }
                Stmt::For { var, from, to, step, body } => {
                    // Declare FOR counter at function scope (not inline) so it
                    // remains accessible when GOSUB blocks are inlined.
                    push(var, &QbType::Single, result, added, exclude);
                    scan_expr(from, result, added, exclude);
                    scan_expr(to,   result, added, exclude);
                    if let Some(s) = step { scan_expr(s, result, added, exclude); }
                    visit(body, result, added, exclude);
                }
                Stmt::Input { vars, .. } => {
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::Read(vars) => {
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::If { cond, then_body, elseif_branches, else_body } => {
                    scan_expr(cond, result, added, exclude);
                    visit(then_body, result, added, exclude);
                    for (ec, b) in elseif_branches {
                        scan_expr(ec, result, added, exclude);
                        visit(b, result, added, exclude);
                    }
                    if let Some(b) = else_body { visit(b, result, added, exclude); }
                }
                Stmt::While { cond, body } => {
                    scan_expr(cond, result, added, exclude);
                    visit(body, result, added, exclude);
                }
                Stmt::Do { kind, body } => {
                    match kind {
                        DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                        DoKind::WhilePost(e) | DoKind::UntilPost(e) =>
                            scan_expr(e, result, added, exclude),
                        DoKind::Infinite => {}
                    }
                    visit(body, result, added, exclude);
                }
                Stmt::Select { expr, cases, default } => {
                    scan_expr(expr, result, added, exclude);
                    for c in cases {
                        for cond in &c.conditions {
                            match cond {
                                CaseCond::Value(e) | CaseCond::Is(_, e) =>
                                    scan_expr(e, result, added, exclude),
                                CaseCond::Range(a, b) => {
                                    scan_expr(a, result, added, exclude);
                                    scan_expr(b, result, added, exclude);
                                }
                            }
                        }
                        visit(&c.body, result, added, exclude);
                    }
                    if let Some(b) = default { visit(b, result, added, exclude); }
                }
                Stmt::Call { args, .. } => {
                    for arg in args { scan_expr(arg, result, added, exclude); }
                }
                Stmt::Print { args, .. } => {
                    for a in args {
                        match a {
                            PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                                scan_expr(e, result, added, exclude),
                            PrintArg::Comma => {}
                        }
                    }
                }
                Stmt::PrintUsing { fmt, args, .. } => {
                    scan_expr(fmt, result, added, exclude);
                    for a in args { scan_expr(a, result, added, exclude); }
                }
                Stmt::PrintFileUsing { file_num, fmt, args, .. } => {
                    scan_expr(file_num, result, added, exclude);
                    scan_expr(fmt, result, added, exclude);
                    for a in args { scan_expr(a, result, added, exclude); }
                }
                // Graphics statements — scan all sub-expressions for vars
                Stmt::Line { x1, y1, x2, y2, color, .. } => {
                    if let Some(e) = x1 { scan_expr(e, result, added, exclude); }
                    if let Some(e) = y1 { scan_expr(e, result, added, exclude); }
                    scan_expr(x2, result, added, exclude);
                    scan_expr(y2, result, added, exclude);
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::View { x1, y1, x2, y2, fill, border } => {
                    for e in [x1, y1, x2, y2] { scan_expr(e, result, added, exclude); }
                    if let Some(f) = fill   { scan_expr(f, result, added, exclude); }
                    if let Some(b) = border { scan_expr(b, result, added, exclude); }
                }
                Stmt::Window { x1, y1, x2, y2, .. } => {
                    for e in [x1, y1, x2, y2] { scan_expr(e, result, added, exclude); }
                }
                Stmt::PaletteUsing(e) => { scan_expr(e, result, added, exclude); }
                Stmt::Circle { x, y, r, color, .. } => {
                    for e in [x, y, r] { scan_expr(e, result, added, exclude); }
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::Paint { x, y, fill, border, .. } => {
                    for e in [x, y, fill] { scan_expr(e, result, added, exclude); }
                    if let Some(b) = border { scan_expr(b, result, added, exclude); }
                }
                Stmt::Pset { x, y, color, .. } => {
                    scan_expr(x, result, added, exclude);
                    scan_expr(y, result, added, exclude);
                    if let Some(c) = color { scan_expr(c, result, added, exclude); }
                }
                Stmt::Locate { row, col, cursor } => {
                    if let Some(e) = row { scan_expr(e, result, added, exclude); }
                    if let Some(e) = col  { scan_expr(e, result, added, exclude); }
                    if let Some(e) = cursor { scan_expr(e, result, added, exclude); }
                }
                Stmt::Color { fg, bg } => {
                    if let Some(e) = fg { scan_expr(e, result, added, exclude); }
                    if let Some(e) = bg { scan_expr(e, result, added, exclude); }
                }
                Stmt::Screen(e) | Stmt::Play(e) | Stmt::Randomize(Some(e)) => {
                    scan_expr(e, result, added, exclude);
                }
                Stmt::Sound { freq, duration } => {
                    scan_expr(freq, result, added, exclude);
                    scan_expr(duration, result, added, exclude);
                }
                Stmt::Wait { port, mask, xormask } => {
                    scan_expr(port, result, added, exclude);
                    scan_expr(mask, result, added, exclude);
                    if let Some(x) = xormask { scan_expr(x, result, added, exclude); }
                }
                Stmt::DefSeg(seg) => {
                    if let Some(s) = seg { scan_expr(s, result, added, exclude); }
                }
                Stmt::Poke { addr, val } | Stmt::Out { port: addr, val } => {
                    scan_expr(addr, result, added, exclude);
                    scan_expr(val, result, added, exclude);
                }
                Stmt::Swap(a, b) => {
                    if let LValue::Scalar { name, ty } = a { push(name, ty, result, added, exclude); }
                    if let LValue::Scalar { name, ty } = b { push(name, ty, result, added, exclude); }
                }
                Stmt::Palette { attr, color64 } => {
                    scan_expr(attr, result, added, exclude);
                    scan_expr(color64, result, added, exclude);
                }
                Stmt::PutSprite { x, y, .. } => {
                    scan_expr(x, result, added, exclude);
                    scan_expr(y, result, added, exclude);
                }
                Stmt::GetSprite { x1, y1, x2, y2, .. } => {
                    scan_expr(x1, result, added, exclude);
                    scan_expr(y1, result, added, exclude);
                    scan_expr(x2, result, added, exclude);
                    scan_expr(y2, result, added, exclude);
                }
                Stmt::Block(inner) => visit(inner, result, added, exclude),
                // FIELD variables must be pre-declared as String locals
                Stmt::Field { fields, .. } => {
                    for (_len_expr, var) in fields {
                        if let LValue::Scalar { name, .. } = var {
                            push(name, &QbType::String, result, added, exclude);
                        }
                    }
                }
                // LSet/RSet assign to a variable — ensure it's declared
                Stmt::LSet { var, expr } | Stmt::RSet { var, expr } => {
                    if let LValue::Scalar { name, ty } = var {
                        push(name, ty, result, added, exclude);
                    }
                    scan_expr(expr, result, added, exclude);
                }
                Stmt::InputFile { vars, file_num } => {
                    scan_expr(file_num, result, added, exclude);
                    for lv in vars {
                        if let LValue::Scalar { name, ty } = lv {
                            push(name, ty, result, added, exclude);
                        }
                    }
                }
                Stmt::LineInputFile { var, file_num } => {
                    scan_expr(file_num, result, added, exclude);
                    if let LValue::Scalar { name, ty } = var {
                        push(name, ty, result, added, exclude);
                    }
                }
                Stmt::PrintFile { file_num, args, .. } => {
                    scan_expr(file_num, result, added, exclude);
                    for a in args {
                        match a {
                            PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                                scan_expr(e, result, added, exclude),
                            PrintArg::Comma => {}
                        }
                    }
                }
                Stmt::WriteFile { file_num, args } => {
                    scan_expr(file_num, result, added, exclude);
                    for a in args { scan_expr(a, result, added, exclude); }
                }
                Stmt::FileGet { file_num, record, record_var } |
                Stmt::FilePut { file_num, record, record_var } => {
                    scan_expr(file_num, result, added, exclude);
                    if let Some(r) = record { scan_expr(r, result, added, exclude); }
                    // Scan any index expressions inside the record variable so a
                    // loop counter used only there (e.g. PUT #1, n, ARR(k)) is seen.
                    if let Some(rv) = record_var {
                        let mut cur = rv;
                        loop {
                            match cur {
                                LValue::Index { indices, .. } => {
                                    for e in indices { scan_expr(e, result, added, exclude); }
                                    break;
                                }
                                LValue::Field { base, .. } |
                                LValue::FieldIndex { base, .. } => cur = base,
                                LValue::Scalar { .. } => break,
                            }
                        }
                    }
                }
                Stmt::Open { path, file_num, rec_len, .. } => {
                    scan_expr(path, result, added, exclude);
                    scan_expr(file_num, result, added, exclude);
                    if let Some(r) = rec_len { scan_expr(r, result, added, exclude); }
                }
                Stmt::Close { file_nums } => {
                    for e in file_nums { scan_expr(e, result, added, exclude); }
                }
                _ => {}
            }
        }
    }

    /// Pre-mark CONST names so collect_locals won't try to declare them as locals.
    fn pre_mark_consts(stmts: &[Stmt], added: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Const { name, .. } => {
                    added.insert(rust_ident(name));
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    pre_mark_consts(then_body, added);
                    for (_, b) in elseif_branches { pre_mark_consts(b, added); }
                    if let Some(b) = else_body { pre_mark_consts(b, added); }
                }
                Stmt::While { body, .. } | Stmt::Do { body, .. } => pre_mark_consts(body, added),
                Stmt::For { body, .. } => pre_mark_consts(body, added),
                Stmt::Select { cases, default, .. } => {
                    for c in cases { pre_mark_consts(&c.body, added); }
                    if let Some(b) = default { pre_mark_consts(b, added); }
                }
                Stmt::Block(inner) => pre_mark_consts(inner, added),
                _ => {}
            }
        }
    }

    // FOR vars are now pushed in visit() itself (not pre-marked/excluded)
    // so they appear in result and get declared at function top scope.
    pre_mark_consts(stmts, &mut added);
    visit(stmts, &mut result, &mut added, exclude);
    result
}

/// Collect names of arrays DIM'd locally inside a function body (not shared).
pub(super) fn collect_local_array_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
                    // Insert the rust_ident_typed name (the name used in emit_lvalue/lift_expr).
                    // For string arrays (e.g. choice$→choice_s), don't insert the bare lowercase
                    // name to avoid colliding with a same-named numeric scalar (e.g. choice: f64).
                    let typed_name = rust_ident_typed(&d.name, &d.ty);
                    names.insert(typed_name);
                    // Also insert bare lowercase for non-string arrays (safe: no _s suffix collision)
                    if d.ty != QbType::String {
                        names.insert(d.name.to_lowercase());
                    }
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

/// Collect bare-lowercase names of all explicit scalar DIM declarations
/// (non-shared, non-array) in the given statement list.  Used to detect when
/// a local `DIM B AS INTEGER` shadows a DIM SHARED string `B$` that shares
/// the same base name after sigil-stripping.
pub(super) fn collect_local_dim_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) if d.dims.is_empty() && !d.shared => {
                    names.insert(d.name.to_lowercase());
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

/// Scan a SUB body to find the maximum index depth used for a given array param.
/// Array params use a 1D placeholder in VarDecl regardless of actual usage,
/// e.g. `spr()` might be accessed as `spr(c, r)` (2D) inside the body.
pub(super) fn array_param_used_dims(name: &str, stmts: &[Stmt]) -> usize {
    let name_lc = name.to_lowercase();
    let mut max = 1usize;
    fn visit_stmts(n: &str, stmts: &[Stmt], m: &mut usize) {
        for s in stmts { visit_stmt(n, s, m); }
    }
    fn visit_stmt(n: &str, stmt: &Stmt, m: &mut usize) {
        match stmt {
            Stmt::Let { var, expr } => { visit_lval(n, var, m); visit_expr(n, expr, m); }
            Stmt::If { cond, then_body, elseif_branches, else_body } => {
                visit_expr(n, cond, m);
                visit_stmts(n, then_body, m);
                for (e, b) in elseif_branches { visit_expr(n, e, m); visit_stmts(n, b, m); }
                if let Some(b) = else_body { visit_stmts(n, b, m); }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                visit_stmts(n, body, m);
            }
            Stmt::Select { cases, default, .. } => {
                for c in cases { visit_stmts(n, &c.body, m); }
                if let Some(b) = default { visit_stmts(n, b, m); }
            }
            Stmt::Block(inner) => visit_stmts(n, inner, m),
            Stmt::Call { args, .. } => { for a in args { visit_expr(n, a, m); } }
            Stmt::Print { args, .. } => {
                for a in args {
                    if let crate::parser::PrintArg::Expr(e) = a { visit_expr(n, e, m); }
                }
            }
            _ => {}
        }
    }
    fn visit_lval(n: &str, lv: &LValue, m: &mut usize) {
        if let LValue::Index { name: ln, indices, .. } = lv {
            if ln.to_lowercase() == n { *m = (*m).max(indices.len()); }
        }
    }
    fn visit_expr(n: &str, e: &Expr, m: &mut usize) {
        match e {
            Expr::Var(lv) => visit_lval(n, lv, m),
            Expr::BinOp { lhs, rhs, .. } => { visit_expr(n, lhs, m); visit_expr(n, rhs, m); }
            Expr::UnOp { operand, .. } => visit_expr(n, operand, m),
            Expr::Call { name, args } => {
                // Array access like spr(c, r) is often parsed as Expr::Call
                if name.to_lowercase() == n && !args.is_empty() {
                    *m = (*m).max(args.len());
                }
                for a in args { visit_expr(n, a, m); }
            }
            _ => {}
        }
    }
    visit_stmts(&name_lc, stmts, &mut max);
    max
}

/// Collect bare-lowercase names of non-shared local DIM'd string arrays.
/// Used by emit_lvalue to emit `name_s[...]` for arrays declared `DIM name(...) AS STRING`
/// even when accessed without the `$` sigil (so the parser records type as Single).
pub(super) fn collect_local_string_arrays(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                // Sigil-less only, same reasoning as collect_local_string_scalars:
                // a sigiled `DIM a$(…)` array's uses always carry the `$`; a bare
                // `a(i)` access refers to a different (numeric) array in QB.
                Stmt::Dim(d) if !d.dims.is_empty() && !d.shared
                    && d.ty == QbType::String && !d.str_sigil => {
                    names.insert(d.name.to_lowercase());
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}

/// Collect bare-lowercase names of non-shared local DIM'd string SCALARS
/// (`DIM name AS STRING`, no `$` sigil, no array dims). Twin of
/// collect_local_string_arrays above — same recursive-walk shape, differing
/// only in `d.dims.is_empty()` (scalar) vs `!d.dims.is_empty()` (array).
/// Used by is_str_expr_ctx so an assignment to one of these scalars gets the
/// `.to_string()` treatment: the parser records a sigil-less string DIM's
/// type as Single at use sites, so without this the emitter has no other way
/// to recover "this local is actually a String".
pub(super) fn collect_local_string_scalars(stmts: &[Stmt]) -> HashSet<String> {
    let mut names = HashSet::new();
    fn visit(stmts: &[Stmt], names: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                // Only sigil-less `DIM name AS STRING` — a sigiled `DIM name$`
                // declares a DISTINCT QB variable whose every use carries the
                // `$` (typed String at parse); including it here would
                // misclassify a coexisting bare NUMERIC `name` as a string
                // (`DIM t, t$` — mario.bas's title-screen frame counter).
                Stmt::Dim(d) if d.dims.is_empty() && !d.shared
                    && d.ty == QbType::String && !d.str_sigil => {
                    names.insert(d.name.to_lowercase());
                }
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, names);
                    for (_, b) in elseif_branches { visit(b, names); }
                    if let Some(b) = else_body { visit(b, names); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, names);
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, names); }
                    if let Some(b) = default { visit(b, names); }
                }
                Stmt::Block(inner) => visit(inner, names),
                _ => {}
            }
        }
    }
    visit(stmts, &mut names);
    names
}


// ── TYPE variable name collector ─────────────────────────────────────────────

/// Walk all DIM/REDIM statements and record var_lower → type_name_lower
/// for any `DIM x [(...)] AS UserTypeName` declarations.
pub(super) fn collect_var_type_names(prog: &AnalyzedProgram, out: &mut HashMap<String, String>) {
    fn visit_stmt(stmt: &Stmt, out: &mut HashMap<String, String>) {
        match stmt {
            Stmt::Dim(d) | Stmt::ReDim(d) => {
                if let QbType::UserType(ty) = &d.ty {
                    out.insert(rust_ident(&d.name), ty.to_lowercase());
                }
            }
            Stmt::Block(inner) => { for s in inner { visit_stmt(s, out); } }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                for s in then_body { visit_stmt(s, out); }
                for (_, b) in elseif_branches { for s in b { visit_stmt(s, out); } }
                if let Some(b) = else_body { for s in b { visit_stmt(s, out); } }
            }
            Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                for s in body { visit_stmt(s, out); }
            }
            Stmt::Select { cases, default, .. } => {
                for c in cases { for s in &c.body { visit_stmt(s, out); } }
                if let Some(b) = default { for s in b { visit_stmt(s, out); } }
            }
            _ => {}
        }
    }
    for s in &prog.main_body { visit_stmt(s, out); }
    for sub in &prog.subs {
        for s in &sub.body { visit_stmt(s, out); }
        // Also record scalar UserType params so call-site expansion can detect them
        for p in &sub.params {
            if p.dims.is_empty() {
                if let QbType::UserType(ty) = &p.ty {
                    out.insert(rust_ident(&p.name), ty.to_lowercase());
                }
            }
        }
    }
    for f in &prog.functions {
        for s in &f.body { visit_stmt(s, out); }
        for p in &f.params {
            if p.dims.is_empty() {
                if let QbType::UserType(ty) = &p.ty {
                    out.insert(rust_ident(&p.name), ty.to_lowercase());
                }
            }
        }
    }
}

// ── Typed array field collector ───────────────────────────────────────────────

/// Scan every SUB, FUNCTION, and main-body for `arr(i).Field` accesses and
/// build a map from lowercased base-array name → ordered list of field names.
/// Also records the max number of indices seen for each typed array into `dims`.
/// This drives both `emit_params` (typed array expansion) and `emit_dim`.
pub(super) fn collect_typed_array_fields(prog: &AnalyzedProgram)
    -> (HashMap<String, Vec<String>>, HashMap<String, usize>)
{
    let mut map:  HashMap<String, Vec<String>> = HashMap::new();
    let mut dims: HashMap<String, usize>       = HashMap::new();

    fn record(base: &str, field: &str, nidx: usize,
              map: &mut HashMap<String, Vec<String>>,
              dims: &mut HashMap<String, usize>)
    {
        let key = base.to_lowercase();
        let entry = map.entry(key.clone()).or_default();
        let fl = field.to_lowercase();
        if !entry.contains(&fl) { entry.push(fl); }
        let d = dims.entry(key).or_insert(0);
        if nidx > *d { *d = nidx; }
    }

    fn visit_lv(lv: &LValue,
                map:  &mut HashMap<String, Vec<String>>,
                dims: &mut HashMap<String, usize>)
    {
        match lv {
            LValue::Field { base, field } => {
                if let LValue::Index { name, indices, .. } = base.as_ref() {
                    record(name, field, indices.len(), map, dims);
                    for e in indices { visit_expr(e, map, dims); }
                }
                visit_lv(base, map, dims);
            }
            LValue::FieldIndex { base, indices, .. } => {
                for e in indices { visit_expr(e, map, dims); }
                visit_lv(base, map, dims);
            }
            LValue::Index { indices, .. } => {
                for e in indices { visit_expr(e, map, dims); }
            }
            LValue::Scalar { .. } => {}
        }
    }

    fn visit_expr(expr: &Expr,
                  map:  &mut HashMap<String, Vec<String>>,
                  dims: &mut HashMap<String, usize>)
    {
        match expr {
            Expr::Var(lv) => visit_lv(lv, map, dims),
            Expr::BinOp { lhs, rhs, .. } => {
                visit_expr(lhs, map, dims); visit_expr(rhs, map, dims);
            }
            Expr::UnOp { operand, .. } => visit_expr(operand, map, dims),
            Expr::Call { args, .. } => { for a in args { visit_expr(a, map, dims); } }
            Expr::Point { x, y } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
            }
            Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        }
    }

    fn visit_stmts(stmts: &[Stmt],
                   map:   &mut HashMap<String, Vec<String>>,
                   dims:  &mut HashMap<String, usize>)
    {
        for s in stmts { visit_stmt(s, map, dims); }
    }

    fn visit_stmt(stmt: &Stmt,
                  map:  &mut HashMap<String, Vec<String>>,
                  dims: &mut HashMap<String, usize>)
    {
        match stmt {
            Stmt::Let { var, expr } => {
                visit_lv(var, map, dims); visit_expr(expr, map, dims);
            }
            Stmt::If { cond, then_body, elseif_branches, else_body } => {
                visit_expr(cond, map, dims);
                visit_stmts(then_body, map, dims);
                for (e, b) in elseif_branches {
                    visit_expr(e, map, dims); visit_stmts(b, map, dims);
                }
                if let Some(b) = else_body { visit_stmts(b, map, dims); }
            }
            Stmt::For { from, to, step, body, .. } => {
                visit_expr(from, map, dims); visit_expr(to, map, dims);
                if let Some(s) = step { visit_expr(s, map, dims); }
                visit_stmts(body, map, dims);
            }
            Stmt::While { cond, body } => {
                visit_expr(cond, map, dims); visit_stmts(body, map, dims);
            }
            Stmt::Do { kind, body } => {
                match kind {
                    DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                    DoKind::WhilePost(e) | DoKind::UntilPost(e) => visit_expr(e, map, dims),
                    DoKind::Infinite => {}
                }
                visit_stmts(body, map, dims);
            }
            Stmt::Select { expr, cases, default } => {
                visit_expr(expr, map, dims);
                for c in cases { visit_stmts(&c.body, map, dims); }
                if let Some(b) = default { visit_stmts(b, map, dims); }
            }
            Stmt::Call { args, .. } | Stmt::Data(args) => {
                for a in args { visit_expr(a, map, dims); }
            }
            Stmt::Print { args, .. } => {
                for a in args {
                    match a {
                        PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) =>
                            visit_expr(e, map, dims),
                        PrintArg::Comma => {}
                    }
                }
            }
            Stmt::PrintUsing { fmt, args, .. } => {
                visit_expr(fmt, map, dims);
                for a in args { visit_expr(a, map, dims); }
            }
            Stmt::PrintFileUsing { file_num, fmt, args, .. } => {
                visit_expr(file_num, map, dims);
                visit_expr(fmt, map, dims);
                for a in args { visit_expr(a, map, dims); }
            }
            Stmt::Circle { x, y, r, color, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                visit_expr(r, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::Line { x1, y1, x2, y2, color, .. } => {
                if let Some(e) = x1 { visit_expr(e, map, dims); }
                if let Some(e) = y1 { visit_expr(e, map, dims); }
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::View { x1, y1, x2, y2, fill, border } => {
                visit_expr(x1, map, dims); visit_expr(y1, map, dims);
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
                if let Some(f) = fill   { visit_expr(f, map, dims); }
                if let Some(b) = border { visit_expr(b, map, dims); }
            }
            Stmt::Window { x1, y1, x2, y2, .. } => {
                visit_expr(x1, map, dims); visit_expr(y1, map, dims);
                visit_expr(x2, map, dims); visit_expr(y2, map, dims);
            }
            Stmt::PaletteUsing(e) => { visit_expr(e, map, dims); }
            Stmt::Pset { x, y, color, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                if let Some(c) = color { visit_expr(c, map, dims); }
            }
            Stmt::Paint { x, y, fill, border, .. } => {
                visit_expr(x, map, dims); visit_expr(y, map, dims);
                visit_expr(fill, map, dims);
                if let Some(b) = border { visit_expr(b, map, dims); }
            }
            Stmt::Block(inner) => visit_stmts(inner, map, dims),
            _ => {}
        }
    }

    for sub in &prog.subs      { visit_stmts(&sub.body, &mut map, &mut dims); }
    for f   in &prog.functions { visit_stmts(&f.body,   &mut map, &mut dims); }
    visit_stmts(&prog.main_body, &mut map, &mut dims);
    (map, dims)
}

// ── TYPE field flattening ─────────────────────────────────────────────────────

/// Recursively flatten a user-defined TYPE's fields, expanding any nested
/// UserType fields into their constituent scalar fields.
/// Returns `Vec<(flat_field_path, QbType)>` where `flat_field_path` uses `__`
/// separators, e.g. `"col__r"` for a `Col AS Color` field where `Color` has `R`.
/// Emit a line that unpacks one record field from buffer `buf` into `acc`
/// (a string or f64 accessor) for a random-access GET.
pub(super) fn record_get_line(acc: &str, repr: &FieldRepr, off: &usize, buf: &str) -> String {
    match repr {
        FieldRepr::Str(n) => format!("{acc} = qb_rec_get_str(&{buf}, {off}, {n});"),
        FieldRepr::I16    => format!("{acc} = qb_rec_get_i16(&{buf}, {off});"),
        FieldRepr::I32    => format!("{acc} = qb_rec_get_i32(&{buf}, {off});"),
        FieldRepr::F32    => format!("{acc} = qb_rec_get_f32(&{buf}, {off});"),
        FieldRepr::F64    => format!("{acc} = qb_rec_get_f64(&{buf}, {off});"),
        FieldRepr::Nested(_) => String::new(), // never a leaf
    }
}

/// Emit a line that packs accessor `acc` into buffer `buf` for a random-access PUT.
pub(super) fn record_put_line(acc: &str, repr: &FieldRepr, off: &usize, buf: &str) -> String {
    match repr {
        FieldRepr::Str(n) => format!("qb_rec_put_str(&mut {buf}, {off}, &{acc}, {n});"),
        FieldRepr::I16    => format!("qb_rec_put_i16(&mut {buf}, {off}, {acc});"),
        FieldRepr::I32    => format!("qb_rec_put_i32(&mut {buf}, {off}, {acc});"),
        FieldRepr::F32    => format!("qb_rec_put_f32(&mut {buf}, {off}, {acc});"),
        FieldRepr::F64    => format!("qb_rec_put_f64(&mut {buf}, {off}, {acc});"),
        FieldRepr::Nested(_) => String::new(),
    }
}

pub(super) fn flatten_type_fields(
    type_name: &str,
    type_defs: &HashMap<String, Vec<(String, QbType)>>,
) -> Vec<(String, QbType)> {
    let Some(fields) = type_defs.get(type_name) else { return Vec::new(); };
    let mut result = Vec::new();
    for (fname, fty) in fields {
        if let QbType::UserType(nested_tn) = fty {
            let nested_lc = nested_tn.to_lowercase();
            let nested = flatten_type_fields(&nested_lc, type_defs);
            for (nested_path, nested_ty) in nested {
                result.push((format!("{fname}__{nested_path}"), nested_ty));
            }
        } else {
            result.push((fname.clone(), fty.clone()));
        }
    }
    result
}

// ── DIM lower-bound helpers ───────────────────────────────────────────────────

/// Extract a constant integer lower bound from a parsed Expr.
/// Returns 0 for any non-constant expression (safe fallback — means no index offset).
pub(super) fn lower_bound_i64(expr: &Expr) -> i64 {
    match expr {
        Expr::IntLit(n)   => *n as i64,
        Expr::FloatLit(f) => *f as i64,
        Expr::UnOp { op: UnOp::Neg, operand } => -lower_bound_i64(operand),
        _ => 0,
    }
}

/// Walk `stmts` recursively and record the lower bound for every DIM/REDIM
/// array declaration into `map`.  Called as a pre-pass before emitting so
/// that subs emitted before `fn main` already know the lower bounds of shared
/// arrays declared in the main body.
pub(super) fn collect_array_lower_bounds(stmts: &[Stmt], map: &mut HashMap<String, Vec<i64>>) {
    for stmt in stmts {
        match stmt {
            Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
                let lows: Vec<i64> = d.dim_lower.iter().map(lower_bound_i64).collect();
                map.entry(d.name.to_lowercase()).or_insert(lows);
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_array_lower_bounds(then_body, map);
                for (_, b) in elseif_branches { collect_array_lower_bounds(b, map); }
                if let Some(b) = else_body { collect_array_lower_bounds(b, map); }
            }
            Stmt::For   { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::While { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::Do    { body, .. } => collect_array_lower_bounds(body, map),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_array_lower_bounds(&c.body, map); }
                if let Some(b) = default { collect_array_lower_bounds(b, map); }
            }
            Stmt::Block(inner) => collect_array_lower_bounds(inner, map),
            _ => {}
        }
    }
}

// ── GOSUB-block extractor ─────────────────────────────────────────────────────

/// Split `main_body` into:
///   - statements that precede the first `Label` (go into `fn main`)
///   - one `(label_name, body_stmts)` per labeled block (emitted as `fn label(...)`)
///
/// Labeled blocks are delimited by successive `Stmt::Label` nodes; `Stmt::Return`
/// inside a block is kept (it becomes `return;` in the emitted fn).
/// Collect every label name that appears as a GOSUB target (recursively).
pub(super) fn collect_gosub_targets(stmts: &[Stmt]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Gosub(label) => { targets.insert(label.clone()); }
            // ON KEY/TIMER GOSUB targets are extracted as functions so the runtime
            // key-event dispatch helper can call them by name.
            Stmt::OnKeyGosub { target, .. } | Stmt::OnTimerGosub { target, .. } => {
                targets.insert(target.clone());
            }
            // ON ERROR GOTO labels are treated as gosub-style targets only when
            // the label is a named (non-numeric) identifier.  Numeric labels live
            // in the state-machine match arms and must NOT be extracted as fns.
            Stmt::OnError { label } if label != "0" && label.parse::<i64>().is_err() => {
                targets.insert(label.clone());
            }
            // ON expr GOSUB L1,L2,… — every label is a GOSUB target (extracted as
            // a fn so the call + RETURN works), including numeric line labels.
            Stmt::OnGoto { labels, is_gosub: true, .. } => {
                for l in labels { targets.insert(l.clone()); }
            }
            // Named ON GOTO targets are also extracted as GOSUB fns so the emitter
            // can convert `ON x GOTO Label` into a direct fn call (Fix 3 in
            // emit_stmt).  Numeric labels must stay in the __pc state machine.
            Stmt::OnGoto { labels, is_gosub: false, .. } => {
                for l in labels {
                    if l.parse::<u32>().is_err() {
                        targets.insert(l.clone());
                    }
                }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                targets.extend(collect_gosub_targets(then_body));
                for (_, eb) in elseif_branches { targets.extend(collect_gosub_targets(eb)); }
                if let Some(eb) = else_body { targets.extend(collect_gosub_targets(eb)); }
            }
            Stmt::For    { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::While  { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::Do     { body, .. } => { targets.extend(collect_gosub_targets(body)); }
            Stmt::Select { cases, default, .. } => {
                for c in cases { targets.extend(collect_gosub_targets(&c.body)); }
                if let Some(b) = default { targets.extend(collect_gosub_targets(b)); }
            }
            Stmt::Block(inner) => { targets.extend(collect_gosub_targets(inner)); }
            _ => {}
        }
    }
    targets
}

/// Scan SUB/FUNCTION bodies for ON KEY(n) GOSUB and ON TIMER(n) GOSUB targets.
/// These labels live in the main body and must be extracted as gosub functions
/// even though no explicit GOSUB statement in the main body references them.
pub(super) fn collect_event_gosub_targets_from_stmts(stmts: &[Stmt], targets: &mut HashSet<String>) {
    for stmt in stmts {
        match stmt {
            Stmt::OnKeyGosub   { target, .. } |
            Stmt::OnTimerGosub { target, .. } => { targets.insert(target.clone()); }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_event_gosub_targets_from_stmts(then_body, targets);
                for (_, b) in elseif_branches { collect_event_gosub_targets_from_stmts(b, targets); }
                if let Some(b) = else_body { collect_event_gosub_targets_from_stmts(b, targets); }
            }
            Stmt::For    { body, .. } |
            Stmt::While  { body, .. } |
            Stmt::Do     { body, .. } => collect_event_gosub_targets_from_stmts(body, targets),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_event_gosub_targets_from_stmts(&c.body, targets); }
                if let Some(b) = default { collect_event_gosub_targets_from_stmts(b, targets); }
            }
            Stmt::Block(inner) => collect_event_gosub_targets_from_stmts(inner, targets),
            _ => {}
        }
    }
}

/// Collect all GOTO target labels reachable from `stmts`.
pub(super) fn collect_goto_targets(stmts: &[Stmt]) -> HashSet<String> {
    let mut targets = HashSet::new();
    for stmt in stmts {
        match stmt {
            Stmt::Goto(label) => { targets.insert(label.clone()); }
            // ON expr GOTO L1,L2,… — every label is a GOTO target (stays an SM arm).
            Stmt::OnGoto { labels, is_gosub: false, .. } => {
                for l in labels { targets.insert(l.clone()); }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                targets.extend(collect_goto_targets(then_body));
                for (_, eb) in elseif_branches { targets.extend(collect_goto_targets(eb)); }
                if let Some(eb) = else_body { targets.extend(collect_goto_targets(eb)); }
            }
            Stmt::For    { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::While  { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::Do     { body, .. } => { targets.extend(collect_goto_targets(body)); }
            Stmt::Select { cases, default, .. } => {
                for c in cases { targets.extend(collect_goto_targets(&c.body)); }
                if let Some(b) = default { targets.extend(collect_goto_targets(b)); }
            }
            Stmt::Block(inner) => { targets.extend(collect_goto_targets(inner)); }
            _ => {}
        }
    }
    targets
}

/// Split main body into (inline_stmts, gosub_fn_blocks).
///
/// Labels that are GOSUB targets start a new extracted function block.
/// While inside an active GOSUB block, intermediate labels that are neither
/// GOSUB targets nor GOTO targets are absorbed into that block — this correctly
/// handles old-style BASIC where GOSUB routines span multiple numbered lines
/// and the code falls through from label to label until a RETURN.
pub(super) fn extract_gosub_blocks(stmts: &[Stmt], extra_gosub_targets: &HashSet<String>) -> (Vec<Stmt>, Vec<(String, Vec<Stmt>)>) {
    let mut gosub_targets = collect_gosub_targets(stmts);
    gosub_targets.extend(extra_gosub_targets.iter().cloned());
    let goto_targets  = collect_goto_targets(stmts);

    let mut main_stmts: Vec<Stmt> = Vec::new();
    let mut blocks: Vec<(String, Vec<Stmt>)> = Vec::new();
    let mut cur_label: Option<String> = None;
    let mut cur_body:  Vec<Stmt> = Vec::new();
    let mut cur_is_gosub = false;

    for stmt in stmts {
        if let Stmt::Label(name) = stmt {
            let is_gosub_target = gosub_targets.contains(name.as_str());
            let is_goto_target  = goto_targets.contains(name.as_str());

            if cur_is_gosub && !is_gosub_target && !is_goto_target {
                // Intermediate line-number label inside a GOSUB body — absorb it.
                // GOSUB subroutines in old-style BASIC fall through multiple
                // numbered lines until RETURN; keep them together in one fn.
                cur_body.push(stmt.clone());
            } else {
                // Flush previous block
                if let Some(label) = cur_label.take() {
                    if cur_is_gosub {
                        blocks.push((label, cur_body.clone()));
                    } else {
                        main_stmts.push(Stmt::Label(label));
                        main_stmts.append(&mut cur_body);
                    }
                    cur_body.clear();
                }
                cur_label    = Some(name.clone());
                cur_is_gosub = is_gosub_target;
            }
        } else if cur_label.is_some() {
            cur_body.push(stmt.clone());
        } else {
            main_stmts.push(stmt.clone());
        }
    }
    // Flush last block
    if let Some(label) = cur_label {
        if cur_is_gosub {
            blocks.push((label, cur_body));
        } else {
            main_stmts.push(Stmt::Label(label));
            main_stmts.extend(cur_body);
        }
    }

    (main_stmts, blocks)
}

// ── Cross-boundary array detection ───────────────────────────────────────────
//
// In QB, GOSUB does not create a new scope — all variables live in the same
// flat namespace.  Our emitter turns GOSUB blocks into separate Rust functions,
// which breaks that assumption.  Any array that is declared in one scope (main
// or a GOSUB block) but *used* in another must be promoted to GameState so
// both scopes can reach it via `__gs`.

pub(super) fn collect_array_names_stmts(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    for stmt in stmts {
        collect_array_names_stmt(stmt, &mut out);
    }
    out
}

pub(super) fn collect_array_names_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        // DIM / REDIM of an array → record the name
        Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
            out.insert(rust_ident(&d.name));
        }
        // GET / PUT sprite ops reference an array directly
        Stmt::GetSprite { arr, .. } | Stmt::PutSprite { arr, .. } => {
            let name = match arr {
                LValue::Scalar { name, .. } | LValue::Index { name, .. } => name,
                LValue::Field  { base, .. } | LValue::FieldIndex { base, .. } => match base.as_ref() {
                    LValue::Scalar { name, .. } | LValue::Index { name, .. } => name,
                    _ => return,
                },
            };
            out.insert(rust_ident(name));
        }
        // CALL args may contain bare array references (array passed without ())
        Stmt::Call { args, .. } => {
            for e in args { collect_array_names_expr(e, out); }
        }
        // Recurse into compound statements
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            out.extend(collect_array_names_stmts(then_body));
            for (_, b) in elseif_branches { out.extend(collect_array_names_stmts(b)); }
            if let Some(eb) = else_body { out.extend(collect_array_names_stmts(eb)); }
        }
        Stmt::For  { body, .. }  => out.extend(collect_array_names_stmts(body)),
        Stmt::While { body, .. } => out.extend(collect_array_names_stmts(body)),
        Stmt::Do    { body, .. } => out.extend(collect_array_names_stmts(body)),
        Stmt::Select { cases, default, .. } => {
            for c in cases { out.extend(collect_array_names_stmts(&c.body)); }
            if let Some(d) = default { out.extend(collect_array_names_stmts(d)); }
        }
        Stmt::Block(inner) => out.extend(collect_array_names_stmts(inner)),
        _ => {}
    }
}

pub(super) fn collect_array_names_expr(expr: &Expr, out: &mut HashSet<String>) {
    match expr {
        // A call with no args is how the parser represents a bare array reference
        // (e.g. `arr()` in CALL MySub(arr())). We cannot tell here whether it is
        // truly an array, but over-inclusion is safe.
        Expr::Call { name, args } if args.is_empty() => { out.insert(rust_ident(name)); }
        Expr::Call { args, .. } => { for a in args { collect_array_names_expr(a, out); } }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_array_names_expr(lhs, out);
            collect_array_names_expr(rhs, out);
        }
        Expr::UnOp { operand, .. } => collect_array_names_expr(operand, out),
        _ => {}
    }
}

// ── Named-GOTO labeled-loop helpers ──────────────────────────────────────────

/// Returns the set of label names that are (a) at the tail of `body` (the very
/// last statements, possibly several consecutive Label stmts) and (b) targeted
/// by a named GOTO somewhere within `body`.  These represent the QB idiom of
/// jumping to the end of a DO loop iteration, equivalent to `continue`.
/// Returns true for `INKEY$ = ""` (the DO: LOOP UNTIL INKEY$ = "" drain pattern).
pub(super) fn is_inkey_eq_empty(expr: &Expr) -> bool {
    match expr {
        Expr::BinOp { op: BinOp::Eq, lhs, rhs } => {
            let lhs_is_inkey = matches!(lhs.as_ref(),
                Expr::Call { name, args } if name.to_lowercase() == "inkey$" && args.is_empty());
            let rhs_is_empty = matches!(rhs.as_ref(), Expr::StrLit(s) if s.is_empty());
            lhs_is_inkey && rhs_is_empty
        }
        _ => false,
    }
}

pub(super) fn find_bottom_goto_labels(body: &[Stmt]) -> HashSet<String> {
    // Collect all named GOTO targets within the body
    let mut goto_targets: HashSet<String> = HashSet::new();
    collect_named_goto_targets_stmts(body, &mut goto_targets);
    if goto_targets.is_empty() { return HashSet::new(); }

    // Walk from the end of the body, collecting consecutive Label stmts
    let mut bottom_labels: HashSet<String> = HashSet::new();
    for stmt in body.iter().rev() {
        match stmt {
            Stmt::Label(name) if goto_targets.contains(name) => {
                bottom_labels.insert(name.clone());
            }
            Stmt::Label(_) => {
                // Label not targeted by any GOTO — stop scanning
                break;
            }
            _ => break,
        }
    }
    bottom_labels
}

pub(super) fn collect_named_goto_targets_stmts(stmts: &[Stmt], out: &mut HashSet<String>) {
    for stmt in stmts {
        collect_named_goto_targets_stmt(stmt, out);
    }
}

pub(super) fn collect_named_goto_targets_stmt(stmt: &Stmt, out: &mut HashSet<String>) {
    match stmt {
        Stmt::Goto(label) if label.parse::<u32>().is_err() => {
            out.insert(label.clone());
        }
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            collect_named_goto_targets_stmts(then_body, out);
            for (_, b) in elseif_branches { collect_named_goto_targets_stmts(b, out); }
            if let Some(b) = else_body { collect_named_goto_targets_stmts(b, out); }
        }
        Stmt::For    { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::While  { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::Do     { body, .. } => collect_named_goto_targets_stmts(body, out),
        Stmt::Select { cases, default, .. } => {
            for c in cases { collect_named_goto_targets_stmts(&c.body, out); }
            if let Some(b) = default { collect_named_goto_targets_stmts(b, out); }
        }
        Stmt::Block(inner) => collect_named_goto_targets_stmts(inner, out),
        _ => {}
    }
}

// ── Cross-boundary scalar detection ──────────────────────────────────────────

/// Collect all scalar variable (lowercase_name → QbType) references from a
/// statement tree.  Array-indexed accesses are excluded (we only want scalars).
pub(super) fn collect_scalar_names_stmts(stmts: &[Stmt]) -> HashMap<String, QbType> {
    let mut out: HashMap<String, QbType> = HashMap::new();
    collect_scalar_names_inner(stmts, &mut out);
    out
}

pub(super) fn collect_scalar_names_inner(stmts: &[Stmt], out: &mut HashMap<String, QbType>) {
    for stmt in stmts {
        collect_scalar_names_stmt(stmt, out);
    }
}

pub(super) fn collect_scalar_names_stmt(stmt: &Stmt, out: &mut HashMap<String, QbType>) {
    match stmt {
        Stmt::Let { var, expr } => {
            if let LValue::Scalar { name, ty } = var {
                out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
            }
            collect_scalar_names_expr(expr, out);
        }
        Stmt::Dim(decl) if decl.dims.is_empty() && !decl.shared => {
            out.entry(decl.name.to_lowercase()).or_insert_with(|| decl.ty.clone());
        }
        Stmt::For { var, from, to, step, body } => {
            out.entry(var.to_lowercase()).or_insert(QbType::Single);
            collect_scalar_names_expr(from, out);
            collect_scalar_names_expr(to, out);
            if let Some(s) = step { collect_scalar_names_expr(s, out); }
            collect_scalar_names_inner(body, out);
        }
        Stmt::If { cond, then_body, elseif_branches, else_body } => {
            collect_scalar_names_expr(cond, out);
            collect_scalar_names_inner(then_body, out);
            for (ec, b) in elseif_branches {
                collect_scalar_names_expr(ec, out);
                collect_scalar_names_inner(b, out);
            }
            if let Some(b) = else_body { collect_scalar_names_inner(b, out); }
        }
        Stmt::While { cond, body } => {
            collect_scalar_names_expr(cond, out);
            collect_scalar_names_inner(body, out);
        }
        Stmt::Do { kind, body } => {
            match kind {
                DoKind::WhilePre(e)  | DoKind::UntilPre(e)  |
                DoKind::WhilePost(e) | DoKind::UntilPost(e) =>
                    collect_scalar_names_expr(e, out),
                DoKind::Infinite => {}
            }
            collect_scalar_names_inner(body, out);
        }
        Stmt::Select { expr, cases, default } => {
            collect_scalar_names_expr(expr, out);
            for c in cases { collect_scalar_names_inner(&c.body, out); }
            if let Some(b) = default { collect_scalar_names_inner(b, out); }
        }
        Stmt::Print { args, .. } => {
            for a in args {
                if let PrintArg::Expr(e) = a { collect_scalar_names_expr(e, out); }
            }
        }
        Stmt::PrintUsing { fmt, args, .. } => {
            // Scalars appearing only in PRINT USING (e.g. `PRINT USING "#####"; K`)
            // must still count as cross-boundary uses — otherwise a value set in
            // main and shown in a GOSUB status routine reads a local zero.
            collect_scalar_names_expr(fmt, out);
            for e in args { collect_scalar_names_expr(e, out); }
        }
        Stmt::PrintFileUsing { file_num, fmt, args, .. } => {
            collect_scalar_names_expr(file_num, out);
            collect_scalar_names_expr(fmt, out);
            for e in args { collect_scalar_names_expr(e, out); }
        }
        Stmt::Input { vars, .. } => {
            for lv in vars {
                if let LValue::Scalar { name, ty } = lv {
                    out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
                }
            }
        }
        Stmt::Call { args, .. } => {
            for a in args { collect_scalar_names_expr(a, out); }
        }
        Stmt::Block(inner) => collect_scalar_names_inner(inner, out),
        _ => {}
    }
}

pub(super) fn collect_scalar_names_expr(expr: &Expr, out: &mut HashMap<String, QbType>) {
    match expr {
        Expr::Var(LValue::Scalar { name, ty }) => {
            out.entry(name.to_lowercase()).or_insert_with(|| ty.clone());
        }
        Expr::BinOp { lhs, rhs, .. } => {
            collect_scalar_names_expr(lhs, out);
            collect_scalar_names_expr(rhs, out);
        }
        Expr::UnOp { operand, .. } => collect_scalar_names_expr(operand, out),
        Expr::Call { args, .. } => {
            for a in args { collect_scalar_names_expr(a, out); }
        }
        Expr::Point { x, y } => {
            collect_scalar_names_expr(x, out);
            collect_scalar_names_expr(y, out);
        }
        Expr::IntLit(_) | Expr::FloatLit(_) | Expr::StrLit(_) => {}
        Expr::Var(_) => {} // Index or Field — not a plain scalar
    }
}

/// Returns scalars that appear in both the main body and at least one GOSUB
/// body but are not already DIM SHARED.  These must be promoted to GameState
/// so the extracted GOSUB function can read the caller's runtime values.
///
/// `exclude` contains lowercase variable names that must NOT be promoted
/// (e.g. parameters of named SUBs, which are already shared via &mut).
pub(super) fn detect_cross_boundary_scalars(
    main_stmts: &[Stmt],
    gosub_fns:  &[(String, Vec<Stmt>)],
    exclude:    &HashSet<String>,
) -> Vec<(String, QbType)> {
    let main_scalars = collect_scalar_names_stmts(main_stmts);
    let mut result: HashMap<String, QbType> = HashMap::new();

    for (_, body) in gosub_fns {
        let gosub_scalars = collect_scalar_names_stmts(body);
        for (name, ty) in &gosub_scalars {
            if main_scalars.contains_key(name)
               && !result.contains_key(name)
               && !exclude.contains(name)
            {
                // Use main's authoritative type, not the gosub's.  This guards
                // against name collisions between e.g. `X` (f64, in main) and
                // `X$` (String, in a gosub body): both normalise to key "x" in
                // collect_scalar_names, so a gosub-body String X$ can falsely
                // appear to be the same cross-boundary variable as numeric X.
                // The main body is the canonical declaration site.
                let effective_ty = main_scalars.get(name).cloned().unwrap_or_else(|| ty.clone());
                result.insert(name.clone(), effective_ty);
            }
        }
    }

    let mut out: Vec<(String, QbType)> = result.into_iter().collect();
    out.sort_by_key(|(n, _)| n.clone());
    out
}

pub(super) fn detect_cross_boundary_arrays(
    main_stmts: &[Stmt],
    gosub_fns:  &[(String, Vec<Stmt>)],
) -> HashSet<String> {
    let main_arrays = collect_array_names_stmts(main_stmts);
    let mut result  = HashSet::new();

    for (_, body) in gosub_fns {
        let gosub_arrays = collect_array_names_stmts(body);
        // Any name that appears in BOTH main and this GOSUB body is cross-boundary
        // (catches arrays DIM'd in both scopes, e.g. old-style GOSUB subroutines
        //  that also DIM their own array).
        for name in gosub_arrays.intersection(&main_arrays) {
            result.insert(name.clone());
        }
        // Also find array USES in the GOSUB body (subscript reads/writes like
        // Numbers(I)) by scanning for Expr::Call with non-empty args where the
        // name matches a known main-body array.  This handles ON GOTO→fn targets
        // that use main-body arrays without re-declaring them.
        let mut uses = HashSet::new();
        collect_array_use_refs_stmts(body, &main_arrays, &mut uses);
        result.extend(uses);
    }

    // Cross-references between two different GOSUB blocks
    let gosub_array_sets: Vec<HashSet<String>> =
        gosub_fns.iter().map(|(_, b)| {
            let mut s = collect_array_names_stmts(b);
            collect_array_use_refs_stmts(b, &main_arrays, &mut s);
            s
        }).collect();
    for i in 0..gosub_array_sets.len() {
        for j in (i + 1)..gosub_array_sets.len() {
            for name in gosub_array_sets[i].intersection(&gosub_array_sets[j]) {
                result.insert(name.clone());
            }
        }
    }

    result
}

/// Collect names of subscript-accessed arrays in `stmts`, filtered to only
/// names that are already known to be arrays (present in `known_arrays`).
/// This avoids promoting plain function calls (e.g. `factorial(n)`) — only
/// calls whose name is a confirmed array declaration are collected.
pub(super) fn collect_array_use_refs_stmts(stmts: &[Stmt], known_arrays: &HashSet<String>,
                                 out: &mut HashSet<String>) {
    for stmt in stmts {
        collect_array_use_refs_stmt(stmt, known_arrays, out);
    }
}

pub(super) fn collect_array_use_refs_stmt(stmt: &Stmt, known_arrays: &HashSet<String>,
                                out: &mut HashSet<String>) {
    // Helper to recurse into an expression
    fn scan_expr(e: &Expr, ka: &HashSet<String>, out: &mut HashSet<String>) {
        match e {
            Expr::Call { name, args } if !args.is_empty() => {
                // Strip sigil chars ($, %, !, #, &) before lowercasing: Expr::Call
                // stores the full name-with-sigil (e.g. "Names$") while known_arrays
                // keys are sigil-free (DIM VarDecl.name strips the sigil at parse time).
                let bare = name.trim_end_matches(['$', '%', '!', '#', '&']).to_lowercase();
                if ka.contains(&bare) { out.insert(bare); }
                for a in args { scan_expr(a, ka, out); }
            }
            Expr::Call { args, .. } => {
                for a in args { scan_expr(a, ka, out); }
            }
            Expr::BinOp { lhs, rhs, .. } => { scan_expr(lhs, ka, out); scan_expr(rhs, ka, out); }
            Expr::UnOp  { operand, .. }   => scan_expr(operand, ka, out),
            _ => {}
        }
    }
    // Helper to recurse into an LValue
    fn scan_lv(lv: &LValue, ka: &HashSet<String>, out: &mut HashSet<String>) {
        match lv {
            LValue::Index { name, indices, .. } => {
                let n = rust_ident(name);
                if ka.contains(&n) { out.insert(n); }
                for e in indices { scan_expr(e, ka, out); }
            }
            LValue::Field { base, .. } => scan_lv(base, ka, out),
            _ => {}
        }
    }

    match stmt {
        Stmt::Let { var, expr }     => { scan_lv(var, known_arrays, out); scan_expr(expr, known_arrays, out); }
        Stmt::LSet { var, expr } | Stmt::RSet { var, expr } => {
            scan_lv(var, known_arrays, out); scan_expr(expr, known_arrays, out);
        }
        Stmt::Print { args, .. } | Stmt::PrintFile { args, .. } => {
            for a in args {
                if let PrintArg::Expr(e) | PrintArg::Tab(e) | PrintArg::Spc(e) = a {
                    scan_expr(e, known_arrays, out);
                }
            }
        }
        Stmt::PrintUsing { fmt, args, .. } => {
            scan_expr(fmt, known_arrays, out);
            for e in args { scan_expr(e, known_arrays, out); }
        }
        Stmt::PrintFileUsing { file_num, fmt, args, .. } => {
            scan_expr(file_num, known_arrays, out);
            scan_expr(fmt, known_arrays, out);
            for e in args { scan_expr(e, known_arrays, out); }
        }
        Stmt::Input  { vars, .. } => { for lv in vars { scan_lv(lv, known_arrays, out); } }
        Stmt::Read   (vars)       => { for lv in vars { scan_lv(lv, known_arrays, out); } }
        Stmt::Call   { args, .. } => { for e in args { scan_expr(e, known_arrays, out); } }
        Stmt::If { cond, then_body, elseif_branches, else_body } => {
            scan_expr(cond, known_arrays, out);
            collect_array_use_refs_stmts(then_body, known_arrays, out);
            for (ec, b) in elseif_branches {
                scan_expr(ec, known_arrays, out);
                collect_array_use_refs_stmts(b, known_arrays, out);
            }
            if let Some(b) = else_body { collect_array_use_refs_stmts(b, known_arrays, out); }
        }
        Stmt::For { from, to, step, body, .. } => {
            scan_expr(from, known_arrays, out);
            scan_expr(to,   known_arrays, out);
            if let Some(s) = step { scan_expr(s, known_arrays, out); }
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::While { cond, body } => {
            scan_expr(cond, known_arrays, out);
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::Do { kind, body } => {
            match kind {
                DoKind::WhilePre(e) | DoKind::UntilPre(e) |
                DoKind::WhilePost(e) | DoKind::UntilPost(e) => scan_expr(e, known_arrays, out),
                DoKind::Infinite => {}
            }
            collect_array_use_refs_stmts(body, known_arrays, out);
        }
        Stmt::Select { expr, cases, default } => {
            scan_expr(expr, known_arrays, out);
            for c in cases { collect_array_use_refs_stmts(&c.body, known_arrays, out); }
            if let Some(b) = default { collect_array_use_refs_stmts(b, known_arrays, out); }
        }
        Stmt::Block(inner)  => collect_array_use_refs_stmts(inner, known_arrays, out),
        Stmt::Dim(d) | Stmt::ReDim(d) if !d.dims.is_empty() => {
            for e in &d.dims      { scan_expr(e, known_arrays, out); }
            for e in &d.dim_lower { scan_expr(e, known_arrays, out); }
        }
        _ => {}
    }
}

// ── State-machine helpers ─────────────────────────────────────────────────────

/// Collect all non-shared DIM'd local array declarations from `stmts` (recursive).
/// Returns `(rust_name, elem_ty_str, ndims)` for each unique array.
pub(super) fn collect_sm_local_arrays(
    stmts: &[Stmt],
    shared_names: &HashSet<String>,
) -> Vec<(String, &'static str, usize)> {
    let mut seen: HashSet<String> = HashSet::new();
    let mut result = Vec::new();
    collect_sm_local_arrays_inner(stmts, shared_names, &mut seen, &mut result);
    result
}

pub(super) fn collect_sm_local_arrays_inner(
    stmts: &[Stmt],
    shared_names: &HashSet<String>,
    seen: &mut HashSet<String>,
    result: &mut Vec<(String, &'static str, usize)>,
) {
    for stmt in stmts {
        match stmt {
            Stmt::Dim(decl) if !decl.dims.is_empty() && !decl.shared => {
                let lc = decl.name.to_lowercase();
                if !shared_names.contains(&lc) {
                    let name = rust_ident_typed(&decl.name, &decl.ty);
                    if seen.insert(name.clone()) {
                        let elem = qb_type_to_rust(&decl.ty);
                        result.push((name, elem, decl.dims.len()));
                    }
                }
            }
            Stmt::If { then_body, elseif_branches, else_body, .. } => {
                collect_sm_local_arrays_inner(then_body, shared_names, seen, result);
                for (_, b) in elseif_branches {
                    collect_sm_local_arrays_inner(b, shared_names, seen, result);
                }
                if let Some(b) = else_body {
                    collect_sm_local_arrays_inner(b, shared_names, seen, result);
                }
            }
            Stmt::For  { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::While { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::Do    { body, .. } => collect_sm_local_arrays_inner(body, shared_names, seen, result),
            Stmt::Select { cases, default, .. } => {
                for c in cases { collect_sm_local_arrays_inner(&c.body, shared_names, seen, result); }
                if let Some(b) = default { collect_sm_local_arrays_inner(b, shared_names, seen, result); }
            }
            Stmt::Block(inner) => collect_sm_local_arrays_inner(inner, shared_names, seen, result),
            _ => {}
        }
    }
}

/// Returns true if `stmt` or any nested stmt contains a `Stmt::Goto`.

/// Like `stmt_has_goto` but only matches GOTO targets that are numeric line
/// numbers (i.e. GW-BASIC / line-numbered programs).  Named-label GOTOs in
/// modern QBasic programs use a different handling path and must NOT trigger
/// the state-machine emitter.
pub(super) fn stmt_has_numeric_goto(stmt: &Stmt) -> bool {
    match stmt {
        Stmt::Goto(label) => label.parse::<u32>().is_ok(),
        // ON expr GOTO <numeric> implies a line-numbered program → state machine.
        Stmt::OnGoto { labels, is_gosub: false, .. } =>
            labels.iter().any(|l| l.parse::<u32>().is_ok()),
        // ON ERROR GOTO <numeric> / RESUME <numeric> also imply a line-numbered
        // program: the handler line must be a state-machine arm to be jumpable.
        Stmt::OnError { label } => label != "0" && label.parse::<u32>().is_ok(),
        Stmt::Resume { label: Some(l), .. } => l.parse::<u32>().is_ok(),
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            then_body.iter().any(stmt_has_numeric_goto)
            || elseif_branches.iter().any(|(_, b)| b.iter().any(stmt_has_numeric_goto))
            || else_body.as_ref().map_or(false, |b| b.iter().any(stmt_has_numeric_goto))
        }
        Stmt::For    { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::While  { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::Do     { body, .. } => body.iter().any(stmt_has_numeric_goto),
        Stmt::Select { cases, default, .. } => {
            cases.iter().any(|c| c.body.iter().any(stmt_has_numeric_goto))
            || default.as_ref().map_or(false, |b| b.iter().any(stmt_has_numeric_goto))
        }
        Stmt::Block(inner) => inner.iter().any(stmt_has_numeric_goto),
        _ => false,
    }
}

/// Does `e` (recursively) read array `bare` (sigil-stripped lowercase name)?
/// Array reads appear as `Expr::Call { name }` (the usual parse of `arr(i)`)
/// or `Expr::Var(LValue::Index { name, .. })`. A same-named SCALAR read is
/// NOT counted — it binds to a different Rust local (`local_scalar_name`),
/// so it can't conflict with a borrow of the array Vec.
pub(super) fn expr_refs_array(e: &Expr, bare: &str) -> bool {
    match e {
        Expr::Call { name, args } => {
            rust_ident(name) == bare || args.iter().any(|a| expr_refs_array(a, bare))
        }
        Expr::Var(LValue::Index { name, indices, .. }) => {
            rust_ident(name) == bare || indices.iter().any(|a| expr_refs_array(a, bare))
        }
        Expr::Var(_) => false,
        Expr::BinOp { lhs, rhs, .. } => {
            expr_refs_array(lhs, bare) || expr_refs_array(rhs, bare)
        }
        Expr::UnOp { operand, .. } => expr_refs_array(operand, bare),
        _ => false,
    }
}

/// Does this statement list (recursively) contain an `ON ERROR GOTO <numeric>`?
/// When true, emit_state_machine declares the `__err_pc`/`__err_resume_pc`
/// resume-point registers so error dispatch can jump to the handler arm and
/// RESUME [NEXT] can jump back.
pub(super) fn has_numeric_on_error(stmts: &[Stmt]) -> bool {
    stmts.iter().any(|stmt| match stmt {
        Stmt::OnError { label } => label != "0" && label.parse::<u32>().is_ok(),
        Stmt::If { then_body, elseif_branches, else_body, .. } => {
            has_numeric_on_error(then_body)
            || elseif_branches.iter().any(|(_, b)| has_numeric_on_error(b))
            || else_body.as_ref().map_or(false, |b| has_numeric_on_error(b))
        }
        Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } =>
            has_numeric_on_error(body),
        Stmt::Select { cases, default, .. } => {
            cases.iter().any(|c| has_numeric_on_error(&c.body))
            || default.as_ref().map_or(false, |b| has_numeric_on_error(b))
        }
        Stmt::Block(inner) => has_numeric_on_error(inner),
        _ => false,
    })
}

/// Partition a flat statement list into blocks separated by numeric line-number labels.
/// Returns `Vec<(pc, body_stmts)>` where `pc` is the line number (or 0 for stmts
/// appearing before the first numeric label).
pub(super) fn flatten_to_blocks(stmts: &[Stmt]) -> Vec<(u32, Vec<Stmt>)> {
    let mut blocks: Vec<(u32, Vec<Stmt>)> = Vec::new();
    let mut current_pc: Option<u32> = None;
    let mut current_body: Vec<Stmt> = Vec::new();

    for stmt in stmts {
        if let Stmt::Label(s) = stmt {
            if let Ok(n) = s.parse::<u32>() {
                // Flush the previous block
                if let Some(pc) = current_pc.take() {
                    blocks.push((pc, std::mem::take(&mut current_body)));
                } else if !current_body.is_empty() {
                    blocks.push((0, std::mem::take(&mut current_body)));
                }
                current_pc = Some(n);
                // Don't add the Label itself to the body — it becomes the block key
            } else {
                // Non-numeric label (e.g. named GOSUB target) — keep as comment in body
                current_body.push(stmt.clone());
            }
        } else {
            current_body.push(stmt.clone());
        }
    }

    // Flush the last block
    if let Some(pc) = current_pc {
        blocks.push((pc, current_body));
    } else if !current_body.is_empty() {
        blocks.push((0, current_body));
    }

    // Sort by pc so arms are in line-number order (source should already be ordered,
    // but guard against edge cases)
    blocks.sort_by_key(|(pc, _)| *pc);
    blocks
}

// ── Per-sub shared-name helpers ───────────────────────────────────────────────

/// Collect names from all `SHARED var` declarations in a sub body (including
/// nested control structures).  These are the module-level variables that THIS
/// specific sub explicitly opts into via a SHARED statement.
pub(super) fn collect_sub_explicit_shared(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::SharedDecl(ns) => { for n in ns { out.insert(n.clone()); } }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } =>
                    visit(body, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, out); }
                    if let Some(b) = default { visit(b, out); }
                }
                _ => {}
            }
        }
    }
    visit(stmts, &mut out);
    out
}

/// Collect names of variables declared with `DIM SHARED` (the `SHARED` keyword
/// directly on the DIM statement) anywhere in the main body.  These are always
/// visible in every sub without a per-sub SHARED declaration.
pub(super) fn collect_dim_shared_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::Dim(d) if d.shared => { out.insert(rust_ident(&d.name)); }
                Stmt::ReDim(d) if d.shared => { out.insert(rust_ident(&d.name)); }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } =>
                    visit(body, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::Select { cases, default, .. } => {
                    for c in cases { visit(&c.body, out); }
                    if let Some(b) = default { visit(b, out); }
                }
                _ => {}
            }
        }
    }
    visit(stmts, &mut out);
    out
}

