# CONFIGS.md — Configuration-surface table

Derived mechanically from the axes the C code actually branches on:
`c_src/include/*.h` public API + the `if`/`switch` branches in
`c_src/src/{lz4.c,lz4hc.c,lz4frame.c,lz4file.c,xxhash.c}`.

## Axes the C branches on

**Block API (lz4.c)**
- `acceleration`: `<=0` → default 1; 1; 2..; clamped at `LZ4_ACCELERATION_MAX` (65537) — lz4.c `LZ4_compress_fast`
- table type: `byU16` when `srcSize < LZ4_64Klimit` (65536+1) else `byU32` — lz4.c:1422/1436/1489
- `dict` directive: `noDict` / `withPrefix64k` / `usingExtDict` / `usingDictCtx` — lz4.c `LZ4_compress_generic`
- `outputDirective`: `notLimited` (dstCapacity >= compressBound) / `limitedOutput` / `fillOutput` (destSize)
- streaming: fresh stream / `LZ4_loadDict` prefix / `LZ4_loadDictSlow` / `LZ4_attach_dictionary` / ring-buffer continue / `LZ4_saveDict` / `LZ4_slideInputBuffer`
- decompress: `safe` / `fast` / `safe_partial` / `_usingDict` / `_withPrefix64k` / `_forceExtDict` / `_continue`

**HC API (lz4hc.c)**
- `compressionLevel` strategy table `k_clTable` — lz4hc.c:92-105:
  `lz4mid` (1-2), `lz4hc` (3-9), `lz4opt` (10-12, level 12 = ultra)
- `favorDecSpeed` (only effective for level >= `LZ4HC_CLEVEL_OPT_MIN` = 10) — lz4hc.c:1409
- `limit`: `notLimited` / `limitedOutput` / `fillOutput`
- ext-dict vs prefix vs dictCtx; `LZ4HC_searchExtDict`

**Frame API (lz4frame.c)** — `LZ4F_preferences_t` axes:
- `blockSizeID` ∈ {0=default64KB, 4=64KB, 5=256KB, 6=1MB, 7=4MB}
- `blockMode` ∈ {blockLinked=0, blockIndependent=1}
- `contentChecksumFlag` ∈ {0,1}
- `blockChecksumFlag` ∈ {0,1}
- `contentSize` ∈ {0=unknown, exact known size}
- `dictID` ∈ {0=absent, nonzero}
- `compressionLevel` ∈ {<0 fast-accel, 0 default, 1, 2, 3, 9, 10, 12}  (selects fast vs HC vs HC-opt engine)
- `autoFlush` ∈ {0,1}
- `favorDecSpeed` ∈ {0,1}
- `LZ4F_compressOptions_t.stableSrc` ∈ {0,1}
- `LZ4F_decompressOptions_t.stableDst` ∈ {0,1}, `.skipChecksums` ∈ {0,1}
- input feeding shape: one-shot / chunked (1 byte, small, exactly blockSize, blockSize±1, huge) / empty
- decompress output shape: one-shot large dst / byte-at-a-time dst / small dst loop
- entry points: one-shot `LZ4F_compressFrame` vs low-level
  `compressBegin`/`compressUpdate`/`uncompressedUpdate`/`flush`/`compressEnd`;
  dict variants `compressBegin_usingDict`, `compressBegin_usingDictOnce`,
  `compressBegin_usingCDict`, `compressFrame_usingCDict`, `decompress_usingDict`

**File API (lz4file.c)**: `writeOpen`/`write`/`writeClose`, `readOpen`/`read`/`readClose`;
read granularity {1, small, large}; prefs NULL vs set.

**xxhash.c**: XXH32/XXH64; one-shot vs streaming reset/update/digest;
update chunking {1, 3, 15, 16, 31, 32, 33, aligned}; canonical from/to hash;
seed {0, nonzero}; length {0,1,3,4,15,16,31,32,33,64,255,1KB}.

---

## Configuration rows

