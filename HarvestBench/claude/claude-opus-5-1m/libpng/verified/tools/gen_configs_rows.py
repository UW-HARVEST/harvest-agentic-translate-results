#!/usr/bin/env python3
"""Append the configuration-surface rows to CONFIGS.md (see the header of that
file for how the axes were derived)."""
rows = []


def R(entry, cfg, test):
    rows.append((entry, cfg, test))


# ---------------- lowlevel / pure -------------------------------------------
R('png_get_uint_32, png_get_uint_16, png_get_int_32, png_get_uint_31, '
  'png_save_uint_32, png_save_uint_16, png_save_int_32',
  'random 4-byte buffers, incl. high bit set (negative png_int_32), 0, '
  '0xffffffff; png_get_uint_31 with and without a png_ptr',
  'lowlevel::byte_accessors')
R('png_sig_cmp',
  'every (start, num_to_check) pair in 0..9 x 0..9, over the true signature, '
  'prefixes of it and random bytes', 'lowlevel::sig_cmp')
R('png_muldiv, png_muldiv_warn',
  'random (times, amount, divisor) triples: divisor 0, +-1, huge; products '
  'that overflow the 32-bit intermediate; exact and inexact rounding',
  'lowlevel::muldiv')
R('png_reciprocal, png_reciprocal2',
  'random fixed-point args incl. 0, 1, PNG_FP_1, PNG_FP_MAX, negatives',
  'lowlevel::reciprocal')
R('png_fixed, png_fixed_ITU',
  'random doubles: in range, at +-2147483647/1e5, out of range (fatal), extremes',
  'lowlevel::fixed')
R('png_gamma_significant, png_gamma_8bit_correct, png_gamma_16bit_correct, '
  'png_gamma_correct',
  'gamma in {0, 1e-5, 0.5, 1.0-eps, 1.0, 1.0+eps, 2.2, 45455, PNG_FP_MAX} x '
  'every 8-bit value and 2048 random 16-bit values', 'lowlevel::gamma_scalar')
R('png_build_gamma_table / png_destroy_gamma_table (via png_read_update_info)',
  'bit_depth 8 and 16 x file gamma x screen gamma pairs incl. equal and unity; '
  'the 16-bit table path and the 8-bit table path',
  'transforms::gamma_tables')
R('png_XYZ_from_xy, png_xy_from_XYZ',
  'random chromaticities: valid sRGB primaries, degenerate (all equal), '
  'zero/negative, sums > 1, PNG_FP_MAX', 'lowlevel::xyz_xy')
R('png_check_fp_number, png_check_fp_string',
  'random ASCII strings over "0123456789+-.eE ", every prefix length; '
  'well-formed and malformed floats', 'lowlevel::fp_parse')
R('png_ascii_from_fp, png_ascii_from_fixed',
  'random doubles / fixed values x buffer sizes from too-small to generous; '
  'precision 1..DBL_DIG+1', 'lowlevel::ascii_from')
R('png_safecat, png_format_number',
  'random source strings x buffer sizes x every format '
  '(PNG_NUMBER_FORMAT_u / 02u / d / 02d / x / 02x / fixed)',
  'lowlevel::safecat_format')
R('png_reset_crc, png_calculate_crc, png_get_io_chunk_type',
  'random chunk data of length 0..4096 fed in 1..n pieces, with CRC checking '
  'enabled and disabled', 'lowlevel::crc')
R('png_do_bgr, png_do_invert, png_do_packswap, png_do_swap, png_do_strip_channel',
  'every (colour type, bit depth) row_info x widths 1..17 x random row bytes; '
  'strip_channel at_start 0 and 1', 'lowlevel::row_ops')
R('png_read_filter_row',
  'filter values NONE/SUB/UP/AVG/PAETH x pixel_depth 1..64 (bpp 1..8) x random '
  'rows and prev_rows x widths 1..33', 'lowlevel::read_filter_row')
