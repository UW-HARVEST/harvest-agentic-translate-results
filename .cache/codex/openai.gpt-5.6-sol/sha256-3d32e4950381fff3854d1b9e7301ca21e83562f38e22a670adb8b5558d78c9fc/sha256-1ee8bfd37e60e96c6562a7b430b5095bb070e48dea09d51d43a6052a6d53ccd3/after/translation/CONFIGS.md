# Configuration Surface

Mechanical enumeration covered all C-defined public symbols from `nm -D`, the
public header, and every runtime `if`/`for` branch in the C source. There are no
Cargo features, C preprocessor feature modes, option setters, enums, formats,
byte-order modes, or caller-supplied sizes/counts.

| # | entry point(s) | configuration (options set + input shape) | |
|---|----------------|-------------------------------------------|-|
| 1 | `printLine` | Non-null NUL-terminated C string; randomized empty and non-empty byte strings | [x] |
| 2 | `printIntLine` | Full C `int` domain; randomized negative, zero, positive, `INT_MIN`, and `INT_MAX` values | [x] |
| 3 | `bad` | Parameterless direct call; undersized `alloca(10)` path and ten-element copy | [x] |
| 4 | `good` | Parameterless direct call; `alloca(10*sizeof(int))` path and ten-element copy | [x] |
| 5 | `driver` | `useGood == 0`, selecting `bad` | [x] |
| 6 | `driver` | `useGood != 0`, selecting `good`; randomized positive and negative C `int` values | [x] |
