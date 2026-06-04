# REVERSI.BAS — How It Works

An architectural walkthrough of `basic-src/reversi.bas`, the Reversi/Othello game
that shipped with MS-DOS QBasic 1.1 (`\DOS\REVERSI.BAS`, 590 lines). Like TORUS
it is all structured `SUB`/`FUNCTION` code — no GOTO state machine — but it is a
full interactive game: an EGA board, a keyboard cursor, an AI opponent with two
difficulty levels, a help screen, and live scoring.

This doc doubles as the record of *why reversi was the last bundled program to
transpile* — it forced four parser/emitter features, two codegen-collision
fixes, and a runtime coordinate-orientation fix (§9–§10).

---

## 1. What the game is

Reversi (Othello) on an 8×8 board. You are **red**, the computer is **blue** (on
a monochrome adapter, white vs black). You take turns placing a disc so that it
brackets one or more of the opponent's discs in a straight line (horizontal,
vertical, or diagonal); every bracketed disc flips to your color. When neither
side can move, the player with more discs wins.

The screen is laid out as a left **control/scoreboard** text panel and a right
**graphics board**:

```
  R E V E R S I            (title, centered)

  Game Controls            ┌─────────────────┐
   S = Start New Game      │  8 × 8 board     │
   P = Pass Turn           │  with red/blue   │
   D = Set Difficulty      │  discs and a     │
   H = Display Help        │  movable cursor  │
   Q = Quit                └─────────────────┘
  Game Status
   Your Score:      2
   Computer Score:  2
   Difficulty:   Novice
```

