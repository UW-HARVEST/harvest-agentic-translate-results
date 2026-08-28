# CONFIGS.md — Phase A configuration-surface table

Mechanically derived from the branches the C code actually takes.

## Axes found in the C source

**A. Entry points (all 10 exported symbols, low-level ones included — not just
the `collided` convenience wrapper).**

`c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Sub`, `c2Dot` (level 0/1 helpers) ·
`c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB` (level 2 predicates) ·
`collided` (level 3 dispatcher).

**B. Runtime options / modes.** The only option-like input in the public API is
the pair of `C2_TYPE` tags of `collided`, giving 4 valid modes
(`CIRCLE×CIRCLE`, `CIRCLE×AABB`, `AABB×CIRCLE` — note the C **swaps** the
arguments here (lib.c:88) — and `AABB×AABB`). There are no global flags, no
`#ifdef`s, no build-time options in `c_src` and **no `[features]` in
`Cargo.toml`**, so the default feature set is the only configuration
(see `feature_matrix.sh`).

**C. Data-shape / value classes the code branches on.**

| branch in the C | shapes it distinguishes |
|-----------------|-------------------------|
| `a.x > b.x ? a.x : b.x` (`c2Maxv`, `comiss`+`jbe`) | greater / less / equal / **unordered (NaN ⇒ takes `b`)** / `+0.0` vs `-0.0` (equal ⇒ takes `b`) |
| `a.x < b.x ? a.x : b.x` (`c2Minv`) | same four cases, mirrored |
| `c2Maxv(lo, c2Minv(a, hi))` (`c2Clampv`) | point below `lo`, inside, above `hi`, per-axis independently (9 regions); **inverted box (`lo > hi`) ⇒ `lo` wins** |
| `d2 < r2` (`c2CircletoCircle`, `c2CircletoAABB`, `comiss`+`seta`) | overlap / miss / **exact touch (`d2 == r2` ⇒ 0)** / unordered ⇒ 0 |
| `!(d0\|d1\|d2\|d3)` (`c2AABBtoAABB`) | each of the 4 separating-axis flags set independently (16 combinations), touching edges (`<` is strict), NaN ⇒ all flags 0 ⇒ returns 1 |
| `switch (typeA) / switch (typeB)` (`collided`) | the 4 valid tag pairs (+ the invalid tags in `ERRORS.md`) |
| IEEE-754 arithmetic in `c2Sub`/`c2Dot` (`subss`, `mulss`, `addss`) | finite, `±0`, subnormal, `±inf`, overflow-to-`inf`, cancellation, qNaN/sNaN payload propagation (operand order fixed by the `-O0` codegen: `mulss` dst=`a.x` / dst=`b.y`, `addss` dst=y-product) |

## Configuration table (one row per combination the C treats differently)

