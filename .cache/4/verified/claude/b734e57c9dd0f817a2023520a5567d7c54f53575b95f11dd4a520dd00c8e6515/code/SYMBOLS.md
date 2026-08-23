# SYMBOLS.md — exported-symbol parity: C `.so` vs Rust `.so`

Mechanically generated from `nm -D --defined-only` on both shared objects.

```
C   .so : c_src/build/libzstd.so
Rust.so : target/release/libzstd.so
```

## Summary

| metric | value |
|---|---|
| C dynamic symbols (defined) | **615** |
| Rust dynamic symbols (defined) | **615** |
| MISSING in Rust | **0** |
| EXTRA in Rust | **0** |
| Rust undefined non-libc symbols | **0** |

**Symbol diff is EMPTY.** Every symbol the C `.so` exports is exported by the
Rust `.so` with the exact same name, and the Rust `.so` exports nothing extra.

## Undefined-symbol check (Rust `.so`)

`nm -D --undefined-only` on the Rust `.so` resolves entirely against libc /
the platform runtime — there are **no unresolved zstd symbols**:

```
_ITM_deregisterTMCloneTable         _ITM_registerTMCloneTable           _Unwind_Backtrace@GCC_3.3           _Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0               _Unwind_GetIPInfo@GCC_4.2.0         _Unwind_GetLanguageSpecificData@GCC_3.0  _Unwind_GetRegionStart@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0      _Unwind_Resume@GCC_3.0              _Unwind_SetGR@GCC_3.0               _Unwind_SetIP@GCC_3.0
__cxa_finalize@GLIBC_2.2.5          __cxa_thread_atexit_impl@GLIBC_2.18  __errno_location@GLIBC_2.2.5        __gmon_start__
__tls_get_addr@GLIBC_2.3            abort@GLIBC_2.2.5                   bcmp@GLIBC_2.2.5                    calloc@GLIBC_2.2.5
clock@GLIBC_2.2.5                   close@GLIBC_2.2.5                   dl_iterate_phdr@GLIBC_2.2.5         free@GLIBC_2.2.5
fstat64@GLIBC_2.33                  getcwd@GLIBC_2.2.5                  getenv@GLIBC_2.2.5                  gettid@GLIBC_2.30
lseek64@GLIBC_2.2.5                 malloc@GLIBC_2.2.5                  memcmp@GLIBC_2.2.5                  memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5                 memset@GLIBC_2.2.5                  mmap64@GLIBC_2.2.5                  munmap@GLIBC_2.2.5
open64@GLIBC_2.2.5                  posix_memalign@GLIBC_2.2.5          pthread_key_create@GLIBC_2.34       pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34      qsort_r@GLIBC_2.8                   read@GLIBC_2.2.5                    readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5                 realpath@GLIBC_2.3                  stat64@GLIBC_2.33                   statx@GLIBC_2.28
strlen@GLIBC_2.2.5                  syscall@GLIBC_2.2.5                 write@GLIBC_2.2.5                   writev@GLIBC_2.2.5
```

For reference the C `.so` additionally leaves `ZSTD_trace_compress_begin/end`
and `ZSTD_trace_decompress_begin/end` undefined (weak trace hooks) and pulls in
`__assert_fail`/`fprintf`/`stderr` from its debug scaffolding. The Rust build
resolves the trace hooks to `NULL` internally, matching `ZSTD_TRACE`'s weak-symbol
behaviour on gcc (all four are undefined at run time, so every trace body is a no-op).

## Run-time reachability (not just presence in a section)

`nm -D` proves the names are in the dynamic symbol table. The suite additionally
proves they are *callable*: `every_c_export_is_dlsym_reachable_in_rust` in
`tests/t00_smoke.rs` reads the 615 names out of this file, `dlsym`s each one in
the Rust `.so`, and fails on the first that does not resolve. Every other test
then reaches its functions only through `dlsym`, so the `#[no_mangle]` export
wrappers are exercised as part of the differential comparison rather than assumed.

## Symbol type breakdown

| type | C | Rust |
|---|---|---|
| `B` | 2 | 2 |
| `T` | 613 | 613 |

The two `B` (BSS) symbols are the mutable globals `g_debuglevel`
(`common/debug.c`) and `g_ZSTD_threading_useless_symbol` (`common/threading.c`).

## Full symbol table, grouped by originating C translation unit

`✔` = present in the Rust `.so` under the identical name.

### `common/debug.c` — 1 symbols, 1 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `g_debuglevel` | `B` | ✔ |

### `common/entropy_common.c` — 9 symbols, 9 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSE_getErrorName` | `T` | ✔ |
| 2 | `FSE_isError` | `T` | ✔ |
| 3 | `FSE_readNCount` | `T` | ✔ |
| 4 | `FSE_readNCount_bmi2` | `T` | ✔ |
| 5 | `FSE_versionNumber` | `T` | ✔ |
| 6 | `HUF_getErrorName` | `T` | ✔ |
| 7 | `HUF_isError` | `T` | ✔ |
| 8 | `HUF_readStats` | `T` | ✔ |
| 9 | `HUF_readStats_wksp` | `T` | ✔ |

### `common/error_private.c` — 1 symbols, 1 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ERR_getErrorString` | `T` | ✔ |

### `common/fse_decompress.c` — 2 symbols, 2 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSE_buildDTable_wksp` | `T` | ✔ |
| 2 | `FSE_decompress_wksp_bmi2` | `T` | ✔ |

### `common/pool.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `POOL_add` | `T` | ✔ |
| 2 | `POOL_create` | `T` | ✔ |
| 3 | `POOL_create_advanced` | `T` | ✔ |
| 4 | `POOL_free` | `T` | ✔ |
| 5 | `POOL_joinJobs` | `T` | ✔ |
| 6 | `POOL_resize` | `T` | ✔ |
| 7 | `POOL_sizeof` | `T` | ✔ |
| 8 | `POOL_tryAdd` | `T` | ✔ |

### `common/threading.c` — 1 symbols, 1 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `g_ZSTD_threading_useless_symbol` | `B` | ✔ |

### `common/xxhash.c` — 19 symbols, 19 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_XXH32` | `T` | ✔ |
| 2 | `ZSTD_XXH32_canonicalFromHash` | `T` | ✔ |
| 3 | `ZSTD_XXH32_copyState` | `T` | ✔ |
| 4 | `ZSTD_XXH32_createState` | `T` | ✔ |
| 5 | `ZSTD_XXH32_digest` | `T` | ✔ |
| 6 | `ZSTD_XXH32_freeState` | `T` | ✔ |
| 7 | `ZSTD_XXH32_hashFromCanonical` | `T` | ✔ |
| 8 | `ZSTD_XXH32_reset` | `T` | ✔ |
| 9 | `ZSTD_XXH32_update` | `T` | ✔ |
| 10 | `ZSTD_XXH64` | `T` | ✔ |
| 11 | `ZSTD_XXH64_canonicalFromHash` | `T` | ✔ |
| 12 | `ZSTD_XXH64_copyState` | `T` | ✔ |
| 13 | `ZSTD_XXH64_createState` | `T` | ✔ |
| 14 | `ZSTD_XXH64_digest` | `T` | ✔ |
| 15 | `ZSTD_XXH64_freeState` | `T` | ✔ |
| 16 | `ZSTD_XXH64_hashFromCanonical` | `T` | ✔ |
| 17 | `ZSTD_XXH64_reset` | `T` | ✔ |
| 18 | `ZSTD_XXH64_update` | `T` | ✔ |
| 19 | `ZSTD_XXH_versionNumber` | `T` | ✔ |

