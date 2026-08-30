# Configuration Surface

Mechanical review covered the public header and every `if`, `switch`, loop,
preprocessor branch, option, mode, flag, and input-shape branch in
`../c_src/src/driver.c`. The only loop is the private byte-printing loop with a
fixed `sizeof(house_t)` bound; there are no runtime options, feature switches,
or shape-dependent public API branches.

| # | entry point(s) | configuration (options set + input shape) | Verified |
|---|----------------|--------------------------------------------|----------|
| 1 | `driver` | No options; one by-value C `int`, covering zero, both signs, `INT_MIN`, `INT_MAX`, and randomized values across the full 32-bit domain; output is the complete stdout byte stream | [x] |