Every row is driven with **many randomized inputs** (fixed-seed PRNG in
`tests/common/mod.rs`, `ITERS` per row) plus the hand-picked edge vectors of its
class, and compared **bit-for-bit** between the C `.so` and the Rust `.so`.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite bit patterns, incl. `±0.0`, subnormals, `±inf`, qNaN/sNaN with distinct payloads (must be copied through untouched, never quieted) | [x] |
| 2 | `c2Maxv` | random finite pairs (all of `<`, `>`, `==` per lane) | [x] |
| 3 | `c2Maxv` | one/both lanes `NaN` (unordered ⇒ result must be `b`'s lane, payload unquieted), and `+0.0`/`-0.0` pairs in both orders | [x] |
| 4 | `c2Minv` | random finite pairs (all of `<`, `>`, `==` per lane) | [x] |
| 5 | `c2Minv` | one/both lanes `NaN`, `±0.0` pairs both orders | [x] |
| 6 | `c2Clampv` | well-formed box (`lo <= hi`), point in each of the 9 regions (below/inside/above per axis), randomized | [x] |
| 7 | `c2Clampv` | **inverted box** (`lo > hi` on one or both axes) — exercises the `Maxv(lo, …)` precedence | [x] |
| 8 | `c2Clampv` | degenerate box (`lo == hi`), and `±0.0` / `±inf` / `NaN` in `a`, `lo`, `hi` in every position | [x] |
| 9 | `c2Sub` | random finite pairs; exact cancellation (`a == b` ⇒ `+0.0`); `±0.0` combinations (`-0.0 - +0.0 == -0.0`) | [x] |
| 10 | `c2Sub` | overflow (`FLT_MAX - (-FLT_MAX) ⇒ inf`), `inf - inf ⇒ default qNaN`, sNaN operand (must be **quieted** by `subss`), subnormal results | [x] |
| 11 | `c2Dot` | random finite vectors, incl. products that overflow to `inf` and sums that cancel exactly | [x] |
| 12 | `c2Dot` | NaN payload propagation matrix: qNaN/sNaN in each of the 4 lanes, several distinct payloads, incl. `0 * inf` (⇒ default qNaN `0x7fc00000`) and `inf + -inf` | [x] |
| 13 | `c2CircletoCircle` | random circles, mixed overlap/miss (both branches of `d2 < r2`) | [x] |
| 14 | `c2CircletoCircle` | **exact touch**: centres exactly `A.r + B.r` apart (`d2 == r2` ⇒ must be `0`), and one ULP either side; zero radius; identical circles | [x] |
| 15 | `c2CircletoCircle` | negative radius (sum can be negative ⇒ `r2 = sum²` positive ⇒ can still report a hit); one radius `-r` cancelling the other (`r2 == 0`) | [x] |
| 16 | `c2CircletoCircle` | `±inf` centres/radii (`inf - inf` in `c2Sub`), `NaN` in any field (⇒ `0`), radii whose sum overflows to `inf`, `FLT_MAX` centres | [x] |
| 17 | `c2CircletoAABB` | point (circle centre) **inside** the box ⇒ clamp is identity ⇒ `d2 == 0` ⇒ hit iff `r != 0` | [x] |
| 18 | `c2CircletoAABB` | centre outside on an **edge** region (clamped on exactly one axis) — randomized over all 4 edges | [x] |
| 19 | `c2CircletoAABB` | centre outside in a **corner** region (clamped on both axes) — all 4 corners | [x] |
| 20 | `c2CircletoAABB` | **exact touch** on an edge and on a corner (`d2 == r²` ⇒ `0`), plus one ULP either side | [x] |
| 21 | `c2CircletoAABB` | degenerate box (`min == max`, zero area / zero-width or zero-height), zero-radius circle | [x] |
| 22 | `c2CircletoAABB` | **inverted box** (`min > max`), which makes the clamp return `min` | [x] |
| 23 | `c2CircletoAABB` | `±inf` / `NaN` / subnormal / `FLT_MAX` fields in the circle and the box (incl. `d2` overflow to `inf`) | [x] |
| 24 | `c2AABBtoAABB` | random boxes spanning all 16 combinations of the `d0..d3` separating flags (overlap, disjoint on x only, y only, both) | [x] |
| 25 | `c2AABBtoAABB` | **touching** boxes (`A.max.x == B.min.x`, etc. — strict `<` ⇒ counts as a hit), containment, identical boxes | [x] |
| 26 | `c2AABBtoAABB` | inverted / degenerate boxes, `±0.0` edges, `±inf` edges, `NaN` edges (all flags `0` ⇒ returns `1`) | [x] |
| 27 | `collided` | `typeA=CIRCLE, typeB=CIRCLE` — randomized, must equal `c2CircletoCircle(*A, *B)` and the C `.so` | [x] |
| 28 | `collided` | `typeA=CIRCLE, typeB=AABB` — randomized (circle read from `A`, box from `B`) | [x] |
| 29 | `collided` | `typeA=AABB, typeB=CIRCLE` — randomized; verifies the **argument swap** `c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)` | [x] |
| 30 | `collided` | `typeA=AABB, typeB=AABB` — randomized | [x] |
| 31 | `collided` | all 4 valid tag pairs driven with the **edge-case value classes** of rows 14-26 (touch/NaN/inf/inverted/degenerate) rather than plain random data | [x] |
| 32 | `collided` | aliasing: `A == B` (same pointer) for each valid tag pair; and unaligned buffers (`repr(packed)`-style odd offsets) | [x] |
| 33 | end-to-end pipeline | a fixed-seed scene of N circles and M boxes, all pairs tested through `collided` **and** through the level-0/1/2 helpers, comparing the whole result vector C vs Rust | [x] |

## Verification result (Phase B)

All 33 rows pass, in both profiles and under every feature combination:
`cargo test --test phase_b_valid` ⇒ **33 passed; 0 failed**
(`./run_all.sh`, `./feature_matrix.sh`).

Each row is driven with `ITERS = 4000` fixed-seed random inputs (plus its
hand-picked edge vectors), and results are compared on **bit patterns**
(`f32::to_bits`), so `NaN` payloads and the sign of zero are part of the
contract, not just numeric equality. Coverage of the branchy rows is asserted
inside the tests themselves rather than assumed:

* row 6 asserts all **9** clamp regions were hit;
* row 24 asserts all **16** separating-axis flag combinations were hit;
* rows 13, 27-30 assert both the hit and the miss branch were taken often;
* rows 27-30 additionally assert the dispatcher agrees with the corresponding
  low-level predicate *within each library*, which is what pins down the
  argument swap of the `AABB × CIRCLE` case (`c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)`).

### Where the interesting behaviour actually was

These are the details that plain "call it once with one input" tests would have
missed, and which the randomized bit-exact comparison confirms:

| behaviour | why it matters |
|---|---|
| `a > b ? a : b` returns **`b`** for `NaN` and for `+0.0` vs `-0.0` | `f32::max`/`f32::min` would be wrong here; the `nan_suppressing_max` / `nan_suppressing_min` mutants prove rows 3/5 catch it |
| `d2 < r2` is **strict** ⇒ an exactly touching pair does **not** collide | rows 14/20 construct exact touches (quantised radii, and 3-4-5 triples scaled by powers of two) plus one ULP either side |
| `c2Dot`'s `mulss`/`addss` operand order decides which `NaN` payload is returned | row 12 uses a 13×13×13×13 payload matrix; the `dot_operand_order` and `dot_sum_order` mutants prove it is checked |
| `!(d0\|d1\|d2\|d3)` returns **1** for all-`NaN` boxes | row 26 asserts this explicitly |
| `c2Clampv` = `Maxv(lo, Minv(a, hi))` ⇒ for an **inverted** box `lo` wins | rows 7/22 |

### Caveat: the C's own `NaN` payload output depends on its build flags

`c2Dot` is the only place where the C library is not stable across compiler
settings. The `.so` produced by `c_src/CMakeLists.txt` (no `CMAKE_BUILD_TYPE`,
hence `-O0`) computes:

```
mulss dst=a.x   (x product)      mulss dst=b.y   (y product)      addss dst=y product
```

while a `-O2` build reassociates to `mulss dst=a.x`, `mulss dst=a.y`,
`addss dst=x product`. The Rust reproduces the **`-O0` ordering**, i.e. the
build this `CMakeLists.txt` actually specifies, and is bit-identical to it for
all 4000+ randomized inputs and the full payload matrix.

Verified explicitly: building the same source with `gcc -O2` and pointing the
harness at it (`C_SO=… cargo test --test phase_b_valid`) leaves **32 of 33 rows
passing**, with only `row12_c2Dot_nan_payload_matrix` differing, and only for
inputs carrying **two or more distinct `NaN` payloads** — e.g.
`a=(0x7fc00000, 0x7fc00000)`, `b=(0x7fc00000, 0x7fc00001)` gives `0x7fc00001`
at `-O0` (matched by the Rust) and `0x7fc00000` at `-O2`. Any input with at most
one distinct `NaN` payload — which includes every non-`NaN` input — is identical
under either ordering, so this affects no realistic caller.

## Feature combinations (Phase D)

`Cargo.toml` declares **no `[features]`**, so the default feature set is the only
configuration. `./feature_matrix.sh` extracts the feature list from `Cargo.toml`
(so it keeps working if features are added later) and still runs the full suite
for `default` and `--no-default-features` × `dev`/`release`:

```
################ profile=dev       combo=default              33 + 14 + 4 passed
################ profile=dev       combo=--no-default-features 33 + 14 + 4 passed
################ profile=--release combo=default              33 + 14 + 4 passed
################ profile=--release combo=--no-default-features 33 + 14 + 4 passed
ALL FEATURE COMBINATIONS PASS
```

## Harness gotcha worth knowing

`cargo test` does **not** rebuild a `crate-type = ["cdylib"]` library, because no
test target links it — so after editing `src/lib.rs` the suite would `dlopen` the
**previous** `.so` and report a false pass. This actually happened during
verification. `tests/common/mod.rs::assert_not_stale` now compares the `.so`
mtime against `src/` + `Cargo.toml` and fails loudly, and `./run_all.sh` always
runs `cargo build` for each profile before `cargo test`.
