# LZ4 Configuration Surface

Mechanically derived from the branches actually present in
`c_src/src/{lz4,lz4hc,lz4frame,lz4file,xxhash}.c` and the public API declared in `c_src/include/`
(including the deprecated/internal entry points actually exported by `build/liblz4.so`).

Constants the C code branches on:
`MINMATCH=4`, `MFLIMIT=12`, `LASTLITERALS=5`, `LZ4_minLength=13`, `RUN_MASK=ML_MASK=15`,
`WILDCOPYLENGTH=8`, `MATCH_SAFEGUARD_DISTANCE=12`, `FASTLOOP_SAFE_DISTANCE=64`,
`LZ4_64Klimit=65547`, `LZ4_DISTANCE_MAX=65535`, `LZ4_MAX_INPUT_SIZE=0x7E000000`,
`LZ4_ACCELERATION_DEFAULT=1`, `LZ4_ACCELERATION_MAX=65537`, `HASH_UNIT=8`,
table-reset / dictCtx-copy threshold `4 KB`, dictionary window `64 KB`, renorm at `0x80000000`,
`LZ4HC_CLEVEL_MIN=2 / DEFAULT=9 / OPT_MIN=10 / MAX=12`, `LZ4HC_HASHSIZE=4`, `LZ4MID_HASHSIZE=8`,
`OPTIMAL_ML=18`, `LZ4_OPT_NUM=4096`, `LZ4F_HEADER_SIZE_MIN=7 / MAX=19`,
`LZ4F_BLOCK_HEADER_SIZE=4`, `LZ4F_BLOCK_CHECKSUM_SIZE=4`, frame block sizes `64KB/256KB/1MB/4MB`,
xxh32 stripe `16`, xxh64 stripe `32`.

Build is configured with `LZ4_HEAPMODE=0`, `LZ4F_HEAPMODE=0`, `LZ4HC_HEAPMODE=1` (default),
`XXH_NAMESPACE=LZ4_`, so all xxHash entry points are exported as `LZ4_XXH*`.

## lz4 block — one-shot compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | LZ4_compress_default | srcSize=0 (src may be NULL), dstCapacity>=1 → single 0x00 "empty block"; dstCapacity=0 → returns 0 | [x] |
| 2 | LZ4_compress_default | srcSize=1; srcSize=12 (== MFLIMIT, < LZ4_minLength → all-literals path); srcSize=13 (main loop entered) | [x] |
| 3 | LZ4_compress_default | highly compressible 1 KB (< 4 KB), dstCapacity = LZ4_compressBound → byU16 + notLimited | [x] |
| 4 | LZ4_compress_default | incompressible random 1 KB → one long literal run, lastRun>=RUN_MASK with 255-byte chain | [x] |
| 5 | LZ4_compress_default | srcSize=65535 (64KB-1) and srcSize=65546 (LZ4_64Klimit-1) → still byU16 | [x] |
| 6 | LZ4_compress_default | srcSize=65547 (== LZ4_64Klimit) → switches to byU32 table type | [x] |
| 7 | LZ4_compress_default | compressible 256 KB / 1 MB / 4 MB (frame block-size boundaries) → byU32 | [x] |
| 8 | LZ4_compress_default | 100 KB of a single repeated byte → offset=1, matchCode>=ML_MASK with 4*255 length chain | [x] |
| 9 | LZ4_compress_default | dstCapacity = LZ4_compressBound(srcSize)-1 → limitedOutput, succeeds; dstCapacity far too small → 0 | [x] |
| 10 | LZ4_compress_default | srcSize negative, or srcSize > LZ4_MAX_INPUT_SIZE → returns 0 | [x] |
| 11 | LZ4_compress_fast | acceleration = 0 and negative → LZ4_ACCELERATION_DEFAULT; = 2 / 8 / 64; = 65537 (MAX) and 1000000 → clamped | [x] |
| 12 | LZ4_sizeofState, LZ4_compress_fast_extState | src<64Klimit + dst>=bound (byU16/notLimited); src>=64Klimit + dst<bound (byU32/limitedOutput) | [x] |
| 13 | LZ4_initStream | valid buffer+size; size < sizeof(LZ4_stream_t) → NULL; misaligned buffer → NULL; NULL buffer → NULL | [x] |
| 14 | LZ4_compress_fast_extState_fastReset | state freshly LZ4_initStream'd (currentOffset==0) → noDictIssue; reused state (currentOffset!=0) → dictSmall; srcSize<64Klimit | [x] |
| 15 | LZ4_compress_fast_extState_fastReset | repeated calls with srcSize<4 KB (table re-used, currentOffset += 64 KB each call) vs srcSize>=4 KB (LZ4_prepareTable full MEM_INIT reset) | [x] |
| 16 | LZ4_compress_fast_extState_fastReset | reuse until currentOffset+srcSize >= 0xFFFF (byU16) or currentOffset > 1 GB (byU32) → forced table reset | [x] |
| 17 | LZ4_compress_destSize | targetDstSize >= LZ4_compressBound(*srcSizePtr) → whole input consumed (delegates to extState) | [x] |
| 18 | LZ4_compress_destSize | srcSize<64Klimit and srcSize>=64Klimit with targetDstSize ≈ half of bound → fillOutput, *srcSizePtr reduced | [x] |
| 19 | LZ4_compress_destSize | targetDstSize=1 (minimum); targetDstSize=0 → 0; incompressible input filling dst exactly (lastRun truncation branch) | [x] |
| 20 | LZ4_compress_destSize, LZ4_compress_destSize_extState (acceleration 1 and 10) | repetitive input forcing match-length truncation + hash-table clearing (ip <= filledIp) | [x] |
| 21 | LZ4_compressBound, LZ4_versionNumber, LZ4_versionString | inputSize 0, 1, LZ4_MAX_INPUT_SIZE, LZ4_MAX_INPUT_SIZE+1 (→0), negative (→0) | [x] |

