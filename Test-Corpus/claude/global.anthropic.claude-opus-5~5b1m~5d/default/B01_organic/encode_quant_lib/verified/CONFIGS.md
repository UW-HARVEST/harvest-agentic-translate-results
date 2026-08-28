# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from the branches `c_src/src/lib.c` actually takes.

## Public entry points (full set, lowest level included)

`c_src/include/lib.h` exposes exactly one entry point, and it *is* the lowest
level one — there is no convenience/one-shot wrapper layered on top of anything:

| entry point | signature |
|---|---|
| `encode_quant` | `int encode_quant(int uni, int step, int pred, int tgt, int tgt2, int lsbit)` |

There is no init/config/teardown object, no setter API and no global state
(`grep` for `static`, `extern`, `struct`, `enum` in the C: 0 hits). All
configuration is therefore carried *in the six `int` arguments*, so the
configuration axes below are exactly the argument-value classes the code
branches on.

## Axes the C actually branches on

**Axis L — the `lsbit` mode selector** (lines 12/13/20, a 4-way switch):

| class | value(s) | C branch taken |
|---|---|---|
| L0 | `0` | whole `if (lsbit)` block skipped — `uni/uni1/uni2` untouched |
| L1 | `4` | dither branch: clear bit0, then OR in `(u>>1)&(u>>2)&1` |
| L2 | `1` | `lsbit & 1` → force bit0 = 1 |
| L3 | `3` | `lsbit & 1` → force bit0 = 1 (odd, >1) |
| L4 | `5` | `lsbit & 1` → force bit0 = 1 (odd, one step past `4`) |
| L5 | `2` | else → clear bit0 (even, one step before `4`) |
| L6 | `6` | else → clear bit0 (even, one step past `4`) |
| L7 | `8` | else → clear bit0 (even, larger) |
| L8 | `-1` | negative odd → force bit0 = 1 (two's-complement `&1`) |
| L9 | `-4` | negative even → clear bit0 (note: `-4 != 4`, so NOT the dither branch) |
| L10 | `INT_MAX` | odd extreme → force bit0 = 1 |
| L11 | `INT_MIN` | even extreme → clear bit0 |
| L12 | any random `i32` | whichever branch the value selects |

**Axis U — the `uni` shape** (lines 8/10 candidate clamping, and lines 31/37/43
sign-of-`diff` via bit 3):

| class | shape | C branch significance |
|---|---|---|
| U0 | `uni == 0` (`uni&7==0`, bit3=0) | `uni-1` crosses the 3-bit field → `uni2 = uni` (clamped) |
| U1 | `uni == 8` (`uni&7==0`, bit3=1) | `uni2` clamped **and** `diff` negated |
| U2 | `uni == 7` (`uni&7==7`, bit3=0) | `uni+1` crosses the field → `uni1 = uni` (clamped) |
| U3 | `uni == 15` (`uni&7==7`, bit3=1) | `uni1` clamped **and** `diff` negated |
| U4 | `uni ∈ {1..6}` | neither candidate clamped, `diff` positive for all three |
| U5 | `uni ∈ {9..14}` | neither clamped, `diff` negated for all three |
| U6 | `uni ∈ 0..=15` random | canonical 4-bit domain, all of the above mixed |
| U7 | `uni` positive with high bits set (`uni & ~15 != 0`) | high bits survive the clamp test and are returned verbatim |
| U8 | `uni` negative random | `uni & 7` / `uni & 8` on a negative value; negative `>>` in the L1 dither |
| U9 | `uni == INT_MAX` | `uni+1` **overflows**; clamp guard then restores `uni1 = uni` |
| U10 | `uni == INT_MIN` | `uni-1` **overflows**; clamp guard then restores `uni2 = uni` |
| U11 | `uni` any random `i32` | unconstrained |

**Axis V — the numeric shape of `step` / `pred` / `tgt` / `tgt2`** (lines 30–56:
the `*step` product, the `/8` truncating division, the `d ^ (d>>31)` absolute
value, and the `d3>>5` secondary-target penalty):

| class | shape | C branch/edge significance |
|---|---|---|
| V0 | `step ∈ 1..=255`, `pred/tgt/tgt2 ∈ -32768..=32767` | typical codec range, no overflow anywhere |
| V1 | `step ∈ 0..=7` | `(2*(uni&7)+1)*step / 8` truncates to `0` for small multipliers → candidates tie |
| V2 | `step == 0` | all three `diff == 0` → all three `p` equal → both `<` tests false |
| V3 | `step ∈ -255..=-1` | negative product; `/8` truncates **toward zero**; `uni&8` sign flip inverts |
| V4 | `step ∈ {INT_MAX, INT_MIN, 0x1000_0000, 0x7FFF_FFF8}` | `*step` **overflows** for multipliers ≥ 3; `-diff` may hit `INT_MIN` |
| V5 | `tgt2 == tgt` | the `d3>>5` penalty is a scaled copy of the primary distortion |
| V6 | `tgt2` far from `pred` (`|tgt2-pred| > 2^26`) | `d3>>5` **dominates** `d0/d1/d2` and drives the selection; also overflows the `+=` |
| V7 | all params drawn from `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}` | every arithmetic step overflows/saturates simultaneously |
| V8 | all params fully random `i32` | unconstrained |

**Axis S — the candidate-selection outcome** (lines 57–61). Both comparisons are
against the *original* `d0`, so this is a 4-outcome axis, not 3:

| class | condition | returned value |
|---|---|---|
| S0 | `d1 >= d0` and `d2 >= d0` | `uni` (post-lsbit) |
| S1 | `d1 < d0` and `d2 >= d0` | `uni1` |
| S2 | `d1 >= d0` and `d2 < d0` | `uni2` |
| S3 | `d1 < d0` **and** `d2 < d0` | `uni2` — the second `if` overwrites the first; **quirk to preserve** |

## Configuration-surface table

One row per meaningful combination. Every row is driven with many randomized
inputs (fixed seed `0x5EED_C0DE_1234_5678`, SplitMix64) over the axes it does not
pin, and both libraries are called through their `.so` exports and compared
byte-for-byte. Rows live in `translation/tests/phase_b_configs.rs`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `encode_quant` | L0 (`lsbit=0`) × U/V randomized | [x] |
| 2 | `encode_quant` | L1 (`lsbit=4`, dither) × U/V randomized | [x] |
| 3 | `encode_quant` | L2 (`lsbit=1`) × U/V randomized | [x] |
| 4 | `encode_quant` | L3 (`lsbit=3`) × U/V randomized | [x] |
| 5 | `encode_quant` | L4 (`lsbit=5`) × U/V randomized | [x] |
| 6 | `encode_quant` | L5 (`lsbit=2`) × U/V randomized | [x] |
| 7 | `encode_quant` | L6 (`lsbit=6`) × U/V randomized | [x] |
| 8 | `encode_quant` | L7 (`lsbit=8`) × U/V randomized | [x] |
| 9 | `encode_quant` | L8 (`lsbit=-1`, negative odd) × U/V randomized | [x] |
| 10 | `encode_quant` | L9 (`lsbit=-4`, negative even — not the dither branch) × U/V randomized | [x] |
| 11 | `encode_quant` | L10 (`lsbit=INT_MAX`) × U/V randomized | [x] |
| 12 | `encode_quant` | L11 (`lsbit=INT_MIN`) × U/V randomized | [x] |
| 13 | `encode_quant` | L12 (`lsbit` fully random) × U/V randomized | [x] |
| 14 | `encode_quant` | U0 (`uni=0`, `uni2` clamped) × L/V randomized | [x] |
| 15 | `encode_quant` | U1 (`uni=8`, clamped + negated `diff`) × L/V randomized | [x] |
| 16 | `encode_quant` | U2 (`uni=7`, `uni1` clamped) × L/V randomized | [x] |
| 17 | `encode_quant` | U3 (`uni=15`, clamped + negated `diff`) × L/V randomized | [x] |
| 18 | `encode_quant` | U4 (`uni ∈ 1..6`, no clamp, positive `diff`) × L/V randomized | [x] |
| 19 | `encode_quant` | U5 (`uni ∈ 9..14`, no clamp, negated `diff`) × L/V randomized | [x] |
| 20 | `encode_quant` | U6 (`uni ∈ 0..=15`) × L/V randomized | [x] |
| 21 | `encode_quant` | U7 (`uni` positive, high bits set) × L/V randomized | [x] |
| 22 | `encode_quant` | U8 (`uni` negative) × L/V randomized | [x] |
| 23 | `encode_quant` | U9 (`uni=INT_MAX`, `uni+1` overflow) × L/V randomized | [x] |
| 24 | `encode_quant` | U10 (`uni=INT_MIN`, `uni-1` overflow) × L/V randomized | [x] |
| 25 | `encode_quant` | U11 (`uni` fully random) × L/V randomized | [x] |
| 26 | `encode_quant` | V0 (typical codec range) × L/U randomized | [x] |
| 27 | `encode_quant` | V1 (`step ∈ 0..=7`, division truncates to 0) × L/U randomized | [x] |
| 28 | `encode_quant` | V2 (`step == 0`, all candidates tie) × L/U randomized | [x] |
| 29 | `encode_quant` | V3 (negative `step`) × L/U randomized | [x] |
| 30 | `encode_quant` | V4 (overflowing `step`) × L/U randomized | [x] |
| 31 | `encode_quant` | V5 (`tgt2 == tgt`) × L/U randomized | [x] |
| 32 | `encode_quant` | V6 (`tgt2` far — `d3>>5` dominates and overflows `+=`) × L/U randomized | [x] |
| 33 | `encode_quant` | V7 (all params at signed extremes) × L/U randomized | [x] |
| 34 | `encode_quant` | V8 (all params fully random) × L/U randomized | [x] |
| 35 | `encode_quant` | S0 — inputs that make `d0` the winner (`uni` returned) | [x] |
| 36 | `encode_quant` | S1 — inputs that make only `d1 < d0` (`uni1` returned) | [x] |
| 37 | `encode_quant` | S2 — inputs that make only `d2 < d0` (`uni2` returned) | [x] |
| 38 | `encode_quant` | S3 — inputs where `d1 < d0` **and** `d2 < d0` (quirk: `uni2` wins) | [x] |
| 39 | `encode_quant` | Exhaustive `uni ∈ 0..=15` × `lsbit ∈ 0..=8` × `step ∈ 0..=64`, `pred/tgt/tgt2` from a fixed representative set | [x] |
| 40 | `encode_quant` | Exhaustive `uni ∈ -16..=16` × `lsbit ∈ -8..=8` × randomized `step/pred/tgt/tgt2` | [x] |
| 41 | `encode_quant` | Exhaustive cross-product of `{INT_MIN, INT_MIN+1, -1, 0, 1, INT_MAX-1, INT_MAX}^6` (117 649 tuples) | [x] |
| 42 | `encode_quant` | Large unconstrained random fuzz, 1 048 576 tuples of 6 random `i32`s | [x] |
| 43 | `encode_quant` | **Full cross-product sweep**: every L class × every U class × every V class (13 × 12 × 9 = 1404 combinations), 128 randomized draws each | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table, so the default build is the
only configuration; all 43 rows above are additionally re-run under
`--no-default-features` by `scripts/check_all_features.sh`, in both the `debug`
and `release` profiles. The `debug` run matters independently: the `debug`
cdylib is compiled with `overflow-checks = on`, so any non-wrapping arithmetic
left in the translation would panic instead of silently wrapping.

## Beyond the table: exhaustive full-domain sweeps

`tests/exhaustive.rs` (marked `#[ignore]`; run with
`cargo test --release --test exhaustive -- --ignored`) goes past sampling and
sweeps entire 2^32 parameter domains, pinning the others:

| sweep | domain covered |
|---|---|
| `exhaustive_all_uni_values` | every one of the 2^32 `uni` values |
| `exhaustive_all_lsbit_values` | every one of the 2^32 `lsbit` values |
| `exhaustive_all_step_values` | every one of the 2^32 `step` values |
| `exhaustive_all_target_values` | every one of the 2^32 `tgt` values and every `tgt2` value |
| `exhaustive_uni_lsbit_joint_window` | the joint (`uni`, `lsbit`) square `[-512, 512]^2` x 8 `step` values |
| `exh_mult{01..15}_uni{0..15}[_neg]` | for EACH of the 8 multipliers `2*(uni&7)+1`, and each with bit 3 clear and set (16 sweeps), every one of the 2^32 `step` values — this is the joint (multiplier x step) coverage that a precedence/parenthesization error in the `((2*(uni&7)+1)*step)/8` expression would require |

`EXHAUSTIVE_STRIDE` subsamples for a fast pass; `EXHAUSTIVE_CONFIGS` caps the
number of pinned configurations so a genuine stride-1 sweep fits a time budget.

## Beyond the table: C compiler-configuration robustness

The C relies on signed-overflow (UB) and negative-right-shift
(implementation-defined) behaviour, so matching a single C build could be luck.
`tests/phase_d_optlevels.rs` rebuilds the **unmodified** `c_src/src/lib.c` into a
temp directory (nothing in `c_src/` is touched) with `gcc`, `clang` and `cc` at
`-O0 -O1 -O2 -O3 -Os -Ofast`, plus `-fwrapv`, `-fno-strict-overflow`,
`-fstrict-overflow` and `-march=native` — 30 variants — and diff-checks the Rust
`.so` against every one over ~214 000 cases spanning all axes.
