# CONFIGS.md — Configuration-surface table (derived mechanically from `c_src/`)

One row per meaningful combination of options × input shape that the C actually
branches on differently. Every row is exercised by a differential test that calls
BOTH the C `.so` and the Rust `.so` via `libloading` with **many randomized inputs**
(fixed seed, property-style), asserting byte-identical output.

Axes derived from the source:

- **lz4.c**: `acceleration` (clamped `<1 → 1`, `>65537 → 65537`), output directive
  (`notLimited` / `limitedOutput` / `fillOutput`), table type (`byU16` when
  `inputSize < LZ4_64KLIMIT` = 65547, else `byU32`), dict directive
  (`noDict` / `withPrefix64k` / `usingExtDict` / `usingDictCtx`), `dictSmall`
  vs `noDictIssue`, decompress variant, size boundaries
  (`0`, `1`, `<MFLIMIT=12`, `13`, `64KB-1`, `64KB`, `65547`, `>64KB`).
- **lz4hc.c**: `compressionLevel` 1..12 (three strategies: `lz4mid` for 1–2,
  `lz4hc` hash-chain for 3–9 with `patternAnalysis` only at 9, `lz4opt` for 10–12
  with `ultra`/`fullUpdate` only at 12), out-of-range levels, `favorDecSpeed`,
  `LZ4_setCompressionLevel`, dictCtx directive (`noDictCtx` / `usingDictCtxHc`,
  with the `position>=64KB` detach and the `position==0 && srcSize>4KB` cold-start
  memcpy), streaming prefix vs extDict vs ring-buffer overlap.
- **lz4frame.c**: every `LZ4F_preferences_t` / `LZ4F_compressOptions_t` /
  `LZ4F_decompressOptions_t` field value the code branches on, all ~40 public
  entry points including the low-level `compressBegin`/`compressUpdate`/`flush`/
  `compressEnd` pipeline (not just `LZ4F_compressFrame`), and the 15-stage
  decompression state machine driven with 1-byte / random / whole-frame chunking.
- **lz4file.c**: `blockSizeID` buffer sizing, write/read chunk shapes.
- **xxhash.c**: XXH32 (16-byte stripe) vs XXH64 (32-byte stripe), seeds, all
  length classes around the stripe and finalize-switch boundaries, one-shot vs
  streaming with arbitrary splits, canonical round-trip, unaligned pointers.

---

## Group 1 — `lz4.c` one-shot compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `LZ4_compress_default` | srcSize=0, dstCapacity=compressBound | [x] |
| 2 | `LZ4_compress_default` | srcSize=1..11 (< MFLIMIT, all-literals path) | [x] |
| 3 | `LZ4_compress_default` | srcSize=12,13 (MFLIMIT boundary) | [x] |
| 4 | `LZ4_compress_default` | srcSize random 14..4096, random bytes (incompressible) | [x] |
| 5 | `LZ4_compress_default` | srcSize random 14..4096, highly repetitive (long matches) | [x] |
| 6 | `LZ4_compress_default` | srcSize 65535 / 65536 / 65546 / 65547 / 65548 (byU16→byU32 switch at LZ4_64KLIMIT) | [x] |
| 7 | `LZ4_compress_default` | srcSize 200000 (byU32 table, multi-64KB) | [x] |
| 8 | `LZ4_compress_default` | dstCapacity exactly = compressed size (tight fit) | [x] |
| 9 | `LZ4_compress_default` | dstCapacity = compressed size − 1 (must fail identically) | [x] |
| 10 | `LZ4_compress_fast` | acceleration = 1 | [x] |
| 11 | `LZ4_compress_fast` | acceleration = 0 and −1 and INT_MIN (clamped to 1) | [x] |
| 12 | `LZ4_compress_fast` | acceleration = 2,3,7,17,64 (distinct skip-strength) | [x] |
| 13 | `LZ4_compress_fast` | acceleration = 65536, 65537, 65538, INT_MAX (ACCELERATION_MAX clamp) | [x] |
| 14 | `LZ4_compress_fast_extState` | fresh `LZ4_sizeofState()` buffer, all acceleration classes | [x] |
| 15 | `LZ4_compress_fast_extState_fastReset` | reused state across many calls (dirty-table path) | [x] |
| 16 | `LZ4_compress_destSize` | targetDstSize large enough for whole input (fillOutput, full consume) | [x] |
| 17 | `LZ4_compress_destSize` | targetDstSize deliberately small → partial consume, check `*srcSizePtr` too | [x] |
| 18 | `LZ4_compress_destSize` | targetDstSize = 1, 2, 3, 12 (minimum fillOutput sizes) | [x] |
| 19 | `LZ4_compress_destSize_extState` | same as 16–18 via explicit state + acceleration | [x] |
| 20 | `LZ4_compressBound` | inputSize 0,1,12,65535,65536,0x7E000000, 0x7E000001, −1 | [x] |
| 21 | `LZ4_sizeofState`, `LZ4_versionNumber`, `LZ4_versionString` | — | [x] |

