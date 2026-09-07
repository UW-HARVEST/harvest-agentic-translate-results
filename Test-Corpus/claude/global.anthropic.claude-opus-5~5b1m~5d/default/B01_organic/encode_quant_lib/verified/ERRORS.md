# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c` (62 lines) and `c_src/include/lib.h`
(1 line). Greps performed over the whole C tree:

```
grep -n 'RETURN_ERROR\|return -1\|return NULL\|assert\|errno\|exit(\|abort(' c_src/src/lib.c   # no match
grep -n 'return'   c_src/src/lib.c   # only line 61: `return (uni);`
grep -n 'if'       c_src/src/lib.c   # lines 8,10,12,13,20,31,37,43,57,59 — all value selection, none error
grep -n 'enum\|#define\|#ifdef\|MIN\|MAX' c_src/src/lib.c c_src/include/lib.h  # no match
grep -n '[*&]' c_src/include/lib.h   # no pointer parameters anywhere
```

## Result: the C error surface is EMPTY

`encode_quant` is a **total function**. It:

* takes six by-value `int` parameters — **no pointers**, so there is no null
  check to mirror and no null-pointer row;
* has **no error-return macro, no sentinel return, no error enum, no `assert`,
  no explicit range check, and no min/max constant**;
* has exactly **one** `return` statement (line 61) which always returns the
  selected `uni`;
* cannot fail: every one of the ten `if` statements selects between values, it
  never rejects input.

So there are **zero classical rejection rows**. To keep this phase meaningful
rather than vacuous, the table below instead enumerates every **implicit** edge
condition in the C — the operations whose result is boundary-, overflow- or
UB-adjacent, i.e. exactly the inputs where a naive Rust translation would
*panic* or *wrap differently* instead of returning the C's value. Each row is a
real differential test in `tests/error_paths.rs`.

| #  | function | trigger (the exact invalid/extreme input or condition) | expected C result |
|----|----------|--------------------------------------------------------|-------------------|
| 1  | `encode_quant` | no pointer args exist → nothing to null-check; call with all-zero args | returns a value (`0`), never faults |
| 2  | `encode_quant` | `uni = INT_MAX`, so `uni1 = uni + 1` overflows (line 6) | wrapping add → `INT_MIN`; result equals C's |
| 3  | `encode_quant` | `uni = INT_MIN`, so `uni2 = uni - 1` overflows (line 7) | wrapping sub → `INT_MAX`; result equals C's |
| 4  | `encode_quant` | `step = INT_MAX` with `uni & 7 == 7` → `(2*7+1)*step` overflows (line 30) | wrapping mul, then `/8`; result equals C's |
| 5  | `encode_quant` | `step = INT_MIN` → `(2*(uni&7)+1)*step` overflows and `diff = -diff` negates `INT_MIN` (lines 30–32) | wrapping neg of `INT_MIN` == `INT_MIN`; result equals C's |
| 6  | `encode_quant` | `diff == INT_MIN` and `uni & 8` set → unary minus on `INT_MIN` (line 32) | `INT_MIN`; no trap |
| 7  | `encode_quant` | `pred + diff` overflows (lines 33/39/45) | wrapping add for `p0`/`p1`/`p2` |
| 8  | `encode_quant` | `tgt - p0` overflows, e.g. `tgt = INT_MAX`, `p0 < 0` (lines 34/40/46) | wrapping sub |
| 9  | `encode_quant` | `tgt2 - p0` overflows (lines 48/51/54) | wrapping sub |
| 10 | `encode_quant` | `d0 ^ (d0 >> 31)` where `d0 == INT_MIN` (line 35) — abs-via-xor cannot represent `+2^31` | `INT_MAX`, *not* `INT_MIN`; arithmetic (sign-propagating) shift |
| 11 | `encode_quant` | `d3 >> 5` where `d3` is negative (lines 50/53/56) | **arithmetic** shift (rounds toward −inf), not logical |
| 12 | `encode_quant` | `d0 += d3 >> 5` overflows (lines 50/53/56) | wrapping add |
| 13 | `encode_quant` | `(2*(uni&7)+1)*step) / 8` with negative numerator (line 30) | truncation **toward zero**, not floor |
| 14 | `encode_quant` | `uni` negative → `uni & 7` / `uni & 8` on two's-complement negative | mask of the two's-complement bit pattern |
| 15 | `encode_quant` | `uni >> 1`, `uni >> 2` with `uni` negative in the `lsbit == 4` path (line 17) | arithmetic shift |
| 16 | `encode_quant` | `lsbit` out-of-range / undocumented value (it is a plain `int`, not an enum with a fixed variant set): `lsbit = 4` exactly | takes the line 13 special branch |
| 17 | `encode_quant` | `lsbit` odd and `!= 4`, incl. negative odd (`-1`, `-3`, `INT_MIN+1`) | takes the line 20 `uni \|= 1` branch |
| 18 | `encode_quant` | `lsbit` even, nonzero, `!= 4`, incl. negative even (`-2`, `2`, `6`, `INT_MIN`) | takes the line 24 `uni &= ~1` else branch |
| 19 | `encode_quant` | `lsbit == 0` | skips the whole line 12 block entirely |
| 20 | `encode_quant` | out-of-`int`-range enum-style value in `lsbit`: `INT_MAX` (odd) and `INT_MIN` (even) | `INT_MAX` → odd branch, `INT_MIN` → else branch |
| 21 | `encode_quant` | `uni & 7 == 7` → `uni ^ uni1` crosses the `~7` boundary (line 8) | `uni1` is clamped back to `uni` |
| 22 | `encode_quant` | `uni & 7 == 0` → `uni ^ uni2` crosses the `~7` boundary (line 10) | `uni2` is clamped back to `uni` |
| 23 | `encode_quant` | `step = 0` → every `diff` is `0`, so `p0 == p1 == p2` and `d1 == d0`, `d2 == d0` | neither `d1 < d0` nor `d2 < d0`; returns the conditioned `uni` |
| 24 | `encode_quant` | tie: `d1 == d0` and `d2 == d0` (strict `<` on lines 57/59) | returns `uni`, **not** a candidate |
| 25 | `encode_quant` | both `d1 < d0` and `d2 < d0` — the C's second `if` overwrites the first | returns `uni2`, even when `d1 < d2` |
| 26 | `encode_quant` | all six args `INT_MIN` simultaneously (every overflow at once) | a defined wrapping value; must match bit-for-bit |
| 27 | `encode_quant` | all six args `INT_MAX` simultaneously | a defined wrapping value; must match bit-for-bit |

