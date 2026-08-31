//! Public types from png.h / pngconf.h.
#![allow(non_camel_case_types)]
#![allow(dead_code)]

use core::ffi::{c_char, c_double, c_int, c_uint, c_void};

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

/// C `FILE *`
pub type png_FILE_p = *mut c_void;

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59.git\0";
pub const PNG_HEADER_VERSION_STRING: &[u8] = b" libpng version 1.6.59.git\n\0";
pub const PNG_LIBPNG_VER: u32 = 10659;
pub const PNG_LIBPNG_VER_MAJOR: c_int = 1;
pub const PNG_LIBPNG_VER_MINOR: c_int = 6;
pub const PNG_LIBPNG_VER_RELEASE: c_int = 59;
pub const PNG_LIBPNG_VER_BUILD: c_int = 1;
pub const PNG_LIBPNG_BUILD_ALPHA: c_int = 1;
pub const PNG_LIBPNG_BUILD_BETA: c_int = 2;
pub const PNG_LIBPNG_BUILD_RC: c_int = 3;
pub const PNG_LIBPNG_BUILD_STABLE: c_int = 4;
pub const PNG_LIBPNG_BUILD_RELEASE_STATUS_MASK: c_int = 7;
pub const PNG_LIBPNG_BUILD_BASE_TYPE: c_int = PNG_LIBPNG_BUILD_BETA;
/// PNG_RELEASE_BUILD == (PNG_LIBPNG_BUILD_BASE_TYPE >= PNG_LIBPNG_BUILD_RC)
pub const PNG_RELEASE_BUILD: bool = PNG_LIBPNG_BUILD_BASE_TYPE >= PNG_LIBPNG_BUILD_RC;

// ---------------------------------------------------------------------------
// Structures
// ---------------------------------------------------------------------------

#[repr(C)]
#[derive(Copy, Clone, Default, PartialEq, Eq)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;

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
pub type png_sPLT_tpp = *mut png_sPLT_tp;

impl Default for png_sPLT_t {
    fn default() -> Self {
        png_sPLT_t {
            name: core::ptr::null_mut(),
            depth: 0,
            entries: core::ptr::null_mut(),
            nentries: 0,
        }
    }
}

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
pub type png_textpp = *mut png_textp;

impl Default for png_text {
    fn default() -> Self {
        png_text {
            compression: 0,
            key: core::ptr::null_mut(),
            text: core::ptr::null_mut(),
            text_length: 0,
            itxt_length: 0,
            lang: core::ptr::null_mut(),
            lang_key: core::ptr::null_mut(),
        }
    }
}

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

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
pub type png_unknown_chunkpp = *mut png_unknown_chunkp;

impl Default for png_unknown_chunk {
    fn default() -> Self {
        png_unknown_chunk {
            name: [0; 5],
            data: core::ptr::null_mut(),
            size: 0,
            location: 0,
        }
    }
}

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

// ---------------------------------------------------------------------------
// Callback types.  All use the "C-unwind" ABI (identical to "C" for calling
// purposes) so that the panic-based emulation of setjmp/longjmp used
// internally by the simplified API can propagate correctly.
// ---------------------------------------------------------------------------

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

/// `jmp_buf` on the target platform (x86-64 / aarch64 glibc: 200 bytes).
/// Only ever passed opaquely to an application supplied `longjmp`.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct jmp_buf(pub [u64; 25]);

impl Default for jmp_buf {
    fn default() -> Self {
        jmp_buf([0; 25])
    }
}

pub type png_longjmp_ptr = Option<unsafe extern "C-unwind" fn(*mut jmp_buf, c_int) -> !>;

pub use crate::pngstruct::{png_info, png_struct};

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

// ---------------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------------

pub const PNG_HAVE_IHDR: png_uint_32 = 0x01;
pub const PNG_HAVE_PLTE: png_uint_32 = 0x02;
pub const PNG_AFTER_IDAT: png_uint_32 = 0x08;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff;
pub const PNG_UINT_32_MAX: png_uint_32 = 0xffffffff;
pub const PNG_SIZE_MAX: usize = usize::MAX;

