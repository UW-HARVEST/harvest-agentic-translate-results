# CONFIGS.md — the CONFIGURATION-SURFACE TABLE (valid inputs)

Mirror of `ERRORS.md`, for **valid** inputs. Derived mechanically from the
public headers (`c_src/src/include/zstd.h`, `zdict.h`, `c_src/src/common/{fse,huf}.h`,
`c_src/src/common/xxhash.h`, `c_src/src/deprecated/zbuff.h`,
`c_src/src/legacy/zstd_v0*.h`, `c_src/src/compress/zstdmt_compress.h`) plus the
`if`/`switch` branches the C actually takes on those settings.

Every row is exercised by a differential test that loads **both** `.so` files
through `libloading` and compares byte-for-byte, with **many randomized inputs
per row** (fixed seed, see `tests/common/mod.rs::Rng`).

## Axes the C code branches on

### Compression-parameter axes (`ZSTD_c_*`, `ZSTD_CCtx_setParameter`)

| axis | values the C distinguishes | where it branches |
|---|---|---|
| `compressionLevel` | `ZSTD_minCLevel()`(-131072), -5..-1 (negative = "fast" table), 0 (=default 3), 1..19, 20..22 (ultra) | `ZSTD_getCParams_internal`, `ZSTD_dedicatedDictSearch_*` |
| `strategy` | `fast`1 `dfast`2 `greedy`3 `lazy`4 `lazy2`5 `btlazy2`6 `btopt`7 `btultra`8 `btultra2`9 | `ZSTD_selectBlockCompressor` (9-way × 4 dictMode × rowMatchFinder) |
| `windowLog` | 10..31 (`ZSTD_WINDOWLOG_MIN..MAX`) | window/`extDict`/overflow-correction paths |
| `hashLog`,`chainLog`,`searchLog` | min..max per `ZSTD_cParam_getBounds` | table sizing, `ZSTD_cwksp` |
| `minMatch` | 3..7 (`ZSTD_MINMATCH_MIN..MAX`) | `ZSTD_count`, `mls` switch in every block compressor |
| `targetLength` | 0..131072 | `btopt`/`btultra`, `ZSTD_compressBlock_fast` |
| `targetCBlockSize` | 0 (off), 1340 (`ZSTD_TARGETCBLOCKSIZE_MIN`) .. 131072 | `ZSTD_compressSuperBlock` path |
| `maxBlockSize` | 0 (=128KB), 1024..131072 | `ZSTD_blockSizeMax` |
| `contentSizeFlag` | 0,1 | frame header `FCS` field size (0/1/2/4/8 bytes) |
| `checksumFlag` | 0,1 | XXH64 trailer + `ZSTD_error_checksum_wrong` |
| `dictIDFlag` | 0,1 | frame header DID field |
| `nbWorkers` | 0 (only value the non-MT build accepts) | `ZSTD_c_nbWorkers` |
| `enableLongDistanceMatching` | auto0/1/2 (`ZSTD_ps_auto/enable/disable`) | `ZSTD_ldm_*` |
| `ldmHashLog`,`ldmMinMatch`,`ldmBucketSizeLog`,`ldmHashRateLog` | bounds per `getBounds` | `ZSTD_ldm_adjustParameters` |
| `format` (exp2) | `ZSTD_f_zstd1`0, `ZSTD_f_zstd1_magicless`1 | frame header write/read |
| `forceMaxWindow` (exp3) | 0,1 | `ZSTD_d_windowLogMax` interaction |
| `forceAttachDict` (exp4) | `ZSTD_dictDefaultAttach`0, `ForceAttach`1, `ForceCopy`2, `ForceLoad`3 | `ZSTD_resetCCtx_usingCDict` 3-way |
| `literalCompressionMode` (exp5) | auto0/huffman1/uncompressed2 | `ZSTD_compressLiterals` |
| `srcSizeHint` (exp7) | 0, small, large | `ZSTD_getCParamsFromCCtxParams` |
| `enableDedicatedDictSearch` (exp8) | 0,1 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` |
| `stableInBuffer` (exp9) | 0,1 | `ZSTD_compressStream2` buffer bypass |
| `stableOutBuffer` (exp10) | 0,1 | `ZSTD_compressStream2` buffer bypass |
| `blockDelimiters` (exp11) | `ZSTD_sf_noBlockDelimiters`0, `ZSTD_sf_explicitBlockDelimiters`1 | `ZSTD_compressSequences` |
| `validateSequences` (exp12) | 0,1 | `ZSTD_validateSequence` |
| `splitAfterSequences` (exp13) | auto0/enable1/disable2 | `ZSTD_preSplitBlock` |
| `useRowMatchFinder` (exp14) | auto0/enable1/disable2 | `ZSTD_selectBlockCompressor` row variants |
| `deterministicRefPrefix` (exp15) | 0,1 | `ZSTD_CCtx_refPrefix` window handling |
| `prefetchCDictTables` (exp16) | auto0/enable1/disable2 | `ZSTD_resetCCtx_byAttachingCDict` |
| `enableSeqProducerFallback` (exp17) | 0,1 | external sequence producer |
| `repcodeResolution` (exp19) | auto0/enable1/disable2 | `ZSTD_compressSequences` |
| `blockSplitterLevel` (exp20) | 0..6 (`ZSTD_BLOCKSPLITTER_LEVEL_MAX`) | `ZSTD_splitBlock` |
| `rsyncable` (exp1) | 0,1 | MT-only, still a settable/gettable param |
| `overlapLog`,`jobSize` | bounds per `getBounds` | MT params, still settable |

### Decompression-parameter axes (`ZSTD_d_*`)

| axis | values | branch |
|---|---|---|
| `windowLogMax` | 10..31 | `ZSTD_error_frameParameter_windowTooLarge` |
| `format` (exp1) | `ZSTD_f_zstd1`, `ZSTD_f_zstd1_magicless` | `ZSTD_getFrameHeader_advanced` |
| `stableOutBuffer` (exp2) | 0,1 | `ZSTD_decompressStream` output bypass |
| `forceIgnoreChecksum` (exp3) | `ZSTD_d_validateChecksum`0, `ZSTD_d_ignoreChecksum`1 | checksum verify |
| `refMultipleDDicts` (exp4) | `ZSTD_rmd_refSingleDDict`0, `ZSTD_rmd_refMultipleDDicts`1 | DDict hash set |
| `disableHuffmanAssembly` (exp5) | 0,1 | `HUF_decompress` (C build has no asm → both identical) |
| `maxBlockSize` (exp6) | 0, 1024..131072 | block size validation |

### Dictionary axes

| axis | values | branch |
|---|---|---|
| entry point | `ZSTD_CCtx_loadDictionary` / `_byReference` / `_advanced`, `ZSTD_CCtx_refPrefix(_advanced)`, `ZSTD_CCtx_refCDict`, `ZSTD_compress_usingDict`, `ZSTD_compress_usingCDict(_advanced)`, `ZSTD_compressBegin_usingDict/usingCDict(_advanced)`, `ZSTD_initCStream_usingDict/usingCDict(_advanced)` | — |
| `ZSTD_dictLoadMethod_e` | `byCopy`0, `byRef`1 | `ZSTD_CDict`/`ZSTD_DDict` init |
| `ZSTD_dictContentType_e` | `dct_auto`0, `dct_rawContent`1, `dct_fullDict`2 | `ZSTD_loadCEntropy`/`ZSTD_loadDEntropy` |
| dict shape | none, raw random bytes, real trained dict (`ZDICT_trainFromBuffer`), dict with wrong magic, tiny dict (<8B) | — |
| decode side | `ZSTD_DCtx_loadDictionary(_byReference/_advanced)`, `ZSTD_DCtx_refDDict`, `ZSTD_DCtx_refPrefix(_advanced)`, `ZSTD_decompress_usingDict`, `ZSTD_decompress_usingDDict`, `ZSTD_initDStream_usingDict/usingDDict`, `ZSTD_decompressBegin_usingDict/usingDDict` | — |

### Input-shape axes

`empty(0)`, `1`, `7`, `128`, `1KB`, `8KB`, `64KB`, `128KB-1`, `128KB`, `128KB+1`
(block boundary), `256KB`, `1MB`, `>window` — crossed with content classes:
`zeros`, `single repeated byte`, `2-byte pattern`, `incompressible random`,
`text-like (low-entropy alphabet)`, `long-range duplicated blocks (LDM bait)`,
`RLE-able runs`, `already-compressed data`.

### Streaming-shape axes

input chunk size ∈ {1, 3, 17, 1KB, `ZSTD_CStreamInSize()`, all-at-once},
output chunk size ∈ {1, 3, 17, 1KB, `ZSTD_CStreamOutSize()`, oversized},
`ZSTD_EndDirective` ∈ {`continue`0, `flush`1, `end`2} in every interleaving.


## Upstream C crashes (configurations excluded from the differential suite)

A few configurations make the **C** library itself die, so no differential
comparison is possible (the process is gone before a value can be returned).
They are recorded here rather than silently dropped.

| # | entry point | configuration | what the C does |
|---|-------------|---------------|-----------------|
| X1 | `ZSTD_estimateCCtxSize_usingCCtxParams` / `ZSTD_estimateCStreamSize_usingCCtxParams` | a bare `ZSTD_CCtx_params` with `ZSTD_c_enableLongDistanceMatching = ZSTD_ps_enable` and `ZSTD_c_ldmMinMatch` left at its default 0 | reaches `ZSTD_ldm_getMaxNbSeq()` -> `maxChunkSize / params.minMatchLength` with `minMatchLength == 0` (`ZSTD_ldm_adjustParameters()` has not run yet) and dies with **SIGFPE** (integer divide by zero). `translation/src/compress/zstd_ldm.rs::ZSTD_ldm_getMaxNbSeq` transliterates the same division, so the Rust also aborts. Not differentiable; the tests always set `ZSTD_c_ldmMinMatch` when they enable LDM on a params object. |
| X2 | `ZSTD_compressSequences` | any valid `(inSeqs, src)` pair with `dstCapacity` smaller than the frame needs (measured with `dstCapacity` = 10 while 18 bytes were needed) | writes **70 bytes *below* `dst`** and *then* returns `ZSTD_error_dstSize_tooSmall`. The Rust port reproduces the same pointer arithmetic and scribbles the same bytes in the same place. The test keeps this row differential by giving both destinations a 64 KiB canary guard band on each side and comparing the whole padded region (`tests/phase_b_sequences.rs::row122_compress_sequences`). |
| X3 | `HUFv07_selectDecoder` | `dstSize == 0` | computes `cSrcSize * 16 / dstSize` and dies with **SIGFPE**. Documented contract is `0 < cSrcSize < dstSize <= 128 KB`; outside it `algoTime[Q]` is also indexed out of bounds. `tests/phase_b_legacy.rs` restricts the domain accordingly. |
| X4 | `ZSTDv07_freeDDict` | `ddict == NULL` | dereferences `ddict->refContext->customMem` without a NULL check and **segfaults**. Unlike `ZSTDv07_freeDCtx`, NULL is not supported. Not exercised. |
| X7 | `ZSTD_compress2` with `ZSTD_c_targetCBlockSize` set | e.g. `targetCBlockSize = 1340`, text-like input of 1300 bytes, `dstCapacity = 792` | the super-block writer **writes past `dst + dstCapacity`**; the C itself notes it is "not bound by the standard `ZSTD_compressBound()`" (`zstd_compress.c:4470-4479`). An unguarded heap destination is corrupted and the process dies on the next allocation. `tests/phase_c_compress.rs::err_superblock_dst_too_small` keeps the row differential with a 64 KiB canary band on each side of both destinations, compares the whole padded region (identical in C and Rust) and asserts the OOB write still reproduces. |
| X8 | `ZSTD_compressStream2` with `ZSTD_c_stableInBuffer = 1`, then `ZSTD_CCtx_reset`, then a new session | any | `ZSTD_compressStream_generic` parks unconsumed stable input in `zcs->stableIn_notConsumed` (`zstd_compress.c:6185`) but `ZSTD_CCtx_reset` never clears it (`zstd_compress.c:1368-1381`). The next session runs `input->pos -= stableIn_notConsumed; ip -= stableIn_notConsumed;` (`zstd_compress.c:6120-6122`) on a fresh `pos == 0`, underflows, and reads **before** the caller's buffer (measured `base - 8928` after a 140 000-byte stable session). The guarding `assert` is compiled out (`DEBUGLEVEL` unset). Output then depends on heap history, so the tests always create a brand-new `ZSTD_CCtx` for stable-buffer sessions. The same underflow is reachable within one session by *shrinking* a stable input buffer after a sub-block-size `ZSTD_e_continue`. |
| X9 | `FSE_normalizeCount` | `(U32)total == 0`, e.g. `total == 2^32` | `ZSTD_div64(..., (U32)total)` divides by zero -> **SIGFPE** in both libraries (verified). |
| X10 | `HUF_buildCTable_wksp` | `maxNbBits` in 1..=4 with a tree deeper than that | `HUF_setMaxHeight` falls through to `huffNode[rankLast[13]]` where `rankLast[13] == 0xF0F0F0F0` (`huf_compress.c:443`) -> **verified SIGSEGV**. Also `maxNbBits > HUF_TABLELOG_MAX+1` with a deeper tree writes past `U32 rankLast[HUF_TABLELOG_MAX+2]` (L411/L420). |
| X11 | `HUF_buildCTable_wksp` | an all-zero `count[]` (cardinality 0) | `nonNullRank == -1` and `HUF_buildCTableFromTree` walks off `huffNode` -> **verified SIGSEGV**. |
| X12 | `ZDICT_addEntropyTablesFromBuffer` | a `dictContentSize` whose low 32 bits + 128 KB do **not** set bit 31 (e.g. `0xFFFF_FFFF`, `1<<40`) | passes the `offcodeMax > 30` guard and then `XXH64`s `dictContentSize` bytes out of bounds -> SIGSEGV in both libraries identically. |
| X13 | `COVER_map_init` | `k - d + 1 >= 2^30` (needs a >= 1 GiB `dictBufferCapacity`) | `sizeLog = ZSTD_highbit32(size) + 2` then `(U32)1 << sizeLog` shifts by >= 32 (C UB; x86 masks the count and Rust release-mode masks identically). Not exercised. |
| X6 | `COVER_computeEpochs` | `nbDmers == 0` | `epochs.size = MIN(k*10, nbDmers) == 0`, then `nbDmers / epochs.size` -> **SIGFPE**. COVER/FASTCOVER never call it that way (`*_ctx_init` guarantees >= 1 dmer). `tests/phase_b_dictbuilder.rs` excludes it. |
| X5 | `ZSTDv07_freeDCtx` after `ZSTDv07_decompress_usingDDict` / `_usingPreparedDCtx` on a context built by `ZSTDv07_createDCtx_advanced(customMem)` | any | `ZSTDv07_copyDCtx()` memcpys the *reference* context over `dctx`, including its `customMem`, so the context is later released with the reference's `free` instead of the caller's — glibc aborts with `free(): invalid pointer`. The tests use a default-allocated context for the DDict path. |

## C preconditions that are UB when violated (excluded)

| # | entry point | precondition | what the C does when violated |
|---|-------------|--------------|-------------------------------|
| P1 | `ZSTD_selectBlockCompressor` | `zstd_compress.c`: *"assumption : strat is a valid strategy"*; `dictMode` in `[0,3]` | indexes `blockCompressor[dictMode][strat]` / `rowBasedBlockCompressors[dictMode][strat-ZSTD_greedy]` with no validation -> out-of-bounds *read*, silently returning a garbage function pointer. Rust's bounds-checked indexing panics instead. Precondition violation, not a translation difference; `tests/phase_c_params.rs` probes only the documented domain. |
| P2 | `ZSTD_CCtxParams_setParameter`, `ZSTD_CCtxParams_getParameter` | non-NULL params object | dereferences `CCtxParams` immediately (`zstd_compress.c:770`) with no NULL check, unlike `ZSTD_CCtxParams_init` which does `RETURN_ERROR_IF(!cctxParams, GENERIC, ...)`. NULL segfaults the C. Excluded; the checked `_init` / `_init_advanced` NULL paths *are* tested. |
| P4 | every compressor taking `(src, srcSize)` (`ZSTD_compress`, `ZSTD_compressCCtx`, `ZSTD_compress2`, `ZSTD_compressStream*`, ...) | `srcSize` must not exceed the buffer `src` actually points at | there is no `srcSize` sanity guard: the C reads `MIN(srcSize, blockSize)` bytes out of `src` before any validation, so an over-stated `srcSize` is a large out-of-bounds read (observed as an intermittent SIGSEGV depending on heap layout). Excluded; the over-range axis is probed on the pure size helpers instead (`tests/phase_c_nulls.rs::oversized_lengths`). |
| P5 | `ZSTD_DDict_dictContent`, `ZSTD_DDict_dictSize` | non-NULL `ddict` | guarded only by `assert(ddict != NULL)`, which this build compiles to `((void)0)` (`DEBUGLEVEL` 0), so NULL is dereferenced. The dictID accessors *do* check (`if (ddict==NULL) return 0;`) and are tested. |
| P6 | `ZSTD_getFrameHeader`, `ZSTD_getFrameHeader_advanced` | non-NULL `zfhPtr` | `zfhPtr->...` is written unconditionally once the header parses. Only the `(src, srcSize)` axis is probed. |
| P7 | `ZSTD_compressRleLiteralsBlock` | `srcSize > 0` | dereferences `src[0]` unconditionally; its header documents "all bytes in `src` are identical" and `dstCapacity >= 4` but not `srcSize > 0`, so `(NULL, 0)` segfaults. Guarded in the tests. |
| P8 | `FSE_optimalTableLog`, `FSE_optimalTableLog_internal`, `FSE_normalizeCount` | `srcSize >= 2` and `maxSymbolValue >= 1` | both feed `ZSTD_highbit32(0)` (`bsr` on zero) otherwise; the C carries `assert(srcSize > 1)` at `fse_compress.c:351/362`, compiled out here. Observed C=5 vs Rust=11 for `(0,1,0,0)`. Excluded. |
| P9 | `HIST_countFast`, `HIST_countFast_wksp`, `HIST_count_simple` | every byte of `src` must be `<= maxSymbolValue` | documented "unsafe : won't check if src contains values beyond count[] limit" (`hist.c:104/172`); `while (!count[maxSymbolValue]) maxSymbolValue--` underflows. Only alphabet-conforming input is used; the *checked* `HIST_count`/`HIST_count_wksp` still get raw data and their `maxSymbolValue_tooSmall` path is compared. |
| P10 | `HUF_selectDecoder` | `dstSize >= 1` | computes `cSrcSize * 16 / dstSize` -> SIGFPE at 0 (both public callers guard it). |
| P11 | `HUF_*` compressors | explicit `tableLog >= HUF_minTableLog(cardinality)` | `HUF_setMaxHeight` gets an impossible target and walks off its table. zstd only ever passes `HUF_TABLELOG_DEFAULT`. |
| P12 | `ZSTD_buildFSETable` | `tableLog >= 1` | `1 << (tableLog-1)` underflows at 0. |
| P13 | `XXH32_reset`, `XXH64_reset` | non-NULL state | guarded only by `XXH_ASSERT(statePtr != NULL)` (`xxhash.h:3116`), then `memset(statePtr, 0, ...)`. `XXH*_update(state, NULL, len)` *is* defined for any `len` and is tested. |
| P3 | `ZSTD_decompressBlock`, `ZSTD_decompressBlock_deprecated` | `zstd.h`: the block API is *"not protected against malicious input"* | fully random block payloads can drive out-of-bounds accesses. `tests/phase_c_decompress.rs` restricts the block-level fuzz accordingly. |

## Unspecified C values (fields excluded from comparison)

| # | entry point | what is unspecified |
|---|-------------|---------------------|
| U2 | any decoder entry point that returns an error mid-frame (`ZSTD_decompress`, `ZSTD_decompressDCtx`, `ZSTD_decompressStream`, `ZSTD_decompressContinue`) | `ZSTD_execSequence()` performs an unconditional 16-byte `ZSTD_copy16(op, *litPtr)` even when `litLength < 16`. For a frame whose literals were staged into `dctx->litBuffer`, up to 14 bytes of **never-written `litBuffer`** are copied into `dst` before the following offset check returns `corruption_detected`. With a heap-allocated DCtx those bytes are whatever `malloc` last left there, so they differ between the two libraries purely because of allocation history. **Verified not a translation difference**: driven through `ZSTD_initStaticDCtx` / `ZSTD_initStaticDStream` over a workspace pre-filled with 0x11 / 0x22 / 0x00, the C and the Rust `.so` emit byte-identical output (the fill byte) in every case. `tests/phase_c_decompress.rs` therefore runs every context-based decode on a static workspace with the same fill in both libraries, which makes the error-path partial output fully specified and comparable; only the self-allocating `ZSTD_decompress()` skips the `dst` comparison on the error path. |
| U1 | `ZSTD_get1BlockSummary` | on its error path the C does `BlockSummary bs; bs.nbSequences = ERROR(externalSequences_invalid); return bs;` (`zstd_compress.c:7462`) and returns `bs.blockSize` / `bs.litSize` **uninitialised**. Only `nbSequences` is observable; the Rust zero-initialises the other two fields. |


## Suite sensitivity (mutation testing)

A differential suite is only worth its checkboxes if it actually fails when the
Rust diverges. Measured by temporarily mutating
`src/compress/zstd_compress_internal.rs::ZSTD_hash4` (the 4-byte match-finder
hash, reached by `minMatch` 3 and 4 at every strategy) and re-running the whole
suite:

| mutation | tests that failed |
|---|---|
| `hash4(u,h,s) -> 0` (constant) | **38+ tests across 5 test files** — `phase_a_smoke::smoke_roundtrip`; 9/16 of `phase_b_bufferless`; 7/24 of `phase_b_core`; 13/20 of `phase_b_dict`; 9/12 of `phase_b_dictbuilder`; ... |
| `hash4(...) ^ 0x80000000` | **0 tests** — and correctly so: XOR-ing a hash *output* by a constant is a bijection on the index space, so it permutes hash-table slots without changing which positions collide. The match finder is therefore bit-for-bit unaffected. This is a *semantically neutral* edit, not a bug, and it is a trap to avoid when mutation-testing hash code. |
| `hash4(...) ^ 1` | 0 tests — the low bit is discarded by the `>> (32 - hashLog)` that follows. |

Reachability of the mutated code was confirmed independently with an exported
call counter (`ZSTD_hash4` is invoked ~56 000 times for a 60 KB input at
`strategy=fast, minMatch=4`, and 0 times for `minMatch >= 5`, which selects
`ZSTD_hash5`/`hash6`/`hash7`).

The neutral-mutation experiment also exposed a real gap in the original row 32 /
row 33 grids: they drew a *single* random `(size, content class)` per
`(strategy, minMatch, windowLog)` triple, so a triple could draw a 0- or 1-byte
input and skip the match finder entirely. Both rows now iterate the size ladder
and the content classes explicitly.

The two libraries are also proven to be genuinely distinct objects rather than
one `dlopen` alias: the C `.so` carries `SONAME libzstd.so` while the Rust
cdylib carries none, and
`tests/phase_b_streaming.rs::sanity_two_distinct_libraries` asserts that all 28
streaming symbols resolve to *different* addresses in the two handles.

---

## Rows

Legend: `[x]` = passes across randomized inputs (both `.so`s byte-identical).

### Group 1 — version / bounds / pure helpers (`tests/phase_b_core.rs`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|-------------------------------------------|-----|
| 1 | `ZSTD_versionNumber`, `ZSTD_versionString` | no input | [x] |
| 2 | `ZSTD_maxCLevel`, `ZSTD_minCLevel`, `ZSTD_defaultCLevel` | no input | [x] |
| 3 | `ZSTD_compressBound` | srcSize = 0,1,…,1MB, `ZSTD_MAX_INPUT_SIZE`, huge | [x] |
| 4 | `ZSTD_decompressBound` | valid 1..8-frame streams + truncated + garbage | [x] |
| 5 | `ZSTD_isError`, `ZSTD_getErrorCode`, `ZSTD_getErrorName`, `ZSTD_getErrorString` | every code 0..130 and every `(size_t)-n` | [x] |
| 6 | `ZSTD_cParam_getBounds` | all 20 public params + all 19 experimental params + invalid ints | [x] |
| 7 | `ZSTD_dParam_getBounds` | `d_windowLogMax` + exp1..exp6 + invalid ints | [x] |
| 8 | `ZSTD_getCParams` | level ∈ min..22 × srcSizeHint ∈ {0,unknown,1,1K,1M,1G} × dictSize ∈ {0,1K,1M} | [x] |
| 9 | `ZSTD_getParams` | same cross-product as row 8 | [x] |
| 10 | `ZSTD_adjustCParams` | all cParams grids × srcSize/dictSize grid | [x] |
| 11 | `ZSTD_checkCParams` | all cParams grids (valid + boundary) | [x] |
| 12 | `ZSTD_cycleLog` | hashLog 0..31 × all 9 strategies | [x] |
| 13 | `ZSTD_CStreamInSize`, `ZSTD_CStreamOutSize`, `ZSTD_DStreamInSize`, `ZSTD_DStreamOutSize` | no input | [x] |
| 14 | `ZSTD_sizeof_CCtx/CDict/CStream/DCtx/DDict/DStream` | fresh, after use, after dict load | [x] |
| 15 | `ZSTD_estimateCCtxSize`, `_usingCParams`, `_usingCCtxParams` | level grid × cParams grid | [x] |
| 16 | `ZSTD_estimateCStreamSize`, `_usingCParams`, `_usingCCtxParams` | level grid × cParams grid | [x] |
| 17 | `ZSTD_estimateDCtxSize`, `ZSTD_estimateDStreamSize`, `_fromFrame` | windowSize grid, real frames | [x] |
| 18 | `ZSTD_estimateCDictSize`, `_advanced` | dictSize grid × level × dictLoadMethod | [x] |
| 19 | `ZSTD_estimateDDictSize` | dictSize grid × dictLoadMethod | [x] |
| 20 | `ZSTD_getBlockSize`, `ZSTD_decodingBufferSize_min`, `ZSTD_decompressionMargin`, `ZSTD_sequenceBound` | windowSize/blockSize grid, real frames | [x] |

### Group 2 — one-shot compress / decompress (`tests/phase_b_core.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 21 | `ZSTD_compress` + `ZSTD_decompress` | level ∈ {min,-5..-1,0,1..22} × all 13 input sizes × all 8 content classes | [x] |
| 22 | `ZSTD_compressCCtx` + `ZSTD_decompressDCtx` | same grid, context reused across calls | [x] |
| 23 | `ZSTD_compress` | dstCapacity exactly `compressBound`, exactly the compressed size, one byte less | [x] |
| 24 | `ZSTD_compress2` (default params) | same grid as row 21 | [x] |
| 25 | `ZSTD_findFrameCompressedSize`, `ZSTD_findDecompressedSize`, `ZSTD_getFrameContentSize`, `ZSTD_getDecompressedSize` | 1..8 concatenated frames, with/without contentSize, with/without checksum, with skippable frames interleaved | [x] |
| 26 | `ZSTD_getFrameHeader`, `ZSTD_getFrameHeader_advanced` | every FCS field width (0/1/2/4/8), every DID width (0/1/2/4), single-segment, checksum on/off, magicless | [x] |
| 27 | `ZSTD_frameHeaderSize` | all header shapes of row 26 | [x] |
| 28 | `ZSTD_isFrame`, `ZSTD_isSkippableFrame` | zstd frames, skippable magics `0x184D2A50..5F`, legacy magics, random | [x] |
| 29 | `ZSTD_writeSkippableFrame`, `ZSTD_readSkippableFrame` | magicVariant 0..15 × payload sizes 0..1K | [x] |
| 30 | `ZSTD_compress_usingDict`, `ZSTD_decompress_usingDict` | dict ∈ {raw, trained, none} × dictSize grid × level grid | [x] |

