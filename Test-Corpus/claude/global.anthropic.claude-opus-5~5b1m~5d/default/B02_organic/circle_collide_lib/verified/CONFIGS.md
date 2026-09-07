# CONFIGS.md — Phase B configuration-surface table

Derived mechanically from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

**Runtime options / modes.** The library has no init function, no global state,
no options struct, and no `#ifdef`. `grep -c '#if\|#ifdef\|static ' c_src/src/lib.c`
→ 0. The one and only mode selector in the whole API is the `C2_TYPE typeB`
argument of `c2Collided` (`lib.c:105`, a 3-way `switch` + `default`).

**Build-time options.** `translation/Cargo.toml` declares no `[features]`
table, so there is exactly one feature combination: the default (empty) one.
Verified with `cargo read-manifest`. `c_src/CMakeLists.txt` declares no options
either.

**Data-dependent branches** (the real surface):

| location | branch |
|----------|--------|
| `lib.c:44-45` `c2Maxv` | `a.x > b.x` ternary, `a.y > b.y` ternary (independent per lane) |
| `lib.c:49-50` `c2Minv` | `a.x < b.x`, `a.y < b.y` ternaries |
| `lib.c:54` `c2Clampv`  | composition of the four above → clamp-low / pass-through / clamp-high / inverted-range per lane |
| `lib.c:72` `c2CircletoCircle` | `d2 < r2` |
| `lib.c:80` `c2CircletoAABB`   | `d2 < r2`, on top of the 4 clamp sub-branches |
| `lib.c:88` `c2CircletoCapsule`| `da < 0` → **branch A** (nearest point is endpoint `a`) |
| `lib.c:92` `c2CircletoCapsule`| `da >= 0 && db < 0` → **branch B** (nearest point is on the segment; the only path that executes the `da / c2Dot(n,n)` division and `c2Mulvs`) |
| `lib.c:95` `c2CircletoCapsule`| `da >= 0 && db >= 0` → **branch C** (nearest point is endpoint `b`) |
| `lib.c:101` `c2CircletoCapsule`| `d2 < r*r` |
| `lib.c:105-113` `c2Collided`  | `typeB` = 0 / 1 / 2 / default |

**Input shapes special-cased by the float semantics** (each must be crossed
with the branches above): normal finite; `+0.0`; `-0.0`; subnormal; `FLT_MAX`;
values whose square/sum overflows to `inf`; `+inf`; `-inf`; quiet NaN
(`0x7fc00000`); quiet NaN with a distinct payload (`0x7fc0dead`); negative NaN
(`0xffc00000`); signalling NaN (`0x7f800001`). Element type is always `float`
and there is only one byte order, so those axes are single-valued.

**Full set of public entry points**, lowest level first — the table exercises
all 12 directly through the `.so`, not just the `circle_collide` wrapper:
`c2V`, `c2Mulvs`, `c2Maxv`, `c2Minv`, `c2Sub`, `c2Dot` (level 0);
`c2Clampv` (level 1); `c2CircletoCircle`, `c2CircletoAABB`,
`c2CircletoCapsule` (level 2); `c2Collided` (level 3); `circle_collide` (level 4).

## Configuration table

Every row is run against BOTH `.so`s with **many randomized inputs**
(seeded `SplitMix64`, fixed seed `0x2024_C0DE_CAFE_F00D`), and results are
compared as **raw bits** (`f32::to_bits`, so `-0.0` vs `+0.0` and NaN payloads
are distinguished). "N" is the number of randomized cases per row.

