# CONFIGS.md — Configuration surface table (valid inputs)

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axis enumeration (from the source, not from guesses)

**Public entry points** — the full set, from the header:

```
c_src/include/lib.h:1:  int get_predict_func(int pfcn);
```

That is the *only* declaration, and `nm -D` confirms `get_predict_func` is
the only exported symbol. There are no lower-level exported entry points to
drive directly: `BTAC1C2_GetPredictFunc`, `BTAC1C2_PredictSample` and
`BTAC1C2_PredictSample_Pfn0..11` are all `static` (internal linkage), so an
external consumer cannot reach them except *through* `get_predict_func`.
Consequently `get_predict_func` **is** the lowest-level entry point
available at the ABI boundary, and every row below drives it directly.

**Runtime options / modes / flags** — none. There is no init function, no
handle, no context struct passed in, no global state, no setter, and no
`#ifdef`. Verified: `grep -cE 'static [^(]*=|extern|#if' c_src/src/lib.c`
finds no mutable file-scope state. The library is a pure function of its one
argument.

**Input shapes the code special-cases** — the `switch` branches on `pfcn`:

- `get_predict_func` (line 233): `case 0` … `case 11`, then `default`.
  Each of the twelve cases performs a *different* pointer-identity
  comparison, so all twelve are distinct code paths.
- `BTAC1C2_GetPredictFunc` (line 183): `case 0` … `case 11`, then `default`.
- `BTAC1C2_PredictSample` (line 18): `case 0` … `case 11`, `case 12..15`
  (fall-through group using `ridx->firfx[pfcn-12]`), then `default`.

Cross-product, pruned to what the C actually distinguishes: because the two
outer switches share the same partition of `pfcn` (`{0},{1},…,{11},else`),
the meaningful configuration set is one row per specialised predictor plus
rows for the internal-fallback region. Boundary/extreme integer values are
enumerated here as valid-input shapes as well as in `ERRORS.md`.

## The table

| #  | entry point(s) | configuration (options set + input shape) | [ ] |
|----|----------------|--------------------------------------------|-----|
| 1  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 0` — selects `_Pfn0`; identity compare against `_Pfn0` | [x] |
| 2  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 1` — selects `_Pfn1` | [x] |
| 3  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 2` — selects `_Pfn2` | [x] |
| 4  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 3` — selects `_Pfn3` | [x] |
| 5  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 4` — selects `_Pfn4` (first two-term `p0`/`p1` shape) | [x] |
| 6  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 5` — selects `_Pfn5` | [x] |
| 7  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 6` — selects `_Pfn6` | [x] |
| 8  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 7` — selects `_Pfn7` (first 5-tap `/16` shape) | [x] |
| 9  | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 8` — selects `_Pfn8` (8-tap `/64`) | [x] |
| 10 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 9` — selects `_Pfn9` (8-tap `/64`, different taps) | [x] |
| 11 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 10` — selects `_Pfn10` (4+4 group, `>>3`; note the C's `case 10` uses `>>4` — divergence preserved) | [x] |
| 12 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 11` — selects `_Pfn11` (4+4 group, `>>1`; note the C's `case 11` uses `>>3` — divergence preserved) | [x] |
| 13 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 12` — fallback region, first `firfx` FIR arm | [x] |
| 14 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 13` — fallback region, 2nd FIR arm | [x] |
| 15 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 14` — fallback region, 3rd FIR arm | [x] |
| 16 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 15` — fallback region, 4th (last) FIR arm | [x] |
| 17 | `get_predict_func` → `BTAC1C2_GetPredictFunc` | `pfcn = 16` — fallback region past every internal `case` | [x] |
| 18 | `get_predict_func` | `pfcn` exhaustive sweep over `-2048..=2048` (covers every boundary between the twelve specialised arms, the FIR group, and both fallback sides, in one contiguous run) | [x] |
| 19 | `get_predict_func` | `pfcn` randomized over the **full** `i32` range, 200 000 values, fixed-seed xorshift PRNG (`seed = 0x9E3779B97F4A7C15`) | [x] |
| 20 | `get_predict_func` | `pfcn` randomized over the *near-valid* band `-64..=64`, 20 000 values, fixed seed — dense re-hit of the interesting partition with a different call order | [x] |
| 21 | `get_predict_func` | `pfcn` randomized over `i32::MIN..=i32::MIN+4096` and `i32::MAX-4096..=i32::MAX`, fixed seed — wrap-around/extreme band | [x] |
| 22 | `get_predict_func` | repeated / interleaved calls: the whole valid set `0..=11` called in a randomized order, 50 000 calls, asserting the function is stateless (no dependence on call history) in both C and Rust | [x] |
| 23 | `get_predict_func` | called from 4 concurrent threads against both `.so`s simultaneously — confirms no hidden shared state in either implementation | [x] |
| 24 | `get_predict_func` | **EXHAUSTIVE**: all 4 294 967 296 `i32` values, every one compared C-vs-Rust and against the source-derived oracle (`tests/exhaustive.rs`). Run against all four cdylib build configurations (release, dev/unoptimised, release+fat-LTO+cgu=1, release+opt-level=z) because pointer identity — the property `get_predict_func` is built on — is exactly what identical-code-folding and LTO can perturb. | [x] |

## Notes

- There are **no** feature flags, byte-order options, element types, widths,
  counts, or formats in this library — I looked for them and the source has
  none. Inventing such axes would produce rows the C does not distinguish.
- Rows 1–17 pin each individual `switch` arm; rows 18–21 supply the
  many-randomized-inputs requirement across the full input domain; rows
  22–23 probe statefulness/concurrency, the only remaining way a
  pointer-identity-based implementation could diverge; row 24 then settles
  the question outright by enumerating the whole input space.
- Because the exported API's entire input domain is a single 32-bit `int`,
  row 24 is a *complete* proof of behavioural equivalence for this ABI
  surface, not a sample of it.
