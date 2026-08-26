# CONFIGS.md — configuration-surface table (Phase B)

## How the axes were derived (mechanical)

The public API is a single entry point (`c_src/include/lib.h`):

```c
void gaussian_kernel(float *dest, int size, float radius);
```

There are **no** convenience wrappers, no context/handle object, no setters, no
runtime flags, no enums and no `#ifdef`s — so the "lowest-level entry point" and
the "one-shot wrapper" are the same function, and it is called directly in every
test below. The axes are therefore exactly the things the body of
`c_src/src/lib.c` branches on or computes differently:

| axis | where the C distinguishes it | distinct cases |
|------|------------------------------|----------------|
| `size` sign & magnitude | `hsize = size / 2` (line 10, truncation toward zero), loop bound `r <= hsize` (15), loop bound `r < size` (25) | `size <= -2` (no-op), `size == -1`, `size == 0`, `size >= 1` |
| `size` parity | taps loop writes `2*(size/2)+1` elements vs. normalisation loop covering `size` elements | odd `size` (writes exactly `size`), even `size` (writes `size+1`, last one unnormalised) |
| `radius` magnitude → `rs = sigma / radius` (12) | controls `x = r*rs` and therefore how many taps hit the `v > 0 ? v : 0` clamp (18) | `radius` ≫ 1 (no clamping), `radius ≈ sigma` (`rs == 1`), `radius` ≪ 1 (all but centre clamped), `radius = ±inf` (`rs = ±0`, flat kernel), `radius = ±0` or `|radius| < 1.6/f32::MAX ≈ 4.7e-39` (`rs = ±inf`, everything clamped incl. the centre) |
| `radius` sign | `rs` sign flips `x`, but `x*x` is even ⇒ same taps | `radius > 0`, `radius < 0`, `-0.0` |
| `radius` non-finite | NaN propagates into the clamp; `0 * inf` = NaN for the centre tap | NaN (both signs, quiet+signalling), `±inf` |
| `sum` | normalisation guard `if (sum > 0.0f)` (23) | `sum > 0` (normalise) vs `sum == 0` (skip) |
| destination buffer shape | raw `float*`, no length, no alignment check | buffer start offset (word-aligned lead padding), unaligned byte offsets, sentinel-filled guard regions before/after the written range |
| call sequence | no static/global state exists in the C | one call vs. repeated calls into the same buffer |