| # | entry point(s) | configuration (options set + input shape) | N | [x] |
|---|----------------|-------------------------------------------|---|-----|
| 1  | `c2V` | random uniform-bit `u32` pairs reinterpreted as `f32` (covers all classes incl. sNaN, subnormal, ±0) — checks the packed-XMM struct return round-trips with no canonicalisation | 20000 | [x] |
| 2  | `c2V` | exhaustive edge list × edge list (all 12 special values, both args) | 144 | [x] |
| 3  | `c2Sub` | both operands random finite in ±1e3 | 20000 | [x] |
| 4  | `c2Sub` | random uniform-bit operands (inf/NaN/subnormal mix, `inf-inf`, `-0-+0`) | 20000 | [x] |
| 5  | `c2Sub` | edge list × edge list, all 4 lanes crossed | 20736 | [x] |
| 6  | `c2Dot` | both operands random finite in ±1e3 (no overflow) | 20000 | [x] |
| 7  | `c2Dot` | operands scaled to ±1e20 so products overflow to `±inf` and `inf + -inf` → NaN | 20000 | [x] |
| 8  | `c2Dot` | random uniform-bit operands — NaN-payload/operand-order sensitive (`mulss`/`addss` destination rule) | 40000 | [x] |
| 9  | `c2Dot` | both lanes distinct NaN payloads/signs, exhaustive over a NaN edge list | 4096 | [x] |
| 10 | `c2Mulvs` | `a` random finite, `b` random finite | 20000 | [x] |
| 11 | `c2Mulvs` | `b` = each special value (`±0`, `±inf`, NaNs) × `a` random-bit → `0*inf`, NaN-vs-NaN order | 20000 | [x] |
| 12 | `c2Mulvs` | exhaustive NaN edge list for `b` × NaN edge list for `a.x`/`a.y` (the lane, not the scalar, is the mulss destination) | 1728 | [x] |
| 13 | `c2Maxv` | random finite pairs, each lane independently `a>b` / `a<b` / `a==b` | 20000 | [x] |
| 14 | `c2Maxv` | `±0.0` combinations and NaN in either operand (ternary returns `b`) | 20000 | [x] |
| 15 | `c2Maxv` | edge list × edge list exhaustive, per lane | 20736 | [x] |
| 16 | `c2Minv` | random finite pairs, all lane orderings | 20000 | [x] |
| 17 | `c2Minv` | `±0.0` and NaN combinations | 20000 | [x] |
| 18 | `c2Minv` | edge list × edge list exhaustive, per lane | 20736 | [x] |
| 19 | `c2Clampv` | valid range `lo <= hi`; `a` below / inside / above, per lane (random finite) | 20000 | [x] |
| 20 | `c2Clampv` | **inverted range** `lo > hi` per lane (no validation in C) | 20000 | [x] |
| 21 | `c2Clampv` | NaN in `a`, in `lo`, in `hi`, and combinations; ±0 ranges | 20000 | [x] |
| 22 | `c2Clampv` | ±inf bounds (`lo=-inf,hi=+inf` and `lo=+inf,hi=-inf`) × random-bit `a` | 20000 | [x] |
| 23 | `c2CircletoCircle` | random finite centres/radii tuned so ~half the cases collide (`d2 < r2` both ways) | 30000 | [x] |
| 24 | `c2CircletoCircle` | exact-boundary cases: `d2 == r2` (touching circles) constructed from Pythagorean triples → must be `0` | 4096 | [x] |
| 25 | `c2CircletoCircle` | zero radii (`r=0`, `r=-0.0`), coincident centres, aliased identical circles | 20000 | [x] |
| 26 | `c2CircletoCircle` | negative radii; radii summing/overflowing to `inf`; `FLT_MAX` centres → `d2 = inf` | 20000 | [x] |
| 27 | `c2CircletoCircle` | NaN in centre and/or radius (all 6 float slots, random-bit) | 20000 | [x] |
| 28 | `c2CircletoAABB` | valid AABB (`min <= max`), circle centre **inside** the box → `d2 = 0` | 20000 | [x] |
| 29 | `c2CircletoAABB` | valid AABB, centre outside on each of the 8 sides/corners (clamp hits lo on one lane, hi on the other, etc.) | 30000 | [x] |
| 30 | `c2CircletoAABB` | **degenerate box** `min == max` (zero area) and zero-width-one-lane boxes | 20000 | [x] |
| 31 | `c2CircletoAABB` | **inverted box** `min > max` on one or both lanes | 20000 | [x] |
| 32 | `c2CircletoAABB` | boundary `d2 == r2` exactly → must be `0` | 4096 | [x] |
| 33 | `c2CircletoAABB` | `A.r` = 0, `-0.0`, negative, `FLT_MAX` (square overflows to `inf`), NaN | 20000 | [x] |
| 34 | `c2CircletoAABB` | NaN / ±inf in `min`, `max`, `A.p` (all 6 slots, random-bit) | 20000 | [x] |
| 35 | `c2CircletoCapsule` | **branch A** (`da < 0`): centre beyond endpoint `a`, random finite | 20000 | [x] |
| 36 | `c2CircletoCapsule` | **branch B** (`da >= 0 && db < 0`): centre projecting inside the segment — exercises `da / c2Dot(n,n)` + `c2Mulvs` | 30000 | [x] |
| 37 | `c2CircletoCapsule` | **branch C** (`da >= 0 && db >= 0`): centre beyond endpoint `b`, random finite | 20000 | [x] |
| 38 | `c2CircletoCapsule` | all three branches reached from fully random finite inputs (branch-hit counters asserted non-zero) | 30000 | [x] |
| 39 | `c2CircletoCapsule` | **degenerate capsule** `a == b` (`n = 0`, `c2Dot(n,n) = 0`) → takes branch C, no division | 20000 | [x] |
| 40 | `c2CircletoCapsule` | **near-degenerate capsule**: `n` at ~`1e-23` scale so `c2Dot(n,n)` underflows to `0`/subnormal on the branch-B path → the unguarded `da / 0` really executes (measured: 10 hits, printed by the test) → `±inf`/`NaN` → `c2Mulvs` → `d2 = NaN` → `0` | 20000 | [x] |
| 41 | `c2CircletoCapsule` | huge `n` so `c2Dot(n,n)` overflows to `inf` → `da/inf = 0` | 20000 | [x] |
| 42 | `c2CircletoCapsule` | `B.r`/`A.r` = 0, `-0.0`, negative, `FLT_MAX` (sum overflows to `inf`), NaN | 20000 | [x] |
| 43 | `c2CircletoCapsule` | boundary `d2 == r*r` exactly → must be `0` | 4096 | [x] |
| 44 | `c2CircletoCapsule` | NaN / ±inf in any of the 7 float slots (random-bit) | 30000 | [x] |
| 45 | `c2Collided` | `typeB = C2_TYPE_CIRCLE (0)`, random finite `c2Circle`/`c2Circle` — must equal a direct `c2CircletoCircle` call | 20000 | [x] |
| 46 | `c2Collided` | `typeB = C2_TYPE_AABB (1)`, random finite `c2Circle`/`c2AABB` (incl. inverted boxes) | 20000 | [x] |
| 47 | `c2Collided` | `typeB = C2_TYPE_CAPSULE (2)`, random finite `c2Circle`/`c2Capsule` (all 3 branches) | 20000 | [x] |
| 48 | `c2Collided` | all 3 valid `typeB` × fully random-bit payload buffers (NaN/inf/subnormal shapes reinterpreted from raw bytes) | 30000 | [x] |
| 49 | `c2Collided` | `A` and `B` **aliased** to the same buffer, each valid `typeB` (C makes no aliasing assumption) | 20000 | [x] |
| 50 | `c2Collided` | valid `typeB`, `B` buffer at an **unaligned** (odd) address → both must read it identically (Rust uses `read_unaligned`) | 20000 | [x] |
| 51 | `circle_collide` | random finite `x`,`y`,`r` over the interesting region (±150) — hits all 8 packed result values | 40000 | [x] |
| 52 | `circle_collide` | inputs aimed at each hard-coded shape in turn: near `circle` `(-70,0,r=20)`, near `aabb` `[-40,-15]^2`, near `capsule` `(-40,40)-(-20,100) r=10`, and at overlaps of two/three | 40000 | [x] |
| 53 | `circle_collide` | `r` = 0, `-0.0`, negative, subnormal, `FLT_MAX` (`r+20` overflows to `inf`) | 20000 | [x] |
| 54 | `circle_collide` | `x`/`y`/`r` = ±inf, NaNs, and fully random-bit triples | 40000 | [x] |
| 55 | `circle_collide` | exhaustive over the edge list cubed (12^3) | 1728 | [x] |

