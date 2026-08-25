//! rustc-error → QBasic-line translation.
//!
//! When the emitted Rust fails to compile, `rustc`'s diagnostics point at
//! lines in the *generated* `.rs` file — which is exactly the wrong place
//! to be looking when the actual mistake lives in the `.bas` source (or in
//! the emitter rule that handled some `.bas` construct). This module maps
//! those line numbers back to the originating QBasic lines.
//!
//! **How the map is built, without a second compile.** `--annotated`
//! output is byte-identical to plain output except for inserted
//! `// QB: <line>` comment lines (verified across all 55 bundled programs).
//! So emitting BOTH and walking them with two pointers — skipping the
//! comment lines on the annotated side — yields a plain-`.rs`-line → QB-line
//! map for the very file rustc just compiled. The walk is self-validating:
//! if the two ever disagree on a non-comment line, the alignment assumption
//! has broken and we return `None` rather than emit a wrong mapping.
//!
//! Inherits `--annotated`'s best-effort scope (see `Emitter::want_annotated`):
//! code inside a GOSUB-extracted function or a GOTO `__pc` state machine
//! carries no annotations, so errors there report "no mapping" rather than
//! guessing.

use std::collections::HashMap;

/// One `error`-level rustc diagnostic, with its location in the generated
/// `.rs` file. Warnings are deliberately ignored — emitted code has broad
/// `#![allow(...)]` coverage, and a warning is never why a build failed.
#[derive(Debug, Clone, PartialEq)]
pub struct RustcDiag {
    /// e.g. "error[E0308]: mismatched types"
    pub headline: String,
    /// 1-based line in the generated `.rs`.
    pub rs_line: usize,
    /// 1-based column in the generated `.rs`.
    pub rs_col: usize,
}

/// Remove ANSI SGR escape sequences so parsing works on colorized output
/// (we pass `--color=always` when stderr is a terminal, to preserve the
/// normal rustc experience while still capturing the text).
pub fn strip_ansi(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    let mut chars = s.chars();
    while let Some(c) = chars.next() {
        if c == '\x1b' {
            // Consume up to and including the terminating byte of the
            // sequence (SGR sequences end in 'm'; be liberal and stop at
            // the first ASCII alphabetic).
            for c2 in chars.by_ref() {
                if c2.is_ascii_alphabetic() { break; }
            }
        } else {
            out.push(c);
        }
    }
    out
}

/// Parse `rustc`'s human-readable stderr for error diagnostics referring to
/// `rs_file`. Recognizes the standard two-line shape:
///
/// ```text
/// error[E0308]: mismatched types
///   --> /path/to/generated.rs:123:45
/// ```
///
/// A `-->` line is only attached to the most recent `error` headline, so
/// the `note:`/`help:` sub-spans rustc prints afterwards (which also use
/// `-->`) can't produce phantom diagnostics.
pub fn parse_rustc_errors(stderr: &str, rs_file: &str) -> Vec<RustcDiag> {
    let plain = strip_ansi(stderr);
    let mut out = Vec::new();
    let mut pending: Option<String> = None;

    for line in plain.lines() {
        let t = line.trim_start();
        if t.starts_with("error") && t.contains(':') {
            pending = Some(t.trim_end().to_string());
            continue;
        }
        if let Some(rest) = t.strip_prefix("--> ") {
            // Only the FIRST `-->` after an error headline is that error's
            // primary span; drop the headline either way so subsequent
            // note/help spans don't re-attach to it.
            if let Some(headline) = pending.take() {
                if let Some((file, line_no, col)) = split_span(rest.trim()) {
                    if same_file(&file, rs_file) {
                        out.push(RustcDiag { headline, rs_line: line_no, rs_col: col });
                    }
                }
            }
        }
    }
    out
}

/// Split `path/to/file.rs:LINE:COL` from the right, so absolute paths
/// containing colons (or a Windows drive letter) don't break parsing.
fn split_span(s: &str) -> Option<(String, usize, usize)> {
    let (rest, col) = s.rsplit_once(':')?;
    let (file, line) = rest.rsplit_once(':')?;
    Some((file.to_string(), line.parse().ok()?, col.parse().ok()?))
}

