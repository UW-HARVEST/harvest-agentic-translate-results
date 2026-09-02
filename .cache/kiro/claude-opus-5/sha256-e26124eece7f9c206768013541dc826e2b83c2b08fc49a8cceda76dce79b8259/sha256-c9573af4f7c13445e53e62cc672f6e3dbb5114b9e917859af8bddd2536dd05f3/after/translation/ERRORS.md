# ERRORS.md — Error / rejection surface table (Phase A)

Derived mechanically from `c_src/src/lib.c`. The C code contains **no**
`assert`, no `errno` use, no error enum, no allocation, and no null checks;
its entire rejection surface consists of the sentinel `return -1`, the
guard branches, and the loop bounds. Every row below corresponds to a
concrete branch in the source (line numbers from `c_src/src/lib.c`).

Rows are checked off when a differential test in
`translation/tests/differential.rs` constructs that exact condition, calls
BOTH `.so`s, and asserts the SAME sentinel / value.

| #  | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|----|----------|---------------------------------------------|-------------------|------|-----|
| 1  | `find_value_in_buffer` | `memchr` returns `NULL` — byte not present in the first `size` bytes (line 36 false → line 39) | returns `-1` | `err_find_not_found` | [x] |
| 2  | `find_value_in_buffer` | `size == 0` (empty range; `memchr` must not read) with a byte value that *is* present just past the end | returns `-1` | `err_find_zero_size` | [x] |
| 3  | `find_value_in_buffer` | `size == 0` **and** `buffer == NULL` (null pointer, no read performed) | returns `-1` | `err_find_null_zero_size` | [x] |
| 4  | `find_value_in_buffer` | `search_val` outside `char` range — narrowed by `(char)search_val` (line 33), so e.g. `0x1FF`, `-1`, `255`, `511` all search for the *same* low byte | index of the low byte, or `-1` | `err_find_search_val_narrowing` | [x] |
| 5  | `find_value_in_buffer` | `search_val` low byte `== 0` (searching for the NUL terminator, not a "no value" sentinel) | index of first `0x00`, or `-1` | `err_find_nul_byte` | [x] |
| 6  | `find_value_in_buffer` | `search_val == INT_MIN` / `INT_MAX` (one step past every documented range) | low byte `0x00` / `0xFF` search result | `err_find_search_val_extremes` | [x] |
| 7  | `find_value_in_buffer` | oversized `size` relative to the real allocation is UB in C; the *defined* boundary is `size == allocation length` with the target absent | returns `-1` | `err_find_size_equals_len` | [x] |
| 8  | `create_numeric_buffer` | `size == 0` — loop body never runs (line 49), buffer untouched, no write through the pointer | returns `void`, buffer unmodified | `err_create_zero_size` | [x] |
| 9  | `create_numeric_buffer` | `size < 0` (e.g. `-1`, `INT_MIN`) — `i < size` false immediately, buffer untouched | returns `void`, buffer unmodified | `err_create_negative_size` | [x] |
| 10 | `create_numeric_buffer` | `size == 0` **and** `buffer == NULL` — must not dereference | returns `void`, no crash | `err_create_null_zero_size` | [x] |
| 11 | `create_numeric_buffer` | `size < 0` **and** `buffer == NULL` — must not dereference | returns `void`, no crash | `err_create_null_negative_size` | [x] |
| 12 | `create_numeric_buffer` | `seed + i * 7` overflows `int` (signed-overflow UB; `seed` near `INT_MAX`/`INT_MIN`) | wrapped value, `%` truncating toward zero → negative bytes for negative intermediates | `err_create_seed_overflow` | [x] |
| 13 | `calculate_with_doubles` | `b == 0` — the division is *skipped* (line 57), `result` stays `0.0`, but the `pow` multiply still runs (line 61) | `0.0 * pow(10, c%10)`, i.e. `0.0` (or `NaN` never occurs since `pow` is finite here) | `err_calc_b_zero` | [x] |
| 14 | `calculate_with_doubles` | `b == -1` with `a == INT_MIN` — the *integer* division that would trap is not performed (both are widened to `double` first) | `2147483648.0 * pow(10, c%10)` | `err_calc_intmin_div_minus1` | [x] |
| 15 | `calculate_with_doubles` | `c < 0` — `c % 10` truncates toward zero, so the exponent is **negative** (`pow(10, -k)`) | value scaled down by `10^-k` | `err_calc_negative_exponent` | [x] |
| 16 | `calculate_with_doubles` | `c == INT_MIN` / `INT_MAX` — `INT_MIN % 10 == -8`, `INT_MAX % 10 == 7` | exponent `-8` / `7` | `err_calc_c_extremes` | [x] |
| 17 | `convert_double_to_int` | value out of `int` range after truncation (`> INT_MAX`, `< INT_MIN`, e.g. `-2^40`, `1e300`) — UB; x86-64 `cvttsd2si` yields the integer-indefinite value | returns `INT_MIN` (`0x80000000`) | `err_conv_out_of_range` | [x] |
| 18 | `convert_double_to_int` | `+INFINITY` / `-INFINITY` — UB | returns `INT_MIN` | `err_conv_infinities` | [x] |
| 19 | `convert_double_to_int` | `NaN` (quiet **and** signalling, both signs) — UB | returns `INT_MIN` | `err_conv_nan` | [x] |
| 20 | `convert_double_to_int` | exactly one step past the valid range: `2147483648.0` and `-2147483649.0`, vs. the in-range `2147483647.x` / `-2147483648.x` | `INT_MIN` for the two out-of-range, exact truncation for the two in-range | `err_conv_boundaries` | [x] |
| 21 | `doubleneg` | `pos < 0` path — a searched value absent from the buffer, so line 112 is false and `result` is **not** incremented (the "Value %d not found" branch). **Proven UNREACHABLE from `doubleneg`**: it always fills 256 bytes with `(char)((param1 + 7i) % 256)`, and since C's `%` keeps the value congruent mod 256 and 7 is invertible mod 256, those bytes are a permutation of all 256 values for *every* `param1` (signed overflow included, because 256 divides 2³²). The differential requirement is that both implementations agree the branch is never taken; the reachable form of the rejection is exercised on `find_value_in_buffer` with `size < 256`. | branch never taken; every search reports "Found value"; `-1` sentinel reachable only for `size < 256` | `doubleneg_error_paths` (rows21/22 block) | [x] |
| 22 | `doubleneg` | `direct_search == NULL` — byte `100` absent from the generated buffer (line 121 false). **UNREACHABLE for the same reason as row 21**; asserted to be unreachable in BOTH implementations, and the NULL-returning form is covered on the primitive. | branch never taken; the "Direct memchr" line always appears | `doubleneg_error_paths` (rows21/22 block) | [x] |
| 23 | `doubleneg` | `param2 == 0` — feeds `b == 0` into `calculate_with_doubles` (row 13) *and* makes every `search_byte` in the combined loop equal `param1 % 256` | identical result & stdout | `doubleneg_error_paths` (row23 block) | [x] |
| 24 | `doubleneg` | all params `INT_MIN` / `INT_MAX` — signed overflow in `param1 + i*param2`, `% 256` on negatives, `% 1000` on `INT_MIN` (`-648`) | identical wrapped result & stdout | `doubleneg_error_paths` (row24 block) | [x] |
| 25 | *(whole API)* | out-of-range "enum" values across the FFI boundary — the API declares **no enum type**; the equivalent inputs are arbitrary `int`s in positions the C narrows or reduces (`search_val`, `size`, `seed`, `c`). Rows 4, 6, 9, 12, 16 cover these; this row records the sweep of `INT_MIN`, `-1`, `0`, `1`, `255`, `256`, `INT_MAX` through every `int` parameter of every entry point. | identical to C for every value | `err_int_parameter_sweep`, `doubleneg_error_paths` (sweep block) | [x] |
| 26 | `process_negation` | there is no rejection path at all: every `int` including `INT_MIN`, `INT_MAX`, `-1` maps to `0`/`1` (never an error) | `0` iff input `== 0`, else `1` | `err_process_negation_total` | [x] |
