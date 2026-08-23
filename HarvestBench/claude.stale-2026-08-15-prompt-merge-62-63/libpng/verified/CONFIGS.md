# CONFIGS.md — configuration surface (valid inputs) of the C implementation

Derived from the branch structure of `c_src/src/*.c` and the public/internal API
surface (`png.h`: 218 `PNG_EXPORT` + 40 `PNG_FP/FIXED_EXPORT` entry points;
`pngpriv.h`: 184 `PNG_INTERNAL_FUNCTION` entry points — **all** of them are
exported from the `.so` and therefore all of them are directly callable).

The axes below are the ones the C code actually branches on:

| axis | values the C distinguishes | where |
|------|----------------------------|-------|
| colour type | 0 GRAY, 2 RGB, 3 PALETTE, 4 GRAY_ALPHA, 6 RGB_ALPHA | `png_check_IHDR`, `png_read_start_row`, all row transforms |
| bit depth | 1, 2, 4, 8, 16 (legal subset per colour type) | `png_check_IHDR`, `PNG_ROWBYTES`, sub-byte packing paths |
| interlace | 0 none, 1 Adam7 (7 passes, per-pass geometry) | `png_do_read_interlace`, `png_do_write_interlace`, `png_combine_row` |
| width/height | 1 (degenerate), small (sub-byte remainder), > 1 row buffer | `PNG_ROWBYTES` rounding, pass emptiness |
| filter set | NONE / SUB / UP / AVG / PAETH / ALL, per-row heuristic | `png_write_find_filter`, `png_read_filter_row` |
| zlib params | level 0/1/6/9, strategy, window bits 8..15, mem level 1..9, method | `png_write_IDAT`, `png_deflate_claim` |
| IDAT buffering | `png_set_compression_buffer_size`, multi-IDAT split | `png_compress_IDAT` |
| write API | `png_write_row` / `png_write_rows` / `png_write_image` / `png_write_png` | `pngwrite.c` |
| read API | sequential (`png_read_row/rows/image/png`) / progressive (`png_process_data`) / simplified (`png_image_*`) | `pngread.c`, `pngpread.c` |
| read transforms | the 26 `png_ptr->transformations` bits | `pngrtran.c` |
| flush | `png_set_flush(n)` + `png_write_flush` | `png_write_row` |
| ancillary chunks | every chunk in `PNG_KNOWN_CHUNKS` (24) + unknown chunks | `pngset.c`, `pngwutil.c`, `pngrutil.c` |
| unknown handling | AS_DEFAULT / NEVER / IF_SAFE / ALWAYS, per-chunk list, user chunk callback | `png_handle_unknown` |
| CRC action | DEFAULT / ERROR_QUIT / WARN_DISCARD / WARN_USE / QUIET_USE / NO_CHANGE, critical vs ancillary | `png_crc_finish`, `png_crc_error` |
| limits | user width/height max, chunk cache max, chunk malloc max | `png_check_IHDR`, `png_handle_unknown` |
| memory | default vs `png_create_*_struct_2` user malloc/free | `pngmem.c` |
| number format | float (`png_set_gAMA`) vs fixed (`png_set_gAMA_fixed`) entry points | `pngset.c` |

Each row below is exercised by a differential test that drives **both** `.so`s
through the identical call sequence with pseudo-random (fixed-seed) data and
compares the complete trace — return values, every warning/error message, the
produced byte stream and every decoded row — byte for byte.

