# Configuration surface

Rows are the source-level cross-product after pruning combinations that take
the same C branch. Sizes use boundary classes: empty, one byte, below/at/above
the parser threshold, below/at/above a block boundary, and many blocks.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| 1 | `LZ4_versionNumber`, `LZ4_versionString`, `LZ4_sizeofState`, `LZ4_sizeofStreamState` | scalar/metadata queries | [x] |
| 2 | `LZ4_compressBound`, `LZ4_decoderRingBufferSize` | valid boundaries `0,1,64KiB,LZ4_MAX_INPUT_SIZE` | [x] |
| 3 | `LZ4_compress_default`, `LZ4_compress`, `LZ4_compress_limitedOutput` | empty, tiny, compressible, incompressible; exact bound and limited output | [x] |
| 4 | `LZ4_compress_fast` | acceleration `<=0,1,2,65537,>65537`; tiny and large random input | [x] |
| 5 | `LZ4_compress_fast_extState`, `LZ4_compress_fast_extState_fastReset` | aligned external state; fresh and reused; bounded/unbounded output | [x] |
| 6 | `LZ4_compress_withState`, `LZ4_compress_limitedOutput_withState` | legacy external state; empty/tiny/many input | [x] |
| 7 | `LZ4_compress_destSize`, `LZ4_compress_destSize_extState` | target `1`, below bound, exact bound; partial/full source consumption | [x] |
| 8 | `LZ4_createStream`, `LZ4_initStream`, `LZ4_resetStream`, `LZ4_resetStream_fast`, `LZ4_freeStream` | heap/external stream lifecycle and reuse | [x] |
| 9 | `LZ4_loadDict`, `LZ4_loadDictSlow`, `LZ4_loadDict_internal` | dictionary empty, `<HASH_UNIT`, `<64KiB`, `64KiB`, `>64KiB`; fast/slow mode | [x] |
| 10 | `LZ4_attach_dictionary`, `LZ4_compress_fast_continue` | no dictionary, loaded dictionary, attached dictionary; contiguous/noncontiguous blocks | [x] |
| 11 | `LZ4_compress_continue`, `LZ4_compress_limitedOutput_continue`, `LZ4_compress_forceExtDict` | legacy/forced external dictionary; limited and bound-sized output | [x] |
| 12 | `LZ4_saveDict`, `LZ4_slideInputBuffer`, `LZ4_create`, `LZ4_resetStreamState` | save sizes `0,<64KiB,64KiB,>64KiB`; legacy lifecycle | [x] |
| 13 | `LZ4_decompress_safe`, `LZ4_uncompress_unknownOutputSize` | empty block, literals only, matches; exact and oversized destination | [x] |
| 14 | `LZ4_decompress_safe_partial` | target `0,1,<decoded,=decoded,>decoded`; capacities around target | [x] |
| 15 | `LZ4_decompress_fast`, `LZ4_uncompress` | empty/tiny/random valid blocks and exact original size | [x] |
| 16 | `LZ4_decompress_safe_withPrefix64k`, `LZ4_decompress_fast_withPrefix64k` | match in 64KiB prefix and no-prefix-needed block | [x] |
| 17 | `LZ4_decompress_safe_forceExtDict`, `LZ4_decompress_safe_partial_forceExtDict` | external dictionary sizes `0,small,64KiB,>64KiB`; full/partial | [x] |
| 18 | `LZ4_decompress_safe_usingDict`, `LZ4_decompress_safe_partial_usingDict`, `LZ4_decompress_fast_usingDict` | contiguous prefix vs noncontiguous external dictionary | [x] |
| 19 | `LZ4_createStreamDecode`, `LZ4_setStreamDecode`, `LZ4_freeStreamDecode` | empty/small/64KiB dictionary lifecycle | [x] |
| 20 | `LZ4_decompress_safe_continue`, `LZ4_decompress_fast_continue` | first call, contiguous output, ring-buffer/noncontiguous output | [x] |
| 21 | `LZ4_sizeofStateHC`, `LZ4_sizeofStreamStateHC`, `LZ4F_compressionLevel_max` | scalar HC metadata | [x] |
| 22 | `LZ4_compress_HC`, `LZ4_compressHC`, `LZ4_compressHC_limitedOutput`, `LZ4_compressHC2`, `LZ4_compressHC2_limitedOutput` | levels `<=0,1,2,9,10,11,12,>12`; bounded/limited output | [x] |
| 23 | `LZ4_compress_HC_extStateHC`, `LZ4_compress_HC_extStateHC_fastReset` | aligned state, fresh/reused; levels selecting mid/hash-chain/optimal parsers | [x] |
| 24 | `LZ4_compressHC_withStateHC`, `LZ4_compressHC_limitedOutput_withStateHC`, `LZ4_compressHC2_withStateHC`, `LZ4_compressHC2_limitedOutput_withStateHC` | legacy external state; default/explicit levels and capacities | [x] |
| 25 | `LZ4_compress_HC_destSize` | target `1`, limited, bound; levels `2,9,10,12`; partial/full consumption | [x] |
| 26 | `LZ4_createStreamHC`, `LZ4_initStreamHC`, `LZ4_resetStreamHC`, `LZ4_resetStreamHC_fast`, `LZ4_freeStreamHC` | heap/external lifecycle; parser strategy changes on reuse | [x] |
| 27 | `LZ4_setCompressionLevel`, `LZ4_favorDecompressionSpeed` | levels below/at/above `10`; favor flag `0,1,nonzero` | [x] |
| 28 | `LZ4_loadDictHC`, `LZ4_attach_HC_dictionary` | empty/small/64KiB/large dictionary; loaded and attached contexts | [x] |
| 29 | `LZ4_compress_HC_continue`, `LZ4_compress_HC_continue_destSize` | contiguous/noncontiguous blocks; full/fill output; all parser strategies | [x] |
| 30 | `LZ4_compressHC_continue`, `LZ4_compressHC_limitedOutput_continue`, `LZ4_compressHC2_continue`, `LZ4_compressHC2_limitedOutput_continue` | legacy streaming aliases, default/explicit levels | [x] |
| 31 | `LZ4_saveDictHC`, `LZ4_slideInputBufferHC`, `LZ4_createHC`, `LZ4_resetStreamStateHC`, `LZ4_freeHC` | save `0,<4,<64KiB,64KiB,>64KiB`; legacy lifecycle | [x] |
| 32 | `LZ4HC_searchExtDict` | local/global dictionary match absent/present; attempts `0,1,many`; best length boundaries | [x] |
| 33 | `LZ4_XXH_versionNumber`, `LZ4_XXH32`, `LZ4_XXH64` | lengths `0,1,3,4,7,8,15,16,31,32,many`; aligned/unaligned; varied seeds | [x] |
| 34 | `LZ4_XXH32_createState`, `copyState`, `reset`, `update`, `digest`, `freeState` | zero/one/many updates, split at 16-byte stripe, copied mid-stream | [x] |
| 35 | `LZ4_XXH64_createState`, `copyState`, `reset`, `update`, `digest`, `freeState` | zero/one/many updates, split at 32-byte stripe, copied mid-stream | [x] |
| 36 | XXH canonical conversion functions | min/max/random hashes; hash-to-canonical round trip | [x] |
| 37 | `LZ4F_getVersion`, `LZ4F_isError`, `LZ4F_getErrorCode`, `LZ4F_getErrorName`, `LZ4F_getBlockSize` | success/all error codes; block IDs `0,4,5,6,7` | [x] |
| 38 | `LZ4F_compressFrameBound`, `LZ4F_compressBound` | null/default prefs; block sizes `0,4..7`; autoFlush `0/1`; checksum flags `0/1`; empty/boundary/multiblock | [x] |
| 39 | `LZ4F_compressFrame` | fast levels `<2` including negative acceleration; sizes empty/tiny/block/multiblock | [x] |
| 40 | `LZ4F_compressFrame` | HC levels `2,9,10,12,>12`; favorDecSpeed `0/1`; randomized sizes | [x] |
| 41 | `LZ4F_compressFrame` | linked/independent; content and block checksum cross-product; contentSize/dictID absent/present | [x] |
| 42 | `LZ4F_createCDict[_advanced]`, `LZ4F_freeCDict`, `LZ4F_compressFrame_usingCDict` | dictionary empty/small/64KiB/large; default/custom memory | [x] |
| 43 | `LZ4F_createCompressionContext[_advanced]`, `LZ4F_freeCompressionContext` | default/custom memory and arbitrary version values | [x] |
| 44 | `LZ4F_compressBegin`, `_internal`, `_usingDictOnce`, `_usingDict`, `_usingCDict` | no/raw/compiled dictionary; all frame preference axes | [x] |
| 45 | `LZ4F_compressUpdate`, `LZ4F_uncompressedUpdate` | empty/partial/full/multiple blocks; stableSrc `0/1`; compressed/uncompressed mode switches | [x] |
| 46 | `LZ4F_flush`, `LZ4F_compressEnd` | nothing/some data buffered; checksums/content size absent/present | [x] |
| 47 | `LZ4F_createDecompressionContext[_advanced]`, `LZ4F_resetDecompressionContext`, `LZ4F_freeDecompressionContext` | default/custom memory, fresh/reset/reused lifecycle | [x] |
| 48 | `LZ4F_headerSize`, `LZ4F_getFrameInfo` | ordinary header lengths `7,11,15,19`; skippable header; fragmented input | [x] |
| 49 | `LZ4F_decompress` | linked/independent, compressed/uncompressed blocks, all block sizes/checksum combinations | [x] |
| 50 | `LZ4F_decompress` | fragmented source/destination at every header/block/suffix boundary; stableDst `0/1`; skipChecksums `0/1` | [x] |
| 51 | `LZ4F_decompress_usingDict` | no/small/64KiB dictionary; linked/independent frame and fragmented I/O | [x] |
| 52 | `LZ4F_readOpen`, `LZ4F_read`, `LZ4F_readClose` | empty/tiny/multiblock file; small/large reads; all frame block sizes | [x] |
| 53 | `LZ4F_writeOpen`, `LZ4F_write`, `LZ4F_writeClose` | empty/tiny/multiblock file; small/large writes; default/custom preferences | [x] |

There are no Cargo features in `Cargo.toml`; the full feature-combination set
is therefore the single default/no-feature build.

Completion:

- [x] Rows 1-53 pass grouped randomized differential tests.
- [x] Default/no-feature build passes.
