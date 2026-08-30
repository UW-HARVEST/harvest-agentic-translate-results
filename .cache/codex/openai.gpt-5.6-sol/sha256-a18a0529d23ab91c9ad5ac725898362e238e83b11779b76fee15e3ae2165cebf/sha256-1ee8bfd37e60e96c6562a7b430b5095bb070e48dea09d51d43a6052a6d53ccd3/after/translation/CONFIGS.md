# Configuration Surface

There are no runtime options, modes, flags, enums, compile-time feature
branches, or Cargo features. The rows below cover the full set of C-defined
dynamic entry points, from the lowest-level output functions through the
composed `driver` function.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null, NUL-terminated byte string; randomized empty, short, and long strings without interior NUL bytes | [x] |
| 2 | `printIntLine` | Any C `int`; randomized full-range values plus `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX` | [x] |
| 3 | `bad` | No options or inputs; emits the unchanged local sum before and after the discarded expression | [x] |
| 4 | `good` | No options or inputs; emits the local sum before and after assignment | [x] |
| 5 | `driver` | No options or inputs; end-to-end composition of `printLine`, `good`, and `bad` | [x] |

Feature combinations: default only (`Cargo.toml` has no `[features]` table).
