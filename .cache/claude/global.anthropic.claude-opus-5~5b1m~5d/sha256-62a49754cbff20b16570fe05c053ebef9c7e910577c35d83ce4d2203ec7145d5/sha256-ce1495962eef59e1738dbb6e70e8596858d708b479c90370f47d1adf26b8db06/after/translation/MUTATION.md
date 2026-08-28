# MUTATION.md — test-adequacy evidence

Passing differential tests only prove *something* was compared. To show the
suite would actually **catch** a mistranslation, `translation/src/lib.rs` was
mutated 123 times (one seeded defect at a time, literal source substitution),
rebuilt in both the `debug` and `release` profiles, and the whole suite re-run.

```
KILLED   112
SURVIVED  11   (all provably equivalent — analysed below)
```

A mutant counts as KILLED if any test fails **or** a test binary crashes (an
inverted NULL check, for example, kills by SIGSEGV rather than by an assertion).

The mutation list covers every function in the file: sign flips, comparison
relaxations (`<` → `<=`, `>` → `>=`), operand swaps, argument-order swaps,
constant perturbations, off-by-one loop bounds, dropped negations, `switch`
`default:` value changes, transposed struct-field assignments, and the
`use_radius` / `!= 0.0` flags of the GJK-backed predicates.

## The 11 survivors, and why each is an equivalent mutant

| mutant | change | why it cannot be observed |
|--------|--------|----------------------------|
| `c2Len-abs` | `c2Dot(a,a).sqrt()` → `c2Dot(a,a).abs().sqrt()` | `c2Dot(a,a) = x*x + y*y`; `x*x` is never negative for any `f32` (including `-0.0`, subnormals, `±inf`, `NaN`), and `NaN.abs()` is still `NaN`. `abs()` is a no-op here. |
| `c2Norm-plus0` | `c2Div(a, c2Len(a))` → `c2Div(a, c2Len(a) + 0.0)` | `x + 0.0` is the identity for every `f32` except `-0.0 → +0.0`. `c2Len` returns `sqrtf(x*x + y*y)`, and `x*x + y*y` can never be `-0.0` (`(-0.0)*(-0.0) = +0.0`, `+0.0 + +0.0 = +0.0`), so the only distinguishing input is unreachable. |
| `c2Support-init` | `for i = 1` → `for i = 0` | The extra iteration compares `verts[0]` against itself: `dot > dmax` with `dot == dmax` is false, and `NaN > NaN` is also false, so `imax` never changes. |
| `c23-b3b` | `uAB > 0.0 && …` → `uAB >= 0.0 && …` (edge-AB region) | Requires `uAB == 0`, `vBC > 0`, `vAB > 0`, `wABC <= 0` simultaneously. Put `e = b - a`; `uAB = dot(b, e) = 0` means `b ⊥ e`, so WLOG `e = (0,h)`, `b = (p,0)`, `a = (p,-h)`. Then `wABC = det2(a,b) · det2(e, c-a) = (h·p)·(-h(c.x-p)) = -h²·p·(c.x-p)`, so `wABC <= 0` ⟺ `p(c.x-p) >= 0`; while `vBC = dot(b, b-c) = p(p-c.x) > 0` ⟺ `p(c.x-p) < 0`. Contradiction — the combination is algebraically impossible. |
| `cap-da` | `if da < 0.0` → `if da <= 0.0` (`c2CircletoCapsule`) | The branches differ only when `da == 0`. `db = dot(A.p - B.b, n) = da - dot(n,n) = -dot(n,n)`. If `n ≠ 0` then `db < 0`, so both variants take the projection branch and `da/dot(n,n) = 0` makes `e = ap` either way. If `n == 0` (degenerate capsule `a == b`) then `db == 0` and the `bp` branch gives `d2 = |A.p - B.b|² = |A.p - B.a|² = |ap|²`, which is what the mutant computes. Identical result in both cases. |
| `pred-ne0` | `c2GJK(…) != 0.0` → `> 0.0` in `c2AABBtoCapsule` | The predicate hard-codes `use_radius = 1`. L480/L488 then guarantee `dist` is either exactly `0` or strictly positive and finite: `dist > rA+rB` is false for any `NaN` (→ `else` → `dist = 0`), and when true the subtraction leaves `dist > 0`. So `dist` is never `NaN` and never negative at this call site, making `!= 0.0` and `> 0.0` the same test. |
| `omni-order` | `c2Collided(A, type_a, B, type_b)` → `c2Collided(B, type_b, A, type_a)` | `c2Collided`'s dispatch table is *symmetric by construction*: each mixed pair routes to the same predicate with the same argument roles (`(CIRCLE,AABB)` → `c2CircletoAABB(A,B)` and `(AABB,CIRCLE)` → `c2CircletoAABB(B,A)`), and each same-type predicate is itself symmetric (`|B.p-A.p|² == |A.p-B.p|²`; the AABB separating-axis OR is symmetric). Swapping the arguments is therefore a no-op for the boolean result. |
| `gjk-count3` | `if s.count == 3` → `>= 3` | After the `switch`, `s.count ∈ {1,2,3}` on every defined path. The only way to reach `s.count > 3` is `cache->count >= 4`, which makes the C write `saveA[3]` past the end of its `int saveA[3]` — undefined behaviour, so there is no C result to match. |
| `gjk-cache-u` | `(*v).u = 0.0` → `1.0` in the cache-reload loop | Every `u` is overwritten by `c22`/`c23` before anything reads it. `count == 1` reads no `u` at all (`c2L` case 1 returns `p`; `c2Witness` case 1 returns `sA`/`sB`); `count == 2`/`3` run a solver that assigns every `u` it will later use. |
| `gjk-fresh-u` | `s.verts[0].u = 1.0f32` → `2.0f32` in the fresh init | Same argument: with `count == 1` no `u` is read, and by the time `count >= 2` the solver has reassigned it. |
| `gjk-iter-cap` / `gjk-iter-cap2` | `while iter < 20` → `< 19` / `< 21` | The cap is dead code for this shape set (max 4 proxy vertices). See `ERRORS.md` row 19: 2,000,000 randomized calls observe a maximum of 5 iterations. |

