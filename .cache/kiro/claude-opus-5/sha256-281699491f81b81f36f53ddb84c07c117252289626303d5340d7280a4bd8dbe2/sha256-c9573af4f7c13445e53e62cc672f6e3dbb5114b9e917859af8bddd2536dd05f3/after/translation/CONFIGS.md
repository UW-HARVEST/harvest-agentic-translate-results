# CONFIGS.md — Configuration-surface table

Axes derived from the C source's actual branches, not from guesses.

## Axes

### lz4.c (block API)
- `A1` acceleration: `<1` (→1), 1, 2, 17, 65537, `>MAX` (clamped)
- `A2` srcSize regime: 0, 1, `<MINMATCH` (1..3), 4..12, `<LZ4_64Klimit` (65534) → `byU16`
  table, `>= LZ4_64Klimit` → `byU32` table, `> 128KB` (multi-window)
- `A3` output directive: `notLimited` (`dstCapacity >= compressBound`),
  `limitedOutput` (smaller), `fillOutput` (`LZ4_compress_destSize`)
- `A4` dict directive: `noDict`, `withPrefix64k`, `usingExtDict`, `usingDictCtx`
  (via `LZ4_attach_dictionary`)
- `A5` dictIssue: `noDictIssue`, `dictSmall` (dictSize<64KB && dictSize<currentOffset)
- `A6` data entropy: incompressible (random), highly compressible (constant),
  mixed, long-match (>15 match len → extra length bytes), long-literal (>15 lits)
- `A7` decode variant: `safe`, `safe_partial`, `fast` (deprecated),
  `safe_continue`, `fast_continue`, `safe_usingDict`, `fast_usingDict`,
  `safe_partial_usingDict`, `safe_withPrefix64k`, `fast_withPrefix64k`,
  `uncompress`, `uncompress_unknownOutputSize`
- `A8` state provisioning: internal (`LZ4_compress_fast`), extState,
  extState_fastReset, createStream, initStream on user buffer, deprecated
  `LZ4_create`/`resetStreamState`

### lz4hc.c
- `B1` compressionLevel: <1 (→9), 1, 2, 3 (`lz4mid`), 4..9 (`lz4hc`), 10, 11,
  12 (`lz4opt`, ultra at 12), >12 (→12)
- `B2` favorDecSpeed: 0, 1 (only affects level >= LZ4HC_CLEVEL_OPT_MIN)
- `B3` entry: `LZ4_compress_HC`, `_extStateHC`, `_extStateHC_fastReset`,
  `_destSize`, `_continue`, `_continue_destSize`, deprecated `LZ4_compressHC*`
- `B4` dict: none, `LZ4_loadDictHC`, `LZ4_attach_HC_dictionary`, `saveDictHC` round-trip
- `B5` srcSize regime: 0, 1..3, 4..64, 65535, 65536, >256KB
- `B6` streaming chunking: one call vs many `_continue` calls

### lz4frame.c
- `C1` blockSizeID: 0 (default→max64KB), 4, 5, 6, 7
- `C2` blockMode: `blockLinked` (0), `blockIndependent` (1)
- `C3` contentChecksumFlag: 0, 1
- `C4` blockChecksumFlag: 0, 1
- `C5` contentSize: 0 (unknown), exact value (adds 8-byte field + frameSize check)
- `C6` dictID: 0, non-zero (adds 4-byte field)
- `C7` compressionLevel: -5 (fast accel), 0 (default fast), 1, 3, 9, 10, 12, 20 (→12)
- `C8` autoFlush: 0, 1 (changes required tmp buffer size)
- `C9` favorDecSpeed: 0, 1
- `C10` frameType: `LZ4F_frame`, `LZ4F_skippableFrame` (decode side)
- `C11` cctx entry style: one-shot `LZ4F_compressFrame`; streaming
  `compressBegin`/`compressUpdate`*/`flush`/`compressEnd`;
  `compressBegin_usingDict`; `compressBegin_usingCDict`;
  `compressBegin_usingDictOnce`; `compressFrame_usingCDict`;
  `createCompressionContext_advanced` (custom mem)
