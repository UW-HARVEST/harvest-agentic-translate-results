# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/driver.c`. The exhaustive grep for
rejection constructs is:

```
grep -n "return\|assert\|exit\|abort" driver.c   -> (no matches at all)
grep -n "if *("                       driver.c   -> lines 31, 46, 66, 85
grep -n "ERROR"                       driver.c   -> lines 57, 77, 96
grep -n "NULL"                        driver.c   -> line 31
```

**Every function in the library returns `void`.** There is no error code, no
sentinel return, no `errno`, no `assert`, no `exit`/`abort`, and no error enum
anywhere in the C source. The library's *entire* rejection surface is therefore:

* 1 null-pointer guard (`driver.c:31`), whose rejection action is **emit nothing**;
* 3 index guards (`driver.c:46`, `:66`, `:85`), whose rejection action is
  **print a specific diagnostic line to stdout**.

Consequently, "same error/rejection" is asserted as **byte-identical stdout**
(the observable rejection signal), captured per call via `dup`/`dup2` on fd 1.
The magic constant governing the guards is the array length `10`
(`int buffer[10]`, `driver.c:45/65/84`, loop bound `i < 10`, and the literal
`(10)` in the `goodB2G` guard at `:85`).

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `printLine` | `line == NULL` — guard `if(line != NULL)` at `:31` is false | Emits **nothing at all** (0 bytes; not even a newline). Returns normally. | `err_01_print_line_null` | [x] |
| 2 | `bad` | `data < 0` — guard `if (data >= 0)` at `:46` is false (`-1`) | Emits exactly `ERROR: Array index is negative.\n`; the 10 `printIntLine` calls are skipped. | `err_02_bad_negative_one` | [x] |
| 3 | `bad` | `data == INT_MIN` — extreme negative, same guard at `:46` | Same as #2: `ERROR: Array index is negative.\n` | `err_03_bad_int_min` | [x] |
| 4 | `bad` | `data < 0` for many randomized negatives | Same as #2 for every one | `err_04_bad_negative_random` | [x] |
| 5 | `bad` | `data >= 10` — **the MISSING guard** (CWE-787). `bad` checks only `data >= 0`, never `data < 10`, unlike `goodB2G` at `:85`. `data == 10` is one step past the valid range. | *Not* rejected. C performs an out-of-bounds write `buffer[data] = 1` and then prints the 10 in-bounds elements, all still `0` (the `1` landed outside them). Compared for `data ∈ {10, 11}`, which is the full set that stays inside `bad`'s own frame; `data >= 12` overwrites the saved `%rbp` / return address and is caller-dependent UB — see the frame-layout analysis in `CONFIGS.md`. | `err_05_bad_oob_write_not_rejected` | [x] |
| 6 | `good` → `goodB2G` | `data < 0` — guard `if (data >= 0 && data < (10))` at `:85`, first conjunct false | `goodG2B` output first (its fixed `data = 7` pattern: nine `0`s with `1` at index 7), then exactly `ERROR: Array index is out-of-bounds\n`. Note: **different message** from #2 and **no trailing period**. | `err_06_good_negative` | [x] |
| 7 | `good` → `goodB2G` | `data == 10` — guard's second conjunct `data < (10)` false; exactly one past the last valid index `9` | `goodG2B` pattern, then `ERROR: Array index is out-of-bounds\n` | `err_07_good_ten_boundary` | [x] |
| 8 | `good` → `goodB2G` | `data == INT_MAX` / `data == INT_MIN` — extremes of both conjuncts | `goodG2B` pattern, then `ERROR: Array index is out-of-bounds\n` | `err_08_good_int_extremes` | [x] |
| 9 | `good` → `goodB2G` | randomized `data` outside `[0, 10)` on both sides | `ERROR: Array index is out-of-bounds\n` every time | `err_09_good_out_of_range_random` | [x] |
| 10 | `good` → `goodG2B` | `else` branch at `:77` (`ERROR: Array index is negative.`) — **DEAD CODE**: `goodG2B` hardcodes `int data = 7`, so `data >= 0` is always true and this message can never be emitted, for any input. | The string `ERROR: Array index is negative.` must **never** appear in `good()`'s output, for any `data`. Asserted negatively. | `err_10_goodg2b_else_is_dead_code` | [x] |
| 11 | `driver` | `badData < 0` (propagates #2) with `goodData` valid | `...Calling bad()...\nERROR: Array index is negative.\nFinished bad()\n` | `err_11_driver_bad_negative` | [x] |
| 12 | `driver` | `goodData` out of `[0,10)` (propagates #6/#7) with `badData` valid | `ERROR: Array index is out-of-bounds\n` inside the `good()` section | `err_12_driver_good_out_of_range` | [x] |
| 13 | `driver` | both `goodData` and `badData` invalid (negative) — both rejections in one call, checking they are emitted in the right order with the right two *distinct* messages | out-of-bounds message in the `good` section, negative message in the `bad` section | `err_13_driver_both_invalid` | [x] |
| 14 | `printIntLine` | **no rejection exists** — the function has no guard whatsoever and accepts every `int`, including `INT_MIN`/`INT_MAX`. Documented so the table is exhaustive; the negative assertion is that neither impl rejects or diverges. | Prints the decimal value + `\n` for every 32-bit input | `err_14_print_int_line_no_rejection` | [x] |

## Generic FFI-boundary cases (required even though not in the table above)

| # | case | covered by |
|---|------|-----------|
| G1 | Null pointer into the only pointer-taking function (`printLine`) | #1 / `err_01_print_line_null` |
| G2 | Zero length / empty input: `printLine("")` (valid, emits just `\n`) | `cfg_*` row 2 in `CONFIGS.md` |
| G3 | Oversized length: `printLine` with a 100 000-byte string | `edge_print_line_huge` |
| G4 | One step past the documented valid range: `data == 10` for `good` (#7) and `data == 10` for `bad` (#5) | #5, #7 |
| G5 | One step *before* the valid range: `data == -1` (#2, #6) | #2, #6 |
| G6 | Out-of-range **enum** values across the FFI boundary | **N/A by construction, and verified:** the public API in `c_src/include/driver.h` declares no `enum` and no `typedef`; `grep -n "enum\|typedef\|struct" c_src/include/driver.h c_src/src/driver.c` returns nothing. Every parameter is a plain `int` or `const char *`. The `int` equivalent of "a value with no valid variant" is an `int` outside `[0,10)`, which is covered exhaustively by #2–#9 including both 32-bit extremes and randomized values. Asserted explicitly in `err_15_no_enum_surface_full_int_sweep`, which sweeps the full `int` range structurally (all 32 bit positions ±, both extremes, and 4096 uniformly random `i32`s) through `good`. |
| G7 | Non-UTF-8 / embedded-`%` bytes through `printLine` (a Rust-specific hazard: `%` would be a format-string bug if the translation passed `line` as the format) | `edge_print_line_percent`, `edge_print_line_non_utf8` |
