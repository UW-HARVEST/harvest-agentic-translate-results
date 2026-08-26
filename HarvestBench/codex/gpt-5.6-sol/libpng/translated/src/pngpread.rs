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
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn png_sig_cmp(sig: png_const_bytep, start: size_t, num_to_check: size_t)
        -> ::core::ffi::c_int;
    fn png_set_read_fn(png_ptr: png_structrp, io_ptr: png_voidp, read_data_fn: png_rw_ptr);
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_get_uint_31(png_ptr: png_const_structrp, buf: png_const_bytep) -> png_uint_32;
    fn png_reset_crc(png_ptr: png_structrp);
    fn png_read_chunk_header(png_ptr: png_structrp) -> png_uint_32;
    fn png_crc_read(png_ptr: png_structrp, buf: png_bytep, length: png_uint_32);
    fn png_crc_finish(png_ptr: png_structrp, skip: png_uint_32) -> ::core::ffi::c_int;
    fn png_calculate_crc(png_ptr: png_structrp, ptr: png_const_bytep, length: size_t);
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
    fn png_zlib_inflate(png_ptr: png_structrp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
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
    fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp);
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
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_16p = *mut png_uint_16;
pub type png_charp = *mut ::core::ffi::c_char;
pub type png_const_charp = *const ::core::ffi::c_char;
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
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;
pub type png_inforp = *mut png_info;
pub type png_handle_result_code = ::core::ffi::c_uint;
pub const handled_ok: png_handle_result_code = 3;
pub const handled_saved: png_handle_result_code = 2;
pub const handled_discarded: png_handle_result_code = 1;
pub const handled_error: png_handle_result_code = 0;
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
pub const PNG_HAVE_IHDR: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_HAVE_PLTE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_AFTER_IDAT: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_SIZE_MAX: size_t = -(1 as ::core::ffi::c_int) as size_t;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_FILTER_VALUE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_LAST: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const Z_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_STREAM_END: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const Z_DATA_ERROR: ::core::ffi::c_int = -(3 as ::core::ffi::c_int);
pub const PNG_READ_SIG_MODE: ::core::ffi::c_int = 0;
pub const PNG_READ_CHUNK_MODE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_READ_IDAT_MODE: ::core::ffi::c_int = 2;
pub const PNG_READ_DONE_MODE: ::core::ffi::c_int = 6 as ::core::ffi::c_int;
static mut png_pass_start: [png_byte; 7] = [
    0 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    1 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
];
static mut png_pass_inc: [png_byte; 7] = [
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
    1 as ::core::ffi::c_int as png_byte,
];
static mut png_pass_ystart: [png_byte; 7] = [
    0 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    1 as ::core::ffi::c_int as png_byte,
];
static mut png_pass_yinc: [png_byte; 7] = [
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    4 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
    2 as ::core::ffi::c_int as png_byte,
];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
    mut buffer: png_bytep,
    mut buffer_size: size_t,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    png_push_restore_buffer(png_ptr, buffer, buffer_size);
    while (*png_ptr).buffer_size != 0 {
        png_process_some_data(png_ptr, info_ptr);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data_pause(
    mut png_ptr: png_structrp,
    mut save: ::core::ffi::c_int,
) -> size_t {
    if !png_ptr.is_null() {
        if save != 0 as ::core::ffi::c_int {
            png_push_save_buffer(png_ptr);
        } else {
            let mut remaining: size_t = (*png_ptr).buffer_size;
            (*png_ptr).buffer_size = 0 as size_t;
            if (*png_ptr).save_buffer_size < remaining {
                return remaining.wrapping_sub((*png_ptr).save_buffer_size);
            }
        }
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_data_skip(mut png_ptr: png_structrp) -> png_uint_32 {
    png_app_warning(
        png_ptr,
        b"png_process_data_skip is not implemented in any current version of libpng\0" as *const u8
            as png_const_charp,
    );
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_some_data(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
) {
    if png_ptr.is_null() {
        return;
    }
    match (*png_ptr).process_mode {
        PNG_READ_SIG_MODE => {
            png_push_read_sig(png_ptr, info_ptr);
        }
        PNG_READ_CHUNK_MODE => {
            png_push_read_chunk(png_ptr, info_ptr);
        }
        PNG_READ_IDAT_MODE => {
            png_push_read_IDAT(png_ptr);
        }
        _ => {
            (*png_ptr).buffer_size = 0 as size_t;
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_sig(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    let mut num_checked: size_t = (*png_ptr).sig_bytes as size_t;
    let mut num_to_check: size_t = (8 as size_t).wrapping_sub(num_checked);
    if (*png_ptr).buffer_size < num_to_check {
        num_to_check = (*png_ptr).buffer_size;
    }
    png_push_fill_buffer(
        png_ptr as png_structp,
        (&raw mut (*info_ptr).signature as *mut png_byte).offset(num_checked as isize) as png_bytep,
        num_to_check,
    );
    (*png_ptr).sig_bytes = ((*png_ptr).sig_bytes as size_t).wrapping_add(num_to_check) as png_byte;
    if png_sig_cmp(
        &raw mut (*info_ptr).signature as *mut png_byte as png_const_bytep,
        num_checked,
        num_to_check,
    ) != 0 as ::core::ffi::c_int
    {
        if num_checked < 4 as size_t
            && png_sig_cmp(
                &raw mut (*info_ptr).signature as *mut png_byte as png_const_bytep,
                num_checked,
                num_to_check.wrapping_sub(4 as size_t),
            ) != 0 as ::core::ffi::c_int
        {
            png_error(png_ptr, b"Not a PNG file\0" as *const u8 as png_const_charp);
        } else {
            png_error(
                png_ptr,
                b"PNG file corrupted by ASCII conversion\0" as *const u8 as png_const_charp,
            );
        }
    } else if (*png_ptr).sig_bytes as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
        (*png_ptr).process_mode = PNG_READ_CHUNK_MODE;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_chunk(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    let mut chunk_name: png_uint_32 = 0;
    let mut keep: ::core::ffi::c_int = 0;
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_CHUNK_HEADER == 0 as ::core::ffi::c_uint {
        if (*png_ptr).buffer_size < 8 as size_t {
            png_push_save_buffer(png_ptr);
            return;
        }
        (*png_ptr).push_length = png_read_chunk_header(png_ptr);
        (*png_ptr).mode |= PNG_HAVE_CHUNK_HEADER;
    }
    chunk_name = (*png_ptr).chunk_name;
    if chunk_name == png_IDAT {
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_AFTER_IDAT as ::core::ffi::c_uint
            != 0 as ::core::ffi::c_uint
        {
            (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT;
        }
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IHDR as ::core::ffi::c_uint
            == 0 as ::core::ffi::c_uint
        {
            png_error(
                png_ptr,
                b"Missing IHDR before IDAT\0" as *const u8 as png_const_charp,
            );
        } else if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
            && (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PLTE as ::core::ffi::c_uint
                == 0 as ::core::ffi::c_uint
        {
            png_error(
                png_ptr,
                b"Missing PLTE before IDAT\0" as *const u8 as png_const_charp,
            );
        }
        (*png_ptr).process_mode = PNG_READ_IDAT_MODE;
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT != 0 as ::core::ffi::c_uint {
            if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_CHUNK_AFTER_IDAT
                == 0 as ::core::ffi::c_uint
            {
                if (*png_ptr).push_length == 0 as ::core::ffi::c_uint {
                    return;
                }
            }
        }
        (*png_ptr).mode |= PNG_HAVE_IDAT;
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_AFTER_IDAT as ::core::ffi::c_uint
            != 0 as ::core::ffi::c_uint
        {
            png_benign_error(
                png_ptr,
                b"Too many IDATs found\0" as *const u8 as png_const_charp,
            );
        }
    } else if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT != 0 as ::core::ffi::c_uint {
        (*png_ptr).mode |= PNG_HAVE_CHUNK_AFTER_IDAT | PNG_AFTER_IDAT as ::core::ffi::c_uint;
    }
    if chunk_name == png_IHDR {
        if (*png_ptr).push_length != 13 as ::core::ffi::c_uint {
            png_error(
                png_ptr,
                b"Invalid IHDR length\0" as *const u8 as png_const_charp,
            );
        }
        if ((*png_ptr).push_length as ::core::ffi::c_uint).wrapping_add(4 as ::core::ffi::c_uint)
            as size_t
            > (*png_ptr).buffer_size
        {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
    } else if chunk_name == png_IEND {
        if ((*png_ptr).push_length as ::core::ffi::c_uint).wrapping_add(4 as ::core::ffi::c_uint)
            as size_t
            > (*png_ptr).buffer_size
        {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
        (*png_ptr).process_mode = PNG_READ_DONE_MODE;
        png_push_have_end(png_ptr, info_ptr);
    } else {
        keep = png_chunk_unknown_handling(png_ptr, chunk_name);
        if keep != 0 as ::core::ffi::c_int {
            if ((*png_ptr).push_length as ::core::ffi::c_uint)
                .wrapping_add(4 as ::core::ffi::c_uint) as size_t
                > (*png_ptr).buffer_size
            {
                png_push_save_buffer(png_ptr);
                return;
            }
            png_handle_unknown(png_ptr, info_ptr, (*png_ptr).push_length, keep);
            if chunk_name == png_PLTE {
                (*png_ptr).mode |= PNG_HAVE_PLTE as ::core::ffi::c_uint;
            }
        } else if chunk_name == png_IDAT {
            (*png_ptr).idat_size = (*png_ptr).push_length;
            (*png_ptr).process_mode = PNG_READ_IDAT_MODE;
            png_push_have_info(png_ptr, info_ptr);
            (*png_ptr).zstream.avail_out =
                ((if (*png_ptr).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    ((*png_ptr).iwidth as size_t)
                        .wrapping_mul((*png_ptr).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
                } else {
                    ((*png_ptr).iwidth as size_t)
                        .wrapping_mul((*png_ptr).pixel_depth as size_t)
                        .wrapping_add(7 as size_t)
                        >> 3 as ::core::ffi::c_int
                }) as ::core::ffi::c_uint)
                    .wrapping_add(1 as ::core::ffi::c_uint) as uInt;
            (*png_ptr).zstream.next_out = (*png_ptr).row_buf as *mut Bytef;
            return;
        } else {
            if ((*png_ptr).push_length as ::core::ffi::c_uint)
                .wrapping_add(4 as ::core::ffi::c_uint) as size_t
                > (*png_ptr).buffer_size
            {
                png_push_save_buffer(png_ptr);
                return;
            }
            png_handle_chunk(png_ptr, info_ptr, (*png_ptr).push_length);
        }
    }
    (*png_ptr).mode &= !PNG_HAVE_CHUNK_HEADER;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_fill_buffer(
    mut png_ptr: png_structp,
    mut buffer: png_bytep,
    mut length: size_t,
) {
    let mut ptr: png_bytep = ::core::ptr::null_mut::<png_byte>();
    if png_ptr.is_null() {
        return;
    }
    ptr = buffer;
    if (*png_ptr).save_buffer_size != 0 as size_t {
        let mut save_size: size_t = 0;
        if length < (*png_ptr).save_buffer_size {
            save_size = length;
        } else {
            save_size = (*png_ptr).save_buffer_size;
        }
        memcpy(
            ptr as *mut ::core::ffi::c_void,
            (*png_ptr).save_buffer_ptr as *const ::core::ffi::c_void,
            save_size,
        );
        length = (length as ::core::ffi::c_ulong).wrapping_sub(save_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        ptr = ptr.offset(save_size as isize);
        (*png_ptr).buffer_size = ((*png_ptr).buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).save_buffer_size = ((*png_ptr).save_buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer_ptr.offset(save_size as isize);
    }
    if length != 0 as size_t && (*png_ptr).current_buffer_size != 0 as size_t {
        let mut save_size_0: size_t = 0;
        if length < (*png_ptr).current_buffer_size {
            save_size_0 = length;
        } else {
            save_size_0 = (*png_ptr).current_buffer_size;
        }
        memcpy(
            ptr as *mut ::core::ffi::c_void,
            (*png_ptr).current_buffer_ptr as *const ::core::ffi::c_void,
            save_size_0,
        );
        (*png_ptr).buffer_size = ((*png_ptr).buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size_0 as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).current_buffer_size = ((*png_ptr).current_buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size_0 as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer_ptr.offset(save_size_0 as isize);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_save_buffer(mut png_ptr: png_structrp) {
    if (*png_ptr).save_buffer_size != 0 as size_t {
        if (*png_ptr).save_buffer_ptr != (*png_ptr).save_buffer {
            let mut i: size_t = 0;
            let mut istop: size_t = 0;
            let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            istop = (*png_ptr).save_buffer_size;
            i = 0 as size_t;
            sp = (*png_ptr).save_buffer_ptr;
            dp = (*png_ptr).save_buffer;
            while i < istop {
                *dp = *sp;
                i = i.wrapping_add(1);
                sp = sp.offset(1);
                dp = dp.offset(1);
            }
        }
    }
    if (*png_ptr)
        .save_buffer_size
        .wrapping_add((*png_ptr).current_buffer_size)
        > (*png_ptr).save_buffer_max
    {
        let mut new_max: size_t = 0;
        let mut old_buffer: png_bytep = ::core::ptr::null_mut::<png_byte>();
        if (*png_ptr).save_buffer_size
            > PNG_SIZE_MAX.wrapping_sub((*png_ptr).current_buffer_size.wrapping_add(256 as size_t))
        {
            png_error(
                png_ptr,
                b"Potential overflow of save_buffer\0" as *const u8 as png_const_charp,
            );
        }
        new_max = (*png_ptr)
            .save_buffer_size
            .wrapping_add((*png_ptr).current_buffer_size)
            .wrapping_add(256 as size_t);
        old_buffer = (*png_ptr).save_buffer;
        (*png_ptr).save_buffer = png_malloc_warn(png_ptr, new_max) as png_bytep;
        if (*png_ptr).save_buffer.is_null() {
            png_free(png_ptr, old_buffer as png_voidp);
            png_error(
                png_ptr,
                b"Insufficient memory for save_buffer\0" as *const u8 as png_const_charp,
            );
        }
        if !old_buffer.is_null() {
            memcpy(
                (*png_ptr).save_buffer as *mut ::core::ffi::c_void,
                old_buffer as *const ::core::ffi::c_void,
                (*png_ptr).save_buffer_size,
            );
        } else if (*png_ptr).save_buffer_size != 0 {
            png_error(
                png_ptr,
                b"save_buffer error\0" as *const u8 as png_const_charp,
            );
        }
        png_free(png_ptr, old_buffer as png_voidp);
        (*png_ptr).save_buffer_max = new_max;
    }
    if (*png_ptr).current_buffer_size != 0 {
        memcpy(
            (*png_ptr)
                .save_buffer
                .offset((*png_ptr).save_buffer_size as isize)
                as *mut ::core::ffi::c_void,
            (*png_ptr).current_buffer_ptr as *const ::core::ffi::c_void,
            (*png_ptr).current_buffer_size,
        );
        (*png_ptr).save_buffer_size = ((*png_ptr).save_buffer_size as ::core::ffi::c_ulong)
            .wrapping_add((*png_ptr).current_buffer_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).current_buffer_size = 0 as size_t;
    }
    (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer;
    (*png_ptr).buffer_size = 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_restore_buffer(
    mut png_ptr: png_structrp,
    mut buffer: png_bytep,
    mut buffer_length: size_t,
) {
    (*png_ptr).current_buffer = buffer;
    (*png_ptr).current_buffer_size = buffer_length;
    (*png_ptr).buffer_size = buffer_length.wrapping_add((*png_ptr).save_buffer_size);
    (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_read_IDAT(mut png_ptr: png_structrp) {
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_CHUNK_HEADER == 0 as ::core::ffi::c_uint {
        let mut chunk_length: [png_byte; 4] = [0; 4];
        let mut chunk_tag: [png_byte; 4] = [0; 4];
        if (*png_ptr).buffer_size < 8 as size_t {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_push_fill_buffer(
            png_ptr as png_structp,
            &raw mut chunk_length as png_bytep,
            4 as size_t,
        );
        (*png_ptr).push_length = png_get_uint_31(
            png_ptr,
            &raw mut chunk_length as *mut png_byte as png_const_bytep,
        );
        png_reset_crc(png_ptr);
        png_crc_read(png_ptr, &raw mut chunk_tag as png_bytep, 4 as png_uint_32);
        (*png_ptr).chunk_name = ((0xffffffff as ::core::ffi::c_uint
            & (0xff as ::core::ffi::c_int
                & chunk_tag[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int)
                as ::core::ffi::c_uint)
            << 24 as ::core::ffi::c_int
            | (0xffffffff as ::core::ffi::c_uint
                & (0xff as ::core::ffi::c_int
                    & chunk_tag[1 as ::core::ffi::c_int as usize] as ::core::ffi::c_int)
                    as ::core::ffi::c_uint)
                << 16 as ::core::ffi::c_int
            | (0xffffffff as ::core::ffi::c_uint
                & (0xff as ::core::ffi::c_int
                    & chunk_tag[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int)
                    as ::core::ffi::c_uint)
                << 8 as ::core::ffi::c_int
            | (0xffffffff as ::core::ffi::c_uint
                & (0xff as ::core::ffi::c_int
                    & chunk_tag[3 as ::core::ffi::c_int as usize] as ::core::ffi::c_int)
                    as ::core::ffi::c_uint)
                << 0 as ::core::ffi::c_int) as png_uint_32;
        (*png_ptr).mode |= PNG_HAVE_CHUNK_HEADER;
        if (*png_ptr).chunk_name != png_IDAT {
            (*png_ptr).process_mode = PNG_READ_CHUNK_MODE;
            if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_ENDED
                == 0 as ::core::ffi::c_uint
            {
                png_error(
                    png_ptr,
                    b"Not enough compressed data\0" as *const u8 as png_const_charp,
                );
            }
            return;
        }
        (*png_ptr).idat_size = (*png_ptr).push_length;
    }
    if (*png_ptr).idat_size != 0 as ::core::ffi::c_uint
        && (*png_ptr).save_buffer_size != 0 as size_t
    {
        let mut save_size: size_t = (*png_ptr).save_buffer_size;
        let mut idat_size: png_uint_32 = (*png_ptr).idat_size;
        if (idat_size as size_t) < save_size {
            save_size = idat_size as size_t;
        } else {
            idat_size = save_size as png_uint_32;
        }
        png_calculate_crc(
            png_ptr,
            (*png_ptr).save_buffer_ptr as png_const_bytep,
            save_size,
        );
        png_process_IDAT_data(png_ptr, (*png_ptr).save_buffer_ptr, save_size);
        (*png_ptr).idat_size = ((*png_ptr).idat_size as ::core::ffi::c_uint)
            .wrapping_sub(idat_size as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        (*png_ptr).buffer_size = ((*png_ptr).buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).save_buffer_size = ((*png_ptr).save_buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).save_buffer_ptr = (*png_ptr).save_buffer_ptr.offset(save_size as isize);
    }
    if (*png_ptr).idat_size != 0 as ::core::ffi::c_uint
        && (*png_ptr).current_buffer_size != 0 as size_t
    {
        let mut save_size_0: size_t = (*png_ptr).current_buffer_size;
        let mut idat_size_0: png_uint_32 = (*png_ptr).idat_size;
        if (idat_size_0 as size_t) < save_size_0 {
            save_size_0 = idat_size_0 as size_t;
        } else {
            idat_size_0 = save_size_0 as png_uint_32;
        }
        png_calculate_crc(
            png_ptr,
            (*png_ptr).current_buffer_ptr as png_const_bytep,
            save_size_0,
        );
        png_process_IDAT_data(png_ptr, (*png_ptr).current_buffer_ptr, save_size_0);
        (*png_ptr).idat_size = ((*png_ptr).idat_size as ::core::ffi::c_uint)
            .wrapping_sub(idat_size_0 as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        (*png_ptr).buffer_size = ((*png_ptr).buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size_0 as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).current_buffer_size = ((*png_ptr).current_buffer_size as ::core::ffi::c_ulong)
            .wrapping_sub(save_size_0 as ::core::ffi::c_ulong)
            as size_t as size_t;
        (*png_ptr).current_buffer_ptr = (*png_ptr).current_buffer_ptr.offset(save_size_0 as isize);
    }
    if (*png_ptr).idat_size == 0 as ::core::ffi::c_uint {
        if (*png_ptr).buffer_size < 4 as size_t {
            png_push_save_buffer(png_ptr);
            return;
        }
        png_crc_finish(png_ptr, 0 as png_uint_32);
        (*png_ptr).mode &= !PNG_HAVE_CHUNK_HEADER;
        (*png_ptr).mode |= PNG_AFTER_IDAT as ::core::ffi::c_uint;
        (*png_ptr).zowner = 0 as png_uint_32;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_process_IDAT_data(
    mut png_ptr: png_structrp,
    mut buffer: png_bytep,
    mut buffer_length: size_t,
) {
    if !(buffer_length > 0 as size_t) || buffer.is_null() {
        png_error(
            png_ptr,
            b"No IDAT data (internal error)\0" as *const u8 as png_const_charp,
        );
    }
    (*png_ptr).zstream.next_in = buffer as *const Bytef;
    (*png_ptr).zstream.avail_in = buffer_length as uInt;
    while (*png_ptr).zstream.avail_in > 0 as ::core::ffi::c_uint
        && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_ENDED
            == 0 as ::core::ffi::c_uint
    {
        let mut ret: ::core::ffi::c_int = 0;
        if !((*png_ptr).zstream.avail_out > 0 as ::core::ffi::c_uint) {
            (*png_ptr).zstream.avail_out =
                (if (*png_ptr).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    ((*png_ptr).iwidth as size_t)
                        .wrapping_mul((*png_ptr).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
                } else {
                    ((*png_ptr).iwidth as size_t)
                        .wrapping_mul((*png_ptr).pixel_depth as size_t)
                        .wrapping_add(7 as size_t)
                        >> 3 as ::core::ffi::c_int
                })
                .wrapping_add(1 as size_t) as uInt;
            (*png_ptr).zstream.next_out = (*png_ptr).row_buf as *mut Bytef;
        }
        ret = png_zlib_inflate(png_ptr, 2 as ::core::ffi::c_int);
        if ret != Z_OK && ret != Z_STREAM_END {
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
            (*png_ptr).zowner = 0 as png_uint_32;
            if (*png_ptr).row_number >= (*png_ptr).num_rows
                || (*png_ptr).pass as ::core::ffi::c_int > 6 as ::core::ffi::c_int
            {
                png_warning(
                    png_ptr,
                    b"Truncated compressed data in IDAT\0" as *const u8 as png_const_charp,
                );
            } else if ret == Z_DATA_ERROR {
                png_benign_error(
                    png_ptr,
                    b"IDAT: ADLER32 checksum mismatch\0" as *const u8 as png_const_charp,
                );
            } else {
                png_error(
                    png_ptr,
                    b"Decompression error in IDAT\0" as *const u8 as png_const_charp,
                );
            }
            return;
        }
        if (*png_ptr).zstream.next_out != (*png_ptr).row_buf {
            if (*png_ptr).row_number >= (*png_ptr).num_rows
                || (*png_ptr).pass as ::core::ffi::c_int > 6 as ::core::ffi::c_int
            {
                png_warning(
                    png_ptr,
                    b"Extra compressed data in IDAT\0" as *const u8 as png_const_charp,
                );
                (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
                (*png_ptr).zowner = 0 as png_uint_32;
                return;
            }
            if (*png_ptr).zstream.avail_out == 0 as ::core::ffi::c_uint {
                png_push_process_row(png_ptr);
            }
        }
        if ret == Z_STREAM_END {
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_ENDED;
        }
    }
    if (*png_ptr).zstream.avail_in > 0 as ::core::ffi::c_uint {
        png_warning(
            png_ptr,
            b"Extra compression data in IDAT\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_process_row(mut png_ptr: png_structrp) {
    let mut row_info: png_row_info = png_row_info {
        width: 0,
        rowbytes: 0,
        color_type: 0,
        bit_depth: 0,
        channels: 0,
        pixel_depth: 0,
    };
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
    if (*png_ptr).transformations != 0 as ::core::ffi::c_uint {
        png_do_read_transformations(png_ptr, &raw mut row_info);
    }
    if (*png_ptr).transformed_pixel_depth as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        (*png_ptr).transformed_pixel_depth = row_info.pixel_depth;
        if row_info.pixel_depth as ::core::ffi::c_int
            > (*png_ptr).maximum_pixel_depth as ::core::ffi::c_int
        {
            png_error(
                png_ptr,
                b"progressive row overflow\0" as *const u8 as png_const_charp,
            );
        }
    } else if (*png_ptr).transformed_pixel_depth as ::core::ffi::c_int
        != row_info.pixel_depth as ::core::ffi::c_int
    {
        png_error(
            png_ptr,
            b"internal progressive row size calculation error\0" as *const u8 as png_const_charp,
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
        match (*png_ptr).pass as ::core::ffi::c_int {
            0 => {
                let mut i: ::core::ffi::c_int = 0;
                i = 0 as ::core::ffi::c_int;
                while i < 8 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 0 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
                    i = 0 as ::core::ffi::c_int;
                    while i < 4 as ::core::ffi::c_int
                        && (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int
                    {
                        png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                    && (*png_ptr).height <= 4 as ::core::ffi::c_uint
                {
                    i = 0 as ::core::ffi::c_int;
                    while i < 2 as ::core::ffi::c_int
                        && (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                    {
                        png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                        png_read_push_finish_row(png_ptr);
                        i += 1;
                    }
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 6 as ::core::ffi::c_int
                    && (*png_ptr).height <= 4 as ::core::ffi::c_uint
                {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                }
            }
            1 => {
                let mut i_0: ::core::ffi::c_int = 0;
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < 8 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 1 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i_0 += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
                    i_0 = 0 as ::core::ffi::c_int;
                    while i_0 < 4 as ::core::ffi::c_int
                        && (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int
                    {
                        png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                        png_read_push_finish_row(png_ptr);
                        i_0 += 1;
                    }
                }
            }
            2 => {
                let mut i_1: ::core::ffi::c_int = 0;
                i_1 = 0 as ::core::ffi::c_int;
                while i_1 < 4 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i_1 += 1;
                }
                i_1 = 0 as ::core::ffi::c_int;
                while i_1 < 4 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 2 as ::core::ffi::c_int
                {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                    i_1 += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int {
                    i_1 = 0 as ::core::ffi::c_int;
                    while i_1 < 2 as ::core::ffi::c_int
                        && (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                    {
                        png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                        png_read_push_finish_row(png_ptr);
                        i_1 += 1;
                    }
                }
            }
            3 => {
                let mut i_2: ::core::ffi::c_int = 0;
                i_2 = 0 as ::core::ffi::c_int;
                while i_2 < 4 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 3 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i_2 += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int {
                    i_2 = 0 as ::core::ffi::c_int;
                    while i_2 < 2 as ::core::ffi::c_int
                        && (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                    {
                        png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                        png_read_push_finish_row(png_ptr);
                        i_2 += 1;
                    }
                }
            }
            4 => {
                let mut i_3: ::core::ffi::c_int = 0;
                i_3 = 0 as ::core::ffi::c_int;
                while i_3 < 2 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i_3 += 1;
                }
                i_3 = 0 as ::core::ffi::c_int;
                while i_3 < 2 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                    i_3 += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 6 as ::core::ffi::c_int {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                }
            }
            5 => {
                let mut i_4: ::core::ffi::c_int = 0;
                i_4 = 0 as ::core::ffi::c_int;
                while i_4 < 2 as ::core::ffi::c_int
                    && (*png_ptr).pass as ::core::ffi::c_int == 5 as ::core::ffi::c_int
                {
                    png_push_have_row(
                        png_ptr,
                        (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                    );
                    png_read_push_finish_row(png_ptr);
                    i_4 += 1;
                }
                if (*png_ptr).pass as ::core::ffi::c_int == 6 as ::core::ffi::c_int {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                }
            }
            6 | _ => {
                png_push_have_row(
                    png_ptr,
                    (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                );
                png_read_push_finish_row(png_ptr);
                if !((*png_ptr).pass as ::core::ffi::c_int != 6 as ::core::ffi::c_int) {
                    png_push_have_row(png_ptr, ::core::ptr::null_mut::<png_byte>());
                    png_read_push_finish_row(png_ptr);
                }
            }
        }
    } else {
        png_push_have_row(
            png_ptr,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
        png_read_push_finish_row(png_ptr);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_push_finish_row(mut png_ptr: png_structrp) {
    (*png_ptr).row_number = (*png_ptr).row_number.wrapping_add(1);
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        (*png_ptr).row_number = 0 as png_uint_32;
        memset(
            (*png_ptr).prev_row as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            (*png_ptr).rowbytes.wrapping_add(1 as size_t),
        );
        loop {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            if (*png_ptr).pass as ::core::ffi::c_int == 1 as ::core::ffi::c_int
                && (*png_ptr).width < 5 as ::core::ffi::c_uint
                || (*png_ptr).pass as ::core::ffi::c_int == 3 as ::core::ffi::c_int
                    && (*png_ptr).width < 3 as ::core::ffi::c_uint
                || (*png_ptr).pass as ::core::ffi::c_int == 5 as ::core::ffi::c_int
                    && (*png_ptr).width < 2 as ::core::ffi::c_uint
            {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
            }
            if (*png_ptr).pass as ::core::ffi::c_int > 7 as ::core::ffi::c_int {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_sub(1);
            }
            if (*png_ptr).pass as ::core::ffi::c_int >= 7 as ::core::ffi::c_int {
                break;
            }
            (*png_ptr).iwidth = ((*png_ptr).width as ::core::ffi::c_uint)
                .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                .wrapping_sub(1 as ::core::ffi::c_uint)
                .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                .wrapping_div(png_pass_inc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                as png_uint_32;
            if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
                != 0 as ::core::ffi::c_uint
            {
                break;
            }
            (*png_ptr).num_rows = ((*png_ptr).height as ::core::ffi::c_uint)
                .wrapping_add(png_pass_yinc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                .wrapping_sub(1 as ::core::ffi::c_uint)
                .wrapping_sub(png_pass_ystart[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                .wrapping_div(png_pass_yinc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                as png_uint_32;
            if !((*png_ptr).iwidth == 0 as ::core::ffi::c_uint
                || (*png_ptr).num_rows == 0 as ::core::ffi::c_uint)
            {
                break;
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_info(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    if (*png_ptr).info_fn.is_some() {
        Some((*png_ptr).info_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as png_structp, info_ptr as png_infop
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_end(mut png_ptr: png_structrp, mut info_ptr: png_inforp) {
    if (*png_ptr).end_fn.is_some() {
        Some((*png_ptr).end_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as png_structp, info_ptr as png_infop
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_push_have_row(mut png_ptr: png_structrp, mut row: png_bytep) {
    if (*png_ptr).row_fn.is_some() {
        Some((*png_ptr).row_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as png_structp,
            row,
            (*png_ptr).row_number,
            (*png_ptr).pass as ::core::ffi::c_int,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_progressive_combine_row(
    mut png_ptr: png_const_structrp,
    mut old_row: png_bytep,
    mut new_row: png_const_bytep,
) {
    if png_ptr.is_null() {
        return;
    }
    if !new_row.is_null() {
        png_combine_row(png_ptr, old_row, 1 as ::core::ffi::c_int);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_progressive_read_fn(
    mut png_ptr: png_structrp,
    mut progressive_ptr: png_voidp,
    mut info_fn: png_progressive_info_ptr,
    mut row_fn: png_progressive_row_ptr,
    mut end_fn: png_progressive_end_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).info_fn = info_fn;
    (*png_ptr).row_fn = row_fn;
    (*png_ptr).end_fn = end_fn;
    png_set_read_fn(
        png_ptr,
        progressive_ptr,
        Some(png_push_fill_buffer as unsafe extern "C" fn(png_structp, png_bytep, size_t) -> ()),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_progressive_ptr(mut png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return NULL_0;
    }
    return (*png_ptr).io_ptr;
}
pub const PNG_HAVE_IDAT: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_HAVE_CHUNK_HEADER: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_INTERLACE: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZSTREAM_ENDED: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
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
