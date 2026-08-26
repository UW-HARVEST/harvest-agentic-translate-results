# CONFIGS.md — the configuration surface of libpng (valid inputs)

The mirror image of `ERRORS.md`: every *valid* configuration the C code
distinguishes.  The axes below were derived from what `c_src/src/*.c` actually
branches on — the `png_set_*` entry points declared in `png.h`, the
`png_struct::transformations` / `mode` / `flags` / `color_type` / `bit_depth`
tests those setters feed, and the `switch`/`if` ladders in `pngrtran.c`,
`pngrutil.c`, `pngwutil.c`, `pngwtran.c`, `pngtrans.c`, `pngpread.c` and
`pngread.c`.

Every row is exercised by a differential test that drives **both** shared
objects through their exported symbols and compares the full event trace and
every output byte.  Rows are driven with **many randomised inputs** from a
fixed-seed SplitMix64 (`common::Rng`), never a single hand-picked value.

## Build configurations

`Cargo.toml` has **no `[features]`**, so there is exactly one Rust build
configuration; `c_src/CMakeLists.txt` globs `src/*.c` with no options and the
config header `c_src/include/pnglibconf.h` is checked in with essentially every
`PNG_*_SUPPORTED` enabled.  The complete enumeration of feature combinations is
therefore the single empty combination, verified with
`cargo check --no-default-features` (and again with `--all-features`, which is
the same thing here).

## Axes

| axis | values the C code distinguishes | where |
|------|---------------------------------|-------|
| A. entry-point family | low-level write · low-level sequential read · high-level `png_read_png`/`png_write_png` · progressive read (`png_process_data`) · simplified read (`png_image_*read*`) · simplified write (`png_image_write_*`) · raw chunk writer (`png_write_chunk*`) · info get/set only · pure/util functions · internal (non-`png.h`) exports | `png.h`, `pngpriv.h` |
| B. colour type × bit depth | the 15 legal pairs: GRAY 1/2/4/8/16, PALETTE 1/2/4/8, RGB 8/16, GRAY_ALPHA 8/16, RGB_ALPHA 8/16 | `png_check_IHDR` |
| C. interlace | `PNG_INTERLACE_NONE`, `PNG_INTERLACE_ADAM7` (7 passes, different row/col strides) | `pngread.c`, `pngwutil.c` |
| D. dimensions | 1×1, 1×N, N×1, sub-byte widths that leave trailing bits (w mod 8/4/2 ≠ 0), 8×8, odd (9×5, 17×3), ≥ 32 (multi-buffer IDAT) | `PNG_ROWBYTES`, `png_do_read_interlace` |
| E. write filters | `PNG_NO_FILTERS`, each single filter (NONE/SUB/UP/AVG/PAETH), `PNG_FAST_FILTERS`, `PNG_ALL_FILTERS`, per-row `png_set_filter` changes | `png_write_find_filter` |
| F. zlib knobs | level 0…9, strategy 0…4, mem level 1…9, window bits 8…15, `png_set_compression_method`, `png_set_compression_buffer_size` (tiny → huge), the separate `png_set_text_compression_*` set | `png_deflate_claim` |
| G. read transforms | 22 `png_set_*` transforms + their combinations, order-sensitive (`png_init_read_transformations`, `png_do_read_transformations`) | `pngrtran.c` |
| H. write transforms | `png_set_bgr`, `png_set_swap`, `png_set_packing`, `png_set_packswap`, `png_set_shift`, `png_set_invert_mono`, `png_set_invert_alpha`, `png_set_swap_alpha`, `png_set_filler`(strip) | `pngwtran.c`, `pngtrans.c` |
| I. gamma / colourspace | `png_set_gamma`, `png_set_alpha_mode` × 4 modes, `png_set_background` × 4 gamma codes, `png_set_rgb_to_gray` × 3 error actions, `gAMA`/`sRGB`/`iCCP`/`cHRM` present or absent | `pngrtran.c`, `png.c` colourspace |
| J. ancillary chunks | gAMA cHRM sRGB iCCP sBIT bKGD hIST tRNS pHYs oFFs tIME pCAL sCAL sPLT tEXt zTXt iTXt eXIf cICP cLLI mDCV + unknown chunks, each absent / present / present-twice, before and after IDAT | `pngset.c`, `pngwutil.c`, `pngrutil.c` |
| K. unknown-chunk policy | `png_set_keep_unknown_chunks` × {AS_DEFAULT, NEVER, IF_SAFE, ALWAYS} × {all chunks, named list} × chunk critical/ancillary/private/safe-to-copy | `png_handle_unknown` |
| L. CRC policy | `png_set_crc_action` crit × ancil ∈ {DEFAULT, ERROR_QUIT, WARN_DISCARD, WARN_USE, QUIET_USE, NO_CHANGE} | `png_crc_finish` |
| M. user limits | `png_set_user_limits`, `png_set_chunk_cache_max`, `png_set_chunk_malloc_max` at/above/below the actual image | `png_check_IHDR`, `png_handle_unknown` |
| N. callbacks | custom `png_set_mem_fn` allocator, `png_set_read_status_fn`/`write_status_fn`, `png_set_read_user_transform_fn`/`write_user_transform_fn` (+ `png_set_user_transform_info`), `png_set_read_user_chunk_fn`, `png_set_flush` | `pngmem.c`, `pngtrans.c`, `pngrutil.c` |
| O. options | `png_set_option` × 5 valid options × {ON, OFF} (notably `PNG_MAXIMUM_INFLATE_WINDOW`, `PNG_SKIP_sRGB_CHECK_PROFILE`, `PNG_IGNORE_ADLER32`) | `png_set_option` |
| P. MNG | `png_permit_mng_features` × {0, EMPTY_PLTE, FILTER_64, ALL} with `PNG_INTRAPIXEL_DIFFERENCING` | `png_do_read_intrapixel`, `png.c` |
| Q. benign errors | `png_set_benign_errors(0/1)` on a read struct and on a write struct | `pngerror.c` |
| R. progressive chunking | feed 1, 2, 3, 5, 13, 100, 8192, whole-file byte counts; `png_process_data_pause`/`png_process_data_skip` | `pngpread.c` |
| S. simplified formats | the 8 8-bit formats × {plain, `_COLORMAP`}, the 4 `LINEAR` formats, `PNG_IMAGE_FLAG_FAST`, `..._16BIT_sRGB`, background supplied or not, `row_stride` positive/negative | `png_image_read_*`, `png_image_write_*` |
| T. byte order / packing | `png_set_swap` on 16-bit, `png_set_packswap` on 1/2/4-bit, `png_set_bgr`, filler before/after, `png_set_add_alpha` | `pngtrans.c` |

