# CONFIGS.md — Configuration-surface table (Phase A, gates Phase B)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

There are **no** runtime options, no mode/flag setters, no global state, and no
`#ifdef` (grep for `^\s*#` in `lib.c` returns only `#include "lib.h"`). The
library is pure functions, so the configuration surface is entirely *input
shape*:

| axis | values the C distinguishes | where |
|------|---------------------------|-------|
| **A1** `pfcn` selector | `0,1,…,11` are separate `case`s in all three `switch`es; `12,13,14,15` are separate `case`s in `BTAC1C2_PredictSample` **only**; everything else hits `default:` | lines 22–101, 185–225, 232–271 |
| **A2** entry point | `get_predict_func` (public); `BTAC1C2_GetPredictFunc`; `BTAC1C2_PredictSample`; `BTAC1C2_PredictSample_Pfn0..11` (12 fns) — 15 routines total | lines 18–229 |
| **A3** `idx` | used only as `(idx - n) & 7`, so: in-range `0..=7`, negative (mask of a negative int), and values where `idx - n` overflows (`INT_MIN`) | `s()`/`psamp[(i-n)&7]` everywhere |
| **A4** `psamp[]` contents | zero; all-positive; **all-negative** (`>>` is an *arithmetic* shift, and `/16`,`/64`,`/256` truncate toward **zero** — the two round differently on negatives); magnitudes small enough that no intermediate overflows; magnitudes large enough that `72*x` etc. **do** overflow; numerators not divisible by the divisor (remainder-sign behaviour) | lines 24–96 |
| **A5** `ridx->firfx[pfcn-12][0..7]` | only read for `pfcn` 12–15; `s16` operands promoted to `int`: zero, positive, negative, `INT16_MIN`/`INT16_MAX` | lines 88–96 |
| **A6** `firfx` row | `pfcn-12` selects row `0`,`1`,`2`,`3` | line 88 |
| **A7** `ridx` pointer | **never dereferenced** unless `pfcn` is 12–15, so `NULL` is a legal input for every other `pfcn` and for all 12 `_Pfn*` routines | lines 22–101, 105–181 |
| **A8** `pfcn` argument to `_Pfn*` | ignored by all 12 `_Pfn*` routines (they take it but never read it) | lines 105–181 |

`get_predict_func` is the only *exported* entry point, but it is a thin
convenience wrapper whose result is a pointer-identity check. The 14 low-level
routines hold every arithmetic branch in the library, so testing only the
wrapper would verify essentially nothing. They are `static`, so both sides are
driven through name-identical shims — `tests/cshim/cshim.c` (which `#include`s
the untouched `c_src/src/lib.c`) and the Rust crate's off-by-default
`diff_internals` feature — and every call still crosses the FFI boundary via
`dlopen`/`dlsym`.

