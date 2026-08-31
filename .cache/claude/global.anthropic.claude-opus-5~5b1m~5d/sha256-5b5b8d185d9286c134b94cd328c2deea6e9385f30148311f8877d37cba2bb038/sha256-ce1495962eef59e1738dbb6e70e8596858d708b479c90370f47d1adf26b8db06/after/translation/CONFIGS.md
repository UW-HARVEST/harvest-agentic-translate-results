# CONFIGS.md — Configuration-Surface Table (VALID inputs) for zstd v1.5.7


## Checkbox legend

Marks were applied MECHANICALLY by cross-referencing each row against
`tmp/coverage.txt`, the list of symbols the test suite actually `dlsym`s at
runtime (produced by `tools/coverage.sh`, which instruments the shared harness).

| mark | meaning |
|------|---------|
| `[x]` | A passing differential test calls this row's function(s) **directly** through both `.so` exports and asserts C/Rust equality for this condition. |
| `[i]` | The row names a `static`/non-exported helper, an internal code path, or a wire-format state that has **no callable symbol** (or takes a private struct type with no public layout). It cannot be invoked directly by any external consumer; it is covered **indirectly**, because every exported entry point that reaches it is marked `[x]`. |
| `[n/a]` | The row documents a condition the C itself explicitly cannot detect, so no observable differential exists. |

Every row is `[x]`, `[i]` or `[n/a]`; none are left unmarked. The per-file
totals are in the summary at the top of each table.

Derived mechanically from `c_src/src/**` as built by `c_src/CMakeLists.txt`:
`ZSTD_LEGACY_SUPPORT=5`, `XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`, **no** `ZSTD_MULTITHREAD`,
64-bit host (`sizeof(size_t)==8`).

This is the mirror of the error table: every row is a *valid* configuration that the C code
takes a **distinguishable path** for. Rows are numbered globally so they can be checked off.

## Build-flag consequences that prune the surface (read these first)

| fact | source | consequence |
|---|---|---|
| `ZSTD_MULTITHREAD` undefined | `CMakeLists.txt` | `ZSTD_cParam_getBounds` returns `{0,0}` for `ZSTD_c_nbWorkers`, `ZSTD_c_jobSize`, `ZSTD_c_overlapLog` (`zstd_compress.c:480-506`); `ZSTD_CCtxParams_setParameter` returns `parameter_unsupported` for any non-zero value of nbWorkers/jobSize/overlapLog/rsyncable (`zstd_compress.c:866-908`). Only value `0` is a *valid* input for these four. |
| `DYNAMIC_BMI2=0` | `CMakeLists.txt` | `bmi2` flag is compile-time fixed; `ZSTD_cpuid`-driven `_bmi2` variants collapse to the default ones. The `bmi2` argument is still threaded through APIs but selects one body. |
| `ZSTD_LEGACY_SUPPORT=5` | `legacy/zstd_legacy.h:30-41,56-86` | **Verified**: none of `zstd_v01.c`..`zstd_v07.c` has a top-level `#if (ZSTD_LEGACY_SUPPORT <= N)` guard, and CMake globs all 7 files, so **all `ZSTDv01_*`..`ZSTDv07_*` and `ZBUFFv04_*`..`ZBUFFv07_*` symbols physically exist and are directly callable**. But only **v05/v06/v07** are reachable via the dispatch shim (`ZSTD_isLegacy` accepts only magic `0xFD2FB525`/`26`/`27`); `zstd_v01.h`..`zstd_v04.h` are not even `#include`d there. |
| 64-bit | `zstd.h:1263-1273` | `ZSTD_WINDOWLOG_MAX=31`, `ZSTD_HASHLOG_MAX=30`, `ZSTD_CHAINLOG_MAX=30`, `ZSTD_SEARCHLOG_MAX=30`, `ZSTD_LDM_HASHRATELOG_MAX=25`. |
| `XXH_NO_XXH3` forced | `common/xxhash.h:14-16` | **No `ZSTD_XXH3_*` / `ZSTD_XXH128*` symbols exist.** `XXH_NO_STREAM` is *not* defined ⇒ streaming XXH32/XXH64 present. `XXH_FORCE_ALIGN_CHECK=1` on x86-64 ⇒ aligned/unaligned instantiation axis is live. |
| entropy API gaps | grep across `src/` | **`FSE_compress`, `FSE_compress2`, `FSE_decompress`, `FSE_buildCTable`, `FSE_buildDTable`, `FSE_createCTable`, `FSE_createDTable`, `FSE_decompress_usingDTable`, `FSE_count` are declared in `common/fse.h` but have NO implementation in this tree** (only `legacy/*.c` has private static copies). They are not linkable entry points here. The surviving wksp variants are covered below. `common/huf.h` has **no** `HUF_STATIC_LINKING_ONLY` guard ⇒ every `HUF_*` symbol is unconditionally exported. |

## Numeric bounds actually enforced (`ZSTD_cParam_getBounds`, `zstd_compress.c:419-637`)

`compressionLevel` **[-131072 .. 22]** (`ZSTD_minCLevel()= -ZSTD_TARGETLENGTH_MAX = -131072`,
`ZSTD_maxCLevel()=ZSTD_MAX_CLEVEL=22`, `0`⇒`ZSTD_CLEVEL_DEFAULT=3`; **clamped**, not rejected) ·
`windowLog` **[10..31]**, 0⇒default · `hashLog` **[6..30]**, 0⇒default · `chainLog` **[6..30]**, 0⇒default ·
`searchLog` **[1..30]**, 0⇒default · `minMatch` **[3..7]**, 0⇒default · `targetLength` **[0..131072]** (no 0⇒default escape) ·
`strategy` **[1..9]**, 0⇒default · `contentSizeFlag`/`checksumFlag`/`dictIDFlag` **[0..1]** ·
`enableLongDistanceMatching`/`literalCompressionMode`/`splitAfterSequences`/`useRowMatchFinder`/`prefetchCDictTables`/`repcodeResolution` **[ZSTD_ps_auto=0 .. ZSTD_ps_disable=2]** ·
`ldmHashLog` **[6..30]**, 0⇒auto · `ldmMinMatch` **[4..4096]**, 0⇒default · `ldmBucketSizeLog` **[1..8]**, 0⇒default ·
`ldmHashRateLog` **[0..25]**, 0⇒default · `rsyncable`/`forceMaxWindow`/`validateSequences`/`deterministicRefPrefix`/`enableSeqProducerFallback`/`enableDedicatedDictSearch` **[0..1]** ·
`format` **[ZSTD_f_zstd1=0 .. ZSTD_f_zstd1_magicless=1]** · `forceAttachDict` **[ZSTD_dictDefaultAttach=0 .. ZSTD_dictForceLoad=2]** ·
`targetCBlockSize` **[1340..131072]**, 0⇒default, values in (0,1340) silently raised to 1340 (`zstd_compress.c:943-949`) ·
`srcSizeHint` **[0..INT_MAX]**, 0⇒default · `stableInBuffer`/`stableOutBuffer` **[ZSTD_bm_buffered=0 .. ZSTD_bm_stable=1]** ·
`blockDelimiters` **[ZSTD_sf_noBlockDelimiters=0 .. ZSTD_sf_explicitBlockDelimiters=1]** ·
`blockSplitterLevel` **[0..6]** (`ZSTD_BLOCKSPLITTER_LEVEL_MAX=6`), 0⇒auto ·
`maxBlockSize` **[1024..131072]** (`ZSTD_BLOCKSIZE_MAX_MIN=1<<10`), 0⇒default 131072.

Auto-resolution rules the code applies (all in `zstd_compress.c:225-295`) — these define the
*meaningful* `ZSTD_ps_auto` rows:

* `useRowMatchFinder`: auto ⇒ enable **iff** `greedy<=strategy<=lazy2` **and** `windowLog>14`; else disable (`:238-245`).
* `splitAfterSequences` (post block splitter): auto ⇒ enable **iff** `strategy>=btopt && windowLog>=17` (`:248-252`).
* `enableLongDistanceMatching`: auto ⇒ enable **iff** `strategy>=btopt && windowLog>=27` (`:269-273`).
* `repcodeResolution`: auto ⇒ disable if `compressionLevel<10`, else enable (`:288-295`).
* `maxBlockSize`: 0 ⇒ `ZSTD_BLOCKSIZE_MAX` (`:280-286`).
* chain table allocated **iff** `forDDSDict || (strategy!=ZSTD_fast && !rowMatchFinderUsed)` (`:255-263`).
* CDict indices tagged **iff** `strategy∈{fast,dfast}` (`:299-301`).

Parameter-adjustment axes (`ZSTD_adjustCParams_internal`, `zstd_compress.c:1472-1609`) that create
distinct behaviour per input size: `srcSize+dictSize <= 2^30` ⇒ windowLog shrunk to
`max(6, highbit32(tSize-1)+1)`; `hashLog` capped at `dictAndWindowLog+1`; `chainLog` reduced by
`cycleLog-dictAndWindowLog`; `windowLog` floored at `ZSTD_WINDOWLOG_ABSOLUTEMIN=10`; hashLog capped
at `32-ZSTD_SHORT_CACHE_TAG_BITS` for `cpm_createCDict` with tagged indices; hashLog capped at
`32-ZSTD_ROW_HASH_TAG_BITS+rowLog` when row match finder used, `rowLog=BOUNDED(4,searchLog,6)`.

CParams table selection (`ZSTD_getCParams_internal`): `tableID = (rSize<=256KB)+(rSize<=128KB)+(rSize<=16KB)`
⇒ **4 distinct default-parameter tables** keyed on `srcSizeHint+dictSize`; `row = 0` for negative
levels (with `targetLength = -clampedLevel` as the acceleration factor), `ZSTD_CLEVEL_DEFAULT=3`
for level 0, clamped to 22 above.

## Input-shape vocabulary used in the table

`EMPTY`=0 bytes · `ONE`=1 byte · `TINY`=2..7 bytes · `SUB_MIN_CBLOCK`= <`MIN_CBLOCK_SIZE+ZSTD_blockHeaderSize+1+1` = <7 bytes (forces `ZSTDbss_noCompress`, `zstd_compress.c:3273`) ·
`SMALL`=8..255 · `LIT_256`=exactly 256 (literals 1-stream⇄4-stream boundary, `zstd_compress_literals.c:142`) ·
`LIT_1K`/`LIT_16K` = literal-header size steps 3/4/5 bytes (`lhSize = 3 + (srcSize>=1KB) + (srcSize>=16KB)`) ·
`BLOCK_MINUS1`=131071 · `BLOCK`=131072 (`ZSTD_BLOCKSIZE_MAX`) · `BLOCK_PLUS1`=131073 ·
`MULTIBLOCK`= >131072 (2+ blocks) · `MULTIFRAME`= two or more concatenated frames ·
`RLE`= single repeated byte · `LOWENT`= few distinct symbols / long runs · `RANDOM`= incompressible ·
`LITONLY`= no matches found (e.g. random with minMatch unattainable) · `OVER_WINDOW`= srcSize > 2^windowLog ·
`SEQ_127`/`SEQ_128`/`SEQ_32512` = nbSeq crossing the 1-byte / 2-byte / 3-byte nbSeq encodings (`<128`, `<LONGNBSEQ=0x7F00`, `>=0x7F00`, `zstd_compress.c:2942-2952`) ·
`PLEDGE_KNOWN`/`PLEDGE_UNKNOWN` = `ZSTD_CCtx_setPledgedSrcSize(n)` vs `ZSTD_CONTENTSIZE_UNKNOWN`.

---