/// rustc may print the path differently than we passed it (relative vs
/// absolute); compare on the file name, which is unique for a single-file
/// compile.
fn same_file(reported: &str, ours: &str) -> bool {
    let base = |p: &str| p.rsplit(['/', '\\']).next().unwrap_or(p).to_string();
    base(reported) == base(ours)
}

/// True for a line consisting solely of a `// QB: <n>` annotation.
fn qb_annotation(line: &str) -> Option<u32> {
    line.trim().strip_prefix("// QB: ")?.trim().parse().ok()
}

/// True when a line begins a new top-level item. An annotation only
/// applies within the item it appears in — without this reset, an error
/// inside an UNannotated body (a GOSUB-extracted fn, a `__pc` state
/// machine) sitting after an annotated one would silently inherit the
/// previous item's last QB line and report a confidently wrong location.
fn starts_top_level_item(line: &str) -> bool {
    !line.starts_with(char::is_whitespace)
        && ["fn ", "struct ", "const ", "static ", "impl ", "enum ", "pub "]
            .iter()
            .any(|k| line.starts_with(k))
}

/// Build a map for a compiled file that ALREADY carries `// QB:` comments
/// (i.e. the program was transpiled with `--annotated`, so that is the file
/// rustc saw and its diagnostics carry *annotated* line numbers). No
/// alignment is needed here — the annotations are read straight out of the
/// compiled text — but the same
/// annotation-does-not-leak-past-a-top-level-item rule applies.
pub fn build_line_map_from_annotated(annotated: &str) -> HashMap<usize, u32> {
    let mut map = HashMap::new();
    let mut cur_qb: Option<u32> = None;
    for (i, line) in annotated.lines().enumerate() {
        if let Some(n) = qb_annotation(line) {
            cur_qb = Some(n);
            continue;
        }
        if starts_top_level_item(line) { cur_qb = None; }
        if let Some(n) = cur_qb { map.insert(i + 1, n); }
    }
    map
}

/// Build a `plain .rs line (1-based)` → `QB source line` map by aligning
/// the plain and annotated emissions. Returns `None` if they diverge on
/// any non-annotation line (the alignment assumption this whole module
/// rests on has broken — report nothing rather than something wrong).
pub fn build_line_map(plain: &str, annotated: &str) -> Option<HashMap<usize, u32>> {
    let plain_lines: Vec<&str> = plain.lines().collect();
    let mut map = HashMap::new();
    let mut pi = 0usize;
    let mut cur_qb: Option<u32> = None;

    for aline in annotated.lines() {
        if let Some(n) = qb_annotation(aline) {
            cur_qb = Some(n);
            continue;
        }
        let pline = plain_lines.get(pi)?;
        if pline != &aline {
            return None; // divergence — refuse to guess
        }
        if starts_top_level_item(aline) {
            cur_qb = None;
        }
        if let Some(n) = cur_qb {
            map.insert(pi + 1, n);
        }
        pi += 1;
    }

    if pi != plain_lines.len() { return None; }
    Some(map)
}

