# CONFIGS.md — Phase B configuration surface

Mechanically derived from `c_src/src/lib.c` (the only translation unit) and
`c_src/include/lib.h` (the only public header).

## Public entry points

`nm -D` and the header agree: there is exactly **one** public entry point, and
it is already the lowest level one — there is no convenience wrapper, no
one-shot helper, no init/destroy pair, no context object:

| entry point | signature |
|---|---|
| `hsv_to_rgb` | `void hsv_to_rgb(float *dest, const float *src)` |

## Axes the C actually branches on

Derived from every `if` / `switch` / conversion in the source; there are no
`#ifdef`s, no runtime option setters, no global/`static` state, and no
Cargo features in `translation/Cargo.toml` (so exactly one feature
combination exists — see `check_all_features.sh`).

| axis | values the C distinguishes | where |
|---|---|---|
| A1 saturation branch | `s == 0` (true, incl. `-0.0`) / `s != 0` (incl. NaN) | `lib.c:12` |
| A2 `switch` arm | `i == 0`, `1`, `2`, `3`, `4`, `default` (unsigned `ja` bound check ⇒ every negative `i` and every `i > 4`) | `lib.c:24-55` |
| A3 hue→selector conversion | `(int)floorf(h/60)` in range / NaN / `>= 2^31` / `<= -2^31` (`cvttss2si` ⇒ `INT_MIN`) | `lib.c:18-19` |
| A4 `h` value class | in `[0,360)`, exact multiple of 60 (`f == 0`), `±0`, negative, `>= 360`, subnormal, huge finite, `±inf`, quiet/signalling NaN (payloads) | `lib.c:18-20` |
| A5 `s` value class | `+0`, `-0`, subnormal, `(0,1)`, exactly `1`, `>1`, negative, `±inf`, NaN | `lib.c:12,21-23` |
| A6 `v` value class | `+0`, `-0`, subnormal, `(0,1]`, `>1`, huge, negative, `±inf`, NaN | `lib.c:13-15,21-23` |
| A7 buffer aliasing | `dest`/`src` disjoint, `dest == src`, `dest == src±1`, `dest == src±2` (the C snapshots `h,s,v` into locals before storing, so in-place is well defined) | `lib.c:8-10` vs `56-58` |
| A8 pointer alignment | 4-byte aligned / misaligned by 1,2,3 bytes (unaligned `movss`, no alignment check) | `lib.c:8-10,56-58` |
| A9 written extent | exactly `dest[0..3]`; `src` never written | `lib.c:13-15,56-58` |
| A10 call sequencing | stateless: repeated and interleaved C/Rust calls must be identical (also catches MXCSR/FTZ contamination between the two objects) | whole file |

## Configuration table (one row per combination the C treats differently)

