//! Declarations of the libc entry points used by the library.

use core::ffi::{c_char, c_double, c_int, c_long, c_uint, c_void};

pub type size_t = usize;
pub type ssize_t = isize;

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

/// x86-64 System V `va_list` (a.k.a. `__va_list_tag`).
///
/// In C a `va_list` argument decays to a pointer to this structure, so every
/// function that accepts a `va_list` in the C sources takes a
/// `*mut VaListTag` here.
#[repr(C)]
#[derive(Clone, Copy)]
pub struct VaListTag {
    pub gp_offset: c_uint,
    pub fp_offset: c_uint,
    pub overflow_arg_area: *mut c_void,
    pub reg_save_area: *mut c_void,
}

extern "C" {
    pub fn malloc(size: size_t) -> *mut c_void;
    pub fn realloc(ptr: *mut c_void, size: size_t) -> *mut c_void;
    pub fn free(ptr: *mut c_void);
    pub fn memcpy(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(dst: *mut c_void, src: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(dst: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    pub fn memchr(s: *const c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strncpy(dst: *mut c_char, src: *const c_char, n: size_t) -> *mut c_char;
    pub fn strerror(errnum: c_int) -> *mut c_char;
    pub fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> c_double;
    pub fn strtoll(s: *const c_char, endptr: *mut *mut c_char, base: c_int) -> i64;
    pub fn qsort(
        base: *mut c_void,
        nmemb: size_t,
        size: size_t,
        compar: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    );

    pub fn snprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ap: *mut VaListTag) -> c_int;

    pub fn fopen(path: *const c_char, mode: *const c_char) -> *mut FILE;
    pub fn fclose(f: *mut FILE) -> c_int;
    pub fn fwrite(ptr: *const c_void, size: size_t, n: size_t, f: *mut FILE) -> size_t;
    pub fn fgetc(f: *mut FILE) -> c_int;

    pub fn read(fd: c_int, buf: *mut c_void, count: size_t) -> ssize_t;
    pub fn write(fd: c_int, buf: *const c_void, count: size_t) -> ssize_t;
    pub fn open(path: *const c_char, flags: c_int, ...) -> c_int;
    pub fn close(fd: c_int) -> c_int;
    pub fn getpid() -> c_int;
    pub fn sched_yield() -> c_int;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;

    pub fn __errno_location() -> *mut c_int;

    #[link_name = "stdin"]
    pub static mut c_stdin: *mut FILE;
}

#[repr(C)]
pub struct timeval {
    pub tv_sec: c_long,
    pub tv_usec: c_long,
}

pub const EOF: c_int = -1;
pub const ERANGE: c_int = 34;
pub const O_RDONLY: c_int = 0;
pub const STDIN_FILENO: c_int = 0;
pub const HUGE_VAL: c_double = f64::INFINITY;

#[inline]
pub unsafe fn errno() -> c_int {
    *__errno_location()
}

#[inline]
pub unsafe fn set_errno(v: c_int) {
    *__errno_location() = v;
}

/// Fetch the next 8-byte integer/pointer sized argument from a `va_list`,
/// following the x86-64 System V calling convention.
#[inline]
pub unsafe fn va_arg_u64(ap: *mut VaListTag) -> u64 {
    let gp = (*ap).gp_offset;
    if gp <= 48 - 8 {
        let p = ((*ap).reg_save_area as *mut u8).add(gp as usize);
        (*ap).gp_offset = gp + 8;
        core::ptr::read_unaligned(p as *const u64)
    } else {
        let p = (*ap).overflow_arg_area as *mut u8;
        (*ap).overflow_arg_area = p.add(8) as *mut c_void;
        core::ptr::read_unaligned(p as *const u64)
    }
}

/// Fetch the next `double` argument from a `va_list`.
#[inline]
pub unsafe fn va_arg_f64(ap: *mut VaListTag) -> f64 {
    let fp = (*ap).fp_offset;
    if fp <= 176 - 16 {
        let p = ((*ap).reg_save_area as *mut u8).add(fp as usize);
        (*ap).fp_offset = fp + 16;
        core::ptr::read_unaligned(p as *const f64)
    } else {
        let p = (*ap).overflow_arg_area as *mut u8;
        (*ap).overflow_arg_area = p.add(8) as *mut c_void;
        core::ptr::read_unaligned(p as *const f64)
    }
}

#[inline]
pub unsafe fn va_arg_int(ap: *mut VaListTag) -> c_int {
    va_arg_u64(ap) as u32 as c_int
}

#[inline]
pub unsafe fn va_arg_ptr<T>(ap: *mut VaListTag) -> *mut T {
    va_arg_u64(ap) as usize as *mut T
}
