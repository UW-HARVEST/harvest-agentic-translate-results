# Error surface

Mechanical searches covered `c_src/include/lib.h`, `c_src/src/lib.c`, and
`c_src/CMakeLists.txt` for error-return statements/macros, assertions, null
checks, range checks, enums, and min/max constants.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|

There are no rows because the C API has no rejection or error path. Its only
parameter is an `int`; there are no pointers, lengths, enums, error codes,
sentinels, assertions, or explicit validity checks.

Phase C status: **complete (zero applicable rejection rows)**.

The table lookup itself is in bounds for `x` in `-16..=8223`. Inputs `x < -16`
or `x > 8223` are not rejected: the C source indexes outside `g_pow43`, making
the result undefined rather than a defined error result. Such inputs cannot
have a portable byte-identical error assertion and are not represented as
invented rejection rows.

Generic FFI boundary applicability:

| boundary class | applicability |
|----------------|---------------|
| null pointer | not applicable; no pointer parameters |
| zero length | not applicable; no length parameters (`x = 0` is valid) |
| oversized length | not applicable; no length parameters |
| out-of-range enum | not applicable; no enum parameters |
| one past documented range | not applicable; the public header documents no range |
