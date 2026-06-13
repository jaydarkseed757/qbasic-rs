# money.bas — Line-by-Line Architectural Walkthrough

`basic-src/money.bas` is the **QBasic Money Manager**, a Microsoft 1990 personal
finance application shipped as a sample with MS-DOS QBasic 1.1.  At 1537 lines
it is one of the larger bundled programs and exercises a distinct set of features:
FIELD-based random-access binary file I/O, CP437 box-drawing characters, DEFINT
default typing, machine-code POKE/CALL Absolute scroll routines, and a layered
pull-down menu system implemented entirely with GOSUB targets.

---

## What It Does

Money Manager lets a user maintain up to 19 named accounts (assets or
liabilities) and record transactions against each one.  It persists data across
runs in two kinds of files:

- **`money.dat`** — sequential text file; stores the active colour-scheme
  preference (`ColorPref`) and the 19 account title/type/description triples.
- **`money.1` … `money.19`** — one random-access binary file per account; each
  file uses 84-byte fixed-length records: record 1 is a header (validity stamp,
  transaction count, running balance); records 2..N+1 are transactions (date,
  reference, description, two monetary figures as packed doubles).

From the top-level menu the user can:

| Menu | Action |
|------|--------|
| File | Exit |
| Accounts | Edit account titles and descriptions |
| Transactions | Open a full-screen transaction editor for any account |
| Reports | Net Worth (screen + printer) or per-account Transaction Summary (printer) |
| Colors | Choose one of four named color schemes |

---

## Execution Flow

```
main body
  ├─ DEF SEG=0 / PEEK(1047) / POKE 1047, 0   ← disable keyboard LEDs
  ├─ ON ERROR GOTO ErrorTrap                  ← probe for money.dat
  ├─ OPEN "money.dat" FOR INPUT AS #1         ← existence check only
  ├─ CLOSE
  ├─ ON ERROR GOTO 0
  ├─ Initialize    ← READ colors, POKE assembly routines
  ├─ Intro         ← title screen with SparklePause animation
  ├─ MenuSystem    ← main event loop (never returns until user exits)
  ├─ COLOR 7,0 / CLS
  ├─ DEF SEG=0 / POKE 1047, KeyFlags          ← restore keyboard LEDs
  └─ END

ErrorTrap (CASE 53):
  └─ file not found → set defaults, SaveState, RESUME (retry OPEN)
ErrorTrap (CASE 24/25):
  └─ printer error → set PrintErr, show dialog, RESUME NEXT
```

The program never falls through `MenuSystem`; it calls `EXIT SUB` once the user
picks `File → Exit` (sets `finished = TRUE`).

---

## TYPEs and Global State

```basic
DEFINT A-Z          ' All undecorated variables are INTEGER by default

TYPE AccountType
    Title   AS STRING * 20
    AType   AS STRING * 1     ' "A" = asset, "L" = liability, "" = unused
    Desc    AS STRING * 50
END TYPE

TYPE Recordtype              ' Layout of each transaction record on disk
    Date    AS STRING * 8    '  8 bytes
    Ref     AS STRING * 10   ' 10 bytes
    Desc    AS STRING * 50   ' 50 bytes
    Fig1    AS DOUBLE        '  8 bytes (IEEE LE — increase)
    Fig2    AS DOUBLE        '  8 bytes (IEEE LE — decrease)
END TYPE                     '        = 84 bytes total (matches LEN=84)

DIM SHARED account(1 TO 19)   AS AccountType
DIM SHARED ColorPref                          ' 1–4 (integer via DEFINT)
DIM SHARED colors(0 TO 20, 1 TO 4)           ' colour attribute table
DIM SHARED ScrollUpAsm(1 TO 7)               ' machine code buffer
DIM SHARED ScrollDownAsm(1 TO 7)
DIM SHARED PrintErr AS INTEGER
```

The transpiler emits `account__title`, `account__atype`, `account__desc` as
parallel `Vec<String>` fields on `GameState` (TYPE flattening + DIM SHARED
promotion).  `colors` becomes `Vec<Vec<f64>>`.

---

## Initialisation (`Initialize`)

```
Initialize
  ├─ WIDTH , 25 / VIEW PRINT             ← full 25-line text viewport
  ├─ FOR ColorSet = 1 TO 4              ← READ 10 colour attributes × 4 schemes
  │    FOR X = 1 TO 10: READ colors(X, ColorSet)
  ├─ LoadState                          ← read money.dat
  ├─ POKE assembly bytes into ScrollUpAsm(1..7)
  └─ POKE assembly bytes into ScrollDownAsm(1..7)
```

