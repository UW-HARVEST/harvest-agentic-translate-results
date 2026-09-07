# Error surface

The C source contains no `RETURN_ERROR`, `return -1`, `return NULL`, error
enum, `assert`, null check, allocation-failure check, or explicit error code.
The only explicit input rejections are the failed sides of the two range
guards below. These reject a requested move, not the whole function call.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `shift_array_data` | `shift_by <= 0`, so `shift_by > 0` is false | [x] Return `void`; do not read or modify `arr`. |
| E2 | `shift_array_data` | `shift_by >= size`, so `shift_by < size` is false | [x] Return `void`; do not read or modify `arr`. |
| E3 | `manipulate_records` | `shift <= 0`, so `shift > 0` is false | [x] Skip `memmove`; sum `records[0..num_records-shift)` and return that exact `int` total. |
| E4 | `manipulate_records` | `shift >= num_records`, so `shift < num_records` is false | [x] Skip `memmove`; the loop bound is non-positive, so return `0`. |

Unchecked FFI boundaries are tested separately because the C code does not
reject them: active null callback/pointer use terminates by signal; null
pointers on inactive/zero-length paths are accepted; zero, negative, and
oversized lengths follow the C loop/allocation expressions exactly. There are
no enum parameters in this API.
