# Configuration Surface

The public headers expose one entry point and no runtime options, modes, flags,
pointer inputs, lengths, formats, element types, or compile-time feature
switches. The implementation has no `if`, `switch`, or preprocessor branches.
Its only input shape is a scalar `uint32_t`; tests must cover the full-width
domain, including values whose upper 16 bits are set because the C masks those
bits away.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `rev16` | No options; scalar `uint32_t`, covering zero, low-16-bit boundaries, upper-16-bit data, and full-width randomized values | [x] |
