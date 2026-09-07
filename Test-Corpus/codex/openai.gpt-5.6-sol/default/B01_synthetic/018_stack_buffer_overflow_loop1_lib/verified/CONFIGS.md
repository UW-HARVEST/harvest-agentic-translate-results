# Configuration surface

Mechanically derived from the public/exported entry points, the
`line != NULL` branch, the `useGood` branch, C integer formatting, and the
fixed no-argument operations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | Non-null, empty NUL-terminated byte string. | [x] |
| 2 | `printLine` | Non-null, non-empty NUL-terminated byte string (randomized lengths and byte values excluding interior NUL). | [x] |
| 3 | `printIntLine` | Any C `int`, including negative, zero, positive, `INT_MIN`, and `INT_MAX` (randomized). | [x] |
| 4 | `bad` | No arguments; execute the fixed undersized-stack-allocation path. | [x] |
| 5 | `good` | No arguments; execute the fixed correctly-sized-stack-allocation path. | [x] |
| 6 | `driver` | `useGood == 0`, selecting `bad`. | [x] |
| 7 | `driver` | `useGood != 0`, selecting `good` (randomized positive and negative nonzero C `int` values). | [x] |

There are no Cargo features, C preprocessor feature flags, runtime modes
other than `driver`'s zero/nonzero selector, element types, lengths, byte-order
options, or format selectors.
