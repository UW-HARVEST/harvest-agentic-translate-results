# SYMBOLS.md — exported-symbol parity

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-j8r5HC.so \
  | awk '$2=="T"||$2=="W"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libreverse_collide_lib.so \
  | awk '$2=="T"||$2=="W"||$2=="B"||$2=="D"{print $3}' | sort > /tmp/rust_syms.txt
comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt   # missing in Rust  -> EMPTY
comm -13 /tmp/c_syms.txt /tmp/rust_syms.txt   # extra   in Rust  -> EMPTY
```

The whole C library is a single translation unit (`c_src/src/lib.c`) with **no
`static` functions**, so every function in the file is a public dynamic symbol.
`c_src/include/lib.h` declares only `reverse_collide`; the other 37 symbols are
exported incidentally, and the Rust crate must (and does) export them too.

## Symbol table (38 symbols)

| # | symbol | in C `.so` | in Rust `.so` | C signature |
|---|--------|-----------|---------------|-------------|
| 1 | `c2V` | T | T | `c2v c2V(float, float)` |
| 2 | `c2Mulvs` | T | T | `c2v c2Mulvs(c2v, float)` |
| 3 | `c2Maxv` | T | T | `c2v c2Maxv(c2v, c2v)` |
| 4 | `c2Minv` | T | T | `c2v c2Minv(c2v, c2v)` |
| 5 | `c2Clampv` | T | T | `c2v c2Clampv(c2v, c2v, c2v)` |
| 6 | `c2Sub` | T | T | `c2v c2Sub(c2v, c2v)` |
| 7 | `c2Dot` | T | T | `float c2Dot(c2v, c2v)` |
| 8 | `c2RotIdentity` | T | T | `c2r c2RotIdentity(void)` |
| 9 | `c2xIdentity` | T | T | `c2x c2xIdentity(void)` |
| 10 | `c2BBVerts` | T | T | `void c2BBVerts(c2v*, c2AABB*)` |
| 11 | `c2MakeProxy` | T | T | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` |
| 12 | `c2Len` | T | T | `float c2Len(c2v)` |
| 13 | `c2Det2` | T | T | `float c2Det2(c2v, c2v)` |
| 14 | `c2GJKSimplexMetric` | T | T | `float c2GJKSimplexMetric(c2Simplex*)` |
| 15 | `c2Mulrv` | T | T | `c2v c2Mulrv(c2r, c2v)` |
| 16 | `c2Add` | T | T | `c2v c2Add(c2v, c2v)` |
| 17 | `c2Mulxv` | T | T | `c2v c2Mulxv(c2x, c2v)` |
| 18 | `c22` | T | T | `void c22(c2Simplex*)` |
| 19 | `c23` | T | T | `void c23(c2Simplex*)` |
| 20 | `c2Neg` | T | T | `c2v c2Neg(c2v)` |
| 21 | `c2Skew` | T | T | `c2v c2Skew(c2v)` |
| 22 | `c2CCW90` | T | T | `c2v c2CCW90(c2v)` |
| 23 | `c2D` | T | T | `c2v c2D(c2Simplex*)` |
| 24 | `c2Support` | T | T | `int c2Support(const c2v*, int, c2v)` |
| 25 | `c2Witness` | T | T | `void c2Witness(c2Simplex*, c2v*, c2v*)` |
| 26 | `c2Div` | T | T | `c2v c2Div(c2v, float)` |
| 27 | `c2Norm` | T | T | `c2v c2Norm(c2v)` |
| 28 | `c2L` | T | T | `c2v c2L(c2Simplex*)` |
| 29 | `c2MulrvT` | T | T | `c2v c2MulrvT(c2r, c2v)` |
| 30 | `c2GJK` | T | T | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` |
| 31 | `c2AABBtoAABB` | T | T | `int c2AABBtoAABB(c2AABB, c2AABB)` |
| 32 | `c2AABBtoCapsule` | T | T | `int c2AABBtoCapsule(c2AABB, c2Capsule)` |
| 33 | `c2CapsuletoCapsule` | T | T | `int c2CapsuletoCapsule(c2Capsule, c2Capsule)` |
| 34 | `c2CircletoCircle` | T | T | `int c2CircletoCircle(c2Circle, c2Circle)` |
| 35 | `c2CircletoAABB` | T | T | `int c2CircletoAABB(c2Circle, c2AABB)` |
| 36 | `c2CircletoCapsule` | T | T | `int c2CircletoCapsule(c2Circle, c2Capsule)` |
| 37 | `c2Collided` | T | T | `int c2Collided(const void*, C2_TYPE, const void*, C2_TYPE)` |
| 38 | `reverse_collide` | T | T | `int reverse_collide(float, float, float)` |

## Result

* **Missing from Rust `.so`: 0.**
* **Extra in Rust `.so`: 0.**
* Rust `.so` undefined symbols are all libc / `_Unwind_*` / `__cxa_*` /
  `_ITM_*` runtime imports — no unresolved project symbols.

Automated re-check: `translation/check_symbols.sh`.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only build
configuration is the default one (`--no-default-features` and the default build
are identical; verified by `check_features.sh`). There are no `#ifdef`s in the C
source either, so there is exactly one compile-time configuration to verify.

## Divergences found and fixed

Symbols matched from the start, and every value-path and error-path test passed
against the translation as delivered. The one real defect was found by the
**pointer-aliasing** axis (`CONFIGS.md` rows 89–94), which the initial
configuration table had missed:

| # | function | defect | fix |
|---|----------|--------|-----|
| 1 | `c2BBVerts` | The Rust snapshotted `bb->min` / `bb->max` into locals before writing `out[0..4]`. The C re-reads `bb` on every line, and neither pointer is `restrict`-qualified, so when `out` overlaps `*bb` the C's `out[1] = (bb->max.x, bb->min.y)` clobbers `bb->max` *before* `out[2] = bb->max` reads it. The snapshot hid that. | read each field through the pointer at its point of use |
| 2 | `c2Witness` | The Rust copied the whole `s->verts` array up front. `*a` is sequenced before the operands of `*b` are read, so an `a` pointing into `*s` must be visible to the `*b` computation. | read each field through the pointer at its point of use |

Defect 1 is also reachable through `c2MakeProxy`: when its `shape` argument
aliases its `c2Proxy`, `bb` lands on `p+0` while `out` is `p->verts` at `p+8`,
so the AABB arm hits exactly this overlap without the caller doing anything
exotic. Both were caught by `tests/aliasing.rs`, which now fails against the
pre-fix source and passes against the current one.

## Completion gate

| requirement | status |
|-------------|--------|
| `nm -D`: 0 symbols missing from the Rust `.so` | **PASS** — 38/38, 0 extra, 0 non-libc undefined (`check_symbols.sh`) |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | **PASS** — 94/94 rows |
| Phase C: every `ERRORS.md` row has a passing differential test | **PASS** — 53/53 rows |
| holds under every feature combination | **PASS** — 2/2 configs (`check_features.sh`); no `[features]` exist |
| not an artefact of one seed | **PASS** — full suite green at `DIFF_SEED_OFFSET` ∈ {0, 1, 2, 7, 13, 101, 4242, 999983, 31337} |

Final state: **61 tests, 0 failures**, clean `cargo build --release` and
`cargo check` with no warnings, and `c_src/` unmodified.
