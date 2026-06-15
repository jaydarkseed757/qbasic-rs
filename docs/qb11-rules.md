# Writing QBasic 1.1–compatible BASIC

A drop-in prompt/rule-set for any project whose `.bas` files must load and run in
**Microsoft QBasic 1.1** (the `QBASIC.EXE` shipped with MS-DOS 5/6), e.g. under
DOSBox-X. Paste the cheat-sheet into small prompts, or the full guide into a project
`CLAUDE.md` / system prompt.

QBasic 1.1 is a strict *subset* of QuickBASIC 4.5. Most "looks like QBasic" code that
fails came from a newer dialect (QB 4.5, QB64, PDS/VB-DOS, GW-BASIC). When unsure
whether a feature exists in 1.1, don't use it.

---

## Cheat-sheet (paste this into smaller prompts)

```
Target = Microsoft QBasic 1.1 (DOS), NOT QuickBASIC 4.5 / QB64 / GW-BASIC. Rules:
- No "_" anywhere: not in names (STATE_X -> STATEX) and not as line continuation.
- Don't name vars/params after built-ins/keywords: VAL POS TIMER DATE TIME PLAY
  LEFT RIGHT MID STR LEN TAB INPUT ERROR SCREEN COLOR LINE POINT KEY SOUND. (FN... is reserved.)
- DIM only at the top of a procedure/module, never in a loop or a multiply-GOSUB'd
  routine (re-running a DIM raises "Duplicate definition"). REDIM only to resize arrays.
- FUNCTION returns use a sigil: FUNCTION Foo%() -- NOT "FUNCTION Foo() AS INTEGER".
  (Typed params like (x AS INTEGER) are fine.)
- Strings need a $ sigil or DIM x AS STRING. Use LONG for numbers > 32767.
- PRINT USING: only "#" is a digit; a literal "0" in the mask prints as "0". Use "######".
- COLOR arg count depends on SCREEN mode (wrong count = "Illegal function call"):
  SCREEN 13/12 -> COLOR fg only; SCREEN 0 -> fg,bg,border; SCREEN 7-10 -> fg,bg.
- LOCATE/draw must fit the mode: SCREEN 13 = 40x25 text, coords 0-319 x 0-199.
- Save ASCII + CRLF line endings; ASCII/CP437 only (no UTF-8 box-drawing/curly quotes).
- Verify by loading in real QBASIC.EXE under DOSBox, not just an emulator/transpiler.
```

---

## Full guide

### Hard rules — these CAUSE ERRORS in QBasic 1.1

1. **No underscores in identifiers or labels.** `_` is not a valid identifier
   character and is not a line-continuation in 1.1. `STATE_TITLE`, `Sub_Draw1`,
   `INV_COLS` → run-together names: `STATETITLE`, `SubDraw1`, `INVCOLS`. (A label like
   `Sub_X` also makes the parser see the reserved word `SUB`.)

2. **No `_` line continuation.** That's a QB4.5 feature. Keep each statement on one
   physical line (up to 255 chars), or split with `:`.

3. **Don't name variables/params after built-in functions or keywords.** Common
   offenders: `VAL`, `POS`, `TIMER`, `DATE`, `TIME`, `PLAY`, `LEFT`, `RIGHT`, `MID`,
   `STR`, `LEN`, `TAB`, `SPC`, `INPUT`, `OUTPUT`, `ERROR`, `SCREEN`, `COLOR`, `LINE`,
   `POINT`, `WIDTH`, `KEY`, `SOUND`, `BEEP`. The symptom is often a misleading
   "Expected: ..." parse error. Rename (`val`→`pips`, `pos`→`charPos`).

4. **Don't reuse the `FN` prefix for ordinary names** — it's reserved for `DEF FN`
   user functions. `fNum`→`fileN`.

5. **`DIM` reached more than once raises "Duplicate definition."** GOSUB routines
   share the module variable scope, so a `DIM` inside a routine that is GOSUB'd again
   re-executes and errors (arrays always; treat scalars the same). **Declare every
   local at the top of its procedure (or module) so each `DIM` runs exactly once.**
   Never put `DIM` in a loop or a re-entered routine. Use `REDIM` only to resize arrays.