### Group 3 — `ZSTD_CCtx` parameter cross-product (`tests/phase_b_params.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 31 | `ZSTD_CCtx_setParameter` + `ZSTD_CCtx_getParameter` | every param × {min, min+1, mid, max-1, max} — get-back value must match | [x] |
| 32 | `ZSTD_CCtx_setParameter` + `compressStream2` + roundtrip | `strategy` 1..9 × `minMatch` 3..7 × `windowLog` {10,17,23} | [x] |
| 33 | same | `strategy` 1..9 × `useRowMatchFinder` {auto,enable,disable} | [x] |
| 34 | same | `strategy` 1..9 × `targetLength` {0,16,64,999,131072} | [x] |
| 35 | same | `hashLog`×`chainLog`×`searchLog` grid (valid combos only) × strategy | [x] |
| 36 | same | `contentSizeFlag`×`checksumFlag`×`dictIDFlag` = 8 combos × pledgedSrcSize {unknown, exact} | [x] |
| 37 | same | `format` {zstd1, magicless} × checksum {0,1} × contentSize {0,1} | [x] |
| 38 | same | `enableLongDistanceMatching` {auto,enable,disable} × `ldmHashLog`×`ldmMinMatch`×`ldmBucketSizeLog`×`ldmHashRateLog` grid, LDM-bait input ≥ 1MB | [x] |
| 39 | same | `literalCompressionMode` {auto,huffman,uncompressed} × content classes | [x] |
| 40 | same | `targetCBlockSize` {0,1340,4096,65536,131072} × input ≥ 256KB (super-block path) | [x] |
| 41 | same | `maxBlockSize` {0,1024,4096,65536,131072} × input ≥ 256KB | [x] |
| 42 | same | `blockSplitterLevel` 0..6 × `splitAfterSequences` {auto,enable,disable} × mixed-entropy input | [x] |
| 43 | same | `srcSizeHint` {0,1,1K,1M,1G} × real srcSize {1K,1M} | [x] |
| 44 | same | `forceMaxWindow` {0,1} × `windowLog` {10,17,27} × decoder `windowLogMax` {10,17,27,31} | [x] |
| 45 | same | `rsyncable` {0,1}, `jobSize` grid, `overlapLog` grid, `nbWorkers`=0 (non-MT build) | [x] |
| 46 | same | `deterministicRefPrefix` {0,1} × `refPrefix` | [x] |
| 47 | same | `prefetchCDictTables` {auto,enable,disable} × CDict | [x] |
| 48 | same | `enableDedicatedDictSearch` {0,1} × strategy {greedy,lazy,lazy2} × CDict | [x] |
| 49 | same | `validateSequences` {0,1} (valid sequences only) | [x] |
| 50 | same | `enableSeqProducerFallback` {0,1} with no producer registered | [x] |
| 51 | same | `repcodeResolution` {auto,enable,disable} | [x] |
| 52 | `ZSTD_CCtx_setCParams`, `ZSTD_CCtx_setFParams`, `ZSTD_CCtx_setParams` | full struct grids | [x] |
| 53 | `ZSTD_CCtx_reset` | `ZSTD_reset_session_only`/`parameters`/`session_and_parameters` at every point in a session | [x] |
| 54 | `ZSTD_CCtx_setPledgedSrcSize` | exact, unknown(0/`ZSTD_CONTENTSIZE_UNKNOWN`), and > actual? (see ERRORS) | [x] |
| 55 | `ZSTD_getFrameProgression`, `ZSTD_toFlushNow` | sampled at every step of a streaming session, all 9 strategies | [x] |
| 56 | `ZSTD_CCtx_trace` | called with a NULL-hook build (always NULL) | [x] |

