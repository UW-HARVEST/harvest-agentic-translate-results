# Required Rust signatures for every exported libpng symbol

Each of these MUST be defined exactly once, prefixed with `#[unsafe(no_mangle)]`:

```rust
pub unsafe extern "C" fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> c_int
pub unsafe extern "C" fn png_access_version_number() -> png_uint_32
pub unsafe extern "C" fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp)
pub unsafe extern "C" fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp)
pub unsafe extern "C" fn png_ascii_from_fixed(png_ptr: png_const_structrp, ascii: png_charp, size: usize, fp: png_fixed_point)
pub unsafe extern "C" fn png_ascii_from_fp(png_ptr: png_const_structrp, ascii: png_charp, size: usize, fp: f64, precision: c_uint)
pub unsafe extern "C" fn png_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp)
pub unsafe extern "C" fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: c_int)
pub unsafe extern "C" fn png_build_grayscale_palette(bit_depth: c_int, palette: png_colorp)
pub unsafe extern "C" fn png_calculate_crc(png_ptr: png_structrp, ptr: png_const_bytep, length: usize)
pub unsafe extern "C" fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp
pub unsafe extern "C" fn png_check_IHDR(png_ptr: png_const_structrp, width: png_uint_32, height: png_uint_32, bit_depth: c_int, color_type: c_int, interlace_type: c_int, compression_type: c_int, filter_type: c_int)
pub unsafe extern "C" fn png_check_fp_number(string: png_const_charp, size: usize, statep: *mut c_int, whereami: *mut usize) -> c_int
pub unsafe extern "C" fn png_check_fp_string(string: png_const_charp, size: usize) -> c_int
pub unsafe extern "C" fn png_check_keyword(png_ptr: png_structrp, key: png_const_charp, new_key: png_bytep) -> png_uint_32
pub unsafe extern "C" fn png_chunk_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp)
pub unsafe extern "C" fn png_chunk_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !
pub unsafe extern "C" fn png_chunk_report(png_ptr: png_const_structrp, message: png_const_charp, error: c_int)
pub unsafe extern "C" fn png_chunk_unknown_handling(png_ptr: png_const_structrp, chunk_name: png_uint_32) -> c_int
pub unsafe extern "C" fn png_chunk_warning(png_ptr: png_const_structrp, warning_message: png_const_charp)
pub unsafe extern "C" fn png_combine_row(png_ptr: png_const_structrp, row: png_bytep, display: c_int)
pub unsafe extern "C" fn png_compress_IDAT(png_ptr: png_structrp, row_data: png_const_bytep, row_data_length: png_alloc_size_t, flush: c_int)
pub unsafe extern "C" fn png_convert_from_struct_tm(ptime: png_timep, ttime: *const tm)
pub unsafe extern "C" fn png_convert_from_time_t(ptime: png_timep, ttime: time_t)
pub unsafe extern "C" fn png_convert_to_rfc1123(png_ptr: png_structrp, ptime: png_const_timep) -> png_const_charp
pub unsafe extern "C" fn png_convert_to_rfc1123_buffer(out: *mut c_char, ptime: png_const_timep) -> c_int
pub unsafe extern "C" fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> c_int
pub unsafe extern "C" fn png_crc_read(png_ptr: png_structrp, buf: png_bytep, length: png_uint_32)
pub unsafe extern "C" fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop
pub unsafe extern "C" fn png_create_png_struct(user_png_ver: png_const_charp, error_ptr: png_voidp, error_fn: png_error_ptr, warn_fn: png_error_ptr, mem_ptr: png_voidp, malloc_fn: png_malloc_ptr, free_fn: png_free_ptr) -> png_structp
pub unsafe extern "C" fn png_create_read_struct(user_png_ver: png_const_charp, error_ptr: png_voidp, error_fn: png_error_ptr, warn_fn: png_error_ptr) -> png_structp
pub unsafe extern "C" fn png_create_read_struct_2(user_png_ver: png_const_charp, error_ptr: png_voidp, error_fn: png_error_ptr, warn_fn: png_error_ptr, mem_ptr: png_voidp, malloc_fn: png_malloc_ptr, free_fn: png_free_ptr) -> png_structp
pub unsafe extern "C" fn png_create_write_struct(user_png_ver: png_const_charp, error_ptr: png_voidp, error_fn: png_error_ptr, warn_fn: png_error_ptr) -> png_structp
pub unsafe extern "C" fn png_create_write_struct_2(user_png_ver: png_const_charp, error_ptr: png_voidp, error_fn: png_error_ptr, warn_fn: png_error_ptr, mem_ptr: png_voidp, malloc_fn: png_malloc_ptr, free_fn: png_free_ptr) -> png_structp
pub unsafe extern "C" fn png_data_freer(png_ptr: png_const_structrp, info_ptr: png_inforp, freer: c_int, mask: png_uint_32)
pub unsafe extern "C" fn png_default_flush(png_ptr: png_structp)
pub unsafe extern "C" fn png_default_read_data(png_ptr: png_structp, data: png_bytep, length: usize)
pub unsafe extern "C" fn png_default_write_data(png_ptr: png_structp, data: png_bytep, length: usize)
pub unsafe extern "C" fn png_destroy_gamma_table(png_ptr: png_structrp)
pub unsafe extern "C" fn png_destroy_info_struct(png_ptr: png_const_structrp, info_ptr_ptr: png_infopp)
pub unsafe extern "C" fn png_destroy_png_struct(png_ptr: png_structrp)
pub unsafe extern "C" fn png_destroy_read_struct(png_ptr_ptr: png_structpp, info_ptr_ptr: png_infopp, end_info_ptr_ptr: png_infopp)
pub unsafe extern "C" fn png_destroy_write_struct(png_ptr_ptr: png_structpp, info_ptr_ptr: png_infopp)
pub unsafe extern "C" fn png_do_bgr(row_info: png_row_infop, row: png_bytep)
pub unsafe extern "C" fn png_do_check_palette_indexes(png_ptr: png_structrp, row_info: png_row_infop)
pub unsafe extern "C" fn png_do_invert(row_info: png_row_infop, row: png_bytep)
pub unsafe extern "C" fn png_do_packswap(row_info: png_row_infop, row: png_bytep)
pub unsafe extern "C" fn png_do_read_interlace(row_info: png_row_infop, row: png_bytep, pass: c_int, transformations: png_uint_32)
pub unsafe extern "C" fn png_do_read_transformations(png_ptr: png_structrp, row_info: png_row_infop)
pub unsafe extern "C" fn png_do_strip_channel(row_info: png_row_infop, row: png_bytep, at_start: c_int)
pub unsafe extern "C" fn png_do_swap(row_info: png_row_infop, row: png_bytep)
pub unsafe extern "C" fn png_do_write_interlace(row_info: png_row_infop, row: png_bytep, pass: c_int)
pub unsafe extern "C" fn png_do_write_transformations(png_ptr: png_structrp, row_info: png_row_infop)
pub unsafe extern "C" fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !
pub unsafe extern "C" fn png_fixed(png_ptr: png_const_structrp, fp: f64, text: png_const_charp) -> png_fixed_point
pub unsafe extern "C" fn png_fixed_ITU(png_ptr: png_const_structrp, fp: f64, text: png_const_charp) -> png_uint_32
pub unsafe extern "C" fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> !
pub unsafe extern "C" fn png_flush(png_ptr: png_structrp)
pub unsafe extern "C" fn png_format_number(start: png_const_charp, end: png_charp, format: c_int, number: png_alloc_size_t) -> png_charp
pub unsafe extern "C" fn png_formatted_warning(png_ptr: png_const_structrp, p: png_warning_parameters, message: png_const_charp)
pub unsafe extern "C" fn png_free(png_ptr: png_const_structrp, ptr: png_voidp)
pub unsafe extern "C" fn png_free_buffer_list(png_ptr: png_structrp, list: *mut png_compression_bufferp)
pub unsafe extern "C" fn png_free_data(png_ptr: png_const_structrp, info_ptr: png_inforp, free_me: png_uint_32, num: c_int)
pub unsafe extern "C" fn png_free_default(png_ptr: png_const_structrp, ptr: png_voidp)
pub unsafe extern "C" fn png_free_jmpbuf(png_ptr: png_structrp)
pub unsafe extern "C" fn png_gamma_16bit_correct(value: c_uint, gamma_value: png_fixed_point) -> png_uint_16
pub unsafe extern "C" fn png_gamma_8bit_correct(value: c_uint, gamma_value: png_fixed_point) -> png_byte
pub unsafe extern "C" fn png_gamma_correct(png_ptr: png_structrp, value: c_uint, gamma_value: png_fixed_point) -> png_uint_16
pub unsafe extern "C" fn png_gamma_significant(gamma_value: png_fixed_point) -> c_int
pub unsafe extern "C" fn png_get_IHDR(png_ptr: png_const_structrp, info_ptr: png_const_inforp, width: *mut png_uint_32, height: *mut png_uint_32, bit_depth: *mut c_int, color_type: *mut c_int, interlace_method: *mut c_int, compression_method: *mut c_int, filter_method: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_PLTE(png_ptr: png_const_structrp, info_ptr: png_inforp, palette: *mut png_colorp, num_palette: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_bKGD(png_ptr: png_const_structrp, info_ptr: png_inforp, background: *mut png_color_16p) -> png_uint_32
pub unsafe extern "C" fn png_get_bit_depth(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_cHRM(png_ptr: png_const_structrp, info_ptr: png_const_inforp, white_x: *mut f64, white_y: *mut f64, red_x: *mut f64, red_y: *mut f64, green_x: *mut f64, green_y: *mut f64, blue_x: *mut f64, blue_y: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_cHRM_XYZ(png_ptr: png_const_structrp, info_ptr: png_const_inforp, red_X: *mut f64, red_Y: *mut f64, red_Z: *mut f64, green_X: *mut f64, green_Y: *mut f64, green_Z: *mut f64, blue_X: *mut f64, blue_Y: *mut f64, blue_Z: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_cHRM_XYZ_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_red_X: *mut png_fixed_point, int_red_Y: *mut png_fixed_point, int_red_Z: *mut png_fixed_point, int_green_X: *mut png_fixed_point, int_green_Y: *mut png_fixed_point, int_green_Z: *mut png_fixed_point, int_blue_X: *mut png_fixed_point, int_blue_Y: *mut png_fixed_point, int_blue_Z: *mut png_fixed_point) -> png_uint_32
pub unsafe extern "C" fn png_get_cHRM_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_white_x: *mut png_fixed_point, int_white_y: *mut png_fixed_point, int_red_x: *mut png_fixed_point, int_red_y: *mut png_fixed_point, int_green_x: *mut png_fixed_point, int_green_y: *mut png_fixed_point, int_blue_x: *mut png_fixed_point, int_blue_y: *mut png_fixed_point) -> png_uint_32
pub unsafe extern "C" fn png_get_cICP(png_ptr: png_const_structrp, info_ptr: png_const_inforp, colour_primaries: png_bytep, transfer_function: png_bytep, matrix_coefficients: png_bytep, video_full_range_flag: png_bytep) -> png_uint_32
pub unsafe extern "C" fn png_get_cLLI(png_ptr: png_const_structrp, info_ptr: png_const_inforp, maximum_content_light_level: *mut f64, maximum_frame_average_light_level: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_cLLI_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, maximum_content_light_level_scaled_by_10000: png_uint_32p, maximum_frame_average_light_level_scaled_by_10000: png_uint_32p) -> png_uint_32
pub unsafe extern "C" fn png_get_channels(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_chunk_cache_max(png_ptr: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_chunk_malloc_max(png_ptr: png_const_structrp) -> png_alloc_size_t
pub unsafe extern "C" fn png_get_color_type(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_compression_buffer_size(png_ptr: png_const_structrp) -> usize
pub unsafe extern "C" fn png_get_compression_type(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_copyright(png_ptr: png_const_structrp) -> png_const_charp
pub unsafe extern "C" fn png_get_current_pass_number(arg: png_const_structrp) -> png_byte
pub unsafe extern "C" fn png_get_current_row_number(arg: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_eXIf(png_ptr: png_const_structrp, info_ptr: png_inforp, exif: *mut png_bytep) -> png_uint_32
pub unsafe extern "C" fn png_get_eXIf_1(png_ptr: png_const_structrp, info_ptr: png_const_inforp, num_exif: *mut png_uint_32, exif: *mut png_bytep) -> png_uint_32
pub unsafe extern "C" fn png_get_error_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_filter_type(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_gAMA(png_ptr: png_const_structrp, info_ptr: png_const_inforp, file_gamma: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_gAMA_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_file_gamma: *mut png_fixed_point) -> png_uint_32
pub unsafe extern "C" fn png_get_hIST(png_ptr: png_const_structrp, info_ptr: png_inforp, hist: *mut png_uint_16p) -> png_uint_32
pub unsafe extern "C" fn png_get_header_ver(png_ptr: png_const_structrp) -> png_const_charp
pub unsafe extern "C" fn png_get_header_version(png_ptr: png_const_structrp) -> png_const_charp
pub unsafe extern "C" fn png_get_iCCP(png_ptr: png_const_structrp, info_ptr: png_inforp, name: png_charpp, compression_type: *mut c_int, profile: png_bytepp, proflen: *mut png_uint_32) -> png_uint_32
pub unsafe extern "C" fn png_get_image_height(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_image_width(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_int_32(buf: png_const_bytep) -> png_int_32
pub unsafe extern "C" fn png_get_interlace_type(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte
pub unsafe extern "C" fn png_get_io_chunk_type(png_ptr: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_io_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_io_state(png_ptr: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_libpng_ver(png_ptr: png_const_structrp) -> png_const_charp
pub unsafe extern "C" fn png_get_mDCV(png_ptr: png_const_structrp, info_ptr: png_const_inforp, white_x: *mut f64, white_y: *mut f64, red_x: *mut f64, red_y: *mut f64, green_x: *mut f64, green_y: *mut f64, blue_x: *mut f64, blue_y: *mut f64, mastering_display_maximum_luminance: *mut f64, mastering_display_minimum_luminance: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_mDCV_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, int_white_x: *mut png_fixed_point, int_white_y: *mut png_fixed_point, int_red_x: *mut png_fixed_point, int_red_y: *mut png_fixed_point, int_green_x: *mut png_fixed_point, int_green_y: *mut png_fixed_point, int_blue_x: *mut png_fixed_point, int_blue_y: *mut png_fixed_point, mastering_display_maximum_luminance_scaled_by_10000: png_uint_32p, mastering_display_minimum_luminance_scaled_by_10000: png_uint_32p) -> png_uint_32
pub unsafe extern "C" fn png_get_mem_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_oFFs(png_ptr: png_const_structrp, info_ptr: png_const_inforp, offset_x: *mut png_int_32, offset_y: *mut png_int_32, unit_type: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_pCAL(png_ptr: png_const_structrp, info_ptr: png_inforp, purpose: *mut png_charp, X0: *mut png_int_32, X1: *mut png_int_32, type_: *mut c_int, nparams: *mut c_int, units: *mut png_charp, params: *mut png_charpp) -> png_uint_32
pub unsafe extern "C" fn png_get_pHYs(png_ptr: png_const_structrp, info_ptr: png_const_inforp, res_x: *mut png_uint_32, res_y: *mut png_uint_32, unit_type: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_pHYs_dpi(png_ptr: png_const_structrp, info_ptr: png_const_inforp, res_x: *mut png_uint_32, res_y: *mut png_uint_32, unit_type: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_palette_max(png_ptr: png_const_structp, info_ptr: png_const_infop) -> c_int
pub unsafe extern "C" fn png_get_pixel_aspect_ratio(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> f32
pub unsafe extern "C" fn png_get_pixel_aspect_ratio_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_fixed_point
pub unsafe extern "C" fn png_get_pixels_per_inch(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_pixels_per_meter(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_progressive_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_rgb_to_gray_status(png_ptr: png_const_structrp) -> png_byte
pub unsafe extern "C" fn png_get_rowbytes(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> usize
pub unsafe extern "C" fn png_get_rows(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_bytepp
pub unsafe extern "C" fn png_get_sBIT(png_ptr: png_const_structrp, info_ptr: png_inforp, sig_bit: *mut png_color_8p) -> png_uint_32
pub unsafe extern "C" fn png_get_sCAL(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, width: *mut f64, height: *mut f64) -> png_uint_32
pub unsafe extern "C" fn png_get_sCAL_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, width: *mut png_fixed_point, height: *mut png_fixed_point) -> png_uint_32
pub unsafe extern "C" fn png_get_sCAL_s(png_ptr: png_const_structrp, info_ptr: png_const_inforp, unit: *mut c_int, swidth: png_charpp, sheight: png_charpp) -> png_uint_32
pub unsafe extern "C" fn png_get_sPLT(png_ptr: png_const_structrp, info_ptr: png_inforp, entries: png_sPLT_tpp) -> c_int
pub unsafe extern "C" fn png_get_sRGB(png_ptr: png_const_structrp, info_ptr: png_const_inforp, file_srgb_intent: *mut c_int) -> png_uint_32
pub unsafe extern "C" fn png_get_signature(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_const_bytep
pub unsafe extern "C" fn png_get_tIME(png_ptr: png_const_structrp, info_ptr: png_inforp, mod_time: *mut png_timep) -> png_uint_32
pub unsafe extern "C" fn png_get_tRNS(png_ptr: png_const_structrp, info_ptr: png_inforp, trans_alpha: *mut png_bytep, num_trans: *mut c_int, trans_color: *mut png_color_16p) -> png_uint_32
pub unsafe extern "C" fn png_get_text(png_ptr: png_const_structrp, info_ptr: png_inforp, text_ptr: *mut png_textp, num_text: *mut c_int) -> c_int
pub unsafe extern "C" fn png_get_uint_16(buf: png_const_bytep) -> png_uint_16
pub unsafe extern "C" fn png_get_uint_31(png_ptr: png_const_structrp, buf: png_const_bytep) -> png_uint_32
pub unsafe extern "C" fn png_get_uint_32(buf: png_const_bytep) -> png_uint_32
pub unsafe extern "C" fn png_get_unknown_chunks(png_ptr: png_const_structrp, info_ptr: png_inforp, entries: png_unknown_chunkpp) -> c_int
pub unsafe extern "C" fn png_get_user_chunk_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_user_height_max(png_ptr: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_user_transform_ptr(png_ptr: png_const_structrp) -> png_voidp
pub unsafe extern "C" fn png_get_user_width_max(png_ptr: png_const_structrp) -> png_uint_32
pub unsafe extern "C" fn png_get_valid(png_ptr: png_const_structrp, info_ptr: png_const_inforp, flag: png_uint_32) -> png_uint_32
pub unsafe extern "C" fn png_get_x_offset_inches(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> f32
pub unsafe extern "C" fn png_get_x_offset_inches_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_fixed_point
pub unsafe extern "C" fn png_get_x_offset_microns(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_int_32
pub unsafe extern "C" fn png_get_x_offset_pixels(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_int_32
pub unsafe extern "C" fn png_get_x_pixels_per_inch(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_x_pixels_per_meter(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_y_offset_inches(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> f32
pub unsafe extern "C" fn png_get_y_offset_inches_fixed(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_fixed_point
pub unsafe extern "C" fn png_get_y_offset_microns(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_int_32
pub unsafe extern "C" fn png_get_y_offset_pixels(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_int_32
pub unsafe extern "C" fn png_get_y_pixels_per_inch(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_get_y_pixels_per_meter(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_uint_32
pub unsafe extern "C" fn png_handle_as_unknown(png_ptr: png_const_structrp, chunk_name: png_const_bytep) -> c_int
pub unsafe extern "C" fn png_handle_chunk(png_ptr: png_structrp, info_ptr: png_inforp, length: png_uint_32) -> png_handle_result_code
pub unsafe extern "C" fn png_handle_unknown(png_ptr: png_structrp, info_ptr: png_inforp, length: png_uint_32, keep: c_int) -> png_handle_result_code
pub unsafe extern "C" fn png_icc_check_header(png_ptr: png_const_structrp, name: png_const_charp, profile_length: png_uint_32, profile: png_const_bytep, color_type: c_int) -> c_int
pub unsafe extern "C" fn png_icc_check_length(png_ptr: png_const_structrp, name: png_const_charp, profile_length: png_uint_32) -> c_int
pub unsafe extern "C" fn png_icc_check_tag_table(png_ptr: png_const_structrp, name: png_const_charp, profile_length: png_uint_32, profile: png_const_bytep) -> c_int
pub unsafe extern "C" fn png_image_begin_read_from_file(image: png_imagep, file_name: *const c_char) -> c_int
pub unsafe extern "C" fn png_image_begin_read_from_memory(image: png_imagep, memory: png_const_voidp, size: usize) -> c_int
pub unsafe extern "C" fn png_image_begin_read_from_stdio(image: png_imagep, file: *mut FILE) -> c_int
pub unsafe extern "C" fn png_image_error(image: png_imagep, error_message: png_const_charp) -> c_int
pub unsafe extern "C" fn png_image_finish_read(image: png_imagep, background: png_const_colorp, buffer: *mut c_void, row_stride: png_int_32, colormap: *mut c_void) -> c_int
pub unsafe extern "C" fn png_image_free(image: png_imagep)
pub unsafe extern "C" fn png_image_write_to_file(image: png_imagep, file: *const c_char, convert_to_8bit: c_int, buffer: *const c_void, row_stride: png_int_32, colormap: *const c_void) -> c_int
pub unsafe extern "C" fn png_image_write_to_memory(image: png_imagep, memory: *mut c_void, memory_bytes: *mut png_alloc_size_t, convert_to_8_bit: c_int, buffer: *const c_void, row_stride: png_int_32, colormap: *const c_void) -> c_int
pub unsafe extern "C" fn png_image_write_to_stdio(image: png_imagep, file: *mut FILE, convert_to_8_bit: c_int, buffer: *const c_void, row_stride: png_int_32, colormap: *const c_void) -> c_int
pub unsafe extern "C" fn png_info_init_3(info_ptr: png_infopp, png_info_struct_size: usize)
pub unsafe extern "C" fn png_init_io(png_ptr: png_structrp, fp: *mut FILE)
pub unsafe extern "C" fn png_init_read_transformations(png_ptr: png_structrp)
pub unsafe extern "C" fn png_longjmp(png_ptr: png_const_structrp, val: c_int) -> !
pub unsafe extern "C" fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp
pub unsafe extern "C" fn png_malloc_array(png_ptr: png_const_structrp, nelements: c_int, element_size: usize) -> png_voidp
pub unsafe extern "C" fn png_malloc_base(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp
pub unsafe extern "C" fn png_malloc_default(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp
pub unsafe extern "C" fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp
pub unsafe extern "C" fn png_muldiv(res: png_fixed_point_p, a: png_fixed_point, multiplied_by: png_int_32, divided_by: png_int_32) -> c_int
pub unsafe extern "C" fn png_permit_mng_features(png_ptr: png_structrp, mng_features_permitted: png_uint_32) -> png_uint_32
pub unsafe extern "C" fn png_process_IDAT_data(png_ptr: png_structrp, buffer: png_bytep, buffer_length: usize)
pub unsafe extern "C" fn png_process_data(png_ptr: png_structrp, info_ptr: png_inforp, buffer: png_bytep, buffer_size: usize)
pub unsafe extern "C" fn png_process_data_pause(arg: png_structrp, save: c_int) -> usize
pub unsafe extern "C" fn png_process_data_skip(arg: png_structrp) -> png_uint_32
pub unsafe extern "C" fn png_process_some_data(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_progressive_combine_row(png_ptr: png_const_structrp, old_row: png_bytep, new_row: png_const_bytep)
pub unsafe extern "C" fn png_push_fill_buffer(png_ptr: png_structp, buffer: png_bytep, length: usize)
pub unsafe extern "C" fn png_push_have_end(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_push_have_info(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_push_have_row(png_ptr: png_structrp, row: png_bytep)
pub unsafe extern "C" fn png_push_process_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_push_read_IDAT(png_ptr: png_structrp)
pub unsafe extern "C" fn png_push_read_chunk(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_push_read_sig(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_push_restore_buffer(png_ptr: png_structrp, buffer: png_bytep, buffer_length: usize)
pub unsafe extern "C" fn png_push_save_buffer(png_ptr: png_structrp)
pub unsafe extern "C" fn png_read_IDAT_data(png_ptr: png_structrp, output: png_bytep, avail_out: png_alloc_size_t)
pub unsafe extern "C" fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32
pub unsafe extern "C" fn png_read_data(png_ptr: png_structrp, data: png_bytep, length: usize)
pub unsafe extern "C" fn png_read_end(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_read_filter_row(pp: png_structrp, row_info: png_row_infop, row: png_bytep, prev_row: png_const_bytep, filter: c_int)
pub unsafe extern "C" fn png_read_finish_IDAT(png_ptr: png_structrp)
pub unsafe extern "C" fn png_read_finish_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_read_image(png_ptr: png_structrp, image: png_bytepp)
pub unsafe extern "C" fn png_read_info(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_read_png(png_ptr: png_structrp, info_ptr: png_inforp, transforms: c_int, params: png_voidp)
pub unsafe extern "C" fn png_read_push_finish_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_read_row(png_ptr: png_structrp, row: png_bytep, display_row: png_bytep)
pub unsafe extern "C" fn png_read_rows(png_ptr: png_structrp, row: png_bytepp, display_row: png_bytepp, num_rows: png_uint_32)
pub unsafe extern "C" fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_read_start_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_read_transform_info(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_read_update_info(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_realloc_array(png_ptr: png_const_structrp, array: png_const_voidp, old_elements: c_int, add_elements: c_int, element_size: usize) -> png_voidp
pub unsafe extern "C" fn png_reciprocal(a: png_fixed_point) -> png_fixed_point
pub unsafe extern "C" fn png_reciprocal2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point
pub unsafe extern "C" fn png_reset_crc(png_ptr: png_structrp)
pub unsafe extern "C" fn png_reset_zstream(png_ptr: png_structrp) -> c_int
pub unsafe extern "C" fn png_resolve_file_gamma(png_ptr: png_const_structrp) -> png_fixed_point
pub unsafe extern "C" fn png_safe_execute(image: png_imagep, function: Option<unsafe extern "C" fn(png_voidp) -> c_int>, arg: png_voidp) -> c_int
pub unsafe extern "C" fn png_safecat(buffer: png_charp, bufsize: usize, pos: usize, string: png_const_charp) -> usize
pub unsafe extern "C" fn png_safe_error(png_ptr: png_structp, error_message: png_const_charp)
pub unsafe extern "C" fn png_safe_warning(png_ptr: png_structp, warning_message: png_const_charp)
pub unsafe extern "C" fn png_save_int_32(buf: png_bytep, i: png_int_32)
pub unsafe extern "C" fn png_save_uint_16(buf: png_bytep, i: c_uint)
pub unsafe extern "C" fn png_save_uint_32(buf: png_bytep, i: png_uint_32)
pub unsafe extern "C" fn png_set_IHDR(png_ptr: png_const_structrp, info_ptr: png_inforp, width: png_uint_32, height: png_uint_32, bit_depth: c_int, color_type: c_int, interlace_method: c_int, compression_method: c_int, filter_method: c_int)
pub unsafe extern "C" fn png_set_PLTE(png_ptr: png_structrp, info_ptr: png_inforp, palette: png_const_colorp, num_palette: c_int)
pub unsafe extern "C" fn png_set_add_alpha(png_ptr: png_structrp, filler: png_uint_32, flags: c_int)
pub unsafe extern "C" fn png_set_alpha_mode(png_ptr: png_structrp, mode: c_int, output_gamma: f64)
pub unsafe extern "C" fn png_set_alpha_mode_fixed(png_ptr: png_structrp, mode: c_int, output_gamma: png_fixed_point)
pub unsafe extern "C" fn png_set_bKGD(png_ptr: png_const_structrp, info_ptr: png_inforp, background: png_const_color_16p)
pub unsafe extern "C" fn png_set_background(png_ptr: png_structrp, background_color: png_const_color_16p, background_gamma_code: c_int, need_expand: c_int, background_gamma: f64)
pub unsafe extern "C" fn png_set_background_fixed(png_ptr: png_structrp, background_color: png_const_color_16p, background_gamma_code: c_int, need_expand: c_int, background_gamma: png_fixed_point)
pub unsafe extern "C" fn png_set_benign_errors(png_ptr: png_structrp, allowed: c_int)
pub unsafe extern "C" fn png_set_bgr(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_cHRM(png_ptr: png_const_structrp, info_ptr: png_inforp, white_x: f64, white_y: f64, red_x: f64, red_y: f64, green_x: f64, green_y: f64, blue_x: f64, blue_y: f64)
pub unsafe extern "C" fn png_set_cHRM_XYZ(png_ptr: png_const_structrp, info_ptr: png_inforp, red_X: f64, red_Y: f64, red_Z: f64, green_X: f64, green_Y: f64, green_Z: f64, blue_X: f64, blue_Y: f64, blue_Z: f64)
pub unsafe extern "C" fn png_set_cHRM_XYZ_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, int_red_X: png_fixed_point, int_red_Y: png_fixed_point, int_red_Z: png_fixed_point, int_green_X: png_fixed_point, int_green_Y: png_fixed_point, int_green_Z: png_fixed_point, int_blue_X: png_fixed_point, int_blue_Y: png_fixed_point, int_blue_Z: png_fixed_point)
pub unsafe extern "C" fn png_set_cHRM_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, int_white_x: png_fixed_point, int_white_y: png_fixed_point, int_red_x: png_fixed_point, int_red_y: png_fixed_point, int_green_x: png_fixed_point, int_green_y: png_fixed_point, int_blue_x: png_fixed_point, int_blue_y: png_fixed_point)
pub unsafe extern "C" fn png_set_cICP(png_ptr: png_const_structrp, info_ptr: png_inforp, colour_primaries: png_byte, transfer_function: png_byte, matrix_coefficients: png_byte, video_full_range_flag: png_byte)
pub unsafe extern "C" fn png_set_cLLI(png_ptr: png_const_structrp, info_ptr: png_inforp, maximum_content_light_level: f64, maximum_frame_average_light_level: f64)
pub unsafe extern "C" fn png_set_cLLI_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, maximum_content_light_level_scaled_by_10000: png_uint_32, maximum_frame_average_light_level_scaled_by_10000: png_uint_32)
pub unsafe extern "C" fn png_set_check_for_invalid_index(png_ptr: png_structrp, allowed: c_int)
pub unsafe extern "C" fn png_set_chunk_cache_max(png_ptr: png_structrp, user_chunk_cache_max: png_uint_32)
pub unsafe extern "C" fn png_set_chunk_malloc_max(png_ptr: png_structrp, user_chunk_cache_max: png_alloc_size_t)
pub unsafe extern "C" fn png_set_compression_buffer_size(png_ptr: png_structrp, size: usize)
pub unsafe extern "C" fn png_set_compression_level(png_ptr: png_structrp, level: c_int)
pub unsafe extern "C" fn png_set_compression_mem_level(png_ptr: png_structrp, mem_level: c_int)
pub unsafe extern "C" fn png_set_compression_method(png_ptr: png_structrp, method: c_int)
pub unsafe extern "C" fn png_set_compression_strategy(png_ptr: png_structrp, strategy: c_int)
pub unsafe extern "C" fn png_set_compression_window_bits(png_ptr: png_structrp, window_bits: c_int)
pub unsafe extern "C" fn png_set_crc_action(png_ptr: png_structrp, crit_action: c_int, ancil_action: c_int)
pub unsafe extern "C" fn png_set_eXIf(png_ptr: png_const_structrp, info_ptr: png_inforp, exif: png_bytep)
pub unsafe extern "C" fn png_set_eXIf_1(png_ptr: png_const_structrp, info_ptr: png_inforp, num_exif: png_uint_32, exif: png_bytep)
pub unsafe extern "C" fn png_set_error_fn(png_ptr: png_structrp, error_ptr: png_voidp, error_fn: png_error_ptr, warning_fn: png_error_ptr)
pub unsafe extern "C" fn png_set_expand(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_expand_16(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_expand_gray_1_2_4_to_8(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_filler(png_ptr: png_structrp, filler: png_uint_32, flags: c_int)
pub unsafe extern "C" fn png_set_filter(png_ptr: png_structrp, method: c_int, filters: c_int)
pub unsafe extern "C" fn png_set_filter_heuristics(png_ptr: png_structrp, heuristic_method: c_int, num_weights: c_int, filter_weights: png_const_doublep, filter_costs: png_const_doublep)
pub unsafe extern "C" fn png_set_filter_heuristics_fixed(png_ptr: png_structrp, heuristic_method: c_int, num_weights: c_int, filter_weights: png_const_fixed_point_p, filter_costs: png_const_fixed_point_p)
pub unsafe extern "C" fn png_set_flush(png_ptr: png_structrp, nrows: c_int)
pub unsafe extern "C" fn png_set_gAMA(png_ptr: png_const_structrp, info_ptr: png_inforp, file_gamma: f64)
pub unsafe extern "C" fn png_set_gAMA_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, int_file_gamma: png_fixed_point)
pub unsafe extern "C" fn png_set_gamma(png_ptr: png_structrp, screen_gamma: f64, override_file_gamma: f64)
pub unsafe extern "C" fn png_set_gamma_fixed(png_ptr: png_structrp, screen_gamma: png_fixed_point, override_file_gamma: png_fixed_point)
pub unsafe extern "C" fn png_set_gray_to_rgb(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_hIST(png_ptr: png_const_structrp, info_ptr: png_inforp, hist: png_const_uint_16p)
pub unsafe extern "C" fn png_set_iCCP(png_ptr: png_const_structrp, info_ptr: png_inforp, name: png_const_charp, compression_type: c_int, profile: png_const_bytep, proflen: png_uint_32)
pub unsafe extern "C" fn png_set_interlace_handling(png_ptr: png_structrp) -> c_int
pub unsafe extern "C" fn png_set_invalid(png_ptr: png_const_structrp, info_ptr: png_inforp, mask: c_int)
pub unsafe extern "C" fn png_set_invert_alpha(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_invert_mono(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_keep_unknown_chunks(png_ptr: png_structrp, keep: c_int, chunk_list: png_const_bytep, num_chunks: c_int)
pub unsafe extern "C" fn png_set_longjmp_fn(png_ptr: png_structrp, longjmp_fn: png_longjmp_ptr, jmp_buf_size: usize) -> *mut jmp_buf
pub unsafe extern "C" fn png_set_mDCV(png_ptr: png_const_structrp, info_ptr: png_inforp, white_x: f64, white_y: f64, red_x: f64, red_y: f64, green_x: f64, green_y: f64, blue_x: f64, blue_y: f64, mastering_display_maximum_luminance: f64, mastering_display_minimum_luminance: f64)
pub unsafe extern "C" fn png_set_mDCV_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, int_white_x: png_fixed_point, int_white_y: png_fixed_point, int_red_x: png_fixed_point, int_red_y: png_fixed_point, int_green_x: png_fixed_point, int_green_y: png_fixed_point, int_blue_x: png_fixed_point, int_blue_y: png_fixed_point, mastering_display_maximum_luminance_scaled_by_10000: png_uint_32, mastering_display_minimum_luminance_scaled_by_10000: png_uint_32)
pub unsafe extern "C" fn png_set_mem_fn(png_ptr: png_structrp, mem_ptr: png_voidp, malloc_fn: png_malloc_ptr, free_fn: png_free_ptr)
pub unsafe extern "C" fn png_set_oFFs(png_ptr: png_const_structrp, info_ptr: png_inforp, offset_x: png_int_32, offset_y: png_int_32, unit_type: c_int)
pub unsafe extern "C" fn png_set_option(png_ptr: png_structrp, option: c_int, onoff: c_int) -> c_int
pub unsafe extern "C" fn png_set_pCAL(png_ptr: png_const_structrp, info_ptr: png_inforp, purpose: png_const_charp, X0: png_int_32, X1: png_int_32, type_: c_int, nparams: c_int, units: png_const_charp, params: png_charpp)
pub unsafe extern "C" fn png_set_pHYs(png_ptr: png_const_structrp, info_ptr: png_inforp, res_x: png_uint_32, res_y: png_uint_32, unit_type: c_int)
pub unsafe extern "C" fn png_set_packing(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_packswap(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_palette_to_rgb(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_progressive_read_fn(png_ptr: png_structrp, progressive_ptr: png_voidp, info_fn: png_progressive_info_ptr, row_fn: png_progressive_row_ptr, end_fn: png_progressive_end_ptr)
pub unsafe extern "C" fn png_set_quantize(png_ptr: png_structrp, palette: png_colorp, num_palette: c_int, maximum_colors: c_int, histogram: png_const_uint_16p, full_quantize: c_int)
pub unsafe extern "C" fn png_set_read_fn(png_ptr: png_structrp, io_ptr: png_voidp, read_data_fn: png_rw_ptr)
pub unsafe extern "C" fn png_set_read_status_fn(png_ptr: png_structrp, read_row_fn: png_read_status_ptr)
pub unsafe extern "C" fn png_set_read_user_chunk_fn(png_ptr: png_structrp, user_chunk_ptr: png_voidp, read_user_chunk_fn: png_user_chunk_ptr)
pub unsafe extern "C" fn png_set_read_user_transform_fn(png_ptr: png_structrp, read_user_transform_fn: png_user_transform_ptr)
pub unsafe extern "C" fn png_set_rgb_coefficients(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_rgb_to_gray(png_ptr: png_structrp, error_action: c_int, red: f64, green: f64)
pub unsafe extern "C" fn png_set_rgb_to_gray_fixed(png_ptr: png_structrp, error_action: c_int, red: png_fixed_point, green: png_fixed_point)
pub unsafe extern "C" fn png_set_rows(png_ptr: png_const_structrp, info_ptr: png_inforp, row_pointers: png_bytepp)
pub unsafe extern "C" fn png_set_sBIT(png_ptr: png_const_structrp, info_ptr: png_inforp, sig_bit: png_const_color_8p)
pub unsafe extern "C" fn png_set_sCAL(png_ptr: png_const_structrp, info_ptr: png_inforp, unit: c_int, width: f64, height: f64)
pub unsafe extern "C" fn png_set_sCAL_fixed(png_ptr: png_const_structrp, info_ptr: png_inforp, unit: c_int, width: png_fixed_point, height: png_fixed_point)
pub unsafe extern "C" fn png_set_sCAL_s(png_ptr: png_const_structrp, info_ptr: png_inforp, unit: c_int, swidth: png_const_charp, sheight: png_const_charp)
pub unsafe extern "C" fn png_set_sPLT(png_ptr: png_const_structrp, info_ptr: png_inforp, entries: png_const_sPLT_tp, nentries: c_int)
pub unsafe extern "C" fn png_set_sRGB(png_ptr: png_const_structrp, info_ptr: png_inforp, srgb_intent: c_int)
pub unsafe extern "C" fn png_set_sRGB_gAMA_and_cHRM(png_ptr: png_const_structrp, info_ptr: png_inforp, srgb_intent: c_int)
pub unsafe extern "C" fn png_set_scale_16(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_shift(png_ptr: png_structrp, true_bits: png_const_color_8p)
pub unsafe extern "C" fn png_set_sig_bytes(png_ptr: png_structrp, num_bytes: c_int)
pub unsafe extern "C" fn png_set_strip_16(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_strip_alpha(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_swap(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_swap_alpha(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_tIME(png_ptr: png_const_structrp, info_ptr: png_inforp, mod_time: png_const_timep)
pub unsafe extern "C" fn png_set_tRNS(png_ptr: png_structrp, info_ptr: png_inforp, trans_alpha: png_const_bytep, num_trans: c_int, trans_color: png_const_color_16p)
pub unsafe extern "C" fn png_set_tRNS_to_alpha(png_ptr: png_structrp)
pub unsafe extern "C" fn png_set_text(png_ptr: png_const_structrp, info_ptr: png_inforp, text_ptr: png_const_textp, num_text: c_int)
pub unsafe extern "C" fn png_set_text_2(png_ptr: png_const_structrp, info_ptr: png_inforp, text_ptr: png_const_textp, num_text: c_int) -> c_int
pub unsafe extern "C" fn png_set_text_compression_level(png_ptr: png_structrp, level: c_int)
pub unsafe extern "C" fn png_set_text_compression_mem_level(png_ptr: png_structrp, mem_level: c_int)
pub unsafe extern "C" fn png_set_text_compression_method(png_ptr: png_structrp, method: c_int)
pub unsafe extern "C" fn png_set_text_compression_strategy(png_ptr: png_structrp, strategy: c_int)
pub unsafe extern "C" fn png_set_text_compression_window_bits(png_ptr: png_structrp, window_bits: c_int)
pub unsafe extern "C" fn png_set_unknown_chunk_location(png_ptr: png_const_structrp, info_ptr: png_inforp, chunk: c_int, location: c_int)
pub unsafe extern "C" fn png_set_unknown_chunks(png_ptr: png_const_structrp, info_ptr: png_inforp, unknowns: png_const_unknown_chunkp, num_unknowns: c_int)
pub unsafe extern "C" fn png_set_user_limits(png_ptr: png_structrp, user_width_max: png_uint_32, user_height_max: png_uint_32)
pub unsafe extern "C" fn png_set_user_transform_info(png_ptr: png_structrp, user_transform_ptr: png_voidp, user_transform_depth: c_int, user_transform_channels: c_int)
pub unsafe extern "C" fn png_set_write_fn(png_ptr: png_structrp, io_ptr: png_voidp, write_data_fn: png_rw_ptr, output_flush_fn: png_flush_ptr)
pub unsafe extern "C" fn png_set_write_status_fn(png_ptr: png_structrp, write_row_fn: png_write_status_ptr)
pub unsafe extern "C" fn png_set_write_user_transform_fn(png_ptr: png_structrp, write_user_transform_fn: png_user_transform_ptr)
pub unsafe extern "C" fn png_sig_cmp(sig: png_const_bytep, start: usize, num_to_check: usize) -> c_int
pub unsafe extern "C" fn png_start_read_image(png_ptr: png_structrp)
pub unsafe extern "C" fn png_user_version_check(png_ptr: png_structrp, user_png_ver: png_const_charp) -> c_int
pub unsafe extern "C" fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp)
pub unsafe extern "C" fn png_warning_parameter(p: png_warning_parameters, number: c_int, string: png_const_charp)
pub unsafe extern "C" fn png_warning_parameter_signed(p: png_warning_parameters, number: c_int, format: c_int, value: png_int_32)
pub unsafe extern "C" fn png_warning_parameter_unsigned(p: png_warning_parameters, number: c_int, format: c_int, value: png_alloc_size_t)
pub unsafe extern "C" fn png_write_IEND(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_IHDR(png_ptr: png_structrp, width: png_uint_32, height: png_uint_32, bit_depth: c_int, color_type: c_int, compression_method: c_int, filter_method: c_int, interlace_method: c_int)
pub unsafe extern "C" fn png_write_PLTE(png_ptr: png_structrp, palette: png_const_colorp, num_pal: png_uint_32)
pub unsafe extern "C" fn png_write_bKGD(png_ptr: png_structrp, values: png_const_color_16p, color_type: c_int)
pub unsafe extern "C" fn png_write_cHRM_fixed(png_ptr: png_structrp, xy: *const png_xy)
pub unsafe extern "C" fn png_write_cICP(png_ptr: png_structrp, colour_primaries: png_byte, transfer_function: png_byte, matrix_coefficients: png_byte, video_full_range_flag: png_byte)
pub unsafe extern "C" fn png_write_cLLI_fixed(png_ptr: png_structrp, maxCLL: png_uint_32, maxFALL: png_uint_32)
pub unsafe extern "C" fn png_write_chunk(png_ptr: png_structrp, chunk_name: png_const_bytep, data: png_const_bytep, length: usize)
pub unsafe extern "C" fn png_write_chunk_data(png_ptr: png_structrp, data: png_const_bytep, length: usize)
pub unsafe extern "C" fn png_write_chunk_end(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_chunk_start(png_ptr: png_structrp, chunk_name: png_const_bytep, length: png_uint_32)
pub unsafe extern "C" fn png_write_data(png_ptr: png_structrp, data: png_const_bytep, length: usize)
pub unsafe extern "C" fn png_write_eXIf(png_ptr: png_structrp, exif: png_bytep, num_exif: c_int)
pub unsafe extern "C" fn png_write_end(png_ptr: png_structrp, info_ptr: png_inforp)
pub unsafe extern "C" fn png_write_find_filter(png_ptr: png_structrp, row_info: png_row_infop)
pub unsafe extern "C" fn png_write_finish_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_flush(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_gAMA_fixed(png_ptr: png_structrp, file_gamma: png_fixed_point)
pub unsafe extern "C" fn png_write_hIST(png_ptr: png_structrp, hist: png_const_uint_16p, num_hist: c_int)
pub unsafe extern "C" fn png_write_iCCP(png_ptr: png_structrp, name: png_const_charp, profile: png_const_bytep, proflen: png_uint_32)
pub unsafe extern "C" fn png_write_iTXt(png_ptr: png_structrp, compression: c_int, key: png_const_charp, lang: png_const_charp, lang_key: png_const_charp, text: png_const_charp)
pub unsafe extern "C" fn png_write_image(png_ptr: png_structrp, image: png_bytepp)
pub unsafe extern "C" fn png_write_info(png_ptr: png_structrp, info_ptr: png_const_inforp)
pub unsafe extern "C" fn png_write_info_before_PLTE(png_ptr: png_structrp, info_ptr: png_const_inforp)
pub unsafe extern "C" fn png_write_mDCV_fixed(png_ptr: png_structrp, red_x: png_uint_16, red_y: png_uint_16, green_x: png_uint_16, green_y: png_uint_16, blue_x: png_uint_16, blue_y: png_uint_16, white_x: png_uint_16, white_y: png_uint_16, maxDL: png_uint_32, minDL: png_uint_32)
pub unsafe extern "C" fn png_write_oFFs(png_ptr: png_structrp, x_offset: png_int_32, y_offset: png_int_32, unit_type: c_int)
pub unsafe extern "C" fn png_write_pCAL(png_ptr: png_structrp, purpose: png_charp, X0: png_int_32, X1: png_int_32, type_: c_int, nparams: c_int, units: png_const_charp, params: png_charpp)
pub unsafe extern "C" fn png_write_pHYs(png_ptr: png_structrp, x_pixels_per_unit: png_uint_32, y_pixels_per_unit: png_uint_32, unit_type: c_int)
pub unsafe extern "C" fn png_write_png(png_ptr: png_structrp, info_ptr: png_inforp, transforms: c_int, params: png_voidp)
pub unsafe extern "C" fn png_write_row(png_ptr: png_structrp, row: png_const_bytep)
pub unsafe extern "C" fn png_write_rows(png_ptr: png_structrp, row: png_bytepp, num_rows: png_uint_32)
pub unsafe extern "C" fn png_write_sBIT(png_ptr: png_structrp, sbit: png_const_color_8p, color_type: c_int)
pub unsafe extern "C" fn png_write_sCAL_s(png_ptr: png_structrp, unit: c_int, width: png_const_charp, height: png_const_charp)
pub unsafe extern "C" fn png_write_sPLT(png_ptr: png_structrp, palette: png_const_sPLT_tp)
pub unsafe extern "C" fn png_write_sRGB(png_ptr: png_structrp, intent: c_int)
pub unsafe extern "C" fn png_write_sig(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_start_row(png_ptr: png_structrp)
pub unsafe extern "C" fn png_write_tEXt(png_ptr: png_structrp, key: png_const_charp, text: png_const_charp, text_len: usize)
pub unsafe extern "C" fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep)
pub unsafe extern "C" fn png_write_tRNS(png_ptr: png_structrp, trans: png_const_bytep, values: png_const_color_16p, number: c_int, color_type: c_int)
pub unsafe extern "C" fn png_write_zTXt(png_ptr: png_structrp, key: png_const_charp, text: png_const_charp, compression: c_int)
pub unsafe extern "C" fn png_xy_from_XYZ(xy: *mut png_xy, XYZ: *const png_XYZ) -> c_int
pub unsafe extern "C" fn png_zalloc(png_ptr: voidpf, items: uInt, size: uInt) -> voidpf
pub unsafe extern "C" fn png_zfree(png_ptr: voidpf, ptr: voidpf)
pub unsafe extern "C" fn png_zlib_inflate(png_ptr: png_structrp, flush: c_int) -> c_int
pub unsafe extern "C" fn png_zstream_error(png_ptr: png_structrp, ret: c_int)
```
