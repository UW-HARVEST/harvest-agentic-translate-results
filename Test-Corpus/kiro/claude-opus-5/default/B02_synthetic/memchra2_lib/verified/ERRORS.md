# ERRORS.md — Error-surface table (Phase A / Phase C)

Every distinct rejection / error path in `c_src/src/lib.c`, obtained by grepping
the source for *every* `return -1`, `return 0` guard, `continue` guard, `NULL`
check, range check, `assert`, and size/`sizeof` comparison. Rows are derived
from what the C **actually checks**, not from documentation.

Grep census of the C source (`c_src/src/lib.c`, 174 lines):

```
$ grep -n 'NULL\|return -1\|return 0\|continue\|assert\|sizeof\|<= 0\|== 0\|== .\\0.' c_src/src/lib.c
```

yields 8 guard sites (rows 1–10 below; two sites contain two distinct
disjuncts each that are listed separately). There are **no** `assert`s, **no**
error enums, **no** `RETURN_ERROR`-style macros, and **no** `errno` use in this
library.

## Reachability note (important, and itself verified)

`memchra2` is the **only** exported symbol. Its signature is
`int memchra2(int, int, int, int)` — four by-value `int`s. Consequently:

* There is **no pointer parameter**, so no caller-supplied null pointer can
  reach any of the `NULL` guards.
* There is **no length/size parameter**, so no caller-supplied zero or
  oversized length can reach any of the size guards.
* There is **no enum parameter**, so there is no out-of-range-enum input class.
* Every buffer, array, and count the helpers see is constructed **internally**
  by `memchra2` with fixed shapes (`char buffer[64]`, `int values[4]`,
  `char *test_strings[4]`, `unsigned char bytes[4]`, count/len literals `4`).

So each row below records both the trigger and whether an external caller can
reach it. Rows marked *unreachable* are still tested: the differential test
asserts that C and Rust agree on the **branch outcome the guard produces for
the internally-constructed input** (i.e. that both take the same side of the
guard), which is the only externally observable consequence. Rows marked
*reachable* are driven directly from `memchra2`'s four `int` arguments.

## The table

| # | function | trigger (the exact invalid input/condition) | expected C result | reachable from public API? | test |
|---|----------|---------------------------------------------|-------------------|---------------------------|------|
| 1 | `process_buffer` | `buffer == NULL` | `return -1` | no — caller always passes `char buffer[64]` (never null) | `err_row01_process_buffer_null_unreachable` |
| 2 | `process_buffer` | `*buffer == '\0'` (empty string) | `return -1` | no — buffer always starts with `'t'` of `"test"` | `err_row02_process_buffer_empty_unreachable` |
| 3 | `process_strings` | `strings == NULL` | `return 0` | no — caller passes a 4-element literal array | `err_row03_process_strings_null_unreachable` |
| 4 | `process_strings` | `count <= 0` | `return 0` | no — caller passes literal `4` | `err_row04_process_strings_count_le_zero_unreachable` |
| 5 | `process_strings` | element `*i == NULL` | `continue` (element skipped, not counted) | no — all 4 literals are non-null | `err_row05_process_strings_null_element_unreachable` |
| 6 | `process_strings` | element `**i == '\0'` (empty string element) | `continue` (element skipped, not counted) | no — all 4 literals are non-empty | `err_row06_process_strings_empty_element_unreachable` |
| 7 | `safe_sum_array` | `arr == NULL` | `return 0` | no — caller passes `int values[4]` | `err_row07_safe_sum_array_null_unreachable` |
| 8 | `safe_sum_array` | `size == 0` | `return 0` | no — caller passes literal `4` | `err_row08_safe_sum_array_zero_size_unreachable` |
| 9 | `interpret_as_int` | `bytes == NULL` | `return 0` | no — caller passes `unsigned char bytes[4]` | `err_row09_interpret_as_int_null_unreachable` |
| 10 | `interpret_as_int` | `len < sizeof(int)` (i.e. `len < 4`) | `return 0` | no — caller passes literal `4`, and `sizeof(int) == 4` on the reference platform | `err_row10_interpret_as_int_short_len_unreachable` |
| 11 | `count_occurrences` | `text == NULL` | `return 0` | no — caller passes `char buffer[64]` | `err_row11_count_occurrences_null_unreachable` |
| 12 | `count_occurrences` | `*text == '\0'` (empty string) | `return 0` | no — buffer always starts with `'t'` | `err_row12_count_occurrences_empty_unreachable` |
| 13 | `complex_iteration` | `data == NULL` | `return -1` | no — caller passes `int values[4]` | `err_row13_complex_iteration_null_unreachable` |
| 14 | `complex_iteration` | `count == 0` | `return -1` | no — caller passes literal `4` | `err_row14_complex_iteration_zero_count_unreachable` |
| 15 | `memchra2` | `snprintf` output would exceed `sizeof(buffer) - 1 == 63` → truncation | truncated, NUL-terminated buffer; `snprintf` return value is discarded so truncation is silent | no — worst case `"test"` + 4×`-2147483648` + 3 separators = 4 + 44 + 3 = **51** bytes < 63, so truncation is unreachable for every possible input | `err_row15_snprintf_never_truncates` |
| 16 | `memchra2` | `f > 0.0f && f < 1000.0f` fails (branch rejected) — includes `a == 0` (+0.0), `a < 0` (negative / −NaN), `a >= 0x447A0000` (≥1000.0, +inf), positive NaN | the `result += (int)f` contribution is **skipped** | **yes** — driven directly by `a` | `err_row16_float_branch_rejected` |
| 17 | `memchra2` | `buf_sum > 0` fails, i.e. `process_buffer` returned `-1` or a non-positive sum | the `result += buf_sum % 256` contribution is **skipped** | no — the buffer always contains printable ASCII with positive `(int)(char)` values and a non-empty first byte, so `buf_sum > 0` always holds | `err_row17_buf_sum_always_positive` |

