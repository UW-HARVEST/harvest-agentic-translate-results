# CONFIGS.md — Phase A: CONFIGURATION-SURFACE TABLE (valid inputs)

Axes derived **mechanically** from `c_src/src/lib.c` — every `switch`/`case` the
C branches on, every distinct input shape it special-cases. There are no
`#ifdef`s, no global/runtime options, no init function and no opaque handle:
the only runtime "option" is the `pfcn` selector, and the only data shapes are
the 8-entry `psamp` ring window, the `idx` rotation of that window, and the
`btac1c_idxstate.firfx[4][8]` FIR coefficient matrix.

## Public entry points

| level | entry point | reachable how |
|-------|-------------|---------------|
| public ABI | `int call_predict(int pfcn)` | `nm -D` / `libloading` — the only exported symbol |
| internal (lowest level) | `BTAC1C2_PredictSample(int*, int, int, btac1c_idxstate*)` | `static`; reached in tests via `nm` local-symbol offset + runtime load base |
| internal (lowest level) | `BTAC1C2_PredictSample_Pfn0 .. _Pfn11` (12 fns) | same |
| internal | `void *BTAC1C2_GetPredictFunc(int pfcn)` | same |

The low-level predictors are exercised **directly** (not only through the
`call_predict` wrapper) in `tests/internal_predictors.rs`; `call_predict` is
exercised through the `#[no_mangle]` export in `tests/differential.rs`.

## Axes

* **A1 `pfcn` selector** (the runtime mode flag): `0..11` = specialised
  predictor; `12..15` = FIR predictor rows `firfx[0..3]` (generic fn only);
  anything else = fall-back.
* **A2 `idx` rotation of the ring window**: `0..7` plus values whose `& 7`
  wraps (`8`, `-1`, `INT_MIN`, `INT_MAX`, large randoms). Determines which
  `psamp` slot each `-k` tap lands on.
* **A3 `psamp` value shape**: all-zero; all-equal (constant signal); ramp;
  alternating ±; single non-zero (impulse); all-negative (exercises `>>` vs `/`
  rounding); saturated `i16` range; full `i32` range near `INT_MIN`/`INT_MAX`
  (wrap-around); fixed-seed uniform randoms.
* **A4 `firfx` shape** (only read for `pfcn` 12..15): row selected is
  `pfcn - 12`; coefficients are `signed short` promoted to `int` — zero row,
  all-`+32767`, all-`-32768`, mixed sign, randoms.
* **A5 tap depth**: predictors use 2, 3, 5 or 8 taps — the shape must be varied
  so that value-dependent taps beyond the first two are actually reached.

## Rows (cross-product, pruned to what the C actually distinguishes)

