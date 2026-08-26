# Error Surface

Derived from `c_src/src/lib.c`. The public API has no error return value: both
explicit rejection conditions terminate the process with `abort()`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `bin2hex` | `bin_len >= SIZE_MAX / 2` (the source spells `SIZE_MAX` as `18446744073709551615UL`) | [x] Calls `abort()`; process exits via `SIGABRT` |
| 2 | `bin2hex` | `bin_len < SIZE_MAX / 2 && hex_maxlen <= bin_len * 2` | [x] Calls `abort()`; process exits via `SIGABRT` |

## Generic FFI boundaries

The C source has no explicit pointer checks or enum parameters. Differential
coverage must additionally compare null output pointers, null input pointers,
zero lengths, the exact valid capacity boundary, and lengths at and above the
explicit maximum.
