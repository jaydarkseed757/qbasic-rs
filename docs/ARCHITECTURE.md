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
├── docs/ARCHITECTURE.md        # This file
│
├── src/                        # Transpiler (qbc binary)
│   ├── main.rs                 # CLI, pipeline orchestration, --verbose stats
│   ├── lexer.rs                # Source text → Vec<Spanned<Token>>
│   ├── parser.rs               # Tokens → AST (Program, Stmt, Expr, LValue)
│   ├── analyzer.rs             # AST → AnalyzedProgram (symbol table, DATA)
│   ├── emitter.rs              # AnalyzedProgram → Rust source string  (~5370 lines)
│   └── error.rs                # QbError enum (Lex / Parse / Analyze / Emit)
│
├── runtime/                    # Runtime library linked by every transpiled program
│   ├── Cargo.toml              # depends on: minifb, crossterm, rodio
│   └── src/
│       ├── lib.rs              # Runtime struct, graphics, I/O, math/string fns  (~3875 lines)
│       └── sound.rs            # PLAY MML parser + SOUND/BEEP via rodio  (~300 lines)
│
└── basic-src/                  # Test .bas programs (44 total)
    ├── gorilla.bas             # Primary target: gorilla-throwing game
    ├── torus.bas               # 3-D torus (arrays of TYPE, WINDOW/PMAP, VGA palette)
    ├── reversi.bas             # Reversi/Othello AI (3-D arrays, WINDOW SCREEN)
    ├── donkey.bas              # GOTO state machine: Q-BASIC Donkey (CGA SCREEN 1)
    ├── mandel.bas              # Mandelbrot renderer (VIEW/WINDOW/PALETTE USING)
    ├── money.bas               # Money manager (binary I/O, CP437 box-drawing)
    ├── sortdemo.bas            # Sorting visualizer (SHARED, animation)
    ├── invaders.bas            # Space Invaders (SCREEN 13 VGA, TYPE records, GOTO-in-SUBs)
    ├── duck.bas                # Cartoon duck — DRAW + PAINT (SCREEN 9 EGA)
    ├── etto.bas                # VGA photo display — 256-color DATA pixels (SCREEN 13)
    ├── kitchen_sink-gw.bas     # GW-BASIC mega test — ON GOTO/GOSUB, DEF FN, RESTORE
    ├── kitchen_sink-qbasic.bas # QBasic 4.5 mega test — 9 menu items, ON GOTO named labels
    ├── screen13.bas            # SCREEN 13 MCGA 256-color demo
    ├── screen13-sprite.bas     # SCREEN 13 GET/PUT 8-bpp sprites
    ├── pi.bas                  # Arbitrary-precision pi via Machin's formula
    ├── evil.bas                # GW-BASIC POKE/PEEK with physical line continuations
    ├── kingdom.bas             # GW-BASIC kingdom resource-management game
    ├── vgadac.bas              # VGA DAC port I/O test (OUT/INP vs PALETTE vs readback)
    └── …                       # nibbles, hangman*, pi-gw, q_sort, fuzzbuzz, primes,
                                #   step, 256c, palette256_expanded, random-pixel, qblocks,
                                #   loopyloop, pixel-gw, pokeit, pokemix, qmaze, demo1,
                                #   hello-world, sound, toccata, gotorama, sortdemo,
                                #   blackjack, textpaint, qbricks
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

## Supported QBasic Keywords & Functions

The authoritative lists live in the lexer keyword table (`src/lexer.rs`), the
statement parser (`src/parser.rs`), and the emitter's built-in dispatch
(`src/emitter.rs`). This set covers all 24 bundled DOS programs.

### Statements & keywords

