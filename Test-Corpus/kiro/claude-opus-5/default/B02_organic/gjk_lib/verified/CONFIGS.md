# CONFIGS.md — Phase B configuration surface table

Derived mechanically from the branches `c_src/src/lib.c` actually takes.

## Axes the C code branches on

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| `C2_TYPE typeA` | `C2_TYPE_CIRCLE` (proxy: r=c->r, count=1), `C2_TYPE_AABB` (r=0, count=4), `C2_TYPE_CAPSULE` (r=c->r, count=2) | `c2MakeProxy` `switch` |
| `C2_TYPE typeB` | same three | `c2MakeProxy` `switch` |
| `ax_ptr` | `NULL` ⇒ `c2xIdentity()`, non-NULL ⇒ arbitrary translation+rotation | `if (!ax_ptr)` |
| `bx_ptr` | `NULL` ⇒ `c2xIdentity()`, non-NULL | `if (!bx_ptr)` |
| `use_radius` | `0` (skip shrink), `!=0` (shrink; then two sub-branches) | `else if (use_radius)` |
| `cache` | `NULL`, cold (`count==0`), warm (`count` 1/2/3 from a previous call), hand-forged | `if (cache)`, `cache_was_good` |
| `outA`/`outB`/`iterations` | `NULL` vs non-NULL, each independently | three `if` guards |
| separation regime | disjoint (`dist > rA+rB`), touching (`dist ≈ rA+rB`), overlapping-but-not-hit, penetrating (`s.count==3` ⇒ `hit`) | `hit`, `dist > rA+rB && dist > FLT_EPSILON` |
| simplex `count` on entry to `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric` | 1, 2, 3, and out-of-range (0/negative/≥4 ⇒ `default`) | every `switch (s->count)` |
| `c22` branch | `v<=0`, `u<=0`, interior | 3-way `if` |
| `c23` branch | vertex A, vertex B, vertex C, edge AB, edge BC, edge CA, interior | 7-way `if` |
| `c2D` branch | count 1, count 2 & `det>0`, count 2 & `det<=0`, count 3/default | nested |
| `c2Support` `count` | 1, 2, 4, 8, and `<=0` | loop bound |
| AABB shape | normal, zero-area, zero-width, zero-height, inverted (`min>max`) | `c2BBVerts` (no validation) |
| capsule shape | normal, zero-length, r=0, r<0, huge r | `c2MakeProxy` |
| `gjk reverse` | `0`, `1`, other non-zero incl. negative `char` | `if (reverse)` |
| float value class | normal, subnormal, ±0, ±inf, NaN, ±FLT_MAX | all comparisons |

`translation/Cargo.toml` has **no `[features]`** table ⇒ exactly one feature
configuration (default == `--no-default-features`).

## Rows

Each row is exercised with **many randomized inputs** (fixed seed
`0x2545F4914F6CDD1D`, xorshift64\* PRNG, ≥256 cases/row unless noted) and asserted
**bit-for-bit** (`to_bits()`) between the C `.so` and the Rust `.so`.