Randomization: deterministic SplitMix64, fixed seed `0x5DEECE66D_9E3779B9`,
**512 iterations per row** unless noted; each iteration redraws the data tier,
`idx` variant and `firfx` contents for that row's axes.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_predict_func` | `pfcn = 0` | [x] |
| 2 | `get_predict_func` | `pfcn = 1` | [x] |
| 3 | `get_predict_func` | `pfcn = 2` | [x] |
| 4 | `get_predict_func` | `pfcn = 3` | [x] |
| 5 | `get_predict_func` | `pfcn = 4` | [x] |
| 6 | `get_predict_func` | `pfcn = 5` | [x] |
| 7 | `get_predict_func` | `pfcn = 6` | [x] |
| 8 | `get_predict_func` | `pfcn = 7` | [x] |
| 9 | `get_predict_func` | `pfcn = 8` | [x] |
| 10 | `get_predict_func` | `pfcn = 9` | [x] |
| 11 | `get_predict_func` | `pfcn = 10` | [x] |
| 12 | `get_predict_func` | `pfcn = 11` | [x] |
| 13 | `get_predict_func` | exhaustive sweep `pfcn ∈ -4096..=4096` (covers every valid arm, both boundaries, and the whole near-range `default:` band) | [x] |
| 14 | `get_predict_func` | randomized `pfcn` over the **full `i32`** range, 100 000 draws | [x] |
| 15 | `BTAC1C2_GetPredictFunc` | `pfcn ∈ 0..=11` → dispatch table must select `_Pfn<pfcn>`; asserts the *identity*, not just a bool | [x] |
| 16 | `BTAC1C2_GetPredictFunc` | `pfcn ∈ 12..=15` (the `PredictSample`-only cases) → `default:` → `BTAC1C2_PredictSample` | [x] |
| 17 | `BTAC1C2_GetPredictFunc` | `pfcn ∈ {-1, -2, 16, 17, INT_MIN, INT_MIN+1, INT_MAX-1, INT_MAX}` → `BTAC1C2_PredictSample` | [x] |
| 18 | `BTAC1C2_GetPredictFunc` | randomized `pfcn` over the full `i32` range | [x] |
| 19 | `struct btac1c_idxstate_s` | ABI layout: `sizeof`, `alignof`, and `offsetof` of all 8 members must match, plus out-of-range probe → `-1` | [x] |
| 20 | `BTAC1C2_PredictSample` | `pfcn = 0`, randomized over A3 × A4 (`ridx = NULL`) | [x] |
| 21 | `BTAC1C2_PredictSample` | `pfcn = 1`, randomized over A3 × A4 | [x] |
| 22 | `BTAC1C2_PredictSample` | `pfcn = 2`, randomized over A3 × A4 (`>>1` on possibly-negative) | [x] |
| 23 | `BTAC1C2_PredictSample` | `pfcn = 3`, randomized over A3 × A4 (`>>2`) | [x] |
| 24 | `BTAC1C2_PredictSample` | `pfcn = 4`, randomized over A3 × A4 (`p0 - (p1>>1)`) | [x] |
| 25 | `BTAC1C2_PredictSample` | `pfcn = 5`, randomized over A3 × A4 (`(3*p0-p1)>>2`) | [x] |
| 26 | `BTAC1C2_PredictSample` | `pfcn = 6`, randomized over A3 × A4 (`(5*p0-p1)>>3`) | [x] |
| 27 | `BTAC1C2_PredictSample` | `pfcn = 7`, randomized over A3 × A4 (5-tap, `/16` truncating) | [x] |
| 28 | `BTAC1C2_PredictSample` | `pfcn = 8`, randomized over A3 × A4 (8-tap, `/64`) | [x] |
| 29 | `BTAC1C2_PredictSample` | `pfcn = 9`, randomized over A3 × A4 (8-tap, `/64`, different taps) | [x] |
| 30 | `BTAC1C2_PredictSample` | `pfcn = 10`, randomized over A3 × A4 (`(5*p0-p1)>>4` — note shift **4** here) | [x] |
| 31 | `BTAC1C2_PredictSample` | `pfcn = 11`, randomized over A3 × A4 (`(p0+p1)>>3` — note shift **3** here) | [x] |
| 32 | `BTAC1C2_PredictSample` | `pfcn = 12` → `firfx` row 0, randomized `firfx` (A5) × A3 × A4 | [x] |
| 33 | `BTAC1C2_PredictSample` | `pfcn = 13` → `firfx` row 1, randomized | [x] |
| 34 | `BTAC1C2_PredictSample` | `pfcn = 14` → `firfx` row 2, randomized | [x] |
| 35 | `BTAC1C2_PredictSample` | `pfcn = 15` → `firfx` row 3, randomized | [x] |
| 36 | `BTAC1C2_PredictSample` | `pfcn ∈ 12..=15` with `firfx` **all zero** (degenerate: `0/256`) | [x] |
| 37 | `BTAC1C2_PredictSample` | `pfcn ∈ 12..=15` with `firfx` saturated to `INT16_MIN`/`INT16_MAX` (max-magnitude taps → overflow of the 8-term sum) | [x] |
| 38 | `BTAC1C2_PredictSample` | `pfcn ∈ 12..=15`, all four rows populated with *different* values — proves the correct row is selected | [x] |
| 39 | `BTAC1C2_PredictSample` | `default:` arm — `pfcn ∈ {16, 17, -1, INT_MIN, INT_MAX}` → `0`, with non-NULL `ridx` | [x] |
| 40 | `BTAC1C2_PredictSample` | `pfcn ∈ 0..=11` with `ridx = NULL` (A7: legal, never dereferenced) | [x] |
| 41 | `_Pfn0` | randomized over A3 × A4, `ridx = NULL`, `pfcn` arg varied (A8) | [x] |
| 42 | `_Pfn1` | randomized over A3 × A4 | [x] |
| 43 | `_Pfn2` | randomized over A3 × A4 | [x] |
| 44 | `_Pfn3` | randomized over A3 × A4 | [x] |
| 45 | `_Pfn4` | randomized over A3 × A4 | [x] |
| 46 | `_Pfn5` | randomized over A3 × A4 | [x] |
| 47 | `_Pfn6` | randomized over A3 × A4 | [x] |
| 48 | `_Pfn7` | randomized over A3 × A4 (`/16`) | [x] |
| 49 | `_Pfn8` | randomized over A3 × A4 (`/64`) | [x] |
| 50 | `_Pfn9` | randomized over A3 × A4 (`/64`) | [x] |
| 51 | `_Pfn10` | randomized over A3 × A4 (`(5*p0-p1)>>3` — shift **3**, unlike `case 10:`) | [x] |
| 52 | `_Pfn11` | randomized over A3 × A4 (`(p0+p1)>>1` — shift **1**, unlike `case 11:`) | [x] |
| 53 | `_Pfn0..11` | `pfcn` argument swept over `{0..15, -1, INT_MIN, INT_MAX}` while `which` is fixed — must be ignored identically (A8) | [x] |
| 54 | `_Pfn10` vs `PredictSample(10)` | same inputs → the two must **disagree** (shift 3 vs 4), and C and Rust must disagree in the *same way* | [x] |
| 55 | `_Pfn11` vs `PredictSample(11)` | same inputs → must disagree (shift 1 vs 3), identically on both sides | [x] |
| 56 | `PredictSample` + `_Pfn*` | `idx ∈ 0..=7` (in-range, no masking needed) across all `pfcn` | [x] |
| 57 | `PredictSample` + `_Pfn*` | `idx ∈ -1..=-16` (mask of a negative `int`) across all `pfcn` | [x] |
| 58 | `PredictSample` + `_Pfn*` | `idx ∈ {INT_MIN..INT_MIN+8, INT_MAX-8..INT_MAX}` (`idx - n` overflows) across all `pfcn` | [x] |
| 59 | `PredictSample` + `_Pfn*` | `psamp` all zero (degenerate) across all `pfcn` | [x] |
| 60 | `PredictSample` + `_Pfn*` | `psamp` **all negative** across all `pfcn` — separates arithmetic `>>` from truncating `/` | [x] |
| 61 | `PredictSample` + `_Pfn*` | `psamp` chosen so numerators are negative and **not** divisible by 16/64/256 (remainder sign) | [x] |
| 62 | `PredictSample` + `_Pfn*` | `psamp` at full-`i32` extremes (`INT_MIN`/`INT_MAX`/`±1`) — wrapping overflow of `72*x`, `5*p0`, etc. | [x] |
| 63 | `PredictSample` + `_Pfn*` | `psamp` "safe-large" tier `|v| ≤ 2^20` — no intermediate overflow, so the row is meaningful independent of overflow semantics | [x] |

