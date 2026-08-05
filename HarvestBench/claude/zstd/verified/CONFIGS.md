# CONFIGS.md — Configuration-surface table (valid inputs)

Meaningful combinations of options × input shapes that the C code treats
differently. Each row is a differential test comparing C .so vs Rust .so.

## Axes
- **Compression level**: negative (fast), 0 (=default 3), 1, 3, 9, 19, 22 (max)
- **Input shape**: empty, 1 byte, small (<128KB), large (>128KB, crosses
  block boundary), highly-compressible (repeated), incompressible (random)
- **Advanced params**: checksumFlag, contentSizeFlag, dictIDFlag, windowLog,
  strategy, enableLongDistanceMatching, targetLength, minMatch
- **Entry points**: one-shot (`ZSTD_compress`/`ZSTD_decompress`),
  ctx-based (`ZSTD_compressCCtx`/`ZSTD_decompressDCtx`),
  advanced (`ZSTD_compress2` + `ZSTD_CCtx_setParameter`),
  streaming (`ZSTD_compressStream2`/`ZSTD_decompressStream`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| 1 | `ZSTD_compressBound` | many random srcSizes incl. 0, 1, boundaries | [x] |
| 2 | `ZSTD_isError`/`getErrorName`/`getErrorCode` | across value range | [x] |
| 3 | `ZSTD_versionNumber`/`versionString`/`min/maxCLevel`/`defaultCLevel` | constants | [x] |
| 4 | `ZSTD_compress`→`ZSTD_decompress` | level 1, small random inputs | [x] |
| 5 | `ZSTD_compress`→`ZSTD_decompress` | level 3, empty & 1-byte | [x] |
| 6 | `ZSTD_compress`→`ZSTD_decompress` | levels {-5,0,9,19,22}, small inputs | [x] |
| 7 | `ZSTD_compress`→`ZSTD_decompress` | large (>128KB) compressible input | [x] |
| 8 | `ZSTD_compress`→`ZSTD_decompress` | large incompressible (random) input | [x] |
| 9 | `ZSTD_compressCCtx`→`ZSTD_decompressDCtx` | various levels, roundtrip | [x] |
| 10 | `ZSTD_getFrameContentSize` | valid frames of known sizes | [x] |
| 11 | `ZSTD_getDecompressedSize` | valid frames | [x] |
| 12 | `ZSTD_findFrameCompressedSize` | valid single frame | [x] |
| 13 | `ZSTD_compress2`+`setParameter(compressionLevel)` | roundtrip | [x] |
| 14 | `ZSTD_compress2`+`setParameter(checksumFlag=1)` | roundtrip, verify checksum frame | [x] |
| 15 | `ZSTD_compress2`+`setParameter(contentSizeFlag=0)` | content size unknown in frame | [x] |
| 16 | `ZSTD_compress2`+`setParameter(windowLog=n)` | various windowLog, roundtrip | [x] |
| 17 | `ZSTD_compress2`+`setParameter(strategy=n)` | all strategies 1..9, roundtrip | [x] |
| 18 | `ZSTD_compress2`+`setParameter(enableLDM=1)` | large input, roundtrip | [x] |
| 19 | `ZSTD_compressStream2`→`ZSTD_decompressStream` | streaming roundtrip, chunked | [x] |
| 20 | `ZSTD_cParam_getBounds` | all valid cParameters | [x] |
| 21 | `ZSTD_dParam_getBounds` | all valid dParameters | [x] |
| 22 | `ZSTD_compressBound`/`ZSTD_decompressBound` | boundary sizes | [x] |
| 23 | `ZSTD_CStreamInSize`/`OutSize`/`DStreamInSize`/`OutSize` | constants | [x] |
| 24 | XXH64 (`XXH64`/`XXH32`) via ZSTD_ namespace | random buffers, all lengths | [x] |
| 25 | `ZSTD_maxCLevel` full sweep every level roundtrip | small input | [x] |
| 26 | `ZSTD_compress2`+`setParameter(minMatch,targetLength)` | roundtrip | [x] |
| 27 | `ZSTD_compress` with all levels min..max | fixed input, byte-identical output | [x] |

## Feature combinations (Phase D)

`Cargo.toml` has **no `[features]` section** and `CMakeLists.txt` defines a
single fixed set of compile definitions (`ZSTD_LEGACY_SUPPORT=5`,
`XXH_NAMESPACE=ZSTD_`, `DYNAMIC_BMI2=0`). Therefore there is exactly ONE valid
build configuration (the default). All rows above were verified under it.
`cargo check` and `cargo check --no-default-features` both succeed with no
errors.

## Fix applied during verification

- `huf_compress.rs` `HUF_addBits`/`HUF_mergeIndex1`: used `wrapping_add` to
  match C's `size_t` accumulator arithmetic (only the low byte is ever read).
- `Cargo.toml`: added `overflow-checks = false` to `[profile.dev]` and
  `[profile.release]`. The C source relies on 2's-complement wrapping
  arithmetic (literally commented `/* intentional overflow */` in zstd_opt.c),
  which C unsigned math performs by definition. Disabling Rust's debug overflow
  panics makes the translation byte-identical to the C ground truth.