R('png_write_find_filter',
  'every filter mask 0x00..0xf8 x every colour type / bit depth x random rows, '
  'first row and later rows', 'lowlevel::write_find_filter')
R('png_combine_row',
  'display 0 and 1 x pass 0..6 x pixel_depth 1..64 x random source rows and '
  'pre-filled destinations', 'lowlevel::combine_row')
R('png_do_read_interlace, png_do_write_interlace',
  'pass 0..6 x every bit depth x widths 1..33 x random rows; transformations '
  'with and without PNG_PACK', 'lowlevel::interlace_row')
R('png_check_IHDR',
  'the 15 legal (colour type, bit depth) pairs x interlace 0/1 x '
  'widths/heights 1, 7, 8, 1000000 x filter method 0/64 with and without MNG '
  'permission', 'lowlevel::check_ihdr')
R('png_check_keyword',
  'random keywords: empty, 1..90 chars, leading/trailing/multiple spaces, '
  'control chars, 8-bit chars, exactly 79 and 80 chars',
  'lowlevel::check_keyword')
R('png_zstream_error, png_reset_zstream',
  'every zlib return code -6..2 with and without a zstream message, on read '
  'and write structs', 'lowlevel::zstream_error')
R('png_icc_check_header, png_icc_check_length, png_icc_check_tag_table '
  '(via png_set_iCCP and the iCCP chunk)',
  'synthetic ICC profiles: correct sRGB profile, wrong length, bad signature, '
  'bad tag table, 0 tags, huge tag count; PNG_SKIP_sRGB_CHECK_PROFILE on/off',
  'chunks::iccp')
R('png_do_check_palette_indexes, png_get_palette_max',
  'palette sizes 1..256 x bit depths 1/2/4/8 x rows whose indices are inside '
  'and outside the palette; check_for_invalid_index on/off',
  'lowlevel::palette_indexes')
R('png_malloc, png_calloc, png_free, png_malloc_warn, png_malloc_base, '
  'png_malloc_array, png_realloc_array, png_free_data',
  'sizes 0, 1, 8, 4096, PNG_SIZE_MAX; array counts 0/1/many with old arrays; '
  'with and without a custom png_set_mem_fn allocator', 'misc::memory')
R('png_convert_to_rfc1123_buffer, png_convert_from_time_t, '
  'png_convert_from_struct_tm, png_convert_to_rfc1123',
  'random png_time values (valid and out-of-range month/day/hour/min/sec), '
  'time_t 0 / now / 2^31 / 2^32, struct tm from gmtime', 'lowlevel::time_conv')
R('png_access_version_number, png_get_libpng_ver, png_get_header_ver, '
  'png_get_header_version, png_get_copyright',
  'with png_ptr NULL and non-NULL', 'smoke::version_numbers_match')

# ---------------- low-level write x read ------------------------------------
NAMES = {0: 'GRAY', 3: 'PALETTE', 2: 'RGB', 4: 'GRAY_ALPHA', 6: 'RGB_ALPHA'}
DEPTHS = {0: [1, 2, 4, 8, 16], 3: [1, 2, 4, 8], 2: [8, 16], 4: [8, 16],
          6: [8, 16]}
for ct in (0, 3, 2, 4, 6):
    for d in DEPTHS[ct]:
        for il, iln in ((0, 'NONE'), (1, 'ADAM7')):
            R('png_create_write_struct, png_set_IHDR, png_write_info, '
              'png_write_row, png_write_end + png_create_read_struct, '
              'png_read_info, png_read_row, png_read_end',
              'colour type %s, bit depth %d, interlace %s; randomised '
              'width/height from {1,2,3,5,7,8,9,15,16,17,33} and random pixel '
              'data; random filter mask and zlib level per iteration'
              % (NAMES[ct], d, iln), 'write_read::matrix')
R('png_write_rows, png_write_image, png_read_rows, png_read_image',
  'the same matrix driven through the bulk row entry points instead of '
  'png_write_row / png_read_row, incl. NULL display_row and NULL row arguments',
  'write_read::bulk_rows')