pub const PNG_FP_1: png_fixed_point = 100000;
pub const PNG_FP_HALF: png_fixed_point = 50000;
pub const PNG_FP_MAX: png_fixed_point = 0x7fffffff;
pub const PNG_FP_MIN: png_fixed_point = -PNG_FP_MAX;

pub const PNG_COLOR_MASK_PALETTE: c_int = 1;
pub const PNG_COLOR_MASK_COLOR: c_int = 2;
pub const PNG_COLOR_MASK_ALPHA: c_int = 4;

pub const PNG_COLOR_TYPE_GRAY: c_int = 0;
pub const PNG_COLOR_TYPE_PALETTE: c_int = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_PALETTE;
pub const PNG_COLOR_TYPE_RGB: c_int = PNG_COLOR_MASK_COLOR;
pub const PNG_COLOR_TYPE_RGB_ALPHA: c_int = PNG_COLOR_MASK_COLOR | PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_GRAY_ALPHA: c_int = PNG_COLOR_MASK_ALPHA;
pub const PNG_COLOR_TYPE_RGBA: c_int = PNG_COLOR_TYPE_RGB_ALPHA;
pub const PNG_COLOR_TYPE_GA: c_int = PNG_COLOR_TYPE_GRAY_ALPHA;

pub const PNG_COMPRESSION_TYPE_BASE: c_int = 0;
pub const PNG_COMPRESSION_TYPE_DEFAULT: c_int = PNG_COMPRESSION_TYPE_BASE;

pub const PNG_FILTER_TYPE_BASE: c_int = 0;
pub const PNG_INTRAPIXEL_DIFFERENCING: c_int = 64;
pub const PNG_FILTER_TYPE_DEFAULT: c_int = PNG_FILTER_TYPE_BASE;

pub const PNG_INTERLACE_NONE: c_int = 0;
pub const PNG_INTERLACE_ADAM7: c_int = 1;
pub const PNG_INTERLACE_LAST: c_int = 2;

pub const PNG_OFFSET_PIXEL: c_int = 0;
pub const PNG_OFFSET_MICROMETER: c_int = 1;
pub const PNG_OFFSET_LAST: c_int = 2;

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

pub const PNG_sRGB_INTENT_PERCEPTUAL: c_int = 0;
pub const PNG_sRGB_INTENT_RELATIVE: c_int = 1;
pub const PNG_sRGB_INTENT_SATURATION: c_int = 2;
pub const PNG_sRGB_INTENT_ABSOLUTE: c_int = 3;
pub const PNG_sRGB_INTENT_LAST: c_int = 4;

pub const PNG_KEYWORD_MAX_LENGTH: usize = 79;
pub const PNG_MAX_PALETTE_LENGTH: c_int = 256;

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
pub const PNG_INFO_eXIf: png_uint_32 = 0x10000;
pub const PNG_INFO_cICP: png_uint_32 = 0x20000;
pub const PNG_INFO_cLLI: png_uint_32 = 0x40000;
pub const PNG_INFO_mDCV: png_uint_32 = 0x80000;
pub const PNG_INFO_acTL: png_uint_32 = 0x100000;
pub const PNG_INFO_fcTL: png_uint_32 = 0x200000;
pub const PNG_INFO_fdAT: png_uint_32 = 0x400000;

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
pub const PNG_TRANSFORM_STRIP_FILLER_BEFORE: c_int = PNG_TRANSFORM_STRIP_FILLER;
pub const PNG_TRANSFORM_STRIP_FILLER_AFTER: c_int = 0x1000;
pub const PNG_TRANSFORM_GRAY_TO_RGB: c_int = 0x2000;
pub const PNG_TRANSFORM_EXPAND_16: c_int = 0x4000;
pub const PNG_TRANSFORM_SCALE_16: c_int = 0x8000;

pub const PNG_FLAG_MNG_EMPTY_PLTE: png_uint_32 = 0x01;
pub const PNG_FLAG_MNG_FILTER_64: png_uint_32 = 0x04;
pub const PNG_ALL_MNG_FEATURES: png_uint_32 = 0x05;

pub const PNG_ERROR_ACTION_NONE: c_int = 1;
pub const PNG_ERROR_ACTION_WARN: c_int = 2;
pub const PNG_ERROR_ACTION_ERROR: c_int = 3;
pub const PNG_RGB_TO_GRAY_DEFAULT: c_int = -1;

