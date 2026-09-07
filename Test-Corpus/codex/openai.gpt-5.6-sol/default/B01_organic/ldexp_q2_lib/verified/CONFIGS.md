# Configuration Surface

The only public entry point is `ldexp_q2(float y, int exp_q2)`. There are no
runtime options, modes, flags, feature declarations, or alternate public
wrappers. The rows below are derived from the C distinctions:

- `e = min(120, exp_q2)`;
- the four `g_expfrac[e & 3]` selectors;
- the `e >> 2` shift count;
- the do-while termination test after at least one pass.

Each row is exercised with randomized `float` bit patterns, covering signs,
zeroes, subnormals, normals, infinities, and NaNs.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ldexp_q2` | Negative exponent, including `INT_MIN` and values near zero; one do-while pass on this build target | [x] |
| 2 | `ldexp_q2` | Exponent `0`; one pass with selector 0 and shift count 0 | [x] |
| 3 | `ldexp_q2` | One-pass exponent `1..119`, `exp_q2 & 3 == 0` | [x] |
| 4 | `ldexp_q2` | One-pass exponent `1..119`, `exp_q2 & 3 == 1` | [x] |
| 5 | `ldexp_q2` | One-pass exponent `1..119`, `exp_q2 & 3 == 2` | [x] |
| 6 | `ldexp_q2` | One-pass exponent `1..119`, `exp_q2 & 3 == 3` | [x] |
| 7 | `ldexp_q2` | Exponent `120`, the `min` boundary; one pass and no remainder | [x] |
| 8 | `ldexp_q2` | Multi-pass exponent `>120` with remainder `0` modulo 120 | [x] |
| 9 | `ldexp_q2` | Multi-pass exponent `>120`, final remainder `r`, `r & 3 == 0` | [x] |
| 10 | `ldexp_q2` | Multi-pass exponent `>120`, final remainder `r`, `r & 3 == 1` | [x] |
| 11 | `ldexp_q2` | Multi-pass exponent `>120`, final remainder `r`, `r & 3 == 2` | [x] |
| 12 | `ldexp_q2` | Multi-pass exponent `>120`, final remainder `r`, `r & 3 == 3`, including `INT_MAX` | [x] |

Cargo.toml declares no features, so the complete feature matrix contains only
the default/no-feature build.