### `common/zstd_common.c` — 6 symbols, 6 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_getErrorCode` | `T` | ✔ |
| 2 | `ZSTD_getErrorName` | `T` | ✔ |
| 3 | `ZSTD_getErrorString` | `T` | ✔ |
| 4 | `ZSTD_isError` | `T` | ✔ |
| 5 | `ZSTD_versionNumber` | `T` | ✔ |
| 6 | `ZSTD_versionString` | `T` | ✔ |

### `compress/fse_compress.c` — 9 symbols, 9 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSE_NCountWriteBound` | `T` | ✔ |
| 2 | `FSE_buildCTable_rle` | `T` | ✔ |
| 3 | `FSE_buildCTable_wksp` | `T` | ✔ |
| 4 | `FSE_compressBound` | `T` | ✔ |
| 5 | `FSE_compress_usingCTable` | `T` | ✔ |
| 6 | `FSE_normalizeCount` | `T` | ✔ |
| 7 | `FSE_optimalTableLog` | `T` | ✔ |
| 8 | `FSE_optimalTableLog_internal` | `T` | ✔ |
| 9 | `FSE_writeNCount` | `T` | ✔ |

### `compress/hist.c` — 7 symbols, 7 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `HIST_add` | `T` | ✔ |
| 2 | `HIST_count` | `T` | ✔ |
| 3 | `HIST_countFast` | `T` | ✔ |
| 4 | `HIST_countFast_wksp` | `T` | ✔ |
| 5 | `HIST_count_simple` | `T` | ✔ |
| 6 | `HIST_count_wksp` | `T` | ✔ |
| 7 | `HIST_isError` | `T` | ✔ |

### `compress/huf_compress.c` — 15 symbols, 15 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `HUF_buildCTable_wksp` | `T` | ✔ |
| 2 | `HUF_cardinality` | `T` | ✔ |
| 3 | `HUF_compress1X_repeat` | `T` | ✔ |
| 4 | `HUF_compress1X_usingCTable` | `T` | ✔ |
| 5 | `HUF_compress4X_repeat` | `T` | ✔ |
| 6 | `HUF_compress4X_usingCTable` | `T` | ✔ |
| 7 | `HUF_compressBound` | `T` | ✔ |
| 8 | `HUF_estimateCompressedSize` | `T` | ✔ |
| 9 | `HUF_getNbBitsFromCTable` | `T` | ✔ |
| 10 | `HUF_minTableLog` | `T` | ✔ |
| 11 | `HUF_optimalTableLog` | `T` | ✔ |
| 12 | `HUF_readCTable` | `T` | ✔ |
| 13 | `HUF_readCTableHeader` | `T` | ✔ |
| 14 | `HUF_validateCTable` | `T` | ✔ |
| 15 | `HUF_writeCTable_wksp` | `T` | ✔ |

