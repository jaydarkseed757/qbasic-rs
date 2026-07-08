use crate::parser::*;
use std::collections::HashSet;

// ── Helpers ───────────────────────────────────────────────────────────────────

/// Format an f64 value as an unambiguous Rust f64 literal (e.g. `42.0f64`, `3.14f64`).
/// Using the `f64` type suffix avoids ambiguity when the literal is the receiver of a
/// method call (e.g. `2.0f64.powf(10.0f64)`) — bare `2.0` would make rustc error with
/// "can't call method `powf` on ambiguous numeric type `{float}`".
pub(super) fn emit_f64_lit(f: f64) -> String {
    let s = format!("{f}");
    if s.contains('.') || s.contains('e') || s.contains('E') {
        format!("{s}f64")   // e.g. "3.14f64", "1.0f64"
    } else {
        format!("{s}.0f64") // e.g. "2.0f64" (float Display dropped the .0)
    }
}

/// True when `s` begins with `(` and that paren's match is the final char — i.e.
/// the whole string is one parenthesized group, e.g. `(*x)` or `(qb_abs(y))`.
/// Used to avoid emitting a redundant outer layer when wrapping an index.
pub(super) fn starts_with_balanced_paren(s: &str) -> bool {
    let b = s.as_bytes();
    if b.first() != Some(&b'(') { return false; }
    let mut depth = 0i32;
    for (k, &c) in b.iter().enumerate() {
        match c {
            b'(' => depth += 1,
            b')' => { depth -= 1; if depth == 0 { return k == b.len() - 1; } }
            _ => {}
        }
    }
    false
}

/// Sign of a compile-time numeric-literal expression: `Some(true)` positive,
/// `Some(false)` negative, `None` for zero or a non-literal. f64-precise (so a
/// fractional `STEP 0.5`/`-0.5` classifies correctly — unlike the i64
/// `lower_bound_i64`, which would truncate it to 0). Used by the T3 constant-
/// step FOR optimization.
pub(super) fn lit_sign(e: &Expr) -> Option<bool> {
    match e {
        Expr::IntLit(n)   => if *n > 0 { Some(true) } else if *n < 0 { Some(false) } else { None },
        Expr::FloatLit(f) => if *f > 0.0 { Some(true) } else if *f < 0.0 { Some(false) } else { None },
        Expr::UnOp { op: UnOp::Neg, operand } => lit_sign(operand).map(|p| !p),
        _ => None,
    }
}

/// True when `e` is a compile-time numeric literal (`IntLit`/`FloatLit`, or a
/// unary `-` over one) — i.e. its emitted Rust text is pure and side-effect-
/// free, so it's safe to duplicate inline (in a loop condition evaluated every
/// iteration) instead of binding it to a `let` temp evaluated once. Unlike
/// `lit_sign`, zero counts as a literal here — a `FOR i = 1 TO 0` bound is
/// unremarkable and should still inline; only `lit_sign`'s STEP-direction
/// check needs a *nonzero* sign. Used by the A4 constant-FOR-bound
/// optimization (`__for_to_{v}` binds nothing when TO is a literal).
pub(super) fn is_const_numeric_lit(e: &Expr) -> bool {
    match e {
        Expr::IntLit(_) | Expr::FloatLit(_) => true,
        Expr::UnOp { op: UnOp::Neg, operand } => is_const_numeric_lit(operand),
        _ => false,
    }
}

/// Precedence of the QB arithmetic operators that emit as *infix* Rust operators
/// — the only operators where operand parenthesization can change parsing. Every
/// other QB operator emits as a call (`qb_mod`, `qb_idiv`, `qb_and`, …) or a
/// self-delimited form (`qb_from_bool(..)`, `.powf(..)`, `(-x)`), so its operands
/// never need extra parens. Mul/Div bind tighter than Add/Sub.
pub(super) fn arith_prec(op: &BinOp) -> Option<u8> {
    match op {
        BinOp::Mul | BinOp::Div => Some(2),
        BinOp::Add | BinOp::Sub => Some(1),
        _ => None,
    }
}

