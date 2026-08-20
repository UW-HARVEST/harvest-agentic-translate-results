# ERRORS.md — Error-surface table (Phase A, gates Phase C)

Derived mechanically from `c_src/src/lib.c`. The greps below are the *complete*
search for rejection constructs:

```
$ grep -nE "assert|RETURN_ERROR|return *-1|NULL|errno|error|ERR" c_src/src/lib.c
(none found)
$ grep -n "default:" c_src/src/lib.c
98:    default:      # BTAC1C2_PredictSample
222:    default:      # BTAC1C2_GetPredictFunc
269:    default:      # get_predict_func
```

This library has **no** error codes, **no** sentinel returns, **no** `assert`s,
**no** null checks, and **no** explicit range checks. Its entire rejection
surface consists of the three `switch` `default:` fall-through arms, which
"reject" an out-of-range `pfcn` by quietly producing a neutral value rather
than by signalling an error. Each of the three is one row below.

`get_predict_func` takes no pointers, so there is no null-pointer surface on
the public API; and `int pfcn` is a plain `int` (not an enum), so *every* one of
the 2^32 bit patterns is a legal argument — the out-of-range values are ordinary
inputs the C handles, and rows 3–8 cover them.

## Table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `BTAC1C2_PredictSample` (`default:`, line 98) | `pfcn` outside `0..=15` (internal fn; reached only via the pointer `BTAC1C2_GetPredictFunc` returns for `pfcn` outside `0..=11`) | `pred = 0`, returns `0`. Not observable through the public API because `get_predict_func` never *calls* the pointer — asserted indirectly via row 3. |
| 2 | `BTAC1C2_GetPredictFunc` (`default:`, line 222) | `pfcn` outside `0..=11` | returns `(void*)BTAC1C2_PredictSample`, i.e. a pointer that matches **none** of the `_Pfn*` addresses `get_predict_func` compares against |
| 3 | `get_predict_func` (`default:`, line 269) | `pfcn` outside `0..=11` — no `case` matches, so `result` keeps its initial value and no comparison is performed | returns `0` |
| 4 | `get_predict_func` | `pfcn == -1` (one step below the valid range) | returns `0` |
| 5 | `get_predict_func` | `pfcn == 12` (one step past the valid range; also the first of the `firfx` cases 12–15 that exist in `BTAC1C2_PredictSample` but **not** in `BTAC1C2_GetPredictFunc`) | returns `0` |
| 6 | `get_predict_func` | `pfcn` in `13..=15` — the remaining `BTAC1C2_PredictSample`-only cases | returns `0` |
| 7 | `get_predict_func` | `pfcn == INT_MIN` (`-2147483648`) | returns `0` |
| 8 | `get_predict_func` | `pfcn == INT_MAX` (`2147483647`) | returns `0` |

## Checklist

Checked off only when a differential test constructs that exact condition,
calls **both** the C `.so` and the Rust `.so` through `libloading`, and asserts
the two return the *same* value (not merely "both non-crashing").

- [x] 1 — `differential::internals_arith::row1_predictsample_default_via_pointer_identity`
      (needs `--features diff_internals`; `BTAC1C2_PredictSample` is `static`)
- [x] 2 — `differential::internals::row2_getpredictfunc_default_arm`
      (needs `--features diff_internals`)
- [x] 3 — `differential::row3_get_predict_func_default_arm`
- [x] 4 — `differential::row4_minus_one_boundary`
- [x] 5 — `differential::row5_twelve_boundary`
- [x] 6 — `differential::row6_thirteen_to_fifteen`
- [x] 7 — `differential::row7_int_min`
- [x] 8 — `differential::row8_int_max`

Generic boundaries additionally covered (see `tests/differential.rs`):

- [x] out-of-range "enum" values across the FFI boundary — `pfcn` has no valid
      variant for any value outside `0..=11`; swept exhaustively over
      `-4096..=4096` and randomized over the full `i32` range
      (`differential::generic_out_of_range_enum_values_full_i32`)
- [x] `INT_MIN` / `INT_MAX` / `INT_MIN+1` / `INT_MAX-1` / `0` / `±1` extremes
      (`differential::generic_extreme_values`)
- [x] no null-pointer or length arguments exist on the public API — documented
      above rather than tested, because `get_predict_func(int)` takes neither

Additionally verified: `diffshim_pfn`'s own out-of-range selector (`which`
outside `0..=11`) returns the identical `0x5EEDBAD` sentinel on both sides —
`differential::internals_arith::generic_pfn_dispatch_out_of_range`.

Every row above was also confirmed to be *sensitive*: mutating the
corresponding branch in `src/lib.rs` makes its test fail (see the mutation
table in `CONFIGS.md`).
