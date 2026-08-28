# CONFIGS.md — configuration surface (valid inputs) of `c_src/src/lib.c`

Mechanically derived from the branch points the C actually takes. Sources of the
axes below:

```sh
grep -nE 'switch *\(|case |default:'  c_src/src/lib.c    # 9 switches
grep -nE 'if *\(|else'                c_src/src/lib.c    # every data-dependent branch
grep -nE '^[a-z].*\(.*\) *\{|^(int|float|void|c2[a-zA-Z]+) '  c_src/src/lib.c   # 38 entry points
cat  c_src/include/lib.h                                    # 1 declared entry point
nm -D --defined-only  …libharvest-work-huLMrZ.so            # 38 exported entry points
```

## Cargo feature axes

`translation/Cargo.toml` declares **no `[features]` table**, so the *only*
feature combination is the default (empty) one. `cargo test`,
`cargo test --no-default-features` and `cargo test --all-features` are therefore
the same build. This is verified by `scripts/check_features.sh`.

## Runtime option axes (grep of the public API)

| axis | values the C branches on | branch site |
|------|--------------------------|-------------|
| `C2_TYPE type` (`c2MakeProxy`) | `0 CIRCLE` (radius=r, count=1), `1 AABB` (radius=0, count=4), `2 CAPSULE` (radius=r, count=2), *other* (writes nothing) | L114 |
| `C2_TYPE typeA × typeB` (`c2Collided`) | 3 × 3 dispatch + 4 `default:` arms; note the **argument swap** for the 4 mixed pairs | L577-616 |
| `c2x *ax_ptr` / `*bx_ptr` (`c2GJK`) | `NULL` ⇒ identity; non-`NULL` ⇒ arbitrary rotation+translation. 2 × 2 = 4 combos | L368/L372 |
| `int use_radius` (`c2GJK`) | `0` ⇒ raw simplex distance; non-`0` ⇒ radius-shrunk distance with 2 sub-branches | L482 |
| `c2GJKCache *cache` (`c2GJK`) | `NULL`; non-`NULL` with `count==0`; non-`NULL` with `count ∈ {1,2,3}` (warm start) | L383/L500 |
| `c2v *outA` / `*outB` / `int *iterations` | `NULL` / non-`NULL` (2³ = 8 combos, output-only) | L510-515 |
| `s->count` (`c22`,`c23`,`c2D`,`c2L`,`c2Witness`,`c2GJKSimplexMetric`) | `1`, `2`, `3`, and the `default:` arm | L161,283,313,348 |

## Input-shape axes

| axis | shapes the C special-cases |
|------|----------------------------|
| proxy vertex count | `1` (circle), `2` (capsule), `4` (AABB) — drives `c2Support`'s loop trip count |
| radius | `0` (AABB), `> 0`, `== 0`, `< 0`, `NaN`, `inf` |
| separation | fully separated, exactly touching, overlapping, identical shapes, one inside the other |
| capsule shape | non-degenerate segment, zero-length segment (`a == b`), axis-aligned, diagonal |
| AABB shape | non-degenerate, zero-area (`min == max`), inverted (`min > max`) |
| float magnitude | small (`~1`), large (`~1e18`, so `dot` overflows), denormal, `±0`, `±inf`, `NaN` |
| simplex `div` | `> 0`, `== ±0` (⇒ `den = ±inf`), `NaN` |
| GJK termination | `hit` (count→3), `d1 > d0`, `dot(d,d) < eps²`, duplicate support point, `iter == 20` |

## Rows — one per combination the C treats differently

Each row is exercised with **many** pseudo-random inputs (fixed seed, `xorshift`
PRNG shared by all rows) unless the row is a fixed-configuration row, in which
case the *shape parameters* are randomised. `[x]` = passing.

