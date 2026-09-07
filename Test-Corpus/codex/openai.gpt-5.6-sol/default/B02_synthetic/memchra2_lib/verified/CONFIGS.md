# Configuration surface

Mechanical branch/input-shape inventory from `include/lib.h` and `src/lib.c`:

- Public entry points: only `memchra2`; all lower-level functions are `static`.
- `a` is also interpreted as IEEE-754 `float` bits. The code distinguishes:
  condition false with sign bit clear (`P0`), `0 < f < 1` (`P1`),
  `1 <= f < 1000` (`P2`), and condition false with sign bit set (`N0`).
- The sign of every decimal argument changes the generated buffer by adding a
  `'-'`, changing both `dash_count` and the byte sum. For `b`, `c`, and `d`
  this is the full eight-way sign-mask cross-product.
- Decimal digit widths, zero, `INT_MIN`/`INT_MAX`, and the low eight bits of
  every value affect loop counts, ASCII sums, byte interpretation, or XOR
  data. Randomized cases within each row span those shapes.
- The fixed internal configuration is a 64-byte `snprintf` buffer (all four-int
  renderings fit), four array elements, four test strings (three `"test"`
  prefixes), four interpreted bytes, native byte order, and mask `0xFF`.
  These have no caller-settable alternative.

Each matrix row receives many fixed-seed randomized cases through both shared
libraries. Exact transition values are listed after the cross-product.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `memchra2` | `P0`; `b>=0,c>=0,d>=0` | [x] |
| C02 | `memchra2` | `P0`; `b<0,c>=0,d>=0` | [x] |
| C03 | `memchra2` | `P0`; `b>=0,c<0,d>=0` | [x] |
| C04 | `memchra2` | `P0`; `b<0,c<0,d>=0` | [x] |
| C05 | `memchra2` | `P0`; `b>=0,c>=0,d<0` | [x] |
| C06 | `memchra2` | `P0`; `b<0,c>=0,d<0` | [x] |
| C07 | `memchra2` | `P0`; `b>=0,c<0,d<0` | [x] |
| C08 | `memchra2` | `P0`; `b<0,c<0,d<0` | [x] |
| C09 | `memchra2` | `P1`; `b>=0,c>=0,d>=0` | [x] |
| C10 | `memchra2` | `P1`; `b<0,c>=0,d>=0` | [x] |
| C11 | `memchra2` | `P1`; `b>=0,c<0,d>=0` | [x] |
| C12 | `memchra2` | `P1`; `b<0,c<0,d>=0` | [x] |
| C13 | `memchra2` | `P1`; `b>=0,c>=0,d<0` | [x] |
| C14 | `memchra2` | `P1`; `b<0,c>=0,d<0` | [x] |
| C15 | `memchra2` | `P1`; `b>=0,c<0,d<0` | [x] |
| C16 | `memchra2` | `P1`; `b<0,c<0,d<0` | [x] |
| C17 | `memchra2` | `P2`; `b>=0,c>=0,d>=0` | [x] |
| C18 | `memchra2` | `P2`; `b<0,c>=0,d>=0` | [x] |
| C19 | `memchra2` | `P2`; `b>=0,c<0,d>=0` | [x] |
| C20 | `memchra2` | `P2`; `b<0,c<0,d>=0` | [x] |
| C21 | `memchra2` | `P2`; `b>=0,c>=0,d<0` | [x] |
| C22 | `memchra2` | `P2`; `b<0,c>=0,d<0` | [x] |
| C23 | `memchra2` | `P2`; `b>=0,c<0,d<0` | [x] |
| C24 | `memchra2` | `P2`; `b<0,c<0,d<0` | [x] |
| C25 | `memchra2` | `N0`; `b>=0,c>=0,d>=0` | [x] |
| C26 | `memchra2` | `N0`; `b<0,c>=0,d>=0` | [x] |
| C27 | `memchra2` | `N0`; `b>=0,c<0,d>=0` | [x] |
| C28 | `memchra2` | `N0`; `b<0,c<0,d>=0` | [x] |
| C29 | `memchra2` | `N0`; `b>=0,c>=0,d<0` | [x] |
| C30 | `memchra2` | `N0`; `b<0,c>=0,d<0` | [x] |
| C31 | `memchra2` | `N0`; `b>=0,c<0,d<0` | [x] |
| C32 | `memchra2` | `N0`; `b<0,c<0,d<0` | [x] |
| C33 | `memchra2` | `a` float bits `+0.0`; zero-valued arguments | [x] |
| C34 | `memchra2` | `a` float bits smallest positive subnormal | [x] |
| C35 | `memchra2` | `a` float bits immediately below `1.0` | [x] |
| C36 | `memchra2` | `a` float bits exactly `1.0` | [x] |
| C37 | `memchra2` | `a` float bits immediately below `1000.0` | [x] |
| C38 | `memchra2` | `a` float bits exactly `1000.0` | [x] |
| C39 | `memchra2` | `a,b,c,d` at `INT_MIN`/`INT_MAX` combinations | [x] |
