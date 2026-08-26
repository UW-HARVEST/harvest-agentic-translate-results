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
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_fixed(
        png_ptr: png_const_structrp,
        fp: ::core::ffi::c_double,
        text: png_const_charp,
    ) -> png_fixed_point;
    fn png_check_IHDR(
        png_ptr: png_const_structrp,
        width: png_uint_32,
        height: png_uint_32,
        bit_depth: ::core::ffi::c_int,
        color_type: ::core::ffi::c_int,
        interlace_type: ::core::ffi::c_int,
        compression_type: ::core::ffi::c_int,
        filter_type: ::core::ffi::c_int,
    );
    fn png_muldiv(
        res: png_fixed_point_p,
        a: png_fixed_point,
        multiplied_by: png_int_32,
        divided_by: png_int_32,
    ) -> ::core::ffi::c_int;
    fn png_XYZ_from_xy(XYZ: *mut png_XYZ, xy: *const png_xy) -> ::core::ffi::c_int;
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
pub type png_uint_32p = *mut png_uint_32;
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
pub type png_const_structp = *const png_struct;
pub type png_const_infop = *const png_info;
pub type png_const_structrp = *const png_struct;
pub type png_inforp = *mut png_info;
pub type png_const_inforp = *const png_info;
pub type png_color_16p = *mut png_color_16;
pub type png_color_8p = *mut png_color_8;
pub type png_sPLT_tpp = *mut *mut png_sPLT_t;
pub type png_timep = *mut png_time;
pub type png_unknown_chunkpp = *mut *mut png_unknown_chunk;
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
pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff as ::core::ffi::c_long as png_uint_32;
pub const PNG_FP_1: ::core::ffi::c_int = 100000 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COMPRESSION_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_OFFSET_PIXEL: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_OFFSET_MICROMETER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_RESOLUTION_METER: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
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
pub const PNG_INFO_sCAL: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_INFO_eXIf: ::core::ffi::c_uint = 0x10000 as ::core::ffi::c_uint;
pub const PNG_INFO_cICP: ::core::ffi::c_uint = 0x20000 as ::core::ffi::c_uint;
pub const PNG_INFO_cLLI: ::core::ffi::c_uint = 0x40000 as ::core::ffi::c_uint;
pub const PNG_INFO_mDCV: ::core::ffi::c_uint = 0x80000 as ::core::ffi::c_uint;
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_valid(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut flag: png_uint_32,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        if flag == PNG_INFO_tRNS
            && (*png_ptr).num_trans as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        {
            return 0 as png_uint_32;
        }
        return (*info_ptr).valid & flag;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rowbytes(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> size_t {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).rowbytes;
    }
    return 0 as size_t;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rows(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_bytepp {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).row_pointers;
    }
    return ::core::ptr::null_mut::<*mut png_byte>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_width(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).width;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_image_height(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).height;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bit_depth(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).bit_depth;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_color_type(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).color_type;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_filter_type(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).filter_type;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_interlace_type(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).interlace_type;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_type(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).compression_type;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_meter(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).phys_unit_type as ::core::ffi::c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).x_pixels_per_unit;
        }
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_meter(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).phys_unit_type as ::core::ffi::c_int == PNG_RESOLUTION_METER {
            return (*info_ptr).y_pixels_per_unit;
        }
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_meter(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).phys_unit_type as ::core::ffi::c_int == PNG_RESOLUTION_METER
            && (*info_ptr).x_pixels_per_unit == (*info_ptr).y_pixels_per_unit
        {
            return (*info_ptr).x_pixels_per_unit;
        }
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> ::core::ffi::c_float {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).x_pixels_per_unit != 0 as ::core::ffi::c_uint {
            return (*info_ptr).y_pixels_per_unit as ::core::ffi::c_float
                / (*info_ptr).x_pixels_per_unit as ::core::ffi::c_float;
        }
    }
    return 0.0f64 as ::core::ffi::c_float;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixel_aspect_ratio_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_fixed_point {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
        && (*info_ptr).x_pixels_per_unit > 0 as ::core::ffi::c_uint
        && (*info_ptr).y_pixels_per_unit > 0 as ::core::ffi::c_uint
        && (*info_ptr).x_pixels_per_unit <= PNG_UINT_31_MAX
        && (*info_ptr).y_pixels_per_unit <= PNG_UINT_31_MAX
    {
        let mut res: png_fixed_point = 0;
        if png_muldiv(
            &raw mut res,
            (*info_ptr).y_pixels_per_unit as png_fixed_point,
            PNG_FP_1,
            (*info_ptr).x_pixels_per_unit as png_int_32,
        ) != 0 as ::core::ffi::c_int
        {
            return res;
        }
    }
    return 0 as png_fixed_point;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_microns(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).offset_unit_type as ::core::ffi::c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).x_offset;
        }
    }
    return 0 as png_int_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_microns(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).offset_unit_type as ::core::ffi::c_int == PNG_OFFSET_MICROMETER {
            return (*info_ptr).y_offset;
        }
    }
    return 0 as png_int_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_pixels(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).offset_unit_type as ::core::ffi::c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).x_offset;
        }
    }
    return 0 as png_int_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_pixels(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_int_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).offset_unit_type as ::core::ffi::c_int == PNG_OFFSET_PIXEL {
            return (*info_ptr).y_offset;
        }
    }
    return 0 as png_int_32;
}
unsafe extern "C" fn ppi_from_ppm(mut ppm: png_uint_32) -> png_uint_32 {
    let mut result: png_fixed_point = 0;
    if ppm <= PNG_UINT_31_MAX
        && png_muldiv(
            &raw mut result,
            ppm as png_fixed_point,
            127 as png_int_32,
            5000 as png_int_32,
        ) != 0 as ::core::ffi::c_int
    {
        return result as png_uint_32;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pixels_per_inch(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    return ppi_from_ppm(png_get_pixels_per_meter(png_ptr, info_ptr));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_pixels_per_inch(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    return ppi_from_ppm(png_get_x_pixels_per_meter(png_ptr, info_ptr));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_pixels_per_inch(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_uint_32 {
    return ppi_from_ppm(png_get_y_pixels_per_meter(png_ptr, info_ptr));
}
unsafe extern "C" fn png_fixed_inches_from_microns(
    mut png_ptr: png_const_structrp,
    mut microns: png_int_32,
) -> png_fixed_point {
    let mut result: png_fixed_point = 0;
    if png_muldiv(
        &raw mut result,
        microns as png_fixed_point,
        500 as png_int_32,
        127 as png_int_32,
    ) != 0 as ::core::ffi::c_int
    {
        return result;
    }
    png_warning(
        png_ptr,
        b"fixed point overflow ignored\0" as *const u8 as png_const_charp,
    );
    return 0 as png_fixed_point;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_inches_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_fixed_point {
    return png_fixed_inches_from_microns(png_ptr, png_get_x_offset_microns(png_ptr, info_ptr));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_inches_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_fixed_point {
    return png_fixed_inches_from_microns(png_ptr, png_get_y_offset_microns(png_ptr, info_ptr));
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_x_offset_inches(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> ::core::ffi::c_float {
    return (png_get_x_offset_microns(png_ptr, info_ptr) as ::core::ffi::c_double * 0.00003937f64)
        as ::core::ffi::c_float;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_y_offset_inches(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> ::core::ffi::c_float {
    return (png_get_y_offset_microns(png_ptr, info_ptr) as ::core::ffi::c_double * 0.00003937f64)
        as ::core::ffi::c_float;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs_dpi(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut res_x: *mut png_uint_32,
    mut res_y: *mut png_uint_32,
    mut unit_type: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0 as png_uint_32;
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if !res_x.is_null() {
            *res_x = (*info_ptr).x_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }
        if !res_y.is_null() {
            *res_y = (*info_ptr).y_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }
        if !unit_type.is_null() {
            *unit_type = (*info_ptr).phys_unit_type as ::core::ffi::c_int;
            retval |= PNG_INFO_pHYs;
            if *unit_type == 1 as ::core::ffi::c_int {
                if !res_x.is_null() {
                    *res_x = (*res_x as ::core::ffi::c_double * 0.0254f64 + 0.50f64) as png_uint_32;
                }
                if !res_y.is_null() {
                    *res_y = (*res_y as ::core::ffi::c_double * 0.0254f64 + 0.50f64) as png_uint_32;
                }
            }
        }
    }
    return retval;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_channels(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_byte {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*info_ptr).channels;
    }
    return 0 as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_signature(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
) -> png_const_bytep {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return &raw const (*info_ptr).signature as png_const_bytep;
    }
    return ::core::ptr::null::<png_byte>();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_bKGD(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut background: *mut png_color_16p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_bKGD != 0 as ::core::ffi::c_uint
        && !background.is_null()
    {
        *background = &raw mut (*info_ptr).background as png_color_16p;
        return PNG_INFO_bKGD;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut whitex: *mut ::core::ffi::c_double,
    mut whitey: *mut ::core::ffi::c_double,
    mut redx: *mut ::core::ffi::c_double,
    mut redy: *mut ::core::ffi::c_double,
    mut greenx: *mut ::core::ffi::c_double,
    mut greeny: *mut ::core::ffi::c_double,
    mut bluex: *mut ::core::ffi::c_double,
    mut bluey: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cHRM != 0 as ::core::ffi::c_uint
    {
        if !whitex.is_null() {
            *whitex = 0.00001f64 * (*info_ptr).cHRM.whitex as ::core::ffi::c_double;
        }
        if !whitey.is_null() {
            *whitey = 0.00001f64 * (*info_ptr).cHRM.whitey as ::core::ffi::c_double;
        }
        if !redx.is_null() {
            *redx = 0.00001f64 * (*info_ptr).cHRM.redx as ::core::ffi::c_double;
        }
        if !redy.is_null() {
            *redy = 0.00001f64 * (*info_ptr).cHRM.redy as ::core::ffi::c_double;
        }
        if !greenx.is_null() {
            *greenx = 0.00001f64 * (*info_ptr).cHRM.greenx as ::core::ffi::c_double;
        }
        if !greeny.is_null() {
            *greeny = 0.00001f64 * (*info_ptr).cHRM.greeny as ::core::ffi::c_double;
        }
        if !bluex.is_null() {
            *bluex = 0.00001f64 * (*info_ptr).cHRM.bluex as ::core::ffi::c_double;
        }
        if !bluey.is_null() {
            *bluey = 0.00001f64 * (*info_ptr).cHRM.bluey as ::core::ffi::c_double;
        }
        return PNG_INFO_cHRM;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut red_X: *mut ::core::ffi::c_double,
    mut red_Y: *mut ::core::ffi::c_double,
    mut red_Z: *mut ::core::ffi::c_double,
    mut green_X: *mut ::core::ffi::c_double,
    mut green_Y: *mut ::core::ffi::c_double,
    mut green_Z: *mut ::core::ffi::c_double,
    mut blue_X: *mut ::core::ffi::c_double,
    mut blue_Y: *mut ::core::ffi::c_double,
    mut blue_Z: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    let mut XYZ: png_XYZ = png_XYZ {
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
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cHRM != 0 as ::core::ffi::c_uint
        && png_XYZ_from_xy(&raw mut XYZ, &raw const (*info_ptr).cHRM) == 0 as ::core::ffi::c_int
    {
        if !red_X.is_null() {
            *red_X = 0.00001f64 * XYZ.red_X as ::core::ffi::c_double;
        }
        if !red_Y.is_null() {
            *red_Y = 0.00001f64 * XYZ.red_Y as ::core::ffi::c_double;
        }
        if !red_Z.is_null() {
            *red_Z = 0.00001f64 * XYZ.red_Z as ::core::ffi::c_double;
        }
        if !green_X.is_null() {
            *green_X = 0.00001f64 * XYZ.green_X as ::core::ffi::c_double;
        }
        if !green_Y.is_null() {
            *green_Y = 0.00001f64 * XYZ.green_Y as ::core::ffi::c_double;
        }
        if !green_Z.is_null() {
            *green_Z = 0.00001f64 * XYZ.green_Z as ::core::ffi::c_double;
        }
        if !blue_X.is_null() {
            *blue_X = 0.00001f64 * XYZ.blue_X as ::core::ffi::c_double;
        }
        if !blue_Y.is_null() {
            *blue_Y = 0.00001f64 * XYZ.blue_Y as ::core::ffi::c_double;
        }
        if !blue_Z.is_null() {
            *blue_Z = 0.00001f64 * XYZ.blue_Z as ::core::ffi::c_double;
        }
        return PNG_INFO_cHRM;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_XYZ_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut int_red_X: *mut png_fixed_point,
    mut int_red_Y: *mut png_fixed_point,
    mut int_red_Z: *mut png_fixed_point,
    mut int_green_X: *mut png_fixed_point,
    mut int_green_Y: *mut png_fixed_point,
    mut int_green_Z: *mut png_fixed_point,
    mut int_blue_X: *mut png_fixed_point,
    mut int_blue_Y: *mut png_fixed_point,
    mut int_blue_Z: *mut png_fixed_point,
) -> png_uint_32 {
    let mut XYZ: png_XYZ = png_XYZ {
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
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cHRM != 0 as ::core::ffi::c_uint
        && png_XYZ_from_xy(&raw mut XYZ, &raw const (*info_ptr).cHRM) == 0 as ::core::ffi::c_int
    {
        if !int_red_X.is_null() {
            *int_red_X = XYZ.red_X;
        }
        if !int_red_Y.is_null() {
            *int_red_Y = XYZ.red_Y;
        }
        if !int_red_Z.is_null() {
            *int_red_Z = XYZ.red_Z;
        }
        if !int_green_X.is_null() {
            *int_green_X = XYZ.green_X;
        }
        if !int_green_Y.is_null() {
            *int_green_Y = XYZ.green_Y;
        }
        if !int_green_Z.is_null() {
            *int_green_Z = XYZ.green_Z;
        }
        if !int_blue_X.is_null() {
            *int_blue_X = XYZ.blue_X;
        }
        if !int_blue_Y.is_null() {
            *int_blue_Y = XYZ.blue_Y;
        }
        if !int_blue_Z.is_null() {
            *int_blue_Z = XYZ.blue_Z;
        }
        return PNG_INFO_cHRM;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cHRM_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut whitex: *mut png_fixed_point,
    mut whitey: *mut png_fixed_point,
    mut redx: *mut png_fixed_point,
    mut redy: *mut png_fixed_point,
    mut greenx: *mut png_fixed_point,
    mut greeny: *mut png_fixed_point,
    mut bluex: *mut png_fixed_point,
    mut bluey: *mut png_fixed_point,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cHRM != 0 as ::core::ffi::c_uint
    {
        if !whitex.is_null() {
            *whitex = (*info_ptr).cHRM.whitex;
        }
        if !whitey.is_null() {
            *whitey = (*info_ptr).cHRM.whitey;
        }
        if !redx.is_null() {
            *redx = (*info_ptr).cHRM.redx;
        }
        if !redy.is_null() {
            *redy = (*info_ptr).cHRM.redy;
        }
        if !greenx.is_null() {
            *greenx = (*info_ptr).cHRM.greenx;
        }
        if !greeny.is_null() {
            *greeny = (*info_ptr).cHRM.greeny;
        }
        if !bluex.is_null() {
            *bluex = (*info_ptr).cHRM.bluex;
        }
        if !bluey.is_null() {
            *bluey = (*info_ptr).cHRM.bluey;
        }
        return PNG_INFO_cHRM;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut file_gamma: *mut png_fixed_point,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_gAMA != 0 as ::core::ffi::c_uint
    {
        if !file_gamma.is_null() {
            *file_gamma = (*info_ptr).gamma;
        }
        return PNG_INFO_gAMA;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_gAMA(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut file_gamma: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_gAMA != 0 as ::core::ffi::c_uint
    {
        if !file_gamma.is_null() {
            *file_gamma = 0.00001f64 * (*info_ptr).gamma as ::core::ffi::c_double;
        }
        return PNG_INFO_gAMA;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sRGB(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut file_srgb_intent: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sRGB != 0 as ::core::ffi::c_uint
    {
        if !file_srgb_intent.is_null() {
            *file_srgb_intent = (*info_ptr).rendering_intent;
        }
        return PNG_INFO_sRGB;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_iCCP(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut name: png_charpp,
    mut compression_type: *mut ::core::ffi::c_int,
    mut profile: png_bytepp,
    mut proflen: *mut png_uint_32,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_iCCP != 0 as ::core::ffi::c_uint
        && !name.is_null()
        && !profile.is_null()
        && !proflen.is_null()
    {
        *name = (*info_ptr).iccp_name as *mut ::core::ffi::c_char;
        *profile = (*info_ptr).iccp_profile as *mut png_byte;
        *proflen = ((*(*info_ptr).iccp_profile as png_uint_32) << 24 as ::core::ffi::c_int)
            .wrapping_add(
                (*(*info_ptr)
                    .iccp_profile
                    .offset(1 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 16 as ::core::ffi::c_int,
            )
            .wrapping_add(
                (*(*info_ptr)
                    .iccp_profile
                    .offset(2 as ::core::ffi::c_int as isize) as png_uint_32)
                    << 8 as ::core::ffi::c_int,
            )
            .wrapping_add(
                *(*info_ptr)
                    .iccp_profile
                    .offset(3 as ::core::ffi::c_int as isize) as png_uint_32,
            );
        if !compression_type.is_null() {
            *compression_type = PNG_COMPRESSION_TYPE_BASE;
        }
        return PNG_INFO_iCCP;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sPLT(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut spalettes: png_sPLT_tpp,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !spalettes.is_null() {
        *spalettes = (*info_ptr).splt_palettes as *mut png_sPLT_t;
        return (*info_ptr).splt_palettes_num;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cICP(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut colour_primaries: png_bytep,
    mut transfer_function: png_bytep,
    mut matrix_coefficients: png_bytep,
    mut video_full_range_flag: png_bytep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cICP != 0 as ::core::ffi::c_uint
        && !colour_primaries.is_null()
        && !transfer_function.is_null()
        && !matrix_coefficients.is_null()
        && !video_full_range_flag.is_null()
    {
        *colour_primaries = (*info_ptr).cicp_colour_primaries;
        *transfer_function = (*info_ptr).cicp_transfer_function;
        *matrix_coefficients = (*info_ptr).cicp_matrix_coefficients;
        *video_full_range_flag = (*info_ptr).cicp_video_full_range_flag;
        return 0x20000 as png_uint_32;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut maxCLL: png_uint_32p,
    mut maxFALL: png_uint_32p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cLLI != 0 as ::core::ffi::c_uint
    {
        if !maxCLL.is_null() {
            *maxCLL = (*info_ptr).maxCLL;
        }
        if !maxFALL.is_null() {
            *maxFALL = (*info_ptr).maxFALL;
        }
        return PNG_INFO_cLLI;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_cLLI(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut maxCLL: *mut ::core::ffi::c_double,
    mut maxFALL: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_cLLI != 0 as ::core::ffi::c_uint
    {
        if !maxCLL.is_null() {
            *maxCLL = (*info_ptr).maxCLL as ::core::ffi::c_double * 0.0001f64;
        }
        if !maxFALL.is_null() {
            *maxFALL = (*info_ptr).maxFALL as ::core::ffi::c_double * 0.0001f64;
        }
        return PNG_INFO_cLLI;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut white_x: *mut png_fixed_point,
    mut white_y: *mut png_fixed_point,
    mut red_x: *mut png_fixed_point,
    mut red_y: *mut png_fixed_point,
    mut green_x: *mut png_fixed_point,
    mut green_y: *mut png_fixed_point,
    mut blue_x: *mut png_fixed_point,
    mut blue_y: *mut png_fixed_point,
    mut mastering_maxDL: png_uint_32p,
    mut mastering_minDL: png_uint_32p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_mDCV != 0 as ::core::ffi::c_uint
    {
        if !white_x.is_null() {
            *white_x = ((*info_ptr).mastering_white_x as ::core::ffi::c_int
                * 2 as ::core::ffi::c_int) as png_fixed_point;
        }
        if !white_y.is_null() {
            *white_y = ((*info_ptr).mastering_white_y as ::core::ffi::c_int
                * 2 as ::core::ffi::c_int) as png_fixed_point;
        }
        if !red_x.is_null() {
            *red_x = ((*info_ptr).mastering_red_x as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as png_fixed_point;
        }
        if !red_y.is_null() {
            *red_y = ((*info_ptr).mastering_red_y as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as png_fixed_point;
        }
        if !green_x.is_null() {
            *green_x = ((*info_ptr).mastering_green_x as ::core::ffi::c_int
                * 2 as ::core::ffi::c_int) as png_fixed_point;
        }
        if !green_y.is_null() {
            *green_y = ((*info_ptr).mastering_green_y as ::core::ffi::c_int
                * 2 as ::core::ffi::c_int) as png_fixed_point;
        }
        if !blue_x.is_null() {
            *blue_x = ((*info_ptr).mastering_blue_x as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as png_fixed_point;
        }
        if !blue_y.is_null() {
            *blue_y = ((*info_ptr).mastering_blue_y as ::core::ffi::c_int * 2 as ::core::ffi::c_int)
                as png_fixed_point;
        }
        if !mastering_maxDL.is_null() {
            *mastering_maxDL = (*info_ptr).mastering_maxDL;
        }
        if !mastering_minDL.is_null() {
            *mastering_minDL = (*info_ptr).mastering_minDL;
        }
        return PNG_INFO_mDCV;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_mDCV(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut white_x: *mut ::core::ffi::c_double,
    mut white_y: *mut ::core::ffi::c_double,
    mut red_x: *mut ::core::ffi::c_double,
    mut red_y: *mut ::core::ffi::c_double,
    mut green_x: *mut ::core::ffi::c_double,
    mut green_y: *mut ::core::ffi::c_double,
    mut blue_x: *mut ::core::ffi::c_double,
    mut blue_y: *mut ::core::ffi::c_double,
    mut mastering_maxDL: *mut ::core::ffi::c_double,
    mut mastering_minDL: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_mDCV != 0 as ::core::ffi::c_uint
    {
        if !white_x.is_null() {
            *white_x = (*info_ptr).mastering_white_x as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !white_y.is_null() {
            *white_y = (*info_ptr).mastering_white_y as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !red_x.is_null() {
            *red_x = (*info_ptr).mastering_red_x as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !red_y.is_null() {
            *red_y = (*info_ptr).mastering_red_y as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !green_x.is_null() {
            *green_x = (*info_ptr).mastering_green_x as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !green_y.is_null() {
            *green_y = (*info_ptr).mastering_green_y as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !blue_x.is_null() {
            *blue_x = (*info_ptr).mastering_blue_x as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !blue_y.is_null() {
            *blue_y = (*info_ptr).mastering_blue_y as ::core::ffi::c_int as ::core::ffi::c_double
                * 0.00002f64;
        }
        if !mastering_maxDL.is_null() {
            *mastering_maxDL = (*info_ptr).mastering_maxDL as ::core::ffi::c_double * 0.0001f64;
        }
        if !mastering_minDL.is_null() {
            *mastering_minDL = (*info_ptr).mastering_minDL as ::core::ffi::c_double * 0.0001f64;
        }
        return PNG_INFO_mDCV;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut exif: *mut png_bytep,
) -> png_uint_32 {
    png_warning(
        png_ptr,
        b"png_get_eXIf does not work; use png_get_eXIf_1\0" as *const u8 as png_const_charp,
    );
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_eXIf_1(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut num_exif: *mut png_uint_32,
    mut exif: *mut png_bytep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_eXIf != 0 as ::core::ffi::c_uint
        && !exif.is_null()
    {
        *num_exif = (*info_ptr).num_exif;
        *exif = (*info_ptr).exif;
        return PNG_INFO_eXIf;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_hIST(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut hist: *mut png_uint_16p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_hIST != 0 as ::core::ffi::c_uint
        && !hist.is_null()
    {
        *hist = (*info_ptr).hist;
        return PNG_INFO_hIST;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_IHDR(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut width: *mut png_uint_32,
    mut height: *mut png_uint_32,
    mut bit_depth: *mut ::core::ffi::c_int,
    mut color_type: *mut ::core::ffi::c_int,
    mut interlace_type: *mut ::core::ffi::c_int,
    mut compression_type: *mut ::core::ffi::c_int,
    mut filter_type: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    if png_ptr.is_null() || info_ptr.is_null() {
        return 0 as png_uint_32;
    }
    if !width.is_null() {
        *width = (*info_ptr).width;
    }
    if !height.is_null() {
        *height = (*info_ptr).height;
    }
    if !bit_depth.is_null() {
        *bit_depth = (*info_ptr).bit_depth as ::core::ffi::c_int;
    }
    if !color_type.is_null() {
        *color_type = (*info_ptr).color_type as ::core::ffi::c_int;
    }
    if !compression_type.is_null() {
        *compression_type = (*info_ptr).compression_type as ::core::ffi::c_int;
    }
    if !filter_type.is_null() {
        *filter_type = (*info_ptr).filter_type as ::core::ffi::c_int;
    }
    if !interlace_type.is_null() {
        *interlace_type = (*info_ptr).interlace_type as ::core::ffi::c_int;
    }
    png_check_IHDR(
        png_ptr,
        (*info_ptr).width,
        (*info_ptr).height,
        (*info_ptr).bit_depth as ::core::ffi::c_int,
        (*info_ptr).color_type as ::core::ffi::c_int,
        (*info_ptr).interlace_type as ::core::ffi::c_int,
        (*info_ptr).compression_type as ::core::ffi::c_int,
        (*info_ptr).filter_type as ::core::ffi::c_int,
    );
    return 1 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_oFFs(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut offset_x: *mut png_int_32,
    mut offset_y: *mut png_int_32,
    mut unit_type: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_oFFs != 0 as ::core::ffi::c_uint
        && !offset_x.is_null()
        && !offset_y.is_null()
        && !unit_type.is_null()
    {
        *offset_x = (*info_ptr).x_offset;
        *offset_y = (*info_ptr).y_offset;
        *unit_type = (*info_ptr).offset_unit_type as ::core::ffi::c_int;
        return PNG_INFO_oFFs;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pCAL(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut purpose: *mut png_charp,
    mut X0: *mut png_int_32,
    mut X1: *mut png_int_32,
    mut type_0: *mut ::core::ffi::c_int,
    mut nparams: *mut ::core::ffi::c_int,
    mut units: *mut png_charp,
    mut params: *mut png_charpp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pCAL != 0 as ::core::ffi::c_uint
        && !purpose.is_null()
        && !X0.is_null()
        && !X1.is_null()
        && !type_0.is_null()
        && !nparams.is_null()
        && !units.is_null()
        && !params.is_null()
    {
        *purpose = (*info_ptr).pcal_purpose;
        *X0 = (*info_ptr).pcal_X0;
        *X1 = (*info_ptr).pcal_X1;
        *type_0 = (*info_ptr).pcal_type as ::core::ffi::c_int;
        *nparams = (*info_ptr).pcal_nparams as ::core::ffi::c_int;
        *units = (*info_ptr).pcal_units;
        *params = (*info_ptr).pcal_params;
        return PNG_INFO_pCAL;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut unit: *mut ::core::ffi::c_int,
    mut width: *mut png_fixed_point,
    mut height: *mut png_fixed_point,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sCAL != 0 as ::core::ffi::c_uint
    {
        *unit = (*info_ptr).scal_unit as ::core::ffi::c_int;
        *width = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_width as *const ::core::ffi::c_char),
            b"sCAL width\0" as *const u8 as png_const_charp,
        );
        *height = png_fixed(
            png_ptr,
            atof((*info_ptr).scal_s_height as *const ::core::ffi::c_char),
            b"sCAL height\0" as *const u8 as png_const_charp,
        );
        return PNG_INFO_sCAL;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut unit: *mut ::core::ffi::c_int,
    mut width: *mut ::core::ffi::c_double,
    mut height: *mut ::core::ffi::c_double,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sCAL != 0 as ::core::ffi::c_uint
    {
        *unit = (*info_ptr).scal_unit as ::core::ffi::c_int;
        *width = atof((*info_ptr).scal_s_width as *const ::core::ffi::c_char);
        *height = atof((*info_ptr).scal_s_height as *const ::core::ffi::c_char);
        return PNG_INFO_sCAL;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sCAL_s(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut unit: *mut ::core::ffi::c_int,
    mut width: png_charpp,
    mut height: png_charpp,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sCAL != 0 as ::core::ffi::c_uint
    {
        *unit = (*info_ptr).scal_unit as ::core::ffi::c_int;
        *width = (*info_ptr).scal_s_width as *mut ::core::ffi::c_char;
        *height = (*info_ptr).scal_s_height as *mut ::core::ffi::c_char;
        return PNG_INFO_sCAL;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_pHYs(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_const_inforp,
    mut res_x: *mut png_uint_32,
    mut res_y: *mut png_uint_32,
    mut unit_type: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0 as png_uint_32;
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_pHYs != 0 as ::core::ffi::c_uint
    {
        if !res_x.is_null() {
            *res_x = (*info_ptr).x_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }
        if !res_y.is_null() {
            *res_y = (*info_ptr).y_pixels_per_unit;
            retval |= PNG_INFO_pHYs;
        }
        if !unit_type.is_null() {
            *unit_type = (*info_ptr).phys_unit_type as ::core::ffi::c_int;
            retval |= PNG_INFO_pHYs;
        }
    }
    return retval;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_PLTE(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut palette: *mut png_colorp,
    mut num_palette: *mut ::core::ffi::c_int,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_PLTE != 0 as ::core::ffi::c_uint
        && !palette.is_null()
    {
        *palette = (*info_ptr).palette;
        *num_palette = (*info_ptr).num_palette as ::core::ffi::c_int;
        return PNG_INFO_PLTE;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_sBIT(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut sig_bit: *mut png_color_8p,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_sBIT != 0 as ::core::ffi::c_uint
        && !sig_bit.is_null()
    {
        *sig_bit = &raw mut (*info_ptr).sig_bit as png_color_8p;
        return PNG_INFO_sBIT;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_text(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut text_ptr: *mut png_textp,
    mut num_text: *mut ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && (*info_ptr).num_text > 0 as ::core::ffi::c_int {
        if !text_ptr.is_null() {
            *text_ptr = (*info_ptr).text;
        }
        if !num_text.is_null() {
            *num_text = (*info_ptr).num_text;
        }
        return (*info_ptr).num_text;
    }
    if !num_text.is_null() {
        *num_text = 0 as ::core::ffi::c_int;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tIME(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut mod_time: *mut png_timep,
) -> png_uint_32 {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_tIME != 0 as ::core::ffi::c_uint
        && !mod_time.is_null()
    {
        *mod_time = &raw mut (*info_ptr).mod_time as png_timep;
        return PNG_INFO_tIME;
    }
    return 0 as png_uint_32;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_tRNS(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut trans_alpha: *mut png_bytep,
    mut num_trans: *mut ::core::ffi::c_int,
    mut trans_color: *mut png_color_16p,
) -> png_uint_32 {
    let mut retval: png_uint_32 = 0 as png_uint_32;
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && (*info_ptr).valid as ::core::ffi::c_uint & PNG_INFO_tRNS != 0 as ::core::ffi::c_uint
    {
        if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
            if !trans_alpha.is_null() {
                *trans_alpha = (*info_ptr).trans_alpha;
                retval |= PNG_INFO_tRNS;
            }
            if !trans_color.is_null() {
                *trans_color = &raw mut (*info_ptr).trans_color as png_color_16p;
            }
        } else {
            if !trans_color.is_null() {
                *trans_color = &raw mut (*info_ptr).trans_color as png_color_16p;
                retval |= PNG_INFO_tRNS;
            }
            if !trans_alpha.is_null() {
                *trans_alpha = ::core::ptr::null_mut::<png_byte>();
            }
        }
        if !num_trans.is_null() {
            *num_trans = (*info_ptr).num_trans as ::core::ffi::c_int;
            retval |= PNG_INFO_tRNS;
        }
    }
    return retval;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_unknown_chunks(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut unknowns: png_unknown_chunkpp,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() && !unknowns.is_null() {
        *unknowns = (*info_ptr).unknown_chunks as *mut png_unknown_chunk;
        return (*info_ptr).unknown_chunks_num;
    }
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_rgb_to_gray_status(mut png_ptr: png_const_structrp) -> png_byte {
    return (if !png_ptr.is_null() {
        (*png_ptr).rgb_to_gray_status as ::core::ffi::c_int
    } else {
        0 as ::core::ffi::c_int
    }) as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_chunk_ptr(mut png_ptr: png_const_structrp) -> png_voidp {
    return if !png_ptr.is_null() {
        (*png_ptr).user_chunk_ptr
    } else {
        NULL_0
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_compression_buffer_size(
    mut png_ptr: png_const_structrp,
) -> size_t {
    if png_ptr.is_null() {
        return 0 as size_t;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint {
        return (*png_ptr).IDAT_read_size as size_t;
    } else {
        return (*png_ptr).zbuffer_size as size_t;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_width_max(mut png_ptr: png_const_structrp) -> png_uint_32 {
    return if !png_ptr.is_null() {
        (*png_ptr).user_width_max
    } else {
        0 as png_uint_32
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_user_height_max(mut png_ptr: png_const_structrp) -> png_uint_32 {
    return if !png_ptr.is_null() {
        (*png_ptr).user_height_max
    } else {
        0 as png_uint_32
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_cache_max(mut png_ptr: png_const_structrp) -> png_uint_32 {
    return if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max
    } else {
        0 as png_uint_32
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_chunk_malloc_max(
    mut png_ptr: png_const_structrp,
) -> png_alloc_size_t {
    return if !png_ptr.is_null() {
        (*png_ptr).user_chunk_malloc_max
    } else {
        0 as png_alloc_size_t
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_state(mut png_ptr: png_const_structrp) -> png_uint_32 {
    return (*png_ptr).io_state;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_io_chunk_type(mut png_ptr: png_const_structrp) -> png_uint_32 {
    return (*png_ptr).chunk_name;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_palette_max(
    mut png_ptr: png_const_structp,
    mut info_ptr: png_const_infop,
) -> ::core::ffi::c_int {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        return (*png_ptr).num_palette_max;
    }
    return -(1 as ::core::ffi::c_int);
}
pub const PNG_IS_READ_STRUCT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
