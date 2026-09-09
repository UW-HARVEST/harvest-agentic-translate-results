# Configuration surface

One mechanically enumerated row per exported C entry point. Each row records the option and input-shape axes selected from the public headers and the implementation branches for that family. Shared lifecycle rows are intentionally repeated per entry point so no low-level or compatibility export disappears from coverage.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---:|----------------|--------------------------------------------|:---:|
| 1 | `LZ4F_compressBegin` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 2 | `LZ4F_compressBegin_internal` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 3 | `LZ4F_compressBegin_usingCDict` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 4 | `LZ4F_compressBegin_usingDict` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 5 | `LZ4F_compressBegin_usingDictOnce` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 6 | `LZ4F_compressBound` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 7 | `LZ4F_compressEnd` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 8 | `LZ4F_compressFrame` | frame one-shot: NULL/default and explicit preferences; block size 0/4/5/6/7; linked/independent; checksums off/on; content size unknown/known; dictID 0/nonzero; compression level negative/0/HC; autoFlush 0/1; favorDecSpeed 0/1; empty/small/block-boundary/multiblock/random input | [x] |
| 9 | `LZ4F_compressFrameBound` | frame one-shot: NULL/default and explicit preferences; block size 0/4/5/6/7; linked/independent; checksums off/on; content size unknown/known; dictID 0/nonzero; compression level negative/0/HC; autoFlush 0/1; favorDecSpeed 0/1; empty/small/block-boundary/multiblock/random input | [x] |
| 10 | `LZ4F_compressFrame_usingCDict` | frame one-shot: NULL/default and explicit preferences; block size 0/4/5/6/7; linked/independent; checksums off/on; content size unknown/known; dictID 0/nonzero; compression level negative/0/HC; autoFlush 0/1; favorDecSpeed 0/1; empty/small/block-boundary/multiblock/random input | [x] |
| 11 | `LZ4F_compressUpdate` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 12 | `LZ4F_compressionLevel_max` | no-input metadata/version result | [x] |
| 13 | `LZ4F_createCDict` | digested dictionary lifecycle: dict lengths 0, short, 64KiB, >64KiB; default/custom allocator; NULL free | [x] |
| 14 | `LZ4F_createCDict_advanced` | digested dictionary lifecycle: dict lengths 0, short, 64KiB, >64KiB; default/custom allocator; NULL free | [x] |
| 15 | `LZ4F_createCompressionContext` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 16 | `LZ4F_createCompressionContext_advanced` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 17 | `LZ4F_createDecompressionContext` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 18 | `LZ4F_createDecompressionContext_advanced` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 19 | `LZ4F_decompress` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 20 | `LZ4F_decompress_usingDict` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 21 | `LZ4F_flush` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 22 | `LZ4F_freeCDict` | digested dictionary lifecycle: dict lengths 0, short, 64KiB, >64KiB; default/custom allocator; NULL free | [x] |
| 23 | `LZ4F_freeCompressionContext` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 24 | `LZ4F_freeDecompressionContext` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 25 | `LZ4F_getBlockSize` | block-size enum: default(0), 4, 5, 6, 7 | [x] |
| 26 | `LZ4F_getErrorCode` | error-code domain: success, each encoded error, arbitrary non-error size_t | [x] |
| 27 | `LZ4F_getErrorName` | error-code domain: success, each encoded error, arbitrary non-error size_t | [x] |
| 28 | `LZ4F_getFrameInfo` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 29 | `LZ4F_getVersion` | no-input metadata/version result | [x] |
| 30 | `LZ4F_headerSize` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 31 | `LZ4F_isError` | error-code domain: success, each encoded error, arbitrary non-error size_t | [x] |
| 32 | `LZ4F_read` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 33 | `LZ4F_readClose` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 34 | `LZ4F_readOpen` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 35 | `LZ4F_resetDecompressionContext` | frame streaming decompression lifecycle: context create; header split at every byte; body split/random chunks; output capacities 0/small/exact/large; stableDst 0/1; skipChecksums 0/1; dictionary none/present; reset/reuse | [x] |
| 36 | `LZ4F_uncompressedUpdate` | frame streaming compression lifecycle: context create; begin; update zero/one/many chunks; compressed and uncompressed update; flush empty/nonempty; end; stableSrc 0/1; default/full preferences; dictionary none/raw/CDict | [x] |
| 37 | `LZ4F_write` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 38 | `LZ4F_writeClose` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 39 | `LZ4F_writeOpen` | FILE* lifecycle: open/read-or-write/close; empty, small, multiblock files; default and explicit frame preferences | [x] |
| 40 | `LZ4HC_searchExtDict` | internal HC dictionary search: no dictionary and external dictionary; MINMATCH and distance boundaries | [x] |
| 41 | `LZ4_XXH32` | one-shot hash: lengths 0, 1, word-1, word, stripe-1, stripe, stripe+1, many; aligned/unaligned input; seeds 0 and nonzero | [x] |
| 42 | `LZ4_XXH32_canonicalFromHash` | canonical hash conversion: zero, all-ones, random values; big-endian byte representation and inverse | [x] |
| 43 | `LZ4_XXH32_copyState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 44 | `LZ4_XXH32_createState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 45 | `LZ4_XXH32_digest` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 46 | `LZ4_XXH32_freeState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 47 | `LZ4_XXH32_hashFromCanonical` | canonical hash conversion: zero, all-ones, random values; big-endian byte representation and inverse | [x] |
| 48 | `LZ4_XXH32_reset` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 49 | `LZ4_XXH32_update` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 50 | `LZ4_XXH64` | one-shot hash: lengths 0, 1, word-1, word, stripe-1, stripe, stripe+1, many; aligned/unaligned input; seeds 0 and nonzero | [x] |
| 51 | `LZ4_XXH64_canonicalFromHash` | canonical hash conversion: zero, all-ones, random values; big-endian byte representation and inverse | [x] |
| 52 | `LZ4_XXH64_copyState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 53 | `LZ4_XXH64_createState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 54 | `LZ4_XXH64_digest` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 55 | `LZ4_XXH64_freeState` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 56 | `LZ4_XXH64_hashFromCanonical` | canonical hash conversion: zero, all-ones, random values; big-endian byte representation and inverse | [x] |
| 57 | `LZ4_XXH64_reset` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 58 | `LZ4_XXH64_update` | stream hash lifecycle: create/reset; zero/one/many updates with varied segmentation and unaligned chunks; digest midstream/final; copy state | [x] |
| 59 | `LZ4_XXH_versionNumber` | no-input metadata/version result | [x] |
| 60 | `LZ4_attach_HC_dictionary` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 61 | `LZ4_attach_dictionary` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 62 | `LZ4_compress` | block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max | [x] |
| 63 | `LZ4_compressBound` | block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max | [x] |
| 64 | `LZ4_compressHC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 65 | `LZ4_compressHC2` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 66 | `LZ4_compressHC2_continue` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 67 | `LZ4_compressHC2_limitedOutput` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 68 | `LZ4_compressHC2_limitedOutput_continue` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 69 | `LZ4_compressHC2_limitedOutput_withStateHC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 70 | `LZ4_compressHC2_withStateHC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 71 | `LZ4_compressHC_continue` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 72 | `LZ4_compressHC_limitedOutput` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 73 | `LZ4_compressHC_limitedOutput_continue` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 74 | `LZ4_compressHC_limitedOutput_withStateHC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 75 | `LZ4_compressHC_withStateHC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 76 | `LZ4_compress_HC` | HC block compression: levels below-min/2/9/10/12/>12; favor-decode off/on where applicable; empty/small/large random and repetitive input; bound/tight destination | [x] |
| 77 | `LZ4_compress_HC_continue` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 78 | `LZ4_compress_HC_continue_destSize` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 79 | `LZ4_compress_HC_destSize` | HC external state/fill-output: aligned state; fresh/fast-reset reuse; levels 2/9/10/12; destination exact/tight; consumed source count | [x] |
| 80 | `LZ4_compress_HC_extStateHC` | HC external state/fill-output: aligned state; fresh/fast-reset reuse; levels 2/9/10/12; destination exact/tight; consumed source count | [x] |
| 81 | `LZ4_compress_HC_extStateHC_fastReset` | HC external state/fill-output: aligned state; fresh/fast-reset reuse; levels 2/9/10/12; destination exact/tight; consumed source count | [x] |
| 82 | `LZ4_compress_continue` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 83 | `LZ4_compress_default` | block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max | [x] |
| 84 | `LZ4_compress_destSize` | fill-output compression: source sizes empty/small/large; destination sizes 1/small/exact/bound; verify consumed source count and round trip | [x] |
| 85 | `LZ4_compress_destSize_extState` | fill-output compression: source sizes empty/small/large; destination sizes 1/small/exact/bound; verify consumed source count and round trip | [x] |
| 86 | `LZ4_compress_fast` | block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max | [x] |
| 87 | `LZ4_compress_fast_continue` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 88 | `LZ4_compress_fast_extState` | external-state block compression: aligned allocated state; fresh and fast-reset reuse; input/capacity/acceleration matrix | [x] |
| 89 | `LZ4_compress_fast_extState_fastReset` | external-state block compression: aligned allocated state; fresh and fast-reset reuse; input/capacity/acceleration matrix | [x] |
| 90 | `LZ4_compress_forceExtDict` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 91 | `LZ4_compress_limitedOutput` | block compression: empty/small/MINMATCH boundaries/255/64KiB/random and repetitive input; destination exact bound/tight; acceleration <=0/1/2/max/>max | [x] |
| 92 | `LZ4_compress_limitedOutput_continue` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 93 | `LZ4_compress_limitedOutput_withState` | external-state block compression: aligned allocated state; fresh and fast-reset reuse; input/capacity/acceleration matrix | [x] |
| 94 | `LZ4_compress_withState` | external-state block compression: aligned allocated state; fresh and fast-reset reuse; input/capacity/acceleration matrix | [x] |
| 95 | `LZ4_create` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 96 | `LZ4_createHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 97 | `LZ4_createStream` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 98 | `LZ4_createStreamDecode` | stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB | [x] |
| 99 | `LZ4_createStreamHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 100 | `LZ4_decoderRingBufferSize` | stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB | [x] |
| 101 | `LZ4_decompress_fast` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 102 | `LZ4_decompress_fast_continue` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 103 | `LZ4_decompress_fast_usingDict` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 104 | `LZ4_decompress_fast_withPrefix64k` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 105 | `LZ4_decompress_safe` | safe block decompression: valid blocks from all compressor modes; empty/small/exact/oversized destination; malformed/truncated/random compressed input | [x] |
| 106 | `LZ4_decompress_safe_continue` | stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB | [x] |
| 107 | `LZ4_decompress_safe_forceExtDict` | dictionary safe decompression: no dictionary; <64KiB, 64KiB and >64KiB dictionaries; prefix-adjacent and external dictionary layouts | [x] |
| 108 | `LZ4_decompress_safe_partial` | partial safe decompression: targets 0/1/middle/exact/>decoded, capacities target/exact/large; dictionary none/prefix/external | [x] |
| 109 | `LZ4_decompress_safe_partial_forceExtDict` | partial safe decompression: targets 0/1/middle/exact/>decoded, capacities target/exact/large; dictionary none/prefix/external | [x] |
| 110 | `LZ4_decompress_safe_partial_usingDict` | partial safe decompression: targets 0/1/middle/exact/>decoded, capacities target/exact/large; dictionary none/prefix/external | [x] |
| 111 | `LZ4_decompress_safe_usingDict` | dictionary safe decompression: no dictionary; <64KiB, 64KiB and >64KiB dictionaries; prefix-adjacent and external dictionary layouts | [x] |
| 112 | `LZ4_decompress_safe_withPrefix64k` | dictionary safe decompression: no dictionary; <64KiB, 64KiB and >64KiB dictionaries; prefix-adjacent and external dictionary layouts | [x] |
| 113 | `LZ4_favorDecompressionSpeed` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 114 | `LZ4_freeHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 115 | `LZ4_freeStream` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 116 | `LZ4_freeStreamDecode` | stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB | [x] |
| 117 | `LZ4_freeStreamHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 118 | `LZ4_initStream` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 119 | `LZ4_initStreamHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 120 | `LZ4_loadDict` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 121 | `LZ4_loadDictHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 122 | `LZ4_loadDictSlow` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 123 | `LZ4_loadDict_internal` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 124 | `LZ4_resetStream` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 125 | `LZ4_resetStreamHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 126 | `LZ4_resetStreamHC_fast` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 127 | `LZ4_resetStreamState` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 128 | `LZ4_resetStreamStateHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 129 | `LZ4_resetStream_fast` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 130 | `LZ4_saveDict` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 131 | `LZ4_saveDictHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 132 | `LZ4_setCompressionLevel` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 133 | `LZ4_setStreamDecode` | stream decoding lifecycle: create/set dictionary/reset; one/many blocks; contiguous/noncontiguous/ring output; dictionary none/short/64KiB | [x] |
| 134 | `LZ4_sizeofState` | no-input metadata/version result | [x] |
| 135 | `LZ4_sizeofStateHC` | public lifecycle/compatibility wrapper exercised with valid zero, boundary, and randomized inputs appropriate to its C prototype | [x] |
| 136 | `LZ4_sizeofStreamState` | no-input metadata/version result | [x] |
| 137 | `LZ4_sizeofStreamStateHC` | public lifecycle/compatibility wrapper exercised with valid zero, boundary, and randomized inputs appropriate to its C prototype | [x] |
| 138 | `LZ4_slideInputBuffer` | fast streaming compression lifecycle: create/init/reset; no/loaded/slow/attached dictionary; one/many contiguous and noncontiguous blocks; save dictionary; acceleration matrix | [x] |
| 139 | `LZ4_slideInputBufferHC` | HC streaming lifecycle: create/init/reset; level 2/9/10/12; favorDecSpeed 0/1; no/loaded/attached dictionary; one/many blocks; save dictionary; continue and fill-output | [x] |
| 140 | `LZ4_uncompress` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 141 | `LZ4_uncompress_unknownOutputSize` | legacy fast decompression: exact original size; no/prefix/external dictionary; streaming continuation | [x] |
| 142 | `LZ4_versionNumber` | no-input metadata/version result | [x] |
| 143 | `LZ4_versionString` | no-input metadata/version result | [x] |

## Build-time configuration

Cargo declares no features. The only Rust feature sets are therefore the equivalent default and `--no-default-features` builds. CMake and `build.rs` both fix `XXH_NAMESPACE=LZ4_`, `LZ4_HEAPMODE=0`, and `LZ4F_HEAPMODE=0`.
