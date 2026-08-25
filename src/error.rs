use thiserror::Error;

#[derive(Debug, Error)]
pub enum QbError {
    #[error("Lex error at line {line}: {msg}")]
    Lex { line: u32, msg: String, near: Option<String> },

    #[error("Parse error at line {line}: {msg}")]
    Parse { line: u32, msg: String, near: Option<String> },

    #[error("Analyze error: {0}")]
    #[allow(dead_code)]
    Analyze(String),

    #[error("Emit error: {0}")]
    #[allow(dead_code)]
    Emit(String),
}

impl QbError {
    /// The 1-based source line this error points at, if it has one.
    pub fn line(&self) -> Option<u32> {
        match self {
            QbError::Lex { line, .. } | QbError::Parse { line, .. } => Some(*line),
            _ => None,
        }
    }

    /// The source text of the offending token/lexeme, used to place the
    /// caret. `None` where the failing construct has no single lexeme.
    pub fn near(&self) -> Option<&str> {
        match self {
            QbError::Lex { near, .. } | QbError::Parse { near, .. } => near.as_deref(),
            _ => None,
        }
    }
}

/// Locate `needle` in `hay`, case-insensitively, returning its char offset
/// ONLY when it occurs exactly once — an ambiguous match would put the
/// caret under the wrong occurrence, which is worse than no caret at all.
///
/// For word-like needles (QB keywords and identifiers) the match must sit
/// on identifier boundaries, so searching for `A` in `DIM A AS INTEGER`
/// finds the standalone `A` rather than also matching the `A` inside `AS`.
/// Operator/punctuation needles skip the boundary test, since `+` or `(`
/// have no word boundaries to speak of.
fn unique_match(hay: &str, needle: &str) -> Option<usize> {
    if needle.is_empty() { return None; }
    let hay_l: Vec<char> = hay.to_lowercase().chars().collect();
    let need_l: Vec<char> = needle.to_lowercase().chars().collect();
    if need_l.len() > hay_l.len() { return None; }

    let wordish = need_l.iter().all(|c| c.is_alphanumeric() || *c == '_');
    let is_ident_char = |c: char| c.is_alphanumeric() || c == '_' || "$%!#&".contains(c);

    let mut found = None;
    for start in 0..=(hay_l.len() - need_l.len()) {
        if hay_l[start..start + need_l.len()] != need_l[..] { continue; }
        if wordish {
            let before_ok = start == 0 || !is_ident_char(hay_l[start - 1]);
            let after_i = start + need_l.len();
            let after_ok = after_i >= hay_l.len() || !is_ident_char(hay_l[after_i]);
            if !(before_ok && after_ok) { continue; }
        }
        if found.is_some() { return None; } // ambiguous — refuse to point
        found = Some(start);
    }
    found
}

/// Render a rustc-style source snippet for a lex/parse error:
///
/// ```text
///    |
/// 46 |     DIM step AS INTEGER
///    |         ^^^^
///    |
/// ```
///
/// The line itself is always exact. The caret appears only when the
/// offending lexeme can be placed unambiguously (see `unique_match`);
/// otherwise the line is shown on its own, which is still the bulk of the
/// value — it saves opening the file to find out what's there.
/// Returns `None` when the line number is out of range for the source
/// (defensive: never render a snippet we can't stand behind).
pub fn render_snippet(line: u32, near: Option<&str>, source: &str) -> Option<String> {
    let text = source.lines().nth(line.checked_sub(1)? as usize)?;
    let text = text.trim_end_matches('\r');
    let gutter = line.to_string();
    let pad = " ".repeat(gutter.len());

    let mut out = String::new();
    out.push_str(&format!("{pad} |\n"));
    out.push_str(&format!("{gutter} | {text}\n"));

    let caret = near
        .and_then(|n| unique_match(text, n).map(|col| (col, n.chars().count())));
    match caret {
        Some((col, len)) => {
            out.push_str(&format!("{pad} | {}{}\n", " ".repeat(col), "^".repeat(len.max(1))));
        }
        None => out.push_str(&format!("{pad} |\n")),
    }
    Some(out)
}

