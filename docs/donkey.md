# DONKEY.BAS — How It Works

A line-by-line architectural walkthrough of `basic-src/donkey.bas`, the original
IBM PC "Donkey" game (© IBM Corp 1981, 1982; "Version 1.10", ~134 lines). It
shipped with PC-DOS BASICA and is one of the oldest microcomputer games. Unlike
GORILLA.BAS this is a **CGA (SCREEN 1) program built almost entirely from
line-numbered GOTO/GOSUB flow** — there are no `SUB`/`FUNCTION` definitions at
all. Line numbers below refer to the source as shipped.

This doc doubles as the record of *why donkey.bas was the program that forced
four QBasic graphics-fidelity fixes in the runtime* (see §10).

---

## 1. What the game is

You drive a Formula-1 car up a two-lane road. A donkey periodically wanders down
one of the two lanes toward you. Press the **space bar** to switch lanes and
dodge it. If you hit the donkey you crash ("BOOM!") and the **Donkey** (your
opponent, left pane) scores. If your car instead creeps all the way to the top of
the screen without crashing, the **Driver** (right pane) scores and the screen
flashes "Donkey loses!". Press **ESC** to quit. It is deliberately, famously
simple — it was a demo of PC graphics.

The screen is three columns: a left **Donkey** scoreboard pane (cyan), the center
**road** (gray with a dashed center line and two lane dividers), and a right
**Driver** scoreboard pane (cyan) with the on-screen instructions.

---

## 2. Top-level execution flow

The program is pure line-numbered BASIC: `GOTO`/`GOSUB`/`IF…THEN <line>`. There
are three phases.

```
940–1299   Title screen + PC-compatibility gate (the "IBM Personal Computer"
           splash, CGA-adapter check, BASICA check). Mostly skippable.
1300–1530  One-time setup: SCREEN 1, allocate sprite arrays, GOSUB-build the
           DONKEY (1940) and CAR (1780) sprites, hand-build the B% road strip.
1540–2250  The game: draw the static board, then the main animation loop, the
           collision/explosion sequence, and the two scoring exits.
```

```
            ┌─────────────── 1540 board setup (panes, dashes, dividers) ───────────────┐
            │                                                                            │
1670 ◄──────┤  print scores                                                              │
            │  1680  CY -= 4   (car creeps up)   ── if CY < 60 ──► 2230  "Donkey loses!" │
            │  1690  PUT car (PRESET)                                                     │
            │  1710  FOR Y … STEP 6   (donkey creeps down)                                │
            │          1730  INKEY$:  ESC→quit ;  any key→switch lane (CX = 252-CX)       │
            │          1740  PUT donkey (PSET)                                            │
            │          1750  if car lane == donkey lane AND overlap ──► 2060  "BOOM!"     │
            │          1760  PUT B% road strip (XOR)                                      │
            │  1770  NEXT ; erase donkey ; GOTO 1670                                      │
            └────────────────────────────────────────────────────────────────────────────┘
```

`SAMPLES$` (980/1000) and the whole `CHAIN "samples"` path (1299) are vestigial
DOS sampler-disk plumbing and never taken here.

---

## 3. The PC-compatibility gate (940–1299)

This block is pure 1982 hardware ceremony and is mostly inert under the
transpiler:

- **975 `DEF SEG: POKE 106,0`** — pokes BIOS scratch memory; a no-op for us.
- **1010 `KEY OFF:SCREEN 0,1:COLOR 15,0,0:WIDTH 40`** — sets 40-column text mode
  and draws the "IBM Personal Computer / DONKEY / Version 1.1O" splash using box-
  drawing glyphs (`CHR$(213)`, `STRING$(21,205)`, …).
- **1100–1150** — wait for the space bar (ESC quits to 1298).
- **1160–1290 `DEF SEG=0 : IF (PEEK(&H410) AND &H30)<>&H30 …`** — reads the BIOS
  equipment word to check for a Color/Graphics Adapter and prints "HOLD IT!
  YOU'RE NOT USING THE COLOR/GRAPHICS MONITOR ADAPTER!" if absent. The transpiler
  always has a framebuffer, so `PEEK` returns 0 and the check is harmless.
- **1291–1299 `ON ERROR GOTO … : PLAY "p16"`** — a "do you have Advanced BASIC?"
  probe. `ON ERROR`/`RESUME` are stubbed; `PLAY "p16"` is a rest.

None of this affects gameplay; the real program starts at 1300.

