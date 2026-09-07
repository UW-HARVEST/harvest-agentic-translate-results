# Configuration Surface

Mechanically inspected the public header and all `if`, `switch`, preprocessor,
loop, and `sizeof` uses in the C source.

There are no runtime options, modes, flags, feature conditionals, pointer
shapes, lengths, counts, formats, byte-order settings, or alternate public
entry points. `driver` always copies the complete native `float` object
representation (`sizeof(float)` bytes) and prints each byte in increasing
address order as two lowercase hexadecimal digits, followed by a newline.
The loop branch depends only on that fixed object size, not on the input value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(float)` | No options; one by-value C `float`; arbitrary 32-bit object representation, including signed zeros, subnormals, finite values, infinities, and NaN payloads | [x] |

Cargo metadata reports no declared features (`features: {}`). The row passes
with both the default invocation and explicit `--no-default-features`, using
18 boundary representations plus 25,000 fixed-seed randomized representations
per run.
