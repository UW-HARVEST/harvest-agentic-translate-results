# SYMBOLS.md — Exported symbol parity

Derived mechanically from `nm -D --defined-only` on both shared libraries.
Regenerated after the `src/lz4frame.rs` enum-signedness fix.

```
C  .so:   c_src/build/liblz4.so                 -> 143 public symbols
Rust .so: translation/target/release/liblz4.so  -> 143 public symbols
comm -23 (missing from Rust): 0
comm -13 (extra in Rust):     0
```

**Result: the symbol diff is EMPTY — 0 missing, 0 extra, 0 undefined non-libc symbols.**

No module was missing from the translation: all five C translation units
(lz4.c, lz4hc.c, lz4frame.c, lz4file.c, xxhash.c) have Rust counterparts
(lz4.rs, lz4hc.rs, lz4frame.rs, lz4file.rs, xxhash.rs), and every symbol —
including the obsolete/deprecated wrappers and the xxhash `LZ4_`-namespaced
macro-generated names — is exported. Nothing was stubbed.

Verified under EVERY build configuration by `./verify_features.sh`.

## Full symbol table

| # | symbol | in C .so | in Rust .so |
|---|--------|----------|-------------|
| 1 | `LZ4F_compressBegin` | yes | yes |
| 2 | `LZ4F_compressBegin_internal` | yes | yes |
| 3 | `LZ4F_compressBegin_usingCDict` | yes | yes |
| 4 | `LZ4F_compressBegin_usingDict` | yes | yes |
| 5 | `LZ4F_compressBegin_usingDictOnce` | yes | yes |
| 6 | `LZ4F_compressBound` | yes | yes |
| 7 | `LZ4F_compressEnd` | yes | yes |
| 8 | `LZ4F_compressFrame` | yes | yes |
| 9 | `LZ4F_compressFrameBound` | yes | yes |
| 10 | `LZ4F_compressFrame_usingCDict` | yes | yes |
| 11 | `LZ4F_compressUpdate` | yes | yes |
| 12 | `LZ4F_compressionLevel_max` | yes | yes |
| 13 | `LZ4F_createCDict` | yes | yes |
| 14 | `LZ4F_createCDict_advanced` | yes | yes |
| 15 | `LZ4F_createCompressionContext` | yes | yes |
| 16 | `LZ4F_createCompressionContext_advanced` | yes | yes |
| 17 | `LZ4F_createDecompressionContext` | yes | yes |
| 18 | `LZ4F_createDecompressionContext_advanced` | yes | yes |
| 19 | `LZ4F_decompress` | yes | yes |
| 20 | `LZ4F_decompress_usingDict` | yes | yes |
| 21 | `LZ4F_flush` | yes | yes |
| 22 | `LZ4F_freeCDict` | yes | yes |
| 23 | `LZ4F_freeCompressionContext` | yes | yes |
| 24 | `LZ4F_freeDecompressionContext` | yes | yes |
| 25 | `LZ4F_getBlockSize` | yes | yes |
| 26 | `LZ4F_getErrorCode` | yes | yes |
| 27 | `LZ4F_getErrorName` | yes | yes |
| 28 | `LZ4F_getFrameInfo` | yes | yes |
| 29 | `LZ4F_getVersion` | yes | yes |
| 30 | `LZ4F_headerSize` | yes | yes |
| 31 | `LZ4F_isError` | yes | yes |
| 32 | `LZ4F_read` | yes | yes |
| 33 | `LZ4F_readClose` | yes | yes |
| 34 | `LZ4F_readOpen` | yes | yes |
| 35 | `LZ4F_resetDecompressionContext` | yes | yes |
| 36 | `LZ4F_uncompressedUpdate` | yes | yes |
| 37 | `LZ4F_write` | yes | yes |
| 38 | `LZ4F_writeClose` | yes | yes |
| 39 | `LZ4F_writeOpen` | yes | yes |
| 40 | `LZ4HC_searchExtDict` | yes | yes |
| 41 | `LZ4_XXH32` | yes | yes |
| 42 | `LZ4_XXH32_canonicalFromHash` | yes | yes |
| 43 | `LZ4_XXH32_copyState` | yes | yes |
| 44 | `LZ4_XXH32_createState` | yes | yes |
| 45 | `LZ4_XXH32_digest` | yes | yes |
| 46 | `LZ4_XXH32_freeState` | yes | yes |
| 47 | `LZ4_XXH32_hashFromCanonical` | yes | yes |
| 48 | `LZ4_XXH32_reset` | yes | yes |
| 49 | `LZ4_XXH32_update` | yes | yes |
| 50 | `LZ4_XXH64` | yes | yes |
| 51 | `LZ4_XXH64_canonicalFromHash` | yes | yes |
| 52 | `LZ4_XXH64_copyState` | yes | yes |
| 53 | `LZ4_XXH64_createState` | yes | yes |
| 54 | `LZ4_XXH64_digest` | yes | yes |
| 55 | `LZ4_XXH64_freeState` | yes | yes |
| 56 | `LZ4_XXH64_hashFromCanonical` | yes | yes |
| 57 | `LZ4_XXH64_reset` | yes | yes |
| 58 | `LZ4_XXH64_update` | yes | yes |
| 59 | `LZ4_XXH_versionNumber` | yes | yes |
| 60 | `LZ4_attach_HC_dictionary` | yes | yes |
| 61 | `LZ4_attach_dictionary` | yes | yes |
| 62 | `LZ4_compress` | yes | yes |
| 63 | `LZ4_compressBound` | yes | yes |
| 64 | `LZ4_compressHC` | yes | yes |
| 65 | `LZ4_compressHC2` | yes | yes |
| 66 | `LZ4_compressHC2_continue` | yes | yes |
| 67 | `LZ4_compressHC2_limitedOutput` | yes | yes |
| 68 | `LZ4_compressHC2_limitedOutput_continue` | yes | yes |
| 69 | `LZ4_compressHC2_limitedOutput_withStateHC` | yes | yes |
| 70 | `LZ4_compressHC2_withStateHC` | yes | yes |
| 71 | `LZ4_compressHC_continue` | yes | yes |
| 72 | `LZ4_compressHC_limitedOutput` | yes | yes |
| 73 | `LZ4_compressHC_limitedOutput_continue` | yes | yes |
| 74 | `LZ4_compressHC_limitedOutput_withStateHC` | yes | yes |
| 75 | `LZ4_compressHC_withStateHC` | yes | yes |
| 76 | `LZ4_compress_HC` | yes | yes |
| 77 | `LZ4_compress_HC_continue` | yes | yes |
| 78 | `LZ4_compress_HC_continue_destSize` | yes | yes |
| 79 | `LZ4_compress_HC_destSize` | yes | yes |
| 80 | `LZ4_compress_HC_extStateHC` | yes | yes |
| 81 | `LZ4_compress_HC_extStateHC_fastReset` | yes | yes |
| 82 | `LZ4_compress_continue` | yes | yes |
| 83 | `LZ4_compress_default` | yes | yes |
| 84 | `LZ4_compress_destSize` | yes | yes |
| 85 | `LZ4_compress_destSize_extState` | yes | yes |
| 86 | `LZ4_compress_fast` | yes | yes |
| 87 | `LZ4_compress_fast_continue` | yes | yes |
| 88 | `LZ4_compress_fast_extState` | yes | yes |
| 89 | `LZ4_compress_fast_extState_fastReset` | yes | yes |
| 90 | `LZ4_compress_forceExtDict` | yes | yes |
| 91 | `LZ4_compress_limitedOutput` | yes | yes |
| 92 | `LZ4_compress_limitedOutput_continue` | yes | yes |
| 93 | `LZ4_compress_limitedOutput_withState` | yes | yes |
| 94 | `LZ4_compress_withState` | yes | yes |
| 95 | `LZ4_create` | yes | yes |
| 96 | `LZ4_createHC` | yes | yes |
| 97 | `LZ4_createStream` | yes | yes |
| 98 | `LZ4_createStreamDecode` | yes | yes |
| 99 | `LZ4_createStreamHC` | yes | yes |
| 100 | `LZ4_decoderRingBufferSize` | yes | yes |
| 101 | `LZ4_decompress_fast` | yes | yes |
| 102 | `LZ4_decompress_fast_continue` | yes | yes |
| 103 | `LZ4_decompress_fast_usingDict` | yes | yes |
| 104 | `LZ4_decompress_fast_withPrefix64k` | yes | yes |
| 105 | `LZ4_decompress_safe` | yes | yes |
| 106 | `LZ4_decompress_safe_continue` | yes | yes |
| 107 | `LZ4_decompress_safe_forceExtDict` | yes | yes |
| 108 | `LZ4_decompress_safe_partial` | yes | yes |
| 109 | `LZ4_decompress_safe_partial_forceExtDict` | yes | yes |
| 110 | `LZ4_decompress_safe_partial_usingDict` | yes | yes |
| 111 | `LZ4_decompress_safe_usingDict` | yes | yes |
| 112 | `LZ4_decompress_safe_withPrefix64k` | yes | yes |
| 113 | `LZ4_favorDecompressionSpeed` | yes | yes |
| 114 | `LZ4_freeHC` | yes | yes |
| 115 | `LZ4_freeStream` | yes | yes |
| 116 | `LZ4_freeStreamDecode` | yes | yes |
| 117 | `LZ4_freeStreamHC` | yes | yes |
| 118 | `LZ4_initStream` | yes | yes |
| 119 | `LZ4_initStreamHC` | yes | yes |
| 120 | `LZ4_loadDict` | yes | yes |
| 121 | `LZ4_loadDictHC` | yes | yes |
| 122 | `LZ4_loadDictSlow` | yes | yes |
| 123 | `LZ4_loadDict_internal` | yes | yes |
| 124 | `LZ4_resetStream` | yes | yes |
| 125 | `LZ4_resetStreamHC` | yes | yes |
| 126 | `LZ4_resetStreamHC_fast` | yes | yes |
| 127 | `LZ4_resetStreamState` | yes | yes |
| 128 | `LZ4_resetStreamStateHC` | yes | yes |
| 129 | `LZ4_resetStream_fast` | yes | yes |
| 130 | `LZ4_saveDict` | yes | yes |
| 131 | `LZ4_saveDictHC` | yes | yes |
| 132 | `LZ4_setCompressionLevel` | yes | yes |
| 133 | `LZ4_setStreamDecode` | yes | yes |
| 134 | `LZ4_sizeofState` | yes | yes |
| 135 | `LZ4_sizeofStateHC` | yes | yes |
| 136 | `LZ4_sizeofStreamState` | yes | yes |
| 137 | `LZ4_sizeofStreamStateHC` | yes | yes |
| 138 | `LZ4_slideInputBuffer` | yes | yes |
| 139 | `LZ4_slideInputBufferHC` | yes | yes |
| 140 | `LZ4_uncompress` | yes | yes |
| 141 | `LZ4_uncompress_unknownOutputSize` | yes | yes |
| 142 | `LZ4_versionNumber` | yes | yes |
| 143 | `LZ4_versionString` | yes | yes |

