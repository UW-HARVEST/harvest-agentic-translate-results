# ERRORS.md — Phase C error / rejection surface table

Derived mechanically from `c_src/src/lib.c`. The library has **no error enum,
no `assert`, no `errno`, no `RETURN_ERROR` macro, and never returns `NULL` or
`-1`.** Every rejection is expressed as one of:

* a `switch` `default:` / unmatched label that yields a sentinel (`0`,
  `c2V(0,0)`, or "leave the out-parameter untouched"), or
* a null-pointer test that substitutes a default / skips a write, or
* a numeric guard (`<= 0`, `< FLT_EPSILON`, `> d0`, `iter < 20`) that aborts a
  loop or picks a degenerate branch.

Grep evidence: `grep -n 'return 0\|default:\|if (!' lib.c` → lines
162, 164, 293, 332, 354, 368, 372, 405, 409, 529, 535, 586-587, 598-599,
610-611, 614-615.

Tests live in `tests/phase_c.rs`; every row below is checked off only after its
test passed against BOTH `.so`s. Three rows are marked **UB-EXCLUDED**: the C's
behaviour there depends on reading uninitialised stack memory or on an
out-of-bounds access, so there is no defined C result to be byte-identical to.
Each exclusion states the evidence for it, and the surrounding defined subset is
still tested.

