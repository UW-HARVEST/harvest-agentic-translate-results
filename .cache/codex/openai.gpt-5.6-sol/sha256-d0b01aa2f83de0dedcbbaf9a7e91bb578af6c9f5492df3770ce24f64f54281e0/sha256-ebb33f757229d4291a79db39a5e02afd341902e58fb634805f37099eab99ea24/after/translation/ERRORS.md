# Error Surface

Mechanically derived from all checks and rejection-like constructs in
`../c_src/include/driver.h` and `../c_src/src/driver.c`. The source contains no
error-return macros, error enums, assertions, range checks, length checks, or
min/max constants. It has one explicit null-pointer check.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `printLine` | `line == NULL` | Return `void` without calling `printf`; produce zero stdout bytes. | [x] |

Generic FFI-boundary audit:

- `driver(int)` has no invalid integer or enum values: C treats zero as false
  and every nonzero `int` as true.
- No public function accepts a length, count, or enum.
- `bad()` has no caller-supplied input. Its uninitialized local pointer is a C
  execution behavior, not an input rejection, and is covered on the valid
  entry-point surface in `CONFIGS.md`.