## Group 2 — `lz4.c` decompression variants

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 22 | `LZ4_decompress_safe` | valid block, dstCapacity == original size | [x] |
| 23 | `LZ4_decompress_safe` | valid block, dstCapacity > original size | [x] |
| 24 | `LZ4_decompress_safe` | valid block, dstCapacity < original size (must fail identically) | [x] |
| 25 | `LZ4_decompress_safe` | random/corrupted bytes as input (fuzz, many seeds) — exact negative value must match | [x] |
| 26 | `LZ4_decompress_safe` | truncated valid block at every prefix length | [x] |
| 27 | `LZ4_decompress_safe_partial` | targetOutputSize = 0, 1, half, exact, > original | [x] |
| 28 | `LZ4_decompress_safe_partial` | targetOutputSize < dstCapacity and > dstCapacity (MIN() clamp) | [x] |
| 29 | `LZ4_decompress_safe_partial` | corrupted input with partial target | [x] |
| 30 | `LZ4_decompress_fast` | valid block, exact originalSize | [x] |
| 31 | `LZ4_decompress_fast` | corrupted input (unsafe generic, `-1` on output-defense) | [x] |
| 32 | `LZ4_decompress_safe_usingDict` | dictSize = 0 (→ plain path) | [x] |
| 33 | `LZ4_decompress_safe_usingDict` | contiguous dict (dict end == dst), dictSize ≥ 64KB−1 (→ prefix64k) | [x] |
| 34 | `LZ4_decompress_safe_usingDict` | contiguous dict, dictSize < 64KB−1 (→ smallPrefix) | [x] |
| 35 | `LZ4_decompress_safe_usingDict` | non-contiguous dict (→ forceExtDict) | [x] |
| 36 | `LZ4_decompress_safe_partial_usingDict` | all three dict placements × partial targets | [x] |
| 37 | `LZ4_decompress_fast_usingDict` | contiguous and non-contiguous dict | [x] |
| 38 | `LZ4_decompress_safe_withPrefix64k` / `LZ4_decompress_fast_withPrefix64k` | 64KB prefix immediately before dst | [x] |
| 39 | `LZ4_decompress_safe_forceExtDict` | separate dict buffer | [x] |
| 40 | `LZ4_decompress_safe_partial_forceExtDict` | separate dict buffer × partial target | [x] |
| 41 | `LZ4_decoderRingBufferSize` | 0,1,12,65536,0x7E000000, 0x7E000001, −1 | [x] |

