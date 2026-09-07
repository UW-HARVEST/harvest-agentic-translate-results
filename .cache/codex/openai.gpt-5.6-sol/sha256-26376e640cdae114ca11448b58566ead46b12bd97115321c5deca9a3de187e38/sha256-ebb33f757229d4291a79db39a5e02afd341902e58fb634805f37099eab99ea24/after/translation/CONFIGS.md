# Configuration surface

Mechanically inspected the public header and all `if`, `switch`, preprocessor,
and loop branches in `../c_src/src/driver.c`.

The API exposes no runtime options, modes, flags, element types, lengths,
formats, byte-order controls, or feature-gated behavior. The loop in the
private `print_hex` helper always processes the fixed `sizeof(house_t)` object.
No branch distinguishes categories of the `floors` input, so the complete
configuration cross-product has one row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver(int floors)` | No options; one C `int` value. Exercise negative, zero, positive, `INT_MIN`, `INT_MAX`, and randomized full-width bit patterns. Output is the byte representation of the fixed-layout `house_t`. | [x] |
