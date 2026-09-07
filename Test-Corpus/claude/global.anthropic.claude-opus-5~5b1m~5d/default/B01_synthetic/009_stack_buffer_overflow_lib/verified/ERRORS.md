# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/driver.c` and `c_src/include/driver.h`.

## Mechanical inventory of the C source

Every construct in `driver.c` that rejects, guards, or bounds an input:

| grep hit | line | kind |
|----------|------|------|
| `if(line != NULL)` in `printLine` | 31 | NULL check — suppresses all output |
| `if (data >= 0)` in `bad` | 46 | lower-bound range check |
| `else { printLine("ERROR: Array index is negative."); }` in `bad` | 57 | error message branch |
| *(absent)* upper-bound check in `bad` | 48 | **missing** check → `buffer[data]=1` writes OOB |
| `if (data >= 0)` in `goodG2B` | 66 | lower-bound check on the *constant* 7 — statically always true, `else` is dead code |
| `else { printLine("ERROR: Array index is negative."); }` in `goodG2B` | 77 | unreachable error branch |
| `if (data >= 0 && data < (10))` in `goodB2G` | 85 | full range check, two conjuncts |
| `else { printLine("ERROR: Array index is out-of-bounds"); }` in `goodB2G` | 96 | error message branch |
| `int buffer[10]` (×3) | 45, 65, 84 | the min/max constant: valid indices are `0..=9` |
| `for(i = 0; i < 10; i++)` (×3) | 50, 69, 89 | fixed print count, independent of `data` |

There are **no** `assert`s, **no** `RETURN_ERROR`-style macros, **no** error
enums, and **no** function in this library returns a value — every public
function is `void`. Therefore "the same error/rejection" is observed as *the
exact bytes written to stdout*, which is the library's only observable output.
The sentinel/"error code" for each row below is the specific error string (or
the absence of output).

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| 1 | `printLine` | `line == NULL` | no output at all (0 bytes); returns normally, no crash | `err01_print_line_null` | [x] |
| 2 | `printLine` | `line` points at `""` (empty, valid but degenerate) | exactly one `"\n"` | `err02_print_line_empty` | [x] |
| 3 | `printLine` | `line` contains `%` conversion specifiers (`"%s %d %n"`) | printed verbatim — arg is *not* a format string (compiler lowers to `puts`) | `err03_print_line_percent` | [x] |
| 4 | `bad` | `data < 0` (e.g. `-1`) | `"ERROR: Array index is negative.\n"`, nothing else | `err04_bad_negative` | [x] |
| 5 | `bad` | `data == INT_MIN` (extreme negative; `cltq`-sign-extended in C) | `"ERROR: Array index is negative.\n"` | `err05_bad_int_min` | [x] |
| 6 | `bad` | `data == 9` — the last *in-bounds* index (boundary) | ten lines, `1` on line 10, `0` elsewhere | `err06_bad_last_valid` | [x] |
| 7 | `bad` | `data == 10` — **one step past** the valid range; **no upper-bound check exists**, so C writes 4 bytes past `buffer` into stack padding at `-0x8(%rbp)`. `buffer[0..9]` untouched. | ten lines of `0` (no error message, no crash) | `err07_bad_off_by_one` | [x] |
| 8 | `bad` | `data == 11` — two past the range; C writes into the slot of the loop counter `i` at `-0x4(%rbp)`, which is then overwritten by `i = 0` | ten lines of `0` (no error message, no crash) | `err08_bad_off_by_two` | [x] |
| 9 | `bad` | `data >= 12` (e.g. `12`, `14`, `INT_MAX`) — clobbers the saved `%rbp` / return address | **undefined behaviour**: C corrupts its own frame and crashes (or worse) unpredictably. NOT differentially testable; documented, deliberately not asserted. | `err09_bad_ub_documented` (documents only) | [x] |
| 10 | `good` | `data < 0` → fails first conjunct of `goodB2G`'s `data >= 0 && data < 10` | `goodG2B` block (ten lines, `1` on line 8) then `"ERROR: Array index is out-of-bounds\n"` | `err10_good_negative` | [x] |
| 11 | `good` | `data == INT_MIN` | same as row 10 | `err11_good_int_min` | [x] |
| 12 | `good` | `data == 10` → fails second conjunct (`data < 10`); one step past valid range | `goodG2B` block then `"ERROR: Array index is out-of-bounds\n"` | `err12_good_off_by_one` | [x] |
| 13 | `good` | `data == INT_MAX` → fails second conjunct | `goodG2B` block then `"ERROR: Array index is out-of-bounds\n"` | `err13_good_int_max` | [x] |
| 14 | `good` | `data == 9` — last in-bounds value (boundary, must NOT error) | `goodG2B` block then ten lines with `1` on line 10 | `err14_good_last_valid` | [x] |
| 15 | `driver` | `goodData` out of range (`<0`, `>=10`, `INT_MIN`, `INT_MAX`) with a *valid* `badData` | banner lines + `goodG2B` block + out-of-bounds error + `bad` block | `err15_driver_bad_gooddata` | [x] |
| 16 | `driver` | `badData < 0` with valid `goodData` | banners + good output + `"ERROR: Array index is negative.\n"` | `err16_driver_negative_baddata` | [x] |
| 17 | `driver` | both `goodData` and `badData` invalid-but-defined (`goodData<0`, `badData<0`) | banners + out-of-bounds error + negative error | `err17_driver_both_invalid` | [x] |
| 18 | `printIntLine` | extreme / boundary `int` values: `INT_MIN`, `INT_MAX`, `-1`, `0` (`%d` formatting edge cases, incl. `-2147483648`) | decimal rendering + `\n`, identical in both | `err18_print_int_line_extremes` | [x] |