---

## 4. Screen setup and global state (1300–1530)

```basic
1410 COLOR 0
1420 DEFINT A-Y                 ' every un-sigiled var A..Y is INTEGER
1440 SCREEN 1, 0 : COLOR 8, 1   ' CGA 320×200, 4 colors
1450 DIM Q%(500)
1460 DIM D1%(150), D2%(150), C1%(200), C2%(200)   ' explosion sprite halves
1470 DIM DNK%(300)              ' donkey sprite
1480 GOSUB 1940                 ' build DNK%  (donkey)
1490 GOSUB 1780                 ' build CAR%  (car)
1500 CLS
1510 DIM B%(300)
1520 FOR I = 2 TO 300 : B%(I) = -16384 + 192 : NEXT
1530 B%(0) = 2 : B%(1) = 193    ' hand-built road-strip sprite header + data
```

**`SCREEN 1, 0 : COLOR 8, 1`** is the key line. In CGA SCREEN 1:
- the first `COLOR` argument (8) is the **background/border** color (EGA index 8
  = dark gray — the road);
- the second (1) selects the **CGA palette** (1 ⇒ colors 1/2/3 = cyan/magenta/
  white). So index 1 = cyan (the scoreboard panes), 3 = white (everything drawn).

`DEFINT A-Y` (note: A–**Y**, not A–Z) makes the loop/coordinate variables
integers. All numerics are still `f64` in the emitted Rust; the sigils only
affect name mangling and the random-access record format.

### The hand-built `B%` road strip (1520–1530)

`B%` is **not** captured with `GET`. The program writes raw integers directly
into the array to fabricate a 1-pixel-wide, 193-pixel-tall white strip:

```
B%(0) = 2     ' QB sprite header word 0 : x-dimension in BITS = 1 px × 2 bpp
B%(1) = 193   ' QB sprite header word 1 : y-dimension in pixels
B%(2..300) = -16384+192 = -16192 = 0xC0C0   ' 2-bpp packed pixel rows
```

This relies on QBasic's **INTEGER-array** GET/PUT byte layout (two 16-bit header
words, then CGA 2-bits-per-pixel packed data), which the runtime reproduces under
its `screen_mode == 1` sprite path — see §11.

---

## 5. The sprites — DRAW → PAINT → GET

Three sprites are built once at startup with vector `DRAW` strings, filled with
`PAINT`, then captured into integer arrays with `GET`. From then on only `PUT`
is used. This GET/PUT-is-symmetric pattern is why most of donkey works under the
transpiler even though the runtime's sprite byte-format isn't QB-authentic — GET
writes and PUT reads the *same* internal format.

### Donkey — `DNK%` (1940–2050)

```basic
1940 CLS
1950 DRAW "S08"                 ' scale 8 (→ 2 px/unit); no color verb
1960 DRAW "BM14,18"             ' move (no draw) to 14,18
1970 DRAW "M+2,-4R8M+1,-1U1M+1,+1M+2,-1"   ' body outline (relative moves)
1980 DRAW "M-1,1M+1,3M-1,1M-1,-2M-1,2"
1990 DRAW "D3L1U3M-1,1D2L1U2L3D2L1U2M-1,-1"
2000 DRAW "D3L1U5M-2,3U1"
2010 PAINT (21,14), 3           ' flood the body interior white (border = 3)
2020 PRESET (37,10) : PRESET (40,10)   ' punch the two eyes back to background
2030 PRESET (37,11) : PRESET (40,11)
2040 GET (13,0)-(45,25), DNK%   ' capture a 33×26 sprite
2050 RETURN
```

The donkey is a *filled* silhouette: outline in white (3), `PAINT (21,14),3`
floods the **interior**, two `PRESET` pixels poke the eyes. Captured background
(color 0 = gray) matches the road, so `PUT … ,PSET` leaves no visible trail.

### Car — `CAR%` (1780–1930)

