# DEMO.BAS — How It Works

An architectural walkthrough of `basic-src/demo.bas`, the transpiler's flagship
demoscene "mega demo": 15 back-to-back scenes in SCREEN 13 (320×200, 256
colors), all written in the classic DOS-demoscene style — draw straight into
VGA memory with `POKE`, sync to the vertical retrace with `WAIT`, animate
mostly through palette DAC writes rather than pixel writes, and cache
expensive-to-compute patterns to disk with `BSAVE`/`BLOAD`. Unlike GORILLA,
TORUS, or DONKEY it isn't a game — it's a showcase, and it's the program that
drove the most transpiler/runtime feature work of any bundled `.bas` file
(`DEF SEG`, segment-aware `POKE`/`PEEK`, `BSAVE`, real `WAIT` vsync,
`vsync_paced` frame composition — see §7).

---

## 1. What the program does

Run it and you get an ascending 3-tone `PLAY` chime, then 15 scenes play back
to back, each fading in from black, holding for up to 600 frames (10 seconds
at 60 fps) or until a key is pressed, then fading to black before the next
scene begins. The final scene is always a Star-Wars-style credits crawl that
ends the program.

```
PLAY "O4 L4 C E G"        intro jingle

Scene1   Scrolling starfield (POKE erase/draw)
Scene2   Title card: "JAY'S QBASIC MEGA DEMO", rainbow letters + twinkle stars
Scene3   Rotating 3D wireframe cube
Scene4   Plasma (palette-cycled, cached to PLASMA.DAT)
Scene5   Shadebobs — 4 glowing blobs, additive blend via bit-field PUT
Scene11  Dot sphere — 82-dot wireframe globe, wobbling tilt axis
Scene6   Copper bars — 6 moving raster bars, "COPPER BARS" text overlay
Scene7   Tunnel — flying through rings (cached to TUNNEL.DAT)
Scene14  Rotozoomer — 64×64 checkerboard rotating + breathing zoom
Scene9   Vector morph — wireframe melting between cube/pyramid/gem/star
Scene10  Starship flight — 2D ship banking through a 3D starfield
Scene13  Death Star trench run — one-point-perspective trench + X-wing
Scene15  Platformer vignette — "MEGA WORLD 1-1", a Mario homage
Scene12  Wavy sine scroller — giant gold text bobbing past two pillars
Scene8   Credits crawl (ALWAYS last) — Star Wars-style scroll, ends the demo
END
```

The dispatch order in the source doesn't match numeric scene order — scenes
were added over time and slotted in wherever they fit the pacing best. The
source comment above the `CALL` list documents the rule: *Scene8 is reserved
for the finale; add new scenes above it.*

---

## 2. The animation model: draw once, animate the palette

The single idea threaded through almost every scene is **palette-cycling
animation**. VGA's DAC (digital-to-analog converter) maps each of the 256
framebuffer index values to an RGB color via 3 registers written through I/O
ports `&H3C8` (write index) and `&H3C9` (R, then G, then B — auto-incrementing
index after each triplet):

```basic
OUT &H3C8, 1              ' select palette entry 1
OUT &H3C9, 20              ' R
OUT &H3C9, 20              ' G
OUT &H3C9, 20              ' B (index auto-advances to 2)
```

If a scene's *shape* never changes — only its *color* — you can draw it once
and then animate purely by rewriting DAC entries, touching zero pixels per
frame:

- **Scene6 (copper bars)**: every screen row `y` is filled once with palette
  index `y` (`LINE (0,y)-(319,y), y`, 200 rows fit in 256 entries). Moving
  bars are pure DAC math: a small lookup table maps "which bar's colored band
  covers this row *right now*" to that row's palette entry.
  the bars.
- **Scene7 (tunnel)** and **Scene14's sibling, Scene4 (plasma)**: the
  ring/plasma pattern is computed once into the framebuffer; "flying forward"
  / "cycling" is just rotating which palette index maps to which color
  (`shift = shift + 3` each frame, wrapping).
- **Scene5 (shadebobs)**: goes one step further — see §4.

Every scene also **fades in and out** by ramping DAC values from `0` up to
target and back down, always synced to `WAIT` so the ramp is exactly one step
per real frame (§3).

---

## 3. Frame pacing: `WAIT &H3DA, 8` and the double-wait idiom

Every scene's main loop starts with the same two lines:

