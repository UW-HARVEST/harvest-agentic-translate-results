# Configuration Surface

The Cargo manifest has no `[features]` table and no default features. There is
one valid Cargo feature combination: the empty set, invoked with
`--no-default-features`.

The CMake target also has one configuration: all five C sources are linked into
`liblz4.so`, xxHash exports are prefixed with `LZ4_`, and both heap-mode macros
are `0`. The rows below are the runtime branch cross-product for that build.
Sizes in randomized rows include empty, one byte, immediately around format
thresholds, and many-block inputs.

| # | entry point(s) | configuration (options set + input shape) | |
|---:|----------------|-------------------------------------------|---|
| 1 | `LZ4_versionNumber`, `LZ4_versionString`, `LZ4F_getVersion`, `LZ4F_compressionLevel_max`, `LZ4_XXH_versionNumber` | scalar metadata, no input | [x] |
| 2 | `LZ4_sizeofState`, `LZ4_sizeofStreamState`, `LZ4_sizeofStateHC`, `LZ4_sizeofStreamStateHC` | state-size metadata, no input | [x] |
| 3 | `LZ4_compressBound`, `LZ4_decoderRingBufferSize` | lengths `0,1,15,16,64K-1,64K,LZ4_MAX_INPUT_SIZE` | [x] |
| 4 | `LZ4_XXH32` | aligned and unaligned inputs; lengths `0,1,3,4,15,16,17,many`; seeds `0,1,u32::MAX` | [x] |
| 5 | `LZ4_XXH64` | aligned and unaligned inputs; lengths `0,1,7,8,31,32,33,many`; seeds `0,1,u64::MAX` | [x] |
| 6 | `LZ4_XXH32_createState`, `LZ4_XXH32_reset`, `LZ4_XXH32_update`, `LZ4_XXH32_digest`, `LZ4_XXH32_freeState` | streaming hash with empty, one, and many updates at the 16-byte stripe boundary | [x] |
| 7 | `LZ4_XXH64_createState`, `LZ4_XXH64_reset`, `LZ4_XXH64_update`, `LZ4_XXH64_digest`, `LZ4_XXH64_freeState` | streaming hash with empty, one, and many updates at the 32-byte stripe boundary | [x] |
| 8 | `LZ4_XXH32_copyState`, `LZ4_XXH64_copyState` | copy before and after partial updates, then diverging suffixes | [x] |
| 9 | `LZ4_XXH32_canonicalFromHash`, `LZ4_XXH32_hashFromCanonical`, `LZ4_XXH64_canonicalFromHash`, `LZ4_XXH64_hashFromCanonical` | zero, patterned, and maximum hash values; canonical big-endian bytes | [x] |
| 10 | `LZ4_compress_default`, `LZ4_compress`, `LZ4_compress_limitedOutput` | empty/small/64K-boundary/many-byte data; compressible, random, and incompressible shapes; exact bound and constrained output | [x] |
| 11 | `LZ4_compress_fast` | same data shapes with acceleration `-1,0,1,2,65537,65538` | [x] |
| 12 | `LZ4_compress_fast_extState`, `LZ4_compress_fast_extState_fastReset`, `LZ4_compress_withState`, `LZ4_compress_limitedOutput_withState` | externally allocated aligned state; first use and initialized fast reset; bounded/unbounded output | [x] |
| 13 | `LZ4_compress_destSize`, `LZ4_compress_destSize_extState` | target sizes `1`, partial budget, exact compressed size, and full bound; acceleration default and nondefault | [x] |
| 14 | `LZ4_decompress_safe`, `LZ4_uncompress_unknownOutputSize` | valid independent blocks from all core compressors; exact and oversized destination | [x] |
| 15 | `LZ4_decompress_safe_partial` | targets `0,1,half,full,larger-than-output`, with destination equal to and larger than target | [x] |
| 16 | `LZ4_decompress_safe_usingDict`, `LZ4_decompress_safe_forceExtDict`, `LZ4_decompress_safe_withPrefix64k` | dictionary sizes `0,small,64K-1,64K,over-64K`; prefix-adjacent and external dictionary layouts | [x] |
| 17 | `LZ4_decompress_safe_partial_usingDict`, `LZ4_decompress_safe_partial_forceExtDict` | the dictionary layouts from row 16 crossed with partial targets | [x] |
| 18 | `LZ4_decompress_fast`, `LZ4_uncompress`, `LZ4_decompress_fast_usingDict`, `LZ4_decompress_fast_withPrefix64k` | trusted valid blocks; empty/nonempty output; no dictionary, prefix, and external dictionary | [x] |
| 19 | `LZ4_createStream`, `LZ4_initStream`, `LZ4_resetStream`, `LZ4_resetStream_fast`, `LZ4_freeStream` | allocated and external state lifecycle; repeated reset and free-null behavior | [x] |
| 20 | `LZ4_loadDict`, `LZ4_loadDictSlow`, `LZ4_loadDict_internal`, `LZ4_attach_dictionary`, `LZ4_saveDict` | dictionary mode fast/slow; sizes `0,small,64K,over-64K`; attached/unattached; save budgets `0,small,64K+` | [x] |
| 21 | `LZ4_compress_fast_continue`, `LZ4_compress_forceExtDict`, `LZ4_compress_continue`, `LZ4_compress_limitedOutput_continue` | one and many blocks; contiguous, disjoint, alternating, and ring-buffer source layouts; bounded output and accelerations | [x] |
| 22 | `LZ4_createStreamDecode`, `LZ4_setStreamDecode`, `LZ4_decompress_safe_continue`, `LZ4_decompress_fast_continue`, `LZ4_freeStreamDecode` | no history, contiguous prefix, external previous block, and double-dictionary/ring layouts | [x] |
| 23 | `LZ4_create`, `LZ4_resetStreamState`, `LZ4_slideInputBuffer` | deprecated external-state lifecycle with null and non-null input buffer markers | [x] |
| 24 | `LZ4_compress_HC`, `LZ4_compressHC`, `LZ4_compressHC_limitedOutput`, `LZ4_compressHC2`, `LZ4_compressHC2_limitedOutput` | empty/small/large compressible and random inputs; levels `-1,0,1,2,9,10,11,12,13`; exact and constrained destinations | [x] |
| 25 | `LZ4_compress_HC_extStateHC`, `LZ4_compress_HC_extStateHC_fastReset`, `LZ4_compressHC_withStateHC`, `LZ4_compressHC_limitedOutput_withStateHC`, `LZ4_compressHC2_withStateHC`, `LZ4_compressHC2_limitedOutput_withStateHC` | external HC state, first/reset use, fast/optimal parser levels, bounded/unbounded output | [x] |
| 26 | `LZ4_compress_HC_destSize` | target sizes `1`, partial, exact, and bound crossed with levels `2,9,10,12` | [x] |
| 27 | `LZ4_createStreamHC`, `LZ4_initStreamHC`, `LZ4_resetStreamHC`, `LZ4_resetStreamHC_fast`, `LZ4_freeStreamHC` | allocated and external state lifecycle; clean and dirty fast reset; repeated sessions | [x] |
| 28 | `LZ4_setCompressionLevel`, `LZ4_favorDecompressionSpeed` | levels below/default/optimal/max/above max crossed with favor flag `0,1,nonzero` | [x] |
| 29 | `LZ4_loadDictHC`, `LZ4_attach_HC_dictionary`, `LZ4_saveDictHC` | dictionary sizes `0,small,64K,over-64K`; attached/unattached; fast and optimal parser levels | [x] |
| 30 | `LZ4_compress_HC_continue`, `LZ4_compress_HC_continue_destSize`, `LZ4_compressHC_continue`, `LZ4_compressHC_limitedOutput_continue` | one/many blocks, contiguous/disjoint/ring input, normal and destination-size budgets | [x] |
| 31 | `LZ4_createHC`, `LZ4_freeHC`, `LZ4_resetStreamStateHC`, `LZ4_slideInputBufferHC`, `LZ4_compressHC2_continue`, `LZ4_compressHC2_limitedOutput_continue` | deprecated HC state lifecycle and continuation at default/optimal/max levels | [x] |
| 32 | `LZ4HC_searchExtDict` | low-level HC dictionary search with no match, short match, long match, and attempts `1,many` | [x] |
| 33 | `LZ4F_getBlockSize` | block size ids `0,4,5,6,7` | [x] |
| 34 | `LZ4F_compressFrameBound`, `LZ4F_compressBound` | empty/small/exact-block/multiblock sizes; null/default preferences and each block size | [x] |
| 35 | `LZ4F_compressFrame` | default preferences; empty, one, and many blocks; compressible and random bytes | [x] |
| 36 | `LZ4F_compressFrame` | block size ids `0,4,5,6,7` crossed with linked and independent block modes | [x] |
| 37 | `LZ4F_compressFrame` | content checksum off/on crossed with block checksum off/on | [x] |
| 38 | `LZ4F_compressFrame` | content size unknown/exact crossed with dict id absent/present | [x] |
| 39 | `LZ4F_compressFrame` | compression level negative/fast/default/HC/above-max crossed with `autoFlush=0/1` and `favorDecSpeed=0/1` | [x] |
| 40 | `LZ4F_createCompressionContext`, `LZ4F_createCompressionContext_advanced`, `LZ4F_freeCompressionContext` | default allocator and custom alloc/calloc/free callbacks; version `100` and arbitrary versions; free null/idle/active | [x] |
| 41 | `LZ4F_compressBegin`, `LZ4F_compressUpdate`, `LZ4F_flush`, `LZ4F_compressEnd` | complete streaming lifecycle; zero/one/many updates; chunk sizes around selected block size; `stableSrc=0/1`; flush empty/buffered | [x] |
| 42 | `LZ4F_uncompressedUpdate` | streaming lifecycle alternating compressed and uncompressed blocks, empty/small/block-sized chunks | [x] |
| 43 | `LZ4F_createCDict`, `LZ4F_createCDict_advanced`, `LZ4F_freeCDict` | dictionary sizes `0,small,64K,over-64K`; default/custom memory; free null/non-null | [x] |
| 44 | `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingDictOnce`, `LZ4F_compressBegin_usingCDict`, `LZ4F_compressBegin_internal`, `LZ4F_compressFrame_usingCDict` | raw/digested/null dictionary; fast/HC compression; linked/independent blocks; one and many blocks | [x] |
| 45 | `LZ4F_createDecompressionContext`, `LZ4F_createDecompressionContext_advanced`, `LZ4F_resetDecompressionContext`, `LZ4F_freeDecompressionContext` | default/custom memory lifecycle; version `100` and arbitrary versions; reset idle/partial/error/completed; free null/partial/completed | [x] |
| 46 | `LZ4F_headerSize`, `LZ4F_getFrameInfo` | regular headers of `7,11,15,19` bytes and 8-byte skippable header; exact and oversized input; info before/after decode starts | [x] |
| 47 | `LZ4F_decompress` | every valid frame option from rows 35-39; whole frame and chunks `1,header,block,many`; destination `0,1,small,full`; `stableDst=0/1`, `skipChecksums=0/1` | [x] |
| 48 | `LZ4F_decompress_usingDict` | raw dictionary sizes `0,small,64K,over-64K`; whole/chunked source and destination; linked/independent blocks | [x] |
| 49 | `LZ4F_isError`, `LZ4F_getErrorCode`, `LZ4F_getErrorName` | success values, every listed frame error code, and unrecognized non-error values | [x] |
| 50 | `LZ4F_writeOpen`, `LZ4F_write`, `LZ4F_writeClose`, `LZ4F_readOpen`, `LZ4F_read`, `LZ4F_readClose` | temporary-file round trip; block sizes `0,4,5,6,7`; empty/one/many writes and reads smaller/equal/larger than content | [x] |

## Export Coverage

Every name in `SYMBOLS.md` occurs in at least one entry-point cell above. The
table includes stable, static-linking-only exports that CMake makes visible,
deprecated aliases, and the two low-level exports `LZ4HC_searchExtDict` and
`LZ4F_compressBegin_internal`.