## Group 3 — `lz4.c` streaming compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 42 | `LZ4_createStream` + `LZ4_compress_fast_continue` | contiguous prefix chaining, N random chunks, acceleration 1 | [x] |
| 43 | same | contiguous chaining with acceleration 5 and 65537 | [x] |
| 44 | same | non-contiguous buffers → `usingExtDict` path | [x] |
| 45 | same | ring buffer smaller than 64KB with wraparound (`LZ4_decoderRingBufferSize`) | [x] |
| 46 | `LZ4_initStream` | user buffer, correct size & alignment; then full stream compress | [x] |
| 47 | `LZ4_resetStream` / `LZ4_resetStream_fast` | reset between streams, verify identical restart | [x] |
| 48 | `LZ4_loadDict` + `LZ4_compress_fast_continue` | dictSize 0, 1, 3 (< HASH_UNIT), 4, 1000, 65535, 65536, 70000 (truncation) | [x] |
| 49 | `LZ4_loadDictSlow` + `LZ4_compress_fast_continue` | same dictSize matrix (denser 2nd-pass indexing) | [x] |
| 50 | `LZ4_attach_dictionary` + `LZ4_compress_fast_continue` | attached dictStream, first chunk < 4KB (`usingDictCtx`) | [x] |
| 51 | `LZ4_attach_dictionary` | first chunk > 4KB (cold-start / dictCtx bypass threshold) | [x] |
| 52 | `LZ4_attach_dictionary(stream, NULL)` | detach → `noDict` | [x] |
| 53 | `LZ4_saveDict` | after streaming, dictSize 0, 1, 3, 4, 65536, 70000; then continue compressing | [x] |
| 54 | `LZ4_compress_forceExtDict` | explicit ext-dict entry point | [x] |
| 55 | `LZ4_decompress_safe_continue` | first block (no history) | [x] |
| 56 | `LZ4_decompress_safe_continue` | rolling contiguous prefix ≥ 64KB−1 (→ prefix64k) | [x] |
| 57 | `LZ4_decompress_safe_continue` | rolling contiguous prefix < 64KB−1, extDictSize==0 (→ smallPrefix) | [x] |
| 58 | `LZ4_decompress_safe_continue` | rolling with non-zero extDictSize (→ doubleDict) | [x] |
| 59 | `LZ4_decompress_safe_continue` | ring-buffer wraparound (→ forceExtDict) | [x] |
| 60 | `LZ4_decompress_fast_continue` | same four dispatch cases | [x] |
| 61 | `LZ4_setStreamDecode` | with dict / with NULL dict / dictSize 0 then continue-decode | [x] |
| 62 | round-trip | compress-stream → decompress-stream, contiguous | [x] |
| 63 | round-trip | compress-stream → decompress-stream, ring buffer both sides | [x] |

## Group 4 — `lz4.c` deprecated / legacy exports

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 64 | `LZ4_compress` | srcSize across all boundary classes (acc fixed at 1) | [x] |
| 65 | `LZ4_compress_limitedOutput` | dstCapacity sufficient and insufficient | [x] |
| 66 | `LZ4_compress_withState` / `LZ4_compress_limitedOutput_withState` | `LZ4_sizeofState()` buffer | [x] |
| 67 | `LZ4_compress_continue` / `LZ4_compress_limitedOutput_continue` | via `LZ4_create` / `LZ4_createStream` state | [x] |
| 68 | `LZ4_uncompress` | valid + corrupt input | [x] |
| 69 | `LZ4_uncompress_unknownOutputSize` | valid + corrupt input | [x] |
| 70 | `LZ4_sizeofStreamState` / `LZ4_resetStreamState` | init then compress via continue | [x] |
| 71 | `LZ4_create` / `LZ4_freeStream` | allocate legacy state, compress, free | [x] |
| 72 | `LZ4_slideInputBuffer` | after streaming; returned offset must match | [x] |

## Group 5 — `lz4hc.c` one-shot HC compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 73 | `LZ4_compress_HC` | level 1 (→ `lz4mid`, NOT promoted to 9) × all size classes | [x] |
| 74 | `LZ4_compress_HC` | level 2 (`LZ4HC_CLEVEL_MIN`, `lz4mid`) × all size classes | [x] |
| 75 | `LZ4_compress_HC` | level 3,4,5,6,7,8 (`lz4hc` hash-chain, `patternAnalysis` off) | [x] |
| 76 | `LZ4_compress_HC` | level 9 (`LZ4HC_CLEVEL_DEFAULT`, `patternAnalysis` **on**) | [x] |
| 77 | `LZ4_compress_HC` | level 10 (`OPT_MIN`, `lz4opt`, nbSearches 96, targetLen 64) | [x] |
| 78 | `LZ4_compress_HC` | level 11 (`lz4opt`, 512/128) | [x] |
| 79 | `LZ4_compress_HC` | level 12 (`MAX`, `lz4opt`, 16384/LZ4_OPT_NUM, `ultra`+`fullUpdate`) | [x] |
| 80 | `LZ4_compress_HC` | level 0, −1, INT_MIN (→ 9) and 13, 100, INT_MAX (→ 12) | [x] |
| 81 | `LZ4_compress_HC` | every level × srcSize {0,1,11,12,13,64,65535,65536,65547,200000} | [x] |
| 82 | `LZ4_compress_HC` | every level × repetitive input (long-match / pattern-analysis path) | [x] |
| 83 | `LZ4_compress_HC` | dstCapacity tight and one-byte-short (limitedOutput vs notLimited) | [x] |
| 84 | `LZ4_compress_HC_extStateHC` | `LZ4_sizeofStateHC()` buffer, all levels | [x] |
| 85 | `LZ4_compress_HC_extStateHC_fastReset` | reused state across calls, all levels | [x] |
| 86 | `LZ4_compress_HC_destSize` | targetDstSize large / small / 1 / 2 / 12, all levels; check `*srcSizePtr` | [x] |
| 87 | `LZ4_initStreamHC` | user buffer of `LZ4_sizeofStateHC()`, then compress | [x] |
| 88 | `LZ4_compressionLevel_max` / `LZ4_sizeofStateHC` | — | [x] |

