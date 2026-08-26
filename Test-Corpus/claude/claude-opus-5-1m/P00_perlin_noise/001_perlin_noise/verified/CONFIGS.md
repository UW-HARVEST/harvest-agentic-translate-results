# CONFIGS.md — configuration surface for VALID inputs (Phase B)

Axes derived mechanically from the C source (`c_src/src/stb_perlin.h`,
`c_src/src/main.c`) — every runtime option the public API can set and every
input shape the code branches on:

* **entry points** (all nine exported symbols, lowest level first):
  `stb_perlin_noise3_internal` (the one every other noise function funnels
  into), `stb_perlin_noise3`, `stb_perlin_noise3_seed`,
  `stb_perlin_ridge_noise3`, `stb_perlin_fbm_noise3`,
  `stb_perlin_turbulence_noise3`, `stb_perlin_noise3_wrap_nonpow2`, `inner`
  (the `switch` dispatcher) and `main` (the `scanf`/`printf` driver).
* **wrap options** (`x_wrap`, `y_wrap`, `z_wrap`): `0` ("don't care" → mask
  `255`, and `256` in the non-pow2 variant), powers of two `1..256` (the
  documented contract), powers of two `> 256`, non-powers of two, negative
  values, `INT_MIN`/`INT_MAX`, and per-axis mixtures (the three axes are
  masked independently).
* **seed**: `unsigned char` `0` (what `stb_perlin_noise3` hard-codes),
  `1..=255`, the `int`→`unsigned char` truncation of
  `stb_perlin_noise3_seed`/`inner`, and the octave counter `(unsigned char)i`
  used by the three fractal functions.
* **coordinate shape** (`x`, `y`, `z`): exact integers (`x -= px` becomes `0`),
  fractional, negative, `±0`, values just below an integer, large finite values,
  values outside `int` range, `±inf`, `NaN`, subnormals — plus per-axis mixtures.
* **fractal options**: `octaves` (`0`, `1`, small, large enough that
  `frequency` overflows), `lacunarity`, `gain` (canonical `2.0`/`0.5`, `1.0`,
  `0`, negative, huge) and `offset` (ridge only: `1.0`, `0`, negative, huge).
* **`which`** for `inner`: `0..=5` (one code path each) and anything else.
* **driver input shapes** for `main`: token count `0..12+`, whitespace kinds,
  decimal / exponent / hex-float / `inf` / `nan` spellings, magnitudes that
  overflow `float`, values that make `%.9g` switch between `%e` and `%f` style.

Every row is checked with **many randomised inputs** (`common::Rng`,
splitmix64, fixed seeds) and compared bit-for-bit (`f32::to_bits`) between the
C `.so` and the Rust `.so`, both loaded with `libloading`.