/// Drop the redundant outer parens from an already-emitted arithmetic operand
/// `child_str` when doing so provably preserves the AST — and therefore the
/// exact f64 evaluation order, since float arithmetic is **not** associative, so
/// `a + (b - c)` must keep its parens. The rule reproduces Rust's left-
/// associative parse: a LEFT operand may drop iff its precedence ≥ the parent's;
/// a RIGHT operand only iff strictly greater. The caller still wraps the parent
/// expression in its own outer parens, so every use-site (`as` casts, `.powf`,
/// unary `-`) stays safe — this only de-nests the *inner* operands. The
/// `starts_with_balanced_paren` guard makes a stray `)` inside a string literal
/// (paren-counting isn't literal-aware) conservatively skip the strip.
pub(super) fn arith_operand(child: &Expr, child_str: String, parent_prec: u8, is_left: bool) -> String {
    if let Expr::BinOp { op, .. } = child {
        if let Some(cp) = arith_prec(op) {
            let droppable = if is_left { cp >= parent_prec } else { cp > parent_prec };
            if droppable && starts_with_balanced_paren(&child_str) {
                return child_str[1..child_str.len() - 1].to_string();
            }
        }
    }
    child_str
}

/// Format an array subscript `[<idx> as usize]`, adding precedence-guarding
/// parens around `idx` only when it isn't already a single balanced group.
/// Avoids the `[((*x)) as usize]` double-paren that arises when `idx` is a
/// deref like `(*x)` (the deref already carries its own parens). The single
/// inner `(*x)` is later reduced to `*x` by `strip_deref_parens`.
///
/// A compile-time constant index (`1.0f64`, from `emit_f64_lit`, or a bare
/// integer) collapses to the plain integer subscript — `arr[(1.0f64) as usize]`
/// → `arr[1]`. `as usize` on an integral non-negative f64 is exactly that
/// integer, so this is value-identical; negatives/fractions/non-literals keep
/// the cast form.
pub(super) fn idx_sub(idx: &str) -> String {
    if let Some(n) = const_usize_lit(idx) {
        return format!("[{n}]");
    }
    if starts_with_balanced_paren(idx) {
        format!("[{idx} as usize]")
    } else {
        format!("[({idx}) as usize]")
    }
}

/// Parse an emitted index string that is a constant, integral, non-negative
/// numeric literal (`"1.0f64"`, `"7"`) into its usize value. Anything else —
/// expressions, negatives, fractions — returns `None`.
fn const_usize_lit(idx: &str) -> Option<u64> {
    let core = idx.strip_suffix("f64").unwrap_or(idx);
    // Must be purely [0-9.] so idents/exprs ("i", "a + b", "(x)") never match.
    if core.is_empty() || !core.bytes().all(|b| b.is_ascii_digit() || b == b'.') {
        return None;
    }
    let v: f64 = core.parse().ok()?;
    (v.fract() == 0.0 && v >= 0.0 && v <= u64::MAX as f64).then(|| v as u64)
}

pub(super) fn rust_ident(name: &str) -> String {
    // String variables get an `_s` suffix so that QB variables `A` and `A$`
    // map to distinct Rust names (`a` and `a_s`) instead of both becoming `a`.
    // This covers names that still carry their `$` sigil (e.g. from CONST lists).
    let has_dollar = name.ends_with('$');
    let stripped = name.trim_end_matches(['$', '#', '!', '%', '&']);
    // Numeric labels (e.g. GOSUB 1780) must be prefixed so they're valid Rust identifiers.
    if stripped.chars().next().map(|c| c.is_ascii_digit()).unwrap_or(false) {
        let base = format!("lbl_{stripped}");
        return if has_dollar { format!("{base}_s") } else { base };
    }
    let base = match stripped.to_lowercase().as_str() {
        "loop" | "move" | "type" | "use" | "fn" | "let" | "return" |
        "match" | "mod"  | "ref"  | "in"  | "as" | "box" |
        "true"  | "false" | "self" | "super" | "crate" | "where" |
        "impl"  | "trait" | "enum" | "struct" | "pub"  | "mut" |
        "if"    | "else"  | "while" | "for"  | "break" | "continue" |
        // Not Rust keywords but conflict with std items used in emitted code
        "format" | "write" | "print" | "panic" | "assert" | "vec" | "string" => {
            format!("qb_{}", stripped.to_lowercase())
        }
        _ => stripped.to_lowercase(),
    };
    if has_dollar { format!("{base}_s") } else { base }
}

/// Like `rust_ident` but also applies the `_s` suffix when the parser has
/// already stripped the `$` sigil from a String variable.  Always use this
/// when you have both the QB name *and* the QbType (i.e. from an LValue or
/// VarDecl).  Use plain `rust_ident` only when operating on raw QB source
/// tokens that may still carry their sigil.
pub(super) fn rust_ident_typed(name: &str, ty: &QbType) -> String {
    let base = rust_ident(name);
    // If the type is String but the name no longer ends with `$` (parser stripped
    // it), add the `_s` suffix so it doesn't collide with a numeric `name`.
    if *ty == QbType::String && !name.ends_with('$') {
        format!("{base}_s")
    } else {
        base // rust_ident already added _s when name had $
    }
}