### `compress/zstd_compress.c` — 121 symbols, 121 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_CCtxParams_getParameter` | `T` | ✔ |
| 2 | `ZSTD_CCtxParams_init` | `T` | ✔ |
| 3 | `ZSTD_CCtxParams_init_advanced` | `T` | ✔ |
| 4 | `ZSTD_CCtxParams_registerSequenceProducer` | `T` | ✔ |
| 5 | `ZSTD_CCtxParams_reset` | `T` | ✔ |
| 6 | `ZSTD_CCtxParams_setParameter` | `T` | ✔ |
| 7 | `ZSTD_CCtx_getParameter` | `T` | ✔ |
| 8 | `ZSTD_CCtx_loadDictionary` | `T` | ✔ |
| 9 | `ZSTD_CCtx_loadDictionary_advanced` | `T` | ✔ |
| 10 | `ZSTD_CCtx_loadDictionary_byReference` | `T` | ✔ |
| 11 | `ZSTD_CCtx_refCDict` | `T` | ✔ |
| 12 | `ZSTD_CCtx_refPrefix` | `T` | ✔ |
| 13 | `ZSTD_CCtx_refPrefix_advanced` | `T` | ✔ |
| 14 | `ZSTD_CCtx_refThreadPool` | `T` | ✔ |
| 15 | `ZSTD_CCtx_reset` | `T` | ✔ |
| 16 | `ZSTD_CCtx_setCParams` | `T` | ✔ |
| 17 | `ZSTD_CCtx_setFParams` | `T` | ✔ |
| 18 | `ZSTD_CCtx_setParameter` | `T` | ✔ |
| 19 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `T` | ✔ |
| 20 | `ZSTD_CCtx_setParams` | `T` | ✔ |
| 21 | `ZSTD_CCtx_setPledgedSrcSize` | `T` | ✔ |
| 22 | `ZSTD_CCtx_trace` | `T` | ✔ |
| 23 | `ZSTD_CStreamInSize` | `T` | ✔ |
| 24 | `ZSTD_CStreamOutSize` | `T` | ✔ |
| 25 | `ZSTD_adjustCParams` | `T` | ✔ |
| 26 | `ZSTD_buildBlockEntropyStats` | `T` | ✔ |
| 27 | `ZSTD_cParam_getBounds` | `T` | ✔ |
| 28 | `ZSTD_checkCParams` | `T` | ✔ |
| 29 | `ZSTD_compress` | `T` | ✔ |
| 30 | `ZSTD_compress2` | `T` | ✔ |
| 31 | `ZSTD_compressBegin` | `T` | ✔ |
| 32 | `ZSTD_compressBegin_advanced` | `T` | ✔ |
| 33 | `ZSTD_compressBegin_advanced_internal` | `T` | ✔ |
| 34 | `ZSTD_compressBegin_usingCDict` | `T` | ✔ |
| 35 | `ZSTD_compressBegin_usingCDict_advanced` | `T` | ✔ |
| 36 | `ZSTD_compressBegin_usingCDict_deprecated` | `T` | ✔ |
| 37 | `ZSTD_compressBegin_usingDict` | `T` | ✔ |
| 38 | `ZSTD_compressBlock` | `T` | ✔ |
| 39 | `ZSTD_compressBlock_deprecated` | `T` | ✔ |
| 40 | `ZSTD_compressBound` | `T` | ✔ |
| 41 | `ZSTD_compressCCtx` | `T` | ✔ |
| 42 | `ZSTD_compressContinue` | `T` | ✔ |
| 43 | `ZSTD_compressContinue_public` | `T` | ✔ |
| 44 | `ZSTD_compressEnd` | `T` | ✔ |
| 45 | `ZSTD_compressEnd_public` | `T` | ✔ |
| 46 | `ZSTD_compressSequences` | `T` | ✔ |
| 47 | `ZSTD_compressSequencesAndLiterals` | `T` | ✔ |
| 48 | `ZSTD_compressStream` | `T` | ✔ |
| 49 | `ZSTD_compressStream2` | `T` | ✔ |
| 50 | `ZSTD_compressStream2_simpleArgs` | `T` | ✔ |
| 51 | `ZSTD_compress_advanced` | `T` | ✔ |
| 52 | `ZSTD_compress_advanced_internal` | `T` | ✔ |
| 53 | `ZSTD_compress_usingCDict` | `T` | ✔ |
| 54 | `ZSTD_compress_usingCDict_advanced` | `T` | ✔ |
| 55 | `ZSTD_compress_usingDict` | `T` | ✔ |
| 56 | `ZSTD_convertBlockSequences` | `T` | ✔ |
| 57 | `ZSTD_copyCCtx` | `T` | ✔ |
| 58 | `ZSTD_createCCtx` | `T` | ✔ |
| 59 | `ZSTD_createCCtxParams` | `T` | ✔ |
| 60 | `ZSTD_createCCtx_advanced` | `T` | ✔ |
| 61 | `ZSTD_createCDict` | `T` | ✔ |
| 62 | `ZSTD_createCDict_advanced` | `T` | ✔ |
| 63 | `ZSTD_createCDict_advanced2` | `T` | ✔ |
| 64 | `ZSTD_createCDict_byReference` | `T` | ✔ |
| 65 | `ZSTD_createCStream` | `T` | ✔ |
| 66 | `ZSTD_createCStream_advanced` | `T` | ✔ |
| 67 | `ZSTD_cycleLog` | `T` | ✔ |
| 68 | `ZSTD_defaultCLevel` | `T` | ✔ |
| 69 | `ZSTD_endStream` | `T` | ✔ |
| 70 | `ZSTD_estimateCCtxSize` | `T` | ✔ |
| 71 | `ZSTD_estimateCCtxSize_usingCCtxParams` | `T` | ✔ |
| 72 | `ZSTD_estimateCCtxSize_usingCParams` | `T` | ✔ |
| 73 | `ZSTD_estimateCDictSize` | `T` | ✔ |
| 74 | `ZSTD_estimateCDictSize_advanced` | `T` | ✔ |
| 75 | `ZSTD_estimateCStreamSize` | `T` | ✔ |
| 76 | `ZSTD_estimateCStreamSize_usingCCtxParams` | `T` | ✔ |
| 77 | `ZSTD_estimateCStreamSize_usingCParams` | `T` | ✔ |
| 78 | `ZSTD_flushStream` | `T` | ✔ |
| 79 | `ZSTD_freeCCtx` | `T` | ✔ |
| 80 | `ZSTD_freeCCtxParams` | `T` | ✔ |
| 81 | `ZSTD_freeCDict` | `T` | ✔ |
| 82 | `ZSTD_freeCStream` | `T` | ✔ |
| 83 | `ZSTD_generateSequences` | `T` | ✔ |
| 84 | `ZSTD_get1BlockSummary` | `T` | ✔ |
| 85 | `ZSTD_getBlockSize` | `T` | ✔ |
| 86 | `ZSTD_getCParams` | `T` | ✔ |
| 87 | `ZSTD_getCParamsFromCCtxParams` | `T` | ✔ |
| 88 | `ZSTD_getCParamsFromCDict` | `T` | ✔ |
| 89 | `ZSTD_getDictID_fromCDict` | `T` | ✔ |
| 90 | `ZSTD_getFrameProgression` | `T` | ✔ |
| 91 | `ZSTD_getParams` | `T` | ✔ |
| 92 | `ZSTD_getSeqStore` | `T` | ✔ |
| 93 | `ZSTD_initCStream` | `T` | ✔ |
| 94 | `ZSTD_initCStream_advanced` | `T` | ✔ |
| 95 | `ZSTD_initCStream_internal` | `T` | ✔ |
| 96 | `ZSTD_initCStream_srcSize` | `T` | ✔ |
| 97 | `ZSTD_initCStream_usingCDict` | `T` | ✔ |
| 98 | `ZSTD_initCStream_usingCDict_advanced` | `T` | ✔ |
| 99 | `ZSTD_initCStream_usingDict` | `T` | ✔ |
| 100 | `ZSTD_initStaticCCtx` | `T` | ✔ |
| 101 | `ZSTD_initStaticCDict` | `T` | ✔ |
| 102 | `ZSTD_initStaticCStream` | `T` | ✔ |
| 103 | `ZSTD_invalidateRepCodes` | `T` | ✔ |
| 104 | `ZSTD_loadCEntropy` | `T` | ✔ |
| 105 | `ZSTD_maxCLevel` | `T` | ✔ |
| 106 | `ZSTD_mergeBlockDelimiters` | `T` | ✔ |
| 107 | `ZSTD_minCLevel` | `T` | ✔ |
| 108 | `ZSTD_referenceExternalSequences` | `T` | ✔ |
| 109 | `ZSTD_registerSequenceProducer` | `T` | ✔ |
| 110 | `ZSTD_resetCStream` | `T` | ✔ |
| 111 | `ZSTD_resetSeqStore` | `T` | ✔ |
| 112 | `ZSTD_reset_compressedBlockState` | `T` | ✔ |
| 113 | `ZSTD_selectBlockCompressor` | `T` | ✔ |
| 114 | `ZSTD_seqToCodes` | `T` | ✔ |
| 115 | `ZSTD_sequenceBound` | `T` | ✔ |
| 116 | `ZSTD_sizeof_CCtx` | `T` | ✔ |
| 117 | `ZSTD_sizeof_CDict` | `T` | ✔ |
| 118 | `ZSTD_sizeof_CStream` | `T` | ✔ |
| 119 | `ZSTD_toFlushNow` | `T` | ✔ |
| 120 | `ZSTD_writeLastEmptyBlock` | `T` | ✔ |
| 121 | `ZSTD_writeSkippableFrame` | `T` | ✔ |

### `compress/zstd_compress_literals.c` — 3 symbols, 3 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressLiterals` | `T` | ✔ |
| 2 | `ZSTD_compressRleLiteralsBlock` | `T` | ✔ |
| 3 | `ZSTD_noCompressLiterals` | `T` | ✔ |

### `compress/zstd_compress_sequences.c` — 5 symbols, 5 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_buildCTable` | `T` | ✔ |
| 2 | `ZSTD_crossEntropyCost` | `T` | ✔ |
| 3 | `ZSTD_encodeSequences` | `T` | ✔ |
| 4 | `ZSTD_fseBitCost` | `T` | ✔ |
| 5 | `ZSTD_selectEncodingType` | `T` | ✔ |