## L — lowest-level entry points (called directly, no png_struct state)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| L1 | `png_get_uint_32`, `png_get_uint_16`, `png_get_int_32` | 512 random 4-byte buffers + all boundary patterns (0, 0x7fffffff, 0x80000000, 0xffffffff) | [x] |
| L2 | `png_get_uint_31` | valid values < 2^31 (read struct present) | [x] |
| L3 | `png_save_uint_32`, `png_save_int_32`, `png_save_uint_16` | 512 random values incl. negative / boundary | [x] |
| L4 | `png_muldiv` | 512 random (a,times,div) triples: exact, rounding, negative, overflowing, div==0 | [x] |
| L5 | `png_reciprocal`, `png_reciprocal2` | random fixed-point values incl. 0 and extremes | [x] |
| L6 | `png_gamma_significant`, `png_gamma_correct`, `png_gamma_8bit_correct`, `png_gamma_16bit_correct` | random gamma × random sample, 8-bit and 16-bit | [x] |
| L7 | `png_XYZ_from_xy`, `png_xy_from_XYZ` | random chromaticities + sRGB values + degenerate (0 sums) | [x] |
| L8 | `png_check_fp_number`, `png_check_fp_string` | random ASCII, valid/invalid float syntax, embedded NUL, all state transitions | [x] |
| L9 | `png_ascii_from_fp`, `png_ascii_from_fixed` | random doubles/fixed × buffer sizes (exact fit, too small), precision 1..15 | [x] |
| L10 | `png_fixed`, `png_fixed_ITU` | random doubles in and out of the representable range | [x] |
| L11 | `png_check_keyword` | random keywords: leading/trailing/multiple spaces, control chars, >79 chars, empty | [x] |
| L12 | `png_safecat`, `png_format_number`, `png_warning_parameter*`, `png_formatted_warning` | random strings/positions, buffer overflow-by-one, all number formats | [x] |
| L13 | `png_build_grayscale_palette` | bit depth 1, 2, 4, 8 (+ invalid 3, 16 → nothing written) | [x] |
| L14 | `png_convert_to_rfc1123_buffer`, `png_convert_from_time_t`, `png_convert_from_struct_tm` | random png_time incl. out-of-range fields | [x] |
| L15 | `png_sig_cmp` | correct signature, every 1-byte corruption, all (start, num_to_check) pairs | [x] |
| L16 | `png_calculate_crc`, `png_reset_crc`, `png_crc_error` | random buffers over a read struct, critical/ancillary chunk names, all CRC actions | [x] |
| L17 | `png_sRGB_table`, `png_sRGB_base`, `png_sRGB_delta` (data symbols) | full table contents compared | [x] |
| L18 | `png_malloc`, `png_calloc`, `png_malloc_warn`, `png_malloc_base`, `png_malloc_array`, `png_realloc_array`, `png_free`, `png_zalloc`, `png_zfree` | sizes 0/1/small/huge, with and without user memory callbacks | [x] |
| L19 | `png_do_bgr`, `png_do_swap`, `png_do_invert`, `png_do_packswap`, `png_do_strip_channel`, `png_do_check_palette_indexes` | every colour type × bit depth × random row content | [x] |
| L20 | `png_read_filter_row` (+ `png_read_filter_row_*` via it) | filter 0..4 × pixel_depth 1..64 (bpp 1..8) × random rows/prev rows | [x] |
| L21 | `png_do_read_interlace`, `png_do_write_interlace` | pass 0..6 × bit depth 1..16 × colour type × random rows | [x] |
| L22 | `png_combine_row` | pass 0..6, display 0/1, bit depth 1..16, random dst/src | [x] |
| L23 | `png_access_version_number`, `png_get_copyright`, `png_get_header_ver`, `png_get_libpng_ver`, `png_get_header_version` | no state | [x] |
| L24 | `png_permit_mng_features`, `png_set_option` | every option index 0..11 × ON/OFF/unset, MNG feature mask bits | [x] |
| L25 | `png_icc_check_header`, `png_icc_check_length`, `png_icc_check_tag_table` | synthetic ICC profiles: valid minimal, every header field wrong | [x] |
| L26 | `png_build_gamma_table`, `png_destroy_gamma_table` | bit depth 8/16 × screen/file gamma combinations × 16-bit table shift | [x] |
| L27 | `png_info_init_3`, `png_create_info_struct`, `png_destroy_info_struct`, `png_data_freer`, `png_free_data` | every `PNG_FREE_*` mask × DESTROY/SET/USER freer | [x] |
| L28 | `png_check_IHDR` | every colour type × bit depth combination (legal and illegal) × width/height 0/1/max × interlace 0/1/2 | [x] |
| L29 | `png_zstream_error`, `png_reset_zstream`, `png_inflate_claim`, `png_zlib_inflate` | all zlib return codes reachable via crafted streams | [x] |