pub(super) fn rust_fn_name(name: &str) -> String {
    match name.to_uppercase().as_str() {
        "INT"     => "qb_int".into(),
        "FIX"     => "qb_fix".into(),
        "ABS"     => "qb_abs".into(),
        "SQR"     => "qb_sqr".into(),
        "SGN"     => "qb_sgn".into(),
        "SIN"     => "qb_sin".into(),
        "COS"     => "qb_cos".into(),
        "TAN"     => "qb_tan".into(),
        "ATN"     => "qb_atn".into(),
        "EXP"     => "qb_exp".into(),
        "LOG"     => "qb_log".into(),
        "CINT"    => "qb_cint".into(),
        "CLNG"    => "qb_cint".into(),
        "CSNG"    => "qb_csng".into(),
        "CDBL"    => "qb_cdbl".into(),
        "LEN"     => "qb_len".into(),
        "LEFT$"   => "qb_left".into(),
        "RIGHT$"  => "qb_right".into(),
        "MID$"    => "qb_mid".into(),
        "UCASE$"  => "qb_ucase".into(),
        "LCASE$"  => "qb_lcase".into(),
        "LTRIM$"  => "qb_ltrim".into(),
        "RTRIM$"  => "qb_rtrim".into(),
        "STR$"    => "qb_str_fn".into(),
        "VAL"     => "qb_val".into(),
        "CHR$"    => "qb_chr".into(),
        "ASC"     => "qb_asc".into(),
        "INSTR"   => "qb_instr".into(),
        "SPACE$"  => "qb_space".into(),
        "STRING$" => "qb_string".into(),
        "HEX$"    => "qb_hex".into(),
        "OCT$"    => "qb_oct".into(),
        "TIMER"   => "qb_timer".into(),
        "PEEK"    => "__rt.qb_peek".into(),
        "ENVIRON$"=> "qb_environ".into(),
        "DIR$"    => "qb_dir".into(),
        // Binary type-conversion functions
        "MKD"     => "MKD".into(),
        "MKS"     => "MKS".into(),
        "MKI"     => "MKI".into(),
        "MKL"     => "MKL".into(),
        "CVD"     => "CVD".into(),
        "CVS"     => "CVS".into(),
        "CVI"     => "CVI".into(),
        "CVL"     => "CVL".into(),
        // File I/O built-ins
        // (EOF routes to __rt.qb_eof in lift_expr — the free-fn stub always
        //  returned "never EOF" and is intentionally NOT mapped here so any
        //  uncovered path fails loudly at rustc instead of looping forever.)
        "LOF"     => "qb_lof_fn".into(),
        // Error handling
        "ERR"     => "__rt.err_code".into(),  // emitted as a field access, not a fn call
        other     => rust_ident(other),
    }
}

pub(super) fn qb_type_to_rust(ty: &QbType) -> &'static str {
    match ty {
        QbType::Integer     => "f64",
        QbType::Single      => "f64",
        QbType::Double      => "f64",
        QbType::String      => "String",
        QbType::UserType(_) => "f64",
    }
}

/// Rust type for an N-dimensional array of `elem`: `Vec<Vec<...<elem>...>>`.
/// `ndims` 1 → `Vec<elem>`, 2 → `Vec<Vec<elem>>`, 3 → `Vec<Vec<Vec<elem>>>`.
pub(super) fn nested_vec_type(elem: &str, ndims: usize) -> String {
    let n = ndims.max(1);
    format!("{}{}{}", "Vec<".repeat(n), elem, ">".repeat(n))
}

/// Rust initializer for an N-dimensional array filled with `default_val`.
/// `allocs` holds the per-dimension lengths, outermost first:
/// `[a0, a1, a2]` → `vec![vec![vec![D; a2]; a1]; a0]`.
pub(super) fn nested_vec_init(default_val: &str, allocs: &[String]) -> String {
    if allocs.is_empty() {
        return format!("vec![{default_val}]");
    }
    // Build inside-out: innermost element is default_val, wrap with each dim.
    let mut expr = default_val.to_string();
    for a in allocs.iter().rev() {
        expr = format!("vec![{expr}; {a}]");
    }
    expr
}

// ── String-expression detector ────────────────────────────────────────────────

