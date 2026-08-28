# CONFIGS.md — configuration surface table (Phase A, gate for Phase B)

Mechanically derived from `c_src/src/lib.c` + `c_src/include/lib.h`. The axes
below are exactly the things the C `if`/`switch` statements branch on. There is
no `#ifdef` in the source and no `[features]` table in `Cargo.toml`, so the
compile-time axis is a single point.

## Axes the C actually branches on

| axis | values the C distinguishes | source |
|------|----------------------------|--------|
| `typeA` / `typeB` | `C2_TYPE_CIRCLE` (1 vert, radius `r`), `C2_TYPE_AABB` (4 verts, radius 0), `C2_TYPE_CAPSULE` (2 verts, radius `r`) | `c2MakeProxy` lines 114–134 |
| `ax_ptr` / `bx_ptr` | `NULL` → identity, non-`NULL` → arbitrary `c2x` (translation + rotation, incl. non-unit `c2r`) | lines 368–375, used by `c2Mulxv`/`c2MulrvT` |
| `use_radius` | `0` (raw Minkowski distance), non-`0` (shrink by `rA+rB`) | line 482 |
| radius sub-branch | `dist > rA+rB && dist > FLT_EPSILON` vs the `else` midpoint collapse; plus the `a==b ⇒ dist=0` re-check | lines 485–498 |
| `cache` | `NULL`; non-`NULL` cold (`count == 0`); non-`NULL` warm (replayed from a previous call) | lines 383–408, 500–509 |
| cache accept/reject | `!(min_metric < max_metric*2 && metric < -1e8f)` — accept in practice; reject only for a huge negative metric | line 405 |
| `outA` / `outB` / `iterations` | `NULL` vs non-`NULL` (independently) | lines 510–515 |
| overlap state | separated (`hit == 0`, `count` ends 1 or 2) vs penetrating (`hit == 1`, `count == 3`) | line 441 |
| loop exit reason | `count == 3`; `d1 > d0`; `dot(d,d) < eps²`; duplicate support point; `iter == 20` | lines 441, 447, 451, 471, 425 |
| simplex `count` in the reduction `switch` | 1 (no-op), 2 (`c22`), 3 (`c23`), anything else (no case ⇒ no-op) | lines 431–440 |
| `c22` arms | `v <= 0`; `u <= 0`; else | lines 191–205 |
| `c23` arms | 7 distinct arms (3× vertex, 3× edge, 1× interior) | lines 222–261 |
| `c2D` arms | `count == 1`; `count == 2` with `det > 0` (`c2Skew`); `count == 2` with `det <= 0` (`c2CCW90`); `count == 3`/default | lines 283–295 |
| `c2Witness` / `c2L` / `c2GJKSimplexMetric` arms | `count` = 1, 2, 3, default | lines 313–335, 348–356, 161–169 |
| `c2Support` | `count` = 0/1/2/4/8, ties, `NaN`s, `d == {0,0}` | lines 298–309 |
| `gjk_cache` `reverse` | `0` → `(AABB, CAPSULE)`; non-`0` → `(CAPSULE, AABB)` | line 552 |
| float input shape | normal; `±0.0`; denormal; `FLT_MIN`/`FLT_MAX`; `±inf`; `NaN` (both payload signs); exact integers; values that make `c2Dot` overflow | every arithmetic helper |

## Rows — one per combination the C treats differently

Every row is driven with **many** randomized inputs from a fixed-seed
xorshift PRNG (see `tests/common/mod.rs`), plus the hand-picked boundary values
named in the row. Both `.so`s are loaded with `libloading` and every output
(return value, all out-params, and the full `cache` / `c2Simplex` / `c2Proxy`
byte images) is compared **bit-for-bit** (`to_bits()`, not `==`).