## Undefined (imported) symbols in the Rust .so

All are glibc/loader-provided (allocation, memcpy/bcmp, stdio for the FILE
API, and the Rust runtime's std support):

```
_ITM_deregisterTMCloneTable
_ITM_registerTMCloneTable
_Unwind_Backtrace@GCC_3.3
_Unwind_GetDataRelBase@GCC_3.0
_Unwind_GetIP@GCC_3.0
_Unwind_GetIPInfo@GCC_4.2.0
_Unwind_GetLanguageSpecificData@GCC_3.0
_Unwind_GetRegionStart@GCC_3.0
_Unwind_GetTextRelBase@GCC_3.0
_Unwind_Resume@GCC_3.0
_Unwind_SetGR@GCC_3.0
_Unwind_SetIP@GCC_3.0
__cxa_finalize@GLIBC_2.2.5
__cxa_thread_atexit_impl@GLIBC_2.18
__errno_location@GLIBC_2.2.5
__gmon_start__
__tls_get_addr@GLIBC_2.3
abort@GLIBC_2.2.5
bcmp@GLIBC_2.2.5
calloc@GLIBC_2.2.5
close@GLIBC_2.2.5
dl_iterate_phdr@GLIBC_2.2.5
fread@GLIBC_2.2.5
free@GLIBC_2.2.5
fstat64@GLIBC_2.33
fwrite@GLIBC_2.2.5
getcwd@GLIBC_2.2.5
getenv@GLIBC_2.2.5
gettid@GLIBC_2.30
lseek64@GLIBC_2.2.5
malloc@GLIBC_2.2.5
memcpy@GLIBC_2.14
memmove@GLIBC_2.2.5
memset@GLIBC_2.2.5
mmap64@GLIBC_2.2.5
munmap@GLIBC_2.2.5
open64@GLIBC_2.2.5
posix_memalign@GLIBC_2.2.5
pthread_key_create@GLIBC_2.34
pthread_key_delete@GLIBC_2.34
pthread_setspecific@GLIBC_2.34
read@GLIBC_2.2.5
readlink@GLIBC_2.2.5
realloc@GLIBC_2.2.5
realpath@GLIBC_2.3
stat64@GLIBC_2.33
statx@GLIBC_2.28
strlen@GLIBC_2.2.5
syscall@GLIBC_2.2.5
write@GLIBC_2.2.5
writev@GLIBC_2.2.5
```