Each row is exercised with many randomized inputs (fixed seed) against BOTH `.so`s.

### Block API — one-shot compression

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `LZ4_compress_default` | random sizes 0..70000, dst = compressBound (notLimited) | [x] |
| 2 | `LZ4_compress_default` | srcSize < 64K (byU16 table), highly compressible data | [x] |
| 3 | `LZ4_compress_default` | srcSize > 64K (byU32 table), highly compressible data | [x] |
| 4 | `LZ4_compress_default` | incompressible random data, exact-bound dst | [x] |
| 5 | `LZ4_compress_default` | dst tighter than compressBound (limitedOutput), sweep capacities | [x] |
| 6 | `LZ4_compress_fast` | acceleration = 1 | [x] |
| 7 | `LZ4_compress_fast` | acceleration = 0 and negative (→ default) | [x] |
| 8 | `LZ4_compress_fast` | acceleration = 2, 5, 17, 100, 65537, 1<<20 (clamp) | [x] |
| 9 | `LZ4_compress_fast_extState` | caller-provided aligned state, sweep acceleration | [x] |
| 10 | `LZ4_compress_fast_extState_fastReset` | reused state across successive calls | [x] |
| 11 | `LZ4_compress_destSize` | fillOutput: targetDstSize sweep 1..bound, returns consumed srcSize | [x] |
| 12 | `LZ4_compress_destSize_extState` | fillOutput with external state, acceleration sweep | [x] |
| 13 | `LZ4_compressBound` | sweep inputSize incl. 0, 1, LZ4_MAX_INPUT_SIZE, over-max | [x] |
| 14 | `LZ4_compress` / `LZ4_compress_limitedOutput` (obsolete) | legacy wrappers, random sizes | [x] |
| 15 | `LZ4_compress_withState` / `LZ4_compress_limitedOutput_withState` | legacy + external state | [x] |
| 16 | `LZ4_sizeofState` / `LZ4_sizeofStreamState` | constants match | [x] |

### Block API — decompression

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 17 | `LZ4_decompress_safe` | round-trip of every row 1-8 payload, exact dstCapacity | [x] |
| 18 | `LZ4_decompress_safe` | dstCapacity larger than needed | [x] |
| 19 | `LZ4_decompress_safe_partial` | targetOutputSize sweep 0..full, dstCapacity == target | [x] |
| 20 | `LZ4_decompress_safe_partial` | targetOutputSize < dstCapacity (over-provisioned dst) | [x] |
| 21 | `LZ4_decompress_fast` | known originalSize, prefix-free | [x] |
| 22 | `LZ4_decompress_fast_withPrefix64k` | 64KB prefix present before dst | [x] |
| 23 | `LZ4_decompress_safe_withPrefix64k` | 64KB prefix | [x] |
| 24 | `LZ4_decompress_safe_usingDict` | external dict sizes {1, 100, 65535, 65536, 70000} | [x] |
| 25 | `LZ4_decompress_fast_usingDict` | external dict, known size | [x] |
| 26 | `LZ4_decompress_safe_partial_usingDict` | ext dict + partial target sweep | [x] |
| 27 | `LZ4_decompress_safe_forceExtDict` | forced ext-dict path | [x] |
| 28 | `LZ4_decompress_safe_partial_forceExtDict` | forced ext-dict + partial | [x] |
| 29 | `LZ4_uncompress` / `LZ4_uncompress_unknownOutputSize` | legacy decompress wrappers | [x] |
| 30 | `LZ4_decoderRingBufferSize` | sweep maxBlockSize | [x] |