## W — write pipeline (`png_write_*`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| W1 | `png_write_row` | GRAY 1/2/4/8/16 bit, interlace none, width 1..17 (sub-byte remainders), random rows | [x] |
| W2 | `png_write_row` | RGB 8/16, interlace none | [x] |
| W3 | `png_write_row` | PALETTE 1/2/4/8 with random palette (+ tRNS), interlace none | [x] |
| W4 | `png_write_row` | GRAY_ALPHA 8/16, interlace none | [x] |
| W5 | `png_write_row` | RGB_ALPHA 8/16, interlace none | [x] |
| W6 | `png_write_row` | all 15 legal colour/depth combos, interlace **Adam7** (7 passes) | [x] |
| W7 | `png_write_rows` | all combos, num_rows 1 / all / more than remaining | [x] |
| W8 | `png_write_image` | all combos × interlace 0/1 (internal per-pass loop) | [x] |
| W9 | `png_write_png` | every `PNG_TRANSFORM_*` write transform bit and combinations, via `png_set_rows` | [x] |
| W10 | `png_set_filter` | NONE / SUB / UP / AVG / PAETH / ALL / NO_FILTERS × colour type × depth (filter heuristic) | [x] |
| W11 | `png_set_compression_level` | 0, 1, 6, 9 × `png_set_compression_strategy` 0..4 | [x] |
| W12 | `png_set_compression_window_bits` | 8..15 (and the 8→9 clamp), × `png_set_compression_mem_level` 1..9 | [x] |
| W13 | `png_set_compression_method`, `png_set_compression_buffer_size` | method 8, buffer sizes 1, 2, 1024, huge (multi-IDAT split) | [x] |
| W14 | `png_set_flush` + `png_write_flush` | nrows 1, 2, 3, 0 (off) — flush callback ordering | [x] |
| W15 | `png_write_sig`, `png_write_chunk`, `png_write_chunk_start/data/end` | manual chunk emission, random names/payloads, zero-length | [x] |
| W16 | `png_write_info_before_PLTE` + `png_write_info` | split header write with palette/ancillary chunks | [x] |
| W17 | write transforms | `png_set_bgr`, `png_set_swap`, `png_set_packswap`, `png_set_invert_mono`, `png_set_invert_alpha`, `png_set_swap_alpha`, `png_set_filler(BEFORE/AFTER)`, `png_set_shift`, `png_set_packing` — each alone and all combinations that apply to the colour type | [x] |
| W18 | `png_set_write_user_transform_fn` + `png_set_user_transform_info` | user transform that rewrites the row, depth/channels overrides | [x] |
| W19 | `png_set_write_status_fn` | row/pass callback sequence for interlace 0/1 | [x] |
| W20 | `png_set_text` / `png_set_text_compression_*` | tEXt, zTXt (all compression levels/strategies/window bits/mem levels), iTXt (compressed + uncompressed, lang/lang_key) | [x] |
| W21 | `png_write_end` with an end-`info` | tIME + text after IDAT, and `png_write_end(NULL)` | [x] |
| W22 | 16-bit + `png_set_swap` + interlace + all filters | interaction of byte-swap with the filter heuristic | [x] |
| W23 | `png_create_write_struct_2` with user malloc/free | full write, allocation trace compared | [x] |
| W24 | `png_set_check_for_invalid_index` (write) | palette images with in-range and out-of-range indices | [x] |

## C — chunk set/get round trips (`pngset.c` ↔ `pngget.c` ↔ `pngwutil.c` ↔ `pngrutil.c`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C1 | `png_set_PLTE`/`png_get_PLTE` | 1, 2, 16, 255, 256 entries × bit depth; palette on RGB image | [x] |
| C2 | `png_set_tRNS`/`png_get_tRNS` | palette alpha (1..256 entries), grey key, RGB key, 16-bit values | [x] |
| C3 | `png_set_gAMA`/`_fixed` + getters | random gammas incl. 0, 1, 100000, > limits, float and fixed entry points | [x] |
| C4 | `png_set_sRGB`, `png_set_sRGB_gAMA_and_cHRM` | intent 0..3 | [x] |
| C5 | `png_set_cHRM`/`_fixed`/`_XYZ`/`_XYZ_fixed` + getters | random valid chromaticities, sRGB endpoints, XYZ round trip | [x] |
| C6 | `png_set_iCCP`/`png_get_iCCP` | synthetic profiles 132..4096 bytes, keyword lengths 1..79, compressed round trip | [x] |
| C7 | `png_set_sBIT`/getter | every colour type with legal sBIT values (and the maxima) | [x] |
| C8 | `png_set_bKGD`/getter | palette index, grey, RGB, 16-bit | [x] |
| C9 | `png_set_hIST`/getter | palette sizes 1..256, random frequencies | [x] |
| C10 | `png_set_pHYs`/getters (`png_get_pHYs_dpi`, `png_get_x_pixels_per_*`, aspect ratio) | random resolutions × unit 0/1 | [x] |
| C11 | `png_set_oFFs`/getters (`png_get_x_offset_*`) | negative/positive offsets × unit 0/1 | [x] |
| C12 | `png_set_tIME`/getter + `png_convert_to_rfc1123_buffer` | random times incl. field extremes | [x] |
| C13 | `png_set_pCAL`/getter | equation type 0..3 × nparams 0..3 × random ASCII params | [x] |
| C14 | `png_set_sCAL`/`_fixed`/`_s` + getters | unit 1/2 × random dimensions (float, fixed, string forms) | [x] |
| C15 | `png_set_sPLT`/getter | 1..3 palettes × depth 8/16 × 1..16 entries, names with spaces | [x] |
| C16 | `png_set_eXIf_1`/getter | random EXIF blobs (II/MM headers), lengths 4..1024 | [x] |
| C17 | `png_set_cICP`/getter | random primaries/transfer/matrix/full-range | [x] |
| C18 | `png_set_cLLI`/`_fixed`/getter | random maxCLL/maxFALL incl. 0 | [x] |
| C19 | `png_set_mDCV`/`_fixed`/getter | random display primaries/white point/luminances | [x] |
| C20 | `png_set_text`/`png_get_text` | tEXt/zTXt/iTXt mixes, 1..8 entries, empty text, 8-bit chars, NULL text | [x] |
| C21 | `png_set_unknown_chunks` + `png_set_unknown_chunk_location` + `png_get_unknown_chunks` | before-PLTE / before-IDAT / after-IDAT locations, 0-length data, critical-looking names | [x] |
| C22 | `png_set_rows`/`png_get_rows` + `png_free_data(PNG_FREE_ROWS)` | round trip through `png_write_png`/`png_read_png` | [x] |
| C23 | `png_set_invalid`, `png_get_valid` | every `PNG_INFO_*` flag | [x] |
| C24 | full-chunk PNG | ONE image carrying **every** supported ancillary chunk, written then read, all getters compared | [x] |