- `C12` compressOptions.stableSrc: 0, 1
- `C13` decompressOptions: stableDst 0/1, skipChecksums 0/1
- `C14` dctx feeding granularity: whole frame at once, 1 byte at a time,
  random-size chunks, header split across calls
- `C15` decode entry: `LZ4F_decompress`, `LZ4F_decompress_usingDict`,
  `LZ4F_getFrameInfo` first then decompress, `LZ4F_headerSize`
- `C16` dictionary: none, small (<64KB), large (>64KB), CDict, DictOnce

### lz4file.c
- `D1` write path: `LZ4F_writeOpen` with NULL prefs / each prefs combo, many
  `LZ4F_write` sizes (0, 1, < maxWriteSize, > maxWriteSize), `LZ4F_writeClose`
- `D2` read path: `LZ4F_readOpen`, `LZ4F_read` sizes (0, 1, exact, oversized),
  `LZ4F_readClose`
- `D3` round-trip through a real temp FILE*

### xxhash.c
- `E1` width: XXH32, XXH64
- `E2` length regime: 0, 1..3, 4..15, 16 (one 32-bit stripe / 64-bit lane),
  31, 32, 33, 63, 64, 65, large (>1KB)
- `E3` seed: 0, 1, 0xFFFFFFFF, 64-bit large seeds
- `E4` entry: one-shot, streaming reset/update*/digest, copyState,
  canonicalFromHash/hashFromCanonical
- `E5` streaming chunking: 1-byte, random chunks, single chunk (exercises
  the internal 16-byte (32) / 32-byte (64) memory buffer boundary)
- `E6` alignment: input pointer offset 0..7 from allocation base

## Rows (cross-product pruned to what the C distinguishes)