The colour DATA rows define 10 attributes per scheme (screen background, dot
colour, menu bar fg, menu bar bg, title fg, shadow, choice, cursor fg, cursor bg,
shadow2).  `colors(0, x)` through `colors(10, x)` are indexed throughout the
display code as `colors(N, ColorPref)`.

---

## File I/O — Two Layers

### Sequential: `money.dat`

Used only by `LoadState` and `SaveState`.  Plain text, one value per line.

```basic
' SaveState writes:
PRINT #2, ColorPref          ' e.g. "2"
FOR a = 1 TO 19
    PRINT #2, account(a).Title
    PRINT #2, account(a).AType
    PRINT #2, account(a).Desc
NEXT a
```

Reading back with `INPUT #1, ColorPref` requires `.trim()` before
`.parse::<f64>()` in the transpiler because `PRINT #n, 2` emits `" 2 "` (QB
leading-space convention) and Rust's parser rejects the surrounding whitespace.
This was the root cause of the "all menus appear black" bug — `ColorPref` parsed
as `0.0`, which indexes `colors(x)(0)` (never populated) → `color(0, 0)` →
black-on-black.

### Random Access: `money.N` (FIELD-based)

Each account file uses `LEN = 84` records and a FIELD statement to map named
string buffers onto the record:

```basic
OPEN file$ FOR RANDOM AS #1 LEN = 84
FIELD #1, 8 AS IoDate$, 10 AS IoRef$, 50 AS IoDesc$, 8 AS IoFig1$, 8 AS IoFig2$
FIELD #1, 11 AS valid$, 5 AS IoMaxRecord$, 8 AS IoBalance$
```

Two overlapping FIELD declarations share the same 84-byte record buffer.  The
first view addresses individual transaction columns; the second addresses the
header fields that live in the first 24 bytes of record 1.

**Monetary values** are packed as 8-byte IEEE 754 doubles via `MKD$` / `CVD`:

```basic
LSET IoFig1$ = MKD$(amount#)   ' pack double → 8-byte Latin-1 string
amount# = CVD(IoFig1$)          ' unpack
```

The runtime encodes these as Latin-1 bytes (byte `b` → `char::from_u32(b)`) so
the in-memory Rust String carries the raw binary payload without UTF-8
corruption.  `qb_lset` / `qb_rset` use `.chars().count()` (not `.len()`) for
correct fixed-width padding.

**Record layout:**

| Record | Content |
|--------|---------|
| 1 | Header: `"THISISVALID"` (11 chars) + `IoMaxRecord$` (5 chars) + `IoBalance$` (MKD$, 8 chars) |
| 2..N+1 | Transaction: date(8) + ref(10) + desc(50) + fig1(MKD$,8) + fig2(MKD$,8) |

On first access, if record 1 does not begin with `"THISISVALID"`, the file is
initialised with a zeroed transaction at record 2 and the header at record 1.

---

## Menu System (`MenuSystem`)

The entire application UI is driven by a single re-entrant `Menu%` FUNCTION:

```basic
FUNCTION Menu (CurrChoiceX, MaxChoice, choice$(), ItemRow(), ItemCol(), help$(), BarMode)
```

`BarMode = TRUE` → horizontal menu bar at row 1.
`BarMode = FALSE` → vertical drop-down with box, shadow, and `FancyCls` wipe.

`Menu` returns the integer selection (1..MaxChoice) when `Enter` is pressed,
`-2` / `-3` for left/right arrow (used in `MenuSystem` to rotate the active
top-level menu without closing), and `0` while no selection has been confirmed.

`MenuSystem` is a single SUB containing both Rust-callable entry code and six
GOSUB targets:

```
MenuSystemMain:      ← draw background, set up bar menu, call Menu(), RETURN
MenuSystemFile:      ← File submenu (1 item: Exit)
MenuSystemEdit:      ← Accounts submenu (1 item: Edit Account Titles)
MenuSystemAccount:   ← 19 account entries, calls EditTrans(subchoice)
MenuSystemReport:    ← Net Worth + 19 per-account Transaction Summary
MenuSystemColors:    ← 4 color schemes, calls SaveState on change
```

The outer WHILE loop dispatches among them via `SELECT CASE choice`.  After each
submenu GOSUB returns, `FancyCls` wipes the dropdown area, and if `subchoice` is
`-2`/`-3` the next submenu is shown without returning to the bar.

---

## Transaction Editor (`EditTrans`)

The longest SUB (409 lines) is a full-screen spreadsheet-style editor.  It opens
`money.item` FOR RANDOM, scans all records to build a running-balance array
`Balance#(0..1000)`, then enters a DO loop handling arrow keys, F2 (save), F9
(insert), F10 (delete), and printable characters via `GetString$`.

