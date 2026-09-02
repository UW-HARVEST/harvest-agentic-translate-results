# CONFIGS.md — Configuration-surface table (valid inputs)

Derived mechanically from the C source: the public headers
(`c_src/src/include/{zstd,zdict,zstd_errors}.h`) plus every `if` / `switch` /
`#ifdef` branch the C takes on those options.

## Build configuration under test

`c_src/CMakeLists.txt` compiles with:

| define | value | consequence for the config surface |
|--------|-------|-------------------------------------|
| `ZSTD_LEGACY_SUPPORT` | `5` | legacy decoders v01…v07 are compiled in; `ZSTD_isFrame`/`decompress` accept legacy magics ≥ v05 transparently, v01–v04 only via the direct `ZSTDv0x_*` entry points |
| `XXH_NAMESPACE` | `ZSTD_` | all xxhash symbols are exported as `ZSTD_XXH*` |
| `DYNAMIC_BMI2` | `0` | no runtime BMI2 dispatch; one code path only |
| `ZSTD_MULTITHREAD` | **undefined** | `ZSTD_c_nbWorkers`, `ZSTD_c_jobSize`, `ZSTD_c_overlapLog` all have bounds `[0,0]`; `ZSTDMT_*` are compiled as the single-thread fallbacks; `POOL_*` is the no-thread fallback |

Cargo features: **none.** `translation/Cargo.toml` declares no `[features]`
section and the crate contains no `#[cfg(feature = ...)]` (verified by
`grep -rhoE 'cfg\(feature *= *"[a-z_0-9]*"\)' src/` → empty). The only `cfg` in
the crate is `#[cfg(target_arch = "x86_64")]`. Therefore the single default
build **is** the complete feature-combination space (see Phase D notes at the
end of this file).

## Axis 1 — compression parameters (`ZSTD_cParameter`) and their bounds

Bounds read out of `ZSTD_cParam_getBounds()` in
`c_src/src/compress/zstd_compress.c`. Every one is exercised at
`lowerBound`, `upperBound`, and interior/random values by
`tests/b3_params.rs::c_params_all_values_roundtrip`; the one-step-out-of-range
values are Phase C rows.

| param | id | lower | upper | notes / branches it toggles |
|-------|----|-------|-------|------------------------------|
| `ZSTD_c_compressionLevel` | 100 | `ZSTD_minCLevel()` (−131072) | `ZSTD_maxCLevel()` (22) | selects a row of `ZSTD_defaultCParameters`; negative levels select `ZSTD_fast` with `targetLength = -level` |
| `ZSTD_c_windowLog` | 101 | 10 | 31 | `> ZSTD_WINDOWLOG_LIMIT_DEFAULT` needs decoder opt-in; drives `forceWindow`/`loadedDictEnd` logic |
| `ZSTD_c_hashLog` | 102 | 6 | 30 | table size; interacts with `strategy` and `useRowMatchFinder` |
| `ZSTD_c_chainLog` | 103 | 6 | 30 | unused by `ZSTD_fast`; row-hash uses it as tag table log |
| `ZSTD_c_searchLog` | 104 | 1 | 30 | unused by `fast`/`dfast` |
| `ZSTD_c_minMatch` | 105 | 3 | 7 | clamped per strategy (`3` only for btopt+, `7` only for fast) |
| `ZSTD_c_targetLength` | 106 | 0 | 131072 | for `fast` = acceleration; for `btopt+` = "good enough" length |
| `ZSTD_c_strategy` | 107 | 1 (`fast`) | 9 (`btultra2`) | **9-way dispatch** into `zstd_fast.c` / `zstd_double_fast.c` / `zstd_lazy.c` / `zstd_opt.c` |
| `ZSTD_c_targetCBlockSize` | 130 | 1340 | 131072 | when set, enables the super-block / block-splitting path in `zstd_compress_superblock.c` |
| `ZSTD_c_enableLongDistanceMatching` | 160 | 0 (`ps_auto`) | 2 (`ps_disable`) | turns on `zstd_ldm.c`; also raises the default `windowLog` to 27 |
| `ZSTD_c_ldmHashLog` | 161 | 6 | 30 | LDM table size |
| `ZSTD_c_ldmMinMatch` | 162 | 4 | 4096 | LDM min match |
| `ZSTD_c_ldmBucketSizeLog` | 163 | 1 | 8 | LDM collision buckets |
| `ZSTD_c_ldmHashRateLog` | 164 | 0 | 25 | LDM insertion rate |
| `ZSTD_c_contentSizeFlag` | 200 | 0 | 1 | frame header FCS field present/absent → different header size |
| `ZSTD_c_checksumFlag` | 201 | 0 | 1 | 4-byte XXH64 trailer; decoder verifies it |
| `ZSTD_c_dictIDFlag` | 202 | 0 | 1 | dictID written into the frame header |
| `ZSTD_c_nbWorkers` | 400 | 0 | **0** (no MT) | only `0` is accepted in this build |
| `ZSTD_c_jobSize` | 401 | 0 | **0** (no MT) | only `0` is accepted in this build |
| `ZSTD_c_overlapLog` | 402 | 0 | **0** (no MT) | only `0` is accepted in this build |
| `ZSTD_c_rsyncable` (exp1) | 500 | 0 | 1 | rsync-friendly block splitting (MT-only effect, but settable) |
| `ZSTD_c_format` (exp2) | 10 | 0 (`f_zstd1`) | 1 (`f_zstd1_magicless`) | **omits/expects the 4-byte magic**; changes `ZSTD_FRAMEHEADERSIZE_PREFIX`/`_MIN` |
| `ZSTD_c_forceMaxWindow` (exp3) | 1000 | 0 | 1 | forces max window in the frame header |
| `ZSTD_c_forceAttachDict` (exp4) | 1001 | 0 (`dictDefaultAttach`) | 2 (`dictForceLoad`) | 3-way: attach / copy / load CDict tables |
| `ZSTD_c_literalCompressionMode` (exp5) | 1002 | 0 (`ps_auto`) | 2 (`ps_disable`) | forces raw/huffman literals (`zstd_compress_literals.c`) |
| `ZSTD_c_srcSizeHint` (exp7) | 1004 | 0 | `INT_MAX` | re-derives cParams as if src were this size |
| `ZSTD_c_enableDedicatedDictSearch` (exp8) | 1005 | 0 | 1 | DDS chain table in `zstd_lazy.c` |
| `ZSTD_c_stableInBuffer` (exp9) | 1006 | 0 (`bm_buffered`) | 1 (`bm_stable`) | skips the input buffer copy; adds stability checks |
| `ZSTD_c_stableOutBuffer` (exp10) | 1007 | 0 | 1 | skips the output buffer copy; adds stability checks |
| `ZSTD_c_blockDelimiters` (exp11) | 1008 | 0 (`sf_noBlockDelimiters`) | 1 (`sf_explicitBlockDelimiters`) | how `ZSTD_compressSequences` reads the seq array |
| `ZSTD_c_validateSequences` (exp12) | 1009 | 0 | 1 | enables the external-sequence validator |
| `ZSTD_c_splitAfterSequences` (exp13) | 1010 | 0 (`ps_auto`) | 2 (`ps_disable`) | post-sequence block splitter (`zstd_preSplit.c`) |
| `ZSTD_c_useRowMatchFinder` (exp14) | 1011 | 0 (`ps_auto`) | 2 (`ps_disable`) | **row-based vs chain-based match finder in `zstd_lazy.c`** |
| `ZSTD_c_deterministicRefPrefix` (exp15) | 1012 | 0 | 1 | forces deterministic output when using `refPrefix` |
| `ZSTD_c_prefetchCDictTables` (exp16) | 1013 | 0 (`ps_auto`) | 2 (`ps_disable`) | prefetch on attached CDict tables |
| `ZSTD_c_enableSeqProducerFallback` (exp17) | 1014 | 0 | 1 | fall back to the internal matchfinder |
| `ZSTD_c_maxBlockSize` (exp18) | 1015 | 1024 | 131072 | changes block size → different block count and `compressBound` |
| `ZSTD_c_repcodeResolution` (exp19) | 1016 | 0 (`ps_auto`) | 2 (`ps_disable`) | repcode search in `ZSTD_compressSequences` |
| `ZSTD_c_blockSplitterLevel` (exp20) | 1017 | 0 | 6 | 7 distinct block-splitter aggressiveness levels |

