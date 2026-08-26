# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| **A. entry point** | `call_predict` (the only *exported* one), `BTAC1C2_GetPredictFunc` (dispatcher, `static`), `BTAC1C2_PredictSample` (generic 17-arm predictor, `static`), `BTAC1C2_PredictSample_Pfn0` … `_Pfn11` (12 specialised predictors, `static`) | lib.c:18, 105–181, 183, 229 |
| **B. `pfcn`** | `0,1,2,3,4,5,6,7,8,9,10,11` (all three switches branch), `12,13,14,15` (only `BTAC1C2_PredictSample` has arms; they index `ridx->firfx[pfcn-12]`), anything else (3× `default:`) | lib.c:22–101, 185–225, 232–271 |
| **C. `idx` (ring phase)** | every value of `idx & 7` → 8 distinct phases; sign of `idx` (C computes `(idx - n) & 7` on a *signed* int); large magnitudes | every `psamp[(idx - n) & 7]` |
| **D. `psamp[8]` shape** | zeros; constant; ascending ramp; descending ramp; alternating ±extremes; all-negative (exercises arithmetic `>>` on negatives vs. `/` truncation-toward-zero — the two differ for negative operands, and the file uses **both**: `>>1/2/3/4` in arms 2–6,10,11 and `/16`, `/64`, `/256` in arms 7,8,9,12–15); random 16-bit audio range; random near-overflow magnitudes | lib.c:24–96, 106–181 |
| **E. `ridx->firfx[4][8]` shape** (only reachable for `pfcn` 12–15) | zeros; unit gain (256); random `s16`; `s16::MIN`/`s16::MAX` extremes; per-row distinct values (verifies the `[pfcn-12]` row selection) | lib.c:88–96 |
| **F. struct layout** | `btac1c_idxstate` = `u16,s16,s16,u8,u8,u8,u8,s16[4][8]` — the pointer is passed across the FFI boundary, so size/align/field offsets must agree | lib.c:7–16 |
| **G. `#ifdef` / build config** | **none.** `c_src/CMakeLists.txt` has no `target_compile_definitions`, no `option()`, no conditional sources; `lib.c`/`lib.h` contain no `#ifdef`. `Cargo.toml` has **no `[features]` section** ⇒ exactly one feature combination: the empty set (`--no-default-features`, identical to default). | CMakeLists.txt, Cargo.toml |

Notes on quirks that MUST be preserved (they make otherwise-identical arms differ,
so the rows below are not redundant):
* `case 10` uses `(5*p0 - p1) >> 4` but `_Pfn10` uses `>> 3` (lib.c:75 vs :171).
* `case 11` uses `(p0 + p1) >> 3` but `_Pfn11` uses `>> 1` (lib.c:82 vs :180).
* `_Pfn0..._Pfn11` ignore `pfcn` and `ridx` entirely.

## Rows (pruned cross-product of the axes above)

