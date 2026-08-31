//! png_struct / png_info / png_control - internal structures (pngstruct.h,
//! pnginfo.h).  Field order matches the C originals.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(dead_code)]

use core::ffi::{c_char, c_int, c_uint, c_void};

use crate::types::*;
use crate::zlib::{uInt, z_stream};

#[repr(C)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1], /* actually zbuffer_size */
}
pub type png_compression_bufferp = *mut png_compression_buffer;

/// `PNG_COMPRESSION_BUFFER_SIZE(pp)`
#[inline]
pub fn png_compression_buffer_size(zbuffer_size: uInt) -> usize {
    core::mem::offset_of!(png_compression_buffer, output) + zbuffer_size as usize
}

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
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

// --- png_index (PNG_KNOWN_CHUNKS) ------------------------------------------
pub const PNG_INDEX_IHDR: u32 = 0;
pub const PNG_INDEX_PLTE: u32 = 1;
pub const PNG_INDEX_IDAT: u32 = 2;
pub const PNG_INDEX_IEND: u32 = 3;
pub const PNG_INDEX_acTL: u32 = 4;
pub const PNG_INDEX_bKGD: u32 = 5;
pub const PNG_INDEX_cHRM: u32 = 6;
pub const PNG_INDEX_cICP: u32 = 7;
pub const PNG_INDEX_cLLI: u32 = 8;
pub const PNG_INDEX_eXIf: u32 = 9;
pub const PNG_INDEX_fcTL: u32 = 10;
pub const PNG_INDEX_fdAT: u32 = 11;
pub const PNG_INDEX_gAMA: u32 = 12;
pub const PNG_INDEX_hIST: u32 = 13;
pub const PNG_INDEX_iCCP: u32 = 14;
pub const PNG_INDEX_iTXt: u32 = 15;
pub const PNG_INDEX_mDCV: u32 = 16;
pub const PNG_INDEX_oFFs: u32 = 17;
pub const PNG_INDEX_pCAL: u32 = 18;
pub const PNG_INDEX_pHYs: u32 = 19;
pub const PNG_INDEX_sBIT: u32 = 20;
pub const PNG_INDEX_sCAL: u32 = 21;
pub const PNG_INDEX_sPLT: u32 = 22;
pub const PNG_INDEX_sRGB: u32 = 23;
pub const PNG_INDEX_tEXt: u32 = 24;
pub const PNG_INDEX_tIME: u32 = 25;
pub const PNG_INDEX_tRNS: u32 = 26;
pub const PNG_INDEX_zTXt: u32 = 27;
pub const PNG_INDEX_unknown: u32 = 28;

#[inline]
pub const fn png_chunk_flag_from_index(i: u32) -> png_uint_32 {
    0x80000000u32 >> (31 - i)
}

pub type png_read_filter_fn =
    Option<unsafe extern "C-unwind" fn(png_row_infop, png_bytep, png_const_bytep)>;

#[repr(C)]
pub struct png_struct {
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

    pub read_filter: [png_read_filter_fn; 4],
}

impl Default for png_struct {
    fn default() -> Self {
        // Equivalent to the memset(0) used by png_create_png_struct.
        unsafe { core::mem::zeroed() }
    }
}

#[repr(C)]
pub struct png_info {
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

impl Default for png_info {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

/// The internal structure that `png_image::opaque` points to.
#[repr(C)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,

    pub memory: png_const_bytep,
    pub size: usize,

    /// bitfield: bit0 = for_write, bit1 = owned_file
    pub bits: c_uint,
}

impl png_control {
    #[inline]
    pub fn for_write(&self) -> bool {
        (self.bits & 1) != 0
    }
    #[inline]
    pub fn set_for_write(&mut self, v: bool) {
        if v {
            self.bits |= 1
        } else {
            self.bits &= !1
        }
    }
    #[inline]
    pub fn owned_file(&self) -> bool {
        (self.bits & 2) != 0
    }
    #[inline]
    pub fn set_owned_file(&mut self, v: bool) {
        if v {
            self.bits |= 2
        } else {
            self.bits &= !2
        }
    }
}

impl Default for png_control {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

#[allow(unused)]
fn _unused(_: *mut c_void) {}
