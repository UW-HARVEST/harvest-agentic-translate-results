# ERRORS.md — Error-surface table (Phase A) + Phase C results

## Derivation

Mechanically grepped from `c_src/src/main.c` — every branch, every rejection,
every size constant:

```sh
grep -n "if\s*(\|else\|return\|NULL\|assert\|define\|> \|< \|== \|!= " c_src/src/main.c
```

```
30:    if(line != NULL)                                            <- NULL guard
41:#define CHAR_ARRAY_SIZE 20                                      <- only size constant
49:        if (fgets(inputBuffer, CHAR_ARRAY_SIZE, stdin) != NULL)  <- fgets failure (bad)
53:        else
80:        if (fgets(inputBuffer, CHAR_ARRAY_SIZE, stdin) != NULL)  <- fgets failure (goodB2G)
84:        else
89:    if (fabs(data) > 0.000001)                                   <- divide-by-zero guard
94:    else
114:    return 0;                                                   <- main's only return
```

That is the **complete** rejection surface. The file contains **no** `assert`,
**no** `return -1` / `return NULL`, **no** error enum, and **no** error-return
macro — the library signals problems only by (a) printing nothing, (b) printing
a fixed diagnostic string, or (c) producing the x86-64 "integer indefinite"
value from an out-of-range/NaN `double`->`int` conversion. Rows 8–13 below cover
the generic FFI boundaries (null pointers, extreme values, out-of-range ints)
that every C ABI has, as required by Phase C.

## Error-surface table

| # | function | trigger (exact invalid input/condition) | expected C result | test | status |
|---|----------|------------------------------------------|-------------------|------|--------|
| 1 | `printLine` (`main.c:30`) | `line == NULL` | guard `if(line != NULL)` fails: **no output at all**, not even a newline; returns void | `err_01_print_line_null` | [x] |
| 2 | `bad` (`main.c:49,53`) | `stdin` is at EOF when `fgets` is called (empty input) | `fgets` returns `NULL` -> prints `"fgets() failed.\n"`; `data` stays `0.0F`; then `(int)(100.0/0.0)` = `(int)+inf` -> prints `"-2147483648\n"` | `err_02_bad_fgets_eof` | [x] |
| 3 | `goodB2G` via `good` (`main.c:80,84`) | `stdin` at EOF when `fgets` is called | `fgets` returns `NULL` -> prints `"fgets() failed.\n"`; `data` stays `0.0F`; `fabs(0.0) > 1e-6` false -> prints `"This would result in a divide by zero\n"` | `err_03_good_fgets_eof` | [x] |
| 4 | `goodB2G` via `good` (`main.c:89,94`) | input parses to `data` with `fabs(data) <= 0.000001` (`0`, `-0`, `1e-7`, `0.000001` exactly, `abc`, whitespace-only, …) | guard fails -> prints `"This would result in a divide by zero\n"`, **no** division performed | `err_04_good_b2g_guard_rejects` | [x] |
| 5 | `goodB2G` via `good` (`main.c:89`) | input is `nan` / `-nan` / `nan(1)` | `fabs(NaN) > 0.000001` is **false** (all NaN comparisons are false) -> takes the *else* branch -> `"This would result in a divide by zero\n"` | `err_05_good_b2g_nan_guard` | [x] |
| 6 | `bad` (`main.c:58`, the FLAW) | input parses to `data == 0.0F` — literal `0`, `-0`, `0.0`, unparseable text (`atof` -> `0.0`), `1e-60` (underflows to `0.0F`) | **no guard**: `100.0/0.0` = `±inf`, `(int)±inf` is UB; on x86-64 `cvttsd2si` yields the integer-indefinite value -> prints `"-2147483648\n"` | `err_06_bad_divide_by_zero` | [x] |
| 7 | `bad` (`main.c:58`) | input parses to `data` so small that `100.0/data` exceeds `INT_MAX`/`INT_MIN` (`1e-30`, `-1e-30`, `1e-9`, …), or `data` is `nan` | out-of-range / NaN `(int)` conversion is UB; x86-64 yields `INT_MIN` -> prints `"-2147483648\n"` | `err_07_bad_out_of_int_range` | [x] |
| 8 | `bad`, `goodB2G` (`main.c:41,49,80`) | input line longer than `CHAR_ARRAY_SIZE - 1` = **19** bytes | `fgets` reads only 19 bytes + NUL; the remainder of the line **stays in `stdin`** and is seen by the *next* `fgets`. Boundary: exactly 19, exactly 20, and 21+ byte lines behave differently | `err_08_fgets_truncation_boundary` | [x] |
| 9 | `printIntLine` | out-of-range / extreme `int` passed over FFI: `INT_MIN`, `INT_MAX`, `-1`, `0` | C `int` accepts any 32-bit value; `printf("%d\n", …)` prints it verbatim (`-2147483648`, `2147483647`, …) | `err_09_print_int_line_extremes` | [x] |
| 10 | `printLine` | zero-length string (`""`) — the degenerate/"zero length" boundary | not NULL, so guard passes: prints just `"\n"` | `err_10_print_line_empty` | [x] |
| 11 | `printLine` | oversized / non-UTF-8 / embedded-`%` payload (64 KiB string, raw bytes `0x80..0xFF`, `"%d %s %n"`) | `printf("%s\n", line)` (GCC: `puts`) copies raw bytes up to NUL then a newline; `%` in the *data* is never interpreted | `err_11_print_line_oversized_nonutf8` | [x] |
| 12 | `main` | any `argc`/`argv`, including `argc = 0` with `argv = NULL` and an out-of-range/negative `argc` (`-1`, `INT_MIN`) | both parameters are unused; always runs the fixed sequence and **returns 0** (`main.c:114`) | `err_12_main_ignores_argv` | [x] |
| 13 | `bad`, `goodB2G` (`main.c:49,80`) | `stdin` is a closed/invalid file descriptor (read error rather than EOF) | `fgets` returns `NULL` -> `"fgets() failed."` path, identical to row 2/3 | `err_13_fgets_read_error` | [x] |

