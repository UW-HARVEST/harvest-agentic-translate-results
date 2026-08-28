# CONFIGS.md — configuration surface table (Phase A → Phase B)

Derived **mechanically** from the branches `c_src/src/lib.c` actually takes.

## Axis 1 — public entry points (the FULL set, lowest level first)

From `nm -D` on the C `.so` (see `SYMBOLS.md`). `c_src/include/lib.h` exposes
only `circle_collide`, but all 12 symbols have external linkage and are callable
by any consumer, so all 12 are exercised directly — not just the one-shot
`circle_collide` wrapper.

| level | entry points |
|-------|--------------|
| L0 (scalar/vector primitives) | `c2V`, `c2Sub`, `c2Dot`, `c2Mulvs` |
| L1 (ternary min/max)          | `c2Maxv`, `c2Minv`, `c2Clampv` |
| L2 (collision kernels)        | `c2CircletoCircle`, `c2CircletoAABB`, `c2CircletoCapsule` |
| L3 (type-dispatch)            | `c2Collided` |
| L4 (one-shot convenience)     | `circle_collide` |

## Axis 2 — runtime options / modes

The only runtime option in the whole API is `c2Collided`'s `C2_TYPE typeB`
(`lib.c:104-115`), which selects the kernel via `switch`:

| `typeB` | branch taken | `B` reinterpreted as | bytes read from `B` |
|---------|--------------|----------------------|---------------------|
| `0` `C2_TYPE_CIRCLE`  | `lib.c:107` | `c2Circle`  | 12 |
| `1` `C2_TYPE_AABB`    | `lib.c:109` | `c2AABB`    | 16 |
| `2` `C2_TYPE_CAPSULE` | `lib.c:111` | `c2Capsule` | 20 |
| any other             | `lib.c:113` `default:` | (nothing) | 0 |

(`A` is *always* reinterpreted as `c2Circle` — the same for all four arms.)

## Axis 3 — data-dependent branches inside the kernels

| function | branch condition | arms |
|----------|------------------|------|
| `c2Maxv` (`lib.c:44`) | `a.x > b.x`, `a.y > b.y` | 2 × 2 (independent per lane) |
| `c2Minv` (`lib.c:49`) | `a.x < b.x`, `a.y < b.y` | 2 × 2 |
| `c2Clampv` (`lib.c:54`) | composition of the two above | 16 |
| `c2CircletoCapsule` (`lib.c:88,92`) | `da < 0` / `db < 0` | 3 regions: **before-A cap**, **shaft** (`da≥0, db<0`), **after-B cap** (`da≥0, db≥0`) |
| all three kernels | final `d2 < r2` | hit / miss / unordered |

## Axis 4 — input shapes / value classes

Every `float` argument is one of these IEEE-754 classes; the code branches
differently (or produces different bits) for each: `+normal`, `-normal`,
`+0.0`, `-0.0`, `+denormal`, `-denormal`, `+inf`, `-inf`, `QNaN` (random
payload, both signs), `SNaN` (random payload, both signs), `FLT_MAX`,
`FLT_MIN`, values that overflow on `+`/`*`.

Geometric shapes: circle **inside / overlapping / touching exactly / outside**
each target; AABB **well-formed / degenerate (min==max) / inverted (min>max)**;
capsule **well-formed / degenerate (a==b) / axis-aligned / diagonal**; circle
centre **inside the box / on a face / on a corner / outside**.

## CONFIGURATION-SURFACE TABLE

Cross-product of axes 1–4, pruned to the combinations the C distinguishes.
Every row is driven with **many randomized inputs** (fixed seed
`0x243F6A8885A308D3`, SplitMix64) through **both** `.so`s and compared
bit-for-bit. The test named `cfgNN_*` implements row NN:

