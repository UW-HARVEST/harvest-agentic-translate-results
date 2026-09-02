# SYMBOLS.md — exported symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-kxbveQ.so | awk '{print $3}' | sort > /tmp/c_syms.txt
nm -D --defined-only translation/target/release/libomni_manifold_lib.so | awk '{print $3}' | sort > /tmp/r_syms.txt
comm -23 /tmp/c_syms.txt /tmp/r_syms.txt   # in C, missing from Rust
comm -13 /tmp/c_syms.txt /tmp/r_syms.txt   # extra in Rust
```

Result: **C exports 46, Rust exports 46, both diffs EMPTY.**

`static` C functions (`c2Clip`, `c2SidePlanes`, `c2SidePlanesFromPoly`,
`c2KeepDeep`, `c2Incident`) have internal linkage and appear in neither `.so`;
they are private `unsafe fn`s in Rust. Correct.

| # | symbol | C `.so` | Rust `.so` | status |
|---|--------|---------|------------|--------|
| 1 | `c22` | T | T | OK |
| 2 | `c23` | T | T | OK |
| 3 | `c2AABBtoAABBManifold` | T | T | OK |
| 4 | `c2AABBtoCapsuleManifold` | T | T | OK |
| 5 | `c2Absv` | T | T | OK |
| 6 | `c2Add` | T | T | OK |
| 7 | `c2BBVerts` | T | T | OK |
| 8 | `c2CCW90` | T | T | OK |
| 9 | `c2CapsuletoCapsuleManifold` | T | T | OK |
| 10 | `c2CapsuletoPolyManifold` | T | T | OK |
| 11 | `c2CircletoAABBManifold` | T | T | OK |
| 12 | `c2CircletoCapsuleManifold` | T | T | OK |
| 13 | `c2CircletoCircleManifold` | T | T | OK |
| 14 | `c2Clampv` | T | T | OK |
| 15 | `c2Collide` | T | T | OK |
| 16 | `c2D` | T | T | OK |
| 17 | `c2Det2` | T | T | OK |
| 18 | `c2Dist` | T | T | OK |
| 19 | `c2Div` | T | T | OK |
| 20 | `c2Dot` | T | T | OK |
| 21 | `c2GJK` | T | T | OK |
| 22 | `c2GJKSimplexMetric` | T | T | OK |
| 23 | `c2Intersect` | T | T | OK |
| 24 | `c2L` | T | T | OK |
| 25 | `c2Len` | T | T | OK |
| 26 | `c2MakeProxy` | T | T | OK |
| 27 | `c2Maxv` | T | T | OK |
| 28 | `c2Minv` | T | T | OK |
| 29 | `c2Mulrv` | T | T | OK |
| 30 | `c2MulrvT` | T | T | OK |
| 31 | `c2Mulvs` | T | T | OK |
| 32 | `c2Mulxv` | T | T | OK |
| 33 | `c2MulxvT` | T | T | OK |
| 34 | `c2Neg` | T | T | OK |
| 35 | `c2Norm` | T | T | OK |
| 36 | `c2Norms` | T | T | OK |
| 37 | `c2PlaneAt` | T | T | OK |
| 38 | `c2RotIdentity` | T | T | OK |
| 39 | `c2Skew` | T | T | OK |
| 40 | `c2Sub` | T | T | OK |
| 41 | `c2Support` | T | T | OK |
| 42 | `c2V` | T | T | OK |
| 43 | `c2Witness` | T | T | OK |
| 44 | `c2xIdentity` | T | T | OK |
| 45 | `omni_manifold` | T | T | OK |
| 46 | `ptr_from_parts` | T | T | OK |

## Undefined (imported) symbols

C imports only `malloc` and `sqrtf` (plus CRT glue). Rust imports `malloc`
plus the Rust `std` runtime's libc surface (`memcpy`, `_Unwind_*`, ...). All
Rust undefined symbols are libc / libgcc-unwind: **0 missing non-libc
symbols**. `sqrtf` is inlined by rustc to the `sqrtss` instruction, which is
the same IEEE-754 correctly-rounded operation glibc's `sqrtf` performs.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` section**, so the only
build configuration is the default one. Verified:

```sh
$ grep -c '\[features\]' translation/Cargo.toml
0
```

Phase D's "repeat for every feature combination" therefore reduces to the
single default configuration. `translation/verify.sh` still enumerates the
power set mechanically (so the check keeps working if features are ever added)
and runs the full suite for each combination in **both** the `release` and
`debug` profiles — the profile matters because `panic = "abort"` and the
optimiser only apply to `release`, and because unoptimized codegen chooses
different registers. Latest run:

```
combinations to verify: 2      (--all-features, <default>)
symbols: C=46 Rust=46          (all 4 combination x profile passes)
ALL PHASE A-D CHECKS PASSED
```
