# Configuration Surface

There are no runtime options, modes, flags, compile-time features, pointer
shapes, element types, byte-order choices, or stateful entry points. The full
public API is the single low-level scalar entry point
`int div_euclid(int v1, int v2)`.

The rows below are the reachable cross-product of the branches in
`../c_src/src/lib.c`, excluding the rejected `v2 == 0` case recorded in
`ERRORS.md`. “Exact” means the magnitude division has remainder zero; “non-
exact” means it has a nonzero remainder. Randomized checks for each row also
include applicable scalar boundaries (`0`, `1`, `-1`, `INT_MAX`, and
`INT_MIN`).

| # | entry point(s) | configuration (options set + input shape) | tested |
|---|----------------|--------------------------------------------|--------|
| 1 | `div_euclid` | `v1 >= 0`, `v2 > 0`; direct nonnegative division return | [x] |
| 2 | `div_euclid` | `v1 >= 0`, `INT_MIN < v2 < 0`; negative divisor general branch | [x] |
| 3 | `div_euclid` | `v1 >= 0`, `v2 == INT_MIN`; negative-divisor minimum special case | [x] |
| 4 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 > 0`, exact division; computed `r == 0` | [x] |
| 5 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 > 0`, non-exact division; computed `r < 0`, so result is `q - 1` | [x] |
| 6 | `div_euclid` | `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, exact division; computed `r == 0` | [x] |
| 7 | `div_euclid` | `INT_MIN < v1 < 0`, `INT_MIN < v2 < 0`, non-exact division; computed `r < 0`, so result is `q + 1` | [x] |
| 8 | `div_euclid` | `INT_MIN < v1 < 0`, `v2 == INT_MIN`; divisor-minimum special case with `r > 0` | [x] |
| 9 | `div_euclid` | `v1 == INT_MIN`, `v2 > 0`, exact division; dividend-minimum special case with `r == 0` | [x] |
| 10 | `div_euclid` | `v1 == INT_MIN`, `v2 > 0`, non-exact division; computed `r < 0`, so result is `q - 1` | [x] |
| 11 | `div_euclid` | `v1 == INT_MIN`, `INT_MIN < v2 < 0`, exact division; dividend-minimum special case with `r == 0` | [x] |
| 12 | `div_euclid` | `v1 == INT_MIN`, `INT_MIN < v2 < 0`, non-exact division; computed `r < 0`, so result is `q + 1` | [x] |
| 13 | `div_euclid` | `v1 == INT_MIN`, `v2 == INT_MIN`; both-minimum special case | [x] |
