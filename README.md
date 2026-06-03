# qbasic_rs

A transpiler that converts QBasic `.bas` source files into native Rust binaries.

The primary correctness target is **GORILLAS.BAS** — the classic gorilla-throwing game shipped with MS-DOS QBasic — running at full fidelity with graphics, sound, and game logic intact.

---

## What it does

```
gorilla.bas  →  [qbc transpiler]  →  gorilla.rs  →  [rustc]  →  gorilla
```

The transpiler (`qbc`) reads a QBasic source file, walks the AST, and emits a self-contained Rust source file that links against a small runtime library. The result is a native binary with no QBasic interpreter involved at runtime.

---

## Quick start

```bash
# Build everything
cargo build

# Transpile and run gorillas
cargo run -- basic-src/gorilla.bas -o bin/gorilla.rs
rustc bin/gorilla.rs --edition 2021 \
  -L target/debug/deps \
  --extern qbasic_runtime=target/debug/libqbasic_runtime.rlib \
  -o bin/gorilla
bin/gorilla

# Or just inspect the emitted Rust
cargo run -- basic-src/gorilla.bas --emit-only
```

---

## Supported programs

| Program | Description | Status |
|---------|-------------|--------|
| `gorilla.bas` | Classic gorilla-throwing game (SCREEN 9 EGA, CIRCLE/PAINT/GET/PUT sprites, PLAY audio) | ✅ |
| `mandel.bas` | Mandelbrot renderer (VIEW/WINDOW coords, PALETTE cycling, PACE) | ✅ |
| `sortdemo.bas` | Animated sorting visualizer (SHARED vars, animation) | ✅ |
| `money.bas` | Money manager (DATA/READ, SELECT CASE, arrays) | ✅ |
| `donkey.bas` | Q-BASIC Donkey game (GOTO state machine, DRAW sprites) | ✅ |
| `pi.bas` | Arbitrary-precision pi via Machin's formula | ✅ |
| `hangman.bas` | Hangman word game (modern QBasic style, DO/LOOP, named GOSUB/GOTO) | ✅ |
| `hangman-gw.bas` | Hangman word game (GW-BASIC style, line numbers, GOTO state machine) | ✅ |
| `sound.bas` | Minimal PLAY/MML demo (text-mode, audible arpeggio) | ✅ |

---

## Features

### Language coverage
- **Control flow**: IF/ELSEIF/ELSE, FOR/NEXT, WHILE/WEND, DO/LOOP, SELECT CASE
- **Subroutines**: SUB/END SUB, FUNCTION/END FUNCTION, GOSUB/RETURN, STATIC locals (persist across calls)
- **Data**: DIM, REDIM, DIM SHARED, COMMON SHARED, DATA/READ/RESTORE, CONST, user-defined TYPE
- **Graphics**: SCREEN, LINE, CIRCLE, PAINT, PSET, PRESET, DRAW, GET, PUT, VIEW, WINDOW, PALETTE
- **Sound**: PLAY (full MML parser), SOUND, BEEP — wired to `rodio`
- **I/O**: PRINT, INPUT, LOCATE, COLOR, CLS, INKEY$, random-access files (OPEN/GET/PUT/CLOSE)

### GOTO → state machine
Line-numbered BASIC programs that use GOTO are compiled to a `match __pc { ... }` state machine, with each line number becoming a match arm. Programs that use only GOSUB get clean named Rust functions instead.

### REM QBC pragmas
Embed transpiler directives anywhere in a `.bas` source file:

```basic
REM QBC FULLSPEED
REM QBC FPS 30
REM QBC SLOWMO 2
REM QBC TITLE My Cool Game
REM QBC SCALE 2
```

| Directive | Example | Effect |
|-----------|---------|--------|
| `FULLSPEED` | `REM QBC FULLSPEED` | Disables the frame-rate throttle; program runs at full native CPU speed. Best for computation-heavy programs (mandel.bas, pi.bas). |
| `FPS N` | `REM QBC FPS 30` | Cap animation at N frames per second instead of the default 60. |
| `SLOWMO N` | `REM QBC SLOWMO 3` | Multiply every QB `SLEEP` duration by N — handy for slow-motion inspection of timed animations. |
| `TITLE text` | `REM QBC TITLE Gorilla Wars` | Set the window title bar text. Default is `QBasic`. |
| `SCALE N` | `REM QBC SCALE 2` | Multiply the output window size by N (default 960×600 → 1920×1200 for N=2). Useful on HiDPI displays. |

Directives are case-insensitive. Multiple directives combine freely.

---

## Project layout

```
qbasic-rust/
├── src/                   # Transpiler (qbc binary)
│   ├── lexer.rs           # Source text → tokens
│   ├── parser.rs          # Tokens → AST
│   ├── analyzer.rs        # AST → symbol table + AnalyzedProgram
│   └── emitter.rs         # AnalyzedProgram → Rust source  (~4500 lines)
│
├── runtime/src/
│   ├── lib.rs             # Runtime struct, graphics, I/O, math  (~2200 lines)
│   └── sound.rs           # PLAY/SOUND/BEEP via rodio  (~300 lines)
│
└── basic-src/             # .bas source files
```

See [ARCHITECTURE.md](ARCHITECTURE.md) for a full description of the pipeline, design decisions, and runtime internals.

---

## CLI

```
qbc <INPUT> [-o OUTPUT] [--emit-only] [--dump-ast] [--verbose]
```

| Flag | Effect |
|------|--------|
| `-o <file>` | Output `.rs` path |
| `--emit-only` | Write `.rs` only; skip rustc |
| `--dump-ast` | Print the parsed AST and exit |
| `--verbose` | Print per-stage timing and stats |

`qbc` auto-locates the runtime rlib relative to its own executable, so `cargo run` works without manual `-L` flags.

---

## Dependencies

| Crate | Used for |
|-------|----------|
| `minifb` | Window creation and pixel buffer display |
| `crossterm` | Terminal input (non-blocking key read) |
| `rodio` | Audio playback for PLAY/SOUND/BEEP |

---

## Design notes

- **All numerics are `f64`** — QB SINGLE precision is widened for simplicity
- **QB-accurate integer math** — `CINT` uses banker's rounding (ties to even); `\` (integer divide) and `MOD` round both operands to integers first, matching QuickBASIC rather than Rust's native operators
- **QB booleans**: `0.0` = false, `-1.0` = true (bitwise NOT convention)
- **Palette-indexed framebuffer** — `POINT(x,y)` returns a palette index, enabling QB-style collision detection
- **SHARED variables** → `GameState` struct passed as `&mut __gs` to every SUB
- **GOSUB targets** → named Rust `fn` (clean path, covers all of gorilla.bas)
- **GOTO** → `match __pc` state machine (fallback for line-numbered programs)