## The two survivors that were NOT equivalent (fixed)

These two initially survived and represented **real gaps** in the test suite.
Both are now killed:

| mutant | change | gap it exposed | test added |
|--------|--------|----------------|------------|
| `gjk-cache-good` | `(*cache).count != 0` → `> 0` | A **negative** `cache->count` is a fully-defined C path (every count-bounded loop runs zero times, every `switch` hits `default:`) and nothing tested it. | `err_row13b_negative_cache_count` (`ERRORS.md` row 13b) |
| `gjk-metric-le` | `min_metric < max_metric * 2.0f32` → `<=` | The `<` in the L400 cache-validity test needs *exact* equality plus `metric < -1e8` plus an observable downstream difference; no random input can produce it. | `err_row15b_cache_metric_equality_boundary` (`ERRORS.md` row 15b) |

Verification after adding those two tests:

```
gjk-cache-good      KILLED
gjk-writeback-count KILLED   (same negative-count path)
gjk-cache-negcount  KILLED   (`!= 0` -> `>= 0`, i.e. count==0 treated as warm)
gjk-metric-le       KILLED
```

## Reproducing

The runner is `work/mut.py` (a scratch script, not part of the crate): it
substitutes one literal snippet in `src/lib.rs`, rebuilds `debug` + `release`,
runs `cargo test --offline`, classifies the mutant, and restores the original.
`work/lib.rs.bak` holds the pristine source. The final state of
`translation/src/lib.rs` is byte-identical to the pre-mutation original — the
mutation run leaves no residue (checked with `diff`).