### Leaf vector maths (lowest-level entry points, called directly)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random `(x,y)` incl. ±0, ±inf, NaN, subnormal, ±FLT_MAX | [x] |
| 2 | `c2Mulvs` | random vec × random scalar incl. 0, -0, inf, NaN | [x] |
| 3 | `c2Maxv` | random pairs; plus NaN in each of the 4 component slots | [x] |
| 4 | `c2Minv` | random pairs; plus NaN in each of the 4 component slots | [x] |
| 5 | `c2Clampv` | `lo<hi` normal; `lo>hi` inverted; `lo==hi`; NaN in `a`/`lo`/`hi` | [x] |
| 6 | `c2Sub` | random pairs incl. inf−inf ⇒ NaN, ±0 combinations | [x] |
| 7 | `c2Dot` | random pairs incl. inf·0 ⇒ NaN, cancellation cases | [x] |
| 8 | `c2Det2` | random pairs incl. collinear (det exactly 0) and overflow | [x] |
| 9 | `c2Len` | random vec; zero vec; inf component; NaN; overflow to inf | [x] |
| 10 | `c2Neg` | random vec; +0 ⇒ −0 and −0 ⇒ +0; NaN sign | [x] |
| 11 | `c2Skew` | random vec incl. ±0 | [x] |
| 12 | `c2CCW90` | random vec incl. ±0 | [x] |
| 13 | `c2Div` | random vec ÷ {random, 0.0, −0.0, inf, NaN, FLT_MIN} | [x] |
| 14 | `c2Norm` | random vec; zero vec ⇒ NaN; huge vec ⇒ overflow; NaN in | [x] |
| 15 | `c2Add` | random pairs incl. inf+(−inf) ⇒ NaN | [x] |
| 16 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant-value parity | [x] |
| 17 | `c2Mulrv` | random `c2r` (incl. non-unit, zero, NaN) × random vec | [x] |
| 18 | `c2MulrvT` | random `c2r` (incl. non-unit, zero, NaN) × random vec | [x] |
| 19 | `c2Mulxv` | random `c2x` (identity, pure translate, pure rotate, both, NaN) × random vec | [x] |

### Shape / proxy layer

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 20 | `c2BBVerts` | normal AABB | [x] |
| 21 | `c2BBVerts` | inverted (`min>max`), zero-area, zero-width, zero-height, NaN/inf corners | [x] |
| 22 | `c2MakeProxy` | `type=C2_TYPE_CIRCLE`, random `c2Circle` incl. r=0, r<0, NaN r; pre-poisoned `c2Proxy` (verifies untouched bytes) | [x] |
| 23 | `c2MakeProxy` | `type=C2_TYPE_AABB`, random/degenerate/inverted `c2AABB`; pre-poisoned proxy | [x] |
| 24 | `c2MakeProxy` | `type=C2_TYPE_CAPSULE`, random `c2Capsule` incl. zero-length, r=0, r<0; pre-poisoned proxy | [x] |
| 25 | `c2Support` | `count=1` | [x] |
| 26 | `c2Support` | `count=2` (capsule shape) | [x] |
| 27 | `c2Support` | `count=4` (AABB shape) | [x] |
| 28 | `c2Support` | `count=8` (full proxy width) | [x] |
| 29 | `c2Support` | ties (all verts identical ⇒ lowest index wins); `d = (0,0)`; NaN in `d`; NaN in verts | [x] |

### Simplex layer (called directly on hand-built `c2Simplex`, all `count` values)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 30 | `c2GJKSimplexMetric` | `count=1` / `2` / `3` with random simplices | [x] |
| 31 | `c2GJKSimplexMetric` | `count` = 0, −1, 4, 99 (`default` fall-through) | [x] |
| 32 | `c22` | random simplices hitting the `v<=0` branch | [x] |
| 33 | `c22` | random simplices hitting the `u<=0` branch | [x] |
| 34 | `c22` | random simplices hitting the interior branch | [x] |
| 35 | `c22` | `a.p == b.p`; `a.p == origin`; NaN in `p` | [x] |
| 36 | `c23` | uniformly random `p` triples (all 7 branches sampled by coverage counter) | [x] |
| 37 | `c23` | triangles constructed to *contain* the origin (interior branch) | [x] |
| 38 | `c23` | degenerate: collinear triples (`area == 0`) | [x] |
| 39 | `c23` | degenerate: two/three coincident points; a point at the origin | [x] |
| 40 | `c2D` | `count=1` random | [x] |
| 41 | `c2D` | `count=2`, `det>0` branch | [x] |
| 42 | `c2D` | `count=2`, `det<=0` branch incl. `det` exactly 0 | [x] |
| 43 | `c2D` | `count=3`, and 0/−1/4 (`default`) | [x] |
| 44 | `c2Witness` | `count=1` random | [x] |
| 45 | `c2Witness` | `count=2` random `div` incl. 0, tiny, huge, negative | [x] |
| 46 | `c2Witness` | `count=3` random `div` incl. 0, tiny, huge, negative | [x] |
| 47 | `c2Witness` | `count` = 0, −1, 4 (`default` ⇒ both zero) | [x] |
| 48 | `c2L` | `count=1`; `count=2` incl. `div=0`; `count=3`/0/−1 (`default`) | [x] |