## R — sequential read pipeline (`pngread.c`, `pngrutil.c`, `pngrtran.c`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| R1 | `png_read_info` + `png_read_row` | all 15 colour/depth combos × interlace 0/1, no transforms | [x] |
| R2 | `png_read_rows` | num_rows 1 / all, with and without `display_row` | [x] |
| R3 | `png_read_image` | all combos × interlace 0/1 (internal pass loop) | [x] |
| R4 | `png_read_png` | every `PNG_TRANSFORM_*` read bit, alone and combined | [x] |
| R5 | `png_set_interlace_handling` + row-by-row | interlaced input read pass-by-pass (7 passes) | [x] |
| R6 | `png_set_packing` | GRAY/PALETTE 1/2/4 bit | [x] |
| R7 | `png_set_packswap` | GRAY/PALETTE 1/2/4 bit | [x] |
| R8 | `png_set_expand` | palette→RGB, grey<8→8, tRNS→alpha (each triggering condition) | [x] |
| R9 | `png_set_expand_16` | 8→16 bit for every colour type | [x] |
| R10 | `png_set_expand_gray_1_2_4_to_8`, `png_set_palette_to_rgb`, `png_set_tRNS_to_alpha` | individually, on matching and non-matching colour types | [x] |
| R11 | `png_set_gray_to_rgb` | GRAY/GRAY_ALPHA 1..16 bit | [x] |
| R12 | `png_set_rgb_to_gray`/`_fixed` | error action NONE/WARN/ERROR × random red/green coefficients × RGB(A) 8/16 bit + `png_get_rgb_to_gray_status` | [x] |
| R13 | `png_set_strip_16` / `png_set_scale_16` | 16-bit input, every colour type (both rounding paths) | [x] |
| R14 | `png_set_strip_alpha` | GRAY_ALPHA/RGB_ALPHA 8/16 | [x] |
| R15 | `png_set_filler` / `png_set_add_alpha` | BEFORE/AFTER × filler value × GRAY/RGB 8/16 | [x] |
| R16 | `png_set_swap`, `png_set_bgr`, `png_set_invert_mono`, `png_set_invert_alpha`, `png_set_swap_alpha` | each on every applicable colour type/depth | [x] |
| R17 | `png_set_shift` | random `png_color_8` shifts × sBIT-bearing input × 8/16 bit | [x] |
| R18 | `png_set_gamma`/`_fixed` | screen/file gamma pairs (incl. 1.0, sRGB, extremes) × 8/16 bit × colour type | [x] |
| R19 | `png_set_alpha_mode`/`_fixed` | PNG/STANDARD/OPTIMIZED/BROKEN × output gamma × alpha channel present | [x] |
| R20 | `png_set_background`/`_fixed` | gamma code UNKNOWN/SCREEN/FILE/UNIQUE × need_expand 0/1 × palette/grey/RGB/16-bit | [x] |
| R21 | `png_set_quantize` | full_quantize 0/1 × maximum_colors 2..256 × with/without histogram × palette and RGB input | [x] |
| R22 | `png_set_read_user_transform_fn` + `png_set_user_transform_info` | user transform rewriting rows, depth/channel override, `png_get_user_transform_ptr` | [x] |
| R23 | `png_set_read_status_fn` | row/pass callback sequence, interlace 0/1 | [x] |
| R24 | `png_set_crc_action` | all 6 critical × 6 ancillary actions on streams with good and bad CRCs | [x] |
| R25 | `png_set_keep_unknown_chunks` + `png_set_read_user_chunk_fn` | AS_DEFAULT/NEVER/IF_SAFE/ALWAYS × per-chunk list × user callback returning -1/0/1 | [x] |
| R26 | `png_set_user_limits`, `png_set_chunk_cache_max`, `png_set_chunk_malloc_max` | limits above and below the actual image/chunk sizes | [x] |
| R27 | `png_set_sig_bytes` | 0..8 signature bytes already consumed | [x] |
| R28 | `png_start_read_image` / `png_read_update_info` | called once, twice, and out of order (documented sequences) | [x] |
| R29 | `png_read_end` | with and without an end-info struct, with trailing chunks after IEND | [x] |
| R30 | transform stacking | expand+gray_to_rgb+add_alpha+swap+bgr+16→8 pipelines over every colour type | [x] |
| R31 | `png_create_read_struct_2` + user memory | full read, allocation trace compared | [x] |
| R32 | `png_get_io_state`/`png_get_io_chunk_type` | polled during read via the read callback | [x] |
| R33 | `png_get_current_row_number`/`png_get_current_pass_number` | polled per row for interlace 0/1 | [x] |