| Group | Keywords |
|-------|----------|
| **Control flow** | `IF` / `THEN` / `ELSE` / `ELSEIF`, `SELECT` / `CASE` / `IS` (ranges via `TO`), `FOR` / `TO` / `STEP` / `NEXT`, `WHILE` / `WEND`, `DO` / `LOOP` / `UNTIL`, `GOTO`, `GOSUB` / `RETURN`, `ON` _expr_ `GOTO` / `GOSUB` (computed branch), `EXIT`, `END`, `STOP` |
| **Procedures & decls** | `SUB`, `FUNCTION`, `DECLARE`, `CALL`, `DEF` (DEF FN), `DIM`, `REDIM` (+ `PRESERVE`), `ERASE`, `SHARED`, `STATIC`, `COMMON`, `CONST`, `TYPE` / `AS`, `OPTION` (BASE), `LET`, `DEFINT`/`DEFSNG`/`DEFDBL`/`DEFSTR`, `SWAP` |
| **Data** | `DATA` / `READ` / `RESTORE` |
| **I/O & text** | `PRINT` / `LPRINT` (+ `PRINT USING`), `INPUT` (+ `LINE INPUT`), `LOCATE`, `COLOR`, `CLS`, `WIDTH`, `VIEW PRINT`, `REM` / `'` |
| **Files** | `OPEN`, `CLOSE`, `FIELD`, `GET`/`PUT` (`#n`), `LSET`/`RSET`, `WRITE`, `PRINT #`, `INPUT #`, `LINE INPUT #` |
| **Graphics** | `SCREEN` (0,1,2,7,8,9,10,12,13), `LINE` (+ `B`/`BF`), `CIRCLE`, `PSET`, `PRESET`, `PAINT`, `DRAW`, `GET`/`PUT` (sprites, all verbs), `VIEW`, `WINDOW` (+ `WINDOW SCREEN`), `PALETTE` / `PALETTE USING`, `STEP` coords |
| **Sound** | `PLAY`, `SOUND`, `BEEP` |
| **Errors** | `ON ERROR` / `GOTO`, `RESUME` (+ `RESUME NEXT`), `ERR` |
| **Misc** | `RANDOMIZE` (TIMER), `SLEEP`, `POKE` (simulated byte store), `OUT` (VGA DAC port write), `INP` (VGA DAC port read), `DEF SEG` (parsed/ignored) |
| **Operators** | `AND`, `OR`, `XOR`, `NOT`, `EQV`, `IMP`, `MOD`, `\` (integer divide), `^`, `+ - * /`, comparisons |

### Built-in functions

| Group | Functions |
|-------|-----------|
| **Math** | `INT FIX ABS SQR SGN SIN COS TAN ATN EXP LOG` |
| **Type / convert** | `CINT CLNG CSNG CDBL`; binary `MKD$ MKI$ MKS$ MKL$` / `CVD CVI CVS CVL` |
| **String** | `LEN LEFT$ RIGHT$ MID$ UCASE$ LCASE$ LTRIM$ RTRIM$ STR$ VAL CHR$ ASC INSTR SPACE$ STRING$ HEX$ OCT$` |
| **Runtime / system** | `RND TIMER INKEY$ INPUT$ POINT PMAP PEEK ERR EOF LOF ENVIRON$ DIR$ UBOUND LBOUND` |
| **Image load** | `BLOAD file$[,offset]` → blits a raw/BSAVE screen image into the framebuffer (the `DEF SEG = &HA000` video-memory case; `DEF SEG` itself is a no-op). `BSAVE` and non-video targets are unmodeled. |

### `DEF FN` — both single-line and multi-line supported
- Single-line: `DEF FnName(x) = expr` (emitted as an inline-expression fn).
- Multi-line: `DEF FNName(args)` … statements … `FNName = result` … `END DEF`
  (`EXIT DEF` for early return). Converted to a `FuncDef` at parse time and emitted via
  the standard `FUNCTION` path. Limitation: the body is an isolated fn, so it only sees
  parameters + shared/global (`GameState`) state, not arbitrary main-module locals
  (QB's true `DEF FN` shares module scope).

### `ON ERROR GOTO` / `RESUME` — named-label happy path only
- `ON ERROR GOTO <named label>` + `RESUME NEXT` work: the handler is extracted as a fn
  and invoked when `__rt.error_pending` is set after a fallible statement; `ERR` returns
  the error code. The only error the runtime raises is **file-not-found (err 53) on
  `OPEN`**. Not implemented: bare `RESUME` retry (treated as `RESUME NEXT`), numeric-
  label handlers in `__pc` state-machine programs (error cleared but no jump), `ERL`,
  and error triggers beyond file-`OPEN`.

### Not supported / stubbed
- `PAINT` with a `CHR$()` tiling pattern → solid-foreground stub (dead on color paths)
- `PRINT USING` `$$` / `**` floating tokens → printed literally
- `CHAIN` / `SHELL` → not modeled (stubbed to program end)
- `OUT` / `INP` — **now supported** for VGA DAC ports (0x3C7/0x3C8/0x3C9); other port
  addresses are silently ignored (not modeled at the hardware level)

`GET`/`PUT` sprites are fully supported across pixel depths: EGA 4-plane planar
(SCREEN 9/12), CGA 2bpp packed (SCREEN 1), and MCGA 8bpp chunky (SCREEN 13).

---

## Lexer (`src/lexer.rs`)

Converts raw source bytes to `Vec<Spanned<Token>>` where each token carries
its source line number for error reporting.

**Key behaviors:**

- **CP437 / Latin-1 safe** — `main.rs` tries `std::str::from_utf8()` first
  (for modern UTF-8–saved files like `money.bas`); falls back to `byte as char`
  for genuine DOS CP437 files, so extended characters (box-drawing, block
  elements) don't cause UTF-8 errors.
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

The largest file (~5500 lines). Walks `AnalyzedProgram` and writes Rust source.

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

### Post-processing passes

Three text passes are chained at `emit()` return (in order):

```rust
Ok(strip_deref_parens(&remove_unnecessary_mut(&inline_single_use_tmps(&self.out))))
```

1. **`inline_single_use_tmps(out)`** — inlines `let __tmpN = expr;` when `__tmpN`
   is referenced exactly once. Folds away hoisting clutter when `lift_expr` was
   overly conservative.

2. **`remove_unnecessary_mut(out)`** — removes `mut` from `let mut varname:` when
   `varname` is never mutated within its enclosing function scope (no `varname =`,
   `varname +=`, `&mut varname`, or `for varname in`). Infrastructure names
   (`__gs`, `__rt`, `__fn_ret`, `__pc`, `__for_*`, `__tmp_*`) are never touched.
   If `mut` is incorrectly removed, rustc catches it at compile time of the emitted
   file — fail-safe. Non-idempotent: a second pass would corrupt `qb_bool(*x)`.

3. **`strip_deref_parens(out)`** — rewrites `(*ident)` → `*ident` unless followed
   by `.` or `[`. Byte scanner; skips string literals. `idx_sub()` helper at emit
   sites avoids generating `[((*x)) as usize]` double-wraps before the pass runs.

---

## Runtime (`runtime/src/`)

Two files totaling ~4175 lines, linked by every transpiled program.

### `lib.rs` (~3875 lines)

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
    rng:             u32,       // QB 24-bit LCG state; power-on seed = 0x50000
    last_rnd:        f64,       // most recent rnd() result, returned by RND(0)

    // VIEW / WINDOW logical coordinate system
    view_x1..view_active, win_x1..win_active,
    gfx_x, gfx_y,          // graphics cursor in logical coords

    // DRAW state
    draw_scale:      f64,       // S value; pixels_per_unit = draw_scale / 4
    draw_color:      u8,        // C value (current DRAW color)

    // VGA DAC port state (OUT 0x3C8/0x3C9/0x3C7, INP 0x3C9)
    dac_write_idx:   usize,     // current write index (set by OUT 0x3C8)
    dac_channel:     u8,        // 0=R 1=G 2=B; auto-advances on OUT 0x3C9
    dac_pending_r/g/b: u8,      // accumulates R then G then B (6-bit each)
    dac_read_idx:    usize,     // current read index (set by OUT 0x3C7)
    dac_read_ch:     u8,        // 0=R 1=G 2=B; auto-advances on INP 0x3C9

    // POKE/PEEK simulated memory
    poke_mem:        HashMap<u32, u8>,  // byte store for POKE/PEEK

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

`present()` rounds logical→framebuffer coordinates to the nearest pixel (not
truncation) so non-integer `WINDOW`/`PMAP` mappings don't drop scanlines — see
Graphics primitives.

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

`OUT &H3C8, idx` / `OUT &H3C9, val` (VGA DAC protocol) also remaps `palette_rgb`
entries. Three sequential `OUT &H3C9` writes supply R, G, B (each 6-bit, 0–63);
the runtime accumulates them and converts via `dac6_to_8(c) = (c << 2) | (c >> 4)`
on the third write. `INP(&H3C9)` reads back the 6-bit value of R, G, or B from
`palette_rgb[dac_read_idx]` in the same round-robin order.

### Arithmetic operators

Most binary operators map directly to Rust on `f64` (`+ - * /`). Three need
QB-specific handling and are emitted as runtime helper calls rather than inline
Rust:

| QB | Emitted as | Notes |
|----|------------|-------|
| `a ^ b` | `a.powf(b)` | Float power. **Left-associative**: `2^3^2 = (2^3)^2 = 64`. Unary minus binds *looser* than `^` (`-2^2 = -4`), handled in the parser. |
| `a \ b` | `qb_idiv(a, b)` | Integer division: both operands are **CINT-rounded to integers first** (banker's), then divided with truncation toward zero. |
| `a MOD b` | `qb_mod(a, b)` | Both operands CINT-rounded first, then remainder. Sign follows the **dividend** (same as Rust `%` on integers). |

**Operator precedence** (tightest to loosest): `^` → unary `-` → `*`/`/` → `\` → `MOD` → `+`/`-` → relational (`<`,`>`,`=`,`<>`,`<=`,`>=`) → `NOT` → `AND` → `OR` → `XOR` → `EQV` → `IMP`. The middle tier (`*`/`/` tighter than `\` tighter than `MOD`) is the common source of precedence bugs — `2 * 3 MOD 4` = `(2*3) MOD 4` = 2, not `2 * (3 MOD 4)` = 6.

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
| `x EQV y` | `qb_eqv(x,y)` | Bitwise XNOR: `!(a as i64 ^ b as i64)` |
| `x IMP y` | `qb_imp(x,y)` | Bitwise implication: `!(a as i64) \| (b as i64)` |
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

Authentic QB 24-bit LCG:
```rust
self.rng = self.rng.wrapping_mul(16598013).wrapping_add(12820163) & 0xFF_FFFF;
self.last_rnd = self.rng as f64 / 16777216.0;
```
Power-on seed is `0x50000` → first `RND` is the canonical QB value `0.7055475`.
`RANDOMIZE seed` mixes the f32 bit pattern of the seed value into `rng`.

`rnd_arg(v)` implements QB's argument semantics: `v < 0` reseeds from the f32
bit pattern of `v` then advances; `v == 0` returns `last_rnd` (repeats);
`v > 0` (or bare `RND`) advances normally.

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

### M18 — Emitter quality passes, VGA DAC, behavioral env overrides, kingdom.bas (build-all 44/44) ✅

Three chained post-processing passes now run at `emit()` return:

- **`inline_single_use_tmps`** — inlines `__tmpN` temporaries used exactly once.
  Reduces clutter from `lift_expr`'s defensive hoisting.
- **`remove_unnecessary_mut`** — removes `mut` from `let mut varname:` declarations
  where no mutation (`varname =`, `&mut varname`, compound `+=`, or `for varname in`)
  appears before the end of the enclosing function scope. Infrastructure names
  (`__gs`, `__rt`, `__fn_ret`, `__pc`, `__for_*`, `__tmp_*`) are never de-mutted.
  Non-idempotent (second pass would corrupt `qb_bool(*mouth)` patterns).
- **`strip_deref_parens`** — rewrites `(*ident)` → `*ident` except when followed
  by `.` or `[` (field/index access). `idx_sub()` at emit sites prevents generating
  `[((*x)) as usize]` double-wraps in the first place.

Other emitter quality improvements:
- **Concrete defaults** — `0.0_f64` / `String::new()` instead of `Default::default()` 
  everywhere: `__fn_ret`, array element inits, `emit_dim` scalars, `emit_game_state`.
- **Duplicate DIM suppression** — `locals_declared: HashSet<String>` tracks what
  `emit_locals` already declared; `emit_dim` skips re-declaring the same numeric
  scalar. String DIMs always emitted (support sigil-less DIM/shadow patterns).

**VGA DAC hardware port I/O** (`OUT`/`INP`) — `Token::Out`/`Token::Inp` added;
`Stmt::Out { port, val }` AST node; `qb_out`/`qb_in` on `Runtime`. The DAC state
machine handles ports 0x3C7 (read index), 0x3C8 (write index), 0x3C9 (R/G/B sequential).
`OUT` is context-sensitive: not a statement when followed by `(` or `=` (avoids
breaking array param names like `out()` in pi.bas). `vgadac.bas` tests the full path.

**Behavioral env-var overrides** — `apply_behavioral_env()` called after pragma
setter calls in `main()` so env vars always override compile-time pragma values.
`QBC_TITLE`/`QBC_SCALE` resolved inside `new_configured()` before window opens.

**kingdom.bas (GW-BASIC)** — four transpiler bugs fixed and one `.bas` source fix:
1. DATA raw-capture in lexer (prevents `1ST` from tokenizing as `IntLit + Ident`)
2. Colon-before-ELSE consumed in single-line IF parser
3. POKE/OUT args use `lift_expr` to prevent double `&mut __rt` borrow
4. `collect_scalar_names_stmt` `PrintUsing` arm added (cross-GOSUB promotion)
5. Source: `KORS` → `K OR S` on line 200 (DOS editor had collapsed the expression)

### M17 — blackjack.bas + QB language fixes (build-all 42/42, 30/30 tests) ✅

`blackjack.bas` (SCREEN 12 VGA casino blackjack) uncovered five transpiler/runtime
gaps. All fixed; build-all now 42/42, integration suite 30/30, 96 unit tests.

- **Single-line IF stealing outer block-IF's ELSE** (`src/parser.rs`) — when a
  block-IF's THEN body ends with a single-line IF, `parse_stmt` consumes the
  trailing newline, making the outer ELSE "adjacent" and wrongly stolen by the
  inner IF. Caused blackjack's `AnimateDeal` to loop forever (card never slid).
  Fix: capture `if_line` at `parse_if` entry; only attach single-line ELSE when
  it is on the same source line. Regression test: `tests/programs/if_single.bas`.

- **minifb use-after-free segfault** (`runtime/src/lib.rs`) — minifb's macOS
  backend stores the raw pointer from `update_with_buffer` without copying; a
  per-call local `Vec<u32>` was freed on `present()` return, leaving a dangling
  pointer that triggered `EXC_BAD_ACCESS` in `drawInMTKView` during the next idle
  `INKEY$` poll. Fix: `present_buf: Vec<u32>` persistent field on `Runtime`.

- **`PLAY(n)` function form** (`src/parser.rs`, `src/emitter.rs`,
  `runtime/src/lib.rs`) — QB's `PLAY(n)` is both a statement and a function
  returning notes remaining in the background queue. The function form was hitting
  `parse_primary`'s error arm. Fix: `Token::Play` in expression context →
  `Expr::Call { "PLAY" }`; both `emit_expr_inner` and `lift_expr` map it to
  `__rt.play_count()`. The runtime adds `bg_playing: Arc<AtomicBool>` set on
  background play start and cleared when the thread finishes; `play_count()`
  returns 10 while playing (≥5 = "don't refill"), 0 when done. Without this the
  title-screen music loop spawned a new background thread every ~0.9 s → doubled
  audio.

- **`MID$(var$, pos[, len]) = val`** statement form (`src/parser.rs`,
  `src/emitter.rs`, `runtime/src/lib.rs`) — in-place substring replacement with
  no string-length change. Was parsed as a 3-D array assignment. Fix: new
  `Stmt::MidAssign`; early detection in `parse_assign_or_call`; emitted as
  `qb_mid_assign(&mut var, pos, len_opt, &val)`.

- **TYPE body array fields** (`src/parser.rs`, `src/emitter.rs`) — `Bar(4) AS
  INTEGER` within a `TYPE` block was silently dropping the dimension. Four
  emission sites fixed: `emit_game_state`, `emit_dim` (shared and local typed
  array paths), `FieldIndex` lvalue emitter, and both `parse_assign_or_call` and
  `parse_primary` dot-chain subscript paths. `DIM boards(2) AS Grid` where `Grid`
  has `Cell(4)` now correctly emits `boards__cell: Vec<Vec<f64>>` and
  `boards(i).Cell(j)` round-trips. Integration test: `tests/programs/type_array_field.bas`.

### M16 — INVADERS.BAS support: QB4.5 line continuation, AS STRING params, local string arrays ✅
Full support for `INVADERS.BAS`, a 1730-line QB4.5 Space Invaders port (build-all 38/38).

- **QB4.5 `_` line continuation** (`src/lexer.rs`) — bare `_` at end of a
  non-line-numbered source line continues onto the next physical line. 18
  continuations in INVADERS; zero effect on non-QB4.5 programs.
- **Double-comma `GET/PUT #n, , var`** (`src/parser.rs`) — empty record-position
  `,,` for sequential binary access; parser returns `None` for the record-number slot.
- **`AS STRING` typed parameters** (`src/emitter.rs`) — `nm AS STRING` (no sigil)
  in SUB/FUNCTION params emits `nm_s: &mut String`; `emit_lvalue`, `Stmt::Let`,
  `is_str_expr_ctx`, `emit_call_args`, and `lift_expr` all extended.
- **Local string arrays** (`src/emitter.rs`) — `DIM rankStr(1 TO 10) AS STRING`
  without `$` sigil tracked in `local_string_arrays: HashSet<String>`; used in
  `emit_lvalue`, `emit_expr_inner`, `lift_expr`, `is_str_expr_ctx`, `Stmt::Let`.
- **Array param dimensionality** (`src/emitter.rs`) — `array_param_used_dims()`
  scans the sub body to determine actual index depth used, so a 2D-accessed
  array declared as 1D in the param list gets `Vec<Vec<f64>>` correctly.

### M15 — QB-fidelity transpiler fixes (code review June 2026) ✅
Ten correctness fixes from a full src/ + runtime/ review (build-all 39/39 at
the time; now 42/42 after M16–M17).

- **`*`/`\`/`MOD` precedence corrected** (`src/parser.rs`) — chain was inverted;
  `2 * 3 MOD 4` now correctly yields 2 (was 6).
- **`^` left-associative** — `2^3^2 = (2^3)^2 = 64` (was right-assoc → 512).
- **Array elements pass BYREF to SUBs** — `CALL Swap(a(i), a(j))` now hoists
  each element to a temp and writes back after the call; mutations were silently
  lost before.
- **`NEXT i, j` multi-counter** — each extra comma-separated name closes one
  enclosing FOR via `pending_nexts: u32` on the parser.
- **DATA backslash escaping** — `DATA "C:\temp"` no longer corrupts (backslash
  was not escaped before `"` in the Rust static string literal).
- **`EQV`/`IMP` operators** end-to-end — lexer tokens, parser precedence levels
  looser than XOR, `qb_eqv`/`qb_imp` runtime fns (bitwise on i64).
- **`UBOUND(a$())` paren-form** — now resolves the `_s` suffix for string arrays
  (emitted `a.len()` instead of `a_s.len()` → rustc error).
- **Authentic QB 24-bit LCG** — replaced MSVC `rand()` formula; first RND from
  power-on seed is now the canonical QB value 0.7055475.
- **`RND(n)` argument semantics** — parser was discarding the argument; now
  captured and routed to `rnd_arg()` (0 = repeat, negative = reseed).
- **`skip_warn` helper** — skipped statements (ON KEY, TIMER ON/OFF, CLEAR,
  WIDTH, bare LOOP) now emit a stderr warning per the project rule.
- **New test**: `tests/programs/qb_semantics.bas` covers all 10 fixes;
  goldens regenerated for gorilla and donkey (RNG-dependent frames).

### M14 — money.bas full support: binary I/O, CP437 font, INPUT# trim ✅
Full end-to-end support for `money.bas` (Microsoft 1990 money manager) including
binary random-access I/O, extended ASCII rendering, and numeric save/load
round-trip (build-all 33/34).

- **UTF-8 source decoding** (`src/main.rs`) — source reader tries `from_utf8()`
  first, falls back to `byte as char` for CP437/Latin-1 DOS files.
- **CP437 / extended ASCII font** — `FONT_8X8` extended from 128 → 256 entries
  (box-drawing 0x80–0xFF: `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼ █ ▓ ▒ ░` etc.).
  `draw_char_fb` uses the full code-point index instead of `& 0x7F`.
- **Latin-1 binary encoding** — `MKD$`/`CVD`, `MKI$`/`CVI`, `MKS$`/`CVS`,
  `MKL$`/`CVL` encode IEEE 754 LE bytes as Latin-1 chars; `qb_lset`/`qb_rset`
  use `.chars().count()` for correct binary-string padding; `qb_field_get`/
  `qb_field_put` share the same encoding. INTEGER/LONG/fixed-STRING fields are
  byte-exact with DOS QBasic 1.1.
- **`INPUT #n` numeric parse trim** — `qb_print_num` emits `" N "` (QB leading-
  space convention); added `.trim()` before `.parse()` in both `INPUT #n, numVar`
  (file) and interactive `INPUT numVar`. Root cause of the "black menu" symptom:
  `colorpref` parsed as 0.0 → `colors[x][0]` (unpopulated) → black-on-black.
- **`local_dim_names`** — `HashSet<String>` tracks explicit `DIM` declarations
  per scope to prevent a local numeric `B` from being shadowed by a promoted
  shared string `B$`. Type-aware exclusion: strings by typed name (`b_s`),
  numerics by bare name (`b`).
- **Parser additions** — `ON KEY(n) GOSUB/GOTO`, `KEY(n) ON/OFF/STOP`,
  `TIMER ON/OFF/STOP` → no-ops; `CLEAR` → no-op; `REDIM SHARED` propagates
  `shared=true`; removed dead duplicate FIELD handler (41 lines) that shadowed
  `parse_field()` and discarded field-length info.
- **`pokemix.bas`** and **`qmaze.bas`** — two new bundled programs; both pass
  build-all with no additional transpiler changes.

### M13 — GW-BASIC line continuation, POKE/PEEK memory ✅
Physical-line continuation support for GW-BASIC programs plus real POKE/PEEK
byte-accurate memory (build-all 31/32).

- **Physical line continuation** — GW-BASIC programs where a logical line wraps
  across multiple physical file lines without repeating the line number. Lexer
  detects `in_line_numbered_mode` on the first `IntLit` at statement position;
  `\n` handling suppresses `Newline` tokens when the next physical line is a
  continuation (no leading digit). Non-line-numbered programs: byte-identical.
- **`POKE` / `PEEK`** — `POKE addr, val` now stores `val & 0xFF` in a
  `HashMap<u32, u8>` (`poke_mem`) on `Runtime`; `PEEK(addr)` returns the stored
  byte or 0. Previously both were stubs. `PEEK` added to `lift_expr` hoist table
  to avoid double-borrow in `PRINT PEEK(...)`.
- **evil.bas** — GW-BASIC "self-modifying POKE matrix" demo; three physical line
  continuations, simulated POKE/PEEK memory, `DEFINT A-Z`, `DEF SEG`.
- **pokeit.bas** — minimal regression: `POKE 1040, D` → `PRINT PEEK(1040)` → ` 25`.
- **demo1.bas** — SCREEN 13 demoscene-style intro (star field + scrolling text).

### M12 — Tooling, MCGA sprites, GW-BASIC mega test ✅
Debugging infrastructure plus the last language gaps for the bundled set
(build-all 28/28).

- **Headless driver** — any transpiled binary honors `QBC_*` env vars (read once
  in `new_configured`, byte-identical when unset): `QBC_HEADLESS`, `QBC_KEYS`,
  `QBC_SEED`, `QBC_DUMP`/`QBC_DUMP_AT`, `QBC_CHECKSUM`, `QBC_FBSTATS`,
  `QBC_TEXT_FB`, `QBC_EXIT_AFTER`. Enables deterministic, windowless renders for
  CI/SSH and turned multi-round "black screen" debugging into framebuffer diffs.
- **Graphics golden tests** — `tests/run-graphics-tests.sh` diffs `fb_checksum`
  against committed goldens (256c, screen13, palette256_expanded, reversi, torus);
  `tools/ppm2png.py` exports PPM dumps to PNG for the README gallery. mandel is
  excluded (timing-dependent + infinite palette cycle).
- **SCREEN 13 (MCGA) GET/PUT sprites** — 8-bpp chunky layout
  (`get_sprite_mode13`/`put_sprite_mode13`: `data[0]=width*8`, `data[1]=height`,
  one color byte/pixel). All three sprite depths now covered (EGA planar, CGA
  2-bpp, MCGA 8-bpp). See `screen13-sprite.bas`.
- **`ON expr GOTO/GOSUB`** — computed multi-way branch lowered to
  `match qb_cint(expr) as i64`; targets join the `__pc` state machine / GOSUB-fn
  set. Plus shared/promoted-scalar naming fixes in the FOR counter and DEF-FN /
  PRINT argument paths (kitchen_sink-gw.bas).

### M11 — More DOS programs: torus, donkey, reversi ✅
Expanded coverage to the remaining graphics-heavy DOS QBasic programs.

- **donkey.bas** (CGA SCREEN 1): authentic 2-bpp packed-INTEGER sprite GET/PUT
  layout; PUT action verbs (PSET/PRESET/AND/OR/XOR, default XOR); DRAW fixes
  (M-command relativity, N no-advance, color-follows-COLOR). See `docs/donkey.md`.
- **torus.bas** (SCREEN 12): arrays of a user TYPE flattened to per-field Vecs;
  `SHARED … AS type` inside SUBs; `PAINT STEP`; typed-array element passed to a
  SUB; **FUNCTION parameters pass by reference** (QB semantics — `Inside()`
  mutates a TYPE arg the caller reads back); `WINDOW`-without-`VIEW` maps to the
  full screen; **Y-axis inversion** for Cartesian `WINDOW`; SCREEN 11/12 PALETTE
  decodes the 18-bit VGA DAC value. See `docs/torus.md`.
- **reversi.bas** (SCREEN 9): `WINDOW SCREEN` (screen-orientation, magnitude-
  mapped so reversed corners don't flip); `ERASE`; **3-D arrays**
  (`nested_vec_type`/`nested_vec_init`); 2-D arrays of a TYPE; shared-field args
  to user FUNCTIONs hoisted to avoid borrow conflicts; scalar/array same-name
  coexistence (`A$` vs `A$()`). See `docs/reversi.md`.
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
`qb_lset`/`qb_rset` free functions pad/truncate to field length using
`.chars().count()` for correct binary-string handling. `qb_field_get`/
`qb_field_put` encode/decode field bytes as Latin-1 chars (byte `b` ↔
`char::from_u32(b as u32)`) so the 8-bit byte values of `MKD$` binary strings
round-trip through QB string variables correctly. FIELD variable declarations
collected into `emit_locals` so field vars are pre-declared as `String` locals.
`INPUT #n, numVar` emits `.trim().parse()` so QB's leading-space numeric format
(`qb_print_num` → `" N "`) round-trips through save/load correctly.
Test program: money.bas (Microsoft 1990).

### M7 — User-defined TYPE completeness ✅
Recursive TYPE flattening (`flatten_type_fields`): nested TYPEs to arbitrary
depth (`Outer.Middle.Inner.Val`), 1-D / 2-D arrays of TYPEs (incl. nested),
string fields, scalar TYPE vars, scalar TYPE params to SUBs (byref, expanded
to per-field `&mut` args), typed-array params, whole-record copy, field-level
swap. Keyword-named types (`TYPE Color`) parse via `advance_as_type_ident`.
Tests: `type_nested`, `type_complex`.

---

## What's Left

**Every bundled DOS QBasic program in `basic-src/` now transpiles, compiles, and
renders** — `build-all.sh` is **44/44** (gorilla, torus, reversi, mandel, donkey,
nibbles, sortdemo, money, pi, pi-gw, primes, hangman, hangman-gfx, hangman-gw,
q_sort, fuzzbuzz, hello-world, sound, step, screen13, screen13-sprite, 256c,
palette256_expanded, random-pixel, qblocks, qbricks, kitchen_sink-gw,
kitchen_sink-qbasic, loopyloop, pixel-gw, evil, pokeit, demo1, pokemix, qmaze,
duck, etto, invaders, toccata, gotorama, blackjack, textpaint, kingdom, vgadac).
The integration suite is **30/30**, with 119 runtime unit tests and 9 graphics golden tests.

Remaining work is one rarely-used feature:

1. **`PRINT USING` floating tokens** — `$$` (floating dollar) and `**`
   (asterisk fill) print literally. `^^^^` scientific notation and wide-field
   `%` overflow are implemented (see Feature Support Notes).

Previously listed items that are now complete:
- **`OUT &H3C8/&H3C9` VGA DAC port writes** — ✅ Implemented. `qb_out`/`qb_in`
  on `Runtime` model the 3-channel VGA DAC state machine. `vgadac.bas` tests
  PALETTE vs OUT writes vs INP readback.
- **Behavioral pragma env-var overrides** — ✅ Implemented. `apply_behavioral_env()`
  (called after pragma setter calls in `main()`) reads `QBC_PACE/FPS/FULLSPEED/
  SLOWMO` and overwrites the compiled-in pragma values. Env always wins.

---

## GORILLAS.BAS Specifics

- **SCREEN 9 first, fallback to SCREEN 1** — gorilla.bas negotiates EGA
  (640×350) via `ON ERROR GOTO`, falls back to CGA (320×200). The
  `ON ERROR`/`RESUME` logic is safely stubbed — just hard-selects SCREEN 9.
  `aspect=0.8333` for CIRCLE corrects non-square EGA pixels.
- **No GOTO** — entire program is GOSUB/RETURN + structured flow. All
  GOSUB targets emit cleanly as named Rust functions.
- **Collision via POINT()** — banana flight loop checks
  `IF POINT(BX, BY) <> BACKCOLOR THEN` every step.
- **PLAY for explosions and victory** — short MML strings (foreground `MF` and
  background `MB` modes); wired to rodio (M5 ✅).
- **RANDOMIZE TIMER** — `qb_timer()` returns seconds-since-midnight.
- **SELECT CASE** — wind direction text display.
- **CIRCLE + PAINT** — gorilla sprites are overlapping filled circles;
  flood fill boundary color must be exact or sprites bleed.
- **LINE with BF** — all buildings drawn as `LINE ...,BF` filled rectangles.
- **GET/PUT sprite system** — gorilla sprites drawn once with vector graphics,
  captured with `GET`, then blitted with `PUT`. Banana sprites from inline DATA.
- **Shared game state** — all global vars (positions, scores, colors) in
  `GameState` struct passed as `&mut __gs` through every SUB.
- **Golden-tested** — headless seed 42, scripted intro + one banana throw
  (angle 45°, velocity 50), captures mid-flight frame at `presents:80`.

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
- **File I/O** — fully supported: sequential (`FOR INPUT/OUTPUT/APPEND`:
  `OPEN/CLOSE/INPUT#/LINE INPUT#/PRINT#/WRITE#`), random-access (`FOR RANDOM`:
  `FIELD/GET#/PUT#/LSET/RSET`), and binary TYPE-record serialization. `MKD$`/
  `CVD` etc. use IEEE 754 LE; INTEGER/LONG/fixed-STRING fields are byte-exact
  with DOS QBasic 1.1.
- **Error handling** — `ON ERROR GOTO` + `RESUME`/`RESUME NEXT` fully emitted.
  Named (non-numeric) handlers are extracted as GOSUB-style Rust functions and
  dispatched after fallible statements (`OPEN`, `SCREEN`). `ERR` variable maps
  to `__rt.err_code`; 53 = file not found.
- **User-defined TYPEs** — fully supported including: arbitrarily deep nested
  TYPEs (e.g. `Outer.Middle.Inner.Val`), 1-D and 2-D arrays of TYPEs (including
  nested), string fields, scalar TYPE variables, scalar TYPE params to SUBs
  (byref, expanded to per-field `&mut f64` args), typed array params to SUBs,
  whole-record copy via field-level assignment, field-level swap using a temp
  TYPE var. Keyword-named types (e.g. `TYPE Color`) parse correctly via
  `advance_as_type_ident`. Regression tests: `type_nested`, `type_complex`.
  Remaining gap: array fields within TYPEs (`Bar(10) AS SINGLE` inside a TYPE
  body) — parser silently discards the dimension; rare in typical QB programs.