## Group 6 — `lz4hc.c` streaming HC + options

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 89 | `LZ4_createStreamHC` + `LZ4_resetStreamHC` + `LZ4_compress_HC_continue` | contiguous chaining, levels 2, 9, 12 | [x] |
| 90 | `LZ4_resetStreamHC_fast` + `LZ4_compress_HC_continue` | contiguous chaining, all levels | [x] |
| 91 | `LZ4_compress_HC_continue` | non-contiguous chunks → `LZ4HC_setExternalDict`, levels 2, 9, 12 | [x] |
| 92 | `LZ4_compress_HC_continue` | ring buffer with wraparound / overlap shrink, levels 2, 9, 12 | [x] |
| 93 | `LZ4_compress_HC_continue_destSize` | fillOutput streaming, levels 2, 9, 12 | [x] |
| 94 | `LZ4_loadDictHC` + `LZ4_compress_HC_continue` | dictSize 0,1,3,4,1000,65535,65536,70000; levels 2, 9, 12 | [x] |
| 95 | `LZ4_attach_HC_dictionary` + continue | first chunk < 4KB (`usingDictCtxHc`), levels 2, 9, 12 | [x] |
| 96 | `LZ4_attach_HC_dictionary` + continue | first chunk > 4KB (cold-start memcpy path), `isStateCompatible` both-mid / both-non-mid / mixed | [x] |
| 97 | `LZ4_attach_HC_dictionary` + continue | total position ≥ 64KB → dictCtx detach | [x] |
| 98 | `LZ4_attach_HC_dictionary(s, NULL)` | detach → `noDictCtx` | [x] |
| 99 | `LZ4_saveDictHC` | dictSize 0,1,3,4,65536,70000; then continue compressing | [x] |
| 100 | `LZ4_setCompressionLevel` | mid-stream change to 1,2,3,9,10,12,0,−1,13,INT_MAX | [x] |
| 101 | `LZ4_favorDecompressionSpeed` | 0 vs 1 × levels 10,11,12 (only `lz4opt` honours it) | [x] |
| 102 | `LZ4_favorDecompressionSpeed` | 0 vs 1 × levels 2 and 9 (must be a no-op) | [x] |
| 103 | round-trip | HC compress (each level) → `LZ4_decompress_safe` | [x] |
| 104 | round-trip | HC streaming → `LZ4_decompress_safe_continue` | [x] |

## Group 7 — `lz4hc.c` deprecated / legacy exports

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 105 | `LZ4_compressHC` | all size classes (level fixed 0 → 9) | [x] |
| 106 | `LZ4_compressHC_limitedOutput` | dstCapacity sufficient / insufficient | [x] |
| 107 | `LZ4_compressHC2` | cLevel 0,1,2,9,12,13,−1,INT_MAX | [x] |
| 108 | `LZ4_compressHC2_limitedOutput` | cLevel matrix × tight dstCapacity | [x] |
| 109 | `LZ4_compressHC_withStateHC` / `LZ4_compressHC_limitedOutput_withStateHC` | `LZ4_sizeofStateHC()` buffer | [x] |
| 110 | `LZ4_compressHC2_withStateHC` / `LZ4_compressHC2_limitedOutput_withStateHC` | cLevel matrix | [x] |
| 111 | `LZ4_compressHC_continue` / `LZ4_compressHC_limitedOutput_continue` | streaming via `LZ4_createStreamHC` | [x] |
| 112 | `LZ4_compressHC2_continue` / `LZ4_compressHC2_limitedOutput_continue` | degraded path (dstCapacity=0 notLimited / limitedOutput, no extDict handling) | [x] |
| 113 | `LZ4_sizeofStreamStateHC` / `LZ4_resetStreamStateHC` | init then compress via continue | [x] |
| 114 | `LZ4_createHC` / `LZ4_freeHC` | legacy alloc, compress, free | [x] |
| 115 | `LZ4_slideInputBufferHC` | after streaming; returned offset must match | [x] |
| 116 | `LZ4HC_searchExtDict` | exported symbol; called directly with a prepared ext-dict state | [x] |

