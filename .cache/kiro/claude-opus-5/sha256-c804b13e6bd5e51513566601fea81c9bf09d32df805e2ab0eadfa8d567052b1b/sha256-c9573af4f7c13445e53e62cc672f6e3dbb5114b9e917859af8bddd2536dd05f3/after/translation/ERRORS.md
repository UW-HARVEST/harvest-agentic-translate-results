# ERRORS.md — error-surface table

Derived mechanically from `c_src/src/lib.c`. The grep sweep below is the whole
evidence base; this translation unit has **no** error macros, no `errno`, no
sentinel `-1`/`NULL` returns, no `assert`, and no explicit range/null checks.

```sh
grep -n "return"                          c_src/src/lib.c   # 15 hits, all value returns
grep -n "default\|assert\|NULL\|ERROR\|errno\|-1\|if (\|if(" c_src/src/lib.c
#   98:    default:      <- BTAC1C2_PredictSample      switch
#  222:    default:      <- BTAC1C2_GetPredictFunc     switch
#  269:    default:      <- call_predict               switch
```

So the *entire* rejection surface of this library is the three `default:`
labels, plus the boundary behaviours of the array indexing / division that the
C performs unconditionally. Rows below are one per distinct rejection path the C
actually contains.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `call_predict` (line 269 `default:`) | `pfcn` outside `0..=11`, negative side: `pfcn == -1` | returns `0` (`result` left at its initialiser) | [x] |
| 2 | `call_predict` (line 269 `default:`) | `pfcn` outside `0..=11`, one step past the top: `pfcn == 12` | returns `0` | [x] |
| 3 | `call_predict` (line 269 `default:`) | `pfcn` in the *partially handled* band `12..=15` — `BTAC1C2_PredictSample`'s switch has arms for these, `call_predict`'s does not | returns `0` for each of 12,13,14,15 | [x] |
| 4 | `call_predict` (line 269 `default:`) | `pfcn == 16` (one step past the widest band any switch in the file recognises) | returns `0` | [x] |
| 5 | `call_predict` (line 269 `default:`) | `pfcn == INT_MIN` (`-2147483648`) — extreme out-of-range enum-style value across the FFI boundary | returns `0` | [x] |
| 6 | `call_predict` (line 269 `default:`) | `pfcn == INT_MAX` (`2147483647`) | returns `0` | [x] |
| 7 | `call_predict` (line 269 `default:`) | `pfcn == INT_MIN + 1`, `INT_MAX - 1` (neighbours of the extremes) | returns `0` | [x] |
| 8 | `call_predict` (line 269 `default:`) | out-of-range value whose low 4 bits alias a *valid* code (e.g. `0x10000000`, `256`, `4096`, `-4`, `0x7FFFFFF0`) — catches any Rust translation that masked instead of compared | returns `0` | [x] |
| 9 | `BTAC1C2_GetPredictFunc` (line 222 `default:`) | `pfcn` outside `0..=11` | yields `(void *)BTAC1C2_PredictSample`, i.e. a pointer that is **not** equal to any `_Pfn*` helper; observable only through row 1–8's `0` result because the function is `static` | [x] |
| 10 | `BTAC1C2_PredictSample` (line 98 `default:`) | `pfcn` outside `0..=15` | `pred = 0`, returns `0`. Unreachable through the exported ABI (`static`, and `call_predict` never *calls* the pointer it obtains) — covered by construction/translation review, not by a differential FFI test, because no exported symbol can reach it. | [x] |
| 11 | `BTAC1C2_PredictSample` arms 12–15 | `ridx == NULL` while `pfcn` in `12..=15` → `ridx->firfx[...]` null deref | undefined behaviour (crash) in C. Unreachable through the exported ABI for the same reason as row 10; the Rust keeps the identical unchecked deref so it cannot be *more* permissive. | [x] |
| 12 | all `_Pfn*` / `BTAC1C2_PredictSample` | `psamp == NULL`, or `psamp` shorter than 8 `int`s | undefined behaviour in C; note `(idx - k) & 7` means the C never indexes outside `[0,7]`, so a genuine 8-element buffer is always in bounds for *any* `idx`, including negative and `INT_MIN`. Unreachable through the exported ABI. | [x] |

## Notes on rows 10–12

`call_predict` is the *only* dynamic symbol (see `SYMBOLS.md`). It obtains a
function pointer from `BTAC1C2_GetPredictFunc` and only ever **compares** it —
it never invokes it, and it never touches `psamp` or `ridx` (it has no such
parameters). Consequently rows 10–12 have no reachable trigger across the FFI
boundary: there is no exported entry point that can pass a `psamp`, an `idx`, or
a `ridx` into the library at all. They are recorded for completeness and
discharged by translation review — the Rust reproduces each `default: pred = 0`,
each unchecked `firfx` index, and each unchecked `psamp` deref one-for-one, with
`& 7` masking so no out-of-bounds index is possible.

Rows 1–9 are all exercised by `tests/differential.rs`
(`errors_*` test functions) against both `.so`s.
