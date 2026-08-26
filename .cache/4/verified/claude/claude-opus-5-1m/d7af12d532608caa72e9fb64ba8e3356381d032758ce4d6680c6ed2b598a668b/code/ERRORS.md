# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. Greps performed:

```
grep -c 'assert'     c_src/src/lib.c   -> 0
grep -c 'NULL'       c_src/src/lib.c   -> 0
grep -c 'if'         c_src/src/lib.c   -> 0
grep -c 'default:'   c_src/src/lib.c   -> 3
grep -c 'return'     c_src/src/lib.c   -> 17
```

The library has **no error enum, no `RETURN_ERROR` macro, no `errno`, no
sentinel-returning allocation, no null checks and no assertions**. Its entire
rejection surface consists of the three `switch` `default:` arms plus the
implicit domain restrictions of the `switch` labels and of the `firfx[pfcn-12]`
indexing. Every one of those is a row below.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| E1 | `call_predict` (lib.c:229, `default:` at :269) | `pfcn` outside `0..=11`, generic case: e.g. `pfcn = 12` (first value past the valid dispatch range) | `result` keeps its initialiser `0` → returns `0` (no crash, `fcn` still computed) |
| E2 | `call_predict` | `pfcn = -1` (one step *below* the valid range) | returns `0` |
| E3 | `call_predict` | `pfcn = 12`, i.e. one step *above* the valid range and the first `BTAC1C2_PredictSample`-only arm | returns `0` |
| E4 | `call_predict` | `pfcn = 13, 14, 15` — valid arms of `BTAC1C2_PredictSample` but *invalid* for `BTAC1C2_GetPredictFunc`/`call_predict` | returns `0` |
| E5 | `call_predict` | `pfcn = 16` (first value past *all* `BTAC1C2_PredictSample` labels) | returns `0` |
| E6 | `call_predict` | `pfcn = INT_MIN` (`-2147483648`) — extreme out-of-range enum-like value across the FFI boundary | returns `0` |
| E7 | `call_predict` | `pfcn = INT_MAX` (`2147483647`) — extreme out-of-range value | returns `0` |
| E8 | `call_predict` | `pfcn` = arbitrary out-of-range value that would alias a valid case if truncated to 8/16 bits (e.g. `256`, `65536`, `0x10000 + 3`, `-4294967293` truncated) — C compares the *full* `int` | returns `0` |
| E9 | `BTAC1C2_GetPredictFunc` (lib.c:183, `default:` at :222) | `pfcn` outside `0..=11` | returns the address of the *generic* `BTAC1C2_PredictSample` (never `NULL`); observable via `aux_getpredict_call`, which then evaluates the generic dispatcher with the same `pfcn` |
| E10 | `BTAC1C2_PredictSample` (lib.c:18, `default:` at :98) | `pfcn` outside `0..=15` (e.g. `16`, `-1`, `INT_MIN`, `INT_MAX`) | `pred = 0` → returns `0`, regardless of `psamp` / `idx` / `ridx` contents |
| E11 | `BTAC1C2_PredictSample` | `pfcn` in `12..=15` — the *only* input-dependent indexing in the file: `ridx->firfx[pfcn - 12][0..7]`, valid because `firfx` is `[4][8]`; `pfcn = 11` and `pfcn = 16` must NOT reach it (would be index `-1` / `4`, out of bounds) | `pfcn=12..15` read `firfx[0..3]`; `pfcn=11`/`pfcn=16` take the `case 11` / `default` arms instead, so no OOB read ever happens |
| E12 | `BTAC1C2_PredictSample`, `_Pfn0..._Pfn11` | `idx` arbitrary (incl. negative and `INT_MAX`): every access is `psamp[(idx - n) & 7]`, so the mask *is* the range check — index is always `0..=7` for a `psamp` of 8 elements; there is no other bound check and no rejection | never rejects; always reads inside `psamp[0..8]` (two's-complement `&7` of a negative value is still `0..=7`) |
| E13 | `call_predict` | `pfcn` valid (`0..=11`) but the function-pointer identity comparison fails (e.g. if two predictors were address-merged by the linker) | returns `0` — the C compiler keeps the 12 predictors distinct, so all of `0..=11` must return `1`; a Rust build that folds two of them would show up here |
| E14 | (whole library) | NULL `psamp` / NULL `ridx` passed to the *exported* API | **not reachable**: `call_predict(int)` takes no pointers, so the only exported entry point cannot be handed a null pointer. Verified by the header/`nm -D`: the exported surface is a single `int(int)`. The `static` predictors do dereference their pointers unconditionally (no null check — rows above), which is exercised with valid buffers only, since a null dereference is UB in both languages and not a defined C behaviour to match. |

## Result of the differential error-path tests

Every row has a passing differential test in `tests/differential.rs`. Both
libraries are loaded with `libloading` and called only through `dlsym`; each
test asserts the two return the **same** value *and* that the C value is the
specific sentinel the table predicts (not merely "both failed somehow").

| row | test | status |
|-----|------|--------|
| E1 | `err_call_predict_out_of_range` (asserts C returns exactly `0`) | [x] |
| E2 | `err_call_predict_out_of_range` (`pfcn = -1`) | [x] |
| E3 | `err_call_predict_out_of_range`, `cfg02_call_predict_predictsample_only_arms` (`pfcn = 12`) | [x] |
| E4 | `cfg02_call_predict_predictsample_only_arms` (`pfcn = 13,14,15`) | [x] |
| E5 | `err_call_predict_out_of_range` (`pfcn = 16`) | [x] |
| E6 | `err_call_predict_out_of_range` (`INT_MIN`, `INT_MIN+1`) | [x] |
| E7 | `err_call_predict_out_of_range` (`INT_MAX`, `INT_MAX-1`) | [x] |
| E8 | `err_call_predict_out_of_range` (`127,128,255,256,257,32767,32768,65535,65536,65539,0x01000000`) | [x] |
| E9 | `err_dispatch_default_arm` (`aux_getpredict_is_null` == 0 in both; call-through compared) | [x] |
| E10 | `err_generic_predict_default_arm` (asserts C yields exactly `0`) | [x] |
| E11 | `err_firfx_index_boundary` (row-dependent coefficients + poisoned neighbouring fields; `pfcn` 10..17) | [x] |
| E12 | `err_idx_mask_is_the_bound_check` (unique powers-of-3 buffer, `idx` −40..40, `INT_MIN+8`, `INT_MAX`, 256 randoms) | [x] |
| E13 | `cfg01_call_predict_valid_range` (asserts the C library itself returns `1` for all of `0..=11`, i.e. the 12 predictors really are at distinct addresses in *both* libraries) | [x] |
| E14 | `cfg42_exported_surface_is_int_int` (`nm -D` on both `.so`s yields exactly `["call_predict"]`, so no exported entry point can be given a pointer) | [x] |

Additionally, `err_call_predict_exhaustive_every_i32` and
`err_generic_predict_exhaustive_every_pfcn` (`#[ignore]`d; run with
`cargo test --release -- --ignored`) compare **all 2^32 `int` values** for the
exported entry point and for the generic predictor's arm selection, so the
"one step past the valid range" and "out-of-range enum value across FFI"
classes are covered without sampling at all.
