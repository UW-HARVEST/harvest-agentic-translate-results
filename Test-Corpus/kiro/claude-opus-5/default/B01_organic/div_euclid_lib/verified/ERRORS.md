# ERRORS.md — Phase C error-surface table

Mechanically derived from every rejection / early-out / guard in
`c_src/src/lib.c`. The whole library is one function with a scalar-only
signature (`int div_euclid(int v1, int v2)`), so there are:

* no pointer parameters → no `NULL` checks, no length/size parameters;
* no `assert`, no `errno`, no error enum, no out-parameters;
* no `return -1` / `return NULL` / `RETURN_ERROR`-style macros.

`grep -nE 'return|assert|!=|==|>=|<=|0x' c_src/src/lib.c` yields the guards
below. Every one is listed, one row per distinct rejection/guard branch.

## Rejections (the only way the C refuses to do the division)

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| E1 | `div_euclid` | `v2 == 0` (line 4: `if (v2 == 0) { return 0; }`) — division by zero, for ANY `v1` including `INT_MIN`, `INT_MAX`, `0`, `-1` | returns `0`, no trap, no UB |

## Guards that exist to AVOID undefined behaviour (each is a distinct branch the C takes on an extreme input, and each must be reproduced exactly)

| # | function | trigger (exact condition) | expected C result |
|---|----------|---------------------------|-------------------|
| E2 | `div_euclid` | `v1 >= 0 && v2 == (-0x7fffffff - 1)` — guard `v2 != INT_MIN` fails, so `-v2` is never evaluated | `q = 0, r = v1` → `r >= 0` → returns `0` |
| E3 | `div_euclid` | `v1 == (-0x7fffffff - 1) && v2 == (-0x7fffffff - 1)` — both `v1 != INT_MIN` and `v2 != INT_MIN` guards fail | `q = 1, r = 0` → `r >= 0` → returns `1` |
| E4 | `div_euclid` | `v1 == (-0x7fffffff - 1) && v2 > 0` — `v1 != INT_MIN` guard fails; `-v1` is never evaluated, the code uses `-(v1 + v2)` instead | `q = -((-(v1+v2))/v2) - 1`, `r = -((-(v1+v2))%v2)`; then `r < 0 ? q - 1 : q` (`v2 > 0`) |
| E5 | `div_euclid` | `v1 == (-0x7fffffff - 1) && v2 < 0 && v2 != INT_MIN` — `v1 != INT_MIN` guard fails; the code uses `-(v1 - v2)` and `-v2` | `q = ((-(v1-v2))/(-v2)) + 1`, `r = -((-(v1-v2))%(-v2))`; then `r < 0 ? q + 1 : q` (`v2 < 0`) |
| E6 | `div_euclid` | `v1 < 0 && v1 != INT_MIN && v2 == (-0x7fffffff - 1)` — inner `v2 != INT_MIN` guard fails, so `-v2` is never evaluated | `q = 1, r = v1 - q*v2` (comma operator: `q` is already `1`) → `r > 0` → returns `1` |
| E7 | `div_euclid` | `v1 == (-0x7fffffff - 1) && v2 == 0` — the `v2 == 0` guard (E1) fires FIRST, before any `INT_MIN` handling | returns `0` |
| E8 | `div_euclid` | `v1 == (-0x7fffffff - 1) && v2 == -1` — the classic `INT_MIN / -1` overflow trap; reached via the E5 path, which never divides `INT_MIN` by `-1` | `q = ((-(INT_MIN - -1))/1) + 1 = INT_MAX + 1`... computed as `-(INT_MIN+1) = INT_MAX`, `q = INT_MAX + 1` wraps → matches C's `int` arithmetic |
| E9 | `div_euclid` | `v1 == 0` with any `v2 != 0` (degenerate but valid: dividend zero) | `0 / v2` or `q = 0, r = 0` → returns `0` |

## Boundary inputs covered even though the C has no explicit check for them

`int` has no invalid bit patterns and there is no enum, pointer or length in the
signature, so *every* one of the 2^64 `(v1, v2)` pairs is a legal call. There is
no "one step past a documented valid range" value that the C rejects — instead
the extremes are the interesting inputs, and they are all exercised:
`INT_MIN`, `INT_MIN + 1`, `-1`, `0`, `1`, `INT_MAX - 1`, `INT_MAX`, and the full
cross-product of those with each other (see `tests/differential.rs`,
`errors_full_boundary_cross_product`).

## Status

| row | test | status |
|-----|------|--------|
| E1 | `error_e1_divisor_zero` | PASS |
| E2 | `error_e2_nonneg_dividend_intmin_divisor` | PASS |
| E3 | `error_e3_intmin_over_intmin` | PASS |
| E4 | `error_e4_intmin_dividend_positive_divisor` | PASS |
| E5 | `error_e5_intmin_dividend_negative_divisor` | PASS |
| E6 | `error_e6_negative_dividend_intmin_divisor` | PASS |
| E7 | `error_e7_intmin_over_zero` | PASS |
| E8 | `error_e8_intmin_over_minus_one` | PASS |
| E9 | `error_e9_zero_dividend` | PASS |
| all | `errors_full_boundary_cross_product` | PASS |