### Correction made while writing the Phase C tests

My first draft of rows 4 and 6 assumed `1e`, `1e+`, `1E-`, `5.` and `.5` were
"unparseable" and would therefore yield `0.0`. Running them against the C
implementation proved otherwise — `strtod` consumes the **longest valid
prefix**, so those inputs convert to `1.0`, `5.0` and `0.5` and take the
*accepted* path (`100`, `20`, `200`). The C is the ground truth, so the test
expectations were corrected to match it and those inputs now appear as explicit
**control** cases in `err_04` / `err_06` (proving the accept/reject split is
tested from both sides). The same applies to `4.7e-8`, which `bad` accepts
(prints `2127659559`) but `goodB2G`'s guard rejects.

Likewise `err_12`'s expected transcript initially omitted the `fgets() failed.`
line that `goodB2G` emits when stdin is empty; corrected against the C.

### Note on "out-of-range enum values passed across the FFI boundary"

The C source declares **no enums** (`grep -c enum c_src/src/main.c` = 0) and no
function takes an enum-like selector. The nearest equivalent — an `int`
parameter that can carry any value, including ones no sane caller would pass —
is `printIntLine`'s `intNumber`; row 9 covers the extremes and the randomized
Phase B row C-01 covers 20 000 random `i32` values including every boundary.
Row 12 covers passing nonsense (`argc = INT_MIN`, `argv = NULL`) to `main`.

## Results

All 13 rows have a passing differential test. Each asserts (a) that C and Rust
produce byte-identical output and (b) that the output is the *specific* sentinel
the C produces — the exact diagnostic string (`fgets() failed.`,
`This would result in a divide by zero`) or the x86-64 integer-indefinite value
`-2147483648` — never merely "both failed somehow".

Run with:

```sh
cargo test --offline --no-default-features -- --test-threads=1 err_
```

```
test err_01_print_line_null ... ok          test err_08_fgets_truncation_boundary ... ok
test err_02_bad_fgets_eof ... ok            test err_09_print_int_line_extremes ... ok
test err_03_good_fgets_eof ... ok           test err_10_print_line_empty ... ok
test err_04_good_b2g_guard_rejects ... ok   test err_11_print_line_oversized_nonutf8 ... ok
test err_05_good_b2g_nan_guard ... ok       test err_12_main_ignores_argv ... ok
test err_06_bad_divide_by_zero ... ok       test err_13_fgets_read_error ... ok
test err_07_bad_out_of_int_range ... ok
test result: ok. 13 passed; 0 failed
```

`mutation_check.sh` confirms these tests are not vacuous: inverting the
`printLine` NULL check, removing the `fgets()` failure message, dropping
`fabs()` from the guard, and un-forcing NaN to `INT_MIN` are all detected.