/// Render the translation report appended after rustc's own output.
/// `bas_source` is the original `.bas` text, used to quote the offending
/// line. Returns `None` when there's nothing useful to say (no errors
/// parsed, or no diagnostic could be mapped).
pub fn render(
    diags: &[RustcDiag],
    map: Option<&HashMap<usize, u32>>,
    bas_path: &str,
    bas_source: &str,
) -> Option<String> {
    if diags.is_empty() { return None; }
    let bas_lines: Vec<&str> = bas_source.lines().collect();
    let map = map?;

    // Only report diagnostics we can actually place in the .bas source.
    let mapped: Vec<(&RustcDiag, u32)> = diags.iter()
        .filter_map(|d| map.get(&d.rs_line).map(|qb| (d, *qb)))
        .collect();
    if mapped.is_empty() { return None; }

    // Dedup BEFORE counting: one .bas statement expands to several Rust
    // lines, so a single mistake yields several diagnostics that collapse to
    // one block. Counting first made the header claim more locations than it
    // then printed.
    let mut seen = std::collections::HashSet::new();
    let shown: Vec<&(&RustcDiag, u32)> = mapped.iter()
        .filter(|(d, qb)| seen.insert((*qb, d.headline.clone())))
        .collect();

    let mut out = String::new();
    out.push_str("\n── qbc: QBasic source locations ────────────────────────────\n");
    let unmapped = diags.len() - mapped.len();
    out.push_str(&format!(
        "{} of {} rustc error(s) mapped back to {}:\n\n",
        shown.len(), diags.len(), bas_path
    ));

    for (d, qb) in &shown {
        out.push_str(&format!("  {bas_path}:{qb}\n"));
        out.push_str(&format!("    {}\n", d.headline));
        if let Some(text) = bas_lines.get(*qb as usize - 1) {
            out.push_str(&format!("    │ {}\n", text.trim_end()));
        }
        out.push('\n');
    }

    if unmapped > 0 {
        out.push_str(&format!(
            "  ({unmapped} error(s) had no QBasic mapping — inside a GOSUB-extracted\n   \
             function or a GOTO state machine, which carry no line annotations.)\n"
        ));
    }
    Some(out)
}

#[cfg(test)]
mod errmap_tests {
    use super::*;

    #[test]
    fn strips_ansi_sequences() {
        assert_eq!(strip_ansi("\x1b[0m\x1b[1;31merror\x1b[0m: bad"), "error: bad");
        assert_eq!(strip_ansi("plain text"), "plain text");
    }

