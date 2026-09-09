# CONFIGS.md — Configuration-surface table

Derived mechanically from `c_src/src/{lz4,lz4hc,lz4frame,lz4file,xxhash}.c` and
`c_src/include/*.h` by enumerating every runtime option, every input shape the
code special-cases, and every public entry point (all 143 exported symbols),
then taking the pruned cross-product of the combinations the C actually
distinguishes.

**Build configuration:** `translation/Cargo.toml` declares **no `[features]`**
and `translation/src/**` contains **no `cfg(feature = ...)`**, so there is
exactly ONE feature combination (the default).  Neither `c_src/CMakeLists.txt`
nor `Cargo.toml` builds an executable, so there is no binary-stdout comparison
to make.  The C library is compiled with `XXH_NAMESPACE=LZ4_`,
`LZ4_HEAPMODE=0`, `LZ4F_HEAPMODE=0`.

Legend for the checkbox column: `[x]` = a differential test exercises this row
against both `.so` files with randomized inputs and passes.

## Key thresholds the C branches on (confirmed in source)

`MINMATCH=4`, `WILDCOPYLENGTH=8`, `LASTLITERALS=5`, `MFLIMIT=12`,
`MATCH_SAFEGUARD_DISTANCE=12`, `FASTLOOP_SAFE_DISTANCE=64`, `LZ4_minLength=13`,
`LZ4_64Klimit=65547`, `LZ4_skipTrigger=6`, `LZ4_DISTANCE_MAX=65535`,
`ML_MASK=RUN_MASK=15`, `LZ4_MAX_INPUT_SIZE=0x7E000000`,
`LZ4_ACCELERATION_MAX=65537`, `HASH_UNIT=sizeof(reg_t)=8`, `OPTIMAL_ML=18`,
`LZ4_OPT_NUM=4096`, `TRAILING_LITERALS=3`, `LZ4HC_HASHSIZE=4`,
`LZ4MID_HASHSIZE=8`, frame `minFHSize=7` / `maxFHSize=19` / `BHSize=4` /
`BFSize=4`, `LZ4F_BLOCKUNCOMPRESSED_FLAG=0x80000000`.

### compressionLevel thresholds (`LZ4HC_getCLevelParams`)

`if (cLevel < 1) cLevel = LZ4HC_CLEVEL_DEFAULT(9)`, then `cLevel = MIN(12, cLevel)`.

| level | strategy | internal compressor | nbSearches | targetLength | notes |
|---|---|---|---|---|---|
| <1 (incl. 0, negative) | — | remapped to level 9 | 256 | 16 | |
| 1 | `lz4mid` | `LZ4MID_compress` | 2 | 16 | |
| 2 (`LZ4HC_CLEVEL_MIN`) | `lz4mid` | `LZ4MID_compress` | 2 | 16 | |
| 3 | `lz4hc` | `LZ4HC_compress_hashChain` | 4 | 16 | patternAnalysis=0 |
| 4 | `lz4hc` | hashChain | 8 | 16 | patternAnalysis=0 |
| 5 | `lz4hc` | hashChain | 16 | 16 | patternAnalysis=0 |
| 6 | `lz4hc` | hashChain | 32 | 16 | patternAnalysis=0 |
| 7 | `lz4hc` | hashChain | 64 | 16 | patternAnalysis=0 |
| 8 | `lz4hc` | hashChain | 128 | 16 | patternAnalysis=**0** |
| 9 (`DEFAULT`) | `lz4hc` | hashChain | 256 | 16 | patternAnalysis=**1** |
| 10 (`OPT_MIN`) | `lz4opt` | `LZ4HC_compress_optimal` | 96 | 64 | fullUpdate=0 |
| 11 | `lz4opt` | optimal | 512 | 128 | fullUpdate=0 |
| 12 (`MAX`) | `lz4opt` | optimal | 16384 | 4096→4095 | fullUpdate=**1** |
| >12 | — | clamped to 12 | | | |

`favorDecSpeed` is a **no-op below level 10** (only reaches
`LZ4HC_compress_optimal` / `LZ4HC_FindLongerMatch`).

`lz4frame.c` splits at `LZ4HC_CLEVEL_MIN == 2`: `level < 2` → `ctxTypeID 1`
(`LZ4_stream_t`, `acceleration = (level < 0) ? -level + 1 : 1`);
`level >= 2` → `ctxTypeID 2` (`LZ4_streamHC_t`, level forwarded to lz4hc).

### tableType selection (`LZ4_compress_generic` callers)

| caller | tableType | outputDirective | dictDirective |
|---|---|---|---|
| `LZ4_compress_fast_extState` (and `_fast`, `_default`, `LZ4_compress`, `_limitedOutput`, `*_withState`) | `inputSize < 65547` → **byU16**; else `(sizeof(void*)==4 && (uptrval)src > 65535) ? byPtr : byU32` | `maxOutputSize >= LZ4_compressBound(inputSize)` → `notLimited`, else `limitedOutput` | `noDict`, `noDictIssue` |
| `LZ4_compress_fast_extState_fastReset` | same, after `LZ4_prepareTable` | same | `noDict`; `dictSmall` when byU16 **and** `currentOffset != 0` |
| `LZ4_compress_destSize_extState_internal` | `targetDstSize >= LZ4_compressBound(*srcSizePtr)` → delegate; else `*srcSizePtr < 65547` → byU16 else byPtr/byU32 | `fillOutput` | `noDict`, `noDictIssue` |
| `LZ4_compress_fast_continue` | **always byU32** | **always limitedOutput** | prefix → `withPrefix64k` + (`dictSize<64K && dictSize<currentOffset` ? `dictSmall` : `noDictIssue`); dictCtx & `inputSize>4K` → `usingExtDict`+`noDictIssue`; dictCtx & `<=4K` → `usingDictCtx`; else `usingExtDict` + dictSmall/noDictIssue |
| `LZ4_compress_forceExtDict` | byU32 | `notLimited` | `usingExtDict` + dictSmall/noDictIssue; acceleration hard-wired 1 |
| `LZ4_loadDict_internal` | byU32 | n/a | n/a |
| `LZ4_resetStream_fast` | `LZ4_prepareTable(0, byU32)` | n/a | n/a |

`LZ4_prepareTable` clears the table when `tableType != clearedTable` **and** any
of: tableType changed; `byU16 && currentOffset+inputSize >= 0xFFFF`;
`byU32 && currentOffset > 1 GB`; `byPtr`; `inputSize >= 4 KB`.  Then, if
`currentOffset != 0 && tableType == byU32`, `currentOffset += 64 KB`.