### `c2GJK` — full cross-product of shape pair × transforms × use_radius × cache

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 49 | `c2GJK` | circle×circle, identity xforms (`NULL`), `use_radius=1`, no cache | [x] |
| 50 | `c2GJK` | circle×aabb, ditto | [x] |
| 51 | `c2GJK` | circle×capsule, ditto | [x] |
| 52 | `c2GJK` | aabb×circle, ditto | [x] |
| 53 | `c2GJK` | aabb×aabb, ditto | [x] |
| 54 | `c2GJK` | aabb×capsule, ditto | [x] |
| 55 | `c2GJK` | capsule×circle, ditto | [x] |
| 56 | `c2GJK` | capsule×aabb, ditto | [x] |
| 57 | `c2GJK` | capsule×capsule, ditto | [x] |
| 58 | `c2GJK` | all 9 shape pairs, `use_radius=0` | [x] |
| 59 | `c2GJK` | all 9 shape pairs, non-NULL `ax` (translate only), `bx = NULL` | [x] |
| 60 | `c2GJK` | all 9 shape pairs, `ax = NULL`, non-NULL `bx` (translate only) | [x] |
| 61 | `c2GJK` | all 9 shape pairs, both xforms non-NULL with random unit rotations | [x] |
| 62 | `c2GJK` | all 9 shape pairs, both xforms non-NULL with **non-unit / zero / NaN** `c2r` | [x] |
| 63 | `c2GJK` | all 9 shape pairs, cold cache (`count=0`), `use_radius=1` — asserts returned `dist`, `outA`, `outB`, `iterations`, **and the whole written-back cache** | [x] |
| 64 | `c2GJK` | all 9 shape pairs, warm cache: call twice with the *same* cache object, shapes unchanged | [x] |
| 65 | `c2GJK` | all 9 shape pairs, warm cache: call twice, shape B *moved* between calls (stale cache) | [x] |
| 66 | `c2GJK` | 8-call chain re-using one cache while a shape sweeps along a line (long-lived cache) | [x] |
| 67 | `c2GJK` | all 9 shape pairs, separation regime = clearly disjoint | [x] |
| 68 | `c2GJK` | all 9 shape pairs, separation regime = deeply penetrating (`hit` path) | [x] |
| 69 | `c2GJK` | all 9 shape pairs, separation regime = grazing/touching (`dist ≈ rA+rB`) | [x] |
| 70 | `c2GJK` | all 9 shape pairs, coincident shapes (identical centres) | [x] |
| 71 | `c2GJK` | degenerate shapes: zero-radius circle, zero-area AABB, zero-length capsule, zero-radius capsule — all 9 pairings | [x] |
| 72 | `c2GJK` | inverted AABB (`min > max`) as A and as B | [x] |
| 73 | `c2GJK` | negative radii (circle and capsule) | [x] |
| 74 | `c2GJK` | huge radii / huge coordinates (FLT_MAX-scale ⇒ overflow to inf) | [x] |
| 75 | `c2GJK` | subnormal-scale coordinates and radii | [x] |
| 76 | `c2GJK` | NaN / ±inf injected into shape fields | [x] |
| 77 | `c2GJK` | `iterations` non-NULL, checked for every regime (verifies the `iter`-not-incremented-on-break quirk) | [x] |
| 78 | `c2GJK` | `outA`/`outB` NULL in all 4 combinations | [x] |
| 79 | `c2GJK` | worst case designed to reach the 20-iteration cap | [x] |