### `compress/zstd_compress_superblock.c` — 1 symbols, 1 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressSuperBlock` | `T` | ✔ |

### `compress/zstd_double_fast.c` — 4 symbols, 4 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressBlock_doubleFast` | `T` | ✔ |
| 2 | `ZSTD_compressBlock_doubleFast_dictMatchState` | `T` | ✔ |
| 3 | `ZSTD_compressBlock_doubleFast_extDict` | `T` | ✔ |
| 4 | `ZSTD_fillDoubleHashTable` | `T` | ✔ |

### `compress/zstd_fast.c` — 4 symbols, 4 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressBlock_fast` | `T` | ✔ |
| 2 | `ZSTD_compressBlock_fast_dictMatchState` | `T` | ✔ |
| 3 | `ZSTD_compressBlock_fast_extDict` | `T` | ✔ |
| 4 | `ZSTD_fillHashTable` | `T` | ✔ |

### `compress/zstd_lazy.c` — 30 symbols, 30 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressBlock_btlazy2` | `T` | ✔ |
| 2 | `ZSTD_compressBlock_btlazy2_dictMatchState` | `T` | ✔ |
| 3 | `ZSTD_compressBlock_btlazy2_extDict` | `T` | ✔ |
| 4 | `ZSTD_compressBlock_greedy` | `T` | ✔ |
| 5 | `ZSTD_compressBlock_greedy_dedicatedDictSearch` | `T` | ✔ |
| 6 | `ZSTD_compressBlock_greedy_dedicatedDictSearch_row` | `T` | ✔ |
| 7 | `ZSTD_compressBlock_greedy_dictMatchState` | `T` | ✔ |
| 8 | `ZSTD_compressBlock_greedy_dictMatchState_row` | `T` | ✔ |
| 9 | `ZSTD_compressBlock_greedy_extDict` | `T` | ✔ |
| 10 | `ZSTD_compressBlock_greedy_extDict_row` | `T` | ✔ |
| 11 | `ZSTD_compressBlock_greedy_row` | `T` | ✔ |
| 12 | `ZSTD_compressBlock_lazy` | `T` | ✔ |
| 13 | `ZSTD_compressBlock_lazy2` | `T` | ✔ |
| 14 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch` | `T` | ✔ |
| 15 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch_row` | `T` | ✔ |
| 16 | `ZSTD_compressBlock_lazy2_dictMatchState` | `T` | ✔ |
| 17 | `ZSTD_compressBlock_lazy2_dictMatchState_row` | `T` | ✔ |
| 18 | `ZSTD_compressBlock_lazy2_extDict` | `T` | ✔ |
| 19 | `ZSTD_compressBlock_lazy2_extDict_row` | `T` | ✔ |
| 20 | `ZSTD_compressBlock_lazy2_row` | `T` | ✔ |
| 21 | `ZSTD_compressBlock_lazy_dedicatedDictSearch` | `T` | ✔ |
| 22 | `ZSTD_compressBlock_lazy_dedicatedDictSearch_row` | `T` | ✔ |
| 23 | `ZSTD_compressBlock_lazy_dictMatchState` | `T` | ✔ |
| 24 | `ZSTD_compressBlock_lazy_dictMatchState_row` | `T` | ✔ |
| 25 | `ZSTD_compressBlock_lazy_extDict` | `T` | ✔ |
| 26 | `ZSTD_compressBlock_lazy_extDict_row` | `T` | ✔ |
| 27 | `ZSTD_compressBlock_lazy_row` | `T` | ✔ |
| 28 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `T` | ✔ |
| 29 | `ZSTD_insertAndFindFirstIndex` | `T` | ✔ |
| 30 | `ZSTD_row_update` | `T` | ✔ |

### `compress/zstd_ldm.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_ldm_adjustParameters` | `T` | ✔ |
| 2 | `ZSTD_ldm_blockCompress` | `T` | ✔ |
| 3 | `ZSTD_ldm_fillHashTable` | `T` | ✔ |
| 4 | `ZSTD_ldm_generateSequences` | `T` | ✔ |
| 5 | `ZSTD_ldm_getMaxNbSeq` | `T` | ✔ |
| 6 | `ZSTD_ldm_getTableSize` | `T` | ✔ |
| 7 | `ZSTD_ldm_skipRawSeqStoreBytes` | `T` | ✔ |
| 8 | `ZSTD_ldm_skipSequences` | `T` | ✔ |

### `compress/zstd_opt.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_compressBlock_btopt` | `T` | ✔ |
| 2 | `ZSTD_compressBlock_btopt_dictMatchState` | `T` | ✔ |
| 3 | `ZSTD_compressBlock_btopt_extDict` | `T` | ✔ |
| 4 | `ZSTD_compressBlock_btultra` | `T` | ✔ |
| 5 | `ZSTD_compressBlock_btultra2` | `T` | ✔ |
| 6 | `ZSTD_compressBlock_btultra_dictMatchState` | `T` | ✔ |
| 7 | `ZSTD_compressBlock_btultra_extDict` | `T` | ✔ |
| 8 | `ZSTD_updateTree` | `T` | ✔ |

### `compress/zstd_preSplit.c` — 1 symbols, 1 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_splitBlock` | `T` | ✔ |

### `compress/zstdmt_compress.c` — 9 symbols, 9 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTDMT_compressStream_generic` | `T` | ✔ |
| 2 | `ZSTDMT_createCCtx_advanced` | `T` | ✔ |
| 3 | `ZSTDMT_freeCCtx` | `T` | ✔ |
| 4 | `ZSTDMT_getFrameProgression` | `T` | ✔ |
| 5 | `ZSTDMT_initCStream_internal` | `T` | ✔ |
| 6 | `ZSTDMT_nextInputSizeHint` | `T` | ✔ |
| 7 | `ZSTDMT_sizeof_CCtx` | `T` | ✔ |
| 8 | `ZSTDMT_toFlushNow` | `T` | ✔ |
| 9 | `ZSTDMT_updateCParams_whileCompressing` | `T` | ✔ |

### `decompress/huf_decompress.c` — 9 symbols, 9 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `HUF_decompress1X1_DCtx_wksp` | `T` | ✔ |
| 2 | `HUF_decompress1X2_DCtx_wksp` | `T` | ✔ |
| 3 | `HUF_decompress1X_DCtx_wksp` | `T` | ✔ |
| 4 | `HUF_decompress1X_usingDTable` | `T` | ✔ |
| 5 | `HUF_decompress4X_hufOnly_wksp` | `T` | ✔ |
| 6 | `HUF_decompress4X_usingDTable` | `T` | ✔ |
| 7 | `HUF_readDTableX1_wksp` | `T` | ✔ |
| 8 | `HUF_readDTableX2_wksp` | `T` | ✔ |
| 9 | `HUF_selectDecoder` | `T` | ✔ |

