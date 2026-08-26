# ERRORS.md — Phase A error / rejection surface

Mechanically derived from `c_src/src/lib.c`. Note up front what the grep found:

```
$ grep -n 'RETURN_ERROR\|return -1\|return NULL\|assert\|errno' c_src/src/lib.c
(no matches)
```

This library has **no error codes, no sentinels, no asserts and no return-value
based rejection at all**. Every function returns `void`, `float`, `int` (an
index) or a `c2v`. Its "error surface" is therefore made of:

1. `switch` statements whose `default:` / missing-`default:` arm is the
   fall-back for an out-of-range discriminant (`C2_TYPE`, `c2Simplex::count`),
2. explicit null-pointer guards (`if (!ax_ptr)`, `if (cache)`, `if (outA)`, …),
3. explicit range/degeneracy guards (`v <= 0`, `d1 > d0`, `c2Dot(d,d) < eps*eps`,
   `dist > rA + rB`, `while (iter < 20)`, the `dup` check),
4. divisions with no zero check (`1.0f / s->div`, `c2Div`, `c2Norm`), whose
   "error result" is an IEEE-754 `inf` / `NaN` that must be reproduced bit-exactly,
5. genuinely **undefined behaviour** for out-of-contract input (out-of-range
   `c2GJKCache::count` / `iA` / `iB`, invalid `C2_TYPE`), where the requirement on
   the Rust side is "must not panic/abort where C does not trap", because a
   bit-exact match to C's uninitialised stack bytes is not achievable.

Legend for the last column:
* **[x]** — differential test written and passing (bit-exact value equality).
* **[UB]** — C behaviour is undefined *and measurably non-deterministic*; the
  test asserts non-trapping behaviour instead of value equality (see the notes at
  the bottom, which also record the real defect this class exposed).
* **[UB-trap]** — the C itself faults; the test asserts **trap parity** by running
  the case in a child process against each `.so` and comparing exit status.

Status: **60 of 60 rows (plus row 35b) have a passing test.**

## Error-surface table

