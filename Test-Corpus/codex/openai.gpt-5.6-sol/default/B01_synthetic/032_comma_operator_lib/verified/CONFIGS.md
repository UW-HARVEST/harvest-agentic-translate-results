# Configuration surface

The public headers expose only `driver(int)`. There are no runtime options,
modes, flags, element types, formats, byte-order choices, compile-time feature
branches, or additional entry points. The C loop distinguishes the following
input shapes through the `i < x` condition.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | `x < 0`: negative `int`, zero iterations and empty output | [x] |
| 2 | `driver` | `x == 0`: boundary value, zero iterations and empty output | [x] |
| 3 | `driver` | `x == 1`: exactly one output record (`0 0\n`) | [x] |
| 4 | `driver` | `x > 1`: many output records; randomized positive counts | [x] |