6. **`FUNCTION Foo () AS INTEGER` (typed return) is not supported.** Use the sigil
   form: `FUNCTION Foo% ()`. Typed *parameters* like `(x AS INTEGER)` ARE fine.

7. **String variables need a `$` sigil or `DIM x AS STRING`.** A bare name defaults to
   SINGLE; assigning a string to it is a type error.

8. **`PRINT USING` has no `0` digit placeholder.** Only `#` is a digit position; a `0`
   in the mask is a literal character (`"#####0"` prints a number then a literal `0`).
   Use `"######"`. (`.` `,` `+` `-` `$$` `**` `^^^^` are the other valid tokens.)

9. **Numeric ranges.** `INTEGER` is −32768..32767, `LONG` is ±2.1e9. Use `LONG` for
   scores/counters that can exceed 32767 or you'll get "Overflow".

10. **`ON ERROR GOTO` targets must be module-level labels**, not labels inside a
    SUB/FUNCTION. Often the handler is unnecessary — prefer an explicit check.

### Screen modes (choose SCREEN 13 / 12 / 9 deliberately)

- **`COLOR` argument count is mode-specific**; a wrong count is a runtime "Illegal
  function call":
  - SCREEN 13 (MCGA 320×200×256): `COLOR fg` — **foreground only** (0–255).
  - SCREEN 12 (VGA 640×480×16): `COLOR fg` — foreground only.
  - SCREEN 0 (text): `COLOR fg[, bg[, border]]`.
  - SCREEN 1 (CGA): `COLOR bg[, palette]`.
  - SCREEN 7/8/9/10 (EGA): `COLOR fg[, bg]`.
- **Text grid is mode-specific** — `LOCATE row, col` must fit it: SCREEN 13 = **40
  cols × 25 rows**; SCREEN 0/9 = 80×25; SCREEN 12 = 80×30. (Code ported from an
  80-column assumption overflows at 40.)
- **Graphics coordinates must be on-screen** (SCREEN 13: x 0–319, y 0–199). Off-screen
  `LINE`/`PSET`/`CIRCLE`/`PAINT` endpoints can raise "Illegal function call".
- **`PALETTE n, value`** in SCREEN 13/12 takes an 18-bit DAC value
  (`red + 256*green + 65536*blue`, each channel 0–63), not an EGA color nibble.

### File format

- **Save as ASCII text with CRLF (`\r\n`) line endings.** If you generate the file
  with a script, write in binary mode and emit `\r\n` explicitly — text-mode tooling
  on macOS/Linux silently strips the CR.
- **ASCII / CP437 only.** No UTF-8 box-drawing, curly quotes, em-dashes, etc.; use
  plain ASCII (`-`, `|`, `+`) or CP437 code points via `CHR$()`.

### Things that ARE safe in 1.1

- Comments: `'` or `REM`. Multi-statement lines with `:`.
- `CONST`, `TYPE…END TYPE`, `SELECT CASE`, `DO…LOOP`, `WHILE…WEND`, `GOSUB/RETURN`,
  `DEF FN` (single- and multi-line), `SUB`/`FUNCTION` with `CALL`, typed params
  (`x AS INTEGER`), `REDIM` for dynamic arrays.
- Implicit module variables are shared across GOSUB routines — lean on that instead of
  DIMing locals inside routines.

### Before handing code back — self-check

Scan for:
- [ ] any `_` in identifiers or end-of-line continuations
- [ ] any variable/param named like a built-in (VAL/POS/TIMER/…)
- [ ] any `DIM` not at the top of its procedure/module (esp. in loops or re-entered
      GOSUB routines)
- [ ] any `FUNCTION … AS <type>` return declaration
- [ ] `COLOR` argument count vs the active SCREEN mode
- [ ] `LOCATE`/draw coordinates vs the mode's text grid and pixel resolution
- [ ] `PRINT USING` masks containing a literal `0`
- [ ] INTEGER vars that can exceed ±32767
- [ ] CRLF line endings + ASCII-only content

When possible, **load and run the program in real `QBASIC.EXE` under DOSBox/DOSBox-X**
(not just an emulator or transpiler) before declaring it done — that's ground truth.
