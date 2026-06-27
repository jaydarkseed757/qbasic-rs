// ── Post-processing: collapse single-use __tmpN temporaries ──────────────────
//
// lift_expr() hoists every user-fn call and __rt.* built-in call into a
// `let __tmpN = expr;` temporary so that args to `__rt.method(...)` calls don't
// double-borrow `__rt` or `__gs`.  That's correct when the result ends up as
// an argument to another `__rt.*` call, but when the result is immediately
// assigned to a plain variable (`y = __tmp1;`) the temp is unnecessary.
//
// This pass detects the pattern:
//   let __tmpN = expr;          (immutable, plain __tmp prefix, no __tmp_*)
//   ...
//   lhs = __tmpN;               (__tmpN appears exactly once in the rest)
// and collapses it to:
//   lhs = expr;
//
// Safety:
// - Only immutable `let` (not `let mut`) are collapsed.
// - Exactly-two-occurrence check: if __tmpN is used anywhere else (as an arg
//   to an __rt call, in a complex expression, etc.) count > 2 → left alone.
// - The use must be a standalone assignment (`lhs = __tmpN;`), not embedded in
//   a larger expression — checked by matching the entire trimmed RHS.

pub(super) fn count_word_occurrences(s: &str, word: &str) -> usize {
    let sb = s.as_bytes();
    let wb = word.as_bytes();
    let wlen = wb.len();
    let mut count = 0;
    let mut i = 0;
    while i + wlen <= sb.len() {
        if &sb[i..i + wlen] == wb {
            let before_ok = i == 0 || !sb[i - 1].is_ascii_alphanumeric() && sb[i - 1] != b'_';
            let after_ok = i + wlen >= sb.len()
                || !sb[i + wlen].is_ascii_alphanumeric() && sb[i + wlen] != b'_';
            if before_ok && after_ok {
                count += 1;
            }
        }
        i += 1;
    }
    count
}

pub(super) fn inline_single_use_tmps(out: &str) -> String {
    let lines: Vec<&str> = out.lines().collect();
    let mut deletions: std::collections::HashSet<usize> = std::collections::HashSet::new();
    let mut replacements: std::collections::HashMap<usize, String> =
        std::collections::HashMap::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        // Only immutable, only plain __tmp (not __tmp_num, __tmp_gs, __tmp_str, etc.)
        if !trimmed.starts_with("let __tmp") { continue; }
        if trimmed.starts_with("let mut __tmp") { continue; }

        // Parse: "let __tmpN = expr;"
        let after_let = &trimmed["let ".len()..];
        let eq_pos = match after_let.find(" = ") { Some(p) => p, None => continue };
        let tmp_name = &after_let[..eq_pos];
        // tmp_name must be __tmp followed by ONLY digits (e.g. __tmp42, not __tmp_num3)
        let digits_part = match tmp_name.strip_prefix("__tmp") { Some(d) => d, None => continue };
        if digits_part.is_empty() || !digits_part.chars().all(|c| c.is_ascii_digit()) { continue; }
        let expr = after_let[eq_pos + 3..].trim_end_matches(';').trim();

        // Count total word-boundary occurrences — must be exactly 2 (def + one use)
        if count_word_occurrences(out, tmp_name) != 2 { continue; }

        // Find the other line and verify it's a standalone `lhs = __tmpN;`
        let use_suffix = format!(" = {};", tmp_name);
        for (j, other) in lines.iter().enumerate() {
            if j == i { continue; }
            let ot = other.trim_start();
            if !ot.ends_with(use_suffix.as_str()) { continue; }
            // lhs is everything before " = __tmpN;"
            let lhs = &ot[..ot.len() - use_suffix.len()];
            // lhs must not itself contain __tmpN (edge-case guard)
            if lhs.contains(tmp_name) { continue; }
            let indent: &str = &other[..other.len() - other.trim_start().len()];
            replacements.insert(j, format!("{indent}{lhs} = {expr};"));
            deletions.insert(i);
            break;
        }
    }

    if deletions.is_empty() {
        return out.to_string();
    }

    let mut result = String::with_capacity(out.len());
    for (i, line) in lines.iter().enumerate() {
        if deletions.contains(&i) { continue; }
        result.push_str(replacements.get(&i).map(String::as_str).unwrap_or(line));
        result.push('\n');
    }
    result
}