### Group 4 — `ZSTD_CCtx_params` object (`tests/phase_b_params.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 57 | `ZSTD_createCCtxParams`/`ZSTD_freeCCtxParams`/`ZSTD_CCtxParams_reset` | fresh + after set | [x] |
| 58 | `ZSTD_CCtxParams_init` | every level in min..22 | [x] |
| 59 | `ZSTD_CCtxParams_init_advanced` | full `ZSTD_parameters` grid | [x] |
| 60 | `ZSTD_CCtxParams_setParameter`/`getParameter` | every param × {min,mid,max} — value must round-trip | [x] |
| 61 | `ZSTD_CCtx_setParametersUsingCCtxParams` + compress | param object grid × input grid | [x] |
| 62 | `ZSTD_getCParamsFromCCtxParams` | param object grid × srcSize/dictSize grid | [x] |
| 63 | `ZSTD_CCtxParams_registerSequenceProducer`, `ZSTD_registerSequenceProducer` | register NULL, then unregister | [x] |
| 64 | `ZSTD_estimateCCtxSize_usingCCtxParams`, `ZSTD_estimateCStreamSize_usingCCtxParams` | param object grid | [x] |

### Group 5 — streaming compression (`tests/phase_b_streaming.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 65 | `ZSTD_compressStream2` | `ZSTD_e_continue` only, then a final `e_end`; in-chunk × out-chunk grid | [x] |
| 66 | `ZSTD_compressStream2` | `e_flush` after every chunk (frame must still decode) | [x] |
| 67 | `ZSTD_compressStream2` | randomized `{continue,flush,end}` interleaving, 200 random scripts | [x] |
| 68 | `ZSTD_compressStream2_simpleArgs` | same in/out grid, checks `srcPos`/`dstPos` | [x] |
| 69 | `ZSTD_compressStream2` | `stableInBuffer`=1 (whole input in one buffer) × `stableOutBuffer`=1 (buffer ≥ bound) | [x] |
| 70 | `ZSTD_initCStream` + `ZSTD_compressStream` + `ZSTD_flushStream` + `ZSTD_endStream` | level grid × chunk grid (legacy streaming API) | [x] |
| 71 | `ZSTD_initCStream_srcSize`, `_advanced`, `_usingDict`, `_usingCDict`, `_usingCDict_advanced`, `ZSTD_initCStream_internal` | full param/dict grid | [x] |
| 72 | `ZSTD_resetCStream` | pledgedSrcSize {0, unknown, exact} mid-session | [x] |
| 73 | `ZSTD_createCStream_advanced`, `ZSTD_createCCtx_advanced` | custom-mem = `{NULL,NULL,NULL}` | [x] |
| 74 | `ZSTD_decompressStream` | in-chunk × out-chunk grid × frames of every shape | [x] |
| 75 | `ZSTD_decompressStream_simpleArgs` | same, checks positions | [x] |
| 76 | `ZSTD_initDStream`, `ZSTD_resetDStream`, `ZSTD_DCtx_reset` (3 directives) | reused DStream across many frames | [x] |
| 77 | `ZSTD_decompressStream` | `d_stableOutBuffer`=1, output ≥ decompressed size | [x] |
| 78 | `ZSTD_decompressStream` | `d_maxBlockSize` {0,1024,131072}, `d_disableHuffmanAssembly` {0,1} | [x] |
| 79 | `ZSTD_decompressStream` | `d_forceIgnoreChecksum` {validate, ignore} × good & bad checksum | [x] |
| 80 | `ZSTD_nextSrcSizeToDecompress`, `ZSTD_nextInputType` | sampled at every step of a bufferless session | [x] |
| 81 | `ZSTD_DCtx_setMaxWindowSize`, `ZSTD_DCtx_setFormat` | windowLogMax grid × magicless frames | [x] |

