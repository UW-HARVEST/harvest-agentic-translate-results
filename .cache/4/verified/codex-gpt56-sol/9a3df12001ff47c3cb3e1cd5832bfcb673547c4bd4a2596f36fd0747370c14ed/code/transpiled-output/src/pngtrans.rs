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
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp);
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
pub type png_const_color_8p = *const png_color_8;
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
pub const PNG_UINT_32_MAX: png_uint_32 = -(1 as ::core::ffi::c_int) as png_uint_32;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_GRAY: ::core::ffi::c_int = 0;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_FILLER_AFTER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bgr(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_BGR;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        (*png_ptr).transformations |= PNG_SWAP_BYTES;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packing(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    if ((*png_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
        (*png_ptr).transformations |= PNG_PACK;
        (*png_ptr).usr_bit_depth = 8 as png_byte;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_packswap(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    if ((*png_ptr).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
        (*png_ptr).transformations |= PNG_PACKSWAP;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_shift(
    mut png_ptr: png_structrp,
    mut true_bits: png_const_color_8p,
) {
    if png_ptr.is_null() || true_bits.is_null() {
        return;
    }
    let mut bit_depth: png_byte = (*png_ptr).bit_depth;
    let mut invalid: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR != 0 as ::core::ffi::c_int
    {
        if (*true_bits).red as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*true_bits).red as ::core::ffi::c_int > bit_depth as ::core::ffi::c_int
            || (*true_bits).green as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*true_bits).green as ::core::ffi::c_int > bit_depth as ::core::ffi::c_int
            || (*true_bits).blue as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*true_bits).blue as ::core::ffi::c_int > bit_depth as ::core::ffi::c_int
        {
            invalid = 1 as ::core::ffi::c_int;
        }
    } else if (*true_bits).gray as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*true_bits).gray as ::core::ffi::c_int > bit_depth as ::core::ffi::c_int
    {
        invalid = 1 as ::core::ffi::c_int;
    }
    if (*png_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA != 0 as ::core::ffi::c_int
        && ((*true_bits).alpha as ::core::ffi::c_int == 0 as ::core::ffi::c_int
            || (*true_bits).alpha as ::core::ffi::c_int > bit_depth as ::core::ffi::c_int)
    {
        invalid = 1 as ::core::ffi::c_int;
    }
    if invalid != 0 {
        png_app_error(
            png_ptr,
            b"png_set_shift: invalid shift values\0" as *const u8 as png_const_charp,
        );
        return;
    }
    (*png_ptr).transformations |= PNG_SHIFT;
    (*png_ptr).shift = *true_bits;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_interlace_handling(
    mut png_ptr: png_structrp,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() && (*png_ptr).interlaced as ::core::ffi::c_int != 0 as ::core::ffi::c_int
    {
        (*png_ptr).transformations |= PNG_INTERLACE;
        return 7 as ::core::ffi::c_int;
    }
    return 1 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_filler(
    mut png_ptr: png_structrp,
    mut filler: png_uint_32,
    mut filler_loc: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint {
        (*png_ptr).filler = filler as png_uint_16;
    } else {
        match (*png_ptr).color_type as ::core::ffi::c_int {
            PNG_COLOR_TYPE_RGB => {
                (*png_ptr).usr_channels = 4 as png_byte;
            }
            PNG_COLOR_TYPE_GRAY => {
                if (*png_ptr).bit_depth as ::core::ffi::c_int >= 8 as ::core::ffi::c_int {
                    (*png_ptr).usr_channels = 2 as png_byte;
                } else {
                    png_app_error(
                        png_ptr,
                        b"png_set_filler is invalid for low bit depth gray output\0" as *const u8
                            as png_const_charp,
                    );
                    return;
                }
            }
            _ => {
                png_app_error(
                    png_ptr,
                    b"png_set_filler: inappropriate color type\0" as *const u8 as png_const_charp,
                );
                return;
            }
        }
    }
    (*png_ptr).transformations |= PNG_FILLER;
    if filler_loc == PNG_FILLER_AFTER {
        (*png_ptr).flags |= PNG_FLAG_FILLER_AFTER;
    } else {
        (*png_ptr).flags &= !PNG_FLAG_FILLER_AFTER;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_add_alpha(
    mut png_ptr: png_structrp,
    mut filler: png_uint_32,
    mut filler_loc: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    png_set_filler(png_ptr, filler, filler_loc);
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_FILLER != 0 as ::core::ffi::c_uint {
        (*png_ptr).transformations |= PNG_ADD_ALPHA;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_swap_alpha(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_SWAP_ALPHA;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_alpha(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_INVERT_ALPHA;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invert_mono(mut png_ptr: png_structrp) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).transformations |= PNG_INVERT_MONO;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_invert(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY {
        let mut rp: png_bytep = row;
        let mut i: size_t = 0;
        let mut istop: size_t = (*row_info).rowbytes;
        i = 0 as size_t;
        while i < istop {
            *rp = !(*rp as ::core::ffi::c_int) as png_byte;
            rp = rp.offset(1);
            i = i.wrapping_add(1);
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
    {
        let mut rp_0: png_bytep = row;
        let mut i_0: size_t = 0;
        let mut istop_0: size_t = (*row_info).rowbytes;
        i_0 = 0 as size_t;
        while i_0 < istop_0 {
            *rp_0 = !(*rp_0 as ::core::ffi::c_int) as png_byte;
            rp_0 = rp_0.offset(2 as ::core::ffi::c_int as isize);
            i_0 = (i_0 as ::core::ffi::c_ulong).wrapping_add(2 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA
        && (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int
    {
        let mut rp_1: png_bytep = row;
        let mut i_1: size_t = 0;
        let mut istop_1: size_t = (*row_info).rowbytes;
        i_1 = 0 as size_t;
        while i_1 < istop_1 {
            *rp_1 = !(*rp_1 as ::core::ffi::c_int) as png_byte;
            *rp_1.offset(1 as ::core::ffi::c_int as isize) =
                !(*rp_1.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int) as png_byte;
            rp_1 = rp_1.offset(4 as ::core::ffi::c_int as isize);
            i_1 = (i_1 as ::core::ffi::c_ulong).wrapping_add(4 as ::core::ffi::c_ulong) as size_t
                as size_t;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_swap(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
        let mut rp: png_bytep = row;
        let mut i: png_uint_32 = 0;
        let mut istop: png_uint_32 = (*row_info)
            .width
            .wrapping_mul((*row_info).channels as png_uint_32);
        i = 0 as png_uint_32;
        while i < istop {
            let mut t: png_byte = *rp;
            *rp = *rp.offset(1 as ::core::ffi::c_int as isize);
            *rp.offset(1 as ::core::ffi::c_int as isize) = t;
            i = i.wrapping_add(1);
            rp = rp.offset(2 as ::core::ffi::c_int as isize);
        }
    }
}
static mut onebppswaptable: [png_byte; 256] = [
    0 as ::core::ffi::c_int as png_byte,
    0x80 as ::core::ffi::c_int as png_byte,
    0x40 as ::core::ffi::c_int as png_byte,
    0xc0 as ::core::ffi::c_int as png_byte,
    0x20 as ::core::ffi::c_int as png_byte,
    0xa0 as ::core::ffi::c_int as png_byte,
    0x60 as ::core::ffi::c_int as png_byte,
    0xe0 as ::core::ffi::c_int as png_byte,
    0x10 as ::core::ffi::c_int as png_byte,
    0x90 as ::core::ffi::c_int as png_byte,
    0x50 as ::core::ffi::c_int as png_byte,
    0xd0 as ::core::ffi::c_int as png_byte,
    0x30 as ::core::ffi::c_int as png_byte,
    0xb0 as ::core::ffi::c_int as png_byte,
    0x70 as ::core::ffi::c_int as png_byte,
    0xf0 as ::core::ffi::c_int as png_byte,
    0x8 as ::core::ffi::c_int as png_byte,
    0x88 as ::core::ffi::c_int as png_byte,
    0x48 as ::core::ffi::c_int as png_byte,
    0xc8 as ::core::ffi::c_int as png_byte,
    0x28 as ::core::ffi::c_int as png_byte,
    0xa8 as ::core::ffi::c_int as png_byte,
    0x68 as ::core::ffi::c_int as png_byte,
    0xe8 as ::core::ffi::c_int as png_byte,
    0x18 as ::core::ffi::c_int as png_byte,
    0x98 as ::core::ffi::c_int as png_byte,
    0x58 as ::core::ffi::c_int as png_byte,
    0xd8 as ::core::ffi::c_int as png_byte,
    0x38 as ::core::ffi::c_int as png_byte,
    0xb8 as ::core::ffi::c_int as png_byte,
    0x78 as ::core::ffi::c_int as png_byte,
    0xf8 as ::core::ffi::c_int as png_byte,
    0x4 as ::core::ffi::c_int as png_byte,
    0x84 as ::core::ffi::c_int as png_byte,
    0x44 as ::core::ffi::c_int as png_byte,
    0xc4 as ::core::ffi::c_int as png_byte,
    0x24 as ::core::ffi::c_int as png_byte,
    0xa4 as ::core::ffi::c_int as png_byte,
    0x64 as ::core::ffi::c_int as png_byte,
    0xe4 as ::core::ffi::c_int as png_byte,
    0x14 as ::core::ffi::c_int as png_byte,
    0x94 as ::core::ffi::c_int as png_byte,
    0x54 as ::core::ffi::c_int as png_byte,
    0xd4 as ::core::ffi::c_int as png_byte,
    0x34 as ::core::ffi::c_int as png_byte,
    0xb4 as ::core::ffi::c_int as png_byte,
    0x74 as ::core::ffi::c_int as png_byte,
    0xf4 as ::core::ffi::c_int as png_byte,
    0xc as ::core::ffi::c_int as png_byte,
    0x8c as ::core::ffi::c_int as png_byte,
    0x4c as ::core::ffi::c_int as png_byte,
    0xcc as ::core::ffi::c_int as png_byte,
    0x2c as ::core::ffi::c_int as png_byte,
    0xac as ::core::ffi::c_int as png_byte,
    0x6c as ::core::ffi::c_int as png_byte,
    0xec as ::core::ffi::c_int as png_byte,
    0x1c as ::core::ffi::c_int as png_byte,
    0x9c as ::core::ffi::c_int as png_byte,
    0x5c as ::core::ffi::c_int as png_byte,
    0xdc as ::core::ffi::c_int as png_byte,
    0x3c as ::core::ffi::c_int as png_byte,
    0xbc as ::core::ffi::c_int as png_byte,
    0x7c as ::core::ffi::c_int as png_byte,
    0xfc as ::core::ffi::c_int as png_byte,
    0x2 as ::core::ffi::c_int as png_byte,
    0x82 as ::core::ffi::c_int as png_byte,
    0x42 as ::core::ffi::c_int as png_byte,
    0xc2 as ::core::ffi::c_int as png_byte,
    0x22 as ::core::ffi::c_int as png_byte,
    0xa2 as ::core::ffi::c_int as png_byte,
    0x62 as ::core::ffi::c_int as png_byte,
    0xe2 as ::core::ffi::c_int as png_byte,
    0x12 as ::core::ffi::c_int as png_byte,
    0x92 as ::core::ffi::c_int as png_byte,
    0x52 as ::core::ffi::c_int as png_byte,
    0xd2 as ::core::ffi::c_int as png_byte,
    0x32 as ::core::ffi::c_int as png_byte,
    0xb2 as ::core::ffi::c_int as png_byte,
    0x72 as ::core::ffi::c_int as png_byte,
    0xf2 as ::core::ffi::c_int as png_byte,
    0xa as ::core::ffi::c_int as png_byte,
    0x8a as ::core::ffi::c_int as png_byte,
    0x4a as ::core::ffi::c_int as png_byte,
    0xca as ::core::ffi::c_int as png_byte,
    0x2a as ::core::ffi::c_int as png_byte,
    0xaa as ::core::ffi::c_int as png_byte,
    0x6a as ::core::ffi::c_int as png_byte,
    0xea as ::core::ffi::c_int as png_byte,
    0x1a as ::core::ffi::c_int as png_byte,
    0x9a as ::core::ffi::c_int as png_byte,
    0x5a as ::core::ffi::c_int as png_byte,
    0xda as ::core::ffi::c_int as png_byte,
    0x3a as ::core::ffi::c_int as png_byte,
    0xba as ::core::ffi::c_int as png_byte,
    0x7a as ::core::ffi::c_int as png_byte,
    0xfa as ::core::ffi::c_int as png_byte,
    0x6 as ::core::ffi::c_int as png_byte,
    0x86 as ::core::ffi::c_int as png_byte,
    0x46 as ::core::ffi::c_int as png_byte,
    0xc6 as ::core::ffi::c_int as png_byte,
    0x26 as ::core::ffi::c_int as png_byte,
    0xa6 as ::core::ffi::c_int as png_byte,
    0x66 as ::core::ffi::c_int as png_byte,
    0xe6 as ::core::ffi::c_int as png_byte,
    0x16 as ::core::ffi::c_int as png_byte,
    0x96 as ::core::ffi::c_int as png_byte,
    0x56 as ::core::ffi::c_int as png_byte,
    0xd6 as ::core::ffi::c_int as png_byte,
    0x36 as ::core::ffi::c_int as png_byte,
    0xb6 as ::core::ffi::c_int as png_byte,
    0x76 as ::core::ffi::c_int as png_byte,
    0xf6 as ::core::ffi::c_int as png_byte,
    0xe as ::core::ffi::c_int as png_byte,
    0x8e as ::core::ffi::c_int as png_byte,
    0x4e as ::core::ffi::c_int as png_byte,
    0xce as ::core::ffi::c_int as png_byte,
    0x2e as ::core::ffi::c_int as png_byte,
    0xae as ::core::ffi::c_int as png_byte,
    0x6e as ::core::ffi::c_int as png_byte,
    0xee as ::core::ffi::c_int as png_byte,
    0x1e as ::core::ffi::c_int as png_byte,
    0x9e as ::core::ffi::c_int as png_byte,
    0x5e as ::core::ffi::c_int as png_byte,
    0xde as ::core::ffi::c_int as png_byte,
    0x3e as ::core::ffi::c_int as png_byte,
    0xbe as ::core::ffi::c_int as png_byte,
    0x7e as ::core::ffi::c_int as png_byte,
    0xfe as ::core::ffi::c_int as png_byte,
    0x1 as ::core::ffi::c_int as png_byte,
    0x81 as ::core::ffi::c_int as png_byte,
    0x41 as ::core::ffi::c_int as png_byte,
    0xc1 as ::core::ffi::c_int as png_byte,
    0x21 as ::core::ffi::c_int as png_byte,
    0xa1 as ::core::ffi::c_int as png_byte,
    0x61 as ::core::ffi::c_int as png_byte,
    0xe1 as ::core::ffi::c_int as png_byte,
    0x11 as ::core::ffi::c_int as png_byte,
    0x91 as ::core::ffi::c_int as png_byte,
    0x51 as ::core::ffi::c_int as png_byte,
    0xd1 as ::core::ffi::c_int as png_byte,
    0x31 as ::core::ffi::c_int as png_byte,
    0xb1 as ::core::ffi::c_int as png_byte,
    0x71 as ::core::ffi::c_int as png_byte,
    0xf1 as ::core::ffi::c_int as png_byte,
    0x9 as ::core::ffi::c_int as png_byte,
    0x89 as ::core::ffi::c_int as png_byte,
    0x49 as ::core::ffi::c_int as png_byte,
    0xc9 as ::core::ffi::c_int as png_byte,
    0x29 as ::core::ffi::c_int as png_byte,
    0xa9 as ::core::ffi::c_int as png_byte,
    0x69 as ::core::ffi::c_int as png_byte,
    0xe9 as ::core::ffi::c_int as png_byte,
    0x19 as ::core::ffi::c_int as png_byte,
    0x99 as ::core::ffi::c_int as png_byte,
    0x59 as ::core::ffi::c_int as png_byte,
    0xd9 as ::core::ffi::c_int as png_byte,
    0x39 as ::core::ffi::c_int as png_byte,
    0xb9 as ::core::ffi::c_int as png_byte,
    0x79 as ::core::ffi::c_int as png_byte,
    0xf9 as ::core::ffi::c_int as png_byte,
    0x5 as ::core::ffi::c_int as png_byte,
    0x85 as ::core::ffi::c_int as png_byte,
    0x45 as ::core::ffi::c_int as png_byte,
    0xc5 as ::core::ffi::c_int as png_byte,
    0x25 as ::core::ffi::c_int as png_byte,
    0xa5 as ::core::ffi::c_int as png_byte,
    0x65 as ::core::ffi::c_int as png_byte,
    0xe5 as ::core::ffi::c_int as png_byte,
    0x15 as ::core::ffi::c_int as png_byte,
    0x95 as ::core::ffi::c_int as png_byte,
    0x55 as ::core::ffi::c_int as png_byte,
    0xd5 as ::core::ffi::c_int as png_byte,
    0x35 as ::core::ffi::c_int as png_byte,
    0xb5 as ::core::ffi::c_int as png_byte,
    0x75 as ::core::ffi::c_int as png_byte,
    0xf5 as ::core::ffi::c_int as png_byte,
    0xd as ::core::ffi::c_int as png_byte,
    0x8d as ::core::ffi::c_int as png_byte,
    0x4d as ::core::ffi::c_int as png_byte,
    0xcd as ::core::ffi::c_int as png_byte,
    0x2d as ::core::ffi::c_int as png_byte,
    0xad as ::core::ffi::c_int as png_byte,
    0x6d as ::core::ffi::c_int as png_byte,
    0xed as ::core::ffi::c_int as png_byte,
    0x1d as ::core::ffi::c_int as png_byte,
    0x9d as ::core::ffi::c_int as png_byte,
    0x5d as ::core::ffi::c_int as png_byte,
    0xdd as ::core::ffi::c_int as png_byte,
    0x3d as ::core::ffi::c_int as png_byte,
    0xbd as ::core::ffi::c_int as png_byte,
    0x7d as ::core::ffi::c_int as png_byte,
    0xfd as ::core::ffi::c_int as png_byte,
    0x3 as ::core::ffi::c_int as png_byte,
    0x83 as ::core::ffi::c_int as png_byte,
    0x43 as ::core::ffi::c_int as png_byte,
    0xc3 as ::core::ffi::c_int as png_byte,
    0x23 as ::core::ffi::c_int as png_byte,
    0xa3 as ::core::ffi::c_int as png_byte,
    0x63 as ::core::ffi::c_int as png_byte,
    0xe3 as ::core::ffi::c_int as png_byte,
    0x13 as ::core::ffi::c_int as png_byte,
    0x93 as ::core::ffi::c_int as png_byte,
    0x53 as ::core::ffi::c_int as png_byte,
    0xd3 as ::core::ffi::c_int as png_byte,
    0x33 as ::core::ffi::c_int as png_byte,
    0xb3 as ::core::ffi::c_int as png_byte,
    0x73 as ::core::ffi::c_int as png_byte,
    0xf3 as ::core::ffi::c_int as png_byte,
    0xb as ::core::ffi::c_int as png_byte,
    0x8b as ::core::ffi::c_int as png_byte,
    0x4b as ::core::ffi::c_int as png_byte,
    0xcb as ::core::ffi::c_int as png_byte,
    0x2b as ::core::ffi::c_int as png_byte,
    0xab as ::core::ffi::c_int as png_byte,
    0x6b as ::core::ffi::c_int as png_byte,
    0xeb as ::core::ffi::c_int as png_byte,
    0x1b as ::core::ffi::c_int as png_byte,
    0x9b as ::core::ffi::c_int as png_byte,
    0x5b as ::core::ffi::c_int as png_byte,
    0xdb as ::core::ffi::c_int as png_byte,
    0x3b as ::core::ffi::c_int as png_byte,
    0xbb as ::core::ffi::c_int as png_byte,
    0x7b as ::core::ffi::c_int as png_byte,
    0xfb as ::core::ffi::c_int as png_byte,
    0x7 as ::core::ffi::c_int as png_byte,
    0x87 as ::core::ffi::c_int as png_byte,
    0x47 as ::core::ffi::c_int as png_byte,
    0xc7 as ::core::ffi::c_int as png_byte,
    0x27 as ::core::ffi::c_int as png_byte,
    0xa7 as ::core::ffi::c_int as png_byte,
    0x67 as ::core::ffi::c_int as png_byte,
    0xe7 as ::core::ffi::c_int as png_byte,
    0x17 as ::core::ffi::c_int as png_byte,
    0x97 as ::core::ffi::c_int as png_byte,
    0x57 as ::core::ffi::c_int as png_byte,
    0xd7 as ::core::ffi::c_int as png_byte,
    0x37 as ::core::ffi::c_int as png_byte,
    0xb7 as ::core::ffi::c_int as png_byte,
    0x77 as ::core::ffi::c_int as png_byte,
    0xf7 as ::core::ffi::c_int as png_byte,
    0xf as ::core::ffi::c_int as png_byte,
    0x8f as ::core::ffi::c_int as png_byte,
    0x4f as ::core::ffi::c_int as png_byte,
    0xcf as ::core::ffi::c_int as png_byte,
    0x2f as ::core::ffi::c_int as png_byte,
    0xaf as ::core::ffi::c_int as png_byte,
    0x6f as ::core::ffi::c_int as png_byte,
    0xef as ::core::ffi::c_int as png_byte,
    0x1f as ::core::ffi::c_int as png_byte,
    0x9f as ::core::ffi::c_int as png_byte,
    0x5f as ::core::ffi::c_int as png_byte,
    0xdf as ::core::ffi::c_int as png_byte,
    0x3f as ::core::ffi::c_int as png_byte,
    0xbf as ::core::ffi::c_int as png_byte,
    0x7f as ::core::ffi::c_int as png_byte,
    0xff as ::core::ffi::c_int as png_byte,
];
static mut twobppswaptable: [png_byte; 256] = [
    0 as ::core::ffi::c_int as png_byte,
    0x40 as ::core::ffi::c_int as png_byte,
    0x80 as ::core::ffi::c_int as png_byte,
    0xc0 as ::core::ffi::c_int as png_byte,
    0x10 as ::core::ffi::c_int as png_byte,
    0x50 as ::core::ffi::c_int as png_byte,
    0x90 as ::core::ffi::c_int as png_byte,
    0xd0 as ::core::ffi::c_int as png_byte,
    0x20 as ::core::ffi::c_int as png_byte,
    0x60 as ::core::ffi::c_int as png_byte,
    0xa0 as ::core::ffi::c_int as png_byte,
    0xe0 as ::core::ffi::c_int as png_byte,
    0x30 as ::core::ffi::c_int as png_byte,
    0x70 as ::core::ffi::c_int as png_byte,
    0xb0 as ::core::ffi::c_int as png_byte,
    0xf0 as ::core::ffi::c_int as png_byte,
    0x4 as ::core::ffi::c_int as png_byte,
    0x44 as ::core::ffi::c_int as png_byte,
    0x84 as ::core::ffi::c_int as png_byte,
    0xc4 as ::core::ffi::c_int as png_byte,
    0x14 as ::core::ffi::c_int as png_byte,
    0x54 as ::core::ffi::c_int as png_byte,
    0x94 as ::core::ffi::c_int as png_byte,
    0xd4 as ::core::ffi::c_int as png_byte,
    0x24 as ::core::ffi::c_int as png_byte,
    0x64 as ::core::ffi::c_int as png_byte,
    0xa4 as ::core::ffi::c_int as png_byte,
    0xe4 as ::core::ffi::c_int as png_byte,
    0x34 as ::core::ffi::c_int as png_byte,
    0x74 as ::core::ffi::c_int as png_byte,
    0xb4 as ::core::ffi::c_int as png_byte,
    0xf4 as ::core::ffi::c_int as png_byte,
    0x8 as ::core::ffi::c_int as png_byte,
    0x48 as ::core::ffi::c_int as png_byte,
    0x88 as ::core::ffi::c_int as png_byte,
    0xc8 as ::core::ffi::c_int as png_byte,
    0x18 as ::core::ffi::c_int as png_byte,
    0x58 as ::core::ffi::c_int as png_byte,
    0x98 as ::core::ffi::c_int as png_byte,
    0xd8 as ::core::ffi::c_int as png_byte,
    0x28 as ::core::ffi::c_int as png_byte,
    0x68 as ::core::ffi::c_int as png_byte,
    0xa8 as ::core::ffi::c_int as png_byte,
    0xe8 as ::core::ffi::c_int as png_byte,
    0x38 as ::core::ffi::c_int as png_byte,
    0x78 as ::core::ffi::c_int as png_byte,
    0xb8 as ::core::ffi::c_int as png_byte,
    0xf8 as ::core::ffi::c_int as png_byte,
    0xc as ::core::ffi::c_int as png_byte,
    0x4c as ::core::ffi::c_int as png_byte,
    0x8c as ::core::ffi::c_int as png_byte,
    0xcc as ::core::ffi::c_int as png_byte,
    0x1c as ::core::ffi::c_int as png_byte,
    0x5c as ::core::ffi::c_int as png_byte,
    0x9c as ::core::ffi::c_int as png_byte,
    0xdc as ::core::ffi::c_int as png_byte,
    0x2c as ::core::ffi::c_int as png_byte,
    0x6c as ::core::ffi::c_int as png_byte,
    0xac as ::core::ffi::c_int as png_byte,
    0xec as ::core::ffi::c_int as png_byte,
    0x3c as ::core::ffi::c_int as png_byte,
    0x7c as ::core::ffi::c_int as png_byte,
    0xbc as ::core::ffi::c_int as png_byte,
    0xfc as ::core::ffi::c_int as png_byte,
    0x1 as ::core::ffi::c_int as png_byte,
    0x41 as ::core::ffi::c_int as png_byte,
    0x81 as ::core::ffi::c_int as png_byte,
    0xc1 as ::core::ffi::c_int as png_byte,
    0x11 as ::core::ffi::c_int as png_byte,
    0x51 as ::core::ffi::c_int as png_byte,
    0x91 as ::core::ffi::c_int as png_byte,
    0xd1 as ::core::ffi::c_int as png_byte,
    0x21 as ::core::ffi::c_int as png_byte,
    0x61 as ::core::ffi::c_int as png_byte,
    0xa1 as ::core::ffi::c_int as png_byte,
    0xe1 as ::core::ffi::c_int as png_byte,
    0x31 as ::core::ffi::c_int as png_byte,
    0x71 as ::core::ffi::c_int as png_byte,
    0xb1 as ::core::ffi::c_int as png_byte,
    0xf1 as ::core::ffi::c_int as png_byte,
    0x5 as ::core::ffi::c_int as png_byte,
    0x45 as ::core::ffi::c_int as png_byte,
    0x85 as ::core::ffi::c_int as png_byte,
    0xc5 as ::core::ffi::c_int as png_byte,
    0x15 as ::core::ffi::c_int as png_byte,
    0x55 as ::core::ffi::c_int as png_byte,
    0x95 as ::core::ffi::c_int as png_byte,
    0xd5 as ::core::ffi::c_int as png_byte,
    0x25 as ::core::ffi::c_int as png_byte,
    0x65 as ::core::ffi::c_int as png_byte,
    0xa5 as ::core::ffi::c_int as png_byte,
    0xe5 as ::core::ffi::c_int as png_byte,
    0x35 as ::core::ffi::c_int as png_byte,
    0x75 as ::core::ffi::c_int as png_byte,
    0xb5 as ::core::ffi::c_int as png_byte,
    0xf5 as ::core::ffi::c_int as png_byte,
    0x9 as ::core::ffi::c_int as png_byte,
    0x49 as ::core::ffi::c_int as png_byte,
    0x89 as ::core::ffi::c_int as png_byte,
    0xc9 as ::core::ffi::c_int as png_byte,
    0x19 as ::core::ffi::c_int as png_byte,
    0x59 as ::core::ffi::c_int as png_byte,
    0x99 as ::core::ffi::c_int as png_byte,
    0xd9 as ::core::ffi::c_int as png_byte,
    0x29 as ::core::ffi::c_int as png_byte,
    0x69 as ::core::ffi::c_int as png_byte,
    0xa9 as ::core::ffi::c_int as png_byte,
    0xe9 as ::core::ffi::c_int as png_byte,
    0x39 as ::core::ffi::c_int as png_byte,
    0x79 as ::core::ffi::c_int as png_byte,
    0xb9 as ::core::ffi::c_int as png_byte,
    0xf9 as ::core::ffi::c_int as png_byte,
    0xd as ::core::ffi::c_int as png_byte,
    0x4d as ::core::ffi::c_int as png_byte,
    0x8d as ::core::ffi::c_int as png_byte,
    0xcd as ::core::ffi::c_int as png_byte,
    0x1d as ::core::ffi::c_int as png_byte,
    0x5d as ::core::ffi::c_int as png_byte,
    0x9d as ::core::ffi::c_int as png_byte,
    0xdd as ::core::ffi::c_int as png_byte,
    0x2d as ::core::ffi::c_int as png_byte,
    0x6d as ::core::ffi::c_int as png_byte,
    0xad as ::core::ffi::c_int as png_byte,
    0xed as ::core::ffi::c_int as png_byte,
    0x3d as ::core::ffi::c_int as png_byte,
    0x7d as ::core::ffi::c_int as png_byte,
    0xbd as ::core::ffi::c_int as png_byte,
    0xfd as ::core::ffi::c_int as png_byte,
    0x2 as ::core::ffi::c_int as png_byte,
    0x42 as ::core::ffi::c_int as png_byte,
    0x82 as ::core::ffi::c_int as png_byte,
    0xc2 as ::core::ffi::c_int as png_byte,
    0x12 as ::core::ffi::c_int as png_byte,
    0x52 as ::core::ffi::c_int as png_byte,
    0x92 as ::core::ffi::c_int as png_byte,
    0xd2 as ::core::ffi::c_int as png_byte,
    0x22 as ::core::ffi::c_int as png_byte,
    0x62 as ::core::ffi::c_int as png_byte,
    0xa2 as ::core::ffi::c_int as png_byte,
    0xe2 as ::core::ffi::c_int as png_byte,
    0x32 as ::core::ffi::c_int as png_byte,
    0x72 as ::core::ffi::c_int as png_byte,
    0xb2 as ::core::ffi::c_int as png_byte,
    0xf2 as ::core::ffi::c_int as png_byte,
    0x6 as ::core::ffi::c_int as png_byte,
    0x46 as ::core::ffi::c_int as png_byte,
    0x86 as ::core::ffi::c_int as png_byte,
    0xc6 as ::core::ffi::c_int as png_byte,
    0x16 as ::core::ffi::c_int as png_byte,
    0x56 as ::core::ffi::c_int as png_byte,
    0x96 as ::core::ffi::c_int as png_byte,
    0xd6 as ::core::ffi::c_int as png_byte,
    0x26 as ::core::ffi::c_int as png_byte,
    0x66 as ::core::ffi::c_int as png_byte,
    0xa6 as ::core::ffi::c_int as png_byte,
    0xe6 as ::core::ffi::c_int as png_byte,
    0x36 as ::core::ffi::c_int as png_byte,
    0x76 as ::core::ffi::c_int as png_byte,
    0xb6 as ::core::ffi::c_int as png_byte,
    0xf6 as ::core::ffi::c_int as png_byte,
    0xa as ::core::ffi::c_int as png_byte,
    0x4a as ::core::ffi::c_int as png_byte,
    0x8a as ::core::ffi::c_int as png_byte,
    0xca as ::core::ffi::c_int as png_byte,
    0x1a as ::core::ffi::c_int as png_byte,
    0x5a as ::core::ffi::c_int as png_byte,
    0x9a as ::core::ffi::c_int as png_byte,
    0xda as ::core::ffi::c_int as png_byte,
    0x2a as ::core::ffi::c_int as png_byte,
    0x6a as ::core::ffi::c_int as png_byte,
    0xaa as ::core::ffi::c_int as png_byte,
    0xea as ::core::ffi::c_int as png_byte,
    0x3a as ::core::ffi::c_int as png_byte,
    0x7a as ::core::ffi::c_int as png_byte,
    0xba as ::core::ffi::c_int as png_byte,
    0xfa as ::core::ffi::c_int as png_byte,
    0xe as ::core::ffi::c_int as png_byte,
    0x4e as ::core::ffi::c_int as png_byte,
    0x8e as ::core::ffi::c_int as png_byte,
    0xce as ::core::ffi::c_int as png_byte,
    0x1e as ::core::ffi::c_int as png_byte,
    0x5e as ::core::ffi::c_int as png_byte,
    0x9e as ::core::ffi::c_int as png_byte,
    0xde as ::core::ffi::c_int as png_byte,
    0x2e as ::core::ffi::c_int as png_byte,
    0x6e as ::core::ffi::c_int as png_byte,
    0xae as ::core::ffi::c_int as png_byte,
    0xee as ::core::ffi::c_int as png_byte,
    0x3e as ::core::ffi::c_int as png_byte,
    0x7e as ::core::ffi::c_int as png_byte,
    0xbe as ::core::ffi::c_int as png_byte,
    0xfe as ::core::ffi::c_int as png_byte,
    0x3 as ::core::ffi::c_int as png_byte,
    0x43 as ::core::ffi::c_int as png_byte,
    0x83 as ::core::ffi::c_int as png_byte,
    0xc3 as ::core::ffi::c_int as png_byte,
    0x13 as ::core::ffi::c_int as png_byte,
    0x53 as ::core::ffi::c_int as png_byte,
    0x93 as ::core::ffi::c_int as png_byte,
    0xd3 as ::core::ffi::c_int as png_byte,
    0x23 as ::core::ffi::c_int as png_byte,
    0x63 as ::core::ffi::c_int as png_byte,
    0xa3 as ::core::ffi::c_int as png_byte,
    0xe3 as ::core::ffi::c_int as png_byte,
    0x33 as ::core::ffi::c_int as png_byte,
    0x73 as ::core::ffi::c_int as png_byte,
    0xb3 as ::core::ffi::c_int as png_byte,
    0xf3 as ::core::ffi::c_int as png_byte,
    0x7 as ::core::ffi::c_int as png_byte,
    0x47 as ::core::ffi::c_int as png_byte,
    0x87 as ::core::ffi::c_int as png_byte,
    0xc7 as ::core::ffi::c_int as png_byte,
    0x17 as ::core::ffi::c_int as png_byte,
    0x57 as ::core::ffi::c_int as png_byte,
    0x97 as ::core::ffi::c_int as png_byte,
    0xd7 as ::core::ffi::c_int as png_byte,
    0x27 as ::core::ffi::c_int as png_byte,
    0x67 as ::core::ffi::c_int as png_byte,
    0xa7 as ::core::ffi::c_int as png_byte,
    0xe7 as ::core::ffi::c_int as png_byte,
    0x37 as ::core::ffi::c_int as png_byte,
    0x77 as ::core::ffi::c_int as png_byte,
    0xb7 as ::core::ffi::c_int as png_byte,
    0xf7 as ::core::ffi::c_int as png_byte,
    0xb as ::core::ffi::c_int as png_byte,
    0x4b as ::core::ffi::c_int as png_byte,
    0x8b as ::core::ffi::c_int as png_byte,
    0xcb as ::core::ffi::c_int as png_byte,
    0x1b as ::core::ffi::c_int as png_byte,
    0x5b as ::core::ffi::c_int as png_byte,
    0x9b as ::core::ffi::c_int as png_byte,
    0xdb as ::core::ffi::c_int as png_byte,
    0x2b as ::core::ffi::c_int as png_byte,
    0x6b as ::core::ffi::c_int as png_byte,
    0xab as ::core::ffi::c_int as png_byte,
    0xeb as ::core::ffi::c_int as png_byte,
    0x3b as ::core::ffi::c_int as png_byte,
    0x7b as ::core::ffi::c_int as png_byte,
    0xbb as ::core::ffi::c_int as png_byte,
    0xfb as ::core::ffi::c_int as png_byte,
    0xf as ::core::ffi::c_int as png_byte,
    0x4f as ::core::ffi::c_int as png_byte,
    0x8f as ::core::ffi::c_int as png_byte,
    0xcf as ::core::ffi::c_int as png_byte,
    0x1f as ::core::ffi::c_int as png_byte,
    0x5f as ::core::ffi::c_int as png_byte,
    0x9f as ::core::ffi::c_int as png_byte,
    0xdf as ::core::ffi::c_int as png_byte,
    0x2f as ::core::ffi::c_int as png_byte,
    0x6f as ::core::ffi::c_int as png_byte,
    0xaf as ::core::ffi::c_int as png_byte,
    0xef as ::core::ffi::c_int as png_byte,
    0x3f as ::core::ffi::c_int as png_byte,
    0x7f as ::core::ffi::c_int as png_byte,
    0xbf as ::core::ffi::c_int as png_byte,
    0xff as ::core::ffi::c_int as png_byte,
];
static mut fourbppswaptable: [png_byte; 256] = [
    0 as ::core::ffi::c_int as png_byte,
    0x10 as ::core::ffi::c_int as png_byte,
    0x20 as ::core::ffi::c_int as png_byte,
    0x30 as ::core::ffi::c_int as png_byte,
    0x40 as ::core::ffi::c_int as png_byte,
    0x50 as ::core::ffi::c_int as png_byte,
    0x60 as ::core::ffi::c_int as png_byte,
    0x70 as ::core::ffi::c_int as png_byte,
    0x80 as ::core::ffi::c_int as png_byte,
    0x90 as ::core::ffi::c_int as png_byte,
    0xa0 as ::core::ffi::c_int as png_byte,
    0xb0 as ::core::ffi::c_int as png_byte,
    0xc0 as ::core::ffi::c_int as png_byte,
    0xd0 as ::core::ffi::c_int as png_byte,
    0xe0 as ::core::ffi::c_int as png_byte,
    0xf0 as ::core::ffi::c_int as png_byte,
    0x1 as ::core::ffi::c_int as png_byte,
    0x11 as ::core::ffi::c_int as png_byte,
    0x21 as ::core::ffi::c_int as png_byte,
    0x31 as ::core::ffi::c_int as png_byte,
    0x41 as ::core::ffi::c_int as png_byte,
    0x51 as ::core::ffi::c_int as png_byte,
    0x61 as ::core::ffi::c_int as png_byte,
    0x71 as ::core::ffi::c_int as png_byte,
    0x81 as ::core::ffi::c_int as png_byte,
    0x91 as ::core::ffi::c_int as png_byte,
    0xa1 as ::core::ffi::c_int as png_byte,
    0xb1 as ::core::ffi::c_int as png_byte,
    0xc1 as ::core::ffi::c_int as png_byte,
    0xd1 as ::core::ffi::c_int as png_byte,
    0xe1 as ::core::ffi::c_int as png_byte,
    0xf1 as ::core::ffi::c_int as png_byte,
    0x2 as ::core::ffi::c_int as png_byte,
    0x12 as ::core::ffi::c_int as png_byte,
    0x22 as ::core::ffi::c_int as png_byte,
    0x32 as ::core::ffi::c_int as png_byte,
    0x42 as ::core::ffi::c_int as png_byte,
    0x52 as ::core::ffi::c_int as png_byte,
    0x62 as ::core::ffi::c_int as png_byte,
    0x72 as ::core::ffi::c_int as png_byte,
    0x82 as ::core::ffi::c_int as png_byte,
    0x92 as ::core::ffi::c_int as png_byte,
    0xa2 as ::core::ffi::c_int as png_byte,
    0xb2 as ::core::ffi::c_int as png_byte,
    0xc2 as ::core::ffi::c_int as png_byte,
    0xd2 as ::core::ffi::c_int as png_byte,
    0xe2 as ::core::ffi::c_int as png_byte,
    0xf2 as ::core::ffi::c_int as png_byte,
    0x3 as ::core::ffi::c_int as png_byte,
    0x13 as ::core::ffi::c_int as png_byte,
    0x23 as ::core::ffi::c_int as png_byte,
    0x33 as ::core::ffi::c_int as png_byte,
    0x43 as ::core::ffi::c_int as png_byte,
    0x53 as ::core::ffi::c_int as png_byte,
    0x63 as ::core::ffi::c_int as png_byte,
    0x73 as ::core::ffi::c_int as png_byte,
    0x83 as ::core::ffi::c_int as png_byte,
    0x93 as ::core::ffi::c_int as png_byte,
    0xa3 as ::core::ffi::c_int as png_byte,
    0xb3 as ::core::ffi::c_int as png_byte,
    0xc3 as ::core::ffi::c_int as png_byte,
    0xd3 as ::core::ffi::c_int as png_byte,
    0xe3 as ::core::ffi::c_int as png_byte,
    0xf3 as ::core::ffi::c_int as png_byte,
    0x4 as ::core::ffi::c_int as png_byte,
    0x14 as ::core::ffi::c_int as png_byte,
    0x24 as ::core::ffi::c_int as png_byte,
    0x34 as ::core::ffi::c_int as png_byte,
    0x44 as ::core::ffi::c_int as png_byte,
    0x54 as ::core::ffi::c_int as png_byte,
    0x64 as ::core::ffi::c_int as png_byte,
    0x74 as ::core::ffi::c_int as png_byte,
    0x84 as ::core::ffi::c_int as png_byte,
    0x94 as ::core::ffi::c_int as png_byte,
    0xa4 as ::core::ffi::c_int as png_byte,
    0xb4 as ::core::ffi::c_int as png_byte,
    0xc4 as ::core::ffi::c_int as png_byte,
    0xd4 as ::core::ffi::c_int as png_byte,
    0xe4 as ::core::ffi::c_int as png_byte,
    0xf4 as ::core::ffi::c_int as png_byte,
    0x5 as ::core::ffi::c_int as png_byte,
    0x15 as ::core::ffi::c_int as png_byte,
    0x25 as ::core::ffi::c_int as png_byte,
    0x35 as ::core::ffi::c_int as png_byte,
    0x45 as ::core::ffi::c_int as png_byte,
    0x55 as ::core::ffi::c_int as png_byte,
    0x65 as ::core::ffi::c_int as png_byte,
    0x75 as ::core::ffi::c_int as png_byte,
    0x85 as ::core::ffi::c_int as png_byte,
    0x95 as ::core::ffi::c_int as png_byte,
    0xa5 as ::core::ffi::c_int as png_byte,
    0xb5 as ::core::ffi::c_int as png_byte,
    0xc5 as ::core::ffi::c_int as png_byte,
    0xd5 as ::core::ffi::c_int as png_byte,
    0xe5 as ::core::ffi::c_int as png_byte,
    0xf5 as ::core::ffi::c_int as png_byte,
    0x6 as ::core::ffi::c_int as png_byte,
    0x16 as ::core::ffi::c_int as png_byte,
    0x26 as ::core::ffi::c_int as png_byte,
    0x36 as ::core::ffi::c_int as png_byte,
    0x46 as ::core::ffi::c_int as png_byte,
    0x56 as ::core::ffi::c_int as png_byte,
    0x66 as ::core::ffi::c_int as png_byte,
    0x76 as ::core::ffi::c_int as png_byte,
    0x86 as ::core::ffi::c_int as png_byte,
    0x96 as ::core::ffi::c_int as png_byte,
    0xa6 as ::core::ffi::c_int as png_byte,
    0xb6 as ::core::ffi::c_int as png_byte,
    0xc6 as ::core::ffi::c_int as png_byte,
    0xd6 as ::core::ffi::c_int as png_byte,
    0xe6 as ::core::ffi::c_int as png_byte,
    0xf6 as ::core::ffi::c_int as png_byte,
    0x7 as ::core::ffi::c_int as png_byte,
    0x17 as ::core::ffi::c_int as png_byte,
    0x27 as ::core::ffi::c_int as png_byte,
    0x37 as ::core::ffi::c_int as png_byte,
    0x47 as ::core::ffi::c_int as png_byte,
    0x57 as ::core::ffi::c_int as png_byte,
    0x67 as ::core::ffi::c_int as png_byte,
    0x77 as ::core::ffi::c_int as png_byte,
    0x87 as ::core::ffi::c_int as png_byte,
    0x97 as ::core::ffi::c_int as png_byte,
    0xa7 as ::core::ffi::c_int as png_byte,
    0xb7 as ::core::ffi::c_int as png_byte,
    0xc7 as ::core::ffi::c_int as png_byte,
    0xd7 as ::core::ffi::c_int as png_byte,
    0xe7 as ::core::ffi::c_int as png_byte,
    0xf7 as ::core::ffi::c_int as png_byte,
    0x8 as ::core::ffi::c_int as png_byte,
    0x18 as ::core::ffi::c_int as png_byte,
    0x28 as ::core::ffi::c_int as png_byte,
    0x38 as ::core::ffi::c_int as png_byte,
    0x48 as ::core::ffi::c_int as png_byte,
    0x58 as ::core::ffi::c_int as png_byte,
    0x68 as ::core::ffi::c_int as png_byte,
    0x78 as ::core::ffi::c_int as png_byte,
    0x88 as ::core::ffi::c_int as png_byte,
    0x98 as ::core::ffi::c_int as png_byte,
    0xa8 as ::core::ffi::c_int as png_byte,
    0xb8 as ::core::ffi::c_int as png_byte,
    0xc8 as ::core::ffi::c_int as png_byte,
    0xd8 as ::core::ffi::c_int as png_byte,
    0xe8 as ::core::ffi::c_int as png_byte,
    0xf8 as ::core::ffi::c_int as png_byte,
    0x9 as ::core::ffi::c_int as png_byte,
    0x19 as ::core::ffi::c_int as png_byte,
    0x29 as ::core::ffi::c_int as png_byte,
    0x39 as ::core::ffi::c_int as png_byte,
    0x49 as ::core::ffi::c_int as png_byte,
    0x59 as ::core::ffi::c_int as png_byte,
    0x69 as ::core::ffi::c_int as png_byte,
    0x79 as ::core::ffi::c_int as png_byte,
    0x89 as ::core::ffi::c_int as png_byte,
    0x99 as ::core::ffi::c_int as png_byte,
    0xa9 as ::core::ffi::c_int as png_byte,
    0xb9 as ::core::ffi::c_int as png_byte,
    0xc9 as ::core::ffi::c_int as png_byte,
    0xd9 as ::core::ffi::c_int as png_byte,
    0xe9 as ::core::ffi::c_int as png_byte,
    0xf9 as ::core::ffi::c_int as png_byte,
    0xa as ::core::ffi::c_int as png_byte,
    0x1a as ::core::ffi::c_int as png_byte,
    0x2a as ::core::ffi::c_int as png_byte,
    0x3a as ::core::ffi::c_int as png_byte,
    0x4a as ::core::ffi::c_int as png_byte,
    0x5a as ::core::ffi::c_int as png_byte,
    0x6a as ::core::ffi::c_int as png_byte,
    0x7a as ::core::ffi::c_int as png_byte,
    0x8a as ::core::ffi::c_int as png_byte,
    0x9a as ::core::ffi::c_int as png_byte,
    0xaa as ::core::ffi::c_int as png_byte,
    0xba as ::core::ffi::c_int as png_byte,
    0xca as ::core::ffi::c_int as png_byte,
    0xda as ::core::ffi::c_int as png_byte,
    0xea as ::core::ffi::c_int as png_byte,
    0xfa as ::core::ffi::c_int as png_byte,
    0xb as ::core::ffi::c_int as png_byte,
    0x1b as ::core::ffi::c_int as png_byte,
    0x2b as ::core::ffi::c_int as png_byte,
    0x3b as ::core::ffi::c_int as png_byte,
    0x4b as ::core::ffi::c_int as png_byte,
    0x5b as ::core::ffi::c_int as png_byte,
    0x6b as ::core::ffi::c_int as png_byte,
    0x7b as ::core::ffi::c_int as png_byte,
    0x8b as ::core::ffi::c_int as png_byte,
    0x9b as ::core::ffi::c_int as png_byte,
    0xab as ::core::ffi::c_int as png_byte,
    0xbb as ::core::ffi::c_int as png_byte,
    0xcb as ::core::ffi::c_int as png_byte,
    0xdb as ::core::ffi::c_int as png_byte,
    0xeb as ::core::ffi::c_int as png_byte,
    0xfb as ::core::ffi::c_int as png_byte,
    0xc as ::core::ffi::c_int as png_byte,
    0x1c as ::core::ffi::c_int as png_byte,
    0x2c as ::core::ffi::c_int as png_byte,
    0x3c as ::core::ffi::c_int as png_byte,
    0x4c as ::core::ffi::c_int as png_byte,
    0x5c as ::core::ffi::c_int as png_byte,
    0x6c as ::core::ffi::c_int as png_byte,
    0x7c as ::core::ffi::c_int as png_byte,
    0x8c as ::core::ffi::c_int as png_byte,
    0x9c as ::core::ffi::c_int as png_byte,
    0xac as ::core::ffi::c_int as png_byte,
    0xbc as ::core::ffi::c_int as png_byte,
    0xcc as ::core::ffi::c_int as png_byte,
    0xdc as ::core::ffi::c_int as png_byte,
    0xec as ::core::ffi::c_int as png_byte,
    0xfc as ::core::ffi::c_int as png_byte,
    0xd as ::core::ffi::c_int as png_byte,
    0x1d as ::core::ffi::c_int as png_byte,
    0x2d as ::core::ffi::c_int as png_byte,
    0x3d as ::core::ffi::c_int as png_byte,
    0x4d as ::core::ffi::c_int as png_byte,
    0x5d as ::core::ffi::c_int as png_byte,
    0x6d as ::core::ffi::c_int as png_byte,
    0x7d as ::core::ffi::c_int as png_byte,
    0x8d as ::core::ffi::c_int as png_byte,
    0x9d as ::core::ffi::c_int as png_byte,
    0xad as ::core::ffi::c_int as png_byte,
    0xbd as ::core::ffi::c_int as png_byte,
    0xcd as ::core::ffi::c_int as png_byte,
    0xdd as ::core::ffi::c_int as png_byte,
    0xed as ::core::ffi::c_int as png_byte,
    0xfd as ::core::ffi::c_int as png_byte,
    0xe as ::core::ffi::c_int as png_byte,
    0x1e as ::core::ffi::c_int as png_byte,
    0x2e as ::core::ffi::c_int as png_byte,
    0x3e as ::core::ffi::c_int as png_byte,
    0x4e as ::core::ffi::c_int as png_byte,
    0x5e as ::core::ffi::c_int as png_byte,
    0x6e as ::core::ffi::c_int as png_byte,
    0x7e as ::core::ffi::c_int as png_byte,
    0x8e as ::core::ffi::c_int as png_byte,
    0x9e as ::core::ffi::c_int as png_byte,
    0xae as ::core::ffi::c_int as png_byte,
    0xbe as ::core::ffi::c_int as png_byte,
    0xce as ::core::ffi::c_int as png_byte,
    0xde as ::core::ffi::c_int as png_byte,
    0xee as ::core::ffi::c_int as png_byte,
    0xfe as ::core::ffi::c_int as png_byte,
    0xf as ::core::ffi::c_int as png_byte,
    0x1f as ::core::ffi::c_int as png_byte,
    0x2f as ::core::ffi::c_int as png_byte,
    0x3f as ::core::ffi::c_int as png_byte,
    0x4f as ::core::ffi::c_int as png_byte,
    0x5f as ::core::ffi::c_int as png_byte,
    0x6f as ::core::ffi::c_int as png_byte,
    0x7f as ::core::ffi::c_int as png_byte,
    0x8f as ::core::ffi::c_int as png_byte,
    0x9f as ::core::ffi::c_int as png_byte,
    0xaf as ::core::ffi::c_int as png_byte,
    0xbf as ::core::ffi::c_int as png_byte,
    0xcf as ::core::ffi::c_int as png_byte,
    0xdf as ::core::ffi::c_int as png_byte,
    0xef as ::core::ffi::c_int as png_byte,
    0xff as ::core::ffi::c_int as png_byte,
];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_packswap(mut row_info: png_row_infop, mut row: png_bytep) {
    if ((*row_info).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
        let mut table: png_const_bytep = ::core::ptr::null::<png_byte>();
        let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
        let mut row_end: png_bytep = row.offset((*row_info).rowbytes as isize);
        if (*row_info).bit_depth as ::core::ffi::c_int == 1 as ::core::ffi::c_int {
            table = &raw const onebppswaptable as *const png_byte as png_const_bytep;
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
            table = &raw const twobppswaptable as *const png_byte as png_const_bytep;
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 4 as ::core::ffi::c_int {
            table = &raw const fourbppswaptable as *const png_byte as png_const_bytep;
        } else {
            return;
        }
        rp = row;
        while rp < row_end {
            *rp = *table.offset(*rp as isize);
            rp = rp.offset(1);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_strip_channel(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut at_start: ::core::ffi::c_int,
) {
    let mut sp: png_bytep = row;
    let mut dp: png_bytep = row;
    let mut ep: png_bytep = row.offset((*row_info).rowbytes as isize);
    if (*row_info).channels as ::core::ffi::c_int == 2 as ::core::ffi::c_int {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if at_start != 0 as ::core::ffi::c_int {
                sp = sp.offset(1);
            } else {
                sp = sp.offset(2 as ::core::ffi::c_int as isize);
                dp = dp.offset(1);
            }
            while sp < ep {
                let fresh0 = dp;
                dp = dp.offset(1);
                *fresh0 = *sp;
                sp = sp.offset(2 as ::core::ffi::c_int as isize);
            }
            (*row_info).pixel_depth = 8 as png_byte;
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            if at_start != 0 as ::core::ffi::c_int {
                sp = sp.offset(2 as ::core::ffi::c_int as isize);
            } else {
                sp = sp.offset(4 as ::core::ffi::c_int as isize);
                dp = dp.offset(2 as ::core::ffi::c_int as isize);
            }
            while sp < ep {
                let fresh1 = sp;
                sp = sp.offset(1);
                let fresh2 = dp;
                dp = dp.offset(1);
                *fresh2 = *fresh1;
                let fresh3 = dp;
                dp = dp.offset(1);
                *fresh3 = *sp;
                sp = sp.offset(3 as ::core::ffi::c_int as isize);
            }
            (*row_info).pixel_depth = 16 as png_byte;
        } else {
            return;
        }
        (*row_info).channels = 1 as png_byte;
        if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_GRAY as png_byte;
        }
    } else if (*row_info).channels as ::core::ffi::c_int == 4 as ::core::ffi::c_int {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if at_start != 0 as ::core::ffi::c_int {
                sp = sp.offset(1);
            } else {
                sp = sp.offset(4 as ::core::ffi::c_int as isize);
                dp = dp.offset(3 as ::core::ffi::c_int as isize);
            }
            while sp < ep {
                let fresh4 = sp;
                sp = sp.offset(1);
                let fresh5 = dp;
                dp = dp.offset(1);
                *fresh5 = *fresh4;
                let fresh6 = sp;
                sp = sp.offset(1);
                let fresh7 = dp;
                dp = dp.offset(1);
                *fresh7 = *fresh6;
                let fresh8 = dp;
                dp = dp.offset(1);
                *fresh8 = *sp;
                sp = sp.offset(2 as ::core::ffi::c_int as isize);
            }
            (*row_info).pixel_depth = 24 as png_byte;
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            if at_start != 0 as ::core::ffi::c_int {
                sp = sp.offset(2 as ::core::ffi::c_int as isize);
            } else {
                sp = sp.offset(8 as ::core::ffi::c_int as isize);
                dp = dp.offset(6 as ::core::ffi::c_int as isize);
            }
            while sp < ep {
                let fresh9 = sp;
                sp = sp.offset(1);
                let fresh10 = dp;
                dp = dp.offset(1);
                *fresh10 = *fresh9;
                let fresh11 = sp;
                sp = sp.offset(1);
                let fresh12 = dp;
                dp = dp.offset(1);
                *fresh12 = *fresh11;
                let fresh13 = sp;
                sp = sp.offset(1);
                let fresh14 = dp;
                dp = dp.offset(1);
                *fresh14 = *fresh13;
                let fresh15 = sp;
                sp = sp.offset(1);
                let fresh16 = dp;
                dp = dp.offset(1);
                *fresh16 = *fresh15;
                let fresh17 = sp;
                sp = sp.offset(1);
                let fresh18 = dp;
                dp = dp.offset(1);
                *fresh18 = *fresh17;
                let fresh19 = dp;
                dp = dp.offset(1);
                *fresh19 = *sp;
                sp = sp.offset(3 as ::core::ffi::c_int as isize);
            }
            (*row_info).pixel_depth = 48 as png_byte;
        } else {
            return;
        }
        (*row_info).channels = 3 as png_byte;
        if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
            (*row_info).color_type = PNG_COLOR_TYPE_RGB as png_byte;
        }
    } else {
        return;
    }
    (*row_info).rowbytes = dp.offset_from(row) as ::core::ffi::c_long as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_bgr(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
        != 0 as ::core::ffi::c_int
    {
        let mut row_width: png_uint_32 = (*row_info).width;
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB {
                let mut rp: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut i: png_uint_32 = 0;
                i = 0 as png_uint_32;
                rp = row;
                while i < row_width {
                    let mut save: png_byte = *rp;
                    *rp = *rp.offset(2 as ::core::ffi::c_int as isize);
                    *rp.offset(2 as ::core::ffi::c_int as isize) = save;
                    i = i.wrapping_add(1);
                    rp = rp.offset(3 as ::core::ffi::c_int as isize);
                }
            } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut i_0: png_uint_32 = 0;
                i_0 = 0 as png_uint_32;
                rp_0 = row;
                while i_0 < row_width {
                    let mut save_0: png_byte = *rp_0;
                    *rp_0 = *rp_0.offset(2 as ::core::ffi::c_int as isize);
                    *rp_0.offset(2 as ::core::ffi::c_int as isize) = save_0;
                    i_0 = i_0.wrapping_add(1);
                    rp_0 = rp_0.offset(4 as ::core::ffi::c_int as isize);
                }
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 16 as ::core::ffi::c_int {
            if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB {
                let mut rp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut i_1: png_uint_32 = 0;
                i_1 = 0 as png_uint_32;
                rp_1 = row;
                while i_1 < row_width {
                    let mut save_1: png_byte = *rp_1;
                    *rp_1 = *rp_1.offset(4 as ::core::ffi::c_int as isize);
                    *rp_1.offset(4 as ::core::ffi::c_int as isize) = save_1;
                    save_1 = *rp_1.offset(1 as ::core::ffi::c_int as isize);
                    *rp_1.offset(1 as ::core::ffi::c_int as isize) =
                        *rp_1.offset(5 as ::core::ffi::c_int as isize);
                    *rp_1.offset(5 as ::core::ffi::c_int as isize) = save_1;
                    i_1 = i_1.wrapping_add(1);
                    rp_1 = rp_1.offset(6 as ::core::ffi::c_int as isize);
                }
            } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
                let mut rp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut i_2: png_uint_32 = 0;
                i_2 = 0 as png_uint_32;
                rp_2 = row;
                while i_2 < row_width {
                    let mut save_2: png_byte = *rp_2;
                    *rp_2 = *rp_2.offset(4 as ::core::ffi::c_int as isize);
                    *rp_2.offset(4 as ::core::ffi::c_int as isize) = save_2;
                    save_2 = *rp_2.offset(1 as ::core::ffi::c_int as isize);
                    *rp_2.offset(1 as ::core::ffi::c_int as isize) =
                        *rp_2.offset(5 as ::core::ffi::c_int as isize);
                    *rp_2.offset(5 as ::core::ffi::c_int as isize) = save_2;
                    i_2 = i_2.wrapping_add(1);
                    rp_2 = rp_2.offset(8 as ::core::ffi::c_int as isize);
                }
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_check_palette_indexes(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
) {
    if ((*png_ptr).num_palette as ::core::ffi::c_int)
        < (1 as ::core::ffi::c_int) << (*row_info).bit_depth as ::core::ffi::c_int
        && (*png_ptr).num_palette as ::core::ffi::c_int > 0 as ::core::ffi::c_int
    {
        let mut padding: ::core::ffi::c_int = (8 as ::core::ffi::c_uint)
            .wrapping_sub(
                ((*row_info).pixel_depth as ::core::ffi::c_uint)
                    .wrapping_mul(
                        ((*row_info).width as ::core::ffi::c_uint)
                            .wrapping_rem(8 as ::core::ffi::c_int as ::core::ffi::c_uint),
                    )
                    .wrapping_rem(8 as ::core::ffi::c_uint),
            )
            .wrapping_rem(8 as ::core::ffi::c_uint)
            as ::core::ffi::c_int;
        let mut rp: png_bytep = (*png_ptr).row_buf.offset((*row_info).rowbytes as isize);
        match (*row_info).bit_depth as ::core::ffi::c_int {
            1 => {
                while rp > (*png_ptr).row_buf {
                    if *rp as ::core::ffi::c_int >> padding != 0 as ::core::ffi::c_int {
                        (*png_ptr).num_palette_max = 1 as ::core::ffi::c_int;
                    }
                    padding = 0 as ::core::ffi::c_int;
                    rp = rp.offset(-1);
                }
            }
            2 => {
                while rp > (*png_ptr).row_buf {
                    let mut i: ::core::ffi::c_int =
                        *rp as ::core::ffi::c_int >> padding & 0x3 as ::core::ffi::c_int;
                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }
                    i = *rp as ::core::ffi::c_int >> padding >> 2 as ::core::ffi::c_int
                        & 0x3 as ::core::ffi::c_int;
                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }
                    i = *rp as ::core::ffi::c_int >> padding >> 4 as ::core::ffi::c_int
                        & 0x3 as ::core::ffi::c_int;
                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }
                    i = *rp as ::core::ffi::c_int >> padding >> 6 as ::core::ffi::c_int
                        & 0x3 as ::core::ffi::c_int;
                    if i > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i;
                    }
                    padding = 0 as ::core::ffi::c_int;
                    rp = rp.offset(-1);
                }
            }
            4 => {
                while rp > (*png_ptr).row_buf {
                    let mut i_0: ::core::ffi::c_int =
                        *rp as ::core::ffi::c_int >> padding & 0xf as ::core::ffi::c_int;
                    if i_0 > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i_0;
                    }
                    i_0 = *rp as ::core::ffi::c_int >> padding >> 4 as ::core::ffi::c_int
                        & 0xf as ::core::ffi::c_int;
                    if i_0 > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = i_0;
                    }
                    padding = 0 as ::core::ffi::c_int;
                    rp = rp.offset(-1);
                }
            }
            8 => {
                while rp > (*png_ptr).row_buf {
                    if *rp as ::core::ffi::c_int > (*png_ptr).num_palette_max {
                        (*png_ptr).num_palette_max = *rp as ::core::ffi::c_int;
                    }
                    rp = rp.offset(-1);
                }
            }
            _ => {}
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_transform_info(
    mut png_ptr: png_structrp,
    mut user_transform_ptr: png_voidp,
    mut user_transform_depth: ::core::ffi::c_int,
    mut user_transform_channels: ::core::ffi::c_int,
) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint
        && (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_ROW_INIT != 0 as ::core::ffi::c_uint
    {
        png_app_error(
            png_ptr,
            b"info change after png_start_read_image or png_read_update_info\0" as *const u8
                as png_const_charp,
        );
        return;
    }
    (*png_ptr).user_transform_ptr = user_transform_ptr;
    (*png_ptr).user_transform_depth = user_transform_depth as png_byte;
    (*png_ptr).user_transform_channels = user_transform_channels as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_transform_ptr(mut png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return NULL_0;
    }
    return (*png_ptr).user_transform_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_row_number(
    mut png_ptr: png_const_structrp,
) -> png_uint_32 {
    if !png_ptr.is_null() {
        return (*png_ptr).row_number;
    }
    return PNG_UINT_32_MAX;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_current_pass_number(mut png_ptr: png_const_structrp) -> png_byte {
    if !png_ptr.is_null() {
        return (*png_ptr).pass;
    }
    return 8 as png_byte;
}
pub const PNG_IS_READ_STRUCT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_BGR: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_INTERLACE: ::core::ffi::c_uint = 0x2 as ::core::ffi::c_uint;
pub const PNG_PACK: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_SHIFT: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_SWAP_BYTES: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_INVERT_MONO: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_FILLER: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_PACKSWAP: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_SWAP_ALPHA: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_INVERT_ALPHA: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PNG_ADD_ALPHA: ::core::ffi::c_uint = 0x1000000 as ::core::ffi::c_uint;
pub const PNG_FLAG_ROW_INIT: ::core::ffi::c_uint = 0x40 as ::core::ffi::c_uint;
pub const PNG_FLAG_FILLER_AFTER: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
