//! Basic C types, libc bindings, zlib bindings and setjmp/longjmp support.
//!
//! This mirrors pngconf.h plus the parts of the C library and of zlib.h that
//! libpng uses.

pub use core::ffi::{c_char, c_double, c_float, c_int, c_long, c_uchar, c_uint, c_ulong, c_void};

/* ---------------------------------------------------------------- pngconf.h */

pub type png_byte = u8;
pub type png_int_16 = i16;
pub type png_uint_16 = u16;
pub type png_int_32 = i32;
pub type png_uint_32 = u32;
pub type png_size_t = usize;
pub type png_ptrdiff_t = isize;
pub type png_alloc_size_t = usize;
pub type png_fixed_point = png_int_32;

pub type png_voidp = *mut c_void;
pub type png_const_voidp = *const c_void;
pub type png_bytep = *mut png_byte;
pub type png_const_bytep = *const png_byte;
pub type png_uint_32p = *mut png_uint_32;
pub type png_const_uint_32p = *const png_uint_32;
pub type png_int_32p = *mut png_int_32;
pub type png_const_int_32p = *const png_int_32;
pub type png_uint_16p = *mut png_uint_16;
pub type png_const_uint_16p = *const png_uint_16;
pub type png_int_16p = *mut png_int_16;
pub type png_const_int_16p = *const png_int_16;
pub type png_charp = *mut c_char;
pub type png_const_charp = *const c_char;
pub type png_fixed_point_p = *mut png_fixed_point;
pub type png_const_fixed_point_p = *const png_fixed_point;
pub type png_size_tp = *mut usize;
pub type png_const_size_tp = *const usize;
pub type png_doublep = *mut f64;
pub type png_const_doublep = *const f64;

pub type png_bytepp = *mut *mut png_byte;
pub type png_uint_32pp = *mut *mut png_uint_32;
pub type png_int_32pp = *mut *mut png_int_32;
pub type png_uint_16pp = *mut *mut png_uint_16;
pub type png_int_16pp = *mut *mut png_int_16;
pub type png_const_charpp = *mut *const c_char;
pub type png_charpp = *mut *mut c_char;
pub type png_fixed_point_pp = *mut *mut png_fixed_point;
pub type png_doublepp = *mut *mut f64;
pub type png_charppp = *mut *mut *mut c_char;

/// Opaque stdio FILE.
pub enum FILE {}
pub type png_FILE_p = *mut FILE;

pub type time_t = i64;

/// `struct tm` as defined by glibc.
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

/* ------------------------------------------------------------- setjmp.h ABI */

/// glibc `struct __jmp_buf_tag` (x86-64): 200 bytes, 8 byte alignment.
#[repr(C)]
#[derive(Copy, Clone)]
pub struct __jmp_buf_tag {
    pub __jmpbuf: [c_long; 8],
    pub __mask_was_saved: c_int,
    pub __saved_mask: [c_ulong; 16],
}

impl __jmp_buf_tag {
    pub const fn new() -> Self {
        __jmp_buf_tag {
            __jmpbuf: [0; 8],
            __mask_was_saved: 0,
            __saved_mask: [0; 16],
        }
    }
}

/// `jmp_buf` is an array of one `__jmp_buf_tag`; as a function argument it
/// decays to `*mut __jmp_buf_tag`.
pub type jmp_buf = [__jmp_buf_tag; 1];

/* ------------------------------------------------------------------ libc */