## lz4 block — decompression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 22 | LZ4_decompress_safe | dstCapacity == exact decompressed size; dstCapacity larger; dstCapacity smaller → negative | [x] |
| 23 | LZ4_decompress_safe | compressedSize=0 → -1; dstCapacity=0 with src="\0" and srcSize=1 → 0; dstCapacity=0 with any other src → -1 | [x] |
| 24 | LZ4_decompress_safe | output < 64 bytes (FASTLOOP_SAFE_DISTANCE) → safe loop only; output ≫ 64 → fast loop then safe tail | [x] |
| 25 | LZ4_decompress_safe | block using offset = 1, 2 and 4 → LZ4_memcpy_using_offset special cases | [x] |
| 26 | LZ4_decompress_safe | offsets 3,5,6,7 (<8, inc32/dec64 tables), 8..15 (<16), >=16 (wildCopy32) | [x] |
| 27 | LZ4_decompress_safe | literal-length token == 15 and match-length token == 15, each with multi-255 extension bytes | [x] |
| 28 | LZ4_decompress_safe | literal length <= 14 with ip<shortiend && op<=shortoend → two-stage 16/18-byte shortcut | [x] |
| 29 | LZ4_decompress_safe | malformed input: offset pointing before the buffer, truncated length bytes, last-5-literals / last-match-12-bytes rules violated → negative | [x] |
| 30 | LZ4_decompress_safe | in-place decompression, src laid at the end of the dst buffer (LZ4_memmove literal path) | [x] |
| 31 | LZ4_decompress_safe_partial | targetOutputSize=0; targetOutputSize > decompressed size with exact srcSize; dstCapacity < targetOutputSize (MIN applied) | [x] |
| 32 | LZ4_decompress_safe_partial | stop inside a literal run; stop inside a match (cpy > oend-MATCH_SAFEGUARD_DISTANCE overlap copy) | [x] |
| 33 | LZ4_decompress_fast | exact originalSize on a normal block; originalSize 0 / 1 / <12; block with long literal and match lengths (read_long_length_no_check) | [x] |
| 34 | LZ4_uncompress, LZ4_uncompress_unknownOutputSize | legacy wrappers over decompress_fast / decompress_safe | [x] |

