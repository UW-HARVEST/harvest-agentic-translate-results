//! Shared declarations: the Rust equivalent of pngconf.h, png.h (types and
//! constants), pnglibconf.h, pngstruct.h, pnginfo.h and pngpriv.h.
//!
//! Every translated module does `use crate::pngpriv::*;`.

pub use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};

/* ========================================================================= *
 *  Basic types (pngconf.h)
 * ========================================================================= */

pub type png_byte = u8;
pub type png_int_16 = i16;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_size_t = usize;
pub type png_alloc_size_t = usize;
pub type png_fixed_point = i32;
pub type png_double = f64;

pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_bytepp = *mut *mut png_byte;
pub type png_const_bytepp = *const *const png_byte;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_uint_16pp = *mut *mut png_uint_16;
pub type png_int_32p = *mut png_int_32;
pub type png_uint_32p = *mut png_uint_32;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_charpp = *mut *mut c_char;
pub type png_const_charpp = *const *const c_char;
pub type png_voidp = *mut c_void;
pub type png_const_voidp = *const c_void;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_doublep = *mut c_double;
pub type png_const_doublep = *const c_double;
pub type png_size_tp = *mut usize;
pub type png_alloc_size_tp = *mut usize;

pub const PNG_UINT_31_MAX: png_uint_32 = 0x7fffffff;
pub const PNG_UINT_32_MAX: png_uint_32 = u32::MAX;
pub const PNG_SIZE_MAX: usize = usize::MAX;

/* PNG_MAX_UINT_32 helpers */
pub const PNG_UINT_MAX: png_uint_32 = u32::MAX;

/* ========================================================================= *
 *  Public structures (png.h)
 * ========================================================================= */

#[repr(C)]
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub struct png_color {
    pub red: png_byte,
    pub green: png_byte,
    pub blue: png_byte,
}
pub type png_colorp = *mut png_color;
pub type png_const_colorp = *const png_color;
pub type png_colorpp = *mut *mut png_color;

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy)]
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
#[derive(Clone, Copy, Default)]
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

pub type png_struct = png_struct_def;
pub type png_structp = *mut png_struct;
pub type png_const_structp = *const png_struct;
pub type png_structpp = *mut *mut png_struct;
pub type png_structrp = *mut png_struct;
pub type png_const_structrp = *const png_struct;

pub type png_info = png_info_def;
pub type png_infop = *mut png_info;
pub type png_const_infop = *const png_info;
pub type png_infopp = *mut *mut png_info;
pub type png_inforp = *mut png_info;
pub type png_const_inforp = *const png_info;

/* ========================================================================= *
 *  Callback types
 * ========================================================================= */

pub type png_error_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_const_charp)>;
pub type png_rw_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, usize)>;
pub type png_flush_ptr = Option<unsafe extern "C-unwind" fn(png_structp)>;
pub type png_read_status_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_uint_32, c_int)>;
pub type png_write_status_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_uint_32, c_int)>;
pub type png_progressive_info_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>;
pub type png_progressive_end_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_infop)>;
pub type png_progressive_row_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_bytep, png_uint_32, c_int)>;
pub type png_user_transform_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_row_infop, png_bytep)>;
pub type png_user_chunk_ptr =
    Option<unsafe extern "C-unwind" fn(png_structp, png_unknown_chunkp) -> c_int>;
pub type png_malloc_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_alloc_size_t) -> png_voidp>;
pub type png_free_ptr = Option<unsafe extern "C-unwind" fn(png_structp, png_voidp)>;
pub type png_longjmp_ptr = Option<unsafe extern "C-unwind" fn(*mut jmp_buf, c_int) -> !>;
pub type png_read_filter_fn =
    Option<unsafe extern "C-unwind" fn(png_row_infop, png_bytep, png_const_bytep)>;

/* ========================================================================= *
 *  setjmp/longjmp
 * ========================================================================= */

/* glibc x86-64: sizeof(jmp_buf) == 200, _Alignof(jmp_buf) == 8. */
#[repr(C, align(8))]
#[derive(Clone, Copy)]
pub struct jmp_buf(pub [u8; 200]);

impl Default for jmp_buf {
    fn default() -> Self {
        jmp_buf([0u8; 200])
    }
}

/* ========================================================================= *
 *  C library imports
 * ========================================================================= */

pub type FILE = c_void;

