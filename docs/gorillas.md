# GORILLA.BAS — How It Works

A line-by-line architectural walkthrough of `basic-src/gorilla.bas`, the
QBasic Gorillas game (© Microsoft Corporation 1990, 1135 lines). This is the
transpiler's primary correctness target. Line numbers below refer to the
source file as shipped.

---

## 1. What the game is

Two gorillas stand on a randomly generated city skyline. Players take turns
throwing an exploding banana at each other by entering an **angle** (0–360°)
and a **velocity**. The banana follows a parabolic trajectory affected by
**gravity** and **wind**. A direct hit on the opposing gorilla wins the round;
a hit on a building blows a crater in it; a banana that flies off-screen
misses. First to the chosen number of points wins the match.

---

## 2. Top-level execution flow

The module-level body (lines 105–122) is the entire driver:

```
DEF FnRan (x) = INT(RND(1) * x) + 1     ' 105  inline RNG helper: 1..x
DEF SEG = 0 : POKE 1047, ... : DEF SEG  ' 106-111  force NumLock ON via BIOS
GOSUB InitVars                          ' 113  one-shot initialization (label)
Intro                                   ' 114  title screen + instructions
GetInputs Name1$, Name2$, NumGames      ' 115  player names, points, gravity
GorillaIntro Name1$, Name2$             ' 116  draw gorillas, optional dance
PlayGame Name1$, Name2$, NumGames       ' 117  ← the actual match loop
DEF SEG = 0 : POKE 1047, KeyFlags ...   ' 119-121  restore NumLock
END                                     ' 122
```

`InitVars`, `CGABanana`, and `EGABanana` are **line-label GOSUB targets / DATA
blocks** (lines 125–223), not SUBs. Everything else is a `SUB`/`FUNCTION`.

```
Intro ──► GetInputs ──► GorillaIntro ──► PlayGame
                                            │
                                 ┌──────────┴───────────┐
                                 │  per round (DO loop)  │
                                 │  MakeCityScape        │
                                 │  PlaceGorillas        │
                                 │  DoSun                │
                                 │  DoShot ─► PlotShot ──┼─► DoExplosion
                                 │             │         │   ExplodeGorilla
                                 │             └─POINT()─┘   VictoryDance
                                 │  UpdateScores         │
                                 └───────────────────────┘
```

---

## 3. Data types and global state

A single user-defined `TYPE` holds a building's top-left corner:

```basic
TYPE XYPoint          ' lines 53-56
  XCoor AS INTEGER
  YCoor AS INTEGER
END TYPE
```

`DEFINT A-Z` (line 21) makes every un-sigiled variable an INTEGER for speed;
explicit sigils override this (`#`=double, `!`=single, `$`=string, `&`=long).

Key `DIM SHARED` globals (lines 76–103):

| Variable | Meaning |
|---|---|
| `GorillaX(1 TO 2)`, `GorillaY(1 TO 2)` | Pixel positions of the two gorillas |
| `LastBuilding` | Index of the right-most building |
| `pi#` | `4 * ATN(1)` — computed, not a literal |
| `LBan&`, `RBan&`, `UBan&`, `DBan&` | Banana sprite bitmaps (Left/Right/Up/Down rotation), stored as `LONG` arrays |
| `GorD&`, `GorL&`, `GorR&` | Gorilla sprite bitmaps (arms Down / Left-up / Right-up) |
| `gravity#`, `Wind` | Physics parameters for the current round |
| `Mode`, `ScrWidth`, `ScrHeight`, `MaxCol`, `GHeight` | Screen geometry |
| `ExplosionColor`, `SunColor`, `BackColor`, `SunHit`, `SunHt` | Color/state |
| `MachSpeed AS SINGLE` | Calibrated speed factor (see §13) |

The sprite arrays are the heart of the rendering: gorillas and bananas are
drawn **once**, captured into these arrays with `GET`, then blitted around the
screen with `PUT` (28 `PUT` calls across the program).