### Block API — streaming / dictionary

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 31 | `LZ4_createStream`+`LZ4_compress_fast_continue` | linked blocks, N random chunks, blockLinked, no dict | [x] |
| 32 | `LZ4_initStream` on caller buffer + `_continue` | aligned buffer, chunk loop | [x] |
| 33 | `LZ4_resetStream_fast` + `_continue` | fast reset between streams | [x] |
| 34 | `LZ4_resetStream` (obsolete) + `_continue` | full reset | [x] |
| 35 | `LZ4_loadDict` + `_continue` | dict sizes {0,1,1000,65535,65536,100000} | [x] |
| 36 | `LZ4_loadDictSlow` + `_continue` | slow-load dict variant, same sizes | [x] |
| 37 | `LZ4_attach_dictionary` + `_continue` | usingDictCtx path, dict stream attached | [x] |
| 38 | `LZ4_attach_dictionary(NULL)` | detach path | [x] |
| 39 | `LZ4_saveDict` | after chunk loop, dictSize {0,100,65536,over-64K} | [x] |
| 40 | `LZ4_compress_forceExtDict` | forced ext-dict compression | [x] |
| 41 | `LZ4_create`/`LZ4_slideInputBuffer`/`LZ4_free` (obsolete) | legacy stream lifecycle | [x] |
| 42 | `LZ4_resetStreamState` (obsolete) + `LZ4_compress_continue` | legacy streaming | [x] |
| 43 | `LZ4_setStreamDecode` + `LZ4_decompress_safe_continue` | ring-buffer decode, chunk loop | [x] |
| 44 | `LZ4_setStreamDecode` + `LZ4_decompress_fast_continue` | ring-buffer fast decode | [x] |
| 45 | `LZ4_createStreamDecode`/`Free` + dict-seeded decode | setStreamDecode with dict | [x] |

### HC API

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 46 | `LZ4_compress_HC` | level 1 (lz4mid), random + compressible data, notLimited | [x] |
| 47 | `LZ4_compress_HC` | level 2 (lz4mid) | [x] |
| 48 | `LZ4_compress_HC` | level 3 (lz4hc, first hc level) | [x] |
| 49 | `LZ4_compress_HC` | level 6 | [x] |
| 50 | `LZ4_compress_HC` | level 9 (default) | [x] |
| 51 | `LZ4_compress_HC` | level 10 (lz4opt / OPT_MIN) | [x] |
| 52 | `LZ4_compress_HC` | level 11 | [x] |
| 53 | `LZ4_compress_HC` | level 12 (MAX / ultra) | [x] |
| 54 | `LZ4_compress_HC` | levels 0, -5, 13, 999 (clamping paths) | [x] |
| 55 | `LZ4_compress_HC` | limitedOutput: dst tighter than bound, level sweep | [x] |
| 56 | `LZ4_compress_HC` | srcSize > 64K at each strategy (mid/hc/opt) | [x] |
| 57 | `LZ4_compress_HC_extStateHC` | external state, level sweep | [x] |
| 58 | `LZ4_compress_HC_extStateHC_fastReset` | reused external state, level sweep | [x] |
| 59 | `LZ4_compress_HC_destSize` | fillOutput, targetDstSize sweep × level sweep | [x] |
| 60 | `LZ4_compress_HC_continue` | streaming HC, chunk loop, level sweep | [x] |
| 61 | `LZ4_compress_HC_continue_destSize` | streaming fillOutput, level sweep | [x] |
| 62 | `LZ4_loadDictHC` + `_continue` | dict sizes {1,1000,65536,100000} × level sweep | [x] |
| 63 | `LZ4_attach_HC_dictionary` + `_continue` | dictCtx path (levels ≥3, mid excluded per lz4hc.c:517) | [x] |
| 64 | `LZ4_saveDictHC` | after HC chunk loop, dictSize sweep | [x] |
| 65 | `LZ4_favorDecompressionSpeed` | favor=1 vs 0, levels 9/10/12 (only ≥10 differs) | [x] |
| 66 | `LZ4_setCompressionLevel` | mid-stream level change, then `_continue` | [x] |
| 67 | `LZ4_initStreamHC` on caller buffer | aligned buffer + `_continue` | [x] |
| 68 | `LZ4_resetStreamHC` / `LZ4_resetStreamHC_fast` | reset variants × level sweep | [x] |
| 69 | `LZ4_resetStreamStateHC` (obsolete) | legacy reset + continue | [x] |
| 70 | `LZ4_createHC`/`LZ4_slideInputBuffer HC`/`LZ4_freeHC` | legacy HC lifecycle | [x] |
| 71 | `LZ4_compressHC` / `LZ4_compressHC_limitedOutput` | obsolete HC wrappers | [x] |
| 72 | `LZ4_compressHC2` / `_limitedOutput` | obsolete, explicit level arg sweep | [x] |
| 73 | `LZ4_compressHC_withStateHC` / `_limitedOutput_withStateHC` | obsolete + state | [x] |
| 74 | `LZ4_compressHC2_withStateHC` / `_limitedOutput_withStateHC` | obsolete + state + level | [x] |
| 75 | `LZ4_compressHC_continue` / `_limitedOutput_continue` | obsolete streaming | [x] |
| 76 | `LZ4_compressHC2_continue` / `_limitedOutput_continue` | obsolete streaming + level | [x] |
| 77 | `LZ4HC_searchExtDict` | exported low-level ext-dict search (via HC ext-dict stream) | [x] |
| 78 | `LZ4_sizeofStateHC` / `LZ4_sizeofStreamStateHC` | constants match | [x] |

