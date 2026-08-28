# Configuration Surface

Mechanically derived from the sole declaration in `c_src/include/pow.h` and
the branches in `c_src/src/pow.c`.

Axes:

- Entry points: `my_pow` only; there are no lower-level public entry points.
- Runtime options/modes/flags: none.
- Compile-time feature combinations: none in `Cargo.toml`.
- Input shape: two by-value C `double` scalars. There are no pointers, lengths,
  element types, formats, byte-order choices, or state objects.
- Valid-path branch: `pow` leaves `errno` as neither `EDOM` nor `ERANGE`;
  `my_pow` returns the `pow` result unchanged. This includes ordinary finite
  results and successful IEEE-754 special-value cases (signed zero, infinity,
  and NaN).

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `my_pow` | no options; randomized pairs of all `double` bit patterns for which C takes the successful return branch, including finite, signed-zero, infinity, and NaN cases | [x] |