### Group 1 — leaf vector maths (lowest level, called by everything else)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C1  | `c2V` | random finite `(x,y)`; plus `±0`, `±inf`, `NaN`, denormal, `FLT_MAX` | [x] |
| C2  | `c2Mulvs` | random finite vector × random finite scalar | [x] |
| C3  | `c2Mulvs` | scalar `0` / `±inf` / `NaN` against `±0`/`±inf` vector (⇒ `0*inf = NaN`) | [x] |
| C4  | `c2Maxv` / `c2Minv` | random finite pairs; equal components; `±0` vs `+0`; one operand `NaN` in each of the 4 positions | [x] |
| C5  | `c2Clampv` | `a` below `lo`, inside, above `hi`, `lo > hi` (inverted), any `NaN` | [x] |
| C6  | `c2Sub` / `c2Add` | random finite; overflow to `±inf`; `inf - inf = NaN`; `NaN` operands | [x] |
| C7  | `c2Dot` | random finite; overflow; `0 * inf`; `NaN` operands (both orders) | [x] |
| C8  | `c2Det2` | random finite; degenerate/parallel (`det == ±0`); overflow; `NaN` | [x] |
| C9  | `c2Len` | random finite; `{0,0}`; huge (overflow ⇒ `inf`); `NaN` | [x] |
| C10 | `c2Div` | `b` random finite, `b == ±0`, `b == ±inf`, `b == NaN`, `a == {0,0}` | [x] |
| C11 | `c2Norm` | random finite; zero vector (⇒ `NaN`); unit vector; huge vector | [x] |
| C12 | `c2Neg` / `c2Skew` / `c2CCW90` | random finite; `±0` (sign-bit flip observable); `NaN` (sign-bit flip) | [x] |
| C13 | `c2RotIdentity` / `c2xIdentity` | no inputs — exact bit pattern of the returned struct | [x] |
| C14 | `c2Mulrv` / `c2MulrvT` | identity rot; random unit rot (`c=cosθ,s=sinθ`); non-normalised rot; `c=s=0`; `NaN` rot | [x] |
| C15 | `c2Mulxv` | identity `c2x`; translation only; rotation only; both; `NaN` in `p` or `r` | [x] |

### Group 2 — proxies

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C16 | `c2BBVerts` | random AABB; zero-area (`min==max`); inverted (`min>max`); `NaN` corner | [x] |
| C17 | `c2MakeProxy` | `type = C2_TYPE_CIRCLE`, random circle (`r>0`, `r==0`, `r<0`) — asserts `radius`, `count==1`, **all 8** `verts` slots | [x] |
| C18 | `c2MakeProxy` | `type = C2_TYPE_AABB`, random AABB incl. inverted — asserts `radius==0`, `count==4`, all 8 slots | [x] |
| C19 | `c2MakeProxy` | `type = C2_TYPE_CAPSULE`, random capsule incl. `a==b` — asserts `radius`, `count==2`, all 8 slots | [x] |
| C20 | `c2MakeProxy` | pre-filled `c2Proxy` (random garbage) + each valid `type` — proves the untouched `verts[count..8]` slots are preserved identically | [x] |

