# Configuration Surface

## Build-time configurations

`Cargo.toml` has no `[features]` table, and `c_src/CMakeLists.txt` declares no
options or compile definitions. There is exactly one valid combination:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features` (empty feature set) | default options with `CMAKE_POSITION_INDEPENDENT_CODE=ON` | [x] |

## Runtime valid-path configurations

The library has no runtime mode or option state. Rows below cover every
defined dynamic entry point, including the four lower-level symbols omitted
from `include/driver.h`. Randomized value classes are listed where output is
data-dependent. Invalid/null branches are enumerated in `ERRORS.md`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null, NUL-terminated byte string with no embedded NUL before the terminator; empty and non-empty lengths. | [x] |
| 2 | `printIntLine` | Any C `int`: negative, zero, positive, `INT_MIN`, and `INT_MAX`. | [x] |
| 3 | `bad` | Checked branch `data >= 0` with a defined array index `0..9`; lower, interior, and upper positions. | [x] |
| 4 | `good` | Fixed `goodG2B` index `7`, followed by `goodB2G` with `data` in `0..9`; lower, interior, and upper positions. | [x] |
| 5 | `driver` | Full composed operation with `goodData` and `badData` independently in `0..9`, covering their cross-product and call-order text. | [x] |
