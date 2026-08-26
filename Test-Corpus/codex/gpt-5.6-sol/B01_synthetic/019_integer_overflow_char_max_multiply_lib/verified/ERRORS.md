# Error Surface

The C API returns `void` throughout. Rejection therefore means producing no
output for a rejected pointer or printing the exact diagnostic selected by a
safety guard.

| # | function | trigger (the exact invalid input/condition) | expected C result | |
|---|----------|---------------------------------------------|-------------------|-|
| 1 | `printLine` | `line == NULL` | Return without output | [x] |
| 2 | `goodB2G`, observable through `good` and nonzero `driver` | Internal `data` is not less than `CHAR_MAX / 2`; the C assignment makes it exactly `CHAR_MAX` | Print `data value is too large to perform arithmetic safely.\n` | [x] |

## Mechanical Rejection Scan

The source contains no `RETURN_ERROR`, `return -1`, `return NULL`, `assert`,
error enum, length parameter, or public enum parameter.

All remaining conditions were classified explicitly:

| Source check | Classification |
|--------------|----------------|
| `bad`: `data > 0`, where `data = CHAR_MAX` | Operation guard; always true on the build platform and has no rejection branch |
| `goodG2B`: `data > 0`, where `data = 2` | Operation guard; always true and has no rejection branch |
| `goodB2G`: `data > 0`, where `data = CHAR_MAX` | Operation guard; always true on the build platform and has no rejection branch |
| `driver`: `useGood` | Runtime mode selection, not an invalid-input check; every `int` value is accepted |

There are no length-taking APIs, documented numeric ranges, or enum-valued
FFI parameters, so generic zero/oversized-length and out-of-range-enum cases
do not exist for this library.
