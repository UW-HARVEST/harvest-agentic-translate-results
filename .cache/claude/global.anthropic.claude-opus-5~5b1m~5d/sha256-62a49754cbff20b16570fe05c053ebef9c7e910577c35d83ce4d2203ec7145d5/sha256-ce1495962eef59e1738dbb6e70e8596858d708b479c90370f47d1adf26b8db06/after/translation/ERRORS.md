# ERRORS.md — Phase A error-surface table

Mechanically derived from `c_src/src/lib.c`. This library has **no** error
macros, no `RETURN_ERROR`, no `assert`, no `errno`, and no error enum. Its
"rejection" surface is therefore made of:

* `switch` statements with (or *without*) a `default:` label,
* `return 0` / `return 1` sentinels from the boolean `c2*to*` predicates,
* null-pointer guards (`if (!ax_ptr)`, `if (cache)`, `if (outA)`, …),
* `<= 0` / `< 0` region rejections inside the simplex solvers,
* the hard iteration cap `while (iter < 20)` and the `FLT_EPSILON` /
  `FLT_MAX` / `-1.0e8f` / `2.0f` magic constants,
* divisions by a possibly-zero denominator (`1.0f / s->div`, `1.0f / b`,
  `da / c2Dot(n, n)`),
* one function that **falls off the end of a non-`void` function**
  (`ptr_from_parts` with an unrecognised type).

Every row below is a distinct rejection branch in the C source, cited by line
number. `[x]` = a differential test in `tests/phase_c_errors.rs` constructs that
exact condition, calls both `.so`s and asserts identical results.

