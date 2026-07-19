# QBasic → Rust Transpiler (qbasic_rs)

You are an expert Rust systems programmer working on `qbasic_rs`, a transpiler
that converts QBasic `.bas` source files into native Rust binaries. The primary
correctness target is **GORILLA.BAS** — the classic gorilla-throwing game
shipped with MS-DOS QBasic — running at 100% fidelity.

---

## Repository Layout

```
qbasic-rust/
├── Cargo.toml                  # Workspace (members: transpiler crate + runtime crate)
├── CLAUDE.md                   # This file
├── docs/
│   ├── ARCHITECTURE.md         # Full architectural reference — read this first
│   ├── gorillas.md             # Line-by-line walkthrough of gorilla.bas
│   ├── money.md                # Line-by-line walkthrough of money.bas
│   ├── BLACKJACK.md            # Deep-dive walkthrough of blackjack.bas (card encoding, state machine, dealer art, sound, high scores)
│   ├── torus.md                # Walkthrough of torus.bas
│   ├── reversi.md              # Walkthrough of reversi.bas
│   ├── donkey.md               # Walkthrough of donkey.bas
│   ├── demo.md                 # Walkthrough of demo.bas (15-scene megademo)
│   ├── mario.md                # Walkthrough of mario.bas (MEGA WORLD platformer)
│   ├── qb11-rules.md           # Drop-in prompt: writing QBasic 1.1-compatible .bas
│   └── screenshots/            # PNG screenshots of rendered programs
│
├── src/                        # Transpiler (qbc binary) — all in one crate
│   ├── main.rs                 # CLI: qbc <file.bas> [-o out.rs] [--emit-only] [--dump-ast]
│   ├── lexer.rs                # Source text → Vec<Spanned<Token>>
│   ├── parser.rs               # Tokens → AST (Program, Stmt, Expr, LValue)
│   ├── analyzer.rs             # AST → AnalyzedProgram (symbol table, labels, DATA)
│   ├── emitter/                # AnalyzedProgram → Rust source string (split module)
│   │   ├── mod.rs              # Emitter struct + all impl methods (~4680 lines)
│   │   ├── helpers.rs          # name-mangling + codegen utilities (~250 lines)
│   │   ├── scan.rs             # AST collection / analysis passes (~1620 lines)
│   │   └── postprocess.rs      # final Rust-output cleanup passes (~270 lines)
│   └── error.rs                # QbError enum (Lex / Parse / Analyze / Emit)
│
├── runtime/                    # Runtime library linked by every transpiled program
│   └── src/
│       ├── lib.rs              # Runtime struct, graphics, I/O, math (~3875 lines)
│       └── sound.rs            # PLAY / SOUND / BEEP via rodio (~300 lines)
│
├── basic-src/                  # Real DOS QBasic programs used for manual testing
│   └── gorilla.bas, nibbles.bas, mandel.bas, donkey.bas, …  (52 programs total)
│
└── tests/
    ├── programs/               # .bas source files for the integration test suite
    ├── expected/               # Expected stdout output for each test program
    └── run-tests.sh            # Transpile → compile → run → diff; 44 tests, all must pass
```

---

## Pipeline

```
file.bas
  │
  ├─[lexer]─────► Vec<Spanned<Token>>
  │
  ├─[parser]────► Program { subs, functions, main_body: Vec<Stmt> }
  │
  ├─[analyzer]──► AnalyzedProgram { global_scope, labels, data_store, subs, functions, main_body }
  │
  ├─[emitter]───► gorilla.rs  (Rust source, uses qbasic_runtime::*)
  │
  └─[rustc]─────► gorilla  (native binary)
```

---

## Current Status

**Every bundled DOS program in `basic-src/` transpiles, compiles, AND renders**
— `bash basic-src/build-all.sh` is **54/54** (gorilla, torus, reversi, mandel,
donkey, nibbles, sortdemo, money, pi, pi-gw, primes, hangman, hangman-gfx,
hangman-gw, q_sort, fuzzbuzz, hello-world, sound, step, screen13, screen13-sprite,
256c, palette256_expanded, random-pixel, qblocks, qbricks, kitchen_sink-gw,
kitchen_sink-qbasic, loopyloop, pixel-gw, evil, pokeit, demo1, demo, bench, pokemix,
qmaze, duck, etto, invaders, toccata, gotorama, blackjak, textpaint, kingdom, vgadac,
deffn-multi, onerror, farkle, pin, towers, pride, pride256c, mario). Test suites:
- **44/44** integration (`tests/run-tests.sh`, stdout-based)
- **145** runtime+transpiler unit tests (`cargo test --workspace`)
- **10/10** graphics golden tests (`tests/run-graphics-tests.sh` — framebuffer
  checksums for 256c, screen13, screen13-sprite, palette256_expanded, reversi,
  torus, hangman-gfx, duck, gorilla, donkey)

gorilla.bas is **fully verified** — headless golden for the banana-throw frame,
and audio (PLAY explosion/victory fanfares), victory animations, and multi-round
scoring have all been confirmed working via human play-through.

See `docs/ARCHITECTURE.md §Milestone Status` (M1–M18) and `§What's Left`.

---

## Critical Design Decisions — Never Deviate From These

### 1. All numerics are f64
QBasic's SINGLE (f32), INTEGER (%), LONG (&), DOUBLE (#) are all stored as
`f64` in emitted Rust. Do not introduce f32. Sigil suffixes on identifiers
affect name mangling only (e.g. `x#` → `x_d`), not the storage type.

