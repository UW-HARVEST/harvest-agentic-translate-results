# CONFIGS.md — Configuration-surface table

Mechanically derived from the C source & headers. Enumerates the meaningful
combinations of options × input shapes that the C code branches on, across all
public entry points (including low-level ones).

Axes:
- **Block API**: acceleration (1, default, >MAX clamps to 65537, <=0 clamps to 1),
  HC compression level (MIN=2, DEFAULT=9, OPT_MIN=10, MAX=12, >MAX clamps),
  destSize (reverse fill), streaming (continue/dict/prefix64k/ringbuffer).
- **Frame API**: blockSizeID (default/max64KB/max256KB/max1MB/max4MB),
  blockMode (linked/independent), contentChecksum (on/off),
  blockChecksum (on/off), contentSize (0/known), autoFlush (0/1),
  favorDecSpeed (0/1), dictID, dictionary (none/CDict/raw).
- **Input shapes**: empty (0), one byte, small (<64KB), large (>block size),
  highly compressible (repeated), incompressible (random), boundary sizes.
- **xxHash**: XXH32 vs XXH64, one-shot vs streaming (reset/update/digest),
  canonical conversion, seed variation, input alignment/size.

| # | entry point(s) | configuration (options + input shape) | [ ] |
|---|----------------|----------------------------------------|-----|
| 1 | LZ4_compressBound | various sizes incl. 0, 1, max | [x] |
| 2 | LZ4_compress_default + LZ4_decompress_safe | random sizes, compressible data | [x] |
| 3 | LZ4_compress_default + LZ4_decompress_safe | incompressible (random) data | [x] |
| 4 | LZ4_compress_default + LZ4_decompress_safe | empty input (size 0) | [x] |
| 5 | LZ4_compress_default + LZ4_decompress_safe | 1-byte input | [x] |
| 6 | LZ4_compress_fast | acceleration = 1 | [x] |
| 7 | LZ4_compress_fast | acceleration = high (>MAX, clamps) | [x] |
| 8 | LZ4_compress_fast | acceleration <= 0 (clamps to default) | [x] |
| 9 | LZ4_compress_fast_extState | external state, various sizes | [x] |
| 10 | LZ4_compress_fast_extState_fastReset | external state fast reset | [x] |
| 11 | LZ4_compress_destSize | reverse fill, tight dst budget | [x] |
| 12 | LZ4_compress_destSize_extState | ext state + acceleration | [x] |
| 13 | LZ4_decompress_safe_partial | targetOutputSize < full, exact block | [x] |
| 14 | LZ4_compress_HC | level MIN(2) | [x] |
| 15 | LZ4_compress_HC | level DEFAULT(9) | [x] |
| 16 | LZ4_compress_HC | level OPT_MIN(10) | [x] |
| 17 | LZ4_compress_HC | level MAX(12) | [x] |
| 18 | LZ4_compress_HC | level > MAX (clamps) | [x] |
| 19 | LZ4_compress_HC | level <= 0 | [x] |
| 20 | LZ4_compress_HC_extStateHC | external HC state | [x] |
| 21 | LZ4_compress_HC_destSize | HC reverse fill | [x] |
| 22 | LZ4_compress_HC_continue | streaming HC, multiple blocks | [x] |
| 23 | LZ4_compress_HC_continue_destSize | streaming HC destSize | [x] |
| 24 | LZ4_favorDecompressionSpeed + HC | favorDecSpeed=1, level>=OPT_MIN | [x] |
| 25 | LZ4 streaming: loadDict + compress_fast_continue + decompress_safe_usingDict | dict + multiple blocks | [x] |
| 26 | LZ4 streaming: loadDictSlow | slow dict load | [x] |
| 27 | LZ4 streaming: attach_dictionary | no-copy dict attach | [x] |
| 28 | LZ4_saveDict / LZ4_setStreamDecode | save+restore dict streaming | [x] |
| 29 | LZ4_decompress_safe_continue | multi-block streaming decode | [x] |
| 30 | LZ4_decompress_safe_usingDict | stateless dict decode | [x] |
| 31 | LZ4_decompress_safe_partial_usingDict | partial + dict | [x] |
| 32 | Obsolete: LZ4_compress / LZ4_compress_limitedOutput | deprecated compress | [x] |
| 33 | Obsolete: LZ4_compressHC / HC2 variants | deprecated HC | [x] |
| 34 | Obsolete: LZ4_uncompress_unknownOutputSize | deprecated decode | [x] |
| 35 | LZ4F_compressFrame + LZ4F_decompress | default prefs, roundtrip | [x] |
| 36 | LZ4F_compressFrame roundtrip | blockSizeID max64KB..max4MB (4 shapes) | [x] |
| 37 | LZ4F_compressFrame roundtrip | blockMode linked vs independent | [x] |
| 38 | LZ4F_compressFrame roundtrip | contentChecksum on/off | [x] |
| 39 | LZ4F_compressFrame roundtrip | blockChecksum on/off | [x] |
| 40 | LZ4F_compressFrame roundtrip | contentSize known (0 vs set) | [x] |
| 41 | LZ4F_compressFrame roundtrip | compressionLevel fast/HC/negative | [x] |
| 42 | LZ4F_compressFrame roundtrip | autoFlush 0/1 | [x] |
| 43 | LZ4F_compressFrameBound | various sizes + prefs | [x] |
| 44 | LZ4F streaming: createCctx+Begin+Update+End+decompress | multi-update chunks | [x] |
| 45 | LZ4F streaming update | with LZ4F_flush mid-stream | [x] |
| 46 | LZ4F_uncompressedUpdate | uncompressed block insertion | [x] |
| 47 | LZ4F_getFrameInfo | extract frame params from header | [x] |
| 48 | LZ4F_headerSize | derive header size | [x] |
| 49 | LZ4F_getBlockSize | blockSizeID → size | [x] |
| 50 | LZ4F dictionary: compressBegin_usingDict + decompress_usingDict | raw dict roundtrip | [x] |
| 51 | LZ4F dictionary: createCDict + compressFrame_usingCDict | CDict roundtrip | [x] |
| 52 | LZ4F dictionary: compressBegin_usingCDict streaming | CDict streaming | [x] |
| 53 | LZ4F_compressBegin_usingDictOnce | dict once path | [x] |
| 54 | LZ4_XXH32 one-shot | various sizes, seeds | [x] |
| 55 | LZ4_XXH64 one-shot | various sizes, seeds | [x] |
| 56 | LZ4_XXH32 streaming: createState/reset/update/digest | multi-chunk update | [x] |
| 57 | LZ4_XXH64 streaming | multi-chunk update | [x] |
| 58 | LZ4_XXH32_canonicalFromHash / hashFromCanonical | canonical roundtrip | [x] |
| 59 | LZ4_XXH64_canonicalFromHash / hashFromCanonical | canonical roundtrip | [x] |
| 60 | LZ4_XXH*_copyState | state copy mid-stream | [x] |
| 61 | LZ4_versionNumber / versionString / LZ4F_getVersion / XXH_versionNumber | version constants | [x] |
| 62 | LZ4_decoderRingBufferSize / LZ4_sizeofState(HC) / LZ4_sizeofStreamState | size query functions | [x] |
| 63 | LZ4F_compressionLevel_max | constant | [x] |
| 64 | LZ4F_readOpen/read/readClose + writeOpen/write/writeClose | file API roundtrip via tmpfile | [x] |
