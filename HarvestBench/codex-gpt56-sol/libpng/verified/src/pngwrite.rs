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
    fn strerror(__errnum: ::core::ffi::c_int) -> *mut ::core::ffi::c_char;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn remove(__filename: *const ::core::ffi::c_char) -> ::core::ffi::c_int;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn fflush(__stream: *mut FILE) -> ::core::ffi::c_int;
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
    fn ferror(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn gmtime(__timer: *const time_t) -> *mut tm;
    fn png_write_sig(png_ptr: png_structrp);
    fn png_write_chunk(
        png_ptr: png_structrp,
        chunk_name: png_const_bytep,
        data: png_const_bytep,
        length: size_t,
    );
    fn png_create_info_struct(png_ptr: png_const_structrp) -> png_infop;
    fn png_set_bgr(png_ptr: png_structrp);
    fn png_set_swap_alpha(png_ptr: png_structrp);
    fn png_set_invert_alpha(png_ptr: png_structrp);
    fn png_set_filler(png_ptr: png_structrp, filler: png_uint_32, flags: ::core::ffi::c_int);
    fn png_set_swap(png_ptr: png_structrp);
    fn png_set_packing(png_ptr: png_structrp);
    fn png_set_packswap(png_ptr: png_structrp);
    fn png_set_shift(png_ptr: png_structrp, true_bits: png_const_color_8p);
    fn png_set_interlace_handling(png_ptr: png_structrp) -> ::core::ffi::c_int;
    fn png_set_invert_mono(png_ptr: png_structrp);
    fn png_destroy_info_struct(png_ptr: png_const_structrp, info_ptr_ptr: png_infopp);
    fn png_set_write_fn(
        png_ptr: png_structrp,
        io_ptr: png_voidp,
        write_data_fn: png_rw_ptr,
        output_flush_fn: png_flush_ptr,
    );
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_set_benign_errors(png_ptr: png_structrp, allowed: ::core::ffi::c_int);
    fn png_get_rowbytes(png_ptr: png_const_structrp, info_ptr: png_const_inforp) -> size_t;
    fn png_set_cHRM_fixed(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        int_white_x: png_fixed_point,
        int_white_y: png_fixed_point,
        int_red_x: png_fixed_point,
        int_red_y: png_fixed_point,
        int_green_x: png_fixed_point,
        int_green_y: png_fixed_point,
        int_blue_x: png_fixed_point,
        int_blue_y: png_fixed_point,
    );
    fn png_set_gAMA_fixed(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        int_file_gamma: png_fixed_point,
    );
    fn png_set_IHDR(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        width: png_uint_32,
        height: png_uint_32,
        bit_depth: ::core::ffi::c_int,
        color_type: ::core::ffi::c_int,
        interlace_method: ::core::ffi::c_int,
        compression_method: ::core::ffi::c_int,
        filter_method: ::core::ffi::c_int,
    );
    fn png_set_PLTE(
        png_ptr: png_structrp,
        info_ptr: png_inforp,
        palette: png_const_colorp,
        num_palette: ::core::ffi::c_int,
    );
    fn png_set_sRGB(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        srgb_intent: ::core::ffi::c_int,
    );
    fn png_set_tRNS(
        png_ptr: png_structrp,
        info_ptr: png_inforp,
        trans_alpha: png_const_bytep,
        num_trans: ::core::ffi::c_int,
        trans_color: png_const_color_16p,
    );
    fn png_handle_as_unknown(
        png_ptr: png_const_structrp,
        chunk_name: png_const_bytep,
    ) -> ::core::ffi::c_int;
    fn png_image_free(image: png_imagep);
    fn __errno_location() -> *mut ::core::ffi::c_int;
    fn deflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    static png_sRGB_base: [png_uint_16; 512];
    static png_sRGB_delta: [png_byte; 512];
    fn png_free_buffer_list(png_ptr: png_structrp, list: *mut png_compression_bufferp);
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
    fn png_flush(png_ptr: png_structrp);
    fn png_write_IHDR(
        png_ptr: png_structrp,
        width: png_uint_32,
        height: png_uint_32,
        bit_depth: ::core::ffi::c_int,
        color_type: ::core::ffi::c_int,
        compression_method: ::core::ffi::c_int,
        filter_method: ::core::ffi::c_int,
        interlace_method: ::core::ffi::c_int,
    );
    fn png_write_PLTE(png_ptr: png_structrp, palette: png_const_colorp, num_pal: png_uint_32);
    fn png_compress_IDAT(
        png_ptr: png_structrp,
        row_data: png_const_bytep,
        row_data_length: png_alloc_size_t,
        flush: ::core::ffi::c_int,
    );
    fn png_write_IEND(png_ptr: png_structrp);
    fn png_write_gAMA_fixed(png_ptr: png_structrp, file_gamma: png_fixed_point);
    fn png_write_sBIT(
        png_ptr: png_structrp,
        sbit: png_const_color_8p,
        color_type: ::core::ffi::c_int,
    );
    fn png_write_cHRM_fixed(png_ptr: png_structrp, xy: *const png_xy);
    fn png_write_cICP(
        png_ptr: png_structrp,
        colour_primaries: png_byte,
        transfer_function: png_byte,
        matrix_coefficients: png_byte,
        video_full_range_flag: png_byte,
    );
    fn png_write_cLLI_fixed(png_ptr: png_structrp, maxCLL: png_uint_32, maxFALL: png_uint_32);
    fn png_write_mDCV_fixed(
        png_ptr: png_structrp,
        red_x: png_uint_16,
        red_y: png_uint_16,
        green_x: png_uint_16,
        green_y: png_uint_16,
        blue_x: png_uint_16,
        blue_y: png_uint_16,
        white_x: png_uint_16,
        white_y: png_uint_16,
        maxDL: png_uint_32,
        minDL: png_uint_32,
    );
    fn png_write_sRGB(png_ptr: png_structrp, intent: ::core::ffi::c_int);
    fn png_write_eXIf(png_ptr: png_structrp, exif: png_bytep, num_exif: ::core::ffi::c_int);
    fn png_write_iCCP(
        png_ptr: png_structrp,
        name: png_const_charp,
        profile: png_const_bytep,
        proflen: png_uint_32,
    );
    fn png_write_sPLT(png_ptr: png_structrp, palette: png_const_sPLT_tp);
    fn png_write_tRNS(
        png_ptr: png_structrp,
        trans: png_const_bytep,
        values: png_const_color_16p,
        number: ::core::ffi::c_int,
        color_type: ::core::ffi::c_int,
    );
    fn png_write_bKGD(
        png_ptr: png_structrp,
        values: png_const_color_16p,
        color_type: ::core::ffi::c_int,
    );
    fn png_write_hIST(
        png_ptr: png_structrp,
        hist: png_const_uint_16p,
        num_hist: ::core::ffi::c_int,
    );
    fn png_write_tEXt(
        png_ptr: png_structrp,
        key: png_const_charp,
        text: png_const_charp,
        text_len: size_t,
    );
    fn png_write_zTXt(
        png_ptr: png_structrp,
        key: png_const_charp,
        text: png_const_charp,
        compression: ::core::ffi::c_int,
    );
    fn png_write_iTXt(
        png_ptr: png_structrp,
        compression: ::core::ffi::c_int,
        key: png_const_charp,
        lang: png_const_charp,
        lang_key: png_const_charp,
        text: png_const_charp,
    );
    fn png_write_oFFs(
        png_ptr: png_structrp,
        x_offset: png_int_32,
        y_offset: png_int_32,
        unit_type: ::core::ffi::c_int,
    );
    fn png_write_pCAL(
        png_ptr: png_structrp,
        purpose: png_charp,
        X0: png_int_32,
        X1: png_int_32,
        type_0: ::core::ffi::c_int,
        nparams: ::core::ffi::c_int,
        units: png_const_charp,
        params: png_charpp,
    );
    fn png_write_pHYs(
        png_ptr: png_structrp,
        x_pixels_per_unit: png_uint_32,
        y_pixels_per_unit: png_uint_32,
        unit_type: ::core::ffi::c_int,
    );
    fn png_write_tIME(png_ptr: png_structrp, mod_time: png_const_timep);
    fn png_write_sCAL_s(
        png_ptr: png_structrp,
        unit: ::core::ffi::c_int,
        width: png_const_charp,
        height: png_const_charp,
    );
    fn png_write_finish_row(png_ptr: png_structrp);
    fn png_write_start_row(png_ptr: png_structrp);
    fn png_do_write_interlace(row_info: png_row_infop, row: png_bytep, pass: ::core::ffi::c_int);
    fn png_write_find_filter(png_ptr: png_structrp, row_info: png_row_infop);
    fn png_do_write_transformations(png_ptr: png_structrp, row_info: png_row_infop);
    fn png_do_check_palette_indexes(png_ptr: png_structrp, row_info: png_row_infop);
    fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp);
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
pub type __time_t = ::core::ffi::c_long;
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
pub type time_t = __time_t;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct tm {
    pub tm_sec: ::core::ffi::c_int,
    pub tm_min: ::core::ffi::c_int,
    pub tm_hour: ::core::ffi::c_int,
    pub tm_mday: ::core::ffi::c_int,
    pub tm_mon: ::core::ffi::c_int,
    pub tm_year: ::core::ffi::c_int,
    pub tm_wday: ::core::ffi::c_int,
    pub tm_yday: ::core::ffi::c_int,
    pub tm_isdst: ::core::ffi::c_int,
    pub __tm_gmtoff: ::core::ffi::c_long,
    pub __tm_zone: *const ::core::ffi::c_char,
}
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
pub type png_const_fixed_point_p = *const png_fixed_point;
pub type png_const_doublep = *const ::core::ffi::c_double;
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
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_timep = *mut png_time;
pub type png_const_timep = *const png_time;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
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
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_image_write_control {
    pub image: png_imagep,
    pub buffer: png_const_voidp,
    pub row_stride: png_int_32,
    pub colormap: png_const_voidp,
    pub convert_to_8bit: ::core::ffi::c_int,
    pub first_row: png_const_voidp,
    pub local_row: png_voidp,
    pub row_step: ptrdiff_t,
    pub memory: png_bytep,
    pub memory_bytes: png_alloc_size_t,
    pub output_bytes: png_alloc_size_t,
}
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
pub const PNG_TEXT_COMPRESSION_NONE_WR: ::core::ffi::c_int = -(3 as ::core::ffi::c_int);
pub const PNG_TEXT_COMPRESSION_zTXt_WR: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const PNG_TEXT_COMPRESSION_NONE: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_TEXT_COMPRESSION_zTXt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_HAVE_IHDR: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_HAVE_PLTE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_AFTER_IDAT: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_FP_1: ::core::ffi::c_int = 100000 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COMPRESSION_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_INTRAPIXEL_DIFFERENCING: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const PNG_INTERLACE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_sRGB_INTENT_PERCEPTUAL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_MAX_PALETTE_LENGTH: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_INFO_gAMA: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_INFO_sBIT: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_INFO_cHRM: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_INFO_PLTE: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_INFO_tRNS: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_INFO_bKGD: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_INFO_hIST: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_INFO_pHYs: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_INFO_oFFs: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_INFO_tIME: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_INFO_pCAL: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_INFO_sRGB: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PNG_INFO_iCCP: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_INFO_sPLT: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_INFO_sCAL: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_INFO_IDAT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_INFO_eXIf: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_INFO_cICP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_INFO_cLLI: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const PNG_INFO_mDCV: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PNG_TRANSFORM_PACKING: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_PACKSWAP: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_INVERT_MONO: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SHIFT: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_BGR: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SWAP_ALPHA: ::core::ffi::c_int = 0x100 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_SWAP_ENDIAN: ::core::ffi::c_int = 0x200 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_INVERT_ALPHA: ::core::ffi::c_int = 0x400 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_STRIP_FILLER: ::core::ffi::c_int = 0x800 as ::core::ffi::c_int;
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: ::core::ffi::c_int = PNG_TRANSFORM_STRIP_FILLER;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: ::core::ffi::c_int = 0x1000 as ::core::ffi::c_int;
pub const PNG_FLAG_MNG_FILTER_64: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_GAMMA_LINEAR: ::core::ffi::c_int = PNG_FP_1;
pub const PNG_FILLER_BEFORE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILLER_AFTER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_NO_FILTERS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_NONE: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_FILTER_SUB: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const PNG_FILTER_UP: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const PNG_FILTER_AVG: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const PNG_FILTER_PAETH: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const PNG_FAST_FILTERS: ::core::ffi::c_int = PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP;
pub const PNG_ALL_FILTERS: ::core::ffi::c_int =
    PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;