// ── Post-processing: remove unnecessary `mut` from local declarations ─────────
//
// emit_locals() marks ALL collected QB locals `let mut` because QB variables are
// all re-assignable by language spec.  But many locals are assigned exactly once
// and then only read — they don't need `mut`.  This pass scans each `let mut`
// binding for actual mutation evidence in the same function scope; if none is
// found the `mut` is stripped.
//
// "Mutation evidence" (keeps `mut`):
//   varname =   / varname +=  / varname -=  / varname *=  / varname /=  / varname %=
//   &mut varname  (passed byref)
//   for varname in  (range-for rebinding)
//
// Scope boundary: the next `fn ` or `pub fn ` line at column 0 (or EOF).
// Infrastructure bindings (__gs, __rt, __fn_ret, __pc, __for_*, __tmp_*, …)
// are always left alone.
//
// Safety: if we ever wrongly drop `mut` from a variable that IS assigned, rustc
// will emit "cannot assign to immutable variable" at generated-program compile
// time — making any false negative immediately visible and easy to fix.

pub(super) fn is_mutated_in_scope(lines: &[&str], varname: &str) -> bool {
    let assign   = format!("{varname} =");
    let add_eq   = format!("{varname} +");
    let sub_eq   = format!("{varname} -");
    let mul_eq   = format!("{varname} *");
    let div_eq   = format!("{varname} /");
    let mod_eq   = format!("{varname} %");
    let index    = format!("{varname}[");   // index-assignment: arr[i] = …
    let byref    = format!("&mut {varname}");
    let for_in   = format!("for {varname} in");
    for line in lines {
        let t = line.trim_start();
        if t.starts_with(assign.as_str())
            || t.starts_with(add_eq.as_str())
            || t.starts_with(sub_eq.as_str())
            || t.starts_with(mul_eq.as_str())
            || t.starts_with(div_eq.as_str())
            || t.starts_with(mod_eq.as_str())
            || t.starts_with(index.as_str())
            || t.contains(byref.as_str())
            || t.starts_with(for_in.as_str())
        {
            return true;
        }
    }
    false
}

pub(super) fn remove_unnecessary_mut(out: &str) -> String {
    // Infrastructure prefixes — always keep mut regardless of apparent usage.
    const SKIP: &[&str] = &[
        "__gs", "__rt", "__fn_ret", "__pc", "__for_", "__tmp_",
        "__pu_", "__file_", "__put_", "__fa", "__handle",
    ];

    let lines: Vec<&str> = out.lines().collect();
    let n = lines.len();
    let mut demut: std::collections::HashSet<usize> = std::collections::HashSet::new();

    for (i, line) in lines.iter().enumerate() {
        let trimmed = line.trim_start();
        if !trimmed.starts_with("let mut ") { continue; }

        // Extract varname: text between "let mut " and the first ":"
        let after = &trimmed["let mut ".len()..];
        let colon = match after.find(':') { Some(p) => p, None => continue };
        let varname = after[..colon].trim();

        // Only plain rust identifiers
        if varname.is_empty()
            || varname.contains(' ')
            || varname.contains('.')
            || !varname.chars().all(|c| c.is_ascii_alphanumeric() || c == '_')
        {
            continue;
        }

        // Leave infrastructure bindings alone
        if SKIP.iter().any(|p| varname.starts_with(p)) { continue; }

        // Scope end: next unindented `fn ` / `pub fn ` line, or EOF
        let scope_end = lines[i + 1..].iter()
            .position(|l| {
                let t = l.trim_start();
                (t.starts_with("fn ") || t.starts_with("pub fn "))
                    && !l.starts_with(' ')
                    && !l.starts_with('\t')
            })
            .map(|p| i + 1 + p)
            .unwrap_or(n);

        if !is_mutated_in_scope(&lines[i + 1..scope_end], varname) {
            demut.insert(i);
        }
    }

    if demut.is_empty() { return out.to_string(); }

    let mut result = String::with_capacity(out.len());
    for (i, line) in lines.iter().enumerate() {
        if demut.contains(&i) {
            result.push_str(&line.replacen("let mut ", "let ", 1));
        } else {
            result.push_str(line);
        }
        result.push('\n');
    }
    result
}

