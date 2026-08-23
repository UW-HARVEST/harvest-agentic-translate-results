# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from the branches the C actually takes. Axes were found by
grepping the public headers for every runtime option/flag, and the `.c` files for
every `if`/`switch` on those flags plus every input-shape special case.

## Axes the C branches on

**lz4.c block API**
- `tableType`: `byU16` iff `inputSize < LZ4_64Klimit` (65547), else `byU32`
  (`byPtr` is 32-bit-only, unreachable here) — lz4.c:1389/1396
- `dict` directive: `noDict` | `withPrefix64k` | `usingExtDict` | `usingDictCtx`
- `dictIssue`: `noDictIssue` | `dictSmall` (`dictSize < 64KB && dictSize < currentOffset`, lz4.c:1747)
- `limitedOutput`: `notLimited` (`dstCapacity >= LZ4_compressBound`) | `limitedOutput` | `fillOutput` (destSize)
- `acceleration`: 1 (default) | 2..65536 | 65537 (`LZ4_ACCELERATION_MAX`)
- input SHAPE: 0 | 1 | 12 (`< LZ4_minLength` 13) | 13 | 64 | 1024 | 65535
  (`LZ4_DISTANCE_MAX`) | 65536 | 65546/65547 (`LZ4_64Klimit` boundary) | 200000;
  incompressible (random) | highly compressible (runs) | mixed | long-match
- dictCtx bulk-copy threshold: `inputSize > 4096` (lz4.c:1762)

**lz4hc.c**
- `cLevel` classes: 1-2 → `lz4mid` | 3-9 → HC chain | 10-12 → optimal parser
  (`LZ4HC_CLEVEL_OPT_MIN`); `LZ4HC_CLEVEL_DEFAULT` 9, `MAX` 12
- `favorDecSpeed`: 0 | 1 (only consulted at levels >= 10)
- `limit`: `notLimited` | `limitedOutput` | `fillOutput` (destSize entry points)
- dict source: none | `LZ4_loadDictHC` | `LZ4_attach_HC_dictionary` | non-contiguous
  `continue` (`LZ4HC_setExternalDict`)
- state-compatibility straddle across the `lz4mid` boundary (lz4hc.c:1434)

**lz4frame.c** (`LZ4F_preferences_t` / `LZ4F_frameInfo_t` / `LZ4F_compressOptions_t` /
`LZ4F_decompressOptions_t`)
- `blockSizeID`: 0 (`default`→64KB) | 4 | 5 | 6 | 7
- `blockMode`: `blockLinked` (0) | `blockIndependent` (1)
- `contentChecksumFlag`: 0 | 1 ; `blockChecksumFlag`: 0 | 1
- `contentSize`: 0 (absent) | exact | (mismatch → error, see ERRORS.md)
- `dictID`: 0 | non-zero ; `frameType`: `LZ4F_frame` | `LZ4F_skippableFrame`
- `compressionLevel`: <2 → fast `LZ4_stream_t` ctx | >=2 → HC ctx (lz4frame.c:705)
- `autoFlush`: 0 | 1 ; `favorDecSpeed`: 0 | 1 ; `stableSrc`: 0 | 1 ; `stableDst`: 0 | 1
- `skipChecksums`: 0 | 1
- API shape: one-shot `LZ4F_compressFrame` | `compressBegin`/`compressUpdate`*/
  `flush`/`compressEnd` | `uncompressedUpdate` (stored blocks) | mixed
  `compressUpdate`+`uncompressedUpdate`
- dict shape: none | `compressBegin_usingDict` | `_usingDictOnce` | `_usingCDict` |
  `compressFrame_usingCDict`
- decode shape: one-shot | chunked src (chunk sizes crossing the 4/7/11/15/19-byte
  header, the 4-byte block header, the 4-byte checksums) | chunked dst | both

**lz4file.c**: `writeOpen` prefs (all frame axes) × write chunk size;
`readOpen` × read chunk size.

**xxhash.c**: `XXH32` (16-byte buffer) / `XXH64` (32-byte buffer); one-shot vs
streaming; chunk sizes crossing the internal buffer boundary; seed; length shape.

## Rows

Each row is exercised with MANY randomized inputs (fixed seed, `tests/common/mod.rs`
`Rng`), comparing the C `.so` and Rust `.so` byte-for-byte.