```basic
DO
    WAIT &H3DA, 8, 8        ' block until the PREVIOUS retrace ENDS
    WAIT &H3DA, 8           ' block until the NEXT retrace STARTS
    ...
LOOP WHILE INKEY$ = "" AND ft < 600
```

`&H3DA` is the VGA Input Status Register 1; bit 3 is the vertical-retrace flag.
`WAIT port, mask[, xormask]` on real hardware spins until
`(INP(port) XOR xormask) AND mask <> 0`. The double-wait pair is the classic
DOS-demoscene idiom for "sync to exactly one frame boundary, no more, no
less": the first `WAIT` (with `xormask=8`) waits for the *end* of whatever
retrace is currently in progress (so you don't accidentally catch the tail of
one you're already inside), and the second `WAIT` (no `xormask`) then waits
for the *next* retrace to *start* — which is also the moment a real VGA card
would flip the display buffer. This gives every scene a rock-solid 60fps
frame budget with no manual delay math.

**Our runtime models this bit from the wall clock**: the retrace flag is
asserted for the last ~2ms of every `frame_interval_ms` period (default 16ms
≈ 60Hz), and completing a wait-for-retrace-start `present()`s the accumulated
frame — so the WAIT call *is* the flip point, exactly mirroring real VGA's
"draw during blank, flip at retrace" behavior. See §7 for why this had to be
more than a no-op.

Scenes exit their loop on `INKEY$ <> ""` (any key) or a frame-count ceiling
(`ft < 600`, i.e. ~10 seconds at 60fps) — nothing spins forever, which matters
for the headless test harness as well as for actually watching the thing.

---

## 4. Direct framebuffer access: `DEF SEG`, `POKE`, `GET`/`PUT`

The other half of the demoscene toolkit is bypassing QBasic's graphics
statements entirely and writing bytes straight into video memory.

```basic
DEF SEG = &HA000           ' segment &HA000 = VGA framebuffer in SCREEN 13
POKE y * 320 + x, colorIndex   ' plot one pixel: one byte write, no bounds check
...
DEF SEG                    ' bare form restores the default segment
```

In SCREEN 13's linear MCGA layout, `offset = y*320 + x` is one byte per pixel
— so a `POKE` is a single write, versus `PSET`'s coordinate-checked,
mode-dispatching graphics call. Scene1's starfield, Scene10's 3D starfield,
Scene11's dot sphere, and Scene8's credits-crawl text all draw exclusively
this way; they track each moving point's *previous* framebuffer address so
erasing is `POKE oldAddr, 0` followed by `POKE newAddr, color` — no full-frame
clear, ever.

`DrawText` (the shared text-drawing SUB used by Scene2's title, Scene6's
"COPPER BARS" label, Scene12's scroller glyphs, and Scene15's "MEGA WORLD 1-1"
banner) uses the segment trick differently — it `PEEK`s the **ROM BIOS 8×8
font** at segment `&HF000`, offset `&HFA6E + char*8`, turning each font byte
into 8 `LINE ...,BF` box draws scaled by the requested factor. This is the
classic "read glyphs straight out of ROM to draw scaled bigtext" trick.

**Scene5's shadebobs** is the cleverest use of direct pixel manipulation in
the whole demo. Each of 4 glowing blobs owns exactly **2 bits** of the pixel
byte (`shl = 1, 4, 16, 64`). Drawing a blob is `PUT sprite, OR` (sets only its
own 2 bits, leaving the other 3 blobs' bits alone even where sprites
overlap); erasing is `PUT constant-255-minus-3*shl mask, AND` (clears only its
own bits). The palette does the actual color math: entry `i`'s color is
computed once at init as the "fire ramp" color for `sum of the four 2-bit
fields packed into i` — so two overlapping blobs (bit-sum higher) really do
render as a hotter, brighter color, with **zero per-pixel BASIC arithmetic**
at animation time. It's true real-time additive blending using nothing but
bit-packing and a 256-entry palette lookup table.

**`GET`/`PUT` sprite blocks** appear throughout for anything that's drawn
once and moved: Scene5's blob sprites, Scene12's pre-rendered scroller glyphs
(each unique letter is drawn via `DrawText` once into an offscreen corner,
captured with `GET`, then blitted with `PUT` every frame after), and Scene15's
runner/goomba sprites (see §6).

---

## 5. Caching expensive patterns: `BSAVE`/`BLOAD`

Two scenes compute a pattern that's expensive on real 1990s hardware but only
needs computing **once ever**, so they cache it to disk:

```basic
' First run: compute the pattern into the framebuffer (with a progress bar),
' then BSAVE it. Later runs: BLOAD it straight back in one statement.
cached = 0
OPEN "PLASMA.DAT" FOR BINARY AS #1
IF LOF(1) = 64007 THEN cached = 1     ' 7-byte BSAVE header + 64000 pixel bytes
CLOSE #1

IF cached THEN
    DEF SEG = &HA000
    BLOAD "PLASMA.DAT", 0
    DEF SEG
ELSE
    ... compute the plasma into the framebuffer, with a live progress bar ...
    BSAVE "PLASMA.DAT", 0, 64000
END IF
```

- **Scene4 (plasma)**: `PLASMA.DAT` — a sine-interference pattern
  (`sx(x) + verticalSine + diagonalSine`, all from precomputed sine tables).
- **Scene7 (tunnel)**: `TUNNEL.DAT` — a polar `SQR`/`ATN` ring pattern,
  computed for one quadrant and mirrored to the other three (4-fold
  symmetry cuts the float math to a quarter).

Both first-run computations draw a live white-fill-on-grey progress bar
*while* computing, using the same POKE-into-framebuffer technique — the bar's
row offsets are precomputed once so advancing it never costs a `LONG`
multiply per pixel.

---

## 6. Scene-by-scene notes worth knowing

**Scene1 — Starfield.** 100 stars, each just an `(x,y,z)` where `z` is speed
(1–3 px/frame); wrap to a fresh random `y` at the right edge. Palette-fade
in/out only, POKE erase/draw.

**Scene2 — Title card.** Draws "JAY'S QBASIC" / "MEGA DEMO" via `DrawText` in
7 rainbow colors (one palette entry per character-color, cycled `MOD 7`),
plus 6 independently-phased twinkling stars animated purely through their own
palette entries (a 256-step sine-derived brightness LUT). Both letters and
stars are invisible while palette entry is black, so the whole title "appears"
via a single 63-frame DAC fade.

**Scene3 — Wireframe cube.** Classic fixed-point 3D: an 8-vertex, 12-edge
cube rotated with a 256-step sine/cosine LUT (`cos(a) = sinT((a+64) AND 255)`,
i.e. reading the same table a quarter-period ahead), then perspective-divided
(`focal=200`, cube center at `z=300`) into screen coordinates. Erase is
redrawing last frame's edges in color 0 before computing this frame's.

**Scene9 — Vector morph.** Reuses Scene3's exact rotation/projection math on
**interpolated** vertices. All four target shapes (cube, pyramid, gem,
antiprism star) share the same 8-vertex/12-edge topology — a pyramid is
"a cube whose back 4 vertices all collapsed onto one point," so the zero-length
edges are simply invisible. A small state machine (`morphing` flag + a
64-frame linear blend, `holdT` between morphs) cycles through all 4 shapes.

**Scene10 — Starship flight** and **Scene13 — Trench run** both combine a
Scene1-style POKE'd 3D/perspective starfield or converging-line trench with a
Scene3-style erase-and-redraw `LINE`-segment ship. Scene10's ship banks by
rotating its local-coordinate polygon through the *velocity* direction of its
Lissajous flight path (sine's derivative is cosine — free, since it's just
reading the sine LUT a quarter-period ahead again). Scene13's X-wing fires
laser bolts that interpolate from wingtip to the vanishing point over 18
sub-frames.

**Scene11 — Dot sphere.** 82 dots (8 latitude rings × 10 + 2 poles) on a
tilted, slowly-wobbling axis. Each dot's own orbital phase is independent, so
"spinning the globe" is two sine lookups per dot rather than a 3D rotation
matrix — deliberately the cheapest possible spinning-3D-object trick in the
whole demo.

**Scene14 — Rotozoomer.** True inverse-mapped affine texture sampling: the
16×20-block sample grid steps through **fixed-point** texture-space
coordinates (8192 units = 64 texels), so each block-to-block step is two
adds and an `AND 8191` wrap — no multiply, divide, or `LONG` in the hot loop.
A `prev()` dirty-block cache skips redrawing any block whose sampled color
didn't change since last frame (the checkerboard texture is mostly flat, so
this saves real work).

