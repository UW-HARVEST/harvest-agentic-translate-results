# Configuration Surface

The public surface is the complete set of defined symbols from the C shared
library, including symbols not declared in `driver.h`. The rows below are
derived from the runtime branches in `driver.c`. There are no compile-time
features or preprocessor-controlled behavior variants.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null NUL-terminated byte string; randomized empty/one/many-byte payloads with no interior NUL. | [x] |
| 2 | `printHexCharLine` | Any C `char`; randomized across the full signed-char domain, including `CHAR_MIN`, `-1`, `0`, `1`, and `CHAR_MAX`. | [x] |
| 3 | `bad` | No input; fixed `data = CHAR_MAX`, positive branch, wrapping signed-char multiplication, then hex output. | [x] |
| 4 | `good` | No input; composed `goodG2B` arithmetic-output branch followed by `goodB2G` safety-rejection branch. | [x] |
| 5 | `driver` | `useGood == 0`, selecting `bad`. | [x] |
| 6 | `driver` | `useGood != 0`, selecting `good`; randomized positive and negative nonzero `int`, including `INT_MIN` and `INT_MAX`. | [x] |

Feature combinations from `Cargo.toml`: one (default/no features).

No CMake or Cargo binary target exists, so binary stdout comparison is not
applicable.
