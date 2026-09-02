# CONFIGS.md — Configuration-surface table (Phase A → gates Phase B)

## Mechanical derivation of the axes

The public API surface is enumerated from `c_src/include/lib.h` (one prototype)
and the branch axes from every `if`/`else`/`switch`/`#if` in `c_src/src/lib.c`:

```
grep -nE '\b(if|else|switch|case|while|for)\b' -r src include
  8:  if ((uni ^ uni1) & (~7))     <-- axis K (upper-candidate clamp)
  10: if ((uni ^ uni2) & (~7))     <-- axis K (lower-candidate clamp)
  12: if (lsbit) {                 <-- axis L (LSB mode)
  13:     if (lsbit == 4) {        <-- axis L
  20: } else if (lsbit & 1) {      <-- axis L
  24: } else {                     <-- axis L
  31: if (uni  & 8)                <-- axis G (diff sign / negation)
  37: if (uni1 & 8)                <-- axis G
  43: if (uni2 & 8)                <-- axis G
  57: if (d1 < d0)                 <-- axis W (winner selection)
  59: if (d2 < d0)                 <-- axis W
```

There are **no `#ifdef`s, no compile-time options, no runtime option/flag
setters, no handle/context object and no state to initialise** — the library is
a single pure function, so the "options" axis is carried entirely by the
`lsbit` mode argument and the "input shape" axes by the value classes below.

### Entry points (full set — no wrappers omitted)

| entry point | level | notes |
|---|---|---|
| `encode_quant(uni, step, pred, tgt, tgt2, lsbit)` | **lowest level == only level** | The library exports exactly one symbol (see `SYMBOLS.md`). There is no convenience / one-shot wrapper above it and no internal helper below it, so "exercise the low-level entry points directly" is satisfied by construction. |

### Axes

* **L — `lsbit` mode** (the one runtime option; dispatch is exhaustive over all `int`):
  * `L0`  : `lsbit == 0` → no LSB forcing at all.
  * `L4`  : `lsbit == 4` → clear bit0, then `x |= (x>>1) & (x>>2) & 1` on all three candidates. Checked **before** `lsbit & 1`, so `4` does *not* take the even branch.
  * `LODD`: `lsbit != 0,4` and `lsbit & 1` → force bit0 **set** on all three candidates (includes negative odd).
  * `LEVEN`: `lsbit != 0,4` and `!(lsbit & 1)` → force bit0 **clear** (includes `-4`, `INT_MIN`).
* **K — candidate-clamp shape**, driven by `uni & 7` (mutually exclusive; "both clamped" is unreachable):
  * `K0`  : `uni & 7 == 0` → `uni-1` borrows past bit 2, so `uni2` is clamped to `uni`.
  * `K7`  : `uni & 7 == 7` → `uni+1` carries past bit 2, so `uni1` is clamped to `uni`.
  * `KMID`: `uni & 7 ∈ 1..=6` → neither candidate clamped.
* **G — `uni & 8`** (`S0` / `S8`): selects whether `diff` is negated. Bit 3 is identical across `uni`/`uni1`/`uni2` after clamping (clamp guarantees it, and `L*` only touches bit 0), so this is a single shared axis.
* **T — `step` class**: `0`, small positive, small negative, magnitude large enough to overflow `(2*(uni&7)+1)*step` (`> INT_MAX/15`), `INT_MAX`, `INT_MIN`.
* **W — winner selection** (`d0` kept / `d1` wins / `d2` wins / both `d1<d0` and `d2<d0` / exact ties): both comparisons are against the **original** `d0`, and `d2` is tested last, so when both beat `d0` the result is `uni2` even if `d1 < d2`.
* **V — value shape of `pred`/`tgt`/`tgt2`**: near-zero, `tgt2 == tgt`, `tgt2` far from `tgt` (so the `d3 >> 5` tiebreak term dominates), and `i32` extremes that wrap.
* **N — sign of `uni`**: negative `uni` makes `uni >> 1` / `uni >> 2` in the `L4` branch arithmetic (sign-propagating) shifts.