## Group 8 — `lz4frame.c` one-shot frame compression (preferences cross-product)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 117 | `LZ4F_compressFrame` | `prefsPtr == NULL` (all defaults) × size classes | [x] |
| 118 | `LZ4F_compressFrame` | blockSizeID 0 (default→64KB) × blockMode linked | [x] |
| 119 | `LZ4F_compressFrame` | blockSizeID 4 (max64KB) × blockMode linked / independent | [x] |
| 120 | `LZ4F_compressFrame` | blockSizeID 5 (max256KB) × blockMode linked / independent | [x] |
| 121 | `LZ4F_compressFrame` | blockSizeID 6 (max1MB) × blockMode linked / independent | [x] |
| 122 | `LZ4F_compressFrame` | blockSizeID 7 (max4MB) × blockMode linked / independent | [x] |
| 123 | `LZ4F_compressFrame` | contentChecksumFlag 0 vs 1 | [x] |
| 124 | `LZ4F_compressFrame` | blockChecksumFlag 0 vs 1 | [x] |
| 125 | `LZ4F_compressFrame` | contentChecksum × blockChecksum (all 4) | [x] |
| 126 | `LZ4F_compressFrame` | contentSize = 0 (unknown) vs = exact srcSize (declared) | [x] |
| 127 | `LZ4F_compressFrame` | dictID = 0 vs 0xDEADBEEF (header field present) | [x] |
| 128 | `LZ4F_compressFrame` | compressionLevel 0 (fast default) | [x] |
| 129 | `LZ4F_compressFrame` | compressionLevel −1, −5, −65537 (fast mode / acceleration) | [x] |
| 130 | `LZ4F_compressFrame` | compressionLevel 1, 2 (LZ4HC_CLEVEL_MIN boundary → HC ctx) | [x] |
| 131 | `LZ4F_compressFrame` | compressionLevel 3,6,9 (HC hash-chain) | [x] |
| 132 | `LZ4F_compressFrame` | compressionLevel 10,11,12 (HC optimal) | [x] |
| 133 | `LZ4F_compressFrame` | compressionLevel 13, 100, INT_MAX (clamped) | [x] |
| 134 | `LZ4F_compressFrame` | autoFlush 0 vs 1 | [x] |
| 135 | `LZ4F_compressFrame` | favorDecSpeed 0 vs 1 × compressionLevel 10/12 | [x] |
| 136 | `LZ4F_compressFrame` | frameType `LZ4F_skippableFrame` (1) | [x] |
| 137 | `LZ4F_compressFrame` | srcSize 0, 1, 11, 12, 13, 65535, 65536, 65537, 262144, 300000 | [x] |
| 138 | `LZ4F_compressFrame` | srcSize > blockSize (multi-block) for each blockSizeID | [x] |
| 139 | `LZ4F_compressFrame` | incompressible random vs highly repetitive input (stored-block path) | [x] |
| 140 | `LZ4F_compressFrame` | dstCapacity == `compressFrameBound` and one byte short | [x] |
| 141 | `LZ4F_compressFrameBound` | srcSize 0,1,65536,262144,1<<20,1<<22 × each prefs combination above | [x] |
| 142 | `LZ4F_compressFrame_usingCDict` | CDict of size 0,1,4,1000,65536,70000 × prefs matrix | [x] |
| 143 | `LZ4F_createCDict` / `LZ4F_createCDict_advanced` / `LZ4F_freeCDict` | dictSize matrix; then compress with it | [x] |
| 144 | `LZ4F_getBlockSize` | blockSizeID 0,4,5,6,7 (valid) | [x] |
| 145 | `LZ4F_compressBound` | srcSize matrix × prefs matrix (incl. `NULL` prefs, autoFlush 0/1) | [x] |
| 146 | `LZ4F_getVersion`, `LZ4F_compressionLevel_max` | — | [x] |

## Group 9 — `lz4frame.c` LOW-LEVEL streaming compression pipeline

