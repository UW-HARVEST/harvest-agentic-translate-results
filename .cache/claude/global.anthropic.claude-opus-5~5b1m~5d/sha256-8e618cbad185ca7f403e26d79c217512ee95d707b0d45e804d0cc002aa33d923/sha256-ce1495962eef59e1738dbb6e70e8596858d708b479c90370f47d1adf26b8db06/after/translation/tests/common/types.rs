//! Public libpng types, mirrored from `c_src/include/png.h` (and the handful of
//! private ones that appear in exported prototypes, from `pngpriv.h`).
//!
//! The tests are *external consumers*: they never link the Rust crate, they
//! `dlopen` both shared objects.  Therefore the types are redeclared here.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

pub use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

pub type png_byte = u8;
pub type png_int_16 = i16;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_size_t = usize;
pub type png_alloc_size_t = usize;
pub type png_fixed_point = i32;

pub type png_voidp = *mut c_void;
pub type png_const_voidp = *const c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_32p = *mut png_uint_32;
pub type png_const_uint_32p = *const png_uint_32;
pub type png_int_32p = *mut png_int_32;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_const_fixed_point_p = *const png_fixed_point;
pub type png_doublep = *mut c_double;
pub type png_const_doublep = *const c_double;
pub type png_bytepp = *mut png_bytep;
pub type png_uint_16pp = *mut png_uint_16p;
pub type png_charpp = *mut png_charp;
pub type png_const_charpp = *mut png_const_charp;
pub type png_FILE_p = *mut c_void;

pub type uInt = c_uint;
pub type uLong = c_ulong;
pub type voidpf = *mut c_void;
pub type time_t = i64;

