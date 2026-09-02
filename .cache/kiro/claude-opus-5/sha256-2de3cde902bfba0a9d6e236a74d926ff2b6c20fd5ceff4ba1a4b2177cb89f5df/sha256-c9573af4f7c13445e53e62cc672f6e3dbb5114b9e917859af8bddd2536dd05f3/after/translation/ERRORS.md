# ERRORS.md — Error / rejection surface table

Derived mechanically from `c_src/src/lib.c` and `c_src/include/lib.h`.

## Mechanical grep evidence

```sh
grep -nE 'assert|RETURN_ERROR|NULL|errno|ERROR|_MIN|_MAX|enum|#if' \
     c_src/src/lib.c c_src/include/lib.h
# -> (no matches)

grep -n 'default:' c_src/src/lib.c
# -> 98:    default:      (in BTAC1C2_PredictSample)
#    222:   default:      (in BTAC1C2_GetPredictFunc)
#    269:   default:      (in get_predict_func)

grep -n 'return' c_src/src/lib.c        # -> 15 returns, none is an error sentinel
```

Findings that shape this table:

- There are **no** `assert`s, **no** error macros, **no** error enums, **no**
  `NULL` checks, **no** `errno` use, and **no** `#if`/`#ifdef` in the C.
- The only public entry point, `int get_predict_func(int pfcn)`, takes a
  single `int` and **no pointers**, so there is no null-pointer, length, or
  buffer surface to abuse at the exported ABI boundary. Generic
  null/zero-length/oversized-length probes are therefore **not applicable**
  to the exported API; the equivalent generic boundary probes for an `int`
  parameter are the extreme and just-past-valid-range integer values, which
  are rows 4–11 below.
- The library's entire "rejection" behaviour lives in the three `default:`
  arms. `pfcn` is used exactly like a C enum tag (valid tags `0..=11`), so
  any `int` with no valid variant is a real input, covered below.

## The table

| #  | function | trigger (the exact invalid input/condition) | expected C result |
|----|----------|----------------------------------------------|-------------------|
| 1  | `get_predict_func` (`default:` @ line 269) | `pfcn` outside `0..=11` — `result` is left at its initialiser | returns `0` |
| 2  | `BTAC1C2_GetPredictFunc` (`default:` @ line 222) | `pfcn` outside `0..=11` — no specialised predictor exists | returns `(void*)BTAC1C2_PredictSample` (the generic fallback), **not** `NULL`; observable only as row 1's `0` because `get_predict_func`'s `default:` arm never compares it |
| 3  | `BTAC1C2_PredictSample` (`default:` @ line 98) | `pfcn` outside `0..=15` | `pred = 0`, returns `0`. Not reachable through the exported ABI: the pointer `BTAC1C2_GetPredictFunc` returns is only ever *compared*, never called, and the function has internal linkage. Documented for completeness. |
| 4  | `get_predict_func` | `pfcn == 12` — first value one step past the valid range `0..=11` (and the first `firfx` FIR arm of `BTAC1C2_PredictSample`) | returns `0` |
| 5  | `get_predict_func` | `pfcn == 13, 14, 15` — remaining FIR arms; still no specialised `_PfnNN` | returns `0` |
| 6  | `get_predict_func` | `pfcn == 16` — one step past the *widest* internal `case` range (`0..=15`) | returns `0` |
| 7  | `get_predict_func` | `pfcn == -1` — one step below the valid range | returns `0` |
| 8  | `get_predict_func` | `pfcn == INT_MIN` (`-2147483648`) | returns `0` |
| 9  | `get_predict_func` | `pfcn == INT_MAX` (`2147483647`) | returns `0` |
| 10 | `get_predict_func` | `pfcn` = arbitrary large negative values (out-of-range "enum" tags crossing the FFI boundary) | returns `0` |
| 11 | `get_predict_func` | `pfcn` = arbitrary large positive values (out-of-range "enum" tags crossing the FFI boundary) | returns `0` |

Note on rows 4–11: the C API has no way to *signal* rejection other than
returning `0`, and `0` is also what a hypothetical "predictor lookup
mismatch" would produce. The tests assert the exact returned value (`0`),
not merely "both failed somehow", and additionally assert C and Rust return
byte-identical `int`s for every probe.

## Status

| # | test | status |
|---|------|--------|
| 1  | `err_row1_default_arm_out_of_range` | [x] PASS |
| 2  | `err_row2_fallback_pointer_not_null` | [x] PASS |
| 3  | `err_row3_generic_fallback_unreachable` | [x] PASS |
| 4  | `err_row4_pfcn_12` | [x] PASS |
| 5  | `err_row5_pfcn_13_14_15` | [x] PASS |
| 6  | `err_row6_pfcn_16` | [x] PASS |
| 7  | `err_row7_pfcn_minus_1` | [x] PASS |
| 8  | `err_row8_int_min` | [x] PASS |
| 9  | `err_row9_int_max` | [x] PASS |
| 10 | `err_row10_large_negative_random` | [x] PASS |
| 11 | `err_row11_large_positive_random` | [x] PASS |
