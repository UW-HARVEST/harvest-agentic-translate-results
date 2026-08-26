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
    fn abs(__x: ::core::ffi::c_int) -> ::core::ffi::c_int;
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
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn floor(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn png_fixed(
        png_ptr: png_const_structrp,
        fp: ::core::ffi::c_double,
        text: png_const_charp,
    ) -> png_fixed_point;
    fn png_do_strip_channel(row_info: png_row_infop, row: png_bytep, at_start: ::core::ffi::c_int);
    fn png_do_swap(row_info: png_row_infop, row: png_bytep);
    fn png_do_packswap(row_info: png_row_infop, row: png_bytep);
    fn png_do_invert(row_info: png_row_infop, row: png_bytep);
    fn png_do_bgr(row_info: png_row_infop, row: png_bytep);
    fn png_set_rgb_coefficients(png_ptr: png_structrp);
    fn png_do_check_palette_indexes(png_ptr: png_structrp, row_info: png_row_infop);
    fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> !;
    fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_muldiv(
        res: png_fixed_point_p,
        a: png_fixed_point,
        multiplied_by: png_int_32,
        divided_by: png_int_32,
    ) -> ::core::ffi::c_int;
    fn png_reciprocal(a: png_fixed_point) -> png_fixed_point;
    fn png_reciprocal2(a: png_fixed_point, b: png_fixed_point) -> png_fixed_point;
    fn png_gamma_significant(gamma_value: png_fixed_point) -> ::core::ffi::c_int;
    fn png_gamma_correct(
        png_ptr: png_structrp,
        value: ::core::ffi::c_uint,
        gamma_value: png_fixed_point,
    ) -> png_uint_16;
    fn png_gamma_8bit_correct(value: ::core::ffi::c_uint, gamma_value: png_fixed_point)
        -> png_byte;
    fn png_build_gamma_table(png_ptr: png_structrp, bit_depth: ::core::ffi::c_int);
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
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;
pub type png_inforp = *mut png_info;
pub type png_const_colorp = *const png_color;
pub type png_const_color_16p = *const png_color_16;
pub type png_const_color_8p = *const png_color_8;
pub type png_dsortpp = *mut *mut png_dsort;
pub type png_dsort = png_dsort_struct;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct png_dsort_struct {
    pub next: *mut png_dsort_struct,
    pub left: png_byte,
    pub right: png_byte,
}
pub type png_dsortp = *mut png_dsort;
pub type png_const_uint_16pp = *const png_uint_16p;
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
pub const PNG_FP_1: ::core::ffi::c_int = 100000 as ::core::ffi::c_int;
pub const PNG_FP_MAX: png_fixed_point = 0x7fffffff as ::core::ffi::c_long as png_fixed_point;
pub const PNG_FP_MIN: png_fixed_point = -PNG_FP_MAX;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_GRAY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_MAX_PALETTE_LENGTH: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_ERROR_ACTION_NONE: ::core::ffi::c_int = 1;
pub const PNG_ERROR_ACTION_WARN: ::core::ffi::c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: ::core::ffi::c_int = 3;
pub const PNG_ALPHA_PNG: ::core::ffi::c_int = 0;
pub const PNG_ALPHA_ASSOCIATED: ::core::ffi::c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: ::core::ffi::c_int = 2;
pub const PNG_ALPHA_BROKEN: ::core::ffi::c_int = 3;
pub const PNG_DEFAULT_sRGB: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_GAMMA_MAC_18: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const PNG_GAMMA_sRGB: ::core::ffi::c_int = 220000 as ::core::ffi::c_int;
pub const PNG_BACKGROUND_GAMMA_UNKNOWN: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_BACKGROUND_GAMMA_SCREEN: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_BACKGROUND_GAMMA_FILE: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: ::core::ffi::c_int = 3;
pub const PNG_CRC_DEFAULT: ::core::ffi::c_int = 0;
pub const PNG_CRC_ERROR_QUIT: ::core::ffi::c_int = 1;
pub const PNG_CRC_WARN_DISCARD: ::core::ffi::c_int = 2;
pub const PNG_CRC_WARN_USE: ::core::ffi::c_int = 3;
pub const PNG_CRC_QUIET_USE: ::core::ffi::c_int = 4;
pub const PNG_CRC_NO_CHANGE: ::core::ffi::c_int = 5;
pub const PNG_QUANTIZE_BLUE_BITS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PNG_QUANTIZE_GREEN_BITS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PNG_QUANTIZE_RED_BITS: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_crc_action(
    mut png_ptr: png_structrp,
    mut crit_action: ::core::ffi::c_int,
    mut ancil_action: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    let mut current_block_8: u64;
    match crit_action {
        PNG_CRC_NO_CHANGE => {
            current_block_8 = 13109137661213826276;
        }
        PNG_CRC_WARN_USE => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE;
            current_block_8 = 13109137661213826276;
        }
        PNG_CRC_QUIET_USE => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
            current_block_8 = 13109137661213826276;
        }
        PNG_CRC_WARN_DISCARD => {
            png_warning(
                png_ptr,
                b"Can't discard critical data on CRC error\0" as *const u8 as png_const_charp,
            );
            current_block_8 = 2805924065016327397;
        }
        PNG_CRC_ERROR_QUIT | PNG_CRC_DEFAULT | _ => {
            current_block_8 = 2805924065016327397;
        }
    }
    match current_block_8 {
        2805924065016327397 => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_CRITICAL_MASK;
        }
        _ => {}
    }
    match ancil_action {
        PNG_CRC_NO_CHANGE => {}
        PNG_CRC_WARN_USE => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE;
        }
        PNG_CRC_QUIET_USE => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
        }
        PNG_CRC_ERROR_QUIT => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
            (*png_ptr).flags |= PNG_FLAG_CRC_ANCILLARY_NOWARN;
        }
        PNG_CRC_WARN_DISCARD | PNG_CRC_DEFAULT | _ => {
            (*png_ptr).flags &= !PNG_FLAG_CRC_ANCILLARY_MASK;
        }
    };
}
unsafe extern "C" fn png_rtran_ok(
    mut png_ptr: png_structrp,
    mut need_IHDR: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() {
        if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT != 0 as ::core::ffi::c_uint {
            png_app_error(
                png_ptr,
                b"invalid after png_start_read_image or png_read_update_info\0" as *const u8
                    as png_const_charp,
            );
        } else if need_IHDR != 0
            && (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IHDR as ::core::ffi::c_uint
                == 0 as ::core::ffi::c_uint
        {
            png_app_error(
                png_ptr,
                b"invalid before the PNG header has been read\0" as *const u8 as png_const_charp,
            );
        } else {
            (*png_ptr).flags |= PNG_FLAG_DETECT_UNINITIALIZED;
            return 1 as ::core::ffi::c_int;
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_background_fixed(
    mut png_ptr: png_structrp,
    mut background_color: png_const_color_16p,
    mut background_gamma_code: ::core::ffi::c_int,
    mut need_expand: ::core::ffi::c_int,
    mut background_gamma: png_fixed_point,
) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int
        || background_color.is_null()
    {
        return;
    }
    if background_gamma_code == PNG_BACKGROUND_GAMMA_UNKNOWN {
        png_warning(
            png_ptr,
            b"Application must supply a known background gamma\0" as *const u8 as png_const_charp,
        );
        return;
    }
    (*png_ptr).transformations |= PNG_COMPOSE | PNG_STRIP_ALPHA;
    (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
    (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
    (*png_ptr).background = *background_color;
    (*png_ptr).background_gamma = background_gamma;
    (*png_ptr).background_gamma_type = background_gamma_code as png_byte;
    if need_expand != 0 as ::core::ffi::c_int {
        (*png_ptr).transformations |= PNG_BACKGROUND_EXPAND;
    } else {
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_background(
    mut png_ptr: png_structrp,
    mut background_color: png_const_color_16p,
    mut background_gamma_code: ::core::ffi::c_int,
    mut need_expand: ::core::ffi::c_int,
    mut background_gamma: ::core::ffi::c_double,
) {
    png_set_background_fixed(
        png_ptr,
        background_color,
        background_gamma_code,
        need_expand,
        png_fixed(
            png_ptr,
            background_gamma,
            b"png_set_background\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_scale_16(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_SCALE_16_TO_8;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_strip_16(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_16_TO_8;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_strip_alpha(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_STRIP_ALPHA;
}
unsafe extern "C" fn translate_gamma_flags(
    mut output_gamma: png_fixed_point,
    mut is_screen: ::core::ffi::c_int,
) -> png_fixed_point {
    if output_gamma == PNG_DEFAULT_sRGB || output_gamma == PNG_FP_1 / PNG_DEFAULT_sRGB {
        if is_screen != 0 as ::core::ffi::c_int {
            output_gamma = PNG_GAMMA_sRGB as png_fixed_point;
        } else {
            output_gamma = PNG_GAMMA_sRGB_INVERSE as png_fixed_point;
        }
    } else if output_gamma == PNG_GAMMA_MAC_18 || output_gamma == PNG_FP_1 / PNG_GAMMA_MAC_18 {
        if is_screen != 0 as ::core::ffi::c_int {
            output_gamma = PNG_GAMMA_MAC_OLD as png_fixed_point;
        } else {
            output_gamma = PNG_GAMMA_MAC_INVERSE as png_fixed_point;
        }
    }
    return output_gamma;
}
unsafe extern "C" fn convert_gamma_value(
    mut png_ptr: png_structrp,
    mut output_gamma: ::core::ffi::c_double,
) -> png_fixed_point {
    if output_gamma > 0 as ::core::ffi::c_int as ::core::ffi::c_double
        && output_gamma < 128 as ::core::ffi::c_int as ::core::ffi::c_double
    {
        output_gamma *= PNG_FP_1 as ::core::ffi::c_double;
    }
    output_gamma = floor(output_gamma + 0.5f64);
    if output_gamma > PNG_FP_MAX as ::core::ffi::c_double
        || output_gamma < PNG_FP_MIN as ::core::ffi::c_double
    {
        png_fixed_error(png_ptr, b"gamma value\0" as *const u8 as png_const_charp);
    }
    return output_gamma as png_fixed_point;
}
unsafe extern "C" fn unsupported_gamma(
    mut png_ptr: png_structrp,
    mut gamma: png_fixed_point,
    mut warn: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if gamma < PNG_LIB_GAMMA_MIN || gamma > PNG_LIB_GAMMA_MAX {
        if warn != 0 {
            png_app_warning(png_ptr, msg.as_ptr());
        } else {
            png_app_error(png_ptr, msg.as_ptr());
        }
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
pub const msg: [::core::ffi::c_char; 29] = unsafe {
    ::core::mem::transmute::<[u8; 29], [::core::ffi::c_char; 29]>(
        *b"gamma out of supported range\0",
    )
};
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_alpha_mode_fixed(
    mut png_ptr: png_structrp,
    mut mode: ::core::ffi::c_int,
    mut output_gamma: png_fixed_point,
) {
    let mut file_gamma: png_fixed_point = 0;
    let mut compose: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    output_gamma = translate_gamma_flags(output_gamma, 1 as ::core::ffi::c_int);
    if unsupported_gamma(png_ptr, output_gamma, 0 as ::core::ffi::c_int) != 0 {
        return;
    }
    file_gamma = (*png_ptr).default_gamma;
    if file_gamma == 0 as ::core::ffi::c_int {
        file_gamma = png_reciprocal(output_gamma);
        (*png_ptr).default_gamma = file_gamma;
    }
    match mode {
        PNG_ALPHA_PNG => {
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        }
        PNG_ALPHA_ASSOCIATED => {
            compose = 1 as ::core::ffi::c_int;
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
            output_gamma = PNG_FP_1 as png_fixed_point;
        }
        PNG_ALPHA_OPTIMIZED => {
            compose = 1 as ::core::ffi::c_int;
            (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
            (*png_ptr).flags |= PNG_FLAG_OPTIMIZE_ALPHA;
        }
        PNG_ALPHA_BROKEN => {
            compose = 1 as ::core::ffi::c_int;
            (*png_ptr).transformations |= PNG_ENCODE_ALPHA;
            (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        }
        _ => {
            png_error(
                png_ptr,
                b"invalid alpha mode\0" as *const u8 as png_const_charp,
            );
        }
    }
    (*png_ptr).screen_gamma = output_gamma;
    if compose != 0 as ::core::ffi::c_int {
        memset(
            &raw mut (*png_ptr).background as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<png_color_16>() as size_t,
        );
        (*png_ptr).background_gamma = file_gamma;
        (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_FILE as png_byte;
        (*png_ptr).transformations &= !PNG_BACKGROUND_EXPAND;
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
        {
            png_error(
                png_ptr,
                b"conflicting calls to set alpha mode and background\0" as *const u8
                    as png_const_charp,
            );
        }
        (*png_ptr).transformations |= PNG_COMPOSE;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_alpha_mode(
    mut png_ptr: png_structrp,
    mut mode: ::core::ffi::c_int,
    mut output_gamma: ::core::ffi::c_double,
) {
    png_set_alpha_mode_fixed(png_ptr, mode, convert_gamma_value(png_ptr, output_gamma));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_quantize(
    mut png_ptr: png_structrp,
    mut palette: png_colorp,
    mut num_palette: ::core::ffi::c_int,
    mut maximum_colors: ::core::ffi::c_int,
    mut histogram: png_const_uint_16p,
    mut full_quantize: ::core::ffi::c_int,
) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    if palette.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_QUANTIZE;
    if full_quantize == 0 as ::core::ffi::c_int {
        let mut i: ::core::ffi::c_int = 0;
        png_free(png_ptr, (*png_ptr).quantize_index as png_voidp);
        (*png_ptr).quantize_index = ::core::ptr::null_mut::<png_byte>();
        (*png_ptr).quantize_index =
            png_malloc(png_ptr, PNG_MAX_PALETTE_LENGTH as png_alloc_size_t) as png_bytep;
        i = 0 as ::core::ffi::c_int;
        while i < PNG_MAX_PALETTE_LENGTH {
            *(*png_ptr).quantize_index.offset(i as isize) = i as png_byte;
            i += 1;
        }
    }
    if num_palette > maximum_colors {
        if !histogram.is_null() {
            let mut quantize_sort: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_0: ::core::ffi::c_int = 0;
            let mut j: ::core::ffi::c_int = 0;
            quantize_sort = png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            i_0 = 0 as ::core::ffi::c_int;
            while i_0 < num_palette {
                *quantize_sort.offset(i_0 as isize) = i_0 as png_byte;
                i_0 += 1;
            }
            i_0 = num_palette - 1 as ::core::ffi::c_int;
            while i_0 >= maximum_colors {
                let mut done: ::core::ffi::c_int = 0;
                done = 1 as ::core::ffi::c_int;
                j = 0 as ::core::ffi::c_int;
                while j < i_0 {
                    if (*histogram.offset(*quantize_sort.offset(j as isize) as isize)
                        as ::core::ffi::c_int)
                        < *histogram.offset(
                            *quantize_sort.offset((j + 1 as ::core::ffi::c_int) as isize) as isize,
                        ) as ::core::ffi::c_int
                    {
                        let mut t: png_byte = 0;
                        t = *quantize_sort.offset(j as isize);
                        *quantize_sort.offset(j as isize) =
                            *quantize_sort.offset((j + 1 as ::core::ffi::c_int) as isize);
                        *quantize_sort.offset((j + 1 as ::core::ffi::c_int) as isize) = t;
                        done = 0 as ::core::ffi::c_int;
                    }
                    j += 1;
                }
                if done != 0 as ::core::ffi::c_int {
                    break;
                }
                i_0 -= 1;
            }
            if full_quantize != 0 as ::core::ffi::c_int {
                j = num_palette;
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < maximum_colors {
                    if *quantize_sort.offset(i_0 as isize) as ::core::ffi::c_int >= maximum_colors {
                        loop {
                            j -= 1;
                            if !(*quantize_sort.offset(j as isize) as ::core::ffi::c_int
                                >= maximum_colors)
                            {
                                break;
                            }
                        }
                        *palette.offset(i_0 as isize) = *palette.offset(j as isize);
                    }
                    i_0 += 1;
                }
            } else {
                j = num_palette;
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < maximum_colors {
                    if *quantize_sort.offset(i_0 as isize) as ::core::ffi::c_int >= maximum_colors {
                        let mut tmp_color: png_color = png_color {
                            red: 0,
                            green: 0,
                            blue: 0,
                        };
                        loop {
                            j -= 1;
                            if !(*quantize_sort.offset(j as isize) as ::core::ffi::c_int
                                >= maximum_colors)
                            {
                                break;
                            }
                        }
                        tmp_color = *palette.offset(j as isize);
                        *palette.offset(j as isize) = *palette.offset(i_0 as isize);
                        *palette.offset(i_0 as isize) = tmp_color;
                        *(*png_ptr).quantize_index.offset(j as isize) = i_0 as png_byte;
                        *(*png_ptr).quantize_index.offset(i_0 as isize) = j as png_byte;
                    }
                    i_0 += 1;
                }
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < num_palette {
                    if *(*png_ptr).quantize_index.offset(i_0 as isize) as ::core::ffi::c_int
                        >= maximum_colors
                    {
                        let mut min_d: ::core::ffi::c_int = 0;
                        let mut k: ::core::ffi::c_int = 0;
                        let mut min_k: ::core::ffi::c_int = 0;
                        let mut d_index: ::core::ffi::c_int = 0;
                        d_index =
                            *(*png_ptr).quantize_index.offset(i_0 as isize) as ::core::ffi::c_int;
                        min_d = abs(
                            (*palette.offset(d_index as isize)).red as ::core::ffi::c_int
                                - (*palette.offset(0 as ::core::ffi::c_int as isize)).red
                                    as ::core::ffi::c_int,
                        ) + abs((*palette.offset(d_index as isize)).green
                            as ::core::ffi::c_int
                            - (*palette.offset(0 as ::core::ffi::c_int as isize)).green
                                as ::core::ffi::c_int)
                            + abs(
                                (*palette.offset(d_index as isize)).blue as ::core::ffi::c_int
                                    - (*palette.offset(0 as ::core::ffi::c_int as isize)).blue
                                        as ::core::ffi::c_int,
                            );
                        k = 1 as ::core::ffi::c_int;
                        min_k = 0 as ::core::ffi::c_int;
                        while k < maximum_colors {
                            let mut d: ::core::ffi::c_int = 0;
                            d = abs(
                                (*palette.offset(d_index as isize)).red as ::core::ffi::c_int
                                    - (*palette.offset(k as isize)).red as ::core::ffi::c_int,
                            ) + abs((*palette.offset(d_index as isize)).green
                                as ::core::ffi::c_int
                                - (*palette.offset(k as isize)).green as ::core::ffi::c_int)
                                + abs((*palette.offset(d_index as isize)).blue
                                    as ::core::ffi::c_int
                                    - (*palette.offset(k as isize)).blue as ::core::ffi::c_int);
                            if d < min_d {
                                min_d = d;
                                min_k = k;
                            }
                            k += 1;
                        }
                        *(*png_ptr).quantize_index.offset(i_0 as isize) = min_k as png_byte;
                    }
                    i_0 += 1;
                }
            }
            png_free(png_ptr, quantize_sort as png_voidp);
        } else {
            let mut i_1: ::core::ffi::c_int = 0;
            let mut max_d: ::core::ffi::c_int = 0;
            let mut num_new_palette: ::core::ffi::c_int = 0;
            let mut t_0: png_dsortp = ::core::ptr::null_mut::<png_dsort>();
            let mut hash: png_dsortpp = ::core::ptr::null_mut::<*mut png_dsort>();
            t_0 = ::core::ptr::null_mut::<png_dsort>();
            (*png_ptr).index_to_palette =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            (*png_ptr).palette_to_index =
                png_malloc(png_ptr, num_palette as png_alloc_size_t) as png_bytep;
            i_1 = 0 as ::core::ffi::c_int;
            while i_1 < num_palette {
                *(*png_ptr).index_to_palette.offset(i_1 as isize) = i_1 as png_byte;
                *(*png_ptr).palette_to_index.offset(i_1 as isize) = i_1 as png_byte;
                i_1 += 1;
            }
            hash = png_calloc(
                png_ptr,
                (769 as usize).wrapping_mul(::core::mem::size_of::<png_dsortp>() as usize),
            ) as png_dsortpp;
            num_new_palette = num_palette;
            max_d = 96 as ::core::ffi::c_int;
            while num_new_palette > maximum_colors {
                i_1 = 0 as ::core::ffi::c_int;
                while i_1 < num_new_palette - 1 as ::core::ffi::c_int {
                    let mut j_0: ::core::ffi::c_int = 0;
                    j_0 = i_1 + 1 as ::core::ffi::c_int;
                    while j_0 < num_new_palette {
                        let mut d_0: ::core::ffi::c_int = 0;
                        d_0 = abs((*palette.offset(i_1 as isize)).red as ::core::ffi::c_int
                            - (*palette.offset(j_0 as isize)).red as ::core::ffi::c_int)
                            + abs((*palette.offset(i_1 as isize)).green as ::core::ffi::c_int
                                - (*palette.offset(j_0 as isize)).green as ::core::ffi::c_int)
                            + abs((*palette.offset(i_1 as isize)).blue as ::core::ffi::c_int
                                - (*palette.offset(j_0 as isize)).blue as ::core::ffi::c_int);
                        if d_0 <= max_d {
                            t_0 = png_malloc_warn(png_ptr, ::core::mem::size_of::<png_dsort>())
                                as png_dsortp;
                            if t_0.is_null() {
                                break;
                            }
                            (*t_0).next = *hash.offset(d_0 as isize) as *mut png_dsort_struct;
                            (*t_0).left = *(*png_ptr).palette_to_index.offset(i_1 as isize);
                            (*t_0).right = *(*png_ptr).palette_to_index.offset(j_0 as isize);
                            let ref mut fresh0 = *hash.offset(d_0 as isize);
                            *fresh0 = t_0 as *mut png_dsort;
                        }
                        j_0 += 1;
                    }
                    if t_0.is_null() {
                        break;
                    }
                    i_1 += 1;
                }
                if !t_0.is_null() {
                    i_1 = 0 as ::core::ffi::c_int;
                    while i_1 <= max_d {
                        if !(*hash.offset(i_1 as isize)).is_null() {
                            let mut p: png_dsortp = ::core::ptr::null_mut::<png_dsort>();
                            p = *hash.offset(i_1 as isize) as png_dsortp;
                            while !p.is_null() {
                                if (*(*png_ptr).index_to_palette.offset((*p).left as isize)
                                    as ::core::ffi::c_int)
                                    < num_new_palette
                                    && (*(*png_ptr).index_to_palette.offset((*p).right as isize)
                                        as ::core::ffi::c_int)
                                        < num_new_palette
                                {
                                    let mut j_1: ::core::ffi::c_int = 0;
                                    let mut next_j: ::core::ffi::c_int = 0;
                                    if num_new_palette & 0x1 as ::core::ffi::c_int != 0 {
                                        j_1 = (*p).left as ::core::ffi::c_int;
                                        next_j = (*p).right as ::core::ffi::c_int;
                                    } else {
                                        j_1 = (*p).right as ::core::ffi::c_int;
                                        next_j = (*p).left as ::core::ffi::c_int;
                                    }
                                    num_new_palette -= 1;
                                    *palette.offset(
                                        *(*png_ptr).index_to_palette.offset(j_1 as isize) as isize,
                                    ) = *palette.offset(num_new_palette as isize);
                                    if full_quantize == 0 as ::core::ffi::c_int {
                                        let mut k_0: ::core::ffi::c_int = 0;
                                        k_0 = 0 as ::core::ffi::c_int;
                                        while k_0 < num_palette {
                                            if *(*png_ptr).quantize_index.offset(k_0 as isize)
                                                as ::core::ffi::c_int
                                                == *(*png_ptr).index_to_palette.offset(j_1 as isize)
                                                    as ::core::ffi::c_int
                                            {
                                                *(*png_ptr).quantize_index.offset(k_0 as isize) =
                                                    *(*png_ptr)
                                                        .index_to_palette
                                                        .offset(next_j as isize);
                                            }
                                            if *(*png_ptr).quantize_index.offset(k_0 as isize)
                                                as ::core::ffi::c_int
                                                == num_new_palette
                                            {
                                                *(*png_ptr).quantize_index.offset(k_0 as isize) =
                                                    *(*png_ptr)
                                                        .index_to_palette
                                                        .offset(j_1 as isize);
                                            }
                                            k_0 += 1;
                                        }
                                    }
                                    *(*png_ptr).index_to_palette.offset(
                                        *(*png_ptr)
                                            .palette_to_index
                                            .offset(num_new_palette as isize)
                                            as isize,
                                    ) = *(*png_ptr).index_to_palette.offset(j_1 as isize);
                                    *(*png_ptr).palette_to_index.offset(
                                        *(*png_ptr).index_to_palette.offset(j_1 as isize) as isize,
                                    ) = *(*png_ptr)
                                        .palette_to_index
                                        .offset(num_new_palette as isize);
                                    *(*png_ptr).index_to_palette.offset(j_1 as isize) =
                                        num_new_palette as png_byte;
                                    *(*png_ptr).palette_to_index.offset(num_new_palette as isize) =
                                        j_1 as png_byte;
                                }
                                if num_new_palette <= maximum_colors {
                                    break;
                                }
                                p = (*p).next as png_dsortp;
                            }
                            if num_new_palette <= maximum_colors {
                                break;
                            }
                        }
                        i_1 += 1;
                    }
                }
                i_1 = 0 as ::core::ffi::c_int;
                while i_1 < 769 as ::core::ffi::c_int {
                    if !(*hash.offset(i_1 as isize)).is_null() {
                        let mut p_0: png_dsortp = *hash.offset(i_1 as isize) as png_dsortp;
                        while !p_0.is_null() {
                            t_0 = (*p_0).next as png_dsortp;
                            png_free(png_ptr, p_0 as png_voidp);
                            p_0 = t_0;
                        }
                    }
                    let ref mut fresh1 = *hash.offset(i_1 as isize);
                    *fresh1 = ::core::ptr::null_mut::<png_dsort>();
                    i_1 += 1;
                }
                max_d += 96 as ::core::ffi::c_int;
            }
            png_free(png_ptr, hash as png_voidp);
            png_free(png_ptr, (*png_ptr).palette_to_index as png_voidp);
            png_free(png_ptr, (*png_ptr).index_to_palette as png_voidp);
            (*png_ptr).palette_to_index = ::core::ptr::null_mut::<png_byte>();
            (*png_ptr).index_to_palette = ::core::ptr::null_mut::<png_byte>();
        }
        num_palette = maximum_colors;
    }
    if (*png_ptr).palette.is_null() {
        (*png_ptr).palette = png_calloc(
            png_ptr,
            (256 as png_alloc_size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as png_alloc_size_t),
        ) as png_colorp;
        memcpy(
            (*png_ptr).palette as *mut ::core::ffi::c_void,
            palette as *const ::core::ffi::c_void,
            (num_palette as ::core::ffi::c_uint as size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as size_t),
        );
    }
    (*png_ptr).num_palette = num_palette as png_uint_16;
    if full_quantize != 0 as ::core::ffi::c_int {
        let mut i_2: ::core::ffi::c_int = 0;
        let mut distance: png_bytep = ::core::ptr::null_mut::<png_byte>();
        let mut total_bits: ::core::ffi::c_int =
            PNG_QUANTIZE_RED_BITS + PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS;
        let mut num_red: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PNG_QUANTIZE_RED_BITS;
        let mut num_green: ::core::ffi::c_int =
            (1 as ::core::ffi::c_int) << PNG_QUANTIZE_GREEN_BITS;
        let mut num_blue: ::core::ffi::c_int = (1 as ::core::ffi::c_int) << PNG_QUANTIZE_BLUE_BITS;
        let mut num_entries: size_t = (1 as ::core::ffi::c_int as size_t) << total_bits;
        (*png_ptr).palette_lookup = png_calloc(png_ptr, num_entries) as png_bytep;
        distance = png_malloc(png_ptr, num_entries) as png_bytep;
        memset(
            distance as *mut ::core::ffi::c_void,
            0xff as ::core::ffi::c_int,
            num_entries,
        );
        i_2 = 0 as ::core::ffi::c_int;
        while i_2 < num_palette {
            let mut ir: ::core::ffi::c_int = 0;
            let mut ig: ::core::ffi::c_int = 0;
            let mut ib: ::core::ffi::c_int = 0;
            let mut r: ::core::ffi::c_int = (*palette.offset(i_2 as isize)).red
                as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_RED_BITS;
            let mut g: ::core::ffi::c_int = (*palette.offset(i_2 as isize)).green
                as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_GREEN_BITS;
            let mut b: ::core::ffi::c_int = (*palette.offset(i_2 as isize)).blue
                as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_BLUE_BITS;
            ir = 0 as ::core::ffi::c_int;
            while ir < num_red {
                let mut dr: ::core::ffi::c_int = if ir > r { ir - r } else { r - ir };
                let mut index_r: ::core::ffi::c_int =
                    ir << PNG_QUANTIZE_BLUE_BITS + PNG_QUANTIZE_GREEN_BITS;
                ig = 0 as ::core::ffi::c_int;
                while ig < num_green {
                    let mut dg: ::core::ffi::c_int = if ig > g { ig - g } else { g - ig };
                    let mut dt: ::core::ffi::c_int = dr + dg;
                    let mut dm: ::core::ffi::c_int = if dr > dg { dr } else { dg };
                    let mut index_g: ::core::ffi::c_int = index_r | ig << PNG_QUANTIZE_BLUE_BITS;
                    ib = 0 as ::core::ffi::c_int;
                    while ib < num_blue {
                        let mut d_index_0: ::core::ffi::c_int = index_g | ib;
                        let mut db: ::core::ffi::c_int = if ib > b { ib - b } else { b - ib };
                        let mut dmax: ::core::ffi::c_int = if dm > db { dm } else { db };
                        let mut d_1: ::core::ffi::c_int = dmax + dt + db;
                        if d_1 < *distance.offset(d_index_0 as isize) as ::core::ffi::c_int {
                            *distance.offset(d_index_0 as isize) = d_1 as png_byte;
                            *(*png_ptr).palette_lookup.offset(d_index_0 as isize) = i_2 as png_byte;
                        }
                        ib += 1;
                    }
                    ig += 1;
                }
                ir += 1;
            }
            i_2 += 1;
        }
        png_free(png_ptr, distance as png_voidp);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gamma_fixed(
    mut png_ptr: png_structrp,
    mut scrn_gamma: png_fixed_point,
    mut file_gamma: png_fixed_point,
) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    scrn_gamma = translate_gamma_flags(scrn_gamma, 1 as ::core::ffi::c_int);
    file_gamma = translate_gamma_flags(file_gamma, 0 as ::core::ffi::c_int);
    if file_gamma <= 0 as ::core::ffi::c_int {
        png_app_error(
            png_ptr,
            b"invalid file gamma in png_set_gamma\0" as *const u8 as png_const_charp,
        );
    }
    if scrn_gamma <= 0 as ::core::ffi::c_int {
        png_app_error(
            png_ptr,
            b"invalid screen gamma in png_set_gamma\0" as *const u8 as png_const_charp,
        );
    }
    if unsupported_gamma(png_ptr, file_gamma, 1 as ::core::ffi::c_int) != 0
        || unsupported_gamma(png_ptr, scrn_gamma, 1 as ::core::ffi::c_int) != 0
    {
        return;
    }
    (*png_ptr).file_gamma = file_gamma;
    (*png_ptr).screen_gamma = scrn_gamma;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gamma(
    mut png_ptr: png_structrp,
    mut scrn_gamma: ::core::ffi::c_double,
    mut file_gamma: ::core::ffi::c_double,
) {
    png_set_gamma_fixed(
        png_ptr,
        convert_gamma_value(png_ptr, scrn_gamma),
        convert_gamma_value(png_ptr, file_gamma),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_palette_to_rgb(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand_gray_1_2_4_to_8(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_EXPAND;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tRNS_to_alpha(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_EXPAND | PNG_EXPAND_tRNS;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_expand_16(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    (*png_ptr).transformations |= PNG_EXPAND_16 | PNG_EXPAND | PNG_EXPAND_tRNS;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gray_to_rgb(mut png_ptr: png_structrp) {
    if png_rtran_ok(png_ptr, 0 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    png_set_expand_gray_1_2_4_to_8(png_ptr);
    (*png_ptr).transformations |= PNG_GRAY_TO_RGB;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_to_gray_fixed(
    mut png_ptr: png_structrp,
    mut error_action: ::core::ffi::c_int,
    mut red: png_fixed_point,
    mut green: png_fixed_point,
) {
    if png_rtran_ok(png_ptr, 1 as ::core::ffi::c_int) == 0 as ::core::ffi::c_int {
        return;
    }
    match error_action {
        PNG_ERROR_ACTION_NONE => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY;
        }
        PNG_ERROR_ACTION_WARN => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY_WARN;
        }
        PNG_ERROR_ACTION_ERROR => {
            (*png_ptr).transformations |= PNG_RGB_TO_GRAY_ERR;
        }
        _ => {
            png_error(
                png_ptr,
                b"invalid error action to rgb_to_gray\0" as *const u8 as png_const_charp,
            );
        }
    }
    if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        (*png_ptr).transformations |= PNG_EXPAND;
    }
    if red >= 0 as ::core::ffi::c_int && green >= 0 as ::core::ffi::c_int && red + green <= PNG_FP_1
    {
        let mut red_int: png_uint_16 = 0;
        let mut green_int: png_uint_16 = 0;
        red_int = (red as ::core::ffi::c_uint)
            .wrapping_mul(32768 as ::core::ffi::c_uint)
            .wrapping_div(100000 as ::core::ffi::c_int as ::core::ffi::c_uint)
            as png_uint_16;
        green_int = (green as ::core::ffi::c_uint)
            .wrapping_mul(32768 as ::core::ffi::c_uint)
            .wrapping_div(100000 as ::core::ffi::c_int as ::core::ffi::c_uint)
            as png_uint_16;
        (*png_ptr).rgb_to_gray_red_coeff = red_int;
        (*png_ptr).rgb_to_gray_green_coeff = green_int;
        (*png_ptr).rgb_to_gray_coefficients_set = 1 as png_byte;
    } else if red >= 0 as ::core::ffi::c_int && green >= 0 as ::core::ffi::c_int {
        png_app_warning(
            png_ptr,
            b"ignoring out of range rgb_to_gray coefficients\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_to_gray(
    mut png_ptr: png_structrp,
    mut error_action: ::core::ffi::c_int,
    mut red: ::core::ffi::c_double,
    mut green: ::core::ffi::c_double,
) {
    png_set_rgb_to_gray_fixed(
        png_ptr,
        error_action,
        png_fixed(
            png_ptr,
            red,
            b"rgb to gray red coefficient\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green,
            b"rgb to gray green coefficient\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_user_transform_fn(
    mut png_ptr: png_structrp,
    mut read_user_transform_fn: png_user_transform_ptr,
) {
    (*png_ptr).transformations |= PNG_USER_TRANSFORM;
    (*png_ptr).read_user_transform_fn = read_user_transform_fn;
}
unsafe extern "C" fn png_gamma_threshold(
    mut screen_gamma: png_fixed_point,
    mut file_gamma: png_fixed_point,
) -> ::core::ffi::c_int {
    let mut gtest: png_fixed_point = 0;
    return (png_muldiv(
        &raw mut gtest,
        screen_gamma,
        file_gamma as png_int_32,
        PNG_FP_1,
    ) == 0
        || png_gamma_significant(gtest) != 0) as ::core::ffi::c_int;
}
unsafe extern "C" fn png_init_palette_transformations(mut png_ptr: png_structrp) {
    let mut input_has_alpha: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut input_has_transparency: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
        let mut i: ::core::ffi::c_int = 0;
        i = 0 as ::core::ffi::c_int;
        while i < (*png_ptr).num_trans as ::core::ffi::c_int {
            if !(*(*png_ptr).trans_alpha.offset(i as isize) as ::core::ffi::c_int
                == 255 as ::core::ffi::c_int)
            {
                if *(*png_ptr).trans_alpha.offset(i as isize) as ::core::ffi::c_int
                    == 0 as ::core::ffi::c_int
                {
                    input_has_transparency = 1 as ::core::ffi::c_int;
                } else {
                    input_has_transparency = 1 as ::core::ffi::c_int;
                    input_has_alpha = 1 as ::core::ffi::c_int;
                    break;
                }
            }
            i += 1;
        }
    }
    if input_has_alpha == 0 as ::core::ffi::c_int {
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        if input_has_transparency == 0 as ::core::ffi::c_int {
            (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BACKGROUND_EXPAND
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND
            != 0 as ::core::ffi::c_uint
    {
        (*png_ptr).background.red = (*(*png_ptr)
            .palette
            .offset((*png_ptr).background.index as isize))
        .red as png_uint_16;
        (*png_ptr).background.green = (*(*png_ptr)
            .palette
            .offset((*png_ptr).background.index as isize))
        .green as png_uint_16;
        (*png_ptr).background.blue = (*(*png_ptr)
            .palette
            .offset((*png_ptr).background.index as isize))
        .blue as png_uint_16;
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INVERT_ALPHA
            != 0 as ::core::ffi::c_uint
        {
            if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_tRNS
                == 0 as ::core::ffi::c_uint
            {
                let mut i_0: ::core::ffi::c_int = 0;
                let mut istop: ::core::ffi::c_int = (*png_ptr).num_trans as ::core::ffi::c_int;
                i_0 = 0 as ::core::ffi::c_int;
                while i_0 < istop {
                    *(*png_ptr).trans_alpha.offset(i_0 as isize) = (255 as ::core::ffi::c_int
                        - *(*png_ptr).trans_alpha.offset(i_0 as isize) as ::core::ffi::c_int)
                        as png_byte;
                    i_0 += 1;
                }
            }
        }
    }
}
unsafe extern "C" fn png_init_rgb_transformations(mut png_ptr: png_structrp) {
    let mut input_has_alpha: ::core::ffi::c_int =
        ((*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
            != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
    let mut input_has_transparency: ::core::ffi::c_int =
        ((*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int)
            as ::core::ffi::c_int;
    if input_has_alpha == 0 as ::core::ffi::c_int {
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        if input_has_transparency == 0 as ::core::ffi::c_int {
            (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_BACKGROUND_EXPAND);
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BACKGROUND_EXPAND
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND
            != 0 as ::core::ffi::c_uint
        && (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            == 0 as ::core::ffi::c_int
    {
        let mut gray: ::core::ffi::c_int = (*png_ptr).background.gray as ::core::ffi::c_int;
        let mut trans_gray: ::core::ffi::c_int = (*png_ptr).trans_color.gray as ::core::ffi::c_int;
        match (*png_ptr).bit_depth as ::core::ffi::c_int {
            1 => {
                gray *= 0xff as ::core::ffi::c_int;
                trans_gray *= 0xff as ::core::ffi::c_int;
            }
            2 => {
                gray *= 0x55 as ::core::ffi::c_int;
                trans_gray *= 0x55 as ::core::ffi::c_int;
            }
            4 => {
                gray *= 0x11 as ::core::ffi::c_int;
                trans_gray *= 0x11 as ::core::ffi::c_int;
            }
            8 | 16 | _ => {}
        }
        (*png_ptr).background.blue = gray as png_uint_16;
        (*png_ptr).background.green = (*png_ptr).background.blue;
        (*png_ptr).background.red = (*png_ptr).background.green;
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_tRNS
            == 0 as ::core::ffi::c_uint
        {
            (*png_ptr).trans_color.blue = trans_gray as png_uint_16;
            (*png_ptr).trans_color.green = (*png_ptr).trans_color.blue;
            (*png_ptr).trans_color.red = (*png_ptr).trans_color.green;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_resolve_file_gamma(
    mut png_ptr: png_const_structrp,
) -> png_fixed_point {
    let mut file_gamma: png_fixed_point = 0;
    file_gamma = (*png_ptr).file_gamma;
    if file_gamma != 0 as ::core::ffi::c_int {
        return file_gamma;
    }
    file_gamma = (*png_ptr).chunk_gamma;
    if file_gamma != 0 as ::core::ffi::c_int {
        return file_gamma;
    }
    file_gamma = (*png_ptr).default_gamma;
    if file_gamma != 0 as ::core::ffi::c_int {
        return file_gamma;
    }
    if (*png_ptr).screen_gamma != 0 as ::core::ffi::c_int {
        file_gamma = png_reciprocal((*png_ptr).screen_gamma);
    }
    return file_gamma;
}
unsafe extern "C" fn png_init_gamma_values(mut png_ptr: png_structrp) -> ::core::ffi::c_int {
    let mut gamma_correction: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut file_gamma: png_fixed_point = 0;
    let mut screen_gamma: png_fixed_point = 0;
    file_gamma = png_resolve_file_gamma(png_ptr);
    screen_gamma = (*png_ptr).screen_gamma;
    if file_gamma > 0 as ::core::ffi::c_int {
        if screen_gamma > 0 as ::core::ffi::c_int {
            gamma_correction = png_gamma_threshold(file_gamma, screen_gamma);
        } else {
            screen_gamma = png_reciprocal(file_gamma);
        }
    } else {
        screen_gamma = PNG_FP_1 as png_fixed_point;
        file_gamma = screen_gamma;
    }
    (*png_ptr).file_gamma = file_gamma;
    (*png_ptr).screen_gamma = screen_gamma;
    return gamma_correction;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_init_read_transformations(mut png_ptr: png_structrp) {
    if png_init_gamma_values(png_ptr) != 0 as ::core::ffi::c_int {
        (*png_ptr).transformations |= PNG_GAMMA;
    } else {
        (*png_ptr).transformations &= !PNG_GAMMA;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_STRIP_ALPHA
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            == 0 as ::core::ffi::c_uint
    {
        (*png_ptr).transformations &= !(PNG_BACKGROUND_EXPAND | PNG_ENCODE_ALPHA | PNG_EXPAND_tRNS);
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
        (*png_ptr).num_trans = 0 as png_uint_16;
    }
    if png_gamma_significant((*png_ptr).screen_gamma) == 0 as ::core::ffi::c_int {
        (*png_ptr).transformations &= !PNG_ENCODE_ALPHA;
        (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
        != 0 as ::core::ffi::c_uint
    {
        png_set_rgb_coefficients(png_ptr);
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BACKGROUND_EXPAND
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            == 0 as ::core::ffi::c_int
        {
            (*png_ptr).mode |= PNG_BACKGROUND_IS_GRAY;
        }
    } else if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GRAY_TO_RGB
            != 0 as ::core::ffi::c_uint
        {
            if (*png_ptr).background.red as ::core::ffi::c_int
                == (*png_ptr).background.green as ::core::ffi::c_int
                && (*png_ptr).background.red as ::core::ffi::c_int
                    == (*png_ptr).background.blue as ::core::ffi::c_int
            {
                (*png_ptr).mode |= PNG_BACKGROUND_IS_GRAY;
                (*png_ptr).background.gray = (*png_ptr).background.red;
            }
        }
    }
    if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        png_init_palette_transformations(png_ptr);
    } else {
        png_init_rgb_transformations(png_ptr);
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_16 != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BACKGROUND_EXPAND
            == 0 as ::core::ffi::c_uint
        && (*png_ptr).bit_depth as ::core::ffi::c_int != 16 as ::core::ffi::c_int
    {
        (*png_ptr).background.red = (((*png_ptr).background.red as ::core::ffi::c_uint)
            .wrapping_mul(255 as ::core::ffi::c_uint)
            .wrapping_add(32895 as ::core::ffi::c_uint)
            >> 16 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.green = (((*png_ptr).background.green as ::core::ffi::c_uint)
            .wrapping_mul(255 as ::core::ffi::c_uint)
            .wrapping_add(32895 as ::core::ffi::c_uint)
            >> 16 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.blue = (((*png_ptr).background.blue as ::core::ffi::c_uint)
            .wrapping_mul(255 as ::core::ffi::c_uint)
            .wrapping_add(32895 as ::core::ffi::c_uint)
            >> 16 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.gray = (((*png_ptr).background.gray as ::core::ffi::c_uint)
            .wrapping_mul(255 as ::core::ffi::c_uint)
            .wrapping_add(32895 as ::core::ffi::c_uint)
            >> 16 as ::core::ffi::c_int) as png_uint_16;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BACKGROUND_EXPAND
            == 0 as ::core::ffi::c_uint
        && (*png_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int
    {
        (*png_ptr).background.red = ((*png_ptr).background.red as ::core::ffi::c_int
            * 257 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.green = ((*png_ptr).background.green as ::core::ffi::c_int
            * 257 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.blue = ((*png_ptr).background.blue as ::core::ffi::c_int
            * 257 as ::core::ffi::c_int) as png_uint_16;
        (*png_ptr).background.gray = ((*png_ptr).background.gray as ::core::ffi::c_int
            * 257 as ::core::ffi::c_int) as png_uint_16;
    }
    (*png_ptr).background_1 = (*png_ptr).background;
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GAMMA != 0 as ::core::ffi::c_uint
        || (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
            != 0 as ::core::ffi::c_uint
            && (png_gamma_significant((*png_ptr).file_gamma) != 0 as ::core::ffi::c_int
                || png_gamma_significant((*png_ptr).screen_gamma) != 0 as ::core::ffi::c_int)
        || (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
            && (png_gamma_significant((*png_ptr).file_gamma) != 0 as ::core::ffi::c_int
                || png_gamma_significant((*png_ptr).screen_gamma) != 0 as ::core::ffi::c_int
                || (*png_ptr).background_gamma_type as ::core::ffi::c_int
                    == PNG_BACKGROUND_GAMMA_UNIQUE
                    && png_gamma_significant((*png_ptr).background_gamma)
                        != 0 as ::core::ffi::c_int)
        || (*png_ptr).transformations as ::core::ffi::c_uint & PNG_ENCODE_ALPHA
            != 0 as ::core::ffi::c_uint
            && png_gamma_significant((*png_ptr).screen_gamma) != 0 as ::core::ffi::c_int
    {
        png_build_gamma_table(png_ptr, (*png_ptr).bit_depth as ::core::ffi::c_int);
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
        {
            if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
                != 0 as ::core::ffi::c_uint
            {
                png_warning(
                    png_ptr,
                    b"libpng does not support gamma+background+rgb_to_gray\0" as *const u8
                        as png_const_charp,
                );
            }
            if ((*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE)
                as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
            {
                let mut back: png_color = png_color {
                    red: 0,
                    green: 0,
                    blue: 0,
                };
                let mut back_1: png_color = png_color {
                    red: 0,
                    green: 0,
                    blue: 0,
                };
                let mut palette: png_colorp = (*png_ptr).palette;
                let mut num_palette: ::core::ffi::c_int =
                    (*png_ptr).num_palette as ::core::ffi::c_int;
                let mut i: ::core::ffi::c_int = 0;
                if (*png_ptr).background_gamma_type as ::core::ffi::c_int
                    == PNG_BACKGROUND_GAMMA_FILE
                {
                    back.red = *(*png_ptr)
                        .gamma_table
                        .offset((*png_ptr).background.red as isize);
                    back.green = *(*png_ptr)
                        .gamma_table
                        .offset((*png_ptr).background.green as isize);
                    back.blue = *(*png_ptr)
                        .gamma_table
                        .offset((*png_ptr).background.blue as isize);
                    back_1.red = *(*png_ptr)
                        .gamma_to_1
                        .offset((*png_ptr).background.red as isize);
                    back_1.green = *(*png_ptr)
                        .gamma_to_1
                        .offset((*png_ptr).background.green as isize);
                    back_1.blue = *(*png_ptr)
                        .gamma_to_1
                        .offset((*png_ptr).background.blue as isize);
                } else {
                    let mut g: png_fixed_point = 0;
                    let mut gs: png_fixed_point = 0;
                    match (*png_ptr).background_gamma_type as ::core::ffi::c_int {
                        PNG_BACKGROUND_GAMMA_SCREEN => {
                            g = (*png_ptr).screen_gamma;
                            gs = PNG_FP_1 as png_fixed_point;
                        }
                        PNG_BACKGROUND_GAMMA_FILE => {
                            g = png_reciprocal((*png_ptr).file_gamma);
                            gs = png_reciprocal2((*png_ptr).file_gamma, (*png_ptr).screen_gamma);
                        }
                        PNG_BACKGROUND_GAMMA_UNIQUE => {
                            g = png_reciprocal((*png_ptr).background_gamma);
                            gs = png_reciprocal2(
                                (*png_ptr).background_gamma,
                                (*png_ptr).screen_gamma,
                            );
                        }
                        _ => {
                            g = PNG_FP_1 as png_fixed_point;
                            gs = PNG_FP_1 as png_fixed_point;
                        }
                    }
                    if png_gamma_significant(gs) != 0 as ::core::ffi::c_int {
                        back.red = png_gamma_8bit_correct(
                            (*png_ptr).background.red as ::core::ffi::c_uint,
                            gs,
                        );
                        back.green = png_gamma_8bit_correct(
                            (*png_ptr).background.green as ::core::ffi::c_uint,
                            gs,
                        );
                        back.blue = png_gamma_8bit_correct(
                            (*png_ptr).background.blue as ::core::ffi::c_uint,
                            gs,
                        );
                    } else {
                        back.red = (*png_ptr).background.red as png_byte;
                        back.green = (*png_ptr).background.green as png_byte;
                        back.blue = (*png_ptr).background.blue as png_byte;
                    }
                    if png_gamma_significant(g) != 0 as ::core::ffi::c_int {
                        back_1.red = png_gamma_8bit_correct(
                            (*png_ptr).background.red as ::core::ffi::c_uint,
                            g,
                        );
                        back_1.green = png_gamma_8bit_correct(
                            (*png_ptr).background.green as ::core::ffi::c_uint,
                            g,
                        );
                        back_1.blue = png_gamma_8bit_correct(
                            (*png_ptr).background.blue as ::core::ffi::c_uint,
                            g,
                        );
                    } else {
                        back_1.red = (*png_ptr).background.red as png_byte;
                        back_1.green = (*png_ptr).background.green as png_byte;
                        back_1.blue = (*png_ptr).background.blue as png_byte;
                    }
                }
                i = 0 as ::core::ffi::c_int;
                while i < num_palette {
                    if i < (*png_ptr).num_trans as ::core::ffi::c_int
                        && *(*png_ptr).trans_alpha.offset(i as isize) as ::core::ffi::c_int
                            != 0xff as ::core::ffi::c_int
                    {
                        if *(*png_ptr).trans_alpha.offset(i as isize) as ::core::ffi::c_int
                            == 0 as ::core::ffi::c_int
                        {
                            *palette.offset(i as isize) = back;
                        } else if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_OPTIMIZE_ALPHA
                            != 0 as ::core::ffi::c_uint
                        {
                            let mut component: png_uint_32 = 0;
                            component = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).red as isize)
                                as png_uint_32;
                            component = (component as ::core::ffi::c_uint)
                                .wrapping_mul(*(*png_ptr).trans_alpha.offset(i as isize)
                                    as ::core::ffi::c_uint)
                                .wrapping_add(128 as ::core::ffi::c_uint)
                                .wrapping_div(255 as ::core::ffi::c_uint)
                                as png_uint_32;
                            (*palette.offset(i as isize)).red =
                                *(*png_ptr).gamma_from_1.offset(component as isize);
                            component = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).green as isize)
                                as png_uint_32;
                            component = (component as ::core::ffi::c_uint)
                                .wrapping_mul(*(*png_ptr).trans_alpha.offset(i as isize)
                                    as ::core::ffi::c_uint)
                                .wrapping_add(128 as ::core::ffi::c_uint)
                                .wrapping_div(255 as ::core::ffi::c_uint)
                                as png_uint_32;
                            (*palette.offset(i as isize)).green =
                                *(*png_ptr).gamma_from_1.offset(component as isize);
                            component = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).blue as isize)
                                as png_uint_32;
                            component = (component as ::core::ffi::c_uint)
                                .wrapping_mul(*(*png_ptr).trans_alpha.offset(i as isize)
                                    as ::core::ffi::c_uint)
                                .wrapping_add(128 as ::core::ffi::c_uint)
                                .wrapping_div(255 as ::core::ffi::c_uint)
                                as png_uint_32;
                            (*palette.offset(i as isize)).blue =
                                *(*png_ptr).gamma_from_1.offset(component as isize);
                        } else {
                            let mut v: png_byte = 0;
                            let mut w: png_byte = 0;
                            v = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).red as isize);
                            let mut temp: png_uint_16 = (v as png_uint_16 as ::core::ffi::c_int
                                * *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                    as ::core::ffi::c_int
                                + back_1.red as png_uint_16 as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                            as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w = (temp as ::core::ffi::c_int
                                + (temp as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            (*palette.offset(i as isize)).red =
                                *(*png_ptr).gamma_from_1.offset(w as isize);
                            v = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).green as isize);
                            let mut temp_0: png_uint_16 = (v as png_uint_16 as ::core::ffi::c_int
                                * *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                    as ::core::ffi::c_int
                                + back_1.green as png_uint_16 as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                            as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w = (temp_0 as ::core::ffi::c_int
                                + (temp_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            (*palette.offset(i as isize)).green =
                                *(*png_ptr).gamma_from_1.offset(w as isize);
                            v = *(*png_ptr)
                                .gamma_to_1
                                .offset((*palette.offset(i as isize)).blue as isize);
                            let mut temp_1: png_uint_16 = (v as png_uint_16 as ::core::ffi::c_int
                                * *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                    as ::core::ffi::c_int
                                + back_1.blue as png_uint_16 as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - *(*png_ptr).trans_alpha.offset(i as isize) as png_uint_16
                                            as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w = (temp_1 as ::core::ffi::c_int
                                + (temp_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            (*palette.offset(i as isize)).blue =
                                *(*png_ptr).gamma_from_1.offset(w as isize);
                        }
                    } else {
                        (*palette.offset(i as isize)).red = *(*png_ptr)
                            .gamma_table
                            .offset((*palette.offset(i as isize)).red as isize);
                        (*palette.offset(i as isize)).green = *(*png_ptr)
                            .gamma_table
                            .offset((*palette.offset(i as isize)).green as isize);
                        (*palette.offset(i as isize)).blue = *(*png_ptr)
                            .gamma_table
                            .offset((*palette.offset(i as isize)).blue as isize);
                    }
                    i += 1;
                }
                (*png_ptr).transformations &= !(PNG_COMPOSE | PNG_GAMMA);
                (*png_ptr).flags &= !PNG_FLAG_OPTIMIZE_ALPHA;
            } else {
                let mut gs_sig: ::core::ffi::c_int = 0;
                let mut g_sig: ::core::ffi::c_int = 0;
                let mut g_0: png_fixed_point = PNG_FP_1;
                let mut gs_0: png_fixed_point = PNG_FP_1;
                match (*png_ptr).background_gamma_type as ::core::ffi::c_int {
                    PNG_BACKGROUND_GAMMA_SCREEN => {
                        g_0 = (*png_ptr).screen_gamma;
                    }
                    PNG_BACKGROUND_GAMMA_FILE => {
                        g_0 = png_reciprocal((*png_ptr).file_gamma);
                        gs_0 = png_reciprocal2((*png_ptr).file_gamma, (*png_ptr).screen_gamma);
                    }
                    PNG_BACKGROUND_GAMMA_UNIQUE => {
                        g_0 = png_reciprocal((*png_ptr).background_gamma);
                        gs_0 =
                            png_reciprocal2((*png_ptr).background_gamma, (*png_ptr).screen_gamma);
                    }
                    _ => {
                        png_error(
                            png_ptr,
                            b"invalid background gamma type\0" as *const u8 as png_const_charp,
                        );
                    }
                }
                g_sig = png_gamma_significant(g_0);
                gs_sig = png_gamma_significant(gs_0);
                if g_sig != 0 as ::core::ffi::c_int {
                    (*png_ptr).background_1.gray = png_gamma_correct(
                        png_ptr,
                        (*png_ptr).background.gray as ::core::ffi::c_uint,
                        g_0,
                    );
                }
                if gs_sig != 0 as ::core::ffi::c_int {
                    (*png_ptr).background.gray = png_gamma_correct(
                        png_ptr,
                        (*png_ptr).background.gray as ::core::ffi::c_uint,
                        gs_0,
                    );
                }
                if (*png_ptr).background.red as ::core::ffi::c_int
                    != (*png_ptr).background.green as ::core::ffi::c_int
                    || (*png_ptr).background.red as ::core::ffi::c_int
                        != (*png_ptr).background.blue as ::core::ffi::c_int
                    || (*png_ptr).background.red as ::core::ffi::c_int
                        != (*png_ptr).background.gray as ::core::ffi::c_int
                {
                    if g_sig != 0 as ::core::ffi::c_int {
                        (*png_ptr).background_1.red = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.red as ::core::ffi::c_uint,
                            g_0,
                        );
                        (*png_ptr).background_1.green = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.green as ::core::ffi::c_uint,
                            g_0,
                        );
                        (*png_ptr).background_1.blue = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.blue as ::core::ffi::c_uint,
                            g_0,
                        );
                    }
                    if gs_sig != 0 as ::core::ffi::c_int {
                        (*png_ptr).background.red = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.red as ::core::ffi::c_uint,
                            gs_0,
                        );
                        (*png_ptr).background.green = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.green as ::core::ffi::c_uint,
                            gs_0,
                        );
                        (*png_ptr).background.blue = png_gamma_correct(
                            png_ptr,
                            (*png_ptr).background.blue as ::core::ffi::c_uint,
                            gs_0,
                        );
                    }
                } else {
                    (*png_ptr).background_1.blue = (*png_ptr).background_1.gray;
                    (*png_ptr).background_1.green = (*png_ptr).background_1.blue;
                    (*png_ptr).background_1.red = (*png_ptr).background_1.green;
                    (*png_ptr).background.blue = (*png_ptr).background.gray;
                    (*png_ptr).background.green = (*png_ptr).background.blue;
                    (*png_ptr).background.red = (*png_ptr).background.green;
                }
                (*png_ptr).background_gamma_type = PNG_BACKGROUND_GAMMA_SCREEN as png_byte;
            }
        } else if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
            && ((*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND
                == 0 as ::core::ffi::c_uint
                || (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
                    == 0 as ::core::ffi::c_uint)
        {
            let mut palette_0: png_colorp = (*png_ptr).palette;
            let mut num_palette_0: ::core::ffi::c_int =
                (*png_ptr).num_palette as ::core::ffi::c_int;
            let mut i_0: ::core::ffi::c_int = 0;
            i_0 = 0 as ::core::ffi::c_int;
            while i_0 < num_palette_0 {
                (*palette_0.offset(i_0 as isize)).red = *(*png_ptr)
                    .gamma_table
                    .offset((*palette_0.offset(i_0 as isize)).red as isize);
                (*palette_0.offset(i_0 as isize)).green = *(*png_ptr)
                    .gamma_table
                    .offset((*palette_0.offset(i_0 as isize)).green as isize);
                (*palette_0.offset(i_0 as isize)).blue = *(*png_ptr)
                    .gamma_table
                    .offset((*palette_0.offset(i_0 as isize)).blue as isize);
                i_0 += 1;
            }
            (*png_ptr).transformations &= !PNG_GAMMA;
        }
    } else if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
    {
        let mut i_1: ::core::ffi::c_int = 0;
        let mut istop: ::core::ffi::c_int = (*png_ptr).num_trans as ::core::ffi::c_int;
        let mut back_0: png_color = png_color {
            red: 0,
            green: 0,
            blue: 0,
        };
        let mut palette_1: png_colorp = (*png_ptr).palette;
        back_0.red = (*png_ptr).background.red as png_byte;
        back_0.green = (*png_ptr).background.green as png_byte;
        back_0.blue = (*png_ptr).background.blue as png_byte;
        i_1 = 0 as ::core::ffi::c_int;
        while i_1 < istop {
            if *(*png_ptr).trans_alpha.offset(i_1 as isize) as ::core::ffi::c_int
                == 0 as ::core::ffi::c_int
            {
                *palette_1.offset(i_1 as isize) = back_0;
            } else if *(*png_ptr).trans_alpha.offset(i_1 as isize) as ::core::ffi::c_int
                != 0xff as ::core::ffi::c_int
            {
                let mut temp_2: png_uint_16 =
                    ((*palette_1.offset(i_1 as isize)).red as png_uint_16 as ::core::ffi::c_int
                        * *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                            as ::core::ffi::c_int
                        + back_0.red as png_uint_16 as ::core::ffi::c_int
                            * (255 as ::core::ffi::c_int
                                - *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                                    as ::core::ffi::c_int)
                                as png_uint_16 as ::core::ffi::c_int
                        + 128 as ::core::ffi::c_int) as png_uint_16;
                (*palette_1.offset(i_1 as isize)).red = (temp_2 as ::core::ffi::c_int
                    + (temp_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                    >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int)
                    as png_byte;
                let mut temp_3: png_uint_16 =
                    ((*palette_1.offset(i_1 as isize)).green as png_uint_16 as ::core::ffi::c_int
                        * *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                            as ::core::ffi::c_int
                        + back_0.green as png_uint_16 as ::core::ffi::c_int
                            * (255 as ::core::ffi::c_int
                                - *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                                    as ::core::ffi::c_int)
                                as png_uint_16 as ::core::ffi::c_int
                        + 128 as ::core::ffi::c_int) as png_uint_16;
                (*palette_1.offset(i_1 as isize)).green = (temp_3 as ::core::ffi::c_int
                    + (temp_3 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                    >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int)
                    as png_byte;
                let mut temp_4: png_uint_16 =
                    ((*palette_1.offset(i_1 as isize)).blue as png_uint_16 as ::core::ffi::c_int
                        * *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                            as ::core::ffi::c_int
                        + back_0.blue as png_uint_16 as ::core::ffi::c_int
                            * (255 as ::core::ffi::c_int
                                - *(*png_ptr).trans_alpha.offset(i_1 as isize) as png_uint_16
                                    as ::core::ffi::c_int)
                                as png_uint_16 as ::core::ffi::c_int
                        + 128 as ::core::ffi::c_int) as png_uint_16;
                (*palette_1.offset(i_1 as isize)).blue = (temp_4 as ::core::ffi::c_int
                    + (temp_4 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                    >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int)
                    as png_byte;
            }
            i_1 += 1;
        }
        (*png_ptr).transformations &= !PNG_COMPOSE;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SHIFT != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND
            == 0 as ::core::ffi::c_uint
        && (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
    {
        let mut i_2: ::core::ffi::c_int = 0;
        let mut istop_0: ::core::ffi::c_int = (*png_ptr).num_palette as ::core::ffi::c_int;
        let mut shift: ::core::ffi::c_int =
            8 as ::core::ffi::c_int - (*png_ptr).sig_bit.red as ::core::ffi::c_int;
        (*png_ptr).transformations &= !PNG_SHIFT;
        if shift > 0 as ::core::ffi::c_int && shift < 8 as ::core::ffi::c_int {
            i_2 = 0 as ::core::ffi::c_int;
            while i_2 < istop_0 {
                let mut component_0: ::core::ffi::c_int =
                    (*(*png_ptr).palette.offset(i_2 as isize)).red as ::core::ffi::c_int;
                component_0 >>= shift;
                (*(*png_ptr).palette.offset(i_2 as isize)).red = component_0 as png_byte;
                i_2 += 1;
            }
        }
        shift = 8 as ::core::ffi::c_int - (*png_ptr).sig_bit.green as ::core::ffi::c_int;
        if shift > 0 as ::core::ffi::c_int && shift < 8 as ::core::ffi::c_int {
            i_2 = 0 as ::core::ffi::c_int;
            while i_2 < istop_0 {
                let mut component_1: ::core::ffi::c_int =
                    (*(*png_ptr).palette.offset(i_2 as isize)).green as ::core::ffi::c_int;
                component_1 >>= shift;
                (*(*png_ptr).palette.offset(i_2 as isize)).green = component_1 as png_byte;
                i_2 += 1;
            }
        }
        shift = 8 as ::core::ffi::c_int - (*png_ptr).sig_bit.blue as ::core::ffi::c_int;
        if shift > 0 as ::core::ffi::c_int && shift < 8 as ::core::ffi::c_int {
            i_2 = 0 as ::core::ffi::c_int;
            while i_2 < istop_0 {
                let mut component_2: ::core::ffi::c_int =
                    (*(*png_ptr).palette.offset(i_2 as isize)).blue as ::core::ffi::c_int;
                component_2 >>= shift;
                (*(*png_ptr).palette.offset(i_2 as isize)).blue = component_2 as png_byte;
                i_2 += 1;
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_read_transform_info(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
) {
    if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        && !(*info_ptr).palette.is_null()
        && !(*png_ptr).palette.is_null()
    {
        memcpy(
            (*info_ptr).palette as *mut ::core::ffi::c_void,
            (*png_ptr).palette as *const ::core::ffi::c_void,
            (PNG_MAX_PALETTE_LENGTH as size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as size_t),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND != 0 as ::core::ffi::c_uint {
        if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
            if (*png_ptr).num_trans as ::core::ffi::c_int > 0 as ::core::ffi::c_int {
                (*info_ptr).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
            } else {
                (*info_ptr).color_type = PNG_COLOR_TYPE_RGB as png_byte;
            }
            (*info_ptr).bit_depth = 8 as png_byte;
            (*info_ptr).num_trans = 0 as png_uint_16;
            if (*png_ptr).palette.is_null() {
                png_error(
                    png_ptr,
                    b"Palette is NULL in indexed image\0" as *const u8 as png_const_charp,
                );
            }
        } else {
            if (*png_ptr).num_trans as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_tRNS
                    != 0 as ::core::ffi::c_uint
                {
                    (*info_ptr).color_type = ((*info_ptr).color_type as ::core::ffi::c_int
                        | PNG_COLOR_MASK_ALPHA)
                        as png_byte;
                }
            }
            if ((*info_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
                (*info_ptr).bit_depth = 8 as png_byte;
            }
            (*info_ptr).num_trans = 0 as png_uint_16;
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE != 0 as ::core::ffi::c_uint {
        (*info_ptr).background = (*png_ptr).background;
    }
    (*info_ptr).gamma = (*png_ptr).file_gamma;
    if (*info_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SCALE_16_TO_8
            != 0 as ::core::ffi::c_uint
        {
            (*info_ptr).bit_depth = 8 as png_byte;
        }
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_16_TO_8
            != 0 as ::core::ffi::c_uint
        {
            (*info_ptr).bit_depth = 8 as png_byte;
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GRAY_TO_RGB
        != 0 as ::core::ffi::c_uint
    {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as ::core::ffi::c_int | PNG_COLOR_MASK_COLOR) as png_byte;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
        != 0 as ::core::ffi::c_uint
    {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as ::core::ffi::c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_QUANTIZE != 0 as ::core::ffi::c_uint
    {
        if ((*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
            || (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA)
            && !(*png_ptr).palette_lookup.is_null()
            && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
        {
            (*info_ptr).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_16 != 0 as ::core::ffi::c_uint
        && (*info_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
        && (*info_ptr).color_type as ::core::ffi::c_int != PNG_COLOR_TYPE_PALETTE
    {
        (*info_ptr).bit_depth = 16 as png_byte;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_PACK != 0 as ::core::ffi::c_uint
        && ((*info_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int
    {
        (*info_ptr).bit_depth = 8 as png_byte;
    }
    if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        (*info_ptr).channels = 1 as png_byte;
    } else if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
        != 0 as ::core::ffi::c_int
    {
        (*info_ptr).channels = 3 as png_byte;
    } else {
        (*info_ptr).channels = 1 as png_byte;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_STRIP_ALPHA
        != 0 as ::core::ffi::c_uint
    {
        (*info_ptr).color_type =
            ((*info_ptr).color_type as ::core::ffi::c_int & !PNG_COLOR_MASK_ALPHA) as png_byte;
        (*info_ptr).num_trans = 0 as png_uint_16;
    }
    if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
        != 0 as ::core::ffi::c_int
    {
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_FILLER != 0 as ::core::ffi::c_uint
        && ((*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
            || (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY)
    {
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_ADD_ALPHA
            != 0 as ::core::ffi::c_uint
        {
            (*info_ptr).color_type =
                ((*info_ptr).color_type as ::core::ffi::c_int | PNG_COLOR_MASK_ALPHA) as png_byte;
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_USER_TRANSFORM
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).user_transform_depth as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            (*info_ptr).bit_depth = (*png_ptr).user_transform_depth;
        }
        if (*png_ptr).user_transform_channels as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            (*info_ptr).channels = (*png_ptr).user_transform_channels;
        }
    }
    (*info_ptr).pixel_depth = ((*info_ptr).channels as ::core::ffi::c_int
        * (*info_ptr).bit_depth as ::core::ffi::c_int) as png_byte;
    (*info_ptr).rowbytes =
        if (*info_ptr).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
            ((*info_ptr).width as size_t)
                .wrapping_mul((*info_ptr).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
        } else {
            ((*info_ptr).width as size_t)
                .wrapping_mul((*info_ptr).pixel_depth as size_t)
                .wrapping_add(7 as size_t)
                >> 3 as ::core::ffi::c_int
        };
    (*png_ptr).info_rowbytes = (*info_ptr).rowbytes;
}
unsafe extern "C" fn png_do_unpack(mut row_info: png_row_infop, mut row: png_bytep) {
    if ((*row_info).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
        let mut i: png_uint_32 = 0;
        let mut row_width: png_uint_32 = (*row_info).width;
        match (*row_info).bit_depth as ::core::ffi::c_int {
            1 => {
                let mut sp: png_bytep = row.offset(
                    ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                        >> 3 as ::core::ffi::c_int) as size_t as isize,
                );
                let mut dp: png_bytep = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut shift: png_uint_32 = (7 as png_uint_32)
                    .wrapping_sub(row_width.wrapping_add(7 as png_uint_32) & 0x7 as png_uint_32);
                i = 0 as png_uint_32;
                while i < row_width {
                    *dp = (*sp as ::core::ffi::c_int >> shift & 0x1 as ::core::ffi::c_int)
                        as png_byte;
                    if shift == 7 as ::core::ffi::c_uint {
                        shift = 0 as png_uint_32;
                        sp = sp.offset(-1);
                    } else {
                        shift = shift.wrapping_add(1);
                    }
                    dp = dp.offset(-1);
                    i = i.wrapping_add(1);
                }
            }
            2 => {
                let mut sp_0: png_bytep = row.offset(
                    ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                        >> 2 as ::core::ffi::c_int) as size_t as isize,
                );
                let mut dp_0: png_bytep = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut shift_0: png_uint_32 = (3 as png_uint_32)
                    .wrapping_sub(row_width.wrapping_add(3 as png_uint_32) & 0x3 as png_uint_32)
                    << 1 as ::core::ffi::c_int;
                i = 0 as png_uint_32;
                while i < row_width {
                    *dp_0 = (*sp_0 as ::core::ffi::c_int >> shift_0 & 0x3 as ::core::ffi::c_int)
                        as png_byte;
                    if shift_0 == 6 as ::core::ffi::c_uint {
                        shift_0 = 0 as png_uint_32;
                        sp_0 = sp_0.offset(-1);
                    } else {
                        shift_0 = (shift_0 as ::core::ffi::c_uint)
                            .wrapping_add(2 as ::core::ffi::c_uint)
                            as png_uint_32 as png_uint_32;
                    }
                    dp_0 = dp_0.offset(-1);
                    i = i.wrapping_add(1);
                }
            }
            4 => {
                let mut sp_1: png_bytep = row.offset(
                    ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                        >> 1 as ::core::ffi::c_int) as size_t as isize,
                );
                let mut dp_1: png_bytep = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut shift_1: png_uint_32 = (1 as png_uint_32)
                    .wrapping_sub(row_width.wrapping_add(1 as png_uint_32) & 0x1 as png_uint_32)
                    << 2 as ::core::ffi::c_int;
                i = 0 as png_uint_32;
                while i < row_width {
                    *dp_1 = (*sp_1 as ::core::ffi::c_int >> shift_1 & 0xf as ::core::ffi::c_int)
                        as png_byte;
                    if shift_1 == 4 as ::core::ffi::c_uint {
                        shift_1 = 0 as png_uint_32;
                        sp_1 = sp_1.offset(-1);
                    } else {
                        shift_1 = 4 as png_uint_32;
                    }
                    dp_1 = dp_1.offset(-1);
                    i = i.wrapping_add(1);
                }
            }
            _ => {}
        }
        (*row_info).bit_depth = 8 as png_byte;
        (*row_info).pixel_depth =
            (8 as ::core::ffi::c_int * (*row_info).channels as ::core::ffi::c_int) as png_byte;
        (*row_info).rowbytes = (row_width as size_t).wrapping_mul((*row_info).channels as size_t);
    }
}
unsafe extern "C" fn png_do_unshift(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut sig_bits: png_const_color_8p,
) {
    let mut color_type: ::core::ffi::c_int = 0;
    color_type = (*row_info).color_type as ::core::ffi::c_int;
    if color_type != PNG_COLOR_TYPE_PALETTE {
        let mut shift: [::core::ffi::c_int; 4] = [0; 4];
        let mut channels: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let mut bit_depth: ::core::ffi::c_int = (*row_info).bit_depth as ::core::ffi::c_int;
        if color_type & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int {
            let fresh2 = channels;
            channels = channels + 1;
            shift[fresh2 as usize] = bit_depth - (*sig_bits).red as ::core::ffi::c_int;
            let fresh3 = channels;
            channels = channels + 1;
            shift[fresh3 as usize] = bit_depth - (*sig_bits).green as ::core::ffi::c_int;
            let fresh4 = channels;
            channels = channels + 1;
            shift[fresh4 as usize] = bit_depth - (*sig_bits).blue as ::core::ffi::c_int;
        } else {
            let fresh5 = channels;
            channels = channels + 1;
            shift[fresh5 as usize] = bit_depth - (*sig_bits).gray as ::core::ffi::c_int;
        }
        if color_type & PNG_COLOR_MASK_ALPHA != 0 as ::core::ffi::c_int {
            let fresh6 = channels;
            channels = channels + 1;
            shift[fresh6 as usize] = bit_depth - (*sig_bits).alpha as ::core::ffi::c_int;
        }
        let mut c: ::core::ffi::c_int = 0;
        let mut have_shift: ::core::ffi::c_int = 0;
        have_shift = 0 as ::core::ffi::c_int;
        c = have_shift;
        while c < channels {
            if shift[c as usize] <= 0 as ::core::ffi::c_int || shift[c as usize] >= bit_depth {
                shift[c as usize] = 0 as ::core::ffi::c_int;
            } else {
                have_shift = 1 as ::core::ffi::c_int;
            }
            c += 1;
        }
        if have_shift == 0 as ::core::ffi::c_int {
            return;
        }
        match bit_depth {
            2 => {
                let mut bp: png_bytep = row;
                let mut bp_end: png_bytep = bp.offset((*row_info).rowbytes as isize);
                while bp < bp_end {
                    let mut b: ::core::ffi::c_int = *bp as ::core::ffi::c_int
                        >> 1 as ::core::ffi::c_int
                        & 0x55 as ::core::ffi::c_int;
                    let fresh7 = bp;
                    bp = bp.offset(1);
                    *fresh7 = b as png_byte;
                }
            }
            4 => {
                let mut bp_0: png_bytep = row;
                let mut bp_end_0: png_bytep = bp_0.offset((*row_info).rowbytes as isize);
                let mut gray_shift: ::core::ffi::c_int = shift[0 as ::core::ffi::c_int as usize];
                let mut mask: ::core::ffi::c_int = 0xf as ::core::ffi::c_int >> gray_shift;
                mask |= mask << 4 as ::core::ffi::c_int;
                while bp_0 < bp_end_0 {
                    let mut b_0: ::core::ffi::c_int =
                        *bp_0 as ::core::ffi::c_int >> gray_shift & mask;
                    let fresh8 = bp_0;
                    bp_0 = bp_0.offset(1);
                    *fresh8 = b_0 as png_byte;
                }
            }
            8 => {
                let mut bp_1: png_bytep = row;
                let mut bp_end_1: png_bytep = bp_1.offset((*row_info).rowbytes as isize);
                let mut channel: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while bp_1 < bp_end_1 {
                    let mut b_1: ::core::ffi::c_int =
                        *bp_1 as ::core::ffi::c_int >> shift[channel as usize];
                    channel += 1;
                    if channel >= channels {
                        channel = 0 as ::core::ffi::c_int;
                    }
                    let fresh9 = bp_1;
                    bp_1 = bp_1.offset(1);
                    *fresh9 = b_1 as png_byte;
                }
            }
            16 => {
                let mut bp_2: png_bytep = row;
                let mut bp_end_2: png_bytep = bp_2.offset((*row_info).rowbytes as isize);
                let mut channel_0: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                while bp_2 < bp_end_2 {
                    let mut value: ::core::ffi::c_int =
                        ((*bp_2.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *bp_2.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
                    value >>= shift[channel_0 as usize];
                    channel_0 += 1;
                    if channel_0 >= channels {
                        channel_0 = 0 as ::core::ffi::c_int;
                    }
                    let fresh10 = bp_2;
                    bp_2 = bp_2.offset(1);
                    *fresh10 = (value >> 8 as ::core::ffi::c_int) as png_byte;
                    let fresh11 = bp_2;
                    bp_2 = bp_2.offset(1);
                    *fresh11 = value as png_byte;
                }
            }
            _ => {}
        }
    }
}
unsafe extern "C" fn png_do_scale_16_to_8(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        let mut sp: png_bytep = row;
        let mut dp: png_bytep = row;
        let mut ep: png_bytep = sp.offset((*row_info).rowbytes as isize);
        while sp < ep {
            let fresh52 = sp;
            sp = sp.offset(1);
            let mut tmp: png_int_32 = *fresh52 as png_int_32;
            let fresh53 = sp;
            sp = sp.offset(1);
            tmp += (*fresh53 as ::core::ffi::c_int - tmp as ::core::ffi::c_int
                + 128 as ::core::ffi::c_int)
                * 65535 as ::core::ffi::c_int
                >> 24 as ::core::ffi::c_int;
            let fresh54 = dp;
            dp = dp.offset(1);
            *fresh54 = tmp as png_byte;
        }
        (*row_info).bit_depth = 8 as png_byte;
        (*row_info).pixel_depth =
            (8 as ::core::ffi::c_int * (*row_info).channels as ::core::ffi::c_int) as png_byte;
        (*row_info).rowbytes =
            ((*row_info).width as size_t).wrapping_mul((*row_info).channels as size_t);
    }
}
unsafe extern "C" fn png_do_chop(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        let mut sp: png_bytep = row;
        let mut dp: png_bytep = row;
        let mut ep: png_bytep = sp.offset((*row_info).rowbytes as isize);
        while sp < ep {
            let fresh51 = dp;
            dp = dp.offset(1);
            *fresh51 = *sp;
            sp = sp.offset(2 as ::core::ffi::c_int as isize);
        }
        (*row_info).bit_depth = 8 as png_byte;
        (*row_info).pixel_depth =
            (8 as ::core::ffi::c_int * (*row_info).channels as ::core::ffi::c_int) as png_byte;
        (*row_info).rowbytes =
            ((*row_info).width as size_t).wrapping_mul((*row_info).channels as size_t);
    }
}
unsafe extern "C" fn png_do_read_swap_alpha(mut row_info: png_row_infop, mut row: png_bytep) {
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp: png_bytep = sp;
            let mut save: png_byte = 0;
            let mut i: png_uint_32 = 0;
            i = 0 as png_uint_32;
            while i < row_width {
                sp = sp.offset(-1);
                save = *sp;
                sp = sp.offset(-1);
                dp = dp.offset(-1);
                *dp = *sp;
                sp = sp.offset(-1);
                dp = dp.offset(-1);
                *dp = *sp;
                sp = sp.offset(-1);
                dp = dp.offset(-1);
                *dp = *sp;
                dp = dp.offset(-1);
                *dp = save;
                i = i.wrapping_add(1);
            }
        } else {
            let mut sp_0: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_0: png_bytep = sp_0;
            let mut save_0: [png_byte; 2] = [0; 2];
            let mut i_0: png_uint_32 = 0;
            i_0 = 0 as png_uint_32;
            while i_0 < row_width {
                sp_0 = sp_0.offset(-1);
                save_0[0 as ::core::ffi::c_int as usize] = *sp_0;
                sp_0 = sp_0.offset(-1);
                save_0[1 as ::core::ffi::c_int as usize] = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = *sp_0;
                dp_0 = dp_0.offset(-1);
                *dp_0 = save_0[0 as ::core::ffi::c_int as usize];
                dp_0 = dp_0.offset(-1);
                *dp_0 = save_0[1 as ::core::ffi::c_int as usize];
                i_0 = i_0.wrapping_add(1);
            }
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp_1: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_1: png_bytep = sp_1;
            let mut save_1: png_byte = 0;
            let mut i_1: png_uint_32 = 0;
            i_1 = 0 as png_uint_32;
            while i_1 < row_width {
                sp_1 = sp_1.offset(-1);
                save_1 = *sp_1;
                sp_1 = sp_1.offset(-1);
                dp_1 = dp_1.offset(-1);
                *dp_1 = *sp_1;
                dp_1 = dp_1.offset(-1);
                *dp_1 = save_1;
                i_1 = i_1.wrapping_add(1);
            }
        } else {
            let mut sp_2: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_2: png_bytep = sp_2;
            let mut save_2: [png_byte; 2] = [0; 2];
            let mut i_2: png_uint_32 = 0;
            i_2 = 0 as png_uint_32;
            while i_2 < row_width {
                sp_2 = sp_2.offset(-1);
                save_2[0 as ::core::ffi::c_int as usize] = *sp_2;
                sp_2 = sp_2.offset(-1);
                save_2[1 as ::core::ffi::c_int as usize] = *sp_2;
                sp_2 = sp_2.offset(-1);
                dp_2 = dp_2.offset(-1);
                *dp_2 = *sp_2;
                sp_2 = sp_2.offset(-1);
                dp_2 = dp_2.offset(-1);
                *dp_2 = *sp_2;
                dp_2 = dp_2.offset(-1);
                *dp_2 = save_2[0 as ::core::ffi::c_int as usize];
                dp_2 = dp_2.offset(-1);
                *dp_2 = save_2[1 as ::core::ffi::c_int as usize];
                i_2 = i_2.wrapping_add(1);
            }
        }
    }
}
unsafe extern "C" fn png_do_read_invert_alpha(mut row_info: png_row_infop, mut row: png_bytep) {
    let mut row_width: png_uint_32 = 0;
    row_width = (*row_info).width;
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp: png_bytep = sp;
            let mut i: png_uint_32 = 0;
            i = 0 as png_uint_32;
            while i < row_width {
                sp = sp.offset(-1);
                dp = dp.offset(-1);
                *dp = (255 as ::core::ffi::c_int - *sp as ::core::ffi::c_int) as png_byte;
                sp = sp.offset(-(3 as ::core::ffi::c_int as isize));
                dp = sp;
                i = i.wrapping_add(1);
            }
        } else {
            let mut sp_0: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_0: png_bytep = sp_0;
            let mut i_0: png_uint_32 = 0;
            i_0 = 0 as png_uint_32;
            while i_0 < row_width {
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = (255 as ::core::ffi::c_int - *sp_0 as ::core::ffi::c_int) as png_byte;
                sp_0 = sp_0.offset(-1);
                dp_0 = dp_0.offset(-1);
                *dp_0 = (255 as ::core::ffi::c_int - *sp_0 as ::core::ffi::c_int) as png_byte;
                sp_0 = sp_0.offset(-(6 as ::core::ffi::c_int as isize));
                dp_0 = sp_0;
                i_0 = i_0.wrapping_add(1);
            }
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp_1: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_1: png_bytep = sp_1;
            let mut i_1: png_uint_32 = 0;
            i_1 = 0 as png_uint_32;
            while i_1 < row_width {
                sp_1 = sp_1.offset(-1);
                dp_1 = dp_1.offset(-1);
                *dp_1 = (255 as ::core::ffi::c_int - *sp_1 as ::core::ffi::c_int) as png_byte;
                sp_1 = sp_1.offset(-1);
                dp_1 = dp_1.offset(-1);
                *dp_1 = *sp_1;
                i_1 = i_1.wrapping_add(1);
            }
        } else {
            let mut sp_2: png_bytep = row.offset((*row_info).rowbytes as isize);
            let mut dp_2: png_bytep = sp_2;
            let mut i_2: png_uint_32 = 0;
            i_2 = 0 as png_uint_32;
            while i_2 < row_width {
                sp_2 = sp_2.offset(-1);
                dp_2 = dp_2.offset(-1);
                *dp_2 = (255 as ::core::ffi::c_int - *sp_2 as ::core::ffi::c_int) as png_byte;
                sp_2 = sp_2.offset(-1);
                dp_2 = dp_2.offset(-1);
                *dp_2 = (255 as ::core::ffi::c_int - *sp_2 as ::core::ffi::c_int) as png_byte;
                sp_2 = sp_2.offset(-(2 as ::core::ffi::c_int as isize));
                dp_2 = sp_2;
                i_2 = i_2.wrapping_add(1);
            }
        }
    }
}
unsafe extern "C" fn png_do_read_filler(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut filler: png_uint_32,
    mut flags: png_uint_32,
) {
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    let mut hi_filler: png_byte = (filler >> 8 as ::core::ffi::c_int) as png_byte;
    let mut lo_filler: png_byte = filler as png_byte;
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER != 0 as ::core::ffi::c_uint {
                let mut sp: png_bytep = row.offset(row_width as size_t as isize);
                let mut dp: png_bytep = sp.offset(row_width as size_t as isize);
                i = 1 as png_uint_32;
                while i < row_width {
                    dp = dp.offset(-1);
                    *dp = lo_filler;
                    sp = sp.offset(-1);
                    dp = dp.offset(-1);
                    *dp = *sp;
                    i = i.wrapping_add(1);
                }
                dp = dp.offset(-1);
                *dp = lo_filler;
                (*row_info).channels = 2 as png_byte;
                (*row_info).pixel_depth = 16 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(2 as size_t);
            } else {
                let mut sp_0: png_bytep = row.offset(row_width as size_t as isize);
                let mut dp_0: png_bytep = sp_0.offset(row_width as size_t as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    sp_0 = sp_0.offset(-1);
                    dp_0 = dp_0.offset(-1);
                    *dp_0 = *sp_0;
                    dp_0 = dp_0.offset(-1);
                    *dp_0 = lo_filler;
                    i = i.wrapping_add(1);
                }
                (*row_info).channels = 2 as png_byte;
                (*row_info).pixel_depth = 16 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(2 as size_t);
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            if flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER != 0 as ::core::ffi::c_uint {
                let mut sp_1: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                let mut dp_1: png_bytep =
                    sp_1.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 1 as png_uint_32;
                while i < row_width {
                    dp_1 = dp_1.offset(-1);
                    *dp_1 = lo_filler;
                    dp_1 = dp_1.offset(-1);
                    *dp_1 = hi_filler;
                    sp_1 = sp_1.offset(-1);
                    dp_1 = dp_1.offset(-1);
                    *dp_1 = *sp_1;
                    sp_1 = sp_1.offset(-1);
                    dp_1 = dp_1.offset(-1);
                    *dp_1 = *sp_1;
                    i = i.wrapping_add(1);
                }
                dp_1 = dp_1.offset(-1);
                *dp_1 = lo_filler;
                dp_1 = dp_1.offset(-1);
                *dp_1 = hi_filler;
                (*row_info).channels = 2 as png_byte;
                (*row_info).pixel_depth = 32 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(4 as size_t);
            } else {
                let mut sp_2: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                let mut dp_2: png_bytep =
                    sp_2.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    sp_2 = sp_2.offset(-1);
                    dp_2 = dp_2.offset(-1);
                    *dp_2 = *sp_2;
                    sp_2 = sp_2.offset(-1);
                    dp_2 = dp_2.offset(-1);
                    *dp_2 = *sp_2;
                    dp_2 = dp_2.offset(-1);
                    *dp_2 = lo_filler;
                    dp_2 = dp_2.offset(-1);
                    *dp_2 = hi_filler;
                    i = i.wrapping_add(1);
                }
                (*row_info).channels = 2 as png_byte;
                (*row_info).pixel_depth = 32 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(4 as size_t);
            }
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER != 0 as ::core::ffi::c_uint {
                let mut sp_3: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(3 as size_t) as isize);
                let mut dp_3: png_bytep = sp_3.offset(row_width as size_t as isize);
                i = 1 as png_uint_32;
                while i < row_width {
                    dp_3 = dp_3.offset(-1);
                    *dp_3 = lo_filler;
                    sp_3 = sp_3.offset(-1);
                    dp_3 = dp_3.offset(-1);
                    *dp_3 = *sp_3;
                    sp_3 = sp_3.offset(-1);
                    dp_3 = dp_3.offset(-1);
                    *dp_3 = *sp_3;
                    sp_3 = sp_3.offset(-1);
                    dp_3 = dp_3.offset(-1);
                    *dp_3 = *sp_3;
                    i = i.wrapping_add(1);
                }
                dp_3 = dp_3.offset(-1);
                *dp_3 = lo_filler;
                (*row_info).channels = 4 as png_byte;
                (*row_info).pixel_depth = 32 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(4 as size_t);
            } else {
                let mut sp_4: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(3 as size_t) as isize);
                let mut dp_4: png_bytep = sp_4.offset(row_width as size_t as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    sp_4 = sp_4.offset(-1);
                    dp_4 = dp_4.offset(-1);
                    *dp_4 = *sp_4;
                    sp_4 = sp_4.offset(-1);
                    dp_4 = dp_4.offset(-1);
                    *dp_4 = *sp_4;
                    sp_4 = sp_4.offset(-1);
                    dp_4 = dp_4.offset(-1);
                    *dp_4 = *sp_4;
                    dp_4 = dp_4.offset(-1);
                    *dp_4 = lo_filler;
                    i = i.wrapping_add(1);
                }
                (*row_info).channels = 4 as png_byte;
                (*row_info).pixel_depth = 32 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(4 as size_t);
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            if flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER != 0 as ::core::ffi::c_uint {
                let mut sp_5: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(6 as size_t) as isize);
                let mut dp_5: png_bytep =
                    sp_5.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 1 as png_uint_32;
                while i < row_width {
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = lo_filler;
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = hi_filler;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    sp_5 = sp_5.offset(-1);
                    dp_5 = dp_5.offset(-1);
                    *dp_5 = *sp_5;
                    i = i.wrapping_add(1);
                }
                dp_5 = dp_5.offset(-1);
                *dp_5 = lo_filler;
                dp_5 = dp_5.offset(-1);
                *dp_5 = hi_filler;
                (*row_info).channels = 4 as png_byte;
                (*row_info).pixel_depth = 64 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(8 as size_t);
            } else {
                let mut sp_6: png_bytep =
                    row.offset((row_width as size_t).wrapping_mul(6 as size_t) as isize);
                let mut dp_6: png_bytep =
                    sp_6.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    sp_6 = sp_6.offset(-1);
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = *sp_6;
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = lo_filler;
                    dp_6 = dp_6.offset(-1);
                    *dp_6 = hi_filler;
                    i = i.wrapping_add(1);
                }
                (*row_info).channels = 4 as png_byte;
                (*row_info).pixel_depth = 64 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(8 as size_t);
            }
        }
    }
}
unsafe extern "C" fn png_do_gray_to_rgb(mut row_info: png_row_infop, mut row: png_bytep) {
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).bit_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int
        && (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            == 0 as ::core::ffi::c_int
    {
        if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                let mut sp: png_bytep = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut dp: png_bytep =
                    sp.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh12 = dp;
                    dp = dp.offset(-1);
                    *fresh12 = *sp;
                    let fresh13 = dp;
                    dp = dp.offset(-1);
                    *fresh13 = *sp;
                    let fresh14 = sp;
                    sp = sp.offset(-1);
                    let fresh15 = dp;
                    dp = dp.offset(-1);
                    *fresh15 = *fresh14;
                    i = i.wrapping_add(1);
                }
            } else {
                let mut sp_0: png_bytep = row
                    .offset((row_width as size_t).wrapping_mul(2 as size_t) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut dp_0: png_bytep =
                    sp_0.offset((row_width as size_t).wrapping_mul(4 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh16 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh16 = *sp_0;
                    let fresh17 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh17 = *sp_0.offset(-(1 as ::core::ffi::c_int as isize));
                    let fresh18 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh18 = *sp_0;
                    let fresh19 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh19 = *sp_0.offset(-(1 as ::core::ffi::c_int as isize));
                    let fresh20 = sp_0;
                    sp_0 = sp_0.offset(-1);
                    let fresh21 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh21 = *fresh20;
                    let fresh22 = sp_0;
                    sp_0 = sp_0.offset(-1);
                    let fresh23 = dp_0;
                    dp_0 = dp_0.offset(-1);
                    *fresh23 = *fresh22;
                    i = i.wrapping_add(1);
                }
            }
        } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                let mut sp_1: png_bytep = row
                    .offset((row_width as size_t).wrapping_mul(2 as size_t) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut dp_1: png_bytep =
                    sp_1.offset((row_width as size_t).wrapping_mul(2 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh24 = sp_1;
                    sp_1 = sp_1.offset(-1);
                    let fresh25 = dp_1;
                    dp_1 = dp_1.offset(-1);
                    *fresh25 = *fresh24;
                    let fresh26 = dp_1;
                    dp_1 = dp_1.offset(-1);
                    *fresh26 = *sp_1;
                    let fresh27 = dp_1;
                    dp_1 = dp_1.offset(-1);
                    *fresh27 = *sp_1;
                    let fresh28 = sp_1;
                    sp_1 = sp_1.offset(-1);
                    let fresh29 = dp_1;
                    dp_1 = dp_1.offset(-1);
                    *fresh29 = *fresh28;
                    i = i.wrapping_add(1);
                }
            } else {
                let mut sp_2: png_bytep = row
                    .offset((row_width as size_t).wrapping_mul(4 as size_t) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                let mut dp_2: png_bytep =
                    sp_2.offset((row_width as size_t).wrapping_mul(4 as size_t) as isize);
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh30 = sp_2;
                    sp_2 = sp_2.offset(-1);
                    let fresh31 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh31 = *fresh30;
                    let fresh32 = sp_2;
                    sp_2 = sp_2.offset(-1);
                    let fresh33 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh33 = *fresh32;
                    let fresh34 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh34 = *sp_2;
                    let fresh35 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh35 = *sp_2.offset(-(1 as ::core::ffi::c_int as isize));
                    let fresh36 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh36 = *sp_2;
                    let fresh37 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh37 = *sp_2.offset(-(1 as ::core::ffi::c_int as isize));
                    let fresh38 = sp_2;
                    sp_2 = sp_2.offset(-1);
                    let fresh39 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh39 = *fresh38;
                    let fresh40 = sp_2;
                    sp_2 = sp_2.offset(-1);
                    let fresh41 = dp_2;
                    dp_2 = dp_2.offset(-1);
                    *fresh41 = *fresh40;
                    i = i.wrapping_add(1);
                }
            }
        }
        (*row_info).channels =
            ((*row_info).channels as ::core::ffi::c_int + 2 as ::core::ffi::c_int) as png_byte;
        (*row_info).color_type =
            ((*row_info).color_type as ::core::ffi::c_int | PNG_COLOR_MASK_COLOR) as png_byte;
        (*row_info).pixel_depth = ((*row_info).channels as ::core::ffi::c_int
            * (*row_info).bit_depth as ::core::ffi::c_int)
            as png_byte;
        (*row_info).rowbytes =
            if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
            } else {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t)
                    .wrapping_add(7 as size_t)
                    >> 3 as ::core::ffi::c_int
            };
    }
}
unsafe extern "C" fn png_do_rgb_to_gray(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
    mut row: png_bytep,
) -> ::core::ffi::c_int {
    let mut rgb_error: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_PALETTE
        == 0 as ::core::ffi::c_int
        && (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            != 0 as ::core::ffi::c_int
    {
        let mut rc: png_uint_32 = (*png_ptr).rgb_to_gray_red_coeff as png_uint_32;
        let mut gc: png_uint_32 = (*png_ptr).rgb_to_gray_green_coeff as png_uint_32;
        let mut bc: png_uint_32 = (32768 as png_uint_32).wrapping_sub(rc).wrapping_sub(gc);
        let mut row_width: png_uint_32 = (*row_info).width;
        let mut have_alpha: ::core::ffi::c_int =
            ((*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
                != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if !(*png_ptr).gamma_from_1.is_null() && !(*png_ptr).gamma_to_1.is_null() {
                let mut sp: png_bytep = row;
                let mut dp: png_bytep = row;
                let mut i: png_uint_32 = 0;
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh55 = sp;
                    sp = sp.offset(1);
                    let mut red: png_byte = *fresh55;
                    let fresh56 = sp;
                    sp = sp.offset(1);
                    let mut green: png_byte = *fresh56;
                    let fresh57 = sp;
                    sp = sp.offset(1);
                    let mut blue: png_byte = *fresh57;
                    if red as ::core::ffi::c_int != green as ::core::ffi::c_int
                        || red as ::core::ffi::c_int != blue as ::core::ffi::c_int
                    {
                        red = *(*png_ptr).gamma_to_1.offset(red as isize);
                        green = *(*png_ptr).gamma_to_1.offset(green as isize);
                        blue = *(*png_ptr).gamma_to_1.offset(blue as isize);
                        rgb_error |= 1 as ::core::ffi::c_int;
                        let fresh58 = dp;
                        dp = dp.offset(1);
                        *fresh58 = *(*png_ptr).gamma_from_1.offset(
                            ((rc as ::core::ffi::c_uint)
                                .wrapping_mul(red as ::core::ffi::c_uint)
                                .wrapping_add(
                                    (gc as ::core::ffi::c_uint)
                                        .wrapping_mul(green as ::core::ffi::c_uint),
                                )
                                .wrapping_add(
                                    (bc as ::core::ffi::c_uint)
                                        .wrapping_mul(blue as ::core::ffi::c_uint),
                                )
                                .wrapping_add(16384 as ::core::ffi::c_uint)
                                >> 15 as ::core::ffi::c_int) as isize,
                        );
                    } else {
                        if !(*png_ptr).gamma_table.is_null() {
                            red = *(*png_ptr).gamma_table.offset(red as isize);
                        }
                        let fresh59 = dp;
                        dp = dp.offset(1);
                        *fresh59 = red;
                    }
                    if have_alpha != 0 as ::core::ffi::c_int {
                        let fresh60 = sp;
                        sp = sp.offset(1);
                        let fresh61 = dp;
                        dp = dp.offset(1);
                        *fresh61 = *fresh60;
                    }
                    i = i.wrapping_add(1);
                }
            } else {
                let mut sp_0: png_bytep = row;
                let mut dp_0: png_bytep = row;
                let mut i_0: png_uint_32 = 0;
                i_0 = 0 as png_uint_32;
                while i_0 < row_width {
                    let fresh62 = sp_0;
                    sp_0 = sp_0.offset(1);
                    let mut red_0: png_byte = *fresh62;
                    let fresh63 = sp_0;
                    sp_0 = sp_0.offset(1);
                    let mut green_0: png_byte = *fresh63;
                    let fresh64 = sp_0;
                    sp_0 = sp_0.offset(1);
                    let mut blue_0: png_byte = *fresh64;
                    if red_0 as ::core::ffi::c_int != green_0 as ::core::ffi::c_int
                        || red_0 as ::core::ffi::c_int != blue_0 as ::core::ffi::c_int
                    {
                        rgb_error |= 1 as ::core::ffi::c_int;
                        let fresh65 = dp_0;
                        dp_0 = dp_0.offset(1);
                        *fresh65 = ((rc as ::core::ffi::c_uint)
                            .wrapping_mul(red_0 as ::core::ffi::c_uint)
                            .wrapping_add(
                                (gc as ::core::ffi::c_uint)
                                    .wrapping_mul(green_0 as ::core::ffi::c_uint),
                            )
                            .wrapping_add(
                                (bc as ::core::ffi::c_uint)
                                    .wrapping_mul(blue_0 as ::core::ffi::c_uint),
                            )
                            >> 15 as ::core::ffi::c_int)
                            as png_byte;
                    } else {
                        let fresh66 = dp_0;
                        dp_0 = dp_0.offset(1);
                        *fresh66 = red_0;
                    }
                    if have_alpha != 0 as ::core::ffi::c_int {
                        let fresh67 = sp_0;
                        sp_0 = sp_0.offset(1);
                        let fresh68 = dp_0;
                        dp_0 = dp_0.offset(1);
                        *fresh68 = *fresh67;
                    }
                    i_0 = i_0.wrapping_add(1);
                }
            }
        } else if !(*png_ptr).gamma_16_to_1.is_null() && !(*png_ptr).gamma_16_from_1.is_null() {
            let mut sp_1: png_bytep = row;
            let mut dp_1: png_bytep = row;
            let mut i_1: png_uint_32 = 0;
            i_1 = 0 as png_uint_32;
            while i_1 < row_width {
                let mut red_1: png_uint_16 = 0;
                let mut green_1: png_uint_16 = 0;
                let mut blue_1: png_uint_16 = 0;
                let mut w: png_uint_16 = 0;
                let mut hi: png_byte = 0;
                let mut lo: png_byte = 0;
                let fresh69 = sp_1;
                sp_1 = sp_1.offset(1);
                hi = *fresh69;
                let fresh70 = sp_1;
                sp_1 = sp_1.offset(1);
                lo = *fresh70;
                red_1 = ((hi as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo as ::core::ffi::c_int) as png_uint_16;
                let fresh71 = sp_1;
                sp_1 = sp_1.offset(1);
                hi = *fresh71;
                let fresh72 = sp_1;
                sp_1 = sp_1.offset(1);
                lo = *fresh72;
                green_1 = ((hi as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo as ::core::ffi::c_int) as png_uint_16;
                let fresh73 = sp_1;
                sp_1 = sp_1.offset(1);
                hi = *fresh73;
                let fresh74 = sp_1;
                sp_1 = sp_1.offset(1);
                lo = *fresh74;
                blue_1 = ((hi as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo as ::core::ffi::c_int) as png_uint_16;
                if red_1 as ::core::ffi::c_int == green_1 as ::core::ffi::c_int
                    && red_1 as ::core::ffi::c_int == blue_1 as ::core::ffi::c_int
                {
                    if !(*png_ptr).gamma_16_table.is_null() {
                        w = *(*(*png_ptr).gamma_16_table.offset(
                            ((red_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                                >> (*png_ptr).gamma_shift) as isize,
                        ))
                        .offset((red_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize);
                    } else {
                        w = red_1;
                    }
                } else {
                    let mut red_1_0: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.offset(
                        ((red_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                            >> (*png_ptr).gamma_shift) as isize,
                    ))
                    .offset((red_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize);
                    let mut green_1_0: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.offset(
                        ((green_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                            >> (*png_ptr).gamma_shift) as isize,
                    ))
                    .offset((green_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize);
                    let mut blue_1_0: png_uint_16 = *(*(*png_ptr).gamma_16_to_1.offset(
                        ((blue_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                            >> (*png_ptr).gamma_shift) as isize,
                    ))
                    .offset((blue_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize);
                    let mut gray16: png_uint_16 = ((rc as ::core::ffi::c_uint)
                        .wrapping_mul(red_1_0 as ::core::ffi::c_uint)
                        .wrapping_add(
                            (gc as ::core::ffi::c_uint)
                                .wrapping_mul(green_1_0 as ::core::ffi::c_uint),
                        )
                        .wrapping_add(
                            (bc as ::core::ffi::c_uint)
                                .wrapping_mul(blue_1_0 as ::core::ffi::c_uint),
                        )
                        .wrapping_add(16384 as ::core::ffi::c_uint)
                        >> 15 as ::core::ffi::c_int)
                        as png_uint_16;
                    w = *(*(*png_ptr).gamma_16_from_1.offset(
                        ((gray16 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                            >> (*png_ptr).gamma_shift) as isize,
                    ))
                    .offset((gray16 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize);
                    rgb_error |= 1 as ::core::ffi::c_int;
                }
                let fresh75 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh75 = (w as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int) as png_byte;
                let fresh76 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh76 = (w as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                if have_alpha != 0 as ::core::ffi::c_int {
                    let fresh77 = sp_1;
                    sp_1 = sp_1.offset(1);
                    let fresh78 = dp_1;
                    dp_1 = dp_1.offset(1);
                    *fresh78 = *fresh77;
                    let fresh79 = sp_1;
                    sp_1 = sp_1.offset(1);
                    let fresh80 = dp_1;
                    dp_1 = dp_1.offset(1);
                    *fresh80 = *fresh79;
                }
                i_1 = i_1.wrapping_add(1);
            }
        } else {
            let mut sp_2: png_bytep = row;
            let mut dp_2: png_bytep = row;
            let mut i_2: png_uint_32 = 0;
            i_2 = 0 as png_uint_32;
            while i_2 < row_width {
                let mut red_2: png_uint_16 = 0;
                let mut green_2: png_uint_16 = 0;
                let mut blue_2: png_uint_16 = 0;
                let mut gray16_0: png_uint_16 = 0;
                let mut hi_0: png_byte = 0;
                let mut lo_0: png_byte = 0;
                let fresh81 = sp_2;
                sp_2 = sp_2.offset(1);
                hi_0 = *fresh81;
                let fresh82 = sp_2;
                sp_2 = sp_2.offset(1);
                lo_0 = *fresh82;
                red_2 = ((hi_0 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo_0 as ::core::ffi::c_int) as png_uint_16;
                let fresh83 = sp_2;
                sp_2 = sp_2.offset(1);
                hi_0 = *fresh83;
                let fresh84 = sp_2;
                sp_2 = sp_2.offset(1);
                lo_0 = *fresh84;
                green_2 = ((hi_0 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo_0 as ::core::ffi::c_int) as png_uint_16;
                let fresh85 = sp_2;
                sp_2 = sp_2.offset(1);
                hi_0 = *fresh85;
                let fresh86 = sp_2;
                sp_2 = sp_2.offset(1);
                lo_0 = *fresh86;
                blue_2 = ((hi_0 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int
                    | lo_0 as ::core::ffi::c_int) as png_uint_16;
                if red_2 as ::core::ffi::c_int != green_2 as ::core::ffi::c_int
                    || red_2 as ::core::ffi::c_int != blue_2 as ::core::ffi::c_int
                {
                    rgb_error |= 1 as ::core::ffi::c_int;
                }
                gray16_0 = ((rc as ::core::ffi::c_uint)
                    .wrapping_mul(red_2 as ::core::ffi::c_uint)
                    .wrapping_add(
                        (gc as ::core::ffi::c_uint).wrapping_mul(green_2 as ::core::ffi::c_uint),
                    )
                    .wrapping_add(
                        (bc as ::core::ffi::c_uint).wrapping_mul(blue_2 as ::core::ffi::c_uint),
                    )
                    .wrapping_add(16384 as ::core::ffi::c_uint)
                    >> 15 as ::core::ffi::c_int) as png_uint_16;
                let fresh87 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh87 = (gray16_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int) as png_byte;
                let fresh88 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh88 =
                    (gray16_0 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                if have_alpha != 0 as ::core::ffi::c_int {
                    let fresh89 = sp_2;
                    sp_2 = sp_2.offset(1);
                    let fresh90 = dp_2;
                    dp_2 = dp_2.offset(1);
                    *fresh90 = *fresh89;
                    let fresh91 = sp_2;
                    sp_2 = sp_2.offset(1);
                    let fresh92 = dp_2;
                    dp_2 = dp_2.offset(1);
                    *fresh92 = *fresh91;
                }
                i_2 = i_2.wrapping_add(1);
            }
        }
        (*row_info).channels =
            ((*row_info).channels as ::core::ffi::c_int - 2 as ::core::ffi::c_int) as png_byte;
        (*row_info).color_type =
            ((*row_info).color_type as ::core::ffi::c_int & !PNG_COLOR_MASK_COLOR) as png_byte;
        (*row_info).pixel_depth = ((*row_info).channels as ::core::ffi::c_int
            * (*row_info).bit_depth as ::core::ffi::c_int)
            as png_byte;
        (*row_info).rowbytes =
            if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
            } else {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t)
                    .wrapping_add(7 as size_t)
                    >> 3 as ::core::ffi::c_int
            };
    }
    return rgb_error;
}
unsafe extern "C" fn png_do_compose(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut png_ptr: png_structrp,
) {
    let mut gamma_table: png_const_bytep = (*png_ptr).gamma_table as png_const_bytep;
    let mut gamma_from_1: png_const_bytep = (*png_ptr).gamma_from_1 as png_const_bytep;
    let mut gamma_to_1: png_const_bytep = (*png_ptr).gamma_to_1 as png_const_bytep;
    let mut gamma_16: png_const_uint_16pp = (*png_ptr).gamma_16_table as png_const_uint_16pp;
    let mut gamma_16_from_1: png_const_uint_16pp =
        (*png_ptr).gamma_16_from_1 as png_const_uint_16pp;
    let mut gamma_16_to_1: png_const_uint_16pp = (*png_ptr).gamma_16_to_1 as png_const_uint_16pp;
    let mut gamma_shift: ::core::ffi::c_int = (*png_ptr).gamma_shift;
    let mut optimize: ::core::ffi::c_int =
        ((*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_OPTIMIZE_ALPHA
            != 0 as ::core::ffi::c_uint) as ::core::ffi::c_int;
    let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    let mut shift: ::core::ffi::c_int = 0;
    match (*row_info).color_type as ::core::ffi::c_int {
        PNG_COLOR_TYPE_GRAY => match (*row_info).bit_depth as ::core::ffi::c_int {
            1 => {
                sp = row;
                shift = 7 as ::core::ffi::c_int;
                i = 0 as png_uint_32;
                while i < row_width {
                    if (*sp as ::core::ffi::c_int >> shift & 0x1 as ::core::ffi::c_int)
                        as png_uint_16 as ::core::ffi::c_int
                        == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                    {
                        let mut tmp: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                            & 0x7f7f as ::core::ffi::c_int >> 7 as ::core::ffi::c_int - shift)
                            as ::core::ffi::c_uint;
                        tmp |= (((*png_ptr).background.gray as ::core::ffi::c_int) << shift)
                            as ::core::ffi::c_uint;
                        *sp = (tmp & 0xff as ::core::ffi::c_uint) as png_byte;
                    }
                    if shift == 0 as ::core::ffi::c_int {
                        shift = 7 as ::core::ffi::c_int;
                        sp = sp.offset(1);
                    } else {
                        shift -= 1;
                    }
                    i = i.wrapping_add(1);
                }
            }
            2 => {
                if !gamma_table.is_null() {
                    sp = row;
                    shift = 6 as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if (*sp as ::core::ffi::c_int >> shift & 0x3 as ::core::ffi::c_int)
                            as png_uint_16 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            let mut tmp_0: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0x3f3f as ::core::ffi::c_int >> 6 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_0 |= ((*png_ptr).background.gray as ::core::ffi::c_uint) << shift;
                            *sp = (tmp_0 & 0xff as ::core::ffi::c_uint) as png_byte;
                        } else {
                            let mut p: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int >> shift
                                & 0x3 as ::core::ffi::c_int)
                                as ::core::ffi::c_uint;
                            let mut g: ::core::ffi::c_uint = (*gamma_table.offset(
                                (p | p << 2 as ::core::ffi::c_int
                                    | p << 4 as ::core::ffi::c_int
                                    | p << 6 as ::core::ffi::c_int)
                                    as isize,
                            )
                                as ::core::ffi::c_int
                                >> 6 as ::core::ffi::c_int
                                & 0x3 as ::core::ffi::c_int)
                                as ::core::ffi::c_uint;
                            let mut tmp_1: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0x3f3f as ::core::ffi::c_int >> 6 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_1 |= g << shift;
                            *sp = (tmp_1 & 0xff as ::core::ffi::c_uint) as png_byte;
                        }
                        if shift == 0 as ::core::ffi::c_int {
                            shift = 6 as ::core::ffi::c_int;
                            sp = sp.offset(1);
                        } else {
                            shift -= 2 as ::core::ffi::c_int;
                        }
                        i = i.wrapping_add(1);
                    }
                } else {
                    sp = row;
                    shift = 6 as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if (*sp as ::core::ffi::c_int >> shift & 0x3 as ::core::ffi::c_int)
                            as png_uint_16 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            let mut tmp_2: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0x3f3f as ::core::ffi::c_int >> 6 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_2 |= ((*png_ptr).background.gray as ::core::ffi::c_uint) << shift;
                            *sp = (tmp_2 & 0xff as ::core::ffi::c_uint) as png_byte;
                        }
                        if shift == 0 as ::core::ffi::c_int {
                            shift = 6 as ::core::ffi::c_int;
                            sp = sp.offset(1);
                        } else {
                            shift -= 2 as ::core::ffi::c_int;
                        }
                        i = i.wrapping_add(1);
                    }
                }
            }
            4 => {
                if !gamma_table.is_null() {
                    sp = row;
                    shift = 4 as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if (*sp as ::core::ffi::c_int >> shift & 0xf as ::core::ffi::c_int)
                            as png_uint_16 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            let mut tmp_3: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0xf0f as ::core::ffi::c_int >> 4 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_3 |= (((*png_ptr).background.gray as ::core::ffi::c_int) << shift)
                                as ::core::ffi::c_uint;
                            *sp = (tmp_3 & 0xff as ::core::ffi::c_uint) as png_byte;
                        } else {
                            let mut p_0: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int >> shift
                                & 0xf as ::core::ffi::c_int)
                                as ::core::ffi::c_uint;
                            let mut g_0: ::core::ffi::c_uint = (*gamma_table
                                .offset((p_0 | p_0 << 4 as ::core::ffi::c_int) as isize)
                                as ::core::ffi::c_int
                                >> 4 as ::core::ffi::c_int
                                & 0xf as ::core::ffi::c_int)
                                as ::core::ffi::c_uint;
                            let mut tmp_4: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0xf0f as ::core::ffi::c_int >> 4 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_4 |= g_0 << shift;
                            *sp = (tmp_4 & 0xff as ::core::ffi::c_uint) as png_byte;
                        }
                        if shift == 0 as ::core::ffi::c_int {
                            shift = 4 as ::core::ffi::c_int;
                            sp = sp.offset(1);
                        } else {
                            shift -= 4 as ::core::ffi::c_int;
                        }
                        i = i.wrapping_add(1);
                    }
                } else {
                    sp = row;
                    shift = 4 as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if (*sp as ::core::ffi::c_int >> shift & 0xf as ::core::ffi::c_int)
                            as png_uint_16 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            let mut tmp_5: ::core::ffi::c_uint = (*sp as ::core::ffi::c_int
                                & 0xf0f as ::core::ffi::c_int >> 4 as ::core::ffi::c_int - shift)
                                as ::core::ffi::c_uint;
                            tmp_5 |= (((*png_ptr).background.gray as ::core::ffi::c_int) << shift)
                                as ::core::ffi::c_uint;
                            *sp = (tmp_5 & 0xff as ::core::ffi::c_uint) as png_byte;
                        }
                        if shift == 0 as ::core::ffi::c_int {
                            shift = 4 as ::core::ffi::c_int;
                            sp = sp.offset(1);
                        } else {
                            shift -= 4 as ::core::ffi::c_int;
                        }
                        i = i.wrapping_add(1);
                    }
                }
            }
            8 => {
                if !gamma_table.is_null() {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else {
                            *sp = *gamma_table.offset(*sp as isize);
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(1);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            *sp = (*png_ptr).background.gray as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(1);
                    }
                }
            }
            16 => {
                if !gamma_16.is_null() {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v: png_uint_16 = 0;
                        v = (((*sp as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                            + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        if v as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            *sp = ((*png_ptr).background.gray as ::core::ffi::c_int
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                ((*png_ptr).background.gray as ::core::ffi::c_int
                                    & 0xff as ::core::ffi::c_int)
                                    as png_byte;
                        } else {
                            v = *(*gamma_16.offset(
                                (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                    >> gamma_shift) as isize,
                            ))
                            .offset(*sp as isize);
                            *sp = (v as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                (v as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v_0: png_uint_16 = 0;
                        v_0 = (((*sp as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                            + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        if v_0 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.gray as ::core::ffi::c_int
                        {
                            *sp = ((*png_ptr).background.gray as ::core::ffi::c_int
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                ((*png_ptr).background.gray as ::core::ffi::c_int
                                    & 0xff as ::core::ffi::c_int)
                                    as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
            }
            _ => {}
        },
        PNG_COLOR_TYPE_RGB => {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                if !gamma_table.is_null() {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int
                            == (*png_ptr).trans_color.red as ::core::ffi::c_int
                            && *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                == (*png_ptr).trans_color.green as ::core::ffi::c_int
                            && *sp.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                == (*png_ptr).trans_color.blue as ::core::ffi::c_int
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.green as png_byte;
                            *sp.offset(2 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.blue as png_byte;
                        } else {
                            *sp = *gamma_table.offset(*sp as isize);
                            *sp.offset(1 as ::core::ffi::c_int as isize) = *gamma_table
                                .offset(*sp.offset(1 as ::core::ffi::c_int as isize) as isize);
                            *sp.offset(2 as ::core::ffi::c_int as isize) = *gamma_table
                                .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(3 as ::core::ffi::c_int as isize);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int
                            == (*png_ptr).trans_color.red as ::core::ffi::c_int
                            && *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                == (*png_ptr).trans_color.green as ::core::ffi::c_int
                            && *sp.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                == (*png_ptr).trans_color.blue as ::core::ffi::c_int
                        {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.green as png_byte;
                            *sp.offset(2 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.blue as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(3 as ::core::ffi::c_int as isize);
                    }
                }
            } else if !gamma_16.is_null() {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut r: png_uint_16 = (((*sp as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    let mut g_1: png_uint_16 = (((*sp.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    let mut b: png_uint_16 = (((*sp.offset(4 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    if r as ::core::ffi::c_int == (*png_ptr).trans_color.red as ::core::ffi::c_int
                        && g_1 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.green as ::core::ffi::c_int
                        && b as ::core::ffi::c_int
                            == (*png_ptr).trans_color.blue as ::core::ffi::c_int
                    {
                        *sp = ((*png_ptr).background.red as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.red
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(2 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(4 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    } else {
                        let mut v_1: png_uint_16 = *(*gamma_16.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_1 = *(*gamma_16.offset(
                            (*sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                        *sp.offset(2 as ::core::ffi::c_int as isize) = (v_1 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) =
                            (v_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_1 = *(*gamma_16.offset(
                            (*sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(4 as ::core::ffi::c_int as isize) as isize);
                        *sp.offset(4 as ::core::ffi::c_int as isize) = (v_1 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) =
                            (v_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(6 as ::core::ffi::c_int as isize);
                }
            } else {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut r_0: png_uint_16 = (((*sp as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    let mut g_2: png_uint_16 = (((*sp.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    let mut b_0: png_uint_16 = (((*sp.offset(4 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    if r_0 as ::core::ffi::c_int == (*png_ptr).trans_color.red as ::core::ffi::c_int
                        && g_2 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.green as ::core::ffi::c_int
                        && b_0 as ::core::ffi::c_int
                            == (*png_ptr).trans_color.blue as ::core::ffi::c_int
                    {
                        *sp = ((*png_ptr).background.red as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.red
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(2 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(4 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(6 as ::core::ffi::c_int as isize);
                }
            }
        }
        PNG_COLOR_TYPE_GRAY_ALPHA => {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut a: png_uint_16 =
                            *sp.offset(1 as ::core::ffi::c_int as isize) as png_uint_16;
                        if a as ::core::ffi::c_int == 0xff as ::core::ffi::c_int {
                            *sp = *gamma_table.offset(*sp as isize);
                        } else if a as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else {
                            let mut v_2: png_byte = 0;
                            let mut w: png_byte = 0;
                            v_2 = *gamma_to_1.offset(*sp as isize);
                            let mut temp: png_uint_16 = (v_2 as png_uint_16 as ::core::ffi::c_int
                                * a as ::core::ffi::c_int
                                + (*png_ptr).background_1.gray as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int - a as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w = (temp as ::core::ffi::c_int
                                + (temp as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            if optimize == 0 as ::core::ffi::c_int {
                                w = *gamma_from_1.offset(w as isize);
                            }
                            *sp = w;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut a_0: png_byte = *sp.offset(1 as ::core::ffi::c_int as isize);
                        if a_0 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                            *sp = (*png_ptr).background.gray as png_byte;
                        } else if (a_0 as ::core::ffi::c_int) < 0xff as ::core::ffi::c_int {
                            let mut temp_0: png_uint_16 = (*sp as png_uint_16 as ::core::ffi::c_int
                                * a_0 as png_uint_16 as ::core::ffi::c_int
                                + (*png_ptr).background.gray as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - a_0 as png_uint_16 as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            *sp = (temp_0 as ::core::ffi::c_int
                                + (temp_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                    }
                }
            } else if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null()
            {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut a_1: png_uint_16 = (((*sp.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    if a_1 as ::core::ffi::c_int
                        == 0xffff as ::core::ffi::c_int as png_uint_16 as ::core::ffi::c_int
                    {
                        let mut v_3: png_uint_16 = 0;
                        v_3 = *(*gamma_16.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_3 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_3 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    } else if a_1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                        *sp = ((*png_ptr).background.gray as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.gray
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    } else {
                        let mut g_3: png_uint_16 = 0;
                        let mut v_4: png_uint_16 = 0;
                        let mut w_0: png_uint_16 = 0;
                        g_3 = *(*gamma_16_to_1.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        let mut temp_1: png_uint_32 = (g_3 as ::core::ffi::c_uint)
                            .wrapping_mul(a_1 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background_1.gray as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_1 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        v_4 = (0xffff as png_uint_32
                            & temp_1.wrapping_add(temp_1 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        if optimize != 0 as ::core::ffi::c_int {
                            w_0 = v_4;
                        } else {
                            w_0 = *(*gamma_16_from_1.offset(
                                ((v_4 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                                    >> gamma_shift) as isize,
                            ))
                            .offset(
                                (v_4 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize,
                            );
                        }
                        *sp = (w_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (w_0 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(4 as ::core::ffi::c_int as isize);
                }
            } else {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut a_2: png_uint_16 = (((*sp.offset(2 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int)
                        << 8 as ::core::ffi::c_int)
                        + *sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                        as png_uint_16;
                    if a_2 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                        *sp = ((*png_ptr).background.gray as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.gray
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    } else if (a_2 as ::core::ffi::c_int) < 0xffff as ::core::ffi::c_int {
                        let mut g_4: png_uint_16 = 0;
                        let mut v_5: png_uint_16 = 0;
                        g_4 = (((*sp as ::core::ffi::c_int) << 8 as ::core::ffi::c_int)
                            + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        let mut temp_2: png_uint_32 = (g_4 as ::core::ffi::c_uint)
                            .wrapping_mul(a_2 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background.gray as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_2 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        v_5 = (0xffff as png_uint_32
                            & temp_2.wrapping_add(temp_2 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        *sp = (v_5 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_5 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(4 as ::core::ffi::c_int as isize);
                }
            }
        }
        PNG_COLOR_TYPE_RGB_ALPHA => {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                if !gamma_to_1.is_null() && !gamma_from_1.is_null() && !gamma_table.is_null() {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut a_3: png_byte = *sp.offset(3 as ::core::ffi::c_int as isize);
                        if a_3 as ::core::ffi::c_int == 0xff as ::core::ffi::c_int {
                            *sp = *gamma_table.offset(*sp as isize);
                            *sp.offset(1 as ::core::ffi::c_int as isize) = *gamma_table
                                .offset(*sp.offset(1 as ::core::ffi::c_int as isize) as isize);
                            *sp.offset(2 as ::core::ffi::c_int as isize) = *gamma_table
                                .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                        } else if a_3 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.green as png_byte;
                            *sp.offset(2 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.blue as png_byte;
                        } else {
                            let mut v_6: png_byte = 0;
                            let mut w_1: png_byte = 0;
                            v_6 = *gamma_to_1.offset(*sp as isize);
                            let mut temp_3: png_uint_16 = (v_6 as png_uint_16 as ::core::ffi::c_int
                                * a_3 as png_uint_16 as ::core::ffi::c_int
                                + (*png_ptr).background_1.red as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - a_3 as png_uint_16 as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w_1 = (temp_3 as ::core::ffi::c_int
                                + (temp_3 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            if optimize == 0 as ::core::ffi::c_int {
                                w_1 = *gamma_from_1.offset(w_1 as isize);
                            }
                            *sp = w_1;
                            v_6 = *gamma_to_1
                                .offset(*sp.offset(1 as ::core::ffi::c_int as isize) as isize);
                            let mut temp_4: png_uint_16 = (v_6 as png_uint_16 as ::core::ffi::c_int
                                * a_3 as png_uint_16 as ::core::ffi::c_int
                                + (*png_ptr).background_1.green as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - a_3 as png_uint_16 as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w_1 = (temp_4 as ::core::ffi::c_int
                                + (temp_4 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            if optimize == 0 as ::core::ffi::c_int {
                                w_1 = *gamma_from_1.offset(w_1 as isize);
                            }
                            *sp.offset(1 as ::core::ffi::c_int as isize) = w_1;
                            v_6 = *gamma_to_1
                                .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                            let mut temp_5: png_uint_16 = (v_6 as png_uint_16 as ::core::ffi::c_int
                                * a_3 as png_uint_16 as ::core::ffi::c_int
                                + (*png_ptr).background_1.blue as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - a_3 as png_uint_16 as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            w_1 = (temp_5 as ::core::ffi::c_int
                                + (temp_5 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            if optimize == 0 as ::core::ffi::c_int {
                                w_1 = *gamma_from_1.offset(w_1 as isize);
                            }
                            *sp.offset(2 as ::core::ffi::c_int as isize) = w_1;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(4 as ::core::ffi::c_int as isize);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut a_4: png_byte = *sp.offset(3 as ::core::ffi::c_int as isize);
                        if a_4 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                            *sp = (*png_ptr).background.red as png_byte;
                            *sp.offset(1 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.green as png_byte;
                            *sp.offset(2 as ::core::ffi::c_int as isize) =
                                (*png_ptr).background.blue as png_byte;
                        } else if (a_4 as ::core::ffi::c_int) < 0xff as ::core::ffi::c_int {
                            let mut temp_6: png_uint_16 = (*sp as png_uint_16 as ::core::ffi::c_int
                                * a_4 as png_uint_16 as ::core::ffi::c_int
                                + (*png_ptr).background.red as ::core::ffi::c_int
                                    * (255 as ::core::ffi::c_int
                                        - a_4 as png_uint_16 as ::core::ffi::c_int)
                                        as png_uint_16
                                        as ::core::ffi::c_int
                                + 128 as ::core::ffi::c_int)
                                as png_uint_16;
                            *sp = (temp_6 as ::core::ffi::c_int
                                + (temp_6 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            let mut temp_7: png_uint_16 =
                                (*sp.offset(1 as ::core::ffi::c_int as isize) as png_uint_16
                                    as ::core::ffi::c_int
                                    * a_4 as png_uint_16 as ::core::ffi::c_int
                                    + (*png_ptr).background.green as ::core::ffi::c_int
                                        * (255 as ::core::ffi::c_int
                                            - a_4 as png_uint_16 as ::core::ffi::c_int)
                                            as png_uint_16
                                            as ::core::ffi::c_int
                                    + 128 as ::core::ffi::c_int)
                                    as png_uint_16;
                            *sp.offset(1 as ::core::ffi::c_int as isize) = (temp_7
                                as ::core::ffi::c_int
                                + (temp_7 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                            let mut temp_8: png_uint_16 =
                                (*sp.offset(2 as ::core::ffi::c_int as isize) as png_uint_16
                                    as ::core::ffi::c_int
                                    * a_4 as png_uint_16 as ::core::ffi::c_int
                                    + (*png_ptr).background.blue as ::core::ffi::c_int
                                        * (255 as ::core::ffi::c_int
                                            - a_4 as png_uint_16 as ::core::ffi::c_int)
                                            as png_uint_16
                                            as ::core::ffi::c_int
                                    + 128 as ::core::ffi::c_int)
                                    as png_uint_16;
                            *sp.offset(2 as ::core::ffi::c_int as isize) = (temp_8
                                as ::core::ffi::c_int
                                + (temp_8 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int)
                                >> 8 as ::core::ffi::c_int
                                & 0xff as ::core::ffi::c_int)
                                as png_byte;
                        }
                        i = i.wrapping_add(1);
                        sp = sp.offset(4 as ::core::ffi::c_int as isize);
                    }
                }
            } else if !gamma_16.is_null() && !gamma_16_from_1.is_null() && !gamma_16_to_1.is_null()
            {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut a_5: png_uint_16 =
                        (((*sp.offset(6 as ::core::ffi::c_int as isize) as png_uint_16
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *sp.offset(7 as ::core::ffi::c_int as isize) as png_uint_16
                                as ::core::ffi::c_int) as png_uint_16;
                    if a_5 as ::core::ffi::c_int
                        == 0xffff as ::core::ffi::c_int as png_uint_16 as ::core::ffi::c_int
                    {
                        let mut v_7: png_uint_16 = 0;
                        v_7 = *(*gamma_16.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_7 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_7 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_7 = *(*gamma_16.offset(
                            (*sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                        *sp.offset(2 as ::core::ffi::c_int as isize) = (v_7 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) =
                            (v_7 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_7 = *(*gamma_16.offset(
                            (*sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(4 as ::core::ffi::c_int as isize) as isize);
                        *sp.offset(4 as ::core::ffi::c_int as isize) = (v_7 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) =
                            (v_7 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    } else if a_5 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                        *sp = ((*png_ptr).background.red as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.red
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(2 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(4 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    } else {
                        let mut v_8: png_uint_16 = 0;
                        let mut w_2: png_uint_16 = 0;
                        v_8 = *(*gamma_16_to_1.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        let mut temp_9: png_uint_32 = (v_8 as ::core::ffi::c_uint)
                            .wrapping_mul(a_5 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background_1.red as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_5 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        w_2 = (0xffff as png_uint_32
                            & temp_9.wrapping_add(temp_9 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        if optimize == 0 as ::core::ffi::c_int {
                            w_2 = *(*gamma_16_from_1.offset(
                                ((w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                                    >> gamma_shift) as isize,
                            ))
                            .offset(
                                (w_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize,
                            );
                        }
                        *sp = (w_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_8 = *(*gamma_16_to_1.offset(
                            (*sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(2 as ::core::ffi::c_int as isize) as isize);
                        let mut temp_10: png_uint_32 = (v_8 as ::core::ffi::c_uint)
                            .wrapping_mul(a_5 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background_1.green as ::core::ffi::c_uint)
                                    .wrapping_mul(
                                        (65535 as ::core::ffi::c_uint)
                                            .wrapping_sub(a_5 as ::core::ffi::c_uint),
                                    ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        w_2 = (0xffff as png_uint_32
                            & temp_10.wrapping_add(temp_10 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        if optimize == 0 as ::core::ffi::c_int {
                            w_2 = *(*gamma_16_from_1.offset(
                                ((w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                                    >> gamma_shift) as isize,
                            ))
                            .offset(
                                (w_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize,
                            );
                        }
                        *sp.offset(2 as ::core::ffi::c_int as isize) = (w_2 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) =
                            (w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        v_8 = *(*gamma_16_to_1.offset(
                            (*sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp.offset(4 as ::core::ffi::c_int as isize) as isize);
                        let mut temp_11: png_uint_32 = (v_8 as ::core::ffi::c_uint)
                            .wrapping_mul(a_5 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background_1.blue as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_5 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        w_2 = (0xffff as png_uint_32
                            & temp_11.wrapping_add(temp_11 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        if optimize == 0 as ::core::ffi::c_int {
                            w_2 = *(*gamma_16_from_1.offset(
                                ((w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
                                    >> gamma_shift) as isize,
                            ))
                            .offset(
                                (w_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int) as isize,
                            );
                        }
                        *sp.offset(4 as ::core::ffi::c_int as isize) = (w_2 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) =
                            (w_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(8 as ::core::ffi::c_int as isize);
                }
            } else {
                sp = row;
                i = 0 as png_uint_32;
                while i < row_width {
                    let mut a_6: png_uint_16 =
                        (((*sp.offset(6 as ::core::ffi::c_int as isize) as png_uint_16
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *sp.offset(7 as ::core::ffi::c_int as isize) as png_uint_16
                                as ::core::ffi::c_int) as png_uint_16;
                    if a_6 as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
                        *sp = ((*png_ptr).background.red as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) = ((*png_ptr).background.red
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(2 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) = ((*png_ptr).background.green
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(4 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) = ((*png_ptr).background.blue
                            as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                    } else if (a_6 as ::core::ffi::c_int) < 0xffff as ::core::ffi::c_int {
                        let mut v_9: png_uint_16 = 0;
                        let mut r_1: png_uint_16 = (((*sp as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        let mut g_5: png_uint_16 = (((*sp.offset(2 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *sp.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        let mut b_1: png_uint_16 = (((*sp.offset(4 as ::core::ffi::c_int as isize)
                            as ::core::ffi::c_int)
                            << 8 as ::core::ffi::c_int)
                            + *sp.offset(5 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                            as png_uint_16;
                        let mut temp_12: png_uint_32 = (r_1 as ::core::ffi::c_uint)
                            .wrapping_mul(a_6 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background.red as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_6 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        v_9 = (0xffff as png_uint_32
                            & temp_12.wrapping_add(temp_12 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        *sp = (v_9 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_9 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        let mut temp_13: png_uint_32 = (g_5 as ::core::ffi::c_uint)
                            .wrapping_mul(a_6 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background.green as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_6 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        v_9 = (0xffff as png_uint_32
                            & temp_13.wrapping_add(temp_13 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        *sp.offset(2 as ::core::ffi::c_int as isize) = (v_9 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(3 as ::core::ffi::c_int as isize) =
                            (v_9 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        let mut temp_14: png_uint_32 = (b_1 as ::core::ffi::c_uint)
                            .wrapping_mul(a_6 as ::core::ffi::c_uint)
                            .wrapping_add(
                                ((*png_ptr).background.blue as ::core::ffi::c_uint).wrapping_mul(
                                    (65535 as ::core::ffi::c_uint)
                                        .wrapping_sub(a_6 as ::core::ffi::c_uint),
                                ),
                            )
                            .wrapping_add(32768 as ::core::ffi::c_uint);
                        v_9 = (0xffff as png_uint_32
                            & temp_14.wrapping_add(temp_14 >> 16 as ::core::ffi::c_int)
                                >> 16 as ::core::ffi::c_int)
                            as png_uint_16;
                        *sp.offset(4 as ::core::ffi::c_int as isize) = (v_9 as ::core::ffi::c_int
                            >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int)
                            as png_byte;
                        *sp.offset(5 as ::core::ffi::c_int as isize) =
                            (v_9 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    }
                    i = i.wrapping_add(1);
                    sp = sp.offset(8 as ::core::ffi::c_int as isize);
                }
            }
        }
        _ => {}
    };
}
unsafe extern "C" fn png_do_gamma(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut png_ptr: png_structrp,
) {
    let mut gamma_table: png_const_bytep = (*png_ptr).gamma_table as png_const_bytep;
    let mut gamma_16_table: png_const_uint_16pp = (*png_ptr).gamma_16_table as png_const_uint_16pp;
    let mut gamma_shift: ::core::ffi::c_int = (*png_ptr).gamma_shift;
    let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).bit_depth as ::core::ffi::c_int <= 8 as ::core::ffi::c_int
        && !gamma_table.is_null()
        || (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int
            && !gamma_16_table.is_null()
    {
        match (*row_info).color_type as ::core::ffi::c_int {
            PNG_COLOR_TYPE_RGB => {
                if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        i = i.wrapping_add(1);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v: png_uint_16 = 0;
                        v = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        v = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        v = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        i = i.wrapping_add(1);
                    }
                }
            }
            PNG_COLOR_TYPE_RGB_ALPHA => {
                if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        sp = sp.offset(1);
                        i = i.wrapping_add(1);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v_0: png_uint_16 = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_0 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        v_0 = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_0 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        v_0 = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_0 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_0 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(4 as ::core::ffi::c_int as isize);
                        i = i.wrapping_add(1);
                    }
                }
            }
            PNG_COLOR_TYPE_GRAY_ALPHA => {
                if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        i = i.wrapping_add(1);
                    }
                } else {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v_1: png_uint_16 = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_1 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_1 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(4 as ::core::ffi::c_int as isize);
                        i = i.wrapping_add(1);
                    }
                }
            }
            PNG_COLOR_TYPE_GRAY => {
                if (*row_info).bit_depth as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut a: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0xc0 as ::core::ffi::c_int;
                        let mut b: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0x30 as ::core::ffi::c_int;
                        let mut c: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0xc as ::core::ffi::c_int;
                        let mut d: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0x3 as ::core::ffi::c_int;
                        *sp = (*gamma_table.offset(
                            (a | a >> 2 as ::core::ffi::c_int
                                | a >> 4 as ::core::ffi::c_int
                                | a >> 6 as ::core::ffi::c_int)
                                as isize,
                        ) as ::core::ffi::c_int
                            & 0xc0 as ::core::ffi::c_int
                            | *gamma_table.offset(
                                (b << 2 as ::core::ffi::c_int
                                    | b
                                    | b >> 2 as ::core::ffi::c_int
                                    | b >> 4 as ::core::ffi::c_int)
                                    as isize,
                            ) as ::core::ffi::c_int
                                >> 2 as ::core::ffi::c_int
                                & 0x30 as ::core::ffi::c_int
                            | *gamma_table.offset(
                                (c << 4 as ::core::ffi::c_int
                                    | c << 2 as ::core::ffi::c_int
                                    | c
                                    | c >> 2 as ::core::ffi::c_int)
                                    as isize,
                            ) as ::core::ffi::c_int
                                >> 4 as ::core::ffi::c_int
                                & 0xc as ::core::ffi::c_int
                            | *gamma_table.offset(
                                (d << 6 as ::core::ffi::c_int
                                    | d << 4 as ::core::ffi::c_int
                                    | d << 2 as ::core::ffi::c_int
                                    | d) as isize,
                            ) as ::core::ffi::c_int
                                >> 6 as ::core::ffi::c_int)
                            as png_byte;
                        sp = sp.offset(1);
                        i = (i as ::core::ffi::c_uint).wrapping_add(4 as ::core::ffi::c_uint)
                            as png_uint_32 as png_uint_32;
                    }
                }
                if (*row_info).bit_depth as ::core::ffi::c_int == 4 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut msb: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0xf0 as ::core::ffi::c_int;
                        let mut lsb: ::core::ffi::c_int =
                            *sp as ::core::ffi::c_int & 0xf as ::core::ffi::c_int;
                        *sp = (*gamma_table.offset((msb | msb >> 4 as ::core::ffi::c_int) as isize)
                            as ::core::ffi::c_int
                            & 0xf0 as ::core::ffi::c_int
                            | *gamma_table.offset((lsb << 4 as ::core::ffi::c_int | lsb) as isize)
                                as ::core::ffi::c_int
                                >> 4 as ::core::ffi::c_int)
                            as png_byte;
                        sp = sp.offset(1);
                        i = (i as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint)
                            as png_uint_32 as png_uint_32;
                    }
                } else if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        *sp = *gamma_table.offset(*sp as isize);
                        sp = sp.offset(1);
                        i = i.wrapping_add(1);
                    }
                } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
                    sp = row;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        let mut v_2: png_uint_16 = *(*gamma_16_table.offset(
                            (*sp.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                                >> gamma_shift) as isize,
                        ))
                        .offset(*sp as isize);
                        *sp = (v_2 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                            & 0xff as ::core::ffi::c_int) as png_byte;
                        *sp.offset(1 as ::core::ffi::c_int as isize) =
                            (v_2 as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                        sp = sp.offset(2 as ::core::ffi::c_int as isize);
                        i = i.wrapping_add(1);
                    }
                }
            }
            _ => {}
        }
    }
}
unsafe extern "C" fn png_do_encode_alpha(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut png_ptr: png_structrp,
) {
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
        != 0 as ::core::ffi::c_int
    {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut table: png_bytep = (*png_ptr).gamma_from_1;
            if !table.is_null() {
                let mut step: ::core::ffi::c_int =
                    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR != 0 {
                        4 as ::core::ffi::c_int
                    } else {
                        2 as ::core::ffi::c_int
                    };
                row = row.offset((step - 1 as ::core::ffi::c_int) as isize);
                while row_width > 0 as ::core::ffi::c_uint {
                    *row = *table.offset(*row as isize);
                    row_width = row_width.wrapping_sub(1);
                    row = row.offset(step as isize);
                }
                return;
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            let mut table_0: png_uint_16pp = (*png_ptr).gamma_16_from_1;
            let mut gamma_shift: ::core::ffi::c_int = (*png_ptr).gamma_shift;
            if !table_0.is_null() {
                let mut step_0: ::core::ffi::c_int =
                    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR != 0 {
                        8 as ::core::ffi::c_int
                    } else {
                        4 as ::core::ffi::c_int
                    };
                row = row.offset((step_0 - 2 as ::core::ffi::c_int) as isize);
                while row_width > 0 as ::core::ffi::c_uint {
                    let mut v: png_uint_16 = 0;
                    v = *(*table_0.offset(
                        (*row.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                            >> gamma_shift) as isize,
                    ))
                    .offset(*row as isize);
                    *row = (v as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                        & 0xff as ::core::ffi::c_int) as png_byte;
                    *row.offset(1 as ::core::ffi::c_int as isize) =
                        (v as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
                    row_width = row_width.wrapping_sub(1);
                    row = row.offset(step_0 as isize);
                }
                return;
            }
        }
    }
    png_warning(
        png_ptr,
        b"png_do_encode_alpha: unexpected call\0" as *const u8 as png_const_charp,
    );
}
unsafe extern "C" fn png_do_expand_palette(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut palette: png_const_colorp,
    mut trans_alpha: png_const_bytep,
    mut num_trans: ::core::ffi::c_int,
) {
    let mut shift: ::core::ffi::c_int = 0;
    let mut value: ::core::ffi::c_int = 0;
    let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        if ((*row_info).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
            match (*row_info).bit_depth as ::core::ffi::c_int {
                1 => {
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 3 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = 7 as ::core::ffi::c_int
                        - ((row_width as ::core::ffi::c_uint)
                            .wrapping_add(7 as ::core::ffi::c_uint)
                            & 0x7 as ::core::ffi::c_uint)
                            as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int >> shift & 0x1 as ::core::ffi::c_int != 0 {
                            *dp = 1 as png_byte;
                        } else {
                            *dp = 0 as png_byte;
                        }
                        if shift == 7 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift += 1;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                2 => {
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 2 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = ((3 as ::core::ffi::c_uint).wrapping_sub(
                        (row_width as ::core::ffi::c_uint).wrapping_add(3 as ::core::ffi::c_uint)
                            & 0x3 as ::core::ffi::c_uint,
                    ) << 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        value = *sp as ::core::ffi::c_int >> shift & 0x3 as ::core::ffi::c_int;
                        *dp = value as png_byte;
                        if shift == 6 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift += 2 as ::core::ffi::c_int;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                4 => {
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 1 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = ((row_width as ::core::ffi::c_uint & 0x1 as ::core::ffi::c_uint)
                        << 2 as ::core::ffi::c_int)
                        as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        value = *sp as ::core::ffi::c_int >> shift & 0xf as ::core::ffi::c_int;
                        *dp = value as png_byte;
                        if shift == 4 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift += 4 as ::core::ffi::c_int;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                _ => {}
            }
            (*row_info).bit_depth = 8 as png_byte;
            (*row_info).pixel_depth = 8 as png_byte;
            (*row_info).rowbytes = row_width as size_t;
        }
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if num_trans > 0 as ::core::ffi::c_int {
                sp = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                dp = row
                    .offset(((row_width as size_t) << 2 as ::core::ffi::c_int) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                i = 0 as png_uint_32;
                while i < row_width {
                    if *sp as ::core::ffi::c_int >= num_trans {
                        let fresh129 = dp;
                        dp = dp.offset(-1);
                        *fresh129 = 0xff as png_byte;
                    } else {
                        let fresh130 = dp;
                        dp = dp.offset(-1);
                        *fresh130 = *trans_alpha.offset(*sp as isize);
                    }
                    let fresh131 = dp;
                    dp = dp.offset(-1);
                    *fresh131 = (*palette.offset(*sp as isize)).blue;
                    let fresh132 = dp;
                    dp = dp.offset(-1);
                    *fresh132 = (*palette.offset(*sp as isize)).green;
                    let fresh133 = dp;
                    dp = dp.offset(-1);
                    *fresh133 = (*palette.offset(*sp as isize)).red;
                    sp = sp.offset(-1);
                    i = i.wrapping_add(1);
                }
                (*row_info).bit_depth = 8 as png_byte;
                (*row_info).pixel_depth = 32 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(4 as size_t);
                (*row_info).color_type = 6 as png_byte;
                (*row_info).channels = 4 as png_byte;
            } else {
                sp = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                dp = row
                    .offset((row_width as size_t).wrapping_mul(3 as size_t) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                i = 0 as png_uint_32;
                while i < row_width {
                    let fresh134 = dp;
                    dp = dp.offset(-1);
                    *fresh134 = (*palette.offset(*sp as isize)).blue;
                    let fresh135 = dp;
                    dp = dp.offset(-1);
                    *fresh135 = (*palette.offset(*sp as isize)).green;
                    let fresh136 = dp;
                    dp = dp.offset(-1);
                    *fresh136 = (*palette.offset(*sp as isize)).red;
                    sp = sp.offset(-1);
                    i = i.wrapping_add(1);
                }
                (*row_info).bit_depth = 8 as png_byte;
                (*row_info).pixel_depth = 24 as png_byte;
                (*row_info).rowbytes = (row_width as size_t).wrapping_mul(3 as size_t);
                (*row_info).color_type = 2 as png_byte;
                (*row_info).channels = 3 as png_byte;
            }
        }
    }
}
unsafe extern "C" fn png_do_expand(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut trans_color: png_const_color_16p,
) {
    let mut shift: ::core::ffi::c_int = 0;
    let mut value: ::core::ffi::c_int = 0;
    let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY {
        let mut gray: ::core::ffi::c_uint = (if !trans_color.is_null() {
            (*trans_color).gray as ::core::ffi::c_int
        } else {
            0 as ::core::ffi::c_int
        }) as ::core::ffi::c_uint;
        if ((*row_info).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
            match (*row_info).bit_depth as ::core::ffi::c_int {
                1 => {
                    gray = (gray & 0x1 as ::core::ffi::c_uint)
                        .wrapping_mul(0xff as ::core::ffi::c_uint);
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 3 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = 7 as ::core::ffi::c_int
                        - ((row_width as ::core::ffi::c_uint)
                            .wrapping_add(7 as ::core::ffi::c_uint)
                            & 0x7 as ::core::ffi::c_uint)
                            as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        if *sp as ::core::ffi::c_int >> shift & 0x1 as ::core::ffi::c_int != 0 {
                            *dp = 0xff as png_byte;
                        } else {
                            *dp = 0 as png_byte;
                        }
                        if shift == 7 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift += 1;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                2 => {
                    gray = (gray & 0x3 as ::core::ffi::c_uint)
                        .wrapping_mul(0x55 as ::core::ffi::c_uint);
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 2 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = ((3 as ::core::ffi::c_uint).wrapping_sub(
                        (row_width as ::core::ffi::c_uint).wrapping_add(3 as ::core::ffi::c_uint)
                            & 0x3 as ::core::ffi::c_uint,
                    ) << 1 as ::core::ffi::c_int) as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        value = *sp as ::core::ffi::c_int >> shift & 0x3 as ::core::ffi::c_int;
                        *dp = (value
                            | value << 2 as ::core::ffi::c_int
                            | value << 4 as ::core::ffi::c_int
                            | value << 6 as ::core::ffi::c_int)
                            as png_byte;
                        if shift == 6 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift += 2 as ::core::ffi::c_int;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                4 => {
                    gray = (gray & 0xf as ::core::ffi::c_uint)
                        .wrapping_mul(0x11 as ::core::ffi::c_uint);
                    sp = row.offset(
                        ((row_width as ::core::ffi::c_uint).wrapping_sub(1 as ::core::ffi::c_uint)
                            >> 1 as ::core::ffi::c_int) as size_t as isize,
                    );
                    dp = row
                        .offset(row_width as size_t as isize)
                        .offset(-(1 as ::core::ffi::c_int as isize));
                    shift = ((1 as ::core::ffi::c_uint).wrapping_sub(
                        (row_width as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint)
                            & 0x1 as ::core::ffi::c_uint,
                    ) << 2 as ::core::ffi::c_int) as ::core::ffi::c_int;
                    i = 0 as png_uint_32;
                    while i < row_width {
                        value = *sp as ::core::ffi::c_int >> shift & 0xf as ::core::ffi::c_int;
                        *dp = (value | value << 4 as ::core::ffi::c_int) as png_byte;
                        if shift == 4 as ::core::ffi::c_int {
                            shift = 0 as ::core::ffi::c_int;
                            sp = sp.offset(-1);
                        } else {
                            shift = 4 as ::core::ffi::c_int;
                        }
                        dp = dp.offset(-1);
                        i = i.wrapping_add(1);
                    }
                }
                _ => {}
            }
            (*row_info).bit_depth = 8 as png_byte;
            (*row_info).pixel_depth = 8 as png_byte;
            (*row_info).rowbytes = row_width as size_t;
        }
        if !trans_color.is_null() {
            if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
                gray = gray & 0xff as ::core::ffi::c_uint;
                sp = row
                    .offset(row_width as size_t as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                dp = row
                    .offset(((row_width as size_t) << 1 as ::core::ffi::c_int) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                i = 0 as png_uint_32;
                while i < row_width {
                    if *sp as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint == gray {
                        let fresh93 = dp;
                        dp = dp.offset(-1);
                        *fresh93 = 0 as png_byte;
                    } else {
                        let fresh94 = dp;
                        dp = dp.offset(-1);
                        *fresh94 = 0xff as png_byte;
                    }
                    let fresh95 = sp;
                    sp = sp.offset(-1);
                    let fresh96 = dp;
                    dp = dp.offset(-1);
                    *fresh96 = *fresh95;
                    i = i.wrapping_add(1);
                }
            } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
                let mut gray_high: ::core::ffi::c_uint =
                    gray >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint;
                let mut gray_low: ::core::ffi::c_uint = gray & 0xff as ::core::ffi::c_uint;
                sp = row
                    .offset((*row_info).rowbytes as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                dp = row
                    .offset(((*row_info).rowbytes << 1 as ::core::ffi::c_int) as isize)
                    .offset(-(1 as ::core::ffi::c_int as isize));
                i = 0 as png_uint_32;
                while i < row_width {
                    if *sp.offset(-(1 as ::core::ffi::c_int as isize)) as ::core::ffi::c_uint
                        & 0xff as ::core::ffi::c_uint
                        == gray_high
                        && *sp as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint == gray_low
                    {
                        let fresh97 = dp;
                        dp = dp.offset(-1);
                        *fresh97 = 0 as png_byte;
                        let fresh98 = dp;
                        dp = dp.offset(-1);
                        *fresh98 = 0 as png_byte;
                    } else {
                        let fresh99 = dp;
                        dp = dp.offset(-1);
                        *fresh99 = 0xff as png_byte;
                        let fresh100 = dp;
                        dp = dp.offset(-1);
                        *fresh100 = 0xff as png_byte;
                    }
                    let fresh101 = sp;
                    sp = sp.offset(-1);
                    let fresh102 = dp;
                    dp = dp.offset(-1);
                    *fresh102 = *fresh101;
                    let fresh103 = sp;
                    sp = sp.offset(-1);
                    let fresh104 = dp;
                    dp = dp.offset(-1);
                    *fresh104 = *fresh103;
                    i = i.wrapping_add(1);
                }
            }
            (*row_info).color_type = PNG_COLOR_TYPE_GRAY_ALPHA as png_byte;
            (*row_info).channels = 2 as png_byte;
            (*row_info).pixel_depth = (((*row_info).bit_depth as ::core::ffi::c_int)
                << 1 as ::core::ffi::c_int) as png_byte;
            (*row_info).rowbytes =
                if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
                } else {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t)
                        .wrapping_add(7 as size_t)
                        >> 3 as ::core::ffi::c_int
                };
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
        && !trans_color.is_null()
    {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut red: png_byte =
                ((*trans_color).red as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
            let mut green: png_byte = ((*trans_color).green as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            let mut blue: png_byte = ((*trans_color).blue as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            sp = row
                .offset((*row_info).rowbytes as isize)
                .offset(-(1 as ::core::ffi::c_int as isize));
            dp = row
                .offset(((row_width as size_t) << 2 as ::core::ffi::c_int) as isize)
                .offset(-(1 as ::core::ffi::c_int as isize));
            i = 0 as png_uint_32;
            while i < row_width {
                if *sp.offset(-(2 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                    == red as ::core::ffi::c_int
                    && *sp.offset(-(1 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                        == green as ::core::ffi::c_int
                    && *sp as ::core::ffi::c_int == blue as ::core::ffi::c_int
                {
                    let fresh105 = dp;
                    dp = dp.offset(-1);
                    *fresh105 = 0 as png_byte;
                } else {
                    let fresh106 = dp;
                    dp = dp.offset(-1);
                    *fresh106 = 0xff as png_byte;
                }
                let fresh107 = sp;
                sp = sp.offset(-1);
                let fresh108 = dp;
                dp = dp.offset(-1);
                *fresh108 = *fresh107;
                let fresh109 = sp;
                sp = sp.offset(-1);
                let fresh110 = dp;
                dp = dp.offset(-1);
                *fresh110 = *fresh109;
                let fresh111 = sp;
                sp = sp.offset(-1);
                let fresh112 = dp;
                dp = dp.offset(-1);
                *fresh112 = *fresh111;
                i = i.wrapping_add(1);
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            let mut red_high: png_byte = ((*trans_color).red as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            let mut green_high: png_byte =
                ((*trans_color).green as ::core::ffi::c_int >> 8 as ::core::ffi::c_int
                    & 0xff as ::core::ffi::c_int) as png_byte;
            let mut blue_high: png_byte = ((*trans_color).blue as ::core::ffi::c_int
                >> 8 as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            let mut red_low: png_byte =
                ((*trans_color).red as ::core::ffi::c_int & 0xff as ::core::ffi::c_int) as png_byte;
            let mut green_low: png_byte = ((*trans_color).green as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            let mut blue_low: png_byte = ((*trans_color).blue as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_int) as png_byte;
            sp = row
                .offset((*row_info).rowbytes as isize)
                .offset(-(1 as ::core::ffi::c_int as isize));
            dp = row
                .offset(((row_width as size_t) << 3 as ::core::ffi::c_int) as isize)
                .offset(-(1 as ::core::ffi::c_int as isize));
            i = 0 as png_uint_32;
            while i < row_width {
                if *sp.offset(-(5 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                    == red_high as ::core::ffi::c_int
                    && *sp.offset(-(4 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                        == red_low as ::core::ffi::c_int
                    && *sp.offset(-(3 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                        == green_high as ::core::ffi::c_int
                    && *sp.offset(-(2 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                        == green_low as ::core::ffi::c_int
                    && *sp.offset(-(1 as ::core::ffi::c_int as isize)) as ::core::ffi::c_int
                        == blue_high as ::core::ffi::c_int
                    && *sp as ::core::ffi::c_int == blue_low as ::core::ffi::c_int
                {
                    let fresh113 = dp;
                    dp = dp.offset(-1);
                    *fresh113 = 0 as png_byte;
                    let fresh114 = dp;
                    dp = dp.offset(-1);
                    *fresh114 = 0 as png_byte;
                } else {
                    let fresh115 = dp;
                    dp = dp.offset(-1);
                    *fresh115 = 0xff as png_byte;
                    let fresh116 = dp;
                    dp = dp.offset(-1);
                    *fresh116 = 0xff as png_byte;
                }
                let fresh117 = sp;
                sp = sp.offset(-1);
                let fresh118 = dp;
                dp = dp.offset(-1);
                *fresh118 = *fresh117;
                let fresh119 = sp;
                sp = sp.offset(-1);
                let fresh120 = dp;
                dp = dp.offset(-1);
                *fresh120 = *fresh119;
                let fresh121 = sp;
                sp = sp.offset(-1);
                let fresh122 = dp;
                dp = dp.offset(-1);
                *fresh122 = *fresh121;
                let fresh123 = sp;
                sp = sp.offset(-1);
                let fresh124 = dp;
                dp = dp.offset(-1);
                *fresh124 = *fresh123;
                let fresh125 = sp;
                sp = sp.offset(-1);
                let fresh126 = dp;
                dp = dp.offset(-1);
                *fresh126 = *fresh125;
                let fresh127 = sp;
                sp = sp.offset(-1);
                let fresh128 = dp;
                dp = dp.offset(-1);
                *fresh128 = *fresh127;
                i = i.wrapping_add(1);
            }
        }
        (*row_info).color_type = PNG_COLOR_TYPE_RGB_ALPHA as png_byte;
        (*row_info).channels = 4 as png_byte;
        (*row_info).pixel_depth =
            (((*row_info).bit_depth as ::core::ffi::c_int) << 2 as ::core::ffi::c_int) as png_byte;
        (*row_info).rowbytes =
            if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
            } else {
                (row_width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t)
                    .wrapping_add(7 as size_t)
                    >> 3 as ::core::ffi::c_int
            };
    }
}
unsafe extern "C" fn png_do_expand_16(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
        && (*row_info).color_type as ::core::ffi::c_int != PNG_COLOR_TYPE_PALETTE
    {
        let mut sp: *mut png_byte = row.offset((*row_info).rowbytes as isize);
        let mut dp: *mut png_byte = sp.offset((*row_info).rowbytes as isize);
        while dp > sp {
            sp = sp.offset(-1);
            let ref mut fresh42 = *dp.offset(-(1 as ::core::ffi::c_int) as isize);
            *fresh42 = *sp;
            *dp.offset(-(2 as ::core::ffi::c_int) as isize) = *fresh42;
            dp = dp.offset(-(2 as ::core::ffi::c_int as isize));
        }
        (*row_info).rowbytes = ((*row_info).rowbytes as ::core::ffi::c_ulong)
            .wrapping_mul(2 as ::core::ffi::c_ulong) as size_t
            as size_t;
        (*row_info).bit_depth = 16 as png_byte;
        (*row_info).pixel_depth =
            ((*row_info).channels as ::core::ffi::c_int * 16 as ::core::ffi::c_int) as png_byte;
    }
}
unsafe extern "C" fn png_do_quantize(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut palette_lookup: png_const_bytep,
    mut quantize_lookup: png_const_bytep,
) {
    let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut row_width: png_uint_32 = (*row_info).width;
    if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
        if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
            && !palette_lookup.is_null()
        {
            let mut r: ::core::ffi::c_int = 0;
            let mut g: ::core::ffi::c_int = 0;
            let mut b: ::core::ffi::c_int = 0;
            let mut p: ::core::ffi::c_int = 0;
            sp = row;
            dp = row;
            i = 0 as png_uint_32;
            while i < row_width {
                let fresh43 = sp;
                sp = sp.offset(1);
                r = *fresh43 as ::core::ffi::c_int;
                let fresh44 = sp;
                sp = sp.offset(1);
                g = *fresh44 as ::core::ffi::c_int;
                let fresh45 = sp;
                sp = sp.offset(1);
                b = *fresh45 as ::core::ffi::c_int;
                p = (r >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_RED_BITS
                    & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_RED_BITS)
                        - 1 as ::core::ffi::c_int)
                    << PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS
                    | (g >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_GREEN_BITS
                        & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_GREEN_BITS)
                            - 1 as ::core::ffi::c_int)
                        << 5 as ::core::ffi::c_int
                    | b >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_BLUE_BITS
                        & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_BLUE_BITS)
                            - 1 as ::core::ffi::c_int;
                let fresh46 = dp;
                dp = dp.offset(1);
                *fresh46 = *palette_lookup.offset(p as isize);
                i = i.wrapping_add(1);
            }
            (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
            (*row_info).channels = 1 as png_byte;
            (*row_info).pixel_depth = (*row_info).bit_depth;
            (*row_info).rowbytes =
                if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
                } else {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t)
                        .wrapping_add(7 as size_t)
                        >> 3 as ::core::ffi::c_int
                };
        } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
            && !palette_lookup.is_null()
        {
            let mut r_0: ::core::ffi::c_int = 0;
            let mut g_0: ::core::ffi::c_int = 0;
            let mut b_0: ::core::ffi::c_int = 0;
            let mut p_0: ::core::ffi::c_int = 0;
            sp = row;
            dp = row;
            i = 0 as png_uint_32;
            while i < row_width {
                let fresh47 = sp;
                sp = sp.offset(1);
                r_0 = *fresh47 as ::core::ffi::c_int;
                let fresh48 = sp;
                sp = sp.offset(1);
                g_0 = *fresh48 as ::core::ffi::c_int;
                let fresh49 = sp;
                sp = sp.offset(1);
                b_0 = *fresh49 as ::core::ffi::c_int;
                sp = sp.offset(1);
                p_0 = (r_0 >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_RED_BITS
                    & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_RED_BITS)
                        - 1 as ::core::ffi::c_int)
                    << PNG_QUANTIZE_GREEN_BITS + PNG_QUANTIZE_BLUE_BITS
                    | (g_0 >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_GREEN_BITS
                        & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_GREEN_BITS)
                            - 1 as ::core::ffi::c_int)
                        << 5 as ::core::ffi::c_int
                    | b_0 >> 8 as ::core::ffi::c_int - PNG_QUANTIZE_BLUE_BITS
                        & ((1 as ::core::ffi::c_int) << PNG_QUANTIZE_BLUE_BITS)
                            - 1 as ::core::ffi::c_int;
                let fresh50 = dp;
                dp = dp.offset(1);
                *fresh50 = *palette_lookup.offset(p_0 as isize);
                i = i.wrapping_add(1);
            }
            (*row_info).color_type = PNG_COLOR_TYPE_PALETTE as png_byte;
            (*row_info).channels = 1 as png_byte;
            (*row_info).pixel_depth = (*row_info).bit_depth;
            (*row_info).rowbytes =
                if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
                } else {
                    (row_width as size_t)
                        .wrapping_mul((*row_info).pixel_depth as size_t)
                        .wrapping_add(7 as size_t)
                        >> 3 as ::core::ffi::c_int
                };
        } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
            && !quantize_lookup.is_null()
        {
            sp = row;
            i = 0 as png_uint_32;
            while i < row_width {
                *sp = *quantize_lookup.offset(*sp as isize);
                i = i.wrapping_add(1);
                sp = sp.offset(1);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_read_transformations(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
) {
    if (*png_ptr).row_buf.is_null() {
        png_error(
            png_ptr,
            b"NULL row buffer\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_DETECT_UNINITIALIZED
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT == 0 as ::core::ffi::c_uint
    {
        png_error(
            png_ptr,
            b"Uninitialized row\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND != 0 as ::core::ffi::c_uint {
        if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
            png_do_expand_palette(
                png_ptr,
                row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                (*png_ptr).palette as png_const_colorp,
                (*png_ptr).trans_alpha as png_const_bytep,
                (*png_ptr).num_trans as ::core::ffi::c_int,
            );
        } else if (*png_ptr).num_trans as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_tRNS
                != 0 as ::core::ffi::c_uint
        {
            png_do_expand(
                row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                &raw mut (*png_ptr).trans_color as png_const_color_16p,
            );
        } else {
            png_do_expand(
                row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
                ::core::ptr::null::<png_color_16>(),
            );
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_STRIP_ALPHA
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            == 0 as ::core::ffi::c_uint
        && ((*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
            || (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
    {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            0 as ::core::ffi::c_int,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
        != 0 as ::core::ffi::c_uint
    {
        let mut rgb_error: ::core::ffi::c_int = png_do_rgb_to_gray(
            png_ptr,
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
        if rgb_error != 0 as ::core::ffi::c_int {
            (*png_ptr).rgb_to_gray_status = 1 as png_byte;
            if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
                == PNG_RGB_TO_GRAY_WARN
            {
                png_warning(
                    png_ptr,
                    b"png_do_rgb_to_gray found nongray pixel\0" as *const u8 as png_const_charp,
                );
            }
            if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
                == PNG_RGB_TO_GRAY_ERR
            {
                png_error(
                    png_ptr,
                    b"png_do_rgb_to_gray found nongray pixel\0" as *const u8 as png_const_charp,
                );
            }
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GRAY_TO_RGB
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).mode as ::core::ffi::c_uint & PNG_BACKGROUND_IS_GRAY
            == 0 as ::core::ffi::c_uint
    {
        png_do_gray_to_rgb(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE != 0 as ::core::ffi::c_uint {
        png_do_compose(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            png_ptr,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GAMMA != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_RGB_TO_GRAY
            == 0 as ::core::ffi::c_uint
        && !((*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
            && ((*png_ptr).num_trans as ::core::ffi::c_int != 0 as ::core::ffi::c_int
                || (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
                    != 0 as ::core::ffi::c_int))
        && (*png_ptr).color_type as ::core::ffi::c_int != PNG_COLOR_TYPE_PALETTE
    {
        png_do_gamma(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            png_ptr,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_STRIP_ALPHA
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).transformations as ::core::ffi::c_uint & PNG_COMPOSE
            != 0 as ::core::ffi::c_uint
        && ((*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA
            || (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA)
    {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            0 as ::core::ffi::c_int,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_ENCODE_ALPHA
        != 0 as ::core::ffi::c_uint
        && (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
            != 0 as ::core::ffi::c_int
    {
        png_do_encode_alpha(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            png_ptr,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SCALE_16_TO_8
        != 0 as ::core::ffi::c_uint
    {
        png_do_scale_16_to_8(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_16_TO_8 != 0 as ::core::ffi::c_uint {
        png_do_chop(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_QUANTIZE != 0 as ::core::ffi::c_uint
    {
        png_do_quantize(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            (*png_ptr).palette_lookup as png_const_bytep,
            (*png_ptr).quantize_index as png_const_bytep,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_EXPAND_16 != 0 as ::core::ffi::c_uint
    {
        png_do_expand_16(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_GRAY_TO_RGB
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).mode as ::core::ffi::c_uint & PNG_BACKGROUND_IS_GRAY
            != 0 as ::core::ffi::c_uint
    {
        png_do_gray_to_rgb(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INVERT_MONO
        != 0 as ::core::ffi::c_uint
    {
        png_do_invert(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INVERT_ALPHA
        != 0 as ::core::ffi::c_uint
    {
        png_do_read_invert_alpha(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SHIFT != 0 as ::core::ffi::c_uint {
        png_do_unshift(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            &raw mut (*png_ptr).shift as png_const_color_8p,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_PACK != 0 as ::core::ffi::c_uint {
        png_do_unpack(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
        && (*png_ptr).num_palette_max >= 0 as ::core::ffi::c_int
    {
        png_do_check_palette_indexes(png_ptr, row_info);
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BGR != 0 as ::core::ffi::c_uint {
        png_do_bgr(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_PACKSWAP != 0 as ::core::ffi::c_uint
    {
        png_do_packswap(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_FILLER != 0 as ::core::ffi::c_uint {
        png_do_read_filler(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            (*png_ptr).filler as png_uint_32,
            (*png_ptr).flags,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SWAP_ALPHA
        != 0 as ::core::ffi::c_uint
    {
        png_do_read_swap_alpha(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SWAP_BYTES
        != 0 as ::core::ffi::c_uint
    {
        png_do_swap(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_USER_TRANSFORM
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).read_user_transform_fn.is_some() {
            Some(
                (*png_ptr)
                    .read_user_transform_fn
                    .expect("non-null function pointer"),
            )
            .expect("non-null function pointer")(
                png_ptr as png_structp,
                row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            );
        }
        if (*png_ptr).user_transform_depth as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            (*row_info).bit_depth = (*png_ptr).user_transform_depth;
        }
        if (*png_ptr).user_transform_channels as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            (*row_info).channels = (*png_ptr).user_transform_channels;
        }
        (*row_info).pixel_depth = ((*row_info).bit_depth as ::core::ffi::c_int
            * (*row_info).channels as ::core::ffi::c_int)
            as png_byte;
        (*row_info).rowbytes =
            if (*row_info).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                ((*row_info).width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
            } else {
                ((*row_info).width as size_t)
                    .wrapping_mul((*row_info).pixel_depth as size_t)
                    .wrapping_add(7 as size_t)
                    >> 3 as ::core::ffi::c_int
            };
    }
}
pub const PNG_BACKGROUND_IS_GRAY: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PNG_BGR: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_PACK: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_SHIFT: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_SWAP_BYTES: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_INVERT_MONO: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_QUANTIZE: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_COMPOSE: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_BACKGROUND_EXPAND: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_EXPAND_16: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_16_TO_8: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_EXPAND: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_GAMMA: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_GRAY_TO_RGB: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_FILLER: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_PACKSWAP: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_SWAP_ALPHA: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_STRIP_ALPHA: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const PNG_INVERT_ALPHA: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PNG_USER_TRANSFORM: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const PNG_RGB_TO_GRAY_ERR: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PNG_RGB_TO_GRAY_WARN: ::core::ffi::c_uint = 0x400000 as ::core::ffi::c_uint;
pub const PNG_RGB_TO_GRAY: ::core::ffi::c_uint = 0x600000 as ::core::ffi::c_uint;
pub const PNG_ENCODE_ALPHA: ::core::ffi::c_uint = 0x800000 as ::core::ffi::c_uint;
pub const PNG_ADD_ALPHA: ::core::ffi::c_uint = 0x1000000 as ::core::ffi::c_uint;
pub const PNG_EXPAND_tRNS: ::core::ffi::c_uint = 0x2000000 as ::core::ffi::c_uint;
pub const PNG_SCALE_16_TO_8: ::core::ffi::c_uint = 0x4000000 as ::core::ffi::c_uint;
pub const PNG_FLAG_ROW_INIT: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_FLAG_FILLER_AFTER: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_USE: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_NOWARN: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_CRITICAL_USE: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_CRITICAL_IGNORE: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PNG_FLAG_OPTIMIZE_ALPHA: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_FLAG_DETECT_UNINITIALIZED: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_MASK: ::core::ffi::c_uint =
    PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
pub const PNG_FLAG_CRC_CRITICAL_MASK: ::core::ffi::c_uint =
    PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
pub const PNG_GAMMA_MAC_OLD: ::core::ffi::c_int = 151724 as ::core::ffi::c_int;
pub const PNG_GAMMA_MAC_INVERSE: ::core::ffi::c_int = 65909 as ::core::ffi::c_int;
pub const PNG_GAMMA_sRGB_INVERSE: ::core::ffi::c_int = 45455 as ::core::ffi::c_int;
pub const PNG_LIB_GAMMA_MIN: ::core::ffi::c_int = 1000 as ::core::ffi::c_int;
pub const PNG_LIB_GAMMA_MAX: ::core::ffi::c_int = 10000000 as ::core::ffi::c_int;