```basic
1790 DRAW "S8C3"                ' scale 8, color 3 (white) — set explicitly
1800 DRAW "BM12,1r3m+1,3d2R1ND2u1r2d4l2u1l1"   ' note the N (no-advance) spurs
1810 DRAW "d7R1nd2u2r3d6l3u2l1d3m-1,1l3"
1820 DRAW "m-1,-1u3l1d2l3u6r3d2nd2r1u7l1d1l2"
1830 DRAW "u4r2d1nd2R1U2"
1840 DRAW "M+1,-3"
1850 DRAW "BD10D2R3U2M-1,-1L1M-1,1"            ' wheel-lug detail
1860 DRAW "BD3D1R1U1L1BR2R1D1L1U1"
1870 DRAW "BD2BL2D1R1U1L1BR2R1D1L1U1"
1880 DRAW "BD2BL2D1R1U1L1BR2R1D1L1U1"
1890 LINE (0,0)-(40,60), , B    ' a box AROUND the car (a paint fence)
1900 PAINT (1,1)                ' flood the EXTERIOR (between box and car) white
1910 DIM CAR%(900)
1920 GET (1,1)-(29,45), CAR%    ' capture a 29×45 sprite
1930 RETURN
```

The car uses the **opposite** trick from the donkey. It draws the car outline,
fences it with a `LINE …,B` box, then `PAINT (1,1)` floods the **exterior** white
— leaving the car's interior as background (0). When `PUT … ,PRESET` blits it,
every pixel is inverted: the white surround becomes 0 (blends with the black/gray
road) and the car's 0-interior becomes white. The result is a solid white car
silhouette with thin black detail lines. This is why the car *must* be drawn with
**PRESET**, not PSET.

Note the lowercase letters (`r3m+1,3d2…`) — DRAW commands are case-insensitive.
Note also the unterminated string literals (1810/1820 etc. have no closing
quote); QBasic treats end-of-line as the terminator.

### Explosion halves — `D1% D2% C1% C2%` (built on the fly, 2070–2110)

These are not pre-built; they are `GET` out of the live screen at collision time
(see §7) so the explosion animation can fling pieces of the *current* car and
donkey apart.

---

## 6. The board and the main loop (1540–1770)

### Static board (1540–1660)

```basic
1590 LINE (0,0)-(305,199), , B          ' outer frame
1600 LINE (6,6)-(97,195), 1, BF         ' left "Donkey" pane (cyan, filled)
1610 LINE (183,6)-(305,195), 1, BF      ' right "Driver" pane (cyan, filled)
1620 LOCATE 3,5  : PRINT "Donkey"
1630 LOCATE 3,29 : PRINT "Driver"
1631 …1636                              ' "Press Space Bar to switch lanes" etc.
1640 FOR Y=4 TO 199 STEP 20 : LINE (140,Y)-(140,Y+10) : NEXT   ' dashed center
1660 LINE (100,0)-(100,199) : LINE (180,0)-(180,199)           ' lane dividers
```

The road is the gray band between x=100 and x=180; the two driving lanes are
centered at roughly x=105 and x=147.

### The loop (1670–1770)

```basic
1670 LOCATE 5,6 : PRINT SD : LOCATE 5,31 : PRINT SM   ' SD = Donkey pane, SM = Driver pane
1680 CY = CY - 4 : IF CY < 60 THEN 2230               ' car creeps UP the screen
1690 PUT (CX,CY), CAR%, PRESET                        ' redraw car at new height
1700 DX = 105 + 42*INT(RND*2)                         ' pick donkey lane: 105 or 147
1710 FOR Y = (RND*-4)*8 TO 124 STEP 6                 ' donkey descends
1720   SOUND 20000, 1                                 ' inaudible engine tick
1730   A$ = INKEY$ : IF A$ = CHR$(27) THEN 1298 _
          ELSE …IF LEN(A$)>0 THEN LINE (CX,CY)-(CX+28,CY+44),0,BF : _
                CX = 252-CX : PUT (CX,CY),CAR%,PRESET : SOUND 200,1   ' switch lane
1740   IF Y => 3 THEN PUT (DX,Y), DNK%, PSET          ' draw donkey
1750   IF CX = DX AND Y+25 >= CY THEN 2060            ' collision!
1760   IF Y AND 3 THEN PUT (140,6), B%                ' animate the road dashes (XOR)
1770 NEXT : LINE (DX,124)-(DX+32,149),0,BF : GOTO 1670   ' erase donkey, repeat
```

Key mechanics:
- **The car** is drawn with `PRESET`. It creeps *up* (CY decreases by 4 each
  outer pass). Because the captured sprite's surround inverts to the road color,
  successive draws don't leave a visible trail.
- **Lane switching** (1730): pressing any non-ESC key erases the car with a black
  filled box (`LINE …,0,BF`) and mirrors its X (`CX = 252-CX`), then redraws it.
- **The donkey** is drawn with `PSET` and descends in `STEP 6` increments.
- **Collision** (1750): if the car and donkey share a lane (`CX = DX`) and they
  overlap vertically, jump to the explosion at 2060.