## Configuration rows

Cross-product `L × K × G` (24 rows, all reachable and all treated differently by
the C), then the `T`, `W`, `V`, `N` classes that the code additionally branches
on. Every row is driven with **many randomized inputs** (fixed-seed xorshift64\*
PRNG in `translation/tests/differential.rs`) over the free arguments, never a
single hand-picked value.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `encode_quant` | L0 · K0 · S0 — `lsbit=0`, `uni&15 == 0`; `step`/`pred`/`tgt`/`tgt2` randomized | [x] |
| 2 | `encode_quant` | L0 · K0 · S8 — `lsbit=0`, `uni&15 == 8` | [x] |
| 3 | `encode_quant` | L0 · K7 · S0 — `lsbit=0`, `uni&15 == 7` | [x] |
| 4 | `encode_quant` | L0 · K7 · S8 — `lsbit=0`, `uni&15 == 15` | [x] |
| 5 | `encode_quant` | L0 · KMID · S0 — `lsbit=0`, `uni&15 ∈ 1..=6` | [x] |
| 6 | `encode_quant` | L0 · KMID · S8 — `lsbit=0`, `uni&15 ∈ 9..=14` | [x] |
| 7 | `encode_quant` | L4 · K0 · S0 — `lsbit=4`, `uni&15 == 0` | [x] |
| 8 | `encode_quant` | L4 · K0 · S8 — `lsbit=4`, `uni&15 == 8` | [x] |
| 9 | `encode_quant` | L4 · K7 · S0 — `lsbit=4`, `uni&15 == 7` | [x] |
| 10 | `encode_quant` | L4 · K7 · S8 — `lsbit=4`, `uni&15 == 15` | [x] |
| 11 | `encode_quant` | L4 · KMID · S0 — `lsbit=4`, `uni&15 ∈ 1..=6` | [x] |
| 12 | `encode_quant` | L4 · KMID · S8 — `lsbit=4`, `uni&15 ∈ 9..=14` | [x] |
| 13 | `encode_quant` | LODD · K0 · S0 — odd `lsbit ∉ {0,4}`, `uni&15 == 0` | [x] |
| 14 | `encode_quant` | LODD · K0 · S8 — odd `lsbit`, `uni&15 == 8` | [x] |
| 15 | `encode_quant` | LODD · K7 · S0 — odd `lsbit`, `uni&15 == 7` | [x] |
| 16 | `encode_quant` | LODD · K7 · S8 — odd `lsbit`, `uni&15 == 15` | [x] |
| 17 | `encode_quant` | LODD · KMID · S0 — odd `lsbit`, `uni&15 ∈ 1..=6` | [x] |
| 18 | `encode_quant` | LODD · KMID · S8 — odd `lsbit`, `uni&15 ∈ 9..=14` | [x] |
| 19 | `encode_quant` | LEVEN · K0 · S0 — even `lsbit ∉ {0,4}`, `uni&15 == 0` | [x] |
| 20 | `encode_quant` | LEVEN · K0 · S8 — even `lsbit`, `uni&15 == 8` | [x] |
| 21 | `encode_quant` | LEVEN · K7 · S0 — even `lsbit`, `uni&15 == 7` | [x] |
| 22 | `encode_quant` | LEVEN · K7 · S8 — even `lsbit`, `uni&15 == 15` | [x] |
| 23 | `encode_quant` | LEVEN · KMID · S0 — even `lsbit`, `uni&15 ∈ 1..=6` | [x] |
| 24 | `encode_quant` | LEVEN · KMID · S8 — even `lsbit`, `uni&15 ∈ 9..=14` | [x] |
| 25 | `encode_quant` | T=`step == 0` (degenerate quantizer: all three predictions equal `pred`) × all four `L` modes | [x] |
| 26 | `encode_quant` | T=small positive `step ∈ 1..=1024` (nominal codec range) × all `L`, `uni ∈ 0..=15` | [x] |
| 27 | `encode_quant` | T=small negative `step ∈ -1024..=-1` (`/8` truncation toward zero on a negative numerator) | [x] |
| 28 | `encode_quant` | T=`step` magnitude `> INT_MAX/15` → `(2*(uni&7)+1)*step` wraps | [x] |
| 29 | `encode_quant` | T=`step == INT_MAX` × all `L` × `uni ∈ 0..=15` | [x] |
| 30 | `encode_quant` | T=`step == INT_MIN` × all `L` × `uni ∈ 0..=15` | [x] |
| 31 | `encode_quant` | N=negative `uni` (arithmetic `>>1`/`>>2`) specifically under L4, plus under L0/LODD/LEVEN | [x] |
| 32 | `encode_quant` | `uni ∈ {INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` — `uni±1` wraps, clamp guard interaction | [x] |
| 33 | `encode_quant` | W=`d0` strictly best (`tgt` aligned on the `uni` prediction) → returns `uni` | [x] |
| 34 | `encode_quant` | W=`d1 < d0` only → returns `uni1` | [x] |
| 35 | `encode_quant` | W=`d2 < d0` only → returns `uni2` | [x] |
| 36 | `encode_quant` | W=**both** `d1 < d0` and `d2 < d0` → C returns `uni2` even when `d1 < d2` (quirk) | [x] |
| 37 | `encode_quant` | W=exact ties `d1 == d0` / `d2 == d0` → strict `<` keeps `uni` | [x] |
| 38 | `encode_quant` | V=`tgt2 == tgt` (the `d3 >> 5` secondary term mirrors the primary) | [x] |
| 39 | `encode_quant` | V=`tgt2` far from `tgt` so `d3 >> 5` dominates and flips the winner | [x] |
| 40 | `encode_quant` | V=`pred`/`tgt`/`tgt2` at `i32` extremes → `pred+diff`, `tgt-p0`, `d0+(d3>>5)` all wrap; `d ^ (d>>31)` maps `INT_MIN → INT_MAX` | [x] |
| 41 | `encode_quant` | unconstrained: all six arguments uniformly random over the full `i32` range | [x] |
| 42 | `encode_quant` | exhaustive nominal domain: every `uni ∈ 0..=15` × every `lsbit ∈ 0..=8`, randomized `step`/`pred`/`tgt`/`tgt2` | [x] |
| 43 | `encode_quant` | exhaustive `lsbit ∈ -16..=16` (covers `0`, `4`, negative odd/even, `-4`) × `uni ∈ 0..=15` | [x] |
| 44 | `encode_quant` | exhaustive low-bit shape: `uni ∈ -32..=32` (all `uni&15` patterns in both signs) × all `L` modes | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table → one configuration only.
All 44 rows are executed under `--no-default-features`, the default feature set,
and `--all-features` by `run_all_feature_combos.sh`.

