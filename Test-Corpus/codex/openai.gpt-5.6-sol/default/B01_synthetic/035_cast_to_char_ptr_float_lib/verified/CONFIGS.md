# Configuration-surface table

Mechanical source inventory:

- Public entry points: `driver(float)`.
- Runtime options, modes, and flags: none.
- Public input shape: one by-value `float`.
- C branches on options or float values: none.
- Fixed internal shape: `sizeof(float)` bytes are emitted in native object-byte
  order; on the built ELF64 x86-64 ABI this is four bytes, little-endian.
- Compile-time Cargo feature combinations: one (no features are declared).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `driver` | No options; one by-value `float`; exercise arbitrary raw 32-bit representations, including signed zero, finite normals, subnormals, infinities, quiet/signaling NaNs, boundary bit patterns, and many fixed-seed randomized patterns; compare the complete emitted byte stream. | [x] |

Phase B/D matrix status: **[x] default** and **[x] `--no-default-features`**.
No named Cargo features exist, so this is the complete feature matrix.
