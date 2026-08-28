# Configuration Surface

The public surface contains only `void sieve(int start)`. There are no runtime
options, modes, flags, element types, formats, byte-order settings, counts,
pointers, lengths, enums, compile-time feature flags, or lower-level entry
points. The rows below are derived from the sole C branch,
`if (val % 10 == 9)`, including C's signed remainder behavior.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `sieve` | Nonnegative `start` with `start % 10 == 9`; the initial value satisfies the termination branch and exactly one line is emitted. | [x] |
| 2 | `sieve` | Nonnegative `start` with `start % 10 != 9` and no signed-overflow execution; values are emitted through the next value whose remainder is 9. This includes the zero boundary. | [x] |
| 3 | `sieve` | Negative `start`; C remainders are negative or zero, so the termination branch is first satisfied at positive 9. | [x] |

Public entry points covered: **1 of 1**