## Finding: row 36 is only reachable through integer wraparound

Row 36 (`d1 < d0 && d2 < d0`, where the C returns `uni2` even when `uni1` is the
better candidate) is **unreachable with ordinary values**. For non-wrapping
inputs the distance is a convex function of the candidate index — it is the sum
of two V-shaped terms, `absish(tgt - p)` and `absish(tgt2 - p) >> 5`, composed
with the monotone index → prediction map (the `lsbit == 4` remap
`k → {0,0,2,2,4,4,7,7}` is monotone too) — so the middle candidate can never be
strictly worse than *both* neighbours.

It becomes reachable only once the `int` arithmetic wraps, which requires
`pred`/`tgt` clustered at one end of the `i32` range and `tgt2` at the other.
Measured hit rates:

| sampling strategy | hits |
|---|---|
| uniform over the full `i32^6` domain, 40M samples | **0** |
| small `step` + far `tgt2` (hunting the `>>5` non-convexity), 40M samples | **0** |
| exhaustive small-value grid (`uni`<64 × `step`,`tgt`∈±80 × `tgt2` swept), ~1.4G | **0** |
| near-overflow values in every slot, 40M samples | 23 |
| focused: `pred`,`tgt ≈ INT_MIN`, `tgt2 ≈ INT_MAX`, `step > 2^24`, 20M samples | **945,405 (≈4.7%)** |
| mirror region (`pred`,`tgt ≈ INT_MAX`, `tgt2 ≈ INT_MIN`), 20M samples | 944,589 |

