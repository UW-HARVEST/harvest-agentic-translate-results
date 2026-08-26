# Configuration-Surface Table

`Cargo.toml` defines no features and `c_src/CMakeLists.txt` defines no options,
preprocessor configurations, or alternate sources. The only build-time
configuration is the empty feature set (`--no-default-features`).

The C source has no runtime mode flags. Its public surface is the global
`array`, `perform_expensive_operations`, and `main`. The rows below enumerate
the input classes distinguished by the arithmetic, `strtoul` acceptance rules,
fixed sizes, and public call hierarchy.

| # | entry point(s) | configuration (options set + input shape) | verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `array` | freshly loaded object: fixed 262,144-element `int` array, all zero-initialized | [x] |
| 2 | `array` | direct external write/read of arbitrary negative, zero, positive, `INT_MIN`, and `INT_MAX` elements | [x] |
| 3 | `perform_expensive_operations` + `array` | fixed full array; randomized nonnegative values including `0` and `INT_MAX` | [x] |
| 4 | `perform_expensive_operations` + `array` | fixed full array; randomized negative values including `-1` and `INT_MIN` | [x] |
| 5 | `perform_expensive_operations` + `array` | fixed full array; randomized mixed-sign values exercising wrapping arithmetic, signed right shift, division, and remainder | [x] |
| 6 | `main` | `argc == 2`; canonical decimal seed in `0..=UINT_MAX`, including both boundaries; initializes the fixed array, applies 2,000 operations, and XOR-reduces it | [x] |
| 7 | `main` | `argc == 2`; accepted decimal with leading whitespace and/or `+`, value in `0..=UINT_MAX` | [x] |
| 8 | `main` | `argc == 2`; empty seed string (no conversion, but C accepts it as seed `0` because `*endptr == '\0'`) | [x] |
| 9 | `main` | `argc == 2`; negative-zero spelling such as `-0`, accepted as seed `0` | [x] |

Invalid `argc`, trailing characters, conversion overflow, `UINT_MAX` overflow,
and null pointers are enumerated in `ERRORS.md`.