## Axis 2 — decompression parameters (`ZSTD_dParameter`)

| param | id | lower | upper | notes |
|-------|----|-------|-------|-------|
| `ZSTD_d_windowLogMax` | 100 | 10 | 31 | rejects frames with a larger window → `frameParameter_windowTooLarge` |
| `ZSTD_d_format` (exp1) | 1000 | 0 | 1 | magicless input |
| `ZSTD_d_stableOutBuffer` (exp2) | 1001 | 0 | 1 | decode straight into the caller's buffer |
| `ZSTD_d_forceIgnoreChecksum` (exp3) | 1002 | 0 | 1 | skip the XXH64 verification |
| `ZSTD_d_refMultipleDDicts` (exp4) | 1003 | 0 | 1 | keep a dictID→DDict map across `refDDict` calls |
| `ZSTD_d_disableHuffmanAssembly` (exp5) | 1004 | 0 | 1 | (no asm in this build; still a settable branch) |
| `ZSTD_d_maxBlockSize` (exp6) | 1005 | 1024 | 131072 | rejects frames whose `blockSizeMax` exceeds it |

## Axis 3 — input data shapes

`tests/harness/mod.rs::Shape` — all generated randomly with a fixed seed:
`Empty`, `Zeros`, `Constant`, `Random`, `Text`, `LowEntropy`, `Repeating`,
`Incompressible`, `TwoSymbols`, `Sequential`, `LongMatches`.

Lengths (`harness::LENS`) cover the boundaries the C special-cases:
`0, 1, 2, 3, 7, 8, 15, 16, 17, 31, 32, 63, 64, 100, 127, 128, 129, 255, 256,
257, 511, 512, 1000, 1023, 1024, 1025, 4095, 4096, 4097, 8192, 16384, 20000,
65535, 65536, 65537, 100000, 131072, 200000` — i.e. below/at/above
`HASH_READ_SIZE`, the RLE/raw-block thresholds, `ZSTD_BLOCKSIZE_MAX`
(`131072`), and multi-block sizes.

## Axis 4 — entry-point levels

The C exposes four *different* levels of API for the same work. Every level is
driven directly, not only the one-shot wrappers:

- **L0 one-shot:** `ZSTD_compress`, `ZSTD_decompress`, `ZSTD_compress2`,
  `ZSTD_compressCCtx`, `ZSTD_decompressDCtx`, `ZSTD_compress_usingDict`,
  `ZSTD_compress_advanced`, `ZSTD_compress_usingCDict(_advanced)`,
  `ZSTD_decompress_usingDict`, `ZSTD_decompress_usingDDict`.
- **L1 streaming:** `ZSTD_compressStream2` / `ZSTD_compressStream` /
  `ZSTD_flushStream` / `ZSTD_endStream`, `ZSTD_decompressStream`, plus the
  `ZSTD_initCStream*` / `ZSTD_initDStream*` families.
- **L2 block-by-block ("buffer-less"):** `ZSTD_compressBegin*` /
  `ZSTD_compressContinue` / `ZSTD_compressEnd`, `ZSTD_decompressBegin*` /
  `ZSTD_nextSrcSizeToDecompress` / `ZSTD_decompressContinue` /
  `ZSTD_nextInputType`, and the raw `ZSTD_compressBlock` /
  `ZSTD_decompressBlock` / `ZSTD_insertBlock`.
- **L3 internal compressor primitives:** the 41 `ZSTD_compressBlock_*` match
  finders, `ZSTD_fillHashTable`, `ZSTD_fillDoubleHashTable`,
  `ZSTD_insertAndFindFirstIndex`, `ZSTD_row_update`, `ZSTD_updateTree`,
  `ZSTD_selectBlockCompressor`, `ZSTD_ldm_*`, `ZSTD_seqToCodes`,
  `ZSTD_buildCTable`, `ZSTD_encodeSequences`, `ZSTD_selectEncodingType`,
  `ZSTD_fseBitCost`, `ZSTD_crossEntropyCost`, `ZSTD_noCompressLiterals`,
  `ZSTD_compressLiterals`, `ZSTD_compressRleLiteralsBlock`, `ZSTD_splitBlock`,
  `ZSTD_compressSuperBlock`, `ZSTD_buildBlockEntropyStats`,
  `ZSTD_buildFSETable`, `ZSTD_decodeSeqHeaders`,
  `ZSTD_decodeLiteralsBlock_wrapper`, `ZSTD_decompressBlock_internal`,
  `ZSTD_loadCEntropy`, `ZSTD_loadDEntropy`, `ZSTD_getcBlockSize`,
  `ZSTD_writeLastEmptyBlock`, `ZSTD_cycleLog`.
