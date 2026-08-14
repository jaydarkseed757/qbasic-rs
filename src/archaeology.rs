//! `qbc --analyze` — BASIC source archaeology.
//!
//! Given a random `.bas` file (the kind found on a DOS-archive site with no
//! provenance), estimate its origin: likely era, dialect, target hardware,
//! programming style, and portability. Standalone analysis mode, same
//! family as `--compatibility` (which it reuses directly for dialect
//! detection — there's no reason to re-derive that signal twice).
//!
//! **Honesty note, load-bearing**: "Likely era" is a heuristic estimate
//! grounded in real, well-known hardware/dialect release timelines (GW-BASIC
//! 1981, CGA 1981, EGA 1984, QuickBASIC 1985-1990, VGA 1987/mainstream ~1990,
//! QBasic bundled free with MS-DOS 5 in 1991) — NOT a scientific dating
//! method. A program using only text I/O gives no graphics-era signal at
//! all and falls back to the dialect-only range. This is presented as an
//! estimate in the report itself, never as a hard claim.

use std::collections::HashMap;
use crate::compat::{self, CompatReport};
use crate::lexer::{Spanned, Token};
use crate::parser::{FileMode, Program, Stmt};

pub struct ArchReport {
    pub era: String,
    pub dialect: String,
    pub graphics: String,
    pub sound: String,
    pub storage: String,
    pub style: String,
    pub line_numbers: bool,
    pub sub_count: usize,
    pub function_count: usize,
    pub gosub_target_count: usize,
    pub goto_target_count: usize,
    pub data_count: usize,
    pub type_count: usize,
    /// (category, occurrence count), in display order, zero-count
    /// categories omitted.
    pub hardware_deps: Vec<(&'static str, usize)>,
    pub portability: &'static str,
}

pub fn analyze(source: &str, raw_bytes: &[u8], tokens: &[Spanned], prog: &Program) -> ArchReport {
    let compat = compat::audit(source, raw_bytes, tokens, prog);

    let line_numbers = has_numeric_labels(prog);
    let (sub_count, function_count) = (prog.subs.len(), prog.functions.len());
    let (gosub_targets, goto_targets) = collect_jump_targets(prog);
    let data_count = count_data_elements(prog);
    let type_count = prog.type_defs.len();

    let graphics = describe_graphics(prog);
    let sound = describe_sound(prog);
    let storage = describe_storage(prog);
    let style = describe_style(line_numbers, sub_count + function_count, goto_targets.len());

    let hardware_deps = count_hardware_deps(prog, tokens);
    let portability = estimate_portability(&compat, &hardware_deps);
    let era = estimate_era(&compat, line_numbers, sub_count + function_count, &graphics);

    ArchReport {
        era,
        dialect: compat.detected.to_string(),
        graphics,
        sound,
        storage,
        style,
        line_numbers,
        sub_count,
        function_count,
        gosub_target_count: gosub_targets.len(),
        goto_target_count: goto_targets.len(),
        data_count,
        type_count,
        hardware_deps,
        portability,
    }
}

// ── Statement walking (self-contained — see compat.rs for the deliberate
//    non-reuse-of-emitter-internals rationale) ────────────────────────────

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

fn has_numeric_labels(prog: &Program) -> bool {
    let mut found = false;
    walk_program(prog, &mut |s| {
        if let Stmt::Label(n) = s {
            if !n.is_empty() && n.chars().all(|c| c.is_ascii_digit()) { found = true; }
        }
    });
    found
}

fn collect_jump_targets(prog: &Program) -> (Vec<String>, Vec<String>) {
    let mut gosub = std::collections::HashSet::new();
    let mut goto = std::collections::HashSet::new();
    walk_program(prog, &mut |s| match s {
        Stmt::Gosub(n) => { gosub.insert(n.to_lowercase()); }
        Stmt::Goto(n) => { goto.insert(n.to_lowercase()); }
        Stmt::OnGoto { labels, is_gosub, .. } => {
            for l in labels {
                if *is_gosub { gosub.insert(l.to_lowercase()); } else { goto.insert(l.to_lowercase()); }
            }
        }
        Stmt::OnKeyGosub { target, .. } | Stmt::OnTimerGosub { target, .. } => {
            gosub.insert(target.to_lowercase());
        }
        _ => {}
    });
    (gosub.into_iter().collect(), goto.into_iter().collect())
}

fn count_data_elements(prog: &Program) -> usize {
    let mut n = 0;
    walk_program(prog, &mut |s| { if let Stmt::Data(items) = s { n += items.len(); } });
    n
}

// ── Descriptive inference ────────────────────────────────────────────────────

fn describe_graphics(prog: &Program) -> String {
    use crate::parser::{Expr, LValue};
    let mut modes: Vec<i32> = Vec::new();
    let mut var_names: Vec<String> = Vec::new();
    walk_program(prog, &mut |s| {
        if let Stmt::Screen(expr) = s {
            match expr {
                Expr::IntLit(n) => { if !modes.contains(n) { modes.push(*n); } }
                // `SCREEN Mode` — real QB programs commonly negotiate the
                // mode into a variable first (gorilla.bas: `Mode = 9` then
                // falls back to `Mode = 1` on error, `SCREEN Mode`). Resolve
                // by collecting every literal ever assigned to that name.
                Expr::Var(LValue::Scalar { name, .. }) => {
                    let lc = name.to_lowercase();
                    if !var_names.contains(&lc) { var_names.push(lc); }
                }
                _ => {}
            }
        }
    });
    if !var_names.is_empty() {
        walk_program(prog, &mut |s| {
            if let Stmt::Let { var: LValue::Scalar { name, .. }, expr: Expr::IntLit(n) } = s {
                if var_names.contains(&name.to_lowercase()) && !modes.contains(n) {
                    modes.push(*n);
                }
            }
        });
    }
    if modes.is_empty() { return "text-only (no SCREEN)".to_string(); }
    modes.sort_by(|a, b| b.cmp(a)); // highest-capability mode first
    let names: Vec<String> = modes.iter().map(|&m| match m {
        0 => "text mode".to_string(),
        1 | 2 => "CGA".to_string(),
        7 | 8 => "EGA (low-res)".to_string(),
        9 => "EGA 640x350".to_string(),
        10 => "EGA monochrome".to_string(),
        11 => "VGA monochrome 640x480".to_string(),
        12 => "VGA 640x480".to_string(),
        13 => "VGA MCGA 320x200 256-color".to_string(),
        n => format!("SCREEN {n}"),
    }).collect();
    names.join(", ")
}

fn describe_sound(prog: &Program) -> String {
    let mut has_sound = false;
    walk_program(prog, &mut |s| {
        if matches!(s, Stmt::Play(_) | Stmt::Sound { .. } | Stmt::Beep) { has_sound = true; }
    });
    if has_sound { "PC speaker (PLAY/SOUND/BEEP)".to_string() } else { "none".to_string() }
}

fn describe_storage(prog: &Program) -> String {
    let mut kinds: Vec<&'static str> = Vec::new();
    walk_program(prog, &mut |s| {
        if let Stmt::Open { mode, .. } = s {
            let k = match mode {
                FileMode::Random => "random-access files",
                FileMode::Binary => "binary files",
                FileMode::Input | FileMode::Output | FileMode::Append => "sequential files",
            };
            if !kinds.contains(&k) { kinds.push(k); }
        }
    });
    if kinds.is_empty() { "none".to_string() } else { kinds.join(", ") }
}

fn describe_style(line_numbers: bool, procedures: usize, goto_targets: usize) -> String {
    match (line_numbers, procedures > 0, goto_targets > 0) {
        (true, _, _)          => "line-numbered + GOTO".to_string(),
        (false, true, false)  => "structured (SUB/FUNCTION, no GOTO)".to_string(),
        (false, true, true)   => "structured + GOTO".to_string(),
        (false, false, true)  => "GOTO-driven, unstructured".to_string(),
        (false, false, false) => "linear (no control-flow jumps)".to_string(),
    }
}

/// Hardware-dependency categories, grounded in concretely countable
/// statement/token occurrences (not a re-derivation of compat.rs's
/// advisory list — that one's scoped to dialect-legality reporting; this
/// one is scoped to relative-intensity display).
fn count_hardware_deps(prog: &Program, tokens: &[Spanned]) -> Vec<(&'static str, usize)> {
    let mut counts: HashMap<&'static str, usize> = HashMap::new();
    walk_program(prog, &mut |s| match s {
        Stmt::Out { .. } | Stmt::Palette { .. } | Stmt::PaletteUsing(_) | Stmt::PaletteReset => {
            *counts.entry("VGA DAC").or_insert(0) += 1;
        }
        Stmt::Poke { .. } | Stmt::DefSeg(_) => {
            *counts.entry("Direct memory").or_insert(0) += 1;
        }
        Stmt::Open { .. } | Stmt::Close { .. } | Stmt::FileGet { .. } | Stmt::FilePut { .. }
        | Stmt::PrintFile { .. } | Stmt::PrintFileUsing { .. } | Stmt::InputFile { .. }
        | Stmt::LineInputFile { .. } | Stmt::WriteFile { .. } | Stmt::Field { .. } => {
            *counts.entry("DOS filesystem").or_insert(0) += 1;
        }
        Stmt::Play(_) | Stmt::Sound { .. } | Stmt::Beep => {
            *counts.entry("PC speaker").or_insert(0) += 1;
        }
        Stmt::Wait { .. } => { *counts.entry("Hardware ports").or_insert(0) += 1; }
        _ => {}
    });
    // PEEK/INP are expressions, not statements — INP has a dedicated lexer
    // token (cheap token-stream scan); PEEK isn't a qbc keyword at all (it
    // lexes as a plain Ident followed by a call), so it's matched by text.
    for sp in tokens {
        match &sp.token {
            Token::Inp => { *counts.entry("Hardware ports").or_insert(0) += 1; }
            Token::Ident(s) if s.eq_ignore_ascii_case("peek") => {
                *counts.entry("Direct memory").or_insert(0) += 1;
            }
            _ => {}
        }
    }

    let order = ["VGA DAC", "DOS filesystem", "PC speaker", "Direct memory", "Hardware ports"];
    order.iter().filter_map(|&k| counts.get(k).map(|&n| (k, n))).collect()
}

fn estimate_portability(compat: &CompatReport, hw: &[(&'static str, usize)]) -> &'static str {
    let base: u32 = match compat.detected_score {
        s if s >= 90.0 => 2, // HIGH
        s if s >= 70.0 => 1, // MEDIUM
        _              => 0, // LOW
    };
    // Heavy hardware-port/direct-memory use is fragile in ways dialect
    // compatibility alone doesn't capture (real VGA timing, real ports) —
    // downgrade one tier past a threshold, floored at LOW.
    let hw_total: usize = hw.iter().map(|(_, n)| n).sum();
    let downgrade = if hw_total > 10 { 1 } else { 0 };
    match base.saturating_sub(downgrade) {
        2 => "HIGH",
        1 => "MEDIUM",
        _ => "LOW",
    }
}

/// A best-effort era ESTIMATE (see module doc). Dialect gives the primary
/// range; a graphics-mode signal (when present) narrows it further.
///
/// `compat::audit`'s "detected" dialect is a best-fit heuristic that ties
/// QB1.1 > QB4.5 > GW-BASIC when scores are equal (e.g. a program that's
/// otherwise dialect-clean except for LF line endings, which docks all
/// three equally) — a real, accepted limitation of that field documented
/// in `compat.rs`. For era estimation specifically, a program that's
/// line-numbered AND has no SUB/FUNCTION at all is independent, strong
/// evidence of GW-BASIC heritage regardless of how the tie-break landed
/// (kingdom.bas is exactly this case).
fn estimate_era(compat: &CompatReport, line_numbers: bool, procedures: usize, graphics: &str) -> String {
    let looks_like_gwbasic = line_numbers && procedures == 0;
    let dialect_range: (u32, u32) = if looks_like_gwbasic {
        (1981, 1991)
    } else {
        match compat.detected {
            "GW-BASIC" => (1981, 1991),
            "QuickBASIC 4.5" => (1988, 1991),
            _ /* QBasic 1.1 */ => if line_numbers { (1981, 1995) } else { (1991, 1995) },
        }
    };
    let gfx_range: Option<(u32, u32)> = if graphics.contains("MCGA") || graphics.contains("VGA") {
        Some((1990, 1995))
    } else if graphics.contains("EGA") {
        Some((1985, 1991))
    } else if graphics.contains("CGA") {
        Some((1981, 1988))
    } else {
        None
    };
    let (lo, hi) = match gfx_range {
        Some((glo, ghi)) => (dialect_range.0.max(glo), dialect_range.1.min(ghi).max(glo)),
        None => dialect_range,
    };
    if lo == hi { lo.to_string() } else { format!("{lo}–{hi}") }
}

// ── Report rendering ─────────────────────────────────────────────────────────

pub fn render(r: &ArchReport) -> String {
    let mut out = String::new();
    out.push_str("Source Archaeology\n");
    out.push_str("==================\n");
    let w = 21;
    out.push_str(&format!("{:<w$}{} (heuristic estimate)\n", "Likely era:", r.era));
    out.push_str(&format!("{:<w$}{}\n", "Likely dialect:", r.dialect));
    out.push_str(&format!("{:<w$}{}\n", "Graphics:", r.graphics));
    out.push_str(&format!("{:<w$}{}\n", "Sound:", r.sound));
    out.push_str(&format!("{:<w$}{}\n", "Storage:", r.storage));
    out.push_str(&format!("{:<w$}{}\n", "Programming style:", r.style));
    out.push_str(&format!("{:<w$}{}\n", "Line numbers:", if r.line_numbers { "present" } else { "absent" }));
    out.push('\n');
    out.push_str("Program structure:\n");
    out.push_str(&format!("  SUBs:              {}\n", r.sub_count));
    out.push_str(&format!("  FUNCTIONs:         {}\n", r.function_count));
    out.push_str(&format!("  GOSUB targets:     {}\n", r.gosub_target_count));
    out.push_str(&format!("  GOTO targets:      {}\n", r.goto_target_count));
    out.push_str(&format!("  DATA elements:     {}\n", r.data_count));
    out.push_str(&format!("  TYPE definitions:  {}\n", r.type_count));
    out.push('\n');
    if r.hardware_deps.is_empty() {
        out.push_str("Hardware dependencies: none\n");
    } else {
        out.push_str("Hardware dependencies:\n");
        for (name, _) in &r.hardware_deps { out.push_str(&format!("  {name}\n")); }
    }
    out.push('\n');
    out.push_str(&format!("Estimated portability: {}\n", r.portability));

    if !r.hardware_deps.is_empty() {
        out.push('\n');
        out.push_str("Hardware Dependencies\n");
        out.push_str("---------------------\n");
        let max = r.hardware_deps.iter().map(|(_, n)| *n).max().unwrap_or(1).max(1);
        let width = r.hardware_deps.iter().map(|(n, _)| n.len()).max().unwrap_or(0) + 2;
        for (name, count) in &r.hardware_deps {
            let bar_len = ((*count as f64 / max as f64) * 20.0).round().max(1.0) as usize;
            let bar = "█".repeat(bar_len);
            out.push_str(&format!("{name:<width$}{bar} ({count})\n"));
        }
    }

    out
}

#[cfg(test)]
mod archaeology_tests {
    use super::*;

    fn run(src: &str) -> ArchReport {
        let tokens = crate::lexer::tokenize(src).expect("lex");
        let ast = crate::parser::parse(tokens.clone()).expect("parse");
        analyze(src, src.as_bytes(), &tokens, &ast)
    }

    #[test]
    fn text_only_structured_program() {
        let r = run("SUB Foo\r\nPRINT \"HI\"\r\nEND SUB\r\nCALL Foo\r\n");
        assert_eq!(r.graphics, "text-only (no SCREEN)");
        assert_eq!(r.sound, "none");
        assert_eq!(r.storage, "none");
        assert!(!r.line_numbers);
        assert_eq!(r.sub_count, 1);
        assert!(r.hardware_deps.is_empty());
    }

    #[test]
    fn line_numbered_program_detected() {
        let r = run("10 PRINT 1\r\n20 GOTO 10\r\n");
        assert!(r.line_numbers);
        assert_eq!(r.style, "line-numbered + GOTO");
        // Line-numbered + zero SUB/FUNCTION is independent evidence of
        // GW-BASIC heritage even when compat's dialect tie-break lands on
        // QB1.1 — era should reflect the earlier GW-BASIC range (1981-91),
        // not QB1.1's 1991-95.
        assert!(r.era.starts_with("1981") || r.era == "1981");
    }

    #[test]
    fn screen_13_reports_vga_mcga() {
        let r = run("SCREEN 13\r\nEND\r\n");
        assert!(r.graphics.contains("MCGA"));
    }

    #[test]
    fn screen_via_variable_is_resolved() {
        let r = run("DIM Mode AS INTEGER\r\nMode = 9\r\nMode = 1\r\nSCREEN Mode\r\nEND\r\n");
        assert!(r.graphics.contains("EGA"));
        assert!(r.graphics.contains("CGA"));
    }

    #[test]
    fn sound_statements_detected() {
        let r = run("SOUND 440, 10\r\nEND\r\n");
        assert_eq!(r.sound, "PC speaker (PLAY/SOUND/BEEP)");
    }

    #[test]
    fn random_file_storage_detected() {
        let r = run("OPEN \"X.DAT\" FOR RANDOM AS #1 LEN = 10\r\nCLOSE #1\r\n");
        assert_eq!(r.storage, "random-access files");
    }

    #[test]
    fn poke_and_defseg_count_as_direct_memory() {
        let r = run("DEF SEG = &HA000\r\nPOKE 0, 1\r\nEND\r\n");
        let dm = r.hardware_deps.iter().find(|(n, _)| *n == "Direct memory");
        assert_eq!(dm, Some(&("Direct memory", 2)));
    }

    #[test]
    fn peek_expression_counts_as_direct_memory() {
        let r = run("DIM x AS INTEGER\r\nx = PEEK(0)\r\nPRINT x\r\n");
        let dm = r.hardware_deps.iter().find(|(n, _)| *n == "Direct memory");
        assert_eq!(dm, Some(&("Direct memory", 1)));
    }

    #[test]
    fn inp_counts_as_hardware_ports() {
        let r = run("DIM x AS INTEGER\r\nx = INP(&H60)\r\nPRINT x\r\n");
        let hp = r.hardware_deps.iter().find(|(n, _)| *n == "Hardware ports");
        assert_eq!(hp, Some(&("Hardware ports", 1)));
    }

    #[test]
    fn clean_structured_program_is_high_portability() {
        let r = run("SUB Foo\r\nPRINT \"HI\"\r\nEND SUB\r\nCALL Foo\r\n");
        assert_eq!(r.portability, "HIGH");
    }

    #[test]
    fn gosub_and_goto_targets_are_counted_separately() {
        let r = run("GOSUB Sub1\r\nGOTO Skip\r\nSub1:\r\nRETURN\r\nSkip:\r\nEND\r\n");
        assert_eq!(r.gosub_target_count, 1);
        assert_eq!(r.goto_target_count, 1);
    }

    #[test]
    fn data_elements_counted_not_statements() {
        let r = run("DATA 1,2,3\r\nDATA 4,5\r\nEND\r\n");
        assert_eq!(r.data_count, 5);
    }

    #[test]
    fn type_count_is_correct() {
        let r = run("TYPE Pt\r\nX AS INTEGER\r\nY AS INTEGER\r\nEND TYPE\r\nDIM p AS Pt\r\nEND\r\n");
        assert_eq!(r.type_count, 1);
    }

    #[test]
    fn render_does_not_panic_and_contains_header() {
        let r = run("PRINT \"HELLO\"\r\n");
        let text = render(&r);
        assert!(text.starts_with("Source Archaeology"));
        assert!(text.contains("Estimated portability"));
    }

    #[test]
    fn render_with_hardware_deps_includes_bar_chart() {
        let r = run("POKE 0, 1\r\nEND\r\n");
        let text = render(&r);
        assert!(text.contains("Hardware Dependencies"));
        assert!(text.contains("█"));
    }
}