---

## 4. Screen-mode auto-detection (the `ON ERROR` trick)

> **Note for this repo:** Gorillas does **not** use `SCREEN 7`. It prefers
> **`SCREEN 9`** (EGA, 640×350, 16 colors) and falls back to **`SCREEN 1`**
> (CGA, 320×200, 4 colors). The `CLAUDE.md` "SCREEN 7" note describes the
> general 320×200 16-color EGA target, but the real file negotiates 9-or-1.

`InitVars` (lines 145–208) picks the best available mode by *deliberately
triggering errors* and trapping them:

```basic
ON ERROR GOTO ScreenModeError   ' 149
Mode = 9 : SCREEN Mode          ' 150-151  try EGA; fails on CGA-only hardware
ON ERROR GOTO PaletteError      ' 152
IF Mode = 9 THEN PALETTE 4, 0   ' 153  fails on 64K (cut-down) EGA cards
ON ERROR GOTO 0                 ' 154  disable trapping
```

- `ScreenModeError` (210–219): if `SCREEN 9` faulted, set `Mode = 1` and
  `RESUME` (retry the failed statement in CGA). If even CGA fails, print a
  "need CGA/EGA/VGA" message and `END`.
- `PaletteError` (221–223): a `PALETTE` failure means a 64K EGA card → drop to
  `Mode = 1` and `RESUME NEXT` (skip the offending line).

Once the mode is known, geometry constants are set (EGA: 640×350, `GHeight`
25; CGA: 320×200, `GHeight` 12) and the correct banana `DATA` is loaded.

---

## 5. Sprites: DATA → READ → GET/PUT

### Banana bitmaps
The banana is stored as raw `PUT`-format bitmap words in `DATA` statements,
two sets (lines 125–143): `CGABanana` (3 longs per rotation) and `EGABanana`
(9 longs per rotation). `InitVars` `RESTORE`s the right block and `READ`s the
four rotations into `LBan&/DBan&/UBan&/RBan&`:

```basic
RESTORE EGABanana
REDIM LBan&(8), RBan&(8), UBan&(8), DBan&(8)   ' 163  dynamic re-dimension
FOR i = 0 TO 8 : READ LBan&(i) : NEXT i        ' 165-167  (then D, U, R)
```

### Gorilla bitmaps
The gorillas are **drawn vectorially once** by `DrawGorilla` (§8) during
`GorillaIntro`, then captured with `GET` into `GorD&/GorL&/GorR&`. From then
on they are only ever `PUT` — that is why moving a gorilla, throwing, the
victory dance, etc. are all fast bitmap blits.

`DrawBan` (377–390) wraps the four rotations behind a `SELECT CASE r`. It uses
`PUT ..., PSET` to draw and `PUT ..., XOR` to erase (XOR-blit is reversible):

```basic
CASE 0: IF bc THEN PUT (xc#,yc#), LBan&, PSET ELSE PUT (xc#,yc#), LBan&, XOR
```

---

## 6. Building the skyline — `MakeCityScape` (689–808)

Receives the building array **by reference as a TYPE array**:
`SUB MakeCityScape (BCoor() AS XYPoint)`.

1. **Pick a slope trend** with `FnRan(6)` (694–700): upward, downward, "V"
   (most common, cases 3–5), or inverted "V". This biases successive building
   heights.
2. **Loop placing buildings left→right** (724–786). For each:
   - Adjust `NewHt` per the slope trend.
   - Random width `BWidth = FnRan(DefBWidth) + DefBWidth`, clamped to screen.
   - Random height `BHeight = FnRan(RandomHeight) + NewHt`, clamped so it
     never overlaps where gorillas will stand (`MaxHeight + GHeight`).
   - Store the top-left corner: `BCoor(CurBuilding).XCoor/.YCoor`.
   - Draw outline (`LINE ..., B`) then fill (`LINE ..., BF`); EGA picks a
     random building color 4–6, CGA uses color 2.
   - **Draw windows** (768–780): a nested loop stamps small filled rectangles;
     each window is randomly "lit" (yellow / color 8 / dark).
