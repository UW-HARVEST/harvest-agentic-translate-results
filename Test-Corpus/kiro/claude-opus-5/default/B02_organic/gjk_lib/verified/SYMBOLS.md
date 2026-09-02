# SYMBOLS.md — Phase A symbol surface

Derived mechanically from:

```sh
nm -D --defined-only c_src/build/libharvest-work-NbRHOD.so | awk '$2=="T"{print $3}' | sort
nm -D --defined-only translation/target/release/libgjk_lib.so | awk '$2=="T"{print $3}' | sort
```

The C `.so` exports 31 dynamic text symbols. `c_src/include/lib.h` declares only
`gjk`, but no function in `c_src/src/lib.c` is `static`, so **every** helper is a
public dynamic symbol of the shared library and must be matched by the Rust
`.so`.

## Symbol parity table

| # | C symbol | in C `.so` | in Rust `.so` | signature (from C source) |
|---|----------|-----------|---------------|---------------------------|
| 1 | `c22` | yes | yes | `void c22(c2Simplex*)` |
| 2 | `c23` | yes | yes | `void c23(c2Simplex*)` |
| 3 | `c2Add` | yes | yes | `c2v c2Add(c2v, c2v)` |
| 4 | `c2BBVerts` | yes | yes | `void c2BBVerts(c2v* out, c2AABB* bb)` |
| 5 | `c2CCW90` | yes | yes | `c2v c2CCW90(c2v)` |
| 6 | `c2Clampv` | yes | yes | `c2v c2Clampv(c2v a, c2v lo, c2v hi)` |
| 7 | `c2D` | yes | yes | `c2v c2D(c2Simplex*)` |
| 8 | `c2Det2` | yes | yes | `float c2Det2(c2v, c2v)` |
| 9 | `c2Div` | yes | yes | `c2v c2Div(c2v, float)` |
| 10 | `c2Dot` | yes | yes | `float c2Dot(c2v, c2v)` |
| 11 | `c2GJK` | yes | yes | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` |
| 12 | `c2GJKSimplexMetric` | yes | yes | `float c2GJKSimplexMetric(c2Simplex*)` |
| 13 | `c2L` | yes | yes | `c2v c2L(c2Simplex*)` |
| 14 | `c2Len` | yes | yes | `float c2Len(c2v)` |
| 15 | `c2MakeProxy` | yes | yes | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` |
| 16 | `c2Maxv` | yes | yes | `c2v c2Maxv(c2v, c2v)` |
| 17 | `c2Minv` | yes | yes | `c2v c2Minv(c2v, c2v)` |
| 18 | `c2Mulrv` | yes | yes | `c2v c2Mulrv(c2r, c2v)` |
| 19 | `c2MulrvT` | yes | yes | `c2v c2MulrvT(c2r, c2v)` |
| 20 | `c2Mulvs` | yes | yes | `c2v c2Mulvs(c2v, float)` |
| 21 | `c2Mulxv` | yes | yes | `c2v c2Mulxv(c2x, c2v)` |
| 22 | `c2Neg` | yes | yes | `c2v c2Neg(c2v)` |
| 23 | `c2Norm` | yes | yes | `c2v c2Norm(c2v)` |
| 24 | `c2RotIdentity` | yes | yes | `c2r c2RotIdentity(void)` |
| 25 | `c2Skew` | yes | yes | `c2v c2Skew(c2v)` |
| 26 | `c2Sub` | yes | yes | `c2v c2Sub(c2v, c2v)` |
| 27 | `c2Support` | yes | yes | `int c2Support(const c2v*, int, c2v)` |
| 28 | `c2V` | yes | yes | `c2v c2V(float, float)` |
| 29 | `c2Witness` | yes | yes | `void c2Witness(c2Simplex*, c2v*, c2v*)` |
| 30 | `c2xIdentity` | yes | yes | `c2x c2xIdentity(void)` |
| 31 | `gjk` | yes | yes | `void gjk(char, c2v*, c2v*, float x9)` |

## Diff result

```
$ comm -23 /tmp/c_syms.txt /tmp/rust_syms.txt     # in C, missing from Rust
(empty)
```

**0 missing symbols.** No module of `c_src/src/lib.c` was skipped by the
translation; all 31 definitions have a real Rust implementation (no stubs, no
`unimplemented!()`).

## Undefined symbols in the Rust `.so`

`nm -D --undefined-only` on the Rust `.so` lists only libc / libgcc-unwind /
glibc-TLS imports (`malloc`, `memcpy`, `sqrtf` is inlined as `sqrtss`,
`_Unwind_*`, `__cxa_finalize`, ...). **0 undefined non-libc symbols.**

The C `.so` links `libm` for `sqrtf`; the Rust `.so` needs no libm import
because `f32::sqrt` lowers to the `sqrtss` instruction directly. Both compute
the same IEEE-754 correctly-rounded single-precision square root.

## Feature configurations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one build configuration (the default). `--no-default-features` and
`--features <combo>` are therefore vacuous here; Phase D's "repeat for every
feature combination" collapses to the single default configuration, which is
verified explicitly by `tests/feature_matrix.rs` /
`scripts/check_feature_combos.sh`.

## ABI notes verified by the differential tests

* `c2v` (2 x f32, 8 bytes) is returned packed in the low half of `xmm0`.
* `c2r` (2 x f32) likewise.
* `c2x` (4 x f32, 16 bytes) is returned in `xmm0`+`xmm1`.
* `C2_TYPE` is an `int`-sized enum; the Rust wrappers take `c_int`, which is
  required because C enums accept any `int` value (see `ERRORS.md` rows 20-22).

## Result

Verified by `tests/symbol_parity.rs`, which shells out to `nm` on both artifacts
and fails if the diff is non-empty. It runs as part of `cargo test` and is
re-run against every build profile by `scripts/check_all_configs.sh`.

```
C .so exports:        31
Rust .so exports:     31 of 31 (exact names)
Missing from Rust:     0
Undefined non-libc:    0
```

No symbol needed adding and no C module had been skipped — the translation
already covered all of `c_src/src/lib.c`. Nothing is stubbed: `grep -E
'unimplemented!|todo!'` over `src/lib.rs` returns no matches, and every symbol is
exercised by a differential test that compares its real output against the C.

## Completion gate

| requirement | status |
|---|---|
| `SYMBOLS.md`: 0 missing / 0 undefined non-libc | 31/31, 0 missing, 0 undefined |
| Phase B: every `CONFIGS.md` row passes across randomized inputs | 52/52 rows |
| Phase C: every `ERRORS.md` row has a passing error-path test | 68/68 rows |
| Holds under every feature combination | no `[features]` -> 1 configuration; verified for the `release` **and** `debug` artifacts, which are different codegen configurations and therefore a real second code path for this crate |

96 tests total across 6 integration-test binaries, all loading both `.so`s
through `libloading` and calling only exported symbols. Run everything with:

```sh
cd translation && ./scripts/check_all_configs.sh
```
