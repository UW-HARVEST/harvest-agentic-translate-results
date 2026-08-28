# CONFIGS.md — configuration / valid-input surface table

Mechanically derived from the axes `c_src/src/lib.c` actually branches on or
indexes with. There are **no** runtime options, no flags, no modes, no global
state, no `#ifdef` in this library (`grep -c 'ifdef\|ifndef\|#if' c_src/src/lib.c
c_src/include/lib.h` → 0), so the configuration surface is entirely made of
*input shapes* and *value ranges*.

## Axes the C code distinguishes

| axis | values the C treats differently | where |
|------|--------------------------------|-------|
| A. entry point | `synth_pair` (only exported fn) · `mp3d_scale_pcm` (static, reachable only through `synth_pair`'s two call sites — lane 0 and lane 1) | `lib.c:23`, `lib.c:33` |
| B. `nch` | `1` (mono/interleave-1) · `2` (stereo, the real mp3 use) · `>2` · `0` · negative · overflowing | `pcm[16 * nch]` |
| C. `z` tap layout | lane 0 reads `z[k*64]`, k∈0..14 · lane 1 reads `z[2 + k*64]` for **even** k∈{0,2,…,14} (8 taps) — 23 distinct floats out of a 899-float extent | `lib.c:15-22`, `lib.c:24-32` |
| D. per-lane sign pattern | lane 0 mixes **differences** (`z14-z0`, `z12-z2`, `z10-z4`, `z8-z6`) and **sums** (`z1+z13`, `z3+z11`, `z5+z9`) plus a bare `z7` term; lane 1 is 8 plain products with 3 negative weights (`-9975`, `-45`, `-5`) | `lib.c:15-32` |
| E. accumulator magnitude | inside `(-32767.5, 32766.5)` (conversion path) · `>= 32766.5` (high clamp) · `<= -32767.5` (low clamp) | `lib.c:4-7` |
| F. accumulator sign | `>= 0` (no bias correction) · `< 0` (`s -= 1`) | `lib.c:9` |
| G. float classes present in `z` | normals · zeros (`+0`/`-0`) · subnormals · huge finite · `±inf` · NaN | IEEE-754 f32 behaviour of `subss`/`addss`/`mulss` |
| H. `pcm` buffer geometry | write targets `pcm[0]` and `pcm[16*nch]` — distinct, identical (`nch==0`), or negative-side | `lib.c:23,33` |

The library has **no Cargo features** (`grep -A5 '\[features\]' Cargo.toml` → no
`[features]` section), so the only build configurations are the default one and
the `--no-default-features` one, which are identical. Both are exercised (see
`run_all_feature_combos.sh`).

## Configuration table (cross-product, pruned to what the C distinguishes)

Every row is driven with **many randomized inputs** from a fixed-seed
xorshift/SplitMix PRNG (see `tests/common/mod.rs`), not a single hand-picked
value, and asserts the **whole `pcm` buffer** matches byte-for-byte between the
C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | iters | test | ✔ |
|---|----------------|--------------------------------------------|-------|------|---|
| C1  | `synth_pair` | `nch=1`, `z` = small uniform normals in `[-1,1)`, conversion path, both signs | 20000 | `cfg_c1_small_uniform_nch1` | [x] |
| C2  | `synth_pair` | `nch=2` (stereo), `z` = small uniform normals in `[-1,1)` | 20000 | `cfg_c2_small_uniform_nch2` | [x] |
| C3  | `synth_pair` | `nch=3..8`, `z` = small uniform normals (checks the `16*nch` stride for many strides) | 20000 | `cfg_c3_varied_nch_small_uniform` | [x] |
| C4  | `synth_pair` | `nch=2`, `z` scaled so lane sums straddle the clamp thresholds (mixed clamp / no-clamp, `±` amplitude ≈ `32767/75038`..`32767/1000`) | 20000 | `cfg_c4_near_clamp_thresholds` | [x] |
| C5  | `synth_pair` | `nch=2`, `z` scaled so **both** lanes almost always clamp high or low (`amp` ≈ 1e2..1e6) | 20000 | `cfg_c5_mostly_clamping` | [x] |
| C6  | `synth_pair` | `nch=1`, `z` = **all zeros** except one randomly chosen *read* tap set to a random value (isolates each of the 23 taps and its weight/sign) | 23×2000 | `cfg_c6_single_tap_isolation` | [x] |
| C7  | `synth_pair` | `nch=2`, `z` = all zeros except one randomly chosen **non-read** index (proves the untouched indices really are untouched in both) | 4000 | `cfg_c7_unread_indices_are_ignored` | [x] |
| C8  | `synth_pair` | `nch=2`, `z` = exact bit patterns drawn from a boundary pool: `±0.0`, `±f32::MIN_POSITIVE`, `±1e-40` (subnormal), `±1.0`, `±0.5`, `±32766.5`, `±32767.5`, `±f32::MAX`, `±f32::EPSILON` | 20000 | `cfg_c8_boundary_value_pool` | [x] |
| C9  | `synth_pair` | `nch=2`, `z` = **fully random 32-bit patterns** reinterpreted as f32 (includes NaNs of every payload, infinities, subnormals, huge values) | 40000 | `cfg_c9_random_bit_patterns` | [x] |
| C10 | `synth_pair` | `nch=2`, paired taps set to *equal* values so the difference terms cancel to `±0.0` (`z14==z0`, `z12==z2`, `z10==z4`, `z8==z6`) | 20000 | `cfg_c10_cancelling_difference_taps` | [x] |
| C11 | `synth_pair` | `nch=2`, paired taps set to *opposite* values so the sum terms cancel (`z1==-z13`, `z3==-z11`, `z5==-z9`) | 20000 | `cfg_c11_cancelling_sum_taps` | [x] |
| C12 | `synth_pair` | `nch=1`, magnitudes spread over the full binade range (random exponent in `2^-45..2^45`, random sign) — catches rounding/ordering differences in the accumulation chain | 40000 | `cfg_c12_wide_exponent_range` | [x] |
| C13 | `synth_pair` | `nch=2`, only lane-0 taps (`z[k*64]`) randomized, lane-1 taps (`z[2+k*64]`) zeroed | 20000 | `cfg_c13_lane0_only` | [x] |
| C14 | `synth_pair` | `nch=2`, only lane-1 taps randomized, lane-0 taps zeroed | 20000 | `cfg_c14_lane1_only` | [x] |
| C15 | `synth_pair` | repeated calls into the **same** `pcm` buffer at different `nch` (statelessness / no hidden global state, interleaved C-then-Rust and Rust-then-C orders) | 8000 | `cfg_c15_repeated_calls_stateless` | [x] |
| C16 | `synth_pair` | `z` pointer at a **non-zero offset** inside a larger allocation (verifies the `z += 2` pointer bump and offset arithmetic, not just base-of-buffer) | 20000 | `cfg_c16_offset_z_pointer` | [x] |
| C17 | `synth_pair` | `pcm` pointer at a non-zero offset with a full-buffer byte-compare afterwards (verifies only 2 elements are written) | 20000 | `cfg_c17_offset_pcm_only_two_writes` | [x] |
| C18 | `synth_pair` | `nch=2`, minimum-extent `z` of exactly `899` floats (no slack) | 20000 | `cfg_c18_exact_extent_z` | [x] |
| C19 | `synth_pair` | `nch` from `{1,2}` × `z` drawn from a mixture: 70 % normals, 15 % boundary pool, 15 % raw bit patterns (interaction of all value classes in one buffer) | 40000 | `cfg_c19_mixed_class_buffers` | [x] |
| C20 | `mp3d_scale_pcm` (through lane 0, `z` = one live tap) | sweep the accumulator through **every** distinct region of E1–E14 by solving `z[7*64] = target/75038` for targets across `[-40000, 40000]` plus exact clamp boundaries | 20000 | `cfg_c20_scale_pcm_full_sweep` | [x] |

**No `CONFIGS.md` row is unchecked.**

## Exhaustive supplements (beyond the randomized rows)

`tests/exhaustive.rs` goes past property-style sampling for the axes where the
whole input domain is enumerable. All of these compare the C `.so` against the
Rust `.so` through `dlopen`/`dlsym`.

| # | entry point | coverage | comparisons | test | ✔ |
|---|-------------|----------|-------------|------|---|
| X1 | `synth_pair` | **all 2^32** `f32` bit patterns through lane 0's last/dominant tap `z[448]` (weight `75038`) — every normal, subnormal, `±0`, `±inf` and NaN payload | 4 294 967 296 | `exhaustive_all_f32_through_lane0_dominant_tap` | [x] |
| X2 | `synth_pair` | **all 2^32** `f32` bit patterns through lane 1's dominant tap `z[514]` (weight `64019`) | 4 294 967 296 | `exhaustive_all_f32_through_lane1_dominant_tap` | [x] |
| X3 | `synth_pair` | strided full-domain sweep through **every one of the 23 read taps** (so every one of the 16 distinct weights and both lanes), plus odd-stride sweeps to hit other mantissa residue classes | ~30 M | `strided_all_f32_through_every_tap` | [x] |
| X4 | `synth_pair` | each of the 4 **difference** pairs, one operand swept over the full domain while the other is pinned to 13 special values (`±0`, `±1`, subnormal, `±MAX`, `±inf`, NaN, clamp-threshold) — covers cancellation, `inf - inf`, sign-of-zero | ~6.8 M | `strided_difference_pairs_against_pinned_operands` | [x] |
| X5 | `synth_pair` | each of the 3 **sum** pairs, same construction | ~2.2 M | `strided_sum_pairs_against_pinned_operands` | [x] |
| X6 | `synth_pair` | validates the `c_scale_pcm_reference` / `model_lane0` cross-check helpers against the **real C `.so`** over all 2^32 lane-0 inputs | 4 294 967 296 | `exhaustive_reference_model_matches_the_c_so` | [x] |

Under `--release` these run at `step == 1` (truly exhaustive); unoptimized runs
use a stride so the suite stays fast (`common::optimized()`).

## Environment-surface supplement

| # | entry point | configuration | test | ✔ |
|---|-------------|---------------|------|---|
| C21 | `synth_pair` | all four IEEE rounding modes installed by the caller via `fesetround` (`FE_TONEAREST`, `FE_DOWNWARD`, `FE_UPWARD`, `FE_TOWARDZERO`) x 2000 randomized `z` buffers x `nch in {1,2}`. Neither library touches the FP environment, so both must honour MXCSR identically. The test asserts it is **non-vacuous**: all 3 non-default modes must actually change some output. | `differential_under_every_rounding_mode` | [x] |

`tests/rounding_mode.rs` is a separate test binary because the rounding mode is
process-global.
