# CONFIGS.md — Configuration-surface table

Valid-input combinations the C code branches on. Focused on the entry
points reachable through the FFI boundary for differential testing:
pure serialization helpers, and the full write->read roundtrip which
exercises `png_handle_chunk`/`png_handle_unknown` (the newly translated
code) plus every `png_handle_XXX`.

| # | entry point(s) | configuration (options + input shape) | [x] |
|---|----------------|----------------------------------------|-----|
| 1  | png_get_uint_32 | all 4-byte big-endian inputs (random + boundary 0/MAX) | [x] |
| 2  | png_get_uint_16 | all 2-byte big-endian inputs | [x] |
| 3  | png_get_int_32  | 4-byte, including negative / INT_MIN sign-magnitude | [x] |
| 4  | png_get_uint_31 | valid values <= 0x7fffffff | [x] |
| 5  | png_save_uint_32 | round-trip write then get_uint_32 | [x] |
| 6  | png_save_int_32  | round-trip incl. negatives | [x] |
| 7  | png_save_uint_16 | round-trip | [x] |
| 8  | png_sig_cmp | valid sig, partial ranges (start_byte, num_bytes) | [x] |
| 9  | png_access_version_number | constant | [x] |
| 10 | png_get_rowbytes-equivalent (roundtrip) | rowbytes for each color_type x bit_depth | [x] |
| 11 | write+read roundtrip | GRAY, bit_depth 1/2/4/8/16 | [x] |
| 12 | write+read roundtrip | RGB, bit_depth 8/16 | [x] |
| 13 | write+read roundtrip | PALETTE, bit_depth 1/2/4/8 (with PLTE) | [x] |
| 14 | write+read roundtrip | GRAY_ALPHA, bit_depth 8/16 | [x] |
| 15 | write+read roundtrip | RGB_ALPHA, bit_depth 8/16 | [x] |
| 16 | write+read roundtrip | interlace NONE vs ADAM7 | [x] |
| 17 | write+read roundtrip | with ancillary chunks (tEXt, gAMA, pHYs, tIME) | [x] |
| 18 | write+read roundtrip | with unknown chunk saved (png_set_keep_unknown_chunks) | [x] |
| 19 | write+read roundtrip | varying widths (1..N) and heights (1..N) | [x] |
| 20 | write+read roundtrip | each filter type / compression level | [x] |
| 21 | png_convert_to_rfc1123_buffer | various png_time values | [x] |
