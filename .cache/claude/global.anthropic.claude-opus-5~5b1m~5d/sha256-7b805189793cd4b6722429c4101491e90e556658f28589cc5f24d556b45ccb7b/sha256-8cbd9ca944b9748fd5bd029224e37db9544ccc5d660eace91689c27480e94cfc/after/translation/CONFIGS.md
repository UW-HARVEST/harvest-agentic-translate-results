# CONFIGS.md — Phase B configuration surface table (VALID inputs)

Derived mechanically from the branch structure of `c_src/src/lib.c`.

## Axes the C code actually branches on

| axis | values the C distinguishes | source |
|------|---------------------------|--------|
| `typeA` / `typeB` | `C2_TYPE_CIRCLE`(1 vert, r), `C2_TYPE_AABB`(4 verts, r=0), `C2_TYPE_CAPSULE`(2 verts, r) | `c2MakeProxy` switch, lib.c:109 |
| `ax_ptr` / `bx_ptr` | `NULL` ⇒ identity, non-NULL ⇒ arbitrary rotation+translation | lib.c:363-370 |
| `use_radius` | `0` skips the whole radius block, `!=0` runs it | lib.c:477 |
| `iterations` | `NULL` / non-NULL | lib.c:509 |
| `cache` | `NULL`; non-NULL cold (`count==0`); non-NULL warm (fed back from a previous call) | lib.c:378, 495 |
| simplex `count` | `1`, `2`, `3`, plus `default` arms (`0`, `>3`, negative) | `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric` switches |
| separation regime | **overlapping** (`hit==1`, simplex reaches 3) / **touching** (`dist <= rA+rB`) / **separated** (`dist > rA+rB && dist > FLT_EPS`) | lib.c:436, 480 |
| shape degeneracy | zero-extent AABB (point), zero-length capsule (= circle), zero radius, negative radius, inverted AABB | no validation anywhere |
| `c2Support` count | `1`, `2`, `4`, and `>4` (proxy `verts[8]`) | lib.c:296 loop |
| `gjk` `reverse` | `0` / nonzero ⇒ operand order swap | lib.c:525 |
| float value class | normal, ±0, subnormal, huge (near `FLT_MAX`), ±Inf, NaN | pure IEEE-754 propagation, no guards |
| feature combos | **none** — `Cargo.toml` has no `[features]`; single default build | Cargo.toml |

## Combination rows

Each row is run against **many randomized inputs with a fixed seed** (a
deterministic SplitMix64 PRNG in `tests/common/mod.rs`), comparing C and Rust
`.so` exports **bit-for-bit** (`f32::to_bits`, and full struct byte compare for
out-params).

### Group 1 — leaf vector/scalar helpers (lowest level first)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random normals; ±0; subnormals; ±Inf; NaN (incl. signalling & payload-carrying) | [x] |
| 2 | `c2Dot` | random normals; mixed magnitudes forcing cancellation; ±0; Inf×0 ⇒ NaN; NaN operands both sides | [x] |
| 3 | `c2Det2` | same value classes as `c2Dot`; also `a==b` ⇒ exactly 0; antiparallel | [x] |
| 4 | `c2Sub`, `c2Add` | random normals; `x-x` ⇒ ±0; Inf−Inf ⇒ NaN; NaN L vs NaN R (which NaN survives) | [x] |
| 5 | `c2Mulvs` | scalar = random, 0, −0, 1, Inf, NaN × vector of each value class | [x] |
| 6 | `c2Div` | divisor = random, ±0, ±Inf, NaN, subnormal, `FLT_MIN` | [x] |
| 7 | `c2Len` | random; zero vector; huge components (overflow to Inf inside `dot`); subnormal; NaN | [x] |
| 8 | `c2Norm` | random; unit; zero ⇒ NaN; huge ⇒ Inf path; NaN | [x] |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | all value classes incl. ±0 sign flip and NaN sign-bit flip | [x] |
| 10 | `c2Maxv`, `c2Minv` | random; equal components; +0 vs −0 (C uses `>`/`<` so ties take the 2nd/2nd arg); NaN in either arg (comparison false ⇒ specific arg wins) | [x] |
| 11 | `c2Clampv` | lo<hi normal; lo>hi (inverted, no validation); a inside/below/above; NaN in a / lo / hi | [x] |
| 12 | `c2RotIdentity`, `c2xIdentity` | no args — exact bit pattern of the returned structs | [x] |
| 13 | `c2Mulrv`, `c2MulrvT` | `c2r` = identity, real `(cos θ, sin θ)` for many θ, unnormalised (c,s) incl. 0, Inf, NaN × vector value classes | [x] |
| 14 | `c2Mulxv` | `c2x` = identity, pure translation, pure rotation, combined, non-finite components | [x] |