unsafe extern "C" {
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(s1: *const c_void, s2: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    pub fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;

    pub fn fread(ptr: *mut c_void, size: usize, n: usize, stream: *mut FILE) -> usize;
    pub fn fwrite(ptr: *const c_void, size: usize, n: usize, stream: *mut FILE) -> usize;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(stream: *mut FILE) -> c_int;
    pub fn ferror(stream: *mut FILE) -> c_int;
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;

    pub fn floor(x: c_double) -> c_double;
    pub fn ceil(x: c_double) -> c_double;
    pub fn pow(x: c_double, y: c_double) -> c_double;
    pub fn log(x: c_double) -> c_double;
    pub fn log10(x: c_double) -> c_double;
    pub fn exp(x: c_double) -> c_double;
    pub fn modf(x: c_double, iptr: *mut c_double) -> c_double;
    pub fn frexp(x: c_double, e: *mut c_int) -> c_double;

    pub fn gmtime(t: *const i64) -> *mut tm;

    #[link_name = "stderr"]
    pub static mut c_stderr: *mut FILE;
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
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

/* ========================================================================= *
 *  zlib (the reference build links the system zlib; so do we, so that the
 *  DEFLATE bit streams are identical)
 * ========================================================================= */

pub type uInt = c_uint;
pub type uLong = c_ulong;
pub type Bytef = u8;
pub type voidpf = *mut c_void;
pub type alloc_func = Option<unsafe extern "C-unwind" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C-unwind" fn(voidpf, voidpf)>;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct z_stream {
    pub next_in: *const Bytef,
    pub avail_in: uInt,
    pub total_in: uLong,
    pub next_out: *mut Bytef,
    pub avail_out: uInt,
    pub total_out: uLong,
    pub msg: *const c_char,
    pub state: *mut c_void,
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: c_int,
    pub adler: uLong,
    pub reserved: uLong,
}

impl Default for z_stream {
    fn default() -> Self {
        unsafe { core::mem::zeroed() }
    }
}

pub type z_streamp = *mut z_stream;

pub const ZLIB_VERSION: &[u8] = b"1.2.11\0";
/* Only used to select code paths; the real values come from the linked zlib. */
pub const ZLIB_VERNUM: u32 = 0x12b0;

pub const Z_NO_FLUSH: c_int = 0;
pub const Z_PARTIAL_FLUSH: c_int = 1;
pub const Z_SYNC_FLUSH: c_int = 2;
pub const Z_FULL_FLUSH: c_int = 3;
pub const Z_FINISH: c_int = 4;
pub const Z_BLOCK: c_int = 5;
pub const Z_TREES: c_int = 6;

pub const Z_OK: c_int = 0;
pub const Z_STREAM_END: c_int = 1;
pub const Z_NEED_DICT: c_int = 2;
pub const Z_ERRNO: c_int = -1;
pub const Z_STREAM_ERROR: c_int = -2;
pub const Z_DATA_ERROR: c_int = -3;
pub const Z_MEM_ERROR: c_int = -4;
pub const Z_BUF_ERROR: c_int = -5;
pub const Z_VERSION_ERROR: c_int = -6;

pub const Z_NO_COMPRESSION: c_int = 0;
pub const Z_BEST_SPEED: c_int = 1;
pub const Z_BEST_COMPRESSION: c_int = 9;
pub const Z_DEFAULT_COMPRESSION: c_int = -1;

pub const Z_FILTERED: c_int = 1;
pub const Z_HUFFMAN_ONLY: c_int = 2;
pub const Z_RLE: c_int = 3;
pub const Z_FIXED: c_int = 4;
pub const Z_DEFAULT_STRATEGY: c_int = 0;

pub const Z_BINARY: c_int = 0;
pub const Z_DEFLATED: c_int = 8;

pub const Z_NULL: usize = 0;

unsafe extern "C-unwind" {
    pub fn deflateInit2_(
        strm: z_streamp,
        level: c_int,
        method: c_int,
        windowBits: c_int,
        memLevel: c_int,
        strategy: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    pub fn deflate(strm: z_streamp, flush: c_int) -> c_int;
    pub fn deflateEnd(strm: z_streamp) -> c_int;
    pub fn deflateReset(strm: z_streamp) -> c_int;
    pub fn deflateBound(strm: z_streamp, sourceLen: uLong) -> uLong;

    pub fn inflateInit2_(
        strm: z_streamp,
        windowBits: c_int,
        version: *const c_char,
        stream_size: c_int,
    ) -> c_int;
    pub fn inflate(strm: z_streamp, flush: c_int) -> c_int;
    pub fn inflateEnd(strm: z_streamp) -> c_int;
    pub fn inflateReset(strm: z_streamp) -> c_int;
    pub fn inflateReset2(strm: z_streamp, windowBits: c_int) -> c_int;
    pub fn inflateValidate(strm: z_streamp, check: c_int) -> c_int;
    pub fn crc32(crc: uLong, buf: *const Bytef, len: uInt) -> uLong;
    pub fn zlibVersion() -> *const c_char;
    pub fn zError(err: c_int) -> *const c_char;
}

#[inline]
pub unsafe fn deflateInit2(
    strm: z_streamp,
    level: c_int,
    method: c_int,
    window_bits: c_int,
    mem_level: c_int,
    strategy: c_int,
) -> c_int {
    unsafe {
        deflateInit2_(
            strm,
            level,
            method,
            window_bits,
            mem_level,
            strategy,
            ZLIB_VERSION.as_ptr() as *const c_char,
            core::mem::size_of::<z_stream>() as c_int,
        )
    }
}

#[inline]
pub unsafe fn inflateInit2(strm: z_streamp, window_bits: c_int) -> c_int {
    unsafe {
        inflateInit2_(
            strm,
            window_bits,
            ZLIB_VERSION.as_ptr() as *const c_char,
            core::mem::size_of::<z_stream>() as c_int,
        )
    }
}

#[inline]
pub unsafe fn inflateInit(strm: z_streamp) -> c_int {
    unsafe {
        inflateInit2_(
            strm,
            15,
            ZLIB_VERSION.as_ptr() as *const c_char,
            core::mem::size_of::<z_stream>() as c_int,
        )
    }
}

/* ========================================================================= *
 *  pnglibconf.h settings
 * ========================================================================= */

pub const PNG_ZBUF_SIZE: usize = 8192;
pub const PNG_IDAT_READ_SIZE: usize = PNG_ZBUF_SIZE;
pub const PNG_INFLATE_BUF_SIZE: usize = 1024;
pub const PNG_GAMMA_THRESHOLD_FIXED: png_fixed_point = 5000;
pub const PNG_MAX_GAMMA_8: c_int = 11;
pub const PNG_QUANTIZE_RED_BITS: c_int = 5;
pub const PNG_QUANTIZE_GREEN_BITS: c_int = 5;
pub const PNG_QUANTIZE_BLUE_BITS: c_int = 5;
pub const PNG_TEXT_Z_DEFAULT_COMPRESSION: c_int = -1;
pub const PNG_TEXT_Z_DEFAULT_STRATEGY: c_int = 0;
pub const PNG_USER_CHUNK_CACHE_MAX: png_uint_32 = 1000;
pub const PNG_USER_CHUNK_MALLOC_MAX: png_alloc_size_t = 8000000;
pub const PNG_USER_WIDTH_MAX: png_uint_32 = 1000000;
pub const PNG_USER_HEIGHT_MAX: png_uint_32 = 1000000;
pub const PNG_Z_DEFAULT_COMPRESSION: c_int = -1;
pub const PNG_Z_DEFAULT_NOFILTER_STRATEGY: c_int = 0;
pub const PNG_Z_DEFAULT_STRATEGY: c_int = 1;
pub const PNG_sCAL_PRECISION: c_int = 5;
pub const PNG_sRGB_PROFILE_CHECKS: c_int = 2;
pub const PNG_API_RULE: c_int = 0;
pub const PNG_DEFAULT_READ_MACROS: c_int = 1;

pub const Z_DEFAULT_NOFILTER_STRATEGY: c_int = PNG_Z_DEFAULT_NOFILTER_STRATEGY;

pub const ZLIB_IO_MAX: uInt = uInt::MAX;

/* ========================================================================= *
 *  png.h constants
 * ========================================================================= */

pub const PNG_LIBPNG_VER_STRING: &[u8] = b"1.6.59.git\0";
pub const PNG_HEADER_VERSION_STRING: &[u8] = b" libpng version 1.6.59.git\n\0";
pub const PNG_LIBPNG_VER_SONUM: c_int = 16;
pub const PNG_LIBPNG_VER_DLLNUM: c_int = 16;
pub const PNG_LIBPNG_VER_MAJOR: c_int = 1;
pub const PNG_LIBPNG_VER_MINOR: c_int = 6;
pub const PNG_LIBPNG_VER_RELEASE: c_int = 59;
pub const PNG_LIBPNG_VER_BUILD: c_int = 1;
pub const PNG_LIBPNG_BUILD_ALPHA: c_int = 1;
pub const PNG_LIBPNG_BUILD_BETA: c_int = 2;
pub const PNG_LIBPNG_BUILD_RC: c_int = 3;
pub const PNG_LIBPNG_BUILD_STABLE: c_int = 4;
pub const PNG_LIBPNG_BUILD_RELEASE_STATUS_MASK: c_int = 7;
pub const PNG_LIBPNG_BUILD_PATCH: c_int = 8;
pub const PNG_LIBPNG_BUILD_PRIVATE: c_int = 16;
pub const PNG_LIBPNG_BUILD_SPECIAL: c_int = 32;
pub const PNG_LIBPNG_BUILD_BASE_TYPE: c_int = PNG_LIBPNG_BUILD_BETA;
pub const PNG_LIBPNG_VER: png_uint_32 = 10659;

pub const PNG_TEXT_COMPRESSION_NONE_WR: c_int = -3;
pub const PNG_TEXT_COMPRESSION_zTXt_WR: c_int = -2;
pub const PNG_TEXT_COMPRESSION_NONE: c_int = -1;
pub const PNG_TEXT_COMPRESSION_zTXt: c_int = 0;
pub const PNG_ITXT_COMPRESSION_NONE: c_int = 1;
pub const PNG_ITXT_COMPRESSION_zTXt: c_int = 2;
pub const PNG_TEXT_COMPRESSION_LAST: c_int = 3;

pub const PNG_HAVE_IHDR: png_uint_32 = 0x01;
pub const PNG_HAVE_PLTE: png_uint_32 = 0x02;
pub const PNG_AFTER_IDAT: png_uint_32 = 0x08;

pub const PNG_FP_1: png_fixed_point = 100000;
pub const PNG_FP_HALF: png_fixed_point = 50000;
pub const PNG_FP_MAX: png_fixed_point = png_fixed_point::MAX;
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
pub const PNG_ALL_FILTERS: c_int =
    PNG_FAST_FILTERS | PNG_FILTER_AVG | PNG_FILTER_PAETH;

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

pub const PNG_INTERLACE_ADAM7_PASSES: usize = 7;

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
pub const PNG_FORMAT_LINEAR_RGB: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR;
pub const PNG_FORMAT_LINEAR_RGB_ALPHA: png_uint_32 =
    PNG_FORMAT_FLAG_LINEAR | PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA;
pub const PNG_FORMAT_RGB_COLORMAP: png_uint_32 =
    PNG_FORMAT_RGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGR_COLORMAP: png_uint_32 =
    PNG_FORMAT_BGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_RGBA_COLORMAP: png_uint_32 =
    PNG_FORMAT_RGBA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ARGB_COLORMAP: png_uint_32 =
    PNG_FORMAT_ARGB | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_BGRA_COLORMAP: png_uint_32 =
    PNG_FORMAT_BGRA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_ABGR_COLORMAP: png_uint_32 =
    PNG_FORMAT_ABGR | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_GA_COLORMAP: png_uint_32 = PNG_FORMAT_GA | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_AG_COLORMAP: png_uint_32 = PNG_FORMAT_AG | PNG_FORMAT_FLAG_COLORMAP;
pub const PNG_FORMAT_GRAY_COLORMAP: png_uint_32 =
    PNG_FORMAT_GRAY | PNG_FORMAT_FLAG_COLORMAP;

pub const PNG_IMAGE_FLAG_COLORSPACE_NOT_sRGB: png_uint_32 = 0x01;
pub const PNG_IMAGE_FLAG_FAST: png_uint_32 = 0x02;
pub const PNG_IMAGE_FLAG_16BIT_sRGB: png_uint_32 = 0x04;

pub const PNG_ARM_NEON: c_int = 0;
pub const PNG_MAXIMUM_INFLATE_WINDOW: c_int = 2;
pub const PNG_SKIP_sRGB_CHECK_PROFILE: c_int = 4;
pub const PNG_MIPS_MSA: c_int = 6;
pub const PNG_IGNORE_ADLER32: c_int = 8;
pub const PNG_POWERPC_VSX: c_int = 10;
pub const PNG_MIPS_MMI: c_int = 12;
pub const PNG_RISCV_RVV: c_int = 14;
pub const PNG_OPTION_NEXT: c_int = 16;

pub const PNG_OPTION_UNSET: c_int = 0;
pub const PNG_OPTION_INVALID: c_int = 1;
pub const PNG_OPTION_OFF: c_int = 2;
pub const PNG_OPTION_ON: c_int = 3;

/* ========================================================================= *
 *  pngpriv.h constants
 * ========================================================================= */

/* mode flags */
pub const PNG_HAVE_IDAT: png_uint_32 = 0x04;
pub const PNG_HAVE_IEND: png_uint_32 = 0x10;
pub const PNG_HAVE_CHUNK_HEADER: png_uint_32 = 0x100;
pub const PNG_WROTE_tIME: png_uint_32 = 0x200;
pub const PNG_WROTE_INFO_BEFORE_PLTE: png_uint_32 = 0x400;
pub const PNG_BACKGROUND_IS_GRAY: png_uint_32 = 0x800;
pub const PNG_HAVE_PNG_SIGNATURE: png_uint_32 = 0x1000;
pub const PNG_HAVE_CHUNK_AFTER_IDAT: png_uint_32 = 0x2000;
pub const PNG_WROTE_eXIf: png_uint_32 = 0x4000;
pub const PNG_IS_READ_STRUCT: png_uint_32 = 0x8000;

/* transformations */
pub const PNG_BGR: png_uint_32 = 0x0001;
pub const PNG_INTERLACE: png_uint_32 = 0x0002;
pub const PNG_PACK: png_uint_32 = 0x0004;
pub const PNG_SHIFT: png_uint_32 = 0x0008;
pub const PNG_SWAP_BYTES: png_uint_32 = 0x0010;
pub const PNG_INVERT_MONO: png_uint_32 = 0x0020;
pub const PNG_QUANTIZE: png_uint_32 = 0x0040;
pub const PNG_COMPOSE: png_uint_32 = 0x0080;
pub const PNG_BACKGROUND_EXPAND: png_uint_32 = 0x0100;
pub const PNG_EXPAND_16: png_uint_32 = 0x0200;
pub const PNG_16_TO_8: png_uint_32 = 0x0400;
pub const PNG_RGBA: png_uint_32 = 0x0800;
pub const PNG_EXPAND: png_uint_32 = 0x1000;
pub const PNG_GAMMA: png_uint_32 = 0x2000;
pub const PNG_GRAY_TO_RGB: png_uint_32 = 0x4000;
pub const PNG_FILLER: png_uint_32 = 0x8000;
pub const PNG_PACKSWAP: png_uint_32 = 0x10000;
pub const PNG_SWAP_ALPHA: png_uint_32 = 0x20000;
pub const PNG_STRIP_ALPHA: png_uint_32 = 0x40000;
pub const PNG_INVERT_ALPHA: png_uint_32 = 0x80000;
pub const PNG_USER_TRANSFORM: png_uint_32 = 0x100000;
pub const PNG_RGB_TO_GRAY_ERR: png_uint_32 = 0x200000;
pub const PNG_RGB_TO_GRAY_WARN: png_uint_32 = 0x400000;
pub const PNG_RGB_TO_GRAY: png_uint_32 = 0x600000;
pub const PNG_ENCODE_ALPHA: png_uint_32 = 0x800000;
pub const PNG_ADD_ALPHA: png_uint_32 = 0x1000000;
pub const PNG_EXPAND_tRNS: png_uint_32 = 0x2000000;
pub const PNG_SCALE_16_TO_8: png_uint_32 = 0x4000000;

/* free_me / struct flags */
pub const PNG_STRUCT_PNG: png_uint_32 = 0x0001;
pub const PNG_STRUCT_INFO: png_uint_32 = 0x0002;

/* png_ptr->flags */
pub const PNG_FLAG_ZLIB_CUSTOM_STRATEGY: png_uint_32 = 0x0001;
pub const PNG_FLAG_ZSTREAM_INITIALIZED: png_uint_32 = 0x0002;
pub const PNG_FLAG_ZSTREAM_ENDED: png_uint_32 = 0x0008;
pub const PNG_FLAG_ROW_INIT: png_uint_32 = 0x0040;
pub const PNG_FLAG_FILLER_AFTER: png_uint_32 = 0x0080;
pub const PNG_FLAG_CRC_ANCILLARY_USE: png_uint_32 = 0x0100;
pub const PNG_FLAG_CRC_ANCILLARY_NOWARN: png_uint_32 = 0x0200;
pub const PNG_FLAG_CRC_CRITICAL_USE: png_uint_32 = 0x0400;
pub const PNG_FLAG_CRC_CRITICAL_IGNORE: png_uint_32 = 0x0800;
pub const PNG_FLAG_OPTIMIZE_ALPHA: png_uint_32 = 0x2000;
pub const PNG_FLAG_DETECT_UNINITIALIZED: png_uint_32 = 0x4000;
pub const PNG_FLAG_LIBRARY_MISMATCH: png_uint_32 = 0x20000;
pub const PNG_FLAG_STRIP_ERROR_NUMBERS: png_uint_32 = 0x40000;
pub const PNG_FLAG_STRIP_ERROR_TEXT: png_uint_32 = 0x80000;
pub const PNG_FLAG_BENIGN_ERRORS_WARN: png_uint_32 = 0x100000;
pub const PNG_FLAG_APP_WARNINGS_WARN: png_uint_32 = 0x200000;
pub const PNG_FLAG_APP_ERRORS_WARN: png_uint_32 = 0x400000;

pub const PNG_FLAG_CRC_ANCILLARY_MASK: png_uint_32 =
    PNG_FLAG_CRC_ANCILLARY_USE | PNG_FLAG_CRC_ANCILLARY_NOWARN;
pub const PNG_FLAG_CRC_CRITICAL_MASK: png_uint_32 =
    PNG_FLAG_CRC_CRITICAL_USE | PNG_FLAG_CRC_CRITICAL_IGNORE;
pub const PNG_FLAG_CRC_MASK: png_uint_32 =
    PNG_FLAG_CRC_ANCILLARY_MASK | PNG_FLAG_CRC_CRITICAL_MASK;

pub const PNG_GAMMA_MAC_OLD: png_fixed_point = 151724;
pub const PNG_GAMMA_MAC_INVERSE: png_fixed_point = 65909;
pub const PNG_GAMMA_sRGB_INVERSE: png_fixed_point = 45455;
pub const PNG_LIB_GAMMA_MIN: png_fixed_point = 1000;
pub const PNG_LIB_GAMMA_MAX: png_fixed_point = 10000000;

pub const PNG_UNEXPECTED_ZLIB_RETURN: c_int = -7;

pub const PNG_CHUNK_WARNING: c_int = 0;
pub const PNG_CHUNK_WRITE_ERROR: c_int = 1;
pub const PNG_CHUNK_ERROR: c_int = 2;

pub const PNG_NUMBER_BUFFER_SIZE: usize = 24;
pub const PNG_NUMBER_FORMAT_u: c_int = 1;
pub const PNG_NUMBER_FORMAT_02u: c_int = 2;
pub const PNG_NUMBER_FORMAT_d: c_int = 1;
pub const PNG_NUMBER_FORMAT_02d: c_int = 2;
pub const PNG_NUMBER_FORMAT_x: c_int = 3;
pub const PNG_NUMBER_FORMAT_02x: c_int = 4;
pub const PNG_NUMBER_FORMAT_fixed: c_int = 5;

pub const PNG_WARNING_PARAMETER_SIZE: usize = 32;
pub const PNG_WARNING_PARAMETER_COUNT: usize = 8;
pub type png_warning_parameters = [[c_char; PNG_WARNING_PARAMETER_SIZE]; PNG_WARNING_PARAMETER_COUNT];

pub const PNG_sCAL_MAX_DIGITS: usize = (PNG_sCAL_PRECISION as usize) + 1 + 1 + 10;

pub const PNG_FP_INTEGER: c_int = 0;
pub const PNG_FP_FRACTION: c_int = 1;
pub const PNG_FP_EXPONENT: c_int = 2;
pub const PNG_FP_STATE: c_int = 3;
pub const PNG_FP_SAW_SIGN: c_int = 4;
pub const PNG_FP_SAW_DIGIT: c_int = 8;
pub const PNG_FP_SAW_DOT: c_int = 16;
pub const PNG_FP_SAW_E: c_int = 32;
pub const PNG_FP_SAW_ANY: c_int = 60;
pub const PNG_FP_WAS_VALID: c_int = 64;
pub const PNG_FP_NEGATIVE: c_int = 128;
pub const PNG_FP_NONZERO: c_int = 256;
pub const PNG_FP_STICKY: c_int = 448;
pub const PNG_FP_INVALID: c_int = 512;
pub const PNG_FP_MAYBE: c_int = 0;
pub const PNG_FP_OK: c_int = 1;
pub const PNG_FP_NZ_MASK: c_int = PNG_FP_SAW_DIGIT | PNG_FP_NEGATIVE | PNG_FP_NONZERO;
pub const PNG_FP_Z_MASK: c_int = PNG_FP_SAW_DIGIT | PNG_FP_NONZERO;

#[inline]
pub fn PNG_FP_IS_ZERO(state: c_int) -> bool {
    (state & PNG_FP_Z_MASK) == PNG_FP_SAW_DIGIT
}
#[inline]
pub fn PNG_FP_IS_POSITIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_Z_MASK
}
#[inline]
pub fn PNG_FP_IS_NEGATIVE(state: c_int) -> bool {
    (state & PNG_FP_NZ_MASK) == PNG_FP_NZ_MASK
}

/* ========================================================================= *
 *  Chunk names
 * ========================================================================= */

#[inline]
pub const fn PNG_U32(b1: u32, b2: u32, b3: u32, b4: u32) -> png_uint_32 {
    (b1 << 24) | (b2 << 16) | (b3 << 8) | b4
}

pub const png_IDAT: png_uint_32 = PNG_U32(73, 68, 65, 84);
pub const png_IEND: png_uint_32 = PNG_U32(73, 69, 78, 68);
pub const png_IHDR: png_uint_32 = PNG_U32(73, 72, 68, 82);
pub const png_PLTE: png_uint_32 = PNG_U32(80, 76, 84, 69);
pub const png_acTL: png_uint_32 = PNG_U32(97, 99, 84, 76);
pub const png_bKGD: png_uint_32 = PNG_U32(98, 75, 71, 68);
pub const png_cHRM: png_uint_32 = PNG_U32(99, 72, 82, 77);
pub const png_cICP: png_uint_32 = PNG_U32(99, 73, 67, 80);
pub const png_cLLI: png_uint_32 = PNG_U32(99, 76, 76, 73);
pub const png_eXIf: png_uint_32 = PNG_U32(101, 88, 73, 102);
pub const png_fcTL: png_uint_32 = PNG_U32(102, 99, 84, 76);
pub const png_fdAT: png_uint_32 = PNG_U32(102, 100, 65, 84);
pub const png_fRAc: png_uint_32 = PNG_U32(102, 82, 65, 99);
pub const png_gAMA: png_uint_32 = PNG_U32(103, 65, 77, 65);
pub const png_gIFg: png_uint_32 = PNG_U32(103, 73, 70, 103);
pub const png_gIFt: png_uint_32 = PNG_U32(103, 73, 70, 116);
pub const png_gIFx: png_uint_32 = PNG_U32(103, 73, 70, 120);
pub const png_hIST: png_uint_32 = PNG_U32(104, 73, 83, 84);
pub const png_iCCP: png_uint_32 = PNG_U32(105, 67, 67, 80);
pub const png_iTXt: png_uint_32 = PNG_U32(105, 84, 88, 116);
pub const png_mDCV: png_uint_32 = PNG_U32(109, 68, 67, 86);
pub const png_oFFs: png_uint_32 = PNG_U32(111, 70, 70, 115);
pub const png_pCAL: png_uint_32 = PNG_U32(112, 67, 65, 76);
pub const png_pHYs: png_uint_32 = PNG_U32(112, 72, 89, 115);
pub const png_sBIT: png_uint_32 = PNG_U32(115, 66, 73, 84);
pub const png_sCAL: png_uint_32 = PNG_U32(115, 67, 65, 76);
pub const png_sPLT: png_uint_32 = PNG_U32(115, 80, 76, 84);
pub const png_sRGB: png_uint_32 = PNG_U32(115, 82, 71, 66);
pub const png_sTER: png_uint_32 = PNG_U32(115, 84, 69, 82);
pub const png_tEXt: png_uint_32 = PNG_U32(116, 69, 88, 116);
pub const png_tIME: png_uint_32 = PNG_U32(116, 73, 77, 69);
pub const png_tRNS: png_uint_32 = PNG_U32(116, 82, 78, 83);
pub const png_zTXt: png_uint_32 = PNG_U32(122, 84, 88, 116);

/* png_index enum values (PNG_KNOWN_CHUNKS) */
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

#[inline]
pub unsafe fn png_file_has_chunk(png_ptr: png_const_structrp, i: u32) -> bool {
    unsafe { ((*png_ptr).chunks & png_chunk_flag_from_index(i)) != 0 }
}

#[inline]
pub unsafe fn png_file_add_chunk(png_ptr: png_structrp, i: u32) {
    unsafe {
        (*png_ptr).chunks |= png_chunk_flag_from_index(i);
    }
}

#[inline]
pub fn PNG_CHUNK_FROM_STRING(s: *const c_char) -> png_uint_32 {
    unsafe {
        PNG_U32(
            (*s.add(0) as u8) as u32,
            (*s.add(1) as u8) as u32,
            (*s.add(2) as u8) as u32,
            (*s.add(3) as u8) as u32,
        )
    }
}

#[inline]
pub unsafe fn PNG_STRING_FROM_CHUNK(s: *mut c_char, c: png_uint_32) {
    unsafe {
        *s.add(0) = ((c >> 24) & 0xff) as u8 as c_char;
        *s.add(1) = ((c >> 16) & 0xff) as u8 as c_char;
        *s.add(2) = ((c >> 8) & 0xff) as u8 as c_char;
        *s.add(3) = (c & 0xff) as u8 as c_char;
    }
}

#[inline]
pub unsafe fn PNG_CSTRING_FROM_CHUNK(s: *mut c_char, c: png_uint_32) {
    unsafe {
        PNG_STRING_FROM_CHUNK(s, c);
        *s.add(4) = 0;
    }
}

#[inline]
pub const fn PNG_CHUNK_ANCILLARY(c: png_uint_32) -> c_int {
    (1 & (c >> 29)) as c_int
}
#[inline]
pub const fn PNG_CHUNK_CRITICAL(c: png_uint_32) -> c_int {
    (PNG_CHUNK_ANCILLARY(c) == 0) as c_int
}
#[inline]
pub const fn PNG_CHUNK_PRIVATE(c: png_uint_32) -> c_int {
    (1 & (c >> 21)) as c_int
}
#[inline]
pub const fn PNG_CHUNK_RESERVED(c: png_uint_32) -> c_int {
    (1 & (c >> 13)) as c_int
}
#[inline]
pub const fn PNG_CHUNK_SAFE_TO_COPY(c: png_uint_32) -> c_int {
    (1 & (c >> 5)) as c_int
}

#[inline]
pub const fn PNG_CN_VALID_UPPER(b: u32) -> bool {
    b >= 65 && b <= 90
}
#[inline]
pub const fn PNG_CN_VALID_ASCII(b: u32) -> bool {
    PNG_CN_VALID_UPPER(b & !32u32)
}
#[inline]
pub const fn PNG_32to8(cn: png_uint_32, s: u32) -> u32 {
    (cn >> s) & 0xff
}
#[inline]
pub const fn PNG_CHUNK_NAME_VALID(cn: png_uint_32) -> bool {
    PNG_CN_VALID_ASCII(PNG_32to8(cn, 24))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 16))
        && PNG_CN_VALID_UPPER(PNG_32to8(cn, 8))
        && PNG_CN_VALID_ASCII(PNG_32to8(cn, 0))
}

