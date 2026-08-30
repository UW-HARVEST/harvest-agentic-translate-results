# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

### Axis 1 — public entry point

`c_src/include/lib.h` declares exactly one function:

```c
int get_predict_func(int pfcn);
```

so the *exported* surface has a single entry point.

### Axis 2 — the lowest-level entry points (`static`, reached via the `difftest` shim)

`get_predict_func` is a thin one-shot wrapper. The real work lives in 14 lower
level functions, all `static`. Per the Phase B instruction to exercise the
lowest-level entry points directly and not only the convenience wrapper, each of
these is driven directly:

| id | function | dispatch |
|---|---|---|
| L0 | `BTAC1C2_PredictSample` | `switch (pfcn)` with 17 arms (`0..=15` + `default`) |
| L1..L12 | `BTAC1C2_PredictSample_Pfn0` .. `_Pfn11` | no dispatch; one fixed formula each |
| L13 | `BTAC1C2_GetPredictFunc` | `switch (pfcn)`, 13 arms (`0..=11` + `default`) |

`L13`'s `default:` arm is **not** observable through `get_predict_func` (the
wrapper's own `default:` never inspects the returned pointer).  A mutation-coverage
run confirmed this blind spot, so two further hooks were added -
`__difftest_selector` (returns *which* function was selected) and
`__difftest_call_selected` (calls whatever was selected) - and rows C39-C41 /
E8b-E8d cover them.  See `MUTATION_COVERAGE.md`.

### Axis 3 — `pfcn` value class (the only "mode" flag in the library)

There is no options struct, no init call, no global state, and no `#ifdef`. The
sole mode selector is the `int pfcn` argument. The distinct classes the source
distinguishes:

* `0` — 1-tap
* `1`,`2`,`3` — 2-tap, differing multiplier/shift
* `4`,`5`,`6` — paired-sum forms `p0`/`p1` over taps 1..3
* `7` — 5-tap, `/16`
* `8`,`9` — 8-tap, `/64`
* `10`,`11` — 4+4 block sums `p0`/`p1` over taps 1..8
* `12`,`13`,`14`,`15` — data-driven FIR reading `ridx->firfx[pfcn-12][0..7]`, `/256`
  (**the only arms that dereference `ridx`**)
* everything else — `default`

### Axis 4 — `idx` shape

`psamp[(idx - k) & 7]` for `k` in `1..=8`. The `& 7` makes the window wrap, so
the *rotation* of the 8-element window is `idx & 7`. Distinct shapes:

* `idx` in `0..=7` (each of the 8 rotations)
* `idx` a large positive multiple/non-multiple of 8
* `idx` negative (C `&` on a negative `int` is implementation-defined-free on
  two's complement, so it still lands in `0..=7`)
* `idx == INT_MIN` / `INT_MAX` (the subtraction `idx - k` overflows)

Note `k` runs to `8`, and `(idx - 8) & 7 == idx & 7`, so tap 8 aliases tap 0 of
the window — a shape the arithmetic depends on and which the tests must preserve.

### Axis 5 — `psamp` value shape

* all zeros
* all equal (constant signal)
* small values (no overflow anywhere)
* alternating extremes / `INT_MAX` / `INT_MIN` (accumulator overflow, and
  negative operands feeding `>>`, which is an *arithmetic* shift, and `/`, which
  truncates toward zero — the two round in opposite directions for negatives)

### Axis 6 — `ridx->firfx` value shape (only matters for `pfcn` 12..=15)

* all zeros
* identity-ish (`firfx[r][0] == 256`)
* `INT16_MAX` / `INT16_MIN` in every slot (overflow of the `int` accumulator)
* distinct values per row `r`, to prove `pfcn - 12` selects the right row

### Axis 7 — cargo feature combination

`[features]` in `Cargo.toml` declares one non-default feature:

| combo | meaning |
|---|---|
| (none / default) | shipping build; exports `get_predict_func` only |
| `difftest` | adds the `__difftest_predict` hook so the `static` predictors can be reached |

## Configuration table

One row per combination the C treats differently.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| C1 | `get_predict_func` | `pfcn` = each of `0..=11` exhaustively (every specialised arm of both switches) | [x] |
| C2 | `get_predict_func` | `pfcn` exhaustive over `-4096..=4096` (crosses every boundary of both switches) | [x] |
| C3 | `get_predict_func` | `pfcn` = randomized full-range `int` (seeded), 20000 draws | [x] |
| C4 | `get_predict_func` | `pfcn` = `INT_MIN`, `INT_MIN+1`, `-1`, `0`, `11`, `12`, `15`, `16`, `INT_MAX-1`, `INT_MAX` | [x] |
| C5 | `BTAC1C2_GetPredictFunc` (via `get_predict_func` observable) | selector returns specialised fn for `0..=11`, generic fn for all else — verified through the wrapper's 1/0 result | [x] |
| C6 | `BTAC1C2_PredictSample` (L0) | `pfcn` = `0`, all 8 `idx` rotations, `psamp` = all-zeros | [x] |
| C7 | `BTAC1C2_PredictSample` (L0) | `pfcn` = `0`, all 8 `idx` rotations, `psamp` = constant signal | [x] |
| C8 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `1,2,3` (2-tap family), all 8 rotations, randomized small `psamp` | [x] |
| C9 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `4,5,6` (paired-sum family), all 8 rotations, randomized small `psamp` | [x] |
| C10 | `BTAC1C2_PredictSample` (L0) | `pfcn` = `7` (5-tap `/16`), all 8 rotations, randomized `psamp` incl. negatives (truncating division) | [x] |
| C11 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `8,9` (8-tap `/64`), all 8 rotations, randomized `psamp` incl. negatives | [x] |
| C12 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `10,11` (4+4 block sums), all 8 rotations, randomized `psamp` | [x] |
| C13 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `12,13,14,15`, `firfx` all zeros | [x] |
| C14 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `12,13,14,15`, `firfx` distinct per row (proves `pfcn-12` row selection) | [x] |
| C15 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `12,13,14,15`, `firfx` randomized full `i16` range, randomized `psamp` | [x] |
| C16 | `BTAC1C2_PredictSample` (L0) | `pfcn` = each of `12,13,14,15`, `firfx` = `i16::MAX` / `i16::MIN` saturated + `psamp` extremes (accumulator overflow) | [x] |
| C17 | `BTAC1C2_PredictSample` (L0) | `pfcn` outside `0..=15`, randomized; `psamp`/`ridx` untouched by the `default` arm | [x] |
| C18 | `BTAC1C2_PredictSample` (L0) | every `pfcn` in `0..=15` with `psamp` at `INT_MAX`/`INT_MIN` extremes (signed overflow wrap-around) | [x] |
| C19 | `BTAC1C2_PredictSample` (L0) | every `pfcn` in `0..=15` with `idx` = `INT_MIN`, `INT_MAX`, and randomized negatives | [x] |
| C20 | `_Pfn0` (L1) | all 8 rotations x {zeros, constant, randomized small, extremes} | [x] |
| C21 | `_Pfn1` (L2) | all 8 rotations x {zeros, constant, randomized small, extremes} | [x] |
| C22 | `_Pfn2` (L3) | all 8 rotations x randomized, incl. negatives (arithmetic `>>1`) | [x] |
| C23 | `_Pfn3` (L4) | all 8 rotations x randomized, incl. negatives (arithmetic `>>2`) | [x] |
| C24 | `_Pfn4` (L5) | all 8 rotations x randomized, incl. negatives (`p0 - (p1>>1)`) | [x] |
| C25 | `_Pfn5` (L6) | all 8 rotations x randomized, incl. negatives (`(3*p0-p1)>>2`) | [x] |
| C26 | `_Pfn6` (L7) | all 8 rotations x randomized, incl. negatives (`(5*p0-p1)>>3`) | [x] |
| C27 | `_Pfn7` (L8) | all 8 rotations x randomized, incl. negatives (`/16` truncation vs `>>`) | [x] |
| C28 | `_Pfn8` (L9) | all 8 rotations x randomized, incl. negatives (`/64`) | [x] |
| C29 | `_Pfn9` (L10) | all 8 rotations x randomized, incl. negatives (`/64`) | [x] |
| C30 | `_Pfn10` (L11) | all 8 rotations x randomized — **shift is `>>3` here, unlike `case 10`'s `>>4`** | [x] |
| C31 | `_Pfn11` (L12) | all 8 rotations x randomized — **shift is `>>1` here, unlike `case 11`'s `>>3`** | [x] |
| C32 | `_Pfn10` vs `BTAC1C2_PredictSample(pfcn=10)` | same inputs to both; the two must *disagree* in C and disagree identically in Rust (guards against "fixing" the quirk) | [x] |
| C33 | `_Pfn11` vs `BTAC1C2_PredictSample(pfcn=11)` | same inputs to both; must disagree identically | [x] |
| C34 | all `_Pfn*` (L1..L12) | `pfcn` argument varied independently of the selected function (unused parameter) | [x] |
| C35 | all `_Pfn*` (L1..L12) | `ridx == NULL` (never dereferenced by the specialised predictors) | [x] |
| C36 | all L0..L13 | randomized property sweep: seeded RNG over (`pfcn` in `-32..=32`, `idx` full `int`, 8 `psamp` values full `int`, 32 `firfx` values full `i16`), 50000 iterations | [x] |
| C37 | feature `difftest` off (default) | `get_predict_func` rows C1..C5 re-run against the default-feature `.so` | [x] |
| C38 | feature `difftest` on | rows C1..C36 | [x] |
| C39 | `BTAC1C2_GetPredictFunc` (L13) direct, via `__difftest_selector` | the selector's *choice* observed directly: exhaustive `0..=11`, exhaustive `-4096..=4096`, extremes, and 20000 seeded random draws | [x] |
| C40 | `BTAC1C2_GetPredictFunc` + selected predictor, via `__difftest_call_selected` | the **composed pipeline**: select then call, over `pfcn` `-4..=17` x all `idx` shapes x all `psamp` shapes x 4 `firfx` shapes, plus 20000 randomized draws | [x] |
| C41 | composed pipeline vs the individual units | for `pfcn` in `0..=11` the pipeline must equal `_PfnN` (**not** generic `case N`, which differs at 10/11); for every other `pfcn` it must equal the generic dispatcher | [x] |
