# ERRORS.md — Phase A error / rejection surface table

This library has **no** `RETURN_ERROR` macro, no error enum, no `assert`, no
`errno`, no `return -1`, and no `return NULL`. Its entire rejection surface
consists of: `switch` `default:` labels, missing `switch` labels (fall-out),
null-pointer guards, and the loop / range constants. Every such site was found
mechanically:

```sh
grep -n "assert\|RETURN_ERROR\|return -1\|return NULL\|return 0;\|errno\|ERROR" c_src/src/lib.c
grep -n "default:" c_src/src/lib.c
grep -n "if (!\|if (out\|if (cache\|if (iterations" c_src/src/lib.c
grep -n "20\|\[8\]\|\[3\]\|e+38\|e-7\|1.0e8" c_src/src/lib.c
```

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2Collided` (lib.c:614 `default:`) | `typeA` not in {0,1,2} (e.g. `-1`, `3`, `999`, `INT_MIN`, `INT_MAX`) | returns `0`, no shape deref | [x] |
| 2 | `c2Collided` (lib.c:586 `default:`) | `typeA == C2_TYPE_CIRCLE(0)` and `typeB` not in {0,1,2} | returns `0` | [x] |
| 3 | `c2Collided` (lib.c:598 `default:`) | `typeA == C2_TYPE_AABB(1)` and `typeB` not in {0,1,2} | returns `0` | [x] |
| 4 | `c2Collided` (lib.c:610 `default:`) | `typeA == C2_TYPE_CAPSULE(2)` and `typeB` not in {0,1,2} | returns `0` | [x] |
| 5 | `c2MakeProxy` (lib.c:112 switch, **no** `default:` label) | `type` not in {0,1,2} | nothing written: `*p` keeps the caller's prior bytes (`radius`/`count`/`verts` unchanged) | [x] |
| 6 | `c2GJKSimplexMetric` (lib.c:162 `default:` falls into `case 1:`) | `s->count` not 2 and not 3 (0, 1, 4, negative, huge) | returns `0.0f` | [x] |
| 7 | `c2D` (lib.c:293 `case 3: default:`) | `s->count` not 1 and not 2 | returns `c2V(0,0)` | [x] |
| 8 | `c2Witness` (lib.c:332 `default:`) | `s->count` not in {1,2,3} (0, 4, negative) | writes `c2V(0,0)` to both `*a` and `*b` | [x] |
| 9 | `c2L` (lib.c:354 `default:`) | `s->count` not 1 and not 2 | returns `c2V(0,0)` | [x] |
| 10 | `c2Witness` / `c2L` | `s->div == 0` → `den = 1.0f/0.0f = +inf`; `count>=2` | products become `inf`/`NaN` per component sign; propagated verbatim | [x] |
| 11 | `c2Witness` / `c2L` | `s->div == -0.0f` → `den = -inf` | `-inf`/`NaN` propagated verbatim | [x] |
| 12 | `c2Support` (lib.c:300) | `count <= 0` (0 or negative) | loop body never runs, but `verts[0]` is **still dereferenced** for `dmax`; returns `0` | [x] |
| 13 | `c2Div` (lib.c:339) | `b == 0.0f` | `c2Mulvs(a, inf)` → `±inf` or `NaN` (for `0*inf`) per component | [x] |
| 14 | `c2Norm` (lib.c:343) | `a == (0,0)` → `c2Len==0` → divide by zero | `(NaN, NaN)` (`0*inf`) | [x] |
| 15 | `c2GJK` (lib.c:368 `if (!ax_ptr)`) | `ax_ptr == NULL` | substitutes `c2xIdentity()` instead of faulting | [x] |
| 16 | `c2GJK` (lib.c:372 `if (!bx_ptr)`) | `bx_ptr == NULL` | substitutes `c2xIdentity()` | [x] |
| 17 | `c2GJK` (lib.c:510 `if (outA)`) | `outA == NULL` | witness point A silently discarded, no write | [x] |
| 18 | `c2GJK` (lib.c:512 `if (outB)`) | `outB == NULL` | witness point B silently discarded, no write | [x] |
| 19 | `c2GJK` (lib.c:514 `if (iterations)`) | `iterations == NULL` | iteration count silently discarded, no write | [x] |
| 20 | `c2GJK` (lib.c:383 `if (cache)`) | `cache == NULL` | cache read **and** write-back both skipped; cold start | [x] |
| 21 | `c2GJK` (lib.c:384) | `cache != NULL` but `cache->count == 0` | `cache_was_good` false → cold start; cache still written back on exit | [x] |
| 22 | `c2GJK` (lib.c:405) | warm cache whose recomputed `metric` fails `!(min<max*2 && metric < -1.0e8f)` | `cache_was_read = 1`: the cached simplex is trusted verbatim, including a stale `div` | [x] |
| 23 | `c2GJK` (lib.c:425 `while (iter < 20)`) | geometry that never terminates early | hard cap: loop exits after at most 20 iterations, `*iterations <= 20`. `c22`/`c23` can REDUCE `s.count`, so the loop can append-then-collapse repeatedly; `iter` is **not** bounded by 2. Measured max over 1.5M randomized calls (incl. NaN/inf/subnormal geometry): **6**. See note below. | [x] |
| 24 | `c2GJK` (lib.c:451) | search direction with `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (`1.4210855e-14f`) | loop breaks before adding a vertex | [x] |
| 25 | `c2GJK` (lib.c:441 `if (d1 > d0)`) | non-decreasing distance (incl. `d1 == NaN`, where `NaN > d0` is false so it does **not** break) | breaks only on strict `>` | [x] |
| 26 | `c2GJK` (lib.c:486) | `use_radius != 0` and `dist <= rA+rB` **or** `dist <= FLT_EPSILON` | midpoint collapse: `a = b = (a+b)*0.5f`, `dist = 0` | [x] |
| 27 | `c2GJK` (lib.c:490) | `use_radius != 0`, shrink applied, and shrunk `a == b` exactly | `dist` forced to `0` even though it was `> 0` | [x] |
| 28 | `c2GJK` | `hit != 0` (simplex reached count 3) | `a = b`, `dist = 0`, radius branch skipped entirely | [x] |
| 29 | `c2GJK` | `use_radius == 0` | radius never subtracted; raw core distance returned even for fat shapes | [x] |
| 30 | `c2GJK` warm cache | `cache->count` in {1,2,3} with `iA[i]`/`iB[i]` **inside** the proxy's valid vertex range | simplex rebuilt from those indices, `u` set to 0 | [x] |
| 31 | `c2AABBtoCapsule` (lib.c:529) | `c2GJK(...) != 0` treated as truthy C float→bool (incl. `NaN`, which is truthy) | returns `0`; only exact `0.0f`/`-0.0f` returns `1` | [x] |
| 32 | `c2CapsuletoCapsule` (lib.c:535) | same truthiness rule | returns `0` unless distance is exactly zero | [x] |
| 33 | `c2CircletoCircle` | negative radii, so `A.r+B.r < 0`; `r2 = (A.r+B.r)^2 > 0` | sign is squared away — negative radii behave like positive ones | [x] |
| 34 | `c2CircletoAABB` | inverted AABB (`min > max`) → `c2Clampv` = `c2Maxv(lo, c2Minv(a,hi))` yields `lo` | no rejection; returns whatever the clamp math gives | [x] |
| 35 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` → `n = (0,0)`, `da = 0` (not `< 0`), `db = 0` (not `< 0`) | takes the **`bp`** branch (`d2 = |A.p-B.b|^2`); the `da/c2Dot(n,n)` division is never reached | [x] |
| 36 | `c2CircletoCapsule` | `da >= 0 && db < 0` with `c2Dot(n,n) == 0` — unreachable given #35, asserted unreachable | n/a (documented; `da<0` or `bp` branch always taken) | [x] |
| 37 | `c2AABBtoAABB` | inverted / NaN-bearing AABBs: all four `<` comparisons are false for NaN | `d0|d1|d2|d3 == 0` → returns `1` (reports collision) | [x] |
| 38 | any `c2v`-taking function | `NaN` / `±inf` component inputs | propagated through arithmetic; comparisons `>`/`<` are false for NaN | [x] |
| 39 | `reverse_collide` | `r <= 0`, `NaN`, `±inf`, huge `x`/`y` | no validation; returns the 3-bit mask from the three `c2Collided` calls | [x] |
| 40 | `reverse_collide` | `-0.0f` inputs | identical to `+0.0f` in all comparisons used | [x] |

## Deliberately excluded (undefined behaviour, not deterministically comparable)

* `c2GJK` with an out-of-range `C2_TYPE`. `c2MakeProxy`'s `switch` has no
  `default:` label, so nothing is written to `c2Proxy pA` — an **uninitialised
  local** of `c2GJK`. Every later read of `pA.count` / `pA.radius` / `pA.verts`
  is then stack residue, and `c2Support(pA.verts, pA.count, ...)` may read far
  out of bounds. Verified empirically in `tests/phase_d_layout.rs`: the identical
  call `c2GJK(shape, /*typeA=*/3, ..., circle, ...)` returns `0.0` when preceded
  by a small-coordinate call and `12727922000.0` when preceded by a
  large-coordinate one. The C's answer is not a function of its arguments, so
  there is nothing for the Rust to match. The *deterministic* out-of-range-enum
  paths — `c2Collided` (rows 1–4) and `c2MakeProxy` itself (row 5) — ARE covered
  bit-exactly.
* `c2GJK` with `cache->count > 3`: reads `cache->iA[3]`/`iB[3]` past the array.
* `c2GJK` with a warm cache whose `iA`/`iB` index **beyond** the proxy's
  `count` (e.g. index 5 for a circle proxy with `count == 1`): the C reads
  uninitialised stack in `pA.verts[5]`; the Rust reads a zeroed `c2Proxy`.
  Not a translation defect — the C result is not a function of its inputs.
* `c2MakeProxy` / `c2BBVerts` / `c2Witness` / `c2Support` with a null pointer
  argument: the C has no guard and segfaults. Row #5 covers the *reachable*
  invalid-`type` case with a valid `p`.
* NaN **sign/payload** selection when two NaN operands meet in a commutative
  `+` or `*`. IEEE 754 §6.2 leaves this unspecified; gcc's choice is per-
  expression register allocation (`c2Add`'s `addss` destination holds `b`,
  `c2Dot`'s `mulss` destination holds `a`) and LLVM canonicalizes commutative
  operand order, so it cannot be steered from Rust source. Comparison is
  bit-exact everywhere else, including NaN-vs-number (a hard failure). See the
  `f32_bits_eq` doc comment in `tests/common/mod.rs`.

## Note on the iteration cap and `verts[count-1].u`

Exiting the `c2GJK` loop via `iter == 20` is the only path that leaves
`verts[s.count-1].u` unwritten while `s.count >= 2`: every other exit runs
`c22`/`c23` at the top of the iteration (which writes every live `u`), and the
`dup` break exits *without* the preceding `++s.count`, so the half-written vertex
is never counted. If the cap fired, the C would read uninitialised stack there
while the Rust reads a zeroed field. `tests/hunt_cap.rs` searches 1.5M randomized
calls and tops out at `iter == 6` (histogram: 993034 at 0, 289138 at 1, 176806 at
2, 38903 at 3, 1987 at 4, 129 at 5, 3 at 6). The cap is not reachable in
practice. This is a measurement, not a proof of unreachability.