Rows below are the cross-product pruned to the combinations the C code actually
treats differently.

| # | entry point(s) | configuration (options set + input shape) | [ ] |
|---|----------------|--------------------------------------------|-----|
| C-1 | `png_get_uint_32, png_get_uint_16, png_get_int_32, png_get_uint_31, png_save_uint_32, png_save_uint_16, png_save_int_32` | random 4-byte buffers, incl. high bit set (negative png_int_32), 0, 0xffffffff; png_get_uint_31 with and without a png_ptr | [x] |
| C-2 | `png_sig_cmp` | every (start, num_to_check) pair in 0..9 x 0..9, over the true signature, prefixes of it and random bytes | [x] |
| C-3 | `png_muldiv, png_muldiv_warn` | random (times, amount, divisor) triples: divisor 0, +-1, huge; products that overflow the 32-bit intermediate; exact and inexact rounding | [x] |
| C-4 | `png_reciprocal, png_reciprocal2` | random fixed-point args incl. 0, 1, PNG_FP_1, PNG_FP_MAX, negatives | [x] |
| C-5 | `png_fixed, png_fixed_ITU` | random doubles: in range, at +-2147483647/1e5, out of range (fatal), extremes | [x] |
| C-6 | `png_gamma_significant, png_gamma_8bit_correct, png_gamma_16bit_correct, png_gamma_correct` | gamma in {0, 1e-5, 0.5, 1.0-eps, 1.0, 1.0+eps, 2.2, 45455, PNG_FP_MAX} x every 8-bit value and 2048 random 16-bit values | [x] |
| C-7 | `png_build_gamma_table / png_destroy_gamma_table (via png_read_update_info)` | bit_depth 8 and 16 x file gamma x screen gamma pairs incl. equal and unity; the 16-bit table path and the 8-bit table path | [x] |
| C-8 | `png_XYZ_from_xy, png_xy_from_XYZ` | random chromaticities: valid sRGB primaries, degenerate (all equal), zero/negative, sums > 1, PNG_FP_MAX | [x] |
| C-9 | `png_check_fp_number, png_check_fp_string` | random ASCII strings over "0123456789+-.eE ", every prefix length; well-formed and malformed floats | [x] |
| C-10 | `png_ascii_from_fp, png_ascii_from_fixed` | random doubles / fixed values x buffer sizes from too-small to generous; precision 1..DBL_DIG+1 | [x] |
| C-11 | `png_safecat, png_format_number` | random source strings x buffer sizes x every format (PNG_NUMBER_FORMAT_u / 02u / d / 02d / x / 02x / fixed) | [x] |
| C-12 | `png_reset_crc, png_calculate_crc, png_get_io_chunk_type` | random chunk data of length 0..4096 fed in 1..n pieces, with CRC checking enabled and disabled | [x] |
| C-13 | `png_do_bgr, png_do_invert, png_do_packswap, png_do_swap, png_do_strip_channel` | every (colour type, bit depth) row_info x widths 1..17 x random row bytes; strip_channel at_start 0 and 1 | [x] |
| C-14 | `png_read_filter_row` | filter values NONE/SUB/UP/AVG/PAETH x pixel_depth 1..64 (bpp 1..8) x random rows and prev_rows x widths 1..33 | [x] |
| C-15 | `png_write_find_filter` | every filter mask 0x00..0xf8 x every colour type / bit depth x random rows, first row and later rows | [x] |
| C-16 | `png_combine_row` | display 0 and 1 x pass 0..6 x pixel_depth 1..64 x random source rows and pre-filled destinations | [x] |
| C-17 | `png_do_read_interlace, png_do_write_interlace` | pass 0..6 x every bit depth x widths 1..33 x random rows; transformations with and without PNG_PACK | [x] |
| C-18 | `png_check_IHDR` | the 15 legal (colour type, bit depth) pairs x interlace 0/1 x widths/heights 1, 7, 8, 1000000 x filter method 0/64 with and without MNG permission | [x] |
| C-19 | `png_check_keyword` | random keywords: empty, 1..90 chars, leading/trailing/multiple spaces, control chars, 8-bit chars, exactly 79 and 80 chars | [x] |
| C-20 | `png_zstream_error, png_reset_zstream` | every zlib return code -6..2 with and without a zstream message, on read and write structs | [x] |
| C-21 | `png_icc_check_header, png_icc_check_length, png_icc_check_tag_table (via png_set_iCCP and the iCCP chunk)` | synthetic ICC profiles: correct sRGB profile, wrong length, bad signature, bad tag table, 0 tags, huge tag count; PNG_SKIP_sRGB_CHECK_PROFILE on/off | [x] |
| C-22 | `png_do_check_palette_indexes, png_get_palette_max` | palette sizes 1..256 x bit depths 1/2/4/8 x rows whose indices are inside and outside the palette; check_for_invalid_index on/off | [x] |
| C-23 | `png_malloc, png_calloc, png_free, png_malloc_warn, png_malloc_base, png_malloc_array, png_realloc_array, png_free_data` | sizes 0, 1, 8, 4096, PNG_SIZE_MAX; array counts 0/1/many with old arrays; with and without a custom png_set_mem_fn allocator | [x] |
| C-24 | `png_convert_to_rfc1123_buffer, png_convert_from_time_t, png_convert_from_struct_tm, png_convert_to_rfc1123` | random png_time values (valid and out-of-range month/day/hour/min/sec), time_t 0 / now / 2^31 / 2^32, struct tm from gmtime | [x] |
| C-25 | `png_access_version_number, png_get_libpng_ver, png_get_header_ver, png_get_header_version, png_get_copyright` | with png_ptr NULL and non-NULL | [x] |
| C-26 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 1, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-27 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 1, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-28 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 2, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-29 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 2, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-30 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 4, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-31 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 4, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-32 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 8, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-33 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 8, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-34 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 16, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-35 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY, bit depth 16, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-36 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 1, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-37 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 1, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-38 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 2, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-39 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 2, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-40 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 4, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-41 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 4, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-42 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 8, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-43 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type PALETTE, bit depth 8, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-44 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB, bit depth 8, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-45 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB, bit depth 8, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-46 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB, bit depth 16, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-47 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB, bit depth 16, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-48 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY_ALPHA, bit depth 8, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-49 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY_ALPHA, bit depth 8, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-50 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY_ALPHA, bit depth 16, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-51 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type GRAY_ALPHA, bit depth 16, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-52 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB_ALPHA, bit depth 8, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-53 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB_ALPHA, bit depth 8, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-54 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB_ALPHA, bit depth 16, interlace NONE; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-55 | `png_create_write_struct, png_set_IHDR, png_write_info, png_write_row, png_write_end + png_create_read_struct, png_read_info, png_read_row, png_read_end` | colour type RGB_ALPHA, bit depth 16, interlace ADAM7; randomised width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel data; random filter mask and zlib level per iteration | [x] |
| C-56 | `png_write_rows, png_write_image, png_read_rows, png_read_image` | the same matrix driven through the bulk row entry points instead of png_write_row / png_read_row, incl. NULL display_row and NULL row arguments | [x] |
| C-57 | `png_set_filter` | every mask: NO_FILTERS, NONE, SUB, UP, AVG, PAETH, FAST_FILTERS, ALL_FILTERS, and the mask changed between rows | [x] |
| C-58 | `png_set_compression_level, png_set_compression_strategy, png_set_compression_mem_level, png_set_compression_window_bits, png_set_compression_method` | level -1..9 x strategy 0..4 x mem level 1..9 x window bits 8..15 (plus the out-of-range values libpng clamps or warns about) | [x] |
| C-59 | `png_set_compression_buffer_size, png_get_compression_buffer_size` | buffer sizes 1, 2, 3, 8, 1024, 8192, 65536 against images larger than one buffer | [x] |
| C-60 | `png_set_text_compression_level, png_set_text_compression_strategy, png_set_text_compression_mem_level, png_set_text_compression_window_bits, png_set_text_compression_method` | the same ranges, observed through a compressed zTXt / iTXt / iCCP payload | [x] |
| C-61 | `png_write_sig, png_write_chunk, png_write_chunk_start, png_write_chunk_data, png_write_chunk_end` | raw chunk writing: chunk names critical / ancillary / private / reserved x payload length 0, 1, 8191, 8192, 8193 written in 1..n pieces | [x] |
| C-62 | `png_set_flush, png_write_flush` | flush every 1, 2, 7 rows and never; interacts with png_write_row and the IDAT buffer | [x] |
| C-63 | `png_get_rowbytes, png_get_channels, png_get_IHDR, png_get_image_width, png_get_image_height, png_get_bit_depth, png_get_color_type, png_get_interlace_type, png_get_compression_type, png_get_filter_type` | read back after png_read_info and after png_read_update_info for every shape | [x] |
| C-64 | `png_get_io_state, png_get_io_chunk_type, png_init_io, png_get_io_ptr` | IO state sampled from the read and write callbacks at every chunk boundary; png_init_io with a real FILE* (tmpfile) | [x] |
| C-65 | `png_set_invalid, png_get_valid` | every PNG_INFO_* bit invalidated singly and in combination before png_write_info | [x] |
| C-66 | `png_set_sig_bytes, png_get_signature` | signature already consumed by the app: 0..8 bytes pre-read | [x] |
| C-67 | `png_set_palette_to_rgb (then png_read_update_info, png_read_image)` | palette 1/2/4/8-bit x with and without tRNS | [x] |
| C-68 | `png_set_expand_gray_1_2_4_to_8 (then png_read_update_info, png_read_image)` | gray 1/2/4-bit | [x] |
| C-69 | `png_set_tRNS_to_alpha (then png_read_update_info, png_read_image)` | every colour type with a tRNS chunk present and absent | [x] |
| C-70 | `png_set_expand (then png_read_update_info, png_read_image)` | all colour types; combined with tRNS and bKGD | [x] |
| C-71 | `png_set_expand_16 (then png_read_update_info, png_read_image)` | 8-bit and 16-bit inputs, with and without png_set_expand | [x] |
| C-72 | `png_set_strip_16 (then png_read_update_info, png_read_image)` | 16-bit inputs of every colour type | [x] |
| C-73 | `png_set_scale_16 (then png_read_update_info, png_read_image)` | 16-bit inputs of every colour type | [x] |
| C-74 | `png_set_strip_alpha (then png_read_update_info, png_read_image)` | GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit | [x] |
| C-75 | `png_set_swap_alpha (then png_read_update_info, png_read_image)` | GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit | [x] |
| C-76 | `png_set_invert_alpha (then png_read_update_info, png_read_image)` | GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit | [x] |
| C-77 | `png_set_filler (then png_read_update_info, png_read_image)` | filler value 0 / 0xff / random x PNG_FILLER_BEFORE/AFTER x GRAY and RGB, 8 and 16 bit | [x] |
| C-78 | `png_set_add_alpha (then png_read_update_info, png_read_image)` | filler value x BEFORE/AFTER x GRAY and RGB, 8 and 16 bit | [x] |
| C-79 | `png_set_bgr (then png_read_update_info, png_read_image)` | RGB and RGB_ALPHA, 8 and 16 bit | [x] |
| C-80 | `png_set_swap (then png_read_update_info, png_read_image)` | 16-bit inputs of every colour type | [x] |
| C-81 | `png_set_packing (then png_read_update_info, png_read_image)` | 1/2/4-bit gray and palette | [x] |
| C-82 | `png_set_packswap (then png_read_update_info, png_read_image)` | 1/2/4-bit gray and palette | [x] |
| C-83 | `png_set_shift (then png_read_update_info, png_read_image)` | random sBIT values <= bit depth for every colour type; sBIT chunk present and absent | [x] |
| C-84 | `png_set_invert_mono (then png_read_update_info, png_read_image)` | 1-bit and 8-bit gray, and non-gray input (no-op) | [x] |
| C-85 | `png_set_gray_to_rgb (then png_read_update_info, png_read_image)` | GRAY and GRAY_ALPHA, every bit depth | [x] |
| C-86 | `png_set_rgb_to_gray / png_set_rgb_to_gray_fixed (then png_read_update_info, png_read_image)` | error action NONE/WARN/ERROR x default and explicit red/green coefficients x RGB, RGB_ALPHA, PALETTE; cHRM present and absent | [x] |
| C-87 | `png_set_quantize (then png_read_update_info, png_read_image)` | palette and RGB input x num_palette 1..256 x maximum_colors 1..256 x histogram supplied and not x full_quantize 0/1 | [x] |
| C-88 | `png_set_background / png_set_background_fixed (then png_read_update_info, png_read_image)` | background gamma code UNKNOWN/SCREEN/FILE/UNIQUE x need_expand 0/1 x random png_color_16 x every colour type with and without alpha | [x] |
| C-89 | `png_set_alpha_mode / png_set_alpha_mode_fixed (then png_read_update_info, png_read_image)` | mode PNG_ALPHA_PNG/STANDARD/OPTIMIZED/BROKEN x screen gamma {1.0, 2.2, 0.45455, PNG_FP_1} | [x] |
| C-90 | `png_set_gamma / png_set_gamma_fixed (then png_read_update_info, png_read_image)` | screen gamma x file gamma pairs incl. equal, unity and extreme; gAMA / sRGB present and absent | [x] |
| C-91 | `png_set_interlace_handling (then png_read_update_info, png_read_image)` | interlaced and non-interlaced input; the returned pass count | [x] |
| C-92 | `png_set_read_user_transform_fn + png_set_user_transform_info (then png_read_update_info, png_read_image)` | user transform that rewrites the row, with user bit depth / channels overridden | [x] |
| C-93 | `png_set_check_for_invalid_index (then png_read_update_info, png_read_image)` | palette images with in-range and out-of-range indices, on and off | [x] |
| C-94 | `all read transforms` | randomised *combinations* of 2..6 read transforms applied together (in the order libpng resolves them in png_init_read_transformations), over every shape | [x] |
| C-95 | `png_read_update_info, png_read_transform_info (internal)` | called once, twice (libpng warns) and not at all before reading rows | [x] |
| C-96 | `png_set_bgr (write side)` | RGB / RGB_ALPHA, 8 and 16 bit | [x] |
| C-97 | `png_set_swap (write side)` | 16-bit | [x] |
| C-98 | `png_set_packing (write side)` | the app supplies one pixel per byte for 1/2/4-bit output | [x] |
| C-99 | `png_set_packswap (write side)` | 1/2/4-bit | [x] |
| C-100 | `png_set_shift (write side)` | sBIT smaller than the bit depth for every colour type | [x] |
| C-101 | `png_set_invert_mono (write side)` | 1-bit gray | [x] |
| C-102 | `png_set_invert_alpha (write side)` | GRAY_ALPHA / RGB_ALPHA | [x] |
| C-103 | `png_set_swap_alpha (write side)` | GRAY_ALPHA / RGB_ALPHA | [x] |
| C-104 | `png_set_filler (strip filler on write) (write side)` | PNG_FILLER_BEFORE/AFTER on RGB and GRAY output | [x] |
| C-105 | `png_set_write_user_transform_fn + png_set_user_transform_info (write side)` | user transform that rewrites the row before filtering | [x] |
| C-106 | `png_permit_mng_features + PNG_INTRAPIXEL_DIFFERENCING` | MNG features 0 / EMPTY_PLTE / FILTER_64 / ALL x filter method 0 and 64 x RGB and RGB_ALPHA, 8 and 16 bit, read and write | [x] |
| C-107 | `png_set_gAMA, png_set_gAMA_fixed, png_get_gAMA, png_get_gAMA_fixed` | gAMA: gamma 0, 1, 100000, 500000, PNG_FP_MAX and values libpng rejects -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-108 | `png_set_cHRM, png_set_cHRM_fixed, png_set_cHRM_XYZ, png_set_cHRM_XYZ_fixed, png_get_cHRM*` | cHRM: sRGB primaries, degenerate, negative, out of range -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-109 | `png_set_sRGB, png_set_sRGB_gAMA_and_cHRM, png_get_sRGB` | sRGB: intent 0..3 (and 4, which is rejected) -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-110 | `png_set_iCCP, png_get_iCCP` | iCCP: name lengths 1/79/80, profile sizes 132..2048, compression type 0, a real sRGB profile -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-111 | `png_set_sBIT, png_get_sBIT` | sBIT: every colour type x every legal sBIT combination -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-112 | `png_set_bKGD, png_get_bKGD` | bKGD: every colour type x random png_color_16 incl. index >= num_palette -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-113 | `png_set_hIST, png_get_hIST` | hIST: palette sizes 1..256 with a matching histogram -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-114 | `png_set_tRNS, png_get_tRNS` | tRNS: palette (num_trans 1..256), gray, RGB; num_trans 0 and > palette -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-115 | `png_set_pHYs, png_get_pHYs, png_get_pHYs_dpi, png_get_x_pixels_per_meter, png_get_y_pixels_per_meter, png_get_x_pixels_per_inch, png_get_y_pixels_per_inch, png_get_pixels_per_inch, png_get_pixels_per_meter, png_get_pixel_aspect_ratio, png_get_pixel_aspect_ratio_fixed` | pHYs: unit 0/1/2 x random resolutions incl. 0 -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-116 | `png_set_oFFs, png_get_oFFs, png_get_x_offset_pixels, png_get_y_offset_pixels, png_get_x_offset_microns, png_get_y_offset_microns, png_get_x_offset_inches, png_get_y_offset_inches, png_get_x_offset_inches_fixed, png_get_y_offset_inches_fixed` | oFFs: unit 0/1/2 x negative and positive offsets -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-117 | `png_set_tIME, png_get_tIME` | tIME: random valid and invalid png_time values -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-118 | `png_set_pCAL, png_get_pCAL` | pCAL: every equation type 0..3 x nparams 0..8 x random purpose/units/params strings -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-119 | `png_set_sCAL, png_set_sCAL_fixed, png_set_sCAL_s, png_get_sCAL, png_get_sCAL_fixed, png_get_sCAL_s` | sCAL: unit 1/2 x random widths/heights incl. 0 and malformed strings -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-120 | `png_set_sPLT, png_get_sPLT` | sPLT: depth 8 and 16 x nentries 0/1/256 x several palettes at once -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-121 | `png_set_text, png_get_text` | tEXt: keys of length 1..80 x text length 0..4096 x several entries -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-122 | `png_set_text with PNG_TEXT_COMPRESSION_zTXt` | zTXt: compressible and incompressible payloads, length 0..65536 -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-123 | `png_set_text with PNG_ITXT_COMPRESSION_NONE / _zTXt` | iTXt: lang and lang_key empty and populated, UTF-8 payloads -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-124 | `png_set_eXIf_1, png_get_eXIf_1, png_set_eXIf, png_get_eXIf` | eXIf: sizes 0..4096, valid "II"/"MM" headers and garbage -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-125 | `png_set_cICP, png_get_cICP` | cICP: sampled colour primaries / transfer / matrix bytes x video_full_range 0/1/2 -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-126 | `png_set_cLLI, png_set_cLLI_fixed, png_get_cLLI, png_get_cLLI_fixed` | cLLI: maxCLL / maxFALL 0, 1, 10000*PNG_FP_1, PNG_FP_MAX -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-127 | `png_set_mDCV, png_set_mDCV_fixed, png_get_mDCV, png_get_mDCV_fixed` | mDCV: random chromaticities x luminance values -- written, read back and compared, with the chunk absent, present once, and present twice | [x] |
| C-128 | `png_set_unknown_chunks, png_get_unknown_chunks, png_set_unknown_chunk_location, png_set_keep_unknown_chunks, png_handle_as_unknown` | keep AS_DEFAULT/NEVER/IF_SAFE/ALWAYS x chunk list {NULL (all), named} x chunk name critical / ancillary / private / reserved / safe-to-copy x location BEFORE_PLTE / before IDAT / after IDAT x data size 0..1024 | [x] |
| C-129 | `png_set_rows, png_get_rows, png_free_data, png_data_freer` | info rows set by the app, freed by the app (PNG_USER_WILL_FREE_DATA) and by libpng (PNG_DESTROY_WILL_FREE_DATA); every PNG_FREE_* mask | [x] |
| C-130 | `png_set_text_2 (internal), png_set_text` | num_text 0, 1, many; the text realloc path (> 8 entries); compression values -3..2 and out of range | [x] |
| C-131 | `png_read_png` | every transform flag valid on read, singly and in random combinations, over every shape; params NULL | [x] |
| C-132 | `png_write_png` | every transform flag valid on write, singly and in random combinations, over every shape | [x] |
| C-133 | `png_read_png + png_write_png` | round trip: read with T, write with T, over random shapes | [x] |
| C-134 | `png_set_progressive_read_fn, png_process_data, png_progressive_combine_row, png_get_progressive_ptr` | feed the file in fixed chunks of 1, 2, 3, 5, 13, 100, 1024, 8192 and all-at-once, x every shape x interlace NONE/ADAM7 | [x] |
| C-135 | `png_process_data_pause, png_process_data_skip` | pause with save 0 and 1 at every chunk boundary; skip after IDAT | [x] |
| C-136 | `png_push_read_chunk, png_push_read_IDAT, png_push_save_buffer, png_push_restore_buffer, png_push_fill_buffer (internal, via png_process_data)` | buffer save/restore boundaries: chunk headers split across feeds, IDAT split mid-row, zero-length feeds | [x] |
| C-137 | `progressive read with transforms` | the read transforms of the rows above, applied to a progressive reader | [x] |
| C-138 | `png_image_begin_read_from_memory, png_image_finish_read, png_image_free` | every output format: GRAY GA AG RGB BGR RGBA ARGB BGRA ABGR, each plain and _COLORMAP, plus the 4 LINEAR formats; flags 0 / FAST / 16BIT_sRGB; background supplied and NULL; row_stride positive, negative and 0 | [x] |
| C-139 | `png_image_begin_read_from_stdio, png_image_begin_read_from_file` | the same via a real FILE* and a real path | [x] |
| C-140 | `png_image_write_to_memory, png_image_write_to_stdio, png_image_write_to_file, png_image_write_to_memory (size query)` | every input format x convert_to_8bit 0/1 x colormap present/absent x row_stride positive/negative x memory buffer exactly / too small / NULL | [x] |
| C-141 | `png_image_write_to_memory + png_image_begin_read_from_memory` | round trip through every format | [x] |
| C-142 | `png_set_crc_action` | crit x ancil over all 36 combinations, on a file with a corrupted ancillary CRC, a corrupted critical CRC and correct CRCs | [x] |
| C-143 | `png_set_user_limits, png_get_user_width_max, png_get_user_height_max, png_set_chunk_cache_max, png_get_chunk_cache_max, png_set_chunk_malloc_max, png_get_chunk_malloc_max` | limits below / equal to / above the image dimensions and the chunk sizes; 0 (= unlimited) | [x] |
| C-144 | `png_set_mem_fn, png_get_mem_ptr, png_create_read_struct_2, png_create_write_struct_2` | custom allocator that records every allocation; also an allocator that fails after N allocations | [x] |
| C-145 | `png_set_error_fn, png_get_error_ptr, png_set_benign_errors, png_error, png_warning, png_app_error, png_app_warning, png_benign_error, png_chunk_error, png_chunk_warning, png_chunk_benign_error, png_chunk_report, png_formatted_warning, png_warning_parameter, png_warning_parameter_signed, png_warning_parameter_unsigned` | benign errors allowed and not, on a read struct and a write struct, with app warnings/errors reported as warnings and as errors | [x] |
| C-146 | `png_set_read_status_fn, png_set_write_status_fn` | row/pass callbacks over interlaced and non-interlaced images | [x] |
| C-147 | `png_set_read_user_chunk_fn, png_get_user_chunk_ptr` | user chunk callback returning -1 (error), 0 (unhandled) and 1 (handled), on ancillary and critical unknown chunks | [x] |
| C-148 | `png_set_option` | every option 0..PNG_OPTION_NEXT (and 2 past the end) x PNG_OPTION_ON/OFF/other, checking the returned previous state | [x] |
| C-149 | `png_get_current_row_number, png_get_current_pass_number` | sampled from the row callback for interlaced and non-interlaced reads | [x] |
| C-150 | `png_set_longjmp_fn, png_longjmp, png_free_jmpbuf (internal)` | jmp_buf_size equal to, smaller than and larger than sizeof(jmp_buf); called twice; NULL longjmp_fn | [x] |
| C-151 | `png_info_init_3, png_create_info_struct, png_destroy_info_struct, png_destroy_read_struct, png_destroy_write_struct, png_destroy_png_struct, png_create_png_struct` | sizes equal / smaller / larger than sizeof(png_info); double destroy; destroy with NULL out-parameters | [x] |
| C-152 | `png_build_grayscale_palette` | bit depth 1, 2, 4, 8 (and the invalid 3 and 16) into a 256-entry palette | [x] |
| C-153 | `png_write_info_before_PLTE, png_write_info, png_write_end with info NULL` | chunks emitted before PLTE vs after; png_write_end(png, NULL) | [x] |
| C-154 | `png_read_info, png_read_end with info NULL, png_read_update_info` | png_read_end(png, NULL) and with an info struct; unread rows at png_read_end | [x] |

