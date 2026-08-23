//! Structures and callback types: png.h "section 3", pngstruct.h, pnginfo.h and
//! the private png_control structure from pngpriv.h.
//!
//! Every structure has the exact layout of its C counterpart.

use crate::ctypes::*;

/* -------------------------------------------------------- callback types */

pub type png_error_ptr = Option<unsafe extern "C" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(png_structp, png_bytep, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(png_structp)>;
pub type png_read_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr = Option<unsafe extern "C" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(png_structp, png_row_infop, png_bytep)>;
pub type png_user_chunk_ptr = Option<unsafe extern "C" fn(png_structp, png_unknown_chunkp) -> c_int>;
pub type png_longjmp_ptr = Option<unsafe extern "C" fn(*mut __jmp_buf_tag, c_int)>;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C" fn(png_structp, png_voidp)>;

/* ------------------------------------------------------- basic structures */

pub type png_struct = png_struct_def;
pub type png_const_structp = *const png_struct;
pub type png_structp = *mut png_struct;
pub type png_structpp = *mut *mut png_struct;
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;

pub type png_info = png_info_def;
pub type png_infop = *mut png_info;
pub type png_const_infop = *const png_info;
pub type png_infopp = *mut *mut png_info;
pub type png_inforp = *mut png_info;
pub type png_const_inforp = *const png_info;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;
pub type png_colorpp = *mut *mut png_color;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}
pub type png_color_16p = *mut png_color_16;
pub type png_const_color_16p = *const png_color_16;
pub type png_color_16pp = *mut *mut png_color_16;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}
pub type png_color_8p = *mut png_color_8;
pub type png_const_color_8p = *const png_color_8;
pub type png_color_8pp = *mut *mut png_color_8;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_sPLT_entry {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}
pub type png_sPLT_entryp = *mut png_sPLT_entry;
pub type png_const_sPLT_entryp = *const png_sPLT_entry;
pub type png_sPLT_entrypp = *mut *mut png_sPLT_entry;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_sPLT_t {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: png_sPLT_entryp,
    pub nentries: png_int_32,
}
pub type png_sPLT_tp = *mut png_sPLT_t;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_sPLT_tpp = *mut *mut png_sPLT_t;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_text {
    pub compression: c_int,
    pub key: png_charp,
    pub text: png_charp,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: png_charp,
    pub lang_key: png_charp,
}
pub type png_textp = *mut png_text;
pub type png_const_textp = *const png_text;
pub type png_textpp = *mut *mut png_text;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_time {
    pub year: png_uint_16,
    pub month: png_byte,
    pub day: png_byte,
    pub hour: png_byte,
    pub minute: png_byte,
    pub second: png_byte,
}
pub type png_timep = *mut png_time;
pub type png_const_timep = *const png_time;
pub type png_timepp = *mut *mut png_time;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_unknown_chunk {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: usize,
    pub location: png_byte,
}
pub type png_unknown_chunkp = *mut png_unknown_chunk;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
pub type png_unknown_chunkpp = *mut *mut png_unknown_chunk;

#[repr(C)]
#[derive(Copy, Clone, Default)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: usize,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}
pub type png_row_infop = *mut png_row_info;
pub type png_row_infopp = *mut *mut png_row_info;

/* ------------------------------------------------- simplified API (png.h) */

pub type png_controlp = *mut png_control;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_image {
    pub opaque: png_controlp,
    pub version: png_uint_32,
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub format: png_uint_32,
    pub flags: png_uint_32,
    pub colormap_entries: png_uint_32,
    pub warning_or_error: png_uint_32,
    pub message: [c_char; 64],
}
pub type png_imagep = *mut png_image;

/* png_control: private, from pngpriv.h.  The two bit fields are the low two
 * bits of a single 'unsigned int' allocation unit.
 */
#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,
    pub memory: png_const_bytep,
    pub size: usize,
    /// bit 0: for_write, bit 1: owned_file
    pub bitfields: c_uint,
}

impl png_control {
    #[inline]
    pub fn for_write(&self) -> c_uint {
        self.bitfields & 1
    }
    #[inline]
    pub fn set_for_write(&mut self, v: c_uint) {
        self.bitfields = (self.bitfields & !1) | (v & 1);
    }
    #[inline]
    pub fn owned_file(&self) -> c_uint {
        (self.bitfields >> 1) & 1
    }
    #[inline]
    pub fn set_owned_file(&mut self, v: c_uint) {
        self.bitfields = (self.bitfields & !2) | ((v & 1) << 1);
    }
}

/* ------------------------------------------------------------ pngstruct.h */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1], /* actually zbuffer_size */
}
pub type png_compression_bufferp = *mut png_compression_buffer;

