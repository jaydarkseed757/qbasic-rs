# qbasic_rs — QBasic → Rust Transpiler

## Goal

Transpile QBasic `.bas` source files into native Rust binaries.
Primary correctness target: **GORILLAS.BAS** — the classic gorilla-throwing
game shipped with MS-DOS QBasic — running at 100% fidelity.

---

## Repository Layout

```
qbasic-rust/
├── Cargo.toml                  # Workspace (members: transpiler crate + runtime crate)
├── CLAUDE.md                   # AI assistant instructions and design rules
├── ARCHITECTURE.md             # This file
│
├── src/                        # Transpiler (qbc binary)
│   ├── main.rs                 # CLI, pipeline orchestration, --verbose stats
│   ├── lexer.rs                # Source text → Vec<Spanned<Token>>
│   ├── parser.rs               # Tokens → AST (Program, Stmt, Expr, LValue)
│   ├── analyzer.rs             # AST → AnalyzedProgram (symbol table, DATA)
│   ├── emitter.rs              # AnalyzedProgram → Rust source string  (~5200 lines)
│   └── error.rs                # QbError enum (Lex / Parse / Analyze / Emit)
│
├── runtime/                    # Runtime library linked by every transpiled program
│   ├── Cargo.toml              # depends on: minifb, crossterm, rodio
│   └── src/
│       ├── lib.rs              # Runtime struct, graphics, I/O, math/string fns  (~3400 lines)
│       └── sound.rs            # PLAY MML parser + SOUND/BEEP via rodio  (~300 lines)
│
└── basic-src/                  # Test .bas programs
    ├── gorilla.bas            # Primary target: gorilla-throwing game
    ├── donkey.bas              # GOTO state machine test: Q-BASIC Donkey game
    ├── mandel.bas              # Mandelbrot renderer (VIEW/WINDOW/PALETTE USING)
    ├── money.bas               # Money manager (DATA/READ, SELECT CASE, arrays)
    ├── sortdemo.bas            # Sorting visualizer (SHARED, animation)
    ├── pi.bas                  # Arbitrary-precision pi via Machin's formula
    ├── nibbles.bas
    ├── hello-world.bas
    ├── fuzzbuzz.bas
    ├── primes.bas
    └── q_sort.bas
```

---

## Pipeline

```
file.bas
  │
  ├─[lexer]──────► Vec<Spanned<Token>>
  │                  tokenize() — context-aware REM, hex/octal literals,
  │                  sigil types ($, %, !, #), multi-statement colon,
  │                  REM QBC directives captured as QbcDirective tokens
  │
  ├─[parser]─────► Program { subs, functions, main_body, directives }
  │                  Recursive-descent. Handles all QB structured flow:
  │                  IF/ELSEIF/ELSE, FOR/NEXT, DO/LOOP, WHILE/WEND,
  │                  SELECT CASE, GOSUB/RETURN, SUB/FUNCTION, DIM/REDIM,
  │                  DATA/READ/RESTORE, LINE/CIRCLE/PAINT/PSET, GET/PUT
  │
  ├─[analyzer]───► AnalyzedProgram
  │                  Builds global symbol table, resolves SHARED vars,
  │                  collects DATA store, folds CONSTs, records labels,
  │                  passes directives through unchanged
  │
  ├─[emitter]────► gorilla.rs  (Rust source text)
  │                  Walks AST → idiomatic-enough Rust.
  │                  Special cases: GOSUB→fn, GOTO→state machine,
  │                  typed arrays, aliased args, STRING$/INSTR/MID$ overloads,
  │                  REM QBC directives → Runtime configuration calls
  │
  └─[rustc]──────► gorilla  (native binary)
                     --extern qbasic_runtime=target/.../libqbasic_runtime.rlib
```

---

## CLI — `qbc`

```
qbc <INPUT> [-o OUTPUT] [--emit-only] [--dump-ast] [--verbose]
```

| Flag | Effect |
|------|--------|
| `-o <file>` | Output `.rs` path (default: input with `.rs` extension) |
| `--emit-only` | Write `.rs` only; skip the `rustc` invocation |
| `--dump-ast` | Print parsed AST to stdout and exit |
| `--verbose` / `-v` | Print per-stage timing and stats after transpilation |

`--verbose` output covers:
- Source line/byte/blank/comment counts
- Token count + lex time
- SUB/FUNCTION/statement counts + parse time
- Global symbol/SHARED/CONST/DATA counts + analyze time
- Emitted Rust lines/bytes/expansion ratio + emit time
- Total pipeline time

`qbc` auto-locates the runtime rlib by inspecting its own executable path
(`target/debug/` or `target/release/`), so the `rustc` invocation works
without manual `-L` flags when run via `cargo run`.

---

## Lexer (`src/lexer.rs`)

Converts raw source bytes to `Vec<Spanned<Token>>` where each token carries
its source line number for error reporting.

**Key behaviors:**

- **CP437 / Latin-1 safe** — source bytes are decoded as `b as char` so
  extended characters in old `.bas` files don't cause UTF-8 errors.
- **Case-insensitive** — all identifiers are uppercased for keyword lookup;
  original casing is preserved in `Ident(word)` for variable names.
- **Sigils** — `$`, `%`, `!`, `#`, `&` immediately following an identifier
  encode the QB type: `IdentStr`, `IdentInt`, `IdentSng`, `IdentDbl`.
- **REM is context-sensitive** — `REM` is a line comment only in *statement
  position*: start of input, after `Newline`, after `:`, or after a line
  number literal (`IntLit`/`FloatLit` directly following `Newline`/start).
  Mid-expression `rem` (e.g. `val = r * BASE`) falls through as a plain
  `Ident`. **Caution**: a variable literally named `rem` at statement-start
  (e.g. `rem = BASE MOD x`) is silently treated as a comment, discarding
  the line. QB programs should use `r` or another name instead.
- **`REM QBC` directives** — when a statement-position REM comment begins
  with the text `QBC`, the lexer emits a `Token::QbcDirective(String)` with
  the directive text instead of discarding the line. All other REM lines are
  still silently discarded. The parser collects these into
  `Program.directives`; the emitter acts on them.
  **Case handling**: the keyword token (e.g. `TITLE`) is uppercased for
  matching; the value portion (e.g. `My Game` in `REM QBC TITLE My Game`) is
  preserved with its original casing so window titles display correctly.
  Recognized directives: `FULLSPEED`, `FPS`, `SLOWMO`, `TITLE`, `SCALE`.
- **Hex/octal literals** — `&H1F` and `&O17` parsed with `from_fn` + `peek()`
  (not `take_while`) to avoid consuming the character after the literal.
- **Multi-statement lines** — `:` emits `Token::Colon`; the parser treats it
  as a statement separator equivalent to newline.
- **Line numbers** — integer literals at the start of a line are tokenized
  normally; the parser recognizes them as `Stmt::Label`.

---

## Parser (`src/parser.rs`)