/// Given `b[start] == b'"'`, return the index just past the matching closing
/// quote, honoring `\"` and `\\` escapes. Shared by every byte-scanning
/// postprocess pass so Rust string literals are skipped verbatim (a `(`, `)`,
/// `;`, etc. inside a literal must never be treated as code).
fn skip_string_literal(b: &[u8], start: usize) -> usize {
    let n = b.len();
    let mut i = start + 1;
    while i < n {
        match b[i] {
            b'\\' => i += 2,
            b'"'  => { i += 1; break; }
            _     => i += 1,
        }
    }
    i
}

// ── Post-processing: strip redundant parentheses around simple derefs ─────────
//
// By-ref scalar params (`&mut f64` / `&mut String`) are read as `(*name)` at
// every use site. The wrapping parens are emitted defensively, but Rust's deref
// `*` binds tighter than every binary operator, `as`, and unary minus, so in
// almost all contexts `*name` is unambiguous and reads far cleaner:
//   qb_bool((*mouth))         -> qb_bool(*mouth)
//   ((*x) - __tmp49)          -> (*x - __tmp49)
//   Some((*row))              -> Some(*row)
//   (*x) = 5.0                -> *x = 5.0
//
// The parens are KEPT only when a postfix `.`/`[` immediately follows, since
// `*s.clone()` parses as `*(s.clone())` and `*v[i]` as `*(v[i])` — both wrong.
//
// Safety: in generated code a deref is always its own parenthesized group
// `(*ident)` (call args, operands, etc. each wrap their deref), so the leading
// `(` always belongs to the deref and never to an enclosing call — making the
// textual rewrite sound. String literals are skipped so a literal `(*x)` inside
// printed text is never altered.
pub(super) fn strip_deref_parens(out: &str) -> String {
    let b = out.as_bytes();
    let n = b.len();
    let mut result = String::with_capacity(n);
    let mut i = 0;
    let mut last = 0; // start of the not-yet-copied region

    while i < n {
        // Skip over Rust string literals verbatim (respecting \" and \\ escapes).
        if b[i] == b'"' { i = skip_string_literal(b, i); continue; }

        // Match a `(*ident)` group.
        if b[i] == b'(' && i + 1 < n && b[i + 1] == b'*' {
            let id_start = i + 2;
            let mut j = id_start;
            while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_') { j += 1; }
            if j > id_start && j < n && b[j] == b')' {
                // Keep the parens if a postfix accessor follows.
                let keep = matches!(b.get(j + 1), Some(b'.') | Some(b'['));
                if !keep {
                    result.push_str(&out[last..i]); // flush text before the group
                    result.push('*');
                    result.push_str(&out[id_start..j]); // the identifier (ASCII)
                    i = j + 1;
                    last = i;
                    continue;
                }
            }
        }
        i += 1;
    }
    result.push_str(&out[last..]);
    result
}

