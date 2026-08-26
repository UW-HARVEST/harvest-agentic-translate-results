# Error Surface

Mechanical search covered `return`, `NULL`, `if`, `switch`, `case`, `assert`,
error macros, and range constants in `c_src/src/main.c`. The source has no
error-return macro, `return -1`, `return NULL`, assertion, enum, length
parameter, or range/min/max check. Its only explicit invalid-input guard is:

| # | function | trigger (the exact invalid input/condition) | expected C result | Status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `printLine` | `line == NULL` | Return `void` without writing any bytes to stdout | [x] `differential_surface_matches` |

Generic FFI boundaries:

- `main` does not dereference `argv`; `argv == NULL` is accepted and returns
  `0`, including with zero, negative, and maximum `argc`. Covered by
  `differential_surface_matches`.
- No API accepts a length, enum, or bounded numeric option, so zero/oversized
  lengths, one-past-range values, and out-of-range enums are not applicable.