/* ========================================================================= *
 *  Small helper macros translated as inline functions
 * ========================================================================= */

#[inline]
pub const fn PNG_ROWBYTES(pixel_bits: usize, width: usize) -> usize {
    if pixel_bits >= 8 {
        width * (pixel_bits >> 3)
    } else {
        ((width * pixel_bits) + 7) >> 3
    }
}

#[inline]
pub const fn PNG_TRAILBITS(pixel_bits: u32, width: u32) -> u32 {
    (pixel_bits * (width % 8)) % 8
}

#[inline]
pub const fn PNG_PADBITS(pixel_bits: u32, width: u32) -> u32 {
    (8 - PNG_TRAILBITS(pixel_bits, width)) % 8
}

#[inline]
pub const fn PNG_DIV65535(v24: png_uint_32) -> png_uint_32 {
    (v24 + 32895) >> 16
}

#[inline]
pub const fn PNG_DIV257(v16: png_uint_32) -> png_uint_32 {
    PNG_DIV65535(v16 * 255)
}

#[inline]
pub fn PNG_OUT_OF_RANGE(value: png_fixed_point, ideal: png_fixed_point, delta: png_fixed_point) -> bool {
    value < ideal - delta || value > ideal + delta
}

#[inline]
pub fn PNG_COLOR_DIST(c1: png_color, c2: png_color) -> c_int {
    (c1.red as c_int - c2.red as c_int).abs()
        + (c1.green as c_int - c2.green as c_int).abs()
        + (c1.blue as c_int - c2.blue as c_int).abs()
}

