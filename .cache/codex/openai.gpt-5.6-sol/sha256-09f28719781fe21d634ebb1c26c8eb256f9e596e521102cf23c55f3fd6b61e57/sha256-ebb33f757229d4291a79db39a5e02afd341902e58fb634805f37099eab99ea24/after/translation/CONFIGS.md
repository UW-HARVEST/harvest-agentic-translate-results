# Configuration surface

Mechanical inspection of the public header and all `if`, `switch`, and
preprocessor branches found one public entry point, no runtime options or
flags, no compile-time feature branches, and no input-shape branches. The API
always accepts four by-value `lm_vec2` values and follows one arithmetic path.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `to_barycentric` | No options; four by-value `lm_vec2` inputs, randomized across ordinary finite coordinates and IEEE-754 edge bit patterns, including zero, subnormal, maximum finite, infinity, NaN, repeated vertices, and collinear/degenerate geometry | [x] |

Cargo feature combinations: **1** (`default`; `Cargo.toml` declares no
features).
