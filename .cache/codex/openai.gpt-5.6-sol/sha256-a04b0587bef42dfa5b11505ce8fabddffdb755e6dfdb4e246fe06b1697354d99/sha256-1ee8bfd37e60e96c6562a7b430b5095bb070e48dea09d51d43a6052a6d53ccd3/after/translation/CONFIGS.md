# Configuration Surface

Mechanically derived from every terminal path in `div_euclid`'s `if` tree,
including the final `r >= 0` branch. There are no runtime options, modes,
flags, compile-time features, pointer inputs, lengths, or enum inputs. The only
public entry point is `div_euclid(int v1, int v2)`.

`MIN` below means the C `int` value `(-0x7fffffff - 1)`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `div_euclid` | `v1 >= 0`, `v2 > 0`; direct nonnegative division path | [x] |
| 2 | `div_euclid` | `v1 >= 0`, `MIN < v2 < 0`; negative divisor path | [x] |
| 3 | `div_euclid` | `v1 >= 0`, `v2 == MIN`; special minimum-divisor path | [x] |
| 4 | `div_euclid` | `MIN < v1 < 0`, `v2 > 0`, `(-v1) % v2 == 0`; exact positive-divisor path, `r == 0` | [x] |
| 5 | `div_euclid` | `MIN < v1 < 0`, `v2 > 0`, `(-v1) % v2 != 0`; non-exact positive-divisor path, `r < 0` | [x] |
| 6 | `div_euclid` | `MIN < v1 < 0`, `MIN < v2 < 0`, `(-v1) % (-v2) == 0`; exact negative-divisor path, `r == 0` | [x] |
| 7 | `div_euclid` | `MIN < v1 < 0`, `MIN < v2 < 0`, `(-v1) % (-v2) != 0`; non-exact negative-divisor path, `r < 0` | [x] |
| 8 | `div_euclid` | `MIN < v1 < 0`, `v2 == MIN`; special minimum-divisor path with positive `r` | [x] |
| 9 | `div_euclid` | `v1 == MIN`, `v2 > 0`, `(-(v1 + v2)) % v2 == 0`; exact minimum-dividend path, `r == 0` | [x] |
| 10 | `div_euclid` | `v1 == MIN`, `v2 > 0`, `(-(v1 + v2)) % v2 != 0`; non-exact minimum-dividend path, `r < 0` | [x] |
| 11 | `div_euclid` | `v1 == MIN`, `MIN < v2 < 0`, `(-(v1 - v2)) % (-v2) == 0`; exact negative-divisor path, `r == 0` | [x] |
| 12 | `div_euclid` | `v1 == MIN`, `MIN < v2 < 0`, `(-(v1 - v2)) % (-v2) != 0`; non-exact negative-divisor path, `r < 0` | [x] |
| 13 | `div_euclid` | `v1 == MIN`, `v2 == MIN`; double-minimum path | [x] |
