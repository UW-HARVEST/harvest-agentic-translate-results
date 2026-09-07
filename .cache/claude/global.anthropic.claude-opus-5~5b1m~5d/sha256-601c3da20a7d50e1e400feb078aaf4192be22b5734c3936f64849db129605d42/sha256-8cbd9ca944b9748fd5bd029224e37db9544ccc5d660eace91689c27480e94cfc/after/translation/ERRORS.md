# ERRORS.md — Phase C error / rejection surface table

Derived mechanically by grepping **every** `return`, `if`, comparison against a
sentinel constant, `assert`, null check and min/max constant in
`c_src/src/lib.c` (the only C source file).

## Mechanical grep result

```
$ grep -nE 'return|assert|NULL|if *\(|0x7fffffff' c_src/src/lib.c
4:    if (v2 == 0) {
5:        return 0;
8:    if (v1 >= 0)
9:        if (v2 >= 0)
10:            return ((v1) / (v2));
11:        else if (v2 != (-0x7fffffff - 1))
15:    else if (v1 != (-0x7fffffff - 1))
16:        if (v2 >= 0)
18:        else if (v2 != (-0x7fffffff - 1))
22:    else if (v2 >= 0)
24:    else if (v2 != (-0x7fffffff - 1))
28:    if (r >= 0)
29:        return q;
31:        return q + (v2 > 0 ? -1 : 1);
```

Notes on the shape of this API's error surface:

* There are **no pointers** in the signature (`int div_euclid(int, int)`), hence
  **no null-pointer checks** and no "null pointer" rejection row is derivable.
* There are **no lengths/buffers**, hence no zero-length or oversized-length rows.
* There are **no enums** in the signature, hence no out-of-range-enum row. (The
  generic FFI robustness cases still get tests — see "Generic boundary coverage".)
* There are **no `assert`s**, no `errno`, and no error enum. The *only* explicit
  rejection is the `v2 == 0` divide-by-zero guard at line 4, which returns the
  in-band sentinel `0`.
* Lines 11/15/18/24 are `INT_MIN` guards: they exist specifically to **avoid the
  UB** of `-INT_MIN` / `INT_MIN / -1`. Each guard is a distinct rejection of an
  otherwise-UB-triggering input and gets its own row.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|---------------------------------------------|-------------------|------|
| E1 | `div_euclid` | `v2 == 0` (divide by zero) — with `v1 == 0` | returns `0` (line 5 sentinel; no trap, no `SIGFPE`) | `e1_v2_zero_v1_zero` |
| E2 | `div_euclid` | `v2 == 0` with `v1 > 0` (incl. `INT_MAX`) | returns `0` (guard taken before any division) | `e2_v2_zero_v1_positive` |
| E3 | `div_euclid` | `v2 == 0` with `v1 < 0` (incl. `INT_MIN`) | returns `0` (guard taken before any division) | `e3_v2_zero_v1_negative` |
| E4 | `div_euclid` | `v2 == 0` with **randomized** `v1` over full `i32` range | returns `0` for every `v1` | `e4_v2_zero_random_v1` |
| E5 | `div_euclid` | line 11 guard: `v1 >= 0 && v2 == INT_MIN` — rejects the `-v2` overflow path, falls to `q=0, r=v1` | `r = v1 >= 0` ⇒ returns `q == 0` | `e5_int_min_v2_nonneg_v1` |
| E6 | `div_euclid` | line 15 guard: `v1 == INT_MIN` — rejects the `-v1` overflow path, diverts to lines 22–27 | takes the `v1 == INT_MIN` branch family, never negates `v1` | `e6_int_min_v1_guard` |
| E7 | `div_euclid` | line 18 guard: `v1 < 0 && v1 != INT_MIN && v2 == INT_MIN` — rejects `-v2` overflow, falls to `q=1, r=v1-q*v2` | `r = v1 - INT_MIN >= 1 > 0` ⇒ returns `q == 1` | `e7_int_min_v2_negative_v1` |
| E8 | `div_euclid` | line 24 guard: `v1 == INT_MIN && v2 == INT_MIN` — rejects both negations, falls to `q=1, r=0` | `r == 0` ⇒ returns `q == 1` | `e8_int_min_both` |
| E9 | `div_euclid` | `v1 == INT_MIN && v2 > 0` (line 23) — the `-(v1+v2)` re-association that avoids `-INT_MIN` | quotient/remainder computed on `-(v1+v2)`; result must match C bit-for-bit incl. the `-1` adjustment | `e9_int_min_v1_positive_v2` |
| E10 | `div_euclid` | `v1 == INT_MIN && v2 < 0 && v2 != INT_MIN` (line 25) — the `-(v1-v2)` re-association | quotient computed on `-(v1-v2)` with `+1` adjustment; must match C | `e10_int_min_v1_negative_v2` |
| E11 | `div_euclid` | `v1 == INT_MIN && v2 == -1` — the classic `INT_MIN / -1` overflow trap input | must **not** trap; C reaches line 25 and returns its wrapped value | `e11_int_min_over_minus_one` |
| E12 | `div_euclid` | `v1 == INT_MIN && v2 == 1` | must not trap; C reaches line 23 | `e12_int_min_over_one` |
| E13 | `div_euclid` | negative-`r` epilogue, `v2 > 0` (line 31, `-1` adjustment) | returns `q - 1` | `e13_epilogue_negative_r_positive_v2` |
| E14 | `div_euclid` | negative-`r` epilogue, `v2 < 0` (line 31, `+1` adjustment) | returns `q + 1` | `e14_epilogue_negative_r_negative_v2` |
| E15 | `div_euclid` | `q + adjustment` overflow at line 31 (`q == INT_MAX` with `v2 < 0`, or `q == INT_MIN` with `v2 > 0`) | wrapping result, no trap — searched exhaustively over the `INT_MIN`/extreme grid | `e15_epilogue_adjust_overflow` |

## Generic boundary coverage (required even though not derivable as rows above)

Covered by `generic_boundaries` in `tests/differential.rs`:

* Both arguments at every extreme: `{INT_MIN, INT_MIN+1, -2, -1, 0, 1, 2, INT_MAX-1, INT_MAX}` full cross product.
* "One step past a valid range": `INT_MIN` and `INT_MAX` on both operands, and
  `v2 == 0` (the sole rejected value) plus `v2 == ±1` (its neighbours).
* Out-of-range "enum-like" ints: since `c_int` accepts any 32-bit value, the
  exhaustive sweeps in `tests/exhaustive.rs` pass **every** representable value
  of one operand while the other is held at each boundary, which subsumes the
  out-of-range-enum class for this signature.
* No pointer arguments exist, so null-pointer tests are not applicable; this is
  asserted explicitly by `no_pointer_arguments_in_api` (documents the reason).

## Status

All 15 rows have a passing differential test (C vs Rust, both loaded via
`libloading` from their `.so`). See test output in `PHASES.md`.
