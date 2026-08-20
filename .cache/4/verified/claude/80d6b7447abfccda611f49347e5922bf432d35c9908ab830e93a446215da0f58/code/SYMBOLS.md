# SYMBOLS.md — Phase A symbol surface

Mechanically derived from `nm -D` on both shared objects.

* C `.so`   : `c_src/build/libtranslated_rust.so`   (gcc 11.5.0, `cmake -DCMAKE_POSITION_INDEPENDENT_CODE=ON`)
* Rust `.so`: `target/debug/libomni_manifold_lib.so` (`cargo build`, `crate-type = ["cdylib"]`)

Regenerate / re-verify with:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort -u > /tmp/c.txt
nm -D --defined-only target/debug/libomni_manifold_lib.so | awk '{print $3}' | sort -u > /tmp/r.txt
comm -23 /tmp/c.txt /tmp/r.txt   # must be EMPTY  (C exports that Rust lacks)
```

## Status: PASS — the symbol diff is empty

Also asserted as a test (`tests/phase_d_symbols.rs`), which additionally
(a) resolves every C symbol through `dlsym` on the Rust `.so`, not merely checks the
table, (b) verifies the five `static` C helpers are exported by *neither* library, and
(c) audits the Rust `.so`'s undefined symbols against a libc/std allowlist.

## Result

```
C dynamic symbols   : 46
Rust dynamic symbols: 46 unmangled `c2*`/`omni_manifold`/`ptr_from_parts` + Rust-internal `_ZN…`
Missing from Rust   : 0      <-- symbol diff is EMPTY
Extra in Rust       : Rust-runtime-internal mangled symbols only (std/panic machinery)
```

## Undefined (imported) symbols

| library | undefined symbols |
|---|---|
| C    | `malloc`, `sqrtf`, `__cxa_finalize`, `__gmon_start__`, `_ITM_*` |
| Rust | `malloc` + libc (`memcpy`, `memset`, `free`, `realloc`, `calloc`, `posix_memalign`, `bcmp`, `strlen`, `abort`, `__errno_location`, …), `pthread_*`, `_Unwind_*` (std panic machinery), `__cxa_finalize`, `__gmon_start__`, `_ITM_*` |

**0 missing / undefined non-libc symbols in the Rust `.so`.** `sqrtf` is not
imported by Rust because `f32::sqrt` lowers to the `sqrtss` instruction directly;
glibc's `sqrtf` does the same, and both are the IEEE-754 exact square root, so
this is not an observable difference (covered by the `c2Len` / `c2Norm` /
`c2Circleto*` differential tests).

## Static (non-exported) C functions

These are `static` in `c_src/src/lib.c` and therefore correctly absent from
*both* dynamic symbol tables. They are private in Rust too (`manifold.rs`):

`c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`, `c2KeepDeep`, `c2Incident`

They are reached indirectly and are covered by the `c2CapsuletoPolyManifold` /
`c2AABBtoCapsuleManifold` / `omni_manifold` differential tests.

## Full table

| # | C symbol | type (nm) | in Rust `.so`? | Rust definition site |
|---|----------|-----------|----------------|----------------------|
| 1 | `c22` | `T` | YES | `src/gjk.rs:32` |
| 2 | `c23` | `T` | YES | `src/gjk.rs:60` |
| 3 | `c2AABBtoAABBManifold` | `T` | YES | `src/manifold.rs:243` |
| 4 | `c2AABBtoCapsuleManifold` | `T` | YES | `src/manifold.rs:428` |
| 5 | `c2Absv` | `T` | YES | `src/math.rs:203` |
| 6 | `c2Add` | `T` | YES | `src/math.rs:142` |
| 7 | `c2BBVerts` | `T` | YES | `src/shapes.rs:34` |
| 8 | `c2CCW90` | `T` | YES | `src/math.rs:191` |
| 9 | `c2CapsuletoCapsuleManifold` | `T` | YES | `src/manifold.rs:443` |
| 10 | `c2CapsuletoPolyManifold` | `T` | YES | `src/manifold.rs:294` |
| 11 | `c2CircletoAABBManifold` | `T` | YES | `src/manifold.rs:164` |
| 12 | `c2CircletoCapsuleManifold` | `T` | YES | `src/manifold.rs:208` |
| 13 | `c2CircletoCircleManifold` | `T` | YES | `src/manifold.rs:141` |
| 14 | `c2Clampv` | `T` | YES | `src/math.rs:60` |
| 15 | `c2Collide` | `T` | YES | `src/api.rs:20` |
| 16 | `c2D` | `T` | YES | `src/gjk.rs:128` |
| 17 | `c2Det2` | `T` | YES | `src/math.rs:109` |
| 18 | `c2Dist` | `T` | YES | `src/math.rs:81` |
| 19 | `c2Div` | `T` | YES | `src/math.rs:170` |
| 20 | `c2Dot` | `T` | YES | `src/math.rs:74` |
| 21 | `c2GJK` | `T` | YES | `src/gjk.rs:235` |
| 22 | `c2GJKSimplexMetric` | `T` | YES | `src/gjk.rs:18` |
| 23 | `c2Intersect` | `T` | YES | `src/math.rs:164` |
| 24 | `c2L` | `T` | YES | `src/gjk.rs:194` |
| 25 | `c2Len` | `T` | YES | `src/math.rs:102` |
| 26 | `c2MakeProxy` | `T` | YES | `src/shapes.rs:55` |
| 27 | `c2Maxv` | `T` | YES | `src/math.rs:40` |
| 28 | `c2Minv` | `T` | YES | `src/math.rs:50` |
| 29 | `c2Mulrv` | `T` | YES | `src/math.rs:116` |
| 30 | `c2MulrvT` | `T` | YES | `src/math.rs:129` |
| 31 | `c2Mulvs` | `T` | YES | `src/math.rs:28` |
| 32 | `c2Mulxv` | `T` | YES | `src/math.rs:152` |
| 33 | `c2MulxvT` | `T` | YES | `src/math.rs:158` |
| 34 | `c2Neg` | `T` | YES | `src/math.rs:184` |
| 35 | `c2Norm` | `T` | YES | `src/math.rs:178` |
| 36 | `c2Norms` | `T` | YES | `src/shapes.rs:109` |
| 37 | `c2PlaneAt` | `T` | YES | `src/shapes.rs:19` |
| 38 | `c2RotIdentity` | `T` | YES | `src/math.rs:87` |
| 39 | `c2Skew` | `T` | YES | `src/math.rs:197` |
| 40 | `c2Sub` | `T` | YES | `src/math.rs:66` |
| 41 | `c2Support` | `T` | YES | `src/shapes.rs:90` |
| 42 | `c2V` | `T` | YES | `src/math.rs:22` |
| 43 | `c2Witness` | `T` | YES | `src/gjk.rs:149` |
| 44 | `c2xIdentity` | `T` | YES | `src/math.rs:93` |
| 45 | `omni_manifold` | `T` | YES | `src/api.rs:152` |
| 46 | `ptr_from_parts` | `T` | YES | `src/api.rs:115` |
