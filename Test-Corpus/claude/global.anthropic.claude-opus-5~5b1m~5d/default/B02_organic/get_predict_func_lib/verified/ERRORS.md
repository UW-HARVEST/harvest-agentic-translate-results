# ERRORS.md — Phase A: error / rejection surface table

Derived mechanically from `c_src/src/lib.c` (273 lines) and
`c_src/include/lib.h` (1 line).

## Mechanical grep results

```
$ grep -n 'RETURN_ERROR\|return -1\|return NULL\|assert\|errno\|EINVAL\|exit(\|abort(' c_src/src/lib.c
  (no matches)
$ grep -cn 'default:' c_src/src/lib.c
  3      # lines 98, 222, 269
$ grep -n 'if\s*(' c_src/src/lib.c
  (no matches — the file contains no if statements at all)
$ grep -n '#define\|#if' c_src/src/lib.c
  (no matches beyond `#include "lib.h"`)
```

The translation unit contains **no error-return macros, no `assert`, no
`errno`, no explicit range checks, no null checks and no min/max constants.**
Rejection of invalid input is expressed *exclusively* through `switch` /
`default:` fall-through, which yields a sentinel value rather than an error
code.  Every such sentinel path is a row below.

## Error-surface table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|---------------------------------------------|-------------------|
| 1 | `get_predict_func` (public) | `pfcn == 12` — first value past the valid `0..=11` specialised-predictor range. `BTAC1C2_GetPredictFunc` takes its `default:` arm (line 222) and `get_predict_func`'s own `switch` takes its `default:` arm (line 269), leaving `result` at its initialiser. | returns `0` |
| 2 | `get_predict_func` (public) | `pfcn == 13`, `14`, `15` — remaining values that `BTAC1C2_PredictSample` recognises internally but that have no specialised `PfnN`. | returns `0` |
| 3 | `get_predict_func` (public) | `pfcn == 16` — one past the largest value any `switch` in the file mentions. | returns `0` |
| 4 | `get_predict_func` (public) | `pfcn == -1` — negative, i.e. one step below the valid range. C `switch` on `int` matches no `case`, so both `default:` arms run. | returns `0` |
| 5 | `get_predict_func` (public) | `pfcn == INT_MIN` (`-2147483648`) — extreme negative out-of-range enum-like value crossing the FFI boundary. | returns `0` |
| 6 | `get_predict_func` (public) | `pfcn == INT_MAX` (`2147483647`) — extreme positive out-of-range value. | returns `0` |
| 7 | `get_predict_func` (public) | any arbitrary out-of-range `int` (property-tested over the full `i32` domain, excluding `0..=11`). C enums accept any `int`; there is no validation, so every such value must fall through both `default:` arms. | returns `0` |
| 8 | `BTAC1C2_GetPredictFunc` (internal, reached via `__difftest_predict` with `which` outside `0..=11`) | `pfcn` outside `0..=11` → returns the *generic* `BTAC1C2_PredictSample` instead of a specialised predictor. This is the "rejection" of an unknown predictor id: a fallback pointer, never `NULL`. | returns non-NULL pointer to `BTAC1C2_PredictSample`; consequently `get_predict_func` reports `0` |
| 9 | `BTAC1C2_PredictSample` (internal, via `__difftest_predict(which = out-of-range, …, pfcn = 16)`) | `pfcn == 16` — one past the largest handled `case` (`case 15`, line 87). `default:` arm at line 98 executes. | `pred = 0`, function returns `0` |
| 10 | `BTAC1C2_PredictSample` (internal, via `__difftest_predict`) | `pfcn == -1` / `INT_MIN` / `INT_MAX` / any value outside `0..=15`. | returns `0` |
| 11 | `BTAC1C2_PredictSample` (internal, via `__difftest_predict`) | `pfcn` in `12..=15` **with `ridx == NULL`** — the code dereferences `ridx->firfx[pfcn - 12][k]` with no null check. Undefined behaviour in C (segfault in practice). | NOT differentially tested for a return value; the table records that neither side validates. Test asserts *both* implementations dereference (i.e. the Rust performs no extra validation and no panic-based rejection is introduced). Exercised only with a valid non-NULL `ridx`. |
| 12 | `BTAC1C2_PredictSample` / all `PfnN` (internal, via `__difftest_predict`) | `psamp == NULL` — dereferenced unconditionally, no null check anywhere in the file. UB in C. | same as row 11: no validation on either side; not invoked with NULL. |
| 13 | all `PfnN` predictors (internal, via `__difftest_predict`) | `idx` arbitrary / negative / `INT_MIN` — the C masks with `& 7` (`psamp[(idx - k) & 7]`), so *no* index is rejected; `INT_MIN - 1` also signed-overflow-wraps. There is no bounds check and no error path. | never errors; must return the same masked-index arithmetic result as Rust for every `idx` incl. `INT_MIN`, `INT_MAX`, negatives |
| 14 | `BTAC1C2_PredictSample` `case 7/8/9/12..15` (internal) | integer division by the constants `16`, `64`, `256` — divisor is a non-zero positive literal, so *no* division-by-zero or `INT_MIN / -1` trap is reachable. Recorded to document that the only division sites cannot error. | no error path exists; truncating-toward-zero result must match |

### Notes on rows 11–12

`c_src/src/lib.c` never checks a pointer.  Passing `NULL` is undefined
behaviour in the C, so "the same error/rejection" is not a meaningful
assertion — there is no error code to compare.  These rows are discharged by
asserting the *absence* of divergent validation: the Rust must not introduce a
check/panic that the C lacks (verified by inspection of `src/lib.rs`, which
performs raw `*psamp.offset(..)` / `(*ridx).firfx[..]` reads with no `is_null`
guard, exactly like the C).
