# ERRORS.md — Phase A: ERROR-SURFACE TABLE

Derived **mechanically** from `c_src/src/lib.c`. Grep results that define the
surface:

```
$ grep -nE "return -1|return NULL|assert|RETURN_ERROR|errno|== NULL|!= NULL|exit\(|abort\(" c_src/src/lib.c
(none found)

$ grep -n "default:" c_src/src/lib.c
98:    default:      # BTAC1C2_PredictSample   -> pred = 0
222:    default:      # BTAC1C2_GetPredictFunc -> fcn = BTAC1C2_PredictSample
269:    default:      # call_predict           -> result stays 0
```

The library has **no** error macros, **no** `assert`, **no** `errno`, **no**
NULL checks, **no** explicit range checks and **no** min/max constants. Its
entire rejection surface is the three `switch` `default:` fall-backs plus the
implicit masking (`& 7`) that keeps array indexing in range. Every distinct
rejection path in the C is one row below.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | ✔ |
|---|----------|---------------------------------------------|-------------------|------|---|
| E1 | `call_predict` (line 269 `default:`) | `pfcn == 12` — first value past the 0..11 range of `GetPredictFunc`'s cases; no `case` matches in `call_predict`'s own switch, so `result` keeps its initialiser | returns `0` | `err_e1_pfcn_12_first_past_range` | [x] |
| E2 | `call_predict` (line 269 `default:`) | `pfcn == 13,14,15` — the values the *inner* `BTAC1C2_PredictSample` switch still has `case`s for (FIR predictors) but `call_predict` does not | returns `0` | `err_e2_pfcn_13_14_15` | [x] |
| E3 | `call_predict` (line 269 `default:`) | `pfcn == 16` — first value past even the inner switch's `case 15` | returns `0` | `err_e3_pfcn_16_past_inner_switch` | [x] |
| E4 | `call_predict` (line 269 `default:`) | `pfcn == -1` — one step below the valid range | returns `0` | `err_e4_pfcn_minus_one` | [x] |
| E5 | `call_predict` (line 269 `default:`) | `pfcn` any negative value (`-2 .. -1000`, randomized negatives) | returns `0` | `err_e5_pfcn_negative_range` | [x] |
| E6 | `call_predict` (line 269 `default:`) | `pfcn == INT_MIN` (`-2147483648`) — extreme out-of-range enum/int value across the FFI boundary | returns `0` | `err_e6_pfcn_int_min` | [x] |
| E7 | `call_predict` (line 269 `default:`) | `pfcn == INT_MAX` (`2147483647`) — extreme out-of-range value | returns `0` | `err_e7_pfcn_int_max` | [x] |
| E8 | `call_predict` (line 269 `default:`) | `pfcn` = out-of-range "enum" values with no valid variant, swept over `12..=4096`, all powers of two, `±2^k`, and 20 000 fixed-seed random `i32`s | returns `0` for every value outside `0..=11`, `1` inside | `err_e8_exhaustive_out_of_range_sweep`, `err_e8b_random_i32_sweep` | [x] |
| E9 | `BTAC1C2_GetPredictFunc` (line 222 `default:`) | `pfcn` outside `0..=11` → returns `(void*)BTAC1C2_PredictSample` instead of a specialised `Pfn` — observable only as "no `case` in `call_predict` matches", i.e. result `0` | returns generic predictor ptr ⇒ `call_predict` → `0` | `err_e9_getpredictfunc_default_fallback` (via internal-symbol harness + `call_predict`) | [x] |
| E10 | `BTAC1C2_PredictSample` (line 98 `default:`) | `pfcn` outside `0..=15` (e.g. `16`, `-1`, `INT_MIN`, `INT_MAX`) → `pred = 0` regardless of `psamp`/`ridx` contents | returns `0` | `err_e10_predictsample_default_zero` (internal-symbol harness) | [x] |
| E11 | `BTAC1C2_PredictSample` cases 12..15 | `ridx` is dereferenced (`ridx->firfx[pfcn-12]`) with **no NULL check**; a NULL `ridx` with `pfcn` in `12..=15` is a segfault in C. Unreachable from the public API (`call_predict` never calls the predictors), so the required behaviour is "not exercised"; the test asserts the *reachable* contract instead: `pfcn` 12..15 through `call_predict` never dereferences anything and returns `0` | no deref via public API; `call_predict` → `0` | `err_e11_no_ridx_deref_via_public_api` | [x] |
| E12 | `BTAC1C2_PredictSample*` — index masking | `idx` far out of any array range (`INT_MIN`, `INT_MAX`, huge negatives) — the C never range-checks `idx`, it *masks* with `& 7`, so no read can leave the 8-element window: `psamp[(idx-k) & 7]` | no out-of-bounds read; result equals the value for `idx & 7` | `err_e12_idx_masking_no_oob` (internal-symbol harness) | [x] |
| E13 | generic FFI boundary: NULL pointers | `call_predict` takes no pointers, so a NULL pointer cannot be passed to the public ABI. Covered by asserting the exported symbol's signature/arity: only one `int` argument | n/a — no pointer parameter exists | `err_e13_no_pointer_params_in_public_abi` | [x] |
| E14 | generic FFI boundary: zero / oversized lengths | there is no length or buffer parameter anywhere in the public ABI (`int call_predict(int)`); the only "length" is the hard-coded 8-entry ring window handled by `& 7` (row E12) | n/a — no length parameter exists | documented; covered by E12 | [x] |

## Notes on deliberate C quirks that must NOT be "fixed"

These are **not** errors, but they are the places a translator is most likely to
silently "correct" the C. The Rust must reproduce them verbatim:

* `BTAC1C2_PredictSample_Pfn10` uses `>> 3`, while `case 10:` of the big switch
  uses `>> 4` for the same formula.
* `BTAC1C2_PredictSample_Pfn11` uses `>> 1`, while `case 11:` of the big switch
  uses `>> 3` for the same formula.
* `/16`, `/64`, `/256` are C integer divisions (truncate toward zero) while the
  other cases use `>>` (arithmetic shift, floors toward −∞). For negative
  operands these differ, and the difference is part of the contract.
