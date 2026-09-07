# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (28 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep result

```
$ grep -nE 'return|assert|NULL|errno|RETURN_ERROR|MAX|MIN' c_src/src/lib.c c_src/include/lib.h
(no matches other than the #include line)
```

There is **no `return` statement, no error enum, no `assert`, no null check,
no explicit range check, and no min/max constant** anywhere in the C. The
function signature is `void`, so it has **no error channel at all**: every
input is "accepted" and the observable result is whatever it writes into
`dest` (or a crash / UB).

Therefore the "error surface" is entirely made of *implicit* rejections: the
guard conditions the C actually evaluates (`v > 0`, `sum > 0.0f`, the two loop
bounds) plus the degenerate arithmetic cases. Each of those is one row below.
"Expected C result" is stated as the observable post-state of `dest`, because
that is the only output.

Notation: `hsize = size / 2` (C truncating division). The write loop
`for (r = -hsize; r <= hsize; r++)` performs `2*hsize + 1` writes when
`hsize >= 0`, starting at `dest[0]`. The normalize loop touches `dest[0 ..
size)`. `s2 = 1.0f / expf(1.6f*1.6f*2.25f)`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `gaussian_kernel` | `size <= -2` (so `hsize <= -1`, hence `-hsize > hsize`): write loop guard `r <= hsize` false on entry | zero iterations; `sum` stays `0.0f`; `sum > 0.0f` false; normalize loop `r < size` false. **No memory touched at all** — `dest` untouched. |
| 2 | `gaussian_kernel` | `dest == NULL` **and** `size <= -2` | no dereference happens (row 1), so this must **return cleanly, not fault**, in both C and Rust |
| 3 | `gaussian_kernel` | `size == -1` (`hsize == 0`) | write loop runs once → `dest[0] = clamp(1-s2)`; `sum > 0` true; normalize loop `r < -1` runs 0 times → `dest[0]` left **un-normalized** |
| 4 | `gaussian_kernel` | `size == 0` (`hsize == 0`) | write loop runs once → `dest[0]` written (**1 element written for a size-0 request**); normalize loop runs 0 times → `dest[0]` un-normalized |
| 5 | `gaussian_kernel` | any **even** `size >= 2`: `2*hsize+1 == size+1` | writes `size+1` floats — a **one-element out-of-bounds write** past `dest[size-1]`; then normalizes only `dest[0..size)`, leaving `dest[size]` un-normalized. Must be reproduced exactly. |
| 6 | `gaussian_kernel` | `radius == 0.0f` → `rs = 1.6f/0.0f = +inf`; `r == 0` gives `x = 0*inf = NaN` | `x*x` = NaN → `expf(NaN)`=NaN → `v = NaN - s2` = NaN → `NaN > 0` **false** → `v = 0`. Non-zero `r` gives `x = ±inf`, `x*x = +inf`, `1/inf = 0`, `v = -s2 < 0` → `0`. All entries `0.0f`, `sum == 0.0f` → `sum > 0.0f` false → **no normalization**; buffer is all `+0.0f`. |
| 7 | `gaussian_kernel` | `radius == -0.0f` → `rs = -inf` | same as row 6 (all `+0.0f`, no normalization) |
| 8 | `gaussian_kernel` | `radius` = NaN → `rs` = NaN → `x` = NaN → `v` = NaN | every `v > 0` comparison false → all `0.0f`; `sum = 0.0f`; no normalization |
| 9 | `gaussian_kernel` | `radius = +inf` → `rs = 1.6/inf = +0.0` → every `x = 0` | every `v = 1 - s2` (positive, identical); `sum = size_written * v > 0` → normalization → each normalized entry `= 1/(2*hsize+1)` (up to float rounding) |
| 10 | `gaussian_kernel` | `radius = -inf` → `rs = -0.0` → `x = r * -0.0 = ∓0.0`, `x*x = +0.0` | identical to row 9 |
| 11 | `gaussian_kernel` | `radius` tiny positive (e.g. `1e-30`, or any denormal) → `rs` huge → `x*x` overflows to `+inf` for **all** `r != 0` | `r != 0` → `v = 0 - s2 < 0` → clamped `0`; `r == 0` → `v = 1 - s2 > 0`. `sum > 0` → normalization makes centre `1.0f` and everything else `0.0f` |
| 12 | `gaussian_kernel` | `radius` very large finite (`3.4e38`) → `rs` denormal-ish/`+0`-ish | same class as row 9; must match bit-for-bit including any denormal `rs` |
| 13 | `gaussian_kernel` | `radius < 0` (ordinary negative, e.g. `-2.5`) → `rs < 0` but `x*x` cancels the sign | result **identical** to `+2.5`; the C never rejects negative radius |
| 14 | `gaussian_kernel` | the `v > 0` clamp branch itself: some `r` yield `v <= 0` while others yield `v > 0` (mixed, ordinary `radius`) | exactly the entries with `1/expf(x*x) <= s2` become `+0.0f`; note `v == 0.0f` exactly also takes the `else` arm (`0 > 0` false) |
| 15 | `gaussian_kernel` | `sum > 0.0f` false via **all** entries clamped to zero (rows 6–8, 11-with-small-window) | normalization skipped entirely — the un-normalized buffer is the output |
| 16 | `gaussian_kernel` | `size == INT_MIN` (`-2147483648`) → `hsize = -1073741824`, `-hsize = +1073741824` (no signed-overflow in C; Rust must not panic/wrap differently) | falls into row 1: zero iterations, `dest` untouched, safe with `dest == NULL` |
| 17 | `gaussian_kernel` | `size == 1` (smallest positive; `hsize == 0`) | one write, `sum > 0`, normalize `dest[0] = 1.0f` exactly |
| 18 | `gaussian_kernel` | `radius` = smallest positive subnormal (`f32::from_bits(1)`) → `rs = 1.6/5e-45 = +inf` | degenerates to row 6's `+inf` `rs` path: `r==0` → `x = 0*inf = NaN` → clamp `0`; all zero, `sum == 0`, no normalization |
| 19 | `gaussian_kernel` | out-of-range "enum"-like `int` for `size`: values with no meaningful kernel (`-3`, `-2`, `INT_MIN+1`, `INT_MAX`) crossing the FFI boundary as raw `int` | `size` is a plain `int` with no validation; C accepts every bit pattern. Negative ≤ -2 → row 1. (`INT_MAX` is not exercised for the write path since `2*(INT_MAX/2)+1` floats cannot be allocated; the *guard evaluation* is still verified via the `dest == NULL`-safe subset.) |

