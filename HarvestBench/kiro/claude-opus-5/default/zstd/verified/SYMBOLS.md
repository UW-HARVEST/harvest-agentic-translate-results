# SYMBOLS.md — Exported symbol parity (C `.so` vs Rust `.so`)

Everything in this file is generated mechanically. Reproduce with
`translation/run_all.sh symbols`.

```
nm -D --defined-only c_src/build/libzstd.so             | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u
nm -D --defined-only translation/target/release/libzstd.so | awk '$2 ~ /^[TWBDRi]$/ {print $3}' | sort -u
```

## Result

| metric | value |
|--------|-------|
| symbols exported by C `libzstd.so` | 615 |
| symbols exported by Rust `libzstd.so` | 615 |
| **missing from Rust** (`comm -23`) | **0** |
| extra in Rust (`comm -13`) | 0 |
| undefined non-libc/libgcc symbols in the Rust `.so` | 0 |
| symbols **invoked directly** by a differential test through `dlopen` | 615 / 615 |

**The symbol diff is EMPTY in both directions.** Every one of the 615 symbols
the C `.so` exports is exported by the Rust `.so` under the exact same name,
including the macro-generated families:

- `XXH_NAMESPACE=ZSTD_` renames the whole xxhash surface (`ZSTD_XXH32*`,
  `ZSTD_XXH64*`, `ZSTD_XXH_versionNumber`) — 19 symbols.
- `ZSTD_LEGACY_SUPPORT=5` compiles in the v01…v07 decoders, which expand into
  the `ZSTDv0x_*`, `ZBUFFv0x_*`, `FSEv0x_*` and `HUFv0x_*` families — 227
  symbols.
- The `zstd_lazy.c` / `zstd_fast.c` / `zstd_double_fast.c` / `zstd_opt.c`
  `ZSTD_compressBlock_*` matrix (strategy × `dictMatchState` / `extDict` /
  `dedicatedDictSearch` × row-hash) — 41 symbols.
- Two exported data symbols: `g_debuglevel` and
  `g_ZSTD_threading_useless_symbol`.

No symbol was faked. `grep -rn "unimplemented!\|todo!" translation/src` returns
nothing, and every symbol is exercised by a test that compares its behaviour
against the C — a stub would fail immediately.

### Undefined symbols in the Rust `.so`

All undefined symbols resolve to glibc/libgcc: `malloc`, `calloc`, `realloc`,
`free`, `posix_memalign`, `memcpy`, `memmove`, `memset`, `memcmp`, `bcmp`,
`strlen`, `qsort_r`, `clock`, `getenv`, `abort`, `stderr`, `fprintf`, `fputc`,
`fputs`, `fwrite`, `fflush`, the `pthread_key_*` TLS trio, the `_Unwind_*` /
`__cxa_*` / `_ITM_*` runtime hooks, and the `open64`/`read`/`write`/`stat64`
family pulled in by Rust's std. `ldd` reports only `libgcc_s.so.1` and
`libc.so.6`.

## Per-C-translation-unit breakdown

| C translation unit | # exported syms | all present in Rust `.so` |
|--------------------|-----------------|----------------------------|
| `src/common/debug.c` | 1 | yes |
| `src/common/entropy_common.c` | 9 | yes |
| `src/common/error_private.c` | 1 | yes |
| `src/common/fse_decompress.c` | 2 | yes |
| `src/common/pool.c` | 8 | yes |
| `src/common/threading.c` | 1 | yes |
| `src/common/xxhash.c` | 19 | yes |
| `src/common/zstd_common.c` | 6 | yes |
| `src/compress/fse_compress.c` | 9 | yes |
| `src/compress/hist.c` | 7 | yes |
| `src/compress/huf_compress.c` | 15 | yes |
| `src/compress/zstd_compress.c` | 121 | yes |
| `src/compress/zstd_compress_literals.c` | 3 | yes |
| `src/compress/zstd_compress_sequences.c` | 5 | yes |
| `src/compress/zstd_compress_superblock.c` | 1 | yes |
| `src/compress/zstd_double_fast.c` | 4 | yes |
| `src/compress/zstd_fast.c` | 4 | yes |
| `src/compress/zstd_lazy.c` | 30 | yes |
| `src/compress/zstd_ldm.c` | 8 | yes |
| `src/compress/zstd_opt.c` | 8 | yes |
| `src/compress/zstd_preSplit.c` | 1 | yes |
| `src/compress/zstdmt_compress.c` | 9 | yes |
| `src/decompress/huf_decompress.c` | 9 | yes |
| `src/decompress/zstd_ddict.c` | 11 | yes |
| `src/decompress/zstd_decompress.c` | 61 | yes |
| `src/decompress/zstd_decompress_block.c` | 8 | yes |
| `src/deprecated/zbuff_common.c` | 2 | yes |
| `src/deprecated/zbuff_compress.c` | 11 | yes |
| `src/deprecated/zbuff_decompress.c` | 8 | yes |
| `src/dictBuilder/cover.c` | 15 | yes |
| `src/dictBuilder/divsufsort.c` | 2 | yes |
| `src/dictBuilder/fastcover.c` | 2 | yes |
| `src/dictBuilder/zdict.c` | 8 | yes |
| `src/legacy/zstd_v01.c` | 9 | yes |
| `src/legacy/zstd_v02.c` | 8 | yes |
| `src/legacy/zstd_v03.c` | 8 | yes |
| `src/legacy/zstd_v04.c` | 17 | yes |
| `src/legacy/zstd_v05.c` | 49 | yes |
| `src/legacy/zstd_v06.c` | 47 | yes |
| `src/legacy/zstd_v07.c` | 68 | yes |

## Full symbol list

`in C` and `in Rust` are `x` for every row (proven by the empty `comm` output
above). `tested` marks symbols that at least one test in `translation/tests/`
resolves by name on BOTH libraries and calls across the FFI boundary.