Every row is driven with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, a splitmix64/PCG-style generator in the test) crossed with
the whole of axes C/D/E, and compared byte-for-byte between the C `.so` and the
Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | verified by (tests/differential.rs) | [x] |
|---|----------------|-------------------------------------------|-------------------------------------|-----|
| 1 | `call_predict` (exported) | `pfcn` exhaustive over the whole valid dispatch range `0..=11` | `cfg01_call_predict_valid_range` | [x] |
| 2 | `call_predict` (exported) | `pfcn` = `12,13,14,15` — valid `BTAC1C2_PredictSample` arms, invalid for the dispatcher | `cfg02_call_predict_predictsample_only_arms` | [x] |
| 3 | `call_predict` (exported) | `pfcn` exhaustive over `-64..=64` plus 4096 random `i32` values plus `{INT_MIN, INT_MIN+1, -1, 16, 255, 256, 65535, 65536, INT_MAX-1, INT_MAX}` | `err_call_predict_out_of_range + err_call_predict_exhaustive_every_i32` | [x] |
| 4 | `call_predict` (exported) | same `pfcn` called repeatedly (16×) — function-pointer identity must be stable across calls, not just on first use | `cfg04_call_predict_repeated` | [x] |
| 5 | `BTAC1C2_PredictSample` (generic) | `pfcn=0` × all 8 `idx` phases × all 8 `psamp` shapes × randomized | `cfg05_generic_arm0` | [x] |
| 6 | `BTAC1C2_PredictSample` | `pfcn=1` × all `idx` phases × all `psamp` shapes × randomized | `cfg06_generic_arm1` | [x] |
| 7 | `BTAC1C2_PredictSample` | `pfcn=2` (`>>1` on possibly-negative value) × all phases/shapes | `cfg07_generic_arm2` | [x] |
| 8 | `BTAC1C2_PredictSample` | `pfcn=3` (`>>2`) × all phases/shapes | `cfg08_generic_arm3` | [x] |
| 9 | `BTAC1C2_PredictSample` | `pfcn=4` (`p0 - (p1>>1)`) × all phases/shapes | `cfg09_generic_arm4` | [x] |
| 10 | `BTAC1C2_PredictSample` | `pfcn=5` (`(3*p0-p1)>>2`) × all phases/shapes | `cfg10_generic_arm5` | [x] |
| 11 | `BTAC1C2_PredictSample` | `pfcn=6` (`(5*p0-p1)>>3`) × all phases/shapes | `cfg11_generic_arm6` | [x] |
| 12 | `BTAC1C2_PredictSample` | `pfcn=7` (5-tap, `/16` truncating division) × all phases/shapes | `cfg12_generic_arm7` | [x] |
| 13 | `BTAC1C2_PredictSample` | `pfcn=8` (8-tap, `/64`) × all phases/shapes | `cfg13_generic_arm8` | [x] |
| 14 | `BTAC1C2_PredictSample` | `pfcn=9` (8-tap, `/64`, different taps) × all phases/shapes | `cfg14_generic_arm9` | [x] |
| 15 | `BTAC1C2_PredictSample` | `pfcn=10` (`(5*p0-p1)>>4` — differs from `_Pfn10`) × all phases/shapes | `cfg15_generic_arm10` | [x] |
| 16 | `BTAC1C2_PredictSample` | `pfcn=11` (`(p0+p1)>>3` — differs from `_Pfn11`) × all phases/shapes | `cfg16_generic_arm11` | [x] |
| 17 | `BTAC1C2_PredictSample` | `pfcn=12` (`firfx[0]`, `/256`) × all phases × all `psamp` shapes × all 5 `firfx` shapes | `cfg17_generic_arm12_firfx0` | [x] |
| 18 | `BTAC1C2_PredictSample` | `pfcn=13` (`firfx[1]`) × all phases/shapes/firfx shapes | `cfg18_generic_arm13_firfx1` | [x] |
| 19 | `BTAC1C2_PredictSample` | `pfcn=14` (`firfx[2]`) × all phases/shapes/firfx shapes | `cfg19_generic_arm14_firfx2` | [x] |
| 20 | `BTAC1C2_PredictSample` | `pfcn=15` (`firfx[3]`) × all phases/shapes/firfx shapes | `cfg20_generic_arm15_firfx3` | [x] |
| 21 | `BTAC1C2_PredictSample_Pfn0` | direct call × all 8 `idx` phases × all `psamp` shapes × randomized (`pfcn`/`ridx` args varied too — they are ignored, so varying them must change nothing) | `cfg21_pfn0` | [x] |
| 22 | `BTAC1C2_PredictSample_Pfn1` | direct call × all phases/shapes | `cfg22_pfn1` | [x] |
| 23 | `BTAC1C2_PredictSample_Pfn2` | direct call × all phases/shapes | `cfg23_pfn2` | [x] |
| 24 | `BTAC1C2_PredictSample_Pfn3` | direct call × all phases/shapes | `cfg24_pfn3` | [x] |
| 25 | `BTAC1C2_PredictSample_Pfn4` | direct call × all phases/shapes | `cfg25_pfn4` | [x] |
| 26 | `BTAC1C2_PredictSample_Pfn5` | direct call × all phases/shapes | `cfg26_pfn5` | [x] |
| 27 | `BTAC1C2_PredictSample_Pfn6` | direct call × all phases/shapes | `cfg27_pfn6` | [x] |
| 28 | `BTAC1C2_PredictSample_Pfn7` | direct call × all phases/shapes | `cfg28_pfn7` | [x] |
| 29 | `BTAC1C2_PredictSample_Pfn8` | direct call × all phases/shapes | `cfg29_pfn8` | [x] |
| 30 | `BTAC1C2_PredictSample_Pfn9` | direct call × all phases/shapes | `cfg30_pfn9` | [x] |
| 31 | `BTAC1C2_PredictSample_Pfn10` | direct call × all phases/shapes — must use `>>3` (NOT `>>4` like arm 10) | `cfg31_pfn10` | [x] |
| 32 | `BTAC1C2_PredictSample_Pfn11` | direct call × all phases/shapes — must use `>>1` (NOT `>>3` like arm 11) | `cfg32_pfn11` | [x] |
| 33 | `BTAC1C2_GetPredictFunc` + call through returned pointer | `pfcn = 0..=11`: verifies the dispatch table maps each `pfcn` to the *right* predictor (a swapped pair would be invisible to `call_predict`) × all phases/shapes | `cfg33_dispatch_valid_range` | [x] |
| 34 | `BTAC1C2_GetPredictFunc` + call through | `pfcn = 12..=15`: falls through `default:` to the generic `BTAC1C2_PredictSample`, which then takes its own `firfx` arms × all firfx shapes | `cfg34_dispatch_firfx_arms` | [x] |
| 35 | `BTAC1C2_GetPredictFunc` + call through | `pfcn` out of range (`-1`, `16`, `INT_MIN`, `INT_MAX`, randoms): generic predictor's `default:` arm ⇒ 0 | `err_dispatch_default_arm` | [x] |
| 36 | struct `btac1c_idxstate` | size, alignment and all 8 field offsets reported by both TUs must be identical (prerequisite for passing `ridx` across the boundary) | `cfg36_struct_layout` | [x] |
| 37 | all predictors (generic + `Pfn0..11`, via dispatch) | `idx` extremes: `INT_MIN+8`, `INT_MIN+9`, `-1`, `0`, `7`, `8`, `INT_MAX-8`, `INT_MAX`, and 512 random `idx` in the overflow-safe range | `cfg37_idx_extremes` | [x] |
| 38 | all predictors | `psamp` all-negative and mixed-sign (isolates arithmetic-shift vs. truncating-division rounding for every arm) | `cfg38_39_value_shapes` | [x] |
| 39 | all predictors | `psamp` at the largest magnitudes that keep the C accumulators inside `int` (`±2^20`, `±(2^20-1)`, `i16::MIN`/`i16::MAX`) — value-dependent path check | `cfg38_39_value_shapes` | [x] |
| 40 | `BTAC1C2_PredictSample` `pfcn=12..15` | `firfx` rows filled with *distinct* patterns per row + `i16::MIN`/`i16::MAX`/`±256` extremes: verifies the `[pfcn-12]` row index and the `/256` truncation on negative accumulators | `cfg40_firfx_row_selection` | [x] |
| 41 | `call_predict` vs. dispatcher | consistency: for `pfcn` in `0..=11`, `call_predict` must return 1 **and** `GetPredictFunc(pfcn)` must behave like `Pfn<pfcn>` in the *same* library — cross-checked in both libraries independently | `cfg41_dispatch_identity` | [x] |
| 42 | exported ABI | `call_predict` called through a `extern "C" fn(c_int) -> c_int` pointer obtained from `dlsym` on both `.so`s (no direct Rust call anywhere) | `cfg42_exported_surface_is_int_int` | [x] |