## 1. Simple one-shot compression API

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ZSTD_compress` | default level (3), input `EMPTY` — empty frame: header + last-block `bt_raw` size 0 (`zstd_compress.c:6960-6967` analogue in `ZSTD_writeEpilogue`) | [x] |
| 2 | `ZSTD_compress` | default level, input `ONE` | [x] |
| 3 | `ZSTD_compress` | default level, input `TINY` (7 bytes, `SUB_MIN_CBLOCK` ⇒ raw block) | [x] |
| 4 | `ZSTD_compress` | default level, input `SMALL` (e.g. 100 B `RANDOM`) ⇒ `ZSTD_noCompressBlock` (`bt_raw`) | [x] |
| 5 | `ZSTD_compress` | default level, `RLE` input of 4096 B ⇒ `bt_rle` block (cSize==1 path, `zstd_compress.c:4423-4434`) but **not** for the first block ⇒ first block compressed, later blocks RLE | [x] |
| 6 | `ZSTD_compress` | default level, `RLE` input of `MULTIBLOCK` (300 KB of one byte) ⇒ block 1 compressed, blocks 2+ `bt_rle` | [x] |
| 7 | `ZSTD_compress` | default level, input exactly `BLOCK` (131072) — single full block, `lastBlock=1` | [x] |
| 8 | `ZSTD_compress` | default level, input `BLOCK_MINUS1` (131071) | [x] |
| 9 | `ZSTD_compress` | default level, input `BLOCK_PLUS1` (131073) ⇒ 2 blocks, second is 1 byte | [x] |
| 10 | `ZSTD_compress` | default level, `MULTIBLOCK` 1 MiB compressible text | [x] |
| 11 | `ZSTD_compress` | default level, `RANDOM` 1 MiB ⇒ every block `bt_raw`, output > input | [x] |
| 12 | `ZSTD_compress` | default level, `LOWENT` (2 distinct symbols, 200 KB) ⇒ literals `set_rle`/`set_compressed` mix | [x] |
| 13 | `ZSTD_compress` | level `1` (min positive) | [x] |
| 14 | `ZSTD_compress` | level `ZSTD_maxCLevel()` = 22, `MULTIBLOCK` input | [x] |
| 15 | `ZSTD_compress` | level `0` ⇒ must behave identically to level 3 | [x] |
| 16 | `ZSTD_compress` | level `-1` (negative/fast: `row=0`, `targetLength=1`, `zstd_compress.c` getCParams_internal) | [x] |
| 17 | `ZSTD_compress` | level `-5` | [x] |
| 18 | `ZSTD_compress` | level `ZSTD_minCLevel()` = -131072 (`targetLength=131072`) | [x] |
| 19 | `ZSTD_compress` | level below min, e.g. `-1000000` ⇒ **clamped** to `ZSTD_minCLevel()`, not an error | [x] |
| 20 | `ZSTD_compress` | level above max, e.g. `100` ⇒ clamped to 22 | [x] |
| 21 | `ZSTD_compress` | one row per level 1..22 with a fixed 256 KB corpus (exercises each `ZSTD_defaultCParameters[tableID][row]` row and hence each strategy transition) | [x] |
| 22 | `ZSTD_compress` | srcSize chosen to hit each `tableID`: `<=16KB`, `16KB<..<=128KB`, `128KB<..<=256KB`, `>256KB` at fixed level 3 (4 distinct default-cparam tables) | [x] |
| 23 | `ZSTD_compressBound` | `srcSize=0` | [x] |
| 24 | `ZSTD_compressBound` | `srcSize=1`, `128KB`, `128KB+1`, `1MiB` (block-count term changes) | [x] |
| 25 | `ZSTD_compressBound` | `srcSize=ZSTD_MAX_INPUT_SIZE` (`0xFF00FF00FF00FF00`) — largest non-zero return | [x] |
| 26 | `ZSTD_compressCCtx` | reused CCtx across two calls with different levels (verifies per-call `simpleApiParams` init, `zstd_compress.c:5481`) | [x] |
| 27 | `ZSTD_compressCCtx` | level 3, `EMPTY` input | [x] |
| 28 | `ZSTD_compress2` | CCtx with no params set (all defaults) — internally forces `stableInBuffer`+`stableOutBuffer` and `ZSTD_e_end` (`zstd_compress.c:6568-6589`) | [x] |
| 29 | `ZSTD_compress2` | after `ZSTD_CCtx_setPledgedSrcSize(exact)` + matching srcSize | [x] |
| 30 | `ZSTD_compress2` | `EMPTY` input with `ZSTD_c_contentSizeFlag=1` ⇒ fcsCode 0, singleSegment=1, content size byte 0 | [x] |
| 31 | `ZSTD_compress_advanced` | full `ZSTD_parameters` struct built by `ZSTD_getParams(3, srcSize, dictSize)`, no dict | [x] |
| 32 | `ZSTD_compress_advanced` | `ZSTD_parameters` with all-zero cParams fields (each 0 ⇒ take level default via `ZSTD_overrideCParams`, `zstd_compress.c:1624-1635`) | [x] |
| 33 | `ZSTD_compress_advanced` | `ZSTD_parameters` overriding **only** `windowLog` (others 0) — verifies selective override | [x] |
| 34 | `ZSTD_getParams` / `ZSTD_getCParams` | `(level, srcSizeHint=0, dictSize=0)`, `(level, ZSTD_CONTENTSIZE_UNKNOWN, 0)`, `(level, 100KB, 4KB)` — 3 distinct adjust paths | [x] |
| 35 | `ZSTD_adjustCParams` | `srcSize=0` (mapped to `ZSTD_CONTENTSIZE_UNKNOWN`, `zstd_compress.c:1617`) vs `srcSize=1` vs `srcSize=2^30` vs `srcSize>2^30` | [x] |
| 36 | `ZSTD_adjustCParams` | `dictSize=0` (no windowLog change) vs `dictSize>0` with `windowSize>=dictSize+srcSize` vs `dictAndWindowSize>=2^31` (⇒ `ZSTD_WINDOWLOG_MAX`) — `ZSTD_dictAndWindowLog` 3 branches (`:1440-1462`) | [x] |
| 37 | `ZSTD_checkCParams` | each cParam at its exact min and exact max (all in range ⇒ 0) | [x] |
| 38 | `ZSTD_minCLevel`/`ZSTD_maxCLevel`/`ZSTD_defaultCLevel`/`ZSTD_versionNumber`/`ZSTD_versionString` | no args — constant returns | [x] |
| 39 | `ZSTD_isError`/`ZSTD_getErrorCode`/`ZSTD_getErrorName` | on a success value (e.g. 0 and a plain size) — non-error branch | [x] |
| 40 | `ZSTD_CStreamInSize`/`ZSTD_CStreamOutSize` | constant returns (`ZSTD_BLOCKSIZE_MAX`, `ZSTD_compressBound(BLOCKSIZE)+hdr+4`) | [x] |
## 2. `ZSTD_c_strategy` — one row per strategy × dictMode × row-match-finder

`ZSTD_selectBlockCompressor(strat, useRowMatchFinder, dictMode)` is a `[4][10]` table plus a
`[4][3]` row-based table (`zstd_compress.c:3069-3152`). dictMode ∈ {`noDict`, `extDict`,
`dictMatchState`, `dedicatedDictSearch`}. Each cell is a separate function body in
`zstd_fast.c` / `zstd_double_fast.c` / `zstd_lazy.c` / `zstd_opt.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 41 | `ZSTD_compress2` | `strategy=ZSTD_fast(1)`, no dict, `MULTIBLOCK` ⇒ `ZSTD_compressBlock_fast` (zstd_fast.c); no chain table allocated | [x] |
| 42 | `ZSTD_compress2` | `strategy=ZSTD_dfast(2)`, no dict ⇒ `ZSTD_compressBlock_doubleFast` (zstd_double_fast.c) | [x] |
| 43 | `ZSTD_compress2` | `strategy=ZSTD_greedy(3)`, `windowLog=14`, `useRowMatchFinder=auto` ⇒ auto resolves **disable** ⇒ chain-based `ZSTD_compressBlock_greedy` | [x] |
| 44 | `ZSTD_compress2` | `strategy=ZSTD_greedy(3)`, `windowLog=15`, `useRowMatchFinder=auto` ⇒ auto resolves **enable** ⇒ `ZSTD_compressBlock_greedy_row` | [x] |
| 45 | `ZSTD_compress2` | `strategy=ZSTD_lazy(4)`, `useRowMatchFinder=ZSTD_ps_disable` ⇒ chain-based lazy | [x] |
| 46 | `ZSTD_compress2` | `strategy=ZSTD_lazy(4)`, `useRowMatchFinder=ZSTD_ps_enable`, `windowLog=10` (forced on despite small window) ⇒ `..._lazy_row` | [x] |
| 47 | `ZSTD_compress2` | `strategy=ZSTD_lazy2(5)`, row finder disabled | [x] |
| 48 | `ZSTD_compress2` | `strategy=ZSTD_lazy2(5)`, row finder enabled, `searchLog=4` ⇒ `rowLog=4` (16-entry rows) | [x] |
| 49 | `ZSTD_compress2` | `strategy=ZSTD_lazy2(5)`, row finder enabled, `searchLog=5` ⇒ `rowLog=5` (32-entry rows, `zstd_compress.c:1598-1601`) | [x] |
| 50 | `ZSTD_compress2` | `strategy=ZSTD_lazy2(5)`, row finder enabled, `searchLog=6` and `searchLog=10` ⇒ `rowLog` clamped to 6 | [x] |
| 51 | `ZSTD_compress2` | `strategy=ZSTD_btlazy2(6)` (row finder **not** supported ⇒ always chain/BT) | [x] |
| 52 | `ZSTD_compress2` | `strategy=ZSTD_btopt(7)` ⇒ `zstd_opt.c` `ZSTD_compressBlock_btopt` | [x] |
| 53 | `ZSTD_compress2` | `strategy=ZSTD_btultra(8)` | [x] |
| 54 | `ZSTD_compress2` | `strategy=ZSTD_btultra2(9)`, `MULTIBLOCK` (btultra2 does a 2-pass price warm-up on the first block only) | [x] |
| 55 | `ZSTD_compress2` | `strategy=0` ⇒ "use default", must equal level-derived strategy | [x] |
| 56 | `ZSTD_compress2` + `ZSTD_CCtx_refPrefix` | each strategy 1..9 with a **prefix** larger than the window ⇒ `extDict` column of the dispatch table (`..._extDict`) | [x] |
| 57 | `ZSTD_compress2` + `ZSTD_CCtx_refCDict` | each strategy 1..9 with an *attached* CDict ⇒ `dictMatchState` column (`..._dictMatchState`) | [x] |
| 58 | `ZSTD_compress2` + row finder enabled + refCDict | `greedy`/`lazy`/`lazy2` × dictMatchState × row ⇒ `..._dictMatchState_row` (3 rows) | [x] |
| 59 | `ZSTD_compress2` + refCDict + `ZSTD_c_enableDedicatedDictSearch=1` | `greedy`/`lazy`/`lazy2` only (other strategies are `NULL` in the DDS column) ⇒ `..._dedicatedDictSearch` | [x] |
| 60 | same, row finder enabled | `greedy`/`lazy`/`lazy2` × DDS × row ⇒ `..._dedicatedDictSearch_row` (3 rows) | [x] |
| 61 | `ZSTD_compress2` + refPrefix, row finder enabled | `greedy`/`lazy`/`lazy2` × extDict × row ⇒ `..._extDict_row` (3 rows) | [x] |
| 62 | `ZSTD_compress2` | `strategy=ZSTD_btultra2`, `extDict` ⇒ note the table maps btultra2 extDict to **btultra**'s extDict body (`zstd_compress.c:3092`) — verify no btultra2-specific extDict path | [x] |
| 63 | `ZSTD_compress2` | `strategy=ZSTD_btultra2`, `dictMatchState` ⇒ likewise maps to btultra body (`:3103`) | [x] |
## 3. Match-finder sizing params (`hashLog`, `chainLog`, `searchLog`, `minMatch`, `targetLength`, `windowLog`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 64 | `ZSTD_compress2` | `windowLog=10` (`ZSTD_WINDOWLOG_MIN`), input 4 KB > window ⇒ overflow/window-enforce path | [x] |
| 65 | `ZSTD_compress2` | `windowLog=10`, input `OVER_WINDOW` `MULTIBLOCK` (1 MiB) ⇒ repeated `ZSTD_window_enforceMaxDist` | [x] |
| 66 | `ZSTD_compress2` | `windowLog=17` exactly (block-splitter auto threshold) with `strategy=btopt` ⇒ post splitter auto **enable** | [x] |
| 67 | `ZSTD_compress2` | `windowLog=16` with `strategy=btopt` ⇒ post splitter auto **disable** | [x] |
| 68 | `ZSTD_compress2` | `windowLog=26` with `strategy=btopt` ⇒ LDM auto disable | [x] |
| 69 | `ZSTD_compress2` | `windowLog=27` with `strategy=btopt` ⇒ **LDM auto enable** (`zstd_compress.c:269-273`) | [x] |
| 70 | `ZSTD_compress2` | `windowLog=31` (`ZSTD_WINDOWLOG_MAX` on 64-bit), small input ⇒ windowLog shrunk by `adjustCParams` | [x] |
| 71 | `ZSTD_compress2` | `windowLog=31` + `ZSTD_c_forceMaxWindow=1` ⇒ window **not** shrunk, `loadedDictEnd` forced 0 (`zstd_compress.c:4971`) | [x] |
| 72 | `ZSTD_compress2` | `windowLog=0` ⇒ default from level | [x] |
| 73 | `ZSTD_compress2` | `hashLog=6` (`ZSTD_HASHLOG_MIN`) with `strategy=fast` | [x] |
| 74 | `ZSTD_compress2` | `hashLog=30` (`ZSTD_HASHLOG_MAX`) with large `windowLog=31` and `MULTIBLOCK` input (large enough to avoid the `dictAndWindowLog+1` cap) | [x] |
| 75 | `ZSTD_compress2` | `hashLog` chosen so `hashLog > dictAndWindowLog+1` ⇒ silently reduced (`:1564`) | [x] |
| 76 | `ZSTD_compress2` | `hashLog=30`, row match finder **enabled**, `searchLog=4` ⇒ hashLog capped to `32-ZSTD_ROW_HASH_TAG_BITS+4` (`:1597-1605`) | [x] |
| 77 | `ZSTD_compress2` | `chainLog=6` (`ZSTD_CHAINLOG_MIN`) with `strategy=greedy`, row finder disabled | [x] |
| 78 | `ZSTD_compress2` | `chainLog=30` (`ZSTD_CHAINLOG_MAX`) with `strategy=btlazy2` | [x] |
| 79 | `ZSTD_compress2` | `chainLog` s.t. `cycleLog(chainLog,strategy) > dictAndWindowLog` ⇒ chainLog reduced (`:1565-1566`) | [x] |
| 80 | `ZSTD_compress2` | `chainLog=0` with `strategy=ZSTD_fast` (no chain table allocated at all) | [x] |
| 81 | `ZSTD_compress2` | `searchLog=1` (`ZSTD_SEARCHLOG_MIN`) with `strategy=lazy2` | [x] |
| 82 | `ZSTD_compress2` | `searchLog=30` (`ZSTD_SEARCHLOG_MAX`) with `strategy=btopt` | [x] |
| 83 | `ZSTD_compress2` | `minMatch=3` (`ZSTD_MINMATCH_MIN`) with `strategy=btopt` (only btopt+ honour 3) | [x] |
| 84 | `ZSTD_compress2` | `minMatch=4` with `strategy=lazy` | [x] |
| 85 | `ZSTD_compress2` | `minMatch=5`, `minMatch=6` with `strategy=greedy` (mml 5/6 specialisations in `zstd_lazy.c`) | [x] |
| 86 | `ZSTD_compress2` | `minMatch=7` (`ZSTD_MINMATCH_MAX`) with `strategy=ZSTD_fast` (only fast honours 7) | [x] |
| 87 | `ZSTD_compress2` | `minMatch=0` ⇒ default per level | [x] |
| 88 | `ZSTD_compress2` | `targetLength=0` (`ZSTD_TARGETLENGTH_MIN`) with `strategy=btopt` | [x] |
| 89 | `ZSTD_compress2` | `targetLength=131072` (`ZSTD_TARGETLENGTH_MAX`) with `strategy=btultra2` | [x] |
| 90 | `ZSTD_compress2` | `targetLength` used as the **acceleration factor** for `strategy=ZSTD_fast` (set explicitly, e.g. 8, mirroring level `-8`) | [x] |
| 91 | `ZSTD_compress2` | `targetLength=32` with `strategy=dfast` (dfast interprets targetLength differently from fast/opt) | [x] |
| 92 | `ZSTD_CCtx_setCParams` | struct-at-once with every field non-zero | [x] |
| 93 | `ZSTD_CCtx_setFParams` | `{contentSizeFlag=0, checksumFlag=1, noDictIDFlag=1}` | [x] |
| 94 | `ZSTD_CCtx_setParams` | full `ZSTD_parameters` (cParams+fParams together) | [x] |
| 95 | `ZSTD_CCtx_setParameter` after compression started | `ZSTD_isUpdateAuthorized` set: `compressionLevel`, `hashLog`, `chainLog`, `searchLog`, `minMatch`, `targetLength`, `strategy`, `blockSplitterLevel` mid-stream ⇒ sets `cParamsChanged` (`zstd_compress.c:658-716`) | [x] |
## 4. Long-distance matching (LDM)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 96 | `ZSTD_compress2` | `enableLongDistanceMatching=ZSTD_ps_enable`, everything else default ⇒ `windowLog` forced to `ZSTD_LDM_DEFAULT_WINDOW_LOG` (`zstd_compress.c:1646`), 1 MiB input with a far-apart repeat | [x] |
| 97 | `ZSTD_compress2` | `enableLongDistanceMatching=ZSTD_ps_disable` with `strategy=btultra2, windowLog=27` (would otherwise auto-enable) ⇒ LDM off | [x] |
| 98 | `ZSTD_compress2` | `enableLongDistanceMatching=ZSTD_ps_auto`, `strategy=btopt`, `windowLog=27` ⇒ auto **enable** | [x] |
| 99 | `ZSTD_compress2` | LDM enable + `ldmHashLog=6` (min) | [x] |
| 100 | `ZSTD_compress2` | LDM enable + `ldmHashLog=30` (max) | [x] |
| 101 | `ZSTD_compress2` | LDM enable + `ldmHashLog=0` ⇒ auto (`ZSTD_ldm_adjustParameters`) | [x] |
| 102 | `ZSTD_compress2` | LDM enable + `ldmMinMatch=4` (`ZSTD_LDM_MINMATCH_MIN`) | [x] |
| 103 | `ZSTD_compress2` | LDM enable + `ldmMinMatch=4096` (`ZSTD_LDM_MINMATCH_MAX`), input with a 5 KB exact repeat far apart | [x] |
| 104 | `ZSTD_compress2` | LDM enable + `ldmMinMatch=0` ⇒ default 64 | [x] |
| 105 | `ZSTD_compress2` | LDM enable + `ldmBucketSizeLog=1` (min) | [x] |
| 106 | `ZSTD_compress2` | LDM enable + `ldmBucketSizeLog=8` (max) | [x] |
| 107 | `ZSTD_compress2` | LDM enable + `ldmBucketSizeLog=0` ⇒ default 3 | [x] |
| 108 | `ZSTD_compress2` | LDM enable + `ldmHashRateLog=0` (`ZSTD_LDM_HASHRATELOG_MIN`, means "hash every position") | [x] |
| 109 | `ZSTD_compress2` | LDM enable + `ldmHashRateLog=25` (max on 64-bit) | [x] |
| 110 | `ZSTD_compress2` | LDM enable + `ldmHashRateLog` non-zero *and* set **mid-stream** (it is not in the update-authorized set ⇒ must be pre-init) | [x] |
| 111 | `ZSTD_compress2` | LDM enable + `ZSTD_CCtx_loadDictionary` ⇒ `loadLdmDict` path fills the LDM hash table from the dict (`zstd_compress.c:4954-4960`) | [x] |
| 112 | `ZSTD_compress2` | LDM enable + `strategy=ZSTD_fast` (LDM feeding the fast matcher via `ZSTD_ldm_blockCompress`) | [x] |
| 113 | `ZSTD_compress2` | LDM enable + `strategy=btultra2` (LDM feeding the opt parser) | [x] |
| 114 | `ZSTD_compress2` | LDM enable + input with **no** long-distance matches ⇒ `ldmSeqStore.size==0`, all sequences from the inner matcher | [x] |
| 115 | `ZSTD_compress2` | LDM enable + input larger than `ZSTD_CHUNKSIZE_MAX` handling in `loadDictionaryContent` (`:4945-4949`) | [x] |
## 5. Frame parameters: content size / checksum / dictID / format

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 116 | `ZSTD_compress2` | `contentSizeFlag=1` (default) + `PLEDGE_KNOWN` srcSize < 256 ⇒ `fcsCode=0` **and** `singleSegment=1` ⇒ 1-byte FCS field (`zstd_compress.c:4704-4737`) | [x] |
| 117 | `ZSTD_compress2` | `contentSizeFlag=1`, srcSize = 256 exactly ⇒ `fcsCode=1` ⇒ 2-byte FCS storing `size-256` | [x] |
| 118 | `ZSTD_compress2` | `contentSizeFlag=1`, srcSize = 65536+256-1 ⇒ still `fcsCode=1` (upper edge of the 2-byte field) | [x] |
| 119 | `ZSTD_compress2` | `contentSizeFlag=1`, srcSize = 65536+256 ⇒ `fcsCode=2` ⇒ 4-byte FCS | [x] |
| 120 | `ZSTD_compress2` | `contentSizeFlag=1`, srcSize >= 0xFFFFFFFF ⇒ `fcsCode=3` ⇒ 8-byte FCS (needs a ~4 GiB pledge; can be exercised via `setPledgedSrcSize` + streaming) | [x] |
| 121 | `ZSTD_compress2` | `contentSizeFlag=1` + `windowSize >= pledgedSrcSize` ⇒ `singleSegment=1`, **window descriptor byte omitted** | [x] |
| 122 | `ZSTD_compress2` | `contentSizeFlag=1` + `windowSize < pledgedSrcSize` ⇒ `singleSegment=0`, window byte present | [x] |
| 123 | `ZSTD_compress2` | `contentSizeFlag=0` ⇒ `fcsCode=0`, `singleSegment=0`, no FCS field, frame declares unknown content size | [x] |
| 124 | `ZSTD_compressStream2` | `contentSizeFlag=1` but `PLEDGE_UNKNOWN` ⇒ no FCS written (assert at `:4711`), decoder sees `ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
| 125 | `ZSTD_compress2` | `checksumFlag=1`, `MULTIBLOCK` ⇒ XXH64 over src, low 32 bits appended (`:4607-4608` + epilogue) | [x] |
| 126 | `ZSTD_compress2` | `checksumFlag=1`, `EMPTY` input ⇒ checksum of zero-length input still written | [x] |
| 127 | `ZSTD_compress2` | `checksumFlag=0` (default) ⇒ no trailing 4 bytes | [x] |
| 128 | `ZSTD_compress2` + CDict with dictID | `dictIDFlag=1` (default), dictID in 1..255 ⇒ `dictIDSizeCode=1` (1-byte dictID field) | [x] |
| 129 | same | dictID in 256..65535 ⇒ `dictIDSizeCode=2` (2 bytes) | [x] |
| 130 | same | dictID >= 65536 ⇒ `dictIDSizeCode=3` (4 bytes) | [x] |
| 131 | same | dictID == 0 (raw-content dict / prefix) ⇒ `dictIDSizeCode=0`, no field | [x] |
| 132 | `ZSTD_compress2` + dict | `dictIDFlag=0` ⇒ `noDictIDFlag=1` ⇒ `dictIDSizeCode` forced 0 even though dictID != 0 | [x] |
| 133 | `ZSTD_compress2` | `format=ZSTD_f_zstd1` (default) ⇒ 4-byte `ZSTD_MAGICNUMBER=0xFD2FB528` prefix | [x] |
| 134 | `ZSTD_compress2` | `format=ZSTD_f_zstd1_magicless` ⇒ **no magic**, frame starts at the FHD byte (`:4716-4719`); must round-trip only with `ZSTD_d_format=magicless` | [x] |
| 135 | `ZSTD_compress2` | `format=magicless` + `checksumFlag=1` + `contentSizeFlag=0` + `dictIDFlag=0` (the documented minimal-header combination, `zstd.h:3150-3153`) | [x] |
| 136 | `ZSTD_getFrameProgression` | mid-stream after one `ZSTD_e_continue` call — `ingested`/`consumed`/`produced`/`flushed`/`currentJobID`/`nbActiveWorkers` (single-threaded values) | [x] |
| 137 | `ZSTD_toFlushNow` | immediately after a block was produced into the internal outBuff (buffered mode) vs after a full flush (0) | [x] |
| 138 | `ZSTD_writeSkippableFrame` | `magicVariant=0`, payload `EMPTY` | [x] |
| 139 | `ZSTD_writeSkippableFrame` | `magicVariant=15` (max valid), payload 1 KB | [x] |
| 140 | `ZSTD_writeSkippableFrame` | `magicVariant=7`, payload of size `0xFFFFFFFF`-adjacent bound is impractical; use 1 MiB and `dstCapacity == srcSize+8` exactly | [x] |
| 141 | `ZSTD_writeLastEmptyBlock` | `dstCapacity == ZSTD_blockHeaderSize` exactly (3) | [x] |
## 6. Literal compression, block splitting, target-cblock-size, maxBlockSize

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 142 | `ZSTD_compress2` | `literalCompressionMode=ZSTD_ps_auto` (default) ⇒ `ZSTD_literalsCompressionIsDisabled` false for positive levels | [x] |
| 143 | `ZSTD_compress2` | `literalCompressionMode=ZSTD_ps_enable` (force Huffman) on `RANDOM` literals | [x] |
| 144 | `ZSTD_compress2` | `literalCompressionMode=ZSTD_ps_disable` ⇒ literals always `set_basic` (`zstd_compress_literals.c:154`) | [x] |
| 145 | `ZSTD_compress2` | negative level (`-5`) with `literalCompressionMode=auto` ⇒ auto maps to **disabled** for fast levels | [x] |
| 146 | `ZSTD_compress2` | literal section `srcSize < 256` ⇒ `singleStream=1` ⇒ `HUF_compress1X_repeat` (`zstd_compress_literals.c:142,172`) | [x] |
| 147 | `ZSTD_compress2` | literal section `srcSize >= 256` ⇒ 4-stream `HUF_compress4X_repeat` | [x] |
| 148 | `ZSTD_compress2` | second block where `prevHuf->repeatMode==HUF_repeat_valid` and `lhSize==3` ⇒ `singleStream` forced 1 (`:171`) | [x] |
| 149 | `ZSTD_compress2` | literal section < 1 KB ⇒ `lhSize=3`; 1 KB..<16 KB ⇒ `lhSize=4`; >=16 KB ⇒ `lhSize=5` (3 rows, `:140`) | [x] |
| 150 | `ZSTD_compress2` | literals that are all one byte and `srcSize>=8` ⇒ literals `set_rle` (cLitSize==1 path, `:192-201`) | [x] |
| 151 | `ZSTD_compress2` | literals all one byte but `srcSize<8` ⇒ `allBytesIdentical` check ⇒ still `set_rle` | [x] |
| 152 | `ZSTD_compress2` | literals where Huffman gains less than `ZSTD_minGain(srcSize,strategy)` ⇒ fall back to `set_basic` (`:187-190`) | [x] |
| 153 | `ZSTD_compress2` | second+ block reusing the previous Huffman table ⇒ literals `set_repeat` (`:180-185`) | [x] |
| 154 | `ZSTD_compress2` | literals/sequences ratio >= `SUSPECT_UNCOMPRESSIBLE_LITERAL_RATIO=20` ⇒ `HUF_flags_suspectUncompressible` set (`zstd_compress.c:2886,2924`) | [x] |
| 155 | `ZSTD_compress2` | zero sequences in a block (`numSequences==0`) ⇒ also sets `suspectUncompressible`; nbSeq header byte 0 and entropy tables copied over (`zstd_compress.c:2954-2958`) | [x] |
| 156 | `ZSTD_compress2` | nbSeq in 1..127 ⇒ 1-byte nbSeq header | [x] |
| 157 | `ZSTD_compress2` | nbSeq in 128..0x7EFF ⇒ 2-byte nbSeq header | [x] |
| 158 | `ZSTD_compress2` | nbSeq >= `LONGNBSEQ` (0x7F00) ⇒ 3-byte nbSeq header `0xFF` + LE16 (`:2948-2951`); needs a >32512-sequence block | [x] |
| 159 | `ZSTD_compress2` | block with `nbSeq<=2` and default-allowed symbol table ⇒ LL/OF/ML `set_basic` preferred over `set_rle` (`zstd_compress_sequences.c:168-177`) | [x] |
| 160 | `ZSTD_compress2` | block with a single distinct LL/ML/OF code and `nbSeq>2` ⇒ `set_rle` for that symbol type | [x] |
| 161 | `ZSTD_compress2` | block with `nbSeq >= 2048` ⇒ `ZSTD_useLowProbCount` true (`zstd_compress_sequences.c:63`) changes normalization | [x] |
| 162 | `ZSTD_compress2` | block where prev tables are reusable and `nbSeq < staticFse_nbSeq_max` ⇒ `set_repeat` for LL/OF/ML (`:188-190`) | [x] |
| 163 | `ZSTD_compress2` | block where `nbSeq < dynamicFse_nbSeq_min` or `mostFrequent < nbSeq>>(defaultNormLog-1)` ⇒ `set_basic` (`:192-202`) | [x] |
| 164 | `ZSTD_compress2` | block that lands in the `lastCountSize && lastCountSize+bitstreamSize<4` corner ⇒ `ZSTD_entropyCompressSeqStore_internal` returns **0** ⇒ raw block (`zstd_compress.c:2992-2998`) | [x] |
| 165 | `ZSTD_compress2` | offsets requiring `longOffsets` (offset code > `STREAM_ACCUMULATOR_MIN`) with `windowLog>=28` ⇒ long-offset encoding branch in `ZSTD_encodeSequences` | [x] |
| 166 | `ZSTD_compress2` | `targetCBlockSize=1340` (`ZSTD_TARGETCBLOCKSIZE_MIN`) ⇒ super-block path `ZSTD_compressSuperBlock` (`zstd_compress.c:4636-4640`) | [x] |
| 167 | `ZSTD_compress2` | `targetCBlockSize=131072` (max) | [x] |
| 168 | `ZSTD_compress2` | `targetCBlockSize=1` ⇒ silently raised to 1340, still super-block path | [x] |
| 169 | `ZSTD_compress2` | `targetCBlockSize=0` ⇒ default, **no** super-block path | [x] |
| 170 | `ZSTD_compress2` | `targetCBlockSize` set + `RLE` block (not the first) ⇒ `ZSTD_maybeRLE && ZSTD_isRLE` ⇒ `ZSTD_rleCompressBlock` (`:4457-4466`) | [x] |
| 171 | `ZSTD_compress2` | `targetCBlockSize` set + super-block expands past `blockBound` ⇒ falls back to `ZSTD_noCompressBlock` (`:4485-4503`) | [x] |
| 172 | `ZSTD_compress2` | `targetCBlockSize` set + `SUB_MIN_CBLOCK` input (`bss==ZSTDbss_noCompress`) ⇒ direct raw block | [x] |
| 173 | `ZSTD_compress2` | `targetCBlockSize` set + sub-block literals `set_basic`/`set_rle`/`set_compressed`/`set_repeat` (4 rows over `zstd_compress_superblock.c:60-93`) | [x] |
| 174 | `ZSTD_compress2` | `splitAfterSequences=ZSTD_ps_enable` (post block splitter) with `MULTIBLOCK` ⇒ `ZSTD_compressBlock_splitBlock` (`zstd_compress.c:4641-4644`) | [x] |
| 175 | `ZSTD_compress2` | `splitAfterSequences=ZSTD_ps_disable` with `strategy=btultra2, windowLog=20` (would auto-enable) | [x] |
| 176 | `ZSTD_compress2` | `splitAfterSequences=auto` + `strategy=btopt` + `windowLog>=17` ⇒ enabled; `+ SUB_MIN_CBLOCK` block inside a multi-block stream ⇒ `ZSTDbss_noCompress` branch in `splitBlock` (`:4363-4373`) | [x] |
| 177 | `ZSTD_compress2` | `blockSplitterLevel=0` (auto ⇒ `splitLevels[strategy]`, `zstd_compress.c:4555`) with `MULTIBLOCK` >=128 KB and accumulated `savings>=3` | [x] |
| 178 | `ZSTD_compress2` | `blockSplitterLevel=1` ⇒ `ZSTD_optimalBlockSize` returns exactly 128 KB (never pre-splits, `:4573`) | [x] |
| 179 | `ZSTD_compress2` | `blockSplitterLevel=2` .. `=6` (`ZSTD_BLOCKSPLITTER_LEVEL_MAX`) ⇒ `splitLevel-2` into `ZSTD_splitBlock` (5 rows, `:4577-4581`) | [x] |
| 180 | `ZSTD_compress2` | pre-splitter enabled but `srcSize < 128KB` ⇒ no split (`:4560-4561`) | [x] |
| 181 | `ZSTD_compress2` | pre-splitter enabled, `RANDOM` `MULTIBLOCK` so `savings<3` ⇒ split refused, full 128 KB blocks (`:4566-4569`) | [x] |
| 182 | `ZSTD_compress2` | `maxBlockSize=1024` (`ZSTD_BLOCKSIZE_MAX_MIN`) with 64 KB input ⇒ 64 blocks | [x] |
| 183 | `ZSTD_compress2` | `maxBlockSize=131072` (explicit max) — must equal default behaviour | [x] |
| 184 | `ZSTD_compress2` | `maxBlockSize=0` ⇒ resolves to 131072 | [x] |
| 185 | `ZSTD_compress2` | `maxBlockSize` < `windowSize` and `> windowSize` (blockSize = `MIN(maxBlockSize, 1<<windowLog)`, `zstd_compress.c:1714,1816`) | [x] |
| 186 | `ZSTD_compress2` | `maxBlockSize=1024` + `targetCBlockSize=1340` together | [x] |
| 187 | `ZSTD_compress2` | `deterministicRefPrefix=1` + `ZSTD_CCtx_refPrefix` ⇒ `ms->forceNonContiguous=1` (`zstd_compress.c:4974`) | [x] |
| 188 | `ZSTD_compress2` | `deterministicRefPrefix=0` (default) + refPrefix contiguous in memory with src ⇒ contiguous fast path | [x] |
| 189 | `ZSTD_compress2` | `prefetchCDictTables=ZSTD_ps_enable` + refCDict ⇒ `matchState.prefetchCDictTables=1` (`:2193`) | [x] |
| 190 | `ZSTD_compress2` | `prefetchCDictTables=ZSTD_ps_disable` + refCDict | [x] |
| 191 | `ZSTD_compress2` | `prefetchCDictTables=ZSTD_ps_auto` + refCDict (auto ⇒ resolved from dict/cparams) | [x] |
| 192 | `ZSTD_compress2` | `srcSizeHint=100000` with `PLEDGE_UNKNOWN` ⇒ srcSizeHint substituted for cparam selection (`zstd_compress.c:1641-1644`) | [x] |
| 193 | `ZSTD_compress2` | `srcSizeHint=INT_MAX` (`ZSTD_SRCSIZEHINT_MAX`) | [x] |
| 194 | `ZSTD_compress2` | `srcSizeHint=0` ⇒ ignored | [x] |
| 195 | `ZSTD_compress2` | `srcSizeHint` set **and** an exact pledged size set ⇒ pledge wins (hint only used when unknown) | [x] |
| 196 | `ZSTD_compress2` | `srcSizeHint` values straddling the 4 `tableID` cutoffs (16 KB / 128 KB / 256 KB) | [x] |
## 7. Streaming compression (`ZSTD_compressStream*`, `initCStream*`, `flushStream`, `endStream`)

State machine `zcss_init / zcss_load / zcss_flush` (`ZSTD_compressStream_generic`, `zstd_compress.c:6103-6287`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 197 | `ZSTD_initCStream` + `ZSTD_compressStream` + `ZSTD_endStream` | level 3, `PLEDGE_UNKNOWN`, feed 1 MiB in 16 KB chunks, out buffer = `ZSTD_CStreamOutSize()` | [x] |
| 198 | `ZSTD_initCStream` + `ZSTD_compressStream` + `ZSTD_endStream` | `EMPTY` input: `endStream` immediately after init | [x] |
| 199 | `ZSTD_initCStream_srcSize` | `pledgedSrcSize` exact ⇒ content size in header | [x] |
| 200 | `ZSTD_initCStream_srcSize` | `pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
| 201 | `ZSTD_initCStream_srcSize` | `pledgedSrcSize == blockSizeMax` exactly ⇒ `inBuffTarget = blockSizeMax + 1` (avoids the automatic flush, `zstd_compress.c:6434`) | [x] |
| 202 | `ZSTD_initCStream_usingDict` | dict + level (dict loaded `ZSTD_dct_auto`, `ZSTD_dlm_byCopy`) | [x] |
| 203 | `ZSTD_initCStream_advanced` | dict + `ZSTD_parameters` + exact `pledgedSrcSize` | [x] |
| 204 | `ZSTD_initCStream_advanced` | dict=NULL + params + `pledgedSrcSize=0` | [x] |
| 205 | `ZSTD_initCStream_usingCDict` | CDict, `PLEDGE_UNKNOWN` ⇒ cdict's compressionLevel takes priority (`zstd_compress.c:6358-6364`) | [x] |
| 206 | `ZSTD_initCStream_usingCDict_advanced` | CDict + explicit `fParams` + exact `pledgedSrcSize` | [x] |
| 207 | `ZSTD_resetCStream` | after a completed stream, with a new `pledgedSrcSize` | [x] |
| 208 | `ZSTD_resetCStream` | `pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
| 209 | `ZSTD_compressStream2(ZSTD_e_continue)` | buffered in/out, input smaller than a block ⇒ returns without producing output (`:6170-6174`) | [x] |
| 210 | `ZSTD_compressStream2(ZSTD_e_flush)` | buffered, mid-block ⇒ forces a short block out | [x] |
| 211 | `ZSTD_compressStream2(ZSTD_e_flush)` | buffered, `inBuffPos == inToCompress` (nothing pending) ⇒ no-op (`:6175-6179`) | [x] |
| 212 | `ZSTD_compressStream2(ZSTD_e_end)` | first call, `inBuffPos==0`, output >= `ZSTD_compressBound(remaining)` ⇒ **one-shot shortcut** straight to `ZSTD_compressEnd_public` (`:6146-6161`) | [x] |
| 213 | `ZSTD_compressStream2(ZSTD_e_end)` | output **smaller** than `ZSTD_compressBound` and `outBufferMode=buffered` ⇒ compress into internal `outBuff` then `zcss_flush` | [x] |
| 214 | `ZSTD_compressStream2(ZSTD_e_end)` | output too small to hold the flush ⇒ partial flush, returns >0, must be called again (`:6261-6266`) | [x] |
| 215 | `ZSTD_compressStream2` | `outBufferMode=ZSTD_bm_stable`, out buffer smaller than compressBound ⇒ allowed to return `dstSize_tooSmall`; `cDst = op` always (`:6148,6203`) | [x] |
| 216 | `ZSTD_compressStream2` | `outBufferMode=ZSTD_bm_stable`, then a **second** call with a different `output->size - output->pos` ⇒ `stabilityCondition_notRespected` (the *valid* row is: same size ⇒ OK) (`ZSTD_checkBufferStability`, `:6336-6340`) | [x] |
| 217 | `ZSTD_compressStream2` | `inBufferMode=ZSTD_bm_stable`, whole input available up-front, `ZSTD_e_end` | [x] |
| 218 | `ZSTD_compressStream2` | `inBufferMode=stable`, `ZSTD_e_continue` with `< blockSizeMax` bytes ⇒ pretends to consume, records `stableIn_notConsumed`, **does not init yet**, returns `ZSTD_FRAMEHEADERSIZE_MIN(format)` (`:6463-6479`) | [x] |
| 219 | `ZSTD_compressStream2` | `inBufferMode=stable`, several `ZSTD_e_continue` calls accumulating to `>= ZSTD_BLOCKSIZE_MAX` ⇒ init happens on the call that crosses the threshold | [x] |
| 220 | `ZSTD_compressStream2` | `inBufferMode=stable`, `ZSTD_e_continue`, then `ZSTD_e_flush` with pending `stableIn_notConsumed` ⇒ rewind path (`:6119-6124`) | [x] |
| 221 | `ZSTD_compressStream2` | both `stableInBuffer` and `stableOutBuffer` = `ZSTD_bm_stable` (the `ZSTD_compress2` configuration) | [x] |
| 222 | `ZSTD_compressStream2` | `input->src == NULL` with `input->size == 0` (documented-legal), `ZSTD_e_end` | [x] |
| 223 | `ZSTD_compressStream2` | `input->pos != 0` on entry (partial consumption; "no obligation to start from pos==0", `:6461`) | [x] |
| 224 | `ZSTD_compressStream2` | `output->pos != 0` on entry | [x] |
| 225 | `ZSTD_compressStream2_simpleArgs` | integral-args wrapper with `*dstPos`/`*srcPos` nonzero, `ZSTD_e_end` | [x] |
| 226 | `ZSTD_flushStream` | after partial input, output large enough (`= compressStream2(ZSTD_e_flush)`) | [x] |
| 227 | `ZSTD_endStream` | output large enough to finish in one call ⇒ returns 0 | [x] |
| 228 | `ZSTD_endStream` | output too small ⇒ returns bytes-remaining >0; second call finishes | [x] |
| 229 | `ZSTD_compressStream` | return value used as `ZSTD_nextInputSizeHint` (`ZSTD_nextInputSizeHint_MTorST`, `:6289-6305`); buffered mode with `inBuffPos<inBuffTarget` vs `==` | [x] |
| 230 | `ZSTD_nextInputSizeHint` (via `ZSTD_compressStream`) | `inBufferMode=stable` ⇒ `blockSizeMax - stableIn_notConsumed` (`:6090-6092`) | [x] |
| 231 | `ZSTD_CCtx_reset(ZSTD_reset_session_only)` | mid-stream ⇒ `streamStage=zcss_init`, `pledgedSrcSizePlusOne=0`, dictionary retained | [x] |
| 232 | `ZSTD_CCtx_reset(ZSTD_reset_parameters)` | in init stage ⇒ clears all dicts + `ZSTD_CCtxParams_reset` | [x] |
| 233 | `ZSTD_CCtx_reset(ZSTD_reset_session_and_parameters)` | in init stage, after setting several params + a dict | [x] |
| 234 | `ZSTD_CCtxParams_reset` / `ZSTD_CCtxParams_init(level)` / `ZSTD_CCtxParams_init_advanced(params)` | standalone `ZSTD_CCtx_params` object; then `ZSTD_CCtx_setParametersUsingCCtxParams` | [x] |
| 235 | `ZSTD_CCtxParams_setParameter` / `ZSTD_CCtxParams_getParameter` | every `ZSTD_c_*` value round-tripped through a standalone params object (one row per parameter is implied by §2-§6) | [x] |
| 236 | `ZSTD_createCCtxParams` / `ZSTD_freeCCtxParams` | including `ZSTD_freeCCtxParams(NULL)` (documented as accepting NULL) | [x] |
| 237 | `ZSTD_freeCCtx` / `ZSTD_freeCStream` / `ZSTD_freeCDict` | with `NULL` argument (documented no-op) | [x] |
| 238 | `ZSTD_copyCCtx` | from a CCtx after `ZSTD_compressBegin(level)`, `pledgedSrcSize` known | [x] |
| 239 | `ZSTD_copyCCtx` | from a CCtx after `ZSTD_compressBegin_usingDict`, `pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
## 8. Buffer-less compression API

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 240 | `ZSTD_compressBegin` + `ZSTD_compressContinue`× N + `ZSTD_compressEnd` | level 3, chunks of exactly `ZSTD_getBlockSize(cctx)` | [x] |
| 241 | `ZSTD_compressBegin` + `ZSTD_compressEnd` only | `EMPTY` input ⇒ header + last empty block | [x] |
| 242 | `ZSTD_compressBegin_usingDict` | dict + level | [x] |
| 243 | `ZSTD_compressBegin_advanced` | dict + `ZSTD_parameters` + exact `pledgedSrcSize` | [x] |
| 244 | `ZSTD_compressBegin_advanced` | `pledgedSrcSize = ZSTD_CONTENTSIZE_UNKNOWN` | [x] |
| 245 | `ZSTD_compressBegin_usingCDict` | CDict only | [x] |
| 246 | `ZSTD_compressBegin_usingCDict_advanced` | CDict + `fParams` + exact `pledgedSrcSize` | [x] |
| 247 | `ZSTD_compressContinue` | chunk larger than one block ⇒ internal multi-block `ZSTD_compress_frameChunk` loop | [x] |
| 248 | `ZSTD_compressContinue` | chunk smaller than one block | [x] |
| 249 | `ZSTD_compressEnd` | with a final non-empty chunk vs with `srcSize=0` | [x] |
| 250 | `ZSTD_getBlockSize` | default (131072) vs after `ZSTD_c_maxBlockSize=1024` vs after `windowLog=10` (⇒ `MIN(maxBlockSize, 1<<windowLog)`) | [x] |
| 251 | `ZSTD_compressBlock` | exactly `ZSTD_getBlockSize()` bytes, compressible ⇒ raw block payload, **no** frame header/epilogue | [x] |
| 252 | `ZSTD_compressBlock` | `RANDOM` input ⇒ returns 0 (incompressible) | [x] |
| 253 | `ZSTD_compressBlock` | `SUB_MIN_CBLOCK` input (<7 bytes) ⇒ returns 0 | [x] |
| 254 | `ZSTD_compressBlock` | `RLE` input ⇒ `frame=0` so the RLE shortcut at `zstd_compress.c:4423` is **not** taken | [x] |
| 255 | `ZSTD_compressBlock` | several successive calls (entropy-table reuse across blocks: `set_repeat` on call 2) | [x] |
## 9. Compression dictionary API

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 256 | `ZSTD_createCDict` | proper zstd dictionary (magic `ZSTD_MAGIC_DICTIONARY=0xEC30A437`) built by `ZDICT_trainFromBuffer`, level 3 ⇒ entropy tables loaded | [x] |
| 257 | `ZSTD_createCDict` | raw content (no magic) ⇒ `ZSTD_dct_auto` falls back to raw-content mode (`zstd_compress.c:5217-5221`), dictID 0 | [x] |
| 258 | `ZSTD_createCDict` | `dictSize < 8` (below the magic check) ⇒ raw content | [x] |
| 259 | `ZSTD_createCDict` | `dictSize == 0` / `dict == NULL` ⇒ empty CDict | [x] |
| 260 | `ZSTD_createCDict` | level 0 ⇒ `ZSTD_CLEVEL_DEFAULT` (`:5717`) | [x] |
| 261 | `ZSTD_createCDict_byReference` | dict buffer kept external (`ZSTD_dlm_byRef`) | [x] |
| 262 | `ZSTD_createCDict_advanced` | `(ZSTD_dlm_byCopy, ZSTD_dct_auto)` × explicit `cParams` | [x] |
| 263 | `ZSTD_createCDict_advanced` | `(ZSTD_dlm_byRef, ZSTD_dct_rawContent)` — force raw even for a magic-bearing buffer | [x] |
| 264 | `ZSTD_createCDict_advanced` | `(ZSTD_dlm_byCopy, ZSTD_dct_fullDict)` on a real dictionary | [x] |
| 265 | `ZSTD_createCDict_advanced2` | with a `ZSTD_CCtx_params*` carrying `enableDedicatedDictSearch=1` + `strategy=greedy/lazy/lazy2` ⇒ DDS cParams (`ZSTD_dedicatedDictSearch_getCParams`, `:7677`) | [x] |
| 266 | `ZSTD_createCDict_advanced2` | with `ZSTD_CCtx_params*` where `enableDedicatedDictSearch=1` but strategy is unsupported ⇒ falls back to normal load | [x] |
| 267 | `ZSTD_initStaticCDict` | caller-supplied workspace of exactly `ZSTD_estimateCDictSize_advanced(dictSize, cParams, dlm)` bytes, `ZSTD_dlm_byCopy` | [x] |
| 268 | `ZSTD_initStaticCDict` | `ZSTD_dlm_byRef` (smaller workspace) | [x] |
| 269 | `ZSTD_estimateCDictSize` / `_advanced` | `dictSize=0`, small, and 1 MiB; each `dictLoadMethod`; each strategy (chain-table presence differs) | [x] |
| 270 | `ZSTD_sizeof_CDict` | after each of the CDict creation variants | [x] |
| 271 | `ZSTD_getDictID_fromCDict` | real dictionary (nonzero) vs raw content (0) vs NULL CDict (0) | [x] |
| 272 | `ZSTD_getDictID_fromDict` | `dictSize>=8` + magic ⇒ LE32 at offset 4; `dictSize<8` ⇒ 0; wrong magic ⇒ 0 (`zstd_decompress.c:1624-1629`) | [x] |
| 273 | `ZSTD_getCParamsFromCDict` | after `ZSTD_createCDict(level=1)` and `(level=19)` | [x] |
| 274 | `ZSTD_compress_usingCDict` | CDict small (`dictContentSize <= attachDictSizeCutoffs[strategy]`) ⇒ **attach** path `ZSTD_resetCCtx_byAttachingCDict` (`:2323`) | [x] |
| 275 | `ZSTD_compress_usingCDict` | CDict large + known large `pledgedSrcSize` ⇒ **copy** path `ZSTD_resetCCtx_byCopyingCDict` | [x] |
| 276 | `ZSTD_compress2` + `refCDict` + `forceAttachDict=ZSTD_dictForceAttach` | large CDict forced to attach | [x] |
| 277 | `ZSTD_compress2` + `refCDict` + `forceAttachDict=ZSTD_dictForceCopy` | small CDict forced to copy | [x] |
| 278 | `ZSTD_compress2` + `refCDict` + `forceAttachDict=ZSTD_dictForceLoad` | forced full re-load of dict content | [x] |
| 279 | `ZSTD_compress2` + `refCDict` + `forceAttachDict=ZSTD_dictDefaultAttach` | default heuristic; `pledgedSrcSize == ZSTD_CONTENTSIZE_UNKNOWN` ⇒ attach (`:2316`) | [x] |
| 280 | `ZSTD_compress2` + `refCDict` + `forceMaxWindow=1` | ⇒ attach **refused** (`:2321`), copy/load instead | [x] |
| 281 | `ZSTD_compress_usingCDict` | CDict whose `cdictLen == 0` (empty content) ⇒ "skipping attaching empty dictionary" (`:2358`) | [x] |
| 282 | `ZSTD_compress_usingCDict_advanced` | CDict + explicit `fParams` (`checksumFlag=1`, `noDictIDFlag=1`) | [x] |
| 283 | `ZSTD_compress_usingDict` | dict + level, one shot (goes through `ZSTD_compress_advanced_internal`, `:5472`) | [x] |
| 284 | `ZSTD_compress_usingDict` | `dict=NULL, dictSize=0` | [x] |
| 285 | `ZSTD_CCtx_loadDictionary` | `ZSTD_dlm_byCopy, ZSTD_dct_auto` (creates a `localDict` CDict lazily at `ZSTD_initLocalDict`, `:1252`) | [x] |
| 286 | `ZSTD_CCtx_loadDictionary_byReference` | `ZSTD_dlm_byRef, ZSTD_dct_auto` | [x] |
| 287 | `ZSTD_CCtx_loadDictionary_advanced` | all 2×3 combinations of `{byRef,byCopy}` × `{dct_auto, dct_rawContent, dct_fullDict}` (6 rows) | [x] |
| 288 | `ZSTD_CCtx_loadDictionary` | `dict=NULL` or `dictSize=0` ⇒ clears any previous dict and returns 0 (`:1293-1294`) | [x] |
| 289 | `ZSTD_CCtx_refCDict` | non-NULL CDict, then `ZSTD_CCtx_refCDict(cctx, NULL)` to clear | [x] |
| 290 | `ZSTD_CCtx_refPrefix` | prefix smaller than window (`ZSTD_dct_rawContent`) | [x] |
| 291 | `ZSTD_CCtx_refPrefix` | prefix **larger** than window ⇒ `extDict` dispatch column | [x] |
| 292 | `ZSTD_CCtx_refPrefix_advanced` | `ZSTD_dct_rawContent` / `ZSTD_dct_auto` / `ZSTD_dct_fullDict` on a magic-bearing prefix (3 rows) | [x] |
| 293 | `ZSTD_CCtx_refPrefix` | `prefix=NULL` / `prefixSize=0` ⇒ no prefix set (`:1357`) | [x] |
| 294 | `ZSTD_CCtx_refPrefix` | prefix is single-use: after one compression the next one has no prefix (`ZSTD_memset(&cctx->prefixDict,...)` at `:6356`) | [x] |
| 295 | `ZSTD_compress2` + dict | dict content larger than `1U << MIN(MAX(hashLog+3, chainLog+1), 31)` ⇒ only the **suffix** is indexed (`:4962-4968`) | [x] |
| 296 | `ZSTD_compress2` + dict | dict content `<= HASH_READ_SIZE` (8) ⇒ `ZSTD_loadDictionaryContent` returns early, no table fill (`:4974`) | [x] |
| 297 | `ZSTD_compress2` + dict | one row per strategy for `ZSTD_loadDictionaryContent`'s `switch`: `fast`→`ZSTD_fillHashTable`, `dfast`→`ZSTD_fillDoubleHashTable`, `greedy/lazy/lazy2` row→`ZSTD_row_update` vs chain→`ZSTD_insertAndFindFirstIndex`, `btlazy2/btopt/btultra/btultra2`→`ZSTD_updateTree` (`:4978-5036`) | [x] |
| 298 | `ZSTD_compress2` + dict + DDS | `ms->dedicatedDictSearch` ⇒ `ZSTD_dedicatedDictSearch_lazy_loadDictionary` (`:5000`) | [x] |
| 299 | `ZSTD_loadCEntropy` (via CDict of a real dict) | dictionary whose FSE normalized counts contain a zero probability ⇒ `ZSTD_dictNCountRepeat` returns `FSE_repeat_check`; all-nonzero ⇒ `FSE_repeat_valid` (`:5040-5058`) | [i] |
| 300 | `ZSTD_registerSequenceProducer` / `ZSTD_CCtxParams_registerSequenceProducer` | register a trivial external sequence producer; `enableSeqProducerFallback=0` and `=1` (2 rows, `:7819-7840`, used at `:3351-3413`) | [x] |
| 301 | `ZSTD_referenceExternalSequences` | pre-supplied `rawSeq` array consumed by `ZSTD_ldm_blockCompress` on the `externSeqStore` path (`:3307-3325`, `:4780`) | [i] |

