# LZ4 Error Surface Table

Derived mechanically from the C sources in `c_src/`. Every row is one distinct
rejection / failure path, traceable to a `file:line`.

## LZ4F_errorCodes enum values

The enum is generated in `c_src/include/lz4frame.h:653-684` by expanding
`LZ4F_LIST_ERRORS(LZ4F_GENERATE_ENUM)`, where
`#define LZ4F_GENERATE_ENUM(ENUM) LZ4F_##ENUM,`. The list order fixes the
integer values (first member `LZ4F_OK_NoError = 0`, then +1 per `ITEM`):

| value | enum name | returned as `LZ4F_errorCode_t` (`size_t`) | produced where |
|-------|-----------|--------------------------------------------|----------------|
| 0  | `LZ4F_OK_NoError`                          | `0` (success) | success returns |
| 1  | `LZ4F_ERROR_GENERIC`                       | `(size_t)-1`  | **never produced** in this source set |
| 2  | `LZ4F_ERROR_maxBlockSize_invalid`          | `(size_t)-2`  | lz4frame.c:339, :1410, :1738; lz4file.c:124, :246 |
| 3  | `LZ4F_ERROR_blockMode_invalid`             | `(size_t)-3`  | **never produced** |
| 4  | `LZ4F_ERROR_parameter_invalid`             | `(size_t)-4`  | lz4frame.c:768 |
| 5  | `LZ4F_ERROR_compressionLevel_invalid`      | `(size_t)-5`  | **never produced** (levels are clamped instead) |
| 6  | `LZ4F_ERROR_headerVersion_wrong`           | `(size_t)-6`  | lz4frame.c:1389 |
| 7  | `LZ4F_ERROR_blockChecksum_invalid`         | `(size_t)-7`  | lz4frame.c:1829, :1878 |
| 8  | `LZ4F_ERROR_reservedFlag_set`              | `(size_t)-8`  | lz4frame.c:1388, :1409, :1411 |
| 9  | `LZ4F_ERROR_allocation_failed`             | `(size_t)-9`  | lz4frame.c:625, :722, :750, :1308, :1686, :1689; lz4file.c:85, :131, :227, :256 |
| 10 | `LZ4F_ERROR_srcSize_tooLarge`              | `(size_t)-10` | **never produced** |
| 11 | `LZ4F_ERROR_dstMaxSize_tooSmall`           | `(size_t)-11` | lz4frame.c:456, :700, :1007, :1010, :1169, :1221, :1227 |
| 12 | `LZ4F_ERROR_frameHeader_incomplete`        | `(size_t)-12` | lz4frame.c:1354, :1450, :1507 |
| 13 | `LZ4F_ERROR_frameType_unknown`             | `(size_t)-13` | lz4frame.c:1374, :1459 |
| 14 | `LZ4F_ERROR_frameSize_wrong`               | `(size_t)-14` | lz4frame.c:1237, :1984 |
| 15 | `LZ4F_ERROR_srcPtr_wrong`                  | `(size_t)-15` | lz4frame.c:1446 |
| 16 | `LZ4F_ERROR_decompressionFailed`           | `(size_t)-16` | lz4frame.c:1905, :1950 |
| 17 | `LZ4F_ERROR_headerChecksum_invalid`        | `(size_t)-17` | lz4frame.c:1418 |
| 18 | `LZ4F_ERROR_contentChecksum_invalid`       | `(size_t)-18` | lz4frame.c:2021 |
| 19 | `LZ4F_ERROR_frameDecoding_alreadyStarted`  | `(size_t)-19` | lz4frame.c:1501 |
| 20 | `LZ4F_ERROR_compressionState_uninitialized`| `(size_t)-20` | lz4frame.c:1005, :1168 |
| 21 | `LZ4F_ERROR_parameter_null`                | `(size_t)-21` | lz4frame.c:622, :1304; lz4file.c:80, :146, :186, :223, :289, :322 |
| 22 | `LZ4F_ERROR_io_write`                      | `(size_t)-22` | lz4file.c:273, :306/307, :334 |
| 23 | `LZ4F_ERROR_io_read`                       | `(size_t)-23` | lz4file.c:98, :162 |
| 24 | `LZ4F_ERROR_maxCode`                       | `(size_t)-24` | sentinel only, never returned |

Conversion helpers:
- `LZ4F_returnErrorCode(code)` = `(LZ4F_errorCode_t)-(ptrdiff_t)code` — lz4frame.c:311-316
- `LZ4F_isError(code)` = `code > (LZ4F_errorCode_t)(-LZ4F_ERROR_maxCode)` i.e. any value in
  `[(size_t)-23 .. (size_t)-1]` — lz4frame.c:293-296
- `LZ4F_getErrorCode(r)` = `(LZ4F_errorCodes)(-(ptrdiff_t)r)`, or `LZ4F_OK_NoError` if not an error — lz4frame.c:305-309
- `LZ4F_getErrorName(code)` indexes `LZ4F_errorStrings[-(int)code]` when `LZ4F_isError`, else the literal `"Unspecified error code"` — lz4frame.c:298-303
- `lz4file.c` re-defines its own `RETURN_ERROR` over a private `returnErrorCode()` with identical semantics — lz4file.c:40-45

## Relevant compile-time / range constants

| constant | value | where |
|---|---|---|
| `LZ4_MAX_INPUT_SIZE` | `0x7E000000` (2 113 929 216) | lz4.h:214 |
| `LZ4_COMPRESSBOUND(isize)` | `0` if `(unsigned)isize > LZ4_MAX_INPUT_SIZE`, else `isize + isize/255 + 16` | lz4.h:215 |
| `LZ4_MEMORY_USAGE_MIN` / `_DEFAULT` / `_MAX` | `10` / `14` / `20`; out-of-range triggers `#error` | lz4.h:162-172 |
| `LZ4_ACCELERATION_DEFAULT` / `LZ4_ACCELERATION_MAX` | `1` / `65537` (values clamped, never rejected) | lz4.h:233-234 |
| `LZ4_DISTANCE_MAX` | `65535` | lz4.h:673-674 |
| `LZ4_STREAM_MINSIZE` / `LZ4_STREAMHC_MINSIZE` | `(1<<LZ4_MEMORY_USAGE)+32` / `262200` | lz4.h:729, lz4hc.h:252 |
| `LZ4HC_CLEVEL_MIN` / `_DEFAULT` / `_OPT_MIN` / `_MAX` | `2` / `9` / `10` / `12` | lz4hc.h:47-50 |
| `LZ4F_VERSION` | `100` (stored, never validated) | lz4frame.h:256 |
| `LZ4F_HEADER_SIZE_MIN` / `_MAX` (`minFHSize`/`maxFHSize`) | `7` / `19` | lz4frame.h:280-281, lz4frame.c:252-253 |
| `LZ4F_BLOCK_HEADER_SIZE` (`BHSize`) / `LZ4F_BLOCK_CHECKSUM_SIZE` (`BFSize`) | `4` / `4` | lz4frame.h:284,287 |
| `LZ4F_MAGICNUMBER` / `LZ4F_MAGIC_SKIPPABLE_START` | `0x184D2204` / `0x184D2A50` (masked `& 0xFFFFFFF0`) | lz4frame.h:402-403 |
| `LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` | `5` | lz4frame.h:404 |
| `LZ4F_BLOCKUNCOMPRESSED_FLAG` | `0x80000000` | lz4frame.c:249 |
| block size IDs | `LZ4F_max64KB=4 .. LZ4F_max4MB=7`; `0` means default (`LZ4F_max64KB`) | lz4frame.c:333-343 |
| `XXH_OK` / `XXH_ERROR` | `0` / `1` | xxhash.h:79 |