| rows | file |
|------|------|
| 1–26  | `translation/tests/phase_b_primitives.rs` |
| 27–46 | `translation/tests/phase_b_kernels.rs` |
| 47–59 | `translation/tests/phase_b_dispatch.rs` |

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `f32` bit patterns (uniform over all 2³² encodings, filtered to finite) | [x] |
| 2 | `c2V` | full random `u32`-as-`f32` incl. NaN/inf/denormal/−0 — bit-exact pass-through | [x] |
| 3 | `c2V` | the exhaustive special-value grid (all 24 classes × 24 classes) | [x] |
| 4 | `c2Sub` | random finite pairs (normal magnitudes 1e−3 … 1e3) | [x] |
| 5 | `c2Sub` | random full-range pairs (any bit pattern) | [x] |
| 6 | `c2Sub` | special-value grid: ±0/±inf/±denormal/NaN combinations, sign-of-zero exact | [x] |
| 7 | `c2Sub` | overflow shapes: `±FLT_MAX` minus `∓FLT_MAX` | [x] |
| 8 | `c2Dot` | random finite vectors, normal magnitudes | [x] |
| 9 | `c2Dot` | random full-range vectors (any bit pattern) — pins `mulss`/`addss` operand order | [x] |
| 10 | `c2Dot` | special-value grid on `(a.x,b.x)` with `a.y,b.y` random NaN payloads | [x] |
| 11 | `c2Dot` | products cancelling exactly (`a.x*b.x == -(a.y*b.y)`) ⇒ `±0` sign | [x] |
| 12 | `c2Dot` | products overflowing to `±inf`, and `inf + −inf` ⇒ NaN | [x] |
| 13 | `c2Dot` | `a == b` (the self-dot used by every kernel) | [x] |
| 14 | `c2Mulvs` | random finite vector × random finite scalar | [x] |
| 15 | `c2Mulvs` | random full-range vector × full-range scalar (NaN order, `0*inf`) | [x] |
| 16 | `c2Mulvs` | scalar ∈ {`+0`,`−0`,`+1`,`−1`,`+inf`,`−inf`,QNaN,SNaN,denormal} × special-value vector grid | [x] |
| 17 | `c2Maxv` | random finite pairs — covers all 4 lane-branch combinations | [x] |
| 18 | `c2Maxv` | equal lanes (`a.x == b.x`) forcing the `else` arm | [x] |
| 19 | `c2Maxv` | full-range random incl. NaN in either/both operands, and ±0 pairs | [x] |
| 20 | `c2Minv` | random finite pairs — all 4 lane-branch combinations | [x] |
| 21 | `c2Minv` | equal lanes forcing the `else` arm | [x] |
| 22 | `c2Minv` | full-range random incl. NaN in either/both operands, and ±0 pairs | [x] |
| 23 | `c2Clampv` | `lo < hi` well-formed range, `a` below / inside / above (all 16 lane paths) | [x] |
| 24 | `c2Clampv` | `lo == hi` degenerate range | [x] |
| 25 | `c2Clampv` | `lo > hi` inverted range | [x] |
| 26 | `c2Clampv` | full-range random `a`/`lo`/`hi` incl. NaN and ±0 | [x] |
| 27 | `c2CircletoCircle` | random finite circles, radii > 0, centres clustered so hits and misses both occur | [x] |
| 28 | `c2CircletoCircle` | exact-touch shape: `dist == A.r + B.r` (boundary of `<`) | [x] |
| 29 | `c2CircletoCircle` | concentric (`A.p == B.p`), zero radii, one zero radius | [x] |
| 30 | `c2CircletoCircle` | negative radii; radii summing to `0`; `−r` vs `+r` | [x] |
| 31 | `c2CircletoCircle` | full-range random bit patterns for all 6 fields (NaN/inf/denormal) | [x] |
| 32 | `c2CircletoAABB` | random finite circle vs well-formed box; centre inside / on face / on corner / outside | [x] |
| 33 | `c2CircletoAABB` | degenerate box `min == max` (a point) | [x] |
| 34 | `c2CircletoAABB` | inverted box `min > max` in one or both axes | [x] |
| 35 | `c2CircletoAABB` | exact-touch: circle centre exactly `A.r` from the nearest face | [x] |
| 36 | `c2CircletoAABB` | `A.r` = 0 / negative / `inf` / NaN | [x] |
| 37 | `c2CircletoAABB` | full-range random bit patterns for all 6 fields | [x] |
| 38 | `c2CircletoCapsule` | **branch `da < 0`** (before-A cap): centre behind `B.a` along `n`; random finite | [x] |
| 39 | `c2CircletoCapsule` | **branch `da ≥ 0, db < 0`** (shaft): centre projecting inside the segment; random finite | [x] |
| 40 | `c2CircletoCapsule` | **branch `da ≥ 0, db ≥ 0`** (after-B cap): centre past `B.b`; random finite | [x] |
| 41 | `c2CircletoCapsule` | branch boundaries: `da == 0` exactly, `db == 0` exactly | [x] |
| 42 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` (`n == 0` ⇒ unguarded divide) | [x] |
| 43 | `c2CircletoCapsule` | axis-aligned capsule (`n.y == 0`, then `n.x == 0`) and diagonal capsule | [x] |
| 44 | `c2CircletoCapsule` | exact-touch on the shaft and on each cap | [x] |
| 45 | `c2CircletoCapsule` | radii: `0`, negative, `inf`, NaN, sum overflowing | [x] |
| 46 | `c2CircletoCapsule` | full-range random bit patterns for all 7 fields | [x] |
| 47 | `c2Collided` | `typeB = C2_TYPE_CIRCLE (0)`, random finite `c2Circle`/`c2Circle` buffers | [x] |
| 48 | `c2Collided` | `typeB = C2_TYPE_AABB (1)`, random finite `c2Circle`/`c2AABB` buffers | [x] |
| 49 | `c2Collided` | `typeB = C2_TYPE_CAPSULE (2)`, random finite `c2Circle`/`c2Capsule` buffers | [x] |
| 50 | `c2Collided` | each of `typeB ∈ {0,1,2}` with **full-range random bytes** in both buffers (matches the kernels' full-range rows through the dispatch layer) | [x] |
| 51 | `c2Collided` | `typeB ∈ {0,1,2}` with **unaligned** `A`/`B` (offset 1,2,3 into a byte buffer) — C's `*(c2Circle*)A` allows it, Rust must use `read_unaligned` | [x] |
| 52 | `c2Collided` | equivalence: `c2Collided(A,B,t)` == the corresponding `c2Circleto*` called directly with the same bytes, for all `t ∈ {0,1,2}` | [x] |
| 53 | `circle_collide` | random `(x, y, r)` over the interesting geometric window `[-150,150]²` × `[0,60]` — hits all 8 result bit patterns | [x] |
| 54 | `circle_collide` | targeted values reaching each of the 8 possible return values `0..=7` | [x] |
| 55 | `circle_collide` | exact-boundary values (touching each hard-coded shape exactly) | [x] |
| 56 | `circle_collide` | full-range random bit patterns for `x`, `y`, `r` (NaN/inf/denormal/−0) | [x] |
| 57 | `circle_collide` | wide random sweep, 200 000 samples, in the hot geometric window | [x] |
| 58 | pipeline | `c2Clampv` ∘ `c2Minv`/`c2Maxv` ∘ `c2V` composed by hand and compared against `c2CircletoAABB`'s internal use (cross-`.so` composition: C helper + Rust kernel and vice-versa is not possible, so the composed intermediate values are compared step-by-step) | [x] |
| 59 | all 12 symbols | monolithic randomized sweep: 100 000 iterations, every entry point called with a shared random state, all outputs compared bit-for-bit | [x] |

## Feature combinations

`translation/Cargo.toml` has no `[features]` table ⇒ exactly one configuration.
`translation/check_features.sh` enumerates the feature powerset (the empty set
⇒ two builds: `<default>` and `--no-default-features`) and, for each, builds the
`cdylib` in **both** the `release` and `debug` profiles, diffs `nm -D` against the
C `.so`, and runs the whole suite. Result: 107 tests pass per combination.

The harness loads *every* Rust `.so` it finds (`target/release` **and**
`target/debug`) and compares each independently against the C `.so`, so both
codegen paths are covered by every row.
