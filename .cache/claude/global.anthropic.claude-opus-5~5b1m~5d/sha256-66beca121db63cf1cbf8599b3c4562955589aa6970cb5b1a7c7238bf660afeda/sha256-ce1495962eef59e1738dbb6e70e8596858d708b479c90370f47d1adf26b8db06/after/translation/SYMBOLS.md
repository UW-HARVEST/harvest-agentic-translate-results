# SYMBOLS.md — Phase A: public symbol surface

Derived mechanically from `nm -D` on both shared objects.

## Build commands

```sh
# C
cd c_src && mkdir -p build && cd build && \
  cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
# -> c_src/build/libharvest-work-RsVh18.so   (name derives from the parent dir name)

# Rust
cd translation && cargo build --release
# -> translation/target/release/libcall_predict_lib.so
```

## C `.so` dynamic symbol table (`nm -D`)

```
                 w _ITM_deregisterTMCloneTable
                 w _ITM_registerTMCloneTable
                 w __cxa_finalize@GLIBC_2.2.5
                 w __gmon_start__
00000000000024c8 T call_predict
```

`nm -D --defined-only` → exactly **one** defined, exported symbol:

| # | symbol | type | exported by Rust `.so`? | notes |
|---|--------|------|-------------------------|-------|
| 1 | `call_predict` | `T` (global text) | **YES** (`T call_predict`) | `#[unsafe(no_mangle)] pub extern "C" fn call_predict(c_int) -> c_int` |

The four `w` (weak/undefined) entries are toolchain/glibc artifacts
(`_ITM_*`, `__cxa_finalize`, `__gmon_start__`), not library API.

## Rust `.so` dynamic symbol table (`nm -D --defined-only`)

```
0000000000011c40 T call_predict
```

## Symbol diff

```
$ comm -23 <(nm -D --defined-only C.so   | awk '{print $NF}' | sort) \
           <(nm -D --defined-only rust.so| awk '{print $NF}' | sort)
(empty)
```

**Symbols exported by C but missing from Rust: 0.** ✅

## Rust undefined (`nm -D -u`) — all libc / libgcc-unwind, no missing crate symbols

`_Unwind_*` (GCC_3.0/3.3/4.2), `__cxa_finalize`, `__cxa_thread_atexit_impl`,
`__errno_location`, `__tls_get_addr`, `abort`, `bcmp`, `calloc`, `close`,
`dl_iterate_phdr`, `free`, `fstat64`, `getcwd`, `getenv`, `gettid`, `lseek64`,
`malloc`, `memcpy`, `memmove`, `memset`, `mmap64`, `munmap`, `open64`,
`posix_memalign`, `pthread_key_{create,delete}`, `pthread_setspecific`, `read`,
`readlink`, `realloc`, `realpath`, `stat64`, `statx`, `strlen`, `syscall`,
`write`, `writev`, `_ITM_*`, `__gmon_start__`.

**0 missing/undefined non-libc symbols.** ✅

## Internal (non-exported) symbols — for reference / deep testing

`c_src/include/lib.h` declares `int get_predict_func(int pfcn);` but `lib.c`
never defines it and never calls it, so it produces **no** symbol in the C
`.so` (verified: absent from both `nm -D` and `nm`). It is therefore *not* part
of the ABI surface and must **not** be exported by the Rust `.so` either.

Everything else in `lib.c` is `static`, so it is a local (`t`) symbol only.
Both builds keep them in `.symtab` (the Rust *debug* build does; the Rust
*release* build inlines/DCEs them because the pointer comparisons in
`call_predict` constant-fold — behaviour is unchanged, only the local symbols
disappear). `tests/internal_predictors.rs` resolves these local symbols by
`nm` offset + runtime load base so the predictor arithmetic itself can also be
differentially tested even though it is not part of the public ABI.

| C local symbol | Rust local symbol (debug, mangled) |
|---|---|
| `BTAC1C2_PredictSample`       | `_ZN16call_predict_lib21BTAC1C2_PredictSample17h...E` |
| `BTAC1C2_PredictSample_Pfn0`  | `_ZN16call_predict_lib26BTAC1C2_PredictSample_Pfn017h...E` |
| `BTAC1C2_PredictSample_Pfn1`  | `..._Pfn1...` |
| `BTAC1C2_PredictSample_Pfn2`  | `..._Pfn2...` |
| `BTAC1C2_PredictSample_Pfn3`  | `..._Pfn3...` |
| `BTAC1C2_PredictSample_Pfn4`  | `..._Pfn4...` |
| `BTAC1C2_PredictSample_Pfn5`  | `..._Pfn5...` |
| `BTAC1C2_PredictSample_Pfn6`  | `..._Pfn6...` |
| `BTAC1C2_PredictSample_Pfn7`  | `..._Pfn7...` |
| `BTAC1C2_PredictSample_Pfn8`  | `..._Pfn8...` |
| `BTAC1C2_PredictSample_Pfn9`  | `..._Pfn9...` |
| `BTAC1C2_PredictSample_Pfn10` | `..._Pfn10...` |
| `BTAC1C2_PredictSample_Pfn11` | `..._Pfn11...` |
| `BTAC1C2_GetPredictFunc`      | `_ZN16call_predict_lib22BTAC1C2_GetPredictFunc17h...E` |

## Cargo feature surface

`translation/Cargo.toml` declares **no** `[features]` table, so the only
feature combinations that exist are `--features ""` (default, which is empty)
and `--no-default-features`. Both are exercised by
`run_all_feature_combos.sh`.