R('png_set_filter',
  'every mask: NO_FILTERS, NONE, SUB, UP, AVG, PAETH, FAST_FILTERS, '
  'ALL_FILTERS, and the mask changed between rows', 'write_read::filters')
R('png_set_compression_level, png_set_compression_strategy, '
  'png_set_compression_mem_level, png_set_compression_window_bits, '
  'png_set_compression_method',
  'level -1..9 x strategy 0..4 x mem level 1..9 x window bits 8..15 (plus the '
  'out-of-range values libpng clamps or warns about)',
  'write_read::zlib_knobs')
R('png_set_compression_buffer_size, png_get_compression_buffer_size',
  'buffer sizes 1, 2, 3, 8, 1024, 8192, 65536 against images larger than one '
  'buffer', 'write_read::buffer_size')
R('png_set_text_compression_level, png_set_text_compression_strategy, '
  'png_set_text_compression_mem_level, png_set_text_compression_window_bits, '
  'png_set_text_compression_method',
  'the same ranges, observed through a compressed zTXt / iTXt / iCCP payload',
  'chunks::text_compression')
R('png_write_sig, png_write_chunk, png_write_chunk_start, '
  'png_write_chunk_data, png_write_chunk_end',
  'raw chunk writing: chunk names critical / ancillary / private / reserved x '
  'payload length 0, 1, 8191, 8192, 8193 written in 1..n pieces',
  'write_read::raw_chunks')
R('png_set_flush, png_write_flush',
  'flush every 1, 2, 7 rows and never; interacts with png_write_row and the '
  'IDAT buffer', 'write_read::flush')
R('png_get_rowbytes, png_get_channels, png_get_IHDR, png_get_image_width, '
  'png_get_image_height, png_get_bit_depth, png_get_color_type, '
  'png_get_interlace_type, png_get_compression_type, png_get_filter_type',
  'read back after png_read_info and after png_read_update_info for every '
  'shape', 'write_read::info_getters')
R('png_get_io_state, png_get_io_chunk_type, png_init_io, png_get_io_ptr',
  'IO state sampled from the read and write callbacks at every chunk '
  'boundary; png_init_io with a real FILE* (tmpfile)', 'misc::io_state')
R('png_set_invalid, png_get_valid',
  'every PNG_INFO_* bit invalidated singly and in combination before '
  'png_write_info', 'chunks::set_invalid')
R('png_set_sig_bytes, png_get_signature',
  'signature already consumed by the app: 0..8 bytes pre-read',
  'write_read::sig_bytes')