### Group 1 — leaf vector helpers (lowest level, called directly)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C01 | `c2V` | 4096 random `(x, y)` bit patterns incl. all-`NaN`, `±inf`, `±0`, denormals | [x] |
| C02 | `c2Mulvs` | random `c2v` × random scalar; scalar ∈ {0, -0, 1, -1, inf, -inf, NaN, denormal, FLT_MAX} | [x] |
| C03 | `c2Add`, `c2Sub` | random pairs + overflow-to-`inf` pairs + `inf - inf` (`NaN`) + `±0` mixes | [x] |
| C04 | `c2Dot` | random pairs; products that cancel exactly; overflow; `0*inf` | [x] |
| C05 | `c2Det2` | random pairs; collinear (det `0`); antiparallel; `NaN`/`inf` | [x] |
| C06 | `c2Len` | random `c2v`; zero vector; huge (overflow inside `c2Dot`); `NaN`; negative-zero components | [x] |
| C07 | `c2Div` | random `c2v` × divisor ∈ random ∪ {0, -0, 1, inf, NaN} | [x] |
| C08 | `c2Norm` | random `c2v`; unit vectors; zero vector; huge; `NaN` | [x] |
| C09 | `c2Neg`, `c2Skew`, `c2CCW90` | random bit patterns incl. `±0` and `NaN` (bit-exact sign check) | [x] |
| C10 | `c2Maxv`, `c2Minv` | random pairs; `a == b`; `+0` vs `-0`; one `NaN`; both `NaN` | [x] |
| C11 | `c2Clampv` | `lo < hi` random; `lo == hi`; `lo > hi` inverted; `a` inside / below / above; `NaN` in each of the three | [x] |
| C12 | `c2RotIdentity`, `c2xIdentity` | no inputs — called repeatedly, result compared bit-exact | [x] |
| C13 | `c2Mulrv` | random `c2r` (incl. non-unit, zero, `NaN`, `inf`) × random `c2v`; exact rotations (0°, 90°, 180°, 270°) | [x] |
| C14 | `c2MulrvT` | same matrix as C13 — separate row because the C spells the sign differently (`-a.s * b.x`), which matters for `NaN` sign and `-0.0` | [x] |
| C15 | `c2Mulrv` ∘ `c2MulrvT` | round-trip on random rotations, verifying both directions agree between C and Rust | [x] |
| C16 | `c2Mulxv` | random `c2x` (identity, pure translation, pure rotation, both, degenerate `c2r`) × random `c2v` | [x] |

### Group 2 — proxy construction

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C17 | `c2BBVerts` | random AABB; normal box; inverted box; empty box (`min == max`); `±inf` corners; `NaN` corners | [x] |
| C18 | `c2MakeProxy` | `type = CIRCLE`, random `c2Circle` (r = 0, r < 0, r huge, `NaN`); `p` pre-poisoned with a known pattern so untouched fields are observable | [x] |
| C19 | `c2MakeProxy` | `type = AABB`, random `c2AABB` incl. inverted/empty/`NaN`; poisoned `p` | [x] |
| C20 | `c2MakeProxy` | `type = CAPSULE`, random `c2Capsule` incl. `a == b`, r = 0, r < 0, `NaN`; poisoned `p` | [x] |

### Group 3 — simplex reduction primitives, driven directly

The `c2Simplex` is built by hand (all 152 bytes controlled) so every arm of
`c22` / `c23` / `c2D` / `c2L` / `c2Witness` / `c2GJKSimplexMetric` is reachable
without going through `c2GJK`. Full struct image compared after each call.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C21 | `c22` | `count = 2`, random `a.p`/`b.p`; targeted `v <= 0` arm (origin beyond `b`) | [x] |
| C22 | `c22` | targeted `u <= 0` arm (origin beyond `a`) — checks the `s->a = s->b` copy of the *whole* `c2sv` (`sA`,`sB`,`p`,`iA`,`iB`) | [x] |
| C23 | `c22` | targeted interior arm (`u > 0 && v > 0`) — checks `div = u + v` | [x] |
| C24 | `c22` | degenerate `a.p == b.p`; `NaN` points; `±inf` points | [x] |
| C25 | `c23` | arm 1: `vAB <= 0 && uCA <= 0` (vertex A region) | [x] |
| C26 | `c23` | arm 2: `uAB <= 0 && vBC <= 0` (vertex B region) — checks `a = b` full copy | [x] |
| C27 | `c23` | arm 3: `uBC <= 0 && vCA <= 0` (vertex C region) — checks `a = c` full copy | [x] |
| C28 | `c23` | arm 4: edge AB (`wABC <= 0`) | [x] |
| C29 | `c23` | arm 5: edge BC (`uABC <= 0`) — checks the `a = b; b = c` shift | [x] |
| C30 | `c23` | arm 6: edge CA (`vABC <= 0`) — checks the `b = a; a = c` swap | [x] |
| C31 | `c23` | arm 7: interior (all barycentrics positive), both CW and CCW winding (sign of `area`) | [x] |
| C32 | `c23` | degenerate: collinear points (`area == 0`), all-equal points, `NaN`, `±inf` | [x] |
| C33 | `c23` | 4096 fully random triangles — hits the arms in their natural proportions and catches arm-ordering mistakes | [x] |
| C34 | `c2D` | `count = 1` random `a.p`; `count = 2` with `det2 > 0` (→ `c2Skew`); `count = 2` with `det2 <= 0` (→ `c2CCW90`); `det2 == 0` exactly; `count = 3` | [x] |
| C35 | `c2L` | `count = 1`; `count = 2` with random `u`s and `div`; `div` = 0 / `NaN` / huge; `count = 3` | [x] |
| C36 | `c2Witness` | `count = 1`; `count = 2`; `count = 3`; random `sA`/`sB`/`u`/`div` for each | [x] |
| C37 | `c2GJKSimplexMetric` | `count = 1`; `count = 2` (→ `c2Len`); `count = 3` (→ `c2Det2`); random points | [x] |
| C38 | `c2Support` | `count` ∈ {1,2,4,8} × random verts × random `d`, plus `d = {0,0}`, tie-inducing verts, `NaN` verts | [x] |