| # | function | trigger (the exact invalid input/condition) | expected C result | test | [x] |
|---|----------|----------------------------------------------|-------------------|------|-----|
| 1 | `c2MakeProxy` | `type` not in {0,1,2} (`3`, `4`, `-1`, `-2`, `99`, `255`, `256`, `1000`, `±0x10000`, `INT_MAX`, `INT_MIN`) — the `switch` has no `default:` label (lib.c:106-127) | `*p` left completely **untouched**: no write to `radius`, `count` or any of the 8 `verts`. The `shape` pointer is never dereferenced, so `NULL` is a valid shape here. | `err01_makeproxy_bad_type_leaves_proxy_untouched` | [x] |
| 2 | `c2Collided` | `typeA` not in {0,1,2} — outer `switch default:` (lib.c:614-615) | returns `0`; neither pointer dereferenced (NULL is fine) | `err02_collided_bad_typeA` | [x] |
| 3 | `c2Collided` | `typeA == C2_TYPE_CIRCLE` and `typeB` not in {0,1,2} — inner `default:` (lib.c:586-587) | returns `0`; `B` not dereferenced | `err03_collided_circle_bad_typeB` | [x] |
| 4 | `c2Collided` | `typeA == C2_TYPE_AABB` and `typeB` not in {0,1,2} — inner `default:` (lib.c:598-599) | returns `0`; `B` not dereferenced | `err04_collided_aabb_bad_typeB` | [x] |
| 5 | `c2Collided` | `typeA == C2_TYPE_CAPSULE` and `typeB` not in {0,1,2} — inner `default:` (lib.c:610-611) | returns `0`; `B` not dereferenced | `err05_collided_capsule_bad_typeB` | [x] |
| 6 | `c2GJKSimplexMetric` | `s->count` is `0`, negative, or `> 3` — `default:` falls through into `case 1:` (lib.c:162-164) | returns `0.0f`; `*s` unmodified | `err06_simplex_metric_bad_count` | [x] |
| 7 | `c2D` | `s->count == 3`, or `0` / negative / `> 3` — `case 3:`/`default:` (lib.c:292-294) | returns `c2V(0,0)` | `err07_c2D_bad_count` | [x] |
| 8 | `c2Witness` | `s->count` not in {1,2,3} (`0`, negative, `>3`) — `default:` (lib.c:331-334) | writes `c2V(0,0)` to **both** `*a` and `*b` (pre-loaded sentinels prove the writes happen) | `err08_witness_bad_count` | [x] |
| 9 | `c2L` | `s->count` not in {1,2} (i.e. `3`, `0`, negative, `>3`) — `default:` (lib.c:353-355) | returns `c2V(0,0)` | `err09_c2L_bad_count` | [x] |
| 10 | `c2Support` | `count <= 0` (`0`, `-1`, `-2`, `-100`, `INT_MIN`) — `verts[0]` is read **before** the loop guard (lib.c:299-301) | returns `0`; `verts[0]` read regardless. Also covered: oversized counts (`9`, `16`, `32`) against a caller-owned 32-slot array. | `err10_support_zero_negative_and_oversized_count` | [x] |
| 11 | `c2GJK` | `ax_ptr == NULL` (lib.c:368-369) | substitutes `c2xIdentity()`; asserted equal to passing an explicit identity `c2x` | `err11_12_gjk_null_transforms_use_identity` | [x] |
| 12 | `c2GJK` | `bx_ptr == NULL` (lib.c:372-373) | substitutes `c2xIdentity()`; ditto | `err11_12_gjk_null_transforms_use_identity` | [x] |
| 13 | `c2GJK` | `outA == NULL` (lib.c:534) | write skipped — the caller's sentinel survives; distance still returned | `err13_14_15_16_gjk_null_outparams_and_cache` | [x] |
| 14 | `c2GJK` | `outB == NULL` (lib.c:536) | write skipped; distance still returned | `err13_14_15_16_gjk_null_outparams_and_cache` | [x] |
| 15 | `c2GJK` | `iterations == NULL` (lib.c:538) | write skipped | `err13_14_15_16_gjk_null_outparams_and_cache` | [x] |
| 16 | `c2GJK` | `cache == NULL` (lib.c:381, 522) | no cache read, no cache write; result identical to a `count == 0` cache | `err13_14_15_16_gjk_null_outparams_and_cache` | [x] |
| 17 | `c2GJK` | `cache->count == 0` with garbage `metric` / `iA` / `iB` / `div` (`cache_was_good` false, lib.c:382-383) | cache ignored, simplex re-seeded from vert 0; the cache is then **overwritten** with the fresh result. Asserted bit-equal to the NULL-cache call. | `err17_gjk_cache_count_zero_is_ignored` | [x] |
| 18 | `c2GJK` | `cache->count != 0` and the metric guard `!(min < max*2 && metric < -1.0e8f)` — the `metric < -1.0e8f` conjunct is essentially never true, so `cache_was_read` is set even for a wildly wrong cached metric (lib.c:405-406) | the cached simplex is trusted verbatim, **no re-seed**. Probed with `metric ∈ {0, ±0, ±1, -1e8, -1.0000001e8, -1e9, -1e30, ±inf, NaN, ±FLT_MAX}` × `count ∈ {1,2,3}` × `div ∈ {1, 0, -1, 2.5}`. Reproduced verbatim, not "fixed". | `err18_gjk_cache_metric_guard_is_always_taken` | [x] |
| 19 | `c2GJK` | `typeA` / `typeB` out of range → `c2MakeProxy` writes nothing, so `pA.count` / `pA.verts` are the **uninitialised** bytes of `c2Proxy pA;` (lib.c:376-379) | **UB-EXCLUDED.** `c2Proxy pA;` is an uninitialised stack local, so `pA.count` is leftover frame data and `c2Support` then loops over it — no defined value exists to match. What IS asserted: the Rust export survives every such call (it zero-initialises the proxy, so it cannot wild-read where the C might). | `err19_gjk_bad_type_documented_ub_no_crash` | [x] |
| 20 | `c2GJK` | `cache->count` in 1..3 with cached `iA`/`iB` **≥ the new proxy's `count`** — the indices are never validated (lib.c:385-397) | **UB-EXCLUDED** for out-of-count indices: `pA.verts[iA]` then reads an uninitialised slot of `c2Proxy pA;`, which is real UB with no reproducible value (observed: the C returns leftover witness points from its previous frame where a zeroed proxy returns `(0,0)`). The DEFINED subset — every cached index `< count` for the current proxy — is asserted bit-equal, as is cross-type reuse where the indices stay in range (`row42`, `row42b` in Phase B). | `err20_gjk_cache_indices_within_proxy_count`, `row42*` | [x] |
| 21 | `c2GJK` | `cache->count >= 4` — the read loop indexes `cache->iA[i]` past the end of `int iA[3]` (lib.c:384) | **UB-EXCLUDED, verified to crash the C.** `iA[3]` aliases `iB[0]` and `iB[3]` aliases the **float** `div` reinterpreted as an `int` (`1.0f` → `1065353216`), which is then used as `pB.verts[1065353216]`. A plain C caller compiled against the C `.so` dies with `SIGSEGV` (confirmed directly). Negative counts, by contrast, are fully defined — the loop is skipped, `s.count` goes negative, and every `switch` takes its `default:` — and ARE asserted (`-1`, `-2`, `-100`, `INT_MIN`). | `err21_gjk_cache_count_four_and_negative` | [x] |
| 22 | `c2GJK` | iteration cap `iter < 20` (lib.c:425) | loop exits with the `iter` reached; the `iterations` out-param is compared bit-for-bit on every GJK row | `err22_to_28_gjk_loop_exits_and_radius_collapse`, `row43` | [x] |
| 23 | `c2GJK` | degenerate search direction: `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` (lib.c:451-453) | `break` out of the GJK loop early | `err22_to_28_...` (coincident-point cases) | [x] |
| 24 | `c2GJK` | no progress: `d1 > d0` (lib.c:445-446) | `break` out of the GJK loop early | `err22_to_28_...` (1e30-scale cases) | [x] |
| 25 | `c2GJK` | duplicate support point (`iA==saveA[i] && iB==saveB[i]`) (lib.c:466-472) | `break`; `s.count` is **not** incremented | `err22_to_28_...`, `row43` | [x] |
| 26 | `c2GJK` | `use_radius != 0` and `dist <= rA+rB` **or** `dist <= FLT_EPSILON` (lib.c:485-497) | `a = b = midpoint(a,b)`, `dist = 0.0f`. Probed with `dist == rA+rB` exactly and with separations of `0`, `1e-30`, `1e-8`, `FLT_EPSILON`, `2·FLT_EPSILON`. | `err22_to_28_...` | [x] |
| 27 | `c2GJK` | `use_radius != 0` and the radius shrink makes the witness points coincide (`a.x==b.x && a.y==b.y`) (lib.c:492-493) | `dist` forced to `0.0f` | `err22_to_28_...`, `row29`/`row44` | [x] |
| 28 | `c2GJK` | `hit != 0` (simplex reached count 3, origin enclosed) (lib.c:481-483) | `a = b`, `dist = 0.0f` | `err22_to_28_...` (identical shapes) | [x] |
| 29 | `c2Div` / `c2Norm` | `b == 0.0f` / `c2Len(a) == 0.0f` — no zero check (lib.c:337-343) | `divss` of `1.0f` by `±0.0f` gives `±inf`; multiplying then yields `±inf` or QNaN-indefinite (`0·inf`). No trap, no `errno`. | `err29_30_31_div_norm_len_degenerate` | [x] |
| 30 | `c2Norm` | `a` contains NaN or `Inf` (incl. signalling NaNs and negative NaNs) | NaN/Inf propagates; the exact surviving payload and sign are byte-compared | `err29_30_31_...`, `fuzz_leaf_vector_ops` | [x] |
| 31 | `c2Len` | `a` with `Inf` (⇒ `Inf`) or NaN (⇒ quieted NaN). `c2Dot(a,a)` is a sum of squares so `sqrtf` never sees a negative. | `sqrtf` propagates without `errno`/`assert` | `err29_30_31_...` | [x] |
| 32 | `c2CircletoCapsule` | degenerate capsule `B.a == B.b` ⇒ `n == (0,0)`, so `da == 0` (the `da < 0` branch is NOT taken) and `db == 0` (the `db < 0` branch is NOT taken) (lib.c:559-570) | falls through to the endpoint-`b` branch; **no division by zero**, because `c2Dot(n,n)` is only evaluated inside the `db < 0` branch | `err32_33_circle_to_capsule_degenerate` | [x] |
| 33 | `c2CircletoCapsule` | negative radii (`A.r + B.r < 0`); no validation anywhere | `r*r` ≥ 0, so a negative total radius behaves like its magnitude; `d2 < r*r` still evaluated | `err32_33_...` | [x] |
| 34 | `c2CircletoCircle` | negative / zero / NaN / ±Inf / ±FLT_MAX radii (full 10×10 cross product) | `r2 = (A.r+B.r)²`; `d2 < r2` (NaN radius ⇒ comparison false ⇒ `0`) | `err34_circle_to_circle_negative_radius` | [x] |
| 35 | `c2CircletoAABB` | inverted AABB (`min > max`), degenerate (`min == max`), NaN corners — `c2Clampv` is `c2Maxv(lo, c2Minv(a,hi))` with no ordering check | clamps to `lo` when inverted; result is still `d2 < r2`. `c2Clampv` itself is compared alongside. | `err35_circle_to_aabb_inverted_box` | [x] |
| 36 | `c2AABBtoAABB` | inverted AABB, or NaN coordinates in either box | all four `<` comparisons are false for NaN ⇒ `!(0) == 1` ⇒ reports **collision** | `err36_aabb_to_aabb_nan_and_inverted` | [x] |
| 37 | `c2AABBtoCapsule` / `c2CapsuletoCapsule` | any input where the GJK distance is not exactly `±0.0f`. The test is a raw `float != 0`, so `-0.0f` counts as a collision (IEEE `-0.0 == 0.0`) while a **NaN** distance is `!= 0` and reports **no collision** (lib.c:528-536) | `0` when the distance is non-zero or NaN, `1` when exactly `±0.0f` | `err37_gjk_backed_bools_nan_distance` | [x] |
| 38 | `capsule` | any float args at all — NaN, signalling NaN, ±Inf, denormals, ±FLT_MAX; there is no validation | a bitmask in `{0..7}`; the args flow straight into the GJK/collision math | `err38_capsule_entry_point_unvalidated`, `row53`, `fuzz_public_capsule_entry_point` | [x] |

## Generic FFI boundaries (not rows in the C, checked anyway)

`generic_boundary_sweep` additionally covers, for every entry point that takes
one, the value **one step past** each documented range in both directions:

* `C2_TYPE` = `-1` and `3` on `c2Collided` and `c2MakeProxy`;
* simplex `count` = `0` and `4` on `c22`, `c23`, `c2D`, `c2L`, `c2Witness`,
  `c2GJKSimplexMetric`;
* `c2Support` `count` = `0`, `1`, `8`, `9` (the last against a 16-slot buffer);
* all-zero / zero-length geometry through all six boolean entry points;
* `NULL` for every pointer the C tolerates (`ax_ptr`, `bx_ptr`, `outA`, `outB`,
  `iterations`, `cache`, and the `shape`/`A`/`B` pointers on the code paths
  where the C provably does not dereference them).

`NULL` is **not** passed as `A`/`B` on paths where the C dereferences it
immediately (`c2Collided` with a valid type pair, `c2GJK` with a valid type):
that is an unconditional null dereference in the C, and both libraries would
simply fault.