    #[test]
    fn parses_error_headline_and_span() {
        let s = "error[E0308]: mismatched types\n  --> /tmp/foo.rs:123:45\n   |\n";
        let d = parse_rustc_errors(s, "/tmp/foo.rs");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].rs_line, 123);
        assert_eq!(d[0].rs_col, 45);
        assert!(d[0].headline.contains("E0308"));
    }

    #[test]
    fn parses_colorized_output() {
        let s = "\x1b[1;31merror[E0425]\x1b[0m: cannot find value\n  \x1b[34m-->\x1b[0m /tmp/a.rs:7:1\n";
        let d = parse_rustc_errors(s, "a.rs");
        assert_eq!(d.len(), 1);
        assert_eq!(d[0].rs_line, 7);
    }

    #[test]
    fn ignores_warnings_and_other_files() {
        let s = "warning: unused variable\n  --> /tmp/foo.rs:5:1\n\
                 error: boom\n  --> /other/bar.rs:9:1\n";
        assert!(parse_rustc_errors(s, "/tmp/foo.rs").is_empty());
    }

    #[test]
    fn note_span_does_not_reattach_to_previous_error() {
        let s = "error[E0308]: mismatched types\n  --> /tmp/foo.rs:10:1\n\
                 note: expected because of this\n  --> /tmp/foo.rs:99:1\n";
        let d = parse_rustc_errors(s, "/tmp/foo.rs");
        assert_eq!(d.len(), 1, "the note's --> must not become a second diagnostic");
        assert_eq!(d[0].rs_line, 10);
    }

    #[test]
    fn builds_map_from_aligned_emissions() {
        let plain = "fn main() {\n    let x = 1;\n    let y = 2;\n}\n";
        let annotated = "fn main() {\n    // QB: 10\n    let x = 1;\n    // QB: 20\n    let y = 2;\n}\n";
        let map = build_line_map(plain, annotated).expect("should align");
        assert_eq!(map.get(&2), Some(&10));
        assert_eq!(map.get(&3), Some(&20));
        assert_eq!(map.get(&1), None, "fn signature line precedes any annotation");
    }

    #[test]
    fn annotated_file_maps_by_its_own_line_numbers() {
        // Under --annotated the compiled file IS the annotated one, so
        // diagnostics carry annotated line numbers and must be looked up
        // directly — aligning two annotated sources would diverge on the
        // first comment line and silently disable mapping.
        let annotated = "fn main() {\n    // QB: 10\n    let x = 1;\n}\n";
        let map = build_line_map_from_annotated(annotated);
        // Line 3 of the ANNOTATED file is `let x = 1;`.
        assert_eq!(map.get(&3), Some(&10));
        assert_eq!(map.get(&1), None);
    }

    #[test]
    fn annotated_map_also_stops_at_top_level_items() {
        let annotated = "fn a() {\n    // QB: 10\n    let x = 1;\n}\nfn b() {\n    let y = 2;\n}\n";
        let map = build_line_map_from_annotated(annotated);
        assert_eq!(map.get(&3), Some(&10));
        assert_eq!(map.get(&6), None, "fn b must not inherit fn a's annotation");
    }

    #[test]
    fn divergence_returns_none() {
        let plain = "fn main() {\n    let x = 1;\n}\n";
        let annotated = "fn main() {\n    // QB: 10\n    let DIFFERENT = 1;\n}\n";
        assert!(build_line_map(plain, annotated).is_none());
    }

    #[test]
    fn annotation_does_not_leak_across_top_level_items() {
        // The second fn is unannotated (as a GOSUB-extracted body would
        // be); its lines must NOT inherit fn_a's last QB line.
        let plain = "fn a() {\n    let x = 1;\n}\nfn b() {\n    let y = 2;\n}\n";
        let annotated = "fn a() {\n    // QB: 10\n    let x = 1;\n}\nfn b() {\n    let y = 2;\n}\n";
        let map = build_line_map(plain, annotated).expect("should align");
        assert_eq!(map.get(&2), Some(&10));
        assert_eq!(map.get(&5), None, "fn b's body must not inherit fn a's annotation");
    }

    #[test]
    fn renders_mapped_diagnostic_with_source_line() {
        let diags = vec![RustcDiag {
            headline: "error[E0308]: mismatched types".into(),
            rs_line: 2, rs_col: 1,
        }];
        let mut map = HashMap::new();
        map.insert(2usize, 3u32);
        let bas = "PRINT 1\nPRINT 2\nLET X = \"oops\"\n";
        let r = render(&diags, Some(&map), "t.bas", bas).expect("some report");
        assert!(r.contains("t.bas:3"));
        assert!(r.contains("LET X = \"oops\""));
        assert!(r.contains("E0308"));
    }

    #[test]
    fn render_none_when_nothing_maps() {
        let diags = vec![RustcDiag { headline: "error: x".into(), rs_line: 42, rs_col: 1 }];
        let map = HashMap::new();
        assert!(render(&diags, Some(&map), "t.bas", "PRINT 1\n").is_none());
        assert!(render(&[], None, "t.bas", "PRINT 1\n").is_none());
    }

    #[test]
    fn header_count_reflects_blocks_actually_printed() {
        // Two diagnostics collapsing to one block must report "1 of 2", not
        // "2 of 2" — the count used to be taken before the dedup.
        let diags = vec![
            RustcDiag { headline: "error: same".into(), rs_line: 1, rs_col: 1 },
            RustcDiag { headline: "error: same".into(), rs_line: 2, rs_col: 1 },
        ];
        let mut map = HashMap::new();
        map.insert(1usize, 5u32);
        map.insert(2usize, 5u32);
        let r = render(&diags, Some(&map), "t.bas", "A\nB\nC\nD\nE\n").unwrap();
        assert!(r.contains("1 of 2 rustc error(s)"), "got:\n{r}");
    }

    #[test]
    fn duplicate_qb_line_and_headline_collapses() {
        let diags = vec![
            RustcDiag { headline: "error: same".into(), rs_line: 1, rs_col: 1 },
            RustcDiag { headline: "error: same".into(), rs_line: 2, rs_col: 1 },
        ];
        let mut map = HashMap::new();
        map.insert(1usize, 5u32);
        map.insert(2usize, 5u32);
        let r = render(&diags, Some(&map), "t.bas", "A\nB\nC\nD\nE\n").unwrap();
        assert_eq!(r.matches("t.bas:5").count(), 1);
    }
}