---

## Error / rejection table

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `LZ4F_getBlockSize` | `blockSizeID` (after `0` -> `LZ4F_BLOCKSIZEID_DEFAULT`) is `< LZ4F_max64KB (4)` or `> LZ4F_max4MB (7)` — lz4frame.c:338-339 | `(size_t)-2` `LZ4F_ERROR_maxBlockSize_invalid` |
| 2 | `LZ4F_compressFrame_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)` — lz4frame.c:456 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 3 | `LZ4F_compressFrame_usingCDict` | error forwarded from `LZ4F_compressBegin_usingCDict` / `LZ4F_compressUpdate` / `LZ4F_compressEnd` via `FORWARD_IF_ERROR` — lz4frame.c:459, 464, 469 | the propagated inner error code |
| 4 | `LZ4F_createCDict_advanced` | `LZ4F_malloc(sizeof(LZ4F_CDict))` fails — lz4frame.c:541-542 | `NULL` |
| 5 | `LZ4F_createCDict_advanced` | any of `dictContent` / `fastCtx` / `HCCtx` allocations fail — lz4frame.c:555-558 | frees partial cdict, returns `NULL` |
| 6 | `LZ4F_createCompressionContext_advanced` | `LZ4F_calloc(sizeof(LZ4F_cctx))` fails — lz4frame.c:598-600 | `NULL` |
| 7 | `LZ4F_createCompressionContext` | `LZ4F_compressionContextPtr == NULL` (also `assert` at lz4frame.c:620) — lz4frame.c:622 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 8 | `LZ4F_createCompressionContext` | inner `LZ4F_createCompressionContext_advanced` returned NULL (`*ptr == NULL`) — lz4frame.c:625 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 9 | `LZ4F_compressBegin_internal` | `dstCapacity < maxFHSize (19)` — lz4frame.c:700 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 10 | `LZ4F_compressBegin_internal` | `LZ4F_malloc` of `LZ4_stream_t`/`LZ4_streamHC_t` returned NULL (`cctx->lz4CtxPtr == NULL`) — lz4frame.c:722 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 11 | `LZ4F_compressBegin_internal` | `LZ4F_malloc(requiredBuffSize)` for `cctx->tmpBuff` returned NULL — lz4frame.c:750 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 12 | `LZ4F_compressBegin_internal` | `dictBuffer != NULL` and `dictSize > INT_MAX` — lz4frame.c:768 | `(size_t)-4` `LZ4F_ERROR_parameter_invalid` |
| 13 | `LZ4F_compressUpdateImpl` (`LZ4F_compressUpdate`) | `cctxPtr->cStage != 1` (compressBegin not called / frame already ended) — lz4frame.c:1005 | `(size_t)-20` `LZ4F_ERROR_compressionState_uninitialized` |
| 14 | `LZ4F_compressUpdateImpl` | `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)` — lz4frame.c:1006-1007 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 15 | `LZ4F_uncompressedUpdate` (`blockCompression == LZ4B_UNCOMPRESSED`) | `dstCapacity < srcSize` — lz4frame.c:1009-1010 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 16 | `LZ4F_flush` | `tmpInSize > 0` and `cctxPtr->cStage != 1` — lz4frame.c:1168 | `(size_t)-20` `LZ4F_ERROR_compressionState_uninitialized` |
| 17 | `LZ4F_flush` | `dstCapacity < tmpInSize + BHSize(4) + BFSize(4)` — lz4frame.c:1169 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 18 | `LZ4F_compressEnd` | remaining `dstCapacity` (after flush) `< 4` for the endMark — lz4frame.c:1221 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 19 | `LZ4F_compressEnd` | `contentChecksumFlag == LZ4F_contentChecksumEnabled` and remaining `dstCapacity < 8` (endMark + CRC) — lz4frame.c:1227 | `(size_t)-11` `LZ4F_ERROR_dstMaxSize_tooSmall` |
| 20 | `LZ4F_compressEnd` | declared `prefs.frameInfo.contentSize != cctxPtr->totalInSize` — lz4frame.c:1235-1237 | `(size_t)-14` `LZ4F_ERROR_frameSize_wrong` |
| 21 | `LZ4F_createDecompressionContext_advanced` | `LZ4F_calloc(sizeof(LZ4F_dctx))` fails — lz4frame.c:1286-1287 | `NULL` |
| 22 | `LZ4F_createDecompressionContext` | `LZ4F_decompressionContextPtr == NULL` (also `assert` at lz4frame.c:1303) — lz4frame.c:1304 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 23 | `LZ4F_createDecompressionContext` | allocation failed (`*ptr == NULL`) — lz4frame.c:1307-1309 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 24 | `LZ4F_decodeHeader` | `srcSize < minFHSize (7)` — lz4frame.c:1354 | `(size_t)-12` `LZ4F_ERROR_frameHeader_incomplete` |
| 25 | `LZ4F_decodeHeader` | first 4 bytes are neither `LZ4F_MAGIC_SKIPPABLE_START` (masked `&0xFFFFFFF0`) nor `LZ4F_MAGICNUMBER 0x184D2204` — lz4frame.c:1372-1375 | `(size_t)-13` `LZ4F_ERROR_frameType_unknown` |
| 26 | `LZ4F_decodeHeader` | FLG byte bit 1 (`(FLG>>1)&1`) set — reserved — lz4frame.c:1388 | `(size_t)-8` `LZ4F_ERROR_reservedFlag_set` |
| 27 | `LZ4F_decodeHeader` | FLG version field `(FLG>>6)&3 != 1` — lz4frame.c:1389 | `(size_t)-6` `LZ4F_ERROR_headerVersion_wrong` |
| 28 | `LZ4F_decodeHeader` | BD byte bit 7 (`(BD>>7)&1`) set — reserved — lz4frame.c:1409 | `(size_t)-8` `LZ4F_ERROR_reservedFlag_set` |
| 29 | `LZ4F_decodeHeader` | `blockSizeID = (BD>>4)&7` is `< 4` (only 4..7 supported) — lz4frame.c:1410 | `(size_t)-2` `LZ4F_ERROR_maxBlockSize_invalid` |
| 30 | `LZ4F_decodeHeader` | BD low 4 bits (`BD & 0x0F`) nonzero — reserved — lz4frame.c:1411 | `(size_t)-8` `LZ4F_ERROR_reservedFlag_set` |
| 31 | `LZ4F_decodeHeader` | header checksum byte mismatch: `LZ4F_headerChecksum(src+4, frameHeaderSize-5) != src[frameHeaderSize-1]` — lz4frame.c:1417-1418 | `(size_t)-17` `LZ4F_ERROR_headerChecksum_invalid` |
| 32 | `LZ4F_headerSize` | `src == NULL` — lz4frame.c:1446 | `(size_t)-15` `LZ4F_ERROR_srcPtr_wrong` |
| 33 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH (5)` — lz4frame.c:1449-1450 | `(size_t)-12` `LZ4F_ERROR_frameHeader_incomplete` |
| 34 | `LZ4F_headerSize` | magic number is neither skippable-start nor `LZ4F_MAGICNUMBER` — lz4frame.c:1458-1459 | `(size_t)-13` `LZ4F_ERROR_frameType_unknown` |
| 35 | `LZ4F_getFrameInfo` | `dctx->dStage == dstage_storeFrameHeader` (already mid-header) — lz4frame.c:1499-1501 | `*srcSizePtr=0`; `(size_t)-19` `LZ4F_ERROR_frameDecoding_alreadyStarted` |
| 36 | `LZ4F_getFrameInfo` | `LZ4F_headerSize()` returned an error — lz4frame.c:1505 | `*srcSizePtr=0`; that error code forwarded |
| 37 | `LZ4F_getFrameInfo` | `*srcSizePtr < hSize` (not enough input for the full header) — lz4frame.c:1506-1508 | `*srcSizePtr=0`; `(size_t)-12` `LZ4F_ERROR_frameHeader_incomplete` |
| 38 | `LZ4F_getFrameInfo` | `LZ4F_decodeHeader()` returned an error — lz4frame.c:1511-1513 | `*srcSizePtr=0`; that error code forwarded |
| 39 | `LZ4F_decompress` (`dstage_getFrameHeader`) | `LZ4F_decodeHeader` error on the fast path (>= maxFHSize available) — lz4frame.c:1650-1651 | forwarded error code |
| 40 | `LZ4F_decompress` (`dstage_storeFrameHeader`) | `LZ4F_decodeHeader` error after buffering the header — lz4frame.c:1673 | forwarded error code |
| 41 | `LZ4F_decompress` (`dstage_init`) | `LZ4F_malloc(maxBlockSize + BFSize)` for `dctx->tmpIn` returned NULL — lz4frame.c:1686 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 42 | `LZ4F_decompress` (`dstage_init`) | `LZ4F_malloc(bufferNeeded)` for `dctx->tmpOutBuffer` returned NULL — lz4frame.c:1689 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 43 | `LZ4F_decompress` (block header decode) | `nextCBlockSize = blockHeader & 0x7FFFFFFF` exceeds `dctx->maxBlockSize` — lz4frame.c:1737-1738 | `(size_t)-2` `LZ4F_ERROR_maxBlockSize_invalid` |
| 44 | `LZ4F_decompress` (`dstage_getBlockChecksum`) | stored block CRC32 `!=` `XXH32_digest(&dctx->blockChecksum)` for an *uncompressed* block, and `skipChecksum == 0` — lz4frame.c:1825-1829 | `(size_t)-7` `LZ4F_ERROR_blockChecksum_invalid` |
| 45 | `LZ4F_decompress` (compressed block CRC) | trailing CRC of a compressed block `readBlockCrc != XXH32(selectedIn, tmpInTarget, 0)` — lz4frame.c:1878 | `(size_t)-7` `LZ4F_ERROR_blockChecksum_invalid` |
| 46 | `LZ4F_decompress` (decode into dst) | `LZ4_decompress_safe_usingDict()` returned `< 0` (corrupt block) — lz4frame.c:1905 | `(size_t)-16` `LZ4F_ERROR_decompressionFailed` |
| 47 | `LZ4F_decompress` (decode into `tmpOut`) | `LZ4_decompress_safe_usingDict()` returned `< 0` — lz4frame.c:1950 | `(size_t)-16` `LZ4F_ERROR_decompressionFailed` |
| 48 | `LZ4F_decompress` (`dstage_getSuffix`) | endMark reached while `dctx->frameRemainingSize != 0` (decoded size < declared contentSize) — lz4frame.c:1984 | `(size_t)-14` `LZ4F_ERROR_frameSize_wrong` |
| 49 | `LZ4F_decompress` (suffix check) | frame content checksum mismatch `readCRC != XXH32_digest(&dctx->xxh)` with `skipChecksum == 0` — lz4frame.c:2018-2021 | `(size_t)-18` `LZ4F_ERROR_contentChecksum_invalid` |
| 50 | `LZ4F_freeDecompressionContext` | context freed while `dctx->dStage != dstage_getFrameHeader(0)` — returns the raw stage value, i.e. a nonzero code that `LZ4F_isError` may report for unfinished frames — lz4frame.c:1313-1324 | `(LZ4F_errorCode_t)dctx->dStage` |
| 51 | `ctxTypeID_to_size` (lz4frame.c) | `ctxTypeID` not `1` (fast) or `2` (HC) — lz4frame.c:676-682 | `0` (drives the "not enough space allocated" branch) |
| 52 | `LZ4F_makeBlock` | compressor returned `cSize == 0` (incompressible) or `cSize >= srcSize` — lz4frame.c:896 | not an error: block is rewritten as stored/uncompressed with `LZ4F_BLOCKUNCOMPRESSED_FLAG` |
| 53 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.blockSizeID == 0` — lz4frame.c:740-741 | not an error: silently replaced by `LZ4F_BLOCKSIZEID_DEFAULT` |
| 54 | `LZ4F_selectCompression` / `LZ4F_compressBlock` | `compressionLevel < LZ4HC_CLEVEL_MIN (2)` selects the fast codec; negative level becomes `acceleration = -level + 1` — lz4frame.c:955-961, :911 | not an error: level is reinterpreted, never rejected |
| 55 | `LZ4F_decompress` (`dstage_getCBlock`) | `dctx->dictSize > 1 GB` — lz4frame.c:1897-1901, :1938-1942 | not an error: dict silently truncated to its last 64 KB (int-overflow guard) |

