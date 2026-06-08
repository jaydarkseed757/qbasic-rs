mod lexer;
mod parser;
mod analyzer;
mod emitter;
mod error;

use std::path::PathBuf;
use std::time::{Duration, Instant};
use anyhow::Result;
use clap::Parser;

/// QBasic to Rust transpiler
#[derive(Parser, Debug)]
#[command(name = "qbc", about = "Transpile QBasic source to Rust")]
struct Args {
    /// Input .bas file
    input: PathBuf,

    /// Output .rs file (defaults to <input>.rs)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Emit Rust source only, do not invoke rustc
    #[arg(long)]
    emit_only: bool,

    /// Dump AST to stdout for debugging
    #[arg(long)]
    dump_ast: bool,

    /// Print transpilation stats
    #[arg(short, long)]
    verbose: bool,
}

fn fmt_dur(d: Duration) -> String {
    let us = d.as_micros();
    if us < 1_000 { format!("{us}µs") }
    else if us < 1_000_000 { format!("{:.2}ms", us as f64 / 1_000.0) }
    else { format!("{:.2}s", us as f64 / 1_000_000.0) }
}

fn count_stmts(stmts: &[parser::Stmt]) -> usize {
    stmts.iter().map(|s| 1 + match s {
        parser::Stmt::If { then_body, else_body, .. } =>
            count_stmts(then_body) + else_body.as_deref().map_or(0, count_stmts),
        parser::Stmt::For   { body, .. } => count_stmts(body),
        parser::Stmt::While { body, .. } => count_stmts(body),
        parser::Stmt::Do    { body, .. } => count_stmts(body),
        parser::Stmt::Block(v)           => count_stmts(v),
        _ => 0,
    }).sum()
}

