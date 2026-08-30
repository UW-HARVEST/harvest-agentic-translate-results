# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/driver.c`. The library has **no return
values at all** (every public function is `void`) and **no error enums, no
`errno` use, no `assert`, no `RETURN_ERROR` macro, no `return -1`, no
`return NULL`**. Its entire "error surface" therefore consists of:

* the one **null-pointer guard** (`if (line != NULL)`),
* the **index guards** (`data >= 0`, `data >= 0 && data < 10`), whose failure is
  reported by printing a fixed diagnostic line to `stdout`,
* the **absent** upper-bound guard in `bad()` — the CWE-121 defect itself, which
  is not a rejection but is listed because it is the boundary a caller can cross.

Grep evidence:

```
$ grep -nE 'return|assert|NULL|ERROR|<|>=' c_src/src/driver.c
31:    if(line != NULL)                            -> row 1 / 2
46:    if (data >= 0)                 (bad)        -> row 3 / 4
57:        printLine("ERROR: Array index is negative.");
66:    if (data >= 0)                 (goodG2B)    -> row 8 (unreachable, data==7)
85:    if (data >= 0 && data < (10))  (goodB2G)    -> rows 5 / 6 / 7
96:        printLine("ERROR: Array index is out-of-bounds");
```

The only magic constant is the buffer length `10` (`int buffer[10]`, loop bound
`i < 10`, guard `data < (10)`).

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✅ |
|---|----------|---------------------------------------------|-------------------|------|----|
| 1 | `printLine` | `line == NULL` (guard at driver.c:31 fails) | rejected silently: **no output at all**, `printf` not reached, returns void | `err_01_print_line_null` | [x] |
| 2 | `printLine` | `line != NULL` but points at `""` (zero-length, degenerate accepted input) | accepted: prints just `"\n"` | `err_02_print_line_empty` | [x] |
| 3 | `bad` | `data < 0` (e.g. `-1`) — guard `data >= 0` at driver.c:46 fails | prints exactly `ERROR: Array index is negative.\n`, **no** buffer dump, no write | `err_03_bad_negative` | [x] |
| 4 | `bad` | `data == INT_MIN` (extreme negative, one step past the negative range) | same as row 3: `ERROR: Array index is negative.\n` | `err_04_bad_int_min` | [x] |
| 5 | `good` (→`goodB2G`) | `data < 0` (e.g. `-1`) — first half of guard at driver.c:85 fails | `goodG2B()` dump (10 lines) **then** `ERROR: Array index is out-of-bounds\n` | `err_05_good_negative` | [x] |
| 6 | `good` (→`goodB2G`) | `data == 10` — exactly one step past the valid range (`data < 10` fails) | `goodG2B()` dump then `ERROR: Array index is out-of-bounds\n` | `err_06_good_at_bound` | [x] |
| 7 | `good` (→`goodB2G`) | `data == INT_MAX` (oversized index) | `goodG2B()` dump then `ERROR: Array index is out-of-bounds\n` | `err_07_good_int_max` | [x] |
| 8 | `good` (→`goodG2B`) | `goodG2B`'s own `data >= 0` guard: **unreachable**, `data` is hard-coded `7`. The `else` branch printing `ERROR: Array index is negative.` is dead code in C and must be dead code in Rust too. | never taken — `good()` output never contains `Array index is negative` | `err_08_goodg2b_else_is_dead` | [x] |
| 9 | `bad` | `data == 10` — the **missing** upper-bound check (CWE-121). Guard `data >= 0` passes, write lands one past the end of `buffer[10]`. | accepted, out-of-bounds write performed; the 10 printed values are all `0` | `err_09_bad_one_past_end` | [x] |
| 10 | `driver` | `badData < 0` → propagates row 3 through the composed pipeline | `Calling good()...` / good output / `Finished good()` / `Calling bad()...` / `ERROR: Array index is negative.` / `Finished bad()` | `err_10_driver_bad_negative` | [x] |
| 11 | `driver` | `goodData` out of range (`<0` or `>=10`) → propagates rows 5–7 | good half emits `goodG2B` dump + out-of-bounds diagnostic; bad half unaffected | `err_11_driver_good_out_of_range` | [x] |
| 12 | `driver` | both `goodData` and `badData` invalid simultaneously | both diagnostics appear, in the fixed order printed by `driver` | `err_12_driver_both_invalid` | [x] |

## Generic FFI boundary cases (required even though not in the C table)

| # | case | note | test | ✅ |
|---|------|------|------|----|
| G1 | null pointer to `printLine` | the library's only pointer parameter | `err_01_print_line_null` | [x] |
| G2 | zero-length input (`""`) to `printLine` | | `err_02_print_line_empty` | [x] |
| G3 | oversized input to `printLine` (64 KiB string, > any internal buffer) | `printf("%s")` has no length limit | `err_g3_print_line_oversized` | [x] |
| G4 | `printLine` string containing `printf` conversion specifiers (`%s %n %d`) | must be passed as an *argument*, never as the format string | `err_g4_print_line_format_specifiers` | [x] |
| G5 | `printLine` string containing non-UTF-8 / high bytes and embedded `\n` | C is byte-oriented; Rust must not assume UTF-8 | `err_g5_print_line_non_utf8` | [x] |
| G6 | `printIntLine` at `INT_MIN` / `INT_MAX` / `-1` / `0` (one step past both ends of the range) | `%d` formatting of extremes | `err_g6_print_int_line_extremes` | [x] |
| G7 | "out-of-range enum value across the FFI boundary" — this library declares **no enum**; the closest analogue is an `int` parameter with no valid variant, i.e. an index with no in-bounds meaning. Covered by feeding `bad`/`good`/`driver` values with no valid interpretation (negative, `==10`, `INT_MAX`, `INT_MIN`). | no `enum`/`switch` exists in `driver.c` (`grep -c 'enum\|switch' == 0`) | `err_g7_no_enum_int_domain_sweep` | [x] |
| G8 | `bad` one step past the guard on the *valid* side (`data == 9`, last in-bounds index) | boundary of the only range check that exists | `err_g8_bad_last_in_bounds` | [x] |