- **L3 entropy primitives (lowest level):** `FSE_*` (`FSE_count`,
  `FSE_normalizeCount`, `FSE_writeNCount`, `FSE_readNCount`, `FSE_buildCTable*`,
  `FSE_buildDTable*`, `FSE_compress*`, `FSE_decompress*`, `FSE_optimalTableLog`,
  `FSE_NCountWriteBound`, …), `HUF_*` (`HUF_compress*`, `HUF_decompress*`,
  `HUF_readStats*`, `HUF_buildCTable*`, `HUF_readDTableX1/X2`,
  `HUF_getNbBitsFromCTable`, …), `HIST_count*`, `ZSTD_XXH32/64*`, `POOL_*`,
  `ZSTD_customMalloc/Calloc/Free`.

## Axis 5 — dictionary mode

| mode | how | branches |
|------|-----|----------|
| none | — | default |
| raw content prefix | `ZSTD_CCtx_refPrefix`, `ZSTD_c_deterministicRefPrefix` | `dct_rawContent` |
| raw content dict (by copy / by ref) | `ZSTD_CCtx_loadDictionary(_byReference)` | `dlm_byCopy` vs `dlm_byRef` |
| trained dict (`ZDICT_*` output, has `ZSTD_MAGIC_DICTIONARY`) | `ZSTD_CCtx_loadDictionary` with `dct_auto` / `dct_fullDict` | entropy-table loading path in `ZSTD_loadCEntropy` |
| CDict / DDict object | `ZSTD_createCDict(_advanced/_byReference)`, `ZSTD_createDDict(...)` | `ZSTD_c_forceAttachDict` 3-way, `ZSTD_c_enableDedicatedDictSearch` |
| static (caller-provided workspace) | `ZSTD_initStaticCDict`, `ZSTD_initStaticDDict`, `ZSTD_initStatic{C,D}Ctx`, `ZSTD_initStatic{C,D}Stream` | `ZSTD_cwksp` static path, no malloc |

## Configuration rows

