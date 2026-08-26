# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table, so Cargo provides only the implicit
empty `default` feature. `c_src/CMakeLists.txt` declares no options or
conditional source files.

| # | Cargo invocation | CMake configuration | [x] check |
|---|------------------|---------------------|-----------|
| 1 | `cargo check --no-default-features` | default, PIC enabled | [x] |

There is exactly one valid feature combination: the empty feature set.

## Runtime Configurations

The public headers expose one entry point and no runtime options, modes,
flags, pointers, lengths, element types, formats, or byte-order choices. The C
implementation has no `if`, `switch`, or preprocessor branches. Its only
value-sensitive distinctions come from IEEE-754 single-precision arithmetic,
so the input classes below partition the meaningful result shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `to_barycentric` | no options; finite, nondegenerate triangles and finite query points (inside, on an edge, and outside) | [x] |
| 2 | `to_barycentric` | no options; collinear but distinct triangle vertices (zero Gram determinant) | [x] |
| 3 | `to_barycentric` | no options; two or three coincident triangle vertices | [x] |
| 4 | `to_barycentric` | no options; signed-zero and subnormal components | [x] |
| 5 | `to_barycentric` | no options; large finite components whose intermediate products may overflow | [x] |
| 6 | `to_barycentric` | no options; one or more positive/negative infinity components | [x] |
| 7 | `to_barycentric` | no options; one or more quiet/signaling NaN bit patterns and payloads | [x] |

`to_barycentric` is itself the lowest-level and only public API. The private C
helpers `lm_v2`, `lm_sub2`, and `lm_dot2` are not entry points and do not
appear in the dynamic symbol table.