## Module `lz4.c` — one-shot block compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `LZ4_versionNumber`, `LZ4_versionString`, `LZ4_sizeofState`, `LZ4_sizeofStreamState`, `LZ4_compressBound` | no options; `compressBound(0)=16`, `(65536)=65809`, `(0x7E000000)` ok, `(0x7E000001)` -> 0, negative -> 0 | [x] |
| 2 | `LZ4_initStream` | buffer=NULL -> NULL; size = `sizeof(LZ4_stream_t)-1` -> NULL; misaligned buffer -> NULL; valid -> zeroed internal | [x] |
| 3 | `LZ4_compress_default`, `LZ4_compress_fast` | srcSize=0 (src may be NULL), dstCapacity>=1 -> single `0x00` token, returns 1; dstCapacity=0 -> 0 | [x] |
| 4 | `LZ4_compress_default` | srcSize=1 and 12 (both `< LZ4_minLength = 13`) -> jump to `_last_literals`, pure literal block; srcSize=13 -> main loop, byU16 | [x] |
| 5 | `LZ4_compress_fast_extState` | srcSize=65546 (`LZ4_64Klimit-1`) + dstCapacity>=bound -> **byU16 + notLimited** | [x] |
| 6 | `LZ4_compress_fast_extState` | srcSize=65547 (`== LZ4_64Klimit`) + dstCapacity>=bound -> **byU32 + notLimited** | [x] |
| 7 | `LZ4_compress_fast_extState` | dstCapacity < `LZ4_compressBound(srcSize)` -> **limitedOutput**; 100 B incompressible into 20 B -> 0 mid-block | [x] |
| 8 | `LZ4_compress_fast` | acceleration=0 / -5 / INT_MIN -> clamped to 1; 65538 / INT_MAX -> clamped to `LZ4_ACCELERATION_MAX=65537`; 8 -> `searchMatchNb = accel << LZ4_skipTrigger(6)` | [x] |
| 9 | `LZ4_compress_default` | 1 MB of 0x00 -> `matchCode >= ML_MASK(15)` with 0xFF runs and `litLength >= RUN_MASK(15)`; srcSize=0x7E000001 or negative -> 0 | [x] |
| 10 | `LZ4_compress_fast_extState_fastReset` | freshly `LZ4_initStream`'d state, srcSize=1000 (<65547), currentOffset==0 -> **byU16 + noDictIssue** | [x] |
| 11 | `LZ4_compress_fast_extState_fastReset` | reused byU16 state, currentOffset!=0, `currentOffset+srcSize < 0xFFFF` -> table reused, **byU16 + dictSmall** | [x] |
| 12 | `LZ4_compress_fast_extState_fastReset` | forced-reset triggers: srcSize>=4 KB; tableType changed; byU16 with `currentOffset+srcSize >= 0xFFFF`; byU32 with `currentOffset > 1 GB`; byPtr | [x] |
| 13 | `LZ4_compress_fast_extState_fastReset` | reused byU32 state, currentOffset>0, srcSize<4 KB -> no reset, `currentOffset += 64 KB` gap | [x] |
| 14 | `LZ4_compress_destSize` | `targetDstSize >= LZ4_compressBound(*srcSizePtr)` -> delegates to `LZ4_compress_fast_extState`, no fillOutput | [x] |
| 15 | `LZ4_compress_destSize`, `LZ4_compress_destSize_extState` | fillOutput: `*srcSizePtr=10000`/target=100 -> byU16; `*srcSizePtr=200000`/target=1000 -> byU32; target=0 -> 0. extState re-runs `LZ4_initStream` on exit | [x] |
| 16 | `LZ4_compress_destSize` | fillOutput edges: last literal run shrunk to exactly fill dst (minus 255-length tokens); over-long match reduced via `newMatchCode` + hash-clearing loop when `ip <= filledIp` | [x] |

## Module `lz4.c` — streaming compression & dictionaries

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 17 | `LZ4_createStream`, `LZ4_freeStream`, `LZ4_resetStream`, `LZ4_resetStream_fast` | full zeroing vs `LZ4_prepareTable(0, byU32)`; `LZ4_freeStream(NULL)` -> 0 | [x] |
| 18 | `LZ4_loadDict`, `LZ4_loadDict_internal` | dictSize=7 (`< HASH_UNIT = 8`) -> returns 0, dictSize=0, currentOffset already bumped to 64 KB | [x] |
| 19 | `LZ4_loadDict` | dictSize=8 and 65536 -> byU32 table filled every 3rd byte; dictSize=100000 -> truncated to last 64 KB, returns 65536 | [x] |
| 20 | `LZ4_loadDictSlow`, `LZ4_loadDict_internal` | `_ld_slow` mode: extra pass writing only slots whose stored index `<= currentOffset-64KB` (favours dictionary START) | [x] |
| 21 | `LZ4_attach_dictionary` | dictionaryStream dictSize>0 and working currentOffset==0 -> currentOffset bumped to 64 KB; dictionaryStream=NULL or `dictCtx->dictSize==0` -> dictCtx=NULL (silently not attached) | [x] |
| 22 | `LZ4_compress_fast_continue` | prefix mode (`src == dictionary+dictSize`), dictSize=1000 (<64 KB and < currentOffset) -> **byU32 + withPrefix64k + dictSmall** | [x] |
| 23 | `LZ4_compress_fast_continue` | prefix mode, dictSize=65536 -> **withPrefix64k + noDictIssue** | [x] |
| 24 | `LZ4_compress_fast_continue` | extDict (separate buffer), dictSize=1000 -> **usingExtDict + dictSmall** | [x] |
| 25 | `LZ4_compress_fast_continue` | extDict, dictSize=65536 -> **usingExtDict + noDictIssue** | [x] |
| 26 | `LZ4_compress_fast_continue`, `LZ4_attach_dictionary` | dictCtx attached, inputSize=4097 (>4 KB) -> dictCtx struct memcpy'd over the working stream, then **usingExtDict + noDictIssue** | [x] |
| 27 | `LZ4_compress_fast_continue`, `LZ4_attach_dictionary` | dictCtx attached, inputSize=4096 (<=4 KB) -> **usingDictCtx + noDictIssue** (dictDelta rebasing, second table lookup) | [x] |
| 28 | `LZ4_compress_fast_continue` | dictSize=3 (<4), `dictEnd != src`, inputSize>0, dictCtx==NULL -> dict discarded, falls back to prefix mode | [x] |
| 29 | `LZ4_compress_fast_continue` | ring buffer: `src+inputSize` lands inside `(dictionary, dictEnd)` -> dictSize recomputed, clamped to 64 KB, zeroed if <4 | [x] |
| 30 | `LZ4_compress_fast_continue` | `currentOffset+inputSize > 0x80000000` -> `LZ4_renormDictT` rescales the whole hash table, currentOffset=64 KB, dictSize clamped. This entry point is ALWAYS limitedOutput | [x] |
| 31 | `LZ4_saveDict`, `LZ4_compress_forceExtDict` | saveDict: dictSize=100000 -> 64 KB; `dictSize > stream dictSize` -> clamped; dictSize=0 / safeBuffer=NULL. forceExtDict: byU32 + usingExtDict + dictSmall/noDictIssue, notLimited, acceleration 1 | [x] |

