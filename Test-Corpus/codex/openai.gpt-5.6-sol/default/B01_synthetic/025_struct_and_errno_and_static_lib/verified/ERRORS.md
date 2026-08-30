# Error Surface

Derived mechanically from `c_src/src/driver.c:66-85`. `parse_val` is static,
so its result is observed through the exported `driver` function.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `parse_val` via `driver` | `endp == str`: `strtol` consumes no characters (including `""`, whitespace-only, and nonnumeric input) | `parse_val` returns `false`; `driver` writes `An error occurred\n` | [x] |
| 2 | `parse_val` via `driver` | `errno != 0` after `strtol` (decimal magnitude outside the C `long` range, including oversized digit strings) | `parse_val` returns `false`; `driver` writes `An error occurred\n` | [x] |
| 3 | `parse_val` via `driver` | `tmp < INT_MIN` while conversion consumed input and `errno == 0` | `parse_val` returns `false`; `driver` writes `An error occurred\n` | [x] |
| 4 | `parse_val` via `driver` | `tmp > INT_MAX` while conversion consumed input and `errno == 0` | `parse_val` returns `false`; `driver` writes `An error occurred\n` | [x] |
| 5 | `driver` FFI boundary | `in == NULL`; the C source has no null check and passes null to `strtol` | Process termination behavior must match the C shared library | [x] |

Boundary applicability:

- Zero-length input is row 1.
- Oversized input is row 2.
- Values one below and above the documented `int` range are rows 3 and 4.
- No public API accepts a separate length, so zero/oversized length arguments
  are not applicable.
- No public API accepts an enum, so out-of-range enum values are not applicable.
- `run` accepts an `int` by value and has no error return or rejection branch.
