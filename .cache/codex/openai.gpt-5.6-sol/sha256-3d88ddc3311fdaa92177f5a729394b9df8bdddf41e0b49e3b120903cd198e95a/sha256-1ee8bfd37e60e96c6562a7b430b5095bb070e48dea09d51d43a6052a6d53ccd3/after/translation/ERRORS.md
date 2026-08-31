# Error surface

This table is derived from `RETURN_ERROR`, `RETURN_ERROR_IF`, explicit
error/sentinel returns, null/range checks, and public-contract assertions in
`src/*.c`. LZ4 frame error results are `-(size_t)LZ4F_ERROR_*`.

| # | function | trigger (the exact invalid input/condition) | expected C result |
|---|----------|----------------------------------------------|-------------------|
| 1 | `LZ4_compressBound` | `inputSize < 0` | `0` |
| 2 | `LZ4_compressBound` | `inputSize > LZ4_MAX_INPUT_SIZE` (`0x7E000000`) | `0` |
| 3 | block compression family | `(unsigned)srcSize > LZ4_MAX_INPUT_SIZE` (includes negative) | `0` |
| 4 | block compression family | `srcSize == 0 && dstCapacity <= 0` for a limited-output API | `0` |
| 5 | block compression family | destination budget cannot hold encoded block | `0` |
| 6 | `LZ4_compress_fast` | internal stream allocation fails | `0` |
| 7 | `LZ4_compress_destSize` | internal stream allocation fails | `0` |
| 8 | `LZ4_initStream` | `buffer == NULL` | `NULL` |
| 9 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` | `NULL` |
| 10 | `LZ4_initStream` | buffer not aligned to `LZ4_stream_t_alignment()` | `NULL` |
| 11 | `LZ4_createStream` | allocation fails | `NULL` |
| 12 | `LZ4_loadDict_internal` | `dictSize < HASH_UNIT` | `0` |
| 13 | `LZ4_decompress_safe` family | `src == NULL` | `-1` |
| 14 | `LZ4_decompress_safe` family | output capacity/size is negative | `-1` |
| 15 | `LZ4_decompress_safe` family | `compressedSize == 0` | `-1` |
| 16 | `LZ4_decompress_safe` family | output size is zero and input is not exactly byte `0` | `-1` |
| 17 | `LZ4_decompress_safe` family | literal run exceeds output buffer | negative decode position |
| 18 | `LZ4_decompress_safe` family | malformed block ends after a match instead of literals | negative decode position |
| 19 | `LZ4_decompress_safe` family | match length exceeds output buffer | negative decode position |
| 20 | `LZ4_decompress_safe` family | match offset is beyond prefix plus dictionary | negative decode position |
| 21 | `LZ4_decompress_fast` family | `src == NULL` or `originalSize < 0` | `-1` |
| 22 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` |
| 23 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` |
| 24 | HC compression family | source size negative or greater than `LZ4_MAX_INPUT_SIZE` | `0` |
| 25 | HC compression family | destination capacity negative | `0` |
| 26 | HC compression family | fill-output destination capacity `< 1` | `0` |
| 27 | HC compression family | destination budget cannot hold encoded block | `0` |
| 28 | `LZ4_compress_HC_extStateHC_fastReset` | state pointer is not correctly aligned | `0` |
| 29 | `LZ4_compress_HC_extStateHC` | state initialization fails | `0` |
| 30 | `LZ4_compress_HC` | stream allocation fails | `0` |
| 31 | `LZ4_compress_HC_destSize` | state initialization fails | `0` |
| 32 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` |
| 33 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` | `NULL` |
| 34 | `LZ4_initStreamHC` | buffer not aligned to `LZ4_streamHC_t_alignment()` | `NULL` |
| 35 | `LZ4_resetStreamStateHC` | `LZ4_initStreamHC` fails | `1` |
| 36 | `LZ4F_getBlockSize` | enum value is neither `0` nor `4..=7` | `ERROR_maxBlockSize_invalid` |
| 37 | `LZ4F_compressFrame[_usingCDict]` | `dstCapacity < LZ4F_compressFrameBound(...)` | `ERROR_dstMaxSize_tooSmall` |
| 38 | `LZ4F_createCompressionContext` | output context pointer is `NULL` | `ERROR_parameter_null` |
| 39 | `LZ4F_createCompressionContext` | context allocation fails | `ERROR_allocation_failed` |
| 40 | `LZ4F_compressBegin*` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` (`19`) | `ERROR_dstMaxSize_tooSmall` |
| 41 | `LZ4F_compressBegin_internal` | LZ4 context allocation fails | `ERROR_allocation_failed` |
| 42 | `LZ4F_compressBegin_internal` | temporary buffer allocation fails | `ERROR_allocation_failed` |
| 43 | `LZ4F_compressBegin_internal` | non-null dictionary has `dictSize > INT_MAX` | `ERROR_parameter_invalid` |
| 44 | `LZ4F_compressUpdate` | compression context stage is not `1` | `ERROR_compressionState_uninitialized` |
| 45 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound_internal(...)` | `ERROR_dstMaxSize_tooSmall` |
| 46 | `LZ4F_uncompressedUpdate` | `dstCapacity < srcSize` | `ERROR_dstMaxSize_tooSmall` |
| 47 | `LZ4F_flush` | buffered data exists and context stage is not `1` | `ERROR_compressionState_uninitialized` |
| 48 | `LZ4F_flush` | destination smaller than buffered data plus block overhead | `ERROR_dstMaxSize_tooSmall` |
| 49 | `LZ4F_compressEnd` | remaining destination after flush is `< 4` | `ERROR_dstMaxSize_tooSmall` |
| 50 | `LZ4F_compressEnd` | checksum enabled and remaining destination is `< 8` | `ERROR_dstMaxSize_tooSmall` |
| 51 | `LZ4F_compressEnd` | declared nonzero `contentSize != totalInSize` | `ERROR_frameSize_wrong` |
| 52 | `LZ4F_createDecompressionContext` | output context pointer is `NULL` | `ERROR_parameter_null` |
| 53 | `LZ4F_createDecompressionContext` | context allocation fails | `ERROR_allocation_failed` |
| 54 | frame header decoder | available header bytes `< 7` | `ERROR_frameHeader_incomplete` |
| 55 | frame header decoder | magic is neither LZ4 frame nor skippable-frame magic | `ERROR_frameType_unknown` |
| 56 | frame header decoder | FLG reserved bit 1 is set | `ERROR_reservedFlag_set` |
| 57 | frame header decoder | FLG version bits are not `01` | `ERROR_headerVersion_wrong` |
| 58 | frame header decoder | BD high reserved bit is set | `ERROR_reservedFlag_set` |
| 59 | frame header decoder | BD block-size ID is `< 4` | `ERROR_maxBlockSize_invalid` |
| 60 | frame header decoder | any BD low reserved bit is set | `ERROR_reservedFlag_set` |
| 61 | frame header decoder | header checksum byte differs from computed XXH32 byte | `ERROR_headerChecksum_invalid` |
| 62 | `LZ4F_headerSize` | `src == NULL` | `ERROR_srcPtr_wrong` |
| 63 | `LZ4F_headerSize` | `srcSize < 5` | `ERROR_frameHeader_incomplete` |
| 64 | `LZ4F_headerSize` | first four bytes are unknown magic | `ERROR_frameType_unknown` |
| 65 | `LZ4F_getFrameInfo` | decoding already passed the header stage | `ERROR_frameDecoding_alreadyStarted` |
| 66 | `LZ4F_getFrameInfo` | supplied bytes are fewer than computed header size | `ERROR_frameHeader_incomplete` |
| 67 | `LZ4F_decompress` | temporary compressed-input allocation fails | `ERROR_allocation_failed` |
| 68 | `LZ4F_decompress` | temporary output allocation fails | `ERROR_allocation_failed` |
| 69 | `LZ4F_decompress` | compressed block size exceeds frame maximum block size | `ERROR_maxBlockSize_invalid` |
| 70 | `LZ4F_decompress` | direct compressed block checksum mismatches | `ERROR_blockChecksum_invalid` |
| 71 | `LZ4F_decompress` | buffered compressed block checksum mismatches | `ERROR_blockChecksum_invalid` |
| 72 | `LZ4F_decompress` | LZ4 block decoder returns a negative result | `ERROR_decompressionFailed` |
| 73 | `LZ4F_decompress` | decoded bytes do not equal declared remaining content size | `ERROR_frameSize_wrong` |
| 74 | `LZ4F_decompress` | frame content checksum mismatches | `ERROR_contentChecksum_invalid` |
| 75 | `LZ4F_readOpen` | `fp == NULL` | `ERROR_parameter_null` |
| 76 | `LZ4F_readOpen` | output state pointer is `NULL` | `ERROR_parameter_null` |
| 77 | `LZ4F_readOpen` | state or source-buffer allocation fails | `ERROR_allocation_failed` |
| 78 | `LZ4F_readOpen` | initial header read is short | `ERROR_io_read` |
| 79 | `LZ4F_readOpen` | decoded block-size ID is outside `4..=7` | `ERROR_maxBlockSize_invalid` |
| 80 | `LZ4F_read` | state or destination buffer is `NULL` | `ERROR_parameter_null` |
| 81 | `LZ4F_read` | `fread()` reports an error | `ERROR_io_read` |
| 82 | `LZ4F_readClose` | state is `NULL` | `ERROR_parameter_null` |
| 83 | `LZ4F_writeOpen` | `fp == NULL` or output state pointer is `NULL` | `ERROR_parameter_null` |
| 84 | `LZ4F_writeOpen` | state or destination-buffer allocation fails | `ERROR_allocation_failed` |
| 85 | `LZ4F_writeOpen` | selected block-size ID is outside `0,4..=7` | `ERROR_maxBlockSize_invalid` |
| 86 | `LZ4F_writeOpen` | header `fwrite()` is short | `ERROR_io_write` |
| 87 | `LZ4F_write` | state or source buffer is `NULL` | `ERROR_parameter_null` |
| 88 | `LZ4F_write` | compressed-data `fwrite()` is short | `ERROR_io_write` |
| 89 | `LZ4F_writeClose` | state is `NULL` | `ERROR_parameter_null` |
| 90 | `LZ4_XXH32_update` / `LZ4_XXH64_update` | state pointer is `NULL` | `XXH_ERROR` (`1`) |
| 91 | XXH state allocators | allocation fails | `NULL` |

## Assertions and compile-time limits

The source contains 184 `assert(...)` tokens: 79 in `lz4.c`, 73 in
`lz4hc.c`, 28 in `lz4frame.c`, 2 in `lz4file.c`, and 2 in `xxhash.c`.
All five modules define `assert(condition)` as `((void)0)` unless
`LZ4_DEBUG>=1`. Neither CMake nor `build.rs` defines `LZ4_DEBUG`, so these
assertions have no runtime rejection result in the tested configuration.
Their externally reachable preconditions that also have production checks
are represented above. Internal-only assertions are invariants and cannot be
triggered through a defined public call without prior memory corruption.

Compile-time/range constants found mechanically in headers and sources:

| constant/enum | values |
|---|---|
| `LZ4_MAX_INPUT_SIZE` | `0x7E000000` |
| `LZ4_DISTANCE_MAX` | `65535` |
| `LZ4_ACCELERATION_DEFAULT..MAX` | `1..=65537` (outside values clamp) |
| `LZ4HC_CLEVEL_MIN/DEFAULT/OPT_MIN/MAX` | `2/9/10/12` (outside values normalize/clamp) |
| `LZ4F_blockSizeID_t` | `0,4,5,6,7`; all other integers reject |
| `LZ4F_blockMode_t` | `0,1` |
| checksum/mode flags | `0,1` |
| frame header sizes | minimum `7`, maximum `19`, `5` bytes to know length |

Completion:

- [x] Rows 1-91 are covered by grouped differential rejection tests.
- [x] Generic defined null, zero, oversized, one-past-range, and invalid-enum inputs pass.