**Scene12 — Wavy sine scroller.** Message text glides right-to-left, each
character's height computed live from a travelling sine wave
(`y = sinT((charX*2 + waveTime) AND 255)`). Every unique glyph in the message
is pre-rendered exactly once (via `DrawText` + `GET` into an offscreen
sprite slot); per-frame work per visible character is one `LINE...,BF` erase
box plus one `PUT`. Two decorative pillars are redrawn every frame *on top*
of the spawn/retire zones — this scene is the one that originally exposed the
`vsync_paced` composition bug (§7): letters must finish sliding behind the
pillars *within* one simulated frame, never visible mid-composition.

**Scene15 — Platformer vignette.** The most elaborate single scene. Sprite
pixel art lives as `DATA` strings after `SpriteData:` (4 sprites × 16 rows ×
16 characters, one letter per pixel color — `R`=red, `S`=skin, `B`=brown,
`O`=overalls, `G`=goomba body, `W`=white, `D`=goomba feet, `.`=transparent).
At init, each sprite is decoded character-by-character with `SELECT CASE` +
`PSET`, then captured twice with `GET`: once as the opaque draw sprite, once
as an `AND`-mask (255=transparent, 0=opaque) built by re-drawing the same
shape in white-255 on a 255-filled box. Drawing an actor is always the same
two-step: `PUT ..., mask, AND` (carves a transparent hole in the background)
then `PUT ..., sprite, OR` (stamps the real colors into that hole) — true
sprite transparency over a busy background, the same masking idea as
Scene5's shadebobs but for full-color sprites instead of packed bit-fields.
The runner's jump physics run in **quarter-pixel fixed point**
(`feetQ = feetQ + vyQ`, gravity `vyQ = vyQ + 1`) so falls look smooth despite
everything being `DEFINT`. Jump timing is scripted by X position windows
(`IF fy = 176 AND cx >= 60 AND cx <= 72 THEN ... vyQ = -18`), not physics or
input — this is a canned vignette, not a playable level.