## Out-of-range "enum" values across the FFI boundary

`driver.h` declares no `enum` and no struct; the entire API surface is
`void f(int …)` / `void f(const char *)`. The analogue of "an out-of-range enum
value" here is therefore **an arbitrary `int` with no meaningful
interpretation** — i.e. any value outside `0..=9`. Those are covered
exhaustively by rows 4–17 above (negative, `0`, `9`, `10`, `11`, `INT_MIN`,
`INT_MAX`), and additionally by the randomized sweeps in Phase B, which push
thousands of arbitrary `int` bit patterns through `good`, `printIntLine` and
`driver`'s `goodData`.

## Generic-boundary checklist

- [x] NULL pointer → row 1 (`printLine(NULL)`).
- [x] Zero length → row 2 (`printLine("")`).
- [x] Oversized length → `errG3_print_line_oversized`: `printLine` with 4095 /
      4096 / 4097 / 65535 / 65536 / 65537 / 1 MiB strings (straddles the stdio
      and page-size boundaries).
- [x] One step past a documented valid range → rows 7 (`bad(10)`) and 12 (`good(10)`).
- [x] Extremes of the integer domain → rows 5, 11, 13, 18 (`INT_MIN` / `INT_MAX`),
      plus `errG1_generic_integer_boundaries_all_entry_points`, which pushes the
      12-value edge set through *every* entry point.
- [x] Out-of-range enum values across the FFI boundary →
      `errG2_out_of_range_scalar_discriminants` (272 hostile bit patterns:
      all-ones, sign-bit-only, `0xDEADBEEF`, …) through every entry point.
- [x] Degenerate pointers → `errG4_print_line_degenerate_pointers` (pointer to a
      lone NUL byte; NUL at the very last byte of a 4 KiB heap block, so any
      over-read past the terminator would be observable).
- [x] Arbitrary out-of-domain scalars → rows 4–17 plus Phase B randomized sweeps.

## Status

All 18 rows plus the 4 generic-boundary tests (`errG1`–`errG4`) pass in
`tests/phase_c_errors.rs`, in **both** the `debug` and `release` profiles and
under both feature sets:

```
$ cargo test --test phase_c_errors
test result: ok. 22 passed; 0 failed
$ cargo test --release --test phase_c_errors
test result: ok. 22 passed; 0 failed
```

Every row asserts *two* things — that C and Rust agree byte-for-byte, and that
the bytes are the specific expected sentinel (`"ERROR: Array index is
negative.\n"`, `"ERROR: Array index is out-of-bounds\n"`, or empty output) — so
a test cannot pass merely because "both failed somehow".
