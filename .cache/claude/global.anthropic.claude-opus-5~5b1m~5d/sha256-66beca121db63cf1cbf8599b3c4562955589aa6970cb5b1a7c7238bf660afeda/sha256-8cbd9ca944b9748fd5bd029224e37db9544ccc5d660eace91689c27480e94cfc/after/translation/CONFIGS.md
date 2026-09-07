# CONFIGS.md — Phase B configuration surface

## Axes the C code actually branches on

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

* **Public entry points (exported ABI):** exactly one — `int call_predict(int pfcn)`.
  (`get_predict_func` is declared in `lib.h` but never defined; not exported.)
* **Runtime options / modes / flags:** none. There is no global state, no
  initialisation function, no option setter, no `#ifdef`, no environment
  variable and no byte-order handling in the C source.
  (`grep -c '#if' c_src/src/lib.c` → 0.)
* **Only input axis:** the `int pfcn` selector.
* **Branch points on that axis:**
  * `BTAC1C2_GetPredictFunc`: `case 0 … case 11` + `default` → 13 distinct arms.
  * `call_predict`'s own `switch`: `case 0 … case 11` + `default` → 13 arms.
  * `BTAC1C2_PredictSample`'s `switch`: `case 0 … 11`, `case 12|13|14|15`
    (shared arm), `default` → 14 arms. Reached only as an *address* from
    `GetPredictFunc`'s `default`, never called by `call_predict`.
* **Input shapes:** `pfcn` is a scalar `int`. The distinguished shapes are
  therefore: each in-range selector value, the shared 12..15 band, negative
  values, and the two `i32` extremes.
* **Cargo feature axes:** the C source has no `#ifdef` at all, so there is no
  compile-time axis to mirror. `translation/Cargo.toml` declares `default = []`
  plus one **test-only** feature, `internal_probe` (see the extension rows at the
  end of this file), which adds probe exports and changes no behaviour. The build
  configurations are therefore `{default, --no-default-features, internal_probe}`
  x `{debug, release}` = 6, and all 6 are executed.

## Configuration rows

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 1 | `call_predict` | `pfcn = 0` → dispatches `_Pfn0`, self-compare arm 0 | [x] |
| 2 | `call_predict` | `pfcn = 1` → `_Pfn1` | [x] |
| 3 | `call_predict` | `pfcn = 2` → `_Pfn2` | [x] |
| 4 | `call_predict` | `pfcn = 3` → `_Pfn3` | [x] |
| 5 | `call_predict` | `pfcn = 4` → `_Pfn4` | [x] |
| 6 | `call_predict` | `pfcn = 5` → `_Pfn5` | [x] |
| 7 | `call_predict` | `pfcn = 6` → `_Pfn6` | [x] |
| 8 | `call_predict` | `pfcn = 7` → `_Pfn7` | [x] |
| 9 | `call_predict` | `pfcn = 8` → `_Pfn8` | [x] |
| 10 | `call_predict` | `pfcn = 9` → `_Pfn9` | [x] |
| 11 | `call_predict` | `pfcn = 10` → `_Pfn10` | [x] |
| 12 | `call_predict` | `pfcn = 11` → `_Pfn11` (last in-range selector) | [x] |
| 13 | `call_predict` | `pfcn` in `12..=15` → the shared `12|13|14|15` FIR band of `BTAC1C2_PredictSample`, selected via `GetPredictFunc`'s `default` | [x] |
| 14 | `call_predict` | `pfcn` = exhaustive sweep `-1000..=1000` (covers every boundary crossing: `-1/0`, `11/12`, `15/16`) | [x] |
| 15 | `call_predict` | `pfcn` = `i32::MIN`, `i32::MIN+1`, `-1`, `0`, `11`, `12`, `15`, `16`, `i32::MAX-1`, `i32::MAX` (boundary set) | [x] |
| 16 | `call_predict` | `pfcn` = 20 000 randomized `i32` values, fixed seed (SplitMix64, seed `0x5EED_1234_ABCD_9876`), full `i32` range | [x] |
| 17 | `call_predict` | `pfcn` = 20 000 randomized values biased into `-32..=47` (near-range random, same fixed seed stream) so in-range hits are dense | [x] |
| 18 | `call_predict` | repeated / interleaved invocation order (each value called twice, and the whole sweep re-run in reverse) — proves there is no hidden per-call state in either library | [x] |
| 19 | `call_predict` | called through a *re-loaded* `.so` handle (dlopen/dlclose then dlopen again) — confirms no initialisation-order dependence | [x] |
| 20 | build config | default features (= `--no-default-features`, since `default = []`) in both `debug` and `release` profiles; the two Rust cdylibs are also compared against each other. See the extension rows below for the test-only `internal_probe` feature. | [x] |

