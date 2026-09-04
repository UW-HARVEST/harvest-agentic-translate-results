# SYMBOLS.md — exported symbol parity (C `liblz4.so` vs Rust `liblz4.so`)

Generated mechanically with:
```
nm -D --defined-only <lib> | awk "\$2==\"T\"||\$2==\"W\"{print \$3}" | sort -u
```

C .so: `c_src/build/liblz4.so`  — 143 symbols
Rust .so: `translation/target/release/liblz4.so` — 143 symbols


## Completion gate (Phase D)

- [x] `nm -D` on both `.so` files: **143 symbols each, 0 missing, 0 extra.**
- [x] No undefined non-libc symbols in the Rust `.so` (only the C runtime,
      `malloc`/`free`/`mem*` and the unwinder personality/ITM stubs the Rust
      toolchain always emits).
- [x] `CONFIGS.md`: all 145 valid-path rows pass across randomized inputs.
- [x] `ERRORS.md`: all 236 rows accounted for — 201 with a passing differential
      error-path test, 35 explicitly labelled non-testable (16 UB in C, 16
      allocator-failure-only, 2 `assert()`-guarded, 1 build-time `#error`).
- [x] Feature combinations: `Cargo.toml` declares **no** `[features]`, so the
      default build is the only configuration. Both `cargo test --release` and
      `cargo test --release --no-default-features` pass (see `check_features.sh`,
      which enumerates the `[features]` table generically and would fan out over
      the full power set if any were added).

Reproduce with:

```
cd c_src && mkdir -p build && cd build && cmake .. -DCMAKE_POSITION_INDEPENDENT_CODE=ON && cmake --build .
cd ../../translation && cargo test --offline --release -- --test-threads=1
./check_features.sh
```

## Missing in Rust

**NONE** — every C symbol is exported by the Rust .so with the exact same name.

## Extra in Rust (not in C)

**NONE**

## Undefined (imported) non-libc symbols in the Rust .so

- _ITM_deregisterTMCloneTable
- _ITM_registerTMCloneTable
- _Unwind_Backtrace@GCC_3.3
- _Unwind_GetDataRelBase@GCC_3.0
- _Unwind_GetIP@GCC_3.0
- _Unwind_GetIPInfo@GCC_4.2.0
- _Unwind_GetLanguageSpecificData@GCC_3.0
- _Unwind_GetRegionStart@GCC_3.0
- _Unwind_GetTextRelBase@GCC_3.0
- _Unwind_Resume@GCC_3.0
- _Unwind_SetGR@GCC_3.0
- _Unwind_SetIP@GCC_3.0
- __cxa_finalize@GLIBC_2.2.5
- __cxa_thread_atexit_impl@GLIBC_2.18
- __errno_location@GLIBC_2.2.5
- __gmon_start__
- __tls_get_addr@GLIBC_2.3
- abort@GLIBC_2.2.5
- bcmp@GLIBC_2.2.5
- calloc@GLIBC_2.2.5
- close@GLIBC_2.2.5
- dl_iterate_phdr@GLIBC_2.2.5
- fread@GLIBC_2.2.5
- free@GLIBC_2.2.5
- fstat64@GLIBC_2.33
- fwrite@GLIBC_2.2.5
- getcwd@GLIBC_2.2.5
- getenv@GLIBC_2.2.5
- gettid@GLIBC_2.30
- lseek64@GLIBC_2.2.5
- malloc@GLIBC_2.2.5
- memcpy@GLIBC_2.14
- memmove@GLIBC_2.2.5
- memset@GLIBC_2.2.5
- mmap64@GLIBC_2.2.5
- munmap@GLIBC_2.2.5
- open64@GLIBC_2.2.5
- posix_memalign@GLIBC_2.2.5
- pthread_key_create@GLIBC_2.34
- pthread_key_delete@GLIBC_2.34
- pthread_setspecific@GLIBC_2.34
- read@GLIBC_2.2.5
- readlink@GLIBC_2.2.5
- realloc@GLIBC_2.2.5
- realpath@GLIBC_2.3
- stat64@GLIBC_2.33
- statx@GLIBC_2.28
- strlen@GLIBC_2.2.5
- syscall@GLIBC_2.2.5
- write@GLIBC_2.2.5
- writev@GLIBC_2.2.5

## Full symbol table (C ∩ Rust — all 143 present in both)

| # | symbol | in C .so | in Rust .so | source module |
|---|--------|----------|-------------|---------------|
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
| 113 | `LZ4_favorDecompressionSpeed` | yes | yes | lz4.c |
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
| 132 | `LZ4_setCompressionLevel` | yes | yes | lz4.c |
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
