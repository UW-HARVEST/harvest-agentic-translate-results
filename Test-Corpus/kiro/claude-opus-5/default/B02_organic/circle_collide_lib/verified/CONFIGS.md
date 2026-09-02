# CONFIGS.md — Configuration surface table (Phase A → gates Phase B)

Derived **mechanically** from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C actually branches on

**Axis O — runtime options/modes.** The library has *no* settable options: no
globals, no init/config function, no flags argument, no `#ifdef`. The single
mode-like input is the `C2_TYPE typeB` argument of `c2Collided`, whose `switch`
(`src/lib.c:105-114`) has 3 valid arms + `default`.

**Axis E — entry points (all 12, including the lowest-level ones).**
Level 0 (leaf): `c2V`, `c2Sub`, `c2Dot`, `c2Mulvs`.
Level 1: `c2Maxv`, `c2Minv`.
Level 2: `c2Clampv`.
Level 3: `c2CircletoCircle`, `c2CircletoAABB`, `c2CircletoCapsule`.
Level 4 (dispatcher): `c2Collided`.
Level 5 (convenience one-shot): `circle_collide` — the only symbol in `lib.h`.
Phase B drives levels 0→5 directly, not just level 5.

**Axis B — control-flow shapes the code special-cases.**
* `c2Maxv`/`c2Minv`: 2 independent ternaries × {taken, not-taken} = 4 shapes each.
* `c2CircletoCapsule`: 3 mutually exclusive arms — `da < 0`, `da ≥ 0 && db < 0`
  (the projection arm with the division), `da ≥ 0 && db ≥ 0`.
* `c2Circleto*`: predicate `d2 < r2` on each side of the boundary, plus exact
  equality (`d2 == r2` ⇒ false).

**Axis V — value shapes for the `float` payloads.**
`normal`, `zero (+0/-0)`, `subnormal`, `huge (near MAX ⇒ overflow in c2Dot)`,
`±inf`, `NaN (quiet, and with payload/sign bits set)`, `mixed`.

**Axis S — geometric input shapes.**
AABB: proper (`min<max`), degenerate (`min==max`, a point / a zero-width line),
inverted (`min>max`), unbounded (`±inf` bounds).
Capsule: proper segment, degenerate (`a==b` ⇒ zero-length ⇒ `dot(n,n)==0`),
axis-aligned, zero radius, negative radius.
Circle: `r>0`, `r==0`, `r<0`.

## Table — one row per combination the C treats differently

Every row is driven with **many randomized inputs from a fixed seed**
(SplitMix64, `SEED = 0x5EED_1234_ABCD_EF01`) — not one hand-picked value — plus
the hand-picked boundary/special values for that row. Both `.so`s are loaded via
`libloading` and their raw returns compared **bit-for-bit** (`f32::to_bits`) or
int-for-int.