### Group 6 — bufferless / block-level API (`tests/phase_b_bufferless.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 82 | `ZSTD_compressBegin` + `ZSTD_compressContinue` + `ZSTD_compressEnd` | level grid × block-splitting of input into 1..N chunks | [x] |
| 83 | `ZSTD_compressBegin_advanced`, `_advanced_internal` | full `ZSTD_parameters` grid × dict | [x] |
| 84 | `ZSTD_compressBegin_usingDict`, `_usingCDict`, `_usingCDict_advanced`, `_usingCDict_deprecated` | dict grid × level grid | [x] |
| 85 | `ZSTD_compressContinue_public`, `ZSTD_compressEnd_public` | same as row 82 | [x] |
| 86 | `ZSTD_copyCCtx` | copy after `compressBegin`, with/without pledgedSrcSize, then continue on the copy | [x] |
| 87 | `ZSTD_decompressBegin` + `ZSTD_nextSrcSizeToDecompress` + `ZSTD_decompressContinue` | full bufferless decode loop over every frame shape | [x] |
| 88 | `ZSTD_decompressBegin_usingDict`, `_usingDDict` | dict grid | [x] |
| 89 | `ZSTD_copyDCtx` | copy mid-decode, continue on the copy | [x] |
| 90 | `ZSTD_compressBlock` / `ZSTD_decompressBlock` (+`_deprecated`) | after `compressBegin`/`decompressBegin`; block sizes 1..`ZSTD_getBlockSize()` × content classes | [x] |
| 91 | `ZSTD_insertBlock`, `ZSTD_checkContinuity`, `ZSTD_invalidateRepCodes` | raw-block insertion into a decode stream | [x] |
| 92 | `ZSTD_getcBlockSize` | every block header shape (raw/rle/compressed/reserved) | [x] |
| 93 | `ZSTD_writeLastEmptyBlock` | dstCapacity = 3 and larger | [x] |
| 94 | `ZSTD_decompressBlock_internal`, `ZSTD_decodeLiteralsBlock_wrapper`, `ZSTD_decodeSeqHeaders` | real block payloads captured from `ZSTD_compressBlock` | [x] |
| 95 | `ZSTD_getSeqStore`, `ZSTD_resetSeqStore`, `ZSTD_reset_compressedBlockState` | after a block compression | [x] |
| 96 | `ZSTD_splitBlock`, `ZSTD_get1BlockSummary` | `blockSplitterLevel` 0..6 × mixed-entropy input | [x] |

