# ERRORS.md — Phase A: error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. Every `return` that is guarded by a
validity check, every early-out / `continue`, every explicit range check, every
null check, every min/max constant, and every implicit truncation bound is one
row. There are **no** `assert`s, no error enums, no `RETURN_ERROR` macros and no
`errno` usage in this library (`grep -c 'assert\|errno\|RETURN_ERROR' → 0`), so
rejections are expressed purely as sentinel return values (`-1`, `0`) and as
`continue` skips.

## Notes on reachability

`memchra2` is the **only** dynamically exported symbol (see `SYMBOLS.md`). All
guarded helpers are `static` and are called by `memchra2` with hard-coded,
always-valid arguments (stack arrays, string literals, constant counts). A row
is therefore annotated:

* **R** = reachable by varying `a`/`b`/`c`/`d` through the public `memchra2`
  export → the differential test drives the exact condition through `memchra2`.
* **U** = unreachable through `memchra2` alone, because the helper is `static`
  and `memchra2` always passes it valid arguments. These rows are **still tested
  differentially**, not just inspected: the condition is constructed and fed to
  both implementations through the `harness_*` C-ABI wrappers described below,
  so C and Rust are compared on the exact invalid input and must return the same
  sentinel.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | reach | test |
|---|----------|----------------------------------------------|-------------------|-------|------|
| 1 | `process_buffer` (lib.c:40) | `buffer == NULL` | returns `-1` immediately | U | `err_row01_process_buffer_null` |
| 2 | `process_buffer` (lib.c:40) | `buffer != NULL` but `*buffer == '\0'` (empty string) | returns `-1` immediately | U | `err_row02_process_buffer_empty` |
| 3 | `process_buffer` (lib.c:45) | `len == 0` (valid ptr, non-empty) | loop body never runs → returns `0` | U | `err_row03_process_buffer_zero_len` |
| 4 | `process_buffer` (lib.c:45) | embedded `'\0'` before `buffer + len` | loop stops early at the NUL; returns partial sum | U | `err_row04_process_buffer_embedded_nul` |
| 5 | `memchra2` (lib.c:157) | `buf_sum <= 0` (i.e. `process_buffer` returned `-1` or `0`) | the `result += buf_sum % 256` contribution is **skipped** | U | `err_row05_bufsum_guard` |
| 6 | `process_strings` (lib.c:62) | `strings == NULL` | returns `0` | U | `err_row06_process_strings_null` |
| 7 | `process_strings` (lib.c:62) | `count <= 0` (`0` and negative) | returns `0` | U | `err_row07_process_strings_nonpositive_count` |
| 8 | `process_strings` (lib.c:69) | element `*i == NULL` | that element is skipped (`continue`), no match counted | U | `err_row08_process_strings_null_element` |
| 9 | `process_strings` (lib.c:69) | element `**i == '\0'` (empty string element) | that element is skipped (`continue`) | U | `err_row09_process_strings_empty_element` |
| 10 | `process_strings` (lib.c:73) | `strncmp` mismatch (element does not start with `target`) | not counted; `matches` unchanged | R* | `err_row10_process_strings_mismatch` |
| 11 | `process_strings` (lib.c:73) | `strlen(target) == 0` (empty target) | `strncmp(...,0) == 0` → **every** non-empty element matches | U | `err_row11_process_strings_empty_target` |
| 12 | `safe_sum_array` (lib.c:82) | `arr == NULL` | returns `0` | U | `err_row12_safe_sum_null` |
| 13 | `safe_sum_array` (lib.c:82) | `size == 0` | returns `0` | U | `err_row13_safe_sum_zero_size` |
| 14 | `safe_sum_array` (lib.c:88) | signed `int` overflow of `sum` (C UB; gcc `-fwrapv`-less codegen wraps) | wraps modulo 2^32, two's complement | R | `err_row14_safe_sum_overflow` |
| 15 | `interpret_as_int` (lib.c:96) | `bytes == NULL` | returns `0` | U | `err_row15_interpret_null` |
| 16 | `interpret_as_int` (lib.c:96) | `len < sizeof(int)` i.e. `len ∈ {0,1,2,3}` (one step below the valid minimum 4) | returns `0` | U | `err_row16_interpret_short_len` |
| 17 | `count_occurrences` (lib.c:105) | `text == NULL` | returns `0` | U | `err_row17_count_occ_null` |
| 18 | `count_occurrences` (lib.c:105) | `*text == '\0'` (empty string) | returns `0` (note: **not** `-1`) | U | `err_row18_count_occ_empty` |
| 19 | `memchra` (lib.c:31) | `n == 0` | loop never runs → returns `0` | U | `err_row19_memchra_zero_n` |
| 20 | `memchra` (lib.c:32) | needle `c` outside `char` range (e.g. `c = 0x12D` for `'-'`) | comparison uses `(char)c`, so only the low 8 bits matter | U | `err_row20_memchra_char_truncation` |
| 21 | `complex_iteration` (lib.c:114) | `data == NULL` | returns `-1` | U | `err_row21_complex_iter_null` |
| 22 | `complex_iteration` (lib.c:114) | `count == 0` | returns `-1` (note: **not** `0`) | U | `err_row22_complex_iter_zero_count` |
| 23 | `memchra2` (lib.c:152) | `f <= 0.0f` — bit pattern of `a` is `-0.0`, `+0.0`, or any negative float (`a < 0`) | float contribution **skipped** | R | `err_row23_float_guard_non_positive` |
| 24 | `memchra2` (lib.c:152) | `f >= 1000.0f` — `a >= 0x447A0000` (and `a` positive) | float contribution **skipped** | R | `err_row24_float_guard_ge_1000` |
| 25 | `memchra2` (lib.c:152) | `f == +INFINITY` (`a == 0x7F800000`) | `f < 1000.0f` false → skipped | R | `err_row25_float_guard_pos_inf` |
| 26 | `memchra2` (lib.c:152) | `f == -INFINITY` (`a == 0xFF800000` = `-8388608`) | `f > 0.0f` false → skipped | R | `err_row26_float_guard_neg_inf` |
| 27 | `memchra2` (lib.c:152) | `f` is NaN (`a` in `0x7F800001..0x7FFFFFFF` or `0xFF800001..0xFFFFFFFF`) | both comparisons false → skipped | R | `err_row27_float_guard_nan` |
| 28 | `memchra2` (lib.c:153) | `(int)f` on a subnormal / `f < 1.0f` (`a` in `1..0x3F7FFFFF`) | truncates toward zero → contributes `0` | R | `err_row28_float_trunc_subnormal` |
| 29 | `memchra2` (lib.c:132) | `snprintf` output longer than `sizeof(buffer)-1 == 63` | would truncate + NUL-terminate. Max output is `4 + 4*11 + 3 = 51` bytes, so truncation is unreachable; the bound must still be honoured | U | `err_row29_snprintf_bound` |
| 30 | `memchra2` (lib.c:132) | `INT_MIN` argument (`-2147483648`, the min constant of the domain) | `%d` prints `-2147483648` (11 chars, longest possible field) | R | `err_row30_int_min_args` |
| 31 | `memchra2` (lib.c:128) | `INT_MAX` argument (`2147483647`, the max constant of the domain) | normal path, 10-char field | R | `err_row31_int_max_args` |
| 32 | `memchra2` (lib.c:129-173) | signed overflow of `result` across `+=` / `*=` / `^=` | wraps modulo 2^32 | R | `err_row32_result_overflow` |
| 33 | `memchra2` FFI boundary | out-of-range "enum-like" ints passed across FFI (no enum params exist in this API; every one of the 2^32 `int` values is in-domain) | no rejection — all 4 params accept the full `int` range; both libs must agree | R | `err_row33_full_int_domain_no_rejection` |
| 34 | `memchra2` FFI boundary | all-zero arguments (degenerate/empty-ish input) | `a=b=c=d=0` → deterministic value, no rejection | R | `err_row34_all_zero` |

