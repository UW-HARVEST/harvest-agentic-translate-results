# Configuration Surface

The C source has one runtime branch: `driver(useGood)` selects `good` for every
nonzero `int` and `bad` for zero. The only data-shaped entry point accepts one
pointer to a single C `int`; it has no size, count, format, byte-order, or
element-type options.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|--------------------------------------------|-|
| 1 | `printIntPtrLine` | Non-null pointer to one C `int`; include `INT_MIN`, `-1`, `0`, `1`, `INT_MAX`, and fixed-seed randomized values | [x] |
| 2 | `good` | No options or inputs; local C `int` is set to `5` and printed through `printIntPtrLine` | [x] |
| 3 | `driver` | `useGood != 0`; include negative, positive, `INT_MIN`, `INT_MAX`, and fixed-seed randomized nonzero values | [x] |
| 4 | `bad` | No options or inputs; uninitialized local pointer path (validated as an isolated process outcome) | [x] |
| 5 | `driver` | `useGood == 0`; composed path through `bad` (validated as an isolated process outcome) | [x] |

Feature surface: `Cargo.toml` declares no features. The applicable Cargo
configurations are the default invocation and `--no-default-features`; both
pass the same differential suite.
