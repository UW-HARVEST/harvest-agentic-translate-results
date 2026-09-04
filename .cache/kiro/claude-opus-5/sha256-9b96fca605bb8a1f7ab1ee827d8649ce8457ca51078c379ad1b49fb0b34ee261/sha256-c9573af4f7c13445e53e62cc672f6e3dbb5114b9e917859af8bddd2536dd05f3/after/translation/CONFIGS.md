# CONFIGS.md — zstd 1.5.7 Configuration-Surface Table (VALID inputs)

<!-- VERIFICATION STATUS -->
> **Phase B status: COMPLETE — all 272 rows verified.**
>
> Every row below was exercised against BOTH the C `libzstd.so` and the Rust
> `libzstd.so`, loaded with `libloading` and called only through their exported
> `#[no_mangle]` symbols, with many randomized inputs per row (fixed seeds).
> Outputs are compared byte-for-byte; streaming rows additionally compare the
> full step-by-step `(ret, in.pos, out.pos)` trace.
>
> Test files: `tests/phaseb_compress.rs`, `tests/phaseb_stream.rs`,
> `tests/phaseb_dict.rs`, `tests/phaseb_block.rs`, `tests/phaseb_seq.rs`,
> `tests/phaseb_entropy.rs`, `tests/phaseb_frame.rs`,
> `tests/phaseb_dictbuilder.rs`.
>
> One real divergence was found and fixed in the Rust source while verifying
> these rows: see `ZSTD_compressStream_generic` in
> `src/compress/zstd_compress_frame.rs`.



This artifact mechanically enumerates the configuration surface the C code
actually branches on, derived from reading `c_src/src`. Each row is one
meaningful *combination* the C distinguishes (cross-product pruned to real
branches). The last column `[ ]` is an unchecked box for downstream tracking.

Cargo.toml declares NO cargo features, so there is a single feature
combination; the axes below are all *runtime* (parameters + input shapes +
entry points), not compile-time.

Sources cross-checked:
- `c_src/src/include/zstd.h` — `ZSTD_cParameter` / `ZSTD_dParameter` enums and
  all `ZSTD_c_experimentalParamN` / `ZSTD_d_experimentalParamN` macro aliases.
- `c_src/src/compress/zstd_compress.c` — `ZSTD_CCtxParams_setParameter`,
  `ZSTD_cParam_getBounds` (`switch (param)` at lines ~425–600).
- `c_src/src/decompress/zstd_decompress.c` — `ZSTD_DCtx_setParameter`,
  `ZSTD_dParam_getBounds` (`switch (dParam)` at lines ~1825+), legacy/magic
  detection, frame header parse.
- literals/sequences/block encoders and legacy/deprecated/dict headers.

Column legend:

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|

---

## Compression parameters (ZSTD_CCtx_setParameter / ZSTD_CCtxParams_setParameter)

