# Configuration Surface

The crate declares no Cargo features, and the C source has no runtime mode
flags, switches, or conditional-compilation branches. The only build
configuration is the default one. Rows below cover every globally exported
entry point and every valid, caller-controlled input shape distinguished by C
branches. Randomized rows include boundary values.

| # | entry point(s) | configuration (options set + input shape) | passed |
|---|----------------|--------------------------------------------|--------|
| 1 | `printLine` | Default build; non-null pointer to a NUL-terminated byte string, including empty and non-empty strings. | [x] |
| 2 | `printIntLine` | Default build; any C `int`, including `INT_MIN`, `-1`, `0`, `1`, and `INT_MAX`. | [x] |
| 3 | `bad` | Default build; nonnegative in-bounds index `0..=9`, including both array boundaries. | [x] |
| 4 | `good` | Default build; `data` in `0..=9`; fixed `goodG2B` path followed by valid `goodB2G`, including both array boundaries. | [x] |
| 5 | `driver` | Default build; `goodData` and `badData` each in `0..=9`; full composed call path and all driver framing. | [x] |
| 6 | `bad` | Default build; one-past index `data == 10`, which C accepts through its nonnegative branch and prints as ten zero elements. | [x] |
| 7 | `driver` | Default build; randomized `goodData` in `0..=9` with one-past `badData == 10`; full composed call path. | [x] |

For `bad`, C branches only on negative versus nonnegative. The one-past value
10 does not take a distinct branch, but it is listed separately because it is
the generic one-past FFI boundary.