## lz4 streaming (chained-block) compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 35 | LZ4_createStream, LZ4_freeStream, LZ4_resetStream (deprecated) | create; free; free(NULL); zeroed stream then LZ4_compress_fast_continue | [x] |
| 36 | LZ4_resetStream_fast + LZ4_compress_fast_continue | reuse of a stream that already compressed data (prepareTable byU32, +64 KB offset gap) | [x] |
| 37 | LZ4_loadDict | dictSize=0 → 0 (reset only); dictSize=3 (< HASH_UNIT=8) → 0 but currentOffset still advanced 64 KB | [x] |
| 38 | LZ4_loadDict | dictSize=65536 exactly (64 KB boundary); dictSize=100 KB (>64 KB) → only trailing 64 KB referenced | [x] |
| 39 | LZ4_loadDictSlow | dictSize 8 B / 32 KB / 64 KB / >64 KB → extra non-overwriting fill pass (p++ instead of p+=3) | [x] |
| 40 | LZ4_compress_fast_continue | contiguous prefix (src == dictEnd), dictSize<64KB && dictSize<currentOffset → withPrefix64k + dictSmall | [x] |
| 41 | LZ4_compress_fast_continue | contiguous prefix with dictSize>=64 KB → withPrefix64k + noDictIssue; and very first block (dictSize=0) | [x] |
| 42 | LZ4_compress_fast_continue | separate src buffer after LZ4_loadDict → usingExtDict (+ dictSmall while dict < 64 KB) | [x] |
| 43 | LZ4_compress_fast_continue | alternating double buffer, each block < 64 KB, buffers separated by at least one byte | [x] |
| 44 | LZ4_compress_fast_continue | ring buffer < 64 KB where src overlaps the dictionary → dictSize recomputed (clamped to 64 KB, dropped when < 4) | [x] |
| 45 | LZ4_compress_fast_continue | previous dictSize < 4 with non-prefix src → dictionary discarded, prefix mode forced | [x] |
| 46 | LZ4_attach_dictionary + LZ4_compress_fast_continue | srcSize <= 4 KB → usingDictCtx (dictCtx hash table consulted on miss) | [x] |
| 47 | LZ4_attach_dictionary + LZ4_compress_fast_continue | srcSize > 4 KB → dictCtx memcpy'd into working ctx → usingExtDict | [x] |
| 48 | LZ4_attach_dictionary | dictionaryStream=NULL → unset; dictCtx with dictSize==0 → not attached; workingStream currentOffset==0 → bumped to 64 KB | [x] |
| 49 | LZ4_compress_fast_continue | cumulative input so that currentOffset+srcSize > 0x80000000 → LZ4_renormDictT hash rescale | [x] |
| 50 | LZ4_compress_fast_continue | dstCapacity < LZ4_compressBound(srcSize) → 0, stream left invalid; acceleration 0 / 1 / 64 / >MAX on a chained block | [x] |
| 51 | LZ4_saveDict, LZ4_compress_forceExtDict | maxDictSize=64 KB, < current dictSize, =0, safeBuffer=NULL; forceExtDict with dictSmall vs noDictIssue | [x] |
| 52 | LZ4_compress_continue, LZ4_compress_limitedOutput_continue, LZ4_compress, LZ4_compress_limitedOutput, LZ4_compress_withState, LZ4_compress_limitedOutput_withState, LZ4_create, LZ4_slideInputBuffer, LZ4_resetStreamState, LZ4_sizeofStreamState | legacy/deprecated wrappers (acceleration fixed to 1, bound-sized vs limited dst, degraded streaming) | [x] |

## lz4 streaming + dictionary decompression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 53 | LZ4_createStreamDecode, LZ4_freeStreamDecode, LZ4_setStreamDecode | create/free/free(NULL); dictionary NULL with size 0 (reset), small dict, exactly 64 KB dict | [x] |
| 54 | LZ4_decompress_safe_continue | very first block, prefixSize==0 → plain safe decode | [x] |
| 55 | LZ4_decompress_safe_continue | contiguous dst (prefixEnd==dest), prefixSize < 64KB-1, extDictSize==0 → withSmallPrefix | [x] |
| 56 | LZ4_decompress_safe_continue | contiguous dst, prefixSize >= 64KB-1 → withPrefix64k | [x] |
| 57 | LZ4_decompress_safe_continue | contiguous dst with an externalDict already recorded → doubleDict | [x] |
| 58 | LZ4_decompress_safe_continue | dst switches buffer or ring wraps → prefix becomes extDict (forceExtDict) | [x] |
| 59 | LZ4_decompress_safe_continue | ring buffer sized exactly LZ4_decoderRingBufferSize(maxBlockSize) = 65536+14+maxBlockSize | [x] |
| 60 | LZ4_decompress_safe_continue | synchronized small ring buffer (< 64 KB) with exact per-block decompressed sizes | [x] |
| 61 | LZ4_decompress_safe_continue | mid-stream failure (dstCapacity too small / corrupt block) → negative, prefix state untouched | [x] |
| 62 | LZ4_decompress_fast_continue | first block; prefix continuation; prefix→extDict switch (all three branches) | [x] |
| 63 | LZ4_decoderRingBufferSize | maxBlockSize 0, 16, 64 KB, 4 MB; negative → 0; > LZ4_MAX_INPUT_SIZE → 0 | [x] |
| 64 | LZ4_decompress_safe_usingDict, LZ4_decompress_safe_partial_usingDict | dictSize=0; dictStart+dictSize==dst with dictSize>=64KB-1; same with small dictSize; separate ext dict with a match straddling the dict/block boundary | [x] |
| 65 | LZ4_decompress_fast_usingDict, LZ4_decompress_safe_forceExtDict, LZ4_decompress_safe_partial_forceExtDict, LZ4_decompress_safe_withPrefix64k, LZ4_decompress_fast_withPrefix64k | dictSize < 64 KB (checkOffset enabled) vs >= 64 KB (disabled); legacy 64 KB-prefix decoders | [x] |

