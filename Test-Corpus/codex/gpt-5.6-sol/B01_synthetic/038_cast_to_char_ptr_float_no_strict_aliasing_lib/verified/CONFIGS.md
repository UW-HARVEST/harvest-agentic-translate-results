# Configuration Surface

Build-time configuration has one valid combination: no Cargo features and no
CMake options. The public header exposes only `driver(float)`. Its C
implementation has no runtime option branches or input-shape branches and
always emits all four bytes of the supplied IEEE-754 object representation in
native byte order.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|--------|
| 1 | `driver` | no options; arbitrary 32-bit `float` object representation, including finite values, signed zero, infinities, subnormals, and NaNs | [x] |