Internal GOSUB targets within `EditTrans`:

| Target | Purpose |
|--------|---------|
| `EditTransPrintWholeScreen` | Redraw all visible transaction rows |
| `EditTransGetLine` | Load one record from disk into `CurrString$/CurrFig#` |
| `EditTransShowCursor` | Highlight active cell |
| `EditTransHideCursor` | Restore normal cell colour |
| `EditTransEditItem` | Call `GetString$`, store result, update display |
| `EditTransMoveUp/Down` | Scroll the viewport, reload balance |
| `EditTransInsertRecord` | Shift records down, write blank, recompute balances |
| `EditTransDeleteRecord` | Shift records up, truncate, recompute balances |
| `EditTransSave` | Write all dirty records; update header with new MaxRecord + balance |
| `EditTransWriteBalance` | Recompute and display the balance column |

`GetString$` is itself a FUNCTION with two internal GOSUB targets
(`GetStringShowText`, `GetStringGetKey`).

**Key editing quirk:** when `GetString$` returns a non-printable key (arrow, F2,
etc.), the key scan-code string is returned as the function result *and* stored
in `end$` (the `start$`/`end$` parameter pair acts as an in/out pair — both
passed by reference).

---

## Reports

### Net Worth Report (`NetWorthReport`)

Reads record 1 of every asset and every liability file (opening and closing each
in turn), sums the `IoBalance$` doubles, and draws a two-column table (assets
left, liabilities right) with `PRINT USING "$$###,###,###.##"` formatting.
F3 triggers `GOSUB NetWorthReportPrint` which LPRINT-formats the same data to
the line printer.

### Transaction Summary (`TransactionSummary`)

Prompts for printer confirmation, then scans all transaction records of one
account file and LPRINT-formats each row.  Uses the same `FIELD` overlay pattern
as `EditTrans`.

Both report SUBs use `ON ERROR GOTO ErrorTrap` / `PrintErr` flag to handle a
missing or offline printer gracefully (CASE 24/25 in `ErrorTrap`).

---

## CP437 Display and Box Drawing

`Box` draws a rectangle using CP437 box-drawing glyphs:

```basic
PRINT "Ú"; STRING$(BoxWidth - 2, "Ä"); "¿";  ' ┌─────┐
PRINT "³"; SPACE$(width);              "³";   ' │     │
PRINT "À"; STRING$(BoxWidth - 2, "Ä"); "Ù";  ' └─────┘
```

`FancyCls` scatters `CHR$(250)` (`·`) dot characters across the viewport for the
"sparkle wipe" transition effect.  The `Intro` title art uses CP437 block
characters (`Û`, `ß`, `Ü`) to compose a shadowed banner.

The runtime's `FONT_8X8` table was extended from 128 to 256 entries to cover
all CP437 code points (0x80–0xFF: box-drawing, block elements, and Latin
extended).  `draw_char_fb` uses the full code-point index rather than masking to
0x7F.

Because `money.bas` is saved as **UTF-8** (not Latin-1/CP437), the transpiler's
source reader tries `std::str::from_utf8()` first and falls back to byte-by-byte
decoding only for files that fail UTF-8 validation.

---

## Assembly Scroll Routines

`Initialize` POKEs 14 raw x86 bytes into each of the `ScrollUpAsm` and
`ScrollDownAsm` integer arrays using `DEF SEG = VARSEG(...)` and `POKE`:

```
ScrollUp  machine code (14 bytes):
  B8 01 06  MOV AX, 0601h    ; BIOS INT 10h, AH=06h scroll up
  B9 01 04  MOV CX, 0401h    ; top-left corner
  BA 4E 16  MOV DX, 164Eh    ; bottom-right corner
  B7 00     MOV BH, 00h      ; attribute
  CD 10     INT 10h
  CB        RETF              ; far return

ScrollDown (14 bytes):
  B8 01 07  MOV AX, 0701h    ; BIOS INT 10h, AH=07h scroll down
  (rest identical)
```

`ScrollUp` / `ScrollDown` SUBs call them via:

```basic
DEF SEG = VARSEG(ScrollUpAsm(1))
CALL Absolute(VARPTR(ScrollUpAsm(1)))
DEF SEG
```

The transpiler stubs `DEF SEG`, `VARSEG`, `VARPTR`, and `CALL Absolute` as
no-ops.  On native Rust the scroll SUBs do nothing; the visual effect is absent
but the program is otherwise correct because the transaction editor redraws the
screen explicitly rather than relying on scroll output.

---

## Colour Scheme System

Four colour schemes are encoded as DATA:

