# Configuration Surface

Derived from the exported entry points and branches in
`c_src/src/driver.c:38-85`. There are no compile-time Cargo features, runtime
options, modes, flags, enums, byte-order choices, element types, or explicit
length arguments. The library is stateful, so call count and mixed entry-point
sequences are meaningful input shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `run` | One direct call with an arbitrary `int`, including negative, zero, positive, and boundary values | [x] |
| 2 | `run` | Many direct calls with randomized `int` values, exercising accumulated global house state | [x] |
| 3 | `driver` | Canonical base-10 C strings whose values are within `INT_MIN..=INT_MAX`; many randomized calls | [x] |
| 4 | `driver` | Valid base-10 values with leading ASCII whitespace and optional `+`/`-`; many randomized calls | [x] |
| 5 | `driver` | Valid numeric prefixes followed by nonnumeric suffix bytes; `endp != str` makes these valid without requiring full consumption | [x] |
| 6 | `driver` | Exact and near `INT_MIN`/`INT_MAX` boundary values | [x] |
| 7 | `driver` | Valid numeric prefix followed by an embedded NUL and ignored trailing bytes | [x] |
| 8 | `run`, `driver` | Randomized mixed call sequence through both exports, exercising their shared global state | [x] |

Feature combinations: one (`default`; `Cargo.toml` declares no features).
