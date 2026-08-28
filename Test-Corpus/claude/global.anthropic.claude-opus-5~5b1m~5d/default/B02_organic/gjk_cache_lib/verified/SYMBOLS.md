# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libharvest-work-XWA1IC.so | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only translation/target/release/libgjk_cache_lib.so | awk '{print $3}' | sort > rust_syms.txt
comm -23 c_syms.txt rust_syms.txt   # missing from Rust  -> MUST be empty
comm -13 c_syms.txt rust_syms.txt   # extra in Rust      -> informational
```

The C `.so` (`libharvest-work-XWA1IC.so`, built from the single TU
`c_src/src/lib.c` at `-O0 -fPIC`) exports **31** global symbols. There are no
macro-generated symbols and no other translation units — `src/lib.c` is the
whole library, so there is no "missing module" to translate.

Result of the diff: **0 missing, 0 extra.** Both lists are byte-identical.

| # | C symbol | C signature (from `c_src/src/lib.c`) | Rust `#[no_mangle]` item | present in Rust `.so` |
|---|----------|--------------------------------------|--------------------------|-----------------------|
| 1 | `c2V` | `c2v c2V(float, float)` | `c2V` | yes |
| 2 | `c2Mulvs` | `c2v c2Mulvs(c2v, float)` | `c2Mulvs` | yes |
| 3 | `c2Maxv` | `c2v c2Maxv(c2v, c2v)` | `c2Maxv` | yes |
| 4 | `c2Minv` | `c2v c2Minv(c2v, c2v)` | `c2Minv` | yes |
| 5 | `c2Clampv` | `c2v c2Clampv(c2v, c2v, c2v)` | `c2Clampv` | yes |
| 6 | `c2Sub` | `c2v c2Sub(c2v, c2v)` | `c2Sub` | yes |
| 7 | `c2Dot` | `float c2Dot(c2v, c2v)` | `c2Dot` | yes |
| 8 | `c2RotIdentity` | `c2r c2RotIdentity(void)` | `c2RotIdentity` | yes |
| 9 | `c2xIdentity` | `c2x c2xIdentity(void)` | `c2xIdentity` | yes |
| 10 | `c2BBVerts` | `void c2BBVerts(c2v*, c2AABB*)` | `c2BBVerts` | yes |
| 11 | `c2MakeProxy` | `void c2MakeProxy(const void*, C2_TYPE, c2Proxy*)` | `c2MakeProxy` | yes |
| 12 | `c2Len` | `float c2Len(c2v)` | `c2Len` | yes |
| 13 | `c2Det2` | `float c2Det2(c2v, c2v)` | `c2Det2` | yes |
| 14 | `c2GJKSimplexMetric` | `float c2GJKSimplexMetric(c2Simplex*)` | `c2GJKSimplexMetric` | yes |
| 15 | `c2Mulrv` | `c2v c2Mulrv(c2r, c2v)` | `c2Mulrv` | yes |
| 16 | `c2Add` | `c2v c2Add(c2v, c2v)` | `c2Add` | yes |
| 17 | `c2Mulxv` | `c2v c2Mulxv(c2x, c2v)` | `c2Mulxv` | yes |
| 18 | `c22` | `void c22(c2Simplex*)` | `c22` | yes |
| 19 | `c23` | `void c23(c2Simplex*)` | `c23` | yes |
| 20 | `c2Neg` | `c2v c2Neg(c2v)` | `c2Neg` | yes |
| 21 | `c2Skew` | `c2v c2Skew(c2v)` | `c2Skew` | yes |
| 22 | `c2CCW90` | `c2v c2CCW90(c2v)` | `c2CCW90` | yes |
| 23 | `c2D` | `c2v c2D(c2Simplex*)` | `c2D` | yes |
| 24 | `c2Support` | `int c2Support(const c2v*, int, c2v)` | `c2Support` | yes |
| 25 | `c2Witness` | `void c2Witness(c2Simplex*, c2v*, c2v*)` | `c2Witness` | yes |
| 26 | `c2Div` | `c2v c2Div(c2v, float)` | `c2Div` | yes |
| 27 | `c2Norm` | `c2v c2Norm(c2v)` | `c2Norm` | yes |
| 28 | `c2L` | `c2v c2L(c2Simplex*)` | `c2L` | yes |
| 29 | `c2MulrvT` | `c2v c2MulrvT(c2r, c2v)` | `c2MulrvT` | yes |
| 30 | `c2GJK` | `float c2GJK(const void*, C2_TYPE, const c2x*, const void*, C2_TYPE, const c2x*, c2v*, c2v*, int, int*, c2GJKCache*)` | `c2GJK` | yes |
| 31 | `gjk_cache` | `void gjk_cache(char, c2v*, c2v*, float×9)` | `gjk_cache` | yes |

