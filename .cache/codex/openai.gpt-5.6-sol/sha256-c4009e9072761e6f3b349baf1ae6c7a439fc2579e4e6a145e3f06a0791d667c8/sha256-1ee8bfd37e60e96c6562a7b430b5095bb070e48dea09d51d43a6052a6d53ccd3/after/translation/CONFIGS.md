# Configuration Surface

Mechanical inspection covered the complete public header and implementation.
There is one public entry point, no runtime options, modes, flags, feature
branches, or variable input shapes. `driver` always processes exactly
`sizeof(float)` bytes in native byte order and writes eight lowercase
hexadecimal digits followed by a newline.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; one by-value `float`; arbitrary 32-bit object representation, including finite values, signed zero, subnormals, infinities, and NaNs | [x] |

Cargo feature combinations: default only. `Cargo.toml` declares no features.
