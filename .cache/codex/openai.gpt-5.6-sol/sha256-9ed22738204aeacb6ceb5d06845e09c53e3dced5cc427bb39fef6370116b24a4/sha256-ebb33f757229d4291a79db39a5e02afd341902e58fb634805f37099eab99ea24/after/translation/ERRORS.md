# Error Surface

Mechanical source scan covered `RETURN_ERROR`, negative and null returns,
assertions, `if`/`switch` guards, null checks, range constants, enums, and the
arguments passed to `div`. `driver.c` contains no explicit rejection branch,
error return, assertion, pointer, length, enum, or range check. Its only invalid
input domains are those of the called C integer-division operation.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `driver` | `y == 0` | process is terminated by `SIGFPE` while evaluating `div(x, y)`; no output line is produced | [x] |
| 2 | `driver` | `x == INT_MIN && y == -1` | process is terminated by `SIGFPE` on the target platform because the mathematical quotient is not representable; no output line is produced | [x] |

Generic FFI boundary audit: this API has no pointer, length, or enum arguments,
so null pointers, zero/oversized lengths, and out-of-range enum values do not
apply.

Both rows pass through the C and Rust shared-library FFI boundaries under the
default and `--no-default-features` builds.