### Group 7 — static (no-malloc) allocation (`tests/phase_b_static.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 97 | `ZSTD_initStaticCCtx` + full compression | workspace = exactly `ZSTD_estimateCCtxSize*`, and larger | [x] |
| 98 | `ZSTD_initStaticCStream` + streaming | workspace = `ZSTD_estimateCStreamSize*` | [x] |
| 99 | `ZSTD_initStaticDCtx` + full decompression | workspace = `ZSTD_estimateDCtxSize()` | [x] |
| 100 | `ZSTD_initStaticDStream` + streaming | workspace = `ZSTD_estimateDStreamSize*` | [x] |
| 101 | `ZSTD_initStaticCDict` | dictSize grid × level × `dictLoadMethod` × `dictContentType` | [x] |
| 102 | `ZSTD_initStaticDDict` | dictSize grid × `dictLoadMethod` × `dictContentType` | [x] |

### Group 8 — dictionaries (`tests/phase_b_dict.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 103 | `ZSTD_createCDict` / `ZSTD_freeCDict` + `ZSTD_compress_usingCDict` | level grid × dict {raw, trained} × dictSize grid | [x] |
| 104 | `ZSTD_createCDict_byReference` | same grid | [x] |
| 105 | `ZSTD_createCDict_advanced` | `dictLoadMethod`{2} × `dictContentType`{3} × cParams grid | [x] |
| 106 | `ZSTD_createCDict_advanced2` | same × `ZSTD_CCtx_params` grid × `compressionLevel` fallback | [x] |
| 107 | `ZSTD_compress_usingCDict_advanced` | `ZSTD_frameParameters` grid (contentSize/checksum/dictID) | [x] |
| 108 | `ZSTD_CCtx_refCDict` + `compressStream2` | `forceAttachDict` {default,forceAttach,forceCopy,forceLoad} × srcSize {tiny, large} × strategy 1..9 | [x] |
| 109 | `ZSTD_CCtx_loadDictionary` | dict grid × level grid × repeated loads | [x] |
| 110 | `ZSTD_CCtx_loadDictionary_byReference`, `_advanced` | `dictLoadMethod` × `dictContentType` grid | [x] |
| 111 | `ZSTD_CCtx_refPrefix`, `_advanced` | prefix ∈ {random, previous frame content} × `dictContentType` × `deterministicRefPrefix` | [x] |
| 112 | `ZSTD_createDDict`, `_byReference`, `_advanced`, `ZSTD_freeDDict` | `dictLoadMethod` × `dictContentType` × dict grid | [x] |
| 113 | `ZSTD_DCtx_refDDict` + `ZSTD_decompressStream` | `refMultipleDDicts` {single, multiple} with 1..4 DDicts | [x] |
| 114 | `ZSTD_DCtx_loadDictionary(_byReference/_advanced)`, `ZSTD_DCtx_refPrefix(_advanced)` | dict grid | [x] |
| 115 | `ZSTD_getDictID_fromDict`, `_fromCDict`, `_fromDDict`, `_fromFrame` | trained dict, raw dict, dictIDFlag {0,1}, every DID width | [x] |
| 116 | `ZSTD_DDict_dictContent`, `ZSTD_DDict_dictSize`, `ZSTD_copyDDictParameters` | all DDict variants | [x] |
| 117 | `ZSTD_getCParamsFromCDict` | all CDict variants | [x] |
| 118 | `ZSTD_loadCEntropy`, `ZSTD_loadDEntropy` | real trained dictionary payloads | [x] |
| 119 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `enableDedicatedDictSearch`=1 × strategy {greedy,lazy,lazy2} | [x] |