## lz4hc block

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 66 | LZ4_compress_HC | compressionLevel = 0 and negative → coerced to LZ4HC_CLEVEL_DEFAULT (9) | [x] |
| 67 | LZ4_compress_HC | compressionLevel = 1 and 2 (LZ4HC_CLEVEL_MIN) → lz4mid strategy (hash4 + hash8 tables, 2 searches) | [x] |
| 68 | LZ4_compress_HC | compressionLevel = 3 (hashChain, 4 searches) and 6 (32 searches) | [x] |
| 69 | LZ4_compress_HC | compressionLevel = 8 → hashChain 128 searches, patternAnalysis OFF (threshold is > 128) | [x] |
| 70 | LZ4_compress_HC | compressionLevel = 9 → hashChain 256 searches, patternAnalysis ON | [x] |
| 71 | LZ4_compress_HC | compressionLevel = 10 (LZ4HC_CLEVEL_OPT_MIN) → optimal parser, 96 searches, sufficient_len 64, fullUpdate off | [x] |
| 72 | LZ4_compress_HC | compressionLevel = 11 → optimal parser, 512 searches, targetLength 128 | [x] |
| 73 | LZ4_compress_HC | compressionLevel = 12 (LZ4HC_CLEVEL_MAX) → optimal parser, 16384 searches, fullUpdate / ultra mode | [x] |
| 74 | LZ4_compress_HC | compressionLevel = 13 and 100 → clamped to 12 | [x] |
| 75 | LZ4_compress_HC | srcSize = 0, 1, and 12 (< LZ4_minLength → last-literals only) at levels 2, 9 and 12 | [x] |
| 76 | LZ4_compress_HC | srcSize negative or > LZ4_MAX_INPUT_SIZE → 0 | [x] |
| 77 | LZ4_compress_HC | dstCapacity >= LZ4_compressBound (notLimited) vs < bound (limitedOutput) vs far too small (→0), at levels 2 / 9 / 12 | [x] |
| 78 | LZ4_compress_HC | long repeated patterns of period 1, 2 and 4 bytes at levels 9/12 → LZ4HC_countPattern / rotatePattern / reverseCountPattern / protectDictEnd | [x] |
| 79 | LZ4_compress_HC | incompressible random input at levels 2, 9 and 12 (lz4mid skip `ip += 1 + ((ip-anchor)>>9)`) | [x] |
| 80 | LZ4_compress_HC | inputs > 64 KB and > 4 MB at level 9 (chainTable/DELTANEXTU16 wrap, LZ4_DISTANCE_MAX clamping); single match longer than LZ4_OPT_NUM=4096 at level 12 | [x] |
| 81 | LZ4_sizeofStateHC, LZ4_compress_HC_extStateHC | externally allocated state at levels 2 / 9 / 12; undersized state → 0; misaligned state → 0 | [x] |
| 82 | LZ4_compress_HC_extStateHC_fastReset | already-initialized state reused with a different level between calls (mid ↔ hashChain ↔ optimal strategy switch) | [x] |
| 83 | LZ4_compress_HC_destSize | fillOutput at level 2 (`_lz4mid_dest_overflow`), level 9 (`_dest_overflow`) and level 12 (optimal overflow); targetDstSize = 1 and 0 | [x] |
| 84 | LZ4_favorDecompressionSpeed, LZ4HC_searchExtDict | favor=1 at level 10/12 (skips offsets < 8, folds matchLen 19..36 → 18) vs favor=0; direct LZ4HC_searchExtDict call against an HC dictCtx (nbAttempts=2, match near dict end) | [x] |