| # | function | trigger (exact invalid input / condition) | expected C result | ok |
|---|----------|--------------------------------------------|-------------------|----|
| 1 | `c2MakeProxy` | `type` not in `{0,1,2}` (`3`, `-1`, `99`, `INT_MIN`, `INT_MAX`) | `switch` has **no `default:`** → `*p` left byte-for-byte untouched, returns void | [x] |
| 2 | `c2GJKSimplexMetric` | `s->count == 1` | `case 1: return 0` | [x] |
| 3 | `c2GJKSimplexMetric` | `s->count` not in `{1,2,3}` (`0`, `4`, `-1`, `INT_MAX`) | `default:` falls into `case 1:` → `0.0f` | [x] |
| 4 | `c2D` | `s->count == 3` | `case 3:` → `c2V(0,0)` | [x] |
| 5 | `c2D` | `s->count` not in `{1,2,3}` (`0`, `4`, `-1`) | `default:` → `c2V(0,0)` | [x] |
| 6 | `c2Witness` | `s->count` not in `{1,2,3}` | `default:` → `*a = *b = c2V(0,0)` | [x] |
| 7 | `c2Witness` | `s->div == 0.0f`, `count` 2 or 3 | `den = 1.0f/0 = +inf`; `den*u` → `±inf` or `NaN` (when `u==0`) written into `*a`/`*b` | [x] |
| 8 | `c2Witness` | `s->div` is `NaN` | `den = NaN` → all components `NaN` | [x] |
| 9 | `c2L` | `s->count == 3`, or any value not in `{1,2}` | `default:` → `c2V(0,0)` (note: **`count==3` is a `default` here but a real case in `c2Witness`**) | [x] |
| 10 | `c2L` | `s->div == 0.0f`, `count == 2` | `den = +inf` → `inf`/`NaN` components | [x] |
| 11 | `c2Support` | `count <= 0` (`0`, `-1`, `INT_MIN`) | `verts[0]` is read **before** the loop guard; loop never runs → returns `0` | [x] |
| 12 | `c2Support` | `count == 1` | loop never runs → returns `0` | [x] |
| 13 | `c2Support` | `d == (0,0)` → every dot is `0`, `dot > dmax` never true | returns `0` (first index wins all ties) | [x] |
| 14 | `c2Support` | some `verts[i]` contains `NaN` → `dot > dmax` false | that index is never selected; earlier index kept | [x] |
| 15 | `c2Div` | `b == 0.0f` | `1.0f/0 = +inf` → `(x*inf, y*inf)`: `±inf`, or `NaN` when the component is `0` | [x] |
| 16 | `c2Div` | `b == -0.0f` | `1.0f/-0.0 = -inf` → sign-flipped `inf`/`NaN` | [x] |
| 17 | `c2Norm` | `a == (0,0)` → `c2Len(a) == 0` | `c2Div(a, 0)` → `(0*inf, 0*inf)` = `(NaN, NaN)` | [x] |
| 18 | `c2Norm` | `a` contains `inf` → `c2Dot` = `inf`, `c2Len` = `inf` | `1/inf = 0` → `(inf*0, …)` = `NaN` components | [x] |
| 19 | `c2Norm` | `a` contains `NaN` | `c2Len` = `NaN` → `(NaN, NaN)` | [x] |
| 20 | `c2Len` | `c2Dot(a,a)` overflows to `+inf` (e.g. `1e30`) | `sqrtf(inf) = inf` | [x] |
| 21 | `c2Len` | `a` contains `NaN` | `sqrtf(NaN) = NaN` | [x] |
| 22 | `c2Maxv` / `c2Minv` | either operand is `NaN` → `a.x > b.x` / `a.x < b.x` is false | always yields `b`'s component; `NaN` in `a` is **dropped**, `NaN` in `b` is **kept** | [x] |
| 23 | `c2Clampv` | `lo > hi` (inverted, "invalid" range — no validation) | `c2Maxv(lo, c2Minv(a,hi))` → `lo` | [x] |
| 24 | `c2GJK` | `ax_ptr == NULL` | `if (!ax_ptr) ax = c2xIdentity()` — accepted, no deref | [x] |
| 25 | `c2GJK` | `bx_ptr == NULL` | `if (!bx_ptr) bx = c2xIdentity()` — accepted, no deref | [x] |
| 26 | `c2GJK` | `cache == NULL` | `if (cache)` guards both the read and the write-back; cold start | [x] |
| 27 | `c2GJK` | `outA == NULL` | `if (outA)` → no store | [x] |
| 28 | `c2GJK` | `outB == NULL` | `if (outB)` → no store | [x] |
| 29 | `c2GJK` | `iterations == NULL` | `if (iterations)` → no store | [x] |
| 30 | `c2GJK` | all four out-params `NULL` simultaneously | only the return value is produced | [x] |
| 31 | `c2GJK` | `cache->count == 0` | `!!cache->count` == 0 → `cache_was_good = 0` → cold start from vertex 0 | [x] |
| 32 | `c2GJK` | `cache->count != 0`, ordinary metrics | `!(min_metric < max_metric*2.0f && metric < -1.0e8f)`: the 2nd conjunct is essentially never true, so the negation is **essentially always true** → `cache_was_read = 1`; **a stale cache is accepted**. This is the "odd" condition; replicated verbatim | [x] |
| 33 | `c2GJK` | `cache->count != 0` **and** `metric < -1.0e8f` **and** `min_metric < max_metric*2.0f` | `cache_was_read` stays `0` → the cached simplex is silently discarded and the search cold-starts (reachable with `count==3` and a hugely negative determinant, e.g. coords ~`1e5`) | [x] |
| 34 | `c2GJK` | `cache->count < 0` (`-1`, `-100`, `INT_MIN`) | `!!count` is true → the read loop body never runs → `s.count < 0`, `s.div = cache->div`; `switch(s.count)` matches no case; `c2L`/`c2D`/`c2Witness` all take `default` → `dist = 0`, `iter = 0`, and the negative `count` is written straight back into the cache. **Fully deterministic**; measured identical in C and Rust | [x] |
| 35 | `c2GJK` | `cache->count == 4` **and `cache->div != 0`** | reads `cache->iA[3]` (aliases `cache->iB[0]`) and `cache->iB[3]` (aliases the *bytes of* `cache->div` type-punned as `int` — e.g. `div=1.0` → index `1065353216`) → wild read → **SIGSEGV in the C itself**. Measured: C and Rust both die with signal 11 | [UB-trap] |
| 35b | `c2GJK` | `cache->count` in `4..=8` **and `cache->div == 0.0`** | the type-punned index is then `0`, so the C does **not** trap: it returns `dist=0`, `iter=0`, and writes the out-of-range `count` straight back. Measured: C and Rust agree exactly (`dist=0`, `count=N`, `iters=0`) | [x] |
| 36 | `c2GJK` | `cache->count >= 9` | writes `verts[i]` past the end of `c2Simplex` and past `saveA`/`saveB` → frame corruption. Measured: C traps at `count=12`, both trap at `count=100`. Not matchable in principle | [UB-trap] |
| 37 | `c2GJK` | `cache->iA[i]` / `iB[i]` in `[shape_vert_count, 8)` | reads an **uninitialised** `c2Proxy::verts[]` slot (`c2Proxy pA;` is a bare local, never zeroed) → garbage. Measured **non-deterministic in C**: the identical call returned `dist=inf` on one run and `dist=0` on another, so there is no fixed value to match. Requirement: Rust must not trap (it reads its own zeroed slot) | [UB] |
| 38 | `c2GJK` | `cache->iA[i]` / `iB[i]` `>= 8` or negative (`8`, `1000`, `-5`) | indexes past `verts[8]` → reads unrelated stack, non-deterministically. **This is where the translation was genuinely defective**: Rust's bounds-checked `pA.verts[iA as usize]` panicked → `abort()` (SIGABRT) while the C returned normally. Fixed by mirroring the C's pointer arithmetic; neither traps now | [UB] |
| 39 | `c2GJK` | `typeA` / `typeB` not in `{0,1,2}` | `c2MakeProxy` leaves the local `c2Proxy` untouched → the whole proxy (`radius`, `count`, `verts`) is uninitialised stack, so C's result is garbage/non-deterministic. Requirement: Rust must not trap | [UB] |
| 40 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (shapes overlap within their radii) | else-branch: `a = b = midpoint(a,b)`, `dist = 0` | [x] |
| 41 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (coincident witnesses) | same else-branch → `dist = 0` | [x] |
| 42 | `c2GJK` | `use_radius != 0`, shrink taken, and `a == b` exactly afterwards | `if (a.x==b.x && a.y==b.y) dist = 0` | [x] |
| 43 | `c2GJK` | `use_radius == 0` with non-zero radii | radii are **ignored**; raw core-shape witness distance returned (can be > 0 for visually overlapping shapes) | [x] |
| 44 | `c2GJK` | `use_radius` non-zero but not `1` (`2`, `-1`, `INT_MIN`) | `if (use_radius)` is a truth test → same as `1` | [x] |
| 45 | `c2GJK` | simplex reaches `count == 3` (containment) | `hit = 1; break;` → `a = b`, `dist = 0`, **bypassing the whole `use_radius` block** | [x] |
| 46 | `c2GJK` | search stops making progress: `d1 > d0` | `break` with the current simplex (no extra vertex appended) | [x] |
| 47 | `c2GJK` | degenerate search direction: `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` | `break` | [x] |
| 48 | `c2GJK` | new support point duplicates a saved `(iA,iB)` pair | `dup = 1; break;` — the vertex **is** written into `verts[s.count]` but `s.count` is **not** incremented | [x] |
| 49 | `c2GJK` | non-terminating configuration | `while (iter < 20)` hard-caps the loop; `*iterations <= 20` | [x] |
| 50 | `c2GJK` | shape coordinates contain `NaN` | every guard comparison is false; `d1 > d0` false, `c2Dot(d,d) < eps*eps` false, `dist > rA+rB` false → midpoint branch, `dist = 0`, witnesses `NaN` | [x] |
| 51 | `c2GJK` | shape coordinates contain `±inf` | `c2Dot` → `inf`/`NaN` propagation through the same guards | [x] |
| 52 | `c2GJK` | AABB with `min > max` (inverted/"invalid" box) | no validation: `c2BBVerts` builds a wound-backwards quad and GJK runs on it | [x] |
| 53 | `c2GJK` | capsule with `a == b` (zero-length segment) | no validation: 2-vertex proxy with duplicate verts; `dup` check terminates the loop | [x] |
| 54 | `c2GJK` | negative shape radius (`c2Circle::r < 0`, `c2Capsule::r < 0`) | no validation: `dist > rA + rB` with a negative sum → shrink branch **grows** the distance | [x] |
| 55 | `c2GJK` | `A` / `B` shape pointer `NULL` with a *valid* type | no null check → `c2MakeProxy` dereferences it → SIGSEGV | [UB-trap] ✔ |
| 56 | `c2BBVerts` | `out == NULL` or `bb == NULL` | no null check → SIGSEGV | [UB-trap] ✔ |
| 57 | `c22` / `c23` / `c2D` / `c2L` / `c2Witness` / `c2GJKSimplexMetric` | `s == NULL` | no null check → SIGSEGV | [UB-trap] ✔ |
| 58 | `c2Support` | `verts == NULL` | `verts[0]` read unconditionally → SIGSEGV | [UB-trap] ✔ |
| 59 | `gjk_cache` | `a9`/`b9` `NULL`, or any `NaN`/`inf` float argument | `a9`/`b9` are **never dereferenced and never written** by the C code; the function has *no* observable output. Must not crash, must not write | [x] |
| 60 | `gjk_cache` | `reverse` any non-zero `char` (`1`, `-1`, `0x7f`) | `if (reverse)` truth test → the swapped-argument `c2GJK` call | [x] |