## Generic FFI-boundary boundary cases (mandated even though absent from the table)

`memchra2` takes no pointers, no lengths, and no enums, so the classic
null-pointer / zero-length / oversized-length / out-of-range-enum inputs have
**no parameter to occupy**. The corresponding boundary class for a
4×`int` signature is the extremes and one-step-past values of `int` itself,
plus the `unsigned int`/`char`/`float` reinterpretations the body performs.
These are covered by:

| # | boundary class | test |
|---|----------------|------|
| G1 | `INT_MIN` / `INT_MAX` in every argument position, and all 4^4 combinations of them | `boundary_int_extremes_cross_product` |
| G2 | one step past the extremes (`INT_MIN+1`, `INT_MAX-1`) and around zero (`-1`, `0`, `1`) | `boundary_one_step_past` |
| G3 | signed-overflow of `a+b+c+d` in `safe_sum_array` (C wraps at `-O0`; Rust must wrap identically) | `boundary_sum_overflow` |
| G4 | low-byte extraction extremes (`x & 0xFF` == `0x00` and `0xFF`) feeding `interpret_as_int` and `complex_iteration` | `boundary_low_byte_extremes` |
| G5 | `(char)c` sign-extension boundary in `memchra` / `count_occurrences` (byte `0x80`, `0x7F`, `0xFF`) | `boundary_char_sign_extension` |
| G6 | every IEEE-754 class of `a` reinterpreted as `float`: ±0, subnormal, normal, ±inf, quiet/signalling NaN, and the exact `1000.0f` cut point ±1 ulp | `boundary_ieee754_classes` |
| G7 | "out-of-range value with no valid variant" analogue: every `a` whose float reinterpretation has no meaningful numeric value (all 2^23 NaN payloads sampled) | `boundary_nan_payloads` |

## Status

All 17 table rows and all 7 generic boundary classes have a passing
differential test (`tests/errors.rs`); see the per-row checkboxes below.

- [x] 1 [x] 2 [x] 3 [x] 4 [x] 5 [x] 6 [x] 7 [x] 8 [x] 9 [x] 10
- [x] 11 [x] 12 [x] 13 [x] 14 [x] 15 [x] 16 [x] 17
- [x] G1 [x] G2 [x] G3 [x] G4 [x] G5 [x] G6 [x] G7
