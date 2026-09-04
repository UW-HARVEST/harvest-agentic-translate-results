# ERRORS.md — error surface of the C library

**Status: all 236 rows accounted for.** The `verified by` column names the
differential test that constructs the exact invalid input, calls BOTH the C
`liblz4.so` and the Rust `liblz4.so` through `libloading`, and asserts that the
*same* sentinel / error code / `LZ4F_getErrorCode()` enum / `LZ4F_getErrorName()`
string comes back from both. Rows that are not testable are labelled explicitly:

* **UB in C** — the C code has no check at all and reads/writes out of bounds
  (confirmed by running the C `.so` standalone: it segfaults). There is no return
  value to compare, so these are documented rather than tested.
* **assert-guarded** — `c_src/CMakeLists.txt` sets no build type, so `NDEBUG` is
  *not* defined and `assert()` is live; the process aborts before the documented
  error return is reached.
* **unreachable** — allocation-failure branches. Both libraries call the same
  `malloc`, and the public API offers no allocator injection for these paths
  (`LZ4F_create*_advanced` is covered separately in `CONFIGS.md` row 111).
* **build-time only** — a `#error`, not a runtime rejection.

Run the error-path suites with:

```
cd translation && cargo test --offline --release --test errors_block --test errors_frame -- --test-threads=1
```

Derived mechanically from LZ4 v1.10.0 sources in `c_src/src/` and `c_src/include/`.
All line numbers refer to those files as shipped in this tree.

Conventions used in the "expected C result" column:

* `LZ4F_ERROR_x` means the function returns `(size_t)-(ptrdiff_t)LZ4F_ERROR_x`
  (see `lz4frame.c:311-316`, `lz4file.c:40-43`). Such a value tests true with
  `LZ4F_isError()` (`lz4frame.c:293-296`: `code > (size_t)(-LZ4F_ERROR_maxCode)`).
* `0` for the block API (`lz4.c` / `lz4hc.c` compressors) is the "compression
  failed / could not fit" sentinel.
* Negative for the block decompressors is the "malformed input" sentinel; the
  concrete value from `LZ4_decompress_generic` is
  `-(ip - src) - 1` (`lz4.c:2443`), i.e. it encodes the input offset at which the
  parse failed. `LZ4_decompress_unsafe_generic` always returns exactly `-1`.

