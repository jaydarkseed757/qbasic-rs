# MARIO.BAS ("MEGA WORLD") — How It Works

A 2,139-line SCREEN 13 flip-screen platformer in QBasic 1.1 style: three
themed worlds of four rooms each, sprite actors with true transparency,
quarter-pixel fixed-point physics, a shrinking boss, and a persistent high
score. It is the newest and (by real-time gameplay complexity) most demanding
bundled program, and — like every big program before it — it paid its way by
surfacing transpiler/runtime gaps nothing else had touched (§8).

---

## 1. What the program does

Title screen (big Mario + goomba, blinking PRESS SPACE) → three worlds
(overworld, underground, castle — same drawing code, different palette
targets) × four rooms each, chained flip-screen style by walking off the
right edge. Coins, bumpable `?` blocks, patrolling goombas (stompable) and
spinies (not stompable), moving platforms, a MEGA GOOMBA boss in the last
room of world 3 (three stomps, shrinking a sprite size per hit — its size IS
its health bar), a flagpole finale per world, ten lives, and a high score
persisted to `MARIOHI.DAT`.

## 2. Rooms and worlds: `WORLD<n>.TXT` with DATA fallback

Each world's four rooms are ASCII tile maps. `LoadWorld(w)` parses all four
rooms of world `w` once per world entry — from `WORLD1.TXT`/`WORLD2.TXT`/
`WORLD3.TXT` when present (easy level editing in a text editor), else from
matching `DATA` fallbacks embedded in the source — into shared room-cache
arrays. `LoadScreen(s)` then just copies room `s` out of the cache. Only
indices 1–4 of the palette (sky/brick/mortar/highlight) change per world
(`SetPalette`), so a "night sky" or "lava castle" theme costs zero drawing
code — actors use shared indices 5–12 and read the same everywhere.

## 3. Sprites: ASCII art → GET color+mask pairs

Sprite art lives as `DATA` strings (9 sprites × 16 rows × 16 characters, one
letter per color: `R`ed, `S`kin, `B`rown, `O`veralls, `G`oomba, `W`hite,
`D`ark feet, `C`oin gold, `.` transparent). At startup each sprite is decoded
with `PSET`, captured with `GET` twice — once as the color image, once as an
AND-mask (255 = transparent, 0 = opaque) — into one big packed `spr()` array
at computed element offsets (`spr(f * 260)` color, `spr(f * 260 + 130)`
mask). Drawing an actor over the busy background is always the two-step
masked blit:

```basic
PUT (x, y), spr(f * 260 + 130), AND    ' carve a transparent hole
PUT (x, y), spr(f * 260), OR           ' stamp the colors into it
```

The boss gets its own `bspr()` array holding pre-scaled 48×48 and 32×32
goomba frames (built once by `BuildBossSprites`); the final 16-px phase
reuses the normal `spr()` frames.

## 4. Physics: quarter-pixel fixed point

Everything is `DEFINT`, so smooth jumps use quarter-pixels: `feetQ` (player
feet × 4) and `vyQ` (velocity in qpx/frame). Jump impulse `vyQ = -18`,
gravity `vyQ = vyQ + 1` per frame; head-bonk detection against block
undersides uses `prevFeetQ` vs `feetQ` crossing tests, and landing snaps
`feetQ = platY * 4`. Moving platforms carry the player (`standingOn` +
`dyMov * 4`). Falling past the bottom (`feetQ \ 4 > 200`) costs a life and
respawns at the room entry point.

## 5. Input: raw scancodes from port `&H60`

Real-time platforming needs *held-key state*, not keypress events, so
`PollKeys` (reused verbatim from PIN.BAS) reads the keyboard data port
directly each frame:

```basic
sc = INP(&H60)
IF sc < 128 THEN kd(sc) = 1 ELSE kd(sc - 128) = 0
```

Make codes (< 128) mark a key down, break codes (+128) mark it up, giving a
true `kd()` keyboard-state array (space 57, arrows 72/75/77/80, ESC 1).
`ReadInput` also drains one `INKEY$` per frame so the BIOS buffer never
overflows and beeps. Menu screens use the release-then-press "armed" idiom
so a key held from the previous screen can't skip a prompt. The source
comments document a subtle real-hardware lesson: gating the poll on the 8042
status port (`&H64`) does NOT work — BIOS's own IRQ1 handler clears that bit
within microseconds — while the data port holds its last scancode
persistently, so an unconditional read every frame is correct.

## 6. Frame loop and drawing

Classic dirty-rect rendering at vsync pace: the `WAIT &H3DA, 8, 8` /
`WAIT &H3DA, 8` double-wait pair paces every loop to the 60 Hz retrace, and
nothing ever repaints the whole screen mid-game — actors are erased by
redrawing the background rectangle under their old position (`EraseRect`,
which re-renders any tiles that intersect it), then re-blitted at the new
one. The HUD (`DrawHUD`) draws on text row 25 with `LOCATE`+`PRINT …;` and
STATIC old-value caching so it only redraws a field whose backing value
changed. Big text (title, world cards) is drawn by the shared ROM-font
scaler: `DEF SEG = &HF000` + `PEEK` of the BIOS 8×8 font, one filled `LINE
…,BF` box per set bit.

