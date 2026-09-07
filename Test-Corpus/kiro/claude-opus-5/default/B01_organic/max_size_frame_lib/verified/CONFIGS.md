# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from the branches the C code actually takes.

## Axes the C distinguishes

`max_size_frame` has no runtime option struct, no global state, no flags, and no
`#ifdef`s. Its entire "configuration" is carried by the three value arguments,
and the code branches on exactly three predicates:

| axis | predicate in C | states | effect |
|------|----------------|--------|--------|
| A. channel mode | `channels != 2` / `channels == 2` | stereo (`== 2`) vs non-stereo (`!= 2`) | selects which of the three bit-count terms contribute. Non-stereo: term1 only. Stereo: term2 + term3 only. |
| B. depth mode | `bitdepth != 32` | `bitdepth == 32` vs `bitdepth != 32` | only read inside term3, i.e. only observable when `channels == 2`. Adds a `+1` per-sample correction unless the depth is exactly 32. |
| C. input shape | (no predicate — value magnitude) | zero / one / small / typical / large / 2^32-wrapping | selects whether the wrapping multiplies and the `+7` stay in range or overflow, and whether `/ 8` truncates. |

Cross-product = 2 (A) x 2 (B) x shape classes, pruned to the combinations the
code actually treats differently. Note that B is *dead* when `channels != 2`
(the `bitdepth != 32` result is multiplied by `(channels == 2)` = 0), so the
`channels != 2` rows must still be tested at both depth states to confirm the
Rust also treats it as dead rather than applying the correction.

## Public entry points

There is exactly one public entry point, and it is simultaneously the
lowest-level and the highest-level one — the header declares no other function,
and `lib.c` defines no internal helpers. There is no convenience wrapper layer
to bypass and no composed pipeline to drive; the "full operation end to end" is
this single call. Every row below therefore calls `max_size_frame` directly
through the `.so` export in both libraries.

There is also no binary/driver target: `c_src/CMakeLists.txt` declares only
`add_library(... SHARED src/lib.c)`, so the stdout-comparison item of the
completion gate is not applicable.

## Configuration rows

Each row is exercised with **many randomized inputs** (fixed seed, deterministic
xorshift PRNG) for the free arguments, plus the row's pinned arguments, and both
libraries' outputs are compared for byte equality.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|-------------------------------------------|-----|
| C1 | `max_size_frame` | non-stereo (`channels != 2`), `bitdepth != 32`, small typical shape: `channels` in 1..=8 \ {2}, `bitdepth` in {8,12,16,20,24}, `blocksize` in 1..=4608 | [x] |
| C2 | `max_size_frame` | non-stereo, `bitdepth == 32` exactly (dead-predicate check), `channels` in 1..=8 \ {2}, `blocksize` in 1..=4608 | [x] |
| C3 | `max_size_frame` | stereo (`channels == 2`), `bitdepth != 32`, small typical shape: `bitdepth` in {8,12,16,20,24}, `blocksize` in 1..=4608 | [x] |
| C4 | `max_size_frame` | stereo, `bitdepth == 32` exactly (the `+1` correction suppressed), `blocksize` in 1..=4608 | [x] |
| C5 | `max_size_frame` | `blocksize == 0` (empty block) x both channel modes x both depth modes, `channels`/`bitdepth` randomized | [x] |
| C6 | `max_size_frame` | `channels == 0` (empty count, a non-stereo sub-case that also zeroes term1) x both depth modes, `blocksize`/`bitdepth` randomized | [x] |
| C7 | `max_size_frame` | `channels == 1` (single, one below the stereo boundary) x both depth modes, randomized `blocksize`/`bitdepth` | [x] |
| C8 | `max_size_frame` | `channels == 3` (one above the stereo boundary) x both depth modes, randomized `blocksize`/`bitdepth` | [x] |
| C9 | `max_size_frame` | `bitdepth == 0` x both channel modes, randomized `blocksize`/`channels` | [x] |
| C10 | `max_size_frame` | `bitdepth` in {31, 33} (one step either side of the 32 boundary) x both channel modes, randomized `blocksize`/`channels` | [x] |
| C11 | `max_size_frame` | `blocksize == 1` (single sample, division truncates hard) x both channel modes x both depth modes | [x] |
| C12 | `max_size_frame` | many channels (`channels` in 3..=255, non-stereo) with typical depth/blocksize — exercises the `channels * (channels != 2)` product and the `18 + channels` tail | [x] |
| C13 | `max_size_frame` | large but non-wrapping shape: `blocksize` in 1..=65535, `bitdepth` in {8,16,24,32}, `channels` in {1,2,3,4} — near but under 2^32 | [x] |
| C14 | `max_size_frame` | overflow shape, non-stereo: `blocksize`/`bitdepth`/`channels` chosen so `blocksize*bitdepth*channels` exceeds 2^32 and wraps | [x] |
| C15 | `max_size_frame` | overflow shape, stereo: `blocksize`/`bitdepth` chosen so term2 + term3 wraps | [x] |
| C16 | `max_size_frame` | `+7` overflow shape: inner sum lands in `UINT32_MAX-6 ..= UINT32_MAX` so the `+7` itself wraps past zero | [x] |
| C17 | `max_size_frame` | final-addition overflow shape: `bytes + channels + 18` wraps | [x] |
| C18 | `max_size_frame` | `UINT32_MAX` in each argument position independently, other two randomized | [x] |
| C19 | `max_size_frame` | powers of two on every axis (`blocksize`, `bitdepth`, `channels` each drawn from `1<<k`, k in 0..32) — cross-product of alignment-sensitive shapes | [x] |
| C20 | `max_size_frame` | unconstrained uniform-random fuzz over the full `u32^3` domain (dominated by non-stereo, huge, wrapping inputs) — the catch-all row | [x] |
| C21 | `max_size_frame` | stereo-forced uniform-random fuzz: `channels` pinned to 2, `blocksize`/`bitdepth` full-range random (stereo is rare under uniform fuzz, so it needs its own row) | [x] |
| C22 | `max_size_frame` | exhaustive small cube: every `blocksize` in 0..=63 x every `channels` in 0..=15 x every `bitdepth` in 0..=39 (covers all three predicates and both sides of the 32 boundary densely, with no randomness) | [x] |