### Group 9 — explicit sequences (`tests/phase_b_sequences.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 120 | `ZSTD_generateSequences` | level grid × content classes × sizes; output compared field-by-field | [x] |
| 121 | `ZSTD_mergeBlockDelimiters` | sequences from row 120 | [x] |
| 122 | `ZSTD_compressSequences` | `blockDelimiters` {explicit, none} × `validateSequences` {0,1} × sequences from rows 120/121 | [x] |
| 123 | `ZSTD_compressSequencesAndLiterals` | same, plus literals buffer + `ZSTD_c_maxBlockSize` | [x] |
| 124 | `ZSTD_sequenceBound`, `ZSTD_convertBlockSequences` | srcSize grid | [x] |
| 125 | `ZSTD_referenceExternalSequences`, `ZSTD_ldm_*` (`generateSequences`, `blockCompress`, `skipSequences`, `skipRawSeqStoreBytes`, `getTableSize`, `getMaxNbSeq`, `fillHashTable`, `adjustParameters`) | LDM param grid × ≥1MB LDM-bait input | [x] |

### Group 10 — entropy / low-level codecs (`tests/phase_b_entropy.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 126 | `ZSTD_XXH32`, `_reset/_update/_digest/_copyState/_createState/_freeState`, `_canonicalFromHash`, `_hashFromCanonical` | seeds {0,1,0xFFFFFFFF} × sizes 0..1KB (all alignments) × update-chunking | [x] |
| 127 | `ZSTD_XXH64` + same family, `ZSTD_XXH_versionNumber` | seeds {0,1,u64::MAX} × sizes 0..1KB × chunking | [x] |
| 128 | `HIST_count`, `HIST_countFast`, `HIST_count_wksp`, `HIST_countFast_wksp`, `HIST_count_simple`, `HIST_isError` | maxSymbolValue {0,1,15,127,255} × content classes × sizes 0..64KB | [x] |
| 129 | `FSE_compress`, `FSE_decompress` | sizes 0..64KB × content classes | [x] |
| 130 | `FSE_compress2` | `maxSymbolValue` {1..255} × `tableLog` {5..`FSE_MAX_TABLELOG`} | [x] |
| 131 | `FSE_count_simple`, `FSE_optimalTableLog(_internal)`, `FSE_normalizeCount`, `FSE_NCountWriteBound`, `FSE_writeNCount`, `FSE_readNCount(_bmi2)` | count tables from random histograms × tableLog grid | [x] |
| 132 | `FSE_buildCTable(_wksp/_raw/_rle)`, `FSE_compress_usingCTable`, `FSE_CTable_size`/`FSE_sizeof_CTable` | normalized counts from row 131 | [x] |
| 133 | `FSE_buildDTable(_wksp/_raw/_rle)`, `FSE_decompress_usingDTable`, `FSE_DTable_size` | tables from row 132, round-trip | [x] |
| 134 | `FSE_decompress_wksp(_bmi2)`, `FSE_readNCount_bmi2` | `bmi2` flag {0,1} (DYNAMIC_BMI2=0 build) | [x] |
| 135 | `FSE_isError`, `FSE_getErrorName`, `FSE_versionNumber` | all codes | [x] |
| 136 | `HUF_compress`, `HUF_compress2`, `HUF_compress4X_wksp`, `HUF_compress4X_repeat`, `HUF_compress1X_wksp`, `HUF_compress1X_repeat` | `maxSymbolValue` × `tableLog` (`HUF_TABLELOG_MAX`) × `HUF_repeat` {none,check,valid} × `flags` bitmask {0..`HUF_flags_*`} | [x] |
| 137 | `HUF_decompress`, `HUF_decompress1X1(_DCtx/_DCtx_wksp/_usingDTable)`, `HUF_decompress1X2(...)`, `HUF_decompress4X1(...)`, `HUF_decompress4X2(...)`, `HUF_decompress1X_DCtx(_wksp)`, `HUF_decompress4X_hufOnly(_wksp)`, `HUF_decompress1X_usingDTable`, `HUF_decompress4X_usingDTable` | round-trip of every row-136 output, plus `flags` {0, `HUF_flags_bmi2`, `disableAsm`, `disableFast`} | [x] |
| 138 | `HUF_readDTableX1(_wksp)`, `HUF_readDTableX2(_wksp)`, `HUF_selectDecoder`, `HUF_readStats(_wksp)`, `HUF_getNbBitsFromCTable`, `HUF_buildCTable(_wksp)`, `HUF_writeCTable(_wksp)`, `HUF_estimateCompressedSize`, `HUF_validateCTable`, `HUF_optimalTableLog`, `HUF_CTableBound`, `HUF_compressBound`, `HUF_isError`, `HUF_getErrorName` | tables from row 136 | [x] |
| 139 | `ZSTD_buildFSETable` | every `ZSTD_symbolEncodingType_e` × real normalized counts × `bmi2` {0,1} | [x] |
| 140 | `ZSTD_decodeSeqHeaders` | real sequence-section payloads from compressed blocks | [x] |
| 141 | `ZSTD_buildCTable`, `ZSTD_selectEncodingType`, `ZSTD_encodeSequences`, `ZSTD_fseBitCost`, `ZSTD_crossEntropyCost`, `ZSTD_seqToCodes`, `ZSTD_buildBlockEntropyStats` | real seqStore states from compression | [x] |
| 142 | `ZSTD_noCompressLiterals`, `ZSTD_compressRleLiteralsBlock`, `ZSTD_compressLiterals` | literal buffers of every content class × sizes | [x] |
| 143 | `ZSTD_compressSuperBlock` | `targetCBlockSize` grid, real seqStore | [x] |
| 144 | `ZSTD_fillHashTable`, `ZSTD_fillDoubleHashTable`, `ZSTD_insertAndFindFirstIndex`, `ZSTD_row_update`, `ZSTD_updateTree`, `ZSTD_selectBlockCompressor`, `ZSTD_count`-driven block compressors | driven end-to-end via all 9 strategies × 4 dictModes × rowMatchFinder{0,1} | [x] |
| 145 | all 60 `ZSTD_compressBlock_*` exports | reached via row 144's cross-product (`noDict`/`extDict`/`dictMatchState`/`dedicatedDictSearch`, `_row` variants) | [x] |
| 146 | `ERR_getErrorString` | every `ZSTD_ErrorCode` 0..130 | [x] |

