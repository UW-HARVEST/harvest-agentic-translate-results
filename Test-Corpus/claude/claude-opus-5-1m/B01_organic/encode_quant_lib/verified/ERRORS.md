# ERRORS.md — Phase A: error-surface table

Mechanically derived by grepping **every** control-flow / rejection construct in
`c_src/src/lib.c`:

```sh
grep -nE 'return|assert|NULL|errno|exit|abort|RETURN_ERROR|-1' c_src/src/lib.c
```

Findings:

* `return` statements: **1** (`return (uni);` on line 61) — a plain value return,
  not an error return.
* `assert` / `NULL` / `errno` / `exit` / `abort` / error enums / `RETURN_ERROR`
  macros / `-1` sentinels: **0 occurrences**.
* Pointer parameters: **0** — the signature is
  `int encode_quant(int, int, int, int, int, int)`, so there is no null-pointer,
  length, or buffer-size check to violate.
* Range checks on inputs: **0** — no parameter is validated. `lsbit` is used as
  a mode selector but has **no** valid-variant check; every one of the 2^32 `int`
  values selects one of its branches.

**Therefore the C library has an empty rejection surface: there is no input for
which it returns an error, sets `errno`, aborts, or refuses to compute.** The
table below therefore has no "rejection" rows; instead it enumerates, one row per
construct, every place a rejection *could* have lived, together with the
behaviour the C actually exhibits — and each is covered by a differential test
that asserts C and Rust agree exactly (same returned `int`, not merely "both
did something").

## Error / rejection table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|---------------------------------------------|-------------------|
| E1 | `encode_quant` | any input at all — the single `return (uni)` (line 61) is unconditional; no error path exists | always returns an `int`; never an error sentinel. Rust must return the identical `int` |
| E2 | `encode_quant` | `lsbit` = out-of-range "enum" value with no dedicated branch, e.g. `3, 5, 6, 7, 8, 12, 100, 0x7fffffff` | no rejection: `lsbit != 0 && lsbit != 4 && (lsbit & 1)` → odd branch sets bit 0; `lsbit != 0 && lsbit != 4 && !(lsbit & 1)` → even branch clears bit 0. Returns a normal value |
| E3 | `encode_quant` | `lsbit` negative, odd (`-1, -3, -7, INT_MIN+1`) | `(-1) & 1 == 1` in two's complement → **odd** branch taken, not rejected |
| E4 | `encode_quant` | `lsbit` negative, even (`-2, -4, -8, INT_MIN`) | `& 1 == 0` → **even** branch taken. Note `-4` is **not** treated as the `lsbit == 4` special case |
| E5 | `encode_quant` | `lsbit == 4` exactly (only value hitting the special branch) | special branch: clear bit 0 then `uni |= (uni>>1)&(uni>>2)&1` on all three candidates |
| E6 | `encode_quant` | `uni == INT_MAX` → `uni + 1` (line 6) signed overflow | not checked. As compiled: wraps to `INT_MIN`; then `(uni ^ uni1) & ~7 != 0` so `uni1 = uni`. Rust must reproduce (`wrapping_add`) |
| E7 | `encode_quant` | `uni == INT_MIN` → `uni - 1` (line 7) signed overflow | not checked. Wraps to `INT_MAX`; `(uni ^ uni2) & ~7 != 0` so `uni2 = uni` |
| E8 | `encode_quant` | `uni` negative → `uni & 7`, `uni & 8`, `uni >> 1`, `uni >> 2` (lines 17-19, 30-31) | not checked. Two's-complement mask; `>>` is an **arithmetic** shift (sign-propagating) on the C implementation. Rust `i32 >>` matches |
| E9 | `encode_quant` | `step` large/extreme (`INT_MAX`, `INT_MIN`, `±0x1000_0000`) → `(2*(uni&7)+1) * step` (lines 30/36/42) signed-multiply overflow | not checked. Wraps mod 2^32; Rust must use `wrapping_mul` |
| E10 | `encode_quant` | `step == INT_MIN` and `uni & 8` set → `diff = -diff` (lines 32/38/44) negating `INT_MIN` | not checked. `-INT_MIN` wraps back to `INT_MIN`; Rust `wrapping_neg` |
| E11 | `encode_quant` | division `… / 8` (lines 30/36/42) with a negative numerator | no divide-by-zero possible (constant 8). Truncates **toward zero** in both C and Rust; must not become floor division |
| E12 | `encode_quant` | `pred` extreme → `pred + diff` (lines 33/39/45) signed overflow | not checked; wraps. Rust `wrapping_add` |
| E13 | `encode_quant` | `tgt` / `tgt2` extreme → `tgt - p0`, `tgt2 - p0` (lines 34/40/46/48/51/54) signed overflow | not checked; wraps. Rust `wrapping_sub` |
| E14 | `encode_quant` | `d0/d1/d2/d3 == INT_MIN` → `d ^ (d >> 31)` (lines 35/41/47/49/52/55) | not checked. `INT_MIN ^ -1 == INT_MAX` (a *pseudo*-abs that is off by one for negatives), never `abs()` overflow. Rust must use the same xor idiom, **not** `.abs()` |
| E15 | `encode_quant` | `d0 += d3 >> 5` (lines 50/53/56) with the pseudo-abs result | not checked; `d3 >= 0` always here, so `>> 5` is a plain divide-by-32 floor; addition can still wrap. Rust `wrapping_add` |
| E16 | `encode_quant` | tie `d1 == d0` | `<` is strict → `uni1` **not** selected (row must not become `<=`) |
| E17 | `encode_quant` | tie `d2 == d0` | `<` is strict → `uni2` **not** selected |
| E18 | `encode_quant` | both `d1 < d0` **and** `d2 < d0` | the second `if` also compares against `d0` (not the running best), so `uni2` wins unconditionally. Must not be "fixed" |
| E19 | `encode_quant` | unused local `p3` (line 5) declared and never assigned | no effect on the return value; Rust legitimately omits it |
| E20 | `encode_quant` | all six arguments simultaneously extreme (`INT_MIN`/`INT_MAX` cross-product, 4096 combos) | no rejection; some deterministic `int`. Full cross-product asserted equal |

## Status

| row | test | status |
|-----|------|--------|
| E1  | `tests/errors.rs::e1_no_error_path_ever` | [x] |
| E2  | `tests/errors.rs::e2_lsbit_out_of_range_enum_values` | [x] |
| E3  | `tests/errors.rs::e3_lsbit_negative_odd` | [x] |
| E4  | `tests/errors.rs::e4_lsbit_negative_even` | [x] |
| E5  | `tests/errors.rs::e5_lsbit_exactly_four` | [x] |
| E6  | `tests/errors.rs::e6_uni_int_max_overflow` | [x] |
| E7  | `tests/errors.rs::e7_uni_int_min_underflow` | [x] |
| E8  | `tests/errors.rs::e8_negative_uni_masks_and_arith_shifts` | [x] |
| E9  | `tests/errors.rs::e9_step_multiply_overflow` | [x] |
| E10 | `tests/errors.rs::e10_negate_int_min_diff` | [x] |
| E11 | `tests/errors.rs::e11_division_truncates_toward_zero` | [x] |
| E12 | `tests/errors.rs::e12_pred_add_overflow` | [x] |
| E13 | `tests/errors.rs::e13_tgt_sub_overflow` | [x] |
| E14 | `tests/errors.rs::e14_pseudo_abs_int_min` | [x] |
| E15 | `tests/errors.rs::e15_penalty_shift_and_add` | [x] |
| E16 | `tests/errors.rs::e16_tie_d1_eq_d0` | [x] |
| E17 | `tests/errors.rs::e17_tie_d2_eq_d0` | [x] |
| E18 | `tests/errors.rs::e18_both_better_uni2_wins` | [x] |
| E19 | `tests/errors.rs::e19_unused_p3_no_effect` | [x] |
| E20 | `tests/errors.rs::e20_all_args_extreme_cross_product` | [x] |
