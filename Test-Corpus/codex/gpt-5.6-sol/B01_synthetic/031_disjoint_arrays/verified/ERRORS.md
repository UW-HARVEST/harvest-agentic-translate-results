# Error Surface

Mechanical search covered every `if`, `return`, `assert`, null/range check,
error macro, and min/max constant in `c_src/src/main.c`. The C source contains
no asserts, error macros, error enums, pointer validation, or explicit integer
range validation. Its only input-rejection branch is the `scanf` result check
at line 52. The two possible rejecting results are listed separately because
they arise from distinct input conditions.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `main` | `scanf("%d", &data[i])` returns `EOF` because input ends or an input failure occurs before a conversion | stop scanning, call `call_fma` with the number already converted, print its result, return `0` | [x] |
| 2 | `main` | `scanf("%d", &data[i])` returns `0` because the next non-whitespace bytes do not match `%d` | leave the bad bytes unread, stop scanning, call `call_fma` with the number already converted, print its result, return `0` | [x] |

Generic FFI boundary audit:

| Boundary | C behavior |
|----------|------------|
| null pointers with `fma_array` length `<= 0` | accepted; loop body is not entered |
| null `data` with `call_fma` length `0` | accepted; returns `0` before dereference |
| null pointers with a positive accessed length | no check; undefined behavior, so there is no C rejection result to compare |
| negative `fma_array` length | accepted; loop body is not entered |
| negative `call_fma` length | no check; invalid VLA bounds and subsequent access produce undefined behavior |
| oversized positive lengths | no check; behavior depends on whether the C VLA allocation succeeds |
| out-of-range enum values | not applicable; the public API has no enums |
| integer text outside the `int` range | no explicit rejection; `%d` conversion behavior is supplied by the platform C library |