Driven the way a real consumer does: `createCompressionContext` →
`compressBegin*` → many `compressUpdate` / `uncompressedUpdate` / `flush` →
`compressEnd`, with the byte stream accumulated and compared in full.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 147 | `LZ4F_createCompressionContext` + `compressBegin` + `compressUpdate` ×N + `compressEnd` | `NULL` prefs, N random chunk sizes | [x] |
| 148 | same pipeline | full prefs cross-product (blockSizeID 4–7 × blockMode × contentChecksum × blockChecksum) | [x] |
| 149 | same pipeline | autoFlush = 1 (each update emits a block) | [x] |
| 150 | same pipeline | autoFlush = 0 with 1-byte updates (internal tmpBuff accumulation) | [x] |
| 151 | same pipeline | chunk sizes straddling blockSize exactly / blockSize+1 / blockSize−1 | [x] |
| 152 | same pipeline | `cOpt.stableSrc = 1` vs `0` | [x] |
| 153 | same pipeline | explicit `LZ4F_flush` interleaved between updates (incl. flush with empty buffer) | [x] |
| 154 | same pipeline | contentSize declared and satisfied exactly | [x] |
| 155 | same pipeline | compressionLevel across fast(−5,0), HC(2,9), opt(10,12) × blockMode linked/independent | [x] |
| 156 | `LZ4F_uncompressedUpdate` | only-uncompressed-blocks frame, various sizes | [x] |
| 157 | `LZ4F_uncompressedUpdate` | interleaved with `LZ4F_compressUpdate` (block-mode switching) | [x] |
| 158 | `LZ4F_uncompressedUpdate` | with blockChecksum on, autoFlush on/off | [x] |
| 159 | `LZ4F_compressBegin_usingCDict` | CDict size matrix × prefs matrix, full pipeline | [x] |
| 160 | `LZ4F_compressBegin_usingDict` | raw dict buffer, dictSize matrix, full pipeline | [x] |
| 161 | `LZ4F_compressBegin_usingDictOnce` | raw dict, only first block uses it | [x] |
| 162 | `LZ4F_createCompressionContext_advanced` | custom-mem variant (NULL customMem), then full pipeline | [x] |
| 163 | cctx reuse | `compressEnd` then `compressBegin` again on the SAME cctx (two frames) | [x] |
| 164 | cctx reuse | switch prefs (level fast→HC→opt) between frames on the same cctx (`lz4CtxAlloc`/`lz4CtxType` transitions) | [x] |

## Group 10 — `lz4frame.c` decompression state machine

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 165 | `LZ4F_headerSize` | valid frame headers with/without contentSize/dictID; skippable magic | [x] |
| 166 | `LZ4F_getFrameInfo` | before any decompress, whole header available | [x] |
| 167 | `LZ4F_getFrameInfo` | after partial decompression (mid-frame) | [x] |
| 168 | `LZ4F_decompress` | whole frame in one call, `dstCapacity` ample (direct-decode-into-dst path) | [x] |
| 169 | `LZ4F_decompress` | whole frame in one call, `dstCapacity` = 1 (forced internal buffering) | [x] |
| 170 | `LZ4F_decompress` | src fed 1 byte at a time (all `store*` reassembly sub-stages) | [x] |
| 171 | `LZ4F_decompress` | src fed in random chunk sizes (fixed seed) | [x] |
| 172 | `LZ4F_decompress` | dst supplied in small random chunks (partial-output stages) | [x] |
| 173 | `LZ4F_decompress` | frames produced with each blockSizeID × blockMode × checksum combination | [x] |
| 174 | `LZ4F_decompress` | frame containing uncompressed (stored) blocks | [x] |
| 175 | `LZ4F_decompress` | frame with declared contentSize (validation path) | [x] |
| 176 | `LZ4F_decompress` | skippable frame (`dstage_skipSkippable`), various skip sizes incl. 0 | [x] |
| 177 | `LZ4F_decompress` | skippable frame followed by a real frame in the same buffer | [x] |
| 178 | `LZ4F_decompress` | `dOpt.stableDst = 1` vs `0`, linked blocks (dict-preservation path) | [x] |
| 179 | `LZ4F_decompress` | `dOpt.skipChecksums = 1` on a frame WITH content+block checksums | [x] |
| 180 | `LZ4F_decompress` | `dOpt.skipChecksums = 1` on a frame with a **corrupted** checksum (must still succeed) | [x] |
| 181 | `LZ4F_decompress` | `dOpt == NULL` | [x] |
| 182 | `LZ4F_resetDecompressionContext` | mid-frame reset, then decode a fresh frame | [x] |
| 183 | `LZ4F_decompress_usingDict` | dictSize matrix (0,1,4,1000,65536,70000), frame compressed with matching dict | [x] |
| 184 | `LZ4F_decompress_usingDict` | 1-byte-at-a-time feeding with a dict | [x] |
| 185 | `LZ4F_createDecompressionContext_advanced` | custom-mem variant (NULL customMem), then decode | [x] |
| 186 | `LZ4F_freeDecompressionContext` | after complete frame (returns 0) vs mid-frame (returns dStage) | [x] |
| 187 | round-trip | for EVERY prefs combination: compress with C, decompress with Rust and vice-versa | [x] |

