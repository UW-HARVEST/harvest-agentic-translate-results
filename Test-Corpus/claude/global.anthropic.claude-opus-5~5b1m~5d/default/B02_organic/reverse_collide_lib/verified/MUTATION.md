# MUTATION.md — sensitivity check of the differential suite

"All tests pass" is only meaningful if the tests *can* fail. `./mutation_check.sh`
injects one deliberate bug at a time into `src/lib.rs`, rebuilds the Rust `.so`
and runs the whole suite against the unmodified C `.so`. Every mutant must be
**killed** (at least one test must fail).

Result: **25 killed, 3 survived** — and all three survivors are provably
*equivalent* mutants (they cannot change observable behaviour for any input),
not blind spots.

## Killed mutants

| mutation | tests that failed |
|----------|-------------------|
| `c2Det2` sign flip | 11 |
| `c2Dot` operand swap | 19 |
| `c22`: `v <= 0` → `v < 0` | 3 |
| `c23`: `vAB <= 0` → `vAB < 0` | 1 |
| `c2Support`: `dot > dmax` → `dot >= dmax` | 10 |
| `c2GJK`: cache metric guard `metric < -1e8` → `>` | 2 |
| `c2GJK`: degenerate-direction epsilon guard weakened | 1 |
| `c2GJK`: `d1 > d0` → `d1 >= d0` | 1 |
| `c2GJK`: radius guard `dist > rA+rB` → `>=` | 2 |
| `c2CircletoCircle`: `d2 < r2` → `d2 <= r2` | 2 |
| `c2AABBtoAABB`: result negation | 2 |
| `c2Collided`: AABB×CIRCLE argument swap removed | 2 |
| `c2MakeProxy`: `default` arm zeroes the proxy instead of leaving it untouched | 2 |
| `c2MakeProxy`: capsule `count = 2` → `1` | 11 |
| `c2BBVerts`: vertex 1 order changed | 9 |
| `c2L`: `default` sentinel → `verts[0].p` | 2 |
| `c2Norm`: zero-vector special case added | 1 |
| `c2GJKSimplexMetric`: `case 2` label moved to `case 1` | 5 |
| `c2Witness`: `den` sign flip | 12 |
| `reverse_collide`: third term shifted `<< 2` → `<< 3` | 1 |
| `c2Clampv`: max/min order swapped | 3 |
| `c2Skew`: sign flip (turns it into `c2CCW90`) | 11 |
| `c2GJK`: `hit` branch also taken when `use_radius == 0` | 4 |
| `c2GJK`: duplicate-support-point `break` disabled | 12 |
| `c2GJK`: `cache_was_read` condition inverted | 14 |

## Surviving mutants — all equivalent, with proof

### 1. `c2Support`: loop start `i = 1` → `i = 0`

`dmax` is initialised to `c2Dot(verts[0], d)`. At `i == 0` the test is
`dot > dmax`, i.e. `dmax > dmax`, which is `false` for every value including
`NaN` (and for `±inf`, since `x > x` is false). `imax` therefore stays `0` and
the remainder of the loop is unchanged. The mutant computes the same result for
every possible input, so no test can distinguish it.

### 2. `c2GJK`: iteration cap `while (iter < 20)` → `while (iter < 19)`

Distinguishing the two requires an input that reaches `iter == 19`. Both proxies
hold at most 4 vertices (`AABB`), so the Minkowski-difference support set is
finite and tiny: the loop always terminates via `s.count == 3` (hit), the
`d1 > d0` guard, the degenerate-direction guard, or the duplicate-index guard
long before 19.

Measured with a dedicated probe (4,000,000 randomized calls covering all 9 type
pairs, random transforms, both `use_radius` values, cold/warm caches, and
non-finite coordinates), the observed distribution of `iterations` was:

```
iterations:  0        1        2       3      4    5   6+
count:    1192231   531981   221272  54307   203   6   0
```

Maximum reachable value: **5**. The cap of 20 is dead code for these three
shape kinds. `stress_gjk_cross_product_and_iteration_histogram` asserts the C
and Rust `iterations` histograms are identical **and** that nothing above 5 ever
appears, so if a future change ever makes higher counts reachable the assertion
fires and this analysis must be revisited.

### 3. `c2CircletoCapsule`: `if (da < 0)` → `if (da <= 0)`

The two differ only when `da == 0`, i.e. `dot(ap, n) == 0` with
`ap = A.p - B.a`, `n = B.b - B.a`.

* If `n != (0,0)`: then `db = dot(A.p - B.b, n) = dot(ap - n, n) = da - dot(n,n)
  = -dot(n,n) < 0`, so the original takes the projection branch and computes
  `e = ap - n * (da / dot(n,n)) = ap - n * 0 = ap`, giving `d2 = dot(ap, ap)` —
  exactly what the mutant's `da <= 0` branch computes.
* If `n == (0,0)`: then `B.a == B.b`, so `bp = A.p - B.b == ap` and the original's
  `bp` branch yields `d2 = dot(ap, ap)` — again identical.

Hence the mutant is behaviourally identical for every input.

## How to re-run

```
cd translation && ./mutation_check.sh
```

The script always restores `src/lib.rs` and rebuilds the release `.so` on exit.