---

## 7. Why this program was hard: the transpiler/runtime work it drove

Unlike GORILLA or TORUS (each fixed in one concentrated session), demo.bas
was built incrementally over several milestones, and each new scene tended to
lean on a QBasic feature the transpiler hadn't needed yet:

1. **`WAIT` was unmodeled entirely at first.** It lexed as an unrecognized
   user-SUB call, emitting an undefined `wait(port, mask)` — 11 such calls
   broke the very first version's compile. Fixed by adding `Token::Wait` →
   `Stmt::Wait` → `__rt.qb_wait(port, mask, xormask)`.

2. **`DEF SEG` was a no-op that silently dropped its argument.** Early on,
   the entire `DEF SEG [= expr]` line was skipped to end-of-line — meaning
   every `POKE`/`PEEK` in the demo always hit the same simulated
   `HashMap<u32,u8>` memory map regardless of what segment the program
   thought it had selected, so nothing ever actually touched the screen or
   the ROM font. Fixed by making `DEF SEG` a real statement with a `def_seg`
   register on `Runtime`, and making `POKE`/`PEEK` segment-aware: `&HA000` in
   SCREEN 13 routes to the linear framebuffer; `&HF000` routes `PEEK`s in the
   font's address range to the runtime's own `FONT_8X8` table; everything
   else still uses the old memory map (so `pokeit.bas`/`evil.bas` were
   unaffected).

3. **`BSAVE` didn't exist** (only `BLOAD` did, from an earlier program). The
   plasma/tunnel caching scenes need the exact mirror operation — `qb_bsave`
   writes the same 7-byte header format `BLOAD` reads, sourced from the
   `&HA000` framebuffer.

4. **The vsync-composition bug** (found by testing on real hardware, not
   headlessly): Scene12's wavy scroller showed letters clipping incorrectly
   against its pillars. Root cause was a painter's-algorithm assumption our
   runtime broke — real QB composes an entire frame (erase → move → PUT
   sprites → redraw overlays) *between* vertical retraces, so intermediate
   states are physically never visible on real hardware. Our runtime,
   though, was presenting mid-composition: `put_sprite` always blits
   immediately (needed elsewhere, so gorilla's banana-throw animation stays
   visible), and the frame-rate throttle can also fire between statements.
   Fixed with a `vsync_paced: bool` on `Runtime`, set the moment a program
   completes a `WAIT &H3DA, 8` and cleared on the next `SCREEN` call — while
   set, both of those mid-frame presents are suppressed, and the `WAIT`'s own
   `present()` becomes the *only* flip point. Verified not to affect
   gorilla/donkey (neither calls `WAIT`) via unchanged golden checksums.