## Group 11 — `lz4file.c`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 188 | `LZ4F_writeOpen` + `LZ4F_write` + `LZ4F_writeClose` | `prefs == NULL`, single write of various sizes | [x] |
| 189 | same | blockSizeID 4,5,6,7 (buffer-sizing branch) | [x] |
| 190 | same | blockMode linked/independent × contentChecksum × blockChecksum | [x] |
| 191 | same | contentSize declared vs unknown | [x] |
| 192 | same | compressionLevel −1, 0, 2, 9, 12 | [x] |
| 193 | same | write size 0, 1, < maxWriteSize, == maxWriteSize, > maxWriteSize (chunking) | [x] |
| 194 | same | many small writes accumulating into one block | [x] |
| 195 | `LZ4F_readOpen` + `LZ4F_read` + `LZ4F_readClose` | read whole file in one call | [x] |
| 196 | same | read 1 byte at a time | [x] |
| 197 | same | read size 0, exactly a block, larger than remaining (partial return at EOF) | [x] |
| 198 | same | read frames written with each blockSizeID / checksum combination | [x] |
| 199 | write+read round-trip | C writes → Rust reads, and Rust writes → C reads, byte-identical payload | [x] |

## Group 12 — `xxhash.c` (`LZ4_XXH*`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 200 | `LZ4_XXH32` | len 0 (short path, no read) | [x] |
| 201 | `LZ4_XXH32` | len 1..15 (below 16-byte stripe; finalize `switch len&15` all cases) | [x] |
| 202 | `LZ4_XXH32` | len 16 (exactly one stripe, remainder 0) | [x] |
| 203 | `LZ4_XXH32` | len 17,31,32,33,63,64,65,127,128 (stripes + every remainder class) | [x] |
| 204 | `LZ4_XXH32` | len random 1..8192, many seeds | [x] |
| 205 | `LZ4_XXH32` | seed 0, 1, 0x7FFFFFFF, 0xFFFFFFFF (v4 = seed − PRIME1 wrap) | [x] |
| 206 | `LZ4_XXH32` | unaligned input pointer (offset 1,2,3 into buffer) | [x] |
| 207 | `LZ4_XXH64` | len 0 (short path) | [x] |
| 208 | `LZ4_XXH64` | len 1..7 (below 8; finalize byte loop) | [x] |
| 209 | `LZ4_XXH64` | len 8,16,24 (PROCESS8_64, below 32-byte stripe) | [x] |
| 210 | `LZ4_XXH64` | len 31,32,33,63,64,65,127,128 (stripes + every remainder class) | [x] |
| 211 | `LZ4_XXH64` | len random 1..8192, many seeds | [x] |
| 212 | `LZ4_XXH64` | seed 0, 1, 0xFFFFFFFF, u64::MAX | [x] |
| 213 | `LZ4_XXH64` | unaligned input pointer (offsets 1..7) | [x] |
| 214 | `LZ4_XXH32_createState`/`reset`/`update`/`digest`/`freeState` | whole input in one `update` | [x] |
| 215 | same | 1-byte-at-a-time updates (mem32 buffering) | [x] |
| 216 | same | random chunk splits crossing the 16-byte buffer boundary | [x] |
| 217 | same | update with len 0 and non-NULL pointer | [x] |
| 218 | same | multiple `digest()` calls (digest must not mutate state) | [x] |
| 219 | `LZ4_XXH32_copyState` | copy mid-stream, continue both, compare digests | [x] |
| 220 | `LZ4_XXH64_*` streaming | one-shot vs 1-byte vs random splits crossing the 32-byte buffer | [x] |
| 221 | `LZ4_XXH64_copyState` | copy mid-stream, continue both, compare digests | [x] |
| 222 | `LZ4_XXH32_canonicalFromHash` / `hashFromCanonical` | round trip over many hashes; canonical bytes are big-endian | [x] |
| 223 | `LZ4_XXH64_canonicalFromHash` / `hashFromCanonical` | round trip over many hashes; canonical bytes are big-endian | [x] |
| 224 | `LZ4_XXH_versionNumber` | — (must be 605) | [x] |
| 225 | state struct sizes | `createState` + `reset` + `copyState` interop between C and Rust (same layout) | [x] |

