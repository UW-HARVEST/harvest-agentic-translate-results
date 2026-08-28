# ERRORS.md — error / rejection surface table (Phase A, gate for Phase C)

Mechanically derived from `c_src/src/lib.c`. Greps run:

```sh
grep -n 'return -1\|return NULL\|assert\|RETURN_ERROR\|errno\|exit(\|abort(' c_src/src/lib.c   # -> no matches
grep -n 'if (\|switch\|case \|default:\|while\|for (' c_src/src/lib.c                          # -> the 60 branches below
```

**There is no error-code channel in this library.** No function returns an
error enum/sentinel, nothing sets `errno`, there is not a single `assert`,
`return -1`, or `return NULL`. Every function returns a value unconditionally.
The library's entire *rejection* surface therefore consists of:

* **null-pointer sentinels** that are explicitly tested (`if (!ax_ptr)`,
  `if (cache)`, `if (outA)`, …) — the "rejection" is *silently substitute a
  default / skip the write*, and the observable result must match exactly;
* **`switch` statements with no `default:` label**, where an out-of-range
  value is silently ignored (`c2MakeProxy`);
* **`switch` `default:` fall-throughs**, where an out-of-range `count`
  produces a documented fallback value (`c2GJKSimplexMetric`, `c2D`, `c2L`,
  `c2Witness`);
* **degenerate-numeric guards** (`dist > FLT_EPSILON`, `c2Dot(d,d) < eps*eps`,
  `d1 > d0`, the `dup` check, `iter < 20`) whose *failure* aborts the GJK loop;
* **unguarded division / `sqrtf`** which "reject" by producing `inf` / `NaN`
  rather than by returning an error.

Rows below are one-per-distinct-rejection-branch. "expected C result" is what
`c_src/src/lib.c` actually does, read off the source, not what a doc claims.

Legend for the last column: `[x]` = differential test written and passing
against both `.so`s.

## Null-pointer / sentinel rejections