5. **`SYSTEM` and `PRINT #n, USING`** were needed by `basic-src/bench.bas` —
   the companion benchmark program (see below) — not by demo.bas directly,
   but landed in the same milestone.

6. **Scene5 (shadebobs) itself was disabled for a full milestone.** The
   source comment used to read *"too slow in interpreter; revisit with CALL
   ABSOLUTE"* — real QBasic 1.1 on period hardware genuinely can't afford a
   busy per-pixel `PEEK`+`POKE` read-modify-write loop. `bench.bas` (which
   measures exactly this operation under real DOSBox-X vs. native Rust) put
   a number on it: ~73,000 ops/sec on real 486-class hardware vs. ~8,000,000
   ops/sec transpiled — see the root README's Performance section for the
   full table. At that native speed the constraint simply evaporates, so
   Scene5 was re-enabled using the bit-field-PUT technique described in §4.

All of this is also recorded, commit-by-commit, in `CLAUDE.md`'s changelog —
search for "DEF SEG", "vsync_paced", and "demo.bas grows to 15 scenes".

---

## 8. Verifying it headlessly

Because every scene's exit condition is `INKEY$ <> ""` (any key), the
headless `QBC_KEYS`/`QBC_DUMP`/`QBC_EXIT_AFTER` driver can script a walk
through scenes and capture a frame from any one of them — this is how each
new scene was regression-tested without a window:

```bash
QBC_HEADLESS=1 QBC_SEED=42 QBC_KEYS="a,b,c,d" \
  QBC_DUMP=frame.ppm QBC_DUMP_AT=presents:60 QBC_EXIT_AFTER=presents:65 \
  ./bin/demo
python3 tools/ppm2png.py frame.ppm frame.png
```

`presents:N` (rather than `ms:N`) is the more reliable trigger for demo.bas
specifically, since real elapsed wall-clock time before a given scene starts
varies with how long earlier scenes' init work (plasma/tunnel computation on
an uncached first run, sprite `GET` captures, etc.) takes — counting actual
`present()` calls sidesteps that entirely. One thing headless capture
*can't* prove is the windowed vsync pacing feel — that needs a real look on
a Mac.

---

## 9. QB features this program exercises (transpiler checklist)

- ✅ `WAIT port, mask[, xormask]` — real wall-clock-modeled VGA retrace sync
- ✅ `DEF SEG [= expr]` + segment-aware `POKE`/`PEEK` (video memory, ROM font)
- ✅ `BSAVE`/`BLOAD` (exact mirror pair, 7-byte header)
- ✅ `OUT`/`INP` — full VGA DAC port I/O (palette read/write via `&H3C7`-`&H3C9`)
- ✅ `GET`/`PUT` sprites, all combine verbs (`OR`, `AND`, `PSET`) — including
  bit-field packing (Scene5) and dual draw+mask blits (Scene15)
- ✅ Dynamic (variable-bound) array `DIM`, e.g. `DIM spr(sprN)` where `sprN`
  is computed at runtime — several scenes size their sprite-data array this way
- ✅ `RESTORE label` + `READ` from module-level `DATA` (Scene15's sprite art)
- ✅ `PLAY` MML (intro jingle)
- ✅ `SELECT CASE` on a single-character `MID$` (Scene15's sprite decoder)
- ✅ Fixed-point arithmetic patterns throughout (`DEFINT A-Z` everywhere;
  quarter-pixel gravity in Scene15; 8192-unit texture space in Scene14) —
  none of this needs special transpiler support since **all QB numerics are
  f64** in the emitted Rust, but it's worth knowing the demo was written
  DOS-integer-faithful regardless
- ⚠️ `ON ERROR` — not used; demo.bas assumes SCREEN 13 always succeeds

SCREEN 13 (320×200, 256-color MCGA) is the only graphics mode this program
uses; every scene calls it fresh (and `SCREEN 0` on exit), which is also
what resets the `vsync_paced` flag between scenes (§7 item 4).