## Row → test mapping

Every row is discharged by a named test. Run `cargo test --no-default-features`
(and see `run_all.sh`, which also runs the suite against the release `.so`).

| rows | test |
|------|------|
| 1 | `errors::err01_makeproxy_invalid_enum`, `errors::err01b_makeproxy_invalid_enum_null_shape` |
| 2, 3 | `errors::err02_03_metric_out_of_range_count` |
| 4, 5 | `errors::err04_05_c2d_out_of_range_count` |
| 6 | `errors::err06_c2witness_out_of_range_count` |
| 7, 8, 10 | `errors::err07_08_10_zero_and_nan_div` |
| 9 | `errors::err09_c2l_out_of_range_count` |
| 11, 12, 13, 14 | `errors::err11_12_13_14_support_degenerate`, `simplex::row26_c2support_nonpositive_count` |
| 15, 16 | `errors::err15_16_div_by_zero` |
| 17, 18, 19 | `errors::err17_18_19_norm_degenerate` |
| 20, 21 | `errors::err20_21_len_overflow_and_nan` |
| 22 | `errors::err22_maxv_minv_nan_asymmetry` |
| 23 | `errors::err23_clampv_inverted_range` |
| 24, 25 | `errors::err24_25_null_transform_pointers` |
| 26 | `errors::err26_null_cache` |
| 27, 28, 29, 30 | `errors::err27_28_29_30_null_out_params` |
| 31 | `errors::err31_cache_count_zero_is_cold_start` |
| 32 | `errors::err32_stale_cache_is_accepted` (4050 accepted stale caches observed) |
| 33 | `errors::err33_cache_rejected_when_metric_below_minus_1e8` (branch taken 1538×) |
| 34 | `errors::err34_negative_cache_count` |
| 35, 36, 55, 56, 57, 58 | `errors::err35_36_55_58_trap_parity` (subprocess trap-parity, 14 cases) |
| 35b | `errors::err35b_out_of_range_count_with_zero_div` |
| 37, 38, 39 | `errors::err37_38_39_ub_rows_do_not_trap` (51 200 calls per side, none trapped) |
| 40, 41, 42 | `errors::err40_41_42_use_radius_branches` |
| 43, 44 | `errors::err43_44_use_radius_zero_and_truthiness` |
| 45 | `errors::err45_hit_path_bypasses_use_radius` |
| 46 | `errors::err46_no_progress_break` |
| 47 | `errors::err47_degenerate_direction_break` |
| 48 | `errors::err48_duplicate_support_break` |
| 49 | `errors::err49_iteration_cap` |
| 50, 51 | `errors::err50_51_nan_and_inf_coordinates` |
| 52, 53, 54 | `errors::err52_53_54_unvalidated_shape_geometry` |
| 59, 60 | `public::row54_null_out_pointers`, `public::row55_extreme_arguments`, `public::row56_random_sweep` |