## Module `lz4.c` — decompression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 32 | `LZ4_decompress_safe` | `outputSize >= FASTLOOP_SAFE_DISTANCE(64)` -> `LZ4_FAST_DEC_LOOP`; outputSize=63 -> straight to `safe_decode` | [x] |
| 33 | `LZ4_decompress_safe` | outputSize=0: `srcSize==1 && *src==0` -> 0, anything else -> -1; srcSize=0 with outputSize>0 -> -1; src=NULL or outputSize<0 -> -1 | [x] |
| 34 | `LZ4_decompress_safe` | match offset 1..7 (`inc32table`/`dec64table` byte expand); 8..15 (`LZ4_memcpy_using_offset` cases 1/2/4/default); >=16 (`LZ4_wildCopy32`) | [x] |
| 35 | `LZ4_decompress_safe` | offset=0, or `offset > (op-lowPrefix)+dictSize` while `checkOffset` (dictSize<64 KB) -> negative error code | [x] |
| 36 | `LZ4_decompress_safe` | litLength token == `RUN_MASK(15)` with 255-byte chain; matchLength token == `ML_MASK(15)` with 255-chain; chain past `ilimit` -> `rvl_error`; 32-bit accumulator overflow guard | [x] |
| 37 | `LZ4_decompress_safe` | safe-loop 2-stage shortcut hit: litLen<=14, `ip < iend-16`, `op <= oend-32`, mlToken!=15, offset>=8 -> blind 16+18 byte copies | [x] |
| 38 | `LZ4_decompress_safe` | malformed tail: `ip+length != iend` or `cpy > oend` on the final literal run -> error | [x] |
| 39 | `LZ4_decompress_safe_partial` | targetOutputSize=100 with dstCapacity=1 MB -> clamped to 100; target=1 MB with dstCapacity=100 -> clamped to 100; truncated input (`ip+length > iend`) -> literal length silently truncated, NO error | [x] |
| 40 | `LZ4_decompress_safe_partial` | match crossing `oend - MATCH_SAFEGUARD_DISTANCE(12)` -> byte-wise overlap-safe copy, break at `op==oend` | [x] |
| 41 | `LZ4_decompress_safe_withPrefix64k`, `LZ4_decompress_fast_withPrefix64k` | `lowPrefix = dest-65536`, dictSize=0 -> withPrefix64k with `checkOffset` still enabled | [x] |
| 42 | `LZ4_decompress_safe_usingDict` | dictSize=0 -> plain noDict; `dictStart+dictSize == dest` with dictSize=65535 -> withPrefix64k; same with dictSize=65534 -> withSmallPrefix | [x] |
| 43 | `LZ4_decompress_safe_usingDict`, `LZ4_decompress_safe_forceExtDict` | non-contiguous dict -> usingExtDict; match wholly inside extDict (`length <= lowPrefix-match`) -> single memmove; straddling with `restSize > (op-lowPrefix)` -> byte-wise overlap copy; dictSize=65536 -> `checkOffset==0` | [x] |
| 44 | `LZ4_decompress_safe_partial_usingDict`, `LZ4_decompress_safe_partial_forceExtDict` | all four sub-cases (dictSize==0 / contiguous>=65535 / contiguous<65535 / non-contiguous); extDict + `op+length > oend-LASTLITERALS(5)` -> `length = MIN(length, oend-op)` instead of an error | [x] |
| 45 | `LZ4_decompress_fast`, `LZ4_uncompress`, `LZ4_uncompress_unknownOutputSize` | unsafe generic (no input-bound checking, returns bytes READ, byte-wise match copy) vs the `LZ4_decompress_safe` alias | [x] |
| 46 | `LZ4_decompress_fast_usingDict` | dictSize==0 or `dictStart+dictSize==dest` -> unsafe generic with prefixSize=dictSize; else extDict variant (match split into extDict tail + prefix) | [x] |
| 47 | `LZ4_createStreamDecode`, `LZ4_freeStreamDecode`, `LZ4_setStreamDecode`, `LZ4_decoderRingBufferSize` | setStreamDecode dictSize=0 -> prefixEnd=dictionary, prefixSize=0; dictSize>0 -> prefixEnd=dict+dictSize, externalDict cleared. ringBufferSize: <0 -> 0, >LZ4_MAX_INPUT_SIZE -> 0, 4 -> treated as 16, 65536 -> 131086 | [x] |
| 48 | `LZ4_decompress_safe_continue` | first call, `prefixSize==0 && extDictSize==0` -> plain `LZ4_decompress_safe` | [x] |
| 49 | `LZ4_decompress_safe_continue` | contiguous (`prefixEnd==dest`): prefixSize>=65535 -> withPrefix64k; <65535 && extDictSize==0 -> withSmallPrefix; <65535 && extDictSize>0 -> **doubleDict** | [x] |
| 50 | `LZ4_decompress_safe_continue` | `dest != prefixEnd` (ring wrap / buffer switch) -> prefix promoted to externalDict, forceExtDict; ring buffer sized exactly `LZ4_DECODER_RING_BUFFER_SIZE(65536)` with 65536- and 65537-byte blocks | [x] |
| 51 | `LZ4_decompress_fast_continue` | the same three branches over `LZ4_decompress_unsafe_generic` | [x] |

## Module `lz4.c` — deprecated / legacy family

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 52 | `LZ4_compress`, `LZ4_compress_limitedOutput`, `LZ4_compress_withState`, `LZ4_compress_limitedOutput_withState` | acceleration hard-wired 1; dstCapacity = `LZ4_compressBound` (plain / `_withState`) vs caller-supplied (`limitedOutput` variants) | [x] |
| 53 | `LZ4_compress_continue`, `LZ4_compress_limitedOutput_continue` | `LZ4_compress_fast_continue` with acceleration 1; dstCapacity=bound vs supplied | [x] |
| 54 | `LZ4_sizeofStreamState`, `LZ4_resetStreamState`, `LZ4_create`, `LZ4_slideInputBuffer` | `inputBuffer` argument ignored throughout; `resetStreamState` = full `LZ4_resetStream` returning 0; `LZ4_create` == `LZ4_createStream`; `LZ4_slideInputBuffer` merely returns `internal.dictionary` (no sliding — legacy stub) | [x] |

