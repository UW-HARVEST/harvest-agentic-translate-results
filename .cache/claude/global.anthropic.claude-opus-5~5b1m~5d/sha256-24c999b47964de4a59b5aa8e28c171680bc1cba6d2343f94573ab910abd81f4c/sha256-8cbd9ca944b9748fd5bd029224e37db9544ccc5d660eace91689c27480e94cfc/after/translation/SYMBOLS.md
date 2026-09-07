# SYMBOLS.md — Phase A symbol surface

Source of truth: `nm -D --defined-only` (text symbols, `T`) on the C shared
object `c_src/build/libharvest-work-D30ByW.so`, compared against the Rust
cdylib `translation/target/release/libgjk_cache_lib.so`.

Reproduce with:

```sh
nm -D --defined-only c_src/build/libharvest-work-D30ByW.so       | awk '$2=="T"{print $3}' | sort > c.txt
nm -D --defined-only translation/target/release/libgjk_cache_lib.so | awk '$2=="T"{print $3}' | sort > r.txt
comm -23 c.txt r.txt   # missing in Rust  -> MUST be empty
comm -13 c.txt r.txt   # extra   in Rust  -> informational
```

## Symbol table (31 symbols)

| # | symbol | C signature (from `c_src/src/lib.c`) | in C .so | in Rust .so |
|---|--------|--------------------------------------|----------|-------------|
| 1  | `c2V`                | `c2v c2V(float x, float y)` | yes | yes |
| 2  | `c2Mulvs`            | `c2v c2Mulvs(c2v a, float b)` | yes | yes |
| 3  | `c2Maxv`             | `c2v c2Maxv(c2v a, c2v b)` | yes | yes |
| 4  | `c2Minv`             | `c2v c2Minv(c2v a, c2v b)` | yes | yes |
| 5  | `c2Clampv`           | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` | yes | yes |
| 6  | `c2Sub`              | `c2v c2Sub(c2v a, c2v b)` | yes | yes |
| 7  | `c2Dot`              | `float c2Dot(c2v a, c2v b)` | yes | yes |
| 8  | `c2RotIdentity`      | `c2r c2RotIdentity(void)` | yes | yes |
| 9  | `c2xIdentity`        | `c2x c2xIdentity(void)` | yes | yes |
| 10 | `c2BBVerts`          | `void c2BBVerts(c2v *out, c2AABB *bb)` | yes | yes |
| 11 | `c2MakeProxy`        | `void c2MakeProxy(const void *shape, C2_TYPE type, c2Proxy *p)` | yes | yes |
| 12 | `c2Len`              | `float c2Len(c2v a)` | yes | yes |
| 13 | `c2Det2`             | `float c2Det2(c2v a, c2v b)` | yes | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex *s)` | yes | yes |
| 15 | `c2Mulrv`            | `c2v c2Mulrv(c2r a, c2v b)` | yes | yes |
| 16 | `c2Add`              | `c2v c2Add(c2v a, c2v b)` | yes | yes |
| 17 | `c2Mulxv`            | `c2v c2Mulxv(c2x a, c2v b)` | yes | yes |
| 18 | `c22`                | `void c22(c2Simplex *s)` | yes | yes |
| 19 | `c23`                | `void c23(c2Simplex *s)` | yes | yes |
| 20 | `c2Neg`              | `c2v c2Neg(c2v a)` | yes | yes |
| 21 | `c2Skew`             | `c2v c2Skew(c2v a)` | yes | yes |
| 22 | `c2CCW90`            | `c2v c2CCW90(c2v a)` | yes | yes |
| 23 | `c2D`                | `c2v c2D(c2Simplex *s)` | yes | yes |
| 24 | `c2Support`          | `int c2Support(const c2v *verts, int count, c2v d)` | yes | yes |
| 25 | `c2Witness`          | `void c2Witness(c2Simplex *s, c2v *a, c2v *b)` | yes | yes |
| 26 | `c2Div`              | `c2v c2Div(c2v a, float b)` | yes | yes |
| 27 | `c2Norm`             | `c2v c2Norm(c2v a)` | yes | yes |
| 28 | `c2L`                | `c2v c2L(c2Simplex *s)` | yes | yes |
| 29 | `c2MulrvT`           | `c2v c2MulrvT(c2r a, c2v b)` | yes | yes |
| 30 | `c2GJK`              | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | yes | yes |
| 31 | `gjk_cache`          | `void gjk_cache(char, c2v*, c2v*, float×9)` | yes | yes |

## Result

- Missing in Rust (`comm -23`): **EMPTY** ✅
- Extra in Rust (`comm -13`): **EMPTY** ✅
- Undefined symbols in the Rust `.so`: only libc / libgcc-unwind
  (`memcpy`, `malloc`, `_Unwind_*`, `abort`, …) — **0 missing non-libc
  symbols** ✅
- No C source file was left untranslated: `c_src/src/lib.c` is the only
  translation unit and every non-static function in it is present.

## Verification status

Verified by `./run_all.sh` from a clean tree (C `build/` and `target/` removed):

| profile | symbol parity | undefined non-libc | differential tests |
|---------|---------------|--------------------|--------------------|
| `debug`   | 31 / 31, 0 missing, 0 extra | none | 88 passed, 0 failed |
| `release` | 31 / 31, 0 missing, 0 extra | none | 88 passed, 0 failed |

`tests/phase_c_errors.rs::harness_loads_two_distinct_libraries` proves the
suite is not comparing one library against itself: it asserts the two `.so`
paths differ and that all 31 resolved function pointers are at different
addresses in the two objects. The test harness selects the Rust `.so` matching
its own build profile (derived from `current_exe()`), so a `--release` run
always exercises the release cdylib.

## Build / feature configurations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
feature combination is the default (empty) one. `--no-default-features`
is therefore equivalent to the default build; both are exercised by the
`run_all.sh` driver. The C project builds **no binary executable**
(`add_library(... SHARED ...)` only), so there is no stdout comparison to
make; the FFI differential tests are the complete surface.