### How rows 46/47/48 are *proved*, not assumed

The three loop-exit guards are checked in a fixed order (`d1 > d0`, then
`c2Dot(d,d) < eps*eps`, then `dup`), so a construction that makes the earlier
guards provably not fire pins which break was taken:

* **row 46** — coordinates near `1e30` make `d1 = dot(p,p)` overflow to `+inf`,
  and on the first pass `d0 == FLT_MAX`, so `d1 > d0` is the *only* guard that
  can fire. The test recomputes `d1` from the exported helpers and asserts it is
  `+inf`, then asserts `*iterations == 0`.
* **row 47** — two byte-identical shapes give `p = sB - sA = (0,0)`, hence
  `d1 = 0` (row 46 cannot fire) and `c2D` returns `(0,0)`, so the degenerate
  direction guard must fire. Proved on 4096 constructions.
* **row 48** — two circles are both 1-vertex proxies, so `c2Support` can only
  return `0`, which already equals the saved `(iA,iB) = (0,0)`. With the circles
  separated, `d1` is finite and `d != (0,0)`, so `dup` is the only reachable
  exit. Proved on 4096 constructions.

### Finding on row 49 (the `iter < 20` cap)

The cap is **structurally unreachable** for this library: every `c2Proxy` holds
at most 4 vertices, so the search always terminates via row 46/47/48 first.
A 30 000-case randomised sweep observed a maximum of **3** iterations
(histogram `[7583, 14280, 6620, 1517, 0, …]`). What the test asserts is therefore
that C and Rust agree on `*iterations` for every case and that the cap is never
breached — not that the cap itself fires.

