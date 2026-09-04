# Error Surface

The C source contains no explicit error return, assertion, null check, enum,
length, range check, or min/max constant. `driver` has no pointer, length, or
enum parameters. Its rejection surface is the behavior of the directly called
C `div(int, int)` operation.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|----------------------------------------------|-------------------|----------|
| 1 | `driver` | `y == 0` (integer division by zero) | does not return; process terminates with `SIGFPE` | [x] |
| 2 | `driver` | `x == INT_MIN && y == -1` (quotient is not representable as `int`) | does not return; process terminates with `SIGFPE` | [x] |

Generic FFI boundary audit:

- Null pointers: not applicable; the API has no pointer parameters.
- Zero lengths: not applicable; the API has no length parameters.
- Oversized lengths: not applicable; the API has no length parameters.
- Out-of-range enum values: not applicable; the API has no enum parameters.
- One past a documented range: not representable across this ABI because both
  arguments already use the full C `int` range.
