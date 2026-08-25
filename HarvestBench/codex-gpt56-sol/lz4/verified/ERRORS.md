# Error Surface

Derived from explicit checks in `c_src/src/*.c` under the CMake definitions
`LZ4_HEAPMODE=0`, `LZ4F_HEAPMODE=0`, and `XXH_NAMESPACE=LZ4_`. Frame errors
are the exact `size_t` encoding `(size_t)-(LZ4F_ERROR_*)`; core block errors
use their documented integer sentinel.

| # | function | trigger (exact invalid input/condition) | expected C result | |
|---:|----------|-----------------------------------------|-------------------|---|
| 1 | `LZ4_compressBound` | `inputSize < 0` | `0` | [x] |
| 2 | `LZ4_compressBound` | `inputSize > LZ4_MAX_INPUT_SIZE` (`0x7E000000`) | `0` | [x] |
| 3 | `LZ4_compress_default`, `LZ4_compress_fast`, `LZ4_compress_fast_extState`, aliases | `srcSize < 0` or `srcSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 4 | core compression entry points | `srcSize == 0` and limited `dstCapacity <= 0` | `0` | [x] |
| 5 | core compression entry points | compressed output cannot fit in `dstCapacity` | `0` | [x] |
| 6 | `LZ4_compress_destSize`, `LZ4_compress_destSize_extState` | `*srcSizePtr < 0`, `*srcSizePtr > LZ4_MAX_INPUT_SIZE`, or target cannot hold even an empty block | `0` | [x] |
| 7 | `LZ4_initStream` | `stateBuffer == NULL` | `NULL` | [x] |
| 8 | `LZ4_initStream` | `size < sizeof(LZ4_stream_t)` | `NULL` | [x] |
| 9 | `LZ4_initStream` | buffer fails `LZ4_stream_t_alignment()` | `NULL` when alignment checks are enabled; accepted in this default build where alignment is `1` | [x] |
| 10 | `LZ4_decoderRingBufferSize` | `maxBlockSize < 0` | `0` | [x] |
| 11 | `LZ4_decoderRingBufferSize` | `maxBlockSize > LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 12 | safe block decompression family | `src == NULL` | `-1` | [x] |
| 13 | safe block decompression family | output capacity is negative | `-1` | [x] |
| 14 | safe block decompression family | `compressedSize == 0` with nonzero output capacity | `-1` | [x] |
| 15 | safe block decompression family | literal length exceeds output capacity | exact negative decode position sentinel | [x] |
| 16 | safe block decompression family | malformed final literal/match placement | exact negative decode position sentinel | [x] |
| 17 | safe block decompression family | match offset exceeds available prefix plus dictionary | exact negative decode position sentinel | [x] |
| 18 | safe block decompression family | match length exceeds output capacity | exact negative decode position sentinel | [x] |
| 19 | `LZ4_initStreamHC` | `buffer == NULL` | `NULL` | [x] |
| 20 | `LZ4_initStreamHC` | `size < sizeof(LZ4_streamHC_t)` | `NULL` | [x] |
| 21 | `LZ4_initStreamHC`, `LZ4_compress_HC_extStateHC_fastReset` | buffer fails HC alignment requirement | `NULL` / `0`; accepted in this default build where alignment is `1` | [x] |
| 22 | HC compression and destination-size families | source size is negative or exceeds `LZ4_MAX_INPUT_SIZE` | `0` | [x] |
| 23 | HC compression and destination-size families | destination size is negative or cannot hold output | `0` | [x] |
| 24 | `LZ4_resetStreamStateHC` | state cannot be initialized | `1` | [x] |
| 25 | `LZ4F_getBlockSize` | enum value is neither `0` nor `4..=7` | `(size_t)-LZ4F_ERROR_maxBlockSize_invalid` | [x] |
| 26 | `LZ4F_compressFrame`, `LZ4F_compressFrame_usingCDict` | `dstCapacity < LZ4F_compressFrameBound(...)` | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 27 | `LZ4F_createCompressionContext` | output context pointer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 28 | `LZ4F_createCompressionContext`, advanced creators | allocator returns `NULL` | allocation error / `NULL` | [x] |
| 29 | `LZ4F_compressBegin*` | `dstCapacity < LZ4F_HEADER_SIZE_MAX` | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 30 | `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingDictOnce`, internal form | `dictSize > INT_MAX` with non-null dictionary | `(size_t)-LZ4F_ERROR_parameter_invalid` | [x] |
| 31 | `LZ4F_compressUpdate`, `LZ4F_uncompressedUpdate` | context stage is not initialized by `LZ4F_compressBegin*` | `(size_t)-LZ4F_ERROR_compressionState_uninitialized` | [x] |
| 32 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound_internal(...)` | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 33 | `LZ4F_uncompressedUpdate` | destination cannot hold the uncompressed source | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 34 | `LZ4F_flush` | buffered data exists but context stage is not active | `(size_t)-LZ4F_ERROR_compressionState_uninitialized` | [x] |
| 35 | `LZ4F_flush` | destination is smaller than buffered data plus block/checksum headers | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 36 | `LZ4F_compressEnd` | fewer than 4 bytes remain after flush | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 37 | `LZ4F_compressEnd` | content checksum enabled and fewer than 8 bytes remain after flush | `(size_t)-LZ4F_ERROR_dstMaxSize_tooSmall` | [x] |
| 38 | `LZ4F_compressEnd` | declared nonzero content size differs from total input | `(size_t)-LZ4F_ERROR_frameSize_wrong` | [x] |
| 39 | `LZ4F_createDecompressionContext` | output context pointer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 40 | `LZ4F_createDecompressionContext`, advanced creator | allocator returns `NULL` | allocation error / `NULL` | [x] |
| 41 | `LZ4F_headerSize` | `src == NULL` | `(size_t)-LZ4F_ERROR_srcPtr_wrong` | [x] |
| 42 | `LZ4F_headerSize` | `srcSize < LZ4F_MIN_SIZE_TO_KNOW_HEADER_LENGTH` (`5`) | `(size_t)-LZ4F_ERROR_frameHeader_incomplete` | [x] |
| 43 | `LZ4F_headerSize`, `LZ4F_getFrameInfo`, `LZ4F_decompress*` | first four bytes are neither frame magic nor skippable-frame magic | `(size_t)-LZ4F_ERROR_frameType_unknown` | [x] |
| 44 | frame header decoder | frame header has FLG reserved bit set | `(size_t)-LZ4F_ERROR_reservedFlag_set` | [x] |
| 45 | frame header decoder | FLG version bits are not `01` | `(size_t)-LZ4F_ERROR_headerVersion_wrong` | [x] |
| 46 | frame header decoder | BD bit 7 or low four reserved bits are nonzero | `(size_t)-LZ4F_ERROR_reservedFlag_set` | [x] |
| 47 | frame header decoder | BD block size id is below `4` | `(size_t)-LZ4F_ERROR_maxBlockSize_invalid` | [x] |
| 48 | frame header decoder | computed header checksum differs from stored checksum | `(size_t)-LZ4F_ERROR_headerChecksum_invalid` | [x] |
| 49 | `LZ4F_getFrameInfo` | called while a partial frame header is already buffered | `(size_t)-LZ4F_ERROR_frameDecoding_alreadyStarted` and `*srcSizePtr = 0` | [x] |
| 50 | `LZ4F_getFrameInfo` | supplied bytes are fewer than the header length encoded by FLG | `(size_t)-LZ4F_ERROR_frameHeader_incomplete` and `*srcSizePtr = 0` | [x] |
| 51 | `LZ4F_decompress*` | block header size exceeds frame's selected maximum block size | `(size_t)-LZ4F_ERROR_maxBlockSize_invalid` | [x] |
| 52 | `LZ4F_decompress*` | stored block checksum differs from computed checksum | `(size_t)-LZ4F_ERROR_blockChecksum_invalid` | [x] |
| 53 | `LZ4F_decompress*` | compressed block is malformed and block decoder returns negative | `(size_t)-LZ4F_ERROR_decompressionFailed` | [x] |
| 54 | `LZ4F_decompress*` | decoded bytes differ from declared content size at end mark | `(size_t)-LZ4F_ERROR_frameSize_wrong` | [x] |
| 55 | `LZ4F_decompress*` | stored content checksum differs from computed checksum | `(size_t)-LZ4F_ERROR_contentChecksum_invalid` | [x] |
| 56 | `LZ4_XXH32_update`, `LZ4_XXH64_update` | `input == NULL` (including zero length; `XXH_ACCEPT_NULL_INPUT_POINTER` is unset) | `XXH_ERROR` (`1`) | [x] |
| 57 | `LZ4F_readOpen` | `fp == NULL` or output state pointer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 58 | `LZ4F_readOpen` | initial `fread` returns fewer than `LZ4F_HEADER_SIZE_MAX` (`19`) bytes | `(size_t)-LZ4F_ERROR_io_read` | [x] |
| 59 | `LZ4F_read` | state or destination buffer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 60 | `LZ4F_readClose` | state is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 61 | `LZ4F_writeOpen` | `fp == NULL` or output state pointer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 62 | `LZ4F_writeOpen` | preferences contain block size id outside `0,4,5,6,7` | `(size_t)-LZ4F_ERROR_maxBlockSize_invalid` | [x] |
| 63 | `LZ4F_writeOpen`, `LZ4F_write`, `LZ4F_writeClose` | `fwrite` writes fewer bytes than requested | `(size_t)-LZ4F_ERROR_io_write` | [x] |
| 64 | `LZ4F_write` | state or source buffer is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |
| 65 | `LZ4F_writeClose` | state is `NULL` | `(size_t)-LZ4F_ERROR_parameter_null` | [x] |

## Assert Accounting

`rg -n 'assert\s*\(' c_src/src` finds assertions in all five source files.
For this default configuration, `lz4.c` and `lz4frame.c` define `assert` as a
no-op because `LZ4_DEBUG` is unset; `lz4hc.c` imports those common definitions.
The two `lz4file.c` assertions require non-null addresses in private cleanup
helpers and all callers satisfy them. The two `xxhash.c` `assert(0)` statements
follow exhaustive `len & 15` and `len & 31` switches and are unreachable for
all inputs. Consequently none is an additional public rejection result beyond
the rows above.
