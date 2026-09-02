# ERRORS.md — Error-surface table

Derived mechanically from the C source by `/tmp/gen_errors.py`, which walks
every `.c` file under `c_src/src/`, tracks the enclosing function, and emits one
row for every `RETURN_ERROR_IF(...)`, `RETURN_ERROR(...)`, `return ERROR(...)`,
`= ERROR(...)`, `BOUNDCHECK(...)`, `CHECK_F(...)`, `return NULL;` and
`return -1;` site.

Totals: **1292** rejection sites — **564** in the core library
(`common`/`compress`/`decompress`/`dictBuilder`/`deprecated`) and **728** in the
legacy decoders (`legacy/`, compiled in because `ZSTD_LEGACY_SUPPORT=5`).

Error codes actually used, by frequency (from the same mechanical extract):
`corruption_detected` 357, `srcSize_wrong` 193, `GENERIC` 121,
`dstSize_tooSmall` 117, `dictionary_corrupted` 70, `memory_allocation` 55,
`tableLog_tooLarge` 45, `stage_wrong` 18, `parameter_unsupported` 17,
`externalSequences_invalid` 16, `prefix_unknown` 14,
`frameParameter_unsupported` 12, `parameter_outOfBound` 11,
`maxSymbolValue_tooLarge` 11, `maxSymbolValue_tooSmall` 9, `workSpace_tooSmall` 5,
`init_missing` 5, `stabilityCondition_notRespected` 4, `dictionary_wrong` 4,
`checksum_wrong` 4, `sequenceProducer_failed` 2, `dstBuffer_null` 2,
`dictionaryCreation_failed` 2, `noForwardProgress_inputEmpty` 1,
`noForwardProgress_destFull` 1, `literals_headerWrong` 1,
`frameParameter_windowTooLarge` 1, `dstBuffer_wrong` 1,
`cannotProduce_uncompressedBlock` 1.

## How the rows are covered

Every row is covered by a differential test that constructs the trigger, calls
BOTH `.so`s, and asserts the **same error code** via
`ZSTD_getErrorCode`/`FSE_isError`/`HUF_isError`/`ZDICT_isError` — not merely
"both failed". The `test` column names the covering test.

Rows fall into two coverage kinds:

- **direct** — the test hands the exact invalid argument to the exact public
  entry point (rows 1–99 below, plus every `parameter_outOfBound` /
  `parameter_unsupported` / `stage_wrong` / `dstSize_tooSmall` /
  `srcSize_wrong` / `memory_allocation` / `workSpace_tooSmall` /
  `dictionary_*` / `stabilityCondition_notRespected` site).
- **constructive-corruption** — for the deep bitstream/entropy checks
  (`corruption_detected`, `tableLog_tooLarge`, `maxSymbolValue_too*`,
  `checksum_wrong`, `literals_headerWrong`, `dictionary_corrupted` inside
  entropy loading, and every legacy-decoder check) the test constructs the
  invalid condition by systematically mutating valid frames/headers/tables:
  `c4_corruption.rs` performs an **exhaustive single-byte sweep** (every byte
  offset × every one of the 8 bit flips, plus 0x00/0xFF stamping) over frames
  produced in all of the `CONFIGS.md` configurations, plus every truncation
  length, plus random-garbage buffers, and asserts C and Rust return the
  identical error code (or the identical success) for *each* mutation. That
  drives these sites with the exact conditions they test, and — critically —
  asserts equality on the whole reachable error lattice rather than one point
  in it.

### Part 1 — API-reachable rejections (one row per distinct trigger)

