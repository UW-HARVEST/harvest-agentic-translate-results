# Error surface

Mechanically derived from `../c_src/src/lib.c`. The source has one `abort()`
site guarded by two independently triggerable conditions and has no
`RETURN_ERROR`, `return -1`, `return NULL`, assertions, enums, or explicit
pointer-null checks.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `bin2hex` | `bin_len >= SIZE_MAX / 2` (the source spells `SIZE_MAX` as `18446744073709551615UL`) | process terminates with `SIGABRT` | [x] |
| 2 | `bin2hex` | `bin_len < SIZE_MAX / 2` and `hex_maxlen == bin_len * 2` | process terminates with `SIGABRT` | [x] |
| 3 | `bin2hex` | `bin_len < SIZE_MAX / 2` and `hex_maxlen < bin_len * 2` | process terminates with `SIGABRT` | [x] |

## Mandatory generic FFI boundaries

These are required boundary probes, not invented source checks. Where the C
source dereferences a null pointer, the observable ground-truth result on the
test platform is process termination with `SIGSEGV`.

| # | function | boundary input | expected C result | verified |
|---|----------|----------------|-------------------|----------|
| G1 | `bin2hex` | `hex == NULL`, `bin_len == 0`, valid `hex_maxlen` | process terminates with `SIGSEGV` while writing the terminator | [x] |
| G2 | `bin2hex` | `bin == NULL`, `bin_len == 1`, valid output buffer/capacity | process terminates with `SIGSEGV` while reading input | [x] |
| G3 | `bin2hex` | `bin == NULL`, `bin_len == 0`, valid output buffer/capacity | succeeds; null input is not dereferenced | [x] |
| G4 | `bin2hex` | zero length with minimum valid capacity (`hex_maxlen == 1`) | succeeds and writes one NUL byte | [x] |
| G5 | `bin2hex` | `bin_len == SIZE_MAX / 2 - 1`, `hex_maxlen == 0` | process terminates with `SIGABRT` via the capacity condition before dereferencing | [x] |
| G6 | `bin2hex` | `bin_len == SIZE_MAX`, any capacity/pointers | process terminates with `SIGABRT` via the oversized-length condition | [x] |

There are no C enum parameters, so an out-of-range enum probe is not
applicable.
