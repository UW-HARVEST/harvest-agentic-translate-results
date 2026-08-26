# Error Surface

Derived mechanically from active `if` checks and error/sentinel returns in
`c_src/src/q_math.c`. The source has no active `assert`, error macro, error
enum, `return -1`, or `return NULL`. Saturating range checks are included
because the requested error surface explicitly includes every range check.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|---------------------------------------------|-------------------|--------|
| 1 | `ClampChar` | `i < -128` | returns `-128` | [x] |
| 2 | `ClampChar` | `i > 127` | returns `127` | [x] |
| 3 | `ClampShort` | `i < -32768` | returns `-32768` | [x] |
| 4 | `ClampShort` | `i > 0x7fff` | returns `0x7fff` | [x] |
| 5 | `DirToByte` | `dir == NULL` | returns `0` | [x] |
| 6 | `ByteToDir` | `b < 0 || b >= NUMVERTEXNORMALS` (`162`) | writes `{0, 0, 0}` to `dir`, then returns | [x] |
| 7 | `PlaneFromPoints` | `VectorNormalize(plane) == 0` after crossing the point differences (degenerate triangle) | returns `qfalse` (`0`) | [x] |

Completion:

- [x] Every row has an exact C-vs-Rust differential test.
- [x] Generic null boundaries match in child-process differential tests.
- [x] No public API accepts a length or enum; out-of-range `signbits` is covered
  through the C `default` arm.
