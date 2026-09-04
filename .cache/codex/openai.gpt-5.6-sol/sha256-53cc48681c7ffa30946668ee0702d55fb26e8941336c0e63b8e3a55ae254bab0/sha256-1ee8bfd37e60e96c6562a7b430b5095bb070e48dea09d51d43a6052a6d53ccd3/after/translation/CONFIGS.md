# Configuration-surface table

Mechanically derived from all five C-defined dynamic symbols and every
input-dependent branch in `../c_src/src/driver.c`. There are no compile-time
features or public mutable options.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `printLine` | non-null NUL-terminated byte string; randomized empty, one-byte, multi-byte, format-token, and non-UTF-8 contents | [x] |
| 2 | `printHexCharLine` | every signed-`char` value from `SCHAR_MIN` (`-128`) through `SCHAR_MAX` (`127`) | [x] |
| 3 | `bad` | no input; fixed `data = CHAR_MAX`, positive guard taken, multiplication converted back to `char` and printed | [x] |
| 4 | `good` | no input; composed `goodG2B` safe multiplication followed by `goodB2G` too-large rejection | [x] |
| 5 | `driver` | `useGood == 0`, selecting `bad` | [x] |
| 6 | `driver` | `useGood != 0`, selecting `good`; randomized negative and positive values including `INT_MIN`, `-1`, `1`, and `INT_MAX` | [x] |