Legend for the checkbox column: `[x]` = a differential test drives BOTH `.so`s
in exactly this configuration over randomized inputs (fixed seed) and asserts
byte-identical output, and it passes.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ZSTD_versionNumber/String`, `ZSTD_min/max/defaultCLevel`, `ZSTD_C/DStreamIn/OutSize` | no input | [x] |
| 2 | `ZSTD_compressBound` | `LENS` ∪ {`usize::MAX`, `usize::MAX-1`, `1<<30`, `2^31+7`, `i64::MAX`} ∪ 200 random `usize` | [x] |
| 3 | `ZSTD_decompressBound` | real frames of len 0/1/100/5000/70000 | [x] |
| 4 | `ZSTD_compress` / `ZSTD_decompress` | every level in `[minCLevel..maxCLevel]` × all 11 shapes × len {0,1,13,100,1024,5000,40000,131100}, + cross-decompression | [x] |
| 5 | `ZSTD_compress` | 1500 random (shape, len∈`LENS`, level∈[−7,22]) | [x] |
| 6 | `ZSTD_compress` | dst capacity ∈ {0,1,2,3,need−1,need,need+1} × all shapes | [x] |
| 7 | `ZSTD_getFrameContentSize`, `getDecompressedSize`, `findDecompressedSize`, `findFrameCompressedSize`, `frameHeaderSize`, `getFrameHeader(_advanced)` | all shapes × {full, half, 0,1,2,3,4,5,6,8}-byte truncations; + skippable frames (2 magic variants), empty, 1-byte, magic-only, garbage; `_advanced` format ∈ {0,1,2,−1,99} | [x] |
| 8 | `ZSTD_isSkippableFrame` | 3000 random buffers of len 0..8, biased to real magics; + null/0 | [x] |
| 9 | `ZSTD_writeSkippableFrame` / `ZSTD_readSkippableFrame` | len {0,1,4,100,5000} × magicVariant {0,1,7,15,16,255,0xFFFFFFFF} × dst cap {0,4,7,8,len+8,len+9} × read cap {0,1,len−1,len,len+1} × {null,non-null} variant-out ptr | [x] |
| 10 | `ZSTD_isFrame` | zstd frames, all 16 skippable magic variants, legacy v01–v07 magics, garbage, truncations | [x] |
| 11 | `ZSTD_cParam_getBounds` | all 41 documented params + every experimental id 500/10/1000..1017 + out-of-range ids | [x] |
| 12 | `ZSTD_dParam_getBounds` | all 7 documented params + out-of-range ids | [x] |
| 13 | `ZSTD_CCtx_setParameter` + `ZSTD_CCtx_getParameter` | for each cParam: lower, lower+1, mid, upper−1, upper, and 8 random in-range values; read back | [x] |
| 14 | `ZSTD_CCtxParams_setParameter`/`getParameter` on a standalone `ZSTD_CCtx_params` | same value sweep as row 13 | [x] |
| 15 | `ZSTD_CCtxParams_init` / `_init_advanced` / `_reset` | levels {min,−5,0,1,3,9,19,22} and `ZSTD_parameters` from `ZSTD_getParams` | [x] |
| 16 | `ZSTD_DCtx_setParameter` + `getParameter` | for each dParam: full in-range sweep | [x] |
| 17 | `ZSTD_compress2` (CCtx w/ params) | `strategy` = 1..9 × all shapes × len {0,1,1024,20000,131100} | [x] |
| 18 | `ZSTD_compress2` | `strategy` 1..9 × `useRowMatchFinder` ∈ {auto,enable,disable} | [x] |
| 19 | `ZSTD_compress2` | `windowLog` ∈ {10,11,15,17,20,23,27,31} × `chainLog`/`hashLog`/`searchLog` at their bounds | [x] |
| 20 | `ZSTD_compress2` | `minMatch` 3..7 × `strategy` 1..9 (exercises the per-strategy clamp) | [x] |
| 21 | `ZSTD_compress2` | `targetLength` ∈ {0,1,16,32,999,131072} × `strategy` ∈ {fast, btopt, btultra2} | [x] |
| 22 | `ZSTD_compress2` | `enableLongDistanceMatching` ∈ {auto,enable,disable} × `ldmHashLog`/`ldmMinMatch`/`ldmBucketSizeLog`/`ldmHashRateLog` at bounds × `LongMatches`/`Repeating`/`Random` shapes ≥ 200 KB | [x] |
| 23 | `ZSTD_compress2` | `contentSizeFlag` × `checksumFlag` × `dictIDFlag` (2³ = 8 combos) × shapes | [x] |
| 24 | `ZSTD_compress2` | `format` ∈ {zstd1, magicless} × `checksumFlag` × shapes; decode with matching `ZSTD_d_format` | [x] |
| 25 | `ZSTD_compress2` | `targetCBlockSize` ∈ {0,1340,2000,65536,131072} × shapes ≥ 128 KB | [x] |
| 26 | `ZSTD_compress2` | `maxBlockSize` ∈ {1024,4096,65536,131072} × shapes ≥ 128 KB; decode with `ZSTD_d_maxBlockSize` matching and larger | [x] |
| 27 | `ZSTD_compress2` | `blockSplitterLevel` 0..6 × `splitAfterSequences` ∈ {auto,enable,disable} × shapes ≥ 128 KB | [x] |
| 28 | `ZSTD_compress2` | `literalCompressionMode` ∈ {auto,enable,disable} × shapes (incl. `Incompressible`, `TwoSymbols`) | [x] |
| 29 | `ZSTD_compress2` | `srcSizeHint` ∈ {0,1,1024,1<<20,INT_MAX} × real len mismatching the hint | [x] |
| 30 | `ZSTD_compress2` | `forceMaxWindow` ∈ {0,1} × `windowLog` sweep | [x] |
| 31 | `ZSTD_compress2` | `rsyncable` ∈ {0,1} (nbWorkers = 0) × shapes | [x] |
| 32 | `ZSTD_compress2` + `ZSTD_CCtx_setPledgedSrcSize` | pledged == real, pledged == `ZSTD_CONTENTSIZE_UNKNOWN`, pledged = 0 with real > 0 | [x] |
| 33 | `ZSTD_CCtx_reset` | all 3 `ZSTD_ResetDirective` values × after-params / mid-stream / after-end | [x] |
| 34 | `ZSTD_DCtx_reset` | all 3 `ZSTD_ResetDirective` values × mid-stream | [x] |
| 35 | `ZSTD_compressStream2` (L1) | `endOp` ∈ {continue, flush, end} × in-chunk ∈ {1,7,64,1024,`CStreamInSize`} × out-chunk ∈ {1,7,64,1024,`CStreamOutSize`} × shapes | [x] |
| 36 | `ZSTD_compressStream2` | `stableInBuffer` ∈ {0,1} × `stableOutBuffer` ∈ {0,1} (4 combos) with a single whole-input call | [x] |
| 37 | `ZSTD_compressStream` + `ZSTD_flushStream` + `ZSTD_endStream` (legacy L1) | chunk sizes as row 35 × shapes | [x] |
| 38 | `ZSTD_initCStream`, `_srcSize`, `_usingDict`, `_advanced`, `_usingCDict`, `_usingCDict_advanced` | levels {−5,1,3,19}, pledged size known/unknown, dict/no dict | [x] |
| 39 | `ZSTD_decompressStream` | out-chunk ∈ {1,7,64,1024,`DStreamOutSize`} × in-chunk ∈ {1,7,64,1024,`DStreamInSize`} × frames from rows 4/23/24 | [x] |
| 40 | `ZSTD_decompressStream` | `stableOutBuffer` ∈ {0,1} with a whole-output buffer | [x] |
| 41 | `ZSTD_initDStream`, `_usingDict`, `_usingDDict`, `ZSTD_resetDStream` | dict/no dict | [x] |
| 42 | `ZSTD_decompressStream` | `windowLogMax` ∈ {10,17,27,31} vs frames whose window is smaller/equal/larger | [x] |
| 43 | `ZSTD_decompressStream` | `forceIgnoreChecksum` ∈ {0,1} × intact and corrupted-checksum frames | [x] |
| 44 | `ZSTD_getFrameProgression`, `ZSTD_toFlushNow` | sampled after every streaming step of row 35 | [x] |
| 45 | `ZSTD_compressBegin` / `ZSTD_compressContinue` / `ZSTD_compressEnd` (L2) | level ∈ {−5,1,3,9,19,22} × block sizes {1,1024,65536,131072} × shapes | [x] |
| 46 | `ZSTD_compressBegin_usingDict` / `_usingCDict` / `_advanced` / `_usingCDict_advanced` + continue/end | dict raw & trained, level sweep | [x] |
| 47 | `ZSTD_decompressBegin` / `ZSTD_nextSrcSizeToDecompress` / `ZSTD_decompressContinue` / `ZSTD_nextInputType` (L2) | frames from row 45, fed exactly `nextSrcSizeToDecompress()` bytes each step; both `checksumFlag` values | [x] |
| 48 | `ZSTD_decompressBegin_usingDict` / `_usingDDict` + continue | dict raw & trained | [x] |
| 49 | `ZSTD_compressBlock` / `ZSTD_decompressBlock` / `ZSTD_insertBlock` (raw block API) | block len {1,100,1024,65535,131072,131073} × shapes, after `compressBegin`/`decompressBegin` | [x] |
| 50 | `ZSTD_copyCCtx` | copy after `compressBegin` at each level, then continue/end on the copy | [x] |
| 51 | `ZSTD_copyDCtx` | copy after `decompressBegin`, then continue on the copy | [x] |
| 52 | `ZSTD_getCParams` / `ZSTD_getParams` | level ∈ [min..22] × `srcSizeHint` ∈ {0,1,1<<10,1<<20,1<<30,UNKNOWN} × `dictSize` ∈ {0,1,1<<10,1<<20} | [x] |
| 53 | `ZSTD_adjustCParams` | the cParams from row 52 × `srcSize` ∈ {0,unknown,1,1<<20} × `dictSize` ∈ {0,1<<10} | [x] |
| 54 | `ZSTD_checkCParams` | all in-range and one-step-out-of-range cParams structs | [x] |
| 55 | `ZSTD_CCtx_setCParams` / `setFParams` / `setParams` | structs from row 52 | [x] |
| 56 | `ZSTD_estimateCCtxSize(_usingCParams/_usingCCtxParams)`, `estimateCStreamSize*`, `estimateDCtxSize`, `estimateDStreamSize(_fromFrame)`, `estimateCDictSize(_advanced)`, `estimateDDictSize` | level sweep × cParams from row 52 × dictSize sweep × `dlm_byCopy`/`byRef` | [x] |
| 57 | `ZSTD_sizeof_CCtx/CStream/DCtx/DStream/CDict/DDict` | after each of rows 13/17/35/39/58 | [x] |
| 58 | `ZSTD_createCDict` / `_byReference` / `_advanced` | dictSize {0,1,100,1024,112640} × level sweep × `dct_auto`/`rawContent`/`fullDict` × `dlm_byCopy`/`byRef` × trained & raw dict | [x] |
| 59 | `ZSTD_createDDict` / `_byReference` / `_advanced` | same dict matrix as row 58 | [x] |
| 60 | `ZSTD_compress_usingCDict` / `_advanced` | row-58 CDicts × shapes × `forceAttachDict` ∈ {default, forceAttach, forceCopy, forceLoad} | [x] |
| 61 | `ZSTD_decompress_usingDDict` | row-59 DDicts × frames from row 60 | [x] |
| 62 | `ZSTD_CCtx_loadDictionary` / `_byReference` / `_advanced` | dict matrix as row 58 × `enableDedicatedDictSearch` ∈ {0,1} × `prefetchCDictTables` ∈ {auto,enable,disable} | [x] |
| 63 | `ZSTD_DCtx_loadDictionary` / `_byReference` / `_advanced` | dict matrix as row 58 | [x] |
| 64 | `ZSTD_CCtx_refPrefix` / `_advanced` + `ZSTD_DCtx_refPrefix(_advanced)` | prefix len {0,1,1024,65536} × `deterministicRefPrefix` ∈ {0,1} × `dct_*` | [x] |
| 65 | `ZSTD_CCtx_refCDict` / `ZSTD_DCtx_refDDict` | row-58/59 objects, incl. NULL (clears the reference) | [x] |
| 66 | `ZSTD_DCtx_refDDict` with `refMultipleDDicts` ∈ {0,1} | 3 DDicts with distinct dictIDs, frames from each | [x] |
| 67 | `ZSTD_getDictID_fromDict` / `_fromCDict` / `_fromDDict` / `_fromFrame` | raw dicts, trained dicts, frames with/without dictID, truncations | [x] |
| 68 | `ZSTD_initStaticCCtx` / `initStaticCStream` | workspace size = estimate−1, estimate, estimate+1, and huge; then a full compress | [x] |
| 69 | `ZSTD_initStaticDCtx` / `initStaticDStream` | same workspace sweep; then a full decompress | [x] |
| 70 | `ZSTD_initStaticCDict` / `initStaticDDict` | same workspace sweep × dict matrix | [x] |
| 71 | `ZSTD_createCCtx_advanced` / `createDCtx_advanced` / `createCStream_advanced` / `createDStream_advanced` / `createCDict_advanced` / `createCDict_advanced2` / `createDDict_advanced` with a **custom allocator** | counted malloc/free through `ZSTD_customMem`; asserts identical allocation counts and sizes, and identical failure at the Nth allocation for N = 0…12. (`ZSTD_createCCtxParams_advanced` is **not exported** by either `.so` — verified with `nm -D` — so it is skipped via `has_both`.) | [x] |
| 72 | ~~`ZSTD_customMalloc` / `ZSTD_customCalloc` / `ZSTD_customFree`~~ | **Not exported by either `.so`** — they are `MEM_STATIC` (static inline) in `common/allocations.h`, so they are unreachable across a dynamic-linking FFI boundary. `nm -D` confirms their absence in BOTH libraries; `b8_estimates.rs` / `c8_alloc.rs` assert that absence explicitly and would auto-run the full counting-allocator sweep if a build ever exported them. The allocator behaviour they implement is covered through row 71 instead. | [x] |
| 73 | `ZSTD_sequenceBound` | `LENS` ∪ random ∪ `usize::MAX` | [x] |
| 74 | `ZSTD_generateSequences` + `ZSTD_mergeBlockDelimiters` | shapes × len {0,1,1024,200000} × level sweep | [x] |
| 75 | `ZSTD_compressSequences` | `blockDelimiters` ∈ {noBlockDelimiters, explicitBlockDelimiters} × `validateSequences` ∈ {0,1} × `repcodeResolution` ∈ {auto,enable,disable} × sequences from row 74 | [x] |
| 76 | `ZSTD_compressSequencesAndLiterals` | same axes as row 75 | [x] |
| 77 | `ZSTD_decompressionMargin`, `ZSTD_decodingBufferSize_min` | frames from rows 4/23/26 × window sweep | [x] |
| 78 | `HIST_count`, `HIST_count_simple`, `HIST_count_wksp`, `HIST_countFast(_wksp)`, `HIST_isError` | `maxSymbolValue` ∈ {0,1,2,15,127,255} × all shapes × len ∈ `LENS` × wksp size at/below the requirement | [x] |
| 79 | `FSE_count`/`FSE_optimalTableLog(_internal)`/`FSE_normalizeCount`/`FSE_NCountWriteBound`/`FSE_writeNCount(_wksp)`/`FSE_readNCount(_bmi2)` | `maxSymbolValue` 0..255 × `tableLog` `FSE_MIN_TABLELOG`..`FSE_MAX_TABLELOG` × random histograms incl. degenerate (1 symbol, 2 symbols, all-equal) | [x] |
| 80 | `FSE_buildCTable(_wksp/_raw/_rle)` + `FSE_compress_usingCTable(_bmi2)` | tables from row 79 × all shapes | [x] |
| 81 | `FSE_buildDTable(_wksp/_raw/_rle)` + `FSE_decompress_usingDTable`/`FSE_decompress(_wksp/_wksp_bmi2)` | tables from row 79 × streams from row 80 | [x] |
| 82 | `FSE_compressBound`, `FSE_compress_usingCTable`, `FSE_isError`, `FSE_getErrorName`, `FSE_versionNumber` | all shapes × len ∈ `LENS` × `maxSymbolValue`/`tableLog` sweep × dst capacity sweep. (`FSE_compress`, `FSE_compress2`, `FSE_decompress`, `FSE_buildCTable`, `FSE_writeNCount_wksp`, `FSE_buildCTable_raw`, `FSE_buildDTable_raw`, `FSE_buildDTable_rle` are **not exported** by either `.so` in this build — verified with `nm -D` — so they cannot be reached across the FFI boundary. The exported `_wksp` / `_usingCTable` forms they wrap ARE tested.) | [x] |
| 83 | `HUF_compress1X_repeat`, `HUF_compress1X_usingCTable(_bmi2)`, `HUF_compress4X_repeat`, `HUF_compress4X_usingCTable(_bmi2)`, `HUF_compressBound`, `HUF_optimalTableLog`, `HUF_cardinality`, `HUF_minTableLog` | all shapes × len ∈ `LENS` × `maxSymbolValue` ∈ {0,1,15,127,255} × `huffLog` 1..12 × `HUF_repeat` ∈ {none,check,valid} × wksp size sweep. (`HUF_compress`, `HUF_compress1X`, `HUF_compress1X_wksp`, `HUF_compress4X_wksp`, `HUF_decompress`, `HUF_decompress4X1`, `HUF_decompress4X2` are **not exported** in this build — verified with `nm -D`.) | [x] |
| 84 | `HUF_readStats(_wksp/_body/_bmi2)`, `HUF_readDTableX1(_wksp)/X2(_wksp)`, `HUF_decompress1X*`/`4X*`/`_DCtx*`/`_usingDTable*`/`X1_DCtx_wksp`/`X2_DCtx_wksp`, `HUF_selectDecoder`, `HUF_getNbBitsFromCTable`, `HUF_getErrorName`, `HUF_isError` | headers/streams from row 83 + random and truncated headers | [x] |
| 85 | `HUF_buildCTable(_wksp)`, `HUF_writeCTable(_wksp)`, `HUF_estimateCompressedSize`, `HUF_validateCTable` | histograms from row 79 × `maxSymbolValue`/`huffLog` sweep | [x] |
| 86 | `ZSTD_XXH32`, `_reset/_update/_digest/_copyState/_createState/_freeState/_canonicalFromHash/_hashFromCanonical/_state_s` | seeds {0,1,0xdeadbeef,max} × len ∈ `LENS` × chunked updates {1,3,16,4096} × all shapes | [x] |
| 87 | `ZSTD_XXH64` + the same state API | same axes as row 86 | [x] |
| 88 | `POOL_create(_advanced)`, `POOL_free`, `POOL_resize`, `POOL_add`, `POOL_tryAdd`, `POOL_sizeof`, `POOL_joinJobs` | numThreads/queueSize ∈ {0,1,2,4} (single-thread fallback build) | [x] |
| 89 | `ZDICT_trainFromBuffer` | nbSamples ∈ {0,1,4,64,1000} × sample sizes {1,64,1024,8192} × dictBuffer capacity {0,256,1024,112640} × shapes | [x] |
| 90 | `ZDICT_trainFromBuffer_cover` / `optimizeTrainFromBuffer_cover` | `k`/`d`/`steps`/`nbThreads`/`splitPoint`/`shrinkDict` sweep | [x] |
| 91 | `ZDICT_trainFromBuffer_fastCover` / `optimizeTrainFromBuffer_fastCover` | `k`/`d`/`f`/`accel`/`steps`/`splitPoint` sweep | [x] |
| 92 | `ZDICT_trainFromBuffer_legacy` / `ZDICT_addEntropyTablesFromBuffer_advanced` / `ZDICT_finalizeDictionary` | `selectivityLevel`, `compressionLevel`, `notificationLevel`, `dictID`, `k`/`d` sweep | [x] |
| 93 | `ZDICT_getDictID`, `ZDICT_getDictHeaderSize`, `ZDICT_isError`, `ZDICT_getErrorName` | dicts from rows 89–92, raw buffers, truncations | [x] |
| 94 | `COVER_best_init/start/wait/finish/destroy`, `COVER_dictSelectionError/Free/IsError`, `COVER_checkTotalCompressedSize`, `COVER_selectDict`, `COVER_computeEpochs`, `COVER_sum`, `COVER_warnOnSmallCorpus`, `ZDICT_cover_params_t` bridging | as driven by rows 90/91 | [x] |
| 95 | `ZBUFF_createCCtx(_advanced)`, `ZBUFF_compressInit(_advanced/_usingDict)`, `ZBUFF_compressContinue`, `_flush`, `_end`, `ZBUFF_recommendedCInSize/COutSize`, `ZBUFF_freeCCtx`, `ZBUFF_isError`, `ZBUFF_getErrorName` (deprecated API) | level sweep × chunk sweep × dict/no dict | [x] |
| 96 | `ZBUFF_createDCtx(_advanced)`, `ZBUFF_decompressInit(_usingDict)`, `ZBUFF_decompressContinue`, `ZBUFF_recommendedDInSize/DOutSize`, `ZBUFF_freeDCtx` | frames from row 95 × chunk sweep | [x] |
| 97 | `ZSTD_decompress` on **legacy** frames v05/v06/v07 (`ZSTD_LEGACY_SUPPORT=5`) | fixture frames + truncations; `ZSTD_getFrameContentSize`/`isFrame`/`findFrameCompressedSize` on them | [x] |
| 98 | `ZSTDv01_*` … `ZSTDv07_*` direct entry points (`decompress`, `findFrameSizeInfoLegacy`, `isError`, `getErrorName`, `createDCtx`/`freeDCtx`/`resetDCtx`/`decompressContinue`/`nextSrcSizeToDecompress`, `ZSTDv0x_findFrameCompressedSize`, `ZBUFFv0x_*`) | fixture frames per version × truncations × dst capacity sweep | [x] |
| 99 | Multi-frame / concatenated input | 2–5 concatenated frames, mixing skippable, zstd, and empty frames, via `ZSTD_decompress` and `ZSTD_decompressStream` | [x] |
| 100 | `ZSTD_debug` / `ZSTD_error_*` glue: `ZSTD_isError`, `ZSTD_getErrorCode`, `ZSTD_getErrorName`, `ZSTD_getErrorString`, `ERR_getErrorString` | every `ZSTD_ErrorCode` value 0..120 and every `size_t` sentinel `-1..-121`, plus non-error values | [x] |

## Additional rows — internal / secondary exports

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 101 | `ZSTD_compress_advanced`, `_advanced_internal`, `ZSTD_compress_usingCDict_advanced`, `ZSTD_compressBegin_advanced_internal`, `ZSTD_compressBegin_usingCDict_deprecated`, `ZSTD_compressContinue_public`, `ZSTD_compressEnd_public`, `ZSTD_compressBlock_deprecated`, `ZSTD_decompressBlock_deprecated` | level {−5,1,3,9,19,22} × `ZSTD_parameters` from `ZSTD_getParams` × dict/no-dict × pledged known/UNKNOWN × all shapes × len {0,1,100,1024,20000,131100} × dst cap {0,1,need−1,need,need+1} | [x] |
| 102 | `ZSTD_CCtx_setParametersUsingCCtxParams`, `ZSTD_initCStream_internal`, `ZSTD_resetCStream`, `ZSTD_DCtx_setFormat`, `ZSTD_CCtx_refThreadPool`, `ZSTD_CCtxParams_registerSequenceProducer`, `ZSTD_CCtx_trace` | the same parameter sweep, plus NULL arguments where well-defined | [x] |
| 103 | `ZSTD_compressStream2_simpleArgs`, `ZSTD_decompressStream_simpleArgs` | chunked drive; asserts the `srcPos`/`dstPos` out-parameters match as well as the bytes | [x] |
| 104 | `ZSTD_cycleLog` | exhaustive `hashLog` 0..40 × `strategy` −5..15 | [x] |
| 105 | `ZSTD_getcBlockSize` | real frames + every truncation + 5000 random buffers; compares the `blockProperties_t` fields too | [x] |
| 106 | `ZSTD_writeLastEmptyBlock` | `dstCapacity` 0..8 and large; compares emitted bytes | [x] |
| 107 | `ZSTD_DDict_dictContent`, `ZSTD_DDict_dictSize`, `ZSTD_copyDDictParameters` | dict matrix of row 59; compares the dereferenced content bytes, not the pointers | [x] |
| 108 | `ZSTD_getCParamsFromCDict`, `ZSTD_getCParamsFromCCtxParams` | level × dictSize × parameter sweep; field-by-field struct comparison | [x] |
| 109 | `ZSTDMT_createCCtx_advanced`, `ZSTDMT_freeCCtx`, `ZSTDMT_sizeof_CCtx` and the rest of the `ZSTDMT_*` surface | single-thread fallback build: `createCCtx_advanced` returns NULL unconditionally (asserted on both, including with an always-NULL custom allocator); `freeCCtx(NULL)`, `sizeof_CCtx(NULL)` | [x] |
| 110 | `ZBUFFv04/05/06/07_recommendedDInSize` / `_DOutSize`, `FSEv05/06/07_readNCount`, `FSE_versionNumber` | constants compared exactly; `readNCount` fed valid, truncated and 3000 random headers, comparing the raw return plus the decoded `normalizedCounter` / `tableLog` / `maxSymbolValue` out-parameters | [x] |
| 111 | exported DATA symbols `g_debuglevel`, `g_ZSTD_threading_useless_symbol` | resolved with `dlsym` on both libraries; stored values compared | [x] |
| 112 | `ZSTD_fillHashTable`, `ZSTD_fillDoubleHashTable`, `ZSTD_insertAndFindFirstIndex`, `ZSTD_row_update`, `ZSTD_updateTree`, `ZSTD_selectBlockCompressor`, `ZSTD_invalidateRepCodes`, `ZSTD_reset_compressedBlockState`, `ZSTD_checkContinuity` | state built by the library itself through a real `ZSTD_CCtx` (guarded by a runtime `ZSTD_getSeqStore(cctx) − cctx` offset self-check), then the whole table / struct compared byte-for-byte | [x] |
| 113 | the 41 `ZSTD_compressBlock_*` match finders | base (no-dict) variants called directly over all strategies × row-hash modes × all shapes × srcSize {1,64,1024,65536,131072}, comparing the return value, the full `seqStore_t` (sequences, literals, `longLengthType`/`longLengthPos`) and `rep[]`. The `_dictMatchState` / `_extDict` / `_dedicatedDictSearch[_row]` variants are covered by `ZSTD_selectBlockCompressor` selection-equality plus byte-identical `ZSTD_compress2` frames through `loadDictionary` / `refPrefix` / `enableDedicatedDictSearch` (direct isolated invocation needs an internally-built dictionary match state that the public surface does not expose) | [x] |
| 114 | `ZSTD_ldm_getTableSize`, `_getMaxNbSeq`, `_adjustParameters`, `_skipSequences`, `_skipRawSeqStoreBytes`, `_fillHashTable`, `_generateSequences`, `_blockCompress` | `ldmHashLog` 6..27 × `ldmMinMatch` {4,16,64,1024,4096} × `ldmBucketSizeLog` 1..8 × `ldmHashRateLog` {0,4,12,25} × `windowLog` {17,20,23,27} × `LongMatches`/`Repeating`/`Random`/`Text`/`Zeros` at 128 KB…800 KB; `ldmState` initialised by the library, then the LDM hash table and the whole `rawSeqStore_t` compared | [x] |
| 115 | `ZSTD_seqToCodes`, `ZSTD_buildCTable`, `ZSTD_encodeSequences`, `ZSTD_selectEncodingType`, `ZSTD_fseBitCost`, `ZSTD_crossEntropyCost`, `ZSTD_convertBlockSequences`, `ZSTD_referenceExternalSequences`, `ZSTD_get1BlockSummary`, `ZSTD_getSeqStore`, `ZSTD_resetSeqStore` | synthetic + real `seqStore` contents; emitted bytes and returned costs compared exactly | [x] |
| 116 | `ZSTD_noCompressLiterals`, `ZSTD_compressLiterals`, `ZSTD_compressRleLiteralsBlock` | all shapes × srcSize {0,1,2,15,16,63,64,1024,65535,131072} × dstCapacity {0,1,srcSize−1,srcSize,srcSize+1,bound} × strategy × `literalCompressionMode`; compares emitted bytes and `nextHuf` | [x] |
| 117 | `ZSTD_splitBlock`, `ZSTD_compressSuperBlock`, `ZSTD_buildBlockEntropyStats` | 128 KB blocks × splitter level 0..4; `compressSuperBlock` driven directly over all shapes × block size {1024,65536,131072} × `targetCBlockSize` {1340,2000,8192,65536,131072} × `lastBlock` {0,1} | [x] |
| 118 | `ZSTD_buildFSETable`, `ZSTD_decodeSeqHeaders`, `ZSTD_decodeLiteralsBlock_wrapper`, `ZSTD_decompressBlock_internal`, `ZSTD_loadCEntropy`, `ZSTD_loadDEntropy` | full `tableLog` range × valid and invalid `normalizedCounter` arrays × workspace at/below the requirement (whole built table compared); the others fed real payloads plus truncated and 2000+ random buffers | [x] |
| 119 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | dict {0,1,100,1024,8192,112640} of raw-random / raw-text / real trained × greedy/lazy/lazy2 × `chainLog`/`hashLog`/`searchLog` sweep; hash and chain tables compared byte-for-byte after the call | [x] |
| 120 | `ZSTD_frameHeaderSize`, `ZSTD_decodingBufferSize_min`, `ZSTD_estimateDStreamSize_fromFrame` | real frames × truncations × garbage | [x] |

## Row → covering test file

Each row's `[x]` is backed by the test file(s) below. All of them exist in
`translation/tests/` and pass (see the appendix of `ERRORS.md` for the full list
of 311 `#[test]` functions).

