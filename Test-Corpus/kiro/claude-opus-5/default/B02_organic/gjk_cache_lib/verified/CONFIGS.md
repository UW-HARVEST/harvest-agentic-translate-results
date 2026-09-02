# CONFIGS.md — configuration surface table (valid inputs)

Mechanically derived from the branch structure of `c_src/src/lib.c`. The axes
the C code actually distinguishes:

**Runtime options / modes the public API can set**

| axis | values the C branches on | site |
|------|--------------------------|------|
| `C2_TYPE typeA` | `C2_TYPE_CIRCLE` (1 vert, radius `r`), `C2_TYPE_AABB` (4 verts, radius forced `0`), `C2_TYPE_CAPSULE` (2 verts, radius `r`) | `c2MakeProxy` `switch` |
| `C2_TYPE typeB` | same three | `c2MakeProxy` `switch` |
| `ax_ptr` | `NULL` ⇒ identity transform, non-`NULL` ⇒ arbitrary `c2x` (rotation + translation) | `if (!ax_ptr)` |
| `bx_ptr` | `NULL` / non-`NULL` | `if (!bx_ptr)` |
| `use_radius` | `0` ⇒ core distance, `!=0` ⇒ radius-shrunk distance (two sub-branches) | `else if (use_radius)` |
| `cache` | `NULL`; cold (`count == 0`); warm (`count` 1/2/3 written back by a prior call) | `if (cache)`, `!!cache->count` |
| `outA` / `outB` / `iterations` | `NULL` / non-`NULL` | three `if (ptr)` guards |
| `reverse` (`gjk_cache`) | `0` / non-`0` | `if (reverse)` |
| `s->count` (`c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric`) | `1`, `2`, `3`, and `default` | six `switch`es |
| `count` (`c2Support`) | `1`, `2`, `4`, `8` (proxy sizes actually produced), plus `>1` loop entry | `for (i = 1; i < count; ++i)` |

**Input shapes the code special-cases**

separated / touching / overlapping / concentric; radius `0` vs `>0`;
degenerate AABB (`min == max`, zero width, zero height); degenerate capsule
(`a == b`); collinear simplex (`area == 0`); duplicate simplex vertices;
origin-enclosing simplex; large magnitudes (`1e18`) and tiny ones (`1e-30`,
subnormals); mixed signs; identity vs non-identity rotation.

Every row is exercised with **many randomized inputs** (`SEED = 0x5EED_1234`,
`splitmix64`-driven, ≥256 samples/row unless noted), compared bit-for-bit
(`f32::to_bits`) between the C `.so` and the Rust `.so`.