## Gate status

All 22 rows pass across their randomized inputs. See
`translation/tests/differential.rs`.

## Verification evidence

Every row is a `cfg_c*` test in `translation/tests/differential.rs`. Randomized
rows use a fixed-seed xorshift64\* PRNG (seed = the row number, e.g. `0xC001`)
with 20 000 iterations per row, so runs are reproducible.

Coverage totals actually executed:

* 22 configuration rows, all passing.
* Row C19 is a full cross-product of `{0} ∪ {1<<k : k∈0..32} ∪ {UINT32_MAX}`
  on all three axes (34³ = 39 304 triples), plus a one-off-power variant of each.
* Row C22 is exhaustive over `blocksize ∈ 0..=63 × channels ∈ 0..=15 ×
  bitdepth ∈ 0..=39` (40 960 triples, no randomness), which densely covers both
  sides of the `channels == 2` and `bitdepth == 32` boundaries.
* Row C12 additionally sweeps every `channels ∈ 0..=255` against seven depths.
* Rows C14–C17 assert that their inputs genuinely overflow before comparing, so
  the overflow rows cannot silently degrade into in-range tests (e.g. C14
  asserts `blocksize·bitdepth·channels > UINT32_MAX` on every iteration, and
  E13 asserts that overflowing draws dominate the row).

### Heavy soak

`heavy_fuzz_full_domain` was run at `HEAVY_FUZZ_ITERS=200000000`: 200 M
iterations × 5 input shapes = **1 000 000 000 paired C/Rust calls**, all
matching. Runtime scaled linearly with the iteration count
(2 M → 0.07 s, 20 M → 0.68 s, 200 M → 6.78 s), which confirms the loop is
really executing and was not elided by the optimiser.

### Why this is exhaustive in effect

The full input domain is 2^96 triples and cannot be enumerated. However, all
arithmetic is in `Z/2^32`, a commutative ring, and the Rust source applies
`wrapping_mul`/`wrapping_add` to the same operands in the same structure as the
C. The two compilers reassociate differently (GCC factors
`term1 + term2` into `blocksize·bitdepth·(channels·(channels≠2) + (channels=2))`,
LLVM uses `cmov`), but reassociation is exact in `Z/2^32`, so the two are
algebraically identical rather than merely empirically close. The randomized and
exhaustive rows above confirm this on ~10^9 concrete points, including every
branch state and every overflow class.

### No binary target

`c_src/CMakeLists.txt` declares only `add_library(${project_name} SHARED src/lib.c)`
and two `install()` rules — there is no `add_executable`. The Rust crate is
`crate-type = ["cdylib"]` with no `[[bin]]`. The stdout-comparison item of the
completion gate is therefore not applicable.

### Feature combinations

The crate declares no `[features]` section, no optional dependencies, and
contains no `cfg(feature = ...)` in `src/` or `tests/` (verified via `grep` and
`cargo metadata`, which reports an empty feature map). The default configuration
is the only one that exists. `translation/verify_all.sh` enumerates the feature
powerset mechanically and runs the suite for each combination × `{debug,
release}` profile; with no features that is `(default)` and
`(--no-default-features)`, and all four runs pass:

```
=== profile=debug   features=(default features)        ===  PASS
=== profile=debug   features=(--no-default-features)   ===  PASS
=== profile=release features=(default features)        ===  PASS
=== profile=release features=(--no-default-features)   ===  PASS
=== Symbol parity (nm -D) ===  symbol sets IDENTICAL
ALL CONFIGURATIONS PASS
```

The debug profile is a meaningful extra axis: it enables overflow checks, so a
plain `+`/`*` left anywhere in the translation would panic there while the
release build silently wrapped. It does not.
