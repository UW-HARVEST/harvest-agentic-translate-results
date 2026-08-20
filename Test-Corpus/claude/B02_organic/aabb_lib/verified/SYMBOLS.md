# SYMBOLS.md — exported-symbol parity (Phase A / Phase D)

Mechanically derived, not hand-written.

Reference (C):  `c_src/build/libtranslated_rust.so`  (built by
`cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .`)
Translation (Rust): `target/debug/libaabb_lib.so` (`crate-type = ["cdylib"]`)

Regenerate / re-verify with:

```sh
./check_symbols.sh
```

## Build configurations

`Cargo.toml` has **no `[features]` section**, and `c_src/CMakeLists.txt` has no
`option()`, no `add_definitions`, no `target_compile_definitions` and the single
source file contains **no `#ifdef`/`#if` at all** (`grep -c '^\s*#\s*if' c_src/src/lib.c` == 0).
Therefore there is exactly **one** build configuration:

| # | feature combination | cargo invocation | status |
|---|---------------------|------------------|--------|
| 1 | *(none — default == no-default == all-features)* | `cargo test --no-default-features` / `cargo test --all-features` / `cargo test` | verified |

## Symbol table

All 38 symbols exported by the C `.so` (`nm -D --defined-only`), with the
corresponding entry in the Rust `.so`.

| # | symbol | C type | Rust type | exported by Rust? |
|---|--------|--------|-----------|-------------------|
| 1 | `aabb` | `T` | `T` | yes |
| 2 | `c22` | `T` | `T` | yes |
| 3 | `c23` | `T` | `T` | yes |
| 4 | `c2AABBtoAABB` | `T` | `T` | yes |
| 5 | `c2AABBtoCapsule` | `T` | `T` | yes |
| 6 | `c2Add` | `T` | `T` | yes |
| 7 | `c2BBVerts` | `T` | `T` | yes |
| 8 | `c2CCW90` | `T` | `T` | yes |
| 9 | `c2CapsuletoCapsule` | `T` | `T` | yes |
| 10 | `c2CircletoAABB` | `T` | `T` | yes |
| 11 | `c2CircletoCapsule` | `T` | `T` | yes |
| 12 | `c2CircletoCircle` | `T` | `T` | yes |
| 13 | `c2Clampv` | `T` | `T` | yes |
| 14 | `c2Collided` | `T` | `T` | yes |
| 15 | `c2D` | `T` | `T` | yes |
| 16 | `c2Det2` | `T` | `T` | yes |
| 17 | `c2Div` | `T` | `T` | yes |
| 18 | `c2Dot` | `T` | `T` | yes |
| 19 | `c2GJK` | `T` | `T` | yes |
| 20 | `c2GJKSimplexMetric` | `T` | `T` | yes |
| 21 | `c2L` | `T` | `T` | yes |
| 22 | `c2Len` | `T` | `T` | yes |
| 23 | `c2MakeProxy` | `T` | `T` | yes |
| 24 | `c2Maxv` | `T` | `T` | yes |
| 25 | `c2Minv` | `T` | `T` | yes |
| 26 | `c2Mulrv` | `T` | `T` | yes |
| 27 | `c2MulrvT` | `T` | `T` | yes |
| 28 | `c2Mulvs` | `T` | `T` | yes |
| 29 | `c2Mulxv` | `T` | `T` | yes |
| 30 | `c2Neg` | `T` | `T` | yes |
| 31 | `c2Norm` | `T` | `T` | yes |
| 32 | `c2RotIdentity` | `T` | `T` | yes |
| 33 | `c2Skew` | `T` | `T` | yes |
| 34 | `c2Sub` | `T` | `T` | yes |
| 35 | `c2Support` | `T` | `T` | yes |
| 36 | `c2V` | `T` | `T` | yes |
| 37 | `c2Witness` | `T` | `T` | yes |
| 38 | `c2xIdentity` | `T` | `T` | yes |

## Result

* C exports: **38**  Rust exports: **38**
* `comm -23 c.syms r.syms` (in C, missing from Rust) → **empty**
* `comm -13 c.syms r.syms` (extra in Rust) → **empty**
* No symbol needed a new `#[no_mangle]` wrapper and no C source file was left
  untranslated: `c_src` contains exactly one translation unit (`src/lib.c`,
  645 lines) and every function with external linkage in it has a
  `#[unsafe(no_mangle)] pub extern "C"` counterpart in `src/lib.rs`.

### Undefined symbols in the Rust `.so`

`nm -D --undefined-only target/debug/libaabb_lib.so` lists only libc / libgcc
runtime imports (`memcpy`, `malloc`, `_Unwind_*`, `__cxa_finalize`, …) that the
Rust standard library pulls in. **0 undefined non-libc symbols.**

The C `.so` additionally imports `sqrtf@GLIBC` (from `-lm`), while the Rust port
has **no** `sqrt` import at all: it uses `f32::sqrt`, which lowers to the
`sqrtss` hardware instruction (confirmed with
`objdump -d target/debug/libaabb_lib.so`). Square root is exactly rounded in
IEEE-754, so glibc's `sqrtf` and `sqrtss` return identical bits; the difference
is therefore behaviour-neutral, and it is covered directly by the `c2Len` /
`c2Norm` differential tests (`b08_c2Len`, `b11_c2Norm`, `e24`, `e25`, `e28`),
which include `FLT_MAX` overflow, denormal underflow, `±0`, `±inf` and `NaN`.

[Full report: see `VERIFICATION.md` for the Phase A-D completion gate.]
