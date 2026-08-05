# ERRORS.md — Error-surface table

Distinct rejection/error conditions in the C source, focused on the
chunk-reading pipeline (newly translated `png_handle_chunk`/
`png_handle_unknown`, `src/pngrutil_b1a.rs`) and the surrounding public API
reachable by differential FFI tests. Derived mechanically from
`png_chunk_error` / `png_chunk_benign_error` / `png_error` / `return` sites.

Every row has a passing differential test (`tests/errors.rs` /
`tests/pure_functions.rs`) that constructs the exact condition, decodes it with
BOTH the C and the Rust `.so`, and asserts the same reaction (same fired /
not-fired AND same error message string).

| # | function | trigger (exact invalid input/condition) | expected C result | test | [x] |
|---|----------|-------------------------------------------|-------------------|------|-----|
| 1 | png_handle_chunk | non-IHDR chunk arrives while (mode & PNG_HAVE_IHDR)==0 | png_chunk_error "missing IHDR" (longjmp) | errors::missing_ihdr_first | [x] |
| 2 | png_handle_chunk | known chunk out of position (pos_before/pos_after violated), ancillary | png_chunk_benign_error "out of place" | errors::ancillary_out_of_place | [x] |
| 3 | png_handle_chunk | known non-multiple chunk seen twice, ancillary | png_chunk_benign_error "duplicate" | errors::duplicate_ancillary | [x] |
| 4 | png_handle_chunk | known chunk length < min_length, ancillary | png_chunk_benign_error "too short" | errors::chunk_too_short | [x] |
| 5 | png_handle_chunk | known chunk length > max_length (fixed), ancillary | png_chunk_benign_error "too long" | errors::chunk_too_long | [x] |
| 6 | png_handle_chunk | Limit-length chunk length > png_chunk_max / 31-bit limit | png_error / benign "length exceeds libpng limit" | errors::oversized_chunk_length | [x] |
| 7 | png_handle_chunk | critical chunk (IHDR/PLTE/IDAT) with any errmsg | png_chunk_error (longjmp) | errors::duplicate_ihdr | [x] |
| 8 | png_handle_unknown | unhandled critical unknown chunk | png_chunk_error "unhandled critical chunk" | errors::unknown_critical_chunk | [x] |
| 9 | png_handle_unknown | unknown ancillary chunk (discarded, no error) | decode succeeds, chunk skipped | errors::unknown_ancillary_chunk | [x] |
| 10 | png_handle_IHDR | duplicate IHDR (critical, multiple==0) | png_chunk_error "duplicate" | errors::duplicate_ihdr | [x] |
| 11 | png_check_IHDR | invalid color_type value | png_error "Invalid color type ..." | errors::invalid_color_type_in_ihdr | [x] |
| 12 | png_read_chunk_header | chunk length > PNG_UINT_31_MAX | png_error / png_chunk_error | errors::oversized_chunk_length | [x] |
| 13 | png_crc_finish | bad CRC on chunk | png_error / png_chunk_error | errors::bad_crc | [x] |
| 14 | png_sig_cmp | signature bytes do not match PNG signature | returns non-zero | pure::sig_cmp_matches, errors::bad_signature | [x] |
| 15 | png_check_IHDR | invalid bit_depth value | png_error "Invalid bit depth" | errors::invalid_bit_depth_in_ihdr | [x] |
| 16 | png_check_IHDR | zero width or height | png_error "Image ... is zero ..." | errors::zero_dimensions_in_ihdr | [x] |
| 17 | png_convert_to_rfc1123_buffer | out == NULL, and out-of-range time fields | returns 0 | pure::convert_to_rfc1123_buffer_matches | [x] |
| 18 | read pipeline | truncated stream (EOF mid-chunk) | png_error | errors::truncated_stream | [x] |
| 19 | png_read_info | first chunk is not IHDR | png_chunk_error "missing IHDR" | errors::missing_ihdr_first | [x] |
| 20 | read pipeline | empty / 1-byte / signature-only inputs | png_error | errors::empty_and_tiny | [x] |
| 21 | rebuild sanity | valid rebuilt stream decodes cleanly on both | ret==0, no error | errors::baseline_rebuild_is_valid | [x] |
