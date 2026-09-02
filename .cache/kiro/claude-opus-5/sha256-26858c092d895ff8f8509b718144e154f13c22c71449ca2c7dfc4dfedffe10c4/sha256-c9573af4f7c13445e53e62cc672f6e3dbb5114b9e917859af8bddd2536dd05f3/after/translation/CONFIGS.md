# CONFIGS.md — configuration / valid-input surface table

Derived mechanically from the branches the C actually takes.

```sh
grep -nE "\?|switch|case|<|>|\||!" c_src/src/lib.c
nm -D --defined-only c_src/build/libharvest-work-oWYE5y.so   # the FULL public API, 10 entry points
```

## Axes the C branches on

**A. Runtime options / modes.** The only runtime-selectable mode in the whole
library is the pair of `C2_TYPE` tags handed to `collided` (`lib.h:1-4`). They
drive a nested `switch` (`lib.c:74-97`) with 4 valid combinations, and the
`AABB × CIRCLE` arm *swaps* the operands (`c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)`
— B is the circle, A is the box). There are no `#ifdef`s, no global state, no
init/config struct, and no flags.

**B. Entry points — all 10 exported symbols, lowest level first.** Phase B drives
the low-level ones (`c2V`, `c2Maxv`, `c2Minv`, `c2Clampv`, `c2Sub`, `c2Dot`)
directly, not only through the `collided` one-shot wrapper, and also drives the
mid-level predicates (`c2CircletoCircle`, `c2CircletoAABB`, `c2AABBtoAABB`)
directly as well as through `collided`.

**C. Input shapes.** Every parameter is a `float` (or a struct of floats), and the
C validates nothing, so **all 2^32 bit patterns are valid input**. The classes the
hardware/compiler treat differently are: normal finite, ±0, subnormal, ±inf,
quiet NaN (payload-carrying), **signalling** NaN (quieted by SSE, so the output
payload differs from the input), overflow-magnitude (squaring `r` overflows to
inf), underflow-magnitude (squaring underflows to 0/subnormal), and the
invalid-operation pairs `0*inf` and `inf-inf` (which produce the x86
QNaN-indefinite `0xFFC00000` rather than propagating an input payload).