### `decompress/zstd_ddict.c` — 11 symbols, 11 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_DDict_dictContent` | `T` | ✔ |
| 2 | `ZSTD_DDict_dictSize` | `T` | ✔ |
| 3 | `ZSTD_copyDDictParameters` | `T` | ✔ |
| 4 | `ZSTD_createDDict` | `T` | ✔ |
| 5 | `ZSTD_createDDict_advanced` | `T` | ✔ |
| 6 | `ZSTD_createDDict_byReference` | `T` | ✔ |
| 7 | `ZSTD_estimateDDictSize` | `T` | ✔ |
| 8 | `ZSTD_freeDDict` | `T` | ✔ |
| 9 | `ZSTD_getDictID_fromDDict` | `T` | ✔ |
| 10 | `ZSTD_initStaticDDict` | `T` | ✔ |
| 11 | `ZSTD_sizeof_DDict` | `T` | ✔ |

### `decompress/zstd_decompress.c` — 61 symbols, 61 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_DCtx_getParameter` | `T` | ✔ |
| 2 | `ZSTD_DCtx_loadDictionary` | `T` | ✔ |
| 3 | `ZSTD_DCtx_loadDictionary_advanced` | `T` | ✔ |
| 4 | `ZSTD_DCtx_loadDictionary_byReference` | `T` | ✔ |
| 5 | `ZSTD_DCtx_refDDict` | `T` | ✔ |
| 6 | `ZSTD_DCtx_refPrefix` | `T` | ✔ |
| 7 | `ZSTD_DCtx_refPrefix_advanced` | `T` | ✔ |
| 8 | `ZSTD_DCtx_reset` | `T` | ✔ |
| 9 | `ZSTD_DCtx_setFormat` | `T` | ✔ |
| 10 | `ZSTD_DCtx_setMaxWindowSize` | `T` | ✔ |
| 11 | `ZSTD_DCtx_setParameter` | `T` | ✔ |
| 12 | `ZSTD_DStreamInSize` | `T` | ✔ |
| 13 | `ZSTD_DStreamOutSize` | `T` | ✔ |
| 14 | `ZSTD_copyDCtx` | `T` | ✔ |
| 15 | `ZSTD_createDCtx` | `T` | ✔ |
| 16 | `ZSTD_createDCtx_advanced` | `T` | ✔ |
| 17 | `ZSTD_createDStream` | `T` | ✔ |
| 18 | `ZSTD_createDStream_advanced` | `T` | ✔ |
| 19 | `ZSTD_dParam_getBounds` | `T` | ✔ |
| 20 | `ZSTD_decodingBufferSize_min` | `T` | ✔ |
| 21 | `ZSTD_decompress` | `T` | ✔ |
| 22 | `ZSTD_decompressBegin` | `T` | ✔ |
| 23 | `ZSTD_decompressBegin_usingDDict` | `T` | ✔ |
| 24 | `ZSTD_decompressBegin_usingDict` | `T` | ✔ |
| 25 | `ZSTD_decompressBound` | `T` | ✔ |
| 26 | `ZSTD_decompressContinue` | `T` | ✔ |
| 27 | `ZSTD_decompressDCtx` | `T` | ✔ |
| 28 | `ZSTD_decompressStream` | `T` | ✔ |
| 29 | `ZSTD_decompressStream_simpleArgs` | `T` | ✔ |
| 30 | `ZSTD_decompress_usingDDict` | `T` | ✔ |
| 31 | `ZSTD_decompress_usingDict` | `T` | ✔ |
| 32 | `ZSTD_decompressionMargin` | `T` | ✔ |
| 33 | `ZSTD_estimateDCtxSize` | `T` | ✔ |
| 34 | `ZSTD_estimateDStreamSize` | `T` | ✔ |
| 35 | `ZSTD_estimateDStreamSize_fromFrame` | `T` | ✔ |
| 36 | `ZSTD_findDecompressedSize` | `T` | ✔ |
| 37 | `ZSTD_findFrameCompressedSize` | `T` | ✔ |
| 38 | `ZSTD_frameHeaderSize` | `T` | ✔ |
| 39 | `ZSTD_freeDCtx` | `T` | ✔ |
| 40 | `ZSTD_freeDStream` | `T` | ✔ |
| 41 | `ZSTD_getDecompressedSize` | `T` | ✔ |
| 42 | `ZSTD_getDictID_fromDict` | `T` | ✔ |
| 43 | `ZSTD_getDictID_fromFrame` | `T` | ✔ |
| 44 | `ZSTD_getFrameContentSize` | `T` | ✔ |
| 45 | `ZSTD_getFrameHeader` | `T` | ✔ |
| 46 | `ZSTD_getFrameHeader_advanced` | `T` | ✔ |
| 47 | `ZSTD_initDStream` | `T` | ✔ |
| 48 | `ZSTD_initDStream_usingDDict` | `T` | ✔ |
| 49 | `ZSTD_initDStream_usingDict` | `T` | ✔ |
| 50 | `ZSTD_initStaticDCtx` | `T` | ✔ |
| 51 | `ZSTD_initStaticDStream` | `T` | ✔ |
| 52 | `ZSTD_insertBlock` | `T` | ✔ |
| 53 | `ZSTD_isFrame` | `T` | ✔ |
| 54 | `ZSTD_isSkippableFrame` | `T` | ✔ |
| 55 | `ZSTD_loadDEntropy` | `T` | ✔ |
| 56 | `ZSTD_nextInputType` | `T` | ✔ |
| 57 | `ZSTD_nextSrcSizeToDecompress` | `T` | ✔ |
| 58 | `ZSTD_readSkippableFrame` | `T` | ✔ |
| 59 | `ZSTD_resetDStream` | `T` | ✔ |
| 60 | `ZSTD_sizeof_DCtx` | `T` | ✔ |
| 61 | `ZSTD_sizeof_DStream` | `T` | ✔ |

### `decompress/zstd_decompress_block.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTD_buildFSETable` | `T` | ✔ |
| 2 | `ZSTD_checkContinuity` | `T` | ✔ |
| 3 | `ZSTD_decodeLiteralsBlock_wrapper` | `T` | ✔ |
| 4 | `ZSTD_decodeSeqHeaders` | `T` | ✔ |
| 5 | `ZSTD_decompressBlock` | `T` | ✔ |
| 6 | `ZSTD_decompressBlock_deprecated` | `T` | ✔ |
| 7 | `ZSTD_decompressBlock_internal` | `T` | ✔ |
| 8 | `ZSTD_getcBlockSize` | `T` | ✔ |

### `deprecated/zbuff_common.c` — 2 symbols, 2 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZBUFF_getErrorName` | `T` | ✔ |
| 2 | `ZBUFF_isError` | `T` | ✔ |