### Frame API — one-shot

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 79 | `LZ4F_compressFrame` | prefs == NULL (all defaults) | [x] |
| 80 | `LZ4F_compressFrame` | blockSizeID sweep {0,4,5,6,7} × blockLinked | [x] |
| 81 | `LZ4F_compressFrame` | blockSizeID sweep {0,4,5,6,7} × blockIndependent | [x] |
| 82 | `LZ4F_compressFrame` | contentChecksumFlag = 1 | [x] |
| 83 | `LZ4F_compressFrame` | blockChecksumFlag = 1 | [x] |
| 84 | `LZ4F_compressFrame` | both checksums enabled | [x] |
| 85 | `LZ4F_compressFrame` | contentSize = exact srcSize (header carries size) | [x] |
| 86 | `LZ4F_compressFrame` | dictID nonzero | [x] |
| 87 | `LZ4F_compressFrame` | compressionLevel sweep {-5,0,1,2,3,9,10,12} | [x] |
| 88 | `LZ4F_compressFrame` | favorDecSpeed = 1 × level {9,10,12} | [x] |
| 89 | `LZ4F_compressFrame` | autoFlush = 1 | [x] |
| 90 | `LZ4F_compressFrame` | srcSize 0 / 1 / blockSize-1 / blockSize / blockSize+1 / multi-block | [x] |
| 91 | `LZ4F_compressFrameBound` / `LZ4F_compressBound` | sweep srcSize × prefs (incl. NULL) | [x] |
| 92 | `LZ4F_compressFrame_usingCDict` | CDict from `LZ4F_createCDict`, level sweep, dict sizes | [x] |
| 93 | `LZ4F_createCDict_advanced` | custom-mem CDict (default allocator struct) | [x] |
| 94 | `LZ4F_getVersion` / `LZ4F_compressionLevel_max` / `LZ4F_getBlockSize` | constants + valid IDs | [x] |