### Group 2 — proxy construction and support

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 15 | `c2BBVerts` | random AABB; zero-extent (min==max); inverted (min>max); non-finite bounds — full 4-vert out-buffer compared | [x] |
| 16 | `c2MakeProxy` | `C2_TYPE_CIRCLE`, random p and r (incl. r=0, r<0, r=Inf/NaN) — whole 72-byte `c2Proxy` compared against a pre-seeded pattern | [x] |
| 17 | `c2MakeProxy` | `C2_TYPE_AABB`, random/degenerate/inverted/non-finite box | [x] |
| 18 | `c2MakeProxy` | `C2_TYPE_CAPSULE`, random a,b,r; a==b (zero-length); r=0/<0/non-finite | [x] |
| 19 | `c2Support` | `count = 1` (circle proxy shape), random `d` incl. zero and NaN | [x] |
| 20 | `c2Support` | `count = 2` (capsule shape), incl. exact ties between the two dots | [x] |
| 21 | `c2Support` | `count = 4` (AABB shape), random `d`, axis-aligned `d` (ties), NaN `d` | [x] |
| 22 | `c2Support` | `count = 8` (full proxy array), random verts and `d` | [x] |

### Group 3 — simplex machinery (driven directly, not via `c2GJK`)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 23 | `c2GJKSimplexMetric` | `count = 1` | [x] |
| 24 | `c2GJKSimplexMetric` | `count = 2`, random `a.p`/`b.p` incl. coincident and non-finite | [x] |
| 25 | `c2GJKSimplexMetric` | `count = 3`, random triangle incl. collinear (area 0) and non-finite | [x] |
| 26 | `c22` | random `a.p`,`b.p` in **general position** ⇒ `else` branch (count 2) — whole 152-byte `c2Simplex` compared | [x] |
| 27 | `c22` | inputs forced into the `v <= 0` branch (origin beyond `a`) ⇒ count 1, `a` kept | [x] |
| 28 | `c22` | inputs forced into the `u <= 0` branch (origin beyond `b`) ⇒ count 1, `s->a = s->b` copy | [x] |
| 29 | `c22` | degenerate `a.p == b.p`; `a.p == b.p == 0`; non-finite ⇒ NaN comparisons | [x] |
| 30 | `c23` | random triangle in general position ⇒ final `else` (count 3, barycentric) | [x] |
| 31 | `c23` | forced vertex region A (`vAB<=0 && uCA<=0`) ⇒ count 1 | [x] |
| 32 | `c23` | forced vertex region B (`uAB<=0 && vBC<=0`) ⇒ count 1, `a=b` | [x] |
| 33 | `c23` | forced vertex region C (`uBC<=0 && vCA<=0`) ⇒ count 1, `a=c` | [x] |
| 34 | `c23` | forced edge region AB (`wABC<=0`) ⇒ count 2 | [x] |
| 35 | `c23` | forced edge region BC (`uABC<=0`) ⇒ count 2 with `a=b, b=c` shift | [x] |
| 36 | `c23` | forced edge region CA (`vABC<=0`) ⇒ count 2 with `b=a, a=c` shift | [x] |
| 37 | `c23` | collinear / duplicate vertices (`area == 0`); all-zero simplex; non-finite | [x] |
| 38 | `c2D` | `count = 1` random `a.p` | [x] |
| 39 | `c2D` | `count = 2`, `c2Det2(ab, -a.p) > 0` ⇒ `c2Skew` branch | [x] |
| 40 | `c2D` | `count = 2`, `c2Det2(ab, -a.p) <= 0` ⇒ `c2CCW90` branch (incl. `== 0` and NaN) | [x] |
| 41 | `c2L` | `count = 1`; `count = 2` with random `u`/`div`; `div` huge/tiny/subnormal | [x] |
| 42 | `c2Witness` | `count = 1` (copies `sA`/`sB` verbatim, `div` irrelevant) | [x] |
| 43 | `c2Witness` | `count = 2`, random `sA`/`sB`/`u`/`div` | [x] |
| 44 | `c2Witness` | `count = 3`, random `sA`/`sB`/`u`/`div` (3-term barycentric, nested `c2Add` order matters) | [x] |

### Group 4 — `c2GJK` full pipeline (the cross-product of the real options)