Every row is exercised with **many pseudo-random inputs** (fixed seed
`0x5EED_1234_ABCD_9876`, SplitMix64) rather than one hand-picked value, and every
call compares the **entire** allocation bit-for-bit (`u32` bit patterns, so
`-0.0` vs `+0.0` and NaN payloads are distinguished), including the guard words,
so any extra or missing write is caught.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|-------------------------------------------|------|-----|
| 1 | `gaussian_kernel` | odd `size` ∈ {1,3,5,…,33} × 64 random "typical" radii in [0.1, 10] | `cfg_01_odd_sizes_typical_radius` | [x] |
| 2 | `gaussian_kernel` | even `size` ∈ {2,4,…,34} × 64 random typical radii — exercises the `size+1` write overrun together with partial normalisation | `cfg_02_even_sizes_typical_radius` | [x] |
| 3 | `gaussian_kernel` | fully random property test: 4000 iterations of random `size` ∈ [-8, 512] × random radius drawn from all magnitude classes | `cfg_03_random_size_random_radius` | [x] |
| 4 | `gaussian_kernel` | `radius == sigma == 1.6f` exactly ⇒ `rs == 1.0f`, `x == r` (integer-valued taps) × sizes 0…33 | `cfg_04_radius_equals_sigma` | [x] |
| 5 | `gaussian_kernel` | `radius < 1` (narrow kernel: only the centre tap survives the clamp, tails all `+0.0`) × random radii in (0, 1) × sizes 1…33 | `cfg_05_radius_below_one_tails_clamped` | [x] |
| 6 | `gaussian_kernel` | `radius ≫ 1` (wide kernel: **no** tap is clamped, `sum` large) × random radii in [10, 1e6] × sizes 1…33 | `cfg_06_radius_large_no_clamping` | [x] |
| 7 | `gaussian_kernel` | negative radii (mirror of rows 1/6; `rs < 0` yet `x*x` even) × random radii in [-1e6, -0.1] × odd and even sizes | `cfg_07_negative_radius` | [x] |
| 8 | `gaussian_kernel` | radius = arbitrary **random 32-bit patterns restricted to finite values** (sweeps normals, subnormals, huge, ±0) × random sizes | `cfg_08_random_finite_bit_pattern_radius` | [x] |
| 9 | `gaussian_kernel` | radius = arbitrary **unrestricted random 32-bit patterns** (includes ±inf, quiet/signalling NaN) × random sizes | `cfg_09_random_any_bit_pattern_radius` | [x] |
| 10 | `gaussian_kernel` | boundary sizes `size ∈ {0, 1}` × the full special-radius set (0, -0, ±inf, NaN, subnormal, MAX, sigma, typical) | `cfg_10_size_zero_and_one_all_radii` | [x] |
| 11 | `gaussian_kernel` | negative sizes `size ∈ {-1, -2, -3, -4, -5, -17, i32::MIN+1, i32::MIN}` × special + random radii (no-op / single-unnormalised-write paths) | `cfg_11_negative_sizes` | [x] |
| 12 | `gaussian_kernel` | pre-filled destination (sentinel `0xCAFEBABE` words) — asserts the guard words before `dest` and after the written range are byte-identical in both libraries, i.e. neither writes more nor fewer elements | `cfg_12_guard_regions_preserved` | [x] |
| 13 | `gaussian_kernel` | `dest` pointing **into the middle** of a bigger allocation, with word lead offsets 0…7 × random sizes/radii | `cfg_13_dest_offset_inside_buffer` | [x] |
| 14 | `gaussian_kernel` | **unaligned** `dest`: byte offsets 1, 2, 3 into a byte buffer × sizes 1…9 × random radii (no alignment check exists in the C) | `cfg_14_unaligned_dest` | [x] |
| 15 | `gaussian_kernel` | repeated invocation: 3 back-to-back calls with different `(size, radius)` into the same buffer, comparing after each — proves no hidden static state in either library | `cfg_15_repeated_calls_same_buffer` | [x] |
| 16 | `gaussian_kernel` | large sizes `{255, 256, 257, 1023, 1024, 4095, 4096, 65535, 65536, 65537}` × random radii — the deepest accumulation of `sum` (float addition order must match exactly) | `cfg_16_large_sizes` | [x] |
| 17 | `gaussian_kernel` | radius small enough that `rs` overflows (`|radius| < 4.7e-39`), or `±0`/NaN ⇒ **every** tap incl. the centre clamps ⇒ `sum == 0` ⇒ normalisation skipped, buffer left all `+0.0f` | `cfg_17_all_taps_clamped_no_normalisation` | [x] |
| 18 | `gaussian_kernel` | radius = exact powers of two `2^-30 … 2^30` (exact `rs`, exercises `x*x` over the full exponent range incl. overflow of `x*x` to `inf`) × sizes 1…17 | `cfg_18_power_of_two_radii` | [x] |
| 19 | `gaussian_kernel` | radius at the float extremes and on **both sides of the `rs = sigma/radius` overflow threshold** (`1.6/f32::MAX ≈ 4.7e-39`): `±MIN_POSITIVE`, `1e-45`, `5e-39`, `4.7e-39`, `4e-39`, `1e-39`, `MAX`, `MIN`, `±1e38`, `EPSILON` × sizes 0…9 | `cfg_19_extreme_radii` | [x] |
| 20 | `gaussian_kernel` | dense cross-product matrix: every `size ∈ -4..=40` × 24-radius set — the full pruned cross product of the size and radius axes | `cfg_20_size_radius_cross_product` | [x] |

All 20 rows pass under the single (default) feature combination, which is the
only build configuration — see `SYMBOLS.md`.