### Group 11 — dictBuilder (`tests/phase_b_dictbuilder.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 147 | `ZDICT_trainFromBuffer` | nbSamples {1,4,32,256} × sample sizes × dictBufferCapacity grid × content classes | [x] |
| 148 | `ZDICT_trainFromBuffer_cover` | `k`×`d`×`steps`×`nbThreads`(0,1)×`splitPoint`×`shrinkDict`×`shrinkDictMaxRegression`×`zParams` grid | [x] |
| 149 | `ZDICT_optimizeTrainFromBuffer_cover` | `steps` {0,2,4} × `d` {6,8} × `k`=0 (auto) | [x] |
| 150 | `ZDICT_trainFromBuffer_fastCover` | `k`×`d`×`f`×`accel`×`steps`×`splitPoint`×`shrinkDict` grid | [x] |
| 151 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `f` {15,20,23} × `accel` {1,5,10} × `steps` {0,2} | [x] |
| 152 | `ZDICT_trainFromBuffer_legacy` | `ZDICT_legacy_params_t` grid (`selectivityLevel` 0..12) | [x] |
| 153 | `ZDICT_finalizeDictionary` | custom dict content + samples + `ZDICT_params_t` grid (`compressionLevel`, `notificationLevel`, `dictID`) | [x] |
| 154 | `ZDICT_getDictID`, `ZDICT_getDictHeaderSize`, `ZDICT_isError`, `ZDICT_getErrorName` | dictionaries from rows 147-153 + raw buffers | [x] |
| 155 | `COVER_best_init/start/wait/finish/destroy`, `COVER_dictSelectionFree`, `COVER_dictSelectionError`, `COVER_dictSelectionIsError`, `COVER_checkTotalCompressedSize`, `COVER_computeEpochs`, `COVER_selectDict`, `COVER_sum` | driven by rows 148-151 + direct calls | [x] |
| 156 | `divsufsort`, `divbwt` | random and structured byte arrays, n ∈ {0,1,2,3,255,4096,65536} | [x] |