### Frame API — low-level streaming compression

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 95 | `compressBegin`+`compressUpdate`+`compressEnd` | defaults, single update | [x] |
| 96 | same | chunked updates: 1 byte at a time | [x] |
| 97 | same | chunked updates: random chunk sizes, blockLinked | [x] |
| 98 | same | chunked updates: random chunk sizes, blockIndependent | [x] |
| 99 | same | blockSizeID sweep {4,5,6,7} × chunked | [x] |
| 100 | same | autoFlush=1 × chunked (forces per-call block emit) | [x] |
| 101 | same | autoFlush=0 × chunk < blockSize (tmpBuff accumulation path) | [x] |
| 102 | same | chunk exactly == blockSize, and blockSize±1 | [x] |
| 103 | same | `compressOptions.stableSrc = 1` | [x] |
| 104 | same | contentSize declared correctly + contentChecksum | [x] |
| 105 | same | blockChecksum enabled × chunked | [x] |
| 106 | same | level sweep {-5,0,1,3,9,10,12} × chunked | [x] |
| 107 | `LZ4F_flush` | explicit flush between updates, partial tmp buffer | [x] |
| 108 | `LZ4F_flush` | flush with empty tmp buffer (no-op path) | [x] |
| 109 | `LZ4F_uncompressedUpdate` | uncompressed-block insertion, sizes ≤/> blockSize | [x] |
| 110 | `LZ4F_uncompressedUpdate` mixed with `compressUpdate` | interleaved compressed + stored blocks | [x] |
| 111 | `LZ4F_compressBegin_usingDict` | dict sizes {1,1000,65536,100000} × blockMode | [x] |
| 112 | `LZ4F_compressBegin_usingDictOnce` | one-shot dict variant | [x] |
| 113 | `LZ4F_compressBegin_usingCDict` | CDict variant × level sweep | [x] |
| 114 | `LZ4F_compressBegin_internal` (exported) | reached via the public begin paths | [x] |
| 115 | `LZ4F_createCompressionContext_advanced` | custom-mem cctx | [x] |
| 116 | cctx reuse | second frame on same cctx after compressEnd | [x] |

### Frame API — decompression

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 117 | `LZ4F_decompress` | one-shot: full src, large dst, all row 79-94 frames | [x] |
| 118 | `LZ4F_decompress` | src fed 1 byte at a time (dStage machine every state) | [x] |
| 119 | `LZ4F_decompress` | src fed in random chunks | [x] |
| 120 | `LZ4F_decompress` | dst 1 byte at a time (output-starved path) | [x] |
| 121 | `LZ4F_decompress` | dst in random small chunks | [x] |
| 122 | `LZ4F_decompress` | `decompressOptions.stableDst = 1` | [x] |
| 123 | `LZ4F_decompress` | `decompressOptions.skipChecksums = 1` on checksummed frame | [x] |
| 124 | `LZ4F_decompress` | blockLinked frames (tmpOut / dict carry-over path) | [x] |
| 125 | `LZ4F_decompress` | blockIndependent frames | [x] |
| 126 | `LZ4F_decompress` | frames with uncompressed (stored) blocks | [x] |
| 127 | `LZ4F_decompress` | skippable frame (magic 0x184D2A5x) skipped correctly | [x] |
| 128 | `LZ4F_decompress` | two concatenated frames on one dctx | [x] |
| 129 | `LZ4F_decompress_usingDict` | dict-compressed frame, dict sizes sweep | [x] |
| 130 | `LZ4F_decompress_usingDict` | chunked src × chunked dst × dict | [x] |
| 131 | `LZ4F_getFrameInfo` | before any data (needs header), all header variants | [x] |
| 132 | `LZ4F_getFrameInfo` | after partial header fed byte-by-byte | [x] |
| 133 | `LZ4F_getFrameInfo` | mid-frame (after some blocks decoded) | [x] |
| 134 | `LZ4F_headerSize` | all header variants incl. contentSize/dictID present | [x] |
| 135 | `LZ4F_resetDecompressionContext` | reset after error, then reuse for a new frame | [x] |
| 136 | `LZ4F_createDecompressionContext_advanced` | custom-mem dctx | [x] |
| 137 | `LZ4F_getErrorName` / `getErrorCode` / `isError` | all 24 enum codes + non-error values | [x] |