## lz4.c (block API)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 56 | `LZ4_compressBound` / `LZ4_COMPRESSBOUND` | `(unsigned)isize > LZ4_MAX_INPUT_SIZE (0x7E000000)` — lz4.h:215, lz4.c:751 | `0` |
| 57 | `LZ4_compress_generic` | `(U32)srcSize > (U32)LZ4_MAX_INPUT_SIZE` — also catches any negative `srcSize` after the unsigned cast — lz4.c:1360 | `0` |
| 58 | `LZ4_compress_generic` | `srcSize == 0` and `outputDirective != notLimited` and `dstCapacity <= 0` (no room for even the 1-byte empty block) — lz4.c:1362 | `0` |
| 59 | `LZ4_compress_generic_validated` | `outputDirective == fillOutput` and `maxOutputSize < 1` — lz4.c:985 | `0` |
| 60 | `LZ4_compress_generic_validated` | `limitedOutput` and literal run does not fit: `op + litLength + (2+1+LASTLITERALS) + litLength/255 > olimit` — lz4.c:1113-1116 | `0` (hash table left valid) |
| 61 | `LZ4_compress_generic_validated` | `limitedOutput` and match-length encoding does not fit in `dst` — lz4.c:1208-1210 | `0` |
| 62 | `LZ4_compress_generic_validated` | `limitedOutput` and final literal run does not fit: `op + lastRun + 1 + (lastRun+255-RUN_MASK)/255 > olimit` — lz4.c:1305-1314 | `0` |
| 63 | `LZ4_compress_fast` / `LZ4_compress_default` | `LZ4_HEAPMODE` build and `ALLOC(sizeof(LZ4_stream_t))` returned NULL — lz4.c:1457-1458 | `0` |
| 64 | `LZ4_compress_destSize` | `LZ4_HEAPMODE` build and `ALLOC(sizeof(LZ4_stream_t))` returned NULL — lz4.c:1509-1510 | `0` |
| 65 | `LZ4_compress_fast_extState` / `_fastReset` | `acceleration < 1` or `> LZ4_ACCELERATION_MAX (65537)` — lz4.c:1386-1387, :1417-1418 | not an error: clamped to `LZ4_ACCELERATION_DEFAULT (1)` / `LZ4_ACCELERATION_MAX` |
| 66 | `LZ4_createStream` | `ALLOC(sizeof(LZ4_stream_t))` returned NULL — lz4.c:1536 | `NULL` |
| 67 | `LZ4_initStream` | `buffer == NULL` — lz4.c:1555 | `NULL` |
| 68 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` (i.e. `< LZ4_STREAM_MINSIZE`) — lz4.c:1556 | `NULL` |
| 69 | `LZ4_initStream` | `buffer` not aligned to `LZ4_stream_t_alignment()` (only enforced when `LZ4_ALIGN_TEST`) — lz4.c:1557 | `NULL` |
| 70 | `LZ4_freeStream` | `LZ4_stream == NULL` — lz4.c:1577 | `0` (free-on-NULL tolerated, not an error) |
| 71 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < HASH_UNIT (sizeof(reg_t)`, 8 on 64-bit) — lz4.c:1613-1615 | `0` — dictionary not installed |
| 72 | `LZ4_loadDict_internal` | `dictSize > 64 KB` — lz4.c:1617 | not an error: only the last 64 KB is kept |
| 73 | `LZ4_compress_fast_continue` | `streamPtr->dictSize < 4` and not prefix mode and `inputSize > 0` and no `dictCtx` — lz4.c:1721-1726 | not an error: dictionary silently discarded (`dictSize = 0`) |
| 74 | `LZ4_compress_fast_continue` | src overlaps the recorded dictionary (`sourceEnd > dictionary && sourceEnd < dictEnd`) — lz4.c:1735-1741 | not an error: dictionary shrunk / zeroed (`<4` -> `0`) |
| 75 | `LZ4_renormDictT` | `currentOffset + nextSize > 0x80000000` (ptrdiff overflow risk in 32-bit) — lz4.c:1690-1704 | not an error: hash table rescaled, `dictSize` capped to 64 KB |
| 76 | `LZ4_saveDict` | `(U32)dictSize > 64 KB`, or `> dict->dictSize` — lz4.c:1821-1822 | not an error: clamped; returns the (possibly reduced) saved size |
| 77 | `LZ4_saveDict` | `safeBuffer == NULL` while `dictSize != 0` — `assert(dictSize == 0)` — lz4.c:1823 | assertion failure in debug; UB in release |
| 78 | `LZ4_freeStreamDecode` | `LZ4_stream == NULL` — lz4.c:2577 | `0` (free-on-NULL tolerated) |
| 79 | `LZ4_setStreamDecode` | `dictSize != 0` with `dictionary == NULL` — `assert(dictionary != NULL)` — lz4.c:2594 | assertion failure in debug; function otherwise always returns `1` |
| 80 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` — lz4.c:2617 | `0` |
| 81 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` — lz4.c:2618 | `0` |
| 82 | `LZ4_decompress_generic` | `src == NULL` — lz4.c:2036 | `-1` |
| 83 | `LZ4_decompress_generic` | `outputSize < 0` — lz4.c:2036 | `-1` |
| 84 | `LZ4_decompress_generic` | `outputSize == 0`, full-block mode, and input is not exactly the 1-byte empty block (`!(srcSize==1 && *ip==0)`) — lz4.c:2062-2068 | `-1` (partial mode returns `0` instead) |
| 85 | `LZ4_decompress_generic` | `srcSize == 0` with `outputSize != 0` — lz4.c:2069 | `-1` |
| 86 | `read_variable_length` | `initial_check` set and `*ip >= ilimit` before the loop — lz4.c:1985-1987 | `rvl_error` = `(size_t)-1` |
| 87 | `read_variable_length` | `*ip > ilimit` after consuming a length byte (input exhausted mid-length) — lz4.c:1992-1994 and :2003-2005 | `rvl_error` |
| 88 | `read_variable_length` | 32-bit accumulator overflow: `sizeof(length) < 8 && length > (Rvl_t)-1/2` — lz4.c:1996-1998 and :2007-2009 | `rvl_error` |
| 89 | `LZ4_decompress_generic` fast loop | long literal length: `read_variable_length(&ip, iend-RUN_MASK, 1) == rvl_error` — lz4.c:2092-2097 | `goto _output_error` |
| 90 | `LZ4_decompress_generic` fast loop | literal-length pointer overflow `(uptrval)op + length < (uptrval)op` — lz4.c:2099 | `goto _output_error` |
| 91 | `LZ4_decompress_generic` fast loop | literal-length pointer overflow `(uptrval)ip + length < (uptrval)ip` — lz4.c:2100 | `goto _output_error` |
| 92 | `LZ4_decompress_generic` fast loop | long match length: `read_variable_length(&ip, iend-LASTLITERALS+1, 0) == rvl_error` — lz4.c:2126-2132 | `goto _output_error` |
| 93 | `LZ4_decompress_generic` fast loop | match-length pointer overflow `(uptrval)op + length < (uptrval)op` — lz4.c:2136 | `goto _output_error` |
| 94 | `LZ4_decompress_generic` fast loop | offset points outside available history: `checkOffset && (match + dictSize < lowPrefix)` — lz4.c:2162-2164 | `goto _output_error` |
| 95 | `LZ4_decompress_generic` fast loop, extDict | full-block mode and `op + length > oend - LASTLITERALS` (end-of-block rule violated) — lz4.c:2167-2175 | `goto _output_error` |
| 96 | `LZ4_decompress_generic` safe loop | long literal length `rvl_error` — lz4.c:2265-2266 | `goto _output_error` |
| 97 | `LZ4_decompress_generic` safe loop | literal-length `op` pointer overflow — lz4.c:2268 | `goto _output_error` |
| 98 | `LZ4_decompress_generic` safe loop | literal-length `ip` pointer overflow — lz4.c:2269 | `goto _output_error` |
| 99 | `LZ4_decompress_generic` `safe_literal_copy` | full-block mode near the buffer end and `(ip+length != iend) || (cpy > oend)` — i.e. this was not the final literal run, or output would overflow — lz4.c:2311-2318 | `goto _output_error` |
| 100 | `LZ4_decompress_generic` `_copy_match` | long match length `rvl_error` — lz4.c:2346-2347 | `goto _output_error` |
| 101 | `LZ4_decompress_generic` `_copy_match` | match-length `op` pointer overflow — lz4.c:2349 | `goto _output_error` |
| 102 | `LZ4_decompress_generic` `safe_match_copy` | `checkOffset && (match + dictSize < lowPrefix)` — offset outside buffers — lz4.c:2356 | `goto _output_error` |
| 103 | `LZ4_decompress_generic` `safe_match_copy`, extDict | full-block mode and `op + length > oend - LASTLITERALS` — lz4.c:2359-2362 | `goto _output_error` |
| 104 | `LZ4_decompress_generic` match copy tail | `cpy > oend - LASTLITERALS` — last 5 bytes of a block must be literals — lz4.c:2422-2423 | `goto _output_error` |
| 105 | `LZ4_decompress_generic` `_output_error` label | any of rows 89-104 | `(int)(-((const char*)ip - src)) - 1` — a negative value encoding the byte offset where parsing failed — lz4.c:2442-2443 |
| 106 | `LZ4_decompress_unsafe_generic` (`LZ4_decompress_fast*`) | literal run longer than remaining output: `(size_t)(oend-op) < ll` — lz4.c:1898 | `-1` |
| 107 | `LZ4_decompress_unsafe_generic` | literals leave `0 < (oend-op) < MFLIMIT (12)`, so no valid match can follow — lz4.c:1902-1907 | `-1` |
| 108 | `LZ4_decompress_unsafe_generic` | match length longer than remaining output: `(size_t)(oend-op) < ml` — lz4.c:1921 | `-1` |
| 109 | `LZ4_decompress_unsafe_generic` | `offset > (size_t)(op - prefixStart) + dictSize` — match before start of history — lz4.c:1925-1928 | `-1` |
| 110 | `LZ4_decompress_unsafe_generic` | match ends with `(size_t)(oend-op) < LASTLITERALS (5)` — lz4.c:1956-1961 | `-1` |
| 111 | `LZ4_decompress_safe_continue` | any of the three inner decode calls returns `<= 0` — lz4.c:2639, :2650, :2657 | that value returned unchanged; `lz4sd` prefix state is **not** advanced |
| 112 | `LZ4_decompress_fast_continue` | any of the three inner decode calls returns `<= 0` — lz4.c:2685, :2693, :2701 | that value returned unchanged |
| 113 | `LZ4_decompress_safe_partial*` | `targetOutputSize > dstCapacity` — lz4.c:2459, :2488, :2515, :2536 | not an error: `dstCapacity = MIN(targetOutputSize, dstCapacity)` |
| 114 | `LZ4_compress_generic_validated` | `tableType == byU16` with `inputSize >= LZ4_64Klimit` — `assert(inputSize<LZ4_64Klimit)` — lz4.c:981 | assertion failure in debug; internal-contract only |
| 115 | `LZ4_compress_generic_validated` | `tableType == byPtr` with `dictDirective != noDict` — `assert(dictDirective==noDict)` — lz4.c:982 | assertion failure in debug |
| 116 | `LZ4_putIndexOnHash` / `LZ4_getIndexOnHash` | called with `tableType == clearedTable` or `byPtr` — `assert(0); return;` — lz4.c:813, :826, :866 | forbidden case: assertion failure, returns without acting / returns `0` |