### Group 4 — `c2GJK`, the real driver (cross-product of its options)

Rows C39–C47 are the 3×3 shape-type cross product; each is run with random
shapes in three separation regimes (well separated / touching / deeply
overlapping) and with `use_radius` both 0 and 1, `cache` NULL, and identity
transforms — i.e. the baseline. Rows C48+ then vary one option at a time on top
of the full 3×3 grid.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C39 | `c2GJK` | `CIRCLE` vs `CIRCLE` × {separated, touching, overlapping} × `use_radius` ∈ {0,1} | [x] |
| C40 | `c2GJK` | `CIRCLE` vs `AABB` × same | [x] |
| C41 | `c2GJK` | `CIRCLE` vs `CAPSULE` × same | [x] |
| C42 | `c2GJK` | `AABB` vs `CIRCLE` × same | [x] |
| C43 | `c2GJK` | `AABB` vs `AABB` × same | [x] |
| C44 | `c2GJK` | `AABB` vs `CAPSULE` × same | [x] |
| C45 | `c2GJK` | `CAPSULE` vs `CIRCLE` × same | [x] |
| C46 | `c2GJK` | `CAPSULE` vs `AABB` × same | [x] |
| C47 | `c2GJK` | `CAPSULE` vs `CAPSULE` × same | [x] |
| C48 | `c2GJK` | full 3×3 grid, `ax_ptr` non-NULL (random translation + random *unit* rotation), `bx_ptr` NULL | [x] |
| C49 | `c2GJK` | full 3×3 grid, `ax_ptr` NULL, `bx_ptr` non-NULL (random translation + rotation) | [x] |
| C50 | `c2GJK` | full 3×3 grid, both transforms non-NULL, random unit rotations | [x] |
| C51 | `c2GJK` | full 3×3 grid, both transforms non-NULL with **non-unit / degenerate** `c2r` (scale ≠ 1, `{0,0}`, `{inf,0}`, `{NaN,NaN}`) | [x] |
| C52 | `c2GJK` | full 3×3 grid, `cache` non-NULL & cold (`count = 0`) — verifies the write-back (`metric`, `count`, `iA[]`, `iB[]`, `div`) byte-for-byte | [x] |
| C53 | `c2GJK` | full 3×3 grid, `cache` non-NULL, called **twice** with the same cache (the `gjk_cache` pattern) — second call replays the cache; both return values, both out-pairs, both `iterations`, and the final cache image compared | [x] |
| C54 | `c2GJK` | full 3×3 grid, same cache reused **8 times** while the shapes are perturbed between calls — exercises cache replay against a *changed* proxy, incl. stale indices that are still in range | [x] |
| C55 | `c2GJK` | full 3×3 grid × `outA = NULL` / `outB = NULL` / `iterations = NULL` (each alone and all together) with `cache` non-NULL, confirming the cache is still written | [x] |
| C56 | `c2GJK` | `use_radius = 1` with radii large enough that `dist <= rA+rB` (midpoint-collapse branch) | [x] |
| C57 | `c2GJK` | `use_radius = 1` with `dist` just above/below `FLT_EPSILON` (boundary of line 485) | [x] |
| C58 | `c2GJK` | `use_radius` non-boolean (`-1`, `2`, `INT_MIN`, `INT_MAX`) on the full 3×3 grid | [x] |
| C59 | `c2GJK` | degenerate shapes: zero-radius circle, empty AABB (`min == max`), inverted AABB, zero-length capsule (`a == b`), negative radii — full 3×3 grid | [x] |
| C60 | `c2GJK` | coincident shapes (A and B at exactly the same place) — drives the `hit == 1` / `count == 3` path and the `dup` break | [x] |
| C61 | `c2GJK` | shapes far apart at `FLT_MAX`-scale coordinates → `c2Dot` overflows to `inf`, exercising the `d1 > d0` early-out | [x] |
| C62 | `c2GJK` | shapes with `NaN` coordinates — full 3×3 grid | [x] |
| C63 | `c2GJK` | tiny (denormal-scale) shapes → `dot(d,d) < eps²` early-out | [x] |
| C64 | `c2GJK` | many-iteration case: thin sliver AABB vs long capsule, tuned so `iter` climbs; verifies `*iterations` matches exactly.  Measured histogram over 200 k configurations: `iter` ∈ 0..5 (the largest value reachable at all is **6**; the `iter < 20` cap is dead code — see ERRORS.md E46) | [x] |
| C65 | `c2GJK` | 4096 fully random draws over the whole option cross-product (types × transforms × `use_radius` × cache-mode × NULL-out mask), seeded — the property-style sweep | [x] |

