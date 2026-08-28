# Configuration Surface

Mechanically derived from the public dynamic symbols, the `mode` switch, the
iteration loop shapes, the strict `value < threshold` branch, and the
`UINT16_MAX` count break in `../c_src/src/lib.c`. The two formally unused
arguments of each low-level operation are varied between boundary integers and
null/non-null context pointers even though C does not branch on them.

Mode classes are `0` (add 10), `1` (double), `2` (triple), and `other`
(default/add 10). "Rejected" includes equality because the comparison is
strict. Each non-boundary row uses many fixed-seed randomized inputs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `process_value` | Safe full-width negative/zero/positive values; ignored integer at boundaries; context null/non-null | [x] |
| 2 | `double_value` | Safe negative/zero/positive values; ignored integer at boundaries; context null/non-null | [x] |
| 3 | `triple_value` | Safe negative/zero/positive values; ignored integer at boundaries; context null/non-null | [x] |
| 4 | `gotomach` | mode 0; zero iterations; valid seed boundaries/interior; threshold unused | [x] |
| 5 | `gotomach` | mode 1; zero iterations; valid seed boundaries/interior; threshold unused | [x] |
| 6 | `gotomach` | mode 2; zero iterations; valid seed boundaries/interior; threshold unused | [x] |
| 7 | `gotomach` | other mode; zero iterations; valid seed boundaries/interior; threshold unused | [x] |
| 8 | `gotomach` | mode 0; one iteration; generated value rejected (including equality) | [x] |
| 9 | `gotomach` | mode 0; one iteration; generated value accepted | [x] |
| 10 | `gotomach` | mode 1; one iteration; generated value rejected (including equality) | [x] |
| 11 | `gotomach` | mode 1; one iteration; generated value accepted | [x] |
| 12 | `gotomach` | mode 2; one iteration; generated value rejected (including equality) | [x] |
| 13 | `gotomach` | mode 2; one iteration; generated value accepted | [x] |
| 14 | `gotomach` | other mode; one iteration; generated value rejected (including equality) | [x] |
| 15 | `gotomach` | other mode; one iteration; generated value accepted | [x] |
| 16 | `gotomach` | mode 0; many iterations; no generated values accepted | [x] |
| 17 | `gotomach` | mode 0; many iterations; mixed rejected and accepted values | [x] |
| 18 | `gotomach` | mode 0; many iterations; all generated values accepted | [x] |
| 19 | `gotomach` | mode 1; many iterations; no generated values accepted | [x] |
| 20 | `gotomach` | mode 1; many iterations; mixed rejected and accepted values | [x] |
| 21 | `gotomach` | mode 1; many iterations; all generated values accepted | [x] |
| 22 | `gotomach` | mode 2; many iterations; no generated values accepted | [x] |
| 23 | `gotomach` | mode 2; many iterations; mixed rejected and accepted values | [x] |
| 24 | `gotomach` | mode 2; many iterations; all generated values accepted | [x] |
| 25 | `gotomach` | other mode; many iterations; no generated values accepted | [x] |
| 26 | `gotomach` | other mode; many iterations; mixed rejected and accepted values | [x] |
| 27 | `gotomach` | other mode; many iterations; all generated values accepted | [x] |
| 28 | `gotomach` | mode 0; 65535 iterations; no values accepted; count break not reached | [x] |
| 29 | `gotomach` | mode 0; 65535 iterations; all values accepted; count reaches `UINT16_MAX` | [x] |
| 30 | `gotomach` | mode 1; 65535 iterations; no values accepted; count break not reached | [x] |
| 31 | `gotomach` | mode 1; 65535 iterations; all values accepted; count reaches `UINT16_MAX` | [x] |
| 32 | `gotomach` | mode 2; 65535 iterations; no values accepted; count break not reached | [x] |
| 33 | `gotomach` | mode 2; 65535 iterations; all values accepted; count reaches `UINT16_MAX` | [x] |
| 34 | `gotomach` | other mode; 65535 iterations; no values accepted; count break not reached | [x] |
| 35 | `gotomach` | other mode; 65535 iterations; all values accepted; count reaches `UINT16_MAX` | [x] |

Compile-time Cargo feature combinations: one (`default`; no features are
declared in `Cargo.toml`).
