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
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn png_write_flush(png_ptr: png_structrp);
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_save_int_32(buf: png_bytep, i: png_int_32);
    fn deflate(strm: z_streamp, flush: ::core::ffi::c_int) -> ::core::ffi::c_int;
    fn deflateEnd(strm: z_streamp) -> ::core::ffi::c_int;
    fn deflateReset(strm: z_streamp) -> ::core::ffi::c_int;
    fn deflateInit2_(
        strm: z_streamp,
        level: ::core::ffi::c_int,
        method: ::core::ffi::c_int,
        windowBits: ::core::ffi::c_int,
        memLevel: ::core::ffi::c_int,
        strategy: ::core::ffi::c_int,
        version: *const ::core::ffi::c_char,
        stream_size: ::core::ffi::c_int,
    ) -> ::core::ffi::c_int;
    fn png_zstream_error(png_ptr: png_structrp, ret: ::core::ffi::c_int);
    fn png_malloc_base(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_reset_crc(png_ptr: png_structrp);
    fn png_write_data(png_ptr: png_structrp, data: png_const_bytep, length: size_t);
    fn png_calculate_crc(png_ptr: png_structrp, ptr: png_const_bytep, length: size_t);
    fn png_safecat(
        buffer: png_charp,
        bufsize: size_t,
        pos: size_t,
        string: png_const_charp,
    ) -> size_t;
    fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_check_keyword(
        png_ptr: png_structrp,
        key: png_const_charp,
        new_key: png_bytep,
    ) -> png_uint_32;
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
pub type png_const_colorp = *const png_color;
pub type png_const_color_16p = *const png_color_16;
pub type png_const_color_8p = *const png_color_8;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_const_timep = *const png_time;
pub type z_streamp = *mut z_stream;
#[derive(Copy, Clone)]
#[repr(C)]
pub struct compression_state {
    pub input: png_const_bytep,
    pub input_len: png_alloc_size_t,
    pub output_len: png_uint_32,
    pub output: [png_byte; 1024],
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
pub const PNG_TEXT_COMPRESSION_NONE: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
pub const PNG_TEXT_COMPRESSION_zTXt: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_ITXT_COMPRESSION_NONE: ::core::ffi::c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: ::core::ffi::c_int = 2;
pub const PNG_HAVE_IHDR: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_HAVE_PLTE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_AFTER_IDAT: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff as ::core::ffi::c_long as png_uint_32;
pub const PNG_SIZE_MAX: size_t = -(1 as ::core::ffi::c_int) as size_t;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_GRAY: ::core::ffi::c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4;
pub const PNG_COMPRESSION_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_INTRAPIXEL_DIFFERENCING: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const PNG_INTERLACE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_INTERLACE_ADAM7: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_OFFSET_LAST: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_EQUATION_LAST: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_RESOLUTION_LAST: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_sRGB_INTENT_LAST: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_MAX_PALETTE_LENGTH: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_FLAG_MNG_EMPTY_PLTE: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_FLAG_MNG_FILTER_64: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_NO_FILTERS: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_NONE: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_FILTER_SUB: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const PNG_FILTER_UP: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const PNG_FILTER_AVG: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const PNG_FILTER_PAETH: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const PNG_FAST_FILTERS: ::core::ffi::c_int = PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP;
pub const PNG_ALL_FILTERS: ::core::ffi::c_int =
    PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;
pub const PNG_FILTER_VALUE_NONE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_SUB: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_UP: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_AVG: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PNG_FILTER_VALUE_PAETH: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_IO_WRITING: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_IO_SIGNATURE: ::core::ffi::c_int = 0x10 as ::core::ffi::c_int;
pub const PNG_IO_CHUNK_HDR: ::core::ffi::c_int = 0x20 as ::core::ffi::c_int;
pub const PNG_IO_CHUNK_DATA: ::core::ffi::c_int = 0x40 as ::core::ffi::c_int;
pub const PNG_IO_CHUNK_CRC: ::core::ffi::c_int = 0x80 as ::core::ffi::c_int;
pub const ZLIB_IO_MAX: uInt = -(1 as ::core::ffi::c_int) as uInt;
pub const PNG_Z_DEFAULT_NOFILTER_STRATEGY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_Z_DEFAULT_STRATEGY: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const ZLIB_VERSION: [::core::ffi::c_char; 7] =
    unsafe { ::core::mem::transmute::<[u8; 7], [::core::ffi::c_char; 7]>(*b"1.2.11\0") };
pub const Z_NO_FLUSH: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_FINISH: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const Z_OK: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const Z_STREAM_END: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const Z_MEM_ERROR: ::core::ffi::c_int = -(4 as ::core::ffi::c_int);
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
pub unsafe extern "C" fn png_save_uint_32(mut buf: png_bytep, mut i: png_uint_32) {
    *buf.offset(0 as ::core::ffi::c_int as isize) =
        (i as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
            as png_byte;
    *buf.offset(1 as ::core::ffi::c_int as isize) =
        (i as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
            as png_byte;
    *buf.offset(2 as ::core::ffi::c_int as isize) =
        (i as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
            as png_byte;
    *buf.offset(3 as ::core::ffi::c_int as isize) =
        (i as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint) as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_uint_16(mut buf: png_bytep, mut i: ::core::ffi::c_uint) {
    *buf.offset(0 as ::core::ffi::c_int as isize) =
        (i >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint) as png_byte;
    *buf.offset(1 as ::core::ffi::c_int as isize) = (i & 0xff as ::core::ffi::c_uint) as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sig(mut png_ptr: png_structrp) {
    let mut png_signature: [png_byte; 8] = [
        137 as ::core::ffi::c_int as png_byte,
        80 as ::core::ffi::c_int as png_byte,
        78 as ::core::ffi::c_int as png_byte,
        71 as ::core::ffi::c_int as png_byte,
        13 as ::core::ffi::c_int as png_byte,
        10 as ::core::ffi::c_int as png_byte,
        26 as ::core::ffi::c_int as png_byte,
        10 as ::core::ffi::c_int as png_byte,
    ];
    (*png_ptr).io_state = (PNG_IO_WRITING | PNG_IO_SIGNATURE) as png_uint_32;
    png_write_data(
        png_ptr,
        (&raw mut png_signature as *mut png_byte).offset((*png_ptr).sig_bytes as isize)
            as *mut png_byte as png_const_bytep,
        (8 as ::core::ffi::c_int - (*png_ptr).sig_bytes as ::core::ffi::c_int) as size_t,
    );
    if ((*png_ptr).sig_bytes as ::core::ffi::c_int) < 3 as ::core::ffi::c_int {
        (*png_ptr).mode |= PNG_HAVE_PNG_SIGNATURE;
    }
}
unsafe extern "C" fn png_write_chunk_header(
    mut png_ptr: png_structrp,
    mut chunk_name: png_uint_32,
    mut length: png_uint_32,
) {
    let mut buf: [png_byte; 8] = [0; 8];
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).io_state = (PNG_IO_WRITING | PNG_IO_CHUNK_HDR) as png_uint_32;
    png_save_uint_32(&raw mut buf as png_bytep, length);
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        chunk_name,
    );
    png_write_data(
        png_ptr,
        &raw mut buf as *mut png_byte as png_const_bytep,
        8 as size_t,
    );
    (*png_ptr).chunk_name = chunk_name;
    png_reset_crc(png_ptr);
    png_calculate_crc(
        png_ptr,
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize) as png_const_bytep,
        4 as size_t,
    );
    (*png_ptr).io_state = (PNG_IO_WRITING | PNG_IO_CHUNK_DATA) as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_start(
    mut png_ptr: png_structrp,
    mut chunk_string: png_const_bytep,
    mut length: png_uint_32,
) {
    png_write_chunk_header(
        png_ptr,
        (0xffffffff as png_uint_32
            & (0xff as ::core::ffi::c_int
                & *chunk_string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                as png_uint_32)
            << 24 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 16 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 8 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 0 as ::core::ffi::c_int,
        length,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_data(
    mut png_ptr: png_structrp,
    mut data: png_const_bytep,
    mut length: size_t,
) {
    if png_ptr.is_null() {
        return;
    }
    if !data.is_null() && length > 0 as size_t {
        png_write_data(png_ptr, data, length);
        png_calculate_crc(png_ptr, data, length);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk_end(mut png_ptr: png_structrp) {
    let mut buf: [png_byte; 4] = [0; 4];
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).io_state = (PNG_IO_WRITING | PNG_IO_CHUNK_CRC) as png_uint_32;
    png_save_uint_32(&raw mut buf as png_bytep, (*png_ptr).crc);
    png_write_data(
        png_ptr,
        &raw mut buf as *mut png_byte as png_const_bytep,
        4 as size_t,
    );
}
unsafe extern "C" fn png_write_complete_chunk(
    mut png_ptr: png_structrp,
    mut chunk_name: png_uint_32,
    mut data: png_const_bytep,
    mut length: size_t,
) {
    if png_ptr.is_null() {
        return;
    }
    if length > PNG_UINT_31_MAX as size_t {
        png_error(
            png_ptr,
            b"length exceeds PNG maximum\0" as *const u8 as png_const_charp,
        );
    }
    png_write_chunk_header(png_ptr, chunk_name, length as png_uint_32);
    png_write_chunk_data(png_ptr, data, length);
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_chunk(
    mut png_ptr: png_structrp,
    mut chunk_string: png_const_bytep,
    mut data: png_const_bytep,
    mut length: size_t,
) {
    png_write_complete_chunk(
        png_ptr,
        (0xffffffff as png_uint_32
            & (0xff as ::core::ffi::c_int
                & *chunk_string.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                as png_uint_32)
            << 24 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 16 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(2 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 8 as ::core::ffi::c_int
            | (0xffffffff as png_uint_32
                & (0xff as ::core::ffi::c_int
                    & *chunk_string.offset(3 as ::core::ffi::c_int as isize) as ::core::ffi::c_int)
                    as png_uint_32)
                << 0 as ::core::ffi::c_int,
        data,
        length,
    );
}
unsafe extern "C" fn png_image_size(mut png_ptr: png_structrp) -> png_alloc_size_t {
    let mut h: png_uint_32 = (*png_ptr).height;
    if (*png_ptr).rowbytes < 32768 as size_t && h < 32768 as ::core::ffi::c_uint {
        if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
            let mut w: png_uint_32 = (*png_ptr).width;
            let mut pd: ::core::ffi::c_uint = (*png_ptr).pixel_depth as ::core::ffi::c_uint;
            let mut cb_base: png_alloc_size_t = 0;
            let mut pass: ::core::ffi::c_int = 0;
            cb_base = 0 as png_alloc_size_t;
            pass = 0 as ::core::ffi::c_int;
            while pass <= 6 as ::core::ffi::c_int {
                let mut pw: png_uint_32 = w.wrapping_add(
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
                            & 7 as ::core::ffi::c_int)) as png_uint_32,
                ) >> (if pass > 1 as ::core::ffi::c_int {
                    7 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                } else {
                    3 as ::core::ffi::c_int
                });
                if pw > 0 as ::core::ffi::c_uint {
                    cb_base = (cb_base as ::core::ffi::c_ulong).wrapping_add(
                        (if pd >= 8 as ::core::ffi::c_uint {
                            (pw as size_t).wrapping_mul(pd as size_t >> 3 as ::core::ffi::c_int)
                        } else {
                            (pw as size_t)
                                .wrapping_mul(pd as size_t)
                                .wrapping_add(7 as size_t)
                                >> 3 as ::core::ffi::c_int
                        })
                        .wrapping_add(1 as size_t)
                        .wrapping_mul(
                            ((h as ::core::ffi::c_uint).wrapping_add(
                                (((1 as ::core::ffi::c_int)
                                    << (if pass > 2 as ::core::ffi::c_int {
                                        8 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                                    } else {
                                        3 as ::core::ffi::c_int
                                    }))
                                    - 1 as ::core::ffi::c_int
                                    - ((1 as ::core::ffi::c_int & !pass)
                                        << 3 as ::core::ffi::c_int
                                            - (pass >> 1 as ::core::ffi::c_int)
                                        & 7 as ::core::ffi::c_int))
                                    as ::core::ffi::c_uint,
                            ) >> (if pass > 2 as ::core::ffi::c_int {
                                8 as ::core::ffi::c_int - pass >> 1 as ::core::ffi::c_int
                            } else {
                                3 as ::core::ffi::c_int
                            })) as size_t,
                        ) as ::core::ffi::c_ulong,
                    ) as png_alloc_size_t as png_alloc_size_t;
                }
                pass += 1;
            }
            return cb_base;
        } else {
            return ((*png_ptr).rowbytes as png_alloc_size_t)
                .wrapping_add(1 as png_alloc_size_t)
                .wrapping_mul(h as png_alloc_size_t);
        }
    } else {
        return 0xffffffff as ::core::ffi::c_uint as png_alloc_size_t;
    };
}
unsafe extern "C" fn optimize_cmf(mut data: png_bytep, mut data_size: png_alloc_size_t) {
    if data_size <= 16384 as png_alloc_size_t {
        let mut z_cmf: ::core::ffi::c_uint =
            *data.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint;
        if z_cmf & 0xf as ::core::ffi::c_uint == 8 as ::core::ffi::c_uint
            && z_cmf & 0xf0 as ::core::ffi::c_uint <= 0x70 as ::core::ffi::c_uint
        {
            let mut z_cinfo: ::core::ffi::c_uint = 0;
            let mut half_z_window_size: ::core::ffi::c_uint = 0;
            z_cinfo = z_cmf >> 4 as ::core::ffi::c_int;
            half_z_window_size =
                (1 as ::core::ffi::c_uint) << z_cinfo.wrapping_add(7 as ::core::ffi::c_uint);
            if data_size <= half_z_window_size as png_alloc_size_t {
                let mut tmp: ::core::ffi::c_uint = 0;
                loop {
                    half_z_window_size >>= 1 as ::core::ffi::c_int;
                    z_cinfo = z_cinfo.wrapping_sub(1);
                    if !(z_cinfo > 0 as ::core::ffi::c_uint
                        && data_size <= half_z_window_size as png_alloc_size_t)
                    {
                        break;
                    }
                }
                z_cmf = z_cmf & 0xf as ::core::ffi::c_uint | z_cinfo << 4 as ::core::ffi::c_int;
                *data.offset(0 as ::core::ffi::c_int as isize) = z_cmf as png_byte;
                tmp = (*data.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                    & 0xe0 as ::core::ffi::c_int) as ::core::ffi::c_uint;
                tmp = tmp.wrapping_add(
                    (0x1f as ::core::ffi::c_uint).wrapping_sub(
                        (z_cmf << 8 as ::core::ffi::c_int)
                            .wrapping_add(tmp)
                            .wrapping_rem(0x1f as ::core::ffi::c_uint),
                    ),
                );
                *data.offset(1 as ::core::ffi::c_int as isize) = tmp as png_byte;
            }
        }
    }
}
unsafe extern "C" fn png_deflate_claim(
    mut png_ptr: png_structrp,
    mut owner: png_uint_32,
    mut data_size: png_alloc_size_t,
) -> ::core::ffi::c_int {
    if (*png_ptr).zowner != 0 as ::core::ffi::c_uint {
        let mut msg: [::core::ffi::c_char; 64] = [0; 64];
        *(&raw mut msg as *mut ::core::ffi::c_char).offset(0 as ::core::ffi::c_int as isize) =
            (owner as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
                as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char).offset(1 as ::core::ffi::c_int as isize) =
            (owner as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
                as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char).offset(2 as ::core::ffi::c_int as isize) =
            (owner as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint)
                as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char).offset(3 as ::core::ffi::c_int as isize) =
            (owner as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
        msg[4 as ::core::ffi::c_int as usize] = ':' as i32 as ::core::ffi::c_char;
        msg[5 as ::core::ffi::c_int as usize] = ' ' as i32 as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char)
            .offset(6 as ::core::ffi::c_int as isize)
            .offset(0 as ::core::ffi::c_int as isize) =
            ((*png_ptr).zowner as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char)
            .offset(6 as ::core::ffi::c_int as isize)
            .offset(1 as ::core::ffi::c_int as isize) =
            ((*png_ptr).zowner as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char)
            .offset(6 as ::core::ffi::c_int as isize)
            .offset(2 as ::core::ffi::c_int as isize) =
            ((*png_ptr).zowner as ::core::ffi::c_uint >> 8 as ::core::ffi::c_int
                & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
        *(&raw mut msg as *mut ::core::ffi::c_char)
            .offset(6 as ::core::ffi::c_int as isize)
            .offset(3 as ::core::ffi::c_int as isize) = ((*png_ptr).zowner as ::core::ffi::c_uint
            & 0xff as ::core::ffi::c_uint)
            as ::core::ffi::c_char;
        png_safecat(
            &raw mut msg as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
            10 as size_t,
            b" using zstream\0" as *const u8 as png_const_charp,
        );
        png_error(
            png_ptr,
            &raw mut msg as *mut ::core::ffi::c_char as png_const_charp,
        );
    }
    let mut level: ::core::ffi::c_int = (*png_ptr).zlib_level;
    let mut method: ::core::ffi::c_int = (*png_ptr).zlib_method;
    let mut windowBits: ::core::ffi::c_int = (*png_ptr).zlib_window_bits;
    let mut memLevel: ::core::ffi::c_int = (*png_ptr).zlib_mem_level;
    let mut strategy: ::core::ffi::c_int = 0;
    let mut ret: ::core::ffi::c_int = 0;
    if owner == png_IDAT {
        if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZLIB_CUSTOM_STRATEGY
            != 0 as ::core::ffi::c_uint
        {
            strategy = (*png_ptr).zlib_strategy;
        } else if (*png_ptr).do_filter as ::core::ffi::c_int != PNG_FILTER_NONE {
            strategy = PNG_Z_DEFAULT_STRATEGY;
        } else {
            strategy = PNG_Z_DEFAULT_NOFILTER_STRATEGY;
        }
    } else {
        level = (*png_ptr).zlib_text_level;
        method = (*png_ptr).zlib_text_method;
        windowBits = (*png_ptr).zlib_text_window_bits;
        memLevel = (*png_ptr).zlib_text_mem_level;
        strategy = (*png_ptr).zlib_text_strategy;
    }
    if data_size <= 16384 as png_alloc_size_t {
        let mut half_window_size: ::core::ffi::c_uint =
            (1 as ::core::ffi::c_uint) << windowBits - 1 as ::core::ffi::c_int;
        while data_size.wrapping_add(262 as png_alloc_size_t)
            <= half_window_size as png_alloc_size_t
        {
            half_window_size >>= 1 as ::core::ffi::c_int;
            windowBits -= 1;
        }
    }
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_INITIALIZED
        != 0 as ::core::ffi::c_uint
        && ((*png_ptr).zlib_set_level != level
            || (*png_ptr).zlib_set_method != method
            || (*png_ptr).zlib_set_window_bits != windowBits
            || (*png_ptr).zlib_set_mem_level != memLevel
            || (*png_ptr).zlib_set_strategy != strategy)
    {
        if deflateEnd(&raw mut (*png_ptr).zstream) != Z_OK {
            png_warning(
                png_ptr,
                b"deflateEnd failed (ignored)\0" as *const u8 as png_const_charp,
            );
        }
        (*png_ptr).flags &= !PNG_FLAG_ZSTREAM_INITIALIZED;
    }
    (*png_ptr).zstream.next_in = ::core::ptr::null::<Bytef>();
    (*png_ptr).zstream.avail_in = 0 as uInt;
    (*png_ptr).zstream.next_out = ::core::ptr::null_mut::<Bytef>();
    (*png_ptr).zstream.avail_out = 0 as uInt;
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ZSTREAM_INITIALIZED
        != 0 as ::core::ffi::c_uint
    {
        ret = deflateReset(&raw mut (*png_ptr).zstream);
    } else {
        ret = deflateInit2_(
            &raw mut (*png_ptr).zstream,
            level,
            method,
            windowBits,
            memLevel,
            strategy,
            ZLIB_VERSION.as_ptr(),
            ::core::mem::size_of::<z_stream>() as ::core::ffi::c_int,
        );
        if ret == Z_OK {
            (*png_ptr).flags |= PNG_FLAG_ZSTREAM_INITIALIZED;
        }
    }
    if ret == Z_OK {
        (*png_ptr).zowner = owner;
    } else {
        png_zstream_error(png_ptr, ret);
    }
    return ret;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_buffer_list(
    mut png_ptr: png_structrp,
    mut listp: *mut png_compression_bufferp,
) {
    let mut list: png_compression_bufferp = *listp;
    if !list.is_null() {
        *listp = ::core::ptr::null_mut::<png_compression_buffer>();
        loop {
            let mut next: png_compression_bufferp = (*list).next as png_compression_bufferp;
            png_free(png_ptr, list as png_voidp);
            list = next;
            if list.is_null() {
                break;
            }
        }
    }
}
unsafe extern "C" fn png_text_compress_init(
    mut comp: *mut compression_state,
    mut input: png_const_bytep,
    mut input_len: png_alloc_size_t,
) {
    (*comp).input = input;
    (*comp).input_len = input_len;
    (*comp).output_len = 0 as png_uint_32;
}
unsafe extern "C" fn png_text_compress(
    mut png_ptr: png_structrp,
    mut chunk_name: png_uint_32,
    mut comp: *mut compression_state,
    mut prefix_len: png_uint_32,
) -> ::core::ffi::c_int {
    let mut ret: ::core::ffi::c_int = 0;
    ret = png_deflate_claim(png_ptr, chunk_name, (*comp).input_len);
    if ret != Z_OK {
        return ret;
    }
    let mut end: *mut png_compression_bufferp = &raw mut (*png_ptr).zbuffer_list;
    let mut input_len: png_alloc_size_t = (*comp).input_len;
    let mut output_len: png_uint_32 = 0;
    (*png_ptr).zstream.next_in = (*comp).input as *const Bytef;
    (*png_ptr).zstream.avail_in = 0 as uInt;
    (*png_ptr).zstream.next_out = &raw mut (*comp).output as *mut png_byte as *mut Bytef;
    (*png_ptr).zstream.avail_out = ::core::mem::size_of::<[png_byte; 1024]>() as uInt;
    output_len = (*png_ptr).zstream.avail_out as png_uint_32;
    loop {
        let mut avail_in: uInt = ZLIB_IO_MAX;
        if avail_in as png_alloc_size_t > input_len {
            avail_in = input_len as uInt;
        }
        input_len = (input_len as ::core::ffi::c_ulong)
            .wrapping_sub(avail_in as ::core::ffi::c_ulong) as png_alloc_size_t
            as png_alloc_size_t;
        (*png_ptr).zstream.avail_in = avail_in;
        if (*png_ptr).zstream.avail_out == 0 as ::core::ffi::c_uint {
            let mut next: *mut png_compression_buffer =
                ::core::ptr::null_mut::<png_compression_buffer>();
            if output_len.wrapping_add(prefix_len) > PNG_UINT_31_MAX {
                ret = Z_MEM_ERROR;
                break;
            } else {
                next = *end as *mut png_compression_buffer;
                if next.is_null() {
                    next = png_malloc_base(
                        png_ptr,
                        (8 as png_alloc_size_t)
                            .wrapping_add((*png_ptr).zbuffer_size as png_alloc_size_t),
                    ) as *mut png_compression_buffer;
                    if next.is_null() {
                        ret = Z_MEM_ERROR;
                        break;
                    } else {
                        (*next).next = ::core::ptr::null_mut::<png_compression_buffer>();
                        *end = next as png_compression_bufferp;
                    }
                }
                (*png_ptr).zstream.next_out =
                    &raw mut (*next).output as *mut png_byte as *mut Bytef;
                (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
                output_len = (output_len as ::core::ffi::c_uint)
                    .wrapping_add((*png_ptr).zstream.avail_out as ::core::ffi::c_uint)
                    as png_uint_32 as png_uint_32;
                end = &raw mut (*next).next as *mut png_compression_bufferp;
            }
        }
        ret = deflate(
            &raw mut (*png_ptr).zstream,
            if input_len > 0 as png_alloc_size_t {
                Z_NO_FLUSH
            } else {
                Z_FINISH
            },
        );
        input_len = (input_len as ::core::ffi::c_ulong)
            .wrapping_add((*png_ptr).zstream.avail_in as ::core::ffi::c_ulong)
            as png_alloc_size_t as png_alloc_size_t;
        (*png_ptr).zstream.avail_in = 0 as uInt;
        if !(ret == Z_OK) {
            break;
        }
    }
    output_len = (output_len as ::core::ffi::c_uint)
        .wrapping_sub((*png_ptr).zstream.avail_out as ::core::ffi::c_uint)
        as png_uint_32 as png_uint_32;
    (*png_ptr).zstream.avail_out = 0 as uInt;
    (*comp).output_len = output_len;
    if output_len.wrapping_add(prefix_len) >= PNG_UINT_31_MAX {
        (*png_ptr).zstream.msg =
            b"compressed data too long\0" as *const u8 as *const ::core::ffi::c_char;
        ret = Z_MEM_ERROR;
    } else {
        png_zstream_error(png_ptr, ret);
    }
    (*png_ptr).zowner = 0 as png_uint_32;
    if ret == Z_STREAM_END && input_len == 0 as png_alloc_size_t {
        optimize_cmf(&raw mut (*comp).output as png_bytep, (*comp).input_len);
        return Z_OK;
    } else {
        return ret;
    };
}
unsafe extern "C" fn png_write_compressed_data_out(
    mut png_ptr: png_structrp,
    mut comp: *mut compression_state,
) {
    let mut output_len: png_uint_32 = (*comp).output_len;
    let mut output: png_const_bytep = &raw mut (*comp).output as *mut png_byte as png_const_bytep;
    let mut avail: png_uint_32 = ::core::mem::size_of::<[png_byte; 1024]>() as png_uint_32;
    let mut next: *mut png_compression_buffer =
        (*png_ptr).zbuffer_list as *mut png_compression_buffer;
    loop {
        if avail > output_len {
            avail = output_len;
        }
        png_write_chunk_data(png_ptr, output, avail as size_t);
        output_len = (output_len as ::core::ffi::c_uint).wrapping_sub(avail as ::core::ffi::c_uint)
            as png_uint_32 as png_uint_32;
        if output_len == 0 as ::core::ffi::c_uint || next.is_null() {
            break;
        }
        avail = (*png_ptr).zbuffer_size as png_uint_32;
        output = &raw mut (*next).output as *mut png_byte as png_const_bytep;
        next = (*next).next as *mut png_compression_buffer;
    }
    if output_len > 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"error writing ancillary chunked compressed data\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IHDR(
    mut png_ptr: png_structrp,
    mut width: png_uint_32,
    mut height: png_uint_32,
    mut bit_depth: ::core::ffi::c_int,
    mut color_type: ::core::ffi::c_int,
    mut compression_type: ::core::ffi::c_int,
    mut filter_type: ::core::ffi::c_int,
    mut interlace_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 13] = [0; 13];
    let mut is_invalid_depth: ::core::ffi::c_int = 0;
    match color_type {
        PNG_COLOR_TYPE_GRAY => match bit_depth {
            1 | 2 | 4 | 8 | 16 => {
                (*png_ptr).channels = 1 as png_byte;
            }
            _ => {
                png_error(
                    png_ptr,
                    b"Invalid bit depth for grayscale image\0" as *const u8 as png_const_charp,
                );
            }
        },
        PNG_COLOR_TYPE_RGB => {
            is_invalid_depth = (bit_depth != 8 as ::core::ffi::c_int) as ::core::ffi::c_int;
            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            if is_invalid_depth != 0 {
                png_error(
                    png_ptr,
                    b"Invalid bit depth for RGB image\0" as *const u8 as png_const_charp,
                );
            }
            (*png_ptr).channels = 3 as png_byte;
        }
        PNG_COLOR_TYPE_PALETTE => match bit_depth {
            1 | 2 | 4 | 8 => {
                (*png_ptr).channels = 1 as png_byte;
            }
            _ => {
                png_error(
                    png_ptr,
                    b"Invalid bit depth for paletted image\0" as *const u8 as png_const_charp,
                );
            }
        },
        PNG_COLOR_TYPE_GRAY_ALPHA => {
            is_invalid_depth = (bit_depth != 8 as ::core::ffi::c_int) as ::core::ffi::c_int;
            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            if is_invalid_depth != 0 {
                png_error(
                    png_ptr,
                    b"Invalid bit depth for grayscale+alpha image\0" as *const u8
                        as png_const_charp,
                );
            }
            (*png_ptr).channels = 2 as png_byte;
        }
        PNG_COLOR_TYPE_RGB_ALPHA => {
            is_invalid_depth = (bit_depth != 8 as ::core::ffi::c_int) as ::core::ffi::c_int;
            is_invalid_depth = (is_invalid_depth != 0 && bit_depth != 16 as ::core::ffi::c_int)
                as ::core::ffi::c_int;
            if is_invalid_depth != 0 {
                png_error(
                    png_ptr,
                    b"Invalid bit depth for RGBA image\0" as *const u8 as png_const_charp,
                );
            }
            (*png_ptr).channels = 4 as png_byte;
        }
        _ => {
            png_error(
                png_ptr,
                b"Invalid image color type specified\0" as *const u8 as png_const_charp,
            );
        }
    }
    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(
            png_ptr,
            b"Invalid compression type specified\0" as *const u8 as png_const_charp,
        );
        compression_type = PNG_COMPRESSION_TYPE_BASE;
    }
    if !((*png_ptr).mng_features_permitted as ::core::ffi::c_uint
        & PNG_FLAG_MNG_FILTER_64 as ::core::ffi::c_uint
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PNG_SIGNATURE
            == 0 as ::core::ffi::c_uint
        && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
        && filter_type == PNG_INTRAPIXEL_DIFFERENCING)
        && filter_type != PNG_FILTER_TYPE_BASE
    {
        png_warning(
            png_ptr,
            b"Invalid filter type specified\0" as *const u8 as png_const_charp,
        );
        filter_type = PNG_FILTER_TYPE_BASE;
    }
    if interlace_type != PNG_INTERLACE_NONE && interlace_type != PNG_INTERLACE_ADAM7 {
        png_warning(
            png_ptr,
            b"Invalid interlace type specified\0" as *const u8 as png_const_charp,
        );
        interlace_type = PNG_INTERLACE_ADAM7;
    }
    (*png_ptr).bit_depth = bit_depth as png_byte;
    (*png_ptr).color_type = color_type as png_byte;
    (*png_ptr).interlaced = interlace_type as png_byte;
    (*png_ptr).filter_type = filter_type as png_byte;
    (*png_ptr).compression_type = compression_type as png_byte;
    (*png_ptr).width = width;
    (*png_ptr).height = height;
    (*png_ptr).pixel_depth = (bit_depth * (*png_ptr).channels as ::core::ffi::c_int) as png_byte;
    (*png_ptr).rowbytes = if (*png_ptr).pixel_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int
    {
        (width as size_t).wrapping_mul((*png_ptr).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
    } else {
        (width as size_t)
            .wrapping_mul((*png_ptr).pixel_depth as size_t)
            .wrapping_add(7 as size_t)
            >> 3 as ::core::ffi::c_int
    };
    (*png_ptr).usr_width = (*png_ptr).width;
    (*png_ptr).usr_bit_depth = (*png_ptr).bit_depth;
    (*png_ptr).usr_channels = (*png_ptr).channels;
    png_save_uint_32(&raw mut buf as png_bytep, width);
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        height,
    );
    buf[8 as ::core::ffi::c_int as usize] = bit_depth as png_byte;
    buf[9 as ::core::ffi::c_int as usize] = color_type as png_byte;
    buf[10 as ::core::ffi::c_int as usize] = compression_type as png_byte;
    buf[11 as ::core::ffi::c_int as usize] = filter_type as png_byte;
    buf[12 as ::core::ffi::c_int as usize] = interlace_type as png_byte;
    png_write_complete_chunk(
        png_ptr,
        png_IHDR,
        &raw mut buf as *mut png_byte as png_const_bytep,
        13 as size_t,
    );
    if (*png_ptr).do_filter as ::core::ffi::c_int == PNG_NO_FILTERS {
        if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
            || ((*png_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int
        {
            (*png_ptr).do_filter = PNG_FILTER_NONE as png_byte;
        } else {
            (*png_ptr).do_filter = PNG_ALL_FILTERS as png_byte;
        }
    }
    (*png_ptr).mode = PNG_HAVE_IHDR as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_PLTE(
    mut png_ptr: png_structrp,
    mut palette: png_const_colorp,
    mut num_pal: png_uint_32,
) {
    let mut max_palette_length: png_uint_32 = 0;
    let mut i: png_uint_32 = 0;
    let mut pal_ptr: png_const_colorp = ::core::ptr::null::<png_color>();
    let mut buf: [png_byte; 3] = [0; 3];
    max_palette_length = (if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        (1 as ::core::ffi::c_int) << (*png_ptr).bit_depth as ::core::ffi::c_int
    } else {
        PNG_MAX_PALETTE_LENGTH
    }) as png_uint_32;
    if (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
        & PNG_FLAG_MNG_EMPTY_PLTE as ::core::ffi::c_uint
        == 0 as ::core::ffi::c_uint
        && num_pal == 0 as ::core::ffi::c_uint
        || num_pal > max_palette_length
    {
        if (*png_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
            png_error(
                png_ptr,
                b"Invalid number of colors in palette\0" as *const u8 as png_const_charp,
            );
        } else {
            png_warning(
                png_ptr,
                b"Invalid number of colors in palette\0" as *const u8 as png_const_charp,
            );
            return;
        }
    }
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR == 0 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Ignoring request to write a PLTE chunk in grayscale PNG\0" as *const u8
                as png_const_charp,
        );
        return;
    }
    (*png_ptr).num_palette = num_pal as png_uint_16;
    png_write_chunk_header(
        png_ptr,
        png_PLTE,
        (num_pal as ::core::ffi::c_uint).wrapping_mul(3 as ::core::ffi::c_uint),
    );
    i = 0 as png_uint_32;
    pal_ptr = palette;
    while i < num_pal {
        buf[0 as ::core::ffi::c_int as usize] = (*pal_ptr).red;
        buf[1 as ::core::ffi::c_int as usize] = (*pal_ptr).green;
        buf[2 as ::core::ffi::c_int as usize] = (*pal_ptr).blue;
        png_write_chunk_data(
            png_ptr,
            &raw mut buf as *mut png_byte as png_const_bytep,
            3 as size_t,
        );
        i = i.wrapping_add(1);
        pal_ptr = pal_ptr.offset(1);
    }
    png_write_chunk_end(png_ptr);
    (*png_ptr).mode |= PNG_HAVE_PLTE as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_compress_IDAT(
    mut png_ptr: png_structrp,
    mut input: png_const_bytep,
    mut input_len: png_alloc_size_t,
    mut flush: ::core::ffi::c_int,
) {
    if (*png_ptr).zowner != png_IDAT {
        if (*png_ptr).zbuffer_list.is_null() {
            (*png_ptr).zbuffer_list = png_malloc(
                png_ptr,
                (8 as png_alloc_size_t).wrapping_add((*png_ptr).zbuffer_size as png_alloc_size_t),
            ) as png_compression_bufferp;
            (*(*png_ptr).zbuffer_list).next = ::core::ptr::null_mut::<png_compression_buffer>();
        } else {
            png_free_buffer_list(png_ptr, &raw mut (*(*png_ptr).zbuffer_list).next);
        }
        if png_deflate_claim(png_ptr, png_IDAT, png_image_size(png_ptr)) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg as png_const_charp);
        }
        (*png_ptr).zstream.next_out =
            &raw mut (*(*png_ptr).zbuffer_list).output as *mut png_byte as *mut Bytef;
        (*png_ptr).zstream.avail_out = (*png_ptr).zbuffer_size;
    }
    (*png_ptr).zstream.next_in = input as *const Bytef;
    (*png_ptr).zstream.avail_in = 0 as uInt;
    loop {
        let mut ret: ::core::ffi::c_int = 0;
        let mut avail: uInt = ZLIB_IO_MAX;
        if avail as png_alloc_size_t > input_len {
            avail = input_len as uInt;
        }
        (*png_ptr).zstream.avail_in = avail;
        input_len = (input_len as ::core::ffi::c_ulong).wrapping_sub(avail as ::core::ffi::c_ulong)
            as png_alloc_size_t as png_alloc_size_t;
        ret = deflate(
            &raw mut (*png_ptr).zstream,
            if input_len > 0 as png_alloc_size_t {
                Z_NO_FLUSH
            } else {
                flush
            },
        );
        input_len = (input_len as ::core::ffi::c_ulong)
            .wrapping_add((*png_ptr).zstream.avail_in as ::core::ffi::c_ulong)
            as png_alloc_size_t as png_alloc_size_t;
        (*png_ptr).zstream.avail_in = 0 as uInt;
        if (*png_ptr).zstream.avail_out == 0 as ::core::ffi::c_uint {
            let mut data: png_bytep = &raw mut (*(*png_ptr).zbuffer_list).output as png_bytep;
            let mut size: uInt = (*png_ptr).zbuffer_size;
            if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT == 0 as ::core::ffi::c_uint
                && (*png_ptr).compression_type as ::core::ffi::c_int == PNG_COMPRESSION_TYPE_BASE
            {
                optimize_cmf(data, png_image_size(png_ptr));
            }
            if size > 0 as ::core::ffi::c_uint {
                png_write_complete_chunk(
                    png_ptr,
                    png_IDAT,
                    data as png_const_bytep,
                    size as size_t,
                );
            }
            (*png_ptr).mode |= PNG_HAVE_IDAT;
            (*png_ptr).zstream.next_out = data as *mut Bytef;
            (*png_ptr).zstream.avail_out = size;
            if ret == Z_OK && flush != Z_NO_FLUSH {
                continue;
            }
        }
        if ret == Z_OK {
            if input_len == 0 as png_alloc_size_t {
                if flush == Z_FINISH {
                    png_error(
                        png_ptr,
                        b"Z_OK on Z_FINISH with output space\0" as *const u8 as png_const_charp,
                    );
                }
                return;
            }
        } else if ret == Z_STREAM_END && flush == Z_FINISH {
            let mut data_0: png_bytep = &raw mut (*(*png_ptr).zbuffer_list).output as png_bytep;
            let mut size_0: uInt = (*png_ptr)
                .zbuffer_size
                .wrapping_sub((*png_ptr).zstream.avail_out);
            if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_IDAT == 0 as ::core::ffi::c_uint
                && (*png_ptr).compression_type as ::core::ffi::c_int == PNG_COMPRESSION_TYPE_BASE
            {
                optimize_cmf(data_0, png_image_size(png_ptr));
            }
            if size_0 > 0 as ::core::ffi::c_uint {
                png_write_complete_chunk(
                    png_ptr,
                    png_IDAT,
                    data_0 as png_const_bytep,
                    size_0 as size_t,
                );
            }
            (*png_ptr).zstream.avail_out = 0 as uInt;
            (*png_ptr).zstream.next_out = ::core::ptr::null_mut::<Bytef>();
            (*png_ptr).mode |= PNG_HAVE_IDAT | PNG_AFTER_IDAT as ::core::ffi::c_uint;
            (*png_ptr).zowner = 0 as png_uint_32;
            return;
        } else {
            png_zstream_error(png_ptr, ret);
            png_error(png_ptr, (*png_ptr).zstream.msg as png_const_charp);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_IEND(mut png_ptr: png_structrp) {
    png_write_complete_chunk(
        png_ptr,
        png_IEND,
        ::core::ptr::null::<png_byte>(),
        0 as size_t,
    );
    (*png_ptr).mode |= PNG_HAVE_IEND;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_gAMA_fixed(
    mut png_ptr: png_structrp,
    mut file_gamma: png_fixed_point,
) {
    let mut buf: [png_byte; 4] = [0; 4];
    png_save_uint_32(&raw mut buf as png_bytep, file_gamma as png_uint_32);
    png_write_complete_chunk(
        png_ptr,
        png_gAMA,
        &raw mut buf as *mut png_byte as png_const_bytep,
        4 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sRGB(
    mut png_ptr: png_structrp,
    mut srgb_intent: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 1] = [0; 1];
    if srgb_intent >= PNG_sRGB_INTENT_LAST {
        png_warning(
            png_ptr,
            b"Invalid sRGB rendering intent specified\0" as *const u8 as png_const_charp,
        );
    }
    buf[0 as ::core::ffi::c_int as usize] = srgb_intent as png_byte;
    png_write_complete_chunk(
        png_ptr,
        png_sRGB,
        &raw mut buf as *mut png_byte as png_const_bytep,
        1 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_iCCP(
    mut png_ptr: png_structrp,
    mut name: png_const_charp,
    mut profile: png_const_bytep,
    mut profile_len: png_uint_32,
) {
    let mut name_len: png_uint_32 = 0;
    let mut new_name: [png_byte; 81] = [0; 81];
    let mut comp: compression_state = compression_state {
        input: ::core::ptr::null::<png_byte>(),
        input_len: 0,
        output_len: 0,
        output: [0; 1024],
    };
    let mut temp: png_uint_32 = 0;
    if profile.is_null() {
        png_error(
            png_ptr,
            b"No profile for iCCP chunk\0" as *const u8 as png_const_charp,
        );
    }
    if profile_len < 132 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"ICC profile too short\0" as *const u8 as png_const_charp,
        );
    }
    if ((*profile as png_uint_32) << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile.offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile.offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(*profile.offset(3 as ::core::ffi::c_int as isize) as png_uint_32)
        != profile_len
    {
        png_error(
            png_ptr,
            b"Incorrect data in iCCP\0" as *const u8 as png_const_charp,
        );
    }
    temp = *profile.offset(8 as ::core::ffi::c_int as isize) as png_uint_32;
    if temp > 3 as ::core::ffi::c_uint
        && profile_len as ::core::ffi::c_uint & 0x3 as ::core::ffi::c_uint != 0
    {
        png_error(
            png_ptr,
            b"ICC profile length invalid (not a multiple of 4)\0" as *const u8 as png_const_charp,
        );
    }
    let mut embedded_profile_len: png_uint_32 = ((*profile as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile.offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile.offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(*profile.offset(3 as ::core::ffi::c_int as isize) as png_uint_32);
    if profile_len != embedded_profile_len {
        png_error(
            png_ptr,
            b"Profile length does not match profile\0" as *const u8 as png_const_charp,
        );
    }
    name_len = png_check_keyword(png_ptr, name, &raw mut new_name as png_bytep);
    if name_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"iCCP: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    name_len = name_len.wrapping_add(1);
    new_name[name_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    name_len = name_len.wrapping_add(1);
    png_text_compress_init(&raw mut comp, profile, profile_len as png_alloc_size_t);
    if png_text_compress(png_ptr, png_iCCP, &raw mut comp, name_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg as png_const_charp);
    }
    png_write_chunk_header(png_ptr, png_iCCP, name_len.wrapping_add(comp.output_len));
    png_write_chunk_data(
        png_ptr,
        &raw mut new_name as *mut png_byte as png_const_bytep,
        name_len as size_t,
    );
    png_write_compressed_data_out(png_ptr, &raw mut comp);
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sPLT(
    mut png_ptr: png_structrp,
    mut spalette: png_const_sPLT_tp,
) {
    let mut name_len: png_uint_32 = 0;
    let mut new_name: [png_byte; 80] = [0; 80];
    let mut entrybuf: [png_byte; 10] = [0; 10];
    let mut entry_size: size_t =
        (if (*spalette).depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            6 as ::core::ffi::c_int
        } else {
            10 as ::core::ffi::c_int
        }) as size_t;
    let mut palette_size: size_t = entry_size.wrapping_mul((*spalette).nentries as size_t);
    let mut ep: png_sPLT_entryp = ::core::ptr::null_mut::<png_sPLT_entry>();
    name_len = png_check_keyword(
        png_ptr,
        (*spalette).name as png_const_charp,
        &raw mut new_name as png_bytep,
    );
    if name_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"sPLT: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    png_write_chunk_header(
        png_ptr,
        png_sPLT,
        ((name_len as ::core::ffi::c_uint).wrapping_add(2 as ::core::ffi::c_uint) as size_t)
            .wrapping_add(palette_size) as png_uint_32,
    );
    png_write_chunk_data(
        png_ptr,
        &raw mut new_name as *mut png_byte as png_const_bytep,
        (name_len as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint) as size_t,
    );
    png_write_chunk_data(png_ptr, &raw const (*spalette).depth, 1 as size_t);
    ep = (*spalette).entries;
    while ep < (*spalette).entries.offset((*spalette).nentries as isize) {
        if (*spalette).depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            entrybuf[0 as ::core::ffi::c_int as usize] = (*ep).red as png_byte;
            entrybuf[1 as ::core::ffi::c_int as usize] = (*ep).green as png_byte;
            entrybuf[2 as ::core::ffi::c_int as usize] = (*ep).blue as png_byte;
            entrybuf[3 as ::core::ffi::c_int as usize] = (*ep).alpha as png_byte;
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
                (*ep).frequency as ::core::ffi::c_uint,
            );
        } else {
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(0 as ::core::ffi::c_int as isize),
                (*ep).red as ::core::ffi::c_uint,
            );
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(2 as ::core::ffi::c_int as isize),
                (*ep).green as ::core::ffi::c_uint,
            );
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
                (*ep).blue as ::core::ffi::c_uint,
            );
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(6 as ::core::ffi::c_int as isize),
                (*ep).alpha as ::core::ffi::c_uint,
            );
            png_save_uint_16(
                (&raw mut entrybuf as *mut png_byte).offset(8 as ::core::ffi::c_int as isize),
                (*ep).frequency as ::core::ffi::c_uint,
            );
        }
        png_write_chunk_data(
            png_ptr,
            &raw mut entrybuf as *mut png_byte as png_const_bytep,
            entry_size,
        );
        ep = ep.offset(1);
    }
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sBIT(
    mut png_ptr: png_structrp,
    mut sbit: png_const_color_8p,
    mut color_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 4] = [0; 4];
    let mut size: size_t = 0;
    if color_type & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int {
        let mut maxbits: png_byte = 0;
        maxbits = (if color_type == PNG_COLOR_TYPE_PALETTE {
            8 as ::core::ffi::c_int
        } else {
            (*png_ptr).usr_bit_depth as ::core::ffi::c_int
        }) as png_byte;
        if (*sbit).red as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*sbit).red as ::core::ffi::c_int > maxbits as ::core::ffi::c_int
            || (*sbit).green as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*sbit).green as ::core::ffi::c_int > maxbits as ::core::ffi::c_int
            || (*sbit).blue as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*sbit).blue as ::core::ffi::c_int > maxbits as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0" as *const u8 as png_const_charp,
            );
            return;
        }
        buf[0 as ::core::ffi::c_int as usize] = (*sbit).red;
        buf[1 as ::core::ffi::c_int as usize] = (*sbit).green;
        buf[2 as ::core::ffi::c_int as usize] = (*sbit).blue;
        size = 3 as size_t;
    } else {
        if (*sbit).gray as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*sbit).gray as ::core::ffi::c_int > (*png_ptr).usr_bit_depth as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0" as *const u8 as png_const_charp,
            );
            return;
        }
        buf[0 as ::core::ffi::c_int as usize] = (*sbit).gray;
        size = 1 as size_t;
    }
    if color_type & PNG_COLOR_MASK_ALPHA != 0 as ::core::ffi::c_int {
        if (*sbit).alpha as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*sbit).alpha as ::core::ffi::c_int > (*png_ptr).usr_bit_depth as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Invalid sBIT depth specified\0" as *const u8 as png_const_charp,
            );
            return;
        }
        let fresh0 = size;
        size = size.wrapping_add(1);
        buf[fresh0 as usize] = (*sbit).alpha;
    }
    png_write_complete_chunk(
        png_ptr,
        png_sBIT,
        &raw mut buf as *mut png_byte as png_const_bytep,
        size,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cHRM_fixed(mut png_ptr: png_structrp, mut xy: *const png_xy) {
    let mut buf: [png_byte; 32] = [0; 32];
    png_save_int_32(&raw mut buf as png_bytep, (*xy).whitex as png_int_32);
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        (*xy).whitey as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(8 as ::core::ffi::c_int as isize),
        (*xy).redx as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(12 as ::core::ffi::c_int as isize),
        (*xy).redy as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(16 as ::core::ffi::c_int as isize),
        (*xy).greenx as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(20 as ::core::ffi::c_int as isize),
        (*xy).greeny as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(24 as ::core::ffi::c_int as isize),
        (*xy).bluex as png_int_32,
    );
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(28 as ::core::ffi::c_int as isize),
        (*xy).bluey as png_int_32,
    );
    png_write_complete_chunk(
        png_ptr,
        png_cHRM,
        &raw mut buf as *mut png_byte as png_const_bytep,
        32 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tRNS(
    mut png_ptr: png_structrp,
    mut trans_alpha: png_const_bytep,
    mut tran: png_const_color_16p,
    mut num_trans: ::core::ffi::c_int,
    mut color_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 6] = [0; 6];
    if color_type == PNG_COLOR_TYPE_PALETTE {
        if num_trans <= 0 as ::core::ffi::c_int
            || num_trans > (*png_ptr).num_palette as ::core::ffi::c_int
        {
            png_app_warning(
                png_ptr,
                b"Invalid number of transparent colors specified\0" as *const u8 as png_const_charp,
            );
            return;
        }
        png_write_complete_chunk(png_ptr, png_tRNS, trans_alpha, num_trans as size_t);
    } else if color_type == PNG_COLOR_TYPE_GRAY {
        if (*tran).gray as ::core::ffi::c_int
            >= (1 as ::core::ffi::c_int) << (*png_ptr).bit_depth as ::core::ffi::c_int
        {
            png_app_warning(
                png_ptr,
                b"Ignoring attempt to write tRNS chunk out-of-range for bit_depth\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        png_save_uint_16(
            &raw mut buf as png_bytep,
            (*tran).gray as ::core::ffi::c_uint,
        );
        png_write_complete_chunk(
            png_ptr,
            png_tRNS,
            &raw mut buf as *mut png_byte as png_const_bytep,
            2 as size_t,
        );
    } else if color_type == PNG_COLOR_TYPE_RGB {
        png_save_uint_16(
            &raw mut buf as png_bytep,
            (*tran).red as ::core::ffi::c_uint,
        );
        png_save_uint_16(
            (&raw mut buf as *mut png_byte).offset(2 as ::core::ffi::c_int as isize),
            (*tran).green as ::core::ffi::c_uint,
        );
        png_save_uint_16(
            (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
            (*tran).blue as ::core::ffi::c_uint,
        );
        if (*png_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
            && buf[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                | buf[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                | buf[4 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            png_app_warning(
                png_ptr,
                b"Ignoring attempt to write 16-bit tRNS chunk when bit_depth is 8\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        png_write_complete_chunk(
            png_ptr,
            png_tRNS,
            &raw mut buf as *mut png_byte as png_const_bytep,
            6 as size_t,
        );
    } else {
        png_app_warning(
            png_ptr,
            b"Can't write tRNS with an alpha channel\0" as *const u8 as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_bKGD(
    mut png_ptr: png_structrp,
    mut back: png_const_color_16p,
    mut color_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 6] = [0; 6];
    if color_type == PNG_COLOR_TYPE_PALETTE {
        if ((*png_ptr).num_palette as ::core::ffi::c_int != 0 as ::core::ffi::c_int
            || (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
                & PNG_FLAG_MNG_EMPTY_PLTE as ::core::ffi::c_uint
                == 0 as ::core::ffi::c_uint)
            && (*back).index as ::core::ffi::c_int >= (*png_ptr).num_palette as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Invalid background palette index\0" as *const u8 as png_const_charp,
            );
            return;
        }
        buf[0 as ::core::ffi::c_int as usize] = (*back).index;
        png_write_complete_chunk(
            png_ptr,
            png_bKGD,
            &raw mut buf as *mut png_byte as png_const_bytep,
            1 as size_t,
        );
    } else if color_type & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int {
        png_save_uint_16(
            &raw mut buf as png_bytep,
            (*back).red as ::core::ffi::c_uint,
        );
        png_save_uint_16(
            (&raw mut buf as *mut png_byte).offset(2 as ::core::ffi::c_int as isize),
            (*back).green as ::core::ffi::c_uint,
        );
        png_save_uint_16(
            (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
            (*back).blue as ::core::ffi::c_uint,
        );
        if (*png_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
            && buf[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                | buf[2 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                | buf[4 as ::core::ffi::c_int as usize] as ::core::ffi::c_int
                != 0 as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Ignoring attempt to write 16-bit bKGD chunk when bit_depth is 8\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        png_write_complete_chunk(
            png_ptr,
            png_bKGD,
            &raw mut buf as *mut png_byte as png_const_bytep,
            6 as size_t,
        );
    } else {
        if (*back).gray as ::core::ffi::c_int
            >= (1 as ::core::ffi::c_int) << (*png_ptr).bit_depth as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Ignoring attempt to write bKGD chunk out-of-range for bit_depth\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        png_save_uint_16(
            &raw mut buf as png_bytep,
            (*back).gray as ::core::ffi::c_uint,
        );
        png_write_complete_chunk(
            png_ptr,
            png_bKGD,
            &raw mut buf as *mut png_byte as png_const_bytep,
            2 as size_t,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cICP(
    mut png_ptr: png_structrp,
    mut colour_primaries: png_byte,
    mut transfer_function: png_byte,
    mut matrix_coefficients: png_byte,
    mut video_full_range_flag: png_byte,
) {
    let mut buf: [png_byte; 4] = [0; 4];
    png_write_chunk_header(png_ptr, png_cICP, 4 as png_uint_32);
    buf[0 as ::core::ffi::c_int as usize] = colour_primaries;
    buf[1 as ::core::ffi::c_int as usize] = transfer_function;
    buf[2 as ::core::ffi::c_int as usize] = matrix_coefficients;
    buf[3 as ::core::ffi::c_int as usize] = video_full_range_flag;
    png_write_chunk_data(
        png_ptr,
        &raw mut buf as *mut png_byte as png_const_bytep,
        4 as size_t,
    );
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_cLLI_fixed(
    mut png_ptr: png_structrp,
    mut maxCLL: png_uint_32,
    mut maxFALL: png_uint_32,
) {
    let mut buf: [png_byte; 8] = [0; 8];
    png_save_uint_32(&raw mut buf as png_bytep, maxCLL);
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        maxFALL,
    );
    png_write_complete_chunk(
        png_ptr,
        png_cLLI,
        &raw mut buf as *mut png_byte as png_const_bytep,
        8 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_mDCV_fixed(
    mut png_ptr: png_structrp,
    mut red_x: png_uint_16,
    mut red_y: png_uint_16,
    mut green_x: png_uint_16,
    mut green_y: png_uint_16,
    mut blue_x: png_uint_16,
    mut blue_y: png_uint_16,
    mut white_x: png_uint_16,
    mut white_y: png_uint_16,
    mut maxDL: png_uint_32,
    mut minDL: png_uint_32,
) {
    let mut buf: [png_byte; 24] = [0; 24];
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(0 as ::core::ffi::c_int as isize),
        red_x as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(2 as ::core::ffi::c_int as isize),
        red_y as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        green_x as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(6 as ::core::ffi::c_int as isize),
        green_y as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(8 as ::core::ffi::c_int as isize),
        blue_x as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(10 as ::core::ffi::c_int as isize),
        blue_y as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(12 as ::core::ffi::c_int as isize),
        white_x as ::core::ffi::c_uint,
    );
    png_save_uint_16(
        (&raw mut buf as *mut png_byte).offset(14 as ::core::ffi::c_int as isize),
        white_y as ::core::ffi::c_uint,
    );
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(16 as ::core::ffi::c_int as isize),
        maxDL,
    );
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(20 as ::core::ffi::c_int as isize),
        minDL,
    );
    png_write_complete_chunk(
        png_ptr,
        png_mDCV,
        &raw mut buf as *mut png_byte as png_const_bytep,
        24 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_eXIf(
    mut png_ptr: png_structrp,
    mut exif: png_bytep,
    mut num_exif: ::core::ffi::c_int,
) {
    let mut i: ::core::ffi::c_int = 0;
    let mut buf: [png_byte; 1] = [0; 1];
    png_write_chunk_header(png_ptr, png_eXIf, num_exif as png_uint_32);
    i = 0 as ::core::ffi::c_int;
    while i < num_exif {
        buf[0 as ::core::ffi::c_int as usize] = *exif.offset(i as isize);
        png_write_chunk_data(
            png_ptr,
            &raw mut buf as *mut png_byte as png_const_bytep,
            1 as size_t,
        );
        i += 1;
    }
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_hIST(
    mut png_ptr: png_structrp,
    mut hist: png_const_uint_16p,
    mut num_hist: ::core::ffi::c_int,
) {
    let mut i: ::core::ffi::c_int = 0;
    let mut buf: [png_byte; 3] = [0; 3];
    if num_hist > (*png_ptr).num_palette as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Invalid number of histogram entries specified\0" as *const u8 as png_const_charp,
        );
        return;
    }
    png_write_chunk_header(
        png_ptr,
        png_hIST,
        (num_hist * 2 as ::core::ffi::c_int) as png_uint_32,
    );
    i = 0 as ::core::ffi::c_int;
    while i < num_hist {
        png_save_uint_16(
            &raw mut buf as png_bytep,
            *hist.offset(i as isize) as ::core::ffi::c_uint,
        );
        png_write_chunk_data(
            png_ptr,
            &raw mut buf as *mut png_byte as png_const_bytep,
            2 as size_t,
        );
        i += 1;
    }
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tEXt(
    mut png_ptr: png_structrp,
    mut key: png_const_charp,
    mut text: png_const_charp,
    mut text_len: size_t,
) {
    let mut key_len: png_uint_32 = 0;
    let mut new_key: [png_byte; 80] = [0; 80];
    key_len = png_check_keyword(png_ptr, key, &raw mut new_key as png_bytep);
    if key_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"tEXt: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    if text.is_null() || *text as ::core::ffi::c_int == '\0' as i32 {
        text_len = 0 as size_t;
    } else {
        text_len = strlen(text as *const ::core::ffi::c_char);
    }
    if text_len
        > PNG_UINT_31_MAX
            .wrapping_sub((key_len as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint))
            as size_t
    {
        png_error(
            png_ptr,
            b"tEXt: text too long\0" as *const u8 as png_const_charp,
        );
    }
    png_write_chunk_header(
        png_ptr,
        png_tEXt,
        (key_len as size_t)
            .wrapping_add(text_len)
            .wrapping_add(1 as size_t) as png_uint_32,
    );
    png_write_chunk_data(
        png_ptr,
        &raw mut new_key as *mut png_byte as png_const_bytep,
        (key_len as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint) as size_t,
    );
    if text_len != 0 as size_t {
        png_write_chunk_data(png_ptr, text as png_const_bytep, text_len);
    }
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_zTXt(
    mut png_ptr: png_structrp,
    mut key: png_const_charp,
    mut text: png_const_charp,
    mut compression: ::core::ffi::c_int,
) {
    let mut key_len: png_uint_32 = 0;
    let mut new_key: [png_byte; 81] = [0; 81];
    let mut comp: compression_state = compression_state {
        input: ::core::ptr::null::<png_byte>(),
        input_len: 0,
        output_len: 0,
        output: [0; 1024],
    };
    if compression == PNG_TEXT_COMPRESSION_NONE {
        png_write_tEXt(png_ptr, key, text, 0 as size_t);
        return;
    }
    if compression != PNG_TEXT_COMPRESSION_zTXt {
        png_error(
            png_ptr,
            b"zTXt: invalid compression type\0" as *const u8 as png_const_charp,
        );
    }
    key_len = png_check_keyword(png_ptr, key, &raw mut new_key as png_bytep);
    if key_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"zTXt: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    key_len = key_len.wrapping_add(1);
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len = key_len.wrapping_add(1);
    png_text_compress_init(
        &raw mut comp,
        text as png_const_bytep,
        if text.is_null() {
            0 as png_alloc_size_t
        } else {
            strlen(text as *const ::core::ffi::c_char) as png_alloc_size_t
        },
    );
    if png_text_compress(png_ptr, png_zTXt, &raw mut comp, key_len) != Z_OK {
        png_error(png_ptr, (*png_ptr).zstream.msg as png_const_charp);
    }
    png_write_chunk_header(png_ptr, png_zTXt, key_len.wrapping_add(comp.output_len));
    png_write_chunk_data(
        png_ptr,
        &raw mut new_key as *mut png_byte as png_const_bytep,
        key_len as size_t,
    );
    png_write_compressed_data_out(png_ptr, &raw mut comp);
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_iTXt(
    mut png_ptr: png_structrp,
    mut compression: ::core::ffi::c_int,
    mut key: png_const_charp,
    mut lang: png_const_charp,
    mut lang_key: png_const_charp,
    mut text: png_const_charp,
) {
    let mut key_len: png_uint_32 = 0;
    let mut prefix_len: png_uint_32 = 0;
    let mut lang_len: size_t = 0;
    let mut lang_key_len: size_t = 0;
    let mut new_key: [png_byte; 82] = [0; 82];
    let mut comp: compression_state = compression_state {
        input: ::core::ptr::null::<png_byte>(),
        input_len: 0,
        output_len: 0,
        output: [0; 1024],
    };
    key_len = png_check_keyword(png_ptr, key, &raw mut new_key as png_bytep);
    if key_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"iTXt: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    match compression {
        PNG_ITXT_COMPRESSION_NONE | PNG_TEXT_COMPRESSION_NONE => {
            key_len = key_len.wrapping_add(1);
            new_key[key_len as usize] = 0 as png_byte;
            compression = new_key[key_len as usize] as ::core::ffi::c_int;
        }
        PNG_TEXT_COMPRESSION_zTXt | PNG_ITXT_COMPRESSION_zTXt => {
            key_len = key_len.wrapping_add(1);
            new_key[key_len as usize] = 1 as png_byte;
            compression = new_key[key_len as usize] as ::core::ffi::c_int;
        }
        _ => {
            png_error(
                png_ptr,
                b"iTXt: invalid compression\0" as *const u8 as png_const_charp,
            );
        }
    }
    key_len = key_len.wrapping_add(1);
    new_key[key_len as usize] = PNG_COMPRESSION_TYPE_BASE as png_byte;
    key_len = key_len.wrapping_add(1);
    if lang.is_null() {
        lang = b"\0" as *const u8 as *const ::core::ffi::c_char as png_const_charp;
    }
    lang_len = strlen(lang as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
    if lang_key.is_null() {
        lang_key = b"\0" as *const u8 as *const ::core::ffi::c_char as png_const_charp;
    }
    lang_key_len = strlen(lang_key as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
    if text.is_null() {
        text = b"\0" as *const u8 as *const ::core::ffi::c_char as png_const_charp;
    }
    prefix_len = key_len;
    if lang_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as size_t {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as size_t).wrapping_add(lang_len) as png_uint_32;
    }
    if lang_key_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as size_t {
        prefix_len = PNG_UINT_31_MAX;
    } else {
        prefix_len = (prefix_len as size_t).wrapping_add(lang_key_len) as png_uint_32;
    }
    png_text_compress_init(
        &raw mut comp,
        text as png_const_bytep,
        strlen(text as *const ::core::ffi::c_char) as png_alloc_size_t,
    );
    if compression != 0 as ::core::ffi::c_int {
        if png_text_compress(png_ptr, png_iTXt, &raw mut comp, prefix_len) != Z_OK {
            png_error(png_ptr, (*png_ptr).zstream.msg as png_const_charp);
        }
    } else {
        if comp.input_len > PNG_UINT_31_MAX.wrapping_sub(prefix_len) as png_alloc_size_t {
            png_error(
                png_ptr,
                b"iTXt: uncompressed text too long\0" as *const u8 as png_const_charp,
            );
        }
        comp.output_len = comp.input_len as png_uint_32;
    }
    png_write_chunk_header(png_ptr, png_iTXt, comp.output_len.wrapping_add(prefix_len));
    png_write_chunk_data(
        png_ptr,
        &raw mut new_key as *mut png_byte as png_const_bytep,
        key_len as size_t,
    );
    png_write_chunk_data(png_ptr, lang as png_const_bytep, lang_len);
    png_write_chunk_data(png_ptr, lang_key as png_const_bytep, lang_key_len);
    if compression != 0 as ::core::ffi::c_int {
        png_write_compressed_data_out(png_ptr, &raw mut comp);
    } else {
        png_write_chunk_data(png_ptr, text as png_const_bytep, comp.output_len as size_t);
    }
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_oFFs(
    mut png_ptr: png_structrp,
    mut x_offset: png_int_32,
    mut y_offset: png_int_32,
    mut unit_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];
    if unit_type >= PNG_OFFSET_LAST {
        png_warning(
            png_ptr,
            b"Unrecognized unit type for oFFs chunk\0" as *const u8 as png_const_charp,
        );
    }
    png_save_int_32(&raw mut buf as png_bytep, x_offset);
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        y_offset,
    );
    buf[8 as ::core::ffi::c_int as usize] = unit_type as png_byte;
    png_write_complete_chunk(
        png_ptr,
        png_oFFs,
        &raw mut buf as *mut png_byte as png_const_bytep,
        9 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_pCAL(
    mut png_ptr: png_structrp,
    mut purpose: png_charp,
    mut X0: png_int_32,
    mut X1: png_int_32,
    mut type_0: ::core::ffi::c_int,
    mut nparams: ::core::ffi::c_int,
    mut units: png_const_charp,
    mut params: png_charpp,
) {
    let mut purpose_len: png_uint_32 = 0;
    let mut units_len: size_t = 0;
    let mut total_len: size_t = 0;
    let mut params_len: *mut size_t = ::core::ptr::null_mut::<size_t>();
    let mut buf: [png_byte; 10] = [0; 10];
    let mut new_purpose: [png_byte; 80] = [0; 80];
    let mut i: ::core::ffi::c_int = 0;
    if type_0 >= PNG_EQUATION_LAST {
        png_error(
            png_ptr,
            b"Unrecognized equation type for pCAL chunk\0" as *const u8 as png_const_charp,
        );
    }
    purpose_len = png_check_keyword(
        png_ptr,
        purpose as png_const_charp,
        &raw mut new_purpose as png_bytep,
    );
    if purpose_len == 0 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"pCAL: invalid keyword\0" as *const u8 as png_const_charp,
        );
    }
    purpose_len = purpose_len.wrapping_add(1);
    units_len = strlen(units as *const ::core::ffi::c_char).wrapping_add(
        (if nparams == 0 as ::core::ffi::c_int {
            0 as ::core::ffi::c_int
        } else {
            1 as ::core::ffi::c_int
        }) as size_t,
    );
    total_len = (purpose_len as size_t)
        .wrapping_add(units_len)
        .wrapping_add(10 as size_t);
    params_len = png_malloc(
        png_ptr,
        (nparams as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<size_t>() as png_alloc_size_t),
    ) as *mut size_t;
    i = 0 as ::core::ffi::c_int;
    while i < nparams {
        *params_len.offset(i as isize) = strlen(*params.offset(i as isize)).wrapping_add(
            (if i == nparams - 1 as ::core::ffi::c_int {
                0 as ::core::ffi::c_int
            } else {
                1 as ::core::ffi::c_int
            }) as size_t,
        );
        total_len = (total_len as ::core::ffi::c_ulong)
            .wrapping_add(*params_len.offset(i as isize) as ::core::ffi::c_ulong)
            as size_t as size_t;
        i += 1;
    }
    png_write_chunk_header(png_ptr, png_pCAL, total_len as png_uint_32);
    png_write_chunk_data(
        png_ptr,
        &raw mut new_purpose as *mut png_byte as png_const_bytep,
        purpose_len as size_t,
    );
    png_save_int_32(&raw mut buf as png_bytep, X0);
    png_save_int_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        X1,
    );
    buf[8 as ::core::ffi::c_int as usize] = type_0 as png_byte;
    buf[9 as ::core::ffi::c_int as usize] = nparams as png_byte;
    png_write_chunk_data(
        png_ptr,
        &raw mut buf as *mut png_byte as png_const_bytep,
        10 as size_t,
    );
    png_write_chunk_data(png_ptr, units as png_const_bytep, units_len);
    i = 0 as ::core::ffi::c_int;
    while i < nparams {
        png_write_chunk_data(
            png_ptr,
            *params.offset(i as isize) as png_const_bytep,
            *params_len.offset(i as isize),
        );
        i += 1;
    }
    png_free(png_ptr, params_len as png_voidp);
    png_write_chunk_end(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_sCAL_s(
    mut png_ptr: png_structrp,
    mut unit: ::core::ffi::c_int,
    mut width: png_const_charp,
    mut height: png_const_charp,
) {
    let mut buf: [png_byte; 64] = [0; 64];
    let mut wlen: size_t = 0;
    let mut hlen: size_t = 0;
    let mut total_len: size_t = 0;
    wlen = strlen(width as *const ::core::ffi::c_char);
    hlen = strlen(height as *const ::core::ffi::c_char);
    total_len = wlen.wrapping_add(hlen).wrapping_add(2 as size_t);
    if total_len > 64 as size_t {
        png_warning(
            png_ptr,
            b"Can't write sCAL (buffer too small)\0" as *const u8 as png_const_charp,
        );
        return;
    }
    buf[0 as ::core::ffi::c_int as usize] = unit as png_byte;
    memcpy(
        (&raw mut buf as *mut png_byte).offset(1 as ::core::ffi::c_int as isize)
            as *mut ::core::ffi::c_void,
        width as *const ::core::ffi::c_void,
        wlen.wrapping_add(1 as size_t),
    );
    memcpy(
        (&raw mut buf as *mut png_byte)
            .offset(wlen as isize)
            .offset(2 as ::core::ffi::c_int as isize) as *mut ::core::ffi::c_void,
        height as *const ::core::ffi::c_void,
        hlen,
    );
    png_write_complete_chunk(
        png_ptr,
        png_sCAL,
        &raw mut buf as *mut png_byte as png_const_bytep,
        total_len,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_pHYs(
    mut png_ptr: png_structrp,
    mut x_pixels_per_unit: png_uint_32,
    mut y_pixels_per_unit: png_uint_32,
    mut unit_type: ::core::ffi::c_int,
) {
    let mut buf: [png_byte; 9] = [0; 9];
    if unit_type >= PNG_RESOLUTION_LAST {
        png_warning(
            png_ptr,
            b"Unrecognized unit type for pHYs chunk\0" as *const u8 as png_const_charp,
        );
    }
    png_save_uint_32(&raw mut buf as png_bytep, x_pixels_per_unit);
    png_save_uint_32(
        (&raw mut buf as *mut png_byte).offset(4 as ::core::ffi::c_int as isize),
        y_pixels_per_unit,
    );
    buf[8 as ::core::ffi::c_int as usize] = unit_type as png_byte;
    png_write_complete_chunk(
        png_ptr,
        png_pHYs,
        &raw mut buf as *mut png_byte as png_const_bytep,
        9 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_tIME(mut png_ptr: png_structrp, mut mod_time: png_const_timep) {
    let mut buf: [png_byte; 7] = [0; 7];
    if (*mod_time).month as ::core::ffi::c_int > 12 as ::core::ffi::c_int
        || ((*mod_time).month as ::core::ffi::c_int) < 1 as ::core::ffi::c_int
        || (*mod_time).day as ::core::ffi::c_int > 31 as ::core::ffi::c_int
        || ((*mod_time).day as ::core::ffi::c_int) < 1 as ::core::ffi::c_int
        || (*mod_time).hour as ::core::ffi::c_int > 23 as ::core::ffi::c_int
        || (*mod_time).second as ::core::ffi::c_int > 60 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Invalid time specified for tIME chunk\0" as *const u8 as png_const_charp,
        );
        return;
    }
    png_save_uint_16(
        &raw mut buf as png_bytep,
        (*mod_time).year as ::core::ffi::c_uint,
    );
    buf[2 as ::core::ffi::c_int as usize] = (*mod_time).month;
    buf[3 as ::core::ffi::c_int as usize] = (*mod_time).day;
    buf[4 as ::core::ffi::c_int as usize] = (*mod_time).hour;
    buf[5 as ::core::ffi::c_int as usize] = (*mod_time).minute;
    buf[6 as ::core::ffi::c_int as usize] = (*mod_time).second;
    png_write_complete_chunk(
        png_ptr,
        png_tIME,
        &raw mut buf as *mut png_byte as png_const_bytep,
        7 as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_start_row(mut png_ptr: png_structrp) {
    let mut buf_size: png_alloc_size_t = 0;
    let mut usr_pixel_depth: ::core::ffi::c_int = 0;
    let mut filters: png_byte = 0;
    usr_pixel_depth = (*png_ptr).usr_channels as ::core::ffi::c_int
        * (*png_ptr).usr_bit_depth as ::core::ffi::c_int;
    buf_size = (if usr_pixel_depth >= 8 as ::core::ffi::c_int {
        ((*png_ptr).width as size_t)
            .wrapping_mul(usr_pixel_depth as size_t >> 3 as ::core::ffi::c_int)
    } else {
        ((*png_ptr).width as size_t)
            .wrapping_mul(usr_pixel_depth as size_t)
            .wrapping_add(7 as size_t)
            >> 3 as ::core::ffi::c_int
    })
    .wrapping_add(1 as size_t) as png_alloc_size_t;
    (*png_ptr).transformed_pixel_depth = (*png_ptr).pixel_depth;
    (*png_ptr).maximum_pixel_depth = usr_pixel_depth as png_byte;
    (*png_ptr).row_buf = png_malloc(png_ptr, buf_size) as png_bytep;
    *(*png_ptr).row_buf.offset(0 as ::core::ffi::c_int as isize) =
        PNG_FILTER_VALUE_NONE as png_byte;
    filters = (*png_ptr).do_filter;
    if (*png_ptr).height == 1 as ::core::ffi::c_uint {
        filters = (filters as ::core::ffi::c_int
            & (0xff as ::core::ffi::c_int & !(PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)))
            as png_byte;
    }
    if (*png_ptr).width == 1 as ::core::ffi::c_uint {
        filters = (filters as ::core::ffi::c_int
            & (0xff as ::core::ffi::c_int & !(PNG_FILTER_SUB | PNG_FILTER_AVG | PNG_FILTER_PAETH)))
            as png_byte;
    }
    if filters as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        filters = PNG_FILTER_NONE as png_byte;
    }
    (*png_ptr).do_filter = filters;
    if filters as ::core::ffi::c_int
        & (PNG_FILTER_SUB | PNG_FILTER_UP | PNG_FILTER_AVG | PNG_FILTER_PAETH)
        != 0 as ::core::ffi::c_int
        && (*png_ptr).try_row.is_null()
    {
        let mut num_filters: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        (*png_ptr).try_row = png_malloc(png_ptr, buf_size) as png_bytep;
        if filters as ::core::ffi::c_int & PNG_FILTER_SUB != 0 {
            num_filters += 1;
        }
        if filters as ::core::ffi::c_int & PNG_FILTER_UP != 0 {
            num_filters += 1;
        }
        if filters as ::core::ffi::c_int & PNG_FILTER_AVG != 0 {
            num_filters += 1;
        }
        if filters as ::core::ffi::c_int & PNG_FILTER_PAETH != 0 {
            num_filters += 1;
        }
        if num_filters > 1 as ::core::ffi::c_int {
            (*png_ptr).tst_row = png_malloc(png_ptr, buf_size) as png_bytep;
        }
    }
    if filters as ::core::ffi::c_int & (PNG_FILTER_AVG | PNG_FILTER_UP | PNG_FILTER_PAETH)
        != 0 as ::core::ffi::c_int
    {
        (*png_ptr).prev_row = png_calloc(png_ptr, buf_size) as png_bytep;
    }
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            == 0 as ::core::ffi::c_uint
        {
            (*png_ptr).num_rows = ((*png_ptr).height as ::core::ffi::c_uint)
                .wrapping_add(
                    png_pass_yinc[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint,
                )
                .wrapping_sub(1 as ::core::ffi::c_uint)
                .wrapping_sub(
                    png_pass_ystart[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint,
                )
                .wrapping_div(
                    png_pass_yinc[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint,
                ) as png_uint_32;
            (*png_ptr).usr_width = ((*png_ptr).width as ::core::ffi::c_uint)
                .wrapping_add(png_pass_inc[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint)
                .wrapping_sub(1 as ::core::ffi::c_uint)
                .wrapping_sub(
                    png_pass_start[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint,
                )
                .wrapping_div(png_pass_inc[0 as ::core::ffi::c_int as usize] as ::core::ffi::c_uint)
                as png_uint_32;
        } else {
            (*png_ptr).num_rows = (*png_ptr).height;
            (*png_ptr).usr_width = (*png_ptr).width;
        }
    } else {
        (*png_ptr).num_rows = (*png_ptr).height;
        (*png_ptr).usr_width = (*png_ptr).width;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_finish_row(mut png_ptr: png_structrp) {
    (*png_ptr).row_number = (*png_ptr).row_number.wrapping_add(1);
    if (*png_ptr).row_number < (*png_ptr).num_rows {
        return;
    }
    if (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        (*png_ptr).row_number = 0 as png_uint_32;
        if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
            != 0 as ::core::ffi::c_uint
        {
            (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
        } else {
            loop {
                (*png_ptr).pass = (*png_ptr).pass.wrapping_add(1);
                if (*png_ptr).pass as ::core::ffi::c_int >= 7 as ::core::ffi::c_int {
                    break;
                }
                (*png_ptr).usr_width = ((*png_ptr).width as ::core::ffi::c_uint)
                    .wrapping_add(png_pass_inc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    .wrapping_sub(1 as ::core::ffi::c_uint)
                    .wrapping_sub(png_pass_start[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    .wrapping_div(png_pass_inc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    as png_uint_32;
                (*png_ptr).num_rows = ((*png_ptr).height as ::core::ffi::c_uint)
                    .wrapping_add(png_pass_yinc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    .wrapping_sub(1 as ::core::ffi::c_uint)
                    .wrapping_sub(png_pass_ystart[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    .wrapping_div(png_pass_yinc[(*png_ptr).pass as usize] as ::core::ffi::c_uint)
                    as png_uint_32;
                if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INTERLACE
                    != 0 as ::core::ffi::c_uint
                {
                    break;
                }
                if !((*png_ptr).usr_width == 0 as ::core::ffi::c_uint
                    || (*png_ptr).num_rows == 0 as ::core::ffi::c_uint)
                {
                    break;
                }
            }
        }
        if ((*png_ptr).pass as ::core::ffi::c_int) < 7 as ::core::ffi::c_int {
            if !(*png_ptr).prev_row.is_null() {
                memset(
                    (*png_ptr).prev_row as *mut ::core::ffi::c_void,
                    0 as ::core::ffi::c_int,
                    (if (*png_ptr).usr_channels as ::core::ffi::c_int
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
                    .wrapping_add(1 as size_t),
                );
            }
            return;
        }
    }
    png_compress_IDAT(
        png_ptr,
        ::core::ptr::null::<png_byte>(),
        0 as png_alloc_size_t,
        Z_FINISH,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_write_interlace(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut pass: ::core::ffi::c_int,
) {
    if pass < 6 as ::core::ffi::c_int {
        match (*row_info).pixel_depth as ::core::ffi::c_int {
            1 => {
                let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut shift: ::core::ffi::c_uint = 0;
                let mut d: ::core::ffi::c_int = 0;
                let mut value: ::core::ffi::c_int = 0;
                let mut i: png_uint_32 = 0;
                let mut row_width: png_uint_32 = (*row_info).width;
                dp = row;
                d = 0 as ::core::ffi::c_int;
                shift = 7 as ::core::ffi::c_uint;
                i = png_pass_start[pass as usize] as png_uint_32;
                while i < row_width {
                    sp = row.offset((i >> 3 as ::core::ffi::c_int) as size_t as isize);
                    value = *sp as ::core::ffi::c_int
                        >> 7 as ::core::ffi::c_int
                            - (i as ::core::ffi::c_uint & 0x7 as ::core::ffi::c_uint)
                                as ::core::ffi::c_int
                        & 0x1 as ::core::ffi::c_int;
                    d |= value << shift;
                    if shift == 0 as ::core::ffi::c_uint {
                        shift = 7 as ::core::ffi::c_uint;
                        let fresh1 = dp;
                        dp = dp.offset(1);
                        *fresh1 = d as png_byte;
                        d = 0 as ::core::ffi::c_int;
                    } else {
                        shift = shift.wrapping_sub(1);
                    }
                    i = (i as ::core::ffi::c_uint)
                        .wrapping_add(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
                        as png_uint_32 as png_uint_32;
                }
                if shift != 7 as ::core::ffi::c_uint {
                    *dp = d as png_byte;
                }
            }
            2 => {
                let mut sp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut shift_0: ::core::ffi::c_uint = 0;
                let mut d_0: ::core::ffi::c_int = 0;
                let mut value_0: ::core::ffi::c_int = 0;
                let mut i_0: png_uint_32 = 0;
                let mut row_width_0: png_uint_32 = (*row_info).width;
                dp_0 = row;
                shift_0 = 6 as ::core::ffi::c_uint;
                d_0 = 0 as ::core::ffi::c_int;
                i_0 = png_pass_start[pass as usize] as png_uint_32;
                while i_0 < row_width_0 {
                    sp_0 = row.offset((i_0 >> 2 as ::core::ffi::c_int) as size_t as isize);
                    value_0 = *sp_0 as ::core::ffi::c_int
                        >> ((3 as ::core::ffi::c_int
                            - (i_0 as ::core::ffi::c_uint & 0x3 as ::core::ffi::c_uint)
                                as ::core::ffi::c_int)
                            << 1 as ::core::ffi::c_int)
                        & 0x3 as ::core::ffi::c_int;
                    d_0 |= value_0 << shift_0;
                    if shift_0 == 0 as ::core::ffi::c_uint {
                        shift_0 = 6 as ::core::ffi::c_uint;
                        let fresh2 = dp_0;
                        dp_0 = dp_0.offset(1);
                        *fresh2 = d_0 as png_byte;
                        d_0 = 0 as ::core::ffi::c_int;
                    } else {
                        shift_0 = shift_0.wrapping_sub(2 as ::core::ffi::c_uint);
                    }
                    i_0 = (i_0 as ::core::ffi::c_uint)
                        .wrapping_add(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
                        as png_uint_32 as png_uint_32;
                }
                if shift_0 != 6 as ::core::ffi::c_uint {
                    *dp_0 = d_0 as png_byte;
                }
            }
            4 => {
                let mut sp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut shift_1: ::core::ffi::c_uint = 0;
                let mut d_1: ::core::ffi::c_int = 0;
                let mut value_1: ::core::ffi::c_int = 0;
                let mut i_1: png_uint_32 = 0;
                let mut row_width_1: png_uint_32 = (*row_info).width;
                dp_1 = row;
                shift_1 = 4 as ::core::ffi::c_uint;
                d_1 = 0 as ::core::ffi::c_int;
                i_1 = png_pass_start[pass as usize] as png_uint_32;
                while i_1 < row_width_1 {
                    sp_1 = row.offset((i_1 >> 1 as ::core::ffi::c_int) as size_t as isize);
                    value_1 = *sp_1 as ::core::ffi::c_int
                        >> ((1 as ::core::ffi::c_int
                            - (i_1 as ::core::ffi::c_uint & 0x1 as ::core::ffi::c_uint)
                                as ::core::ffi::c_int)
                            << 2 as ::core::ffi::c_int)
                        & 0xf as ::core::ffi::c_int;
                    d_1 |= value_1 << shift_1;
                    if shift_1 == 0 as ::core::ffi::c_uint {
                        shift_1 = 4 as ::core::ffi::c_uint;
                        let fresh3 = dp_1;
                        dp_1 = dp_1.offset(1);
                        *fresh3 = d_1 as png_byte;
                        d_1 = 0 as ::core::ffi::c_int;
                    } else {
                        shift_1 = shift_1.wrapping_sub(4 as ::core::ffi::c_uint);
                    }
                    i_1 = (i_1 as ::core::ffi::c_uint)
                        .wrapping_add(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
                        as png_uint_32 as png_uint_32;
                }
                if shift_1 != 4 as ::core::ffi::c_uint {
                    *dp_1 = d_1 as png_byte;
                }
            }
            _ => {
                let mut sp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut i_2: png_uint_32 = 0;
                let mut row_width_2: png_uint_32 = (*row_info).width;
                let mut pixel_bytes: size_t = 0;
                dp_2 = row;
                pixel_bytes = ((*row_info).pixel_depth as ::core::ffi::c_int
                    >> 3 as ::core::ffi::c_int) as size_t;
                i_2 = png_pass_start[pass as usize] as png_uint_32;
                while i_2 < row_width_2 {
                    sp_2 = row.offset((i_2 as size_t).wrapping_mul(pixel_bytes) as isize);
                    if dp_2 != sp_2 {
                        memcpy(
                            dp_2 as *mut ::core::ffi::c_void,
                            sp_2 as *const ::core::ffi::c_void,
                            pixel_bytes,
                        );
                    }
                    dp_2 = dp_2.offset(pixel_bytes as isize);
                    i_2 = (i_2 as ::core::ffi::c_uint)
                        .wrapping_add(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
                        as png_uint_32 as png_uint_32;
                }
            }
        }
        (*row_info).width = ((*row_info).width as ::core::ffi::c_uint)
            .wrapping_add(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
            .wrapping_sub(1 as ::core::ffi::c_uint)
            .wrapping_sub(png_pass_start[pass as usize] as ::core::ffi::c_uint)
            .wrapping_div(png_pass_inc[pass as usize] as ::core::ffi::c_uint)
            as png_uint_32;
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
unsafe extern "C" fn png_setup_sub_row(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
    mut lmins: size_t,
) -> size_t {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    let mut sum: size_t = 0 as size_t;
    let mut v: ::core::ffi::c_uint = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_SUB as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp as size_t {
        *dp = *rp;
        v = *dp as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        dp = dp.offset(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        *dp = (*rp as ::core::ffi::c_int - *lp as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            as png_byte;
        v = *dp as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if sum > lmins {
            break;
        }
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        lp = lp.offset(1);
        dp = dp.offset(1);
    }
    return sum;
}
unsafe extern "C" fn png_setup_sub_row_only(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
) {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_SUB as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp as size_t {
        *dp = *rp;
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        dp = dp.offset(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        *dp = (*rp as ::core::ffi::c_int - *lp as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            as png_byte;
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        lp = lp.offset(1);
        dp = dp.offset(1);
    }
}
unsafe extern "C" fn png_setup_up_row(
    mut png_ptr: png_structrp,
    mut row_bytes: size_t,
    mut lmins: size_t,
) -> size_t {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    let mut sum: size_t = 0 as size_t;
    let mut v: ::core::ffi::c_uint = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_UP as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        *dp = (*rp as ::core::ffi::c_int - *pp as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            as png_byte;
        v = *dp as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if sum > lmins {
            break;
        }
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        pp = pp.offset(1);
        dp = dp.offset(1);
    }
    return sum;
}
unsafe extern "C" fn png_setup_up_row_only(mut png_ptr: png_structrp, mut row_bytes: size_t) {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_UP as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        *dp = (*rp as ::core::ffi::c_int - *pp as ::core::ffi::c_int & 0xff as ::core::ffi::c_int)
            as png_byte;
        i = i.wrapping_add(1);
        rp = rp.offset(1);
        pp = pp.offset(1);
        dp = dp.offset(1);
    }
}
unsafe extern "C" fn png_setup_avg_row(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
    mut lmins: size_t,
) -> size_t {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    let mut sum: size_t = 0 as size_t;
    let mut v: ::core::ffi::c_uint = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_AVG as png_byte;
    i = 0 as png_uint_32;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp {
        let fresh20 = rp;
        rp = rp.offset(1);
        let fresh21 = pp;
        pp = pp.offset(1);
        let fresh22 = dp;
        dp = dp.offset(1);
        *fresh22 = (*fresh20 as ::core::ffi::c_int
            - *fresh21 as ::core::ffi::c_int / 2 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        v = *fresh22 as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        i = i.wrapping_add(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    while (i as size_t) < row_bytes {
        let fresh23 = rp;
        rp = rp.offset(1);
        let fresh24 = pp;
        pp = pp.offset(1);
        let fresh25 = lp;
        lp = lp.offset(1);
        let fresh26 = dp;
        dp = dp.offset(1);
        *fresh26 = (*fresh23 as ::core::ffi::c_int
            - (*fresh24 as ::core::ffi::c_int + *fresh25 as ::core::ffi::c_int)
                / 2 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        v = *fresh26 as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if sum > lmins {
            break;
        }
        i = i.wrapping_add(1);
    }
    return sum;
}
unsafe extern "C" fn png_setup_avg_row_only(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
) {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: png_uint_32 = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) = PNG_FILTER_VALUE_AVG as png_byte;
    i = 0 as png_uint_32;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp {
        let fresh27 = rp;
        rp = rp.offset(1);
        let fresh28 = pp;
        pp = pp.offset(1);
        let fresh29 = dp;
        dp = dp.offset(1);
        *fresh29 = (*fresh27 as ::core::ffi::c_int
            - *fresh28 as ::core::ffi::c_int / 2 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        i = i.wrapping_add(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    while (i as size_t) < row_bytes {
        let fresh30 = rp;
        rp = rp.offset(1);
        let fresh31 = pp;
        pp = pp.offset(1);
        let fresh32 = lp;
        lp = lp.offset(1);
        let fresh33 = dp;
        dp = dp.offset(1);
        *fresh33 = (*fresh30 as ::core::ffi::c_int
            - (*fresh31 as ::core::ffi::c_int + *fresh32 as ::core::ffi::c_int)
                / 2 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn png_setup_paeth_row(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
    mut lmins: size_t,
) -> size_t {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut cp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    let mut sum: size_t = 0 as size_t;
    let mut v: ::core::ffi::c_uint = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) =
        PNG_FILTER_VALUE_PAETH as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp as size_t {
        let fresh4 = rp;
        rp = rp.offset(1);
        let fresh5 = pp;
        pp = pp.offset(1);
        let fresh6 = dp;
        dp = dp.offset(1);
        *fresh6 = (*fresh4 as ::core::ffi::c_int - *fresh5 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        v = *fresh6 as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        i = i.wrapping_add(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    cp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        let mut a: ::core::ffi::c_int = 0;
        let mut b: ::core::ffi::c_int = 0;
        let mut c: ::core::ffi::c_int = 0;
        let mut pa: ::core::ffi::c_int = 0;
        let mut pb: ::core::ffi::c_int = 0;
        let mut pc: ::core::ffi::c_int = 0;
        let mut p: ::core::ffi::c_int = 0;
        let fresh7 = pp;
        pp = pp.offset(1);
        b = *fresh7 as ::core::ffi::c_int;
        let fresh8 = cp;
        cp = cp.offset(1);
        c = *fresh8 as ::core::ffi::c_int;
        let fresh9 = lp;
        lp = lp.offset(1);
        a = *fresh9 as ::core::ffi::c_int;
        p = b - c;
        pc = a - c;
        pa = if p < 0 as ::core::ffi::c_int { -p } else { p };
        pb = if pc < 0 as ::core::ffi::c_int {
            -pc
        } else {
            pc
        };
        pc = if p + pc < 0 as ::core::ffi::c_int {
            -(p + pc)
        } else {
            p + pc
        };
        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };
        let fresh10 = rp;
        rp = rp.offset(1);
        let fresh11 = dp;
        dp = dp.offset(1);
        *fresh11 = (*fresh10 as ::core::ffi::c_int - p & 0xff as ::core::ffi::c_int) as png_byte;
        v = *fresh11 as ::core::ffi::c_uint;
        sum = (sum as ::core::ffi::c_ulong).wrapping_add(
            (if v < 128 as ::core::ffi::c_uint {
                v
            } else {
                (256 as ::core::ffi::c_uint).wrapping_sub(v)
            }) as ::core::ffi::c_ulong,
        ) as size_t as size_t;
        if sum > lmins {
            break;
        }
        i = i.wrapping_add(1);
    }
    return sum;
}
unsafe extern "C" fn png_setup_paeth_row_only(
    mut png_ptr: png_structrp,
    mut bpp: png_uint_32,
    mut row_bytes: size_t,
) {
    let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut pp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut cp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut lp: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut i: size_t = 0;
    *(*png_ptr).try_row.offset(0 as ::core::ffi::c_int as isize) =
        PNG_FILTER_VALUE_PAETH as png_byte;
    i = 0 as size_t;
    rp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    dp = (*png_ptr).try_row.offset(1 as ::core::ffi::c_int as isize);
    pp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < bpp as size_t {
        let fresh12 = rp;
        rp = rp.offset(1);
        let fresh13 = pp;
        pp = pp.offset(1);
        let fresh14 = dp;
        dp = dp.offset(1);
        *fresh14 = (*fresh12 as ::core::ffi::c_int - *fresh13 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_int) as png_byte;
        i = i.wrapping_add(1);
    }
    lp = (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize);
    cp = (*png_ptr).prev_row.offset(1 as ::core::ffi::c_int as isize);
    while i < row_bytes {
        let mut a: ::core::ffi::c_int = 0;
        let mut b: ::core::ffi::c_int = 0;
        let mut c: ::core::ffi::c_int = 0;
        let mut pa: ::core::ffi::c_int = 0;
        let mut pb: ::core::ffi::c_int = 0;
        let mut pc: ::core::ffi::c_int = 0;
        let mut p: ::core::ffi::c_int = 0;
        let fresh15 = pp;
        pp = pp.offset(1);
        b = *fresh15 as ::core::ffi::c_int;
        let fresh16 = cp;
        cp = cp.offset(1);
        c = *fresh16 as ::core::ffi::c_int;
        let fresh17 = lp;
        lp = lp.offset(1);
        a = *fresh17 as ::core::ffi::c_int;
        p = b - c;
        pc = a - c;
        pa = if p < 0 as ::core::ffi::c_int { -p } else { p };
        pb = if pc < 0 as ::core::ffi::c_int {
            -pc
        } else {
            pc
        };
        pc = if p + pc < 0 as ::core::ffi::c_int {
            -(p + pc)
        } else {
            p + pc
        };
        p = if pa <= pb && pa <= pc {
            a
        } else if pb <= pc {
            b
        } else {
            c
        };
        let fresh18 = rp;
        rp = rp.offset(1);
        let fresh19 = dp;
        dp = dp.offset(1);
        *fresh19 = (*fresh18 as ::core::ffi::c_int - p & 0xff as ::core::ffi::c_int) as png_byte;
        i = i.wrapping_add(1);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_write_find_filter(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
) {
    let mut filter_to_do: ::core::ffi::c_uint = (*png_ptr).do_filter as ::core::ffi::c_uint;
    let mut row_buf: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut best_row: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut bpp: png_uint_32 = 0;
    let mut mins: size_t = 0;
    let mut row_bytes: size_t = (*row_info).rowbytes;
    bpp = ((*row_info).pixel_depth as ::core::ffi::c_int + 7 as ::core::ffi::c_int
        >> 3 as ::core::ffi::c_int) as png_uint_32;
    row_buf = (*png_ptr).row_buf;
    mins = PNG_SIZE_MAX.wrapping_sub(256 as size_t);
    best_row = (*png_ptr).row_buf;
    if PNG_SIZE_MAX.wrapping_div(128 as size_t) <= row_bytes {
        filter_to_do &= (0 as ::core::ffi::c_uint).wrapping_sub(filter_to_do);
    } else if filter_to_do & PNG_FILTER_NONE as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint
        && filter_to_do != PNG_FILTER_NONE as ::core::ffi::c_uint
    {
        let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
        let mut sum: size_t = 0 as size_t;
        let mut i: size_t = 0;
        let mut v: ::core::ffi::c_uint = 0;
        i = 0 as size_t;
        rp = row_buf.offset(1 as ::core::ffi::c_int as isize);
        while i < row_bytes {
            v = *rp as ::core::ffi::c_uint;
            sum = (sum as ::core::ffi::c_ulong).wrapping_add(
                (if v < 128 as ::core::ffi::c_uint {
                    v
                } else {
                    (256 as ::core::ffi::c_uint).wrapping_sub(v)
                }) as ::core::ffi::c_ulong,
            ) as size_t as size_t;
            i = i.wrapping_add(1);
            rp = rp.offset(1);
        }
        mins = sum;
    }
    if filter_to_do == PNG_FILTER_SUB as ::core::ffi::c_uint {
        png_setup_sub_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if filter_to_do & PNG_FILTER_SUB as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
        let mut sum_0: size_t = 0;
        let mut lmins: size_t = mins;
        sum_0 = png_setup_sub_row(png_ptr, bpp, row_bytes, lmins);
        if sum_0 < mins {
            mins = sum_0;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }
    if filter_to_do == PNG_FILTER_UP as ::core::ffi::c_uint {
        png_setup_up_row_only(png_ptr, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if filter_to_do & PNG_FILTER_UP as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
        let mut sum_1: size_t = 0;
        let mut lmins_0: size_t = mins;
        sum_1 = png_setup_up_row(png_ptr, row_bytes, lmins_0);
        if sum_1 < mins {
            mins = sum_1;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }
    if filter_to_do == PNG_FILTER_AVG as ::core::ffi::c_uint {
        png_setup_avg_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if filter_to_do & PNG_FILTER_AVG as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
        let mut sum_2: size_t = 0;
        let mut lmins_1: size_t = mins;
        sum_2 = png_setup_avg_row(png_ptr, bpp, row_bytes, lmins_1);
        if sum_2 < mins {
            mins = sum_2;
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }
    if filter_to_do == PNG_FILTER_PAETH as ::core::ffi::c_uint {
        png_setup_paeth_row_only(png_ptr, bpp, row_bytes);
        best_row = (*png_ptr).try_row;
    } else if filter_to_do & PNG_FILTER_PAETH as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
        let mut sum_3: size_t = 0;
        let mut lmins_2: size_t = mins;
        sum_3 = png_setup_paeth_row(png_ptr, bpp, row_bytes, lmins_2);
        if sum_3 < mins {
            best_row = (*png_ptr).try_row;
            if !(*png_ptr).tst_row.is_null() {
                (*png_ptr).try_row = (*png_ptr).tst_row;
                (*png_ptr).tst_row = best_row;
            }
        }
    }
    png_write_filtered_row(
        png_ptr,
        best_row,
        (*row_info).rowbytes.wrapping_add(1 as size_t),
    );
}
unsafe extern "C" fn png_write_filtered_row(
    mut png_ptr: png_structrp,
    mut filtered_row: png_bytep,
    mut full_row_length: size_t,
) {
    png_compress_IDAT(
        png_ptr,
        filtered_row as png_const_bytep,
        full_row_length as png_alloc_size_t,
        Z_NO_FLUSH,
    );
    if !(*png_ptr).prev_row.is_null() {
        let mut tptr: png_bytep = ::core::ptr::null_mut::<png_byte>();
        tptr = (*png_ptr).prev_row;
        (*png_ptr).prev_row = (*png_ptr).row_buf;
        (*png_ptr).row_buf = tptr;
    }
    png_write_finish_row(png_ptr);
    (*png_ptr).flush_rows = (*png_ptr).flush_rows.wrapping_add(1);
    if (*png_ptr).flush_dist > 0 as ::core::ffi::c_uint
        && (*png_ptr).flush_rows >= (*png_ptr).flush_dist
    {
        png_write_flush(png_ptr);
    }
}
pub const PNG_HAVE_IDAT: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_HAVE_IEND: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_HAVE_PNG_SIGNATURE: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_INTERLACE: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZLIB_CUSTOM_STRATEGY: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_FLAG_ZSTREAM_INITIALIZED: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
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
pub const png_bKGD: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 98 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 75 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 71 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 68 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_cHRM: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 99 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 72 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 82 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 77 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_cICP: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 99 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 80 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_cLLI: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 99 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_eXIf: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 101 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 88 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 102 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_gAMA: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 103 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 65 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 77 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 65 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_hIST: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 104 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 83 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_iCCP: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 105 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 80 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_iTXt: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 105 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 88 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 116 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_mDCV: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 109 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 68 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 86 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_oFFs: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 111 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 70 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 70 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 115 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_pCAL: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 112 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 65 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_pHYs: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 112 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 72 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 89 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 115 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_sBIT: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 115 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 66 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_sCAL: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 115 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 67 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 65 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_sPLT: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 115 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 80 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 76 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_sRGB: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 115 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 82 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 71 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 66 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_tEXt: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 116 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 69 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 88 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 116 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_tIME: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 116 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 73 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 77 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 69 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_tRNS: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 116 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 82 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 78 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 83 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
pub const png_zTXt: ::core::ffi::c_uint = (0xffffffff as ::core::ffi::c_uint
    & 122 as ::core::ffi::c_uint)
    << 24 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 84 as ::core::ffi::c_uint) << 16 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 88 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int
    | (0xffffffff as ::core::ffi::c_uint & 116 as ::core::ffi::c_uint) << 0 as ::core::ffi::c_int;