### Group 5 — `gjk_cache`, the header's public entry point

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C66 | `gjk_cache` | `reverse = 0`, random `a1..a4` / `b1..b5`; `a9`/`b9` non-NULL and pre-poisoned (must stay untouched) | [x] |
| C67 | `gjk_cache` | `reverse = 1`, same | [x] |
| C68 | `gjk_cache` | `reverse` ∈ {`-1`, `2`, `0x7F`, `-128`} (non-boolean `char`) | [x] |
| C69 | `gjk_cache` | `a9 = NULL`, `b9 = NULL`, both `reverse` values | [x] |
| C70 | `gjk_cache` | degenerate params: inverted AABB, empty AABB, zero-length capsule, `b5 = 0`, `b5 < 0` | [x] |
| C71 | `gjk_cache` | extreme params: `±inf`, `NaN`, `FLT_MAX`, `FLT_MIN`, denormals, `±0` | [x] |
| C72 | `gjk_cache` | 4096 fully random parameter draws × both `reverse` values (seeded sweep) | [x] |

## Compile-time configuration matrix

| # | feature set | command | [ ] |
|---|-------------|---------|-----|
| F1 | default, release profile | `cargo test --release` | [x] |
| F2 | `--no-default-features`, release profile | `cargo test --release --no-default-features` | [x] |
| F3 | default, dev profile | `cargo test` | [x] |
| F4 | `--no-default-features`, dev profile | `cargo test --no-default-features` | [x] |

`Cargo.toml` has no `[features]` table, so F1/F2 and F3/F4 are the same code;
they are run separately anyway because the *profile* is a real axis (see the
`[profile.dev]` note in `Cargo.toml`: `debug-assertions` and `overflow-checks`
had to be turned off so that a NULL dereference faults with SIGSEGV in both
libraries instead of aborting from a libcore UB-check).  `./verify.sh` walks all
four automatically and would pick up any features added later.

Result: **113 tests, 0 failures, in all four configurations; 31/31 symbols
present in each.**

## Test files backing the rows

| file | rows | tests |
|------|------|-------|
| `tests/abi_sanity.rs` | ABI/ground-truth anchors (absolute hand-computed answers, not just C-vs-Rust agreement) | 4 |
| `tests/leaf_helpers.rs` | C01–C16 | 16 |
| `tests/proxy.rs` | C17–C20 | 4 |
| `tests/simplex.rs` | C21–C38 | 9 |
| `tests/gjk.rs` | C39–C65 | 27 |
| `tests/gjk_cache_entry.rs` | C66–C72 | 7 |
| `tests/nan_payloads.rs` | NaN-payload / signed-zero hardening for C01–C16 + `c2GJK` | 13 |
| `tests/error_paths.rs` | E01–E93 | 30 |
| `tests/coverage_probes.rs` | branch-reachability searches (E43, E46) | 3 |