**D. Geometric shapes the predicates distinguish.** overlapping; exactly touching
(`d2 == r2`, where the `<` is strictly false — the boundary case); separated;
one contained in the other; AABB **inverted** (`min > max`, which
`c2Clampv`'s `max(lo, min(a,hi))` handles asymmetrically); degenerate zero-area
AABB (`min == max`); negative radius; zero radius; edge/corner-touching boxes.

## Table (cross-product, pruned to combinations the C distinguishes)

Every row is exercised with **1000+ randomized inputs** from a fixed-seed PRNG
(seed `0x2545F4914F6CDD1D`), not one hand-picked value, plus a hand-written
boundary corpus. Row is checked only after all of its inputs match bit-for-bit.

| # | entry point(s) | configuration (options set + input shape) | test | [x] |
|---|----------------|--------------------------------------------|------|-----|
| 1 | `c2V` | random full-range `u32` bit patterns reinterpreted as `(x, y)` — covers every float class incl. SNaN payloads | `cfg_row01_c2v_random_bits` | [x] |
| 2 | `c2V` | boundary corpus: ±0, ±1, ±inf, QNaN/SNaN with distinct payloads, `f32::MIN_POSITIVE`, subnormals, `MAX` | `cfg_row02_c2v_boundary_corpus` | [x] |
| 3 | `c2Maxv` | random full-range bits ×2 — exercises `a>b ? a : b` incl. the NaN-⇒-take-`b` path | `cfg_row03_c2maxv_random_bits` | [x] |
| 4 | `c2Maxv` | boundary corpus cross-product (both operands drawn from the special-value list, incl. NaN vs NaN, ±0 vs ∓0, inf vs inf) | `cfg_row04_c2maxv_boundary_cross` | [x] |
| 5 | `c2Minv` | random full-range bits ×2 — exercises `a<b ? a : b` | `cfg_row05_c2minv_random_bits` | [x] |
| 6 | `c2Minv` | boundary corpus cross-product | `cfg_row06_c2minv_boundary_cross` | [x] |
| 7 | `c2Clampv` | random `(a, lo, hi)` full-range bits — includes **inverted** ranges (`lo > hi`) since the C never orders them | `cfg_row07_c2clampv_random_bits` | [x] |
| 8 | `c2Clampv` | boundary corpus triple-cross: `a`/`lo`/`hi` from specials; explicit `lo>hi`, `lo==hi`, NaN in each of the three positions | `cfg_row08_c2clampv_boundary_cross` | [x] |
| 9 | `c2Sub` | random full-range bits ×2 — exercises `subss` incl. `inf - inf` ⇒ QNaN-indefinite and SNaN quieting | `cfg_row09_c2sub_random_bits` | [x] |
| 10 | `c2Sub` | boundary corpus cross-product; explicit `inf-inf`, `-inf-(-inf)`, `0-0`, `-0-0`, overflow (`MAX - -MAX`), underflow (nearest subnormals) | `cfg_row10_c2sub_boundary_cross` | [x] |
| 11 | `c2Dot` | random full-range bits ×4 — exercises the pinned `mulss(a.x,b.x)` / `mulss(b.y,a.y)` / `addss(q,p)` operand order that decides which NaN payload survives | `cfg_row11_c2dot_random_bits` | [x] |
| 12 | `c2Dot` | boundary corpus: `0*inf`, `inf*0`, two *different* NaN payloads in the two products (payload-selection test), `inf + -inf` from the two products, overflow to inf, underflow to 0 | `cfg_row12_c2dot_boundary_cross` | [x] |
| 13 | `c2CircletoCircle` | direct call, random full-range bits (all 6 floats) — unconstrained, hits NaN/inf radii and the `A.r + B.r` operand order | `cfg_row13_circle_circle_random_bits` | [x] |
| 14 | `c2CircletoCircle` | direct call, random *plausible geometry* (finite coords in ±100, radii in ±10, so overlap and non-overlap are both frequent) | `cfg_row14_circle_circle_random_geometry` | [x] |
| 15 | `c2CircletoCircle` | direct call, geometric boundaries: exactly touching (`d == rA+rB`, tests strict `<`), concentric, zero radius, **negative** radius (sum can be negative ⇒ `r2` positive after squaring), radii summing to inf | `cfg_row15_circle_circle_boundaries` | [x] |
| 16 | `c2CircletoAABB` | direct call, random full-range bits (all 7 floats) | `cfg_row16_circle_aabb_random_bits` | [x] |
| 17 | `c2CircletoAABB` | direct call, random plausible geometry with **well-ordered** box (`min <= max`) | `cfg_row17_circle_aabb_random_geometry` | [x] |
| 18 | `c2CircletoAABB` | direct call, random plausible geometry with **inverted** box (`min > max`) — different `c2Clampv` path | `cfg_row18_circle_aabb_inverted_box` | [x] |
| 19 | `c2CircletoAABB` | direct call, boundaries: centre inside box, centre on edge, centre on corner, exactly-touching edge/corner, zero-area box, zero and negative radius, NaN in one box component only | `cfg_row19_circle_aabb_boundaries` | [x] |
| 20 | `c2AABBtoAABB` | direct call, random full-range bits (all 8 floats) — exercises the `int` bitwise `d0|d1|d2|d3` and `!` | `cfg_row20_aabb_aabb_random_bits` | [x] |
| 21 | `c2AABBtoAABB` | direct call, random plausible geometry, both boxes well-ordered | `cfg_row21_aabb_aabb_random_geometry` | [x] |
| 22 | `c2AABBtoAABB` | direct call, random plausible geometry, one or both boxes **inverted** | `cfg_row22_aabb_aabb_inverted` | [x] |
| 23 | `c2AABBtoAABB` | direct call, boundaries: edge-touching (`A.max.x == B.min.x`, tests strict `<`), corner-touching, identical, contained, zero-area, separated on each of the 4 axes independently | `cfg_row23_aabb_aabb_boundaries` | [x] |
| 24 | `c2Dot`+`c2Sub`+`c2Clampv` composed | the composed pipeline as a real consumer runs it: `c2Sub`→`c2Dot` and `c2Clampv`→`c2Sub`→`c2Dot` chained by the test itself, with random full-range bits, cross-checked against `c2CircletoCircle`/`c2CircletoAABB` in **both** libraries | `cfg_row24_composed_pipeline` | [x] |
| 25 | `collided` | `typeA=CIRCLE, typeB=CIRCLE` — random full-range bits **and** random plausible geometry; result cross-checked against a direct `c2CircletoCircle` call | `cfg_row25_collided_circle_circle` | [x] |
| 26 | `collided` | `typeA=CIRCLE, typeB=AABB` — random bits + geometry, well-ordered and inverted boxes; cross-checked against direct `c2CircletoAABB(A,B)` | `cfg_row26_collided_circle_aabb` | [x] |
| 27 | `collided` | `typeA=AABB, typeB=CIRCLE` — the **operand-swapping** arm; cross-checked against direct `c2CircletoAABB(*B, *A)` to confirm the swap is reproduced, not "fixed" | `cfg_row27_collided_aabb_circle` | [x] |
| 28 | `collided` | `typeA=AABB, typeB=AABB` — random bits + geometry, well-ordered and inverted | `cfg_row28_collided_aabb_aabb` | [x] |
| 29 | `collided` | aliasing: `A == B` (same pointer) for all 4 valid tag combinations, incl. the 12-byte-circle-read-as-16-byte-AABB overlap the tags allow | `cfg_row29_collided_aliased_pointers` | [x] |
| 30 | `collided` | tag combos read from a **`u8` blob** interpreted as both shapes: a single 16-byte buffer of random bytes passed with all 4 tag pairs, i.e. arbitrary struct contents rather than constructed values | `cfg_row30_collided_raw_blob` | [x] |

## Feature combinations

`translation/Cargo.toml` declares no `[features]`, so `default`,
`--no-default-features`, and `--all-features` are the same build. All 30 rows are
re-run under each of those invocations by `scripts/verify_all.sh` to prove it.