---

## Phase B status

All 225 rows above pass, each driven with many randomized inputs from a fixed
seed (`splitmix64`, no external crates) across six payload shapes
(`Random`, `SmallAlphabet`, `Constant`, `Periodic`, `Textish`, `Chunky`).
Tests live in:

| file | tests | groups |
|------|-------|--------|
| `tests/harness_selfcheck.rs` | 3 | proves the two `.so`s are distinct objects and cross-decode each other |
| `tests/lz4_block.rs` | 14 | 1, 2, 4 |
| `tests/lz4_stream.rs` | 8 | 3 |
| `tests/lz4hc.rs` | 14 | 5, 6, 7 |
| `tests/repro_hc.rs` | 2 | minimal HC streaming reproducers |
| `tests/lz4frame_oneshot.rs` | 4 | 8 |
| `tests/lz4frame_pipeline.rs` | 6 | 9 |
| `tests/lz4frame_decode.rs` | 9 | 10 |
| `tests/lz4file.rs` | 2 | 11 |
| `tests/xxhash.rs` | 7 | 12 |

Beyond comparing return values and output bytes, several tests compare the
**internal state** byte-for-byte after each call (`LZ4_compress_fast_extState`,
`LZ4_initStream`, `LZ4_initStreamHC`, `LZ4_resetStreamState`, and the
`XXH32_state_t` / `XXH64_state_t` layouts), and the xxhash tests feed a state
produced by one library into the other library's `digest`. `LZ4HC_searchExtDict`
— an exported symbol absent from the public header — is called directly and its
by-value `{off, len, back}` struct return compared.

### No binary executable

`c_src/CMakeLists.txt` declares only `add_library(lz4 SHARED …)` and
`translation/Cargo.toml` declares only `[lib] crate-type = ["cdylib"]`. Neither
side builds a driver program, so the "compare binary stdout" item does not apply.

### Test-harness contract violations found while writing Phase B

These were bugs in the *tests*, not the translation, and each is now documented
in the test that hit it. They are recorded because in every case both libraries
misbehaved *identically at first glance* while diverging for reasons unrelated to
the translation — exactly the kind of false positive that wastes a debugging pass:

1. **Freed source blocks between `_continue` calls.** `lz4.h:431` — "The previous
   64KB of source data is __assumed__ to remain present, unmodified, at same
   address in memory!" (and `lz4hc.h:134` for HC). Dropping a block is a
   use-after-free; the C and Rust `.so`s have independent allocators, so they
   read different residual bytes out of the freed dictionary tail. Fixed with
   `common::BlockArena`, which parks every block for the life of the stream.
2. **Zero-capacity `Vec` as a `src` pointer.** `Vec::new().as_ptr()` is a
   dangling non-null pointer; the level-10+ optimal parser reached through
   `LZ4_compress_HC_destSize` dereferences `src` before consulting `srcSize`, so
   both libraries segfault. `common::mkdata` now always backs the slice with at
   least 64 zero-initialised bytes.
3. **`LZ4_compress_forceExtDict` is `notLimited`** (4 args, no `dstCapacity`), so
   `dst` must be at least `LZ4_compressBound(srcSize)`.
4. **`LZ4_compressHC2_continue` after `LZ4_slideInputBufferHC`.**
   `LZ4_resetStreamHC_fast` sets `prefixStart = NULL`, and the degraded
   `HC2_continue` path calls `LZ4HC_compress_generic` directly without the
   auto-init that `LZ4_compress_HC_continue` performs — the C nul-derefs.
5. **`LZ4F_uncompressedUpdate` requires `LZ4F_blockIndependent`**
   (`lz4frame.h:707`) and `dstCapacity >= srcSize` (`lz4frame.h:701`).
6. **`stableDst` requires a persistent output buffer.** The pledge is that the
   previously decoded 64 KB sits immediately before `dstBuffer`; handing the
   decoder a fresh allocation each call makes both libraries produce the same
   wrong bytes.
7. **`frameInfo.frameType` is a read-only field** (`lz4frame.h:177`): setting it
   does not make `LZ4F_compressFrame` emit a skippable frame.
8. **`LZ4F_readOpen` `fread`s the full 19-byte `LZ4F_HEADER_SIZE_MAX`**
   (`lz4file.c:92`), so frames for tiny payloads legitimately fail with
   `ERROR_io_read`.