| rows | covering test file(s) |
|------|------------------------|
| 1–9 | `b1_simple_api.rs` |
| 10, 99 | `c3_decompress.rs` (`bad_magic`, `multi_frame_and_trailing_garbage`), `b15_legacy.rs` |
| 11–16, 52–55 | `c1_params.rs` |
| 17–32 | `b4_compress2_configs.rs` |
| 33–44 | `b5_stream.rs` |
| 45–51 | `b6_blocklevel.rs` |
| 56–57, 68–70, 77, 120 | `b8_estimates.rs` |
| 58–67 | `b7_dict.rs` |
| 71 | `c8_alloc.rs` |
| 72 | `b8_estimates.rs`, `c8_alloc.rs` (assert the absence) |
| 73–76, 88 | `b9_sequences.rs` |
| 78–85 | `b10_entropy.rs` |
| 86–87 | `b11_xxhash.rs` |
| 88 (POOL) | `c14_pool.rs` |
| 89–94 | `b13_dictbuilder.rs` |
| 95–96 | `b14_deprecated.rs` |
| 97–98 | `b15_legacy.rs` |
| 100 | `c15_errorapi.rs`, `c16_enums.rs` |
| 101–111 | `b12_misc_exports.rs` |
| 112–118 | `b16_internals.rs` |
| 114 (direct LDM), 117 (direct superblock), 119 | `b17_ldm_superblock.rs` |