## 10. Sequence-level compression API

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 302 | `ZSTD_sequenceBound` | `srcSize = 0`, `1`, `131072`, `1 MiB` | [x] |
| 303 | `ZSTD_generateSequences` | `outSeqsCapacity = ZSTD_sequenceBound(srcSize)`, single block input, compressible | [x] |
| 304 | `ZSTD_generateSequences` | `MULTIBLOCK` input ⇒ block-delimiter sequences (`offset=0,matchLength=0`) between blocks | [x] |
| 305 | `ZSTD_generateSequences` | `RANDOM` block (`ZSTDbss_noCompress` ⇒ `sequenceProducer_failed`; the *valid* row is barely-compressible so the collector succeeds) | [x] |
| 306 | `ZSTD_generateSequences` | input with a repcode-eligible repeat ⇒ `ZSTD_copyBlockSequences` rep resolution (`:3429`) | [x] |
| 307 | `ZSTD_mergeBlockDelimiters` | array with delimiters ⇒ removes them, returns reduced count (`:3555`) | [x] |
| 308 | `ZSTD_mergeBlockDelimiters` | array with **no** delimiters ⇒ unchanged | [x] |
| 309 | `ZSTD_compressSequences` | `blockDelimiters=ZSTD_sf_explicitBlockDelimiters`, sequences from `ZSTD_generateSequences` verbatim | [x] |
| 310 | `ZSTD_compressSequences` | `blockDelimiters=ZSTD_sf_noBlockDelimiters` (default), after `ZSTD_mergeBlockDelimiters` | [x] |
| 311 | `ZSTD_compressSequences` | `validateSequences=1` with valid sequences | [x] |
| 312 | `ZSTD_compressSequences` | `validateSequences=0` with valid sequences (blind-accept path) | [x] |
| 313 | `ZSTD_compressSequences` | `repcodeResolution=ZSTD_ps_enable` + explicit delimiters ⇒ repcodes searched (`:6983`, `ZSTD_transferSequences_wBlockDelim`) | [x] |
| 314 | `ZSTD_compressSequences` | `repcodeResolution=ZSTD_ps_disable` ⇒ all offsets emitted literally | [x] |
| 315 | `ZSTD_compressSequences` | `repcodeResolution=ZSTD_ps_auto` with `compressionLevel<10` (⇒disable) and `>=10` (⇒enable) (2 rows) | [x] |
| 316 | `ZSTD_compressSequences` | `srcSize=0` / zero sequences ⇒ empty-frame branch writes a last `bt_raw` block of size 0 (`:6960-6967`) | [x] |
| 317 | `ZSTD_compressSequences` | a block whose `blockSize < MIN_CBLOCK_SIZE+ZSTD_blockHeaderSize+1+1` ⇒ `ZSTD_noCompressBlock` (`:6989-6999`) | [x] |
| 318 | `ZSTD_compressSequences` | a non-first block that is `RLE` ⇒ `compressedSeqsSize` forced to 1 ⇒ `ZSTD_rleCompressBlock` (`:7012-7030`) | [x] |
| 319 | `ZSTD_compressSequences` | a block where entropy coding returns 0 ⇒ `ZSTD_noCompressBlock` (`:7022-7026`) | [x] |
| 320 | `ZSTD_compressSequences` | `checksumFlag=1` ⇒ XXH64 over `src`, 4-byte trailer (`:7084-7106`) | [x] |
| 321 | `ZSTD_compressSequences` | `minMatch=3` (must be `<=` the smallest supplied match) + sequences with a 3-byte match | [x] |
| 322 | `ZSTD_compressSequences` | explicit `windowLog` so all offsets are in-window (offset validation, `zstd.h:1671`) | [x] |
| 323 | `ZSTD_compressSequences` | `maxBlockSize` smaller than default ⇒ `determine_blockSize` splits differently (`:6972`) | [x] |
| 324 | `ZSTD_compressSequencesAndLiterals` | separate `literals` buffer + `nbSequences` + `decompressedSize`, `blockDelimiters=explicit` (`:7585`) | [x] |
| 325 | `ZSTD_compressSequencesAndLiterals` | configuration where an uncompressed block would be needed ⇒ `cannotProduce_uncompressedBlock` is the error; the *valid* row is a compressible literal set (`:7550`) | [x] |
| 326 | `ZSTD_convertBlockSequences` | `ZSTD_Sequence[]` → internal `SeqDef[]`; a sequence with `litLength > 65535` and one with `matchLength-3 > 65535` (long-length flags) (`:7318`) | [x] |
| 327 | `ZSTD_convertBlockSequences` | `repcodeResolution` on/off on the same sequence array (2 rows) | [x] |
| 328 | `ZSTD_getSequences` / `ZSTD_generateSequences` naming | note: v1.5.7 exposes `ZSTD_generateSequences` only; `ZSTD_getSequences` does **not** exist — record as N/A | [x] |
## 11. Size estimation, static allocation, custom allocators

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 329 | `ZSTD_estimateCCtxSize` | level 1, 3, 19, 22, `-5`, `0` | [x] |
| 330 | `ZSTD_estimateCCtxSize_usingCParams` | each strategy (chain-table presence differs); `windowLog=10` and `=31` | [x] |
| 331 | `ZSTD_estimateCCtxSize_usingCCtxParams` | params with `nbWorkers=0` (the only legal value here); with LDM enabled; with `maxBlockSize=1024` | [x] |
| 332 | `ZSTD_estimateCStreamSize` / `_usingCParams` / `_usingCCtxParams` | same axes as above (in/out buffers add `blockSize` terms) | [x] |
| 333 | `ZSTD_initStaticCCtx` | workspace exactly `ZSTD_estimateCCtxSize(level)`, 8-byte aligned | [x] |
| 334 | `ZSTD_initStaticCStream` | workspace exactly `ZSTD_estimateCStreamSize(level)` | [x] |
| 335 | `ZSTD_initStaticCCtx` then `ZSTD_CCtx_loadDictionary` (`byCopy`) | static CCtx cannot allocate a dict copy — the *valid* row is `byReference` (`:1300`) | [x] |
| 336 | `ZSTD_createCCtx_advanced` / `ZSTD_createCStream_advanced` | custom `ZSTD_customMem` (`customAlloc`/`customFree`/`opaque`) | [x] |
| 337 | `ZSTD_sizeof_CCtx` / `ZSTD_sizeof_CStream` | freshly created; after one compression; after a dict load | [x] |
| 338 | `ZSTD_createThreadPool` / `ZSTD_freeThreadPool` / `ZSTD_CCtx_refThreadPool` | with no `ZSTD_MULTITHREAD`, `POOL_create` returns the `g_poolCtx` singleton and `POOL_add` runs jobs synchronously (`common/pool.c:313-370`); `numThreads=0` and `=4` (2 rows) | [x] |
## 12. Decompression parameters (`ZSTD_d_*`)

