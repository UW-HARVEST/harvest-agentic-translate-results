# ERRORS.md — error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The grep sweep for every
rejection idiom found:

```sh
grep -n 'RETURN_ERROR\|return -1\|return NULL\|assert\|errno\|exit(\|abort(' c_src/src/lib.c
#   -> no matches
grep -n 'if *(\|MAX\|MIN\|<=\|>=' c_src/src/lib.c
#   -> no matches (no explicit range checks, no null checks, no min/max constants)
grep -n 'switch\|default:' c_src/src/lib.c
#   -> 22, 98 | 185, 222 | 232, 269   (three switches, each with a `default:`)
```

So this library has **no error codes, no error enums, no asserts, no null
checks, and no explicit range checks**. Its *entire* rejection surface is the
`default:` arm of the three `switch (pfcn)` statements — an out-of-domain
`pfcn` is "rejected" by falling through to a fallback value rather than by
returning an error code. Each distinct `default:` / out-of-domain branch gets
its own row.

`pfcn` is a plain `int` parameter (not an `enum`), so the C accepts any 32-bit
value; every value outside the switch's case labels is a real input the C
handles and the Rust must handle identically.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result | test | status |
|---|----------|----------------------------------------------|-------------------|------|--------|
| 1 | `get_predict_func` (line 269 `default:`) | `pfcn == 12` — first value past the 0..11 domain of `BTAC1C2_GetPredictFunc`; also a *valid* label for `BTAC1C2_PredictSample`'s switch, so it is the trickiest boundary | `0` (`result` keeps its initialiser; `BTAC1C2_GetPredictFunc` returned `&BTAC1C2_PredictSample`, which matches no `case`) | `err_row01_pfcn_12` | [x] |
| 2 | `get_predict_func` (line 269 `default:`) | `pfcn == 13, 14, 15` — the remaining `BTAC1C2_PredictSample` case labels, still outside `GetPredictFunc`'s 0..11 domain | `0` for each | `err_row02_pfcn_13_14_15` | [x] |
| 3 | `get_predict_func` (line 269 `default:`) | `pfcn == 16` — first value past *every* case label in *every* switch in the file | `0` | `err_row03_pfcn_16` | [x] |
| 4 | `get_predict_func` (line 269 `default:`) | `pfcn == -1` — one step below the low end of the valid domain | `0` | `err_row04_pfcn_minus_1` | [x] |
| 5 | `get_predict_func` (line 269 `default:`) | `pfcn < -1` (arbitrary negative, e.g. `-2, -12, -16, -1000`) | `0` for each | `err_row05_pfcn_negative` | [x] |
| 6 | `get_predict_func` (line 269 `default:`) | `pfcn == INT_MAX` (`2147483647`) — extreme positive, exercises the switch's jump-table/range lowering | `0` | `err_row06_pfcn_int_max` | [x] |
| 7 | `get_predict_func` (line 269 `default:`) | `pfcn == INT_MIN` (`-2147483648`) — extreme negative; note `pfcn - 12` would overflow, so any lowering that computes the jump-table offset must not mis-dispatch | `0` | `err_row07_pfcn_int_min` | [x] |
| 8 | `get_predict_func` (line 269 `default:`) | out-of-range "enum-like" values crossing the FFI boundary: values that would be plausible variants of a 4-bit predictor-function enum but have no matching `case` — `0x10, 0xFF, 0x100, 0xFFFF, 0x10000, 0x7FFFFFFE` | `0` for each | `err_row08_out_of_range_enum_values` | [x] |
| 9 | `get_predict_func` (line 269 `default:`) | bit-pattern aliasing: values congruent to a *valid* index modulo a power of two (`16, 32, 256, 4096, -16, -256`, and `k + 65536` for k in 0..11). A masking bug (`pfcn & 15`, `pfcn & 7`) would wrongly return `1` | `0` for each | `err_row09_masking_aliases` | [x] |
| 10 | `get_predict_func` (line 269 `default:`) | exhaustive sweep of the whole near-domain `-2048..=2048`, plus 200k pseudo-random `i32` values (fixed seed) | `1` iff `0 <= pfcn <= 11`, else `0` | `err_row10_exhaustive_and_random` | [x] |
| 11 | `BTAC1C2_GetPredictFunc` (line 222 `default:`) | `pfcn` outside 0..11 | returns `(void *)BTAC1C2_PredictSample` (the fallback pointer, **not** `NULL`) — observable only indirectly: `get_predict_func` then returns `0` because that pointer matches no `case` | covered transitively by rows 1–10; `static`, so not FFI-reachable | [x] |
| 12 | `BTAC1C2_PredictSample` (line 98 `default:`) | `pfcn < 0` or `pfcn > 15` | `pred = 0` | not FFI-reachable (`static`, and `get_predict_func` never calls it); verified by source inspection — Rust `_ => { pred = 0; }` | [x] |

## Notes on non-reachable rows

Rows 11 and 12 concern `static` C functions with local (`t`) linkage. They are
absent from `nm -D` for both libraries, so no differential FFI test can call
them directly; row 11 is exercised transitively (every row 1–10 goes through
it), and row 12's Rust counterpart was checked against the C by reading both
sources. This is documented rather than stubbed — no fake export was added to
make them appear callable.

Deliberately *not* tested as "null pointer" cases: `get_predict_func` takes no
pointer parameters, so there is no null-pointer input to construct at the ABI
boundary. The `*mut c_int` / `*mut btac1c_idxstate` parameters exist only on the
non-exported predictors.