Every row is driven with **many** randomized inputs (fixed seed, `Rng::new`)
in addition to the hand-picked boundary values, and every row asserts
bit-identical `[u32; 3]` output from the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | test | ✔ |
|---|----------------|--------------------------------------------|------|---|
| 1 | `hsv_to_rgb` | A1=`s==+0.0`; random normal `h∈[-1e3,1e3]`, `v∈[-2,2]`; disjoint aligned | `b01_s_zero_random` | [x] |
| 2 | `hsv_to_rgb` | A1=`s==-0.0`; random normal `h`, `v` | `b02_s_negative_zero_random` | [x] |
| 3 | `hsv_to_rgb` | A1=`s==±0`; A6 = every `SPECIAL` and every NaN `v` | `b03_s_zero_v_special` | [x] |
| 4 | `hsv_to_rgb` | A1=`s==±0`; A4 = every `SPECIAL`/NaN `h` (must be ignored) | `b04_s_zero_h_special` | [x] |
| 5 | `hsv_to_rgb` | A1=`s==+0`; A7 `dest == src` (in-place) | `b05_s_zero_in_place` | [x] |
| 6 | `hsv_to_rgb` | A1=`s==+0`; A7 `dest == src+1` and `src == dest+1` | `b06_s_zero_overlap_one` | [x] |
| 7 | `hsv_to_rgb` | A1=`s==+0`; A7 `dest == src+2` and `src == dest+2` | `b07_s_zero_overlap_two` | [x] |
| 8 | `hsv_to_rgb` | A1=`s==+0`; A8 misaligned `src` and `dest` (1,2,3 byte offsets) | `b08_s_zero_misaligned` | [x] |
| 9 | `hsv_to_rgb` | A1=`s==+0`; A9 canary/immutability check | `b09_s_zero_extent` | [x] |
| 10 | `hsv_to_rgb` | A2 arm 0 (`h∈[0,60)`), `s∈(0,1]`, `v∈[0,1]` random | `b10_arm0_random` | [x] |
| 11 | `hsv_to_rgb` | A2 arm 1 (`h∈[60,120)`) | `b11_arm1_random` | [x] |
| 12 | `hsv_to_rgb` | A2 arm 2 (`h∈[120,180)`) | `b12_arm2_random` | [x] |
| 13 | `hsv_to_rgb` | A2 arm 3 (`h∈[180,240)`) | `b13_arm3_random` | [x] |
| 14 | `hsv_to_rgb` | A2 arm 4 (`h∈[240,300)`) | `b14_arm4_random` | [x] |
| 15 | `hsv_to_rgb` | A2 `default` via `i==5` (`h∈[300,360)`) | `b15_arm5_default_random` | [x] |
| 16 | `hsv_to_rgb` | A2 `default` via `i>=6` (`h∈[360,3600)`, no hue wrap) | `b16_arm_ge6_default_random` | [x] |
| 17 | `hsv_to_rgb` | A2 `default` via `i<0` (`h∈(-3600,0)`) | `b17_arm_negative_default_random` | [x] |
| 18 | `hsv_to_rgb` | A4 `h` exactly `k*60`, `k∈[-64,64]` (`f == 0` boundary) × random `s`,`v` | `b18_hue_exact_multiples` | [x] |
| 19 | `hsv_to_rgb` | A4 `h` = `nextafter(k*60, ±inf)` for `k∈[-8,8]` (arm boundaries) | `b19_hue_next_to_multiples` | [x] |
| 20 | `hsv_to_rgb` | A4 `h == ±0.0`, `s∈(0,1]`, random `v` | `b20_hue_signed_zero` | [x] |
| 21 | `hsv_to_rgb` | A4 `h` subnormal / tiny (`±1e-45`, `±1e-40`, `±MIN_POSITIVE`) | `b21_hue_subnormal` | [x] |
| 22 | `hsv_to_rgb` | A3 `h` huge finite but `h/60 < 2^31` (large in-range `i`) | `b22_hue_huge_in_int_range` | [x] |
| 23 | `hsv_to_rgb` | A3 `h/60` at/just past `±2^31` ⇒ `cvttss2si` `INT_MIN` | `b23_hue_int_conversion_boundary` | [x] |
| 24 | `hsv_to_rgb` | A4 `h == ±inf` × all `s`,`v` classes | `b24_hue_infinite` | [x] |
| 25 | `hsv_to_rgb` | A4 `h` = every NaN in `NANS` (quiet, signalling, payloads) × `s`,`v` classes | `b25_hue_nan` | [x] |
| 26 | `hsv_to_rgb` | A5 `s == 1.0` exactly × all 8 arms × random `v` | `b26_s_exactly_one` | [x] |
| 27 | `hsv_to_rgb` | A5 `s` subnormal/tiny (`1e-45`, `1e-40`, `MIN_POSITIVE`) × all arms | `b27_s_subnormal` | [x] |
| 28 | `hsv_to_rgb` | A5 `s > 1` (`1.5`, `1e30`, `f32::MAX`) × all arms | `b28_s_above_one` | [x] |
| 29 | `hsv_to_rgb` | A5 `s < 0` (`-1e-45`, `-0.5`, `-1e30`, `f32::MIN`) × all arms | `b29_s_negative` | [x] |
| 30 | `hsv_to_rgb` | A5 `s == ±inf` × all arms × `v` classes (includes `0*inf`) | `b30_s_infinite` | [x] |
| 31 | `hsv_to_rgb` | A5 `s` = every NaN in `NANS` × all arms × `v` classes | `b31_s_nan` | [x] |
| 32 | `hsv_to_rgb` | A6 `v == ±0` with `s != 0` × all arms (invalid-operation NaN generation) | `b32_v_zero` | [x] |
| 33 | `hsv_to_rgb` | A6 `v` ∈ `SPECIAL ∪ NANS` × all arms × `s∈{0.25,1,1.5,-0.5,inf}` | `b33_v_special` | [x] |
| 34 | `hsv_to_rgb` | full cross-product fuzz: `h`,`s`,`v` uniform over **all** `f32` bit patterns, 300 000 triples | `b34_full_random_bitpatterns` | [x] |
| 35 | `hsv_to_rgb` | A7 `dest == src` in-place, main path, all arms × randomized `s`,`v` | `b35_in_place_main_path` | [x] |
| 36 | `hsv_to_rgb` | A7 partial overlap `dest = src±1`, `dest = src±2`, main path, randomized | `b36_overlap_main_path` | [x] |
| 37 | `hsv_to_rgb` | A8 misaligned `src`/`dest` (offsets 1,2,3), main path, randomized | `b37_misaligned_main_path` | [x] |
| 38 | `hsv_to_rgb` | A9 exact written extent + `src` immutability, main path, randomized | `b38_extent_main_path` | [x] |
| 39 | `hsv_to_rgb` | A10 statelessness: identical repeated calls, and C/Rust calls interleaved in randomized order | `b39_stateless_interleaved` | [x] |
| 40 | `hsv_to_rgb` | deterministic grid sweep `h = -720..1080 step 0.25` × `s,v ∈ {0, 1e-45, 0.25, 0.5, 0.75, 1, 1.5}` | `b40_grid_sweep` | [x] |
| 41 | `hsv_to_rgb` | NaN-payload cross product: 24 NaN encodings (quiet/signalling, both signs, min/max payloads) for `(s,v)`, `(h,s)`, `(h,v)` and `(h,s,v)`, crossed with every arm — the axis on which a real divergence was found (SSE destination-operand-wins NaN selection) | `b41_nan_payload_cross_product` | [x] |
| 42 | `hsv_to_rgb` | strided exhaustive sweep of the whole 2^32 bit-pattern space of each axis (prime stride 65521 ⇒ every exponent, both signs, subnormals, infinities, NaNs) × 6 pinned settings of the other two | `b42_strided_bitpattern_sweeps` | [x] |

Rows 1-9 cover the `s == 0` early-return configuration against every input
shape; rows 10-34 cover the main path against every value class and every
`switch` arm; rows 35-40 re-run the shape axes (aliasing / alignment / extent /
sequencing) on the main path, because the C reaches its stores through a
different code path there; rows 41-42 add depth where an actual bug was found.
