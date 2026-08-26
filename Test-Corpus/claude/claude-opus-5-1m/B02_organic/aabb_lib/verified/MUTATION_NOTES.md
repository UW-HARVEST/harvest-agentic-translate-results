# MUTATION_NOTES.md — evidence that the differential suite actually observes the code

Passing tests only prove something if a *wrong* Rust implementation would make
them fail. `./mutation_check.sh` injects 31 deliberate divergences into
`src/lib.rs`, one at a time, rebuilds and re-runs the whole suite, and asserts
that the suite **fails**.

```
$ ./mutation_check.sh
killed    c2GJK: d1 > d0  ->  d1 >= d0
killed    c2GJK: eps break < -> <=
SURVIVED  c2GJK: iter < 20 -> iter < 19
killed    c2GJK: use_radius != 0 -> == 1
killed    c2GJK: dist > rA+rB -> >=
killed    c2GJK: cache metric -1.0e8 -> +1.0e8
killed    c2GJK: drop the a=b on hit
killed    c22: v <= 0 -> v < 0
killed    c22: u <= 0 -> u < 0
killed    c23: reorder branch 4/5 test
killed    c2D: det > 0 -> det >= 0
killed    c2Support: dot > dmax -> >=
SURVIVED  c2Support: start i at 0
killed    c2Skew: -a.y -> 0.0 - a.y (loses -0.0)
killed    c2Neg: -a.x -> 0.0 - a.x (loses -0.0)
killed    c2Maxv: > -> >= (NaN / -0.0 tie)
killed    c2Minv: < -> <= (NaN / -0.0 tie)
killed    c2MakeProxy: invalid enum writes radius
killed    c2CircletoCircle: d2 < r2 -> <=
killed    c2CircletoAABB: d2 < r2 -> <=
SURVIVED  c2CircletoCapsule: da < 0 -> da <= 0
killed    c2CircletoCapsule: db < 0 -> db <= 0
killed    c2AABBtoAABB: drop the -X separating axis
killed    c2AABBtoAABB: drop the +Y separating axis
killed    c2Collided: bad typeB returns 1 (circle arm)
killed    c2Collided: outer bad typeA returns 1
killed    c2Witness: bad count -> (1,1) instead of (0,0)
killed    c2L: default arm returns a.p
killed    c2GJKSimplexMetric: swap the count 2/3 arms
killed    c2Dot: + -> -
killed    aabb: shift the capsule term by 3 not 2

MUTATION CHECK: 3 mutant(s) SURVIVED
```

**28 / 31 killed.** The three survivors are *semantically equivalent* mutants —
they cannot change the observable behaviour of the library for any input, so no
test could possibly kill them. Each one is justified below and, where a boundary
exists at all, that boundary now has its own test.

### 1. `while iter < 20` → `while iter < 19`

The GJK loop can never run 19 times. `c2Proxy` holds at most 4 vertices, so
there are at most 16 distinct `(iA, iB)` support pairs, and the
duplicate-support break (`lib.c:464`) or the `d1 > d0` break (`lib.c:446`) always
fires long before. Measured by `e35_gjk_iteration_cap`, which sweeps 108 000
queries over all 9 type pairs with arbitrary bit-pattern coordinates
(`NaN`, `±inf`, denormals, `FLT_MAX`):

```
e35: max iterations observed = 5; histogram = [49054, 31251, 22186, 5391, 111, 7, 0, ...]
```

The maximum is **5**, so every bound in `6..=20` is behaviourally identical.
`ERRORS.md` row 35 records this, and the guard is verified instead by asserting
that C and Rust return the *same* `*iterations` for all 108 000 cases.

*(Corollary: because the cap is unreachable, the C never reads the
never-initialised `u` field of the vertex it appended last — the only
uninitialised read the normal `c2GJK` path could have performed.)*

### 2. `c2Support`: loop starting index `1` → `0`

`dmax` is initialised to `c2Dot(verts[0], d)`, so an extra `i == 0` iteration
evaluates `dot > dmax` with `dot` bit-identical to `dmax`. `x > x` is false for
every float — including `NaN` — so the extra iteration can never update `imax`.
Covered by `e19_boundary_support_first_vertex_never_beats_itself`, which asserts
the invariant directly over 50 000 random `(v0, d)` pairs (all float classes) and
checks `c2Support` on all-identical vertex arrays for counts 1/2/4/8.

### 3. `c2CircletoCapsule`: `da < 0` → `da <= 0`

The two variants can only diverge when `da == ±0`, and in that case they compute
the *same* `d2`:

* if `db < 0`, the C takes the segment-interior branch with
  `da / c2Dot(n,n) == ±0`, so `c2Mulvs(n, ±0)` is `(±0, ±0)` and
  `e == ap` in value ⇒ `d2 == c2Dot(ap, ap)` — exactly what the mutant computes;
* if `db >= 0`, then `db == da - |n|² >= 0` with `da == 0` forces `|n|² == 0`,
  i.e. a degenerate capsule with `a == b`, so `bp == ap` and again
  `d2 == c2Dot(ap, ap)`.

This is not left as an argument: `e60_proof_da_zero_branches_coincide`
recomputes *both* candidate `d2` values **using the C library's own exported
primitives** (`c2Dot`, `c2Sub`, `c2Mulvs`) and asserts they are bit-equal:

```
e60 proof: 316000 inputs with da == ±0 checked
           (228357 via the segment-interior branch, 87643 via endpoint b)
           — the `da < 0` / `da <= 0` variants never disagreed
```

`e60_boundary_da_exactly_zero` additionally drives the real
`c2CircletoCapsule` differentially on that boundary (axis-aligned and arbitrary
orientations, `len` from `FLT_MIN_POSITIVE` to `FLT_MAX`, degenerate capsules).

---

## Side finding: `cargo test` alone can silently test a stale library

The crate is `crate-type = ["cdylib"]` and the tests reach it only through
`libloading`, so **cargo has no dependency edge from any test target to the
library** and `cargo test` does *not* rebuild it. Worse, cargo's fingerprinting
is mtime-based, so restoring a file with `mv`/`cp -p`/`git checkout` (which can
move the mtime *backwards*) leaves cargo convinced a stale artifact is current.

This was hit for real while writing `mutation_check.sh`: the first run reported
all 31 mutants as "SURVIVED" because every test ran against a `.so` built before
the first mutation.

Two guards now make that impossible:

1. `tests/common/mod.rs::rebuild_cdylib()` forces `src/lib.rs`'s mtime forward
   and runs `cargo build --offline` before the first `dlopen`, so the loaded
   `.so` always corresponds to the source on disk. (`HARNESS_NO_REBUILD=1`
   opts out.)
2. `tests/common/mod.rs::assert_fresh()` then hard-fails if the `.so` is still
   older than `src/lib.rs` or `Cargo.toml`.

`run_verification.sh` also builds explicitly before every `cargo test`.
