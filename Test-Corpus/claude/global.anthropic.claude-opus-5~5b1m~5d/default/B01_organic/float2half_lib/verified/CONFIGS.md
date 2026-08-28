# CONFIGS.md — Configuration-surface table (Phase A / Phase B)

## How this table was derived

The library has **no runtime options**: `include/lib.h` declares exactly one
entry point, `uint16_t float2half(float)`, with no flags, no mode enum, no
context/handle struct, no globals that a caller can set, and no
`#ifdef`/`#if` conditional compilation (mechanically confirmed: 0 occurrences).
There is therefore exactly one entry point and it is simultaneously the
lowest-level and the only public API — there are no convenience wrappers to
mistake for the real surface.

Consequently the entire configuration surface is the **shape of the single
`float` input**. The C body is branchless; all of its behavioural variation
comes from the two 512-entry lookup tables:

```c
j = (n >> 23) & 0x1ff;                                  // 9 bits = sign | exponent
return (uint16_t)((uint32_t)m__base[j]
                  + ((n & 0x007fffff) >> m__shift[j])); // 23-bit mantissa
```

So the axes the code actually distinguishes are exactly:

* **Axis 1 — `j = (sign << 8) | exponent`** (512 values). This is the real
  "switch": each `j` selects a `(base, shift)` pair. Grouping the tables into
  maximal runs of a constant `(base, shift)` pair yields **86 distinct runs**,
  which collapse into 7 behavioural regions per sign (14 total, listed below).
  The negative half was mechanically verified to be an exact sign-mirror of the
  positive half (`base[j+256] == base[j] | 0x8000`, `shift[j+256] == shift[j]`
  for all `j < 256`), so the regions come in symmetric pairs.
* **Axis 2 — the 23-bit mantissa** `n & 0x007fffff`. It matters only through
  `>> shift`, so the interesting shapes are `0`, `1`, values straddling the
  region's own `1 << shift` granularity boundary, and all-ones.

Region map (computed from the table data, not assumed):

| region | `j` (positive half) | exponent | `base` | `shift` | behaviour |
|--------|--------------------|----------|--------|---------|-----------|
| R1 | 0 | 0 | `0x0000` | 24 | float zero / float subnormal -> `+0` |
| R2 | 1..102 | 1..102 | `0x0000` | 24 | underflow, flush to `+0` |
| R3 | 103..112 | 103..112 | `0x0001`..`0x0200` | 23..14 (10 distinct) | -> half **subnormal**, granularity varies per exponent |
| R4 | 113 | 113 | `0x0400` | 13 | smallest half **normal** |
| R5 | 114..142 | 114..142 | `0x0800`..`0x7800` | 13 | half normal range |
| R6 | 143..254 | 143..254 | `0x7C00` | 24 | overflow -> `+Inf`, mantissa discarded |
| R7 | 255 | 255 | `0x7C00` | **13** | `Inf`/`NaN`: mantissa **is** kept -> `0x7C00 + (mant >> 13)` |

R7 is the subtlest row: unlike R6 it uses shift 13, so NaN payloads propagate
and a NaN with a small payload degenerates to `Inf`. R3 is the second subtlest:
it is the only region where `shift` varies from one `j` to the next.

Every row below is exercised with **many randomized inputs** (SplitMix64,
fixed seed `0x2024_0611_C0FFEE`) in addition to the named boundary mantissas,
by calling **both** `.so` files through `libloading` and comparing the returned
`u16` byte-for-byte.