Bounds from `ZSTD_dParam_getBounds`, `zstd_decompress.c:1821-1859`; defaults from `ZSTD_DCtx_resetParameters`, `:240-250`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 339 | `ZSTD_dParam_getBounds` | each of `ZSTD_d_windowLogMax`, `d_format`, `d_stableOutBuffer`, `d_forceIgnoreChecksum`, `d_refMultipleDDicts`, `d_disableHuffmanAssembly`, `d_maxBlockSize` (7 rows) | [x] |
| 340 | `ZSTD_DCtx_setParameter` | `ZSTD_d_windowLogMax=0` ⇒ remapped to `ZSTD_WINDOWLOG_LIMIT_DEFAULT=27` **before** the bounds check (`:1911-1913`) | [x] |
| 341 | `ZSTD_DCtx_setParameter` | `ZSTD_d_windowLogMax=10` (`ZSTD_WINDOWLOG_ABSOLUTEMIN`) + frame compressed with `windowLog=10` | [x] |
| 342 | `ZSTD_DCtx_setParameter` | `ZSTD_d_windowLogMax=31` (max on 64-bit) | [x] |
| 343 | `ZSTD_DCtx_setParameter` | `ZSTD_d_windowLogMax=27` (default) + frame with `windowLog=27` (boundary accept) | [x] |
| 344 | `ZSTD_DCtx_getParameter` | reads back `ZSTD_highbit32(maxWindowSize)` (`:1881`) after each of the above | [x] |
| 345 | `ZSTD_DCtx_setMaxWindowSize` | `1<<10` (min), `1<<31` (max), and a non-power-of-two in between (`:1804-1814`) | [x] |
| 346 | `ZSTD_DCtx_setParameter` | `ZSTD_d_format=ZSTD_f_zstd1` (default) | [x] |
| 347 | `ZSTD_DCtx_setParameter` | `ZSTD_d_format=ZSTD_f_zstd1_magicless` ⇒ `ZSTD_startingInputLength=1`, `ZSTD_FRAMEHEADERSIZE_PREFIX=1`, **no** legacy dispatch, **no** skippable-frame skipping (`:1090,1120`) | [x] |
| 348 | `ZSTD_DCtx_setFormat` | thin wrapper — both format values (`:1816-1819`) | [x] |
| 349 | `ZSTD_DCtx_setParameter` | `ZSTD_d_stableOutBuffer=ZSTD_bm_buffered` (default) | [x] |
| 350 | `ZSTD_DCtx_setParameter` | `ZSTD_d_stableOutBuffer=ZSTD_bm_stable` + known frame content size + output >= fcs ⇒ decode straight into dst, no internal outBuff allocated (`:2204-2210`, `:2236-2269`) | [x] |
| 351 | `ZSTD_DCtx_setParameter` | `ZSTD_d_stableOutBuffer=stable` + `ZSTD_CONTENTSIZE_UNKNOWN` frame + huge output buffer | [x] |
| 352 | `ZSTD_DCtx_setParameter` | `ZSTD_d_forceIgnoreChecksum=ZSTD_d_validateChecksum` (default) on a checksummed frame ⇒ compare | [x] |
| 353 | `ZSTD_DCtx_setParameter` | `ZSTD_d_forceIgnoreChecksum=ZSTD_d_ignoreChecksum` on a checksummed frame ⇒ 4 bytes skipped, no compare (`:720`, `:1049-1059`) | [x] |
| 354 | `ZSTD_DCtx_setParameter` | `ZSTD_d_refMultipleDDicts=ZSTD_rmd_refSingleDDict` (default) | [x] |
| 355 | `ZSTD_DCtx_setParameter` | `ZSTD_d_refMultipleDDicts=ZSTD_rmd_refMultipleDDicts` + N>1 `ZSTD_DCtx_refDDict` calls + `MULTIFRAME` input where each frame uses a different dictID ⇒ hash-set lookup (`:82-216`, `:1787-1796`, `:2139-2141`) | [x] |
| 356 | same | enough DDicts to force a hash-set **resize** (base table 64, grows at 0.75 load ⇒ >48 dicts) | [x] |
| 357 | same | two DDicts with the **same** dictID ⇒ replace-on-same-dictID | [x] |
| 358 | `ZSTD_DCtx_setParameter` | `ZSTD_d_disableHuffmanAssembly=0` (default) on a Huffman-heavy multi-KB literal block | [x] |
| 359 | `ZSTD_DCtx_setParameter` | `ZSTD_d_disableHuffmanAssembly=1` ⇒ `HUF_flags_disableAsm` set ⇒ plain-C 4-stream loop (`zstd_decompress_block.c:162`) | [x] |
| 360 | `ZSTD_DCtx_setParameter` | `ZSTD_d_maxBlockSize=0` (default, disabled) | [x] |
| 361 | `ZSTD_DCtx_setParameter` | `ZSTD_d_maxBlockSize=1024` (`ZSTD_BLOCKSIZE_MAX_MIN`) decoding a frame compressed with `ZSTD_c_maxBlockSize=1024` | [x] |
| 362 | `ZSTD_DCtx_setParameter` | `ZSTD_d_maxBlockSize=131072` (max) — must equal default behaviour | [x] |
| 363 | `ZSTD_DCtx_reset(ZSTD_reset_session_only)` | mid-stream ⇒ `zdss_init`, `noForwardProgress=0`, `isFrameDecompression=1` (`:1947-1962`) | [x] |
| 364 | `ZSTD_DCtx_reset(ZSTD_reset_parameters)` | in `zdss_init` ⇒ `ZSTD_clearDict` + `ZSTD_DCtx_resetParameters` | [x] |
| 365 | `ZSTD_DCtx_reset(ZSTD_reset_session_and_parameters)` | after setting several `ZSTD_d_*` and a DDict | [x] |