## Module `lz4hc.c`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 55 | `LZ4_compress_HC`, `LZ4_compressHC`, `LZ4_compressHC_limitedOutput` | compressionLevel <= 0 -> `LZ4HC_CLEVEL_DEFAULT(9)` -> **hashChain**, nbSearches=256, patternAnalysis=1 | [x] |
| 56 | `LZ4_compress_HC` | level=1 -> **lz4mid** (`LZ4MID_compress`), nbSearches=2, hash4 + hash8 tables | [x] |
| 57 | `LZ4_compress_HC` | level=2 (`LZ4HC_CLEVEL_MIN`) -> **lz4mid**; no-match skip is `ip += 1 + ((ip-anchor)>>9)` | [x] |
| 58 | `LZ4_compress_HC` | level=3 -> hashChain, nbSearches=4, patternAnalysis=0 | [x] |
| 59 | `LZ4_compress_HC` | level=8 -> nbSearches=128, patternAnalysis=**0** (strict `> 128` test) | [x] |
| 60 | `LZ4_compress_HC` | level=9 -> nbSearches=256, patternAnalysis=**1** (only hashChain level with repeat-pattern analysis) | [x] |
| 61 | `LZ4_compress_HC` | level=10 (`OPT_MIN`) -> **lz4opt**, 96 / 64, fullUpdate=0; level=11 -> lz4opt, 512 / 128, fullUpdate=0 | [x] |
| 62 | `LZ4_compress_HC` | level=12 (`MAX`) -> **lz4opt**, 16384 searches, `sufficient_len=LZ4_OPT_NUM(4096)` clamped to 4095, **fullUpdate=1** | [x] |
| 63 | `LZ4_compress_HC`, `LZ4_compressHC2`, `LZ4_compressHC2_limitedOutput` | level=13 / 1000 / INT_MAX -> clamped to 12; explicit-cLevel wrappers with bound vs supplied dstCapacity | [x] |
| 64 | `LZ4_compress_HC` | `dstCapacity >= LZ4_compressBound` -> notLimited; `< bound` -> limitedOutput (returns 0 and sets `ctx->dirty=1`) | [x] |
| 65 | `LZ4_compress_HC` | srcSize=12 (`< LZ4_minLength=13`) -> all-literals exit in both `LZ4MID_compress` and hashChain; srcSize=0; srcSize > LZ4_MAX_INPUT_SIZE -> 0 | [x] |
| 66 | `LZ4_compress_HC_extStateHC`, `LZ4_compress_HC_extStateHC_fastReset`, `LZ4_resetStreamHC_fast` | misaligned state -> 0; `dirty==1` -> full `LZ4_initStreamHC`; `dirty==0` -> `dictLimit += (end-prefixStart)`, prefixStart/end/dictCtx nulled | [x] |
| 67 | `LZ4_compress_HC_destSize` | fillOutput: `oend -= LASTLITERALS` hack, `*sourceSizePtr` updated, last sequence trimmed to `maxMlSize`; targetDestSize=0 -> 0 | [x] |
| 68 | `LZ4_compress_HC_continue`, `LZ4_compressHC_continue`, `LZ4_compressHC_limitedOutput_continue` | `src == ctx->end` (contiguous prefix); dstCapacity>=bound -> notLimited else limitedOutput | [x] |
| 69 | `LZ4_compress_HC_continue` | `src != ctx->end` -> `LZ4HC_setExternalDict` (prefix becomes extDict, dictCtx dropped); level>=3 also runs `LZ4HC_Insert(end-3)` when `end-prefixStart>=4` — levels 1/2 (lz4mid) SKIP the Insert | [x] |
| 70 | `LZ4_compress_HC_continue` | src overlapping `[dictStart, dictStart + (dictLimit-lowLimit))` (ring buffer); resulting `dictLimit-lowLimit < LZ4HC_HASHSIZE(4)` -> extDict invalidated | [x] |
| 71 | `LZ4_compress_HC_continue` | `(end-prefixStart) + dictLimit > 2 GB` -> forced `LZ4_loadDictHC` re-seed from the last `min(prefixSize, 64 KB)` bytes | [x] |
| 72 | `LZ4_compress_HC_continue_destSize` | fillOutput on a continued stream (prefix or extDict), `*srcSizePtr` only partially consumed | [x] |
| 73 | `LZ4_loadDictHC` | dictSize=100000 -> last 64 KB only; dictSize=3 (`< LZ4HC_HASHSIZE=4`) -> no `LZ4HC_Insert`; level>=3 -> `LZ4HC_Insert(end-3)` | [x] |
| 74 | `LZ4_loadDictHC` | level 1/2 -> **`LZ4MID_fillHTable`** instead of `LZ4HC_Insert`; `dictSize <= LZ4MID_HASHSIZE(8)` -> immediate return; `dictSize > 32 KB + 8` -> dense hash8 pass restricted to the last 32 KB | [x] |
| 75 | `LZ4_attach_HC_dictionary` | `position = (end-prefixStart) + (dictLimit-lowLimit) >= 64 KB` -> dictCtx dropped, noDictCtx path | [x] |
| 76 | `LZ4_attach_HC_dictionary` | position==0 && srcSize=4097 (>4 KB) && `isStateCompatible` (both lz4mid or both non-mid) -> dictCtx memcpy'd into ctx + `LZ4HC_setExternalDict`, then noDictCtx | [x] |
| 77 | `LZ4_attach_HC_dictionary` | position==0 && srcSize<=4 KB, OR strategy mismatch (working level 9 vs dictCtx level 2) at any size -> **usingDictCtxHc** | [x] |
| 78 | `LZ4HC_searchExtDict` | direct call; nbAttempts=2 (via `LZ4MID_searchHCDict`) vs nbAttempts=nbSearches; only candidates with `ipIndex-matchIndex <= LZ4_DISTANCE_MAX(65535)` walked | [x] |
| 79 | `LZ4_favorDecompressionSpeed` | favor=1 + level 10..12 -> offsets <8 skipped in `InsertAndGetWiderMatch`, matchLen in (18,36] forced to 18, price tie-break `- (int)favorDecSpeed` | [x] |
| 80 | `LZ4_favorDecompressionSpeed` | favor=1 + level 1..9 -> flag stored but never consumed (always `favorCompressionRatio`) | [x] |
| 81 | `LZ4_setCompressionLevel`, `LZ4_resetStreamHC`, `LZ4_initStreamHC` | setCompressionLevel: <1 -> 9, >12 -> 12; initStreamHC: NULL -> NULL, `size < sizeof(LZ4_streamHC_t)` -> NULL, misaligned -> NULL; resetStreamHC = init + setCompressionLevel | [x] |
| 82 | `LZ4_createStreamHC`, `LZ4_freeStreamHC`, `LZ4_createHC`, `LZ4_freeHC`, `LZ4_slideInputBufferHC` | createStreamHC presets level 9; createHC = createStreamHC + `LZ4HC_init_internal(inputBuffer)`; slideInputBufferHC returns `prefixStart - dictLimit + lowLimit` AND performs `LZ4_resetStreamHC_fast`; free(NULL) -> 0 | [x] |
| 83 | `LZ4_saveDictHC`, `LZ4_sizeofStateHC`, `LZ4_sizeofStreamStateHC`, `LZ4_resetStreamStateHC` | saveDictHC: dictSize=100000 -> 64 KB, dictSize=3 -> 0, `dictSize > prefixSize` -> prefixSize, safeBuffer=NULL -> dictSize must be 0; resetStreamStateHC returns 1 on init failure / 0 on success | [x] |
| 84 | `LZ4_compressHC_withStateHC`, `LZ4_compressHC_limitedOutput_withStateHC`, `LZ4_compressHC2_withStateHC`, `LZ4_compressHC2_limitedOutput_withStateHC` | level hard-wired 0 (-> 9) vs explicit cLevel (2 -> lz4mid, 12 -> lz4opt); full `LZ4_initStreamHC` per call; bound vs supplied dstCapacity | [x] |
| 85 | `LZ4_compressHC2_continue`, `LZ4_compressHC2_limitedOutput_continue` | call `LZ4HC_compress_generic` DIRECTLY: the continue variant forces dstCapacity=0 with notLimited (no bound check); both bypass `ctxPtr->compressionLevel`, `setExternalDict`, overlap handling and the >2 GB renorm | [x] |
| 86 | `LZ4_compress_HC` levels 10..12 | `firstMatch.len > sufficient_len` (level 10, match >64 B) -> immediate encode, price table never built; `newMatch.len + cur >= LZ4_OPT_NUM(4096)` -> immediate encode; fullUpdate=1 changes the skip heuristic | [x] |
| 87 | `LZ4_compress_HC` levels 3..9 | 3-ascending-match squeeze path in hashChain: start0/start2/start3, `OPTIMAL_ML=18` corrections, `(start2-ip) < 3` match removal, `_dest_overflow` last-sequence salvage | [x] |
| 88 | `LZ4_compress_HC` levels 9..12 | 1 MB of one repeated byte -> `LZ4HC_countPattern` / `reverseCountPattern` / `rotatePattern` + `protectDictEnd` (last 3 bytes of dict); chainSwap active at levels 10-12 | [x] |

