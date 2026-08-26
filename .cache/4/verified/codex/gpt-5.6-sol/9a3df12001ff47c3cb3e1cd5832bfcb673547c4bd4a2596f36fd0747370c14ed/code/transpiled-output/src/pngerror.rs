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
    fn abort() -> !;
    fn memcpy(
        __dest: *mut ::core::ffi::c_void,
        __src: *const ::core::ffi::c_void,
        __n: size_t,
    ) -> *mut ::core::ffi::c_void;
    static mut stdin: *mut FILE;
    static mut stdout: *mut FILE;
    static mut stderr: *mut FILE;
    fn fprintf(
        __stream: *mut FILE,
        __format: *const ::core::ffi::c_char,
        ...
    ) -> ::core::ffi::c_int;
    fn vfprintf(
        __s: *mut FILE,
        __format: *const ::core::ffi::c_char,
        __arg: *mut ::core::ffi::c_void,
    ) -> ::core::ffi::c_int;
    fn getc(__stream: *mut FILE) -> ::core::ffi::c_int;
    fn putc(__c: ::core::ffi::c_int, __stream: *mut FILE) -> ::core::ffi::c_int;
    fn _setjmp(__env: *mut __jmp_buf_tag) -> ::core::ffi::c_int;
    fn longjmp(__env: *mut __jmp_buf_tag, __val: ::core::ffi::c_int) -> !;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_image_free(image: png_imagep);
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
pub const PNG_IMAGE_WARNING: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_IMAGE_ERROR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_LITERAL_LEFT_SQUARE_BRACKET: ::core::ffi::c_int = 0x5b as ::core::ffi::c_int;
pub const PNG_LITERAL_RIGHT_SQUARE_BRACKET: ::core::ffi::c_int = 0x5d as ::core::ffi::c_int;
pub const PNG_STRING_NEWLINE: [::core::ffi::c_char; 2] =
    unsafe { ::core::mem::transmute::<[u8; 2], [::core::ffi::c_char; 2]>(*b"\n\0") };
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) -> ! {
    if !png_ptr.is_null() && (*png_ptr).error_fn.is_some() {
        Some((*png_ptr).error_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as *const ::core::ffi::c_void as *mut ::core::ffi::c_void as png_structp,
            error_message,
        );
    }
    png_default_error(png_ptr, error_message);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safecat(
    mut buffer: png_charp,
    mut bufsize: size_t,
    mut pos: size_t,
    mut string: png_const_charp,
) -> size_t {
    if !buffer.is_null() && pos < bufsize {
        if !string.is_null() {
            while *string as ::core::ffi::c_int != '\0' as i32
                && pos < bufsize.wrapping_sub(1 as size_t)
            {
                let fresh9 = string;
                string = string.offset(1);
                let fresh10 = pos;
                pos = pos.wrapping_add(1);
                *buffer.offset(fresh10 as isize) = *fresh9;
            }
        }
        *buffer.offset(pos as isize) = '\0' as i32 as ::core::ffi::c_char;
    }
    return pos;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_format_number(
    mut start: png_const_charp,
    mut end: png_charp,
    mut format: ::core::ffi::c_int,
    mut number: png_alloc_size_t,
) -> png_charp {
    let mut count: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut mincount: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    let mut output: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    end = end.offset(-1);
    *end = '\0' as i32 as ::core::ffi::c_char;
    while end > start as png_charp && (number != 0 as png_alloc_size_t || count < mincount) {
        static mut digits: [::core::ffi::c_char; 17] = unsafe {
            ::core::mem::transmute::<[u8; 17], [::core::ffi::c_char; 17]>(*b"0123456789ABCDEF\0")
        };
        let mut current_block_13: u64;
        match format {
            PNG_NUMBER_FORMAT_fixed => {
                mincount = 5 as ::core::ffi::c_int;
                if output != 0 as ::core::ffi::c_int
                    || number.wrapping_rem(10 as png_alloc_size_t) != 0 as png_alloc_size_t
                {
                    end = end.offset(-1);
                    *end = digits[number.wrapping_rem(10 as png_alloc_size_t) as usize];
                    output = 1 as ::core::ffi::c_int;
                }
                number = (number as ::core::ffi::c_ulong).wrapping_div(10 as ::core::ffi::c_ulong)
                    as png_alloc_size_t as png_alloc_size_t;
                current_block_13 = 1054647088692577877;
            }
            PNG_NUMBER_FORMAT_02u => {
                mincount = 2 as ::core::ffi::c_int;
                current_block_13 = 7970010499853014200;
            }
            PNG_NUMBER_FORMAT_u => {
                current_block_13 = 7970010499853014200;
            }
            PNG_NUMBER_FORMAT_02x => {
                mincount = 2 as ::core::ffi::c_int;
                current_block_13 = 6918155199378383918;
            }
            PNG_NUMBER_FORMAT_x => {
                current_block_13 = 6918155199378383918;
            }
            _ => {
                number = 0 as png_alloc_size_t;
                current_block_13 = 1054647088692577877;
            }
        }
        match current_block_13 {
            6918155199378383918 => {
                end = end.offset(-1);
                *end = digits[(number & 0xf as png_alloc_size_t) as usize];
                number >>= 4 as ::core::ffi::c_int;
            }
            7970010499853014200 => {
                end = end.offset(-1);
                *end = digits[number.wrapping_rem(10 as png_alloc_size_t) as usize];
                number = (number as ::core::ffi::c_ulong).wrapping_div(10 as ::core::ffi::c_ulong)
                    as png_alloc_size_t as png_alloc_size_t;
            }
            _ => {}
        }
        count += 1;
        if format == PNG_NUMBER_FORMAT_fixed
            && count == 5 as ::core::ffi::c_int
            && end > start as png_charp
        {
            if output != 0 as ::core::ffi::c_int {
                end = end.offset(-1);
                *end = '.' as i32 as ::core::ffi::c_char;
            } else if number == 0 as png_alloc_size_t {
                end = end.offset(-1);
                *end = '0' as i32 as ::core::ffi::c_char;
            }
        }
    }
    return end;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning(
    mut png_ptr: png_const_structrp,
    mut warning_message: png_const_charp,
) {
    let mut offset: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    if !png_ptr.is_null() && (*png_ptr).warning_fn.is_some() {
        Some((*png_ptr).warning_fn.expect("non-null function pointer"))
            .expect("non-null function pointer")(
            png_ptr as *const ::core::ffi::c_void as *mut ::core::ffi::c_void as png_structp,
            warning_message.offset(offset as isize),
        );
    } else {
        png_default_warning(png_ptr, warning_message.offset(offset as isize));
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter(
    mut p: *mut [::core::ffi::c_char; 32],
    mut number: ::core::ffi::c_int,
    mut string: png_const_charp,
) {
    if number > 0 as ::core::ffi::c_int && number <= PNG_WARNING_PARAMETER_COUNT {
        png_safecat(
            &raw mut *p.offset((number - 1 as ::core::ffi::c_int) as isize) as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 32]>() as size_t,
            0 as size_t,
            string,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_unsigned(
    mut p: *mut [::core::ffi::c_char; 32],
    mut number: ::core::ffi::c_int,
    mut format: ::core::ffi::c_int,
    mut value: png_alloc_size_t,
) {
    let mut buffer: [::core::ffi::c_char; 24] = [
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
        0,
        0,
        0,
        0,
        0,
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
    png_warning_parameter(
        p,
        number,
        png_format_number(
            &raw mut buffer as *mut ::core::ffi::c_char as png_const_charp,
            (&raw mut buffer as *mut ::core::ffi::c_char)
                .offset(::core::mem::size_of::<[::core::ffi::c_char; 24]>() as usize as isize),
            format,
            value,
        ) as png_const_charp,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_warning_parameter_signed(
    mut p: *mut [::core::ffi::c_char; 32],
    mut number: ::core::ffi::c_int,
    mut format: ::core::ffi::c_int,
    mut value: png_int_32,
) {
    let mut u: png_alloc_size_t = 0;
    let mut str: png_charp = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut buffer: [::core::ffi::c_char; 24] = [
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
        0,
        0,
        0,
        0,
        0,
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
    u = value as png_alloc_size_t;
    if value < 0 as ::core::ffi::c_int {
        u = (!u).wrapping_add(1 as png_alloc_size_t);
    }
    str = png_format_number(
        &raw mut buffer as *mut ::core::ffi::c_char as png_const_charp,
        (&raw mut buffer as *mut ::core::ffi::c_char)
            .offset(::core::mem::size_of::<[::core::ffi::c_char; 24]>() as usize as isize),
        format,
        u,
    );
    if value < 0 as ::core::ffi::c_int && str > &raw mut buffer as *mut ::core::ffi::c_char {
        str = str.offset(-1);
        *str = '-' as i32 as ::core::ffi::c_char;
    }
    png_warning_parameter(p, number, str as png_const_charp);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_formatted_warning(
    mut png_ptr: png_const_structrp,
    mut p: *mut [::core::ffi::c_char; 32],
    mut message: png_const_charp,
) {
    let mut i: size_t = 0 as size_t;
    let mut msg: [::core::ffi::c_char; 192] = [0; 192];
    while i
        < (::core::mem::size_of::<[::core::ffi::c_char; 192]>() as usize).wrapping_sub(1 as usize)
        && *message as ::core::ffi::c_int != '\0' as i32
    {
        if !p.is_null()
            && *message as ::core::ffi::c_int == '@' as i32
            && *message.offset(1 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
                != '\0' as i32
        {
            message = message.offset(1);
            let mut parameter_char: ::core::ffi::c_int = *message as ::core::ffi::c_int;
            static mut valid_parameters: [::core::ffi::c_char; 10] = unsafe {
                ::core::mem::transmute::<[u8; 10], [::core::ffi::c_char; 10]>(*b"123456789\0")
            };
            let mut parameter: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
            while valid_parameters[parameter as usize] as ::core::ffi::c_int != parameter_char
                && valid_parameters[parameter as usize] as ::core::ffi::c_int != '\0' as i32
            {
                parameter += 1;
            }
            if parameter < PNG_WARNING_PARAMETER_COUNT {
                let mut parm: png_const_charp = &raw mut *p.offset(parameter as isize)
                    as *mut ::core::ffi::c_char
                    as png_const_charp;
                let mut pend: png_const_charp = (&raw mut *p.offset(parameter as isize)
                    as *mut ::core::ffi::c_char)
                    .offset(::core::mem::size_of::<[::core::ffi::c_char; 32]>() as usize as isize)
                    as png_const_charp;
                while i
                    < (::core::mem::size_of::<[::core::ffi::c_char; 192]>() as usize)
                        .wrapping_sub(1 as usize)
                    && *parm as ::core::ffi::c_int != '\0' as i32
                    && parm < pend
                {
                    let fresh11 = parm;
                    parm = parm.offset(1);
                    let fresh12 = i;
                    i = i.wrapping_add(1);
                    msg[fresh12 as usize] = *fresh11;
                }
                message = message.offset(1);
                continue;
            }
        }
        let fresh13 = message;
        message = message.offset(1);
        let fresh14 = i;
        i = i.wrapping_add(1);
        msg[fresh14 as usize] = *fresh13;
    }
    msg[i as usize] = '\0' as i32 as ::core::ffi::c_char;
    png_warning(
        png_ptr,
        &raw mut msg as *mut ::core::ffi::c_char as png_const_charp,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_benign_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) {
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_BENIGN_ERRORS_WARN
        != 0 as ::core::ffi::c_uint
    {
        if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint
            && (*png_ptr).chunk_name != 0 as ::core::ffi::c_uint
        {
            png_chunk_warning(png_ptr, error_message);
        } else {
            png_warning(png_ptr, error_message);
        }
    } else if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT
        != 0 as ::core::ffi::c_uint
        && (*png_ptr).chunk_name != 0 as ::core::ffi::c_uint
    {
        png_chunk_error(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_warning(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) {
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_APP_WARNINGS_WARN
        != 0 as ::core::ffi::c_uint
    {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_app_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) {
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_APP_ERRORS_WARN
        != 0 as ::core::ffi::c_uint
    {
        png_warning(png_ptr, error_message);
    } else {
        png_error(png_ptr, error_message);
    };
}
pub const PNG_MAX_ERROR_TEXT: ::core::ffi::c_int = 196 as ::core::ffi::c_int;
static mut png_digit: [::core::ffi::c_char; 16] = [
    '0' as i32 as ::core::ffi::c_char,
    '1' as i32 as ::core::ffi::c_char,
    '2' as i32 as ::core::ffi::c_char,
    '3' as i32 as ::core::ffi::c_char,
    '4' as i32 as ::core::ffi::c_char,
    '5' as i32 as ::core::ffi::c_char,
    '6' as i32 as ::core::ffi::c_char,
    '7' as i32 as ::core::ffi::c_char,
    '8' as i32 as ::core::ffi::c_char,
    '9' as i32 as ::core::ffi::c_char,
    'A' as i32 as ::core::ffi::c_char,
    'B' as i32 as ::core::ffi::c_char,
    'C' as i32 as ::core::ffi::c_char,
    'D' as i32 as ::core::ffi::c_char,
    'E' as i32 as ::core::ffi::c_char,
    'F' as i32 as ::core::ffi::c_char,
];
unsafe extern "C" fn png_format_buffer(
    mut png_ptr: png_const_structrp,
    mut buffer: png_charp,
    mut error_message: png_const_charp,
) {
    let mut chunk_name: png_uint_32 = (*png_ptr).chunk_name;
    let mut iout: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut ishift: ::core::ffi::c_int = 24 as ::core::ffi::c_int;
    while ishift >= 0 as ::core::ffi::c_int {
        let mut c: ::core::ffi::c_int =
            (chunk_name >> ishift) as ::core::ffi::c_int & 0xff as ::core::ffi::c_int;
        ishift -= 8 as ::core::ffi::c_int;
        if (c < 65 as ::core::ffi::c_int
            || c > 122 as ::core::ffi::c_int
            || c > 90 as ::core::ffi::c_int && c < 97 as ::core::ffi::c_int)
            as ::core::ffi::c_int
            != 0 as ::core::ffi::c_int
        {
            let fresh0 = iout;
            iout = iout + 1;
            *buffer.offset(fresh0 as isize) =
                PNG_LITERAL_LEFT_SQUARE_BRACKET as ::core::ffi::c_char;
            let fresh1 = iout;
            iout = iout + 1;
            *buffer.offset(fresh1 as isize) =
                png_digit[((c & 0xf0 as ::core::ffi::c_int) >> 4 as ::core::ffi::c_int) as usize];
            let fresh2 = iout;
            iout = iout + 1;
            *buffer.offset(fresh2 as isize) = png_digit[(c & 0xf as ::core::ffi::c_int) as usize];
            let fresh3 = iout;
            iout = iout + 1;
            *buffer.offset(fresh3 as isize) =
                PNG_LITERAL_RIGHT_SQUARE_BRACKET as ::core::ffi::c_char;
        } else {
            let fresh4 = iout;
            iout = iout + 1;
            *buffer.offset(fresh4 as isize) = c as ::core::ffi::c_char;
        }
    }
    if error_message.is_null() {
        *buffer.offset(iout as isize) = '\0' as i32 as ::core::ffi::c_char;
    } else {
        let mut iin: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
        let fresh5 = iout;
        iout = iout + 1;
        *buffer.offset(fresh5 as isize) = ':' as i32 as ::core::ffi::c_char;
        let fresh6 = iout;
        iout = iout + 1;
        *buffer.offset(fresh6 as isize) = ' ' as i32 as ::core::ffi::c_char;
        while iin < PNG_MAX_ERROR_TEXT - 1 as ::core::ffi::c_int
            && *error_message.offset(iin as isize) as ::core::ffi::c_int != '\0' as i32
        {
            let fresh7 = iin;
            iin = iin + 1;
            let fresh8 = iout;
            iout = iout + 1;
            *buffer.offset(fresh8 as isize) = *error_message.offset(fresh7 as isize);
        }
        *buffer.offset(iout as isize) = '\0' as i32 as ::core::ffi::c_char;
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) -> ! {
    let mut msg: [::core::ffi::c_char; 214] = [0; 214];
    if png_ptr.is_null() {
        png_error(png_ptr, error_message);
    } else {
        png_format_buffer(png_ptr, &raw mut msg as png_charp, error_message);
        png_error(
            png_ptr,
            &raw mut msg as *mut ::core::ffi::c_char as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_warning(
    mut png_ptr: png_const_structrp,
    mut warning_message: png_const_charp,
) {
    let mut msg: [::core::ffi::c_char; 214] = [0; 214];
    if png_ptr.is_null() {
        png_warning(png_ptr, warning_message);
    } else {
        png_format_buffer(png_ptr, &raw mut msg as png_charp, warning_message);
        png_warning(
            png_ptr,
            &raw mut msg as *mut ::core::ffi::c_char as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_benign_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) {
    if (*png_ptr).flags as ::core::ffi::c_uint & PNG_FLAG_BENIGN_ERRORS_WARN
        != 0 as ::core::ffi::c_uint
    {
        png_chunk_warning(png_ptr, error_message);
    } else {
        png_chunk_error(png_ptr, error_message);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_chunk_report(
    mut png_ptr: png_const_structrp,
    mut message: png_const_charp,
    mut error: ::core::ffi::c_int,
) {
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint {
        if error < PNG_CHUNK_ERROR {
            png_chunk_warning(png_ptr, message);
        } else {
            png_chunk_benign_error(png_ptr, message);
        }
    } else if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT
        == 0 as ::core::ffi::c_uint
    {
        if error < PNG_CHUNK_WRITE_ERROR {
            png_app_warning(png_ptr, message);
        } else {
            png_app_error(png_ptr, message);
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_fixed_error(
    mut png_ptr: png_const_structrp,
    mut name: png_const_charp,
) -> ! {
    let mut iin: ::core::ffi::c_uint = 0;
    let mut msg: [::core::ffi::c_char; 220] = [0; 220];
    memcpy(
        &raw mut msg as *mut ::core::ffi::c_char as *mut ::core::ffi::c_void,
        fixed_message.as_ptr() as *const ::core::ffi::c_void,
        fixed_message_ln,
    );
    iin = 0 as ::core::ffi::c_uint;
    if !name.is_null() {
        while iin < (PNG_MAX_ERROR_TEXT - 1 as ::core::ffi::c_int) as ::core::ffi::c_uint
            && *name.offset(iin as isize) as ::core::ffi::c_int != 0 as ::core::ffi::c_int
        {
            msg[fixed_message_ln.wrapping_add(iin as usize) as usize] = *name.offset(iin as isize);
            iin = iin.wrapping_add(1);
        }
    }
    msg[fixed_message_ln.wrapping_add(iin as usize) as usize] = 0 as ::core::ffi::c_char;
    png_error(
        png_ptr,
        &raw mut msg as *mut ::core::ffi::c_char as png_const_charp,
    );
}
pub const fixed_message: [::core::ffi::c_char; 25] = unsafe {
    ::core::mem::transmute::<[u8; 25], [::core::ffi::c_char; 25]>(*b"fixed point overflow in \0")
};
pub const fixed_message_ln: usize =
    (::core::mem::size_of::<[::core::ffi::c_char; 25]>() as usize).wrapping_sub(1 as usize);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_longjmp_fn(
    mut png_ptr: png_structrp,
    mut longjmp_fn: png_longjmp_ptr,
    mut jmp_buf_size: size_t,
) -> *mut jmp_buf {
    if png_ptr.is_null() {
        return ::core::ptr::null_mut::<jmp_buf>();
    }
    if (*png_ptr).jmp_buf_ptr.is_null() {
        (*png_ptr).jmp_buf_size = 0 as size_t;
        if jmp_buf_size <= ::core::mem::size_of::<jmp_buf>() as usize {
            (*png_ptr).jmp_buf_ptr = &raw mut (*png_ptr).jmp_buf_local;
        } else {
            (*png_ptr).jmp_buf_ptr =
                png_malloc_warn(png_ptr, jmp_buf_size as png_alloc_size_t) as *mut jmp_buf;
            if (*png_ptr).jmp_buf_ptr.is_null() {
                return ::core::ptr::null_mut::<jmp_buf>();
            }
            (*png_ptr).jmp_buf_size = jmp_buf_size;
        }
    } else {
        let mut size: size_t = (*png_ptr).jmp_buf_size;
        if size == 0 as size_t {
            size = ::core::mem::size_of::<jmp_buf>() as usize as size_t;
            if (*png_ptr).jmp_buf_ptr != &raw mut (*png_ptr).jmp_buf_local {
                png_error(
                    png_ptr,
                    b"Libpng jmp_buf still allocated\0" as *const u8 as png_const_charp,
                );
            }
        }
        if size != jmp_buf_size {
            png_warning(
                png_ptr,
                b"Application jmp_buf size changed\0" as *const u8 as png_const_charp,
            );
            return ::core::ptr::null_mut::<jmp_buf>();
        }
    }
    (*png_ptr).longjmp_fn = longjmp_fn;
    return (*png_ptr).jmp_buf_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_free_jmpbuf(mut png_ptr: png_structrp) {
    if !png_ptr.is_null() {
        let mut jb: *mut jmp_buf = (*png_ptr).jmp_buf_ptr;
        if !jb.is_null() && (*png_ptr).jmp_buf_size > 0 as size_t {
            if jb != &raw mut (*png_ptr).jmp_buf_local {
                let mut free_jmp_buf: jmp_buf = [__jmp_buf_tag {
                    __jmpbuf: [0; 8],
                    __mask_was_saved: 0,
                    __saved_mask: __sigset_t { __val: [0; 16] },
                }; 1];
                if _setjmp(&raw mut free_jmp_buf as *mut __jmp_buf_tag) == 0 {
                    (*png_ptr).jmp_buf_ptr = &raw mut free_jmp_buf;
                    (*png_ptr).jmp_buf_size = 0 as size_t;
                    (*png_ptr).longjmp_fn = ::core::mem::transmute::<
                        Option<unsafe extern "C" fn(*mut __jmp_buf_tag, ::core::ffi::c_int) -> !>,
                        png_longjmp_ptr,
                    >(Some(
                        longjmp
                            as unsafe extern "C" fn(*mut __jmp_buf_tag, ::core::ffi::c_int) -> !,
                    ));
                    png_free(png_ptr, jb as png_voidp);
                }
            }
        }
        (*png_ptr).jmp_buf_size = 0 as size_t;
        (*png_ptr).jmp_buf_ptr = ::core::ptr::null_mut::<jmp_buf>();
        (*png_ptr).longjmp_fn = None;
    }
}
unsafe extern "C" fn png_default_error(
    mut png_ptr: png_const_structrp,
    mut error_message: png_const_charp,
) -> ! {
    fprintf(
        stderr,
        b"libpng error: %s\0" as *const u8 as *const ::core::ffi::c_char,
        if !error_message.is_null() {
            error_message
        } else {
            b"undefined\0" as *const u8 as png_const_charp
        },
    );
    fprintf(stderr, PNG_STRING_NEWLINE.as_ptr());
    png_longjmp(png_ptr, 1 as ::core::ffi::c_int);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_longjmp(
    mut png_ptr: png_const_structrp,
    mut val: ::core::ffi::c_int,
) -> ! {
    if !png_ptr.is_null() && (*png_ptr).longjmp_fn.is_some() && !(*png_ptr).jmp_buf_ptr.is_null() {
        (*png_ptr).longjmp_fn.expect("non-null function pointer")(
            &raw mut *(*png_ptr).jmp_buf_ptr as *mut __jmp_buf_tag,
            val,
        );
    }
    abort();
}
unsafe extern "C" fn png_default_warning(
    mut png_ptr: png_const_structrp,
    mut warning_message: png_const_charp,
) {
    fprintf(
        stderr,
        b"libpng warning: %s\0" as *const u8 as *const ::core::ffi::c_char,
        warning_message,
    );
    fprintf(stderr, PNG_STRING_NEWLINE.as_ptr());
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_error_fn(
    mut png_ptr: png_structrp,
    mut error_ptr: png_voidp,
    mut error_fn: png_error_ptr,
    mut warning_fn: png_error_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).error_ptr = error_ptr;
    (*png_ptr).error_fn = error_fn;
    (*png_ptr).warning_fn = warning_fn;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_get_error_ptr(mut png_ptr: png_const_structrp) -> png_voidp {
    if png_ptr.is_null() {
        return NULL_0;
    }
    return (*png_ptr).error_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_error(
    mut png_nonconst_ptr: png_structp,
    mut error_message: png_const_charp,
) -> ! {
    let mut png_ptr: png_const_structrp = png_nonconst_ptr as png_const_structrp;
    let mut image: png_imagep = (*png_ptr).error_ptr as png_imagep;
    if !image.is_null() {
        png_safecat(
            &raw mut (*image).message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
            0 as size_t,
            error_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_ERROR as ::core::ffi::c_uint;
        if !(*image).opaque.is_null() && !(*(*image).opaque).error_buf.is_null() {
            longjmp(
                (*(*image).opaque).error_buf as *mut __jmp_buf_tag,
                1 as ::core::ffi::c_int,
            );
        }
        let mut pos: size_t = png_safecat(
            &raw mut (*image).message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
            0 as size_t,
            b"bad longjmp: \0" as *const u8 as png_const_charp,
        );
        png_safecat(
            &raw mut (*image).message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
            pos,
            error_message,
        );
    }
    abort();
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_warning(
    mut png_nonconst_ptr: png_structp,
    mut warning_message: png_const_charp,
) {
    let mut png_ptr: png_const_structrp = png_nonconst_ptr as png_const_structrp;
    let mut image: png_imagep = (*png_ptr).error_ptr as png_imagep;
    if (*image).warning_or_error == 0 as ::core::ffi::c_uint {
        png_safecat(
            &raw mut (*image).message as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 64]>() as size_t,
            0 as size_t,
            warning_message,
        );
        (*image).warning_or_error |= PNG_IMAGE_WARNING as ::core::ffi::c_uint;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_safe_execute(
    mut image: png_imagep,
    mut function: Option<unsafe extern "C" fn(png_voidp) -> ::core::ffi::c_int>,
    mut arg: png_voidp,
) -> ::core::ffi::c_int {
    let saved_error_buf: png_voidp = (*(*image).opaque).error_buf;
    let mut safe_jmpbuf: jmp_buf = [__jmp_buf_tag {
        __jmpbuf: [0; 8],
        __mask_was_saved: 0,
        __saved_mask: __sigset_t { __val: [0; 16] },
    }; 1];
    if _setjmp(&raw mut safe_jmpbuf as *mut __jmp_buf_tag) == 0 as ::core::ffi::c_int {
        let mut result: ::core::ffi::c_int = 0;
        (*(*image).opaque).error_buf = &raw mut safe_jmpbuf as *mut __jmp_buf_tag as png_voidp;
        result = function.expect("non-null function pointer")(arg);
        (*(*image).opaque).error_buf = saved_error_buf;
        if result != 0 {
            return 1 as ::core::ffi::c_int;
        }
    }
    (*(*image).opaque).error_buf = saved_error_buf;
    if saved_error_buf.is_null() {
        png_image_free(image);
    }
    return 0 as ::core::ffi::c_int;
}
pub const PNG_IS_READ_STRUCT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_FLAG_BENIGN_ERRORS_WARN: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const PNG_FLAG_APP_WARNINGS_WARN: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PNG_FLAG_APP_ERRORS_WARN: ::core::ffi::c_uint = 0x400000 as ::core::ffi::c_uint;
pub const PNG_NUMBER_FORMAT_u: ::core::ffi::c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: ::core::ffi::c_int = 2;
pub const PNG_NUMBER_FORMAT_x: ::core::ffi::c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: ::core::ffi::c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const PNG_WARNING_PARAMETER_COUNT: ::core::ffi::c_int = 8 as ::core::ffi::c_int;
pub const PNG_CHUNK_WRITE_ERROR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_CHUNK_ERROR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