pub const PNG_ALPHA_PNG: c_int = 0;
pub const PNG_ALPHA_STANDARD: c_int = 1;
pub const PNG_ALPHA_ASSOCIATED: c_int = 1;
pub const PNG_ALPHA_PREMULTIPLIED: c_int = 1;
pub const PNG_ALPHA_OPTIMIZED: c_int = 2;
pub const PNG_ALPHA_BROKEN: c_int = 3;

pub const PNG_DEFAULT_sRGB: png_fixed_point = -1;
pub const PNG_GAMMA_MAC_18: png_fixed_point = -2;
pub const PNG_GAMMA_sRGB: png_fixed_point = 220000;
pub const PNG_GAMMA_LINEAR: png_fixed_point = PNG_FP_1;

pub const PNG_FILLER_BEFORE: c_int = 0;
pub const PNG_FILLER_AFTER: c_int = 1;

pub const PNG_BACKGROUND_GAMMA_UNKNOWN: c_int = 0;
pub const PNG_BACKGROUND_GAMMA_SCREEN: c_int = 1;
pub const PNG_BACKGROUND_GAMMA_FILE: c_int = 2;
pub const PNG_BACKGROUND_GAMMA_UNIQUE: c_int = 3;

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
pub const PNG_FAST_FILTERS: c_int = PNG_FILTER_NONE | PNG_FILTER_SUB | PNG_FILTER_UP;
pub const PNG_ALL_FILTERS: c_int = PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;

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

pub const PNG_DESTROY_WILL_FREE_DATA: c_int = 1;
pub const PNG_SET_WILL_FREE_DATA: c_int = 1;
pub const PNG_USER_WILL_FREE_DATA: c_int = 2;

pub const PNG_FREE_HIST: png_uint_32 = 0x0008;
pub const PNG_FREE_ICCP: png_uint_32 = 0x0010;
pub const PNG_FREE_SPLT: png_uint_32 = 0x0020;
pub const PNG_FREE_ROWS: png_uint_32 = 0x0040;
pub const PNG_FREE_PCAL: png_uint_32 = 0x0080;
pub const PNG_FREE_SCAL: png_uint_32 = 0x0100;
pub const PNG_FREE_UNKN: png_uint_32 = 0x0200;
pub const PNG_FREE_PLTE: png_uint_32 = 0x1000;
pub const PNG_FREE_TRNS: png_uint_32 = 0x2000;
pub const PNG_FREE_TEXT: png_uint_32 = 0x4000;
pub const PNG_FREE_EXIF: png_uint_32 = 0x8000;
pub const PNG_FREE_ALL: png_uint_32 = 0xffff;
pub const PNG_FREE_MUL: png_uint_32 = 0x4220;

pub const PNG_HANDLE_CHUNK_AS_DEFAULT: c_int = 0;
pub const PNG_HANDLE_CHUNK_NEVER: c_int = 1;
pub const PNG_HANDLE_CHUNK_IF_SAFE: c_int = 2;
pub const PNG_HANDLE_CHUNK_ALWAYS: c_int = 3;
pub const PNG_HANDLE_CHUNK_LAST: c_int = 4;

pub const PNG_IO_NONE: png_uint_32 = 0x0000;
pub const PNG_IO_READING: png_uint_32 = 0x0001;
pub const PNG_IO_WRITING: png_uint_32 = 0x0002;
pub const PNG_IO_SIGNATURE: png_uint_32 = 0x0010;
pub const PNG_IO_CHUNK_HDR: png_uint_32 = 0x0020;
pub const PNG_IO_CHUNK_DATA: png_uint_32 = 0x0040;
pub const PNG_IO_CHUNK_CRC: png_uint_32 = 0x0080;
pub const PNG_IO_MASK_OP: png_uint_32 = 0x000f;
pub const PNG_IO_MASK_LOC: png_uint_32 = 0x00f0;

pub const PNG_INTERLACE_ADAM7_PASSES: c_int = 7;

pub const PNG_IMAGE_VERSION: png_uint_32 = 1;
pub const PNG_IMAGE_WARNING: png_uint_32 = 1;
pub const PNG_IMAGE_ERROR: png_uint_32 = 2;

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

pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
pub const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: png_uint_32 = 0x04;

pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_OPTION_NEXT: c_int = 16;
pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

// ---------------------------------------------------------------------------
// png_image (simplified API)
// ---------------------------------------------------------------------------

pub type png_controlp = *mut crate::pngstruct::png_control;

#[repr(C)]
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

// ---------------------------------------------------------------------------
// Interlace helper functions (the png.h macros)
// ---------------------------------------------------------------------------

#[inline]
pub const fn png_pass_start_row(pass: c_int) -> c_int {
    ((1 & !pass) << (3 - (pass >> 1))) & 7
}
#[inline]
pub const fn png_pass_start_col(pass: c_int) -> c_int {
    ((1 & pass) << (3 - ((pass + 1) >> 1))) & 7
}
#[inline]
pub const fn png_pass_row_offset(pass: c_int) -> c_int {
    if pass > 2 {
        8 >> ((pass - 1) >> 1)
    } else {
        8
    }
}
#[inline]
pub const fn png_pass_col_offset(pass: c_int) -> c_int {
    1 << ((7 - pass) >> 1)
}
#[inline]
pub const fn png_pass_row_shift(pass: c_int) -> c_int {
    if pass > 2 {
        (8 - pass) >> 1
    } else {
        3
    }
}
#[inline]
pub const fn png_pass_col_shift(pass: c_int) -> c_int {
    if pass > 1 {
        (7 - pass) >> 1
    } else {
        3
    }
}
#[inline]
pub const fn png_pass_rows(height: png_uint_32, pass: c_int) -> png_uint_32 {
    (height.wrapping_add(
        ((1u32 << png_pass_row_shift(pass)) - 1).wrapping_sub(png_pass_start_row(pass) as u32),
    )) >> png_pass_row_shift(pass)
}
#[inline]
pub const fn png_pass_cols(width: png_uint_32, pass: c_int) -> png_uint_32 {
    (width.wrapping_add(
        ((1u32 << png_pass_col_shift(pass)) - 1).wrapping_sub(png_pass_start_col(pass) as u32),
    )) >> png_pass_col_shift(pass)
}
#[inline]
pub const fn png_row_from_pass_row(y_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (y_in << png_pass_row_shift(pass)).wrapping_add(png_pass_start_row(pass) as u32)
}
#[inline]
pub const fn png_col_from_pass_col(x_in: png_uint_32, pass: c_int) -> png_uint_32 {
    (x_in << png_pass_col_shift(pass)).wrapping_add(png_pass_start_col(pass) as u32)
}
#[inline]
pub const fn png_pass_mask(pass: c_int, off: c_int) -> c_uint {
    ((0x110145AFu32 >> (((7 - off) - pass) << 2)) & 0xF)
        | ((0x01145AF0u32 >> (((7 - off) - pass) << 2)) & 0xF0)
}
#[inline]
pub const fn png_row_in_interlace_pass(y: png_uint_32, pass: c_int) -> c_uint {
    (png_pass_mask(pass, 0) >> (y & 7)) & 1
}
#[inline]
pub const fn png_col_in_interlace_pass(x: png_uint_32, pass: c_int) -> c_uint {
    (png_pass_mask(pass, 1) >> (x & 7)) & 1
}

/// `png_composite` (NODIV variant, which is what this build uses)
#[inline]
pub fn png_composite(fg: png_uint_16, alpha: png_uint_16, bg: png_uint_16) -> png_byte {
    let temp: png_uint_16 = (fg)
        .wrapping_mul(alpha)
        .wrapping_add((bg).wrapping_mul(255u16.wrapping_sub(alpha)))
        .wrapping_add(128);
    (((temp as u32 + (temp as u32 >> 8)) >> 8) & 0xff) as png_byte
}

/// `png_composite_16` (NODIV variant)
#[inline]
pub fn png_composite_16(fg: png_uint_32, alpha: png_uint_32, bg: png_uint_32) -> png_uint_16 {
    let temp: png_uint_32 = (fg)
        .wrapping_mul(alpha)
        .wrapping_add((bg).wrapping_mul(65535u32.wrapping_sub(alpha)));
    let temp = temp.wrapping_add(32768);
    (0xffff & ((temp.wrapping_add(temp >> 16)) >> 16)) as png_uint_16
}