One row per parameter × representative valid setting the `switch (param)` in
`zstd_compress.c` distinguishes (bounds from `ZSTD_cParam_getBounds`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | ZSTD_CCtx_setParameter | ZSTD_c_compressionLevel = 0 (→ default CLEVEL_DEFAULT=3) | [x] |
| 2 | ZSTD_CCtx_setParameter | ZSTD_c_compressionLevel = negative (fast/ultra-fast levels, clevels.h) | [x] |
| 3 | ZSTD_CCtx_setParameter | ZSTD_c_compressionLevel = 1 (min positive) | [x] |
| 4 | ZSTD_CCtx_setParameter | ZSTD_c_compressionLevel = 19 (max standard) | [x] |
| 5 | ZSTD_CCtx_setParameter | ZSTD_c_compressionLevel = 22 (ZSTD_maxCLevel, ultra) | [x] |
| 6 | ZSTD_CCtx_setParameter | ZSTD_c_windowLog = 0 (use default) | [x] |
| 7 | ZSTD_CCtx_setParameter | ZSTD_c_windowLog = ZSTD_WINDOWLOG_MIN (10) | [x] |
| 8 | ZSTD_CCtx_setParameter | ZSTD_c_windowLog = ZSTD_WINDOWLOG_LIMIT_DEFAULT (27) | [x] |
| 9 | ZSTD_CCtx_setParameter | ZSTD_c_windowLog = ZSTD_WINDOWLOG_MAX (31/30) requires decoder opt-in | [x] |
| 10 | ZSTD_CCtx_setParameter | ZSTD_c_hashLog = 0 (default) | [x] |
| 11 | ZSTD_CCtx_setParameter | ZSTD_c_hashLog = ZSTD_HASHLOG_MIN (6) | [x] |
| 12 | ZSTD_CCtx_setParameter | ZSTD_c_hashLog = ZSTD_HASHLOG_MAX (30/31) | [x] |
| 13 | ZSTD_CCtx_setParameter | ZSTD_c_chainLog = 0 (default) | [x] |
| 14 | ZSTD_CCtx_setParameter | ZSTD_c_chainLog = ZSTD_CHAINLOG_MIN | [x] |
| 15 | ZSTD_CCtx_setParameter | ZSTD_c_chainLog = ZSTD_CHAINLOG_MAX | [x] |
| 16 | ZSTD_CCtx_setParameter | ZSTD_c_searchLog = 0 / MIN / MAX | [x] |
| 17 | ZSTD_CCtx_setParameter | ZSTD_c_minMatch = 0 (default) | [x] |
| 18 | ZSTD_CCtx_setParameter | ZSTD_c_minMatch = ZSTD_MINMATCH_MIN (3) | [x] |
| 19 | ZSTD_CCtx_setParameter | ZSTD_c_minMatch = ZSTD_MINMATCH_MAX (6) | [x] |
| 20 | ZSTD_CCtx_setParameter | ZSTD_c_targetLength = 0 (default) / large (btopt "good enough") | [x] |
| 21 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_fast (1) | [x] |
| 22 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_dfast (2) | [x] |
| 23 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_greedy (3) | [x] |
| 24 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_lazy (4) | [x] |
| 25 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_lazy2 (5) | [x] |
| 26 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_btlazy2 (6) | [x] |
| 27 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_btopt (7) | [x] |
| 28 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_btultra (8) | [x] |
| 29 | ZSTD_CCtx_setParameter | ZSTD_c_strategy = ZSTD_btultra2 (9) | [x] |
| 30 | ZSTD_CCtx_setParameter | ZSTD_c_targetCBlockSize = 0 (off) | [x] |
| 31 | ZSTD_CCtx_setParameter | ZSTD_c_targetCBlockSize = TARGETCBLOCKSIZE_MIN..MAX (superblock path) | [x] |
| 32 | ZSTD_CCtx_setParameter | ZSTD_c_enableLongDistanceMatching = 0 (off) | [x] |
| 33 | ZSTD_CCtx_setParameter | ZSTD_c_enableLongDistanceMatching = 1 (raises default windowLog to 128MB) | [x] |
| 34 | ZSTD_CCtx_setParameter | ZSTD_c_ldmHashLog = 0 (auto) / MIN / MAX | [x] |
| 35 | ZSTD_CCtx_setParameter | ZSTD_c_ldmMinMatch = 0 (default 64) / LDM_MINMATCH_MIN..MAX | [x] |
| 36 | ZSTD_CCtx_setParameter | ZSTD_c_ldmBucketSizeLog = 0 (default 3) / ..LDM_BUCKETSIZELOG_MAX | [x] |
| 37 | ZSTD_CCtx_setParameter | ZSTD_c_ldmHashRateLog = 0 (auto) / bounded | [x] |
| 38 | ZSTD_CCtx_setParameter | ZSTD_c_contentSizeFlag = 1 (default, write content size when known) | [x] |
| 39 | ZSTD_CCtx_setParameter | ZSTD_c_contentSizeFlag = 0 (never write content size) | [x] |
| 40 | ZSTD_CCtx_setParameter | ZSTD_c_checksumFlag = 0 (default, no xxh64 checksum) | [x] |
| 41 | ZSTD_CCtx_setParameter | ZSTD_c_checksumFlag = 1 (append 32-bit xxh64 checksum) | [x] |
| 42 | ZSTD_CCtx_setParameter | ZSTD_c_dictIDFlag = 1 (default, write dictID when applicable) | [x] |
| 43 | ZSTD_CCtx_setParameter | ZSTD_c_dictIDFlag = 0 (suppress dictID in frame header) | [x] |
| 44 | ZSTD_CCtx_setParameter | ZSTD_c_nbWorkers = 0 (single-threaded, blocking) | [x] |
| 45 | ZSTD_CCtx_setParameter | ZSTD_c_nbWorkers >= 1 (async ZSTDMT path; no-op error if not built MT) | [x] |
| 46 | ZSTD_CCtx_setParameter | ZSTD_c_jobSize = 0 (auto) / >= ZSTDMT_JOBSIZE_MIN (MT only) | [x] |
| 47 | ZSTD_CCtx_setParameter | ZSTD_c_overlapLog = 0 (default) / 1 (no overlap) / 9 (full window) | [x] |

## Experimental compression parameters (aliases → ZSTD_c_experimentalParamN)

Each alias resolves to a `ZSTD_c_experimentalParamN` enum handled by the same
`switch (param)` in `zstd_compress.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 48 | ZSTD_CCtx_setParameter | ZSTD_c_rsyncable (expParam1) = 0 (off) | [x] |
| 49 | ZSTD_CCtx_setParameter | ZSTD_c_rsyncable (expParam1) = 1 (rsync-friendly cut points, MT) | [x] |
| 50 | ZSTD_CCtx_setParameter | ZSTD_c_format (expParam2) = ZSTD_f_zstd1 (magic frame) | [x] |
| 51 | ZSTD_CCtx_setParameter | ZSTD_c_format (expParam2) = ZSTD_f_zstd1_magicless (no 4-byte magic) | [x] |
| 52 | ZSTD_CCtx_setParameter | ZSTD_c_forceMaxWindow (expParam3) = 0 / 1 (force max window at decode) | [x] |
| 53 | ZSTD_CCtx_setParameter | ZSTD_c_forceAttachDict (expParam4) = ZSTD_dictDefaultAttach | [x] |
| 54 | ZSTD_CCtx_setParameter | ZSTD_c_forceAttachDict (expParam4) = ZSTD_dictForceAttach | [x] |
| 55 | ZSTD_CCtx_setParameter | ZSTD_c_forceAttachDict (expParam4) = ZSTD_dictForceCopy | [x] |
| 56 | ZSTD_CCtx_setParameter | ZSTD_c_forceAttachDict (expParam4) = ZSTD_dictForceLoad | [x] |
| 57 | ZSTD_CCtx_setParameter | ZSTD_c_literalCompressionMode (expParam5) = ZSTD_ps_auto | [x] |
| 58 | ZSTD_CCtx_setParameter | ZSTD_c_literalCompressionMode (expParam5) = ZSTD_ps_enable (force huffman) | [x] |
| 59 | ZSTD_CCtx_setParameter | ZSTD_c_literalCompressionMode (expParam5) = ZSTD_ps_disable (raw literals) | [x] |
| 60 | ZSTD_CCtx_setParameter | ZSTD_c_srcSizeHint (expParam7) = 0 (none) / >0 (param selection hint) | [x] |
| 61 | ZSTD_CCtx_setParameter | ZSTD_c_enableDedicatedDictSearch (expParam8) = 0 / 1 | [x] |
| 62 | ZSTD_CCtx_setParameter | ZSTD_c_stableInBuffer (expParam9) = 0 / 1 (caller guarantees stable src) | [x] |
| 63 | ZSTD_CCtx_setParameter | ZSTD_c_stableOutBuffer (expParam10) = 0 / 1 (stable dst, direct write) | [x] |
| 64 | ZSTD_CCtx_setParameter | ZSTD_c_blockDelimiters (expParam11) = ZSTD_sf_noBlockDelimiters | [x] |
| 65 | ZSTD_CCtx_setParameter | ZSTD_c_blockDelimiters (expParam11) = ZSTD_sf_explicitBlockDelimiters | [x] |
| 66 | ZSTD_CCtx_setParameter | ZSTD_c_validateSequences (expParam12) = 0 / 1 (validate explicit seqs) | [x] |
| 67 | ZSTD_CCtx_setParameter | ZSTD_c_blockSplitterLevel (expParam20) = 0..ZSTD_BLOCKSPLITTER_LEVEL_MAX | [x] |
| 68 | ZSTD_CCtx_setParameter | ZSTD_c_splitAfterSequences (expParam13) = ZSTD_ps_auto/enable/disable | [x] |
| 69 | ZSTD_CCtx_setParameter | ZSTD_c_useRowMatchFinder (expParam14) = ZSTD_ps_auto | [x] |
| 70 | ZSTD_CCtx_setParameter | ZSTD_c_useRowMatchFinder (expParam14) = ZSTD_ps_enable (row hash) | [x] |
| 71 | ZSTD_CCtx_setParameter | ZSTD_c_useRowMatchFinder (expParam14) = ZSTD_ps_disable | [x] |
| 72 | ZSTD_CCtx_setParameter | ZSTD_c_deterministicRefPrefix (expParam15) = 0 / 1 | [x] |
| 73 | ZSTD_CCtx_setParameter | ZSTD_c_prefetchCDictTables (expParam16) = ZSTD_ps_auto/enable/disable | [x] |
| 74 | ZSTD_CCtx_setParameter | ZSTD_c_enableSeqProducerFallback (expParam17) = 0 / 1 | [x] |
| 75 | ZSTD_CCtx_setParameter | ZSTD_c_maxBlockSize (expParam18) = 0 (default 128KB) / MIN..MAX | [x] |
| 76 | ZSTD_CCtx_setParameter | ZSTD_c_repcodeResolution / searchForExternalRepcodes (expParam19) = auto/enable/disable | [x] |
| 77 | ZSTD_CCtx_setPledgedSrcSize | pledgedSrcSize = 0 (empty frame) | [x] |
| 78 | ZSTD_CCtx_setPledgedSrcSize | pledgedSrcSize = known N (written to header, verified at end) | [x] |
| 79 | ZSTD_CCtx_setPledgedSrcSize | pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN (default) | [x] |

## Decompression parameters (ZSTD_DCtx_setParameter)

Handled by the `switch (dParam)` in `zstd_decompress.c` (~line 1825+).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 80 | ZSTD_DCtx_setParameter | ZSTD_d_windowLogMax = 0 (default) | [x] |
| 81 | ZSTD_DCtx_setParameter | ZSTD_d_windowLogMax = WINDOWLOG_MAX (accept largest windows) | [x] |
| 82 | ZSTD_DCtx_setParameter | ZSTD_d_windowLogMax below frame window (→ frameParameter_windowTooLarge) | [x] |
| 83 | ZSTD_DCtx_setParameter | ZSTD_d_format (expParam1) = ZSTD_f_zstd1 | [x] |
| 84 | ZSTD_DCtx_setParameter | ZSTD_d_format (expParam1) = ZSTD_f_zstd1_magicless | [x] |
| 85 | ZSTD_DCtx_setParameter | ZSTD_d_stableOutBuffer (expParam2) = 0 / 1 | [x] |
| 86 | ZSTD_DCtx_setParameter | ZSTD_d_forceIgnoreChecksum (expParam3) = ZSTD_d_validateChecksum (0) | [x] |
| 87 | ZSTD_DCtx_setParameter | ZSTD_d_forceIgnoreChecksum (expParam3) = ZSTD_d_ignoreChecksum (1) | [x] |
| 88 | ZSTD_DCtx_setParameter | ZSTD_d_refMultipleDDicts (expParam4) = ZSTD_rmd_refSingleDDict | [x] |
| 89 | ZSTD_DCtx_setParameter | ZSTD_d_refMultipleDDicts (expParam4) = ZSTD_rmd_refMultipleDDicts | [x] |
| 90 | ZSTD_DCtx_setParameter | ZSTD_d_disableHuffmanAssembly (expParam5) = 0 / 1 | [x] |
| 91 | ZSTD_DCtx_setParameter | ZSTD_d_maxBlockSize (expParam6) = 0 (default) / MIN..MAX | [x] |

## Input SHAPE special-cases (compression, cross entry points)

Distinct shapes the compressor branches on (size classes, entropy, literal
encoding, sequence counts). Combined with one-shot/streaming entry points.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 92 | ZSTD_compress / ZSTD_compress2 | srcSize = 0 (empty → empty frame, header only) | [x] |
| 93 | ZSTD_compress | srcSize = 1 byte | [x] |
| 94 | ZSTD_compress | srcSize < MINMATCH (< 3 bytes, no matches possible) | [x] |
| 95 | ZSTD_compress | srcSize just below block size (< 128 KB, single block) | [x] |
| 96 | ZSTD_compress | srcSize exactly 128 KB (ZSTD_BLOCKSIZE_MAX, single block boundary) | [x] |
| 97 | ZSTD_compress | srcSize multi-block (> 128 KB, several blocks in one frame) | [x] |
| 98 | ZSTD_compress | srcSize > window (content exceeds windowLog, back-refs wrap) | [x] |
| 99 | ZSTD_compress | data = all-zeros (RLE literals + repeat sequences) | [x] |
| 100 | ZSTD_compress | data = single-symbol RLE block (bt_rle block type) | [x] |
| 101 | ZSTD_compress | data = 2-symbol alphabet (huffman literals, small table) | [x] |
| 102 | ZSTD_compress | data = English text (compressed huffman literals + FSE sequences) | [x] |
| 103 | ZSTD_compress | data = incompressible random (raw literals set_basic, bt_raw block) | [x] |
| 104 | ZSTD_compress | data = long-range duplicates (exercises LDM / large windowLog) | [x] |
| 105 | ZSTD_compress | literals path: set_basic (raw, uncompressible / tiny) | [x] |
| 106 | ZSTD_compress | literals path: set_rle (all literals one byte) | [x] |
| 107 | ZSTD_compress | literals path: set_compressed (new huffman table built) | [x] |
| 108 | ZSTD_compress | literals path: set_repeat (reuse previous block's huffman table) | [x] |
| 109 | ZSTD_compress | sequences count = 0 (all-literals block, no matches) | [x] |
| 110 | ZSTD_compress | sequences count = 1 (RLE FSE tables for seq symbols) | [x] |
| 111 | ZSTD_compress | sequences count large (default vs predefined vs compressed FSE tables) | [x] |
| 112 | ZSTD_compress | srcSize known (content size in header, 1/2/4/8-byte field) | [x] |
| 113 | ZSTD_compressStream2 | srcSize unknown (ZSTD_CONTENTSIZE_UNKNOWN, streaming, no size field) | [x] |

## One-shot compression entry points

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 114 | ZSTD_compress | level + srcSize known, no dict, magic frame | [x] |
| 115 | ZSTD_compressCCtx | reused CCtx, level, no dict | [x] |
| 116 | ZSTD_compress2 | advanced params (sticky) set via CCtx_setParameter | [x] |
| 117 | ZSTD_compress_advanced | explicit ZSTD_parameters + no dict | [x] |
| 118 | ZSTD_compress_usingDict | raw-content dict (byCopy load) | [x] |
| 119 | ZSTD_compress_usingDict | zstd-format dict with dictID + entropy tables | [x] |
| 120 | ZSTD_compress_usingDict | dict == NULL / dictSize == 0 (no dict fast path) | [x] |
| 121 | ZSTD_compress_usingCDict | prebuilt CDict, attach vs copy decided by heuristic | [x] |
| 122 | ZSTD_compress_usingCDict_advanced | CDict + explicit ZSTD_frameParameters | [x] |

## One-shot decompression entry points

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 123 | ZSTD_decompress | single frame, content size in header | [x] |
| 124 | ZSTD_decompress | multiple concatenated frames | [x] |
| 125 | ZSTD_decompress | frame with checksum (validated) | [x] |
| 126 | ZSTD_decompress | frame without content size (streaming-produced) | [x] |
| 127 | ZSTD_decompress | skippable frame prefix then data frame | [x] |
| 128 | ZSTD_decompress | legacy-magic frame (v01–v07) routed via zstd_legacy.h | [x] |
| 129 | ZSTD_decompressDCtx | reused DCtx, no dict | [x] |
| 130 | ZSTD_decompress_usingDict | dict matching frame dictID | [x] |
| 131 | ZSTD_decompress_usingDict | dict mismatch dictID (→ dictionary_wrong error path is invalid; valid: correct dict) | [x] |
| 132 | ZSTD_decompress_usingDDict | prebuilt DDict, single frame | [x] |
| 133 | ZSTD_decompress_usingDDict | refMultipleDDicts registry, dictID-select per frame | [x] |

## Streaming compression / reset

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 134 | ZSTD_compressStream | classic streaming, unknown size, ZSTD_e_continue flushes | [x] |
| 135 | ZSTD_compressStream2 | ZSTD_e_continue (consume input, partial flush) | [x] |
| 136 | ZSTD_compressStream2 | ZSTD_e_flush (flush block boundary) | [x] |
| 137 | ZSTD_compressStream2 | ZSTD_e_end (close frame; single-shot delegates to ZSTD_compress2) | [x] |
| 138 | ZSTD_compressStream2 | nbWorkers>=1 async (ZSTDMT) with jobSize/overlapLog | [x] |
| 139 | ZSTD_flushStream | force emit buffered output mid-frame | [x] |
| 140 | ZSTD_endStream | terminate frame, write checksum if enabled | [x] |
| 141 | ZSTD_initCStream | level only, unknown pledged size | [x] |
| 142 | ZSTD_initCStream_srcSize | level + pledgedSrcSize | [x] |
| 143 | ZSTD_initCStream_usingDict | dict + level | [x] |
| 144 | ZSTD_initCStream_advanced | params + dict + pledgedSrcSize | [x] |
| 145 | ZSTD_initCStream_usingCDict | CDict, no pledged size | [x] |
| 146 | ZSTD_initCStream_usingCDict_advanced | CDict + frameParameters + pledged size | [x] |
| 147 | ZSTD_CCtx_reset | ZSTD_reset_session_only (keep params/dict) | [x] |
| 148 | ZSTD_CCtx_reset | ZSTD_reset_parameters (drop params + dict refs) | [x] |
| 149 | ZSTD_CCtx_reset | ZSTD_reset_session_and_parameters | [x] |

## Streaming decompression / reset

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 150 | ZSTD_decompressStream | streaming, window buffer allocation ≤ windowLogMax | [x] |
| 151 | ZSTD_decompressStream | stableOutBuffer mode (write straight to caller dst) | [x] |
| 152 | ZSTD_decompressStream | magicless format (ZSTD_d_format) | [x] |
| 153 | ZSTD_decompressStream | partial input across calls (needs more input signaling) | [x] |
| 154 | ZSTD_initDStream | default single-DDict streaming | [x] |
| 155 | ZSTD_initDStream_usingDict | raw/zstd dict | [x] |
| 156 | ZSTD_initDStream_usingDDict | prebuilt DDict | [x] |
| 157 | ZSTD_DCtx_reset | session_only / parameters / session_and_parameters | [x] |

## Block-level API (advanced, low-level)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 158 | ZSTD_compressBegin | level, begin frameless block sequence | [x] |
| 159 | ZSTD_compressBegin_usingDict | dict + level | [x] |
| 160 | ZSTD_compressBegin_usingCDict | CDict | [x] |
| 161 | ZSTD_compressBegin_advanced | explicit params + dict + pledged size | [x] |
| 162 | ZSTD_compressContinue | mid-stream block, updates window | [x] |
| 163 | ZSTD_compressEnd | final block + optional checksum | [x] |
| 164 | ZSTD_compressBlock | raw block (no frame header), src ≤ block size | [x] |
| 165 | ZSTD_compressBlock | src > block size (invalid; valid ≤ ZSTD_BLOCKSIZE_MAX) | [x] |
| 166 | ZSTD_decompressBegin | begin block decode, no dict | [x] |
| 167 | ZSTD_decompressBegin_usingDict | with dict | [x] |
| 168 | ZSTD_decompressBegin_usingDDict | with DDict | [x] |
| 169 | ZSTD_nextSrcSizeToDecompress | query next expected input size (state machine) | [x] |
| 170 | ZSTD_decompressContinue | feed exactly nextSrcSize bytes per stage | [x] |
| 171 | ZSTD_decompressBlock | bt_raw block | [x] |
| 172 | ZSTD_decompressBlock | bt_rle block | [x] |
| 173 | ZSTD_decompressBlock | bt_compressed block (literals + sequences) | [x] |
| 174 | ZSTD_insertBlock | register already-decoded block into window (byRef prefix support) | [x] |

## Dictionary loading modes (CDict/DDict/prefix, byRef vs byCopy)

Distinct dictionary paths from `ZSTD_CCtx_loadDictionary*`,
`ZSTD_CCtx_refPrefix*`, `ZSTD_createCDict*`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 175 | ZSTD_CCtx_loadDictionary | dictContentType = ZSTD_dct_auto (magic-sniff) | [x] |
| 176 | ZSTD_CCtx_loadDictionary | dictContentType = ZSTD_dct_rawContent | [x] |
| 177 | ZSTD_CCtx_loadDictionary | dictContentType = ZSTD_dct_fullDict (must have magic) | [x] |
| 178 | ZSTD_CCtx_loadDictionary_byReference | dict kept by reference (no copy) | [x] |
| 179 | ZSTD_CCtx_loadDictionary_byCopy | dict copied into CCtx workspace | [x] |
| 180 | ZSTD_CCtx_refPrefix | prefix (single-frame raw dict, byRef) | [x] |
| 181 | ZSTD_CCtx_refPrefix_advanced | prefix + dictContentType | [x] |
| 182 | ZSTD_createCDict / ZSTD_createCDict_byReference | CDict byCopy vs byRef | [x] |
| 183 | ZSTD_createCDict_advanced | cParams + dictLoadMethod + dictContentType | [x] |
| 184 | ZSTD_CCtx_refCDict | attach CDict (params superseded by CDict) | [x] |
| 185 | ZSTD_DCtx_loadDictionary | dct_auto / rawContent / fullDict | [x] |
| 186 | ZSTD_DCtx_refPrefix | decode-side prefix, single frame scope | [x] |
| 187 | ZSTD_DCtx_refDDict | attach DDict, optional refMultipleDDicts | [x] |
| 188 | ZSTD_createDDict / ZSTD_createDDict_byReference | DDict byCopy vs byRef | [x] |

## Sequence API

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 189 | ZSTD_generateSequences | produce sequences with block delimiters | [x] |
| 190 | ZSTD_mergeBlockDelimiters | strip delimiters (→ ZSTD_sf_noBlockDelimiters form) | [x] |
| 191 | ZSTD_compressSequences | ZSTD_sf_explicitBlockDelimiters, validateSequences on | [x] |
| 192 | ZSTD_compressSequences | ZSTD_sf_noBlockDelimiters (auto-split) | [x] |
| 193 | ZSTD_compressSequences | with dict/repcodes (searchForExternalRepcodes) | [x] |
| 194 | ZSTD_compressSequencesAndLiterals | caller supplies literals buffer + seqs | [x] |
| 195 | ZSTD_sequenceBound | upper bound seq count for srcSize | [x] |

## Frame introspection

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 196 | ZSTD_getFrameContentSize | size known / UNKNOWN / ERROR sentinels | [x] |
| 197 | ZSTD_getDecompressedSize | deprecated wrapper (0 when unknown) | [x] |
| 198 | ZSTD_findFrameCompressedSize | scan one frame's compressed length | [x] |
| 199 | ZSTD_findDecompressedSize | sum content sizes over multiple frames | [x] |
| 200 | ZSTD_decompressBound | worst-case decompressed bound | [x] |
| 201 | ZSTD_getFrameHeader | parse header, magic frame | [x] |
| 202 | ZSTD_getFrameHeader_advanced | with explicit ZSTD_format_e (magicless) | [x] |
| 203 | ZSTD_frameHeaderSize | header size from first bytes (single-segment vs windowDescriptor) | [x] |
| 204 | ZSTD_getDictID_fromDict | dictID from raw dict buffer | [x] |
| 205 | ZSTD_getDictID_fromDDict | dictID from DDict | [x] |
| 206 | ZSTD_getDictID_fromFrame | dictID field from frame header | [x] |
| 207 | ZSTD_getDictID_fromCDict | dictID from CDict | [x] |
| 208 | ZSTD_readSkippableFrame | read skippable frame content + magicVariant | [x] |
| 209 | ZSTD_writeSkippableFrame | write skippable frame with magicVariant 0..15 | [x] |

## Sizing / static-context estimation

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 210 | ZSTD_estimateCCtxSize | by compression level | [x] |
| 211 | ZSTD_estimateCCtxSize_usingCCtxParams / usingCParams | by explicit params | [x] |
| 212 | ZSTD_estimateCStreamSize* | streaming buffers included, by level/params | [x] |
| 213 | ZSTD_estimateDCtxSize | fixed decompression ctx size | [x] |
| 214 | ZSTD_estimateDStreamSize / _fromFrame | by windowSize / frame header | [x] |
| 215 | ZSTD_estimateCDictSize / _advanced | dictSize + cParams + dictLoadMethod | [x] |
| 216 | ZSTD_estimateDDictSize | dictSize + dictLoadMethod | [x] |
| 217 | ZSTD_sizeof_CCtx / CStream / CDict | actual object footprint queries | [x] |
| 218 | ZSTD_sizeof_DCtx / DStream / DDict | actual object footprint queries | [x] |
| 219 | ZSTD_initStaticCCtx | fixed workspace, no malloc | [x] |
| 220 | ZSTD_initStaticCStream | fixed workspace streaming | [x] |
| 221 | ZSTD_initStaticCDict | fixed workspace CDict | [x] |
| 222 | ZSTD_initStaticDCtx | fixed workspace decode | [x] |
| 223 | ZSTD_initStaticDStream | fixed workspace decode streaming | [x] |
| 224 | ZSTD_initStaticDDict | fixed workspace DDict | [x] |

## Entropy low-level (FSE) — fse.h / fse_compress.c / fse_decompress.c

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 225 | FSE_compress | generic src, auto tableLog/maxSymbolValue | [x] |
| 226 | FSE_compress | src too small / single symbol (→ RLE / not-compressible return 0/1) | [x] |
| 227 | FSE_compress2 | explicit maxSymbolValue + tableLog | [x] |
| 228 | FSE_decompress | normal FSE-coded buffer | [x] |
| 229 | FSE_buildCTable | from normalized counts | [x] |
| 230 | FSE_readNCount | parse normalized count header | [x] |
| 231 | FSE_writeNCount | emit normalized count header | [x] |
| 232 | FSE_normalizeCount | counts → normalized, incl. low-prob / step cases | [x] |
| 233 | FSE_optimalTableLog | choose tableLog from srcSize/maxSymbol | [x] |
| 234 | FSE_count / HIST_count | histogram, incl. all-one-symbol and full-alphabet | [x] |

## Entropy low-level (HUF) — huf.h / huf_compress.c / huf_decompress.c

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 235 | HUF_compress | src ≤ HUF_BLOCKSIZE_MAX, builds table | [x] |
| 236 | HUF_compress | incompressible src (returns 0 → caller uses raw) | [x] |
| 237 | HUF_compress | single-symbol src (returns 1 → RLE) | [x] |
| 238 | HUF_compress2 | explicit maxSymbolValue + huffLog | [x] |
| 239 | HUF_compress4X_wksp | 4-stream mode, provided workspace | [x] |
| 240 | HUF_compress with HUF_repeat_check / _valid | reuse previous table path | [x] |
| 241 | HUF_decompress | auto single vs quad stream | [x] |
| 242 | HUF_decompress1X* | single-stream (X1 fast / X2 fallback) | [x] |
| 243 | HUF_decompress4X* | quad-stream (X1 / X2), asm vs C (disableHuffmanAssembly) | [x] |
| 244 | HUF_readStats | parse weight header, incl. FSE-coded vs raw weights | [x] |
| 245 | HUF_readDTableX1 | build 1-symbol-per-entry decode table | [x] |
| 246 | HUF_readDTableX2 | build 2-symbol-per-entry decode table | [x] |

## xxhash (ZSTD_-namespaced) — xxhash.h

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 247 | ZSTD_XXH32 | one-shot 32-bit, len 0 / <16 / aligned / unaligned | [x] |
| 248 | ZSTD_XXH64 | one-shot 64-bit, len 0 / <32 / large | [x] |
| 249 | ZSTD_XXH64_reset / update / digest | streaming, partial buffer accumulation | [x] |
| 250 | ZSTD_XXH64_canonicalFromHash / hashFromCanonical | endian-canonical roundtrip | [x] |

## Dictionary builder (ZDICT) — zdict.h / zdict.c / cover.c / fastcover.c

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 251 | ZDICT_trainFromBuffer | default (fastCover under the hood) | [x] |
| 252 | ZDICT_trainFromBuffer_cover | COVER with explicit ZDICT_cover_params_t | [x] |
| 253 | ZDICT_optimizeTrainFromBuffer_cover | COVER param optimization loop | [x] |
| 254 | ZDICT_trainFromBuffer_fastCover | fastCover with params | [x] |
| 255 | ZDICT_optimizeTrainFromBuffer_fastCover | fastCover param optimization | [x] |
| 256 | ZDICT_trainFromBuffer_legacy | legacy trainer + ZDICT_legacy_params_t | [x] |
| 257 | ZDICT_finalizeDictionary | raw content + entropy tables → full dict | [x] |
| 258 | ZDICT_getDictID | dictID from dict buffer (0 if raw) | [x] |
| 259 | ZDICT_getDictHeaderSize | header size of a zstd dict | [x] |

## Deprecated ZBUFF — deprecated/zbuff.h

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 260 | ZBUFF_compressInit / _usingDict | legacy streaming compress init | [x] |
| 261 | ZBUFF_compressContinue / _flush / _end | legacy streaming compress cycle | [x] |
| 262 | ZBUFF_decompressInit / _usingDict | legacy streaming decompress init | [x] |
| 263 | ZBUFF_decompressContinue | legacy streaming decompress cycle | [x] |

## Legacy decoders v01–v07 — legacy/zstd_legacy.h (reached via ZSTD_decompress on legacy magic)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 264 | ZSTD_decompress (legacy dispatch) | v01 magic frame (ZSTDv01_decompress) | [x] |
| 265 | ZSTD_decompress (legacy dispatch) | v02 magic frame (ZSTDv02_decompress) | [x] |
| 266 | ZSTD_decompress (legacy dispatch) | v03 magic frame (ZSTDv03_decompress) | [x] |
| 267 | ZSTD_decompress (legacy dispatch) | v04 magic frame (ZSTDv04_decompress) | [x] |
| 268 | ZSTD_decompress (legacy dispatch) | v05 magic frame (ZSTDv05_decompress) | [x] |
| 269 | ZSTD_decompress (legacy dispatch) | v06 magic frame (ZSTDv06_decompress) | [x] |
| 270 | ZSTD_decompress (legacy dispatch) | v07 magic frame (ZSTDv07_decompress) | [x] |
| 271 | ZSTD_decompressStream (legacy dispatch) | streaming legacy frame decode | [x] |
| 272 | ZSTD_getFrameContentSize (legacy) | content size query on legacy frame | [x] |
