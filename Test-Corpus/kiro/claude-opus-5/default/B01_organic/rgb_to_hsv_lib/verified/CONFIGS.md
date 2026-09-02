# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/include/lib.h` and every branch in
`c_src/src/lib.c`.

## Axes the C actually branches on

**Runtime options / modes / flags:** none.
`lib.h` exposes exactly one entry point and it takes no flag, mode, enum, or
context argument:

```c
void rgb_to_hsv(float *dest, const float *src);
```

There are no `#ifdef`s in `lib.c`, no global configuration state, no
initialisation function, and no `[features]` section in `translation/Cargo.toml`
(verified). So the configuration cross-product is driven **entirely** by input
shape, and the only feature combination is the default one (see Phase D).

**Entry points:** `rgb_to_hsv` is simultaneously the lowest-level and the only
public entry point — there is no convenience wrapper above it and no internal
helper below it (the min/max reductions are open-coded ternaries, not calls). It
is therefore always exercised directly through its `.so` export, never via a
wrapper.

**Input-shape axes the code special-cases** (each maps to a real branch or a
real value-dependent path):

- A1 — which channel holds `max`: `r` (line 26), `g` (line 28), or the `else`
  fallthrough (line 30). Three-way, and ties change which one wins.
- A2 — `delta == 0` vs `delta != 0` (line 19, first disjunct).
- A3 — `max == 0` vs `max != 0` (line 19, second disjunct) — independently
  reachable from A2 because `max` can be `0` while `delta > 0`.
- A4 — sign of `h` before line 33: `h < 0` (wrap by `+360`) vs `h >= 0`.
- A5 — magnitude class of the channels: normal, subnormal, `0`, huge
  (`FLT_MAX`-scale, where `delta` can overflow), and mixed-exponent pairs where
  `max - min` loses precision.
- A6 — sign class: all non-negative (the intended 0..1 domain), all negative,
  mixed sign, signed zeros.
- A7 — non-finite: `NaN` (in each of the 3 positions, and in combinations),
  `+inf`, `-inf`.
- A8 — pointer/buffer shape: disjoint `dest`/`src`, `dest == src` (in-place),
  and partially overlapping (`src ± 1`).
- A9 — value distribution within the canonical `[0,1]` domain, including the
  exact hue boundaries (0/60/120/180/240/300/360°) and near-tie values where
  `r`, `g`, `b` differ by 1 ULP.

## Configuration table