## Module `lz4frame.c` — compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 89 | `LZ4F_getVersion`, `LZ4F_isError`, `LZ4F_getErrorName`, `LZ4F_getErrorCode`, `LZ4F_compressionLevel_max` | code=0 (OK) vs every code up to `-(maxCode-1)`; isError boundary at `-LZ4F_ERROR_maxCode`; compressionLevel_max returns 12 | [x] |
| 90 | `LZ4F_getBlockSize` | blockSizeID=0 -> 64 KB (`LZ4F_BLOCKSIZEID_DEFAULT`); 4/5/6/7 -> 64 KB/256 KB/1 MB/4 MB; 1,2,3,8 and any other int -> `maxBlockSize_invalid` | [x] |
| 91 | `LZ4F_compressBound` | prefs=NULL -> worst case (content AND block checksum forced on, 64 KB blocks) with `alreadyBuffered=(size_t)-1` -> `maxBuffered=blockSize-1`; autoFlush=1 -> alreadyBuffered=0; srcSize=0 -> flush bound | [x] |
| 92 | `LZ4F_compressFrameBound` | forces autoFlush=1 and adds `maxFHSize=19`; prefs=NULL -> zeroed prefs (blockSizeID 0 -> 64 KB) | [x] |
| 93 | `LZ4F_compressFrame` | prefs=NULL -> zeroed prefs (64 KB, linked, no checksums, level 0, stack `LZ4_stream_t` since LZ4F_HEAPMODE=0); srcSize=0 -> header + endMark only | [x] |
| 94 | `LZ4F_compressFrame`, `LZ4F_compressFrame_usingCDict` | blockSizeID=7 (4 MB) with srcSize=1000 -> `LZ4F_optimalBSID` downgrades to `LZ4F_max64KB(4)` | [x] |
| 95 | `LZ4F_compressFrame` | `srcSize <= LZ4F_getBlockSize(blockSizeID)` -> blockMode forced to **blockIndependent**; srcSize=blockSize+1 with linked -> stays linked; autoFlush and `options.stableSrc` both forced to 1 | [x] |
| 96 | `LZ4F_compressFrame` | `frameInfo.contentSize != 0` -> auto-corrected to the actual srcSize; `dstCapacity < compressFrameBound` -> `dstMaxSize_tooSmall`; level>=2 -> HC ctx heap-allocated inside compressBegin then freed | [x] |
| 97 | `LZ4F_compressFrame_usingCDict`, `LZ4F_createCDict`, `LZ4F_createCDict_advanced`, `LZ4F_freeCDict` | dictSize=100000 -> only the last 64 KB copied; fastCtx built with **`LZ4_loadDictSlow`**, HCCtx with **`LZ4_loadDictHC` at level 9**; level<2 -> `LZ4_resetStream_fast` + `LZ4_attach_dictionary`; level>=2 -> `LZ4_resetStreamHC_fast` + `LZ4_attach_HC_dictionary`; custom `LZ4F_CustomMem` vs `LZ4F_defaultCMem` | [x] |
| 98 | `LZ4F_createCompressionContext`, `LZ4F_createCompressionContext_advanced`, `LZ4F_freeCompressionContext` | default cmem (calloc) vs customCalloc vs customAlloc+memset; free(NULL) -> OK; contextPtr==NULL -> `parameter_null` | [x] |
| 99 | `LZ4F_compressBegin`, `LZ4F_compressBegin_internal` | prefs=NULL -> `LZ4F_INIT_PREFERENCES`; `dstCapacity < maxFHSize(19)` -> `dstMaxSize_tooSmall`; level<2 -> ctxTypeID 1 (`LZ4_sizeofState`); level>=2 -> ctxTypeID 2 (`LZ4_sizeofStateHC`); re-begin switching level 9 -> 0 keeps the buffer but re-inits it as `LZ4_stream_t` | [x] |
| 100 | `LZ4F_compressBegin` | `frameInfo.blockSizeID=0` -> coerced to `LZ4F_max64KB` before maxBlockSize is computed; maxBlockSize = 64 KB / 256 KB / 1 MB / 4 MB | [x] |
| 101 | `LZ4F_compressBegin` | tmpBuff sizing matrix: (autoFlush=1, linked) -> 64 KB; (autoFlush=1, independent) -> 0; (autoFlush=0, linked) -> maxBlockSize+128 KB; (autoFlush=0, independent) -> maxBlockSize | [x] |
| 102 | `LZ4F_compressBegin` | contentSize=0 -> 7-byte header; contentSize=12345 -> +8 bytes and totalInSize reset; dictID=0 vs 0xDEADBEEF -> +4 bytes; both -> 19-byte header; FLG bits `(blockMode<<5, blockChecksum<<4, (contentSize>0)<<3, contentChecksum<<2, (dictID>0)<<0)`, BD = `blockSizeID<<4`, trailing `XXH32(header)>>8` byte | [x] |
| 103 | `LZ4F_compressBegin` | level>=2 + favorDecSpeed=1 -> `LZ4_favorDecompressionSpeed` on the HC ctx (only effective at level>=10); level<2 -> favorDecSpeed silently ignored | [x] |
| 104 | `LZ4F_compressBegin` | blockMode=linked -> `LZ4F_initStream` once per frame; blockMode=independent -> stream re-initialised per block inside `LZ4F_compressBlock` / `LZ4F_compressBlockHC` | [x] |
| 105 | `LZ4F_compressBegin_usingDict`, `LZ4F_compressBegin_usingDictOnce` | dictBuffer non-NULL: level<2 -> `LZ4_loadDict`, level>=2 -> `LZ4_loadDictHC`; `dictSize > INT_MAX` -> `parameter_invalid`; with blockIndependent the dict applies only to the FIRST block (documented defect: `_usingDict` == `_usingDictOnce`) | [x] |
| 106 | `LZ4F_compressBegin_usingCDict` | cdict + blockLinked -> attached once at begin; cdict + blockIndependent -> `LZ4F_compressBlock` re-attaches per block and uses **`LZ4_compress_fast_continue`** instead of `LZ4_compress_fast_extState_fastReset` | [x] |
| 107 | `LZ4F_compressUpdate` | autoFlush=0, srcSize=1000 < blockSize=65536 -> fully buffered into tmpIn, returns 0 | [x] |
| 108 | `LZ4F_compressUpdate` | srcSize == blockSize exactly -> one block straight from srcBuffer (`fromSrcBuffer`); srcSize == blockSize+1 -> autoFlush=0: 1 block + 1 byte buffered; autoFlush=1: full block + a 1-byte block | [x] |
| 109 | `LZ4F_compressUpdate` | tmpInSize>0 and `srcSize >= blockSize-tmpInSize` -> `fromTmpBuffer` block, and `tmpIn += blockSize` when blockLinked; `srcSize < blockSize-tmpInSize` -> append only, nothing emitted | [x] |
| 110 | `LZ4F_compressUpdate` | blockLinked + fromSrcBuffer + `compressOptions.stableSrc=1` -> tmpIn reset to tmpBuff, dictionary left in the caller's src; stableSrc=0 (or optionsPtr=NULL) -> `LZ4F_localSaveDict` = `LZ4_saveDict` (level<2) / `LZ4_saveDictHC` (level>=2) | [x] |
| 111 | `LZ4F_compressUpdate` | autoFlush=0 and `tmpIn+blockSize > tmpBuff+maxBufferSize` -> forced localSaveDict and tmpIn rewound (reachable with blockSize=4 MB, linked) | [x] |
| 112 | `LZ4F_compressUpdate` | `dstCapacity < LZ4F_compressBound_internal(srcSize, prefs, tmpInSize)` -> `dstMaxSize_tooSmall`; `cStage != 1` -> `compressionState_uninitialized` | [x] |
| 113 | `LZ4F_compressUpdate` | incompressible block (`cSize==0 || cSize >= srcSize`) -> stored raw with `LZ4F_BLOCKUNCOMPRESSED_FLAG (0x80000000)`; blockChecksumFlag=on -> 4-byte XXH32 of the STORED bytes appended per block | [x] |
| 114 | `LZ4F_uncompressedUpdate`, `LZ4F_compressUpdate`, `LZ4F_flush` | `LZ4B_UNCOMPRESSED` (`LZ4F_doNotCompressBlock` always returns 0) -> every block stored raw; requires blockIndependent; `dstCapacity < srcSize` -> `dstMaxSize_tooSmall`; alternating compressed/uncompressed on one cctx -> mode change triggers an implicit `LZ4F_flush` | [x] |
| 115 | `LZ4F_flush`, `LZ4F_compressEnd` | flush: tmpInSize==0 -> 0; `dstCapacity < tmpInSize+BHSize(4)+BFSize(4)` -> `dstMaxSize_tooSmall`; blockLinked -> tmpIn advance + possible localSaveDict. compressEnd: 4-byte endMark, +4-byte `XXH32_digest` when contentChecksum on (needs dstCapacity>=8); declared contentSize != totalInSize -> `frameSize_wrong`; cStage reset to 0 so the cctx is reusable | [x] |