# ---------------- read transforms -------------------------------------------
TR = [
    ('png_set_palette_to_rgb', 'palette 1/2/4/8-bit x with and without tRNS'),
    ('png_set_expand_gray_1_2_4_to_8', 'gray 1/2/4-bit'),
    ('png_set_tRNS_to_alpha',
     'every colour type with a tRNS chunk present and absent'),
    ('png_set_expand', 'all colour types; combined with tRNS and bKGD'),
    ('png_set_expand_16',
     '8-bit and 16-bit inputs, with and without png_set_expand'),
    ('png_set_strip_16', '16-bit inputs of every colour type'),
    ('png_set_scale_16', '16-bit inputs of every colour type'),
    ('png_set_strip_alpha', 'GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit'),
    ('png_set_swap_alpha', 'GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit'),
    ('png_set_invert_alpha', 'GRAY_ALPHA and RGB_ALPHA, 8 and 16 bit'),
    ('png_set_filler',
     'filler value 0 / 0xff / random x PNG_FILLER_BEFORE/AFTER x GRAY and RGB, '
     '8 and 16 bit'),
    ('png_set_add_alpha',
     'filler value x BEFORE/AFTER x GRAY and RGB, 8 and 16 bit'),
    ('png_set_bgr', 'RGB and RGB_ALPHA, 8 and 16 bit'),
    ('png_set_swap', '16-bit inputs of every colour type'),
    ('png_set_packing', '1/2/4-bit gray and palette'),
    ('png_set_packswap', '1/2/4-bit gray and palette'),
    ('png_set_shift',
     'random sBIT values <= bit depth for every colour type; sBIT chunk '
     'present and absent'),
    ('png_set_invert_mono',
     '1-bit and 8-bit gray, and non-gray input (no-op)'),
    ('png_set_gray_to_rgb', 'GRAY and GRAY_ALPHA, every bit depth'),
    ('png_set_rgb_to_gray / png_set_rgb_to_gray_fixed',
     'error action NONE/WARN/ERROR x default and explicit red/green '
     'coefficients x RGB, RGB_ALPHA, PALETTE; cHRM present and absent'),
    ('png_set_quantize',
     'palette and RGB input x num_palette 1..256 x maximum_colors 1..256 x '
     'histogram supplied and not x full_quantize 0/1'),
    ('png_set_background / png_set_background_fixed',
     'background gamma code UNKNOWN/SCREEN/FILE/UNIQUE x need_expand 0/1 x '
     'random png_color_16 x every colour type with and without alpha'),
    ('png_set_alpha_mode / png_set_alpha_mode_fixed',
     'mode PNG_ALPHA_PNG/STANDARD/OPTIMIZED/BROKEN x screen gamma '
     '{1.0, 2.2, 0.45455, PNG_FP_1}'),
    ('png_set_gamma / png_set_gamma_fixed',
     'screen gamma x file gamma pairs incl. equal, unity and extreme; '
     'gAMA / sRGB present and absent'),
    ('png_set_interlace_handling',
     'interlaced and non-interlaced input; the returned pass count'),
    ('png_set_read_user_transform_fn + png_set_user_transform_info',
     'user transform that rewrites the row, with user bit depth / channels '
     'overridden'),
    ('png_set_check_for_invalid_index',
     'palette images with in-range and out-of-range indices, on and off'),
]
for name, cfg in TR:
    R(name + ' (then png_read_update_info, png_read_image)', cfg,
      'transforms::single')
R('all read transforms',
  'randomised *combinations* of 2..6 read transforms applied together (in the '
  'order libpng resolves them in png_init_read_transformations), over every '
  'shape', 'transforms::combinations')
R('png_read_update_info, png_read_transform_info (internal)',
  'called once, twice (libpng warns) and not at all before reading rows',
  'transforms::update_info')

# ---------------- write transforms ------------------------------------------
for name, cfg in [
    ('png_set_bgr', 'RGB / RGB_ALPHA, 8 and 16 bit'),
    ('png_set_swap', '16-bit'),
    ('png_set_packing',
     'the app supplies one pixel per byte for 1/2/4-bit output'),
    ('png_set_packswap', '1/2/4-bit'),
    ('png_set_shift', 'sBIT smaller than the bit depth for every colour type'),
    ('png_set_invert_mono', '1-bit gray'),
    ('png_set_invert_alpha', 'GRAY_ALPHA / RGB_ALPHA'),
    ('png_set_swap_alpha', 'GRAY_ALPHA / RGB_ALPHA'),
    ('png_set_filler (strip filler on write)',
     'PNG_FILLER_BEFORE/AFTER on RGB and GRAY output'),
    ('png_set_write_user_transform_fn + png_set_user_transform_info',
     'user transform that rewrites the row before filtering'),
]:
    R(name + ' (write side)', cfg, 'transforms::write_side')
R('png_permit_mng_features + PNG_INTRAPIXEL_DIFFERENCING',
  'MNG features 0 / EMPTY_PLTE / FILTER_64 / ALL x filter method 0 and 64 x '
  'RGB and RGB_ALPHA, 8 and 16 bit, read and write',
  'transforms::mng_intrapixel')