/// Full user-facing rendering of a lex/parse failure: the one-line
/// summary followed by the source snippet. Falls back to the bare summary
/// for error kinds that carry no line (Analyze/Emit).
pub fn render_error(err: &QbError, path: &str, source: &str) -> String {
    let head = format!("{err}");
    match err.line().and_then(|l| render_snippet(l, err.near(), source)) {
        Some(snip) => format!("{head}\n --> {path}:{}\n{snip}", err.line().unwrap()),
        None => head,
    }
}

#[cfg(test)]
mod error_tests {
    use super::*;

    #[test]
    fn snippet_places_caret_under_unique_token() {
        let src = "PRINT 1\nDIM step AS INTEGER\nPRINT 2\n";
        let s = render_snippet(2, Some("step"), src).unwrap();
        let lines: Vec<&str> = s.lines().collect();
        assert_eq!(lines[1], "2 | DIM step AS INTEGER");
        // Caret row carries a BLANK gutter (rustc style), then 4 spaces
        // to clear "DIM " before the 4 carets under `step`.
        assert_eq!(lines[2], "  |     ^^^^");
    }

    #[test]
    fn caret_is_case_insensitive() {
        let src = "DIM STEP AS INTEGER\n";
        let s = render_snippet(1, Some("step"), src).unwrap();
        assert!(s.contains("^^^^"), "keyword casing must not defeat the caret:\n{s}");
    }

    #[test]
    fn ambiguous_token_gets_no_caret() {
        // `X` occurs twice — pointing at either one could be wrong.
        let src = "X = X + 1\n";
        let s = render_snippet(1, Some("X"), src).unwrap();
        assert!(!s.contains('^'), "ambiguous match must not draw a caret:\n{s}");
        assert!(s.contains("X = X + 1"), "the line itself is still shown");
    }

    #[test]
    fn word_boundary_prevents_substring_match() {
        // `A` also appears inside `AS`; only the standalone one counts, so
        // this is unambiguous and SHOULD get a caret.
        let src = "DIM A AS INTEGER\n";
        let s = render_snippet(1, Some("A"), src).unwrap();
        let caret_row = s.lines().nth(2).unwrap();
        assert_eq!(caret_row, "  |     ^");
    }

    #[test]
    fn operator_needle_skips_boundary_test() {
        let src = "LET Y = (1\n";
        let s = render_snippet(1, Some("("), src).unwrap();
        assert!(s.contains('^'), "punctuation should still place a caret:\n{s}");
    }

    #[test]
    fn missing_token_text_still_shows_the_line() {
        let src = "PRINT 1\nBAD LINE HERE\n";
        let s = render_snippet(2, Some("nowhere-in-line"), src).unwrap();
        assert!(s.contains("BAD LINE HERE"));
        assert!(!s.contains('^'));
    }

    #[test]
    fn no_near_text_still_shows_the_line() {
        let src = "PRINT 1\n";
        let s = render_snippet(1, None, src).unwrap();
        assert!(s.contains("PRINT 1"));
        assert!(!s.contains('^'));
    }

    #[test]
    fn out_of_range_line_returns_none() {
        assert!(render_snippet(99, None, "PRINT 1\n").is_none());
        assert!(render_snippet(0, None, "PRINT 1\n").is_none());
    }

    #[test]
    fn crlf_source_does_not_leak_carriage_return() {
        let src = "PRINT 1\r\nDIM step AS INTEGER\r\n";
        let s = render_snippet(2, Some("step"), src).unwrap();
        assert!(!s.contains('\r'), "CR must be trimmed from the quoted line");
        assert_eq!(s.lines().nth(2).unwrap(), "  |     ^^^^");
    }

    #[test]
    fn render_error_includes_path_line_and_summary() {
        let e = QbError::Parse {
            line: 2,
            msg: "expected identifier, got Step".into(),
            near: Some("step".into()),
        };
        let out = render_error(&e, "t.bas", "PRINT 1\nDIM step AS INTEGER\n");
        assert!(out.contains("Parse error at line 2"));
        assert!(out.contains("--> t.bas:2"));
        assert!(out.contains("DIM step AS INTEGER"));
        assert!(out.contains("^^^^"));
    }

    #[test]
    fn render_error_falls_back_for_line_less_kinds() {
        let e = QbError::Emit("boom".into());
        assert_eq!(render_error(&e, "t.bas", "PRINT 1\n"), "Emit error: boom");
    }
}