`R*` = row 10 is exercised on the fixed literal table (`"other"` never matches
`"test"`), which every `memchra2` call executes; it is pinned by the
`matches * 5` contribution in the differential result.

## Sentinel summary (what "the same error" means)

| sentinel | meaning | functions |
|---|---|---|
| `-1` | invalid buffer / empty data set | `process_buffer`, `complex_iteration` |
| `0` | null / empty / too-short input treated as "nothing to do" | `process_strings`, `safe_sum_array`, `interpret_as_int`, `count_occurrences`, `memchra` |
| `continue` | per-element skip, not a whole-call failure | `process_strings` |
| branch not taken | contribution omitted from `result` | `memchra2` float guard, `memchra2` `buf_sum` guard |

Note the deliberate asymmetry the tests must preserve: `count_occurrences`
returns **`0`** for an empty string while `process_buffer` and
`complex_iteration` return **`-1`** for the analogous condition. This is not
"fixed" in the Rust translation.

## Status — every row is checked off

All 34 rows have a passing differential test in `tests/errors.rs`
(`cargo test --features test_internals --test errors` → **35 passed, 0 failed**;
the 35th is the bonus bitwise check `err_row_extra_int_to_float_bits_bitwise`,
which compares `int_to_float_bits` bit-for-bit over 200 000 random `int`s plus
14 pinned bit patterns).

How the rows are driven across the FFI boundary:

* Rows **23-28, 30-34** go through the exported `memchra2` symbol of both `.so`s.
* Rows **1-22 and 29** concern the `static` helpers. They are driven through
  `harness_*` C-ABI wrappers:
  * C side — `tests/c_harness/harness.c`, which `#include`s the **unmodified**
    `c_src/src/lib.c` and adds external-linkage forwarders (nothing in `c_src/`
    is touched);
  * Rust side — the `#[cfg(feature = "test_internals")]` exports at the bottom of
    `src/lib.rs`, which are **off by default** so the shipped Rust `.so` exports
    exactly the same symbol set as the C `.so`.
  Both are loaded with `libloading` + `dlsym`; no Rust function is ever called
  directly.

Generic FFI boundaries additionally covered:

| boundary | where |
|---|---|
| NULL pointer in every pointer parameter | rows 1, 6, 8, 12, 15, 17, 19, 21 |
| zero length / count | rows 3, 7, 13, 16, 19, 22 |
| oversized length (far beyond the data) | rows 4 (`len = 4096`), 12/15/21 (`1<<20`), 16 (`len = 64` on a 4-byte payload) |
| one step past the valid minimum | row 16 (`len = 3` vs `sizeof(int) = 4`), row 7 (`count = 0`, `-1`), row 24 (`a = 0x447A0000`, one step past `f < 1000`) |
| out-of-range "enum-like" integer across FFI | rows 20 (needle `int` outside `char` range, incl. `INT_MIN`/`INT_MAX` and 5 000 random needles) and 33 (`lib.h` declares no enums; all four parameters are plain `int`, so every 32-bit value is an in-domain input and is exercised) |
| negative `int` where a count is expected | row 7 (`count = INT_MIN`, `INT_MIN+1`, `-2`, `-1`) |