## lz4hc streaming + legacy

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 85 | LZ4_createStreamHC, LZ4_freeStreamHC, LZ4_initStreamHC | create (level defaults to 9), free, free(NULL); initStreamHC valid / size too small → NULL / misaligned → NULL / NULL → NULL | [x] |
| 86 | LZ4_resetStreamHC | compressionLevel 0, 9, 12, negative and > 12 (clamping inside LZ4_setCompressionLevel) | [x] |
| 87 | LZ4_resetStreamHC_fast | clean stream (dirty==0) → cheap reset; stream after a failed compression (dirty==1) → full LZ4_initStreamHC | [x] |
| 88 | LZ4_setCompressionLevel | level changed between blocks of one stream, crossing lz4mid ↔ hashChain ↔ optimal boundaries | [x] |
| 89 | LZ4_loadDictHC | dictSize 0; 3 (< LZ4HC_HASHSIZE=4, no Insert); 64 KB exactly; > 64 KB (trailing 64 KB kept) | [x] |
| 90 | LZ4_loadDictHC | level set to 2 before loading → LZ4MID_fillHTable (incl. the 32 KB second-pass window); level 9/12 → LZ4HC_Insert(end-3) | [x] |
| 91 | LZ4_compress_HC_continue | contiguous blocks (src == ctx->end) at levels 2, 9 and 12 | [x] |
| 92 | LZ4_compress_HC_continue | non-contiguous src → LZ4HC_setExternalDict (prefix becomes extDict, dictCtx cleared) | [x] |
| 93 | LZ4_compress_HC_continue | ring buffer where src overlaps the dict → lowLimit/dictStart advance; dict invalidated when the remainder < LZ4HC_HASHSIZE | [x] |
| 94 | LZ4_compress_HC_continue | after LZ4_loadDictHC, blocks matching into extDict at level 9 (chain) and level 2 (mid extDict candidates) | [x] |
| 95 | LZ4_compress_HC_continue | dstCapacity < LZ4_compressBound (limitedOutput) vs >= (notLimited); failure sets the dirty flag | [x] |
| 96 | LZ4_compress_HC_continue | cumulative (end-prefixStart)+dictLimit > 2 GB → automatic LZ4_loadDictHC re-anchor | [x] |
| 97 | LZ4_attach_HC_dictionary + LZ4_compress_HC_continue | position==0 && srcSize>4 KB && isStateCompatible → dictCtx memcpy + setExternalDict; otherwise usingDictCtxHc (incl. dict at level 2 vs stream at level 9) | [x] |
| 98 | LZ4_attach_HC_dictionary | >= 64 KB already compressed (position>=64 KB) → dictCtx dropped; dictionary_stream=NULL → unset | [x] |
| 99 | LZ4_compress_HC_continue_destSize | fillOutput on a chained stream, partial input consumption, at levels 2 / 9 / 12 | [x] |
| 100 | LZ4_saveDictHC | dictSize = 64 KB; dictSize < 4 → 0; dictSize > prefixSize (clamped); safeBuffer = NULL | [x] |
| 101 | LZ4_compressHC, LZ4_compressHC_limitedOutput, LZ4_compressHC2, LZ4_compressHC2_limitedOutput | deprecated one-shots, cLevel 0 / 1 / 9 / 12, bound-sized vs limited dst | [x] |
| 102 | LZ4_compressHC_withStateHC, LZ4_compressHC_limitedOutput_withStateHC, LZ4_compressHC2_withStateHC, LZ4_compressHC2_limitedOutput_withStateHC | deprecated external-state variants, cLevel 0 / 9 / 12 | [x] |
| 103 | LZ4_compressHC_continue, LZ4_compressHC_limitedOutput_continue, LZ4_compressHC2_continue (dstCapacity fixed 0 + notLimited), LZ4_compressHC2_limitedOutput_continue, LZ4_createHC, LZ4_freeHC, LZ4_slideInputBufferHC, LZ4_resetStreamStateHC, LZ4_sizeofStreamStateHC | deprecated chained compression + deprecated HC state API | [x] |

