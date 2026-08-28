# Error Surface

Mechanical review covered every `return`, `if`, loop bound, assertion pattern,
null token, preprocessor condition, enum token, and min/max token in
`c_src/src/lib.c` and `c_src/include/lib.h`.

The C implementation has no error-return macro, `return -1`, `return NULL`,
error enum, assertion, null check, or allocation-failure check. Its only
explicit rejection-style checks are the false sides of two range predicates.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `shift_array_data` | `shift_by <= 0` (false side of `shift_by > 0`) | Return `void`; leave all array bytes unchanged. | [x] |
| 2 | `shift_array_data` | `shift_by >= size` (false side of `shift_by < size`) | Return `void`; leave all array bytes unchanged. | [x] |
| 3 | `manipulate_records` | `shift <= 0` (false side of `shift > 0`) | Skip `memmove`; sum indices `0..(num_records - shift)` and return that exact integer total. | [x] |
| 4 | `manipulate_records` | `shift >= num_records` (false side of `shift < num_records`) | Skip `memmove`; for `shift >= num_records`, execute zero sum iterations and return `0`. | [x] |

Unchecked-pointer behavior is not represented as an error row because C does
not reject it. Phase C separately compares process outcomes for null callback
and data pointers, and covers zero and out-of-range lengths.
