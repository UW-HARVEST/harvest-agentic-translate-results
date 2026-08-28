# Error Surface

Mechanical searches covered every `return`, `assert`, conditional, pointer
operation, and numeric threshold in `../c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|---------------------------------------------|-------------------|-----|

There are no explicit input-rejection branches in the C implementation: no
error code or sentinel return, `assert`, null check, enum, or length argument.
In particular, `divide_multiplier(_, 0)` is an explicitly accepted no-op on
the multiplier, not an error.

## Generic FFI Boundary Inventory

| Boundary class | Applicable C entry point(s) | C behavior | Status |
|----------------|-----------------------------|------------|--------|
| Null pointer | `process_octal_string(NULL, value)` | Undefined behavior; this build terminates by signal | [x] |
| Null pointer | `find_and_replace_char(NULL, value)` | Undefined behavior; this build terminates by signal | [x] |
| Zero length | None | No public API accepts a length | N/A |
| Oversized length | None | No public API accepts a length | N/A |
| Out-of-range enum | None | No public API accepts an enum | N/A |
| One past documented range | `validate_and_normalize(63)`, `validate_and_normalize(512)` | Clamps to 64 and 511 respectively; valid-path rows C13 and C17 | [x] |