### 2. QBasic boolean semantics
- `0.0`  = false
- `-1.0` = true  (bitwise NOT of 0 in two's complement, QB convention)
- All comparisons emit `qb_from_bool(...)` → returns -1.0 or 0.0
- All conditionals wrap in `qb_bool(v)` → `v != 0.0`
- Never emit bare Rust `bool` for a QB numeric comparison result

### 3. Arrays — wasted-slots strategy
`DIM arr(lo TO hi)` allocates `(hi + 1)` slots, **not** `(hi - lo + 1)`.
Raw QB indices are used directly as Vec indices everywhere — no offset subtraction
at access sites. This means a few low-index slots are wasted when `lo > 0`, but
it is safe to pass an array to a SUB and index it identically in both scopes.
`LBOUND` reads from the `array_lower` pre-pass map; `UBOUND = arr.len() - 1`.

### 4. SUB → Rust fn with &mut Runtime + &mut GameState
Every QB SUB becomes:
```rust
fn name(__rt: &mut Runtime, __gs: &mut GameState, ...) { }
```
`Runtime` carries I/O, graphics, RNG, sound. `GameState` (a generated struct)
carries all `DIM SHARED` variables and any scalars/arrays promoted across GOSUB
boundaries.

### 5. FUNCTION → Rust fn returning f64 or String
QB FUNCTIONs return by assigning to the function name. Emitted as:
```rust
fn name(__rt: &mut Runtime, __gs: &mut GameState, ...) -> f64 {
    let mut __fn_ret: f64 = 0.0;
    // ... body; assignments to name → __fn_ret ...
    __fn_ret
}
```

### 6. GOSUB targets → Rust fns
GOSUB-only targets (not reachable by GOTO) are extracted and emitted as named
`fn` blocks that receive `__gs` by reference. This is the clean path and covers
gorilla.bas entirely (no GOTO in that program).

### 7. GOTO → state machine fallback
When GOTO cannot be rewritten as a structured loop:
- Emit `let mut __pc: u32 = <first_line>;`
- Wrap body in `loop { match __pc { ... } }`
- Each numbered line is a match arm that falls to the next arm or sets `__pc`

### 8. Graphics: palette-indexed framebuffer, always open
`Runtime` stores a `Vec<u8>` of EGA palette indices (0–15) as the framebuffer.
`POINT(x,y)` returns the index at that pixel — collision detection in gorilla.bas
reads colour indices, not RGBA. Never convert to RGBA until `present()` blit.

The **window opens immediately** in `Runtime::new()` (eager, not lazy). The
`had_screen_call: bool` flag tracks whether any explicit `SCREEN N` call has
been made and controls two behaviours:
- `wait_for_key()` only blocks when `had_screen_call` is true — text-only
  programs exit immediately (so integration tests don't timeout).
- `print_gfx()` echoes to stdout when `!had_screen_call` so integration tests
  capture output; graphics programs are window-only.

### 9. PUT (sprite blit) always calls present() — except when vsync-paced
`put_sprite` (QB `PUT`) calls `self.present()` directly after each blit.
Sprite blits are game-level operations (1–2 per animation frame); always
flushing ensures animations like the banana flight are visible.
Pixel-level operations (PSET, LINE segments, CIRCLE points) use `auto_present()`
which throttles to one blit per 256 calls / frame interval.

**Exception**: a program that has synced with `WAIT &H3DA, 8` (the modeled VGA
retrace) sets `vsync_paced = true` on `Runtime`, and BOTH `put_sprite`'s
immediate present and `auto_present`'s timer are suppressed until the next
`SCREEN` call resets it. This is for demoscene-style code that composes an
entire frame (erase, PUT sprites, redraw overlays) and expects nothing to hit
the window until it flips at the WAIT — without this, mid-frame composition
states (e.g. a scroller's letters sitting on top of a pillar before the pillar
is redrawn over them) leak to the screen. gorilla/donkey never call WAIT, so
their behavior — and their golden checksums — are unaffected.

### 10. User-defined TYPEs — recursive flattening
TYPE fields are flattened to `__`-joined scalar variable names:
`player.Pos.X` → `player__pos__x`. The `flatten_type_fields(type_name, type_defs)`
free function in `emitter/scan.rs` recurses through nested UserType fields.

Keywords used as TYPE names (e.g. `TYPE Color` where `Color` is lexed as
`Token::Color`) are handled by `advance_as_type_ident()` in the parser.

Scalar TYPE parameters to SUBs are expanded to per-field `&mut f64` parameters.
Call sites expand the corresponding variable to `&mut var__field` for each field.

### 11. String handling
QB strings are value types → `String` in Rust. String literals → `&str` at
call sites. String functions (`LEFT$`, `MID$`, etc.) are free functions in
`lib.rs`, called as `qb_left(&s, n)` etc. — not methods.

### 12. Frame pacing
`auto_present()` fires at most once per `frame_interval_ms` (default 16ms ≈ 60fps).
This is skipped when `FULLSPEED` pragma is set (for compute-heavy programs).
The `REM QBC` pragma system provides: `FULLSPEED`, `FPS N`, `PACE N`,
`SLOWMO N`, `TITLE text`, `SCALE N`.

**`PACE N`** (vs `FPS`/`FULLSPEED`): the normal throttle and `FULLSPEED` only
*skip* blits that arrive too soon — they never block, so the *computation* always
runs at full native speed (a Mandelbrot finishes in ~ms regardless). `PACE N`
instead makes `auto_present()` **sleep** the remainder of each frame interval
(at `N` blits/sec), which blocks and therefore *paces the compute*, making an
otherwise-instant native draw watchable (it sweeps in roughly source-draw order).
Gated finer (every 64 calls) for smoothness; total run time scales with how much
the program draws, so tune `N` (lower = slower). `mandel.bas` uses `REM QBC PACE
30`. Implemented as `Runtime::set_pace` + the `pace_ms` branch in `auto_present`.

---

## GORILLA.BAS Specific Facts

Read `docs/gorillas.md` for the full architectural walkthrough. Key facts:

- **SCREEN 9 first, fallback to SCREEN 1** — gorilla.bas negotiates EGA (640×350)
  via `ON ERROR GOTO` and falls back to CGA (320×200). It does NOT use SCREEN 7.
  The `ON ERROR`/`RESUME` logic is safe to stub — just hard-select SCREEN 9.
- **No GOTO** — entire program uses GOSUB/RETURN and structured flow only.
  All GOSUB targets emit cleanly as named Rust functions.
- **Collision via POINT()** — banana flight loop samples the framebuffer palette
  index at the leading edge each step: colour 0 = background (keep flying),
  `SUNATTR` (3) and `y < SunHt` = sun hit (shock face), anything else = impact.
- **GET/PUT sprite system** — gorilla sprites are drawn once with vector graphics
  then captured with `GET` into `GorD&/GorL&/GorR&` arrays; from then on only
  `PUT` is used. Banana sprites are loaded from inline `DATA` statements.
- **PLAY for explosion and victory** — short MML strings, mix of foreground
  (`MF`) and background (`MB`) modes. Wired to rodio.
- **RANDOMIZE TIMER** — `qb_timer()` returns seconds since midnight as f64.
- **Scl() function** — scales pixel coordinates between EGA and CGA modes.
- **Rest() function** — a calibrated busy-wait; `CalcDelay!` probes machine
  speed at startup. In native Rust these run far faster than DOS. `inkey()`
  yields 1ms per iteration so Rest() is accurate to ~1ms. The DoExplosion
  circle-expansion loop has no Rest() calls and runs at native speed; use
  `QBC_PACE=10 ./bin/gorilla` to slow circle drawing to a visible pace.
  `QBC_PACE=10` is the recommended way to run gorilla for the full DOS-era feel.
- **Dead declarations** — `EndGame`, `ClearGorillas`, `Getn#` are declared but
  never defined. The transpiler can ignore them safely.

---

## EGA Palette (hardcoded, never change)

```rust
pub const EGA: [(u8, u8, u8); 16] = [
    (0,0,0),       (0,0,170),     (0,170,0),     (0,170,170),
    (170,0,0),     (170,0,170),   (170,85,0),     (170,170,170),
    (85,85,85),    (85,85,255),   (85,255,85),    (85,255,255),
    (255,85,85),   (255,85,255),  (255,255,85),   (255,255,255),
];
```

---

## QBasic Built-in Function Mapping

| QBasic        | Rust runtime fn        | Notes                           |
|---------------|------------------------|---------------------------------|
| INT(x)        | qb_int(x)              | floor()                         |
| FIX(x)        | qb_fix(x)              | trunc()                         |
| ABS(x)        | qb_abs(x)              | abs()                           |
| SQR(x)        | qb_sqr(x)              | sqrt()                          |
| RND           | rt.rnd()               | LCG seeded by RANDOMIZE         |
| SGN(x)        | qb_sgn(x)              | signum()                        |
| SIN/COS/TAN   | qb_sin/cos/tan(x)      | radians                         |
| ATN(x)        | qb_atn(x)              | atan(), NOT atan2               |
| LEN(s)        | qb_len(&s)             |                                 |
| LEFT$(s,n)    | qb_left(&s, n)         | 1-indexed                       |
| RIGHT$(s,n)   | qb_right(&s, n)        |                                 |
| MID$(s,p,n)   | qb_mid(&s, p, Some(n)) | 1-indexed                       |
| UCASE$/LCASE$ | qb_ucase/lcase(&s)     |                                 |
| STR$(n)       | qb_str_fn(n)           | leading space for positives     |
| VAL(s)        | qb_val(&s)             |                                 |
| CHR$(n)       | qb_chr(n)              |                                 |
| ASC(s)        | qb_asc(&s)             |                                 |
| INSTR(s1,s2)  | qb_instr(1.0,&s1,&s2)  | 1-indexed start                 |
| TIMER         | qb_timer()             | seconds since midnight          |
| POINT(x,y)    | rt.point(x, y)         | returns palette index as f64    |
| CINT(x)       | qb_cint(x)             | banker's rounding               |

---

## Emitted Code Structure

```rust
// Generated by qbc — QBasic to Rust transpiler
#![allow(non_snake_case, unused_variables, dead_code, unused_mut,
         unused_assignments, unused_parens, unreachable_code,
         non_upper_case_globals, const_item_mutation, clippy::all)]
use qbasic_runtime::*;

// DATA store (programs that use DATA/READ)
static __DATA: &[&str] = &["val1", "val2", ...];
static __DATA_PTR: std::sync::atomic::AtomicUsize = ...;

// Shared mutable game state (DIM SHARED + cross-GOSUB-boundary promotions)
#[derive(Default)]
struct GameState {
    gorilla_x: Vec<f64>,
    wind: f64,
    // ...
}

// SUBs
fn draw_gorilla(__rt: &mut Runtime, __gs: &mut GameState, x: f64, y: f64) { ... }

// FUNCTIONs
fn plot_shot(__rt: &mut Runtime, __gs: &mut GameState, ...) -> f64 {
    let mut __fn_ret: f64 = 0.0;
    // ...
    __fn_ret
}

fn main() {
    let mut __rt = Runtime::new();        // or new_configured() for TITLE/SCALE pragmas
    let mut __gs = GameState::default();
    // main body
    __rt.quit();  // END statement — waits for keypress in graphics programs
}
```

---

## Testing

```bash
cargo build --release          # build transpiler + runtime
bash tests/run-tests.sh        # 27 integration tests — must all pass
bash tests/run-tests.sh -v     # verbose: show actual vs expected on failure
cargo test --workspace         # unit tests (lexer, print_using, draw)
```

Never break the integration tests. Before any PR run the full suite.
The bundled programs in `basic-src/` are for manual/visual verification only.

---

## Common Pitfalls

1. **PRINT semicolon vs comma**: `PRINT A; B` = concatenated, `PRINT A, B` = next tab zone (every 14 columns)
2. **Array indexing**: wasted-slots — never subtract the lower bound at access sites; LBOUND comes from the `array_lower` pre-pass map
3. **TYPE flattening**: `DIM p AS Pixel` where `Pixel` has `Col AS Color` → `p__col__r`, `p__col__g`, `p__col__b` — NOT `p__col`. Use `flatten_type_fields()`.
4. **Keyword TYPE names**: `TYPE Color` — `Color` is `Token::Color` (a keyword), not `Token::Ident`. Use `advance_as_type_ident()` when parsing a TYPE name or field type.
5. **Scalar TYPE SUB params**: `SUB Foo(p AS Pixel)` must expand to per-field `&mut f64` params; call sites must expand the arg to per-field `&mut var__field`.
6. **LINE syntax**: `LINE (x1,y1)-(x2,y2), color [,B[F]]` — the `-` between coord pairs is not subtraction
7. **CIRCLE aspect ratio**: defaults to `0.8333` for SCREEN 9 (EGA 640×350 has non-square pixels); pass explicitly to `rt.circle()`
8. **PAINT boundary**: flood fill stops at `border_color` exactly — wrong colour bleeds through gorilla sprites
9. **GOSUB vs SUB**: `GOSUB 100` jumps to a line-label in the same scope; `CALL MySub` calls a named SUB — both appear in QB programs, both must work
10. **SCREEN 0 after graphics**: gorilla.bas calls `SCREEN 0` inside `Intro` (text mode title screen) even though the window is already open from `SCREEN 9` in `InitVars`. The `had_screen_call` flag handles this — all text still renders in the open window.
11. **PUT always presents**: `put_sprite` calls `present()` directly (not `auto_present()`). Do not revert this — banana animation becomes invisible without it. (Suppressed only when `vsync_paced` — see §9.)
12. **PRINT USING field width**: the field width is the literal character count of the format spec. The previous off-by-one that padded every field one space too wide is fixed. `^^^^` exponential notation and `%` wide-field overflow are implemented.
13. **Multi-statement lines**: `A = 1 : B = 2 : PRINT A+B` — colon separates statements; the lexer emits `Token::Newline` for both `\n` and `:`.
14. **QB1.1 DOS compatibility (`.bas` files only)**:
    - **Variable names must not shadow QB built-in functions** — e.g. `pos` conflicts with `POS()`, causing a misleading "Expected: SHARED" parse error. Rename the variable.
    - **`ON ERROR GOTO` targets must be module-level labels** — a label inside a SUB/FUNCTION is not reachable by `ON ERROR GOTO`; QB reports "Label not defined". Remove the handler or restructure (e.g. use LOF check instead).
    - **`_` in identifiers is illegal in QB1.1** — underscore is only valid as end-of-line continuation. Use run-together names (`INVCOLS` not `INV_COLS`).
    - **`FUNCTION Foo() AS INTEGER` not supported in QB1.1** — use sigil form: `FUNCTION Foo%()`.
    - **Reserved words as identifiers** — `timer`, `fNum` (FN prefix), etc. will error. Rename to avoid the collision.
    - **File must have CRLF line endings for DOS QBasic** — Python text-mode I/O silently strips CR on macOS; always use binary mode (`open(f, 'rb'/'wb')`) and explicitly apply `\r\n`.

---

## Recently Added Language Features

- **`COMMON [SHARED] varlist`** — parsed like `DIM SHARED` (the variables become
  `GameState` fields), and the ordered list is recorded on
  `Program::common_decls` for `CHAIN`'s positional value passing.
  (`Token::Common`, `parse_common` in `parser.rs`.)
- **`STATIC var [AS type]`** (statement form, inside a SUB/FUNCTION) — a local
  that persists across calls. Emitted as `Stmt::SharedDecl`, so it rides the
  shared-promotion path and becomes a persistent `GameState` field. Caveat:
  same-named STATIC locals in different procedures would alias. (The `SUB … STATIC`
  *suffix* is separate — still just skipped to EOL.)
- **Random-access TYPE records `GET/PUT #n, rec, var`** — fully serialized and
  **persisted to disk** (cross-run). The record variable is captured as an
  `LValue` (`parser.rs`), and a side **layout table** (`type_layouts`:
  type_name → `[(field, FieldRepr)]`) is built at TYPE-parse time where the
  `STRING * n` length and the INTEGER/LONG distinction are still visible
  (`FieldRepr::{Str(n),I16,I32,F32,F64,Nested}`; `field_repr()` in `parser.rs`).
  The emitter (`record_layout()` + `record_get_line()/record_put_line()`)
  recurses the layout, computes byte offsets, and emits per-field
  `qb_rec_put_*`/`qb_rec_get_*` calls (runtime: fixed strings = raw bytes,
  numerics little-endian) around the existing `read_record`/`write_record`
  (which already do real on-disk byte I/O). f64 storage is unchanged — the
  layout table only describes packing. A **bare array name** with no subscript
  (`PUT #1, n, HALLFAME`) is QB-faithful = element at lbound (`HALLFAME(1)`):
  DOS writes only the first element per record (aster7's latent leaderboard
  quirk is reproduced exactly). The `FIELD`-based path is unchanged and still
  used when there's no TYPE record var. **Caveat:** SINGLE/DOUBLE fields use
  IEEE LE, not QuickBASIC's Microsoft Binary Format, so a record file with
  SINGLE/DOUBLE fields is not byte-identical to one written by DOS QBasic 1.1
  (INTEGER/LONG/fixed-STRING — all aster7 uses — are byte-exact). Covered by
  `record_tests` (runtime) and `tests/programs/record_io.bas` (integration).
- **`STEP` relative graphics coordinates** — `PSET`/`PRESET`, `LINE`, `CIRCLE`,
  `GET`, `PUT` accept a `STEP(dx,dy)` coordinate prefix meaning "relative to the
  current graphics cursor (last point referenced)". Parsed via `opt_step()` in
  `parser.rs` (each coord pair carries a `step`/`step1`/`step2` flag);
  `emitter/mod.rs` lowers a relative point to `__rt.cur_x()/cur_y() + delta` temps
  and calls the absolute runtime methods. Semantics: a single point and a LINE/GET
  *first* point are relative to the cursor; a LINE/GET *second* `STEP` point is
  relative to the *first point* (not the cursor). Non-STEP statements emit
  byte-identical output to before. Runtime adds `cur_x()`/`cur_y()` getters, and
  `circle()` now moves the cursor to its center (QB LPR behavior). `GET`/`PUT` do
  not move the cursor. Covered by `step_tests` (runtime) and `basic-src/step.bas`.
- **`PUT` sprite action verbs** — `PUT (x,y),arr[,verb]` supports all five QB
  verbs `PSET`/`PRESET`/`AND`/`OR`/`XOR`, and the **default verb (none written)
  is `XOR`** (QB semantics), not PSET. AST carries `PutAction` (`parser.rs`);
  the runtime `put_sprite` dispatches per verb (`runtime/src/lib.rs`), with
  `PRESET` inverting within the mode's pixel depth via `sprite_color_mask()`
  (CGA=3, EGA=15, mode13=255). Fixes `donkey.bas`: the car (`PRESET`), the donkey
  (`PSET`), and the explosion / road animation (bare `PUT` = `XOR` draw-erase).
  `gorilla.bas` is unaffected — every gorilla PUT is an explicit `PSET` or `XOR`.
  Covered by `sprite_tests` (runtime).
- **CGA SCREEN 1 GET/PUT sprite format** — mode 1 uses the authentic QuickBASIC
  **2-bpp packed INTEGER-array** layout (`data[0]=width×2`, `data[1]=height`,
  then `ceil(width/4)` bytes/row at 2 bits/pixel MSB-first, two bytes per
  element), via a `screen_mode == 1` branch in `get_sprite`/`put_sprite`
  (`runtime/src/lib.rs`). Every other mode keeps the EGA 4-plane planar layout
  (single 32-bit header) byte-identically, so gorilla/step (SCREEN 9) are
  unaffected. This makes donkey's hand-built `B%` road-dash strip render, so the
  scrolling dashed center-line now animates; GET-captured CGA sprites (CAR%,
  DNK%, explosion) round-trip through the same layout. donkey is the only bundled
  CGA-sprite program. Covered by `cga_sprite_tests` (runtime). (SCREEN 2's 1-bpp
  sprites are still unhandled — no program uses them.)
- **`DRAW "M x,y"` relativity** — a leading sign on the **X** coordinate makes the
  whole move relative (`x` *and* `y` added to the current point); no sign = an
  absolute move. The Y sign only sets its own direction — it does **not**
  independently switch the mode (`runtime/src/lib.rs` `draw()`). This is the QB/
  GW-BASIC rule; the previous code decided each axis independently, so a move like
  `M-1,1` (signed x, bare y — common in donkey's sprite outlines) wrongly jumped
  Y to the absolute coordinate, shattering the outline and making `PAINT` flood
  the region (donkey rendered as a solid white box). Only `donkey.bas` uses
  `DRAW M` among the bundled programs. Covered by a `draw_m_*` test in
  `sprite_tests`.
- **`DRAW` default color follows `COLOR`** — a `DRAW` string with no `C` verb
  paints in the *current COLOR foreground* (QB behavior). `color()` now syncs
  `draw_color = fg_color` (`runtime/src/lib.rs`). Previously `draw_color` was
  only seeded in `screen()` and went stale after a `COLOR` call, so an
  uncoloured `DRAW` (e.g. donkey's `"S08"` sprite outline) drew in the old
  default color while the following `PAINT (x,y),3` looked for border color 3 —
  mismatch → flood-fill leak → solid white sprite. This (plus the `DRAW M` fix)
  is what made `donkey.bas`'s donkey render as a white box. The car was spared
  because it sets the color inline via `DRAW "S8C3"`. Covered by
  `draw_uses_current_color_foreground` in `sprite_tests`.
- **`DRAW "N"` no-advance modifier** — `N` before a direction draws the segment
  but leaves the cursor where it started (a "spur"). `self.line()` advances the
  cursor to the endpoint internally, so `N` must *restore* the origin, not merely
  skip a second advance (`runtime/src/lib.rs` `draw()`, both the `M` and
  directional branches). Previously the cursor drifted to each spur's end, so
  donkey's car sprite (`...R1ND2u1...`, several `ND2` spurs) had a misplaced
  outline that didn't close → `PAINT (1,1)` (which fills the exterior to be
  PRESET-inverted into a white car) flooded the body, leaving only a few
  fragments. The donkey was spared because it uses no `N` commands. Covered by
  `draw_n_modifier_does_not_advance_cursor` in `sprite_tests`.

### From torus.bas (SCREEN 12 — arrays of TYPE, WINDOW/PMAP, VGA palette)
- **FUNCTION parameters pass by reference for UDT params.** QB passes ALL params
  byref; a FUNCTION that mutates a `TYPE` arg and lets the caller read it back
  relies on it (torus's `Inside()` sets `T.xc`/`T.yc`, which `TileDraw` then uses
  to PAINT). `setup_param_sets` now registers UDT FUNCTION param fields as
  `numeric_params` (byref), and call sites pass per-field `&mut`. Plain numeric
  FUNCTION params stay by-value (return via the fn result). Without this every
  tile painted at (0,0) → black screen.
- **`WINDOW` without an explicit `VIEW` maps onto the WHOLE screen.** Previously
  `view_x1..view_x2` were 0 → everything collapsed to pixel (0,0).
  `effective_viewport()` falls back to the full framebuffer. Used in
  `logical_to_fb` and `pmap`.
- **Plain `WINDOW` inverts the Y axis** (Cartesian, y-up). torus's `Inside()`
  row-scan depends on it; mandel is vertically symmetric so unaffected. (See
  `WINDOW SCREEN` below for the non-inverting variant.)
- **`SCREEN 11/12` `PALETTE` takes an 18-bit VGA DAC value** (`r + 256*g +
  65536*b`, each 0–63), like SCREEN 13 — not the EGA irgb nibble. Otherwise most
  tiles decode to black.
- Also landed: `SHARED name AS type` inside a SUB body (consume+discard the type);
  `PAINT STEP(dx,dy)`; passing a typed-array ELEMENT to a SUB (`TileDraw
  T(Index(Til))`); scalar UserType → per-field GameState fields; `REDIM … AS Tile`
  resizes each field Vec; per-sub `shared_names` scoping (DIM SHARED vs explicit
  `SHARED` in a sub); `SCREEN n, , m` empty middle args. Walkthrough: `docs/torus.md`.

### From reversi.bas (SCREEN 9 — game, 2-D/3-D arrays, WINDOW SCREEN)
- **`WINDOW SCREEN`** = screen-orientation Y (NO inversion) AND mapped by
  coordinate *magnitude* (min → top-left) so reversed corners don't flip the
  image. reversi passes `(640,480)-(0,0)`; a naive corner-order map rendered the
  board rotated 180° on the wrong side with backwards arrow keys. `win_screen`
  flag on Runtime; handled in `logical_to_fb` + `pmap`.
- **`ERASE name[,…]`** — `Token::Erase`/`Stmt::Erase`; `emit_erase` zeroes arrays
  in place with loop-nesting matched to dimensionality (`array_dims` map).
- **3-D plain arrays** (`DIM GP(8,8,8)`) — `nested_vec_type`/`nested_vec_init`
  helpers; threaded through GameState struct decl, `emit_dim`, `emit_redim`.
  (2-D arrays of a TYPE already worked.)
- **Scalar/array same-name coexistence** — QB lets `A$` and `A$()` be distinct
  variables (reversi's DisplayHelp). `local_scalar_name()` suffixes the colliding
  scalar binding `__sc`.
- Shared-field args to user FUNCTIONs are hoisted to temps in a block expr to
  avoid the `&mut __gs` borrow conflict. Walkthrough: `docs/reversi.md`.

### From qblocks.bas (SCREEN 7/8 — last bundled program; build-all 24/24)
- **Zero-arg FUNCTION called WITHOUT parens is a CALL.** `IF CheckFit = FALSE`
  calls `CheckFit()` in QB. `emit_expr_inner` turns a bare reference to a zero-arg
  user FUNCTION into a call (READ path only — assignment to the function's own
  name still maps to `__fn_ret`). Two corollaries:
  - **Never declare a local for a function name** — `collect_locals` was emitting
    an f64 local for the bare reference, shadowing the fn (so gorilla's
    `CalcDelay` / nibbles' `StillWantsToPlay` silently read 0). `emit_locals`
    excludes `user_fns`.
  - **Never promote a CONST to GameState** — `CONST TRUE = -1` referenced across
    scopes was promoted to a `qb_true` field (default 0.0) shadowing the const →
    `BadMode = TRUE` broke qblocks' screen probe. Cross-boundary scalar promotion
    excludes const names.
- **Whole-record copy of a scalar TYPE var** — `OldBlock = CurBlock` → per-field
  assignment (`emit_scalar_type_copy`), the scalar analog of the array-of-TYPE
  copy. Shared TYPE-field call-args use the hoist-to-temp + writeback path.
- **`fold_const` handles `\` (IntDiv) and `MOD`** — round both operands (CINT
  banker's) then i64 div/rem, matching runtime `qb_idiv`/`qb_mod`. Previously a
  `CONST … \ 2` was silently dropped → undefined at its use site.
- **`PALETTE USING <bareArrayName>`** (no subscript) — resolve to the array
  binding and slice from the lower bound; do NOT route a bare array name through
  scalar disambiguation (was producing `colors__sc[..]` on an f64).

### From kitchen_sink-gw.bas (GW-BASIC "mega test" — menu loop, ON GOTO/GOSUB)
- **`ON expr GOTO/GOSUB label,label,…`** — computed branch, now parsed (was
  skipped to EOL → silently dropped). AST: `Stmt::OnGoto { expr, labels, is_gosub }`
  (`parser.rs`). Emitter lowers to `match qb_cint({expr}) as i64 { 1 => <goto/gosub
  label1>, 2 => …, _ => {} }` (1-indexed, out-of-range falls through — QB
  semantics). Wired into `collect_goto_targets`/`collect_gosub_targets` and
  `stmt_has_numeric_goto` so the targets join the `__pc` state machine / GOSUB-fn
  set correctly.
- **A promoted/shared scalar must use the SAME name in every emission path.**
  Two bugs surfaced together (45 rustc errors):
  - **FOR counter** — `Stmt::For` hardcoded the bare `rust_ident(var)` for
    init/condition/increment, so a counter read across a GOSUB boundary (promoted
    to `__gs.i`) emitted undeclared `i`. Fix: compute `vref =
    emit_lvalue(Scalar{var})` (yields `__gs.i` / `i` / `(*i)`) for the three
    counter touch-points; keep bare `v` only as the `__for_to_/__for_step_` temp
    suffix. Byte-identical for local counters.
  - **Promoted string field naming** — promoted scalars were *stored* with
    `rust_ident_typed` (string → `_s` suffix) but *referenced* via `emit_lvalue`
    as bare `rust_ident` (the torus `Available$`→`available` rule), so `A$` had
    field `a_s` while every use was `__gs.a`. Fixed the field decl + dedup set to
    bare `rust_ident` (numerics unaffected; promoted strings were already orphaned,
    so only a fix).
- **`lift_expr` hoists borrow-conflicting subexprs in PRINT/args.** `rnd`/`inkey$`/
  `pmap`/`input$` and any user-fn numeric arg containing `__gs` are hoisted to a
  `let __tmpN = …;` temp first, so `PRINT INT(RND*100)` and `FNSQ(I)` (→
  `fnsq(&mut __gs, __gs.i)`) don't double-borrow `__gs`.

### GW-BASIC physical line continuation (evil.bas, pokeit.bas)
- **Physical line continuation** — in GW-BASIC (and any line-numbered BASIC), a
  physical file line that does NOT begin with a line number is a continuation of
  the previous logical line. The lexer (`src/lexer.rs`) now detects line-numbered
  mode (first `IntLit` seen at a statement-start position sets
  `in_line_numbered_mode = true`), then suppresses `Newline` tokens at `\n`
  boundaries when the next physical line starts with whitespace / non-digit.
  Non-line-numbered programs are byte-identical (the `in_line_numbered_mode` flag
  stays `false` → else branch → old path). The three continuation cases that
  appear in `evil.bas` (`PRINT … CHR$(…);`, `NEXT I: RETURN`, `GOTO 140` spread
  across two physical lines) all parse correctly.
- **`POKE addr, val`** — parsed as `Stmt::Poke { addr, val }` (`parser.rs`).
  Previously a silent no-op; now calls `__rt.qb_poke(addr, val)` which stores
  `val & 0xFF` in a `HashMap<u32, u8>` on `Runtime` (`poke_mem` field). The same
  byte is returned by subsequent `PEEK(addr)` calls.
- **`PEEK(addr)`** — previously stubbed to return `0.0`; now calls
  `__rt.qb_peek(addr)` → looks up `poke_mem` → returns the stored byte or `0.0`
  if never written. Added to the `lift_expr` hoist table so `PRINT PEEK(...)` doesn't
  double-borrow `__rt`.
- **`evil.bas`** — GW-BASIC "self-modifying POKE matrix" demo; all three physical
  line continuations parse, POKE/PEEK memory round-trips. `basic-src/evil.bas`.
- **`pokeit.bas`** — minimal POKE→PEEK→PRINT regression test. Line-numbered GW-BASIC
  style; `POKE 1040, D` then `PRINT PEEK(1040)` outputs ` 25`. `basic-src/pokeit.bas`.

### money.bas full support — FIELD binary I/O, CP437 font, INPUT# trim (build-all 33/34)
`money.bas` is a Microsoft 1990 money-manager with random-access binary file I/O,
CP437 box-drawing in text menus, and color preference save/load. Four interlocking
fixes were needed:

- **UTF-8 source decoding** (`src/main.rs`) — `money.bas` is saved as UTF-8.
  The source reader now tries `std::str::from_utf8()` first and falls back to
  `byte as char` for genuine Latin-1/CP437 DOS files (nibbles.bas etc.). Previously
  the multi-byte fallback was applied unconditionally, splitting U+00C4 (Ä) into two
  junk chars and corrupting string literals.
- **CP437 / extended ASCII font** (`runtime/src/lib.rs`) — `FONT_8X8` extended from
  128 to 256 entries covering the full CP437 character set (box-drawing characters
  0x80–0xFF such as `─ │ ┌ ┐ └ ┘ ├ ┤ ┬ ┴ ┼`, block elements `█ ▓ ▒ ░`, etc.).
  `draw_char_fb` changed to use the full code-point index instead of masking to
  0x7F, so all 256 glyphs render correctly.
- **Latin-1 binary string encoding** (`runtime/src/lib.rs`) — `MKD$`/`CVD`,
  `MKI$`/`CVI`, `MKS$`/`CVS`, `MKL$`/`CVL` store IEEE 754 little-endian bytes as
  Latin-1 characters (byte `b` → `char::from_u32(b as u32)` and back). `qb_lset`
  / `qb_rset` use `.chars().count()` (not `.len()`) for correct binary-string
  padding. `qb_field_get`/`qb_field_put` use the same Latin-1 encoding for
  FIELD-based random-access record buffers. INTEGER/LONG/fixed-STRING fields are
  byte-exact with DOS QBasic 1.1; SINGLE/DOUBLE use IEEE LE (not MBF), same caveat
  as the typed-record path.
- **`INPUT #n` numeric parse trim** (`src/emitter.rs`) — `qb_print_num(1.0)`
  emits `" 1 "` (QB's leading-space convention). Reading it back with
  `INPUT #1, ColorPref` and Rust's `.parse::<f64>()` fails silently (Rust rejects
  leading whitespace), returning `0.0`. Fixed by adding `.trim()` before `.parse()`
  for both `INPUT #n, numVar` (file) and interactive `INPUT numVar`. Root cause of
  the "all menus appear black" symptom — `colorpref=0` → `colors[x][0]` (never
  populated, DATA fills indices 1–4) → `color(0, Some(0))` → black-on-black.
- **`local_dim_names` shadowing** (`src/emitter.rs`) — a `HashSet<String>` tracks
  explicit `DIM` declarations per scope so a local integer `B` is not shadowed by a
  promoted shared string `B$` (stored with a different typed name but same base
  ident). Type-aware exclusion: string shared vars excluded by typed name (`b_s`),
  numeric by bare name (`b`).
- **Parser additions** (`src/parser.rs`):
  - `ON KEY(n) GOSUB/GOTO`, `KEY(n) ON/OFF/STOP`, `TIMER ON/OFF/STOP` → no-ops
    (consume to EOL; event traps not modelled).
  - `CLEAR` statement → skip to EOL, return `None`.
  - `REDIM SHARED` — propagates `shared=true` to `parse_var_decl` correctly so
    REDIM'd arrays shared across subs land in `GameState`.
  - Removed dead duplicate FIELD handler (41 lines) that shadowed the correct
    `parse_field()` and silently discarded all field-length information.
- **`pokemix.bas`** and **`qmaze.bas`** — two new programs added to `basic-src/`
  that now pass `build-all.sh`.

### kitchen_sink-qbasic.bas (QBasic 4.5 mega-test — build-all 35/35)
`kitchen_sink-qbasic.bas` is a QBasic 4.5-dialect mega-test with 9 menu items
dispatched via `ON Choice GOTO named_label`. Three interlocking fixes were needed:

- **Standalone `LOOP` no-op** (`src/parser.rs`) — the program has one `DO` (line 25)
  but TWO `LOOP` statements: the real closing one at line 45 (consumed by `parse_do`'s
  `expect(&Token::Loop)`) and a bare unreachable `LOOP` at line 232 (`ContinueLoop:`
  restart target, dead code after ON GOTO→GOSUB extraction). Added `Token::Loop` as a
  no-op case in `parse_stmt` — fires only for bare LOOPs not inside a DO block.

- **ON GOTO → GOSUB extraction** (`src/emitter.rs`) — named ON GOTO targets (ArrayTest,
  StringTest, etc.) are extracted as GOSUB fns via `collect_gosub_targets` (added
  `OnGoto { is_gosub: false }` arm for non-numeric labels) and emitted as direct
  function calls in the `emit_stmt` `OnGoto` handler when `user_fns` contains the label.
  After each section fn returns, the DO loop re-displays the menu.

- **Cross-boundary array promotion fixes** (`src/emitter.rs`) — three bugs in the array
  promotion pipeline:
  1. `Expr::Call` stores names with sigil (`"Names$"`) but `shared_names`/`array_names`
     keys are sigil-free (`"names"` from `VarDecl.name`). Fixed: use `name_bare =
     name.trim_end_matches(['$','%','!','#','&']).to_lowercase()` for set lookups in
     both `lift_expr` and `emit_expr_inner` (separate from `name_lc` used for built-in
     function checks that require the sigil, e.g. `"inkey$"`). Same fix in `scan_expr`
     inside `collect_array_use_refs_stmt`.
  2. Promoted arrays were emitted as `Vec<f64>` in GameState regardless of type/dims.
     Fixed `emit_game_state` to look up each promoted name in `scope.symbols` via
     `rust_ident_typed` and emit correct type (`Vec<String>` for string arrays) and
     dimensionality (`Vec<Vec<f64>>` for 2-D arrays).
  3. `shared_types` was not populated for cross-boundary promoted arrays, so
     `emit_expr_inner` fell back to bare name instead of typed name. Fixed: after
     `detect_cross_boundary_arrays`, look up each promoted name in `prog.global_scope`
     and insert its type into `shared_types`.

### Headless driver + tooling (debugging / graphics tests)
- **Any transpiled binary honors `QBC_*` env vars** (read once in
  `new_configured`) to run windowless — no codegen change, byte-identical when
  unset: `QBC_HEADLESS`, `QBC_KEYS="DOWN,DOWN,ENTER,Q"` (scripted INKEY$/INPUT,
  maps identically to real keys via `normalize_key`), `QBC_SEED` (pins the RNG
  past `RANDOMIZE TIMER` via `seed_locked`), `QBC_DUMP=out.ppm`,
  `QBC_DUMP_AT=exit|present:N|ms:T`, `QBC_CHECKSUM`, `QBC_FBSTATS`,
  `QBC_EXIT_AFTER=idle|ms:T|presents:N` (guaranteed termination + 10 s safety),
  `QBC_TEXT_FB` (render text INTO the fb for full-screen screenshots — OFF by
  default so goldens stay graphics-only). Runtime: `fb_to_rgb`, `export_ppm`,
  `fb_checksum`, `inject_key`, `headless_finish`/`headless_tick`. The input-hang
  guard in `input_str`/`input_line` + the `Drop` hook ensure a scripted run always
  dumps and exits.
- **`DRAIN` / `BARRIER` sentinel in `QBC_KEYS`** — some QB programs have a
  `WHILE INKEY$ <> "": WEND` buffer-drain idiom (e.g. SparklePause and GetNum#
  in gorilla.bas). Since all scripted keys are pre-loaded at startup, a drain loop
  would consume them all before the real input-read. `normalize_key("DRAIN")` →
  `"\x00"` (null byte); `inkey()` returns `""` when it pops `"\x00"`, causing the
  drain loop to exit while leaving subsequent keys intact. `BARRIER` is an alias.
  Usage: `QBC_KEYS="DRAIN,ENTER,..."` — place a DRAIN immediately before every
  `WHILE INKEY$<>"":WEND` flush in the program's execution path. Headless graphics
  programs (had_screen_call=true, no window) silently suppress PRINT output to
  prevent native-speed spam from cursor-blink loops — does not affect integration
  tests (text programs) or QBC_TEXT_FB screenshots.
- **Graphics golden tests** — `tests/run-graphics-tests.sh` runs a program
  headless (seed + key script) and diffs its `fb_checksum` against
  `tests/golden/<name>.txt`. A good golden draws once and stops (or finishes
  within the exit window); mandel is EXCLUDED (slow draw + infinite palette cycle
  = no reproducible snapshot). `--write-golden` regenerates.
- **Screenshots** — `tools/ppm2png.py` (pure-stdlib PPM→PNG, scales to 960×600
  nearest-neighbor like the window). Capture: `QBC_TEXT_FB=1 QBC_DUMP=x.ppm` →
  `python3 tools/ppm2png.py x.ppm docs/screenshots/x.png`. README has a gallery.

### `VAL("&H…")` / `VAL("&O…")` hex and octal support (etto.bas)
- **`qb_val` &H/&O prefix handling** (`runtime/src/lib.rs`) — `qb_val` now
  recognises `&H`/`&h` (hex) and `&O`/`&o` (octal) prefixes before the existing
  decimal parser, matching QB semantics. Previously `VAL("&H6F")` returned `0.0`,
  silently breaking any program that decodes hex strings at runtime. Root cause of
  etto.bas showing a uniform cream/white image (all pixel indices decoded as 0).
  Added 5 unit tests: `&H6F`→111, `&hFF`→255, `&H0`→0, `&O10`→8, `&o77`→63.

### INVADERS.BAS (QB4.5 Space Invaders — build-all 38/38; QB1.1 DOS-compatible)
`INVADERS.BAS` is a 1730-line QBasic 4.5 Space Invaders port with SCREEN 13 (256-color
VGA), TYPE definitions, GOTO inside SUBs, binary file I/O (high-score persistence),
FREEFILE, LOF(), and QB4.5-specific syntax. Four interlocking transpiler fixes were
needed to get `qbc` to accept it, then a round of QB1.1 DOS compatibility fixes to
make it load and run correctly in real QBasic:

#### Transpiler fixes (src/ changes)
- **QB4.5 `_` line continuation** (`src/lexer.rs`) — in QB4.5 a line ending with `_`
  followed by a newline continues on the next physical line (distinct from GW-BASIC's
  line-number-based continuation). The lexer detects a bare `_` word followed by
  optional whitespace + newline and consumes the newline without emitting a token, in
  the `in_line_numbered_mode = false` path. 18 continuations in INVADERS; zero effect
  on non-QB4.5 programs.
- **Double-comma `GET #n, , var`** (`src/parser.rs`) — sequential binary file access
  with no explicit record number uses `,,` (empty record position). `parse_get` and
  `parse_put` now check for `self.peek() == &Token::Comma` after advancing past the
  first comma, returning `None` for the record-number slot when the second comma or EOL
  follows.
- **`AS STRING` typed parameters** (`src/emitter.rs`) — QB allows `nm AS STRING` (no
  sigil) in SUB/FUNCTION params. The Rust param is renamed `nm_s: &mut String`. Four
  related emitter fixes:
  1. `emit_lvalue` detects `str_params` membership for sigil-less names and returns the
     `_s`-suffixed name, enabling correct deref-assign in `Stmt::Let`.
  2. `Stmt::Let` str_params arm broadened from `ty: QbType::String` to `..` so sigil-less
     params (AST type = Single) also emit `*nm_s = (rhs).to_string()`.
  3. `is_str_expr_ctx` extended to recognize local string array `Expr::Call` accesses
     (e.g. `rankStr(i)` where `DIM rankStr(1 TO 10) AS STRING`).
  4. `emit_call_args` and `lift_expr` extended with `local_string_arrays` checks so
     `rankStr(i)` is emitted as `rankstr_s[(i) as usize]` (array element) rather than a
     function call.
- **Local string arrays** (`src/emitter.rs`) — `DIM rankStr(1 TO 10) AS STRING` declares
  a string array without `$` sigil. A new `local_string_arrays: HashSet<String>` field
  (populated by `collect_local_string_arrays()`) tracks these per sub/function. Used in
  `emit_lvalue` (Index), `emit_expr_inner` (Call), `lift_expr`, `is_str_expr_ctx`, and
  `Stmt::Let` to produce `rankstr_s[...]` instead of `rankstr[...]` or `rankstr(...)`.
- **Array param dimensionality** (`src/emitter.rs`) — `DrawSpriteArr(spr() AS INTEGER)`
  declares `spr` as a 1D placeholder but the body accesses `spr(c, r)` (2D). A new
  `array_param_used_dims()` free function scans the sub body and returns the max index
  depth actually used, so `emit_params` emits `spr: &mut Vec<Vec<f64>>` correctly.

#### QB1.1 DOS compatibility fixes (basic-src/invaders.bas only)
These are `.bas`-only changes — no transpiler modifications — made so the file loads
and runs correctly in real QBasic 1.1 under DOSBox. Applied after all transpiler work.

**DOSBox speed**: set `cycles=27000` in your DOSBox config to emulate a 486 DX2/66
(the era this game targets). Use Ctrl+F12/Ctrl+F11 to adjust live. `cycles=max`
runs at full host speed.

- **Unicode box-drawing → ASCII** — QB1.1 uses CP437; UTF-8 multi-byte characters
  (─, ═, ║, etc.) displayed as garbage. Replaced with plain ASCII (`-`, `=`, `|`).
- **CRLF line endings** — DOS QBasic requires `\r\n`. File is kept in binary mode;
  every Python edit script must explicitly re-apply CRLF (text mode on macOS silently
  strips the CR on read and doesn't restore it on write).
- **`_` identifiers renamed** — QB4.5-style `_` in CONST names (e.g. `SCR_W`,
  `STATE_TITLE`) is illegal in QB1.1 (underscore at end-of-line is the only valid
  `_` use — as a line continuation). All 38 underscore CONSTs renamed to run-together
  form: `SCR_W`→`SCRW`, `STATE_TITLE`→`STATETITLE`, `INV_COLS`→`INVCOLS`, etc.
- **`timer` TYPE field** — QB reserved word. Renamed to `tmr` in `ExplType` and all
  four use sites.
- **`FUNCTION BoxHit() AS INTEGER`** — QB1.1 does not support `AS built-in-type` on
  the FUNCTION declaration line. Changed to sigil form: `FUNCTION BoxHit%(...)` with
  all call sites updated to `BoxHit%(...)` and return assignments to `BoxHit% = 1/0`.
- **`fNum` variable** — QB1.1 reserves the `FN` prefix for `DEF FN` user functions.
  Renamed to `fileN` in `LoadHiScores` and `SaveHiScores`.
- **`_` line continuations eliminated** — QB4.5 supports `_` at end-of-line as a
  continuation; QB1.1 does not. All 16 continuation instances collapsed to single lines
  (SUB parameter lists, IF conditions, call arguments, LINE statements).
- **`pos` variable renamed to `charPos`** — `POS` is a QB1.1 built-in function.
  Using `pos` as a local variable in `EnterInitials` caused QB to misparse
  `DIM pos AS INTEGER` and emit a misleading "Expected: SHARED" error. Renamed
  throughout `EnterInitials`.
- **`ON ERROR GOTO DefaultScores` removed** — QB1.1 requires `ON ERROR GOTO` targets
  to be module-level labels; `DefaultScores:` is inside `SUB LoadHiScores`. The
  handler was unnecessary: `OPEN FOR BINARY` creates a missing file (leaving
  `LOF = 0`), and the existing `IF LOF(fileN) = 0 THEN GOTO DefaultScores` check
  already handles first-run initialization.

### New programs: duck.bas and etto.bas
- **`basic-src/duck.bas`** — SCREEN 9 (EGA 640×350, 16 colors) cartoon duck drawn
  entirely with `DRAW` (turtle-graphics closed paths) and `PAINT` (flood fill).
  Back-to-front draw order: sky/water → sun → tail → body → wing → head → beak →
  eye. All DRAW paths are geometrically closed; interior paint points verified
  inside each closed outline. Golden-tested (`tests/golden/duck.txt`).
- **`basic-src/etto.bas`** — SCREEN 13 (320×200, 256 colors) VGA photo display.
  256-color custom palette + 190 DATA lines of 2-hex-char-per-pixel image data
  (97×190 px, centered). Uses `VAL("&H" + MID$(row$, …, 2))` to decode pixels —
  the fix above was required for correct rendering. `REM QBC PACE 8` slows pixel
  draw to a watchable DOS-era pace. Generated from source photo via Pillow
  MEDIANCUT quantization. Not fully QB 1.1-compatible (requires `:` on WHILE line,
  PALETTE in SCREEN 13, and &H decode in VAL) but runs correctly under qbc.

### QB-fidelity fixes from the June 2026 transpiler code review
Ten fixes from a full src/+runtime/ review; regression-tested by
`tests/programs/qb_semantics.bas` + `rng_and_logic_tests` (runtime):

- **Operator precedence corrected** (`src/parser.rs`): QB binds `*`/`/` tighter
  than `\` tighter than `MOD` — the chain was exactly inverted (`2 * 3 MOD 4`
  emitted 6, QB gives 2). Chain is now `add → mod → intdiv → mul → negate → pow`.
- **`^` is left-associative** (`parse_pow`): `2^3^2` = `(2^3)^2` = 64 (was
  right-assoc → 512). A unary sign on the exponent (`2^-3`) parses via
  `parse_pow_operand` without re-entering pow.
- **Array elements pass BYREF to SUBs/FUNCTIONs** (`emit_call_args`,
  `src/emitter.rs`): `CALL Swap(a(i), a(j))` now hoists each element to a temp
  (indices evaluated once) and writes back after the call — both `__gs.` shared
  and local arrays, numeric and string. Previously mutations were silently lost.
- **`NEXT i, j` multi-counter** (`parse_for` + `pending_nexts` field): each
  extra comma-separated name closes one enclosing FOR; `parse_block_until`
  unwinds while `pending_nexts > 0`.
- **DATA backslash escaping** (`emit_data_store`): `\` is escaped before `"` —
  `DATA "C:\temp"` previously emitted an invalid/corrupting Rust escape.
- **`EQV`/`IMP` operators** (lexer + parser + `qb_eqv`/`qb_imp` in runtime):
  precedence looser than XOR (EQV, then IMP loosest), bitwise on i64.
- **QBasic's real RNG** (`runtime/src/lib.rs`): 24-bit LCG
  `x = (x*16598013 + 12820163) AND &HFFFFFF`, `RND = x/2^24`, power-on seed
  `0x50000` → first RND is the authentic `.7055475`. (Was the MSVC rand() LCG.)
  Golden checksums for gorilla/donkey were regenerated (RND-dependent frames).
- **`RND(n)` argument semantics** (`rnd_arg`): `RND(0)` repeats the last value,
  `RND(neg)` reseeds deterministically, `RND(pos)` advances. The parser
  previously discarded the argument entirely.
- **`UBOUND(a$())`/`LBOUND(a$())`**: string arrays resolve to their `_s` name;
  sigil-carrying names are stripped before the `array_lower` lookup (LBOUND of
  `w$()` returned 0 instead of the declared lower bound).
- **Skipped statements now warn** (`Parser::skip_warn`): ON KEY/TIMER traps,
  KEY/TIMER ON/OFF/STOP, CLEAR, WIDTH print a stderr warning instead of
  vanishing silently (per the "never silently drop" rule).

### blackjack.bas full support (SCREEN 12 VGA casino game — build-all 42/42)
`blackjack.bas` is a QBasic 1.1 casino blackjack game in SCREEN 12 (640×480, 16
colors) with GET/PUT-free vector card rendering, deck shuffle, and a TIMER-based
deal animation. Three interlocking fixes were needed:

- **Zero-arg string FUNCTION call-site naming** (`src/emitter.rs`) — `FUNCTION
  GetKey$()` has its `$` stripped by the parser, so the AST stores the bare name
  `GetKey` (`Expr::Var(Scalar{name:"GetKey", ty:String})`) while `user_fns`
  holds `getkey_s` (from `rust_ident("GetKey$")`). Both the `lift_expr` zero-arg
  path and the `emit_expr_inner` bare-reference guard checked only
  `rust_ident(name)` (`getkey`) → missed the function → emitted a call to the
  nonexistent `getkey()`. Fixed: both paths now also check
  `rust_ident_typed(name, ty)` (`getkey_s`) and emit the call with the matching
  typed name.
- **SCREEN 12/11 character cell height** (`runtime/src/lib.rs` `screen()`) —
  `char_h` was 8 for mode 12 (fell through the `_ => 8` arm), but VGA 640×480
  uses an 8×16 text font (80×30 grid). Every `LOCATE row` landed at half its
  correct y-pixel, so all text rendered in the middle of the screen instead of
  the top/bottom status lines. Fixed: `char_h = match mode { 0|11|12 => 16, 9 =>
  14, _ => 8 }`. SCREEN 11 (640×480 mono) shares the 8×16 font.
- **Single-line IF stealing an enclosing block-IF's ELSE** (`src/parser.rs`
  `parse_if`) — the most serious bug, an infinite deal loop. In
  ```basic
  IF dy < py THEN
     py = py - 24
     IF py < dy THEN py = dy      ' single-line IF ends the THEN body
  ELSE                            ' belongs to the OUTER block IF
     py = py + 24
     ...
  END IF
  ```
  the trailing single-line `IF py < dy THEN py = dy` has its then-body parsed by
  `parse_stmt`, which **consumes the trailing newline**. The single-line IF's
  `else_body` check (`if self.peek() == &Token::Else`) then saw the *outer*
  block-IF's `ELSE` (now adjacent) and stole it with an empty body, collapsing
  the outer else-branch (`py = py + 24`) into the outer THEN. For a player deal
  (`dy=304 > py=224`), `dy < py` was false → `py` never advanced → the
  `DO WHILE py <> dy` slide loop in `AnimateDeal` spun forever on the very first
  card. Fix: capture `if_line = self.line()` at the top of `parse_if` and only
  attach a single-line ELSE when `self.line() == if_line` (i.e. the ELSE is on
  the *same physical line*; a newline-separated ELSE belongs to an enclosing
  block IF). Regression-tested by the extended `tests/programs/if_single.bas`.
- **Persistent blit buffer — minifb use-after-free segfault** (`runtime/src/lib.rs`)
  — `blackjack.bas` crashed with `EXC_BAD_ACCESS` in
  `drawInMTKView`/`replaceRegion` on the FarewellScreen (the `GetKey$` idle wait
  after quitting with money left). Root cause: minifb 0.27's macOS backend
  *stores the raw buffer pointer* from `update_with_buffer` (`MacMiniFB.m:509`
  `win->draw_parameters->buffer = buffer;` — no copy) and re-reads it from
  `drawInMTKView` on *every* later `update()` event-pump. `present()` (and the
  `quit()` wait loop) passed a **local** `out: Vec<u32>` that was freed on
  return, so minifb held a dangling pointer; the next idle `INKEY$` poll
  (`inkey` → `pump_events`/`update`) drove an MTKView redraw from freed memory.
  It only crashed once the freed page was actually unmapped/reused, which is why
  it surfaced during FarewellScreen's long idle wait (after `SndWin` churned the
  allocator) rather than mid-game. Fix: render into a persistent `present_buf:
  Vec<u32>` field on `Runtime` (resized once to `win_w*win_h`, overwritten each
  blit) so the pointer minifb retains stays valid for the window's whole
  lifetime. Headless runs are unaffected (`present()` early-returns before
  touching it). Belongs to the runtime, so it fixes this class of crash for
  *every* transpiled program that idles on `INKEY$` after a graphics blit, not
  just blackjack.

### Additional blackjack fixes (title-screen music + QB language gaps)
Three more fixes landed after the initial blackjack port:

- **`PLAY(n)` function form** (`src/parser.rs`, `src/emitter.rs`, `runtime/src/lib.rs`)
  — QB's `PLAY(n)` is both a statement (`PLAY "MBL2G"`) and a function
  (`IF PLAY(0) < 5 THEN …`). The function form returns the number of notes
  remaining in the background music buffer. Previously `Token::Play` in expression
  context hit `parse_primary`'s error arm. Fix: `parse_primary` handles
  `Token::Play` followed by `(…)` → `Expr::Call { name: "PLAY", … }`; both
  `emit_expr_inner` and `lift_expr` map `PLAY` → `__rt.play_count()`. The runtime
  tracks a `bg_playing: Arc<AtomicBool>` flag: set when background PLAY fires,
  cleared by the thread when it finishes. `play_count()` returns 10 while playing
  (≥5 → throttle), 0 when done (< 5 → queue more). Without this the title-screen
  music loop queued a new background thread every ~0.9 s → doubled/stacked audio.

- **`MID$(var$, pos[, len]) = val`** statement form (`src/parser.rs`,
  `src/emitter.rs`, `runtime/src/lib.rs`) — QB's MID$ has a function form
  `MID$(s, 1, 3)` (already handled) and a *statement* form `MID$(ini$, nch, 1) =
  k$` that replaces characters in-place without changing string length. The
  statement form was being parsed as a 3D array assignment `mid_s[ini$][nch][1] =
  k$`. Fix: new `Stmt::MidAssign { var, pos, len, val }` AST node; early detection
  in `parse_assign_or_call` when name is `MID` with `$` sigil; emitted as
  `qb_mid_assign(&mut var, pos, len_opt, &val)`; runtime `pub fn qb_mid_assign`
  replaces characters in-place, preserving string length.

- **TYPE body array fields** (`src/parser.rs`, `src/emitter.rs`) — `TYPE Foo /
  Bar(4) AS INTEGER / END TYPE` was silently dropping the `(4)` dimension; `Bar`
  became a scalar `f64`. The parser already stored upper bounds in
  `type_field_dims`, but four emission sites ignored it. Fixes: (1) `emit_game_state`
  typed-array path now adds one extra `Vec<>` wrapping for each array field, so
  `DIM boards(2) AS Grid` with `Grid.Cell(4)` → `boards__cell: Vec<Vec<f64>>`;
  (2) `emit_dim` shared and local array paths emit `vec![vec![…; field_upper+1];
  outer_size]`; (3) `emit_lvalue` `FieldIndex` branch has a new `LValue::Index`
  base arm that emits `arr__field[outer_idx][inner_idx]`; (4) parser fixes for
  `arr(i).Field(j)` in both assignment (`parse_assign_or_call`) and expression
  (`parse_primary`) contexts — the dot chain now checks for a trailing `(index)`
  before expecting `=`, wrapping the result in `LValue::FieldIndex`. Integration
  test: `tests/programs/type_array_field.bas` (scalar, shared, and outer-array
  forms, 30/30 pass).

### Emitter code quality passes + VGA DAC + behavioral env overrides + kingdom.bas (build-all 44/44)

Three chained post-processing passes are now applied at `emit()` return, in order:
```rust
Ok(strip_deref_parens(&remove_unnecessary_mut(&inline_single_use_tmps(&self.out))))
```

- **`inline_single_use_tmps`** — replaces `let __tmpN = expr; ... use(__tmpN)` (where
  `__tmpN` is used exactly once and `expr` is safe to inline) with the expression
  directly at the use site. Reduces clutter from `lift_expr`'s defensive hoisting.

- **`remove_unnecessary_mut`** — scans each function body for `let mut varname:` where
  `varname` is never mutated (no `varname =`, `varname +=`, `&mut varname`, or
  `for varname in` after the declaration to the function's closing brace). Removes
  `mut` from those declarations. Reduces compiler noise and clarifies intent.
  Excluded prefixes (always kept `mut`): `__gs`, `__rt`, `__fn_ret`, `__pc`,
  `__for_`, `__tmp_`, `__pu_`, `__file_`, `__put_`, `__fa`, `__handle`.
  **Non-idempotent** — do not run a second pass (would corrupt `qb_bool(*mouth)`).
  **Also**: `emit_locals` now tracks `locals_declared: HashSet<String>` so a later
  `emit_dim` for the same numeric scalar (e.g. a `DIM x` inside a GOSUB body) is
  skipped rather than emitting a duplicate declaration. String DIMs are always emitted.

- **`strip_deref_parens`** — rewrites `(*ident)` → `*ident` everywhere except when
  followed by `.` or `[` (which need the parens for field/index access). Implemented
  as a byte scanner that skips string literals. Reduces parenthesis noise around
  byref parameter dereferences throughout emitted code.
  **Caution**: `idx_sub(expr)` at emit sites avoids pre-wrapping index expressions
  that are already balanced parens, preventing double-wrapping `[((*x)) as usize]`.

- **Concrete defaults** — array and scalar `let mut` initializers changed from
  `Default::default()` to `0.0_f64` (numeric) or `String::new()` (string) everywhere
  in the emitter: `__fn_ret`, array element inits, `emit_dim` scalars, `emit_game_state`.

### VGA DAC hardware port I/O (`OUT`/`INP`) (build-all 44/44)

- **`OUT port, val`** (`src/lexer.rs`, `src/parser.rs`, `src/emitter.rs`,
  `runtime/src/lib.rs`) — `Token::Out` added; `Stmt::Out { port, val }` AST node;
  emitted as `__rt.qb_out(port, val)`. Context-sensitive: `OUT` only becomes a
  statement when NOT followed by `(` or `=` (avoids breaking `SUB SubArr(out())`
  parameter lists in pi.bas). Falls through to `parse_assign_or_call` for identifier use.
- **`INP(port)`** (`src/lexer.rs`, `src/parser.rs`, `src/emitter.rs`) — `Token::Inp`;
  `parse_primary` → `Expr::Call { name: "INP", args: [port] }`; emitted as
  `__rt.qb_in(port)`. Added to `lift_expr` hoist table to avoid double-borrow in
  `PRINT INP(...)`.
- **`qb_out` / `qb_in`** (`runtime/src/lib.rs`) — DAC state machine on `Runtime`:
  - Port `0x3C8` (write index): sets `dac_write_idx`, resets `dac_channel` to 0
  - Port `0x3C9` (R/G/B data): accumulates into `dac_pending_r/g/b`; on the third
    write commits via `dac6_to_8(c) = (c << 2) | (c >> 4)` (6-bit → 8-bit) and
    advances `dac_write_idx`. Mirrors the real VGA DAC hardware protocol.
  - Port `0x3C7` (read index): sets `dac_read_idx`, resets `dac_read_ch` to 0
  - `INP(0x3C9)`: returns R, G, or B of `palette_rgb[dac_read_idx]` as 6-bit (>>2)
    and advances `dac_read_ch` / `dac_read_idx`.
- **`basic-src/vgadac.bas`** — test program comparing `PALETTE` statement vs `OUT`
  port writes vs `INP` readback in SCREEN 13; confirms both paths produce identical
  palette entries.

### Behavioral pragma env-var overrides (runtime)

`apply_behavioral_env()` is called unconditionally in emitted `main()` **after** all
`__rt.set_*()` pragma calls, so env vars always win over compile-time pragmas:

```rust
__rt.set_pace(30.0);        // REM QBC PACE 30 — baked in at transpile time
__rt.apply_behavioral_env();// reads QBC_PACE (if set) and overwrites
```

| Env var | Overrides pragma | Effect |
|---------|-----------------|--------|
| `QBC_PACE=N` | `REM QBC PACE N` | Set pace (blits/sec; sleeps to pace draw) |
| `QBC_FPS=N` | `REM QBC FPS N` | Cap frame rate at N fps |
| `QBC_FULLSPEED=1` | `REM QBC FULLSPEED` | Disable frame throttle |
| `QBC_SLOWMO=N` | `REM QBC SLOWMO N` | Multiply SLEEP durations by N |
| `QBC_TITLE=text` | `REM QBC TITLE text` | Override window title (at creation) |
| `QBC_SCALE=N` | `REM QBC SCALE N` | Override window scale (at creation) |

`QBC_TITLE` and `QBC_SCALE` are resolved inside `new_configured()` before the window
opens, so they function as compile-time overrides at run-start. The window is always
opened exactly once; subsequent env-only changes to title/scale have no effect.

### kingdom.bas (GW-BASIC text game — build-all 44/44)

`kingdom.bas` is a GW-BASIC resource-management kingdom simulation. Four transpiler
bugs were uncovered, plus one source-level `.bas` fix:

- **GW-BASIC DATA raw capture** (`src/lexer.rs`) — `DATA 1ST,2ND,3RD` was parsed as
  `IntLit(1)+Ident("ST")` → parse error. The lexer now detects `DATA` in statement
  position and switches to raw-capture mode: it accumulates characters until colon/
  newline, splits on commas (respecting quotes), and emits each element as a `StrLit`.
  Each element is finalized by `finalize_data_elem(elem, quoted)` to strip/preserve
  quotes. Non-line-numbered programs: byte-identical.
- **Colon-before-ELSE in single-line IF** (`src/parser.rs`) — `IF … THEN 2330: ELSE`
  left the `:` unconsumed before `ELSE`, causing a parse error "unexpected token: Else".
  Fixed: in `parse_if`'s single-line branch, if `peek() == Colon && peek_next() ==
  Else` consume the colon before checking for ELSE attachment.
- **POKE/OUT arg lifting** (`src/emitter.rs`) — `POKE x, PEEK(x) OR &H60` double-
  borrowed `__rt` (`E0499`). Fixed: both `Stmt::Poke` and `Stmt::Out` now use
  `lift_expr` for their address and value arguments (not `emit_expr_inline`), so
  `PEEK` inside a `POKE` is hoisted to a `__tmp` first.
- **PRINT USING cross-GOSUB scalar promotion** (`src/emitter.rs`) — `K` and `S`
  variables only seen via `PRINT USING "…"; K, S` were not being detected as
  cross-GOSUB boundary scalars. `collect_scalar_names_stmt` was missing a
  `Stmt::PrintUsing { fmt, args }` arm. Fixed: both `fmt` and each `arg` are now
  scanned. This caused kingdom.bas to print `0 KNIGHTS / 0 SERVANTS` instead of
  the correct values.
- **Source fix: `KORS` → `K OR S` on line 200** (`basic-src/kingdom.bas`) — the
  GW-BASIC source had the expression `K OR S` with spaces collapsed to the identifier
  `KORS` (DOS editor artifact). Restored as `K OR S`; with QB precedence
  (`*` > relational > `OR`) the condition reads `(S <= 2*K) OR (S < T1)` — "not
  enough servants relative to knights or land." Player confirmed "tested good here."

### Multi-line `DEF FN` + `ON ERROR`/`RESUME` solidified (build-all 46/46)

- **Multi-line `DEF FN`** (`src/parser.rs` only) — the block form
  ```basic
  DEF FNName (args)
      ... statements ...
      FNName = result       ' assign to the function name
  END DEF                   ' EXIT DEF allowed for early return
  ```
  is now supported. **Key idea:** a multi-line `DEF FN` is structurally a `FUNCTION`,
  so `parse_def` converts it into a `FuncDef` and pushes it onto a new
  `Parser::pending_funcs` side-channel (merged into `Program::functions` at the end of
  `parse_program`). It then rides the entire existing, tested `emit_functions` path —
  locals, `let mut __fn_ret`, return type (numeric/string via the name sigil),
  assignment-to-name → `__fn_ret`, recursion redirect, and `user_fns` call-site
  resolution — with **no emitter or analyzer changes**. Supporting tweaks: `Token::Def`
  added to `is_block_end` (so `END DEF` closes the body) and to `parse_exit` (so
  `EXIT DEF` ≡ `EXIT FUNCTION`). The **single-line** form
  (`DEF FnName(x) = expr`, e.g. gorilla's `FnRan`, kitchen_sink's `FNSQ`/`FNDB`) is on
  an untouched branch and emits byte-identical inline-expression fns (golden checksums
  unchanged). `parse_def` also now recognizes FN-prefixed names of any sigil. **Known
  limitation** (inherited from the single-line form, not a regression): a body that
  reads a *main-module local* that wasn't a parameter or promoted to `GameState` won't
  see it — we emit the body as an isolated fn, whereas QB's true `DEF FN` shares module
  scope. Bodies using their parameters + shared/global state work. Demo +
  integration test: `tests/programs/deffn_multi.bas` (iterative Fibonacci) and
  `basic-src/deffn-multi.bas`.

- **`ON ERROR GOTO <named label>` / `RESUME NEXT` verified end-to-end** — the
  named-handler + `RESUME NEXT` path (handler extracted as a fn via
  `collect_gosub_targets`, dispatched by `emit_error_dispatch()` after each fallible
  statement, `ERR` → the runtime error code) now has a real regression test driven by
  the one error the runtime actually raises: **file-not-found (err 53) on `OPEN`**
  (`runtime/src/lib.rs`). `tests/programs/onerror.bas` opens a missing file, traps it,
  prints `ERR`, and `RESUME NEXT`s past the faulting `OPEN`. One runtime addition:
  `Runtime::err_code()` getter method (the field `err_code` is read directly at most
  emission sites, but the generic zero-arg call path emits `__rt.err_code()` with
  parens, so a method is needed too — field and method coexist legally in Rust).
  **Deliberately NOT implemented** (Tier-1 scope — no bundled program needs them and
  the failure paths don't fire on modern hardware):
  - Bare `RESUME` (retry) on the NAMED-handler path behaves as `RESUME NEXT` (the
    handler fn returns to the dispatch site after the faulting statement; a retry
    is not representable there). The numeric/state-machine path DOES retry — see
    the numeric-handler section below.
  - Trappable errors are the file-I/O family + out-of-DATA (see the expanded-trap
    section below); `SCREEN`/`PALETTE`/divide-by-zero/subscript-out-of-range do not
    raise (adding SCREEN errors would regress programs that rely on us accepting
    every mode; the numeric ones would need per-operation checks everywhere).
  - (`ERL` — previously listed here as unimplemented — now works: see the ERL
    section below.)

### `ON ERROR GOTO <numeric line>` in `__pc` state-machine programs + real RESUME

The numeric-handler gap is closed: in a line-numbered (`__pc` state-machine)
program, `ON ERROR GOTO 1000` now really jumps to line 1000's match arm, and
all three `RESUME` forms work with genuine QB semantics at numbered-line
granularity:

- **Dispatch** (`emit_error_dispatch`, `emitter/mod.rs`): when emitting inside
  the state machine (`sm_mode`) and the active handler label is numeric, the
  post-statement check records the fault site into two resume registers —
  `__err_pc = <current block pc>` (RESUME's retry target) and
  `__err_resume_pc = <fall-through successor pc>` (RESUME NEXT's target) —
  then `__pc = <handler>; continue '__sm;`. The registers are declared before
  the loop only when the program has a numeric `ON ERROR GOTO`
  (`has_numeric_on_error()` in `emitter/scan.rs`); `sm_err_vars` on the
  Emitter gates every consumer. Named handlers (extracted fns) and non-SM
  numeric handlers keep their previous behavior.
- **`RESUME`** = `__pc = __err_pc` (retry the faulting line — verified with a
  handler that creates the missing file, so the retried `OPEN` succeeds);
  **`RESUME NEXT`** = `__pc = __err_resume_pc`; **`RESUME <line>`** = direct
  `__pc = <line>` (newly parsed — `Stmt::Resume` gained a `label:
  Option<String>` field; `RESUME 0` is QB shorthand for bare `RESUME`).
  Granularity is the numbered LINE, not the statement: for a multi-statement
  faulting line, `RESUME` re-runs the whole line and `RESUME NEXT` skips its
  remainder (in line-numbered code they almost always coincide).
- **Parser bug fixed en passant**: `RESUME NEXT` matched `Token::Ident("NEXT")`
  but `NEXT` lexes as `Token::Next`, so it had ALWAYS parsed as bare `RESUME` —
  latent only because both forms were previously treated identically.
- **`stmt_has_numeric_goto` now counts numeric `ON ERROR GOTO`/`RESUME <n>`**
  so a line-numbered program whose only jumps are error-handling still gets the
  state machine (otherwise the handler line wouldn't be an arm at all).
- **`remove_unnecessary_mut` exclusion list** gained the `__err_` prefix — the
  register assignments are embedded mid-line in the dispatch `if`, which the
  pass's mutation detector doesn't see, so it was stripping the needed `mut`s.

Regression test `tests/programs/onerror_sm.bas`: two missing-file OPENs, two
numeric handlers, `ERR` capture, `RESUME NEXT` and `RESUME 90`, and
deliberately zero plain `GOTO`s (proving numeric `ON ERROR` alone activates
the state machine). Verified: 137 unit, 38/38 integration, 53/53 build-all,
10/10 goldens (checksums unchanged).

### Expanded trappable errors: file-I/O family + out-of-DATA

`OPEN` file-not-found (53) is no longer the only trappable error. The full
set now raised (matching QB 1.1 codes):

| Code | Error | Raised by |
|------|-------|-----------|
| 53 | File not found | `OPEN FOR INPUT` on a missing file (as before) |
| 62 | Input past end of file | `INPUT #` / `LINE INPUT #` reading at EOF |
| 54 | Bad file mode | read on a write-mode handle, write on a read-mode handle, `GET`/`PUT #` on a sequential handle |
| 52 | Bad file name or number | any `INPUT #`/`PRINT #`/`WRITE #`/`GET #`/`PUT #` on an unopened file number |
| 4 | Out of DATA | `READ` past the last `DATA` element |

- **Runtime** (`runtime/src/lib.rs`): new `pub fn raise_err(&mut self, code)`
  centralizes `err_code`+`error_pending`; `read_file_line`, `write_file`,
  `read_record`, `write_record` classify their previously-silent failure
  branches (untrapped programs still see the same silent-continue behavior —
  the pending flag is only consulted by emitted dispatch code, which doesn't
  exist without an active `ON ERROR`). Borrow note: the sequential fns record
  the code in a local and call `raise_err` after the `files.get_mut` match so
  the map borrow has ended.
- **Emitter** (`emitter/mod.rs`): `emit_error_dispatch()` (a no-op emission
  when no handler is active) is now appended after `INPUT #`, `LINE INPUT #`,
  `PRINT #`, `PRINT #, USING`, `WRITE #`, `GET #`, and `PUT #`. Dispatch is
  placed BEFORE the input-variable assignments (`INPUT #`'s parse/assign,
  `GET #`'s field deserialization) so a state-machine handler jump leaves the
  variables untouched, per QB; `LINE INPUT #` splits into a temp read +
  dispatch + assign, but only when a handler is active (untrapped output
  byte-identical). Out-of-DATA is a pre-check emitted before each `READ`
  element (`__DATA_PTR.load(..) >= __DATA.len()` → `raise_err(4.0)`), again
  only when a handler is active.
- **Known approximation** (pre-existing to the whole ON ERROR design):
  `on_error_label` is tracked linearly at emission time, so fallible
  statements inside SUBs (emitted before main) don't get dispatch from a
  main-body `ON ERROR`. `EOF()` on an unopened handle stays a silent `-1.0`
  (QB raises 52) — reporting EOF keeps `DO WHILE NOT EOF` loops terminating.

Regression test `tests/programs/trap_errors.bas`: one file round-trip, then
faults for 62, 54, 52, and 4 in sequence, each trapped by a numeric handler
and `RESUME NEXT`-ed, printing the captured `ERR` after each. Verified: 137
unit, 39/39 integration, 53/53 build-all, 10/10 goldens (checksums
unchanged).

### Sigiled vs sigil-less string DIM coexistence (`DIM t, t$`) — mario.bas (build-all 54/54)

`mario.bas` (SCREEN 13 platformer, "MEGA WORLD", 3 worlds loaded from
`WORLD<n>.TXT` with DATA fallbacks) failed to compile: its title screen
declares `DIM t, mf, blink, armed, t$` — numeric `t` and string `t$` are
DISTINCT QB variables sharing a base name in one DIM statement. Root cause
chain:

- **The lexer strips sigils** (`t$` → `IdentStr("t")`), so `DIM t$` and
  `DIM t AS STRING` produced identical `VarDecl { name: "t", ty: String }` —
  the distinction was unrecoverable downstream. But it matters: a sigiled
  `t$`'s every use carries the `$` (typed String at parse), while a
  sigil-less `AS STRING` name's bare uses parse as Single and need the
  `local_string_scalars` type-recovery. Feeding sigiled decls into that
  recovery set made every bare NUMERIC `t` use in the SUB emit as a string
  (`t = 0.0f64.to_string()`, `qb_mod(t, …)` type errors).
- **Fix 1 — `VarDecl.str_sigil: bool`** (`src/parser.rs`): records whether
  the String type came from the `$` sigil (set in `parse_var_decl` and
  `parse_param_list`; an `AS` clause clears it). The only two VarDecl
  construction sites both set it.
- **Fix 2 — collectors skip sigiled decls** (`emitter/scan.rs`):
  `collect_local_string_scalars` AND `collect_local_string_arrays` now
  require `!d.str_sigil` — same reasoning for arrays (`DIM a$(…)` vs bare
  `a(i)` = different arrays in QB).
- **Fix 3 — `emit_dim` sigiled-string declaration name** (`emitter/mod.rs`):
  the String scalar branch emitted `let mut t: String` (BARE name) — correct
  for sigil-less decls (bare uses emit `t`), but a sigiled `t$`'s uses emit
  the typed `t_s`, so the bare-named String decl just shadowed the numeric
  `t` local. Sigiled decls now declare `let mut t_s: String`, deduped
  against `locals_declared` like the numeric path (the no-dedup rule exists
  only for sigil-less decls, which must shadow the Single-typed binding
  collect_locals infers).

mario.bas now compiles, runs, and renders (title screen + sprite animation
verified headlessly). Regression test `tests/programs/sigil_coexist.bas`
(numeric/string same-base-name pairs in one DIM, SUB + main scopes).
Verified: 137 unit, 40/40 integration, **54/54 build-all** (mario now
included), 10/10 goldens (checksums unchanged). Related latent (not fixed,
noted): SUB **params** `SUB Foo(t, t$)` would hit the same conflation via
`str_params`' `rust_ident_typed` lookup — no program does this.

### `INP(&H60)` keyboard data port — real held-key scancode polling (mario, pin)

mario.bas loaded but SPACE didn't start the game: its `PollKeys` SUB (reused
from PIN.BAS) reads **raw XT set-1 make/break scancodes from port `&H60`**
every frame to maintain a held-key `kd()` array (make < 128 = key down;
make+128 = key up), and `qb_in` only modeled the VGA DAC ports — every poll
returned 0, so `kd()` never saw a key. Now modeled on `Runtime`
(`runtime/src/lib.rs`):

- **Windowed**: `feed_scancodes()` diffs the minifb window's held-key set
  (`win.get_keys()`) on every event pump (`pump_events` + `present`) and
  pushes make/break codes for the transitions into a bounded (64) queue.
  `minifb_key_to_scancode()` maps letters, digits, space/enter/esc/tab/
  backspace, arrows, shifts/ctrl/alt, F1–F10; arrows use their bare codes
  (72/75/77/80, no `0xE0` prefix — QB-era pollers index a 0..127 array).
- **`INP(&H60)`** (`qb_in`): pops the queue; an empty queue returns
  `sc_last` — the port *holds* the last scancode until a new event replaces
  it, exactly the real-hardware persistence mario's own source comment
  documents relying on.
- **Headless**: scripted `QBC_KEYS` have no window key state, so `inkey()`
  feeds the model a make+break pair as each scripted key is consumed — a
  scancode-polling game sees a one-frame press. This let the SPACE-start and
  in-game "MEGA WORLD 1-1" render be verified headlessly end-to-end.
- **Gotcha re-learned**: a root `cargo build --release` does NOT refresh the
  non-hashed `target/release/libqbasic_runtime.rlib` — manual `rustc` links
  against the stale runtime and new runtime features silently no-op. Use
  `cargo build --release -p qbasic_runtime` first (the test scripts already
  do).

Covered by `scancode_tests` (runtime, 3 tests: make/break/persistence, held-
state diffing, scripted-key mapping). Verified: 140 unit, 40/40 integration,
54/54 build-all, 10/10 goldens (checksums unchanged — no golden program
reads the port).

### Deferred end-of-line text wrap (QB pending-wrap) — mario "graphics ruined" bug

Playing mario, reaching a two-digit coin count permanently corrupted all
graphics (floating enemy sprites, stacked HUD rows, misaligned bricks for
the rest of the game). Root cause was NOT sprites/clipping (all blit paths
clip correctly — verified with a super-jump probe): it was the text cursor.
`DrawHUD` does `LOCATE 25, 34: PRINT "C:"; coinCt; " ";` — with `coinCt >=
10` that print ends EXACTLY at column 40 of row 25. Our `cursor_advance`
wrapped eagerly: printing the last column immediately moved the cursor to
row 26 and **scrolled the whole framebuffer up 8px**. In a dirty-rect game
nothing ever redraws the full screen, so every erase thereafter missed by
8px — permanent corruption, re-triggered on every HUD update.

Real QB **defers** the wrap: after printing the last column the cursor sits
one past it (pending), and the wrap + bottom-row scroll fire only when a
FURTHER character actually needs the cell; an intervening `LOCATE` clears
the pending state. (pin.bas's identical bottom-row `DrawScore` idiom is
documented proven under real QBASIC 1.1 + DOSBox-X, which pins the faithful
semantics.) Fix (`runtime/src/lib.rs`): `cursor_advance` just increments;
new `wrap_cursor_if_pending()` performs the deferred wrap and is called
before each character draw (`print_gfx` loop + `input_line` echo). An
explicit newline absorbs a pending wrap (a full-width `PRINT` without `;`
advances exactly one row, as in QB). Genuine overflow past the last cell
still wraps and scrolls.

Covered by `deferred_wrap_tests` (runtime, 2 tests: HUD idiom must not
scroll + repeat after re-LOCATE; genuine overflow must still scroll).
Verified: 142 unit, 40/40 integration, 54/54 build-all, 10/10 goldens
(checksums unchanged), plus an end-to-end SCREEN 13 HUD repro (QBC_TEXT_FB
screenshot: values overprint in place, sky intact). Note: the stale-rlib
trap struck AGAIN during verification (manual rustc against
`target/release/libqbasic_runtime.rlib` without `cargo build --release -p
qbasic_runtime` first shows pre-fix behavior).

### `ERL` — error line number (closes the last ON ERROR gap)

`ERL` returns the numeric line label nearest before the statement that
raised the most recent error (0 = no error yet, or unnumbered code — QB's
convention). The error is *raised* inside runtime fns that don't know source
lines, so the line is recorded by the **emitted dispatch**, which does:

- **Emitter** (`emitter/mod.rs`): new `last_line_label: u32` tracks the most
  recent numeric line label during emission — updated per-block in
  `emit_state_machine` (alongside `sm_cur_pc`) and in the `Stmt::Label` arm
  (so non-SM programs with numbered lines work too). All three
  `emit_error_dispatch` variants (SM-numeric jump, named-handler call,
  clear-only) now write `__rt.erl_line = {line}.0;` inside the
  pending-check, so ERL only updates when an error actually fired. Same
  linear-emission approximation as `on_error_label` (SUB bodies emitted
  before main don't see main's numbering).
- **Parser** (`parser.rs`): bare `ERL` → zero-arg `Expr::Call` (exact ERR
  pattern). **Emitter routing**: `emit_expr_inner` + `rust_fn_name` map it
  to `__rt.erl_line` (field); **runtime** adds the `erl_line: f64` field +
  `erl_line()` method (field/method coexistence, same reason as `err_code`).

Regression test `tests/programs/erl.bas`: ERL=0 before any error, two faults
on different numbered lines (30: OPEN missing file → 53; 50: INPUT # on
unopened → 52) each reporting its own line in the handler, values surviving
`RESUME NEXT`. Gotcha note: an early draft used `READ` for the second fault
— a program with `READ` but NO `DATA` statements cannot compile (references
the never-emitted `__DATA` statics); switched to the unopened-file fault.
Verified: 142 unit, 41/41 integration, 54/54 build-all, 10/10 goldens
(checksums unchanged).

### `CHAIN` (positional COMMON passing) + `SHELL` + `ENVIRON` — no longer stubs

All three previously emitted `// STUB` + `__rt.quit()` (ending the program —
actively wrong for SHELL, which must continue). Now:

- **`SHELL cmd$`** (`Runtime::qb_shell`) — runs `/bin/sh -c cmd` (`cmd /C` on
  Windows) synchronously with inherited stdio, matching QB's blocking SHELL;
  the command's output interleaves with the program's own PRINTs. Bare
  `SHELL` (interactive COMMAND.COM) emits a no-op comment.
- **`ENVIRON "NAME=value"`** (`Runtime::qb_environ`) — sets a process env var
  (previously quit the program!).
- **`CHAIN prog$`** (`Runtime::qb_chain`) — QB loads and runs another program
  passing COMMON variables **positionally** (matched by ORDER, not name). The
  native model: exec the transpiled binary of the named program. Pieces:
  - **Parser**: `parse_common` still lowers COMMON to shared DIMs but now ALSO
    records the ordered decls on a `Parser::common_decls` side-channel →
    `Program::common_decls` → `AnalyzedProgram` (order was previously lost).
  - **Emitter**: `common_list: Vec<(name, QbType)>` (scalars only —
    arrays/UserTypes are skipped with a transpile-time warning). The CHAIN
    call site serializes each as `ChainVal::N(__gs.x)` / `ChainVal::S(…)`
    via `emit_lvalue` (so shared/sigiled naming is handled by the existing
    machinery); the top of `emit_main` assigns `__gs.x =
    __rt.chain_in_num(i)` / `chain_in_str(i)` per position. Started
    directly, the getters return type defaults = a fresh variable's value,
    so standalone behavior is unchanged.
  - **Runtime**: `ChainVal` enum; values travel through a temp file
    (`N <f64>` / `S <escaped>` lines) whose path rides the
    `QBC_CHAIN_COMMON` env var; `load_chain_common()` (in `new_configured`)
    reads and deletes it. Target resolution: strip `.bas`/`.BAS`, try
    as-given + lowercased next to `current_exe()`, then the working dir;
    then `exec()` (unix — process replaced; non-unix falls back to
    spawn+wait+exit). Missing target → stderr warning + trappable err 53,
    silent-continue when untrapped (consistent with the error-model
    convention). Open QB file handles do NOT survive the exec (real QB keeps
    them open across CHAIN — documented divergence, nothing uses it).

Integration tests: `shell.bas` (SHELL echo interleaves between PRINTs), and
the pair `chain_child.bas`/`chain_main.bas` — child standalone prints type
defaults; main sets `score`/`msg$`, CHAINs, and the child (found next to the
current executable in `tests/tmp/`; it compiles first alphabetically)
prints the passed 42/"hi there". Verified: 142 unit, 44/44 integration,
54/54 build-all, 10/10 goldens (checksums unchanged).

**Also corrected while here (stale docs)**: `PAINT` with a `CHR$()` tiling
pattern was listed in three places as an unimplemented solid-fill stub — it
has been fully implemented for some time (emitter dispatches string fills to
`Runtime::paint_pattern`, `flood_fill_pattern` tiles 1-byte-per-row / bit 7
leftmost / fg on set bits, locked by `tests/programs/paint_pattern.bas`).
The simplification vs real QB: EGA planar modes' multi-byte-per-row plane
grouping is read as single-plane; no bundled program uses tiling at all.

### Dead-`GameState`-field elimination (postprocess pass 5)

`strip_dead_gamestate_fields` (`emitter/postprocess.rs`, appended as the
OUTERMOST pass in `emit()`'s chain) removes any `struct GameState` field
whose `__gs.<name>` never appears anywhere in the emitted program. A
zero-reference field is provably dead: every read AND write goes through
`__gs.`, and the struct derives `Default`, so nothing else can name a
field. Matching is word-boundary-safe (`x` doesn't match `__gs.x2`), the
struct block is located by its exact `struct GameState {` / column-0 `}`
lines, and a program with no GameState is a no-op. Deliberately NOT
removed: write-only fields (e.g. a shared array whose only reference is
its `__gs.arr = vec![…]` init) — those have a textual reference, and
eliminating them would mean deleting statements. Idempotent.

Real dead code found in the bundled set: gorilla.bas's `DIM SHARED
SunColor` (a genuine leftover in Microsoft's 1990 source — the sun is
drawn with the hardcoded `SUNATTR` instead) and three unused TYPE members
in qbricks (`ball__size`/`ball__score`/`ball__numbrickshit`); 24→23 and
46→43 GameState fields respectively. Covered by `dead_gamestate_tests`
(3 unit tests: keep/drop, prefix-aliasing, no-struct no-op). Verified: 145
unit, 44/44 integration byte-identical, 54/54 build-all, 10/10 goldens
(checksums unchanged — gorilla still golden despite the removed field).

### farkle.bas (SCREEN 13 dice game — sigil-less `DIM … AS STRING` in comparisons)

`farkle.bas` is a SCREEN 13 push-your-luck dice game. Two transpiler fixes plus one
source tweak were needed:

- **Promoted scalar keeps its authoritative declared type** (`src/emitter.rs`) — a
  sigil-less `DIM k AS STRING` used across GOSUB boundaries is promoted to a `GameState`
  field. The cross-boundary-scalar promotion trusted the *usage-inferred* type from
  `detect_cross_boundary_scalars` (which defaults to Single), so `shared_types["k"]`
  came out numeric even though the GameState field was `String`. Fixed: the promotion
  loop now prefers the symbol-table type (`prog.global_scope.symbols[…].ty`) over the
  inferred one (mirroring the array-promotion path). Without this, `is_str_expr_ctx`
  couldn't tell `k` was a string.
- **All three string-comparison emitters consult `is_str_expr_ctx`** (`src/emitter.rs`)
  — relational/equality codegen normalizes both sides to `&str` only when a side
  `is_str_expr` (literal/sigil). A sigil-less declared string (`k >= "1"`) slipped
  through → `String >= &str`, which doesn't compile. There are **three** comparison
  emission sites (`emit_cond_expr`, the `BinOp` arm of `lift_expr`, and the `BinOp` arm
  of `emit_expr_inner` — the last is the one used for operands of `AND`/`OR`); all three
  now treat a side as a string when `is_str_expr(x) || self.is_str_expr_ctx(x)`, so
  declared-string vars get `(x).as_str()`. General fix — any program comparing a
  sigil-less `AS STRING` variable to a literal benefits.
- **Source: dropped a stray `DIM wt AS SINGLE`** (`basic-src/farkle.bas`) buried inside
  an `IF frames > 8 THEN` block while `wt` is used across several GOSUB routines. QB
  scopes a SUB/module DIM procedure-wide, but the emitter placed the `let mut wt` at the
  inner block, so sibling branches/routines couldn't see it. Removing the misplaced DIM
  lets the implicit `wt` ride the normal cross-GOSUB promotion (block-scoped DIM hoisting
  is a known transpiler limitation).
- **QB1.1 DOS compatibility (`.bas`-only)** — for loading under real QBASIC 1.1: renamed
  the three underscore GOSUB labels `Sub_DrawDie1at60_80`/etc. → `DecoDie1`/`DecoDie3`/
  `DecoDie5` (underscores are illegal in QB1.1 identifiers, and the `Sub_` prefix made
  QBASIC lex `Sub` as the reserved SUB keyword → "Expected: label or line number"), and
  renamed the `val AS INTEGER` parameter (shadows the `VAL()` built-in) → `pips` in
  `DrawDieFace`/`DrawDieFaceHL`/`DrawPips`. Also dropped the background arg from every
  `COLOR fg, 0` → `COLOR fg`: SCREEN 13 (MCGA) accepts only the single foreground form,
  and `COLOR fg, bg` raises a runtime "Illegal function call" there (bg was always 0 =
  the mode default, so appearance is unchanged). qbc accepts all these forms; the edits
  are purely for DOS-QBASIC fidelity.

  A subsequent **full QB1.1 audit** found and fixed the rest:
  - **`DIM` hoisted out of GOSUB routines** — QB1.1 GOSUB routines share the module
    variable scope, so a `DIM` inside a routine GOSUB'd more than once re-executes and
    raises "Duplicate definition" (arrays always; scalars too). All 13 locals
    (`dx`/`dy`/`adx`/`frames`/`diceRemaining`/`cnt(6)`/`isStraight`/`pairs`/`sc(6)`/
    `selCount`/`allDiff`/`prs`/`flashC`, plus the earlier `wt`) were moved to the
    top-of-module `DIM` block so each runs exactly once.
  - **Two more two-arg `COLOR`s** hidden inline in `IF … THEN COLOR x, 0 ELSE COLOR y, 0`
    (the earlier `^COLOR` sweep missed them) → single-arg.
  - **`PRINT USING "#####0"` → `"######"`** — QBasic has no `0` digit-placeholder, so the
    trailing `0` was a literal and scores printed ×10 (`123` → `12300`). Runs in QB1.1
    either way; this corrects the display.
  - **CRLF line endings** — converted from LF for real-DOS fidelity (qbc's lexer handles
    both). Audit also confirmed clean: all `LOCATE` within the SCREEN 13 40×25 grid, all
    draw coords within 320×200, no other reserved-word identifiers, no `SOUND`/`PLAY`.

  **Player 2 is a computer AI** (not hot-seat). The turn loop branches on
  `currentPlayer`: human GOSUBs `SelectDicePhase`/`BankOrRoll`, the computer GOSUBs
  `AISelectDice`/`AIBankOrRoll`. `AIKeepDice` greedily keeps every scoring die (all 1s/5s,
  any triple, or a full straight / three-pairs); `AIBankOrRoll` pushes with ≥4 dice and
  banks once the turn is worth protecting (≥1000, or ≥600 with 3 dice, or ≥300 with ≤2),
  always banking a winning total. All AI routines reuse module-scope vars (no new DIMs)
  and stay QB1.1-clean. UI relabeled via the `msg` string ("COMPUTER" vs "PLAYER 1").

### `BLOAD` + `DIR$` (pin.bas optional graphical title screen)

`pin.bas` (two-level SCREEN 13 pinball) loads an optional title image:
`IF DIR$("TITLE.BIN") <> "" THEN … DEF SEG = &HA000 : BLOAD "TITLE.BIN", 0 …`. Two
built-ins were added so it compiles and runs (text-title fallback when the file is
absent, real image blit when present):

- **`DIR$(spec)`** (`qb_dir` free fn, `runtime/src/lib.rs`) — returns `spec` if the path
  exists, else `""` (the file-exists guard programs use). Wired via `rust_fn_name`
  (`DIR$`→`qb_dir`) + `str_arg_positions` (`qb_dir`→`&[0]`); a `$`-suffixed call is
  already string-typed by `is_str_expr`, so `DIR$(…) <> ""` routes through the string
  path.
- **`BLOAD file$[, offset]`** (`Runtime::qb_bload`, dispatched from the `Stmt::Call`
  built-in chain alongside `sleep`/`chain`/`draw`) — `std::fs::read`s the file, skips the
  7-byte BSAVE header when the `0xFD` magic is present, and copies the bytes straight into
  the palette-indexed framebuffer at `offset` (one byte/pixel = the MCGA layout), then
  `present()`s. Missing file = silent no-op (guarded by `DIR$`); off-screen bytes clip.
  At the time, `DEF SEG` was a no-op and `BSAVE` unmodeled — both since superseded (see
  the "DEF SEG + segment-aware POKE/PEEK + BSAVE + WAIT vsync" section below). Covered
  by `bload_tests` (runtime). build-all was **51/51** (pin.bas was failing to compile
  before this).

### `WAIT` statement + emitter module split (build-all 52/52)

- **`WAIT port, mask[, xormask]`** (`src/lexer.rs`, `src/parser.rs`,
  `src/emitter/mod.rs`, `runtime/src/lib.rs`) — `Token::Wait` →
  `Stmt::Wait { port, mask, xormask }` → `__rt.qb_wait(port, mask, xormask)`. On
  real hardware `WAIT &H3DA, 8` blocks until `(INP(port) XOR xormask) AND mask !=
  0` — the VGA vertical-retrace status bit. Initially `qb_wait` just
  `pump_events()`-ed and returned; it has since been upgraded to a real wall-clock
  retrace model (see the "DEF SEG + …" section below). Without the
  keyword, `WAIT` lexed as a user-SUB call and emitted as an undefined
  `wait(port, mask)` — 11 such calls broke `demo.bas`'s compile. Parser mirrors
  the `OUT`/`POKE` pattern (statement only; the optional third arg is the XOR
  mask). New program: `basic-src/demo.bas`. (`blackjack.bas` was also renamed to
  the 8.3-DOS-safe `blackjak.bas`.)

- **`src/emitter.rs` (6,812 lines) split into `src/emitter/`** — the largest
  source file by far. The ~2,200 lines of free functions at the bottom (no `self`
  access — they take `&[Stmt]` / `&AnalyzedProgram`) were extracted into three
  submodules, leaving `mod.rs` with the `Emitter` struct and all `impl` methods:
  - `mod.rs` (~4,680) — `Emitter` struct + every `impl` method + the public
    `emit()` shim
  - `helpers.rs` (~250) — name-mangling / codegen utilities (`rust_ident`,
    `rust_ident_typed`, `rust_fn_name`, `qb_type_to_rust`, `emit_f64_lit`,
    `idx_sub`, `is_str_expr`, `nested_vec_*`, …)
  - `scan.rs` (~1,620) — all AST `collect_*`/`detect_*`/`extract_*` analysis
    passes
  - `postprocess.rs` (~270) — `inline_single_use_tmps`, `remove_unnecessary_mut`,
    `strip_deref_parens` + their tests

  All call sites are unchanged: each submodule's functions are `pub(super)` and
  re-exported into `mod.rs` via `use helpers::*; use scan::*; use postprocess::*`.
  **No `impl Emitter` method bodies moved** — that split is deferred (it would
  require marking the struct's ~34 private fields `pub(crate)`). **Note for future
  edits:** the many `(src/emitter.rs)` path references elsewhere in this file now
  resolve to `src/emitter/mod.rs` (impl methods) or the relevant submodule
  (`scan.rs` for collection passes, `helpers.rs` for name-mangling,
  `postprocess.rs` for output cleanup).

### Idiomatic-output passes (T1–T5) — cosmetic, behavior-preserving

Five transforms make the emitted Rust read more like hand-written Rust. All are
**behavior-preserving** (proven by byte-identical integration stdout + unchanged
graphics goldens); they never touch the f64-everywhere invariant or QB semantics,
and only the plain-numeric / string-concat paths are affected.

- **T1 — compound assignment** (`Stmt::Let`, `src/emitter/mod.rs`): `x = x + e`
  → `x += e` (also `-= *= /=`). Numeric LHS only, and only when the LHS is the
  *left* operand of the BinOp (so `x = 1 + x` is left alone). Catches both FOR-
  body and plain `while`-body manual increments.
- **T2 — string-literal arg unwrap** (PRINT builder): `qb_str(&("Fizz"))` →
  `qb_str("Fizz")`. `qb_str` takes `impl Display`, so a bare `&str` literal needs
  no `&(...)`.
- **T3 — constant-step FOR** (`Stmt::For`): when STEP is a compile-time numeric
  literal (or the default `+1`), emit a single-direction `while {v} <= {to}` (or
  `>=`) and inline the constant increment instead of the dual-direction guard
  `(step>0 && …) || (step<0 && …)`. STEP 0 and non-literal steps keep the runtime-
  checked form. `lit_sign()` helper detects the sign (IntLit/FloatLit/Neg).
- **T4 — `simplify_parens`** (`src/emitter/postprocess.rs`, runs FIRST in the
  postprocess chain): drops precedence-neutral parens — `(atom)`/`("lit")`
  *grouping* parens (NOT call/index parens, guarded by the preceding char) and a
  fully-parenthesized assignment RHS `= (E);` → `= E;`. String-literal-aware byte
  scanner; leaves `(*x)` to `strip_deref_parens`. 7 unit tests in
  `deref_paren_tests`.
- **T5 — string accumulation → `push_str`** (`Stmt::Let`): `s$ = s$ + a + b` →
  `s.push_str(&a); s.push_str(&b);` instead of the full-rebuild
  `(format!("{}{}…", s, a, b)).to_string()`. `concat_append_terms()` flattens a
  left-nested Add chain and requires the leftmost leaf to equal the LHS lvalue
  (so `G$ = M$ + K$` and `End$ = WhoWon$ + …` stay `format!`); `expr_refs_lvalue()`
  guards against any appended term referencing the LHS (sequential `push_str`
  would otherwise see a half-built string, unlike QB's evaluate-then-assign). Each
  String term coerces to `&str` via `&`; literals append bare. Works for locals,
  shared `__gs` fields, `&mut String` params, and string array elements. Covered
  by `tests/programs/string_accum.bas` (single, chain, different-leftmost-var, and
  self-referential-term cases). **Deferred:** `.is_empty()` for `(s).as_str() ==
  ""` (touches the regression-prone string-comparison emitter).

### Idiomatic-output audit round 2 — A1–A5 + T6, COMPLETE (behavior-preserving)

A full audit of 12 representative emitted programs (~10,900 lines) found ~1,250
remaining non-idiomatic sites. All six fixes landed across four commits; the
audit backlog is now empty:

- **A1 — constant array indexes** (901 sites): `arr[(1.0f64) as usize]` →
  `arr[1]`. One central change: `idx_sub()` (`emitter/helpers.rs`) collapses an
  integral non-negative numeric-literal index (`const_usize_lit`, digits-only
  parse) to a plain integer subscript. Negatives/fractions/expressions keep the
  cast form. Value-identical by construction (`as usize` on integral f64);
  goldens unchanged. Unit test `idx_sub_collapses_const_literal_indexes`.
- **A3 — `String::new()` for empty-string values** (31 sites): `x$ = ""` emitted
  `("").to_string()`. Lives on THREE emit paths, all fixed: the `Stmt::Let`
  string arms (shared `srhs` computation), the byref string-temp materializer
  (`let mut __tmp_strN = …` in `emit_call_args`), and `LSET`/`RSET` — which as a
  bonus were wrapping *every* string literal in `&("…").to_string()` when
  `qb_lset`/`qb_rset` take `&str`; literals now pass straight through.
- **A2 — redundant `.to_string()` on already-owned Strings** (~67 sites, also a
  real perf win — each was a needless heap clone): `(format!("{}{}",a,b)).to_string()`
  and `(qb_left(&s,5.0)).to_string()` etc. New `expr_returns_owned_string()`
  (`emitter/helpers.rs`) is true for string-concat (routes to `format!`) and
  calls to known owned-String-returning QB builtins (`LEFT$`/`RIGHT$`/`MID$`/
  `UCASE$`/`CHR$`/`MKD`/`INPUT$`/…, cross-checked against runtime signatures).
  Deliberately excludes bare variable reads (emitting one directly would MOVE
  it, breaking later reads) and array-element access (QB sometimes parses
  `help$(i)` as `Expr::Call`, and unlike a builtin call it reads a
  `Vec<String>` element that genuinely needs cloning). Applied at 9 emit
  sites: `Stmt::Let`, `LSET`/`RSET`, both `PrintUsing`/`PrintFileUsing`
  arg-lift blocks, `PrintFile` arg building, `emit_case_cond`, and two
  call-argument string-temp materializers (one required threading a 5th tuple
  field through `arg_info`). Unit tests `owned_string_string_concat_and_builtins`,
  `owned_string_excludes_vars_and_array_element_calls`.
- **A4 — inline constant FOR bounds** (148 sites): QB evaluates TO once, so the
  `__for_to_{v}` temp exists to freeze a non-literal bound before the loop
  starts. When TO is a compile-time numeric literal, that "once" is free — no
  temp needed, the condition reads the literal text directly:
  `let __for_to_i: f64 = 10.0f64; while i <= __for_to_i` → `while i <= 10.0f64`.
  New `is_const_numeric_lit()` (`emitter/helpers.rs`) recognizes `IntLit`/
  `FloatLit`/unary-`-`-of-one; deliberately broader than T3's `lit_sign()` at
  zero (a `FOR i = 1 TO 0` bound is unremarkable — `lit_sign`'s `None`-at-zero
  is specifically about STEP direction, which *is* undefined there). Applies
  independently of whether STEP is itself constant — a non-literal STEP with a
  literal TO still inlines the TO side of T3's dual-direction guard. A
  non-literal TO (could change during the loop) is untouched. Unit test
  `is_const_numeric_lit_classifies_literals`.

- **T6 — `(s).as_str() == ""` → `s.is_empty()`** (39 sites: 23 `==`, 16 `!=`;
  `<> ""` → `!s.is_empty()`). The twice-deferred one. New shared helper
  `empty_string_cmp_subject()` fires only for `Eq`/`Ne` where one side is an
  *empty* string literal and the other is a string-typed non-literal (both
  operand orders; `"" = ""` stays on the normal path; ordering ops untouched).
  Wired into all three string-comparison emitters: `emit_cond_expr` emits the
  bare `subj.is_empty()` form for `if`/`while`, and the BinOp arms of
  `lift_expr`/`emit_expr_inner` wrap in `qb_from_bool(…)` to keep QB's
  −1.0/0.0 boolean. No parens around the subject: every string-typed emission
  is self-delimited and method postfix binds tighter than `!`. Locked by
  `tests/programs/is_empty.bas` (value/cond paths, reversed operands, DO
  guard, builtin-call subject, both-literal case, and the farkle-style
  sigil-less `DIM SHARED … AS STRING` ctx path — farkle.bas was the canary:
  its `__gs.k.is_empty()` line is exactly the historically regression-prone
  path).
- **A5 — `qb_str(&(v))` → `qb_str(&v)`** (87 sites, cosmetic): one line in
  `print_scalar_part` (the single site producing every such wrap).

Verified after each commit: 137 unit, 35/35 integration byte-identical (incl.
the new `is_empty` test), 53/53 build-all, 10/10 goldens (incl. gorilla+donkey
— checksums unchanged proves every rewrite above is value-identical, not just
plausible; gorilla needed the usual idle-system retry per its documented
wall-clock flake).

### DEF SEG + segment-aware POKE/PEEK + BSAVE + WAIT vsync (demoscene batch, demo.bas)

`demo.bas` grew into a multi-scene megademo (starfield, ROM-font titles, wireframe
cube, plasma, copper bars, tunnel, vector morph, starship, credits) written in the
classic demoscene style: draw by POKEing VGA memory directly, read the ROM font via
PEEK, sync to the vertical retrace with WAIT, and cache precomputed textures with
BSAVE. Four interlocking features made it compile AND render correctly:

- **`DEF SEG [= expr]` is a real statement** — `Stmt::DefSeg(Option<Expr>)`
  (`src/parser.rs` `parse_def`; bare `DEF SEG` = restore default segment 0) →
  `__rt.set_def_seg(v)` with a `def_seg: u32` register on `Runtime`. Previously the
  whole line was skipped, which also silently hid any expression on it (see VARSEG
  below).
- **Segment-aware `POKE`/`PEEK`** (`runtime/src/lib.rs` `qb_poke`/`qb_peek`):
  - `DEF SEG = &HA000` + SCREEN 13 → the address is a linear framebuffer offset
    (y*320+x, one byte per pixel = MCGA layout): POKE plots a pixel (with
    `auto_present()`), PEEK reads one back. This is the demoscene draw-via-POKE
    idiom — demo.bas's starfield, plasma, copper bars, and tunnel drew *nothing*
    before this.
  - `DEF SEG = &HF000` → PEEKs in `FA6E..FA6E+2048` serve the ROM BIOS 8×8 font
    from our `FONT_8X8` table (the classic "PEEK the ROM font to draw scaled
    bigtext" trick — demo.bas's `DrawText` letters were invisible before).
  - Any other segment (including the default 0) → the simulated
    `HashMap<u32, u8>` memory map, exactly as before (pokeit/evil unaffected).
- **`BSAVE file$, offset, length`** (`Runtime::qb_bsave`, dispatched in the
  `Stmt::Call` built-in chain next to `bload`) — the exact mirror of `BLOAD`:
  writes the 7-byte header (magic `0xFD`, segment, offset, length, all LE) + the
  framebuffer slice. Only `DEF SEG = &HA000` is modeled; other segments are a
  silent no-op. demo.bas uses the BLOAD/BSAVE pair to cache its 64,000-byte
  plasma/tunnel patterns (`PLASMA.DAT`/`TUNNEL.DAT`) across runs — verified
  byte-correct (`FD 00 A0 00 00 00 FA` + 64000 pixels).
- **`WAIT &H3DA, 8[, xormask]` really paces to vsync** (`Runtime::qb_wait`) — the
  retrace bit is modeled from the wall clock: asserted the last ~2 ms of every
  `frame_interval_ms` period (default 16 ms ≈ 60 Hz). `WAIT &H3DA, 8` blocks until
  retrace starts **and then `present()`s the frame** (the draw-then-flip-at-vsync
  boundary); the double-wait prefix `WAIT &H3DA, 8, 8` blocks until the previous
  retrace ends, so the classic pair syncs to exactly one frame edge. Guards:
  headless runs return immediately (goldens unaffected), unmodeled ports/masks
  keep the old pump-and-return (never hang on an unemulated bit), and a 2-frame
  safety deadline bounds the worst case.
- **`VARSEG`/`VARPTR` stub → `0.0`** (`lift_expr` Call arm) — regression caught by
  build-all: money.bas's `DEF SEG = VARSEG(ScrollUpAsm(1))` (a CALL ABSOLUTE
  machine-code trick we don't model) was previously hidden inside the skipped DEF
  SEG line. Segment 0 routes its POKEs to the memory map — same net behavior as
  the old skip.

Verified headlessly frame-by-frame (QBC_KEYS scene advance + QBC_DUMP): POKE
starfield, rainbow ROM-font title, wireframe cube, full-screen plasma + cache.
(Windowed vsync pacing is the one thing headless can't prove — verify by eye.)

### Vsync-paced frame composition (`vsync_paced`), SYSTEM, PRINT #n USING, real EOF, bench.bas (build-all 53/53)

Testing `demo.bas` on real hardware surfaced a rendering bug in the wavy
sine-scroller scene: letters appeared to clip incorrectly against the pillar
graphics. Root cause was a **painter's-algorithm assumption our runtime broke**:
QB composes a frame (erase → move → PUT sprites → redraw overlays) entirely
between vertical retraces, so intermediate states are never visible on real
hardware — but our runtime was presenting mid-composition, because
`put_sprite` always blits immediately (§9) and `auto_present`'s timer can also
fire between statements. The scroller's letters showed up sitting on top of
the pillars *before* the pillars got redrawn over them, and the erase LINEs
briefly punched black holes in the pillar bodies.

- **`vsync_paced: bool` on `Runtime`** — set the moment a program completes a
  `WAIT &H3DA, 8` (retrace-start) wait; cleared by every `SCREEN` call (so a
  later non-WAIT scene presents normally again). While set, `put_sprite`'s
  unconditional `present()` and `auto_present`'s timer-driven blit are both
  suppressed — the WAIT's own `present()` (already implemented — see the DEF
  SEG/WAIT section above) becomes the *only* flip point, exactly mirroring
  real VGA's draw-during-blank/flip-at-retrace behavior. Verified headlessly
  (captured mid-scene frames show the letters composing correctly behind the
  pillars) and confirmed the full test suite (gorilla/donkey never call WAIT,
  so their behavior and golden checksums are provably unaffected).

While fixing `bench.bas` (the interpreter-vs-native benchmark the demo's
effect budget is tuned against) so it would even compile and produce correct
output, three more gaps surfaced — all were latent for every program, not just
this one:

- **`SYSTEM` was unmodeled** — fell through to the generic unknown-SUB path
  and emitted an undefined `system()` call. New `Runtime::qb_system()`: exits
  immediately (headless dump/checksum first), critically **without**
  `wait_for_key()` — unlike `END`/`quit()`, `SYSTEM` never holds the window
  open, since programs that use it (bench.bas included) do their own "press
  any key" prompt first.
- **`PRINT #n, USING fmt$; args` was never parsed** — the file-print path went
  straight to the plain-arg parser, so `USING` was read as an empty numeric
  variable reference and the format string printed literally instead of
  formatting the values. New `Stmt::PrintFileUsing` (parser: shared
  `parse_using_tail()` helper used by both `PRINT USING` and `PRINT #n, USING`;
  emitter: identical `qb_print_using` value-building as `PrintUsing`, written
  to the file instead of stdout). All four AST-collection scan passes
  (`collect_scalar_names_stmt`, cross-boundary array/scalar detection, etc.)
  got matching arms — cross-GOSUB scalars referenced only inside a `PRINT #n,
  USING` now promote correctly, same as the existing `PrintUsing` fix from the
  kingdom.bas milestone.
- **`EOF(n)` never worked, in any program** — both ends were stubs:
  `Runtime::eof_check` hardcoded `true` (dead code, unused), and the free
  `qb_eof_fn` hardcoded `0.0` ("never EOF"). Worse, a bare `WHILE NOT EOF(1)`
  condition routes through `emit_cond_expr` → `emit_expr_inner`, which had
  **no** `__rt` routing for `EOF`/`PEEK`/`INP` at all — those three only
  worked inside `lift_expr`'s hoisting path, so a bare condition emitted an
  undefined free-function call. Fixed: real `Runtime::qb_eof` peeks the
  sequential-file `BufReader` via `fill_buf` (true EOF, QB boolean −1.0/0.0,
  no bytes consumed) and routes through **both** expression-emission paths;
  `PEEK`/`INP` got the same second routing, closing the identical latent gap
  for them in bare conditions. No bundled program had ever exercised
  `EOF` before this, so nothing had caught it. New integration test
  `tests/programs/print_file_using.bas` covers both the USING-format fix and
  an `EOF`-driven read-back loop.

`basic-src/bench.bas` measures the demo's hot-loop operations (empty
FOR/NEXT, integer math, array access, POKE/PEEK, PSET, LINE, GET/PUT, empty
SUB call) as ops/sec + ops-per-60fps-frame-budget, run once under real
QBasic 1.1 interpreted in DOSBox-X (Pentium 66 MHz, same cycle count as the
demo) and once transpiled to native Rust. Measured speedup ranges **~30×**
(LINE, bandwidth-bound — both platforms spend real time touching pixels) to
**~275×** (integer multiply/divide, dispatch-bound — pure interpreter
bytecode-fetch overhead disappears in native code). Full comparison table in
`README.md`'s Performance section. This is the empirical justification for
`PEEK+POKE RMW`'s "killed shadebobs v1" comment in `bench.bas` — 73K ops/sec
on real hardware vs. 8M ops/sec natively means the per-pixel read-modify-write
budget that broke shadebobs on period hardware is a non-issue now.

### demo.bas grows to 15 scenes: shadebobs revived, rotozoomer, platformer vignette

`bench.bas`'s numbers (above) directly unblocked **Scene5 — shadebobs**, which
had been commented out since the demo's first cut ("too slow in interpreter;
revisit with CALL ABSOLUTE"). True additive blending with zero per-pixel
BASIC work: each of 4 bobs owns 2 bits of a pixel byte; draw = `PUT … OR`
(sets only its own bits), erase = `PUT … AND` with a constant mask (clears
only its own bits, even under overlaps), and the palette does the actual
adding — entry *b* renders the sum of the four 2-bit levels through a fire
ramp, so crossings glow genuinely hotter. 8 machine-speed `PUT`s per frame;
native Rust has no trouble at all with the per-pixel work that killed it on
period hardware.

Two new scenes, both reusing existing transpiler/runtime features (no new
`qbc`/runtime work required):
- **Scene14 — rotozoomer**: a source-texture buffer rotated and scaled per
  frame via `SIN`/`COS` and read back with `POINT`, the classic demoscene
  effect.
- **Scene15 — platformer vignette** ("MEGA WORLD 1-1", a Mario homage): sprite
  `DATA` for a 3-frame runner + a goomba, `GET`/`PUT` animation, brick-block
  platform tiles, and a scrolling ground row.

Also added: a 3-tone ascending `PLAY "O4 L4 C E G"` intro jingle before
Scene1. Scene ordering updated to interleave the new scenes among the
existing ones (`basic-src/demo.bas`'s header comment documents the current
order — Scene8, the credits crawl, remains hard-pinned last).

Verified: full `demo.bas` and an isolated `CALL Scene5: CALL Scene14: CALL
Scene15` harness both transpile and compile clean; headless captures confirm
shadebobs animate (glowing, moving) and the platformer scene renders its
title/platforms/character with no crashes. build-all 53/53.

### Fixed: local sigil-less `DIM x AS STRING` scalar assignment gap

Closed the previously-latent TODO item: a purely-local, sigil-less
`DIM k AS STRING` scalar (no `$`, never promoted to shared state) declared
correctly (`let mut k: String`) but its assignment emitted the bare
`k = "…";` instead of `k = ("…").to_string();` — a rustc E0308.
`is_str_expr_ctx()` — the single oracle every string-typing decision funnels
through — knew about `str_params`/`shared_types`/local string ARRAYS
(`local_string_arrays`) but had no equivalent for local string SCALARS.

- New `local_string_scalars: HashSet<String>` field + `collect_local_string_scalars()`
  (`emitter/scan.rs`, mirrors `collect_local_string_arrays` exactly — same
  recursive walk, differing only in `d.dims.is_empty()` vs `!d.dims.is_empty()`),
  consulted in `is_str_expr_ctx` right after the `str_params` check. Reset at
  all three scope boundaries: `emit_subs`, `emit_functions`, and — previously
  missing entirely — `emit_main`. `Stmt::Let`'s assignment dispatch already
  fell through to `is_str_expr_ctx` as its last non-numeric arm, so fixing
  the oracle alone fixed emission; no direct dispatch-arm edit was needed.
- **A regression was caught and reverted during verification, not shipped**:
  `emit_main` was *also* going to reset `local_dim_names`/`local_string_arrays`
  (same "was never reset for main" gap, same fix shape) — this broke
  `farkle.bas` and `torus.bas` (both E0425, "cannot find value"). Root cause:
  `is_str_expr_ctx`'s `shared_types` lookup is guarded by
  `!self.local_dim_names.contains(&lc)`, meant to let a genuine local shadow
  an unrelated `DIM SHARED` of the same base name — but a cross-GOSUB-
  *promoted* scalar (farkle's `k`, torus's `Available$`) still has its
  original pre-promotion `DIM` statement sitting in the body text, so
  correctly resetting `local_dim_names` for main made that guard fire for the
  *promoted* variable too, skipping the `shared_types` lookup that identifies
  it as the shared/GameState field it actually is. `local_string_scalars` has
  no such guard anywhere (it's purely additive — only ever a positive "treat
  as string" signal), so it doesn't share this hazard; only its own reset
  landed. The `local_dim_names`/`local_string_arrays` main-body non-reset
  remains a real, separate, still-latent gap — see Known Issues / TODO.
- New regression test `tests/programs/local_string_scalar.bas`: a numeric
  local `n` in main reusing a bare name that's STRING-typed inside an
  earlier-processed SUB (proves the per-scope reset doesn't leak), plus a
  main-body string scalar exercising the assignment fix and T6's
  `is_empty()` path together. `tests/programs/is_empty.bas`'s `DIM SHARED`
  workaround was removed now that plain `DIM k AS STRING` works.

Verified: 137 unit, 36/36 integration byte-identical (incl. the new test),
53/53 build-all (incl. the farkle/torus regression catch-and-revert), 10/10
graphics goldens (torus's checksum confirms the revert restored correct
*behavior*, not just compilation; gorilla needed its documented idle-system
retry).

### Fixed: per-scope DIM bookkeeping reset in `emit_main` + `emit_gosub_fn`

Closed both scope-bookkeeping TODO items in one change: `emit_main` never
reset `local_dim_names`/`local_string_arrays` (only `local_string_scalars`,
from the earlier fix), and `emit_gosub_fn` reset none of the three — so both
scopes emitted against whatever the last-processed SUB/FUNCTION left behind.
The naive "just add the reset" was known-unsafe (previously broke
farkle.bas/torus.bas with E0425): a cross-GOSUB-*promoted* scalar still has
its original pre-promotion `DIM` statement in the body text, so a fresh
collection made `is_str_expr_ctx`'s local-shadows-shared guard
(`!self.local_dim_names.contains(&lc)`) fire for the promoted variable
itself, skipping the `shared_types` lookup that identifies it as a
GameState field.

- **`retain_unpromoted()`** (new helper, `emitter/mod.rs`) filters a
  collected per-scope DIM-name set against `shared_names` — which is fully
  populated with DIM SHARED + promoted arrays + promoted scalars before any
  emission starts, and save/restored around the per-sub scoping, so it's
  valid at both call sites. Checks both the lowercase-bare and `rust_ident`
  forms (shared_names holds a mix, e.g. keyword-prefixed `qb_true`).
- **`emit_main`** now resets all three sets (collected over the main body,
  filtered through `retain_unpromoted`); **`emit_gosub_fn`** does the same
  over each GOSUB-target body — anything DIM'd there that's also used in
  main was promoted (and thus excluded), leaving only genuine locals of the
  extracted fn. `emit_subs`/`emit_functions` are deliberately untouched:
  a sub-local DIM shadowing a DIM SHARED is the guard's intended positive
  case, and promotion never originates from SUB bodies.
- New regression test `tests/programs/scope_reset.bas`: a SUB's local
  `DIM msg AS INTEGER` (genuine shadow of shared sigil-less
  `DIM SHARED msg AS STRING` inside the SUB) followed by main-body and
  GOSUB-target assignments to the shared string. Pre-fix: the leaked
  `local_dim_names` entry made main's `msg = "…"` stop routing to
  `__gs.msg` → three E0425s. Confirmed failing on the pre-fix baseline and
  passing with the fix.

Verified: 137 unit, 37/37 integration, 53/53 build-all (farkle + torus — the
two programs the naive reset broke — compile and golden-pass), 10/10
graphics goldens (torus checksum unchanged proves the promoted-`Available$`
path still emits identically; gorilla/donkey passed without flaking).

## Known Issues / TODO

- **Idiomatic-output audit round 2 is COMPLETE** (A1–A5 + T6 all landed — see
  the changelog section above). Audit non-findings, recorded so nobody
  re-chases them: `.clone()` uses are all required byref-writeback temps;
  remaining `((` nestings are precedence-required (f64 non-associativity);
  zero `qb_bool(qb_from_bool(…))` round-trips.
- **`gorilla`/`donkey` golden tests are load-sensitive (intermittent flakes,
  pre-existing).** Bisected during the M21 session by stashing all M21 changes
  and re-testing the clean, already-pushed baseline commit: both flakes
  reproduce **identically** with or without M21's changes, so neither is a
  regression from any recent change.
  - `gorilla` fails its checksum non-deterministically (observed ~3-in-4 under
    heavy concurrent system load, 0-in-4 once the system settled). Likely
    cause: gorilla.bas's `Rest()` SUB paces via real wall-clock `TIMER` (not
    simulated frame count) — `CalcDelay!` returns the fixed `SPEEDCONST`, so
    it's not a speed-calibration issue, but under CPU contention the scripted
    `QBC_KEYS` stream and `Rest()`'s wall-clock waits can interleave
    differently, landing the `presents:80` capture on a different simulation
    frame (e.g. before vs. after the banana becomes visible).
  - `donkey` occasionally produces **no checksum at all** (times out its full
    30s `timeout` with zero output), reproduced on the clean baseline under
    heavy load. Root cause not yet isolated (donkey.bas has no TIMER/SLEEP
    calls, so it's a different mechanism than gorilla's — possibly headless
    event-pump/window-init contention).
  - A real fix for either would need the affected pacing/init path to key off
    a simulated/injectable clock in headless mode rather than wall time or
    real scheduling. Until then: if either flakes in CI or a local run, retry
    once on an idle system before assuming a real regression.
  - **Bonus finding while debugging this:** `tests/run-graphics-tests.sh` had
    a latent bug (now fixed) that made the "no checksum" case far worse than
    a mere per-test failure — `sum="$(... | grep -o ... | ...)"` had no
    `|| true` guard, so under `set -o pipefail` a `grep` no-match (exactly the
    "no checksum" case) aborted the **entire script** via `set -e`, silently
    skipping every remaining test and the final `Results:` summary instead of
    reporting one clean "no checksum" failure and continuing. This is what
    made the `donkey` flake invisible until the pipe was patched.
- **`SCREEN 13` (320×200, 256 colors) — SUPPORTED.** `palette_rgb` is a
  256-entry table; `screen(13)` loads the authentic VGA BIOS power-on default
  palette (`vga256_default()` — 16 EGA + 16 grays + 216-color HSV cube, matches
  DOSBox 0.74 / Allegro). Color indices are wrapped mod 256 in mode 13 / mod 16
  in EGA modes via `color_mod()`. `PALETTE`/`PALETTE USING` decode the 18-bit
  DAC value (`red + 256*green + 65536*blue`, each channel 0–63) via
  `dac18_to_rgb()` in mode 13, keeping the EGA `irgb` decode otherwise. Covered
  by `screen13_tests` (runtime) and `basic-src/screen13.bas` (visual).
  `SCREEN 12` (640×480, 16 colors) is also supported. `OUT &H3C8/&H3C9`
  VGA DAC port writes are now supported (see below).
- **`INKEY$` FULLSPEED slowness — FIXED.** (Was ~5 min on `mandel.bas`.) Root
  cause was minifb's built-in rate limiter (default 250 FPS / 4 ms), which sleeps
  inside *both* `update()` and `update_with_buffer()` — so an "events-only"
  path alone would not have helped. Fix: `set_target_fps(0)` at window creation
  to disable minifb's limiter (we do our own pacing), plus `inkey()` now blits at
  most once per `frame_interval_ms` and uses a cheap `pump_events()` (event poll
  + key harvest, no framebuffer rebuild) the rest of the time. Trade-off: a pure
  idle `DO … LOOP WHILE INKEY$ = ""` now busy-spins (DOS-faithful).
- **Two "QBC" control surfaces, intentionally separate.** `REM QBC …` source
  pragmas (`FULLSPEED/FPS/PACE/SLOWMO/TITLE/SCALE`) are compile-time, baked into
  the binary via `parse_qbc_config` (emitter). The `QBC_*` env vars are run-time
  (the headless driver, runtime). They share the "QBC" name but don't overlap.
  The behavioral pragmas ARE now env-overridable via `apply_behavioral_env()`:
  `QBC_PACE`, `QBC_FPS`, `QBC_FULLSPEED`, `QBC_SLOWMO` override compile-time
  pragma settings; `QBC_TITLE` and `QBC_SCALE` override via `new_configured()`
  pre-creation. Env vars always win. The debug knobs (`HEADLESS/KEYS/SEED/DUMP/
  CHECKSUM/FBSTATS/EXIT_AFTER`) remain run-time only — they are not pragmas.
- **`GET`/`PUT` sprite layouts — all depths supported:** EGA 4-plane planar
  (SCREEN 9/12), CGA 2bpp packed (SCREEN 1), MCGA 8bpp chunky (SCREEN 13). The
  mode-13 path (`get_sprite_mode13`/`put_sprite_mode13`, gated on
  `screen_mode == 13`) uses `data[0]=width*8`, `data[1]=height`, one full color
  byte per pixel (2/INTEGER). Covered by `mode13_sprite_tests` + `screen13-sprite.bas`.
- **`GET`/`PUT` honor a packed-array element OFFSET** (`Arr(n)` buffer). QB lets a
  program pack many sprites into ONE array at distinct element offsets
  (`GET …, BlockImage(((style-1)*4+rot)*ELEMENTSPERBLOCK)`), then blit each from its
  offset. The emitter's `sprite_arr_name` used to *drop* the index ("always the whole
  vec"), so every GET wrote to `arr[0]` (each overwriting the last; `get_sprite`'s
  `resize` even shrank the array) and every PUT read `arr[0]` — i.e. every sprite was
  the same wrong image. Fixed: runtime `get_sprite_at`/`put_sprite_at(…, offset: usize)`
  place the header at `data[offset]` (the old `get_sprite`/`put_sprite` are offset-0
  wrappers → all other callers byte-identical; `get_sprite_at` resize is **grow-only**
  for `offset > 0` so packed sprites don't clobber each other). The emitter's
  `sprite_offset_expr()` emits the `_at` variant with the element offset when the buffer
  is `Arr(n)` with a non-zero index (bare names / `Arr(0)` → unchanged plain call). This
  is what broke **qblocks.bas** — every falling piece was the same shape and spilled
  outside the well (gorilla/donkey use one array per sprite, offset 0, so were spared).
  Threaded through the cga/mode13 variants too. Covered by `get_put_at_offset_*` in
  `sprite_tests`.
- **`PRINT USING` is fully implemented**, including the `$$` floating dollar,
  `**` asterisk fill, and `**$` combined prefixes (`*` slots extend digit
  capacity, `$` does not), alongside numeric/string/`_X`/`^^^^`/`%` formats. 36
  unit tests (`print_using_tests` + `print_using_prefix_tests`). The only
  unmodeled QB features are hardware-level port I/O beyond the VGA DAC +
  keyboard ports (`PAINT CHR$()` tiling, `CHAIN` + positional COMMON, and
  `SHELL` are all supported — see the CHAIN/SHELL changelog section).
  `OUT`/`INP` are supported for VGA DAC ports — see the VGA DAC section above and
  `vgadac.bas`.
- **gorilla is now golden-tested** — seed 42, scripted intro + one banana throw
  (angle 45°, velocity 50), captures mid-flight frame (`presents:80`).
  The `DRAIN` sentinel stops two `WHILE INKEY$<>"":WEND` drain-loops (SparklePause
  + GetNum#). **donkey** is also golden-tested now.
  Audio (PLAY), victory animations, and multi-round scoring confirmed working
  via human play-through. The other graphics programs (256c/screen13/palette256_expanded
  /reversi/torus/hangman-gfx/duck) are also golden-tested.

---

## When You Are Unsure

- Read `docs/gorillas.md` for gorilla.bas specifics — it is the ground truth
- Read `docs/ARCHITECTURE.md` for the full feature/limitation inventory
- QB documentation: assume Microsoft QBasic 1.1 (DOS) behaviour
- For numeric edge cases, prefer matching QB output over mathematical purity
- Never silently drop an unimplemented statement — emit `// TODO: <stmt>` in
  the Rust output AND a warning to stderr during transpilation
- Run `bash tests/run-tests.sh` before declaring anything fixed