## Rows

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random finite `(x,y)`; also `±0.0`, `±inf`, `NaN`, subnormals | [x] |
| 2 | `c2Mulvs` | random vector × random scalar; scalar `0`, `-0`, `inf`, `NaN`; huge×huge (overflow) | [x] |
| 3 | `c2Add`, `c2Sub` | random pairs; cancelling pairs (`a-a`); overflow pairs; `±inf` mixes | [x] |
| 4 | `c2Dot` | random pairs; orthogonal pairs; huge magnitudes (overflow to `inf`); `inf*0` ⇒ `NaN` | [x] |
| 5 | `c2Det2` | random pairs; collinear pairs (det `0`); antiparallel; huge magnitudes | [x] |
| 6 | `c2Len` | random vectors; zero vector; huge (`>sqrt(FLT_MAX)`) ⇒ `inf`; subnormal | [x] |
| 7 | `c2Maxv`, `c2Minv` | random pairs; equal components; `±0.0` pairs; one component `NaN`; both `NaN` | [x] |
| 8 | `c2Clampv` | random `a` inside / below / above `[lo,hi]`; inverted range `lo > hi`; `lo == hi`; `NaN` in each slot | [x] |
| 9 | `c2Neg`, `c2Skew`, `c2CCW90` | random vectors; `±0.0` (sign of zero matters); `NaN`; `inf` | [x] |
| 10 | `c2Div`, `c2Norm` | random vector ÷ random scalar; divisor `0` / `inf` / `NaN`; `c2Norm` of unit, huge, subnormal and zero vectors | [x] |
| 11 | `c2RotIdentity`, `c2xIdentity` | no inputs — constant-value parity | [x] |
| 12 | `c2Mulrv`, `c2MulrvT` | random `c2r` (normalised `cos/sin` from random angle) × random vector; also non-normalised `c2r`; identity rotation; `NaN` rotation | [x] |
| 13 | `c2Mulxv` | random `c2x` (rotation + translation) × random vector; identity `c2x`; huge translation | [x] |
| 14 | `c2BBVerts` | random AABB; inverted AABB (`min > max`); zero-area (`min == max`); zero-width; zero-height; `NaN` bounds. Output buffer pre-filled identically, all 8 slots compared so untouched tail is checked | [x] |
| 15 | `c2MakeProxy` | `type = CIRCLE` — pre-filled `c2Proxy`, verify `radius`/`count`/`verts[0]` set and `verts[1..8]` untouched | [x] |
| 16 | `c2MakeProxy` | `type = AABB` — `radius` forced to `0`, `count = 4`, `verts[0..4]` set from `c2BBVerts`, `verts[4..8]` untouched | [x] |
| 17 | `c2MakeProxy` | `type = CAPSULE` — `radius = r`, `count = 2`, `verts[0..2]` set, `verts[2..8]` untouched | [x] |
| 18 | `c2Support` | `count = 1` (circle proxy shape) with random `d` incl. `d = (0,0)` | [x] |
| 19 | `c2Support` | `count = 2` (capsule proxy shape), random verts and `d` | [x] |
| 20 | `c2Support` | `count = 4` (AABB proxy shape), random verts and `d`; ties (all verts equal) | [x] |
| 21 | `c2Support` | `count = 8` (full `c2Proxy.verts` capacity), random verts and `d`; `NaN` among the dots | [x] |
| 22 | `c22` | random 2-vertex simplex ⇒ hits all three branches (`v<=0`, `u<=0`, interior) across the sample set; plus duplicate `a.p == b.p`, `a.p == -b.p`, origin on segment | [x] |
| 23 | `c23` | random 3-vertex simplex ⇒ all seven branches; plus collinear triangle (`area == 0`), duplicated vertices, origin-enclosing triangle, origin on an edge | [x] |
| 24 | `c2D` | `count = 1`, `2` (both `det > 0` and `det <= 0` sub-branches), `3`, and out-of-range `count` | [x] |
| 25 | `c2L` | `count = 1`, `2` (random `u`/`div`), `3`/out-of-range; `div = 0` | [x] |
| 26 | `c2Witness` | `count = 1`, `2`, `3`, out-of-range; random `u` and `div`; `div = 0`; `div` huge | [x] |
| 27 | `c2GJKSimplexMetric` | `count = 1`, `2`, `3`, `0`, negative, `>3`; random simplex points | [x] |
| 28 | `c2GJK` | circle vs circle, identity transforms (`ax=bx=NULL`), `use_radius=0`, `cache=NULL` | [x] |
| 29 | `c2GJK` | circle vs circle, identity, `use_radius=1`, `cache=NULL` — separated, touching, overlapping, concentric | [x] |
| 30 | `c2GJK` | circle vs AABB, identity, `use_radius` ∈ {0,1}, `cache=NULL` | [x] |
| 31 | `c2GJK` | circle vs capsule, identity, `use_radius` ∈ {0,1}, `cache=NULL` | [x] |
| 32 | `c2GJK` | AABB vs circle, identity, `use_radius` ∈ {0,1} | [x] |
| 33 | `c2GJK` | AABB vs AABB, identity, `use_radius` ∈ {0,1} — disjoint, touching, overlapping, nested, degenerate (zero-area) | [x] |
| 34 | `c2GJK` | AABB vs capsule, identity, `use_radius` ∈ {0,1} | [x] |
| 35 | `c2GJK` | capsule vs circle, identity, `use_radius` ∈ {0,1} | [x] |
| 36 | `c2GJK` | capsule vs AABB, identity, `use_radius` ∈ {0,1} | [x] |
| 37 | `c2GJK` | capsule vs capsule, identity, `use_radius` ∈ {0,1} — parallel, crossing, collinear, degenerate (`a == b`) | [x] |
| 38 | `c2GJK` | all 9 type pairs, **non-NULL `ax_ptr`** (random rotation + translation), `bx_ptr = NULL` | [x] |
| 39 | `c2GJK` | all 9 type pairs, `ax_ptr = NULL`, **non-NULL `bx_ptr`** | [x] |
| 40 | `c2GJK` | all 9 type pairs, **both** transforms non-NULL, random rotations/translations, `use_radius=1` | [x] |
| 41 | `c2GJK` | all 9 type pairs, both transforms non-NULL, `use_radius=0` | [x] |
| 42 | `c2GJK` | cold cache (`count = 0`) then **warm cache reuse** — call twice with the same shapes, compare both return values, both witness pairs, both `iterations` and the full `c2GJKCache` after each call | [x] |
| 43 | `c2GJK` | warm cache **reused after moving the shapes** (transforms changed between calls) — exercises the near-dead staleness guard | [x] |
| 44 | `c2GJK` | warm cache carried across a **type change** (cache from circle/capsule reused for AABB/capsule) with `cache->iA/iB` still in range of the new proxies | [x] |
| 45 | `c2GJK` | long-chain cache reuse: 8 successive calls sharing one cache while shapes drift, full cache compared after each | [x] |
| 46 | `c2GJK` | `outA = NULL`, `outB` non-NULL; `outA` non-NULL, `outB = NULL`; both NULL; `iterations = NULL` — with `use_radius` ∈ {0,1} | [x] |
| 47 | `c2GJK` | radius edge shapes: `r = 0` circle, `r = 0` capsule, huge `r`, `r` such that `dist == rA+rB` exactly (touching-after-shrink) | [x] |
| 48 | `c2GJK` | shapes far apart (`1e18` coordinates) and extremely close (`1e-30` separation, subnormal witness deltas) | [x] |
| 49 | `c2GJK` | deeply overlapping shapes forcing `hit = 1` (`s.count == 3`) for every type pair | [x] |
| 50 | `c2GJK` | inputs tuned to force the 20-iteration cap / duplicate-support break / `d1 > d0` break, `iterations` compared | [x] |
| 51 | `gjk_cache` | `reverse = 0`, random `a1..a4`, `b1..b5`; `a9`/`b9` non-NULL pre-filled (must stay untouched) | [x] |
| 52 | `gjk_cache` | `reverse = 1`, same input sweep | [x] |
| 53 | `gjk_cache` | `reverse` = other non-zero `char` values (`-1`, `2`, `127`, `-128`); `a9 = b9 = NULL` | [x] |
| 54 | `gjk_cache` | degenerate AABB/capsule arguments (inverted AABB, zero-size AABB, `b1..b4` equal, `b5 = 0`, NaN/inf floats) | [x] |
| 55 | full pipeline | `c2MakeProxy` → `c2Support` → `c22`/`c23` → `c2D`/`c2L` → `c2Witness` driven **manually** in a randomized 20-step loop (low-level entry points only, mirroring `c2GJK`'s composition without calling it), simplex compared byte-for-byte at every step | [x] |

## How the rows are executed

| file | rows | tests |
|------|------|-------|
| `tests/phase_b_math.rs` | 1-21 | 22 |
| `tests/phase_b_simplex.rs` | 22-27, 55 | 7 |
| `tests/phase_b_gjk.rs` | 28-50 | 22 |
| `tests/phase_b_gjkcache.rs` | 51-54 | 4 |
| `tests/phase_c_errors.rs` | (`ERRORS.md` 1-52 + generic FFI boundaries) | 32 |
| `tests/symbols.rs` | Phase D symbol parity / struct layout | 4 |
| `tests/search.rs` | wide randomized hunts + reachability measurements | 3 |

Both `.so`s are loaded with `libloading` and every call goes through an exported
C symbol; the Rust crate is never linked directly, so the `#[no_mangle]` wrappers
are themselves under test. Comparison is `f32::to_bits` per field (NaN payload
excepted — see `ERRORS.md`), plus raw byte comparison for out-parameters and for
"must not be written" assertions.

## Build hazard worth knowing

`cargo test` does **not** build a `cdylib`. Running only `cargo test` will load
whatever `libgjk_cache_lib.so` happens to be in `target/<profile>/` from an
earlier build, so an edited `src/lib.rs` would silently *not* be under test.
The harness therefore asserts that the `.so` is newer than everything under
`src/` and `Cargo.toml`, and `./verify_all.sh` always runs `cargo build` before
`cargo test`. This was a real defect in the first version of this harness.

## Suite sensitivity (mutation testing)

Each mutation below was applied to `src/lib.rs`, the cdylib rebuilt, and the
full suite re-run. "caught by N tests" is the number of failing tests.

| mutation | caught by |
|----------|-----------|
| `c2Det2` sign flip | 19 |
| `c2Skew` / `c2CCW90` swapped | 19 |
| `c2Maxv` NaN ties select `a` instead of `b` | 3 |
| `c22`: `v <= 0` → `v < 0` | 10 |
| `c23`: `wABC <= 0` → `wABC < 0` | 13 |
| `c23`: interior barycentric order | 18 |
| `c2GJK`: `eps*eps` → `eps` | 3 |
| `c2GJK`: staleness guard sign | 5 |
| `c2GJK`: `cache->count != 0` → `> 0` | 2 |
| `c2GJK`: radius applied to the wrong witness | 19 |
| `c2GJK`: `dist > FLT_EPSILON` → `>=` | 2 |
| `c2GJK`: `hit` sets `b = a` instead of `a = b` | 19 |
| `c2MulrvT` sign (transpose dropped) | 9 |
| `c2MakeProxy`: AABB radius not forced to 0 | 16 |
| `c2MakeProxy`: capsule `count = 1` | 19 |
| `c2BBVerts`: corner order | 18 |
| `c2Support`: `>` → `>=` | 22 |
| `c2Witness`: `den = div` instead of `1/div` | 22 |

Three mutations were **not** caught. All three were investigated and are
provably unobservable through the public ABI, not test gaps:

1. `c2GJK`: `d1 > d0` → `d1 >= d0`. `d1 == d0` *is* reachable (2 836 times in
   400 000 replayed runs, `search_d1_eq_d0`), but whenever it happens the next
   support point is a duplicate, so the loop breaks anyway without incrementing
   `s.count` or `iter`. A 300 000-input differential hunt against a deliberately
   mutated `.so` (`search_any_c2GJK_divergence` with `C2_RUST_SO`) found **zero**
   distinguishing inputs.
2. `c2GJK`: `while (iter < 20)` → `iter < 19`. A `c2Proxy` holds at most 4
   vertices, so the simplex saturates first: the observed maximum `*iterations`
   over 400 000 randomized calls (including primed caches) is **3**
   (`search_max_iterations`). Iterations 4..20 are unreachable through the
   public API.
3. `gjk_cache`: `if (reverse)` inverted. `gjk_cache` returns `void`, never
   dereferences `a9`/`b9`, and discards every `c2GJK` result it computes, so
   `reverse` has **no** observable effect. The only testable properties — it
   must not fault and must not write through either pointer — are asserted with
   canary-guarded buffers (rows 51-54, `ERRORS.md` rows 50-52).

## Completion status

* `SYMBOLS.md`: 31/31 symbols exported by both `.so`s; `comm -23` empty; 0
  undefined non-libc symbols in the Rust `.so`. Re-checked per feature
  combination and per profile by `verify_all.sh`.
* Phase B: all 55 rows pass across randomized inputs (fixed seed
  `0x5EED_1234_ABCD_9876`).
* Phase C: all 52 `ERRORS.md` rows pass; the UB-only rows are documented with
  the evidence that makes them untestable.
* Feature combinations: `Cargo.toml` declares no `[features]`, so the default
  and `--no-default-features` builds are the complete set; both pass in the
  `dev` and `release` profiles.
* The suite additionally passes against the C source compiled at `-O1`, `-O2`,
  `-O3` and `-Os`, not just the CMake default, so the match is to the C's
  semantics rather than to one particular build.
