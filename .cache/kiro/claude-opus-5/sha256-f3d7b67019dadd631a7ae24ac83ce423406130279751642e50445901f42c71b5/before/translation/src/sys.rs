//! C library / system bindings used by libpng.
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_void};

/* ------------------------------------------------------------------ */
/* setjmp / longjmp                                                    */
/* ------------------------------------------------------------------ */

/// glibc x86-64 `jmp_buf`: 200 bytes, 8 byte alignment.
#[repr(C, align(8))]
#[derive(Copy, Clone)]
pub struct jmp_buf(pub [u64; 25]);

impl jmp_buf {
    pub const fn new() -> jmp_buf {
        jmp_buf([0u64; 25])
    }
}

extern "C" {
    #[link_name = "longjmp"]
    pub fn longjmp(env: *mut jmp_buf, val: c_int) -> !;
    #[link_name = "_setjmp"]
    pub fn setjmp(env: *mut jmp_buf) -> c_int;
    pub fn abort() -> !;
}

/* ------------------------------------------------------------------ */
/* stdlib / string                                                     */
/* ------------------------------------------------------------------ */

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtod(s: *const c_char, endp: *mut *mut c_char) -> c_double;
    pub fn __errno_location() -> *mut c_int;
}

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

/* ------------------------------------------------------------------ */
/* math                                                                */
/* ------------------------------------------------------------------ */

extern "C" {
    pub fn pow(x: c_double, y: c_double) -> c_double;
    pub fn frexp(x: c_double, e: *mut c_int) -> c_double;
    pub fn modf(x: c_double, ip: *mut c_double) -> c_double;
    pub fn floor(x: c_double) -> c_double;
    pub fn ceil(x: c_double) -> c_double;
}

/* DBL_DIG / DBL_MIN / DBL_MAX from <float.h> */
pub const DBL_DIG: c_int = 15;
pub const DBL_MIN: c_double = 2.2250738585072014e-308_f64;
pub const DBL_MAX: c_double = 1.7976931348623157e+308_f64;

/* ------------------------------------------------------------------ */
/* time                                                                */
/* ------------------------------------------------------------------ */

pub type time_t = c_long;

#[repr(C)]
#[derive(Copy, Clone)]
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

extern "C" {
    pub fn gmtime(t: *const time_t) -> *mut tm;
}

/* ------------------------------------------------------------------ */
/* stdio                                                              */
/* ------------------------------------------------------------------ */

pub type FILE = c_void;

extern "C" {
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fread(buf: *mut c_void, size: usize, n: usize, f: *mut FILE) -> usize;
    pub fn fwrite(buf: *const c_void, size: usize, n: usize, f: *mut FILE) -> usize;
    pub fn fflush(f: *mut FILE) -> c_int;
    pub fn ferror(f: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, f: *mut FILE) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn remove(path: *const c_char) -> c_int;
    pub static mut stderr: *mut FILE;
}

/* ------------------------------------------------------------------ */
/* zlib                                                               */
/* ------------------------------------------------------------------ */

pub type uInt = c_uint;
pub type uLong = core::ffi::c_ulong;
pub type Bytef = u8;
pub type voidpf = *mut c_void;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf)>;

#[repr(C)]
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

pub type z_streamp = *mut z_stream;

pub const Z_OK: c_int = 0;
pub const Z_STREAM_END: c_int = 1;
pub const Z_NEED_DICT: c_int = 2;
pub const Z_ERRNO: c_int = -1;
pub const Z_STREAM_ERROR: c_int = -2;
pub const Z_DATA_ERROR: c_int = -3;
pub const Z_MEM_ERROR: c_int = -4;
pub const Z_BUF_ERROR: c_int = -5;
pub const Z_VERSION_ERROR: c_int = -6;

pub const Z_NO_FLUSH: c_int = 0;
pub const Z_PARTIAL_FLUSH: c_int = 1;
pub const Z_SYNC_FLUSH: c_int = 2;
pub const Z_FULL_FLUSH: c_int = 3;
pub const Z_FINISH: c_int = 4;
pub const Z_BLOCK: c_int = 5;
pub const Z_TREES: c_int = 6;

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
pub const Z_TEXT: c_int = 1;
pub const Z_UNKNOWN: c_int = 2;

pub const Z_DEFLATED: c_int = 8;
pub const MAX_WBITS: c_int = 15;
pub const MAX_MEM_LEVEL: c_int = 9;

/// zlib version this build targets (`ZLIB_VERNUM` of the reference C build).
pub const ZLIB_VERNUM: c_int = 0x12b0;

pub const ZLIB_VERSION_STR: &[u8] = b"1.2.11\0";

#[link(name = "z")]
extern "C" {
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
    pub fn crc32(crc: uLong, buf: *const Bytef, len: uInt) -> uLong;
}

/// `deflateInit2()` as the C macro expands it.
#[inline]
pub unsafe fn deflateInit2(
    strm: z_streamp,
    level: c_int,
    method: c_int,
    windowBits: c_int,
    memLevel: c_int,
    strategy: c_int,
) -> c_int {
    deflateInit2_(
        strm,
        level,
        method,
        windowBits,
        memLevel,
        strategy,
        ZLIB_VERSION_STR.as_ptr() as *const c_char,
        core::mem::size_of::<z_stream>() as c_int,
    )
}

/// `inflateInit2()` as the C macro expands it.
#[inline]
pub unsafe fn inflateInit2(strm: z_streamp, windowBits: c_int) -> c_int {
    inflateInit2_(
        strm,
        windowBits,
        ZLIB_VERSION_STR.as_ptr() as *const c_char,
        core::mem::size_of::<z_stream>() as c_int,
    )
}