## Row → test mapping

| # | test that covers it |
|---|---------------------|
| C-1 | `lowlevel::byte_accessors` |
| C-2 | `lowlevel::sig_cmp` |
| C-3 | `lowlevel::muldiv` |
| C-4 | `lowlevel::reciprocal` |
| C-5 | `lowlevel::fixed` |
| C-6 | `lowlevel::gamma_scalar` |
| C-7 | `transforms::gamma_tables` |
| C-8 | `lowlevel::xyz_xy` |
| C-9 | `lowlevel::fp_parse` |
| C-10 | `lowlevel::ascii_from` |
| C-11 | `lowlevel::safecat_format` |
| C-12 | `lowlevel::crc` |
| C-13 | `lowlevel::row_ops` |
| C-14 | `lowlevel::read_filter_row` |
| C-15 | `lowlevel::write_find_filter` |
| C-16 | `lowlevel::combine_row` |
| C-17 | `lowlevel::interlace_row` |
| C-18 | `lowlevel::check_ihdr` |
| C-19 | `lowlevel::check_keyword` |
| C-20 | `lowlevel::zstream_error` |
| C-21 | `chunks::iccp` |
| C-22 | `lowlevel::palette_indexes` |
| C-23 | `misc::memory` |
| C-24 | `lowlevel::time_conv` |
| C-25 | `smoke::version_numbers_match` |
| C-26 | `write_read::matrix` |
| C-27 | `write_read::matrix` |
| C-28 | `write_read::matrix` |
| C-29 | `write_read::matrix` |
| C-30 | `write_read::matrix` |
| C-31 | `write_read::matrix` |
| C-32 | `write_read::matrix` |
| C-33 | `write_read::matrix` |
| C-34 | `write_read::matrix` |
| C-35 | `write_read::matrix` |
| C-36 | `write_read::matrix` |
| C-37 | `write_read::matrix` |
| C-38 | `write_read::matrix` |
| C-39 | `write_read::matrix` |
| C-40 | `write_read::matrix` |
| C-41 | `write_read::matrix` |
| C-42 | `write_read::matrix` |
| C-43 | `write_read::matrix` |
| C-44 | `write_read::matrix` |
| C-45 | `write_read::matrix` |
| C-46 | `write_read::matrix` |
| C-47 | `write_read::matrix` |
| C-48 | `write_read::matrix` |
| C-49 | `write_read::matrix` |
| C-50 | `write_read::matrix` |
| C-51 | `write_read::matrix` |
| C-52 | `write_read::matrix` |
| C-53 | `write_read::matrix` |
| C-54 | `write_read::matrix` |
| C-55 | `write_read::matrix` |
| C-56 | `write_read::bulk_rows` |
| C-57 | `write_read::filters` |
| C-58 | `write_read::zlib_knobs` |
| C-59 | `write_read::buffer_size` |
| C-60 | `chunks::text_compression` |
| C-61 | `write_read::raw_chunks` |
| C-62 | `write_read::flush` |
| C-63 | `write_read::info_getters` |
| C-64 | `misc::io_state` |
| C-65 | `chunks::set_invalid` |
| C-66 | `write_read::sig_bytes` |
| C-67 | `transforms::single` |
| C-68 | `transforms::single` |
| C-69 | `transforms::single` |
| C-70 | `transforms::single` |
| C-71 | `transforms::single` |
| C-72 | `transforms::single` |
| C-73 | `transforms::single` |
| C-74 | `transforms::single` |
| C-75 | `transforms::single` |
| C-76 | `transforms::single` |
| C-77 | `transforms::single` |
| C-78 | `transforms::single` |
| C-79 | `transforms::single` |
| C-80 | `transforms::single` |
| C-81 | `transforms::single` |
| C-82 | `transforms::single` |
| C-83 | `transforms::single` |
| C-84 | `transforms::single` |
| C-85 | `transforms::single` |
| C-86 | `transforms::single` |
| C-87 | `transforms::single` |
| C-88 | `transforms::single` |
| C-89 | `transforms::single` |
| C-90 | `transforms::single` |
| C-91 | `transforms::single` |
| C-92 | `transforms::single` |
| C-93 | `transforms::single` |
| C-94 | `transforms::combinations` |
| C-95 | `transforms::update_info` |
| C-96 | `transforms::write_side` |
| C-97 | `transforms::write_side` |
| C-98 | `transforms::write_side` |
| C-99 | `transforms::write_side` |
| C-100 | `transforms::write_side` |
| C-101 | `transforms::write_side` |
| C-102 | `transforms::write_side` |
| C-103 | `transforms::write_side` |
| C-104 | `transforms::write_side` |
| C-105 | `transforms::write_side` |
| C-106 | `transforms::mng_intrapixel` |
| C-107 | `chunks::round_trip` |
| C-108 | `chunks::round_trip` |
| C-109 | `chunks::round_trip` |
| C-110 | `chunks::round_trip` |
| C-111 | `chunks::round_trip` |
| C-112 | `chunks::round_trip` |
| C-113 | `chunks::round_trip` |
| C-114 | `chunks::round_trip` |
| C-115 | `chunks::round_trip` |
| C-116 | `chunks::round_trip` |
| C-117 | `chunks::round_trip` |
| C-118 | `chunks::round_trip` |
| C-119 | `chunks::round_trip` |
| C-120 | `chunks::round_trip` |
| C-121 | `chunks::round_trip` |
| C-122 | `chunks::round_trip` |
| C-123 | `chunks::round_trip` |
| C-124 | `chunks::round_trip` |
| C-125 | `chunks::round_trip` |
| C-126 | `chunks::round_trip` |
| C-127 | `chunks::round_trip` |
| C-128 | `chunks::unknown` |
| C-129 | `chunks::rows_and_freer` |
| C-130 | `chunks::text_many` |
| C-131 | `highlevel::read_png` |
| C-132 | `highlevel::write_png` |
| C-133 | `highlevel::round_trip` |
| C-134 | `progressive::chunk_sizes` |
| C-135 | `progressive::pause_skip` |
| C-136 | `progressive::split_boundaries` |
| C-137 | `progressive::transforms` |
| C-138 | `simplified::read_formats` |
| C-139 | `simplified::read_stdio` |
| C-140 | `simplified::write_formats` |
| C-141 | `simplified::round_trip` |
| C-142 | `misc::crc_action` |
| C-143 | `misc::user_limits` |
| C-144 | `misc::custom_alloc` |
| C-145 | `errors::reporting_matrix` |
| C-146 | `misc::status_callbacks` |
| C-147 | `chunks::user_chunk_fn` |
| C-148 | `misc::options` |
| C-149 | `misc::row_number` |
| C-150 | `misc::longjmp_fn` |
| C-151 | `misc::struct_lifecycle` |
| C-152 | `misc::grayscale_palette` |
| C-153 | `chunks::write_order` |
| C-154 | `write_read::read_end` |