## Feature combinations (Phase D)

`Cargo.toml` declares one non-default, test-only feature. Both combinations are
run:

| combo | `cargo test --no-default-features --features …` | rows covered | result |
|-------|------------------------------------------------|--------------|--------|
| *(none)* | `--no-default-features` | 1–14 (the public ABI is the only surface that exists in this build); 15–63 are `#[cfg]`-skipped | 22 tests, 0 failed |
| `diff_internals` | `--no-default-features --features diff_internals` | **all** rows 1–63 | 72 tests, 0 failed |

Run both with `./verify_all.sh`, which enumerates the feature power set out of
`Cargo.toml` and then does `cargo check --all-targets`, `cargo test`, and the
`nm -D` symbol diff for each combination.

## Test-sensitivity evidence (mutation testing)

A suite that passes is only meaningful if it can fail. 20 mutations were
injected into `src/lib.rs` one at a time, rebuilding and re-running the whole
suite each time; `src/lib.rs` was restored byte-identically afterwards
(verified with `diff`).

| mutation | caught by |
|----------|-----------|
| `_Pfn11` shift `>>1` → `>>2` | rows 52, 53, 55, 56 |
| `case 7` `/16` → `>>4` (truncate-toward-zero vs floor) | rows 27, 40, 56, 57 |
| `case 8` `/64` → `>>6` | rows 28, 40, 56, 57 |
| FIR `/256` → `>>8` | rows 32–35 |
| FIR row index `pfcn-12` → `0` | rows 33–35, 37 |
| `case 2` `>>1` → `>>2` | rows 22, 40, 56, 57 |
| `case 10` `>>4` → `>>3` | rows 30, 40, 54, 56 |
| `case 11` `>>3` → `>>1` | rows 31, 40, 55, 56 |
| index mask `&7` → `&15` | rows 20–23, … |
| dispatch `3 => _Pfn3` → `_Pfn4` | row 15, row 4, row 13 |
| `case 9` tap `17` → `16` | rows 29, 40, 56, 57 |
| extra struct field (ABI/layout change) | row 19, rows 32–34 |
| `get_predict_func`: `pfcn==11` → always 0 | rows 12, 13, extremes |
| `get_predict_func`: also accept `pfcn==12` | rows 13, 14, extremes |
| `GetPredictFunc` `default:` → `_Pfn0` | rows 16, 17, 18 |
| `PredictSample` `default:` → `1` | ERRORS row 1; rows 40, 56 |
| `case 0` tap `(i-1)` → `(i-2)` | rows 20, 40, 56 |
| arithmetic `>>` → logical `>>` (via `u32`) | rows 22, 40, 56 |
| FIR `i16` → `u16` promotion | rows 32–34 |

**19 / 19 genuine mutations were caught.** A 20th (`11 =>` rewritten to the
catch-all binding pattern `_x11 =>`) was correctly *not* flagged, because it is
an equivalent mutant: the arm still matches 11, and for every other value the
`fcn == _Pfn11` comparison is false — exactly what the original `_ => {}` arm
already produced.
