# CONFIGS.md — Phase A: configuration surface table (valid inputs)

Derived mechanically from the `switch` arms of `c_src/src/lib.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| A1 `pfcn` selecting a specialised predictor | `0,1,2,3,4,5,6,7,8,9,10,11` + `default` | `BTAC1C2_GetPredictFunc` switch (lines 185-225) and `get_predict_func` switch (lines 232-271) |
| A2 `pfcn` inside the generic predictor | `0..=11` individually, `12\|13\|14\|15` grouped, `default` | `BTAC1C2_PredictSample` switch (lines 22-101) |
| A3 entry point | `get_predict_func` (public), `BTAC1C2_GetPredictFunc` (internal), `BTAC1C2_PredictSample` (internal, lowest level), `BTAC1C2_PredictSample_Pfn0..11` (internal, lowest level) | whole file |
| A4 history depth touched | 2 taps (`pfcn` 0-3), 3 taps (4-6), 5 taps (7), 8 taps (8-11, 12-15) | index expressions `(i-1)..(i-8)` |
| A5 `idx` shape | in-range `0..=7`; `8..` (wraps via `&7`); `0..=-8` negative (masking of negatives); `INT_MIN`, `INT_MAX` (signed wrap in `idx - k`) | `psamp[(i - k) & 7]` |
| A6 `psamp[]` value magnitude | small (`|v| < 1000`, no overflow), medium (`|v| ~ 2^20`), extreme (`i32::MIN`/`MAX`, overflow in `72*v`, `5*p0`) | multiplications in every arm |
| A7 `psamp[]` sign pattern | all-positive, all-negative, mixed (drives sign of `>>` and of truncating `/`) | `>> n` (arithmetic shift, rounds toward −∞) vs `/ n` (truncates toward 0) |
| A8 `ridx->firfx[row][tap]` | `row = pfcn - 12` ∈ `{0,1,2,3}`; taps `0..7`; values incl. `0`, `i16::MIN`, `i16::MAX`, mixed | `case 12..15` |
| A9 rounding operator | arithmetic right shift (`>>1,>>2,>>3,>>4`) vs truncating divide (`/16`, `/64`, `/256`) | arms 0-6,10,11 vs 7-9,12-15 |
| A10 Cargo feature set | `<none>` (default) and `difftest` | `translation/Cargo.toml [features]` |

Note the C's intentional inconsistencies that MUST be reproduced (they make the
specialised and generic paths distinct configurations, not duplicates):

* `case 10` uses `>> 4` but `Pfn10` uses `>> 3`.
* `case 11` uses `>> 3` but `Pfn11` uses `>> 1`.

## Configuration table

Legend for "entry point": `PUB` = `get_predict_func`; `GEN` =
`BTAC1C2_PredictSample` via `__difftest_predict(which = 99, …, pfcn = …)`;
`SPEC N` = `BTAC1C2_PredictSample_PfnN` via `__difftest_predict(which = N, …)`.
Every row is exercised with **many randomized inputs (seeded, deterministic)**,
not one hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | PUB | `pfcn` swept over every value in `0..=11` (the whole valid domain) | [x] |
| 2 | PUB + GEN + SPEC, cross-checked | `BTAC1C2_GetPredictFunc` identity: for each `pfcn` in `0..=11`, the pointer returned equals `PfnN` — asserted via `PUB(pfcn) == 1` on both sides | [x] |
| 3 | SPEC 0 | 2-tap; `idx` random full-`i32`; `psamp` small values, mixed signs | [x] |
| 4 | SPEC 0 | `idx` in `0..=7` exhaustively × `psamp` extreme (`i32::MIN`/`MAX`) | [x] |
| 5 | SPEC 1 | 2-tap, `>>`-free `2*a - b`; small values, all sign patterns | [x] |
| 6 | SPEC 1 | overflow shape: `psamp` at `i32::MIN`/`i32::MAX` so `2*a-b` wraps | [x] |
| 7 | SPEC 2 | `(3a - b) >> 1`; mixed signs (exercises arithmetic-shift rounding of negatives) | [x] |
| 8 | SPEC 3 | `(5a - b) >> 2`; mixed signs + extreme magnitudes | [x] |
| 9 | SPEC 4 | 3-tap `p0 - (p1 >> 1)`; small + extreme | [x] |
| 10 | SPEC 5 | 3-tap `(3*p0 - p1) >> 2`; small + extreme | [x] |
| 11 | SPEC 6 | 3-tap `(5*p0 - p1) >> 3`; small + extreme | [x] |
| 12 | SPEC 7 | 5-tap, **truncating divide** `/16`; positive-only (divide rounds toward 0) | [x] |
| 13 | SPEC 7 | 5-tap `/16`; negative-only and mixed (divide toward 0 ≠ shift toward −∞) | [x] |
| 14 | SPEC 8 | 8-tap `/64` with large coefficients (`72`, `16`, …); small values | [x] |
| 15 | SPEC 8 | 8-tap `/64`; extreme values so `72 * v` overflows `i32` | [x] |
| 16 | SPEC 9 | 8-tap `/64` (`76,17,10,7,5,4,4,3`); small + mixed signs | [x] |
| 17 | SPEC 9 | 8-tap `/64`; extreme values, overflow | [x] |
| 18 | SPEC 10 | 8-tap `(5*p0 - p1) >> 3` — **note: 3, not 4** (differs from `case 10`); small + mixed | [x] |
| 19 | SPEC 10 | same, extreme values | [x] |
| 20 | SPEC 11 | 8-tap `(p0 + p1) >> 1` — **note: 1, not 3** (differs from `case 11`); small + mixed | [x] |
| 21 | SPEC 11 | same, extreme values | [x] |
| 22 | GEN, `pfcn = 0` | generic switch arm 0; random `idx`, small + extreme `psamp` | [x] |
| 23 | GEN, `pfcn = 1` | generic arm 1 | [x] |
| 24 | GEN, `pfcn = 2` | generic arm 2 (`>>1`) | [x] |
| 25 | GEN, `pfcn = 3` | generic arm 3 (`>>2`) | [x] |
| 26 | GEN, `pfcn = 4` | generic arm 4 (3-tap) | [x] |
| 27 | GEN, `pfcn = 5` | generic arm 5 | [x] |
| 28 | GEN, `pfcn = 6` | generic arm 6 | [x] |
| 29 | GEN, `pfcn = 7` | generic arm 7 (`/16`), all sign patterns | [x] |
| 30 | GEN, `pfcn = 8` | generic arm 8 (`/64`), all sign patterns + overflow | [x] |
| 31 | GEN, `pfcn = 9` | generic arm 9 (`/64`), all sign patterns + overflow | [x] |
| 32 | GEN, `pfcn = 10` | generic arm 10 — `>> 4` (must NOT equal SPEC 10's `>> 3`) | [x] |
| 33 | GEN, `pfcn = 11` | generic arm 11 — `>> 3` (must NOT equal SPEC 11's `>> 1`) | [x] |
| 34 | GEN, `pfcn = 12` | FIR arm, `firfx` row 0, random `i16` taps, `/256` | [x] |
| 35 | GEN, `pfcn = 13` | FIR arm, `firfx` row 1 | [x] |
| 36 | GEN, `pfcn = 14` | FIR arm, `firfx` row 2 | [x] |
| 37 | GEN, `pfcn = 15` | FIR arm, `firfx` row 3 | [x] |
| 38 | GEN, `pfcn = 12..15` | FIR arm with `firfx` all-zero taps (→ `pred = 0`) | [x] |
| 39 | GEN, `pfcn = 12..15` | FIR arm with `firfx` taps at `i16::MIN`/`i16::MAX` × `psamp` extreme (overflow of `32767 * i32::MAX`) | [x] |
| 40 | GEN, `pfcn = 12..15` | FIR arm: struct-layout check — distinct values in every one of the 4×8 `firfx` cells, plus non-zero `idx/lpred/rpred/tag/bcfcn/bsfcn/usefx`, verifying `#[repr(C)]` offsets match the C struct | [x] |
| 41 | SPEC 0..11 + GEN 0..15 | `idx` boundary sweep: `0..=8`, `-1..=-9`, `i32::MIN`, `i32::MIN+1`, `i32::MAX`, `i32::MAX-8`, `7`, `8` (wrap points of `(idx-k)&7`) | [x] |
| 42 | SPEC 0..11 | `pfcn` argument varied independently of `which` (the `PfnN` ignore `pfcn`; passing 0/5/99/-1 must not change the result) | [x] |
| 43 | GEN | `ridx` non-NULL but `pfcn` in `0..=11` (arm never touches `ridx`; garbage `firfx` must not affect result) | [x] |
| 44 | all of the above | Cargo feature set = `difftest` (the only feature; needed to expose the internal-predictor hook) | [x] |
| 45 | all of the above | Cargo feature set = `<none>` / `--no-default-features`: only `get_predict_func` reachable → rows 1-2 + all ERRORS rows re-run against the default-feature `.so` | [x] |
| 46 | n/a | Binary/driver executable: `c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` and no `add_executable`; `Cargo.toml` declares only `[lib]` and there is no `src/main.rs` / `src/bin/`. **No binary exists → no stdout comparison applicable.** | [x] |
