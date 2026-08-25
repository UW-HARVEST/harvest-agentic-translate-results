# Configuration Surface

## Build-Time Configurations

`Cargo.toml` has no `[features]` table and `c_src/CMakeLists.txt` has no
options or conditional source selection. There is exactly one valid feature
combination:

| # | Cargo feature set | C configuration | status |
|---|-------------------|-----------------|-----|
| 1 | Empty set (`--no-default-features`) | Default CMake configuration | [x] |

## Runtime Configurations

Rows are derived from the public declarations in `stb_perlin.h`, all global
definitions found by `nm -D`, the `switch` in `inner`, the loop conditions in
the fractal functions, the flooring and wrap branches, and the `scanf`-driven
`main` entry point.

| # | entry point(s) | configuration (options set + input shape) | status |
|---|----------------|--------------------------------------------|-----|
| 1 | `stb_perlin_noise3_internal` | Integer coordinates; wraps all zero (effective 256); seed 0 | [x] |
| 2 | `stb_perlin_noise3_internal` | Positive fractional coordinates; wraps all zero; seed in 1..255 | [x] |
| 3 | `stb_perlin_noise3_internal` | Mixed negative-fractional, integer, and positive-fractional coordinates; wraps 1, power-of-two below 256, and 256; seed nonzero | [x] |
| 4 | `stb_perlin_noise3_internal` | Valid power-of-two wraps independently varied on all axes; seed boundary 255 | [x] |
| 5 | `stb_perlin_noise3` | Positive fractional coordinates; no wrapping requested (all wraps zero) | [x] |
| 6 | `stb_perlin_noise3` | Negative fractional coordinates exercise the `a < (int)a` floor branch; power-of-two wraps | [x] |
| 7 | `stb_perlin_noise3` | Integer/boundary coordinates; wrap values 1 and 256 | [x] |
| 8 | `stb_perlin_noise3_seed` | Seed 0; no wraps | [x] |
| 9 | `stb_perlin_noise3_seed` | Seed in 1..255; mixed valid power-of-two wraps and coordinate signs | [x] |
| 10 | `stb_perlin_noise3_seed` | Seed outside 0..255, including negative and above 255; C truncates to the low 8 bits | [x] |
| 11 | `stb_perlin_ridge_noise3` | `octaves <= 0`; loop executes zero times and parameters are otherwise unused | [x] |
| 12 | `stb_perlin_ridge_noise3` | One octave; randomized lacunarity, gain, offset, and coordinate signs | [x] |
| 13 | `stb_perlin_ridge_noise3` | Multiple octaves below 256; randomized lacunarity, gain, and offset | [x] |
| 14 | `stb_perlin_ridge_noise3` | More than 256 octaves; internal octave seed conversion wraps through `unsigned char` | [x] |
| 15 | `stb_perlin_fbm_noise3` | `octaves <= 0`; loop executes zero times | [x] |
| 16 | `stb_perlin_fbm_noise3` | One octave; randomized lacunarity, gain, and coordinate signs | [x] |
| 17 | `stb_perlin_fbm_noise3` | Multiple octaves below 256; frequency and amplitude update each iteration | [x] |
| 18 | `stb_perlin_fbm_noise3` | More than 256 octaves; internal octave seed conversion wraps through `unsigned char` | [x] |
| 19 | `stb_perlin_turbulence_noise3` | `octaves <= 0`; loop executes zero times | [x] |
| 20 | `stb_perlin_turbulence_noise3` | One octave; absolute-value accumulation for both noise signs | [x] |
| 21 | `stb_perlin_turbulence_noise3` | Multiple octaves below 256; frequency and amplitude update each iteration | [x] |
| 22 | `stb_perlin_turbulence_noise3` | More than 256 octaves; internal octave seed conversion wraps through `unsigned char` | [x] |
| 23 | `stb_perlin_noise3_wrap_nonpow2` | Wraps all zero, selecting the explicit 256 fallback; seed 0 | [x] |
| 24 | `stb_perlin_noise3_wrap_nonpow2` | Wrap 1 on one or more axes, making adjacent lattice indices coincide | [x] |
| 25 | `stb_perlin_noise3_wrap_nonpow2` | Positive non-power-of-two wraps in 2..255; mixed coordinate signs and seed 1..255 | [x] |
| 26 | `stb_perlin_noise3_wrap_nonpow2` | Negative fractional coordinates make remainders negative before all three correction branches; wrap boundary 256 | [x] |
| 27 | `inner` | `which = 0`; dispatches unseeded power-of-two noise | [x] |
| 28 | `inner` | `which = 1`; dispatches seeded power-of-two noise | [x] |
| 29 | `inner` | `which = 2`; dispatches ridge noise with all fractal options | [x] |
| 30 | `inner` | `which = 3`; dispatches FBM noise with lacunarity, gain, and octaves | [x] |
| 31 | `inner` | `which = 4`; dispatches turbulence noise with lacunarity, gain, and octaves | [x] |
| 32 | `inner` | `which = 5`; dispatches non-power-of-two wrapped noise and truncates the `int` seed to `unsigned char` | [x] |
| 33 | `main` | Complete 12-field input for each `which` value 0..5, including ordinary single-line whitespace | [x] |
| 34 | `main` | Complete 12-field input split across arbitrary whitespace and multiple lines (`scanf` token behavior) | [x] |
| 35 | `main` | Empty and partially populated input; zero-initialized fields not assigned by `scanf` remain zero | [x] |

All randomized rows use finite coordinates whose conversion to C `int` is
defined. Inputs that would make the C implementation access outside its
fixed 512-entry tables or perform undefined float-to-int conversion are not
valid differential-test configurations.