## Configuration-surface table

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `float2half` | R1, sign `+`: exponent 0, mantissa `0` (i.e. `+0.0`) and randomized non-zero mantissas (float subnormals) | `cfg_row01_r1_pos_zero_and_subnormal` | [x] |
| 2 | `float2half` | R1, sign `-`: exponent 0, mantissa `0` (`-0.0`) and randomized non-zero mantissas | `cfg_row02_r1_neg_zero_and_subnormal` | [x] |
| 3 | `float2half` | R2, sign `+`: exponents 1..102 (all 102, incl. boundaries 1 and 102) x mantissa shapes {0, 1, `0x400000`, `0x7FFFFF`} + randomized | `cfg_row03_r2_pos_underflow` | [x] |
| 4 | `float2half` | R2, sign `-`: exponents 1..102 x same mantissa shapes + randomized (must yield `0x8000`, i.e. `-0`) | `cfg_row04_r2_neg_underflow` | [x] |
| 5 | `float2half` | R3, sign `+`: each of the 10 exponents 103..112 individually (each has its OWN shift 23..14) x mantissa shapes {0, 1, `(1<<shift)-1`, `1<<shift`, `(1<<shift)+1`, `0x7FFFFF`} + randomized — the granularity-boundary row | `cfg_row05_r3_pos_half_subnormal_per_exponent` | [x] |
| 6 | `float2half` | R3, sign `-`: same 10 exponents and shapes, negative | `cfg_row06_r3_neg_half_subnormal_per_exponent` | [x] |
| 7 | `float2half` | R4, sign `+`: exponent 113 exactly (smallest half normal) x {0, 1, `0x1FFF`, `0x2000`, `0x2001`, `0x7FFFFF`} + randomized | `cfg_row07_r4_pos_smallest_normal` | [x] |
| 8 | `float2half` | R4, sign `-`: exponent 113 exactly, negative | `cfg_row08_r4_neg_smallest_normal` | [x] |
| 9 | `float2half` | R5, sign `+`: all exponents 114..142 x mantissa shapes {0, 1, `0x1FFF`, `0x2000`, `0x3FFF`, `0x7FFFFF`} + randomized | `cfg_row09_r5_pos_normal_range` | [x] |
| 10 | `float2half` | R5, sign `-`: all exponents 114..142, negative | `cfg_row10_r5_neg_normal_range` | [x] |
| 11 | `float2half` | R6, sign `+`: all exponents 143..254 x mantissa shapes + randomized (mantissa must be discarded -> always `0x7C00`) | `cfg_row11_r6_pos_overflow_to_inf` | [x] |
| 12 | `float2half` | R6, sign `-`: all exponents 143..254, negative (-> always `0xFC00`) | `cfg_row12_r6_neg_overflow_to_inf` | [x] |
| 13 | `float2half` | R7, sign `+`: exponent 255 x {mantissa 0 = `+Inf`, 1, `0x1FFF`, `0x2000`, `0x400000` (qNaN), `0x200000` (sNaN), `0x7FFFFF`} + randomized NaN payloads — shift is 13 here, so payload propagates | `cfg_row13_r7_pos_inf_and_nan_payloads` | [x] |
| 14 | `float2half` | R7, sign `-`: exponent 255, negative (`-Inf` and negative NaN payloads) | `cfg_row14_r7_neg_inf_and_nan_payloads` | [x] |
| 15 | `float2half` | Region-transition boundaries: for each of the 86 maximal constant-`(base,shift)` runs in the tables, the first and last `j` of the run, each with mantissa `0` and `0x7FFFFF` — catches an off-by-one in any table row | `cfg_row15_all_86_run_boundaries` | [x] |
| 16 | `float2half` | Full index sweep, mantissa `0`: all 512 `j` values (256 exponents x 2 signs) — direct per-entry parity of `m__base` | `cfg_row16_all_512_indices_mantissa_zero` | [x] |
| 17 | `float2half` | Full index sweep, mantissa `0x7FFFFF`: all 512 `j` values — direct per-entry parity of `m__shift` at its maximal input | `cfg_row17_all_512_indices_mantissa_max` | [x] |
| 18 | `float2half` | Full index sweep x randomized mantissas: all 512 `j` values x 64 random mantissas each (32768 calls) | `cfg_row18_all_512_indices_random_mantissas` | [x] |
| 19 | `float2half` | Power-of-two mantissa sweep: all 512 `j` x each of the 24 mantissas `1 << k` for `k` in 0..23, plus `(1<<k)-1` — probes each shift's exact cut point | `cfg_row19_all_512_indices_power_of_two_mantissas` | [x] |
| 20 | `float2half` | Uniformly random full 32-bit patterns (no structure imposed), 2,000,000 samples, fixed seed — value-dependent bug net | `cfg_row20_uniform_random_bit_patterns` | [x] |
| 21 | `float2half` | "Real" `f32` values a normal consumer would pass, generated as randomized decimal-ish magnitudes across the whole exponent span (incl. `1.0`, `-1.0`, `65504.0` = half max, `65520.0`, `6.1e-5`, `6e-8`, `1e-45`, `1e38`, `3.4e38`) | `cfg_row21_realistic_float_values` | [x] |
| 22 | `float2half` | Exhaustive: **all 2^32 bit patterns**, in ascending order, comparing C and Rust on every one. This row subsumes rows 1–21; it is the complete input space of the function. | `exhaustive_all_2_pow_32_bit_patterns` (`phase_d_exhaustive.rs`) | [x] |

**Rows: 22. Unchecked rows: 0.**

## Feature combinations

No `[features]` in `Cargo.toml` and no `cfg(feature = ...)` in `src/`, so the
only combination is the default (empty) one; `--no-default-features` selects
the identical build. `run_all.sh` runs the whole suite under both invocations
anyway so the claim is tested rather than assumed.