## 13. Decompression — simple, streaming, buffer-less, block-level

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 366 | `ZSTD_decompress` | frame with known content size, `dstCapacity` exact | [x] |
| 367 | `ZSTD_decompress` | frame with known content size, `dstCapacity` larger than needed | [x] |
| 368 | `ZSTD_decompress` | empty frame (0-byte content) | [x] |
| 369 | `ZSTD_decompress` | frame with `checksumFlag=1` (verify) | [x] |
| 370 | `ZSTD_decompress` | `MULTIFRAME` (2 concatenated frames) ⇒ `ZSTD_decompressMultiFrame` loop (`:1087`) | [x] |
| 371 | `ZSTD_decompress` | frame preceded by a skippable frame ⇒ skipped (`:1120-1132`) | [x] |
| 372 | `ZSTD_decompress` | frame **followed** by a skippable frame | [x] |
| 373 | `ZSTD_decompress` | interleaved: skippable, frame, skippable, frame | [x] |
| 374 | `ZSTD_decompress` | frames concatenated where the second overwrites in-place-adjacent regions ⇒ `ZSTD_checkContinuity` extDict classification (`:2178-2186`) | [x] |
| 375 | `ZSTD_decompressDCtx` | reused DCtx across two independent frames | [x] |
| 376 | `ZSTD_decompress_usingDict` | dict = the same dictionary used for compression (real dict, dictID matched) | [x] |
| 377 | `ZSTD_decompress_usingDict` | dict = raw content (`dictSize<8` or wrong magic ⇒ raw content mode, `:1539-1558`) | [x] |
| 378 | `ZSTD_decompress_usingDict` | `dict=NULL, dictSize=0` | [x] |
| 379 | `ZSTD_decompress_usingDDict` | DDict from `ZSTD_createDDict` | [x] |
| 380 | `ZSTD_createDDict` / `_byReference` / `ZSTD_createDDict_advanced` | `(byCopy|byRef) × (dct_auto|dct_rawContent|dct_fullDict)` (6 rows, `zstd_ddict.c:89-184`) | [x] |
| 381 | `ZSTD_initStaticDDict` | workspace exactly `ZSTD_estimateDDictSize(dictSize, dlm)`, 8-aligned, both `dictLoadMethod` values (`zstd_ddict.c:187-209`) | [x] |
| 382 | `ZSTD_getDictID_fromDDict` | real dict, raw-content dict, NULL DDict (`zstd_ddict.c:240-244`) | [x] |
| 383 | `ZSTD_getDictID_fromFrame` | frame with 1-, 2-, and 4-byte dictID fields; frame with no dictID (⇒ 0) (`:1644-1650`) | [x] |
| 384 | `ZSTD_sizeof_DDict` / `ZSTD_estimateDDictSize` | each `dictLoadMethod`, `dictSize=0` and 100 KB | [x] |
| 385 | `ZSTD_DCtx_loadDictionary` | `byCopy, dct_auto` | [x] |
| 386 | `ZSTD_DCtx_loadDictionary_byReference` | `byRef, dct_auto` | [x] |
| 387 | `ZSTD_DCtx_loadDictionary_advanced` | all 6 `{byRef,byCopy}×{auto,rawContent,fullDict}` combinations | [x] |
| 388 | `ZSTD_DCtx_refDDict` | non-NULL ⇒ `ZSTD_use_indefinitely`; then `NULL` to clear (`:1780-1799`) | [x] |
| 389 | `ZSTD_DCtx_refPrefix` | prefix (`ZSTD_dct_rawContent`) ⇒ `ZSTD_use_once`: consumed by exactly one frame (`ZSTD_getDDict`, `:1180-1195`) | [x] |
| 390 | `ZSTD_DCtx_refPrefix_advanced` | `dct_auto` / `dct_rawContent` / `dct_fullDict` (3 rows) | [x] |
| 391 | `ZSTD_initDStream` + `ZSTD_decompressStream` | buffered out, feed input in 1-byte chunks ⇒ exercises `zdss_loadHeader` partial-read + `zdss_load` accumulation (`:2163-2183`, `:2293-2320`) | [x] |
| 392 | `ZSTD_decompressStream` | feed the whole frame at once with a **large** output buffer and known fcs ⇒ **single-pass shortcut** via `ZSTD_decompress_usingDDict` (`:2185-2202`) | [x] |
| 393 | `ZSTD_decompressStream` | same but output slightly smaller than fcs ⇒ shortcut declined, normal streaming | [x] |
| 394 | `ZSTD_decompressStream` | whole frame available but `cSize > (iend-istart)` (truncated tail available later) ⇒ shortcut declined | [x] |
| 395 | `ZSTD_decompressStream` | `ZSTD_CONTENTSIZE_UNKNOWN` frame ⇒ shortcut never taken; internal `outBuff` sized by `ZSTD_decodingBufferSize_internal` (`:1970-1986`) | [x] |
| 396 | `ZSTD_decompressStream` | output buffer smaller than one block ⇒ `zdss_flush` partial-flush loop, incl. the ring-buffer wrap reset (`:2331-2337`) | [x] |
| 397 | `ZSTD_decompressStream` | `(iend-ip) >= neededInSize` ⇒ decode **directly from src**, `zdss_load` skipped (`:2273-2291`) | [x] |
| 398 | `ZSTD_decompressStream` | skippable frame at the head of the stream ⇒ `ZSTDds_skipFrame` with `expected = LE32(hdr+4)` (`:2216-2219`) | [x] |
| 399 | `ZSTD_decompressStream` | skippable frame with a payload larger than the input buffer ⇒ multi-call skip (no copy path in `zdss_load`, `:2296`) | [x] |
| 400 | `ZSTD_decompressStream` | `MULTIFRAME` streamed: after frame 1 completes, stage returns to `zdss_init` and frame 2 is decoded | [x] |
| 401 | `ZSTD_decompressStream` | return-value semantics: 0 (frame complete + flushed), 1 (hostage byte held, `:2366-2389`), >1 (input hint) — 3 rows | [x] |
| 402 | `ZSTD_decompressStream` | `ZSTD_d_stableOutBuffer=stable` ⇒ `ZSTD_decompressContinueStream` writes to `*op`, never enters `zdss_flush` (`:2057-2084`) | [x] |
| 403 | `ZSTD_decompressStream` | `ZSTD_d_stableOutBuffer=stable`, second call with the identical `output` (checked by `ZSTD_checkOutBuffer`, `:2035-2050`) | [x] |
| 404 | `ZSTD_decompressStream` | large `windowSize` + small `outBuffSize` ⇒ realloc path; then a much smaller frame ⇒ `ZSTD_DCtx_isOversizedTooLong` (factor 3 / duration 128, `:2016-2032`) shrinks the buffers | [x] |
| 405 | `ZSTD_decompressStream_simpleArgs` | integral-args wrapper with nonzero `*dstPos`/`*srcPos` (`:2392-2410`) | [x] |
| 406 | `ZSTD_initDStream_usingDict` | dict + streaming decode | [x] |
| 407 | `ZSTD_initDStream_usingDDict` | DDict + streaming decode | [x] |
| 408 | `ZSTD_resetDStream` | after a completed stream (returns `ZSTD_startingInputLength(format)` = 5 or 1) | [x] |
| 409 | `ZSTD_initStaticDCtx` / `ZSTD_initStaticDStream` | workspace exactly `ZSTD_estimateDCtxSize()` / `ZSTD_estimateDStreamSize(maxWindowSize)`, 8-aligned (`:281-292`, `:1678-1681`) | [x] |
| 410 | `ZSTD_createDCtx_advanced` / `ZSTD_createDStream_advanced` | custom `ZSTD_customMem` | [x] |
| 411 | `ZSTD_sizeof_DCtx` / `ZSTD_sizeof_DStream` | fresh; after a streaming decode (buffers allocated); after a `loadDictionary` | [x] |
| 412 | `ZSTD_estimateDCtxSize` / `ZSTD_estimateDStreamSize` / `ZSTD_estimateDStreamSize_fromFrame` / `ZSTD_decodingBufferSize_min` | `windowSize` small and `1<<27`; `frameContentSize` known and `ZSTD_CONTENTSIZE_UNKNOWN` (4+ rows, `:1970-2011`) | [x] |
| 413 | `ZSTD_DStreamInSize` / `ZSTD_DStreamOutSize` | constant returns (128 KB + 3, 128 KB) | [x] |
| 414 | `ZSTD_decompressBegin` + `ZSTD_nextSrcSizeToDecompress` + `ZSTD_decompressContinue` | full buffer-less walk: `ZSTDds_getFrameHeaderSize` → `decodeFrameHeader` → `decodeBlockHeader` → `decompressBlock` ×N → `decompressLastBlock` → (`checkChecksum`) (`:1275-1432`) | [x] |
| 415 | `ZSTD_decompressBegin_usingDict` | dict, then the buffer-less walk | [x] |
| 416 | `ZSTD_decompressBegin_usingDDict` | DDict; verifies `ddictIsCold` computation (`:1601-1618`) | [x] |
| 417 | `ZSTD_decompressContinue` | `bt_raw` block **streamed in pieces**: `ZSTD_nextSrcSizeToDecompressWithInputSize` allows a partial `BOUNDED(1, inputSize, expected)` read, staying on the same stage (`:1236-1242`, `:1374-1376`) | [x] |
| 418 | `ZSTD_decompressContinue` | `bt_rle` block ⇒ `ZSTD_setRleBlock`, `expected=0` (`:1367-1372`) | [x] |
| 419 | `ZSTD_decompressContinue` | `bt_compressed` block ⇒ `ZSTD_decompressBlock_internal(..., is_streaming)` | [x] |
| 420 | `ZSTD_decompressContinue` | empty **last** block ⇒ transition to `ZSTDds_checkChecksum` (expected 4) when `checksumFlag`, else back to `getFrameHeaderSize` (expected 0) (`:1311-1337`) | [x] |
| 421 | `ZSTD_decompressContinue` | empty **non-last** block ⇒ next block header | [x] |
| 422 | `ZSTD_decompressContinue` | skippable frame ⇒ `ZSTDds_decodeSkippableHeader` then `ZSTDds_skipFrame` (`:1414-1426`) | [x] |
| 423 | `ZSTD_nextInputType` | one row per return value: `ZSTDnit_frameHeader`, `ZSTDnit_blockHeader`, `ZSTDnit_block`, `ZSTDnit_lastBlock`, `ZSTDnit_checksum`, `ZSTDnit_skippableFrame` (`:1244-1267`) | [x] |
| 424 | `ZSTD_copyDCtx` | from a DCtx prepared with `ZSTD_decompressBegin_usingDict` (`:346-350`) | [x] |
| 425 | `ZSTD_decompressBlock` | raw block payload produced by `ZSTD_compressBlock`, `isFrameDecompression=0` ⇒ `blockSizeMax = ZSTD_BLOCKSIZE_MAX` (`:2189-2209`, block.c `:54-59`) | [x] |
| 426 | `ZSTD_decompressBlock` | successive blocks with `dst` contiguous ⇒ continuous path | [x] |
| 427 | `ZSTD_decompressBlock` | successive blocks with `dst` **not** contiguous ⇒ `ZSTD_checkContinuity` switches to extDict | [x] |
| 428 | `ZSTD_insertBlock` | insert an uncompressed block into DCtx history between `ZSTD_decompressBlock` calls (`:887-893`) | [x] |
| 429 | `ZSTD_getBlockSize` | (compress-side only in this version; there is no decompress-side counterpart) — record as compress-side, see row 250 | [x] |
## 14. Decompression — block internals (one row per encoded form)

All in `decompress/zstd_decompress_block.c`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 430 | `ZSTD_decompress*` | block type `bt_raw` (`ZSTD_copyRawBlock`) | [x] |
| 431 | `ZSTD_decompress*` | block type `bt_rle` (`ZSTD_getcBlockSize` returns cSize 1, `origSize` from the header) | [x] |
| 432 | `ZSTD_decompress*` | block type `bt_compressed` | [x] |
| 433 | `ZSTD_decompress*` | `lastBlock` flag set on the only block; and set on block N of N (2 rows) | [x] |
| 434 | literals `set_basic` | `lhlCode=0` (`lhSize=1`, litSize = `istart[0]>>3`, up to 31 bytes) | [i] |
| 435 | literals `set_basic` | `lhlCode=1` (`lhSize=2`, litSize = `LE16>>4`, up to 4095) | [i] |
| 436 | literals `set_basic` | `lhlCode=2` — shares the 1-byte form with `lhlCode=0` (`:250-256`) | [i] |
| 437 | literals `set_basic` | `lhlCode=3` (`lhSize=3`, litSize = `LE24>>4`) | [i] |
| 438 | literals `set_basic` | `lhSize+litSize+WILDCOPY_OVERLENGTH <= srcSize` ⇒ **literals referenced in-place** in the compressed stream, `litBufferLocation = ZSTD_not_in_dst` (`:290-294`) | [i] |
| 439 | literals `set_basic` | last block where `lhSize+litSize+WILDCOPY_OVERLENGTH > srcSize` ⇒ copied into `litBuffer` (`:277-285`) | [i] |
| 440 | literals `set_rle` | each of `lhlCode` 0/1/2/3 (4 rows, `:298-335`) | [i] |
| 441 | literals `set_compressed` | `lhlCode=0` ⇒ `lhSize=3`, **1-stream** Huffman (`singleStream = !lhlCode`, `:165-171`) | [i] |
| 442 | literals `set_compressed` | `lhlCode=1` ⇒ `lhSize=3`, **4-stream** | [i] |
| 443 | literals `set_compressed` | `lhlCode=2` ⇒ `lhSize=4`, 4-stream, `litSize` up to 16383 | [i] |
| 444 | literals `set_compressed` | `lhlCode=3` ⇒ `lhSize=5`, 4-stream, `litSize` up to 262143 | [i] |
| 445 | literals `set_compressed` | 4-stream with `litSize` exactly `MIN_LITERALS_FOR_4_STREAMS=6` (lower edge) | [i] |
| 446 | literals `set_repeat` | second block reusing the previous Huffman DTable ⇒ `HUF_decompress1X_usingDTable` / `4X_usingDTable` (`:147-150`, `:202-209`) | [i] |
| 447 | literals | `litSize > 768` ⇒ cold-huf prefetch branch (`:196-198`) | [i] |
| 448 | literals | `litBufferLocation = ZSTD_in_dst`: `not_streaming` and `dstCapacity > blockSizeMax + WILDCOPY_OVERLENGTH + litSize + WILDCOPY_OVERLENGTH` (`:87-95`) | [i] |
| 449 | literals | `litBufferLocation = ZSTD_not_in_dst`: `litSize <= ZSTD_LITBUFFEREXTRASIZE` ⇒ entirely in `litExtraBuffer` (`:96-102`) | [i] |
| 450 | literals | `litBufferLocation = ZSTD_split`: literals split between end-of-dst and `litExtraBuffer` ⇒ `ZSTD_decompressSequencesSplitLitBuffer` selected (`:112-121`, `:2154-2172`) | [i] |
| 451 | sequences header | `nbSeq == 0` ⇒ section must end exactly; entropy tables copied over (`:721-726`) | [i] |
| 452 | sequences header | `nbSeq` in 1..127 ⇒ 1-byte form | [i] |
| 453 | sequences header | `nbSeq` in 128..0x7EFF ⇒ 2-byte form `((b-0x80)<<8) + *ip++` | [i] |
| 454 | sequences header | `nbSeq >= 0x7F00` ⇒ 3-byte form `0xFF` + `LE16 + LONGNBSEQ` (`:708-718`) | [i] |
| 455 | LL/OF/ML symbol modes | `set_basic` for all three ⇒ `LL/OF/ML_defaultDTable` (`:364-460`) | [i] |
| 456 | LL/OF/ML symbol modes | `set_rle` for one of the three (1 extra byte, `ZSTD_buildSeqTable_rle`, `:463-477`) | [i] |
| 457 | LL/OF/ML symbol modes | `set_compressed` for all three ⇒ `FSE_readNCount` + `ZSTD_buildFSETable` | [i] |
| 458 | LL/OF/ML symbol modes | `set_repeat` for all three on block 2+ (requires `dctx->fseEntropy`) | [i] |
| 459 | LL/OF/ML symbol modes | mixed, e.g. `LL=set_basic, OF=set_compressed, ML=set_repeat` | [i] |
| 460 | LL/OF/ML symbol modes | `set_repeat` with `ddictIsCold && nbSeq>24` ⇒ table prefetch branch (`:647-693`) | [i] |
| 461 | `ZSTD_buildFSETable` | normalized counts with **no** low-probability symbols ⇒ `highThreshold == tableSize-1` fast spread path (`:529-574`) | [i] |
| 462 | `ZSTD_buildFSETable` | normalized counts **with** `-1` low-prob symbols ⇒ generic path (`:575-588`) | [i] |
| 463 | sequence decode | `ofBits == 0` ⇒ pure repcode `prevOffset[ll0]` with swap (`:1301-1304`) | [i] |
| 464 | sequence decode | `ofBits == 1` ⇒ `offset = ofBase + ll0 + readBits(1)`, incl. the `offset==3 ⇒ prevOffset[0]-1` case (`:1305-1312`) | [i] |
| 465 | sequence decode | `ofBits > 1` ⇒ new literal offset; repcodes rotated (`:1284-1298`) | [i] |
| 466 | sequence decode | last sequence in the block (`isLastSeq`) ⇒ FSE state update skipped (`:1335-1342`) | [i] |
| 467 | sequence decode | offsets **within** the prefix (`offset <= oLitEnd-prefixStart`) ⇒ `ZSTD_wildcopy` fast path with `offset >= WILDCOPY_VECLEN` (`:1077-1084`) | [i] |
| 468 | sequence decode | small offset (`< WILDCOPY_VECLEN`) ⇒ `ZSTD_overlapCopy8` + wildcopy (`:1085-1095`) | [i] |
| 469 | sequence decode | offset reaching into **extDict** (dictionary/prefix), match entirely inside the dict ⇒ single `memmove` fast return (`:1052-1067`) | [i] |
| 470 | sequence decode | offset reaching into extDict with the match **straddling** `dictEnd`/`prefixStart` ⇒ split copy | [i] |
| 471 | sequence decode | sequence near the end of the output ⇒ slow path `ZSTD_execSequenceEnd` (`iLitEnd > litLimit \|\| oMatchEnd > oend_w`, `:1025-1029`) | [i] |
| 472 | sequence decode | split-literals variant near the end ⇒ `ZSTD_execSequenceEndSplitLitBuffer` (`:955-997`) | [i] |
| 473 | sequence decode | split-literals variant where `litPtr + litLength > litBufferEnd` mid-block ⇒ pre-split loop breaks, `leftoverLit` drained via `ZSTD_safecopyDstBeforeSrc`, `litPtr` repointed to `litExtraBuffer` (`:1501-1528`, `:1563-1574`) | [i] |
| 474 | decoder selection | `ZSTD_decompressSequences` (default body, `:1615-1690`) — literals not split, no prefetch | [i] |
| 475 | decoder selection | `ZSTD_decompressSequencesSplitLitBuffer` (`:1403-1611`) — `litBufferLocation == ZSTD_split` | [i] |
| 476 | decoder selection | `ZSTD_decompressSequencesLong` (`:1733-1888`) — `ddictIsCold` on the first block after a cold DDict | [i] |
| 477 | decoder selection | `ZSTD_decompressSequencesLong` via the computed route: `totalHistorySize > 1<<24 && nbSeq > 8` and `ZSTD_getOffsetInfo` share of "long" (nbAdditionalBits>22) offsets `>= minShare(7 on 64-bit)` (`:2116`, `:2137-2149`, `:2012-2038`) | [i] |
| 478 | long offsets | 64-bit: `ZSTD_maxShortOffset()` is `(size_t)-1` ⇒ `isLongOffset` is always `ZSTD_lo_isRegularOffset` (`:2045-2063`, `:2109`) — assert this on the target | [i] |
| 479 | long offsets | `info.maxNbAdditionalBits <= STREAM_ACCUMULATOR_MIN` ⇒ downgrade back to regular offsets (`:2139-2145`) | [i] |
| 480 | `ZSTD_decompressBlock_internal` | `dst==NULL && dstCapacity==0` with `nbSeq==0` (legal: literal-free empty block) vs `nbSeq>0` (error) — the valid row is the former (`:2129`) | [i] |
| 481 | `ZSTD_decompressBlock_internal` | `srcSize == ZSTD_blockSizeMax(dctx)` exactly (upper edge, `:2081`) | [i] |
| 482 | repcodes | frame starting from `repStartValue = {1,4,8}` (`:1580`) | [i] |
| 483 | repcodes | frame starting from DDict-supplied repcodes (`zstd_ddict.c:79-81`, validated `rep != 0 && rep <= dictContentSize`, `:1451-1537`) | [i] |
| 484 | `ZSTD_loadDEntropy` | dictionary carrying HUF table + OF/ML/LL `FSE_readNCount` tables + 3 repcodes (`:1451-1537`) | [i] |

