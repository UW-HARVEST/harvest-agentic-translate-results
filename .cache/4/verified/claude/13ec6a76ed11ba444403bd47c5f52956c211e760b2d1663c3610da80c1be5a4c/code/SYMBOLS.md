# SYMBOLS.md — Exported symbol parity (C `.so` vs Rust `.so`)

Generated mechanically from `nm -D --defined-only` on both shared libraries.

```
C   .so: c_src/build/liblz4.so
Rust.so: target/release/liblz4.so
```

## Summary

| metric | count |
|--------|-------|
| symbols exported by C `.so`     | 143 |
| symbols exported by Rust `.so`  | 143 |
| **missing from Rust**            | **0** |
| extra in Rust (not in C)         | 0 |
| undefined non-libc in Rust `.so`| 0 |

**Result: symbol surface is at full parity — 0 missing, 0 extra.**

All undefined symbols in the Rust `.so` resolve to glibc
(`malloc`, `free`, `calloc`, `realloc`, `memcpy`, `memmove`, `memset`, `bcmp`,
`fread`, `fwrite`, `read`, `write`, `open64`, `close`, `lseek64`, `fstat64`, ...)
plus the Rust standard-library runtime's own libc imports. No unresolved
translation-level symbols remain.

## Full symbol table

`present` = exported by that `.so` with the exact same name.