#[repr(C)]
pub struct tm {
    pub tm_sec: c_int,
    pub tm_min: c_int,
    pub tm_hour: c_int,
    pub tm_mday: c_int,
    pub tm_mon: c_int,
    pub tm_year: c_int,
    pub tm_wday: c_int,
    pub tm_yday: c_int,
    pub tm_isdst: c_int,
    pub tm_gmtoff: c_long,
    pub tm_zone: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct png_color_16 {
    pub index: png_byte,
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub gray: png_uint_16,
}
pub type png_color_16p = *mut png_color_16;
pub type png_const_color_16p = *const png_color_16;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct png_color_8 {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
    pub gray: png_byte,
    pub alpha: png_byte,
}
pub type png_color_8p = *mut png_color_8;
pub type png_const_color_8p = *const png_color_8;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct png_sPLT_entry {
    pub red: png_uint_16,
    pub green: png_uint_16,
    pub blue: png_uint_16,
    pub alpha: png_uint_16,
    pub frequency: png_uint_16,
}
pub type png_sPLT_entryp = *mut png_sPLT_entry;
pub type png_const_sPLT_entryp = *const png_sPLT_entry;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct png_sPLT_t {
    pub name: png_charp,
    pub depth: png_byte,
    pub entries: png_sPLT_entryp,
    pub nentries: png_int_32,
}
pub type png_sPLT_tp = *mut png_sPLT_t;
pub type png_const_sPLT_tp = *const png_sPLT_t;
pub type png_sPLT_tpp = *mut png_sPLT_tp;

#[repr(C)]
#[derive(Copy, Clone, Debug)]
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
pub type png_textpp = *mut png_textp;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
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

#[repr(C)]
#[derive(Copy, Clone, Debug)]
pub struct png_unknown_chunk {
    pub name: [png_byte; 5],
    pub data: *mut png_byte,
    pub size: usize,
    pub location: png_byte,
}
pub type png_unknown_chunkp = *mut png_unknown_chunk;
pub type png_const_unknown_chunkp = *const png_unknown_chunk;
pub type png_unknown_chunkpp = *mut png_unknown_chunkp;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
pub struct png_row_info {
    pub width: png_uint_32,
    pub rowbytes: usize,
    pub color_type: png_byte,
    pub bit_depth: png_byte,
    pub channels: png_byte,
    pub pixel_depth: png_byte,
}
pub type png_row_infop = *mut png_row_info;

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
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
#[derive(Copy, Clone, Default, PartialEq, Eq, Debug)]
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

/* Opaque structures -- only ever handled through pointers. */
#[repr(C)]
pub struct png_struct {
    _private: [u8; 0],
}
#[repr(C)]
pub struct png_info {
    _private: [u8; 0],
}
#[repr(C)]
pub struct png_control {
    _private: [u8; 0],
}
#[repr(C)]
pub struct png_compression_buffer {
    _private: [u8; 0],
}
pub type png_compression_bufferp = *mut png_compression_buffer;
pub type png_controlp = *mut png_control;

pub type png_structp = *mut png_struct;
pub type png_const_structp = *const png_struct;
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;
pub type png_structpp = *mut png_structp;
pub type png_infop = *mut png_info;
pub type png_const_infop = *const png_info;
pub type png_inforp = *mut png_info;
pub type png_const_inforp = *const png_info;
pub type png_infopp = *mut png_infop;

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

impl Default for png_image {
    fn default() -> Self {
        png_image {
            opaque: core::ptr::null_mut(),
            version: PNG_IMAGE_VERSION,
            width: 0,
            height: 0,
            format: 0,
            flags: 0,
            colormap_entries: 0,
            warning_or_error: 0,
            message: [0; 64],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct jmp_buf(pub [u64; 25]);
impl Default for jmp_buf {
    fn default() -> Self {
        jmp_buf([0; 25])
    }
}

pub type png_error_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C-unwind" fn(png_structp)>;
pub type png_read_status_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_row_infop, png_bytep)>;
pub type png_user_chunk_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_unknown_chunkp) -> c_int>;
pub type png_malloc_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_voidp)>;
pub type png_longjmp_ptr = Option<unsafe extern "C-unwind" fn(*mut jmp_buf, c_int) -> !>;

// ---------------------------------------------------------------------------
// Constants from png.h / pngpriv.h that the tests need.
// ---------------------------------------------------------------------------

pub const PNG_LIBPNG_VER_STRING: &str = "1.6.59.git";
pub const PNG_IMAGE_VERSION: png_uint_32 = 1;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fff_ffff;
pub const PNG_UINT_32_MAX: png_uint_32 = 0xffff_ffff;

pub const PNG_FP_1: png_fixed_point = 100_000;
pub const PNG_FP_HALF: png_fixed_point = 50_000;
pub const PNG_FP_MAX: png_fixed_point = 0x7fff_ffff;
pub const PNG_FP_MIN: png_fixed_point = -PNG_FP_MAX;

pub const PNG_COLOR_MASK_PALETTE: c_int = 1;
pub const PNG_COLOR_MASK_COLOR: c_int = 2;
pub const PNG_COLOR_MASK_ALPHA: c_int = 4;

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: c_int = 3;
pub const PNG_COLOR_TYPE_RGB: c_int = 2;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = 6;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = 4;

pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;
pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

pub const PNG_INFO_gAMA: png_uint_32 = 0x0001;
pub const PNG_INFO_sBIT: png_uint_32 = 0x0002;
pub const PNG_INFO_cHRM: png_uint_32 = 0x0004;
pub const PNG_INFO_PLTE: png_uint_32 = 0x0008;
pub const PNG_INFO_tRNS: png_uint_32 = 0x0010;
pub const PNG_INFO_bKGD: png_uint_32 = 0x0020;
pub const PNG_INFO_hIST: png_uint_32 = 0x0040;
pub const PNG_INFO_pHYs: png_uint_32 = 0x0080;
pub const PNG_INFO_oFFs: png_uint_32 = 0x0100;
pub const PNG_INFO_tIME: png_uint_32 = 0x0200;
pub const PNG_INFO_pCAL: png_uint_32 = 0x0400;
pub const PNG_INFO_sRGB: png_uint_32 = 0x0800;
pub const PNG_INFO_iCCP: png_uint_32 = 0x1000;
pub const PNG_INFO_sPLT: png_uint_32 = 0x2000;
pub const PNG_INFO_sCAL: png_uint_32 = 0x4000;
pub const PNG_INFO_IDAT: png_uint_32 = 0x8000;
pub const PNG_INFO_eXIf: png_uint_32 = 0x0001_0000;
pub const PNG_INFO_cICP: png_uint_32 = 0x0002_0000;
pub const PNG_INFO_cLLI: png_uint_32 = 0x0004_0000;
pub const PNG_INFO_mDCV: png_uint_32 = 0x0008_0000;

pub const PNG_TRANSFORM_IDENTITY: c_int = 0x0000;
pub const PNG_TRANSFORM_STRIP_16: c_int = 0x0001;
pub const PNG_TRANSFORM_STRIP_ALPHA: c_int = 0x0002;
pub const PNG_TRANSFORM_PACKING: c_int = 0x0004;
pub const PNG_TRANSFORM_PACKSWAP: c_int = 0x0008;
pub const PNG_TRANSFORM_EXPAND: c_int = 0x0010;
pub const PNG_TRANSFORM_INVERT_MONO: c_int = 0x0020;
pub const PNG_TRANSFORM_SHIFT: c_int = 0x0040;
pub const PNG_TRANSFORM_BGR: c_int = 0x0080;
pub const PNG_TRANSFORM_SWAP_ALPHA: c_int = 0x0100;
pub const PNG_TRANSFORM_SWAP_ENDIAN: c_int = 0x0200;
pub const PNG_TRANSFORM_INVERT_ALPHA: c_int = 0x0400;
pub const PNG_TRANSFORM_STRIP_FILLER: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: c_int = 0x0800;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: c_int = 0x1000;
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;
pub const PNG_TRANSFORM_EXPAND_16: c_int = 0x4000;
pub const PNG_TRANSFORM_SCALE_16: c_int = 0x8000;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_ASSOCIATED: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_DEFAULT_sRGB: png_fixed_point = -1;
pub const PNG_GAMMA_MAC_18: png_fixed_point = -2;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_FAST_FILTERS: c_int = 0x08 | 0x10 | 0x20;
pub const PNG_ALL_FILTERS: c_int = 0x08 | 0x10 | 0x20 | 0x40 | 0x80;

pub const PNG_FILTER_VALUE_NONE: c_int = 0;
pub const PNG_FILTER_VALUE_SUB: c_int = 1;
pub const PNG_FILTER_VALUE_UP: c_int = 2;
pub const PNG_FILTER_VALUE_AVG: c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: c_int = 4;
pub const PNG_FILTER_VALUE_LAST: c_int = 5;

pub const PNG_FILTER_HEURISTIC_DEFAULT: c_int = 0;
pub const PNG_FILTER_HEURISTIC_UNWEIGHTED: c_int = 1;
pub const PNG_FILTER_HEURISTIC_WEIGHTED: c_int = 2;
pub const PNG_FILTER_HEURISTIC_LAST: c_int = 3;

pub const PNG_FREE_HIST: c_int = 0x0008;
pub const PNG_FREE_ICCP: c_int = 0x0010;
pub const PNG_FREE_SPLT: c_int = 0x0020;
pub const PNG_FREE_ROWS: c_int = 0x0040;
pub const PNG_FREE_PCAL: c_int = 0x0080;
pub const PNG_FREE_SCAL: c_int = 0x0100;
pub const PNG_FREE_UNKN: c_int = 0x0200;
pub const PNG_FREE_PLTE: c_int = 0x1000;
pub const PNG_FREE_TRNS: c_int = 0x2000;
pub const PNG_FREE_TEXT: c_int = 0x4000;
pub const PNG_FREE_EXIF: c_int = 0x8000;
pub const PNG_FREE_ALL: c_int = 0x7fff;
pub const PNG_FREE_MUL: c_int = 0x4220;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

pub const PNG_AFTER_IDAT: png_uint_32 = 0x08;
pub const PNG_HAVE_IHDR: png_uint_32 = 0x01;
pub const PNG_HAVE_PLTE: png_uint_32 = 0x02;

pub const PNG_IO_NONE: c_int = 0x0000;
pub const PNG_IO_READING: c_int = 0x0001;
pub const PNG_IO_WRITING: c_int = 0x0002;
pub const PNG_IO_SIGNATURE: c_int = 0x0010;
pub const PNG_IO_CHUNK_HDR: c_int = 0x0020;
pub const PNG_IO_CHUNK_DATA: c_int = 0x0040;
pub const PNG_IO_CHUNK_CRC: c_int = 0x0080;

pub const PNG_OPTION_NEXT: c_int = 12;
pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

pub const PNG_INFO_IMAGE_SUPPORTED: bool = true;

/* Simplified API formats */
pub const PNG_FORMAT_FLAG_ALPHA: png_uint_32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: png_uint_32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: png_uint_32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: png_uint_32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: png_uint_32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: png_uint_32 = 0x20;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: png_uint_32 = 0x40;

pub const PNG_FORMAT_GRAY: png_uint_32 = 0;
pub const PNG_FORMAT_GA: png_uint_32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: png_uint_32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: png_uint_32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: png_uint_32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: png_uint_32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: png_uint_32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: png_uint_32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: png_uint_32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: png_uint_32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: png_uint_32 = PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: png_uint_32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: png_uint_32 = PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;

pub const PNG_IMAGE_WARNING: png_uint_32 = 1;
pub const PNG_IMAGE_ERROR: png_uint_32 = 2;

pub const PNG_MNG_FEATURES_SUPPORTED_FLAG: png_uint_32 = 0;
pub const PNG_FLAG_MNG_EMPTY_PLTE: png_uint_32 = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: png_uint_32 = 0x04;
pub const PNG_ALL_MNG_FEATURES: png_uint_32 = 0x05;

pub const PNG_DATA_FREER: c_int = 0;
pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 2;
pub const PNG_USER_WILL_FREE_DATA: c_int = 3;

pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;
pub const PNG_NUMBER_BUFFER_SIZE: usize = 24;
pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_d: c_int = 1;
pub const PNG_NUMBER_FORMAT_02d: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;

pub static PNG_SIG: [png_byte; 8] = [137, 80, 78, 71, 13, 10, 26, 10];