```
'  scrn  dots  bar  back  title shdow choice curs  cursbk shdow
DATA 0,    7,   15,   7,    0,    7,    0,    15,    0,     0   ' monochrome
DATA 1,    9,   12,   3,    0,    1,   15,     0,    7,     0   ' cyan/blue
DATA 3,   15,   13,   1,   14,    3,   15,     0,    7,     0   ' blue/cyan
DATA 7,   12,   15,   4,   14,    0,   15,    15,    1,     0   ' red/grey
```

Indices 1–10 are read into `colors(1..10, ColorSet)` during `Initialize`.
All display code accesses attributes as `colors(N, ColorPref)`:
- `colors(1, cp)` = screen background
- `colors(2, cp)` = FancyCls dot colour
- `colors(3, cp)` = menu bar / title fg
- `colors(4, cp)` = menu bar / box background
- `colors(5, cp)` = title bar fg
- `colors(6, cp)` = dropdown shadow bg
- `colors(7, cp)` = normal text fg
- `colors(8, cp)` = cursor highlight fg
- `colors(9, cp)` = cursor highlight bg
- `colors(10, cp)` = shadow overlay colour

`ColorPref` is saved to `money.dat` after any change, making the preference
persistent across runs.

---

## Control Flow Characteristics

- **`DEFINT A-Z`** — every bare identifier is integer-typed.  The transpiler
  stores all values as f64 regardless; DEFINT affects name mangling only (no
  sigil suffix on most variables).
- **No line numbers** — pure structured QBasic (SUBs, FUNCTIONs, DO/WHILE/FOR).
- **GOSUBs only, no GOTO in normal flow** — all GOSUB targets live inside their
  enclosing SUB.  The transpiler extracts them as labelled `'__gosub_…: loop {`
  blocks.
- **`ON ERROR GOTO ErrorTrap`** — used in two places: the startup file existence
  check and the printer test.  The transpiler stubs `ON ERROR` (no runtime error
  dispatch is modelled); `ErrorTrap` code in the main body is dead in the Rust
  binary.  The first error case (file not found) is handled by the logic that
  checks `OPEN` success.
- **`RESUME`** / **`RESUME NEXT`** — not implemented; the stub is safe because
  `ON ERROR` is itself a stub.
- **`CALL Absolute`** — stubbed to no-op.
- **`DEF SEG`** / **`VARSEG`** / **`VARPTR`** — stubbed to no-op / `0.0`.
- **`LPRINT`** — emitted as `__rt.lprint(...)` which currently writes to stdout
  in the transpiled binary (no actual LPT1 support).
- **`DATE$`** — emitted as `__rt.date_str()` which returns the system date.
- **`WIDTH`** / **`VIEW PRINT`** — no-ops (text mode only, no graphics).
- **`LOCATE , , 0/1`** — cursor visibility; runtime stub ignores it.

---

## Why `money.bas` Forced Transpiler Fixes

The program exposed five independent bugs, all of which were fixed before it
joined the `build-all.sh` suite:

### 1. UTF-8 source decoding (`src/main.rs`)

`money.bas` was saved as UTF-8.  The old source reader used the byte-by-byte
Latin-1 fallback unconditionally, splitting the multi-byte U+00C4 (Ä, 0xC3 0x84)
into two separate characters, corrupting string literals such as `"ÄÄÄÄÄÄÄÄ"`.
Fix: try `std::str::from_utf8()` first; only fall back to byte-as-char for files
that are not valid UTF-8.

### 2. CP437 font extension (`runtime/src/lib.rs`)

`FONT_8X8` covered only the first 128 code points.  Characters above 0x7F (all
box-drawing and block glyphs, the full Money Manager UI) rendered as space or
garbage.  Fix: extend `FONT_8X8` to 256 entries with a complete CP437 table.
`draw_char_fb` changed to use the raw code-point index.

### 3. Latin-1 binary string encoding (`runtime/src/lib.rs`)

`MKD$` produces a string where each byte of the IEEE 754 little-endian double is
stored as a Latin-1 character (`byte b` → `char::from_u32(b)`).  Reading back
with `CVD` reverses this.  The old implementation used ASCII, which silently
truncated bytes above 0x7F.  Fix: use Latin-1 encoding throughout `MKD$`/`CVD`,
`MKI$`/`CVI`, `MKS$`/`CVS`, `MKL$`/`CVL`.  `qb_lset`/`qb_rset` updated to
measure string width in chars (`.chars().count()`) not bytes.

### 4. `INPUT #n` numeric trim (`src/emitter.rs`)

