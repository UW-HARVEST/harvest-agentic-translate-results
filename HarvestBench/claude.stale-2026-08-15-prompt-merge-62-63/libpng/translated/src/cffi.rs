//! C library and zlib FFI declarations used throughout the port.
//!
//! libpng relies on the C standard library (malloc/free/memcpy/…), libm
//! (pow/floor/…), stdio (FILE*), and the system zlib for DEFLATE/INFLATE.  We
//! link the same system zlib the reference C build uses so that compressed
//! output is byte-identical.
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_long, c_void};

pub type size_t = usize;

// ---- libc ----
extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: size_t) -> c_int;
    pub fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;
    pub fn strtod(s: *const c_char, endp: *mut *mut c_char) -> f64;
    pub fn atof(s: *const c_char) -> f64;
    pub fn abort() -> !;
    pub fn snprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;

    // stdio
    pub fn fread(ptr: *mut c_void, size: size_t, n: size_t, stream: *mut FILE) -> size_t;
    pub fn fwrite(ptr: *const c_void, size: size_t, n: size_t, stream: *mut FILE) -> size_t;
    pub fn fflush(stream: *mut FILE) -> c_int;
    pub fn fprintf(stream: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn fputs(s: *const c_char, stream: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, stream: *mut FILE) -> c_int;

    // time
    pub fn gmtime(t: *const time_t) -> *mut tm;

    // math
    pub fn pow(x: f64, y: f64) -> f64;
    pub fn floor(x: f64) -> f64;
    pub fn ceil(x: f64) -> f64;
    pub fn modf(x: f64, iptr: *mut f64) -> f64;
    pub fn frexp(x: f64, exp: *mut c_int) -> f64;
    pub fn fabs(x: f64) -> f64;
    pub fn log(x: f64) -> f64;
    pub fn exp(x: f64) -> f64;
    pub fn sqrt(x: f64) -> f64;

    // errno-less longjmp support: the caller supplies longjmp via
    // png_set_longjmp_fn, so we do not need setjmp here.
    pub fn longjmp(env: *mut c_void, val: c_int) -> !;
}

extern "C" {
    /// setjmp shim (see csupport/shim.c). Returns 0 on the direct call and the
    /// longjmp value when returning via longjmp.
    #[link_name = "png_rust_setjmp"]
    pub fn setjmp_shim(env: *mut crate::pstruct::jmp_buf) -> c_int;
}

// stderr accessor: `stderr` is a macro in C; use a helper symbol.
extern "C" {
    #[link_name = "stderr"]
    static mut STDERR: *mut FILE;
}

#[inline]
pub unsafe fn stderr() -> *mut FILE {
    STDERR
}

// Opaque FILE type.
#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

pub type time_t = c_long;

/// struct tm — layout as defined by glibc <time.h>.
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

// ---- zlib ----
// z_stream layout must match the system zlib's z_stream_s exactly, since a
// z_stream lives embedded in png_struct and is passed to zlib functions.
pub type uInt = core::ffi::c_uint;
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
    pub msg: *mut c_char,
    pub state: *mut c_void, // struct internal_state*
    pub zalloc: alloc_func,
    pub zfree: free_func,
    pub opaque: voidpf,
    pub data_type: c_int,
    pub adler: uLong,
    pub reserved: uLong,
}

pub type z_streamp = *mut z_stream;

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

pub const ZLIB_VERSION: &[u8] = b"1.2.11\0";
pub const ZLIB_VERNUM: c_int = 0x12b0;

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

#[inline]
pub unsafe fn inflateInit2(strm: z_streamp, window_bits: c_int) -> c_int {
    inflateInit2_(
        strm,
        window_bits,
        ZLIB_VERSION.as_ptr() as *const c_char,
        core::mem::size_of::<z_stream>() as c_int,
    )
}

// zlib return codes / flush values
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

pub const Z_DEFLATED: c_int = 8;
pub const Z_DEFAULT_STRATEGY: c_int = 0;
pub const Z_DEFAULT_COMPRESSION: c_int = -1;