## Binary executable

`c_src/CMakeLists.txt` builds only `add_library(... SHARED ...)`; there is no
`add_executable`, and `translation/Cargo.toml` declares only `[lib]` with
`crate-type = ["cdylib"]`. **No driver binary exists**, so the "compare stdout
of C and Rust binaries" gate is not applicable.

---

## Extension rows — the `static` (internal) functions

`call_predict` only ever *compares addresses*; it never calls a predictor. So
the arithmetic of the 14 `static` C helpers is **not observable** through the
exported ABI and was initially untested. To close that gap without touching
`c_src/`, `translation/tests/cprobe/probe.c` `#include`s the unmodified
`c_src/src/lib.c` and re-exports the statics as `probe_*`; the Rust crate grows
matching `probe_*` exports behind the test-only `internal_probe` feature
(which only ADDS symbols, so parity is unaffected). Both are then loaded with
`libloading` and compared function by function.

Input magnitudes are bounded (`|sample| <= 2047`, `|firfx| <= 255`) so no C
intermediate can signed-overflow — that would be UB and could legitimately
differ from Rust.

Run with: `cargo test --offline --features internal_probe`

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| 21 | `btac1c_idxstate` layout | `size_of` / `align_of` / `offset_of(firfx)` reported by both libraries must be equal (else the FIR arm reads wrong coefficients) | [x] |
| 22 | `_Pfn0` .. `_Pfn11` (12 fns) | 400 random 8-sample windows x 45 `idx` values each (`-20..=20` plus `i32::MIN+9`, `-1000`, `1000`, `i32::MAX`) — covers every `(idx-k)&7` residue and negative/extreme `idx` | [x] |
| 23 | `BTAC1C2_PredictSample` (switch) | 2000 random windows x `pfcn` in `-4..=20` plus `i32::MIN`, `-1000`, `1000`, `i32::MAX` — every arm: `0..=11`, the `12\|13\|14\|15` band, and `default` | [x] |
| 24 | `BTAC1C2_PredictSample`, FIR band | `pfcn` 12,13,14,15 with four *distinct* `firfx` rows, 2000 random windows — proves row `pfcn - 12` is selected | [x] |
| 25 | `BTAC1C2_PredictSample` + all `_PfnN` | all-negative / all-positive / mixed-sign windows, plus small-magnitude (`-9..=9`) windows, 3000 iterations x all 16 `pfcn` — pins C's arithmetic `>>` on negatives and `/` truncating toward zero | [x] |
| 26 | `BTAC1C2_GetPredictFunc` | dispatch decision as an index over `pfcn` in `-2000..=2000` plus the `i32` extremes: `0..=11` -> that index, everything else -> `-1` (`BTAC1C2_PredictSample`) | [x] |
| 27 | `_Pfn10`/`_Pfn11` vs switch arms 10/11 | guards the C source's deliberate asymmetry (`_Pfn10` uses `>>3` where arm 10 uses `>>4`; `_Pfn11` uses `>>1` where arm 11 uses `>>3`); the *relationship* must match in both libraries, and the test asserts the difference is actually observed, so a translation that "fixed" the C would fail | [x] |
| 28 | build config | all rows above re-run under both `debug` and `release`, x `{default, --no-default-features, internal_probe}` — 6 combinations total, driven by `run_all_combos.sh` | [x] |

## Feature/profile matrix actually executed

`run_all_combos.sh` enumerates the feature power set from `Cargo.toml`
automatically and runs every combination in both profiles:

| profile | features | result |
|---------|----------|--------|
| debug | (default) | PASS — 37 tests |
| debug | `--no-default-features` | PASS — 37 tests |
| debug | `internal_probe` | PASS — 44 tests |
| release | (default) | PASS — 37 tests |
| release | `--no-default-features` | PASS — 37 tests |
| release | `internal_probe` | PASS — 44 tests |

Symbol parity (C-only symbols missing from Rust): **0** in both profiles.