- **The road dashes** (1760) animate by `PUT (140,6),B%` with the **default PUT
  verb = XOR** — drawing then re-drawing toggles the dashes to create motion.

---

## 7. Collision and explosion (2060–2220)

```basic
2060 SD = SD + 1 : LOCATE 14,6 : PRINT "BOOM!"        ' Donkey scores
2070 GET (DX,Y)-(DX+16,Y+25), D1%                     ' grab left half of donkey
2090 GET (DX+17,Y)-(DX+31,Y+25), D2%                  ' right half
2100 GET (CX,CY)-(CX+14,CY+44), C1%                   ' left half of car
2110 GET (CX+15,CY)-(CX+28,CY+44), C2%                ' right half
2130 FOR P = 6 TO 0 STEP -1 : Z = 1/(2^P) : Z1 = 1-Z  ' ease-out interpolation
2140   PUT (C1X,C1Y),C1% : PUT (C2X,C1Y),C2%          ' XOR-erase old positions
2150   PUT (D1X,D1Y),D1% : PUT (D2X,D1Y),D2%
2160   …recompute positions, flinging halves toward screen edges…
2180   PUT (C1X,C1Y),C1% : PUT (C2X,C1Y),C2%          ' XOR-draw new positions
2190   PUT (D1X,D1Y),D1% : PUT (D2X,D1Y),D2%
2200   SOUND 37+RND*200, 4 : NEXT                     ' crash noise
2210 FOR Y=1 TO 2000 : NEXT                           ' pause
2220 CLS : GOTO 1540                                  ' restart the board
```

The four sprite halves are `GET` from the live screen, then animated apart with
the **default XOR PUT** (draw-then-erase) so the pieces fly without leaving
trails — `Z = 1/2^P` gives an ease-out as `P` counts down 6→0.

The other exit, **2230 "Donkey loses!"** (reached from 1680 when the car drives
off the top, `CY < 60`), bumps `SM`, pauses, and `GOTO 1540` to restart.

---

## 8. Data types and arrays

There are **no user-defined `TYPE`s** and no `SUB`/`FUNCTION`s in donkey — it is
pure GOTO/GOSUB BASIC. The only "structures" are the integer sprite arrays:

| Array              | Built by | Size           | Holds                         |
|--------------------|----------|----------------|-------------------------------|
| `DNK%(300)`        | DRAW+GET | 33×26          | the donkey                    |
| `CAR%(900)`        | DRAW+GET | 29×45          | the race car                  |
| `B%(300)`          | by hand  | 1×193          | the scrolling road dash strip |
| `D1% D2% C1% C2%`  | GET      | car/donkey halves | explosion fragments        |
| `Q%(500)`          | —        | —              | declared, never used          |

`SD` (left **Donkey** pane, ++ on a `BOOM!` collision at 2060) and `SM` (right
**Driver** pane, ++ on "Donkey loses!" at 2230 when the car reaches the top) are
the two scoreboard counters, both printed at 1670. `CX,CY` track the car;
`DX,Y` track the donkey.

---

## 9. Control flow: GOTO state machine