### `deprecated/zbuff_compress.c` — 11 symbols, 11 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZBUFF_compressContinue` | `T` | ✔ |
| 2 | `ZBUFF_compressEnd` | `T` | ✔ |
| 3 | `ZBUFF_compressFlush` | `T` | ✔ |
| 4 | `ZBUFF_compressInit` | `T` | ✔ |
| 5 | `ZBUFF_compressInitDictionary` | `T` | ✔ |
| 6 | `ZBUFF_compressInit_advanced` | `T` | ✔ |
| 7 | `ZBUFF_createCCtx` | `T` | ✔ |
| 8 | `ZBUFF_createCCtx_advanced` | `T` | ✔ |
| 9 | `ZBUFF_freeCCtx` | `T` | ✔ |
| 10 | `ZBUFF_recommendedCInSize` | `T` | ✔ |
| 11 | `ZBUFF_recommendedCOutSize` | `T` | ✔ |

### `deprecated/zbuff_decompress.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZBUFF_createDCtx` | `T` | ✔ |
| 2 | `ZBUFF_createDCtx_advanced` | `T` | ✔ |
| 3 | `ZBUFF_decompressContinue` | `T` | ✔ |
| 4 | `ZBUFF_decompressInit` | `T` | ✔ |
| 5 | `ZBUFF_decompressInitDictionary` | `T` | ✔ |
| 6 | `ZBUFF_freeDCtx` | `T` | ✔ |
| 7 | `ZBUFF_recommendedDInSize` | `T` | ✔ |
| 8 | `ZBUFF_recommendedDOutSize` | `T` | ✔ |

### `dictBuilder/cover.c` — 15 symbols, 15 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `COVER_best_destroy` | `T` | ✔ |
| 2 | `COVER_best_finish` | `T` | ✔ |
| 3 | `COVER_best_init` | `T` | ✔ |
| 4 | `COVER_best_start` | `T` | ✔ |
| 5 | `COVER_best_wait` | `T` | ✔ |
| 6 | `COVER_checkTotalCompressedSize` | `T` | ✔ |
| 7 | `COVER_computeEpochs` | `T` | ✔ |
| 8 | `COVER_dictSelectionError` | `T` | ✔ |
| 9 | `COVER_dictSelectionFree` | `T` | ✔ |
| 10 | `COVER_dictSelectionIsError` | `T` | ✔ |
| 11 | `COVER_selectDict` | `T` | ✔ |
| 12 | `COVER_sum` | `T` | ✔ |
| 13 | `COVER_warnOnSmallCorpus` | `T` | ✔ |
| 14 | `ZDICT_optimizeTrainFromBuffer_cover` | `T` | ✔ |
| 15 | `ZDICT_trainFromBuffer_cover` | `T` | ✔ |

### `dictBuilder/divsufsort.c` — 2 symbols, 2 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `divbwt` | `T` | ✔ |
| 2 | `divsufsort` | `T` | ✔ |

### `dictBuilder/fastcover.c` — 2 symbols, 2 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `T` | ✔ |
| 2 | `ZDICT_trainFromBuffer_fastCover` | `T` | ✔ |

### `dictBuilder/zdict.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZDICT_addEntropyTablesFromBuffer` | `T` | ✔ |
| 2 | `ZDICT_finalizeDictionary` | `T` | ✔ |
| 3 | `ZDICT_getDictHeaderSize` | `T` | ✔ |
| 4 | `ZDICT_getDictID` | `T` | ✔ |
| 5 | `ZDICT_getErrorName` | `T` | ✔ |
| 6 | `ZDICT_isError` | `T` | ✔ |
| 7 | `ZDICT_trainFromBuffer` | `T` | ✔ |
| 8 | `ZDICT_trainFromBuffer_legacy` | `T` | ✔ |

### `legacy/zstd_v01.c` — 9 symbols, 9 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTDv01_createDCtx` | `T` | ✔ |
| 2 | `ZSTDv01_decompress` | `T` | ✔ |
| 3 | `ZSTDv01_decompressContinue` | `T` | ✔ |
| 4 | `ZSTDv01_decompressDCtx` | `T` | ✔ |
| 5 | `ZSTDv01_findFrameSizeInfoLegacy` | `T` | ✔ |
| 6 | `ZSTDv01_freeDCtx` | `T` | ✔ |
| 7 | `ZSTDv01_isError` | `T` | ✔ |
| 8 | `ZSTDv01_nextSrcSizeToDecompress` | `T` | ✔ |
| 9 | `ZSTDv01_resetDCtx` | `T` | ✔ |

### `legacy/zstd_v02.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTDv02_createDCtx` | `T` | ✔ |
| 2 | `ZSTDv02_decompress` | `T` | ✔ |
| 3 | `ZSTDv02_decompressContinue` | `T` | ✔ |
| 4 | `ZSTDv02_findFrameSizeInfoLegacy` | `T` | ✔ |
| 5 | `ZSTDv02_freeDCtx` | `T` | ✔ |
| 6 | `ZSTDv02_isError` | `T` | ✔ |
| 7 | `ZSTDv02_nextSrcSizeToDecompress` | `T` | ✔ |
| 8 | `ZSTDv02_resetDCtx` | `T` | ✔ |

### `legacy/zstd_v03.c` — 8 symbols, 8 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZSTDv03_createDCtx` | `T` | ✔ |
| 2 | `ZSTDv03_decompress` | `T` | ✔ |
| 3 | `ZSTDv03_decompressContinue` | `T` | ✔ |
| 4 | `ZSTDv03_findFrameSizeInfoLegacy` | `T` | ✔ |
| 5 | `ZSTDv03_freeDCtx` | `T` | ✔ |
| 6 | `ZSTDv03_isError` | `T` | ✔ |
| 7 | `ZSTDv03_nextSrcSizeToDecompress` | `T` | ✔ |
| 8 | `ZSTDv03_resetDCtx` | `T` | ✔ |

### `legacy/zstd_v04.c` — 17 symbols, 17 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `ZBUFFv04_createDCtx` | `T` | ✔ |
| 2 | `ZBUFFv04_decompressContinue` | `T` | ✔ |
| 3 | `ZBUFFv04_decompressInit` | `T` | ✔ |
| 4 | `ZBUFFv04_decompressWithDictionary` | `T` | ✔ |
| 5 | `ZBUFFv04_freeDCtx` | `T` | ✔ |
| 6 | `ZBUFFv04_getErrorName` | `T` | ✔ |
| 7 | `ZBUFFv04_isError` | `T` | ✔ |
| 8 | `ZBUFFv04_recommendedDInSize` | `T` | ✔ |
| 9 | `ZBUFFv04_recommendedDOutSize` | `T` | ✔ |
| 10 | `ZSTDv04_createDCtx` | `T` | ✔ |
| 11 | `ZSTDv04_decompress` | `T` | ✔ |
| 12 | `ZSTDv04_decompressContinue` | `T` | ✔ |
| 13 | `ZSTDv04_decompressDCtx` | `T` | ✔ |
| 14 | `ZSTDv04_findFrameSizeInfoLegacy` | `T` | ✔ |
| 15 | `ZSTDv04_freeDCtx` | `T` | ✔ |
| 16 | `ZSTDv04_nextSrcSizeToDecompress` | `T` | ✔ |
| 17 | `ZSTDv04_resetDCtx` | `T` | ✔ |

