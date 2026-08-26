# ERRORS.md — error / rejection surface of `c_src/src/lib.c`

Derived mechanically by grepping every `return -1`, `return 0` guard, `continue`,
`== NULL`, `== '\0'`, `<= 0`, `== 0`, `< sizeof(...)` and every conditional that
suppresses a contribution in `c_src/src/lib.c`. There are no `assert`s, no error
enums and no `RETURN_ERROR`-style macros in this library; rejection is expressed
with sentinel return values (`-1`, `0`), with `continue`, and with skipped
`if` bodies.

Differential test file: `tests/error_paths.rs` (needs `--features internal_test_api`
for the `static` helpers), plus `tests/differential.rs` for the public entry point.

| #  | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|----|----------|------------------------------------------|-------------------|------|-----|
| 1  | `process_buffer`    | `buffer == NULL` (len arbitrary, incl. 0 and huge)          | returns `-1`                                | `err01_process_buffer_null` | [x] |
| 2  | `process_buffer`    | `*buffer == '\0'` (non-NULL pointer to empty string)        | returns `-1`                                | `err02_process_buffer_empty` | [x] |
| 3  | `process_buffer`    | `len == 0`, buffer non-empty (guard passes, loop body never runs) | returns `0`                            | `err03_process_buffer_zero_len` | [x] |
| 4  | `process_buffer`    | interior `'\0'` before `buffer+len` (loop stops early)      | sum of bytes before the NUL only            | `err04_process_buffer_interior_nul` | [x] |
| 5  | `process_strings`   | `strings == NULL`                                           | returns `0`                                 | `err05_process_strings_null_array` | [x] |
| 6  | `process_strings`   | `count == 0`                                                | returns `0`                                 | `err06_process_strings_count_zero` | [x] |
| 7  | `process_strings`   | `count < 0` (e.g. `-1`, `INT_MIN`)                          | returns `0`                                 | `err07_process_strings_count_negative` | [x] |
| 8  | `process_strings`   | element `strings[i] == NULL`                                | that element is skipped (`continue`), others still counted | `err08_process_strings_null_element` | [x] |
| 9  | `process_strings`   | element `*strings[i] == '\0'` (empty string element)        | that element is skipped (`continue`)        | `err09_process_strings_empty_element` | [x] |
| 10 | `process_strings`   | `strlen(target) == 0` (empty target) — `strncmp(...,0)` is 0 | every non-NULL/non-empty element matches   | `err10_process_strings_empty_target` | [x] |
| 11 | `process_strings`   | element shorter than `target` (`strncmp` hits element NUL first) | no match for that element               | `err11_process_strings_short_element` | [x] |
| 12 | `safe_sum_array`    | `arr == NULL`                                               | returns `0`                                 | `err12_safe_sum_null` | [x] |
| 13 | `safe_sum_array`    | `size == 0` (non-NULL arr)                                  | returns `0`                                 | `err13_safe_sum_zero_size` | [x] |
| 14 | `interpret_as_int`  | `bytes == NULL`                                             | returns `0`                                 | `err14_interpret_null` | [x] |
| 15 | `interpret_as_int`  | `len < sizeof(int)` → `len ∈ {0,1,2,3}`                     | returns `0`                                 | `err15_interpret_short_len` | [x] |
| 16 | `interpret_as_int`  | `len == sizeof(int)` exactly (boundary, one step inside)     | returns the 4 bytes as a native-endian `int`| `err16_interpret_len_boundary` | [x] |
| 17 | `count_occurrences` | `text == NULL`                                              | returns `0`                                 | `err17_count_null` | [x] |
| 18 | `count_occurrences` | `*text == '\0'` (empty string)                              | returns `0`                                 | `err18_count_empty` | [x] |
| 19 | `count_occurrences` | `ch == '\0'` on a non-empty string (needle is the terminator, never inside `[0,strlen)`) | returns `0` | `err19_count_nul_needle` | [x] |
| 20 | `complex_iteration` | `data == NULL`                                              | returns `-1`                                | `err20_complex_null` | [x] |
| 21 | `complex_iteration` | `count == 0` (non-NULL data)                                | returns `-1`                                | `err21_complex_zero_count` | [x] |
| 22 | `complex_iteration` | valid input whose XOR result is legitimately `-1`-ambiguous? impossible: XOR of `u & 0xFF` ∈ `[0,255]`, so `-1` is unambiguously the error sentinel | result always in `[0,255]` for valid input | `err22_complex_result_range` | [x] |
| 23 | `memchra`           | `n == 0` (loop never executes)                              | returns `0`                                 | `err23_memchra_zero_n` | [x] |
| 24 | `memchra`           | needle absent from `str[0..n]`                              | returns `0`                                 | `err24_memchra_absent` | [x] |
| 25 | `memchra`           | `c` outside `char` range (e.g. `0x141`, `256`, `-1`, `INT_MIN`, `INT_MAX`): `(char)c` narrows | matches on the truncated low byte, so `c=0x141` matches `'A'` | `err25_memchra_out_of_char_range` | [x] |
| 26 | `memchra2`          | `f = int_to_float_bits(a)` is `<= 0.0f` (a == 0, a < 0 i.e. sign bit set) → `if` body skipped | no `(int)f` contribution | `err26_memchra2_float_nonpositive` | [x] |
| 27 | `memchra2`          | `f >= 1000.0f` (e.g. `a = 0x447A0000`, `a = INT_MAX`, `a = 0x7F800000` = `+inf`) → skipped | no `(int)f` contribution | `err27_memchra2_float_too_big` | [x] |
| 28 | `memchra2`          | `f` is NaN (`a = 0x7FC00000`, `a = 0x7FFFFFFF`): both `>` and `<` are false → skipped | no `(int)f` contribution | `err28_memchra2_float_nan` | [x] |
| 29 | `memchra2`          | `buf_sum <= 0` — unreachable in practice (formatted buffer is printable ASCII so the sum is always > 0); the guard is still asserted to be taken for every input | `buf_sum % 256` always added | `err29_memchra2_bufsum_positive` | [x] |
| 30 | `memchra2`          | extreme/boundary arguments: `INT_MIN`, `INT_MAX`, `0`, `-1` in every position (longest `%d` expansion, 51 bytes < 63 → `snprintf` never truncates) | well-defined `int`, no truncation | `err30_memchra2_extreme_args` | [x] |
| 31 | `memchra2`          | out-of-range "enum-like" ints across FFI: every argument is a plain `int`, so *all* 2^32 bit patterns are valid input; no argument validation exists at all | never rejects, always returns a value | `err31_memchra2_no_rejection` | [x] |

## Boundaries that are **unchecked** in the C (documented, deliberately not executed)

These are inputs for which the C performs *no* check and would invoke undefined
behaviour / crash. They are not differentially testable (the C process dies), so
they are listed for completeness rather than tested. The Rust translation
reproduces the same *absence* of a check, so it fails the same way.

| function | unchecked input | C behaviour |
|---|---|---|
| `process_strings` | `target == NULL` | `strlen(NULL)` → SIGSEGV (no null check on `target`) |
| `count_occurrences` | `text` not NUL-terminated | `strlen` runs off the end |
| `memchra` | `n` larger than the real buffer | reads out of bounds |
| `process_buffer` | `len` larger than the real buffer and no interior NUL | reads out of bounds |
| `safe_sum_array` / `complex_iteration` | `size`/`count` larger than the real array | reads out of bounds |
| `safe_sum_array` | signed overflow of `sum` | UB; gcc wraps two's-complement — Rust uses `wrapping_add` to match |
| `interpret_as_int` | misaligned `bytes` | UB; x86-64 gcc emits a plain load — Rust uses `read_unaligned` to match |