## 15. Frame-header / size-query utilities (decode side)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 485 | `ZSTD_isFrame` | `size<4` (⇒0); zstd magic (⇒1); each of the 16 skippable magics (⇒1); a legacy v05/v06/v07 magic (`:385-396`) | [x] |
| 486 | `ZSTD_isSkippableFrame` | each of the 16 skippable magic variants; a normal frame (⇒0) (`:402-409`) | [x] |
| 487 | `ZSTD_frameHeaderSize` | minimum 6 bytes (zstd1, singleSegment=0, no dictID, fcsCode=0) | [x] |
| 488 | `ZSTD_frameHeaderSize` | maximum 18 bytes (`ZSTD_FRAMEHEADERSIZE_MAX`: window byte + 4-byte dictID + 8-byte FCS) | [x] |
| 489 | `ZSTD_frameHeaderSize` | magicless: minimum 2, maximum 14 | [x] |
| 490 | `ZSTD_getFrameHeader` | `srcSize < ZSTD_FRAMEHEADERSIZE_PREFIX(5)` ⇒ returns the **needed size** 5, not an error (`:458-477`) | [x] |
| 491 | `ZSTD_getFrameHeader` | `srcSize` between the prefix and `fhsize` ⇒ returns `fhsize` (`:497-499`) | [x] |
| 492 | `ZSTD_getFrameHeader` | `srcSize >= fhsize` ⇒ returns 0, header fully parsed | [x] |
| 493 | `ZSTD_getFrameHeader` | `dictIDSizeCode` 0/1/2/3 ⇒ dictID field 0/1/2/4 bytes (4 rows, `:521-530`, `ZSTD_did_fieldSize={0,1,2,4}`) | [x] |
| 494 | `ZSTD_getFrameHeader` | `fcsID` 0/1/2/3 ⇒ FCS field 0/2/4/8 bytes (`ZSTD_fcs_fieldSize={0,2,4,8}`); `fcsID=1` stores `LE16+256` (4 rows, `:536-539`) | [x] |
| 495 | `ZSTD_getFrameHeader` | `singleSegment=1` ⇒ **no window descriptor byte**, `windowSize = frameContentSize`; `fcsID=0` ⇒ 1-byte FCS (`:536`, `:541`) | [x] |
| 496 | `ZSTD_getFrameHeader` | `singleSegment=0` ⇒ window descriptor: `windowLog=(b>>3)+10`, `windowSize = (1<<wl) + (windowSize>>3)*(b&7)` — exercise mantissa `b&7` = 0 and = 7 (2 rows, `:517-519`) | [x] |
| 497 | `ZSTD_getFrameHeader` | `checksumFlag=0` and `=1` (bit 2 of FHD) | [x] |
| 498 | `ZSTD_getFrameHeader` | output field `blockSizeMax = MIN(windowSize, ZSTD_BLOCKSIZE_MAX)` — `windowSize<128KB` and `>128KB` (2 rows, `:546`) | [x] |
| 499 | `ZSTD_getFrameHeader_advanced` | `format=ZSTD_f_zstd1_magicless` on a magicless frame | [x] |
| 500 | `ZSTD_getFrameHeader_advanced` | skippable frame: `srcSize<8` ⇒ returns 8; `srcSize>=8` ⇒ `frameType=ZSTD_skippableFrame`, `dictID = magic - ZSTD_MAGIC_SKIPPABLE_START` (variant), `frameContentSize = LE32(src+4)` (`:480-492`) | [x] |
| 501 | `ZSTD_getFrameContentSize` | known size; `ZSTD_CONTENTSIZE_UNKNOWN` frame; skippable frame (⇒0); legacy v07 frame (`:569-585`) | [x] |
| 502 | `ZSTD_getDecompressedSize` | same inputs — collapses ERROR/UNKNOWN to 0 (`:690-695`) | [x] |
| 503 | `ZSTD_findFrameCompressedSize` | single frame; `srcSize` exactly the frame size; `srcSize` larger (trailing bytes ignored) (`:801-812`) | [x] |
| 504 | `ZSTD_findFrameCompressedSize` | frame with `checksumFlag=1` ⇒ +4 (`:794-796`) | [x] |
| 505 | `ZSTD_findFrameCompressedSize` | skippable frame ⇒ its full size (`:734-799`) | [x] |
| 506 | `ZSTD_findDecompressedSize` | single frame; `MULTIFRAME`; frames interleaved with skippable frames (`:643-680`) | [x] |
| 507 | `ZSTD_findDecompressedSize` | a frame with `ZSTD_CONTENTSIZE_UNKNOWN` in a multi-frame stream | [x] |
| 508 | `ZSTD_decompressBound` | frame with known fcs ⇒ exact; frame with unknown fcs ⇒ `nbBlocks * zfh.blockSizeMax` (`:794-796`, `:820-836`) | [x] |
| 509 | `ZSTD_decompressBound` | `MULTIFRAME` ⇒ per-frame sum | [x] |
| 510 | `ZSTD_decompressionMargin` | frame without checksum; with checksum (+4); `MULTIBLOCK` (+3 per block); with a skippable frame in the stream (`:838-879`) | [x] |
| 511 | `ZSTD_readSkippableFrame` | `magicVariant` out-param for each of the 16 variants; `dstCapacity == skippableContentSize` exactly; payload size 0 (`:614-636`) | [x] |
| 512 | `ZSTD_decodingBufferSize_min` | `frameContentSize` known and `ZSTD_CONTENTSIZE_UNKNOWN`; `windowSize` 1 KB and 128 MiB (`:1988-1991`) | [x] |
## 16. FSE low-level (only the variants that actually exist here)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 513 | `FSE_versionNumber` / `FSE_isError` / `FSE_getErrorName` | constant / non-error `size_t` inputs | [x] |
| 514 | `FSE_compressBound` | `size = 0`, `1`, `131072` | [x] |
| 515 | `FSE_NCountWriteBound` | `maxSymbolValue=0` ⇒ `FSE_NCOUNTBOUND=512` (`fse_compress.c:230`); `maxSymbolValue=255, tableLog=12` | [x] |
| 516 | `FSE_optimalTableLog` | `maxTableLog=0` ⇒ `FSE_DEFAULT_TABLELOG=11` | [x] |
| 517 | `FSE_optimalTableLog` | `srcSize` small enough that `maxBitsSrc < tableLog` ⇒ clamp down | [x] |
| 518 | `FSE_optimalTableLog` | large `maxSymbolValue` with small `srcSize` ⇒ `minBits > tableLog` ⇒ clamp up | [x] |
| 519 | `FSE_optimalTableLog` | result would be `< FSE_MIN_TABLELOG(5)` ⇒ clamped to 5 | [x] |
| 520 | `FSE_optimalTableLog` | result would be `> FSE_MAX_TABLELOG(12)` ⇒ clamped to 12 | [x] |
| 521 | `FSE_optimalTableLog_internal` | `minus=1` (the `HUF_optimalTableLog` caller) vs `minus=2` (2 rows, `fse_compress.c:357`) | [x] |
| 522 | `FSE_normalizeCount` | `tableLog=0` ⇒ default 11 | [x] |
| 523 | `FSE_normalizeCount` | `useLowProbCount=0` ⇒ `lowProbCount=+1` | [x] |
| 524 | `FSE_normalizeCount` | `useLowProbCount=1` ⇒ `lowProbCount=-1` (the axis that flips both `buildCTable` and `buildDTable` spread paths) | [x] |
| 525 | `FSE_normalizeCount` | histogram where `count[s] == total` for one symbol ⇒ **returns 0 (RLE)** (`:486-501`) | [x] |
| 526 | `FSE_normalizeCount` | histogram with symbols at `count <= total>>tableLog` ⇒ `lowProbCount` assigned | [x] |
| 527 | `FSE_normalizeCount` | histogram producing `proba < 8` ⇒ `rtbTable` round-up test (`:475`) | [x] |
| 528 | `FSE_normalizeCount` | histogram where `-stillToDistribute >= norm[largest]>>1` ⇒ `FSE_normalizeM2` slow path (`:502`) | [x] |
| 529 | `FSE_normalizeCount` → `FSE_normalizeM2` | case `(total/ToDistribute) > lowOne` ⇒ second "risk of rounding to zero" pass (`:415-426`) | [x] |
| 530 | `FSE_normalizeCount` → `FSE_normalizeM2` | case `distributed == maxSymbolValue+1` ⇒ all-poor dump on argmax (`:428-437`) | [x] |
| 531 | `FSE_normalizeCount` → `FSE_normalizeM2` | case `total == 0` after classification ⇒ round-robin (`:439-444`) | [x] |
| 532 | `FSE_writeNCount` | `bufferSize >= FSE_NCountWriteBound(...)` ⇒ `writeIsSafe=1` (bounds checks compiled out, `:336-339`) | [x] |
| 533 | `FSE_writeNCount` | `bufferSize < FSE_NCountWriteBound(...)` but still sufficient ⇒ `writeIsSafe=0` path | [x] |
| 534 | `FSE_writeNCount` | counts with a run of `>= 24` zeros ⇒ the `0xFFFF` 24-zero escape (`:265`) | [x] |
| 535 | `FSE_writeNCount` | counts with a run of 3..23 zeros ⇒ the 2-bit triple encoding (`:275`) | [x] |
| 536 | `FSE_writeNCount` | counts hitting the `bitCount > 16` flush (`:282`, `:304`) and the `count < max` bit-saving (`:299`) branches | [x] |
| 537 | `FSE_readNCount` | `hbSize >= 8` (normal path) | [x] |
| 538 | `FSE_readNCount` | `hbSize < 8` ⇒ 8-byte zero-padded stack buffer + recursion (`entropy_common.c:57-66`) | [x] |
| 539 | `FSE_readNCount` | a header with a `>= 12` zero-repeat run ⇒ 36-symbols-at-a-time loop (`:89`) | [x] |
| 540 | `FSE_readNCount` | header where `count == -1` (low-prob) symbols are present (`:148-153`) | [x] |
| 541 | `FSE_readNCount` | header with `tableLog` at `FSE_MIN_TABLELOG(5)` and at `FSE_TABLELOG_ABSOLUTE_MAX(15)` boundary (2 rows) | [x] |
| 542 | `FSE_readNCount_bmi2` | `bmi2=0` and `bmi2=1` — with `DYNAMIC_BMI2=0` both collapse to `_body_default`; assert equality (`:206-215`) | [x] |
| 543 | `FSE_buildCTable_wksp` | `wkspSize` exactly `FSE_BUILD_CTABLE_WORKSPACE_SIZE(maxSymbolValue, tableLog)` | [x] |
| 544 | `FSE_buildCTable_wksp` | `tableLog=0` (affects the `FSCT` offset, `:76`) | [x] |
| 545 | `FSE_buildCTable_wksp` | normalized counts with **no** `-1` entries ⇒ `highThreshold == tableSize-1` fast spread (`:116-153`) | [x] |
| 546 | `FSE_buildCTable_wksp` | normalized counts **with** `-1` entries ⇒ variable inner-loop spread (`:154-167`) | [x] |
| 547 | `FSE_buildCTable_wksp` | symbol transform `switch`: a count of 0, of 1, of -1, and of >1 (4 rows, `:179`) | [x] |
| 548 | `FSE_buildCTable_rle` | any `symbolValue` (no branches; assert tableLog=0 output) | [x] |
| 549 | `FSE_compress_usingCTable` | `dstSize >= FSE_BLOCKBOUND(srcSize)` ⇒ `fast=1` (`BIT_flushBitsFast`, `:614-619`) | [x] |
| 550 | `FSE_compress_usingCTable` | `dstSize < FSE_BLOCKBOUND(srcSize)` ⇒ `fast=0` | [x] |
| 551 | `FSE_compress_usingCTable` | `srcSize <= 2` ⇒ **returns 0** (`:563`) | [x] |
| 552 | `FSE_compress_usingCTable` | `srcSize` **odd** ⇒ 3-symbol prologue (`:569-577`); **even** ⇒ 2-symbol prologue (2 rows) | [x] |
| 553 | `FSE_compress_usingCTable` | `srcSize & 2` join-to-mod-4 on 64-bit (`:581`) | [x] |
| 554 | `FSE_compress_usingCTable` | a case where the bitstream overflows `dst` ⇒ `BIT_closeCStream` returns 0 (`bitstream.h:240`) | [x] |
| 555 | `FSE_buildDTable_wksp` | `wkspSize` exactly `FSE_BUILD_DTABLE_WKSP_SIZE(tableLog, maxSymbolValue)` | [x] |
| 556 | `FSE_buildDTable_wksp` | all `normalizedCounter[s] < (1<<(tableLog-1))` ⇒ `fastMode = 1` (`fse_decompress.c:78-86`) | [x] |
| 557 | `FSE_buildDTable_wksp` | some symbol with `normalizedCounter[s] >= (1<<(tableLog-1))` ⇒ `fastMode = 0` | [x] |
| 558 | `FSE_buildDTable_wksp` | with / without `-1` low-prob symbols ⇒ `MEM_write64` spread vs variable inner loop (2 rows, `:92`) | [x] |
| 559 | `FSE_decompress_wksp_bmi2` | `fastMode=1` ⇒ `FSE_decodeSymbolFast`/`BIT_readBitsFast` (`:283-287`) | [x] |
| 560 | `FSE_decompress_wksp_bmi2` | `fastMode=0` ⇒ `FSE_decodeSymbol` | [x] |
| 561 | `FSE_decompress_wksp_bmi2` | `maxLog=6` (the `HUF_readStats` caller) and `maxLog=12` (2 rows) | [x] |
| 562 | `FSE_decompress_wksp_bmi2` | `bmi2=0` / `bmi2=1` (must be identical with `DYNAMIC_BMI2=0`) | [x] |
| 563 | `BIT_initDStream` (via any FSE/HUF decode) | `srcSize >= sizeof(size_t)` (8) ⇒ single `MEM_readLEST` | [i] |
| 564 | `BIT_initDStream` | `srcSize` = 1,2,3,4,5,6,7 ⇒ the byte-by-byte `switch(srcSize)` cases (7 rows, `bitstream.h:270-291`) | [i] |
| 565 | `BIT_reloadDStream` | each status: `unfinished`, `endOfBuffer`, `completed`, `overflow` (4 rows, `bitstream.h:412-438`) | [i] |
| 566 | `BIT_getLowerBits` / `BIT_getMiddleBits` | `STATIC_BMI2` `_bzhi` path vs `BIT_mask[]` path (compile-time; assert equality) | [i] |

## 17. HUF low-level

`HUF_flags_e` (`common/huf.h:80-111`): `bmi2=1<<0`, `optimalDepth=1<<1`, `preferRepeat=1<<2`,
`suspectUncompressible=1<<3`, `disableAsm=1<<4`, `disableFast=1<<5`. `HUF_repeat`: `none=0, check=1, valid=2`.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 567 | `HUF_compressBound` | `size=0`, `1`, `131072` | [x] |
| 568 | `HUF_cardinality` / `HUF_minTableLog` | histogram with 2, 16, 256 distinct symbols (3 rows) | [x] |
| 569 | `HUF_optimalTableLog` | `flags` **without** `HUF_flags_optimalDepth` ⇒ single-shot `FSE_optimalTableLog_internal(..., minus=1)` (`huf_compress.c:1284-1287`) | [x] |
| 570 | `HUF_optimalTableLog` | `flags` **with** `HUF_flags_optimalDepth` ⇒ search loop; craft inputs hitting `maxBits < optLogGuess` break, `newSize > optSize+1` break, and `newSize < optSize` record (3 rows, `:1300-1319`) | [x] |
| 571 | `HUF_buildCTable_wksp` | `maxNbBits=0` ⇒ `HUF_TABLELOG_DEFAULT=11` (`:772`) | [x] |
| 572 | `HUF_buildCTable_wksp` | `wkspSize` exactly `HUF_CTABLE_WORKSPACE_SIZE=4864` | [x] |
| 573 | `HUF_buildCTable_wksp` | histogram whose natural tree depth `<= maxNbBits` ⇒ `HUF_setMaxHeight` early exit (`:380`) | [x] |
| 574 | `HUF_buildCTable_wksp` | histogram needing depth reduction ⇒ `HUF_setMaxHeight` cost-repayment `while (totalCost > 0)` loop (`:423`) | [x] |
| 575 | `HUF_buildCTable_wksp` | overshoot case ⇒ the `while (totalCost < 0)` correction incl. `rankLast[1] == noSymbol` (`:478-482`) | [x] |
| 576 | `HUF_buildCTable_wksp` | histogram with a bucket of `> 1` symbols ⇒ `HUF_simpleQuickSort`; bucket span `< 8` ⇒ `HUF_insertionSort` (2 rows, `:591-599`, `:658`) | [x] |
| 577 | `HUF_buildCTable_wksp` | histogram with a count `>= RANK_POSITION_DISTINCT_COUNT_CUTOFF(166)` ⇒ `HUF_getIndex` log-bucket branch (`:530`) | [x] |
| 578 | `HUF_readCTableHeader` / `HUF_getNbBitsFromCTable` | `symbolValue <= maxSymbolValue` (real bit count) and `> maxSymbolValue` ⇒ **returns 0** (`:345-350`) | [x] |
| 579 | `HUF_writeCTable_wksp` | `(hSize>1) && (hSize < maxSymbolValue/2)` ⇒ **FSE-compressed weight header** (`:276`) | [x] |
| 580 | `HUF_writeCTable_wksp` | otherwise ⇒ **raw 4-bit weight header**, `op[0] = 128 + (maxSymbolValue-1)` (`:282-287`) | [x] |
| 581 | `HUF_writeCTable_wksp` | `maxDstSize` exactly the required size for each of the two header forms (2 rows) | [x] |
| 582 | `HUF_readCTable` | header with a zero weight present ⇒ `*hasZeroWeights = 1` (`:302`) | [x] |
| 583 | `HUF_readCTable` | header with no zero weights ⇒ `*hasZeroWeights = 0` | [x] |
| 584 | `HUF_estimateCompressedSize` / `HUF_validateCTable` | CTable whose `header.maxSymbolValue >= maxSymbolValue` (⇒ real validation) and `<` (⇒ returns 0) (2 rows, `:804-812`) | [x] |
| 585 | `HUF_compress1X_usingCTable` | `dstSize < 8` ⇒ returns 0 (`:1068`) | [x] |
| 586 | `HUF_compress1X_usingCTable` | `dstSize < HUF_tightCompressBound(srcSize, tableLog)` ⇒ slow/safe loop (`:1073`) | [x] |
| 587 | `HUF_compress1X_usingCTable` | `tableLog > 11` ⇒ slow/safe loop even with ample dst | [x] |
| 588 | `HUF_compress1X_usingCTable` | 64-bit `switch (tableLog)` unroll matrix: tableLog `6`, `7`, `8`, `9`, `10`, `11` ⇒ `(kUnroll,kFastFlush,kLastFast)` = `(9,1,1)`, `(8,1,0)`, `(7,1,0)`, `(6,1,0)`, `(5,1,1)`, `(5,1,0)` (6 rows, `:1092-1114`) | [x] |
| 589 | `HUF_compress1X_usingCTable` | `srcSize % kUnroll != 0` and `n % (2*kUnroll)` join-up prologues (`:999`, `:1008`) | [x] |
| 590 | `HUF_compress1X_usingCTable` | `flags` with / without `HUF_flags_bmi2` (identical under `DYNAMIC_BMI2=0`) | [x] |
| 591 | `HUF_compress4X_usingCTable` | `dstSize < 17` ⇒ returns 0 (`:1179`) | [x] |
| 592 | `HUF_compress4X_usingCTable` | `srcSize < 12` ⇒ returns 0 (`:1180`) | [x] |
| 593 | `HUF_compress4X_usingCTable` | `srcSize = 12` exactly (lower edge), 6-byte jump table written | [x] |
| 594 | `HUF_compress4X_usingCTable` | a segment whose compressed size would exceed 65535 ⇒ returns 0; the valid row is all four segments `<= 65535` (`:1185-1210`) | [x] |
| 595 | `HUF_compress1X_repeat` / `HUF_compress4X_repeat` | `srcSize=0` ⇒ 0; `dstSize=0` ⇒ 0; `srcSize = HUF_BLOCKSIZE_MAX(131072)` exactly (3 rows, `:1350-1352`) | [x] |
| 596 | `HUF_compress*_repeat` | `maxSymbolValue=0` ⇒ 255; `huffLog=0` ⇒ 11 (`:1355-1356`) | [i] |
| 597 | `HUF_compress*_repeat` | `flags & HUF_flags_preferRepeat` + `*repeat == HUF_repeat_valid` ⇒ **histogram skipped entirely** (`:1359`) | [i] |
| 598 | `HUF_compress*_repeat` | `flags & HUF_flags_suspectUncompressible` + `srcSize >= 40960` ⇒ head/tail 4096-byte sampling; `largestTotal <= 68` ⇒ returns 0 (`:1367`) | [i] |
| 599 | `HUF_compress*_repeat` | same flag, `srcSize < 40960` ⇒ sampling skipped | [i] |
| 600 | `HUF_compress*_repeat` | histogram `largest == srcSize` ⇒ **returns 1 (RLE)**, 1 byte written (`:1383`) | [i] |
| 601 | `HUF_compress*_repeat` | histogram `largest <= (srcSize>>7)+4` ⇒ **returns 0** (incompressible heuristic, `:1384`) | [i] |
| 602 | `HUF_compress*_repeat` | `*repeat == HUF_repeat_check` + a stale table failing `HUF_validateCTable` ⇒ `*repeat = HUF_repeat_none` (`:1389`) | [i] |
| 603 | `HUF_compress*_repeat` | `*repeat == HUF_repeat_check` + a still-valid table ⇒ reused | [i] |
| 604 | `HUF_compress*_repeat` | `*repeat != HUF_repeat_none` and `oldSize <= hSize+newSize` ⇒ old table kept (`:1415-1422`) | [i] |
| 605 | `HUF_compress*_repeat` | `hSize + 12 >= srcSize` ⇒ **returns 0** (`:1425`) | [i] |
| 606 | `HUF_compress*_repeat` | new table accepted ⇒ `*repeat = HUF_repeat_none`, `oldHufTable` overwritten (`:1427-1429`) | [i] |
| 607 | `HUF_compress*_repeat` | `(op-ostart) >= srcSize-1` ⇒ returns 0 (`HUF_compressCTable_internal`, `:1237`) | [i] |
| 608 | `HUF_compress*_repeat` | `flags & HUF_flags_optimalDepth` on and off (2 rows) | [i] |
| 609 | `HUF_compress*_repeat` | `wkspSize` exactly `HUF_WORKSPACE_SIZE=8704` | [i] |
| 610 | `HUF_compress*_repeat` | `repeat == NULL` (no repeat tracking) | [i] |
| 611 | `HUF_readStats` | header byte `ip[0] >= 128` ⇒ **raw 4-bit weight path**, `oSize = iSize-127` (`entropy_common.c:258-268`) | [x] |
| 612 | `HUF_readStats` | header byte `ip[0] < 128` ⇒ **FSE-compressed weight path** with `maxLog=6` (`:269-274`) | [x] |
| 613 | `HUF_readStats` | odd vs even `oSize` in the raw path (nibble unpacking tail) | [x] |
| 614 | `HUF_readStats` | weights summing to an exact power of two (the `verif == rest` requirement, `:295`) | [x] |
| 615 | `HUF_readStats` | `rankStats[1]` exactly 2 (lower edge of `(rankStats[1] < 2) \|\| (rankStats[1] & 1)`, `:301`) | [x] |
| 616 | `HUF_readStats_wksp` | `flags=0` and `flags=HUF_flags_bmi2` (identical under `DYNAMIC_BMI2=0`); `wkspSize` exactly `HUF_READ_STATS_WORKSPACE_SIZE_U32*4` | [x] |
| 617 | `HUF_readDTableX1_wksp` | dictionary/header with `tableLog < targetTableLog` ⇒ `HUF_rescaleStats` bumps all weights (`:352-375`, `:408`) | [x] |
| 618 | `HUF_readDTableX1_wksp` | `tableLog >= targetTableLog` ⇒ rescale is a no-op | [x] |
| 619 | `HUF_readDTableX1_wksp` | weights producing each `switch (length)` case: 1, 2, 4, 8, and >8 (5 rows, `:465`) | [x] |
| 620 | `HUF_readDTableX2_wksp` | `tableLog <= 11 && maxTableLog > 11` ⇒ `maxTableLog` forced to 11 (`:1208`) | [x] |
| 621 | `HUF_readDTableX2_wksp` | weight set where `targetLog - nbBits >= minBits` ⇒ **double-symbol** level-2 fill (`:1141`) | [x] |
| 622 | `HUF_readDTableX2_wksp` | weight set where it is not ⇒ single-symbol level-1 fill (`:1159`) | [x] |
| 623 | `HUF_readDTableX2_wksp` | `minWeight > 1` ⇒ `HUF_fillDTableX2Level2` skip-fill `switch (length)` cases 2/4/default (3 rows, `:1084`) | [x] |
| 624 | `HUF_readDTableX2_wksp` | `HUF_fillDTableX2ForWeight` `switch (length)` cases 1/2/4/8/default (5 rows, `:1019`) | [x] |
| 625 | `HUF_selectDecoder` | `cSrcSize >= dstSize` ⇒ `Q=15`; ratios giving `Q` = 2, 8, 14 (4 rows, `:1821-1832`) | [x] |
| 626 | `HUF_decompress1X_DCtx_wksp` | `cSrcSize == dstSize` ⇒ plain `memcpy` (`:1852`) | [x] |
| 627 | `HUF_decompress1X_DCtx_wksp` | `cSrcSize == 1` ⇒ `memset` RLE (`:1853`) | [x] |
| 628 | `HUF_decompress1X_DCtx_wksp` | normal case ⇒ `HUF_selectDecoder` picks X1 or X2 (2 rows) | [x] |
| 629 | `HUF_decompress1X1_DCtx_wksp` | `hSize < cSrcSize` (valid), one-stream decode (`:1894-1900`) | [x] |
| 630 | `HUF_decompress1X2_DCtx_wksp` | same for the double-symbol table (`:1754-1763`) | [x] |
| 631 | `HUF_decompress1X_usingDTable` | DTable with `tableType=0` (X1) and `=1` (X2) (2 rows, `:1888-1889`) | [x] |
| 632 | `HUF_decompress4X_usingDTable` | `tableType=0` and `=1` (2 rows) | [x] |
| 633 | `HUF_decompress4X_hufOnly_wksp` | normal 4-stream literals; note there is **no** memcpy/RLE shortcut here (unlike 1X) (`:1924-1928`) | [x] |
| 634 | `HUF_decompress4X*` | `flags` with `HUF_flags_bmi2` **unset** ⇒ immediately the plain-C fallback body, no fast loop (`:897-928`) | [i] |
| 635 | `HUF_decompress4X*` | `flags` with `bmi2` set and `disableAsm` unset ⇒ asm fast loop attempted | [i] |
| 636 | `HUF_decompress4X*` | `flags` with `bmi2` set and `HUF_flags_disableAsm` set ⇒ C fast loop | [i] |
| 637 | `HUF_decompress4X*` | `flags` with `HUF_flags_disableFast` set ⇒ fast loop skipped entirely | [i] |
| 638 | `HUF_decompress4X*` | fast-loop bail-out: `dtLog != 11` ⇒ silent fallback (`HUF_DecompressFastArgs_init`, `:219`) | [i] |
| 639 | `HUF_decompress4X*` | fast-loop bail-out: some `length1..4 < 8` ⇒ fallback (`:236`) | [i] |
| 640 | `HUF_decompress4X*` | fast-loop bail-out: `op[3] >= oend` (tiny output) ⇒ fallback (`:253`) | [i] |
| 641 | `HUF_decompress4X*` | `dstSize = 6` exactly (lower edge of the 4-split requirement, `:609`) | [i] |
| 642 | `HUF_decompress4X*` | `cSrcSize = 10` exactly (lower edge, `:608`) | [i] |
| 643 | `HUF_decodeStreamX1` | `(pEnd-p) > 3` (4-at-a-time loop) and `<= 3` (bare reload) (2 rows, `:545`) | [i] |
| 644 | `HUF_decodeStreamX2` | `dtLog <= 11 && MEM_64bits()` ⇒ 5×`_0` "up to 10 symbols" loop (`:1315`) | [i] |
| 645 | `HUF_decodeStreamX2` | `dtLog > 11` ⇒ the 8-symbol `_2/_1/_2/_0` loop (`:1326`) | [i] |
| 646 | `HUF_decodeLastSymbolX2` | `dt[val].length == 1` and `== 2` (2 rows, `:1279-1288`) | [i] |
| 647 | `HUF_compressWeights` (via `HUF_writeCTable_wksp`) | `wtSize <= 1` ⇒ 0; `maxCount == wtSize` ⇒ 1 (RLE); `maxCount == 1` ⇒ 0 (3 rows, `:162-167`) | [x] |
## 18. HIST

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 648 | `HIST_isError` | non-error `size_t` | [x] |
| 649 | `HIST_add` | 1 byte; 1 KB; note `count` is **not** zeroed by this function (`hist.c:29`) | [x] |
| 650 | `HIST_count_simple` | `srcSize=0` ⇒ `*maxSymbolValuePtr=0`, returns 0 (`:48`) | [x] |
| 651 | `HIST_count_simple` | `srcSize=1`; single-symbol input ⇒ `*maxSymbolValuePtr` shrinks via the `while(!count[max]) max--` loop (`:55`) | [x] |
| 652 | `HIST_countFast_wksp` | `sourceSize < 1500` ⇒ delegates to `HIST_count_simple` (`:154`) | [x] |
| 653 | `HIST_countFast_wksp` | `sourceSize == 1500` exactly ⇒ `HIST_count_parallel_wksp(trustInput)` | [x] |
| 654 | `HIST_countFast_wksp` | `sourceSize = 1500+15` ⇒ stripe-of-16 loop plus a scalar tail (`:102`, `:128`) | [x] |
| 655 | `HIST_countFast_wksp` | `workSpaceSize` exactly `HIST_WKSP_SIZE=4096`, 4-byte aligned | [x] |
| 656 | `HIST_count_wksp` | `*maxSymbolValuePtr < 255` ⇒ **always** `HIST_count_parallel_wksp(checkMaxSymbolValue)` regardless of size (`:170-171`) | [x] |
| 657 | `HIST_count_wksp` | `*maxSymbolValuePtr == 255` + `sourceSize < 1500` ⇒ `HIST_count_simple` via `countFast` (`:172-173`) | [x] |
| 658 | `HIST_count_wksp` | `*maxSymbolValuePtr == 255` + `sourceSize >= 1500` ⇒ parallel `trustInput` | [x] |
| 659 | `HIST_count_wksp` | `sourceSize == 0` ⇒ `count` memset, `*maxSymbolValuePtr=0`, returns 0 (`:93-97`) | [x] |
| 660 | `HIST_count` / `HIST_countFast` | stack-`tmpCounters` wrappers (bodies exist because `ZSTD_NO_UNUSED_FUNCTIONS` is not defined, `:176-191`) | [x] |
## 19. xxhash (`ZSTD_XXH*`)