| # | symbol | C .so | Rust .so | owning C translation unit |
|---|--------|-------|----------|---------------------------|
| 1 | `LZ4F_compressBegin` | yes | yes | lz4frame.c |
| 2 | `LZ4F_compressBegin_internal` | yes | yes | lz4frame.c |
| 3 | `LZ4F_compressBegin_usingCDict` | yes | yes | lz4frame.c |
| 4 | `LZ4F_compressBegin_usingDict` | yes | yes | lz4frame.c |
| 5 | `LZ4F_compressBegin_usingDictOnce` | yes | yes | lz4frame.c |
| 6 | `LZ4F_compressBound` | yes | yes | lz4frame.c |
| 7 | `LZ4F_compressEnd` | yes | yes | lz4frame.c |
| 8 | `LZ4F_compressFrame` | yes | yes | lz4frame.c |
| 9 | `LZ4F_compressFrameBound` | yes | yes | lz4frame.c |
| 10 | `LZ4F_compressFrame_usingCDict` | yes | yes | lz4frame.c |
| 11 | `LZ4F_compressUpdate` | yes | yes | lz4frame.c |
| 12 | `LZ4F_compressionLevel_max` | yes | yes | lz4frame.c |
| 13 | `LZ4F_createCDict` | yes | yes | lz4frame.c |
| 14 | `LZ4F_createCDict_advanced` | yes | yes | lz4frame.c |
| 15 | `LZ4F_createCompressionContext` | yes | yes | lz4frame.c |
| 16 | `LZ4F_createCompressionContext_advanced` | yes | yes | lz4frame.c |
| 17 | `LZ4F_createDecompressionContext` | yes | yes | lz4frame.c |
| 18 | `LZ4F_createDecompressionContext_advanced` | yes | yes | lz4frame.c |
| 19 | `LZ4F_decompress` | yes | yes | lz4frame.c |
| 20 | `LZ4F_decompress_usingDict` | yes | yes | lz4frame.c |
| 21 | `LZ4F_flush` | yes | yes | lz4frame.c |
| 22 | `LZ4F_freeCDict` | yes | yes | lz4frame.c |
| 23 | `LZ4F_freeCompressionContext` | yes | yes | lz4frame.c |
| 24 | `LZ4F_freeDecompressionContext` | yes | yes | lz4frame.c |
| 25 | `LZ4F_getBlockSize` | yes | yes | lz4frame.c |
| 26 | `LZ4F_getErrorCode` | yes | yes | lz4frame.c |
| 27 | `LZ4F_getErrorName` | yes | yes | lz4frame.c |
| 28 | `LZ4F_getFrameInfo` | yes | yes | lz4frame.c |
| 29 | `LZ4F_getVersion` | yes | yes | lz4frame.c |
| 30 | `LZ4F_headerSize` | yes | yes | lz4frame.c |
| 31 | `LZ4F_isError` | yes | yes | lz4frame.c |
| 32 | `LZ4F_read` | yes | yes | lz4file.c |
| 33 | `LZ4F_readClose` | yes | yes | lz4file.c |
| 34 | `LZ4F_readOpen` | yes | yes | lz4file.c |
| 35 | `LZ4F_resetDecompressionContext` | yes | yes | lz4frame.c |
| 36 | `LZ4F_uncompressedUpdate` | yes | yes | lz4frame.c |
| 37 | `LZ4F_write` | yes | yes | lz4file.c |
| 38 | `LZ4F_writeClose` | yes | yes | lz4file.c |
| 39 | `LZ4F_writeOpen` | yes | yes | lz4file.c |
| 40 | `LZ4HC_searchExtDict` | yes | yes | lz4hc.c |
| 41 | `LZ4_XXH32` | yes | yes | xxhash.c |
| 42 | `LZ4_XXH32_canonicalFromHash` | yes | yes | xxhash.c |
| 43 | `LZ4_XXH32_copyState` | yes | yes | xxhash.c |
| 44 | `LZ4_XXH32_createState` | yes | yes | xxhash.c |
| 45 | `LZ4_XXH32_digest` | yes | yes | xxhash.c |
| 46 | `LZ4_XXH32_freeState` | yes | yes | xxhash.c |
| 47 | `LZ4_XXH32_hashFromCanonical` | yes | yes | xxhash.c |
| 48 | `LZ4_XXH32_reset` | yes | yes | xxhash.c |
| 49 | `LZ4_XXH32_update` | yes | yes | xxhash.c |
| 50 | `LZ4_XXH64` | yes | yes | xxhash.c |
| 51 | `LZ4_XXH64_canonicalFromHash` | yes | yes | xxhash.c |
| 52 | `LZ4_XXH64_copyState` | yes | yes | xxhash.c |
| 53 | `LZ4_XXH64_createState` | yes | yes | xxhash.c |
| 54 | `LZ4_XXH64_digest` | yes | yes | xxhash.c |
| 55 | `LZ4_XXH64_freeState` | yes | yes | xxhash.c |
| 56 | `LZ4_XXH64_hashFromCanonical` | yes | yes | xxhash.c |
| 57 | `LZ4_XXH64_reset` | yes | yes | xxhash.c |
| 58 | `LZ4_XXH64_update` | yes | yes | xxhash.c |
| 59 | `LZ4_XXH_versionNumber` | yes | yes | xxhash.c |
| 60 | `LZ4_attach_HC_dictionary` | yes | yes | lz4hc.c |
| 61 | `LZ4_attach_dictionary` | yes | yes | lz4.c |
| 62 | `LZ4_compress` | yes | yes | lz4.c |
| 63 | `LZ4_compressBound` | yes | yes | lz4.c |
| 64 | `LZ4_compressHC` | yes | yes | lz4hc.c |
| 65 | `LZ4_compressHC2` | yes | yes | lz4hc.c |
| 66 | `LZ4_compressHC2_continue` | yes | yes | lz4hc.c |
| 67 | `LZ4_compressHC2_limitedOutput` | yes | yes | lz4hc.c |
| 68 | `LZ4_compressHC2_limitedOutput_continue` | yes | yes | lz4hc.c |
| 69 | `LZ4_compressHC2_limitedOutput_withStateHC` | yes | yes | lz4hc.c |
| 70 | `LZ4_compressHC2_withStateHC` | yes | yes | lz4hc.c |
| 71 | `LZ4_compressHC_continue` | yes | yes | lz4hc.c |
| 72 | `LZ4_compressHC_limitedOutput` | yes | yes | lz4hc.c |
| 73 | `LZ4_compressHC_limitedOutput_continue` | yes | yes | lz4hc.c |
| 74 | `LZ4_compressHC_limitedOutput_withStateHC` | yes | yes | lz4hc.c |
| 75 | `LZ4_compressHC_withStateHC` | yes | yes | lz4hc.c |
| 76 | `LZ4_compress_HC` | yes | yes | lz4hc.c |
| 77 | `LZ4_compress_HC_continue` | yes | yes | lz4hc.c |
| 78 | `LZ4_compress_HC_continue_destSize` | yes | yes | lz4hc.c |
| 79 | `LZ4_compress_HC_destSize` | yes | yes | lz4hc.c |
| 80 | `LZ4_compress_HC_extStateHC` | yes | yes | lz4hc.c |
| 81 | `LZ4_compress_HC_extStateHC_fastReset` | yes | yes | lz4hc.c |
| 82 | `LZ4_compress_continue` | yes | yes | lz4.c |
| 83 | `LZ4_compress_default` | yes | yes | lz4.c |
| 84 | `LZ4_compress_destSize` | yes | yes | lz4.c |
| 85 | `LZ4_compress_destSize_extState` | yes | yes | lz4.c |
| 86 | `LZ4_compress_fast` | yes | yes | lz4.c |
| 87 | `LZ4_compress_fast_continue` | yes | yes | lz4.c |
| 88 | `LZ4_compress_fast_extState` | yes | yes | lz4.c |
| 89 | `LZ4_compress_fast_extState_fastReset` | yes | yes | lz4.c |
| 90 | `LZ4_compress_forceExtDict` | yes | yes | lz4.c |
| 91 | `LZ4_compress_limitedOutput` | yes | yes | lz4.c |
| 92 | `LZ4_compress_limitedOutput_continue` | yes | yes | lz4.c |
| 93 | `LZ4_compress_limitedOutput_withState` | yes | yes | lz4.c |
| 94 | `LZ4_compress_withState` | yes | yes | lz4.c |
| 95 | `LZ4_create` | yes | yes | lz4.c |
| 96 | `LZ4_createHC` | yes | yes | lz4hc.c |
| 97 | `LZ4_createStream` | yes | yes | lz4.c |
| 98 | `LZ4_createStreamDecode` | yes | yes | lz4.c |
| 99 | `LZ4_createStreamHC` | yes | yes | lz4hc.c |
| 100 | `LZ4_decoderRingBufferSize` | yes | yes | lz4.c |
| 101 | `LZ4_decompress_fast` | yes | yes | lz4.c |
| 102 | `LZ4_decompress_fast_continue` | yes | yes | lz4.c |
| 103 | `LZ4_decompress_fast_usingDict` | yes | yes | lz4.c |
| 104 | `LZ4_decompress_fast_withPrefix64k` | yes | yes | lz4.c |
| 105 | `LZ4_decompress_safe` | yes | yes | lz4.c |
| 106 | `LZ4_decompress_safe_continue` | yes | yes | lz4.c |
| 107 | `LZ4_decompress_safe_forceExtDict` | yes | yes | lz4.c |
| 108 | `LZ4_decompress_safe_partial` | yes | yes | lz4.c |
| 109 | `LZ4_decompress_safe_partial_forceExtDict` | yes | yes | lz4.c |
| 110 | `LZ4_decompress_safe_partial_usingDict` | yes | yes | lz4.c |
| 111 | `LZ4_decompress_safe_usingDict` | yes | yes | lz4.c |
| 112 | `LZ4_decompress_safe_withPrefix64k` | yes | yes | lz4.c |
| 113 | `LZ4_favorDecompressionSpeed` | yes | yes | lz4hc.c |
| 114 | `LZ4_freeHC` | yes | yes | lz4hc.c |
| 115 | `LZ4_freeStream` | yes | yes | lz4.c |
| 116 | `LZ4_freeStreamDecode` | yes | yes | lz4.c |
| 117 | `LZ4_freeStreamHC` | yes | yes | lz4hc.c |
| 118 | `LZ4_initStream` | yes | yes | lz4.c |
| 119 | `LZ4_initStreamHC` | yes | yes | lz4hc.c |
| 120 | `LZ4_loadDict` | yes | yes | lz4.c |
| 121 | `LZ4_loadDictHC` | yes | yes | lz4hc.c |
| 122 | `LZ4_loadDictSlow` | yes | yes | lz4.c |
| 123 | `LZ4_loadDict_internal` | yes | yes | lz4.c |
| 124 | `LZ4_resetStream` | yes | yes | lz4.c |
| 125 | `LZ4_resetStreamHC` | yes | yes | lz4hc.c |
| 126 | `LZ4_resetStreamHC_fast` | yes | yes | lz4hc.c |
| 127 | `LZ4_resetStreamState` | yes | yes | lz4.c |
| 128 | `LZ4_resetStreamStateHC` | yes | yes | lz4hc.c |
| 129 | `LZ4_resetStream_fast` | yes | yes | lz4.c |
| 130 | `LZ4_saveDict` | yes | yes | lz4.c |
| 131 | `LZ4_saveDictHC` | yes | yes | lz4hc.c |
| 132 | `LZ4_setCompressionLevel` | yes | yes | lz4hc.c |
| 133 | `LZ4_setStreamDecode` | yes | yes | lz4.c |
| 134 | `LZ4_sizeofState` | yes | yes | lz4.c |
| 135 | `LZ4_sizeofStateHC` | yes | yes | lz4hc.c |
| 136 | `LZ4_sizeofStreamState` | yes | yes | lz4.c |
| 137 | `LZ4_sizeofStreamStateHC` | yes | yes | lz4hc.c |
| 138 | `LZ4_slideInputBuffer` | yes | yes | lz4.c |
| 139 | `LZ4_slideInputBufferHC` | yes | yes | lz4hc.c |
| 140 | `LZ4_uncompress` | yes | yes | lz4.c |
| 141 | `LZ4_uncompress_unknownOutputSize` | yes | yes | lz4.c |
| 142 | `LZ4_versionNumber` | yes | yes | lz4.c |
| 143 | `LZ4_versionString` | yes | yes | lz4.c |

## Per-translation-unit breakdown

| C source file | exported symbols | Rust module | all present |
|---------------|------------------|-------------|-------------|
| lz4.c | 50 | src/lz4.rs | yes |
| lz4hc.c | 35 | src/lz4hc.rs | yes |
| lz4frame.c | 33 | src/lz4frame.rs | yes |
| lz4file.c | 6 | src/lz4file.rs | yes |
| xxhash.c | 19 | src/xxhash.rs | yes |
