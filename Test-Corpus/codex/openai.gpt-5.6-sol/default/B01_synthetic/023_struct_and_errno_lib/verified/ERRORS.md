# Error Surface

The C source has no error enum, assertion, null check, length parameter, or
explicit error-returning public function. All explicit rejection branches are
the independently triggerable terms of `parse_val`'s condition in
`src/driver.c:64`; `driver` exposes each rejection as the same output line.

| # | function | trigger (the exact invalid input/condition) | expected C result | verified |
|---|----------|---------------------------------------------|-------------------|----------|
| E1 | `driver` via `parse_val` | `endp == str`: `strtol` consumes no characters (empty input, whitespace-only input, or no decimal number at the first non-space character) | prints exactly `An error occurred\n`; returns `void` | [x] |
| E2 | `driver` via `parse_val` | `errno != 0`: decimal magnitude overflows or underflows `long`, so `strtol` sets `ERANGE` | prints exactly `An error occurred\n`; returns `void` | [x] |
| E3 | `driver` via `parse_val` | `tmp < INT_MIN` while conversion itself fits in `long` | prints exactly `An error occurred\n`; returns `void` | [x] |
| E4 | `driver` via `parse_val` | `tmp > INT_MAX` while conversion itself fits in `long` | prints exactly `An error occurred\n`; returns `void` | [x] |

Generic FFI boundaries not represented by explicit C checks are tested
separately: null `driver` input and null `run` house pointers. The API has no
lengths or enums, so zero/oversized lengths and out-of-range enum values are
not applicable.
