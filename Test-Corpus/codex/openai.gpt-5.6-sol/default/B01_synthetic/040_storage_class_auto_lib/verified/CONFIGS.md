# Configuration Surface

The public headers expose one entry point and no runtime options, modes,
flags, element types, formats, byte-order choices, counts, pointers, or
feature-controlled APIs. The C implementation contains no `if` or `switch`
branch. Its one input shape is a by-value C `int`; tests include fixed-seed
random values plus `INT_MIN`, `INT_MIN + 1`, `-1`, `0`, `1`, `INT_MAX - 1`,
and `INT_MAX`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `driver(int)` | No options; every by-value C `int` input. Observable output is the bytes written to stdout by `printf("%d\n", 2*x + 300)`. | [x] |
