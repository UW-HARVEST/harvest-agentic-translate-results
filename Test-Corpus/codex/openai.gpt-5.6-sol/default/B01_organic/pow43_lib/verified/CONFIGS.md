# Configuration surface

The public API has one entry point, `pow43(int)`. There are no runtime options,
modes, flags, element types, formats, byte-order choices, pointer/count shapes,
compile-time feature branches, or convenience wrappers. The rows below are the
cross-product of the C control-flow branch and the formula's two possible
`sign` states, pruned where `sign` is not computed.

The defined table-access domain is `-16..=8223`. Tests include all branch
boundaries and many fixed-seed randomized values for each nontrivial row.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pow43` | direct table lookup: `-16 <= x < 129` (negative, zero, positive, and both boundaries) | [x] |
| 2 | `pow43` | scaled interpolation: `129 <= x < 1024`, `x <<= 3`, `mult = 16`, computed `sign = 0` | [x] |
| 3 | `pow43` | scaled interpolation: `129 <= x < 1024`, `x <<= 3`, `mult = 16`, computed `sign = 64` | [x] |
| 4 | `pow43` | unscaled interpolation: `1024 <= x <= 8223`, `mult = 256`, computed `sign = 0` | [x] |
| 5 | `pow43` | unscaled interpolation: `1024 <= x <= 8223`, `mult = 256`, computed `sign = 64` | [x] |

Cargo feature combinations:

| combination | [ ] |
|-------------|-----|
| default (the manifest declares no features) | [x] |
| `--no-default-features` (equivalent empty feature set) | [x] |