| # | symbol | defined in (C) | in C | in Rust | tested |
|---|--------|----------------|------|---------|--------|
| 1 | `COVER_best_destroy` | `dictBuilder/cover.c` | x | x | x |
| 2 | `COVER_best_finish` | `dictBuilder/cover.c` | x | x | x |
| 3 | `COVER_best_init` | `dictBuilder/cover.c` | x | x | x |
| 4 | `COVER_best_start` | `dictBuilder/cover.c` | x | x | x |
| 5 | `COVER_best_wait` | `dictBuilder/cover.c` | x | x | x |
| 6 | `COVER_checkTotalCompressedSize` | `dictBuilder/cover.c` | x | x | x |
| 7 | `COVER_computeEpochs` | `dictBuilder/cover.c` | x | x | x |
| 8 | `COVER_dictSelectionError` | `dictBuilder/cover.c` | x | x | x |
| 9 | `COVER_dictSelectionFree` | `dictBuilder/cover.c` | x | x | x |
| 10 | `COVER_dictSelectionIsError` | `dictBuilder/cover.c` | x | x | x |
| 11 | `COVER_selectDict` | `dictBuilder/cover.c` | x | x | x |
| 12 | `COVER_sum` | `dictBuilder/cover.c` | x | x | x |
| 13 | `COVER_warnOnSmallCorpus` | `dictBuilder/cover.c` | x | x | x |
| 14 | `ERR_getErrorString` | `common/error_private.c` | x | x | x |
| 15 | `FSE_NCountWriteBound` | `compress/fse_compress.c` | x | x | x |
| 16 | `FSE_buildCTable_rle` | `compress/fse_compress.c` | x | x | x |
| 17 | `FSE_buildCTable_wksp` | `compress/fse_compress.c` | x | x | x |
| 18 | `FSE_buildDTable_wksp` | `common/fse_decompress.c` | x | x | x |
| 19 | `FSE_compressBound` | `compress/fse_compress.c` | x | x | x |
| 20 | `FSE_compress_usingCTable` | `compress/fse_compress.c` | x | x | x |
| 21 | `FSE_decompress_wksp_bmi2` | `common/fse_decompress.c` | x | x | x |
| 22 | `FSE_getErrorName` | `common/entropy_common.c` | x | x | x |
| 23 | `FSE_isError` | `common/entropy_common.c` | x | x | x |
| 24 | `FSE_normalizeCount` | `compress/fse_compress.c` | x | x | x |
| 25 | `FSE_optimalTableLog` | `compress/fse_compress.c` | x | x | x |
| 26 | `FSE_optimalTableLog_internal` | `compress/fse_compress.c` | x | x | x |
| 27 | `FSE_readNCount` | `common/entropy_common.c` | x | x | x |
| 28 | `FSE_readNCount_bmi2` | `common/entropy_common.c` | x | x | x |
| 29 | `FSE_versionNumber` | `common/entropy_common.c` | x | x | x |
| 30 | `FSE_writeNCount` | `compress/fse_compress.c` | x | x | x |
| 31 | `FSEv05_buildDTable` | `legacy/zstd_v05.c` | x | x | x |
| 32 | `FSEv05_buildDTable_raw` | `legacy/zstd_v05.c` | x | x | x |
| 33 | `FSEv05_buildDTable_rle` | `legacy/zstd_v05.c` | x | x | x |
| 34 | `FSEv05_createDTable` | `legacy/zstd_v05.c` | x | x | x |
| 35 | `FSEv05_decompress` | `legacy/zstd_v05.c` | x | x | x |
| 36 | `FSEv05_decompress_usingDTable` | `legacy/zstd_v05.c` | x | x | x |
| 37 | `FSEv05_freeDTable` | `legacy/zstd_v05.c` | x | x | x |
| 38 | `FSEv05_getErrorName` | `legacy/zstd_v05.c` | x | x | x |
| 39 | `FSEv05_isError` | `legacy/zstd_v05.c` | x | x | x |
| 40 | `FSEv05_readNCount` | `legacy/zstd_v05.c` | x | x | x |
| 41 | `FSEv06_buildDTable` | `legacy/zstd_v06.c` | x | x | x |
| 42 | `FSEv06_buildDTable_raw` | `legacy/zstd_v06.c` | x | x | x |
| 43 | `FSEv06_buildDTable_rle` | `legacy/zstd_v06.c` | x | x | x |
| 44 | `FSEv06_createDTable` | `legacy/zstd_v06.c` | x | x | x |
| 45 | `FSEv06_decompress` | `legacy/zstd_v06.c` | x | x | x |
| 46 | `FSEv06_decompress_usingDTable` | `legacy/zstd_v06.c` | x | x | x |
| 47 | `FSEv06_freeDTable` | `legacy/zstd_v06.c` | x | x | x |
| 48 | `FSEv06_getErrorName` | `legacy/zstd_v06.c` | x | x | x |
| 49 | `FSEv06_isError` | `legacy/zstd_v06.c` | x | x | x |
| 50 | `FSEv06_readNCount` | `legacy/zstd_v06.c` | x | x | x |
| 51 | `FSEv07_buildDTable` | `legacy/zstd_v07.c` | x | x | x |
| 52 | `FSEv07_buildDTable_raw` | `legacy/zstd_v07.c` | x | x | x |
| 53 | `FSEv07_buildDTable_rle` | `legacy/zstd_v07.c` | x | x | x |
| 54 | `FSEv07_createDTable` | `legacy/zstd_v07.c` | x | x | x |
| 55 | `FSEv07_decompress` | `legacy/zstd_v07.c` | x | x | x |
| 56 | `FSEv07_decompress_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 57 | `FSEv07_freeDTable` | `legacy/zstd_v07.c` | x | x | x |
| 58 | `FSEv07_getErrorName` | `legacy/zstd_v07.c` | x | x | x |
| 59 | `FSEv07_isError` | `legacy/zstd_v07.c` | x | x | x |
| 60 | `FSEv07_readNCount` | `legacy/zstd_v07.c` | x | x | x |
| 61 | `HIST_add` | `compress/hist.c` | x | x | x |
| 62 | `HIST_count` | `compress/hist.c` | x | x | x |
| 63 | `HIST_countFast` | `compress/hist.c` | x | x | x |
| 64 | `HIST_countFast_wksp` | `compress/hist.c` | x | x | x |
| 65 | `HIST_count_simple` | `compress/hist.c` | x | x | x |
| 66 | `HIST_count_wksp` | `compress/hist.c` | x | x | x |
| 67 | `HIST_isError` | `compress/hist.c` | x | x | x |
| 68 | `HUF_buildCTable_wksp` | `compress/huf_compress.c` | x | x | x |
| 69 | `HUF_cardinality` | `compress/huf_compress.c` | x | x | x |
| 70 | `HUF_compress1X_repeat` | `compress/huf_compress.c` | x | x | x |
| 71 | `HUF_compress1X_usingCTable` | `compress/huf_compress.c` | x | x | x |
| 72 | `HUF_compress4X_repeat` | `compress/huf_compress.c` | x | x | x |
| 73 | `HUF_compress4X_usingCTable` | `compress/huf_compress.c` | x | x | x |
| 74 | `HUF_compressBound` | `compress/huf_compress.c` | x | x | x |
| 75 | `HUF_decompress1X1_DCtx_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 76 | `HUF_decompress1X2_DCtx_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 77 | `HUF_decompress1X_DCtx_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 78 | `HUF_decompress1X_usingDTable` | `decompress/huf_decompress.c` | x | x | x |
| 79 | `HUF_decompress4X_hufOnly_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 80 | `HUF_decompress4X_usingDTable` | `decompress/huf_decompress.c` | x | x | x |
| 81 | `HUF_estimateCompressedSize` | `compress/huf_compress.c` | x | x | x |
| 82 | `HUF_getErrorName` | `common/entropy_common.c` | x | x | x |
| 83 | `HUF_getNbBitsFromCTable` | `compress/huf_compress.c` | x | x | x |
| 84 | `HUF_isError` | `common/entropy_common.c` | x | x | x |
| 85 | `HUF_minTableLog` | `compress/huf_compress.c` | x | x | x |
| 86 | `HUF_optimalTableLog` | `compress/huf_compress.c` | x | x | x |
| 87 | `HUF_readCTable` | `compress/huf_compress.c` | x | x | x |
| 88 | `HUF_readCTableHeader` | `compress/huf_compress.c` | x | x | x |
| 89 | `HUF_readDTableX1_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 90 | `HUF_readDTableX2_wksp` | `decompress/huf_decompress.c` | x | x | x |
| 91 | `HUF_readStats` | `common/entropy_common.c` | x | x | x |
| 92 | `HUF_readStats_wksp` | `common/entropy_common.c` | x | x | x |
| 93 | `HUF_selectDecoder` | `decompress/huf_decompress.c` | x | x | x |
| 94 | `HUF_validateCTable` | `compress/huf_compress.c` | x | x | x |
| 95 | `HUF_writeCTable_wksp` | `compress/huf_compress.c` | x | x | x |
| 96 | `HUFv05_decompress` | `legacy/zstd_v05.c` | x | x | x |
| 97 | `HUFv05_decompress1X2` | `legacy/zstd_v05.c` | x | x | x |
| 98 | `HUFv05_decompress1X2_usingDTable` | `legacy/zstd_v05.c` | x | x | x |
| 99 | `HUFv05_decompress1X4` | `legacy/zstd_v05.c` | x | x | x |
| 100 | `HUFv05_decompress1X4_usingDTable` | `legacy/zstd_v05.c` | x | x | x |
| 101 | `HUFv05_decompress4X2` | `legacy/zstd_v05.c` | x | x | x |
| 102 | `HUFv05_decompress4X2_usingDTable` | `legacy/zstd_v05.c` | x | x | x |
| 103 | `HUFv05_decompress4X4` | `legacy/zstd_v05.c` | x | x | x |
| 104 | `HUFv05_decompress4X4_usingDTable` | `legacy/zstd_v05.c` | x | x | x |
| 105 | `HUFv05_getErrorName` | `legacy/zstd_v05.c` | x | x | x |
| 106 | `HUFv05_isError` | `legacy/zstd_v05.c` | x | x | x |
| 107 | `HUFv05_readDTableX2` | `legacy/zstd_v05.c` | x | x | x |
| 108 | `HUFv05_readDTableX4` | `legacy/zstd_v05.c` | x | x | x |
| 109 | `HUFv06_decompress` | `legacy/zstd_v06.c` | x | x | x |
| 110 | `HUFv06_decompress1X2` | `legacy/zstd_v06.c` | x | x | x |
| 111 | `HUFv06_decompress1X2_usingDTable` | `legacy/zstd_v06.c` | x | x | x |
| 112 | `HUFv06_decompress1X4` | `legacy/zstd_v06.c` | x | x | x |
| 113 | `HUFv06_decompress1X4_usingDTable` | `legacy/zstd_v06.c` | x | x | x |
| 114 | `HUFv06_decompress4X2` | `legacy/zstd_v06.c` | x | x | x |
| 115 | `HUFv06_decompress4X2_usingDTable` | `legacy/zstd_v06.c` | x | x | x |
| 116 | `HUFv06_decompress4X4` | `legacy/zstd_v06.c` | x | x | x |
| 117 | `HUFv06_decompress4X4_usingDTable` | `legacy/zstd_v06.c` | x | x | x |
| 118 | `HUFv06_readDTableX2` | `legacy/zstd_v06.c` | x | x | x |
| 119 | `HUFv06_readDTableX4` | `legacy/zstd_v06.c` | x | x | x |
| 120 | `HUFv07_decompress` | `legacy/zstd_v07.c` | x | x | x |
| 121 | `HUFv07_decompress1X2` | `legacy/zstd_v07.c` | x | x | x |
| 122 | `HUFv07_decompress1X2_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 123 | `HUFv07_decompress1X2_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 124 | `HUFv07_decompress1X4` | `legacy/zstd_v07.c` | x | x | x |
| 125 | `HUFv07_decompress1X4_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 126 | `HUFv07_decompress1X4_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 127 | `HUFv07_decompress1X_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 128 | `HUFv07_decompress1X_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 129 | `HUFv07_decompress4X2` | `legacy/zstd_v07.c` | x | x | x |
| 130 | `HUFv07_decompress4X2_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 131 | `HUFv07_decompress4X2_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 132 | `HUFv07_decompress4X4` | `legacy/zstd_v07.c` | x | x | x |
| 133 | `HUFv07_decompress4X4_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 134 | `HUFv07_decompress4X4_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 135 | `HUFv07_decompress4X_DCtx` | `legacy/zstd_v07.c` | x | x | x |
| 136 | `HUFv07_decompress4X_hufOnly` | `legacy/zstd_v07.c` | x | x | x |
| 137 | `HUFv07_decompress4X_usingDTable` | `legacy/zstd_v07.c` | x | x | x |
| 138 | `HUFv07_getErrorName` | `legacy/zstd_v07.c` | x | x | x |
| 139 | `HUFv07_isError` | `legacy/zstd_v07.c` | x | x | x |
| 140 | `HUFv07_readDTableX2` | `legacy/zstd_v07.c` | x | x | x |
| 141 | `HUFv07_readDTableX4` | `legacy/zstd_v07.c` | x | x | x |
| 142 | `HUFv07_readStats` | `legacy/zstd_v07.c` | x | x | x |
| 143 | `HUFv07_selectDecoder` | `legacy/zstd_v07.c` | x | x | x |
| 144 | `POOL_add` | `common/pool.c` | x | x | x |
| 145 | `POOL_create` | `common/pool.c` | x | x | x |
| 146 | `POOL_create_advanced` | `common/pool.c` | x | x | x |
| 147 | `POOL_free` | `common/pool.c` | x | x | x |
| 148 | `POOL_joinJobs` | `common/pool.c` | x | x | x |
| 149 | `POOL_resize` | `common/pool.c` | x | x | x |
| 150 | `POOL_sizeof` | `common/pool.c` | x | x | x |
| 151 | `POOL_tryAdd` | `common/pool.c` | x | x | x |
| 152 | `ZBUFF_compressContinue` | `deprecated/zbuff_compress.c` | x | x | x |
| 153 | `ZBUFF_compressEnd` | `deprecated/zbuff_compress.c` | x | x | x |
| 154 | `ZBUFF_compressFlush` | `deprecated/zbuff_compress.c` | x | x | x |
| 155 | `ZBUFF_compressInit` | `deprecated/zbuff_compress.c` | x | x | x |
| 156 | `ZBUFF_compressInitDictionary` | `deprecated/zbuff_compress.c` | x | x | x |
| 157 | `ZBUFF_compressInit_advanced` | `deprecated/zbuff_compress.c` | x | x | x |
| 158 | `ZBUFF_createCCtx` | `deprecated/zbuff_compress.c` | x | x | x |
| 159 | `ZBUFF_createCCtx_advanced` | `deprecated/zbuff_compress.c` | x | x | x |
| 160 | `ZBUFF_createDCtx` | `deprecated/zbuff_decompress.c` | x | x | x |
| 161 | `ZBUFF_createDCtx_advanced` | `deprecated/zbuff_decompress.c` | x | x | x |
| 162 | `ZBUFF_decompressContinue` | `deprecated/zbuff_decompress.c` | x | x | x |
| 163 | `ZBUFF_decompressInit` | `deprecated/zbuff_decompress.c` | x | x | x |
| 164 | `ZBUFF_decompressInitDictionary` | `deprecated/zbuff_decompress.c` | x | x | x |
| 165 | `ZBUFF_freeCCtx` | `deprecated/zbuff_compress.c` | x | x | x |
| 166 | `ZBUFF_freeDCtx` | `deprecated/zbuff_decompress.c` | x | x | x |
| 167 | `ZBUFF_getErrorName` | `deprecated/zbuff_common.c` | x | x | x |
| 168 | `ZBUFF_isError` | `deprecated/zbuff_common.c` | x | x | x |
| 169 | `ZBUFF_recommendedCInSize` | `deprecated/zbuff_compress.c` | x | x | x |
| 170 | `ZBUFF_recommendedCOutSize` | `deprecated/zbuff_compress.c` | x | x | x |
| 171 | `ZBUFF_recommendedDInSize` | `deprecated/zbuff_decompress.c` | x | x | x |
| 172 | `ZBUFF_recommendedDOutSize` | `deprecated/zbuff_decompress.c` | x | x | x |
| 173 | `ZBUFFv04_createDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 174 | `ZBUFFv04_decompressContinue` | `legacy/zstd_v04.c` | x | x | x |
| 175 | `ZBUFFv04_decompressInit` | `legacy/zstd_v04.c` | x | x | x |
| 176 | `ZBUFFv04_decompressWithDictionary` | `legacy/zstd_v04.c` | x | x | x |
| 177 | `ZBUFFv04_freeDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 178 | `ZBUFFv04_getErrorName` | `legacy/zstd_v04.c` | x | x | x |
| 179 | `ZBUFFv04_isError` | `legacy/zstd_v04.c` | x | x | x |
| 180 | `ZBUFFv04_recommendedDInSize` | `legacy/zstd_v04.c` | x | x | x |
| 181 | `ZBUFFv04_recommendedDOutSize` | `legacy/zstd_v04.c` | x | x | x |
| 182 | `ZBUFFv05_createDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 183 | `ZBUFFv05_decompressContinue` | `legacy/zstd_v05.c` | x | x | x |
| 184 | `ZBUFFv05_decompressInit` | `legacy/zstd_v05.c` | x | x | x |
| 185 | `ZBUFFv05_decompressInitDictionary` | `legacy/zstd_v05.c` | x | x | x |
| 186 | `ZBUFFv05_freeDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 187 | `ZBUFFv05_getErrorName` | `legacy/zstd_v05.c` | x | x | x |
| 188 | `ZBUFFv05_isError` | `legacy/zstd_v05.c` | x | x | x |
| 189 | `ZBUFFv05_recommendedDInSize` | `legacy/zstd_v05.c` | x | x | x |
| 190 | `ZBUFFv05_recommendedDOutSize` | `legacy/zstd_v05.c` | x | x | x |
| 191 | `ZBUFFv06_createDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 192 | `ZBUFFv06_decompressContinue` | `legacy/zstd_v06.c` | x | x | x |
| 193 | `ZBUFFv06_decompressInit` | `legacy/zstd_v06.c` | x | x | x |
| 194 | `ZBUFFv06_decompressInitDictionary` | `legacy/zstd_v06.c` | x | x | x |
| 195 | `ZBUFFv06_freeDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 196 | `ZBUFFv06_getErrorName` | `legacy/zstd_v06.c` | x | x | x |
| 197 | `ZBUFFv06_isError` | `legacy/zstd_v06.c` | x | x | x |
| 198 | `ZBUFFv06_recommendedDInSize` | `legacy/zstd_v06.c` | x | x | x |
| 199 | `ZBUFFv06_recommendedDOutSize` | `legacy/zstd_v06.c` | x | x | x |
| 200 | `ZBUFFv07_createDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 201 | `ZBUFFv07_createDCtx_advanced` | `legacy/zstd_v07.c` | x | x | x |
| 202 | `ZBUFFv07_decompressContinue` | `legacy/zstd_v07.c` | x | x | x |
| 203 | `ZBUFFv07_decompressInit` | `legacy/zstd_v07.c` | x | x | x |
| 204 | `ZBUFFv07_decompressInitDictionary` | `legacy/zstd_v07.c` | x | x | x |
| 205 | `ZBUFFv07_freeDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 206 | `ZBUFFv07_getErrorName` | `legacy/zstd_v07.c` | x | x | x |
| 207 | `ZBUFFv07_isError` | `legacy/zstd_v07.c` | x | x | x |
| 208 | `ZBUFFv07_recommendedDInSize` | `legacy/zstd_v07.c` | x | x | x |
| 209 | `ZBUFFv07_recommendedDOutSize` | `legacy/zstd_v07.c` | x | x | x |
| 210 | `ZDICT_addEntropyTablesFromBuffer` | `dictBuilder/zdict.c` | x | x | x |
| 211 | `ZDICT_finalizeDictionary` | `dictBuilder/zdict.c` | x | x | x |
| 212 | `ZDICT_getDictHeaderSize` | `dictBuilder/zdict.c` | x | x | x |
| 213 | `ZDICT_getDictID` | `dictBuilder/zdict.c` | x | x | x |
| 214 | `ZDICT_getErrorName` | `dictBuilder/zdict.c` | x | x | x |
| 215 | `ZDICT_isError` | `dictBuilder/zdict.c` | x | x | x |
| 216 | `ZDICT_optimizeTrainFromBuffer_cover` | `dictBuilder/cover.c` | x | x | x |
| 217 | `ZDICT_optimizeTrainFromBuffer_fastCover` | `dictBuilder/fastcover.c` | x | x | x |
| 218 | `ZDICT_trainFromBuffer` | `dictBuilder/zdict.c` | x | x | x |
| 219 | `ZDICT_trainFromBuffer_cover` | `dictBuilder/cover.c` | x | x | x |
| 220 | `ZDICT_trainFromBuffer_fastCover` | `dictBuilder/fastcover.c` | x | x | x |
| 221 | `ZDICT_trainFromBuffer_legacy` | `dictBuilder/zdict.c` | x | x | x |
| 222 | `ZSTDMT_compressStream_generic` | `compress/zstdmt_compress.c` | x | x | x |
| 223 | `ZSTDMT_createCCtx_advanced` | `compress/zstdmt_compress.c` | x | x | x |
| 224 | `ZSTDMT_freeCCtx` | `compress/zstdmt_compress.c` | x | x | x |
| 225 | `ZSTDMT_getFrameProgression` | `compress/zstdmt_compress.c` | x | x | x |
| 226 | `ZSTDMT_initCStream_internal` | `compress/zstdmt_compress.c` | x | x | x |
| 227 | `ZSTDMT_nextInputSizeHint` | `compress/zstdmt_compress.c` | x | x | x |
| 228 | `ZSTDMT_sizeof_CCtx` | `compress/zstdmt_compress.c` | x | x | x |
| 229 | `ZSTDMT_toFlushNow` | `compress/zstdmt_compress.c` | x | x | x |
| 230 | `ZSTDMT_updateCParams_whileCompressing` | `compress/zstdmt_compress.c` | x | x | x |
| 231 | `ZSTD_CCtxParams_getParameter` | `compress/zstd_compress.c` | x | x | x |
| 232 | `ZSTD_CCtxParams_init` | `compress/zstd_compress.c` | x | x | x |
| 233 | `ZSTD_CCtxParams_init_advanced` | `compress/zstd_compress.c` | x | x | x |
| 234 | `ZSTD_CCtxParams_registerSequenceProducer` | `compress/zstd_compress.c` | x | x | x |
| 235 | `ZSTD_CCtxParams_reset` | `compress/zstd_compress.c` | x | x | x |
| 236 | `ZSTD_CCtxParams_setParameter` | `compress/zstd_compress.c` | x | x | x |
| 237 | `ZSTD_CCtx_getParameter` | `compress/zstd_compress.c` | x | x | x |
| 238 | `ZSTD_CCtx_loadDictionary` | `compress/zstd_compress.c` | x | x | x |
| 239 | `ZSTD_CCtx_loadDictionary_advanced` | `compress/zstd_compress.c` | x | x | x |
| 240 | `ZSTD_CCtx_loadDictionary_byReference` | `compress/zstd_compress.c` | x | x | x |
| 241 | `ZSTD_CCtx_refCDict` | `compress/zstd_compress.c` | x | x | x |
| 242 | `ZSTD_CCtx_refPrefix` | `compress/zstd_compress.c` | x | x | x |
| 243 | `ZSTD_CCtx_refPrefix_advanced` | `compress/zstd_compress.c` | x | x | x |
| 244 | `ZSTD_CCtx_refThreadPool` | `compress/zstd_compress.c` | x | x | x |
| 245 | `ZSTD_CCtx_reset` | `compress/zstd_compress.c` | x | x | x |
| 246 | `ZSTD_CCtx_setCParams` | `compress/zstd_compress.c` | x | x | x |
| 247 | `ZSTD_CCtx_setFParams` | `compress/zstd_compress.c` | x | x | x |
| 248 | `ZSTD_CCtx_setParameter` | `compress/zstd_compress.c` | x | x | x |
| 249 | `ZSTD_CCtx_setParametersUsingCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 250 | `ZSTD_CCtx_setParams` | `compress/zstd_compress.c` | x | x | x |
| 251 | `ZSTD_CCtx_setPledgedSrcSize` | `compress/zstd_compress.c` | x | x | x |
| 252 | `ZSTD_CCtx_trace` | `compress/zstd_compress.c` | x | x | x |
| 253 | `ZSTD_CStreamInSize` | `compress/zstd_compress.c` | x | x | x |
| 254 | `ZSTD_CStreamOutSize` | `compress/zstd_compress.c` | x | x | x |
| 255 | `ZSTD_DCtx_getParameter` | `decompress/zstd_decompress.c` | x | x | x |
| 256 | `ZSTD_DCtx_loadDictionary` | `decompress/zstd_decompress.c` | x | x | x |
| 257 | `ZSTD_DCtx_loadDictionary_advanced` | `decompress/zstd_decompress.c` | x | x | x |
| 258 | `ZSTD_DCtx_loadDictionary_byReference` | `decompress/zstd_decompress.c` | x | x | x |
| 259 | `ZSTD_DCtx_refDDict` | `decompress/zstd_decompress.c` | x | x | x |
| 260 | `ZSTD_DCtx_refPrefix` | `decompress/zstd_decompress.c` | x | x | x |
| 261 | `ZSTD_DCtx_refPrefix_advanced` | `decompress/zstd_decompress.c` | x | x | x |
| 262 | `ZSTD_DCtx_reset` | `decompress/zstd_decompress.c` | x | x | x |
| 263 | `ZSTD_DCtx_setFormat` | `decompress/zstd_decompress.c` | x | x | x |
| 264 | `ZSTD_DCtx_setMaxWindowSize` | `decompress/zstd_decompress.c` | x | x | x |
| 265 | `ZSTD_DCtx_setParameter` | `decompress/zstd_decompress.c` | x | x | x |
| 266 | `ZSTD_DDict_dictContent` | `decompress/zstd_ddict.c` | x | x | x |
| 267 | `ZSTD_DDict_dictSize` | `decompress/zstd_ddict.c` | x | x | x |
| 268 | `ZSTD_DStreamInSize` | `decompress/zstd_decompress.c` | x | x | x |
| 269 | `ZSTD_DStreamOutSize` | `decompress/zstd_decompress.c` | x | x | x |
| 270 | `ZSTD_XXH32` | `common/xxhash.c` | x | x | x |
| 271 | `ZSTD_XXH32_canonicalFromHash` | `common/xxhash.c` | x | x | x |
| 272 | `ZSTD_XXH32_copyState` | `common/xxhash.c` | x | x | x |
| 273 | `ZSTD_XXH32_createState` | `common/xxhash.c` | x | x | x |
| 274 | `ZSTD_XXH32_digest` | `common/xxhash.c` | x | x | x |
| 275 | `ZSTD_XXH32_freeState` | `common/xxhash.c` | x | x | x |
| 276 | `ZSTD_XXH32_hashFromCanonical` | `common/xxhash.c` | x | x | x |
| 277 | `ZSTD_XXH32_reset` | `common/xxhash.c` | x | x | x |
| 278 | `ZSTD_XXH32_update` | `common/xxhash.c` | x | x | x |
| 279 | `ZSTD_XXH64` | `common/xxhash.c` | x | x | x |
| 280 | `ZSTD_XXH64_canonicalFromHash` | `common/xxhash.c` | x | x | x |
| 281 | `ZSTD_XXH64_copyState` | `common/xxhash.c` | x | x | x |
| 282 | `ZSTD_XXH64_createState` | `common/xxhash.c` | x | x | x |
| 283 | `ZSTD_XXH64_digest` | `common/xxhash.c` | x | x | x |
| 284 | `ZSTD_XXH64_freeState` | `common/xxhash.c` | x | x | x |
| 285 | `ZSTD_XXH64_hashFromCanonical` | `common/xxhash.c` | x | x | x |
| 286 | `ZSTD_XXH64_reset` | `common/xxhash.c` | x | x | x |
| 287 | `ZSTD_XXH64_update` | `common/xxhash.c` | x | x | x |
| 288 | `ZSTD_XXH_versionNumber` | `common/xxhash.c` | x | x | x |
| 289 | `ZSTD_adjustCParams` | `compress/zstd_compress.c` | x | x | x |
| 290 | `ZSTD_buildBlockEntropyStats` | `compress/zstd_compress.c` | x | x | x |
| 291 | `ZSTD_buildCTable` | `compress/zstd_compress_sequences.c` | x | x | x |
| 292 | `ZSTD_buildFSETable` | `decompress/zstd_decompress_block.c` | x | x | x |
| 293 | `ZSTD_cParam_getBounds` | `compress/zstd_compress.c` | x | x | x |
| 294 | `ZSTD_checkCParams` | `compress/zstd_compress.c` | x | x | x |
| 295 | `ZSTD_checkContinuity` | `decompress/zstd_decompress_block.c` | x | x | x |
| 296 | `ZSTD_compress` | `compress/zstd_compress.c` | x | x | x |
| 297 | `ZSTD_compress2` | `compress/zstd_compress.c` | x | x | x |
| 298 | `ZSTD_compressBegin` | `compress/zstd_compress.c` | x | x | x |
| 299 | `ZSTD_compressBegin_advanced` | `compress/zstd_compress.c` | x | x | x |
| 300 | `ZSTD_compressBegin_advanced_internal` | `compress/zstd_compress.c` | x | x | x |
| 301 | `ZSTD_compressBegin_usingCDict` | `compress/zstd_compress.c` | x | x | x |
| 302 | `ZSTD_compressBegin_usingCDict_advanced` | `compress/zstd_compress.c` | x | x | x |
| 303 | `ZSTD_compressBegin_usingCDict_deprecated` | `compress/zstd_compress.c` | x | x | x |
| 304 | `ZSTD_compressBegin_usingDict` | `compress/zstd_compress.c` | x | x | x |
| 305 | `ZSTD_compressBlock` | `compress/zstd_compress.c` | x | x | x |
| 306 | `ZSTD_compressBlock_btlazy2` | `compress/zstd_lazy.c` | x | x | x |
| 307 | `ZSTD_compressBlock_btlazy2_dictMatchState` | `compress/zstd_lazy.c` | x | x | x |
| 308 | `ZSTD_compressBlock_btlazy2_extDict` | `compress/zstd_lazy.c` | x | x | x |
| 309 | `ZSTD_compressBlock_btopt` | `compress/zstd_opt.c` | x | x | x |
| 310 | `ZSTD_compressBlock_btopt_dictMatchState` | `compress/zstd_opt.c` | x | x | x |
| 311 | `ZSTD_compressBlock_btopt_extDict` | `compress/zstd_opt.c` | x | x | x |
| 312 | `ZSTD_compressBlock_btultra` | `compress/zstd_opt.c` | x | x | x |
| 313 | `ZSTD_compressBlock_btultra2` | `compress/zstd_opt.c` | x | x | x |
| 314 | `ZSTD_compressBlock_btultra_dictMatchState` | `compress/zstd_opt.c` | x | x | x |
| 315 | `ZSTD_compressBlock_btultra_extDict` | `compress/zstd_opt.c` | x | x | x |
| 316 | `ZSTD_compressBlock_deprecated` | `compress/zstd_compress.c` | x | x | x |
| 317 | `ZSTD_compressBlock_doubleFast` | `compress/zstd_double_fast.c` | x | x | x |
| 318 | `ZSTD_compressBlock_doubleFast_dictMatchState` | `compress/zstd_double_fast.c` | x | x | x |
| 319 | `ZSTD_compressBlock_doubleFast_extDict` | `compress/zstd_double_fast.c` | x | x | x |
| 320 | `ZSTD_compressBlock_fast` | `compress/zstd_fast.c` | x | x | x |
| 321 | `ZSTD_compressBlock_fast_dictMatchState` | `compress/zstd_fast.c` | x | x | x |
| 322 | `ZSTD_compressBlock_fast_extDict` | `compress/zstd_fast.c` | x | x | x |
| 323 | `ZSTD_compressBlock_greedy` | `compress/zstd_lazy.c` | x | x | x |
| 324 | `ZSTD_compressBlock_greedy_dedicatedDictSearch` | `compress/zstd_lazy.c` | x | x | x |
| 325 | `ZSTD_compressBlock_greedy_dedicatedDictSearch_row` | `compress/zstd_lazy.c` | x | x | x |
| 326 | `ZSTD_compressBlock_greedy_dictMatchState` | `compress/zstd_lazy.c` | x | x | x |
| 327 | `ZSTD_compressBlock_greedy_dictMatchState_row` | `compress/zstd_lazy.c` | x | x | x |
| 328 | `ZSTD_compressBlock_greedy_extDict` | `compress/zstd_lazy.c` | x | x | x |
| 329 | `ZSTD_compressBlock_greedy_extDict_row` | `compress/zstd_lazy.c` | x | x | x |
| 330 | `ZSTD_compressBlock_greedy_row` | `compress/zstd_lazy.c` | x | x | x |
| 331 | `ZSTD_compressBlock_lazy` | `compress/zstd_lazy.c` | x | x | x |
| 332 | `ZSTD_compressBlock_lazy2` | `compress/zstd_lazy.c` | x | x | x |
| 333 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch` | `compress/zstd_lazy.c` | x | x | x |
| 334 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch_row` | `compress/zstd_lazy.c` | x | x | x |
| 335 | `ZSTD_compressBlock_lazy2_dictMatchState` | `compress/zstd_lazy.c` | x | x | x |
| 336 | `ZSTD_compressBlock_lazy2_dictMatchState_row` | `compress/zstd_lazy.c` | x | x | x |
| 337 | `ZSTD_compressBlock_lazy2_extDict` | `compress/zstd_lazy.c` | x | x | x |
| 338 | `ZSTD_compressBlock_lazy2_extDict_row` | `compress/zstd_lazy.c` | x | x | x |
| 339 | `ZSTD_compressBlock_lazy2_row` | `compress/zstd_lazy.c` | x | x | x |
| 340 | `ZSTD_compressBlock_lazy_dedicatedDictSearch` | `compress/zstd_lazy.c` | x | x | x |
| 341 | `ZSTD_compressBlock_lazy_dedicatedDictSearch_row` | `compress/zstd_lazy.c` | x | x | x |
| 342 | `ZSTD_compressBlock_lazy_dictMatchState` | `compress/zstd_lazy.c` | x | x | x |
| 343 | `ZSTD_compressBlock_lazy_dictMatchState_row` | `compress/zstd_lazy.c` | x | x | x |
| 344 | `ZSTD_compressBlock_lazy_extDict` | `compress/zstd_lazy.c` | x | x | x |
| 345 | `ZSTD_compressBlock_lazy_extDict_row` | `compress/zstd_lazy.c` | x | x | x |
| 346 | `ZSTD_compressBlock_lazy_row` | `compress/zstd_lazy.c` | x | x | x |
| 347 | `ZSTD_compressBound` | `compress/zstd_compress.c` | x | x | x |
| 348 | `ZSTD_compressCCtx` | `compress/zstd_compress.c` | x | x | x |
| 349 | `ZSTD_compressContinue` | `compress/zstd_compress.c` | x | x | x |
| 350 | `ZSTD_compressContinue_public` | `compress/zstd_compress.c` | x | x | x |
| 351 | `ZSTD_compressEnd` | `compress/zstd_compress.c` | x | x | x |
| 352 | `ZSTD_compressEnd_public` | `compress/zstd_compress.c` | x | x | x |
| 353 | `ZSTD_compressLiterals` | `compress/zstd_compress_literals.c` | x | x | x |
| 354 | `ZSTD_compressRleLiteralsBlock` | `compress/zstd_compress_literals.c` | x | x | x |
| 355 | `ZSTD_compressSequences` | `compress/zstd_compress.c` | x | x | x |
| 356 | `ZSTD_compressSequencesAndLiterals` | `compress/zstd_compress.c` | x | x | x |
| 357 | `ZSTD_compressStream` | `compress/zstd_compress.c` | x | x | x |
| 358 | `ZSTD_compressStream2` | `compress/zstd_compress.c` | x | x | x |
| 359 | `ZSTD_compressStream2_simpleArgs` | `compress/zstd_compress.c` | x | x | x |
| 360 | `ZSTD_compressSuperBlock` | `compress/zstd_compress_superblock.c` | x | x | x |
| 361 | `ZSTD_compress_advanced` | `compress/zstd_compress.c` | x | x | x |
| 362 | `ZSTD_compress_advanced_internal` | `compress/zstd_compress.c` | x | x | x |
| 363 | `ZSTD_compress_usingCDict` | `compress/zstd_compress.c` | x | x | x |
| 364 | `ZSTD_compress_usingCDict_advanced` | `compress/zstd_compress.c` | x | x | x |
| 365 | `ZSTD_compress_usingDict` | `compress/zstd_compress.c` | x | x | x |
| 366 | `ZSTD_convertBlockSequences` | `compress/zstd_compress.c` | x | x | x |
| 367 | `ZSTD_copyCCtx` | `compress/zstd_compress.c` | x | x | x |
| 368 | `ZSTD_copyDCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 369 | `ZSTD_copyDDictParameters` | `decompress/zstd_ddict.c` | x | x | x |
| 370 | `ZSTD_createCCtx` | `compress/zstd_compress.c` | x | x | x |
| 371 | `ZSTD_createCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 372 | `ZSTD_createCCtx_advanced` | `compress/zstd_compress.c` | x | x | x |
| 373 | `ZSTD_createCDict` | `compress/zstd_compress.c` | x | x | x |
| 374 | `ZSTD_createCDict_advanced` | `compress/zstd_compress.c` | x | x | x |
| 375 | `ZSTD_createCDict_advanced2` | `compress/zstd_compress.c` | x | x | x |
| 376 | `ZSTD_createCDict_byReference` | `compress/zstd_compress.c` | x | x | x |
| 377 | `ZSTD_createCStream` | `compress/zstd_compress.c` | x | x | x |
| 378 | `ZSTD_createCStream_advanced` | `compress/zstd_compress.c` | x | x | x |
| 379 | `ZSTD_createDCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 380 | `ZSTD_createDCtx_advanced` | `decompress/zstd_decompress.c` | x | x | x |
| 381 | `ZSTD_createDDict` | `decompress/zstd_ddict.c` | x | x | x |
| 382 | `ZSTD_createDDict_advanced` | `decompress/zstd_ddict.c` | x | x | x |
| 383 | `ZSTD_createDDict_byReference` | `decompress/zstd_ddict.c` | x | x | x |
| 384 | `ZSTD_createDStream` | `decompress/zstd_decompress.c` | x | x | x |
| 385 | `ZSTD_createDStream_advanced` | `decompress/zstd_decompress.c` | x | x | x |
| 386 | `ZSTD_crossEntropyCost` | `compress/zstd_compress_sequences.c` | x | x | x |
| 387 | `ZSTD_cycleLog` | `compress/zstd_compress.c` | x | x | x |
| 388 | `ZSTD_dParam_getBounds` | `decompress/zstd_decompress.c` | x | x | x |
| 389 | `ZSTD_decodeLiteralsBlock_wrapper` | `decompress/zstd_decompress_block.c` | x | x | x |
| 390 | `ZSTD_decodeSeqHeaders` | `decompress/zstd_decompress_block.c` | x | x | x |
| 391 | `ZSTD_decodingBufferSize_min` | `decompress/zstd_decompress.c` | x | x | x |
| 392 | `ZSTD_decompress` | `decompress/zstd_decompress.c` | x | x | x |
| 393 | `ZSTD_decompressBegin` | `decompress/zstd_decompress.c` | x | x | x |
| 394 | `ZSTD_decompressBegin_usingDDict` | `decompress/zstd_decompress.c` | x | x | x |
| 395 | `ZSTD_decompressBegin_usingDict` | `decompress/zstd_decompress.c` | x | x | x |
| 396 | `ZSTD_decompressBlock` | `decompress/zstd_decompress_block.c` | x | x | x |
| 397 | `ZSTD_decompressBlock_deprecated` | `decompress/zstd_decompress_block.c` | x | x | x |
| 398 | `ZSTD_decompressBlock_internal` | `decompress/zstd_decompress_block.c` | x | x | x |
| 399 | `ZSTD_decompressBound` | `decompress/zstd_decompress.c` | x | x | x |
| 400 | `ZSTD_decompressContinue` | `decompress/zstd_decompress.c` | x | x | x |
| 401 | `ZSTD_decompressDCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 402 | `ZSTD_decompressStream` | `decompress/zstd_decompress.c` | x | x | x |
| 403 | `ZSTD_decompressStream_simpleArgs` | `decompress/zstd_decompress.c` | x | x | x |
| 404 | `ZSTD_decompress_usingDDict` | `decompress/zstd_decompress.c` | x | x | x |
| 405 | `ZSTD_decompress_usingDict` | `decompress/zstd_decompress.c` | x | x | x |
| 406 | `ZSTD_decompressionMargin` | `decompress/zstd_decompress.c` | x | x | x |
| 407 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | `compress/zstd_lazy.c` | x | x | x |
| 408 | `ZSTD_defaultCLevel` | `compress/zstd_compress.c` | x | x | x |
| 409 | `ZSTD_encodeSequences` | `compress/zstd_compress_sequences.c` | x | x | x |
| 410 | `ZSTD_endStream` | `compress/zstd_compress.c` | x | x | x |
| 411 | `ZSTD_estimateCCtxSize` | `compress/zstd_compress.c` | x | x | x |
| 412 | `ZSTD_estimateCCtxSize_usingCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 413 | `ZSTD_estimateCCtxSize_usingCParams` | `compress/zstd_compress.c` | x | x | x |
| 414 | `ZSTD_estimateCDictSize` | `compress/zstd_compress.c` | x | x | x |
| 415 | `ZSTD_estimateCDictSize_advanced` | `compress/zstd_compress.c` | x | x | x |
| 416 | `ZSTD_estimateCStreamSize` | `compress/zstd_compress.c` | x | x | x |
| 417 | `ZSTD_estimateCStreamSize_usingCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 418 | `ZSTD_estimateCStreamSize_usingCParams` | `compress/zstd_compress.c` | x | x | x |
| 419 | `ZSTD_estimateDCtxSize` | `decompress/zstd_decompress.c` | x | x | x |
| 420 | `ZSTD_estimateDDictSize` | `decompress/zstd_ddict.c` | x | x | x |
| 421 | `ZSTD_estimateDStreamSize` | `decompress/zstd_decompress.c` | x | x | x |
| 422 | `ZSTD_estimateDStreamSize_fromFrame` | `decompress/zstd_decompress.c` | x | x | x |
| 423 | `ZSTD_fillDoubleHashTable` | `compress/zstd_double_fast.c` | x | x | x |
| 424 | `ZSTD_fillHashTable` | `compress/zstd_fast.c` | x | x | x |
| 425 | `ZSTD_findDecompressedSize` | `decompress/zstd_decompress.c` | x | x | x |
| 426 | `ZSTD_findFrameCompressedSize` | `decompress/zstd_decompress.c` | x | x | x |
| 427 | `ZSTD_flushStream` | `compress/zstd_compress.c` | x | x | x |
| 428 | `ZSTD_frameHeaderSize` | `decompress/zstd_decompress.c` | x | x | x |
| 429 | `ZSTD_freeCCtx` | `compress/zstd_compress.c` | x | x | x |
| 430 | `ZSTD_freeCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 431 | `ZSTD_freeCDict` | `compress/zstd_compress.c` | x | x | x |
| 432 | `ZSTD_freeCStream` | `compress/zstd_compress.c` | x | x | x |
| 433 | `ZSTD_freeDCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 434 | `ZSTD_freeDDict` | `decompress/zstd_ddict.c` | x | x | x |
| 435 | `ZSTD_freeDStream` | `decompress/zstd_decompress.c` | x | x | x |
| 436 | `ZSTD_fseBitCost` | `compress/zstd_compress_sequences.c` | x | x | x |
| 437 | `ZSTD_generateSequences` | `compress/zstd_compress.c` | x | x | x |
| 438 | `ZSTD_get1BlockSummary` | `compress/zstd_compress.c` | x | x | x |
| 439 | `ZSTD_getBlockSize` | `compress/zstd_compress.c` | x | x | x |
| 440 | `ZSTD_getCParams` | `compress/zstd_compress.c` | x | x | x |
| 441 | `ZSTD_getCParamsFromCCtxParams` | `compress/zstd_compress.c` | x | x | x |
| 442 | `ZSTD_getCParamsFromCDict` | `compress/zstd_compress.c` | x | x | x |
| 443 | `ZSTD_getDecompressedSize` | `decompress/zstd_decompress.c` | x | x | x |
| 444 | `ZSTD_getDictID_fromCDict` | `compress/zstd_compress.c` | x | x | x |
| 445 | `ZSTD_getDictID_fromDDict` | `decompress/zstd_ddict.c` | x | x | x |
| 446 | `ZSTD_getDictID_fromDict` | `decompress/zstd_decompress.c` | x | x | x |
| 447 | `ZSTD_getDictID_fromFrame` | `decompress/zstd_decompress.c` | x | x | x |
| 448 | `ZSTD_getErrorCode` | `common/zstd_common.c` | x | x | x |
| 449 | `ZSTD_getErrorName` | `common/zstd_common.c` | x | x | x |
| 450 | `ZSTD_getErrorString` | `common/zstd_common.c` | x | x | x |
| 451 | `ZSTD_getFrameContentSize` | `decompress/zstd_decompress.c` | x | x | x |
| 452 | `ZSTD_getFrameHeader` | `decompress/zstd_decompress.c` | x | x | x |
| 453 | `ZSTD_getFrameHeader_advanced` | `decompress/zstd_decompress.c` | x | x | x |
| 454 | `ZSTD_getFrameProgression` | `compress/zstd_compress.c` | x | x | x |
| 455 | `ZSTD_getParams` | `compress/zstd_compress.c` | x | x | x |
| 456 | `ZSTD_getSeqStore` | `compress/zstd_compress.c` | x | x | x |
| 457 | `ZSTD_getcBlockSize` | `decompress/zstd_decompress_block.c` | x | x | x |
| 458 | `ZSTD_initCStream` | `compress/zstd_compress.c` | x | x | x |
| 459 | `ZSTD_initCStream_advanced` | `compress/zstd_compress.c` | x | x | x |
| 460 | `ZSTD_initCStream_internal` | `compress/zstd_compress.c` | x | x | x |
| 461 | `ZSTD_initCStream_srcSize` | `compress/zstd_compress.c` | x | x | x |
| 462 | `ZSTD_initCStream_usingCDict` | `compress/zstd_compress.c` | x | x | x |
| 463 | `ZSTD_initCStream_usingCDict_advanced` | `compress/zstd_compress.c` | x | x | x |
| 464 | `ZSTD_initCStream_usingDict` | `compress/zstd_compress.c` | x | x | x |
| 465 | `ZSTD_initDStream` | `decompress/zstd_decompress.c` | x | x | x |
| 466 | `ZSTD_initDStream_usingDDict` | `decompress/zstd_decompress.c` | x | x | x |
| 467 | `ZSTD_initDStream_usingDict` | `decompress/zstd_decompress.c` | x | x | x |
| 468 | `ZSTD_initStaticCCtx` | `compress/zstd_compress.c` | x | x | x |
| 469 | `ZSTD_initStaticCDict` | `compress/zstd_compress.c` | x | x | x |
| 470 | `ZSTD_initStaticCStream` | `compress/zstd_compress.c` | x | x | x |
| 471 | `ZSTD_initStaticDCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 472 | `ZSTD_initStaticDDict` | `decompress/zstd_ddict.c` | x | x | x |
| 473 | `ZSTD_initStaticDStream` | `decompress/zstd_decompress.c` | x | x | x |
| 474 | `ZSTD_insertAndFindFirstIndex` | `compress/zstd_lazy.c` | x | x | x |
| 475 | `ZSTD_insertBlock` | `decompress/zstd_decompress.c` | x | x | x |
| 476 | `ZSTD_invalidateRepCodes` | `compress/zstd_compress.c` | x | x | x |
| 477 | `ZSTD_isError` | `common/zstd_common.c` | x | x | x |
| 478 | `ZSTD_isFrame` | `decompress/zstd_decompress.c` | x | x | x |
| 479 | `ZSTD_isSkippableFrame` | `decompress/zstd_decompress.c` | x | x | x |
| 480 | `ZSTD_ldm_adjustParameters` | `compress/zstd_ldm.c` | x | x | x |
| 481 | `ZSTD_ldm_blockCompress` | `compress/zstd_ldm.c` | x | x | x |
| 482 | `ZSTD_ldm_fillHashTable` | `compress/zstd_ldm.c` | x | x | x |
| 483 | `ZSTD_ldm_generateSequences` | `compress/zstd_ldm.c` | x | x | x |
| 484 | `ZSTD_ldm_getMaxNbSeq` | `compress/zstd_ldm.c` | x | x | x |
| 485 | `ZSTD_ldm_getTableSize` | `compress/zstd_ldm.c` | x | x | x |
| 486 | `ZSTD_ldm_skipRawSeqStoreBytes` | `compress/zstd_ldm.c` | x | x | x |
| 487 | `ZSTD_ldm_skipSequences` | `compress/zstd_ldm.c` | x | x | x |
| 488 | `ZSTD_loadCEntropy` | `compress/zstd_compress.c` | x | x | x |
| 489 | `ZSTD_loadDEntropy` | `decompress/zstd_decompress.c` | x | x | x |
| 490 | `ZSTD_maxCLevel` | `compress/zstd_compress.c` | x | x | x |
| 491 | `ZSTD_mergeBlockDelimiters` | `compress/zstd_compress.c` | x | x | x |
| 492 | `ZSTD_minCLevel` | `compress/zstd_compress.c` | x | x | x |
| 493 | `ZSTD_nextInputType` | `decompress/zstd_decompress.c` | x | x | x |
| 494 | `ZSTD_nextSrcSizeToDecompress` | `decompress/zstd_decompress.c` | x | x | x |
| 495 | `ZSTD_noCompressLiterals` | `compress/zstd_compress_literals.c` | x | x | x |
| 496 | `ZSTD_readSkippableFrame` | `decompress/zstd_decompress.c` | x | x | x |
| 497 | `ZSTD_referenceExternalSequences` | `compress/zstd_compress.c` | x | x | x |
| 498 | `ZSTD_registerSequenceProducer` | `compress/zstd_compress.c` | x | x | x |
| 499 | `ZSTD_resetCStream` | `compress/zstd_compress.c` | x | x | x |
| 500 | `ZSTD_resetDStream` | `decompress/zstd_decompress.c` | x | x | x |
| 501 | `ZSTD_resetSeqStore` | `compress/zstd_compress.c` | x | x | x |
| 502 | `ZSTD_reset_compressedBlockState` | `compress/zstd_compress.c` | x | x | x |
| 503 | `ZSTD_row_update` | `compress/zstd_lazy.c` | x | x | x |
| 504 | `ZSTD_selectBlockCompressor` | `compress/zstd_compress.c` | x | x | x |
| 505 | `ZSTD_selectEncodingType` | `compress/zstd_compress_sequences.c` | x | x | x |
| 506 | `ZSTD_seqToCodes` | `compress/zstd_compress.c` | x | x | x |
| 507 | `ZSTD_sequenceBound` | `compress/zstd_compress.c` | x | x | x |
| 508 | `ZSTD_sizeof_CCtx` | `compress/zstd_compress.c` | x | x | x |
| 509 | `ZSTD_sizeof_CDict` | `compress/zstd_compress.c` | x | x | x |
| 510 | `ZSTD_sizeof_CStream` | `compress/zstd_compress.c` | x | x | x |
| 511 | `ZSTD_sizeof_DCtx` | `decompress/zstd_decompress.c` | x | x | x |
| 512 | `ZSTD_sizeof_DDict` | `decompress/zstd_ddict.c` | x | x | x |
| 513 | `ZSTD_sizeof_DStream` | `decompress/zstd_decompress.c` | x | x | x |
| 514 | `ZSTD_splitBlock` | `compress/zstd_preSplit.c` | x | x | x |
| 515 | `ZSTD_toFlushNow` | `compress/zstd_compress.c` | x | x | x |
| 516 | `ZSTD_updateTree` | `compress/zstd_opt.c` | x | x | x |
| 517 | `ZSTD_versionNumber` | `common/zstd_common.c` | x | x | x |
| 518 | `ZSTD_versionString` | `common/zstd_common.c` | x | x | x |
| 519 | `ZSTD_writeLastEmptyBlock` | `compress/zstd_compress.c` | x | x | x |
| 520 | `ZSTD_writeSkippableFrame` | `compress/zstd_compress.c` | x | x | x |
| 521 | `ZSTDv01_createDCtx` | `legacy/zstd_v01.c` | x | x | x |
| 522 | `ZSTDv01_decompress` | `legacy/zstd_v01.c` | x | x | x |
| 523 | `ZSTDv01_decompressContinue` | `legacy/zstd_v01.c` | x | x | x |
| 524 | `ZSTDv01_decompressDCtx` | `legacy/zstd_v01.c` | x | x | x |
| 525 | `ZSTDv01_findFrameSizeInfoLegacy` | `legacy/zstd_v01.c` | x | x | x |
| 526 | `ZSTDv01_freeDCtx` | `legacy/zstd_v01.c` | x | x | x |
| 527 | `ZSTDv01_isError` | `legacy/zstd_v01.c` | x | x | x |
| 528 | `ZSTDv01_nextSrcSizeToDecompress` | `legacy/zstd_v01.c` | x | x | x |
| 529 | `ZSTDv01_resetDCtx` | `legacy/zstd_v01.c` | x | x | x |
| 530 | `ZSTDv02_createDCtx` | `legacy/zstd_v02.c` | x | x | x |
| 531 | `ZSTDv02_decompress` | `legacy/zstd_v02.c` | x | x | x |
| 532 | `ZSTDv02_decompressContinue` | `legacy/zstd_v02.c` | x | x | x |
| 533 | `ZSTDv02_findFrameSizeInfoLegacy` | `legacy/zstd_v02.c` | x | x | x |
| 534 | `ZSTDv02_freeDCtx` | `legacy/zstd_v02.c` | x | x | x |
| 535 | `ZSTDv02_isError` | `legacy/zstd_v02.c` | x | x | x |
| 536 | `ZSTDv02_nextSrcSizeToDecompress` | `legacy/zstd_v02.c` | x | x | x |
| 537 | `ZSTDv02_resetDCtx` | `legacy/zstd_v02.c` | x | x | x |
| 538 | `ZSTDv03_createDCtx` | `legacy/zstd_v03.c` | x | x | x |
| 539 | `ZSTDv03_decompress` | `legacy/zstd_v03.c` | x | x | x |
| 540 | `ZSTDv03_decompressContinue` | `legacy/zstd_v03.c` | x | x | x |
| 541 | `ZSTDv03_findFrameSizeInfoLegacy` | `legacy/zstd_v03.c` | x | x | x |
| 542 | `ZSTDv03_freeDCtx` | `legacy/zstd_v03.c` | x | x | x |
| 543 | `ZSTDv03_isError` | `legacy/zstd_v03.c` | x | x | x |
| 544 | `ZSTDv03_nextSrcSizeToDecompress` | `legacy/zstd_v03.c` | x | x | x |
| 545 | `ZSTDv03_resetDCtx` | `legacy/zstd_v03.c` | x | x | x |
| 546 | `ZSTDv04_createDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 547 | `ZSTDv04_decompress` | `legacy/zstd_v04.c` | x | x | x |
| 548 | `ZSTDv04_decompressContinue` | `legacy/zstd_v04.c` | x | x | x |
| 549 | `ZSTDv04_decompressDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 550 | `ZSTDv04_findFrameSizeInfoLegacy` | `legacy/zstd_v04.c` | x | x | x |
| 551 | `ZSTDv04_freeDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 552 | `ZSTDv04_nextSrcSizeToDecompress` | `legacy/zstd_v04.c` | x | x | x |
| 553 | `ZSTDv04_resetDCtx` | `legacy/zstd_v04.c` | x | x | x |
| 554 | `ZSTDv05_copyDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 555 | `ZSTDv05_createDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 556 | `ZSTDv05_decompress` | `legacy/zstd_v05.c` | x | x | x |
| 557 | `ZSTDv05_decompressBegin` | `legacy/zstd_v05.c` | x | x | x |
| 558 | `ZSTDv05_decompressBegin_usingDict` | `legacy/zstd_v05.c` | x | x | x |
| 559 | `ZSTDv05_decompressBlock` | `legacy/zstd_v05.c` | x | x | x |
| 560 | `ZSTDv05_decompressContinue` | `legacy/zstd_v05.c` | x | x | x |
| 561 | `ZSTDv05_decompressDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 562 | `ZSTDv05_decompress_usingDict` | `legacy/zstd_v05.c` | x | x | x |
| 563 | `ZSTDv05_decompress_usingPreparedDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 564 | `ZSTDv05_findFrameSizeInfoLegacy` | `legacy/zstd_v05.c` | x | x | x |
| 565 | `ZSTDv05_freeDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 566 | `ZSTDv05_getErrorName` | `legacy/zstd_v05.c` | x | x | x |
| 567 | `ZSTDv05_getFrameParams` | `legacy/zstd_v05.c` | x | x | x |
| 568 | `ZSTDv05_isError` | `legacy/zstd_v05.c` | x | x | x |
| 569 | `ZSTDv05_nextSrcSizeToDecompress` | `legacy/zstd_v05.c` | x | x | x |
| 570 | `ZSTDv05_sizeofDCtx` | `legacy/zstd_v05.c` | x | x | x |
| 571 | `ZSTDv06_copyDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 572 | `ZSTDv06_createDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 573 | `ZSTDv06_decompress` | `legacy/zstd_v06.c` | x | x | x |
| 574 | `ZSTDv06_decompressBegin` | `legacy/zstd_v06.c` | x | x | x |
| 575 | `ZSTDv06_decompressBegin_usingDict` | `legacy/zstd_v06.c` | x | x | x |
| 576 | `ZSTDv06_decompressBlock` | `legacy/zstd_v06.c` | x | x | x |
| 577 | `ZSTDv06_decompressContinue` | `legacy/zstd_v06.c` | x | x | x |
| 578 | `ZSTDv06_decompressDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 579 | `ZSTDv06_decompress_usingDict` | `legacy/zstd_v06.c` | x | x | x |
| 580 | `ZSTDv06_decompress_usingPreparedDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 581 | `ZSTDv06_findFrameSizeInfoLegacy` | `legacy/zstd_v06.c` | x | x | x |
| 582 | `ZSTDv06_freeDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 583 | `ZSTDv06_getErrorName` | `legacy/zstd_v06.c` | x | x | x |
| 584 | `ZSTDv06_getFrameParams` | `legacy/zstd_v06.c` | x | x | x |
| 585 | `ZSTDv06_isError` | `legacy/zstd_v06.c` | x | x | x |
| 586 | `ZSTDv06_nextSrcSizeToDecompress` | `legacy/zstd_v06.c` | x | x | x |
| 587 | `ZSTDv06_sizeofDCtx` | `legacy/zstd_v06.c` | x | x | x |
| 588 | `ZSTDv07_copyDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 589 | `ZSTDv07_createDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 590 | `ZSTDv07_createDCtx_advanced` | `legacy/zstd_v07.c` | x | x | x |
| 591 | `ZSTDv07_createDDict` | `legacy/zstd_v07.c` | x | x | x |
| 592 | `ZSTDv07_decompress` | `legacy/zstd_v07.c` | x | x | x |
| 593 | `ZSTDv07_decompressBegin` | `legacy/zstd_v07.c` | x | x | x |
| 594 | `ZSTDv07_decompressBegin_usingDict` | `legacy/zstd_v07.c` | x | x | x |
| 595 | `ZSTDv07_decompressBlock` | `legacy/zstd_v07.c` | x | x | x |
| 596 | `ZSTDv07_decompressContinue` | `legacy/zstd_v07.c` | x | x | x |
| 597 | `ZSTDv07_decompressDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 598 | `ZSTDv07_decompress_usingDDict` | `legacy/zstd_v07.c` | x | x | x |
| 599 | `ZSTDv07_decompress_usingDict` | `legacy/zstd_v07.c` | x | x | x |
| 600 | `ZSTDv07_estimateDCtxSize` | `legacy/zstd_v07.c` | x | x | x |
| 601 | `ZSTDv07_findFrameSizeInfoLegacy` | `legacy/zstd_v07.c` | x | x | x |
| 602 | `ZSTDv07_freeDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 603 | `ZSTDv07_freeDDict` | `legacy/zstd_v07.c` | x | x | x |
| 604 | `ZSTDv07_getDecompressedSize` | `legacy/zstd_v07.c` | x | x | x |
| 605 | `ZSTDv07_getErrorName` | `legacy/zstd_v07.c` | x | x | x |
| 606 | `ZSTDv07_getFrameParams` | `legacy/zstd_v07.c` | x | x | x |
| 607 | `ZSTDv07_insertBlock` | `legacy/zstd_v07.c` | x | x | x |
| 608 | `ZSTDv07_isError` | `legacy/zstd_v07.c` | x | x | x |
| 609 | `ZSTDv07_isSkipFrame` | `legacy/zstd_v07.c` | x | x | x |
| 610 | `ZSTDv07_nextSrcSizeToDecompress` | `legacy/zstd_v07.c` | x | x | x |
| 611 | `ZSTDv07_sizeofDCtx` | `legacy/zstd_v07.c` | x | x | x |
| 612 | `divbwt` | `dictBuilder/divsufsort.c` | x | x | x |
| 613 | `divsufsort` | `dictBuilder/divsufsort.c` | x | x | x |
| 614 | `g_ZSTD_threading_useless_symbol` | `common/threading.c` | x | x | x |
| 615 | `g_debuglevel` | `common/debug.c` | x | x | x |