| # | function | trigger (the exact invalid input/condition) | expected C result | test |
|---|----------|----------------------------------------------|-------------------|------|
| 1 | `ZSTD_cParam_getBounds` | `param` is not a recognised `ZSTD_cParameter` (e.g. 0, 99, 108, 203, 403, 501, 1018, −1, `INT_MIN`, `INT_MAX`) | `bounds.error = ZSTD_error_parameter_unsupported`, `lowerBound == upperBound == 0` | `c1_params.rs` |
| 2 | `ZSTD_dParam_getBounds` | `param` is not a recognised `ZSTD_dParameter` (0, 99, 101, 999, 1006, −1, `INT_MIN`, `INT_MAX`) | `bounds.error = ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 3 | `ZSTD_CCtx_setParameter` | `param` is an unrecognised enum value | `ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 4 | `ZSTD_CCtx_setParameter` | `value` one step below `lowerBound` for a *bounded-checked* param (`windowLog`, `hashLog`, `chainLog`, `searchLog`, `minMatch`, `targetLength`, `strategy`, `ldm*`, `targetCBlockSize`, `maxBlockSize`, `blockSplitterLevel`, `format`, `forceAttachDict`, `literalCompressionMode`, `stableIn/OutBuffer`, `blockDelimiters`, `splitAfterSequences`, `useRowMatchFinder`, `prefetchCDictTables`, `repcodeResolution`) | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 5 | `ZSTD_CCtx_setParameter` | `value` one step above `upperBound` for the same set | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 6 | `ZSTD_CCtx_setParameter` | `value` out of range for a *clamping* param (`compressionLevel`, `contentSizeFlag`, `checksumFlag`, `dictIDFlag`, `nbWorkers`, `jobSize`, `overlapLog`, `rsyncable`, `forceMaxWindow`, `srcSizeHint`, `enableDedicatedDictSearch`, `validateSequences`, `deterministicRefPrefix`, `enableSeqProducerFallback`, `enableLongDistanceMatching`) | success (value silently clamped) — asserted identical via read-back with `ZSTD_CCtx_getParameter` | `c1_params.rs` |
| 7 | `ZSTD_CCtx_setParameter` | called after compression started (stage != `ZSTDcs_init`) for a non-updatable param | `ZSTD_error_stage_wrong` | `c1_params.rs` |
| 8 | `ZSTD_CCtx_getParameter` | unrecognised `param` | `ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 9 | `ZSTD_CCtxParams_setParameter` / `_getParameter` | unrecognised `param` / out-of-range value | same as rows 3–5, `parameter_unsupported` / `parameter_outOfBound` | `c1_params.rs` |
| 10 | `ZSTD_CCtxParams_init_advanced` | `params.cParams` fail `ZSTD_checkCParams` | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 11 | `ZSTD_DCtx_setParameter` | unrecognised `param` | `ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 12 | `ZSTD_DCtx_setParameter` | `value` one step outside the bounds of `windowLogMax`, `format`, `stableOutBuffer`, `forceIgnoreChecksum`, `refMultipleDDicts`, `disableHuffmanAssembly`, `maxBlockSize` | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 13 | `ZSTD_DCtx_setParameter` | called while a frame is being decoded (stage != `ZSTDds_init`) | `ZSTD_error_stage_wrong` | `c1_params.rs` |
| 14 | `ZSTD_DCtx_getParameter` | unrecognised `param` | `ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 15 | `ZSTD_checkCParams` | `windowLog`/`chainLog`/`hashLog`/`searchLog`/`minMatch`/`targetLength`/`strategy` outside its `[MIN,MAX]` | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 16 | `ZSTD_CCtx_setCParams` / `ZSTD_CCtx_setParams` | `cParams` fail `ZSTD_checkCParams` | `ZSTD_error_parameter_outOfBound` | `c1_params.rs` |
| 17 | `ZSTD_CCtx_reset` | `reset` is not one of 1/2/3 (e.g. 0, 4, −1, `INT_MAX`) | `ZSTD_error_parameter_unsupported` (reset_parameters while not init → `stage_wrong`) | `c1_params.rs` |
| 18 | `ZSTD_DCtx_reset` | `reset` is not one of 1/2/3 | `ZSTD_error_parameter_unsupported` | `c1_params.rs` |
| 19 | `ZSTD_CCtx_reset(reset_parameters)` | called mid-frame | `ZSTD_error_stage_wrong` | `c1_params.rs` |
| 20 | `ZSTD_compress` / `ZSTD_compressCCtx` / `ZSTD_compress2` | `dstCapacity` smaller than the produced frame | `ZSTD_error_dstSize_tooSmall` | `b1_simple_api.rs`, `c3_decompress.rs` |
| 21 | `ZSTD_compress*` / `ZSTD_decompress*` | `dst == NULL` with `dstCapacity == 0`, and a valid `dst` with `dstCapacity == 0` | whatever the C returns (`dstSize_tooSmall` / `dstBuffer_null`), asserted equal on both entry points | `c3_decompress.rs` |
| 22 | `ZSTD_compress*` / `ZSTD_decompress*` | `src == NULL` with `srcSize == 0` (the well-defined case) | asserted identical to C on every one-shot entry point. **`src == NULL` with `srcSize > 0` is NOT compared**: the C dereferences it without a guard and segfaults, so there is no defined C result to compare against. | `c3_decompress.rs` |
| 23 | `ZSTD_compress2` | `pledgedSrcSize` set via `ZSTD_CCtx_setPledgedSrcSize` ≠ actual `srcSize` | `ZSTD_error_srcSize_wrong` | `c1_params.rs`, `c6_stream.rs` |
| 24 | `ZSTD_CCtx_setPledgedSrcSize` | called mid-frame | `ZSTD_error_stage_wrong` | `c1_params.rs` |
| 25 | `ZSTD_decompress` / `ZSTD_decompressDCtx` | `srcSize == 0` | `ZSTD_error_srcSize_wrong` | `c3_decompress.rs` |
| 26 | `ZSTD_decompress*` | `src` does not start with a valid magic (and is not a legacy magic) | `ZSTD_error_prefix_unknown` | `c3_decompress.rs` |
| 27 | `ZSTD_decompress*` | frame header truncated (1 … `headerSize−1` bytes) | `ZSTD_error_srcSize_wrong` | `c3_decompress.rs` |
| 28 | `ZSTD_decompress*` | frame body truncated after a valid header | `ZSTD_error_srcSize_wrong` | `c3_decompress.rs` |
| 29 | `ZSTD_decompress*` | `dstCapacity` < decompressed size | `ZSTD_error_dstSize_tooSmall` | `c3_decompress.rs` |
| 30 | `ZSTD_decompress*` | `dst == NULL`, `dstCapacity == 0`, frame content non-empty | `ZSTD_error_dstSize_tooSmall` | `c3_decompress.rs` |
| 31 | `ZSTD_decompress*` | trailing garbage after a complete frame | `ZSTD_error_prefix_unknown` | `c3_decompress.rs` |
| 32 | `ZSTD_decompress*` | frame's XXH64 checksum trailer altered | `ZSTD_error_checksum_wrong` | `c4_corruption.rs` |
| 33 | `ZSTD_decompress*` | frame `windowLog` exceeds `ZSTD_d_windowLogMax` | `ZSTD_error_frameParameter_windowTooLarge` | `c3_decompress.rs` |
| 34 | `ZSTD_decompress*` | frame header reserved bit set / unsupported frame parameter | `ZSTD_error_frameParameter_unsupported` | `c4_corruption.rs` |
| 35 | `ZSTD_decompress*` | frame declares a dictID but no dictionary is loaded | `ZSTD_error_dictionary_wrong` | `c5_dict.rs` |
| 36 | `ZSTD_decompress*` | a *different* dictionary is loaded than the one used to compress | `ZSTD_error_dictionary_wrong` | `c5_dict.rs` |
| 37 | `ZSTD_getFrameContentSize` | `srcSize` too small to read the header, or bad magic | `ZSTD_CONTENTSIZE_ERROR` (`(unsigned long long)-2`) or `ZSTD_CONTENTSIZE_UNKNOWN` (`-1`) — asserted bit-identical | `b1_simple_api.rs` |
| 38 | `ZSTD_frameHeaderSize` | `srcSize < ZSTD_FRAMEHEADERSIZE_PREFIX(format)` | `ZSTD_error_srcSize_wrong` | `b1_simple_api.rs` |
| 39 | `ZSTD_getFrameHeader(_advanced)` | `srcSize` < the needed header size | returns the number of bytes still needed (> 0), *not* an error | `b1_simple_api.rs` |
| 40 | `ZSTD_getFrameHeader_advanced` | `format` is an out-of-range enum value (2, −1, 99) | asserted identical to C | `b1_simple_api.rs` |
| 41 | `ZSTD_findFrameCompressedSize` | truncated / invalid frame | `ZSTD_error_srcSize_wrong` or `prefix_unknown` | `b1_simple_api.rs` |
| 42 | `ZSTD_findDecompressedSize` | one frame in the chain is invalid | `ZSTD_CONTENTSIZE_ERROR` | `b1_simple_api.rs`, `c3_decompress.rs` |
| 43 | `ZSTD_decompressBound` | invalid frame in the chain | `ZSTD_CONTENTSIZE_ERROR` | `b1_simple_api.rs` |
| 44 | `ZSTD_readSkippableFrame` | `src` is not a skippable frame | `ZSTD_error_frameParameter_unsupported` | `b1_simple_api.rs` |
| 45 | `ZSTD_readSkippableFrame` | `dstCapacity` < skippable payload size | `ZSTD_error_dstSize_tooSmall` | `b1_simple_api.rs` |
| 46 | `ZSTD_writeSkippableFrame` | `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE` | `ZSTD_error_dstSize_tooSmall` | `b1_simple_api.rs` |
| 47 | `ZSTD_writeSkippableFrame` | `magicVariant > 15` | `ZSTD_error_parameter_outOfBound` | `b1_simple_api.rs` |
| 48 | `ZSTD_compressStream2` | `input.pos > input.size` or `output.pos > output.size` | `ZSTD_error_srcSize_wrong` / `dstSize_tooSmall` (asserted equal) | `c6_stream.rs` |
| 49 | `ZSTD_compressStream2` | `endOp` is not 0/1/2 (e.g. 3, −1, `INT_MAX`) | `ZSTD_error_parameter_outOfBound` | `c6_stream.rs` |
| 50 | `ZSTD_compressStream2` | `ZSTD_c_stableInBuffer` set and the input buffer moved/changed between calls | `ZSTD_error_stabilityCondition_notRespected` / `srcBuffer_wrong` | `c6_stream.rs` |
| 51 | `ZSTD_compressStream2` | `ZSTD_c_stableOutBuffer` set and the output buffer moved/shrank between calls | `ZSTD_error_stabilityCondition_notRespected` / `dstBuffer_wrong` | `c6_stream.rs` |
| 52 | `ZSTD_compressStream2` | more input supplied than the pledged src size | `ZSTD_error_srcSize_wrong` | `c6_stream.rs` |
| 53 | `ZSTD_compressStream2` | `endOp == ZSTD_e_end` with fewer bytes than pledged | `ZSTD_error_srcSize_wrong` | `c6_stream.rs` |
| 54 | `ZSTD_compressStream2` | zero-size output buffer and zero-size input repeatedly | `ZSTD_error_noForwardProgress_destFull` / `noForwardProgress_inputEmpty` | `c6_stream.rs` |
| 55 | `ZSTD_decompressStream` | zero-size output buffer, non-empty input, called repeatedly | `ZSTD_error_noForwardProgress_destFull` | `c6_stream.rs` |
| 56 | `ZSTD_decompressStream` | `input.pos > input.size` / `output.pos > output.size` | `ZSTD_error_srcSize_wrong` / `dstSize_tooSmall` | `c6_stream.rs` |
| 57 | `ZSTD_decompressStream` | `ZSTD_d_stableOutBuffer` set and the output buffer changed | `ZSTD_error_dstBuffer_wrong` | `c6_stream.rs` |
| 58 | `ZSTD_flushStream` / `ZSTD_endStream` | called before `ZSTD_initCStream` on a fresh CCtx | `ZSTD_error_init_missing` / `stage_wrong` | `c6_stream.rs` |
| 59 | `ZSTD_initCStream_advanced` | `params.cParams` fail `ZSTD_checkCParams` | `ZSTD_error_parameter_outOfBound` | `c6_stream.rs` |
| 60 | `ZSTD_compressBegin*` | invalid level / cParams | `ZSTD_error_parameter_outOfBound` | `c7_blocklevel.rs` |
| 61 | `ZSTD_compressContinue` | called without a preceding `ZSTD_compressBegin*` | `ZSTD_error_stage_wrong` | `c7_blocklevel.rs` |
| 62 | `ZSTD_compressContinue` | `srcSize > ZSTD_BLOCKSIZE_MAX` when it must fit in one block | `ZSTD_error_srcSize_wrong` | `c7_blocklevel.rs` |
| 63 | `ZSTD_compressContinue` / `ZSTD_compressEnd` | `dstCapacity` too small | `ZSTD_error_dstSize_tooSmall` | `c7_blocklevel.rs` |
| 64 | `ZSTD_compressBlock` | `srcSize > ZSTD_BLOCKSIZE_MAX` (131073) | `ZSTD_error_srcSize_wrong` | `c7_blocklevel.rs` |
| 65 | `ZSTD_compressBlock` | `dstCapacity` too small | `ZSTD_error_dstSize_tooSmall` | `c7_blocklevel.rs` |
| 66 | `ZSTD_decompressBlock` | `srcSize > ZSTD_BLOCKSIZE_MAX` | `ZSTD_error_srcSize_wrong` | `c7_blocklevel.rs` |
| 67 | `ZSTD_decompressBlock` | corrupted block body | `ZSTD_error_corruption_detected` | `c4_corruption.rs` |
| 68 | `ZSTD_decompressContinue` | `srcSize != ZSTD_nextSrcSizeToDecompress(dctx)` | `ZSTD_error_srcSize_wrong` | `c7_blocklevel.rs` |
| 69 | `ZSTD_decompressContinue` | called before `ZSTD_decompressBegin*` | `ZSTD_error_stage_wrong` / `srcSize_wrong` | `c7_blocklevel.rs` |
| 70 | `ZSTD_insertBlock` | `blockSize > ZSTD_BLOCKSIZE_MAX` | asserted identical to C | `c7_blocklevel.rs` |
| 71 | `ZSTD_createCCtx_advanced` etc. | `customMem` allocator returns `NULL` (failing allocator injected) | `NULL` return / `ZSTD_error_memory_allocation` | `c8_alloc.rs` |
| 72 | `ZSTD_initStaticCCtx` / `initStaticCStream` / `initStaticDCtx` / `initStaticDStream` / `initStaticCDict` / `initStaticDDict` | `workspaceSize` < `ZSTD_estimate*Size(...)`, or `workspace` misaligned, or `workspace == NULL` | `NULL` | `c8_alloc.rs` |
| 73 | `ZSTD_CCtx_loadDictionary(_advanced)` | `dictSize` non-zero but `dict == NULL` | `ZSTD_error_dictionary_wrong` / `GENERIC` | `c5_dict.rs` |
| 74 | `ZSTD_CCtx_loadDictionary_advanced` | `dictContentType == ZSTD_dct_fullDict` but the buffer lacks `ZSTD_MAGIC_DICTIONARY` | `ZSTD_error_dictionary_wrong` | `c5_dict.rs` |
| 75 | `ZSTD_CCtx_loadDictionary_advanced` | `dictContentType` / `dictLoadMethod` out-of-range enum (3, −1, 99) | asserted identical to C | `c5_dict.rs` |
| 76 | `ZSTD_CCtx_loadDictionary` | trained dict whose entropy tables are corrupted | `ZSTD_error_dictionary_corrupted` | `c5_dict.rs` |
| 77 | `ZSTD_createCDict*` / `ZSTD_createDDict*` | `dictSize > 0` with `dict == NULL`, or a corrupted trained dict, or an invalid level | `NULL` | `c5_dict.rs` |
| 78 | `ZSTD_DCtx_loadDictionary(_advanced)` | same triggers as rows 73–76 | `ZSTD_error_dictionary_corrupted` / `dictionary_wrong` | `c5_dict.rs` |
| 79 | `ZSTD_CCtx_refPrefix_advanced` / `ZSTD_DCtx_refPrefix_advanced` | out-of-range `dictContentType`; `prefixSize > 0` with `prefix == NULL` | asserted identical to C | `c5_dict.rs` |
| 80 | `ZSTD_getDictID_fromDict` / `_fromFrame` | buffer too small / not a dict / not a frame | `0` | `c5_dict.rs` |
| 81 | `ZSTD_compressSequences` | a sequence has `offset == 0`, or `offset` > the window / available history | `ZSTD_error_externalSequences_invalid` (with `validateSequences=1`) / `corruption_detected` | `c9_sequences.rs` |
| 82 | `ZSTD_compressSequences` | `litLength + matchLength` sums do not cover `srcSize` | `ZSTD_error_externalSequences_invalid` / `srcSize_wrong` | `c9_sequences.rs` |
| 83 | `ZSTD_compressSequences` | `matchLength < MINMATCH` | `ZSTD_error_externalSequences_invalid` | `c9_sequences.rs` |
| 84 | `ZSTD_compressSequences` | `blockDelimiters == explicitBlockDelimiters` but the array has no delimiter / a malformed one | `ZSTD_error_externalSequences_invalid` / `corruption_detected` | `c9_sequences.rs` |
| 85 | `ZSTD_compressSequences` | `dstCapacity` too small | `ZSTD_error_dstSize_tooSmall` | `c9_sequences.rs` |
| 86 | `ZSTD_compressSequencesAndLiterals` | `srcSize` implied by the sequences ≠ `decompressedSize`; literals buffer too small | `ZSTD_error_externalSequences_invalid` / `srcSize_wrong` | `c9_sequences.rs` |
| 87 | `ZSTD_generateSequences` | `outSeqsCapacity < ZSTD_sequenceBound(srcSize)` | `ZSTD_error_dstSize_tooSmall` | `c9_sequences.rs` |
| 88 | `ZSTD_mergeBlockDelimiters` | (no error path; identity on a delimiter-free array) | asserted byte-identical output | `b9_sequences.rs` |
| 89 | `FSE_readNCount(_bmi2)` | `hbSize` shorter than the encoded table; `nbBits > FSE_TABLELOG_ABSOLUTE_MAX`; `remaining != 1`; `charnum > maxSV1`; `bitCount > 32` | `corruption_detected` / `tableLog_tooLarge` / `maxSymbolValue_tooSmall` | `c10_entropy.rs` |
| 90 | `FSE_normalizeCount` | `tableLog < FSE_MIN_TABLELOG`; `tableLog > FSE_MAX_TABLELOG`; `tableLog < highbit(maxSymbolValue)+2`; `total` smaller than the symbol count | `GENERIC` / `tableLog_tooLarge` / `maxSymbolValue_tooLarge` | `c10_entropy.rs` |
| 91 | `FSE_writeNCount(_wksp)` | `tableLog > FSE_MAX_TABLELOG`; `maxSymbolValue > FSE_MAX_SYMBOL_VALUE`; `bufferSize < FSE_NCountWriteBound(...)` | `GENERIC` / `tableLog_tooLarge` / `dstSize_tooSmall` | `c10_entropy.rs` |
| 92 | `FSE_buildCTable(_wksp)` / `FSE_buildDTable(_wksp)` | `tableLog > FSE_MAX_TABLELOG`; `maxSymbolValue > FSE_MAX_SYMBOL_VALUE`; `wkspSize` too small; a normalized count that does not sum to `1<<tableLog` | `tableLog_tooLarge` / `maxSymbolValue_tooLarge` / `GENERIC` | `c10_entropy.rs` |
| 93 | `FSE_compress` / `FSE_compress2` / `FSE_compress_usingCTable(_bmi2)` | `srcSize <= 1`; `dstCapacity` too small; `maxSymbolValue > FSE_MAX_SYMBOL_VALUE`; `tableLog > FSE_MAX_TABLELOG` | `0` (incompressible), `dstSize_tooSmall`, `maxSymbolValue_tooLarge`, `tableLog_tooLarge` | `c10_entropy.rs` |
| 94 | `FSE_decompress(_wksp/_wksp_bmi2)` / `FSE_decompress_usingDTable` | `cSrcSize == 0`; truncated/corrupted stream; `wkspSize` too small; `maxLog` too small for the encoded table | `srcSize_wrong` / `corruption_detected` / `tableLog_tooLarge` / `maxSymbolValue_tooLarge` | `c10_entropy.rs` |
| 95 | `HUF_readStats(_wksp/_body/_bmi2)` | `srcSize == 0`; `iSize+1 > srcSize`; `oSize >= hwSize`; `huffWeight[n] > HUF_TABLELOG_MAX`; `weightTotal == 0`; `tableLog > HUF_TABLELOG_MAX`; `verif != rest`; `rankStats[1] < 2 || odd` | `srcSize_wrong` / `corruption_detected` | `c10_entropy.rs` |
| 96 | `HUF_compress1X*` / `HUF_compress4X*` | `srcSize == 0` / `> HUF_BLOCKSIZE_MAX`; `dstSize == 0`; `maxSymbolValue > HUF_SYMBOLVALUE_MAX`; `huffLog > HUF_TABLELOG_MAX`; `wkspSize` too small | `srcSize_wrong` / `dstSize_tooSmall` / `maxSymbolValue_tooLarge` / `tableLog_tooLarge` / `workSpace_tooSmall` | `c10_entropy.rs` |
| 97 | `HUF_decompress*` / `HUF_readDTableX1(_wksp)` / `X2(_wksp)` | `cSrcSize == 0`; `dstSize == 0`; corrupted header; `tableLog > maxTableLog`; `wkspSize` too small | `srcSize_wrong` / `dstSize_tooSmall` / `corruption_detected` / `tableLog_tooLarge` / `maxSymbolValue_tooLarge` | `c10_entropy.rs` |
| 98 | `HUF_buildCTable(_wksp)` / `HUF_writeCTable(_wksp)` / `HUF_estimateCompressedSize` / `HUF_validateCTable` | `maxSymbolValue > HUF_SYMBOLVALUE_MAX`; `maxNbBits > HUF_TABLELOG_MAX`; `wkspSize`/`dstCapacity` too small | `maxSymbolValue_tooLarge` / `tableLog_tooLarge` / `dstSize_tooSmall` / `GENERIC` | `c10_entropy.rs` |
| 99 | `HIST_count(_wksp)` / `HIST_countFast(_wksp)` | `maxSymbolValue > 255`; `workSpaceSize < HIST_WKSP_SIZE`; a symbol in `src` exceeds `maxSymbolValue` (`HIST_count` only) | `maxSymbolValue_tooLarge` / `GENERIC`; `HIST_countFast` reads out of range instead (replicated) | `c10_entropy.rs` |
| 100 | `ZDICT_trainFromBuffer*` / `finalizeDictionary` / `addEntropyTablesFromBuffer_advanced` | `dictBufferCapacity < ZDICT_DICTSIZE_MIN`; `nbSamples == 0`; total sample size below the minimum; `k`/`d`/`f`/`accel`/`steps` out of range; `splitPoint` outside `(0,1]` | `ZDICT_error_dstSize_tooSmall` / `srcSize_wrong` / `GENERIC` / `parameter_outOfBound` (all via `ZDICT_isError`) | `c11_dictbuilder.rs` |
| 101 | `ZDICT_getDictHeaderSize` / `ZDICT_getDictID` | buffer too small / wrong magic / corrupted entropy tables | `ZDICT_error_dictionary_corrupted` / `0` | `c11_dictbuilder.rs` |
| 102 | `ZBUFF_compressContinue` / `_flush` / `_end` / `ZBUFF_decompressContinue` (deprecated API) | called without init; `dst` too small; corrupted input | error via `ZBUFF_isError`, code asserted equal | `c12_deprecated.rs` |
| 103 | `ZSTDv01_decompress` … `ZSTDv07_decompress`, `ZSTDv0x_findFrameSizeInfoLegacy`, `ZBUFFv0x_decompressContinue` | wrong magic; truncated frame; `dst` too small; corrupted body | version-specific error codes, asserted equal via the matching `ZSTDv0x_isError` | `c13_legacy.rs` |
| 104 | `POOL_create(_advanced)` / `POOL_resize` / `POOL_add` / `POOL_tryAdd` / `POOL_joinJobs` | `numThreads == 0`; `NULL` ctx; allocation failure | `NULL` / `0` / no-op — asserted identical | `c14_pool.rs` |
| 105 | `ZSTD_isError` / `ZSTD_getErrorCode` / `ZSTD_getErrorName` / `ZSTD_getErrorString` | every `size_t` value `(size_t)-1 .. (size_t)-160`, `0`, `1`, small positives, and huge non-error values; every `ZSTD_ErrorCode` `0..160` including values with no valid variant | identical `unsigned`/`ZSTD_ErrorCode`/string for every input | `c15_errorapi.rs` |
| 106 | `ZSTD_customMalloc` / `_customCalloc` / `_customFree` | size 0; `NULL` pointer free; custom allocator returning `NULL` | identical behaviour (asserted via a counting allocator) | `c8_alloc.rs` |
| 107 | `ZSTD_decompressionMargin` | invalid / truncated frame | `ZSTD_error_*` (asserted equal) | `b8_estimates.rs` |
| 108 | `ZSTD_estimateDStreamSize_fromFrame` | `srcSize` too small / invalid frame | `ZSTD_error_srcSize_wrong` / `prefix_unknown` | `b8_estimates.rs` |
| 109 | `ZSTD_copyCCtx` | source CCtx is not in `ZSTDcs_init` stage | `ZSTD_error_stage_wrong` | `c7_blocklevel.rs` |
| 110 | out-of-range enum across the FFI boundary (C enums accept any `int`) | `ZSTD_strategy`, `ZSTD_cParameter`, `ZSTD_dParameter`, `ZSTD_EndDirective`, `ZSTD_ResetDirective`, `ZSTD_dictContentType_e`, `ZSTD_dictLoadMethod_e`, `ZSTD_format_e`, `ZSTD_ParamSwitch_e`, `ZSTD_SequenceFormat_e`, `ZSTD_dictAttachPref_e`, `ZSTD_bufferMode_e`, `ZSTD_ErrorCode` — each passed the values `−1`, `INT_MIN`, `INT_MAX`, `upper+1`, `lower−1`, and a random `i32` | whatever the C returns, asserted identical for every value | `c16_enums.rs` |

### Part 2 — full mechanical enumeration (core library)

564 rejection sites in `src/common`, `src/compress`, `src/decompress`, `src/dictBuilder`, `src/deprecated`.

| # | function / site | trigger (exact condition the C tests) | expected C result |
|---|-----------------|----------------------------------------|-------------------|
| 101 | `FSE_readNCount_body` <br/>`src/common/entropy_common.c:64` | `if (countSize > hbSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 102 | `FSE_readNCount_body` <br/>`src/common/entropy_common.c:73` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 103 | `FSE_readNCount_body` <br/>`src/common/entropy_common.c:179` | `if (remaining != 1) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 104 | `FSE_readNCount_body` <br/>`src/common/entropy_common.c:181` | `if (charnum > maxSV1) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 105 | `FSE_readNCount_body` <br/>`src/common/entropy_common.c:182` | `if (bitCount > 32) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 106 | `HUF_readStats` <br/>`src/common/entropy_common.c:254` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 107 | `HUF_readStats` <br/>`src/common/entropy_common.c:261` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 108 | `HUF_readStats` <br/>`src/common/entropy_common.c:262` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 109 | `HUF_readStats` <br/>`src/common/entropy_common.c:270` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 110 | `HUF_readStats` <br/>`src/common/entropy_common.c:280` | `if (huffWeight[n] > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 111 | `HUF_readStats` <br/>`src/common/entropy_common.c:284` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 112 | `HUF_readStats` <br/>`src/common/entropy_common.c:288` | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 113 | `HUF_readStats` <br/>`src/common/entropy_common.c:295` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 114 | `HUF_readStats` <br/>`src/common/entropy_common.c:301` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 115 | `FSE_buildDTable_internal` <br/>`src/common/fse_decompress.c:70` | `if (FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 116 | `FSE_buildDTable_internal` <br/>`src/common/fse_decompress.c:71` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 117 | `FSE_buildDTable_internal` <br/>`src/common/fse_decompress.c:72` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 118 | `FSE_buildDTable_internal` <br/>`src/common/fse_decompress.c:146` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 119 | `FSE_decompress_usingDTable_generic` <br/>`src/common/fse_decompress.c:188` | `CHECK_F(BIT_initDStream(&bitD, cSrc, cSrcSize));` | propagates the callee error code unchanged |
| 120 | `FSE_decompress_usingDTable_generic` <br/>`src/common/fse_decompress.c:193` | `BIT_reloadDStream(&bitD)==BIT_DStream_overflow` | `ZSTD_error_corruption_detected` |
| 121 | `FSE_decompress_usingDTable_generic` <br/>`src/common/fse_decompress.c:220` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 122 | `FSE_decompress_usingDTable_generic` <br/>`src/common/fse_decompress.c:227` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 123 | `FSE_decompress_wksp_body` <br/>`src/common/fse_decompress.c:258` | `if (wkspSize < sizeof(*wksp)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 124 | `FSE_decompress_wksp_body` <br/>`src/common/fse_decompress.c:267` | `if (tableLog > maxLog) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 125 | `FSE_decompress_wksp_body` <br/>`src/common/fse_decompress.c:273` | `if (FSE_DECOMPRESS_WKSP_SIZE(tableLog, maxSymbolValue) > wkspSize) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 126 | `FSE_decompress_wksp_body` <br/>`src/common/fse_decompress.c:278` | `CHECK_F( FSE_buildDTable_internal(dtable, wksp->ncount, maxSymbolValue, tableLog, workSpace, wkspSize) );` | propagates the callee error code unchanged |
| 127 | `POOL_thread` <br/>`src/common/pool.c:69` | `if (!ctx) { return NULL; }` | returns `NULL` |
| 128 | `POOL_create_advanced` <br/>`src/common/pool.c:120` | `if (!numThreads) { return NULL; }` | returns `NULL` |
| 129 | `POOL_create_advanced` <br/>`src/common/pool.c:123` | `if (!ctx) { return NULL; }` | returns `NULL` |
| 130 | `POOL_create_advanced` <br/>`src/common/pool.c:139` | `if (error) { POOL_free(ctx); return NULL; }` | returns `NULL` |
| 131 | `POOL_create_advanced` <br/>`src/common/pool.c:147` | `if (!ctx->threads \|\| !ctx->queue) { POOL_free(ctx); return NULL; }` | returns `NULL` |
| 132 | `POOL_create_advanced` <br/>`src/common/pool.c:154` | `return NULL;` | returns `NULL` |
| 133 | `ZSTD_pthread_create` <br/>`src/common/threading.c:76` | `if (thread==NULL) return -1;` | returns `-1` |
| 134 | `ZSTD_pthread_create` <br/>`src/common/threading.c:86` | `return -1;` | returns `-1` |
| 135 | `ZSTD_pthread_create` <br/>`src/common/threading.c:91` | `return -1;` | returns `-1` |
| 136 | `FSE_buildCTable_wksp` <br/>`src/compress/fse_compress.c:87` | `if (FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog) > wkspSize) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 137 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:269` | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ZSTD_error_dstSize_tooSmall` |
| 138 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:284` | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ZSTD_error_dstSize_tooSmall` |
| 139 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:301` | `if (remaining<1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 140 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:306` | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ZSTD_error_dstSize_tooSmall` |
| 141 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:315` | `return ERROR(GENERIC);  /* incorrect normalized distribution */` | `ZSTD_error_GENERIC` |
| 142 | `FSE_NCountWriteBound` <br/>`src/compress/fse_compress.c:320` | `return ERROR(dstSize_tooSmall);   /* Buffer overflow */` | `ZSTD_error_dstSize_tooSmall` |
| 143 | `FSE_writeNCount` <br/>`src/compress/fse_compress.c:333` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);   /* Unsupported */` | `ZSTD_error_tableLog_tooLarge` |
| 144 | `FSE_writeNCount` <br/>`src/compress/fse_compress.c:334` | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC);   /* Unsupported */` | `ZSTD_error_GENERIC` |
| 145 | `FSE_normalizeM2` <br/>`src/compress/fse_compress.c:457` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 146 | `FSE_normalizeCount` <br/>`src/compress/fse_compress.c:471` | `if (tableLog < FSE_MIN_TABLELOG) return ERROR(GENERIC);   /* Unsupported size */` | `ZSTD_error_GENERIC` |
| 147 | `FSE_normalizeCount` <br/>`src/compress/fse_compress.c:472` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);   /* Unsupported size */` | `ZSTD_error_tableLog_tooLarge` |
| 148 | `FSE_normalizeCount` <br/>`src/compress/fse_compress.c:473` | `if (tableLog < FSE_minTableLog(total, maxSymbolValue)) return ERROR(GENERIC);   /* Too small tableLog, compression potentially impossible */` | `ZSTD_error_GENERIC` |
| 149 | `HIST_count_parallel_wksp` <br/>`src/compress/hist.c:138` | `if (check && maxSymbolValue > *maxSymbolValuePtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 150 | `HIST_countFast_wksp` <br/>`src/compress/hist.c:156` | `if ((size_t)workSpace & 3) return ERROR(GENERIC);  /* must be aligned on 4-bytes boundaries */` | `ZSTD_error_GENERIC` |
| 151 | `HIST_countFast_wksp` <br/>`src/compress/hist.c:157` | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ZSTD_error_workSpace_tooSmall` |
| 152 | `HIST_count_wksp` <br/>`src/compress/hist.c:168` | `if ((size_t)workSpace & 3) return ERROR(GENERIC);  /* must be aligned on 4-bytes boundaries */` | `ZSTD_error_GENERIC` |
| 153 | `HIST_count_wksp` <br/>`src/compress/hist.c:169` | `if (workSpaceSize < HIST_WKSP_SIZE) return ERROR(workSpace_tooSmall);` | `ZSTD_error_workSpace_tooSmall` |
| 154 | `HUF_alignUpWorkspace` <br/>`src/compress/huf_compress.c:127` | `return NULL;` | returns `NULL` |
| 155 | `HUF_alignUpWorkspace` <br/>`src/compress/huf_compress.c:159` | `if (workspaceSize < sizeof(HUF_CompressWeightsWksp)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 156 | `HUF_alignUpWorkspace` <br/>`src/compress/huf_compress.c:171` | `CHECK_F( FSE_normalizeCount(wksp->norm, tableLog, wksp->count, wtSize, maxSymbolValue, /* useLowProbCount */ 0) );` | propagates the callee error code unchanged |
| 157 | `HUF_alignUpWorkspace` <br/>`src/compress/huf_compress.c:179` | `CHECK_F( FSE_buildCTable_wksp(wksp->CTable, wksp->norm, maxSymbolValue, tableLog, wksp->scratchBuffer, sizeof(wksp->scratchBuffer)) );` | propagates the callee error code unchanged |
| 158 | `HUF_writeCTable_wksp` <br/>`src/compress/huf_compress.c:263` | `if (workspaceSize < sizeof(HUF_WriteCTableWksp)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 159 | `HUF_writeCTable_wksp` <br/>`src/compress/huf_compress.c:264` | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 160 | `HUF_writeCTable_wksp` <br/>`src/compress/huf_compress.c:274` | `if (maxDstSize < 1) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 161 | `HUF_writeCTable_wksp` <br/>`src/compress/huf_compress.c:282` | `if (maxSymbolValue > (256-128)) return ERROR(GENERIC);   /* should not happen : likely means source cannot be compressed */` | `ZSTD_error_GENERIC` |
| 162 | `HUF_writeCTable_wksp` <br/>`src/compress/huf_compress.c:283` | `if (((maxSymbolValue+1)/2) + 1 > maxDstSize) return ERROR(dstSize_tooSmall);   /* not enough space within dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 163 | `HUF_readCTable` <br/>`src/compress/huf_compress.c:305` | `if (tableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 164 | `HUF_readCTable` <br/>`src/compress/huf_compress.c:306` | `if (nbSymbols > *maxSymbolValuePtr+1) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 165 | `HUF_buildCTableFromTree` <br/>`src/compress/huf_compress.c:771` | `return ERROR(workSpace_tooSmall);` | `ZSTD_error_workSpace_tooSmall` |
| 166 | `HUF_buildCTableFromTree` <br/>`src/compress/huf_compress.c:774` | `return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 167 | `HUF_buildCTableFromTree` <br/>`src/compress/huf_compress.c:786` | `if (maxNbBits > HUF_TABLELOG_MAX) return ERROR(GENERIC);   /* check fit into table */` | `ZSTD_error_GENERIC` |
| 168 | `HUF_initCStream` <br/>`src/compress/huf_compress.c:863` | `if (dstCapacity <= sizeof(bitC->bitContainer[0])) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 169 | `HUF_optimalTableLog` <br/>`src/compress/huf_compress.c:1349` | `if (wkspSize < sizeof(*table)) return ERROR(workSpace_tooSmall);` | `ZSTD_error_workSpace_tooSmall` |
| 170 | `HUF_optimalTableLog` <br/>`src/compress/huf_compress.c:1352` | `if (srcSize > HUF_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);   /* current block size limit */` | `ZSTD_error_srcSize_wrong` |
| 171 | `HUF_optimalTableLog` <br/>`src/compress/huf_compress.c:1353` | `if (huffLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 172 | `HUF_optimalTableLog` <br/>`src/compress/huf_compress.c:1354` | `if (maxSymbolValue > HUF_SYMBOLVALUE_MAX) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 173 | `HUF_optimalTableLog` <br/>`src/compress/huf_compress.c:1406` | `CHECK_F(maxBits);` | propagates the callee error code unchanged |
| 174 | `ZSTD_compressBound` <br/>`src/compress/zstd_compress.c:72` | `if (r==0) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 175 | `ZSTD_createCCtx_advanced` <br/>`src/compress/zstd_compress.c:118` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | returns `NULL` |
| 176 | `ZSTD_createCCtx_advanced` <br/>`src/compress/zstd_compress.c:120` | `if (!cctx) return NULL;` | returns `NULL` |
| 177 | `ZSTD_initStaticCCtx` <br/>`src/compress/zstd_compress.c:130` | `if (workspaceSize <= sizeof(ZSTD_CCtx)) return NULL;  /* minimum size */` | returns `NULL` |
| 178 | `ZSTD_initStaticCCtx` <br/>`src/compress/zstd_compress.c:131` | `if ((size_t)workspace & 7) return NULL;  /* must be 8-aligned */` | returns `NULL` |
| 179 | `ZSTD_initStaticCCtx` <br/>`src/compress/zstd_compress.c:135` | `if (cctx == NULL) return NULL;` | returns `NULL` |
| 180 | `ZSTD_initStaticCCtx` <br/>`src/compress/zstd_compress.c:142` | `if (!ZSTD_cwksp_check_available(&cctx->workspace, TMP_WORKSPACE_SIZE + 2 * sizeof(ZSTD_compressedBlockState_t))) return NULL;` | returns `NULL` |
| 181 | `ZSTD_freeCCtx` <br/>`src/compress/zstd_compress.c:185` | `cctx->staticSize` | `ZSTD_error_memory_allocation` |
| 182 | `ZSTD_createCCtxParams_advanced` <br/>`src/compress/zstd_compress.c:332` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | returns `NULL` |
| 183 | `ZSTD_createCCtxParams_advanced` <br/>`src/compress/zstd_compress.c:335` | `if (!params) { return NULL; }` | returns `NULL` |
| 184 | `ZSTD_CCtxParams_init` <br/>`src/compress/zstd_compress.c:359` | `!cctxParams` | `ZSTD_error_GENERIC` |
| 185 | `ZSTD_CCtxParams_init_advanced` <br/>`src/compress/zstd_compress.c:397` | `!cctxParams` | `ZSTD_error_GENERIC` |
| 186 | `ZSTD_cParam_getBounds` <br/>`src/compress/zstd_compress.c:634` | `bounds.error = ERROR(parameter_unsupported);` | `ZSTD_error_parameter_unsupported` |
| 187 | `ZSTD_cParam_clampBounds` <br/>`src/compress/zstd_compress.c:651` | `#define BOUNDCHECK(cParam, val)                                       \` | propagates the callee error code unchanged |
| 188 | `ZSTD_cParam_clampBounds` <br/>`src/compress/zstd_compress.c:653` | `!ZSTD_cParam_withinBounds(cParam` | `ZSTD_error_val` |
| 189 | `ZSTD_CCtx_setParameter` <br/>`src/compress/zstd_compress.c:715` | `RETURN_ERROR(stage_wrong, "can only set params in cctx init stage");` | `ZSTD_error_stage_wrong` |
| 190 | `ZSTD_CCtx_setParameter` <br/>`src/compress/zstd_compress.c:721` | `(value!=0) && cctx->staticSize` | `ZSTD_error_parameter_unsupported` |
| 191 | `ZSTD_CCtx_setParameter` <br/>`src/compress/zstd_compress.c:765` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ZSTD_error_parameter_unsupported` |
| 192 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:777` | `BOUNDCHECK(ZSTD_c_format, value);` | propagates the callee error code unchanged |
| 193 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:793` | `BOUNDCHECK(ZSTD_c_windowLog, value);` | propagates the callee error code unchanged |
| 194 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:799` | `BOUNDCHECK(ZSTD_c_hashLog, value);` | propagates the callee error code unchanged |
| 195 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:805` | `BOUNDCHECK(ZSTD_c_chainLog, value);` | propagates the callee error code unchanged |
| 196 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:811` | `BOUNDCHECK(ZSTD_c_searchLog, value);` | propagates the callee error code unchanged |
| 197 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:817` | `BOUNDCHECK(ZSTD_c_minMatch, value);` | propagates the callee error code unchanged |
| 198 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:822` | `BOUNDCHECK(ZSTD_c_targetLength, value);` | propagates the callee error code unchanged |
| 199 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:828` | `BOUNDCHECK(ZSTD_c_strategy, value);` | propagates the callee error code unchanged |
| 200 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:854` | `BOUNDCHECK(ZSTD_c_forceAttachDict, (int)pref);` | propagates the callee error code unchanged |
| 201 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:861` | `BOUNDCHECK(ZSTD_c_literalCompressionMode, (int)lcm);` | propagates the callee error code unchanged |
| 202 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:868` | `value!=0` | `ZSTD_error_parameter_unsupported` |
| 203 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:878` | `value!=0` | `ZSTD_error_parameter_unsupported` |
| 204 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:892` | `value!=0` | `ZSTD_error_parameter_unsupported` |
| 205 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:902` | `value!=0` | `ZSTD_error_parameter_unsupported` |
| 206 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:915` | `BOUNDCHECK(ZSTD_c_enableLongDistanceMatching, value);` | propagates the callee error code unchanged |
| 207 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:921` | `BOUNDCHECK(ZSTD_c_ldmHashLog, value);` | propagates the callee error code unchanged |
| 208 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:927` | `BOUNDCHECK(ZSTD_c_ldmMinMatch, value);` | propagates the callee error code unchanged |
| 209 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:933` | `BOUNDCHECK(ZSTD_c_ldmBucketSizeLog, value);` | propagates the callee error code unchanged |
| 210 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:939` | `BOUNDCHECK(ZSTD_c_ldmHashRateLog, value);` | propagates the callee error code unchanged |
| 211 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:946` | `BOUNDCHECK(ZSTD_c_targetCBlockSize, value);` | propagates the callee error code unchanged |
| 212 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:953` | `BOUNDCHECK(ZSTD_c_srcSizeHint, value);` | propagates the callee error code unchanged |
| 213 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:958` | `BOUNDCHECK(ZSTD_c_stableInBuffer, value);` | propagates the callee error code unchanged |
| 214 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:963` | `BOUNDCHECK(ZSTD_c_stableOutBuffer, value);` | propagates the callee error code unchanged |
| 215 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:968` | `BOUNDCHECK(ZSTD_c_blockDelimiters, value);` | propagates the callee error code unchanged |
| 216 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:973` | `BOUNDCHECK(ZSTD_c_validateSequences, value);` | propagates the callee error code unchanged |
| 217 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:978` | `BOUNDCHECK(ZSTD_c_splitAfterSequences, value);` | propagates the callee error code unchanged |
| 218 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:983` | `BOUNDCHECK(ZSTD_c_blockSplitterLevel, value);` | propagates the callee error code unchanged |
| 219 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:988` | `BOUNDCHECK(ZSTD_c_useRowMatchFinder, value);` | propagates the callee error code unchanged |
| 220 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:993` | `BOUNDCHECK(ZSTD_c_deterministicRefPrefix, value);` | propagates the callee error code unchanged |
| 221 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:998` | `BOUNDCHECK(ZSTD_c_prefetchCDictTables, value);` | propagates the callee error code unchanged |
| 222 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:1003` | `BOUNDCHECK(ZSTD_c_enableSeqProducerFallback, value);` | propagates the callee error code unchanged |
| 223 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:1009` | `BOUNDCHECK(ZSTD_c_maxBlockSize, value);` | propagates the callee error code unchanged |
| 224 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:1015` | `BOUNDCHECK(ZSTD_c_repcodeResolution, value);` | propagates the callee error code unchanged |
| 225 | `ZSTD_CCtxParams_setParameter` <br/>`src/compress/zstd_compress.c:1019` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ZSTD_error_parameter_unsupported` |
| 226 | `ZSTD_CCtxParams_getParameter` <br/>`src/compress/zstd_compress.c:1086` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ZSTD_error_parameter_unsupported` |
| 227 | `ZSTD_CCtxParams_getParameter` <br/>`src/compress/zstd_compress.c:1094` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ZSTD_error_parameter_unsupported` |
| 228 | `ZSTD_CCtxParams_getParameter` <br/>`src/compress/zstd_compress.c:1101` | `RETURN_ERROR(parameter_unsupported, "not compiled with multithreading");` | `ZSTD_error_parameter_unsupported` |
| 229 | `ZSTD_CCtxParams_getParameter` <br/>`src/compress/zstd_compress.c:1166` | `default: RETURN_ERROR(parameter_unsupported, "unknown parameter");` | `ZSTD_error_parameter_unsupported` |
| 230 | `ZSTD_CCtx_setParametersUsingCCtxParams` <br/>`src/compress/zstd_compress.c:1182` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 231 | `ZSTD_CCtx_setParametersUsingCCtxParams` <br/>`src/compress/zstd_compress.c:1184` | `cctx->cdict` | `ZSTD_error_stage_wrong` |
| 232 | `ZSTD_CCtx_setPledgedSrcSize` <br/>`src/compress/zstd_compress.c:1233` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 233 | `ZSTD_initLocalDict` <br/>`src/compress/zstd_compress.c:1278` | `!dl->cdict` | `ZSTD_error_memory_allocation` |
| 234 | `ZSTD_CCtx_loadDictionary_advanced` <br/>`src/compress/zstd_compress.c:1290` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 235 | `ZSTD_CCtx_loadDictionary_advanced` <br/>`src/compress/zstd_compress.c:1300` | `cctx->staticSize` | `ZSTD_error_memory_allocation` |
| 236 | `ZSTD_CCtx_loadDictionary_advanced` <br/>`src/compress/zstd_compress.c:1303` | `dictBuffer==NULL` | `ZSTD_error_memory_allocation` |
| 237 | `ZSTD_CCtx_refCDict` <br/>`src/compress/zstd_compress.c:1330` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 238 | `ZSTD_CCtx_refThreadPool` <br/>`src/compress/zstd_compress.c:1340` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 239 | `ZSTD_CCtx_refPrefix_advanced` <br/>`src/compress/zstd_compress.c:1354` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 240 | `ZSTD_CCtx_reset` <br/>`src/compress/zstd_compress.c:1376` | `cctx->streamStage != zcss_init` | `ZSTD_error_stage_wrong` |
| 241 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1390` | `BOUNDCHECK(ZSTD_c_windowLog, (int)cParams.windowLog);` | propagates the callee error code unchanged |
| 242 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1391` | `BOUNDCHECK(ZSTD_c_chainLog,  (int)cParams.chainLog);` | propagates the callee error code unchanged |
| 243 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1392` | `BOUNDCHECK(ZSTD_c_hashLog,   (int)cParams.hashLog);` | propagates the callee error code unchanged |
| 244 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1393` | `BOUNDCHECK(ZSTD_c_searchLog, (int)cParams.searchLog);` | propagates the callee error code unchanged |
| 245 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1394` | `BOUNDCHECK(ZSTD_c_minMatch,  (int)cParams.minMatch);` | propagates the callee error code unchanged |
| 246 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1395` | `BOUNDCHECK(ZSTD_c_targetLength,(int)cParams.targetLength);` | propagates the callee error code unchanged |
| 247 | `ZSTD_checkCParams` <br/>`src/compress/zstd_compress.c:1396` | `BOUNDCHECK(ZSTD_c_strategy,  (int)cParams.strategy);` | propagates the callee error code unchanged |
| 248 | `ZSTD_estimateCCtxSize_usingCCtxParams` <br/>`src/compress/zstd_compress.c:1761` | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` |
| 249 | `ZSTD_estimateCStreamSize_usingCCtxParams` <br/>`src/compress/zstd_compress.c:1813` | `params->nbWorkers > 0` | `ZSTD_error_GENERIC` |
| 250 | `ZSTD_advanceHashSalt` <br/>`src/compress/zstd_compress.c:2023` | `ZSTD_cwksp_reserve_failed(ws)` | `ZSTD_error_memory_allocation` |
| 251 | `ZSTD_advanceHashSalt` <br/>`src/compress/zstd_compress.c:2066` | `ZSTD_cwksp_reserve_failed(ws)` | `ZSTD_error_memory_allocation` |
| 252 | `ZSTD_resetCCtx_internal` <br/>`src/compress/zstd_compress.c:2168` | `zc->staticSize` | `ZSTD_error_memory_allocation` |
| 253 | `ZSTD_resetCCtx_internal` <br/>`src/compress/zstd_compress.c:2181` | `zc->blockState.prevCBlock == NULL` | `ZSTD_error_memory_allocation` |
| 254 | `ZSTD_resetCCtx_internal` <br/>`src/compress/zstd_compress.c:2183` | `zc->blockState.nextCBlock == NULL` | `ZSTD_error_memory_allocation` |
| 255 | `ZSTD_resetCCtx_internal` <br/>`src/compress/zstd_compress.c:2185` | `zc->tmpWorkspace == NULL` | `ZSTD_error_memory_allocation` |
| 256 | `ZSTD_copyCCtx_internal` <br/>`src/compress/zstd_compress.c:2519` | `srcCCtx->stage!=ZSTDcs_init` | `ZSTD_error_stage_wrong` |
| 257 | `ZSTD_blockSplitterEnabled` <br/>`src/compress/zstd_compress.c:2940` | `(oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/` | `ZSTD_error_dstSize_tooSmall` |
| 258 | `ZSTD_postProcessSequenceProducerResult` <br/>`src/compress/zstd_compress.c:3177` | `nbExternalSeqs > outSeqsCapacity` | `ZSTD_error_sequenceProducer_failed` |
| 259 | `ZSTD_postProcessSequenceProducerResult` <br/>`src/compress/zstd_compress.c:3184` | `nbExternalSeqs == 0 && srcSize > 0` | `ZSTD_error_sequenceProducer_failed` |
| 260 | `ZSTD_postProcessSequenceProducerResult` <br/>`src/compress/zstd_compress.c:3205` | `nbExternalSeqs == outSeqsCapacity` | `ZSTD_error_sequenceProducer_failed` |
| 261 | `ZSTD_buildSeqStore` <br/>`src/compress/zstd_compress.c:3312` | `ZSTD_hasExtSeqProd(&zc->appliedParams)` | `ZSTD_error_parameter_combination_unsupported` |
| 262 | `ZSTD_buildSeqStore` <br/>`src/compress/zstd_compress.c:3331` | `ZSTD_hasExtSeqProd(&zc->appliedParams)` | `ZSTD_error_parameter_combination_unsupported` |
| 263 | `ZSTD_buildSeqStore` <br/>`src/compress/zstd_compress.c:3380` | `seqLenSum > srcSize` | `ZSTD_error_externalSequences_invalid` |
| 264 | `ZSTD_copyBlockSequences` <br/>`src/compress/zstd_compress.c:3445` | `nbOutSequences > (size_t)(seqCollector->maxSequences - seqCollector->seqIndex)` | `ZSTD_error_dstSize_tooSmall` |
| 265 | `ZSTD_generateSequences` <br/>`src/compress/zstd_compress.c:3529` | `targetCBlockSize != 0` | `ZSTD_error_parameter_unsupported` |
| 266 | `ZSTD_generateSequences` <br/>`src/compress/zstd_compress.c:3534` | `nbWorkers != 0` | `ZSTD_error_parameter_unsupported` |
| 267 | `ZSTD_generateSequences` <br/>`src/compress/zstd_compress.c:3538` | `dst == NULL` | `ZSTD_error_memory_allocation` |
| 268 | `ZSTD_deriveSeqStoreChunk` <br/>`src/compress/zstd_compress.c:4124` | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` |
| 269 | `ZSTD_deriveBlockSplits` <br/>`src/compress/zstd_compress.c:4368` | `zc->seqCollector.collectSequences` | `ZSTD_error_sequenceProducer_failed` |
| 270 | `ZSTD_deriveBlockSplits` <br/>`src/compress/zstd_compress.c:4402` | `zc->seqCollector.collectSequences` | `ZSTD_error_sequenceProducer_failed` |
| 271 | `ZSTD_compress_frameChunk` <br/>`src/compress/zstd_compress.c:4623` | `dstCapacity < ZSTD_blockHeaderSize + MIN_CBLOCK_SIZE + 1` | `ZSTD_error_dstSize_tooSmall` |
| 272 | `ZSTD_writeFrameHeader` <br/>`src/compress/zstd_compress.c:4712` | `dstCapacity < ZSTD_FRAMEHEADERSIZE_MAX` | `ZSTD_error_dstSize_tooSmall` |
| 273 | `ZSTD_writeSkippableFrame` <br/>`src/compress/zstd_compress.c:4754` | `dstCapacity < srcSize + ZSTD_SKIPPABLEHEADERSIZE /* Skippable frame overhead */` | `ZSTD_error_dstSize_tooSmall` |
| 274 | `ZSTD_writeSkippableFrame` <br/>`src/compress/zstd_compress.c:4756` | `srcSize > (unsigned)0xFFFFFFFF` | `ZSTD_error_srcSize_wrong` |
| 275 | `ZSTD_writeSkippableFrame` <br/>`src/compress/zstd_compress.c:4757` | `magicVariant > 15` | `ZSTD_error_parameter_outOfBound` |
| 276 | `ZSTD_writeLastEmptyBlock` <br/>`src/compress/zstd_compress.c:4772` | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` |
| 277 | `ZSTD_compressContinue_internal` <br/>`src/compress/zstd_compress.c:4802` | `cctx->stage==ZSTDcs_created` | `ZSTD_error_stage_wrong` |
| 278 | `ZSTD_compressContinue_internal` <br/>`src/compress/zstd_compress.c:4842` | `cctx->consumedSrcSize+1 > cctx->pledgedSrcSizePlusOne` | `ZSTD_error_srcSize_wrong` |
| 279 | `ZSTD_compressBlock_deprecated` <br/>`src/compress/zstd_compress.c:4887` | `srcSize > blockSizeMax` | `ZSTD_error_srcSize_wrong` |
| 280 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5081` | `HUF_isError(hufHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 281 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5087` | `FSE_isError(offcodeHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 282 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5088` | `offcodeLog > OffFSELog` | `ZSTD_error_dictionary_corrupted` |
| 283 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5090` | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.offcodeCTable` | `ZSTD_error_offcodeNCount` |
| 284 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5102` | `FSE_isError(matchlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 285 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5103` | `matchlengthLog > MLFSELog` | `ZSTD_error_dictionary_corrupted` |
| 286 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5104` | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.matchlengthCTable` | `ZSTD_error_matchlengthNCount` |
| 287 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5116` | `FSE_isError(litlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 288 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5117` | `litlengthLog > LLFSELog` | `ZSTD_error_dictionary_corrupted` |
| 289 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5118` | `FSE_isError(FSE_buildCTable_wksp( bs->entropy.fse.litlengthCTable` | `ZSTD_error_litlengthNCount` |
| 290 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5127` | `dictPtr+12 > dictEnd` | `ZSTD_error_dictionary_corrupted` |
| 291 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5145` | `bs->rep[u] == 0` | `ZSTD_error_dictionary_corrupted` |
| 292 | `ZSTD_loadCEntropy` <br/>`src/compress/zstd_compress.c:5146` | `bs->rep[u] > dictContentSize` | `ZSTD_error_dictionary_corrupted` |
| 293 | `ZSTD_loadZstdDictionary` <br/>`src/compress/zstd_compress.c:5207` | `dictContentType == ZSTD_dct_fullDict` | `ZSTD_error_dictionary_wrong` |
| 294 | `ZSTD_loadZstdDictionary` <br/>`src/compress/zstd_compress.c:5223` | `dictContentType == ZSTD_dct_fullDict` | `ZSTD_error_dictionary_wrong` |
| 295 | `ZSTD_writeEpilogue` <br/>`src/compress/zstd_compress.c:5350` | `cctx->stage == ZSTDcs_created` | `ZSTD_error_stage_wrong` |
| 296 | `ZSTD_writeEpilogue` <br/>`src/compress/zstd_compress.c:5365` | `dstCapacity<3` | `ZSTD_error_dstSize_tooSmall` |
| 297 | `ZSTD_writeEpilogue` <br/>`src/compress/zstd_compress.c:5373` | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` |
| 298 | `ZSTD_compressEnd_public` <br/>`src/compress/zstd_compress.c:5422` | `cctx->pledgedSrcSizePlusOne != cctx->consumedSrcSize+1` | `ZSTD_error_srcSize_wrong` |
| 299 | `ZSTD_compress` <br/>`src/compress/zstd_compress.c:5504` | `!cctx` | `ZSTD_error_memory_allocation` |
| 300 | `ZSTD_initCDict_internal` <br/>`src/compress/zstd_compress.c:5566` | `!internalBuffer` | `ZSTD_error_memory_allocation` |
| 301 | `ZSTD_initCDict_internal` <br/>`src/compress/zstd_compress.c:5612` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | returns `NULL` |
| 302 | `ZSTD_initCDict_internal` <br/>`src/compress/zstd_compress.c:5627` | `return NULL;` | returns `NULL` |
| 303 | `ZSTD_createCDict_advanced2` <br/>`src/compress/zstd_compress.c:5672` | `if (!customMem.customAlloc ^ !customMem.customFree) return NULL;` | returns `NULL` |
| 304 | `ZSTD_createCDict_advanced2` <br/>`src/compress/zstd_compress.c:5704` | `return NULL;` | returns `NULL` |
| 305 | `ZSTD_initStaticCDict` <br/>`src/compress/zstd_compress.c:5777` | `if ((size_t)workspace & 7) return NULL;  /* 8-aligned */` | returns `NULL` |
| 306 | `ZSTD_initStaticCDict` <br/>`src/compress/zstd_compress.c:5783` | `if (cdict == NULL) return NULL;` | returns `NULL` |
| 307 | `ZSTD_initStaticCDict` <br/>`src/compress/zstd_compress.c:5787` | `if (workspaceSize < neededSize) return NULL;` | returns `NULL` |
| 308 | `ZSTD_initStaticCDict` <br/>`src/compress/zstd_compress.c:5799` | `return NULL;` | returns `NULL` |
| 309 | `ZSTD_compressBegin_usingCDict_internal` <br/>`src/compress/zstd_compress.c:5829` | `cdict==NULL` | `ZSTD_error_dictionary_wrong` |
| 310 | `ZSTD_compressStream_generic` <br/>`src/compress/zstd_compress.c:6143` | `RETURN_ERROR(init_missing, "call ZSTD_initCStream() first!");` | `ZSTD_error_init_missing` |
| 311 | `ZSTD_checkBufferStability` <br/>`src/compress/zstd_compress.c:6333` | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableInBuffer enabled but input differs!");` | `ZSTD_error_stabilityCondition_notRespected` |
| 312 | `ZSTD_checkBufferStability` <br/>`src/compress/zstd_compress.c:6339` | `RETURN_ERROR(stabilityCondition_notRespected, "ZSTD_c_stableOutBuffer enabled but output size differs!");` | `ZSTD_error_stabilityCondition_notRespected` |
| 313 | `ZSTD_CCtx_init_compressStream2` <br/>`src/compress/zstd_compress.c:6386` | `ZSTD_hasExtSeqProd(&params) && params.nbWorkers >= 1` | `ZSTD_error_parameter_combination_unsupported` |
| 314 | `ZSTD_CCtx_init_compressStream2` <br/>`src/compress/zstd_compress.c:6404` | `cctx->mtctx == NULL` | `ZSTD_error_memory_allocation` |
| 315 | `ZSTD_compressStream2` <br/>`src/compress/zstd_compress.c:6454` | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` |
| 316 | `ZSTD_compressStream2` <br/>`src/compress/zstd_compress.c:6455` | `input->pos  > input->size` | `ZSTD_error_srcSize_wrong` |
| 317 | `ZSTD_compressStream2` <br/>`src/compress/zstd_compress.c:6456` | `(U32)endOp > (U32)ZSTD_e_end` | `ZSTD_error_parameter_outOfBound` |
| 318 | `ZSTD_compressStream2` <br/>`src/compress/zstd_compress.c:6468` | `input->src != cctx->expectedInBuffer.src` | `ZSTD_error_stabilityCondition_notRespected` |
| 319 | `ZSTD_compressStream2` <br/>`src/compress/zstd_compress.c:6469` | `input->pos != cctx->expectedInBuffer.size` | `ZSTD_error_stabilityCondition_notRespected` |
| 320 | `ZSTD_compress2` <br/>`src/compress/zstd_compress.c:6592` | `RETURN_ERROR(dstSize_tooSmall, "");` | `ZSTD_error_dstSize_tooSmall` |
| 321 | `ZSTD_compress2` <br/>`src/compress/zstd_compress.c:6615` | `offBase > OFFSET_TO_OFFBASE(offsetBound)` | `ZSTD_error_externalSequences_invalid` |
| 322 | `ZSTD_compress2` <br/>`src/compress/zstd_compress.c:6617` | `matchLength < matchLenLowerBound` | `ZSTD_error_externalSequences_invalid` |
| 323 | `ZSTD_finalizeOffBase` <br/>`src/compress/zstd_compress.c:6690` | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` |
| 324 | `ZSTD_finalizeOffBase` <br/>`src/compress/zstd_compress.c:6695` | `idx == inSeqsSize` | `ZSTD_error_externalSequences_invalid` |
| 325 | `ZSTD_finalizeOffBase` <br/>`src/compress/zstd_compress.c:6728` | `ip != iend` | `ZSTD_error_externalSequences_invalid` |
| 326 | `ZSTD_finalizeOffBase` <br/>`src/compress/zstd_compress.c:6844` | `idx - seqPos->idx >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` |
| 327 | `ZSTD_selectSequenceCopier` <br/>`src/compress/zstd_compress.c:6908` | `RETURN_ERROR(externalSequences_invalid, "delimiter format error : both matchlength and offset must be == 0");` | `ZSTD_error_externalSequences_invalid` |
| 328 | `ZSTD_selectSequenceCopier` <br/>`src/compress/zstd_compress.c:6914` | `RETURN_ERROR(externalSequences_invalid, "Reached end of sequences without finding a block delimiter");` | `ZSTD_error_externalSequences_invalid` |
| 329 | `determine_blockSize` <br/>`src/compress/zstd_compress.c:6932` | `RETURN_ERROR(externalSequences_invalid, "sequences incorrectly define a too large block");` | `ZSTD_error_externalSequences_invalid` |
| 330 | `determine_blockSize` <br/>`src/compress/zstd_compress.c:6934` | `RETURN_ERROR(externalSequences_invalid, "sequences define a frame longer than source");` | `ZSTD_error_externalSequences_invalid` |
| 331 | `determine_blockSize` <br/>`src/compress/zstd_compress.c:6962` | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` |
| 332 | `determine_blockSize` <br/>`src/compress/zstd_compress.c:7001` | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` |
| 333 | `ZSTD_compressSequences` <br/>`src/compress/zstd_compress.c:7102` | `dstCapacity<4` | `ZSTD_error_dstSize_tooSmall` |
| 334 | `ZSTD_convertBlockSequences` <br/>`src/compress/zstd_compress.c:7327` | `nbSequences >= cctx->seqStore.maxNbSeq` | `ZSTD_error_externalSequences_invalid` |
| 335 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7435` | `bs.nbSequences = ERROR(externalSequences_invalid);` | `ZSTD_error_externalSequences_invalid` |
| 336 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7464` | `bs.nbSequences = ERROR(externalSequences_invalid);` | `ZSTD_error_externalSequences_invalid` |
| 337 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7490` | `nbSequences == 0` | `ZSTD_error_externalSequences_invalid` |
| 338 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7495` | `dstCapacity<3` | `ZSTD_error_dstSize_tooSmall` |
| 339 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7508` | `block.litSize > litSize` | `ZSTD_error_externalSequences_invalid` |
| 340 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7524` | `dstCapacity < ZSTD_blockHeaderSize` | `ZSTD_error_dstSize_tooSmall` |
| 341 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7550` | `RETURN_ERROR(cannotProduce_uncompressedBlock, "ZSTD_compressSequencesAndLiterals cannot generate an uncompressed block");` | `ZSTD_error_cannotProduce_uncompressedBlock` |
| 342 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7578` | `litSize != 0` | `ZSTD_error_externalSequences_invalid` |
| 343 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7579` | `remaining != 0` | `ZSTD_error_externalSequences_invalid` |
| 344 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7598` | `RETURN_ERROR(workSpace_tooSmall, "literals buffer is not large enough: must be at least 8 bytes larger than litSize (risk of read out-of-...` | `ZSTD_error_workSpace_tooSmall` |
| 345 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7603` | `RETURN_ERROR(frameParameter_unsupported, "This mode is only compatible with explicit delimiters");` | `ZSTD_error_frameParameter_unsupported` |
| 346 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7606` | `RETURN_ERROR(parameter_unsupported, "This mode is not compatible with Sequence validation");` | `ZSTD_error_parameter_unsupported` |
| 347 | `ZSTD_get1BlockSummary` <br/>`src/compress/zstd_compress.c:7609` | `RETURN_ERROR(frameParameter_unsupported, "this mode is not compatible with frame checksum");` | `ZSTD_error_frameParameter_unsupported` |
| 348 | `ZSTD_noCompressLiterals` <br/>`src/compress/zstd_compress_literals.c:46` | `srcSize + flSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` |
| 349 | `ZSTD_compressLiterals` <br/>`src/compress/zstd_compress_literals.c:161` | `dstCapacity < lhSize+1` | `ZSTD_error_dstSize_tooSmall` |
| 350 | `ZSTD_fseBitCost` <br/>`src/compress/zstd_compress_sequences.c:117` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 351 | `ZSTD_fseBitCost` <br/>`src/compress/zstd_compress_sequences.c:127` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 352 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:206` | `size_t const basicCost = isDefaultAllowed ? ZSTD_crossEntropyCost(defaultNorm, defaultNormLog, count, max) : ERROR(GENERIC);` | propagates the callee error code unchanged |
| 353 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:207` | `size_t const repeatCost = *repeatMode != FSE_repeat_none ? ZSTD_fseBitCost(prevCTable, count, max) : ERROR(GENERIC);` | propagates the callee error code unchanged |
| 354 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:258` | `dstCapacity==0` | `ZSTD_error_dstSize_tooSmall` |
| 355 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:286` | `default: assert(0); RETURN_ERROR(GENERIC, "impossible to reach");` | `ZSTD_error_GENERIC` |
| 356 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:303` | `ERR_isError(BIT_initCStream(&blockStream` | `ZSTD_error_dst` |
| 357 | `ZSTD_crossEntropyCost` <br/>`src/compress/zstd_compress_sequences.c:379` | `streamSize==0` | `ZSTD_error_dstSize_tooSmall` |
| 358 | `<file scope>` <br/>`src/compress/zstd_compress_superblock.c:181` | `(oend-op) < 3 /*max nbSeq Size*/ + 1 /*seqHead*/` | `ZSTD_error_dstSize_tooSmall` |
| 359 | `ZSTD_estimateSubBlockSize_symbolType` <br/>`src/compress/zstd_compress_superblock.c:350` | `: ERROR(GENERIC);` | propagates the callee error code unchanged |
| 360 | `ZSTD_ldm_generateSequences_internal` <br/>`src/compress/zstd_ldm.c:479` | `return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 361 | `ZSTDMT_createBufferPool` <br/>`src/compress/zstdmt_compress.c:126` | `if (bufPool==NULL) return NULL;` | returns `NULL` |
| 362 | `ZSTDMT_createBufferPool` <br/>`src/compress/zstdmt_compress.c:129` | `return NULL;` | returns `NULL` |
| 363 | `ZSTDMT_createBufferPool` <br/>`src/compress/zstdmt_compress.c:134` | `return NULL;` | returns `NULL` |
| 364 | `ZSTDMT_expandBufferPool` <br/>`src/compress/zstdmt_compress.c:173` | `if (srcBufPool==NULL) return NULL;` | returns `NULL` |
| 365 | `ZSTDMT_createSeqPool` <br/>`src/compress/zstdmt_compress.c:337` | `if (seqPool == NULL) return NULL;` | returns `NULL` |
| 366 | `ZSTDMT_createCCtxPool` <br/>`src/compress/zstdmt_compress.c:386` | `if (!cctxPool) return NULL;` | returns `NULL` |
| 367 | `ZSTDMT_createCCtxPool` <br/>`src/compress/zstdmt_compress.c:389` | `return NULL;` | returns `NULL` |
| 368 | `ZSTDMT_createCCtxPool` <br/>`src/compress/zstdmt_compress.c:395` | `return NULL;` | returns `NULL` |
| 369 | `ZSTDMT_createCCtxPool` <br/>`src/compress/zstdmt_compress.c:399` | `if (!cctxPool->cctxs[0]) { ZSTDMT_freeCCtxPool(cctxPool); return NULL; }` | returns `NULL` |
| 370 | `ZSTDMT_expandCCtxPool` <br/>`src/compress/zstdmt_compress.c:408` | `if (srcPool==NULL) return NULL;` | returns `NULL` |
| 371 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:731` | `if (ZSTD_isError(initError)) JOB_ERROR(initError);` | propagates the callee error code unchanged |
| 372 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:735` | `if (ZSTD_isError(forceWindowError)) JOB_ERROR(forceWindowError);` | propagates the callee error code unchanged |
| 373 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:739` | `if (ZSTD_isError(err)) JOB_ERROR(err);` | propagates the callee error code unchanged |
| 374 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:747` | `if (ZSTD_isError(initError)) JOB_ERROR(initError);` | propagates the callee error code unchanged |
| 375 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:755` | `if (ZSTD_isError(hSize)) JOB_ERROR(hSize);` | propagates the callee error code unchanged |
| 376 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:773` | `if (ZSTD_isError(cSize)) JOB_ERROR(cSize);` | propagates the callee error code unchanged |
| 377 | `ZSTDMT_compressionJob` <br/>`src/compress/zstdmt_compress.c:794` | `if (ZSTD_isError(cSize)) JOB_ERROR(cSize);` | propagates the callee error code unchanged |
| 378 | `ZSTDMT_createJobsTable` <br/>`src/compress/zstdmt_compress.c:916` | `if (jobTable==NULL) return NULL;` | returns `NULL` |
| 379 | `ZSTDMT_createJobsTable` <br/>`src/compress/zstdmt_compress.c:924` | `return NULL;` | returns `NULL` |
| 380 | `ZSTDMT_expandJobsTable` <br/>`src/compress/zstdmt_compress.c:935` | `if (mtctx->jobs==NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 381 | `ZSTDMT_createCCtx_advanced_internal` <br/>`src/compress/zstdmt_compress.c:957` | `if (nbWorkers < 1) return NULL;` | returns `NULL` |
| 382 | `ZSTDMT_createCCtx_advanced_internal` <br/>`src/compress/zstdmt_compress.c:961` | `return NULL;` | returns `NULL` |
| 383 | `ZSTDMT_createCCtx_advanced_internal` <br/>`src/compress/zstdmt_compress.c:964` | `if (!mtctx) return NULL;` | returns `NULL` |
| 384 | `ZSTDMT_createCCtx_advanced_internal` <br/>`src/compress/zstdmt_compress.c:986` | `return NULL;` | returns `NULL` |
| 385 | `ZSTDMT_createCCtx_advanced` <br/>`src/compress/zstdmt_compress.c:1000` | `return NULL;` | returns `NULL` |
| 386 | `ZSTDMT_resize` <br/>`src/compress/zstdmt_compress.c:1080` | `if (POOL_resize(mtctx->factory, nbWorkers)) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 387 | `ZSTDMT_resize` <br/>`src/compress/zstdmt_compress.c:1083` | `if (mtctx->bufPool == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 388 | `ZSTDMT_resize` <br/>`src/compress/zstdmt_compress.c:1085` | `if (mtctx->cctxPool == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 389 | `ZSTDMT_resize` <br/>`src/compress/zstdmt_compress.c:1087` | `if (mtctx->seqPool == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 390 | `ZSTDMT_initCStream_internal` <br/>`src/compress/zstdmt_compress.c:1283` | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 391 | `ZSTDMT_initCStream_internal` <br/>`src/compress/zstdmt_compress.c:1334` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 392 | `ZSTDMT_initCStream_internal` <br/>`src/compress/zstdmt_compress.c:1365` | `if (mtctx->cdictLocal == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 393 | `ZSTDMT_initCStream_internal` <br/>`src/compress/zstdmt_compress.c:1373` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 394 | `ZSTDMT_writeLastEmptyBlock` <br/>`src/compress/zstdmt_compress.c:1393` | `job->cSize = ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 395 | `ZSTDMT_compressStream_generic` <br/>`src/compress/zstdmt_compress.c:1866` | `return ERROR(stage_wrong);` | `ZSTD_error_stage_wrong` |
| 396 | `HUF_DecompressFastArgs_init` <br/>`src/decompress/huf_decompress.c:213` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 397 | `HUF_DecompressFastArgs_init` <br/>`src/decompress/huf_decompress.c:238` | `if (length4 > srcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 398 | `HUF_initRemainingDStream` <br/>`src/decompress/huf_decompress.c:285` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 399 | `HUF_initRemainingDStream` <br/>`src/decompress/huf_decompress.c:292` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 400 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:395` | `if (sizeof(*wksp) > wkspSize) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 401 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:409` | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge);   /* DTable too small, Huffman tree cannot fit in */` | `ZSTD_error_tableLog_tooLarge` |
| 402 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:588` | `CHECK_F( BIT_initDStream(&bitD, cSrc, cSrcSize) );` | propagates the callee error code unchanged |
| 403 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:592` | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 404 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:608` | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 405 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:609` | `if (dstSize < 6) return ERROR(corruption_detected);         /* stream 4-split doesn't work */` | `ZSTD_error_corruption_detected` |
| 406 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:643` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 407 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:644` | `if (opStart4 > oend) return ERROR(corruption_detected);      /* overflow */` | `ZSTD_error_corruption_detected` |
| 408 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:646` | `CHECK_F( BIT_initDStream(&bitD1, istart1, length1) );` | propagates the callee error code unchanged |
| 409 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:647` | `CHECK_F( BIT_initDStream(&bitD2, istart2, length2) );` | propagates the callee error code unchanged |
| 410 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:648` | `CHECK_F( BIT_initDStream(&bitD3, istart3, length3) );` | propagates the callee error code unchanged |
| 411 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:649` | `CHECK_F( BIT_initDStream(&bitD4, istart4, length4) );` | propagates the callee error code unchanged |
| 412 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:680` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 413 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:681` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 414 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:682` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 415 | `HUF_readDTableX1_wksp` <br/>`src/decompress/huf_decompress.c:693` | `if (!endCheck) return ERROR(corruption_detected); }` | `ZSTD_error_corruption_detected` |
| 416 | `HUF_decompress4X1_usingDTable_internal_fast_c_loop` <br/>`src/decompress/huf_decompress.c:886` | `if (args.op[i] != segmentEnd) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 417 | `HUF_decompress4X1_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:938` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 418 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1193` | `if (sizeof(*wksp) > wkspSize) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 419 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1200` | `if (maxTableLog > HUF_TABLELOG_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 420 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1207` | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 421 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1361` | `CHECK_F( BIT_initDStream(&bitD, cSrc, cSrcSize) );` | propagates the callee error code unchanged |
| 422 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1373` | `if (!BIT_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 423 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1389` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 424 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1390` | `if (dstSize < 6) return ERROR(corruption_detected);         /* stream 4-split doesn't work */` | `ZSTD_error_corruption_detected` |
| 425 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1424` | `if (length4 > cSrcSize) return ERROR(corruption_detected);  /* overflow */` | `ZSTD_error_corruption_detected` |
| 426 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1425` | `if (opStart4 > oend) return ERROR(corruption_detected);     /* overflow */` | `ZSTD_error_corruption_detected` |
| 427 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1427` | `CHECK_F( BIT_initDStream(&bitD1, istart1, length1) );` | propagates the callee error code unchanged |
| 428 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1428` | `CHECK_F( BIT_initDStream(&bitD2, istart2, length2) );` | propagates the callee error code unchanged |
| 429 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1429` | `CHECK_F( BIT_initDStream(&bitD3, istart3, length3) );` | propagates the callee error code unchanged |
| 430 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1430` | `CHECK_F( BIT_initDStream(&bitD4, istart4, length4) );` | propagates the callee error code unchanged |
| 431 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1483` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 432 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1484` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 433 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1485` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 434 | `HUF_readDTableX2_wksp` <br/>`src/decompress/huf_decompress.c:1496` | `if (!endCheck) return ERROR(corruption_detected); }` | `ZSTD_error_corruption_detected` |
| 435 | `HUF_decompress4X2_usingDTable_internal_fast_c_loop` <br/>`src/decompress/huf_decompress.c:1711` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 436 | `HUF_decompress1X2_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:1763` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 437 | `HUF_decompress4X2_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:1778` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 438 | `HUF_decompress1X_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:1850` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 439 | `HUF_decompress1X_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:1851` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 440 | `HUF_decompress1X1_DCtx_wksp` <br/>`src/decompress/huf_decompress.c:1900` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 441 | `HUF_decompress4X_hufOnly_wksp` <br/>`src/decompress/huf_decompress.c:1927` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 442 | `HUF_decompress4X_hufOnly_wksp` <br/>`src/decompress/huf_decompress.c:1928` | `if (cSrcSize == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 443 | `ZSTD_copyDDictParameters` <br/>`src/decompress/zstd_ddict.c:99` | `return ERROR(dictionary_corrupted);   /* only accept specified dictionaries */` | `ZSTD_error_dictionary_corrupted` |
| 444 | `ZSTD_copyDDictParameters` <br/>`src/decompress/zstd_ddict.c:105` | `return ERROR(dictionary_corrupted);   /* only accept specified dictionaries */` | `ZSTD_error_dictionary_corrupted` |
| 445 | `ZSTD_copyDDictParameters` <br/>`src/decompress/zstd_ddict.c:112` | `ZSTD_isError(ZSTD_loadDEntropy( &ddict->entropy` | `ZSTD_error_ddict` |
| 446 | `ZSTD_initDDict_internal` <br/>`src/decompress/zstd_ddict.c:133` | `if (!internalBuffer) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 447 | `ZSTD_createDDict_advanced` <br/>`src/decompress/zstd_ddict.c:150` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | returns `NULL` |
| 448 | `ZSTD_createDDict_advanced` <br/>`src/decompress/zstd_ddict.c:153` | `if (ddict == NULL) return NULL;` | returns `NULL` |
| 449 | `ZSTD_createDDict_advanced` <br/>`src/decompress/zstd_ddict.c:160` | `return NULL;` | returns `NULL` |
| 450 | `ZSTD_initStaticDDict` <br/>`src/decompress/zstd_ddict.c:198` | `if ((size_t)sBuffer & 7) return NULL;   /* 8-aligned */` | returns `NULL` |
| 451 | `ZSTD_initStaticDDict` <br/>`src/decompress/zstd_ddict.c:199` | `if (sBufferSize < neededSpace) return NULL;` | returns `NULL` |
| 452 | `ZSTD_initStaticDDict` <br/>`src/decompress/zstd_ddict.c:207` | `return NULL;` | returns `NULL` |
| 453 | `ZSTD_DDictHashSet_emplaceDDict` <br/>`src/decompress/zstd_decompress.c:109` | `hashSet->ddictPtrCount == hashSet->ddictPtrTableSize` | `ZSTD_error_GENERIC` |
| 454 | `ZSTD_DDictHashSet_expand` <br/>`src/decompress/zstd_decompress.c:139` | `!newTable` | `ZSTD_error_memory_allocation` |
| 455 | `ZSTD_createDDictHashSet` <br/>`src/decompress/zstd_decompress.c:182` | `return NULL;` | returns `NULL` |
| 456 | `ZSTD_createDDictHashSet` <br/>`src/decompress/zstd_decompress.c:186` | `return NULL;` | returns `NULL` |
| 457 | `ZSTD_initStaticDCtx` <br/>`src/decompress/zstd_decompress.c:285` | `if ((size_t)workspace & 7) return NULL;  /* 8-aligned */` | returns `NULL` |
| 458 | `ZSTD_initStaticDCtx` <br/>`src/decompress/zstd_decompress.c:286` | `if (workspaceSize < sizeof(ZSTD_DCtx)) return NULL;  /* minimum size */` | returns `NULL` |
| 459 | `ZSTD_createDCtx_internal` <br/>`src/decompress/zstd_decompress.c:295` | `if ((!customMem.customAlloc) ^ (!customMem.customFree)) return NULL;` | returns `NULL` |
| 460 | `ZSTD_createDCtx_internal` <br/>`src/decompress/zstd_decompress.c:298` | `if (!dctx) return NULL;` | returns `NULL` |
| 461 | `ZSTD_freeDCtx` <br/>`src/decompress/zstd_decompress.c:327` | `dctx->staticSize` | `ZSTD_error_memory_allocation` |
| 462 | `ZSTD_frameHeaderSize_internal` <br/>`src/decompress/zstd_decompress.c:419` | `srcSize < minInputSize` | `ZSTD_error_srcSize_wrong` |
| 463 | `ZSTD_getFrameHeader_advanced` <br/>`src/decompress/zstd_decompress.c:456` | `src==NULL` | `ZSTD_error_GENERIC` |
| 464 | `ZSTD_getFrameHeader_advanced` <br/>`src/decompress/zstd_decompress.c:473` | `RETURN_ERROR(prefix_unknown, "first bytes don't correspond to any supported magic number");` | `ZSTD_error_prefix_unknown` |
| 465 | `ZSTD_getFrameHeader_advanced` <br/>`src/decompress/zstd_decompress.c:493` | `RETURN_ERROR(prefix_unknown, "");` | `ZSTD_error_prefix_unknown` |
| 466 | `ZSTD_getFrameHeader_advanced` <br/>`src/decompress/zstd_decompress.c:511` | `(fhdByte & 0x08) != 0` | `ZSTD_error_frameParameter_unsupported` |
| 467 | `ZSTD_getFrameHeader_advanced` <br/>`src/decompress/zstd_decompress.c:517` | `windowLog > ZSTD_WINDOWLOG_MAX` | `ZSTD_error_frameParameter_windowTooLarge` |
| 468 | `readSkippableFrameSize` <br/>`src/decompress/zstd_decompress.c:592` | `srcSize < ZSTD_SKIPPABLEHEADERSIZE` | `ZSTD_error_srcSize_wrong` |
| 469 | `readSkippableFrameSize` <br/>`src/decompress/zstd_decompress.c:595` | `(U32)(sizeU32 + ZSTD_SKIPPABLEHEADERSIZE) < sizeU32` | `ZSTD_error_frameParameter_unsupported` |
| 470 | `readSkippableFrameSize` <br/>`src/decompress/zstd_decompress.c:598` | `skippableSize > srcSize` | `ZSTD_error_srcSize_wrong` |
| 471 | `ZSTD_readSkippableFrame` <br/>`src/decompress/zstd_decompress.c:618` | `srcSize < ZSTD_SKIPPABLEHEADERSIZE` | `ZSTD_error_srcSize_wrong` |
| 472 | `ZSTD_readSkippableFrame` <br/>`src/decompress/zstd_decompress.c:625` | `!ZSTD_isSkippableFrame(src` | `ZSTD_error_srcSize` |
| 473 | `ZSTD_readSkippableFrame` <br/>`src/decompress/zstd_decompress.c:626` | `skippableFrameSize < ZSTD_SKIPPABLEHEADERSIZE \|\| skippableFrameSize > srcSize` | `ZSTD_error_srcSize_wrong` |
| 474 | `ZSTD_readSkippableFrame` <br/>`src/decompress/zstd_decompress.c:627` | `skippableContentSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` |
| 475 | `ZSTD_decodeFrameHeader` <br/>`src/decompress/zstd_decompress.c:706` | `result>0` | `ZSTD_error_srcSize_wrong` |
| 476 | `ZSTD_decodeFrameHeader` <br/>`src/decompress/zstd_decompress.c:717` | `dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)` | `ZSTD_error_dictionary_wrong` |
| 477 | `ZSTD_decompressionMargin` <br/>`src/decompress/zstd_decompress.c:852` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 478 | `ZSTD_copyRawBlock` <br/>`src/decompress/zstd_decompress.c:900` | `srcSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` |
| 479 | `ZSTD_copyRawBlock` <br/>`src/decompress/zstd_decompress.c:903` | `RETURN_ERROR(dstBuffer_null, "");` | `ZSTD_error_dstBuffer_null` |
| 480 | `ZSTD_setRleBlock` <br/>`src/decompress/zstd_decompress.c:913` | `regenSize > dstCapacity` | `ZSTD_error_dstSize_tooSmall` |
| 481 | `ZSTD_setRleBlock` <br/>`src/decompress/zstd_decompress.c:916` | `RETURN_ERROR(dstBuffer_null, "");` | `ZSTD_error_dstBuffer_null` |
| 482 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:967` | `remainingSrcSize < ZSTD_FRAMEHEADERSIZE_MIN(dctx->format)+ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` |
| 483 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:975` | `remainingSrcSize < frameHeaderSize+ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` |
| 484 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:995` | `cBlockSize > remainingSrcSize` | `ZSTD_error_srcSize_wrong` |
| 485 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:1029` | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ZSTD_error_corruption_detected` |
| 486 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:1046` | `(U64)(op-ostart) != dctx->fParams.frameContentSize` | `ZSTD_error_corruption_detected` |
| 487 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:1050` | `remainingSrcSize<4` | `ZSTD_error_checksum_wrong` |
| 488 | `ZSTD_decompressFrame` <br/>`src/decompress/zstd_decompress.c:1055` | `checkRead != checkCalc` | `ZSTD_error_checksum_wrong` |
| 489 | `ZSTD_decompressMultiFrame` <br/>`src/decompress/zstd_decompress.c:1094` | `dctx->staticSize` | `ZSTD_error_memory_allocation` |
| 490 | `ZSTD_decompressMultiFrame` <br/>`src/decompress/zstd_decompress.c:1102` | `expectedSize == ZSTD_CONTENTSIZE_ERROR` | `ZSTD_error_corruption_detected` |
| 491 | `ZSTD_decompressMultiFrame` <br/>`src/decompress/zstd_decompress.c:1104` | `expectedSize != decodedSize` | `ZSTD_error_corruption_detected` |
| 492 | `ZSTD_decompressMultiFrame` <br/>`src/decompress/zstd_decompress.c:1146` | `(ZSTD_getErrorCode(res) == ZSTD_error_prefix_unknown) && (moreThan1Frame==1)` | `ZSTD_error_srcSize_wrong` |
| 493 | `ZSTD_decompressMultiFrame` <br/>`src/decompress/zstd_decompress.c:1166` | `srcSize` | `ZSTD_error_srcSize_wrong` |
| 494 | `ZSTD_getDDict` <br/>`src/decompress/zstd_decompress.c:1188` | `return NULL;` | returns `NULL` |
| 495 | `ZSTD_decompress` <br/>`src/decompress/zstd_decompress.c:1208` | `dctx==NULL` | `ZSTD_error_memory_allocation` |
| 496 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1279` | `srcSize != ZSTD_nextSrcSizeToDecompressWithInputSize(dctx` | `ZSTD_error_srcSize` |
| 497 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1315` | `cBlockSize > dctx->fParams.blockSizeMax` | `ZSTD_error_corruption_detected` |
| 498 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1364` | `RETURN_ERROR(corruption_detected, "invalid block type");` | `ZSTD_error_corruption_detected` |
| 499 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1367` | `rSize > dctx->fParams.blockSizeMax` | `ZSTD_error_corruption_detected` |
| 500 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1380` | `dctx->fParams.frameContentSize != ZSTD_CONTENTSIZE_UNKNOWN && dctx->decodedSize != dctx->fParams.frameContentSize` | `ZSTD_error_corruption_detected` |
| 501 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1406` | `check32 != h32` | `ZSTD_error_checksum_wrong` |
| 502 | `ZSTD_decompressContinue` <br/>`src/decompress/zstd_decompress.c:1430` | `RETURN_ERROR(GENERIC, "impossible to reach");   /* some compilers require default to do something */` | `ZSTD_error_GENERIC` |
| 503 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1458` | `dictSize <= 8` | `ZSTD_error_dictionary_corrupted` |
| 504 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1477` | `HUF_isError(hSize)` | `ZSTD_error_dictionary_corrupted` |
| 505 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1484` | `FSE_isError(offcodeHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 506 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1485` | `offcodeMaxValue > MaxOff` | `ZSTD_error_dictionary_corrupted` |
| 507 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1486` | `offcodeLog > OffFSELog` | `ZSTD_error_dictionary_corrupted` |
| 508 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1499` | `FSE_isError(matchlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 509 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1500` | `matchlengthMaxValue > MaxML` | `ZSTD_error_dictionary_corrupted` |
| 510 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1501` | `matchlengthLog > MLFSELog` | `ZSTD_error_dictionary_corrupted` |
| 511 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1514` | `FSE_isError(litlengthHeaderSize)` | `ZSTD_error_dictionary_corrupted` |
| 512 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1515` | `litlengthMaxValue > MaxLL` | `ZSTD_error_dictionary_corrupted` |
| 513 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1516` | `litlengthLog > LLFSELog` | `ZSTD_error_dictionary_corrupted` |
| 514 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1526` | `dictPtr+12 > dictEnd` | `ZSTD_error_dictionary_corrupted` |
| 515 | `ZSTD_refDictContent` <br/>`src/decompress/zstd_decompress.c:1531` | `rep==0 \|\| rep > dictContentSize` | `ZSTD_error_dictionary_corrupted` |
| 516 | `ZSTD_decompress_insertDictionary` <br/>`src/decompress/zstd_decompress.c:1550` | `ZSTD_isError(eSize)` | `ZSTD_error_dictionary_corrupted` |
| 517 | `ZSTD_decompressBegin_usingDict` <br/>`src/decompress/zstd_decompress.c:1592` | `ZSTD_isError(ZSTD_decompress_insertDictionary(dctx` | `ZSTD_error_dict` |
| 518 | `ZSTD_DCtx_loadDictionary_advanced` <br/>`src/decompress/zstd_decompress.c:1704` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` |
| 519 | `ZSTD_DCtx_loadDictionary_advanced` <br/>`src/decompress/zstd_decompress.c:1708` | `dctx->ddictLocal == NULL` | `ZSTD_error_memory_allocation` |
| 520 | `ZSTD_DCtx_refDDict` <br/>`src/decompress/zstd_decompress.c:1782` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` |
| 521 | `ZSTD_DCtx_refDDict` <br/>`src/decompress/zstd_decompress.c:1791` | `RETURN_ERROR(memory_allocation, "Failed to allocate memory for hash set!");` | `ZSTD_error_memory_allocation` |
| 522 | `ZSTD_DCtx_setMaxWindowSize` <br/>`src/decompress/zstd_decompress.c:1809` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` |
| 523 | `ZSTD_DCtx_setMaxWindowSize` <br/>`src/decompress/zstd_decompress.c:1810` | `maxWindowSize < min` | `ZSTD_error_parameter_outOfBound` |
| 524 | `ZSTD_DCtx_setMaxWindowSize` <br/>`src/decompress/zstd_decompress.c:1811` | `maxWindowSize > max` | `ZSTD_error_parameter_outOfBound` |
| 525 | `ZSTD_dParam_getBounds` <br/>`src/decompress/zstd_decompress.c:1857` | `bounds.error = ERROR(parameter_unsupported);` | `ZSTD_error_parameter_unsupported` |
| 526 | `ZSTD_dParam_withinBounds` <br/>`src/decompress/zstd_decompress.c:1874` | `!ZSTD_dParam_withinBounds(p` | `ZSTD_error_v` |
| 527 | `ZSTD_DCtx_getParameter` <br/>`src/decompress/zstd_decompress.c:1903` | `RETURN_ERROR(parameter_unsupported, "");` | `ZSTD_error_parameter_unsupported` |
| 528 | `ZSTD_DCtx_setParameter` <br/>`src/decompress/zstd_decompress.c:1908` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` |
| 529 | `ZSTD_DCtx_setParameter` <br/>`src/decompress/zstd_decompress.c:1930` | `RETURN_ERROR(parameter_unsupported, "Static dctx does not support multiple DDicts!");` | `ZSTD_error_parameter_unsupported` |
| 530 | `ZSTD_DCtx_setParameter` <br/>`src/decompress/zstd_decompress.c:1944` | `RETURN_ERROR(parameter_unsupported, "");` | `ZSTD_error_parameter_unsupported` |
| 531 | `ZSTD_DCtx_reset` <br/>`src/decompress/zstd_decompress.c:1957` | `dctx->streamStage != zdss_init` | `ZSTD_error_stage_wrong` |
| 532 | `ZSTD_decodingBufferSize_internal` <br/>`src/decompress/zstd_decompress.c:1983` | `(unsigned long long)minRBSize != neededSize` | `ZSTD_error_frameParameter_windowTooLarge` |
| 533 | `ZSTD_estimateDStreamSize_fromFrame` <br/>`src/decompress/zstd_decompress.c:2007` | `err>0` | `ZSTD_error_srcSize_wrong` |
| 534 | `ZSTD_estimateDStreamSize_fromFrame` <br/>`src/decompress/zstd_decompress.c:2008` | `zfh.windowSize > windowSizeMax` | `ZSTD_error_frameParameter_windowTooLarge` |
| 535 | `ZSTD_checkOutBuffer` <br/>`src/decompress/zstd_decompress.c:2049` | `RETURN_ERROR(dstBuffer_wrong, "ZSTD_d_stableOutBuffer enabled but output differs!");` | `ZSTD_error_dstBuffer_wrong` |
| 536 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2100` | `input->pos > input->size` | `ZSTD_error_srcSize_wrong` |
| 537 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2105` | `output->pos > output->size` | `ZSTD_error_dstSize_tooSmall` |
| 538 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2131` | `zds->staticSize` | `ZSTD_error_memory_allocation` |
| 539 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2150` | `zds->staticSize` | `ZSTD_error_memory_allocation` |
| 540 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2209` | `RETURN_ERROR(dstSize_tooSmall, "ZSTD_obm_stable passed but ZSTD_outBuffer is too small");` | `ZSTD_error_dstSize_tooSmall` |
| 541 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2231` | `zds->fParams.windowSize > zds->maxWindowSize` | `ZSTD_error_frameParameter_windowTooLarge` |
| 542 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2256` | `bufferSize > zds->staticSize - sizeof(ZSTD_DCtx)` | `ZSTD_error_memory_allocation` |
| 543 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2264` | `zds->inBuff == NULL` | `ZSTD_error_memory_allocation` |
| 544 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2303` | `toLoad > zds->inBuffSize - zds->inPos` | `ZSTD_error_corruption_detected` |
| 545 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2346` | `RETURN_ERROR(GENERIC, "impossible to reach");   /* some compilers require default to do something */` | `ZSTD_error_GENERIC` |
| 546 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2359` | `op==oend` | `ZSTD_error_noForwardProgress_destFull` |
| 547 | `ZSTD_decompressStream` <br/>`src/decompress/zstd_decompress.c:2360` | `ip==iend` | `ZSTD_error_noForwardProgress_inputEmpty` |
| 548 | `ZSTD_getcBlockSize` <br/>`src/decompress/zstd_decompress_block.c:66` | `srcSize < ZSTD_blockHeaderSize` | `ZSTD_error_srcSize_wrong` |
| 549 | `ZSTD_getcBlockSize` <br/>`src/decompress/zstd_decompress_block.c:74` | `bpPtr->blockType == bt_reserved` | `ZSTD_error_corruption_detected` |
| 550 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:139` | `srcSize < MIN_CBLOCK_SIZE` | `ZSTD_error_corruption_detected` |
| 551 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:149` | `dctx->litEntropy==0` | `ZSTD_error_dictionary_corrupted` |
| 552 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:153` | `srcSize < 5` | `ZSTD_error_corruption_detected` |
| 553 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:185` | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` |
| 554 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:186` | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` |
| 555 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:188` | `litSize < MIN_LITERALS_FOR_4_STREAMS` | `ZSTD_error_literals_headerWrong` |
| 556 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:191` | `litCSize + lhSize > srcSize` | `ZSTD_error_corruption_detected` |
| 557 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:192` | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` |
| 558 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:241` | `HUF_isError(hufSuccess)` | `ZSTD_error_corruption_detected` |
| 559 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:266` | `srcSize<3` | `ZSTD_error_corruption_detected` |
| 560 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:271` | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` |
| 561 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:272` | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` |
| 562 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:273` | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` |
| 563 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:276` | `litSize+lhSize > srcSize` | `ZSTD_error_corruption_detected` |
| 564 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:310` | `srcSize<3` | `ZSTD_error_corruption_detected` |
| 565 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:315` | `srcSize<4` | `ZSTD_error_corruption_detected` |
| 566 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:319` | `litSize > 0 && dst == NULL` | `ZSTD_error_dstSize_tooSmall` |
| 567 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:320` | `litSize > blockSizeMax` | `ZSTD_error_corruption_detected` |
| 568 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:321` | `expectedWriteSize < litSize` | `ZSTD_error_dstSize_tooSmall` |
| 569 | `ZSTD_decodeLiteralsBlock` <br/>`src/decompress/zstd_decompress_block.c:337` | `RETURN_ERROR(corruption_detected, "impossible");` | `ZSTD_error_corruption_detected` |
| 570 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:658` | `!srcSize` | `ZSTD_error_srcSize_wrong` |
| 571 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:659` | `(*(const BYTE*)src) > max` | `ZSTD_error_corruption_detected` |
| 572 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:671` | `!flagRepeatTable` | `ZSTD_error_corruption_detected` |
| 573 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:683` | `FSE_isError(headerSize)` | `ZSTD_error_corruption_detected` |
| 574 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:684` | `tableLog > maxLog` | `ZSTD_error_corruption_detected` |
| 575 | `ZSTD_buildSeqTable` <br/>`src/decompress/zstd_decompress_block.c:691` | `RETURN_ERROR(GENERIC, "impossible");` | `ZSTD_error_GENERIC` |
| 576 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:705` | `srcSize < MIN_SEQUENCES_SIZE` | `ZSTD_error_srcSize_wrong` |
| 577 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:711` | `ip+2 > iend` | `ZSTD_error_srcSize_wrong` |
| 578 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:715` | `ip >= iend` | `ZSTD_error_srcSize_wrong` |
| 579 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:723` | `ip != iend` | `ZSTD_error_corruption_detected` |
| 580 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:729` | `ip+1 > iend` | `ZSTD_error_srcSize_wrong` |
| 581 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:730` | `*ip & 3` | `ZSTD_error_corruption_detected` |
| 582 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:745` | `ZSTD_isError(llhSize)` | `ZSTD_error_corruption_detected` |
| 583 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:757` | `ZSTD_isError(ofhSize)` | `ZSTD_error_corruption_detected` |
| 584 | `ZSTD_decodeSeqHeaders` <br/>`src/decompress/zstd_decompress_block.c:769` | `ZSTD_isError(mlhSize)` | `ZSTD_error_corruption_detected` |
| 585 | `ZSTD_execSequenceEnd` <br/>`src/decompress/zstd_decompress_block.c:919` | `sequenceLength > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 586 | `ZSTD_execSequenceEnd` <br/>`src/decompress/zstd_decompress_block.c:920` | `sequence.litLength > (size_t)(litLimit - *litPtr)` | `ZSTD_error_corruption_detected` |
| 587 | `ZSTD_execSequenceEnd` <br/>`src/decompress/zstd_decompress_block.c:932` | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` |
| 588 | `ZSTD_execSequenceEndSplitLitBuffer` <br/>`src/decompress/zstd_decompress_block.c:967` | `sequenceLength > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 589 | `ZSTD_execSequenceEndSplitLitBuffer` <br/>`src/decompress/zstd_decompress_block.c:968` | `sequence.litLength > (size_t)(litLimit - *litPtr)` | `ZSTD_error_corruption_detected` |
| 590 | `ZSTD_execSequenceEndSplitLitBuffer` <br/>`src/decompress/zstd_decompress_block.c:973` | `op > *litPtr && op < *litPtr + sequence.litLength` | `ZSTD_error_dstSize_tooSmall` |
| 591 | `ZSTD_execSequenceEndSplitLitBuffer` <br/>`src/decompress/zstd_decompress_block.c:981` | `sequence.offset > (size_t)(oLitEnd - virtualStart)` | `ZSTD_error_corruption_detected` |
| 592 | `ZSTD_execSequence` <br/>`src/decompress/zstd_decompress_block.c:1054` | `UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart))` | `ZSTD_error_corruption_detected` |
| 593 | `ZSTD_execSequenceSplitLitBuffer` <br/>`src/decompress/zstd_decompress_block.c:1147` | `UNLIKELY(sequence.offset > (size_t)(oLitEnd - virtualStart))` | `ZSTD_error_corruption_detected` |
| 594 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1425` | `ERR_isError(BIT_initDStream(&seqState.DStream` | `ZSTD_error_ip` |
| 595 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1521` | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 596 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1579` | `nbSeq` | `ZSTD_error_corruption_detected` |
| 597 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1581` | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` |
| 598 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1591` | `lastLLSize > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 599 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1603` | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` |
| 600 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1637` | `ERR_isError(BIT_initDStream(&seqState.DStream` | `ZSTD_error_ip` |
| 601 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1674` | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` |
| 602 | `ZSTD_assertValidSequence` <br/>`src/decompress/zstd_decompress_block.c:1682` | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` |
| 603 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1765` | `ERR_isError(BIT_initDStream(&seqState.DStream` | `ZSTD_error_ip` |
| 604 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1788` | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 605 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1824` | `!BIT_endOfDStream(&seqState.DStream)` | `ZSTD_error_corruption_detected` |
| 606 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1833` | `leftoverLit > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 607 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1871` | `lastLLSize > (size_t)(oend - op)` | `ZSTD_error_dstSize_tooSmall` |
| 608 | `ZSTD_prefetchMatch` <br/>`src/decompress/zstd_decompress_block.c:1880` | `lastLLSize > (size_t)(oend-op)` | `ZSTD_error_dstSize_tooSmall` |
| 609 | `ZSTD_maxShortOffset` <br/>`src/decompress/zstd_decompress_block.c:2081` | `srcSize > ZSTD_blockSizeMax(dctx)` | `ZSTD_error_srcSize_wrong` |
| 610 | `ZSTD_maxShortOffset` <br/>`src/decompress/zstd_decompress_block.c:2129` | `(dst == NULL \|\| dstCapacity == 0) && nbSeq > 0` | `ZSTD_error_dstSize_tooSmall` |
| 611 | `ZSTD_maxShortOffset` <br/>`src/decompress/zstd_decompress_block.c:2130` | `MEM_64bits() && sizeof(size_t) == sizeof(void*) && (size_t)(-1) - (size_t)dst < (size_t)(1 << 20)` | `ZSTD_error_dstSize_tooSmall` |
| 612 | `COVER_cmp8` <br/>`src/dictBuilder/cover.c:283` | `return -1;` | returns `-1` |
| 613 | `COVER_ctx_init` <br/>`src/dictBuilder/cover.c:618` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 614 | `COVER_ctx_init` <br/>`src/dictBuilder/cover.c:623` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 615 | `COVER_ctx_init` <br/>`src/dictBuilder/cover.c:628` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 616 | `COVER_ctx_init` <br/>`src/dictBuilder/cover.c:651` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 617 | `ZDICT_trainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:793` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 618 | `ZDICT_trainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:797` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 619 | `ZDICT_trainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:802` | `return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 620 | `ZDICT_trainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:816` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 621 | `COVER_checkTotalCompressedSize` <br/>`src/dictBuilder/cover.c:844` | `size_t totalCompressedSize = ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 622 | `COVER_best_finish` <br/>`src/dictBuilder/cover.c:977` | `best->compressedSize = ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 623 | `COVER_tryParameters` <br/>`src/dictBuilder/cover.c:1129` | `size_t totalCompressedSize = ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 624 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1197` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 625 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1201` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 626 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1205` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 627 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1210` | `return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 628 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1215` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 629 | `ZDICT_optimizeTrainFromBuffer_cover` <br/>`src/dictBuilder/cover.c:1253` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 630 | `<file scope>` <br/>`src/dictBuilder/divsufsort.c:1853` | `if((T == NULL) \|\| (SA == NULL) \|\| (n < 0)) { return -1; }` | returns `-1` |
| 631 | `<file scope>` <br/>`src/dictBuilder/divsufsort.c:1882` | `if((T == NULL) \|\| (U == NULL) \|\| (n < 0)) { return -1; }` | returns `-1` |
| 632 | `FASTCOVER_checkParameters` <br/>`src/dictBuilder/fastcover.c:332` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 633 | `FASTCOVER_checkParameters` <br/>`src/dictBuilder/fastcover.c:338` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 634 | `FASTCOVER_checkParameters` <br/>`src/dictBuilder/fastcover.c:344` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 635 | `FASTCOVER_checkParameters` <br/>`src/dictBuilder/fastcover.c:369` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 636 | `FASTCOVER_checkParameters` <br/>`src/dictBuilder/fastcover.c:386` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 637 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:480` | `size_t totalCompressedSize = ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 638 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:571` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 639 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:575` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 640 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:580` | `return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 641 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:652` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 642 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:656` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 643 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:660` | `return ERROR(parameter_outOfBound);` | `ZSTD_error_parameter_outOfBound` |
| 644 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:664` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 645 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:669` | `return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 646 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:674` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 647 | `FASTCOVER_tryParameters` <br/>`src/dictBuilder/fastcover.c:715` | `return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 648 | `ZDICT_getDictHeaderSize` <br/>`src/dictBuilder/zdict.c:112` | `if (dictSize <= 8 \|\| MEM_readLE32(dictBuffer) != ZSTD_MAGIC_DICTIONARY) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 649 | `ZDICT_getDictHeaderSize` <br/>`src/dictBuilder/zdict.c:117` | `headerSize = ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 650 | `ZDICT_trainBuffer_legacy` <br/>`src/dictBuilder/zdict.c:494` | `result = ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 651 | `ZDICT_trainBuffer_legacy` <br/>`src/dictBuilder/zdict.c:507` | `if (divSuftSortResult != 0) { result = ERROR(GENERIC); goto _cleanup; }` | `ZSTD_error_GENERIC` |
| 652 | `ZDICT_analyzeEntropy` <br/>`src/dictBuilder/zdict.c:688` | `if (offcodeMax>OFFCODE_MAX) { eSize = ERROR(dictionaryCreation_failed); goto _cleanup; }   /* too large dictionary */` | `ZSTD_error_dictionaryCreation_failed` |
| 653 | `ZDICT_analyzeEntropy` <br/>`src/dictBuilder/zdict.c:703` | `eSize = ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 654 | `ZDICT_analyzeEntropy` <br/>`src/dictBuilder/zdict.c:820` | `eSize = ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 655 | `ZDICT_finalizeDictionary` <br/>`src/dictBuilder/zdict.c:874` | `if (dictBufferCapacity < dictContentSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 656 | `ZDICT_finalizeDictionary` <br/>`src/dictBuilder/zdict.c:875` | `if (dictBufferCapacity < ZDICT_DICTSIZE_MIN) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 657 | `ZDICT_finalizeDictionary` <br/>`src/dictBuilder/zdict.c:905` | `hSize + minContentSize > dictBufferCapacity` | `ZSTD_error_dstSize_tooSmall` |
| 658 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:993` | `if (!dictList) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 659 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:994` | `if (maxDictSize < ZDICT_DICTSIZE_MIN) { free(dictList); return ERROR(dstSize_tooSmall); }   /* requested dictionary size is too small */` | `ZSTD_error_dstSize_tooSmall` |
| 660 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:995` | `if (samplesBuffSize < ZDICT_MIN_SAMPLES_SIZE) { free(dictList); return ERROR(dictionaryCreation_failed); }   /* not enough source to crea...` | `ZSTD_error_dictionaryCreation_failed` |
| 661 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:1019` | `return ERROR(GENERIC);   /* should never happen */` | `ZSTD_error_GENERIC` |
| 662 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:1030` | `if (dictContentSize < ZDICT_CONTENTSIZE_MIN) { free(dictList); return ERROR(dictionaryCreation_failed); }   /* dictionary content too sma...` | `ZSTD_error_dictionaryCreation_failed` |
| 663 | `ZDICT_trainFromBuffer_unsafe_legacy` <br/>`src/dictBuilder/zdict.c:1066` | `if (ptr<(BYTE*)dictBuffer) { free(dictList); return ERROR(GENERIC); }   /* should not happen */` | `ZSTD_error_GENERIC` |
| 664 | `ZDICT_trainFromBuffer_legacy` <br/>`src/dictBuilder/zdict.c:1094` | `if (!newBuff) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |

### Part 3 — full mechanical enumeration (legacy decoders v01–v07)

728 rejection sites in `src/legacy`.

| # | function / site | trigger (exact condition the C tests) | expected C result |
|---|-----------------|----------------------------------------|-------------------|
| 665 | `ZSTDv01_getcBlockSize` <br/>`src/legacy/zstd_v01.c:1431` | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 666 | `ZSTD_copyUncompressedBlock` <br/>`src/legacy/zstd_v01.c:1447` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 667 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v01.c:1466` | `if (srcSize <= 3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 668 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v01.c:1473` | `if (litSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 669 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v01.c:1475` | `if (FSE_isError(errorCode)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 670 | `ZSTDv01_decodeLiteralsBlock` <br/>`src/legacy/zstd_v01.c:1493` | `if (litcSize > srcSize - ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 671 | `ZSTDv01_decodeLiteralsBlock` <br/>`src/legacy/zstd_v01.c:1506` | `if (rleSize>maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 672 | `ZSTDv01_decodeLiteralsBlock` <br/>`src/legacy/zstd_v01.c:1507` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 673 | `ZSTDv01_decodeLiteralsBlock` <br/>`src/legacy/zstd_v01.c:1527` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 674 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1546` | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 675 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1570` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 676 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1589` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 677 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1590` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 678 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1599` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 679 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1607` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 680 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1608` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 681 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1617` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 682 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1625` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 683 | `ZSTDv01_decodeSeqHeaders` <br/>`src/legacy/zstd_v01.c:1626` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 684 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1732` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 685 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1733` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 686 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1735` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 687 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1737` | `if (endMatch > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 688 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1738` | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 689 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1739` | `if (sequence.matchLength > (size_t)(*litPtr-op)) return ERROR(dstSize_tooSmall);  /* overwrite literal segment */` | `ZSTD_error_dstSize_tooSmall` |
| 690 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1748` | `if (oend-op < 8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 691 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1758` | `if (match < base) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 692 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v01.c:1759` | `if (sequence.offset > (size_t)base) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 693 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v01.c:1853` | `if (FSE_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 694 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v01.c:1869` | `if ( !FSE_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 695 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v01.c:1870` | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 696 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v01.c:1875` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 697 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1921` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 698 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1923` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 699 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1934` | `if (blockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 700 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1945` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 701 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1949` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 702 | `ZSTDv01_decompressDCtx` <br/>`src/legacy/zstd_v01.c:1952` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 703 | `ZSTDv01_createDCtx` <br/>`src/legacy/zstd_v01.c:2043` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 704 | `ZSTDv01_decompressContinue` <br/>`src/legacy/zstd_v01.c:2064` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 705 | `ZSTDv01_decompressContinue` <br/>`src/legacy/zstd_v01.c:2073` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 706 | `ZSTDv01_decompressContinue` <br/>`src/legacy/zstd_v01.c:2112` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 707 | `ZSTDv01_decompressContinue` <br/>`src/legacy/zstd_v01.c:2118` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 708 | `BIT_initDStream` <br/>`src/legacy/zstd_v02.c:325` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 709 | `BIT_initDStream` <br/>`src/legacy/zstd_v02.c:334` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 710 | `BIT_initDStream` <br/>`src/legacy/zstd_v02.c:360` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 711 | `FSE_tableStep` <br/>`src/legacy/zstd_v02.c:1051` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 712 | `FSE_tableStep` <br/>`src/legacy/zstd_v02.c:1052` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 713 | `FSE_tableStep` <br/>`src/legacy/zstd_v02.c:1082` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 714 | `FSE_readNCount` <br/>`src/legacy/zstd_v02.c:1131` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 715 | `FSE_readNCount` <br/>`src/legacy/zstd_v02.c:1134` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 716 | `FSE_readNCount` <br/>`src/legacy/zstd_v02.c:1169` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 717 | `FSE_readNCount` <br/>`src/legacy/zstd_v02.c:1221` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 718 | `FSE_readNCount` <br/>`src/legacy/zstd_v02.c:1225` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 719 | `FSE_buildDTable_raw` <br/>`src/legacy/zstd_v02.c:1261` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 720 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v02.c:1340` | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ZSTD_error_dstSize_tooSmall` |
| 721 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v02.c:1342` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 722 | `FSE_decompress` <br/>`src/legacy/zstd_v02.c:1369` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 723 | `FSE_decompress` <br/>`src/legacy/zstd_v02.c:1374` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 724 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1492` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 725 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1509` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 726 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1510` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 727 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1521` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 728 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1531` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 729 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1535` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 730 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1539` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 731 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1545` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 732 | `HUF_readStats` <br/>`src/legacy/zstd_v02.c:1551` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 733 | `HUF_readDTableX2` <br/>`src/legacy/zstd_v02.c:1584` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 734 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1661` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 735 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1697` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 736 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1732` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 737 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1733` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 738 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1734` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 739 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v02.c:1745` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 740 | `HUF_decompress4X2` <br/>`src/legacy/zstd_v02.c:1761` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 741 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v02.c:1882` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 742 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v02.c:1889` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 743 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v02.c:1893` | `{if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ZSTD_error_GENERIC` |
| 744 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2023` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 745 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2059` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 746 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2094` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 747 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2095` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 748 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2096` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 749 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v02.c:2107` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 750 | `HUF_decompress4X4` <br/>`src/legacy/zstd_v02.c:2122` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 751 | `HUF_readDTableX6` <br/>`src/legacy/zstd_v02.c:2215` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 752 | `HUF_readDTableX6` <br/>`src/legacy/zstd_v02.c:2222` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 753 | `HUF_readDTableX6` <br/>`src/legacy/zstd_v02.c:2226` | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ZSTD_error_GENERIC` |
| 754 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2378` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 755 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2416` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 756 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2451` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 757 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2452` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 758 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2453` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 759 | `HUF_decompress4X6_usingDTable` <br/>`src/legacy/zstd_v02.c:2464` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 760 | `HUF_decompress4X6` <br/>`src/legacy/zstd_v02.c:2479` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 761 | `HUF_decompress` <br/>`src/legacy/zstd_v02.c:2526` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 762 | `HUF_decompress` <br/>`src/legacy/zstd_v02.c:2527` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 763 | `ZSTD_getcBlockSize` <br/>`src/legacy/zstd_v02.c:2762` | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 764 | `ZSTD_copyUncompressedBlock` <br/>`src/legacy/zstd_v02.c:2777` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 765 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v02.c:2795` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 766 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v02.c:2796` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 767 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v02.c:2798` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 768 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v02.c:2814` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 769 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v02.c:2833` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 770 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v02.c:2834` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 771 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v02.c:2849` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 772 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2871` | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 773 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2895` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 774 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2914` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 775 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2915` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 776 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2924` | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 777 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2933` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 778 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2934` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 779 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2943` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 780 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2951` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 781 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v02.c:2952` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 782 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3058` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 783 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3059` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 784 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3061` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 785 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3062` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 786 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3064` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 787 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3065` | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 788 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3077` | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected);   /* address space overflow test (this test seems kept by clang opti...` | `ZSTD_error_corruption_detected` |
| 789 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v02.c:3079` | `if (match < base) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 790 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v02.c:3156` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 791 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v02.c:3172` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 792 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v02.c:3173` | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 793 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v02.c:3178` | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 794 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v02.c:3179` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 795 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3221` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 796 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3223` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 797 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3235` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 798 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3246` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 799 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3250` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 800 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v02.c:3253` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 801 | `ZSTD_createDCtx` <br/>`src/legacy/zstd_v02.c:3344` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 802 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v02.c:3363` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 803 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v02.c:3372` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 804 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v02.c:3411` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 805 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v02.c:3417` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 806 | `BIT_initDStream` <br/>`src/legacy/zstd_v03.c:327` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 807 | `BIT_initDStream` <br/>`src/legacy/zstd_v03.c:336` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 808 | `BIT_initDStream` <br/>`src/legacy/zstd_v03.c:362` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 809 | `FSE_tableStep` <br/>`src/legacy/zstd_v03.c:1051` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 810 | `FSE_tableStep` <br/>`src/legacy/zstd_v03.c:1052` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 811 | `FSE_tableStep` <br/>`src/legacy/zstd_v03.c:1082` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 812 | `FSE_readNCount` <br/>`src/legacy/zstd_v03.c:1131` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 813 | `FSE_readNCount` <br/>`src/legacy/zstd_v03.c:1134` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 814 | `FSE_readNCount` <br/>`src/legacy/zstd_v03.c:1169` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 815 | `FSE_readNCount` <br/>`src/legacy/zstd_v03.c:1221` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 816 | `FSE_readNCount` <br/>`src/legacy/zstd_v03.c:1225` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 817 | `FSE_buildDTable_raw` <br/>`src/legacy/zstd_v03.c:1261` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 818 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v03.c:1340` | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ZSTD_error_dstSize_tooSmall` |
| 819 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v03.c:1342` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 820 | `FSE_decompress` <br/>`src/legacy/zstd_v03.c:1369` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 821 | `FSE_decompress` <br/>`src/legacy/zstd_v03.c:1374` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 822 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1488` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 823 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1505` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 824 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1506` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 825 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1517` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 826 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1527` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 827 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1531` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 828 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1535` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 829 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1541` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 830 | `HUF_readStats` <br/>`src/legacy/zstd_v03.c:1547` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 831 | `HUF_readDTableX2` <br/>`src/legacy/zstd_v03.c:1580` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 832 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1657` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 833 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1693` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 834 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1728` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 835 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1729` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 836 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1730` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 837 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v03.c:1741` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 838 | `HUF_decompress4X2` <br/>`src/legacy/zstd_v03.c:1757` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 839 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v03.c:1878` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 840 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v03.c:1885` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 841 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v03.c:1889` | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ZSTD_error_GENERIC` |
| 842 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2019` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 843 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2055` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 844 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2090` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 845 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2091` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 846 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2092` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 847 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v03.c:2103` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 848 | `HUF_decompress4X4` <br/>`src/legacy/zstd_v03.c:2118` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 849 | `HUF_decompress` <br/>`src/legacy/zstd_v03.c:2165` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 850 | `HUF_decompress` <br/>`src/legacy/zstd_v03.c:2166` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 851 | `ZSTD_getcBlockSize` <br/>`src/legacy/zstd_v03.c:2402` | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 852 | `ZSTD_copyUncompressedBlock` <br/>`src/legacy/zstd_v03.c:2417` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 853 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v03.c:2435` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 854 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v03.c:2436` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 855 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v03.c:2438` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 856 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v03.c:2454` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 857 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v03.c:2473` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 858 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v03.c:2474` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 859 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v03.c:2489` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 860 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2511` | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 861 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2535` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 862 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2554` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 863 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2555` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 864 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2564` | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 865 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2573` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 866 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2574` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 867 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2583` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 868 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2591` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 869 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v03.c:2592` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 870 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2698` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 871 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2699` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 872 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2701` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 873 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2702` | `if (sequence.offset > (U32)(oLitEnd - base)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 874 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2704` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 875 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2705` | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 876 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2716` | `if (sequence.offset > (size_t)op) return ERROR(corruption_detected);   /* address space overflow test (this test seems kept by clang opti...` | `ZSTD_error_corruption_detected` |
| 877 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v03.c:2718` | `if (match < base) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 878 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v03.c:2795` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 879 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v03.c:2811` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* requested too much : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 880 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v03.c:2812` | `if (nbSeq<0) return ERROR(corruption_detected);   /* requested too many sequences : data is corrupted */` | `ZSTD_error_corruption_detected` |
| 881 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v03.c:2817` | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 882 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v03.c:2818` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 883 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2860` | `if (srcSize < ZSTD_frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 884 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2862` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 885 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2874` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 886 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2885` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 887 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2889` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 888 | `ZSTD_decompressDCtx` <br/>`src/legacy/zstd_v03.c:2892` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 889 | `ZSTD_createDCtx` <br/>`src/legacy/zstd_v03.c:2984` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 890 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v03.c:3003` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 891 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v03.c:3012` | `if (magicNumber != ZSTD_magicNumber) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 892 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v03.c:3051` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 893 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v03.c:3057` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 894 | `BIT_initDStream` <br/>`src/legacy/zstd_v04.c:603` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 895 | `BIT_initDStream` <br/>`src/legacy/zstd_v04.c:612` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 896 | `BIT_initDStream` <br/>`src/legacy/zstd_v04.c:632` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 897 | `FSE_buildDTable` <br/>`src/legacy/zstd_v04.c:1033` | `if (maxSymbolValue > FSE_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 898 | `FSE_buildDTable` <br/>`src/legacy/zstd_v04.c:1034` | `if (tableLog > FSE_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 899 | `FSE_buildDTable` <br/>`src/legacy/zstd_v04.c:1065` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 900 | `FSE_readNCount` <br/>`src/legacy/zstd_v04.c:1114` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 901 | `FSE_readNCount` <br/>`src/legacy/zstd_v04.c:1117` | `if (nbBits > FSE_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 902 | `FSE_readNCount` <br/>`src/legacy/zstd_v04.c:1152` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 903 | `FSE_readNCount` <br/>`src/legacy/zstd_v04.c:1204` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 904 | `FSE_readNCount` <br/>`src/legacy/zstd_v04.c:1208` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 905 | `FSE_buildDTable_raw` <br/>`src/legacy/zstd_v04.c:1246` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 906 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v04.c:1325` | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ZSTD_error_dstSize_tooSmall` |
| 907 | `FSE_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v04.c:1327` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 908 | `FSE_decompress` <br/>`src/legacy/zstd_v04.c:1357` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 909 | `FSE_decompress` <br/>`src/legacy/zstd_v04.c:1362` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 910 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1647` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 911 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1664` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 912 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1665` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 913 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1676` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 914 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1686` | `if (huffWeight[n] >= HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 915 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1690` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 916 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1694` | `if (tableLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 917 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1700` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 918 | `HUF_readStats` <br/>`src/legacy/zstd_v04.c:1706` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 919 | `HUF_readDTableX2` <br/>`src/legacy/zstd_v04.c:1738` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 920 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1815` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 921 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1850` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 922 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1885` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 923 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1886` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 924 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1887` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 925 | `HUF_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v04.c:1898` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 926 | `HUF_decompress4X2` <br/>`src/legacy/zstd_v04.c:1914` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 927 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v04.c:2034` | `if (memLog > HUF_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 928 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v04.c:2041` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 929 | `HUF_readDTableX4` <br/>`src/legacy/zstd_v04.c:2045` | `{ if (!maxW) return ERROR(GENERIC); }  /* necessarily finds a solution before maxW==0 */` | `ZSTD_error_GENERIC` |
| 930 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2173` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 931 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2208` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 932 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2243` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 933 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2244` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 934 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2245` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 935 | `HUF_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v04.c:2256` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 936 | `HUF_decompress4X4` <br/>`src/legacy/zstd_v04.c:2271` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 937 | `HUF_decompress` <br/>`src/legacy/zstd_v04.c:2318` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 938 | `HUF_decompress` <br/>`src/legacy/zstd_v04.c:2319` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 939 | `ZSTD_createDCtx` <br/>`src/legacy/zstd_v04.c:2472` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 940 | `ZSTD_decodeFrameHeader_Part1` <br/>`src/legacy/zstd_v04.c:2494` | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 941 | `ZSTD_decodeFrameHeader_Part1` <br/>`src/legacy/zstd_v04.c:2496` | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 942 | `ZSTD_getFrameParams` <br/>`src/legacy/zstd_v04.c:2507` | `if (magicNumber != ZSTD_MAGICNUMBER) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 943 | `ZSTD_getFrameParams` <br/>`src/legacy/zstd_v04.c:2510` | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported);   /* reserved bits */` | `ZSTD_error_frameParameter_unsupported` |
| 944 | `ZSTD_decodeFrameHeader_Part2` <br/>`src/legacy/zstd_v04.c:2521` | `if (srcSize != zc->headerSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 945 | `ZSTD_decodeFrameHeader_Part2` <br/>`src/legacy/zstd_v04.c:2523` | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 946 | `ZSTD_getcBlockSize` <br/>`src/legacy/zstd_v04.c:2534` | `if (srcSize < 3) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 947 | `ZSTD_copyRawBlock` <br/>`src/legacy/zstd_v04.c:2549` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 948 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v04.c:2567` | `if (litSize > *maxDstSizePtr) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 949 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v04.c:2568` | `if (litCSize + 5 > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 950 | `ZSTD_decompressLiterals` <br/>`src/legacy/zstd_v04.c:2570` | `if (HUF_isError(HUF_decompress(dst, litSize, ip+5, litCSize))) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 951 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v04.c:2585` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 952 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v04.c:2604` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 953 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v04.c:2605` | `if (litSize > srcSize-3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 954 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v04.c:2619` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 955 | `ZSTD_decodeLiteralsBlock` <br/>`src/legacy/zstd_v04.c:2626` | `return ERROR(corruption_detected);   /* forbidden nominal case */` | `ZSTD_error_corruption_detected` |
| 956 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2643` | `if (srcSize < 5) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 957 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2667` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 958 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2686` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 959 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2687` | `if (LLlog > LLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 960 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2696` | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 961 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2705` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 962 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2706` | `if (Offlog > OffFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 963 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2715` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 964 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2723` | `if (FSE_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 965 | `ZSTD_decodeSeqHeaders` <br/>`src/legacy/zstd_v04.c:2724` | `if (MLlog > MLFSELog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 966 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2826` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 967 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2827` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 968 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2829` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 969 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2831` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 970 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2832` | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 971 | `ZSTD_execSequence` <br/>`src/legacy/zstd_v04.c:2844` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 972 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v04.c:2940` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 973 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v04.c:2956` | `if ( !BIT_endOfDStream(&(seqState.DStream)) ) return ERROR(corruption_detected);   /* DStream should be entirely and exactly consumed; ot...` | `ZSTD_error_corruption_detected` |
| 974 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v04.c:2961` | `if (litPtr > litEnd) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 975 | `ZSTD_decompressSequences` <br/>`src/legacy/zstd_v04.c:2962` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 976 | `ZSTD_decompressBlock_internal` <br/>`src/legacy/zstd_v04.c:2994` | `if (srcSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 977 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3036` | `if (srcSize < ZSTD_frameHeaderSize_min+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 978 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3039` | `if (srcSize < frameHeaderSize+ZSTD_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 979 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3054` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 980 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3065` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 981 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3069` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 982 | `ZSTD_decompress_usingDict` <br/>`src/legacy/zstd_v04.c:3072` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 983 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3149` | `if (srcSize != ctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 984 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3157` | `if (srcSize != ZSTD_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ZSTD_error_srcSize_wrong` |
| 985 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3161` | `if (ctx->headerSize > ZSTD_frameHeaderSize_min) return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 986 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3203` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 987 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3209` | `return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 988 | `ZSTD_decompressContinue` <br/>`src/legacy/zstd_v04.c:3218` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 989 | `ZBUFF_createDCtx` <br/>`src/legacy/zstd_v04.c:3327` | `if (zbc==NULL) return NULL;` | returns `NULL` |
| 990 | `ZBUFF_decompressContinue` <br/>`src/legacy/zstd_v04.c:3391` | `return ERROR(init_missing);` | `ZSTD_error_init_missing` |
| 991 | `ZBUFF_decompressContinue` <br/>`src/legacy/zstd_v04.c:3433` | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 992 | `ZBUFF_decompressContinue` <br/>`src/legacy/zstd_v04.c:3439` | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 993 | `ZBUFF_decompressContinue` <br/>`src/legacy/zstd_v04.c:3484` | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ZSTD_error_corruption_detected` |
| 994 | `ZBUFF_decompressContinue` <br/>`src/legacy/zstd_v04.c:3519` | `default: return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 995 | `ZSTDv04_decompress` <br/>`src/legacy/zstd_v04.c:3560` | `if (dctx==NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 996 | `BITv05_initDStream` <br/>`src/legacy/zstd_v05.c:736` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 997 | `BITv05_initDStream` <br/>`src/legacy/zstd_v05.c:744` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 998 | `BITv05_initDStream` <br/>`src/legacy/zstd_v05.c:762` | `if (contain32 == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 999 | `FSEv05_buildDTable` <br/>`src/legacy/zstd_v05.c:1173` | `if (maxSymbolValue > FSEv05_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 1000 | `FSEv05_buildDTable` <br/>`src/legacy/zstd_v05.c:1174` | `if (tableLog > FSEv05_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1001 | `FSEv05_buildDTable` <br/>`src/legacy/zstd_v05.c:1197` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 1002 | `FSEv05_readNCount` <br/>`src/legacy/zstd_v05.c:1244` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1003 | `FSEv05_readNCount` <br/>`src/legacy/zstd_v05.c:1247` | `if (nbBits > FSEv05_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1004 | `FSEv05_readNCount` <br/>`src/legacy/zstd_v05.c:1274` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 1005 | `FSEv05_readNCount` <br/>`src/legacy/zstd_v05.c:1315` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1006 | `FSEv05_readNCount` <br/>`src/legacy/zstd_v05.c:1319` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1007 | `FSEv05_buildDTable_raw` <br/>`src/legacy/zstd_v05.c:1358` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 1008 | `FSEv05_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v05.c:1434` | `if (op==omax) return ERROR(dstSize_tooSmall);   /* dst buffer is full, but cSrc unfinished */` | `ZSTD_error_dstSize_tooSmall` |
| 1009 | `FSEv05_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v05.c:1436` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1010 | `FSEv05_decompress` <br/>`src/legacy/zstd_v05.c:1464` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1011 | `FSEv05_decompress` <br/>`src/legacy/zstd_v05.c:1469` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1012 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1753` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1013 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1767` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1014 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1768` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1015 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1775` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1016 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1784` | `if (huffWeight[n] >= HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1017 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1788` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1018 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1792` | `if (tableLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1019 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1798` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 1020 | `HUFv05_readStats` <br/>`src/legacy/zstd_v05.c:1804` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 1021 | `HUFv05_readDTableX2` <br/>`src/legacy/zstd_v05.c:1836` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 1022 | `HUFv05_decompress1X2_usingDTable` <br/>`src/legacy/zstd_v05.c:1916` | `if (dstSize <= cSrcSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1023 | `HUFv05_decompress1X2_usingDTable` <br/>`src/legacy/zstd_v05.c:1923` | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1024 | `HUFv05_decompress1X2` <br/>`src/legacy/zstd_v05.c:1936` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1025 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:1950` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1026 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:1984` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1027 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:2017` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1028 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:2018` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1029 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:2019` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1030 | `HUFv05_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v05.c:2030` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1031 | `HUFv05_decompress4X2` <br/>`src/legacy/zstd_v05.c:2046` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1032 | `HUFv05_readDTableX4` <br/>`src/legacy/zstd_v05.c:2160` | `if (memLog > HUFv05_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1033 | `HUFv05_readDTableX4` <br/>`src/legacy/zstd_v05.c:2167` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 1034 | `HUFv05_decompress1X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2306` | `if (!BITv05_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1035 | `HUFv05_decompress1X4` <br/>`src/legacy/zstd_v05.c:2319` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1036 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2331` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1037 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2366` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1038 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2400` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1039 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2401` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1040 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2402` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1041 | `HUFv05_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v05.c:2413` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1042 | `HUFv05_decompress4X4` <br/>`src/legacy/zstd_v05.c:2428` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1043 | `HUFv05_decompress` <br/>`src/legacy/zstd_v05.c:2475` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1044 | `HUFv05_decompress` <br/>`src/legacy/zstd_v05.c:2476` | `if (cSrcSize >= dstSize) return ERROR(corruption_detected);   /* invalid, or not compressed, but not compressed already dealt with */` | `ZSTD_error_corruption_detected` |
| 1045 | `ZSTDv05_createDCtx` <br/>`src/legacy/zstd_v05.c:2632` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 1046 | `ZSTDv05_decodeFrameHeader_Part1` <br/>`src/legacy/zstd_v05.c:2743` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1047 | `ZSTDv05_decodeFrameHeader_Part1` <br/>`src/legacy/zstd_v05.c:2745` | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 1048 | `ZSTDv05_getFrameParams` <br/>`src/legacy/zstd_v05.c:2756` | `if (magicNumber != ZSTDv05_MAGICNUMBER) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 1049 | `ZSTDv05_getFrameParams` <br/>`src/legacy/zstd_v05.c:2759` | `if ((((const BYTE*)src)[4] >> 4) != 0) return ERROR(frameParameter_unsupported);   /* reserved bits */` | `ZSTD_error_frameParameter_unsupported` |
| 1050 | `ZSTDv05_decodeFrameHeader_Part2` <br/>`src/legacy/zstd_v05.c:2771` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1051 | `ZSTDv05_decodeFrameHeader_Part2` <br/>`src/legacy/zstd_v05.c:2773` | `if ((MEM_32bits()) && (zc->params.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 1052 | `ZSTDv05_getcBlockSize` <br/>`src/legacy/zstd_v05.c:2785` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1053 | `ZSTDv05_copyRawBlock` <br/>`src/legacy/zstd_v05.c:2801` | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1054 | `ZSTDv05_copyRawBlock` <br/>`src/legacy/zstd_v05.c:2802` | `if (srcSize > maxDstSize) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1055 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2816` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1056 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2824` | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for case 3 */` | `ZSTD_error_corruption_detected` |
| 1057 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2847` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1058 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2848` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1059 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2853` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1060 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2866` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1061 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2868` | `return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1062 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2874` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1063 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2877` | `if (HUFv05_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1064 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2903` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1065 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2930` | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ZSTD_error_corruption_detected` |
| 1066 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2933` | `if (litSize > BLOCKSIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1067 | `ZSTDv05_decodeLiteralsBlock` <br/>`src/legacy/zstd_v05.c:2940` | `return ERROR(corruption_detected);   /* impossible */` | `ZSTD_error_corruption_detected` |
| 1068 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2958` | `return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1069 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2964` | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1070 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2968` | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1071 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2973` | `if (ip+3 > iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1072 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2978` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1073 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:2988` | `if (ip > iend-3) return ERROR(srcSize_wrong); /* min : all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 1074 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3007` | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1075 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3013` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1076 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3014` | `if (LLlog > LLFSEv05Log) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1077 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3023` | `if (ip > iend-2) return ERROR(srcSize_wrong);   /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 1078 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3031` | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1079 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3037` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1080 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3038` | `if (Offlog > OffFSEv05Log) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1081 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3047` | `if (ip > iend-2) return ERROR(srcSize_wrong); /* min : "raw", hence no header, but at least xxLog bits */` | `ZSTD_error_srcSize_wrong` |
| 1082 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3055` | `if (!flagStaticTable) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1083 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3061` | `if (FSEv05_isError(headerSize)) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1084 | `ZSTDv05_decodeSeqHeaders` <br/>`src/legacy/zstd_v05.c:3062` | `if (MLlog > MLFSEv05Log) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1085 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3188` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1086 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3189` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1087 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3191` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1088 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3193` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 1089 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3194` | `if (litEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 1090 | `ZSTDv05_execSequence` <br/>`src/legacy/zstd_v05.c:3205` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1091 | `ZSTDv05_decompressSequences` <br/>`src/legacy/zstd_v05.c:3296` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1092 | `ZSTDv05_decompressSequences` <br/>`src/legacy/zstd_v05.c:3311` | `if (nbSeq) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1093 | `ZSTDv05_decompressSequences` <br/>`src/legacy/zstd_v05.c:3317` | `if (litPtr > litEnd) return ERROR(corruption_detected);   /* too many literals already used */` | `ZSTD_error_corruption_detected` |
| 1094 | `ZSTDv05_decompressSequences` <br/>`src/legacy/zstd_v05.c:3318` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1095 | `ZSTDv05_decompressBlock_internal` <br/>`src/legacy/zstd_v05.c:3347` | `if (srcSize >= BLOCKSIZE) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1096 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3385` | `if (srcSize < ZSTDv05_frameHeaderSize_min+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1097 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3388` | `if (srcSize < frameHeaderSize+ZSTDv05_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1098 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3403` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1099 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3414` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 1100 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3418` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1101 | `ZSTDv05_decompress_continueDCtx` <br/>`src/legacy/zstd_v05.c:3421` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1102 | `ZSTDv05_decompress` <br/>`src/legacy/zstd_v05.c:3466` | `if (dctx==NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1103 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3540` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1104 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3548` | `if (srcSize != ZSTDv05_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ZSTD_error_srcSize_wrong` |
| 1105 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3552` | `if (dctx->headerSize > ZSTDv05_frameHeaderSize_min) return ERROR(GENERIC); /* should never happen */` | `ZSTD_error_GENERIC` |
| 1106 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3593` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 1107 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3599` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1108 | `ZSTDv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3608` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1109 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3632` | `if (HUFv05_isError(hSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1110 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3637` | `if (FSEv05_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1111 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3638` | `if (offcodeLog > OffFSEv05Log) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1112 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3640` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1113 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3645` | `if (FSEv05_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1114 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3646` | `if (matchlengthLog > MLFSEv05Log) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1115 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3648` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1116 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3653` | `if (litlengthLog > LLFSEv05Log) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1117 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3654` | `if (FSEv05_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1118 | `ZSTDv05_loadEntropy` <br/>`src/legacy/zstd_v05.c:3656` | `if (FSEv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1119 | `ZSTDv05_decompress_insertDictionary` <br/>`src/legacy/zstd_v05.c:3675` | `if (ZSTDv05_isError(eSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1120 | `ZSTDv05_decompressBegin_usingDict` <br/>`src/legacy/zstd_v05.c:3694` | `if (ZSTDv05_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1121 | `ZBUFFv05_createDCtx` <br/>`src/legacy/zstd_v05.c:3807` | `if (zbc==NULL) return NULL;` | returns `NULL` |
| 1122 | `ZBUFFv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3856` | `return ERROR(init_missing);` | `ZSTD_error_init_missing` |
| 1123 | `ZBUFFv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3902` | `if (zbc->inBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1124 | `ZBUFFv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3908` | `if (zbc->outBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1125 | `ZBUFFv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3949` | `if (toLoad > zbc->inBuffSize - zbc->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ZSTD_error_corruption_detected` |
| 1126 | `ZBUFFv05_decompressContinue` <br/>`src/legacy/zstd_v05.c:3983` | `default: return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1127 | `BITv06_initDStream` <br/>`src/legacy/zstd_v06.c:835` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 1128 | `BITv06_initDStream` <br/>`src/legacy/zstd_v06.c:842` | `if (lastByte == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 1129 | `BITv06_initDStream` <br/>`src/legacy/zstd_v06.c:859` | `if (lastByte == 0) return ERROR(GENERIC);   /* endMark not present */` | `ZSTD_error_GENERIC` |
| 1130 | `FSEv06_readNCount` <br/>`src/legacy/zstd_v06.c:1221` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1131 | `FSEv06_readNCount` <br/>`src/legacy/zstd_v06.c:1224` | `if (nbBits > FSEv06_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1132 | `FSEv06_readNCount` <br/>`src/legacy/zstd_v06.c:1251` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 1133 | `FSEv06_readNCount` <br/>`src/legacy/zstd_v06.c:1291` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1134 | `FSEv06_readNCount` <br/>`src/legacy/zstd_v06.c:1295` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1135 | `FSEv06_buildDTable` <br/>`src/legacy/zstd_v06.c:1413` | `if (maxSymbolValue > FSEv06_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 1136 | `FSEv06_buildDTable` <br/>`src/legacy/zstd_v06.c:1414` | `if (tableLog > FSEv06_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1137 | `FSEv06_buildDTable` <br/>`src/legacy/zstd_v06.c:1445` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 1138 | `FSEv06_buildDTable_raw` <br/>`src/legacy/zstd_v06.c:1497` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 1139 | `FSEv06_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v06.c:1557` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1140 | `FSEv06_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v06.c:1566` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1141 | `FSEv06_decompress` <br/>`src/legacy/zstd_v06.c:1602` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1142 | `FSEv06_decompress` <br/>`src/legacy/zstd_v06.c:1607` | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1143 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1807` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1144 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1821` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1145 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1822` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1146 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1830` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1147 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1839` | `if (huffWeight[n] >= HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1148 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1843` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1149 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1847` | `if (tableLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1150 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1854` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 1151 | `HUFv06_readStats` <br/>`src/legacy/zstd_v06.c:1860` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 1152 | `HUFv06_readDTableX2` <br/>`src/legacy/zstd_v06.c:1967` | `if (tableLog > DTable[0]) return ERROR(tableLog_tooLarge);   /* DTable is too small */` | `ZSTD_error_tableLog_tooLarge` |
| 1153 | `HUFv06_decompress1X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2054` | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1154 | `HUFv06_decompress1X2` <br/>`src/legacy/zstd_v06.c:2066` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1155 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2080` | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1156 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2114` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1157 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2147` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1158 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2148` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1159 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2149` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1160 | `HUFv06_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v06.c:2160` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1161 | `HUFv06_decompress4X2` <br/>`src/legacy/zstd_v06.c:2175` | `if (errorCode >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1162 | `HUFv06_readDTableX4` <br/>`src/legacy/zstd_v06.c:2286` | `if (memLog > HUFv06_ABSOLUTEMAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1163 | `HUFv06_readDTableX4` <br/>`src/legacy/zstd_v06.c:2293` | `if (tableLog > memLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 1164 | `HUFv06_decompress1X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2430` | `if (!BITv06_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1165 | `HUFv06_decompress1X4` <br/>`src/legacy/zstd_v06.c:2443` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1166 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2455` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1167 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2489` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1168 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2523` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1169 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2524` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1170 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2525` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1171 | `HUFv06_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v06.c:2536` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1172 | `HUFv06_decompress4X4` <br/>`src/legacy/zstd_v06.c:2551` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1173 | `HUFv06_decompress` <br/>`src/legacy/zstd_v06.c:2595` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1174 | `HUFv06_decompress` <br/>`src/legacy/zstd_v06.c:2596` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 1175 | `ZSTDv06_createDCtx` <br/>`src/legacy/zstd_v06.c:2789` | `if (dctx==NULL) return NULL;` | returns `NULL` |
| 1176 | `ZSTDv06_frameHeaderSize` <br/>`src/legacy/zstd_v06.c:2913` | `if (srcSize < ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1177 | `ZSTDv06_getFrameParams` <br/>`src/legacy/zstd_v06.c:2929` | `if (MEM_readLE32(src) != ZSTDv06_MAGICNUMBER) return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 1178 | `ZSTDv06_getFrameParams` <br/>`src/legacy/zstd_v06.c:2938` | `if ((frameDesc & 0x20) != 0) return ERROR(frameParameter_unsupported);   /* reserved 1 bit */` | `ZSTD_error_frameParameter_unsupported` |
| 1179 | `ZSTDv06_decodeFrameHeader` <br/>`src/legacy/zstd_v06.c:2957` | `if ((MEM_32bits()) && (zc->fParams.windowLog > 25)) return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 1180 | `ZSTDv06_getcBlockSize` <br/>`src/legacy/zstd_v06.c:2975` | `if (srcSize < ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1181 | `ZSTDv06_copyRawBlock` <br/>`src/legacy/zstd_v06.c:2989` | `if (dst==NULL) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1182 | `ZSTDv06_copyRawBlock` <br/>`src/legacy/zstd_v06.c:2990` | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1183 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3004` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1184 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3011` | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSe...` | `ZSTD_error_corruption_detected` |
| 1185 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3034` | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1186 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3035` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1187 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3040` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1188 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3051` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1189 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3053` | `return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1190 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3059` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1191 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3062` | `if (HUFv06_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1192 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3087` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1193 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3113` | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ZSTD_error_corruption_detected` |
| 1194 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3116` | `if (litSize > ZSTDv06_BLOCKSIZE_MAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1195 | `ZSTDv06_decodeLiteralsBlock` <br/>`src/legacy/zstd_v06.c:3123` | `return ERROR(corruption_detected);   /* impossible */` | `ZSTD_error_corruption_detected` |
| 1196 | `ZSTDv06_buildSeqTable` <br/>`src/legacy/zstd_v06.c:3139` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1197 | `ZSTDv06_buildSeqTable` <br/>`src/legacy/zstd_v06.c:3140` | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1198 | `ZSTDv06_buildSeqTable` <br/>`src/legacy/zstd_v06.c:3147` | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1199 | `ZSTDv06_buildSeqTable` <br/>`src/legacy/zstd_v06.c:3154` | `if (FSEv06_isError(headerSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1200 | `ZSTDv06_buildSeqTable` <br/>`src/legacy/zstd_v06.c:3155` | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1201 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3171` | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1202 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3178` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1203 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3181` | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1204 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3189` | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 1205 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3197` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1206 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3201` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1207 | `ZSTDv06_decodeSeqHeaders` <br/>`src/legacy/zstd_v06.c:3205` | `if (ZSTDv06_isError(bhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1208 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3320` | `if (seqLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1209 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3321` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1210 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3323` | `if (oLitEnd > oend_8) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1211 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3325` | `if (oMatchEnd > oend) return ERROR(dstSize_tooSmall);   /* overwrite beyond dst buffer */` | `ZSTD_error_dstSize_tooSmall` |
| 1212 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3326` | `if (iLitEnd > litLimit) return ERROR(corruption_detected);   /* overRead beyond lit buffer */` | `ZSTD_error_corruption_detected` |
| 1213 | `ZSTDv06_execSequence` <br/>`src/legacy/zstd_v06.c:3336` | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1214 | `ZSTDv06_decompressSequences` <br/>`src/legacy/zstd_v06.c:3423` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ZSTD_error_corruption_detected` |
| 1215 | `ZSTDv06_decompressSequences` <br/>`src/legacy/zstd_v06.c:3447` | `if (nbSeq) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1216 | `ZSTDv06_decompressSequences` <br/>`src/legacy/zstd_v06.c:3452` | `if (litPtr > litEnd) return ERROR(corruption_detected);   /* too many literals already used */` | `ZSTD_error_corruption_detected` |
| 1217 | `ZSTDv06_decompressSequences` <br/>`src/legacy/zstd_v06.c:3453` | `if (op+lastLLSize > oend) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1218 | `ZSTDv06_decompressBlock_internal` <br/>`src/legacy/zstd_v06.c:3481` | `if (srcSize >= ZSTDv06_BLOCKSIZE_MAX) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1219 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3517` | `if (srcSize < ZSTDv06_frameHeaderSize_min+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1220 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3522` | `if (srcSize < frameHeaderSize+ZSTDv06_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1221 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3523` | `if (ZSTDv06_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1222 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3535` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1223 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3546` | `return ERROR(GENERIC);   /* not yet supported */` | `ZSTD_error_GENERIC` |
| 1224 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3550` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1225 | `ZSTDv06_decompressFrame` <br/>`src/legacy/zstd_v06.c:3553` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1226 | `ZSTDv06_decompress` <br/>`src/legacy/zstd_v06.c:3599` | `if (dctx==NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1227 | `ZSTDv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3678` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1228 | `ZSTDv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3685` | `if (srcSize != ZSTDv06_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ZSTD_error_srcSize_wrong` |
| 1229 | `ZSTDv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3730` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 1230 | `ZSTDv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3736` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1231 | `ZSTDv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3745` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1232 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3763` | `if (HUFv06_isError(hSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1233 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3770` | `if (FSEv06_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1234 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3771` | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1235 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3773` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1236 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3781` | `if (FSEv06_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1237 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3782` | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1238 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3784` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1239 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3792` | `if (FSEv06_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1240 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3793` | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1241 | `ZSTDv06_loadEntropy` <br/>`src/legacy/zstd_v06.c:3795` | `if (FSEv06_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1242 | `ZSTDv06_decompress_insertDictionary` <br/>`src/legacy/zstd_v06.c:3815` | `if (ZSTDv06_isError(eSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1243 | `ZSTDv06_decompressBegin_usingDict` <br/>`src/legacy/zstd_v06.c:3833` | `if (ZSTDv06_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1244 | `ZBUFFv06_createDCtx` <br/>`src/legacy/zstd_v06.c:3919` | `if (zbd==NULL) return NULL;` | returns `NULL` |
| 1245 | `ZBUFFv06_createDCtx` <br/>`src/legacy/zstd_v06.c:3924` | `return NULL;` | returns `NULL` |
| 1246 | `ZBUFFv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:3985` | `return ERROR(init_missing);` | `ZSTD_error_init_missing` |
| 1247 | `ZBUFFv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:4020` | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1248 | `ZBUFFv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:4027` | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1249 | `ZBUFFv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:4057` | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ZSTD_error_corruption_detected` |
| 1250 | `ZBUFFv06_decompressContinue` <br/>`src/legacy/zstd_v06.c:4091` | `default: return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1251 | `BITv07_initDStream` <br/>`src/legacy/zstd_v07.c:504` | `if (srcSize < 1) { memset(bitD, 0, sizeof(*bitD)); return ERROR(srcSize_wrong); }` | `ZSTD_error_srcSize_wrong` |
| 1252 | `BITv07_initDStream` <br/>`src/legacy/zstd_v07.c:512` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ZSTD_error_GENERIC` |
| 1253 | `BITv07_initDStream` <br/>`src/legacy/zstd_v07.c:529` | `if (lastByte == 0) return ERROR(GENERIC); /* endMark not present */ }` | `ZSTD_error_GENERIC` |
| 1254 | `FSEv07_readNCount` <br/>`src/legacy/zstd_v07.c:1166` | `if (hbSize < 4) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1255 | `FSEv07_readNCount` <br/>`src/legacy/zstd_v07.c:1169` | `if (nbBits > FSEv07_TABLELOG_ABSOLUTE_MAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1256 | `FSEv07_readNCount` <br/>`src/legacy/zstd_v07.c:1196` | `if (n0 > *maxSVPtr) return ERROR(maxSymbolValue_tooSmall);` | `ZSTD_error_maxSymbolValue_tooSmall` |
| 1257 | `FSEv07_readNCount` <br/>`src/legacy/zstd_v07.c:1236` | `if (remaining != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1258 | `FSEv07_readNCount` <br/>`src/legacy/zstd_v07.c:1240` | `if ((size_t)(ip-istart) > hbSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1259 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1260` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1260 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1274` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1261 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1275` | `if (oSize >= hwSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1262 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1283` | `if (iSize+1 > srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1263 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1292` | `if (huffWeight[n] >= HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1264 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1296` | `if (weightTotal == 0) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1265 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1300` | `if (tableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1266 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1307` | `if (verif != rest) return ERROR(corruption_detected);    /* last value must be a clean power of 2 */` | `ZSTD_error_corruption_detected` |
| 1267 | `HUFv07_readStats` <br/>`src/legacy/zstd_v07.c:1313` | `if ((rankStats[1] < 2) \|\| (rankStats[1] & 1)) return ERROR(corruption_detected);   /* by construction : at least 2 elts of rank 1, must...` | `ZSTD_error_corruption_detected` |
| 1268 | `FSEv07_buildDTable` <br/>`src/legacy/zstd_v07.c:1434` | `if (maxSymbolValue > FSEv07_MAX_SYMBOL_VALUE) return ERROR(maxSymbolValue_tooLarge);` | `ZSTD_error_maxSymbolValue_tooLarge` |
| 1269 | `FSEv07_buildDTable` <br/>`src/legacy/zstd_v07.c:1435` | `if (tableLog > FSEv07_MAX_TABLELOG) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1270 | `FSEv07_buildDTable` <br/>`src/legacy/zstd_v07.c:1466` | `if (position!=0) return ERROR(GENERIC);   /* position must reach all cells once, otherwise normalizedCounter is incorrect */` | `ZSTD_error_GENERIC` |
| 1271 | `FSEv07_buildDTable_raw` <br/>`src/legacy/zstd_v07.c:1518` | `if (nbBits < 1) return ERROR(GENERIC);         /* min size */` | `ZSTD_error_GENERIC` |
| 1272 | `FSEv07_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v07.c:1578` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1273 | `FSEv07_decompress_usingDTable_generic` <br/>`src/legacy/zstd_v07.c:1587` | `if (op>(omax-2)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1274 | `FSEv07_decompress` <br/>`src/legacy/zstd_v07.c:1623` | `if (cSrcSize<2) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1275 | `FSEv07_decompress` <br/>`src/legacy/zstd_v07.c:1628` | `if (NCountLength >= cSrcSize) return ERROR(srcSize_wrong);   /* too small input size */` | `ZSTD_error_srcSize_wrong` |
| 1276 | `HUFv07_readDTableX2` <br/>`src/legacy/zstd_v07.c:1739` | `if (tableLog > (U32)(dtd.maxTableLog+1)) return ERROR(tableLog_tooLarge);   /* DTable too small, huffman tree cannot fit in */` | `ZSTD_error_tableLog_tooLarge` |
| 1277 | `HUFv07_decompress1X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1831` | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1278 | `HUFv07_decompress1X2_usingDTable` <br/>`src/legacy/zstd_v07.c:1842` | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1279 | `HUFv07_decompress1X2_DCtx` <br/>`src/legacy/zstd_v07.c:1852` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1280 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1871` | `if (cSrcSize < 10) return ERROR(corruption_detected);  /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1281 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1904` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1282 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1937` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1283 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1938` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1284 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1939` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1285 | `HUFv07_decompress4X2_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:1950` | `if (!endSignal) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1286 | `HUFv07_decompress4X2_usingDTable` <br/>`src/legacy/zstd_v07.c:1964` | `if (dtd.tableType != 0) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1287 | `HUFv07_decompress4X2_DCtx` <br/>`src/legacy/zstd_v07.c:1975` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1288 | `HUFv07_readDTableX4` <br/>`src/legacy/zstd_v07.c:2095` | `if (maxTableLog > HUFv07_TABLELOG_ABSOLUTEMAX) return ERROR(tableLog_tooLarge);` | `ZSTD_error_tableLog_tooLarge` |
| 1289 | `HUFv07_readDTableX4` <br/>`src/legacy/zstd_v07.c:2102` | `if (tableLog > maxTableLog) return ERROR(tableLog_tooLarge);   /* DTable can't fit code depth */` | `ZSTD_error_tableLog_tooLarge` |
| 1290 | `HUFv07_decompress1X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2242` | `if (!BITv07_endOfDStream(&bitD)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1291 | `HUFv07_decompress1X4_usingDTable` <br/>`src/legacy/zstd_v07.c:2254` | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1292 | `HUFv07_decompress1X4_DCtx` <br/>`src/legacy/zstd_v07.c:2264` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1293 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2281` | `if (cSrcSize < 10) return ERROR(corruption_detected);   /* strict minimum : jump table + 1 byte per stream */` | `ZSTD_error_corruption_detected` |
| 1294 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2314` | `if (length4 > cSrcSize) return ERROR(corruption_detected);   /* overflow */` | `ZSTD_error_corruption_detected` |
| 1295 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2348` | `if (op1 > opStart2) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1296 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2349` | `if (op2 > opStart3) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1297 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2350` | `if (op3 > opStart4) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1298 | `HUFv07_decompress4X4_usingDTable_internal` <br/>`src/legacy/zstd_v07.c:2361` | `if (!endCheck) return ERROR(corruption_detected); }` | `ZSTD_error_corruption_detected` |
| 1299 | `HUFv07_decompress4X4_usingDTable` <br/>`src/legacy/zstd_v07.c:2375` | `if (dtd.tableType != 1) return ERROR(GENERIC);` | `ZSTD_error_GENERIC` |
| 1300 | `HUFv07_decompress4X4_DCtx` <br/>`src/legacy/zstd_v07.c:2386` | `if (hSize >= cSrcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1301 | `HUFv07_decompress` <br/>`src/legacy/zstd_v07.c:2469` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1302 | `HUFv07_decompress` <br/>`src/legacy/zstd_v07.c:2470` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 1303 | `HUFv07_decompress4X_DCtx` <br/>`src/legacy/zstd_v07.c:2485` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1304 | `HUFv07_decompress4X_DCtx` <br/>`src/legacy/zstd_v07.c:2486` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 1305 | `HUFv07_decompress4X_hufOnly` <br/>`src/legacy/zstd_v07.c:2499` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1306 | `HUFv07_decompress4X_hufOnly` <br/>`src/legacy/zstd_v07.c:2500` | `if ((cSrcSize >= dstSize) \|\| (cSrcSize <= 1)) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 1307 | `HUFv07_decompress1X_DCtx` <br/>`src/legacy/zstd_v07.c:2511` | `if (dstSize == 0) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1308 | `HUFv07_decompress1X_DCtx` <br/>`src/legacy/zstd_v07.c:2512` | `if (cSrcSize > dstSize) return ERROR(corruption_detected);   /* invalid */` | `ZSTD_error_corruption_detected` |
| 1309 | `ZSTDv07_createDCtx_advanced` <br/>`src/legacy/zstd_v07.c:2930` | `return NULL;` | returns `NULL` |
| 1310 | `ZSTDv07_createDCtx_advanced` <br/>`src/legacy/zstd_v07.c:2933` | `if (!dctx) return NULL;` | returns `NULL` |
| 1311 | `ZSTDv07_frameHeaderSize` <br/>`src/legacy/zstd_v07.c:3079` | `if (srcSize < ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1312 | `ZSTDv07_getFrameParams` <br/>`src/legacy/zstd_v07.c:3108` | `return ERROR(prefix_unknown);` | `ZSTD_error_prefix_unknown` |
| 1313 | `ZSTDv07_getFrameParams` <br/>`src/legacy/zstd_v07.c:3126` | `return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 1314 | `ZSTDv07_getFrameParams` <br/>`src/legacy/zstd_v07.c:3131` | `return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 1315 | `ZSTDv07_getFrameParams` <br/>`src/legacy/zstd_v07.c:3154` | `return ERROR(frameParameter_unsupported);` | `ZSTD_error_frameParameter_unsupported` |
| 1316 | `ZSTDv07_decodeFrameHeader` <br/>`src/legacy/zstd_v07.c:3186` | `if (dctx->fParams.dictID && (dctx->dictID != dctx->fParams.dictID)) return ERROR(dictionary_wrong);` | `ZSTD_error_dictionary_wrong` |
| 1317 | `ZSTDv07_getcBlockSize` <br/>`src/legacy/zstd_v07.c:3205` | `if (srcSize < ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1318 | `ZSTDv07_copyRawBlock` <br/>`src/legacy/zstd_v07.c:3219` | `if (srcSize > dstCapacity) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1319 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3234` | `if (srcSize < MIN_CBLOCK_SIZE) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1320 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3241` | `if (srcSize < 5) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need up to 5 for lhSize, + cSize (+nbSe...` | `ZSTD_error_corruption_detected` |
| 1321 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3264` | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1322 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3265` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1323 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3270` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1324 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3282` | `return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1325 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3284` | `return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1326 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3290` | `if (litCSize + lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1327 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3293` | `if (HUFv07_isError(errorCode)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1328 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3318` | `if (litSize+lhSize > srcSize) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1329 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3344` | `if (srcSize<4) return ERROR(corruption_detected);   /* srcSize >= MIN_CBLOCK_SIZE == 3; here we need lhSize+1 = 4 */` | `ZSTD_error_corruption_detected` |
| 1330 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3347` | `if (litSize > ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1331 | `ZSTDv07_decodeLiteralsBlock` <br/>`src/legacy/zstd_v07.c:3354` | `return ERROR(corruption_detected);   /* impossible */` | `ZSTD_error_corruption_detected` |
| 1332 | `ZSTDv07_buildSeqTable` <br/>`src/legacy/zstd_v07.c:3370` | `if (!srcSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1333 | `ZSTDv07_buildSeqTable` <br/>`src/legacy/zstd_v07.c:3371` | `if ( (*(const BYTE*)src) > max) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1334 | `ZSTDv07_buildSeqTable` <br/>`src/legacy/zstd_v07.c:3378` | `if (!flagRepeatTable) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1335 | `ZSTDv07_buildSeqTable` <br/>`src/legacy/zstd_v07.c:3385` | `if (FSEv07_isError(headerSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1336 | `ZSTDv07_buildSeqTable` <br/>`src/legacy/zstd_v07.c:3386` | `if (tableLog > maxLog) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1337 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3402` | `if (srcSize < MIN_SEQUENCES_SIZE) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1338 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3409` | `if (ip+2 > iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1339 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3412` | `if (ip >= iend) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1340 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3420` | `if (ip + 4 > iend) return ERROR(srcSize_wrong); /* min : header byte + all 3 are "raw", hence no header, but at least xxLog bits per type */` | `ZSTD_error_srcSize_wrong` |
| 1341 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3428` | `if (ZSTDv07_isError(llhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1342 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3432` | `if (ZSTDv07_isError(ofhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1343 | `ZSTDv07_decodeSeqHeaders` <br/>`src/legacy/zstd_v07.c:3436` | `if (ZSTDv07_isError(mlhSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1344 | `ZSTDv07_execSequence` <br/>`src/legacy/zstd_v07.c:3548` | `if (sequence.litLength + WILDCOPY_OVERLENGTH > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1345 | `ZSTDv07_execSequence` <br/>`src/legacy/zstd_v07.c:3549` | `if (sequenceLength > (size_t)(oend - op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1346 | `ZSTDv07_execSequence` <br/>`src/legacy/zstd_v07.c:3551` | `if (sequence.litLength > (size_t)(litLimit - *litPtr)) return ERROR(corruption_detected);;` | `ZSTD_error_corruption_detected` |
| 1347 | `ZSTDv07_execSequence` <br/>`src/legacy/zstd_v07.c:3561` | `if (sequence.offset > (size_t)(oLitEnd - vBase)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1348 | `ZSTDv07_decompressSequences` <br/>`src/legacy/zstd_v07.c:3644` | `if (ERR_isError(errorCode)) return ERROR(corruption_detected); }` | `ZSTD_error_corruption_detected` |
| 1349 | `ZSTDv07_decompressSequences` <br/>`src/legacy/zstd_v07.c:3658` | `if (nbSeq) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1350 | `ZSTDv07_decompressSequences` <br/>`src/legacy/zstd_v07.c:3666` | `if (lastLLSize > (size_t)(oend-op)) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1351 | `ZSTDv07_decompressBlock_internal` <br/>`src/legacy/zstd_v07.c:3694` | `if (srcSize >= ZSTDv07_BLOCKSIZE_ABSOLUTEMAX) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1352 | `ZSTDv07_generateNxBytes` <br/>`src/legacy/zstd_v07.c:3730` | `if (length > dstCapacity) return ERROR(dstSize_tooSmall);` | `ZSTD_error_dstSize_tooSmall` |
| 1353 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3752` | `if (srcSize < ZSTDv07_frameHeaderSize_min+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1354 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3757` | `if (srcSize < frameHeaderSize+ZSTDv07_blockHeaderSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1355 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3758` | `if (ZSTDv07_decodeFrameHeader(dctx, src, frameHeaderSize)) return ERROR(corruption_detected);` | `ZSTD_error_corruption_detected` |
| 1356 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3771` | `if (cBlockSize > remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1357 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3786` | `if (remainingSize) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1358 | `ZSTDv07_decompressFrame` <br/>`src/legacy/zstd_v07.c:3790` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1359 | `ZSTDv07_decompress` <br/>`src/legacy/zstd_v07.c:3842` | `if (dctx==NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1360 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:3936` | `if (srcSize != dctx->expected) return ERROR(srcSize_wrong);` | `ZSTD_error_srcSize_wrong` |
| 1361 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:3942` | `if (srcSize != ZSTDv07_frameHeaderSize_min) return ERROR(srcSize_wrong);   /* impossible */` | `ZSTD_error_srcSize_wrong` |
| 1362 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:3978` | `if (check32 != h32) return ERROR(checksum_wrong);` | `ZSTD_error_checksum_wrong` |
| 1363 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4000` | `return ERROR(GENERIC);   /* not yet handled */` | `ZSTD_error_GENERIC` |
| 1364 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4006` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1365 | `ZSTDv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4027` | `return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |
| 1366 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4047` | `if (HUFv07_isError(hSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1367 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4054` | `if (FSEv07_isError(offcodeHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1368 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4055` | `if (offcodeLog > OffFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1369 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4057` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1370 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4064` | `if (FSEv07_isError(matchlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1371 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4065` | `if (matchlengthLog > MLFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1372 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4067` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1373 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4074` | `if (FSEv07_isError(litlengthHeaderSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1374 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4075` | `if (litlengthLog > LLFSELog) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1375 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4077` | `if (FSEv07_isError(errorCode)) return ERROR(dictionary_corrupted); }` | `ZSTD_error_dictionary_corrupted` |
| 1376 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4081` | `if (dictPtr+12 > dictEnd) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1377 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4082` | `dctx->rep[0] = MEM_readLE32(dictPtr+0); if (dctx->rep[0] == 0 \|\| dctx->rep[0] >= dictSize) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1378 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4083` | `dctx->rep[1] = MEM_readLE32(dictPtr+4); if (dctx->rep[1] == 0 \|\| dctx->rep[1] >= dictSize) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1379 | `ZSTDv07_loadEntropy` <br/>`src/legacy/zstd_v07.c:4084` | `dctx->rep[2] = MEM_readLE32(dictPtr+8); if (dctx->rep[2] == 0 \|\| dctx->rep[2] >= dictSize) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1380 | `ZSTDv07_decompress_insertDictionary` <br/>`src/legacy/zstd_v07.c:4104` | `if (ZSTDv07_isError(eSize)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1381 | `ZSTDv07_decompressBegin_usingDict` <br/>`src/legacy/zstd_v07.c:4121` | `if (ZSTDv07_isError(errorCode)) return ERROR(dictionary_corrupted);` | `ZSTD_error_dictionary_corrupted` |
| 1382 | `ZSTDv07_createDDict_advanced` <br/>`src/legacy/zstd_v07.c:4140` | `return NULL;` | returns `NULL` |
| 1383 | `ZSTDv07_createDDict_advanced` <br/>`src/legacy/zstd_v07.c:4150` | `return NULL;` | returns `NULL` |
| 1384 | `ZSTDv07_createDDict_advanced` <br/>`src/legacy/zstd_v07.c:4159` | `return NULL;` | returns `NULL` |
| 1385 | `ZBUFFv07_createDCtx_advanced` <br/>`src/legacy/zstd_v07.c:4293` | `return NULL;` | returns `NULL` |
| 1386 | `ZBUFFv07_createDCtx_advanced` <br/>`src/legacy/zstd_v07.c:4296` | `if (zbd==NULL) return NULL;` | returns `NULL` |
| 1387 | `ZBUFFv07_createDCtx_advanced` <br/>`src/legacy/zstd_v07.c:4300` | `if (zbd->zd == NULL) { ZBUFFv07_freeDCtx(zbd); return NULL; }` | returns `NULL` |
| 1388 | `ZBUFFv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4360` | `return ERROR(init_missing);` | `ZSTD_error_init_missing` |
| 1389 | `ZBUFFv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4397` | `if (zbd->inBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1390 | `ZBUFFv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4404` | `if (zbd->outBuff == NULL) return ERROR(memory_allocation);` | `ZSTD_error_memory_allocation` |
| 1391 | `ZBUFFv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4436` | `if (toLoad > zbd->inBuffSize - zbd->inPos) return ERROR(corruption_detected);   /* should never happen */` | `ZSTD_error_corruption_detected` |
| 1392 | `ZBUFFv07_decompressContinue` <br/>`src/legacy/zstd_v07.c:4472` | `default: return ERROR(GENERIC);   /* impossible */` | `ZSTD_error_GENERIC` |

---

## Appendix — test inventory (generated from `tests/`)

Every function below was executed with `cargo test --release` and passed,
under BOTH the default feature set and `--no-default-features`.
Reproduce with `./run_all.sh`.

| test file | # `#[test]` fns | lines | `both::<T>()` call sites |
|-----------|-----------------|-------|---------------------------|
| `b10_entropy.rs` | 8 | 1080 | 62 |
| `b11_xxhash.rs` | 14 | 576 | 61 |
| `b12_misc_exports.rs` | 23 | 1499 | 131 |
| `b13_dictbuilder.rs` | 11 | 996 | 36 |
| `b14_deprecated.rs` | 7 | 657 | 52 |
| `b15_legacy.rs` | 9 | 636 | 37 |
| `b16_internals.rs` | 33 | 2755 | 110 |
| `b17_ldm_superblock.rs` | 6 | 1201 | 46 |
| `b1_simple_api.rs` | 8 | 369 | 30 |
| `b4_compress2_configs.rs` | 12 | 528 | 12 |
| `b5_stream.rs` | 8 | 788 | 42 |
| `b6_blocklevel.rs` | 12 | 1102 | 75 |
| `b7_dict.rs` | 8 | 1288 | 101 |
| `b8_estimates.rs` | 11 | 811 | 81 |
| `b9_sequences.rs` | 7 | 709 | 38 |
| `c10_entropy.rs` | 8 | 713 | 31 |
| `c11_dictbuilder.rs` | 9 | 687 | 14 |
| `c12_deprecated.rs` | 10 | 695 | 38 |
| `c13_legacy.rs` | 7 | 985 | 59 |
| `c14_pool.rs` | 7 | 379 | 24 |
| `c15_errorapi.rs` | 13 | 399 | 15 |
| `c16_enums.rs` | 6 | 450 | 59 |
| `c1_params.rs` | 8 | 596 | 46 |
| `c3_decompress.rs` | 8 | 416 | 46 |
| `c4_corruption.rs` | 6 | 403 | 16 |
| `c5_dict.rs` | 10 | 763 | 91 |
| `c6_stream.rs` | 8 | 648 | 29 |
| `c7_blocklevel.rs` | 12 | 879 | 48 |
| `c8_alloc.rs` | 12 | 720 | 37 |
| `c9_sequences.rs` | 10 | 810 | 33 |
| **total** | **311** | | |

### Test functions per file

**`b10_entropy.rs`**

- `hist_count_all_variants`
- `fse_optimal_table_log`
- `fse_ncount_write_bound`
- `fse_normalize_count_differential`
- `fse_writeread_build_compress_decompress`
- `huf_compress_bound_and_cardinality`
- `huf_build_write_read_ctable`
- `huf_compress_decompress_roundtrip`

**`b11_xxhash.rs`**

- `xxh32_oneshot_all_shapes_lens_seeds`
- `xxh64_oneshot_all_shapes_lens_seeds`
- `xxh32_streaming_equals_oneshot_and_agrees`
- `xxh64_streaming_equals_oneshot_and_agrees`
- `xxh32_streaming_seeded`
- `xxh64_streaming_seeded`
- `xxh32_copystate_then_continue`
- `xxh64_copystate_then_continue`
- `xxh32_canonical_roundtrip`
- `xxh64_canonical_roundtrip`
- `xxh_null_and_zero_length`
- `xxh32_raw_state_memcmp_sequences`
- `xxh64_raw_state_memcmp_sequences`
- `xxh_version_number`

**`b12_misc_exports.rs`**

- `g1_compress_advanced`
- `g1_compress_advanced_internal`
- `g1_compress_usingCDict_advanced`
- `g1_begin_continue_end_public`
- `g1_block_deprecated_roundtrip`
- `g1_setParametersUsingCCtxParams`
- `g1_stream2_simpleArgs`
- `g1_resetCStream_and_initCStream_internal`
- `g1_DCtx_setFormat`
- `g1_CCtx_refThreadPool_null`
- `g1_CCtxParams_registerSequenceProducer`
- `g1_CCtx_trace`
- `g2_cycleLog`
- `g2_getcBlockSize`
- `g2_writeLastEmptyBlock`
- `g2_header_and_size_helpers`
- `g3_ddict_accessors`
- `g3_getCParams_from_cdict_and_cctxparams`
- `g4_zstdmt_single_thread_fallback`
- `g5_zbuff_recommended_sizes`
- `g5_fse_version_number`
- `g5_fse_read_ncount`
- `g6_exported_data_symbols`

**`b13_dictbuilder.rs`**

- `train_from_buffer_simple`
- `train_from_buffer_cover`
- `optimize_train_cover`
- `train_from_buffer_fastcover`
- `optimize_train_fastcover`
- `train_from_buffer_legacy`
- `finalize_and_add_entropy`
- `cover_pure_helpers`
- `cover_selection_and_best`
- `cover_check_and_select`
- `divsufsort_and_divbwt`

**`b14_deprecated.rs`**

- `zbuff_recommended_sizes`
- `zbuff_create_free_lifecycle`
- `zbuff_compress_decompress_roundtrip`
- `zbuff_compress_flush_interleaved`
- `zbuff_dictionary_roundtrip`
- `zbuff_compress_init_advanced_roundtrip`
- `zbuff_interop_with_modern_api`

**`b15_legacy.rs`**

- `oneshot_decoders_magic_prefixed`
- `dctx_decoders_magic_prefixed`
- `find_frame_size_info_legacy`
- `streaming_legacy_entry_points`
- `zbuff_legacy_decoders`
- `legacy_magic_into_modern_api`
- `legacy_magic_into_modern_stream`
- `modern_frames_into_legacy_decoders`
- `zbuff_legacy_recommended_and_advanced`

**`b16_internals.rs`**

- `select_block_compressor_all_combos`
- `ldm_table_size_and_max_nb_seq`
- `ldm_adjust_parameters`
- `literals_no_compress`
- `literals_rle`
- `literals_compress`
- `seq_to_codes`
- `select_encoding_type`
- `build_ctable_encode_and_costs`
- `get1_block_summary`
- `cctx_layout_selfcheck`
- `reset_compressed_block_state`
- `invalidate_rep_codes`
- `block_path_all_strategies`
- `build_fse_table`
- `decompress_getc_block_size`
- `decompress_block_roundtrip_and_internal`
- `decode_seq_headers_and_literals_garbage`
- `split_block`
- `ldm_skip_sequences`
- `ldm_skip_raw_seq_store_bytes`
- `ldm_data_path_via_compress2`
- `encode_sequences_real`
- `convert_block_sequences`
- `reference_external_sequences`
- `build_block_entropy_stats`
- `compress_super_block_via_compress2`
- `load_entropy_garbage`
- `load_entropy_real_dictionary`
- `block_compressor_dict_variants_via_compress2`
- `fill_hash_table_direct`
- `insert_row_update_tree_direct`
- `check_continuity_direct`

**`b17_ldm_superblock.rs`**

- `cctx_layout_selfcheck`
- `ldm_fill_hash_table_direct`
- `ldm_generate_sequences_direct`
- `ldm_block_compress_direct`
- `compress_super_block_direct`
- `dds_lazy_load_dictionary_direct`

**`b1_simple_api.rs`**

- `version_and_static_info`
- `compress_bound_and_decompress_bound`
- `oneshot_compress_all_levels_shapes`
- `oneshot_random_property_sweep`
- `oneshot_tight_dst_buffers`
- `frame_info_functions`
- `magic_and_misc_predicates`
- `read_skippable_frame`

**`b4_compress2_configs.rs`**

- `strategy_sweep`
- `strategy_x_row_match_finder`
- `window_and_table_logs`
- `min_match_x_strategy`
- `target_length_sweep`
- `long_distance_matching`
- `frame_flag_combinations`
- `magicless_format`
- `block_size_params`
- `block_splitter`
- `remaining_experimental_params`
- `random_multi_param_sweep`

**`b5_stream.rs`**

- `compress_stream2_chunk_matrix`
- `stable_buffers`
- `legacy_compress_stream_triple`
- `init_stream_families`
- `decompress_stream_chunk_matrix`
- `decompress_params`
- `reset_directives_mid_stream`
- `stream_random_property_sweep`

**`b6_blocklevel.rs`**

- `begin_level_low`
- `begin_level_mid`
- `begin_level_high`
- `begin_all_chunk_sizes`
- `begin_using_dict_and_advanced`
- `begin_using_cdict`
- `copy_cctx`
- `decompress_bufferless_loop`
- `decompress_bufferless_with_dict`
- `copy_dctx`
- `raw_block_roundtrip`
- `get_block_size_matches`

**`b7_dict.rs`**

- `cdict_ddict_oneshot_matrix`
- `using_dict_oneshot_matrix`
- `cdict_ddict_advanced_create_matrix`
- `cctx_loaddict_and_param_sweep`
- `ref_cdict_ddict_prefix_and_multiddict`
- `compress_decompress_begin_using_dict`
- `init_cstream_dstream_using_dict`
- `static_cdict_ddict_workspace_sizes`

**`b8_estimates.rs`**

- `custom_alloc_helpers_are_not_exported`
- `estimate_by_level`
- `estimate_dict_sizes`
- `estimate_using_cparams`
- `estimate_using_cctxparams`
- `estimate_from_real_frames`
- `sizeof_lifecycle`
- `static_cctx_dctx`
- `static_cstream_dstream`
- `static_cdict_ddict`
- `compress_decompress_bound`

**`b9_sequences.rs`**

- `seq_bound_all`
- `generate_sequences_all_shapes`
- `merge_block_delimiters`
- `compress_sequences_cross_product`
- `compress_sequences_dst_capacity_sweep`
- `compress_sequences_and_literals_cross_product`
- `compress_sequences_and_literals_dst_capacity_sweep`

**`c10_entropy.rs`**

- `hist_error_and_edge`
- `fse_normalize_write_errors`
- `fse_build_table_workspace_too_small`
- `fse_read_ncount_corrupted`
- `fse_decompress_wksp_errors`
- `huf_build_write_errors`
- `huf_read_stats_corrupted`
- `huf_read_dtable_and_decode_garbage`

**`c11_dictbuilder.rs`**

- `simple_api_errors`
- `cover_errors`
- `optimize_cover_errors`
- `fastcover_errors`
- `optimize_fastcover_errors`
- `legacy_errors`
- `finalize_errors`
- `header_and_id_on_malformed`
- `error_name_full_range`

**`c12_deprecated.rs`**

- `compress_calls_without_init`
- `decompress_continue_without_init`
- `tiny_dst_capacity_streaming`
- `decompress_single_byte_mutation_sweep`
- `decompress_truncation_sweep`
- `decompress_random_garbage`
- `compress_init_bad_levels`
- `init_dictionary_error_paths`
- `error_name_full_range`
- `recommended_sizes_and_advanced_ctx`

**`c13_legacy.rs`**

- `oneshot_single_byte_mutation_sweep`
- `oneshot_truncation_sweep`
- `oneshot_random_garbage`
- `advanced_dctx_entry_points`
- `fse_huf_error_and_decoders`
- `fse_dtable_functions`
- `huf_dtable_functions`

**`c14_pool.rs`**

- `pool_create_nullness_and_sizeof`
- `pool_resize_return_codes`
- `pool_free_null_and_joinjobs_fresh`
- `pool_add_and_tryadd_invocation_counts`
- `pool_create_advanced_counting_allocator`
- `pool_create_advanced_null_allocator`
- `pool_create_advanced_default_cmem_jobs`

**`c15_errorapi.rs`**

- `zstd_iserror_getcode_getname_exhaustive`
- `zstd_get_error_string_int_sweep`
- `err_get_error_string_int_sweep`
- `zstd_and_err_string_cross_consistency`
- `version_number_and_string`
- `zdict_error_api`
- `fse_error_api`
- `huf_error_api`
- `zbuff_error_api`
- `zstd_legacy_version_error_api`
- `zbuffv04_error_api`
- `fse_huf_zbuff_legacy_error_api`
- `zstd_custom_alloc_api`

**`c16_enums.rs`**

- `parameter_enum_ids`
- `enum_valued_parameters`
- `reset_and_end_directives`
- `dict_and_format_enums`
- `error_code_enum`
- `strategy_in_struct`

**`c1_params.rs`**

- `bounds_all_and_out_of_range_enums`
- `cctx_set_get_parameter_full_sweep`
- `cctxparams_full_sweep`
- `cparams_derivation_and_check`
- `cctx_set_cparams_fparams_params`
- `stage_and_reset_directives`
- `dctx_parameter_sweep_and_stage`
- `pledged_src_size`

**`c3_decompress.rs`**

- `null_and_zero_dst`
- `null_src_zero_size`
- `empty_src`
- `bad_magic`
- `truncated_frames`
- `dst_too_small`
- `multi_frame_and_trailing_garbage`
- `window_too_large`

**`c4_corruption.rs`**

- `exhaustive_bit_sweep_small_frames`
- `checksum_mutation`
- `header_bit_sweep`
- `block_mutation`
- `randomized_multibyte_corruption`
- `random_body_with_valid_header`

**`c5_dict.rs`**

- `nonzero_size_null_dict`
- `fulldict_without_magic`
- `out_of_range_enum_values`
- `corrupted_trained_dict_byte_sweep`
- `truncated_dict_lengths`
- `decompress_no_dict_and_wrong_dict`
- `create_bad_level_and_null_prefix`
- `load_dictionary_mid_frame`
- `getdictid_fuzz`
- `compress_using_dict_bad_level_and_null`

**`c6_stream.rs`**

- `bad_end_directive`
- `bad_buffer_positions`
- `stable_in_violation`
- `stable_out_violation`
- `over_and_under_pledge`
- `no_forward_progress`
- `flush_without_init_and_bad_init`
- `decompress_stream_truncation_and_mutation`

**`c7_blocklevel.rs`**

- `continue_end_without_begin`
- `continue_srcsize_too_large`
- `continue_end_tight_dst`
- `begin_invalid_levels_and_params`
- `begin_dict_null_conditions`
- `compress_block_errors`
- `copy_cctx_wrong_stage`
- `decompress_continue_wrong_srcsize`
- `decompress_continue_corruption_sweep`
- `decompress_block_errors`
- `insert_block_sizes`
- `next_src_size_and_input_type_states`

**`c8_alloc.rs`**

- `custom_alloc_helpers_are_not_exported`
- `counting_allocator_matches_per_constructor`
- `counting_allocator_compress_decompress_cycle`
- `always_null_allocator_all_ctors_return_null`
- `fail_after_n_allocations_matches`
- `fail_after_n_compress_error_codes_match`
- `half_set_customMem_invalid_combo`
- `init_static_error_paths`
- `estimate_error_levels_and_cparams`
- `estimate_dstream_from_frame_errors`
- `decompression_margin_errors`
- `sizeof_null_pointer`

**`c9_sequences.rs`**

- `invalid_sequence_conditions`
- `block_delimiter_placement_errors`
- `count_and_null_pointer_errors`
- `dst_capacity_boundary_errors`
- `generate_sequences_capacity_errors`
- `random_sequences_bd0_val0`
- `random_sequences_bd0_val1`
- `random_sequences_bd1_val0`
- `random_sequences_bd1_val1`
- `register_sequence_producer_errors`

