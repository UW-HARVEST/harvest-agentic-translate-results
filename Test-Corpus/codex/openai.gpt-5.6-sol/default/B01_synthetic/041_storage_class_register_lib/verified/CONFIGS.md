# Configuration Surface

Mechanically derived from the complete public header and every runtime branch,
switch, conditional-compilation branch, option, mode, flag, and special input
shape in the C source.

The library has one public entry point, no runtime options, no runtime branches,
no feature flags, and one scalar `int` input. The C implementation applies the
same operation to the full `int` domain.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver(int)` | No options; every representable C `int`, including `0`, positive/negative values, `INT_MIN`, `INT_MAX`, and values whose `2*x + 300` machine arithmetic crosses the signed boundary | [x] |

Verified with 20 targeted boundary inputs and 8,192 fixed-seed randomized
inputs under both the default build and `--no-default-features`.
