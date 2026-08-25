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
    fn free(__ptr: *mut ::core::ffi::c_void);
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
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn fclose(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn longjmp(__env: *mut __jmp_buf_tag, __val: ::core::ffi::c_int) -> !;
    fn png_destroy_read_struct(
        png_ptr_ptr: png_structpp,
        info_ptr_ptr: png_infopp,
        end_info_ptr_ptr: png_infopp,
    );
    fn png_destroy_write_struct(png_ptr_ptr: png_structpp, info_ptr_ptr: png_infopp);
    fn png_set_error_fn(
        png_ptr: png_structrp,
        error_ptr: png_voidp,
        error_fn: png_error_ptr,
        warning_fn: png_error_ptr,
    );
    fn png_set_mem_fn(
        png_ptr: png_structrp,
        mem_ptr: png_voidp,
        malloc_fn: png_malloc_ptr,
        free_fn: png_free_ptr,
    );
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_chunk_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_save_uint_32(buf: png_bytep, i: png_uint_32);
    fn frexp(
        __x: ::core::ffi::c_double,
        __exponent: *mut ::core::ffi::c_int,
    ) -> ::core::ffi::c_double;
    fn modf(
        __x: ::core::ffi::c_double,
        __iptr: *mut ::core::ffi::c_double,
    ) -> ::core::ffi::c_double;
    fn pow(__x: ::core::ffi::c_double, __y: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn floor(__x: ::core::ffi::c_double) -> ::core::ffi::c_double;
    fn inflateReset(strm: z_streamp) -> ::core::ffi::c_int;
    fn crc32(crc: uLong, buf: *const Bytef, len: uInt) -> uLong;
    fn png_malloc_base(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_fixed_error(png_ptr: png_const_structrp, name: png_const_charp) -> !;
    fn png_safecat(
        buffer: png_charp,
        bufsize: size_t,
        pos: size_t,
        string: png_const_charp,
    ) -> size_t;
    fn png_format_number(
        start: png_const_charp,
        end: png_charp,
        format: ::core::ffi::c_int,
        number: png_alloc_size_t,
    ) -> png_charp;
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
pub type png_const_timep = *const png_time;
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
pub struct png_XYZ {
    pub red_X: png_fixed_point,
    pub red_Y: png_fixed_point,
    pub red_Z: png_fixed_point,
    pub green_X: png_fixed_point,
    pub green_Y: png_fixed_point,
    pub green_Z: png_fixed_point,
    pub blue_X: png_fixed_point,
    pub blue_Y: png_fixed_point,
    pub blue_Z: png_fixed_point,
}
pub type C2RustUnnamed_0 = ::core::ffi::c_uint;
pub const PNG_INDEX_unknown: C2RustUnnamed_0 = 28;
pub const PNG_INDEX_zTXt: C2RustUnnamed_0 = 27;
pub const PNG_INDEX_tRNS: C2RustUnnamed_0 = 26;
pub const PNG_INDEX_tIME: C2RustUnnamed_0 = 25;
pub const PNG_INDEX_tEXt: C2RustUnnamed_0 = 24;
pub const PNG_INDEX_sRGB: C2RustUnnamed_0 = 23;
pub const PNG_INDEX_sPLT: C2RustUnnamed_0 = 22;
pub const PNG_INDEX_sCAL: C2RustUnnamed_0 = 21;
pub const PNG_INDEX_sBIT: C2RustUnnamed_0 = 20;
pub const PNG_INDEX_pHYs: C2RustUnnamed_0 = 19;
pub const PNG_INDEX_pCAL: C2RustUnnamed_0 = 18;
pub const PNG_INDEX_oFFs: C2RustUnnamed_0 = 17;
pub const PNG_INDEX_mDCV: C2RustUnnamed_0 = 16;
pub const PNG_INDEX_iTXt: C2RustUnnamed_0 = 15;
pub const PNG_INDEX_iCCP: C2RustUnnamed_0 = 14;
pub const PNG_INDEX_hIST: C2RustUnnamed_0 = 13;
pub const PNG_INDEX_gAMA: C2RustUnnamed_0 = 12;
pub const PNG_INDEX_fdAT: C2RustUnnamed_0 = 11;
pub const PNG_INDEX_fcTL: C2RustUnnamed_0 = 10;
pub const PNG_INDEX_eXIf: C2RustUnnamed_0 = 9;
pub const PNG_INDEX_cLLI: C2RustUnnamed_0 = 8;
pub const PNG_INDEX_cICP: C2RustUnnamed_0 = 7;
pub const PNG_INDEX_cHRM: C2RustUnnamed_0 = 6;
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
pub const PNG_LIBPNG_VER: ::core::ffi::c_int = 10659 as ::core::ffi::c_int;
pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff as ::core::ffi::c_long as png_uint_32;
pub const PNG_SIZE_MAX: size_t = -(1 as ::core::ffi::c_int) as size_t;
pub const PNG_FP_1: ::core::ffi::c_int = 100000 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COMPRESSION_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FILTER_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_INTRAPIXEL_DIFFERENCING: ::core::ffi::c_int = 64 as ::core::ffi::c_int;
pub const PNG_INTERLACE_LAST: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_sRGB_INTENT_LAST: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_INFO_PLTE: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_INFO_tRNS: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_INFO_hIST: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_INFO_pCAL: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_INFO_iCCP: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_INFO_sPLT: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_INFO_sCAL: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_INFO_IDAT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_INFO_eXIf: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_FLAG_MNG_FILTER_64: ::core::ffi::c_int = 0x4 as ::core::ffi::c_int;
pub const PNG_DESTROY_WILL_FREE_DATA: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_USER_WILL_FREE_DATA: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_FREE_HIST: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_FREE_ICCP: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_FREE_SPLT: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_FREE_ROWS: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_FREE_PCAL: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_FREE_SCAL: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_FREE_UNKN: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_FREE_PLTE: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_FREE_TRNS: ::core::ffi::c_uint = 0x2000 as ::core::ffi::c_uint;
pub const PNG_FREE_TEXT: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_FREE_EXIF: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_FREE_ALL: ::core::ffi::c_uint = 0xffff as ::core::ffi::c_uint;
pub const PNG_FREE_MUL: ::core::ffi::c_uint = 0x4220 as ::core::ffi::c_uint;
pub const PNG_HANDLE_CHUNK_AS_DEFAULT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_IMAGE_ERROR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_OPTION_NEXT: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const PNG_OPTION_INVALID: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_GAMMA_THRESHOLD_FIXED: ::core::ffi::c_int = 5000 as ::core::ffi::c_int;
pub const PNG_MAX_GAMMA_8: ::core::ffi::c_int = 11 as ::core::ffi::c_int;
pub const PNG_USER_CHUNK_CACHE_MAX: ::core::ffi::c_int = 1000 as ::core::ffi::c_int;
pub const PNG_USER_CHUNK_MALLOC_MAX: ::core::ffi::c_int = 8000000 as ::core::ffi::c_int;
pub const PNG_USER_HEIGHT_MAX: ::core::ffi::c_int = 1000000 as ::core::ffi::c_int;
pub const PNG_USER_WIDTH_MAX: ::core::ffi::c_int = 1000000 as ::core::ffi::c_int;
pub const DBL_DIG: ::core::ffi::c_int = __DBL_DIG__;
pub const DBL_MIN_10_EXP: ::core::ffi::c_int = __DBL_MIN_10_EXP__;
pub const DBL_MAX: ::core::ffi::c_double = __DBL_MAX__;
pub const DBL_MIN: ::core::ffi::c_double = __DBL_MIN__;
pub const Z_OK: ::core::ffi::c_int = 0;
pub const Z_STREAM_END: ::core::ffi::c_int = 1;
pub const Z_NEED_DICT: ::core::ffi::c_int = 2;
pub const Z_ERRNO: ::core::ffi::c_int = -1;
pub const Z_STREAM_ERROR: ::core::ffi::c_int = -(2 as ::core::ffi::c_int);
pub const Z_DATA_ERROR: ::core::ffi::c_int = -3;
pub const Z_MEM_ERROR: ::core::ffi::c_int = -4;
pub const Z_BUF_ERROR: ::core::ffi::c_int = -5;
pub const Z_VERSION_ERROR: ::core::ffi::c_int = -6;
pub const Z_NULL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sig_bytes(
    mut png_ptr: png_structrp,
    mut num_bytes: ::core::ffi::c_int,
) {
    let mut nb: ::core::ffi::c_uint = num_bytes as ::core::ffi::c_uint;
    if png_ptr.is_null() {
        return;
    }
    if num_bytes < 0 as ::core::ffi::c_int {
        nb = 0 as ::core::ffi::c_uint;
    }
    if nb > 8 as ::core::ffi::c_uint {
        png_error(
            png_ptr,
            b"Too many bytes for PNG signature\0" as *const u8 as png_const_charp,
        );
    }
    (*png_ptr).sig_bytes = nb as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_sig_cmp(
    mut sig: png_const_bytep,
    mut start: size_t,
    mut num_to_check: size_t,
) -> ::core::ffi::c_int {
    static mut png_signature: [png_byte; 8] = [
        137 as ::core::ffi::c_int as png_byte,
        80 as ::core::ffi::c_int as png_byte,
        78 as ::core::ffi::c_int as png_byte,
        71 as ::core::ffi::c_int as png_byte,
        13 as ::core::ffi::c_int as png_byte,
        10 as ::core::ffi::c_int as png_byte,
        26 as ::core::ffi::c_int as png_byte,
        10 as ::core::ffi::c_int as png_byte,
    ];
    if num_to_check > 8 as size_t {
        num_to_check = 8 as size_t;
    } else if num_to_check < 1 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    if start > 7 as size_t {
        return -(1 as ::core::ffi::c_int);
    }
    if start.wrapping_add(num_to_check) > 8 as size_t {
        num_to_check = (8 as size_t).wrapping_sub(start);
    }
    return memcmp(
        sig.offset(start as isize) as *const png_byte as *const ::core::ffi::c_void,
        (&raw const png_signature as *const png_byte).offset(start as isize) as *const png_byte
            as *const ::core::ffi::c_void,
        num_to_check,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zalloc(
    mut png_ptr: voidpf,
    mut items: uInt,
    mut size: uInt,
) -> voidpf {
    let mut num_bytes: png_alloc_size_t = size as png_alloc_size_t;
    if png_ptr.is_null() {
        return NULL_0;
    }
    if size != 0 as ::core::ffi::c_uint
        && items as png_alloc_size_t
            >= (!(0 as ::core::ffi::c_int as png_alloc_size_t))
                .wrapping_div(size as png_alloc_size_t)
    {
        png_warning(
            png_ptr as png_const_structrp,
            b"Potential overflow in png_zalloc()\0" as *const u8 as png_const_charp,
        );
        return NULL_0;
    }
    num_bytes = (num_bytes as ::core::ffi::c_ulong).wrapping_mul(items as ::core::ffi::c_ulong)
        as png_alloc_size_t as png_alloc_size_t;
    return png_malloc_warn(png_ptr as png_const_structrp, num_bytes) as voidpf;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zfree(mut png_ptr: voidpf, mut ptr: voidpf) {
    png_free(png_ptr as png_const_structrp, ptr as png_voidp);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_crc(mut png_ptr: png_structrp) {
    (*png_ptr).crc = crc32(0 as uLong, ::core::ptr::null::<Bytef>(), 0 as uInt) as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_calculate_crc(
    mut png_ptr: png_structrp,
    mut ptr: png_const_bytep,
    mut length: size_t,
) {
    let mut need_crc: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if 1 as png_uint_32 & (*png_ptr).chunk_name >> 29 as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_CRC_ANCILLARY_MASK
            == PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN
        {
            need_crc = 0 as ::core::ffi::c_int;
        }
    } else if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_CRC_CRITICAL_IGNORE
        != 0 as ::core::ffi::c_uint
    {
        need_crc = 0 as ::core::ffi::c_int;
    }
    if need_crc != 0 as ::core::ffi::c_int && length > 0 as size_t {
        let mut crc: uLong = (*png_ptr).crc as uLong;
        loop {
            let mut safe_length: uInt = length as uInt;
            if safe_length == 0 as ::core::ffi::c_uint {
                safe_length = -(1 as ::core::ffi::c_int) as uInt;
            }
            crc = crc32(crc, ptr as *const Bytef, safe_length);
            ptr = ptr.offset(safe_length as isize);
            length = (length as ::core::ffi::c_ulong)
                .wrapping_sub(safe_length as ::core::ffi::c_ulong) as size_t
                as size_t;
            if !(length > 0 as size_t) {
                break;
            }
        }
        (*png_ptr).crc = crc as png_uint_32;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_user_version_check(
    mut png_ptr: png_structrp,
    mut user_png_ver: png_const_charp,
) -> ::core::ffi::c_int {
    if !user_png_ver.is_null() {
        let mut i: ::core::ffi::c_int = -(1 as ::core::ffi::c_int);
        let mut found_dots: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        loop {
            i += 1;
            if *user_png_ver.offset(i as isize) as ::core::ffi::c_int
                != PNG_LIBPNG_VER_STRING[i as usize] as ::core::ffi::c_int
            {
                (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
            }
            if *user_png_ver.offset(i as isize) as ::core::ffi::c_int == '.' as i32 {
                found_dots += 1;
            }
            if !(found_dots < 2 as ::core::ffi::c_int
                && *user_png_ver.offset(i as isize) as ::core::ffi::c_int
                    != 0 as ::core::ffi::c_int
                && PNG_LIBPNG_VER_STRING[i as usize] as ::core::ffi::c_int
                    != 0 as ::core::ffi::c_int)
            {
                break;
            }
        }
    } else {
        (*png_ptr).flags |= PNG_FLAG_LIBRARY_MISMATCH;
    }
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_LIBRARY_MISMATCH
        != 0 as ::core::ffi::c_uint
    {
        let mut pos: size_t = 0 as size_t;
        let mut m: [::core::ffi::c_char; 128] = [0; 128];
        pos = png_safecat(
            &raw mut m as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            pos,
            b"Application built with libpng-\0" as *const u8 as png_const_charp,
        );
        pos = png_safecat(
            &raw mut m as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            pos,
            user_png_ver,
        );
        pos = png_safecat(
            &raw mut m as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            pos,
            b" but running with \0" as *const u8 as png_const_charp,
        );
        pos = png_safecat(
            &raw mut m as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 128]>() as size_t,
            pos,
            PNG_LIBPNG_VER_STRING.as_ptr(),
        );
        png_warning(
            png_ptr,
            &raw mut m as *mut ::core::ffi::c_char as png_const_charp,
        );
        return 0 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_png_struct(
    mut user_png_ver: png_const_charp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warn_fn: png_error_ptr,
    mut mem_ptr: png_voidp,
    mut malloc_fn: png_malloc_ptr,
    mut free_fn: png_free_ptr,
) -> png_structp {
    let mut create_struct: png_struct = png_struct {
        jmp_buf_local: [__jmp_buf_tag {
            __jmpbuf: [0; 8],
            __mask_was_saved: 0,
            __saved_mask: __sigset_t { __val: [0; 16] },
        }; 1],
        longjmp_fn: None,
        jmp_buf_ptr: ::core::ptr::null_mut::<jmp_buf>(),
        jmp_buf_size: 0,
        error_fn: None,
        warning_fn: None,
        error_ptr: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        write_data_fn: None,
        read_data_fn: None,
        io_ptr: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        read_user_transform_fn: None,
        write_user_transform_fn: None,
        user_transform_ptr: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        user_transform_depth: 0,
        user_transform_channels: 0,
        mode: 0,
        flags: 0,
        transformations: 0,
        zowner: 0,
        zstream: z_stream {
            next_in: ::core::ptr::null::<Bytef>(),
            avail_in: 0,
            total_in: 0,
            next_out: ::core::ptr::null_mut::<Bytef>(),
            avail_out: 0,
            total_out: 0,
            msg: ::core::ptr::null::<::core::ffi::c_char>(),
            state: ::core::ptr::null_mut::<::core::ffi::c_void>(),
            zalloc: None,
            zfree: None,
            opaque: ::core::ptr::null_mut::<::core::ffi::c_void>(),
            data_type: 0,
            adler: 0,
            reserved: 0,
        },
        zbuffer_list: ::core::ptr::null_mut::<png_compression_buffer>(),
        zbuffer_size: 0,
        zlib_level: 0,
        zlib_method: 0,
        zlib_window_bits: 0,
        zlib_mem_level: 0,
        zlib_strategy: 0,
        zlib_text_level: 0,
        zlib_text_method: 0,
        zlib_text_window_bits: 0,
        zlib_text_mem_level: 0,
        zlib_text_strategy: 0,
        zlib_set_level: 0,
        zlib_set_method: 0,
        zlib_set_window_bits: 0,
        zlib_set_mem_level: 0,
        zlib_set_strategy: 0,
        chunks: 0,
        width: 0,
        height: 0,
        num_rows: 0,
        usr_width: 0,
        rowbytes: 0,
        iwidth: 0,
        row_number: 0,
        chunk_name: 0,
        prev_row: ::core::ptr::null_mut::<png_byte>(),
        row_buf: ::core::ptr::null_mut::<png_byte>(),
        try_row: ::core::ptr::null_mut::<png_byte>(),
        tst_row: ::core::ptr::null_mut::<png_byte>(),
        info_rowbytes: 0,
        idat_size: 0,
        crc: 0,
        palette: ::core::ptr::null_mut::<png_color>(),
        num_palette: 0,
        num_palette_max: 0,
        num_trans: 0,
        compression: 0,
        filter: 0,
        interlaced: 0,
        pass: 0,
        do_filter: 0,
        color_type: 0,
        bit_depth: 0,
        usr_bit_depth: 0,
        pixel_depth: 0,
        channels: 0,
        usr_channels: 0,
        sig_bytes: 0,
        maximum_pixel_depth: 0,
        transformed_pixel_depth: 0,
        zstream_start: 0,
        filler: 0,
        background_gamma_type: 0,
        background_gamma: 0,
        background: png_color_16 {
            index: 0,
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
        },
        background_1: png_color_16 {
            index: 0,
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
        },
        output_flush_fn: None,
        flush_dist: 0,
        flush_rows: 0,
        chromaticities: png_xy {
            redx: 0,
            redy: 0,
            greenx: 0,
            greeny: 0,
            bluex: 0,
            bluey: 0,
            whitex: 0,
            whitey: 0,
        },
        gamma_shift: 0,
        screen_gamma: 0,
        file_gamma: 0,
        chunk_gamma: 0,
        default_gamma: 0,
        gamma_table: ::core::ptr::null_mut::<png_byte>(),
        gamma_16_table: ::core::ptr::null_mut::<*mut png_uint_16>(),
        gamma_from_1: ::core::ptr::null_mut::<png_byte>(),
        gamma_to_1: ::core::ptr::null_mut::<png_byte>(),
        gamma_16_from_1: ::core::ptr::null_mut::<*mut png_uint_16>(),
        gamma_16_to_1: ::core::ptr::null_mut::<*mut png_uint_16>(),
        sig_bit: png_color_8 {
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
            alpha: 0,
        },
        shift: png_color_8 {
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
            alpha: 0,
        },
        trans_alpha: ::core::ptr::null_mut::<png_byte>(),
        trans_color: png_color_16 {
            index: 0,
            red: 0,
            green: 0,
            blue: 0,
            gray: 0,
        },
        read_row_fn: None,
        write_row_fn: None,
        info_fn: None,
        row_fn: None,
        end_fn: None,
        save_buffer_ptr: ::core::ptr::null_mut::<png_byte>(),
        save_buffer: ::core::ptr::null_mut::<png_byte>(),
        current_buffer_ptr: ::core::ptr::null_mut::<png_byte>(),
        current_buffer: ::core::ptr::null_mut::<png_byte>(),
        push_length: 0,
        skip_length: 0,
        save_buffer_size: 0,
        save_buffer_max: 0,
        buffer_size: 0,
        current_buffer_size: 0,
        process_mode: 0,
        cur_palette: 0,
        palette_lookup: ::core::ptr::null_mut::<png_byte>(),
        quantize_index: ::core::ptr::null_mut::<png_byte>(),
        options: 0,
        time_buffer: [0; 29],
        free_me: 0,
        user_chunk_ptr: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        read_user_chunk_fn: None,
        unknown_default: 0,
        num_chunk_list: 0,
        chunk_list: ::core::ptr::null_mut::<png_byte>(),
        rgb_to_gray_status: 0,
        rgb_to_gray_coefficients_set: 0,
        rgb_to_gray_red_coeff: 0,
        rgb_to_gray_green_coeff: 0,
        riffled_palette: ::core::ptr::null_mut::<png_byte>(),
        mng_features_permitted: 0,
        filter_type: 0,
        mem_ptr: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        malloc_fn: None,
        free_fn: None,
        big_row_buf: ::core::ptr::null_mut::<png_byte>(),
        index_to_palette: ::core::ptr::null_mut::<png_byte>(),
        palette_to_index: ::core::ptr::null_mut::<png_byte>(),
        compression_type: 0,
        user_width_max: 0,
        user_height_max: 0,
        user_chunk_cache_max: 0,
        user_chunk_malloc_max: 0,
        unknown_chunk: png_unknown_chunk {
            name: [0; 5],
            data: ::core::ptr::null_mut::<png_byte>(),
            size: 0,
            location: 0,
        },
        old_big_row_buf_size: 0,
        read_buffer: ::core::ptr::null_mut::<png_byte>(),
        read_buffer_size: 0,
        IDAT_read_size: 0,
        io_state: 0,
        big_prev_row: ::core::ptr::null_mut::<png_byte>(),
        read_filter: [None; 4],
    };
    let mut create_jmp_buf: jmp_buf = [__jmp_buf_tag {
        __jmpbuf: [0; 8],
        __mask_was_saved: 0,
        __saved_mask: __sigset_t { __val: [0; 16] },
    }; 1];
    memset(
        &raw mut create_struct as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<png_struct>() as size_t,
    );
    create_struct.user_width_max = PNG_USER_WIDTH_MAX as png_uint_32;
    create_struct.user_height_max = PNG_USER_HEIGHT_MAX as png_uint_32;
    create_struct.user_chunk_cache_max = PNG_USER_CHUNK_CACHE_MAX as png_uint_32;
    create_struct.user_chunk_malloc_max = PNG_USER_CHUNK_MALLOC_MAX as png_alloc_size_t;
    png_set_mem_fn(&raw mut create_struct, mem_ptr, malloc_fn, free_fn);
    png_set_error_fn(&raw mut create_struct, error_ptr, error_fn, warn_fn);
    if _setjmp(&raw mut create_jmp_buf as *mut __jmp_buf_tag) == 0 {
        create_struct.jmp_buf_ptr = &raw mut create_jmp_buf;
        create_struct.jmp_buf_size = 0 as size_t;
        create_struct.longjmp_fn = ::core::mem::transmute::<
            Option<unsafe extern "C" fn(*mut __jmp_buf_tag, ::core::ffi::c_int) -> !>,
            png_longjmp_ptr,
        >(Some(
            longjmp as unsafe extern "C" fn(*mut __jmp_buf_tag, ::core::ffi::c_int) -> !,
        ));
        if png_user_version_check(&raw mut create_struct, user_png_ver) != 0 as ::core::ffi::c_int {
            let mut png_ptr: png_structrp = png_malloc_warn(
                &raw mut create_struct,
                ::core::mem::size_of::<png_struct>() as png_alloc_size_t,
            ) as png_structrp;
            if !png_ptr.is_null() {
                create_struct.zstream.zalloc =
                    Some(png_zalloc as unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf)
                        as alloc_func;
                create_struct.zstream.zfree =
                    Some(png_zfree as unsafe extern "C" fn(voidpf, voidpf) -> ()) as free_func;
                create_struct.zstream.opaque = png_ptr as voidpf;
                create_struct.jmp_buf_ptr = ::core::ptr::null_mut::<jmp_buf>();
                create_struct.jmp_buf_size = 0 as size_t;
                create_struct.longjmp_fn = None;
                *png_ptr = create_struct;
                return png_ptr as png_structp;
            }
        }
    }
    return ::core::ptr::null_mut::<png_struct>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_create_info_struct(mut png_ptr: png_const_structrp) -> png_infop {
    let mut info_ptr: png_inforp = ::core::ptr::null_mut::<png_info>();
    if png_ptr.is_null() {
        return ::core::ptr::null_mut::<png_info>();
    }
    info_ptr = png_malloc_base(
        png_ptr,
        ::core::mem::size_of::<png_info>() as png_alloc_size_t,
    ) as *mut png_info as png_inforp;
    if !info_ptr.is_null() {
        memset(
            info_ptr as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<png_info>() as size_t,
        );
    }
    return info_ptr as png_infop;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_info_struct(
    mut png_ptr: png_const_structrp,
    mut info_ptr_ptr: png_infopp,
) {
    let mut info_ptr: png_inforp = ::core::ptr::null_mut::<png_info>();
    if png_ptr.is_null() {
        return;
    }
    if !info_ptr_ptr.is_null() {
        info_ptr = *info_ptr_ptr as png_inforp;
    }
    if !info_ptr.is_null() {
        *info_ptr_ptr = ::core::ptr::null_mut::<png_info>();
        png_free_data(png_ptr, info_ptr, PNG_FREE_ALL, -(1 as ::core::ffi::c_int));
        memset(
            info_ptr as *mut ::core::ffi::c_void,
            0 as ::core::ffi::c_int,
            ::core::mem::size_of::<png_info>() as size_t,
        );
        png_free(png_ptr, info_ptr as png_voidp);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_info_init_3(
    mut ptr_ptr: png_infopp,
    mut png_info_struct_size: size_t,
) {
    let mut info_ptr: png_inforp = *ptr_ptr;
    if info_ptr.is_null() {
        return;
    }
    if ::core::mem::size_of::<png_info>() as usize > png_info_struct_size {
        *ptr_ptr = ::core::ptr::null_mut::<png_info>();
        free(info_ptr as *mut ::core::ffi::c_void);
        info_ptr = png_malloc_base(
            ::core::ptr::null::<png_struct>(),
            ::core::mem::size_of::<png_info>() as png_alloc_size_t,
        ) as *mut png_info as png_inforp;
        if info_ptr.is_null() {
            return;
        }
        *ptr_ptr = info_ptr as *mut png_info;
    }
    memset(
        info_ptr as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ::core::mem::size_of::<png_info>() as size_t,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_data_freer(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut freer: ::core::ffi::c_int,
    mut mask: png_uint_32,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if freer == PNG_DESTROY_WILL_FREE_DATA {
        (*info_ptr).free_me |= mask as ::core::ffi::c_uint;
    } else if freer == PNG_USER_WILL_FREE_DATA {
        (*info_ptr).free_me &= !mask as ::core::ffi::c_uint;
    } else {
        png_error(
            png_ptr,
            b"Unknown freer parameter in png_data_freer\0" as *const u8 as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_data(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut mask: png_uint_32,
    mut num: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if !(*info_ptr).text.is_null()
        && mask & PNG_FREE_TEXT & (*info_ptr).free_me != 0 as ::core::ffi::c_uint
    {
        if num != -(1 as ::core::ffi::c_int) {
            png_free(
                png_ptr,
                (*(*info_ptr).text.offset(num as isize)).key as png_voidp,
            );
            let ref mut fresh5 = (*(*info_ptr).text.offset(num as isize)).key;
            *fresh5 = ::core::ptr::null_mut::<::core::ffi::c_char>();
        } else {
            let mut i: ::core::ffi::c_int = 0;
            i = 0 as ::core::ffi::c_int;
            while i < (*info_ptr).num_text {
                png_free(
                    png_ptr,
                    (*(*info_ptr).text.offset(i as isize)).key as png_voidp,
                );
                i += 1;
            }
            png_free(png_ptr, (*info_ptr).text as png_voidp);
            (*info_ptr).text = ::core::ptr::null_mut::<png_text>();
            (*info_ptr).num_text = 0 as ::core::ffi::c_int;
            (*info_ptr).max_text = 0 as ::core::ffi::c_int;
        }
    }
    if mask & PNG_FREE_TRNS & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        (*info_ptr).valid &= !PNG_INFO_tRNS;
        png_free(png_ptr, (*info_ptr).trans_alpha as png_voidp);
        (*info_ptr).trans_alpha = ::core::ptr::null_mut::<png_byte>();
        (*info_ptr).num_trans = 0 as png_uint_16;
    }
    if mask & PNG_FREE_SCAL & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
        png_free(png_ptr, (*info_ptr).scal_s_height as png_voidp);
        (*info_ptr).scal_s_width = ::core::ptr::null_mut::<::core::ffi::c_char>();
        (*info_ptr).scal_s_height = ::core::ptr::null_mut::<::core::ffi::c_char>();
        (*info_ptr).valid &= !PNG_INFO_sCAL;
    }
    if mask & PNG_FREE_PCAL & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        png_free(png_ptr, (*info_ptr).pcal_purpose as png_voidp);
        png_free(png_ptr, (*info_ptr).pcal_units as png_voidp);
        (*info_ptr).pcal_purpose = ::core::ptr::null_mut::<::core::ffi::c_char>();
        (*info_ptr).pcal_units = ::core::ptr::null_mut::<::core::ffi::c_char>();
        if !(*info_ptr).pcal_params.is_null() {
            let mut i_0: ::core::ffi::c_int = 0;
            i_0 = 0 as ::core::ffi::c_int;
            while i_0 < (*info_ptr).pcal_nparams as ::core::ffi::c_int {
                png_free(
                    png_ptr,
                    *(*info_ptr).pcal_params.offset(i_0 as isize) as png_voidp,
                );
                i_0 += 1;
            }
            png_free(png_ptr, (*info_ptr).pcal_params as png_voidp);
            (*info_ptr).pcal_params = ::core::ptr::null_mut::<*mut ::core::ffi::c_char>();
        }
        (*info_ptr).valid &= !PNG_INFO_pCAL;
    }
    if mask & PNG_FREE_ICCP & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        png_free(png_ptr, (*info_ptr).iccp_name as png_voidp);
        png_free(png_ptr, (*info_ptr).iccp_profile as png_voidp);
        (*info_ptr).iccp_name = ::core::ptr::null_mut::<::core::ffi::c_char>();
        (*info_ptr).iccp_profile = ::core::ptr::null_mut::<png_byte>();
        (*info_ptr).valid &= !PNG_INFO_iCCP;
    }
    if !(*info_ptr).splt_palettes.is_null()
        && mask & PNG_FREE_SPLT & (*info_ptr).free_me != 0 as ::core::ffi::c_uint
    {
        if num != -(1 as ::core::ffi::c_int) {
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.offset(num as isize)).name as png_voidp,
            );
            png_free(
                png_ptr,
                (*(*info_ptr).splt_palettes.offset(num as isize)).entries as png_voidp,
            );
            let ref mut fresh6 = (*(*info_ptr).splt_palettes.offset(num as isize)).name;
            *fresh6 = ::core::ptr::null_mut::<::core::ffi::c_char>();
            let ref mut fresh7 = (*(*info_ptr).splt_palettes.offset(num as isize)).entries;
            *fresh7 = ::core::ptr::null_mut::<png_sPLT_entry>();
        } else {
            let mut i_1: ::core::ffi::c_int = 0;
            i_1 = 0 as ::core::ffi::c_int;
            while i_1 < (*info_ptr).splt_palettes_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.offset(i_1 as isize)).name as png_voidp,
                );
                png_free(
                    png_ptr,
                    (*(*info_ptr).splt_palettes.offset(i_1 as isize)).entries as png_voidp,
                );
                i_1 += 1;
            }
            png_free(png_ptr, (*info_ptr).splt_palettes as png_voidp);
            (*info_ptr).splt_palettes = ::core::ptr::null_mut::<png_sPLT_t>();
            (*info_ptr).splt_palettes_num = 0 as ::core::ffi::c_int;
            (*info_ptr).valid &= !PNG_INFO_sPLT;
        }
    }
    if !(*info_ptr).unknown_chunks.is_null()
        && mask & PNG_FREE_UNKN & (*info_ptr).free_me != 0 as ::core::ffi::c_uint
    {
        if num != -(1 as ::core::ffi::c_int) {
            png_free(
                png_ptr,
                (*(*info_ptr).unknown_chunks.offset(num as isize)).data as png_voidp,
            );
            let ref mut fresh8 = (*(*info_ptr).unknown_chunks.offset(num as isize)).data;
            *fresh8 = ::core::ptr::null_mut::<png_byte>();
        } else {
            let mut i_2: ::core::ffi::c_int = 0;
            i_2 = 0 as ::core::ffi::c_int;
            while i_2 < (*info_ptr).unknown_chunks_num {
                png_free(
                    png_ptr,
                    (*(*info_ptr).unknown_chunks.offset(i_2 as isize)).data as png_voidp,
                );
                i_2 += 1;
            }
            png_free(png_ptr, (*info_ptr).unknown_chunks as png_voidp);
            (*info_ptr).unknown_chunks = ::core::ptr::null_mut::<png_unknown_chunk>();
            (*info_ptr).unknown_chunks_num = 0 as ::core::ffi::c_int;
        }
    }
    if mask & PNG_FREE_EXIF & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        if !(*info_ptr).exif.is_null() {
            png_free(png_ptr, (*info_ptr).exif as png_voidp);
            (*info_ptr).exif = ::core::ptr::null_mut::<png_byte>();
        }
        (*info_ptr).valid &= !PNG_INFO_eXIf;
    }
    if mask & PNG_FREE_HIST & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        png_free(png_ptr, (*info_ptr).hist as png_voidp);
        (*info_ptr).hist = ::core::ptr::null_mut::<png_uint_16>();
        (*info_ptr).valid &= !PNG_INFO_hIST;
    }
    if mask & PNG_FREE_PLTE & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        png_free(png_ptr, (*info_ptr).palette as png_voidp);
        (*info_ptr).palette = ::core::ptr::null_mut::<png_color>();
        (*info_ptr).valid &= !PNG_INFO_PLTE;
        (*info_ptr).num_palette = 0 as png_uint_16;
    }
    if mask & PNG_FREE_ROWS & (*info_ptr).free_me != 0 as ::core::ffi::c_uint {
        if !(*info_ptr).row_pointers.is_null() {
            let mut row: png_uint_32 = 0;
            row = 0 as png_uint_32;
            while row < (*info_ptr).height {
                png_free(
                    png_ptr,
                    *(*info_ptr).row_pointers.offset(row as isize) as png_voidp,
                );
                row = row.wrapping_add(1);
            }
            png_free(png_ptr, (*info_ptr).row_pointers as png_voidp);
            (*info_ptr).row_pointers = ::core::ptr::null_mut::<*mut png_byte>();
        }
        (*info_ptr).valid &= !PNG_INFO_IDAT;
    }
    if num != -(1 as ::core::ffi::c_int) {
        mask &= !PNG_FREE_MUL;
    }
    (*info_ptr).free_me &= !mask as ::core::ffi::c_uint;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_ptr(mut png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return NULL_0;
    }
    return (*png_ptr).io_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_init_io(mut png_ptr: png_structrp, mut fp: *mut FILE) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).io_ptr = fp as png_voidp;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_save_int_32(mut buf: png_bytep, mut i: png_int_32) {
    png_save_uint_32(buf, i as png_uint_32);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123_buffer(
    mut out: *mut ::core::ffi::c_char,
    mut ptime: png_const_timep,
) -> ::core::ffi::c_int {
    static mut short_months: [[::core::ffi::c_char; 4]; 12] = unsafe {
        [
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Jan\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Feb\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Mar\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Apr\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"May\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Jun\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Jul\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Aug\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Sep\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Oct\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Nov\0"),
            ::core::mem::transmute::<[u8; 4], [::core::ffi::c_char; 4]>(*b"Dec\0"),
        ]
    };
    if out.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    if (*ptime).year as ::core::ffi::c_int > 9999 as ::core::ffi::c_int
        || (*ptime).month as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*ptime).month as ::core::ffi::c_int > 12 as ::core::ffi::c_int
        || (*ptime).day as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*ptime).day as ::core::ffi::c_int > 31 as ::core::ffi::c_int
        || (*ptime).hour as ::core::ffi::c_int > 23 as ::core::ffi::c_int
        || (*ptime).minute as ::core::ffi::c_int > 59 as ::core::ffi::c_int
        || (*ptime).second as ::core::ffi::c_int > 60 as ::core::ffi::c_int
    {
        return 0 as ::core::ffi::c_int;
    }
    let mut pos: size_t = 0 as size_t;
    let mut number_buf: [::core::ffi::c_char; 5] = [
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
        0 as ::core::ffi::c_int as ::core::ffi::c_char,
    ];
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        png_format_number(
            &raw mut number_buf as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut number_buf as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize as isize),
            1 as ::core::ffi::c_int,
            (*ptime).day as ::core::ffi::c_uint as png_alloc_size_t,
        ) as png_const_charp,
    );
    if pos < 28 as size_t {
        let fresh0 = pos;
        pos = pos.wrapping_add(1);
        *out.offset(fresh0 as isize) = ' ' as i32 as ::core::ffi::c_char;
    }
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        &raw const *(&raw const short_months as *const [::core::ffi::c_char; 4])
            .offset(((*ptime).month as ::core::ffi::c_int - 1 as ::core::ffi::c_int) as isize)
            as png_const_charp,
    );
    if pos < 28 as size_t {
        let fresh1 = pos;
        pos = pos.wrapping_add(1);
        *out.offset(fresh1 as isize) = ' ' as i32 as ::core::ffi::c_char;
    }
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        png_format_number(
            &raw mut number_buf as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut number_buf as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize as isize),
            1 as ::core::ffi::c_int,
            (*ptime).year as png_alloc_size_t,
        ) as png_const_charp,
    );
    if pos < 28 as size_t {
        let fresh2 = pos;
        pos = pos.wrapping_add(1);
        *out.offset(fresh2 as isize) = ' ' as i32 as ::core::ffi::c_char;
    }
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        png_format_number(
            &raw mut number_buf as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut number_buf as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize as isize),
            2 as ::core::ffi::c_int,
            (*ptime).hour as ::core::ffi::c_uint as png_alloc_size_t,
        ) as png_const_charp,
    );
    if pos < 28 as size_t {
        let fresh3 = pos;
        pos = pos.wrapping_add(1);
        *out.offset(fresh3 as isize) = ':' as i32 as ::core::ffi::c_char;
    }
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        png_format_number(
            &raw mut number_buf as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut number_buf as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize as isize),
            2 as ::core::ffi::c_int,
            (*ptime).minute as ::core::ffi::c_uint as png_alloc_size_t,
        ) as png_const_charp,
    );
    if pos < 28 as size_t {
        let fresh4 = pos;
        pos = pos.wrapping_add(1);
        *out.offset(fresh4 as isize) = ':' as i32 as ::core::ffi::c_char;
    }
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        png_format_number(
            &raw mut number_buf as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut number_buf as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 5]>() as usize as isize),
            2 as ::core::ffi::c_int,
            (*ptime).second as ::core::ffi::c_uint as png_alloc_size_t,
        ) as png_const_charp,
    );
    pos = png_safecat(
        out as png_charp,
        29 as size_t,
        pos,
        b" +0000\0" as *const u8 as png_const_charp,
    );
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_convert_to_rfc1123(
    mut png_ptr: png_structrp,
    mut ptime: png_const_timep,
) -> png_const_charp {
    if !png_ptr.is_null() {
        if png_convert_to_rfc1123_buffer(
            &raw mut (*png_ptr).time_buffer as *mut ::core::ffi::c_char,
            ptime,
        ) == 0 as ::core::ffi::c_int
        {
            png_warning(
                png_ptr,
                b"Ignoring invalid time value\0" as *const u8 as png_const_charp,
            );
        } else {
            return &raw mut (*png_ptr).time_buffer as *mut ::core::ffi::c_char as png_const_charp;
        }
    }
    return ::core::ptr::null::<::core::ffi::c_char>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_copyright(mut png_ptr: png_const_structrp) -> png_const_charp {
    return b"\nlibpng version 1.6.59.git\nCopyright (c) 2018-2026 Cosmin Truta\nCopyright (c) 1998-2002,2004,2006-2018 Glenn Randers-Pehrson\nCopyright (c) 1996-1997 Andreas Dilger\nCopyright (c) 1995-1996 Guy Eric Schalnat, Group 42, Inc.\n\0"
        as *const u8 as png_const_charp;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_libpng_ver(mut png_ptr: png_const_structrp) -> png_const_charp {
    return png_get_header_ver(png_ptr);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_ver(mut png_ptr: png_const_structrp) -> png_const_charp {
    return PNG_LIBPNG_VER_STRING.as_ptr();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_header_version(
    mut png_ptr: png_const_structrp,
) -> png_const_charp {
    return b" libpng version 1.6.59.git\n\n\0" as *const u8 as png_const_charp;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_grayscale_palette(
    mut bit_depth: ::core::ffi::c_int,
    mut palette: png_colorp,
) {
    let mut num_palette: ::core::ffi::c_int = 0;
    let mut color_inc: ::core::ffi::c_int = 0;
    let mut i: ::core::ffi::c_int = 0;
    let mut v: ::core::ffi::c_int = 0;
    if palette.is_null() {
        return;
    }
    match bit_depth {
        1 => {
            num_palette = 2 as ::core::ffi::c_int;
            color_inc = 0xff as ::core::ffi::c_int;
        }
        2 => {
            num_palette = 4 as ::core::ffi::c_int;
            color_inc = 0x55 as ::core::ffi::c_int;
        }
        4 => {
            num_palette = 16 as ::core::ffi::c_int;
            color_inc = 0x11 as ::core::ffi::c_int;
        }
        8 => {
            num_palette = 256 as ::core::ffi::c_int;
            color_inc = 1 as ::core::ffi::c_int;
        }
        _ => {
            num_palette = 0 as ::core::ffi::c_int;
            color_inc = 0 as ::core::ffi::c_int;
        }
    }
    i = 0 as ::core::ffi::c_int;
    v = 0 as ::core::ffi::c_int;
    while i < num_palette {
        (*palette.offset(i as isize)).red = (v & 0xff as ::core::ffi::c_int) as png_byte;
        (*palette.offset(i as isize)).green = (v & 0xff as ::core::ffi::c_int) as png_byte;
        (*palette.offset(i as isize)).blue = (v & 0xff as ::core::ffi::c_int) as png_byte;
        i += 1;
        v += color_inc;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_handle_as_unknown(
    mut png_ptr: png_const_structrp,
    mut chunk_name: png_const_bytep,
) -> ::core::ffi::c_int {
    let mut p: png_const_bytep = ::core::ptr::null::<png_byte>();
    let mut p_end: png_const_bytep = ::core::ptr::null::<png_byte>();
    if png_ptr.is_null()
        || chunk_name.is_null()
        || (*png_ptr).num_chunk_list == 0 as ::core::ffi::c_uint
    {
        return PNG_HANDLE_CHUNK_AS_DEFAULT;
    }
    p_end = (*png_ptr).chunk_list as png_const_bytep;
    p = p_end.offset(
        (*png_ptr)
            .num_chunk_list
            .wrapping_mul(5 as ::core::ffi::c_uint) as isize,
    );
    loop {
        p = p.offset(-(5 as ::core::ffi::c_int as isize));
        if memcmp(
            chunk_name as *const ::core::ffi::c_void,
            p as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
        {
            return *p.offset(4 as ::core::ffi::c_int as isize) as ::core::ffi::c_int;
        }
        if !(p > p_end) {
            break;
        }
    }
    return PNG_HANDLE_CHUNK_AS_DEFAULT;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_unknown_handling(
    mut png_ptr: png_const_structrp,
    mut chunk_name: png_uint_32,
) -> ::core::ffi::c_int {
    let mut chunk_string: [png_byte; 5] = [0; 5];
    *(&raw mut chunk_string as *mut png_byte as *mut ::core::ffi::c_char)
        .offset(0 as ::core::ffi::c_int as isize) =
        (chunk_name as ::core::ffi::c_uint >> 24 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
    *(&raw mut chunk_string as *mut png_byte as *mut ::core::ffi::c_char)
        .offset(1 as ::core::ffi::c_int as isize) =
        (chunk_name as ::core::ffi::c_uint >> 16 as ::core::ffi::c_int
            & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
    *(&raw mut chunk_string as *mut png_byte as *mut ::core::ffi::c_char)
        .offset(2 as ::core::ffi::c_int as isize) = (chunk_name as ::core::ffi::c_uint
        >> 8 as ::core::ffi::c_int
        & 0xff as ::core::ffi::c_uint)
        as ::core::ffi::c_char;
    *(&raw mut chunk_string as *mut png_byte as *mut ::core::ffi::c_char)
        .offset(3 as ::core::ffi::c_int as isize) =
        (chunk_name as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint) as ::core::ffi::c_char;
    *(&raw mut chunk_string as *mut png_byte as *mut ::core::ffi::c_char)
        .offset(4 as ::core::ffi::c_int as isize) = 0 as ::core::ffi::c_char;
    return png_handle_as_unknown(
        png_ptr,
        &raw mut chunk_string as *mut png_byte as png_const_bytep,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reset_zstream(mut png_ptr: png_structrp) -> ::core::ffi::c_int {
    if png_ptr.is_null() {
        return Z_STREAM_ERROR;
    }
    return inflateReset(&raw mut (*png_ptr).zstream);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_access_version_number() -> png_uint_32 {
    return PNG_LIBPNG_VER as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_zstream_error(mut png_ptr: png_structrp, mut ret: ::core::ffi::c_int) {
    if (*png_ptr).zstream.msg.is_null() {
        match ret {
            Z_STREAM_END => {
                (*png_ptr).zstream.msg =
                    b"unexpected end of LZ stream\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_NEED_DICT => {
                (*png_ptr).zstream.msg =
                    b"missing LZ dictionary\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_ERRNO => {
                (*png_ptr).zstream.msg =
                    b"zlib IO error\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_STREAM_ERROR => {
                (*png_ptr).zstream.msg =
                    b"bad parameters to zlib\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_DATA_ERROR => {
                (*png_ptr).zstream.msg =
                    b"damaged LZ stream\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_MEM_ERROR => {
                (*png_ptr).zstream.msg =
                    b"insufficient memory\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_BUF_ERROR => {
                (*png_ptr).zstream.msg = b"truncated\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_VERSION_ERROR => {
                (*png_ptr).zstream.msg =
                    b"unsupported zlib version\0" as *const u8 as *const ::core::ffi::c_char;
            }
            PNG_UNEXPECTED_ZLIB_RETURN => {
                (*png_ptr).zstream.msg =
                    b"unexpected zlib return\0" as *const u8 as *const ::core::ffi::c_char;
            }
            Z_OK | _ => {
                (*png_ptr).zstream.msg =
                    b"unexpected zlib return code\0" as *const u8 as *const ::core::ffi::c_char;
            }
        }
    }
}
unsafe extern "C" fn png_fp_add(
    mut addend0: png_int_32,
    mut addend1: png_int_32,
    mut error: *mut ::core::ffi::c_int,
) -> png_int_32 {
    if addend0 > 0 as ::core::ffi::c_int {
        if 0x7fffffff as png_int_32 - addend0 >= addend1 {
            return addend0 + addend1;
        }
    } else if addend0 < 0 as ::core::ffi::c_int {
        if -(0x7fffffff as png_int_32) - addend0 <= addend1 {
            return addend0 + addend1;
        }
    } else {
        return addend1;
    }
    *error = 1 as ::core::ffi::c_int;
    return PNG_FP_1 / 2 as png_int_32;
}
unsafe extern "C" fn png_fp_sub(
    mut addend0: png_int_32,
    mut addend1: png_int_32,
    mut error: *mut ::core::ffi::c_int,
) -> png_int_32 {
    if addend1 > 0 as ::core::ffi::c_int {
        if -(0x7fffffff as png_int_32) + addend1 <= addend0 {
            return addend0 - addend1;
        }
    } else if addend1 < 0 as ::core::ffi::c_int {
        if 0x7fffffff as png_int_32 + addend1 >= addend0 {
            return addend0 - addend1;
        }
    } else {
        return addend0;
    }
    *error = 1 as ::core::ffi::c_int;
    return PNG_FP_1 / 2 as png_int_32;
}
unsafe extern "C" fn png_safe_add(
    mut addend0_and_result: *mut png_int_32,
    mut addend1: png_int_32,
    mut addend2: png_int_32,
) -> ::core::ffi::c_int {
    let mut error: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut result: ::core::ffi::c_int = png_fp_add(
        *addend0_and_result,
        png_fp_add(addend1, addend2, &raw mut error),
        &raw mut error,
    ) as ::core::ffi::c_int;
    if error == 0 {
        *addend0_and_result = result as png_int_32;
    }
    return error;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_xy_from_XYZ(
    mut xy: *mut png_xy,
    mut XYZ: *const png_XYZ,
) -> ::core::ffi::c_int {
    let mut d: png_int_32 = 0;
    let mut dred: png_int_32 = 0;
    let mut dgreen: png_int_32 = 0;
    let mut dblue: png_int_32 = 0;
    let mut dwhite: png_int_32 = 0;
    let mut whiteX: png_int_32 = 0;
    let mut whiteY: png_int_32 = 0;
    d = (*XYZ).red_X as png_int_32;
    if png_safe_add(
        &raw mut d,
        (*XYZ).red_Y as png_int_32,
        (*XYZ).red_Z as png_int_32,
    ) != 0
    {
        return 1 as ::core::ffi::c_int;
    }
    dred = d;
    if png_muldiv(&raw mut (*xy).redx, (*XYZ).red_X, PNG_FP_1, dred) == 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(&raw mut (*xy).redy, (*XYZ).red_Y, PNG_FP_1, dred) == 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    d = (*XYZ).green_X as png_int_32;
    if png_safe_add(
        &raw mut d,
        (*XYZ).green_Y as png_int_32,
        (*XYZ).green_Z as png_int_32,
    ) != 0
    {
        return 1 as ::core::ffi::c_int;
    }
    dgreen = d;
    if png_muldiv(&raw mut (*xy).greenx, (*XYZ).green_X, PNG_FP_1, dgreen)
        == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(&raw mut (*xy).greeny, (*XYZ).green_Y, PNG_FP_1, dgreen)
        == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    d = (*XYZ).blue_X as png_int_32;
    if png_safe_add(
        &raw mut d,
        (*XYZ).blue_Y as png_int_32,
        (*XYZ).blue_Z as png_int_32,
    ) != 0
    {
        return 1 as ::core::ffi::c_int;
    }
    dblue = d;
    if png_muldiv(&raw mut (*xy).bluex, (*XYZ).blue_X, PNG_FP_1, dblue) == 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(&raw mut (*xy).bluey, (*XYZ).blue_Y, PNG_FP_1, dblue) == 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    d = dblue;
    if png_safe_add(&raw mut d, dred, dgreen) != 0 {
        return 1 as ::core::ffi::c_int;
    }
    dwhite = d;
    d = (*XYZ).red_X as png_int_32;
    if png_safe_add(
        &raw mut d,
        (*XYZ).green_X as png_int_32,
        (*XYZ).blue_X as png_int_32,
    ) != 0
    {
        return 1 as ::core::ffi::c_int;
    }
    whiteX = d;
    d = (*XYZ).red_Y as png_int_32;
    if png_safe_add(
        &raw mut d,
        (*XYZ).green_Y as png_int_32,
        (*XYZ).blue_Y as png_int_32,
    ) != 0
    {
        return 1 as ::core::ffi::c_int;
    }
    whiteY = d;
    if png_muldiv(
        &raw mut (*xy).whitex,
        whiteX as png_fixed_point,
        PNG_FP_1,
        dwhite,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*xy).whitey,
        whiteY as png_fixed_point,
        PNG_FP_1,
        dwhite,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_XYZ_from_xy(
    mut XYZ: *mut png_XYZ,
    mut xy: *const png_xy,
) -> ::core::ffi::c_int {
    let mut red_inverse: png_fixed_point = 0;
    let mut green_inverse: png_fixed_point = 0;
    let mut blue_scale: png_fixed_point = 0;
    let mut left: png_fixed_point = 0;
    let mut right: png_fixed_point = 0;
    let mut denominator: png_fixed_point = 0;
    let fpLimit: png_fixed_point = PNG_FP_1 + PNG_FP_1 / 10 as png_fixed_point;
    if (*xy).redx < 0 as ::core::ffi::c_int || (*xy).redx > fpLimit {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).redy < 0 as ::core::ffi::c_int || (*xy).redy > fpLimit - (*xy).redx {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).greenx < 0 as ::core::ffi::c_int || (*xy).greenx > fpLimit {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).greeny < 0 as ::core::ffi::c_int || (*xy).greeny > fpLimit - (*xy).greenx {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).bluex < 0 as ::core::ffi::c_int || (*xy).bluex > fpLimit {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).bluey < 0 as ::core::ffi::c_int || (*xy).bluey > fpLimit - (*xy).bluex {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).whitex < 0 as ::core::ffi::c_int || (*xy).whitex > fpLimit {
        return 1 as ::core::ffi::c_int;
    }
    if (*xy).whitey < 5 as ::core::ffi::c_int || (*xy).whitey > fpLimit - (*xy).whitex {
        return 1 as ::core::ffi::c_int;
    }
    let mut error: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if png_muldiv(
        &raw mut left,
        (*xy).greenx - (*xy).bluex,
        (*xy).redy as png_int_32 - (*xy).bluey as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut right,
        (*xy).greeny - (*xy).bluey,
        (*xy).redx as png_int_32 - (*xy).bluex as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    denominator =
        png_fp_sub(left as png_int_32, right as png_int_32, &raw mut error) as png_fixed_point;
    if error != 0 {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut left,
        (*xy).greenx - (*xy).bluex,
        (*xy).whitey as png_int_32 - (*xy).bluey as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut right,
        (*xy).greeny - (*xy).bluey,
        (*xy).whitex as png_int_32 - (*xy).bluex as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut red_inverse,
        (*xy).whitey,
        denominator as png_int_32,
        png_fp_sub(left as png_int_32, right as png_int_32, &raw mut error),
    ) == 0 as ::core::ffi::c_int
        || error != 0
        || red_inverse <= (*xy).whitey
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut left,
        (*xy).redy - (*xy).bluey,
        (*xy).whitex as png_int_32 - (*xy).bluex as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut right,
        (*xy).redx - (*xy).bluex,
        (*xy).whitey as png_int_32 - (*xy).bluey as png_int_32,
        8 as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut green_inverse,
        (*xy).whitey,
        denominator as png_int_32,
        png_fp_sub(left as png_int_32, right as png_int_32, &raw mut error),
    ) == 0 as ::core::ffi::c_int
        || error != 0
        || green_inverse <= (*xy).whitey
    {
        return 1 as ::core::ffi::c_int;
    }
    blue_scale = png_fp_sub(
        png_fp_sub(
            png_reciprocal((*xy).whitey) as png_int_32,
            png_reciprocal(red_inverse) as png_int_32,
            &raw mut error,
        ),
        png_reciprocal(green_inverse) as png_int_32,
        &raw mut error,
    ) as png_fixed_point;
    if error != 0 || blue_scale <= 0 as ::core::ffi::c_int {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).red_X,
        (*xy).redx,
        PNG_FP_1,
        red_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).red_Y,
        (*xy).redy,
        PNG_FP_1,
        red_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).red_Z,
        PNG_FP_1 - (*xy).redx - (*xy).redy,
        PNG_FP_1,
        red_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).green_X,
        (*xy).greenx,
        PNG_FP_1,
        green_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).green_Y,
        (*xy).greeny,
        PNG_FP_1,
        green_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).green_Z,
        PNG_FP_1 - (*xy).greenx - (*xy).greeny,
        PNG_FP_1,
        green_inverse as png_int_32,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).blue_X,
        (*xy).bluex,
        blue_scale as png_int_32,
        PNG_FP_1,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).blue_Y,
        (*xy).bluey,
        blue_scale as png_int_32,
        PNG_FP_1,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    if png_muldiv(
        &raw mut (*XYZ).blue_Z,
        PNG_FP_1 - (*xy).bluex - (*xy).bluey,
        blue_scale as png_int_32,
        PNG_FP_1,
    ) == 0 as ::core::ffi::c_int
    {
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_icc_tag_char(mut byte: png_uint_32) -> ::core::ffi::c_char {
    byte &= 0xff as ::core::ffi::c_uint;
    if byte >= 32 as ::core::ffi::c_uint && byte <= 126 as ::core::ffi::c_uint {
        return byte as ::core::ffi::c_char;
    } else {
        return '?' as i32 as ::core::ffi::c_char;
    };
}
unsafe extern "C" fn png_icc_tag_name(mut name: *mut ::core::ffi::c_char, mut tag: png_uint_32) {
    *name.offset(0 as ::core::ffi::c_int as isize) = '\'' as i32 as ::core::ffi::c_char;
    *name.offset(1 as ::core::ffi::c_int as isize) =
        png_icc_tag_char(tag >> 24 as ::core::ffi::c_int);
    *name.offset(2 as ::core::ffi::c_int as isize) =
        png_icc_tag_char(tag >> 16 as ::core::ffi::c_int);
    *name.offset(3 as ::core::ffi::c_int as isize) =
        png_icc_tag_char(tag >> 8 as ::core::ffi::c_int);
    *name.offset(4 as ::core::ffi::c_int as isize) = png_icc_tag_char(tag);
    *name.offset(5 as ::core::ffi::c_int as isize) = '\'' as i32 as ::core::ffi::c_char;
}
unsafe extern "C" fn is_ICC_signature_char(mut it: png_alloc_size_t) -> ::core::ffi::c_int {
    return (it == 32 as png_alloc_size_t
        || it >= 48 as png_alloc_size_t && it <= 57 as png_alloc_size_t
        || it >= 65 as png_alloc_size_t && it <= 90 as png_alloc_size_t
        || it >= 97 as png_alloc_size_t && it <= 122 as png_alloc_size_t)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn is_ICC_signature(mut it: png_alloc_size_t) -> ::core::ffi::c_int {
    return (is_ICC_signature_char(it >> 24 as ::core::ffi::c_int) != 0
        && is_ICC_signature_char(it >> 16 as ::core::ffi::c_int & 0xff as png_alloc_size_t) != 0
        && is_ICC_signature_char(it >> 8 as ::core::ffi::c_int & 0xff as png_alloc_size_t) != 0
        && is_ICC_signature_char(it & 0xff as png_alloc_size_t) != 0)
        as ::core::ffi::c_int;
}
unsafe extern "C" fn png_icc_profile_error(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
    mut value: png_alloc_size_t,
    mut reason: png_const_charp,
) -> ::core::ffi::c_int {
    let mut pos: size_t = 0;
    let mut message: [::core::ffi::c_char; 196] = [0; 196];
    pos = png_safecat(
        &raw mut message as png_charp,
        ::core::mem::size_of::<[::core::ffi::c_char; 196]>() as size_t,
        0 as size_t,
        b"profile '\0" as *const u8 as png_const_charp,
    );
    pos = png_safecat(
        &raw mut message as png_charp,
        pos.wrapping_add(79 as size_t),
        pos,
        name,
    );
    pos = png_safecat(
        &raw mut message as png_charp,
        ::core::mem::size_of::<[::core::ffi::c_char; 196]>() as size_t,
        pos,
        b"': \0" as *const u8 as png_const_charp,
    );
    if is_ICC_signature(value) != 0 as ::core::ffi::c_int {
        png_icc_tag_name(
            (&raw mut message as *mut ::core::ffi::c_char).offset(pos as isize),
            value as png_uint_32,
        );
        pos = (pos as ::core::ffi::c_ulong).wrapping_add(6 as ::core::ffi::c_ulong) as size_t
            as size_t;
        let fresh9 = pos;
        pos = pos.wrapping_add(1);
        message[fresh9 as usize] = ':' as i32 as ::core::ffi::c_char;
        let fresh10 = pos;
        pos = pos.wrapping_add(1);
        message[fresh10 as usize] = ' ' as i32 as ::core::ffi::c_char;
    } else {
        let mut number: [::core::ffi::c_char; 24] = [0; 24];
        pos = png_safecat(
            &raw mut message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 196]>() as size_t,
            pos,
            png_format_number(
                &raw mut number as *mut ::core::ffi::c_char as png_const_charp,
                (&raw mut number as *mut ::core::ffi::c_char)
                    .offset(::core::mem::size_of::<[::core::ffi::c_char; 24]>() as usize as isize),
                PNG_NUMBER_FORMAT_x,
                value,
            ) as png_const_charp,
        );
        pos = png_safecat(
            &raw mut message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 196]>() as size_t,
            pos,
            b"h: \0" as *const u8 as png_const_charp,
        );
    }
    pos = png_safecat(
        &raw mut message as png_charp,
        ::core::mem::size_of::<[::core::ffi::c_char; 196]>() as size_t,
        pos,
        reason,
    );
    png_chunk_benign_error(
        png_ptr,
        &raw mut message as *mut ::core::ffi::c_char as png_const_charp,
    );
    return 0 as ::core::ffi::c_int;
}
static mut D50_nCIEXYZ: [png_byte; 12] = [
    0 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0xf6 as ::core::ffi::c_int as png_byte,
    0xd6 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0x1 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0 as ::core::ffi::c_int as png_byte,
    0xd3 as ::core::ffi::c_int as png_byte,
    0x2d as ::core::ffi::c_int as png_byte,
];
unsafe extern "C" fn icc_check_length(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
    mut profile_length: png_uint_32,
) -> ::core::ffi::c_int {
    if profile_length < 132 as ::core::ffi::c_uint {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"too short\0" as *const u8 as png_const_charp,
        );
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_length(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
    mut profile_length: png_uint_32,
) -> ::core::ffi::c_int {
    if icc_check_length(png_ptr, name, profile_length) == 0 {
        return 0 as ::core::ffi::c_int;
    }
    if profile_length as png_alloc_size_t > (*png_ptr).user_chunk_malloc_max {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"profile too long\0" as *const u8 as png_const_charp,
        );
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_header(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
    mut profile_length: png_uint_32,
    mut profile: png_const_bytep,
    mut color_type: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut temp: png_uint_32 = 0;
    temp = ((*profile as png_uint_32) << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile.offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile.offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(*profile.offset(3 as ::core::ffi::c_int as isize) as png_uint_32);
    if temp != profile_length {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"length does not match profile\0" as *const u8 as png_const_charp,
        );
    }
    temp = *profile.offset(8 as ::core::ffi::c_int as isize) as png_uint_32;
    if temp > 3 as ::core::ffi::c_uint
        && profile_length as ::core::ffi::c_uint & 3 as ::core::ffi::c_uint != 0
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            profile_length as png_alloc_size_t,
            b"invalid length\0" as *const u8 as png_const_charp,
        );
    }
    temp = ((*profile.offset(128 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    if temp > 357913930 as ::core::ffi::c_int as ::core::ffi::c_uint
        || profile_length
            < (132 as png_uint_32).wrapping_add((12 as png_uint_32).wrapping_mul(temp))
    {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"tag count too large\0" as *const u8 as png_const_charp,
        );
    }
    temp = ((*profile.offset(64 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(64 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(64 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(64 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    if temp >= 0xffff as ::core::ffi::c_uint {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"invalid rendering intent\0" as *const u8 as png_const_charp,
        );
    }
    if temp >= PNG_sRGB_INTENT_LAST as ::core::ffi::c_uint {
        png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"intent outside defined range\0" as *const u8 as png_const_charp,
        );
    }
    temp = ((*profile.offset(36 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(36 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(36 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(36 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    if temp != 0x61637370 as ::core::ffi::c_int as ::core::ffi::c_uint {
        return png_icc_profile_error(
            png_ptr,
            name,
            temp as png_alloc_size_t,
            b"invalid signature\0" as *const u8 as png_const_charp,
        );
    }
    if memcmp(
        profile.offset(68 as ::core::ffi::c_int as isize) as *const ::core::ffi::c_void,
        &raw const D50_nCIEXYZ as *const png_byte as *const ::core::ffi::c_void,
        12 as size_t,
    ) != 0 as ::core::ffi::c_int
    {
        png_icc_profile_error(
            png_ptr,
            name,
            0 as png_alloc_size_t,
            b"PCS illuminant is not D50\0" as *const u8 as png_const_charp,
        );
    }
    temp = ((*profile.offset(16 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(16 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(16 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(16 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    match temp {
        1380401696 => {
            if color_type & PNG_COLOR_MASK_COLOR == 0 as ::core::ffi::c_int {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    b"RGB color space not permitted on grayscale PNG\0" as *const u8
                        as png_const_charp,
                );
            }
        }
        1196573017 => {
            if color_type & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int {
                return png_icc_profile_error(
                    png_ptr,
                    name,
                    temp as png_alloc_size_t,
                    b"Gray color space not permitted on RGB PNG\0" as *const u8 as png_const_charp,
                );
            }
        }
        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"invalid ICC profile color space\0" as *const u8 as png_const_charp,
            );
        }
    }
    temp = ((*profile.offset(12 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(12 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(12 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(12 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    match temp {
        1935896178 | 1835955314 | 1886549106 | 1936744803 => {}
        1633842036 => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"invalid embedded Abstract ICC profile\0" as *const u8 as png_const_charp,
            );
        }
        1818848875 => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected DeviceLink ICC profile class\0" as *const u8 as png_const_charp,
            );
        }
        1852662636 => {
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected NamedColor ICC profile class\0" as *const u8 as png_const_charp,
            );
        }
        _ => {
            png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unrecognized ICC profile class\0" as *const u8 as png_const_charp,
            );
        }
    }
    temp = ((*profile.offset(20 as ::core::ffi::c_int as isize) as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(20 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(20 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(20 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    match temp {
        1482250784 | 1281450528 => {}
        _ => {
            return png_icc_profile_error(
                png_ptr,
                name,
                temp as png_alloc_size_t,
                b"unexpected ICC PCS encoding\0" as *const u8 as png_const_charp,
            );
        }
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_icc_check_tag_table(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
    mut profile_length: png_uint_32,
    mut profile: png_const_bytep,
) -> ::core::ffi::c_int {
    let mut tag_count: png_uint_32 = ((*profile.offset(128 as ::core::ffi::c_int as isize)
        as png_uint_32)
        << 24 as ::core::ffi::c_int)
        .wrapping_add(
            (*profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                << 16 as ::core::ffi::c_int,
        )
        .wrapping_add(
            (*profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                << 8 as ::core::ffi::c_int,
        )
        .wrapping_add(
            *profile
                .offset(128 as ::core::ffi::c_int as isize)
                .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
        );
    let mut itag: png_uint_32 = 0;
    let mut tag: png_const_bytep = profile.offset(132 as ::core::ffi::c_int as isize);
    itag = 0 as png_uint_32;
    while itag < tag_count {
        let mut tag_id: png_uint_32 = ((*tag.offset(0 as ::core::ffi::c_int as isize)
            as png_uint_32)
            << 24 as ::core::ffi::c_int)
            .wrapping_add(
                (*tag
                    .offset(0 as ::core::ffi::c_int as isize)
                    .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*tag
                    .offset(0 as ::core::ffi::c_int as isize)
                    .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                *tag.offset(0 as ::core::ffi::c_int as isize)
                    .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
            );
        let mut tag_start: png_uint_32 = ((*tag.offset(4 as ::core::ffi::c_int as isize)
            as png_uint_32)
            << 24 as ::core::ffi::c_int)
            .wrapping_add(
                (*tag
                    .offset(4 as ::core::ffi::c_int as isize)
                    .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*tag
                    .offset(4 as ::core::ffi::c_int as isize)
                    .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                *tag.offset(4 as ::core::ffi::c_int as isize)
                    .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
            );
        let mut tag_length: png_uint_32 = ((*tag.offset(8 as ::core::ffi::c_int as isize)
            as png_uint_32)
            << 24 as ::core::ffi::c_int)
            .wrapping_add(
                (*tag
                    .offset(8 as ::core::ffi::c_int as isize)
                    .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*tag
                    .offset(8 as ::core::ffi::c_int as isize)
                    .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                *tag.offset(8 as ::core::ffi::c_int as isize)
                    .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
            );
        if tag_start > profile_length || tag_length > profile_length.wrapping_sub(tag_start) {
            return png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                b"ICC profile tag outside profile\0" as *const u8 as png_const_charp,
            );
        }
        if tag_start as ::core::ffi::c_uint & 3 as ::core::ffi::c_uint != 0 as ::core::ffi::c_uint {
            png_icc_profile_error(
                png_ptr,
                name,
                tag_id as png_alloc_size_t,
                b"ICC profile tag start not a multiple of 4\0" as *const u8 as png_const_charp,
            );
        }
        itag = itag.wrapping_add(1);
        tag = tag.offset(12 as ::core::ffi::c_int as isize);
    }
    return 1 as ::core::ffi::c_int;
}
unsafe extern "C" fn have_chromaticities(mut png_ptr: png_const_structrp) -> ::core::ffi::c_int {
    if (*png_ptr).chunks as ::core::ffi::c_uint
        & 0x80000000 as ::core::ffi::c_uint
            >> 31 as ::core::ffi::c_int - PNG_INDEX_mDCV as ::core::ffi::c_int
        != 0 as ::core::ffi::c_uint
    {
        return 1 as ::core::ffi::c_int;
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
        return 1 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rgb_coefficients(mut png_ptr: png_structrp) {
    if (*png_ptr).rgb_to_gray_coefficients_set as ::core::ffi::c_int == 0 as ::core::ffi::c_int {
        let mut xyz: png_XYZ = png_XYZ {
            red_X: 0,
            red_Y: 0,
            red_Z: 0,
            green_X: 0,
            green_Y: 0,
            green_Z: 0,
            blue_X: 0,
            blue_Y: 0,
            blue_Z: 0,
        };
        if have_chromaticities(png_ptr) != 0
            && png_XYZ_from_xy(&raw mut xyz, &raw mut (*png_ptr).chromaticities)
                == 0 as ::core::ffi::c_int
        {
            let mut r: png_fixed_point = xyz.red_Y;
            let mut g: png_fixed_point = xyz.green_Y;
            let mut b: png_fixed_point = xyz.blue_Y;
            let mut total: png_fixed_point = r + g + b;
            if total > 0 as ::core::ffi::c_int
                && r >= 0 as ::core::ffi::c_int
                && png_muldiv(&raw mut r, r, 32768 as png_int_32, total as png_int_32) != 0
                && r >= 0 as ::core::ffi::c_int
                && r <= 32768 as ::core::ffi::c_int
                && g >= 0 as ::core::ffi::c_int
                && png_muldiv(&raw mut g, g, 32768 as png_int_32, total as png_int_32) != 0
                && g >= 0 as ::core::ffi::c_int
                && g <= 32768 as ::core::ffi::c_int
                && b >= 0 as ::core::ffi::c_int
                && png_muldiv(&raw mut b, b, 32768 as png_int_32, total as png_int_32) != 0
                && b >= 0 as ::core::ffi::c_int
                && b <= 32768 as ::core::ffi::c_int
                && r + g + b <= 32769 as ::core::ffi::c_int
            {
                let mut add: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
                if r + g + b > 32768 as ::core::ffi::c_int {
                    add = -(1 as ::core::ffi::c_int);
                } else if r + g + b < 32768 as ::core::ffi::c_int {
                    add = 1 as ::core::ffi::c_int;
                }
                if add != 0 as ::core::ffi::c_int {
                    if g >= r && g >= b {
                        g += add;
                    } else if r >= g && r >= b {
                        r += add;
                    } else {
                        b += add;
                    }
                }
                if r + g + b != 32768 as ::core::ffi::c_int {
                    png_error(
                        png_ptr,
                        b"internal error handling cHRM coefficients\0" as *const u8
                            as png_const_charp,
                    );
                } else {
                    (*png_ptr).rgb_to_gray_red_coeff = r as png_uint_16;
                    (*png_ptr).rgb_to_gray_green_coeff = g as png_uint_16;
                }
            }
        } else {
            (*png_ptr).rgb_to_gray_red_coeff = 6968 as png_uint_16;
            (*png_ptr).rgb_to_gray_green_coeff = 23434 as png_uint_16;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_IHDR(
    mut png_ptr: png_const_structrp,
    mut width: png_uint_32,
    mut height: png_uint_32,
    mut bit_depth: ::core::ffi::c_int,
    mut color_type: ::core::ffi::c_int,
    mut interlace_type: ::core::ffi::c_int,
    mut compression_type: ::core::ffi::c_int,
    mut filter_type: ::core::ffi::c_int,
) {
    let mut error: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if width == 0 as ::core::ffi::c_uint {
        png_warning(
            png_ptr,
            b"Image width is zero in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if width > PNG_UINT_31_MAX {
        png_warning(
            png_ptr,
            b"Invalid image width in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if (width as ::core::ffi::c_uint).wrapping_add(7 as ::core::ffi::c_uint) as png_alloc_size_t
        & !(7 as ::core::ffi::c_int as png_alloc_size_t)
        > PNG_SIZE_MAX
            .wrapping_sub(48 as size_t)
            .wrapping_sub(1 as size_t)
            .wrapping_div(8 as size_t)
            .wrapping_sub(1 as size_t)
    {
        png_warning(
            png_ptr,
            b"Image width is too large for this architecture\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if width > (*png_ptr).user_width_max {
        png_warning(
            png_ptr,
            b"Image width exceeds user limit in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if height == 0 as ::core::ffi::c_uint {
        png_warning(
            png_ptr,
            b"Image height is zero in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if height > PNG_UINT_31_MAX {
        png_warning(
            png_ptr,
            b"Invalid image height in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if height > (*png_ptr).user_height_max {
        png_warning(
            png_ptr,
            b"Image height exceeds user limit in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if bit_depth != 1 as ::core::ffi::c_int
        && bit_depth != 2 as ::core::ffi::c_int
        && bit_depth != 4 as ::core::ffi::c_int
        && bit_depth != 8 as ::core::ffi::c_int
        && bit_depth != 16 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Invalid bit depth in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if color_type < 0 as ::core::ffi::c_int
        || color_type == 1 as ::core::ffi::c_int
        || color_type == 5 as ::core::ffi::c_int
        || color_type > 6 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Invalid color type in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if color_type == PNG_COLOR_TYPE_PALETTE && bit_depth > 8 as ::core::ffi::c_int
        || (color_type == PNG_COLOR_TYPE_RGB
            || color_type == PNG_COLOR_TYPE_GRAY_ALPHA
            || color_type == PNG_COLOR_TYPE_RGB_ALPHA)
            && bit_depth < 8 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Invalid color type/bit depth combination in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if interlace_type >= PNG_INTERLACE_LAST {
        png_warning(
            png_ptr,
            b"Unknown interlace method in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_warning(
            png_ptr,
            b"Unknown compression method in IHDR\0" as *const u8 as png_const_charp,
        );
        error = 1 as ::core::ffi::c_int;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PNG_SIGNATURE != 0 as ::core::ffi::c_uint
        && (*png_ptr).mng_features_permitted != 0 as ::core::ffi::c_uint
    {
        png_warning(
            png_ptr,
            b"MNG features are not allowed in a PNG datastream\0" as *const u8 as png_const_charp,
        );
    }
    if filter_type != PNG_FILTER_TYPE_BASE {
        if !((*png_ptr).mng_features_permitted as ::core::ffi::c_uint
            & PNG_FLAG_MNG_FILTER_64 as ::core::ffi::c_uint
            != 0 as ::core::ffi::c_uint
            && filter_type == PNG_INTRAPIXEL_DIFFERENCING
            && (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PNG_SIGNATURE
                == 0 as ::core::ffi::c_uint
            && (color_type == PNG_COLOR_TYPE_RGB || color_type == PNG_COLOR_TYPE_RGB_ALPHA))
        {
            png_warning(
                png_ptr,
                b"Unknown filter method in IHDR\0" as *const u8 as png_const_charp,
            );
            error = 1 as ::core::ffi::c_int;
        }
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_HAVE_PNG_SIGNATURE
            != 0 as ::core::ffi::c_uint
        {
            png_warning(
                png_ptr,
                b"Invalid filter method in IHDR\0" as *const u8 as png_const_charp,
            );
            error = 1 as ::core::ffi::c_int;
        }
    }
    if error == 1 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"Invalid IHDR data\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_number(
    mut string: png_const_charp,
    mut size: size_t,
    mut statep: *mut ::core::ffi::c_int,
    mut whereami: *mut size_t,
) -> ::core::ffi::c_int {
    let mut state: ::core::ffi::c_int = *statep;
    let mut i: size_t = *whereami;
    while i < size {
        let mut type_0: ::core::ffi::c_int = 0;
        match *string.offset(i as isize) as ::core::ffi::c_int {
            43 => {
                type_0 = PNG_FP_SAW_SIGN;
            }
            45 => {
                type_0 = PNG_FP_SAW_SIGN + PNG_FP_NEGATIVE;
            }
            46 => {
                type_0 = PNG_FP_SAW_DOT;
            }
            48 => {
                type_0 = PNG_FP_SAW_DIGIT;
            }
            49 | 50 | 51 | 52 | 53 | 54 | 55 | 56 | 57 => {
                type_0 = PNG_FP_SAW_DIGIT + PNG_FP_NONZERO;
            }
            69 | 101 => {
                type_0 = PNG_FP_SAW_E;
            }
            _ => {
                break;
            }
        }
        match (state & PNG_FP_STATE) + (type_0 & PNG_FP_SAW_ANY) {
            4 => {
                if state & PNG_FP_SAW_ANY != 0 as ::core::ffi::c_int {
                    break;
                }
                state |= type_0;
            }
            16 => {
                if state & PNG_FP_SAW_DOT != 0 as ::core::ffi::c_int {
                    break;
                }
                if state & PNG_FP_SAW_DIGIT != 0 as ::core::ffi::c_int {
                    state |= type_0;
                } else {
                    state = 1 as ::core::ffi::c_int | type_0 | state & PNG_FP_STICKY;
                }
            }
            8 => {
                if state & PNG_FP_SAW_DOT != 0 as ::core::ffi::c_int {
                    state =
                        1 as ::core::ffi::c_int | 16 as ::core::ffi::c_int | state & PNG_FP_STICKY;
                }
                state |= type_0 | 64 as ::core::ffi::c_int;
            }
            32 => {
                if state & PNG_FP_SAW_DIGIT == 0 as ::core::ffi::c_int {
                    break;
                }
                state = 2 as ::core::ffi::c_int | state & PNG_FP_STICKY;
            }
            9 => {
                state |= type_0 | 64 as ::core::ffi::c_int;
            }
            33 => {
                if state & PNG_FP_SAW_DIGIT == 0 as ::core::ffi::c_int {
                    break;
                }
                state = 2 as ::core::ffi::c_int | state & PNG_FP_STICKY;
            }
            6 => {
                if state & PNG_FP_SAW_ANY != 0 as ::core::ffi::c_int {
                    break;
                }
                state |= 4 as ::core::ffi::c_int;
            }
            10 => {
                state |= 8 as ::core::ffi::c_int | 64 as ::core::ffi::c_int;
            }
            _ => {
                break;
            }
        }
        i = i.wrapping_add(1);
    }
    *statep = state;
    *whereami = i;
    return (state & PNG_FP_SAW_DIGIT != 0 as ::core::ffi::c_int) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_fp_string(
    mut string: png_const_charp,
    mut size: size_t,
) -> ::core::ffi::c_int {
    let mut state: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut char_index: size_t = 0 as size_t;
    if png_check_fp_number(string, size, &raw mut state, &raw mut char_index)
        != 0 as ::core::ffi::c_int
        && (char_index == size
            || *string.offset(char_index as isize) as ::core::ffi::c_int == 0 as ::core::ffi::c_int)
    {
        return state;
    }
    return 0 as ::core::ffi::c_int;
}
unsafe extern "C" fn png_pow10(mut power: ::core::ffi::c_int) -> ::core::ffi::c_double {
    let mut recip: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut d: ::core::ffi::c_double = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
    if power < 0 as ::core::ffi::c_int {
        if power < DBL_MIN_10_EXP {
            return 0 as ::core::ffi::c_int as ::core::ffi::c_double;
        }
        recip = 1 as ::core::ffi::c_int;
        power = -power;
    }
    if power > 0 as ::core::ffi::c_int {
        let mut mult: ::core::ffi::c_double = 10 as ::core::ffi::c_int as ::core::ffi::c_double;
        loop {
            if power & 1 as ::core::ffi::c_int != 0 {
                d *= mult;
            }
            mult *= mult;
            power >>= 1 as ::core::ffi::c_int;
            if !(power > 0 as ::core::ffi::c_int) {
                break;
            }
        }
        if recip != 0 as ::core::ffi::c_int {
            d = 1 as ::core::ffi::c_int as ::core::ffi::c_double / d;
        }
    }
    return d;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fp(
    mut png_ptr: png_const_structrp,
    mut ascii: png_charp,
    mut size: size_t,
    mut fp: ::core::ffi::c_double,
    mut precision: ::core::ffi::c_uint,
) {
    if precision < 1 as ::core::ffi::c_uint {
        precision = DBL_DIG as ::core::ffi::c_uint;
    }
    if precision > (DBL_DIG + 1 as ::core::ffi::c_int) as ::core::ffi::c_uint {
        precision = (DBL_DIG + 1 as ::core::ffi::c_int) as ::core::ffi::c_uint;
    }
    if size >= precision.wrapping_add(5 as ::core::ffi::c_uint) as size_t {
        if fp < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
            fp = -fp;
            let fresh11 = ascii;
            ascii = ascii.offset(1);
            *fresh11 = 45 as ::core::ffi::c_char;
            size = size.wrapping_sub(1);
        }
        if fp >= DBL_MIN && fp <= DBL_MAX {
            let mut exp_b10: ::core::ffi::c_int = 0;
            let mut base: ::core::ffi::c_double = 0.;
            frexp(fp, &raw mut exp_b10);
            exp_b10 = exp_b10 * 77 as ::core::ffi::c_int >> 8 as ::core::ffi::c_int;
            base = png_pow10(exp_b10);
            while base < DBL_MIN || base < fp {
                let mut test: ::core::ffi::c_double = png_pow10(exp_b10 + 1 as ::core::ffi::c_int);
                if !(test <= DBL_MAX) {
                    break;
                }
                exp_b10 += 1;
                base = test;
            }
            fp /= base;
            while fp >= 1 as ::core::ffi::c_int as ::core::ffi::c_double {
                fp /= 10 as ::core::ffi::c_int as ::core::ffi::c_double;
                exp_b10 += 1;
            }
            let mut czero: ::core::ffi::c_uint = 0;
            let mut clead: ::core::ffi::c_uint = 0;
            let mut cdigits: ::core::ffi::c_uint = 0;
            let mut exponent: [::core::ffi::c_char; 10] = [0; 10];
            if exp_b10 < 0 as ::core::ffi::c_int && exp_b10 > -(3 as ::core::ffi::c_int) {
                czero = (0 as ::core::ffi::c_uint).wrapping_sub(exp_b10 as ::core::ffi::c_uint);
                exp_b10 = 0 as ::core::ffi::c_int;
            } else {
                czero = 0 as ::core::ffi::c_uint;
            }
            clead = czero;
            cdigits = 0 as ::core::ffi::c_uint;
            loop {
                let mut d: ::core::ffi::c_double = 0.;
                fp *= 10 as ::core::ffi::c_int as ::core::ffi::c_double;
                if cdigits
                    .wrapping_add(czero)
                    .wrapping_add(1 as ::core::ffi::c_uint)
                    < precision.wrapping_add(clead)
                {
                    fp = modf(fp, &raw mut d);
                } else {
                    d = floor(fp + 0.5f64);
                    if d > 9 as ::core::ffi::c_int as ::core::ffi::c_double {
                        if czero > 0 as ::core::ffi::c_uint {
                            czero = czero.wrapping_sub(1);
                            d = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
                            if cdigits == 0 as ::core::ffi::c_uint {
                                clead = clead.wrapping_sub(1);
                            }
                        } else {
                            while cdigits > 0 as ::core::ffi::c_uint
                                && d > 9 as ::core::ffi::c_int as ::core::ffi::c_double
                            {
                                ascii = ascii.offset(-1);
                                let mut ch: ::core::ffi::c_int = *ascii as ::core::ffi::c_int;
                                if exp_b10 != -(1 as ::core::ffi::c_int) {
                                    exp_b10 += 1;
                                } else if ch == 46 as ::core::ffi::c_int {
                                    ascii = ascii.offset(-1);
                                    ch = *ascii as ::core::ffi::c_int;
                                    size = size.wrapping_add(1);
                                    exp_b10 = 1 as ::core::ffi::c_int;
                                }
                                cdigits = cdigits.wrapping_sub(1);
                                d = (ch - 47 as ::core::ffi::c_int) as ::core::ffi::c_double;
                            }
                            if d > 9 as ::core::ffi::c_int as ::core::ffi::c_double {
                                if exp_b10 == -(1 as ::core::ffi::c_int) {
                                    ascii = ascii.offset(-1);
                                    let mut ch_0: ::core::ffi::c_int = *ascii as ::core::ffi::c_int;
                                    if ch_0 == 46 as ::core::ffi::c_int {
                                        size = size.wrapping_add(1);
                                        exp_b10 = 1 as ::core::ffi::c_int;
                                    }
                                } else {
                                    exp_b10 += 1;
                                }
                                d = 1 as ::core::ffi::c_int as ::core::ffi::c_double;
                            }
                        }
                    }
                    fp = 0 as ::core::ffi::c_int as ::core::ffi::c_double;
                }
                if d == 0 as ::core::ffi::c_int as ::core::ffi::c_double {
                    czero = czero.wrapping_add(1);
                    if cdigits == 0 as ::core::ffi::c_uint {
                        clead = clead.wrapping_add(1);
                    }
                } else {
                    cdigits = cdigits.wrapping_add(czero.wrapping_sub(clead));
                    clead = 0 as ::core::ffi::c_uint;
                    while czero > 0 as ::core::ffi::c_uint {
                        if exp_b10 != -(1 as ::core::ffi::c_int) {
                            if exp_b10 == 0 as ::core::ffi::c_int {
                                let fresh12 = ascii;
                                ascii = ascii.offset(1);
                                *fresh12 = 46 as ::core::ffi::c_char;
                                size = size.wrapping_sub(1);
                            }
                            exp_b10 -= 1;
                        }
                        let fresh13 = ascii;
                        ascii = ascii.offset(1);
                        *fresh13 = 48 as ::core::ffi::c_char;
                        czero = czero.wrapping_sub(1);
                    }
                    if exp_b10 != -(1 as ::core::ffi::c_int) {
                        if exp_b10 == 0 as ::core::ffi::c_int {
                            let fresh14 = ascii;
                            ascii = ascii.offset(1);
                            *fresh14 = 46 as ::core::ffi::c_char;
                            size = size.wrapping_sub(1);
                        }
                        exp_b10 -= 1;
                    }
                    let fresh15 = ascii;
                    ascii = ascii.offset(1);
                    *fresh15 =
                        (48 as ::core::ffi::c_int + d as ::core::ffi::c_int) as ::core::ffi::c_char;
                    cdigits = cdigits.wrapping_add(1);
                }
                if !(cdigits.wrapping_add(czero) < precision.wrapping_add(clead) && fp > DBL_MIN) {
                    break;
                }
            }
            if exp_b10 >= -(1 as ::core::ffi::c_int) && exp_b10 <= 2 as ::core::ffi::c_int {
                loop {
                    let fresh16 = exp_b10;
                    exp_b10 = exp_b10 - 1;
                    if !(fresh16 > 0 as ::core::ffi::c_int) {
                        break;
                    }
                    let fresh17 = ascii;
                    ascii = ascii.offset(1);
                    *fresh17 = 48 as ::core::ffi::c_char;
                }
                *ascii = 0 as ::core::ffi::c_char;
                return;
            }
            size = (size as ::core::ffi::c_ulong).wrapping_sub(cdigits as ::core::ffi::c_ulong)
                as size_t as size_t;
            let fresh18 = ascii;
            ascii = ascii.offset(1);
            *fresh18 = 69 as ::core::ffi::c_char;
            size = size.wrapping_sub(1);
            let mut uexp_b10: ::core::ffi::c_uint = 0;
            if exp_b10 < 0 as ::core::ffi::c_int {
                let fresh19 = ascii;
                ascii = ascii.offset(1);
                *fresh19 = 45 as ::core::ffi::c_char;
                size = size.wrapping_sub(1);
                uexp_b10 = (0 as ::core::ffi::c_uint).wrapping_sub(exp_b10 as ::core::ffi::c_uint);
            } else {
                uexp_b10 = (0 as ::core::ffi::c_uint).wrapping_add(exp_b10 as ::core::ffi::c_uint);
            }
            cdigits = 0 as ::core::ffi::c_uint;
            while uexp_b10 > 0 as ::core::ffi::c_uint {
                let fresh20 = cdigits;
                cdigits = cdigits.wrapping_add(1);
                exponent[fresh20 as usize] = (48 as ::core::ffi::c_uint)
                    .wrapping_add(uexp_b10.wrapping_rem(10 as ::core::ffi::c_uint))
                    as ::core::ffi::c_char;
                uexp_b10 = uexp_b10.wrapping_div(10 as ::core::ffi::c_uint);
            }
            if size > cdigits as size_t {
                while cdigits > 0 as ::core::ffi::c_uint {
                    cdigits = cdigits.wrapping_sub(1);
                    let fresh21 = ascii;
                    ascii = ascii.offset(1);
                    *fresh21 = exponent[cdigits as usize];
                }
                *ascii = 0 as ::core::ffi::c_char;
                return;
            }
        } else if !(fp >= DBL_MIN) {
            let fresh22 = ascii;
            ascii = ascii.offset(1);
            *fresh22 = 48 as ::core::ffi::c_char;
            *ascii = 0 as ::core::ffi::c_char;
            return;
        } else {
            let fresh23 = ascii;
            ascii = ascii.offset(1);
            *fresh23 = 105 as ::core::ffi::c_char;
            let fresh24 = ascii;
            ascii = ascii.offset(1);
            *fresh24 = 110 as ::core::ffi::c_char;
            let fresh25 = ascii;
            ascii = ascii.offset(1);
            *fresh25 = 102 as ::core::ffi::c_char;
            *ascii = 0 as ::core::ffi::c_char;
            return;
        }
    }
    png_error(
        png_ptr,
        b"ASCII conversion buffer too small\0" as *const u8 as png_const_charp,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_ascii_from_fixed(
    mut png_ptr: png_const_structrp,
    mut ascii: png_charp,
    mut size: size_t,
    mut fp: png_fixed_point,
) {
    if size > 12 as size_t {
        let mut num: png_uint_32 = 0;
        if fp < 0 as ::core::ffi::c_int {
            let fresh26 = ascii;
            ascii = ascii.offset(1);
            *fresh26 = 45 as ::core::ffi::c_char;
            num = -fp as png_uint_32;
        } else {
            num = fp as png_uint_32;
        }
        if num <= 0x80000000 as ::core::ffi::c_uint {
            let mut ndigits: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
            let mut first: ::core::ffi::c_uint = 16 as ::core::ffi::c_uint;
            let mut digits: [::core::ffi::c_char; 10] = [
                0 as ::core::ffi::c_int as ::core::ffi::c_char,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
                0,
            ];
            while num != 0 {
                let mut tmp: ::core::ffi::c_uint =
                    (num as ::core::ffi::c_uint).wrapping_div(10 as ::core::ffi::c_uint);
                num = (num as ::core::ffi::c_uint)
                    .wrapping_sub(tmp.wrapping_mul(10 as ::core::ffi::c_uint))
                    as png_uint_32 as png_uint_32;
                let fresh27 = ndigits;
                ndigits = ndigits.wrapping_add(1);
                digits[fresh27 as usize] =
                    (48 as png_uint_32).wrapping_add(num) as ::core::ffi::c_char;
                if first == 16 as ::core::ffi::c_uint && num > 0 as ::core::ffi::c_uint {
                    first = ndigits;
                }
                num = tmp as png_uint_32;
            }
            if ndigits > 0 as ::core::ffi::c_uint {
                while ndigits > 5 as ::core::ffi::c_uint {
                    ndigits = ndigits.wrapping_sub(1);
                    let fresh28 = ascii;
                    ascii = ascii.offset(1);
                    *fresh28 = digits[ndigits as usize];
                }
                if first <= 5 as ::core::ffi::c_uint {
                    let mut i: ::core::ffi::c_uint = 0;
                    let fresh29 = ascii;
                    ascii = ascii.offset(1);
                    *fresh29 = 46 as ::core::ffi::c_char;
                    i = 5 as ::core::ffi::c_uint;
                    while ndigits < i {
                        let fresh30 = ascii;
                        ascii = ascii.offset(1);
                        *fresh30 = 48 as ::core::ffi::c_char;
                        i = i.wrapping_sub(1);
                    }
                    while ndigits >= first {
                        ndigits = ndigits.wrapping_sub(1);
                        let fresh31 = ascii;
                        ascii = ascii.offset(1);
                        *fresh31 = digits[ndigits as usize];
                    }
                }
            } else {
                let fresh32 = ascii;
                ascii = ascii.offset(1);
                *fresh32 = 48 as ::core::ffi::c_char;
            }
            *ascii = 0 as ::core::ffi::c_char;
            return;
        }
    }
    png_error(
        png_ptr,
        b"ASCII conversion buffer too small\0" as *const u8 as png_const_charp,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed(
    mut png_ptr: png_const_structrp,
    mut fp: ::core::ffi::c_double,
    mut text: png_const_charp,
) -> png_fixed_point {
    let mut r: ::core::ffi::c_double =
        floor(100000 as ::core::ffi::c_int as ::core::ffi::c_double * fp + 0.5f64);
    if r > 2147483647.0f64 || r < -2147483648.0f64 {
        png_fixed_error(png_ptr, text);
    }
    return r as png_fixed_point;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_ITU(
    mut png_ptr: png_const_structrp,
    mut fp: ::core::ffi::c_double,
    mut text: png_const_charp,
) -> png_uint_32 {
    let mut r: ::core::ffi::c_double =
        floor(10000 as ::core::ffi::c_int as ::core::ffi::c_double * fp + 0.5f64);
    if r > 2147483647.0f64 || r < 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        png_fixed_error(png_ptr, text);
    }
    return r as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_muldiv(
    mut res: png_fixed_point_p,
    mut a: png_fixed_point,
    mut times: png_int_32,
    mut divisor: png_int_32,
) -> ::core::ffi::c_int {
    if divisor != 0 as ::core::ffi::c_int {
        if a == 0 as ::core::ffi::c_int || times == 0 as ::core::ffi::c_int {
            *res = 0 as ::core::ffi::c_int as png_fixed_point;
            return 1 as ::core::ffi::c_int;
        } else {
            let mut r: ::core::ffi::c_double = a as ::core::ffi::c_double;
            r *= times as ::core::ffi::c_double;
            r /= divisor as ::core::ffi::c_double;
            r = floor(r + 0.5f64);
            if r <= 2147483647.0f64 && r >= -2147483648.0f64 {
                *res = r as png_fixed_point;
                return 1 as ::core::ffi::c_int;
            }
        }
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal(mut a: png_fixed_point) -> png_fixed_point {
    let mut r: ::core::ffi::c_double = floor(1E10f64 / a as ::core::ffi::c_double + 0.5f64);
    if r <= 2147483647.0f64 && r >= -2147483648.0f64 {
        return r as png_fixed_point;
    }
    return 0 as png_fixed_point;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_significant(
    mut gamma_val: png_fixed_point,
) -> ::core::ffi::c_int {
    return (gamma_val < PNG_FP_1 - PNG_GAMMA_THRESHOLD_FIXED
        || gamma_val > PNG_FP_1 + PNG_GAMMA_THRESHOLD_FIXED) as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_reciprocal2(
    mut a: png_fixed_point,
    mut b: png_fixed_point,
) -> png_fixed_point {
    if a != 0 as ::core::ffi::c_int && b != 0 as ::core::ffi::c_int {
        let mut r: ::core::ffi::c_double = 1E15f64 / a as ::core::ffi::c_double;
        r /= b as ::core::ffi::c_double;
        r = floor(r + 0.5f64);
        if r <= 2147483647.0f64 && r >= -2147483648.0f64 {
            return r as png_fixed_point;
        }
    }
    return 0 as png_fixed_point;
}
#[inline]
fn png_c_double_to_i32(value: ::core::ffi::c_double) -> ::core::ffi::c_int {
    if !value.is_finite()
        || value >= 2147483648.0f64
        || value < -2147483648.0f64
    {
        ::core::ffi::c_int::MIN
    } else {
        value as ::core::ffi::c_int
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_8bit_correct(
    mut value: ::core::ffi::c_uint,
    mut gamma_val: png_fixed_point,
) -> png_byte {
    if value > 0 as ::core::ffi::c_uint && value < 255 as ::core::ffi::c_uint {
        let mut r: ::core::ffi::c_double = floor(
            255 as ::core::ffi::c_int as ::core::ffi::c_double
                * pow(
                    value as ::core::ffi::c_int as ::core::ffi::c_double / 255.0f64,
                    gamma_val as ::core::ffi::c_double * 0.00001f64,
                )
                + 0.5f64,
        );
        return png_c_double_to_i32(r) as png_byte;
    }
    return (value & 0xff as ::core::ffi::c_uint) as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_16bit_correct(
    mut value: ::core::ffi::c_uint,
    mut gamma_val: png_fixed_point,
) -> png_uint_16 {
    if value > 0 as ::core::ffi::c_uint && value < 65535 as ::core::ffi::c_uint {
        let mut r: ::core::ffi::c_double = floor(
            65535 as ::core::ffi::c_int as ::core::ffi::c_double
                * pow(
                    value as png_int_32 as ::core::ffi::c_double / 65535.0f64,
                    gamma_val as ::core::ffi::c_double * 0.00001f64,
                )
                + 0.5f64,
        );
        return png_c_double_to_i32(r) as png_uint_16;
    }
    return value as png_uint_16;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_gamma_correct(
    mut png_ptr: png_structrp,
    mut value: ::core::ffi::c_uint,
    mut gamma_val: png_fixed_point,
) -> png_uint_16 {
    if (*png_ptr).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
        return png_gamma_8bit_correct(value, gamma_val) as png_uint_16;
    } else {
        return png_gamma_16bit_correct(value, gamma_val);
    };
}
unsafe extern "C" fn png_build_16bit_table(
    mut png_ptr: png_structrp,
    mut ptable: *mut png_uint_16pp,
    mut shift: ::core::ffi::c_uint,
    mut gamma_val: png_fixed_point,
) {
    let mut num: ::core::ffi::c_uint =
        (1 as ::core::ffi::c_uint) << (8 as ::core::ffi::c_uint).wrapping_sub(shift);
    let mut fmax: ::core::ffi::c_double = 1.0f64
        / (((1 as ::core::ffi::c_int) << (16 as ::core::ffi::c_uint).wrapping_sub(shift))
            - 1 as ::core::ffi::c_int) as ::core::ffi::c_double;
    let mut max: ::core::ffi::c_uint = ((1 as ::core::ffi::c_uint)
        << (16 as ::core::ffi::c_uint).wrapping_sub(shift))
    .wrapping_sub(1 as ::core::ffi::c_uint);
    let mut max_by_2: ::core::ffi::c_uint =
        (1 as ::core::ffi::c_uint) << (15 as ::core::ffi::c_uint).wrapping_sub(shift);
    let mut i: ::core::ffi::c_uint = 0;
    *ptable = png_calloc(
        png_ptr,
        (num as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<png_uint_16p>() as png_alloc_size_t),
    ) as png_uint_16pp;
    let mut table: png_uint_16pp = *ptable;
    i = 0 as ::core::ffi::c_uint;
    while i < num {
        let ref mut fresh33 = *table.offset(i as isize);
        *fresh33 = png_malloc(
            png_ptr,
            (256 as png_alloc_size_t)
                .wrapping_mul(::core::mem::size_of::<png_uint_16>() as png_alloc_size_t),
        ) as png_uint_16p as *mut png_uint_16;
        let mut sub_table: png_uint_16p = *fresh33;
        if png_gamma_significant(gamma_val) != 0 as ::core::ffi::c_int {
            let mut j: ::core::ffi::c_uint = 0;
            j = 0 as ::core::ffi::c_uint;
            while j < 256 as ::core::ffi::c_uint {
                let mut ig: png_uint_32 = ((j as png_uint_32)
                    << (8 as ::core::ffi::c_uint).wrapping_sub(shift))
                .wrapping_add(i as png_uint_32);
                let mut d: ::core::ffi::c_double = floor(
                    65535.0f64
                        * pow(
                            ig as ::core::ffi::c_double * fmax,
                            gamma_val as ::core::ffi::c_double * 0.00001f64,
                        )
                        + 0.5f64,
                );
                *sub_table.offset(j as isize) = d as png_uint_16;
                j = j.wrapping_add(1);
            }
        } else {
            let mut j_0: ::core::ffi::c_uint = 0;
            j_0 = 0 as ::core::ffi::c_uint;
            while j_0 < 256 as ::core::ffi::c_uint {
                let mut ig_0: png_uint_32 = ((j_0 as png_uint_32)
                    << (8 as ::core::ffi::c_uint).wrapping_sub(shift))
                .wrapping_add(i as png_uint_32);
                if shift != 0 as ::core::ffi::c_uint {
                    ig_0 = (ig_0 as ::core::ffi::c_uint)
                        .wrapping_mul(65535 as ::core::ffi::c_uint)
                        .wrapping_add(max_by_2)
                        .wrapping_div(max) as png_uint_32;
                }
                *sub_table.offset(j_0 as isize) = ig_0 as png_uint_16;
                j_0 = j_0.wrapping_add(1);
            }
        }
        i = i.wrapping_add(1);
    }
}
unsafe extern "C" fn png_build_16to8_table(
    mut png_ptr: png_structrp,
    mut ptable: *mut png_uint_16pp,
    mut shift: ::core::ffi::c_uint,
    mut gamma_val: png_fixed_point,
) {
    let mut num: ::core::ffi::c_uint =
        (1 as ::core::ffi::c_uint) << (8 as ::core::ffi::c_uint).wrapping_sub(shift);
    let mut max: ::core::ffi::c_uint = ((1 as ::core::ffi::c_uint)
        << (16 as ::core::ffi::c_uint).wrapping_sub(shift))
    .wrapping_sub(1 as ::core::ffi::c_uint);
    let mut i: ::core::ffi::c_uint = 0;
    let mut last: png_uint_32 = 0;
    *ptable = png_calloc(
        png_ptr,
        (num as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<png_uint_16p>() as png_alloc_size_t),
    ) as png_uint_16pp;
    let mut table: png_uint_16pp = *ptable;
    i = 0 as ::core::ffi::c_uint;
    while i < num {
        let ref mut fresh34 = *table.offset(i as isize);
        *fresh34 = png_malloc(
            png_ptr,
            (256 as png_alloc_size_t)
                .wrapping_mul(::core::mem::size_of::<png_uint_16>() as png_alloc_size_t),
        ) as png_uint_16p as *mut png_uint_16;
        i = i.wrapping_add(1);
    }
    last = 0 as png_uint_32;
    i = 0 as ::core::ffi::c_uint;
    while i < 255 as ::core::ffi::c_uint {
        let mut out: png_uint_16 = i.wrapping_mul(257 as ::core::ffi::c_uint) as png_uint_16;
        let mut bound: png_uint_32 = png_gamma_16bit_correct(
            (out as ::core::ffi::c_uint).wrapping_add(128 as ::core::ffi::c_uint),
            gamma_val,
        ) as png_uint_32;
        bound = (bound as ::core::ffi::c_uint)
            .wrapping_mul(max)
            .wrapping_add(32768 as ::core::ffi::c_uint)
            .wrapping_div(65535 as ::core::ffi::c_uint)
            .wrapping_add(1 as ::core::ffi::c_uint) as png_uint_32;
        while last < bound {
            *(*table.offset(
                (last as ::core::ffi::c_uint & 0xff as ::core::ffi::c_uint >> shift) as isize,
            ))
            .offset((last >> (8 as ::core::ffi::c_uint).wrapping_sub(shift)) as isize) = out;
            last = last.wrapping_add(1);
        }
        i = i.wrapping_add(1);
    }
    while last < num << 8 as ::core::ffi::c_int {
        *(*table.offset(
            (last as ::core::ffi::c_uint
                & (0xff as ::core::ffi::c_int >> shift) as ::core::ffi::c_uint)
                as isize,
        ))
        .offset((last >> (8 as ::core::ffi::c_uint).wrapping_sub(shift)) as isize) =
            65535 as png_uint_16;
        last = last.wrapping_add(1);
    }
}
unsafe extern "C" fn png_build_8bit_table(
    mut png_ptr: png_structrp,
    mut ptable: png_bytepp,
    mut gamma_val: png_fixed_point,
) {
    let mut i: ::core::ffi::c_uint = 0;
    *ptable = png_malloc(png_ptr, 256 as png_alloc_size_t) as png_bytep as *mut png_byte;
    let mut table: png_bytep = *ptable;
    if png_gamma_significant(gamma_val) != 0 as ::core::ffi::c_int {
        i = 0 as ::core::ffi::c_uint;
        while i < 256 as ::core::ffi::c_uint {
            *table.offset(i as isize) = png_gamma_8bit_correct(i, gamma_val);
            i = i.wrapping_add(1);
        }
    } else {
        i = 0 as ::core::ffi::c_uint;
        while i < 256 as ::core::ffi::c_uint {
            *table.offset(i as isize) = (i & 0xff as ::core::ffi::c_uint) as png_byte;
            i = i.wrapping_add(1);
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_destroy_gamma_table(mut png_ptr: png_structrp) {
    png_free(png_ptr, (*png_ptr).gamma_table as png_voidp);
    (*png_ptr).gamma_table = ::core::ptr::null_mut::<png_byte>();
    if !(*png_ptr).gamma_16_table.is_null() {
        let mut i: ::core::ffi::c_int = 0;
        let mut istop: ::core::ffi::c_int =
            (1 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int - (*png_ptr).gamma_shift;
        i = 0 as ::core::ffi::c_int;
        while i < istop {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_table.offset(i as isize) as png_voidp,
            );
            i += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_table as png_voidp);
        (*png_ptr).gamma_16_table = ::core::ptr::null_mut::<*mut png_uint_16>();
    }
    png_free(png_ptr, (*png_ptr).gamma_from_1 as png_voidp);
    (*png_ptr).gamma_from_1 = ::core::ptr::null_mut::<png_byte>();
    png_free(png_ptr, (*png_ptr).gamma_to_1 as png_voidp);
    (*png_ptr).gamma_to_1 = ::core::ptr::null_mut::<png_byte>();
    if !(*png_ptr).gamma_16_from_1.is_null() {
        let mut i_0: ::core::ffi::c_int = 0;
        let mut istop_0: ::core::ffi::c_int =
            (1 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int - (*png_ptr).gamma_shift;
        i_0 = 0 as ::core::ffi::c_int;
        while i_0 < istop_0 {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_from_1.offset(i_0 as isize) as png_voidp,
            );
            i_0 += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_from_1 as png_voidp);
        (*png_ptr).gamma_16_from_1 = ::core::ptr::null_mut::<*mut png_uint_16>();
    }
    if !(*png_ptr).gamma_16_to_1.is_null() {
        let mut i_1: ::core::ffi::c_int = 0;
        let mut istop_1: ::core::ffi::c_int =
            (1 as ::core::ffi::c_int) << 8 as ::core::ffi::c_int - (*png_ptr).gamma_shift;
        i_1 = 0 as ::core::ffi::c_int;
        while i_1 < istop_1 {
            png_free(
                png_ptr,
                *(*png_ptr).gamma_16_to_1.offset(i_1 as isize) as png_voidp,
            );
            i_1 += 1;
        }
        png_free(png_ptr, (*png_ptr).gamma_16_to_1 as png_voidp);
        (*png_ptr).gamma_16_to_1 = ::core::ptr::null_mut::<*mut png_uint_16>();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_build_gamma_table(
    mut png_ptr: png_structrp,
    mut bit_depth: ::core::ffi::c_int,
) {
    let mut file_gamma: png_fixed_point = 0;
    let mut screen_gamma: png_fixed_point = 0;
    let mut correction: png_fixed_point = 0;
    let mut file_to_linear: png_fixed_point = 0;
    let mut linear_to_screen: png_fixed_point = 0;
    if !(*png_ptr).gamma_table.is_null() || !(*png_ptr).gamma_16_table.is_null() {
        png_warning(
            png_ptr,
            b"gamma table being rebuilt\0" as *const u8 as png_const_charp,
        );
        png_destroy_gamma_table(png_ptr);
    }
    file_gamma = (*png_ptr).file_gamma;
    screen_gamma = (*png_ptr).screen_gamma;
    file_to_linear = png_reciprocal(file_gamma);
    if screen_gamma > 0 as ::core::ffi::c_int {
        linear_to_screen = png_reciprocal(screen_gamma);
        correction = png_reciprocal2(screen_gamma, file_gamma);
    } else {
        linear_to_screen = file_gamma;
        correction = PNG_FP_1 as png_fixed_point;
    }
    if bit_depth <= 8 as ::core::ffi::c_int {
        png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_table, correction);
        if (*png_ptr).transformations as ::core::ffi::c_uint & (PNG_COMPOSE | PNG_RGB_TO_GRAY)
            != 0 as ::core::ffi::c_uint
        {
            png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_to_1, file_to_linear);
            png_build_8bit_table(png_ptr, &raw mut (*png_ptr).gamma_from_1, linear_to_screen);
        }
    } else {
        let mut shift: png_byte = 0;
        let mut sig_bit: png_byte = 0;
        if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            != 0 as ::core::ffi::c_int
        {
            sig_bit = (*png_ptr).sig_bit.red;
            if (*png_ptr).sig_bit.green as ::core::ffi::c_int > sig_bit as ::core::ffi::c_int {
                sig_bit = (*png_ptr).sig_bit.green;
            }
            if (*png_ptr).sig_bit.blue as ::core::ffi::c_int > sig_bit as ::core::ffi::c_int {
                sig_bit = (*png_ptr).sig_bit.blue;
            }
        } else {
            sig_bit = (*png_ptr).sig_bit.gray;
        }
        if sig_bit as ::core::ffi::c_int > 0 as ::core::ffi::c_int
            && (sig_bit as ::core::ffi::c_uint) < 16 as ::core::ffi::c_uint
        {
            shift = ((16 as ::core::ffi::c_uint).wrapping_sub(sig_bit as ::core::ffi::c_uint)
                & 0xff as ::core::ffi::c_uint) as png_byte;
        } else {
            shift = 0 as png_byte;
        }
        if (*png_ptr).transformations as ::core::ffi::c_uint & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)
            != 0 as ::core::ffi::c_uint
        {
            if (shift as ::core::ffi::c_uint)
                < (16 as ::core::ffi::c_uint).wrapping_sub(PNG_MAX_GAMMA_8 as ::core::ffi::c_uint)
            {
                shift = (16 as ::core::ffi::c_uint)
                    .wrapping_sub(PNG_MAX_GAMMA_8 as ::core::ffi::c_uint)
                    as png_byte;
            }
        }
        if shift as ::core::ffi::c_uint > 8 as ::core::ffi::c_uint {
            shift = 8 as png_byte;
        }
        (*png_ptr).gamma_shift = shift as ::core::ffi::c_int;
        if (*png_ptr).transformations as ::core::ffi::c_uint & (PNG_16_TO_8 | PNG_SCALE_16_TO_8)
            != 0 as ::core::ffi::c_uint
        {
            png_build_16to8_table(
                png_ptr,
                &raw mut (*png_ptr).gamma_16_table,
                shift as ::core::ffi::c_uint,
                png_reciprocal(correction),
            );
        } else {
            png_build_16bit_table(
                png_ptr,
                &raw mut (*png_ptr).gamma_16_table,
                shift as ::core::ffi::c_uint,
                correction,
            );
        }
        if (*png_ptr).transformations as ::core::ffi::c_uint & (PNG_COMPOSE | PNG_RGB_TO_GRAY)
            != 0 as ::core::ffi::c_uint
        {
            png_build_16bit_table(
                png_ptr,
                &raw mut (*png_ptr).gamma_16_to_1,
                shift as ::core::ffi::c_uint,
                file_to_linear,
            );
            png_build_16bit_table(
                png_ptr,
                &raw mut (*png_ptr).gamma_16_from_1,
                shift as ::core::ffi::c_uint,
                linear_to_screen,
            );
        }
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_option(
    mut png_ptr: png_structrp,
    mut option: ::core::ffi::c_int,
    mut onoff: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null()
        && option >= 0 as ::core::ffi::c_int
        && option < PNG_OPTION_NEXT
        && option & 1 as ::core::ffi::c_int == 0 as ::core::ffi::c_int
    {
        let mut mask: png_uint_32 = (3 as png_uint_32) << option;
        let mut setting: png_uint_32 = (2 as png_uint_32)
            .wrapping_add((onoff != 0 as ::core::ffi::c_int) as ::core::ffi::c_int as png_uint_32)
            << option;
        let mut current: png_uint_32 = (*png_ptr).options;
        (*png_ptr).options = current & !mask | setting;
        return (current & mask) as ::core::ffi::c_int >> option;
    }
    return PNG_OPTION_INVALID;
}
#[unsafe(no_mangle)]
pub static png_sRGB_table: [png_uint_16; 256] = [
    0 as ::core::ffi::c_int as png_uint_16,
    20 as ::core::ffi::c_int as png_uint_16,
    40 as ::core::ffi::c_int as png_uint_16,
    60 as ::core::ffi::c_int as png_uint_16,
    80 as ::core::ffi::c_int as png_uint_16,
    99 as ::core::ffi::c_int as png_uint_16,
    119 as ::core::ffi::c_int as png_uint_16,
    139 as ::core::ffi::c_int as png_uint_16,
    159 as ::core::ffi::c_int as png_uint_16,
    179 as ::core::ffi::c_int as png_uint_16,
    199 as ::core::ffi::c_int as png_uint_16,
    219 as ::core::ffi::c_int as png_uint_16,
    241 as ::core::ffi::c_int as png_uint_16,
    264 as ::core::ffi::c_int as png_uint_16,
    288 as ::core::ffi::c_int as png_uint_16,
    313 as ::core::ffi::c_int as png_uint_16,
    340 as ::core::ffi::c_int as png_uint_16,
    367 as ::core::ffi::c_int as png_uint_16,
    396 as ::core::ffi::c_int as png_uint_16,
    427 as ::core::ffi::c_int as png_uint_16,
    458 as ::core::ffi::c_int as png_uint_16,
    491 as ::core::ffi::c_int as png_uint_16,
    526 as ::core::ffi::c_int as png_uint_16,
    562 as ::core::ffi::c_int as png_uint_16,
    599 as ::core::ffi::c_int as png_uint_16,
    637 as ::core::ffi::c_int as png_uint_16,
    677 as ::core::ffi::c_int as png_uint_16,
    718 as ::core::ffi::c_int as png_uint_16,
    761 as ::core::ffi::c_int as png_uint_16,
    805 as ::core::ffi::c_int as png_uint_16,
    851 as ::core::ffi::c_int as png_uint_16,
    898 as ::core::ffi::c_int as png_uint_16,
    947 as ::core::ffi::c_int as png_uint_16,
    997 as ::core::ffi::c_int as png_uint_16,
    1048 as ::core::ffi::c_int as png_uint_16,
    1101 as ::core::ffi::c_int as png_uint_16,
    1156 as ::core::ffi::c_int as png_uint_16,
    1212 as ::core::ffi::c_int as png_uint_16,
    1270 as ::core::ffi::c_int as png_uint_16,
    1330 as ::core::ffi::c_int as png_uint_16,
    1391 as ::core::ffi::c_int as png_uint_16,
    1453 as ::core::ffi::c_int as png_uint_16,
    1517 as ::core::ffi::c_int as png_uint_16,
    1583 as ::core::ffi::c_int as png_uint_16,
    1651 as ::core::ffi::c_int as png_uint_16,
    1720 as ::core::ffi::c_int as png_uint_16,
    1790 as ::core::ffi::c_int as png_uint_16,
    1863 as ::core::ffi::c_int as png_uint_16,
    1937 as ::core::ffi::c_int as png_uint_16,
    2013 as ::core::ffi::c_int as png_uint_16,
    2090 as ::core::ffi::c_int as png_uint_16,
    2170 as ::core::ffi::c_int as png_uint_16,
    2250 as ::core::ffi::c_int as png_uint_16,
    2333 as ::core::ffi::c_int as png_uint_16,
    2418 as ::core::ffi::c_int as png_uint_16,
    2504 as ::core::ffi::c_int as png_uint_16,
    2592 as ::core::ffi::c_int as png_uint_16,
    2681 as ::core::ffi::c_int as png_uint_16,
    2773 as ::core::ffi::c_int as png_uint_16,
    2866 as ::core::ffi::c_int as png_uint_16,
    2961 as ::core::ffi::c_int as png_uint_16,
    3058 as ::core::ffi::c_int as png_uint_16,
    3157 as ::core::ffi::c_int as png_uint_16,
    3258 as ::core::ffi::c_int as png_uint_16,
    3360 as ::core::ffi::c_int as png_uint_16,
    3464 as ::core::ffi::c_int as png_uint_16,
    3570 as ::core::ffi::c_int as png_uint_16,
    3678 as ::core::ffi::c_int as png_uint_16,
    3788 as ::core::ffi::c_int as png_uint_16,
    3900 as ::core::ffi::c_int as png_uint_16,
    4014 as ::core::ffi::c_int as png_uint_16,
    4129 as ::core::ffi::c_int as png_uint_16,
    4247 as ::core::ffi::c_int as png_uint_16,
    4366 as ::core::ffi::c_int as png_uint_16,
    4488 as ::core::ffi::c_int as png_uint_16,
    4611 as ::core::ffi::c_int as png_uint_16,
    4736 as ::core::ffi::c_int as png_uint_16,
    4864 as ::core::ffi::c_int as png_uint_16,
    4993 as ::core::ffi::c_int as png_uint_16,
    5124 as ::core::ffi::c_int as png_uint_16,
    5257 as ::core::ffi::c_int as png_uint_16,
    5392 as ::core::ffi::c_int as png_uint_16,
    5530 as ::core::ffi::c_int as png_uint_16,
    5669 as ::core::ffi::c_int as png_uint_16,
    5810 as ::core::ffi::c_int as png_uint_16,
    5953 as ::core::ffi::c_int as png_uint_16,
    6099 as ::core::ffi::c_int as png_uint_16,
    6246 as ::core::ffi::c_int as png_uint_16,
    6395 as ::core::ffi::c_int as png_uint_16,
    6547 as ::core::ffi::c_int as png_uint_16,
    6700 as ::core::ffi::c_int as png_uint_16,
    6856 as ::core::ffi::c_int as png_uint_16,
    7014 as ::core::ffi::c_int as png_uint_16,
    7174 as ::core::ffi::c_int as png_uint_16,
    7335 as ::core::ffi::c_int as png_uint_16,
    7500 as ::core::ffi::c_int as png_uint_16,
    7666 as ::core::ffi::c_int as png_uint_16,
    7834 as ::core::ffi::c_int as png_uint_16,
    8004 as ::core::ffi::c_int as png_uint_16,
    8177 as ::core::ffi::c_int as png_uint_16,
    8352 as ::core::ffi::c_int as png_uint_16,
    8528 as ::core::ffi::c_int as png_uint_16,
    8708 as ::core::ffi::c_int as png_uint_16,
    8889 as ::core::ffi::c_int as png_uint_16,
    9072 as ::core::ffi::c_int as png_uint_16,
    9258 as ::core::ffi::c_int as png_uint_16,
    9445 as ::core::ffi::c_int as png_uint_16,
    9635 as ::core::ffi::c_int as png_uint_16,
    9828 as ::core::ffi::c_int as png_uint_16,
    10022 as ::core::ffi::c_int as png_uint_16,
    10219 as ::core::ffi::c_int as png_uint_16,
    10417 as ::core::ffi::c_int as png_uint_16,
    10619 as ::core::ffi::c_int as png_uint_16,
    10822 as ::core::ffi::c_int as png_uint_16,
    11028 as ::core::ffi::c_int as png_uint_16,
    11235 as ::core::ffi::c_int as png_uint_16,
    11446 as ::core::ffi::c_int as png_uint_16,
    11658 as ::core::ffi::c_int as png_uint_16,
    11873 as ::core::ffi::c_int as png_uint_16,
    12090 as ::core::ffi::c_int as png_uint_16,
    12309 as ::core::ffi::c_int as png_uint_16,
    12530 as ::core::ffi::c_int as png_uint_16,
    12754 as ::core::ffi::c_int as png_uint_16,
    12980 as ::core::ffi::c_int as png_uint_16,
    13209 as ::core::ffi::c_int as png_uint_16,
    13440 as ::core::ffi::c_int as png_uint_16,
    13673 as ::core::ffi::c_int as png_uint_16,
    13909 as ::core::ffi::c_int as png_uint_16,
    14146 as ::core::ffi::c_int as png_uint_16,
    14387 as ::core::ffi::c_int as png_uint_16,
    14629 as ::core::ffi::c_int as png_uint_16,
    14874 as ::core::ffi::c_int as png_uint_16,
    15122 as ::core::ffi::c_int as png_uint_16,
    15371 as ::core::ffi::c_int as png_uint_16,
    15623 as ::core::ffi::c_int as png_uint_16,
    15878 as ::core::ffi::c_int as png_uint_16,
    16135 as ::core::ffi::c_int as png_uint_16,
    16394 as ::core::ffi::c_int as png_uint_16,
    16656 as ::core::ffi::c_int as png_uint_16,
    16920 as ::core::ffi::c_int as png_uint_16,
    17187 as ::core::ffi::c_int as png_uint_16,
    17456 as ::core::ffi::c_int as png_uint_16,
    17727 as ::core::ffi::c_int as png_uint_16,
    18001 as ::core::ffi::c_int as png_uint_16,
    18277 as ::core::ffi::c_int as png_uint_16,
    18556 as ::core::ffi::c_int as png_uint_16,
    18837 as ::core::ffi::c_int as png_uint_16,
    19121 as ::core::ffi::c_int as png_uint_16,
    19407 as ::core::ffi::c_int as png_uint_16,
    19696 as ::core::ffi::c_int as png_uint_16,
    19987 as ::core::ffi::c_int as png_uint_16,
    20281 as ::core::ffi::c_int as png_uint_16,
    20577 as ::core::ffi::c_int as png_uint_16,
    20876 as ::core::ffi::c_int as png_uint_16,
    21177 as ::core::ffi::c_int as png_uint_16,
    21481 as ::core::ffi::c_int as png_uint_16,
    21787 as ::core::ffi::c_int as png_uint_16,
    22096 as ::core::ffi::c_int as png_uint_16,
    22407 as ::core::ffi::c_int as png_uint_16,
    22721 as ::core::ffi::c_int as png_uint_16,
    23038 as ::core::ffi::c_int as png_uint_16,
    23357 as ::core::ffi::c_int as png_uint_16,
    23678 as ::core::ffi::c_int as png_uint_16,
    24002 as ::core::ffi::c_int as png_uint_16,
    24329 as ::core::ffi::c_int as png_uint_16,
    24658 as ::core::ffi::c_int as png_uint_16,
    24990 as ::core::ffi::c_int as png_uint_16,
    25325 as ::core::ffi::c_int as png_uint_16,
    25662 as ::core::ffi::c_int as png_uint_16,
    26001 as ::core::ffi::c_int as png_uint_16,
    26344 as ::core::ffi::c_int as png_uint_16,
    26688 as ::core::ffi::c_int as png_uint_16,
    27036 as ::core::ffi::c_int as png_uint_16,
    27386 as ::core::ffi::c_int as png_uint_16,
    27739 as ::core::ffi::c_int as png_uint_16,
    28094 as ::core::ffi::c_int as png_uint_16,
    28452 as ::core::ffi::c_int as png_uint_16,
    28813 as ::core::ffi::c_int as png_uint_16,
    29176 as ::core::ffi::c_int as png_uint_16,
    29542 as ::core::ffi::c_int as png_uint_16,
    29911 as ::core::ffi::c_int as png_uint_16,
    30282 as ::core::ffi::c_int as png_uint_16,
    30656 as ::core::ffi::c_int as png_uint_16,
    31033 as ::core::ffi::c_int as png_uint_16,
    31412 as ::core::ffi::c_int as png_uint_16,
    31794 as ::core::ffi::c_int as png_uint_16,
    32179 as ::core::ffi::c_int as png_uint_16,
    32567 as ::core::ffi::c_int as png_uint_16,
    32957 as ::core::ffi::c_int as png_uint_16,
    33350 as ::core::ffi::c_int as png_uint_16,
    33745 as ::core::ffi::c_int as png_uint_16,
    34143 as ::core::ffi::c_int as png_uint_16,
    34544 as ::core::ffi::c_int as png_uint_16,
    34948 as ::core::ffi::c_int as png_uint_16,
    35355 as ::core::ffi::c_int as png_uint_16,
    35764 as ::core::ffi::c_int as png_uint_16,
    36176 as ::core::ffi::c_int as png_uint_16,
    36591 as ::core::ffi::c_int as png_uint_16,
    37008 as ::core::ffi::c_int as png_uint_16,
    37429 as ::core::ffi::c_int as png_uint_16,
    37852 as ::core::ffi::c_int as png_uint_16,
    38278 as ::core::ffi::c_int as png_uint_16,
    38706 as ::core::ffi::c_int as png_uint_16,
    39138 as ::core::ffi::c_int as png_uint_16,
    39572 as ::core::ffi::c_int as png_uint_16,
    40009 as ::core::ffi::c_int as png_uint_16,
    40449 as ::core::ffi::c_int as png_uint_16,
    40891 as ::core::ffi::c_int as png_uint_16,
    41337 as ::core::ffi::c_int as png_uint_16,
    41785 as ::core::ffi::c_int as png_uint_16,
    42236 as ::core::ffi::c_int as png_uint_16,
    42690 as ::core::ffi::c_int as png_uint_16,
    43147 as ::core::ffi::c_int as png_uint_16,
    43606 as ::core::ffi::c_int as png_uint_16,
    44069 as ::core::ffi::c_int as png_uint_16,
    44534 as ::core::ffi::c_int as png_uint_16,
    45002 as ::core::ffi::c_int as png_uint_16,
    45473 as ::core::ffi::c_int as png_uint_16,
    45947 as ::core::ffi::c_int as png_uint_16,
    46423 as ::core::ffi::c_int as png_uint_16,
    46903 as ::core::ffi::c_int as png_uint_16,
    47385 as ::core::ffi::c_int as png_uint_16,
    47871 as ::core::ffi::c_int as png_uint_16,
    48359 as ::core::ffi::c_int as png_uint_16,
    48850 as ::core::ffi::c_int as png_uint_16,
    49344 as ::core::ffi::c_int as png_uint_16,
    49841 as ::core::ffi::c_int as png_uint_16,
    50341 as ::core::ffi::c_int as png_uint_16,
    50844 as ::core::ffi::c_int as png_uint_16,
    51349 as ::core::ffi::c_int as png_uint_16,
    51858 as ::core::ffi::c_int as png_uint_16,
    52369 as ::core::ffi::c_int as png_uint_16,
    52884 as ::core::ffi::c_int as png_uint_16,
    53401 as ::core::ffi::c_int as png_uint_16,
    53921 as ::core::ffi::c_int as png_uint_16,
    54445 as ::core::ffi::c_int as png_uint_16,
    54971 as ::core::ffi::c_int as png_uint_16,
    55500 as ::core::ffi::c_int as png_uint_16,
    56032 as ::core::ffi::c_int as png_uint_16,
    56567 as ::core::ffi::c_int as png_uint_16,
    57105 as ::core::ffi::c_int as png_uint_16,
    57646 as ::core::ffi::c_int as png_uint_16,
    58190 as ::core::ffi::c_int as png_uint_16,
    58737 as ::core::ffi::c_int as png_uint_16,
    59287 as ::core::ffi::c_int as png_uint_16,
    59840 as ::core::ffi::c_int as png_uint_16,
    60396 as ::core::ffi::c_int as png_uint_16,
    60955 as ::core::ffi::c_int as png_uint_16,
    61517 as ::core::ffi::c_int as png_uint_16,
    62082 as ::core::ffi::c_int as png_uint_16,
    62650 as ::core::ffi::c_int as png_uint_16,
    63221 as ::core::ffi::c_int as png_uint_16,
    63795 as ::core::ffi::c_int as png_uint_16,
    64372 as ::core::ffi::c_int as png_uint_16,
    64952 as ::core::ffi::c_int as png_uint_16,
    65535 as ::core::ffi::c_int as png_uint_16,
];
#[unsafe(no_mangle)]
pub static png_sRGB_base: [png_uint_16; 512] = [
    128 as ::core::ffi::c_int as png_uint_16,
    1782 as ::core::ffi::c_int as png_uint_16,
    3383 as ::core::ffi::c_int as png_uint_16,
    4644 as ::core::ffi::c_int as png_uint_16,
    5675 as ::core::ffi::c_int as png_uint_16,
    6564 as ::core::ffi::c_int as png_uint_16,
    7357 as ::core::ffi::c_int as png_uint_16,
    8074 as ::core::ffi::c_int as png_uint_16,
    8732 as ::core::ffi::c_int as png_uint_16,
    9346 as ::core::ffi::c_int as png_uint_16,
    9921 as ::core::ffi::c_int as png_uint_16,
    10463 as ::core::ffi::c_int as png_uint_16,
    10977 as ::core::ffi::c_int as png_uint_16,
    11466 as ::core::ffi::c_int as png_uint_16,
    11935 as ::core::ffi::c_int as png_uint_16,
    12384 as ::core::ffi::c_int as png_uint_16,
    12816 as ::core::ffi::c_int as png_uint_16,
    13233 as ::core::ffi::c_int as png_uint_16,
    13634 as ::core::ffi::c_int as png_uint_16,
    14024 as ::core::ffi::c_int as png_uint_16,
    14402 as ::core::ffi::c_int as png_uint_16,
    14769 as ::core::ffi::c_int as png_uint_16,
    15125 as ::core::ffi::c_int as png_uint_16,
    15473 as ::core::ffi::c_int as png_uint_16,
    15812 as ::core::ffi::c_int as png_uint_16,
    16142 as ::core::ffi::c_int as png_uint_16,
    16466 as ::core::ffi::c_int as png_uint_16,
    16781 as ::core::ffi::c_int as png_uint_16,
    17090 as ::core::ffi::c_int as png_uint_16,
    17393 as ::core::ffi::c_int as png_uint_16,
    17690 as ::core::ffi::c_int as png_uint_16,
    17981 as ::core::ffi::c_int as png_uint_16,
    18266 as ::core::ffi::c_int as png_uint_16,
    18546 as ::core::ffi::c_int as png_uint_16,
    18822 as ::core::ffi::c_int as png_uint_16,
    19093 as ::core::ffi::c_int as png_uint_16,
    19359 as ::core::ffi::c_int as png_uint_16,
    19621 as ::core::ffi::c_int as png_uint_16,
    19879 as ::core::ffi::c_int as png_uint_16,
    20133 as ::core::ffi::c_int as png_uint_16,
    20383 as ::core::ffi::c_int as png_uint_16,
    20630 as ::core::ffi::c_int as png_uint_16,
    20873 as ::core::ffi::c_int as png_uint_16,
    21113 as ::core::ffi::c_int as png_uint_16,
    21349 as ::core::ffi::c_int as png_uint_16,
    21583 as ::core::ffi::c_int as png_uint_16,
    21813 as ::core::ffi::c_int as png_uint_16,
    22041 as ::core::ffi::c_int as png_uint_16,
    22265 as ::core::ffi::c_int as png_uint_16,
    22487 as ::core::ffi::c_int as png_uint_16,
    22707 as ::core::ffi::c_int as png_uint_16,
    22923 as ::core::ffi::c_int as png_uint_16,
    23138 as ::core::ffi::c_int as png_uint_16,
    23350 as ::core::ffi::c_int as png_uint_16,
    23559 as ::core::ffi::c_int as png_uint_16,
    23767 as ::core::ffi::c_int as png_uint_16,
    23972 as ::core::ffi::c_int as png_uint_16,
    24175 as ::core::ffi::c_int as png_uint_16,
    24376 as ::core::ffi::c_int as png_uint_16,
    24575 as ::core::ffi::c_int as png_uint_16,
    24772 as ::core::ffi::c_int as png_uint_16,
    24967 as ::core::ffi::c_int as png_uint_16,
    25160 as ::core::ffi::c_int as png_uint_16,
    25352 as ::core::ffi::c_int as png_uint_16,
    25542 as ::core::ffi::c_int as png_uint_16,
    25730 as ::core::ffi::c_int as png_uint_16,
    25916 as ::core::ffi::c_int as png_uint_16,
    26101 as ::core::ffi::c_int as png_uint_16,
    26284 as ::core::ffi::c_int as png_uint_16,
    26465 as ::core::ffi::c_int as png_uint_16,
    26645 as ::core::ffi::c_int as png_uint_16,
    26823 as ::core::ffi::c_int as png_uint_16,
    27000 as ::core::ffi::c_int as png_uint_16,
    27176 as ::core::ffi::c_int as png_uint_16,
    27350 as ::core::ffi::c_int as png_uint_16,
    27523 as ::core::ffi::c_int as png_uint_16,
    27695 as ::core::ffi::c_int as png_uint_16,
    27865 as ::core::ffi::c_int as png_uint_16,
    28034 as ::core::ffi::c_int as png_uint_16,
    28201 as ::core::ffi::c_int as png_uint_16,
    28368 as ::core::ffi::c_int as png_uint_16,
    28533 as ::core::ffi::c_int as png_uint_16,
    28697 as ::core::ffi::c_int as png_uint_16,
    28860 as ::core::ffi::c_int as png_uint_16,
    29021 as ::core::ffi::c_int as png_uint_16,
    29182 as ::core::ffi::c_int as png_uint_16,
    29341 as ::core::ffi::c_int as png_uint_16,
    29500 as ::core::ffi::c_int as png_uint_16,
    29657 as ::core::ffi::c_int as png_uint_16,
    29813 as ::core::ffi::c_int as png_uint_16,
    29969 as ::core::ffi::c_int as png_uint_16,
    30123 as ::core::ffi::c_int as png_uint_16,
    30276 as ::core::ffi::c_int as png_uint_16,
    30429 as ::core::ffi::c_int as png_uint_16,
    30580 as ::core::ffi::c_int as png_uint_16,
    30730 as ::core::ffi::c_int as png_uint_16,
    30880 as ::core::ffi::c_int as png_uint_16,
    31028 as ::core::ffi::c_int as png_uint_16,
    31176 as ::core::ffi::c_int as png_uint_16,
    31323 as ::core::ffi::c_int as png_uint_16,
    31469 as ::core::ffi::c_int as png_uint_16,
    31614 as ::core::ffi::c_int as png_uint_16,
    31758 as ::core::ffi::c_int as png_uint_16,
    31902 as ::core::ffi::c_int as png_uint_16,
    32045 as ::core::ffi::c_int as png_uint_16,
    32186 as ::core::ffi::c_int as png_uint_16,
    32327 as ::core::ffi::c_int as png_uint_16,
    32468 as ::core::ffi::c_int as png_uint_16,
    32607 as ::core::ffi::c_int as png_uint_16,
    32746 as ::core::ffi::c_int as png_uint_16,
    32884 as ::core::ffi::c_int as png_uint_16,
    33021 as ::core::ffi::c_int as png_uint_16,
    33158 as ::core::ffi::c_int as png_uint_16,
    33294 as ::core::ffi::c_int as png_uint_16,
    33429 as ::core::ffi::c_int as png_uint_16,
    33564 as ::core::ffi::c_int as png_uint_16,
    33697 as ::core::ffi::c_int as png_uint_16,
    33831 as ::core::ffi::c_int as png_uint_16,
    33963 as ::core::ffi::c_int as png_uint_16,
    34095 as ::core::ffi::c_int as png_uint_16,
    34226 as ::core::ffi::c_int as png_uint_16,
    34357 as ::core::ffi::c_int as png_uint_16,
    34486 as ::core::ffi::c_int as png_uint_16,
    34616 as ::core::ffi::c_int as png_uint_16,
    34744 as ::core::ffi::c_int as png_uint_16,
    34873 as ::core::ffi::c_int as png_uint_16,
    35000 as ::core::ffi::c_int as png_uint_16,
    35127 as ::core::ffi::c_int as png_uint_16,
    35253 as ::core::ffi::c_int as png_uint_16,
    35379 as ::core::ffi::c_int as png_uint_16,
    35504 as ::core::ffi::c_int as png_uint_16,
    35629 as ::core::ffi::c_int as png_uint_16,
    35753 as ::core::ffi::c_int as png_uint_16,
    35876 as ::core::ffi::c_int as png_uint_16,
    35999 as ::core::ffi::c_int as png_uint_16,
    36122 as ::core::ffi::c_int as png_uint_16,
    36244 as ::core::ffi::c_int as png_uint_16,
    36365 as ::core::ffi::c_int as png_uint_16,
    36486 as ::core::ffi::c_int as png_uint_16,
    36606 as ::core::ffi::c_int as png_uint_16,
    36726 as ::core::ffi::c_int as png_uint_16,
    36845 as ::core::ffi::c_int as png_uint_16,
    36964 as ::core::ffi::c_int as png_uint_16,
    37083 as ::core::ffi::c_int as png_uint_16,
    37201 as ::core::ffi::c_int as png_uint_16,
    37318 as ::core::ffi::c_int as png_uint_16,
    37435 as ::core::ffi::c_int as png_uint_16,
    37551 as ::core::ffi::c_int as png_uint_16,
    37668 as ::core::ffi::c_int as png_uint_16,
    37783 as ::core::ffi::c_int as png_uint_16,
    37898 as ::core::ffi::c_int as png_uint_16,
    38013 as ::core::ffi::c_int as png_uint_16,
    38127 as ::core::ffi::c_int as png_uint_16,
    38241 as ::core::ffi::c_int as png_uint_16,
    38354 as ::core::ffi::c_int as png_uint_16,
    38467 as ::core::ffi::c_int as png_uint_16,
    38580 as ::core::ffi::c_int as png_uint_16,
    38692 as ::core::ffi::c_int as png_uint_16,
    38803 as ::core::ffi::c_int as png_uint_16,
    38915 as ::core::ffi::c_int as png_uint_16,
    39026 as ::core::ffi::c_int as png_uint_16,
    39136 as ::core::ffi::c_int as png_uint_16,
    39246 as ::core::ffi::c_int as png_uint_16,
    39356 as ::core::ffi::c_int as png_uint_16,
    39465 as ::core::ffi::c_int as png_uint_16,
    39574 as ::core::ffi::c_int as png_uint_16,
    39682 as ::core::ffi::c_int as png_uint_16,
    39790 as ::core::ffi::c_int as png_uint_16,
    39898 as ::core::ffi::c_int as png_uint_16,
    40005 as ::core::ffi::c_int as png_uint_16,
    40112 as ::core::ffi::c_int as png_uint_16,
    40219 as ::core::ffi::c_int as png_uint_16,
    40325 as ::core::ffi::c_int as png_uint_16,
    40431 as ::core::ffi::c_int as png_uint_16,
    40537 as ::core::ffi::c_int as png_uint_16,
    40642 as ::core::ffi::c_int as png_uint_16,
    40747 as ::core::ffi::c_int as png_uint_16,
    40851 as ::core::ffi::c_int as png_uint_16,
    40955 as ::core::ffi::c_int as png_uint_16,
    41059 as ::core::ffi::c_int as png_uint_16,
    41163 as ::core::ffi::c_int as png_uint_16,
    41266 as ::core::ffi::c_int as png_uint_16,
    41369 as ::core::ffi::c_int as png_uint_16,
    41471 as ::core::ffi::c_int as png_uint_16,
    41573 as ::core::ffi::c_int as png_uint_16,
    41675 as ::core::ffi::c_int as png_uint_16,
    41777 as ::core::ffi::c_int as png_uint_16,
    41878 as ::core::ffi::c_int as png_uint_16,
    41979 as ::core::ffi::c_int as png_uint_16,
    42079 as ::core::ffi::c_int as png_uint_16,
    42179 as ::core::ffi::c_int as png_uint_16,
    42279 as ::core::ffi::c_int as png_uint_16,
    42379 as ::core::ffi::c_int as png_uint_16,
    42478 as ::core::ffi::c_int as png_uint_16,
    42577 as ::core::ffi::c_int as png_uint_16,
    42676 as ::core::ffi::c_int as png_uint_16,
    42775 as ::core::ffi::c_int as png_uint_16,
    42873 as ::core::ffi::c_int as png_uint_16,
    42971 as ::core::ffi::c_int as png_uint_16,
    43068 as ::core::ffi::c_int as png_uint_16,
    43165 as ::core::ffi::c_int as png_uint_16,
    43262 as ::core::ffi::c_int as png_uint_16,
    43359 as ::core::ffi::c_int as png_uint_16,
    43456 as ::core::ffi::c_int as png_uint_16,
    43552 as ::core::ffi::c_int as png_uint_16,
    43648 as ::core::ffi::c_int as png_uint_16,
    43743 as ::core::ffi::c_int as png_uint_16,
    43839 as ::core::ffi::c_int as png_uint_16,
    43934 as ::core::ffi::c_int as png_uint_16,
    44028 as ::core::ffi::c_int as png_uint_16,
    44123 as ::core::ffi::c_int as png_uint_16,
    44217 as ::core::ffi::c_int as png_uint_16,
    44311 as ::core::ffi::c_int as png_uint_16,
    44405 as ::core::ffi::c_int as png_uint_16,
    44499 as ::core::ffi::c_int as png_uint_16,
    44592 as ::core::ffi::c_int as png_uint_16,
    44685 as ::core::ffi::c_int as png_uint_16,
    44778 as ::core::ffi::c_int as png_uint_16,
    44870 as ::core::ffi::c_int as png_uint_16,
    44962 as ::core::ffi::c_int as png_uint_16,
    45054 as ::core::ffi::c_int as png_uint_16,
    45146 as ::core::ffi::c_int as png_uint_16,
    45238 as ::core::ffi::c_int as png_uint_16,
    45329 as ::core::ffi::c_int as png_uint_16,
    45420 as ::core::ffi::c_int as png_uint_16,
    45511 as ::core::ffi::c_int as png_uint_16,
    45601 as ::core::ffi::c_int as png_uint_16,
    45692 as ::core::ffi::c_int as png_uint_16,
    45782 as ::core::ffi::c_int as png_uint_16,
    45872 as ::core::ffi::c_int as png_uint_16,
    45961 as ::core::ffi::c_int as png_uint_16,
    46051 as ::core::ffi::c_int as png_uint_16,
    46140 as ::core::ffi::c_int as png_uint_16,
    46229 as ::core::ffi::c_int as png_uint_16,
    46318 as ::core::ffi::c_int as png_uint_16,
    46406 as ::core::ffi::c_int as png_uint_16,
    46494 as ::core::ffi::c_int as png_uint_16,
    46583 as ::core::ffi::c_int as png_uint_16,
    46670 as ::core::ffi::c_int as png_uint_16,
    46758 as ::core::ffi::c_int as png_uint_16,
    46846 as ::core::ffi::c_int as png_uint_16,
    46933 as ::core::ffi::c_int as png_uint_16,
    47020 as ::core::ffi::c_int as png_uint_16,
    47107 as ::core::ffi::c_int as png_uint_16,
    47193 as ::core::ffi::c_int as png_uint_16,
    47280 as ::core::ffi::c_int as png_uint_16,
    47366 as ::core::ffi::c_int as png_uint_16,
    47452 as ::core::ffi::c_int as png_uint_16,
    47538 as ::core::ffi::c_int as png_uint_16,
    47623 as ::core::ffi::c_int as png_uint_16,
    47709 as ::core::ffi::c_int as png_uint_16,
    47794 as ::core::ffi::c_int as png_uint_16,
    47879 as ::core::ffi::c_int as png_uint_16,
    47964 as ::core::ffi::c_int as png_uint_16,
    48048 as ::core::ffi::c_int as png_uint_16,
    48133 as ::core::ffi::c_int as png_uint_16,
    48217 as ::core::ffi::c_int as png_uint_16,
    48301 as ::core::ffi::c_int as png_uint_16,
    48385 as ::core::ffi::c_int as png_uint_16,
    48468 as ::core::ffi::c_int as png_uint_16,
    48552 as ::core::ffi::c_int as png_uint_16,
    48635 as ::core::ffi::c_int as png_uint_16,
    48718 as ::core::ffi::c_int as png_uint_16,
    48801 as ::core::ffi::c_int as png_uint_16,
    48884 as ::core::ffi::c_int as png_uint_16,
    48966 as ::core::ffi::c_int as png_uint_16,
    49048 as ::core::ffi::c_int as png_uint_16,
    49131 as ::core::ffi::c_int as png_uint_16,
    49213 as ::core::ffi::c_int as png_uint_16,
    49294 as ::core::ffi::c_int as png_uint_16,
    49376 as ::core::ffi::c_int as png_uint_16,
    49458 as ::core::ffi::c_int as png_uint_16,
    49539 as ::core::ffi::c_int as png_uint_16,
    49620 as ::core::ffi::c_int as png_uint_16,
    49701 as ::core::ffi::c_int as png_uint_16,
    49782 as ::core::ffi::c_int as png_uint_16,
    49862 as ::core::ffi::c_int as png_uint_16,
    49943 as ::core::ffi::c_int as png_uint_16,
    50023 as ::core::ffi::c_int as png_uint_16,
    50103 as ::core::ffi::c_int as png_uint_16,
    50183 as ::core::ffi::c_int as png_uint_16,
    50263 as ::core::ffi::c_int as png_uint_16,
    50342 as ::core::ffi::c_int as png_uint_16,
    50422 as ::core::ffi::c_int as png_uint_16,
    50501 as ::core::ffi::c_int as png_uint_16,
    50580 as ::core::ffi::c_int as png_uint_16,
    50659 as ::core::ffi::c_int as png_uint_16,
    50738 as ::core::ffi::c_int as png_uint_16,
    50816 as ::core::ffi::c_int as png_uint_16,
    50895 as ::core::ffi::c_int as png_uint_16,
    50973 as ::core::ffi::c_int as png_uint_16,
    51051 as ::core::ffi::c_int as png_uint_16,
    51129 as ::core::ffi::c_int as png_uint_16,
    51207 as ::core::ffi::c_int as png_uint_16,
    51285 as ::core::ffi::c_int as png_uint_16,
    51362 as ::core::ffi::c_int as png_uint_16,
    51439 as ::core::ffi::c_int as png_uint_16,
    51517 as ::core::ffi::c_int as png_uint_16,
    51594 as ::core::ffi::c_int as png_uint_16,
    51671 as ::core::ffi::c_int as png_uint_16,
    51747 as ::core::ffi::c_int as png_uint_16,
    51824 as ::core::ffi::c_int as png_uint_16,
    51900 as ::core::ffi::c_int as png_uint_16,
    51977 as ::core::ffi::c_int as png_uint_16,
    52053 as ::core::ffi::c_int as png_uint_16,
    52129 as ::core::ffi::c_int as png_uint_16,
    52205 as ::core::ffi::c_int as png_uint_16,
    52280 as ::core::ffi::c_int as png_uint_16,
    52356 as ::core::ffi::c_int as png_uint_16,
    52432 as ::core::ffi::c_int as png_uint_16,
    52507 as ::core::ffi::c_int as png_uint_16,
    52582 as ::core::ffi::c_int as png_uint_16,
    52657 as ::core::ffi::c_int as png_uint_16,
    52732 as ::core::ffi::c_int as png_uint_16,
    52807 as ::core::ffi::c_int as png_uint_16,
    52881 as ::core::ffi::c_int as png_uint_16,
    52956 as ::core::ffi::c_int as png_uint_16,
    53030 as ::core::ffi::c_int as png_uint_16,
    53104 as ::core::ffi::c_int as png_uint_16,
    53178 as ::core::ffi::c_int as png_uint_16,
    53252 as ::core::ffi::c_int as png_uint_16,
    53326 as ::core::ffi::c_int as png_uint_16,
    53400 as ::core::ffi::c_int as png_uint_16,
    53473 as ::core::ffi::c_int as png_uint_16,
    53546 as ::core::ffi::c_int as png_uint_16,
    53620 as ::core::ffi::c_int as png_uint_16,
    53693 as ::core::ffi::c_int as png_uint_16,
    53766 as ::core::ffi::c_int as png_uint_16,
    53839 as ::core::ffi::c_int as png_uint_16,
    53911 as ::core::ffi::c_int as png_uint_16,
    53984 as ::core::ffi::c_int as png_uint_16,
    54056 as ::core::ffi::c_int as png_uint_16,
    54129 as ::core::ffi::c_int as png_uint_16,
    54201 as ::core::ffi::c_int as png_uint_16,
    54273 as ::core::ffi::c_int as png_uint_16,
    54345 as ::core::ffi::c_int as png_uint_16,
    54417 as ::core::ffi::c_int as png_uint_16,
    54489 as ::core::ffi::c_int as png_uint_16,
    54560 as ::core::ffi::c_int as png_uint_16,
    54632 as ::core::ffi::c_int as png_uint_16,
    54703 as ::core::ffi::c_int as png_uint_16,
    54774 as ::core::ffi::c_int as png_uint_16,
    54845 as ::core::ffi::c_int as png_uint_16,
    54916 as ::core::ffi::c_int as png_uint_16,
    54987 as ::core::ffi::c_int as png_uint_16,
    55058 as ::core::ffi::c_int as png_uint_16,
    55129 as ::core::ffi::c_int as png_uint_16,
    55199 as ::core::ffi::c_int as png_uint_16,
    55269 as ::core::ffi::c_int as png_uint_16,
    55340 as ::core::ffi::c_int as png_uint_16,
    55410 as ::core::ffi::c_int as png_uint_16,
    55480 as ::core::ffi::c_int as png_uint_16,
    55550 as ::core::ffi::c_int as png_uint_16,
    55620 as ::core::ffi::c_int as png_uint_16,
    55689 as ::core::ffi::c_int as png_uint_16,
    55759 as ::core::ffi::c_int as png_uint_16,
    55828 as ::core::ffi::c_int as png_uint_16,
    55898 as ::core::ffi::c_int as png_uint_16,
    55967 as ::core::ffi::c_int as png_uint_16,
    56036 as ::core::ffi::c_int as png_uint_16,
    56105 as ::core::ffi::c_int as png_uint_16,
    56174 as ::core::ffi::c_int as png_uint_16,
    56243 as ::core::ffi::c_int as png_uint_16,
    56311 as ::core::ffi::c_int as png_uint_16,
    56380 as ::core::ffi::c_int as png_uint_16,
    56448 as ::core::ffi::c_int as png_uint_16,
    56517 as ::core::ffi::c_int as png_uint_16,
    56585 as ::core::ffi::c_int as png_uint_16,
    56653 as ::core::ffi::c_int as png_uint_16,
    56721 as ::core::ffi::c_int as png_uint_16,
    56789 as ::core::ffi::c_int as png_uint_16,
    56857 as ::core::ffi::c_int as png_uint_16,
    56924 as ::core::ffi::c_int as png_uint_16,
    56992 as ::core::ffi::c_int as png_uint_16,
    57059 as ::core::ffi::c_int as png_uint_16,
    57127 as ::core::ffi::c_int as png_uint_16,
    57194 as ::core::ffi::c_int as png_uint_16,
    57261 as ::core::ffi::c_int as png_uint_16,
    57328 as ::core::ffi::c_int as png_uint_16,
    57395 as ::core::ffi::c_int as png_uint_16,
    57462 as ::core::ffi::c_int as png_uint_16,
    57529 as ::core::ffi::c_int as png_uint_16,
    57595 as ::core::ffi::c_int as png_uint_16,
    57662 as ::core::ffi::c_int as png_uint_16,
    57728 as ::core::ffi::c_int as png_uint_16,
    57795 as ::core::ffi::c_int as png_uint_16,
    57861 as ::core::ffi::c_int as png_uint_16,
    57927 as ::core::ffi::c_int as png_uint_16,
    57993 as ::core::ffi::c_int as png_uint_16,
    58059 as ::core::ffi::c_int as png_uint_16,
    58125 as ::core::ffi::c_int as png_uint_16,
    58191 as ::core::ffi::c_int as png_uint_16,
    58256 as ::core::ffi::c_int as png_uint_16,
    58322 as ::core::ffi::c_int as png_uint_16,
    58387 as ::core::ffi::c_int as png_uint_16,
    58453 as ::core::ffi::c_int as png_uint_16,
    58518 as ::core::ffi::c_int as png_uint_16,
    58583 as ::core::ffi::c_int as png_uint_16,
    58648 as ::core::ffi::c_int as png_uint_16,
    58713 as ::core::ffi::c_int as png_uint_16,
    58778 as ::core::ffi::c_int as png_uint_16,
    58843 as ::core::ffi::c_int as png_uint_16,
    58908 as ::core::ffi::c_int as png_uint_16,
    58972 as ::core::ffi::c_int as png_uint_16,
    59037 as ::core::ffi::c_int as png_uint_16,
    59101 as ::core::ffi::c_int as png_uint_16,
    59165 as ::core::ffi::c_int as png_uint_16,
    59230 as ::core::ffi::c_int as png_uint_16,
    59294 as ::core::ffi::c_int as png_uint_16,
    59358 as ::core::ffi::c_int as png_uint_16,
    59422 as ::core::ffi::c_int as png_uint_16,
    59486 as ::core::ffi::c_int as png_uint_16,
    59549 as ::core::ffi::c_int as png_uint_16,
    59613 as ::core::ffi::c_int as png_uint_16,
    59677 as ::core::ffi::c_int as png_uint_16,
    59740 as ::core::ffi::c_int as png_uint_16,
    59804 as ::core::ffi::c_int as png_uint_16,
    59867 as ::core::ffi::c_int as png_uint_16,
    59930 as ::core::ffi::c_int as png_uint_16,
    59993 as ::core::ffi::c_int as png_uint_16,
    60056 as ::core::ffi::c_int as png_uint_16,
    60119 as ::core::ffi::c_int as png_uint_16,
    60182 as ::core::ffi::c_int as png_uint_16,
    60245 as ::core::ffi::c_int as png_uint_16,
    60308 as ::core::ffi::c_int as png_uint_16,
    60370 as ::core::ffi::c_int as png_uint_16,
    60433 as ::core::ffi::c_int as png_uint_16,
    60495 as ::core::ffi::c_int as png_uint_16,
    60558 as ::core::ffi::c_int as png_uint_16,
    60620 as ::core::ffi::c_int as png_uint_16,
    60682 as ::core::ffi::c_int as png_uint_16,
    60744 as ::core::ffi::c_int as png_uint_16,
    60806 as ::core::ffi::c_int as png_uint_16,
    60868 as ::core::ffi::c_int as png_uint_16,
    60930 as ::core::ffi::c_int as png_uint_16,
    60992 as ::core::ffi::c_int as png_uint_16,
    61054 as ::core::ffi::c_int as png_uint_16,
    61115 as ::core::ffi::c_int as png_uint_16,
    61177 as ::core::ffi::c_int as png_uint_16,
    61238 as ::core::ffi::c_int as png_uint_16,
    61300 as ::core::ffi::c_int as png_uint_16,
    61361 as ::core::ffi::c_int as png_uint_16,
    61422 as ::core::ffi::c_int as png_uint_16,
    61483 as ::core::ffi::c_int as png_uint_16,
    61544 as ::core::ffi::c_int as png_uint_16,
    61605 as ::core::ffi::c_int as png_uint_16,
    61666 as ::core::ffi::c_int as png_uint_16,
    61727 as ::core::ffi::c_int as png_uint_16,
    61788 as ::core::ffi::c_int as png_uint_16,
    61848 as ::core::ffi::c_int as png_uint_16,
    61909 as ::core::ffi::c_int as png_uint_16,
    61969 as ::core::ffi::c_int as png_uint_16,
    62030 as ::core::ffi::c_int as png_uint_16,
    62090 as ::core::ffi::c_int as png_uint_16,
    62150 as ::core::ffi::c_int as png_uint_16,
    62211 as ::core::ffi::c_int as png_uint_16,
    62271 as ::core::ffi::c_int as png_uint_16,
    62331 as ::core::ffi::c_int as png_uint_16,
    62391 as ::core::ffi::c_int as png_uint_16,
    62450 as ::core::ffi::c_int as png_uint_16,
    62510 as ::core::ffi::c_int as png_uint_16,
    62570 as ::core::ffi::c_int as png_uint_16,
    62630 as ::core::ffi::c_int as png_uint_16,
    62689 as ::core::ffi::c_int as png_uint_16,
    62749 as ::core::ffi::c_int as png_uint_16,
    62808 as ::core::ffi::c_int as png_uint_16,
    62867 as ::core::ffi::c_int as png_uint_16,
    62927 as ::core::ffi::c_int as png_uint_16,
    62986 as ::core::ffi::c_int as png_uint_16,
    63045 as ::core::ffi::c_int as png_uint_16,
    63104 as ::core::ffi::c_int as png_uint_16,
    63163 as ::core::ffi::c_int as png_uint_16,
    63222 as ::core::ffi::c_int as png_uint_16,
    63281 as ::core::ffi::c_int as png_uint_16,
    63340 as ::core::ffi::c_int as png_uint_16,
    63398 as ::core::ffi::c_int as png_uint_16,
    63457 as ::core::ffi::c_int as png_uint_16,
    63515 as ::core::ffi::c_int as png_uint_16,
    63574 as ::core::ffi::c_int as png_uint_16,
    63632 as ::core::ffi::c_int as png_uint_16,
    63691 as ::core::ffi::c_int as png_uint_16,
    63749 as ::core::ffi::c_int as png_uint_16,
    63807 as ::core::ffi::c_int as png_uint_16,
    63865 as ::core::ffi::c_int as png_uint_16,
    63923 as ::core::ffi::c_int as png_uint_16,
    63981 as ::core::ffi::c_int as png_uint_16,
    64039 as ::core::ffi::c_int as png_uint_16,
    64097 as ::core::ffi::c_int as png_uint_16,
    64155 as ::core::ffi::c_int as png_uint_16,
    64212 as ::core::ffi::c_int as png_uint_16,
    64270 as ::core::ffi::c_int as png_uint_16,
    64328 as ::core::ffi::c_int as png_uint_16,
    64385 as ::core::ffi::c_int as png_uint_16,
    64443 as ::core::ffi::c_int as png_uint_16,
    64500 as ::core::ffi::c_int as png_uint_16,
    64557 as ::core::ffi::c_int as png_uint_16,
    64614 as ::core::ffi::c_int as png_uint_16,
    64672 as ::core::ffi::c_int as png_uint_16,
    64729 as ::core::ffi::c_int as png_uint_16,
    64786 as ::core::ffi::c_int as png_uint_16,
    64843 as ::core::ffi::c_int as png_uint_16,
    64900 as ::core::ffi::c_int as png_uint_16,
    64956 as ::core::ffi::c_int as png_uint_16,
    65013 as ::core::ffi::c_int as png_uint_16,
    65070 as ::core::ffi::c_int as png_uint_16,
    65126 as ::core::ffi::c_int as png_uint_16,
    65183 as ::core::ffi::c_int as png_uint_16,
    65239 as ::core::ffi::c_int as png_uint_16,
    65296 as ::core::ffi::c_int as png_uint_16,
    65352 as ::core::ffi::c_int as png_uint_16,
    65409 as ::core::ffi::c_int as png_uint_16,
    65465 as ::core::ffi::c_int as png_uint_16,
];
#[unsafe(no_mangle)]
pub static png_sRGB_delta: [png_byte; 512] = [
    207 as ::core::ffi::c_int as png_byte,
    201 as ::core::ffi::c_int as png_byte,
    158 as ::core::ffi::c_int as png_byte,
    129 as ::core::ffi::c_int as png_byte,
    113 as ::core::ffi::c_int as png_byte,
    100 as ::core::ffi::c_int as png_byte,
    90 as ::core::ffi::c_int as png_byte,
    82 as ::core::ffi::c_int as png_byte,
    77 as ::core::ffi::c_int as png_byte,
    72 as ::core::ffi::c_int as png_byte,
    68 as ::core::ffi::c_int as png_byte,
    64 as ::core::ffi::c_int as png_byte,
    61 as ::core::ffi::c_int as png_byte,
    59 as ::core::ffi::c_int as png_byte,
    56 as ::core::ffi::c_int as png_byte,
    54 as ::core::ffi::c_int as png_byte,
    52 as ::core::ffi::c_int as png_byte,
    50 as ::core::ffi::c_int as png_byte,
    49 as ::core::ffi::c_int as png_byte,
    47 as ::core::ffi::c_int as png_byte,
    46 as ::core::ffi::c_int as png_byte,
    45 as ::core::ffi::c_int as png_byte,
    43 as ::core::ffi::c_int as png_byte,
    42 as ::core::ffi::c_int as png_byte,
    41 as ::core::ffi::c_int as png_byte,
    40 as ::core::ffi::c_int as png_byte,
    39 as ::core::ffi::c_int as png_byte,
    39 as ::core::ffi::c_int as png_byte,
    38 as ::core::ffi::c_int as png_byte,
    37 as ::core::ffi::c_int as png_byte,
    36 as ::core::ffi::c_int as png_byte,
    36 as ::core::ffi::c_int as png_byte,
    35 as ::core::ffi::c_int as png_byte,
    34 as ::core::ffi::c_int as png_byte,
    34 as ::core::ffi::c_int as png_byte,
    33 as ::core::ffi::c_int as png_byte,
    33 as ::core::ffi::c_int as png_byte,
    32 as ::core::ffi::c_int as png_byte,
    32 as ::core::ffi::c_int as png_byte,
    31 as ::core::ffi::c_int as png_byte,
    31 as ::core::ffi::c_int as png_byte,
    30 as ::core::ffi::c_int as png_byte,
    30 as ::core::ffi::c_int as png_byte,
    30 as ::core::ffi::c_int as png_byte,
    29 as ::core::ffi::c_int as png_byte,
    29 as ::core::ffi::c_int as png_byte,
    28 as ::core::ffi::c_int as png_byte,
    28 as ::core::ffi::c_int as png_byte,
    28 as ::core::ffi::c_int as png_byte,
    27 as ::core::ffi::c_int as png_byte,
    27 as ::core::ffi::c_int as png_byte,
    27 as ::core::ffi::c_int as png_byte,
    27 as ::core::ffi::c_int as png_byte,
    26 as ::core::ffi::c_int as png_byte,
    26 as ::core::ffi::c_int as png_byte,
    26 as ::core::ffi::c_int as png_byte,
    25 as ::core::ffi::c_int as png_byte,
    25 as ::core::ffi::c_int as png_byte,
    25 as ::core::ffi::c_int as png_byte,
    25 as ::core::ffi::c_int as png_byte,
    24 as ::core::ffi::c_int as png_byte,
    24 as ::core::ffi::c_int as png_byte,
    24 as ::core::ffi::c_int as png_byte,
    24 as ::core::ffi::c_int as png_byte,
    23 as ::core::ffi::c_int as png_byte,
    23 as ::core::ffi::c_int as png_byte,
    23 as ::core::ffi::c_int as png_byte,
    23 as ::core::ffi::c_int as png_byte,
    23 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    22 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    21 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    20 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    19 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    18 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    17 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    16 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    15 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    14 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    13 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    12 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    11 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    10 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    9 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    8 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
    7 as ::core::ffi::c_int as png_byte,
];
unsafe extern "C" fn png_image_free_function(mut argument: png_voidp) -> ::core::ffi::c_int {
    let mut image: png_imagep = argument as png_imagep;
    let mut cp: png_controlp = (*image).opaque;
    let mut c: png_control = png_control {
        png_ptr: ::core::ptr::null_mut::<png_struct>(),
        info_ptr: ::core::ptr::null_mut::<png_info>(),
        error_buf: ::core::ptr::null_mut::<::core::ffi::c_void>(),
        memory: ::core::ptr::null::<png_byte>(),
        size: 0,
        for_write_owned_file: [0; 1],
        c2rust_padding: [0; 7],
    };
    if (*cp).png_ptr.is_null() {
        return 0 as ::core::ffi::c_int;
    }
    if (*cp).owned_file() as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        let mut fp: *mut FILE = (*(*cp).png_ptr).io_ptr as *mut FILE;
        (*cp).set_owned_file(0 as ::core::ffi::c_uint as ::core::ffi::c_uint);
        if !fp.is_null() {
            (*(*cp).png_ptr).io_ptr = NULL_0 as png_voidp;
            fclose(fp);
        }
    }
    c = *cp as png_control;
    (*image).opaque = &raw mut c as png_controlp;
    png_free(c.png_ptr as png_const_structrp, cp as png_voidp);
    if c.for_write() as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        png_destroy_write_struct(&raw mut c.png_ptr, &raw mut c.info_ptr);
    } else {
        png_destroy_read_struct(
            &raw mut c.png_ptr,
            &raw mut c.info_ptr,
            ::core::ptr::null_mut::<*mut png_info>(),
        );
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_free(mut image: png_imagep) {
    if !image.is_null() && !(*image).opaque.is_null() && (*(*image).opaque).error_buf.is_null() {
        png_image_free_function(image as png_voidp);
        (*image).opaque = ::core::ptr::null_mut::<png_control>();
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_image_error(
    mut image: png_imagep,
    mut error_message: png_const_charp,
) -> ::core::ffi::c_int {
    png_safecat(
        &raw mut (*image).message as png_charp,
        ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
        0 as size_t,
        error_message,
    );
    (*image).warning_or_error |= PNG_IMAGE_ERROR as ::core::ffi::c_uint;
    png_image_free(image);
    return 0 as ::core::ffi::c_int;
}
pub const PNG_HAVE_PNG_SIGNATURE: ::core::ffi::c_uint = 0x1000 as ::core::ffi::c_uint;
pub const PNG_COMPOSE: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
pub const PNG_16_TO_8: ::core::ffi::c_uint = 0x400 as ::core::ffi::c_uint;
pub const PNG_RGB_TO_GRAY: ::core::ffi::c_uint = 0x600000 as ::core::ffi::c_uint;
pub const PNG_SCALE_16_TO_8: ::core::ffi::c_uint = 0x4000000 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_USE: ::core::ffi::c_uint = 0x100 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_NOWARN: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_CRITICAL_IGNORE: ::core::ffi::c_uint = 0x800 as ::core::ffi::c_uint;
pub const PNG_FLAG_LIBRARY_MISMATCH: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_FLAG_CRC_ANCILLARY_MASK: ::core::ffi::c_uint =
    PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
pub const PNG_UNEXPECTED_ZLIB_RETURN: ::core::ffi::c_int = -7;
pub const PNG_NUMBER_FORMAT_x: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PNG_FP_INTEGER: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_FP_FRACTION: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_FP_EXPONENT: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_FP_STATE: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PNG_FP_SAW_SIGN: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_FP_SAW_DIGIT: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const PNG_FP_SAW_DOT: ::core::ffi::c_int = 16 as ::core::ffi::c_int;
pub const PNG_FP_SAW_E: ::core::ffi::c_int = 32 as ::core::ffi::c_int;
pub const PNG_FP_SAW_ANY: ::core::ffi::c_int = 60 as ::core::ffi::c_int;
pub const PNG_FP_NEGATIVE: ::core::ffi::c_int = 128 as ::core::ffi::c_int;
pub const PNG_FP_NONZERO: ::core::ffi::c_int = 256 as ::core::ffi::c_int;
pub const PNG_FP_STICKY: ::core::ffi::c_int = 448 as ::core::ffi::c_int;
pub const __DBL_DIG__: ::core::ffi::c_int = 15 as ::core::ffi::c_int;
pub const __DBL_MAX__: ::core::ffi::c_double = 1.7976931348623157e+308f64;
pub const __DBL_MIN_10_EXP__: ::core::ffi::c_int = -(307 as ::core::ffi::c_int);
pub const __DBL_MIN__: ::core::ffi::c_double = 2.2250738585072014e-308f64;
