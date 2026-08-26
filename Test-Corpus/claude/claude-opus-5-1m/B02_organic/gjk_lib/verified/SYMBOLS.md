# SYMBOLS.md — Exported symbol parity (Phase A / Phase D)

Derived mechanically:

```sh
nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > c_syms.txt
nm -D --defined-only target/release/libgjk_lib.so      | awk '{print $3}' | sort > r_syms.txt
comm -23 c_syms.txt r_syms.txt          # C exports missing from Rust  -> EMPTY
```

## Build configurations

`Cargo.toml` declares **no `[features]` table**, and `c_src/CMakeLists.txt` has no
`option()` / `add_definitions()` / `#ifdef`-driven switches. Therefore there is
exactly **ONE** valid build configuration:

| # | combination | `cargo check --no-default-features` |
|---|-------------|--------------------------------------|
| 1 | *(no features — the only combination)* | clean |

`grep -c '^\[features\]' Cargo.toml` -> 0. `grep -c '#ifdef\|#if \|option(' c_src/src/lib.c c_src/CMakeLists.txt` -> 0.

## Symbol table (31 C exports)

| symbol | in C `.so` | in Rust `.so` | status |
|--------|-----------|---------------|--------|
| `c22` | yes | yes | OK |
| `c23` | yes | yes | OK |
| `c2Add` | yes | yes | OK |
| `c2BBVerts` | yes | yes | OK |
| `c2CCW90` | yes | yes | OK |
| `c2Clampv` | yes | yes | OK |
| `c2D` | yes | yes | OK |
| `c2Det2` | yes | yes | OK |
| `c2Div` | yes | yes | OK |
| `c2Dot` | yes | yes | OK |
| `c2GJK` | yes | yes | OK |
| `c2GJKSimplexMetric` | yes | yes | OK |
| `c2L` | yes | yes | OK |
| `c2Len` | yes | yes | OK |
| `c2MakeProxy` | yes | yes | OK |
| `c2Maxv` | yes | yes | OK |
| `c2Minv` | yes | yes | OK |
| `c2Mulrv` | yes | yes | OK |
| `c2MulrvT` | yes | yes | OK |
| `c2Mulvs` | yes | yes | OK |
| `c2Mulxv` | yes | yes | OK |
| `c2Neg` | yes | yes | OK |
| `c2Norm` | yes | yes | OK |
| `c2RotIdentity` | yes | yes | OK |
| `c2Skew` | yes | yes | OK |
| `c2Sub` | yes | yes | OK |
| `c2Support` | yes | yes | OK |
| `c2V` | yes | yes | OK |
| `c2Witness` | yes | yes | OK |
| `c2xIdentity` | yes | yes | OK |
| `gjk` | yes | yes | OK |

**Total: 31 C exports, 31 present in the Rust `.so`. Symbol diff is EMPTY.**

No symbol required a new translation: `c_src` is a single translation unit
(`src/lib.c`, 530 lines) and every function with external linkage in it was
already translated in `src/lib.rs`. Nothing is stubbed or `unimplemented!()`.

## Undefined-symbol check

Rust `.so` undefined symbols are all libc / libgcc-unwind imports
(`malloc`, `memcpy`, `sqrtf`-equivalent inlined, `_Unwind_*`, `__cxa_finalize`, ...).
**0 missing/undefined non-libc symbols.**

```sh
nm -D --undefined-only target/release/libgjk_lib.so | awk '{print $2}' \
  | grep -v '@GLIBC\|@GCC\|_ITM_\|__gmon_start__'   # -> EMPTY
```

## Notes on the C ABI surface

`include/lib.h` declares only `c2v` and `gjk()`. The other 30 symbols are
non-`static` definitions in `src/lib.c` that therefore get external linkage and
appear in `nm -D`; they are part of the tested ABI. Struct-by-value returns
(`c2v`, `c2r`, `c2x`) are all-`float` aggregates <= 16 bytes, so SysV AMD64
classifies them SSE and returns them in `XMM0`(/`XMM1`); `#[repr(C)]` Rust
structs of the same shape match this exactly and are exercised through
`libloading` in the differential tests.

## Phase D result

Symbol diff re-run after all test work (see `VERIFICATION.md` for the full report):

```
$ nm -D --defined-only c_src/build/libtranslated_rust.so | awk '{print $3}' | sort > c_syms
$ nm -D --defined-only target/release/libgjk_lib.so      | awk '{print $3}' | sort > r_syms
$ comm -23 c_syms r_syms
(empty)
$ wc -l < c_syms
31
```

* 31 / 31 C exports present in the Rust `.so`, exact names.
* 0 undefined non-libc symbols in the Rust `.so`.
* 1 / 1 feature combinations verified (`./check_all_features.sh`).
* 149 differential tests, 0 failures.
* Mutation score 18 killed / 2 provably-equivalent / 0 unexplained survivors
  (`./mutation_check.sh`).

The Rust `.so` also exports the usual Rust runtime/metadata symbols
(`rust_eh_personality`, `_ZN*` monomorphisations, `__rust_*` allocator shims).
Those are *additional* to the C surface and harmless — the gate is that no C
symbol is missing, which holds.
