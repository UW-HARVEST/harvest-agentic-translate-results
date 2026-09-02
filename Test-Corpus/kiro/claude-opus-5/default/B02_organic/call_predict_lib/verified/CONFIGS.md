# CONFIGS.md — configuration-surface table

## Axes the C actually branches on

Derived from `c_src/src/lib.c` and `c_src/include/lib.h`.

1. **Public entry points.** Exactly one: `int call_predict(int pfcn)`.
   `c_src/include/lib.h` additionally declares `int get_predict_func(int pfcn);`
   but no definition exists and `nm -D` confirms it is not in the ABI, so it is
   not an entry point. There are no convenience-vs-low-level layers to choose
   between: `call_predict` *is* the lowest level the ABI exposes. The internal
   layers it drives (`BTAC1C2_GetPredictFunc`, and the address identity of
   `BTAC1C2_PredictSample_Pfn0..11` / `BTAC1C2_PredictSample`) are `static` and
   reachable only through it.
2. **Runtime options / modes / flags.** None. No global state, no init call, no
   setter, no `#ifdef`, no environment lookup, no struct the caller can
   configure. `grep -n "#if\|#ifdef\|static [^i]" c_src/src/lib.c` finds no
   mutable module state — every `static` is a function.
3. **The single input axis: `pfcn`.** Three separate `switch (pfcn)` statements
   branch on it, with these distinct case partitions:
   * `BTAC1C2_PredictSample` (line 19): `0,1,2,3,4,5,6,7,8,9,10,11,12,13,14,15`
     (12–15 share one arm), `default`.
   * `BTAC1C2_GetPredictFunc` (line 183): `0..=11` each its own arm, `default`.
   * `call_predict` (line 228): `0..=11` each its own arm, `default`.
   The finest partition is therefore per-value for `0..=11`, the `12..=15` band,
   and everything else. Each of `0..=11` selects a **different** helper address,
   so each is a genuinely distinct configuration, not a repetition.
4. **Input shape.** `pfcn` is a scalar `int`; there is no buffer, length, count,
   width, element type, byte order, or format axis in the exported ABI, and no
   "empty / one / many" dimension. The C never reads memory through a pointer on
   any path reachable from `call_predict`. The shape axis therefore degenerates
   to the *value domain* of a 32-bit signed int, whose meaningful partitions are
   the ones enumerated above plus the sign/extremum boundaries.

Because axes 2 and 4 are singletons, the cross-product reduces to the `pfcn`
partition. Rows below are that pruned cross-product: one row per band the C
distinguishes, each driven with many randomised values (fixed seed) rather than
a single hand-picked one.

## Table

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `call_predict` | `pfcn = 0` — selects `_Pfn0`; no options (none exist) | [x] |
| 2 | `call_predict` | `pfcn = 1` — selects `_Pfn1` | [x] |
| 3 | `call_predict` | `pfcn = 2` — selects `_Pfn2` | [x] |
| 4 | `call_predict` | `pfcn = 3` — selects `_Pfn3` | [x] |
| 5 | `call_predict` | `pfcn = 4` — selects `_Pfn4` | [x] |
| 6 | `call_predict` | `pfcn = 5` — selects `_Pfn5` | [x] |
| 7 | `call_predict` | `pfcn = 6` — selects `_Pfn6` | [x] |
| 8 | `call_predict` | `pfcn = 7` — selects `_Pfn7` | [x] |
| 9 | `call_predict` | `pfcn = 8` — selects `_Pfn8` | [x] |
| 10 | `call_predict` | `pfcn = 9` — selects `_Pfn9` | [x] |
| 11 | `call_predict` | `pfcn = 10` — selects `_Pfn10` (the arm whose shift differs from the `case 10:` arm of the big switch) | [x] |
| 12 | `call_predict` | `pfcn = 11` — selects `_Pfn11` (likewise `>> 1` vs `>> 3`) | [x] |
| 13 | `call_predict` | `pfcn` in the `12..=15` band recognised by `BTAC1C2_PredictSample` but not by `call_predict`'s switch — all four values | [x] |
| 14 | `call_predict` | `pfcn` in `16..=1023`, randomised (fixed seed), positive out-of-band | [x] |
| 15 | `call_predict` | `pfcn` negative, randomised (fixed seed) over `-1 ..= -1_000_000` | [x] |
| 16 | `call_predict` | `pfcn` randomised over the **full** `i32` domain (fixed seed, many draws) — value-dependent / aliasing coverage | [x] |
| 17 | `call_predict` | `pfcn` at signed boundaries: `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `1`, `INT_MAX-1`, `INT_MAX` | [x] |
| 18 | `call_predict` | exhaustive sweep of the whole small-value neighbourhood `-4096..=4096` (contains every distinguished band and both boundaries of each) | [x] |
| 19 | `call_predict` | repeated / interleaved invocation order (same value twice, then alternating in-band and out-of-band) — proves the library is stateless in both implementations | [x] |
| 20 | `call_predict` | called from a second thread and concurrently from several threads — the C has no shared mutable state; the Rust must not either | [x] |

All 20 rows are exercised by `translation/tests/differential.rs`, loading **both**
`.so` files through `libloading` and comparing byte-for-byte (`i32` results
compared as raw 4-byte little-endian images as well as as integers).

## Feature combinations

`translation/Cargo.toml` declares **no** `[features]` table, so the only build
configuration is the default one (`--no-default-features` is equivalent). See
`check_features.sh` for the automated enumeration that confirms this.