## 7. Persistence and polish

- `MARIOHI.DAT` — one INTEGER high score via binary `GET #`/`PUT #`.
- `AddScore` caps at 30,000: score is a 16-bit INTEGER and screen-hopping
  for points could otherwise overflow it.
- Palette fade in/out per screen (`FadeIn`/`FadeOut` ramp the DAC toward the
  `SetPalette` targets), coin-pop visual above bumped blocks, stomp
  squish frames, post-hit invulnerability flicker (skip drawing every few
  frames), and `PLAY "MB…"` background stingers (`Blip`) that never stall
  the 60 fps loop.

## 8. Why it was hard: the transpiler/runtime work it drove

mario.bas failed in three distinct, deep ways — each a genuine QB-fidelity
gap that none of the other 53 bundled programs had ever exercised:

1. **`DIM t, mf, blink, armed, t$` — sigiled/sigil-less coexistence.** In QB,
   numeric `t` and string `t$` are *different variables*. The lexer strips
   sigils, so `DIM t$` and `DIM t AS STRING` used to produce identical
   `VarDecl`s, and the sigil-less-string type-recovery machinery swallowed
   the sigiled `t$` too — every use of the *numeric* title-screen frame
   counter `t` emitted as a string (rustc E0308). Fixed with a
   `VarDecl.str_sigil` flag: sigiled declarations skip the recovery
   collectors and declare under their typed name (`t_s`).

2. **`INP(&H60)` was unmodeled** — the game loaded but SPACE never started
   it, because `kd()` never saw a key (every port read returned 0). The
   runtime now maintains an XT set-1 make/break scancode stream: diffed from
   the real window's held-key set when windowed, synthesized from scripted
   keys when headless, with the port's authentic last-scancode persistence.

3. **Eager end-of-line text wrap corrupted the game permanently.** Once the
   coin HUD hit two digits, `LOCATE 25, 34: PRINT "C:"; coinCt; " ";` ends
   exactly at column 40 of row 25 — and our runtime wrapped the cursor
   eagerly, scrolling the whole framebuffer up 8 px. In a dirty-rect game
   nothing repaints the full screen, so every erase thereafter missed by
   8 px: floating enemies, stacked score text, debris everywhere, forever.
   Real QB *defers* the wrap (pending-cursor semantics; a `LOCATE` clears
   it), which is exactly why this classic bottom-row HUD idiom is safe on
   DOS. The runtime now defers identically.

A fourth fix (the `emit_main`/`emit_gosub_fn` per-scope DIM bookkeeping
reset) landed in the same batch and was a prerequisite for #1's collector
changes behaving predictably in main-body scopes.

## 9. Verifying it headlessly

The scancode model's headless side makes the game drivable by script — a
scripted key is consumed by `INKEY$` and feeds the port model a make+break
pair, so the armed-press title check passes:

```bash
cd basic-src   # WORLD<n>.TXT are cwd-relative
QBC_HEADLESS=1 QBC_SEED=42 QBC_KEYS="SPACE" \
  QBC_DUMP=frame.ppm QBC_DUMP_AT=presents:500 QBC_EXIT_AFTER=presents:550 \
  ../bin/mario
python3 ../tools/ppm2png.py frame.ppm frame.png   # → "MEGA WORLD 1-1" in-game frame
```

Full gameplay (precise jumps) needs real held keys — verify by playing
windowed. Everything up to and including room rendering, actor animation,
and HUD updates has been confirmed headlessly.

## 10. QB features this program exercises (transpiler checklist)

- ✅ `INP(&H60)` raw keyboard scancode polling (the only bundled program
  besides pin.bas's flippers to need true held-key state)
- ✅ `WAIT &H3DA` vsync double-wait pacing + `vsync_paced` composition
- ✅ `GET`/`PUT` packed-array element offsets (`spr(f * 260)`) with
  `AND`/`OR` masked-blit transparency, mode-13 chunky layout
- ✅ Sigiled/sigil-less same-base-name variable coexistence (`DIM t, t$`)
- ✅ Bottom-row `LOCATE`+`PRINT …;` HUD (deferred wrap; must never scroll)
- ✅ ROM-font bigtext (`DEF SEG = &HF000` + `PEEK`)
- ✅ VGA DAC palette fades (`OUT &H3C8/&H3C9`) with per-world themes
- ✅ Binary file I/O: `WORLD<n>.TXT` room data (with `DATA` fallback) +
  `MARIOHI.DAT` high-score record
- ✅ `STATIC` locals (DrawHUD's change-detection caches)
- ✅ `DEFINT A-Z` + quarter-pixel fixed-point arithmetic throughout
- ✅ `PLAY "MB…"` non-blocking background stingers