// ── Post-processing: drop precedence-neutral parentheses ──────────────────────
//
// The emitter wraps every arithmetic BinOp in parens and every by-ref/string arg
// in `&(...)`, which is defensive but noisy. Two rewrites are always sound:
//   1. parens around a single atom (ident / dotted-path / number / string lit):
//        &(ans_s)            -> &ans_s
//        (choice_s).as_str() -> choice_s.as_str()
//        [(i) as usize]      -> [i as usize]
//      An atom binds tighter than any surrounding operator, so the parens never
//      affect parsing. Content containing `*` is left alone for strip_deref_parens
//      (it owns the `(*ident)` case); content with spaces/operators/`(`/`[` is kept.
//   2. a fully-parenthesized assignment RHS: `= (E);` -> `= E;`. The `;` and the
//      `= ` boundary mean no operator binds across, so the outer pair is free.
//      `==`/`<=`/`>=`/`!=`/`+=`/… are excluded so conditions and compound-assigns
//      stay untouched.
//
// Runs FIRST in the postprocess chain (before strip_deref_parens) so it never
// fights the `(*x)` handling. String literals are skipped verbatim.
pub(super) fn simplify_parens(out: &str) -> String {
    let b = out.as_bytes();
    let n = b.len();
    let mut result = String::with_capacity(n);
    let mut i = 0;
    let mut last = 0; // start of the not-yet-copied region

    while i < n {
        // Skip Rust string literals verbatim (respecting \" and \\ escapes).
        if b[i] == b'"' { i = skip_string_literal(b, i); continue; }

        if b[i] == b'(' {
            // Rule 2 — `= (E);`. Require a real assignment `= ` (not `==`, `<=`,
            // `>=`, `!=`, `+=`, etc.) and a `;` right after the matching `)`.
            let is_assign_rhs = i >= 2
                && b[i - 1] == b' ' && b[i - 2] == b'='
                && !matches!(i.checked_sub(3).and_then(|k| b.get(k)),
                    Some(b'=') | Some(b'<') | Some(b'>') | Some(b'!')
                    | Some(b'+') | Some(b'-') | Some(b'*') | Some(b'/'));
            if is_assign_rhs {
                if let Some(close) = matching_paren(b, i) {
                    if b.get(close + 1) == Some(&b';') {
                        result.push_str(&out[last..i]);     // text up to '('
                        result.push_str(&out[i + 1..close]); // inner E
                        i = close + 1;                       // skip ')'
                        last = i;
                        continue;
                    }
                }
            }

            // Rule 1 — `(atom)`. Atom = one identifier/dotted-path/number
            // ([A-Za-z0-9_.]) OR one string literal, terminated by ')'. Only when
            // the `(` is a *grouping* paren, not a call/index/macro/path paren —
            // i.e. it is not preceded by an identifier char, a closing `)`/`]`, a
            // `!` (macro invocation `vec!(…)`), or a `:` (path `Foo::(…)`). This
            // keeps `qb_str("(a)")`, `qb_print_num(i)`, and any future
            // `macro!(x)` / `Foo::(x)` intact rather than mangling them to
            // `macro!x` / `Foo::x`.
            let is_grouping = i == 0 || !(b[i - 1].is_ascii_alphanumeric()
                || b[i - 1] == b'_' || b[i - 1] == b')' || b[i - 1] == b']'
                || b[i - 1] == b'!' || b[i - 1] == b':');
            if is_grouping {
                let a_start = i + 1;
                let mut j = a_start;
                if j < n && b[j] == b'"' {
                    j = skip_string_literal(b, j);
                } else {
                    while j < n && (b[j].is_ascii_alphanumeric() || b[j] == b'_' || b[j] == b'.') {
                        j += 1;
                    }
                }
                if j > a_start && j < n && b[j] == b')' {
                    result.push_str(&out[last..i]);
                    result.push_str(&out[a_start..j]); // the atom
                    i = j + 1;
                    last = i;
                    continue;
                }
            }
        }
        i += 1;
    }
    result.push_str(&out[last..]);
    result
}

/// Index of the `)` matching the `(` at `open`, skipping string literals and
/// honoring nesting. `None` if unbalanced.
fn matching_paren(b: &[u8], open: usize) -> Option<usize> {
    let n = b.len();
    let mut depth = 0i32;
    let mut i = open;
    while i < n {
        match b[i] {
            b'"' => { i = skip_string_literal(b, i); continue; }
            b'(' => depth += 1,
            b')' => {
                depth -= 1;
                if depth == 0 { return Some(i); }
            }
            _ => {}
        }
        i += 1;
    }
    None
}