`PRINT #2, ColorPref` emits `" 1 "` (QB leading-space convention for positive
numbers).  When read back with `INPUT #1, ColorPref`, the emitted code called
`.parse::<f64>()` on `" 1 "` — Rust rejects the surrounding whitespace and
returns `Err`, leaving `ColorPref` as `0.0`.  `colors(x)(0)` is never
populated, so all colour lookups returned black.  Fix: `.trim()` before
`.parse::<f64>()` in both file and interactive INPUT paths.

### 5. `local_dim_names` shadowing (`src/emitter.rs`)

A `HashSet<String>` tracks names that have been explicitly `DIM`'d within the
current scope so that a local integer variable `B` is not shadowed by a
cross-scope promoted string `B$` (which has the distinct Rust name `b_s`).
Without this, `DIM B` in a SUB that also referenced `DIM SHARED B$` would be
suppressed, leaving the local uninitialised.

### Parser additions (`src/parser.rs`)

- `ON KEY(n) GOSUB/GOTO`, `KEY(n) ON/OFF/STOP`, `TIMER ON/OFF/STOP` — consume
  to EOL as no-ops (event traps are not modelled).
- `CLEAR` — consume to EOL, return `None`.
- `REDIM SHARED` — propagate `shared = true` to `parse_var_decl` correctly.
- Removed dead duplicate FIELD handler (41 lines) that shadowed the correct
  `parse_field()` and discarded all field-length information.

---

## Verification

```bash
cargo build --release
bash tests/run-tests.sh               # 27/27 — must stay green
cargo test --workspace                 # 84+ unit tests — must stay green

# Build and run money interactively
./target/release/qbc basic-src/money.bas -o bin/money.rs
RLIB=$(ls -t target/release/deps/libqbasic_runtime-*.rlib | head -1)
rustc bin/money.rs --edition 2021 \
  -L target/release/deps --extern qbasic_runtime=$RLIB \
  -o bin/money
./bin/money                            # creates money.dat on first run
```

Expected on first run: the intro sparkle screen appears; pressing any key opens
the five-item menu bar.  Accounts/Transactions opens a 19-row account list;
selecting one opens `EditTrans`; F2 saves and returns.  The `money.dat` and
`money.N` files are created on disk and persist across runs.

---

## QB Feature Checklist

| Feature | Status |
|---------|--------|
| `DEFINT A-Z` | ✅ no change needed — all vars are f64 internally |
| `TYPE` with fixed-length `STRING * n` | ✅ flattened to `Vec<String>` in GameState |
| `DIM SHARED … AS Type` | ✅ emits per-field Vec fields |
| `OPEN FOR RANDOM … LEN =` | ✅ random-access file I/O |
| `FIELD #n, N AS var$` | ✅ qb_field_get / qb_field_put |
| `LSET` / `RSET` | ✅ qb_lset / qb_rset (char-count padding) |
| `MKD$` / `CVD` | ✅ Latin-1 IEEE 754 LE |
| `MKI$` / `CVI` | ✅ Latin-1 LE i16 |
| `GET #n, rec` / `PUT #n, rec` | ✅ read_record / write_record |
| `INPUT #n, var` | ✅ (with .trim() fix) |
| `LINE INPUT #n, var$` | ✅ |
| `PRINT #n, val` | ✅ |
| `OPEN FOR INPUT/OUTPUT` | ✅ sequential text I/O |
| `ON ERROR GOTO` / `RESUME` | ⚠️ stub; startup probe works by presence check |
| `CALL Absolute(VARPTR(…))` | ⚠️ stub (scroll no-ops) |
| `DEF SEG` / `VARSEG` / `VARPTR` | ⚠️ stub (0.0) |
| `POKE` / `PEEK` | ✅ poke_mem HashMap |
| `LPRINT` | ✅ stdout in transpiled binary |
| `DATE$` | ✅ system date via qb_date_str() |
| `WIDTH` / `VIEW PRINT` | ⚠️ no-op (text mode) |
| `LOCATE` / `COLOR` / `CLS` | ✅ text framebuffer |
| CP437 characters 0x80–0xFF | ✅ extended FONT_8X8 (256 entries) |
| UTF-8 source file | ✅ try_from_utf8 first |
| `PRINT USING "###,###.##"` | ✅ including `$$` floating-dollar and `+` sign |
| GOSUB targets within SUBs | ✅ labelled-loop extraction |
| `REDIM` local arrays | ✅ |
| `REDIM SHARED` | ✅ |
| `EXIT SUB` / `EXIT FUNCTION` | ✅ |
| `SELECT CASE` | ✅ |
| `DO … LOOP UNTIL` | ✅ |
| `WHILE … WEND` | ✅ |