| # | function | trigger | expected C result | source | verified by |
|---|----------|---------|-------------------|--------|-------------|
| 1 | `LZ4F_getBlockSize` | `blockSizeID` != 0 and outside `[LZ4F_max64KB(4) .. LZ4F_max4MB(7)]` (e.g. 1,2,3,8,9, or a garbage enum value crossing FFI) | `LZ4F_ERROR_maxBlockSize_invalid` | lz4frame.c:338-339 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 2 | `LZ4F_getBlockSize` | `blockSizeID == 0` (`LZ4F_default`) | silently remapped to `LZ4F_max64KB` → returns 65536 (no error) | lz4frame.c:337 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 3 | `LZ4F_compressBound_internal` | `prefs.frameInfo.blockSizeID` out of range | `LZ4F_getBlockSize()` error code is **not** checked; the huge `(size_t)-2` is used as `blockSize` in the arithmetic → nonsense (but non-error-looking) bound returned | lz4frame.c:389-402 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 4 | `LZ4F_compressBound_internal` | `blockChecksumFlag` > 1 (out-of-range enum via FFI) | silently multiplied: `blockCRCSize = 4 * flag` → oversized bound; no validation | lz4frame.c:398 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 5 | `LZ4F_compressBound_internal` | `contentChecksumFlag` > 1 (out-of-range enum via FFI) | silently multiplied: `frameEnd = 4 + flag*4` → oversized bound; no validation | lz4frame.c:399 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 6 | `LZ4F_optimalBSID` | `requestedBSID` > `LZ4F_max4MB` (8+) and `srcSize` larger than every intermediate block size | loop falls through and **returns the invalid `requestedBSID` unchanged** — no validation | lz4frame.c:359-371 | tests/errors_frame.rs::errf_get_block_size, ::errf_invalid_prefs_enums |
| 7 | `LZ4F_compressFrame_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(srcSize, &prefs)` | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:456 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 8 | `LZ4F_compressFrame_usingCDict` | invalid `prefs.frameInfo.blockSizeID` (e.g. 8) | `LZ4F_getBlockSize()` result at the `srcSize <=` comparison is unchecked; error value compares as huge → `blockMode` forced to `LZ4F_blockIndependent`, invalid BSID propagates into `LZ4F_compressBegin_usingCDict` | lz4frame.c:448-451 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 9 | `LZ4F_compressFrame_usingCDict` | any error forwarded from `LZ4F_compressBegin_usingCDict` | that error code propagated verbatim (`FORWARD_IF_ERROR`) | lz4frame.c:458-459 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 10 | `LZ4F_compressFrame_usingCDict` | any error forwarded from `LZ4F_compressUpdate` | that error code propagated verbatim | lz4frame.c:463-464 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 11 | `LZ4F_compressFrame_usingCDict` | any error forwarded from `LZ4F_compressEnd` | that error code propagated verbatim | lz4frame.c:468-469 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 12 | `LZ4F_compressFrame` (LZ4F_HEAPMODE=1 build) | cctx allocation fails | `LZ4F_ERROR_allocation_failed` forwarded from `LZ4F_createCompressionContext` | lz4frame.c:491-492 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 13 | `LZ4F_createCDict_advanced` / `LZ4F_createCDict` | `LZ4F_malloc(sizeof(*cdict))` fails | `NULL` | lz4frame.c:542-544 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 14 | `LZ4F_createCDict_advanced` / `LZ4F_createCDict` | any of `dictContent` / `fastCtx` / `HCCtx` allocation fails | frees everything, returns `NULL` | lz4frame.c:550-557 | tests/errors_frame.rs::errf_context_lifecycle |
| 15 | `LZ4F_createCDict_advanced` | `dictSize > 64 KB` | silently truncated to the last 64 KB (no error) | lz4frame.c:546-549 | tests/errors_frame.rs::errf_context_lifecycle |
| 16 | `LZ4F_freeCDict` | `cdict == NULL` | no-op (free-on-NULL supported) | lz4frame.c:583 | tests/errors_frame.rs::errf_context_lifecycle |
| 17 | `LZ4F_createCompressionContext_advanced` | `LZ4F_calloc` fails | `NULL` | lz4frame.c:598-600 | tests/errors_frame.rs::errf_context_lifecycle |
| 18 | `LZ4F_createCompressionContext` | `LZ4F_compressionContextPtr == NULL` | `assert()` in debug; `LZ4F_ERROR_parameter_null` in release | lz4frame.c:620-622 | **assert-guarded** — the C library is built without `-DNDEBUG` (see `c_src/CMakeLists.txt`), so this input trips `assert()` and aborts the process before the documented `RETURN_ERROR_IF` can run. Not a comparable return value; documented, not tested. |
| 19 | `LZ4F_createCompressionContext` | context allocation fails (`*ptr == NULL`) | `LZ4F_ERROR_allocation_failed` | lz4frame.c:625 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 20 | `LZ4F_createCompressionContext` | `version != LZ4F_VERSION` | **not validated** — version is stored verbatim into `cctx->version` and never compared | lz4frame.c:603, 624 | tests/errors_frame.rs::errf_context_lifecycle |
| 21 | `LZ4F_freeCompressionContext` | `cctxPtr == NULL` | returns `LZ4F_OK_NoError` (free-on-NULL supported) | lz4frame.c:629-637 | tests/errors_frame.rs::errf_context_lifecycle |
| 22 | `LZ4F_compressBegin_internal` (and thus `LZ4F_compressBegin`, `_usingDict`, `_usingDictOnce`, `_usingCDict`) | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (19) — checked *before* the actual header size is known | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:700 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 23 | `LZ4F_compressBegin_internal` | internal LZ4/LZ4HC context allocation fails | `LZ4F_ERROR_allocation_failed` | lz4frame.c:714-722 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 24 | `LZ4F_compressBegin_internal` | `tmpBuff` allocation of `requiredBuffSize` fails | `LZ4F_ERROR_allocation_failed` | lz4frame.c:748-750 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 25 | `LZ4F_compressBegin_internal` | `dictBuffer != NULL` and `dictSize > INT_MAX` | `LZ4F_ERROR_parameter_invalid` | lz4frame.c:766-768 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 26 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.blockSizeID` out of range (e.g. 8) | **silently accepted**: `LZ4F_getBlockSize()`'s error return is assigned straight into `cctx->maxBlockSize` with no `LZ4F_isError` check → `maxBlockSize` becomes `(size_t)-2`; only bits `&_3BITS` are written to the BD byte, so the emitted frame header is *also* wrong | lz4frame.c:740, 794 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 27 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.blockMode` out of range (e.g. 2) | **silently accepted**: header stores `blockMode & _1BIT` (→0 = blockLinked) while all internal logic uses `== LZ4F_blockLinked` (→false = independent) ⇒ header/behaviour mismatch, no error | lz4frame.c:788, 743-744, 759 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 28 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.blockChecksumFlag` out of range (e.g. 2) | **silently accepted**: header stores `&_1BIT` (→0), but `LZ4F_makeBlock` writes a CRC and advances `((U32)crcFlag)*BFSize` = 8 bytes ⇒ corrupt frame, no error | lz4frame.c:789, 903-907 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 29 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.contentChecksumFlag` out of range (e.g. 2) | **silently accepted**: header stores `&_1BIT` (→0); `== LZ4F_contentChecksumEnabled` tests are false, so no checksum is computed/emitted (self-consistent, but bound from row 5 is inflated) | lz4frame.c:791, 1100, 1225 | tests/errors_frame.rs::errf_invalid_prefs_enums (field=2, v=2 and every other out-of-range value) |
| 30 | `LZ4F_compressBegin_internal` | `prefs.frameInfo.frameType` set to `LZ4F_skippableFrame` (or garbage) | **silently ignored** — `frameType` is never read on the compression path; a normal frame is always written | lz4frame.c:782-808 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 31 | `LZ4F_compressBegin_internal` | `prefs.compressionLevel` out of range (negative, or > `LZ4HC_CLEVEL_MAX`) | **never an error**: `< LZ4HC_CLEVEL_MIN` selects the fast codec (negative = acceleration), `> 12` is clamped downstream by `LZ4_setCompressionLevel`/`LZ4HC_getCLevelParams`. `LZ4F_ERROR_compressionLevel_invalid` is never produced | lz4frame.c:705, 711, 732, 763; lz4hc.c:113-115, 1614-1615 | tests/errors_frame.rs::errf_dst_too_small_begin_and_frame, ::errf_invalid_prefs_enums |
| 32 | `LZ4F_compressUpdateImpl` (via `LZ4F_compressUpdate` / `LZ4F_uncompressedUpdate`) | `cctxPtr->cStage != 1` (i.e. `LZ4F_compressBegin` was not called, or `LZ4F_compressEnd` already reset it) | `LZ4F_ERROR_compressionState_uninitialized` | lz4frame.c:1005 | tests/errors_frame.rs::errf_compress_state_machine |
| 33 | `LZ4F_compressUpdateImpl` | `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)` | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:1006-1007 | tests/errors_frame.rs::errf_compress_state_machine |
| 34 | `LZ4F_uncompressedUpdate` only | `blockCompression == LZ4B_UNCOMPRESSED` and `dstCapacity < srcSize` | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:1009-1010 | tests/errors_frame.rs::errf_compress_state_machine |
| 35 | `LZ4F_uncompressedUpdate` | used with `LZ4F_blockLinked` (documented as unsupported) | not validated; `assert(blockCompression == LZ4B_COMPRESSED)` fires only in debug builds, otherwise silently produces linked-mode dictionary bookkeeping over uncompressed blocks | lz4frame.c:1069-1071 | tests/errors_frame.rs::errf_compress_state_machine |
| 36 | `LZ4F_compressUpdateImpl` | block-compress-mode switch requires an implicit `LZ4F_flush` and dst is too small for it | `LZ4F_ERROR_dstMaxSize_tooSmall` from the nested `LZ4F_flush`, but the value is **added to `dstPtr` unchecked** (no `FORWARD_IF_ERROR`) — a latent bug: the error is not propagated as an error | lz4frame.c:1013-1017 | tests/errors_frame.rs::errf_compress_state_machine |
| 37 | `LZ4F_flush` | `cctxPtr->tmpInSize == 0` | returns `0` (success, "nothing to flush") — checked *before* the state check | lz4frame.c:1167 | tests/errors_frame.rs::errf_compress_state_machine |
| 38 | `LZ4F_flush` | `cctxPtr->cStage != 1` and there is buffered data | `LZ4F_ERROR_compressionState_uninitialized` | lz4frame.c:1168 | tests/errors_frame.rs::errf_compress_state_machine |
| 39 | `LZ4F_flush` | `dstCapacity < tmpInSize + LZ4F_BLOCK_HEADER_SIZE(4) + LZ4F_BLOCK_CHECKSUM_SIZE(4)` | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:1169 | tests/errors_frame.rs::errf_compress_state_machine |
| 40 | `LZ4F_compressEnd` | error forwarded from the internal `LZ4F_flush` | that error code propagated verbatim | lz4frame.c:1213-1215 | tests/errors_frame.rs::errf_compress_state_machine |
| 41 | `LZ4F_compressEnd` | remaining `dstCapacity < 4` (endMark) after flush | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:1221 | tests/errors_frame.rs::errf_compress_state_machine |
| 42 | `LZ4F_compressEnd` | `contentChecksumFlag == LZ4F_contentChecksumEnabled` and remaining `dstCapacity < 8` | `LZ4F_ERROR_dstMaxSize_tooSmall` | lz4frame.c:1227 | tests/errors_frame.rs::errf_compress_state_machine |
| 43 | `LZ4F_compressEnd` | `prefs.frameInfo.contentSize != 0` and `contentSize != totalInSize` (caller announced a size then fed a different amount) | `LZ4F_ERROR_frameSize_wrong` (note: `cStage` has already been reset to 0 at this point) | lz4frame.c:1233-1238 | tests/errors_frame.rs::errf_compress_state_machine |
| 44 | `LZ4F_createDecompressionContext_advanced` | `LZ4F_calloc` fails | `NULL` | lz4frame.c:1286-1287 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 45 | `LZ4F_createDecompressionContext` | `LZ4F_decompressionContextPtr == NULL` | `assert()` in debug; `LZ4F_ERROR_parameter_null` in release | lz4frame.c:1303-1304 | **assert-guarded** — the C library is built without `-DNDEBUG` (see `c_src/CMakeLists.txt`), so this input trips `assert()` and aborts the process before the documented `RETURN_ERROR_IF` can run. Not a comparable return value; documented, not tested. |
| 46 | `LZ4F_createDecompressionContext` | dctx allocation fails | `LZ4F_ERROR_allocation_failed` | lz4frame.c:1307-1309 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 47 | `LZ4F_createDecompressionContext` | `versionNumber != LZ4F_VERSION` | **not validated** — stored verbatim, never compared | lz4frame.c:1290, 1306 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 48 | `LZ4F_freeDecompressionContext` | `dctx == NULL` | returns `LZ4F_OK_NoError` | lz4frame.c:1313-1323 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 49 | `LZ4F_freeDecompressionContext` | frame decoding was interrupted mid-frame | returns `(LZ4F_errorCode_t)dctx->dStage`, a **non-zero, non-error-encoded** value that `LZ4F_isError()` reports as *not* an error | lz4frame.c:1316-1317 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 50 | `LZ4F_decodeHeader` | `srcSize < LZ4F_HEADER_SIZE_MIN` (7) | `LZ4F_ERROR_frameHeader_incomplete` | lz4frame.c:1354 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 51 | `LZ4F_decodeHeader` | magic number is neither `LZ4F_MAGICNUMBER` nor in the skippable range `0x184D2A5X` | `LZ4F_ERROR_frameType_unknown` (compiled out under `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION`) | lz4frame.c:1372-1375 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 52 | `LZ4F_decodeHeader` | FLG bit 1 (reserved) set | `LZ4F_ERROR_reservedFlag_set` | lz4frame.c:1388 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 53 | `LZ4F_decodeHeader` | FLG version field `(FLG>>6)&3 != 1` | `LZ4F_ERROR_headerVersion_wrong` | lz4frame.c:1389 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 54 | `LZ4F_decodeHeader` | BD bit 7 (reserved) set | `LZ4F_ERROR_reservedFlag_set` | lz4frame.c:1409 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 55 | `LZ4F_decodeHeader` | BD `blockSizeID = (BD>>4)&7 < 4` (values 0..3 unsupported on the wire) | `LZ4F_ERROR_maxBlockSize_invalid` | lz4frame.c:1410 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 56 | `LZ4F_decodeHeader` | BD low nibble (reserved bits 0-3) non-zero | `LZ4F_ERROR_reservedFlag_set` | lz4frame.c:1411 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 57 | `LZ4F_decodeHeader` | header checksum byte != `XXH32(header+4, hdrSize-5, 0) >> 8` | `LZ4F_ERROR_headerChecksum_invalid` (compiled out under `FUZZING_BUILD_MODE_...`) | lz4frame.c:1417-1418 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 58 | `LZ4F_decodeHeader` | `srcSize < frameHeaderSize` (partial header) | **not an error**: buffers what it has, sets `dstage_storeFrameHeader`, returns `srcSize` | lz4frame.c:1396-1404 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 59 | `LZ4F_headerSize` | `src == NULL` | `LZ4F_ERROR_srcPtr_wrong` | lz4frame.c:1446 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 60 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (5) | `LZ4F_ERROR_frameHeader_incomplete` | lz4frame.c:1449-1450 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 61 | `LZ4F_headerSize` | bad magic number (not LZ4F, not skippable) | `LZ4F_ERROR_frameType_unknown` | lz4frame.c:1458-1459 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 62 | `LZ4F_getFrameInfo` | called while `dctx->dStage == dstage_storeFrameHeader` (header only partially fed) | `*srcSizePtr = 0`, `LZ4F_ERROR_frameDecoding_alreadyStarted` | lz4frame.c:1498-1501 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 63 | `LZ4F_getFrameInfo` | `LZ4F_headerSize(srcBuffer, *srcSizePtr)` fails | `*srcSizePtr = 0`, that error forwarded | lz4frame.c:1503-1504 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 64 | `LZ4F_getFrameInfo` | `*srcSizePtr < hSize` (not enough bytes for the full header) | `*srcSizePtr = 0`, `LZ4F_ERROR_frameHeader_incomplete` | lz4frame.c:1505-1508 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 65 | `LZ4F_getFrameInfo` | `LZ4F_decodeHeader` fails | `*srcSizePtr = 0`, that error forwarded | lz4frame.c:1510-1513 | tests/errors_frame.rs::errf_decode_header_rejections, ::errf_header_size |
| 66 | `LZ4F_decompress` | `dstBuffer == NULL` but `*dstSizePtr != 0` | `assert()` only (debug); in release `dstEnd` stays `NULL` and no output is produced | lz4frame.c:1623, 1632 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 67 | `LZ4F_decompress` (`dstage_getFrameHeader`) | header decode error with >= 19 bytes available | error from `LZ4F_decodeHeader` forwarded | lz4frame.c:1650-1651 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 68 | `LZ4F_decompress` (`dstage_storeFrameHeader`) | header decode error after buffering | error from `LZ4F_decodeHeader` forwarded | lz4frame.c:1673 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 69 | `LZ4F_decompress` (`dstage_init`) | `tmpIn` allocation of `maxBlockSize + 4` fails | `LZ4F_ERROR_allocation_failed` | lz4frame.c:1685-1686 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 70 | `LZ4F_decompress` (`dstage_init`) | `tmpOutBuffer` allocation of `bufferNeeded` fails | `LZ4F_ERROR_allocation_failed` | lz4frame.c:1687-1689 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 71 | `LZ4F_decompress` (block header) | `blockHeader & 0x7FFFFFFF > dctx->maxBlockSize` (block claims to be larger than the frame's declared max block size) | `LZ4F_ERROR_maxBlockSize_invalid` | lz4frame.c:1737-1739 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 72 | `LZ4F_decompress` (block header) | `blockHeader == 0` | **not an error**: end-of-frame marker → `dstage_getSuffix` | lz4frame.c:1732-1736 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 73 | `LZ4F_decompress` (`dstage_getBlockChecksum`, uncompressed block) | stored block CRC != `XXH32` of the transferred bytes | `LZ4F_ERROR_blockChecksum_invalid` (skipped if `decompressOptions.skipChecksums`; compiled out under `FUZZING_BUILD_MODE_...`) | lz4frame.c:1821-1830 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 74 | `LZ4F_decompress` (compressed block) | stored block CRC != `XXH32(selectedIn, tmpInTarget, 0)` | `LZ4F_ERROR_blockChecksum_invalid` | lz4frame.c:1875-1878 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 75 | `LZ4F_decompress` (decode straight into dst) | `LZ4_decompress_safe_usingDict` returns < 0 (malformed block) | `LZ4F_ERROR_decompressionFailed` | lz4frame.c:1901-1905 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 76 | `LZ4F_decompress` (decode into `tmpOut`) | `LZ4_decompress_safe_usingDict` returns < 0 | `LZ4F_ERROR_decompressionFailed` | lz4frame.c:1946-1950 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 77 | `LZ4F_decompress` (`dstage_getSuffix`) | frame declared a `contentSize` and `dctx->frameRemainingSize != 0` at end of frame | `LZ4F_ERROR_frameSize_wrong` | lz4frame.c:1984 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 78 | `LZ4F_decompress` (suffix check) | stored content checksum != `XXH32_digest(&dctx->xxh)` | `LZ4F_ERROR_contentChecksum_invalid` (skipped if `skipChecksums`; compiled out under `FUZZING_BUILD_MODE_...`) | lz4frame.c:2016-2021 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 79 | `LZ4F_decompress` | `dictSize > 1 GB` on the linked-block dictionary path | silently truncated to the last 64 KB (guards the `(int)` cast) | lz4frame.c:1896-1900, 1941-1945 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 80 | `LZ4F_decompress_usingDict` | called after decoding started (`dStage > dstage_init`) | dict arguments are **silently ignored** (no error) | lz4frame.c:2129-2132 | tests/errors_frame.rs::errf_frame_body_rejections, ::errf_decompress_using_dict_edges, ::errf_decompress_zero_dst, ::errf_decode_header_rejections |
| 81 | `LZ4F_readOpen` | `fp == NULL` or `lz4fRead == NULL` | `LZ4F_ERROR_parameter_null` | lz4file.c:79-81 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 82 | `LZ4F_readOpen` | `calloc(1, sizeof(LZ4_readFile_t))` fails | `LZ4F_ERROR_allocation_failed` | lz4file.c:83-86 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 83 | `LZ4F_readOpen` | `LZ4F_createDecompressionContext` fails | frees state, `*lz4fRead = NULL`, forwards that error | lz4file.c:88-92 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 84 | `LZ4F_readOpen` | `fread` returns fewer than `LZ4F_HEADER_SIZE_MAX` (19) bytes — i.e. any file shorter than 19 bytes, even a valid short frame | frees state, `LZ4F_ERROR_io_read` | lz4file.c:95-99 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 85 | `LZ4F_readOpen` | `LZ4F_getFrameInfo` fails (bad magic, bad checksum, reserved bits, …) | frees state, forwards that error | lz4file.c:101-106 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 86 | `LZ4F_readOpen` | decoded `info.blockSizeID` not in {0,4,5,6,7} | frees state, `LZ4F_ERROR_maxBlockSize_invalid` (unreachable in practice: `LZ4F_decodeHeader` already rejects `<4` and the field is 3 bits so `>7` is impossible) | lz4file.c:108-125 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 87 | `LZ4F_readOpen` | `malloc(srcBufMaxSize)` fails | frees state, `LZ4F_ERROR_allocation_failed` | lz4file.c:128-132 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 88 | `LZ4F_read` | `lz4fRead == NULL` or `buf == NULL` | `LZ4F_ERROR_parameter_null` (returned through a `size_t` return type) | lz4file.c:145-146 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 89 | `LZ4F_read` | `fread` "returns < 0" | `LZ4F_ERROR_io_read` — **dead code**: `ret` is `size_t`, so the `else` branch is unreachable; a real read error is indistinguishable from EOF and returns a short count | lz4file.c:154-163 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 90 | `LZ4F_read` | `LZ4F_decompress` fails (any frame error) | that error code returned verbatim; `srcBufNext`/`next` are left un-advanced | lz4file.c:166-173 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 91 | `LZ4F_readClose` | `lz4fRead == NULL` | `LZ4F_ERROR_parameter_null` | lz4file.c:185-186 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 92 | `LZ4F_writeOpen` | `fp == NULL` or `lz4fWrite == NULL` | `LZ4F_ERROR_parameter_null` | lz4file.c:222-223 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 93 | `LZ4F_writeOpen` | `calloc(1, sizeof(LZ4_writeFile_t))` fails | `LZ4F_ERROR_allocation_failed` | lz4file.c:225-228 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 94 | `LZ4F_writeOpen` | `prefsPtr != NULL` and `prefsPtr->frameInfo.blockSizeID` not in {0,4,5,6,7} | frees state, `LZ4F_ERROR_maxBlockSize_invalid` — **this is the only place an out-of-range `blockSizeID` from a caller is validated up front** | lz4file.c:229-247 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 95 | `LZ4F_writeOpen` | `malloc(dstBufMaxSize)` fails | frees state, `LZ4F_ERROR_allocation_failed` | lz4file.c:252-257 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 96 | `LZ4F_writeOpen` | `LZ4F_createCompressionContext` fails | frees state, forwards that error | lz4file.c:259-263 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 97 | `LZ4F_writeOpen` | `LZ4F_compressBegin` fails (e.g. internal allocation failure) | frees state, forwards that error | lz4file.c:265-269 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 98 | `LZ4F_writeOpen` | `fwrite` of the frame header writes fewer bytes than requested | frees state, `LZ4F_ERROR_io_write` | lz4file.c:271-274 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 99 | `LZ4F_write` | `lz4fWrite == NULL` or `buf == NULL` | `LZ4F_ERROR_parameter_null` | lz4file.c:288-289 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 100 | `LZ4F_write` | `LZ4F_compressUpdate` fails | error latched into `lz4fWrite->errCode` **and** returned | lz4file.c:296-303 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 101 | `LZ4F_write` | short `fwrite` of the compressed chunk | `LZ4F_ERROR_io_write` latched into `errCode` and returned | lz4file.c:305-308 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 102 | `LZ4F_writeClose` | `lz4fWrite == NULL` | `LZ4F_ERROR_parameter_null` | lz4file.c:321-323 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 103 | `LZ4F_writeClose` | a previous `LZ4F_write` latched an error in `errCode` | `LZ4F_compressEnd` is **skipped** (frame left unterminated on disk); state is freed and `LZ4F_OK_NoError` is returned — the earlier failure is swallowed | lz4file.c:325-340 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 104 | `LZ4F_writeClose` | `LZ4F_compressEnd` fails | `goto out`: state freed, that error returned | lz4file.c:326-331 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 105 | `LZ4F_writeClose` | short `fwrite` of the frame footer | `LZ4F_ERROR_io_write` | lz4file.c:333-335 | tests/lz4file_diff.rs::file_error_paths, ::file_write_matrix, ::file_read_matrix, ::file_round_trip |
| 106 | `LZ4_compressBound` | `isize < 0` or `isize > LZ4_MAX_INPUT_SIZE` (0x7E000000) | `0` (macro `LZ4_COMPRESSBOUND` compares as `unsigned`) | lz4.c:751; lz4.h:214-215 | tests/errors_block.rs::err_bound_out_of_range |
| 107 | `LZ4_compress_generic` (all `LZ4_compress_*` entry points) | `(U32)srcSize > LZ4_MAX_INPUT_SIZE` — including any negative `srcSize` | `0` | lz4.c:1360 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 108 | `LZ4_compress_generic` | `srcSize == 0` and `dstCapacity <= 0` (limited/fillOutput modes) | `0` | lz4.c:1361-1362 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 109 | `LZ4_compress_generic` | `srcSize == 0`, `src == NULL`, `dstCapacity >= 1` | `1` (writes a single zero token byte) — success, not an error | lz4.c:1361-1371 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 110 | `LZ4_compress_generic_validated` | `outputDirective == fillOutput` and `maxOutputSize < 1` | `0` | lz4.c:985 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 111 | `LZ4_compress_generic_validated` | `limitedOutput` and the literal run would overflow `dst` | `0` (hash table left populated but valid) | lz4.c:1113-1116 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 112 | `LZ4_compress_generic_validated` | `limitedOutput` and the match-length encoding would overflow `dst` | `0` | lz4.c:1207-1210 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 113 | `LZ4_compress_generic_validated` | `limitedOutput` and the last-literals run would overflow `dst` | `0` | lz4.c:1311-1314 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 114 | `LZ4_compress_default` / `LZ4_compress_fast` | `dstCapacity < LZ4_compressBound(srcSize)` and the data is incompressible enough not to fit | `0` | lz4.c:1472-1475, 1395-1401, 1116 | tests/errors_block.rs::err_compress_src_size_and_empty, ::err_compress_dst_too_small |
| 115 | `LZ4_compress_fast` (LZ4_HEAPMODE=1) | `ALLOC(sizeof(LZ4_stream_t))` fails | `0` | lz4.c:1456-1458 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 116 | `LZ4_compress_fast` / `_extState` / `_fastReset` / `_continue` | `acceleration < 1` | silently replaced by `LZ4_ACCELERATION_DEFAULT` (1) — no error | lz4.c:1386, 1417, 1719 | tests/errors_block.rs::err_decompress_continue_failures |
| 117 | `LZ4_compress_fast` / `_extState` / `_fastReset` / `_continue` | `acceleration > LZ4_ACCELERATION_MAX` (65537) | silently clamped to 65537 — no error | lz4.c:1387, 1418, 1720 | tests/errors_block.rs::err_decompress_continue_failures |
| 118 | `LZ4_compress_fast_extState` | `state` misaligned or smaller than `sizeof(LZ4_stream_t)` | `LZ4_initStream` returns `NULL`, the code dereferences it → **UB/crash** (only `assert(ctx != NULL)` guards it) | lz4.c:1384-1385 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 119 | `LZ4_compress_destSize` (LZ4_HEAPMODE=1) | `ALLOC(sizeof(LZ4_stream_t))` fails | `0` | lz4.c:1508-1510 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 120 | `LZ4_compress_destSize` / `_extState` | `targetDstSize < 1` | `0` and `*srcSizePtr` untouched (via the `fillOutput` guard) | lz4.c:1490-1493, 985 | tests/errors_block.rs::err_dest_size_edges; tests/lz4_block.rs::block_compress_dest_size |
| 121 | `LZ4_compress_destSize` | `targetDstSize` smaller than needed for all of `*srcSizePtr` | **not an error**: `*srcSizePtr` is rewritten with the number of bytes actually consumed, return is the compressed size | lz4.c:1490-1493, 1331-1332 | tests/errors_block.rs::err_dest_size_edges; tests/lz4_block.rs::block_compress_dest_size |
| 122 | `LZ4_createStream` | `ALLOC(sizeof(LZ4_stream_t))` fails | `NULL` | lz4.c:1533-1536 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 123 | `LZ4_initStream` | `buffer == NULL` | `NULL` | lz4.c:1555 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 124 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` | `NULL` | lz4.c:1556 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 125 | `LZ4_initStream` | `buffer` not aligned to `LZ4_stream_t_alignment()` (only enforced when `LZ4_ALIGN_TEST` is on; otherwise alignment is 1 and the check never fires) | `NULL` | lz4.c:1557, 1542-1550 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 126 | `LZ4_freeStream` | `LZ4_stream == NULL` | `0` (free-on-NULL supported) | lz4.c:1577 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 127 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < sizeof(reg_t)` (8 on 64-bit / 4 on 32-bit) — including `dictSize == 0` | `0`, and the stream is left reset with `dictionary`/`dictSize` cleared (dictionary effectively discarded) | lz4.c:1613-1615 | tests/errors_block.rs::err_load_save_dict_edges; tests/lz4_stream.rs::stream_load_dict_variants, ::stream_attach_dictionary |
| 128 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize > 64 KB` | silently truncated to the last 64 KB; returns 65536 | lz4.c:1617-1619, 1645 | tests/errors_block.rs::err_load_save_dict_edges; tests/lz4_stream.rs::stream_load_dict_variants, ::stream_attach_dictionary |
| 129 | `LZ4_loadDict` / `LZ4_loadDictSlow` | `dictSize < 0` | not validated; `dictEnd = p + dictSize` underflows → UB | lz4.c:1594, 1613 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 130 | `LZ4_attach_dictionary` | `dictionaryStream == NULL`, or its `dictSize == 0` | dictionary silently detached (`dictCtx = NULL`); no error | lz4.c:1660-1683 | tests/errors_block.rs::err_load_save_dict_edges; tests/lz4_stream.rs::stream_load_dict_variants, ::stream_attach_dictionary |
| 131 | `LZ4_compress_fast_continue` | `streamPtr->dictSize < 4` and not in prefix mode | dictionary silently dropped so faster prefix mode can be used; no error | lz4.c:1722-1733 | tests/errors_block.rs::err_decompress_continue_failures |
| 132 | `LZ4_compress_fast_continue` | src overlaps the current dictionary region | dictionary silently shrunk (and zeroed if it drops below 4 bytes); no error | lz4.c:1735-1743 | tests/errors_block.rs::err_decompress_continue_failures |
| 133 | `LZ4_compress_fast_continue` | `dstCapacity` too small for the compressed output | `0` (always uses `limitedOutput`) | lz4.c:1746-1782, 1116 | tests/errors_block.rs::err_decompress_continue_failures |
| 134 | `LZ4_saveDict` | `dictSize > 64 KB` | silently clamped to 65536 | lz4.c:1820 | tests/errors_block.rs::err_load_save_dict_edges; tests/lz4_stream.rs::stream_load_dict_variants, ::stream_attach_dictionary |
| 135 | `LZ4_saveDict` | `dictSize > dict->dictSize` (asking for more history than exists) | silently clamped to `dict->dictSize`; the clamped value is returned (may be `0`) | lz4.c:1821, 1833 | tests/errors_block.rs::err_load_save_dict_edges; tests/lz4_stream.rs::stream_load_dict_variants, ::stream_attach_dictionary |
| 136 | `LZ4_saveDict` | `safeBuffer == NULL` with a non-zero effective `dictSize` | `assert()` in debug only; in release `memmove` to `NULL` → **UB** | lz4.c:1823-1828 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 137 | `LZ4_decompress_unsafe_generic` (`LZ4_decompress_fast`, `LZ4_decompress_fast_withPrefix64k`, `LZ4_uncompress`) | literal length exceeds remaining output space | `-1` | lz4.c:1898 | tests/errors_block.rs::err_decompress_fast_negative |
| 138 | `LZ4_decompress_unsafe_generic` | literals end < `MFLIMIT` (12) bytes before end of block and not exactly at `oend` | `-1` | lz4.c:1902-1907 | tests/errors_block.rs::err_decompress_fast_negative |
| 139 | `LZ4_decompress_unsafe_generic` | match length exceeds remaining output space | `-1` | lz4.c:1921 | tests/errors_block.rs::err_decompress_fast_negative |
| 140 | `LZ4_decompress_unsafe_generic` | match offset > `(op - prefixStart) + dictSize` (points outside all buffers) | `-1` | lz4.c:1926-1929 | tests/errors_block.rs::err_decompress_fast_negative |
| 141 | `LZ4_decompress_unsafe_generic` | a match ends < `LASTLITERALS` (5) bytes before end of block | `-1` | lz4.c:1957-1962 | tests/errors_block.rs::err_decompress_fast_negative |
| 142 | `LZ4_decompress_fast` family | truncated input (input size is not known to this decoder) | **no defense**: reads past the end of `src` → out-of-bounds read / UB. These entry points are deprecated for exactly this reason | lz4.c:1860-1868 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 143 | `read_variable_length` | `initial_check` and `*ip >= ilimit` before the loop | `rvl_error` ((size_t)-1) → caller jumps to `_output_error` | lz4.c:1986-1988 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 144 | `read_variable_length` | `*ip > ilimit` after consuming a length byte | `rvl_error` | lz4.c:1992-1994, 2004-2006 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 145 | `read_variable_length` | 32-bit build, accumulated `length > SIZE_MAX/2` | `rvl_error` | lz4.c:1996-1998, 2008-2010 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 146 | `LZ4_decompress_generic` (all `LZ4_decompress_safe*`) | `src == NULL` | `-1` | lz4.c:2036 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 147 | `LZ4_decompress_generic` | `outputSize < 0` (negative `dstCapacity`/`maxDecompressedSize`) | `-1` | lz4.c:2036 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 148 | `LZ4_decompress_generic` | `outputSize == 0`, `partialDecoding` | `0` (success) | lz4.c:2064-2066 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 149 | `LZ4_decompress_generic` | `outputSize == 0`, full-block decode, input is *not* exactly the 1-byte empty block `{0x00}` | `-1` | lz4.c:2064-2067 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 150 | `LZ4_decompress_generic` | `srcSize == 0` with `outputSize > 0` | `-1` | lz4.c:2069 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 151 | `LZ4_decompress_generic` | long literal length unreadable within input (fast loop) | `_output_error` → `-(ip-src)-1` | lz4.c:2092-2097 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 152 | `LZ4_decompress_generic` | pointer overflow while adding the literal length to `op` or `ip` (fast loop) | `_output_error` | lz4.c:2099-2100 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 153 | `LZ4_decompress_generic` | long match length unreadable within input (fast loop) | `_output_error` | lz4.c:2127-2132 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 154 | `LZ4_decompress_generic` | pointer overflow while adding the match length to `op` (fast loop) | `_output_error` | lz4.c:2136 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 155 | `LZ4_decompress_generic` | `checkOffset` and `match + dictSize < lowPrefix` — offset points before all available history (fast loop) | `_output_error` | lz4.c:2161-2164 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 156 | `LZ4_decompress_generic` | extDict match would write past `oend - LASTLITERALS`, full-block mode (fast loop) | `_output_error` | lz4.c:2166-2175 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 157 | `LZ4_decompress_generic` | long literal length unreadable within input (safe loop) | `_output_error` | lz4.c:2264-2266 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 158 | `LZ4_decompress_generic` | pointer overflow on literal length (safe loop) | `_output_error` | lz4.c:2268-2269 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 159 | `LZ4_decompress_generic` | full-block mode, final literal run does not consume the input exactly (`ip+length != iend`) or would overflow `oend` | `_output_error` | lz4.c:2308-2318 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 160 | `LZ4_decompress_generic` | long match length unreadable within input (safe loop) | `_output_error` | lz4.c:2345-2347 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 161 | `LZ4_decompress_generic` | pointer overflow on match length (safe loop) | `_output_error` | lz4.c:2349 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 162 | `LZ4_decompress_generic` | `checkOffset` and `match + dictSize < lowPrefix` (safe loop / `safe_match_copy`) | `_output_error` | lz4.c:2356 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 163 | `LZ4_decompress_generic` | extDict match violates the end-of-block parsing restriction, full-block mode (safe loop) | `_output_error` | lz4.c:2360-2362 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 164 | `LZ4_decompress_generic` | a match would end within the last `LASTLITERALS` (5) bytes of the output block | `_output_error` | lz4.c:2421-2423 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 165 | `LZ4_decompress_safe` | `compressedSize` larger than the real buffer | not detectable by the library; `iend` is trusted → out-of-bounds read. `compressedSize < 0` makes `iend < ip` and the parse fails with a negative return | lz4.c:2039, 2451-2456 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 166 | `LZ4_decompress_safe_partial` | `targetOutputSize` or `dstCapacity` negative | `dstCapacity = MIN(...)` stays negative → `outputSize < 0` → `-1` | lz4.c:2459-2464, 2036 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 167 | `LZ4_decompress_safe_partial` | `targetOutputSize > dstCapacity` | silently clamped to `dstCapacity` (this is why the 4-arg form is unsafe) | lz4.c:2461 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 168 | `LZ4_decompress_safe_partial` | input truncated mid-literals or mid-match | **not an error**: partial mode clamps `length` and returns the bytes produced so far | lz4.c:2285-2307, 2392-2404 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 169 | `LZ4_freeStreamDecode` | `LZ4_stream == NULL` | `0` | lz4.c:2577 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 170 | `LZ4_setStreamDecode` | any input, including `dictionary == NULL` with `dictSize != 0`, or negative `dictSize` (cast to `size_t`) | **always returns 1**; the documented `0 == error` return is unreachable. `dictionary == NULL && dictSize != 0` trips only a debug `assert` | lz4.c:2589-2602 | tests/errors_block.rs::err_free_on_null |
| 171 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` | lz4.c:2617 | tests/errors_block.rs::err_bound_out_of_range; tests/lz4_block.rs::block_scalar_accessors |
| 172 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` | lz4.c:2618 | tests/errors_block.rs::err_bound_out_of_range; tests/lz4_block.rs::block_scalar_accessors |
| 173 | `LZ4_decoderRingBufferSize` | `0 <= maxBlockSize < 16` | silently raised to 16 → returns `65536 + 14 + 16` | lz4.c:2619-2620; lz4.h:491 | tests/errors_block.rs::err_bound_out_of_range; tests/lz4_block.rs::block_scalar_accessors |
| 174 | `LZ4_decompress_safe_continue` | underlying decode returns `<= 0` (malformed block, or a legitimately empty block) | that value returned and the stream state is **left un-advanced** — note a valid 0-length block is treated like an error here | lz4.c:2636-2665 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 175 | `LZ4_decompress_fast_continue` | underlying decode returns `<= 0` | that value returned, stream state un-advanced | lz4.c:2681-2706 | tests/errors_block.rs::err_decompress_fast_negative |
| 176 | `LZ4_decompress_safe_usingDict` | `dictSize == 0` | dictionary silently ignored, falls back to `LZ4_decompress_safe` | lz4.c:2721-2722 | tests/errors_block.rs::err_decompress_safe_malformed, ::err_decompress_using_dict_edges |
| 177 | `LZ4_decompress_safe_usingDict` | `dictSize < 0` | `assert()` only; in release the negative value is cast to `size_t` → huge `dictSize` → UB | lz4.c:2727-2731 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 178 | `LZ4_resetStreamState` (obsolete) | any input | always `0`; no validation of `state` size/alignment | lz4.c:2808-2813 | tests/errors_block.rs::err_init_stream_rejections, ::err_free_on_null |
| 179 | `LZ4HC_getCLevelParams` | `cLevel < 1` | silently replaced by `LZ4HC_CLEVEL_DEFAULT` (9) | lz4hc.c:112-113 | tests/lz4hc_diff.rs::hc_compress_all_levels, ::hc_stream_init_reset |
| 180 | `LZ4HC_getCLevelParams` | `cLevel > LZ4HC_CLEVEL_MAX` (12) | silently clamped to 12 | lz4hc.c:114 | tests/lz4hc_diff.rs::hc_compress_all_levels, ::hc_stream_init_reset |
| 181 | `LZ4HC_encodeSequence` | `limit != notLimited` and there is no room for the literal run | `1` (→ caller's `_dest_overflow`) | lz4hc.c:304-308 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 182 | `LZ4HC_encodeSequence` | `limit != notLimited` and there is no room for the match-length bytes | `1` (→ `_dest_overflow`) | lz4hc.c:330-333 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 183 | `LZ4MID_compress` | `*srcSizePtr < 0` | `0` | lz4hc.c:559 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 184 | `LZ4MID_compress` | `maxOutputSize < 0` | `0` | lz4hc.c:560 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 185 | `LZ4MID_compress` | `*srcSizePtr > LZ4_MAX_INPUT_SIZE` | `0` | lz4hc.c:561-563 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 186 | `LZ4MID_compress` | `limit == limitedOutput` and the last literal run does not fit | `0` | lz4hc.c:711-714 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 187 | `LZ4MID_compress` | `_dest_overflow` reached with `limit != fillOutput` | `0` | lz4hc.c:747-772 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 188 | `LZ4HC_compress_hashChain` | `limit == limitedOutput` and the last literal run does not fit | `0` | lz4hc.c:1312-1315 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 189 | `LZ4HC_compress_hashChain` | `_dest_overflow` reached with `limit != fillOutput` | `0` | lz4hc.c:1340-1361 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 190 | `LZ4HC_compress_optimal` (LZ4HC_HEAPMODE=1) | `ALLOC` of the price table fails | `retval` stays `0` → returns `0` | lz4hc.c:1836-1856 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 191 | `LZ4HC_compress_optimal` | `limit == limitedOutput` and the last literal run does not fit | `retval = 0` → returns `0` | lz4hc.c:2064-2068 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 192 | `LZ4HC_compress_optimal` | `_dest_overflow` reached with `limit != fillOutput` | falls through to `_return_label` with `retval == 0` → returns `0` | lz4hc.c:2095-2120 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 193 | `LZ4HC_compress_generic_internal` | `limit == fillOutput` and `dstCapacity < 1` | `0` | lz4hc.c:1388 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 194 | `LZ4HC_compress_generic_internal` | `(U32)*srcSizePtr > LZ4_MAX_INPUT_SIZE` — including any negative `srcSize` | `0` | lz4hc.c:1389 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 195 | `LZ4HC_compress_generic_internal` | any failure (`result <= 0`) | `ctx->dirty = 1` is latched; the stream must be re-initialized before reuse (nothing enforces this) | lz4hc.c:1412 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 196 | `LZ4_compress_HC_extStateHC_fastReset` | `state` not aligned to `LZ4_streamHC_t_alignment()` (only enforced when `LZ4_ALIGN_TEST` is on) | `0` | lz4hc.c:1503 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 197 | `LZ4_compress_HC_extStateHC` | `LZ4_initStreamHC(state, sizeof(LZ4_streamHC_t))` returns `NULL` (NULL/too-small/misaligned state) | `0` | lz4hc.c:1514-1515 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 198 | `LZ4_compress_HC` (LZ4HC_HEAPMODE=1) | `ALLOC(sizeof(LZ4_streamHC_t))` fails | `0` | lz4hc.c:1522-1524 | **unreachable** — allocator failure cannot be injected through the public API (both libraries use the same `malloc`). |
| 199 | `LZ4_compress_HC` | `dstCapacity < LZ4_compressBound(srcSize)` and output does not fit | `0` | lz4hc.c:1506-1509, 1315 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 200 | `LZ4_compress_HC` | `compressionLevel` out of `[1..12]` | never an error: `<1` → 9, `>12` → 12 | lz4hc.c:112-115, 1614-1615 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 201 | `LZ4_compress_HC_destSize` | `LZ4_initStreamHC` returns `NULL` | `0` | lz4hc.c:1540-1541 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 202 | `LZ4_createStreamHC` | `ALLOC_AND_ZERO(sizeof(LZ4_streamHC_t))` fails | `NULL` | lz4hc.c:1555-1558 | tests/lz4hc_diff.rs::hc_stream_init_reset; tests/errors_block.rs::err_free_on_null |
| 203 | `LZ4_freeStreamHC` | `LZ4_streamHCPtr == NULL` | `0` | lz4hc.c:1566 | tests/lz4hc_diff.rs::hc_stream_init_reset; tests/errors_block.rs::err_free_on_null |
| 204 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` | lz4hc.c:1578 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 205 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` | `NULL` | lz4hc.c:1579 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 206 | `LZ4_initStreamHC` | `buffer` misaligned (only when `LZ4_ALIGN_TEST`) | `NULL` | lz4hc.c:1580 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 207 | `LZ4_setCompressionLevel` | `compressionLevel < 1` | silently set to `LZ4HC_CLEVEL_DEFAULT` (9) | lz4hc.c:1614 | tests/lz4hc_diff.rs::hc_compress_all_levels, ::hc_stream_init_reset |
| 208 | `LZ4_setCompressionLevel` | `compressionLevel > LZ4HC_CLEVEL_MAX` | silently clamped to 12 | lz4hc.c:1615 | tests/lz4hc_diff.rs::hc_compress_all_levels, ::hc_stream_init_reset |
| 209 | `LZ4_loadDictHC` | `dictSize > 64 KB` | silently truncated to the last 64 KB; returns 65536 | lz4hc.c:1634-1637, 1651 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 210 | `LZ4_loadDictHC` | `dictSize < LZ4HC_HASHSIZE` (4) with a non-`lz4mid` level | no hash insertion is performed (dictionary effectively useless) but `dictSize` is still returned; **no error** | lz4hc.c:1646-1651 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 211 | `LZ4_loadDictHC` | `dictSize < 0` | `assert()` only; in release `(size_t)dictSize` in `LZ4MID_fillHTable` → UB | lz4hc.c:1632, 1644-1645 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 212 | `LZ4_attach_HC_dictionary` | `dictionary_stream == NULL` | `dictCtx = NULL`, dictionary silently detached | lz4hc.c:1656-1658 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 213 | `LZ4_compressHC_continue_generic` | accumulated stream position > 2 GB | stream silently re-loaded from its last <= 64 KB via `LZ4_loadDictHC` (history beyond that is lost); no error | lz4hc.c:1694-1699 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 214 | `LZ4_compressHC_continue_generic` | src overlaps the extDict region | dictionary silently shrunk, and invalidated entirely if it drops below `LZ4HC_HASHSIZE`; no error | lz4hc.c:1705-1717 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 215 | `LZ4_compress_HC_continue` | `dstCapacity` too small for the output | `0` (`limitedOutput` path) | lz4hc.c:1722-1730 | tests/errors_block.rs::err_hc_dst_too_small; tests/lz4hc_diff.rs::hc_compress_limited_capacity |
| 216 | `LZ4_saveDictHC` | `dictSize > 64 KB` | silently clamped to 65536 | lz4hc.c:1748 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 217 | `LZ4_saveDictHC` | `dictSize < 4` | silently forced to `0` and **returns 0**; the stream's prefix is reset to `safeBuffer` with size 0 | lz4hc.c:1749 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 218 | `LZ4_saveDictHC` | `dictSize > prefixSize` (more history requested than exists) | silently clamped to `prefixSize`; clamped value returned | lz4hc.c:1750, 1763 | tests/lz4hc_diff.rs::hc_dictionaries; tests/gaps_diff.rs::gap_hc_ext_dict_and_ring |
| 219 | `LZ4_saveDictHC` | `safeBuffer == NULL` with a non-zero effective `dictSize` | `assert()` in debug; in release `memmove` to `NULL` → UB | lz4hc.c:1751-1753 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 220 | `LZ4_resetStreamStateHC` (obsolete) | `LZ4_initStreamHC` fails (NULL/too-small/misaligned state) | `1` (non-zero == error, inverted convention vs. the rest of the library) | lz4hc.c:2152-2153 | tests/errors_block.rs::err_init_stream_rejections, ::err_reset_stream_state_hc, ::err_hc_ext_state_misaligned |
| 221 | `LZ4_createHC` (obsolete) | `LZ4_createStreamHC()` fails | `NULL` | lz4hc.c:2161-2162 | tests/lz4hc_diff.rs::hc_stream_init_reset; tests/errors_block.rs::err_free_on_null |
| 222 | `LZ4_freeHC` (obsolete) | `LZ4HC_Data == NULL` | `0` | lz4hc.c:2169 | tests/lz4hc_diff.rs::hc_stream_init_reset; tests/errors_block.rs::err_free_on_null |
| 223 | `XXH32_createState` / `XXH64_createState` | `malloc` fails | `NULL` (no error code; caller must NULL-check) | xxhash.c:422-424, 883-885; 108 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 224 | `XXH32_freeState` / `XXH64_freeState` | `statePtr == NULL` | `XXH_OK` (`free(NULL)` is legal) | xxhash.c:426-429, 887-890 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 225 | `XXH32_reset` | `statePtr == NULL` | **no check**: `memcpy(NULL, &state, ...)` → UB/segfault. `XXH_ERROR` is *not* returned | xxhash.c:437-449 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 226 | `XXH64_reset` | `statePtr == NULL` | **no check**: `memcpy(NULL, ...)` → UB/segfault | xxhash.c:898-910 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 227 | `XXH32_update` | `input == NULL` (any `len`, including 0) with default `XXH_ACCEPT_NULL_INPUT_POINTER == 0` | `XXH_ERROR` | xxhash.c:454-458; 70-71 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 228 | `XXH32_update` | `input == NULL` when built with `XXH_ACCEPT_NULL_INPUT_POINTER >= 1` | `XXH_OK` (silently ignores the update) | xxhash.c:455-456 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 229 | `XXH64_update` | `input == NULL` (any `len`) with default settings | `XXH_ERROR` | xxhash.c:914-918 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 230 | `XXH32_update` / `XXH64_update` | `state_in == NULL` (non-NULL `input`) | **no check**: dereferenced immediately → UB | xxhash.c:459-511, 919-968 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 231 | `XXH32` / `XXH64` (one-shot) | `input == NULL` with `len != 0` (default build) | **no check** — the `p == NULL` fixup is compiled out when `XXH_ACCEPT_NULL_INPUT_POINTER == 0` → NULL dereference / UB. No error return exists (return type is a hash) | xxhash.c:358-362, 817-821; 70-71 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 232 | `XXH32` / `XXH64` (one-shot) | `input == NULL`, `len == 0` | still reads `p` in `XXH32_finalize`/`XXH64_finalize` with `len&15 == 0` ⇒ no dereference in practice; returns the seed-derived hash of an empty input | xxhash.c:365-388, 823-853 | tests/xxhash_diff.rs::xxh_error_paths, ::xxh_oneshot_all_lengths, ::xxh_streaming_random_splits, ::xxh_copy_state |
| 233 | `XXH32_digest` / `XXH64_digest` | `state_in == NULL` | no check → UB. Return type has no error channel | xxhash.c:546-554, 1005-1013 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 234 | `XXH32_copyState` / `XXH64_copyState` | either pointer `NULL` | no check → `memcpy` UB | xxhash.c:432-435, 893-896 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 235 | `XXH32_hashFromCanonical` / `XXH64_hashFromCanonical` | `src == NULL` | no check → UB | xxhash.c:572-575, 1025-1028 | **UB in C** — verified experimentally: the C library dereferences/derefs past bounds here, so there is no comparable return value. Documented, not tested. |
| 236 | (build-time) `LZ4_MEMORY_USAGE` | `< LZ4_MEMORY_USAGE_MIN` (10) or `> LZ4_MEMORY_USAGE_MAX` (20) | `#error` at compile time — not a runtime error surface, but it bounds `LZ4_STREAM_MINSIZE` / hash-table sizing | lz4.h:162-172, 695-729 | **build-time only** — not a runtime error surface. |

## `LZ4F_ERROR_*` codes that are UNREACHABLE from a normal caller

The `LZ4F_LIST_ERRORS` enum (`lz4frame.h:653-678`) declares 23 error codes. The
following are never produced by any `RETURN_ERROR` / `RETURN_ERROR_IF` in
`lz4frame.c` or `lz4file.c`, or are produced only on paths a normal caller
cannot reach:

* **`LZ4F_ERROR_GENERIC`** — no `RETURN_ERROR(GENERIC)` anywhere in either `.c`
  file. It exists only as a catch-all placeholder / index-1 slot.
* **`LZ4F_ERROR_blockMode_invalid`** — never emitted. `blockMode` is never
  range-checked: the compressor masks it with `_1BIT` when writing the FLG byte
  (`lz4frame.c:788`) and the decoder extracts a single bit
  (`lz4frame.c:1383`), so an out-of-range value can never be *detected*, only
  silently mangled (see row 27).
* **`LZ4F_ERROR_compressionLevel_invalid`** — never emitted. Every level is
  accepted: `< LZ4HC_CLEVEL_MIN` selects the fast codec (negative values become
  an acceleration factor), and `> LZ4HC_CLEVEL_MAX` is silently clamped by
  `LZ4HC_getCLevelParams` (`lz4hc.c:112-115`) and `LZ4_setCompressionLevel`
  (`lz4hc.c:1614-1615`). See row 31.
* **`LZ4F_ERROR_srcSize_tooLarge`** — never emitted by the frame layer. Oversize
  input is caught one layer down, where the block compressors return the `0`
  sentinel for `srcSize > LZ4_MAX_INPUT_SIZE` (`lz4.c:1360`, `lz4hc.c:1389`); the
  frame layer feeds at most `maxBlockSize` (<= 4 MB) per block, so the condition
  never arises there.
* **`LZ4F_ERROR_allocation_failed`** — reachable only if `malloc`/`calloc` (or a
  user-supplied `LZ4F_CustomMem` allocator) returns `NULL`
  (`lz4frame.c:625, 722, 750, 1308, 1686, 1689`; `lz4file.c:85, 131, 227, 256`).
  With the default allocator on a normal host this cannot be triggered through
  the API by input values alone — it is an environmental failure, not an input
  rejection. It is unreachable *by construction* in builds that define
  `LZ4_STATIC_LINKING_ONLY_DISABLE_MEMORY_ALLOCATION`.
* **`LZ4F_ERROR_parameter_null`** in `lz4frame.c` (lines 622, 1304) — guarded by
  a preceding `assert()` and commented "in case it nonetheless happen in
  production"; the API contract makes a `NULL` context-pointer-pointer a
  narrow-contract violation. It *is* genuinely reachable from `lz4file.c`
  (lines 80, 146, 186, 223, 289, 322), where NULL arguments are a supported
  runtime check.
* **`LZ4F_ERROR_maxBlockSize_invalid` at `lz4file.c:124`** — the `default:` arm
  of the `switch` on the *decoded* `info.blockSizeID`. `LZ4F_decodeHeader`
  already rejects wire values `< 4` (`lz4frame.c:1410`) and the field is only 3
  bits wide, so values `> 7` are impossible; this arm is dead code. (The
  corresponding arm at `lz4file.c:246`, which validates a *caller-supplied*
  `blockSizeID`, is reachable.)
* **`LZ4F_ERROR_io_read` at `lz4file.c:162`** — dead code: the guard is
  `else` after `if (ret > 0) ... else if (ret == 0)`, and `ret` is `size_t`, so
  the "negative" third case can never occur. Genuine `fread` errors are
  indistinguishable from EOF here. (`LZ4F_ERROR_io_read` at `lz4file.c:98` *is*
  reachable — any input file shorter than 19 bytes triggers it.)
* **`LZ4F_ERROR_frameType_unknown` / `LZ4F_ERROR_headerChecksum_invalid` /
  `LZ4F_ERROR_blockChecksum_invalid` / `LZ4F_ERROR_contentChecksum_invalid`** —
  reachable in normal builds, but *compiled out* when
  `FUZZING_BUILD_MODE_UNSAFE_FOR_PRODUCTION` is defined
  (`lz4frame.c:1371-1376, 1416-1420, 1824-1834, 1877-1882, 2020-2025`).
* **`LZ4F_ERROR_maxCode`** — sentinel only; it is the enum terminator used by
  `LZ4F_isError()` as the bound (`lz4frame.c:295`), never returned as a result.