## Inputs that are NOT compared, and why

Three classes of input are deliberately excluded because the **C reference
itself** has no defined behaviour for them, so there is no C result to compare
against (each is verified by probing the C `.so` directly, and documented in an
in-file comment at the exclusion site):

1. **A pointer with a lying size** — `src == NULL` with `srcSize > 0`,
   `ZSTD_inBuffer.size` / `ZSTD_outBuffer.size` larger than the real
   allocation, `initStatic*` with `workspace == NULL` and a non-zero
   `workspaceSize`, `ZSTD_getDictID_fromDict(NULL, n>0)`. The C dereferences
   these without a guard and segfaults. Every `pos > size` combination — which
   the C DOES check — is compared.
2. **Degenerate arithmetic in the C** — `FSE_normalizeCount(srcSize = 0)`
   (divide by zero), `FSE_optimalTableLog_internal(srcSize ≤ 1)` /
   `HUF_minTableLog(0)` (`highbit32(0)`, which the C guards only with an
   `assert` that is compiled out), `ZSTD_estimateCCtxSize_usingCCtxParams` with
   LDM explicitly enabled (divide by zero in the LDM min-match derivation),
   `ZSTD_estimate*Size` at levels near `INT_MAX` (unbounded loop),
   `ZSTD_ldm_getMaxNbSeq(minMatchLength = 0)`. Both libraries fail identically;
   the sweeps stay on the defined side of each boundary.