pub const PNG_FILTER_VALUE_NONE: ::core::ffi::c_int = 0;
pub const PNG_FILTER_VALUE_SUB: ::core::ffi::c_int = 1;
pub const PNG_FILTER_VALUE_UP: ::core::ffi::c_int = 2;
pub const PNG_FILTER_VALUE_AVG: ::core::ffi::c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: ::core::ffi::c_int = 4;
pub const PNG_HANDLE_CHUNK_AS_DEFAULT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_HANDLE_CHUNK_NEVER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_HANDLE_CHUNK_ALWAYS: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PNG_IMAGE_VERSION: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_FORMAT_FLAG_ALPHA: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_COLOR: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_LINEAR: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_COLORMAP: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_BGR: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_FORMAT_FLAG_AFIRST: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_IMAGE_FLAG_FAST: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_TEXT_Z_DEFAULT_COMPRESSION: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_TEXT_Z_DEFAULT_STRATEGY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_ZBUF_SIZE: ::core::ffi::c_int = 8192 as ::core::ffi::c_int;
pub const PNG_Z_DEFAULT_COMPRESSION: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_Z_DEFAULT_STRATEGY: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const Z_SYNC_FLUSH: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
unsafe extern "C" fn write_unknown_chunks(
    mut png_ptr: png_structrp,
    mut info_ptr: png_const_inforp,
    mut where_0: ::core::ffi::c_uint,
) {
    if (*info_ptr).unknown_chunks_num != 0 as ::core::ffi::c_int {
        let mut up: png_const_unknown_chunkp = ::core::ptr::null::<png_unknown_chunk>();
        up = (*info_ptr).unknown_chunks as png_const_unknown_chunkp;
        while up
            < (*info_ptr)
                .unknown_chunks
                .offset((*info_ptr).unknown_chunks_num as isize)
                as png_const_unknown_chunkp
        {
            if (*up).location as ::core::ffi::c_uint & where_0 != 0 as ::core::ffi::c_uint {
                let mut keep: ::core::ffi::c_int =
                    png_handle_as_unknown(png_ptr, &raw const (*up).name as png_const_bytep);
                if keep != PNG_HANDLE_CHUNK_NEVER
                    && ((*up).name[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                        & 0x20 as ::core::ffi::c_int
                        != 0
                        || keep == PNG_HANDLE_CHUNK_ALWAYS
                        || keep == PNG_HANDLE_CHUNK_AS_DEFAULT
                            && (*png_ptr).unknown_default == PNG_HANDLE_CHUNK_ALWAYS)
                {
                    if (*up).size == 0 as size_t {
                        png_warning(
                            png_ptr,
                            b"Writing zero-length unknown chunk\0" as *const u8 as png_const_charp,
                        );
                    }
                    png_write_chunk(
                        png_ptr,
                        &raw const (*up).name as png_const_bytep,
                        (*up).data as png_const_bytep,
                        (*up).size,
                    );
                }
            }
            up = up.offset(1);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info_before_PLTE(
    mut png_ptr: png_structrp,
    mut info_ptr: png_const_inforp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_INFO_BEFORE_PLTE
        == 0 as ::core::ffi::c_uint
    {
        png_write_sig(png_ptr);
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PNG_SIGNATURE
            != 0 as ::core::ffi::c_uint
            && (*png_ptr).mng_features_permitted != 0 as ::core::ffi::c_uint
        {
            png_warning(
                png_ptr,
                b"MNG features are not allowed in a PNG datastream\0" as *const u8
                    as png_const_charp,
            );
            (*png_ptr).mng_features_permitted = 0 as png_uint_32;
        }
        png_write_IHDR(
            png_ptr,
            (*info_ptr).width,
            (*info_ptr).height,
            (*info_ptr).bit_depth as ::core::ffi::c_int,
            (*info_ptr).color_type as ::core::ffi::c_int,
            (*info_ptr).compression_type as ::core::ffi::c_int,
            (*info_ptr).filter_type as ::core::ffi::c_int,
            (*info_ptr).interlace_type as ::core::ffi::c_int,
        );
        write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_IHDR as ::core::ffi::c_uint);
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sBIT != 0 as ::core::ffi::c_uint {
            png_write_sBIT(
                png_ptr,
                &raw const (*info_ptr).sig_bit,
                (*info_ptr).color_type as ::core::ffi::c_int,
            );
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cLLI != 0 as ::core::ffi::c_uint {
            png_write_cLLI_fixed(png_ptr, (*info_ptr).maxCLL, (*info_ptr).maxFALL);
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_mDCV != 0 as ::core::ffi::c_uint {
            png_write_mDCV_fixed(
                png_ptr,
                (*info_ptr).mastering_red_x,
                (*info_ptr).mastering_red_y,
                (*info_ptr).mastering_green_x,
                (*info_ptr).mastering_green_y,
                (*info_ptr).mastering_blue_x,
                (*info_ptr).mastering_blue_y,
                (*info_ptr).mastering_white_x,
                (*info_ptr).mastering_white_y,
                (*info_ptr).mastering_maxDL,
                (*info_ptr).mastering_minDL,
            );
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cICP != 0 as ::core::ffi::c_uint {
            png_write_cICP(
                png_ptr,
                (*info_ptr).cicp_colour_primaries,
                (*info_ptr).cicp_transfer_function,
                (*info_ptr).cicp_matrix_coefficients,
                (*info_ptr).cicp_video_full_range_flag,
            );
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_iCCP != 0 as ::core::ffi::c_uint {
            png_write_iCCP(
                png_ptr,
                (*info_ptr).iccp_name as png_const_charp,
                (*info_ptr).iccp_profile as png_const_bytep,
                (*info_ptr).iccp_proflen,
            );
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sRGB != 0 as ::core::ffi::c_uint {
            png_write_sRGB(png_ptr, (*info_ptr).rendering_intent);
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_gAMA != 0 as ::core::ffi::c_uint {
            png_write_gAMA_fixed(png_ptr, (*info_ptr).gamma);
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cHRM != 0 as ::core::ffi::c_uint {
            png_write_cHRM_fixed(png_ptr, &raw const (*info_ptr).cHRM);
        }
        (*png_ptr).mode |= PNG_WROTE_INFO_BEFORE_PLTE;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_info(mut png_ptr: png_structrp, mut info_ptr: png_const_inforp) {
    let mut i: ::core::ffi::c_int = 0;
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    png_write_info_before_PLTE(png_ptr, info_ptr);
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_PLTE != 0 as ::core::ffi::c_uint {
        png_write_PLTE(
            png_ptr,
            (*info_ptr).palette as png_const_colorp,
            (*info_ptr).num_palette as png_uint_32,
        );
    } else if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        png_error(
            png_ptr,
            b"Valid palette required for paletted images\0" as *const u8 as png_const_charp,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_tRNS != 0 as ::core::ffi::c_uint {
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INVERT_ALPHA
            != 0 as ::core::ffi::c_uint
            && (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        {
            let mut j: ::core::ffi::c_int = 0;
            let mut jend: ::core::ffi::c_int = 0;
            jend = (*info_ptr).num_trans as ::core::ffi::c_int;
            if jend > PNG_MAX_PALETTE_LENGTH {
                jend = PNG_MAX_PALETTE_LENGTH;
            }
            j = 0 as ::core::ffi::c_int;
            while j < jend {
                *(*info_ptr).trans_alpha.offset(j as isize) = (255 as ::core::ffi::c_int
                    - *(*info_ptr).trans_alpha.offset(j as isize) as ::core::ffi::c_int)
                    as png_byte;
                j += 1;
            }
        }
        png_write_tRNS(
            png_ptr,
            (*info_ptr).trans_alpha as png_const_bytep,
            &raw const (*info_ptr).trans_color,
            (*info_ptr).num_trans as ::core::ffi::c_int,
            (*info_ptr).color_type as ::core::ffi::c_int,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_bKGD != 0 as ::core::ffi::c_uint {
        png_write_bKGD(
            png_ptr,
            &raw const (*info_ptr).background,
            (*info_ptr).color_type as ::core::ffi::c_int,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_eXIf != 0 as ::core::ffi::c_uint {
        png_write_eXIf(
            png_ptr,
            (*info_ptr).exif,
            (*info_ptr).num_exif as ::core::ffi::c_int,
        );
        (*png_ptr).mode |= PNG_WROTE_eXIf;
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_hIST != 0 as ::core::ffi::c_uint {
        png_write_hIST(
            png_ptr,
            (*info_ptr).hist as png_const_uint_16p,
            (*info_ptr).num_palette as ::core::ffi::c_int,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint {
        png_write_oFFs(
            png_ptr,
            (*info_ptr).x_offset,
            (*info_ptr).y_offset,
            (*info_ptr).offset_unit_type as ::core::ffi::c_int,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pCAL != 0 as ::core::ffi::c_uint {
        png_write_pCAL(
            png_ptr,
            (*info_ptr).pcal_purpose,
            (*info_ptr).pcal_X0,
            (*info_ptr).pcal_X1,
            (*info_ptr).pcal_type as ::core::ffi::c_int,
            (*info_ptr).pcal_nparams as ::core::ffi::c_int,
            (*info_ptr).pcal_units as png_const_charp,
            (*info_ptr).pcal_params,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sCAL != 0 as ::core::ffi::c_uint {
        png_write_sCAL_s(
            png_ptr,
            (*info_ptr).scal_unit as ::core::ffi::c_int,
            (*info_ptr).scal_s_width as png_const_charp,
            (*info_ptr).scal_s_height as png_const_charp,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint {
        png_write_pHYs(
            png_ptr,
            (*info_ptr).x_pixels_per_unit,
            (*info_ptr).y_pixels_per_unit,
            (*info_ptr).phys_unit_type as ::core::ffi::c_int,
        );
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_tIME != 0 as ::core::ffi::c_uint {
        png_write_tIME(png_ptr, &raw const (*info_ptr).mod_time);
        (*png_ptr).mode |= PNG_WROTE_tIME;
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sPLT != 0 as ::core::ffi::c_uint {
        i = 0 as ::core::ffi::c_int;
        while i < (*info_ptr).splt_palettes_num {
            png_write_sPLT(
                png_ptr,
                (*info_ptr).splt_palettes.offset(i as isize) as png_const_sPLT_tp,
            );
            i += 1;
        }
    }
    i = 0 as ::core::ffi::c_int;
    while i < (*info_ptr).num_text {
        if (*(*info_ptr).text.offset(i as isize)).compression > 0 as ::core::ffi::c_int {
            png_write_iTXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).compression,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).lang as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).lang_key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
            );
            if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_NONE {
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            } else {
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            }
        } else if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_zTXt {
            png_write_zTXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).compression,
            );
            (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
        } else if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_NONE {
            png_write_tEXt(
                png_ptr,
                (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                0 as size_t,
            );
            (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_NONE_WR;
        }
        i += 1;
    }
    write_unknown_chunks(png_ptr, info_ptr, PNG_HAVE_PLTE as ::core::ffi::c_uint);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_end(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"No IDATs written into file\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= (*png_ptr).num_palette as ::core::ffi::c_int
    {
        png_benign_error(
            png_ptr,
            b"Wrote palette index exceeding num_palette\0" as *const u8 as png_const_charp,
        );
    }
    if !info_ptr.is_null() {
        let mut i: ::core::ffi::c_int = 0;
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_tIME != 0 as ::core::ffi::c_uint
            && (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_tIME == 0 as ::core::ffi::c_uint
        {
            png_write_tIME(png_ptr, &raw mut (*info_ptr).mod_time as png_const_timep);
        }
        i = 0 as ::core::ffi::c_int;
        while i < (*info_ptr).num_text {
            if (*(*info_ptr).text.offset(i as isize)).compression > 0 as ::core::ffi::c_int {
                png_write_iTXt(
                    png_ptr,
                    (*(*info_ptr).text.offset(i as isize)).compression,
                    (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).lang as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).lang_key as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                );
                if (*(*info_ptr).text.offset(i as isize)).compression == PNG_TEXT_COMPRESSION_NONE {
                    (*(*info_ptr).text.offset(i as isize)).compression =
                        PNG_TEXT_COMPRESSION_NONE_WR;
                } else {
                    (*(*info_ptr).text.offset(i as isize)).compression =
                        PNG_TEXT_COMPRESSION_zTXt_WR;
                }
            } else if (*(*info_ptr).text.offset(i as isize)).compression
                >= PNG_TEXT_COMPRESSION_zTXt
            {
                png_write_zTXt(
                    png_ptr,
                    (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).compression,
                );
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_zTXt_WR;
            } else if (*(*info_ptr).text.offset(i as isize)).compression
                == PNG_TEXT_COMPRESSION_NONE
            {
                png_write_tEXt(
                    png_ptr,
                    (*(*info_ptr).text.offset(i as isize)).key as png_const_charp,
                    (*(*info_ptr).text.offset(i as isize)).text as png_const_charp,
                    0 as size_t,
                );
                (*(*info_ptr).text.offset(i as isize)).compression = PNG_TEXT_COMPRESSION_NONE_WR;
            }
            i += 1;
        }
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_eXIf != 0 as ::core::ffi::c_uint
            && (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_eXIf == 0 as ::core::ffi::c_uint
        {
            png_write_eXIf(
                png_ptr,
                (*info_ptr).exif,
                (*info_ptr).num_exif as ::core::ffi::c_int,
            );
        }
        write_unknown_chunks(png_ptr, info_ptr, PNG_AFTER_IDAT as ::core::ffi::c_uint);
    }
    (*png_ptr).mode |= PNG_AFTER_IDAT as ::core::ffi::c_uint;
    png_write_IEND(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_struct_tm(mut ptime: png_timep, mut ttime: *const tm) {
    (*ptime).year = (1900 as ::core::ffi::c_int + (*ttime).tm_year) as png_uint_16;
    (*ptime).month = ((*ttime).tm_mon + 1 as ::core::ffi::c_int) as png_byte;
    (*ptime).day = (*ttime).tm_mday as png_byte;
    (*ptime).hour = (*ttime).tm_hour as png_byte;
    (*ptime).minute = (*ttime).tm_min as png_byte;
    (*ptime).second = (*ttime).tm_sec as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_from_time_t(mut ptime: png_timep, mut ttime: time_t) {
    let mut tbuf: *mut tm = ::core::ptr::null_mut::<tm>();
    tbuf = gmtime(&raw mut ttime);
    if tbuf.is_null() {
        memset(
            ptime as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<png_time>() as size_t,
        );
        return;
    }
    png_convert_from_struct_tm(ptime, tbuf);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_write_struct(
    mut user_png_ver: png_const_charp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warn_fn: png_error_ptr,
) -> png_structp {
    return png_create_write_struct_2(
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
pub unsafe extern "C" fn png_create_write_struct_2(
    mut user_png_ver: png_const_charp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warn_fn: png_error_ptr,
    mut mem_ptr: png_voidp,
    mut malloc_fn: png_malloc_ptr,
    mut free_fn: png_free_ptr,
) -> png_structp {
    let mut png_ptr: png_structrp = png_create_png_struct(
        user_png_ver,
        error_ptr,
        error_fn,
        warn_fn,
        mem_ptr,
        malloc_fn,
        free_fn,
    ) as png_structrp;
    if !png_ptr.is_null() {
        (*png_ptr).zbuffer_size = PNG_ZBUF_SIZE as uInt;
        (*png_ptr).zlib_strategy = PNG_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_level = PNG_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_mem_level = 8 as ::core::ffi::c_int;
        (*png_ptr).zlib_window_bits = 15 as ::core::ffi::c_int;
        (*png_ptr).zlib_method = 8 as ::core::ffi::c_int;
        (*png_ptr).zlib_text_strategy = PNG_TEXT_Z_DEFAULT_STRATEGY;
        (*png_ptr).zlib_text_level = PNG_TEXT_Z_DEFAULT_COMPRESSION;
        (*png_ptr).zlib_text_mem_level = 8 as ::core::ffi::c_int;
        (*png_ptr).zlib_text_window_bits = 15 as ::core::ffi::c_int;
        (*png_ptr).zlib_text_method = 8 as ::core::ffi::c_int;
        png_set_write_fn(png_ptr, NULL_0, None, None);
    }
    return png_ptr as png_structp;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_rows(
    mut png_ptr: png_structrp,
    mut row: png_bytepp,
    mut num_rows: png_uint_32,
) {
    let mut i: png_uint_32 = 0;
    let mut rp: png_bytepp = ::core::ptr::null_mut::<*mut png_byte>();
    if png_ptr.is_null() {
        return;
    }
    i = 0 as png_uint_32;
    rp = row;
    while i < num_rows {
        png_write_row(png_ptr, *rp as png_const_bytep);
        i = i.wrapping_add(1);
        rp = rp.offset(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_image(mut png_ptr: png_structrp, mut image: png_bytepp) {
    let mut i: png_uint_32 = 0;
    let mut pass: ::core::ffi::c_int = 0;
    let mut num_pass: ::core::ffi::c_int = 0;
    let mut rp: png_bytepp = ::core::ptr::null_mut::<*mut png_byte>();
    if png_ptr.is_null() {
        return;
    }
    num_pass = png_set_interlace_handling(png_ptr);
    pass = 0 as ::core::ffi::c_int;
    while pass < num_pass {
        i = 0 as png_uint_32;
        rp = image;
        while i < (*png_ptr).height {
            png_write_row(png_ptr, *rp as png_const_bytep);
            i = i.wrapping_add(1);
            rp = rp.offset(1);
        }
        pass += 1;
    }
}
unsafe extern "C" fn png_do_write_intrapixel(mut row_info: png_row_infop, mut row: png_bytep) {
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
                *rp = (*rp as ::core::ffi::c_int
                    - *rp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_byte;
                *rp.offset(2 as ::core::ffi::c_int as isize) =
                    (*rp.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                        - *rp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
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
                let mut red: png_uint_32 = (s0.wrapping_sub(s1) as ::core::ffi::c_long
                    & 0xffff as ::core::ffi::c_long)
                    as png_uint_32;
                let mut blue: png_uint_32 = (s2.wrapping_sub(s1) as ::core::ffi::c_long
                    & 0xffff as ::core::ffi::c_long)
                    as png_uint_32;
                *rp_0 = (red >> 8 as ::core::ffi::c_int) as png_byte;
                *rp_0.offset(1 as ::core::ffi::c_int as isize) = red as png_byte;
                *rp_0.offset(4 as ::core::ffi::c_int as isize) =
                    (blue >> 8 as ::core::ffi::c_int) as png_byte;
                *rp_0.offset(5 as ::core::ffi::c_int as isize) = blue as png_byte;
                i_0 = i_0.wrapping_add(1);
                rp_0 = rp_0.offset(bytes_per_pixel as isize);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_row(mut png_ptr: png_structrp, mut row: png_const_bytep) {
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
    if (*png_ptr).row_number == 0 as ::core::ffi::c_uint
        && (*png_ptr).pass as ::core::ffi::c_int == 0 as ::core::ffi::c_int
    {
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_INFO_BEFORE_PLTE
            == 0 as ::core::ffi::c_uint
        {
            png_error(
                png_ptr,
                b"png_write_info was never called before png_write_row\0" as *const u8
                    as png_const_charp,
            );
        }
        png_write_start_row(png_ptr);
    }
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            != 0 as ::core::ffi::c_uint
    {
        match (*png_ptr).pass as ::core::ffi::c_int {
            0 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint
                    != 0 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            1 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint
                    != 0 as ::core::ffi::c_uint
                    || (*png_ptr).width < 5 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            2 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint
                    != 4 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            3 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x3 as ::core::ffi::c_uint
                    != 0 as ::core::ffi::c_uint
                    || (*png_ptr).width < 3 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            4 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x3 as ::core::ffi::c_uint
                    != 2 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            5 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x1 as ::core::ffi::c_uint
                    != 0 as ::core::ffi::c_uint
                    || (*png_ptr).width < 2 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            6 => {
                if (*png_ptr).row_number as ::core::ffi::c_uint & 0x1 as ::core::ffi::c_uint
                    == 0 as ::core::ffi::c_uint
                {
                    png_write_finish_row(png_ptr);
                    return;
                }
            }
            _ => {}
        }
    }
    row_info.color_type = (*png_ptr).color_type;
    row_info.width = (*png_ptr).usr_width;
    row_info.channels = (*png_ptr).usr_channels;
    row_info.bit_depth = (*png_ptr).usr_bit_depth;
    row_info.pixel_depth = (row_info.bit_depth as ::core::ffi::c_int
        * row_info.channels as ::core::ffi::c_int) as png_byte;
    row_info.rowbytes = if row_info.pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
        (row_info.width as size_t)
            .wrapping_mul(row_info.pixel_depth as size_t >> 3 as ::core::ffi::c_int)
    } else {
        (row_info.width as size_t)
            .wrapping_mul(row_info.pixel_depth as size_t)
            .wrapping_add(7 as size_t)
            >> 3 as ::core::ffi::c_int
    };
    memcpy(
        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        row as *const ::core::ffi::c_void,
        row_info.rowbytes,
    );
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0
        && ((*png_ptr).pass as ::core::ffi::c_int) < 6 as ::core::ffi::c_int
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            != 0 as ::core::ffi::c_uint
    {
        png_do_write_interlace(
            &raw mut row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            (*png_ptr).pass as ::core::ffi::c_int,
        );
        if row_info.width == 0 as ::core::ffi::c_uint {
            png_write_finish_row(png_ptr);
            return;
        }
    }
    if (*png_ptr).transformations != 0 as ::core::ffi::c_uint {
        png_do_write_transformations(png_ptr, &raw mut row_info);
    }
    if row_info.pixel_depth as ::core::ffi::c_int != (*png_ptr).pixel_depth as ::core::ffi::c_int
        || row_info.pixel_depth as ::core::ffi::c_int
            != (*png_ptr).transformed_pixel_depth as ::core::ffi::c_int
    {
        png_error(
            png_ptr,
            b"internal write transform logic error\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
        & PNG_FLAG_MNG_FILTER_64 as ::core::ffi::c_uint
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).filter_type as ::core::ffi::c_int == PNG_INTRAPIXEL_DIFFERENCING
    {
        png_do_write_intrapixel(
            &raw mut row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if row_info.color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= 0 as ::core::ffi::c_int
    {
        png_do_check_palette_indexes(png_ptr, &raw mut row_info);
    }
    png_write_find_filter(png_ptr, &raw mut row_info);
    if (*png_ptr).write_row_fn.is_some() {
        Some((*png_ptr).write_row_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as png_structp,
            (*png_ptr).row_number,
            (*png_ptr).pass as ::core::ffi::c_int,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_flush(mut png_ptr: png_structrp, mut nrows: ::core::ffi::c_int) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).flush_dist = (if nrows < 0 as ::core::ffi::c_int {
        0 as ::core::ffi::c_uint
    } else {
        nrows as ::core::ffi::c_uint
    }) as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_flush(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).row_number >= (*png_ptr).num_rows {
        return;
    }
    png_compress_IDAT(
        png_ptr,
        ::core::ptr::null::<png_byte>(),
        0 as png_alloc_size_t,
        Z_SYNC_FLUSH,
    );
    (*png_ptr).flush_rows = 0 as png_uint_32;
    png_flush(png_ptr);
}
unsafe extern "C" fn png_write_destroy(mut png_ptr: png_structrp) {
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_INITIALIZED
        != 0 as ::core::ffi::c_uint
    {
        deflateEnd(&raw mut (*png_ptr).zstream);
    }
    png_free_buffer_list(png_ptr, &raw mut (*png_ptr).zbuffer_list);
    png_free(png_ptr, (*png_ptr).row_buf as png_voidp);
    (*png_ptr).row_buf = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).prev_row as png_voidp);
    png_free(png_ptr, (*png_ptr).try_row as png_voidp);
    png_free(png_ptr, (*png_ptr).tst_row as png_voidp);
    (*png_ptr).prev_row = ::core::ptr::null_mut::<png_byte>();
    (*png_ptr).try_row = ::core::ptr::null_mut::<png_byte>();
    (*png_ptr).tst_row = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
    (*png_ptr).chunk_list = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
    (*png_ptr).trans_alpha = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = ::core::ptr::null_mut::<png_color>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_write_struct(
    mut png_ptr_ptr: png_structpp,
    mut info_ptr_ptr: png_infopp,
) {
    if !png_ptr_ptr.is_null() {
        let mut png_ptr: png_structrp = *png_ptr_ptr;
        if !png_ptr.is_null() {
            png_destroy_info_struct(png_ptr, info_ptr_ptr);
            *png_ptr_ptr = ::core::ptr::null_mut::<png_struct>();
            png_write_destroy(png_ptr);
            png_destroy_png_struct(png_ptr);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter(
    mut png_ptr: png_structrp,
    mut method: ::core::ffi::c_int,
    mut filters: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
        & PNG_FLAG_MNG_FILTER_64 as ::core::ffi::c_uint
        != 0 as ::core::ffi::c_uint
        && method == PNG_INTRAPIXEL_DIFFERENCING
    {
        method = PNG_FILTER_TYPE_BASE;
    }
    if method == PNG_FILTER_TYPE_BASE {
        let mut current_block_11: u64;
        match filters & (PNG_ALL_FILTERS | 0x7 as ::core::ffi::c_int) {
            5 | 6 | 7 => {
                png_app_error(
                    png_ptr,
                    b"Unknown row filter for method 0\0" as *const u8 as png_const_charp,
                );
                current_block_11 = 13864189353457988289;
            }
            PNG_FILTER_VALUE_NONE => {
                current_block_11 = 13864189353457988289;
            }
            PNG_FILTER_VALUE_SUB => {
                (*png_ptr).do_filter = PNG_FILTER_SUB as png_byte;
                current_block_11 = 7976072742316086414;
            }
            PNG_FILTER_VALUE_UP => {
                (*png_ptr).do_filter = PNG_FILTER_UP as png_byte;
                current_block_11 = 7976072742316086414;
            }
            PNG_FILTER_VALUE_AVG => {
                (*png_ptr).do_filter = PNG_FILTER_AVG as png_byte;
                current_block_11 = 7976072742316086414;
            }
            PNG_FILTER_VALUE_PAETH => {
                (*png_ptr).do_filter = PNG_FILTER_PAETH as png_byte;
                current_block_11 = 7976072742316086414;
            }
            _ => {
                (*png_ptr).do_filter = filters as png_byte;
                current_block_11 = 7976072742316086414;
            }
        }
        match current_block_11 {
            13864189353457988289 => {
                (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
            }
            _ => {}
        }
        if !(*png_ptr).row_buf.is_null() {
            let mut num_filters: ::core::ffi::c_int = 0;
            let mut buf_size: png_alloc_size_t = 0;
            if (*png_ptr).height == 1 as ::core::ffi::c_uint {
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }
            if (*png_ptr).width == 1 as ::core::ffi::c_uint {
                filters &= !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }
            if filters & (PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)
                != 0 as ::core::ffi::c_int
                && (*png_ptr).prev_row.is_null()
            {
                png_app_warning(
                    png_ptr,
                    b"png_set_filter: UP/AVG/PAETH cannot be added after start\0" as *const u8
                        as png_const_charp,
                );
                filters &= !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH);
            }
            num_filters = 0 as ::core::ffi::c_int;
            if filters & PNG_FILTER_SUB != 0 {
                num_filters += 1;
            }
            if filters & PNG_FILTER_UP != 0 {
                num_filters += 1;
            }
            if filters & PNG_FILTER_AVG != 0 {
                num_filters += 1;
            }
            if filters & PNG_FILTER_PAETH != 0 {
                num_filters += 1;
            }
            buf_size = (if (*png_ptr).usr_channels as ::core::ffi::c_int
                * (*png_ptr).usr_bit_depth as ::core::ffi::c_int
                >= 8 as ::core::ffi::c_int
            {
                ((*png_ptr).width as size_t).wrapping_mul(
                    ((*png_ptr).usr_channels as ::core::ffi::c_int
                        * (*png_ptr).usr_bit_depth as ::core::ffi::c_int)
                        as size_t
                        >> 3 as ::core::ffi::c_int,
                )
            } else {
                ((*png_ptr).width as size_t)
                    .wrapping_mul(
                        ((*png_ptr).usr_channels as ::core::ffi::c_int
                            * (*png_ptr).usr_bit_depth as ::core::ffi::c_int)
                            as size_t,
                    )
                    .wrapping_add(7 as size_t)
                    >> 3 as ::core::ffi::c_int
            })
            .wrapping_add(1 as size_t) as png_alloc_size_t;
            if (*png_ptr).try_row.is_null() {
                (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;
            }
            if num_filters > 1 as ::core::ffi::c_int {
                if (*png_ptr).tst_row.is_null() {
                    (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
                }
            }
        }
        (*png_ptr).do_filter = filters as png_byte;
    } else {
        png_error(
            png_ptr,
            b"Unknown custom filter method\0" as *const u8 as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics(
    mut png_ptr: png_structrp,
    mut heuristic_method: ::core::ffi::c_int,
    mut num_weights: ::core::ffi::c_int,
    mut filter_weights: png_const_doublep,
    mut filter_costs: png_const_doublep,
) {
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filter_heuristics_fixed(
    mut png_ptr: png_structrp,
    mut heuristic_method: ::core::ffi::c_int,
    mut num_weights: ::core::ffi::c_int,
    mut filter_weights: png_const_fixed_point_p,
    mut filter_costs: png_const_fixed_point_p,
) {
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_level(
    mut png_ptr: png_structrp,
    mut level: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_level = level;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_mem_level(
    mut png_ptr: png_structrp,
    mut mem_level: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_mem_level = mem_level;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_strategy(
    mut png_ptr: png_structrp,
    mut strategy: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).flags |= PNG_FLAG_ZLIB_CUSTOM_STRATEGY;
    (*png_ptr).zlib_strategy = strategy;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_window_bits(
    mut png_ptr: png_structrp,
    mut window_bits: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if window_bits > 15 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression windows <= 32k supported by PNG\0" as *const u8 as png_const_charp,
        );
        window_bits = 15 as ::core::ffi::c_int;
    } else if window_bits < 8 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression windows >= 256 supported by PNG\0" as *const u8 as png_const_charp,
        );
        window_bits = 8 as ::core::ffi::c_int;
    }
    (*png_ptr).zlib_window_bits = window_bits;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_method(
    mut png_ptr: png_structrp,
    mut method: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if method != 8 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression method 8 is supported by PNG\0" as *const u8 as png_const_charp,
        );
    }
    (*png_ptr).zlib_method = method;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_level(
    mut png_ptr: png_structrp,
    mut level: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_level = level;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_mem_level(
    mut png_ptr: png_structrp,
    mut mem_level: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_mem_level = mem_level;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_strategy(
    mut png_ptr: png_structrp,
    mut strategy: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).zlib_text_strategy = strategy;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_window_bits(
    mut png_ptr: png_structrp,
    mut window_bits: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if window_bits > 15 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression windows <= 32k supported by PNG\0" as *const u8 as png_const_charp,
        );
        window_bits = 15 as ::core::ffi::c_int;
    } else if window_bits < 8 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression windows >= 256 supported by PNG\0" as *const u8 as png_const_charp,
        );
        window_bits = 8 as ::core::ffi::c_int;
    }
    (*png_ptr).zlib_text_window_bits = window_bits;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_compression_method(
    mut png_ptr: png_structrp,
    mut method: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if method != 8 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Only compression method 8 is supported by PNG\0" as *const u8 as png_const_charp,
        );
    }
    (*png_ptr).zlib_text_method = method;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_status_fn(
    mut png_ptr: png_structrp,
    mut write_row_fn: png_write_status_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).write_row_fn = write_row_fn;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_write_user_transform_fn(
    mut png_ptr: png_structrp,
    mut write_user_transform_fn: png_user_transform_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).write_user_transform_fn = write_user_transform_fn;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_png(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
    mut transforms: ::core::ffi::c_int,
    mut params: png_voidp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_IDAT == 0 as ::core::ffi::c_uint {
        png_app_error(
            png_ptr,
            b"no rows for png_write_image to write\0" as *const u8 as png_const_charp,
        );
        return;
    }
    png_write_info(png_ptr, info_ptr);
    if transforms & PNG_TRANSFORM_INVERT_MONO != 0 as ::core::ffi::c_int {
        png_set_invert_mono(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SHIFT != 0 as ::core::ffi::c_int {
        if (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sBIT != 0 as ::core::ffi::c_uint {
            png_set_shift(png_ptr, &raw mut (*info_ptr).sig_bit as png_const_color_8p);
        }
    }
    if transforms & PNG_TRANSFORM_PACKING != 0 as ::core::ffi::c_int {
        png_set_packing(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SWAP_ALPHA != 0 as ::core::ffi::c_int {
        png_set_swap_alpha(png_ptr);
    }
    if transforms & (PNG_TRANSFORM_STRIP_FILLER_AFTER | PNG_TRANSFORM_STRIP_FILLER_BEFORE)
        != 0 as ::core::ffi::c_int
    {
        if transforms & PNG_TRANSFORM_STRIP_FILLER_AFTER != 0 as ::core::ffi::c_int {
            if transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE != 0 as ::core::ffi::c_int {
                png_app_error(
                    png_ptr,
                    b"PNG_TRANSFORM_STRIP_FILLER: BEFORE+AFTER not supported\0" as *const u8
                        as png_const_charp,
                );
            }
            png_set_filler(png_ptr, 0 as png_uint_32, PNG_FILLER_AFTER);
        } else if transforms & PNG_TRANSFORM_STRIP_FILLER_BEFORE != 0 as ::core::ffi::c_int {
            png_set_filler(png_ptr, 0 as png_uint_32, PNG_FILLER_BEFORE);
        }
    }
    if transforms & PNG_TRANSFORM_BGR != 0 as ::core::ffi::c_int {
        png_set_bgr(png_ptr);
    }
    if transforms & PNG_TRANSFORM_SWAP_ENDIAN != 0 as ::core::ffi::c_int {
        png_set_swap(png_ptr);
    }
    if transforms & PNG_TRANSFORM_PACKSWAP != 0 as ::core::ffi::c_int {
        png_set_packswap(png_ptr);
    }
    if transforms & PNG_TRANSFORM_INVERT_ALPHA != 0 as ::core::ffi::c_int {
        png_set_invert_alpha(png_ptr);
    }
    png_write_image(png_ptr, (*info_ptr).row_pointers);
    png_write_end(png_ptr, info_ptr);
}
unsafe extern "C" fn png_image_write_init(mut image: png_imagep) -> ::core::ffi::c_int {
    let mut png_ptr: png_structp = png_create_write_struct(
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
                (*control).set_for_write(1 as ::core::ffi::c_uint as ::core::ffi::c_uint);
                (*image).opaque = control;
                return 1 as ::core::ffi::c_int;
            }
            png_destroy_info_struct(png_ptr as png_const_structrp, &raw mut info_ptr);
        }
        png_destroy_write_struct(&raw mut png_ptr, ::core::ptr::null_mut::<*mut png_info>());
    }
    return png_image_error(
        image,
        b"png_image_write_: out of memory\0" as *const u8 as png_const_charp,
    );
}
unsafe extern "C" fn png_write_image_16bit(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
    let mut output_row: png_uint_16p = (*display).local_row as png_uint_16p;
    let mut row_end: png_uint_16p = ::core::ptr::null_mut::<png_uint_16>();
    let mut channels: ::core::ffi::c_uint = (if (*image).format as ::core::ffi::c_uint
        & PNG_FORMAT_FLAG_COLOR
        != 0 as ::core::ffi::c_uint
    {
        3 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    let mut aindex: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut y: png_uint_32 = (*image).height;
    if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
        if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST
            != 0 as ::core::ffi::c_uint
        {
            aindex = -(1 as ::core::ffi::c_int);
            input_row = input_row.offset(1);
            output_row = output_row.offset(1);
        } else {
            aindex = channels as ::core::ffi::c_int;
        }
    } else {
        png_error(
            png_ptr,
            b"png_write_image: internal call error\0" as *const u8 as png_const_charp,
        );
    }
    row_end = output_row.offset(
        ((*image).width as ::core::ffi::c_uint)
            .wrapping_mul(channels.wrapping_add(1 as ::core::ffi::c_uint)) as isize,
    );
    while y > 0 as ::core::ffi::c_uint {
        let mut in_ptr: png_const_uint_16p = input_row;
        let mut out_ptr: png_uint_16p = output_row;
        while out_ptr < row_end {
            let mut alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
            let mut reciprocal: png_uint_32 = 0 as png_uint_32;
            let mut c: ::core::ffi::c_int = 0;
            *out_ptr.offset(aindex as isize) = alpha;
            if alpha as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                && (alpha as ::core::ffi::c_int) < 65535 as ::core::ffi::c_int
            {
                reciprocal = ((((0xffff as ::core::ffi::c_int) << 15 as ::core::ffi::c_int)
                    + (alpha as ::core::ffi::c_int >> 1 as ::core::ffi::c_int))
                    / alpha as ::core::ffi::c_int) as png_uint_32;
            }
            c = channels as ::core::ffi::c_int;
            loop {
                let fresh4 = in_ptr;
                in_ptr = in_ptr.offset(1);
                let mut component: png_uint_16 = *fresh4;
                if component as ::core::ffi::c_int >= alpha as ::core::ffi::c_int {
                    component = 65535 as png_uint_16;
                } else if component as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                    && (alpha as ::core::ffi::c_int) < 65535 as ::core::ffi::c_int
                {
                    let mut calc: png_uint_32 = (component as png_uint_32).wrapping_mul(reciprocal);
                    calc = (calc as ::core::ffi::c_uint).wrapping_add(16384 as ::core::ffi::c_uint)
                        as png_uint_32 as png_uint_32;
                    component = (calc >> 15 as ::core::ffi::c_int) as png_uint_16;
                }
                let fresh5 = out_ptr;
                out_ptr = out_ptr.offset(1);
                *fresh5 = component;
                c -= 1;
                if !(c > 0 as ::core::ffi::c_int) {
                    break;
                }
            }
            in_ptr = in_ptr.offset(1);
            out_ptr = out_ptr.offset(1);
        }
        png_write_row(png_ptr, (*display).local_row as png_const_bytep);
        input_row = input_row.offset(((*display).row_step / 2 as ptrdiff_t) as isize);
        y = y.wrapping_sub(1);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_unpremultiply(
    mut component: png_uint_32,
    mut alpha: png_uint_32,
    mut reciprocal: png_uint_32,
) -> png_byte {
    if component >= alpha || alpha < 128 as ::core::ffi::c_uint {
        return 255 as png_byte;
    } else if component > 0 as ::core::ffi::c_uint {
        if alpha < 65407 as ::core::ffi::c_uint {
            component = (component as ::core::ffi::c_uint)
                .wrapping_mul(reciprocal as ::core::ffi::c_uint)
                as png_uint_32 as png_uint_32;
            component = (component as ::core::ffi::c_uint).wrapping_add(64 as ::core::ffi::c_uint)
                as png_uint_32 as png_uint_32;
            component >>= 7 as ::core::ffi::c_int;
        } else {
            component = (component as ::core::ffi::c_uint).wrapping_mul(255 as ::core::ffi::c_uint)
                as png_uint_32 as png_uint_32;
        }
        return (0xff as ::core::ffi::c_uint
            & (png_sRGB_base[(component >> 15 as ::core::ffi::c_int) as usize]
                as ::core::ffi::c_uint)
                .wrapping_add(
                    (component as ::core::ffi::c_uint & 0x7fff as ::core::ffi::c_uint)
                        .wrapping_mul(
                            png_sRGB_delta[(component >> 15 as ::core::ffi::c_int) as usize]
                                as ::core::ffi::c_uint,
                        )
                        >> 12 as ::core::ffi::c_int,
                )
                >> 8 as ::core::ffi::c_int) as png_byte;
    } else {
        return 0 as png_byte;
    };
}
unsafe extern "C" fn png_write_image_8bit(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut input_row: png_const_uint_16p = (*display).first_row as png_const_uint_16p;
    let mut output_row: png_bytep = (*display).local_row as png_bytep;
    let mut y: png_uint_32 = (*image).height;
    let mut channels: ::core::ffi::c_uint = (if (*image).format as ::core::ffi::c_uint
        & PNG_FORMAT_FLAG_COLOR
        != 0 as ::core::ffi::c_uint
    {
        3 as ::core::ffi::c_int
    } else {
        1 as ::core::ffi::c_int
    }) as ::core::ffi::c_uint;
    if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint {
        let mut row_end: png_bytep = ::core::ptr::null_mut::<png_byte>();
        let mut aindex: ::core::ffi::c_int = 0;
        if (*image).format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST
            != 0 as ::core::ffi::c_uint
        {
            aindex = -(1 as ::core::ffi::c_int);
            input_row = input_row.offset(1);
            output_row = output_row.offset(1);
        } else {
            aindex = channels as ::core::ffi::c_int;
        }
        row_end = output_row.offset(
            ((*image).width as ::core::ffi::c_uint)
                .wrapping_mul(channels.wrapping_add(1 as ::core::ffi::c_uint)) as isize,
        );
        while y > 0 as ::core::ffi::c_uint {
            let mut in_ptr: png_const_uint_16p = input_row;
            let mut out_ptr: png_bytep = output_row;
            while out_ptr < row_end {
                let mut alpha: png_uint_16 = *in_ptr.offset(aindex as isize);
                let mut alphabyte: png_byte = ((alpha as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    .wrapping_add(32895 as ::core::ffi::c_uint)
                    >> 16 as ::core::ffi::c_int)
                    as png_byte;
                let mut reciprocal: png_uint_32 = 0 as png_uint_32;
                let mut c: ::core::ffi::c_int = 0;
                *out_ptr.offset(aindex as isize) = alphabyte;
                if alphabyte as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                    && (alphabyte as ::core::ffi::c_int) < 255 as ::core::ffi::c_int
                {
                    reciprocal = ((((0xffff as ::core::ffi::c_int * 0xff as ::core::ffi::c_int)
                        << 7 as ::core::ffi::c_int)
                        + (alpha as ::core::ffi::c_int >> 1 as ::core::ffi::c_int))
                        / alpha as ::core::ffi::c_int)
                        as png_uint_32;
                }
                c = channels as ::core::ffi::c_int;
                loop {
                    let fresh0 = in_ptr;
                    in_ptr = in_ptr.offset(1);
                    let fresh1 = out_ptr;
                    out_ptr = out_ptr.offset(1);
                    *fresh1 =
                        png_unpremultiply(*fresh0 as png_uint_32, alpha as png_uint_32, reciprocal);
                    c -= 1;
                    if !(c > 0 as ::core::ffi::c_int) {
                        break;
                    }
                }
                in_ptr = in_ptr.offset(1);
                out_ptr = out_ptr.offset(1);
            }
            png_write_row(png_ptr, (*display).local_row as png_const_bytep);
            input_row = input_row.offset(((*display).row_step / 2 as ptrdiff_t) as isize);
            y = y.wrapping_sub(1);
        }
    } else {
        let mut row_end_0: png_bytep = output_row
            .offset(((*image).width as ::core::ffi::c_uint).wrapping_mul(channels) as isize);
        while y > 0 as ::core::ffi::c_uint {
            let mut in_ptr_0: png_const_uint_16p = input_row;
            let mut out_ptr_0: png_bytep = output_row;
            while out_ptr_0 < row_end_0 {
                let fresh2 = in_ptr_0;
                in_ptr_0 = in_ptr_0.offset(1);
                let mut component: png_uint_32 = *fresh2 as png_uint_32;
                component = (component as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    as png_uint_32 as png_uint_32;
                let fresh3 = out_ptr_0;
                out_ptr_0 = out_ptr_0.offset(1);
                *fresh3 = (0xff as ::core::ffi::c_uint
                    & (png_sRGB_base[(component >> 15 as ::core::ffi::c_int) as usize]
                        as ::core::ffi::c_uint)
                        .wrapping_add(
                            (component as ::core::ffi::c_uint & 0x7fff as ::core::ffi::c_uint)
                                .wrapping_mul(
                                    png_sRGB_delta[(component >> 15 as ::core::ffi::c_int) as usize]
                                        as ::core::ffi::c_uint,
                                )
                                >> 12 as ::core::ffi::c_int,
                        )
                        >> 8 as ::core::ffi::c_int) as png_byte;
            }
            png_write_row(png_ptr, output_row as png_const_bytep);
            input_row = input_row.offset(((*display).row_step / 2 as ptrdiff_t) as isize);
            y = y.wrapping_sub(1);
        }
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_image_set_PLTE(mut display: *mut png_image_write_control) {
    let mut image: png_imagep = (*display).image;
    let mut cmap: *const ::core::ffi::c_void = (*display).colormap as *const ::core::ffi::c_void;
    let mut entries: ::core::ffi::c_int = if (*image).colormap_entries > 256 as ::core::ffi::c_uint
    {
        256 as ::core::ffi::c_int
    } else {
        (*image).colormap_entries as ::core::ffi::c_int
    };
    let mut format: png_uint_32 = (*image).format;
    let mut channels: ::core::ffi::c_uint = (format as ::core::ffi::c_uint
        & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA))
        .wrapping_add(1 as ::core::ffi::c_uint);
    let mut afirst: ::core::ffi::c_int = (format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST
        != 0 as ::core::ffi::c_uint
        && format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint)
        as ::core::ffi::c_int;
    let mut bgr: ::core::ffi::c_int =
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_BGR != 0 as ::core::ffi::c_uint {
            2 as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        };
    let mut i: ::core::ffi::c_int = 0;
    let mut num_trans: ::core::ffi::c_int = 0;
    let mut palette: [png_color; 256] = [png_color {
        red: 0,
        green: 0,
        blue: 0,
    }; 256];
    let mut tRNS: [png_byte; 256] = [0; 256];
    memset(
        &raw mut tRNS as *mut png_byte as *mut ::core::ffi::c_void,
        255 as ::core::ffi::c_int,
        ::core::mem::size_of::<[png_byte; 256]>() as size_t,
    );
    memset(
        &raw mut palette as *mut png_color as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<[png_color; 256]>() as size_t,
    );
    num_trans = 0 as ::core::ffi::c_int;
    i = num_trans;
    while i < entries {
        if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR != 0 as ::core::ffi::c_uint {
            let mut entry: png_const_uint_16p = cmap as png_const_uint_16p;
            entry = entry.offset((i as ::core::ffi::c_uint).wrapping_mul(channels) as isize);
            if channels & 1 as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
                if channels >= 3 as ::core::ffi::c_uint {
                    palette[i as usize].blue = (0xff as ::core::ffi::c_int
                        & png_sRGB_base[(255 as ::core::ffi::c_int
                            * *entry.offset((2 as ::core::ffi::c_int ^ bgr) as isize)
                                as ::core::ffi::c_int
                            >> 15 as ::core::ffi::c_int)
                            as usize] as ::core::ffi::c_int
                            + ((255 as ::core::ffi::c_int
                                * *entry.offset((2 as ::core::ffi::c_int ^ bgr) as isize)
                                    as ::core::ffi::c_int
                                & 0x7fff as ::core::ffi::c_int)
                                * png_sRGB_delta[(255 as ::core::ffi::c_int
                                    * *entry.offset((2 as ::core::ffi::c_int ^ bgr) as isize)
                                        as ::core::ffi::c_int
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_int
                                >> 12 as ::core::ffi::c_int)
                            >> 8 as ::core::ffi::c_int)
                        as png_byte;
                    palette[i as usize].green = (0xff as ::core::ffi::c_int
                        & png_sRGB_base[(255 as ::core::ffi::c_int
                            * *entry.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >> 15 as ::core::ffi::c_int)
                            as usize] as ::core::ffi::c_int
                            + ((255 as ::core::ffi::c_int
                                * *entry.offset(1 as ::core::ffi::c_int as isize)
                                    as ::core::ffi::c_int
                                & 0x7fff as ::core::ffi::c_int)
                                * png_sRGB_delta[(255 as ::core::ffi::c_int
                                    * *entry.offset(1 as ::core::ffi::c_int as isize)
                                        as ::core::ffi::c_int
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_int
                                >> 12 as ::core::ffi::c_int)
                            >> 8 as ::core::ffi::c_int)
                        as png_byte;
                    palette[i as usize].red = (0xff as ::core::ffi::c_int
                        & png_sRGB_base[(255 as ::core::ffi::c_int
                            * *entry.offset(bgr as isize) as ::core::ffi::c_int
                            >> 15 as ::core::ffi::c_int)
                            as usize] as ::core::ffi::c_int
                            + ((255 as ::core::ffi::c_int
                                * *entry.offset(bgr as isize) as ::core::ffi::c_int
                                & 0x7fff as ::core::ffi::c_int)
                                * png_sRGB_delta[(255 as ::core::ffi::c_int
                                    * *entry.offset(bgr as isize) as ::core::ffi::c_int
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_int
                                >> 12 as ::core::ffi::c_int)
                            >> 8 as ::core::ffi::c_int)
                        as png_byte;
                } else {
                    palette[i as usize].green = (0xff as ::core::ffi::c_int
                        & png_sRGB_base[(255 as ::core::ffi::c_int * *entry as ::core::ffi::c_int
                            >> 15 as ::core::ffi::c_int)
                            as usize] as ::core::ffi::c_int
                            + ((255 as ::core::ffi::c_int * *entry as ::core::ffi::c_int
                                & 0x7fff as ::core::ffi::c_int)
                                * png_sRGB_delta[(255 as ::core::ffi::c_int
                                    * *entry as ::core::ffi::c_int
                                    >> 15 as ::core::ffi::c_int)
                                    as usize]
                                    as ::core::ffi::c_int
                                >> 12 as ::core::ffi::c_int)
                            >> 8 as ::core::ffi::c_int)
                        as png_byte;
                    palette[i as usize].red = palette[i as usize].green;
                    palette[i as usize].blue = palette[i as usize].red;
                }
            } else {
                let mut alpha: png_uint_16 = *entry.offset(
                    (if afirst != 0 {
                        0 as ::core::ffi::c_uint
                    } else {
                        channels.wrapping_sub(1 as ::core::ffi::c_uint)
                    }) as isize,
                );
                let mut alphabyte: png_byte = ((alpha as ::core::ffi::c_uint)
                    .wrapping_mul(255 as ::core::ffi::c_uint)
                    .wrapping_add(32895 as ::core::ffi::c_uint)
                    >> 16 as ::core::ffi::c_int)
                    as png_byte;
                let mut reciprocal: png_uint_32 = 0 as png_uint_32;
                if alphabyte as ::core::ffi::c_int > 0 as ::core::ffi::c_int
                    && (alphabyte as ::core::ffi::c_int) < 255 as ::core::ffi::c_int
                {
                    reciprocal = ((((0xffff as ::core::ffi::c_int * 0xff as ::core::ffi::c_int)
                        << 7 as ::core::ffi::c_int)
                        + (alpha as ::core::ffi::c_int >> 1 as ::core::ffi::c_int))
                        / alpha as ::core::ffi::c_int)
                        as png_uint_32;
                }
                tRNS[i as usize] = alphabyte;
                if (alphabyte as ::core::ffi::c_int) < 255 as ::core::ffi::c_int {
                    num_trans = i + 1 as ::core::ffi::c_int;
                }
                if channels >= 3 as ::core::ffi::c_uint {
                    palette[i as usize].blue = png_unpremultiply(
                        *entry.offset((afirst + (2 as ::core::ffi::c_int ^ bgr)) as isize)
                            as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].green = png_unpremultiply(
                        *entry.offset((afirst + 1 as ::core::ffi::c_int) as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].red = png_unpremultiply(
                        *entry.offset((afirst + bgr) as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                } else {
                    palette[i as usize].green = png_unpremultiply(
                        *entry.offset(afirst as isize) as png_uint_32,
                        alpha as png_uint_32,
                        reciprocal,
                    );
                    palette[i as usize].red = palette[i as usize].green;
                    palette[i as usize].blue = palette[i as usize].red;
                }
            }
        } else {
            let mut entry_0: png_const_bytep = cmap as png_const_bytep;
            entry_0 = entry_0.offset((i as ::core::ffi::c_uint).wrapping_mul(channels) as isize);
            let mut current_block_35: u64;
            match channels {
                4 => {
                    tRNS[i as usize] = *entry_0.offset(
                        (if afirst != 0 {
                            0 as ::core::ffi::c_int
                        } else {
                            3 as ::core::ffi::c_int
                        }) as isize,
                    );
                    if (tRNS[i as usize] as ::core::ffi::c_int) < 255 as ::core::ffi::c_int {
                        num_trans = i + 1 as ::core::ffi::c_int;
                    }
                    current_block_35 = 549485244942086866;
                }
                3 => {
                    current_block_35 = 549485244942086866;
                }
                2 => {
                    tRNS[i as usize] = *entry_0.offset((1 as ::core::ffi::c_int ^ afirst) as isize);
                    if (tRNS[i as usize] as ::core::ffi::c_int) < 255 as ::core::ffi::c_int {
                        num_trans = i + 1 as ::core::ffi::c_int;
                    }
                    current_block_35 = 2054208007906464708;
                }
                1 => {
                    current_block_35 = 2054208007906464708;
                }
                _ => {
                    current_block_35 = 15597372965620363352;
                }
            }
            match current_block_35 {
                549485244942086866 => {
                    palette[i as usize].blue =
                        *entry_0.offset((afirst + (2 as ::core::ffi::c_int ^ bgr)) as isize);
                    palette[i as usize].green =
                        *entry_0.offset((afirst + 1 as ::core::ffi::c_int) as isize);
                    palette[i as usize].red = *entry_0.offset((afirst + bgr) as isize);
                }
                2054208007906464708 => {
                    palette[i as usize].green = *entry_0.offset(afirst as isize);
                    palette[i as usize].red = palette[i as usize].green;
                    palette[i as usize].blue = palette[i as usize].red;
                }
                _ => {}
            }
        }
        i += 1;
    }
    png_set_PLTE(
        (*(*image).opaque).png_ptr as png_structrp,
        (*(*image).opaque).info_ptr as png_inforp,
        &raw mut palette as *mut png_color as png_const_colorp,
        entries,
    );
    if num_trans > 0 as ::core::ffi::c_int {
        png_set_tRNS(
            (*(*image).opaque).png_ptr as png_structrp,
            (*(*image).opaque).info_ptr as png_inforp,
            &raw mut tRNS as *mut png_byte as png_const_bytep,
            num_trans,
            ::core::ptr::null::<png_color_16>(),
        );
    }
    (*image).colormap_entries = entries as png_uint_32;
}
#[cfg(target_arch = "x86_64")]
#[inline]
unsafe fn png_c_u32_div(
    mut dividend: png_uint_32,
    divisor: png_uint_32,
) -> png_uint_32 {
    ::core::arch::asm!(
        "div {divisor:e}",
        divisor = in(reg) divisor,
        inout("eax") dividend,
        inout("edx") 0u32 => _,
        options(nomem, nostack)
    );
    dividend
}
#[cfg(not(target_arch = "x86_64"))]
#[inline]
unsafe fn png_c_u32_div(
    dividend: png_uint_32,
    divisor: png_uint_32,
) -> png_uint_32 {
    dividend / divisor
}
unsafe extern "C" fn png_image_write_main(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_write_control = argument as *mut png_image_write_control;
    let mut image: png_imagep = (*display).image;
    let mut png_ptr: png_structrp = (*(*image).opaque).png_ptr as png_structrp;
    let mut info_ptr: png_inforp = (*(*image).opaque).info_ptr as png_inforp;
    let mut format: png_uint_32 = (*image).format;
    let mut colormap: ::core::ffi::c_int =
        (format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP) as ::core::ffi::c_int;
    let mut linear: ::core::ffi::c_int = (colormap == 0
        && format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_LINEAR != 0)
        as ::core::ffi::c_int;
    let mut alpha: ::core::ffi::c_int = (colormap == 0
        && format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0)
        as ::core::ffi::c_int;
    let mut write_16bit: ::core::ffi::c_int = (linear != 0
        && (*display).convert_to_8bit == 0 as ::core::ffi::c_int)
        as ::core::ffi::c_int;
    png_set_benign_errors(png_ptr, 0 as ::core::ffi::c_int);
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
        let mut png_row_stride: png_uint_32 = (*image).width.wrapping_mul(channels as png_uint_32);
        if (*display).row_stride == 0 as ::core::ffi::c_int {
            (*display).row_stride = png_row_stride as png_int_32;
        }
        if (*display).row_stride < 0 as ::core::ffi::c_int {
            check = ((*display).row_stride as png_uint_32).wrapping_neg();
        } else {
            check = (*display).row_stride as png_uint_32;
        }
        if check >= png_row_stride {
            if (*image).height
                > png_c_u32_div(0xffffffff as png_uint_32, png_row_stride)
            {
                png_error(
                    (*(*image).opaque).png_ptr as png_const_structrp,
                    b"memory image too large\0" as *const u8 as png_const_charp,
                );
            }
        } else {
            png_error(
                (*(*image).opaque).png_ptr as png_const_structrp,
                b"supplied row stride too small\0" as *const u8 as png_const_charp,
            );
        }
    } else {
        png_error(
            (*(*image).opaque).png_ptr as png_const_structrp,
            b"image row stride too large\0" as *const u8 as png_const_charp,
        );
    }
    if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLORMAP != 0 as ::core::ffi::c_uint {
        if !(*display).colormap.is_null() && (*image).colormap_entries > 0 as ::core::ffi::c_uint {
            let mut entries: png_uint_32 = (*image).colormap_entries;
            png_set_IHDR(
                png_ptr,
                info_ptr,
                (*image).width,
                (*image).height,
                if entries > 16 as ::core::ffi::c_uint {
                    8 as ::core::ffi::c_int
                } else if entries > 4 as ::core::ffi::c_uint {
                    4 as ::core::ffi::c_int
                } else if entries > 2 as ::core::ffi::c_uint {
                    2 as ::core::ffi::c_int
                } else {
                    1 as ::core::ffi::c_int
                },
                PNG_COLOR_TYPE_PALETTE,
                PNG_INTERLACE_NONE,
                PNG_COMPRESSION_TYPE_BASE,
                PNG_FILTER_TYPE_BASE,
            );
            png_image_set_PLTE(display);
        } else {
            png_error(
                (*(*image).opaque).png_ptr as png_const_structrp,
                b"no color-map for color-mapped image\0" as *const u8 as png_const_charp,
            );
        }
    } else {
        png_set_IHDR(
            png_ptr,
            info_ptr,
            (*image).width,
            (*image).height,
            if write_16bit != 0 {
                16 as ::core::ffi::c_int
            } else {
                8 as ::core::ffi::c_int
            },
            (if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 {
                PNG_COLOR_MASK_COLOR
            } else {
                0 as ::core::ffi::c_int
            }) + (if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 {
                PNG_COLOR_MASK_ALPHA
            } else {
                0 as ::core::ffi::c_int
            }),
            PNG_INTERLACE_NONE,
            PNG_COMPRESSION_TYPE_BASE,
            PNG_FILTER_TYPE_BASE,
        );
    }
    if write_16bit != 0 as ::core::ffi::c_int {
        png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_LINEAR);
        if (*image).flags as ::core::ffi::c_uint
            & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB as ::core::ffi::c_uint
            == 0 as ::core::ffi::c_uint
        {
            png_set_cHRM_fixed(
                png_ptr,
                info_ptr,
                31270 as png_fixed_point,
                32900 as png_fixed_point,
                64000 as png_fixed_point,
                33000 as png_fixed_point,
                30000 as png_fixed_point,
                60000 as png_fixed_point,
                15000 as png_fixed_point,
                6000 as png_fixed_point,
            );
        }
    } else if (*image).flags as ::core::ffi::c_uint
        & PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB as ::core::ffi::c_uint
        == 0 as ::core::ffi::c_uint
    {
        png_set_sRGB(png_ptr, info_ptr, PNG_sRGB_INTENT_PERCEPTUAL);
    } else {
        png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);
    }
    png_write_info(png_ptr, info_ptr);
    if write_16bit != 0 as ::core::ffi::c_int {
        let mut le: png_uint_16 = 0x1 as png_uint_16;
        if *(&raw mut le as png_const_bytep) as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            png_set_swap(png_ptr);
        }
    }
    if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_BGR != 0 as ::core::ffi::c_uint {
        if colormap == 0 as ::core::ffi::c_int
            && format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_COLOR != 0 as ::core::ffi::c_uint
        {
            png_set_bgr(png_ptr);
        }
        format &= !PNG_FORMAT_FLAG_BGR;
    }
    if format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_AFIRST != 0 as ::core::ffi::c_uint {
        if colormap == 0 as ::core::ffi::c_int
            && format as ::core::ffi::c_uint & PNG_FORMAT_FLAG_ALPHA != 0 as ::core::ffi::c_uint
        {
            png_set_swap_alpha(png_ptr);
        }
        format &= !PNG_FORMAT_FLAG_AFIRST;
    }
    if colormap != 0 as ::core::ffi::c_int && (*image).colormap_entries <= 16 as ::core::ffi::c_uint
    {
        png_set_packing(png_ptr);
    }
    if format
        & !(PNG_FORMAT_FLAG_COLOR
            | PNG_FORMAT_FLAG_LINEAR
            | PNG_FORMAT_FLAG_ALPHA
            | PNG_FORMAT_FLAG_COLORMAP)
        != 0 as ::core::ffi::c_uint
    {
        png_error(
            png_ptr,
            b"png_write_image: unsupported transformation\0" as *const u8 as png_const_charp,
        );
    }
    let mut row: png_const_bytep = (*display).buffer as png_const_bytep;
    let mut row_step: ptrdiff_t = (*display).row_stride as ptrdiff_t;
    if linear != 0 as ::core::ffi::c_int {
        row_step = (row_step as ::core::ffi::c_long * 2 as ::core::ffi::c_long) as ptrdiff_t;
    }
    if row_step < 0 as ptrdiff_t {
        row = row.offset(
            (((*image).height as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                as ptrdiff_t
                * -row_step) as isize,
        );
    }
    (*display).first_row = row as png_const_voidp;
    (*display).row_step = row_step;
    if (*image).flags as ::core::ffi::c_uint & PNG_IMAGE_FLAG_FAST as ::core::ffi::c_uint
        != 0 as ::core::ffi::c_uint
    {
        png_set_filter(png_ptr, PNG_FILTER_TYPE_BASE, PNG_NO_FILTERS);
        png_set_compression_level(png_ptr, 3 as ::core::ffi::c_int);
    }
    if linear != 0 as ::core::ffi::c_int
        && (alpha != 0 as ::core::ffi::c_int
            || (*display).convert_to_8bit != 0 as ::core::ffi::c_int)
    {
        let mut row_0: png_bytep = png_malloc(
            png_ptr,
            png_get_rowbytes(png_ptr, info_ptr) as png_alloc_size_t,
        ) as png_bytep;
        let mut result: ::core::ffi::c_int = 0;
        (*display).local_row = row_0 as png_voidp;
        if write_16bit != 0 as ::core::ffi::c_int {
            result = png_safe_execute(
                image,
                Some(
                    png_write_image_16bit as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                ),
                display as png_voidp,
            );
        } else {
            result = png_safe_execute(
                image,
                Some(png_write_image_8bit as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int),
                display as png_voidp,
            );
        }
        (*display).local_row = NULL_0 as png_voidp;
        png_free(png_ptr, row_0 as png_voidp);
        if result == 0 as ::core::ffi::c_int {
            return 0 as ::core::ffi::c_int;
        }
    } else {
        let mut row_1: png_const_bytep = (*display).first_row as png_const_bytep;
        let mut row_step_0: ptrdiff_t = (*display).row_step;
        let mut y: png_uint_32 = (*image).height;
        while y > 0 as ::core::ffi::c_uint {
            png_write_row(png_ptr, row_1);
            row_1 = row_1.offset(row_step_0 as isize);
            y = y.wrapping_sub(1);
        }
    }
    png_write_end(png_ptr, info_ptr);
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn image_memory_write(
    mut png_ptr: png_structp,
    mut data: png_bytep,
    mut size: size_t,
) {
    let mut display: *mut png_image_write_control =
        (*png_ptr).io_ptr as *mut png_image_write_control;
    let mut ob: png_alloc_size_t = (*display).output_bytes;
    if size <= (-(1 as ::core::ffi::c_int) as png_alloc_size_t).wrapping_sub(ob) {
        if size > 0 as size_t {
            if (*display).memory_bytes >= ob.wrapping_add(size as png_alloc_size_t) {
                memcpy(
                    (*display).memory.offset(ob as isize) as *mut ::core::ffi::c_void,
                    data as *const ::core::ffi::c_void,
                    size,
                );
            }
            (*display).output_bytes = ob.wrapping_add(size as png_alloc_size_t);
        }
    } else {
        png_error(
            png_ptr as png_const_structrp,
            b"png_image_write_to_memory: PNG too big\0" as *const u8 as png_const_charp,
        );
    };
}
unsafe extern "C" fn image_memory_flush(mut png_ptr: png_structp) {}
unsafe extern "C" fn png_image_write_memory(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut display: *mut png_image_write_control = argument as *mut png_image_write_control;
    png_set_write_fn(
        (*(*(*display).image).opaque).png_ptr as png_structrp,
        display as png_voidp,
        Some(image_memory_write as unsafe extern "C" fn(png_structp, png_bytep, size_t) -> ()),
        Some(image_memory_flush as unsafe extern "C" fn(png_structp) -> ()),
    );
    return png_image_write_main(display as png_voidp);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_memory(
    mut image: png_imagep,
    mut memory: *mut ::core::ffi::c_void,
    mut memory_bytes: *mut png_alloc_size_t,
    mut convert_to_8bit: ::core::ffi::c_int,
    mut buffer: *const ::core::ffi::c_void,
    mut row_stride: png_int_32,
    mut colormap: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !memory_bytes.is_null() && !buffer.is_null() {
            if memory.is_null() {
                *memory_bytes = 0 as png_alloc_size_t;
            }
            if png_image_write_init(image) != 0 as ::core::ffi::c_int {
                let mut display: png_image_write_control = png_image_write_control {
                    image: ::core::ptr::null_mut::<C2RustUnnamed>(),
                    buffer: ::core::ptr::null::<::core::ffi::c_void>(),
                    row_stride: 0,
                    colormap: ::core::ptr::null::<::core::ffi::c_void>(),
                    convert_to_8bit: 0,
                    first_row: ::core::ptr::null::<::core::ffi::c_void>(),
                    local_row: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                    row_step: 0,
                    memory: ::core::ptr::null_mut::<png_byte>(),
                    memory_bytes: 0,
                    output_bytes: 0,
                };
                let mut result: ::core::ffi::c_int = 0;
                memset(
                    &raw mut display as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    ::core::mem::size_of::<png_image_write_control>() as size_t,
                );
                display.image = image;
                display.buffer = buffer as png_const_voidp;
                display.row_stride = row_stride;
                display.colormap = colormap as png_const_voidp;
                display.convert_to_8bit = convert_to_8bit;
                display.memory = memory as png_bytep;
                display.memory_bytes = *memory_bytes;
                display.output_bytes = 0 as png_alloc_size_t;
                result = png_safe_execute(
                    image,
                    Some(
                        png_image_write_memory
                            as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                    ),
                    &raw mut display as png_voidp,
                );
                png_image_free(image);
                if result != 0 {
                    if !memory.is_null() && display.output_bytes > *memory_bytes {
                        result = 0 as ::core::ffi::c_int;
                    }
                    *memory_bytes = display.output_bytes;
                }
                return result;
            } else {
                return 0 as ::core::ffi::c_int;
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_memory: invalid argument\0" as *const u8 as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_write_to_memory: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    } else {
        return 0 as ::core::ffi::c_int;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_stdio(
    mut image: png_imagep,
    mut file: *mut FILE,
    mut convert_to_8bit: ::core::ffi::c_int,
    mut buffer: *const ::core::ffi::c_void,
    mut row_stride: png_int_32,
    mut colormap: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !file.is_null() && !buffer.is_null() {
            if png_image_write_init(image) != 0 as ::core::ffi::c_int {
                let mut display: png_image_write_control = png_image_write_control {
                    image: ::core::ptr::null_mut::<C2RustUnnamed>(),
                    buffer: ::core::ptr::null::<::core::ffi::c_void>(),
                    row_stride: 0,
                    colormap: ::core::ptr::null::<::core::ffi::c_void>(),
                    convert_to_8bit: 0,
                    first_row: ::core::ptr::null::<::core::ffi::c_void>(),
                    local_row: ::core::ptr::null_mut::<::core::ffi::c_void>(),
                    row_step: 0,
                    memory: ::core::ptr::null_mut::<png_byte>(),
                    memory_bytes: 0,
                    output_bytes: 0,
                };
                let mut result: ::core::ffi::c_int = 0;
                (*(*(*image).opaque).png_ptr).io_ptr = file as png_voidp;
                memset(
                    &raw mut display as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    ::core::mem::size_of::<png_image_write_control>() as size_t,
                );
                display.image = image;
                display.buffer = buffer as png_const_voidp;
                display.row_stride = row_stride;
                display.colormap = colormap as png_const_voidp;
                display.convert_to_8bit = convert_to_8bit;
                result = png_safe_execute(
                    image,
                    Some(
                        png_image_write_main
                            as unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int,
                    ),
                    &raw mut display as png_voidp,
                );
                png_image_free(image);
                return result;
            } else {
                return 0 as ::core::ffi::c_int;
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_stdio: invalid argument\0" as *const u8 as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_write_to_stdio: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    } else {
        return 0 as ::core::ffi::c_int;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_write_to_file(
    mut image: png_imagep,
    mut file_name: *const ::core::ffi::c_char,
    mut convert_to_8bit: ::core::ffi::c_int,
    mut buffer: *const ::core::ffi::c_void,
    mut row_stride: png_int_32,
    mut colormap: *const ::core::ffi::c_void,
) -> ::core::ffi::c_int {
    if !image.is_null() && (*image).version == PNG_IMAGE_VERSION as ::core::ffi::c_uint {
        if !file_name.is_null() && !buffer.is_null() {
            let mut fp: *mut FILE = fopen(
                file_name,
                b"wb\0" as *const u8 as *const ::core::ffi::c_char,
            );
            if !fp.is_null() {
                if png_image_write_to_stdio(
                    image,
                    fp,
                    convert_to_8bit,
                    buffer,
                    row_stride,
                    colormap,
                ) != 0 as ::core::ffi::c_int
                {
                    let mut error: ::core::ffi::c_int = 0;
                    if fflush(fp) == 0 as ::core::ffi::c_int
                        && ferror(fp) == 0 as ::core::ffi::c_int
                    {
                        if fclose(fp) == 0 as ::core::ffi::c_int {
                            return 1 as ::core::ffi::c_int;
                        }
                        error = *__errno_location();
                    } else {
                        error = *__errno_location();
                        fclose(fp);
                    }
                    remove(file_name);
                    return png_image_error(image, strerror(error) as png_const_charp);
                } else {
                    fclose(fp);
                    remove(file_name);
                    return 0 as ::core::ffi::c_int;
                }
            } else {
                return png_image_error(image, strerror(*__errno_location()) as png_const_charp);
            }
        } else {
            return png_image_error(
                image,
                b"png_image_write_to_file: invalid argument\0" as *const u8 as png_const_charp,
            );
        }
    } else if !image.is_null() {
        return png_image_error(
            image,
            b"png_image_write_to_file: incorrect PNG_IMAGE_VERSION\0" as *const u8
                as png_const_charp,
        );
    } else {
        return 0 as ::core::ffi::c_int;
    };
}
pub const PNG_HAVE_IDAT: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_WROTE_tIME: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_WROTE_INFO_BEFORE_PLTE: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_HAVE_PNG_SIGNATURE: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_WROTE_eXIf: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_INTERLACE: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_INVERT_ALPHA: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PNG_USER_TRANSFORM: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZLIB_CUSTOM_STRATEGY: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZSTREAM_INITIALIZED: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_GAMMA_sRGB_INVERSE: ::core::ffi::c_int = 45455 as ::core::ffi::c_int;