## lz4hc.c (HC block API)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 117 | `LZ4HC_getCLevelParams` | `cLevel < 1` — lz4hc.c:112-113 | not an error: replaced by `LZ4HC_CLEVEL_DEFAULT (9)` |
| 118 | `LZ4HC_getCLevelParams` | `cLevel > LZ4HC_CLEVEL_MAX (12)` — `cLevel = MIN(LZ4HC_CLEVEL_MAX, cLevel)` — lz4hc.c:114 | not an error: clamped to `12` (table `k_clTable[0..12]`, lz4hc.c:92-105) |
| 119 | `LZ4_setCompressionLevel` | `compressionLevel < 1` — lz4hc.c:1614 | not an error: set to `LZ4HC_CLEVEL_DEFAULT (9)` |
| 120 | `LZ4_setCompressionLevel` | `compressionLevel > LZ4HC_CLEVEL_MAX (12)` — lz4hc.c:1615 | not an error: clamped to `12` |
| 121 | `LZ4HC_compress_generic_internal` | `limit == fillOutput` and `dstCapacity < 1` — lz4hc.c:1388 | `0` |
| 122 | `LZ4HC_compress_generic_internal` | `(U32)*srcSizePtr > (U32)LZ4_MAX_INPUT_SIZE` (also catches negative sizes) — lz4hc.c:1389 | `0` |
| 123 | `LZ4HC_compress_generic_internal` | inner strategy returned `result <= 0` — lz4hc.c:1415 | `ctx->dirty = 1` is latched, the `<= 0` value is returned; the stream must be re-initialized (see `LZ4_resetStreamHC_fast`, lz4hc.c:1598-1601) |
| 124 | `LZ4MID_compress` | `*srcSizePtr < 0` — lz4hc.c:559 (also `assert(*srcSizePtr >= 0)` at :556) | `0` |
| 125 | `LZ4MID_compress` | `maxOutputSize < 0` — lz4hc.c:560 | `0` |
| 126 | `LZ4MID_compress` | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` — lz4hc.c:561-563 | `0` |
| 127 | `LZ4MID_compress` | `*srcSizePtr != 0` with `src == NULL` — `assert(src != NULL)` — lz4hc.c:557 | assertion failure in debug |
| 128 | `LZ4MID_compress` | `maxOutputSize != 0` with `dst == NULL` — `assert(dst != NULL)` — lz4hc.c:558 | assertion failure in debug |
| 129 | `LZ4MID_compress` `_lz4mid_last_literals` | `limit == limitedOutput` and `op + totalSize > oend` (final literal run does not fit) — lz4hc.c:713-714 | `0` |
| 130 | `LZ4MID_compress` `_lz4mid_dest_overflow` | dst overflow detected with `limit != fillOutput` — lz4hc.c:770-772 | `0` (compression failed) |
| 131 | `LZ4HC_encodeSequence` | `limit != notLimited` and `op + length/255 + length + (2+1+LASTLITERALS) > oend` — literals do not fit — lz4hc.c:304-308 | `1` (buffer issue), caller jumps to its `_dest_overflow` handler |
| 132 | `LZ4HC_encodeSequence` | `limit != notLimited` and `op + length/255 + (1+LASTLITERALS) > oend` — match length does not fit — lz4hc.c:330-333 | `1` (buffer issue) |
| 133 | `LZ4HC_encodeSequence` | `offset > LZ4_DISTANCE_MAX (65535)` or `offset <= 0` — `assert(offset <= LZ4_DISTANCE_MAX); assert(offset > 0)` — lz4hc.c:324-325 | assertion failure in debug |
| 134 | `LZ4HC_encodeSequence` | `matchLength < MINMATCH (4)` — `assert(matchLength >= MINMATCH)` — lz4hc.c:329 | assertion failure in debug |
| 135 | `LZ4HC_compress_hashChain` `_last_literals` | `limit == limitedOutput` and `op + totalSize > oend` — lz4hc.c:1314-1315 | `0` |
| 136 | `LZ4HC_compress_hashChain` `_dest_overflow` | dst overflow with `limit != fillOutput` — lz4hc.c:1359-1361 | `0` (compression failed) |
| 137 | `LZ4HC_compress_optimal` | `LZ4HC_HEAPMODE==1` and `ALLOC(sizeof(LZ4HC_optimal_t) * (LZ4_OPT_NUM + 3))` returned NULL — lz4hc.c:1856-1857 | `retval = 0` via `_return_label` |
| 138 | `LZ4HC_compress_optimal` `_last_literals` | `limit == limitedOutput` and `op + totalSize > oend` — lz4hc.c:2065-2068 | `retval = 0` |
| 139 | `LZ4HC_compress_optimal` `_dest_overflow` | dst overflow (from `LZ4HC_encodeSequence` returning 1 at lz4hc.c:1879/:2055) with `limit != fillOutput` — falls through to `_return_label` with `retval` still `0` (initialized lz4hc.c:1835) — lz4hc.c:2095-2118 | `0` |
| 140 | `LZ4_compress_HC_extStateHC_fastReset` | `state` not aligned to `LZ4_streamHC_t_alignment()` — lz4hc.c:1503 | `0` |
| 141 | `LZ4_compress_HC_extStateHC` | `LZ4_initStreamHC(state, sizeof(LZ4_streamHC_t))` returned NULL (bad ptr / too small / misaligned) — lz4hc.c:1515 | `0` (init failure) |
| 142 | `LZ4_compress_HC` | `LZ4HC_HEAPMODE==1` and `ALLOC(sizeof(LZ4_streamHC_t))` returned NULL — lz4hc.c:1524 | `0` |
| 143 | `LZ4_compress_HC_destSize` | `LZ4_initStreamHC` returned NULL — lz4hc.c:1541 | `0` (init failure) |
| 144 | `LZ4_createStreamHC` | `ALLOC_AND_ZERO(sizeof(LZ4_streamHC_t))` returned NULL — lz4hc.c:1558 | `NULL` |
| 145 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` — lz4hc.c:1566 | `0` (free-on-NULL tolerated) |
| 146 | `LZ4_initStreamHC` | `buffer == NULL` — lz4hc.c:1578 | `NULL` |
| 147 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` (i.e. `< LZ4_STREAMHC_MINSIZE = 262200`) — lz4hc.c:1579 | `NULL` |
| 148 | `LZ4_initStreamHC` | `buffer` not aligned to `LZ4_streamHC_t_alignment()` — lz4hc.c:1580 | `NULL` |
| 149 | `select_searchDict_function` | `dictCtx == NULL` — lz4hc.c:516 | `NULL` search function |
| 150 | `LZ4_loadDictHC` | `dictSize > 64 KB` — lz4hc.c:1635-1638 | not an error: only the last 64 KB is used; returns the truncated `dictSize` |
| 151 | `LZ4_loadDictHC` | negative `dictSize` — `assert(dictSize >= 0)` — lz4hc.c:1632 | assertion failure in debug (no runtime rejection) |
| 152 | `LZ4_loadDictHC` | `dictSize < LZ4HC_HASHSIZE (4)` in non-`lz4mid` strategies — lz4hc.c:1648 | not an error: no chain insertion performed |
| 153 | `LZ4_compressHC_continue_generic` | history index overflow `(end - prefixStart) + dictLimit > 2 GB` — lz4hc.c:1694-1698 | not an error: stream is re-loaded from the last (<=64 KB) of history |
| 154 | `LZ4_compressHC_continue_generic` | src overlaps the recorded extDict — lz4hc.c:1705-1716 | not an error: extDict is trimmed; invalidated entirely when `dictLimit - lowLimit < LZ4HC_HASHSIZE (4)` |
| 155 | `LZ4_saveDictHC` | `dictSize > 64 KB` -> `64 KB`; `dictSize < 4` -> `0`; `dictSize > prefixSize` -> `prefixSize` — lz4hc.c:1748-1750 | not an error: clamped; returns the clamped size |
| 156 | `LZ4_saveDictHC` | `safeBuffer == NULL` while `dictSize != 0` — `assert(dictSize == 0)` — lz4hc.c:1751 | assertion failure in debug |
| 157 | `LZ4_resetStreamStateHC` (obsolete) | `LZ4_initStreamHC(state, sizeof(*hc4))` returned NULL — lz4hc.c:2152-2153 | `1` (note: inverted convention — `0` means success here) |
| 158 | `LZ4_createHC` (obsolete) | `LZ4_createStreamHC()` returned NULL — lz4hc.c:2161-2162 | `NULL` |
| 159 | `LZ4_freeHC` (obsolete) | `LZ4HC_Data == NULL` — lz4hc.c:2170 | `0` (free-on-NULL tolerated) |

## lz4file.c (stdio file API)

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 160 | `LZ4F_readOpen` | `fp == NULL` or `lz4fRead == NULL` — lz4file.c:79-81 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 161 | `LZ4F_readOpen` | `calloc(1, sizeof(LZ4_readFile_t))` returned NULL — lz4file.c:84-86 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 162 | `LZ4F_readOpen` | `LZ4F_createDecompressionContext` failed — lz4file.c:88-92 | state freed and nulled; that error code forwarded |
| 163 | `LZ4F_readOpen` | short read: `fread(buf,1,LZ4F_HEADER_SIZE_MAX(19),fp) != 19` (file shorter than 19 bytes, or I/O error) — lz4file.c:95-99 | state freed and nulled; `(size_t)-23` `LZ4F_ERROR_io_read` |
| 164 | `LZ4F_readOpen` | `LZ4F_getFrameInfo` failed (bad magic / reserved bits / header CRC / etc.) — lz4file.c:102-106 | state freed and nulled; that error code forwarded |
| 165 | `LZ4F_readOpen` | `info.blockSizeID` not in `{LZ4F_default(0), LZ4F_max64KB(4), LZ4F_max256KB(5), LZ4F_max1MB(6), LZ4F_max4MB(7)}` — lz4file.c:108-125 | state freed and nulled; `(size_t)-2` `LZ4F_ERROR_maxBlockSize_invalid` |
| 166 | `LZ4F_readOpen` | `malloc(srcBufMaxSize)` returned NULL — lz4file.c:128-132 | state freed and nulled; `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 167 | `LZ4F_read` | `lz4fRead == NULL` or `buf == NULL` — lz4file.c:145-146 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 168 | `LZ4F_read` | `fread` fell into the "negative" branch — lz4file.c:159-163 (note: `ret` is `size_t`, so this branch is unreachable in practice; `ret == 0` breaks the loop and returns a short count instead) | `(size_t)-23` `LZ4F_ERROR_io_read` |
| 169 | `LZ4F_read` | `LZ4F_decompress` returned an error (corrupt stream, checksum mismatch, ...) — lz4file.c:166-173 | that error code returned verbatim |
| 170 | `LZ4F_readClose` | `lz4fRead == NULL` — lz4file.c:185-186 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 171 | `LZ4F_writeOpen` | `fp == NULL` or `lz4fWrite == NULL` — lz4file.c:222-223 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 172 | `LZ4F_writeOpen` | `calloc(1, sizeof(LZ4_writeFile_t))` returned NULL — lz4file.c:225-228 | `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 173 | `LZ4F_writeOpen` | `prefsPtr != NULL` and `prefsPtr->frameInfo.blockSizeID` not in `{0,4,5,6,7}` — lz4file.c:229-247 | state freed and nulled; `(size_t)-2` `LZ4F_ERROR_maxBlockSize_invalid` |
| 174 | `LZ4F_writeOpen` | `malloc(LZ4F_compressBound(maxWriteSize, prefsPtr))` returned NULL — lz4file.c:252-257 | state freed and nulled; `(size_t)-9` `LZ4F_ERROR_allocation_failed` |
| 175 | `LZ4F_writeOpen` | `LZ4F_createCompressionContext` failed — lz4file.c:259-263 | state freed and nulled; that error code forwarded |
| 176 | `LZ4F_writeOpen` | `LZ4F_compressBegin(cctx, buf, LZ4F_HEADER_SIZE_MAX, prefsPtr)` failed — lz4file.c:265-269 | state freed and nulled; that error code forwarded |
| 177 | `LZ4F_writeOpen` | short write of the frame header: `fwrite(buf,1,ret,fp) != ret` — lz4file.c:271-274 | state freed and nulled; `(size_t)-22` `LZ4F_ERROR_io_write` |
| 178 | `LZ4F_write` | `lz4fWrite == NULL` or `buf == NULL` — lz4file.c:288-289 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 179 | `LZ4F_write` | `LZ4F_compressUpdate` returned an error — lz4file.c:296-303 | error latched in `lz4fWrite->errCode` and returned |
| 180 | `LZ4F_write` | short write of a compressed chunk: `fwrite(dstBuf,1,ret,fp) != ret` — lz4file.c:305-308 | `errCode = (size_t)-22`; returns `(size_t)-22` `LZ4F_ERROR_io_write` |
| 181 | `LZ4F_writeClose` | `lz4fWrite == NULL` — lz4file.c:321-323 | `(size_t)-21` `LZ4F_ERROR_parameter_null` |
| 182 | `LZ4F_writeClose` | `LZ4F_compressEnd` failed — lz4file.c:326-331 | `goto out`: state freed, that error code returned |
| 183 | `LZ4F_writeClose` | short write of the frame trailer: `fwrite(dstBuf,1,ret,fp) != ret` — lz4file.c:333-335 | state freed; `(size_t)-22` `LZ4F_ERROR_io_write` |
| 184 | `LZ4F_writeClose` | `lz4fWrite->errCode != LZ4F_OK_NoError` from an earlier `LZ4F_write` — lz4file.c:325, :338-340 | the `compressEnd`/trailer write is **skipped** and `LZ4F_OK_NoError (0)` is returned — the earlier error is not re-reported |

## xxhash.c

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 185 | `XXH32_update` / `XXH32_update_endian` | `input == NULL` with the default `XXH_ACCEPT_NULL_INPUT_POINTER == 0` (xxhash.c:70-72) — xxhash.c:454-458 | `XXH_ERROR` (`1`) |
| 186 | `XXH64_update` / `XXH64_update_endian` | `input == NULL` with `XXH_ACCEPT_NULL_INPUT_POINTER == 0` — xxhash.c:914-918 | `XXH_ERROR` (`1`) |
| 187 | `XXH32_update` / `XXH64_update` | `input == NULL` when built with `XXH_ACCEPT_NULL_INPUT_POINTER >= 1` — xxhash.c:456, :916 | `XXH_OK` (`0`) — treated as a zero-length input |
| 188 | `XXH32` / `XXH64` (one-shot) | `input == NULL` when built with `XXH_ACCEPT_NULL_INPUT_POINTER >= 1` — xxhash.c:359-364, :818-823 | hash of an empty input; with the default `0` the NULL pointer is dereferenced (documented segfault, xxhash.c:65-69) |
| 189 | `XXH32_createState` | `XXH_malloc(sizeof(XXH32_state_t))` returned NULL — xxhash.c:422-425 | `NULL` |
| 190 | `XXH64_createState` | `XXH_malloc(sizeof(XXH64_state_t))` returned NULL — xxhash.c:883-886 | `NULL` |
| 191 | `XXH32_freeState` / `XXH64_freeState` | `statePtr == NULL` — xxhash.c:426-430, :887-891 | `XXH_OK` (`0`) — free-on-NULL tolerated, never returns `XXH_ERROR` |
| 192 | `XXH32_reset` / `XXH64_reset` | `statePtr == NULL` — there is **no** NULL check; the final `memcpy(statePtr, &state, ...)` dereferences it — xxhash.c:437-450, :898-911 | no error return possible: always `XXH_OK`; NULL state is undefined behaviour |
| 193 | `XXH32_digest` / `XXH64_digest` | `state_in == NULL` or an unreset state — no validation at all — xxhash.c:545-555, :1005-1014 | no error path: a hash value is always returned (`XXH32_hash_t` / `XXH64_hash_t`, not an error code) |
| 194 | `XXH32_copyState` / `XXH64_copyState` | NULL `dstState` or `srcState` — plain `memcpy`, no checks — xxhash.c:432-435, :893-896 | `void`; undefined behaviour |
| 195 | `XXH32_finalize` / `XXH64_finalize` | control reaches past the `switch` on `len & 15` / `len & 31` — `assert(0)` marked "reaching this point is deemed impossible" — xxhash.c:346-347, :806-807 | assertion failure in debug; returns the un-avalanched accumulator in release |
| 196 | `XXH32_canonicalFromHash` / `XXH64_canonicalFromHash` | `sizeof(XXH32_canonical_t) != sizeof(XXH32_hash_t)` (resp. 64-bit) — `XXH_STATIC_ASSERT` — xxhash.c:567, :1020 | compile-time failure (division by zero in an enum initializer) |
| 197 | `XXH_STATIC_ASSERT` in `lz4frame.c` `LZ4F_returnErrorCode` | `sizeof(ptrdiff_t) < sizeof(size_t)` — `LZ4F_STATIC_ASSERT` — lz4frame.c:313-314 | compile-time failure ("a compilation error here means sizeof(ptrdiff_t) is not large enough") |
| 198 | `LZ4_MEMORY_USAGE` build config | `LZ4_MEMORY_USAGE < LZ4_MEMORY_USAGE_MIN (10)` or `> LZ4_MEMORY_USAGE_MAX (20)` — lz4.h:166-172 | `#error "LZ4_MEMORY_USAGE is too small !"` / `"... too large !"` — compile-time rejection |