| #  | entry point(s) | configuration (options + input shape) | [x] |
|----|----------------|----------------------------------------|-----|
| 1  | `LZ4_compressBound` | srcSize ∈ {0,1,4,63,64,65535,65536,0x7E000000, negative, >max} | [x] |
| 2  | `LZ4_compress_default` | srcSize 0..64, all data patterns, dst = compressBound | [x] |
| 3  | `LZ4_compress_default` | srcSize 1..65535 random (byU16 table), dst = compressBound | [x] |
| 4  | `LZ4_compress_default` | srcSize 65536..300000 (byU32 table), dst = compressBound | [x] |
| 5  | `LZ4_compress_default` | dstCapacity = exact-1, bound/2, 1 (limitedOutput fail/partial) | [x] |
| 6  | `LZ4_compress_fast` | accel ∈ {-1,0,1,2,3,17,1000,65536,65537,100000}, mixed sizes | [x] |
| 7  | `LZ4_compress_fast_extState` | accel sweep × srcSize regimes × notLimited/limited | [x] |
| 8  | `LZ4_compress_fast_extState_fastReset` | same, state reused across calls (currentOffset!=0 → dictSmall) | [x] |
| 9  | `LZ4_compress_destSize` | targetDstSize ∈ {0,1,2,8,srcSize/4,/2,bound}, srcSize regimes | [x] |
| 10 | `LZ4_compress_destSize` | targetDstSize >= compressBound (fallback path) | [x] |
| 11 | `LZ4_sizeofState`, `LZ4_sizeofStreamState`, `LZ4_sizeofStateHC`, `LZ4_sizeofStreamStateHC` | — | [x] |
| 12 | `LZ4_initStream` | user buffer exact size, oversized, aligned; then compress | [x] |
| 13 | `LZ4_createStream`+`LZ4_resetStream`+`LZ4_compress_fast_continue` | prefix mode (contiguous src), many chunks | [x] |
| 14 | `LZ4_createStream`+`LZ4_resetStream_fast`+`_continue` | prefix mode, chunk sizes random | [x] |
| 15 | `LZ4_loadDict`+`_continue` | dictSize ∈ {0,1,3,4,64,4096,65535,65536,70000}, extDict mode | [x] |
| 16 | `LZ4_loadDictSlow`+`_continue` | same dictSize sweep (slow fill changes hash table) | [x] |
| 17 | `LZ4_attach_dictionary`+`_continue` | usingDictCtx, inputSize <=4KB and >4KB (table-copy branch) | [x] |
| 18 | `LZ4_attach_dictionary(NULL)` | detach; also attach empty dict (dictSize==0 → NULL) | [x] |
| 19 | `LZ4_saveDict`+`_continue` | maxDictSize ∈ {0,1,4,1000,65536,70000}, then continue | [x] |
| 20 | `LZ4_compress_fast_continue` | overlapping src/dict window (sourceEnd inside dict) | [x] |
| 21 | `LZ4_compress_fast_continue` | tiny dict (<4) invalidation branch | [x] |
| 22 | `LZ4_compress_forceExtDict` | forced ext dict, dictSmall and noDictIssue | [x] |
| 23 | `LZ4_decompress_safe` | valid frames from every compress row, dstCapacity exact | [x] |
| 24 | `LZ4_decompress_safe` | dstCapacity > exact (slack), and exact-1 | [x] |
| 25 | `LZ4_decompress_safe_partial` | targetOutputSize ∈ {0,1,half,exact,exact+1}, dstCapacity variants | [x] |
| 26 | `LZ4_decompress_fast` | originalSize exact (legacy unsafe path) | [x] |
| 27 | `LZ4_createStreamDecode`+`LZ4_decompress_safe_continue` | linked blocks, contiguous output buffer | [x] |
| 28 | `LZ4_setStreamDecode`+`_safe_continue` | external dict set explicitly, dictSize sweep | [x] |
| 29 | `LZ4_decompress_fast_continue` | linked blocks legacy path | [x] |
| 30 | `LZ4_decompress_safe_usingDict` | dictSize ∈ {0,4,1000,65536}, prefix and separate-buffer | [x] |
| 31 | `LZ4_decompress_fast_usingDict` | same sweep | [x] |
| 32 | `LZ4_decompress_safe_partial_usingDict` | targetOutputSize sweep × dictSize sweep | [x] |
| 33 | `LZ4_decompress_safe_withPrefix64k`, `LZ4_decompress_fast_withPrefix64k` | 64KB prefix in same buffer | [x] |
| 34 | `LZ4_decoderRingBufferSize` | maxBlockSize ∈ {-1,0,1,15,16,17,65536,4MB,>max} | [x] |
| 35 | `LZ4_compress`, `LZ4_compress_limitedOutput` (deprecated) | srcSize regimes | [x] |
| 36 | `LZ4_compress_withState`, `LZ4_compress_limitedOutput_withState` | srcSize regimes | [x] |
| 37 | `LZ4_compress_continue`, `LZ4_compress_limitedOutput_continue` | streaming | [x] |
| 38 | `LZ4_uncompress`, `LZ4_uncompress_unknownOutputSize` | valid + short output | [x] |
| 39 | `LZ4_create`+`LZ4_slideInputBuffer`+`LZ4_resetStreamState` | legacy streaming | [x] |
| 40 | `LZ4_compress_HC` | level sweep {-1,0,1,2,3,4,6,9,10,11,12,13,99} × srcSize regimes | [x] |
| 41 | `LZ4_compress_HC` | dstCapacity = bound, exact-1, small (limitedOutput) | [x] |
| 42 | `LZ4_compress_HC_extStateHC` | level sweep, fresh state each call | [x] |
| 43 | `LZ4_compress_HC_extStateHC_fastReset` | level sweep, reused state | [x] |
| 44 | `LZ4_compress_HC_destSize` | level sweep × targetDstSize ∈ {0,1,8,src/4,src/2,bound} | [x] |
| 45 | `LZ4_createStreamHC`+`LZ4_resetStreamHC`+`_continue` | level sweep, multi-chunk | [x] |
| 46 | `LZ4_createStreamHC`+`LZ4_resetStreamHC_fast`+`_continue` | level sweep, multi-chunk | [x] |
| 47 | `LZ4_setCompressionLevel` mid-stream | level changed between `_continue` calls | [x] |
| 48 | `LZ4_favorDecompressionSpeed` | 0/1 × level ∈ {9,10,11,12} | [x] |
| 49 | `LZ4_loadDictHC`+`_continue` | dictSize sweep {0,1,3,4,64,4096,65536,70000} × level sweep | [x] |
| 50 | `LZ4_attach_HC_dictionary`+`_continue` | CDict-style attach × level sweep | [x] |
| 51 | `LZ4_saveDictHC`+`_continue` | maxDictSize sweep | [x] |
| 52 | `LZ4_compress_HC_continue_destSize` | level sweep × targetDstSize sweep, multi-chunk | [x] |
| 53 | `LZ4_initStreamHC` | user buffer, then compress at each level | [x] |
| 54 | `LZ4_compressHC*` deprecated family (8 fns) | level sweep, limited/unlimited output | [x] |
| 55 | `LZ4_createHC`+`LZ4_slideInputBufferHC`+`LZ4_compressHC2_continue` | legacy HC streaming | [x] |
| 56 | `LZ4_resetStreamStateHC` | user buffer init then compress | [x] |
| 57 | `LZ4F_getBlockSize` | blockSizeID ∈ {0,1,2,3,4,5,6,7,8,255,-1} | [x] |
| 58 | `LZ4F_compressFrameBound` | srcSize sweep × blockSizeID sweep × autoFlush × blockMode | [x] |
| 59 | `LZ4F_compressBound` | srcSize ∈ {0,1,blk-1,blk,blk+1,2*blk} × blockSizeID × autoFlush | [x] |
| 60 | `LZ4F_compressFrame` | prefs = NULL (defaults) × srcSize regimes | [x] |
| 61 | `LZ4F_compressFrame` | blockSizeID ∈ {0,4,5,6,7} × blockMode {0,1} × srcSize regimes | [x] |
| 62 | `LZ4F_compressFrame` | contentChecksum {0,1} × blockChecksum {0,1} × blockSizeID sweep | [x] |
| 63 | `LZ4F_compressFrame` | contentSize {0, exact} × dictID {0, 0xDEADBEEF} | [x] |
| 64 | `LZ4F_compressFrame` | compressionLevel ∈ {-5,-1,0,1,3,9,10,11,12,20} | [x] |
| 65 | `LZ4F_compressFrame` | autoFlush {0,1} × favorDecSpeed {0,1} × level {0,9,12} | [x] |
| 66 | `LZ4F_compressFrame_usingCDict` | CDict dictSize ∈ {1,64,4096,65536,70000} × level {0,9,12} × blockMode | [x] |
| 67 | streaming `compressBegin`/`Update`/`End` | full prefs cross-product, single Update | [x] |
| 68 | streaming | many `compressUpdate` calls, random chunk sizes, blockLinked | [x] |
| 69 | streaming | many `compressUpdate` calls, blockIndependent | [x] |
| 70 | streaming | `LZ4F_flush` interleaved between updates (tmpInSize>0 and ==0) | [x] |
| 70a| `LZ4F_uncompressedUpdate` | blockIndependent (only supported mode) × chunk sweep × contentChecksum/blockChecksum | [x] |
| 70b| `LZ4F_uncompressedUpdate` | interleaved with `LZ4F_compressUpdate` (forces a buffered flush) | [x] |
| 70c| `LZ4F_uncompressedUpdate` | blockLinked (unsupported mode — whatever the C does must be matched) | [x] |
| 71 | streaming | autoFlush=1 (no tmp buffering) × chunk sweep | [x] |
| 72 | streaming | `compressOptions.stableSrc` = 0 and 1 × chunk sweep | [x] |
| 73 | streaming | reuse same cctx for a 2nd frame with different prefs (ctx type switch fast↔HC) | [x] |
| 74 | `LZ4F_compressBegin_usingDict` | dictSize sweep × level {0,9,12} × blockMode | [x] |
| 75 | `LZ4F_compressBegin_usingDictOnce` | dictSize sweep, then 2nd frame (dict not reused) | [x] |
| 76 | `LZ4F_compressBegin_usingCDict` | CDict × level sweep × blockMode | [x] |
| 77 | `LZ4F_compressBegin_internal` | direct call, dictBuffer + cdict combinations | [x] |
| 78 | `LZ4F_createCompressionContext_advanced` | custom malloc/calloc/free via CustomMem | [x] |
| 79 | `LZ4F_createDecompressionContext_advanced` | custom mem | [x] |
| 80 | `LZ4F_createCDict_advanced` | custom mem × dictSize sweep | [x] |
| 81 | `LZ4F_headerSize` | every FLG combination (contentSize/dictID flags) | [x] |
| 82 | `LZ4F_getFrameInfo` | before decode, exact header size, oversized src | [x] |
| 83 | `LZ4F_getFrameInfo` | after decode started (dStage > storeFrameHeader) | [x] |
| 84 | `LZ4F_decompress` | whole frame in one call, every prefs combo | [x] |
| 85 | `LZ4F_decompress` | 1 byte at a time (exercises tmpIn/tmpOut store stages) | [x] |
| 86 | `LZ4F_decompress` | random chunk sizes, dst also fed in random chunks | [x] |
| 87 | `LZ4F_decompress` | dstCapacity smaller than block (partial output, tmpOut path) | [x] |
| 88 | `LZ4F_decompress` | `decompressOptions.stableDst` 0/1 | [x] |
| 89 | `LZ4F_decompress` | `decompressOptions.skipChecksums` 0/1 × content/block checksum on | [x] |
| 90 | `LZ4F_decompress` | skippable frame (magic 0x184D2A50..5F), sizes 0 and large | [x] |
| 91 | `LZ4F_decompress` | multiple concatenated frames on one dctx | [x] |
| 92 | `LZ4F_decompress_usingDict` | dict sweep × level × blockMode, chunked feeding | [x] |
| 93 | `LZ4F_resetDecompressionContext` | mid-frame reset then re-decode | [x] |
| 94 | `LZ4F_isError`/`getErrorName`/`getErrorCode` | codes 0..-(maxCode) and out-of-range | [x] |
| 95 | `LZ4F_getVersion`, `LZ4F_compressionLevel_max` | — | [x] |
| 96 | `LZ4F_writeOpen`/`write`/`writeClose` round-trip | prefs = NULL and each prefs combo × write sizes {0,1,7,blk-1,blk,blk+1,3*blk} | [x] |
| 97 | `LZ4F_readOpen`/`read`/`readClose` round-trip | read sizes {0,1,7,blk-1,blk,blk+1,huge} × frames from row 96 | [x] |
| 98 | file API | write via C, read via Rust and vice-versa (byte-identical file) | [x] |
| 99 | `LZ4_XXH32` | length ∈ {0,1,2,3,4,5,12,15,16,17,31,32,33,63,64,65,1000,4096} × seed sweep | [x] |
| 100| `LZ4_XXH64` | same length sweep × 64-bit seed sweep | [x] |
| 101| `LZ4_XXH32` streaming | reset/update*/digest, 1-byte chunks and random chunks | [x] |
| 102| `LZ4_XXH64` streaming | same | [x] |
| 103| `LZ4_XXH32_copyState` / `LZ4_XXH64_copyState` | copy mid-stream, continue both | [x] |
| 104| canonical round-trip | `canonicalFromHash` + `hashFromCanonical` for 32 and 64 | [x] |
| 105| `LZ4_XXH_versionNumber` | — | [x] |
| 106| xxhash | unaligned input pointers (offset 1..7) × length sweep | [x] |
| 107| cross-module | compress with C, decompress with Rust and vice-versa (all modules) | [x] |
| 108| cross-module | `LZ4F` frame → `LZ4_decompress_safe` on inner block (blockIndependent) | [x] |

## Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only
configuration is the default (empty) feature set. Verified with
`cargo metadata` — see `scripts/features.sh`. Phase D's "every feature
combination" therefore reduces to one combination, which is the one tested.

## Where each row is tested

| rows | test file / test |
|------|------------------|
| 1, 11 | `tests/lz4_block.rs::r001_compress_bound`, `r011_sizeof_and_versions` |
| 2–5 | `tests/lz4_block.rs::r002_compress_default_small`, `r003_compress_default_byU16`, `r004_compress_default_byU32`, `e003_compress_default_bad_sizes` |
| 6–8 | `tests/lz4_block.rs::r006_compress_fast_accel`, `r007_compress_fast_extState`, `r008_compress_fast_extState_fastReset` |
| 9–10 | `tests/lz4_block.rs::r009_compress_destSize` |
| 12 | `tests/lz4_block.rs::r012_initStream_then_compress`, `e028_initStream_guards` |
| 13–14 | `tests/lz4_block.rs::r013_r014_prefix_streaming` |
| 15–16 | `tests/lz4_block.rs::r015_r016_loadDict_extDict` |
| 17–18 | `tests/lz4_block.rs::r017_r018_attach_dictionary` |
| 19 | `tests/lz4_block.rs::r019_saveDict` |
| 20–21 | `tests/lz4_block.rs::r020_r021_overlapping_and_tiny_dict` |
| 22 | `tests/lz4_block.rs::r022_compress_forceExtDict` |
| 23–24 | `tests/lz4_block.rs::r023_decompress_safe_roundtrip` |
| 25 | `tests/lz4_block.rs::r025_decompress_safe_partial` |
| 26, 38 | `tests/lz4_block.rs::r026_decompress_fast` |
| 27, 29 | `tests/lz4_block.rs::r027_r029_decode_continue` |
| 28 | `tests/lz4_block.rs::r028_setStreamDecode` |
| 30–32 | `tests/lz4_block.rs::r030_r032_usingDict_decoders` |
| 33 | `tests/lz4_block.rs::r033_withPrefix64k` |
| 34 | `tests/lz4_block.rs::r034_decoderRingBufferSize` |
| 35–37 | `tests/lz4_block.rs::r035_r037_deprecated_block_api` |
| 39 | `tests/lz4_block.rs::r039_legacy_create_and_resetStreamState` |
| 40–41 | `tests/lz4hc.rs::r040_compress_HC_levels`, `e050_compress_HC_bad_sizes_and_tight_dst` |
| 42–43 | `tests/lz4hc.rs::r042_extStateHC`, `r043_extStateHC_fastReset` |
| 44 | `tests/lz4hc.rs::r044_HC_destSize` |
| 45–46 | `tests/lz4hc.rs::r045_r046_HC_streaming` |
| 47 | `tests/lz4hc.rs::r047_setCompressionLevel_midstream` |
| 48 | `tests/lz4hc.rs::r048_favorDecompressionSpeed` |
| 49 | `tests/lz4hc.rs::r049_loadDictHC` |
| 50 | `tests/lz4hc.rs::r050_attach_HC_dictionary` |
| 51 | `tests/lz4hc.rs::r051_saveDictHC` |
| 52 | `tests/lz4hc.rs::r052_HC_continue_destSize`, `e070_HC_continue_tight_dst` |
| 53 | `tests/lz4hc.rs::r053_initStreamHC_then_compress` |
| 54 | `tests/lz4hc.rs::r054_deprecated_HC_oneshot` |
| 55 | `tests/lz4hc.rs::r055_legacy_HC_streaming` |
| 56 | `tests/lz4hc.rs::r056_resetStreamStateHC` |
| 57 | `tests/lz4frame_valid.rs::r057_getBlockSize` |
| 58–59 | `tests/lz4frame_valid.rs::r058_r059_bounds` |
| 60 | `tests/lz4frame_valid.rs::r060_compressFrame_default_prefs` |
| 61–62, 64–65 | `tests/lz4frame_valid.rs::r061_r065_compressFrame_matrix`, `r061_random_sizes` |
| 63 | `tests/lz4frame_valid.rs::r063_contentSize_and_dictID` |
| 66 | `tests/lz4frame_valid.rs::r066_compressFrame_usingCDict` |
| 67 | `tests/lz4frame_valid.rs::r067_streaming_single_update` |
| 68–69 | `tests/lz4frame_valid.rs::r068_r069_streaming_many_updates` |
| 70 | `tests/lz4frame_valid.rs::r070_streaming_with_flush` |
| 70a–70c | `tests/lz4frame_valid.rs::r070a_r070c_uncompressedUpdate` |
| 71–72 | `tests/lz4frame_valid.rs::r071_r072_autoflush_and_stableSrc` |
| 73 | `tests/lz4frame_valid.rs::r073_cctx_reuse_switches_ctx_type` |
| 74, 76–77 | `tests/lz4frame_valid.rs::r074_r077_begin_using_dicts` |
| 75 | `tests/lz4frame_valid.rs::r075_dictOnce_second_frame` |
| 78–80 | `tests/lz4frame_valid.rs::r078_r080_custom_mem` |
| 81 | `tests/lz4frame_valid.rs::r081_headerSize` |
| 82–83 | `tests/lz4frame_valid.rs::r082_r083_getFrameInfo` |
| 84 | `tests/lz4frame_valid.rs::r084_decompress_whole_frame` |
| 85 | `tests/lz4frame_valid.rs::r085_decompress_byte_at_a_time` |
| 86–87 | `tests/lz4frame_valid.rs::r086_r087_decompress_random_chunks` |
| 88–89 | `tests/lz4frame_valid.rs::r088_r089_decompress_options` |
| 90 | `tests/lz4frame_valid.rs::r090_skippable_frames` |
| 91 | `tests/lz4frame_valid.rs::r091_concatenated_frames` |
| 92 | `tests/lz4frame_valid.rs::r092_decompress_usingDict` |
| 93 | `tests/lz4frame_valid.rs::r093_reset_dctx` |
| 94 | `tests/lz4frame_errors.rs::e123_error_helpers` |
| 95 | `tests/lz4frame_valid.rs::r095_version_and_level_max` |
| 96 | `tests/lz4file.rs::r096_write_roundtrip`, `e134_write_oversized_chunks` |
| 97 | `tests/lz4file.rs::r097_read_roundtrip` |
| 98 | `tests/lz4file.rs::r098_cross_impl_files` |
| 99–100 | `tests/xxhash.rs::r099_r100_oneshot` |
| 101–102 | `tests/xxhash.rs::r101_r102_streaming` |
| 103 | `tests/xxhash.rs::r103_copyState` |
| 104 | `tests/xxhash.rs::r104_canonical` |
| 105 | `tests/xxhash.rs::r105_version` |
| 106 | `tests/xxhash.rs::r106_unaligned` |
| 107 | `tests/cross_impl.rs` (all four tests) |
| 108 | `tests/lz4frame_valid.rs::r108_frame_block_via_block_api` |
