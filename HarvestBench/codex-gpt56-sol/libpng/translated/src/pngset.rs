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
    fn png_malloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_calloc(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_warn(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_free(png_ptr: png_const_structrp, ptr: png_voidp);
    fn png_free_data(
        png_ptr: png_const_structrp,
        info_ptr: png_inforp,
        free_me: png_uint_32,
        num: ::core::ffi::c_int,
    );
    fn png_error(png_ptr: png_const_structrp, error_message: png_const_charp) -> !;
    fn png_warning(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_benign_error(png_ptr: png_const_structrp, warning_message: png_const_charp);
    fn png_free_buffer_list(png_ptr: png_structrp, list: *mut png_compression_bufferp);
    fn png_fixed(
        png_ptr: png_const_structrp,
        fp: ::core::ffi::c_double,
        text: png_const_charp,
    ) -> png_fixed_point;
    fn png_fixed_ITU(
        png_ptr: png_const_structrp,
        fp: ::core::ffi::c_double,
        text: png_const_charp,
    ) -> png_uint_32;
    fn png_malloc_base(png_ptr: png_const_structrp, size: png_alloc_size_t) -> png_voidp;
    fn png_malloc_array(
        png_ptr: png_const_structrp,
        nelements: ::core::ffi::c_int,
        element_size: size_t,
    ) -> png_voidp;
    fn png_realloc_array(
        png_ptr: png_const_structrp,
        array: png_const_voidp,
        old_elements: ::core::ffi::c_int,
        add_elements: ::core::ffi::c_int,
        element_size: size_t,
    ) -> png_voidp;
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
    fn png_warning_parameter(
        p: *mut [::core::ffi::c_char; 32],
        number: ::core::ffi::c_int,
        string: png_const_charp,
    );
    fn png_warning_parameter_signed(
        p: *mut [::core::ffi::c_char; 32],
        number: ::core::ffi::c_int,
        format: ::core::ffi::c_int,
        value: png_int_32,
    );
    fn png_formatted_warning(
        png_ptr: png_const_structrp,
        p: *mut [::core::ffi::c_char; 32],
        message: png_const_charp,
    );
    fn png_app_warning(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_app_error(png_ptr: png_const_structrp, message: png_const_charp);
    fn png_chunk_report(
        png_ptr: png_const_structrp,
        message: png_const_charp,
        error: ::core::ffi::c_int,
    );
    fn png_ascii_from_fp(
        png_ptr: png_const_structrp,
        ascii: png_charp,
        size: size_t,
        fp: ::core::ffi::c_double,
        precision: ::core::ffi::c_uint,
    );
    fn png_ascii_from_fixed(
        png_ptr: png_const_structrp,
        ascii: png_charp,
        size: size_t,
        fp: png_fixed_point,
    );
    fn png_check_fp_string(string: png_const_charp, size: size_t) -> ::core::ffi::c_int;
    fn png_xy_from_XYZ(xy: *mut png_xy, XYZ: *const png_XYZ) -> ::core::ffi::c_int;
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
pub type png_const_voidp = *const ::core::ffi::c_void;
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
pub type png_inforp = *mut png_info;
pub type png_const_colorp = *const png_color;
pub type png_const_color_16p = *const png_color_16;
pub type png_const_color_8p = *const png_color_8;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_const_textp = *const png_text;
pub type png_const_timep = *const png_time;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
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
pub type png_warning_parameters = [[::core::ffi::c_char; 32]; 8];
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
pub const PNG_ITXT_COMPRESSION_NONE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_TEXT_COMPRESSION_LAST: ::core::ffi::c_int = 3 as ::core::ffi::c_int;
pub const PNG_HAVE_IHDR: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_HAVE_PLTE: ::core::ffi::c_int = 0x2 as ::core::ffi::c_int;
pub const PNG_AFTER_IDAT: ::core::ffi::c_int = 0x8 as ::core::ffi::c_int;
pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff as ::core::ffi::c_long as png_uint_32;
pub const PNG_SIZE_MAX: size_t = -(1 as ::core::ffi::c_int) as size_t;
pub const PNG_COLOR_MASK_PALETTE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_COLOR: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COLOR_MASK_ALPHA: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_GRAY: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_COLOR_TYPE_PALETTE: ::core::ffi::c_int =
    PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: ::core::ffi::c_int = 2 as ::core::ffi::c_int;
pub const PNG_COMPRESSION_TYPE_BASE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
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
pub const PNG_FLAG_MNG_EMPTY_PLTE: ::core::ffi::c_int = 0x1 as ::core::ffi::c_int;
pub const PNG_ALL_MNG_FEATURES: ::core::ffi::c_int = 0x5 as ::core::ffi::c_int;
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
pub const PNG_HANDLE_CHUNK_AS_DEFAULT: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
pub const PNG_HANDLE_CHUNK_LAST: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const ZLIB_IO_MAX: uInt = -(1 as ::core::ffi::c_int) as uInt;
pub const PNG_sCAL_PRECISION: ::core::ffi::c_int = 5 as ::core::ffi::c_int;
pub const INT_MAX: ::core::ffi::c_int = __INT_MAX__;
pub const UINT_MAX: ::core::ffi::c_uint = (__INT_MAX__ as ::core::ffi::c_uint)
    .wrapping_mul(2 as ::core::ffi::c_uint)
    .wrapping_add(1 as ::core::ffi::c_uint);
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_bKGD(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut background: png_const_color_16p,
) {
    if png_ptr.is_null() || info_ptr.is_null() || background.is_null() {
        return;
    }
    (*info_ptr).background = *background;
    (*info_ptr).valid |= PNG_INFO_bKGD;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut white_x: png_fixed_point,
    mut white_y: png_fixed_point,
    mut red_x: png_fixed_point,
    mut red_y: png_fixed_point,
    mut green_x: png_fixed_point,
    mut green_y: png_fixed_point,
    mut blue_x: png_fixed_point,
    mut blue_y: png_fixed_point,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).cHRM.redx = red_x;
    (*info_ptr).cHRM.redy = red_y;
    (*info_ptr).cHRM.greenx = green_x;
    (*info_ptr).cHRM.greeny = green_y;
    (*info_ptr).cHRM.bluex = blue_x;
    (*info_ptr).cHRM.bluey = blue_y;
    (*info_ptr).cHRM.whitex = white_x;
    (*info_ptr).cHRM.whitey = white_y;
    (*info_ptr).valid |= PNG_INFO_cHRM;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_XYZ_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut int_red_X: png_fixed_point,
    mut int_red_Y: png_fixed_point,
    mut int_red_Z: png_fixed_point,
    mut int_green_X: png_fixed_point,
    mut int_green_Y: png_fixed_point,
    mut int_green_Z: png_fixed_point,
    mut int_blue_X: png_fixed_point,
    mut int_blue_Y: png_fixed_point,
    mut int_blue_Z: png_fixed_point,
) {
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
    let mut xy: png_xy = png_xy {
        redx: 0,
        redy: 0,
        greenx: 0,
        greeny: 0,
        bluex: 0,
        bluey: 0,
        whitex: 0,
        whitey: 0,
    };
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    XYZ.red_X = int_red_X;
    XYZ.red_Y = int_red_Y;
    XYZ.red_Z = int_red_Z;
    XYZ.green_X = int_green_X;
    XYZ.green_Y = int_green_Y;
    XYZ.green_Z = int_green_Z;
    XYZ.blue_X = int_blue_X;
    XYZ.blue_Y = int_blue_Y;
    XYZ.blue_Z = int_blue_Z;
    if png_xy_from_XYZ(&raw mut xy, &raw mut XYZ) == 0 as ::core::ffi::c_int {
        (*info_ptr).cHRM = xy;
        (*info_ptr).valid |= PNG_INFO_cHRM;
    } else {
        png_app_error(
            png_ptr,
            b"invalid cHRM XYZ\0" as *const u8 as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut white_x: ::core::ffi::c_double,
    mut white_y: ::core::ffi::c_double,
    mut red_x: ::core::ffi::c_double,
    mut red_y: ::core::ffi::c_double,
    mut green_x: ::core::ffi::c_double,
    mut green_y: ::core::ffi::c_double,
    mut blue_x: ::core::ffi::c_double,
    mut blue_y: ::core::ffi::c_double,
) {
    png_set_cHRM_fixed(
        png_ptr,
        info_ptr,
        png_fixed(
            png_ptr,
            white_x,
            b"cHRM White X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            white_y,
            b"cHRM White Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_x,
            b"cHRM Red X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_y,
            b"cHRM Red Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_x,
            b"cHRM Green X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_y,
            b"cHRM Green Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_x,
            b"cHRM Blue X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_y,
            b"cHRM Blue Y\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cHRM_XYZ(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut red_X: ::core::ffi::c_double,
    mut red_Y: ::core::ffi::c_double,
    mut red_Z: ::core::ffi::c_double,
    mut green_X: ::core::ffi::c_double,
    mut green_Y: ::core::ffi::c_double,
    mut green_Z: ::core::ffi::c_double,
    mut blue_X: ::core::ffi::c_double,
    mut blue_Y: ::core::ffi::c_double,
    mut blue_Z: ::core::ffi::c_double,
) {
    png_set_cHRM_XYZ_fixed(
        png_ptr,
        info_ptr,
        png_fixed(
            png_ptr,
            red_X,
            b"cHRM Red X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_Y,
            b"cHRM Red Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_Z,
            b"cHRM Red Z\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_X,
            b"cHRM Green X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_Y,
            b"cHRM Green Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_Z,
            b"cHRM Green Z\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_X,
            b"cHRM Blue X\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_Y,
            b"cHRM Blue Y\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_Z,
            b"cHRM Blue Z\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cICP(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut colour_primaries: png_byte,
    mut transfer_function: png_byte,
    mut matrix_coefficients: png_byte,
    mut video_full_range_flag: png_byte,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).cicp_colour_primaries = colour_primaries;
    (*info_ptr).cicp_transfer_function = transfer_function;
    (*info_ptr).cicp_matrix_coefficients = matrix_coefficients;
    (*info_ptr).cicp_video_full_range_flag = video_full_range_flag;
    if (*info_ptr).cicp_matrix_coefficients as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Invalid cICP matrix coefficients\0" as *const u8 as png_const_charp,
        );
        return;
    }
    (*info_ptr).valid |= PNG_INFO_cICP;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut maxCLL: png_uint_32,
    mut maxFALL: png_uint_32,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if maxCLL > 0x7fffffff as ::core::ffi::c_uint || maxFALL > 0x7fffffff as ::core::ffi::c_uint {
        png_chunk_report(
            png_ptr,
            b"cLLI light level exceeds PNG limit\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    (*info_ptr).maxCLL = maxCLL;
    (*info_ptr).maxFALL = maxFALL;
    (*info_ptr).valid |= PNG_INFO_cLLI;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_cLLI(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut maxCLL: ::core::ffi::c_double,
    mut maxFALL: ::core::ffi::c_double,
) {
    png_set_cLLI_fixed(
        png_ptr,
        info_ptr,
        png_fixed_ITU(
            png_ptr,
            maxCLL,
            b"png_set_cLLI(maxCLL)\0" as *const u8 as png_const_charp,
        ),
        png_fixed_ITU(
            png_ptr,
            maxFALL,
            b"png_set_cLLI(maxFALL)\0" as *const u8 as png_const_charp,
        ),
    );
}
unsafe extern "C" fn png_ITU_fixed_16(
    mut error: *mut ::core::ffi::c_int,
    mut v: png_fixed_point,
) -> png_uint_16 {
    v /= 2 as ::core::ffi::c_int;
    if v > 65535 as ::core::ffi::c_int || v < 0 as ::core::ffi::c_int {
        *error = 1 as ::core::ffi::c_int;
        return 0 as png_uint_16;
    }
    return v as png_uint_16;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mDCV_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut white_x: png_fixed_point,
    mut white_y: png_fixed_point,
    mut red_x: png_fixed_point,
    mut red_y: png_fixed_point,
    mut green_x: png_fixed_point,
    mut green_y: png_fixed_point,
    mut blue_x: png_fixed_point,
    mut blue_y: png_fixed_point,
    mut maxDL: png_uint_32,
    mut minDL: png_uint_32,
) {
    let mut rx: png_uint_16 = 0;
    let mut ry: png_uint_16 = 0;
    let mut gx: png_uint_16 = 0;
    let mut gy: png_uint_16 = 0;
    let mut bx: png_uint_16 = 0;
    let mut by: png_uint_16 = 0;
    let mut wx: png_uint_16 = 0;
    let mut wy: png_uint_16 = 0;
    let mut error: ::core::ffi::c_int = 0;
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    error = 0 as ::core::ffi::c_int;
    rx = png_ITU_fixed_16(&raw mut error, red_x);
    ry = png_ITU_fixed_16(&raw mut error, red_y);
    gx = png_ITU_fixed_16(&raw mut error, green_x);
    gy = png_ITU_fixed_16(&raw mut error, green_y);
    bx = png_ITU_fixed_16(&raw mut error, blue_x);
    by = png_ITU_fixed_16(&raw mut error, blue_y);
    wx = png_ITU_fixed_16(&raw mut error, white_x);
    wy = png_ITU_fixed_16(&raw mut error, white_y);
    if error != 0 {
        png_chunk_report(
            png_ptr,
            b"mDCV chromaticities outside representable range\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    if maxDL > 0x7fffffff as ::core::ffi::c_uint || minDL > 0x7fffffff as ::core::ffi::c_uint {
        png_chunk_report(
            png_ptr,
            b"mDCV display light level exceeds PNG limit\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    (*info_ptr).mastering_red_x = rx;
    (*info_ptr).mastering_red_y = ry;
    (*info_ptr).mastering_green_x = gx;
    (*info_ptr).mastering_green_y = gy;
    (*info_ptr).mastering_blue_x = bx;
    (*info_ptr).mastering_blue_y = by;
    (*info_ptr).mastering_white_x = wx;
    (*info_ptr).mastering_white_y = wy;
    (*info_ptr).mastering_maxDL = maxDL;
    (*info_ptr).mastering_minDL = minDL;
    (*info_ptr).valid |= PNG_INFO_mDCV;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_mDCV(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut white_x: ::core::ffi::c_double,
    mut white_y: ::core::ffi::c_double,
    mut red_x: ::core::ffi::c_double,
    mut red_y: ::core::ffi::c_double,
    mut green_x: ::core::ffi::c_double,
    mut green_y: ::core::ffi::c_double,
    mut blue_x: ::core::ffi::c_double,
    mut blue_y: ::core::ffi::c_double,
    mut maxDL: ::core::ffi::c_double,
    mut minDL: ::core::ffi::c_double,
) {
    png_set_mDCV_fixed(
        png_ptr,
        info_ptr,
        png_fixed(
            png_ptr,
            white_x,
            b"png_set_mDCV(white(x))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            white_y,
            b"png_set_mDCV(white(y))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_x,
            b"png_set_mDCV(red(x))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            red_y,
            b"png_set_mDCV(red(y))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_x,
            b"png_set_mDCV(green(x))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            green_y,
            b"png_set_mDCV(green(y))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_x,
            b"png_set_mDCV(blue(x))\0" as *const u8 as png_const_charp,
        ),
        png_fixed(
            png_ptr,
            blue_y,
            b"png_set_mDCV(blue(y))\0" as *const u8 as png_const_charp,
        ),
        png_fixed_ITU(
            png_ptr,
            maxDL,
            b"png_set_mDCV(maxDL)\0" as *const u8 as png_const_charp,
        ),
        png_fixed_ITU(
            png_ptr,
            minDL,
            b"png_set_mDCV(minDL)\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut exif: png_bytep,
) {
    png_warning(
        png_ptr,
        b"png_set_eXIf does not work; use png_set_eXIf_1\0" as *const u8 as png_const_charp,
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_eXIf_1(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut num_exif: png_uint_32,
    mut exif: png_bytep,
) {
    let mut new_exif: png_bytep = ::core::ptr::null_mut::<png_byte>();
    if png_ptr.is_null()
        || info_ptr.is_null()
        || (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_eXIf != 0 as ::core::ffi::c_uint
        || exif.is_null()
    {
        return;
    }
    new_exif = png_malloc_warn(png_ptr, num_exif as png_alloc_size_t) as png_bytep;
    if new_exif.is_null() {
        png_warning(
            png_ptr,
            b"Insufficient memory for eXIf chunk data\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        new_exif as *mut ::core::ffi::c_void,
        exif as *const ::core::ffi::c_void,
        num_exif as size_t,
    );
    png_free_data(png_ptr, info_ptr, PNG_FREE_EXIF, 0 as ::core::ffi::c_int);
    (*info_ptr).num_exif = num_exif;
    (*info_ptr).exif = new_exif;
    (*info_ptr).free_me |= PNG_FREE_EXIF;
    (*info_ptr).valid |= PNG_INFO_eXIf;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut file_gamma: png_fixed_point,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).gamma = file_gamma;
    (*info_ptr).valid |= PNG_INFO_gAMA;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_gAMA(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut file_gamma: ::core::ffi::c_double,
) {
    png_set_gAMA_fixed(
        png_ptr,
        info_ptr,
        png_fixed(
            png_ptr,
            file_gamma,
            b"png_set_gAMA\0" as *const u8 as png_const_charp,
        ),
    );
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_hIST(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut hist: png_const_uint_16p,
) {
    let mut safe_hist: [png_uint_16; 256] = [0; 256];
    let mut i: ::core::ffi::c_int = 0;
    if png_ptr.is_null() || info_ptr.is_null() || hist.is_null() {
        return;
    }
    if (*info_ptr).num_palette as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*info_ptr).num_palette as ::core::ffi::c_int > PNG_MAX_PALETTE_LENGTH
    {
        png_warning(
            png_ptr,
            b"Invalid palette size, hIST allocation skipped\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        &raw mut safe_hist as *mut png_uint_16 as *mut ::core::ffi::c_void,
        hist as *const ::core::ffi::c_void,
        ((*info_ptr).num_palette as ::core::ffi::c_uint as size_t)
            .wrapping_mul(::core::mem::size_of::<png_uint_16>() as size_t),
    );
    hist = &raw mut safe_hist as *mut png_uint_16 as png_const_uint_16p;
    png_free_data(png_ptr, info_ptr, PNG_FREE_HIST, 0 as ::core::ffi::c_int);
    (*info_ptr).hist = png_malloc_warn(
        png_ptr,
        (256 as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<png_uint_16>() as png_alloc_size_t),
    ) as png_uint_16p;
    if (*info_ptr).hist.is_null() {
        png_warning(
            png_ptr,
            b"Insufficient memory for hIST chunk data\0" as *const u8 as png_const_charp,
        );
        return;
    }
    i = 0 as ::core::ffi::c_int;
    while i < (*info_ptr).num_palette as ::core::ffi::c_int {
        *(*info_ptr).hist.offset(i as isize) = *hist.offset(i as isize);
        i += 1;
    }
    (*info_ptr).free_me |= PNG_FREE_HIST;
    (*info_ptr).valid |= PNG_INFO_hIST;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_IHDR(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut width: png_uint_32,
    mut height: png_uint_32,
    mut bit_depth: ::core::ffi::c_int,
    mut color_type: ::core::ffi::c_int,
    mut interlace_type: ::core::ffi::c_int,
    mut compression_type: ::core::ffi::c_int,
    mut filter_type: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).width = width;
    (*info_ptr).height = height;
    (*info_ptr).bit_depth = bit_depth as png_byte;
    (*info_ptr).color_type = color_type as png_byte;
    (*info_ptr).compression_type = compression_type as png_byte;
    (*info_ptr).filter_type = filter_type as png_byte;
    (*info_ptr).interlace_type = interlace_type as png_byte;
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
    if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
        (*info_ptr).channels = 1 as png_byte;
    } else if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_COLOR
        != 0 as ::core::ffi::c_int
    {
        (*info_ptr).channels = 3 as png_byte;
    } else {
        (*info_ptr).channels = 1 as png_byte;
    }
    if (*info_ptr).color_type as ::core::ffi::c_int & PNG_COLOR_MASK_ALPHA
        != 0 as ::core::ffi::c_int
    {
        (*info_ptr).channels = (*info_ptr).channels.wrapping_add(1);
    }
    (*info_ptr).pixel_depth = ((*info_ptr).channels as ::core::ffi::c_int
        * (*info_ptr).bit_depth as ::core::ffi::c_int) as png_byte;
    (*info_ptr).rowbytes = if (*info_ptr).pixel_depth as ::core::ffi::c_int
        >= 8 as ::core::ffi::c_int
    {
        (width as size_t).wrapping_mul((*info_ptr).pixel_depth as size_t >> 3 as ::core::ffi::c_int)
    } else {
        (width as size_t)
            .wrapping_mul((*info_ptr).pixel_depth as size_t)
            .wrapping_add(7 as size_t)
            >> 3 as ::core::ffi::c_int
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_oFFs(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut offset_x: png_int_32,
    mut offset_y: png_int_32,
    mut unit_type: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).x_offset = offset_x;
    (*info_ptr).y_offset = offset_y;
    (*info_ptr).offset_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_oFFs;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_pCAL(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut purpose: png_const_charp,
    mut X0: png_int_32,
    mut X1: png_int_32,
    mut type_0: ::core::ffi::c_int,
    mut nparams: ::core::ffi::c_int,
    mut units: png_const_charp,
    mut params: png_charpp,
) {
    let mut length: size_t = 0;
    let mut i: ::core::ffi::c_int = 0;
    if png_ptr.is_null()
        || info_ptr.is_null()
        || purpose.is_null()
        || units.is_null()
        || nparams > 0 as ::core::ffi::c_int && params.is_null()
    {
        return;
    }
    length = strlen(purpose as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
    if type_0 < 0 as ::core::ffi::c_int || type_0 > 3 as ::core::ffi::c_int {
        png_chunk_report(
            png_ptr,
            b"Invalid pCAL equation type\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    if nparams < 0 as ::core::ffi::c_int || nparams > 255 as ::core::ffi::c_int {
        png_chunk_report(
            png_ptr,
            b"Invalid pCAL parameter count\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    i = 0 as ::core::ffi::c_int;
    while i < nparams {
        if (*params.offset(i as isize)).is_null()
            || png_check_fp_string(
                *params.offset(i as isize) as png_const_charp,
                strlen(*params.offset(i as isize)),
            ) == 0
        {
            png_chunk_report(
                png_ptr,
                b"Invalid format for pCAL parameter\0" as *const u8 as png_const_charp,
                PNG_CHUNK_WRITE_ERROR,
            );
            return;
        }
        i += 1;
    }
    (*info_ptr).pcal_purpose = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;
    if (*info_ptr).pcal_purpose.is_null() {
        png_chunk_report(
            png_ptr,
            b"Insufficient memory for pCAL purpose\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    memcpy(
        (*info_ptr).pcal_purpose as *mut ::core::ffi::c_void,
        purpose as *const ::core::ffi::c_void,
        length,
    );
    (*info_ptr).free_me |= PNG_FREE_PCAL;
    (*info_ptr).pcal_X0 = X0;
    (*info_ptr).pcal_X1 = X1;
    (*info_ptr).pcal_type = type_0 as png_byte;
    (*info_ptr).pcal_nparams = nparams as png_byte;
    length = strlen(units as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
    (*info_ptr).pcal_units = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;
    if (*info_ptr).pcal_units.is_null() {
        png_warning(
            png_ptr,
            b"Insufficient memory for pCAL units\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        (*info_ptr).pcal_units as *mut ::core::ffi::c_void,
        units as *const ::core::ffi::c_void,
        length,
    );
    (*info_ptr).pcal_params = png_malloc_warn(
        png_ptr,
        ((nparams as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint) as usize)
            .wrapping_mul(::core::mem::size_of::<png_charp>() as usize),
    ) as png_charpp;
    if (*info_ptr).pcal_params.is_null() {
        png_warning(
            png_ptr,
            b"Insufficient memory for pCAL params\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memset(
        (*info_ptr).pcal_params as *mut ::core::ffi::c_void,
        0 as ::core::ffi::c_int,
        ((nparams as ::core::ffi::c_uint).wrapping_add(1 as ::core::ffi::c_uint) as size_t)
            .wrapping_mul(::core::mem::size_of::<png_charp>() as size_t),
    );
    i = 0 as ::core::ffi::c_int;
    while i < nparams {
        length = strlen(*params.offset(i as isize)).wrapping_add(1 as size_t);
        let ref mut fresh0 = *(*info_ptr).pcal_params.offset(i as isize);
        *fresh0 = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp
            as *mut ::core::ffi::c_char;
        if (*(*info_ptr).pcal_params.offset(i as isize)).is_null() {
            png_warning(
                png_ptr,
                b"Insufficient memory for pCAL parameter\0" as *const u8 as png_const_charp,
            );
            return;
        }
        memcpy(
            *(*info_ptr).pcal_params.offset(i as isize) as *mut ::core::ffi::c_void,
            *params.offset(i as isize) as *const ::core::ffi::c_void,
            length,
        );
        i += 1;
    }
    (*info_ptr).valid |= PNG_INFO_pCAL;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL_s(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut unit: ::core::ffi::c_int,
    mut swidth: png_const_charp,
    mut sheight: png_const_charp,
) {
    let mut lengthw: size_t = 0 as size_t;
    let mut lengthh: size_t = 0 as size_t;
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if unit != 1 as ::core::ffi::c_int && unit != 2 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"Invalid sCAL unit\0" as *const u8 as png_const_charp,
        );
    }
    if swidth.is_null()
        || {
            lengthw = strlen(swidth as *const ::core::ffi::c_char);
            lengthw == 0 as size_t
        }
        || *swidth.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 45 as ::core::ffi::c_int
        || png_check_fp_string(swidth, lengthw) == 0
    {
        png_error(
            png_ptr,
            b"Invalid sCAL width\0" as *const u8 as png_const_charp,
        );
    }
    if sheight.is_null()
        || {
            lengthh = strlen(sheight as *const ::core::ffi::c_char);
            lengthh == 0 as size_t
        }
        || *sheight.offset(0 as ::core::ffi::c_int as isize) as ::core::ffi::c_int
            == 45 as ::core::ffi::c_int
        || png_check_fp_string(sheight, lengthh) == 0
    {
        png_error(
            png_ptr,
            b"Invalid sCAL height\0" as *const u8 as png_const_charp,
        );
    }
    (*info_ptr).scal_unit = unit as png_byte;
    lengthw = lengthw.wrapping_add(1);
    (*info_ptr).scal_s_width = png_malloc_warn(png_ptr, lengthw as png_alloc_size_t) as png_charp;
    if (*info_ptr).scal_s_width.is_null() {
        png_warning(
            png_ptr,
            b"Memory allocation failed while processing sCAL\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        (*info_ptr).scal_s_width as *mut ::core::ffi::c_void,
        swidth as *const ::core::ffi::c_void,
        lengthw,
    );
    lengthh = lengthh.wrapping_add(1);
    (*info_ptr).scal_s_height = png_malloc_warn(png_ptr, lengthh as png_alloc_size_t) as png_charp;
    if (*info_ptr).scal_s_height.is_null() {
        png_free(png_ptr, (*info_ptr).scal_s_width as png_voidp);
        (*info_ptr).scal_s_width = ::core::ptr::null_mut::<::core::ffi::c_char>();
        png_warning(
            png_ptr,
            b"Memory allocation failed while processing sCAL\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        (*info_ptr).scal_s_height as *mut ::core::ffi::c_void,
        sheight as *const ::core::ffi::c_void,
        lengthh,
    );
    (*info_ptr).free_me |= PNG_FREE_SCAL;
    (*info_ptr).valid |= PNG_INFO_sCAL;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut unit: ::core::ffi::c_int,
    mut width: ::core::ffi::c_double,
    mut height: ::core::ffi::c_double,
) {
    if width <= 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        png_warning(
            png_ptr,
            b"Invalid sCAL width ignored\0" as *const u8 as png_const_charp,
        );
    } else if height <= 0 as ::core::ffi::c_int as ::core::ffi::c_double {
        png_warning(
            png_ptr,
            b"Invalid sCAL height ignored\0" as *const u8 as png_const_charp,
        );
    } else {
        let mut swidth: [::core::ffi::c_char; 18] = [0; 18];
        let mut sheight: [::core::ffi::c_char; 18] = [0; 18];
        png_ascii_from_fp(
            png_ptr,
            &raw mut swidth as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 18]>() as size_t,
            width,
            PNG_sCAL_PRECISION as ::core::ffi::c_uint,
        );
        png_ascii_from_fp(
            png_ptr,
            &raw mut sheight as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 18]>() as size_t,
            height,
            PNG_sCAL_PRECISION as ::core::ffi::c_uint,
        );
        png_set_sCAL_s(
            png_ptr,
            info_ptr,
            unit,
            &raw mut swidth as *mut ::core::ffi::c_char as png_const_charp,
            &raw mut sheight as *mut ::core::ffi::c_char as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sCAL_fixed(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut unit: ::core::ffi::c_int,
    mut width: png_fixed_point,
    mut height: png_fixed_point,
) {
    if width <= 0 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Invalid sCAL width ignored\0" as *const u8 as png_const_charp,
        );
    } else if height <= 0 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"Invalid sCAL height ignored\0" as *const u8 as png_const_charp,
        );
    } else {
        let mut swidth: [::core::ffi::c_char; 18] = [0; 18];
        let mut sheight: [::core::ffi::c_char; 18] = [0; 18];
        png_ascii_from_fixed(
            png_ptr,
            &raw mut swidth as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 18]>() as size_t,
            width,
        );
        png_ascii_from_fixed(
            png_ptr,
            &raw mut sheight as png_charp,
            ::core::mem::size_of::<[::core::ffi::c_char; 18]>() as size_t,
            height,
        );
        png_set_sCAL_s(
            png_ptr,
            info_ptr,
            unit,
            &raw mut swidth as *mut ::core::ffi::c_char as png_const_charp,
            &raw mut sheight as *mut ::core::ffi::c_char as png_const_charp,
        );
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_pHYs(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut res_x: png_uint_32,
    mut res_y: png_uint_32,
    mut unit_type: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).x_pixels_per_unit = res_x;
    (*info_ptr).y_pixels_per_unit = res_y;
    (*info_ptr).phys_unit_type = unit_type as png_byte;
    (*info_ptr).valid |= PNG_INFO_pHYs;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_PLTE(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
    mut palette: png_const_colorp,
    mut num_palette: ::core::ffi::c_int,
) {
    let mut safe_palette: [png_color; 256] = [png_color {
        red: 0,
        green: 0,
        blue: 0,
    }; 256];
    let mut max_palette_length: png_uint_32 = 0;
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    max_palette_length = (if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE
    {
        (1 as ::core::ffi::c_int) << (*info_ptr).bit_depth as ::core::ffi::c_int
    } else {
        PNG_MAX_PALETTE_LENGTH
    }) as png_uint_32;
    if num_palette < 0 as ::core::ffi::c_int
        || num_palette > max_palette_length as ::core::ffi::c_int
    {
        if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_PALETTE {
            png_error(
                png_ptr,
                b"Invalid palette length\0" as *const u8 as png_const_charp,
            );
        } else {
            png_warning(
                png_ptr,
                b"Invalid palette length\0" as *const u8 as png_const_charp,
            );
            return;
        }
    }
    if num_palette > 0 as ::core::ffi::c_int && palette.is_null()
        || num_palette == 0 as ::core::ffi::c_int
            && (*png_ptr).mng_features_permitted as ::core::ffi::c_uint
                & PNG_FLAG_MNG_EMPTY_PLTE as ::core::ffi::c_uint
                == 0 as ::core::ffi::c_uint
    {
        png_error(
            png_ptr,
            b"Invalid palette\0" as *const u8 as png_const_charp,
        );
    }
    if num_palette > 0 as ::core::ffi::c_int {
        memcpy(
            &raw mut safe_palette as *mut png_color as *mut ::core::ffi::c_void,
            palette as *const ::core::ffi::c_void,
            (num_palette as ::core::ffi::c_uint as size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as size_t),
        );
    }
    palette = &raw mut safe_palette as *mut png_color as png_const_colorp;
    png_free_data(png_ptr, info_ptr, PNG_FREE_PLTE, 0 as ::core::ffi::c_int);
    png_free(png_ptr, (*png_ptr).palette as png_voidp);
    (*png_ptr).palette = ::core::ptr::null_mut::<png_color>();
    (*png_ptr).palette = png_calloc(
        png_ptr,
        (256 as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<png_color>() as png_alloc_size_t),
    ) as png_colorp;
    (*info_ptr).palette = png_calloc(
        png_ptr,
        (256 as png_alloc_size_t)
            .wrapping_mul(::core::mem::size_of::<png_color>() as png_alloc_size_t),
    ) as png_colorp;
    (*info_ptr).num_palette = num_palette as png_uint_16;
    (*png_ptr).num_palette = (*info_ptr).num_palette;
    if num_palette > 0 as ::core::ffi::c_int {
        memcpy(
            (*info_ptr).palette as *mut ::core::ffi::c_void,
            palette as *const ::core::ffi::c_void,
            (num_palette as ::core::ffi::c_uint as size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as size_t),
        );
        memcpy(
            (*png_ptr).palette as *mut ::core::ffi::c_void,
            palette as *const ::core::ffi::c_void,
            (num_palette as ::core::ffi::c_uint as size_t)
                .wrapping_mul(::core::mem::size_of::<png_color>() as size_t),
        );
    }
    (*info_ptr).free_me |= PNG_FREE_PLTE;
    (*info_ptr).valid |= PNG_INFO_PLTE;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sBIT(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut sig_bit: png_const_color_8p,
) {
    if png_ptr.is_null() || info_ptr.is_null() || sig_bit.is_null() {
        return;
    }
    (*info_ptr).sig_bit = *sig_bit;
    (*info_ptr).valid |= PNG_INFO_sBIT;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sRGB(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut srgb_intent: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    (*info_ptr).rendering_intent = srgb_intent;
    (*info_ptr).valid |= PNG_INFO_sRGB;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sRGB_gAMA_and_cHRM(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut srgb_intent: ::core::ffi::c_int,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    png_set_sRGB(png_ptr, info_ptr, srgb_intent);
    png_set_gAMA_fixed(png_ptr, info_ptr, PNG_GAMMA_sRGB_INVERSE);
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
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_iCCP(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut name: png_const_charp,
    mut compression_type: ::core::ffi::c_int,
    mut profile: png_const_bytep,
    mut proflen: png_uint_32,
) {
    let mut new_iccp_name: png_charp = ::core::ptr::null_mut::<::core::ffi::c_char>();
    let mut new_iccp_profile: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut length: size_t = 0;
    if png_ptr.is_null() || info_ptr.is_null() || name.is_null() || profile.is_null() {
        return;
    }
    if compression_type != PNG_COMPRESSION_TYPE_BASE {
        png_app_error(
            png_ptr,
            b"Invalid iCCP compression method\0" as *const u8 as png_const_charp,
        );
    }
    length = strlen(name as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
    new_iccp_name = png_malloc_warn(png_ptr, length as png_alloc_size_t) as png_charp;
    if new_iccp_name.is_null() {
        png_benign_error(
            png_ptr,
            b"Insufficient memory to process iCCP chunk\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        new_iccp_name as *mut ::core::ffi::c_void,
        name as *const ::core::ffi::c_void,
        length,
    );
    new_iccp_profile = png_malloc_warn(png_ptr, proflen as png_alloc_size_t) as png_bytep;
    if new_iccp_profile.is_null() {
        png_free(png_ptr, new_iccp_name as png_voidp);
        png_benign_error(
            png_ptr,
            b"Insufficient memory to process iCCP profile\0" as *const u8 as png_const_charp,
        );
        return;
    }
    memcpy(
        new_iccp_profile as *mut ::core::ffi::c_void,
        profile as *const ::core::ffi::c_void,
        proflen as size_t,
    );
    png_free_data(png_ptr, info_ptr, PNG_FREE_ICCP, 0 as ::core::ffi::c_int);
    (*info_ptr).iccp_proflen = proflen;
    (*info_ptr).iccp_name = new_iccp_name;
    (*info_ptr).iccp_profile = new_iccp_profile;
    (*info_ptr).free_me |= PNG_FREE_ICCP;
    (*info_ptr).valid |= PNG_INFO_iCCP;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut text_ptr: png_const_textp,
    mut num_text: ::core::ffi::c_int,
) {
    let mut ret: ::core::ffi::c_int = 0;
    ret = png_set_text_2(png_ptr, info_ptr, text_ptr, num_text);
    if ret != 0 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"Insufficient memory to store text\0" as *const u8 as png_const_charp,
        );
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_text_2(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut text_ptr: png_const_textp,
    mut num_text: ::core::ffi::c_int,
) -> ::core::ffi::c_int {
    let mut i: ::core::ffi::c_int = 0;
    let mut old_text: png_textp = ::core::ptr::null_mut::<png_text>();
    if png_ptr.is_null()
        || info_ptr.is_null()
        || num_text <= 0 as ::core::ffi::c_int
        || text_ptr.is_null()
    {
        return 0 as ::core::ffi::c_int;
    }
    if num_text > (*info_ptr).max_text - (*info_ptr).num_text {
        let mut old_num_text: ::core::ffi::c_int = (*info_ptr).num_text;
        let mut max_text: ::core::ffi::c_int = 0;
        let mut new_text: png_textp = ::core::ptr::null_mut::<png_text>();
        max_text = old_num_text;
        if num_text <= INT_MAX - max_text {
            max_text += num_text;
            if max_text < INT_MAX - 8 as ::core::ffi::c_int {
                max_text = max_text + 8 as ::core::ffi::c_int & !(0x7 as ::core::ffi::c_int);
            } else {
                max_text = INT_MAX;
            }
            new_text = png_realloc_array(
                png_ptr,
                (*info_ptr).text as png_const_voidp,
                old_num_text,
                max_text - old_num_text,
                ::core::mem::size_of::<png_text>() as size_t,
            ) as png_textp;
        }
        if new_text.is_null() {
            png_chunk_report(
                png_ptr,
                b"too many text chunks\0" as *const u8 as png_const_charp,
                PNG_CHUNK_WRITE_ERROR,
            );
            return 1 as ::core::ffi::c_int;
        }
        old_text = (*info_ptr).text;
        (*info_ptr).text = new_text;
        (*info_ptr).free_me |= PNG_FREE_TEXT;
        (*info_ptr).max_text = max_text;
    }
    i = 0 as ::core::ffi::c_int;
    while i < num_text {
        let mut text_length: size_t = 0;
        let mut key_len: size_t = 0;
        let mut lang_len: size_t = 0;
        let mut lang_key_len: size_t = 0;
        let mut textp: png_textp =
            (*info_ptr).text.offset((*info_ptr).num_text as isize) as png_textp;
        if !(*text_ptr.offset(i as isize)).key.is_null() {
            if (*text_ptr.offset(i as isize)).compression < PNG_TEXT_COMPRESSION_NONE
                || (*text_ptr.offset(i as isize)).compression >= PNG_TEXT_COMPRESSION_LAST
            {
                png_chunk_report(
                    png_ptr,
                    b"text compression mode is out of range\0" as *const u8 as png_const_charp,
                    PNG_CHUNK_WRITE_ERROR,
                );
            } else {
                key_len = strlen((*text_ptr.offset(i as isize)).key as *const ::core::ffi::c_char);
                if (*text_ptr.offset(i as isize)).compression <= 0 as ::core::ffi::c_int {
                    lang_len = 0 as size_t;
                    lang_key_len = 0 as size_t;
                } else {
                    if !(*text_ptr.offset(i as isize)).lang.is_null() {
                        lang_len = strlen(
                            (*text_ptr.offset(i as isize)).lang as *const ::core::ffi::c_char,
                        );
                    } else {
                        lang_len = 0 as size_t;
                    }
                    if !(*text_ptr.offset(i as isize)).lang_key.is_null() {
                        lang_key_len = strlen(
                            (*text_ptr.offset(i as isize)).lang_key as *const ::core::ffi::c_char,
                        );
                    } else {
                        lang_key_len = 0 as size_t;
                    }
                }
                if (*text_ptr.offset(i as isize)).text.is_null()
                    || *(*text_ptr.offset(i as isize))
                        .text
                        .offset(0 as ::core::ffi::c_int as isize)
                        as ::core::ffi::c_int
                        == '\0' as i32
                {
                    text_length = 0 as size_t;
                    if (*text_ptr.offset(i as isize)).compression > 0 as ::core::ffi::c_int {
                        (*textp).compression = PNG_ITXT_COMPRESSION_NONE;
                    } else {
                        (*textp).compression = PNG_TEXT_COMPRESSION_NONE;
                    }
                } else {
                    text_length =
                        strlen((*text_ptr.offset(i as isize)).text as *const ::core::ffi::c_char);
                    (*textp).compression = (*text_ptr.offset(i as isize)).compression;
                }
                (*textp).key = png_malloc_base(
                    png_ptr,
                    (key_len as png_alloc_size_t)
                        .wrapping_add(text_length as png_alloc_size_t)
                        .wrapping_add(lang_len as png_alloc_size_t)
                        .wrapping_add(lang_key_len as png_alloc_size_t)
                        .wrapping_add(4 as png_alloc_size_t),
                ) as png_charp;
                if (*textp).key.is_null() {
                    png_chunk_report(
                        png_ptr,
                        b"text chunk: out of memory\0" as *const u8 as png_const_charp,
                        PNG_CHUNK_WRITE_ERROR,
                    );
                    png_free(png_ptr, old_text as png_voidp);
                    return 1 as ::core::ffi::c_int;
                }
                memcpy(
                    (*textp).key as *mut ::core::ffi::c_void,
                    (*text_ptr.offset(i as isize)).key as *const ::core::ffi::c_void,
                    key_len,
                );
                *(*textp).key.offset(key_len as isize) = '\0' as i32 as ::core::ffi::c_char;
                if (*text_ptr.offset(i as isize)).compression > 0 as ::core::ffi::c_int {
                    (*textp).lang = (*textp)
                        .key
                        .offset(key_len as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                    memcpy(
                        (*textp).lang as *mut ::core::ffi::c_void,
                        (*text_ptr.offset(i as isize)).lang as *const ::core::ffi::c_void,
                        lang_len,
                    );
                    *(*textp).lang.offset(lang_len as isize) = '\0' as i32 as ::core::ffi::c_char;
                    (*textp).lang_key = (*textp)
                        .lang
                        .offset(lang_len as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                    memcpy(
                        (*textp).lang_key as *mut ::core::ffi::c_void,
                        (*text_ptr.offset(i as isize)).lang_key as *const ::core::ffi::c_void,
                        lang_key_len,
                    );
                    *(*textp).lang_key.offset(lang_key_len as isize) =
                        '\0' as i32 as ::core::ffi::c_char;
                    (*textp).text = (*textp)
                        .lang_key
                        .offset(lang_key_len as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                } else {
                    (*textp).lang = ::core::ptr::null_mut::<::core::ffi::c_char>();
                    (*textp).lang_key = ::core::ptr::null_mut::<::core::ffi::c_char>();
                    (*textp).text = (*textp)
                        .key
                        .offset(key_len as isize)
                        .offset(1 as ::core::ffi::c_int as isize);
                }
                if text_length != 0 as size_t {
                    memcpy(
                        (*textp).text as *mut ::core::ffi::c_void,
                        (*text_ptr.offset(i as isize)).text as *const ::core::ffi::c_void,
                        text_length,
                    );
                }
                *(*textp).text.offset(text_length as isize) = '\0' as i32 as ::core::ffi::c_char;
                if (*textp).compression > 0 as ::core::ffi::c_int {
                    (*textp).text_length = 0 as size_t;
                    (*textp).itxt_length = text_length;
                } else {
                    (*textp).text_length = text_length;
                    (*textp).itxt_length = 0 as size_t;
                }
                (*info_ptr).num_text += 1;
            }
        }
        i += 1;
    }
    png_free(png_ptr, old_text as png_voidp);
    return 0 as ::core::ffi::c_int;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tIME(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut mod_time: png_const_timep,
) {
    if png_ptr.is_null()
        || info_ptr.is_null()
        || mod_time.is_null()
        || (*png_ptr).mode as ::core::ffi::c_uint & PNG_WROTE_tIME != 0 as ::core::ffi::c_uint
    {
        return;
    }
    if (*mod_time).month as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*mod_time).month as ::core::ffi::c_int > 12 as ::core::ffi::c_int
        || (*mod_time).day as ::core::ffi::c_int == 0 as ::core::ffi::c_int
        || (*mod_time).day as ::core::ffi::c_int > 31 as ::core::ffi::c_int
        || (*mod_time).hour as ::core::ffi::c_int > 23 as ::core::ffi::c_int
        || (*mod_time).minute as ::core::ffi::c_int > 59 as ::core::ffi::c_int
        || (*mod_time).second as ::core::ffi::c_int > 60 as ::core::ffi::c_int
    {
        png_warning(
            png_ptr,
            b"Ignoring invalid time value\0" as *const u8 as png_const_charp,
        );
        return;
    }
    (*info_ptr).mod_time = *mod_time;
    (*info_ptr).valid |= PNG_INFO_tIME;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_tRNS(
    mut png_ptr: png_structrp,
    mut info_ptr: png_inforp,
    mut trans_alpha: png_const_bytep,
    mut num_trans: ::core::ffi::c_int,
    mut trans_color: png_const_color_16p,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if !trans_alpha.is_null() {
        let mut safe_trans: [png_byte; 256] = [0; 256];
        if num_trans > 0 as ::core::ffi::c_int && num_trans <= PNG_MAX_PALETTE_LENGTH {
            memcpy(
                &raw mut safe_trans as *mut png_byte as *mut ::core::ffi::c_void,
                trans_alpha as *const ::core::ffi::c_void,
                num_trans as size_t,
            );
        }
        trans_alpha = &raw mut safe_trans as *mut png_byte as png_const_bytep;
        png_free_data(png_ptr, info_ptr, PNG_FREE_TRNS, 0 as ::core::ffi::c_int);
        if num_trans > 0 as ::core::ffi::c_int && num_trans <= PNG_MAX_PALETTE_LENGTH {
            (*info_ptr).trans_alpha = png_malloc(png_ptr, 256 as png_alloc_size_t) as png_bytep;
            memset(
                (*info_ptr).trans_alpha as *mut ::core::ffi::c_void,
                0xff as ::core::ffi::c_int,
                PNG_MAX_PALETTE_LENGTH as size_t,
            );
            memcpy(
                (*info_ptr).trans_alpha as *mut ::core::ffi::c_void,
                trans_alpha as *const ::core::ffi::c_void,
                num_trans as size_t,
            );
            (*info_ptr).free_me |= PNG_FREE_TRNS;
            (*info_ptr).valid |= PNG_INFO_tRNS;
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = ::core::ptr::null_mut::<png_byte>();
            (*png_ptr).trans_alpha = png_malloc(png_ptr, 256 as png_alloc_size_t) as png_bytep;
            memset(
                (*png_ptr).trans_alpha as *mut ::core::ffi::c_void,
                0xff as ::core::ffi::c_int,
                PNG_MAX_PALETTE_LENGTH as size_t,
            );
            memcpy(
                (*png_ptr).trans_alpha as *mut ::core::ffi::c_void,
                trans_alpha as *const ::core::ffi::c_void,
                num_trans as size_t,
            );
        } else {
            png_free(png_ptr, (*png_ptr).trans_alpha as png_voidp);
            (*png_ptr).trans_alpha = ::core::ptr::null_mut::<png_byte>();
        }
    }
    if !trans_color.is_null() {
        if ((*info_ptr).bit_depth as ::core::ffi::c_int) < 16 as ::core::ffi::c_int {
            let mut sample_max: ::core::ffi::c_int = ((1 as ::core::ffi::c_int)
                << (*info_ptr).bit_depth as ::core::ffi::c_int)
                - 1 as ::core::ffi::c_int;
            if (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_GRAY
                && (*trans_color).gray as ::core::ffi::c_int > sample_max
                || (*info_ptr).color_type as ::core::ffi::c_int == PNG_COLOR_TYPE_RGB
                    && ((*trans_color).red as ::core::ffi::c_int > sample_max
                        || (*trans_color).green as ::core::ffi::c_int > sample_max
                        || (*trans_color).blue as ::core::ffi::c_int > sample_max)
            {
                png_warning(
                    png_ptr,
                    b"tRNS chunk has out-of-range samples for bit_depth\0" as *const u8
                        as png_const_charp,
                );
            }
        }
        (*info_ptr).trans_color = *trans_color;
        if num_trans == 0 as ::core::ffi::c_int {
            num_trans = 1 as ::core::ffi::c_int;
        }
    }
    (*info_ptr).num_trans = num_trans as png_uint_16;
    if num_trans != 0 as ::core::ffi::c_int {
        (*info_ptr).free_me |= PNG_FREE_TRNS;
        (*info_ptr).valid |= PNG_INFO_tRNS;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_sPLT(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut entries: png_const_sPLT_tp,
    mut nentries: ::core::ffi::c_int,
) {
    let mut np: png_sPLT_tp = ::core::ptr::null_mut::<png_sPLT_t>();
    let mut old_spalettes: png_sPLT_tp = ::core::ptr::null_mut::<png_sPLT_t>();
    if png_ptr.is_null()
        || info_ptr.is_null()
        || nentries <= 0 as ::core::ffi::c_int
        || entries.is_null()
    {
        return;
    }
    np = png_realloc_array(
        png_ptr,
        (*info_ptr).splt_palettes as png_const_voidp,
        (*info_ptr).splt_palettes_num,
        nentries,
        ::core::mem::size_of::<png_sPLT_t>() as size_t,
    ) as png_sPLT_tp;
    if np.is_null() {
        png_chunk_report(
            png_ptr,
            b"too many sPLT chunks\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    old_spalettes = (*info_ptr).splt_palettes;
    (*info_ptr).splt_palettes = np;
    (*info_ptr).free_me |= PNG_FREE_SPLT;
    np = np.offset((*info_ptr).splt_palettes_num as isize);
    loop {
        let mut length: size_t = 0;
        if (*entries).name.is_null() || (*entries).entries.is_null() {
            png_app_error(
                png_ptr,
                b"png_set_sPLT: invalid sPLT\0" as *const u8 as png_const_charp,
            );
        } else {
            (*np).depth = (*entries).depth;
            length =
                strlen((*entries).name as *const ::core::ffi::c_char).wrapping_add(1 as size_t);
            (*np).name = png_malloc_base(png_ptr, length as png_alloc_size_t) as png_charp;
            if (*np).name.is_null() {
                break;
            }
            memcpy(
                (*np).name as *mut ::core::ffi::c_void,
                (*entries).name as *const ::core::ffi::c_void,
                length,
            );
            (*np).entries = png_malloc_array(
                png_ptr,
                (*entries).nentries as ::core::ffi::c_int,
                ::core::mem::size_of::<png_sPLT_entry>() as size_t,
            ) as png_sPLT_entryp;
            if (*np).entries.is_null() {
                png_free(png_ptr, (*np).name as png_voidp);
                (*np).name = ::core::ptr::null_mut::<::core::ffi::c_char>();
                break;
            } else {
                (*np).nentries = (*entries).nentries;
                memcpy(
                    (*np).entries as *mut ::core::ffi::c_void,
                    (*entries).entries as *const ::core::ffi::c_void,
                    ((*entries).nentries as ::core::ffi::c_uint as size_t)
                        .wrapping_mul(::core::mem::size_of::<png_sPLT_entry>() as size_t),
                );
                (*info_ptr).valid |= PNG_INFO_sPLT;
                (*info_ptr).splt_palettes_num += 1;
                np = np.offset(1);
                entries = entries.offset(1);
            }
        }
        nentries -= 1;
        if !(nentries != 0) {
            break;
        }
    }
    png_free(png_ptr, old_spalettes as png_voidp);
    if nentries > 0 as ::core::ffi::c_int {
        png_chunk_report(
            png_ptr,
            b"sPLT out of memory\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
    }
}
unsafe extern "C" fn check_location(
    mut png_ptr: png_const_structrp,
    mut location: ::core::ffi::c_int,
) -> png_byte {
    location &= PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT;
    if location == 0 as ::core::ffi::c_int
        && (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT == 0 as ::core::ffi::c_uint
    {
        png_app_warning(
            png_ptr,
            b"png_set_unknown_chunks now expects a valid location\0" as *const u8
                as png_const_charp,
        );
        location = ((*png_ptr).mode as ::core::ffi::c_uint
            & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) as ::core::ffi::c_uint)
            as png_byte as ::core::ffi::c_int;
    }
    if location == 0 as ::core::ffi::c_int {
        png_error(
            png_ptr,
            b"invalid location in png_set_unknown_chunks\0" as *const u8 as png_const_charp,
        );
    }
    while location != location & -location {
        location &= !(location & -location);
    }
    return location as png_byte;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunks(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut unknowns: png_const_unknown_chunkp,
    mut num_unknowns: ::core::ffi::c_int,
) {
    let mut np: png_unknown_chunkp = ::core::ptr::null_mut::<png_unknown_chunk>();
    let mut old_unknowns: png_unknown_chunkp = ::core::ptr::null_mut::<png_unknown_chunk>();
    if png_ptr.is_null()
        || info_ptr.is_null()
        || num_unknowns <= 0 as ::core::ffi::c_int
        || unknowns.is_null()
    {
        return;
    }
    np = png_realloc_array(
        png_ptr,
        (*info_ptr).unknown_chunks as png_const_voidp,
        (*info_ptr).unknown_chunks_num,
        num_unknowns,
        ::core::mem::size_of::<png_unknown_chunk>() as size_t,
    ) as png_unknown_chunkp;
    if np.is_null() {
        png_chunk_report(
            png_ptr,
            b"too many unknown chunks\0" as *const u8 as png_const_charp,
            PNG_CHUNK_WRITE_ERROR,
        );
        return;
    }
    old_unknowns = (*info_ptr).unknown_chunks;
    (*info_ptr).unknown_chunks = np;
    (*info_ptr).free_me |= PNG_FREE_UNKN;
    np = np.offset((*info_ptr).unknown_chunks_num as isize);
    let mut current_block_22: u64;
    while num_unknowns > 0 as ::core::ffi::c_int {
        memcpy(
            &raw mut (*np).name as *mut png_byte as *mut ::core::ffi::c_void,
            &raw const (*unknowns).name as *const png_byte as *const ::core::ffi::c_void,
            ::core::mem::size_of::<[png_byte; 5]>() as size_t,
        );
        (*np).name[(::core::mem::size_of::<[png_byte; 5]>() as usize).wrapping_sub(1 as usize)
            as usize] = '\0' as i32 as png_byte;
        (*np).location = check_location(png_ptr, (*unknowns).location as ::core::ffi::c_int);
        if (*unknowns).size == 0 as size_t {
            (*np).data = ::core::ptr::null_mut::<png_byte>();
            (*np).size = 0 as size_t;
            current_block_22 = 6009453772311597924;
        } else {
            (*np).data =
                png_malloc_base(png_ptr, (*unknowns).size as png_alloc_size_t) as *mut png_byte;
            if (*np).data.is_null() {
                png_chunk_report(
                    png_ptr,
                    b"unknown chunk: out of memory\0" as *const u8 as png_const_charp,
                    PNG_CHUNK_WRITE_ERROR,
                );
                current_block_22 = 17216689946888361452;
            } else {
                memcpy(
                    (*np).data as *mut ::core::ffi::c_void,
                    (*unknowns).data as *const ::core::ffi::c_void,
                    (*unknowns).size,
                );
                (*np).size = (*unknowns).size;
                current_block_22 = 6009453772311597924;
            }
        }
        match current_block_22 {
            6009453772311597924 => {
                np = np.offset(1);
                (*info_ptr).unknown_chunks_num += 1;
            }
            _ => {}
        }
        num_unknowns -= 1;
        unknowns = unknowns.offset(1);
    }
    png_free(png_ptr, old_unknowns as png_voidp);
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_unknown_chunk_location(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut chunk: ::core::ffi::c_int,
    mut location: ::core::ffi::c_int,
) {
    if !png_ptr.is_null()
        && !info_ptr.is_null()
        && chunk >= 0 as ::core::ffi::c_int
        && chunk < (*info_ptr).unknown_chunks_num
    {
        if location & (PNG_HAVE_IHDR | PNG_HAVE_PLTE | PNG_AFTER_IDAT) == 0 as ::core::ffi::c_int {
            png_app_error(
                png_ptr,
                b"invalid unknown chunk location\0" as *const u8 as png_const_charp,
            );
            if location as ::core::ffi::c_uint & PNG_HAVE_IDAT != 0 as ::core::ffi::c_uint {
                location = PNG_AFTER_IDAT;
            } else {
                location = PNG_HAVE_IHDR;
            }
        }
        (*(*info_ptr).unknown_chunks.offset(chunk as isize)).location =
            check_location(png_ptr, location);
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_permit_mng_features(
    mut png_ptr: png_structrp,
    mut mng_features: png_uint_32,
) -> png_uint_32 {
    if png_ptr.is_null() {
        return 0 as png_uint_32;
    }
    (*png_ptr).mng_features_permitted = (mng_features as ::core::ffi::c_uint
        & PNG_ALL_MNG_FEATURES as ::core::ffi::c_uint)
        as png_uint_32;
    return (*png_ptr).mng_features_permitted;
}
unsafe extern "C" fn add_one_chunk(
    mut list: png_bytep,
    mut count: ::core::ffi::c_uint,
    mut add: png_const_bytep,
    mut keep: ::core::ffi::c_int,
) -> ::core::ffi::c_uint {
    let mut i: ::core::ffi::c_uint = 0;
    i = 0 as ::core::ffi::c_uint;
    while i < count {
        if memcmp(
            list as *const ::core::ffi::c_void,
            add as *const ::core::ffi::c_void,
            4 as size_t,
        ) == 0 as ::core::ffi::c_int
        {
            *list.offset(4 as ::core::ffi::c_int as isize) = keep as png_byte;
            return count;
        }
        i = i.wrapping_add(1);
        list = list.offset(5 as ::core::ffi::c_int as isize);
    }
    if keep != PNG_HANDLE_CHUNK_AS_DEFAULT {
        count = count.wrapping_add(1);
        memcpy(
            list as *mut ::core::ffi::c_void,
            add as *const ::core::ffi::c_void,
            4 as size_t,
        );
        *list.offset(4 as ::core::ffi::c_int as isize) = keep as png_byte;
    }
    return count;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_keep_unknown_chunks(
    mut png_ptr: png_structrp,
    mut keep: ::core::ffi::c_int,
    mut chunk_list: png_const_bytep,
    mut num_chunks_in: ::core::ffi::c_int,
) {
    let mut new_list: png_bytep = ::core::ptr::null_mut::<png_byte>();
    let mut num_chunks: ::core::ffi::c_uint = 0;
    let mut old_num_chunks: ::core::ffi::c_uint = 0;
    if png_ptr.is_null() {
        return;
    }
    if keep < 0 as ::core::ffi::c_int || keep >= PNG_HANDLE_CHUNK_LAST {
        png_app_error(
            png_ptr,
            b"png_set_keep_unknown_chunks: invalid keep\0" as *const u8 as png_const_charp,
        );
        return;
    }
    if num_chunks_in <= 0 as ::core::ffi::c_int {
        (*png_ptr).unknown_default = keep;
        if num_chunks_in == 0 as ::core::ffi::c_int {
            return;
        }
    }
    if num_chunks_in < 0 as ::core::ffi::c_int {
        static mut chunks_to_ignore: [png_byte; 105] = [
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
            99 as ::core::ffi::c_int as png_byte,
            76 as ::core::ffi::c_int as png_byte,
            76 as ::core::ffi::c_int as png_byte,
            73 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            101 as ::core::ffi::c_int as png_byte,
            88 as ::core::ffi::c_int as png_byte,
            73 as ::core::ffi::c_int as png_byte,
            102 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            103 as ::core::ffi::c_int as png_byte,
            65 as ::core::ffi::c_int as png_byte,
            77 as ::core::ffi::c_int as png_byte,
            65 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            104 as ::core::ffi::c_int as png_byte,
            73 as ::core::ffi::c_int as png_byte,
            83 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            105 as ::core::ffi::c_int as png_byte,
            67 as ::core::ffi::c_int as png_byte,
            67 as ::core::ffi::c_int as png_byte,
            80 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            105 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            88 as ::core::ffi::c_int as png_byte,
            116 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            109 as ::core::ffi::c_int as png_byte,
            68 as ::core::ffi::c_int as png_byte,
            67 as ::core::ffi::c_int as png_byte,
            86 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            111 as ::core::ffi::c_int as png_byte,
            70 as ::core::ffi::c_int as png_byte,
            70 as ::core::ffi::c_int as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            112 as ::core::ffi::c_int as png_byte,
            67 as ::core::ffi::c_int as png_byte,
            65 as ::core::ffi::c_int as png_byte,
            76 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            112 as ::core::ffi::c_int as png_byte,
            72 as ::core::ffi::c_int as png_byte,
            89 as ::core::ffi::c_int as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            66 as ::core::ffi::c_int as png_byte,
            73 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            67 as ::core::ffi::c_int as png_byte,
            65 as ::core::ffi::c_int as png_byte,
            76 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            80 as ::core::ffi::c_int as png_byte,
            76 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            69 as ::core::ffi::c_int as png_byte,
            82 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            115 as ::core::ffi::c_int as png_byte,
            82 as ::core::ffi::c_int as png_byte,
            71 as ::core::ffi::c_int as png_byte,
            66 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            116 as ::core::ffi::c_int as png_byte,
            69 as ::core::ffi::c_int as png_byte,
            88 as ::core::ffi::c_int as png_byte,
            116 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            116 as ::core::ffi::c_int as png_byte,
            73 as ::core::ffi::c_int as png_byte,
            77 as ::core::ffi::c_int as png_byte,
            69 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
            122 as ::core::ffi::c_int as png_byte,
            84 as ::core::ffi::c_int as png_byte,
            88 as ::core::ffi::c_int as png_byte,
            116 as ::core::ffi::c_int as png_byte,
            '\0' as i32 as png_byte,
        ];
        chunk_list = &raw const chunks_to_ignore as *const png_byte as png_const_bytep;
        num_chunks = (::core::mem::size_of::<[png_byte; 105]>() as ::core::ffi::c_uint)
            .wrapping_div(5 as ::core::ffi::c_uint);
    } else {
        if chunk_list.is_null() {
            png_app_error(
                png_ptr,
                b"png_set_keep_unknown_chunks: no chunk list\0" as *const u8 as png_const_charp,
            );
            return;
        }
        num_chunks = num_chunks_in as ::core::ffi::c_uint;
    }
    old_num_chunks = (*png_ptr).num_chunk_list;
    if (*png_ptr).chunk_list.is_null() {
        old_num_chunks = 0 as ::core::ffi::c_uint;
    }
    if num_chunks.wrapping_add(old_num_chunks) > UINT_MAX.wrapping_div(5 as ::core::ffi::c_uint) {
        png_app_error(
            png_ptr,
            b"png_set_keep_unknown_chunks: too many chunks\0" as *const u8 as png_const_charp,
        );
        return;
    }
    if keep != 0 as ::core::ffi::c_int {
        new_list = png_malloc(
            png_ptr,
            (5 as ::core::ffi::c_uint).wrapping_mul(num_chunks.wrapping_add(old_num_chunks))
                as png_alloc_size_t,
        ) as png_bytep;
        if old_num_chunks > 0 as ::core::ffi::c_uint {
            memcpy(
                new_list as *mut ::core::ffi::c_void,
                (*png_ptr).chunk_list as *const ::core::ffi::c_void,
                (5 as ::core::ffi::c_uint).wrapping_mul(old_num_chunks) as size_t,
            );
        }
    } else if old_num_chunks > 0 as ::core::ffi::c_uint {
        new_list = (*png_ptr).chunk_list;
    } else {
        new_list = ::core::ptr::null_mut::<png_byte>();
    }
    if !new_list.is_null() {
        let mut inlist: png_const_bytep = ::core::ptr::null::<png_byte>();
        let mut outlist: png_bytep = ::core::ptr::null_mut::<png_byte>();
        let mut i: ::core::ffi::c_uint = 0;
        i = 0 as ::core::ffi::c_uint;
        while i < num_chunks {
            old_num_chunks = add_one_chunk(
                new_list,
                old_num_chunks,
                chunk_list.offset((5 as ::core::ffi::c_uint).wrapping_mul(i) as isize),
                keep,
            );
            i = i.wrapping_add(1);
        }
        num_chunks = 0 as ::core::ffi::c_uint;
        i = 0 as ::core::ffi::c_uint;
        outlist = new_list;
        inlist = outlist as png_const_bytep;
        while i < old_num_chunks {
            if *inlist.offset(4 as ::core::ffi::c_int as isize) != 0 {
                if outlist != inlist as png_bytep {
                    memcpy(
                        outlist as *mut ::core::ffi::c_void,
                        inlist as *const ::core::ffi::c_void,
                        5 as size_t,
                    );
                }
                outlist = outlist.offset(5 as ::core::ffi::c_int as isize);
                num_chunks = num_chunks.wrapping_add(1);
            }
            i = i.wrapping_add(1);
            inlist = inlist.offset(5 as ::core::ffi::c_int as isize);
        }
        if num_chunks == 0 as ::core::ffi::c_uint {
            if (*png_ptr).chunk_list != new_list {
                png_free(png_ptr, new_list as png_voidp);
            }
            new_list = ::core::ptr::null_mut::<png_byte>();
        }
    } else {
        num_chunks = 0 as ::core::ffi::c_uint;
    }
    (*png_ptr).num_chunk_list = num_chunks;
    if (*png_ptr).chunk_list != new_list {
        if !(*png_ptr).chunk_list.is_null() {
            png_free(png_ptr, (*png_ptr).chunk_list as png_voidp);
        }
        (*png_ptr).chunk_list = new_list;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_read_user_chunk_fn(
    mut png_ptr: png_structrp,
    mut user_chunk_ptr: png_voidp,
    mut read_user_chunk_fn: png_user_chunk_ptr,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).read_user_chunk_fn = read_user_chunk_fn;
    (*png_ptr).user_chunk_ptr = user_chunk_ptr;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_rows(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut row_pointers: png_bytepp,
) {
    if png_ptr.is_null() || info_ptr.is_null() {
        return;
    }
    if !(*info_ptr).row_pointers.is_null() && (*info_ptr).row_pointers != row_pointers {
        png_free_data(png_ptr, info_ptr, PNG_FREE_ROWS, 0 as ::core::ffi::c_int);
    }
    (*info_ptr).row_pointers = row_pointers;
    if !row_pointers.is_null() {
        (*info_ptr).valid |= PNG_INFO_IDAT;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_compression_buffer_size(
    mut png_ptr: png_structrp,
    mut size: size_t,
) {
    if png_ptr.is_null() {
        return;
    }
    if size == 0 as size_t || size > PNG_UINT_31_MAX as size_t {
        png_error(
            png_ptr,
            b"invalid compression buffer size\0" as *const u8 as png_const_charp,
        );
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT != 0 as ::core::ffi::c_uint {
        (*png_ptr).IDAT_read_size = size as png_uint_32 as uInt;
        return;
    }
    if (*png_ptr).mode as ::core::ffi::c_uint & PNG_IS_READ_STRUCT == 0 as ::core::ffi::c_uint {
        if (*png_ptr).zowner != 0 as ::core::ffi::c_uint {
            png_warning(
                png_ptr,
                b"Compression buffer size cannot be changed because it is in use\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        if size > ZLIB_IO_MAX as size_t {
            png_warning(
                png_ptr,
                b"Compression buffer size limited to system maximum\0" as *const u8
                    as png_const_charp,
            );
            size = ZLIB_IO_MAX as size_t;
        }
        if size < 6 as size_t {
            png_warning(
                png_ptr,
                b"Compression buffer size cannot be reduced below 6\0" as *const u8
                    as png_const_charp,
            );
            return;
        }
        if (*png_ptr).zbuffer_size as size_t != size {
            png_free_buffer_list(png_ptr, &raw mut (*png_ptr).zbuffer_list);
            (*png_ptr).zbuffer_size = size as uInt;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_invalid(
    mut png_ptr: png_const_structrp,
    mut info_ptr: png_inforp,
    mut mask: ::core::ffi::c_int,
) {
    if !png_ptr.is_null() && !info_ptr.is_null() {
        (*info_ptr).valid &= !mask as ::core::ffi::c_uint;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_user_limits(
    mut png_ptr: png_structrp,
    mut user_width_max: png_uint_32,
    mut user_height_max: png_uint_32,
) {
    if png_ptr.is_null() {
        return;
    }
    (*png_ptr).user_width_max = user_width_max;
    (*png_ptr).user_height_max = user_height_max;
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_chunk_cache_max(
    mut png_ptr: png_structrp,
    mut user_chunk_cache_max: png_uint_32,
) {
    if !png_ptr.is_null() {
        (*png_ptr).user_chunk_cache_max = user_chunk_cache_max;
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_chunk_malloc_max(
    mut png_ptr: png_structrp,
    mut user_chunk_malloc_max: png_alloc_size_t,
) {
    if !png_ptr.is_null() {
        if user_chunk_malloc_max == 0 as png_alloc_size_t {
            (*png_ptr).user_chunk_malloc_max = PNG_SIZE_MAX as png_alloc_size_t;
        } else {
            (*png_ptr).user_chunk_malloc_max = user_chunk_malloc_max;
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_benign_errors(
    mut png_ptr: png_structrp,
    mut allowed: ::core::ffi::c_int,
) {
    if allowed != 0 as ::core::ffi::c_int {
        (*png_ptr).flags |=
            PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN;
    } else {
        (*png_ptr).flags &=
            !(PNG_FLAG_BENIGN_ERRORS_WARN | PNG_FLAG_APP_WARNINGS_WARN | PNG_FLAG_APP_ERRORS_WARN);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_set_check_for_invalid_index(
    mut png_ptr: png_structrp,
    mut allowed: ::core::ffi::c_int,
) {
    if allowed > 0 as ::core::ffi::c_int {
        (*png_ptr).num_palette_max = 0 as ::core::ffi::c_int;
    } else {
        (*png_ptr).num_palette_max = -(1 as ::core::ffi::c_int);
    };
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn png_check_keyword(
    mut png_ptr: png_structrp,
    mut key: png_const_charp,
    mut new_key: png_bytep,
) -> png_uint_32 {
    let mut orig_key: png_const_charp = key;
    let mut key_len: png_uint_32 = 0 as png_uint_32;
    let mut bad_character: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
    let mut space: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
    if key.is_null() {
        *new_key = 0 as png_byte;
        return 0 as png_uint_32;
    }
    while *key as ::core::ffi::c_int != 0 && key_len < 79 as ::core::ffi::c_uint {
        let fresh1 = key;
        key = key.offset(1);
        let mut ch: png_byte = *fresh1 as png_byte;
        if ch as ::core::ffi::c_int > 32 as ::core::ffi::c_int
            && ch as ::core::ffi::c_int <= 126 as ::core::ffi::c_int
            || ch as ::core::ffi::c_int >= 161 as ::core::ffi::c_int
        {
            let fresh2 = new_key;
            new_key = new_key.offset(1);
            *fresh2 = ch;
            key_len = key_len.wrapping_add(1);
            space = 0 as ::core::ffi::c_int;
        } else if space == 0 as ::core::ffi::c_int {
            let fresh3 = new_key;
            new_key = new_key.offset(1);
            *fresh3 = 32 as png_byte;
            key_len = key_len.wrapping_add(1);
            space = 1 as ::core::ffi::c_int;
            if ch as ::core::ffi::c_int != 32 as ::core::ffi::c_int {
                bad_character = ch as ::core::ffi::c_int;
            }
        } else if bad_character == 0 as ::core::ffi::c_int {
            bad_character = ch as ::core::ffi::c_int;
        }
    }
    if key_len > 0 as ::core::ffi::c_uint && space != 0 as ::core::ffi::c_int {
        key_len = key_len.wrapping_sub(1);
        new_key = new_key.offset(-1);
        if bad_character == 0 as ::core::ffi::c_int {
            bad_character = 32 as ::core::ffi::c_int;
        }
    }
    *new_key = 0 as png_byte;
    if key_len == 0 as ::core::ffi::c_uint {
        return 0 as png_uint_32;
    }
    if *key as ::core::ffi::c_int != 0 as ::core::ffi::c_int {
        png_warning(
            png_ptr,
            b"keyword truncated\0" as *const u8 as png_const_charp,
        );
    } else if bad_character != 0 as ::core::ffi::c_int {
        let mut p: png_warning_parameters = [[0; 32]; 8];
        png_warning_parameter(
            &raw mut p as *mut [::core::ffi::c_char; 32],
            1 as ::core::ffi::c_int,
            orig_key,
        );
        png_warning_parameter_signed(
            &raw mut p as *mut [::core::ffi::c_char; 32],
            2 as ::core::ffi::c_int,
            PNG_NUMBER_FORMAT_02x,
            bad_character as png_int_32,
        );
        png_formatted_warning(
            png_ptr,
            &raw mut p as *mut [::core::ffi::c_char; 32],
            b"keyword \"@1\": bad character '0x@2'\0" as *const u8 as png_const_charp,
        );
    }
    return key_len;
}
pub const PNG_HAVE_IDAT: ::core::ffi::c_uint = 0x4 as ::core::ffi::c_uint;
pub const PNG_WROTE_tIME: ::core::ffi::c_uint = 0x200 as ::core::ffi::c_uint;
pub const PNG_WROTE_eXIf: ::core::ffi::c_uint = 0x4000 as ::core::ffi::c_uint;
pub const PNG_IS_READ_STRUCT: ::core::ffi::c_uint = 0x8000 as ::core::ffi::c_uint;
pub const PNG_FLAG_BENIGN_ERRORS_WARN: ::core::ffi::c_uint = 0x100000 as ::core::ffi::c_uint;
pub const PNG_FLAG_APP_WARNINGS_WARN: ::core::ffi::c_uint = 0x200000 as ::core::ffi::c_uint;
pub const PNG_FLAG_APP_ERRORS_WARN: ::core::ffi::c_uint = 0x400000 as ::core::ffi::c_uint;
pub const PNG_GAMMA_sRGB_INVERSE: ::core::ffi::c_int = 45455 as ::core::ffi::c_int;
pub const PNG_NUMBER_FORMAT_02x: ::core::ffi::c_int = 4 as ::core::ffi::c_int;
pub const PNG_CHUNK_WRITE_ERROR: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const __INT_MAX__: ::core::ffi::c_int = 2147483647 as ::core::ffi::c_int;