## Phase C status — all 198 rows have a differential test

Each row `N` has a test whose function name contains `err_N` / `errors_N`, in:

| rows | test file |
|---|---|
| 1-23    | `tests/errors_frame.rs` |
| 24-55   | `tests/lz4frame_decomp.rs` |
| 56-159  | `tests/errors_block.rs` |
| 160-184 | `tests/errors_lz4file.rs` |
| 185-198 | `tests/errors_xxhash.rs` |

Audit mechanically with:

```sh
grep -ohE 'fn (err|errors)_[0-9]+[a-z_0-9]*' tests/*.rs \
  | tr -cs '0-9' '\n' | grep -E '^[0-9]+$' | sort -n -u > /tmp/errs
for i in $(seq 1 198); do grep -qx "$i" /tmp/errs || echo "UNCOVERED $i"; done
```

Every test asserts the EXACT error code or sentinel returned by both libraries — never
merely "both failed". The three distinct error conventions in this library are each
checked exactly: lz4frame's `(size_t)-enum`; lz4 compression's `0`; and lz4
decompression's `-1` or negative-offset `(int)(-(ip-src))-1` encoding the failure
position. The two inverted-convention outliers are pinned explicitly:
`LZ4_resetStreamStateHC` (`1` = failure, `0` = success) and
`LZ4F_freeDecompressionContext` (returns the raw `dStage`).