3. `LastBuilding = CurBuilding - 1` (788).
4. **Wind** (790–807): `Wind = FnRan(10) - 5`, occasionally amplified, giving
   roughly −15..+15. A red arrow is drawn at the bottom center whose length is
   proportional to wind strength and whose head points downwind.

---

## 7. Placing the gorillas — `PlaceGorillas` (816–838)

Also takes `BCoor() AS XYPoint`. Player 1 stands on the 2nd or 3rd building
from the left (`FnRan(2)+1`); Player 2 on the 2nd or 3rd from the right
(`LastBuilding - FnRan(2)`). The gorilla is centered on the building roof
(width/2, minus per-mode `XAdj/YAdj`) and blitted with `PUT (...), GorD&, PSET`.
`GorillaX/Y(i)` are stored for the physics and explosion code.

---

## 8. Drawing a gorilla — `DrawGorilla` (399–453)

Pure vector art built from `LINE ... BF` rectangles (head, body, legs) and
`CIRCLE` arcs (rounded chest/legs/arms), with `PSET` dots for eyes/nose. The
`arms` parameter selects which arm is raised:

```basic
SELECT CASE arms            ' 435
  CASE 1  'RIGHTUP  → ...then GET into GorR&
  CASE 2  'LEFTUP   → ...then GET into GorL&
  CASE 3  'ARMSDOWN → ...then GET into GorD&
```

Each case ends with a `GET (...) , Gor?&` to snapshot the freshly drawn
gorilla into the appropriate sprite array. `DIM i AS SINGLE` (line 400) is a
deliberate local override of `DEFINT A-Z` because the arc loops step by
fractional `Scl()` values.

---

## 9. The core — `PlotShot` (902–1020)

This is where the game lives. It animates the banana and detects collisions.

### Setup
```basic
Angle# = Angle# / 180 * pi#         ' degrees → radians
InitXVel# = COS(Angle#) * Velocity  ' horizontal launch speed
InitYVel# = SIN(Angle#) * Velocity  ' vertical launch speed
```
A throw animation plays (raise arm via `PUT GorL&/GorR&`, throw sound, lower
arm), then a stepping loop integrates the trajectory.

### Trajectory (per step, `t#` += 0.1)
```basic
x# = StartXPos + (InitXVel# * t#) + (.5 * (Wind / 5) * t# ^ 2)
y# = StartYPos + ((-1 * (InitYVel# * t#)) + (.5 * gravity# * t# ^ 2)) * (ScrHeight / 350)
```
Classic kinematics `p = p₀ + v·t + ½·a·t²`:
- **Horizontal**: constant launch velocity plus *wind as acceleration*
  (`Wind/5`).
- **Vertical**: `-InitYVel·t` (up; screen Y grows downward) plus *gravity*
  pulling back down, scaled to the resolution.

Off-screen test ends the loop with `OnScreen = FALSE` (966–968).

### Collision via `POINT()` — the crucial mechanic
For each on-screen step, the banana's **leading edge** is sampled against the
framebuffer (974–992):

```basic
pointval = POINT(x# + LookX, y# + LookY)
IF pointval = 0 THEN
  Impact = FALSE                       ' background → keep flying
ELSEIF pointval = SUNATTR AND y# < SunHt THEN
  IF NOT SunHit THEN DoSun SUNSHOCK    ' hit the sun → shocked face
  SunHit = TRUE : ShotInSun = TRUE
ELSE
  Impact = TRUE                        ' any other color → something solid
END IF
```

`POINT(x,y)` returns the **EGA palette index already at that pixel** — this is
exactly the palette-indexed-framebuffer design the runtime must preserve (no
RGBA round-trip). `OBJECTCOLOR` (1) means a gorilla; anything else non-zero
(and not the sun) means a building/ground. The banana is only actually drawn
(`DrawBan ... TRUE`) when it is neither impacting nor passing through the sun,
and the previous frame is XOR-erased first (`NeedErase`).