Row 79 note: the cap is **structurally unreachable**. Each proxy holds at most
4 vertices, so the duplicate-support-pair (`dup`) test always terminates the
loop first; the highest `iter` observed across the whole suite is **3**
(histogram over 27 000 pairs: `iter` 0 → 7529, 1 → 11907, 2 → 5797, 3 → 1767,
4+ → 0). The row is covered by long thin boxes/capsules at random angles across
four coordinate scales, and asserts `iter <= 20` and `iter` parity rather than
claiming to hit 20.

### Terminal-regime coverage (evidence for rows 49–79)

Measured over 27 000 shape pairs, classified from observable outputs only:

| regime | C source location | cases hit |
|--------|-------------------|-----------|
| `hit` (`s.count == 3`) | `if (s.count == 3) { hit = 1; break; }` | 2 401 |
| positive separation | witness distance `> 0` | 24 599 |
| `use_radius` shrink | `if (dist > rA + rB && dist > FLT_EPSILON)` | 12 403 |
| `use_radius` midpoint collapse | the `else` branch | 9 819 |

`c22`'s 3 branches and `c23`'s 7 branches are each covered and asserted
non-zero by a branch counter in `tests/phase_b_simplex.rs`
(`c23` hits: `[17700, 7743, 6502, 7336, 4706, 2015, 9998]`).

### `gjk` — the public header entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 80 | `gjk` | `reverse=0`, random AABB + random capsule (large random sweep, 20000 cases) | [x] |
| 81 | `gjk` | `reverse=1`, same sweep | [x] |
| 82 | `gjk` | `reverse` = other non-zero values incl. `-1`, `2`, `0x7f`, `(char)0x80` | [x] |
| 83 | `gjk` | overlapping AABB/capsule (hit path) | [x] |
| 84 | `gjk` | disjoint AABB/capsule, far apart | [x] |
| 85 | `gjk` | touching exactly (`dist == r`) | [x] |
| 86 | `gjk` | integer-grid coordinates (many exact ties in `c2Support`) | [x] |
| 87 | `gjk` | zero-extent AABB, zero-length capsule, `r=0`, `r<0`, huge `r` | [x] |
| 88 | `gjk` | NaN / ±inf in each of the 9 float parameters, one at a time | [x] |
| 89 | `gjk` | `a`/`b` NULL in all 4 combinations | [x] |
| 90 | `gjk` | `a` and `b` aliasing the same `c2v` | [x] |

### Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is **no `add_executable`**, and the Rust crate is `crate-type = ["cdylib"]` with
no `src/main.rs`. No driver binary exists, so the "compare stdout" clause of
Phase B is not applicable.

## Divergences found and fixed

Two real translation defects were found by these rows and fixed in
`translation/src/lib.rs` (the C was never modified):

1. **`c2BBVerts` — aliasing read order** (row 21). The Rust cached `bb->min`
   and `bb->max` into locals before writing any output, whereas the C re-reads
   `bb` between each of the four stores:
   ```c
   out[0] = bb->min;
   out[1] = c2V(bb->max.x, bb->min.y);
   out[2] = bb->max;                    /* re-reads bb, possibly after out[1] overwrote it */
   out[3] = c2V(bb->min.x, bb->max.y);
   ```
   With `out` overlapping `*bb` the two diverged on the very first case
   (`out[2]`: C `(9.5, 4.0)` vs Rust `(9.5, 9.627013)`). Fixed by reading each
   field through a raw pointer at the exact point of use.

2. **`c2Witness` — borrow held across aliasing writes**. The Rust held
   `&(*s).verts` while storing through `*a`, which may alias the simplex. The
   observable ordering happened to be correct under both `-O0` and `-O2`, but
   the shared reference is invalidated by the write, so LLVM was free to hoist
   the later loads above it. Rewritten with raw-pointer reads, matching the C's
   read-after-write ordering unconditionally.

## Build-flag sensitivity (not a translation defect)

The suite was additionally run against the C library rebuilt out-of-tree at
`-O2` and at `-O3 -march=native`:

| C build | result |
|---------|--------|
| documented CMake command (no `-O`) | all 92 tests pass |
| `-O2` | all 92 tests pass |
| `-O3 -march=native` | 10 tests fail |
| `-O3 -march=native -ffp-contract=off` | all 92 tests pass |

The `-march=native` failures come from GCC's default `-ffp-contract=fast`
fusing `a*b + c` into 3 `vfmadd` instructions, which changes the **C's own**
float results (a single rounding step instead of two). Turning contraction off
restores exact parity. The reference build defined by `c_src/CMakeLists.txt`
does not enable FMA, so the translation matches the ground truth as built.

## Comparison strictness (what "byte-identical" means here, and its one bound)

Every assertion compares `f32::to_bits()`, so signed zeros, infinities, and all
finite values are compared strictly. Integer outputs (`c2Support`'s index,
`c2GJK`'s `*iterations`, and the cache's `count`/`iA`/`iB`) are compared with
plain `==` and have **no exemption of any kind**.

The single exemption is the *sign and payload of a produced NaN*
(`common::feq` treats "both NaN" as equal). `tests/strictness_bounds.rs` bounds
that exemption instead of taking it on faith:

| assertion | measured result |
|-----------|-----------------|
| `gjk`: every strict-bit difference has BOTH sides NaN, and only for NaN/inf inputs | 200 000 calls, 10 434 differing out-vectors, all NaN-only |
| `c2GJK`: same, plus `iterations`/cache compared exactly | 70 NaN-only differences; 70 947 calls with non-NaN `dist` all matched exactly |
| exported leaves: every non-NaN result bit-identical | 900 000 calls; NaN-sign-only differences confined to `c2Add`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv`, `c2Dot`, `c2Det2` (release) |
| simplex functions: every non-NaN field and every int field bit-identical | 28 000 cases, 1 690 NaN-sign-only differences |
| **normal-range inputs, no exemption at all** | **836 000 strict comparisons, ZERO differences** |

Why the NaN exemption is correct rather than a concession:

* There is no stable *set* of affected functions — it depends on the
  optimization level of both libraries. Measured: Rust release affects
  `c2Add, c2Mulrv, c2MulrvT, c2Mulxv, c2Dot, c2Det2`; Rust debug affects
  `c2Mulvs, c2Div, c2Norm, c2Mulrv, c2MulrvT, c2Mulxv, c2Dot, c2Det2, c2Len`.
  On SSE, `addss`/`subss`/`mulss` propagate whichever NaN operand sits in the
  destination register, and both GCC and LLVM treat these ops as commutable, so
  operand order — and therefore which NaN survives — is a codegen choice with no
  source-level expression.
* IEEE-754 does not specify the sign or payload of a produced NaN.
* Decisively, **the C library disagrees with itself**. Built at `-O0` (the
  reference CMake build) and at `-O2`, `gjk` returns different NaN sign bits for
  the same input. Measured on six NaN-containing inputs:

  | case | C `-O0` | C `-O2` | Rust | O0==O2 | O0==Rust | O2==Rust |
  |------|---------|---------|------|--------|----------|----------|
  | 0 | `7fc00000` | `7fc00000` | `7fc00000` | yes | yes | yes |
  | 1 | `ffc00000,ffc00000` | `ffc00000,7fc00000` | `ffc00000,7fc00000` | **no** | no | yes |
  | 2 | `7fc00000` | `7fc00000` | `7fc00000` | yes | yes | yes |
  | 3 | `7fc00000,ffc00000` | `7fc00000,7fc00000` | `7fc00000,7fc00000` | **no** | no | yes |
  | 4 | `7fc00000,ffc00000` | `7fc00000,7fc00000` | `7fc00000,7fc00000` | **no** | no | yes |
  | 5 | `7fc00000` | `7fc00000` | `7fc00000` | yes | yes | yes |

  The Rust matches C `-O2` on all six, and matches C `-O0` on every case where
  the C is self-consistent. A NaN sign bit is therefore an artifact of one
  particular GCC codegen, not a property of the C's semantics.
