# Error surface

The mechanical scan covered `c_src/include/` and `c_src/src/` for error-return
statements/macros, assertions, conditionals, switches, null checks, range
constants, and enums. The C API contains no rejection or error path:
`driver(int)` returns `void`, accepts the full C `int` type, and has no pointer,
length, enum, option, or error-code parameters.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

Generic FFI error-boundary applicability:

- Null pointers: not applicable; there are no pointer parameters.
- Zero lengths: not applicable; there are no length parameters (`x == 0` is a
  valid no-output configuration covered in `CONFIGS.md`).
- Oversized lengths: not applicable; there are no length parameters.
- One-past-range values: not applicable; no restricted/documented range exists
  within the `int` parameter type.
- Out-of-range enum values: not applicable; there are no enum parameters.
