# ERRORS.md — Phase C error-surface table

Mechanically derived from `c_src/src/lib.c`. There are **no** `RETURN_ERROR`
macros, **no** `assert`, **no** `return -1`, **no** `return NULL`, **no** error
enums, **no** null checks and **no** explicit range checks anywhere in the C
source (verified by grep, see below). The only rejection-like behaviour is the
`default:` arm of the two `switch (pfcn)` statements and the `default:` arm of
the `switch (pfcn)` in `BTAC1C2_GetPredictFunc`.

```
$ grep -nE 'assert|RETURN_ERROR|return -1|return NULL|== *NULL|!= *NULL|errno|ERROR' c_src/src/lib.c
(no matches)
$ grep -nc 'default:' c_src/src/lib.c
3
```

`pfcn` is a plain `int` parameter (a C "enum-like" selector that accepts any
`int`), so every out-of-domain `int` value is a real input the C handles.

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 1 | `call_predict` | `pfcn == 12` — first value past the last selector handled by `BTAC1C2_GetPredictFunc` (`0..=11`); `GetPredictFunc` falls to `default:` and returns `BTAC1C2_PredictSample`, `call_predict`'s own `switch` falls to `default:` which leaves `result` untouched | returns `0` | [x] |
| 2 | `call_predict` | `pfcn == -1` — one step below the valid range | `default:` in both switches → `result` stays `0` | returns `0` | [x] |
| 3 | `call_predict` | `pfcn == 13, 14, 15` — values handled by the `12..15` arm of `BTAC1C2_PredictSample` but *not* by `GetPredictFunc` | returns `0` | [x] |
| 4 | `call_predict` | `pfcn == 16` — one step past the highest value any switch names | returns `0` | [x] |
| 5 | `call_predict` | `pfcn == INT_MAX` (`2147483647`) — extreme positive out-of-range enum value across FFI | returns `0` | [x] |
| 6 | `call_predict` | `pfcn == INT_MIN` (`-2147483648`) — extreme negative out-of-range enum value across FFI | returns `0` | [x] |
| 7 | `call_predict` | every `pfcn` in `-1000..=1000` outside `0..=11` — exhaustive sweep of the `default:` rejection arm | returns `0` | [x] |
| 8 | `call_predict` | randomized `pfcn` over the full `i32` range (10k fixed-seed samples) | `0` unless `pfcn` in `0..=11`, then `1` | [x] |
| 9 | `BTAC1C2_GetPredictFunc` (internal, reached via `call_predict`) | `pfcn` outside `0..=11` → `default:` returns `(void*)BTAC1C2_PredictSample`, an address that can never equal any `_PfnN`, so `call_predict` cannot report `1` | `call_predict` returns `0` | [x] |
| 10 | `BTAC1C2_PredictSample` (internal) | `pfcn` outside `0..=15` → `default: pred = 0` | prediction `0` (unobservable through the exported ABI; covered structurally) | [x] |
| 11 | `call_predict` | no pointer parameters exist in the exported ABI, so "null pointer" / "zero length" / "oversized length" boundaries are **not applicable**; documented here so the generic-boundary requirement is explicitly discharged | N/A — signature is `int call_predict(int)` | [x] |

## Notes on generic C-API boundaries

* **Null pointers** — the only exported function takes no pointer, so there is
  no null-pointer path. (The `btac1c_idxstate *ridx` pointer is only ever
  dereferenced by the internal `BTAC1C2_PredictSample` for `pfcn` 12..15, and
  that function is never *called* by `call_predict`, only address-compared.)
* **Zero / oversized lengths** — no length or size parameter exists.
* **Out-of-range enum values** — covered by rows 1–9; `pfcn` is the enum-like
  selector and is swept exhaustively over `-1000..=1000` plus `INT_MIN`,
  `INT_MAX` and 10k random `i32` values.

---

## Extension rows — error/degenerate paths of the `static` functions

Reachable only via the `internal_probe` test surface (see `CONFIGS.md`).

| # | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|---|----------|---------------------------------------------|-------------------|-----|
| 12 | `BTAC1C2_PredictSample` | `pfcn` outside `0..=15` (`-4..=-1`, `16..=20`, `i32::MIN`, `i32::MAX`) → `default: pred = 0` | returns `0`; verified byte-identical against the C probe | [x] |
| 13 | `BTAC1C2_PredictSample` | `pfcn` in `12..=15` with `ridx` pointing at a struct whose `firfx` rows are all zero → FIR sum is `0`, `0 / 256 == 0` | returns `0` | [x] |
| 14 | `BTAC1C2_GetPredictFunc` | `pfcn` outside `0..=11` → `default:` yields `&BTAC1C2_PredictSample`, never a `_PfnN` | dispatch index `-1` | [x] |
| 15 | all predictors | `idx` negative, incl. `i32::MIN + 9` — `(idx - k) & 7` in C wraps on signed overflow; Rust must use wrapping subtraction, not a panic or a different residue | identical residue and identical result | [x] |
| 16 | all predictors | `idx` = `i32::MAX` — `(idx - k) & 7` at the top of the range | identical result | [x] |
| 17 | all predictors | negative samples through `>>` (arithmetic shift, gcc) and `/` (truncation toward zero) — the two round in opposite directions, so a `>>`/`/` mix-up shows up only here | identical result | [x] |

`ridx` is never null in any exercised path: the C source dereferences it
unconditionally in the `12..=15` arm, so passing null would be UB in the C
itself (not a defined rejection) and is therefore out of scope — the C has no
null check to replicate (`grep '== *NULL' c_src/src/lib.c` → no matches).