## P — progressive read (`pngpread.c`)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| P1 | `png_process_data` | feed sizes 1, 2, 3, 7, 13, 64, whole-file for all colour/depth combos | [x] |
| P2 | `png_process_data` + interlace | Adam7 input, `png_progressive_combine_row` in the row callback | [x] |
| P3 | `png_process_data_pause` | pause with save=0 and save=1 at every chunk boundary | [x] |
| P4 | `png_process_data_skip` | after a paused unknown/large chunk | [x] |
| P5 | progressive + transforms | `png_set_expand`, `gray_to_rgb`, `strip_16`, `packing`, gamma set in the info callback | [x] |
| P6 | progressive + multi-IDAT / zero-length IDAT | IDAT split into 1-byte chunks | [x] |
| P7 | progressive + ancillary chunks | all-chunk image, unknown chunk handling, user chunk callback | [x] |
| P8 | `png_get_progressive_ptr` | pointer round trip | [x] |

## S — simplified API (`pngread.c` / `pngwrite.c` simplified sections)

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| S1 | `png_image_begin_read_from_memory` + `png_image_finish_read` | every input colour type/depth × output format GRAY/GA/AG/RGB/BGR/RGBA/ARGB/BGRA/ABGR | [x] |
| S2 | same, LINEAR formats | LINEAR_Y / LINEAR_Y_ALPHA / LINEAR_RGB / LINEAR_RGB_ALPHA (16-bit output) | [x] |
| S3 | same, COLORMAP formats | RGB/BGR/RGBA/ARGB/BGRA/ABGR + colormap, `colormap_entries` | [x] |
| S4 | `png_image_finish_read` with background | `png_color` background × alpha-bearing input × row_stride negative (bottom-up) | [x] |
| S5 | `png_image_write_to_memory` | every format × `convert_to_8_bit` 0/1 × colormap, two-pass size query (`memory=NULL`) | [x] |
| S6 | `png_image_begin_read_from_stdio` / `png_image_write_to_stdio` / `..._to_file` | real temporary files | [x] |
| S7 | `png_image_free` | after success, after failure, twice | [x] |

## M — cross-cutting state / modes

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| M1 | `png_set_benign_errors(0/1)` | read and write structs, driven into a benign-error condition | [ ] |
| M2 | `png_set_error_fn` with NULL / non-NULL callbacks | `png_get_error_ptr` round trip; default handler path | [ ] |
| M3 | `png_set_mem_fn`, `png_get_mem_ptr` | user memory on read and write, `png_malloc` failure injection | [ ] |
| M4 | `png_set_longjmp_fn` | jmp_buf sizes: equal to `sizeof(jmp_buf)`, smaller, larger (allocated) | [ ] |
| M5 | write → read round trip | every colour/depth/interlace combination, image content preserved | [ ] |
| M6 | `png_read_png`/`png_write_png` paired | all transform bits, round trip | [ ] |
| M7 | `png_handle_as_unknown`, `png_set_keep_unknown_chunks` interaction | every keep value with an all-chunk image | [ ] |
| M8 | `png_free_data` / `png_data_freer` | every mask × freer combination on a fully populated info struct | [ ] |