/// `PNG_COMPRESSION_BUFFER_SIZE(pp)`
#[inline]
pub unsafe fn PNG_COMPRESSION_BUFFER_SIZE(pp: png_const_structrp) -> usize {
    core::mem::offset_of!(png_compression_buffer, output) + (*pp).zbuffer_size as usize
}

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

#[repr(C)]
#[derive(Copy, Clone, Default)]
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

pub type png_const_uint_16pp = *const png_uint_16p;

/// Row filter function pointer stored in png_struct::read_filter.
pub type png_read_filter_fn = Option<unsafe extern "C" fn(png_row_infop, png_bytep, png_const_bytep)>;

#[repr(C)]
pub struct png_struct_def {
    pub jmp_buf_local: jmp_buf,
    pub longjmp_fn: png_longjmp_ptr,
    pub jmp_buf_ptr: *mut jmp_buf,
    pub jmp_buf_size: usize,

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

    pub zlib_level: c_int,
    pub zlib_method: c_int,
    pub zlib_window_bits: c_int,
    pub zlib_mem_level: c_int,
    pub zlib_strategy: c_int,

    pub zlib_text_level: c_int,
    pub zlib_text_method: c_int,
    pub zlib_text_window_bits: c_int,
    pub zlib_text_mem_level: c_int,
    pub zlib_text_strategy: c_int,

    pub zlib_set_level: c_int,
    pub zlib_set_method: c_int,
    pub zlib_set_window_bits: c_int,
    pub zlib_set_mem_level: c_int,
    pub zlib_set_strategy: c_int,

    pub chunks: png_uint_32,

    pub width: png_uint_32,
    pub height: png_uint_32,
    pub num_rows: png_uint_32,
    pub usr_width: png_uint_32,
    pub rowbytes: usize,
    pub iwidth: png_uint_32,
    pub row_number: png_uint_32,
    pub chunk_name: png_uint_32,
    pub prev_row: png_bytep,
    pub row_buf: png_bytep,
    pub try_row: png_bytep,
    pub tst_row: png_bytep,
    pub info_rowbytes: usize,

    pub idat_size: png_uint_32,
    pub crc: png_uint_32,
    pub palette: png_colorp,
    pub num_palette: png_uint_16,

    pub num_palette_max: c_int,

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

    pub gamma_shift: c_int,
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
    pub save_buffer_size: usize,
    pub save_buffer_max: usize,
    pub buffer_size: usize,
    pub current_buffer_size: usize,
    pub process_mode: c_int,
    pub cur_palette: c_int,

    pub palette_lookup: png_bytep,
    pub quantize_index: png_bytep,

    pub options: png_uint_32,

    pub time_buffer: [c_char; 29],

    pub free_me: png_uint_32,

    pub user_chunk_ptr: png_voidp,
    pub read_user_chunk_fn: png_user_chunk_ptr,

    pub unknown_default: c_int,
    pub num_chunk_list: c_uint,
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

    pub old_big_row_buf_size: usize,

    pub read_buffer: png_bytep,
    pub read_buffer_size: png_alloc_size_t,

    pub IDAT_read_size: uInt,

    pub io_state: png_uint_32,

    pub big_prev_row: png_bytep,

    /// PNG_FILTER_VALUE_LAST - 1 == 4 entries
    pub read_filter: [png_read_filter_fn; 4],
}

/* -------------------------------------------------------------- pnginfo.h */

#[repr(C)]
#[derive(Copy, Clone)]
pub struct png_info_def {
    pub width: png_uint_32,
    pub height: png_uint_32,
    pub valid: png_uint_32,
    pub rowbytes: usize,
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

    pub num_text: c_int,
    pub max_text: c_int,
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
    pub unknown_chunks_num: c_int,

    pub splt_palettes: png_sPLT_tp,
    pub splt_palettes_num: c_int,

    pub scal_unit: png_byte,
    pub scal_s_width: png_charp,
    pub scal_s_height: png_charp,

    pub row_pointers: png_bytepp,

    pub cHRM: png_xy,

    pub gamma: png_fixed_point,

    pub rendering_intent: c_int,
}

/* ------------------------------------------------ png_handle_result_code */

pub type png_handle_result_code = c_int;
pub const handled_error: c_int = 0;
pub const handled_discarded: c_int = 1;
pub const handled_saved: c_int = 2;
pub const handled_ok: c_int = 3;

/* --------------------------------------------------- warning parameters */

pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;

/// `typedef char png_warning_parameters[8][32];`  As a function parameter this
/// decays to a pointer to the first row.
pub type png_warning_parameters = *mut [c_char; PNG_WARNING_PARAMETER_SIZE];