`shapes` = the 9 ordered `(typeA, typeB)` pairs over {CIRCLE, AABB, CAPSULE}.
Each row below is run for **all 9 shape pairs** × many random instances.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 45 | `c2GJK` | 9 shape pairs, `ax=bx=NULL`, `use_radius=0`, `cache=NULL`, `iterations` captured — **separated** shapes | [x] |
| 46 | `c2GJK` | 9 shape pairs, identity transforms, `use_radius=1`, `cache=NULL` — separated (radius-shrink branch) | [x] |
| 47 | `c2GJK` | 9 shape pairs, `use_radius=1` — **overlapping** shapes ⇒ `hit==1` path | [x] |
| 48 | `c2GJK` | 9 shape pairs, `use_radius=1` — **touching** (`dist <= rA+rB`) ⇒ midpoint path | [x] |
| 49 | `c2GJK` | 9 shape pairs, `use_radius=1`, `dist` in `(0, FLT_EPS]` ⇒ midpoint despite separation | [x] |
| 50 | `c2GJK` | 9 shape pairs, **non-NULL `ax_ptr` only** (rotation+translation on A, identity on B) | [x] |
| 51 | `c2GJK` | 9 shape pairs, **non-NULL `bx_ptr` only** | [x] |
| 52 | `c2GJK` | 9 shape pairs, **both transforms non-NULL**, random rotations `(cos θ, sin θ)` and translations, `use_radius` ∈ {0,1} | [x] |
| 53 | `c2GJK` | 9 shape pairs, both transforms with **unnormalised** `c2r` (c²+s²≠1) — legal, no validation | [x] |
| 54 | `c2GJK` | 9 shape pairs, `iterations = NULL` vs non-NULL — return value and out-params must be identical either way | [x] |
| 55 | `c2GJK` | 9 shape pairs, `outA = NULL`, `outB` set | [x] |
| 56 | `c2GJK` | 9 shape pairs, `outA` set, `outB = NULL` | [x] |
| 57 | `c2GJK` | 9 shape pairs, `outA = outB = NULL` (only return value + iterations observable) | [x] |
| 58 | `c2GJK` | 9 shape pairs, **cold cache** (`c2GJKCache` zeroed) — return value **and the whole 36-byte written cache** compared | [x] |
| 59 | `c2GJK` | 9 shape pairs, **warm cache**: call twice with the same cache object, unchanged shapes; compare both returns and the cache after each | [x] |
| 60 | `c2GJK` | 9 shape pairs, **warm cache + moved shapes**: seed the cache, then translate B and call again (exercises the metric-staleness test at lib.c:400) | [x] |
| 61 | `c2GJK` | 9 shape pairs, warm cache over a **chain of 8 sequential calls** with the shape drifting each step (the real consumer pattern; state carried across calls) | [x] |
| 62 | `c2GJK` | 9 shape pairs, cache with `count == 1/2/3` and **valid in-range** `iA`/`iB` (`< pX.count`) hand-seeded, plus random `metric`/`div` | [x] |
| 63 | `c2GJK` | degenerate shapes: zero-radius circle, zero-extent AABB (point), zero-length capsule, radius 0 capsule — all 9 pairings | [x] |
| 64 | `c2GJK` | inverted AABB (`min > max`) as A and as B | [x] |
| 65 | `c2GJK` | negative radius on circle/capsule (grows `dist` in the radius block) | [x] |
| 66 | `c2GJK` | coincident shapes (A and B at exactly the same place) — forces `hit`/degenerate simplex | [x] |
| 67 | `c2GJK` | very large coordinates (near `FLT_MAX`/1e30) ⇒ overflow to Inf inside `c2Dot` | [x] |
| 68 | `c2GJK` | very small / subnormal coordinates and radii | [x] |
| 69 | `c2GJK` | non-finite (NaN / ±Inf) coordinates and radii ⇒ the 20-iteration loop with NaN comparisons | [x] |
| 70 | `c2GJK` | inputs chosen to drive the loop to the **iteration cap** and to the `d1 > d0` and `dup` early-exits (iteration count compared) | [x] |