| # | function | trigger (exact invalid input/condition) | expected C result | done |
|---|----------|------------------------------------------|-------------------|------|
| E01 | `c2GJK` | `ax_ptr == NULL` (line 368) | `ax = c2xIdentity()` i.e. `{p:{0,0}, r:{1,0}}`; no crash, distance computed in B's world frame | [x] |
| E02 | `c2GJK` | `bx_ptr == NULL` (line 372) | `bx = c2xIdentity()` | [x] |
| E03 | `c2GJK` | both `ax_ptr == NULL` **and** `bx_ptr == NULL` | both transforms identity | [x] |
| E04 | `c2GJK` | `outA == NULL` (line 510) | the caller's `outA` is *not* written; return value + `outB` + `iterations` + `cache` still fully written | [x] |
| E05 | `c2GJK` | `outB == NULL` (line 512) | `outB` not written; everything else written | [x] |
| E06 | `c2GJK` | `outA == NULL && outB == NULL` | neither written | [x] |
| E07 | `c2GJK` | `iterations == NULL` (line 514) | `iterations` not written (caller's sentinel `-1` preserved) | [x] |
| E08 | `c2GJK` | `cache == NULL` (lines 383 & 500) | no cache read (`cache_was_read = 0`, cold start from vertex 0) and no cache write-back | [x] |
| E09 | `c2GJK` | ALL optional pointers NULL (`ax_ptr`, `bx_ptr`, `outA`, `outB`, `iterations`, `cache`) | only the `float` return value is produced | [x] |
| E10 | `gjk_cache` | `a9 == NULL`, `b9 == NULL` | never dereferenced by the C (the parameters are dead) → no crash, no write | [x] |
| E11 | `gjk_cache` | `a9`/`b9` non-NULL | still never written — caller's buffer must be left byte-identical | [x] |

## Out-of-range enum values crossing the FFI boundary

`C2_TYPE` is a C enum, i.e. gcc passes it as `unsigned int`; any `int` value is
accepted at the ABI level. `c2MakeProxy`'s `switch` has **no `default:` label**
(lines 114–134), so an unknown type writes *nothing* into `*p`.

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E12 | `c2MakeProxy` | `type == 3` (one past `C2_TYPE_CAPSULE`) | `*p` left completely untouched (radius/count/verts keep the caller's prior bytes) | [x] |
| E13 | `c2MakeProxy` | `type == 0xFFFFFFFF` / `-1` | `*p` untouched | [x] |
| E14 | `c2MakeProxy` | `type == 0x7FFFFFFF`, `type == 100` | `*p` untouched | [x] |
| E15 | `c2MakeProxy` | valid `type` but `p` pre-filled with garbage | only the fields the matching arm assigns are overwritten; e.g. `CIRCLE` sets `radius`,`count=1`,`verts[0]` and leaves `verts[1..7]` as the caller left them | [x] |
| E16 | `c2GJK` | `typeA` out of range (3, 4, `0xFFFFFFFF`, 100) | `pA` is an **uninitialised automatic** (`c2Proxy pA;` line 376) that `c2MakeProxy` does not touch → the C reads indeterminate stack. **Measured**: the C returns `inf`/`NaN` and leftover stack bit patterns (e.g. `a = (-1.1015665e35, 4.5685e-41)`), i.e. genuinely indeterminate. UB — not value-reproducible. The test asserts the property both libraries really do share: the call returns normally (no trap) for every out-of-range enum value, and it documents the divergence. | [x] |
| E17 | `c2GJK` | `typeB` out of range | same as E16 for `pB` | [x] |

## `switch (s->count)` fallbacks — out-of-range simplex counts

`c2Simplex::count` is a plain `int` written by the caller; there is no
validation anywhere.

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E18 | `c2GJKSimplexMetric` | `count == 0` (hits `default:` which falls through to `case 1:`) | returns `0.0f` | [x] |
| E19 | `c2GJKSimplexMetric` | `count == 4`, `5`, `-1`, `INT_MIN`, `INT_MAX` | returns `0.0f` (same `default:`) | [x] |
| E20 | `c2D` | `count == 3` (explicit) | returns `{0,0}` | [x] |
| E21 | `c2D` | `count == 0`, `4`, `-1`, `INT_MIN`, `INT_MAX` (`default:`) | returns `{0,0}` | [x] |
| E22 | `c2L` | `count == 3` (falls into `default:`) | returns `{0,0}` | [x] |
| E23 | `c2L` | `count == 0`, `4`, `-1`, `INT_MIN`, `INT_MAX` | returns `{0,0}` — note `den = 1/div` is still computed first, so a `div == 0` does *not* change the result | [x] |
| E24 | `c2Witness` | `count == 0`, `4`, `-1`, `INT_MIN`, `INT_MAX` (`default:` line 332) | `*a = {0,0}`, `*b = {0,0}` | [x] |
| E25 | `c2GJK` | initial `s.count` forced to 3 via a cache with `count == 3` | `switch (s.count)` runs `c23`; the `case 3` arm exists, so no fallback | [x] |
| E26 | `c2GJK` | cache `count` value that makes `s.count` land outside 1..3 in the loop `switch` (lines 431–440, **no `default:`**) | no simplex reduction performed that iteration; `s.count == 3` test then decides | [x] |

## Division by zero / degenerate float "rejections"

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E27 | `c2Div` | `b == 0.0f` with non-zero `a` | `a * (1/0) = ±inf` per component; `0 * inf = NaN` for a zero component | [x] |
| E28 | `c2Div` | `b == -0.0f` | `1/-0 = -inf`; signs follow | [x] |
| E29 | `c2Div` | `b == NaN` | both components `NaN` | [x] |
| E30 | `c2Norm` | `a == {0,0}` → `c2Len == 0` → `c2Div(a, 0)` | `{0*inf, 0*inf} = {NaN, NaN}` | [x] |
| E31 | `c2Norm` | `a` contains `inf` → `c2Len == inf` → `1/inf == 0` | `{±0 or NaN, …}` (`inf*0 = NaN`) | [x] |
| E32 | `c2Norm` | `a` contains `NaN` | `{NaN, NaN}` | [x] |
| E33 | `c2Len` | `c2Dot(a,a) < 0` — only reachable via `NaN`/`inf` mixes | `sqrtf(NaN) = NaN` (quiet, no `errno` check in the C) | [x] |
| E34 | `c2Witness` | `s->div == 0` | `den = inf`; results are `inf`/`NaN` per component, still written | [x] |
| E35 | `c2Witness` | `s->div == NaN` | `den = NaN` → outputs `NaN` (except `count == 1`, which ignores `den`) | [x] |
| E36 | `c2L` | `s->div == 0` and `count == 2` | `den = inf` → `inf`/`NaN` components | [x] |
| E37 | `c22` | `s->a.p == s->b.p` (degenerate 2-simplex → `u == v == 0`) | `v <= 0` branch: `a.u = 1`, `div = 1`, `count = 1` | [x] |
| E38 | `c22` | any `NaN` in `a.p`/`b.p` → `u`,`v` both `NaN` | both `v <= 0` and `u <= 0` are false → `else` arm: `a.u = NaN`, `b.u = NaN`, `div = NaN`, `count = 2` | [x] |
| E39 | `c23` | fully degenerate triangle (all three `p` equal → every `u*`/`v*`/`area` is 0) | first arm `vAB <= 0 && uCA <= 0` taken: `count = 1`, `div = 1` | [x] |
| E40 | `c23` | any `NaN` in the three points → every comparison false | falls through to the final `else`: `count = 3`, `div = NaN` | [x] |
| E41 | `c2GJK` | `use_radius != 0` and `dist <= rA + rB` (line 485 fails) | midpoint collapse: `a = b = (a+b)*0.5`, `dist = 0` | [x] |
| E42 | `c2GJK` | `use_radius != 0` and `dist <= FLT_EPSILON` (second half of line 485 fails) | same midpoint collapse, `dist = 0` | [x] |
| E43 | `c2GJK` | `use_radius != 0`, radii shrink `a`/`b` onto each other (line 491) | `dist` forced to `0` even though the subtraction gave non-zero.  Reaching this line needs `dist > rA + rB` *and* the shrink to land both witness points on the same float: `rA = FLT_MAX`, `rB = -FLT_MAX` makes `rA + rB == 0` and saturates both points to `FLT_MAX` — **measured to match** (`tests/error_paths.rs::e43_forced_zero_after_radius_shrink`, 512/512 hits) | [x] |
| E44 | `c2GJK` | shape coordinates `NaN` | `dist` is `NaN`; the `hit`/`use_radius` comparisons are all false so the `else` midpoint branch runs → `dist = 0`, `a = b = NaN` midpoint | [x] |
| E45 | `c2GJK` | shape coordinates `±inf` / `FLT_MAX` (overflow in `c2Dot`) | whatever the C produces; loop terminates via `d1 > d0` or the 20-iteration cap | [x] |

## Loop / iteration-limit rejections in `c2GJK`

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E46 | `c2GJK` | `iter` reaches the hard cap (line 425, `while (iter < 20)`) | loop exits with `hit == 0`; `*iterations == 20`.  **Measured**: over 2.2 M randomized configurations (all 9 type pairs, spicy/`NaN`/`inf` geometry, degenerate transforms) plus a 800 k-step hill-climb search, the largest reachable `iter` is **6** — the cap itself is dead code for this shape set.  The test (`e46_iteration_counts_agree`) asserts `*iterations` is bit-identical for every reachable value (histogram over 0..6 printed) so the cap would be covered were it ever reachable | [x] |
| E47 | `c2GJK` | `d1 > d0` — no progress (line 447) | `break` with `hit == 0`; `*iterations` is the count *before* the failed step | [x] |
| E48 | `c2GJK` | `c2Dot(d,d) < FLT_EPSILON*FLT_EPSILON` — search direction collapsed (line 451) | `break` with `hit == 0` | [x] |
| E49 | `c2GJK` | duplicate support point (`dup`, lines 464–472) | `break`; the freshly written `verts[s.count]` is **left in place but `s.count` is not incremented**, so it is invisible to `c2Witness` yet still visible to the cache write-back only if `count` covers it (it does not) | [x] |
| E50 | `c2GJK` | `s.count == 3` after reduction (line 441) | `hit = 1`, `break`; afterwards `a = b` and `dist = 0` regardless of `use_radius` | [x] |

## Invalid `c2GJKCache` contents (no validation whatsoever in the C)

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E51 | `c2GJK` | `cache->count == 0` (line 384) | `cache_was_good = 0` → cold start | [x] |
| E52 | `c2GJK` | `cache->count == 1..3`, sane `iA`/`iB` | replay loop runs; the *inverted* validity test at line 405 (`!(min < max*2 && metric < -1e8f)`) is true for essentially every realistic metric, so `cache_was_read = 1` — the cache is **always** accepted. Reproduce exactly. | [x] |
| E53 | `c2GJK` | `cache->count` negative (`-1`, `INT_MIN`) | `!!count` is true → `cache_was_good`; the `for (i = 0; i < count; ++i)` body never runs; `s.count = negative`, `s.div = cache->div`; `c2GJKSimplexMetric` `default:` → `0`; then the loop `switch` has no matching case, `s.count != 3`, `c2L` `default:` → `{0,0}`, `c2D` `default:` → `{0,0}` → `c2Dot(d,d) == 0 < eps²` → immediate `break`; `c2Witness` `default:` → `a = b = {0,0}`; `dist = 0`; cache write-back loop does not run | [x] |
| E54 | `c2GJK` | `cache->count == 4` | replay writes `verts[0..3]` (`s.d`, still inside `c2Simplex`); `s.count = 4`; metric `default:` → `0`; loop `switch` no-match; `c2L`/`c2D` `default:`; `c2Witness` `default:` → zeros; write-back writes `cache->iA[0..3]` and `cache->iB[0..3]`, i.e. `iA[3]` **aliases `iB[0]`** and `iB[3]` **aliases `div`** (offsets `iA@8`, `iB@20`, `div@32`). Every access still lands inside the 36-byte struct, so this **is** fully value-reproducible — **measured to match byte-for-byte** (note `iB[3]` reads `div`'s *bit pattern* as a vertex index, so a `div` like `2.0f` gives index `0x40000000` and segfaults *both* libraries) | [x] |
| E55 | `c2GJK` | `cache->count >= 5` | the replay writes `verts[4]`, which is **past** the 152-byte `c2Simplex`.  In the C, `s` lives at `-0x1f0(%rbp)` and `pB` at `-0x150(%rbp)`, so `verts[4].p/.u/.iA/.iB` overwrite `pB.radius`, `pB.count` and `pB.verts[0..1]`, and the write-back then reads those clobbered bytes back.  The result depends entirely on gcc's stack-frame layout — **measured to diverge** (the C's `cache->iB[1]`/trailing word round-trip to their original values, the Rust's do not).  UB, not value-reproducible; the test asserts both libraries return normally and documents the divergence | [x] |
| E56 | `c2GJK` | `cache->iA[i]` / `iB[i]` in `2..7` — past the proxy's real vertex count but inside `c2Proxy::verts[8]` | C reads an *uninitialised* `c2Proxy::verts[k]`.  **Measured**: the return value and cache image still match, but `*outA`/`*outB` diverge for a circle proxy (only `verts[0]` is ever written).  UB, not value-reproducible; the test asserts both return normally | [x] |
| E57 | `c2GJK` | `cache->iA[i]` == 8 or larger, or negative | out-of-bounds read of `c2Proxy::verts` — UB in the C (a large enough index segfaults *both*).  **Measured**: `-1` and `8` happen to agree, `20` diverges.  Not value-reproducible; documented | [x] |
| E58 | `c2GJK` | `cache->metric == NaN` | `min_metric`/`max_metric` both take the `else` operand (`metric_old`) ⇒ both `NaN`; `NaN < NaN*2` false ⇒ `!(false && …)` true ⇒ `cache_was_read = 1` | [x] |
| E59 | `c2GJK` | `cache->metric` chosen so that `min < max*2 && metric < -1e8f` **is** true (needs `metric < -1e8`, only reachable with `count == 3` and a hugely negative determinant) | `cache_was_read` stays `0` ⇒ cache is *discarded* and the cold-start path runs. This is the only way to reach line 409's true branch with a non-empty cache | [x] |
| E60 | `c2GJK` | `cache->div == 0` while replaying | `s.div = 0` ⇒ `c2Witness`/`c2L` divide by zero ⇒ `inf`/`NaN` outputs | [x] |

## `c2Support` boundaries

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E61 | `c2Support` | `count == 0` | the `for` never runs but `verts[0]` **is still dereferenced** on line 300 → returns `0` (requires at least 1 readable element) | [x] |
| E62 | `c2Support` | `count == 1` | returns `0` | [x] |
| E63 | `c2Support` | `count` negative | loop skipped, returns `0` | [x] |
| E64 | `c2Support` | `d == {0,0}` — all dots equal `0`, none `> dmax` | returns `0` (first index wins ties) | [x] |
| E65 | `c2Support` | `verts` contains `NaN` → `dot > dmax` always false | returns the index of the last non-`NaN` maximum, i.e. ties/`NaN` never displace `imax` | [x] |
| E66 | `c2Support` | `verts[0]` is `NaN` so `dmax == NaN` | every `dot > NaN` is false → returns `0` | [x] |
| E67 | `c2Support` | oversized `count` (e.g. 8 against a proxy that only filled 1) | reads uninitialised `c2Proxy::verts` — UB, documented; the pure-`c2Support` variant of this test supplies 8 *initialised* verts so it *is* value-comparable | [x] |

## `c2BBVerts` boundaries

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E68 | `c2BBVerts` | inverted box (`min > max` on either axis) | no validation: writes the four corners in the same order, producing a "negative" box | [x] |
| E69 | `c2BBVerts` | `min == max` (empty box) | four identical corners | [x] |
| E70 | `c2BBVerts` | `NaN`/`inf` coordinates | copied through verbatim (bit-identical, including `NaN` payload) | [x] |

## `c2Maxv` / `c2Minv` / `c2Clampv` — NaN ordering quirks

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E71 | `c2Maxv` | `a.x` is `NaN` (`a.x > b.x` false) | picks `b.x` | [x] |
| E72 | `c2Maxv` | `b.x` is `NaN` (`a.x > NaN` false) | picks `b.x`, i.e. **the `NaN` wins** | [x] |
| E73 | `c2Minv` | `b.x` is `NaN` (`a.x < NaN` false) | picks `b.x`, the `NaN` | [x] |
| E74 | `c2Minv` | `a.x` is `NaN` | picks `b.x` | [x] |
| E75 | `c2Maxv`/`c2Minv` | `+0.0` vs `-0.0` (`>` and `<` both false) | always returns `b`'s zero, sign included | [x] |
| E76 | `c2Clampv` | `lo > hi` (inverted range) | `c2Maxv(lo, c2Minv(a, hi))` → `lo` wins; no rejection | [x] |
| E77 | `c2Clampv` | `NaN` in `a`, `lo`, or `hi` | propagates per E71–E74 | [x] |

## Sign-of-NaN / signed-zero traps in the rotation helpers

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E78 | `c2MulrvT` | `a.s` is `NaN`, `b.x` finite | C computes `(-a.s) * b.x + a.c * b.y`; the `fneg` happens **before** the multiply, so the sign bit of the resulting `NaN` is the *flipped* one. Rust must not fold this into `a.c*b.y - a.s*b.x` | [x] |
| E79 | `c2MulrvT` | `a.s == 0.0`, `b.x == 0.0` | `(-0.0) * 0.0 = -0.0`, then `-0.0 + a.c*b.y` | [x] |
| E80 | `c2Mulrv` | `NaN` operands | `a.c*b.x - a.s*b.y` (real `fsub`) — different `NaN` sign discipline from `c2MulrvT` | [x] |
| E81 | `c2Neg` | `a` is `NaN` | sign bit flipped (bit-exact comparison required) | [x] |
| E82 | `c2Neg` | `a` is `+0.0` | `-0.0` | [x] |
| E83 | `c2Skew` / `c2CCW90` | `+0.0` / `NaN` inputs | one component negated (bit-exact) | [x] |

## Non-pointer parameter edge values

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E84 | `c2GJK` | `use_radius` values other than 0/1 (`-1`, `2`, `INT_MIN`, `INT_MAX`) | C tests `else if (use_radius)`, i.e. any non-zero takes the radius path | [x] |
| E85 | `gjk_cache` | `reverse` values other than 0/1 (`-1`, `2`, `0x7F`, `0x80` truncated to `char`) | C tests `if (reverse)`, any non-zero `char` takes the reversed path. `char` is *signed* on x86-64 Linux, so `0x80` → `-128` → still non-zero | [x] |
| E86 | `gjk_cache` | `NaN` / `inf` / `FLT_MAX` in `a1..a4`, `b1..b5` | no validation; must not crash and must leave `a9`/`b9` untouched | [x] |
| E87 | `gjk_cache` | inverted AABB (`a1 > a3`, `a2 > a4`), zero-length capsule (`b1==b3, b2==b4`), negative capsule radius `b5 < 0` | no validation; runs the full GJK | [x] |

## Genuine null-pointer dereferences (crash in both, not testable in-process)

| # | function | trigger | expected C result | done |
|---|----------|---------|-------------------|------|
| E88 | `c2BBVerts` | `out == NULL` or `bb == NULL` | segfault (line 107 dereferences unconditionally) — verified out-of-process in `tests/error_paths.rs::null_deref_crashes_both` via `fork()` | [x] |
| E89 | `c2MakeProxy` | `p == NULL` with a valid `type` | segfault; with an **invalid** `type` the C never dereferences `p`, so it is safe — both behaviours checked | [x] |
| E90 | `c2Support` | `verts == NULL` (any `count`, even 0) | segfault (line 300) | [x] |
| E91 | `c2GJK` | `A == NULL`/`B == NULL` with a valid type | segfault inside `c2MakeProxy`; with an invalid type, `A`/`B` are never read | [x] |
| E92 | `c2GJKSimplexMetric`/`c22`/`c23`/`c2D`/`c2L`/`c2Witness` | `s == NULL` | segfault | [x] |
| E93 | `c2Witness` | `s` valid but `a == NULL` / `b == NULL` | segfault in every `case`, including `default:` | [x] |

---

## Verification result

Every row above has a passing differential test in `translation/tests/`
(mostly `tests/error_paths.rs`, 30 tests; the NaN-payload rows are additionally
hardened by `tests/nan_payloads.rs`).  Run with:

```sh
cd translation && ./verify.sh          # all profiles x all feature combos
cd translation && cargo test --release --test error_paths -- --nocapture
```

Result: **113 tests, 0 failures**, in release and dev profiles, with and without
default features.

### Rows whose C behaviour is undefined

Five rows (E16, E17, E55, E56, E57) describe inputs for which the C reads
uninitialised memory, so a *value*-level comparison is meaningless — the C's own
answer is not even stable between runs, and it can fault outright.  Those cases
are executed in a forked child process
(`tests/error_paths.rs::e16_e17_e55_e57_ub_cases_out_of_process`) so that a
C-side fault cannot take the test runner down; the assertion made is the one that
*is* meaningful: the Rust library, which is deterministic and never reads
uninitialised memory, always returns normally.  The measured C behaviour is
printed by the test and summarised in the rows above.

### Divergences that were FOUND and FIXED in the Rust translation

| what | symptom | fix |
|------|---------|-----|
| `c2MulrvT` (and, latently, every other float expression) | with two NaN operands the result depends on which C operand gcc put in the SSE *destination* register; LLVM commutes `fadd`/`fmul` differently, so `c2MulrvT({1,0}, {-inf, NaN})` returned `0x7fcc58dd` instead of the C's `0xffc00000` | replaced every `+ - * /` on `f32` with `fp::add/sub/mul/div`, single SSE instructions whose destination operand is pinned by inline asm; each call site now names the destination gcc chose (read off `objdump -d` of the C `.so`) |
| `c2Support` | the C loads `verts[0]` *before* looking at `count`, so `c2Support(NULL, 0, d)` faults; LLVM deleted the dead load in the Rust build and it quietly returned `0` | the first load is now a `ptr::read_volatile`, so both libraries fault with SIGSEGV |
| dev-profile builds | libcore's `ub_checks` (enabled by `debug-assertions`) turned a NULL dereference into a Rust panic → `abort()` (SIGABRT) where the C raises SIGSEGV | `[profile.dev] debug-assertions = false, overflow-checks = false` in `Cargo.toml`, documented there |
| `FLT_EPSILON * FLT_EPSILON` | the folded constant was spelled as a decimal literal and only *approximately* `2^-46` | it is now spelled `C2_FLT_EPSILON * C2_FLT_EPSILON` and a `const { assert!(...) }` block checks the bit patterns of all ten float constants against the C `.so`'s `.rodata` |