| # | entry point(s) | configuration (options set + input shape) | verified | test |
|---|----------------|-------------------------------------------|-----|------|
| C1 | `stb_perlin_noise3_internal` | wraps `(0,0,0)`, `seed=0`, random fractional coords in `[-4,4]` | [x] | `phase_b_noise3::c1_internal_no_wrap_seed0` |
| C2 | `stb_perlin_noise3_internal` | wraps `(0,0,0)`, random `seed` in `0..=255`, random fractional coords | [x] | `phase_b_noise3::c2_internal_no_wrap_random_seed` |
| C3 | `stb_perlin_noise3_internal` | wraps `(0,0,0)`, `seed ∈ {0,255}`, coords exactly integral (`x-px == 0`) and `±0` | [x] | `phase_b_noise3::c3_internal_integral_coords` |
| C4 | `stb_perlin_noise3_internal` | all three wraps the same power of two `1,2,4,…,256`, random seed/coords | [x] | `phase_b_noise3::c4_internal_pow2_wraps` |
| C5 | `stb_perlin_noise3_internal` | per-axis *different* powers of two (e.g. `4,8,16`), random seed/coords | [x] | `phase_b_noise3::c5_internal_mixed_pow2_wraps` |
| C6 | `stb_perlin_noise3_internal` | wrap `= 1` on one/two/three axes (mask `0`) | [x] | `phase_b_noise3::c6_internal_wrap_one` |
| C7 | `stb_perlin_noise3_internal` | wrap `= 256` (mask `255`, i.e. the same as `0`) | [x] | `phase_b_noise3::c7_internal_wrap_256` |
| C8 | `stb_perlin_noise3_internal` | powers of two `> 256` (`512`, `1024`, `2^30`) — mask still `255` | [x] | `phase_b_noise3::c8_internal_wrap_pow2_above_256` |
| C9 | `stb_perlin_noise3_internal` | non-powers of two (`3,5,7,100,255`, random) — undocumented but accepted | [x] | `phase_b_noise3::c9_internal_non_pow2_wraps` |
| C10 | `stb_perlin_noise3_internal` | negative wraps (`-1,-5,-256`, random negative) | [x] | `phase_b_noise3::c10_internal_negative_wraps` |
| C11 | `stb_perlin_noise3_internal` | wraps `INT_MIN`/`INT_MAX` (signed overflow in `x_wrap-1`) | [x] | `phase_b_noise3::c11_internal_wrap_int_extremes` |
| C12 | `stb_perlin_noise3_internal` | coords just below/above integers (`k±1ulp`), `±0`, subnormals | [x] | `phase_b_noise3::c12_internal_near_integer_coords` |
| C13 | `stb_perlin_noise3_internal` | large finite coords (`2^20`, `2^23`, `2^30`, `±(2^31-1)`) | [x] | `phase_b_noise3::c13_internal_large_coords` |
| C14 | `stb_perlin_noise3_internal` | fully random finite `f32` bit patterns per axis × random wraps × random seed | [x] | `phase_b_noise3::c14_internal_random_everything` |
| C15 | `stb_perlin_noise3_internal` | exhaustive `seed = 0..=255` with several fixed coord/wrap sets | [x] | `phase_b_noise3::c15_internal_all_seeds` |
| C16 | `stb_perlin_noise3` | the C1/C4/C9/C10/C13 wrap shapes through the `seed = 0` wrapper | [x] | `phase_b_noise3::c16_noise3_wrapper` |
| C17 | `stb_perlin_noise3_seed` | random full-range `int` seed (truncation to `unsigned char`) × random wrap shapes | [x] | `phase_b_noise3::c17_noise3_seed_full_int` |
| C18 | `stb_perlin_noise3_seed` | seeds `-1`, `0`, `255`, `256`, `INT_MIN`, `INT_MAX` × pow2/non-pow2 wraps | [x] | `phase_b_noise3::c18_noise3_seed_boundaries` |
| C19 | `stb_perlin_ridge_noise3` | canonical `lacunarity=2, gain=0.5, offset=1`, `octaves = 1..8`, random coords | [x] | `phase_b_fractal::c19_ridge_canonical` |
| C20 | `stb_perlin_ridge_noise3` | `offset ∈ {0, -1, 1, 8, 1e30}` × `octaves ∈ {1,2,6}` | [x] | `phase_b_fractal::c20_ridge_offset_shapes` |
| C21 | `stb_perlin_ridge_noise3` | random `lacunarity`/`gain` in `[-4,4]` (incl. `0`, negatives), `octaves = 1..8` | [x] | `phase_b_fractal::c21_ridge_random_lac_gain` |
| C22 | `stb_perlin_ridge_noise3` | `octaves ∈ {32, 64, 300}` so `frequency`/`amplitude` reach `inf`/`0` | [x] | `phase_b_fractal::c22_ridge_extreme_octaves` |
| C23 | `stb_perlin_ridge_noise3` | `lacunarity`/`gain` `= 0` / `= 1` / `= huge`, coords large | [x] | `phase_b_fractal::c23_ridge_degenerate_lac_gain` |
| C24 | `stb_perlin_fbm_noise3` | canonical `2/0.5`, `octaves = 1..8`, random coords | [x] | `phase_b_fractal::c24_fbm_canonical` |
| C25 | `stb_perlin_fbm_noise3` | random `lacunarity`/`gain`, `octaves = 1..8`, random coords incl. negatives | [x] | `phase_b_fractal::c25_fbm_random_lac_gain` |
| C26 | `stb_perlin_fbm_noise3` | `octaves ∈ {32,64,300}`, `lacunarity ∈ {1e30, -2}` (overflow to `inf`) | [x] | `phase_b_fractal::c26_fbm_extreme` |
| C27 | `stb_perlin_turbulence_noise3` | canonical `2/0.5`, `octaves = 1..8`, random coords | [x] | `phase_b_fractal::c27_turbulence_canonical` |
| C28 | `stb_perlin_turbulence_noise3` | random `lacunarity`/`gain`, `octaves = 1..8` | [x] | `phase_b_fractal::c28_turbulence_random_lac_gain` |
| C29 | `stb_perlin_turbulence_noise3` | `octaves ∈ {32,64,300}` + `inf`/`NaN` `lacunarity`/`gain` | [x] | `phase_b_fractal::c29_turbulence_extreme` |
| C30 | `stb_perlin_noise3_wrap_nonpow2` | wraps `(0,0,0)` → `256`, random seed/coords | [x] | `phase_b_nonpow2::c30_nonpow2_zero_wraps` |
| C31 | `stb_perlin_noise3_wrap_nonpow2` | all wraps equal, random in `1..=256`, random seed, coords in `[-300,300]` | [x] | `phase_b_nonpow2::c31_nonpow2_uniform_wraps` |
| C32 | `stb_perlin_noise3_wrap_nonpow2` | per-axis different wraps in `1..=256` × large coords (`±2^20`) | [x] | `phase_b_nonpow2::c32_nonpow2_mixed_wraps` |
| C33 | `stb_perlin_noise3_wrap_nonpow2` | prime / non-power-of-two wraps (`3,5,7,13,97,251`) — the function's raison d'être | [x] | `phase_b_nonpow2::c33_nonpow2_prime_wraps` |
| C34 | `stb_perlin_noise3_wrap_nonpow2` | wrap `= 1` on one/two/three axes (every index `0`) | [x] | `phase_b_nonpow2::c34_nonpow2_wrap_one` |
| C35 | `stb_perlin_noise3_wrap_nonpow2` | wraps `> 256` restricted to inputs whose table indices stay inside the modelled `0..1024` window | [x] | `phase_b_nonpow2::c35_nonpow2_wrap_above_256_in_window` |
| C36 | `stb_perlin_noise3_wrap_nonpow2` | negative wraps with `px >= 0` (`px % -w >= 0`, still in-bounds) | [x] | `phase_b_nonpow2::c36_nonpow2_negative_wrap_positive_px` |
| C37 | `stb_perlin_noise3_wrap_nonpow2` | exhaustive `seed = 0..=255` with fixed wraps/coords | [x] | `phase_b_nonpow2::c37_nonpow2_all_seeds` |
| C38 | `inner` | `which = 0..=5`, each with the arguments that case actually forwards, randomised | [x] | `phase_b_inner::c38_inner_each_case` |
| C39 | `inner` | all twelve arguments randomised simultaneously (incl. unused ones) with `which` in `0..=5` | [x] | `phase_b_inner::c39_inner_random_all_args` |
| C40 | `inner` | `which = 0..=5` × special floats (`±0`, `±inf`, `NaN`) × wrap/seed extremes | [x] | `phase_b_inner::c40_inner_special_floats` |
| C41 | `main` (driver exe / Rust bin) | twelve well-formed decimal tokens, `which = 0..=5`, randomised values | [x] | `driver_cli::c41_driver_random_valid_inputs` |
| C42 | `main` | whitespace shapes: leading/trailing, tabs, `\r\n`, blank lines, no trailing newline | [x] | `driver_cli::c42_driver_whitespace_shapes` |
| C43 | `main` | number spellings: `+1`, `.5`, `5.`, `1e3`, `1E-3`, `0x1p4`, `inf`, `infinity`, `nan`, `-nan` | [x] | `driver_cli::c43_driver_number_spellings` |
| C44 | `main` | magnitudes that stress `%.9g`: `1e38`, `1e-45`, subnormals, `1e400`, values switching `%e`/`%f` style | [x] | `driver_cli::c44_driver_extreme_magnitudes` |
| C45 | `main` | only `0..11` of the twelve tokens present (each prefix length) | [x] | `driver_cli::c45_driver_short_input` |
| C46 | `main` | more than twelve tokens / trailing junk after the twelfth | [x] | `driver_cli::c46_driver_extra_tokens` |
| C47 | `main` | randomised complete inputs, comparing stdout **and** exit status of the C executable vs the Rust binary | [x] | `driver_cli::c47_driver_exit_status_and_stdout` |
| C48 | `main` **exported from the two shared objects** (called through `dlopen`, not the executables) | randomised complete inputs, stdin/stdout piped | [x] | `driver_cli::c48_so_main_export` |
| C49 | `main` | corpus of 230 float spellings (hand written + randomly generated hex and decimal ones, 17+ digit mantissas) placed in each of the six float slots | [x] | `driver_cli::float_token_corpus` |
| C50 | `main` | randomised "token soup": 0..15 tokens of random spellings and separators | [x] | `driver_cli::scanf_random_token_soup` |
| C51 | `scanf` conversions (`%d`, `%f`) compared **directly against glibc** | 40 000 randomised tokens incl. hex floats, exponent markers without digits, saturating integers; value **and** `%n` consumption compared | [x] | `cscan::glibc_tests::*` |
| C52 | `printf("%.9g")` compared **directly against glibc** | every binary exponent, 200 000 random bit patterns, 200 000 small values, precisions 0/1/2/3/6/9/12/17 | [x] | `cfmt::glibc_tests::*` |
| C53 | the whole pipeline through the two entry-point kinds | C executable vs Rust binary *and* C `.so` `main` vs Rust `.so` `main` for the same inputs | [x] | `driver_cli::c41..c50` |

All 53 rows pass across their randomised inputs (see `VERIFICATION.md` for the
run log).

## Feature / build configurations

`Cargo.toml` declares **no `[features]`**, and `c_src/CMakeLists.txt` declares
no options, so the cross-product of build configurations is a single cell:

| # | configuration | command | verified |
|---|---------------|---------|-----|
| F1 | default (no features) | `cargo test --offline` | [x] |
| F2 | `--no-default-features` (identical, no feature table exists) | `cargo test --offline --no-default-features` | [x] |
| F3 | `--all-features` (identical) | `cargo test --offline --all-features` | [x] |
| F4 | release profile (`overflow-checks = false`, optimisations on) | `cargo test --offline --release` | [x] |

`scripts/check_features.sh [check|test]` enumerates the `[features]` table
automatically and runs all of them.
