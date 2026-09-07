# CONFIGS.md — Phase B configuration-surface table

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`.

## Axes the C code actually branches on

There is no global/persistent state, no init function, no option struct and no
`#ifdef` in the library — the only "options" are the *arguments*. The axes are
therefore:

1. **Entry point** (10 exported functions; the header exposes only `collided`,
   but the other nine have external linkage and are real entry points that must
   be driven directly — see `SYMBOLS.md`).
2. **`C2_TYPE` tag pair** for `collided` — the 2×2 valid cross-product
   (`CIRCLE×CIRCLE`, `CIRCLE×AABB`, `AABB×CIRCLE`, `AABB×AABB`), of which
   `AABB×CIRCLE` is special because the C **swaps the arguments**
   (`c2CircletoAABB(*(c2Circle*)B, *(c2AABB*)A)`, lib.c:88).
3. **Float value class** per component — the C has no explicit branches on
   value, but the hardware does: normal, `±0.0`, subnormal, `±inf`, qNaN, sNaN,
   and huge/tiny magnitudes that overflow/underflow the squaring in `d2`/`r2`.
4. **Comparison outcome** — every collision routine ends in a strict `<`, so
   each needs the *below / exactly-equal / above* boundary triple (touching
   shapes, tangent circles, centre exactly on an AABB face/corner).
5. **Geometric relationship** — clamp position (inside / on each of the 4 faces /
   at each of the 4 corners / outside on each side), separation axis
   (x-only, y-only, both, none) for `c2AABBtoAABB`.
6. **Degenerate shape** — zero radius, negative radius, zero-extent AABB
   (`min == max`), inverted AABB (`min > max`).
7. **Pointer shape** for `collided` — aligned vs. unaligned buffer, and the
   distinct struct sizes (12-byte `c2Circle` vs 16-byte `c2AABB`).

Every row below is exercised through **both** `.so`s via `libloading` with many
randomized inputs (fixed seed, deterministic xorshift PRNG) and asserted equal
bit-for-bit (`f32::to_bits` for float returns, exact `int` for predicates).