## Gate — row → test mapping (all passing)

Each row has a differential test in `tests/error_paths.rs` asserting C and Rust
agree on the observable result (bitwise on `dest`, plus "returned without
faulting"). Both sides are called through their `.so` exports via `libloading`.

| ERRORS.md row(s) | test |
|---|---|
| 1 | `err01_size_le_minus_two_touches_nothing` |
| 2, 16 | `err02_null_dest_with_no_write_is_clean` |
| 3 | `err03_size_minus_one_writes_one_unnormalized` |
| 4 | `err04_size_zero_writes_one_element` |
| 5 | `err05_even_size_writes_one_past_end` |
| 6, 7 | `err06_07_radius_zero_and_negative_zero` |
| 8 | `err08_radius_nan_payloads` (10 quiet/signaling NaN encodings, both signs) |
| 9, 10 | `err09_10_radius_infinite` |
| 11 | `err11_tiny_radius_only_centre_survives` |
| 12 | `err12_huge_finite_radius` |
| 13 | `err13_negative_radius_matches_positive` |
| 14 | `err14_clamp_branch_mixed` |
| 15 | `err15_sum_zero_skips_normalization` |
| 16 | `err16_int_min_size` |
| 17 | `err17_size_one_normalizes_to_exactly_one` |
| 18 | `err18_subnormal_radius_makes_rs_infinite` |
| 19 | `err19_arbitrary_int_bit_patterns_for_size` |

Generic C-API boundaries, covered even though not derived from an explicit
check in the C:

| boundary | test |
|---|---|
| NULL pointer (in the provably no-write regime) | `err02_...`, `err19_...(a)`, `fuzz_null_dest_no_write_regime` |
| zero length (`size == 0`) | `err04_...` |
| negative / oversized length (`INT_MIN`, `-INT_MAX`, 65536, 100001) | `err16_...`, `generic_oversized_size_guard_only` |
| one step past every interesting `size` (`-3..5`) | `err19_...(c)`, `row16_...` |
| every `f32` exponent field × sampled mantissas × both signs (the float analogue of an out-of-range enum value: bit patterns with no "valid" meaning, incl. inf, all-NaN classes, and subnormals) | `generic_full_exponent_sweep_for_radius` |
| raw out-of-range `int` bit patterns for `size` across the FFI boundary | `err19_arbitrary_int_bit_patterns_for_size` (4000 randomized + 2000 null-dest) |

### Two test expectations were wrong; the C was right

While building this table, `err09_10` and `err18` initially failed. Both were
**incorrect assumptions in the test**, not divergences — `assert_same` (the
C↔Rust comparison) passed in both cases. Verified against the C directly:

- `radius = inf, size = 2` → `dest = [0.333333343, 0.333333343, 0.996848881]`.
  The third element is the out-of-bounds write, and the normalize loop only
  covers `[0, size)`, so it stays **un-normalized**. The test now expects that.
- `radius = f32::from_bits(0x003FFFFF)` (a *large* subnormal) → `rs = 1.6f /
  5.877e-39 = 2.72e38`, which is **finite**. Only the small subnormals overflow
  `rs` to infinity. The test now gates the all-zero expectation on
  `(1.6f32 / radius).is_infinite()`.

No Rust code was changed as a result of either.

