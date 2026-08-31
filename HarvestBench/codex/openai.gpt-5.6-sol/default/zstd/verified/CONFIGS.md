# Configuration Surface

Generated mechanically from the full `nm -D` entry-point set. The configuration column maps each name to the input-shape and option axes selected by the C implementation family; detailed shared axes follow the table.

| # | entry point(s) | configuration (options set + input shape) | test |
|---:|----------------|-------------------------------------------|:----:|
| 1 | `COVER_best_destroy` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 2 | `COVER_best_finish` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 3 | `COVER_best_init` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 4 | `COVER_best_start` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 5 | `COVER_best_wait` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 6 | `COVER_checkTotalCompressedSize` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 7 | `COVER_computeEpochs` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 8 | `COVER_dictSelectionError` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 9 | `COVER_dictSelectionFree` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 10 | `COVER_dictSelectionIsError` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 11 | `COVER_selectDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 12 | `COVER_sum` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 13 | `COVER_warnOnSmallCorpus` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 14 | `ERR_getErrorString` | documented baseline plus zero, boundary, and randomized values | [x] |
| 15 | `FSE_NCountWriteBound` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 16 | `FSE_buildCTable_rle` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 17 | `FSE_buildCTable_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 18 | `FSE_buildDTable_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 19 | `FSE_compressBound` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 20 | `FSE_compress_usingCTable` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 21 | `FSE_decompress_wksp_bmi2` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 22 | `FSE_getErrorName` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 23 | `FSE_isError` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 24 | `FSE_normalizeCount` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 25 | `FSE_optimalTableLog` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 26 | `FSE_optimalTableLog_internal` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 27 | `FSE_readNCount` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 28 | `FSE_readNCount_bmi2` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 29 | `FSE_versionNumber` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 30 | `FSE_writeNCount` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 31 | `FSEv05_buildDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 32 | `FSEv05_buildDTable_raw` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 33 | `FSEv05_buildDTable_rle` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 34 | `FSEv05_createDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 35 | `FSEv05_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 36 | `FSEv05_decompress_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 37 | `FSEv05_freeDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 38 | `FSEv05_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 39 | `FSEv05_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 40 | `FSEv05_readNCount` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 41 | `FSEv06_buildDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 42 | `FSEv06_buildDTable_raw` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 43 | `FSEv06_buildDTable_rle` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 44 | `FSEv06_createDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 45 | `FSEv06_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 46 | `FSEv06_decompress_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 47 | `FSEv06_freeDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 48 | `FSEv06_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 49 | `FSEv06_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 50 | `FSEv06_readNCount` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 51 | `FSEv07_buildDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 52 | `FSEv07_buildDTable_raw` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 53 | `FSEv07_buildDTable_rle` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 54 | `FSEv07_createDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 55 | `FSEv07_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 56 | `FSEv07_decompress_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 57 | `FSEv07_freeDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 58 | `FSEv07_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 59 | `FSEv07_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 60 | `FSEv07_readNCount` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 61 | `HIST_add` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 62 | `HIST_count` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 63 | `HIST_countFast` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 64 | `HIST_countFast_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 65 | `HIST_count_simple` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 66 | `HIST_count_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 67 | `HIST_isError` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 68 | `HUF_buildCTable_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 69 | `HUF_cardinality` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 70 | `HUF_compress1X_repeat` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 71 | `HUF_compress1X_usingCTable` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 72 | `HUF_compress4X_repeat` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 73 | `HUF_compress4X_usingCTable` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 74 | `HUF_compressBound` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 75 | `HUF_decompress1X1_DCtx_wksp` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 76 | `HUF_decompress1X2_DCtx_wksp` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 77 | `HUF_decompress1X_DCtx_wksp` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 78 | `HUF_decompress1X_usingDTable` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 79 | `HUF_decompress4X_hufOnly_wksp` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 80 | `HUF_decompress4X_usingDTable` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 81 | `HUF_estimateCompressedSize` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 82 | `HUF_getErrorName` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 83 | `HUF_getNbBitsFromCTable` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 84 | `HUF_isError` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 85 | `HUF_minTableLog` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 86 | `HUF_optimalTableLog` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 87 | `HUF_readCTable` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 88 | `HUF_readCTableHeader` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 89 | `HUF_readDTableX1_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 90 | `HUF_readDTableX2_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 91 | `HUF_readStats` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 92 | `HUF_readStats_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 93 | `HUF_selectDecoder` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 94 | `HUF_validateCTable` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 95 | `HUF_writeCTable_wksp` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 96 | `HUFv05_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 97 | `HUFv05_decompress1X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 98 | `HUFv05_decompress1X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 99 | `HUFv05_decompress1X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 100 | `HUFv05_decompress1X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 101 | `HUFv05_decompress4X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 102 | `HUFv05_decompress4X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 103 | `HUFv05_decompress4X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 104 | `HUFv05_decompress4X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 105 | `HUFv05_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 106 | `HUFv05_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 107 | `HUFv05_readDTableX2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 108 | `HUFv05_readDTableX4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 109 | `HUFv06_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 110 | `HUFv06_decompress1X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 111 | `HUFv06_decompress1X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 112 | `HUFv06_decompress1X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 113 | `HUFv06_decompress1X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 114 | `HUFv06_decompress4X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 115 | `HUFv06_decompress4X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 116 | `HUFv06_decompress4X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 117 | `HUFv06_decompress4X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 118 | `HUFv06_readDTableX2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 119 | `HUFv06_readDTableX4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 120 | `HUFv07_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 121 | `HUFv07_decompress1X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 122 | `HUFv07_decompress1X2_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 123 | `HUFv07_decompress1X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 124 | `HUFv07_decompress1X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 125 | `HUFv07_decompress1X4_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 126 | `HUFv07_decompress1X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 127 | `HUFv07_decompress1X_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 128 | `HUFv07_decompress1X_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 129 | `HUFv07_decompress4X2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 130 | `HUFv07_decompress4X2_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 131 | `HUFv07_decompress4X2_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 132 | `HUFv07_decompress4X4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 133 | `HUFv07_decompress4X4_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 134 | `HUFv07_decompress4X4_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 135 | `HUFv07_decompress4X_DCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 136 | `HUFv07_decompress4X_hufOnly` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 137 | `HUFv07_decompress4X_usingDTable` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 138 | `HUFv07_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 139 | `HUFv07_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 140 | `HUFv07_readDTableX2` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 141 | `HUFv07_readDTableX4` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 142 | `HUFv07_readStats` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 143 | `HUFv07_selectDecoder` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 144 | `POOL_add` | documented baseline plus zero, boundary, and randomized values | [x] |
| 145 | `POOL_create` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 146 | `POOL_create_advanced` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 147 | `POOL_free` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 148 | `POOL_joinJobs` | documented baseline plus zero, boundary, and randomized values | [x] |
| 149 | `POOL_resize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 150 | `POOL_sizeof` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 151 | `POOL_tryAdd` | documented baseline plus zero, boundary, and randomized values | [x] |
| 152 | `ZBUFF_compressContinue` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 153 | `ZBUFF_compressEnd` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 154 | `ZBUFF_compressFlush` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 155 | `ZBUFF_compressInit` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 156 | `ZBUFF_compressInitDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 157 | `ZBUFF_compressInit_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 158 | `ZBUFF_createCCtx` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 159 | `ZBUFF_createCCtx_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 160 | `ZBUFF_createDCtx` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 161 | `ZBUFF_createDCtx_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 162 | `ZBUFF_decompressContinue` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 163 | `ZBUFF_decompressInit` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 164 | `ZBUFF_decompressInitDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 165 | `ZBUFF_freeCCtx` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 166 | `ZBUFF_freeDCtx` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 167 | `ZBUFF_getErrorName` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 168 | `ZBUFF_isError` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 169 | `ZBUFF_recommendedCInSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 170 | `ZBUFF_recommendedCOutSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 171 | `ZBUFF_recommendedDInSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 172 | `ZBUFF_recommendedDOutSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 173 | `ZBUFFv04_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 174 | `ZBUFFv04_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 175 | `ZBUFFv04_decompressInit` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 176 | `ZBUFFv04_decompressWithDictionary` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 177 | `ZBUFFv04_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 178 | `ZBUFFv04_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 179 | `ZBUFFv04_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 180 | `ZBUFFv04_recommendedDInSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 181 | `ZBUFFv04_recommendedDOutSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 182 | `ZBUFFv05_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 183 | `ZBUFFv05_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 184 | `ZBUFFv05_decompressInit` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 185 | `ZBUFFv05_decompressInitDictionary` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 186 | `ZBUFFv05_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 187 | `ZBUFFv05_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 188 | `ZBUFFv05_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 189 | `ZBUFFv05_recommendedDInSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 190 | `ZBUFFv05_recommendedDOutSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 191 | `ZBUFFv06_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 192 | `ZBUFFv06_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 193 | `ZBUFFv06_decompressInit` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 194 | `ZBUFFv06_decompressInitDictionary` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 195 | `ZBUFFv06_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 196 | `ZBUFFv06_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 197 | `ZBUFFv06_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 198 | `ZBUFFv06_recommendedDInSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 199 | `ZBUFFv06_recommendedDOutSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 200 | `ZBUFFv07_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 201 | `ZBUFFv07_createDCtx_advanced` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 202 | `ZBUFFv07_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 203 | `ZBUFFv07_decompressInit` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 204 | `ZBUFFv07_decompressInitDictionary` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 205 | `ZBUFFv07_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 206 | `ZBUFFv07_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 207 | `ZBUFFv07_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 208 | `ZBUFFv07_recommendedDInSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 209 | `ZBUFFv07_recommendedDOutSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 210 | `ZDICT_addEntropyTablesFromBuffer` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 211 | `ZDICT_finalizeDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 212 | `ZDICT_getDictHeaderSize` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 213 | `ZDICT_getDictID` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 214 | `ZDICT_getErrorName` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 215 | `ZDICT_isError` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 216 | `ZDICT_optimizeTrainFromBuffer_cover` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 217 | `ZDICT_optimizeTrainFromBuffer_fastCover` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 218 | `ZDICT_trainFromBuffer` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 219 | `ZDICT_trainFromBuffer_cover` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 220 | `ZDICT_trainFromBuffer_fastCover` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 221 | `ZDICT_trainFromBuffer_legacy` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 222 | `ZSTDMT_compressStream_generic` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 223 | `ZSTDMT_createCCtx_advanced` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 224 | `ZSTDMT_freeCCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 225 | `ZSTDMT_getFrameProgression` | documented baseline plus zero, boundary, and randomized values | [x] |
| 226 | `ZSTDMT_initCStream_internal` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 227 | `ZSTDMT_nextInputSizeHint` | documented baseline plus zero, boundary, and randomized values | [x] |
| 228 | `ZSTDMT_sizeof_CCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 229 | `ZSTDMT_toFlushNow` | documented baseline plus zero, boundary, and randomized values | [x] |
| 230 | `ZSTDMT_updateCParams_whileCompressing` | documented baseline plus zero, boundary, and randomized values | [x] |
| 231 | `ZSTD_CCtxParams_getParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 232 | `ZSTD_CCtxParams_init` | documented baseline plus zero, boundary, and randomized values | [x] |
| 233 | `ZSTD_CCtxParams_init_advanced` | documented baseline plus zero, boundary, and randomized values | [x] |
| 234 | `ZSTD_CCtxParams_registerSequenceProducer` | documented baseline plus zero, boundary, and randomized values | [x] |
| 235 | `ZSTD_CCtxParams_reset` | documented baseline plus zero, boundary, and randomized values | [x] |
| 236 | `ZSTD_CCtxParams_setParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 237 | `ZSTD_CCtx_getParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 238 | `ZSTD_CCtx_loadDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 239 | `ZSTD_CCtx_loadDictionary_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 240 | `ZSTD_CCtx_loadDictionary_byReference` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 241 | `ZSTD_CCtx_refCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 242 | `ZSTD_CCtx_refPrefix` | documented baseline plus zero, boundary, and randomized values | [x] |
| 243 | `ZSTD_CCtx_refPrefix_advanced` | documented baseline plus zero, boundary, and randomized values | [x] |
| 244 | `ZSTD_CCtx_refThreadPool` | documented baseline plus zero, boundary, and randomized values | [x] |
| 245 | `ZSTD_CCtx_reset` | documented baseline plus zero, boundary, and randomized values | [x] |
| 246 | `ZSTD_CCtx_setCParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 247 | `ZSTD_CCtx_setFParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 248 | `ZSTD_CCtx_setParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 249 | `ZSTD_CCtx_setParametersUsingCCtxParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 250 | `ZSTD_CCtx_setParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 251 | `ZSTD_CCtx_setPledgedSrcSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 252 | `ZSTD_CCtx_trace` | documented baseline plus zero, boundary, and randomized values | [x] |
| 253 | `ZSTD_CStreamInSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 254 | `ZSTD_CStreamOutSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 255 | `ZSTD_DCtx_getParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 256 | `ZSTD_DCtx_loadDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 257 | `ZSTD_DCtx_loadDictionary_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 258 | `ZSTD_DCtx_loadDictionary_byReference` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 259 | `ZSTD_DCtx_refDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 260 | `ZSTD_DCtx_refPrefix` | documented baseline plus zero, boundary, and randomized values | [x] |
| 261 | `ZSTD_DCtx_refPrefix_advanced` | documented baseline plus zero, boundary, and randomized values | [x] |
| 262 | `ZSTD_DCtx_reset` | documented baseline plus zero, boundary, and randomized values | [x] |
| 263 | `ZSTD_DCtx_setFormat` | documented baseline plus zero, boundary, and randomized values | [x] |
| 264 | `ZSTD_DCtx_setMaxWindowSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 265 | `ZSTD_DCtx_setParameter` | documented baseline plus zero, boundary, and randomized values | [x] |
| 266 | `ZSTD_DDict_dictContent` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 267 | `ZSTD_DDict_dictSize` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 268 | `ZSTD_DStreamInSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 269 | `ZSTD_DStreamOutSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 270 | `ZSTD_XXH32` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 271 | `ZSTD_XXH32_canonicalFromHash` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 272 | `ZSTD_XXH32_copyState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 273 | `ZSTD_XXH32_createState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 274 | `ZSTD_XXH32_digest` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 275 | `ZSTD_XXH32_freeState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 276 | `ZSTD_XXH32_hashFromCanonical` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 277 | `ZSTD_XXH32_reset` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 278 | `ZSTD_XXH32_update` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 279 | `ZSTD_XXH64` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 280 | `ZSTD_XXH64_canonicalFromHash` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 281 | `ZSTD_XXH64_copyState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 282 | `ZSTD_XXH64_createState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 283 | `ZSTD_XXH64_digest` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 284 | `ZSTD_XXH64_freeState` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 285 | `ZSTD_XXH64_hashFromCanonical` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 286 | `ZSTD_XXH64_reset` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 287 | `ZSTD_XXH64_update` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 288 | `ZSTD_XXH_versionNumber` | empty/one/many bytes; aligned and unaligned lengths; fixed-seed randomized contents | [x] |
| 289 | `ZSTD_adjustCParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 290 | `ZSTD_buildBlockEntropyStats` | documented baseline plus zero, boundary, and randomized values | [x] |
| 291 | `ZSTD_buildCTable` | documented baseline plus zero, boundary, and randomized values | [x] |
| 292 | `ZSTD_buildFSETable` | tableLog/symbol count at minimum, normal, and maximum; empty/single/many symbols | [x] |
| 293 | `ZSTD_cParam_getBounds` | documented baseline plus zero, boundary, and randomized values | [x] |
| 294 | `ZSTD_checkCParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 295 | `ZSTD_checkContinuity` | documented baseline plus zero, boundary, and randomized values | [x] |
| 296 | `ZSTD_compress` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 297 | `ZSTD_compress2` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 298 | `ZSTD_compressBegin` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 299 | `ZSTD_compressBegin_advanced` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 300 | `ZSTD_compressBegin_advanced_internal` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 301 | `ZSTD_compressBegin_usingCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 302 | `ZSTD_compressBegin_usingCDict_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 303 | `ZSTD_compressBegin_usingCDict_deprecated` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 304 | `ZSTD_compressBegin_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 305 | `ZSTD_compressBlock` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 306 | `ZSTD_compressBlock_btlazy2` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 307 | `ZSTD_compressBlock_btlazy2_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 308 | `ZSTD_compressBlock_btlazy2_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 309 | `ZSTD_compressBlock_btopt` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 310 | `ZSTD_compressBlock_btopt_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 311 | `ZSTD_compressBlock_btopt_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 312 | `ZSTD_compressBlock_btultra` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 313 | `ZSTD_compressBlock_btultra2` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 314 | `ZSTD_compressBlock_btultra_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 315 | `ZSTD_compressBlock_btultra_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 316 | `ZSTD_compressBlock_deprecated` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 317 | `ZSTD_compressBlock_doubleFast` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 318 | `ZSTD_compressBlock_doubleFast_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 319 | `ZSTD_compressBlock_doubleFast_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 320 | `ZSTD_compressBlock_fast` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 321 | `ZSTD_compressBlock_fast_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 322 | `ZSTD_compressBlock_fast_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 323 | `ZSTD_compressBlock_greedy` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 324 | `ZSTD_compressBlock_greedy_dedicatedDictSearch` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 325 | `ZSTD_compressBlock_greedy_dedicatedDictSearch_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 326 | `ZSTD_compressBlock_greedy_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 327 | `ZSTD_compressBlock_greedy_dictMatchState_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 328 | `ZSTD_compressBlock_greedy_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 329 | `ZSTD_compressBlock_greedy_extDict_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 330 | `ZSTD_compressBlock_greedy_row` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 331 | `ZSTD_compressBlock_lazy` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 332 | `ZSTD_compressBlock_lazy2` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 333 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 334 | `ZSTD_compressBlock_lazy2_dedicatedDictSearch_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 335 | `ZSTD_compressBlock_lazy2_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 336 | `ZSTD_compressBlock_lazy2_dictMatchState_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 337 | `ZSTD_compressBlock_lazy2_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 338 | `ZSTD_compressBlock_lazy2_extDict_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 339 | `ZSTD_compressBlock_lazy2_row` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 340 | `ZSTD_compressBlock_lazy_dedicatedDictSearch` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 341 | `ZSTD_compressBlock_lazy_dedicatedDictSearch_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 342 | `ZSTD_compressBlock_lazy_dictMatchState` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 343 | `ZSTD_compressBlock_lazy_dictMatchState_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 344 | `ZSTD_compressBlock_lazy_extDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 345 | `ZSTD_compressBlock_lazy_extDict_row` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 346 | `ZSTD_compressBlock_lazy_row` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 347 | `ZSTD_compressBound` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 348 | `ZSTD_compressCCtx` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 349 | `ZSTD_compressContinue` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 350 | `ZSTD_compressContinue_public` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 351 | `ZSTD_compressEnd` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 352 | `ZSTD_compressEnd_public` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 353 | `ZSTD_compressLiterals` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 354 | `ZSTD_compressRleLiteralsBlock` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 355 | `ZSTD_compressSequences` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 356 | `ZSTD_compressSequencesAndLiterals` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 357 | `ZSTD_compressStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 358 | `ZSTD_compressStream2` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 359 | `ZSTD_compressStream2_simpleArgs` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 360 | `ZSTD_compressSuperBlock` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 361 | `ZSTD_compress_advanced` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 362 | `ZSTD_compress_advanced_internal` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 363 | `ZSTD_compress_usingCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 364 | `ZSTD_compress_usingCDict_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 365 | `ZSTD_compress_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 366 | `ZSTD_convertBlockSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 367 | `ZSTD_copyCCtx` | documented baseline plus zero, boundary, and randomized values | [x] |
| 368 | `ZSTD_copyDCtx` | documented baseline plus zero, boundary, and randomized values | [x] |
| 369 | `ZSTD_copyDDictParameters` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 370 | `ZSTD_createCCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 371 | `ZSTD_createCCtxParams` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 372 | `ZSTD_createCCtx_advanced` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 373 | `ZSTD_createCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 374 | `ZSTD_createCDict_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 375 | `ZSTD_createCDict_advanced2` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 376 | `ZSTD_createCDict_byReference` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 377 | `ZSTD_createCStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 378 | `ZSTD_createCStream_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 379 | `ZSTD_createDCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 380 | `ZSTD_createDCtx_advanced` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 381 | `ZSTD_createDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 382 | `ZSTD_createDDict_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 383 | `ZSTD_createDDict_byReference` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 384 | `ZSTD_createDStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 385 | `ZSTD_createDStream_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 386 | `ZSTD_crossEntropyCost` | documented baseline plus zero, boundary, and randomized values | [x] |
| 387 | `ZSTD_cycleLog` | documented baseline plus zero, boundary, and randomized values | [x] |
| 388 | `ZSTD_dParam_getBounds` | documented baseline plus zero, boundary, and randomized values | [x] |
| 389 | `ZSTD_decodeLiteralsBlock_wrapper` | documented baseline plus zero, boundary, and randomized values | [x] |
| 390 | `ZSTD_decodeSeqHeaders` | documented baseline plus zero, boundary, and randomized values | [x] |
| 391 | `ZSTD_decodingBufferSize_min` | documented baseline plus zero, boundary, and randomized values | [x] |
| 392 | `ZSTD_decompress` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 393 | `ZSTD_decompressBegin` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 394 | `ZSTD_decompressBegin_usingDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 395 | `ZSTD_decompressBegin_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 396 | `ZSTD_decompressBlock` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 397 | `ZSTD_decompressBlock_deprecated` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 398 | `ZSTD_decompressBlock_internal` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 399 | `ZSTD_decompressBound` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 400 | `ZSTD_decompressContinue` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 401 | `ZSTD_decompressDCtx` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 402 | `ZSTD_decompressStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 403 | `ZSTD_decompressStream_simpleArgs` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 404 | `ZSTD_decompress_usingDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 405 | `ZSTD_decompress_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 406 | `ZSTD_decompressionMargin` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 407 | `ZSTD_dedicatedDictSearch_lazy_loadDictionary` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 408 | `ZSTD_defaultCLevel` | documented baseline plus zero, boundary, and randomized values | [x] |
| 409 | `ZSTD_encodeSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 410 | `ZSTD_endStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 411 | `ZSTD_estimateCCtxSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 412 | `ZSTD_estimateCCtxSize_usingCCtxParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 413 | `ZSTD_estimateCCtxSize_usingCParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 414 | `ZSTD_estimateCDictSize` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 415 | `ZSTD_estimateCDictSize_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 416 | `ZSTD_estimateCStreamSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 417 | `ZSTD_estimateCStreamSize_usingCCtxParams` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 418 | `ZSTD_estimateCStreamSize_usingCParams` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 419 | `ZSTD_estimateDCtxSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 420 | `ZSTD_estimateDDictSize` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 421 | `ZSTD_estimateDStreamSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 422 | `ZSTD_estimateDStreamSize_fromFrame` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 423 | `ZSTD_fillDoubleHashTable` | documented baseline plus zero, boundary, and randomized values | [x] |
| 424 | `ZSTD_fillHashTable` | documented baseline plus zero, boundary, and randomized values | [x] |
| 425 | `ZSTD_findDecompressedSize` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 426 | `ZSTD_findFrameCompressedSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 427 | `ZSTD_flushStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 428 | `ZSTD_frameHeaderSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 429 | `ZSTD_freeCCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 430 | `ZSTD_freeCCtxParams` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 431 | `ZSTD_freeCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 432 | `ZSTD_freeCStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 433 | `ZSTD_freeDCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 434 | `ZSTD_freeDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 435 | `ZSTD_freeDStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 436 | `ZSTD_fseBitCost` | documented baseline plus zero, boundary, and randomized values | [x] |
| 437 | `ZSTD_generateSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 438 | `ZSTD_get1BlockSummary` | documented baseline plus zero, boundary, and randomized values | [x] |
| 439 | `ZSTD_getBlockSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 440 | `ZSTD_getCParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 441 | `ZSTD_getCParamsFromCCtxParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 442 | `ZSTD_getCParamsFromCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 443 | `ZSTD_getDecompressedSize` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 444 | `ZSTD_getDictID_fromCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 445 | `ZSTD_getDictID_fromDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 446 | `ZSTD_getDictID_fromDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 447 | `ZSTD_getDictID_fromFrame` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 448 | `ZSTD_getErrorCode` | documented baseline plus zero, boundary, and randomized values | [x] |
| 449 | `ZSTD_getErrorName` | documented baseline plus zero, boundary, and randomized values | [x] |
| 450 | `ZSTD_getErrorString` | documented baseline plus zero, boundary, and randomized values | [x] |
| 451 | `ZSTD_getFrameContentSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 452 | `ZSTD_getFrameHeader` | documented baseline plus zero, boundary, and randomized values | [x] |
| 453 | `ZSTD_getFrameHeader_advanced` | documented baseline plus zero, boundary, and randomized values | [x] |
| 454 | `ZSTD_getFrameProgression` | documented baseline plus zero, boundary, and randomized values | [x] |
| 455 | `ZSTD_getParams` | documented baseline plus zero, boundary, and randomized values | [x] |
| 456 | `ZSTD_getSeqStore` | documented baseline plus zero, boundary, and randomized values | [x] |
| 457 | `ZSTD_getcBlockSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 458 | `ZSTD_initCStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 459 | `ZSTD_initCStream_advanced` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 460 | `ZSTD_initCStream_internal` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 461 | `ZSTD_initCStream_srcSize` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 462 | `ZSTD_initCStream_usingCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 463 | `ZSTD_initCStream_usingCDict_advanced` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 464 | `ZSTD_initCStream_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 465 | `ZSTD_initDStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 466 | `ZSTD_initDStream_usingDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 467 | `ZSTD_initDStream_usingDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 468 | `ZSTD_initStaticCCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 469 | `ZSTD_initStaticCDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 470 | `ZSTD_initStaticCStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 471 | `ZSTD_initStaticDCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 472 | `ZSTD_initStaticDDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 473 | `ZSTD_initStaticDStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 474 | `ZSTD_insertAndFindFirstIndex` | documented baseline plus zero, boundary, and randomized values | [x] |
| 475 | `ZSTD_insertBlock` | documented baseline plus zero, boundary, and randomized values | [x] |
| 476 | `ZSTD_invalidateRepCodes` | documented baseline plus zero, boundary, and randomized values | [x] |
| 477 | `ZSTD_isError` | documented baseline plus zero, boundary, and randomized values | [x] |
| 478 | `ZSTD_isFrame` | documented baseline plus zero, boundary, and randomized values | [x] |
| 479 | `ZSTD_isSkippableFrame` | documented baseline plus zero, boundary, and randomized values | [x] |
| 480 | `ZSTD_ldm_adjustParameters` | documented baseline plus zero, boundary, and randomized values | [x] |
| 481 | `ZSTD_ldm_blockCompress` | documented baseline plus zero, boundary, and randomized values | [x] |
| 482 | `ZSTD_ldm_fillHashTable` | documented baseline plus zero, boundary, and randomized values | [x] |
| 483 | `ZSTD_ldm_generateSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 484 | `ZSTD_ldm_getMaxNbSeq` | documented baseline plus zero, boundary, and randomized values | [x] |
| 485 | `ZSTD_ldm_getTableSize` | documented baseline plus zero, boundary, and randomized values | [x] |
| 486 | `ZSTD_ldm_skipRawSeqStoreBytes` | documented baseline plus zero, boundary, and randomized values | [x] |
| 487 | `ZSTD_ldm_skipSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 488 | `ZSTD_loadCEntropy` | documented baseline plus zero, boundary, and randomized values | [x] |
| 489 | `ZSTD_loadDEntropy` | documented baseline plus zero, boundary, and randomized values | [x] |
| 490 | `ZSTD_maxCLevel` | documented baseline plus zero, boundary, and randomized values | [x] |
| 491 | `ZSTD_mergeBlockDelimiters` | documented baseline plus zero, boundary, and randomized values | [x] |
| 492 | `ZSTD_minCLevel` | documented baseline plus zero, boundary, and randomized values | [x] |
| 493 | `ZSTD_nextInputType` | documented baseline plus zero, boundary, and randomized values | [x] |
| 494 | `ZSTD_nextSrcSizeToDecompress` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 495 | `ZSTD_noCompressLiterals` | documented baseline plus zero, boundary, and randomized values | [x] |
| 496 | `ZSTD_readSkippableFrame` | documented baseline plus zero, boundary, and randomized values | [x] |
| 497 | `ZSTD_referenceExternalSequences` | documented baseline plus zero, boundary, and randomized values | [x] |
| 498 | `ZSTD_registerSequenceProducer` | documented baseline plus zero, boundary, and randomized values | [x] |
| 499 | `ZSTD_resetCStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 500 | `ZSTD_resetDStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 501 | `ZSTD_resetSeqStore` | documented baseline plus zero, boundary, and randomized values | [x] |
| 502 | `ZSTD_reset_compressedBlockState` | one-shot empty/one/many bytes; destination below/exact/above bound; compression levels min/default/max | [x] |
| 503 | `ZSTD_row_update` | documented baseline plus zero, boundary, and randomized values | [x] |
| 504 | `ZSTD_selectBlockCompressor` | documented baseline plus zero, boundary, and randomized values | [x] |
| 505 | `ZSTD_selectEncodingType` | documented baseline plus zero, boundary, and randomized values | [x] |
| 506 | `ZSTD_seqToCodes` | documented baseline plus zero, boundary, and randomized values | [x] |
| 507 | `ZSTD_sequenceBound` | documented baseline plus zero, boundary, and randomized values | [x] |
| 508 | `ZSTD_sizeof_CCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 509 | `ZSTD_sizeof_CDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 510 | `ZSTD_sizeof_CStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 511 | `ZSTD_sizeof_DCtx` | null/allocated/static/custom-memory object lifecycle and boundary sizes | [x] |
| 512 | `ZSTD_sizeof_DDict` | dictionary absent/raw/full; copy/reference load; empty/one/many samples or bytes | [x] |
| 513 | `ZSTD_sizeof_DStream` | stream start/continue/flush/end; empty/partial/full buffers; one or many chunks | [x] |
| 514 | `ZSTD_splitBlock` | documented baseline plus zero, boundary, and randomized values | [x] |
| 515 | `ZSTD_toFlushNow` | documented baseline plus zero, boundary, and randomized values | [x] |
| 516 | `ZSTD_updateTree` | documented baseline plus zero, boundary, and randomized values | [x] |
| 517 | `ZSTD_versionNumber` | documented baseline plus zero, boundary, and randomized values | [x] |
| 518 | `ZSTD_versionString` | documented baseline plus zero, boundary, and randomized values | [x] |
| 519 | `ZSTD_writeLastEmptyBlock` | documented baseline plus zero, boundary, and randomized values | [x] |
| 520 | `ZSTD_writeSkippableFrame` | documented baseline plus zero, boundary, and randomized values | [x] |
| 521 | `ZSTDv01_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 522 | `ZSTDv01_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 523 | `ZSTDv01_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 524 | `ZSTDv01_decompressDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 525 | `ZSTDv01_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 526 | `ZSTDv01_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 527 | `ZSTDv01_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 528 | `ZSTDv01_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 529 | `ZSTDv01_resetDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 530 | `ZSTDv02_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 531 | `ZSTDv02_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 532 | `ZSTDv02_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 533 | `ZSTDv02_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 534 | `ZSTDv02_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 535 | `ZSTDv02_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 536 | `ZSTDv02_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 537 | `ZSTDv02_resetDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 538 | `ZSTDv03_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 539 | `ZSTDv03_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 540 | `ZSTDv03_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 541 | `ZSTDv03_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 542 | `ZSTDv03_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 543 | `ZSTDv03_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 544 | `ZSTDv03_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 545 | `ZSTDv03_resetDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 546 | `ZSTDv04_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 547 | `ZSTDv04_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 548 | `ZSTDv04_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 549 | `ZSTDv04_decompressDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 550 | `ZSTDv04_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 551 | `ZSTDv04_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 552 | `ZSTDv04_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 553 | `ZSTDv04_resetDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 554 | `ZSTDv05_copyDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 555 | `ZSTDv05_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 556 | `ZSTDv05_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 557 | `ZSTDv05_decompressBegin` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 558 | `ZSTDv05_decompressBegin_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 559 | `ZSTDv05_decompressBlock` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 560 | `ZSTDv05_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 561 | `ZSTDv05_decompressDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 562 | `ZSTDv05_decompress_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 563 | `ZSTDv05_decompress_usingPreparedDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 564 | `ZSTDv05_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 565 | `ZSTDv05_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 566 | `ZSTDv05_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 567 | `ZSTDv05_getFrameParams` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 568 | `ZSTDv05_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 569 | `ZSTDv05_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 570 | `ZSTDv05_sizeofDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 571 | `ZSTDv06_copyDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 572 | `ZSTDv06_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 573 | `ZSTDv06_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 574 | `ZSTDv06_decompressBegin` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 575 | `ZSTDv06_decompressBegin_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 576 | `ZSTDv06_decompressBlock` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 577 | `ZSTDv06_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 578 | `ZSTDv06_decompressDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 579 | `ZSTDv06_decompress_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 580 | `ZSTDv06_decompress_usingPreparedDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 581 | `ZSTDv06_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 582 | `ZSTDv06_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 583 | `ZSTDv06_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 584 | `ZSTDv06_getFrameParams` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 585 | `ZSTDv06_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 586 | `ZSTDv06_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 587 | `ZSTDv06_sizeofDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 588 | `ZSTDv07_copyDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 589 | `ZSTDv07_createDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 590 | `ZSTDv07_createDCtx_advanced` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 591 | `ZSTDv07_createDDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 592 | `ZSTDv07_decompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 593 | `ZSTDv07_decompressBegin` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 594 | `ZSTDv07_decompressBegin_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 595 | `ZSTDv07_decompressBlock` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 596 | `ZSTDv07_decompressContinue` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 597 | `ZSTDv07_decompressDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 598 | `ZSTDv07_decompress_usingDDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 599 | `ZSTDv07_decompress_usingDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 600 | `ZSTDv07_estimateDCtxSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 601 | `ZSTDv07_findFrameSizeInfoLegacy` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 602 | `ZSTDv07_freeDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 603 | `ZSTDv07_freeDDict` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 604 | `ZSTDv07_getDecompressedSize` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 605 | `ZSTDv07_getErrorName` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 606 | `ZSTDv07_getFrameParams` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 607 | `ZSTDv07_insertBlock` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 608 | `ZSTDv07_isError` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 609 | `ZSTDv07_isSkipFrame` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 610 | `ZSTDv07_nextSrcSizeToDecompress` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 611 | `ZSTDv07_sizeofDCtx` | legacy format version encoded by entry-point name; empty/one/many bytes; exact and truncated frames | [x] |
| 612 | `divbwt` | documented baseline plus zero, boundary, and randomized values | [x] |
| 613 | `divsufsort` | documented baseline plus zero, boundary, and randomized values | [x] |
| 614 | `g_ZSTD_threading_useless_symbol` | exported data symbol; initial value and external read/write visibility | [x] |
| 615 | `g_debuglevel` | exported data symbol; initial value and external read/write visibility | [x] |

## Shared Branch Axes

The per-entry-point rows above are crossed with the applicable C branches below. Combinations that a family cannot consume are pruned.

| axis | C-distinguished values |
|------|------------------------|
| input count | 0, 1, many |
| input size | 0, 1, block boundary - 1, block boundary, block boundary + 1, randomized larger values |
| destination capacity | 0, one below required, exact required, above required |
| compression level | `ZSTD_minCLevel()`, negative fast levels, 0/default, `ZSTD_maxCLevel()` |
| strategy | `ZSTD_fast` through `ZSTD_btultra2` |
| frame flags | content size off/on x checksum off/on x dictionary ID off/on |
| frame format | standard/magicless; normal/skippable; current/legacy v01-v07 |
| dictionary | absent, empty raw, non-empty raw, full dictionary; copy/reference; CDict/DDict |
| stream directive | continue, flush, end |
| stream chunking | all-at-once, byte-at-a-time, randomized chunks; zero/exact/oversized output |
| reset directive | session, parameters, session-and-parameters |
| decompression | checksum validate/ignore; window default/min/max; standard/magicless |
| entropy tables | RLE/raw/compressed/repeat; 1X/4X; X1/X2 decoder; min/default/max table log |
| threading | compiled single-threaded (`ZSTD_MULTITHREAD` absent), worker count 0 and rejected nonzero |
| memory | heap/static/custom allocator; aligned/misaligned workspace; exact/undersized workspace |
| byte content | zeroes, repeated bytes, ramps, high entropy, fixed-seed random |
| enum FFI boundary | every declared value plus one below/above and an unrelated integer |
