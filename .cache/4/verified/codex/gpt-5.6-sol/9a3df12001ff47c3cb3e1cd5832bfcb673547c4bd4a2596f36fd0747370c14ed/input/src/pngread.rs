use ::c2rust_bitfields;
extern "C" {
    fn strtod(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
    ) -> ::core::ffi::c_double;
    fn strtol(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_long;
    fn strtoll(
        __nptr: *const ::core::ffi::c_char,
        __endptr: *mut *mut ::core::ffi::c_char,
        __base: ::core::ffi::c_int,
    ) -> ::core::ffi::c_longlong;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memset(
        __s: *mut ::core::ffi::c_void,
        __c: ::core::ffi::c_int,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    fn memcmp(
        __s1: *const ::core::ffi::c_void,
        __s2: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> ::core::ffi::c_int;
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fopen(
        __filename: *const ::core::ffi::c_char,
        __modes: *const ::core::ffi::c_char,
    ) -> *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop;
    fn png_set_expand(png_ptr: png_structrp);
    fn png_set_tRNS_to_alpha(png_ptr: png_structrp);
    fn png_set_expand_16(png_ptr: png_structrp);
    fn png_set_bgr(png_ptr: png_structrp);
    fn png_set_gray_to_rgb(png_ptr: png_structrp);
    fn png_set_rgb_to_gray_fixed(
        png_ptr: png_structrp,
        error_action: ::core::ffi::c_int,
        red: png_fixed_point,
        green: png_fixed_point,
    );
    fn png_set_alpha_mode_fixed(
        png_ptr: png_structrp,
        mode: ::core::ffi::c_int,
        output_gamma: png_fixed_point,
    );
    fn png_set_strip_alpha(png_ptr: png_structrp);
    fn png_set_swap_alpha(png_ptr: png_structrp);
    fn png_set_invert_alpha(png_ptr: png_structrp);
    fn png_set_add_alpha(png_ptr: png_structrp, filler: png_uint_32, flags: ::core::ffi::c_int);
    fn png_set_swap(png_ptr: png_structrp);
    fn png_set_packing(png_ptr: png_structrp);
    fn png_set_packswap(png_ptr: png_structrp);
    fn png_set_shift(png_ptr: png_structrp, true_bits: png_const_color_8p);
    fn png_set_interlace_handling(png_ptr: png_structrp) -> ::core::ffi::c_int;
    fn png_set_invert_mono(png_ptr: png_structrp);
    fn png_set_background_fixed(
        png_ptr: png_structrp,
        background_color: png_const_color_16p,
        background_gamma_code: ::core::ffi::c_int,
        need_expand: ::core::ffi::c_int,
        background_gamma: png_fixed_point,
    );
    fn png_set_scale_16(png_ptr: png_structrp);
    fn png_set_strip_16(png_ptr: png_structrp);
    fn png_destroy_info_struct(png_ptr: png_const_structrp, info_ptr_ptr: png_infopp);
    fn png_set_read_fn(png_ptr: png_structrp, io_ptr: png_voidp, read_data_fn: png_rw_ptr);
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_free_data(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        free_me: png_uint_32,
        num: ::core::ffi::c_int,
    );
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_chunk_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_chunk_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_set_benign_errors(png_ptr: png_structrp, allowed: ::core::ffi::c_int);
    fn png_get_rowbytes(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> size_t;
    fn png_get_channels(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> png_byte;
    fn png_set_keep_unknown_chunks(
        png_ptr: png_structrp,
        keep: ::core::ffi::c_int,
        chunk_list: png_const_bytep,
        num_chunks: ::core::ffi::c_int,
    );
    fn png_image_free(image: png_imagep);
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn inflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    static png_sRGB_table: [png_uint_16; 256];
    static png_sRGB_base: [png_uint_16; 512];
    static png_sRGB_delta: [png_byte; 512];
    fn png_create_png_struct(
        user_png_ver: png_const_charp,
        error_ptr: png_voidp,
        error_fn: png_error_ptr,
        warn_fn: png_error_ptr,
        mem_ptr: png_voidp,
        malloc_fn: png_malloc_ptr,
        free_fn: png_free_ptr,
    ) -> png_structp;
    fn png_destroy_png_struct(png_ptr: png_structrp);
    fn png_read_sig(png_ptr: png_structrp, info_ptr: png_inforp);
    fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32;
    fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> ::core::ffi::c_int;
    fn png_combine_row(png_ptr: png_const_structrp, row: png_bytep, display: ::core::ffi::c_int);
    fn png_do_read_interlace(
        row_info: png_row_infop,
        row: png_bytep,
        pass: ::core::ffi::c_int,
        transformations: png_uint_32,
    );
    fn png_read_filter_row(
        pp: png_structrp,
        row_info: png_row_infop,
        row: png_bytep,
        prev_row: png_const_bytep,
        filter: ::core::ffi::c_int,
    );
    fn png_read_IDAT_data(png_ptr: png_structrp, output: png_bytep, avail_out: png_alloc_size_t);
    fn png_read_finish_IDAT(png_ptr: png_structrp);
    fn png_read_finish_row(png_ptr: png_structrp);
    fn png_read_start_row(png_ptr: png_structrp);
    fn png_read_transform_info(png_ptr: png_structrp, info_ptr: png_inforp);
    fn png_handle_unknown(
        png_ptr: png_structrp,
        info_ptr: png_inforp,
        length: png_uint_32,
        keep: ::core::ffi::c_int,
    ) -> png_handle_result_code;
    fn png_handle_chunk(
        png_ptr: png_structrp,
        info_ptr: png_inforp,
        length: png_uint_32,
    ) -> png_handle_result_code;
    fn png_chunk_unknown_handling(
        png_ptr: png_const_structrp,
        chunk_name: png_uint_32,
    ) -> ::core::ffi::c_int;
    fn png_do_read_transformations(png_ptr: png_structrp, row_info: png_row_infop);
    fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_muldiv(
        res: png_fixed_point_p,
        a: png_fixed_point,
        multiplied_by: png_int_32,
        divided_by: png_int_32,
    ) -> ::core::ffi::c_int;
    fn png_reciprocal(a: png_fixed_point) -> png_fixed_point;
    fn png_gamma_significant(gamma_value: png_fixed_point) -> ::core::ffi::c_int;
    fn png_resolve_file_gamma(png_ptr: png_const_structrp) -> png_fixed_point;
    fn png_gamma_16bit_correct(
        value: ::core::ffi::c_uint,
        gamma_value: png_fixed_point,
    ) -> png_uint_16;
    fn png_destroy_gamma_table(png_ptr: png_structrp);
    fn png_safe_error(png_ptr: png_structp, error_message: png_const_charp) -> !;
    fn png_safe_warning(png_ptr: png_structp, warning_message: png_const_charp);
    fn png_safe_execute(
        image: png_imagep,
        function: Option<unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int>,
        arg: png_voidp,
    ) -> ::core::ffi::c_int;
    fn png_image_error(image: png_imagep, error_message: png_const_charp) -> ::core::ffi::c_int;
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __va_list_tag {
    pub gp_offset: ::core::ffi::c_uint,
    pub fp_offset: ::core::ffi::c_uint,
    pub overflow_arg_area: *mut ::core::ffi::c_void,
    pub reg_save_area: *mut ::core::ffi::c_void,
}
pub type size_t = usize;
pub type __compar_fn_t = Option<
    unsafe extern "C" fn(
        *const ::core::ffi::c_void,
        *const ::core::ffi::c_void,
    ) -> ::core::ffi::c_int,
>;
pub type ptrdiff_t = isize;
pub type __off_t = ::core::ffi::c_long;
pub type __off64_t = ::core::ffi::c_long;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct _IO_FILE {
    pub _flags: ::core::ffi::c_int,
    pub _IO_read_ptr: *mut ::core::ffi::c_char,
    pub _IO_read_end: *mut ::core::ffi::c_char,
    pub _IO_read_base: *mut ::core::ffi::c_char,
    pub _IO_write_base: *mut ::core::ffi::c_char,
    pub _IO_write_ptr: *mut ::core::ffi::c_char,
    pub _IO_write_end: *mut ::core::ffi::c_char,
    pub _IO_buf_base: *mut ::core::ffi::c_char,
    pub _IO_buf_end: *mut ::core::ffi::c_char,
    pub _IO_save_base: *mut ::core::ffi::c_char,
    pub _IO_backup_base: *mut ::core::ffi::c_char,
    pub _IO_save_end: *mut ::core::ffi::c_char,
    pub _markers: *mut ::core::ffi::c_void,
    pub _chain: *mut _IO_FILE,
    pub _fileno: ::core::ffi::c_int,
    pub _flags2: ::core::ffi::c_int,
    pub _old_offset: __off_t,
    pub _cur_column: ::core::ffi::c_ushort,
    pub _vtable_offset: ::core::ffi::c_schar,
    pub _shortbuf: [::core::ffi::c_char; 1],
    pub _lock: *mut ::core::ffi::c_void,
    pub _offset: __off64_t,
    pub _codecvt: *mut ::core::ffi::c_void,
    pub _wide_data: *mut ::core::ffi::c_void,
    pub _freeres_list: *mut _IO_FILE,
    pub _freeres_buf: *mut ::core::ffi::c_void,
    pub __pad5: size_t,
    pub _mode: ::core::ffi::c_int,
    pub _unused2: [::core::ffi::c_char; 20],
}
pub type _IO_lock_t = ();
pub type FILE = _IO_FILE;
pub type __jmp_buf = [::core::ffi::c_long; 8];
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __sigset_t {
    pub __val: [::core::ffi::c_ulong; 16],
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct __jmp_buf_tag {
    pub __jmpbuf: __jmp_buf,
    pub __mask_was_saved: ::core::ffi::c_int,
    pub __saved_mask: __sigset_t,
}
pub type jmp_buf = [__jmp_buf_tag; 1];
pub type png_byte = ::core::ffi::c_uchar;
pub type png_uint_16 = ::core::ffi::c_ushort;
pub type png_int_32 = ::core::ffi::c_int;
pub type png_uint_32 = ::core::ffi::c_uint;
pub type png_alloc_size_t = size_t;
pub type png_fixed_point = png_int_32;
pub type png_voidp = *mut ::core::ffi::c_void;
pub type png_const_voidp = *const ::core::ffi::c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_charp = *mut ::core::ffi::c_char;
pub type png_const_charp = *const ::core::ffi::c_char;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_bytepp = *mut *mut png_byte;
pub type png_uint_16pp = *mut *mut png_uint_16;
pub type png_charpp = *mut *mut ::core::ffi::c_char;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_struct_def {
    pub jmp_buf_local: jmp_buf,
    pub longjmp_fn: png_longjmp_ptr,
    pub jmp_buf_ptr: *mut jmp_buf,
    pub jmp_buf_size: size_t,
    pub error_fn: png_error_ptr,
    pub warning_fn: png_error_ptr,
    pub error_ptr: png_voidp,
    pub write_data_fn: png_rw_ptr,
    pub read_data_fn: png_rw_ptr,
    pub io_ptr: png_voidp,
    pub read_user_transform_fn: png_user_transform_ptr,
    pub write_user_transform_fn: png_user_transform_ptr,
    pub user_transform_ptr: png_voidp,
    pub user_transform_depth: png_byte,
    pub user_transform_channels: png_byte,
    pub mode: png_uint_32,
    pub flags: png_uint_32,
    pub transformations: png_uint_32,
    pub zowner: png_uint_32,
    pub zstream: z_stream,
    pub zbuffer_list: png_compression_bufferp,
    pub zbuffer_size: uInt,
    pub zlib_level: ::core::ffi::c_int,
    pub zlib_method: ::core::ffi::c_int,
    pub zlib_window_bits: ::core::ffi::c_int,
    pub zlib_mem_level: ::core::ffi::c_int,
    pub zlib_strategy: ::core::ffi::c_int,
    pub zlib_text_level: ::core::ffi::c_int,
    pub zlib_text_method: ::core::ffi::c_int,
    pub zlib_text_window_bits: ::core::ffi::c_int,
    pub zlib_text_mem_level: ::core::ffi::c_int,
    pub zlib_text_strategy: ::core::ffi::c_int,
    pub zlib_set_level: ::core::ffi::c_int,
    pub zlib_set_method: ::core::ffi::c_int,
    pub zlib_set_window_bits: ::core::ffi::c_int,
    pub zlib_set_mem_level: ::core::ffi::c_int,
    pub zlib_set_strategy: ::core::ffi::c_int,
    pub chunks: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub num_rows: png_uint_32,
    pub usr_width: png_uint_32,
    pub rowbytes: size_t,
    pub iwidth: png_uint_32,
    pub row_number: png_uint_32,
    pub chunk_name: png_uint_32,
    pub prev_row: png_bytep,
    pub row_buf: png_bytep,
    pub try_row: png_bytep,
    pub tst_row: png_bytep,
    pub info_rowbytes: size_t,
    pub idat_size: png_uint_32,
    pub crc: png_uint_32,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,
    pub num_palette_max: ::core::ffi::c_int,
    pub num_trans: png_uint_16,
    pub compression: png_byte,
    pub filter: png_byte,
    pub interlaced: png_byte,
    pub pass: png_byte,
    pub do_filter: png_byte,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub usr_bit_depth: png_byte,
    pub pixel_depth: png_byte,
    pub channels: png_byte,
    pub usr_channels: png_byte,
    pub sig_bytes: png_byte,
    pub maximum_pixel_depth: png_byte,
    pub transformed_pixel_depth: png_byte,
    pub zstream_start: png_byte,
    pub filler: png_uint_16,
    pub background_gamma_type: png_byte,
    pub background_gamma: png_fixed_point,
    pub background: png_color_16,
    pub background_1: png_color_16,
    pub output_flush_fn: png_flush_ptr,
    pub flush_dist: png_uint_32,
    pub flush_rows: png_uint_32,
    pub chromaticities: png_xy,
    pub gamma_shift: ::core::ffi::c_int,
    pub screen_gamma: png_fixed_point,
    pub file_gamma: png_fixed_point,
    pub chunk_gamma: png_fixed_point,
    pub default_gamma: png_fixed_point,
    pub gamma_table: png_bytep,
    pub gamma_16_table: png_uint_16pp,
    pub gamma_from_1: png_bytep,
    pub gamma_to_1: png_bytep,
    pub gamma_16_from_1: png_uint_16pp,
    pub gamma_16_to_1: png_uint_16pp,
    pub sig_bit: png_color_8,
    pub shift: png_color_8,
    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,
    pub read_row_fn: png_read_status_ptr,
    pub write_row_fn: png_write_status_ptr,
    pub info_fn: png_progressive_info_ptr,
    pub row_fn: png_progressive_row_ptr,
    pub end_fn: png_progressive_end_ptr,
    pub save_buffer_ptr: png_bytep,
    pub save_buffer: png_bytep,
    pub current_buffer_ptr: png_bytep,
    pub current_buffer: png_bytep,
    pub push_length: png_uint_32,
    pub skip_length: png_uint_32,
    pub save_buffer_size: size_t,
    pub save_buffer_max: size_t,
    pub buffer_size: size_t,
    pub current_buffer_size: size_t,
    pub process_mode: ::core::ffi::c_int,
    pub cur_palette: ::core::ffi::c_int,
    pub palette_lookup: png_bytep,
    pub quantize_index: png_bytep,
    pub options: png_uint_32,
    pub time_buffer: [::core::ffi::c_char; 29],
    pub free_me: png_uint_32,
    pub user_chunk_ptr: png_voidp,
    pub read_user_chunk_fn: png_user_chunk_ptr,
    pub unknown_default: ::core::ffi::c_int,
    pub num_chunk_list: ::core::ffi::c_uint,
    pub chunk_list: png_bytep,
    pub rgb_to_gray_status: png_byte,
    pub rgb_to_gray_coefficients_set: png_byte,
    pub rgb_to_gray_red_coeff: png_uint_16,
    pub rgb_to_gray_green_coeff: png_uint_16,
    pub riffled_palette: png_bytep,
    pub mng_features_permitted: png_uint_32,
    pub filter_type: png_byte,
    pub mem_ptr: png_voidp,
    pub malloc_fn: png_malloc_ptr,
    pub free_fn: png_free_ptr,
    pub big_row_buf: png_bytep,
    pub index_to_palette: png_bytep,
    pub palette_to_index: png_bytep,
    pub compression_type: png_byte,
    pub user_width_max: png_uint_32,
    pub user_height_max: png_uint_32,
    pub user_chunk_cache_max: png_uint_32,
    pub user_chunk_malloc_max: png_alloc_size_t,
    pub unknown_chunk: png_unknown_chunk,
    pub old_big_row_buf_size: size_t,
    pub read_buffer: png_bytep,
    pub read_buffer_size: png_alloc_size_t,
    pub IDAT_read_size: uInt,
    pub io_state: png_uint_32,
    pub big_prev_row: png_bytep,
    pub read_filter:
        [Option<unsafe extern "C" fn(png_row_infop, png_bytep, png_const_bytep) -> ()>; 4],
}
pub type png_row_infop = *mut png_row_info;
pub type png_row_info = png_row_info_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_row_info_struct {
    pub width: png_uint_32,
    pub rowbytes: size_t,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}
pub type uInt = ::core::ffi::c_uint;
pub type png_unknown_chunk = png_unknown_chunk_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_unknown_chunk_t {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: size_t,
    pub location: png_byte,
}
pub type png_free_ptr = Option<unsafe extern "C" fn(png_structp, png_voidp) -> ()>;
pub type png_structp = *mut png_struct;
pub type png_struct = png_struct_def;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_user_chunk_ptr =
    Option<unsafe extern "C" fn(png_structp, png_unknown_chunkp) -> ::core::ffi::c_int>;
pub type png_unknown_chunkp = *mut png_unknown_chunk;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(png_structp, png_infop) -> ()>;
pub type png_infop = *mut png_info;
pub type png_info = png_info_def;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_info_def {
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub valid: png_uint_32,
    pub rowbytes: size_t,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,
    pub num_trans: png_uint_16,
    pub bit_depth: png_byte,
    pub color_type: png_byte,
    pub compression_type: png_byte,
    pub filter_type: png_byte,
    pub interlace_type: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
    pub spare_byte: png_byte,
    pub signature: [png_byte; 8],
    pub cicp_colour_primaries: png_byte,
    pub cicp_transfer_function: png_byte,
    pub cicp_matrix_coefficients: png_byte,
    pub cicp_video_full_range_flag: png_byte,
    pub iccp_name: png_charp,
    pub iccp_profile: png_bytep,
    pub iccp_proflen: png_uint_32,
    pub maxCLL: png_uint_32,
    pub maxFALL: png_uint_32,
    pub mastering_red_x: png_uint_16,
    pub mastering_red_y: png_uint_16,
    pub mastering_green_x: png_uint_16,
    pub mastering_green_y: png_uint_16,
    pub mastering_blue_x: png_uint_16,
    pub mastering_blue_y: png_uint_16,
    pub mastering_white_x: png_uint_16,
    pub mastering_white_y: png_uint_16,
    pub mastering_maxDL: png_uint_32,
    pub mastering_minDL: png_uint_32,
    pub num_text: ::core::ffi::c_int,
    pub max_text: ::core::ffi::c_int,
    pub text: png_textp,
    pub mod_time: png_time,
    pub sig_bit: png_color_8,
    pub trans_alpha: png_bytep,
    pub trans_color: png_color_16,
    pub background: png_color_16,
    pub x_offset: png_int_32,
    pub y_offset: png_int_32,
    pub offset_unit_type: png_byte,
    pub x_pixels_per_unit: png_uint_32,
    pub y_pixels_per_unit: png_uint_32,
    pub phys_unit_type: png_byte,
    pub num_exif: png_uint_32,
    pub exif: png_bytep,
    pub hist: png_uint_16p,
    pub pcal_purpose: png_charp,
    pub pcal_X0: png_int_32,
    pub pcal_X1: png_int_32,
    pub pcal_units: png_charp,
    pub pcal_params: png_charpp,
    pub pcal_type: png_byte,
    pub pcal_nparams: png_byte,
    pub free_me: png_uint_32,
    pub unknown_chunks: png_unknown_chunkp,
    pub unknown_chunks_num: ::core::ffi::c_int,
    pub splt_palettes: png_sPLT_tp,
    pub splt_palettes_num: ::core::ffi::c_int,
    pub scal_unit: png_byte,
    pub scal_s_width: png_charp,
    pub scal_s_height: png_charp,
    pub row_pointers: png_bytepp,
    pub cHRM: png_xy,
    pub gamma: png_fixed_point,
    pub rendering_intent: ::core::ffi::c_int,
}
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_xy {
    pub redx: png_fixed_point,
    pub redy: png_fixed_point,
    pub greenx: png_fixed_point,
    pub greeny: png_fixed_point,
    pub bluex: png_fixed_point,
    pub bluey: png_fixed_point,
    pub whitex: png_fixed_point,
    pub whitey: png_fixed_point,
}
pub type png_sPLT_tp = *mut png_sPLT_t;
pub type png_sPLT_t = png_sPLT_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_sPLT_struct {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: png_sPLT_entryp,
    pub nentries: png_int_32,
}
pub type png_sPLT_entryp = *mut png_sPLT_entry;
pub type png_sPLT_entry = png_sPLT_entry_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_sPLT_entry_struct {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}
pub type png_color_16 = png_color_16_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_color_16_struct {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}
pub type png_color_8 = png_color_8_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_color_8_struct {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}
pub type png_time = png_time_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_time_struct {
    pub year: png_uint_16,
    pub month: png_byte,
    pub day: png_byte,
    pub hour: png_byte,
    pub minute: png_byte,
    pub second: png_byte,
}
pub type png_textp = *mut png_text;
pub type png_text = png_text_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_text_struct {
    pub compression: ::core::ffi::c_int,
    pub key: png_charp,
    pub text: png_charp,
    pub text_length: size_t,
    pub itxt_length: size_t,
    pub lang: png_charp,
    pub lang_key: png_charp,
}
pub type png_colorp = *mut png_color;
pub type png_color = png_color_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_color_struct {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(png_structp, png_bytep, png_uint_32, ::core::ffi::c_int) -> ()>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(png_structp, png_infop) -> ()>;
pub type png_write_status_ptr =
    Option<unsafe extern "C" fn(png_structp, png_uint_32, ::core::ffi::c_int) -> ()>;
pub type png_read_status_ptr =
    Option<unsafe extern "C" fn(png_structp, png_uint_32, ::core::ffi::c_int) -> ()>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(png_structp) -> ()>;
pub type png_compression_bufferp = *mut png_compression_buffer;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1],
}
pub type z_stream = z_stream_s;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct z_stream_s {
    pub next_in: *const Bytef,
    pub avail_in: uInt,
    pub total_in: uLong,
    pub next_out: *mut Bytef,
    pub avail_out: uInt,
    pub total_out: uLong,
    pub msg: *const ::core::ffi::c_char,
    pub state: *mut ::core::ffi::c_void,
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: ::core::ffi::c_int,
    pub adler: uLong,
    pub reserved: uLong,
}
pub type uLong = ::core::ffi::c_ulong;
pub type voidpf = *mut ::core::ffi::c_void;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf) -> ()>;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type Bytef = Byte;
pub type Byte = ::core::ffi::c_uchar;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(png_structp, png_row_infop, png_bytep) -> ()>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(png_structp, png_bytep, size_t) -> ()>;
pub type png_error_ptr = Option<unsafe extern "C" fn(png_structp, png_const_charp) -> ()>;
pub type png_longjmp_ptr =
    Option<unsafe extern "C" fn(*mut __jmp_buf_tag, ::core::ffi::c_int) -> ()>;
pub type png_structpp = *mut *mut png_struct;
pub type png_infopp = *mut *mut png_info;
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;
pub type png_inforp = *mut png_info;
pub type png_const_inforp = *const png_info;
pub type png_const_colorp = *const png_color;
pub type png_const_color_16p = *const png_color_16;
pub type png_const_color_8p = *const png_color_8;
pub type png_handle_result_code = ::core::ffi::c_uint;
pub const handled_ok: png_handle_result_code = 3;
pub const handled_saved: png_handle_result_code = 2;
pub const handled_discarded: png_handle_result_code = 1;
pub const handled_error: png_handle_result_code = 0;
pub type z_streamp = *mut z_stream;
#[derive(Copy, Clone, BitfieldStruct)]
#[repr(C)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,
    pub memory: png_const_bytep,
    pub size: size_t,
    #[bitfield(name = "for_write", ty = "::core::ffi::c_uint", bits = "0..=0")]
    #[bitfield(name = "owned_file", ty = "::core::ffi::c_uint", bits = "1..=1")]
    pub for_write_owned_file: [u8; 1],
    #[bitfield(padding)]
    pub c2rust_padding: [u8; 7],
}
pub type png_controlp = *mut png_control;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct C2RustUnnamed {
    pub opaque: png_controlp,
    pub version: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub format: png_uint_32,
    pub flags: png_uint_32,
    pub colormap_entries: png_uint_32,
    pub warning_or_error: png_uint_32,
    pub message: [::core::ffi::c_char; 64],
}
pub type png_imagep = *mut C2RustUnnamed;
pub const PNG_INDEX_cHRM: C2RustUnnamed_0 = 6;
pub const PNG_INDEX_sRGB: C2RustUnnamed_0 = 23;
pub const PNG_INDEX_mDCV: C2RustUnnamed_0 = 16;
pub const PNG_INDEX_cICP: C2RustUnnamed_0 = 7;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_image_read_control {
    pub image: png_imagep,
    pub buffer: png_voidp,
    pub row_stride: png_int_32,
    pub colormap: png_voidp,
    pub background: png_const_colorp,
    pub local_row: png_voidp,
    pub first_row: png_voidp,
    pub row_step: ptrdiff_t,
    pub file_encoding: ::core::ffi::c_int,
    pub gamma_to_linear: png_fixed_point,
    pub colormap_processing: ::core::ffi::c_int,
}
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const PNG_INDEX_unknown: C2RustUnnamed_0 = 28;
pub const PNG_INDEX_zTXt: C2RustUnnamed_0 = 27;
pub const PNG_INDEX_tRNS: C2RustUnnamed_0 = 26;
pub const PNG_INDEX_tIME: C2RustUnnamed_0 = 25;
pub const PNG_INDEX_tEXt: C2RustUnnamed_0 = 24;
pub const PNG_INDEX_sPLT: C2RustUnnamed_0 = 22;
pub const PNG_INDEX_sCAL: C2RustUnnamed_0 = 21;
pub const PNG_INDEX_sBIT: C2RustUnnamed_0 = 20;
pub const PNG_INDEX_pHYs: C2RustUnnamed_0 = 19;
pub const PNG_INDEX_pCAL: C2RustUnnamed_0 = 18;
pub const PNG_INDEX_oFFs: C2RustUnnamed_0 = 17;
pub const PNG_INDEX_iTXt: C2RustUnnamed_0 = 15;
pub const PNG_INDEX_iCCP: C2RustUnnamed_0 = 14;
pub const PNG_INDEX_hIST: C2RustUnnamed_0 = 13;
pub const PNG_INDEX_gAMA: C2RustUnnamed_0 = 12;
pub const PNG_INDEX_fdAT: C2RustUnnamed_0 = 11;
pub const PNG_INDEX_fcTL: C2RustUnnamed_0 = 10;
pub const PNG_INDEX_eXIf: C2RustUnnamed_0 = 9;
pub const PNG_INDEX_cLLI: C2RustUnnamed_0 = 8;
pub const PNG_INDEX_bKGD: C2RustUnnamed_0 = 5;
pub const PNG_INDEX_acTL: C2RustUnnamed_0 = 4;
pub const PNG_INDEX_IEND: C2RustUnnamed_0 = 3;
pub const PNG_INDEX_IDAT: C2RustUnnamed_0 = 2;
pub const PNG_INDEX_PLTE: C2RustUnnamed_0 = 1;
pub const PNG_INDEX_IHDR: C2RustUnnamed_0 = 0;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const NULL_0: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
#[inline]
unsafe extern "C" fn atoi(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_int {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    ) as ::core::ffi::c_int;
}
#[inline]
unsafe extern "C" fn atol(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_long {
    return strtol(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn atoll(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_longlong {
    return strtoll(
        __nptr,
        NULL as *mut *mut ::core::ffi::c_char,
        10 as ::core::ffi::c_int,
    );
}
#[inline]
unsafe extern "C" fn bsearch(
    mut __key: *const ::core::ffi::c_void,
    mut __base: *const ::core::ffi::c_void,
    mut __nmemb: size_t,
    mut __size: size_t,
    mut __compar: __compar_fn_t,
) -> *mut ::core::ffi::c_void {
    let mut __l: size_t = 0;
    let mut __u: size_t = 0;
    let mut __idx: size_t = 0;
    let mut __p: *const ::core::ffi::c_void = ::core::ptr::null::<::core::ffi::c_void>();
    let mut __comparison: ::core::ffi::c_int = 0;
    __l = 0 as size_t;
    __u = __nmemb;
    while __l < __u {
        __idx = __l.wrapping_add(__u).wrapping_div(2 as size_t);
        __p = (__base as *const ::core::ffi::c_char).offset(__idx.wrapping_mul(__size) as isize)
            as *const ::core::ffi::c_void;
        __comparison = Some(__compar.expect("non-null function pointer"))
            .expect("non-null function pointer")(__key, __p);
        if __comparison < 0 as ::core::ffi::c_int {
            __u = __idx;
        } else if __comparison > 0 as ::core::ffi::c_int {
            __l = __idx.wrapping_add(1 as size_t);
        } else {
            return __p as *mut ::core::ffi::c_void;
        }
    }
    return NULL;
}
#[inline]
unsafe extern "C" fn atof(mut __nptr: *const ::core::ffi::c_char) -> ::core::ffi::c_double {
    return strtod(__nptr, NULL as *mut *mut ::core::ffi::c_char);
}
#[inline]
unsafe extern "C" fn vprintf(
    mut __fmt: *const ::core::ffi::c_char,
    mut __arg: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    return vfprintf(stdout, __fmt, __arg);
}
#[inline]
unsafe extern "C" fn getchar() -> ::core::ffi::c_int {
    return getc(stdin);
}
#[inline]
unsafe extern "C" fn putchar(mut __c: ::core::ffi::c_int) -> ::core::ffi::c_int {
    return putc(__c, stdout);
}
pub const PNG_LIBPNG_VER_STRING: [::core::ffi::c_char; 11] =
    unsafe { ::core::mem::transmute::<[u8; 11], [::core::ffi::c_char; 11]>(*b"1.6.59.git\0") };
pub const PNG_HAVE_IHDR: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_HAVE_PLTE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_AFTER_IDAT: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_UINT_32_MAX: png_uint_32 = -(1 as ::core::ffi::c_int) as png_uint_32;
pub const PNG_FP_1: ::core::ffi::c_int = 100000 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_GRAY: ::core::ffi::c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_INTRAPIXEL_DIFFERENCING: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const PNG_INTERLACE_NONE: ::core::ffi::c_int = 0;
pub const PNG_INTERLACE_ADAM7: ::core::ffi::c_int = 1;
pub const PNG_INFO_sBIT: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_INFO_IDAT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_TRANSFORM_STRIP_16: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_STRIP_ALPHA: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_PACKING: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_PACKSWAP: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_EXPAND: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_INVERT_MONO: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SHIFT: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_BGR: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SWAP_ALPHA: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SWAP_ENDIAN: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_INVERT_ALPHA: ::core::ffi::c_int = 0x400 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_GRAY_TO_RGB: ::core::ffi::c_int = 0x2000 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_EXPAND_16: ::core::ffi::c_int = 0x4000 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SCALE_16: ::core::ffi::c_int = 0x8000 as ::core::ffi::c_int;
pub const PNG_FLAG_MNG_FILTER_64: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_ERROR_ACTION_NONE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_RGB_TO_GRAY_DEFAULT: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_ALPHA_PNG: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_ALPHA_STANDARD: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_ALPHA_OPTIMIZED: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_DEFAULT_sRGB: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_GAMMA_sRGB: ::core::ffi::c_int = 220000 as ::core::ffi::c_int;
pub const PNG_GAMMA_LINEAR: ::core::ffi::c_int = PNG_FP_1;
pub const PNG_FILLER_BEFORE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILLER_AFTER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_BACKGROUND_GAMMA_SCREEN: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_LAST: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PNG_FREE_ROWS: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_HANDLE_CHUNK_AS_DEFAULT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_HANDLE_CHUNK_NEVER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_INTERLACE_ADAM7_PASSES: ::core::ffi::c_int = 7 as ::core::ffi::c_int;
pub const PNG_IMAGE_VERSION: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_FORMAT_FLAG_ALPHA: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_COLOR: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_LINEAR: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_COLORMAP: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_BGR: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_AFIRST: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_IDAT_READ_SIZE: ::core::ffi::c_int = PNG_ZBUF_SIZE;
pub const PNG_ZBUF_SIZE: ::core::ffi::c_int = 8192 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct(
    mut user_png_ver: png_const_charp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warn_fn: png_error_ptr,
) -> png_structp {
    return png_create_read_struct_2(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        NULL_0,
        None,
        None,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_read_struct_2(
    mut user_png_ver: png_const_charp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warn_fn: png_error_ptr,
    mut mem_ptr: png_voidp,
    mut malloc_fn: png_malloc_ptr,
    mut free_fn: png_free_ptr,
) -> png_structp {
    let mut png_ptr: png_structp = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    );
    if !png_ptr.is_null() {
        (*png_ptr).mode = PNG_IS_READ_STRUCT as png_uint_32;
        (*png_ptr).IDAT_read_size = PNG_IDAT_READ_SIZE as uInt;
        (*png_ptr).flags |= PNG_FLAG_BENIGN_ERRORS_WARN;
        png_set_read_fn(png_ptr as png_structrp, NULL_0, None);
    }
    return png_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_info(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    let mut keep: ::core::ffi::c_int = 0;
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    png_read_sig(png_ptr, info_ptr);
    loop {
        let mut length: png_uint_32 = png_read_chunk_header(png_ptr);
        let mut chunk_name: png_uint_32 = (*png_ptr).chunk_name;
        if chunk_name == png_IDAT {
            if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IHDR as ::core::ffi::c_uint
                == 0 as ::core::ffi::c_uint
            {
                png_chunk_error(
                    png_ptr,
                    b"Missing IHDR before IDAT\0" as *const u8 as png_const_charp,
                );
            } else if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
                && (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PLTE as ::core::ffi::c_uint
                    == 0 as ::core::ffi::c_uint
            {
                png_chunk_error(
                    png_ptr,
                    b"Missing PLTE before IDAT\0" as *const u8 as png_const_charp,
                );
            } else if (*png_ptr).mode as ::core::ffi::c_uint & PNG_AFTER_IDAT as ::core::ffi::c_uint
                != 0 as ::core::ffi::c_uint
            {
                png_chunk_benign_error(
                    png_ptr,
                    b"Too many IDATs found\0" as *const u8 as png_const_charp,
                );
            }
            (*png_ptr).mode |= PNG_HAVE_IDAT;
        } else if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT != 0 as ::core::ffi::c_uint
        {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
            (*png_ptr).mode |= PNG_AFTER_IDAT as ::core::ffi::c_uint;
        }
        if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            if keep != 0 as ::core::ffi::c_int {
                png_handle_unknown(png_ptr, info_ptr, length, keep);
                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE as ::core::ffi::c_uint;
                } else {
                    if !(chunk_name == png_IDAT) {
                        continue;
                    }
                    (*png_ptr).idat_size = 0 as png_uint_32;
                    break;
                }
            } else if chunk_name == png_IDAT {
                (*png_ptr).idat_size = length;
                break;
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_update_info(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    if !png_ptr.is_null() {
        if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT == 0 as ::core::ffi::c_uint {
            png_read_start_row(png_ptr);
            png_read_transform_info(png_ptr, info_ptr);
        } else {
            png_app_error(
                png_ptr,
                b"png_read_update_info/png_start_read_image: duplicate call\0" as *const u8
                    as png_const_charp,
            );
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_start_read_image(mut png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT == 0 as ::core::ffi::c_uint {
            png_read_start_row(png_ptr);
        } else {
            png_app_error(
                png_ptr,
                b"png_start_read_image/png_read_update_info: duplicate call\0" as *const u8
                    as png_const_charp,
            );
        }
    }
}
unsafe extern "C" fn png_do_read_intrapixel(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
        != 0 as ::core::ffi::c_int
    {
        let mut bytes_per_pixel: ::core::ffi::c_int = 0;
        let mut row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i: png_uint_32 = 0;
            if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 3 as ::core::ffi::c_int;
            } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 4 as ::core::ffi::c_int;
            } else {
                return;
            }
            i = 0 as png_uint_32;
            rp = row;
            while i < row_width {
                *rp = (256 as ::core::ffi::c_int
                    + *rp as ::core::ffi::c_int
                    + *rp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int) as png_byte;
                *rp.offset(2 as ::core::ffi::c_int as isize) = (256 as ::core::ffi::c_int
                    + *rp.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    + *rp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int)
                    as png_byte;
                i = i.wrapping_add(1);
                rp = rp.offset(bytes_per_pixel as isize);
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            let mut rp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_0: png_uint_32 = 0;
            if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB {
                bytes_per_pixel = 6 as ::core::ffi::c_int;
            } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                bytes_per_pixel = 8 as ::core::ffi::c_int;
            } else {
                return;
            }
            i_0 = 0 as png_uint_32;
            rp_0 = row;
            while i_0 < row_width {
                let mut s0: png_uint_32 = ((*rp_0 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                    as png_uint_32
                    | *rp_0.offset(1 as ::core::ffi::c_int as isize) as png_uint_32;
                let mut s1: png_uint_32 =
                    ((*rp_0.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int) as png_uint_32
                        | *rp_0.offset(3 as ::core::ffi::c_int as isize) as png_uint_32;
                let mut s2: png_uint_32 =
                    ((*rp_0.offset(4 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int) as png_uint_32
                        | *rp_0.offset(5 as ::core::ffi::c_int as isize) as png_uint_32;
                let mut red: png_uint_32 = s0
                    .wrapping_add(s1)
                    .wrapping_add(65536 as ::core::ffi::c_int as png_uint_32)
                    & 0xffff as png_uint_32;
                let mut blue: png_uint_32 = s2
                    .wrapping_add(s1)
                    .wrapping_add(65536 as ::core::ffi::c_int as png_uint_32)
                    & 0xffff as png_uint_32;
                *rp_0 = (red as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_uint) as png_byte;
                *rp_0.offset(1 as ::core::ffi::c_int as isize) =
                    (red as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint) as png_byte;
                *rp_0.offset(4 as ::core::ffi::c_int as isize) =
                    (blue as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int
                        & 0xff as ::core::ffi::c_uint) as png_byte;
                *rp_0.offset(5 as ::core::ffi::c_int as isize) =
                    (blue as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint) as png_byte;
                i_0 = i_0.wrapping_add(1);
                rp_0 = rp_0.offset(bytes_per_pixel as isize);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_row(
    mut png_ptr: png_structrp,
    mut row: png_bytep,
    mut dsp_row: png_bytep,
) {
    let mut row_info: png_row_info = png_row_info {
        width: 0,
        rowbytes: 0,
        color_type: 0,
        bit_depth: 0,
        channels: 0,
        pixel_depth: 0,
    };
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT == 0 as ::core::ffi::c_uint {
        png_read_start_row(png_ptr);
    }
    row_info.width = (*png_ptr).iwidth;
    row_info.color_type = (*png_ptr).color_type;
    row_info.bit_depth = (*png_ptr).bit_depth;
    row_info.channels = (*png_ptr).channels;
    row_info.pixel_depth = (*png_ptr).pixel_depth;
    row_info.rowbytes = if row_info.pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
        (row_info.width as size_t)
            .wrapping_mul(row_info.pixel_depth as size_t >> 3 as ::core::ffi::c_int)
    } else {
        (row_info.width as size_t)
            .wrapping_mul(row_info.pixel_depth as size_t)
            .wrapping_add(7 as size_t)
            >> 3 as ::core::ffi::c_int
    };
    (*png_ptr).row_number == 0 as ::core::ffi::c_uint
        && (*png_ptr).pass as ::core::ffi::c_int == 0 as ::core::ffi::c_int;
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            != 0 as ::core::ffi::c_uint
    {
        match (*png_ptr).pass as ::core::ffi::c_int {
            0 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint != 0 {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            1 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint != 0
                    || (*png_ptr).width < 5 as ::core::ffi::c_uint
                {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            2 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint
                    != 4 as ::core::ffi::c_uint
                {
                    if !dsp_row.is_null()
                        && (*png_ptr).row_number as ::core::ffi::c_uint & 4 as ::core::ffi::c_uint
                            != 0
                    {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            3 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 3 as ::core::ffi::c_uint != 0
                    || (*png_ptr).width < 3 as ::core::ffi::c_uint
                {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            4 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 3 as ::core::ffi::c_uint
                    != 2 as ::core::ffi::c_uint
                {
                    if !dsp_row.is_null()
                        && (*png_ptr).row_number as ::core::ffi::c_uint & 2 as ::core::ffi::c_uint
                            != 0
                    {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            5 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 1 as ::core::ffi::c_uint != 0
                    || (*png_ptr).width < 2 as ::core::ffi::c_uint
                {
                    if !dsp_row.is_null() {
                        png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
                    }
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
            6 | _ => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 1 as ::core::ffi::c_uint
                    == 0 as ::core::ffi::c_uint
                {
                    png_read_finish_row(png_ptr);
                    return;
                }
            }
        }
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"Invalid attempt to read row data\0" as *const u8 as png_const_charp,
        );
    }
    *(*png_ptr).row_buf.offset(0 as ::core::ffi::c_int as isize) = 255 as png_byte;
    png_read_IDAT_data(
        png_ptr,
        (*png_ptr).row_buf,
        (row_info.rowbytes as png_alloc_size_t).wrapping_add(1 as png_alloc_size_t),
    );
    if *(*png_ptr).row_buf.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
        > PNG_FILTER_VALUE_NONE
    {
        if (*(*png_ptr).row_buf.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
            < PNG_FILTER_VALUE_LAST
        {
            png_read_filter_row(
                png_ptr,
                &raw mut row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize) as png_const_bytep,
                *(*png_ptr).row_buf.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int,
            );
        } else {
            png_error(
                png_ptr,
                b"bad adaptive filter value\0" as *const u8 as png_const_charp,
            );
        }
    }
    memcpy(
        (*png_ptr).prev_row as *mut ::core::ffi::c_void,
        (*png_ptr).row_buf as *const ::core::ffi::c_void,
        row_info.rowbytes.wrapping_add(1 as size_t),
    );
    if (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
        & PNG_FLAG_MNG_FILTER_64 as ::core::ffi::c_uint
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).filter_type as ::core::ffi::c_int == PNG_INTRAPIXEL_DIFFERENCING
    {
        png_do_read_intrapixel(
            &raw mut row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations != 0 || (*png_ptr).num_palette_max >= 0 as ::core::ffi::c_int {
        png_do_read_transformations(png_ptr, &raw mut row_info);
    }
    if (*png_ptr).transformed_pixel_depth as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth as ::core::ffi::c_int
            > (*png_ptr).maximum_pixel_depth as ::core::ffi::c_int
        {
            png_error(
                png_ptr,
                b"sequential row overflow\0" as *const u8 as png_const_charp,
            );
        }
    } else if (*png_ptr).transformed_pixel_depth as ::core::ffi::c_int
        != row_info.pixel_depth as ::core::ffi::c_int
    {
        png_error(
            png_ptr,
            b"internal sequential row size calculation error\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            != 0 as ::core::ffi::c_uint
    {
        if ((*png_ptr).pass as ::core::ffi::c_int) < 6 as ::core::ffi::c_int {
            png_do_read_interlace(
                &raw mut row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                (*png_ptr).pass as ::core::ffi::c_int,
                (*png_ptr).transformations,
            );
        }
        if !dsp_row.is_null() {
            png_combine_row(png_ptr, dsp_row, 1 as ::core::ffi::c_int);
        }
        if !row.is_null() {
            png_combine_row(png_ptr, row, 0 as ::core::ffi::c_int);
        }
    } else {
        if !row.is_null() {
            png_combine_row(png_ptr, row, -(1 as ::core::ffi::c_int));
        }
        if !dsp_row.is_null() {
            png_combine_row(png_ptr, dsp_row, -(1 as ::core::ffi::c_int));
        }
    }
    png_read_finish_row(png_ptr);
    if (*png_ptr).read_row_fn.is_some() {
        Some((*png_ptr).read_row_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as png_structp,
            (*png_ptr).row_number,
            (*png_ptr).pass as ::core::ffi::c_int,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_rows(
    mut png_ptr: png_structrp,
    mut row: png_bytepp,
    mut display_row: png_bytepp,
    mut num_rows: png_uint_32,
) {
    let mut i: png_uint_32 = 0;
    let mut rp: png_bytepp = ::core::ptr::null_mut::<*mut png_byte>();
    let mut dp: png_bytepp = ::core::ptr::null_mut::<*mut png_byte>();
    if png_ptr.is_null() {
        return;
    }
    rp = row;
    dp = display_row;
    if !rp.is_null() && !dp.is_null() {
        i = 0 as png_uint_32;
        while i < num_rows {
            let fresh0 = rp;
            rp = rp.offset(1);
            let mut rptr: png_bytep = *fresh0;
            let fresh1 = dp;
            dp = dp.offset(1);
            let mut dptr: png_bytep = *fresh1;
            png_read_row(png_ptr, rptr, dptr);
            i = i.wrapping_add(1);
        }
    } else if !rp.is_null() {
        i = 0 as png_uint_32;
        while i < num_rows {
            let mut rptr_0: png_bytep = *rp;
            png_read_row(png_ptr, rptr_0, ::core::ptr::null_mut::<png_byte>());
            rp = rp.offset(1);
            i = i.wrapping_add(1);
        }
    } else if !dp.is_null() {
        i = 0 as png_uint_32;
        while i < num_rows {
            let mut dptr_0: png_bytep = *dp;
            png_read_row(png_ptr, ::core::ptr::null_mut::<png_byte>(), dptr_0);
            dp = dp.offset(1);
            i = i.wrapping_add(1);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_image(mut png_ptr: png_structrp, mut image: png_bytepp) {
    let mut i: png_uint_32 = 0;
    let mut image_height: png_uint_32 = 0;
    let mut pass: ::core::ffi::c_int = 0;
    let mut j: ::core::ffi::c_int = 0;
    let mut rp: png_bytepp = ::core::ptr::null_mut::<*mut png_byte>();
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT == 0 as ::core::ffi::c_uint {
        pass = png_set_interlace_handling(png_ptr);
        png_start_read_image(png_ptr);
    } else {
        if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
                == 0 as ::core::ffi::c_uint
        {
            png_warning(
                png_ptr,
                b"Interlace handling should be turned on when using png_read_image\0" as *const u8
                    as png_const_charp,
            );
            (*png_ptr).num_rows = (*png_ptr).height;
        }
        pass = png_set_interlace_handling(png_ptr);
    }
    image_height = (*png_ptr).height;
    j = 0 as ::core::ffi::c_int;
    while j < pass {
        rp = image;
        i = 0 as png_uint_32;
        while i < image_height {
            png_read_row(png_ptr, *rp, ::core::ptr::null_mut::<png_byte>());
            rp = rp.offset(1);
            i = i.wrapping_add(1);
        }
        j += 1;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_end(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    let mut keep: ::core::ffi::c_int = 0;
    if png_ptr.is_null() {
        return;
    }
    if png_chunk_unknown_handling(png_ptr, png_IDAT) == 0 as ::core::ffi::c_int {
        png_read_finish_IDAT(png_ptr);
    }
    if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as ::core::ffi::c_int
    {
        png_benign_error(
            png_ptr,
            b"Read palette index exceeding num_palette\0" as *const u8 as png_const_charp,
        );
    }
    loop {
        let mut length: png_uint_32 = png_read_chunk_header(png_ptr);
        let mut chunk_name: png_uint_32 = (*png_ptr).chunk_name;
        if chunk_name != png_IDAT {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT as ::core::ffi::c_uint;
        }
        if chunk_name == png_IEND {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if chunk_name == png_IHDR {
            png_handle_chunk(png_ptr, info_ptr, length);
        } else if info_ptr.is_null() {
            png_crc_finish(png_ptr, length);
        } else {
            keep = png_chunk_unknown_handling(png_ptr, chunk_name);
            if keep != 0 as ::core::ffi::c_int {
                if chunk_name == png_IDAT {
                    if length > 0 as ::core::ffi::c_uint
                        && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_ENDED == 0
                        || (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_CHUNK_AFTER_IDAT
                            != 0 as ::core::ffi::c_uint
                    {
                        png_benign_error(
                            png_ptr,
                            b".Too many IDATs found\0" as *const u8 as png_const_charp,
                        );
                    }
                }
                png_handle_unknown(png_ptr, info_ptr, length, keep);
                if chunk_name == png_PLTE {
                    (*png_ptr).mode |= PNG_HAVE_PLTE as ::core::ffi::c_uint;
                }
            } else if chunk_name == png_IDAT {
                if length > 0 as ::core::ffi::c_uint
                    && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_ENDED == 0
                    || (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_CHUNK_AFTER_IDAT
                        != 0 as ::core::ffi::c_uint
                {
                    png_benign_error(
                        png_ptr,
                        b"..Too many IDATs found\0" as *const u8 as png_const_charp,
                    );
                }
                png_crc_finish(png_ptr, length);
            } else {
                png_handle_chunk(png_ptr, info_ptr, length);
            }
        }
        if !((*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IEND == 0 as ::core::ffi::c_uint) {
            break;
        }
    }
}
unsafe extern "C" fn png_read_destroy(mut png_ptr: png_structrp) {
    png_destroy_gamma_table(png_ptr);
    png_free(png_ptr, (*png_ptr).big_row_buf as png_voidp);
    (*png_ptr).big_row_buf = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).big_prev_row as png_voidp);
    (*png_ptr).big_prev_row = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).read_buffer as png_voidp);
    (*png_ptr).read_buffer = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).palette_lookup as png_voidp);
    (*png_ptr).palette_lookup = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
    (*png_ptr).quantize_index = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = ::core::ptr::null_mut::<png_color>();
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = ::core::ptr::null_mut::<png_byte>();
    inflateEnd(&raw mut (*png_ptr).zstream);
    png_free(png_ptr, (*png_ptr).save_buffer as png_voidp);
    (*png_ptr).save_buffer = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).unknown_chunk.data as png_voidp);
    (*png_ptr).unknown_chunk.data = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).riffled_palette as png_voidp);
    (*png_ptr).riffled_palette = ::core::ptr::null_mut::<png_byte>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_read_struct(
    mut png_ptr_ptr: png_structpp,
    mut info_ptr_ptr: png_infopp,
    mut end_info_ptr_ptr: png_infopp,
) {
    let mut png_ptr: png_structrp = ::core::ptr::null_mut::<png_struct>();
    if !png_ptr_ptr.is_null() {
        png_ptr = *png_ptr_ptr as png_structrp;
    }
    if png_ptr.is_null() {
        return;
    }
    png_destroy_info_struct(png_ptr, end_info_ptr_ptr);
    png_destroy_info_struct(png_ptr, info_ptr_ptr);
    *png_ptr_ptr = ::core::ptr::null_mut::<png_struct>();
    png_read_destroy(png_ptr);
    png_destroy_png_struct(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_status_fn(
    mut png_ptr: png_structrp,
    mut read_row_fn: png_read_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).read_row_fn = read_row_fn;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_png(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
    mut transforms: ::core::ffi::c_int,
    mut params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    png_read_info(png_ptr, info_ptr);
    if (*info_ptr).height as usize
        > (PNG_UINT_32_MAX as usize).wrapping_div(::core::mem::size_of::<png_bytep>() as usize)
    {
        png_error(
            png_ptr,
            b"Image is too high to process with png_read_png()\0" as *const u8 as png_const_charp,
        );
    }
    if transforms & PNG_TRANSFORM_SCALE_16 != 0 as ::core::ffi::c_int {
        png_set_scale_16(png_ptr);
    }
    if transforms & PNG_TRANSFORM_STRIP_16 != 0 as ::core::ffi::c_int {
        png_set_strip_16(png_ptr);
    }
    if transforms & PNG_TRANSFORM_STRIP_ALPHA != 0 as ::core::ffi::c_int {
        png_set_strip_alpha(png_ptr);
    }
    if transforms & PNG_TRANSFORM_PACKING != 0 as ::core::ffi::c_int {
        png_set_packing(png_ptr);
    }
    if transforms & PNG_TRANSFORM_PACKSWAP != 0 as ::core::ffi::c_int {
        png_set_packswap(png_ptr);
    }
    if transforms & PNG_TRANSFORM_EXPAND != 0 as ::core::ffi::c_int {
        png_set_expand(png_ptr);
    }
    if transforms & PNG_TRANSFORM_INVERT_MONO != 0 as ::core::ffi::c_int {
        png_set_invert_mono(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SHIFT != 0 as ::core::ffi::c_int {
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sBIT != 0 as ::core::ffi::c_uint {
            png_set_shift(png_ptr, &raw mut (*info_ptr).sig_bit as png_const_color_8p);
        }
    }
    if transforms & PNG_TRANSFORM_BGR != 0 as ::core::ffi::c_int {
        png_set_bgr(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SWAP_ALPHA != 0 as ::core::ffi::c_int {
        png_set_swap_alpha(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SWAP_ENDIAN != 0 as ::core::ffi::c_int {
        png_set_swap(png_ptr);
    }
    if transforms & PNG_TRANSFORM_INVERT_ALPHA != 0 as ::core::ffi::c_int {
        png_set_invert_alpha(png_ptr);
    }
    if transforms & PNG_TRANSFORM_GRAY_TO_RGB != 0 as ::core::ffi::c_int {
        png_set_gray_to_rgb(png_ptr);
    }
    if transforms & PNG_TRANSFORM_EXPAND_16 != 0 as ::core::ffi::c_int {
        png_set_expand_16(png_ptr);
    }
    png_set_interlace_handling(png_ptr);
    png_read_update_info(png_ptr, info_ptr);
    png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0 as ::core::ffi::c_int);
    if (*info_ptr).row_pointers.is_null() {
        let mut iptr: png_uint_32 = 0;
        (*info_ptr).row_pointers = png_malloc(
            png_ptr,
            ((*info_ptr).height as png_alloc_size_t)
                .wrapping_mul(::core::mem::size_of::<png_bytep>() as png_alloc_size_t),
        ) as png_bytepp;
        iptr = 0 as png_uint_32;
        while iptr < (*info_ptr).height {
            let ref mut fresh2 = *(*info_ptr).row_pointers.offset(iptr as isize);
            *fresh2 = ::core::ptr::null_mut::<png_byte>();
            iptr = iptr.wrapping_add(1);
        }
        (*info_ptr).free_me |= PNG_FREE_ROWS;
        iptr = 0 as png_uint_32;
        while iptr < (*info_ptr).height {
            let ref mut fresh3 = *(*info_ptr).row_pointers.offset(iptr as isize);
            *fresh3 =
                png_malloc(png_ptr, (*info_ptr).rowbytes as png_alloc_size_t) as *mut png_byte;
            iptr = iptr.wrapping_add(1);
        }
    }
    png_read_image(png_ptr, (*info_ptr).row_pointers);
    (*info_ptr).valid |= PNG_INFO_IDAT;
    png_read_end(png_ptr, info_ptr);
}
pub const P_NOTSET: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const P_sRGB: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const P_LINEAR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const P_FILE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const P_LINEAR8: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_CMAP_NONE: ::core::ffi::c_int = 0;
pub const PNG_CMAP_GA: ::core::ffi::c_int = 1;
pub const PNG_CMAP_TRANS: ::core::ffi::c_int = 2;
pub const PNG_CMAP_RGB: ::core::ffi::c_int = 3;
pub const PNG_CMAP_RGB_ALPHA: ::core::ffi::c_int = 4;
pub const PNG_CMAP_NONE_BACKGROUND: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_CMAP_GA_BACKGROUND: ::core::ffi::c_int = 231 as ::core::ffi::c_int;
pub const PNG_CMAP_TRANS_BACKGROUND: ::core::ffi::c_int = 254 as ::core::ffi::c_int;
pub const PNG_CMAP_RGB_BACKGROUND: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_CMAP_RGB_ALPHA_BACKGROUND: ::core::ffi::c_int = 216 as ::core::ffi::c_int;
unsafe extern "C" fn png_image_read_init(mut image: png_imagep) -> ::core::ffi::c_int {
    if (*image).opaque.is_null() {
        let mut png_ptr: png_structp = png_create_read_struct(
            PNG_LIBPNG_VER_STRING.as_ptr(),
            image as png_voidp,
            ::core::mem::transmute::<
                Option<unsafe extern "C" fn(png_structp, png_const_charp) -> !>,
                png_error_ptr,
            >(Some(
                png_safe_error as unsafe extern "C" fn(png_structp, png_const_charp) -> !,
            )),
            Some(png_safe_warning as unsafe extern "C" fn(png_structp, png_const_charp) -> ()),
        );
        memset(
            image as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<C2RustUnnamed>() as size_t,
        );
        (*image).version = PNG_IMAGE_VERSION as png_uint_32;
        if !png_ptr.is_null() {
            let mut info_ptr: png_infop = png_create_info_struct(png_ptr as png_const_structrp);
            if !info_ptr.is_null() {
                let mut control: png_controlp = png_malloc_warn(
                    png_ptr as png_const_structrp,
                    ::core::mem::size_of::<png_control>() as png_alloc_size_t,
                ) as png_controlp;
                if !control.is_null() {
                    memset(
                        control as *mut ::core::ffi::c_void,
                        0 as ::core::ffi::c_int,
                        ::core::mem::size_of::<png_control>() as size_t,
                    );
                    (*control).png_ptr = png_ptr;
                    (*control).info_ptr = info_ptr;
                    (*control).set_for_write(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
                    (*image).opaque = control;
                    return 1 as ::core::ffi::c_int;
                }
                png_destroy_info_struct(png_ptr as png_const_structrp, &raw mut info_ptr);
            }
            png_destroy_read_struct(
                &raw mut png_ptr,
                ::core::ptr::null_mut::<*mut png_info>(),
                ::core::ptr::null_mut::<*mut png_info>(),
            );
        }
        return png_image_error(
            image,
            b"png_image_read: out of memory\0" as *const u8 as png_const_charp,
        );
    }
    return png_image_error(
        image,
        b"png_image_read: opaque pointer not NULL\0" as *const u8 as png_const_charp,
    );
}
unsafe extern "C" fn png_image_format(mut png_ptr: png_structrp) -> png_uint_32 {
    let mut format: png_uint_32 = 0 as png_uint_32;
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int
    {
        format |= PNG_FORMAT_FLAG_COLOR;
    }
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA != 0 as ::core::ffi::c_int
    {
        format |= PNG_FORMAT_FLAG_ALPHA;
    } else if (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        format |= PNG_FORMAT_FLAG_ALPHA;
    }
    if (*png_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        format |= PNG_FORMAT_FLAG_LINEAR;
    }
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_PALETTE
        != 0 as ::core::ffi::c_int
    {
        format |= PNG_FORMAT_FLAG_COLORMAP;
    }
    return format;
}
unsafe extern "C" fn chromaticities_match_sRGB(mut xy: *const png_xy) -> ::core::ffi::c_int {
    static mut sRGB_xy: png_xy = png_xy {
        redx: 64000 as png_fixed_point,
        redy: 33000 as png_fixed_point,
        greenx: 30000 as png_fixed_point,
        greeny: 60000 as png_fixed_point,
        bluex: 15000 as png_fixed_point,
        bluey: 6000 as png_fixed_point,
        whitex: 31270 as png_fixed_point,
        whitey: 32900 as png_fixed_point,
    };
    if (*xy).whitex < sRGB_xy.whitex as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
        || (*xy).whitex > sRGB_xy.whitex as ::core::ffi::c_int + 1000 as ::core::ffi::c_int
        || ((*xy).whitey < sRGB_xy.whitey as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).whitey > sRGB_xy.whitey as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).redx < sRGB_xy.redx as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).redx > sRGB_xy.redx as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).redy < sRGB_xy.redy as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).redy > sRGB_xy.redy as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).greenx < sRGB_xy.greenx as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).greenx > sRGB_xy.greenx as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).greeny < sRGB_xy.greeny as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).greeny > sRGB_xy.greeny as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).bluex < sRGB_xy.bluex as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).bluex > sRGB_xy.bluex as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
        || ((*xy).bluey < sRGB_xy.bluey as ::core::ffi::c_int - 1000 as ::core::ffi::c_int
            || (*xy).bluey > sRGB_xy.bluey as ::core::ffi::c_int + 1000 as ::core::ffi::c_int)
    {
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_gamma_not_sRGB(mut g: png_fixed_point) -> ::core::ffi::c_int {
    if g < PNG_LIB_GAMMA_MIN || g > PNG_LIB_GAMMA_MAX {
        return 0 as ::core::ffi::c_int;
    }
    return png_gamma_significant(
        (g * 11 as png_fixed_point + 2 as png_fixed_point) / 5 as png_fixed_point,
    );
}
unsafe extern "C" fn png_image_is_not_sRGB(mut png_ptr: png_const_structrp) -> ::core::ffi::c_int {
    if (*png_ptr).chunks as ::core::ffi::c_uint
        & 0x80000000 as ::core::ffi::c_uint
            >> 31 as ::core::ffi::c_int - PNG_INDEX_cICP as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint
        || (*png_ptr).chunks as ::core::ffi::c_uint
            & 0x80000000 as ::core::ffi::c_uint
                >> 31 as ::core::ffi::c_int - PNG_INDEX_mDCV as ::core::ffi::c_int
            != 0 as ::core::ffi::c_uint
    {
        return (chromaticities_match_sRGB(&raw const (*png_ptr).chromaticities) == 0)
            as ::core::ffi::c_int;
    }
    if (*png_ptr).chunks as ::core::ffi::c_uint
        & 0x80000000 as ::core::ffi::c_uint
            >> 31 as ::core::ffi::c_int - PNG_INDEX_sRGB as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint
    {
        return 0 as ::core::ffi::c_int;
    }
    if (*png_ptr).chunks as ::core::ffi::c_uint
        & 0x80000000 as ::core::ffi::c_uint
            >> 31 as ::core::ffi::c_int - PNG_INDEX_cHRM as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint
    {
        return (chromaticities_match_sRGB(&raw const (*png_ptr).chromaticities) == 0)
            as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_header(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut image: png_imagep = argument as png_imagep;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*(*image).opaque).info_ptr as png_inforp;
    png_set_benign_errors(png_ptr, 1 as ::core::ffi::c_int);
    png_read_info(png_ptr, info_ptr);
    (*image).width = (*png_ptr).width;
    (*image).height = (*png_ptr).height;
    let mut format: png_uint_32 = png_image_format(png_ptr);
    (*image).format = format;
    if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 as ::core::ffi::c_uint
        && png_image_is_not_sRGB(png_ptr) != 0
    {
        (*image).flags |= PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB as ::core::ffi::c_uint;
    }
    let mut cmap_entries: png_uint_32 = 0;
    match (*png_ptr).color_type as ::core::ffi::c_int {
        PNG_COLOR_TYPE_GRAY => {
            cmap_entries = ((1 as ::core::ffi::c_uint)
                << (*png_ptr).bit_depth as ::core::ffi::c_int)
                as png_uint_32;
        }
        PNG_COLOR_TYPE_PALETTE => {
            cmap_entries = (*png_ptr).num_palette as png_uint_32;
        }
        _ => {
            cmap_entries = 256 as png_uint_32;
        }
    }
    if cmap_entries > 256 as ::core::ffi::c_uint {
        cmap_entries = 256 as png_uint_32;
    }
    (*image).colormap_entries = cmap_entries;
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_stdio(
    mut image: png_imagep,
    mut file: *mut FILE,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !file.is_null() {
            if png_image_read_init(image) != 0 as ::core::ffi::c_int {
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                return png_safe_execute(
                    image,
                    Some(
                        png_image_read_header
                            as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                    ),
                    image as png_voidp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_stdio: invalid argument\0" as *const u8
                    as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_stdio: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_file(
    mut image: png_imagep,
    mut file_name: *const ::core::ffi::c_char,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !file_name.is_null() {
            let mut fp: *mut FILE = fopen(
                file_name,
                b"rb\0" as *const u8 as *const ::core::ffi::c_char,
            );
            if !fp.is_null() {
                if png_image_read_init(image) != 0 as ::core::ffi::c_int {
                    (*(*(*image).opaque).png_ptr).io_ptr = fp as png_voidp;
                    (*(*image).opaque)
                        .set_owned_file(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
                    return png_safe_execute(
                        image,
                        Some(
                            png_image_read_header
                                as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                        ),
                        image as png_voidp,
                    );
                }
                fclose(fp);
            } else {
                return png_image_error(image, strerror(*__errno_location()) as png_const_charp);
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_file: invalid argument\0" as *const u8
                    as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_file: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_memory_read(
    mut png_ptr: png_structp,
    mut out: png_bytep,
    mut need: size_t,
) {
    if !png_ptr.is_null() {
        let mut image: png_imagep = (*png_ptr).io_ptr as png_imagep;
        if !image.is_null() {
            let mut cp: png_controlp = (*image).opaque;
            if !cp.is_null() {
                let mut memory: png_const_bytep = (*cp).memory;
                let mut size: size_t = (*cp).size;
                if !memory.is_null() && size >= need {
                    memcpy(
                        out as *mut ::core::ffi::c_void,
                        memory as *const ::core::ffi::c_void,
                        need,
                    );
                    (*cp).memory = memory.offset(need as isize);
                    (*cp).size = size.wrapping_sub(need);
                    return;
                }
                png_error(
                    png_ptr as png_const_structrp,
                    b"read beyond end of data\0" as *const u8 as png_const_charp,
                );
            }
        }
        png_error(
            png_ptr as png_const_structrp,
            b"invalid memory read\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_begin_read_from_memory(
    mut image: png_imagep,
    mut memory: png_const_voidp,
    mut size: size_t,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !memory.is_null() && size > 0 as size_t {
            if png_image_read_init(image) != 0 as ::core::ffi::c_int {
                (*(*image).opaque).memory = memory as png_const_bytep;
                (*(*image).opaque).size = size;
                (*(*(*image).opaque).png_ptr).io_ptr = image as png_voidp;
                (*(*(*image).opaque).png_ptr).read_data_fn = Some(
                    png_image_memory_read
                        as unsafe extern "C" fn(png_structp, png_bytep, size_t) -> (),
                ) as png_rw_ptr;
                return png_safe_execute(
                    image,
                    Some(
                        png_image_read_header
                            as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                    ),
                    image as png_voidp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_begin_read_from_memory: invalid argument\0" as *const u8
                    as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_begin_read_from_memory: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_skip_unused_chunks(mut png_ptr: png_structrp) {
    static mut chunks_to_process: [png_byte; 35] = [
        98 as ::core::ffi::c_int as png_byte,
        75 as ::core::ffi::c_int as png_byte,
        71 as ::core::ffi::c_int as png_byte,
        68 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        99 as ::core::ffi::c_int as png_byte,
        72 as ::core::ffi::c_int as png_byte,
        82 as ::core::ffi::c_int as png_byte,
        77 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        99 as ::core::ffi::c_int as png_byte,
        73 as ::core::ffi::c_int as png_byte,
        67 as ::core::ffi::c_int as png_byte,
        80 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        103 as ::core::ffi::c_int as png_byte,
        65 as ::core::ffi::c_int as png_byte,
        77 as ::core::ffi::c_int as png_byte,
        65 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        109 as ::core::ffi::c_int as png_byte,
        68 as ::core::ffi::c_int as png_byte,
        67 as ::core::ffi::c_int as png_byte,
        86 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        115 as ::core::ffi::c_int as png_byte,
        66 as ::core::ffi::c_int as png_byte,
        73 as ::core::ffi::c_int as png_byte,
        84 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
        115 as ::core::ffi::c_int as png_byte,
        82 as ::core::ffi::c_int as png_byte,
        71 as ::core::ffi::c_int as png_byte,
        66 as ::core::ffi::c_int as png_byte,
        '\0' as i32 as png_byte,
    ];
    png_set_keep_unknown_chunks(
        png_ptr,
        PNG_HANDLE_CHUNK_NEVER,
        ::core::ptr::null::<png_byte>(),
        -(1 as ::core::ffi::c_int),
    );
    png_set_keep_unknown_chunks(
        png_ptr,
        PNG_HANDLE_CHUNK_AS_DEFAULT,
        &raw const chunks_to_process as png_const_bytep,
        ::core::mem::size_of::<[png_byte; 35]>() as ::core::ffi::c_int / 5 as ::core::ffi::c_int,
    );
}
unsafe extern "C" fn set_file_encoding(mut display: *mut png_image_read_control) {
    let mut png_ptr: png_structrp = (*(*(*display).image).opaque).png_ptr as png_structrp;
    let mut g: png_fixed_point = png_resolve_file_gamma(png_ptr);
    if g == 0 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"internal: default gamma not set\0" as *const u8 as png_const_charp,
        );
    }
    if png_gamma_significant(g) != 0 as ::core::ffi::c_int {
        if png_gamma_not_sRGB(g) != 0 as ::core::ffi::c_int {
            (*display).file_encoding = P_FILE;
            (*display).gamma_to_linear = png_reciprocal(g);
        } else {
            (*display).file_encoding = P_sRGB;
        }
    } else {
        (*display).file_encoding = P_LINEAR8;
    };
}
unsafe extern "C" fn decode_gamma(
    mut display: *mut png_image_read_control,
    mut value: png_uint_32,
    mut encoding: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    if encoding == P_FILE {
        encoding = (*display).file_encoding;
    }
    if encoding == P_NOTSET {
        set_file_encoding(display);
        encoding = (*display).file_encoding;
    }
    match encoding {
        P_FILE => {
            value = png_gamma_16bit_correct(
                (value as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint),
                (*display).gamma_to_linear,
            ) as png_uint_32;
        }
        P_sRGB => {
            value = png_sRGB_table[value as usize] as png_uint_32;
        }
        P_LINEAR => {}
        P_LINEAR8 => {
            value = (value as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint)
                as png_uint_32 as png_uint_32;
        }
        _ => {
            png_error(
                (*(*(*display).image).opaque).png_ptr as png_const_structrp,
                b"unexpected encoding (internal error)\0" as *const u8 as png_const_charp,
            );
        }
    }
    return value as ::core::ffi::c_uint;
}
unsafe extern "C" fn png_colormap_compose(
    mut display: *mut png_image_read_control,
    mut foreground: png_uint_32,
    mut foreground_encoding: ::core::ffi::c_int,
    mut alpha: png_uint_32,
    mut background: png_uint_32,
    mut encoding: ::core::ffi::c_int,
) -> png_uint_32 {
    let mut f: png_uint_32 = decode_gamma(display, foreground, foreground_encoding) as png_uint_32;
    let mut b: png_uint_32 = decode_gamma(display, background, encoding) as png_uint_32;
    f = f
        .wrapping_mul(alpha)
        .wrapping_add(b.wrapping_mul((255 as png_uint_32).wrapping_sub(alpha)));
    if encoding == P_LINEAR {
        f = (f as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint) as png_uint_32
            as png_uint_32;
        f = (f as ::core::ffi::c_uint)
            .wrapping_add((f >> 16 as ::core::ffi::c_int) as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        f = ((f as ::core::ffi::c_uint).wrapping_add(32768 as ::core::ffi::c_uint)
            >> 16 as ::core::ffi::c_int) as png_uint_32;
    } else {
        f = (0xff as ::core::ffi::c_uint
            & (png_sRGB_base[(f >> 15 as ::core::ffi::c_int) as usize] as ::core::ffi::c_uint)
                .wrapping_add(
                    (f as ::core::ffi::c_uint & 0x7fff as ::core::ffi::c_uint).wrapping_mul(
                        png_sRGB_delta[(f >> 15 as ::core::ffi::c_int) as usize]
                            as ::core::ffi::c_uint,
                    ) >> 12 as ::core::ffi::c_int,
                )
                >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
    }
    return f;
}
unsafe extern "C" fn png_create_colormap_entry(
    mut display: *mut png_image_read_control,
    mut ip: png_uint_32,
    mut red: png_uint_32,
    mut green: png_uint_32,
    mut blue: png_uint_32,
    mut alpha: png_uint_32,
    mut encoding: ::core::ffi::c_int,
) {
    let mut image: png_imagep = (*display).image;
    let mut output_encoding: ::core::ffi::c_int = if (*image).format as ::core::ffi::c_uint
        & PNG_FORMAT_FLAG_LINEAR
        != 0 as ::core::ffi::c_uint
    {
        P_LINEAR
    } else {
        P_sRGB
    };
    let mut convert_to_Y: ::core::ffi::c_int =
        ((*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR == 0 as ::core::ffi::c_uint
            && (red != green || green != blue)) as ::core::ffi::c_int;
    if ip > 255 as ::core::ffi::c_uint {
        png_error(
            (*(*image).opaque).png_ptr as png_const_structrp,
            b"color-map index out of range\0" as *const u8 as png_const_charp,
        );
    }
    if encoding == P_FILE {
        if (*display).file_encoding == P_NOTSET {
            set_file_encoding(display);
        }
        encoding = (*display).file_encoding;
    }
    if encoding == P_FILE {
        let mut g: png_fixed_point = (*display).gamma_to_linear;
        red = png_gamma_16bit_correct(
            (red as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint),
            g,
        ) as png_uint_32;
        green = png_gamma_16bit_correct(
            (green as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint),
            g,
        ) as png_uint_32;
        blue = png_gamma_16bit_correct(
            (blue as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint),
            g,
        ) as png_uint_32;
        if convert_to_Y != 0 as ::core::ffi::c_int || output_encoding == P_LINEAR {
            alpha = (alpha as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint)
                as png_uint_32 as png_uint_32;
            encoding = P_LINEAR;
        } else {
            red = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((red as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((red as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((red as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            green = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((green as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((green as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((green as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            blue = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((blue as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((blue as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((blue as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            encoding = P_sRGB;
        }
    } else if encoding == P_LINEAR8 {
        red = (red as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint) as png_uint_32
            as png_uint_32;
        green = (green as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        blue = (blue as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint) as png_uint_32
            as png_uint_32;
        alpha = (alpha as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        encoding = P_LINEAR;
    } else if encoding == P_sRGB
        && (convert_to_Y != 0 as ::core::ffi::c_int || output_encoding == P_LINEAR)
    {
        red = png_sRGB_table[red as usize] as png_uint_32;
        green = png_sRGB_table[green as usize] as png_uint_32;
        blue = png_sRGB_table[blue as usize] as png_uint_32;
        alpha = (alpha as ::core::ffi::c_uint).wrapping_mul(257 as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        encoding = P_LINEAR;
    }
    if encoding == P_LINEAR {
        if convert_to_Y != 0 as ::core::ffi::c_int {
            let mut y: png_uint_32 = (6968 as ::core::ffi::c_int as png_uint_32)
                .wrapping_mul(red)
                .wrapping_add((23434 as ::core::ffi::c_int as png_uint_32).wrapping_mul(green))
                .wrapping_add((2366 as ::core::ffi::c_int as png_uint_32).wrapping_mul(blue));
            if output_encoding == P_LINEAR {
                y = ((y as ::core::ffi::c_uint).wrapping_add(16384 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as png_uint_32;
            } else {
                y = ((y as ::core::ffi::c_uint).wrapping_add(128 as ::core::ffi::c_uint)
                    >> 8 as ::core::ffi::c_int) as png_uint_32;
                y = (y as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                    as png_uint_32 as png_uint_32;
                y = (0xff as ::core::ffi::c_uint
                    & (png_sRGB_base[((y as ::core::ffi::c_uint)
                        .wrapping_add(64 as ::core::ffi::c_uint)
                        >> 7 as ::core::ffi::c_int
                        >> 15 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint)
                        .wrapping_add(
                            ((y as ::core::ffi::c_uint).wrapping_add(64 as ::core::ffi::c_uint)
                                >> 7 as ::core::ffi::c_int
                                & 0x7fff as ::core::ffi::c_uint)
                                .wrapping_mul(
                                    png_sRGB_delta[((y as ::core::ffi::c_uint)
                                        .wrapping_add(64 as ::core::ffi::c_uint)
                                        >> 7 as ::core::ffi::c_int
                                        >> 15 as ::core::ffi::c_int)
                                        as usize]
                                        as ::core::ffi::c_uint,
                                )
                                >> 12 as ::core::ffi::c_int,
                        )
                        >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
                alpha = (alpha
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    .wrapping_add(32895 as ::core::ffi::c_uint)
                    >> 16 as ::core::ffi::c_int) as png_uint_32;
                encoding = P_sRGB;
            }
            green = y;
            red = green;
            blue = red;
        } else if output_encoding == P_sRGB {
            red = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((red as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((red as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((red as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            green = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((green as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((green as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((green as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            blue = (0xff as ::core::ffi::c_uint
                & (png_sRGB_base[((blue as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as usize]
                    as ::core::ffi::c_uint)
                    .wrapping_add(
                        ((blue as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                            & 0x7fff as ::core::ffi::c_uint)
                            .wrapping_mul(
                                png_sRGB_delta[((blue as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint,
                            )
                            >> 12 as ::core::ffi::c_int,
                    )
                    >> 8 as ::core::ffi::c_int) as png_byte as png_uint_32;
            alpha = (alpha
                .wrapping_mul(255 as ::core::ffi::c_uint)
                .wrapping_add(32895 as ::core::ffi::c_uint)
                >> 16 as ::core::ffi::c_int) as png_uint_32;
            encoding = P_sRGB;
        }
    }
    if encoding != output_encoding {
        png_error(
            (*(*image).opaque).png_ptr as png_const_structrp,
            b"bad encoding (internal error)\0" as *const u8 as png_const_charp,
        );
    }
    let mut afirst: ::core::ffi::c_int =
        ((*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST
            != 0 as ::core::ffi::c_uint
            && (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    let mut bgr: ::core::ffi::c_int =
        if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_BGR != 0 as ::core::ffi::c_uint
        {
            2 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        };
    if output_encoding == P_LINEAR {
        let mut entry: png_uint_16p = (*display).colormap as png_uint_16p;
        entry = entry.offset(
            (ip as ::core::ffi::c_uint).wrapping_mul(
                ((*image).format as ::core::ffi::c_uint
                    & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
                    .wrapping_add(1 as ::core::ffi::c_uint),
            ) as isize,
        );
        let mut current_block_76: u64;
        match ((*image).format as ::core::ffi::c_uint
            & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
            .wrapping_add(1 as ::core::ffi::c_uint)
        {
            4 => {
                *entry.offset(
                    (if afirst != 0 {
                        0 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }) as isize,
                ) = alpha as png_uint_16;
                current_block_76 = 8579630259310548259;
            }
            3 => {
                current_block_76 = 8579630259310548259;
            }
            2 => {
                *entry.offset((1 as ::core::ffi::c_int ^ afirst) as isize) = alpha as png_uint_16;
                current_block_76 = 16815402613598748580;
            }
            1 => {
                current_block_76 = 16815402613598748580;
            }
            _ => {
                current_block_76 = 5181772461570869434;
            }
        }
        match current_block_76 {
            16815402613598748580 => {
                if alpha < 65535 as ::core::ffi::c_uint {
                    if alpha > 0 as ::core::ffi::c_uint {
                        green = (green as ::core::ffi::c_uint)
                            .wrapping_mul(alpha as ::core::ffi::c_uint)
                            .wrapping_add(32767 as ::core::ffi::c_uint)
                            .wrapping_div(65535 as ::core::ffi::c_uint)
                            as png_uint_32;
                    } else {
                        green = 0 as png_uint_32;
                    }
                }
                *entry.offset(afirst as isize) = green as png_uint_16;
            }
            8579630259310548259 => {
                if alpha < 65535 as ::core::ffi::c_uint {
                    if alpha > 0 as ::core::ffi::c_uint {
                        blue = (blue as ::core::ffi::c_uint)
                            .wrapping_mul(alpha as ::core::ffi::c_uint)
                            .wrapping_add(32767 as ::core::ffi::c_uint)
                            .wrapping_div(65535 as ::core::ffi::c_uint)
                            as png_uint_32;
                        green = (green as ::core::ffi::c_uint)
                            .wrapping_mul(alpha as ::core::ffi::c_uint)
                            .wrapping_add(32767 as ::core::ffi::c_uint)
                            .wrapping_div(65535 as ::core::ffi::c_uint)
                            as png_uint_32;
                        red = (red as ::core::ffi::c_uint)
                            .wrapping_mul(alpha as ::core::ffi::c_uint)
                            .wrapping_add(32767 as ::core::ffi::c_uint)
                            .wrapping_div(65535 as ::core::ffi::c_uint)
                            as png_uint_32;
                    } else {
                        blue = 0 as png_uint_32;
                        green = blue;
                        red = green;
                    }
                }
                *entry.offset((afirst + (2 as ::core::ffi::c_int ^ bgr)) as isize) =
                    blue as png_uint_16;
                *entry.offset((afirst + 1 as ::core::ffi::c_int) as isize) = green as png_uint_16;
                *entry.offset((afirst + bgr) as isize) = red as png_uint_16;
            }
            _ => {}
        }
    } else {
        let mut entry_0: png_bytep = (*display).colormap as png_bytep;
        entry_0 = entry_0.offset(
            (ip as ::core::ffi::c_uint).wrapping_mul(
                ((*image).format as ::core::ffi::c_uint
                    & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
                    .wrapping_add(1 as ::core::ffi::c_uint),
            ) as isize,
        );
        let mut current_block_85: u64;
        match ((*image).format as ::core::ffi::c_uint
            & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
            .wrapping_add(1 as ::core::ffi::c_uint)
        {
            4 => {
                *entry_0.offset(
                    (if afirst != 0 {
                        0 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }) as isize,
                ) = alpha as png_byte;
                current_block_85 = 6250133072218299440;
            }
            3 => {
                current_block_85 = 6250133072218299440;
            }
            2 => {
                *entry_0.offset((1 as ::core::ffi::c_int ^ afirst) as isize) = alpha as png_byte;
                current_block_85 = 10549295154225939871;
            }
            1 => {
                current_block_85 = 10549295154225939871;
            }
            _ => {
                current_block_85 = 3229571381435211107;
            }
        }
        match current_block_85 {
            6250133072218299440 => {
                *entry_0.offset((afirst + (2 as ::core::ffi::c_int ^ bgr)) as isize) =
                    blue as png_byte;
                *entry_0.offset((afirst + 1 as ::core::ffi::c_int) as isize) = green as png_byte;
                *entry_0.offset((afirst + bgr) as isize) = red as png_byte;
            }
            10549295154225939871 => {
                *entry_0.offset(afirst as isize) = green as png_byte;
            }
            _ => {}
        }
    };
}
unsafe extern "C" fn make_gray_file_colormap(
    mut display: *mut png_image_read_control,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < 256 as ::core::ffi::c_uint {
        png_create_colormap_entry(
            display,
            i as png_uint_32,
            i as png_uint_32,
            i as png_uint_32,
            i as png_uint_32,
            255 as png_uint_32,
            P_FILE,
        );
        i = i.wrapping_add(1);
    }
    return i as ::core::ffi::c_int;
}
unsafe extern "C" fn make_gray_colormap(
    mut display: *mut png_image_read_control,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < 256 as ::core::ffi::c_uint {
        png_create_colormap_entry(
            display,
            i as png_uint_32,
            i as png_uint_32,
            i as png_uint_32,
            i as png_uint_32,
            255 as png_uint_32,
            P_sRGB,
        );
        i = i.wrapping_add(1);
    }
    return i as ::core::ffi::c_int;
}
pub const PNG_GRAY_COLORMAP_ENTRIES: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
unsafe extern "C" fn make_ga_colormap(
    mut display: *mut png_image_read_control,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_uint = 0;
    let mut a: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < 231 as ::core::ffi::c_uint {
        let mut gray: ::core::ffi::c_uint = i
            .wrapping_mul(256 as ::core::ffi::c_uint)
            .wrapping_add(115 as ::core::ffi::c_uint)
            .wrapping_div(231 as ::core::ffi::c_uint);
        let fresh16 = i;
        i = i.wrapping_add(1);
        png_create_colormap_entry(
            display,
            fresh16,
            gray as png_uint_32,
            gray as png_uint_32,
            gray as png_uint_32,
            255 as png_uint_32,
            P_sRGB,
        );
    }
    let fresh17 = i;
    i = i.wrapping_add(1);
    png_create_colormap_entry(
        display,
        fresh17,
        255 as png_uint_32,
        255 as png_uint_32,
        255 as png_uint_32,
        0 as png_uint_32,
        P_sRGB,
    );
    a = 1 as ::core::ffi::c_uint;
    while a < 5 as ::core::ffi::c_uint {
        let mut g: ::core::ffi::c_uint = 0;
        g = 0 as ::core::ffi::c_uint;
        while g < 6 as ::core::ffi::c_uint {
            let fresh18 = i;
            i = i.wrapping_add(1);
            png_create_colormap_entry(
                display,
                fresh18,
                (g as png_uint_32).wrapping_mul(51 as png_uint_32),
                (g as png_uint_32).wrapping_mul(51 as png_uint_32),
                (g as png_uint_32).wrapping_mul(51 as png_uint_32),
                (a as png_uint_32).wrapping_mul(51 as png_uint_32),
                P_sRGB,
            );
            g = g.wrapping_add(1);
        }
        a = a.wrapping_add(1);
    }
    return i as ::core::ffi::c_int;
}
pub const PNG_GA_COLORMAP_ENTRIES: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
unsafe extern "C" fn make_rgb_colormap(
    mut display: *mut png_image_read_control,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_uint = 0;
    let mut r: ::core::ffi::c_uint = 0;
    r = 0 as ::core::ffi::c_uint;
    i = r;
    while r < 6 as ::core::ffi::c_uint {
        let mut g: ::core::ffi::c_uint = 0;
        g = 0 as ::core::ffi::c_uint;
        while g < 6 as ::core::ffi::c_uint {
            let mut b: ::core::ffi::c_uint = 0;
            b = 0 as ::core::ffi::c_uint;
            while b < 6 as ::core::ffi::c_uint {
                let fresh15 = i;
                i = i.wrapping_add(1);
                png_create_colormap_entry(
                    display,
                    fresh15,
                    (r as png_uint_32).wrapping_mul(51 as png_uint_32),
                    (g as png_uint_32).wrapping_mul(51 as png_uint_32),
                    (b as png_uint_32).wrapping_mul(51 as png_uint_32),
                    255 as png_uint_32,
                    P_sRGB,
                );
                b = b.wrapping_add(1);
            }
            g = g.wrapping_add(1);
        }
        r = r.wrapping_add(1);
    }
    return i as ::core::ffi::c_int;
}
pub const PNG_RGB_COLORMAP_ENTRIES: ::core::ffi::c_int = 216 as ::core::ffi::c_int;
unsafe extern "C" fn png_image_read_colormap(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut output_format: png_uint_32 = (*image).format;
    let mut output_encoding: ::core::ffi::c_int = if output_format as ::core::ffi::c_uint
        & PNG_FORMAT_FLAG_LINEAR
        != 0 as ::core::ffi::c_uint
    {
        P_LINEAR
    } else {
        P_sRGB
    };
    let mut cmap_entries: ::core::ffi::c_uint = 0;
    let mut output_processing: ::core::ffi::c_uint = 0;
    let mut data_encoding: ::core::ffi::c_uint = P_NOTSET as ::core::ffi::c_uint;
    let mut background_index: ::core::ffi::c_uint = 256 as ::core::ffi::c_uint;
    let mut back_r: png_uint_32 = 0;
    let mut back_g: png_uint_32 = 0;
    let mut back_b: png_uint_32 = 0;
    let mut expand_tRNS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if ((*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
        != 0 as ::core::ffi::c_int
        || (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int)
        && output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA == 0 as ::core::ffi::c_uint
    {
        if output_encoding == P_LINEAR {
            back_r = 0 as png_uint_32;
            back_g = back_r;
            back_b = back_g;
        } else if (*display).background.is_null() {
            png_error(
                png_ptr,
                b"background color must be supplied to remove alpha/transparency\0" as *const u8
                    as png_const_charp,
            );
        } else {
            back_g = (*(*display).background).green as png_uint_32;
            if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR
                != 0 as ::core::ffi::c_uint
            {
                back_r = (*(*display).background).red as png_uint_32;
                back_b = (*(*display).background).blue as png_uint_32;
            } else {
                back_r = back_g;
                back_b = back_r;
            }
        }
    } else if output_encoding == P_LINEAR {
        back_g = 65535 as png_uint_32;
        back_r = back_g;
        back_b = back_r;
    } else {
        back_g = 255 as png_uint_32;
        back_r = back_g;
        back_b = back_r;
    }
    if (*png_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int
        && (*image).flags as ::core::ffi::c_uint & PNG_IMAGE_FLAG_16BIT_sRGB as ::core::ffi::c_uint
            == 0 as ::core::ffi::c_uint
    {
        (*png_ptr).default_gamma = PNG_GAMMA_LINEAR as png_fixed_point;
    } else {
        (*png_ptr).default_gamma = PNG_GAMMA_sRGB_INVERSE as png_fixed_point;
    }
    let mut current_block_224: u64;
    match (*png_ptr).color_type as ::core::ffi::c_int {
        PNG_COLOR_TYPE_GRAY => {
            if (*png_ptr).bit_depth as ::core::ffi::c_int <= 8 as ::core::ffi::c_int {
                let mut step: ::core::ffi::c_uint = 0;
                let mut i: ::core::ffi::c_uint = 0;
                let mut val: ::core::ffi::c_uint = 0;
                let mut trans: ::core::ffi::c_uint = 256 as ::core::ffi::c_uint;
                let mut back_alpha: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
                cmap_entries =
                    (1 as ::core::ffi::c_uint) << (*png_ptr).bit_depth as ::core::ffi::c_int;
                if cmap_entries > (*image).colormap_entries {
                    png_error(
                        png_ptr,
                        b"gray[8] color-map: too few entries\0" as *const u8 as png_const_charp,
                    );
                }
                step = (255 as ::core::ffi::c_uint)
                    .wrapping_div(cmap_entries.wrapping_sub(1 as ::core::ffi::c_uint));
                output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
                if (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                    trans = (*png_ptr).trans_color.gray as ::core::ffi::c_uint;
                    if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                        == 0 as ::core::ffi::c_uint
                    {
                        back_alpha = (if output_encoding == P_LINEAR {
                            65535 as ::core::ffi::c_int
                        } else {
                            255 as ::core::ffi::c_int
                        }) as ::core::ffi::c_uint;
                    }
                }
                val = 0 as ::core::ffi::c_uint;
                i = val;
                while i < cmap_entries {
                    if i != trans {
                        png_create_colormap_entry(
                            display,
                            i as png_uint_32,
                            val as png_uint_32,
                            val as png_uint_32,
                            val as png_uint_32,
                            255 as png_uint_32,
                            P_FILE,
                        );
                    } else {
                        png_create_colormap_entry(
                            display,
                            i as png_uint_32,
                            back_r,
                            back_g,
                            back_b,
                            back_alpha as png_uint_32,
                            output_encoding,
                        );
                    }
                    i = i.wrapping_add(1);
                    val = val.wrapping_add(step);
                }
                data_encoding = P_FILE as ::core::ffi::c_uint;
                if ((*png_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
                    png_set_packing(png_ptr);
                }
            } else {
                data_encoding = P_sRGB as ::core::ffi::c_uint;
                if PNG_GRAY_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                    png_error(
                        png_ptr,
                        b"gray[16] color-map: too few entries\0" as *const u8 as png_const_charp,
                    );
                }
                cmap_entries = make_gray_colormap(display) as ::core::ffi::c_uint;
                if (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                    let mut back_alpha_0: ::core::ffi::c_uint = 0;
                    if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                        != 0 as ::core::ffi::c_uint
                    {
                        back_alpha_0 = 0 as ::core::ffi::c_uint;
                        current_block_224 = 11777552016271000781;
                    } else if back_r == back_g && back_g == back_b {
                        let mut c: png_color_16 = png_color_16 {
                            index: 0,
                            red: 0,
                            green: 0,
                            blue: 0,
                            gray: 0,
                        };
                        let mut gray: png_uint_32 = back_g;
                        if output_encoding == P_LINEAR {
                            gray = (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[((gray as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((gray as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[((gray as ::core::ffi::c_uint)
                                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32;
                            png_create_colormap_entry(
                                display,
                                gray,
                                back_g,
                                back_g,
                                back_g,
                                65535 as png_uint_32,
                                P_LINEAR,
                            );
                        }
                        c.index = 0 as png_byte;
                        c.blue = gray as png_uint_16;
                        c.green = c.blue;
                        c.red = c.green;
                        c.gray = c.red;
                        png_set_background_fixed(
                            png_ptr,
                            &raw mut c as png_const_color_16p,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0 as ::core::ffi::c_int,
                            0 as png_fixed_point,
                        );
                        output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
                        current_block_224 = 3166194604430448652;
                    } else {
                        back_alpha_0 = (if output_encoding == P_LINEAR {
                            65535 as ::core::ffi::c_int
                        } else {
                            255 as ::core::ffi::c_int
                        }) as ::core::ffi::c_uint;
                        current_block_224 = 11777552016271000781;
                    }
                    match current_block_224 {
                        3166194604430448652 => {}
                        _ => {
                            expand_tRNS = 1 as ::core::ffi::c_int;
                            output_processing = PNG_CMAP_TRANS as ::core::ffi::c_uint;
                            background_index = 254 as ::core::ffi::c_uint;
                            png_create_colormap_entry(
                                display,
                                254 as png_uint_32,
                                back_r,
                                back_g,
                                back_b,
                                back_alpha_0 as png_uint_32,
                                output_encoding,
                            );
                        }
                    }
                } else {
                    output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
                }
            }
        }
        PNG_COLOR_TYPE_GRAY_ALPHA => {
            data_encoding = P_sRGB as ::core::ffi::c_uint;
            if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                != 0 as ::core::ffi::c_uint
            {
                if PNG_GA_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                    png_error(
                        png_ptr,
                        b"gray+alpha color-map: too few entries\0" as *const u8 as png_const_charp,
                    );
                }
                cmap_entries = make_ga_colormap(display) as ::core::ffi::c_uint;
                background_index = PNG_CMAP_GA_BACKGROUND as ::core::ffi::c_uint;
                output_processing = PNG_CMAP_GA as ::core::ffi::c_uint;
            } else if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR
                == 0 as ::core::ffi::c_uint
                || back_r == back_g && back_g == back_b
            {
                let mut c_0: png_color_16 = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                };
                let mut gray_0: png_uint_32 = back_g;
                if PNG_GRAY_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                    png_error(
                        png_ptr,
                        b"gray-alpha color-map: too few entries\0" as *const u8 as png_const_charp,
                    );
                }
                cmap_entries = make_gray_colormap(display) as ::core::ffi::c_uint;
                if output_encoding == P_LINEAR {
                    gray_0 = (0xff as ::core::ffi::c_uint
                        & (png_sRGB_base[((gray_0 as ::core::ffi::c_uint)
                            .wrapping_mul(255 as ::core::ffi::c_uint)
                            >> 15 as ::core::ffi::c_int)
                            as usize] as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((gray_0 as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    & 0x7fff as ::core::ffi::c_uint)
                                    .wrapping_mul(
                                        png_sRGB_delta[((gray_0 as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            >> 15 as ::core::ffi::c_int)
                                            as usize]
                                            as ::core::ffi::c_uint,
                                    )
                                    >> 12 as ::core::ffi::c_int,
                            )
                            >> 8 as ::core::ffi::c_int) as png_byte
                        as png_uint_32;
                    png_create_colormap_entry(
                        display,
                        gray_0,
                        back_g,
                        back_g,
                        back_g,
                        65535 as png_uint_32,
                        P_LINEAR,
                    );
                }
                c_0.index = 0 as png_byte;
                c_0.blue = gray_0 as png_uint_16;
                c_0.green = c_0.blue;
                c_0.red = c_0.green;
                c_0.gray = c_0.red;
                png_set_background_fixed(
                    png_ptr,
                    &raw mut c_0 as png_const_color_16p,
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    0 as ::core::ffi::c_int,
                    0 as png_fixed_point,
                );
                output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
            } else {
                let mut i_0: png_uint_32 = 0;
                let mut a: png_uint_32 = 0;
                if PNG_GA_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                    png_error(
                        png_ptr,
                        b"ga-alpha color-map: too few entries\0" as *const u8 as png_const_charp,
                    );
                }
                i_0 = 0 as png_uint_32;
                while i_0 < 231 as ::core::ffi::c_uint {
                    let mut gray_1: png_uint_32 = i_0
                        .wrapping_mul(256 as png_uint_32)
                        .wrapping_add(115 as png_uint_32)
                        .wrapping_div(231 as png_uint_32);
                    let fresh8 = i_0;
                    i_0 = i_0.wrapping_add(1);
                    png_create_colormap_entry(
                        display,
                        fresh8,
                        gray_1,
                        gray_1,
                        gray_1,
                        255 as png_uint_32,
                        P_sRGB,
                    );
                }
                background_index = i_0 as ::core::ffi::c_uint;
                let fresh9 = i_0;
                i_0 = i_0.wrapping_add(1);
                png_create_colormap_entry(
                    display,
                    fresh9,
                    back_r,
                    back_g,
                    back_b,
                    if output_encoding == P_LINEAR {
                        65535 as png_uint_32
                    } else {
                        255 as png_uint_32
                    },
                    output_encoding,
                );
                if output_encoding == P_sRGB {
                    back_r = png_sRGB_table[back_r as usize] as png_uint_32;
                    back_g = png_sRGB_table[back_g as usize] as png_uint_32;
                    back_b = png_sRGB_table[back_b as usize] as png_uint_32;
                }
                a = 1 as png_uint_32;
                while a < 5 as ::core::ffi::c_uint {
                    let mut g: ::core::ffi::c_uint = 0;
                    let mut alpha: png_uint_32 = (51 as png_uint_32).wrapping_mul(a);
                    let mut back_rx: png_uint_32 = (255 as png_uint_32)
                        .wrapping_sub(alpha)
                        .wrapping_mul(back_r);
                    let mut back_gx: png_uint_32 = (255 as png_uint_32)
                        .wrapping_sub(alpha)
                        .wrapping_mul(back_g);
                    let mut back_bx: png_uint_32 = (255 as png_uint_32)
                        .wrapping_sub(alpha)
                        .wrapping_mul(back_b);
                    g = 0 as ::core::ffi::c_uint;
                    while g < 6 as ::core::ffi::c_uint {
                        let mut gray_2: png_uint_32 = (png_sRGB_table
                            [g.wrapping_mul(51 as ::core::ffi::c_uint) as usize]
                            as png_uint_32)
                            .wrapping_mul(alpha);
                        let fresh10 = i_0;
                        i_0 = i_0.wrapping_add(1);
                        png_create_colormap_entry(
                            display,
                            fresh10,
                            (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[(gray_2.wrapping_add(back_rx)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((gray_2 as ::core::ffi::c_uint)
                                            .wrapping_add(back_rx as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[(gray_2.wrapping_add(back_rx)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32,
                            (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[(gray_2.wrapping_add(back_gx)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((gray_2 as ::core::ffi::c_uint)
                                            .wrapping_add(back_gx as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[(gray_2.wrapping_add(back_gx)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32,
                            (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[(gray_2.wrapping_add(back_bx)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((gray_2 as ::core::ffi::c_uint)
                                            .wrapping_add(back_bx as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[(gray_2.wrapping_add(back_bx)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32,
                            255 as png_uint_32,
                            P_sRGB,
                        );
                        g = g.wrapping_add(1);
                    }
                    a = a.wrapping_add(1);
                }
                cmap_entries = i_0 as ::core::ffi::c_uint;
                output_processing = PNG_CMAP_GA as ::core::ffi::c_uint;
            }
        }
        PNG_COLOR_TYPE_RGB | PNG_COLOR_TYPE_RGB_ALPHA => {
            if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR
                == 0 as ::core::ffi::c_uint
            {
                png_set_rgb_to_gray_fixed(
                    png_ptr,
                    PNG_ERROR_ACTION_NONE,
                    -(1 as png_fixed_point),
                    -(1 as png_fixed_point),
                );
                data_encoding = P_sRGB as ::core::ffi::c_uint;
                if ((*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
                    || (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int)
                    && output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                        != 0 as ::core::ffi::c_uint
                {
                    expand_tRNS = 1 as ::core::ffi::c_int;
                    if PNG_GA_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                        png_error(
                            png_ptr,
                            b"rgb[ga] color-map: too few entries\0" as *const u8 as png_const_charp,
                        );
                    }
                    cmap_entries = make_ga_colormap(display) as ::core::ffi::c_uint;
                    background_index = PNG_CMAP_GA_BACKGROUND as ::core::ffi::c_uint;
                    output_processing = PNG_CMAP_GA as ::core::ffi::c_uint;
                } else {
                    let gamma: png_fixed_point = png_resolve_file_gamma(png_ptr) as png_fixed_point;
                    if PNG_GRAY_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries
                    {
                        png_error(
                            png_ptr,
                            b"rgb[gray] color-map: too few entries\0" as *const u8
                                as png_const_charp,
                        );
                    }
                    if ((*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int)
                        && png_gamma_not_sRGB(gamma) != 0 as ::core::ffi::c_int
                    {
                        cmap_entries = make_gray_file_colormap(display) as ::core::ffi::c_uint;
                        data_encoding = P_FILE as ::core::ffi::c_uint;
                    } else {
                        cmap_entries = make_gray_colormap(display) as ::core::ffi::c_uint;
                    }
                    if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
                        || (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                    {
                        let mut c_1: png_color_16 = png_color_16 {
                            index: 0,
                            red: 0,
                            green: 0,
                            blue: 0,
                            gray: 0,
                        };
                        let mut gray_3: png_uint_32 = back_g;
                        if data_encoding == P_FILE as ::core::ffi::c_uint {
                            if output_encoding == P_sRGB {
                                gray_3 = png_sRGB_table[gray_3 as usize] as png_uint_32;
                            }
                            gray_3 = ((png_gamma_16bit_correct(gray_3 as ::core::ffi::c_uint, gamma)
                                as ::core::ffi::c_uint)
                                .wrapping_mul(255 as ::core::ffi::c_uint)
                                .wrapping_add(32895 as ::core::ffi::c_uint)
                                >> 16 as ::core::ffi::c_int)
                                as png_uint_32;
                            png_create_colormap_entry(
                                display,
                                gray_3,
                                back_g,
                                back_g,
                                back_g,
                                0 as png_uint_32,
                                output_encoding,
                            );
                        } else if output_encoding == P_LINEAR {
                            gray_3 = (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[((gray_3 as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((gray_3 as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[((gray_3 as ::core::ffi::c_uint)
                                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32;
                            png_create_colormap_entry(
                                display,
                                gray_3,
                                back_g,
                                back_g,
                                back_g,
                                0 as png_uint_32,
                                P_LINEAR,
                            );
                        }
                        c_1.index = 0 as png_byte;
                        c_1.blue = gray_3 as png_uint_16;
                        c_1.green = c_1.blue;
                        c_1.red = c_1.green;
                        c_1.gray = c_1.red;
                        expand_tRNS = 1 as ::core::ffi::c_int;
                        png_set_background_fixed(
                            png_ptr,
                            &raw mut c_1 as png_const_color_16p,
                            PNG_BACKGROUND_GAMMA_SCREEN,
                            0 as ::core::ffi::c_int,
                            0 as png_fixed_point,
                        );
                    }
                    output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
                }
            } else {
                data_encoding = P_sRGB as ::core::ffi::c_uint;
                if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
                    || (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                {
                    if output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                        != 0 as ::core::ffi::c_uint
                    {
                        let mut r: png_uint_32 = 0;
                        if (PNG_RGB_COLORMAP_ENTRIES
                            + 1 as ::core::ffi::c_int
                            + 27 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                            > (*image).colormap_entries
                        {
                            png_error(
                                png_ptr,
                                b"rgb+alpha color-map: too few entries\0" as *const u8
                                    as png_const_charp,
                            );
                        }
                        cmap_entries = make_rgb_colormap(display) as ::core::ffi::c_uint;
                        png_create_colormap_entry(
                            display,
                            cmap_entries as png_uint_32,
                            255 as png_uint_32,
                            255 as png_uint_32,
                            255 as png_uint_32,
                            0 as png_uint_32,
                            P_sRGB,
                        );
                        let fresh11 = cmap_entries;
                        cmap_entries = cmap_entries.wrapping_add(1);
                        background_index = fresh11;
                        r = 0 as png_uint_32;
                        while r < 256 as ::core::ffi::c_uint {
                            let mut g_0: png_uint_32 = 0;
                            g_0 = 0 as png_uint_32;
                            while g_0 < 256 as ::core::ffi::c_uint {
                                let mut b: png_uint_32 = 0;
                                b = 0 as png_uint_32;
                                while b < 256 as ::core::ffi::c_uint {
                                    let fresh12 = cmap_entries;
                                    cmap_entries = cmap_entries.wrapping_add(1);
                                    png_create_colormap_entry(
                                        display,
                                        fresh12,
                                        r,
                                        g_0,
                                        b,
                                        128 as png_uint_32,
                                        P_sRGB,
                                    );
                                    b = ((b as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int
                                        | 0x7f as ::core::ffi::c_uint)
                                        as png_uint_32;
                                }
                                g_0 = ((g_0 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int
                                    | 0x7f as ::core::ffi::c_uint)
                                    as png_uint_32;
                            }
                            r = ((r as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int
                                | 0x7f as ::core::ffi::c_uint)
                                as png_uint_32;
                        }
                        expand_tRNS = 1 as ::core::ffi::c_int;
                        output_processing = PNG_CMAP_RGB_ALPHA as ::core::ffi::c_uint;
                    } else {
                        let mut sample_size: ::core::ffi::c_uint = (output_format
                            as ::core::ffi::c_uint
                            & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
                            .wrapping_add(1 as ::core::ffi::c_uint)
                            .wrapping_mul(
                                ((output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR)
                                    >> 2 as ::core::ffi::c_int)
                                    .wrapping_add(1 as ::core::ffi::c_uint),
                            );
                        let mut r_0: png_uint_32 = 0;
                        let mut g_1: png_uint_32 = 0;
                        let mut b_0: png_uint_32 = 0;
                        if (PNG_RGB_COLORMAP_ENTRIES
                            + 1 as ::core::ffi::c_int
                            + 27 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint
                            > (*image).colormap_entries
                        {
                            png_error(
                                png_ptr,
                                b"rgb-alpha color-map: too few entries\0" as *const u8
                                    as png_const_charp,
                            );
                        }
                        cmap_entries = make_rgb_colormap(display) as ::core::ffi::c_uint;
                        png_create_colormap_entry(
                            display,
                            cmap_entries as png_uint_32,
                            back_r,
                            back_g,
                            back_b,
                            0 as png_uint_32,
                            output_encoding,
                        );
                        if output_encoding == P_LINEAR {
                            r_0 = (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[((back_r as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((back_r as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[((back_r as ::core::ffi::c_uint)
                                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32;
                            g_1 = (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[((back_g as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((back_g as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[((back_g as ::core::ffi::c_uint)
                                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32;
                            b_0 = (0xff as ::core::ffi::c_uint
                                & (png_sRGB_base[((back_b as ::core::ffi::c_uint)
                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_uint)
                                    .wrapping_add(
                                        ((back_b as ::core::ffi::c_uint)
                                            .wrapping_mul(255 as ::core::ffi::c_uint)
                                            & 0x7fff as ::core::ffi::c_uint)
                                            .wrapping_mul(
                                                png_sRGB_delta[((back_b as ::core::ffi::c_uint)
                                                    .wrapping_mul(255 as ::core::ffi::c_uint)
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint,
                                            )
                                            >> 12 as ::core::ffi::c_int,
                                    )
                                    >> 8 as ::core::ffi::c_int)
                                as png_byte as png_uint_32;
                        } else {
                            r_0 = back_r;
                            g_1 = back_g;
                            b_0 = back_b;
                        }
                        if memcmp(
                            ((*display).colormap as png_const_bytep)
                                .offset(sample_size.wrapping_mul(cmap_entries) as isize)
                                as *const ::core::ffi::c_void,
                            ((*display).colormap as png_const_bytep).offset(
                                sample_size.wrapping_mul(
                                    (6 as ::core::ffi::c_uint)
                                        .wrapping_mul(
                                            (6 as ::core::ffi::c_uint)
                                                .wrapping_mul(
                                                    (r_0 as ::core::ffi::c_uint)
                                                        .wrapping_mul(5 as ::core::ffi::c_uint)
                                                        .wrapping_add(130 as ::core::ffi::c_uint)
                                                        >> 8 as ::core::ffi::c_int,
                                                )
                                                .wrapping_add(
                                                    (g_1 as ::core::ffi::c_uint)
                                                        .wrapping_mul(5 as ::core::ffi::c_uint)
                                                        .wrapping_add(130 as ::core::ffi::c_uint)
                                                        >> 8 as ::core::ffi::c_int,
                                                ),
                                        )
                                        .wrapping_add(
                                            (b_0 as ::core::ffi::c_uint)
                                                .wrapping_mul(5 as ::core::ffi::c_uint)
                                                .wrapping_add(130 as ::core::ffi::c_uint)
                                                >> 8 as ::core::ffi::c_int,
                                        ) as png_byte
                                        as ::core::ffi::c_uint,
                                ) as isize,
                            ) as *const ::core::ffi::c_void,
                            sample_size as size_t,
                        ) != 0 as ::core::ffi::c_int
                        {
                            let fresh13 = cmap_entries;
                            cmap_entries = cmap_entries.wrapping_add(1);
                            background_index = fresh13;
                            r_0 = 0 as png_uint_32;
                            while r_0 < 256 as ::core::ffi::c_uint {
                                g_1 = 0 as png_uint_32;
                                while g_1 < 256 as ::core::ffi::c_uint {
                                    b_0 = 0 as png_uint_32;
                                    while b_0 < 256 as ::core::ffi::c_uint {
                                        let fresh14 = cmap_entries;
                                        cmap_entries = cmap_entries.wrapping_add(1);
                                        png_create_colormap_entry(
                                            display,
                                            fresh14,
                                            png_colormap_compose(
                                                display,
                                                r_0,
                                                P_sRGB,
                                                128 as png_uint_32,
                                                back_r,
                                                output_encoding,
                                            ),
                                            png_colormap_compose(
                                                display,
                                                g_1,
                                                P_sRGB,
                                                128 as png_uint_32,
                                                back_g,
                                                output_encoding,
                                            ),
                                            png_colormap_compose(
                                                display,
                                                b_0,
                                                P_sRGB,
                                                128 as png_uint_32,
                                                back_b,
                                                output_encoding,
                                            ),
                                            0 as png_uint_32,
                                            output_encoding,
                                        );
                                        b_0 = ((b_0 as ::core::ffi::c_uint)
                                            << 1 as ::core::ffi::c_int
                                            | 0x7f as ::core::ffi::c_uint)
                                            as png_uint_32;
                                    }
                                    g_1 = ((g_1 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int
                                        | 0x7f as ::core::ffi::c_uint)
                                        as png_uint_32;
                                }
                                r_0 = ((r_0 as ::core::ffi::c_uint) << 1 as ::core::ffi::c_int
                                    | 0x7f as ::core::ffi::c_uint)
                                    as png_uint_32;
                            }
                            expand_tRNS = 1 as ::core::ffi::c_int;
                            output_processing = PNG_CMAP_RGB_ALPHA as ::core::ffi::c_uint;
                        } else {
                            let mut c_2: png_color_16 = png_color_16 {
                                index: 0,
                                red: 0,
                                green: 0,
                                blue: 0,
                                gray: 0,
                            };
                            c_2.index = 0 as png_byte;
                            c_2.red = back_r as png_uint_16;
                            c_2.green = back_g as png_uint_16;
                            c_2.gray = c_2.green;
                            c_2.blue = back_b as png_uint_16;
                            png_set_background_fixed(
                                png_ptr,
                                &raw mut c_2 as png_const_color_16p,
                                PNG_BACKGROUND_GAMMA_SCREEN,
                                0 as ::core::ffi::c_int,
                                0 as png_fixed_point,
                            );
                            output_processing = PNG_CMAP_RGB as ::core::ffi::c_uint;
                        }
                    }
                } else {
                    if PNG_RGB_COLORMAP_ENTRIES as ::core::ffi::c_uint > (*image).colormap_entries {
                        png_error(
                            png_ptr,
                            b"rgb color-map: too few entries\0" as *const u8 as png_const_charp,
                        );
                    }
                    cmap_entries = make_rgb_colormap(display) as ::core::ffi::c_uint;
                    output_processing = PNG_CMAP_RGB as ::core::ffi::c_uint;
                }
            }
        }
        PNG_COLOR_TYPE_PALETTE => {
            let mut num_trans: ::core::ffi::c_uint = (*png_ptr).num_trans as ::core::ffi::c_uint;
            let mut trans_0: png_const_bytep = (if num_trans > 0 as ::core::ffi::c_uint {
                (*png_ptr).trans_alpha
            } else {
                ::core::ptr::null_mut::<png_byte>()
            }) as png_const_bytep;
            let mut colormap: png_const_colorp = (*png_ptr).palette as png_const_colorp;
            let mut do_background: ::core::ffi::c_int = (!trans_0.is_null()
                && output_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                    == 0 as ::core::ffi::c_uint)
                as ::core::ffi::c_int;
            let mut i_1: ::core::ffi::c_uint = 0;
            if trans_0.is_null() {
                num_trans = 0 as ::core::ffi::c_uint;
            }
            output_processing = PNG_CMAP_NONE as ::core::ffi::c_uint;
            data_encoding = P_FILE as ::core::ffi::c_uint;
            cmap_entries = (*png_ptr).num_palette as ::core::ffi::c_uint;
            if cmap_entries > 256 as ::core::ffi::c_uint {
                cmap_entries = 256 as ::core::ffi::c_uint;
            }
            if cmap_entries > (*image).colormap_entries {
                png_error(
                    png_ptr,
                    b"palette color-map: too few entries\0" as *const u8 as png_const_charp,
                );
            }
            i_1 = 0 as ::core::ffi::c_uint;
            while i_1 < cmap_entries {
                if do_background != 0 as ::core::ffi::c_int
                    && i_1 < num_trans
                    && (*trans_0.offset(i_1 as isize) as ::core::ffi::c_int)
                        < 255 as ::core::ffi::c_int
                {
                    if *trans_0.offset(i_1 as isize) as ::core::ffi::c_int
                        == 0 as ::core::ffi::c_int
                    {
                        png_create_colormap_entry(
                            display,
                            i_1 as png_uint_32,
                            back_r,
                            back_g,
                            back_b,
                            0 as png_uint_32,
                            output_encoding,
                        );
                    } else {
                        png_create_colormap_entry(
                            display,
                            i_1 as png_uint_32,
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i_1 as isize)).red as png_uint_32,
                                P_FILE,
                                *trans_0.offset(i_1 as isize) as png_uint_32,
                                back_r,
                                output_encoding,
                            ),
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i_1 as isize)).green as png_uint_32,
                                P_FILE,
                                *trans_0.offset(i_1 as isize) as png_uint_32,
                                back_g,
                                output_encoding,
                            ),
                            png_colormap_compose(
                                display,
                                (*colormap.offset(i_1 as isize)).blue as png_uint_32,
                                P_FILE,
                                *trans_0.offset(i_1 as isize) as png_uint_32,
                                back_b,
                                output_encoding,
                            ),
                            if output_encoding == P_LINEAR {
                                (*trans_0.offset(i_1 as isize) as png_uint_32)
                                    .wrapping_mul(257 as png_uint_32)
                            } else {
                                *trans_0.offset(i_1 as isize) as png_uint_32
                            },
                            output_encoding,
                        );
                    }
                } else {
                    png_create_colormap_entry(
                        display,
                        i_1 as png_uint_32,
                        (*colormap.offset(i_1 as isize)).red as png_uint_32,
                        (*colormap.offset(i_1 as isize)).green as png_uint_32,
                        (*colormap.offset(i_1 as isize)).blue as png_uint_32,
                        if i_1 < num_trans {
                            *trans_0.offset(i_1 as isize) as png_uint_32
                        } else {
                            255 as png_uint_32
                        },
                        P_FILE,
                    );
                }
                i_1 = i_1.wrapping_add(1);
            }
            if ((*png_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
                png_set_packing(png_ptr);
            }
        }
        _ => {
            png_error(
                png_ptr,
                b"invalid PNG color type\0" as *const u8 as png_const_charp,
            );
        }
    }
    if expand_tRNS != 0 as ::core::ffi::c_int
        && (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int
        && (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
            == 0 as ::core::ffi::c_int
    {
        png_set_tRNS_to_alpha(png_ptr);
    }
    let mut current_block_231: u64;
    match data_encoding {
        1 => {
            png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, PNG_GAMMA_sRGB);
            current_block_231 = 15366051406109840598;
        }
        3 => {
            current_block_231 = 15366051406109840598;
        }
        _ => {
            png_error(
                png_ptr,
                b"bad data option (internal error)\0" as *const u8 as png_const_charp,
            );
            current_block_231 = 1948584361000433526;
        }
    }
    match current_block_231 {
        15366051406109840598 => {
            if (*png_ptr).bit_depth as ::core::ffi::c_int > 8 as ::core::ffi::c_int {
                png_set_scale_16(png_ptr);
            }
        }
        _ => {}
    }
    if cmap_entries > 256 as ::core::ffi::c_uint || cmap_entries > (*image).colormap_entries {
        png_error(
            png_ptr,
            b"color map overflow (BAD internal error)\0" as *const u8 as png_const_charp,
        );
    }
    (*image).colormap_entries = cmap_entries as png_uint_32;
    match output_processing {
        0 => {
            if background_index != PNG_CMAP_NONE_BACKGROUND as ::core::ffi::c_uint {
                current_block = 10483825807117846875;
            } else {
                current_block = 17916325244215494384;
            }
        }
        1 => {
            if background_index != PNG_CMAP_GA_BACKGROUND as ::core::ffi::c_uint {
                current_block = 10483825807117846875;
            } else {
                current_block = 17916325244215494384;
            }
        }
        2 => {
            if background_index >= cmap_entries
                || background_index != PNG_CMAP_TRANS_BACKGROUND as ::core::ffi::c_uint
            {
                current_block = 10483825807117846875;
            } else {
                current_block = 17916325244215494384;
            }
        }
        3 => {
            if background_index != PNG_CMAP_RGB_BACKGROUND as ::core::ffi::c_uint {
                current_block = 10483825807117846875;
            } else {
                current_block = 17916325244215494384;
            }
        }
        4 => {
            if background_index != PNG_CMAP_RGB_ALPHA_BACKGROUND as ::core::ffi::c_uint {
                current_block = 10483825807117846875;
            } else {
                current_block = 17916325244215494384;
            }
        }
        _ => {
            png_error(
                png_ptr,
                b"bad processing option (internal error)\0" as *const u8 as png_const_charp,
            );
            current_block = 10483825807117846875;
        }
    }
    match current_block {
        10483825807117846875 => {
            png_error(
                png_ptr,
                b"bad background index (internal error)\0" as *const u8 as png_const_charp,
            );
        }
        _ => {}
    }
    (*display).colormap_processing = output_processing as ::core::ffi::c_int;
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_and_map(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut passes: ::core::ffi::c_int = 0;
    match (*png_ptr).interlaced as ::core::ffi::c_int {
        PNG_INTERLACE_NONE => {
            passes = 1 as ::core::ffi::c_int;
        }
        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }
        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0" as *const u8 as png_const_charp,
            );
        }
    }
    let mut height: png_uint_32 = (*image).height;
    let mut width: png_uint_32 = (*image).width;
    let mut proc_0: ::core::ffi::c_int = (*display).colormap_processing;
    let mut first_row: png_bytep = (*display).first_row as png_bytep;
    let mut row_step: ptrdiff_t = (*display).row_step;
    let mut pass: ::core::ffi::c_int = 0;
    let mut current_block_60: u64;
    pass = 0 as ::core::ffi::c_int;
    while pass < passes {
        let mut startx: ::core::ffi::c_uint = 0;
        let mut stepx: ::core::ffi::c_uint = 0;
        let mut stepy: ::core::ffi::c_uint = 0;
        let mut y: png_uint_32 = 0;
        if (*png_ptr).interlaced as ::core::ffi::c_int == PNG_INTERLACE_ADAM7 {
            if (width as ::core::ffi::c_uint).wrapping_add(
                (((1 as ::core::ffi::c_int)
                    << (if pass > 1 as ::core::ffi::c_int {
                        7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }))
                    - 1 as ::core::ffi::c_int
                    - ((1 as ::core::ffi::c_int & pass)
                        << 3 as ::core::ffi::c_int
                            - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                        & 7 as ::core::ffi::c_int)) as ::core::ffi::c_uint,
            ) >> (if pass > 1 as ::core::ffi::c_int {
                7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
            } else {
                3 as ::core::ffi::c_int
            }) == 0 as ::core::ffi::c_uint
            {
                current_block_60 = 1917311967535052937;
            } else {
                startx = ((1 as ::core::ffi::c_int & pass)
                    << 3 as ::core::ffi::c_int
                        - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                    & 7 as ::core::ffi::c_int) as ::core::ffi::c_uint;
                stepx = ((1 as ::core::ffi::c_int)
                    << (7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int))
                    as ::core::ffi::c_uint;
                y = ((1 as ::core::ffi::c_int & !pass)
                    << 3 as ::core::ffi::c_int - (pass >> 1 as ::core::ffi::c_int)
                    & 7 as ::core::ffi::c_int) as png_uint_32;
                stepy = (if pass > 2 as ::core::ffi::c_int {
                    8 as ::core::ffi::c_int
                        >> (pass - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                } else {
                    8 as ::core::ffi::c_int
                }) as ::core::ffi::c_uint;
                current_block_60 = 12124785117276362961;
            }
        } else {
            y = 0 as png_uint_32;
            startx = 0 as ::core::ffi::c_uint;
            stepy = 1 as ::core::ffi::c_uint;
            stepx = stepy;
            current_block_60 = 12124785117276362961;
        }
        match current_block_60 {
            12124785117276362961 => {
                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep =
                        first_row.offset((y as ptrdiff_t * row_step) as isize);
                    let mut row_end: png_const_bytep =
                        outrow.offset(width as isize) as png_const_bytep;
                    png_read_row(png_ptr, inrow, ::core::ptr::null_mut::<png_byte>());
                    outrow = outrow.offset(startx as isize);
                    match proc_0 {
                        PNG_CMAP_GA => {
                            while outrow < row_end as png_bytep {
                                let fresh4 = inrow;
                                inrow = inrow.offset(1);
                                let mut gray: ::core::ffi::c_uint = *fresh4 as ::core::ffi::c_uint;
                                let fresh5 = inrow;
                                inrow = inrow.offset(1);
                                let mut alpha: ::core::ffi::c_uint = *fresh5 as ::core::ffi::c_uint;
                                let mut entry: ::core::ffi::c_uint = 0;
                                if alpha > 229 as ::core::ffi::c_uint {
                                    entry = (231 as ::core::ffi::c_uint)
                                        .wrapping_mul(gray)
                                        .wrapping_add(128 as ::core::ffi::c_uint)
                                        >> 8 as ::core::ffi::c_int;
                                } else if alpha < 26 as ::core::ffi::c_uint {
                                    entry = 231 as ::core::ffi::c_uint;
                                } else {
                                    entry = (226 as ::core::ffi::c_uint)
                                        .wrapping_add(
                                            (6 as ::core::ffi::c_uint).wrapping_mul(
                                                alpha
                                                    .wrapping_mul(5 as ::core::ffi::c_uint)
                                                    .wrapping_add(130 as ::core::ffi::c_uint)
                                                    >> 8 as ::core::ffi::c_int,
                                            ),
                                        )
                                        .wrapping_add(
                                            gray.wrapping_mul(5 as ::core::ffi::c_uint)
                                                .wrapping_add(130 as ::core::ffi::c_uint)
                                                >> 8 as ::core::ffi::c_int,
                                        );
                                }
                                *outrow = entry as png_byte;
                                outrow = outrow.offset(stepx as isize);
                            }
                        }
                        PNG_CMAP_TRANS => {
                            while outrow < row_end as png_bytep {
                                let fresh6 = inrow;
                                inrow = inrow.offset(1);
                                let mut gray_0: png_byte = *fresh6;
                                let fresh7 = inrow;
                                inrow = inrow.offset(1);
                                let mut alpha_0: png_byte = *fresh7;
                                if alpha_0 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                                    *outrow = PNG_CMAP_TRANS_BACKGROUND as png_byte;
                                } else if gray_0 as ::core::ffi::c_int != PNG_CMAP_TRANS_BACKGROUND
                                {
                                    *outrow = gray_0;
                                } else {
                                    *outrow = (PNG_CMAP_TRANS_BACKGROUND + 1 as ::core::ffi::c_int)
                                        as png_byte;
                                }
                                outrow = outrow.offset(stepx as isize);
                            }
                        }
                        PNG_CMAP_RGB => {
                            while outrow < row_end as png_bytep {
                                *outrow = (6 as ::core::ffi::c_int
                                    * (6 as ::core::ffi::c_int
                                        * (*inrow.offset(0 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            * 5 as ::core::ffi::c_int
                                            + 130 as ::core::ffi::c_int
                                            >> 8 as ::core::ffi::c_int)
                                        + (*inrow.offset(1 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            * 5 as ::core::ffi::c_int
                                            + 130 as ::core::ffi::c_int
                                            >> 8 as ::core::ffi::c_int))
                                    + (*inrow.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        * 5 as ::core::ffi::c_int
                                        + 130 as ::core::ffi::c_int
                                        >> 8 as ::core::ffi::c_int))
                                    as png_byte;
                                inrow = inrow.offset(3 as ::core::ffi::c_int as isize);
                                outrow = outrow.offset(stepx as isize);
                            }
                        }
                        PNG_CMAP_RGB_ALPHA => {
                            while outrow < row_end as png_bytep {
                                let mut alpha_1: ::core::ffi::c_uint = *inrow
                                    .offset(3 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_uint;
                                if alpha_1 >= 196 as ::core::ffi::c_uint {
                                    *outrow = (6 as ::core::ffi::c_int
                                        * (6 as ::core::ffi::c_int
                                            * (*inrow.offset(0 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                * 5 as ::core::ffi::c_int
                                                + 130 as ::core::ffi::c_int
                                                >> 8 as ::core::ffi::c_int)
                                            + (*inrow.offset(1 as ::core::ffi::c_int as isize)
                                                as ::core::ffi::c_int
                                                * 5 as ::core::ffi::c_int
                                                + 130 as ::core::ffi::c_int
                                                >> 8 as ::core::ffi::c_int))
                                        + (*inrow.offset(2 as ::core::ffi::c_int as isize)
                                            as ::core::ffi::c_int
                                            * 5 as ::core::ffi::c_int
                                            + 130 as ::core::ffi::c_int
                                            >> 8 as ::core::ffi::c_int))
                                        as png_byte;
                                } else if alpha_1 < 64 as ::core::ffi::c_uint {
                                    *outrow = PNG_CMAP_RGB_ALPHA_BACKGROUND as png_byte;
                                } else {
                                    let mut back_i: ::core::ffi::c_uint =
                                        (PNG_CMAP_RGB_ALPHA_BACKGROUND + 1 as ::core::ffi::c_int)
                                            as ::core::ffi::c_uint;
                                    if *inrow.offset(0 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x80 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(9 as ::core::ffi::c_uint);
                                    }
                                    if *inrow.offset(0 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x40 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(9 as ::core::ffi::c_uint);
                                    }
                                    if *inrow.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x80 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(3 as ::core::ffi::c_uint);
                                    }
                                    if *inrow.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x40 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(3 as ::core::ffi::c_uint);
                                    }
                                    if *inrow.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x80 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(1 as ::core::ffi::c_uint);
                                    }
                                    if *inrow.offset(2 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                        & 0x40 as ::core::ffi::c_int
                                        != 0
                                    {
                                        back_i = back_i.wrapping_add(1 as ::core::ffi::c_uint);
                                    }
                                    *outrow = back_i as png_byte;
                                }
                                inrow = inrow.offset(4 as ::core::ffi::c_int as isize);
                                outrow = outrow.offset(stepx as isize);
                            }
                        }
                        _ => {}
                    }
                    y = (y as ::core::ffi::c_uint).wrapping_add(stepy) as png_uint_32
                        as png_uint_32;
                }
            }
            _ => {}
        }
        pass += 1;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_colormapped(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut current_block: u64;
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut control: png_controlp = (*image).opaque;
    let mut png_ptr: png_structrp = (*control).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*control).info_ptr as png_inforp;
    let mut passes: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    png_image_skip_unused_chunks(png_ptr);
    if (*display).colormap_processing == PNG_CMAP_NONE {
        passes = png_set_interlace_handling(png_ptr);
    }
    png_read_update_info(png_ptr, info_ptr);
    match (*display).colormap_processing {
        PNG_CMAP_NONE => {
            if ((*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
                || (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY)
                && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
            {
                current_block = 9606288038608642794;
            } else {
                current_block = 12146204548183233919;
            }
        }
        PNG_CMAP_TRANS | PNG_CMAP_GA => {
            if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA
                && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 256 as ::core::ffi::c_uint
            {
                current_block = 9606288038608642794;
            } else {
                current_block = 12146204548183233919;
            }
        }
        PNG_CMAP_RGB => {
            if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
                && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 216 as ::core::ffi::c_uint
            {
                current_block = 9606288038608642794;
            } else {
                current_block = 12146204548183233919;
            }
        }
        PNG_CMAP_RGB_ALPHA => {
            if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
                && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
                && (*png_ptr).screen_gamma == PNG_GAMMA_sRGB
                && (*image).colormap_entries == 244 as ::core::ffi::c_uint
            {
                current_block = 9606288038608642794;
            } else {
                current_block = 12146204548183233919;
            }
        }
        _ => {
            current_block = 12146204548183233919;
        }
    }
    match current_block {
        12146204548183233919 => {
            png_error(
                png_ptr,
                b"bad color-map processing (internal error)\0" as *const u8 as png_const_charp,
            );
        }
        _ => {}
    }
    let mut first_row: png_voidp = (*display).buffer;
    let mut row_step: ptrdiff_t = (*display).row_stride as ptrdiff_t;
    if row_step < 0 as ptrdiff_t {
        let mut ptr: *mut ::core::ffi::c_char = first_row as *mut ::core::ffi::c_char;
        ptr = ptr.offset(
            (((*image).height as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                as ptrdiff_t
                * -row_step) as isize,
        );
        first_row = ptr as png_voidp;
    }
    (*display).first_row = first_row;
    (*display).row_step = row_step;
    if passes == 0 as ::core::ffi::c_int {
        let mut result: ::core::ffi::c_int = 0;
        let mut row: png_voidp = png_malloc(
            png_ptr,
            png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t,
        );
        (*display).local_row = row;
        result = png_safe_execute(
            image,
            Some(png_image_read_and_map as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int),
            display as png_voidp,
        );
        (*display).local_row = NULL_0 as png_voidp;
        png_free(png_ptr, row);
        return result;
    } else {
        let mut row_step_0: ptrdiff_t = (*display).row_step;
        loop {
            passes -= 1;
            if !(passes >= 0 as ::core::ffi::c_int) {
                break;
            }
            let mut y: png_uint_32 = (*image).height;
            let mut row_0: png_bytep = (*display).first_row as png_bytep;
            while y > 0 as ::core::ffi::c_uint {
                png_read_row(png_ptr, row_0, ::core::ptr::null_mut::<png_byte>());
                row_0 = row_0.offset(row_step_0 as isize);
                y = y.wrapping_sub(1);
            }
        }
        return 1 as ::core::ffi::c_int;
    };
}
unsafe extern "C" fn png_image_read_direct_scaled(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*(*image).opaque).info_ptr as png_inforp;
    let mut local_row: png_bytep = (*display).local_row as png_bytep;
    let mut first_row: png_bytep = (*display).first_row as png_bytep;
    let mut row_step: ptrdiff_t = (*display).row_step;
    let mut row_bytes: size_t = png_get_rowbytes(png_ptr, info_ptr);
    let mut passes: ::core::ffi::c_int = 0;
    match (*png_ptr).interlaced as ::core::ffi::c_int {
        PNG_INTERLACE_NONE => {
            passes = 1 as ::core::ffi::c_int;
        }
        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }
        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0" as *const u8 as png_const_charp,
            );
        }
    }
    loop {
        passes -= 1;
        if !(passes >= 0 as ::core::ffi::c_int) {
            break;
        }
        let mut y: png_uint_32 = (*image).height;
        let mut output_row: png_bytep = first_row;
        while y > 0 as ::core::ffi::c_uint {
            png_read_row(png_ptr, local_row, ::core::ptr::null_mut::<png_byte>());
            memcpy(
                output_row as *mut ::core::ffi::c_void,
                local_row as *const ::core::ffi::c_void,
                row_bytes,
            );
            output_row = output_row.offset(row_step as isize);
            y = y.wrapping_sub(1);
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_composite(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut passes: ::core::ffi::c_int = 0;
    match (*png_ptr).interlaced as ::core::ffi::c_int {
        PNG_INTERLACE_NONE => {
            passes = 1 as ::core::ffi::c_int;
        }
        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }
        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0" as *const u8 as png_const_charp,
            );
        }
    }
    let mut height: png_uint_32 = (*image).height;
    let mut width: png_uint_32 = (*image).width;
    let mut row_step: ptrdiff_t = (*display).row_step;
    let mut channels: ::core::ffi::c_uint = (if (*image).format as ::core::ffi::c_uint
        & PNG_FORMAT_FLAG_COLOR
        != 0 as ::core::ffi::c_uint
    {
        3 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    let mut optimize_alpha: ::core::ffi::c_int =
        ((*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_OPTIMIZE_ALPHA
            != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    let mut pass: ::core::ffi::c_int = 0;
    let mut current_block_42: u64;
    pass = 0 as ::core::ffi::c_int;
    while pass < passes {
        let mut startx: ::core::ffi::c_uint = 0;
        let mut stepx: ::core::ffi::c_uint = 0;
        let mut stepy: ::core::ffi::c_uint = 0;
        let mut y: png_uint_32 = 0;
        if (*png_ptr).interlaced as ::core::ffi::c_int == PNG_INTERLACE_ADAM7 {
            if (width as ::core::ffi::c_uint).wrapping_add(
                (((1 as ::core::ffi::c_int)
                    << (if pass > 1 as ::core::ffi::c_int {
                        7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }))
                    - 1 as ::core::ffi::c_int
                    - ((1 as ::core::ffi::c_int & pass)
                        << 3 as ::core::ffi::c_int
                            - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                        & 7 as ::core::ffi::c_int)) as ::core::ffi::c_uint,
            ) >> (if pass > 1 as ::core::ffi::c_int {
                7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
            } else {
                3 as ::core::ffi::c_int
            }) == 0 as ::core::ffi::c_uint
            {
                current_block_42 = 1917311967535052937;
            } else {
                startx = (((1 as ::core::ffi::c_int & pass)
                    << 3 as ::core::ffi::c_int
                        - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                    & 7 as ::core::ffi::c_int) as ::core::ffi::c_uint)
                    .wrapping_mul(channels);
                stepx = (((1 as ::core::ffi::c_int)
                    << (7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int))
                    as ::core::ffi::c_uint)
                    .wrapping_mul(channels);
                y = ((1 as ::core::ffi::c_int & !pass)
                    << 3 as ::core::ffi::c_int - (pass >> 1 as ::core::ffi::c_int)
                    & 7 as ::core::ffi::c_int) as png_uint_32;
                stepy = (if pass > 2 as ::core::ffi::c_int {
                    8 as ::core::ffi::c_int
                        >> (pass - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                } else {
                    8 as ::core::ffi::c_int
                }) as ::core::ffi::c_uint;
                current_block_42 = 4808432441040389987;
            }
        } else {
            y = 0 as png_uint_32;
            startx = 0 as ::core::ffi::c_uint;
            stepx = channels;
            stepy = 1 as ::core::ffi::c_uint;
            current_block_42 = 4808432441040389987;
        }
        match current_block_42 {
            4808432441040389987 => {
                while y < height {
                    let mut inrow: png_bytep = (*display).local_row as png_bytep;
                    let mut outrow: png_bytep = ::core::ptr::null_mut::<png_byte>();
                    let mut row_end: png_const_bytep = ::core::ptr::null::<png_byte>();
                    png_read_row(png_ptr, inrow, ::core::ptr::null_mut::<png_byte>());
                    outrow = (*display).first_row as png_bytep;
                    outrow = outrow.offset((y as ptrdiff_t * row_step) as isize);
                    row_end = outrow
                        .offset((width as ::core::ffi::c_uint).wrapping_mul(channels) as isize)
                        as png_const_bytep;
                    outrow = outrow.offset(startx as isize);
                    while outrow < row_end as png_bytep {
                        let mut alpha: png_byte = *inrow.offset(channels as isize);
                        if alpha as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                            let mut c: ::core::ffi::c_uint = 0;
                            c = 0 as ::core::ffi::c_uint;
                            while c < channels {
                                let mut component: png_uint_32 =
                                    *inrow.offset(c as isize) as png_uint_32;
                                if (alpha as ::core::ffi::c_int) < 255 as ::core::ffi::c_int {
                                    if optimize_alpha != 0 as ::core::ffi::c_int {
                                        component = (component as ::core::ffi::c_uint).wrapping_mul(
                                            (257 as ::core::ffi::c_int * 255 as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint,
                                        )
                                            as png_uint_32
                                            as png_uint_32;
                                        component = (component as ::core::ffi::c_uint).wrapping_add(
                                            ((255 as ::core::ffi::c_int
                                                - alpha as ::core::ffi::c_int)
                                                * png_sRGB_table
                                                    [*outrow.offset(c as isize) as usize]
                                                    as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint,
                                        )
                                            as png_uint_32
                                            as png_uint_32;
                                        if component
                                            > (255 as ::core::ffi::c_int
                                                * 65535 as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint
                                        {
                                            component = (255 as ::core::ffi::c_int
                                                * 65535 as ::core::ffi::c_int)
                                                as png_uint_32;
                                        }
                                        component = (0xff as ::core::ffi::c_uint
                                            & (png_sRGB_base
                                                [(component >> 15 as ::core::ffi::c_int) as usize]
                                                as ::core::ffi::c_uint)
                                                .wrapping_add(
                                                    (component as ::core::ffi::c_uint
                                                        & 0x7fff as ::core::ffi::c_uint)
                                                        .wrapping_mul(
                                                            png_sRGB_delta[(component
                                                                >> 15 as ::core::ffi::c_int)
                                                                as usize]
                                                                as ::core::ffi::c_uint,
                                                        )
                                                        >> 12 as ::core::ffi::c_int,
                                                )
                                                >> 8 as ::core::ffi::c_int)
                                            as png_byte
                                            as png_uint_32;
                                    } else {
                                        let mut background: png_uint_32 =
                                            *outrow.offset(c as isize) as png_uint_32;
                                        component = (component as ::core::ffi::c_uint).wrapping_add(
                                            ((255 as ::core::ffi::c_int
                                                - alpha as ::core::ffi::c_int)
                                                as ::core::ffi::c_uint)
                                                .wrapping_mul(background as ::core::ffi::c_uint)
                                                .wrapping_add(127 as ::core::ffi::c_uint)
                                                .wrapping_div(255 as ::core::ffi::c_uint),
                                        )
                                            as png_uint_32
                                            as png_uint_32;
                                        if component > 255 as ::core::ffi::c_uint {
                                            component = 255 as png_uint_32;
                                        }
                                    }
                                }
                                *outrow.offset(c as isize) = component as png_byte;
                                c = c.wrapping_add(1);
                            }
                        }
                        inrow =
                            inrow.offset(channels.wrapping_add(1 as ::core::ffi::c_uint) as isize);
                        outrow = outrow.offset(stepx as isize);
                    }
                    y = (y as ::core::ffi::c_uint).wrapping_add(stepy) as png_uint_32
                        as png_uint_32;
                }
            }
            _ => {}
        }
        pass += 1;
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_background(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*(*image).opaque).info_ptr as png_inforp;
    let mut height: png_uint_32 = (*image).height;
    let mut width: png_uint_32 = (*image).width;
    let mut pass: ::core::ffi::c_int = 0;
    let mut passes: ::core::ffi::c_int = 0;
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
        == 0 as ::core::ffi::c_uint
    {
        png_error(
            png_ptr,
            b"lost rgb to gray\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE != 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"unexpected compose\0" as *const u8 as png_const_charp,
        );
    }
    if png_get_channels(png_ptr, info_ptr) as ::core::ffi::c_int != 2 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"lost/gained channels\0" as *const u8 as png_const_charp,
        );
    }
    if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR == 0 as ::core::ffi::c_uint
        && (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
            != 0 as ::core::ffi::c_uint
    {
        png_error(
            png_ptr,
            b"unexpected 8-bit transformation\0" as *const u8 as png_const_charp,
        );
    }
    match (*png_ptr).interlaced as ::core::ffi::c_int {
        PNG_INTERLACE_NONE => {
            passes = 1 as ::core::ffi::c_int;
        }
        PNG_INTERLACE_ADAM7 => {
            passes = PNG_INTERLACE_ADAM7_PASSES;
        }
        _ => {
            png_error(
                png_ptr,
                b"unknown interlace type\0" as *const u8 as png_const_charp,
            );
        }
    }
    match (*info_ptr).bit_depth as ::core::ffi::c_int {
        8 => {
            let mut first_row: png_bytep = (*display).first_row as png_bytep;
            let mut row_step: ptrdiff_t = (*display).row_step;
            let mut current_block_54: u64;
            pass = 0 as ::core::ffi::c_int;
            while pass < passes {
                let mut startx: ::core::ffi::c_uint = 0;
                let mut stepx: ::core::ffi::c_uint = 0;
                let mut stepy: ::core::ffi::c_uint = 0;
                let mut y: png_uint_32 = 0;
                if (*png_ptr).interlaced as ::core::ffi::c_int == PNG_INTERLACE_ADAM7 {
                    if (width as ::core::ffi::c_uint).wrapping_add(
                        (((1 as ::core::ffi::c_int)
                            << (if pass > 1 as ::core::ffi::c_int {
                                7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                            } else {
                                3 as ::core::ffi::c_int
                            }))
                            - 1 as ::core::ffi::c_int
                            - ((1 as ::core::ffi::c_int & pass)
                                << 3 as ::core::ffi::c_int
                                    - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                                & 7 as ::core::ffi::c_int))
                            as ::core::ffi::c_uint,
                    ) >> (if pass > 1 as ::core::ffi::c_int {
                        7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }) == 0 as ::core::ffi::c_uint
                    {
                        current_block_54 = 5143058163439228106;
                    } else {
                        startx = ((1 as ::core::ffi::c_int & pass)
                            << 3 as ::core::ffi::c_int
                                - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                            & 7 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint;
                        stepx = ((1 as ::core::ffi::c_int)
                            << (7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int))
                            as ::core::ffi::c_uint;
                        y = ((1 as ::core::ffi::c_int & !pass)
                            << 3 as ::core::ffi::c_int - (pass >> 1 as ::core::ffi::c_int)
                            & 7 as ::core::ffi::c_int) as png_uint_32;
                        stepy = (if pass > 2 as ::core::ffi::c_int {
                            8 as ::core::ffi::c_int
                                >> (pass - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                        } else {
                            8 as ::core::ffi::c_int
                        }) as ::core::ffi::c_uint;
                        current_block_54 = 2232869372362427478;
                    }
                } else {
                    y = 0 as png_uint_32;
                    startx = 0 as ::core::ffi::c_uint;
                    stepy = 1 as ::core::ffi::c_uint;
                    stepx = stepy;
                    current_block_54 = 2232869372362427478;
                }
                match current_block_54 {
                    2232869372362427478 => {
                        if (*display).background.is_null() {
                            while y < height {
                                let mut inrow: png_bytep = (*display).local_row as png_bytep;
                                let mut outrow: png_bytep =
                                    first_row.offset((y as ptrdiff_t * row_step) as isize);
                                let mut row_end: png_const_bytep =
                                    outrow.offset(width as isize) as png_const_bytep;
                                png_read_row(png_ptr, inrow, ::core::ptr::null_mut::<png_byte>());
                                outrow = outrow.offset(startx as isize);
                                while outrow < row_end as png_bytep {
                                    let mut alpha: png_byte =
                                        *inrow.offset(1 as ::core::ffi::c_int as isize);
                                    if alpha as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                                        let mut component: png_uint_32 = *inrow
                                            .offset(0 as ::core::ffi::c_int as isize)
                                            as png_uint_32;
                                        if (alpha as ::core::ffi::c_int) < 255 as ::core::ffi::c_int
                                        {
                                            component = (png_sRGB_table[component as usize]
                                                as ::core::ffi::c_int
                                                * alpha as ::core::ffi::c_int)
                                                as png_uint_32;
                                            component = (component as ::core::ffi::c_uint)
                                                .wrapping_add(
                                                    (png_sRGB_table[*outrow
                                                        .offset(0 as ::core::ffi::c_int as isize)
                                                        as usize]
                                                        as ::core::ffi::c_int
                                                        * (255 as ::core::ffi::c_int
                                                            - alpha as ::core::ffi::c_int))
                                                        as ::core::ffi::c_uint,
                                                )
                                                as png_uint_32
                                                as png_uint_32;
                                            component = (0xff as ::core::ffi::c_uint
                                                & (png_sRGB_base[(component
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint)
                                                    .wrapping_add(
                                                        (component as ::core::ffi::c_uint
                                                            & 0x7fff as ::core::ffi::c_uint)
                                                            .wrapping_mul(
                                                                png_sRGB_delta[(component
                                                                    >> 15 as ::core::ffi::c_int)
                                                                    as usize]
                                                                    as ::core::ffi::c_uint,
                                                            )
                                                            >> 12 as ::core::ffi::c_int,
                                                    )
                                                    >> 8 as ::core::ffi::c_int)
                                                as png_byte
                                                as png_uint_32;
                                        }
                                        *outrow.offset(0 as ::core::ffi::c_int as isize) =
                                            component as png_byte;
                                    }
                                    inrow = inrow.offset(2 as ::core::ffi::c_int as isize);
                                    outrow = outrow.offset(stepx as isize);
                                }
                                y = (y as ::core::ffi::c_uint).wrapping_add(stepy) as png_uint_32
                                    as png_uint_32;
                            }
                        } else {
                            let mut background8: png_byte = (*(*display).background).green;
                            let mut background: png_uint_16 = png_sRGB_table[background8 as usize];
                            while y < height {
                                let mut inrow_0: png_bytep = (*display).local_row as png_bytep;
                                let mut outrow_0: png_bytep =
                                    first_row.offset((y as ptrdiff_t * row_step) as isize);
                                let mut row_end_0: png_const_bytep =
                                    outrow_0.offset(width as isize) as png_const_bytep;
                                png_read_row(png_ptr, inrow_0, ::core::ptr::null_mut::<png_byte>());
                                outrow_0 = outrow_0.offset(startx as isize);
                                while outrow_0 < row_end_0 as png_bytep {
                                    let mut alpha_0: png_byte =
                                        *inrow_0.offset(1 as ::core::ffi::c_int as isize);
                                    if alpha_0 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                                        let mut component_0: png_uint_32 = *inrow_0
                                            .offset(0 as ::core::ffi::c_int as isize)
                                            as png_uint_32;
                                        if (alpha_0 as ::core::ffi::c_int)
                                            < 255 as ::core::ffi::c_int
                                        {
                                            component_0 = (png_sRGB_table[component_0 as usize]
                                                as ::core::ffi::c_int
                                                * alpha_0 as ::core::ffi::c_int)
                                                as png_uint_32;
                                            component_0 = (component_0 as ::core::ffi::c_uint)
                                                .wrapping_add(
                                                    (background as ::core::ffi::c_int
                                                        * (255 as ::core::ffi::c_int
                                                            - alpha_0 as ::core::ffi::c_int))
                                                        as ::core::ffi::c_uint,
                                                )
                                                as png_uint_32
                                                as png_uint_32;
                                            component_0 = (0xff as ::core::ffi::c_uint
                                                & (png_sRGB_base[(component_0
                                                    >> 15 as ::core::ffi::c_int)
                                                    as usize]
                                                    as ::core::ffi::c_uint)
                                                    .wrapping_add(
                                                        (component_0 as ::core::ffi::c_uint
                                                            & 0x7fff as ::core::ffi::c_uint)
                                                            .wrapping_mul(
                                                                png_sRGB_delta[(component_0
                                                                    >> 15 as ::core::ffi::c_int)
                                                                    as usize]
                                                                    as ::core::ffi::c_uint,
                                                            )
                                                            >> 12 as ::core::ffi::c_int,
                                                    )
                                                    >> 8 as ::core::ffi::c_int)
                                                as png_byte
                                                as png_uint_32;
                                        }
                                        *outrow_0.offset(0 as ::core::ffi::c_int as isize) =
                                            component_0 as png_byte;
                                    } else {
                                        *outrow_0.offset(0 as ::core::ffi::c_int as isize) =
                                            background8;
                                    }
                                    inrow_0 = inrow_0.offset(2 as ::core::ffi::c_int as isize);
                                    outrow_0 = outrow_0.offset(stepx as isize);
                                }
                                y = (y as ::core::ffi::c_uint).wrapping_add(stepy) as png_uint_32
                                    as png_uint_32;
                            }
                        }
                    }
                    _ => {}
                }
                pass += 1;
            }
        }
        16 => {
            let mut first_row_0: png_uint_16p = (*display).first_row as png_uint_16p;
            let mut row_step_0: ptrdiff_t = (*display).row_step / 2 as ptrdiff_t;
            let mut preserve_alpha: ::core::ffi::c_uint =
                ((*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                    != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int
                    as ::core::ffi::c_uint;
            let mut outchannels: ::core::ffi::c_uint =
                (1 as ::core::ffi::c_uint).wrapping_add(preserve_alpha);
            let mut swap_alpha: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            if preserve_alpha != 0 as ::core::ffi::c_uint
                && (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST
                    != 0 as ::core::ffi::c_uint
            {
                swap_alpha = 1 as ::core::ffi::c_int;
            }
            let mut current_block_87: u64;
            pass = 0 as ::core::ffi::c_int;
            while pass < passes {
                let mut startx_0: ::core::ffi::c_uint = 0;
                let mut stepx_0: ::core::ffi::c_uint = 0;
                let mut stepy_0: ::core::ffi::c_uint = 0;
                let mut y_0: png_uint_32 = 0;
                if (*png_ptr).interlaced as ::core::ffi::c_int == PNG_INTERLACE_ADAM7 {
                    if (width as ::core::ffi::c_uint).wrapping_add(
                        (((1 as ::core::ffi::c_int)
                            << (if pass > 1 as ::core::ffi::c_int {
                                7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                            } else {
                                3 as ::core::ffi::c_int
                            }))
                            - 1 as ::core::ffi::c_int
                            - ((1 as ::core::ffi::c_int & pass)
                                << 3 as ::core::ffi::c_int
                                    - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                                & 7 as ::core::ffi::c_int))
                            as ::core::ffi::c_uint,
                    ) >> (if pass > 1 as ::core::ffi::c_int {
                        7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                    } else {
                        3 as ::core::ffi::c_int
                    }) == 0 as ::core::ffi::c_uint
                    {
                        current_block_87 = 15970011996474399071;
                    } else {
                        startx_0 = (((1 as ::core::ffi::c_int & pass)
                            << 3 as ::core::ffi::c_int
                                - (pass + 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                            & 7 as ::core::ffi::c_int)
                            as ::core::ffi::c_uint)
                            .wrapping_mul(outchannels);
                        stepx_0 = (((1 as ::core::ffi::c_int)
                            << (7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int))
                            as ::core::ffi::c_uint)
                            .wrapping_mul(outchannels);
                        y_0 = ((1 as ::core::ffi::c_int & !pass)
                            << 3 as ::core::ffi::c_int - (pass >> 1 as ::core::ffi::c_int)
                            & 7 as ::core::ffi::c_int) as png_uint_32;
                        stepy_0 = (if pass > 2 as ::core::ffi::c_int {
                            8 as ::core::ffi::c_int
                                >> (pass - 1 as ::core::ffi::c_int >> 1 as ::core::ffi::c_int)
                        } else {
                            8 as ::core::ffi::c_int
                        }) as ::core::ffi::c_uint;
                        current_block_87 = 10809827304263610514;
                    }
                } else {
                    y_0 = 0 as png_uint_32;
                    startx_0 = 0 as ::core::ffi::c_uint;
                    stepx_0 = outchannels;
                    stepy_0 = 1 as ::core::ffi::c_uint;
                    current_block_87 = 10809827304263610514;
                }
                match current_block_87 {
                    10809827304263610514 => {
                        while y_0 < height {
                            let mut inrow_1: png_const_uint_16p =
                                ::core::ptr::null::<png_uint_16>();
                            let mut outrow_1: png_uint_16p =
                                first_row_0.offset((y_0 as ptrdiff_t * row_step_0) as isize);
                            let mut row_end_1: png_uint_16p = outrow_1
                                .offset((width as ::core::ffi::c_uint).wrapping_mul(outchannels)
                                    as isize);
                            png_read_row(
                                png_ptr,
                                (*display).local_row as png_bytep,
                                ::core::ptr::null_mut::<png_byte>(),
                            );
                            inrow_1 = (*display).local_row as png_const_uint_16p;
                            outrow_1 = outrow_1.offset(startx_0 as isize);
                            while outrow_1 < row_end_1 {
                                let mut component_1: png_uint_32 = *inrow_1
                                    .offset(0 as ::core::ffi::c_int as isize)
                                    as png_uint_32;
                                let mut alpha_1: png_uint_16 =
                                    *inrow_1.offset(1 as ::core::ffi::c_int as isize);
                                if alpha_1 as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                                    if (alpha_1 as ::core::ffi::c_int) < 65535 as ::core::ffi::c_int
                                    {
                                        component_1 = (component_1 as ::core::ffi::c_uint)
                                            .wrapping_mul(alpha_1 as ::core::ffi::c_uint)
                                            as png_uint_32
                                            as png_uint_32;
                                        component_1 = (component_1 as ::core::ffi::c_uint)
                                            .wrapping_add(32767 as ::core::ffi::c_uint)
                                            as png_uint_32
                                            as png_uint_32;
                                        component_1 = (component_1 as ::core::ffi::c_uint)
                                            .wrapping_div(65535 as ::core::ffi::c_uint)
                                            as png_uint_32
                                            as png_uint_32;
                                    }
                                } else {
                                    component_1 = 0 as png_uint_32;
                                }
                                *outrow_1.offset(swap_alpha as isize) = component_1 as png_uint_16;
                                if preserve_alpha != 0 as ::core::ffi::c_uint {
                                    *outrow_1
                                        .offset((1 as ::core::ffi::c_int ^ swap_alpha) as isize) =
                                        alpha_1;
                                }
                                inrow_1 = inrow_1.offset(2 as ::core::ffi::c_int as isize);
                                outrow_1 = outrow_1.offset(stepx_0 as isize);
                            }
                            y_0 = (y_0 as ::core::ffi::c_uint).wrapping_add(stepy_0) as png_uint_32
                                as png_uint_32;
                        }
                    }
                    _ => {}
                }
                pass += 1;
            }
        }
        _ => {
            png_error(
                png_ptr,
                b"unexpected bit depth\0" as *const u8 as png_const_charp,
            );
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_read_direct(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_read_control = argument as *mut png_image_read_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*(*image).opaque).info_ptr as png_inforp;
    let mut format: png_uint_32 = (*image).format;
    let mut linear: ::core::ffi::c_int = (format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR
        != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    let mut do_local_compose: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut do_local_background: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut do_local_scale: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut passes: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    png_set_expand(png_ptr);
    let mut base_format: png_uint_32 = png_image_format(png_ptr) & !PNG_FORMAT_FLAG_COLORMAP;
    let mut change: png_uint_32 = format ^ base_format;
    let mut output_gamma: png_fixed_point = 0;
    let mut mode: ::core::ffi::c_int = 0;
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 as ::core::ffi::c_uint {
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 as ::core::ffi::c_uint {
            png_set_gray_to_rgb(png_ptr);
        } else {
            if base_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA
                != 0 as ::core::ffi::c_uint
            {
                do_local_background = 1 as ::core::ffi::c_int;
            }
            png_set_rgb_to_gray_fixed(
                png_ptr,
                PNG_ERROR_ACTION_NONE,
                PNG_RGB_TO_GRAY_DEFAULT,
                PNG_RGB_TO_GRAY_DEFAULT,
            );
        }
        change &= !PNG_FORMAT_FLAG_COLOR;
    }
    let mut input_gamma_default: png_fixed_point = 0;
    if base_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR != 0 as ::core::ffi::c_uint
        && (*image).flags as ::core::ffi::c_uint & PNG_IMAGE_FLAG_16BIT_sRGB as ::core::ffi::c_uint
            == 0 as ::core::ffi::c_uint
    {
        input_gamma_default = PNG_GAMMA_LINEAR as png_fixed_point;
    } else {
        input_gamma_default = PNG_DEFAULT_sRGB as png_fixed_point;
    }
    png_set_alpha_mode_fixed(png_ptr, PNG_ALPHA_PNG, input_gamma_default);
    if linear != 0 as ::core::ffi::c_int {
        if base_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
            mode = PNG_ALPHA_STANDARD;
        } else {
            mode = PNG_ALPHA_PNG;
        }
        output_gamma = PNG_GAMMA_LINEAR as png_fixed_point;
    } else {
        mode = PNG_ALPHA_PNG;
        output_gamma = PNG_DEFAULT_sRGB as png_fixed_point;
    }
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA != 0 as ::core::ffi::c_uint
    {
        mode = PNG_ALPHA_OPTIMIZED;
        change &= !PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
    }
    if do_local_background != 0 as ::core::ffi::c_int {
        let mut gtest: png_fixed_point = 0;
        if png_muldiv(
            &raw mut gtest,
            output_gamma,
            png_resolve_file_gamma(png_ptr) as png_int_32,
            PNG_FP_1,
        ) != 0 as ::core::ffi::c_int
            && png_gamma_significant(gtest) == 0 as ::core::ffi::c_int
        {
            do_local_background = 0 as ::core::ffi::c_int;
        } else if mode == PNG_ALPHA_STANDARD {
            do_local_background = 2 as ::core::ffi::c_int;
            mode = PNG_ALPHA_PNG;
        }
    }
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR != 0 as ::core::ffi::c_uint {
        if linear != 0 as ::core::ffi::c_int {
            png_set_expand_16(png_ptr);
        } else {
            png_set_scale_16(png_ptr);
            if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                do_local_scale = 1 as ::core::ffi::c_int;
            }
        }
        change &= !PNG_FORMAT_FLAG_LINEAR;
    }
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
        if base_format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
            if do_local_background != 0 as ::core::ffi::c_int {
                do_local_background = 2 as ::core::ffi::c_int;
            } else if linear != 0 as ::core::ffi::c_int {
                png_set_strip_alpha(png_ptr);
            } else if !(*display).background.is_null() {
                let mut c: png_color_16 = png_color_16 {
                    index: 0,
                    red: 0,
                    green: 0,
                    blue: 0,
                    gray: 0,
                };
                c.index = 0 as png_byte;
                c.red = (*(*display).background).red as png_uint_16;
                c.green = (*(*display).background).green as png_uint_16;
                c.blue = (*(*display).background).blue as png_uint_16;
                c.gray = (*(*display).background).green as png_uint_16;
                png_set_background_fixed(
                    png_ptr,
                    &raw mut c as png_const_color_16p,
                    PNG_BACKGROUND_GAMMA_SCREEN,
                    0 as ::core::ffi::c_int,
                    0 as png_fixed_point,
                );
            } else {
                do_local_compose = 1 as ::core::ffi::c_int;
                mode = PNG_ALPHA_OPTIMIZED;
            }
        } else {
            let mut filler: png_uint_32 = 0;
            let mut where_0: ::core::ffi::c_int = 0;
            if linear != 0 as ::core::ffi::c_int {
                filler = 65535 as png_uint_32;
            } else {
                filler = 255 as png_uint_32;
            }
            if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST != 0 as ::core::ffi::c_uint {
                where_0 = PNG_FILLER_BEFORE;
                change &= !PNG_FORMAT_FLAG_AFIRST;
            } else {
                where_0 = PNG_FILLER_AFTER;
            }
            png_set_add_alpha(png_ptr, filler, where_0);
        }
        change &= !PNG_FORMAT_FLAG_ALPHA;
    }
    png_set_alpha_mode_fixed(png_ptr, mode, output_gamma);
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_BGR != 0 as ::core::ffi::c_uint {
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 as ::core::ffi::c_uint {
            png_set_bgr(png_ptr);
        } else {
            format &= !PNG_FORMAT_FLAG_BGR;
        }
        change &= !PNG_FORMAT_FLAG_BGR;
    }
    if change as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST != 0 as ::core::ffi::c_uint {
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
            if do_local_background != 2 as ::core::ffi::c_int {
                png_set_swap_alpha(png_ptr);
            }
        } else {
            format &= !PNG_FORMAT_FLAG_AFIRST;
        }
        change &= !PNG_FORMAT_FLAG_AFIRST;
    }
    if linear != 0 as ::core::ffi::c_int {
        let mut le: png_uint_16 = 0x1 as png_uint_16;
        if *(&raw mut le as png_const_bytep) as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            png_set_swap(png_ptr);
        }
    }
    if change != 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"png_read_image: unsupported transformation\0" as *const u8 as png_const_charp,
        );
    }
    png_image_skip_unused_chunks(png_ptr);
    if do_local_compose == 0 as ::core::ffi::c_int && do_local_background != 2 as ::core::ffi::c_int
    {
        passes = png_set_interlace_handling(png_ptr);
    }
    png_read_update_info(png_ptr, info_ptr);
    let mut info_format: png_uint_32 = 0 as png_uint_32;
    if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
        != 0 as ::core::ffi::c_int
    {
        info_format |= PNG_FORMAT_FLAG_COLOR;
    }
    if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
        != 0 as ::core::ffi::c_int
    {
        if do_local_compose == 0 as ::core::ffi::c_int {
            if do_local_background != 2 as ::core::ffi::c_int
                || format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint
            {
                info_format |= PNG_FORMAT_FLAG_ALPHA;
            }
        }
    } else if do_local_compose != 0 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"png_image_read: alpha channel lost\0" as *const u8 as png_const_charp,
        );
    }
    if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ASSOCIATED_ALPHA != 0 as ::core::ffi::c_uint
    {
        info_format |= PNG_FORMAT_FLAG_ASSOCIATED_ALPHA;
    }
    if (*info_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        info_format |= PNG_FORMAT_FLAG_LINEAR;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BGR != 0 as ::core::ffi::c_uint {
        info_format |= PNG_FORMAT_FLAG_BGR;
    }
    if do_local_background == 2 as ::core::ffi::c_int {
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST != 0 as ::core::ffi::c_uint {
            info_format |= PNG_FORMAT_FLAG_AFIRST;
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SWAP_ALPHA
        != 0 as ::core::ffi::c_uint
        || (*png_ptr).transformations as ::core::ffi::c_uint & PNG_ADD_ALPHA
            != 0 as ::core::ffi::c_uint
            && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER
                == 0 as ::core::ffi::c_uint
    {
        if do_local_background == 2 as ::core::ffi::c_int {
            png_error(
                png_ptr,
                b"unexpected alpha swap transformation\0" as *const u8 as png_const_charp,
            );
        }
        info_format |= PNG_FORMAT_FLAG_AFIRST;
    }
    if info_format != format {
        png_error(
            png_ptr,
            b"png_read_image: invalid transformations\0" as *const u8 as png_const_charp,
        );
    }
    let mut first_row: png_voidp = (*display).buffer;
    let mut row_step: ptrdiff_t = (*display).row_stride as ptrdiff_t;
    if linear != 0 as ::core::ffi::c_int {
        row_step = (row_step as ::core::ffi::c_long * 2 as ::core::ffi::c_long) as ptrdiff_t;
    }
    if row_step < 0 as ptrdiff_t {
        let mut ptr: *mut ::core::ffi::c_char = first_row as *mut ::core::ffi::c_char;
        ptr = ptr.offset(
            (((*image).height as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                as ptrdiff_t
                * -row_step) as isize,
        );
        first_row = ptr as png_voidp;
    }
    (*display).first_row = first_row;
    (*display).row_step = row_step;
    if do_local_compose != 0 as ::core::ffi::c_int {
        let mut result: ::core::ffi::c_int = 0;
        let mut row: png_voidp = png_malloc(
            png_ptr,
            png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t,
        );
        (*display).local_row = row;
        result = png_safe_execute(
            image,
            Some(png_image_read_composite as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int),
            display as png_voidp,
        );
        (*display).local_row = NULL_0 as png_voidp;
        png_free(png_ptr, row);
        return result;
    } else if do_local_background == 2 as ::core::ffi::c_int {
        let mut result_0: ::core::ffi::c_int = 0;
        let mut row_0: png_voidp = png_malloc(
            png_ptr,
            png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t,
        );
        (*display).local_row = row_0;
        result_0 = png_safe_execute(
            image,
            Some(
                png_image_read_background as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
            ),
            display as png_voidp,
        );
        (*display).local_row = NULL_0 as png_voidp;
        png_free(png_ptr, row_0);
        return result_0;
    } else if do_local_scale != 0 as ::core::ffi::c_int {
        let mut result_1: ::core::ffi::c_int = 0;
        let mut row_1: png_voidp = png_malloc(
            png_ptr,
            png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t,
        );
        (*display).local_row = row_1;
        result_1 = png_safe_execute(
            image,
            Some(
                png_image_read_direct_scaled
                    as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
            ),
            display as png_voidp,
        );
        (*display).local_row = NULL_0 as png_voidp;
        png_free(png_ptr, row_1);
        return result_1;
    } else {
        let mut row_step_0: ptrdiff_t = (*display).row_step;
        loop {
            passes -= 1;
            if !(passes >= 0 as ::core::ffi::c_int) {
                break;
            }
            let mut y: png_uint_32 = (*image).height;
            let mut row_2: png_bytep = (*display).first_row as png_bytep;
            while y > 0 as ::core::ffi::c_uint {
                png_read_row(png_ptr, row_2, ::core::ptr::null_mut::<png_byte>());
                row_2 = row_2.offset(row_step_0 as isize);
                y = y.wrapping_sub(1);
            }
        }
        return 1 as ::core::ffi::c_int;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_finish_read(
    mut image: png_imagep,
    mut background: png_const_colorp,
    mut buffer: *mut ::core::ffi::c_void,
    mut row_stride: png_int_32,
    mut colormap: *mut ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        let mut channels: ::core::ffi::c_uint =
            if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP != 0 {
                1 as ::core::ffi::c_uint
            } else {
                ((*image).format as ::core::ffi::c_uint
                    & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
                    .wrapping_add(1 as ::core::ffi::c_uint)
            };
        if (*image).width <= (0x7fffffff as ::core::ffi::c_uint).wrapping_div(channels) {
            let mut check: png_uint_32 = 0;
            let mut png_row_stride: png_uint_32 =
                (*image).width.wrapping_mul(channels as png_uint_32);
            if row_stride == 0 as ::core::ffi::c_int {
                row_stride = png_row_stride as png_int_32;
            }
            if row_stride < 0 as ::core::ffi::c_int {
                check = (row_stride as png_uint_32).wrapping_neg();
            } else {
                check = row_stride as png_uint_32;
            }
            if !(*image).opaque.is_null() && !buffer.is_null() && check >= png_row_stride {
                if (*image).height
                    <= (0xffffffff as png_uint_32)
                        .wrapping_div(
                            (if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP
                                != 0
                            {
                                1 as png_uint_32
                            } else {
                                (((*image).format & PNG_FORMAT_FLAG_LINEAR)
                                    >> 2 as ::core::ffi::c_int)
                                    .wrapping_add(1 as png_uint_32)
                            }),
                        )
                        .wrapping_div(check)
                {
                    if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP
                        == 0 as ::core::ffi::c_uint
                        || (*image).colormap_entries > 0 as ::core::ffi::c_uint
                            && !colormap.is_null()
                    {
                        let mut result: ::core::ffi::c_int = 0;
                        let mut display: png_image_read_control = png_image_read_control {
                            image: ::core::ptr::null_mut::<C2RustUnnamed>(),
                            buffer: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                            row_stride: 0,
                            colormap: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                            background: ::core::ptr::null::<png_color>(),
                            local_row: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                            first_row: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                            row_step: 0,
                            file_encoding: 0,
                            gamma_to_linear: 0,
                            colormap_processing: 0,
                        };
                        memset(
                            &raw mut display as *mut ::core::ffi::c_void,
                            0 as ::core::ffi::c_int,
                            ::core::mem::size_of::<png_image_read_control>() as size_t,
                        );
                        display.image = image;
                        display.buffer = buffer as png_voidp;
                        display.row_stride = row_stride;
                        display.colormap = colormap as png_voidp;
                        display.background = background;
                        display.local_row = NULL_0 as png_voidp;
                        if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP
                            != 0 as ::core::ffi::c_uint
                        {
                            result = (png_safe_execute(
                                image,
                                Some(
                                    png_image_read_colormap
                                        as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                                ),
                                &raw mut display as png_voidp,
                            ) != 0
                                && png_safe_execute(
                                    image,
                                    Some(
                                        png_image_read_colormapped
                                            as unsafe extern "C" fn(
                                                png_voidp,
                                            )
                                                -> ::core::ffi::c_int,
                                    ),
                                    &raw mut display as png_voidp,
                                ) != 0) as ::core::ffi::c_int;
                        } else {
                            result = png_safe_execute(
                                image,
                                Some(
                                    png_image_read_direct
                                        as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                                ),
                                &raw mut display as png_voidp,
                            );
                        }
                        png_image_free(image);
                        return result;
                    } else {
                        return png_image_error(
                            image,
                            b"png_image_finish_read[color-map]: no color-map\0" as *const u8
                                as png_const_charp,
                        );
                    }
                } else {
                    return png_image_error(
                        image,
                        b"png_image_finish_read: image too large\0" as *const u8 as png_const_charp,
                    );
                }
            } else {
                return png_image_error(
                    image,
                    b"png_image_finish_read: invalid argument\0" as *const u8 as png_const_charp,
                );
            }
        } else {
            return png_image_error(
                image,
                b"png_image_finish_read: row_stride too large\0" as *const u8 as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_finish_read: damaged PNG_IMAGE_VERSION\0" as *const u8 as png_const_charp,
        );
    }
    return 0 as ::core::ffi::c_int;
}
pub const PNG_HAVE_IDAT: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_HAVE_IEND: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_IS_READ_STRUCT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_BGR: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_INTERLACE: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_COMPOSE: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_SWAP_ALPHA: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_RGB_TO_GRAY: ::core::ffi::c_uint = 0x600000 as ::core::ffi::c_uint;
pub const PNG_ADD_ALPHA: ::core::ffi::c_uint = 0x1000000 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZSTREAM_ENDED: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_FLAG_ROW_INIT: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_FLAG_FILLER_AFTER: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_FLAG_OPTIMIZE_ALPHA: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_FLAG_BENIGN_ERRORS_WARN: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const png_IDAT: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 73 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 68 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 65 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_IEND: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 73 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 69 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 78 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 68 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_IHDR: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 73 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 72 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 68 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 82 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_PLTE: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 80 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 69 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const PNG_GAMMA_sRGB_INVERSE: ::core::ffi::c_int = 45455 as ::core::ffi::c_int;
pub const PNG_LIB_GAMMA_MIN: ::core::ffi::c_int = 1000 as ::core::ffi::c_int;
pub const PNG_LIB_GAMMA_MAX: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
