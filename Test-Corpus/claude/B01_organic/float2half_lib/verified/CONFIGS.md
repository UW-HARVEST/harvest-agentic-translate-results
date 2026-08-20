# CONFIGS.md — Phase A configuration-surface table

Derived mechanically from `c_src/src/lib.c`: the rows below were *generated* by
parsing the two 512-entry tables out of the C source and grouping the index
space `j = 0..511` into maximal runs of identical `(m__base[j], m__shift[j])`.
Nothing here is guessed.

## Axes the C code actually branches on

`float2half` has **no runtime options, modes or flags** (`c_src/include/lib.h`
declares exactly one function taking one `float`), and **no `#ifdef`s**, so
there is no option axis. All behavioural variation is *data-driven*, through
exactly two axes:

| axis | source line | reachable values | effect |
|------|-------------|------------------|--------|
| **A. table index `j`** | `j = (n >> 23) & 0x1ff;` | `0..=511` — i.e. the f32 **sign bit** (1) concatenated with the f32 **exponent** (8). All 512 values reachable. | selects `m__base[j]` (the result's base bit pattern) and `m__shift[j]` (how much of the mantissa survives) |
| **B. mantissa `n & 0x007fffff`** | `(n & 0x007fffff) >> m__shift[j]` | `0..=0x7FFFFF` | added to the base after truncation by `m__shift[j]` |

Cross-product pruned to what the code distinguishes: axis A collapses to the
**86 maximal `(base, shift)` runs** below (indices inside one run are treated
identically apart from the base value, which is swept per-index anyway); axis B
is swept per row with the boundary shapes `0`, `1`, `0x400000` (mantissa MSB),
`0x7FFFFE`, `0x7FFFFF` (all ones) **plus randomized mantissas** (fixed seed).

Distinct `m__shift` values exercised: `13,14,15,16,17,18,19,20,21,22,23,24`
(all 12 that occur in the table).

## Full set of public entry points

`float2half` is the **only** public entry point — it is simultaneously the
lowest-level and the highest-level API; there is no convenience wrapper to
mistake for the real interface. Every row is therefore driven through the
`.so` export directly via `libloading`, for both C and Rust.

## Which test covers which row

| rows | test |
|------|------|
| 1–86, all mantissa shapes | `tests/differential.rs::config_rows_all_86_base_shift_classes` |
| 1–86, randomized mantissas (seeded) | `tests/differential.rs::config_rows_randomized_mantissas` |
| 1–86, every individual `j` (all 512) | `tests/differential.rs::every_index_j_with_boundary_mantissas` |
| 1–86, exhaustive over all 2^32 inputs | `tests/exhaustive.rs::exhaustive_all_2_pow_32_inputs` |

## CONFIGURATION-SURFACE TABLE

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `float2half` | `j=0..102` (sign=0, exp=0..102) &rarr; base=`0x0000`, shift=`0x18`. +0.0, +subnormal f32, and small +normals -- ALL flush to +0.0h; shift 24 always yields 0. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 2 | `float2half` | `j=103` (sign=0, exp=103) &rarr; base=`0x0001`, shift=`0x17`. + f32 normal exp=103 -> half SUBNORMAL range; shift 23 keeps top 0 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 3 | `float2half` | `j=104` (sign=0, exp=104) &rarr; base=`0x0002`, shift=`0x16`. + f32 normal exp=104 -> half SUBNORMAL range; shift 22 keeps top 1 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 4 | `float2half` | `j=105` (sign=0, exp=105) &rarr; base=`0x0004`, shift=`0x15`. + f32 normal exp=105 -> half SUBNORMAL range; shift 21 keeps top 2 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 5 | `float2half` | `j=106` (sign=0, exp=106) &rarr; base=`0x0008`, shift=`0x14`. + f32 normal exp=106 -> half SUBNORMAL range; shift 20 keeps top 3 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 6 | `float2half` | `j=107` (sign=0, exp=107) &rarr; base=`0x0010`, shift=`0x13`. + f32 normal exp=107 -> half SUBNORMAL range; shift 19 keeps top 4 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 7 | `float2half` | `j=108` (sign=0, exp=108) &rarr; base=`0x0020`, shift=`0x12`. + f32 normal exp=108 -> half SUBNORMAL range; shift 18 keeps top 5 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 8 | `float2half` | `j=109` (sign=0, exp=109) &rarr; base=`0x0040`, shift=`0x11`. + f32 normal exp=109 -> half SUBNORMAL range; shift 17 keeps top 6 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 9 | `float2half` | `j=110` (sign=0, exp=110) &rarr; base=`0x0080`, shift=`0x10`. + f32 normal exp=110 -> half SUBNORMAL range; shift 16 keeps top 7 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 10 | `float2half` | `j=111` (sign=0, exp=111) &rarr; base=`0x0100`, shift=`0x0f`. + f32 normal exp=111 -> half SUBNORMAL range; shift 15 keeps top 8 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 11 | `float2half` | `j=112` (sign=0, exp=112) &rarr; base=`0x0200`, shift=`0x0e`. + f32 normal exp=112 -> half SUBNORMAL range; shift 14 keeps top 9 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 12 | `float2half` | `j=113` (sign=0, exp=113) &rarr; base=`0x0400`, shift=`0x0d`. + f32 exp=113 -> SMALLEST half NORMAL (base 0x0400), shift 13. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 13 | `float2half` | `j=114` (sign=0, exp=114) &rarr; base=`0x0800`, shift=`0x0d`. + f32 normal exp=114 -> half normal (base 0x0800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 14 | `float2half` | `j=115` (sign=0, exp=115) &rarr; base=`0x0c00`, shift=`0x0d`. + f32 normal exp=115 -> half normal (base 0x0c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 15 | `float2half` | `j=116` (sign=0, exp=116) &rarr; base=`0x1000`, shift=`0x0d`. + f32 normal exp=116 -> half normal (base 0x1000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 16 | `float2half` | `j=117` (sign=0, exp=117) &rarr; base=`0x1400`, shift=`0x0d`. + f32 normal exp=117 -> half normal (base 0x1400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 17 | `float2half` | `j=118` (sign=0, exp=118) &rarr; base=`0x1800`, shift=`0x0d`. + f32 normal exp=118 -> half normal (base 0x1800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 18 | `float2half` | `j=119` (sign=0, exp=119) &rarr; base=`0x1c00`, shift=`0x0d`. + f32 normal exp=119 -> half normal (base 0x1c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 19 | `float2half` | `j=120` (sign=0, exp=120) &rarr; base=`0x2000`, shift=`0x0d`. + f32 normal exp=120 -> half normal (base 0x2000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 20 | `float2half` | `j=121` (sign=0, exp=121) &rarr; base=`0x2400`, shift=`0x0d`. + f32 normal exp=121 -> half normal (base 0x2400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 21 | `float2half` | `j=122` (sign=0, exp=122) &rarr; base=`0x2800`, shift=`0x0d`. + f32 normal exp=122 -> half normal (base 0x2800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 22 | `float2half` | `j=123` (sign=0, exp=123) &rarr; base=`0x2c00`, shift=`0x0d`. + f32 normal exp=123 -> half normal (base 0x2c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 23 | `float2half` | `j=124` (sign=0, exp=124) &rarr; base=`0x3000`, shift=`0x0d`. + f32 normal exp=124 -> half normal (base 0x3000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 24 | `float2half` | `j=125` (sign=0, exp=125) &rarr; base=`0x3400`, shift=`0x0d`. + f32 normal exp=125 -> half normal (base 0x3400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 25 | `float2half` | `j=126` (sign=0, exp=126) &rarr; base=`0x3800`, shift=`0x0d`. + f32 normal exp=126 -> half normal (base 0x3800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 26 | `float2half` | `j=127` (sign=0, exp=127) &rarr; base=`0x3c00`, shift=`0x0d`. + f32 normal exp=127 -> half normal (base 0x3c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 27 | `float2half` | `j=128` (sign=0, exp=128) &rarr; base=`0x4000`, shift=`0x0d`. + f32 normal exp=128 -> half normal (base 0x4000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 28 | `float2half` | `j=129` (sign=0, exp=129) &rarr; base=`0x4400`, shift=`0x0d`. + f32 normal exp=129 -> half normal (base 0x4400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 29 | `float2half` | `j=130` (sign=0, exp=130) &rarr; base=`0x4800`, shift=`0x0d`. + f32 normal exp=130 -> half normal (base 0x4800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 30 | `float2half` | `j=131` (sign=0, exp=131) &rarr; base=`0x4c00`, shift=`0x0d`. + f32 normal exp=131 -> half normal (base 0x4c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 31 | `float2half` | `j=132` (sign=0, exp=132) &rarr; base=`0x5000`, shift=`0x0d`. + f32 normal exp=132 -> half normal (base 0x5000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 32 | `float2half` | `j=133` (sign=0, exp=133) &rarr; base=`0x5400`, shift=`0x0d`. + f32 normal exp=133 -> half normal (base 0x5400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 33 | `float2half` | `j=134` (sign=0, exp=134) &rarr; base=`0x5800`, shift=`0x0d`. + f32 normal exp=134 -> half normal (base 0x5800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 34 | `float2half` | `j=135` (sign=0, exp=135) &rarr; base=`0x5c00`, shift=`0x0d`. + f32 normal exp=135 -> half normal (base 0x5c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 35 | `float2half` | `j=136` (sign=0, exp=136) &rarr; base=`0x6000`, shift=`0x0d`. + f32 normal exp=136 -> half normal (base 0x6000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 36 | `float2half` | `j=137` (sign=0, exp=137) &rarr; base=`0x6400`, shift=`0x0d`. + f32 normal exp=137 -> half normal (base 0x6400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 37 | `float2half` | `j=138` (sign=0, exp=138) &rarr; base=`0x6800`, shift=`0x0d`. + f32 normal exp=138 -> half normal (base 0x6800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 38 | `float2half` | `j=139` (sign=0, exp=139) &rarr; base=`0x6c00`, shift=`0x0d`. + f32 normal exp=139 -> half normal (base 0x6c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 39 | `float2half` | `j=140` (sign=0, exp=140) &rarr; base=`0x7000`, shift=`0x0d`. + f32 normal exp=140 -> half normal (base 0x7000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 40 | `float2half` | `j=141` (sign=0, exp=141) &rarr; base=`0x7400`, shift=`0x0d`. + f32 normal exp=141 -> half normal (base 0x7400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 41 | `float2half` | `j=142` (sign=0, exp=142) &rarr; base=`0x7800`, shift=`0x0d`. + f32 normal exp=142 -> half normal (base 0x7800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 42 | `float2half` | `j=143..254` (sign=0, exp=143..254) &rarr; base=`0x7c00`, shift=`0x18`. + f32 exp=143..254 -> OVERFLOW, always +Inf (0x7c00); shift 24 discards mantissa. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 43 | `float2half` | `j=255` (sign=0, exp=255) &rarr; base=`0x7c00`, shift=`0x0d`. +Inf / +NaN (exp=255) -> 0x7c00 + (mant>>13); NaN payload propagates into low 10 bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 44 | `float2half` | `j=256..358` (sign=1, exp=0..102) &rarr; base=`0x8000`, shift=`0x18`. -0.0, -subnormal f32, and small -normals -- ALL flush to -0.0h; shift 24 always yields 0. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 45 | `float2half` | `j=359` (sign=1, exp=103) &rarr; base=`0x8001`, shift=`0x17`. - f32 normal exp=103 -> half SUBNORMAL range; shift 23 keeps top 0 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 46 | `float2half` | `j=360` (sign=1, exp=104) &rarr; base=`0x8002`, shift=`0x16`. - f32 normal exp=104 -> half SUBNORMAL range; shift 22 keeps top 1 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 47 | `float2half` | `j=361` (sign=1, exp=105) &rarr; base=`0x8004`, shift=`0x15`. - f32 normal exp=105 -> half SUBNORMAL range; shift 21 keeps top 2 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 48 | `float2half` | `j=362` (sign=1, exp=106) &rarr; base=`0x8008`, shift=`0x14`. - f32 normal exp=106 -> half SUBNORMAL range; shift 20 keeps top 3 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 49 | `float2half` | `j=363` (sign=1, exp=107) &rarr; base=`0x8010`, shift=`0x13`. - f32 normal exp=107 -> half SUBNORMAL range; shift 19 keeps top 4 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 50 | `float2half` | `j=364` (sign=1, exp=108) &rarr; base=`0x8020`, shift=`0x12`. - f32 normal exp=108 -> half SUBNORMAL range; shift 18 keeps top 5 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 51 | `float2half` | `j=365` (sign=1, exp=109) &rarr; base=`0x8040`, shift=`0x11`. - f32 normal exp=109 -> half SUBNORMAL range; shift 17 keeps top 6 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 52 | `float2half` | `j=366` (sign=1, exp=110) &rarr; base=`0x8080`, shift=`0x10`. - f32 normal exp=110 -> half SUBNORMAL range; shift 16 keeps top 7 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 53 | `float2half` | `j=367` (sign=1, exp=111) &rarr; base=`0x8100`, shift=`0x0f`. - f32 normal exp=111 -> half SUBNORMAL range; shift 15 keeps top 8 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 54 | `float2half` | `j=368` (sign=1, exp=112) &rarr; base=`0x8200`, shift=`0x0e`. - f32 normal exp=112 -> half SUBNORMAL range; shift 14 keeps top 9 mantissa bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 55 | `float2half` | `j=369` (sign=1, exp=113) &rarr; base=`0x8400`, shift=`0x0d`. - f32 exp=113 -> SMALLEST half NORMAL (base 0x8400), shift 13. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 56 | `float2half` | `j=370` (sign=1, exp=114) &rarr; base=`0x8800`, shift=`0x0d`. - f32 normal exp=114 -> half normal (base 0x8800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 57 | `float2half` | `j=371` (sign=1, exp=115) &rarr; base=`0x8c00`, shift=`0x0d`. - f32 normal exp=115 -> half normal (base 0x8c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 58 | `float2half` | `j=372` (sign=1, exp=116) &rarr; base=`0x9000`, shift=`0x0d`. - f32 normal exp=116 -> half normal (base 0x9000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 59 | `float2half` | `j=373` (sign=1, exp=117) &rarr; base=`0x9400`, shift=`0x0d`. - f32 normal exp=117 -> half normal (base 0x9400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 60 | `float2half` | `j=374` (sign=1, exp=118) &rarr; base=`0x9800`, shift=`0x0d`. - f32 normal exp=118 -> half normal (base 0x9800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 61 | `float2half` | `j=375` (sign=1, exp=119) &rarr; base=`0x9c00`, shift=`0x0d`. - f32 normal exp=119 -> half normal (base 0x9c00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 62 | `float2half` | `j=376` (sign=1, exp=120) &rarr; base=`0xa000`, shift=`0x0d`. - f32 normal exp=120 -> half normal (base 0xa000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 63 | `float2half` | `j=377` (sign=1, exp=121) &rarr; base=`0xa400`, shift=`0x0d`. - f32 normal exp=121 -> half normal (base 0xa400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 64 | `float2half` | `j=378` (sign=1, exp=122) &rarr; base=`0xa800`, shift=`0x0d`. - f32 normal exp=122 -> half normal (base 0xa800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 65 | `float2half` | `j=379` (sign=1, exp=123) &rarr; base=`0xac00`, shift=`0x0d`. - f32 normal exp=123 -> half normal (base 0xac00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 66 | `float2half` | `j=380` (sign=1, exp=124) &rarr; base=`0xb000`, shift=`0x0d`. - f32 normal exp=124 -> half normal (base 0xb000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 67 | `float2half` | `j=381` (sign=1, exp=125) &rarr; base=`0xb400`, shift=`0x0d`. - f32 normal exp=125 -> half normal (base 0xb400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 68 | `float2half` | `j=382` (sign=1, exp=126) &rarr; base=`0xb800`, shift=`0x0d`. - f32 normal exp=126 -> half normal (base 0xb800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 69 | `float2half` | `j=383` (sign=1, exp=127) &rarr; base=`0xbc00`, shift=`0x0d`. - f32 normal exp=127 -> half normal (base 0xbc00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 70 | `float2half` | `j=384` (sign=1, exp=128) &rarr; base=`0xc000`, shift=`0x0d`. - f32 normal exp=128 -> half normal (base 0xc000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 71 | `float2half` | `j=385` (sign=1, exp=129) &rarr; base=`0xc400`, shift=`0x0d`. - f32 normal exp=129 -> half normal (base 0xc400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 72 | `float2half` | `j=386` (sign=1, exp=130) &rarr; base=`0xc800`, shift=`0x0d`. - f32 normal exp=130 -> half normal (base 0xc800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 73 | `float2half` | `j=387` (sign=1, exp=131) &rarr; base=`0xcc00`, shift=`0x0d`. - f32 normal exp=131 -> half normal (base 0xcc00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 74 | `float2half` | `j=388` (sign=1, exp=132) &rarr; base=`0xd000`, shift=`0x0d`. - f32 normal exp=132 -> half normal (base 0xd000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 75 | `float2half` | `j=389` (sign=1, exp=133) &rarr; base=`0xd400`, shift=`0x0d`. - f32 normal exp=133 -> half normal (base 0xd400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 76 | `float2half` | `j=390` (sign=1, exp=134) &rarr; base=`0xd800`, shift=`0x0d`. - f32 normal exp=134 -> half normal (base 0xd800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 77 | `float2half` | `j=391` (sign=1, exp=135) &rarr; base=`0xdc00`, shift=`0x0d`. - f32 normal exp=135 -> half normal (base 0xdc00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 78 | `float2half` | `j=392` (sign=1, exp=136) &rarr; base=`0xe000`, shift=`0x0d`. - f32 normal exp=136 -> half normal (base 0xe000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 79 | `float2half` | `j=393` (sign=1, exp=137) &rarr; base=`0xe400`, shift=`0x0d`. - f32 normal exp=137 -> half normal (base 0xe400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 80 | `float2half` | `j=394` (sign=1, exp=138) &rarr; base=`0xe800`, shift=`0x0d`. - f32 normal exp=138 -> half normal (base 0xe800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 81 | `float2half` | `j=395` (sign=1, exp=139) &rarr; base=`0xec00`, shift=`0x0d`. - f32 normal exp=139 -> half normal (base 0xec00), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 82 | `float2half` | `j=396` (sign=1, exp=140) &rarr; base=`0xf000`, shift=`0x0d`. - f32 normal exp=140 -> half normal (base 0xf000), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 83 | `float2half` | `j=397` (sign=1, exp=141) &rarr; base=`0xf400`, shift=`0x0d`. - f32 normal exp=141 -> half normal (base 0xf400), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 84 | `float2half` | `j=398` (sign=1, exp=142) &rarr; base=`0xf800`, shift=`0x0d`. - f32 normal exp=142 -> half normal (base 0xf800), shift 13, mantissa truncated (no rounding). Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 85 | `float2half` | `j=399..510` (sign=1, exp=143..254) &rarr; base=`0xfc00`, shift=`0x18`. - f32 exp=143..254 -> OVERFLOW, always -Inf (0xfc00); shift 24 discards mantissa. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
| 86 | `float2half` | `j=511` (sign=1, exp=255) &rarr; base=`0xfc00`, shift=`0x0d`. -Inf / -NaN (exp=255) -> 0xfc00 + (mant>>13); NaN payload propagates into low 10 bits. Mantissa shapes swept: `0`, `1`, `0x400000`, `0x7FFFFE`, `0x7FFFFF` + randomized | [x] |
