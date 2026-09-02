# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

## Mechanical derivation

Greps over the complete C source (`c_src/src/lib.c`, 28 lines;
`c_src/include/lib.h`, 1 line):

| grep pattern | hits |
|--------------|------|
| `return` | **0** |
| `assert` | **0** |
| `NULL` | **0** |
| `errno` | **0** |
| `-1` (error sentinel) | **0** |
| `ERROR` / `RETURN_ERROR` / error enums | **0** |
| `MIN` / `MAX` constants | **0** |
| `if` | 1 — `if (sum > 0.0f)` (line 23) |
| `for` | 2 — lines 15, 25 |
| `?:` | 1 — `v = (((v) > (0)) ? (v) : (0));` (line 18) |

`gaussian_kernel` returns `void`, has **no return-code channel, no output error
flag, no asserts, and no pointer/range validation whatsoever.** So the entire
rejection surface is *implicit*: the function "rejects" bad input by silently
taking a degenerate branch (writing nothing, writing `0.0f`, or skipping
normalization) or by propagating an IEEE-754 special value. Each row below is
one distinct such rejection, taken from an actual branch condition or an actual
arithmetic operation in the source — not from documentation.

The "expected C result" column is what the C **must** do, and each row's
differential test asserts the Rust `.so` produces the *same* observable state
(same `f32` bit patterns in the destination buffer, same set of bytes left
untouched), not merely "both did something".

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `gaussian_kernel` | `size <= -2` ⇒ `hsize = size/2 < 0` ⇒ loop guard `-hsize <= hsize` false on entry (line 15) | **zero iterations, zero stores**; `dest` left completely untouched; `sum` stays `0.0f` so line 23 is false. Reachable with `dest == NULL` without any dereference. |
| E2 | `gaussian_kernel` | `dest == NULL` **combined with** `size <= -2` (the only `size` class that performs no store — see E1) | no dereference, no crash, returns normally. Must be identical in Rust (no eager null-check / no `assume`). |
| E3 | `gaussian_kernel` | `dest == NULL` with `size >= -1` | unchecked dereference at line 19 ⇒ UB/SIGSEGV in C. Rust must be *equally* unchecked (no defensive early return that would make Rust survive where C dies). Verified out-of-process. |
| E4 | `gaussian_kernel` | `size == 0` ⇒ `hsize = 0` ⇒ loop still runs once for `r == 0` (line 15 is `<=`) | writes **1** element (`dest[0]`) even though caller asked for size 0 — a **one-element out-of-bounds store** for a 0-length buffer. Then `if (sum > 0)` is true but `for (r=0; r<0)` never runs, so `dest[0]` is left **un-normalised** (`1.0f - s2`). |
| E5 | `gaussian_kernel` | `size == -1` ⇒ `hsize = 0` (C division truncates toward zero, **not** floor) | same as E4: one store at `dest[0]`, value `1.0f - s2`, no normalisation (`r < -1` false). Distinct row because a floor-division translation would give `hsize == -1` and zero stores. |
| E6 | `gaussian_kernel` | any **even** `size > 0` ⇒ `2*hsize + 1 == size + 1` | loop stores **size+1** elements: a **one-past-the-end write** of `dest[size]`. The normalisation loop then only scales `dest[0..size)`, so `dest[size]` is left un-normalised. Rust must commit the same overrun. |
| E7 | `gaussian_kernel` | `radius == 0.0f` ⇒ `rs = sigma/0.0f = +inf` (division by zero, no guard at line 12) | `r == 0`: `x = 0 * inf = NaN`, `expf(NaN)=NaN`, `v = NaN`; **`NaN > 0` is false** so line 18 stores `0.0f` (not NaN). `r != 0`: `x = ±inf`, `v = 0 - s2 < 0` ⇒ `0.0f`. All stores `0.0f`, `sum == 0.0f` ⇒ **no normalisation**. Whole buffer `+0.0f`. |
| E8 | `gaussian_kernel` | `radius == -0.0f` | `rs = -inf`; identical outcome to E7 (`x*x` kills the sign). All `+0.0f`, no normalisation. |
| E9 | `gaussian_kernel` | `radius == NaN` | `rs = NaN`, `x = NaN`, `v = NaN`, clamp ⇒ `0.0f` for **every** `r` including `r == 0`; `sum == 0` ⇒ no normalisation. Buffer all `+0.0f`. |
| E10 | `gaussian_kernel` | `radius == ±inf` ⇒ `rs = ±0.0f` | `x = ±0.0` for every `r`, `v = 1.0f - s2 > 0` for every `r` ⇒ flat kernel, `sum > 0` ⇒ normalised to `1/(2*hsize+1)` — note this is `1/(size+1)`, *not* `1/size`, for even `size`. |
| E11 | `gaussian_kernel` | `radius` subnormal (e.g. `1e-45`) ⇒ `rs = sigma/radius` overflows to `+inf` | same degenerate all-zero result as E7 (overflow in `sigma/radius` is silent). **Measured caveat:** only *small* subnormals overflow — the largest subnormal (`≈1.1755e-38`) gives `rs ≈ 1.36e38`, still finite, so that sub-case lands in E12's Dirac-spike regime instead. The test asserts both sub-cases and requires each to occur. |
| E12 | `gaussian_kernel` | `radius` so small that `\|r\|*rs >= 2.4` for all `r != 0` but `r == 0` still gives `v = 1-s2 > 0` | `sum > 0` ⇒ normalisation runs; result is a **Dirac spike** `dest[hsize] == 1.0f`, all other elements `+0.0f`. Boundary between E7 (no normalisation) and the normal path. |
| E13 | `gaussian_kernel` | `radius` huge but finite (e.g. `1e30`) ⇒ `rs` underflows to a subnormal / `0.0f` | flat kernel, same as E10 but reached by underflow rather than by an infinite `radius`. |
| E14 | `gaussian_kernel` | `\|x\| == 2.4` exactly ⇒ `x*x == 5.76f == sigma*sigma*tetha` ⇒ `v == 1/expf(5.76f) - s2 == +0.0` exactly. Reached at `radius == \|r\| * (2/3)`; **measured:** many but not all integer `r` land exactly on it after rounding | clamp condition is **strict** `> 0`, so an exactly-zero `v` takes the `else` arm and stores the integer literal `0` ⇒ `+0.0`, never `-0.0`. Asserted bitwise, and the test requires the exact-zero path to be hit at least 8 times so it cannot pass vacuously. |
| E15 | `gaussian_kernel` | `radius < 0` (negative but normal, e.g. `-3.0f`) — never rejected by the C | `rs < 0` but `x*x` is even ⇒ result **bit-identical** to `+\|radius\|`. Not an error in C; asserted so the Rust does not "validate" it. |
| E16 | `gaussian_kernel` | `size == INT_MIN` ⇒ `hsize = -1073741824`, `-hsize = +1073741824` (no signed-overflow trap; `INT_MIN/2` is well defined) | falls into E1: zero stores. Confirms the negation and the truncating division do not diverge at the extreme. |
| E17 | `gaussian_kernel` | `size == 1` (minimum size that produces exactly one normalised element) | `hsize = 0`, one store, `sum = 1-s2 > 0`, normalisation runs once ⇒ `dest[0] == 1.0f` exactly for **every** finite non-zero `radius`. |
| E18 | `gaussian_kernel` | `size == 2` (smallest even ⇒ smallest overrun) | 3 stores (`dest[0..2]`), only `dest[0..1]` normalised. Smallest instance of E6. |
| E19 | `gaussian_kernel` | out-of-range "enum"-style ints passed for `size` (the only integer parameter): `size` values with no meaningful kernel interpretation — `-3`, `-2`, `-1`, `0`, `INT_MIN`, `INT_MIN+1` | C accepts every `int`; each maps to one of E1/E4/E5/E16. The FFI signature has no enum, so this row is the equivalent "no valid variant" check: Rust must accept and reproduce all of them rather than panicking. |
| E20 | `gaussian_kernel` | `sum` becomes a tiny positive subnormal ⇒ `isum = 1.0f/sum` overflows to `+inf` (line 24, unguarded) | **UNREACHABLE — proven by search, not assumed.** A positive `sum` is a sum of values `1/expf(x*x) - s2`; since `s2 ≈ 3.15e-3`, the smallest representable positive `v` is about one ulp at that magnitude (`≈2.3e-10`), so `sum` can never be small enough for `1.0f/sum` to overflow. `e20_*` walks 48 000 radii ULP-by-ULP around the clamp threshold plus a randomized sweep, asserts C/Rust parity on every one, records the smallest positive `sum` actually achievable, and asserts it is many orders of magnitude above the overflow threshold. Documented as unreachable rather than faked as passing. |