extern "C" {
    pub fn malloc(size: usize) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn abort() -> !;
    pub fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(dest: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
    pub fn gmtime(timep: *const time_t) -> *mut tm;
    pub fn pow(x: f64, y: f64) -> f64;
    pub fn floor(x: f64) -> f64;
    pub fn ceil(x: f64) -> f64;
    pub fn fabs(x: f64) -> f64;
    pub fn modf(x: f64, iptr: *mut f64) -> f64;
    pub fn frexp(x: f64, exp: *mut c_int) -> f64;
    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fread(ptr: *mut c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    pub fn fwrite(ptr: *const c_void, size: usize, nmemb: usize, f: *mut FILE) -> usize;
    pub fn fflush(f: *mut FILE) -> c_int;
    pub fn ferror(f: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, f: *mut FILE) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn remove(path: *const c_char) -> c_int;
    pub fn __errno_location() -> *mut c_int;
    pub static mut stderr: *mut FILE;
}

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

/* ----------------------------------------------------------------- zlib.h */

pub type uInt = c_uint;
pub type uLong = c_ulong;
pub type uLongf = c_ulong;
pub type Bytef = u8;
pub type voidpf = *mut c_void;
pub type alloc_func = Option<unsafe extern "C" fn(voidpf, uInt, uInt) -> voidpf>;
pub type free_func = Option<unsafe extern "C" fn(voidpf, voidpf)>;

pub const ZLIB_VERNUM: c_int = 0x12b0;

#[repr(C)]
#[derive(Copy, Clone)]
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

/* Allowed flush values */
pub const Z_NO_FLUSH: c_int = 0;
pub const Z_PARTIAL_FLUSH: c_int = 1;
pub const Z_SYNC_FLUSH: c_int = 2;
pub const Z_FULL_FLUSH: c_int = 3;
pub const Z_FINISH: c_int = 4;
pub const Z_BLOCK: c_int = 5;
pub const Z_TREES: c_int = 6;

/* Return codes */
pub const Z_OK: c_int = 0;
pub const Z_STREAM_END: c_int = 1;
pub const Z_NEED_DICT: c_int = 2;
pub const Z_ERRNO: c_int = -1;
pub const Z_STREAM_ERROR: c_int = -2;
pub const Z_DATA_ERROR: c_int = -3;
pub const Z_MEM_ERROR: c_int = -4;
pub const Z_BUF_ERROR: c_int = -5;
pub const Z_VERSION_ERROR: c_int = -6;

/* Compression levels */
pub const Z_NO_COMPRESSION: c_int = 0;
pub const Z_BEST_SPEED: c_int = 1;
pub const Z_BEST_COMPRESSION: c_int = 9;
pub const Z_DEFAULT_COMPRESSION: c_int = -1;

/* Compression strategy */
pub const Z_FILTERED: c_int = 1;
pub const Z_HUFFMAN_ONLY: c_int = 2;
pub const Z_RLE: c_int = 3;
pub const Z_FIXED: c_int = 4;
pub const Z_DEFAULT_STRATEGY: c_int = 0;

/* data_type */
pub const Z_BINARY: c_int = 0;
pub const Z_TEXT: c_int = 1;
pub const Z_UNKNOWN: c_int = 2;

pub const Z_DEFLATED: c_int = 8;
pub const Z_NULL: c_int = 0;

pub const MAX_WBITS: c_int = 15;
pub const MAX_MEM_LEVEL: c_int = 9;

/// zlib version string used for the `*Init2_` calls: "1.2.11"
pub static ZLIB_VERSION: [c_char; 7] = [49, 46, 50, 46, 49, 49, 0];

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
    pub fn crc32(crc: uLong, buf: *const Bytef, len: uInt) -> uLong;
}

/// `deflateInit2()` as the zlib macro expands it.
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
        ZLIB_VERSION.as_ptr(),
        core::mem::size_of::<z_stream>() as c_int,
    )
}

/// `inflateInit2()` as the zlib macro expands it.
#[inline]
pub unsafe fn inflateInit2(strm: z_streamp, windowBits: c_int) -> c_int {
    inflateInit2_(
        strm,
        windowBits,
        ZLIB_VERSION.as_ptr(),
        core::mem::size_of::<z_stream>() as c_int,
    )
}

/* ------------------------------------------------------- setjmp / longjmp */

/// Internal `setjmp` replacement, used only for libpng's own private jmp_bufs
/// (`png_control::error_buf`, see png_safe_execute).  The application visible
/// jmp_buf (png_struct::jmp_buf_local) is always handled by the caller supplied
/// `longjmp_fn`, i.e. by the C library's own setjmp/longjmp pair.
///
/// Saves the callee saved registers, the stack pointer and the return address
/// in the first 64 bytes of the buffer.  Returns 0 when called directly and the
/// value passed to png_private_longjmp() when arriving via a jump.
#[unsafe(naked)]
pub unsafe extern "C" fn png_private_setjmp(_env: *mut __jmp_buf_tag) -> c_int {
    core::arch::naked_asm!(
        "mov [rdi], rbx",
        "mov [rdi + 8], rbp",
        "mov [rdi + 16], r12",
        "mov [rdi + 24], r13",
        "mov [rdi + 32], r14",
        "mov [rdi + 40], r15",
        "lea rax, [rsp + 8]",
        "mov [rdi + 48], rax",
        "mov rax, [rsp]",
        "mov [rdi + 56], rax",
        "xor eax, eax",
        "ret",
    )
}

/// Counterpart of png_private_setjmp().
#[unsafe(naked)]
pub unsafe extern "C" fn png_private_longjmp(_env: *mut __jmp_buf_tag, _val: c_int) -> ! {
    core::arch::naked_asm!(
        "mov eax, esi",
        "test eax, eax",
        "jne 2f",
        "mov eax, 1",
        "2:",
        "mov rbx, [rdi]",
        "mov rbp, [rdi + 8]",
        "mov r12, [rdi + 16]",
        "mov r13, [rdi + 24]",
        "mov r14, [rdi + 32]",
        "mov r15, [rdi + 40]",
        "mov rdx, [rdi + 56]",
        "mov rsp, [rdi + 48]",
        "jmp rdx",
    )
}