3. **Violated internal preconditions** — `HUF_buildCTable_wksp` with `huffLog`
   below the required tree depth, `HUF_decompress*_usingDTable` with an unbuilt
   DTable, `ZSTD_compressSuperBlock` with `dstCapacity` below the Huffman
   header budget (an unchecked `memcpy` in
   `zstd_compress_superblock.c`), out-of-range `dictMode`/`strategy` into
   `ZSTD_selectBlockCompressor` (an out-of-bounds `[4][10]` array read), and
   `ZSTD_get1BlockSummary` on an array with no block delimiter (it leaves
   `blockSize`/`litSize` uninitialised). Both libraries crash or read garbage
   identically.

`memory_allocation` results are also excluded from comparison via
`Err2::eq_or_oom`: both `.so`s live in the SAME test process, so a configuration
requesting a multi-gigabyte workspace (e.g. `ldmHashLog = 29` ⇒ a 4 GiB LDM
table) can succeed in whichever library runs first and then OOM in the other.
That outcome depends on host free memory, not on the translation. Every other
error code is compared strictly.

## Phase D — feature combinations

`cargo metadata --no-deps --format-version 1 | jq '.packages[0].features'` → `{}`.
There are no optional dependencies and no `#[cfg(feature)]` in the source, so
`--no-default-features` and the default build produce the **same** artifact.
The verification script (`translation/run_all.sh`) executes the full suite under
both `cargo test --release` and `cargo test --release --no-default-features`,
proving this mechanically rather than by assertion. Both passes are green:
29 test targets, 311 `#[test]` functions, 0 failures.
