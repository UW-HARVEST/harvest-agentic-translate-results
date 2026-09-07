# SYMBOLS.md — Public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## C `.so`

Built with:
```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
```
Artifact: `c_src/build/libharvest-work-HnrXsH.so`

```
$ nm -D c_src/build/libharvest-work-HnrXsH.so
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
                 U sqrtf@GLIBC_2.2.5
0000000000001109 T tfm
```

## Rust `.so`

Artifact: `translation/target/release/libtfm_lib.so`

```
$ nm -D translation/target/release/libtfm_lib.so | grep -v ' U '
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __cxa_thread_atexit_impl@GLIBC_2.18
                 w __gmon_start__
                 w gettid@GLIBC_2.30
                 w statx@GLIBC_2.28
0000000000011710 T tfm
```

## Parity table

| # | symbol | type | in C `.so` | in Rust `.so` | notes |
|---|--------|------|-----------|--------------|-------|
| 1 | `tfm`  | `T` (defined, global) | yes | yes | `#[unsafe(no_mangle)] pub unsafe extern "C" fn tfm` in `src/lib.rs` |

### Undefined / imported symbols

| symbol | C | Rust | notes |
|--------|---|------|-------|
| `sqrtf@GLIBC_2.2.5` | `U` | not imported | libc import. Rust uses `f32::sqrt` (lowers to `sqrtss`), plus explicit NaN / negative-argument handling that reproduces glibc `sqrtf`'s observable results. Not a public-surface symbol, so no parity requirement. |
| `_ITM_*`, `__gmon_start__`, `__cxa_finalize` | `w` | `w` | toolchain/CRT weak symbols, identical class |
| `__cxa_thread_atexit_impl`, `gettid`, `statx` | — | `w` | extra *weak* libc references pulled in by the Rust `std` runtime. Not exported API; they do not affect the public surface. |

### Symbol diff

```
$ diff <(nm -D --defined-only c_src/build/lib*.so   | awk '$2=="T"{print $3}' | sort) \
       <(nm -D --defined-only translation/target/release/libtfm_lib.so | awk '$2=="T"{print $3}' | sort)
(no output)
```

**Result: 0 missing symbols. Symbol diff is EMPTY.**

The whole public API of the C library is one function, declared in
`c_src/include/lib.h`:

```c
void tfm(float *dest, const float *src, int count);
```

There are no macro-generated symbols, no additional translation units
(`CMakeLists.txt` lists only `src/lib.c`), no global/static data, and no
`static` helper functions in the C source. Nothing was skipped.

---

## Verification result

Checked by `tests/phase_d_parity.rs::symbol_diff_is_empty` (runs `nm -D` itself)
and by `verify.sh`, for **all four build configurations**
(default / `--no-default-features`, × debug / release):

- [x] symbol diff empty in every configuration
- [x] `nm -D` shows 0 missing symbols in the Rust `.so`
- [x] `RTLD_NOW` `dlopen` of the Rust `.so` succeeds, proving 0 unresolvable
      non-libc undefined symbols
      (`tests/phase_d_parity.rs::rust_so_has_no_unresolvable_dependencies`)
