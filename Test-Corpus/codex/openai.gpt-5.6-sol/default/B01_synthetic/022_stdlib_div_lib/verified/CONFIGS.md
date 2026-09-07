# Configuration Surface

The public surface has one entry point, no runtime options, no flags, no
compile-time Cargo features, and no C conditional branches. The meaningful
valid input configurations are the sign, exactness, zero, and representable
boundary classes distinguished by C signed integer division.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | no options; zero numerator, nonzero denominator | [x] |
| 2 | `driver` | no options; positive numerator and positive denominator, exact division | [x] |
| 3 | `driver` | no options; positive numerator and positive denominator, nonzero remainder | [x] |
| 4 | `driver` | no options; positive numerator and negative denominator, exact division | [x] |
| 5 | `driver` | no options; positive numerator and negative denominator, nonzero remainder | [x] |
| 6 | `driver` | no options; negative numerator and positive denominator, exact division | [x] |
| 7 | `driver` | no options; negative numerator and positive denominator, nonzero remainder | [x] |
| 8 | `driver` | no options; negative numerator and negative denominator, exact division | [x] |
| 9 | `driver` | no options; negative numerator and negative denominator, nonzero remainder | [x] |
| 10 | `driver` | no options; representable `int` boundary values (`INT_MIN`/`INT_MAX`) excluding `INT_MIN / -1` | [x] |

Feature/build combinations to run:

- default feature set (empty)
- `--no-default-features` (also empty, independently exercised)

There is no binary target in either build definition.

Each row passes 128 fixed-seed randomized inputs through both shared-library
FFI boundaries under both listed feature/build combinations.
