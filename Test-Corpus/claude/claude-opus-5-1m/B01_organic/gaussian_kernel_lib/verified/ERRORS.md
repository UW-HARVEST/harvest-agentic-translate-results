# ERRORS.md — error/rejection surface table (Phase C)

## How this table was derived (mechanical)

```sh
grep -nE 'return|assert|NULL|errno|-1|exit|abort|if|\?|<=|>=|<|>|MAX|MIN|#if' \
     c_src/src/lib.c c_src/include/lib.h
```

Complete set of matches in `c_src/src/lib.c`:

| line | construct |
|------|-----------|
| 15 | `for (r = -hsize; r <= hsize; r++)` — kernel-taps loop bound |
| 18 | `v = (((v) > (0)) ? (v) : (0));` — negative/NaN clamp |
| 23 | `if (sum > 0.0f)` — normalisation guard |
| 25 | `for (r = 0; r < size; r++)` — normalisation loop bound |

There are **no** `return` statements (the function is `void`), **no** error
codes, **no** `errno` use, **no** `assert`, **no** `NULL` checks, **no** enums,
**no** `MIN`/`MAX` constants and **no** `#if`/`#ifdef` in the library. The
public header declares one `void` function, so *the C API has no error-reporting
channel at all*.

Consequently every row below is a **degenerate/invalid-input rejection path**:
the exact condition under which the C code declines to do the "normal" thing
(clamps to `0.0f`, skips normalisation, writes nothing, or reads/writes out of
bounds). "Expected C result" is the observable behaviour — the byte pattern
written into the caller's buffer — because that is the only result the API has.
The Rust translation must reproduce each one bit-for-bit.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|---------------------------------------------|-------------------|------|--------|
| 1 | `gaussian_kernel` | line 18 clamp: computed tap `v = 1/expf(x*x) - s2` is **negative** (`x*x` large enough that `expf(-x²) < s2`, i.e. taps in the tails) | tap stored as exactly `+0.0f` (bit pattern `0x00000000`), and `0.0f` added to `sum` — never a negative tap | `err_01_negative_tap_clamped_to_zero` | [x] |
| 2 | `gaussian_kernel` | line 18 clamp with **NaN**: `v` is NaN (arises when `radius == ±0.0f` or `|radius| < 1.6/f32::MAX ≈ 4.7e-39` ⇒ `rs = ±inf` ⇒ `x = 0 * ±inf = NaN` for `r == 0`, or when `radius` is NaN) | `comiss`+`jbe` ⇒ NaN comparison false ⇒ tap stored as `+0.0f`; NaN is **not** propagated to the buffer or to `sum` | `err_02_nan_tap_clamped_to_zero` | [x] |
| 3 | `gaussian_kernel` | line 23 guard fails because **every tap clamped to 0** ⇒ `sum == 0.0f`. Reachable only when `rs` is non-finite so the centre tap is NaN: `radius == ±0.0f`, `|radius| < 1.6/f32::MAX ≈ 4.7e-39` (deep subnormals such as `1e-45`), or `radius` NaN — with `size >= -1`. NB `f32::MIN_POSITIVE` (1.175e-38) is **not** small enough: `rs = 1.36e38` stays finite, so its centre tap survives and normalisation *does* run | normalisation loop skipped entirely: buffer left as all `+0.0f`, **no division by zero**, no `inf`/NaN written | `err_03_sum_zero_skips_normalisation` | [x] |
| 4 | `gaussian_kernel` | line 23 guard fails because the **taps loop never executed** ⇒ `sum` still `0.0f` (any `size <= -2`, since `hsize = size/2 <= -1` makes `-hsize > hsize`) | function is a complete **no-op**: not one byte of `dest` is touched, no normalisation | `err_04_size_le_minus2_is_noop` | [x] |
| 5 | `gaussian_kernel` | `dest == NULL` combined with row 4's condition (`size <= -2`) — the only `NULL` case the C survives, as there is no null check | no dereference happens, function returns normally without faulting | `err_05_null_dest_with_noop_size` | [x] |
| 6 | `gaussian_kernel` | `dest == NULL` with `size >= -1` | dereferences NULL → **SIGSEGV (undefined behaviour)**; identical UB in Rust. Not exercised as a differential assertion (crashing the harness proves nothing); documented and asserted only for the survivable case in row 5 | documented; see `err_05_*` note | [x] |
| 7 | `gaussian_kernel` | `size == -1` (odd negative): `hsize = 0` ⇒ taps loop writes `dest[0]`, then line 25 loop `r < -1` never runs | `dest[0]` = **unnormalised** `1/expf(0) - s2` = `1.0f - s2`; `dest[1..]` untouched | `err_07_size_minus1_writes_one_unnormalised` | [x] |
| 8 | `gaussian_kernel` | `size == 0`: `hsize = 0` ⇒ taps loop still writes `dest[0]` (**one element past the caller's zero-length buffer** — a genuine C overrun), `sum > 0`, then line 25 loop `r < 0` never runs | `dest[0]` = unnormalised `1.0f - s2`; nothing normalised | `err_08_size_zero_writes_one_past_end` | [x] |
| 9 | `gaussian_kernel` | **even `size >= 2`**: taps loop runs `2*(size/2)+1 == size+1` times, so it writes `dest[size]` — one element **beyond** the `size`-element buffer; the normalisation loop only covers `0..size-1` | `dest[0..size-1]` normalised (sum 1.0), plus a stray **unnormalised** tap written at `dest[size]`; overrun by exactly one `float` | `err_09_even_size_overruns_by_one` | [x] |
| 10 | `gaussian_kernel` | `radius == 0.0f` (no divide-by-zero check on line 12 `rs = sigma / radius`) | `rs = +inf`; per rows 2–3 the whole buffer becomes `+0.0f` (`2*(size/2)+1` zeros), unnormalised | `err_10_radius_zero` | [x] |
| 11 | `gaussian_kernel` | `radius == -0.0f` | `rs = -inf`; same all-`+0.0f` result as row 10 | `err_11_radius_negative_zero` | [x] |
| 12 | `gaussian_kernel` | `radius` NaN (quiet **and** signalling bit patterns, both signs) | `rs` NaN ⇒ every tap NaN ⇒ clamped to `+0.0f` (row 2) ⇒ `sum == 0` ⇒ no normalisation | `err_12_radius_nan` | [x] |
| 13 | `gaussian_kernel` | `radius = ±inf` | `rs = ±0.0f` ⇒ `x = 0` (or `-0.0`) for every tap ⇒ every tap `1.0f - s2` ⇒ `sum > 0` ⇒ every in-range element normalised to `1/(2*hsize+1)` | `err_13_radius_infinite` | [x] |
| 14 | `gaussian_kernel` | `radius` a **deep subnormal** (`±1e-45`, `f32::from_bits(1)`, `±f32::MIN_POSITIVE`) ⇒ `sigma/radius` either **overflows** to `±inf` (when `|radius| < 1.6/f32::MAX ≈ 4.7e-39`) or stays finite-but-enormous (`f32::MIN_POSITIVE` ⇒ `rs = 1.36e38`) | overflow case: all `+0.0f`, no normalisation (row 3). Finite-`rs` case: centre tap survives ⇒ normalised centre `1.0f`, tails `+0.0f`. Both boundaries are exercised | `err_14_radius_subnormal_overflow` | [x] |
| 15 | `gaussian_kernel` | `radius` huge (`f32::MAX`) ⇒ `rs` subnormal/0 ⇒ all taps identical | all in-range elements normalised to `1/(2*hsize+1)`; no overflow trap | `err_15_radius_huge` | [x] |
| 16 | `gaussian_kernel` | `size == i32::MIN` — extreme boundary of the `int` parameter; `hsize = -1073741824`, `-hsize` does **not** overflow | no-op (row 4); notably Rust must not panic on the `-hsize` negation or on `size / 2` in a debug build | `err_16_size_int_min` | [x] |
| 17 | `gaussian_kernel` | `size` one step past each interesting boundary: `-3, -2, -1, 0, 1, 2, 3` | per-row behaviour above; `size == 1` and `size == 3` normalise fully, `size == 2` overruns, `size <= -2` no-ops | `err_17_size_boundary_sweep` | [x] |
| 18 | `gaussian_kernel` | "out-of-range enum" analogue: the C API declares **no enum/flag parameter**, so the only cross-FFI integer whose domain is unconstrained is `int size`. Arbitrary/garbage `int` values (including huge positives, `i32::MIN`, `i32::MIN+1`, `-1`) are pushed across the boundary | must not be rejected or trapped: `size/2` truncates toward zero exactly as C does; behaviour matches rows 4/7/8/9/16. (Large positive `size` values are exercised up to a memory-safe bound.) | `err_18_arbitrary_int_size_values` | [x] |
| 19 | `gaussian_kernel` | oversized `size` (e.g. `i32::MAX`) with a buffer that cannot hold `size+1` floats | C walks off the end and faults — unbounded UB in both implementations; not asserted differentially. Largest *memory-safe* sizes (up to 65537) are covered instead | documented; `cfg_16_large_sizes` | [x] |
| 20 | `gaussian_kernel` | **unaligned** `dest` (byte-offset `float*`) — no alignment check anywhere in the C | on x86-64 both emit plain `movss`, so both produce the identical bytes; exercised for all 1–3 byte offsets | `err_20_unaligned_dest` | [x] |

All 20 rows have a passing differential test (or, for rows 6 and 19, a
documented UB rationale plus the closest survivable assertion), see
`tests/differential.rs`.