## Module `lz4frame.c` — decompression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 116 | `LZ4F_createDecompressionContext`, `LZ4F_createDecompressionContext_advanced`, `LZ4F_freeDecompressionContext`, `LZ4F_resetDecompressionContext` | default vs custom cmem; freeDecompressionContext returns the current dStage (non-zero -> frame incomplete); reset clears dict/dictSize/skipChecksum/frameRemainingSize | [x] |
| 117 | `LZ4F_headerSize` | src=NULL -> `srcPtr_wrong`; `srcSize < 5` -> `frameHeader_incomplete`; skippable magic -> 8; wrong magic -> `frameType_unknown`; FLG contentSize/dictID combinations -> 7 / 11 / 15 / 19 | [x] |
| 118 | `LZ4F_getFrameInfo` | fresh dctx with >= headerSize bytes -> header decoded, `*srcSizePtr=headerSize`, returns `BHSize=4`; dStage==`dstage_storeFrameHeader` -> `frameDecoding_alreadyStarted` with `*srcSizePtr=0`; dStage>storeFrameHeader -> no input consumed | [x] |
| 119 | `LZ4F_decompress` | `srcSize >= maxFHSize(19)` -> header shortcut branch; srcSize=0 on a fresh dctx -> returns `minFHSize(7)` consuming nothing; header fed 1 byte per call -> `dstage_storeFrameHeader` loop with hint `(tmpInTarget-tmpInSize)+4` | [x] |
| 120 | `LZ4F_decompress` | invalid headers: FLG bit1 -> `reservedFlag_set`; version!=1 -> `headerVersion_wrong`; BD bit7 or low 4 bits nonzero -> `reservedFlag_set`; blockSizeID<4 -> `maxBlockSize_invalid`; wrong HC byte -> `headerChecksum_invalid` | [x] |
| 121 | `LZ4F_decompress` | **skippable frame** (`magic & 0xFFFFFFF0 == LZ4F_MAGIC_SKIPPABLE_START`): `src != dctx->header` -> `dstage_getSFrameSize` (consumes 4); `src == dctx->header` -> `dstage_storeSFrameSize` (tmpInTarget=8); SFrameSize larger than the provided src -> `dstage_skipSkippable` across several calls | [x] |
| 122 | `LZ4F_decompress` | `dstage_init` buffer sizing: `tmpIn = maxBlockSize + BFSize(4)`; `tmpOutBuffer = maxBlockSize + (blockLinked ? 128 KB : 0)`; realloc only when `bufferNeeded > maxBufferSize` (reuse across frames) | [x] |
| 123 | `LZ4F_decompress` | blockHeader==0 -> `dstage_getSuffix`; `nextCBlockSize > maxBlockSize` -> `maxBlockSize_invalid`; block header split across calls -> `dstage_storeBlockHeader`; `dstPtr==dstEnd` or `srcPtr==srcEnd` right after the header -> early return with hint `4 + cBlockSize + crcSize` | [x] |
| 124 | `LZ4F_decompress` | uncompressed block (high bit set) -> `dstage_copyDirect`; dstBuffer=NULL -> sizeToCopy=0, no progress; block larger than the remaining dst -> partial memcpy, tmpInTarget decremented, resumes next call | [x] |
| 125 | `LZ4F_decompress` | uncompressed block + blockChecksumFlag -> `dstage_getBlockChecksum`; the 4 CRC bytes split across calls are buffered in `dctx->header`; mismatch -> `blockChecksum_invalid` | [x] |
| 126 | `LZ4F_decompress` | compressed block fully present in src -> decoded straight from srcPtr; split across calls -> `dstage_storeCBlock` buffering into tmpIn; blockChecksum verified over `tmpInTarget-4` bytes | [x] |
| 127 | `LZ4F_decompress` | `(dstEnd-dstPtr) >= maxBlockSize` AND NOT (`dict+dictSize == tmpOut`) -> `LZ4_decompress_safe_usingDict` directly into the caller's dstBuffer | [x] |
| 128 | `LZ4F_decompress` | dst smaller than maxBlockSize (or dict ends exactly at tmpOut) -> decode into tmpOut, then `dstage_flushOut` in chunks with hint `BHSize` | [x] |
| 129 | `LZ4F_decompress` | `dict != NULL && dictSize > 1 GB` -> dict advanced to the last 64 KB before the `LZ4_decompress_safe_usingDict` call (both the direct-to-dst and tmpOut variants) | [x] |
| 130 | `LZ4F_decompress` | blockLinked, `dict == tmpOutBuffer && dictSize > 128 KB` -> last 64 KB memcpy'd to the front and dictSize=64 KB; dict outside tmpOut -> `tmpOut = tmpOutBuffer + MIN(dictSize, 64 KB)` | [x] |
| 131 | `LZ4F_decompress` | blockLinked `LZ4F_updateDict`: prefix case (`dict+dictSize == dstPtr`) -> dictSize just extended; `(dstPtr-dstStart)+dstSize >= 64 KB` -> dict repointed into the caller's dstBuffer | [x] |
| 132 | `LZ4F_decompress` | blockLinked `LZ4F_updateDict` tmpOut branches: withinTmp=1 with `dict==tmpOutBuffer` (extend); withinTmp=1 otherwise (copy `64KB-tmpOutSize` in front of tmpOut); withinTmp=0 with `dict==tmpOutBuffer` (append, shrink when `dictSize+dstSize > maxBufferSize`); withinTmp=0 join dict+dst into tmp | [x] |
| 133 | `LZ4F_decompress` | blockLinked + `decompressOptions.stableDst=1` -> the tail history-preservation memcpy is skipped entirely; stableDst=0 -> `dstage_flushOut` takes the partial-tmpOut preserve path, other stages in `[dstage_init, dstage_getSuffix)` take the `MIN(dictSize,64KB)` tail copy | [x] |
| 134 | `LZ4F_decompress` | `decompressOptions.skipChecksums=1` -> block AND content checksum verification skipped; sticky (OR'ed into `dctx->skipChecksum`) for the rest of the frame | [x] |
| 135 | `LZ4F_decompress` | contentChecksumFlag on with the 4 suffix bytes split across calls -> `dstage_storeSuffix`; mismatch -> `contentChecksum_invalid`; flag off -> frame completes at endMark and the dctx auto-resets; contentSize declared but `frameRemainingSize != 0` -> `frameSize_wrong` | [x] |
| 136 | `LZ4F_decompress` | blockIndependent -> no `LZ4F_updateDict` and no tail preservation at all (dict stays NULL); corrupt block payload -> `LZ4_decompress_safe_usingDict < 0` -> `decompressionFailed` | [x] |
| 137 | `LZ4F_decompress_usingDict` | called while `dStage <= dstage_init` -> external dict installed for the frame (extDict path inside `LZ4_decompress_safe_usingDict` for the first block); called mid-frame -> the dict argument is silently ignored | [x] |

## Module `lz4file.c`

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 138 | `LZ4F_writeOpen` | prefs=NULL -> maxWriteSize 64 KB; blockSizeID 0/4 -> 64 KB, 5 -> 256 KB, 6 -> 1 MB, 7 -> 4 MB; other IDs -> `maxBlockSize_invalid`; fp=NULL or lz4fWrite=NULL -> `parameter_null`; dstBuf sized by `LZ4F_compressBound(maxWriteSize, prefs)` | [x] |
| 139 | `LZ4F_write`, `LZ4F_writeClose` | `size <= maxWriteSize` -> single `LZ4F_compressUpdate`; `size > maxWriteSize` -> chunked loop; size=0 -> no-op; buf=NULL -> `parameter_null`. writeClose: errCode==OK -> `LZ4F_compressEnd` + fwrite; errCode already latched -> compressEnd skipped, state still freed | [x] |
| 140 | `LZ4F_readOpen`, `LZ4F_read`, `LZ4F_readClose` | readOpen requires a file >= `LZ4F_HEADER_SIZE_MAX(19)` bytes, else `io_read` (fails even for a valid tiny frame); srcBuf sized from the header blockSizeID; read spanning several srcBuf refills; EOF mid-stream (`fread==0`) -> loop breaks, short count returned; NULL args -> `parameter_null` | [x] |

## Module `xxhash.c` (exported as `LZ4_XXH*`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 141 | `LZ4_XXH_versionNumber` | returns `XXH_VERSION_NUMBER` | [x] |
| 142 | `LZ4_XXH32` | len=0 (`h32 = seed+PRIME32_5`, finalize case 0); len=1..15 (no round loop, `switch(len&15)` cases 1,2,3,5..15); len=16 (exactly one 4x round pass); len=17 (pass + case 1); len=31/32 | [x] |
| 143 | `LZ4_XXH32` | input pointer 4-byte aligned vs unaligned (`XXH_FORCE_ALIGN_CHECK`); seed=0 vs seed=0x9E3779B1 and the full seed sweep | [x] |
| 144 | `LZ4_XXH32_createState`, `_reset`, `_update`, `_digest`, `_freeState` | update with `memsize+len < 16` -> buffered only; update filling `mem32` exactly to 16; update leaving a 1..15 byte remainder; digest with `total_len < 16`; digest of a partially-buffered state | [x] |
| 145 | `LZ4_XXH32_copyState`, `_canonicalFromHash`, `_hashFromCanonical` | copy a mid-stream state (nonzero memsize) then continue both copies independently; 4-byte big-endian canonical round trip | [x] |
| 146 | `LZ4_XXH64` | len=0; len=1..31 (`seed+PRIME64_5`, `switch(len&31)` 8/4/1-byte PROCESS combinations); len=32 (one 4x round pass); len=33..63; 8-byte aligned vs unaligned; seed=0 vs nonzero | [x] |
| 147 | `LZ4_XXH64_createState`, `_reset`, `_update`, `_digest`, `_freeState` | 32-byte memsize buffering: `memsize+len < 32` -> buffer only; exact fill; 1..31 byte remainder; digest with `total_len < 32` | [x] |
| 148 | `LZ4_XXH64_copyState`, `_canonicalFromHash`, `_hashFromCanonical` | mid-stream copy; 8-byte big-endian canonical round trip | [x] |

## Symbol coverage

All 143 exported symbols appear in at least one row.  Notes:

* `LZ4_compress_forceExtDict`, `LZ4_decompress_safe_forceExtDict` and
  `LZ4_decompress_safe_partial_forceExtDict` are declared inside `lz4.c` under
  *"Internal Definitions, used only in Tests"* — exported, but absent from every
  public header.  They are real entry points and are covered (rows 31, 43, 44).
* `LZ4_slideInputBuffer` / `LZ4_slideInputBufferHC` are non-functional legacy
  stubs whose only observable effects are the returned pointer and (for the HC
  variant) an implicit `LZ4_resetStreamHC_fast` (rows 54, 82).
* `LZ4HC_searchExtDict` and `LZ4F_compressBegin_internal` /
  `LZ4F_compressBegin_usingDictOnce` / `LZ4_loadDict_internal` are exported
  internals; they are driven directly (rows 78, 99, 105, 18/20).

## Which test covers which rows

All tests live in `translation/tests/` and load BOTH `.so` files with
`libloading`; no Rust function is ever called directly.  Run everything with
`translation/verify.sh`.

| rows | module | test file | `#[test]`s | result |
|------|--------|-----------|-----------|--------|
| 1-54 | `lz4.c` block + streaming + legacy | `tests/block.rs` | 35 | pass |
| 55-88 | `lz4hc.c` | `tests/hc.rs` | 25 | pass |
| 89-137 | `lz4frame.c` valid paths | `tests/frame.rs` | 30 | pass |
| 138-140 | `lz4file.c` | `tests/file.rs` | 15 | pass |
| 141-148 | `xxhash.c` | `tests/xxh.rs` | 16 | pass |
| 12, 30, 70, 71 (deep) + large inputs | cross-module | `tests/extra.rs` | 5 + 4 `#[ignore]`d | pass |
| symbol parity | all | `tests/smoke.rs` | 4 | pass |
| (error surface) | all | `tests/frame_err.rs` + the error sections of the above | 36 | pass |

**Total: 170 `#[test]` functions (166 default + 4 `#[ignore]`d), 0 failures.**  `translation/src/**` was never
modified — no divergence was found that required a fix (verified with
`md5sum` against a baseline taken before testing began).

### Rows that needed extra work to reach

* **Rows 12, 30** (`LZ4_renormDictT`, `LZ4_prepareTable` above 1 GB) and
  **rows 70, 71** (the HC 1 GB / 2 GB re-seeds) are only reachable after
  gigabytes have passed through a single stream, so per-call tests cannot touch
  them.  `tests/extra.rs` streams 2.1 GiB / 2.2 GiB through one
  `LZ4_stream_t` / `LZ4_streamHC_t`, comparing the full state bytes on **every**
  iteration, and then asserts via `assert_renormalized()` that `currentOffset`
  really was rewound — otherwise the test could pass vacuously without ever
  reaching the branch.  These four tests are `#[ignore]`d (~20 min) and are run
  by `verify.sh` unless `VERIFY_SKIP_SLOW` is set.
* **Row 139, the `size > maxWriteSize` branch**, was initially NOT covered: every
  `LZ4F_write` call in the first version of `tests/file.rs` was smaller than the
  frame block size, so `lz4file.c`'s chunking loop never ran.  Found by mutation
  testing (see below) and fixed by
  `file_write_larger_than_maxWriteSize_in_one_call`, which hands a multi-megabyte
  payload to a single `LZ4F_write` for every block size.

### Rows deliberately not reached

* **Row 129** (`LZ4F_decompress` advancing `dict` when `dictSize > 1 GB`) needs a
  decoded history above 1 GiB inside one frame.  It is unreachable from any
  input this harness can construct in reasonable time and is recorded here as a
  known, explicit gap rather than silently checked off.
* `byPtr` tableType (mentioned in the tableType table above) is
  32-bit-only — `sizeof(void*) == 4` — and cannot be reached on this
  x86-64 build of either library.

## Mutation testing (evidence that the tests can actually fail)

Because the suite found zero divergences, its *sensitivity* was measured
directly: seven single-token mutations were injected into copies of
`translation/src/**` (never the real tree), each built to its own `.so`, and the
whole suite was re-run against the mutant via the harness's `LZ4_RUST_SO`
override.

| mutation | detected by |
|----------|-------------|
| `lz4.rs` byU16 hash shift `HASHLOG+1` -> `HASHLOG+2` | smoke, block, frame, frame_err, file, extra (out-of-bounds table write -> SIGSEGV) |
| `lz4.rs` `LZ4_hash5` prime `889523592379` -> `...377` | block (16), frame (16), frame_err (6), file (3), extra (3) |
| `xxhash.rs` `PRIME32_2` `2246822519` -> `2246822521` | xxh (8), frame (27), frame_err (22), file (11), extra (2) |
| `lz4hc.rs` patternAnalysis `> 128` -> `>= 128` | hc (8), extra (1) |
| `lz4frame.rs` header-checksum byte `>> 8` -> `>> 7` | frame (27), frame_err (20), file (11), extra (1) |
| `lz4file.rs` `writeOpen` maxWriteSize `256K` -> `255K` | file (1) — **only after adding the row-139 test above; this is how that gap was found** |
| `lz4file.rs` `readOpen` srcBufMaxSize `256K` -> `255K` | **not detected — provably equivalent.** `srcBufMaxSize` only sizes the `fread` staging buffer; `LZ4F_decompress` accepts any input chunking, so a 255 KiB buffer just refills more often and the output bytes, return codes and consumed counts are identical. Confirmed by a control mutation to `1` byte, which **is** detected (file, 1 failure) — so the field is observable, and the 255 KiB variant genuinely lies in the equivalence class. |

One earlier attempted mutation (the non-`BY_U16` branch of `LZ4_hash4`) produced a
byte-identical `.so`: that branch is dead code, since `LZ4_hashPosition` only
calls `LZ4_hash4` when `table_type == BY_U16`.  It is listed here so the null
result is not mistaken for a coverage gap.