### Group 12 — legacy decoders v0.1 … v0.7 (`tests/phase_b_legacy.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 157 | `ZSTD_decompress`/`ZSTD_decompressStream` on legacy magics | magic `0xFD2FB522..0xFD2FB527` + `0xFD2FB51E`(v0.1) frames, valid & truncated; v0.1-v0.4 must be rejected (`ZSTD_LEGACY_SUPPORT=5`) | [x] |
| 158 | `ZSTDv01_*` (`decompress`, `findFrameSizeInfoLegacy`, `isError`, `resetDCtx`, `decompressContinue`, `nextSrcSizeToDecompress`, `createDCtx`, `freeDCtx`, `decompressDCtx`) — no `ZSTDv01_getDecompressedSize` is exported | crafted v0.1 frames; RAW + END blocks decode, `bt_rle` is `ERROR(GENERIC)` | [x] |
| 159 | `ZSTDv02_*` (no `FSEv02_*`/`HUFv02_*` symbols are exported — they are `static` in `zstd_v02.c`) | crafted v0.2 frames | [x] |
| 160 | `ZSTDv03_*` (no `FSEv03_*` symbols are exported) | crafted v0.3 frames | [x] |
| 161 | `ZSTDv04_*` (+ `ZBUFFv04_*`) | crafted v0.4 frames, direct + buffered streaming over a 10-shape chunk grid, dict | [x] |
| 162 | `ZSTDv05_*`, `FSEv05_*`, `HUFv05_*` (+ `ZBUFFv05_*`) | crafted v0.5 frames (RAW/RLE/compressed blocks), streaming, dict | [x] |
| 163 | `ZSTDv06_*`, `FSEv06_*`, `HUFv06_*` (+ `ZBUFFv06_*`) | crafted v0.6 frames (RAW/RLE/compressed blocks), streaming, dict | [x] |
| 164 | `ZSTDv07_*`, `FSEv07_*`, `HUFv07_*` (+ `ZBUFFv07_*`) | crafted v0.7 frames, DDict, custom allocator, correct + corrupted 22-bit frame checksum, streaming, `ZSTDv07_findFrameSizeInfoLegacy` | [x] |
| 165 | `ZSTD_isFrame` / `ZSTD_findFrameCompressedSize` / `ZSTD_getFrameContentSize` / `ZSTD_decompressBound` / `ZSTD_getDictID_fromFrame` / `ZSTD_getDecompressedSize` on legacy frames | all 7 legacy magics | [x] |

### Group 13 — deprecated ZBUFF + ZSTDMT (`tests/phase_b_misc.rs`)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 166 | `ZBUFF_createCCtx(_advanced)`, `ZBUFF_compressInit(_advanced/_usingDict)`, `ZBUFF_compressContinue`, `ZBUFF_compressFlush`, `ZBUFF_compressEnd`, `ZBUFF_freeCCtx` | level grid × chunk grid × dict | [x] |
| 167 | `ZBUFF_createDCtx(_advanced)`, `ZBUFF_decompressInit(_usingDict)`, `ZBUFF_decompressContinue`, `ZBUFF_freeDCtx` | chunk grid × frames from row 166 | [x] |
| 168 | `ZBUFF_isError`, `ZBUFF_getErrorName`, `ZBUFF_recommendedCInSize/COutSize/DInSize/DOutSize` | no input | [x] |
| 169 | `ZSTDMT_createCCtx(_advanced)`, `ZSTDMT_freeCCtx`, `ZSTDMT_sizeof_CCtx`, `ZSTDMT_compressStream_generic`, `ZSTDMT_initCStream_internal`, `ZSTDMT_nextInputSizeHint`, `ZSTDMT_toFlushNow`, `ZSTDMT_getFrameProgression`, `ZSTDMT_updateCParams_whileCompressing`, `ZSTDMT_getCParamsFromMTCCtx`, `ZSTDMT_CCtxParam_setNbWorkers`, `ZSTDMT_CCtxParam_setMTCtxParameter`, `ZSTDMT_CCtxParam_getMTCtxParameter` | nbWorkers ∈ {0,1,2} in the **non-MT** build | [x] |
| 170 | `POOL_create(_advanced)`, `POOL_free`, `POOL_joinJobs`, `POOL_resize`, `POOL_sizeof`, `POOL_add`, `POOL_tryAdd` | numThreads {0,1,4} × queueSize {0,1,4} (non-MT build) | [x] |
| 171 | `ZSTD_getErrorString` / `ZSTD_getErrorName` for every produced error | all rows above | [x] |