Recursive-descent parser producing an untyped AST.

### Key types

```rust
pub struct Program {
    pub subs:       Vec<SubDef>,
    pub functions:  Vec<FuncDef>,
    pub main_body:  Vec<Stmt>,
    pub type_defs:  HashMap<String, Vec<(String, QbType)>>,
    pub directives: Vec<String>,  // QBC pragma strings, e.g. "FULLSPEED"
}

pub enum Stmt {
    Let { lvalue, expr },
    Print { args, newline, separator },
    If { cond, then_body, elseif_branches, else_body },
    For { var, from, to, step, body },
    While { cond, body },
    Do { cond, check_at_top, body },
    Select { expr, cases },
    Gosub(String),              // label name
    GosubLine(u32),             // numeric line target
    Call { name, args },
    Dim { decls: Vec<VarDecl> },
    ReDim { decls },
    Data(Vec<String>),
    Read(Vec<LValue>),
    Line { ... },               // LINE (x1,y1)-(x2,y2),color[,BF]
    Circle { ... },
    Paint { x, y, fill, border },
    Pset { x, y, color, preset },  // preset=true for PRESET statement
    Screen(Expr),
    Color { fg, bg },
    Cls,
    Locate { row, col },
    Label(String),
    // ... and more
}

pub enum Expr {
    Num(f64),
    StrLit(String),
    Var(LValue),
    BinOp { op, lhs, rhs },
    UnOp { op, expr },
    Call { name, args },       // function call OR array index
    Point { x, y },
    // ...
}

pub struct VarDecl {
    pub name:      String,
    pub ty:        QbType,
    pub dims:      Vec<Expr>,   // upper bounds (array rank)
    pub dim_lower: Vec<Expr>,   // lower bounds (parallel; stored, not yet emitted)
    pub shared:    bool,
}
```

### Notable parsing rules

- **`DIM arr(lo TO hi)`** — lower bound stored in `dim_lower`; currently
  emission uses only upper bounds for allocation size (see emitter notes).
- **`LINE (x1,y1)-(x2,y2)`** — the `-` between coordinate pairs is NOT
  subtraction; the parser handles this as a special two-coordinate syntax.
- **`CIRCLE (x,y),r,c[,start,end,aspect]`** — aspect defaults to 0.8333 in
  SCREEN 7 (320×200) to produce circular output on non-square pixels.
- **`FOR/NEXT`** — counter variable at `NEXT` is optional and ignored.
- **`SELECT CASE`** — supports `CASE x`, `CASE x TO y`, `CASE IS > x`,
  `CASE ELSE`.
- **`COMMON [SHARED] varlist`** — parsed by `parse_common`, mirroring
  `parse_dim`; emits shared `Stmt::Dim` decls (single-module only — no `CHAIN`).
- **`STATIC varlist [AS type]`** (statement form inside a procedure) — parsed by
  `parse_static` into `Stmt::SharedDecl`, so the analyzer promotes the names to
  persistent `GameState` fields (see Analyzer). The `SUB … STATIC` *header
  suffix* is unrelated and is just skipped to end-of-line in `parse_sub`/`parse_function`.
- **`GET/PUT #n, rec, var`** — random-access record I/O. The record *variable*
  is parsed and discarded: without a `FIELD` layout the runtime can't map record
  bytes onto a TYPE variable, so the transfer is an in-session no-op (reads leave
  the target at its default). Graphics `GET (x1,y1)-(x2,y2),arr` / `PUT (x,y),arr`
  are the unrelated sprite forms.
- **`REM QBC <directive>`** — consumed by the parser without producing a
  `Stmt`; stored in `Parser.directives` and moved into `Program.directives`.

---

## Analyzer (`src/analyzer.rs`)

Single-pass over the AST that builds:

- **`global_scope: Scope`** — `HashMap<String, Symbol>` recording name, type,
  array rank, and SHARED flag for all variables declared at module level.
- **`data_store: Vec<String>`** — all DATA literal values flattened in order,
  consumed at runtime via an `AtomicUsize` pointer.
- **`data_labels: HashMap<String, usize>`** — maps label name → DATA store
  index for `RESTORE label`.
- **`consts: Vec<(String, f64)>`** — CONST declarations, constant-folded.
- **`labels: Vec<String>`** — all label names (populated but not yet consumed
  by the emitter, reserved for GOTO support).
- **`directives: Vec<String>`** — QBC pragma strings, passed through from
  `Program` unchanged.

**SHARED promotion** (`promote_shared_globals`): a bare `SHARED name` inside a
procedure (also how `STATIC` and `COMMON SHARED` are represented) flips the
`shared` flag on the matching module-level symbol so it lands in `GameState`. If
the name has **no** module-level `DIM` (e.g. mandel's `ColorRange`, set only via
a by-ref SUB param, or a `STATIC` local), a shared scalar `Symbol` is
**synthesized** so it still becomes a `GameState` field rather than silently
falling back to a fresh local. Arrays must be `DIM`'d to be SHARED, so an
unmatched name is always treated as a scalar.

A startup pass also **warns on duplicate numeric line numbers** (QB replaces the
first definition; the transpiler emits both — flagged on stderr).

---

## Emitter (`src/emitter.rs`)

The largest file (~5200 lines). Walks `AnalyzedProgram` and writes Rust source.

### Emitted file structure

```rust
// Generated by qbc
#![allow(non_snake_case, unused_variables, dead_code, unused_mut,
         unused_assignments, unused_parens, unreachable_code,
         non_upper_case_globals, const_item_mutation, clippy::all)]
use qbasic_runtime::*;

// DATA store (if program has DATA statements)
static __DATA: &[&str] = &["val1", "val2", ...];
static __DATA_PTR: std::sync::atomic::AtomicUsize = ...;

// GameState struct (holds all SHARED variables)
#[derive(Default)]
struct GameState { ... }

// SUBs
fn DrawCity(__rt: &mut Runtime, __gs: &mut GameState, ...) { ... }

// FUNCTIONs
fn Angle(__rt: &mut Runtime, __gs: &mut GameState, ...) -> f64 {
    let mut angle: f64 = 0.0;
    // ... body ...
    angle
}

fn main() {
    // Runtime::new() when no TITLE/SCALE directive is present.
    // Runtime::new_configured("My Game", 1920, 1200) when TITLE/SCALE present.
    let mut __rt = Runtime::new();
    __rt.set_fullspeed(true);   // REM QBC FULLSPEED
    __rt.set_fps(30.0_f64);     // REM QBC FPS 30
    __rt.set_slowmo(2.0_f64);   // REM QBC SLOWMO 2
    let mut __gs = GameState::default();
    // main body
}
```

### REM QBC directives

Pragma directives found during lexing are propagated through to the emitter
via `AnalyzedProgram.directives`. The emitter parses them with a small helper:

```rust
#[derive(Default)]
struct QbcConfig {
    fullspeed: bool,
    fps:       Option<f64>,    // "FPS 30"
    slowmo:    Option<f64>,    // "SLOWMO 2"
    title:     Option<String>, // "TITLE My Game" — value casing preserved
    scale:     Option<u32>,    // "SCALE 2"
}

fn parse_qbc_config(directives: &[String]) -> QbcConfig { ... }
```

`parse_qbc_config` is called from `emit()` and its result stored on `self.qbc`.
`emit_main()` consults `self.qbc` to decide whether to call `Runtime::new()`
or `Runtime::new_configured(title, win_w, win_h)` and which setter calls to
emit immediately after.

| Directive | Syntax | Effect |
|-----------|--------|--------|
| `FULLSPEED` | `REM QBC FULLSPEED` | Emits `__rt.set_fullspeed(true);`. Disables the `auto_present()` frame throttle; program runs at full native speed. Useful for compute-heavy programs (pi.bas, mandel.bas). |
| `FPS` | `REM QBC FPS 30` | Emits `__rt.set_fps(30.0_f64);`. Caps frame rate at N fps (default 60). Affects the `auto_present()` poll interval. |
| `SLOWMO` | `REM QBC SLOWMO 3` | Emits `__rt.set_slowmo(3.0_f64);`. Multiplies all QB `SLEEP` durations by N. Good for slowing down timed animations. |
| `TITLE` | `REM QBC TITLE My Game` | Uses `Runtime::new_configured("My Game", ...)` instead of `Runtime::new()`. Sets the minifb window title. Value casing is preserved. |
| `SCALE` | `REM QBC SCALE 2` | Uses `Runtime::new_configured(..., 1920, 1200)`. Multiplies the default 960×600 window size by N. Useful on HiDPI displays. |

TITLE and SCALE affect window creation, which cannot be changed after the
window opens. When either is present the emitter calls `new_configured()`
instead of `new()`. When both are present they combine:
```basic
REM QBC TITLE Gorilla Wars
REM QBC SCALE 2
```
emits `Runtime::new_configured("Gorilla Wars", 1920, 1200)`.

Directives are case-insensitive and may appear anywhere in the file.

### SHARED variables → GameState

All `DIM x SHARED` variables are collected into a generated `GameState`
struct. Every SUB and FUNCTION receives `__gs: &mut GameState`. Main body
accesses them as `__gs.varname`.

### Array allocation

`DIM arr(N)` → `vec![0.0_f64; (N + 1) as usize]`

The `+1` matches QB's 1-based default (`OPTION BASE 1`). QB arrays are
0-indexed in the allocation but accessed at index 1..=N, so allocating N+1
elements ensures index N is in bounds.

`DIM arr(1 TO N)` — lower bound is parsed and stored in the AST
(`dim_lower`) but currently not used in emission. Allocation uses upper bound
only: `(N + 1) as usize`. This works for Gorillas; general lower-bound
support is future work.

### Array indexing