### File API

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 138 | `LZ4F_writeOpen`+`write`+`writeClose` | prefs NULL, single write | [x] |
| 139 | same | prefs NULL, many small writes | [x] |
| 140 | same | blockSizeID sweep {0,4,5,6,7} | [x] |
| 141 | same | contentChecksum + blockChecksum enabled | [x] |
| 142 | same | level sweep {0,1,3,9,12} | [x] |
| 143 | same | write sizes {0,1,blockSize-1,blockSize,blockSize+1,large} | [x] |
| 144 | `LZ4F_readOpen`+`read`+`readClose` | read whole file in one call | [x] |
| 145 | same | read 1 byte at a time | [x] |
| 146 | same | read in random chunks | [x] |
| 147 | full file round-trip | write via C then read via Rust and vice-versa (cross) | [x] |
| 148 | file round-trip | each blockSizeID × checksum combo | [x] |

### xxhash (namespaced LZ4_XXH*)

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 149 | `LZ4_XXH32` | one-shot, len sweep {0,1,3,4,7,8,15,16,31,32,33,64,255,1024,4096}, seed 0 | [x] |
| 150 | `LZ4_XXH32` | same len sweep, random nonzero seeds | [x] |
| 151 | `LZ4_XXH64` | one-shot len sweep, seed 0 | [x] |
| 152 | `LZ4_XXH64` | one-shot len sweep, random nonzero seeds | [x] |
| 153 | `LZ4_XXH32_createState`/`reset`/`update`/`digest`/`freeState` | streaming, single update | [x] |
| 154 | `LZ4_XXH32` streaming | update chunking {1,3,15,16,17,31,32,33,random} | [x] |
| 155 | `LZ4_XXH64` streaming | update chunking sweep | [x] |
| 156 | `LZ4_XXH32_copyState` / `LZ4_XXH64_copyState` | copy mid-stream, both continue | [x] |
| 157 | `LZ4_XXH32_canonicalFromHash` / `hashFromCanonical` | round-trip random hashes | [x] |
| 158 | `LZ4_XXH64_canonicalFromHash` / `hashFromCanonical` | round-trip random hashes | [x] |
| 159 | `LZ4_XXH_versionNumber` | constant match | [x] |
| 160 | `LZ4_XXH32_digest` called twice | digest is non-destructive | [x] |

### Cross-implementation interoperability

| # | entry point(s) | configuration | [ ] |
|---|----------------|---------------|-----|
| 161 | C compress → Rust decompress | block API, all row 1-16 configs | [x] |
| 162 | Rust compress → C decompress | block API, all row 1-16 configs | [x] |
| 163 | C `LZ4F_compressFrame` → Rust `LZ4F_decompress` | all frame configs | [x] |
| 164 | Rust `LZ4F_compressFrame` → C `LZ4F_decompress` | all frame configs | [x] |
| 165 | C HC compress → Rust decompress | all level/dict configs | [x] |
| 166 | Rust HC compress → C decompress | all level/dict configs | [x] |

### Feature combinations

`translation/Cargo.toml` declares **no `[features]` table**, so the only build
configuration is the default (no features). Verified mechanically — see
`FEATURES.md` / the feature-combination loop output. Phases B and C therefore
apply to the single existing configuration.

---

## Verification status

All 166 rows above are checked: each has a differential test that drives BOTH
the C `.so` and the Rust `.so` through their exported symbols in that
configuration, over many randomized inputs with fixed seeds, and asserts
byte-for-byte equality of every output and return value.

Row-to-test mapping is by test name (`rowNN_...`) in:

| file | rows | tests |
|------|------|-------|
| `tests/block_api.rs`  | 1-45    | 18 |
| `tests/hc_api.rs`     | 46-78   | 11 |
| `tests/frame_api.rs`  | 79-137  | 17 |
| `tests/file_api.rs`   | 138-148 | 7  |
| `tests/xxhash_api.rs` | 149-160 | 11 |
| `tests/interop.rs`    | 161-166 | 8  |

Feature combinations: the crate declares no `[features]` and no optional
dependencies (`cargo metadata` reports `features: {}`), and the source contains
no `cfg(feature = ...)` gates. The only build configurations are therefore the
default build and `--no-default-features`, which are identical. Both are built
and fully tested by `./verify_features.sh`, which also re-checks `nm -D` symbol
parity for each.
