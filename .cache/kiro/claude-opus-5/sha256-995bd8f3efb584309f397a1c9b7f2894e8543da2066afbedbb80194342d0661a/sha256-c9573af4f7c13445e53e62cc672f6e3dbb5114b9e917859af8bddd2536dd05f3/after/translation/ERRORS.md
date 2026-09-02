# ERRORS.md — error / rejection surface table (Phase A, gates Phase C)

This library has **no error codes, no `errno`, no `assert`, no `RETURN_ERROR`
macro and no `return -1`**. Verified mechanically:

```sh
$ grep -n 'assert\|RETURN_ERROR\|errno\|return -1\|return NULL' c_src/src/lib.c
(no output)
```

Its entire "rejection surface" is therefore built out of:

* early `return` statements that leave an output struct in a partially-written
  state (`m->count == 0`, or worse: `m->count` and `m->n` written but
  `contact_points`/`depths` untouched),
* `switch`/`if` fall-through arms that silently do nothing (unhandled enum
  values), including one that **falls off the end of a non-`void` function**
  and returns an indeterminate value,
* `0`-return sentinels from the private clipping helpers,
* division by zero producing `inf` / `NaN` rather than rejecting,
* unchecked array indexing / null dereference (undefined behaviour that the
  Rust must reproduce for the *observable*, non-trapping cases).

Every row below is a distinct rejection branch in the C, with the C source
line, the exact triggering input, and the observable C result the Rust must
reproduce bit-for-bit.

