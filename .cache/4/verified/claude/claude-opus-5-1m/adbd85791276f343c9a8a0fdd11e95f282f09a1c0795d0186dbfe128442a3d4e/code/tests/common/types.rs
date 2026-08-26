//! The public libpng ABI types, transcribed from `c_src/include/png.h`.
//!
//! These are the *caller's* view of the ABI: the differential tests are an
//! ordinary external consumer of the shared library, so they only ever use the
//! types png.h exposes.  `png_struct` / `png_info` are opaque.
#![allow(non_snake_case, non_camel_case_types, non_upper_case_globals, dead_code)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

/// Opaque `png_struct`.
#[repr(C)]
pub struct PngStruct {
    _opaque: [u8; 0],
}
/// Opaque `png_info`.
#[repr(C)]
pub struct PngInfo {
    _opaque: [u8; 0],
}
/// Opaque `jmp_buf` (glibc x86-64: 200 bytes).
#[repr(C)]
#[derive(Clone, Copy)]
pub struct JmpBuf(pub [u64; 25]);

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct Tm {
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
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color_16 {
    pub index: u8,
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub gray: u16,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_color_8 {
    pub red: u8,
    pub green: u8,
    pub blue: u8,
    pub gray: u8,
    pub alpha: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_sPLT_entry {
    pub red: u16,
    pub green: u16,
    pub blue: u16,
    pub alpha: u16,
    pub frequency: u16,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_sPLT_t {
    pub name: *mut c_char,
    pub depth: u8,
    pub entries: *mut png_sPLT_entry,
    pub nentries: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_text {
    pub compression: c_int,
    pub key: *mut c_char,
    pub text: *mut c_char,
    pub text_length: usize,
    pub itxt_length: usize,
    pub lang: *mut c_char,
    pub lang_key: *mut c_char,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_time {
    pub year: u16,
    pub month: u8,
    pub day: u8,
    pub hour: u8,
    pub minute: u8,
    pub second: u8,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_unknown_chunk {
    pub name: [u8; 5],
    pub data: *mut u8,
    pub size: usize,
    pub location: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_row_info {
    pub width: u32,
    pub rowbytes: usize,
    pub color_type: u8,
    pub bit_depth: u8,
    pub channels: u8,
    pub pixel_depth: u8,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_xy {
    pub redx: i32,
    pub redy: i32,
    pub greenx: i32,
    pub greeny: i32,
    pub bluex: i32,
    pub bluey: i32,
    pub whitex: i32,
    pub whitey: i32,
}

#[repr(C)]
#[derive(Clone, Copy, Default, Debug, PartialEq, Eq)]
pub struct png_XYZ {
    pub red_X: i32,
    pub red_Y: i32,
    pub red_Z: i32,
    pub green_X: i32,
    pub green_Y: i32,
    pub green_Z: i32,
    pub blue_X: i32,
    pub blue_Y: i32,
    pub blue_Z: i32,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct png_image {
    pub opaque: *mut c_void,
    pub version: u32,
    pub width: u32,
    pub height: u32,
    pub format: u32,
    pub flags: u32,
    pub colormap_entries: u32,
    pub warning_or_error: u32,
    pub message: [c_char; 64],
}

impl Default for png_image {
    fn default() -> Self {
        png_image {
            opaque: core::ptr::null_mut(),
            version: 0,
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

pub type png_error_ptr = Option<unsafe extern "C" fn(*mut PngStruct, *const c_char)>;
pub type png_rw_ptr = Option<unsafe extern "C" fn(*mut PngStruct, *mut u8, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C" fn(*mut PngStruct)>;
pub type png_read_status_ptr = Option<unsafe extern "C" fn(*mut PngStruct, u32, c_int)>;
pub type png_write_status_ptr = Option<unsafe extern "C" fn(*mut PngStruct, u32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C" fn(*mut PngStruct, *mut PngInfo)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C" fn(*mut PngStruct, *mut PngInfo)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C" fn(*mut PngStruct, *mut u8, u32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C" fn(*mut PngStruct, *mut png_row_info, *mut u8)>;
pub type png_user_chunk_ptr =
    Option<unsafe extern "C" fn(*mut PngStruct, *mut png_unknown_chunk) -> c_int>;
pub type png_longjmp_ptr = Option<unsafe extern "C" fn(*mut JmpBuf, c_int)>;
pub type png_malloc_ptr = Option<unsafe extern "C" fn(*mut PngStruct, usize) -> *mut c_void>;
pub type png_free_ptr = Option<unsafe extern "C" fn(*mut PngStruct, *mut c_void)>;

/* ------------------------------------------------------------------ */
/* png.h constants                                                     */
/* ------------------------------------------------------------------ */

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59.git\0";

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
pub const PNG_INTERLACE_LAST: c_int = 2;

pub const PNG_NO_FILTERS: c_int = 0x00;
pub const PNG_FILTER_NONE: c_int = 0x08;
pub const PNG_FILTER_SUB: c_int = 0x10;
pub const PNG_FILTER_UP: c_int = 0x20;
pub const PNG_FILTER_AVG: c_int = 0x40;
pub const PNG_FILTER_PAETH: c_int = 0x80;
pub const PNG_FAST_FILTERS: c_int = PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP;
pub const PNG_ALL_FILTERS: c_int = PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;

pub const PNG_FILTER_VALUE_NONE: c_int = 0;
pub const PNG_FILTER_VALUE_SUB: c_int = 1;
pub const PNG_FILTER_VALUE_UP: c_int = 2;
pub const PNG_FILTER_VALUE_AVG: c_int = 3;
pub const PNG_FILTER_VALUE_PAETH: c_int = 4;
pub const PNG_FILTER_VALUE_LAST: c_int = 5;

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

pub const PNG_INFO_gAMA: u32 = 0x0001;
pub const PNG_INFO_sBIT: u32 = 0x0002;
pub const PNG_INFO_cHRM: u32 = 0x0004;
pub const PNG_INFO_PLTE: u32 = 0x0008;
pub const PNG_INFO_tRNS: u32 = 0x0010;
pub const PNG_INFO_bKGD: u32 = 0x0020;
pub const PNG_INFO_hIST: u32 = 0x0040;
pub const PNG_INFO_pHYs: u32 = 0x0080;
pub const PNG_INFO_oFFs: u32 = 0x0100;
pub const PNG_INFO_tIME: u32 = 0x0200;
pub const PNG_INFO_pCAL: u32 = 0x0400;
pub const PNG_INFO_sRGB: u32 = 0x0800;
pub const PNG_INFO_iCCP: u32 = 0x1000;
pub const PNG_INFO_sPLT: u32 = 0x2000;
pub const PNG_INFO_sCAL: u32 = 0x4000;
pub const PNG_INFO_IDAT: u32 = 0x8000;
pub const PNG_INFO_eXIf: u32 = 0x10000;
pub const PNG_INFO_cICP: u32 = 0x20000;
pub const PNG_INFO_cLLI: u32 = 0x40000;
pub const PNG_INFO_mDCV: u32 = 0x80000;

pub const PNG_FREE_ALL: u32 = 0xffff;
pub const PNG_FREE_TEXT: u32 = 0x4000;
pub const PNG_FREE_ROWS: u32 = 0x0040;
pub const PNG_FREE_PLTE: u32 = 0x1000;
pub const PNG_FREE_TRNS: u32 = 0x2000;
pub const PNG_FREE_HIST: u32 = 0x0008;
pub const PNG_FREE_ICCP: u32 = 0x0010;
pub const PNG_FREE_SPLT: u32 = 0x0020;
pub const PNG_FREE_PCAL: u32 = 0x0080;
pub const PNG_FREE_SCAL: u32 = 0x0100;
pub const PNG_FREE_EXIF: u32 = 0x8000;
pub const PNG_FREE_UNKN: u32 = 0x0200;
pub const PNG_FREE_MUL: u32 = 0x4220;
pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 1;
pub const PNG_USER_WILL_FREE_DATA: c_int = 2;

pub const PNG_CRC_DEFAULT: c_int = 0;
pub const PNG_CRC_ERROR_QUIT: c_int = 1;
pub const PNG_CRC_WARN_DISCARD: c_int = 2;
pub const PNG_CRC_WARN_USE: c_int = 3;
pub const PNG_CRC_QUIET_USE: c_int = 4;
pub const PNG_CRC_NO_CHANGE: c_int = 5;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;
pub const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

pub const PNG_EQUATION_LINEAR: c_int = 0;
pub const PNG_EQUATION_BASE_E: c_int = 1;
pub const PNG_EQUATION_ARBITRARY: c_int = 2;
pub const PNG_EQUATION_HYPERBOLIC: c_int = 3;
pub const PNG_EQUATION_LAST: c_int = 4;

pub const PNG_SCALE_UNKNOWN: c_int = 0;
pub const PNG_SCALE_METER: c_int = 1;
pub const PNG_SCALE_RADIAN: c_int = 2;
pub const PNG_SCALE_LAST: c_int = 3;

pub const PNG_RESOLUTION_UNKNOWN: c_int = 0;
pub const PNG_RESOLUTION_METER: c_int = 1;
pub const PNG_RESOLUTION_LAST: c_int = 2;

pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_OFFSET_LAST: c_int = 2;

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_RELATIVE: c_int = 1;
pub const PNG_sRGB_INTENT_SATURATION: c_int = 2;
pub const PNG_sRGB_INTENT_ABSOLUTE: c_int = 3;
pub const PNG_sRGB_INTENT_LAST: c_int = 4;

pub const PNG_OPTION_NEXT: c_int = 16;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;
pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_IGNORE_ADLER32: c_int = 8;

pub const PNG_INFO_CHUNK_LOCATION_BEFORE_PLTE: u8 = 1;
pub const PNG_HAVE_IHDR: c_int = 0x01;
pub const PNG_HAVE_PLTE: c_int = 0x02;
pub const PNG_AFTER_IDAT: c_int = 0x08;

pub const PNG_FLAG_MNG_EMPTY_PLTE: c_int = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: c_int = 0x04;
pub const PNG_ALL_MNG_FEATURES: c_int = 0x05;

pub const PNG_IO_NONE: c_int = 0x0000;
pub const PNG_IO_READING: c_int = 0x0001;
pub const PNG_IO_WRITING: c_int = 0x0002;
pub const PNG_IO_SIGNATURE: c_int = 0x0010;
pub const PNG_IO_CHUNK_HDR: c_int = 0x0020;
pub const PNG_IO_CHUNK_DATA: c_int = 0x0040;
pub const PNG_IO_CHUNK_CRC: c_int = 0x0080;
pub const PNG_IO_MASK_OP: c_int = 0x000f;
pub const PNG_IO_MASK_LOC: c_int = 0x00f0;

pub const PNG_IMAGE_VERSION: u32 = 1;
pub const PNG_FORMAT_FLAG_ALPHA: u32 = 0x01;
pub const PNG_FORMAT_FLAG_COLOR: u32 = 0x02;
pub const PNG_FORMAT_FLAG_LINEAR: u32 = 0x04;
pub const PNG_FORMAT_FLAG_COLORMAP: u32 = 0x08;
pub const PNG_FORMAT_FLAG_BGR: u32 = 0x10;
pub const PNG_FORMAT_FLAG_AFIRST: u32 = 0x20;
pub const PNG_FORMAT_FLAG_ASSOCIATED_ALPHA: u32 = 0x40;

pub const PNG_FORMAT_GRAY: u32 = 0;
pub const PNG_FORMAT_GA: u32 = PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_AG: u32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_RGB: u32 = PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_BGR: u32 = PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_BGR;
pub const PNG_FORMAT_RGBA: u32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ARGB: u32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_BGRA: u32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_ABGR: u32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_AFIRST;
pub const PNG_FORMAT_LINEAR_Y: u32 = PNG_FORMAT_FLAG_LINEAR;
pub const PNG_FORMAT_LINEAR_Y_ALPHA: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_LINEAR_RGB: u32 = PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: u32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: u32 = PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: u32 = PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: u32 = PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: u32 = PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: u32 = PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: u32 = PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;

pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: u32 = 0x01;
pub const PNG_IMAGE_FLAG_FAST: u32 = 0x02;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: u32 = 0x04;

pub const PNG_UINT_31_MAX: u32 = 0x7fffffff;
pub const PNG_UINT_32_MAX: u32 = 0xffffffff;
pub const PNG_MAX_PALETTE_LENGTH: c_int = 256;
pub const PNG_FP_1: c_int = 100000;
pub const PNG_FP_HALF: c_int = 50000;
pub const PNG_FP_MAX: i32 = 0x7fffffff;
pub const PNG_FP_MIN: i32 = -PNG_FP_MAX;

pub const PNG_DEFAULT_sBIT: c_int = 0;
pub const PNG_GAMMA_THRESHOLD_FIXED: i32 = 5000;

/// `PNG_ROWBYTES` from pngpriv.h
pub fn png_rowbytes(pixel_bits: usize, width: usize) -> usize {
    if pixel_bits >= 8 {
        width * (pixel_bits / 8)
    } else {
        (width * pixel_bits + 7) >> 3
    }
}

pub fn channels_of(color_type: c_int) -> usize {
    match color_type {
        PNG_COLOR_TYPE_GRAY => 1,
        PNG_COLOR_TYPE_PALETTE => 1,
        PNG_COLOR_TYPE_RGB => 3,
        PNG_COLOR_TYPE_GRAY_ALPHA => 2,
        PNG_COLOR_TYPE_RGB_ALPHA => 4,
        _ => 0,
    }
}

/// The 15 valid (color_type, bit_depth) pairs of the PNG specification.
pub const VALID_SHAPES: [(c_int, c_int); 15] = [
    (PNG_COLOR_TYPE_GRAY, 1),
    (PNG_COLOR_TYPE_GRAY, 2),
    (PNG_COLOR_TYPE_GRAY, 4),
    (PNG_COLOR_TYPE_GRAY, 8),
    (PNG_COLOR_TYPE_GRAY, 16),
    (PNG_COLOR_TYPE_PALETTE, 1),
    (PNG_COLOR_TYPE_PALETTE, 2),
    (PNG_COLOR_TYPE_PALETTE, 4),
    (PNG_COLOR_TYPE_PALETTE, 8),
    (PNG_COLOR_TYPE_RGB, 8),
    (PNG_COLOR_TYPE_RGB, 16),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 8),
    (PNG_COLOR_TYPE_GRAY_ALPHA, 16),
    (PNG_COLOR_TYPE_RGB_ALPHA, 8),
    (PNG_COLOR_TYPE_RGB_ALPHA, 16),
];

pub const _UNUSED: (c_uint, c_ulong) = (0, 0);