# ---------------- chunks ----------------------------------------------------
CHUNKS = [
    ('gAMA', 'png_set_gAMA, png_set_gAMA_fixed, png_get_gAMA, '
             'png_get_gAMA_fixed',
     'gamma 0, 1, 100000, 500000, PNG_FP_MAX and values libpng rejects'),
    ('cHRM', 'png_set_cHRM, png_set_cHRM_fixed, png_set_cHRM_XYZ, '
             'png_set_cHRM_XYZ_fixed, png_get_cHRM*',
     'sRGB primaries, degenerate, negative, out of range'),
    ('sRGB', 'png_set_sRGB, png_set_sRGB_gAMA_and_cHRM, png_get_sRGB',
     'intent 0..3 (and 4, which is rejected)'),
    ('iCCP', 'png_set_iCCP, png_get_iCCP',
     'name lengths 1/79/80, profile sizes 132..2048, compression type 0, a '
     'real sRGB profile'),
    ('sBIT', 'png_set_sBIT, png_get_sBIT',
     'every colour type x every legal sBIT combination'),
    ('bKGD', 'png_set_bKGD, png_get_bKGD',
     'every colour type x random png_color_16 incl. index >= num_palette'),
    ('hIST', 'png_set_hIST, png_get_hIST',
     'palette sizes 1..256 with a matching histogram'),
    ('tRNS', 'png_set_tRNS, png_get_tRNS',
     'palette (num_trans 1..256), gray, RGB; num_trans 0 and > palette'),
    ('pHYs', 'png_set_pHYs, png_get_pHYs, png_get_pHYs_dpi, '
             'png_get_x_pixels_per_meter, png_get_y_pixels_per_meter, '
             'png_get_x_pixels_per_inch, png_get_y_pixels_per_inch, '
             'png_get_pixels_per_inch, png_get_pixels_per_meter, '
             'png_get_pixel_aspect_ratio, png_get_pixel_aspect_ratio_fixed',
     'unit 0/1/2 x random resolutions incl. 0'),
    ('oFFs', 'png_set_oFFs, png_get_oFFs, png_get_x_offset_pixels, '
             'png_get_y_offset_pixels, png_get_x_offset_microns, '
             'png_get_y_offset_microns, png_get_x_offset_inches, '
             'png_get_y_offset_inches, png_get_x_offset_inches_fixed, '
             'png_get_y_offset_inches_fixed',
     'unit 0/1/2 x negative and positive offsets'),
    ('tIME', 'png_set_tIME, png_get_tIME',
     'random valid and invalid png_time values'),
    ('pCAL', 'png_set_pCAL, png_get_pCAL',
     'every equation type 0..3 x nparams 0..8 x random purpose/units/params '
     'strings'),
    ('sCAL', 'png_set_sCAL, png_set_sCAL_fixed, png_set_sCAL_s, '
             'png_get_sCAL, png_get_sCAL_fixed, png_get_sCAL_s',
     'unit 1/2 x random widths/heights incl. 0 and malformed strings'),
    ('sPLT', 'png_set_sPLT, png_get_sPLT',
     'depth 8 and 16 x nentries 0/1/256 x several palettes at once'),
    ('tEXt', 'png_set_text, png_get_text',
     'keys of length 1..80 x text length 0..4096 x several entries'),
    ('zTXt', 'png_set_text with PNG_TEXT_COMPRESSION_zTXt',
     'compressible and incompressible payloads, length 0..65536'),
    ('iTXt', 'png_set_text with PNG_ITXT_COMPRESSION_NONE / _zTXt',
     'lang and lang_key empty and populated, UTF-8 payloads'),
    ('eXIf', 'png_set_eXIf_1, png_get_eXIf_1, png_set_eXIf, png_get_eXIf',
     'sizes 0..4096, valid "II"/"MM" headers and garbage'),
    ('cICP', 'png_set_cICP, png_get_cICP',
     'sampled colour primaries / transfer / matrix bytes x video_full_range '
     '0/1/2'),
    ('cLLI', 'png_set_cLLI, png_set_cLLI_fixed, png_get_cLLI, '
             'png_get_cLLI_fixed',
     'maxCLL / maxFALL 0, 1, 10000*PNG_FP_1, PNG_FP_MAX'),
    ('mDCV', 'png_set_mDCV, png_set_mDCV_fixed, png_get_mDCV, '
             'png_get_mDCV_fixed',
     'random chromaticities x luminance values'),
]
for cn, api, cfg in CHUNKS:
    R(api, '%s: %s -- written, read back and compared, with the chunk absent, '
           'present once, and present twice' % (cn, cfg),
      'chunks::round_trip')