Rows 2–13, 26 and 27 are the ones that would make a *non*-wrapping Rust
translation panic in a debug build (`attempt to add with overflow`,
`attempt to negate with overflow`), which is why they belong in the error phase.

---

## Verification status (Phase C)

All 27 rows have a passing differential test in `tests/error_paths.rs`, plus one
extra generic sweep. Both `.so`s are loaded via `libloading`; the Rust function
is never called directly.

| # | test | [x] |
|---|------|-----|
| 1 | `row01_no_pointer_args_all_zero_call` | [x] |
| 2 | `row02_uni_int_max_plus_one_overflow` | [x] |
| 3 | `row03_uni_int_min_minus_one_overflow` | [x] |
| 4 | `row04_step_int_max_multiply_overflow` | [x] |
| 5 | `row05_step_int_min_overflow_and_negate` | [x] |
| 6 | `row06_negate_diff_at_attainable_extremes` (see correction below) | [x] |
| 7 | `row07_pred_plus_diff_overflow` | [x] |
| 8 | `row08_tgt_minus_p_overflow` | [x] |
| 9 | `row09_tgt2_minus_p_overflow` | [x] |
| 10 | `row10_xor_abs_of_int_min` | [x] |
| 11 | `row11_d3_shift_right_five_arithmetic` | [x] |
| 12 | `row12_penalty_add_overflow` | [x] |
| 13 | `row13_division_truncates_toward_zero` | [x] |
| 14 | `row14_negative_uni_masks` | [x] |
| 15 | `row15_lsbit4_negative_arithmetic_shift` | [x] |
| 16 | `row16_lsbit_exactly_four` | [x] |
| 17 | `row17_lsbit_odd_including_negative_and_int_max` | [x] |
| 18 | `row18_lsbit_even_nonzero_including_negative_and_int_min` | [x] |
| 19 | `row19_lsbit_zero_skips_block` | [x] |
| 20 | `row20_lsbit_out_of_range_extremes` | [x] |
| 21 | `row21_uni1_clamped_at_group_top` | [x] |
| 22 | `row22_uni2_clamped_at_group_bottom` | [x] |
| 23 | `row23_step_zero_no_candidate_wins` | [x] |
| 24 | `row24_strict_less_than_keeps_uni_on_ties` | [x] |
| 25 | `row25_both_better_second_if_overwrites` | [x] |
| 26 | `row26_all_args_int_min` | [x] |
| 27 | `row27_all_args_int_max_and_extreme_cross_product` | [x] |
| — | `generic_per_parameter_boundary_sweep` (every boundary in every position) | [x] |

### Correction to row 6 (found by exhaustive reachability search)

Row 6 as originally derived ("`diff == INT_MIN`, then negate it") is
**UNREACHABLE**. `diff` is `(m * step) / 8`, and the wrapped product is an
`i32`, so the `/ 8` confines `diff` to exactly

```
[INT_MIN/8, INT_MAX/8] == [-268435456, 268435455]
```

Hence the unary minus on C line 32 can never overflow. The test now (a) asserts
that bound holds over a 200k-sample sweep, and (b) drives the two ATTAINABLE
extremes deterministically — the window of products mapping to `INT_MIN/8` is
only 8 wide out of 2^32, so it is reached by solving `step = product * m^-1`
(mod 2^32; every multiplier `m = 2*(uni&7)+1` is odd, hence invertible) rather
than by random search.

### Note on row 25 (reachability)

The "both candidates better" case (`d1 < d0` AND `d2 < d0`) is **unreachable for
small, non-overflowing inputs**: `d(p)` is convex in `p` and `p0` lies strictly
between `p1` and `p2`, so `d0 <= max(d1, d2)`. It becomes reachable only once the
subtractions overflow, which breaks that argument. An offline 20M-sample
full-range search hits it ~18% of the time, so the test draws from the FULL
`i32` range (an early version used a small window and never reached the case).
When it occurs, the C's second `if` overwrites the first and `uni2` is returned
even if `uni1` was strictly better — the Rust reproduces this exactly.

### Harness integrity

`cargo test` does **not** rebuild a `cdylib`, so an early version of this suite
was silently testing a stale `.so` and passed even with a deliberately mutated
Rust source. Two safeguards now prevent a vacuous pass:

1. `tests/common/mod.rs::assert_fresh` fails the test if either `.so` is older
   than its sources (self-tested: touching `src/lib.rs` without rebuilding makes
   every test fail with `STALE SHARED OBJECT`).
2. `run_tests.sh` rebuilds both libraries before each test run.

Mutation-tested: changing `if d2 < d0` to `if d2 <= d0` in the Rust is caught by
19+ rows across both test files.
