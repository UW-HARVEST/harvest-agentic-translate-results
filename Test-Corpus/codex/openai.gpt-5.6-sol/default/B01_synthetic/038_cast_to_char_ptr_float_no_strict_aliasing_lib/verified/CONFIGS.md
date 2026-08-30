# Configuration Surface

The complete public API is `void driver(float x)`. It has no runtime options,
modes, flags, feature-controlled branches, or lower-level public entry points.
The C implementation always copies `sizeof(float)` bytes and prints each byte
in native object-representation order with `%02x`, followed by a newline.

The source has one data-independent loop over the fixed byte count. It does not
branch on the float's sign, exponent, significand, classification, or value.
Consequently one row covers the full input domain; its randomized corpus must
sample raw bit patterns rather than only finite numeric values.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; every raw 32-bit `float` object representation, including positive/negative zero, normals, subnormals, infinities, quiet/signaling NaNs, and randomized payloads; native byte order; fixed `sizeof(float)` byte count | [x] |
