# CONFIGS.md — configuration-surface table

Derived **mechanically** from the branches in `c_src/src/lib.c` and the full
public header `c_src/include/lib.h`.

## Axis enumeration

### Axis 1 — runtime options / modes / flags

```
$ grep -cE '#ifdef|#if |switch|extern|static [^c]|set[A-Z_]|_init|struct' c_src/src/lib.c
0
```

**None.** The library has:
* no `#ifdef` / `#if` compile-time switches,
* no `switch` statements,
* no global mutable state, no init/config/setter function,
* no struct or context handle,
* exactly **one** public entry point, `float pow43(int)`, and it *is* the
  lowest-level entry point — there is no convenience wrapper and nothing beneath
  it. The call hierarchy has depth 1.

So the option axis is a single point, and Phase B must drive `pow43` directly.

### Axis 2 — Cargo feature combinations

```
$ grep -n '\[features\]' translation/Cargo.toml
(no match)
```

`translation/Cargo.toml` declares **no `[features]` section** and no optional
dependencies, so the complete set of feature combinations is exactly one: the
empty/default set. `--no-default-features` and the default build are the same
build. Both are still run in `run_all.sh` for completeness.

### Axis 3 — input shapes the C actually special-cases

The C branches on, in source order:

| source line | branch predicate | state it toggles |
|---|---|---|
| 37 | `x < 129` | early return, direct table index `16 + x`, no arithmetic |
| 40 | `x < 1024` | `mult = 16` (else stays `256`) **and** `x <<= 3` |
| 43 | `2 * x & 64` | `sign ∈ {0, 64}` — flips the sign of the interpolation offset |
| 44 | `(x & 63)` | interpolation fraction numerator; `== 0` is the exact-grid-point case |
| 46 | `(x + sign) >> 6` | which table entry is interpolated from |

Derived shape axes: **branch(3) × sign(2) × `x&63`==0 or not(2) × position
(first / last / interior / random)**, plus the two table sub-regions the data
itself distinguishes (indices 0–15 hold *negative* values, 16–144 hold
non-negative values), plus the UB regions below and above the domain.

## The table

One row per combination the C treats differently. Every row is driven through
BOTH `.so` exports and compared bit-for-bit (`f32::to_bits`), over **many**
randomized inputs (fixed seed `0x5EED_1234`, SplitMix64) *and*, because the
defined domain is only 8240 values wide, **exhaustively**.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1  | `pow43` | branch 1 (`x < 129`), negative table region: `x ∈ [-16, -1]` → indices 0–15, all table values negative or zero | [x] |
| 2  | `pow43` | branch 1, `x == -16` (lowest in-bounds index, index 0, value `+0.0f`) — exact lower boundary | [x] |
| 3  | `pow43` | branch 1, `x == 0` → index 16, value `+0.0f` (second zero in the table) | [x] |
| 4  | `pow43` | branch 1, non-negative table region: `x ∈ [1, 128]` → indices 17–144 | [x] |
| 5  | `pow43` | branch 1, `x == 128` (upper boundary of branch 1, last table entry, index 144) | [x] |
| 6  | `pow43` | branch 1, randomized `x` drawn uniformly from `[-16, 128]` | [x] |
| 7  | `pow43` | branch 2 (`129 <= x < 1024`, `mult = 16`, `x <<= 3`), `x == 129` (lower boundary) | [x] |
| 8  | `pow43` | branch 2, `x == 1023` (upper boundary; post-shift `x = 8184`, index 144, `sign = 64`) | [x] |
| 9  | `pow43` | branch 2, `sign == 0` (post-shift bit 5 clear) | [x] |
| 10 | `pow43` | branch 2, `sign == 64` (post-shift bit 5 set) | [x] |
| 11 | `pow43` | branch 2, post-shift `x & 63 == 0` (exact grid point ⇒ `frac == 0` or `-64/den`) | [x] |
| 12 | `pow43` | branch 2, post-shift `x & 63 != 0` (interpolating) | [x] |
| 13 | `pow43` | branch 2, randomized `x` drawn uniformly from `[129, 1023]` | [x] |
| 14 | `pow43` | branch 2, **exhaustive** over all of `[129, 1023]` | [x] |
| 15 | `pow43` | branch 3 (`x >= 1024`, `mult = 256`, no shift), `x == 1024` (lower boundary) | [x] |
| 16 | `pow43` | branch 3, `x == 8223` (largest in-bounds argument, index 144) | [x] |
| 17 | `pow43` | branch 3, `sign == 0` (bit 5 of `x` clear) | [x] |
| 18 | `pow43` | branch 3, `sign == 64` (bit 5 of `x` set) | [x] |
| 19 | `pow43` | branch 3, `x & 63 == 0` (multiple of 64 ⇒ `frac` numerator is `0 - sign`) | [x] |
| 20 | `pow43` | branch 3, `x & 63 == 63` (largest fraction within a segment) | [x] |
| 21 | `pow43` | branch 3, randomized `x` drawn uniformly from `[1024, 8223]` | [x] |
| 22 | `pow43` | branch 3, **exhaustive** over all of `[1024, 8223]` | [x] |
| 23 | `pow43` | branch-selector transitions driven as adjacent pairs: `(128,129)`, `(1023,1024)`, `(8223,8224)` — asserts the two sides of each `if` agree between C and Rust | [x] |
| 24 | `pow43` | **exhaustive** sweep of the entire defined domain `[-16, 8223]` (8240 values), all three branches and both `sign` states in one pass | [x] |
| 25 | `pow43` | sequential/stateful invocation: the whole domain called in randomized order, then again in ascending order, asserting results are order-independent (proves the C's `static` table is read-only and there is no hidden state) | [x] |
| 26 | `pow43` | LAME's real consumer domain `x ∈ [0, 8206]` driven end-to-end as a consumer would (dequantize a whole spectrum of 576 randomized indices per iteration, 200 iterations) | [x] |
| 27 | `pow43` | feature combination: default features (= no features; the crate declares none) | [x] |
| 28 | `pow43` | feature combination: `--no-default-features` (identical build; verified separately) | [x] |

## Coverage note

Rows 1–26 are implemented as `tests/phase_b_valid.rs::row01..row26`, one test
per row, each loading BOTH `.so`s through `libloading` and comparing
`f32::to_bits()` (so `+0.0` vs `-0.0` and NaN payloads cannot slip through).

Because the defined domain is only 8240 values wide, the property-style rows
(6, 13, 21) are *supplemented* by full exhaustive sweeps (rows 14, 22, 24), so
coverage of the valid input space is **100%, not sampled**: every one of the
8240 defined inputs is compared bit-for-bit. Rows 1–5, 7–12, 15–20 and 23
additionally pin the specific structural cases so that a regression reports the
axis it broke rather than just "some input differs".

Rows 27–28 are enforced by `run_all.sh`, which enumerates the feature power set
from `Cargo.toml` and runs `cargo check` + `cargo build` + `check_symbols.sh` +
`cargo test` for each combination in **both** the release and the debug profile.
The debug profile matters independently: it turns on Rust's integer-overflow
checks, so it proves the `wrapping_mul` / `wrapping_add` / `wrapping_shl`
transcription of the C's two's-complement arithmetic never traps.

Verified result: 4 combination × profile pairs, 47 tests each, 0 failures.