## Result

All 42 rows pass. Every row compares the C `.so` against the Rust `.so`
call-for-call through `dlsym`; the grid runner additionally asserts that
neither implementation mutates `psamp` or `*ridx`.

Beyond the randomized grid, the two *exhaustive* sweeps
`err_call_predict_exhaustive_every_i32` and
`err_generic_predict_exhaustive_every_pfcn` (marked `#[ignore]`, run with
`cargo test --release -- --ignored`, ~36 s) compare **all 2^32 possible `int`
inputs** of the exported entry point and of the generic predictor's arm
selection, so axis B is covered without sampling.

Reproduce:

```
cargo build --release            # shipped Rust cdylib
cargo test  --release            # 45 differential tests
cargo test  --release -- --ignored          # 2 exhaustive 2^32 sweeps
AUX_OVERFLOW_CHECKS=on cargo test --release # same, Rust shim with overflow checks ON
bash run_all_configs.sh          # every feature combo x profile + symbol parity
```

### Harness self-validation (mutation testing)

To prove the suite is not vacuously passing, 8 deliberate mutations were
injected into `src/lib.rs` one at a time and the suite re-run (`src/lib.rs` was
restored bit-identically afterwards, verified with `diff`):

| mutation | detected by |
|----------|-------------|
| arm 10 `>>4` → `>>3` | `cfg15_generic_arm10` |
| `_Pfn11` `>>1` → `>>3` | `cfg32_pfn11` |
| arm 7 `/16` → `>>4` (only differs for negatives) | `cfg38_39_value_shapes` |
| dispatch table entries 10 and 11 swapped | `cfg33_dispatch_valid_range` |
| generic `default:` arm returns 1 instead of 0 | `err_generic_predict_default_arm` |
| `firfx[pfcn-12]` → `firfx[(pfcn-12)^1]` | `err_firfx_index_boundary` |
| `call_predict` arm 11 forced to 0 | `cfg01_call_predict_valid_range` |
| `& 7` → `& 15 & 7` (semantically identical — must NOT be flagged) | not detected, as expected |

7/7 behaviour-changing mutations were caught; the one equivalent mutation was
correctly not flagged.