| #  | function | trigger (the exact invalid input/condition) | expected C result | [x] |
|----|----------|----------------------------------------------|-------------------|-----|
| 1  | `c2Collided` (L609) | `typeA` is any `int` outside `{0,1,2}` (e.g. `3`, `-1`, `INT_MIN`, `INT_MAX`) | outer `switch` `default:` → `return 0`; `A`/`B` never dereferenced | [x] |
| 2  | `c2Collided` (L581) | `typeA == C2_TYPE_CIRCLE (1)` and `typeB` outside `{0,1,2}` | inner `default:` → `return 0` | [x] |
| 3  | `c2Collided` (L593) | `typeA == C2_TYPE_AABB (2)` and `typeB` outside `{0,1,2}` | inner `default:` → `return 0` | [x] |
| 4  | `c2Collided` (L605) | `typeA == C2_TYPE_CAPSULE (0)` and `typeB` outside `{0,1,2}` | inner `default:` → `return 0` | [x] |
| 5  | `c2Collided` (L576-604), `c2GJK` (L373-374 via `c2MakeProxy`), `c2BBVerts` (L102), `c2MakeProxy` (L111/117/123), `c2Support` (L295), `c22`/`c23`/`c2D`/`c2L`/`c2Witness`/`c2GJKSimplexMetric` (`s`) | any of these pointers is `NULL` while the corresponding type is valid | dereferences NULL → SIGSEGV in both; *not* a testable difference (documented, deliberately not exercised) | [x] (documented, skipped) |
| 6  | `ptr_from_parts` (L636) | `typ` outside `{0,1,2}` | control reaches the closing `}` of a non-`void` function → **indeterminate** return value (C UB). Rust returns `NULL`. Not observable through a deterministic assertion; only its *consequence* (row 7/8) is asserted | [x] (documented, indirect) |
| 7  | `omni_collide` (L641) | `type_a` outside `{0,1,2}` (any `type_b`) | `ptr_from_parts` returns indeterminate, then `c2Collided`'s outer `default:` → `0` (pointer never dereferenced) | [x] |
| 8  | `omni_collide` (L642) | `type_a` valid, `type_b` outside `{0,1,2}` | `c2Collided`'s inner `default:` → `0` | [x] |
| 9  | `c2MakeProxy` (L109-129) | `type` outside `{0,1,2}` | `switch` has **no `default:`** → `*p` is left *completely unmodified* (radius, count and all 8 verts keep the caller's prior bytes) | [x] |
| 10 | `c2GJK` (L363) | `ax_ptr == NULL` | `ax = c2xIdentity()` = `{p:{0,0}, r:{c:1,s:0}}` | [x] |
| 11 | `c2GJK` (L367) | `bx_ptr == NULL` | `bx = c2xIdentity()` | [x] |
| 12 | `c2GJK` (L378, L495) | `cache == NULL` | both the cache-read and the cache-write blocks are skipped; a fresh 1-vertex simplex is built | [x] |
| 13 | `c2GJK` (L379) | `cache != NULL` **and** `cache->count == 0` | `cache_was_good = 0` → cache **not** read → fresh simplex; cache is still *written* on the way out | [x] |
| 13b | `c2GJK` (L379-504) | `cache != NULL` **and** `cache->count < 0` (`-1`, `-100`, `INT_MIN`, ...) | `!!cache->count` is **true**, so the C *enters* the reload block with a negative count: every count-bounded loop runs 0 times and every `switch (s.count)` falls to `default:`, so the whole path is well defined -> `dist = 0`, `outA = outB = (0,0)`, `iterations = 0`, and the cache is written back with `metric = 0`, `count` unchanged (still negative) and `iA`/`iB` **untouched**. This is the *only* input that distinguishes `!!cache->count` from `cache->count > 0` | [x] |
| 14 | `c2GJK` (L400) | `cache != NULL`, `cache->count != 0`, and `!(min_metric < max_metric*2.0f && metric < -1.0e8f)` (which is *almost always* true because of the `-1.0e8f`) | `cache_was_read = 1` → the cached simplex is reused verbatim, `s.count = cache->count`, `s.div = cache->div` | [x] |
| 14b | `c2GJK` (L381-393) | every legitimate warm cache: `count` in `{1,2,3}` x every in-range `iA`/`iB` index for each proxy type (1 vert for a circle, 2 for a capsule, 4 for an AABB) x hostile `metric`/`div` | the cached simplex is rebuilt from the *current* poses and re-validated against them | [x] |
| 15 | `c2GJK` (L400) | `cache->metric` / `cache->div` deliberately poisoned so that `min_metric < max_metric*2 && metric < -1.0e8f` **holds** (needs `metric < -1e8`, only reachable with `count == 3`) | `cache_was_read` stays `0` → cached indices are overwritten by the fresh simplex | [x] |
| 15b | `c2GJK` (L400) | the exact `<` boundary: `min_metric == max_metric * 2.0f` **and** `metric < -1.0e8f`. Constructed as `A = AABB(0,0)-(1,y)`, `B = AABB(x,0)-(0,y)` (an inverted box -- the C validates nothing), `cache = {count:3, iA:[0,0,0], iB:[0,1,2]}` so the reloaded `metric = c2Det2((-x,0),(-x,y)) = -x*y`; setting `cache->metric = metric/2` puts `max_metric*2.0f` exactly on `min_metric` | strict `<` => the `&&` is **false** => `cache_was_read = 1` (cached simplex used). The test self-validates that the boundary is observable: moving `cache->metric` one f32 step toward zero flips to the fresh-simplex path and yields a different `dist`, witness pair and written-back cache | [x] |
| 16 | `c2GJK` (L505) | `outA == NULL` | no store to `*outA` (caller's buffer untouched) | [x] |
| 17 | `c2GJK` (L507) | `outB == NULL` | no store to `*outB` | [x] |
| 18 | `c2GJK` (L509) | `iterations == NULL` | no store to `*iterations` | [x] |
| 19 | `c2GJK` (L420) | GJK never converges within the cap | `while (iter < 20)` would exit with `iter == 20`. **STRUCTURALLY UNREACHABLE**: a `c2Proxy` here holds at most 4 vertices (AABB), so the loop always leaves through one of the other four exits first. 2,000,000 randomized calls (`tests/search_maxiter.rs`, an `#[ignore]`d diagnostic) across all 9 type pairs, weird rotors, extreme coordinates and hostile caches observe a maximum of **5** iterations -- histogram `[982285, 758030, 228759, 30419, 496, 11, 0, ...]`. The test therefore asserts the two things that ARE observable: both implementations agree on `*iterations` for every input, and it always stays within `0..=20`. **Corroborated by gcov**: running the whole suite against a `--coverage` build of `lib.c` reaches 100% of its 445 lines and 100% of its 157 branches, and the *only* branch arc never taken in the entire library is `while (iter < 20)` branch 1 (the cap-exit fallthrough) -- see the Coverage section below | [x] (unreachable; agreement + bound asserted) |
| 20 | `c2GJK` (L442) | `d1 > d0` (distance to origin stopped decreasing) | `break` out of the loop *before* `++iter` | [x] |
| 21 | `c2GJK` (L446) | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` where `FLT_EPSILON = 1.1920929e-7f` | `break` (degenerate search direction) | [x] |
| 22 | `c2GJK` (L461-467) | new support pair `(iA,iB)` equals a saved pair | `dup = 1` → `break` without incrementing `s.count`/`iter` | [x] |
| 23 | `c2GJK` (L436) | `s.count == 3` after `c23()` (origin enclosed) | `hit = 1`, `break`, then `a = b` and `dist = 0` — **radii are never subtracted even when `use_radius != 0`** | [x] |
| 24 | `c2GJK` (L480-493) | `use_radius != 0` and **not** (`dist > rA+rB && dist > FLT_EPSILON`) — i.e. shapes overlap once radii are counted, or the witness points coincide | `a = b = 0.5*(a+b)`, `dist = 0` | [x] |
| 25 | `c2GJK` (L486) | `use_radius != 0`, separated, but `a.x==b.x && a.y==b.y` after shrinking by `rA`/`rB` | `dist = 0` (while `a`,`b` keep their shrunk values) | [x] |
| 26 | `c2GJK` (L477) | `use_radius == 0` | radii are **never** subtracted; `dist` is the core-shape distance | [x] |
| 27 | `c2GJK` (L381-393) | `cache->iA[i]` / `cache->iB[i]` index at or beyond `pA.count` / `pB.count` (but still `< 8`) | reads a *never-initialised* `c2Proxy.verts[]` slot → indeterminate in C. Documented; asserted only for indices that are in range | [x] (documented, skipped) |
| 28 | `c2GJKSimplexMetric` (L157-159) | `s->count` is anything other than `2` or `3` (`0`, `1`, `4`, `-1`, `INT_MAX`, …) | `default:`/`case 1:` → `return 0` | [x] |
| 29 | `c2D` (L287-289) | `s->count == 3`, or any value outside `{1,2}` | `case 3:`/`default:` → `return c2V(0,0)` | [x] |
| 30 | `c2Witness` (L327-329) | `s->count` outside `{1,2,3}` | `default:` → `*a = *b = c2V(0,0)` (note: `den` is still computed, so a `div` of 0 is harmless here) | [x] |
| 31 | `c2Witness` (L307) | `s->div == 0` with `s->count` in `{1,2,3}` | `den = 1.0f/0.0f = +inf`; `count==1` is unaffected, `count>=2` yields `inf`/`NaN` witness components | [x] |
| 32 | `c2L` (L349-350) | `s->count` outside `{1,2}` (incl. `3`) | `default:` → `return c2V(0,0)` | [x] |
| 33 | `c2L` (L342) | `s->div == 0` and `s->count == 2` | `den = +inf` → `inf`/`NaN` components | [x] |
| 34 | `c2Support` (L296) | `count <= 1` (incl. `0`, `-1`, `INT_MIN`) | the `for` body never runs → `return 0`; **`verts[0]` is still dereferenced unconditionally**, so `count == 0` is *not* a "no read" case | [x] |
| 35 | `c2Support` (L293) | `count` larger than the real array (over-long length) | reads past the end → indeterminate. Documented; only in-range counts are asserted | [x] (documented, skipped) |
| 36 | `c2Div` (L334) | `b == 0` | `1.0f/0.0f = +inf`; `(x*inf, y*inf)`, and `0.0f*inf = NaN` | [x] |
| 37 | `c2Div` (L334) | `b == -0.0` | `1.0f/-0.0f = -inf` | [x] |
| 38 | `c2Norm` (L338) | `a == (0,0)` | `c2Len = 0` → `c2Div(a, 0)` → `(NaN, NaN)` (`0 * inf`) | [x] |
| 39 | `c2Len` (L148) | `a` contains `inf` / `NaN` | `c2Dot(a,a)` is `inf`/`NaN` → `sqrtf` gives `inf`/`NaN` (never the negative-argument `NaN` path, since `x*x+y*y` cannot be finite-negative) | [x] |
| 40 | `c22` (L186) | `v = c2Dot(a, a-b) <= 0` | collapse to vertex A: `a.u = 1`, `div = 1`, `count = 1` | [x] |
| 41 | `c22` (L190) | `u = c2Dot(b, b-a) <= 0` (and `v > 0`) | collapse to vertex B: `s->a = s->b`, `a.u = 1`, `div = 1`, `count = 1` | [x] |
| 42 | `c23` (L217) | `vAB <= 0 && uCA <= 0` | vertex-A region: `count = 1`, `div = 1` | [x] |
| 43 | `c23` (L221) | `uAB <= 0 && vBC <= 0` | vertex-B region: `s->a = s->b`, `count = 1` | [x] |
| 44 | `c23` (L226) | `uBC <= 0 && vCA <= 0` | vertex-C region: `s->a = s->c`, `count = 1` | [x] |
| 45 | `c23` (L231) | `uAB > 0 && vAB > 0 && wABC <= 0` | edge-AB region: `count = 2`, `div = uAB+vAB` | [x] |
| 46 | `c23` (L236) | `uBC > 0 && vBC > 0 && uABC <= 0` | edge-BC region: `a=b; b=c;`, `count = 2` | [x] |
| 47 | `c23` (L243) | `uCA > 0 && vCA > 0 && vABC <= 0` | edge-CA region: `b=a; a=c;`, `count = 2` | [x] |
| 48 | `c23` (L250) | none of the six region tests fire | interior: `count = 3`, `div = uABC+vABC+wABC` (may be `0` or negative) | [x] |
| 49 | `c2AABBtoAABB` (L515-519) | any of the four separating tests is true (`B.max.x < A.min.x`, `A.max.x < B.min.x`, `B.max.y < A.min.y`, `A.max.y < B.min.y`) | `return 0` | [x] |
| 50 | `c2AABBtoAABB` (L515-519) | any coordinate is `NaN` | every `<` is false → `d0|d1|d2|d3 == 0` → **`return 1`** ("NaN boxes always collide") | [x] |
| 51 | `c2AABBtoAABB` (L515-519) | inverted box (`min > max`) | no validation at all; the four tests are evaluated literally | [x] |
| 52 | `c2CircletoCircle` (L539) | `d2 >= r2` | `return 0`; note `r2 = (A.r+B.r)^2` so a **negative** total radius behaves like its magnitude | [x] |
| 53 | `c2CircletoCircle` (L539) | `A.r + B.r == 0` | `r2 == 0`, `d2 < 0` impossible → always `return 0`, even for coincident centres | [x] |
| 54 | `c2CircletoAABB` (L547) | `d2 >= r2`, i.e. `A.r == 0` or the clamped point is outside the radius | `return 0` (`A.r*A.r` — a negative radius acts like `|A.r|`) | [x] |
| 55 | `c2CircletoAABB` (L543) | inverted AABB (`B.min > B.max`) | `c2Clampv = c2Maxv(lo, c2Minv(a, hi))` collapses to `lo`; no validation | [x] |
| 56 | `c2CircletoCapsule` (L555) | `da = dot(A.p-B.a, B.b-B.a) < 0` | nearest feature is capsule end `a`; `d2 = |ap|^2` | [x] |
| 57 | `c2CircletoCapsule` (L559) | `da >= 0` and `db = dot(A.p-B.b, n) < 0` | projection onto the segment; if the capsule is **degenerate** (`B.a == B.b`) then `c2Dot(n,n) == 0` and `da/0` is `NaN`/`inf` → `d2 = NaN` → `return 0`. (With `B.a == B.b`, `n == (0,0)` so `da == 0`, `db == 0`, so this branch is *not* taken — the `db >= 0` branch is.) | [x] |
| 58 | `c2CircletoCapsule` (L562) | `da >= 0 && db >= 0` | nearest feature is capsule end `b`; `d2 = |bp|^2` | [x] |
| 59 | `c2CircletoCapsule` (L568) | `d2 >= r*r` where `r = A.r+B.r` | `return 0` | [x] |
| 60 | `c2AABBtoCapsule` (L523) | `c2GJK(...) != 0.0f` (C's implicit `if (float)` truth test) | `return 0`, else `return 1`. Note: the predicate hard-codes `use_radius = 1`, and the L488 `else` branch forces `dist = 0` whenever the L480 comparison is not true -- so a **`NaN` distance can never escape `c2GJK` at this call site**, and `!= 0.0f` is behaviourally indistinguishable from `> 0.0f` here. Confirmed by mutation testing (equivalent mutant `pred-ne0`, see `MUTATION.md`) | [x] |
| 61 | `c2CapsuletoCapsule` (L529) | `c2GJK(...) != 0.0f` | `return 0`, else `return 1` (same `use_radius = 1` note as row 60) | [x] |
| 62 | generic FFI boundary | out-of-range `C2_TYPE` int at *every* entry point that takes one (`c2MakeProxy`, `c2Collided`, `ptr_from_parts`, `omni_collide`, `c2GJK`) | see rows 1-4, 6-9; for `c2GJK` an invalid type leaves the (uninitialised in C) `c2Proxy` untouched → indeterminate, documented and skipped | [x] |
| 63 | generic FFI boundary | zero-length / negative-length arrays (`c2Support` with `count = 0`, `-1`, `INT_MIN`) | row 34 | [x] |
| 64 | generic FFI boundary | one step past every documented range: `C2_TYPE` = `3` (one past `C2_TYPE_AABB`) and `-1` (one before `C2_TYPE_CAPSULE`); `s->count` = `0` and `4`; `iter` cap `19`/`20`/`21` | rows 1-4, 9, 19, 28-30, 32 | [x] |

## Deliberately-not-asserted rows

Rows 5, 6, 27, 35 and the `c2GJK`-with-invalid-type part of row 62 are the only
rows without a *value* assertion, because the C behaviour there is genuinely
indeterminate:

| row | why the C is indeterminate |
|-----|-----------------------------|
| 5   | `c2Collided` dereferences `A`/`B` unconditionally once the type is valid, so a NULL pointer is an immediate SIGSEGV in both implementations |
| 6   | `ptr_from_parts` falls off the end of a non-`void` function; the returned value is whatever happens to be in `rax` |
| 27  | `cache->iA[i] >= pA.count` reads a `c2Proxy.verts[]` slot that `c2MakeProxy` never wrote, and the C's `c2Proxy` is an **uninitialised automatic** (the Rust zeroes it, which is a *different* indeterminate value) |
| 35  | `c2Support` with a `count` larger than the caller's array reads past its end |
| 62 (`c2GJK` part) | an out-of-range `C2_TYPE` leaves that same uninitialised `c2Proxy` untouched, after which `c2Support` loops over a garbage `pA.count` -- an out-of-bounds read that can and does segfault |

Each is documented in the table, and each has a test pinning down the
*observable* consequence where one exists:

* rows 7 and 8 pin down row 6's consequence through `omni_collide`;
* `err_row05_null_shape_pointers_documented` proves the type rejection happens
  *before* any dereference (a NULL pointer paired with an out-of-range type
  returns 0 rather than crashing);
* `err_row27_35_62_indeterminate_documented` asserts the *defined* half of rows
  27 and 35 (in-range cache indices, exact array lengths).

Row 27 was found the hard way. An early version of
`err_row19_iteration_cap_unreachable_but_iterations_agree` seeded the cache with
indices in `0..4` regardless of shape type, and C/Rust diverged
(C `dist = 0.9527172`, Rust `dist = 0.0`) purely because the C was reading
uninitialised stack where the Rust read zeros -- with a 1-vertex circle proxy
and `iA = [3,1,2]`. The test now clamps every index to the proxy's real vertex
count, and the divergence is recorded here as C-side UB rather than a
translation defect.

## NaN payloads

Three of the ~700k differential float comparisons differ in the NaN **sign bit**
only (`0xffc00000` vs `0x7fc00000`), and only where two or more operands are
already NaN. On x86-64, `mulss`/`addss` forward whichever source operand the
back end placed first, which the C semantics of `a*b` do not fix, so GCC -O0 and
LLVM legitimately disagree. `tests/common/mod.rs::f32_same` therefore treats any
two NaNs as equal; run the suite with `STRICT_NAN_BITS=1` to see the exact three
cases (tabulated in that function's doc comment). Everything else -- `+0.0` vs
`-0.0`, subnormals, `+-inf`, `+-FLT_MAX` -- is compared bit-exactly.

## C-side coverage (does the suite actually reach these branches?)

Passing tests prove agreement; they do not by themselves prove the C's rejection
branches were *entered*. So the suite was also run against a separately
compiled, instrumented copy of the C source:

```
cp c_src/src/lib.c c_src/include/lib.h  <scratch>/
gcc -O0 -fPIC --coverage -fprofile-update=atomic -I. -shared -o libcov.so lib.c -lm
C_SO=<scratch>/libcov.so cargo test --offline -- --test-threads=1
gcov -b libcov.so-lib.gcda
```

(`$C_SO` overrides the harness's `.so` search; `-fprofile-update=atomic` plus
`--test-threads=1` is required, because gcov's default non-atomic counters are
corrupted by parallel test threads and produce nonsense like
`call 0 returned -12003%`.)

Result — all 111 tests pass against this second, independently built C library,
and:

```
File 'lib.c'
Lines executed:100.00% of 445
Branches executed:100.00% of 157
Taken at least once:99.36% of 157
Calls executed:100.00% of 149

uncovered lines (#####):        0
never-executed branches:        0
branch arcs never taken:        1
```

The single untaken arc in the whole library is:

```
  1059652:  420:	while (iter < 20) {
branch  0 taken 100%
branch  1 taken 0% (fallthrough)
```

i.e. exactly **row 19** — the iteration cap — independently confirming the
structural-unreachability argument above. Every other rejection branch in this
table, including all six `c23` region tests, all four `c2AABBtoAABB` separating
axes, every `switch` `default:`, every null guard, and the fall-off-the-end path
at `lib.c:637` (executed 19,717 times), is entered by the suite.

## Test-adequacy evidence

`MUTATION.md` records a 123-mutant mutation-testing run against
`translation/src/lib.rs`: **112 killed, 11 provably-equivalent survivors**. Two
of the original survivors were genuine blind spots and are exactly why rows 13b
and 15b exist:

* `let cache_was_good = (*cache).count != 0;` -> `> 0` survived until row 13b
  (negative `cache->count`) was added;
* `min_metric < max_metric * 2.0f32` -> `<=` survived until row 15b (the exact
  equality boundary) was added.
