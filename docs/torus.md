# TORUS.BAS — How It Works

An architectural walkthrough of `basic-src/torus.bas`, the interactive 3D-torus
demo that shipped with MS-DOS QBasic 1.1 (`\DOS\TORUS.BAS`). Unlike GORILLA or
DONKEY this program is **all structured `SUB`/`FUNCTION` code** — no GOTO state
machine — and it leans hard on QBasic's logical-coordinate graphics
(`WINDOW`/`PMAP`), arrays of a user-defined `TYPE`, and the VGA palette. It was
the program that forced **seven** transpiler fixes (three in the parser/emitter
to make it *compile*, four in the runtime to make it *render*).

This doc doubles as the record of *why torus.bas was hard*, in §10–§11.

---

## 1. What the program does

You get a text setup screen with seven tunable fields:

```
Thickness                 [ 3 ]
Panels per Section        [ 8 ]
Sections per Torus        [ 14 ]
Tilt around Horizontal Axis [ 60 ]
Tilt around Vertical Axis   [ 165 ]
Tile Border               [ YES ]
Screen Mode               [ 12 ]
```

Arrow keys move between fields (UP/DOWN) and rotate a field's value
(LEFT/RIGHT). Press **ENTER** to build the torus, **ESC** to quit. After ENTER
the program computes the torus geometry, switches to a graphics mode (SCREEN 12
on a VGA), draws the doughnut as a mesh of shaded quadrilateral *tiles*, and then
**rotates the palette** to give the illusion of the torus spinning. Press any key
to return to the setup screen.

The torus is a true back-to-front painter's-algorithm 3D render: every tile is
sorted by Z distance and the far tiles are drawn first so near tiles overdraw
them.

---

## 2. Top-level execution flow

The module-level code is short; everything lives in SUBs/FUNCTIONs.

```
GetConfig                 probe the best graphics mode (VGA 12 → … → CGA 1),
                          via ON ERROR fall-through; reset to text mode
DO WHILE TRUE             (loop forever; exit is ESC inside TorusDefine)
   TorusDefine            the interactive setup screen (INKEY$ field editor)
   REDIM T(0..Max-1) AS Tile,  Index(0..Max-1)   (dynamic, Max = Panel*Sect)
   TorusCalc T()          fill every tile's 4 corner (x,y) + 1 z coordinate
   TorusColor T()         assign each tile an attribute (1..Atribs-2)
   TorusSort 0, Max-1     quicksort the Index() array by tile Z (painter order)
   SCREEN VC.Scrn         switch to the chosen graphics mode
   SetPalette             mix a palette of colors into Pal()
   WINDOW (...)           logical coordinate system, centered on 0
   TorusDraw T(), Index() draw every tile farthest-first
   DO WHILE INKEY$ = ""   spin: Delay + TorusRotate (palette cycle)
      …
   LOOP
   SCREEN 0               back to text; loop to the setup screen
LOOP
```

There are five `ON ERROR` traps (`VideoErr`, `EGAErr`, `MemErr`, `RowErr`) used
for hardware probing and out-of-memory recovery. On modern hardware every SCREEN
succeeds, so these never fire — the transpiler's `ON ERROR`/`RESUME` stubs are
sufficient.

---

## 3. The two user-defined TYPEs

```basic
TYPE Tile                 ' one quadrilateral tile of the mesh
   x1..x4 AS SINGLE       ' four corner X (logical/window coords)
   y1..y4 AS SINGLE       ' four corner Y
   z1     AS SINGLE       ' one Z (only z1 is used, for the sort)
   xc, yc AS SINGLE       ' a point known to be INSIDE the tile (set by Inside())
   TColor AS INTEGER      ' the tile's color attribute
END TYPE

TYPE Config               ' the detected video configuration
   Scrn, Colors, Atribs, XPix, YPix, TCOL, TROW AS INTEGER
END TYPE
```