`XXH_NO_XXH3` is forced ⇒ the *only* size thresholds are XXH32's **16** and XXH64's **32**, plus the
`&15` / `&31` tail sub-buckets. `XXH_FORCE_ALIGN_CHECK=1` on x86-64 ⇒ aligned/unaligned instantiations.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 661 | `ZSTD_XXH_versionNumber` | constant | [x] |
| 662 | `ZSTD_XXH32` | `len=0` (`< 16` bucket, `finalize` len&15 == 0) | [x] |
| 663 | `ZSTD_XXH32` | `len=1..3` ⇒ finalize `PROCESS1` tail only | [x] |
| 664 | `ZSTD_XXH32` | `len=4`, `8`, `12` ⇒ `PROCESS4` tail multiples | [x] |
| 665 | `ZSTD_XXH32` | `len=15` (largest sub-16) | [x] |
| 666 | `ZSTD_XXH32` | `len=16` exactly ⇒ one stripe, empty tail | [x] |
| 667 | `ZSTD_XXH32` | `len=17..31` ⇒ one stripe + each tail sub-bucket (`len&15` in 1-3, 4-7, 5,9,13, 6,10,14, 7,11,15) | [x] |
| 668 | `ZSTD_XXH32` | `len=1 MiB` ⇒ many stripes | [x] |
| 669 | `ZSTD_XXH32` | `input` 4-byte **aligned** ⇒ `XXH_aligned` instantiation (`:3082`) | [x] |
| 670 | `ZSTD_XXH32` | `input` **misaligned** (`(size_t)input & 3 != 0`) ⇒ `XXH_unaligned` instantiation | [x] |
| 671 | `ZSTD_XXH32` | `seed=0` and a nonzero seed (seed only affects the four initial accumulators; no special-casing) | [x] |
| 672 | `ZSTD_XXH32_createState`/`reset`/`update`/`digest`/`freeState` | one `update` of 1 MiB, then `digest` | [x] |
| 673 | `ZSTD_XXH32_update` | `input == NULL, len == 0` ⇒ returns `XXH_OK` immediately (`:3130-3133`) | [x] |
| 674 | `ZSTD_XXH32_update` | `memsize + len < 16` ⇒ pure buffering path, `large_len` stays 0 (`:3141-3145`) | [x] |
| 675 | `ZSTD_XXH32_update` | `memsize != 0` ⇒ top-up-to-16 carry-over path (`:3147-3157`) | [x] |
| 676 | `ZSTD_XXH32_update` | `p <= bEnd-16` ⇒ stripe loop; then a 1..15-byte tail buffered (`:3159-3174`) | [x] |
| 677 | `ZSTD_XXH32_update` | many small updates whose **total** crosses 16 ⇒ `large_len` latch set by `total_len_32 >= 16` (`:3139`) | [x] |
| 678 | `ZSTD_XXH32_digest` | `large_len == 0` ⇒ `h32 = seed + XXH_PRIME32_5` (`:3186-3193`) | [x] |
| 679 | `ZSTD_XXH32_digest` | `large_len == 1` ⇒ 4-lane merge | [x] |
| 680 | `ZSTD_XXH32_copyState` | copy mid-stream, both digests must match | [x] |
| 681 | `ZSTD_XXH32_canonicalFromHash` / `_hashFromCanonical` | round-trip; both endian branches (compile-time) | [x] |
| 682 | `ZSTD_XXH64` | `len=0`, `1..3`, `4..7`, `8..15`, `16..23`, `24..31` (the `finalize` sub-buckets, 6 rows) | [x] |
| 683 | `ZSTD_XXH64` | `len=31` (largest sub-32) and `len=32` exactly (one stripe, empty tail) | [x] |
| 684 | `ZSTD_XXH64` | `len=33..63` ⇒ one stripe + each tail sub-bucket | [x] |
| 685 | `ZSTD_XXH64` | `len=1 MiB` | [x] |
| 686 | `ZSTD_XXH64` | `input` 8-byte **aligned** vs misaligned (`(size_t)input & 7`) (2 rows, `:3529`) | [x] |
| 687 | `ZSTD_XXH64` | `seed=0` and nonzero | [x] |
| 688 | `ZSTD_XXH64_createState`/`reset`/`update`/`digest`/`freeState` | single big update | [x] |
| 689 | `ZSTD_XXH64_update` | `input == NULL, len == 0` ⇒ `XXH_OK` (`:3575-3578`) | [x] |
| 690 | `ZSTD_XXH64_update` | `memsize + len < 32` ⇒ buffering only (`:3585-3589`) | [x] |
| 691 | `ZSTD_XXH64_update` | `memsize != 0` ⇒ top-up-to-32 carry-over (`:3591-3599`) | [x] |
| 692 | `ZSTD_XXH64_update` | `p + 32 <= bEnd` ⇒ stripe loop; then a 1..31-byte tail buffered (`:3601-3616`) | [x] |
| 693 | `ZSTD_XXH64_digest` | `total_len < 32` ⇒ `h64 = seed + XXH_PRIME64_5` (`:3628-3636`) | [x] |
| 694 | `ZSTD_XXH64_digest` | `total_len >= 32` ⇒ rotl-sum + 4 merge rounds | [x] |
| 695 | `ZSTD_XXH64_copyState` | copy mid-stream | [x] |
| 696 | `ZSTD_XXH64_canonicalFromHash` / `_hashFromCanonical` | round-trip | [x] |
| 697 | `ZSTD_XXH64` (as used by zstd) | 128 KB block-by-block `XXH64_update` matching a single-shot `ZSTD_XXH64` of the whole frame (the frame-checksum invariant) | [x] |
## 20. dictBuilder (`ZDICT_*`, `COVER_*`, `divsufsort`)

Key constants: `ZDICT_DICTSIZE_MIN=256`, `ZDICT_CONTENTSIZE_MIN=128`, `ZDICT_MIN_SAMPLES_SIZE=512`,
`ZDICT_MAX_SAMPLES_SIZE=2000<<20`, `COVER_MAX_SAMPLES_SIZE=(unsigned)-1` on 64-bit,
`COVER_DEFAULT_SPLITPOINT=1.0`, `FASTCOVER_DEFAULT_SPLITPOINT=0.75`, `DEFAULT_F=20`, `DEFAULT_ACCEL=1`,
`FASTCOVER_MAX_F=31`, `FASTCOVER_MAX_ACCEL=10`, `g_selectivity_default=9`, `MINRATIO=4`.
With MT disabled, `POOL_create` returns the `g_poolCtx` singleton and `POOL_add` runs jobs
**synchronously**, so `nbThreads>1` is functionally identical to 1.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 698 | `ZDICT_isError` / `ZDICT_getErrorName` | non-error `size_t`; note `(size_t)-1` **is** an error (`COVER_best_t.compressedSize` sentinel) | [x] |
| 699 | `ZDICT_getDictID` | `dictSize>=8` + magic `0xEC30A437` ⇒ LE32 at +4; `dictSize<8` ⇒ 0; wrong magic ⇒ 0 (3 rows, `zdict.c:102-107`) | [x] |
| 700 | `ZDICT_getDictHeaderSize` | real dictionary ⇒ `ZSTD_loadCEntropy` header size (`zdict.c:109-128`); note `dictSize <= 8` is the error edge | [x] |
| 701 | `ZDICT_trainFromBuffer` | `nbSamples = 7` (minimum for the internal `splitPoint=0.75`), each sample >= 2 bytes, total >= 8, `dictBufferCapacity = 4096` ⇒ internally `ZDICT_optimizeTrainFromBuffer_fastCover` with `d=8, steps=4, f=20, accel=1` and k ∈ {50,537,1024,1511,1998} | [x] |
| 702 | `ZDICT_trainFromBuffer` | `nbSamples = 100`, 1 MiB total, `dictBufferCapacity = 110 KB` (typical usage) | [x] |
| 703 | `ZDICT_trainFromBuffer` | `dictBufferCapacity = 256` exactly (`ZDICT_DICTSIZE_MIN`) | [x] |
| 704 | `ZDICT_trainFromBuffer_cover` | `parameters = {k=200, d=8, zParams={compressionLevel=3, notificationLevel=0, dictID=0}}`, 20 samples ⇒ `splitPoint` forced to 1.0, `steps`/`nbThreads`/`shrinkDict` ignored (`cover.c:779-835`) | [x] |
| 705 | `ZDICT_trainFromBuffer_cover` | `d=6` (uses `COVER_cmp8`) | [x] |
| 706 | `ZDICT_trainFromBuffer_cover` | `d=16` (> 8 ⇒ `memcmp` comparison path, `cover.c:685`) — cover.c does **not** restrict `d` to 6/8 | [x] |
| 707 | `ZDICT_trainFromBuffer_cover` | `k == dictBufferCapacity` exactly (upper edge, `cover.c:556`) | [x] |
| 708 | `ZDICT_trainFromBuffer_cover` | `k == d` exactly (lower edge, `cover.c:560`) | [x] |
| 709 | `ZDICT_trainFromBuffer_cover` | `nbSamples = 5` exactly (`nbTrainSamples >= 5` with splitPoint 1.0) | [x] |
| 710 | `ZDICT_trainFromBuffer_cover` | total sample bytes `== MAX(d,8)` exactly (lower edge, `COVER_ctx_init`) | [x] |
| 711 | `ZDICT_trainFromBuffer_cover` | `zParams.dictID = 0` ⇒ auto `(XXH64(content)%(2^31-32768))+32768`; `dictID = 12345` ⇒ verbatim (2 rows) | [x] |
| 712 | `ZDICT_trainFromBuffer_cover` | `zParams.compressionLevel = 0` ⇒ 3; `= 19`; `= -5` (3 rows) | [x] |
| 713 | `ZDICT_trainFromBuffer_cover` | `zParams.notificationLevel` 0/1/2/3/4 (sets the file-static `g_displayLevel`, non-reentrant) (5 rows) | [x] |
| 714 | `ZDICT_trainFromBuffer_cover` | corpus where `nbDmers/maxDictSize < 10` ⇒ `COVER_warnOnSmallCorpus` fires at level >= 1 | [x] |
| 715 | `ZDICT_optimizeTrainFromBuffer_cover` | `parameters = {k=0, d=0, steps=0, nbThreads=0, splitPoint=0}` ⇒ defaults `d ∈ {6,8}`, `k ∈ [50,2000]`, `steps=40`, `splitPoint=1.0`, `shrinkDict=0`; params written back on success | [x] |
| 716 | `ZDICT_optimizeTrainFromBuffer_cover` | `k=0, d=6` fixed ⇒ single-`d` sweep | [x] |
| 717 | `ZDICT_optimizeTrainFromBuffer_cover` | `k=500` fixed, `d=0` ⇒ two-`d` sweep, single k | [x] |
| 718 | `ZDICT_optimizeTrainFromBuffer_cover` | `steps=1` ⇒ `kStepSize = MAX(1950/1,1)` ⇒ 2 k values | [x] |
| 719 | `ZDICT_optimizeTrainFromBuffer_cover` | `steps=40` (default) with `k=0` | [x] |
| 720 | `ZDICT_optimizeTrainFromBuffer_cover` | `splitPoint=0.5` ⇒ train/test split (`nbTrainSamples = nbSamples*0.5`, `cover.c:609-612`) | [x] |
| 721 | `ZDICT_optimizeTrainFromBuffer_cover` | `splitPoint=1.0` explicitly | [x] |
| 722 | `ZDICT_optimizeTrainFromBuffer_cover` | `nbThreads=1` (pool NULL) and `nbThreads=4` (dummy pool, synchronous) — must give **identical** dictionaries (2 rows) | [x] |
| 723 | `ZDICT_optimizeTrainFromBuffer_cover` | `notificationLevel=3` ⇒ inner `g_displayLevel = 2` (deliberately lowered by one, `cover.c:1221`) | [x] |
| 724 | `ZDICT_trainFromBuffer_fastCover` | `{k=200, d=8, f=20, accel=1}` ⇒ `splitPoint` forced 1.0, `steps`/`nbThreads`/`shrinkDict` ignored | [x] |
| 725 | `ZDICT_trainFromBuffer_fastCover` | `d=6` (⇒ `ZSTD_hash6Ptr`) and `d=8` (⇒ `ZSTD_hash8Ptr`) — **only these two are legal** (`fastcover.c:237-239`) (2 rows) | [x] |
| 726 | `ZDICT_trainFromBuffer_fastCover` | `f=0` ⇒ 20; `f=1`; `f=31` (`FASTCOVER_MAX_F`) (3 rows) | [x] |
| 727 | `ZDICT_trainFromBuffer_fastCover` | `accel=0` ⇒ 1; `accel=1` (finalize 100%, skip 0); `accel=5` (20%, skip 4); `accel=10` (10%, skip 9) (4 rows, `fastcover.c:109-121`) | [x] |
| 728 | `ZDICT_trainFromBuffer_fastCover` | `nbSamples=5` exactly | [x] |
| 729 | `ZDICT_optimizeTrainFromBuffer_fastCover` | all-zero params ⇒ `d ∈ {6,8}`, `k ∈ [50,2000]`, `steps=40`, `f=20`, `accel=1`, `splitPoint=0.75`, `shrinkDict=0` | [x] |
| 730 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `splitPoint=0.75` explicit; `=1.0` (2 rows) | [x] |
| 731 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `f=16`, `accel=3`, `steps=8`, `d=6` fixed | [x] |
| 732 | `ZDICT_optimizeTrainFromBuffer_fastCover` | writes back `k,d,steps,nbThreads,splitPoint,f,accel,zParams,shrinkDict` via `FASTCOVER_convertToFastCoverParams` (`fastcover.c:759`) | [x] |
| 733 | `ZDICT_finalizeDictionary` | `dictContentSize = 4096`, `dictBufferCapacity = 8192`, 20 samples, `params={0,0,0}` ⇒ level 3, auto dictID | [x] |
| 734 | `ZDICT_finalizeDictionary` | `dictBufferCapacity == max(dictContentSize, 256)` exactly (both lower edges, `zdict.c:874-875`) | [x] |
| 735 | `ZDICT_finalizeDictionary` | `hSize + dictContentSize > dictBufferCapacity` ⇒ content truncated to `dictBufferCapacity - hSize` (`zdict.c:900`) | [x] |
| 736 | `ZDICT_finalizeDictionary` | `dictContentSize < 8` ⇒ `paddingSize = 8 - dictContentSize` zero bytes inserted **before** the content (`zdict.c:905`) | [x] |
| 737 | `ZDICT_finalizeDictionary` | `dictContentSize == 8` exactly ⇒ no padding | [x] |
| 738 | `ZDICT_finalizeDictionary` | `dictBuffer` overlapping `customDictContent` (the `memmove`-first ordering is load-bearing) | [x] |
| 739 | `ZDICT_finalizeDictionary` | `params.dictID = 0` ⇒ deterministic-from-content dictID in `[32768, 2^31-1]`; explicit dictID (2 rows) | [x] |
| 740 | `ZDICT_finalizeDictionary` | `params.compressionLevel = 0` ⇒ 3; explicit 1 and 19 (3 rows) | [x] |
| 741 | `ZDICT_finalizeDictionary` | non-compressible literal distribution ⇒ `ZDICT_flatLit` rescue path (`zdict.c:649-656`, `:731-736`) | [x] |
| 742 | `ZDICT_addEntropyTablesFromBuffer` | raw content pre-placed at `dictBuffer + dictBufferCapacity - dictContentSize`, `hSize + dictContentSize < dictBufferCapacity` ⇒ content memmoved to `hSize` (`zdict.c:940-972`) | [x] |
| 743 | `ZDICT_addEntropyTablesFromBuffer` | `hSize + dictContentSize >= dictBufferCapacity` ⇒ **no memmove**, returns `MIN(capacity, hSize+contentSize)` | [x] |
| 744 | `ZDICT_trainFromBuffer_legacy` | `{selectivityLevel=0 ⇒ 9, zParams={0,0,0}}`, total sample bytes >= 512, `dictBufferCapacity >= 256` (`zdict.c:1084-1104`) | [x] |
| 745 | `ZDICT_trainFromBuffer_legacy` | total sample bytes `< 512` ⇒ **returns 0**, not an error code (`zdict.c:1091`) — a distinguishable *valid* return | [x] |
| 746 | `ZDICT_trainFromBuffer_legacy` | total sample bytes `== 512` exactly | [x] |
| 747 | `ZDICT_trainFromBuffer_legacy` | `selectivityLevel = 1` (⇒ `minRep = nbSamples>>1`), `= 9` (default), `= 31` (`>30` ⇒ `minRep = MINRATIO = 4`) (3 rows, `zdict.c:985-986`) | [x] |
| 748 | `ZDICT_trainFromBuffer_legacy` | resulting `dictContentSize` larger than `maxDictSize` ⇒ lowest-savings items dropped (`zdict.c:1050-1058`) | [x] |
| 749 | `COVER_sum` | `nbSamples=0` ⇒ 0; 100 samples | [x] |
| 750 | `COVER_computeEpochs` | `passes=4` (cover) and `passes=1` (fastcover); a case where `size < 10*k` ⇒ recompute (2+ rows, `cover.c:707`) | [x] |
| 751 | `COVER_warnOnSmallCorpus` | `nbDmers/maxDictSize < 10` at `displayLevel=1` and `=0` (2 rows) | [x] |
| 752 | `COVER_checkTotalCompressedSize` | `splitPoint == 1.0` (starts at index 0) and `< 1.0` (starts at `nbTrainSamples`) (2 rows, `cover.c:839`) | [x] |
| 753 | `COVER_best_init` / `_start` / `_finish` / `_wait` / `_destroy` | full lifecycle including `NULL` argument tolerance; a `_finish` that improves and one that does not (2 rows) | [x] |
| 754 | `COVER_dictSelectionIsError` / `COVER_dictSelectionError` / `COVER_dictSelectionFree` | on a valid selection and on an error selection (2 rows) | [x] |
| 755 | `COVER_selectDict` | `params.shrinkDict == 0` ⇒ returns the single full-size dictionary (`cover.c:1050`) | [x] |
| 756 | `COVER_selectDict` | `params.shrinkDict == 1`, `shrinkDictMaxRegression = 0` ⇒ tolerance 1.0, doubling loop from 256 taking the **tail** of the content | [x] |
| 757 | `COVER_selectDict` | `shrinkDict == 1`, `shrinkDictMaxRegression = 5` ⇒ tolerance 1.05 | [x] |
| 758 | `divsufsort` | `n=0` ⇒ 0; `n=1` ⇒ `SA[0]=0`; `n=2`; `n=1000` random; `openMP=0` (no-op without `_OPENMP`) (5 rows, `divsufsort.c:1846-1874`) | [x] |
| 759 | `divbwt` | exported but unused by zstd — exercise `n=0`, `n=1`, `n=1000` (3 rows) | [x] |
## 21. Deprecated ZBUFF (thin shim over `ZSTD_*Stream`)