One row per combination the C treats differently. Every row is driven with
**many randomized inputs** (fixed seed `0x5EED_1234_ABCD_0001`, a SplitMix64
generator) plus the row's pinned edge values, and asserts the 3 output `f32`s
are **bitwise identical** between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| C1 | `rgb_to_hsv` | canonical domain: uniform random `r,g,b ∈ [0,1]`, disjoint buffers (the ordinary consumer case; covers A1 all three, A2/A3 false, A4 both) | `cfg_c1_unit_domain_random` | [x] |
| C2 | `rgb_to_hsv` | `r` strict max, `g > b` ⇒ `h >= 0`, no wrap (A1=r, A4=false) | `cfg_c2_r_max_no_wrap` | [x] |
| C3 | `rgb_to_hsv` | `r` strict max, `g < b` ⇒ `h < 0` ⇒ `+360` wrap (A1=r, A4=true) | `cfg_c3_r_max_wrap` | [x] |
| C4 | `rgb_to_hsv` | `g` strict max (A1=g, `h = 2 + (b-r)/delta`) | `cfg_c4_g_max` | [x] |
| C5 | `rgb_to_hsv` | `b` strict max ⇒ `else` branch (A1=else, `h = 4 + (r-g)/delta`) | `cfg_c5_b_max` | [x] |
| C6 | `rgb_to_hsv` | achromatic `r == g == b`, random magnitude ⇒ A2 true, early return | `cfg_c6_achromatic_random` | [x] |
| C7 | `rgb_to_hsv` | near-achromatic: channels differing by 1–4 ULP, so `delta` is subnormal-to-tiny and `s`/`h` are catastrophically ill-conditioned (A2 false but barely, A5 subnormal) | `cfg_c7_one_ulp_apart` | [x] |
| C8 | `rgb_to_hsv` | exact two-channel ties: `r==g>b`, `g==b>r`, `r==b>g` — exercises the `if/else if` priority (A1 ties) | `cfg_c8_two_channel_ties` | [x] |
| C9 | `rgb_to_hsv` | exact hue boundaries: the 6 primary/secondary colours and the 0/60/120/180/240/300 degree points, at several `v` and `s` levels | `cfg_c9_hue_boundaries` | [x] |
| C10 | `rgb_to_hsv` | all channels negative (A6 all-negative): `max`, `v`, and `s` go negative; `s = delta/max` is negative | `cfg_c10_all_negative` | [x] |
| C11 | `rgb_to_hsv` | mixed-sign channels (A6 mixed) — includes the `max == 0, delta > 0` early return (A3) reached from random data | `cfg_c11_mixed_sign` | [x] |
| C12 | `rgb_to_hsv` | signed zeros in every one of the 8 `{±0.0}³` combinations (A6 signed zero; tie-breaking of `<`/`>` on `-0.0 == +0.0`) | `cfg_c12_signed_zero_grid` | [x] |
| C13 | `rgb_to_hsv` | subnormal channels (A5 subnormal): random subnormal `r,g,b`, plus `MIN_POSITIVE`, smallest subnormal `1e-45`, and subnormal/normal mixes | `cfg_c13_subnormals` | [x] |
| C14 | `rgb_to_hsv` | huge magnitudes (A5 huge): `±FLT_MAX`-scale, where `max - min` overflows to `inf` and `delta/max` can overflow | `cfg_c14_huge_magnitudes` | [x] |
| C15 | `rgb_to_hsv` | wide dynamic range (A5 mixed exponent): one channel near `FLT_MAX`, another near `FLT_MIN`, so `delta` and the quotients lose all precision | `cfg_c15_wide_exponent_spread` | [x] |
| C16 | `rgb_to_hsv` | fully unconstrained bit-pattern fuzz (A5+A6+A7 jointly): all 3 channels are uniformly random 32-bit patterns reinterpreted as `f32`, so `NaN`s, `inf`s, subnormals and wild exponents all occur naturally | `cfg_c16_random_bit_patterns` | [x] |
| C17 | `rgb_to_hsv` | non-finite grid (A7): the cross-product of `{NaN, +inf, -inf, 0.0, -0.0, 1.0, -1.0, FLT_MAX, -FLT_MAX, FLT_MIN}` over all 3 channels (1000 combinations, exhaustive) | `cfg_c17_nonfinite_grid` | [x] |
| C18 | `rgb_to_hsv` | in-place, `dest == src` (A8 exact alias), over random `[0,1]` and random bit patterns | `cfg_c18_inplace_alias` | [x] |
| C19 | `rgb_to_hsv` | partial overlap `dest = src + 1` and `dest = src - 1` (A8 partial alias), verifying the whole 5-element window afterwards | `cfg_c19_partial_overlap` | [x] |
| C20 | `rgb_to_hsv` | 8-bit-quantised inputs `k/255.0` for `k ∈ 0..=255` (the real-world image-pixel shape), random triples plus the full grey ramp | `cfg_c20_u8_quantised` | [x] |
| C21 | `rgb_to_hsv` | bulk sequential invocation over a large buffer (many pixels, one call per pixel) to confirm no cross-call state and identical results in sequence | `cfg_c21_bulk_sequence` | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` section**, so the default
configuration is the only feature combination. `run_all.sh` still enumerates the
powerset mechanically (so a future feature is picked up automatically) and runs
the whole suite under `--no-default-features` as well, against **both** the
debug and the release cdylib — the release profile sets `panic = "abort"` and
the debug profile enables `debug_assertions`, and those really do exercise
different code paths (see the E23/E24 divergence recorded in `ERRORS.md`).

## Gate status

- [x] Every row above passes across its randomized inputs, comparing C `.so`
      output to Rust `.so` output bit-for-bit.
- [x] Verified under default features and `--no-default-features`, against the
      debug cdylib and the release cdylib (4 combinations, all green).
