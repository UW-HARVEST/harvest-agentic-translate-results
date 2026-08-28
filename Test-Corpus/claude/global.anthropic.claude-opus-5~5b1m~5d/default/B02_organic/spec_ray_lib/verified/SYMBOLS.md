# SYMBOLS.md — exported-symbol surface (Phase A)

Mechanically derived from

```
nm -D --defined-only c_src/build/libharvest-work-6VP3Pl.so   # C reference (cmake default build == -O0)
nm -D --defined-only translation/target/release/libspec_ray_lib.so
```

The C shared-library name is derived by `c_src/CMakeLists.txt` from the *parent
directory* of `c_src` (`project(${project_name})`), so it is
`lib<workdir-name>.so`; the tests glob `c_src/build/*.so` instead of hard-coding
it.

`c_src/src/lib.c` is the only translation unit, so the symbol surface is exactly
"every non-`static` function in that file". The two `static inline` helpers
(`c2SignedDistPointToPlane_OneDimensional`, `c2RayToPlane_OneDimensional`) are
deliberately **not** exported by either library — they are `static` in C and
private `fn`s in Rust.

## Exported symbol table (22 / 22)

| # | symbol | C signature (`c_src/src/lib.c`) | in C `.so` | in Rust `.so` |
|---|--------|--------------------------------|-----------|---------------|
|  1 | `c2V`             | `c2v c2V(float x, float y)`                                   | T | T |
|  2 | `c2Dot`           | `float c2Dot(c2v a, c2v b)`                                   | T | T |
|  3 | `c2Len`           | `float c2Len(c2v a)`                                          | T | T |
|  4 | `c2Add`           | `c2v c2Add(c2v a, c2v b)`                                     | T | T |
|  5 | `c2Sub`           | `c2v c2Sub(c2v a, c2v b)`                                     | T | T |
|  6 | `c2Mulvs`         | `c2v c2Mulvs(c2v a, float b)`                                 | T | T |
|  7 | `c2Div`           | `c2v c2Div(c2v a, float b)`                                   | T | T |
|  8 | `c2Norm`          | `c2v c2Norm(c2v a)`                                           | T | T |
|  9 | `c2Minv`          | `c2v c2Minv(c2v a, c2v b)`                                    | T | T |
| 10 | `c2Maxv`          | `c2v c2Maxv(c2v a, c2v b)`                                    | T | T |
| 11 | `c2Skew`          | `c2v c2Skew(c2v a)`                                           | T | T |
| 12 | `c2Absv`          | `c2v c2Absv(c2v a)`                                           | T | T |
| 13 | `c2CCW90`         | `c2v c2CCW90(c2v a)`                                          | T | T |
| 14 | `c2MulmvT`        | `c2v c2MulmvT(c2m a, c2v b)`                                  | T | T |
| 15 | `c2RaytoCircle`   | `int c2RaytoCircle(c2Ray A, c2Circle B, c2Raycast *out)`      | T | T |
| 16 | `c2AABBtoAABB`    | `int c2AABBtoAABB(c2AABB A, c2AABB B)`                        | T | T |
| 17 | `c2RaytoAABB`     | `int c2RaytoAABB(c2Ray A, c2AABB B, c2Raycast *out)`          | T | T |
| 18 | `c2AABBtoPoint`   | `int c2AABBtoPoint(c2AABB A, c2v B)`                          | T | T |
| 19 | `c2CircleToPoint` | `int c2CircleToPoint(c2Circle A, c2v B)`                       | T | T |
| 20 | `c2RaytoCapsule`  | `int c2RaytoCapsule(c2Ray A, c2Capsule B, c2Raycast *out)`    | T | T |
| 21 | `c2CastRay`       | `int c2CastRay(c2Ray A, const void *B, C2_TYPE typeB, c2Raycast *out)` | T | T |
| 22 | `spec_ray`        | `int spec_ray(c2Raycast *cast, float mp_x, float mp_y, float c_p_x, float c_p_y, float c_r, float r_p_x, float r_p_y)` | T | T |

## Diff result

```
comm -23 c_syms r_syms   ->  (empty)   # exported by C, missing from Rust
comm -13 c_syms r_syms   ->  (empty)   # extra exports in Rust
```

* **Missing symbols: 0.** No `#[no_mangle]` wrapper had to be added and no C
  source file was left untranslated (`src/lib.c` is the whole library).
* **Undefined symbols in the Rust `.so`: libc / libgcc-unwind only**
  (`memcpy`, `malloc`, `_Unwind_*`, `__errno_location`, …). No unresolved
  library symbol. Verified with `nm -D --undefined-only`.
* `sqrtf` is the only libm call in the C build; the Rust build lowers it to the
  `sqrtss` instruction, so it does not appear as an undefined symbol. Bit-exact
  equality of the two is covered by the `c2Len` / `c2RaytoCircle` differential
  tests.

## ABI notes that the differential tests must (and do) exercise

* `c2v` = `{float,float}` = 8 bytes, SysV class SSE → passed/returned in the low
  half of an XMM register. Every helper returning `c2v` therefore tests Rust's
  small-struct return ABI (`c2V`, `c2Add`, …).
* `c2m` = `{c2v,c2v}` = 16 bytes → two SSE eightbytes (xmm0/xmm1).
* `c2Ray` = 20 bytes and `c2Capsule` = 20 bytes → class MEMORY, passed on the
  stack. `c2AABB` = 16 bytes → xmm registers.
* `C2_TYPE` is an unsigned-int-sized enum in GCC; `c2CastRay` receives it in
  `esi` and the Rust side declares `c_int` — identical register/width behaviour
  for every 32-bit value, including values with no valid variant (see
  `ERRORS.md` row 24).

## Automated re-check

`./verify.sh` re-derives this diff for **every** build in the matrix (each cargo
feature combination x the dev and release cdylib) and fails if a single symbol is
missing:

```sh
nm -D --defined-only --format=posix "$C_SO"   | awk '$2=="T" && $1 !~ /^_/ {print $1}' | sort > c_syms
nm -D --defined-only --format=posix "$RUST_SO"| awk '$2=="T" && $1 !~ /^_/ {print $1}' | sort > r_syms
comm -23 c_syms r_syms     # must be empty
```

`tests/smoke_probe.rs::symbol_parity` performs the same check from inside the
test suite (and additionally resolves all 22 symbols in both libraries with
`dlsym`, which catches a symbol that exists but has the wrong type of
visibility), so a regression fails `cargo test` too.

Result for every build in the matrix: **22 exported, 22 resolved, 0 missing, 0
non-libc undefined symbols.**
