# SYMBOLS.md — Phase A symbol surface

Derived mechanically from `nm -D --defined-only` on both shared objects.

* C  `.so`: `c_src/build/libharvest-work-wVEwSX.so` (gcc 11.5, `CMAKE_BUILD_TYPE=""` → `-O0`)
* Rust `.so`: `translation/target/release/libgen_ray_lib.so`

Regenerate / re-diff with:

```sh
./check_symbols.sh
```

## Exported (`T`) symbols

| # | C symbol | in C `.so` | in Rust `.so` | C source | notes |
|---|----------|-----------|---------------|----------|-------|
|  1 | `c2V`             | T | T | `src/lib.c:32`  | ctor |
|  2 | `c2Dot`           | T | T | `src/lib.c:39`  | |
|  3 | `c2Len`           | T | T | `src/lib.c:43`  | calls `sqrtf` |
|  4 | `c2Add`           | T | T | `src/lib.c:47`  | |
|  5 | `c2Sub`           | T | T | `src/lib.c:53`  | |
|  6 | `c2Mulvs`         | T | T | `src/lib.c:59`  | |
|  7 | `c2Div`           | T | T | `src/lib.c:65`  | reciprocal multiply |
|  8 | `c2Norm`          | T | T | `src/lib.c:69`  | |
|  9 | `c2Minv`          | T | T | `src/lib.c:73`  | ternary `min`, not `fminf` |
| 10 | `c2Maxv`          | T | T | `src/lib.c:78`  | ternary `max`, not `fmaxf` |
| 11 | `c2Skew`          | T | T | `src/lib.c:83`  | |
| 12 | `c2Absv`          | T | T | `src/lib.c:90`  | ternary `abs`, not `fabsf` |
| 13 | `c2RaytoCircle`   | T | T | `src/lib.c:94`  | writes `*out` |
| 14 | `c2AABBtoAABB`    | T | T | `src/lib.c:112` | |
| 15 | `c2RaytoAABB`     | T | T | `src/lib.c:139` | writes `*out` |
| 16 | `c2CCW90`         | T | T | `src/lib.c:203` | |
| 17 | `c2MulmvT`        | T | T | `src/lib.c:210` | |
| 18 | `c2AABBtoPoint`   | T | T | `src/lib.c:217` | |
| 19 | `c2CircleToPoint` | T | T | `src/lib.c:225` | |
| 20 | `c2RaytoCapsule`  | T | T | `src/lib.c:231` | writes `*out` |
| 21 | `c2CastRay`       | T | T | `src/lib.c:294` | dispatch on `C2_TYPE`; **no `default:`** |
| 22 | `gen_ray`         | T | T | `src/lib.c:306` | public header entry point |

**22 / 22 present. Symbol diff is EMPTY.**

## `static` (non-exported) C functions — deliberately NOT exported by Rust either

| C symbol | C source | in C `.so` | in Rust `.so` |
|----------|----------|-----------|---------------|
| `c2SignedDistPointToPlane_OneDimensional` | `src/lib.c:120` (`static inline`) | local `t` only | private `fn` |
| `c2RayToPlane_OneDimensional`              | `src/lib.c:125` (`static inline`) | local `t` only | private `fn` |

## Undefined / imported symbols

| C `.so` undefined | Rust `.so` |
|-------------------|------------|
| `sqrtf@GLIBC_2.2.5` (`U`)      | inlined `sqrtss` (no import needed) |
| `__cxa_finalize@GLIBC_2.2.5` (`w`) | present (glibc/`std` startup) |
| `_ITM_deregisterTMCloneTable` (`w`) | n/a (weak, transaction-memory stub) |
| `_ITM_registerTMCloneTable` (`w`)   | n/a (weak, transaction-memory stub) |
| `__gmon_start__` (`w`)              | n/a (weak, profiling stub) |

0 missing/undefined **non-libc** symbols in the Rust `.so`. The Rust `.so`
additionally exports the usual `std`/`unwind`-related symbols, which is a
superset and therefore harmless.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, therefore the only
feature combination that exists is the default (empty) one. Verified by
`grep -n '^\[features\]' Cargo.toml` → no match. Phase D's "every feature
combination" therefore collapses to:

* `cargo test` (default)
* `cargo test --no-default-features` (identical — no default features exist)
* `cargo test --release`

All three are run by `./run_all.sh`.