Arrow keys move the cursor (it wraps at the edges); **Enter**/**Space** place a
disc on a legal square; **S** restarts, **P** passes (first move only), **D**
toggles Novice/Expert, **H** shows help, **Q** quits.

---

## 2. Top-level execution flow

```
ON ERROR GOTO BadMode
DO: READ smode : SCREEN smode : LOOP UNTIL vmode   ' probe a graphics mode
IF smode = 0 THEN  …no graphics, bail…
ELSE
   GS.stat = START
   WHILE GS.stat <> QUIT
      IF GS.stat = START THEN InitGame : DrawGameBoard
      IF it's the human's turn THEN
         IF ValidMove(COMP) THEN UserMove           ' human has moves → play
         ELSEIF ValidMove(HUMAN) THEN …force pass… ComputerMove
         ELSE GameOver
      ELSE  ' computer's turn (mirror image)
         …ComputerMove / pass / GameOver…
      END IF
   WEND
END IF
DATA 9, 10, 2, 3, 0
```

The `SCREEN smode` probe reads modes from `DATA 9,10,2,3,0` and tries each until
one works, falling through `BadMode` (`vmode = FALSE : RESUME NEXT`) on failure.
On modern hardware SCREEN 9 (EGA 640×350) succeeds immediately, so the
transpiler's `ON ERROR`/`RESUME` stub is sufficient (same as torus). `smode`
ends at 9 → the 16-color EGA color path.

---

## 3. The two user-defined TYPEs

```basic
TYPE GameGrid              ' one board square
   player AS INTEGER       ' which color sits here (or the empty/background attr)
   nTake  AS INTEGER       ' # discs this square would flip if played (0 = illegal)
   cx, cy AS INTEGER       ' the square's pixel center, for drawing
END TYPE

TYPE GameStatus            ' overall game state
   curRow, curCol AS INTEGER   ' cursor position (1..8)
   stat   AS INTEGER            ' whose turn / game phase
   rScore, bScore AS INTEGER    ' red / blue disc counts
   mDisplay AS INTEGER          ' a status message is showing
   dLevel AS STRING * 6         ' "Novice" / "Expert" (fixed-length string)
   GColor AS INTEGER
END TYPE
```

The shared globals tie it together:

```basic
DIM SHARED GS AS GameStatus, smode AS INTEGER
DIM SHARED GG(8, 8) AS GameGrid, GBoard AS INTEGER   ' 2-D array of a TYPE
DIM SHARED COMP AS INTEGER, HUMAN AS INTEGER, BG AS INTEGER
DIM SHARED GP(8, 8, 8) AS INTEGER, GW(8, 8) AS INTEGER   ' 3-D + 2-D arrays
```

- **`GG(8,8)`** — a 2-D array of `GameGrid`. The transpiler flattens it to one
  `Vec<Vec<f64>>` per field: `gg__player[r][c]`, `gg__ntake[r][c]`, `gg__cx`,
  `gg__cy` (CLAUDE.md decisions #3/#10, extended to 2-D).
- **`GP(8,8,8)`** — the **3-D** "take-paths" array: `GP(r,c,d)` is how many discs
  would flip from square `(r,c)` in direction `d` (1–8 = the eight compass rays).
- **`GW(8,8)`** — a static positional-weight table (corners 99, edges 5, else 2)
  used by the Expert AI.

---

## 4. Setup and the board — InitGame + DrawGameBoard

`InitGame` picks colors per mode (SCREEN 9: HUMAN=4 red, COMP=1 blue, BG=3,
GBoard=8 the empty attr), sets the logical coordinate system, computes every
square's pixel center, seeds the four center discs, and fills `GW`:

```basic
WINDOW SCREEN (640, 480)-(0, 0)            ' see §10 — resolution-independent coords
GG(row,col).cx = 270 + (col - .5) * 40     ' board occupies x≈270–590
GG(row,col).cy = 70  + (row - .5) * 40      ' y≈70–390
GG(4,4)=HUMAN : GG(5,5)=HUMAN : GG(5,4)=COMP : GG(4,5)=COMP   ' opening position
```

`DrawGameBoard` draws the panels (`LINE …,B/BF`), flood-fills them
(`PAINT (x,y),color,border`), prints the control/status text (`LOCATE`+`PRINT`),
draws the 8×8 grid lines, and places any non-empty discs via `DrawGamePiece`
(a `CIRCLE` + `PAINT`). The B&W path (`GBoard = 85`) uses `PAINT (x,y),CHR$(85),0`
— a string *tiling pattern* — but that branch is dead in SCREEN 9 (see §9, fix 4).

---

## 5. The cursor and input — UserMove

`UserMove` loops on `INKEY$`, decoding the extended scan code via
`ASC(RIGHT$(a$, 1))`:

```basic
SELECT CASE move
   CASE 71 TO 81:           ' the arrow / diagonal block
      …erase old cursor…
      IF move < 74 THEN  curRow up   (wraps 1↔8)
      ELSEIF move > 78 THEN curRow down
      IF move IN (71,75,79) THEN curCol left
      ELSEIF move IN (73,77,81) THEN curCol right
      …draw new cursor…
   CASE START:  GS.stat = START
   CASE PASS:   …only legal on the first move…
   CASE HELP:   DisplayHelp
   CASE DIFF:   toggle "Novice"/"Expert"
   CASE ENTER, SPACE:
      IF GG(curRow,curCol).nTake > 0 THEN TakeBlocks …, HUMAN : GS.stat = COMP
      ELSE  DisplayMsg "Invalid move…"
   CASE QUIT:   GS.stat = QUIT
END SELECT
```

`DrawCursor` draws a circle on a *legal* square (`nTake > 0`) or a small
crosshair on an illegal one, so the player can see at a glance where they may
play. Because the cursor is drawn through the `WINDOW` coordinate system, its
on-screen direction depends on that mapping being correct (§10).

---

## 6. Move legality — ValidMove + CheckPath

`ValidMove(Opponent)` is the heart of the rules. For every empty square it walks
the eight compass directions with `CheckPath`, recording the flip count per
direction into `GP(row,col,dir)` and summing them into `GG(row,col).nTake`:

```basic
ERASE GP                                   ' zero the 3-D take-paths array
FOR row, col …
   IF GG(row,col).player = GBoard THEN       ' empty square
      GP(row,col,1) = CheckPath(row,row,0, col-1,0,-1, Opponent)   ' west
      …seven more directions…
      GG(row,col).nTake = Σ GP(row,col,*)
      IF nTake > 0 THEN ValidMove = TRUE
```

`CheckPath` steps from a starting square in a fixed `(IStep, JStep)` direction:
it counts a run of opponent discs and returns that count **only if** the run is
capped by one of your own discs (a legal bracket); otherwise it returns 0.

This is exactly correct: at the opening position `ValidMove` reports **4 legal
moves**, the textbook Reversi opening — which is how the transpiled build was
verified end-to-end (it exercises `ERASE`, the 3-D `GP`, and `CheckPath`).

---

## 7. Making a move — TakeBlocks

`TakeBlocks(row,col,player)` places the disc and replays the eight `GP` counts to
flip the bracketed discs, redrawing each with `DrawGamePiece`, then updates the
scores. The 3-D `GP` array is what makes this O(flips) instead of re-scanning.

---

## 8. The AI — ComputerMove

The computer scans every square with `nTake > 0` and scores it:

- **Novice**: `value = nTake + GW(row,col)` — flips plus positional weight
  (grab corners, avoid the squares next to them).
- **Expert**: adds edge/corner heuristics — bonuses for moves that secure an edge
  or deny the opponent a corner-adjacent square (the nested `SELECT CASE row` /
  `SELECT CASE col` blocks).

It plays the highest-scoring square via `TakeBlocks`, then hands the turn back.

---

## 9. Why reversi forced six transpiler fixes

reversi needed **four features to compile** and **two codegen collisions
resolved**, plus the runtime orientation fix in §10.

1. **`SHARED`/`DIM` plumbing already in place** — 2-D arrays of a TYPE
   (`GG(8,8) AS GameGrid`), `STRING * 6` TYPE fields (`dLevel`), and
   `OPTION BASE 1` were all already supported (the last is a no-op under the
   wasted-slots strategy). No work needed.

2. **`WINDOW SCREEN` (parse blocker)** — the parser only accepted plain
   `WINDOW (…)`. Added an optional `SCREEN` keyword → `Stmt::Window.screen`, and
   a runtime `win_screen` flag (§10).

3. **`ERASE arrayname` (parse blocker)** — `ValidMove` calls `ERASE GP` every
   turn. Added `Token::Erase` + `Stmt::Erase`; the emitter zeroes the array in
   place with loop-nesting matched to its dimensionality (an `array_dims` map).

4. **3-D plain arrays (`GP(8,8,8)`)** — previously a 3-D `DIM` silently allocated
   only 2-D and dropped the third index, which would have broken every flip.
   Generalized the emitter with `nested_vec_type`/`nested_vec_init` helpers,
   threaded through the GameState struct decl, `emit_dim`, and `emit_redim`.
   (Element access already iterated every index.)

5. **`PAINT (x,y), CHR$(n), border` string tiling pattern (compile blocker)** —
   `paint()` takes an `f64` fill, so a string pattern wouldn't compile. It is
   dead code on the EGA color path, so the emitter now flags it (a `// TODO` +
   stderr warning) and emits a solid foreground flood. Real pattern tiling is
   left for a program that needs it.

6. **Two codegen collisions surfaced while compiling:**
   - **FUNCTION arg reading a shared field** — `ValidMove(COMP)` emitted
     `validmove(&mut __gs, __gs.comp)`, a borrow conflict (E0503). Such args are
     now hoisted to a temp inside a block expression before the call.
   - **Scalar/array same name** — QB lets `A$` and `A$()` coexist; `DisplayHelp`
     uses both (`DIM a$(1 TO 18)` for the help text *and* `a$ = INKEY$`). The
     emitter now suffixes the colliding scalar binding (`local_scalar_name`) so
     they don't share one Rust local.

---

## 10. `WINDOW SCREEN` and the coordinate orientation

`InitGame` runs `WINDOW SCREEN (640, 480)-(0, 0)`. The intent is **resolution
independence**: the game's drawing code is written in a fixed 640×480 logical
space, and `WINDOW` scales it to whatever the actual mode is (SCREEN 9 is
640×350, others differ). `SCREEN` selects *screen orientation* — Y increases
downward — as opposed to plain `WINDOW`'s Cartesian (Y-up) convention that torus
relies on.

The subtlety is the **reversed corners** `(640,480)-(0,0)`. A naive linear map
that honors corner order maps logical (640,480) to the top-left pixel and (0,0)
to the bottom-right — flipping **both** axes. That renders the board rotated 180°
on the *left* half (overlapping the control panel) and, because the cursor is
drawn through these same coordinates, makes the **arrow keys move backwards**.

The fix: `WINDOW SCREEN` maps by coordinate **magnitude** — smallest logical
coordinate → top-left, largest → bottom-right — so corner order can't flip the
image. With that, logical (290,90) → pixel (290,65) (board cell 1,1, upper-left)
and (570,390) → (569,284) (cell 8,8, lower-right): board on the right, row 1 at
top, cursor tracking the keys. Plain `WINDOW` (Cartesian, Y-inverted — torus,
mandel) is unchanged. Implemented in `logical_to_fb` and `pmap`
(`runtime/src/lib.rs`), guarded by `window_screen_reversed_corners_no_flip` and
`window_screen_no_y_invert` tests.

---

## 11. QB features this program exercises (transpiler checklist)

- ✅ 2-D array of a `TYPE` (`GG(8,8) AS GameGrid`) → per-field `Vec<Vec<f64>>`
- ✅ **3-D** plain array (`GP(8,8,8)`) — `nested_vec_type`/`nested_vec_init`
- ✅ `ERASE` (dimension-aware in-place zeroing)
- ✅ `WINDOW SCREEN` (screen-orientation, magnitude-mapped coordinates)
- ✅ `STRING * 6` fixed-length TYPE field; `OPTION BASE 1` (no-op)
- ✅ Scalar/array same-name coexistence (`A$` and `A$()`)
- ✅ FUNCTIONs returning `%` (`CheckPath%`, `ValidMove%`); by-value args hoisted
  when they read shared state
- ✅ `SELECT CASE` with ranges (`CASE 71 TO 81`) and multi-value (`CASE ENTER, SPACE`)
- ✅ `LINE …,B/BF`, `CIRCLE`, `PAINT (x,y),color,border`
- ✅ `INKEY$` extended scan codes, `SLEEP`, `LOCATE`+`PRINT`, `DATA/READ`
- ✅ `ABS`, `ASC`, `RIGHT$`, `LEN`, nested SUB/FUNCTION calls
- ⚠️ `ON ERROR GOTO`/`RESUME NEXT` — stubbed (the SCREEN probe succeeds natively)
- ⚠️ `PAINT` with a `CHR$` tiling pattern — solid fill stub (dead on the EGA path)

SCREEN 9 (EGA 640×350, 16 colors) is the render path; the `DATA 9,10,2,3,0`
mode probe collapses to 9 on modern hardware.
