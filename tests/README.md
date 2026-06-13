# Integration Tests

Regression suite for the `qbasic-rust` transpiler. Every test transpiles a
`.bas` source file to Rust, compiles it, runs it, and diffs stdout against a
golden expected-output file. All 23 tests must pass before any change is merged.

---

## Directory layout

```
tests/
├── run-tests.sh        # Test runner — the only script you need
├── programs/           # QBasic source files (.bas) — one per test
├── expected/           # Golden stdout for each test (.txt, same stem)
└── tmp/                # Working directory: .rs, binary, .out (auto-cleaned)
```

---

## Running the tests

```bash
# Build everything first (transpiler + runtime)
cargo build --release

# Run all 30 tests
bash tests/run-tests.sh

# Verbose: show actual vs expected on failure + full output on pass
bash tests/run-tests.sh -v

# Debug: keep the generated .rs files in tests/tmp/ after the run
bash tests/run-tests.sh -d
```

The script auto-builds the transpiler and runtime if `target/release/qbc`
or `libqbasic_runtime.rlib` are missing.

Exit code is `0` when all tests pass, `1` when any fail.

---

## What the runner does (per test)

```
programs/foo.bas
   │
   ├─[1. qbc foo.bas -o tmp/foo.rs]──────────────► tmp/foo.rs
   │                                                   │
   ├─[2. rustc tmp/foo.rs ... -o tmp/foo]─────────► tmp/foo
   │                                                   │
   ├─[3. timeout 5 tmp/foo > tmp/foo.out]─────────► tmp/foo.out
   │                                                   │
   └─[4. diff expected/foo.txt tmp/foo.out]────────► PASS / FAIL
```

A test fails at the first broken stage — transpile error, compile error,
runtime crash/timeout, or output mismatch — and execution continues with the
next test. The final summary lists every failure with its stage.

---

## Test catalog

Tests are grouped below by the language feature they primarily exercise.

### Output formatting

| Test | What it covers |
|---|---|
| `print` | `PRINT` with semicolons (packed), commas (tab zones), bare `PRINT` (blank line), numeric and string values |

### Control flow

| Test | What it covers |
|---|---|
| `if_single` | Single-line `IF/THEN`, `IF/THEN/ELSE`, nested single-line `IF`, multiple statements after `THEN` (colon-separated); block-IF whose THEN ends with a single-line IF — the block ELSE must not be stolen |
| `if_multi` | Multi-line `IF/ELSEIF/ELSE/END IF`, untaken `ELSE` branch |
| `for_loop` | `FOR/NEXT` with positive step, negative step, fractional step (`STEP 0.5`), `EXIT FOR` |
| `while_wend` | `WHILE/WEND`, loop that never executes (`WHILE 0`), nested `WHILE` |
| `do_loop` | All four `DO/LOOP` variants: pre-`WHILE`, pre-`UNTIL`, post-`WHILE`, post-`UNTIL`; `EXIT DO` |
| `select_case` | `SELECT CASE` on numeric and string expressions; `CASE val`, `CASE v1, v2`, `CASE v1 TO v2`, `CASE IS > n`, `CASE ELSE` |

### Line numbers, GOTO, and GOSUB

| Test | What it covers |
|---|---|
| `goto_linenum` | Simple `GOTO` loop with line-number labels (`10`–`70`) |
| `nested_goto` | Two nested `GOTO` counter loops — state-machine fallback path |
| `linenum_for` | `FOR/NEXT` entirely within a line-numbered program |
| `gosub_linenum` | `GOSUB` to a numeric line label, `RETURN`, shared variable (`result`) |
| `gosub_scope` | `GOSUB` reading and writing main-scope variables; named labels (`AddThem:`, `DoubleX:`) |

### Subroutines and functions

| Test | What it covers |
|---|---|
| `sub_byref` | `SUB` with by-reference numeric params, by-reference string params, multiple `&mut` params |
| `function_ret` | `FUNCTION` returning `f64` (numeric), `FUNCTION` returning `String` (`$`), recursive `FUNCTION` (`Factorial`) |

### Arrays