#[inline]
pub fn png_float(_png_ptr: png_const_structrp, fixed: png_fixed_point, _s: png_const_charp) -> f64 {
    0.00001f64 * (fixed as f64)
}

/* ========================================================================= *
 *  Colorspace helper structures (pngstruct.h)
 * ========================================================================= */

#[repr(C)]
#[derive(Clone, Copy, Default)]
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
#[derive(Clone, Copy, Default)]
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

#[repr(C)]
pub struct png_compression_buffer {
    pub next: *mut png_compression_buffer,
    pub output: [png_byte; 1],
}
pub type png_compression_bufferp = *mut png_compression_buffer;

/* offsetof(png_compression_buffer, output) + zbuffer_size */
#[inline]
pub unsafe fn PNG_COMPRESSION_BUFFER_SIZE(pp: png_const_structrp) -> usize {
    unsafe { core::mem::size_of::<*mut png_compression_buffer>() + (*pp).zbuffer_size as usize }
}

/* ========================================================================= *
 *  png_handle_result_code
 * ========================================================================= */

pub type png_handle_result_code = c_int;
pub const handled_error: c_int = 0;
pub const handled_discarded: c_int = 1;
pub const handled_saved: c_int = 2;
pub const handled_ok: c_int = 3;

/* ========================================================================= *
 *  png_struct
 * ========================================================================= */

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

    pub read_filter: [png_read_filter_fn; (PNG_FILTER_VALUE_LAST - 1) as usize],
}