| # | function | trigger (exact invalid input/condition) | expected C result |
|---|----------|------------------------------------------|-------------------|
| 1 | `c2Clip` (`lib.c:206`, private) | both segment endpoints strictly outside the plane (`d0 > 0 && d1 > 0`) | returns `sp == 0`; `seg[0..1]` overwritten with the **uninitialized** `out[]`; caller (`c2SidePlanes`) sees `< 2` and rejects. Never observable through a public symbol. |
| 2 | `c2Clip` (`lib.c:206`) | exactly one endpoint outside and the other exactly on the plane (`d0 > 0 && d1 == 0`) | returns `sp == 1` (only the intersection is emitted) → `c2SidePlanes` rejects with `0`. |
| 3 | `c2SidePlanes` (`lib.c:252`) | segment entirely outside the **left** side plane | returns `0`, `*h` NOT written |
| 4 | `c2SidePlanes` (`lib.c:254`) | segment entirely outside the **right** side plane | returns `0`, `*h` NOT written |
| 5 | `c2SidePlanes` (`lib.c:246`) | `ra == rb` (degenerate reference edge) → `c2Norm` divides by `0` | `in = (NaN, NaN)`; both `c2Clip` comparisons false; returns `0` |
| 6 | `c2SidePlanes` (`lib.c:258`) | `h == NULL` | side-plane output skipped, still returns `1` |
| 7 | `c2AABBtoAABBManifold` (`lib.c:665`) | `dx < 0` — boxes separated on X | early `return` with `m->count == 0`; `m->n`, `m->depths`, `m->contact_points` left **untouched** (caller's prior bytes preserved) |
| 8 | `c2AABBtoAABBManifold` (`lib.c:668`) | `dy < 0` — boxes separated on Y | early `return` with `m->count == 0`; rest of `*m` untouched |
| 9 | `c2CapsuletoPolyManifold` (`lib.c:782`) | `code == 0` and `c2SidePlanesFromPoly` rejects | early `return`; `m->count == 0`, `m->n`/depths/points untouched, and the `depths[i] += A.r` loop is skipped |
| 10 | `c2CapsuletoPolyManifold` (`lib.c:791`) | `code == 1` and `c2SidePlanes` rejects | early `return`, same partial state as #9 |
| 11 | `c2CapsuletoPolyManifold` (`lib.c:799`) | `code == 2` and `c2SidePlanes` rejects | early `return`, same partial state as #9 |
| 12 | `c2CapsuletoPolyManifold` (`lib.c:803`) | `switch (code) default:` — unreachable given `code ∈ {0,1,2}` | early `return`, `m->count == 0` |
| 13 | `c2CapsuletoPolyManifold` (`lib.c:730`) | `d >= 1e-6 && d >= A.r` — shapes too far apart | neither branch taken: `m->count == 0`, nothing else written |
| 14 | `c2CapsuletoPolyManifold` (`lib.c:740`) | `A.a == A.b` (degenerate capsule) → `c2Norm(0)` | `ab = (NaN,NaN)` ⇒ `s0`, `s1` are `NaN`; `NaN > sep` is false so `code` stays `0` and `index` comes from the plane loop; result is a well-defined non-NaN manifold |
| 15 | `c2CapsuletoPolyManifold` (`lib.c:751`) | `B->count == 0` | plane loop never runs ⇒ `index` stays `~0 == -1`, `sep` stays `-FLT_MAX`; `c2Support` still reads `verts[0]` and returns `0`; if `s0`/`s1` don't win, `code == 0` with `index == -1` ⇒ **negative array index** into `p->verts` |
| 16 | `c2CapsuletoPolyManifold` (`lib.c:751`) | `B->count < 0` | identical to #15 (loop condition `i < count` immediately false) |
| 17 | `c2CapsuletoPolyManifold` (`lib.c:759/764`) | `B->count > 8` | `c2Support` / `c2PlaneAt` read **past** `verts[8]`/`norms[8]` into the adjacent struct bytes |
| 18 | `c2CapsuletoPolyManifold` (`lib.c:733`) | `bx_ptr == NULL` | `bx = c2xIdentity()` (accepted, not an error) |
| 19 | `c2Incident` (`lib.c:697`, private) | `ip->count <= 0`, or every `dot` is `NaN` | `index` stays `~0 == -1` ⇒ reads `verts[-1]` |
| 20 | `c2CircletoCircleManifold` (`lib.c:583`) | `d2 >= r*r` — circles disjoint | `m->count == 0`, `m->n`/depths/points untouched |
| 21 | `c2CircletoCircleManifold` (`lib.c:589`) | coincident centres (`l == 0`) and `r > 0` | takes the `l != 0 ? ... : c2V(0,1)` fallback: `n = (0,1)` (no NaN) |
| 22 | `c2CircletoCircleManifold` | `A.r + B.r == 0` (both radii 0) | `d2 < 0` is false ⇒ `m->count == 0` |
| 23 | `c2CircletoCircleManifold` | negative radius, `A.r + B.r < 0` | `r*r > 0` so a manifold may still be produced with a **negative** `depths[0]` |
| 24 | `c2CircletoAABBManifold` (`lib.c:600`) | `d2 >= A.r*A.r` — no overlap | `m->count == 0`, rest untouched |
| 25 | `c2CircletoAABBManifold` (`lib.c:604`) | centre strictly inside the box (`d2 == 0`) | deep-penetration branch: axis of least overlap, `depths[0] = A.r + depth` |
| 26 | `c2CircletoAABBManifold` (`lib.c:604`) | `d2 == 0` and `x_overlap == y_overlap` | `x_overlap < y_overlap` false ⇒ **Y** axis chosen |
| 27 | `c2CircletoAABBManifold` (`lib.c:601`) | inverted AABB (`min > max`) | `c2Clampv` = `max(lo, min(a, hi))` yields `lo`; still produces a manifold (possibly negative depth) — must match, not be "fixed" |
| 28 | `c2CircletoAABBManifold` | `A.r < 0` | `r2 = A.r*A.r > 0`, so overlap can be reported with negative depth |
| 29 | `c2CircletoCapsuleManifold` (`lib.c:643`) | `d >= A.r + B.r` | `m->count == 0`, rest untouched |
| 30 | `c2CircletoCapsuleManifold` (`lib.c:645`) | `d == 0` (touching/overlapping) **and** `B.a == B.b` | `c2Norm(c2Skew(0))` = `(NaN, NaN)` ⇒ `m->n` and `contact_points[0]` are `NaN`; `depths[0] = r` |
| 31 | `c2CapsuletoCapsuleManifold` (`lib.c:842`) | `d == 0` **and** `A.a == A.b` | `n = (NaN, NaN)`; NaN propagates into `m->n` / `contact_points[0]` |
| 32 | `c2CapsuletoCapsuleManifold` (`lib.c:840`) | `d >= A.r + B.r` | `m->count == 0`, rest untouched |
| 33 | `c2AABBtoCapsuleManifold` (`lib.c:830`) | any input where the inner `c2CapsuletoPolyManifold` early-returns (#9–#13) | `m->count == 0` **but `m->n` is still negated unconditionally** — `m->n = c2Neg(m->n)` runs after the call, so a caller's pre-existing `m->n` gets sign-flipped |
| 34 | `c2AABBtoCapsuleManifold` (`lib.c:826`) | degenerate AABB (`min == max`) | `c2Norms` calls `c2Norm(0)` ⇒ all four `p.norms` are `(NaN, NaN)` |
| 35 | `c2Norms` (`lib.c:816`) | `count <= 0` | loop body never runs, `norms` untouched |
| 36 | `c2Norms` (`lib.c:820`) | two consecutive identical verts | that norm becomes `(NaN, NaN)` (division by zero length) |
| 37 | `c2MakeProxy` (`lib.c:126`) | `type == C2_TYPE_POLY` (**3**) — no `case` for it | `switch` falls through: `*p` left **completely unwritten** |
| 38 | `c2MakeProxy` (`lib.c:126`) | `type` out of range (e.g. `-1`, `4`, `99`, `INT_MAX`) — C enums accept any `int` | same as #37: `*p` untouched, no crash |
| 39 | `c2GJK` (`lib.c:427/431`) | `ax_ptr == NULL` and/or `bx_ptr == NULL` | substitutes `c2xIdentity()` (accepted) |
| 40 | `c2GJK` (`lib.c:475`) | `outA == NULL`, `outB == NULL`, `iterations == NULL`, `cache == NULL` | each write is skipped; the distance is still returned |
| 41 | `c2GJK` (`lib.c:437`) | `typeA`/`typeB == C2_TYPE_POLY` or out of range | proxy is never filled ⇒ reads an **indeterminate** `c2Proxy` (see NOTE below) |
| 42 | `c2GJK` (`lib.c:443`) | `cache->count != 0` with `cache->iA[i]`/`iB[i]` out of `[0,8)` | unchecked `pA.verts[iA]` read past the 8-element array. Also: a forged `cache->count >= 4` makes the loop write `saveA[3]`, past the end of `int saveA[3]` (`lib.c:478`), clobbering the C's own loop state — the C **segfaults**. Out of contract: the library never writes a cache with `count > 3`. Tested for `count ∈ [0,3]` and in-range indices; `count >= 4` is documented, not asserted. |
| 43 | `c2GJK` (`lib.c:464`) | cache metric check `!(min < max*2 && metric < -1e8)` | `cache_was_read = 1` — the *rejection* of the cache is the common case; the simplex is reused as-is |
| 44 | `c2GJK` (`lib.c:485`) | shapes so configured that the loop never terminates early | hard cap `iter < 20`; `*iterations` is at most `20` |
| 45 | `c2GJK` (`lib.c:513`) | `d1 > d0` (no progress) | `break`s out; result taken from the current simplex |
| 46 | `c2GJK` (`lib.c:519`) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (search direction collapsed) | `break` |
| 47 | `c2GJK` (`lib.c:539`) | duplicate support point (`iA`/`iB` already in `saveA`/`saveB`) | `break` **before** `++s.count`, so the vertex just written at `verts[s.count]` is discarded |
| 48 | `c2GJK` (`lib.c:556`) | `hit` (simplex reached count 3, i.e. overlap) | `a = b`, `dist = 0` |
| 49 | `c2GJK` (`lib.c:562`) | `use_radius != 0` and `dist <= rA + rB` **or** `dist <= FLT_EPSILON` | midpoint fallback: `a = b = (a+b)/2`, `dist = 0` |
| 50 | `c2GJK` (`lib.c:566`) | `use_radius != 0`, radii shrink `a`/`b` onto the same point | `dist` forced to `0` |
| 51 | `c2Witness` (`lib.c:365`) | `s->count` not in `{1,2,3}` (`0` or `> 3`) | `default:` ⇒ `*a = *b = (0,0)` |
| 52 | `c2Witness` (`lib.c:342`) | `s->div == 0` | `den = 1/0 = inf` ⇒ `inf`/`NaN` propagates into `*a`/`*b` |
| 53 | `c2L` (`lib.c:417`) | `s->count` not `1` or `2` (incl. `3`) | `default:` ⇒ returns `(0,0)` |
| 54 | `c2L` (`lib.c:412`) | `s->div == 0` with `count == 2` | `den = inf` ⇒ `inf`/`NaN` components |
| 55 | `c2D` (`lib.c:403`) | `s->count == 3` or any other value | `case 3: default:` ⇒ returns `(0,0)` |
| 56 | `c2GJKSimplexMetric` (`lib.c:174`) | `s->count` not `2` or `3` (incl. `0`, `1`, `4`, negative) | `default: case 1:` ⇒ returns `0` |
| 57 | `c2Support` (`lib.c:427`) | `count <= 0` | still dereferences `verts[0]` unconditionally, returns `0` |
| 58 | `c2Support` | all dots equal, or all `NaN` | `dot > dmax` never true ⇒ returns `0` (first index) |
| 59 | `c2Collide` (`lib.c:855`) | `typeA == C2_TYPE_POLY` (3) or out of range | outer `switch` falls through: `m->count == 0`, nothing else written, `A`/`B` never dereferenced |
| 60 | `c2Collide` (`lib.c:857/870/884`) | valid `typeA` but `typeB == C2_TYPE_POLY` / out of range | inner `switch` falls through: `m->count == 0`, and (for the AABB/CAPSULE rows) the post-call `m->n = c2Neg(m->n)` is **not** reached since it lives inside the taken `case` |
| 61 | `ptr_from_parts` (`lib.c:906`) | `typ == C2_TYPE_POLY` or out of range | **falls off the end of a non-`void` function** — no `return`. Indeterminate return value (in practice whatever is in `rax`). Rust returns `NULL`. Only reachable via `omni_manifold`, where `c2Collide` then never dereferences it (#59/#60), so the value is unobservable there. |
| 62 | `ptr_from_parts` (`lib.c:908`) | `malloc` returns `NULL` (OOM) | unchecked ⇒ null-pointer write. Not testable without an allocator fault injector. |
| 63 | `omni_manifold` (`lib.c:923`) | `type_a` and/or `type_b` == `C2_TYPE_POLY` (3) or out of range (`-1`, `4`, `INT_MAX`, `INT_MIN`) | `m->count = 0`; the rest of `*m` is left exactly as the caller had it |
| 64 | `omni_manifold` | any of `a1..a5`, `b1..b5` is `NaN` / `±inf` | no check whatsoever; `NaN`/`inf` propagate through every comparison (all `<` comparisons with `NaN` are false), producing a defined-but-odd manifold |
| 65 | `c2PlaneAt` (`lib.c:91`) | `i < 0` or `i >= 8` | unchecked index into `norms`/`verts` |
| 66 | `c2Div` / `c2Norm` (`lib.c:238/242`) | `b == 0` / zero-length vector | `1/0 = inf`, `0*inf = NaN` ⇒ `(NaN, NaN)` (or `±inf` for a non-zero component). No rejection. |
| 67 | `c2Intersect` (`lib.c:200`) | `da == db` | `da/(da-db)` = `±inf` or `NaN` ⇒ non-finite point. No rejection. |
| 68 | `c2Clip` (`lib.c:206`) | `d0 < 0 && d1 < 0` **and** `d0 * d1` UNDERFLOWS to `+0` (e.g. both ≈ `-1e-24`) | both endpoints are pushed *and* the `d0 * d1 <= 0` arm fires, so `out[sp++]` writes `out[2]` — one past the end of the C's `c2v out[2]`. Returns `sp == 3`. Only `out[0]`, `out[1]` and `sp >= 2` are ever observed. |
| 69 | `c2Incident` (`lib.c:697`) reached from `c2AABBtoCapsuleManifold` (`lib.c:826`) | degenerate AABB (`min == max`) ⇒ all `p.norms[i]` are `NaN` ⇒ `index` stays `~0 == -1` | evaluates `ip->verts[-1]`, reading the 8 bytes below `p.verts`. In gcc's frame (`c2Poly p` at `rbp-0xa0`, the by-value `c2AABB A` at `rbp-0xb0`) those are **`A.max.y`** followed by **`p.count`** (`== 4`, i.e. `5.6e-45` read as a float). Confirmed by disassembly and by differential test. |

## How the two indeterminate-memory rows are handled

Rows **#37 / #41** (uninitialized `c2Proxy` on the `C2_TYPE_POLY` path) and row
**#69** (`verts[-1]`) are the only places where "match the C" has no
input-determined answer: the C reads memory it never wrote.

**Row #69 is fully replicated.** `objdump -d` on `c2AABBtoCapsuleManifold` pins
the frame layout, so `src/lib.rs` places the poly inside a `#[repr(C)]`
`AabbCapsulePolyFrame { before_verts: f32, poly: c2Poly }` and seeds
`before_verts` with `A.max.y`. `verts[-1]` then reads `(A.max.y, count)` in both
libraries. This turned a systematic mismatch into an exact match — see
`tests/phase_c_errors.rs::rows13_19_capsule_poly_edge_counts`.

Note that when a caller passes its *own* `c2Poly` to
`c2CapsuletoPolyManifold` directly, `verts[-1]` reads the caller's buffer, so
both libraries read the same address and agree without any special handling.
`tests/phase_c_errors.rs` wraps its polys in a `PaddedPoly` to make that
explicit.

**Rows #37 / #41 are pinned by controlling the stack.** `c2GJK`'s
`c2Proxy pA, pB;` are never written for a poly, and the bytes they read come
from whatever the *caller* left on the stack — verified experimentally in
`tests/phase_c_indeterminate_stack.rs::stack_dependence_of_the_c_library`, where
the C returns different manifolds for identical inputs depending only on call
depth. The Rust zero-initializes its proxies; the differential tests therefore
zero-fill 4 KiB of stack (`common::scrub_stack`) immediately before each FFI
call, which pins the C's indeterminate proxy to all-zeros and makes the two
libraries agree bit-for-bit. Measured effect on a 300 000-case randomized
`omni_manifold` sweep over all 16 type pairs: **5 743 mismatches without
scrubbing, 0 with it.**

Rows **#62** (`malloc` returning `NULL`) and **#61** (`ptr_from_parts` falling
off the end of a non-`void` function) are not asserted: the first needs an
allocator fault injector, and the second has no defined C value to compare
against. The Rust returns `NULL`; `c2Collide` has no poly arm, so the value is
never dereferenced and never observable through `omni_manifold`.

## Row → test mapping

Every row above is exercised by `translation/tests/phase_c_errors.rs` unless
noted. All listed tests pass against both `.so`s.

| rows | test |
|------|------|
| 1–6, 9–12, 68 | `rows01_12_clip_and_sideplane_rejections` |
| 7, 8 | `rows07_08_aabb_separated` |
| 13–19, 69 | `rows13_19_capsule_poly_edge_counts` |
| 20–23 | `rows20_23_circle_circle_rejections` |
| 24–28 | `rows24_28_circle_aabb_rejections` |
| 29–34 | `rows29_34_capsule_rejections` |
| 35, 36 | `rows35_36_norms_edge_cases` |
| 37, 38 | `rows37_38_make_proxy_unhandled_types` |
| 39, 40, 44–50 | `rows39_50_gjk_error_and_boundary_paths` |
| 41, 42, 43 | `rows41_42_43_gjk_bad_types_and_cache` |
| 51–56 | `rows51_56_simplex_out_of_range_state` |
| 57, 58 | `rows57_58_support_degenerate` |
| 59, 60, 63 | `rows59_60_63_dispatch_unhandled_types` |
| 64 | `row64_omni_nonfinite_inputs` |
| 65 | `row65_planeat_out_of_range_index` |
| 66, 67 | `rows66_67_division_degeneracies` |
| 37, 41 (indeterminacy) | `phase_c_indeterminate_stack.rs` (all tests) |
| 61, 62 | documented above; not asserted |

Additionally `phase_c_errors.rs::phase_c_harness_detects_divergence` proves the
comparison helper actually fails on a real difference, so a green Phase C cannot
be an artefact of a no-op assertion.