| Test | What it covers |
|---|---|
| `array_1d` | 1-D numeric and string arrays, `UBOUND`, passing an array to a `SUB` |
| `array_2d` | 2-D array `DIM`, read/write with nested `FOR`, row-major print |
| `array_bounds` | Explicit `TO` lower bounds (`DIM a(5 TO 10)`), `LBOUND`/`UBOUND`, 2-D with both dims bounded, default lower bound 0 |

### Data and math

| Test | What it covers |
|---|---|
| `data_read` | `DATA` literals (numeric and string), `READ`, `RESTORE` and re-read |
| `math_funcs` | `INT`, `FIX`, `ABS`, `SQR`, `SGN`, `MOD`, integer division (`\`), `^` (power), edge cases (`INT(-3.1)`, `FIX(-3.9)`) |
| `numeric` | `CINT` banker's rounding (ties to even), integer division and `MOD` with pre-rounded operands, operator precedence (`*`/`/` tighter than `\` tighter than `MOD`) |
| `qb_semantics` | Full QB-fidelity regression: operator precedence, `^` left-associativity, byref array-element mutation in SUBs, `NEXT i, j` multi-counter, `DATA` backslash escaping, `EQV`/`IMP`, `UBOUND`/`LBOUND` on string arrays, `RND(0)`/`RND(-n)`, QB LCG first value |
| `val_edge` | `VAL` stops at first non-numeric character; `VAL("&Hnn")` hex and `VAL("&Onn")` octal prefixes |

### Strings

| Test | What it covers |
|---|---|
| `string_concat` | `+` concatenation, `LEN`, comparison operators (`=`, `<`, `>`), QB boolean result (`-1` / `0`) |
| `string_ops` | `LEFT$`, `RIGHT$`, `MID$`, `UCASE$`, `LCASE$`, `INSTR`, `CHR$`, `ASC`, `SPACE$`, `STRING$`, `STR$`, `VAL`, empty-string edge cases |

### Graphics (headless)

| Test | What it covers |
|---|---|
| `paint_pattern` | `PAINT (x,y), CHR$(n), border` pattern tiling and `POINT(x,y)` readback — uses the default framebuffer (no window), output to stdout |

### Files and memory

| Test | What it covers |
|---|---|
| `record_io` | Random-access TYPE record round-trip: `OPEN FOR RANDOM`, `PUT #n, rec, var`, `GET #n, rec, var`, fixed-string and INTEGER fields, close/reopen |

### User-defined types

| Test | What it covers |
|---|---|
| `type_nested` | Single-level `TYPE` with scalar fields, nested `TYPE` field (`Col AS Color`), field access via `.` |
| `type_complex` | Nested `TYPE` array (`DIM px(1 TO 3) AS Pixel`), scalar `TYPE` passed to `SUB` (expanded to per-field `&mut` params), field-level swap using a temp `TYPE` variable |
| `type_array_field` | Array field inside a `TYPE` body (`Cell(4) AS INTEGER`); scalar, DIM SHARED, and array-of-TYPE forms; `arr(i).Field(j)` access |

### Variables and scope

| Test | What it covers |
|---|---|
| `common_static` | `COMMON SHARED` variable visible in main and SUBs; `STATIC` local persisting across calls |

---

## Adding a new test

1. Write `tests/programs/my_feature.bas` — text only, no graphics, no `INKEY$`
   blocking, terminates within 5 seconds.
2. Run the binary by hand to capture correct output:
   ```bash
   cargo build --release
   ./target/release/qbc tests/programs/my_feature.bas -o /tmp/my_feature.rs
   rustc /tmp/my_feature.rs --edition 2021 \
       -L target/release/deps \
       --extern qbasic_runtime=target/release/libqbasic_runtime.rlib \
       -o /tmp/my_feature
   /tmp/my_feature
   ```
3. Paste the output into `tests/expected/my_feature.txt` (include trailing
   newlines exactly as printed).
4. Run the full suite to confirm `PASS: my_feature` and that no existing tests
   regressed:
   ```bash
   bash tests/run-tests.sh
   ```

> **Text-only rule:** the runner captures `stdout` and diffs it. Programs that
> open a graphics window (`SCREEN N`) print nothing to stdout (the runtime
> suppresses output when `had_screen_call` is true), so they would always
> produce an empty diff and test nothing useful. Keep integration tests
> text-only; use the programs in `basic-src/` for manual visual verification.
