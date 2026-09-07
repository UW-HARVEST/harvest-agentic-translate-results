# ERRORS.md — error / rejection surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no error enum
and no `RETURN_ERROR` macro**: every rejection is either

* an early `return;` that leaves `m->count == 0`,
* a fall-through `switch` with no `default:` (silently does nothing),
* a sentinel numeric result (`0`, `(0,0)`, `FLT_MAX`, `inf`, `NaN`),
* or a deliberate out-of-contract read (`index == ~0`, `count <= 0`).

Every row below is a distinct rejection branch in the C source, with the
source line(s) it comes from.

| # | function | trigger (exact invalid input / condition) | expected C result |
|---|----------|-------------------------------------------|-------------------|
| 1 | `c2MakeProxy` (L125) | `type == C2_TYPE_POLY` (3) — switch has no such case | `*p` left **completely untouched** (caller's uninitialised proxy) |
| 2 | `c2MakeProxy` (L125) | `type` out of range (e.g. `-1`, `4`, `99`, `INT_MIN`) — no `default:` | `*p` left untouched |
| 3 | `ptr_from_parts` (L901) | `typ == C2_TYPE_POLY` or any other value — no `default:`, no trailing `return` | falls off end → indeterminate pointer (UB). Never dereferenced by `c2Collide` for those types |
| 4 | `c2Collide` (L855) | `typeA == C2_TYPE_POLY` (3) | outer switch no case → `m->count = 0`, rest of `*m` untouched |
| 5 | `c2Collide` (L855) | `typeA` out of range (`-1`, `4`, `INT_MIN`, `INT_MAX`) | same as #4 |
| 6 | `c2Collide` (L857/870/884) | valid `typeA`, `typeB == C2_TYPE_POLY` | inner switch no case → `m->count = 0`, `m->n` untouched (NOT negated) |
| 7 | `c2Collide` (L857/870/884) | valid `typeA`, `typeB` out of range | same as #6 |
| 8 | `c2AABBtoAABBManifold` (L664) | `dx = eA.x+eB.x-|d.x| < 0` (separated on x) | early `return`, `m->count == 0`, `depths`/`contact_points`/`n` untouched |
| 9 | `c2AABBtoAABBManifold` (L667) | `dx >= 0` but `dy < 0` (separated on y) | early `return`, `m->count == 0` |
| 10 | `c2CircletoCircleManifold` (L587) | `d2 >= r*r` (no overlap) | `m->count == 0`, nothing else written |
| 11 | `c2CircletoAABBManifold` (L603) | `d2 >= r2` (circle outside AABB by ≥ r) | `m->count == 0` |
| 12 | `c2CircletoCapsuleManifold` (L643) | GJK `d >= r` | `m->count == 0` |
| 13 | `c2CapsuletoCapsuleManifold` (L840) | GJK `d >= r` | `m->count == 0` |
| 14 | `c2CapsuletoPolyManifold` (L734/807) | `d >= 1e-6f` **and** `d >= A.r` | `m->count == 0` |
| 15 | `c2CapsuletoPolyManifold` code 0 (L781) | `c2SidePlanesFromPoly` returns 0 | early `return` with `m->count == 0` (`c2KeepDeep` never runs) |
| 16 | `c2CapsuletoPolyManifold` code 1 (L790) | `c2SidePlanes` returns 0 | early `return`, `m->count == 0` |
| 17 | `c2CapsuletoPolyManifold` code 2 (L798) | `c2SidePlanes` returns 0 | early `return`, `m->count == 0` |
| 18 | `c2CapsuletoPolyManifold` `default:` (L802) | `code` not in {0,1,2} — unreachable in practice | `return`, `m->count == 0` |
| 19 | `c2SidePlanes` (L251) | first `c2Clip(seg,left) < 2` | returns 0; `seg` overwritten with partially-uninitialised `out[]` |
| 20 | `c2SidePlanes` (L253) | second `c2Clip(seg,right) < 2` | returns 0 |
| 21 | `c2SidePlanes` (L255) | `h == NULL` | skips writing `*h`, still returns 1 |
| 22 | `c2Clip` (L214-225) | both `d0 >= 0` and `d1 >= 0` and `d0*d1 > 0` | `sp == 0`; `seg[0]`/`seg[1]` set from **uninitialised** `out[]` |
| 23 | `c2Clip` | `d0 == 0 && d1 == 0` | `sp` becomes 2 or 4 (the two `<0` tests are false, then 2 pushes) → ≥2, accepted |
| 24 | `c2Incident` (L714) | `ip->count <= 0` → loop never runs | `index` stays `~0 == -1` → reads `ip->verts[-1]` (out-of-bounds) |
| 25 | `c2Incident` (L718) | every `c2Dot(...)` is `NaN` (`dot < min_dot` always false) | `index` stays `-1` → `ip->verts[-1]` OOB read. Reached via `c2AABBtoCapsuleManifold` with a degenerate AABB (zero-area ⇒ NaN normals) |
| 26 | `c2Support` (L369) | `count <= 0` | reads `verts[0]` unconditionally, loop skipped, returns `0` |
| 27 | `c2Norm` / `c2Div` (L228-234) | `c2Len(a) == 0` (zero vector) | `1.0f/0 = +inf`, `0*inf = NaN` → returns `(NaN, NaN)` |
| 28 | `c2Norm` (L232) | `a` contains `NaN`/`inf` | propagates `NaN` |
| 29 | `c2Len` (L164) | `c2Dot(a,a)` overflows to `+inf` | `sqrtf(inf) = inf` |
| 30 | `c2Len` (L164) | `c2Dot(a,a)` is `NaN` (inf-inf) | `sqrtf(NaN) = NaN` |
| 31 | `c2Intersect` (L206) | `da == db` | `da/(da-db)` = `±inf` or `NaN` (0/0) → `NaN`/`inf` components |
| 32 | `c2GJKSimplexMetric` (L173) | `s->count` not in {2,3} (0, 1, 4, negative) | `default:`/`case 1:` → `0` |
| 33 | `c2D` (L354) | `s->count` not in {1,2} (0, 3, 4, negative) | `case 3:`/`default:` → `(0,0)` |
| 34 | `c2Witness` (L384) | `s->count` not in {1,2,3} (0, 4, negative) | `default:` → `*a = *b = (0,0)` |
| 35 | `c2Witness` (L383) | `s->div == 0` | `den = 1/0 = +inf` → `inf`/`NaN` components for count 2/3 |
| 36 | `c2L` (L411) | `s->count` not in {1,2} | `default:` → `(0,0)` |
| 37 | `c2L` (L410) | `s->div == 0` | `den = +inf` → `inf`/`NaN` |
| 38 | `c2GJK` (L427) | `ax_ptr == NULL` | substitutes `c2xIdentity()` |
| 39 | `c2GJK` (L431) | `bx_ptr == NULL` | substitutes `c2xIdentity()` |
| 40 | `c2GJK` (L569) | `outA == NULL` | skips write, still returns `dist` |
| 41 | `c2GJK` (L571) | `outB == NULL` | skips write |
| 42 | `c2GJK` (L573) | `iterations == NULL` | skips write |
| 43 | `c2GJK` (L442) | `cache == NULL` | no cache read and no cache write |
| 44 | `c2GJK` (L443) | `cache != NULL` with `cache->count == 0` | `cache_was_good == 0` → cold start; cache still written on exit |
| 45 | `c2GJK` (L464) | cache present, `!(min_metric < max_metric*2 && metric < -1e8f)` | `cache_was_read = 1` → simplex kept from cache (note: `metric < -1e8f` makes the conjunction almost always false, so the cache is almost always "read") |
| 46 | `c2GJK` (L484) | 20 iterations elapse without convergence | loop exits on `iter == 20`, `hit == 0`, returns current witness distance |
| 47 | `c2GJK` (L506) | `d1 > d0` (no progress) | `break` out of loop |
| 48 | `c2GJK` (L510) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` | `break` |
| 49 | `c2GJK` (L530) | duplicate support point (`iA`,`iB`) already in simplex | `break` |
| 50 | `c2GJK` (L544) | `use_radius != 0` and `dist <= rA+rB` (or `dist <= FLT_EPSILON`) | midpoint collapse: `a = b = (a+b)/2`, `dist = 0` |
| 51 | `c2GJK` (L538) | `hit != 0` (3-simplex containing origin) | `a = b`, `dist = 0` |
| 52 | `c2GJK` (L550) | after radius shrink `a == b` exactly | `dist = 0` |
| 53 | `c2GJK` typeA/typeB `POLY` | proxy left uninitialised by `c2MakeProxy` | `pA`/`pB` are indeterminate stack contents (UB); the only in-library caller is `c2CapsuletoPolyManifold` |
| 54 | `c2PlaneAt` (L91) | `i` out of `[0,8)` | out-of-bounds read of `p->norms[i]` / `p->verts[i]` |
| 55 | `c2BBVerts` (L118) | `bb->min > bb->max` (inverted AABB) | no validation; emits an inverted (clockwise) quad |
| 56 | `c2Norms` (L815) | `count <= 0` | writes nothing |
| 57 | `c2Norms` (L820) | duplicate consecutive verts (zero-length edge) | `c2Norm` of `(0,0)` → `(NaN,NaN)` normal |
| 58 | `c2Clampv` (L73) | `lo > hi` | no validation; result is `max(lo, min(a,hi))` = `lo` |
| 59 | `c2Maxv`/`c2Minv` (L63-71) | operand contains `NaN` | ternary `a.x > b.x` is false for NaN ⇒ always returns `b`'s component |
| 60 | `omni_manifold` (L926) | `type_a`/`type_b == C2_TYPE_POLY` or out of range | `ptr_from_parts` returns indeterminate, `c2Collide` ignores it → `m->count = 0` |
| 61 | `c2Div` (L228) | `b == 0` | `1.0f/0 = inf`; `0*inf = NaN` |
| 62 | `c2Dot` (L83) | `inf * 0` operand pair | `NaN` |
| 63 | `c2CircletoCircleManifold` (L589) | `l == 0` (coincident centers) | `n = (0, 1)` fallback instead of normalising |
| 64 | `c2CircletoAABBManifold` (L604) | `d2 == 0` (center inside AABB) | deep-penetration branch using x/y overlap |
| 65 | `c2CircletoCapsuleManifold` (L645) | `d == 0` | `n = c2Norm(c2Skew(B.b - B.a))`; if `B.a == B.b` this is `(NaN,NaN)` |
| 66 | `c2CapsuletoCapsuleManifold` (L842) | `d == 0` | `n = c2Norm(c2Skew(A.b - A.a))`; `(NaN,NaN)` if `A.a == A.b` |
| 67 | `c2CapsuletoPolyManifold` (L739) | `A_in_B.a == A_in_B.b` (degenerate capsule) | `ab = c2Norm((0,0)) = (NaN,NaN)` → all planes NaN → `code` stays 0, `index` stays `-1` ⇒ `c2SidePlanesFromPoly(..., -1, ...)` OOB read |
| 68 | `c2AABBtoCapsuleManifold` (L824) | degenerate AABB (`min == max`, or zero width/height) | `c2Norms` yields NaN normals; `c2Incident`/`index = -1` OOB path (row 24/25) |
| 69 | `c2AABBtoCapsuleManifold` (L831) | no collision (`m->count == 0`) | `m->n = c2Neg(m->n)` is still applied to the **untouched** `m->n` — caller-visible sign flip of stale data |
| 70 | `c2Collide` AABB/CIRCLE (L873) | no collision | `m->n = c2Neg(m->n)` applied to untouched `m->n` (same stale-data flip) |

## Results — every row has a passing differential test

Test file: `tests/phase_c_errors.rs` (19 tests, all passing). Mapping:

| rows | test |
|------|------|
| 1, 2 | `row01_row02_makeproxy_unhandled_type` |
| 3 | `row03_ptr_from_parts_no_case` |
| 4, 5, 6, 7 | `row04_row07_collide_bad_types` |
| 8, 9 | `row08_row09_aabb_aabb_separated` |
| 10, 11, 12, 13, 14 | `row10_row14_no_overlap` |
| 15, 16, 17, 18, 19, 20, 21, 22, 23 | `row15_row23_clip_sideplane_rejections` |
| 24, 25, 67, 68 | `row24_row25_row67_row68_negative_index` |
| 26 | `row26_support_nonpositive_count` |
| 27, 28, 29, 30, 31, 61, 62 | `row27_row31_arithmetic_sentinels` |
| 32, 33, 34, 35, 36, 37 | `row32_row37_simplex_out_of_contract` |
| 38, 39, 40, 41, 42, 43, 44, 45 | `row38_row52_gjk_null_and_limits` |
| 46, 47, 48, 49, 50, 51, 52 | `row46_row52_gjk_termination_and_radius` |
| 53 | `row53_poly_proxy_uninitialised` |
| 54 | `row54_planeat_out_of_range` |
| 55, 56, 57, 58, 59 | `row55_row59_unvalidated_helpers` |
| 60 | `row60_omni_bad_types` |
| 63, 64, 65, 66 | `row63_row66_degenerate_fallbacks` |
| 69, 70 | `row69_row70_stale_normal_negation` |
| generic FFI boundaries (null pointers, zero/oversized/negative lengths, out-of-range enum ints one step past every valid variant) | `generic_boundaries` + the per-row tests above |

All 70 rows are checked off.

## Two rows need special handling — read this before trusting the numbers

### Row 3 — `ptr_from_parts` has no `return` on the fall-through path

For `C2_TYPE_POLY` and every out-of-range tag, the C function reaches `}`
without executing a `return`. The value left in `%rax` is indeterminate *by
definition*, so there is nothing to compare. The test therefore asserts what is
actually observable and what `c2Collide` relies on:

* for the three handled tags, both libraries `malloc` and populate identical
  payloads;
* for the fall-through tags neither library crashes, and the Rust translation
  returns an explicit `NULL` sentinel (the pointer is never dereferenced by
  `c2Collide`, whose `switch` has no case for those tags either).

### Row 53 — `c2GJK` reads an uninitialised `c2Proxy` for `C2_TYPE_POLY`

`c2MakeProxy` has no `C2_TYPE_POLY` case, so `c2GJK`'s `c2Proxy pB` local is
never written when `typeB == C2_TYPE_POLY`. `c2GJK` then reads
`pB.verts[0]`, `pB.count` and `pB.radius`. This is a genuine
uninitialised-memory read in the C source, and it is on the path of three
*public* entry points:

```
omni_manifold / c2Collide  (AABB vs CAPSULE, either order)
  -> c2AABBtoCapsuleManifold
       -> c2CapsuletoPolyManifold
            -> c2GJK(..., C2_TYPE_POLY, ...)
