# Error Surface

The C source contains no explicit error return, assertion, null check, range
check, enum, pointer, or length parameter. Its two invalid arithmetic inputs
are rejected by libc `div` through process termination on the target platform.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| [x] 1 | `driver` | `y == 0` | process terminates with `SIGFPE` |
| [x] 2 | `driver` | `x == INT_MIN && y == -1` | process terminates with `SIGFPE` |

Generic pointer, length, and enum boundary cases are not applicable: the only
public signature is `void driver(int x, int y)`.