### Rows that cannot be triggered through the ABI

These keep a named test that documents WHY and asserts the closest reachable observable
behaviour instead. They are not stubs and they are not silently skipped:

- **Allocation failures reachable only via a custom allocator** — rows 4, 5, 6, 10, 11,
  21, 41, 42 WERE forced for real, using the exported `*_advanced` entry points with a
  caller-supplied `LZ4F_CustomMem` whose alloc/calloc shims fail on the Nth call.
- **Allocation failures with no hook** — rows 8, 23 (lz4frame hard-codes
  `LZ4F_defaultCMem`), 161, 166, 172, 174 (lz4file calls libc `calloc`/`malloc`
  directly), 189, 190 (xxhash's `XXH_malloc`). Process-wide allocator interposition
  would also break the test harness, so the success half of the same statement is pinned.
- **Logically dead branches** — row 16 (`cStage == 0` implies `tmpInSize == 0`, so
  `LZ4F_flush` returns 0, not `err(20)`), row 24 (`LZ4F_decodeHeader`'s `srcSize < 7`
  guard is pre-empted by its callers), row 51 (`ctxTypeID` is only ever literal 1 or 2),
  row 95 (incompatible with the fast loop's own branch condition), row 168 (`ret` is
  `size_t`, so the "negative fread" arm is unreachable — the reachable EOF short-count
  consequence IS tested), row 179, rows 1/5 dead enum members.
- **Documented undefined behaviour** — rows 77, 156 (`safeBuffer == NULL` memmoves to
  NULL), 192, 194 (xxhash `reset`/`copyState` have no NULL check), 188 with `len > 0`
  (one-shot dereferences a NULL input), and the 64-bit pointer-overflow guards
  (rows 88, 90, 91, 93, 97, 98, 101) which cannot wrap because
  `length <= 255*(2^31-1) < 2^40`. These are NOT invoked, since doing so would crash the
  test process rather than reveal a divergence.
- **Compile-time assertions** — rows 196, 197, 198 (`XXH_STATIC_ASSERT`,
  `LZ4F_STATIC_ASSERT`, the `LZ4_MEMORY_USAGE` `#error`). Both libraries compiled, so
  these held; the tests assert the runtime invariant each one protects.

### Result

No genuine C-vs-Rust divergence was found on any error path. All 372 tests pass.