## Undefined (imported) symbols

| library | non-libc undefined symbols |
|---------|----------------------------|
| C `.so` | none (`sqrtf@GLIBC`, `__cxa_finalize@GLIBC`, `_ITM_*`, `__gmon_start__` are all libc / toolchain weak stubs) |
| Rust `.so` | none (all imports are glibc: `memcpy`, `malloc`, `_Unwind_*` from the std panic runtime, etc.) |

`sqrtf` is the C `.so`'s only libm import.  The Rust side does not import it:
`fp::sqrt` emits a single `sqrtss` instruction via inline asm.  `sqrtss` is
correctly rounded per IEEE-754 and quiets NaNs the same way glibc's `sqrtf`
does, so the results are bit-identical — verified over the full NaN/±inf/±0/
denormal matrix in `tests/nan_payloads.rs::nan_scalar_vector_ops` and
`tests/leaf_helpers.rs::c06_c2len`.  (`c2Dot(a, a)` is a sum of squares, so it
is never negative and glibc's `errno`-setting negative branch is unreachable
through `c2Len`.)

## Struct-layout parity (verified by compiling an equivalent C TU with gcc 11.5, x86-64 SysV)

| type | C size | C offsets | Rust repr(C) equivalent |
|------|--------|-----------|-------------------------|
| `c2v` | 8 (align 4) | x@0 y@4 | `c2v` — same |
| `c2r` | 8 | c@0 s@4 | `c2r` — same |
| `c2x` | 16 | p@0 r@8 | `c2x` — same |
| `c2Circle` | 12 | p@0 r@8 | `c2Circle` — same |
| `c2AABB` | 16 | min@0 max@8 | `c2AABB` — same |
| `c2Capsule` | 20 | a@0 b@8 r@16 | `c2Capsule` — same |
| `c2GJKCache` | 36 | metric@0 count@4 iA@8 iB@20 div@32 | `c2GJKCache` — same |
| `c2Proxy` | 72 | radius@0 count@4 verts@8 | `c2Proxy` — same |
| `c2sv` | 36 | sA@0 sB@8 p@16 u@24 iA@28 iB@32 | `c2sv` — same |
| `c2Simplex` | 152 | a@0 b@36 c@72 d@108 div@144 count@148 | `c2Simplex { verts: [c2sv;4], div, count }` — same |

`C2_TYPE` is an unpromoted C enum with values 0..2, so gcc gives it type
`unsigned int` — the Rust side takes `c_uint`, which lets out-of-range enum
values (e.g. `3`, `0xFFFF_FFFF`) cross the FFI boundary exactly as C would
accept them.

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so there is exactly
one feature configuration; the *profile* is nonetheless a real axis (see the
`[profile.dev]` comment in `Cargo.toml`).  `./verify.sh` re-checks the symbol
diff and re-runs the whole differential suite for all four combinations and
would automatically pick up any features added later:

| profile | features | symbols C / Rust | missing | tests |
|---------|----------|------------------|---------|-------|
| release | default | 31 / 31 | 0 | 113 passed |
| release | `--no-default-features` | 31 / 31 | 0 | 113 passed |
| dev | default | 31 / 31 | 0 | 113 passed |
| dev | `--no-default-features` | 31 / 31 | 0 | 113 passed |

## Completeness of the translation

`c_src` contains exactly one translation unit (`src/lib.c`, 557 lines) and one
header (`include/lib.h`, 7 lines).  Every non-`static` function in that file is
exported by the C `.so` and every one of them is implemented — not stubbed — in
`translation/src/lib.rs`; there is no `unimplemented!()`, `todo!()`, `panic!()`
or placeholder anywhere in the crate:

```sh
$ grep -c 'unimplemented!\|todo!\|panic!\|unreachable!' translation/src/lib.rs
0
```