| # | entry point(s) | configuration (options set + input shape) | ✔ |
|---|----------------|-------------------------------------------|---|
| C1 | `call_predict` | `pfcn = 0` (specialised `Pfn0` selected & identity-compared) | [x] |
| C2 | `call_predict` | `pfcn = 1` | [x] |
| C3 | `call_predict` | `pfcn = 2` | [x] |
| C4 | `call_predict` | `pfcn = 3` | [x] |
| C5 | `call_predict` | `pfcn = 4` | [x] |
| C6 | `call_predict` | `pfcn = 5` | [x] |
| C7 | `call_predict` | `pfcn = 6` | [x] |
| C8 | `call_predict` | `pfcn = 7` | [x] |
| C9 | `call_predict` | `pfcn = 8` | [x] |
| C10 | `call_predict` | `pfcn = 9` | [x] |
| C11 | `call_predict` | `pfcn = 10` | [x] |
| C12 | `call_predict` | `pfcn = 11` (last in-range value) | [x] |
| C13 | `call_predict` | full contiguous sweep `pfcn = -8..=32` (boundary crossing in both directions) | [x] |
| C14 | `call_predict` | 20 000 fixed-seed random `i32` selectors + all `±2^k` (repeated-call determinism / no state) | [x] |
| C15 | `call_predict` | repeated invocation of the same `pfcn` 1 000× — asserts the library is stateless and function-pointer identity is stable across calls | [x] |
| C16 | `BTAC1C2_PredictSample_Pfn0` (1-tap) | `idx = 0..7`; `psamp` = zero / const / ramp / alternating / impulse / negative / i16-saturated / i32-extreme / random ×64 | [x] |
| C17 | `BTAC1C2_PredictSample_Pfn1` (2-tap, `2a-b`) | same A2×A3 matrix (wrap-around on `2*a-b` at `i32` extremes) | [x] |
| C18 | `BTAC1C2_PredictSample_Pfn2` (2-tap, `(3a-b)>>1`) | same A2×A3 matrix, incl. negative values (arithmetic-shift rounding) | [x] |
| C19 | `BTAC1C2_PredictSample_Pfn3` (2-tap, `(5a-b)>>2`) | same A2×A3 matrix | [x] |
| C20 | `BTAC1C2_PredictSample_Pfn4` (3-tap, `p0-(p1>>1)`) | same A2×A3 matrix | [x] |
| C21 | `BTAC1C2_PredictSample_Pfn5` (3-tap, `(3p0-p1)>>2`) | same A2×A3 matrix | [x] |
| C22 | `BTAC1C2_PredictSample_Pfn6` (3-tap, `(5p0-p1)>>3`) | same A2×A3 matrix | [x] |
| C23 | `BTAC1C2_PredictSample_Pfn7` (5-tap, `/16` truncating division) | same A2×A3 matrix; negatives specifically, since `/` truncates while `>>` floors | [x] |
| C24 | `BTAC1C2_PredictSample_Pfn8` (8-tap, `/64`) | same A2×A3 matrix; negatives + i32 extremes (wrapping mul/add before divide) | [x] |
| C25 | `BTAC1C2_PredictSample_Pfn9` (8-tap, `/64`, different coefficients) | same A2×A3 matrix | [x] |
| C26 | `BTAC1C2_PredictSample_Pfn10` (8-tap, `(5p0-p1)>>3` — note: differs from `case 10`, which is `>>4`) | same A2×A3 matrix | [x] |
| C27 | `BTAC1C2_PredictSample_Pfn11` (8-tap, `(p0+p1)>>1` — note: differs from `case 11`, which is `>>3`) | same A2×A3 matrix | [x] |
| C28 | `BTAC1C2_PredictSample` generic | `pfcn = 0..11` — the switch bodies, which must match the `Pfn*` variants **except** for `pfcn` 10 and 11 where the C deliberately differs | [x] |
| C29 | `BTAC1C2_PredictSample` generic | `pfcn = 12` → `firfx[0]`; coefficient rows: zero / `+32767` / `-32768` / mixed / random ×64, `psamp` from the A3 set | [x] |
| C30 | `BTAC1C2_PredictSample` generic | `pfcn = 13` → `firfx[1]`, same A4×A3 matrix | [x] |
| C31 | `BTAC1C2_PredictSample` generic | `pfcn = 14` → `firfx[2]`, same A4×A3 matrix | [x] |
| C32 | `BTAC1C2_PredictSample` generic | `pfcn = 15` → `firfx[3]` (last FIR row), same A4×A3 matrix | [x] |
| C33 | `BTAC1C2_PredictSample` generic | `idx` outside `0..7` (`8`, `-1`, `-9`, `INT_MIN`, `INT_MAX`, randoms) for every `pfcn` `0..15` — exercises the `& 7` masking | [x] |
| C34 | `BTAC1C2_GetPredictFunc` | `pfcn = 0..11` each returns a *distinct* pointer; identity is what `call_predict` observes | [x] |
| C35 | all `Pfn*` + generic | full randomized property sweep: 4 000 fixed-seed cases × (random `pfcn` 0..15, random `idx` full `i32`, random 8-word `psamp`, random `firfx[4][8]`) driven through both `.so`s | [x] |
| C36 | `call_predict` (release `.so`) | the shipped `--release` artifact (predictors inlined + pointer comparisons constant-folded) must give the same answers as the C `-O0` build | [x] |
| C37 | `call_predict` (debug `.so`) | the un-optimised artifact (predictors kept as real, distinct functions) must give the same answers | [x] |

## Feature combinations

`Cargo.toml` declares no `[features]`; the complete set of combinations is
therefore `{default}` and `{--no-default-features}`. `run_all_feature_combos.sh`
runs the whole suite under both.
