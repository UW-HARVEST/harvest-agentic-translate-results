//! Bindings to the C runtime facilities that libpng uses directly, plus the
//! zlib API (libpng does not implement DEFLATE itself; the reference build
//! links the system zlib and so do we, which guarantees byte-identical
//! compressed output).

#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_uint, c_ulong, c_void};

/* ------------------------------------------------------------------ */
/* <stdio.h>                                                           */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct FILE {
    _opaque: [u8; 0],
}

extern "C" {
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fread(ptr: *mut c_void, size: usize, n: usize, f: *mut FILE) -> usize;
    pub fn fwrite(ptr: *const c_void, size: usize, n: usize, f: *mut FILE) -> usize;
    pub fn fflush(f: *mut FILE) -> c_int;
    pub fn ferror(f: *mut FILE) -> c_int;
    pub fn fputs(s: *const c_char, f: *mut FILE) -> c_int;
    pub static mut stderr: *mut FILE;
}

/// `fprintf(stderr, "<prefix>%s", msg); fprintf(stderr, "\n");`
pub unsafe fn png_stderr_message(prefix: *const c_char, msg: *const c_char) {
    fputs(prefix, stderr);
    fputs(msg, stderr);
    fputs(b"\n\0".as_ptr() as *const c_char, stderr);
}

/* ------------------------------------------------------------------ */
/* <stdlib.h> / <string.h>                                             */
/* ------------------------------------------------------------------ */

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;
    pub fn atof(s: *const c_char) -> f64;
    pub fn remove(path: *const c_char) -> c_int;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn __errno_location() -> *mut c_int;
    pub fn strtod(s: *const c_char, end: *mut *mut c_char) -> f64;
}

/* ------------------------------------------------------------------ */
/* <setjmp.h>                                                          */
/* ------------------------------------------------------------------ */

/// `jmp_buf` on x86-64 glibc: 200 bytes, 8 byte alignment.
pub type jmp_buf = [u64; 25];

extern "C" {
    #[link_name = "setjmp"]
    pub fn setjmp(env: *mut jmp_buf) -> c_int;
    #[link_name = "longjmp"]
    pub fn longjmp(env: *mut jmp_buf, val: c_int) -> !;
}

/* ------------------------------------------------------------------ */
/* <time.h>                                                            */
/* ------------------------------------------------------------------ */

pub type time_t = i64;

#[repr(C)]
#[derive(Clone, Copy)]
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
/* <math.h>                                                            */
/* ------------------------------------------------------------------ */

extern "C" {
    pub fn pow(x: f64, y: f64) -> f64;
    pub fn floor(x: f64) -> f64;
    pub fn ceil(x: f64) -> f64;
    pub fn fabs(x: f64) -> f64;
    pub fn log(x: f64) -> f64;
    pub fn exp(x: f64) -> f64;
    pub fn modf(x: f64, i: *mut f64) -> f64;
    pub fn frexp(x: f64, e: *mut c_int) -> f64;
}

pub const DBL_DIG: c_int = 15;
pub const DBL_MIN: f64 = 2.2250738585072014e-308;
pub const DBL_MAX: f64 = 1.7976931348623157e308;

/* ------------------------------------------------------------------ */
/* zlib                                                                */
/* ------------------------------------------------------------------ */

pub type uInt = c_uint;
pub type uLong = c_ulong;
pub type Bytef = u8;
pub type voidpf = *mut c_void;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf)>;

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

pub type z_streamp = *mut z_stream;

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
pub const Z_TEXT: c_int = 1;
pub const Z_UNKNOWN: c_int = 2;

pub const Z_DEFLATED: c_int = 8;

pub const ZLIB_VERNUM: c_int = 0x12b0;

pub const ZLIB_VERSION: &[u8] = b"1.2.11\0";

pub const MAX_WBITS: c_int = 15;
pub const MAX_MEM_LEVEL: c_int = 9;

/// `(uInt)-1`
pub const ZLIB_IO_MAX: uInt = u32::MAX;

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

/// `deflateInit2(strm, level, method, windowBits, memLevel, strategy)`
#[inline]
pub unsafe fn deflateInit2(
    strm: z_streamp,
    level: c_int,
    method: c_int,
    window_bits: c_int,
    mem_level: c_int,
    strategy: c_int,
) -> c_int {
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

/// `inflateInit2(strm, windowBits)`
#[inline]
pub unsafe fn inflateInit2(strm: z_streamp, window_bits: c_int) -> c_int {
    inflateInit2_(
        strm,
        window_bits,
        ZLIB_VERSION.as_ptr() as *const c_char,
        core::mem::size_of::<z_stream>() as c_int,
    )
}