## Table

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `c2V` | random normal `(x, y)` pairs, full-range | [x] |
| 2 | `c2V` | special values: `±0.0`, `±inf`, subnormals, qNaN, sNaN, random raw `u32` bit patterns (must be copied verbatim, never quieted) | [x] |
| 3 | `c2Minv` | both operands random normals, all four orderings of `x` and of `y` | [x] |
| 4 | `c2Minv` | operands sharing a component (`a.x == b.x`) — equality path returns `b` | [x] |
| 5 | `c2Minv` | signed-zero combinations (`+0/-0`, `-0/+0`, `+0/+0`, `-0/-0`) — sign bit of result | [x] |
| 6 | `c2Minv` | NaN in any subset of the 4 components (all 15 non-empty subsets) — non-suppressing ternary | [x] |
| 7 | `c2Minv` | `±inf` in any component, mixed with normals | [x] |
| 8 | `c2Maxv` | both operands random normals, all four orderings | [x] |
| 9 | `c2Maxv` | equality path (`a.x == b.x`) returns `b` | [x] |
| 10 | `c2Maxv` | signed-zero combinations | [x] |
| 11 | `c2Maxv` | NaN in any of the 15 non-empty component subsets | [x] |
| 12 | `c2Maxv` | `±inf` mixed with normals | [x] |
| 13 | `c2Clampv` | `a` strictly inside `[lo, hi]`; well-ordered random interval | [x] |
| 14 | `c2Clampv` | `a` below `lo` / above `hi` / exactly `== lo` / exactly `== hi`, per component independently (3×3 grid) | [x] |
| 15 | `c2Clampv` | degenerate interval `lo == hi` | [x] |
| 16 | `c2Clampv` | inverted interval `lo > hi` (no validation in C) | [x] |
| 17 | `c2Clampv` | NaN / `±inf` in `a`, `lo`, `hi` independently and combined | [x] |
| 18 | `c2Clampv` | signed zeros as interval endpoints | [x] |
| 19 | `c2Dot` | random normal vectors, magnitudes ~1 | [x] |
| 20 | `c2Dot` | random vectors spanning the whole exponent range (`2^-60 … 2^60`) — exercises rounding of the `mul`+`add` chain | [x] |
| 21 | `c2Dot` | products that **overflow** to `±inf`; products that **underflow** to subnormal/`±0` | [x] |
| 22 | `c2Dot` | cancellation: `a.x*b.x == -(a.y*b.y)` exactly ⇒ result `±0.0`, sign checked | [x] |
| 23 | `c2Dot` | `±inf` operands incl. `0 * inf` and `inf + (-inf)` invalid ops ⇒ default qNaN | [x] |
| 24 | `c2Dot` | qNaN and sNaN in any of the 15 non-empty subsets of the 4 components — payload propagation and quieting | [x] |
| 25 | `c2Dot` | fully random raw `u32` bit patterns in all 4 components (100k iterations, covers every class jointly) | [x] |
| 26 | `c2Sub` | random normals; `±0.0` results and sign of zero (`x - x`, `(-0) - (+0)`) | [x] |
| 27 | `c2Sub` | `inf - inf` per component ⇒ qNaN; `inf - finite`; overflow to `±inf` | [x] |
| 28 | `c2Sub` | NaN in any of the 15 non-empty subsets; sNaN quieting | [x] |
| 29 | `c2CircletoCircle` | random separated circles (no overlap) | [x] |
| 30 | `c2CircletoCircle` | random overlapping circles | [x] |
| 31 | `c2CircletoCircle` | **exactly tangent** (`d² == (rA+rB)²` bit-exactly, via Pythagorean triples) — strict-`<` boundary | [x] |
| 32 | `c2CircletoCircle` | identical centres (`d2 == 0`), with `rA+rB > 0` and with `rA == rB == 0` | [x] |
| 33 | `c2CircletoCircle` | zero radius on one / both circles | [x] |
| 34 | `c2CircletoCircle` | negative radius on one / both (sign squared away) | [x] |
| 35 | `c2CircletoCircle` | huge radii/centres so `r2` or `d2` overflows to `+inf` | [x] |
| 36 | `c2CircletoCircle` | `±inf` radius; `+inf + -inf` radii ⇒ `r2 = NaN` | [x] |
| 37 | `c2CircletoCircle` | NaN in centre / in radius (independently) | [x] |
| 38 | `c2CircletoCircle` | fully random raw `u32` bits in all 6 floats (100k iterations) | [x] |
| 39 | `c2CircletoAABB` | circle centre **inside** the box, `r > 0` (⇒ `d2 = 0 < r2`) | [x] |
| 40 | `c2CircletoAABB` | centre outside past each of the 4 faces (clamp lands on a face) | [x] |
| 41 | `c2CircletoAABB` | centre outside past each of the 4 corners (clamp lands on a corner) | [x] |
| 42 | `c2CircletoAABB` | centre **exactly on** a face / a corner (`d2 == 0`) | [x] |
| 43 | `c2CircletoAABB` | **exactly tangent**: distance from clamp point `== r` bit-exactly | [x] |
| 44 | `c2CircletoAABB` | zero-extent box (`min == max`, a point) | [x] |
| 45 | `c2CircletoAABB` | inverted box (`min > max` on x, on y, on both) | [x] |
| 46 | `c2CircletoAABB` | `r == 0`; `r < 0`; `r = ±inf`; `r = NaN` | [x] |
| 47 | `c2CircletoAABB` | NaN / `±inf` in `A.p`, `B.min`, `B.max` independently | [x] |
| 48 | `c2CircletoAABB` | fully random raw `u32` bits in all 7 floats (100k iterations) | [x] |
| 49 | `c2AABBtoAABB` | random overlapping boxes | [x] |
| 50 | `c2AABBtoAABB` | separated on x only / y only / both (each of the 4 `dN` branches driven alone) | [x] |
| 51 | `c2AABBtoAABB` | **exactly touching** on each of the 4 edges (`A.max.x == B.min.x`, …) — strict `<` | [x] |
| 52 | `c2AABBtoAABB` | one box fully containing the other; identical boxes | [x] |
| 53 | `c2AABBtoAABB` | zero-extent (point) boxes; point-vs-box; point-vs-point | [x] |
| 54 | `c2AABBtoAABB` | inverted boxes on either / both operands | [x] |
| 55 | `c2AABBtoAABB` | NaN in any of the 8 coordinates (all 255 non-empty subsets) ⇒ must return `1` | [x] |
| 56 | `c2AABBtoAABB` | `±inf` coordinates, incl. the infinite box `(-inf,-inf)..(inf,inf)` | [x] |
| 57 | `c2AABBtoAABB` | fully random raw `u32` bits in all 8 floats (100k iterations) | [x] |
| 58 | `collided` | `typeA=CIRCLE, typeB=CIRCLE`, random circles — must agree with `c2CircletoCircle` | [x] |
| 59 | `collided` | `typeA=CIRCLE, typeB=AABB`, random shapes | [x] |
| 60 | `collided` | `typeA=AABB, typeB=CIRCLE` — the **argument-swapping** arm (lib.c:88) | [x] |
| 61 | `collided` | `typeA=AABB, typeB=AABB`, random boxes | [x] |
| 62 | `collided` | all 4 valid tag pairs with **fully random raw `u32`** payload bytes (100k iterations) | [x] |
| 63 | `collided` | all 4 valid tag pairs with **unaligned** buffers (offsets 1, 2, 3, 5, 7 bytes) | [x] |
| 64 | `collided` | `A` and `B` aliasing the **same** buffer (self-collision), all 4 tag pairs | [x] |
| 65 | `collided` | tag/pointee size mismatch: `AABB` tag over a 12-byte circle buffer and vice versa (byte reinterpretation) | [x] |
| 66 | *(cross-check)* | `collided(CIRCLE,CIRCLE)` == `c2CircletoCircle`, `collided(AABB,CIRCLE)` == `c2CircletoAABB(B,A)`, etc. — composition of the dispatch pipeline, verified within each `.so` and across them | [x] |

## Binary executable

`c_src/CMakeLists.txt` declares only `add_library(... SHARED src/lib.c)` — there
is no `add_executable`, and `translation/Cargo.toml` declares only
`[lib] crate-type = ["cdylib"]` with no `[[bin]]` and no `src/main.rs`.
**No driver binary exists in either tree, so the stdout-comparison item is not
applicable.**

## Feature combinations

`translation/Cargo.toml` has no `[features]` table ⇒ the single (default/empty)
combination is the only one. Both `cargo test --release` and
`cargo test --release --no-default-features` were run over the full suite.