### Group 5 — `gjk` top-level wrapper (AABB vs capsule)

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 71 | `gjk` | `reverse = 0`, random box + capsule, separated | [x] |
| 72 | `gjk` | `reverse = 1`, same inputs (operands swapped) | [x] |
| 73 | `gjk` | `reverse` = other nonzero `char` values (`2`, `0x7f`, `-1`, `-128`) — all must behave like `1` | [x] |
| 74 | `gjk` | overlapping box/capsule (`hit` path), both `reverse` values | [x] |
| 75 | `gjk` | touching box/capsule (`dist <= r`), both `reverse` values | [x] |
| 76 | `gjk` | degenerate: zero-extent box, zero-length capsule, `r = 0`, `r < 0` — both `reverse` values | [x] |
| 77 | `gjk` | inverted box (`a1>a3`, `a2>a4`) — both `reverse` values | [x] |
| 78 | `gjk` | non-finite args (NaN / ±Inf in each of the 9 float positions) — both `reverse` values | [x] |
| 79 | `gjk` | `a = NULL` / `b = NULL` / both NULL — both `reverse` values | [x] |
| 80 | `gjk` | large-scale random sweep: 20 000 random 9-float tuples × both `reverse` values, all-bits comparison of `*a` and `*b` | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]`** table and no optional
dependencies, so the complete set of feature combinations is:

| combo | command | status |
|-------|---------|--------|
| default (empty) | `cargo test --release` | [x] |
| `--no-default-features` (identical to default; there is no `default` feature) | `cargo test --release --no-default-features` | [x] |

Both are exercised by `check_features.sh`, which enumerates features from
`Cargo.toml` and runs the full suite for each.

## Binary / driver

`c_src/CMakeLists.txt` builds only `add_library(... SHARED ...)`; there is no
`add_executable`, and the Rust crate is `crate-type = ["cdylib"]` with no
`src/main.rs` / `[[bin]]`. **No binary exists in either project**, so the
"compare C and Rust stdout" gate item is not applicable. `gjk` (rows 71-80) is
the library's driver-equivalent entry point and is exercised exhaustively above.

## Addendum: NaN-provenance rows (added after fault-injection review)

Randomized inputs reach "two NaNs meet at one arithmetic site" only by
coincidence, yet that is exactly where the C's `addss`/`mulss` **destination
operand** decides which NaN payload survives. Mutation testing showed the
original rows did not pin every such site, so these rows were added and are
covered by `tests/phase_c_errors.rs`:

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 81 | `c2Add`, `c2Sub`, `c2Dot`, `c2Det2`, `c2Mulvs`, `c2Div`, `c2Len`, `c2Norm`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Mulrv`, `c2MulrvT`, `c2Mulxv` | the full 8×8 matrix of NaNs with **distinct** payloads/signs (incl. sNaN) on both operands, in each component position (`nan_prov_leaf_operand_matrix`) | [x] |
| 82 | `c2Witness`, `c2L` | `div` NaN **and** `u` NaN with distinct payloads — all 8×8 pairs × `count` ∈ {1,2,3}, plus one-NaN-slot-at-a-time and NaN `sA`/`sB`/`p` on top (`nan_prov_witness_den_times_u`) | [x] |
| 83 | `c22`, `c23` | `div = u + v` and `div = uABC + vABC + wABC` with the addends simultaneously NaN with distinct payloads, over all 6 vertex permutations (`nan_prov_simplex_div_sums`) | [x] |

## Addendum: provably unreachable C branches

Two C branches cannot be reached through the public API. They are recorded here
so the coverage claim is honest rather than silently incomplete:

| C site | why it is unreachable |
|--------|------------------------|
| `while (iter < 20)` reaching the cap (lib.c:420) | `c2MakeProxy` builds a proxy with **at most 4 vertices** (AABB), so the GJK support-point search exhausts the distinct `(iA, iB)` pairs and hits the `dup` break long before 20 iterations. A 1.5-million-sample sweep over all 9 shape pairs × wild transforms × hand-seeded caches (both `.so`s) observed a **maximum of 5** iterations. Mutating the literal `20` to `19` in the Rust is therefore not observable — it is dead code in both implementations. `*iterations` itself IS compared bit-for-bit on every `c2GJK` call in rows 45-70, so any *reachable* change in loop-trip count is caught. |
| `mul_r(den, verts[0].u)` in `c2Witness` / `c2L` | The `c2Add` that consumes this product is `add_r`, i.e. the **later** addend is the `addss` destination. Whenever `den` is NaN (the only way both `mulss` operands can be NaN), the later addend is NaN too and therefore wins unconditionally, masking slot 0's NaN choice. The slot-1 and slot-2 sites *are* observable and *are* pinned by row 82. |

## Fault-injection validation of this table

`CONFIGS.md` + `ERRORS.md` coverage was validated by mutating the Rust and
checking the suite fails. 28 mutants were injected; **22 were caught**. The 6
survivors were each shown to be *equivalent mutants* (no observable behaviour
change), not coverage gaps:

| survivor | why it is equivalent |
|----------|----------------------|
| `div_l(1.0f32, b)` → `1.0f32 / b` (×2 sites) | the left operand is the literal `1.0`, which is never NaN, so the `_l` destination rule degenerates to plain IEEE division |
| `fneg(x)` → `-x` (×2 sites) | Rust's unary `-` on `f32` is a pure sign-bit flip, identical to the `xorps` GCC emits |
| `a.x > b.x` → `b.x < a.x` | IEEE-identical for all inputs, NaN included (both unordered comparisons are false) |
| `add_l(uAB, vAB)` → `add_r`, and likewise for `uBC/vBC`, `uCA/vCA` | those branches are guarded by `uX > 0.0 && vX > 0.0`, which is false if either operand is NaN, so the two addends can never both be NaN |
| `mul_r(den, verts[0].u)` → `mul_l` | see the unreachable-branch table above |

The harness itself is validated by `gate2_harness_is_a_real_negative_control`,
which asserts the two `.so`s are distinct files whose symbols resolve to
different addresses, and that the comparison helpers reject a 1-bit difference,
`+0.0` vs `-0.0`, and two NaNs differing only in payload.