```

Consequences that were **measured** on the built C `.so`:

* Calling `c2AABBtoCapsuleManifold` twice with byte-identical arguments returned
  two different manifolds (`count == 2` then `count == 0`), i.e. the C library is
  not a function of its inputs here.
* With hostile stack residue, `pB.count` becomes a large positive integer and
  `c2Support` walks off the top of the stack — the C library **SIGSEGVs** on
  ordinary, in-contract public input.

The Rust translation models the local with `std::mem::zeroed()`, which is the
behaviour of the C library whenever that stack region happens to be zero (a
freshly-grown stack). To make the comparison well-defined and reproducible, the
harness pins that memory:

1. `with_clean_stack()` zeroes 16 KiB of stack immediately below the frame that
   makes the FFI call, using **volatile** writes (`black_box` on a raw pointer
   does not clobber memory, so a plain `write_bytes` version was silently
   optimised away — verified).
2. `fresh()` runs each test on a newly spawned thread with a unique, increasing
   stack size, so glibc cannot hand back a cached (dirty) thread stack, and
   scrubs 1 MiB once at thread entry.
3. Both `.so`s are `dlopen`ed with **`RTLD_NOW`**. This is the subtle one: under
   the default `RTLD_LAZY`, the first call to each of the C library's own PLT
   entries runs `_dl_runtime_resolve_xsavec`, which spills the entire vector
   register file several hundred bytes down the stack — landing exactly on
   `pB`. That made the *first* AABB-vs-capsule call of a process behave
   differently from every later one, and was the sole remaining source of
   irreproducibility.
4. Failure messages are built lazily (`check(..., || format!(...))`); eagerly
   formatting a context string on every iteration left ~100 bytes of `core::fmt`
   debris in the same region.

With all four in place the C library is deterministic and matches the Rust
translation bit-for-bit on this path, over the tens of thousands of randomised
inputs in `CONFIGS.md` rows 44–57 and `ERRORS.md` rows 15–25, 53, 67–68.

**Caveat for downstream users (not a translation defect):** an application that
calls the *C* library's AABB-vs-capsule path from a dirty stack will get
different answers from the Rust one, because the C answer is not defined. The
Rust translation is the deterministic, zero-proxy interpretation.