## lz4frame

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 104 | LZ4F_compressFrame | preferencesPtr = NULL → all defaults (blockSizeID max64KB, blockLinked, no checksums, level 0); internally autoFlush=1 and stableSrc=1 | [x] |
| 105 | LZ4F_compressFrame | blockSizeID = LZ4F_default / max64KB / max256KB / max1MB / max4MB with srcSize > that block size (several blocks per frame) | [x] |
| 106 | LZ4F_compressFrame | blockSizeID = max4MB with srcSize = 1 KB → LZ4F_optimalBSID downgrades the stored BD byte to max64KB | [x] |
| 107 | LZ4F_compressFrame | blockMode = LZ4F_blockLinked vs LZ4F_blockIndependent; srcSize <= blockSize → blockMode forced to blockIndependent | [x] |
| 108 | LZ4F_compressFrame | contentChecksumFlag no/enabled × blockChecksumFlag no/enabled (frame footer XXH32 and per-block XXH32) | [x] |
| 109 | LZ4F_compressFrame | contentSize = 0 (unknown) vs non-zero (auto-corrected to srcSize, adds the 8-byte header field) | [x] |
| 110 | LZ4F_compressFrame | dictID = 0 vs non-zero; all resulting header sizes 7 / 11 / 15 / 19 in combination with contentSize | [x] |
| 111 | LZ4F_compressFrame | compressionLevel 0 and 1 (< LZ4HC_CLEVEL_MIN → LZ4_stream_t context, acceleration 1) | [x] |
| 112 | LZ4F_compressFrame | compressionLevel negative (-1, -10, -1000) → LZ4 acceleration = -level + 1 | [x] |
| 113 | LZ4F_compressFrame | compressionLevel 2 and 3..9 → LZ4_streamHC_t context, HC block functions | [x] |
| 114 | LZ4F_compressFrame | compressionLevel 10, 12, and 13/100 (clamped by LZ4_setCompressionLevel) | [x] |
| 115 | LZ4F_compressFrame | favorDecSpeed = 1 with level >= 10 (applied), with level 0/1 (no HC ctx, not applied), favorDecSpeed = 0 | [x] |
| 116 | LZ4F_compressFrame | srcSize = 0 (header + endMark only), 1, blockSize-1, blockSize exactly, blockSize+1, N*blockSize | [x] |
| 117 | LZ4F_compressFrame | incompressible random data → LZ4F_makeBlock emits a stored block (LZ4F_BLOCKUNCOMPRESSED_FLAG, cSize>=srcSize) | [x] |
| 118 | LZ4F_compressFrame | dstCapacity < LZ4F_compressFrameBound(srcSize, prefs) → ERROR_dstMaxSize_tooSmall | [x] |
| 119 | LZ4F_compressFrameBound, LZ4F_compressBound | prefs NULL (worst case: both checksums on), autoFlush 0 vs 1, srcSize 0 (flush/compressEnd bound), each blockSizeID | [x] |
| 120 | LZ4F_createCDict, LZ4F_createCDict_advanced, LZ4F_freeCDict | dictSize 0 / < 64 KB / > 64 KB (trailing 64 KB copied); freeCDict(NULL); custom LZ4F_CustomMem (alloc only, calloc+alloc, free) | [x] |
| 121 | LZ4F_compressFrame_usingCDict | cdict non-NULL at level 0/1 (fastCtx attach) and level >= 2 (HCCtx attach); cdict = NULL; blockIndependent multi-block (dict per block) vs blockLinked | [x] |
| 122 | LZ4F_createCompressionContext, LZ4F_createCompressionContext_advanced, LZ4F_freeCompressionContext | version = LZ4F_VERSION; free(NULL); custom allocator | [x] |
| 123 | LZ4F_compressBegin | dstCapacity < LZ4F_HEADER_SIZE_MAX (19) → dstMaxSize_tooSmall; dstCapacity == 19; prefsPtr NULL vs fully populated | [x] |
| 124 | LZ4F_compressBegin + LZ4F_compressUpdate + LZ4F_compressEnd | autoFlush=1, stableSrc=0, blockLinked, several updates of varying size | [x] |
| 125 | LZ4F_compressBegin/compressUpdate/compressEnd | autoFlush=0 → residual input buffered in tmpIn; several sub-blockSize updates, then one crossing blockSize | [x] |
| 126 | LZ4F_compressUpdate | compressOptions stableSrc=1 with blockLinked → tmpIn reset to tmpBuff (no copy); stableSrc=0 → LZ4F_localSaveDict (LZ4_saveDict / LZ4_saveDictHC) each call | [x] |
| 127 | LZ4F_compressUpdate | autoFlush=0 + blockLinked where tmpIn+blockSize exceeds tmpBuff+maxBufferSize → forced localSaveDict to keep 64 KB | [x] |
| 128 | LZ4F_compressUpdate | srcSize=0; dstCapacity exactly LZ4F_compressBound(srcSize, prefs); one byte less → dstMaxSize_tooSmall | [x] |
| 129 | LZ4F_compressUpdate, LZ4F_flush, LZ4F_compressEnd | called before LZ4F_compressBegin or after a completed LZ4F_compressEnd (cStage != 1) → compressionState_uninitialized | [x] |
| 130 | LZ4F_flush | nothing buffered → returns 0; partial block buffered (blockLinked and blockIndependent); dstCapacity < tmpInSize+BHSize+BFSize → dstMaxSize_tooSmall | [x] |
| 131 | LZ4F_uncompressedUpdate | blockIndependent frame; srcSize < / == / > blockSize; dstCapacity < srcSize → dstMaxSize_tooSmall | [x] |
| 132 | LZ4F_compressUpdate interleaved with LZ4F_uncompressedUpdate | blockCompressMode change (COMPRESSED ↔ UNCOMPRESSED) triggers an implicit LZ4F_flush of buffered data | [x] |
| 133 | LZ4F_compressEnd | contentSize declared but != total bytes fed → frameSize_wrong; dstCapacity < 4, and < 8 with contentChecksum → dstMaxSize_tooSmall | [x] |
| 134 | LZ4F_compressBegin (cctx reuse) | second frame whose compressionLevel crosses LZ4HC_CLEVEL_MIN (lz4CtxType switch / realloc) and whose blockSizeID is larger (tmpBuff realloc) | [x] |
| 135 | LZ4F_compressBegin_usingDict, LZ4F_compressBegin_usingDictOnce | dictSize < 64 KB and > 64 KB; level < 2 (LZ4_loadDict) vs >= 2 (LZ4_loadDictHC); blockIndependent (dict effective only for the first block) | [x] |
| 136 | LZ4F_compressBegin_usingCDict | prefsPtr NULL (no dictID in header) vs prefs carrying dictID; blockLinked (init once) vs blockIndependent (per-block init) | [x] |
| 137 | LZ4F_getBlockSize, LZ4F_getVersion, LZ4F_compressionLevel_max | blockSizeID 0 (→ default 64 KB), 4, 5, 6, 7; 1/2/3/8 → maxBlockSize_invalid | [x] |
| 138 | LZ4F_isError, LZ4F_getErrorName, LZ4F_getErrorCode | a success value, each LZ4F_ERROR_* code, and a value outside the error range | [x] |
| 139 | LZ4F_headerSize | srcSize < 5 → frameHeader_incomplete; skippable magic → 8; plain frame → 7; +contentSize → 15; +dictID → 11; both → 19; bad magic → frameType_unknown; src=NULL → srcPtr_wrong | [x] |
| 140 | LZ4F_getFrameInfo | fresh dctx with >= headerSize bytes (consumes header, returns BHSize hint); fewer bytes → frameHeader_incomplete with *srcSizePtr=0; dctx stopped mid-header → frameDecoding_alreadyStarted; after decoding started → cached frameInfo + next-size hint | [x] |
| 141 | LZ4F_createDecompressionContext, LZ4F_createDecompressionContext_advanced, LZ4F_freeDecompressionContext, LZ4F_resetDecompressionContext | free returns current dStage; reset after an error then reuse for a new frame | [x] |
| 142 | LZ4F_decompress | whole frame in one call with *dstSizePtr >= maxBlockSize → decode straight into dstBuffer | [x] |
| 143 | LZ4F_decompress | source fed one byte at a time → dstage_storeFrameHeader / storeBlockHeader / storeCBlock / storeSuffix / storeSFrameSize all exercised | [x] |
| 144 | LZ4F_decompress | dst capacity < maxBlockSize → decode into tmpOut then dstage_flushOut, with the flush split over several calls | [x] |
| 145 | LZ4F_decompress | blockLinked frame → all LZ4F_updateDict branches (prefix continuation, dst history >= 64 KB, withinTmp continue, withinTmp copy-in-front, dict==tmpOutBuffer top-up, join dict+dst) | [x] |
| 146 | LZ4F_decompress | blockIndependent frame (no dictionary management) at each blockSizeID | [x] |
| 147 | LZ4F_decompress | decompressOptions stableDst = 1 vs 0 on a blockLinked frame (end-of-call history preservation skipped vs performed) | [x] |
| 148 | LZ4F_decompress | decompressOptions skipChecksums = 1 on a frame carrying content and block checksums (sticky for the rest of the frame) | [x] |
| 149 | LZ4F_decompress | corrupted content checksum → contentChecksum_invalid; corrupted block checksum on a compressed block and on a stored block → blockChecksum_invalid; corrupted header checksum → headerChecksum_invalid | [x] |
| 150 | LZ4F_decompress | bad magic → frameType_unknown; FLG version != 1 → headerVersion_wrong; FLG bit1 / BD bit7 / BD low nibble set → reservedFlag_set; blockSizeID < 4 → maxBlockSize_invalid | [x] |
| 151 | LZ4F_decompress | stored (uncompressed) blocks → dstage_copyDirect, including a copy split across calls and the following block checksum | [x] |
| 152 | LZ4F_decompress | declared block size > frame maxBlockSize → maxBlockSize_invalid; blockHeader == 0 → clean end of frame; truncated compressed block → decompressionFailed | [x] |
| 153 | LZ4F_decompress | contentSize present in the header but decoded total differs → frameSize_wrong at dstage_getSuffix | [x] |
| 154 | LZ4F_decompress | skippable frame (magic LZ4F_MAGIC_SKIPPABLE_START..+0xF): content size 0, small, larger than the supplied input (split over calls), and 8-byte header split across calls | [x] |
| 155 | LZ4F_decompress | two frames concatenated in one dctx (auto reset at end of frame); frame followed by a skippable frame; srcSize=0 → returns minFHSize hint; dstBuffer NULL with *dstSizePtr=0 (hint-only call) | [x] |
| 156 | LZ4F_decompress_usingDict | dict supplied before init (dStage <= dstage_init) → applied; supplied mid-frame → ignored; dictSize > 1 GB → truncated to the last 64 KB | [x] |