The test therefore uses a dedicated generator for that region plus six
hardcoded witnesses, and separately asserts coverage of the sub-case where
`d1 < d2` (so `uni1` is genuinely better yet `uni2` still wins) — that sub-case
is the only thing that distinguishes the C's comparison order from the
swapped order. `mutation_check.sh` confirms it: the "swap selection order"
mutant is detected, which is possible *only* if row 36 is truly exercised.

## Validation of this table

`mutation_check.sh` injects 19 distinct plausible mistranslations (mask changes,
shift-width changes, `<` vs `<=`, dropped branches, saturating instead of
wrapping arithmetic, swapped comparison order, wrong operand) and asserts that
the suite detects **every** one. It also pins one *equivalent* mutant
(arithmetic vs logical shift underneath a `& 1` mask, which is provably
unobservable) and asserts it is correctly **not** flagged.

Result: 19/19 real bugs caught, 0 missed.

## Row → test mapping (all in `translation/tests/differential.rs`)

| rows | test |
|---|---|
| 1–24 | `cfg_rows_1_to_24_lsmode_x_clamp_x_bit3` |
| 25 | `cfg_row_25_step_zero` |
| 26 | `cfg_row_26_step_small_positive` |
| 27 | `cfg_row_27_step_small_negative` |
| 28 | `cfg_row_28_step_multiply_overflow` |
| 29, 30 | `cfg_rows_29_30_step_extremes` |
| 31 | `cfg_row_31_negative_uni` |
| 32 | `cfg_row_32_uni_boundaries` |
| 33–36 | `cfg_rows_33_to_36_winner_selection` |
| 37 | `cfg_row_37_ties` |
| 38 | `cfg_row_38_tgt2_equals_tgt` |
| 39 | `cfg_row_39_tgt2_far_from_tgt` |
| 40 | `cfg_row_40_extreme_values_wraparound` |
| 41 | `cfg_row_41_fully_random_full_i32` |
| 42 | `cfg_row_42_exhaustive_nominal_domain` |
| 43 | `cfg_row_43_exhaustive_lsbit_signed` |
| 44 | `cfg_row_44_exhaustive_low_bit_shapes` |

Two additional cross-cutting tests back the whole table up:

* `sweep_dense_exhaustive_projection` — **45,664,560** differential call pairs
  over a dense contiguous projection (`uni ∈ -1..=16` × 8 `lsbit`
  representatives × `step ∈ -72..=72` × `tgt ∈ -40..=40` × 9 `tgt2` × 3 `pred`).
  Contiguous rather than sampled, which is what catches off-by-one errors at the
  `/8` quantization and `>>5` tiebreak boundaries.
* `sweep_large_random_sample` — **21,320,000** differential call pairs: uniform
  over the full `i32^6` domain, boundary-biased, and "one axis pinned to an
  extreme, rest random" for each of the six arguments.

Total: ≈67M differential call pairs (≈134M FFI calls) in 2.6 s. Both sweeps are
time-budgeted (`DIFF_SWEEP_SECS`, default 120 s) so they cannot run away.

## Harness integrity note

`cargo test` does **not** build a `crate-type = ["cdylib"]` library, because no
test target links against it. An early version of the harness silently fell back
to a stale `target/release` artifact, which made the suite pass even with a
deliberately injected bug. The loader now builds the cdylib for the running
profile itself and refuses to run against a `.so` older than any file in `src/`
(`assert_not_stale`). `mutation_check.sh` is what surfaced this.