`DIM T(0 TO Max-1) AS Tile` is an **array of a TYPE** — the single most demanding
feature in the program. The transpiler flattens it to one `Vec<f64>` per field
(`t__x1`, `t__y1`, …, `t__tcolor`), all parallel-indexed (CLAUDE.md design
decision #3 / #10). `T(i).x1` becomes `t__x1[i]`.

---

## 4. The geometry — TorusCalc

`TorusCalc` walks two nested angular loops (sections around the tube × panels
around the ring) and, for each tile, computes four 3D corner points rotated by
the two tilt angles, then projects them to 2D. The math is plain `SIN`/`COS`
with a rotation matrix; `DegToRad!` converts degrees to radians. The notable
transpiler detail is that `TorusCalc` is bracketed by `DEFSNG A-Z` … `DEFINT
A-Z`, which in QBasic changes default variable precision — but since the
transpiler stores **every** numeric as `f64`, these are no-ops for storage
(CLAUDE.md design decision #1).

Each tile shares corners with its neighbors, so the assignment pattern is
deliberately staggered (`T(XSect).x1`, `T(XSect-1).x2`, …) to avoid recomputing
shared seam points.

---

## 5. Drawing a tile — TileDraw + Inside

`TileDraw` is where the rendering subtleties live:

```basic
Border = VC.Atribs - 1               ' 15 in SCREEN 12
LINE (x1,y1)-(x2,y2), Border          ' draw the 4-sided outline in the border color
LINE -(x3,y3), Border : LINE -(x4,y4), Border : LINE -(x1,y1), Border
IF Inside(T) THEN                     ' is the tile big enough to fill?
   PRESET (T.xc, T.yc)                ' blank the interior point
   PAINT STEP(0,0), BACK,  Border      ' flood the interior to background, bounded by border
   PAINT STEP(0,0), T.TColor, Border   ' then flood to the tile color
END IF
IF TOR.Bord = "YES" THEN Border = BACK ELSE Border = T.TColor
LINE (x1,y1)-(x2,y2), Border          ' redraw the outline (black border, or hidden)
...
```

**`Inside(T)`** is a FUNCTION that decides whether a tile is large enough to be
worth filling. It computes a center point `T.xc`/`T.yc`, **writes them back into
the tile**, then scans pixel rows up and down from the center using `POINT` and
`PMAP`, looking for the border color on both sides. If it finds the tile's top
and bottom edges, the point is genuinely interior and the tile gets painted.

Two things make `Inside` a stress test:
1. It **mutates its TYPE parameter** (`T.xc`, `T.yc`) and the caller reads the
   result back — this only works if QBasic's by-reference parameter passing is
   honored (see §10, fix #6).
2. Its scan relies on `WINDOW`'s **Y-axis being inverted** (larger logical-Y is
   higher on screen) so that "Highest"/"Lowest" map to the right physical rows
   (see §10, fix #7).

---

## 6. The palette and the spin — SetPalette + TorusRotate

`SetPalette` mixes a palette of up to ~216 colors into the `Pal()` array. The
encoding depends on the mode (the program itself documents the split):

| Modes      | `Pal(i)` encoding              | Bits             |
|------------|--------------------------------|------------------|
| 1, 2, 7, 8 | `Hs*8 + Rs*4 + Gs*2 + Bs`      | 4-bit `irgb`     |
| 9          | `Rs*32+Gs*16+Bs*8+HRs*4+…`    | 6-bit EGA `rgbRGB` |
| 11, 12, 13 | `65536*Bs + 256*Gs + Rs`      | **18-bit VGA DAC** (each channel 0–63) |

`TorusRotate` cycles which `Pal()` entries are assigned to which color
attributes via the `PALETTE` statement. Because the *tiles* keep their fixed
attribute numbers but the *palette behind those attributes* rotates, the torus
appears to rotate without redrawing a single pixel — a classic palette-animation
trick.

---

## 7. Painter's algorithm — TorusSort

`TorusSort` is an in-place quicksort over the `Index()` array (not the `Tile`
array itself — sorting 2-byte indices is far cheaper than 46-byte records),
keyed on each tile's `z1`. `TorusDraw` then walks `Index()` in order, so tiles
are drawn farthest-Z first and nearer tiles overdraw them. The sort uses `SWAP
Index(i), Index(j)` repeatedly.

---

## 8. Data types and arrays

- `T()` — array of `Tile` (TYPE) → 12 parallel `Vec<f64>` fields in `GameState`
- `Index()` — `INTEGER` array, the sort permutation
- `Pal()` — `LONG` array of packed DAC color values
- `VC` — scalar `Config` (TYPE) → flattened `vc__scrn`, `vc__atribs`, …
- `TOR` — scalar `TORUS` (TYPE) → `tor__thick`, `tor__panel`, …
- Module-level scalars shared into SUBs via `SHARED name AS type`

Almost every SUB begins with `SHARED VC AS Config, TOR AS TORUS, …` — the
type-annotated SHARED form that the transpiler previously could not parse (§10,
fix #1).

---

## 9. Control flow

No GOTO-as-loop: the program is pure structured `SUB`/`FUNCTION` + `DO`/`LOOP` +
`SELECT CASE`. The only line labels are the `ON ERROR` targets. This means torus
emits entirely as clean named Rust `fn`s — no `__pc` state machine — which is
the path the transpiler handles best.

---

## 10. Why torus forced seven fixes

torus needed **three parser/emitter fixes to compile** and **four runtime fixes
to render**. The render bugs were layered: each one hid the next, and the only
reliable way to find them was a headless harness that dumped framebuffer color
statistics (`Runtime::fb_stats()`) after the draw, rather than eyeballing a black
window.

**Compile fixes (transpiler):**

1. **`SHARED name AS type` inside a SUB body.** The `Token::Shared` handler in
   `parser.rs` consumed an optional `()` but not the trailing `AS typename`, so
   every `SHARED VC AS Config` was a parse error. Fix: consume and discard the
   `AS` clause (the type is already known from the matching DIM).

2. **`PAINT STEP(0,0), color, border`.** `parse_paint` didn't accept the `STEP`
   relative-coordinate prefix. Fix: route it through the existing `opt_step()`
   helper and add a `step: bool` to `Stmt::Paint`; the emitter resolves STEP to
   `__rt.cur_x()/cur_y() + delta`, the same pattern PSET/LINE already used.

3. **Typed-array element passed to a SUB.** `TileDraw T(Index(Til))` passes one
   `Tile` element to a `SUB Foo(T AS Tile)`. The emitter expanded *whole* typed
   arrays and *scalar* TYPE vars but had no case for a subscripted element. Fix:
   evaluate the index once into a temp and emit per-field `&mut gs.t__field[idx]`.
   (Plus supporting work: scalar-`Config`/`TORUS` GameState fields, `REDIM …  AS
   Tile` resizing each field Vec, per-sub `SHARED`-name scoping, and PMAP in the
   non-lifting expression path.)

**Render fixes (runtime — `runtime/src/lib.rs`):**

4. **`WINDOW` without `VIEW` mapped everything to pixel (0,0).** `logical_to_fb`
   scaled logical coords onto the VIEW rectangle, but torus never calls `VIEW`,
   so that rect was zero-size and collapsed every point to the origin. Fix:
   `effective_viewport()` — an absent VIEW defaults to the whole screen, which is
   QB's behavior.

5. **SCREEN 12 PALETTE decoded as EGA, not VGA DAC.** torus mixes its palette as
   18-bit DAC values (§6), but `palette()` only used the DAC decode for SCREEN 13
   — SCREEN 12 fell through to the EGA 6-bit decode, which reads only the low
   bits (`= Rs`), and `Rs` is 0 for the first 36 entries → most tiles decoded to
   black. Fix: modes 11/12/13 all use `dac18_to_rgb`.

6. **FUNCTION parameters passed by value instead of by reference** — *the real
   "black screen".* `Inside()` writes `T.xc`/`T.yc` and `TileDraw` reads them
   back to position the PAINT. With by-value params those writes vanished, so
   every tile painted at logical (0,0). Fix: UDT FUNCTION params now pass by
   reference (per-field `&mut f64`), matching SUBs and QB semantics; call sites
   reborrow (`&mut *t__xc`).

7. **`WINDOW` didn't invert the Y axis.** QB's `WINDOW` (no `SCREEN`) puts larger
   logical-Y higher on screen. `Inside()`'s scan computes top/bottom physical
   rows and loops until it passes both; without inversion those bounds were
   swapped, the scan stopped after one step, and `Inside()` returned false for
   *every* tile — so each tile's border (color 15) was redrawn in background
   color 0 → fully black. Fix: invert Y in `logical_to_fb` and the `pmap` Y
   modes. mandel.bas is vertically symmetric, so the change is visually
   invisible there (verified headlessly: still renders).

---

## 11. The headless verification harness

Because the bugs were layered, each fix moved the framebuffer from "all black" to
"still all black" for a *different* reason. The breakthrough was instrumenting:

```rust
pub fn fb_stats(&self) -> (usize, usize)   // (non-background pixels, distinct colors)
```

and building a throwaway non-interactive copy of the emitted `torus.rs` (hardcode
the setup defaults, dump `fb_stats()` after `TorusDraw`, `std::process::exit`).
The progression told the story precisely:

```
after fixes 1–5:  nonzero=0   colors=1    ← still black: Inside() always false
after fix 6:      nonzero=0   colors=1    ← still black: Y bounds swapped
after fix 7:      nonzero=123138 colors=15  ← the torus renders, all 15 attributes
```

`fb_stats()` was kept in the runtime as a permanent diagnostic.

---

## 12. QB features this program exercises (transpiler checklist)

- ✅ Arrays of a user-defined `TYPE` (`DIM T() AS Tile`) — flattened to per-field Vecs
- ✅ Scalar `TYPE` variables (`VC AS Config`, `TOR AS TORUS`) — flattened GameState fields
- ✅ `SHARED name AS type` inside SUB/FUNCTION bodies
- ✅ `REDIM … AS Tile` (dynamic typed-array allocation, inside a loop)
- ✅ Typed-array element passed to a SUB / FUNCTION (by reference)
- ✅ FUNCTION parameters by reference (mutate-and-read-back)
- ✅ `WINDOW` logical coordinates with QB Y-axis inversion
- ✅ `PMAP` (all four modes), `POINT` for collision/interior tests
- ✅ `PAINT STEP`, `PRESET`, `LINE -(x,y)` relative
- ✅ `PALETTE` with 18-bit VGA DAC values (SCREEN 12); palette-cycle animation
- ✅ `SWAP` of array elements (quicksort)
- ✅ `SELECT CASE`, `DO`/`LOOP`, nested SUB/FUNCTION calls, recursion (TorusSort)
- ✅ `DEFINT`/`DEFSNG` (no-ops under all-f64 storage)
- ✅ `HEX$`, `VAL`, `INSTR`, `STR$`, `SPACE$`, `PRINT USING`
- ⚠️ `ON ERROR GOTO` / `RESUME` — stubbed (hardware probes never fail natively)
- ⚠️ `WIDTH`, `LOCATE` cursor-shape args — parsed and ignored (no effect windowed)

SCREEN 12 (640×480, 16 colors) is the default render path; `GetConfig`'s probe
chain (`VGA → MCGA → EGA → CGA → MONO → HERC`) collapses to VGA on modern
hardware.