**Binary executable:** `c_src/CMakeLists.txt` builds only
`add_library(... SHARED src/lib.c)` — there is no `add_executable`, and the
Rust crate is `crate-type = ["cdylib"]` with no `[[bin]]`. So there is no
driver binary and no stdout to compare; that gate item is N/A.

---

## Row → test mapping and status

Status recorded after `./verify.sh`: **55 / 55 rows passing**, against both the
debug and the release Rust `.so`, with all comparisons made on **raw bits**
(`f32::to_bits`) rather than `==`, so `±0.0` and NaN payloads are distinguished.

| rows | test file | test fns |
|------|-----------|----------|
| 1-22  | `tests/phase_b_primitives.rs` | `row01_…` … `row22_…` (22 tests) |
| 23-44 | `tests/phase_b_shapes.rs`     | `row23_…` … `row44_…` (22 tests) |
| 45-55 | `tests/phase_b_toplevel.rs`   | `row45_…` … `row55_…` (11 tests) |

Coverage assertions baked into the tests, so a row cannot silently stop
exercising its configuration:

- row 24 / 32 / 43 assert the exactly-`d2 == r2` constructions really are
  rejected (`cv == 0`), proving the strict `<` boundary is hit and not merely
  approached;
- rows 35-37 reject-sample until the input provably lands on the intended
  capsule branch (checked with `capsule_branch`, computed from the C `.so`'s own
  primitives), and fail if fewer than half the requested inputs were produced;
- row 38 asserts all three capsule branches are reached from purely random
  input — measured `A/B/C = 14858 / 8410 / 6732`;
- row 39 asserts the exactly-degenerate capsule takes branch C;
- row 40 reports how often the zero-divisor division actually ran (10);
- row 41 asserts branch B was reached with an overflowing `c2Dot(n,n)` (17209);
- row 52 asserts all 8 packed `circle_collide` results are produced;
- rows 45-47 additionally cross-check the `c2Collided` dispatch against a direct
  call to the underlying `c2Circleto*` routine.

## Configuration axes: final findings

- **1 feature combination.** `Cargo.toml` has no `[features]` table, confirmed
  mechanically by `verify.sh` (which would otherwise expand the cross-product).
  The suite is nevertheless run against two build profiles of the cdylib
  (`debug`, `release` — the latter with `panic = "abort"`), and was additionally
  re-run against `opt-level` 0/1/2/3/s, `target-cpu=native`, and `lto=fat`:
  **12 exported symbols and 0 failures in every configuration.**
- **1 runtime mode selector**, `c2Collided`'s `typeB`, fully covered: the three
  valid variants in rows 45-50 and every invalid `int` in `ERRORS.md` row 1.
  Note the C dispatch uses an *unsigned* compare (`cmp 2` / `ja default`), so
  negative values also fall through to `default:`.
- **Lowest-level entry points are driven directly**, not only through
  `circle_collide`: rows 1-22 call the six level-0 primitives and `c2Clampv`
  through the `.so` exports, which is where all four real bugs were found.