| # | entry point(s) | configuration (options set + input shape) | [x] |
|---|----------------|--------------------------------------------|-----|
| **xxhash** | | | |
| 1 | `LZ4_XXH32` | one-shot, len 0..300 sweep × seeds {0,1,0x9E3779B1,UINT32_MAX} | [x] |
| 2 | `LZ4_XXH64` | one-shot, len 0..300 sweep × seeds {0,1,0x9E3779B185EBCA87,UINT64_MAX} | [x] |
| 3 | `LZ4_XXH32` | one-shot, large len {1KB,4KB,64KB,100000} random | [x] |
| 4 | `LZ4_XXH64` | one-shot, large len {1KB,4KB,64KB,100000} random | [x] |
| 5 | `LZ4_XXH32_reset/update/digest` | streaming, fixed chunk sizes 1..40 (crosses the 16-byte buffer) | [x] |
| 6 | `LZ4_XXH64_reset/update/digest` | streaming, fixed chunk sizes 1..40 (crosses the 32-byte buffer) | [x] |
| 7 | `LZ4_XXH32/64_*` | streaming, RANDOM chunk sizes, multiple `digest()` calls mid-stream | [x] |
| 8 | `LZ4_XXH32/64_copyState` | copy mid-stream, continue both copies, digests must match | [x] |
| 9 | `LZ4_XXH32/64_createState/freeState` | alloc/free lifecycle | [x] |
| 10 | `LZ4_XXH32/64_canonicalFromHash` + `_hashFromCanonical` | round trip {0,1,MAX,random} | [x] |
| 11 | `LZ4_XXH_versionNumber` | constant | [x] |
| **lz4 block compress — one-shot** | | | |
| 12 | `LZ4_compress_default` | `notLimited`, `byU16` (srcSize < 65547), random/incompressible | [x] |
| 13 | `LZ4_compress_default` | `notLimited`, `byU16`, highly compressible (runs, periodic) | [x] |
| 14 | `LZ4_compress_default` | `notLimited`, `byU32` (srcSize >= 65547), random | [x] |
| 15 | `LZ4_compress_default` | `notLimited`, `byU32`, highly compressible | [x] |
| 16 | `LZ4_compress_default` | `byU16`/`byU32` boundary sweep: srcSize in {65535,65536,65546,65547,65548} | [x] |
| 17 | `LZ4_compress_default` | tiny shapes: srcSize in {0,1,2,11,12,13,14,63,64,65} | [x] |
| 18 | `LZ4_compress_default` | `limitedOutput`: `dstCapacity` swept from 1 to bound | [x] |
| 19 | `LZ4_compress_fast` | acceleration {1,2,3,7,17,64,1000,65536,65537} × both tableTypes | [x] |
| 20 | `LZ4_compress_fast` | acceleration sweep × `limitedOutput` dstCapacity | [x] |
| 21 | `LZ4_compress_fast_extState` | caller-owned state, acceleration sweep, both tableTypes | [x] |
| 22 | `LZ4_compress_fast_extState_fastReset` | fresh state, then REUSED state (fast-reset path) × acceleration | [x] |
| 23 | `LZ4_compress_fast_extState_fastReset` | reused state across many calls of differing sizes (currentOffset growth) | [x] |
| 24 | `LZ4_compress_destSize` | `fillOutput`, targetDstSize swept 1..bound, both tableTypes | [x] |
| 25 | `LZ4_compress_destSize` | `fillOutput`, targetDstSize exactly at bound (all src consumed) | [x] |
| 26 | `LZ4_compress_destSize_extState` | same as rows 24-25 with a caller-owned state | [x] |
| 27 | `LZ4_compress` (deprecated) | `notLimited` wrapper, both tableTypes | [x] |
| 28 | `LZ4_compress_limitedOutput` (deprecated) | `limitedOutput` wrapper, dstCapacity sweep | [x] |
| 29 | `LZ4_compress_withState` / `_limitedOutput_withState` | deprecated ext-state wrappers | [x] |
| 30 | `LZ4_compressBound` / `LZ4_sizeofState` / `LZ4_sizeofStreamState` | size sweep incl. boundaries | [x] |
| 31 | `LZ4_versionNumber` / `LZ4_versionString` | constants | [x] |
| **lz4 block compress — streaming** | | | |
| 32 | `LZ4_createStream`/`_compress_fast_continue`/`_freeStream` | contiguous prefix chain, uniform block sizes | [x] |
| 33 | `LZ4_compress_fast_continue` | contiguous prefix chain, RANDOM block sizes, many blocks | [x] |
| 34 | `LZ4_compress_fast_continue` | prefix `dictSmall` path (`dictSize < 64KB && < currentOffset`) | [x] |
| 35 | `LZ4_compress_fast_continue` | prefix chain crossing 64 KB total (withPrefix64k) | [x] |
| 36 | `LZ4_compress_fast_continue` | non-contiguous blocks ⇒ `usingExtDict` | [x] |
| 37 | `LZ4_compress_fast_continue` | ring buffer (wrap-around dst/src), `LZ4_decoderRingBufferSize` | [x] |
| 38 | `LZ4_loadDict` + `_compress_fast_continue` | dictSize {8,64,1024,65535,65536,70000} × block sizes | [x] |
| 39 | `LZ4_loadDictSlow` + `_compress_fast_continue` | same dictSize sweep (different table-fill path) | [x] |
| 40 | `LZ4_attach_dictionary` + `_compress_fast_continue` | `usingDictCtx`, inputSize <= 4096 (no bulk copy) | [x] |
| 41 | `LZ4_attach_dictionary` + `_compress_fast_continue` | `usingDictCtx`, inputSize > 4096 (bulk table copy, lz4.c:1762) | [x] |
| 42 | `LZ4_saveDict` | after a prefix chain, dictSize {0,4,1024,65536,70000} | [x] |
| 43 | `LZ4_resetStream` / `_resetStream_fast` / `_initStream` | reset then reuse; output must equal a fresh stream | [x] |
| 44 | `LZ4_compress_continue` / `_limitedOutput_continue` (deprecated) | `LZ4_create`-based chain | [x] |
| 45 | `LZ4_compress_forceExtDict` | forced extDict with a dict of {4,1024,65536} | [x] |
| 46 | `LZ4_slideInputBuffer` / `LZ4_resetStreamState` / `LZ4_create` / `LZ4_freeStream` | deprecated lifecycle | [x] |
| 47 | `LZ4_loadDict` + `_compress_fast_continue` | `LZ4_renormDictT` rescale path (currentOffset near 2^31) | [x] |
| **lz4 block decompress** | | | |
| 48 | `LZ4_decompress_safe` | round trip of every row 12-31 output, exact dstCapacity | [x] |
| 49 | `LZ4_decompress_safe` | dstCapacity LARGER than decoded size | [x] |
| 50 | `LZ4_decompress_safe_partial` | targetOutputSize swept 0..decodedSize, dstCapacity == decodedSize | [x] |
| 51 | `LZ4_decompress_safe_partial` | targetOutputSize == dstCapacity < decodedSize (truncating) | [x] |
| 52 | `LZ4_decompress_safe_partial` | targetOutputSize > dstCapacity | [x] |
| 53 | `LZ4_decompress_fast` | round trip with exact `originalSize` | [x] |
| 54 | `LZ4_uncompress` / `LZ4_uncompress_unknownOutputSize` | deprecated wrappers, round trip | [x] |
| 55 | `LZ4_decompress_safe_withPrefix64k` | 64 KB contiguous prefix | [x] |
| 56 | `LZ4_decompress_fast_withPrefix64k` | 64 KB contiguous prefix | [x] |
| 57 | `LZ4_decompress_safe_usingDict` | dictSize {0,4,1024,65535,65536,70000} — crosses the `checkOffset` cutoff at 65536 | [x] |
| 58 | `LZ4_decompress_safe_usingDict` | contiguous dict (`dictStart+dictSize == dest`) vs separate buffer | [x] |
| 59 | `LZ4_decompress_safe_partial_usingDict` | dict × targetOutputSize sweep | [x] |
| 60 | `LZ4_decompress_fast_usingDict` | dict × contiguous/non-contiguous | [x] |
| 61 | `LZ4_decompress_safe_forceExtDict` | forced extDict, dictSize sweep | [x] |
| 62 | `LZ4_decompress_safe_partial_forceExtDict` | forced extDict × targetOutputSize sweep | [x] |
| 63 | `LZ4_setStreamDecode` + `_decompress_safe_continue` | linked-block stream, uniform block sizes | [x] |
| 64 | `LZ4_decompress_safe_continue` | linked-block stream, RANDOM block sizes, many blocks | [x] |
| 65 | `LZ4_decompress_safe_continue` | separate output buffer per block ⇒ `forceExtDict` promotion (lz4.c:2656) | [x] |
| 66 | `LZ4_decompress_safe_continue` | ring buffer sized by `LZ4_decoderRingBufferSize`, wrap-around | [x] |
| 67 | `LZ4_decompress_safe_continue` | small-prefix (`prefixSize < 65535`) and `doubleDict` paths (lz4.c:2647/2651) | [x] |
| 68 | `LZ4_decompress_fast_continue` | linked-block stream, uniform + random block sizes | [x] |
| 69 | `LZ4_createStreamDecode` / `_freeStreamDecode` / `LZ4_decoderRingBufferSize` | lifecycle + size sweep | [x] |
| **lz4hc compress** | | | |
| 70 | `LZ4_compress_HC` | cLevel 1..12 sweep × `notLimited` × random src | [x] |
| 71 | `LZ4_compress_HC` | cLevel 1..12 sweep × `notLimited` × highly compressible src | [x] |
| 72 | `LZ4_compress_HC` | cLevel 1..12 × input shapes {0,1,12,13,64,1024,65535,65536,65547,200000} | [x] |
| 73 | `LZ4_compress_HC` | cLevel 1..12 × `limitedOutput` dstCapacity sweep | [x] |
| 74 | `LZ4_compress_HC` | cLevel 1,2 (`lz4mid`) specifically, all shapes | [x] |
| 75 | `LZ4_compress_HC` | cLevel 3..9 (HC chain) specifically, all shapes | [x] |
| 76 | `LZ4_compress_HC` | cLevel 10,11,12 (optimal parser) specifically, all shapes | [x] |
| 77 | `LZ4_compress_HC_extStateHC` | caller-owned state × cLevel sweep | [x] |
| 78 | `LZ4_compress_HC_extStateHC_fastReset` | fresh + REUSED state × cLevel sweep | [x] |
| 79 | `LZ4_compress_HC_destSize` | `fillOutput`, targetDstSize sweep × cLevel 1..12 | [x] |
| 80 | `LZ4_compress_HC_destSize` | cLevel 1-2 salvage path (lz4hc.c:756) and 3-9 (1349) and 10-12 (2104) | [x] |
| 81 | `LZ4_favorDecompressionSpeed` + `LZ4_compress_HC_continue` | favor 0 vs 1 × cLevel 10,11,12 | [x] |
| 82 | `LZ4_favorDecompressionSpeed` + `LZ4_compress_HC_continue` | favor 0 vs 1 × cLevel 1..9 (ignored) | [x] |
| 83 | `LZ4_setCompressionLevel` + `LZ4_compress_HC_continue` | level changed BETWEEN blocks (1↔12, straddling lz4mid) | [x] |
| 84 | `LZ4_createStreamHC`/`_compress_HC_continue`/`_freeStreamHC` | contiguous chain × cLevel sweep, uniform blocks | [x] |
| 85 | `LZ4_compress_HC_continue` | contiguous chain, RANDOM block sizes × cLevel sweep | [x] |
| 86 | `LZ4_compress_HC_continue` | non-contiguous blocks ⇒ `LZ4HC_setExternalDict` × cLevel sweep | [x] |
| 87 | `LZ4_compress_HC_continue` | chain crossing 64 KB and 2 GB-index reload (lz4hc.c:1695) | [x] |
| 88 | `LZ4_loadDictHC` + `_compress_HC_continue` | dictSize {0,3,4,8,9,1024,65535,65536,70000} × cLevel sweep | [x] |
| 89 | `LZ4_attach_HC_dictionary` + `_compress_HC_continue` | compatible levels (both <=2 or both >=3), srcSize <=4096 and >4096 | [x] |
| 90 | `LZ4_attach_HC_dictionary` + `_compress_HC_continue` | INCOMPATIBLE levels straddling lz4mid (lz4hc.c:1434) | [x] |
| 91 | `LZ4_saveDictHC` | after a chain, dictSize {0,3,4,1024,65536,70000} × cLevel | [x] |
| 92 | `LZ4_resetStreamHC` / `_resetStreamHC_fast` / `_initStreamHC` | reset then reuse; must equal a fresh stream | [x] |
| 93 | `LZ4_compress_HC_continue_destSize` | `fillOutput` chain, targetDestSize sweep × cLevel | [x] |
| 94 | `LZ4_compressHC` / `_compressHC2` / `_compressHC_limitedOutput` / `_compressHC2_limitedOutput` | deprecated one-shots × cLevel | [x] |
| 95 | `LZ4_compressHC_withStateHC` / `_compressHC2_withStateHC` / `..._limitedOutput_withStateHC` | deprecated ext-state × cLevel | [x] |
| 96 | `LZ4_compressHC_continue` / `_compressHC_limitedOutput_continue` | deprecated chain via `LZ4_createHC` | [x] |
| 97 | `LZ4_createHC` / `_freeHC` / `_slideInputBufferHC` / `_resetStreamStateHC` / `_sizeofStateHC` | deprecated lifecycle | [x] |
| 98 | `LZ4HC_searchExtDict` (exported internal) | called via the extDict chain paths of rows 86/88 | [x] |
| 99 | HC output → `LZ4_decompress_safe` | every HC row round-trips through the block decoder | [x] |
| **lz4frame — compression** | | | |
| 100 | `LZ4F_compressFrame` | `prefsPtr == NULL` (all defaults) × src shapes | [x] |
| 101 | `LZ4F_compressFrame` | blockSizeID {0,4,5,6,7} × blockMode {linked,independent} | [x] |
| 102 | `LZ4F_compressFrame` | contentChecksumFlag {0,1} × blockChecksumFlag {0,1} (full 2×2) | [x] |
| 103 | `LZ4F_compressFrame` | contentSize {0, exact} × dictID {0, 0xDEADBEEF} | [x] |
| 104 | `LZ4F_compressFrame` | compressionLevel {-5,-1,0,1,2,3,6,9,10,11,12,13,100} (fast-ctx vs HC-ctx split at 2) | [x] |
| 105 | `LZ4F_compressFrame` | favorDecSpeed {0,1} × compressionLevel {1,9,12} | [x] |
| 106 | `LZ4F_compressFrame` | autoFlush {0,1} × blockSizeID {4,7} | [x] |
| 107 | `LZ4F_compressFrame` | src shapes {0,1,64,65535,65536,262144,1048576+1} vs blockSizeID (crosses block boundaries) | [x] |
| 108 | `LZ4F_compressFrame` | full cross-product sweep: blockSizeID × blockMode × contentChecksum × blockChecksum × level {1,9,12} | [x] |
| 109 | `LZ4F_compressBegin`/`compressUpdate`/`compressEnd` | single `compressUpdate`, all prefs axes as in row 108 | [x] |
| 110 | `LZ4F_compressBegin`/`compressUpdate`*/`compressEnd` | MANY `compressUpdate` calls, uniform chunk sizes | [x] |
| 111 | `LZ4F_compressBegin`/`compressUpdate`*/`compressEnd` | MANY `compressUpdate` calls, RANDOM chunk sizes (tmpBuff accumulation) | [x] |
| 112 | `LZ4F_compressBegin`/`compressUpdate`*/`flush`/`compressEnd` | explicit `LZ4F_flush` interleaved at random points | [x] |
| 113 | `LZ4F_compressUpdate` | `LZ4F_compressOptions_t.stableSrc` {0,1} | [x] |
| 114 | `LZ4F_uncompressedUpdate` | stored blocks, blockIndependent, chunk-size sweep | [x] |
| 115 | `LZ4F_uncompressedUpdate` + `LZ4F_compressUpdate` | MIXED in one frame (forces the internal `flush`, lz4frame.c:1013) | [x] |
| 116 | `LZ4F_compressBegin_usingDict` | dictSize {1,64,1024,65535,65536,70000} × prefs axes | [x] |
| 117 | `LZ4F_compressBegin_usingDictOnce` | same dict sweep (dict used for the first block only) | [x] |
| 118 | `LZ4F_createCDict` + `LZ4F_compressBegin_usingCDict` | dict sweep × level {1,9,12} × blockMode | [x] |
| 119 | `LZ4F_createCDict_advanced` + `LZ4F_compressFrame_usingCDict` | `LZ4F_defaultCMem`, dict sweep | [x] |
| 120 | `LZ4F_createCompressionContext_advanced` | `LZ4F_defaultCMem` + reused cctx across MANY frames | [x] |
| 121 | `LZ4F_compressBound` / `LZ4F_compressFrameBound` | srcSize sweep × all valid blockSizeID × autoFlush {0,1} × blockChecksum | [x] |
| 122 | `LZ4F_getBlockSize` | blockSizeID {0,4,5,6,7} | [x] |
| 123 | `LZ4F_compressionLevel_max` / `LZ4F_getVersion` | constants | [x] |
| **lz4frame — decompression** | | | |
| 124 | `LZ4F_decompress` | one-shot decode of every row 100-120 frame | [x] |
| 125 | `LZ4F_headerSize` + `LZ4F_getFrameInfo` | on every frame variant of row 108 (7/11/15/19-byte headers) | [x] |
| 126 | `LZ4F_getFrameInfo` | called BEFORE any decompress, then decode continues | [x] |
| 127 | `LZ4F_getFrameInfo` | called mid-frame (after the header was consumed by `LZ4F_decompress`) | [x] |
| 128 | `LZ4F_decompress` | src fed in FIXED chunks {1,2,3,4,5,7,11,15,19,20,33,64,1000} (splits header, block header, checksums) | [x] |
| 129 | `LZ4F_decompress` | src fed in RANDOM chunk sizes | [x] |
| 130 | `LZ4F_decompress` | dst offered in FIXED small chunks {1,2,3,7,64,1000} (tmpOut buffering) | [x] |
| 131 | `LZ4F_decompress` | dst offered in RANDOM chunk sizes | [x] |
| 132 | `LZ4F_decompress` | BOTH src and dst chunked randomly, `nextSrcSizeHint` respected | [x] |
| 133 | `LZ4F_decompress` | `LZ4F_decompressOptions_t.stableDst` {0,1} | [x] |
| 134 | `LZ4F_decompress` | `skipChecksums` {0,1} on frames with content and/or block checksums | [x] |
| 135 | `LZ4F_decompress` | blockLinked frames chunked so matches reach into `tmpOutBuffer` history | [x] |
| 136 | `LZ4F_decompress` | frames containing STORED (uncompressed) blocks from row 114 | [x] |
| 137 | `LZ4F_decompress` | multiple concatenated frames on one dctx | [x] |
| 138 | `LZ4F_decompress` | skippable frame (`0x184D2A50..5F`) followed by a real frame | [x] |
| 139 | `LZ4F_decompress` | empty frame (0-byte content) with every checksum combination | [x] |
| 140 | `LZ4F_decompress_usingDict` | dict sweep matching rows 116-119 × chunked src | [x] |
| 141 | `LZ4F_resetDecompressionContext` | reset mid-frame then decode a fresh frame | [x] |
| 142 | `LZ4F_createDecompressionContext_advanced` | `LZ4F_defaultCMem` + reused dctx across MANY frames | [x] |
| 143 | `LZ4F_decompress` | C-produced frame decoded by Rust and Rust-produced frame decoded by C (cross-decode) | [x] |
| **lz4file** | | | |
| 144 | `LZ4F_writeOpen`/`LZ4F_write`*/`LZ4F_writeClose` | `prefsPtr == NULL`, write chunk-size sweep | [x] |
| 145 | `LZ4F_writeOpen`/`write`*/`writeClose` | blockSizeID {0,4,5,6,7} × blockMode × checksums × level {1,9,12} | [x] |
| 146 | `LZ4F_writeOpen`/`write`*/`writeClose` | RANDOM write chunk sizes, total sizes crossing block boundaries | [x] |
| 147 | `LZ4F_readOpen`/`LZ4F_read`*/`LZ4F_readClose` | read chunk-size sweep {1,2,7,64,1000,1MB} over row 145's files | [x] |
| 148 | `LZ4F_readOpen`/`read`*/`readClose` | RANDOM read chunk sizes | [x] |
| 149 | lz4file round trip | C-written file read by Rust and Rust-written file read by C | [x] |
| 150 | lz4file | file containing multiple frames / trailing garbage | [x] |
| **cross-library interop** | | | |
| 151 | block API | C compresses → Rust decompresses, and vice versa, over rows 12-99 | [x] |
| 152 | frame API | C compresses → Rust decompresses, and vice versa, over rows 100-143 | [x] |

## Notes on axes deliberately NOT crossed

- `byPtr` tableType and the 32-bit `read_variable_length` overflow checks
  (`sizeof(size_t) < 8`) are unreachable on this x86-64 target.
- `LZ4_HEAPMODE`/`LZ4F_HEAPMODE` are fixed to 0 by `CMakeLists.txt`, so the
  heap-allocation `return 0` paths in `lz4.c` are not compiled;
  `LZ4HC_HEAPMODE` defaults to 1 and IS compiled.
- `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION` is not defined, so all magic-number
  and checksum validations are active.
- `Cargo.toml` declares NO `[features]`, so there is exactly ONE feature
  combination: the default (identical to `--no-default-features`).
