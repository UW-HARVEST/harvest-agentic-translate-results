# Error Surface

Rows are derived from every rejecting guard, allocation check, and explicit
range check in `c_src/src/lib.c`. The C source has no `assert`, error enum,
explicit pointer-null input check, or public min/max constant.

| # | function | trigger (the exact invalid input/condition) | expected C result | tested |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `shift_array` | `positions <= 0`, so `positions > 0 && positions < size` is false | returns `void`; `arr` is not accessed and remains byte-identical | [x] |
| 2 | `shift_array` | `positions > 0 && positions >= size`, so `positions > 0 && positions < size` is false | returns `void`; `arr` is not accessed and remains byte-identical | [x] |
| 3 | `compare_allocations` | `malloc(sizeof(int))` returns `NULL` for either `ptr1` or `ptr2` | frees both values (including `NULL`) and returns `-1` | [x] |
| 4 | `arity` | the implementation's `unsigned char len` is less than 2; for an ABI `int`, its low byte is `0` or `1` | returns `-1` without accessing `params` | [x] |

Unchecked rows: **0**.

