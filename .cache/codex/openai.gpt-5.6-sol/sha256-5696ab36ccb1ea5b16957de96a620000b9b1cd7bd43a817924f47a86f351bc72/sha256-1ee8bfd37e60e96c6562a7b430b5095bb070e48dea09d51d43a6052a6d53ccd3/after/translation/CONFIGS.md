# Configuration Surface

The public headers expose only `pow43(int)`. There are no runtime options,
flags, compile-time features, element types, byte orders, pointers, or lengths.
The rows below are derived from the two C range branches and the `sign`
calculation that selects different interpolation shapes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `pow43` | Direct table path: `-16 <= x < 129` | [x] |
| 2 | `pow43` | Scaled path: `129 <= x < 1024`, `(x << 3) & 32 == 0` (`sign == 0`) | [x] |
| 3 | `pow43` | Scaled path: `129 <= x < 1024`, `(x << 3) & 32 != 0` (`sign == 64`) | [x] |
| 4 | `pow43` | Unscaled path: `1024 <= x <= 8223`, `x & 32 == 0` (`sign == 0`) | [x] |
| 5 | `pow43` | Unscaled path: `1024 <= x <= 8223`, `x & 32 != 0` (`sign == 64`) | [x] |

The valid boundaries come mechanically from the 145-element table and its
index expressions: `16 + x` on the direct path and
`16 + ((x + sign) >> 6)` on interpolation paths.