### Group 3 — simplex internals (called by `c2GJK`; exported, so tested directly)

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C21 | `c2GJKSimplexMetric` | `count = 1` | [x] |
| C22 | `c2GJKSimplexMetric` | `count = 2`, random `a.p`/`b.p` (incl. equal ⇒ metric `0`) | [x] |
| C23 | `c2GJKSimplexMetric` | `count = 3`, random triangle (incl. degenerate/collinear ⇒ `det == ±0`) | [x] |
| C24 | `c22` | `count = 2`, random `a.p`,`b.p` — hits all 3 arms (`v<=0`, `u<=0`, else); asserts the **whole 152-byte simplex** after the call | [x] |
| C25 | `c22` | `a.p == b.p` (⇒ `u = v = 0` ⇒ first arm) | [x] |
| C26 | `c22` | origin strictly inside segment `ab` (⇒ else arm, `count = 2`) | [x] |
| C27 | `c23` | `count = 3`, random triangle — the randomised sweep covers all 7 arms; asserts the whole simplex | [x] |
| C28 | `c23` | origin **inside** the triangle (⇒ else arm, `count = 3`) | [x] |
| C29 | `c23` | degenerate triangle, `area == 0` (⇒ `uABC=vABC=wABC=±0`) | [x] |
| C30 | `c23` | `a.p == b.p == c.p` (fully degenerate) | [x] |
| C31 | `c2D` | `count = 1` | [x] |
| C32 | `c2D` | `count = 2`, `det > 0` (⇒ `c2Skew`) and `det <= 0` (⇒ `c2CCW90`), plus `det == 0` | [x] |
| C33 | `c2D` | `count = 3` | [x] |
| C34 | `c2Support` | `count = 1` | [x] |
| C35 | `c2Support` | `count = 2`, random verts and random `d` | [x] |
| C36 | `c2Support` | `count = 4` (AABB proxy), random `d` incl. axis-aligned ties | [x] |
| C37 | `c2Support` | `count = 8` (full `verts[8]` array) | [x] |
| C38 | `c2Support` | all vertices equal (every `dot` ties ⇒ index `0` wins) | [x] |
| C39 | `c2Witness` | `count = 1`, random `sA`/`sB` | [x] |
| C40 | `c2Witness` | `count = 2`, random `u`, `div > 0` | [x] |
| C41 | `c2Witness` | `count = 3`, random `u`, `div > 0` | [x] |
| C42 | `c2Witness` | `div == ±0` at each `count` (⇒ `den = ±inf`, `inf*0 = NaN`) | [x] |
| C43 | `c2L` | `count = 1` | [x] |
| C44 | `c2L` | `count = 2`, `div > 0` and `div == ±0` | [x] |
| C45 | `c2L` | `count = 3` (falls in `default:` ⇒ `{0,0}`) | [x] |

### Group 4 — `c2GJK`, the low-level composed pipeline

`c2GJK` has 3 (`typeA`) × 3 (`typeB`) × 2 (`ax_ptr`) × 2 (`bx_ptr`) × 2
(`use_radius`) × 3 (`cache`) = 216 distinguishable combinations. The rows below
group them the way the C actually branches; every row is driven with many
random shapes, and rows C46-C54 sweep the **full 3×3 type cross-product**.

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C46 | `c2GJK` | `typeA×typeB = CIRCLE×CIRCLE`, `ax=bx=NULL`, `use_radius=0`, `cache=NULL`; random shapes; asserts `rc`, `*outA`, `*outB`, `*iterations` | [x] |
| C47 | `c2GJK` | `CIRCLE×AABB`, same options | [x] |
| C48 | `c2GJK` | `CIRCLE×CAPSULE`, same options | [x] |
| C49 | `c2GJK` | `AABB×CIRCLE`, same options | [x] |
| C50 | `c2GJK` | `AABB×AABB`, same options | [x] |
| C51 | `c2GJK` | `AABB×CAPSULE`, same options | [x] |
| C52 | `c2GJK` | `CAPSULE×CIRCLE`, same options | [x] |
| C53 | `c2GJK` | `CAPSULE×AABB`, same options | [x] |
| C54 | `c2GJK` | `CAPSULE×CAPSULE`, same options | [x] |
| C55 | `c2GJK` | full 3×3 types, `use_radius = 1`, `ax=bx=NULL`, `cache=NULL` | [x] |
| C56 | `c2GJK` | full 3×3 types, `ax_ptr` non-`NULL` (random rot+translation), `bx_ptr = NULL` | [x] |
| C57 | `c2GJK` | full 3×3 types, `ax_ptr = NULL`, `bx_ptr` non-`NULL` | [x] |
| C58 | `c2GJK` | full 3×3 types, **both** transforms non-`NULL`, `use_radius ∈ {0,1}` | [x] |
| C59 | `c2GJK` | non-normalised `c2r` (`c²+s² ≠ 1`) in `ax`/`bx` — the C never normalises | [x] |
| C60 | `c2GJK` | `cache != NULL` starting zeroed (`count = 0`) — cold start, then read back the written cache | [x] |
| C61 | `c2GJK` | `cache != NULL` **warm-started by a previous call** (the real consumer pattern): call `c2GJK` twice with the same cache and compare `rc`, outputs *and* the full cache struct after each call | [x] |
| C62 | `c2GJK` | `cache` warm-start chain of 5 calls with the shape *moving* between calls (cache indices become stale) | [x] |
| C63 | `c2GJK` | `cache->count ∈ {1,2,3}` hand-crafted with valid `iA`/`iB` for the proxy | [x] |
| C64 | `c2GJK` | `cache->count ∈ {1,2,3}` × `metric ∈ {-1e9, -FLT_MAX, -inf, -1.00000001e8, -1e8, -9.9e7, 0, NaN, +inf}` — drives **both** sides of the `!(min < max*2 && metric < -1.0e8f)` validity test (E13 *and* E14), which a plain zeroed cache never reaches | [x] |
| C65 | `c2GJK` | `outA = NULL` only / `outB = NULL` only / `iterations = NULL` only / all three `NULL` (8 combos) | [x] |
| C66 | `c2GJK` | deeply overlapping shapes ⇒ `hit = 1` path (`count` reaches 3) | [x] |
| C67 | `c2GJK` | exactly touching shapes (`dist == rA+rB`) ⇒ midpoint branch, `use_radius = 1` | [x] |
| C68 | `c2GJK` | identical shapes at the identical position (⇒ `dist = 0`, `d == {0,0}` early break) | [x] |
| C69 | `c2GJK` | zero-length capsule (`a == b`) as A and/or B | [x] |
| C70 | `c2GJK` | zero-area AABB (`min == max`) as A and/or B | [x] |
| C71 | `c2GJK` | inverted AABB (`min > max`) as A and/or B | [x] |
| C72 | `c2GJK` | `radius = 0` on circle/capsule with `use_radius = 1` | [x] |
| C73 | `c2GJK` | negative radius with `use_radius = 1` (E23) | [x] |
| C74 | `c2GJK` | huge coordinates (`~1e18`, `dot` overflows to `inf`) | [x] |
| C75 | `c2GJK` | denormal coordinates and radii | [x] |
| C76 | `c2GJK` | `NaN` / `±inf` coordinates and radii, `use_radius ∈ {0,1}` (E24, E25) | [x] |
| C77 | `c2GJK` | shapes placed so the loop runs to `iter == 20` / breaks on `d1 > d0` / breaks on `dup` (E26-E29) — the randomised sweep records which terminator fired and asserts C and Rust agree on `*iterations` | [x] |