**Total: 20 rows.** Every row is exercised by
`translation/tests/errors.rs` (E3 by `translation/tests/null_deref.rs`, which
forks so the SIGSEGV does not take the harness down).

## Results

All 20 rows pass. `E1..E2, E4..E20` are in `translation/tests/errors.rs`
(19 tests); `E3` is in `translation/tests/null_deref.rs`.

| row | test | status |
|-----|------|--------|
| E1 | `e01_size_le_minus_two_performs_zero_stores` | pass |
| E2 | `e02_null_dest_with_negative_size_is_survivable_in_both` | pass |
| E3 | `e03_null_dereference_matches_c` (out-of-process) | pass |
| E4 | `e04_size_zero_stores_one_unnormalised_element` | pass |
| E5 | `e05_size_minus_one_truncates_toward_zero` | pass |
| E6 | `e06_even_size_overruns_by_one_element` | pass |
| E7 | `e07_radius_positive_zero_yields_all_positive_zero` | pass |
| E8 | `e08_radius_negative_zero_yields_all_positive_zero` | pass |
| E9 | `e09_radius_nan_clamps_every_tap` | pass |
| E10 | `e10_radius_infinite_gives_flat_kernel_normalised_by_2hsize_plus_1` | pass |
| E11 | `e11_subnormal_radius_overflows_rs_to_infinity` | pass |
| E12 | `e12_dirac_spike_regime` | pass |
| E13 | `e13_huge_finite_radius_underflows_rs` | pass |
| E14 | `e14_exactly_zero_v_takes_the_else_arm` | pass |
| E15 | `e15_negative_radius_is_not_rejected` | pass |
| E16 | `e16_size_int_min_does_not_overflow` | pass |
| E17 | `e17_size_one_normalises_to_exactly_one` | pass |
| E18 | `e18_size_two_smallest_overrun` | pass |
| E19 | `e19_every_int_size_is_accepted_identically` | pass |
| E20 | `e20_reciprocal_of_sum_overflow_is_unreachable_but_parity_holds` | pass (row proven unreachable) |

### The one real divergence found and fixed

`e03_null_dereference_matches_c` failed against the **debug** Rust `.so`:

```
size=-1: C = Signal(11) [SIGSEGV]   Rust = Signal(6) [SIGABRT]
  thread '<unnamed>' panicked at src/lib.rs:38:13:
  null pointer dereference occurred
```

rustc's debug-only UB checks intercepted the unchecked `*k = v` store that the
C performs blind, converting the C's segfault into a controlled abort. Fixed in
`Cargo.toml` by disabling `debug-assertions`/`overflow-checks` on the `dev`
profile (the `release` profile was already unaffected), so both profiles now
reproduce the C's semantics. This is exactly the class of bug that only an
out-of-process differential test can see.