/// Returns true if `expr` statically evaluates to a QB String value.
pub(super) fn is_str_expr(expr: &Expr) -> bool {
    match expr {
        Expr::StrLit(_) => true,
        Expr::Var(LValue::Scalar { ty: QbType::String, .. }) => true,
        // String array element access
        Expr::Var(LValue::Index { ty: QbType::String, .. }) => true,
        // TYPE field access — can't know type without type_defs; check name for $ sigil
        Expr::Var(LValue::Field { field, .. }) => field.ends_with('$'),
        Expr::BinOp { op: BinOp::Add, lhs, rhs } => is_str_expr(lhs) || is_str_expr(rhs),
        Expr::Call { name, .. } => {
            // String-returning built-ins or any user name ending with $
            // (covers string array element access like help$(i) and string functions)
            name.ends_with('$') || matches!(
                name.to_uppercase().as_str(),
                "LEFT$" | "RIGHT$" | "MID$" | "UCASE$" | "LCASE$" |
                "LTRIM$" | "RTRIM$" | "STR$" | "CHR$" | "HEX$" |
                "OCT$" | "STRING$" | "SPACE$" | "ENVIRON$" | "INKEY$" |
                "MKD" | "MKS" | "MKI" | "MKL"
            )
        }
        _ => false,
    }
}

/// True when `expr` emits as a Rust expression that is ALREADY an owned,
/// freshly-allocated `String` — so a caller wrapping it in `(…).to_string()`
/// (to materialize a temp, satisfy a `String ==` comparison, etc.) is cloning
/// a value that's about to be dropped anyway. Two cases:
/// - string concatenation (`BinOp::Add` on string operands) → emits `format!(…)`
/// - a call to a QB built-in whose runtime fn returns `String` by value
///   (`qb_left`, `qb_mid`, `qb_chr`, `MKD`, …)
///
/// Deliberately narrower than `is_str_expr`: a *bare* `name$` ending in `$`
/// there also covers array-element access (`help$(i)`, parsed as `Expr::Call`
/// when the array/function form is ambiguous) — that reads a `Vec<String>`
/// element and is NOT already an owned temporary, so it's excluded here. A
/// plain `Expr::Var` (scalar or array-index string read) is excluded for a
/// different reason: emitting it bare would MOVE the variable, which breaks
/// any later read of it — the `.to_string()` clone is load-bearing there.
pub(super) fn expr_returns_owned_string(expr: &Expr) -> bool {
    match expr {
        Expr::BinOp { op: BinOp::Add, lhs, rhs } => is_str_expr(lhs) || is_str_expr(rhs),
        Expr::Call { name, .. } => matches!(
            name.to_uppercase().as_str(),
            "LEFT$" | "RIGHT$" | "MID$" | "UCASE$" | "LCASE$" |
            "LTRIM$" | "RTRIM$" | "STR$" | "CHR$" | "HEX$" |
            "OCT$" | "STRING$" | "SPACE$" | "ENVIRON$" | "INKEY$" | "INPUT$" |
            "MKD" | "MKS" | "MKI" | "MKL"
        ),
        _ => false,
    }
}


// ── &str argument positions for built-in functions ────────────────────────────

/// Returns which zero-based argument positions of `fn_name` expect `&str`.
pub(super) fn str_arg_positions(fn_name: &str) -> &'static [usize] {
    match fn_name {
        "qb_len" | "qb_left" | "qb_right" | "qb_mid" |
        "qb_ucase" | "qb_lcase" | "qb_ltrim" | "qb_rtrim" |
        "qb_val" | "qb_asc" | "qb_environ" | "qb_dir" |
        "CVD" | "CVS" | "CVI" | "CVL" => &[0],
        "qb_instr" => &[1, 2],
        _ => &[],
    }
}

// ── REDIM name collector ──────────────────────────────────────────────────────

/// Collect the rust_ident_typed names of all locally REDIM'd arrays in a body.
/// These are declared inline by emit_redim(), so emit_locals must exclude them.
pub(super) fn collect_redim_names(stmts: &[Stmt]) -> HashSet<String> {
    let mut out = HashSet::new();
    fn visit(stmts: &[Stmt], out: &mut HashSet<String>) {
        for stmt in stmts {
            match stmt {
                Stmt::ReDim(d) if !d.dims.is_empty() && !d.shared => {
                    out.insert(rust_ident_typed(&d.name, &d.ty));
                }
                Stmt::Block(inner) => visit(inner, out),
                Stmt::If { then_body, elseif_branches, else_body, .. } => {
                    visit(then_body, out);
                    for (_, b) in elseif_branches { visit(b, out); }
                    if let Some(b) = else_body { visit(b, out); }
                }
                Stmt::For { body, .. } | Stmt::While { body, .. } | Stmt::Do { body, .. } => {
                    visit(body, out);
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

