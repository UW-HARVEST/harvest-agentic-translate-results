# Configuration Surface

Mechanically derived from the public header, all symbols exported by the C
shared object, and every runtime branch in `src/driver.c`. There are no
compile-time feature flags or runtime options other than `driver.useGood`.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `printLine` | Non-null, NUL-terminated C string; empty and nonempty byte strings without interior NULs | [x] |
| 2 | `printHexCharLine` | Negative signed `char`, including `CHAR_MIN` and `-1` | [x] |
| 3 | `printHexCharLine` | Nonnegative signed `char`, including `0`, `1`, and `CHAR_MAX` | [x] |
| 4 | `bad` | No input; fixed `CHAR_MAX`, `data > 0`, overflowing multiply, then hexadecimal output | [x] |
| 5 | `good` | No input; fixed `goodG2B` arithmetic followed by `goodB2G`'s `data >= CHAR_MAX/2` message branch | [x] |
| 6 | `driver` | `useGood == 0`, selecting `bad` | [x] |
| 7 | `driver` | Any positive or negative nonzero `useGood`, selecting `good` | [x] |

`goodG2B` and `goodB2G` are static and therefore are exercised through the
exported `good` and `driver` pipelines. The source has no size, count, format,
byte-order, element-type, or compile-time feature axes.