### `legacy/zstd_v05.c` — 49 symbols, 49 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSEv05_buildDTable` | `T` | ✔ |
| 2 | `FSEv05_buildDTable_raw` | `T` | ✔ |
| 3 | `FSEv05_buildDTable_rle` | `T` | ✔ |
| 4 | `FSEv05_createDTable` | `T` | ✔ |
| 5 | `FSEv05_decompress` | `T` | ✔ |
| 6 | `FSEv05_decompress_usingDTable` | `T` | ✔ |
| 7 | `FSEv05_freeDTable` | `T` | ✔ |
| 8 | `FSEv05_getErrorName` | `T` | ✔ |
| 9 | `FSEv05_isError` | `T` | ✔ |
| 10 | `FSEv05_readNCount` | `T` | ✔ |
| 11 | `HUFv05_decompress` | `T` | ✔ |
| 12 | `HUFv05_decompress1X2` | `T` | ✔ |
| 13 | `HUFv05_decompress1X2_usingDTable` | `T` | ✔ |
| 14 | `HUFv05_decompress1X4` | `T` | ✔ |
| 15 | `HUFv05_decompress1X4_usingDTable` | `T` | ✔ |
| 16 | `HUFv05_decompress4X2` | `T` | ✔ |
| 17 | `HUFv05_decompress4X2_usingDTable` | `T` | ✔ |
| 18 | `HUFv05_decompress4X4` | `T` | ✔ |
| 19 | `HUFv05_decompress4X4_usingDTable` | `T` | ✔ |
| 20 | `HUFv05_getErrorName` | `T` | ✔ |
| 21 | `HUFv05_isError` | `T` | ✔ |
| 22 | `HUFv05_readDTableX2` | `T` | ✔ |
| 23 | `HUFv05_readDTableX4` | `T` | ✔ |
| 24 | `ZBUFFv05_createDCtx` | `T` | ✔ |
| 25 | `ZBUFFv05_decompressContinue` | `T` | ✔ |
| 26 | `ZBUFFv05_decompressInit` | `T` | ✔ |
| 27 | `ZBUFFv05_decompressInitDictionary` | `T` | ✔ |
| 28 | `ZBUFFv05_freeDCtx` | `T` | ✔ |
| 29 | `ZBUFFv05_getErrorName` | `T` | ✔ |
| 30 | `ZBUFFv05_isError` | `T` | ✔ |
| 31 | `ZBUFFv05_recommendedDInSize` | `T` | ✔ |
| 32 | `ZBUFFv05_recommendedDOutSize` | `T` | ✔ |
| 33 | `ZSTDv05_copyDCtx` | `T` | ✔ |
| 34 | `ZSTDv05_createDCtx` | `T` | ✔ |
| 35 | `ZSTDv05_decompress` | `T` | ✔ |
| 36 | `ZSTDv05_decompressBegin` | `T` | ✔ |
| 37 | `ZSTDv05_decompressBegin_usingDict` | `T` | ✔ |
| 38 | `ZSTDv05_decompressBlock` | `T` | ✔ |
| 39 | `ZSTDv05_decompressContinue` | `T` | ✔ |
| 40 | `ZSTDv05_decompressDCtx` | `T` | ✔ |
| 41 | `ZSTDv05_decompress_usingDict` | `T` | ✔ |
| 42 | `ZSTDv05_decompress_usingPreparedDCtx` | `T` | ✔ |
| 43 | `ZSTDv05_findFrameSizeInfoLegacy` | `T` | ✔ |
| 44 | `ZSTDv05_freeDCtx` | `T` | ✔ |
| 45 | `ZSTDv05_getErrorName` | `T` | ✔ |
| 46 | `ZSTDv05_getFrameParams` | `T` | ✔ |
| 47 | `ZSTDv05_isError` | `T` | ✔ |
| 48 | `ZSTDv05_nextSrcSizeToDecompress` | `T` | ✔ |
| 49 | `ZSTDv05_sizeofDCtx` | `T` | ✔ |

### `legacy/zstd_v06.c` — 47 symbols, 47 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSEv06_buildDTable` | `T` | ✔ |
| 2 | `FSEv06_buildDTable_raw` | `T` | ✔ |
| 3 | `FSEv06_buildDTable_rle` | `T` | ✔ |
| 4 | `FSEv06_createDTable` | `T` | ✔ |
| 5 | `FSEv06_decompress` | `T` | ✔ |
| 6 | `FSEv06_decompress_usingDTable` | `T` | ✔ |
| 7 | `FSEv06_freeDTable` | `T` | ✔ |
| 8 | `FSEv06_getErrorName` | `T` | ✔ |
| 9 | `FSEv06_isError` | `T` | ✔ |
| 10 | `FSEv06_readNCount` | `T` | ✔ |
| 11 | `HUFv06_decompress` | `T` | ✔ |
| 12 | `HUFv06_decompress1X2` | `T` | ✔ |
| 13 | `HUFv06_decompress1X2_usingDTable` | `T` | ✔ |
| 14 | `HUFv06_decompress1X4` | `T` | ✔ |
| 15 | `HUFv06_decompress1X4_usingDTable` | `T` | ✔ |
| 16 | `HUFv06_decompress4X2` | `T` | ✔ |
| 17 | `HUFv06_decompress4X2_usingDTable` | `T` | ✔ |
| 18 | `HUFv06_decompress4X4` | `T` | ✔ |
| 19 | `HUFv06_decompress4X4_usingDTable` | `T` | ✔ |
| 20 | `HUFv06_readDTableX2` | `T` | ✔ |
| 21 | `HUFv06_readDTableX4` | `T` | ✔ |
| 22 | `ZBUFFv06_createDCtx` | `T` | ✔ |
| 23 | `ZBUFFv06_decompressContinue` | `T` | ✔ |
| 24 | `ZBUFFv06_decompressInit` | `T` | ✔ |
| 25 | `ZBUFFv06_decompressInitDictionary` | `T` | ✔ |
| 26 | `ZBUFFv06_freeDCtx` | `T` | ✔ |
| 27 | `ZBUFFv06_getErrorName` | `T` | ✔ |
| 28 | `ZBUFFv06_isError` | `T` | ✔ |
| 29 | `ZBUFFv06_recommendedDInSize` | `T` | ✔ |
| 30 | `ZBUFFv06_recommendedDOutSize` | `T` | ✔ |
| 31 | `ZSTDv06_copyDCtx` | `T` | ✔ |
| 32 | `ZSTDv06_createDCtx` | `T` | ✔ |
| 33 | `ZSTDv06_decompress` | `T` | ✔ |
| 34 | `ZSTDv06_decompressBegin` | `T` | ✔ |
| 35 | `ZSTDv06_decompressBegin_usingDict` | `T` | ✔ |
| 36 | `ZSTDv06_decompressBlock` | `T` | ✔ |
| 37 | `ZSTDv06_decompressContinue` | `T` | ✔ |
| 38 | `ZSTDv06_decompressDCtx` | `T` | ✔ |
| 39 | `ZSTDv06_decompress_usingDict` | `T` | ✔ |
| 40 | `ZSTDv06_decompress_usingPreparedDCtx` | `T` | ✔ |
| 41 | `ZSTDv06_findFrameSizeInfoLegacy` | `T` | ✔ |
| 42 | `ZSTDv06_freeDCtx` | `T` | ✔ |
| 43 | `ZSTDv06_getErrorName` | `T` | ✔ |
| 44 | `ZSTDv06_getFrameParams` | `T` | ✔ |
| 45 | `ZSTDv06_isError` | `T` | ✔ |
| 46 | `ZSTDv06_nextSrcSizeToDecompress` | `T` | ✔ |
| 47 | `ZSTDv06_sizeofDCtx` | `T` | ✔ |

