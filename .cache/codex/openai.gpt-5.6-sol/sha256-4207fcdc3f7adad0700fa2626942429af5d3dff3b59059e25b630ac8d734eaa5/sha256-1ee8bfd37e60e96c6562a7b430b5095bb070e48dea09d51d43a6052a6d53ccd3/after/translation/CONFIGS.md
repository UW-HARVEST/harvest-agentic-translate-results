# Configuration Surface

Mechanical inspection covered the public header and every branch, loop,
preprocessor conditional, and call in `../c_src/src/driver.c`.

There are no runtime options, modes, flags, feature conditionals, element
types, variable lengths, or alternate public entry points. The sole input
axis is the complete C `int` bit pattern. Its bytes are printed in native
memory order as two lowercase hexadecimal digits per byte followed by `\n`.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | No options; any C `int` value, covering zero, positive, negative, extrema, repeated-byte patterns, and randomized bit patterns | [x] |
