# Error-surface table

Mechanical source scan covered `return`, `assert`, `NULL`, `if`, comparison
operators, and octal min/max constants in `c_src/src/lib.c` and
`c_src/include/lib.h`.

The C API defines no error enum, error-return macro, `assert`, `return -1`, or
`return NULL`. It does not validate pointer arguments. The distinct input
rejections/suppressions it does implement are:

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `divide_multiplier` | divisor `b == 0` | skip division, increment `operation_count`, return the unchanged `multiplier` | [x] |
| 2 | `validate_and_normalize` | positive `value < 0100` (64), excluding zero | return lower bound `0100` (64) | [x] |
| 3 | `validate_and_normalize` | positive `value > 0777` (511) | return upper bound `0777` (511) | [x] |
| 4 | `process_octal_string` | `dest == NULL` (generic FFI pointer boundary; C has no check) | process terminates with the C library's native fault behavior | [x] |
| 5 | `find_and_replace_char` | `str == NULL` (generic FFI pointer boundary; C has no check) | process terminates with the C library's native fault behavior | [x] |
| 6 | `divide_multiplier` | current `multiplier == INT_MIN` and divisor `b == -1` | process terminates with the C library's native integer-division fault | [x] |

There are no length parameters, public enum parameters, or documented
one-past-enum values in this API. Zero and extreme integer values are valid
inputs and are covered in `CONFIGS.md`.