### `legacy/zstd_v07.c` — 68 symbols, 68 present in Rust

| # | symbol | type | in Rust |
|---|---|---|---|
| 1 | `FSEv07_buildDTable` | `T` | ✔ |
| 2 | `FSEv07_buildDTable_raw` | `T` | ✔ |
| 3 | `FSEv07_buildDTable_rle` | `T` | ✔ |
| 4 | `FSEv07_createDTable` | `T` | ✔ |
| 5 | `FSEv07_decompress` | `T` | ✔ |
| 6 | `FSEv07_decompress_usingDTable` | `T` | ✔ |
| 7 | `FSEv07_freeDTable` | `T` | ✔ |
| 8 | `FSEv07_getErrorName` | `T` | ✔ |
| 9 | `FSEv07_isError` | `T` | ✔ |
| 10 | `FSEv07_readNCount` | `T` | ✔ |
| 11 | `HUFv07_decompress` | `T` | ✔ |
| 12 | `HUFv07_decompress1X2` | `T` | ✔ |
| 13 | `HUFv07_decompress1X2_DCtx` | `T` | ✔ |
| 14 | `HUFv07_decompress1X2_usingDTable` | `T` | ✔ |
| 15 | `HUFv07_decompress1X4` | `T` | ✔ |
| 16 | `HUFv07_decompress1X4_DCtx` | `T` | ✔ |
| 17 | `HUFv07_decompress1X4_usingDTable` | `T` | ✔ |
| 18 | `HUFv07_decompress1X_DCtx` | `T` | ✔ |
| 19 | `HUFv07_decompress1X_usingDTable` | `T` | ✔ |
| 20 | `HUFv07_decompress4X2` | `T` | ✔ |
| 21 | `HUFv07_decompress4X2_DCtx` | `T` | ✔ |
| 22 | `HUFv07_decompress4X2_usingDTable` | `T` | ✔ |
| 23 | `HUFv07_decompress4X4` | `T` | ✔ |
| 24 | `HUFv07_decompress4X4_DCtx` | `T` | ✔ |
| 25 | `HUFv07_decompress4X4_usingDTable` | `T` | ✔ |
| 26 | `HUFv07_decompress4X_DCtx` | `T` | ✔ |
| 27 | `HUFv07_decompress4X_hufOnly` | `T` | ✔ |
| 28 | `HUFv07_decompress4X_usingDTable` | `T` | ✔ |
| 29 | `HUFv07_getErrorName` | `T` | ✔ |
| 30 | `HUFv07_isError` | `T` | ✔ |
| 31 | `HUFv07_readDTableX2` | `T` | ✔ |
| 32 | `HUFv07_readDTableX4` | `T` | ✔ |
| 33 | `HUFv07_readStats` | `T` | ✔ |
| 34 | `HUFv07_selectDecoder` | `T` | ✔ |
| 35 | `ZBUFFv07_createDCtx` | `T` | ✔ |
| 36 | `ZBUFFv07_createDCtx_advanced` | `T` | ✔ |
| 37 | `ZBUFFv07_decompressContinue` | `T` | ✔ |
| 38 | `ZBUFFv07_decompressInit` | `T` | ✔ |
| 39 | `ZBUFFv07_decompressInitDictionary` | `T` | ✔ |
| 40 | `ZBUFFv07_freeDCtx` | `T` | ✔ |
| 41 | `ZBUFFv07_getErrorName` | `T` | ✔ |
| 42 | `ZBUFFv07_isError` | `T` | ✔ |
| 43 | `ZBUFFv07_recommendedDInSize` | `T` | ✔ |
| 44 | `ZBUFFv07_recommendedDOutSize` | `T` | ✔ |
| 45 | `ZSTDv07_copyDCtx` | `T` | ✔ |
| 46 | `ZSTDv07_createDCtx` | `T` | ✔ |
| 47 | `ZSTDv07_createDCtx_advanced` | `T` | ✔ |
| 48 | `ZSTDv07_createDDict` | `T` | ✔ |
| 49 | `ZSTDv07_decompress` | `T` | ✔ |
| 50 | `ZSTDv07_decompressBegin` | `T` | ✔ |
| 51 | `ZSTDv07_decompressBegin_usingDict` | `T` | ✔ |
| 52 | `ZSTDv07_decompressBlock` | `T` | ✔ |
| 53 | `ZSTDv07_decompressContinue` | `T` | ✔ |
| 54 | `ZSTDv07_decompressDCtx` | `T` | ✔ |
| 55 | `ZSTDv07_decompress_usingDDict` | `T` | ✔ |
| 56 | `ZSTDv07_decompress_usingDict` | `T` | ✔ |
| 57 | `ZSTDv07_estimateDCtxSize` | `T` | ✔ |
| 58 | `ZSTDv07_findFrameSizeInfoLegacy` | `T` | ✔ |
| 59 | `ZSTDv07_freeDCtx` | `T` | ✔ |
| 60 | `ZSTDv07_freeDDict` | `T` | ✔ |
| 61 | `ZSTDv07_getDecompressedSize` | `T` | ✔ |
| 62 | `ZSTDv07_getErrorName` | `T` | ✔ |
| 63 | `ZSTDv07_getFrameParams` | `T` | ✔ |
| 64 | `ZSTDv07_insertBlock` | `T` | ✔ |
| 65 | `ZSTDv07_isError` | `T` | ✔ |
| 66 | `ZSTDv07_isSkipFrame` | `T` | ✔ |
| 67 | `ZSTDv07_nextSrcSizeToDecompress` | `T` | ✔ |
| 68 | `ZSTDv07_sizeofDCtx` | `T` | ✔ |

## Grand total

**615 / 615** C-exported symbols present in the Rust `.so` (100%).
