# ERRORS.md — error / rejection surface table (Phase C gate)

This library has **no error enum, no `RETURN_ERROR` macro, no `assert`, no
`errno` use, and no negative sentinel**. Derived mechanically:

```sh
grep -n "return 0\|return NULL\|return -1\|default:\|assert" c_src/src/lib.c
grep -n "if (!\|if (out\|if (iterations)\|if (cache)" c_src/src/lib.c
```

Every rejection is one of:

* a `switch` with **no matching case** — falls out of the `switch` leaving the
  out-parameter untouched, or hits an explicit `default: return 0`;
* a **predicate returning 0** (`return d2 < r2` etc. → the "no collision"
  answer, which is this API's only failure signal);
* a **null-pointer guard** that substitutes a default instead of erroring;
* a **loop / convergence bail-out** (`break`) that terminates the algorithm;
* in one case, **undefined behaviour** (`ptr_from_parts` falls off the end of a
  non-void function).

One row per distinct rejection branch in the C source. Line numbers refer to
`c_src/src/lib.c`.

| # | function | trigger (the exact invalid input/condition) | expected C result | status |
|---|----------|----------------------------------------------|-------------------|--------|
| 1 | `c2MakeProxy` (L107) | `type` matches none of the 3 cases (e.g. `3`, `-1`, `INT_MAX`) — switch has **no `default:`** | Returns without writing `*p`; `p->radius/count/verts` keep their prior (in C: indeterminate) contents. **No crash, no error code.** | [x] |
| 2 | `c2GJKSimplexMetric` (L157-159) | `s->count` is not 2 and not 3 (`0`, `1`, `4`, negative, huge) — `default:` falls through to `case 1:` | returns `0.0f` | [x] |
| 3 | `c2D` (L288) | `s->count == 3` **or** any other value (`case 3: default:` share a body) | returns `c2V(0,0)` | [x] |
| 4 | `c2Witness` (L327) | `s->count` not in {1,2,3} (`0`, `4`, negative) — `default:` | writes `*a = c2V(0,0)`, `*b = c2V(0,0)` | [x] |
| 5 | `c2L` (L349) | `s->count` not in {1,2} — `default:` | returns `c2V(0,0)` | [x] |
| 6 | `c2Support` (L299-307) | `count <= 0` — `verts[0]` is read **before** the loop guard, so `count==0` still dereferences index 0 and the loop body never runs | returns `0` | [x] |
| 7 | `c2Support` | all dots equal / `d` is the zero vector — strict `>` never fires | returns `0` (first index wins ties) | [x] |
| 8 | `c2Support` | `d` contains NaN → every `dot > dmax` comparison is false | returns `0` | [x] |
| 9 | `c2GJK` (L363) | `ax_ptr == NULL` | substitutes `c2xIdentity()`; **not an error** | [x] |
| 10 | `c2GJK` (L367) | `bx_ptr == NULL` | substitutes `c2xIdentity()`; **not an error** | [x] |
| 11 | `c2GJK` (L505) | `outA == NULL` | skips the store; return value still valid | [x] |
| 12 | `c2GJK` (L507) | `outB == NULL` | skips the store | [x] |
| 13 | `c2GJK` (L509) | `iterations == NULL` | skips the store | [x] |
| 14 | `c2GJK` (L378/495) | `cache == NULL` | skips both the warm-start read **and** the write-back | [x] |
| 15 | `c2GJK` (L379) | `cache != NULL` but `cache->count == 0` (`cache_was_good` false) | cold start: simplex reset to vertex 0, `count=1`, `div=1` | [x] |
| 16 | `c2GJK` (L400) | warm cache whose metric fails `!(min_metric < max_metric*2 && metric < -1e8)` | `cache_was_read = 1`, the cached simplex is **kept** (note: the guard is `metric < -1.0e8f`, so for any ordinary metric the condition is false and the cache is *always* accepted — replicate, do not "fix") | [x] |
| 17 | `c2GJK` (L420) | iteration budget exhausted: `iter == 20` | loop exits; witness computed from whatever simplex exists; `*iterations == 20`. **MEASURED: structurally unreachable.** `tests/iter_cap_search.rs` drove 300 000 randomized calls across all 9 type pairs, both `use_radius` values, wild transforms and warm caches: the highest `iterations` ever reported is **4**. With proxies of at most 4 vertices GJK cannot need more, so the `20` cap is dead code. What IS verified is that C and Rust agree on `iterations` exactly on every call, and that it never exceeds 20. | [x] |
| 18 | `c2GJK` (L440) | `d1 > d0` — objective stopped decreasing | `break` before adding a vertex | [x] |
| 19 | `c2GJK` (L446) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` — search direction degenerate | `break` | [x] |
| 20 | `c2GJK` (L466-473) | duplicate support point (`iA==saveA[i] && iB==saveB[i]`) | `break` without incrementing `s.count` | [x] |
| 21 | `c2GJK` (L430) | `s.count` reaches 3 after `c23` | `hit = 1`, `break`, then `a = b` and `dist = 0` | [x] |
| 22 | `c2GJK` (L481) | `use_radius != 0` and **not** (`dist > rA+rB && dist > FLT_EPSILON`) — i.e. shapes closer than the radius sum | collapses both witnesses to the midpoint `0.5*(a+b)` and forces `dist = 0` | [x] |
| 23 | `c2GJK` (L485-487) | `use_radius != 0`, `dist > rA+rB`, but after shrinking `a.x==b.x && a.y==b.y` | `dist = 0` | [x] |
| 24 | `c2GJK` (L478) | `use_radius == 0` | radius is **ignored entirely**; raw core distance returned | [x] |
| 25 | `c2GJK` | `cache->count > 3` or `cache->iA[i]` out of `[0, proxy.count)` — **the C bounds-checks nothing** | Split by what is actually defined (see "Rows deliberately NOT asserted as value-equal" below): **(a)** `count` in `1..=3` with indices below the proxy's initialised vertex count → fully defined, asserted bit-for-bit. **(b)** index ≥ proxy vertex count → the C reads uninitialised `c2Proxy.verts[]` stack bytes (measured: only 23 of 57 probes happened to agree). **(c)** `count > 3` → the C writes past `int saveA[3]` *and* past the 4-slot `c2sv a,b,c,d` simplex, corrupting its own frame; both builds SIGSEGV, at a threshold that moves between runs (observed 5 and 7) because it depends on stack layout. | [x] |
| 26 | `c2AABBtoCapsule` (L524) | `c2GJK(...)` returns non-zero (shapes apart) | returns `0` | [x] |
| 27 | `c2CapsuletoCapsule` (L530) | `c2GJK(...)` returns non-zero | returns `0` | [x] |
| 28 | `c2CircletoCircle` | `d2 >= r2` where `r2 = (A.r+B.r)^2` — note **`<`, so exactly touching is a miss** | returns `0` | [x] |
| 29 | `c2CircletoAABB` | `d2 >= A.r*A.r` after clamping — again strict `<` | returns `0` | [x] |
| 30 | `c2CircletoCapsule` | `d2 >= (A.r+B.r)^2` on the `da < 0` (before-`a`) branch | returns `0` | [x] |
| 31 | `c2CircletoCapsule` | same rejection on the `db < 0` (mid-segment) branch, which divides by `c2Dot(n,n)` → **`n == 0` (degenerate capsule `a==b`) yields NaN/inf** | returns `0` (NaN `<` compare is false) | [x] |
| 32 | `c2CircletoCapsule` | same rejection on the `db >= 0` (after-`b`) branch | returns `0` | [x] |
| 33 | `c2AABBtoAABB` | separated on **+x**: `B.max.x < A.min.x` | returns `0` | [x] |
| 34 | `c2AABBtoAABB` | separated on **-x**: `A.max.x < B.min.x` | returns `0` | [x] |
| 35 | `c2AABBtoAABB` | separated on **+y**: `B.max.y < A.min.y` | returns `0` | [x] |
| 36 | `c2AABBtoAABB` | separated on **-y**: `A.max.y < B.min.y` | returns `0` | [x] |
| 37 | `c2Collided` (L581-582) | `typeA == C2_TYPE_CIRCLE`, `typeB` out of range | `default: return 0` | [x] |
| 38 | `c2Collided` (L593-594) | `typeA == C2_TYPE_AABB`, `typeB` out of range | `default: return 0` | [x] |
| 39 | `c2Collided` (L605-606) | `typeA == C2_TYPE_CAPSULE`, `typeB` out of range | `default: return 0` | [x] |
| 40 | `c2Collided` (L609-610) | `typeA` out of range (any `typeB`) | `default: return 0` — **B is never dereferenced**, so a null/garbage `B` is safe here | [x] |
| 41 | `ptr_from_parts` (L614-641) | `typ` out of range — the `switch` has **no `default:` and the function falls off the end of a non-`void` function** | **Undefined behaviour.** With gcc `-O0` the return register holds leftover garbage. Rust returns `NULL`. Untestable as a value comparison; what *is* testable is that the pointer is never dereferenced by the only consumer (`omni_collide` → `c2Collided` row 37-40 returns 0 first), so `omni_collide` agrees. See row 42. | [x] |
| 42 | `omni_collide` | `type_a` and/or `type_b` out of range (`3`, `-1`, `4`, `INT_MAX`, `INT_MIN`) | `ptr_from_parts` returns an indeterminate pointer (row 41) which `c2Collided` discards via its `default:` → **`omni_collide` returns `0`** deterministically in both C and Rust | [x] |
| 43 | `c2Div` / `c2Norm` | `b == 0` / `c2Len(a) == 0` (zero vector) | `1.0f/0.0f = +inf` → components become `inf`, `-inf` or `NaN` (`0*inf`). No guard, no error. | [x] |
| 44 | `c2Witness` (L312) | `s->div == 0` | `den = 1/0 = inf`; witnesses become `inf`/`NaN` | [x] |
| 45 | `c2L` (L340) | `s->div == 0` | `den = inf`; for `count==2` result is `inf`/`NaN`; for `count==1` `den` is unused | [x] |
| 46 | `c2BBVerts` | called with an **inverted** AABB (`min > max`) | writes the 4 corners unchanged; no validation | [x] |
| 47 | all | any `float` argument is `NaN` / `±inf` / subnormal / `±0.0` | propagates through IEEE-754 arithmetic; every `<`/`>` against NaN is false. No checks anywhere. | [x] |

## Explicit magic constants in the C (all replicated verbatim in Rust)

| constant | C spelling | use |
|---|---|---|
| `FLT_MAX` | `3.40282346638528859811704183484516925e+38F` | initial `d0`, `d1` |
| `FLT_EPSILON` | `1.19209289550781250000000000000000000e-7F` | direction-degeneracy and `dist` thresholds |
| `-1.0e8f` | literal | cache-metric acceptance guard |
| `2.0f` | literal | cache-metric acceptance guard |
| `0.5f` | literal | witness-midpoint collapse |
| `20` | `while (iter < 20)` | GJK iteration cap |
| `3` | `int iA[3]`, `int iB[3]`, `saveA[3]`, `saveB[3]` | max cached / saved simplex vertices |
| `4` | `c2sv a, b, c, d` | simplex vertex slots |
| `8` | `c2v verts[8]` | proxy vertex slots (only 1/2/4 ever used) |

## Rows deliberately NOT asserted as value-equal

Three inputs are undefined behaviour in the C. For each, the test suite asserts
the part that IS defined and *measures* the rest rather than assuming it.

| row | why it is UB | how it is handled |
|---|---|---|
| 41 | `ptr_from_parts` falls off the end of a non-`void` function for an unknown `typ` — no `return` statement. | The pointer value is not compared. `errors_phase_c.rs::err_rows41_42_*` asserts neither build crashes and that the only consumer (`omni_collide`, row 42) returns a deterministic `0` on both. |
| 25(b) | An index ≥ the proxy's initialised vertex count reads uninitialised `c2Proxy.verts[]` stack bytes (the C's `c2Proxy pA;` is not zero-initialised). | `level3_gjk.rs::row64_out_of_count_cache_index_is_ub` asserts in-count indices agree bit-for-bit and reports the out-of-count agreement rate (23/57) instead of asserting it. |
| 25(c) / CONFIGS 74 | `cache->count > 3` writes past `saveA[3]` and past the 4-slot simplex; an out-of-range `C2_TYPE` passed straight to `c2GJK` leaves `c2Proxy.count` uninitialised so `c2Support` walks arbitrary memory. Both can segfault. | `probe_ub_isolated.rs` runs **each library in a forked child** so a crash is data, not a dead test binary. It asserts the crash thresholds match and that the Rust never crashes where the C survives. |

## The one tolerated difference: NaN payload bits

`eq_f32` in the test harness is bit-exact — `+0.0 != -0.0`, `+inf != -inf`, NaN
never equals a number — with a single tolerance: **two NaNs compare equal even
if their payload/sign bits differ.**

Root cause: gcc `-O0` and LLVM choose *opposite* SSE operand orders for the same
C expression. For `a.x += b.x` gcc emits `addss %xmm1,%xmm0` with `b.x` in
`xmm0` (computing `b + a`, so `b` is `src1`); LLVM emits `addps %xmm1,%xmm0`
(computing `a + b`, so `a` is `src1`). x86 returns `src1` when **both** operands
are NaN, so the two builds propagate a different input NaN. It also shows up
with internally generated NaNs: `0.0 * inf` raises invalid-operation and yields
the x86 "QNaN indefinite" `0xffc00000`, which then meets a propagated
`0x7fc00000` in the following add inside `c2Dot` / `c2Det2` / `c2Mulrv`.

It is not fixable from Rust source: LLVM canonicalizes commutative `fadd`/`fmul`
operand order and *merges* `b + a` with `a + b` into a single function — verified
by compiling both spellings and observing `nm` report two names at one address.
Matching gcc `-O0` would need per-expression inline assembly, and would then
stop matching a C build at any other optimization level.

It is also unobservable in anything the library decides: every `<` / `>` in
`c_src/src/lib.c` is false when either side is NaN, whatever the payload.
`tests/nan_payload.rs` asserts this directly — with arbitrary NaN payloads and
signaling NaNs on the input, **every `int` result** (including `omni_collide`
across all 9 type pairs × 30 000 iterations) and every non-NaN float result is
bit-identical, and `is_nan()` agreement is exact.

## Defect found and fixed during Phase C

`c2GJK` in the Rust indexed `saveA[i as usize]` / `saveB[i as usize]` with a
plain array index. A caller-supplied `cache->count > 3` therefore tripped a Rust
bounds check and **aborted the process**, where the C silently scribbles past
`int saveA[3]` and keeps running. The Rust now skips the out-of-range stores
(and reads them back through `get()`), so it does not abort; for every `count`
the library can itself produce — `1..=3`, since the write-back never reports
more — the behaviour is unchanged. See the comment at `src/lib.rs` in the
`while iter < 20` loop.

## Row → test mapping (Phase C gate)

All 47 rows are checked off above. Each is covered by a named test in
`translation/tests/`; run `cargo test --release` to verify.

| rows | test |
|---|---|
| 1 | `errors_phase_c.rs::err_row01_makeproxy_no_default_writes_nothing` |
| 2 | `errors_phase_c.rs::err_row02_simplex_metric_default_is_zero` |
| 3 | `errors_phase_c.rs::err_row03_c2D_default_is_zero_vector` |
| 4 | `errors_phase_c.rs::err_row04_witness_default_writes_zero_vectors` |
| 5 | `errors_phase_c.rs::err_row05_c2L_default_is_zero_vector` |
| 6, 7, 8 | `errors_phase_c.rs::err_rows06_08_support_returns_zero` |
| 9, 10 | `errors_phase_c.rs::err_rows09_10_null_transforms_equal_identity` |
| 11, 12, 13 | `errors_phase_c.rs::err_rows11_13_null_out_params_skip_stores` |
| 14 | `errors_phase_c.rs::err_row14_null_cache_matches_cold_cache` |
| 15 | `errors_phase_c.rs::err_row15_zero_count_cache_forces_cold_start` |
| 16 | `errors_phase_c.rs::err_row16_cache_metric_guard_both_outcomes` |
| 17, 18, 19, 20, 21 | `errors_phase_c.rs::err_rows17_21_gjk_loop_bailouts`, `iter_cap_search.rs::search_max_gjk_iterations` |
| 22, 23, 24 | `errors_phase_c.rs::err_rows22_24_use_radius_block` |
| 25 | `errors_phase_c.rs::err_row25_unchecked_cache_indices` (defined part), `level3_gjk.rs::row64_out_of_count_cache_index_is_ub` + `probe_ub_isolated.rs` (UB parts) |
| 26, 27 | `errors_phase_c.rs::err_rows26_27_gjk_backed_predicates_return_zero` |
| 28, 29 | `errors_phase_c.rs::err_rows28_29_strict_less_than_rejects_exact_touch` |
| 30, 31, 32 | `errors_phase_c.rs::err_rows30_32_circle_to_capsule_rejections` |
| 33, 34, 35, 36 | `errors_phase_c.rs::err_rows33_36_aabb_separating_axes` |
| 37, 38, 39, 40 | `errors_phase_c.rs::err_rows37_40_collided_default_arms` |
| 41, 42 | `errors_phase_c.rs::err_rows41_42_ptr_from_parts_ub_and_omni_collide`, `level5_dispatch.rs::row91_*` |
| 43 | `errors_phase_c.rs::err_row43_div_by_zero_unguarded` |
| 44, 45 | `errors_phase_c.rs::err_rows44_45_zero_div_in_witness_and_L` |
| 46 | `errors_phase_c.rs::err_row46_bbverts_no_validation` |
| 47 | `errors_phase_c.rs::err_row47_no_input_validation_anywhere`, `nan_payload.rs` |

Generic boundaries required by Phase C beyond the table:

| boundary | test |
|---|---|
| null pointers where the C does not dereference | `errors_phase_c.rs::generic_null_pointer_boundary` |
| out-of-range enum values (one past each end of `0..=2`, plus `i32::MIN`/`i32::MAX`) | `errors_phase_c.rs::generic_out_of_range_enum_boundary` |
| zero / oversized lengths and counts | `errors_phase_c.rs::generic_zero_and_oversized_counts` |