## Notes on the `[UB]` rows (37–39) and the `[UB-trap]` rows (35, 36, 55–58)

* **Rows 37–39** read memory outside the object bounds in C. There is no Rust
  construct that reproduces "whatever gcc -O0 left on the stack" — and the C is
  measurably **non-deterministic** here (the identical call returned `dist=inf`
  on one run and `dist=0` on another), so a bit-exact differential assertion is
  impossible *in principle*. What is required, and what is tested, is that Rust
  never turns a non-trapping C execution into a `panic`/`abort`.
  **This is where a real defect was found and fixed:** the original translation
  used bounds-checked slice indexing (`pA.verts[iA as usize]`,
  `(*cache).iA[i as usize]`, `s.verts[s.count as usize]`, `saveA[i as usize]`).
  With `cache->iA[0] = 8` the C returned `dist=inf` normally while Rust panicked
  with *"index out of bounds: the len is 8 but the index is 8"* and died of
  SIGABRT. `src/lib.rs` now mirrors the C's pointer arithmetic
  (`pa_verts.offset(iA as isize)`, `verts.offset(i as isize)`, …) at every
  caller-controlled index. See `tests/errors.rs::err37_38_39_ub_rows_do_not_trap`,
  which makes 51 200 such calls per library without a single trap.

* **Rows 35, 36, 55–58** fault in the C itself. They are *not* skipped: each is
  executed in a **child process** (`tests/errors.rs::trap_worker`, driven by
  `err35_36_55_58_trap_parity`) once against the C `.so` and once against the
  Rust `.so`, and the two exit statuses are compared. All 14 cases die with
  **signal 11 in both**. Child runs are bounded by a 10 s deadline because frame
  corruption can spin instead of faulting.

* **Trap parity is asserted against the `release` `.so`**, the artifact this
  crate actually ships (`crate-type = ["cdylib"]`). In a `dev` build rustc's
  `-Cdebug-assertions` inserts a null check on every raw-pointer dereference, so
  a null deref reports *"null pointer dereference occurred"* and aborts with
  SIGABRT instead of segfaulting. That is a Rust development aid, not a property
  of the translated code; `run_all.sh` builds release and the test skips with an
  explanatory message if `target/release/libgjk_cache_lib.so` is absent.

* **A second real defect** surfaced while chasing row 58: the C evaluates
  `c2Dot(verts[0], d)` *unconditionally*, before the loop guard ever reads
  `count`. In the optimised Rust build LLVM deleted that load because `dmax` is
  unused when `count <= 0`, so `c2Support(NULL, 0, d)` returned `0` quietly while
  the C segfaulted. Fixed with `core::ptr::read_volatile` to pin the
  dereference.

## Note on NaN payloads

Wherever an *input* already contains a NaN, the tests compare "both are NaN"
rather than the exact payload, and the reason is mechanical rather than a
concession: `addss dst, src` returns the **destination** operand when both
operands are NaN, and which product lands in the destination register is a
register-allocation choice that differs between gcc -O0 and LLVM. From the
disassembly of `c2Dot`:

```
C   (gcc -O0): mulss ->%xmm1 (a.x*b.x), mulss ->%xmm0 (a.y*b.y), addss %xmm1,%xmm0
Rust (LLVM)  : mulss ->%xmm3 (a.y*b.y), mulss ->%xmm0 (a.x*b.x), addss %xmm3,%xmm0
```

Recompiling the *C* at `-O2` changes its payload too, so the payload is outside
the ABI contract. Everything else stays bit-exact, **including**
hardware-generated NaNs: an invalid operation (`inf*0`, `0/0`, `inf-inf`) yields
the x86 indefinite `0xffc00000` in both languages because both emit the same SSE
instruction. Consequently all NaN-free inputs — including `±0`, `±inf`,
subnormals and `±FLT_MAX` — are compared with full bit-exactness.