R('png_set_unknown_chunks, png_get_unknown_chunks, '
  'png_set_unknown_chunk_location, png_set_keep_unknown_chunks, '
  'png_handle_as_unknown',
  'keep AS_DEFAULT/NEVER/IF_SAFE/ALWAYS x chunk list {NULL (all), named} x '
  'chunk name critical / ancillary / private / reserved / safe-to-copy x '
  'location BEFORE_PLTE / before IDAT / after IDAT x data size 0..1024',
  'chunks::unknown')
R('png_set_rows, png_get_rows, png_free_data, png_data_freer',
  'info rows set by the app, freed by the app (PNG_USER_WILL_FREE_DATA) and by '
  'libpng (PNG_DESTROY_WILL_FREE_DATA); every PNG_FREE_* mask',
  'chunks::rows_and_freer')
R('png_set_text_2 (internal), png_set_text',
  'num_text 0, 1, many; the text realloc path (> 8 entries); compression '
  'values -3..2 and out of range', 'chunks::text_many')

# ---------------- high level ------------------------------------------------
R('png_read_png',
  'every transform flag valid on read, singly and in random combinations, over '
  'every shape; params NULL', 'highlevel::read_png')
R('png_write_png',
  'every transform flag valid on write, singly and in random combinations, '
  'over every shape', 'highlevel::write_png')
R('png_read_png + png_write_png',
  'round trip: read with T, write with T, over random shapes',
  'highlevel::round_trip')

# ---------------- progressive ----------------------------------------------
R('png_set_progressive_read_fn, png_process_data, '
  'png_progressive_combine_row, png_get_progressive_ptr',
  'feed the file in fixed chunks of 1, 2, 3, 5, 13, 100, 1024, 8192 and '
  'all-at-once, x every shape x interlace NONE/ADAM7',
  'progressive::chunk_sizes')
R('png_process_data_pause, png_process_data_skip',
  'pause with save 0 and 1 at every chunk boundary; skip after IDAT',
  'progressive::pause_skip')
R('png_push_read_chunk, png_push_read_IDAT, png_push_save_buffer, '
  'png_push_restore_buffer, png_push_fill_buffer (internal, via '
  'png_process_data)',
  'buffer save/restore boundaries: chunk headers split across feeds, IDAT '
  'split mid-row, zero-length feeds', 'progressive::split_boundaries')
R('progressive read with transforms',
  'the read transforms of the rows above, applied to a progressive reader',
  'progressive::transforms')

# ---------------- simplified ------------------------------------------------
R('png_image_begin_read_from_memory, png_image_finish_read, png_image_free',
  'every output format: GRAY GA AG RGB BGR RGBA ARGB BGRA ABGR, each plain and '
  '_COLORMAP, plus the 4 LINEAR formats; flags 0 / FAST / 16BIT_sRGB; '
  'background supplied and NULL; row_stride positive, negative and 0',
  'simplified::read_formats')
R('png_image_begin_read_from_stdio, png_image_begin_read_from_file',
  'the same via a real FILE* and a real path', 'simplified::read_stdio')
R('png_image_write_to_memory, png_image_write_to_stdio, '
  'png_image_write_to_file, png_image_write_to_memory (size query)',
  'every input format x convert_to_8bit 0/1 x colormap present/absent x '
  'row_stride positive/negative x memory buffer exactly / too small / NULL',
  'simplified::write_formats')
R('png_image_write_to_memory + png_image_begin_read_from_memory',
  'round trip through every format', 'simplified::round_trip')