## lz4file

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 157 | LZ4F_writeOpen | prefsPtr = NULL → maxWriteSize 64 KB; blockSizeID default/max64KB/max256KB/max1MB/max4MB; invalid blockSizeID → maxBlockSize_invalid; fp = NULL or handle = NULL → parameter_null | [x] |
| 158 | LZ4F_write | size = 0; size < maxWriteSize; size == maxWriteSize; size > maxWriteSize (chunking loop); buf = NULL or handle = NULL → parameter_null | [x] |
| 159 | LZ4F_writeOpen + LZ4F_write | non-default prefs forwarded to compressBegin/compressUpdate: contentChecksum, blockChecksum, contentSize, dictID, level 0/9/12, autoFlush 0/1, blockLinked vs blockIndependent | [x] |
| 160 | LZ4F_writeClose | normal close (compressEnd + fwrite); after a previous write error (errCode set → compressEnd skipped); handle = NULL → parameter_null | [x] |
| 161 | LZ4F_readOpen | file shorter than LZ4F_HEADER_SIZE_MAX (19) → io_read; frame blockSizeID → srcBufMaxSize 64 KB / 256 KB / 1 MB / 4 MB; invalid blockSizeID → maxBlockSize_invalid; fp/handle NULL → parameter_null | [x] |
| 162 | LZ4F_read, LZ4F_readClose | size = 0; size < remaining frame content; size > frame content (EOF break → short count); several sequential reads; frames with linked vs independent blocks and with checksums; handle/buf NULL → parameter_null | [x] |

