# ERRORS.md — Phase A error-surface table

Mechanically derived by grepping `c_src/src/lib.c` and `c_src/include/lib.h` for
every rejection mechanism:

```
grep -n 'return\|assert\|NULL\|if\|switch\|default\|case\|-1\|ERROR' c_src/src/lib.c
```

## Findings from the grep

* `assert` occurrences: **0**
* `NULL` occurrences: **0** (no null checks anywhere)
* `errno` / error enums / `RETURN_ERROR`-style macros: **0**
* `return -1` / negative sentinels: **0**
* explicit range checks (`if (x < lo || x > hi)`): **0**
* `#ifdef` / compile-time gates: **0**
* min/max constants: **0**

The library has **no error codes and no sentinel returns**. Its *entire*
rejection surface is expressed through the `default:` arms of its three
`switch (pfcn)` statements. Each `default:` arm is a distinct rejection path and
gets its own row below.

## The three `default:` arms (the only rejection sites)

| site | file:line | rejection behaviour |
|---|---|---|
| `BTAC1C2_PredictSample` | lib.c:98-100 | `pred = 0; break;` → returns `0` |
| `BTAC1C2_GetPredictFunc` | lib.c:222-224 | `fcn = (void *)BTAC1C2_PredictSample;` → falls back to the generic dispatcher |
| `get_predict_func` | lib.c:269-270 | `break;` with `result` still `0` → returns `0` |

## Error-surface table

`pfcn` is a plain `int` parameter, so *any* `int` is an accepted input across the
FFI boundary — there is no enum type and therefore no "invalid enum value" that
the C would trap. Out-of-range values are handled, not rejected, and the rows
below pin down exactly *how*.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|----------------------------------------------|-------------------|-----|
| E1 | `get_predict_func` | `pfcn == 12` — one step past the last handled case (`11`); hits `get_predict_func`'s `default:` | returns `0` | [x] |
| E2 | `get_predict_func` | `pfcn == -1` — one step below the first handled case (`0`) | returns `0` | [x] |
| E3 | `get_predict_func` | `pfcn` in `13..=15` — values `BTAC1C2_PredictSample` handles (firfx arms) but `BTAC1C2_GetPredictFunc` and `get_predict_func` do not | returns `0` for each | [x] |
| E4 | `get_predict_func` | `pfcn == 16` — one step past the last case of the *innermost* switch (`BTAC1C2_PredictSample`'s `case 15`) | returns `0` | [x] |
| E5 | `get_predict_func` | `pfcn == INT_MAX` (`2147483647`) — upper extreme of the parameter's range | returns `0` | [x] |
| E6 | `get_predict_func` | `pfcn == INT_MIN` (`-2147483648`) — lower extreme; also the value for which `pfcn - 12` overflows in `BTAC1C2_PredictSample` | returns `0` | [x] |
| E7 | `get_predict_func` | `pfcn` = every other out-of-range `int` (exhaustive `-4096..=4096` plus randomized full-`int` sweep) | returns `0` for every value outside `0..=11`, `1` for every value inside | [x] |
| E8 | `BTAC1C2_GetPredictFunc` (via `get_predict_func`) | `pfcn` outside `0..=11` → `default:` returns the *generic* `BTAC1C2_PredictSample` pointer rather than a specialised one; the caller's own `default:` then never compares it | observable as `get_predict_func(pfcn) == 0`; no crash, no dereference | [x] |
| E8b | `BTAC1C2_GetPredictFunc` (via `__difftest_selector`, `difftest` feature) | `pfcn` outside `0..=11` — the selector's `default:` arm observed **directly** rather than through the wrapper | returns the generic `BTAC1C2_PredictSample` pointer (hook index `12`), never a specialised one | [x] |
| E8c | `BTAC1C2_GetPredictFunc` (via `__difftest_selector`) | any `int` at all (20000 random full-range draws) | always one of the 13 known pointers; never an unrecognised value | [x] |
| E8d | `BTAC1C2_GetPredictFunc` + selected predictor (via `__difftest_call_selected`) | `pfcn` outside `0..=15` — the composed error path: selector falls back to generic, whose own `default:` returns 0 without dereferencing `ridx` | returns `0`; no crash | [x] |
| E9 | `BTAC1C2_PredictSample` (via `__difftest_predict`, `difftest` feature) | `pfcn` outside `0..=15` → `default:` arm; `psamp` is never read and `ridx` is never dereferenced | returns `0` | [x] |
| E10 | `BTAC1C2_PredictSample` (via `__difftest_predict`) | `pfcn == 16` and `pfcn == 11` boundaries around the `12..=15` firfx block — `11` must NOT index `firfx`, `16` must NOT index `firfx` | `11` → shift-by-3 formula; `16` → `0`; neither touches `ridx` | [x] |
| E11 | `BTAC1C2_PredictSample` (via `__difftest_predict`) | `pfcn == 12` with `ridx` pointing at an all-zero `btac1c_idxstate` — the *only* path that dereferences `ridx` | returns `0 / 256 == 0`; must not read out of the `firfx[4][8]` bounds | [x] |
| E12 | all predictors (via `__difftest_predict`) | `idx` negative / `idx == INT_MIN` / huge — `psamp[(idx - k) & 7]` must stay inside the 8-element array for every `idx` | index always in `0..=7`; no out-of-bounds read; value matches C | [x] |
| E13 | all predictors (via `__difftest_predict`) | `psamp` entries at `INT_MAX` / `INT_MIN` so the accumulator overflows signed `int` | C (gcc, two's complement, no `-ftrapv`) wraps; Rust must wrap identically | [x] |
| E14 | `BTAC1C2_PredictSample_Pfn*` (via `__difftest_predict`) | `pfcn` argument set to a value *inconsistent* with the selected specialised function (the C ignores the parameter entirely) | result depends only on `psamp`/`idx`; identical for every `pfcn` passed | [x] |
| E15 | `BTAC1C2_PredictSample_Pfn*` (via `__difftest_predict`) | `ridx == NULL` — the specialised predictors never dereference `ridx` | no crash; result matches the non-null case | [x] |

## Notes on generic FFI boundaries

* **Null pointers.** `get_predict_func` takes no pointers, so there is no null
  pointer to pass. For the internal predictors, `ridx == NULL` is safe for all of
  `Pfn0..Pfn11` and for `BTAC1C2_PredictSample` with `pfcn` outside `12..=15`
  (rows E15, E9). `psamp` is dereferenced unconditionally by every non-`default`
  path, so a null `psamp` is undefined behaviour in the C and is deliberately not
  tested.
* **Zero / oversized lengths.** There are no length or size parameters in the
  API, so this class does not apply.
* **Out-of-range enum values.** There is no `enum` in the source. `pfcn` is an
  `int`, and rows E1-E7 cover every out-of-range integer class (one past each
  boundary, both extremes, exhaustive small range, randomized full range).