`arr(i)` emits `arr[(i) as usize]` — raw QB index without subtracting the
lower bound. This is consistent between caller and callee (arrays passed as
`&mut Vec<f64>` don't carry lower-bound metadata).

### GOSUB → named functions

The emitter pre-scans the main body for all `Gosub` targets and emits each
as a named Rust function rather than inline code. This produces clean,
readable output for programs like Gorillas that use GOSUB exclusively.
`collect_gosub_targets` recurses into IF branches, ELSEIF, SELECT CASE,
FOR, WHILE, DO bodies.

### GOTO → `__pc` state machine

When the main body contains any `Stmt::Goto` after GOSUB-block extraction,
the emitter switches to a state-machine mode instead of linear emission:

```rust
let mut __pc: u32 = {first_line};
'__sm: loop {
    match __pc {
        1000 => { /* stmts for line 1000 */ __pc = 1010; continue '__sm; }
        1010 => { /* stmts for line 1010 */ __pc = 1020; continue '__sm; }
        // ...
        _ => break '__sm,
    }
}
```

**Trigger**: `has_goto` flag set if any statement in main_body is
`Stmt::Goto` after `extract_gosub_blocks()` runs.

**Block flattening**: `flatten_to_blocks()` walks the statement list and
splits at every integer `Stmt::Label`, producing `(pc: u32, Vec<Stmt>)`
pairs. Statements before the first label go in a synthetic `pc=0` block.

**Per-arm emission**: within each arm, `Stmt::Goto(label)` emits
`{ __pc = label; continue '__sm; }`. Single-line `IF … THEN GOTO n` emits
`if cond { __pc = n; continue '__sm; }` with fall-through to the next arm.

**Multi-line FOR loops**: when a `FOR` and its `NEXT` land in different
match arms (cross-arm loop), the emitter hoists `__for_to_var` and
`__for_step_var` as locals, emits the range check and branch at the FOR
arm, and emits the increment and back-branch at the NEXT arm.

**GOSUB inside state machine**: calls extracted GOSUB targets as normal Rust
functions; `RETURN` in the main body emits `break '__sm`.

Used by: **donkey.bas** (46 line-number labels, 14 GOTO targets).

### Array pass-by-reference

QB passes arrays by reference with the `arr()` syntax. The emitter maps this:

- **Shared array** (`SHARED`) → `&mut __gs.arrname`
- **Local array** → `&mut arrname`
- **Array parameter** (already `&mut Vec<f64>`) → `&mut *arrname` (explicit
  reborrow; avoids `&mut &mut Vec<f64>`)
- **Typed arrays** (UDT fields) → expanded to per-field `&mut` arguments

**Alias detection**: when the same array appears in multiple argument positions
of one call (e.g. `CALL SubArr(cells, arr(), tmp(), arr())`), the emitter
pre-scans for duplicates, emits `let mut __arr_alias_N = arr.clone()` for
all but the last occurrence, and passes the real reference only for the
final (output) position. This avoids Rust's double-mutable-borrow error.

### Special built-in dispatch

| QB call | Emits | Notes |
|---------|-------|-------|
| `INKEY$` | `__rt.inkey()` | Needs Runtime |
| `RND` | `__rt.rnd()` | Needs Runtime |
| `STRING$(n, c)` | `qb_string(n, c)` | c is f64 char code |
| `STRING$(n, s$)` | `qb_string_s(n, &s)` | c is string → first char |
| `MID$(s,p)` | `qb_mid(&s, p, None)` | 2-arg form |
| `MID$(s,p,n)` | `qb_mid(&s, p, Some(n))` | 3-arg form |
| `INSTR(s1,s2)` | `qb_instr(1.0, &s1, &s2)` | 2-arg → prepend start=1 |
| `SLEEP n` | `__rt.sleep(n)` | Not a free fn |
| `END` / `STOP` | `__rt.quit()` | Waits for key before exit |

### `lift_expr` vs `emit_expr`

`emit_expr_inner` is a pure expression emitter (takes `&self`, no side
effects). `lift_expr` takes `&mut self` and can emit `let __tmp_N = ...`
bindings for user-defined function calls that appear inside `__rt.method()`
argument lists, preventing double-borrow of `__rt`.

---

## Runtime (`runtime/src/`)

Two files totaling ~3700 lines, linked by every transpiled program.

### `lib.rs` (~3400 lines)

Everything except sound: Runtime struct, graphics primitives, I/O,
math/string functions, window management.

### `sound.rs` (~300 lines)

QB-compatible PLAY / SOUND / BEEP implementation via `rodio`.

- **`MmlState`** — persists across PLAY calls (octave, length, tempo, style,
  background flag). Stored as a field on Runtime so QB's `MB`/`MF` mode and
  tempo settings survive between separate `PLAY` calls.
- **`parse_mml(mml, state)`** — full QB MML parser: notes A–G with
  accidentals, octave commands O/</>, length L, tempo T, rest P, absolute
  note N, style MN/ML/MS, mode MB/MF.
- **`events_to_pcm(events)`** — synthesizes a sine-wave PCM buffer at 44100 Hz
  with 8ms fade-in/out per note to avoid clicks. Amplitude 0.25.
- **`play_events_blocking(events)`** — foreground playback via `rodio`;
  blocks until audio finishes.
- **`play_events_background(events)`** — spawns a thread; returns immediately.
  `OutputStream` is created per-call (not Send, can't be stored globally).
- **`play_sound(freq, ticks)`** — SOUND statement; duration = ticks/18.2 s.
- **`play_beep()`** — 800 Hz for ~220 ms.

### Runtime struct

```rust
pub struct Runtime {
    // Text rendering state
    pub fg_color:    u8,        // current foreground palette index (0–15)
    pub bg_color:    u8,        // current background palette index
    cursor_row:      usize,     // 1-based text row
    cursor_col:      usize,     // 1-based text col

    // Screen / framebuffer
    pub screen_mode: u8,        // 0=text, 7=320×200, 9=640×350, etc.
    pub width:       u32,       // framebuffer pixel width
    pub height:      u32,       // framebuffer pixel height
    char_w:          u32,       // character cell width (always 8)
    char_h:          u32,       // character cell height (8 or 16)
    fb:              Vec<u8>,   // palette-indexed pixels (one byte per pixel)
    palette_rgb:     [(u8,u8,u8); 16],  // remappable via PALETTE statement

    // Window
    window:          Option<minifb::Window>,
    win_w:           usize,     // window pixel width  (default 960, scaled by REM QBC SCALE)
    win_h:           usize,     // window pixel height (default 600, scaled by REM QBC SCALE)
    last_present:    Instant,
    pset_counter:    u32,       // throttle for auto_present
    fullspeed:       bool,      // when true, skip auto_present() throttle
    frame_interval_ms: u64,     // auto_present() poll interval in ms (default 16 ≈ 60fps)
    slowmo:          f64,       // SLEEP duration multiplier (default 1.0)

    // Key input
    key_queue:       VecDeque<String>,  // harvested QB key strings

    // RNG
    rng:             u32,       // LCG state matching QB's generator

    // VIEW / WINDOW logical coordinate system
    view_x1..view_active, win_x1..win_active,
    gfx_x, gfx_y,          // graphics cursor in logical coords

    // DRAW state
    draw_scale:      f64,       // S value; pixels_per_unit = draw_scale / 4
    draw_color:      u8,        // C value (current DRAW color)

    // PLAY MML state
    mml_state:       MmlState,  // persists across PLAY calls
}
```

### Window management

A single `minifb` window opens at startup and stays open for the entire
program lifetime — both text and graphics modes render into it.

**Default window size**: 960×600. Overridable with `REM QBC SCALE N`
(multiplies to 1920×1200 for N=2, etc.) or `REM QBC TITLE text` (both use
`Runtime::new_configured(title, win_w, win_h)` at construction time).

**`Runtime::new_configured(title, win_w, win_h)`** — primary constructor.
`Runtime::new()` is now a convenience wrapper: `new_configured("QBasic", 960, 600)`.

**Software scaling**: `present()` nearest-neighbor scales the logical
framebuffer (any size: 640×400 for text, 320×200 for SCREEN 7, etc.) into
`self.win_w × self.win_h`. Effective scale factors with default window:

| Mode | FB size | Window | Scale |
|------|---------|--------|-------|
| Text (mode 0) | 640×400 | 960×600 | 1.5× |
| SCREEN 1 / 7 / 13 | 320×200 | 960×600 | 3× |
| SCREEN 8 | 640×200 | 960×600 | 1.5× / 3× |
| SCREEN 9 | 640×350 | 960×600 | ~1.7× |
| SCREEN 12 | 640×480 | 960×600 | 1.5× / 1.25× |

All modes use a **16-color** EGA palette; `SCREEN 13`'s 256 colors are **not**
supported (pixels are clamped `% 16`). `present()` rounds logical→framebuffer
coordinates to the nearest pixel (not truncation) so non-integer
`WINDOW`/`PMAP` mappings don't drop scanlines — see Graphics primitives.

Switching `SCREEN` modes resizes `fb` and changes `char_h` but never
closes or reopens the window.

### Frame pacing, FULLSPEED, FPS, and SLOWMO

`auto_present()` is called by `pset()` after every write. It increments a
counter and, every 256 psets, checks whether `frame_interval_ms` ms has
elapsed since the last `present()` call. If so, it blits the framebuffer to
the window. This keeps animation loops smooth without explicit `present()`
calls in the emitted code.

**`REM QBC FULLSPEED`** — sets `fullspeed = true`. `auto_present()` returns
immediately without any throttling. Graphics still appear on screen via
explicit `present()` calls at PRINT, INKEY$, CLS, etc. — only the per-pixel
rate limiting is skipped. Useful for computation-heavy programs (mandel.bas,
pi.bas) where the 16ms cap otherwise limits throughput.

**`REM QBC FPS N`** — calls `set_fps(N)` which sets
`frame_interval_ms = (1000.0 / N) as u64`. Default is 16ms ≈ 60fps.
Cap at a lower fps (e.g. 10) to slow down graphics-heavy programs visually.

**`REM QBC SLOWMO N`** — calls `set_slowmo(N)` which sets `slowmo = N`.
All `Runtime::sleep()` calls multiply their duration by `slowmo`. Default 1.0.
Useful for slow-motion inspection of timed animations that use QB `SLEEP`.

### Key input — the key_queue

`minifb`'s `get_keys_pressed(KeyRepeat::No)` only reports a key as
"newly pressed" for **one** `update_with_buffer` call. If `present()` is
called between a keypress and `inkey()` (e.g. by a PRINT statement inside
an animation loop), the "newly pressed" window is consumed and the key is
lost.

**Fix**: every call to `present()` harvests `get_keys_pressed` immediately
after `update_with_buffer` and appends results to `key_queue`. `inkey()`
then pops from the queue rather than calling `get_keys_pressed` directly.
Keys are never lost regardless of how many `present()` calls intervene.

**`INKEY$` cost + minifb rate limiter.** minifb's built-in frame-rate limiter
(default 250 FPS = 4 ms) sleeps inside *both* `update()` and
`update_with_buffer()`. A program that polls `INKEY$` per pixel (mandel:
`IF INKEY$ <> "" THEN END`, ~73 k calls) would therefore sleep ~73 k × 4 ms ≈
5 minutes, and rebuild the 960×600 frame on every call. Two-part fix:

1. **`set_target_fps(0)` at window creation** disables minifb's limiter
   entirely — the runtime does its own pacing via `frame_interval_ms`.
2. **`inkey()` self-throttles the blit**: it does a full `present()` at most
   once per `frame_interval_ms` and otherwise calls `pump_events()` — a cheap
   `win.update()` + key harvest with no framebuffer rebuild. Progressive
   rendering stays visible (~60 fps) while the per-poll cost collapses.

Trade-off: with minifb's limiter off, a pure idle `DO … LOOP WHILE INKEY$ = ""`
busy-spins at 100 % CPU until a key arrives (DOS-faithful, and the correct trade
for native-speed renders).

### Framebuffer text rendering

All text (PRINT, INPUT prompts, LOCATE output) renders into the
palette-indexed framebuffer using a baked-in IBM PC 8×8 bitmap font
(`FONT_8X8: [[u8; 8]; 128]`). This means text is visible in both text
and graphics modes without switching rendering paths.

Scrolling: when `cursor_row` exceeds `height / char_h`, the framebuffer is
`copy_within`'d upward by one character row and the bottom row is cleared
to `bg_color`.

### present() call sites

`present()` is called from:
- Every `print_gfx()` call (PRINT, LOCATE, INPUT prompt)
- `inkey()` — but **throttled to once per `frame_interval_ms`**; other polls use
  the cheap `pump_events()` (event poll + key harvest, no blit). See key input.
- `cls()` — makes clear visible immediately
- `auto_present()` — called by `pset()` every 256 pixels, rate-limited to
  16ms, keeps animation loops responsive without explicit present() calls
  (skipped entirely when `fullspeed = true`)
- `sleep()` — shows current frame before sleeping
- `input_line()` — every 16ms during blocking input (for cursor blink)
- `tick()` — called by flood fill every 2000 iterations to keep window alive
- `wait_for_key()` / `quit()` — end-of-program hold

### Program termination

`END` / `STOP` emit `__rt.quit()` (NOT `std::process::exit(0)`).
`quit()` calls `wait_for_key()` which:
1. Prints `[Press any key to exit]` at the bottom of the screen in dark grey
2. Loops at 16ms presenting the framebuffer until any key or window close
3. Then calls `std::process::exit(0)`

Programs that fall off the end of `main()` without `END` get the same
behavior through the `Drop` impl.

### Graphics primitives

| QB statement | Runtime method | Notes |
|---|---|---|
| `PSET (x,y), c` | `rt.pset(x, y, c)` | Writes palette index |
| `PRESET (x,y)` | `rt.pset(x, y, bg)` | Resets to bg_color |
| `LINE (x1,y1)-(x2,y2),c` | `rt.line(...)` | Bresenham |
| `LINE ...,B` | `rt.line_box(...)` | Rectangle outline |
| `LINE ...,BF` | `rt.line_box_fill(...)` | Filled rectangle |
| `CIRCLE (x,y),r,c` | `rt.circle(...)` | Midpoint ellipse |
| `PAINT (x,y),fill,border` | `rt.paint(...)` | Flood fill (stack-based) |
| `POINT(x,y)` | `rt.point(x, y)` | Returns palette index as f64 |
| `GET (x1,y1)-(x2,y2),arr` | `rt.get_sprite(...)` | EGA planar capture |
| `PUT (x,y),arr,PSET\|XOR` | `rt.put_sprite(...)` | EGA planar blit |
| `DRAW string` | `rt.draw(...)` | Full MML-style DRAW interpreter |

`POINT(x,y)` returning the raw palette index is critical for Gorillas
collision detection: `IF POINT(BX, BY) <> BACKCOLOR THEN 'hit`.

**Coordinate rounding** (`logical_to_fb`): under a `VIEW`/`WINDOW` transform,
logical coordinates are mapped to framebuffer pixels and **rounded to the
nearest pixel** (`.round()`), not truncated via `as i32`. The `PMAP`/`WINDOW`
round-trip yields values like `5.99999999` that must map to pixel 6; truncation
dropped whole scanlines, which showed as horizontal black gaps in
line-per-scanline renderers like mandel.bas. This is also the QB-correct mapping
(nearest pixel) and keeps `POINT` reads consistent with `PSET`/`LINE` writes.

**Flood fill**: `PAINT(x, y, fill, border)` — fill color of `-1` uses the
current `draw_color` (set by `DRAW "Cn"`). Flood fill marks each pixel as
the fill color *before* pushing it to the stack, preventing duplicate stack
entries and infinite loops on convex shapes.

**PRESET vs PSET**: `PRESET` resets the pixel to `bg_color`; `PSET` sets it
to the specified or current foreground color. Handled via the `preset: bool`
field on `Stmt::Pset` — the emitter passes `__rt.bg_color as f64` for PRESET.

**CIRCLE aspect ratio**: `aspect = 0.8333` for SCREEN 7/1 corrects for the
non-square pixel geometry of 320×200 EGA, making circles appear round.

**DRAW statement**: full QB DRAW MML interpreter supporting commands
`U/D/L/R/E/F/G/H` (directional moves), `M` (absolute/relative move),
`B` (blank — move without drawing), `N` (no-update — draw and return),
`S` (scale), `C` (color), `X` (execute substring), `A` (angle), `P` (paint).

### EGA Palette

```rust
pub const EGA: [(u8, u8, u8); 16] = [
    (0,0,0),       (0,0,170),     (0,170,0),     (0,170,170),
    (170,0,0),     (170,0,170),   (170,85,0),     (170,170,170),
    (85,85,85),    (85,85,255),   (85,255,85),    (85,255,255),
    (255,85,85),   (255,85,255),  (255,255,85),   (255,255,255),
];
```

`PALETTE attr, color64` remaps entries using the EGA 64-color encoding
(bits [5:4:3] = RGB high, bits [2:1:0] = RGB low).

### Arithmetic operators

Most binary operators map directly to Rust on `f64` (`+ - * /`). Three need
QB-specific handling and are emitted as runtime helper calls rather than inline
Rust:

| QB | Emitted as | Notes |
|----|------------|-------|
| `a ^ b` | `a.powf(b)` | Float power. Unary minus binds *looser* than `^` (`-2^2 = -4`), handled in the parser. |
| `a \ b` | `qb_idiv(a, b)` | Integer division: both operands are **CINT-rounded to integers first** (banker's), then divided with truncation toward zero. |
| `a MOD b` | `qb_mod(a, b)` | Both operands CINT-rounded first, then remainder. Sign follows the **dividend** (same as Rust `%` on integers). |

`qb_idiv`/`qb_mod` exist because QB rounds operands to integers before the
operation: `2.7 MOD 2` is `CINT(2.7) MOD 2 = 3 MOD 2 = 1`, not `0.7`; and
`2.6 \ 1` is `3 \ 1 = 3`, not `2`. Emitting inline `as i64` / float `%` would
truncate operands and leak fractional results.

### Math and string functions

All exposed as free functions in the `use qbasic_runtime::*` glob:

| QB | Rust | Notes |
|----|------|-------|
| `INT(x)` | `qb_int(x)` | `floor()` |
| `FIX(x)` | `qb_fix(x)` | `trunc()` |
| `CINT(x)` | `qb_cint(x)` | Banker's rounding (ties to **even**), not Rust's `round()` |
| `ABS(x)` | `qb_abs(x)` | |
| `SQR(x)` | `qb_sqr(x)` | `sqrt()` |
| `SGN(x)` | `qb_sgn(x)` | Returns 0.0 for zero (not Rust's signum) |
| `NOT x` | `qb_not(x)` | Bitwise NOT of integer part: `(!(v as i64)) as f64` |
| `x AND y` | `qb_and(x,y)` | Bitwise |
| `x OR y` | `qb_or(x,y)` | Bitwise |
| `x XOR y` | `qb_xor(x,y)` | Bitwise |
| `SIN/COS/TAN/ATN` | `qb_sin/cos/tan/atn` | Radians; ATN = atan (not atan2) |
| `TIMER` | `qb_timer()` | Seconds since midnight as f64 |
| `LEN(s)` | `qb_len(&s)` | Char-based (not byte-based) |
| `LEFT$(s,n)` | `qb_left(&s, n)` | Char-based |
| `RIGHT$(s,n)` | `qb_right(&s, n)` | Char-based |
| `MID$(s,p,n)` | `qb_mid(&s, p, Some(n))` | 1-indexed, char-based |
| `UCASE$/LCASE$` | `qb_ucase/lcase(&s)` | |
| `LTRIM$/RTRIM$` | `qb_ltrim/rtrim(&s)` | |
| `STR$(n)` | `qb_str_fn(n)` | Leading space for positives |
| `VAL(s)` | `qb_val(&s)` | Parses the longest valid numeric prefix (sign, digits, `.`, `e`/`E` exponent); a sign/exponent char is only consumed in its grammatical position. `VAL("1-2")=1`, `VAL("12e")=12` |
| `CHR$(n)` | `qb_chr(n)` | |
| `ASC(s)` | `qb_asc(&s)` | |
| `INSTR(s1,s2)` | `qb_instr(1.0,&s1,&s2)` | 1-indexed |
| `STRING$(n,c)` | `qb_string(n, c)` | c = char code |
| `STRING$(n,s$)` | `qb_string_s(n, &s)` | c = string → first char |
| `SPACE$(n)` | `qb_space(n)` | |
| `HEX$(n)` | `qb_hex(n)` | |
| `OCT$(n)` | `qb_oct(n)` | |
| `PRINT USING` | `qb_print_using(fmt, &[vals])` | `#`, `##.##`, `,` supported |

String functions are **char-based** throughout (using `.chars()` iteration),
matching QB's behavior for multi-byte Unicode edge cases.

### RNG

LCG matching QB's internal generator:
```rust
self.rng = self.rng.wrapping_mul(214013).wrapping_add(2531011);
((self.rng >> 16) & 0x7FFF) as f64 / 32768.0
```
`RANDOMIZE seed` sets `self.rng = seed.abs() as u32`.

### Sound

Wired to `rodio` via `runtime/src/sound.rs`. See sound.rs section above.

---

## Design Decisions

### All numerics are f64

QB's default is SINGLE (f32). We use f64 throughout. Integer variables (`%`
sigil) are also stored as f64. This simplifies the type system at the cost
of minor precision differences that don't affect Gorillas.

### QB boolean semantics

```
0.0  = false
-1.0 = true   (bitwise NOT of 0 in two's complement)
```

All comparisons emit `qb_from_bool(expr)` → `-1.0` or `0.0`.
All conditionals wrap in `qb_bool(v)` → `v != 0.0`.
Never emit bare Rust `bool` for a QB comparison result.

### NOT is bitwise, not logical

`NOT x` in QB is `~(x as integer)`, not `!x`. `NOT 0 = -1`, `NOT -1 = 0`,
`NOT 2 = -3`. Emits `qb_not(x)` = `(!(v as i64)) as f64`.

### 1-indexed arrays

QB default is `OPTION BASE 1`. `DIM arr(N)` allocates `N+1` elements so
index N is valid. Indices are used raw: `arr[(i) as usize]`.

### GOSUB targets → Rust functions

When GOSUB targets are only reachable via GOSUB (not GOTO), they are emitted
as named Rust functions. This is the clean path and covers 100% of
GORILLAS.BAS. Programs using GOTO fall back to a `match __pc` state machine.

### Emitted allow list

Every `.rs` file begins with a broad `#![allow(...)]` covering:
`non_snake_case`, `unused_variables`, `dead_code`, `unused_mut`,
`unused_assignments`, `unused_parens`, `unreachable_code`,
`non_upper_case_globals`, `const_item_mutation`, `clippy::all`.

These suppressions are intentional — QB programs routinely have patterns
(uppercase names, defensive parens, unreachable else-branches) that Rust
considers bad style.

---

## Building

```bash
# Build the transpiler
cargo build -p qbasic-transpiler

# Build the runtime
cargo build -p qbasic_runtime

# Transpile + compile a .bas file (qbc auto-invokes rustc)
cargo run -- basic-src/gorilla.bas -o bin/gorilla.rs
./bin/gorilla

# Transpile only (inspect the .rs output)
cargo run -- basic-src/gorilla.bas -o bin/gorilla.rs --emit-only

# Release build (faster binary, same correctness)
cargo build --release -p qbasic_runtime
cargo run --release -- basic-src/gorilla.bas -o bin/gorilla.rs --emit-only
rustc bin/gorilla.rs --edition 2021 \
  -L target/release/deps \
  --extern qbasic_runtime=target/release/libqbasic_runtime.rlib \
  -C opt-level=3 \
  -o bin/gorilla

# Build all .bas files
for bas in basic-src/*.bas; do
    name=$(basename "$bas" .bas)
    cargo run -q -- "$bas" -o "bin/${name}.rs" --emit-only
    rustc "bin/${name}.rs" --edition 2021 \
        -L target/debug/deps \
        --extern qbasic_runtime=target/debug/libqbasic_runtime.rlib \
        -o "bin/${name}"
done

# Verbose stats
cargo run -- basic-src/gorilla.bas --emit-only --verbose
```

---

## Milestone Status

### M11 — More DOS programs: torus, donkey, reversi ✅
Expanded coverage to the remaining graphics-heavy DOS QBasic programs.

- **donkey.bas** (CGA SCREEN 1): authentic 2-bpp packed-INTEGER sprite GET/PUT
  layout; PUT action verbs (PSET/PRESET/AND/OR/XOR, default XOR); DRAW fixes
  (M-command relativity, N no-advance, color-follows-COLOR). See `donkey.md`.
- **torus.bas** (SCREEN 12): arrays of a user TYPE flattened to per-field Vecs;
  `SHARED … AS type` inside SUBs; `PAINT STEP`; typed-array element passed to a
  SUB; **FUNCTION parameters pass by reference** (QB semantics — `Inside()`
  mutates a TYPE arg the caller reads back); `WINDOW`-without-`VIEW` maps to the
  full screen; **Y-axis inversion** for Cartesian `WINDOW`; SCREEN 11/12 PALETTE
  decodes the 18-bit VGA DAC value. See `torus.md`.
- **reversi.bas** (SCREEN 9): `WINDOW SCREEN` (screen-orientation, magnitude-
  mapped so reversed corners don't flip); `ERASE`; **3-D arrays**
  (`nested_vec_type`/`nested_vec_init`); 2-D arrays of a TYPE; shared-field args
  to user FUNCTIONs hoisted to avoid borrow conflicts; scalar/array same-name
  coexistence (`A$` vs `A$()`). See `reversi.md`.
- **screen13.bas / 256c / palette256_expanded** (SCREEN 13, MCGA 256-color):
  256-entry `palette_rgb`, VGA BIOS default palette, 18-bit DAC `PALETTE`.

Verified headlessly via a `Runtime::fb_stats()` diagnostic (non-background pixel
+ distinct-color counts) where the program needs interactive input.

### M1 — Text programs ✅
LET, PRINT, INPUT, IF/THEN/ELSE/ELSEIF, FOR/NEXT, WHILE/WEND, DO/LOOP,
GOSUB/RETURN, SUB/END SUB, FUNCTION/END FUNCTION, DECLARE, SELECT CASE,
DIM, REDIM, DATA/READ/RESTORE, END, STOP.

### M2 — Graphics / GUI ✅
SCREEN, COLOR, CLS, LOCATE, LINE (plain/B/BF), CIRCLE, PAINT, PSET, PRESET,
VIEW, WINDOW, PALETTE, GET, PUT.
minifb window open at startup; text and graphics share one framebuffer;
software nearest-neighbor scaling to fixed 960×600 window.

### M3 — Full game loop ✅
POINT() collision, RANDOMIZE, TIMER, INKEY$ (with key_queue fix),
frame pacing via auto_present, SLEEP, END/STOP wait-for-key.
VIEW/WINDOW logical coordinate system, PMAP, PALETTE USING, relative LINE,
SHARED-inside-SUB promotion to GameState.
DRAW statement (full MML-style interpreter: directions, scale, color, paint).
Test programs: mandel.bas (Mandelbrot renderer), money.bas, sortdemo.bas.

### M4 — GOTO state machine ✅
`__pc` match-loop state machine emitter for line-numbered BASIC programs.
Multi-line FOR/NEXT across arms, GOSUB extraction within state machine,
`parse_single_line_body` line-boundary detection fix.
PRESET bug fix (now correctly writes bg_color, not fg_color).
PAINT fill color fix (-1 uses draw_color, not EGA index 15).
Test program: donkey.bas (46 line labels, 14 GOTO targets).

### M5 — Sound ✅
PLAY MML wired to rodio: notes A–G, octave, length, tempo, rests, note
numbers, MB (background) / MF (foreground) mode. MML state persists across
PLAY calls via `mml_state` field on Runtime.
SOUND (frequency + ticks), BEEP (800 Hz, ~220ms).
Sine-wave PCM synthesis at 44100 Hz with 8ms fade-in/out per note.

### M6 — Pragmas and polish ✅
`REM QBC` directive system implemented end-to-end (lexer → parser → analyzer → emitter → runtime):
- `FULLSPEED` ✅ — disables `auto_present()` throttle for compute-heavy programs (pi.bas, mandel.bas)
- `FPS N` ✅ — cap frame rate at N fps via configurable `frame_interval_ms`
- `SLOWMO N` ✅ — multiply QB `SLEEP` durations by N via `slowmo` field
- `TITLE text` ✅ — set minifb window title at construction time via `new_configured()`
- `SCALE N` ✅ — multiply default 960×600 window to N×960 × N×600 via `new_configured()`

### M10 — VIEW PRINT text viewport + CLS argument ✅
`VIEW PRINT top TO bot` restricts the text scrolling region and sets the `vp_top`/
`vp_bot` fields on Runtime (1-based rows). `VIEW PRINT` bare resets to full screen.
`CLS [arg]`: 0 or no arg = full framebuffer clear (previous behaviour); 2 = clear
only the active viewport rows. `scroll_if_needed` scrolls within `vp_top..=vp_bot`.
Fixes gorilla.bas `GorillaIntro` (draws gorillas correctly between `VIEW PRINT 9 TO 24`
/ `CLS 2` pairs) and money.bas `FancyCls` (clears body area while preserving row 1).

### M9 — ON ERROR GOTO / RESUME ✅
`ON ERROR GOTO label` parsed into `Stmt::OnError`; named (non-numeric) labels
are extracted as GOSUB-style handler functions and called via `emit_error_dispatch()`
after fallible statements (OPEN, SCREEN). `RESUME` / `RESUME NEXT` parsed into
`Stmt::Resume`; both emit `__rt.error_pending = false` (RESUME-retry is treated
as RESUME NEXT since retrying the faulting statement requires coroutine machinery).
`ERR` system variable emits `__rt.err_code`; set to 53 on file-not-found failures.
Works for gorilla.bas (screen mode negotiation), money.bas (file-not-found → case 53
initialises new data file), and any program with named ON ERROR handlers.
Numeric-line-label handlers (e.g. `ON ERROR GOTO 1295`) in state-machine programs
remain in SM match arms; the error flag is cleared gracefully without dispatch.

### M8 — File I/O ✅
Sequential files (`FOR INPUT/OUTPUT/APPEND`): `OPEN`, `CLOSE`, `INPUT #n`,
`LINE INPUT #n`, `PRINT #n`, `WRITE #n`. Random-access files (`FOR RANDOM`):
`FIELD`, `GET #n`, `PUT #n`, `LSET`, `RSET`. Binary type conversions: `MKD$`,
`MKI$`, `MKS$`, `MKL$`, `CVD`, `CVI`, `CVS`, `CVL` (IEEE 754 little-endian).
`qb_lset`/`qb_rset` free functions pad/truncate to field length.
`qb_field_get`/`qb_field_put` slice record buffers for GET/PUT.
FIELD variable declarations collected into `emit_locals` so field vars are
pre-declared as `String` locals. Test program: money.bas (Microsoft 1990).

### M7 — User-defined TYPE completeness ✅
Recursive TYPE flattening (`flatten_type_fields`): nested TYPEs to arbitrary
depth (`Outer.Middle.Inner.Val`), 1-D / 2-D arrays of TYPEs (incl. nested),
string fields, scalar TYPE vars, scalar TYPE params to SUBs (byref, expanded
to per-field `&mut` args), typed-array params, whole-record copy, field-level
swap. Keyword-named types (`TYPE Color`) parse via `advance_as_type_ident`.
Tests: `type_nested`, `type_complex`.

---

## What's Left

Of the bundled DOS QBasic programs in `basic-src/`, **all but one transpile,
compile, and render**: gorilla, torus, reversi, mandel, donkey, nibbles,
sortdemo, money, pi, pi-gw, primes, hangman, hangman-gw, q_sort, fuzzbuzz,
hello-world, sound, step, screen13, 256c, palette256_expanded. The current
integration suite is **27/27**, with 68 runtime unit tests.

Remaining work is verification and a few rarely-used features:

1. **qblocks.bas** — the one bundled program that does not yet transpile (a Tetris
   clone; not yet analyzed for the missing feature(s)).
2. **gorilla.bas full playthrough (prime target)** — compiles and links;
   needs interactive + visual + audio verification of a complete game:
   skyline render, banana physics, POINT() collision, explosion sound, scoring,
   wind. The one acceptance test that can't be checked headlessly.
3. **PAINT tiling patterns** — `PAINT (x,y), CHR$(n), border` (B&W dither fill)
   emits a solid-foreground stub + warning, not real pattern tiling. Dead code
   on reversi's EGA path; no color-mode program needs it.
4. **Array fields inside a TYPE body** — `Bar(10) AS SINGLE` within a `TYPE`
   block: the parser discards the dimension. Rare; no bundled program uses it.
5. **`PRINT USING` floating tokens** — `$$` (floating dollar) and `**`
   (asterisk fill) print literally. `^^^^` scientific notation and wide-field
   `%` overflow are implemented (see Feature Support Notes).
6. **Unify `REM QBC` pragmas and `QBC_*` env vars (idea — to review).** The
   source pragmas (`FULLSPEED/FPS/PACE/SLOWMO/TITLE/SCALE`, via
   `parse_qbc_config` in `emitter.rs`) are baked in at transpile time; the
   headless-driver env vars (`HEADLESS/KEYS/SEED/DUMP/CHECKSUM/FBSTATS/
   EXIT_AFTER`, read by `runtime/src/lib.rs`) are read at run time. The valuable
   half is one-directional: let the **behavioral** pragmas also be set/overridden
   by an env var (`QBC_PACE`, `QBC_FPS`, `QBC_SCALE`, `QBC_FULLSPEED`,
   `QBC_SLOWMO`, `QBC_TITLE`) so they can be tuned without re-transpiling — low
   effort, since the runtime already exposes the `set_*` methods; read env after
   the pragma-emitted calls so env wins. The reverse (debug knobs as pragmas) is
   mostly **not** worth it: `HEADLESS/KEYS/DUMP/CHECKSUM/FBSTATS/EXIT_AFTER` are
   external observation/test knobs, not program behavior, and `SEED`-as-pragma
   would defeat real randomness. So: do the env-override half if/when useful;
   skip full bidirectional unification. Needs a clear precedence rule (env
   overrides pragma) given the shared "QBC" name.

---

## GORILLAS.BAS Specifics

- **SCREEN 7** — 320×200, 16 EGA colors. `aspect=0.8333` for CIRCLE.
- **No GOTO** — entire program is GOSUB/RETURN + structured flow. All
  GOSUB targets emit cleanly as named Rust functions.
- **Collision via POINT()** — banana flight loop checks
  `IF POINT(BX, BY) <> BACKCOLOR THEN` every step.
- **PLAY for explosions** — short MML string; wired to rodio (M5 ✅).
- **RANDOMIZE TIMER** — `qb_timer()` returns seconds-since-midnight.
- **SELECT CASE** — wind direction text display.
- **CIRCLE + PAINT** — gorilla sprites are overlapping filled circles;
  flood fill boundary color must be exact or sprites bleed.
- **LINE with BF** — all buildings drawn as `LINE ...,BF` filled rectangles.
- **Shared game state** — all global vars (positions, scores, colors) in
  `GameState` struct passed as `&mut __gs` through every SUB.

---

## Feature Support Notes

Per-feature detail and caveats. For the prioritized roadmap of what remains,
see **What's Left** above.

- **Array lower bounds** — `DIM arr(lo TO hi)` is fully supported via the
  "wasted-slots" strategy: allocation uses `(hi + 1)` slots and indices are
  used raw (no offset subtraction), so an array can be passed to a SUB and
  indexed identically by caller and callee. `LBOUND` reads the declared lower
  bound from the `array_lower` map; `UBOUND` is `arr.len() - 1`. Covered by the
  `array_bounds` regression test.
- **PRINT USING** — numeric (`#`, `.`, `,` grouping, leading/trailing sign),
  string (`!`, `\ \`, `&`), and literal-escape (`_X`) fields all work.
  Exponential `^^^^` scientific notation is supported: the mantissa is
  normalized to one significant integer digit (e.g. `##.##^^^^; 234.56` →
  ` 2.35E+02`, `.####^^^^; 888888` → `.8889E+06`), extra carets widen the
  exponent. Wide-field overflow follows QB: a value too large for the field
  is printed in full behind a leading `%` (e.g. `##; 123` → `%123`). Field
  width now equals the literal width of the format spec (a prior off-by-one
  that over-padded every numeric field by one space is fixed). 20 unit tests
  in `runtime/src/lib.rs::print_using_tests`. Not yet special-cased: `$$`
  floating dollar and `**` asterisk fill (they print literally).
- **File I/O** — `OPEN/CLOSE/READ#/WRITE#/INPUT#` not implemented.
- **Error handling** — `ON ERROR GOTO`, `RESUME` parsed but not emitted.
- **User-defined TYPEs** — fully supported including: arbitrarily deep nested
  TYPEs (e.g. `Outer.Middle.Inner.Val`), 1-D and 2-D arrays of TYPEs (including
  nested), string fields, scalar TYPE variables, scalar TYPE params to SUBs
  (byref, expanded to per-field `&mut f64` args), typed array params to SUBs,
  whole-record copy via field-level assignment, field-level swap using a temp
  TYPE var. Keyword-named types (e.g. `TYPE Color`) parse correctly via
  `advance_as_type_ident`. Regression tests: `type_nested`, `type_complex`.
  Remaining gap: array fields within TYPEs (`Bar(10) AS SINGLE` inside a TYPE
  body) — parser silently discards the dimension; rare in typical QB programs.