## xxhash (namespaced LZ4_XXH*)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 163 | LZ4_XXH32 | length 0 with a valid pointer and with NULL; seed 0 and seed 0x9E3779B1 | [x] |
| 164 | LZ4_XXH32 | length 1, 2, 3 (PROCESS1 chain only) and 4, 5, 6, 7 (PROCESS4 + 0..3 PROCESS1) | [x] |
| 165 | LZ4_XXH32 | length 8..15 — every remaining `len & 15` residue class — with seed 0 and non-zero | [x] |
| 166 | LZ4_XXH32 | length 16 (first full stripe), 17, 31, 32, 100, 1 MB; seed 0 and non-zero; 4-byte-aligned and deliberately misaligned input pointers | [x] |
| 167 | LZ4_XXH32_createState, LZ4_XXH32_freeState, LZ4_XXH32_reset, LZ4_XXH32_update, LZ4_XXH32_digest | single update equal to the one-shot result; seed 0 and non-zero; freeState(NULL) | [x] |
| 168 | LZ4_XXH32_update | 1 byte at a time; chunks summing to < 16 (pure mem32 buffering, large_len unset); chunks crossing the 16-byte stripe boundary with an unaligned leftover; total >= 16 reached only via small chunks (large_len via total_len_32) | [x] |
| 169 | LZ4_XXH32_digest, LZ4_XXH32_copyState | digest called twice; digest then further updates then digest again; copyState then diverging updates on the copy | [x] |
| 170 | LZ4_XXH32_canonicalFromHash, LZ4_XXH32_hashFromCanonical | round-trip of 0, 0xFFFFFFFF and an arbitrary hash (big-endian canonical byte order) | [x] |
| 171 | LZ4_XXH64 | length 0 (valid pointer and NULL), 1..7, 8, 9..31 (every `len & 31` residue class), 32, 33, 63, 64, 1 MB; seed 0 and non-zero; 8-byte-aligned vs misaligned input | [x] |
| 172 | LZ4_XXH64_createState, LZ4_XXH64_freeState, LZ4_XXH64_reset, LZ4_XXH64_update, LZ4_XXH64_digest, LZ4_XXH64_copyState | single update equal to the one-shot; 1-byte chunks; chunks summing < 32 (mem64 buffering); chunks straddling the 32-byte stripe boundary; total_len < 32 vs >= 32 at digest time | [x] |
| 173 | LZ4_XXH64_canonicalFromHash, LZ4_XXH64_hashFromCanonical, LZ4_XXH_versionNumber | round-trip of 0, UINT64_MAX and an arbitrary hash; version constant | [x] |

## Phase B status — all 173 rows verified

Every row above is checked off. Each row `N` has at least one differential test whose
function name contains `row_N` (or `rows_..N..`), located in:

| rows | test file |
|---|---|
| 1-34    | `tests/lz4_block.rs` |
| 35-65   | `tests/lz4_stream.rs` |
| 66-103  | `tests/lz4hc.rs` |
| 104-141 | `tests/lz4frame_comp.rs` |
| 142-156 | `tests/lz4frame_decomp.rs` |
| 157-162 | `tests/lz4file.rs` |
| 163-173 | `tests/xxhash.rs` |

Audit the mapping mechanically with:

```sh
grep -ohE 'fn (row|rows)_[0-9]+[a-z_0-9]*' tests/*.rs \
  | tr -cs '0-9' '\n' | grep -E '^[0-9]+$' | sort -n -u > /tmp/rows
for i in $(seq 1 173); do grep -qx "$i" /tmp/rows || echo "UNCOVERED $i"; done
```

Every test is property-style with a fixed seed, sweeping many randomized inputs across
several sizes and data shapes per row. C and Rust always get separate `0xCD`-filled
destination buffers with guard tails, and both the return value and the full buffer are
compared; compressed output is additionally cross-decompressed (C output through the Rust
decoder and vice versa). All opaque contexts are created and freed by the owning library —
no context ever crosses the FFI boundary between the two `.so`s.