### Outcome (1012–1018)
```basic
IF pointval <> OBJECTCOLOR AND Impact THEN
  CALL DoExplosion(x# + adjust, y# + adjust)   ' crater in a building
ELSEIF pointval = OBJECTCOLOR THEN
  PlayerHit = ExplodeGorilla(x#, y#)           ' direct gorilla hit
END IF
PlotShot = PlayerHit                           ' 0 = nobody, else 1 or 2
```

---

## 10. Explosions

- **`DoExplosion` (255–269)** — a banana hitting a building. Plays a short
  `PLAY` melody, then expands a filled `CIRCLE` in `ExplosionColor` and
  contracts it again in `BACKATTR` (background), leaving a round crater. This
  crater is *real*: subsequent `POINT()` checks see background there, so the
  skyline is genuinely destructible.
- **`ExplodeGorilla` (459–484)** — a direct hit. Determines which gorilla
  (left half vs right half of screen), plays a melody, and animates a vertical
  fireball over the victim with concentric `CIRCLE`s (note the `, , , -1.57`
  aspect-ratio argument squashing the circle into an oval). Returns the hit
  player's number.

---

## 11. The sun — `DoSun` (328–368)

Drawn once per round at top-center. Body is `CIRCLE` + `PAINT` flood-fill; eight
`LINE`s form rays. The `Mouth` parameter toggles the face:
- `SUNHAPPY` (FALSE) → a smile arc (`CIRCLE` with start/end angles).
- `SUNSHOCK` (TRUE) → an "O" mouth (`CIRCLE` + `PAINT`), shown when a banana
  passes through the sun (`SunHit`). Reset to happy after the round/shot.

---

## 12. Input and text UI

- **`GetInputs` (491–526)** — `LINE INPUT` for names (truncated to 10 chars),
  validated `INPUT` loops for total points and gravity (defaults 3 and 9.8).
- **`GetNum#` (532–571)** — the in-game angle/velocity entry. A hand-rolled
  `INKEY$` loop that accepts `0`–`9` and one `.`, handles Backspace
  (`CHR$(8)`) and Enter (`CHR$(13)`), rejects values > 360, and `BEEP`s on
  invalid keys. It echoes a `CHR$(95)` (`_`) cursor while typing.
- **`Center` (244–248)** — prints a string centered on a row using `MaxCol`.
- **`SparklePause` (1075–1104)** — animated `*` border that runs until any key
  is pressed; used on the intro and game-over screens.

---

## 13. Timing and speed calibration

### Original DOS design

The original gorilla.bas self-calibrated to the host machine's CPU speed:

- **`CalcDelay!` (original, 228–236)** — busy-counts increments of a `SINGLE`
  loop variable for 0.5 s via `TIMER` and returns the count (a rough "ticks per
  half-second" number). On a 10 MHz 286 this yields ~500, matching
  `SPEEDCONST = 500` (line 61).
- **`Rest (t#)` (original, 1024–1029)** — pauses for `t#` "game seconds" by
  spinning until `TIMER - s# > MachSpeed * t# / SPEEDCONST`. The formula
  normalizes to real seconds when `MachSpeed ≈ SPEEDCONST`.

### What goes wrong on native hardware

On a modern CPU the busy-loop in `CalcDelay!` finishes billions of iterations
in 0.5 s, making `MachSpeed` roughly 2 × 10⁹. A call like `Rest .1` then waits
`2e9 × 0.1 / 500 = 400,000 seconds` instead of 0.1 s — the game freezes
permanently on every throw.

Separately, the original tight spin in `Rest` never yields to the OS, which on
macOS starves the Cocoa event loop of CPU time, causing the window to lose
keyboard focus and stop accepting input.

### Changes made in `basic-src/gorilla.bas`

```basic
'CalcDelay! — return SPEEDCONST directly so MachSpeed == SPEEDCONST
'and Rest() uses a 1:1 second-to-second mapping on any hardware.
FUNCTION CalcDelay!
  CalcDelay! = SPEEDCONST
END FUNCTION

'Rest — compare TIMER directly (MachSpeed * t# / SPEEDCONST == t# because
'CalcDelay returns SPEEDCONST); call INKEY$ each iteration to pump OS events
'so the window stays alive and keeps keyboard focus.
SUB Rest (t#)
  DIM s AS DOUBLE
  DIM Dummy AS STRING
  s = TIMER
  DO
    Dummy = INKEY$          'pump OS events; keeps window alive and focused
  LOOP UNTIL TIMER - s >= t#
END SUB
```

With `MachSpeed = SPEEDCONST`, the formula `MachSpeed * t# / SPEEDCONST`
collapses to `t#` and `Rest` waits exactly `t#` real seconds. The `INKEY$`
call inside the loop keeps the Cocoa/minifb event pump running so the window
retains keyboard focus throughout throws and animations.

> **`INKEY$` is now cheap and non-blocking.** `Rest`, `GetNum#`, and
> `SparklePause` all poll `INKEY$` in tight loops. The runtime disables minifb's
> built-in 4 ms rate limiter (`set_target_fps(0)`) and `inkey()` only blits the
> framebuffer once per frame interval (otherwise just pumps events + harvests
> keys), so these loops run at full speed and stay responsive — see
> `docs/ARCHITECTURE.md §Key input`.

> **Other runtime pacing levers:** the `REM QBC FPS N` and `REM QBC SLOWMO N`
> pragmas in the source can trim frame rate further if needed without touching
> `SPEEDCONST`.

---

## 14. Match loop and scoring — `PlayGame` (845–893)

```basic
DIM BCoor(0 TO 30) AS XYPoint   ' building array (TYPE array, by-ref to subs)
DIM TotalWins(1 TO 2)           ' scores

FOR i = 1 TO NumGames
  CLS : RANDOMIZE (TIMER)
  CALL MakeCityScape(BCoor()) : CALL PlaceGorillas(BCoor()) : DoSun SUNHAPPY
  DO WHILE Hit = FALSE
    J = 1 - J                       ' alternate turns (J flips 0/1)
    ... print names + "score" banner ...
    Tosser = J + 1 : Tossee = 3 - J
    Hit = DoShot(Tosser, GorillaX(Tosser), GorillaY(Tosser))
    IF SunHit THEN DoSun SUNHAPPY   ' un-shock the sun
    IF Hit = TRUE THEN CALL UpdateScores(TotalWins(), Tosser, Hit)
  LOOP
  SLEEP 1
NEXT i
```

- **Turn taking** uses `J = 1 - J` to flip between players each iteration.
- **`DoShot` (278–321)** prompts the current player (Player 2's angle is
  mirrored, `Angle# = 180 - Angle#`, since he faces left), calls `PlotShot`,
  and on a hit triggers `VictoryDance`. If a player destroyed *his own*
  gorilla (`PlayerHit = PlayerNum`), the dancer is swapped to the opponent
  (`PlayerNum = 3 - PlayerNum`).
- **`UpdateScores` (1112–1118)** credits the winner. (The `HITSELF` branch is
  defensive; in this call path `Results` is always `TRUE`, so the `Tosser`
  entry of `Record()` is incremented.)
- **`VictoryDance` (1124–1134)** alternates `GorL&`/`GorR&` PUTs with a melody.

After all games, a text scoreboard is shown and `SparklePause` waits for a key.

---

## 15. SUB / FUNCTION reference

| Routine | Line | Role | Notable QB features |
|---|---|---|---|
| `CalcDelay!` | 228 | Returns `SPEEDCONST` (was: machine-speed probe) | original: `TIMER` busy-count; patched for native |
| `Center` | 244 | Center text on a row | `LOCATE`, `LEN` |
| `DoExplosion` | 255 | Building crater | `PLAY`, expanding/contracting `CIRCLE` |
| `DoShot` | 278 | Prompt + resolve a throw | returns hit, mirrors P2 angle |
| `DoSun` | 328 | Draw the sun face | `CIRCLE`, `PAINT`, `LINE`, `PSET` |
| `DrawBan` | 377 | Draw/erase banana | `SELECT CASE`, `PUT PSET/XOR` |
| `DrawGorilla` | 399 | Vector-draw + snapshot gorilla | `LINE BF`, `CIRCLE` arcs, `GET` |
| `ExplodeGorilla` | 459 | Direct-hit fireball | oval `CIRCLE` (aspect arg), returns player |
| `GetInputs` | 491 | Names / points / gravity | `LINE INPUT`, `INPUT`, `VAL` |
| `GetNum#` | 532 | In-game numeric entry | `INKEY$`, `SELECT CASE` ranges, `BEEP` |
| `GorillaIntro` | 579 | First gorilla draw + optional dance | `VIEW PRINT`, `PALETTE`, `PLAY` |
| `Intro` | 661 | Title screen | `SCREEN 0`, `WIDTH`, `PLAY`, `SparklePause` |
| `MakeCityScape` | 689 | Random skyline + wind | TYPE array by-ref, `LINE B/BF`, `FnRan` |
| `PlaceGorillas` | 816 | Seat gorillas on roofs | TYPE array by-ref, `PUT` |
| `PlayGame` | 845 | Match loop + scoreboard | `RANDOMIZE TIMER`, `CALL`, `SLEEP` |
| `PlotShot` | 902 | **Trajectory + `POINT()` collision** | physics, `POINT`, `DrawBan` |
| `Rest` | 1024 | Pause `t#` real seconds; pumps OS events via `INKEY$` | original: `TIMER` busy-wait without event pump; patched for native |
| `Scl` | 1036 | CGA/EGA coordinate scaler | `CINT`, `INT` |
| `SetScreen` | 1051 | Per-mode palette/colors | `PALETTE`, `COLOR` |
| `SparklePause` | 1075 | Animated border until keypress | `INKEY$`, `MID$`, `LOCATE` |
| `UpdateScores` | 1112 | Increment a score | array by-ref |
| `VictoryDance` | 1124 | Winner animation | `PUT`, `PLAY`, `Rest` |

**Declared but never defined (dead declarations):** `EndGame`,
`ClearGorillas`, `Getn#`. The transpiler can ignore these.

---

## 16. QB features this program exercises (transpiler checklist)

Gorillas is a near-complete stress test of the language surface:

- **Control flow:** `SUB`/`FUNCTION` with by-ref params, `GOSUB`+line labels,
  `SELECT CASE` (incl. `CASE x TO y` ranges and `CASE ELSE`), `DO/LOOP UNTIL`,
  `WHILE/WEND`, multi-line `IF/ELSEIF/ELSE`.
- **Types & data:** user-defined `TYPE` arrays passed by reference,
  `DEFINT A-Z` with per-variable sigil overrides, `DIM SHARED`, dynamic
  `REDIM`, `DATA`/`READ`/`RESTORE` (to labels).
- **Graphics:** `SCREEN 9`/`1` negotiation, `LINE` (plain/`B`/`BF`), `CIRCLE`
  (with start/end angle and aspect args), `PAINT`, `PSET`, `GET`/`PUT` sprite
  blits (`PSET`/`XOR`), `PALETTE`, `VIEW PRINT`, `COLOR`, `CLS n`.
- **The collision model:** `POINT()` reading palette indices straight from the
  framebuffer — *the* reason the runtime stores indices, not RGBA.
- **Audio:** 12 `PLAY` MML strings (foreground & `MB` background modes).
- **Input/time:** `INKEY$`, `LINE INPUT`, `INPUT`, `TIMER`, `RANDOMIZE`,
  `SLEEP`, `BEEP`.
- **Math/strings:** `ATN`, `SIN`, `COS`, `INT`, `CINT` (banker's rounding — see
  §17), `ABS`, `RND`, `VAL`, integer divide `\` and `MOD` (operand-rounded — §17),
  `STR$`, `LEFT$`, `MID$`, `LTRIM$`, `UCASE$`, `SPACE$`, `INSTR`, `CHR$`.

**Not strictly needed for play (and currently stubbed/skipped):** the
`ON ERROR GOTO` / `RESUME` mode-detection (the transpiler can hard-select a
mode), and the `DEF SEG`/`PEEK`/`POKE` NumLock fiddling (BIOS-specific, safe
to no-op).

**Platform note (macOS 14+):** `GetNum#` uses `INKEY$` to read angle/velocity
keystrokes. On macOS 14+, `NSView.keyDown:` passes events through
`interpretKeyEvents:` which NSBeeps for printable characters and never reaches
the window's `key_callback`. The fix is in the vendored minifb source:
`OSXWindowFrameView.m` overrides `keyDown:`/`keyUp:` to forward directly to the
`OSXWindow`, bypassing `interpretKeyEvents:` entirely. Without this patch digit
and letter keys in `GetNum#` and `GetInputs` beep and are silently dropped.

---

## 17. Integer arithmetic semantics — `CINT`, `\`, `MOD` (transpiler correctness)

QBasic's integer operators **round their operands to integers before
operating**, and that rounding is **banker's rounding** (round half to *even*),
not the C/Rust "round half away from zero". Gorillas relies on this in spots
where getting it wrong yields *subtly wrong graphics rather than a crash*, so
the runtime must match QB exactly. The runtime maps `CINT`→`qb_cint` (banker's),
`\`→`qb_idiv`, and `MOD`→`qb_mod`; the latter two `qb_cint`-round both operands
before the i64 op. (See `docs/ARCHITECTURE.md §Arithmetic operators`.)

The two that actually matter for fidelity:

- **`Scl(n!)` — the CGA/EGA coordinate scaler (lines 1036–1047).** Both branches
  return `CINT(...)` (`CINT(n! / 2 + .1)` for CGA, `CINT(n!)` for EGA). With the
  wrong rounding mode, scaled sprite/arc/coordinate values land a pixel off on
  exact `.5` boundaries. `Scl` is called all over the rendering code, so this
  has to be banker's-correct.

- **`rot = (t# * 10) MOD 4` — banana rotation (line 992).** `t#` is the
  floating-point trajectory time (stepped by `0.1`, so `t# * 10` is *not* an
  exact integer in f64). QB evaluates this as `CINT(t# * 10) MOD 4`, producing a
  clean `0–3` that `DrawBan`'s `SELECT CASE r` uses to pick the L/D/U/R rotation
  sprite. If `MOD` were applied to the raw float (Rust `%`), `rot` would be
  fractional (`2.9999…`) and the rotation index would mis-select — the spinning
  banana would render wrong. `qb_mod` rounds operands first, so it's correct.

Other `\`/`MOD` sites (all integer operands in practice, but routed through the
same helpers): screen centering (`MaxCol \ 2`, `ScrWidth \ 2`), CGA height adjust
(`NewHt * 20 \ 35`), wind-arrow length (`Wind * 3 * (ScrWidth \ 320)`), explosion
ring colour (`i MOD 2 + 1`), `Radius = Mode MOD 7`, and the sparkle-border colour
cycle (`(A + b) MOD 5`).

> **Note:** Gorillas declares all shared state with module-level `DIM SHARED`
> (§3) and never uses a bare `SHARED` statement inside a SUB. It is therefore
> unaffected by the analyzer's "promote a non-`DIM`'d `SHARED` variable to
> `GameState`" path (which exists for programs like `mandel.bas`).
