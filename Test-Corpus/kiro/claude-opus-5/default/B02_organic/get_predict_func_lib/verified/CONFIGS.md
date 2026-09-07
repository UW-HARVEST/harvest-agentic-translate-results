# CONFIGS.md — configuration surface table (valid inputs)

Derived mechanically from `c_src/include/lib.h` and `c_src/src/lib.c`.

## Axes the C actually branches on

Enumerated from the public header plus every `if` / `switch` / `#ifdef` in the
source:

- **Public entry points** (`nm -D` ∩ `lib.h`): exactly one —
  `int get_predict_func(int pfcn)`. There are no convenience-vs-low-level
  layers to choose between at the ABI boundary; the "low-level" functions
  (`BTAC1C2_GetPredictFunc`, `BTAC1C2_PredictSample`,
  `BTAC1C2_PredictSample_Pfn0..11`) are all `static`.
- **Runtime options / modes / flags**: none. There is no init call, no
  settable state, no global, no context object, no `#ifdef` in the file
  (`grep -c '#if' c_src/src/lib.c` → 0). The library is a pure function of its
  single argument.
- **Input shapes**: the only input is the scalar `int pfcn`. The shape axis is
  therefore the *value class* of `pfcn`, and the code distinguishes these
  classes:
  - each individual value `0..=11` — `BTAC1C2_GetPredictFunc` has a separate
    `case` per value, and `get_predict_func` has a separate pointer comparison
    per value, so each is its own distinct path (12 classes);
  - `12..=15` — outside `GetPredictFunc`'s domain but inside
    `BTAC1C2_PredictSample`'s (4 classes, covered as error rows);
  - everything else (covered as error rows).
- **Call-sequence shape**: the function is stateless, so repeated / interleaved
  calls are an axis only insofar as a buggy translation might cache. Covered by
  row 13.
- Byte order, element widths, buffer sizes, counts, empty/one/many: **not
  applicable** — no buffers, no arrays, no lengths cross the ABI boundary.

Because there are no option flags, the cross-product collapses to
`{single entry point} × {value class of pfcn}`, plus the call-sequence row.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `get_predict_func` | `pfcn = 0` → `BTAC1C2_PredictSample_Pfn0` selected; expect `1` | [x] |
| 2 | `get_predict_func` | `pfcn = 1` → `_Pfn1`; expect `1` | [x] |
| 3 | `get_predict_func` | `pfcn = 2` → `_Pfn2`; expect `1` | [x] |
| 4 | `get_predict_func` | `pfcn = 3` → `_Pfn3`; expect `1` | [x] |
| 5 | `get_predict_func` | `pfcn = 4` → `_Pfn4`; expect `1` | [x] |
| 6 | `get_predict_func` | `pfcn = 5` → `_Pfn5`; expect `1` | [x] |
| 7 | `get_predict_func` | `pfcn = 6` → `_Pfn6`; expect `1` | [x] |
| 8 | `get_predict_func` | `pfcn = 7` → `_Pfn7`; expect `1` | [x] |
| 9 | `get_predict_func` | `pfcn = 8` → `_Pfn8`; expect `1` | [x] |
| 10 | `get_predict_func` | `pfcn = 9` → `_Pfn9`; expect `1` | [x] |
| 11 | `get_predict_func` | `pfcn = 10` → `_Pfn10`; expect `1`. Note `_Pfn10` body (`>> 3`) disagrees with `PredictSample` case 10 (`>> 4`); identity selection is what is observable here | [x] |
| 12 | `get_predict_func` | `pfcn = 11` → `_Pfn11`; expect `1`. Note `_Pfn11` body (`>> 1`) disagrees with `PredictSample` case 11 (`>> 3`) | [x] |
| 13 | `get_predict_func` | full valid domain `0..=11` driven end-to-end as a real consumer does: each value called repeatedly, in ascending, descending, and pseudo-random interleaved order (fixed seed), asserting statelessness — the same value must give the same answer regardless of what preceded it | [x] |
| 14 | `get_predict_func` | randomized property sweep over the *whole* `i32` domain (fixed-seed LCG, 200k draws) biased to include the valid domain, boundaries, and masking aliases: C and Rust return values compared for every draw | [x] |
| 15 | `get_predict_func` | pointer-identity invariant implied by the C: for the valid domain the return value is `1` for *every* value, i.e. `BTAC1C2_GetPredictFunc` must return the *matching* specialised predictor and never the shared `BTAC1C2_PredictSample` fallback. A Rust translation that collapsed distinct predictors onto one address, or that used a per-call trampoline/shim address, would break this. Asserted for all 12 values against C | [x] |

## Non-rows (recorded so the pruning is auditable)

- No `#ifdef` / build-time configuration in the C, so there is no compile-time
  axis to cross with the above.
- `c_src/CMakeLists.txt` builds a `SHARED` library only — **no binary
  executable / driver**, so the "compare C and Rust stdout" gate is not
  applicable (nothing to run). `translation/Cargo.toml` likewise declares only
  `crate-type = ["cdylib"]` and no `[[bin]]`.
- `translation/Cargo.toml` has **no `[features]` section**, so the only feature
  combination is the default (empty) one. `--no-default-features` and the
  default build are therefore the same code; both are exercised.
