# ERRORS.md — Phase C error-surface table

Derived mechanically from `c_src/src/lib.c`. Greps run over the whole C source:

```
grep -n 'assert|return -1|return NULL|RETURN_ERROR|ERROR|errno|if *(|#if|#ifdef|malloc|free(' src/lib.c include/lib.h
  -> no matches (exit 1)
grep -c 'default:' src/lib.c   -> 3
grep -n 'switch'   src/lib.c   -> lines 22, 185, 232
```

So the C library has **no** error macros, no `assert`, no `errno`, no allocation,
no null-pointer checks, no explicit range checks, and not a single `if`
statement. Its entire rejection surface is the `default:` label of its three
`switch` statements. Every row below is one of those three defaults, plus the
generic FFI-boundary boundaries required by the task.

`call_predict` is the only exported entry point, so "expected C result" is stated
as what `call_predict` returns (`int`).

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `call_predict` (line 232 `switch`, `default:`) | `pfcn` negative, e.g. `-1` | falls to `default:`, `result` keeps its initialiser → returns `0` |
| 2 | `call_predict` (line 232 `switch`, `default:`) | `pfcn > 11`, e.g. `12` | returns `0` |
| 3 | `call_predict` (line 232 `switch`, `default:`) | `pfcn == INT_MIN` (`-2147483648`) | returns `0` |
| 4 | `call_predict` (line 232 `switch`, `default:`) | `pfcn == INT_MAX` (`2147483647`) | returns `0` |
| 5 | `BTAC1C2_GetPredictFunc` (line 185 `switch`, `default:`) | `pfcn` outside `0..=11` | returns `(void*)BTAC1C2_PredictSample` (the generic dispatcher), **not** a `Pfn*` helper; observable only as row 1–4's `0` |
| 6 | `BTAC1C2_PredictSample` (line 22 `switch`, `default:`) | `pfcn` outside `0..=15` (incl. negative) | `pred = 0`, returns `0` regardless of `psamp` contents |
| 7 | `BTAC1C2_PredictSample`, `case 12..=15` | `pfcn` in `12..=15` — indexes `ridx->firfx[pfcn - 12]`; **no bounds check and no null check on `ridx`** | dereferences `ridx`; with a valid `ridx` returns the FIR sum `/ 256` |
| 8 | `BTAC1C2_PredictSample*` — all arms | `idx` negative or huge; `psamp[(idx - k) & 7]` | **no rejection**: `& 7` is applied to an `int`, and because `(idx-k)` is masked with `7` the index is always in `0..=7` for any `int` (two's complement `&`), so no out-of-range access and no error path |
| 9 | `BTAC1C2_PredictSample*` — all arms | `psamp == NULL` | **no check**: unconditional dereference → UB / SIGSEGV in C. Not reachable from the exported API; not exercised (both sides would crash identically by construction, and asserting on UB is meaningless) |
| 10 | out-of-range "enum" value across FFI | `pfcn` is a plain `int`, so any 32-bit value is accepted; there is no enum and no validation | every value outside `0..=11` returns `0`; values `0..=11` return `1` |

## Notes on the boundaries the task asks about generically

* **Null pointers** — the only pointer parameters (`psamp`, `ridx`) belong to
  `static` functions that the public ABI cannot reach. `call_predict` takes no
  pointers. Row 9 records this; it is UB in C, so it is not asserted on.
* **Zero / oversized lengths** — the API has no length or size parameter.
* **One step past a valid range** — rows 1 and 2 (`-1` and `12`) are exactly the
  two values one step outside `0..=11`; row 7 covers `12..=15`, the extra range
  the inner `switch` distinguishes, and the tests sweep `16` and `-2` as well.
* **Out-of-range enum values** — row 10. `pfcn` is `int`, not an enum, so the
  test sweeps the entire boundary neighbourhood plus randomized 32-bit values.

## Gate

- [x] Row 1 — covered by `errors::row1_negative_one`
- [x] Row 2 — covered by `errors::row2_twelve`
- [x] Row 3 — covered by `errors::row3_int_min`
- [x] Row 4 — covered by `errors::row4_int_max`
- [x] Row 5 — covered by `errors::row5_dispatch_default_is_generic`
- [x] Row 6 — covered by `errors::row6_predict_sample_default_returns_zero`
- [x] Row 7 — covered by `errors::row7_fir_arms_12_to_15`
- [x] Row 8 — covered by `errors::row8_extreme_idx_never_out_of_range`
- [x] Row 9 — documented as UB, intentionally not asserted (see above)
- [x] Row 10 — covered by `errors::row10_full_int_sweep_and_random`
