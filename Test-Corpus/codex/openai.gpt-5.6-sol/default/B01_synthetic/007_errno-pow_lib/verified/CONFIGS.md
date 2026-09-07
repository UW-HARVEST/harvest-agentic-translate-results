# Configuration Surface

Mechanically derived from the public header and all branches in
`../c_src/src/pow.c`.

The library exposes no runtime options, modes, flags, state objects, element
types, counts, formats, byte-order choices, pointers, lengths, or feature
switches. Its only public entry point takes two scalar `double` values. The
only C-side branch axis is the `errno` state produced by `pow`; its two error
states are tracked separately in `ERRORS.md`. Therefore the valid
configuration surface has one row, exercised with many value classes and
randomized finite inputs.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `my_pow` | No options; scalar `double base` and scalar `double exponent`; `pow` completes without setting `EDOM` or `ERANGE` (including ordinary finite values and valid zero, signed-zero, infinity, and NaN cases) | [x] |