/* ========================================================================= *
 *  png_info
 * ========================================================================= */

#[repr(C)]
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

/* ========================================================================= *
 *  png_image (simplified API)
 * ========================================================================= */

#[repr(C)]
pub struct png_control {
    pub png_ptr: png_structp,
    pub info_ptr: png_infop,
    pub error_buf: png_voidp,
    pub memory: png_const_bytep,
    pub size: usize,
    /* unsigned int for_write:1; unsigned int owned_file:1; */
    pub bitfields: c_uint,
}
pub type png_controlp = *mut png_control;

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

/* PNG_IMAGE_* helper macros used by the simplified API implementation. */
#[inline]
pub const fn PNG_IMAGE_SAMPLE_CHANNELS(fmt: png_uint_32) -> png_uint_32 {
    (fmt & (PNG_FORMAT_FLAG_COLOR | PNG_FORMAT_FLAG_ALPHA)) + 1
}
#[inline]
pub const fn PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_LINEAR) != 0 {
        2
    } else {
        1
    }
}
#[inline]
pub const fn PNG_IMAGE_SAMPLE_SIZE(fmt: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_CHANNELS(fmt) * PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)
}
#[inline]
pub const fn PNG_IMAGE_MAXIMUM_COLORMAP_COMPONENTS(fmt: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_CHANNELS(fmt) * 256
}
#[inline]
pub const fn PNG_IMAGE_PIXEL_(test: png_uint_32, fmt: png_uint_32) -> png_uint_32 {
    /* Emulates: ((fmt)&PNG_FORMAT_FLAG_COLORMAP)?1:test(fmt) */
    let _ = test;
    let _ = fmt;
    0
}
#[inline]
pub const fn PNG_IMAGE_PIXEL_CHANNELS(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_CHANNELS(fmt)
    }
}
#[inline]
pub const fn PNG_IMAGE_PIXEL_COMPONENT_SIZE(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_COMPONENT_SIZE(fmt)
    }
}
#[inline]
pub const fn PNG_IMAGE_PIXEL_SIZE(fmt: png_uint_32) -> png_uint_32 {
    if (fmt & PNG_FORMAT_FLAG_COLORMAP) != 0 {
        1
    } else {
        PNG_IMAGE_SAMPLE_SIZE(fmt)
    }
}
#[inline]
pub unsafe fn PNG_IMAGE_ROW_STRIDE(image: &png_image) -> png_uint_32 {
    PNG_IMAGE_PIXEL_CHANNELS(image.format) * image.width
}
#[inline]
pub const fn PNG_IMAGE_COLORMAP_SIZE(fmt: png_uint_32) -> png_uint_32 {
    PNG_IMAGE_SAMPLE_SIZE(fmt) * 256
}

/* ========================================================================= *
 *  Debug macros: no-ops, exactly as in the reference build
 * ========================================================================= */

#[macro_export]
macro_rules! png_debug {
    ($($arg:tt)*) => {};
}
#[macro_export]
macro_rules! png_debug1 {
    ($($arg:tt)*) => {};
}
#[macro_export]
macro_rules! png_debug2 {
    ($($arg:tt)*) => {};
}

/// Equivalent of C's `(void)param;`
#[inline(always)]
pub fn PNG_UNUSED<T>(_v: T) {}
