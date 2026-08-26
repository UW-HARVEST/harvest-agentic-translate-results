# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` defines no
options or conditional compilation. There is one valid build configuration:

| # | Cargo invocation | CMake configuration | [ ] |
|---|------------------|---------------------|-----|
| 1 | `--no-default-features --features ''` | default | [x] |

## Runtime Configurations

The sole public entry point is `float pow43(int x)`. It has no runtime options,
state, pointer shapes, element types, formats, or byte-order modes. The rows
below are the cross-product retained by the C arithmetic branches: direct
lookup versus each multiplier path, and the two values of the bit-derived
`sign` term used by both interpolation paths.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pow43` | direct table lookup: `-16 <= x < 129` | [x] |
| 2 | `pow43` | scaled interpolation: `129 <= x < 1024`, `(2 * (x << 3) & 64) == 0`, multiplier 16 | [x] |
| 3 | `pow43` | scaled interpolation: `129 <= x < 1024`, `(2 * (x << 3) & 64) == 64`, multiplier 16 | [x] |
| 4 | `pow43` | unscaled interpolation: `1024 <= x <= 8223`, `(2 * x & 64) == 0`, multiplier 256 | [x] |
| 5 | `pow43` | unscaled interpolation: `1024 <= x <= 8223`, `(2 * x & 64) == 64`, multiplier 256 | [x] |

The tested defined-input domain is `-16 <= x <= 8223`. The lower boundary is
set by `g_pow43[16 + x]`; the upper boundary is the final contiguous value for
which `g_pow43[16 + ((x + sign) >> 6)]` remains within its 145 elements.
