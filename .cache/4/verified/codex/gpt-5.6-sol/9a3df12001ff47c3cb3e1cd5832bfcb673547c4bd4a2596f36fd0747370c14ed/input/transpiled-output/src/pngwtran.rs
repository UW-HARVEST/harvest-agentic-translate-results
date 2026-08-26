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
    fn png_do_strip_channel(row_info: png_row_infop, row: png_bytep, at_start: ::core::ffi::c_int);
    fn png_do_swap(row_info: png_row_infop, row: png_bytep);
    fn png_do_packswap(row_info: png_row_infop, row: png_bytep);
    fn png_do_invert(row_info: png_row_infop, row: png_bytep);
    fn png_do_bgr(row_info: png_row_infop, row: png_bytep);
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
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB_ALPHA: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
unsafe extern "C" fn png_do_pack(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut bit_depth: png_uint_32,
) {
    if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int
        && (*row_info).channels as ::core::ffi::c_int == 1 as ::core::ffi::c_int
    {
        match bit_depth as ::core::ffi::c_int {
            1 => {
                let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut mask: ::core::ffi::c_int = 0;
                let mut v: ::core::ffi::c_int = 0;
                let mut i: png_uint_32 = 0;
                let mut row_width: png_uint_32 = (*row_info).width;
                sp = row;
                dp = row;
                mask = 0x80 as ::core::ffi::c_int;
                v = 0 as ::core::ffi::c_int;
                i = 0 as png_uint_32;
                while i < row_width {
                    if *sp as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
                        v |= mask;
                    }
                    sp = sp.offset(1);
                    if mask > 1 as ::core::ffi::c_int {
                        mask >>= 1 as ::core::ffi::c_int;
                    } else {
                        mask = 0x80 as ::core::ffi::c_int;
                        *dp = v as png_byte;
                        dp = dp.offset(1);
                        v = 0 as ::core::ffi::c_int;
                    }
                    i = i.wrapping_add(1);
                }
                if mask != 0x80 as ::core::ffi::c_int {
                    *dp = v as png_byte;
                }
            }
            2 => {
                let mut sp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut shift: ::core::ffi::c_uint = 0;
                let mut v_0: ::core::ffi::c_int = 0;
                let mut i_0: png_uint_32 = 0;
                let mut row_width_0: png_uint_32 = (*row_info).width;
                sp_0 = row;
                dp_0 = row;
                shift = 6 as ::core::ffi::c_uint;
                v_0 = 0 as ::core::ffi::c_int;
                i_0 = 0 as png_uint_32;
                while i_0 < row_width_0 {
                    let mut value: png_byte = 0;
                    value = (*sp_0 as ::core::ffi::c_int & 0x3 as ::core::ffi::c_int) as png_byte;
                    v_0 |= (value as ::core::ffi::c_int) << shift;
                    if shift == 0 as ::core::ffi::c_uint {
                        shift = 6 as ::core::ffi::c_uint;
                        *dp_0 = v_0 as png_byte;
                        dp_0 = dp_0.offset(1);
                        v_0 = 0 as ::core::ffi::c_int;
                    } else {
                        shift = shift.wrapping_sub(2 as ::core::ffi::c_uint);
                    }
                    sp_0 = sp_0.offset(1);
                    i_0 = i_0.wrapping_add(1);
                }
                if shift != 6 as ::core::ffi::c_uint {
                    *dp_0 = v_0 as png_byte;
                }
            }
            4 => {
                let mut sp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut dp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
                let mut shift_0: ::core::ffi::c_uint = 0;
                let mut v_1: ::core::ffi::c_int = 0;
                let mut i_1: png_uint_32 = 0;
                let mut row_width_1: png_uint_32 = (*row_info).width;
                sp_1 = row;
                dp_1 = row;
                shift_0 = 4 as ::core::ffi::c_uint;
                v_1 = 0 as ::core::ffi::c_int;
                i_1 = 0 as png_uint_32;
                while i_1 < row_width_1 {
                    let mut value_0: png_byte = 0;
                    value_0 = (*sp_1 as ::core::ffi::c_int & 0xf as ::core::ffi::c_int) as png_byte;
                    v_1 |= (value_0 as ::core::ffi::c_int) << shift_0;
                    if shift_0 == 0 as ::core::ffi::c_uint {
                        shift_0 = 4 as ::core::ffi::c_uint;
                        *dp_1 = v_1 as png_byte;
                        dp_1 = dp_1.offset(1);
                        v_1 = 0 as ::core::ffi::c_int;
                    } else {
                        shift_0 = shift_0.wrapping_sub(4 as ::core::ffi::c_uint);
                    }
                    sp_1 = sp_1.offset(1);
                    i_1 = i_1.wrapping_add(1);
                }
                if shift_0 != 4 as ::core::ffi::c_uint {
                    *dp_1 = v_1 as png_byte;
                }
            }
            _ => {}
        }
        (*row_info).bit_depth = bit_depth as png_byte;
        (*row_info).pixel_depth = (bit_depth as ::core::ffi::c_uint)
            .wrapping_mul((*row_info).channels as ::core::ffi::c_uint)
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
unsafe extern "C" fn png_do_shift(
    mut row_info: png_row_infop,
    mut row: png_bytep,
    mut bit_depth: png_const_color_8p,
) {
    if (*row_info).color_type as ::core::ffi::c_int != PNG_COLOR_TYPE_PALETTE {
        let mut shift_start: [::core::ffi::c_int; 4] = [0; 4];
        let mut shift_dec: [::core::ffi::c_int; 4] = [0; 4];
        let mut channels: ::core::ffi::c_uint = 0 as ::core::ffi::c_uint;
        if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
            != 0 as ::core::ffi::c_int
        {
            shift_start[channels as usize] = (*row_info).bit_depth as ::core::ffi::c_int
                - (*bit_depth).red as ::core::ffi::c_int;
            shift_dec[channels as usize] = (*bit_depth).red as ::core::ffi::c_int;
            channels = channels.wrapping_add(1);
            shift_start[channels as usize] = (*row_info).bit_depth as ::core::ffi::c_int
                - (*bit_depth).green as ::core::ffi::c_int;
            shift_dec[channels as usize] = (*bit_depth).green as ::core::ffi::c_int;
            channels = channels.wrapping_add(1);
            shift_start[channels as usize] = (*row_info).bit_depth as ::core::ffi::c_int
                - (*bit_depth).blue as ::core::ffi::c_int;
            shift_dec[channels as usize] = (*bit_depth).blue as ::core::ffi::c_int;
            channels = channels.wrapping_add(1);
        } else {
            shift_start[channels as usize] = (*row_info).bit_depth as ::core::ffi::c_int
                - (*bit_depth).gray as ::core::ffi::c_int;
            shift_dec[channels as usize] = (*bit_depth).gray as ::core::ffi::c_int;
            channels = channels.wrapping_add(1);
        }
        if (*row_info).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
            != 0 as ::core::ffi::c_int
        {
            shift_start[channels as usize] = (*row_info).bit_depth as ::core::ffi::c_int
                - (*bit_depth).alpha as ::core::ffi::c_int;
            shift_dec[channels as usize] = (*bit_depth).alpha as ::core::ffi::c_int;
            channels = channels.wrapping_add(1);
        }
        if ((*row_info).bit_depth as ::core::ffi::c_int) < 8 as ::core::ffi::c_int {
            let mut bp: png_bytep = row;
            let mut i: size_t = 0;
            let mut mask: ::core::ffi::c_uint = 0;
            let mut row_bytes: size_t = (*row_info).rowbytes;
            if (*bit_depth).gray as ::core::ffi::c_int == 1 as ::core::ffi::c_int
                && (*row_info).bit_depth as ::core::ffi::c_int == 2 as ::core::ffi::c_int
            {
                mask = 0x55 as ::core::ffi::c_uint;
            } else if (*row_info).bit_depth as ::core::ffi::c_int == 4 as ::core::ffi::c_int
                && (*bit_depth).gray as ::core::ffi::c_int == 3 as ::core::ffi::c_int
            {
                mask = 0x11 as ::core::ffi::c_uint;
            } else {
                mask = 0xff as ::core::ffi::c_uint;
            }
            i = 0 as size_t;
            while i < row_bytes {
                let mut j: ::core::ffi::c_int = 0;
                let mut v: ::core::ffi::c_uint = 0;
                let mut out: ::core::ffi::c_uint = 0;
                v = *bp as ::core::ffi::c_uint;
                out = 0 as ::core::ffi::c_uint;
                j = shift_start[0 as ::core::ffi::c_int as usize];
                while j > -shift_dec[0 as ::core::ffi::c_int as usize] {
                    if j > 0 as ::core::ffi::c_int {
                        out |= v << j;
                    } else {
                        out |= v >> -j & mask;
                    }
                    j -= shift_dec[0 as ::core::ffi::c_int as usize];
                }
                *bp = (out & 0xff as ::core::ffi::c_uint) as png_byte;
                i = i.wrapping_add(1);
                bp = bp.offset(1);
            }
        } else if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut bp_0: png_bytep = row;
            let mut i_0: png_uint_32 = 0;
            let mut istop: png_uint_32 = (channels as png_uint_32).wrapping_mul((*row_info).width);
            i_0 = 0 as png_uint_32;
            while i_0 < istop {
                let mut c: ::core::ffi::c_uint =
                    (i_0 as ::core::ffi::c_uint).wrapping_rem(channels);
                let mut j_0: ::core::ffi::c_int = 0;
                let mut v_0: ::core::ffi::c_uint = 0;
                let mut out_0: ::core::ffi::c_uint = 0;
                v_0 = *bp_0 as ::core::ffi::c_uint;
                out_0 = 0 as ::core::ffi::c_uint;
                j_0 = shift_start[c as usize];
                while j_0 > -shift_dec[c as usize] {
                    if j_0 > 0 as ::core::ffi::c_int {
                        out_0 |= v_0 << j_0;
                    } else {
                        out_0 |= v_0 >> -j_0;
                    }
                    j_0 -= shift_dec[c as usize];
                }
                *bp_0 = (out_0 & 0xff as ::core::ffi::c_uint) as png_byte;
                i_0 = i_0.wrapping_add(1);
                bp_0 = bp_0.offset(1);
            }
        } else {
            let mut bp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_1: png_uint_32 = 0;
            let mut istop_0: png_uint_32 =
                (channels as png_uint_32).wrapping_mul((*row_info).width);
            bp_1 = row;
            i_1 = 0 as png_uint_32;
            while i_1 < istop_0 {
                let mut c_0: ::core::ffi::c_uint =
                    (i_1 as ::core::ffi::c_uint).wrapping_rem(channels);
                let mut j_1: ::core::ffi::c_int = 0;
                let mut value: ::core::ffi::c_uint = 0;
                let mut v_1: ::core::ffi::c_uint = 0;
                v_1 = ((*bp_1 as ::core::ffi::c_uint) << 8 as ::core::ffi::c_int).wrapping_add(
                    *bp_1.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_uint,
                ) as png_uint_16 as ::core::ffi::c_uint;
                value = 0 as ::core::ffi::c_uint;
                j_1 = shift_start[c_0 as usize];
                while j_1 > -shift_dec[c_0 as usize] {
                    if j_1 > 0 as ::core::ffi::c_int {
                        value |= v_1 << j_1;
                    } else {
                        value |= v_1 >> -j_1;
                    }
                    j_1 -= shift_dec[c_0 as usize];
                }
                let fresh47 = bp_1;
                bp_1 = bp_1.offset(1);
                *fresh47 =
                    (value >> 8 as ::core::ffi::c_int & 0xff as ::core::ffi::c_uint) as png_byte;
                let fresh48 = bp_1;
                bp_1 = bp_1.offset(1);
                *fresh48 = (value & 0xff as ::core::ffi::c_uint) as png_byte;
                i_1 = i_1.wrapping_add(1);
            }
        }
    }
}
unsafe extern "C" fn png_do_write_swap_alpha(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i: png_uint_32 = 0;
            let mut row_width: png_uint_32 = (*row_info).width;
            i = 0 as png_uint_32;
            dp = row;
            sp = dp;
            while i < row_width {
                let fresh11 = sp;
                sp = sp.offset(1);
                let mut save: png_byte = *fresh11;
                let fresh12 = sp;
                sp = sp.offset(1);
                let fresh13 = dp;
                dp = dp.offset(1);
                *fresh13 = *fresh12;
                let fresh14 = sp;
                sp = sp.offset(1);
                let fresh15 = dp;
                dp = dp.offset(1);
                *fresh15 = *fresh14;
                let fresh16 = sp;
                sp = sp.offset(1);
                let fresh17 = dp;
                dp = dp.offset(1);
                *fresh17 = *fresh16;
                let fresh18 = dp;
                dp = dp.offset(1);
                *fresh18 = save;
                i = i.wrapping_add(1);
            }
        } else {
            let mut sp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_0: png_uint_32 = 0;
            let mut row_width_0: png_uint_32 = (*row_info).width;
            i_0 = 0 as png_uint_32;
            dp_0 = row;
            sp_0 = dp_0;
            while i_0 < row_width_0 {
                let mut save_0: [png_byte; 2] = [0; 2];
                let fresh19 = sp_0;
                sp_0 = sp_0.offset(1);
                save_0[0 as ::core::ffi::c_int as usize] = *fresh19;
                let fresh20 = sp_0;
                sp_0 = sp_0.offset(1);
                save_0[1 as ::core::ffi::c_int as usize] = *fresh20;
                let fresh21 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh22 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh22 = *fresh21;
                let fresh23 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh24 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh24 = *fresh23;
                let fresh25 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh26 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh26 = *fresh25;
                let fresh27 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh28 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh28 = *fresh27;
                let fresh29 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh30 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh30 = *fresh29;
                let fresh31 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh32 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh32 = *fresh31;
                let fresh33 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh33 = save_0[0 as ::core::ffi::c_int as usize];
                let fresh34 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh34 = save_0[1 as ::core::ffi::c_int as usize];
                i_0 = i_0.wrapping_add(1);
            }
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_1: png_uint_32 = 0;
            let mut row_width_1: png_uint_32 = (*row_info).width;
            i_1 = 0 as png_uint_32;
            dp_1 = row;
            sp_1 = dp_1;
            while i_1 < row_width_1 {
                let fresh35 = sp_1;
                sp_1 = sp_1.offset(1);
                let mut save_1: png_byte = *fresh35;
                let fresh36 = sp_1;
                sp_1 = sp_1.offset(1);
                let fresh37 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh37 = *fresh36;
                let fresh38 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh38 = save_1;
                i_1 = i_1.wrapping_add(1);
            }
        } else {
            let mut sp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_2: png_uint_32 = 0;
            let mut row_width_2: png_uint_32 = (*row_info).width;
            i_2 = 0 as png_uint_32;
            dp_2 = row;
            sp_2 = dp_2;
            while i_2 < row_width_2 {
                let mut save_2: [png_byte; 2] = [0; 2];
                let fresh39 = sp_2;
                sp_2 = sp_2.offset(1);
                save_2[0 as ::core::ffi::c_int as usize] = *fresh39;
                let fresh40 = sp_2;
                sp_2 = sp_2.offset(1);
                save_2[1 as ::core::ffi::c_int as usize] = *fresh40;
                let fresh41 = sp_2;
                sp_2 = sp_2.offset(1);
                let fresh42 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh42 = *fresh41;
                let fresh43 = sp_2;
                sp_2 = sp_2.offset(1);
                let fresh44 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh44 = *fresh43;
                let fresh45 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh45 = save_2[0 as ::core::ffi::c_int as usize];
                let fresh46 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh46 = save_2[1 as ::core::ffi::c_int as usize];
                i_2 = i_2.wrapping_add(1);
            }
        }
    }
}
unsafe extern "C" fn png_do_write_invert_alpha(mut row_info: png_row_infop, mut row: png_bytep) {
    if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i: png_uint_32 = 0;
            let mut row_width: png_uint_32 = (*row_info).width;
            i = 0 as png_uint_32;
            dp = row;
            sp = dp;
            while i < row_width {
                sp = sp.offset(3 as ::core::ffi::c_int as isize);
                dp = sp;
                let fresh0 = sp;
                sp = sp.offset(1);
                *dp = (255 as ::core::ffi::c_int - *fresh0 as ::core::ffi::c_int) as png_byte;
                i = i.wrapping_add(1);
            }
        } else {
            let mut sp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_0: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_0: png_uint_32 = 0;
            let mut row_width_0: png_uint_32 = (*row_info).width;
            i_0 = 0 as png_uint_32;
            dp_0 = row;
            sp_0 = dp_0;
            while i_0 < row_width_0 {
                sp_0 = sp_0.offset(6 as ::core::ffi::c_int as isize);
                dp_0 = sp_0;
                let fresh1 = sp_0;
                sp_0 = sp_0.offset(1);
                let fresh2 = dp_0;
                dp_0 = dp_0.offset(1);
                *fresh2 = (255 as ::core::ffi::c_int - *fresh1 as ::core::ffi::c_int) as png_byte;
                let fresh3 = sp_0;
                sp_0 = sp_0.offset(1);
                *dp_0 = (255 as ::core::ffi::c_int - *fresh3 as ::core::ffi::c_int) as png_byte;
                i_0 = i_0.wrapping_add(1);
            }
        }
    } else if (*row_info).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY_ALPHA {
        if (*row_info).bit_depth as ::core::ffi::c_int == 8 as ::core::ffi::c_int {
            let mut sp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_1: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_1: png_uint_32 = 0;
            let mut row_width_1: png_uint_32 = (*row_info).width;
            i_1 = 0 as png_uint_32;
            dp_1 = row;
            sp_1 = dp_1;
            while i_1 < row_width_1 {
                let fresh4 = sp_1;
                sp_1 = sp_1.offset(1);
                let fresh5 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh5 = *fresh4;
                let fresh6 = sp_1;
                sp_1 = sp_1.offset(1);
                let fresh7 = dp_1;
                dp_1 = dp_1.offset(1);
                *fresh7 = (255 as ::core::ffi::c_int - *fresh6 as ::core::ffi::c_int) as png_byte;
                i_1 = i_1.wrapping_add(1);
            }
        } else {
            let mut sp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut dp_2: png_bytep = ::core::ptr::null_mut::<png_byte>();
            let mut i_2: png_uint_32 = 0;
            let mut row_width_2: png_uint_32 = (*row_info).width;
            i_2 = 0 as png_uint_32;
            dp_2 = row;
            sp_2 = dp_2;
            while i_2 < row_width_2 {
                sp_2 = sp_2.offset(2 as ::core::ffi::c_int as isize);
                dp_2 = sp_2;
                let fresh8 = sp_2;
                sp_2 = sp_2.offset(1);
                let fresh9 = dp_2;
                dp_2 = dp_2.offset(1);
                *fresh9 = (255 as ::core::ffi::c_int - *fresh8 as ::core::ffi::c_int) as png_byte;
                let fresh10 = sp_2;
                sp_2 = sp_2.offset(1);
                *dp_2 = (255 as ::core::ffi::c_int - *fresh10 as ::core::ffi::c_int) as png_byte;
                i_2 = i_2.wrapping_add(1);
            }
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_do_write_transformations(
    mut png_ptr: png_structrp,
    mut row_info: png_row_infop,
) {
    if png_ptr.is_null() {
        return;
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_USER_TRANSFORM
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).write_user_transform_fn.is_some() {
            Some(
                (*png_ptr)
                    .write_user_transform_fn
                    .expect("non-null function pointer"),
            )
            .expect("non-null function pointer")(
                png_ptr as png_structp,
                row_info,
                (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            );
        }
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_FILLER != 0 as ::core::ffi::c_uint {
        png_do_strip_channel(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            ((*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_FILLER_AFTER == 0)
                as ::core::ffi::c_int,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_PACKSWAP != 0 as ::core::ffi::c_uint
    {
        png_do_packswap(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_PACK != 0 as ::core::ffi::c_uint {
        png_do_pack(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            (*png_ptr).bit_depth as png_uint_32,
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
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SHIFT != 0 as ::core::ffi::c_uint {
        png_do_shift(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
            &raw mut (*png_ptr).shift as png_const_color_8p,
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_SWAP_ALPHA
        != 0 as ::core::ffi::c_uint
    {
        png_do_write_swap_alpha(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_INVERT_ALPHA
        != 0 as ::core::ffi::c_uint
    {
        png_do_write_invert_alpha(
            row_info,
            (*png_ptr).row_buf.offset(1 as ::core::ffi::c_int as isize),
        );
    }
    if (*png_ptr).transformations as ::core::ffi::c_uint & PNG_BGR != 0 as ::core::ffi::c_uint {
        png_do_bgr(
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
}
pub const PNG_BGR: ::core::ffi::c_uint = 0x1 as ::core::ffi::c_uint;
pub const PNG_PACK: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_SHIFT: ::core::ffi::c_uint = 0x8 as ::core::ffi::c_uint;
pub const PNG_SWAP_BYTES: ::core::ffi::c_uint = 0x10 as ::core::ffi::c_uint;
pub const PNG_INVERT_MONO: ::core::ffi::c_uint = 0x20 as ::core::ffi::c_uint;
pub const PNG_FILLER: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_PACKSWAP: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_SWAP_ALPHA: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_INVERT_ALPHA: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
pub const PNG_USER_TRANSFORM: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const PNG_FLAG_FILLER_AFTER: ::core::ffi::c_uint = 0x80 as ::core::ffi::c_uint;