### Group 5 — boolean collision routines and the public entry point

| #  | entry point(s) | configuration (options set + input shape) | [x] |
|----|----------------|-------------------------------------------|-----|
| C78 | `c2AABBtoAABB` | random pairs: separated on x, on y, on both, overlapping, touching, nested, zero-area, inverted | [x] |
| C79 | `c2CircletoCircle` | separated / touching (`d2 == r2`) / overlapping / concentric / `r = 0` | [x] |
| C80 | `c2CircletoAABB` | centre inside / outside on each of the 8 Voronoi regions / on an edge / on a corner / zero-area AABB / inverted AABB | [x] |
| C81 | `c2CircletoCapsule` | `da < 0` (before `a`) / `db < 0` (middle, hits the division) / `db >= 0` (past `b`) / `a == b` (E57) / `r = 0` | [x] |
| C82 | `c2AABBtoCapsule` | random pairs incl. touching, nested, `a == b` capsule, zero-area AABB | [x] |
| C83 | `c2CapsuletoCapsule` | random pairs: crossing, parallel, collinear, touching, nested, `a == b` on either side | [x] |
| C84 | `c2Collided` | `typeA×typeB` = full 3×3 valid cross-product, random shapes (validates the argument **swap** for the 4 mixed pairs) | [x] |
| C85 | `capsule` | random `(min_x,min_y,max_x,max_y,r)` in the interesting range (must hit all 8 result bit patterns that are reachable) | [x] |
| C86 | `capsule` | boundary values: `r = 0`, `r < 0`, `min == max`, huge/denormal coords, `±0`, `±inf`, `NaN` (E73, E74) | [x] |
| C87 | `capsule` | the exact three collision configurations hard-coded in the C (circle at `(-70,0) r20`, AABB `(-40,-40)..(-15,-15)`, capsule `(-40,40)..(-20,100) r10`) probed on a dense grid so each of the 3 result bits flips both ways | [x] |
