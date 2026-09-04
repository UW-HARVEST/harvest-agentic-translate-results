# Configuration Surface

There are no runtime options, modes, flags, element types, widths, byte-order
choices, format choices, or Cargo features. The public dynamic-symbol surface
contains the low-level `foo` entry point and the header-declared `driver`
entry point.

For `foo`, the loop distinguishes zero, one, and multiple matches. Tests use
random non-NUL target bytes (including high-bit bytes) and randomized
NUL-terminated byte strings. A NUL target is excluded because the C loop
advances beyond the string terminator and subsequent access has undefined
behavior.

For `driver`, the two independent `foo` calls produce the mechanically derived
cross-product of zero, one, and multiple occurrences of `A` and `x`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `foo` | no options; target occurs zero times (empty and nonempty inputs included) | [x] |
| 2 | `foo` | no options; target occurs exactly once | [x] |
| 3 | `foo` | no options; target occurs multiple times | [x] |
| 4 | `driver` | no options; `A` count zero, `x` count zero | [x] |
| 5 | `driver` | no options; `A` count zero, `x` count one | [x] |
| 6 | `driver` | no options; `A` count zero, `x` count multiple | [x] |
| 7 | `driver` | no options; `A` count one, `x` count zero | [x] |
| 8 | `driver` | no options; `A` count one, `x` count one | [x] |
| 9 | `driver` | no options; `A` count one, `x` count multiple | [x] |
| 10 | `driver` | no options; `A` count multiple, `x` count zero | [x] |
| 11 | `driver` | no options; `A` count multiple, `x` count one | [x] |
| 12 | `driver` | no options; `A` count multiple, `x` count multiple | [x] |