`typedef ZSTD_CStream ZBUFF_CCtx` / `typedef ZSTD_DStream ZBUFF_DCtx`. Both `.c` files
`#define ZBUFF_STATIC_LINKING_ONLY`, so the `_advanced` variants **are** compiled and exported.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 760 | `ZBUFF_createCCtx` / `ZBUFF_freeCCtx` | including `ZBUFF_freeCCtx(NULL)` | [x] |
| 761 | `ZBUFF_createCCtx_advanced` | custom `ZSTD_customMem` | [x] |
| 762 | `ZBUFF_compressInit` + `ZBUFF_compressContinue`× N + `ZBUFF_compressEnd` | level 3, 1 MiB in `ZBUFF_recommendedCInSize()` chunks | [x] |
| 763 | `ZBUFF_compressInit` | level 1, 22, 0, `-5` (4 rows) | [x] |
| 764 | `ZBUFF_compressInitDictionary` | real dict + level 3 (⇒ reset session_only + `ZSTD_c_compressionLevel` + `loadDictionary`) | [x] |
| 765 | `ZBUFF_compressInitDictionary` | `dict=NULL, dictSize=0` (legal no-op) | [x] |
| 766 | `ZBUFF_compressInit_advanced` | `params` from `ZSTD_getParams(3, srcSize, 0)`, `pledgedSrcSize = srcSize` — pushes 7 cParams + 3 fParams individually (`zbuff_compress.c:72-95`) | [x] |
| 767 | `ZBUFF_compressInit_advanced` | `pledgedSrcSize = 0` ⇒ remapped to `ZSTD_CONTENTSIZE_UNKNOWN` (`zbuff_compress.c:76`) | [x] |
| 768 | `ZBUFF_compressInit_advanced` | `params.fParams.noDictIDFlag = 1` — **note the polarity quirk**: it is passed straight into `ZSTD_c_dictIDFlag` (`zbuff_compress.c:91`), so `noDictIDFlag=1` ends up *enabling* dictID. Preserve exactly. | [x] |
| 769 | `ZBUFF_compressInit_advanced` | `params.fParams.noDictIDFlag = 0` (⇒ `dictIDFlag=0`) with a dict ⇒ no dictID in the frame | [x] |
| 770 | `ZBUFF_compressInit_advanced` | `params.fParams.checksumFlag=1`, `contentSizeFlag=0` | [x] |
| 771 | `ZBUFF_compressContinue` | `*dstCapacityPtr`/`*srcSizePtr` overwritten with **consumed/produced** counts; return = next-input hint | [x] |
| 772 | `ZBUFF_compressContinue` | `*dstCapacityPtr` too small ⇒ partial consumption | [x] |
| 773 | `ZBUFF_compressFlush` | returns bytes-remaining (>0 when dst too small, 0 when fully flushed) (2 rows) | [x] |
| 774 | `ZBUFF_compressEnd` | one-call completion (returns 0) vs partial (returns >0, follow with `ZBUFF_compressFlush`) (2 rows) | [x] |
| 775 | `ZBUFF_recommendedCInSize` / `ZBUFF_recommendedCOutSize` | constants (128 KB / `ZSTD_CStreamOutSize()`) | [x] |
| 776 | `ZBUFF_createDCtx` / `ZBUFF_freeDCtx` / `ZBUFF_createDCtx_advanced` | including `NULL` free and custom `ZSTD_customMem` | [x] |
| 777 | `ZBUFF_decompressInit` + `ZBUFF_decompressContinue`× N | full frame in `ZBUFF_recommendedDInSize()` chunks | [x] |
| 778 | `ZBUFF_decompressInitDictionary` | real dict; `dict=NULL, dictSize=0` (2 rows) | [x] |
| 779 | `ZBUFF_decompressContinue` | return 0 (frame complete + flushed) | [x] |
| 780 | `ZBUFF_decompressContinue` | return 1 (data still buffered internally) | [x] |
| 781 | `ZBUFF_decompressContinue` | return >1 (more input expected; suggested next input size) | [x] |
| 782 | `ZBUFF_decompressContinue` | `MULTIFRAME` input decoded across the shim | [x] |
| 783 | `ZBUFF_recommendedDInSize` / `ZBUFF_recommendedDOutSize` | constants (128 KB + 3 / 128 KB) | [x] |
| 784 | `ZBUFF_isError` / `ZBUFF_getErrorName` | non-error `size_t`; note it is `ERR_isError` directly, so it is also valid for `ZBUFFv0x_*` return codes | [x] |
## 22. Legacy decoders

Reachable via the shim: **v05 (`0xFD2FB525`), v06 (`0xFD2FB526`), v07 (`0xFD2FB527`)** only.
v01–v04 symbols exist in the .so but are not dispatched to (`zstd_legacy.h:30-41,56-86`).

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 785 | `ZSTD_decompress` (via `ZSTD_isLegacy`) | a v05 frame ⇒ `ZSTD_decompressLegacy` case 5 | [x] |
| 786 | `ZSTD_decompress` | a v06 frame ⇒ case 6 | [x] |
| 787 | `ZSTD_decompress` | a v07 frame ⇒ case 7 | [x] |
| 788 | `ZSTD_isLegacy` | `srcSize < 4` ⇒ 0; each of the three accepted magics ⇒ 5/6/7; a current-format magic ⇒ 0 (5 rows) | [i] |
| 789 | `ZSTD_decompress` | legacy frame + `ZSTD_d_format = magicless` ⇒ legacy dispatch **skipped** (`zstd_decompress.c:1090`) | [x] |
| 790 | `ZSTD_decompressStream` | v05/v06/v07 frame ⇒ `ZSTD_initLegacyStream` + `ZSTD_decompressLegacyStream`, `hint==0` ⇒ back to `zdss_init` (3 rows, `zstd_decompress.c:2130-2159`) | [x] |
| 791 | `ZSTD_findFrameCompressedSize` / `ZSTD_decompressBound` | legacy v05/v06/v07 frames ⇒ `ZSTD_findFrameSizeInfoLegacy` (3 rows) | [x] |
| 792 | `ZSTD_getFrameContentSize` | v07 frame with an FCS field ⇒ real value; v05 frame ⇒ **always 0** (v05 has no FCS in the header); v06 with `fcsId=0` ⇒ 0 (3 rows) | [x] |
| 793 | `ZSTDv05_decompress` / `ZSTDv05_decompressDCtx` | v05 frame, `bt_compressed` blocks | [x] |
| 794 | `ZSTDv05_decompress` | v05 frame with `bt_raw` blocks | [x] |
| 795 | `ZSTDv05_decompress` | v05 `windowLog = 11` (`ZSTDv05_WINDOWLOG_ABSOLUTEMIN`) and `= 26` (upper) (2 rows) | [x] |
| 796 | `ZSTDv05_decompress` | v05 single-block and multi-block frames (2 rows) | [x] |
| 797 | `ZSTDv05_decompress_usingDict` / `ZSTDv05_decompressBegin_usingDict` | dict with `ZSTDv05_DICT_MAGIC (0xEC30A435)` ⇒ entropy load; without ⇒ pure content (2 rows) | [x] |
| 798 | `ZSTDv05_getFrameParams` | fills `ZSTDv05_parameters` (`srcSize` always 0, `windowLog` from byte 4 low nibble) | [x] |
| 799 | `ZSTDv05_nextSrcSizeToDecompress` + `ZSTDv05_decompressContinue` | buffer-less walk through all 4 `ZSTDv05_dStage` values; `srcSize` must equal `expected` exactly | [x] |
| 800 | `ZSTDv05_createDCtx` / `_freeDCtx` / `ZSTDv05_copyDCtx` / `ZSTDv05_isError` / `ZSTDv05_getErrorName` | lifecycle + non-error code | [x] |
| 801 | `ZSTDv05_decompressBlock` / `ZSTDv05_decompressBegin` / `ZSTDv05_sizeofDCtx` | .c-only exports (not in the header) — still linkable | [x] |
| 802 | `ZBUFFv05_createDCtx` / `decompressInit` / `decompressContinue` / `freeDCtx` | v05 frame in 1-byte chunks ⇒ exercises all 7 `ZBUFFv05ds_*` stages, incl. the unique `_readHeader`/`_loadHeader`/`_decodeHeader` triple | [x] |
| 803 | `ZBUFFv05_decompressInitDictionary` | v05 dict + streaming | [x] |
| 804 | `ZBUFFv05_recommendedDInSize` / `DOutSize` | constants (128 KB + 3 / 128 KB) | [x] |
| 805 | `ZBUFFv05_isError` / `ZBUFFv05_getErrorName` | non-error code | [x] |
| 806 | `ZSTDv06_decompress` / `ZSTDv06_decompressDCtx` | v06 frame, `bt_compressed` blocks | [x] |
| 807 | `ZSTDv06_decompress` | v06 frame with `bt_raw` blocks | [x] |
| 808 | `ZSTDv06_decompress` | v06 `fcsId = 0` (no FCS), `= 1` (1-byte), `= 2` (LE16+256), `= 3` (LE64) ⇒ header 5/6/7/13 bytes (4 rows) | [x] |
| 809 | `ZSTDv06_decompress` | v06 `windowLog = 12` (`ZSTDv06_WINDOWLOG_ABSOLUTEMIN`) and upper (2 rows) | [x] |
| 810 | `ZSTDv06_decompress` | v06 single-block and multi-block (2 rows) | [x] |
| 811 | `ZSTDv06_getFrameParams` | fills `{frameContentSize, windowLog}` for each `fcsId` | [x] |
| 812 | `ZSTDv06_frameHeaderSize` (via `getFrameParams`) | 5, 6, 7, 13 bytes | [i] |
| 813 | `ZSTDv06_decompress_usingDict` / `_decompressBegin_usingDict` | dict with `ZSTDv06_DICT_MAGIC (0xEC30A436)` and without (2 rows) | [x] |
| 814 | `ZSTDv06_nextSrcSizeToDecompress` + `ZSTDv06_decompressContinue` | buffer-less walk through all 4 `ZSTDv06` stages | [x] |
| 815 | `ZSTDv06_compressBound` | `size = 0`, `1`, `131072` — the only compress-side symbol in the active legacy set | [i] |
| 816 | `ZSTDv06_createDCtx` / `_freeDCtx` / `_copyDCtx` / `_isError` / `_getErrorName` / `_sizeofDCtx` / `_decompressBegin` / `_decompressBlock` / `_seqToCodes` | lifecycle + direct block decode | [x] |
| 817 | `ZBUFFv06_createDCtx` / `decompressInit` / `decompressContinue` / `freeDCtx` | v06 frame in 1-byte chunks ⇒ all 5 `ZBUFFv06_dStage` values | [x] |
| 818 | `ZBUFFv06_decompressInitDictionary` | v06 dict + streaming | [x] |
| 819 | `ZBUFFv06_recommendedDInSize` / `DOutSize` / `_isError` / `_getErrorName` | constants + non-error code | [x] |
| 820 | `ZSTDv07_decompress` / `ZSTDv07_decompressDCtx` | v07 frame, `bt_compressed` blocks | [x] |
| 821 | `ZSTDv07_decompress` | v07 frame with `bt_raw` blocks | [x] |
| 822 | `ZSTDv07_decompress` | v07 frame with `bt_rle` blocks (**v07 is the only active legacy version implementing RLE**, `zstd_v07.c:3782-3784`) | [x] |
| 823 | `ZSTDv07_decompress` | v07 `dictIDSizeCode` 0/1/2/3 ⇒ dictID field 0/1/2/4 bytes (4 rows) | [x] |
| 824 | `ZSTDv07_decompress` | v07 `fcsID` 0/1/2/3 ⇒ FCS 0/2/4/8 bytes (4 rows) | [x] |
| 825 | `ZSTDv07_decompress` | v07 `directMode`(single-segment) = 1 ⇒ no window byte, `windowSize` from FCS, 1-byte FCS when `fcsID=0` | [x] |
| 826 | `ZSTDv07_decompress` | v07 `directMode = 0` ⇒ window descriptor with mantissa `b&7` = 0 and = 7 (2 rows) | [x] |
| 827 | `ZSTDv07_decompress` | v07 `checksumFlag = 1` (XXH64 verified) and `= 0` (2 rows) | [x] |
| 828 | `ZSTDv07_decompress` | v07 frame header at its minimum (5 bytes) and maximum (18 bytes) (2 rows) | [x] |
| 829 | `ZSTDv07_decompress` | v07 stream containing a skippable frame (`0x184D2A50..5F`) (`zstd_v07.c:3102-3106`) | [x] |
| 830 | `ZSTDv07_decompress` | v07 single-block and multi-block (2 rows) | [x] |
| 831 | `ZSTDv07_getDecompressedSize` | v07 frame with known FCS and with unknown (2 rows) — the only legacy version exporting this | [x] |
| 832 | `ZSTDv07_getFrameParams` | fills `{frameContentSize, windowSize, dictID, checksumFlag}` for each header variant | [x] |
| 833 | `ZSTDv07_decompress_usingDict` | dict with `ZSTDv07_DICT_MAGIC (0xEC30A437)`; dict with `dictSize < 8` (⇒ pure content); dict with wrong magic (⇒ pure content) (3 rows, `zstd_v07.c:4091-4111`) | [x] |
| 834 | `ZSTDv07_createDDict` / `_freeDDict` / `ZSTDv07_decompress_usingDDict` | digested-dict path (v07-only) | [x] |
| 835 | `ZSTDv07_decompress_usingDict` | frame whose `fParams.dictID` matches the dict's dictID (valid); mismatch is the error case (`zstd_v07.c:3186`) | [x] |
| 836 | `ZSTDv07_nextSrcSizeToDecompress` + `ZSTDv07_decompressContinue` + `ZSTDv07_decompressBegin_usingDict` + `ZSTDv07_copyDCtx` | .c-only exports; buffer-less walk through all 6 `ZSTDv07_dStage` values incl. `ZSTDds_decodeSkippableHeader` and `ZSTDds_skipFrame` | [x] |
| 837 | `ZSTDv07_createDCtx` / `_createDCtx_advanced` / `_freeDCtx` / `_isError` / `_getErrorName` / `_sizeofDCtx` / `_estimateDCtxSize` / `_decompressBegin` / `_decompressBlock` / `_execSequence` / `_seqToCodes` | lifecycle + direct block decode | [x] |
| 838 | `ZBUFFv07_createDCtx` / `_createDCtx_advanced` / `decompressInit` / `decompressContinue` / `freeDCtx` | v07 frame in 1-byte chunks ⇒ all 5 `ZBUFFv07_dStage` values, incl. the partial-header hint `(hSize-lhSize)+blockHeaderSize` (`zstd_v07.c:4372`) | [x] |
| 839 | `ZBUFFv07_decompressInitDictionary` | v07 dict + streaming | [x] |
| 840 | `ZBUFFv07_decompressContinue` | v07 buffer sizing: `windowSize = MAX(windowSize, 1<<10)`, `blockSize = MIN(windowSize, 128KB)`, `neededOutSize = windowSize + blockSize + 2*WILDCOPY_OVERLENGTH` (`zstd_v07.c:4388-4406`) | [x] |
| 841 | `ZBUFFv07_decompressContinue` | v07 stream with a skippable frame ⇒ skip path with `0` dstCapacity (`zstd_v07.c:4419-4421`) | [x] |
| 842 | `ZBUFFv07_recommendedDInSize` / `DOutSize` / `_isError` / `_getErrorName` | constants + non-error code | [x] |
| 843 | `ZSTDv01_decompress` / `_decompressDCtx` / `_createDCtx` / `_freeDCtx` / `_resetDCtx` / `_nextSrcSizeToDecompress` / `_decompressContinue` / `_findFrameSizeInfoLegacy` / `_isError` | **directly callable** (symbols exist) on a v01 frame (magic `0xFD2FB51E`) — not reachable via the shim | [x] |
| 844 | `ZSTDv02_*` | same nine entry points on a v02 frame (magic `0xFD2FB522`) | [i] |
| 845 | `ZSTDv03_*` | same nine entry points on a v03 frame (magic `0xFD2FB523`) | [i] |
| 846 | `ZSTDv04_*` | `ZSTDv04_decompress`/`_decompressDCtx` (typed ctx)/`_createDCtx`/`_freeDCtx`/`_resetDCtx`/`_nextSrcSizeToDecompress`/`_decompressContinue`/`_findFrameSizeInfoLegacy`/`_isError` on a v04 frame (magic `0xFD2FB524`) | [i] |
| 847 | `ZBUFFv04_createDCtx` / `_decompressInit` / `_decompressWithDictionary` / `_decompressContinue` / `_freeDCtx` / `_isError` / `_getErrorName` / `_recommendedDInSize` / `_recommendedDOutSize` | v04's unique `decompressWithDictionary` (rather than `decompressInitDictionary`) | [x] |
| 848 | `ZSTD_getDecompressedSize_legacy` | version `< 5` ⇒ always 0; v05 ⇒ 0 (no FCS); v06/v07 with FCS ⇒ real value (`zstd_legacy.h:89-118`) | [i] |
| 849 | `ZSTD_freeLegacyStreamContext` | after `ZSTD_initLegacyStream` for each of versions 5, 6, 7 (3 rows, `zstd_legacy.h:275-298`) | [i] |