fn main() -> Result<()> {
    let args = Args::parse();
    let total_start = Instant::now();

    // 1. Read source.
    //    • Try UTF-8 first so money.bas / other modern files with box-drawing
    //      characters stored as proper Unicode are decoded correctly.
    //    • Fall back to byte-per-char for genuine Latin-1 / CP437 files whose
    //      raw bytes 0x80-0xFF would be invalid UTF-8 (like nibbles.bas with
    //      raw DOS bytes in string literals).  In that mode every byte N maps
    //      to U+00N0, which equals its CP437 glyph index in FONT_8X8.
    let raw_bytes = std::fs::read(&args.input)?;
    let source: String = if let Ok(s) = std::str::from_utf8(&raw_bytes) {
        s.to_owned()
    } else {
        raw_bytes.iter().map(|&b| b as char).collect()
    };
    let src_bytes  = raw_bytes.len();
    let src_lines  = source.lines().count();
    let blank_lines   = source.lines().filter(|l| l.trim().is_empty()).count();
    let comment_lines = source.lines().filter(|l| {
        let t = l.trim();
        t.starts_with('\'') || t.to_uppercase().starts_with("REM")
    }).count();

    // 2. Lex
    let t0 = Instant::now();
    let tokens = lexer::tokenize(&source)?;
    let lex_dur = t0.elapsed();
    let token_count = tokens.len();

    // 3. Parse → AST
    let t0 = Instant::now();
    let ast = parser::parse(tokens)?;
    let parse_dur = t0.elapsed();

    if args.dump_ast {
        println!("{ast:#?}");
        return Ok(());
    }

    let sub_count  = ast.subs.len();
    let fn_count   = ast.functions.len();
    let main_stmts = count_stmts(&ast.main_body);
    let sub_stmts: usize  = ast.subs.iter().map(|s| count_stmts(&s.body)).sum();
    let fn_stmts: usize   = ast.functions.iter().map(|f| count_stmts(&f.body)).sum();
    let label_count = ast.main_body.iter()
        .filter(|s| matches!(s, parser::Stmt::Label(_))).count();

    // 4. Analyze
    let t0 = Instant::now();
    let program = analyzer::analyze(ast)?;
    let analyze_dur = t0.elapsed();

    let global_syms = program.global_scope.symbols.len();
    let shared_syms = program.global_scope.symbols.values().filter(|s| s.shared).count();
    let data_items  = program.data_store.len();
    let const_count = program.consts.len();

    // 5. Emit Rust source
    let t0 = Instant::now();
    let rust_source = emitter::emit(&program)?;
    let emit_dur = t0.elapsed();

    let out_lines = rust_source.lines().count();
    let out_bytes = rust_source.len();

    // 6. Write output
    let out_path = args.output.unwrap_or_else(|| args.input.with_extension("rs"));
    std::fs::write(&out_path, &rust_source)?;

    if !args.verbose {
        eprintln!("Emitted: {}", out_path.display());
    }

    // 7. Optionally invoke rustc
    if !args.emit_only {
        let binary = out_path.with_extension("");
        // Locate the target profile dir (release or debug) by inspecting the
        // directory the qbc executable itself lives in.
        let exe = std::env::current_exe()?;
        let target_profile = exe.parent()
            .and_then(|p| {
                let name = p.file_name()?.to_str()?;
                if name == "debug" || name == "release" { Some(p.to_path_buf()) } else { None }
            })
            .or_else(|| {
                let mut p = exe.parent()?.to_path_buf();
                loop {
                    for profile in ["release", "debug"] {
                        let candidate = p.join("target").join(profile);
                        if candidate.join("libqbasic_runtime.rlib").exists() {
                            return Some(candidate);
                        }
                    }
                    if !p.pop() { break; }
                }
                None
            })
            .unwrap_or_else(|| std::path::PathBuf::from("target/debug"));
        let is_release = target_profile.ends_with("release");
        let deps_dir = target_profile.join("deps");
        let rlib = target_profile.join("libqbasic_runtime.rlib");
        let extern_arg = format!("qbasic_runtime={}", rlib.display());
        let mut rustc_args = vec![
            out_path.to_str().unwrap(),
            "--edition", "2021",
            "-L", deps_dir.to_str().unwrap(),
            "--extern", &extern_arg,
            "-o", binary.to_str().unwrap(),
        ];
        if is_release { rustc_args.push("-C"); rustc_args.push("opt-level=3"); }
        let status = std::process::Command::new("rustc")
            .args(&rustc_args)
            .status()?;
        if status.success() {
            if !args.verbose {
                eprintln!("Compiled: {}", binary.display());
            }
        } else {
            anyhow::bail!("rustc failed");
        }
    }

    // 8. Verbose report
    if args.verbose {
        let total_dur = total_start.elapsed();
        println!();
        println!("  qbc — transpilation report");
        println!("  ══════════════════════════════════════");
        println!("  Input   : {}", args.input.display());
        println!("  Output  : {}", out_path.display());
        println!();
        println!("  ── Source (.bas) ──────────────────────");
        println!("  Lines          : {src_lines:>8}  ({blank_lines} blank, {comment_lines} comment)");
        println!("  Bytes          : {src_bytes:>8}");
        println!();
        println!("  ── Lexer ──────────────────────────────");
        println!("  Tokens         : {token_count:>8}   [{lex_time}]", lex_time = fmt_dur(lex_dur));
        println!();
        println!("  ── Parser ─────────────────────────────");
        println!("  SUBs           : {sub_count:>8}   [{parse_time}]", parse_time = fmt_dur(parse_dur));
        println!("  FUNCTIONs      : {fn_count:>8}");
        println!("  Labels         : {label_count:>8}");
        println!("  Stmts (main)   : {main_stmts:>8}");
        println!("  Stmts (subs)   : {sub_stmts:>8}");
        println!("  Stmts (fns)    : {fn_stmts:>8}");
        println!("  Stmts (total)  : {:>8}", main_stmts + sub_stmts + fn_stmts);
        println!();
        println!("  ── Analyzer ───────────────────────────");
        println!("  Global symbols : {global_syms:>8}   [{analyze_time}]", analyze_time = fmt_dur(analyze_dur));
        println!("  Shared (DIM)   : {shared_syms:>8}");
        println!("  CONSTs         : {const_count:>8}");
        println!("  DATA items     : {data_items:>8}");
        println!();
        println!("  ── Emitter ────────────────────────────");
        println!("  Lines of Rust  : {out_lines:>8}   [{emit_time}]", emit_time = fmt_dur(emit_dur));
        println!("  Bytes          : {out_bytes:>8}");
        println!("  Expansion      : {:>7.1}x  (Rust lines / BAS lines)",
            out_lines as f64 / src_lines.max(1) as f64);
        println!();
        println!("  ── Total ──────────────────────────────");
        println!("  Pipeline time  : {}", fmt_dur(total_dur));
        println!();
    }

    Ok(())
}