# ---------------- policies / limits / callbacks -----------------------------
R('png_set_crc_action',
  'crit x ancil over all 36 combinations, on a file with a corrupted ancillary '
  'CRC, a corrupted critical CRC and correct CRCs', 'misc::crc_action')
R('png_set_user_limits, png_get_user_width_max, png_get_user_height_max, '
  'png_set_chunk_cache_max, png_get_chunk_cache_max, '
  'png_set_chunk_malloc_max, png_get_chunk_malloc_max',
  'limits below / equal to / above the image dimensions and the chunk sizes; '
  '0 (= unlimited)', 'misc::user_limits')
R('png_set_mem_fn, png_get_mem_ptr, png_create_read_struct_2, '
  'png_create_write_struct_2',
  'custom allocator that records every allocation; also an allocator that '
  'fails after N allocations', 'misc::custom_alloc')
R('png_set_error_fn, png_get_error_ptr, png_set_benign_errors, png_error, '
  'png_warning, png_app_error, png_app_warning, png_benign_error, '
  'png_chunk_error, png_chunk_warning, png_chunk_benign_error, '
  'png_chunk_report, png_formatted_warning, png_warning_parameter, '
  'png_warning_parameter_signed, png_warning_parameter_unsigned',
  'benign errors allowed and not, on a read struct and a write struct, with '
  'app warnings/errors reported as warnings and as errors',
  'errors::reporting_matrix')
R('png_set_read_status_fn, png_set_write_status_fn',
  'row/pass callbacks over interlaced and non-interlaced images',
  'misc::status_callbacks')
R('png_set_read_user_chunk_fn, png_get_user_chunk_ptr',
  'user chunk callback returning -1 (error), 0 (unhandled) and 1 (handled), on '
  'ancillary and critical unknown chunks', 'chunks::user_chunk_fn')
R('png_set_option',
  'every option 0..PNG_OPTION_NEXT (and 2 past the end) x '
  'PNG_OPTION_ON/OFF/other, checking the returned previous state',
  'misc::options')
R('png_get_current_row_number, png_get_current_pass_number',
  'sampled from the row callback for interlaced and non-interlaced reads',
  'misc::row_number')
R('png_set_longjmp_fn, png_longjmp, png_free_jmpbuf (internal)',
  'jmp_buf_size equal to, smaller than and larger than sizeof(jmp_buf); '
  'called twice; NULL longjmp_fn', 'misc::longjmp_fn')
R('png_info_init_3, png_create_info_struct, png_destroy_info_struct, '
  'png_destroy_read_struct, png_destroy_write_struct, png_destroy_png_struct, '
  'png_create_png_struct',
  'sizes equal / smaller / larger than sizeof(png_info); double destroy; '
  'destroy with NULL out-parameters', 'misc::struct_lifecycle')
R('png_build_grayscale_palette',
  'bit depth 1, 2, 4, 8 (and the invalid 3 and 16) into a 256-entry palette',
  'misc::grayscale_palette')
R('png_write_info_before_PLTE, png_write_info, png_write_end with info NULL',
  'chunks emitted before PLTE vs after; png_write_end(png, NULL)',
  'chunks::write_order')
R('png_read_info, png_read_end with info NULL, png_read_update_info',
  'png_read_end(png, NULL) and with an info struct; unread rows at '
  'png_read_end', 'write_read::read_end')

out = ['| C-%d | `%s` | %s | [ ] |'
       % (i + 1, e.replace('|', '\\|'), c.replace('|', '\\|'))
       for i, (e, c, t) in enumerate(rows)]
with open('CONFIGS.md', 'a') as f:
    f.write('\n'.join(out) + '\n')
    f.write('\n## Row → test mapping\n\n| # | test that covers it |\n'
            '|---|---------------------|\n')
    for i, (e, c, t) in enumerate(rows):
        f.write('| C-%d | `%s` |\n' % (i + 1, t))
print('rows:', len(rows))
