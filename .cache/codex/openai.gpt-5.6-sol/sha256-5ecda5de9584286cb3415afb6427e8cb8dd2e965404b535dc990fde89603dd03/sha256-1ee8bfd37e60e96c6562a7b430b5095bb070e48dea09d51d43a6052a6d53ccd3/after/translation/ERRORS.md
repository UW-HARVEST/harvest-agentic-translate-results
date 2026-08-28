# Error Surface

The mechanical search covered `return`, `assert`, `if`, `switch`, `NULL`,
`MIN`, `MAX`, enums, and error-like macros throughout `../c_src`. There are no
assertions, null checks, enum parameters, error enums, `RETURN_ERROR` uses,
`return -1`, or `return NULL` branches. The only input rejection is the
bit-limit check in `src/lib.c:7-8`; its two call sites expose distinct output
behavior.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `dequantize_granule` / direct `get_bits` call | `bitalloc[i]` is `1..16` and a sample read makes updated `bs.pos > bs.limit` | [x] `get_bits` returns `0`, leaves `bs.pos` advanced, and the sample is `-((1 << (bitalloc[i] - 1)) - 1)` |
| 2 | `dequantize_granule` / grouped `get_bits` call | `bitalloc[i]` is `17..21` and the one grouped-code read makes updated `bs.pos > bs.limit` | [x] `get_bits` returns `0`, leaves `bs.pos` advanced, and all decoded digits come from grouped code `0` |

The public API documents no recoverable error return: its return value is
always `group_size * 4`. Null pointers, `total_bands > 32`, `bitalloc > 21`,
and output buffers too small for the indexed writes are unchecked C
precondition violations with undefined behavior, not C rejection results.
