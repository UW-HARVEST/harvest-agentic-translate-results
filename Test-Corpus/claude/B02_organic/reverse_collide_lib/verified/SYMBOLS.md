# SYMBOLS.md — dynamic symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from:
```
nm -D --defined-only c_src/build/libtranslated_rust.so        # C
nm -D --defined-only target/debug/libreverse_collide_lib.so   # Rust
```

C .so defined symbols: `38`  |  Rust .so defined symbols: `38`

| # | C symbol | present in Rust `.so` | exercised by a differential test |
|---|----------|----------------------|-------|
| 1 | `c22` | YES | YES |
| 2 | `c23` | YES | YES |
| 3 | `c2AABBtoAABB` | YES | YES |
| 4 | `c2AABBtoCapsule` | YES | YES |
| 5 | `c2Add` | YES | YES |
| 6 | `c2BBVerts` | YES | YES |
| 7 | `c2CCW90` | YES | YES |
| 8 | `c2CapsuletoCapsule` | YES | YES |
| 9 | `c2CircletoAABB` | YES | YES |
| 10 | `c2CircletoCapsule` | YES | YES |
| 11 | `c2CircletoCircle` | YES | YES |
| 12 | `c2Clampv` | YES | YES |
| 13 | `c2Collided` | YES | YES |
| 14 | `c2D` | YES | YES |
| 15 | `c2Det2` | YES | YES |
| 16 | `c2Div` | YES | YES |
| 17 | `c2Dot` | YES | YES |
| 18 | `c2GJK` | YES | YES |
| 19 | `c2GJKSimplexMetric` | YES | YES |
| 20 | `c2L` | YES | YES |
| 21 | `c2Len` | YES | YES |
| 22 | `c2MakeProxy` | YES | YES |
| 23 | `c2Maxv` | YES | YES |
| 24 | `c2Minv` | YES | YES |
| 25 | `c2Mulrv` | YES | YES |
| 26 | `c2MulrvT` | YES | YES |
| 27 | `c2Mulvs` | YES | YES |
| 28 | `c2Mulxv` | YES | YES |
| 29 | `c2Neg` | YES | YES |
| 30 | `c2Norm` | YES | YES |
| 31 | `c2RotIdentity` | YES | YES |
| 32 | `c2Skew` | YES | YES |
| 33 | `c2Sub` | YES | YES |
| 34 | `c2Support` | YES | YES |
| 35 | `c2V` | YES | YES |
| 36 | `c2Witness` | YES | YES |
| 37 | `c2xIdentity` | YES | YES |
| 38 | `reverse_collide` | YES | YES |

## Diff result

```
$ comm -23 c_syms rust_syms   # in C, missing from Rust
(empty)

$ comm -13 c_syms rust_syms   # Rust-only extras
(empty)
```

**0 missing symbols. 0 extra symbols.**

## Undefined symbols in the Rust `.so`

All undefined imports are libc / libgcc-unwind / Rust-runtime primitives
(`memcpy`, `malloc`, `_Unwind_*`, `__errno_location`, `pthread_*`, ...).
There are **0 undefined non-libc symbols**, i.e. no unresolved translation-unit
references. The C `.so` imports only `sqrtf` from libm; the Rust `.so` implements
that inline via the `sqrtss` instruction (`f32::sqrt`), which is the same
IEEE-754 correctly-rounded operation glibc's `sqrtf` performs.

```
_ITM_deregisterTMCloneTable _ITM_registerTMCloneTable _Unwind_Backtrace@GCC_3.3 
_Unwind_GetDataRelBase@GCC_3.0 _Unwind_GetIP@GCC_3.0 _Unwind_GetIPInfo@GCC_4.2.0 
_Unwind_GetLanguageSpecificData@GCC_3.0 _Unwind_GetRegionStart@GCC_3.0 
_Unwind_GetTextRelBase@GCC_3.0 _Unwind_RaiseException@GCC_3.0 _Unwind_Resume@GCC_3.0 
_Unwind_SetGR@GCC_3.0 _Unwind_SetIP@GCC_3.0 __cxa_finalize@GLIBC_2.2.5 
__cxa_thread_atexit_impl@GLIBC_2.18 __errno_location@GLIBC_2.2.5 __gmon_start__ 
__tls_get_addr@GLIBC_2.3 abort@GLIBC_2.2.5 bcmp@GLIBC_2.2.5 calloc@GLIBC_2.2.5 close@GLIBC_2.2.5 
dl_iterate_phdr@GLIBC_2.2.5 free@GLIBC_2.2.5 fstat64@GLIBC_2.33 getcwd@GLIBC_2.2.5 
getenv@GLIBC_2.2.5 gettid@GLIBC_2.30 lseek64@GLIBC_2.2.5 malloc@GLIBC_2.2.5 memcpy@GLIBC_2.14 
memmove@GLIBC_2.2.5 memset@GLIBC_2.2.5 mmap64@GLIBC_2.2.5 munmap@GLIBC_2.2.5 open64@GLIBC_2.2.5 
posix_memalign@GLIBC_2.2.5 pthread_key_create@GLIBC_2.34 pthread_key_delete@GLIBC_2.34 
pthread_setspecific@GLIBC_2.34 read@GLIBC_2.2.5 readlink@GLIBC_2.2.5 realloc@GLIBC_2.2.5 
realpath@GLIBC_2.3 stat64@GLIBC_2.33 statx@GLIBC_2.28 strlen@GLIBC_2.2.5 syscall@GLIBC_2.2.5 
write@GLIBC_2.2.5 writev@GLIBC_2.2.5 
```

## Completeness

No C source file was skipped: `c_src/` contains exactly one translation unit
(`src/lib.c`, 646 lines) and one header (`include/lib.h`, 1 line), and all 38
of its external definitions are present in `src/lib.rs` as real translations —
**no stubs, no `unimplemented!()`, no `todo!()`**:

```
$ grep -c 'unsafe(no_mangle)' src/lib.rs
38
$ grep -cE 'unimplemented!|todo!|unreachable!|panic!' src/lib.rs
0
```

The Rust `.so` also exports **no extra** symbols, so the two ABIs are
interchangeable in both directions.

`nm -D` parity is additionally re-checked from inside the test suite
(`a04_nm_dynamic_symbol_parity`) and by `./verify_all_features.sh`, so it is
re-verified for every feature combination and both build profiles rather than
being a one-off measurement.

### Note on `sqrtf`

`c_src/CMakeLists.txt` never links `-lm`, so the C `.so` carries an unresolved
`sqrtf` that must be satisfied by the process's global symbol scope. The test
harness publishes libm with `dlopen("libm.so.6", RTLD_NOW | RTLD_GLOBAL)` before
loading the C library — a property of the harness only; nothing in `c_src/` is
modified. `a06_sqrtf_domain_parity` then confirms that glibc's `sqrtf` and
Rust's `f32::sqrt` agree bit-exactly, including NaN payloads and signed zeros.
