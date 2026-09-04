# Configuration-Surface Table

The public header and complete C implementation contain one entry point,
`driver(float)`. There are no runtime options, modes, flags, enums, formats,
counts, configurable widths, feature macros, or state transitions. The only
loop is internal `print_hex` iteration over `sizeof(float)`, which is fixed to
four bytes on the built ABI. There are no input-dependent `if` or `switch`
branches.

The single row covers the complete valid configuration surface. Its
property-style differential test includes random values spanning arbitrary
`float` bit patterns, plus explicit zero, signed zero, finite extrema,
subnormal, infinity, and NaN encodings.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(float)` | No options; one by-value 32-bit C `float`; output is the four native-representation bytes as lowercase hex followed by newline. | [x] |

Cargo metadata reports no named features. The checked row passes under both
the default build and the empty `--no-default-features` build.