| #  | entry point(s) | configuration (options set + input shape) | reps | ✔ |
|----|----------------|-------------------------------------------|------|---|
| 1  | `c2V` | random normal `(x,y)` pairs; identity/passthrough | 4096 | [x] |
| 2  | `c2V` | special floats: `±0`, `±inf`, NaN w/ payloads, subnormal, `MIN`/`MAX` (full cross product) | 26² | [x] |
| 3  | `c2Sub` | random normal pairs of `c2v` | 4096 | [x] |
| 4  | `c2Sub` | special-float cross product (`inf-inf`⇒NaN, `+0 - +0`⇒`+0`, `+0 - -0`⇒`+0`, `-0 - +0`⇒`-0`) | 26² | [x] |
| 5  | `c2Dot` | random normal `c2v` pairs | 4096 | [x] |
| 6  | `c2Dot` | huge magnitudes ⇒ `mulss` overflow to `inf`; `inf*0`⇒NaN; `NaN+(-NaN)` operand-order-dependent payload | 26² | [x] |
| 7  | `c2Mulvs` | random `c2v` × random normal scalar | 4096 | [x] |
| 8  | `c2Mulvs` | special-float vector × special-float scalar (subnormal underflow, `inf*0`, NaN payload propagation / sign bit) | 26²·2 | [x] |
| 9  | `c2Maxv` | random pairs — hits all 4 ternary-branch combinations statistically | 4096 | [x] |
| 10 | `c2Maxv` | equal components (`a.x==b.x` ⇒ `>` false ⇒ returns `b`), `±0` pairs (`+0 > -0` false ⇒ returns `b`) | 26² | [x] |
| 11 | `c2Minv` | random pairs — all 4 ternary-branch combinations | 4096 | [x] |
| 12 | `c2Minv` | equal components, `±0` pairs, NaN operands (comparison false ⇒ returns `b`) | 26² | [x] |
| 13 | `c2Clampv` | `lo < hi` (proper range), random `a` inside / below / above | 4096 | [x] |
| 14 | `c2Clampv` | `lo > hi` (inverted range) — no ordering check in C | 4096 | [x] |
| 15 | `c2Clampv` | `lo == hi`; and `±inf` bounds (unbounded clamp); and NaN in `a`/`lo`/`hi` | 26³ | [x] |
| 16 | `c2CircletoCircle` | random circles, radii `>0`, overlapping and disjoint | 4096 | [x] |
| 17 | `c2CircletoCircle` | grazing boundary: `d2` swept across `r2` incl. exact equality ⇒ must be `0` | 2048 | [x] |
| 18 | `c2CircletoCircle` | `r == 0` (point circle), one or both; and `r < 0` | 2048 | [x] |
| 19 | `c2CircletoCircle` | huge radii ⇒ `(A.r+B.r)^2` overflows to `inf`; and NaN/`inf` positions | 26² | [x] |
| 20 | `c2CircletoAABB` | proper AABB (`min<max`), circle inside / edge / corner / outside | 4096 | [x] |
| 21 | `c2CircletoAABB` | degenerate AABB `min == max` (point) and zero-width on one axis only | 2048 | [x] |
| 22 | `c2CircletoAABB` | inverted AABB `min > max` on one or both axes | 2048 | [x] |
| 23 | `c2CircletoAABB` | unbounded AABB (`±inf` bounds); NaN bounds; `r==0`; `r<0` | 26² | [x] |
| 24 | `c2CircletoAABB` | grazing boundary `d2` swept across `r2 = r*r` incl. exact equality | 2048 | [x] |
| 25 | `c2CircletoCapsule` | proper segment, circle before `a` ⇒ **arm 1** (`da < 0`) | 2048 | [x] |
| 26 | `c2CircletoCapsule` | proper segment, circle beside segment ⇒ **arm 2** (`da≥0 && db<0`, the `da/dot(n,n)` projection arm) | 2048 | [x] |
| 27 | `c2CircletoCapsule` | proper segment, circle past `b` ⇒ **arm 3** (`da≥0 && db≥0`) | 2048 | [x] |
| 28 | `c2CircletoCapsule` | fully random circle+capsule — arms chosen by the data, incl. `da == 0` / `db == 0` exactly | 4096 | [x] |
| 29 | `c2CircletoCapsule` | degenerate capsule `a == b` ⇒ `dot(n,n)==0` ⇒ unguarded division | 2048 | [x] |
| 30 | `c2CircletoCapsule` | axis-aligned segments (horizontal, vertical), `B.r==0`, `B.r<0`, `A.r==0` | 2048 | [x] |
| 31 | `c2CircletoCapsule` | huge / subnormal / `±inf` / NaN coordinates and radii | 26² | [x] |
| 32 | `c2Collided` | `typeB = C2_TYPE_CIRCLE (0)`, random `c2Circle` × `c2Circle` buffers | 4096 | [x] |
| 33 | `c2Collided` | `typeB = C2_TYPE_AABB (1)`, random `c2Circle` × `c2AABB` buffers (incl. inverted/degenerate boxes) | 4096 | [x] |
| 34 | `c2Collided` | `typeB = C2_TYPE_CAPSULE (2)`, random `c2Circle` × `c2Capsule` buffers (incl. degenerate segments) — exercises the 20-byte MEMORY-class by-value struct pass | 4096 | [x] |
| 35 | `c2Collided` | each valid `typeB` with all-random *raw bytes* in the operand buffers (so the floats are arbitrary bit patterns incl. NaNs/subnormals) | 3·2048 | [x] |
| 36 | `c2Collided` | each valid `typeB` with the operand at a **non-8-byte-aligned** address | 3·1024 | [x] |
| 37 | `circle_collide` | random `(x,y,r)` normals over the region spanned by the 3 hard-coded shapes ⇒ exercises the 3-bit result packing | 8192 | [x] |
| 38 | `circle_collide` | targeted `(x,y,r)` that hit each of the 8 possible result bit-patterns (0b000…0b111) where reachable, incl. the exact hard-coded shape centres/edges | 2048 | [x] |
| 39 | `circle_collide` | `r == 0`, `r < 0`, huge `r` (⇒ all bits set), `±inf` and NaN in `x`/`y`/`r` | 26³ | [x] |
| 40 | full pipeline | composed cross-check: recompute `circle_collide`'s expected value by calling the C `.so`'s *low-level* exports (`c2Collided`×3 + shifts) and compare against the C and Rust one-shot `circle_collide` — catches divergence that per-wrapper tests hide | 4096 | [x] |

## Feature combinations

`translation/Cargo.toml` has **no `[features]` table** ⇒ exactly one build
configuration. `check_features.sh` enumerates them from `Cargo.toml` and runs the
whole suite under `--no-default-features` and `--all-features` as well, all of
which resolve to the same single code path. It also runs every combination under
**both** cargo profiles, which is what exposed the `c2Collided` unaligned-load
defect (debug-assertions only).

## How to reproduce

```sh
cd translation && ./run_tests.sh          # build C + Rust, run everything
./check_features.sh                      # every feature combo x both profiles
```

`cargo test` alone is **not** sufficient: the crate is `crate-type = ["cdylib"]`
only, so no test target links the library and cargo skips building it. The
harness (`tests/common/mod.rs`) hard-fails with a `STALE ARTIFACT` panic if the
`.so` predates `src/lib.rs`, so a stale build can never silently pass.

## Result

All 40 rows pass. Row-by-row test mapping:

| rows | test file |
|------|-----------|
| 1–15  | `tests/phase_b_leaf.rs` (19 tests) |
| 16–31 | `tests/phase_b_shapes.rs` (19 tests) |
| 32–40 | `tests/phase_b_dispatch.rs` (10 tests) |
| all   | `tests/fuzz_all.rs` — 300 k random raw-bit inputs per entry point |

Coverage of each row is *asserted*, not assumed: the capsule-arm rows check
`capsule_arm(...)` really equals 1 / 2 / 3, and the random rows assert that both
outcomes (collide and not-collide) actually occur, so a row cannot pass
vacuously.
