# Error Surface

Mechanical searches covered `RETURN_ERROR`, `return -1`, `return NULL`,
assertions, null checks, enums, range checks, and all `if`/`switch` statements
in `c_src/include` and `c_src/src`.

| # | function | trigger (the exact invalid input/condition) | expected C result | [ ] |
|---|----------|----------------------------------------------|-------------------|-----|

There are no rejection or error paths in the C API. `pow43` takes a scalar
`int`, so pointer, length, and enum boundary cases do not apply.

Phase C status: [x] complete (zero applicable rejection rows).

The table access is defined for `-16 <= x <= 8223`. Inputs outside that range
perform an out-of-bounds C array access (and extreme positive inputs can also
overflow signed arithmetic), which is undefined behavior rather than an error
result. Differential tests must not assign an expected value to undefined C
behavior.