Because donkey is line-numbered with `GOTO`/`IF…THEN <line>`, the transpiler
lowers it via the **GOTO state-machine fallback** (CLAUDE.md design decision #7):
each numbered line becomes a `match __pc { … }` arm inside `loop { }`, falling
through to the next arm or setting `__pc` on a jump. The two sprite builders
(1780, 1940) are `GOSUB` targets reached by `GOSUB`/`RETURN`. This is the
opposite of GORILLA.BAS, which is entirely structured (no GOTO).

---

## 10. Why donkey forced four runtime fixes

donkey is the transpiler's hardest CGA/graphics test. Getting it pixel-correct
exposed four genuine QBasic-fidelity bugs in the runtime, each now fixed and
regression-tested in `sprite_tests` (runtime). All four are QB-faithful and leave
GORILLA.BAS (EGA, explicit verbs only) untouched.

1. **PUT action verbs.** QBasic's `PUT (x,y),arr[,verb]` supports
   `PSET/PRESET/AND/OR/XOR`, and the **default verb when none is written is
   `XOR`**, not PSET. The transpiler had collapsed everything to PSET. donkey
   needs all three behaviors: the car (`PRESET`), the donkey (`PSET`), and the
   road/explosion (bare `PUT` = `XOR`). `PRESET` inverts within the mode's pixel
   depth (CGA = 2-bit, so `!c & 3`).

2. **DRAW `M x,y` relativity.** A leading sign on the **X** coordinate makes the
   *whole* move relative ("if x is preceded by + or −, x **and** y are added to
   the current position"). The runtime had decided each axis independently, so a
   move like `M-1,1` (signed x, bare y — all over the donkey outline) wrongly
   treated Y as absolute, shattering the outline so `PAINT` flooded the region
   → the donkey rendered as a solid white box.

3. **DRAW default color follows `COLOR`.** A `DRAW` string with no `C` verb paints
   in the current `COLOR` foreground. The runtime only seeded the DRAW color in
   `SCREEN`, so after `COLOR 8,1` it went stale — the donkey's uncoloured
   `DRAW "S08"` outline drew in the old default while `PAINT (21,14),3` looked for
   border color 3 → mismatch → flood leak. (The car was spared because it sets
   color inline via `DRAW "S8C3"`.)

4. **DRAW `N` (no-advance) modifier.** `N` draws a spur but must leave the cursor
   where it started. The runtime's `line()` advances the cursor internally, so the
   old `if !no_adv { advance }` guard was a no-op — every `ND2` spur drifted the
   cursor, misplacing later segments so the car silhouette never closed. `PAINT
   (1,1)` then flooded the car body, and PRESET inverted it to a few fragments
   instead of a car. (The donkey was spared because it uses no `N` commands.)

The debugging method that found these: replay each sprite's exact `DRAW`/`PAINT`
sequence in a headless `Runtime` and dump the framebuffer region as ASCII, then
(for #4) log every line segment the DRAW parser emits.

---

## 11. The `B%` road strip and the CGA sprite format

The hand-built `B%` (1520–1530) relies on QuickBASIC's **CGA INTEGER-array**
sprite byte layout: two 16-bit header words (`B%(0)` = width in bits = 1 px × 2
bpp = 2; `B%(1)` = 193 px tall) followed by 2-bits-per-pixel packed rows
(`0xC0C0` per element → each row a single white pixel). The runtime implements
this layout under a `screen_mode == 1` branch in `get_sprite`/`put_sprite`, so
`PUT (140,6),B%` blits a 1×193 white column that XORs the center line each
qualifying frame and the **dashed center-line scrolls** as on real hardware.

Every other mode keeps the EGA 4-plane planar layout (gorilla/step on SCREEN 9
are byte-identical), and donkey's GET-captured sprites (`CAR% DNK% D1%…`)
round-trip through the same CGA layout. The one remaining unhandled case is
SCREEN 2 (CGA 640×200, 1-bpp) sprites — no bundled program uses them.

---

## 12. QB features this program exercises (transpiler checklist)

- **CGA `SCREEN 1`** (320×200, 4 colors) with `COLOR bg, palette` semantics.
- **`DEFINT A-Y`** range default typing.
- **Line-numbered `GOTO`/`GOSUB`/`RETURN`/`IF…THEN <line>`** → GOTO state machine.
- **`DRAW`** with scale (`S`), color (`C`), absolute/relative move (`M`),
  directional moves (`U D L R`), and the `B` (blind) and `N` (no-advance)
  modifiers; lowercase commands; unterminated DRAW string literals.
- **`PAINT (x,y)[,fill[,border]]`** flood fill (both interior- and exterior-seed
  patterns).
- **`LINE (x1,y1)-(x2,y2)[,color][,B|BF]`** — outlines and filled boxes.
- **`GET (x1,y1)-(x2,y2),arr`** / **`PUT (x,y),arr[,verb]`** sprite blit with all
  five action verbs and the XOR default.
- **`PRESET (x,y)`** single-pixel reset (the donkey eyes).
- **`INKEY$`** non-blocking input; **`SOUND freq,dur`**; **`PLAY`** (rest).
- **`LOCATE`/`PRINT`** text over graphics; **`RND`**.
- Inert-but-parsed legacy: **`DEF SEG` / `POKE` / `PEEK`**, **`KEY OFF`**,
  **`ON ERROR GOTO` / `RESUME`**, **`WIDTH`**, **`CHAIN`**.

---

*See `docs/gorillas.md` for the structured-flow counterpart (EGA, SUB/FUNCTION-based).
donkey.bas is the CGA / GOTO / vector-DRAW stress test; gorilla.bas is the
EGA / structured / GET-captured-sprite target.*
