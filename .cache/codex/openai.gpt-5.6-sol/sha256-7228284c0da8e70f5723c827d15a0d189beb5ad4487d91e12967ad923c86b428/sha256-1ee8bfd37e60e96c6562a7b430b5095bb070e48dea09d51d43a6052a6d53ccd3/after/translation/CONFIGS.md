# Configuration Surface

The public header declares one entry point, `driver(int x, int y)`. There are
no runtime options, flags, modes, element types, formats, byte-order choices,
compile-time feature branches, or lower-level public entry points. The input
shape is one pair of C `int` values. The C operation distinguishes quotient
sign, exact versus non-exact division, remainder sign (the numerator's sign),
zero, and the representable integer boundaries.

Every row excludes the invalid pairs listed in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x == 0`, `y > 0` | [x] |
| 2 | `driver` | `x == 0`, `y < 0` | [x] |
| 3 | `driver` | `x > 0`, `y > 0`, exact division (`x % y == 0`) | [x] |
| 4 | `driver` | `x > 0`, `y > 0`, nonzero remainder | [x] |
| 5 | `driver` | `x > 0`, `y < 0`, exact division | [x] |
| 6 | `driver` | `x > 0`, `y < 0`, nonzero remainder | [x] |
| 7 | `driver` | `x < 0`, `y > 0`, exact division | [x] |
| 8 | `driver` | `x < 0`, `y > 0`, nonzero remainder | [x] |
| 9 | `driver` | `x < 0`, `y < 0`, exact division | [x] |
| 10 | `driver` | `x < 0`, `y < 0`, nonzero remainder | [x] |
| 11 | `driver` | `x == INT_MIN`, valid `y` (`y != 0 && y != -1`) | [x] |
| 12 | `driver` | `x == INT_MAX`, nonzero `y` | [x] |
| 13 | `driver` | `y == INT_MIN`, any `x` | [x] |
| 14 | `driver` | `y == INT_MAX`, any `x` | [x] |
