# Configuration surface

Rows are generated from the complete dynamic-symbol list and the option/shape branches in the public headers and C sources. Repeated family descriptions intentionally retain every low-level and compatibility entry point.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `LZ4F_compressBegin` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 2 | `LZ4F_compressBegin_internal` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 3 | `LZ4F_compressBegin_usingCDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 4 | `LZ4F_compressBegin_usingDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 5 | `LZ4F_compressBegin_usingDictOnce` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 6 | `LZ4F_compressBound` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 7 | `LZ4F_compressEnd` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 8 | `LZ4F_compressFrame` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 9 | `LZ4F_compressFrameBound` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 10 | `LZ4F_compressFrame_usingCDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 11 | `LZ4F_compressUpdate` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 12 | `LZ4F_compressionLevel_max` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 13 | `LZ4F_createCDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 14 | `LZ4F_createCDict_advanced` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 15 | `LZ4F_createCompressionContext` | context allocation/free lifecycle using default and custom allocators; supported version and one-past version | [x] |
| 16 | `LZ4F_createCompressionContext_advanced` | context allocation/free lifecycle using default and custom allocators; supported version and one-past version | [x] |
| 17 | `LZ4F_createDecompressionContext` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 18 | `LZ4F_createDecompressionContext_advanced` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 19 | `LZ4F_decompress` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 20 | `LZ4F_decompress_usingDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 21 | `LZ4F_flush` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 22 | `LZ4F_freeCDict` | frame dictionary path; dictionary lengths 0,1,64,65535,65536,65537 and NULL/no-dictionary mode | [x] |
| 23 | `LZ4F_freeCompressionContext` | context allocation/free lifecycle using default and custom allocators; supported version and one-past version | [x] |
| 24 | `LZ4F_freeDecompressionContext` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 25 | `LZ4F_getBlockSize` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 26 | `LZ4F_getErrorCode` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 27 | `LZ4F_getErrorName` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 28 | `LZ4F_getFrameInfo` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 29 | `LZ4F_getVersion` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 30 | `LZ4F_headerSize` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 31 | `LZ4F_isError` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 32 | `LZ4F_read` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 33 | `LZ4F_readClose` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 34 | `LZ4F_readOpen` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 35 | `LZ4F_resetDecompressionContext` | frame decompression lifecycle; whole/chunked header and body, output sizes 0/1/exact/short, regular/skippable frame, with and without dictionary | [x] |
| 36 | `LZ4F_uncompressedUpdate` | frame compression cross-product: block size 0/4/5/6/7, linked/independent, content and block checksums off/on, level -1/0/9/10/12/13, autoFlush 0/1, favorDecSpeed 0/1; empty/small/block-boundary/multi-block input | [x] |
| 37 | `LZ4F_write` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 38 | `LZ4F_writeClose` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 39 | `LZ4F_writeOpen` | frame/file lifecycle; NULL/default preferences and blockSizeID 0/4/5/6/7; empty, one-byte, block-boundary, and multi-block data | [x] |
| 40 | `LZ4HC_searchExtDict` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 41 | `LZ4_XXH32` | one-shot hash; lengths 0,1,2,3,4,7,8,15,16,17,31,32,33,255; aligned and unaligned input; zero and nonzero seeds | [x] |
| 42 | `LZ4_XXH32_canonicalFromHash` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 43 | `LZ4_XXH32_copyState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 44 | `LZ4_XXH32_createState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 45 | `LZ4_XXH32_digest` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 46 | `LZ4_XXH32_freeState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 47 | `LZ4_XXH32_hashFromCanonical` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 48 | `LZ4_XXH32_reset` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 49 | `LZ4_XXH32_update` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 50 | `LZ4_XXH64` | one-shot hash; lengths 0,1,2,3,4,7,8,15,16,17,31,32,33,255; aligned and unaligned input; zero and nonzero seeds | [x] |
| 51 | `LZ4_XXH64_canonicalFromHash` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 52 | `LZ4_XXH64_copyState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 53 | `LZ4_XXH64_createState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 54 | `LZ4_XXH64_digest` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 55 | `LZ4_XXH64_freeState` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 56 | `LZ4_XXH64_hashFromCanonical` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 57 | `LZ4_XXH64_reset` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 58 | `LZ4_XXH64_update` | streaming hash lifecycle; empty input, one chunk, irregular chunks, zero-length NULL update, canonical conversion, state copy | [x] |
| 59 | `LZ4_XXH_versionNumber` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 60 | `LZ4_attach_HC_dictionary` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 61 | `LZ4_attach_dictionary` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 62 | `LZ4_compress` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 63 | `LZ4_compressBound` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 64 | `LZ4_compressHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 65 | `LZ4_compressHC2` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 66 | `LZ4_compressHC2_continue` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 67 | `LZ4_compressHC2_limitedOutput` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 68 | `LZ4_compressHC2_limitedOutput_continue` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 69 | `LZ4_compressHC2_limitedOutput_withStateHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 70 | `LZ4_compressHC2_withStateHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 71 | `LZ4_compressHC_continue` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 72 | `LZ4_compressHC_limitedOutput` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 73 | `LZ4_compressHC_limitedOutput_continue` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 74 | `LZ4_compressHC_limitedOutput_withStateHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 75 | `LZ4_compressHC_withStateHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 76 | `LZ4_compress_HC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 77 | `LZ4_compress_HC_continue` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 78 | `LZ4_compress_HC_continue_destSize` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 79 | `LZ4_compress_HC_destSize` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 80 | `LZ4_compress_HC_extStateHC` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 81 | `LZ4_compress_HC_extStateHC_fastReset` | HC compression; levels -1/0/1/2/9/10/12/13, empty/small/64KiB/large random and repetitive input, exact and constrained destination | [x] |
| 82 | `LZ4_compress_continue` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 83 | `LZ4_compress_default` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 84 | `LZ4_compress_destSize` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 85 | `LZ4_compress_destSize_extState` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 86 | `LZ4_compress_fast` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 87 | `LZ4_compress_fast_continue` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 88 | `LZ4_compress_fast_extState` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 89 | `LZ4_compress_fast_extState_fastReset` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 90 | `LZ4_compress_forceExtDict` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 91 | `LZ4_compress_limitedOutput` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 92 | `LZ4_compress_limitedOutput_continue` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 93 | `LZ4_compress_limitedOutput_withState` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 94 | `LZ4_compress_withState` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 95 | `LZ4_create` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 96 | `LZ4_createHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 97 | `LZ4_createStream` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 98 | `LZ4_createStreamDecode` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 99 | `LZ4_createStreamHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 100 | `LZ4_decoderRingBufferSize` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 101 | `LZ4_decompress_fast` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 102 | `LZ4_decompress_fast_continue` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 103 | `LZ4_decompress_fast_usingDict` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 104 | `LZ4_decompress_fast_withPrefix64k` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 105 | `LZ4_decompress_safe` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 106 | `LZ4_decompress_safe_continue` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 107 | `LZ4_decompress_safe_forceExtDict` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 108 | `LZ4_decompress_safe_partial` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 109 | `LZ4_decompress_safe_partial_forceExtDict` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 110 | `LZ4_decompress_safe_partial_usingDict` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 111 | `LZ4_decompress_safe_usingDict` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 112 | `LZ4_decompress_safe_withPrefix64k` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 113 | `LZ4_favorDecompressionSpeed` | compression shape matrix; empty/small/64KiB/large random and repetitive input, destination 0/1/exact bound/short, acceleration 0/1/2/65537/65538, one-shot/ext-state/streaming/destSize | [x] |
| 114 | `LZ4_freeHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 115 | `LZ4_freeStream` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 116 | `LZ4_freeStreamDecode` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 117 | `LZ4_freeStreamHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 118 | `LZ4_initStream` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 119 | `LZ4_initStreamHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 120 | `LZ4_loadDict` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 121 | `LZ4_loadDictHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 122 | `LZ4_loadDictSlow` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 123 | `LZ4_loadDict_internal` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 124 | `LZ4_resetStream` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 125 | `LZ4_resetStreamHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 126 | `LZ4_resetStreamHC_fast` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 127 | `LZ4_resetStreamState` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 128 | `LZ4_resetStreamStateHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 129 | `LZ4_resetStream_fast` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 130 | `LZ4_saveDict` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 131 | `LZ4_saveDictHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 132 | `LZ4_setCompressionLevel` | valid public ABI call using zero/default and representative nonzero inputs | [x] |
| 133 | `LZ4_setStreamDecode` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 134 | `LZ4_sizeofState` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 135 | `LZ4_sizeofStateHC` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 136 | `LZ4_sizeofStreamState` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 137 | `LZ4_sizeofStreamStateHC` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 138 | `LZ4_slideInputBuffer` | stream/context lifecycle; create or aligned external init, reset, empty/short/64KiB dictionary, attach/load/save, one and multiple blocks, free | [x] |
| 139 | `LZ4_slideInputBufferHC` | HC stream/context lifecycle; aligned exact-size external state, undersized/misaligned state, empty/short/64KiB dictionary, reset/load/save/attach/favor modes | [x] |
| 140 | `LZ4_uncompress` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 141 | `LZ4_uncompress_unknownOutputSize` | decompression shape matrix; empty/malformed/valid compressed input, output 0/1/exact/short, full and partial decode, no dictionary/prefix/external dictionary, one-shot and streaming | [x] |
| 142 | `LZ4_versionNumber` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
| 143 | `LZ4_versionString` | metadata/scalar boundary values, including 0, valid extrema, and one-past-range | [x] |
