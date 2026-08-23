//! Declarations of the C standard library functions used by the translated code.
#![allow(non_camel_case_types)]
#![allow(dead_code)]

pub use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_ushort, c_void};

pub type size_t = usize;
pub type time_t = c_long;
pub type suseconds_t = c_long;

#[repr(C)]
#[derive(Copy, Clone)]
pub struct timeval {
    pub tv_sec: time_t,
    pub tv_usec: suseconds_t,
}

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

pub const NULL_TM: tm = tm {
    tm_sec: 0,
    tm_min: 0,
    tm_hour: 0,
    tm_mday: 0,
    tm_mon: 0,
    tm_year: 0,
    tm_wday: 0,
    tm_yday: 0,
    tm_isdst: 0,
    tm_gmtoff: 0,
    tm_zone: core::ptr::null(),
};

extern "C" {
    pub fn malloc(n: size_t) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: size_t) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn memcpy(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: size_t) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: size_t) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: size_t) -> c_int;
    pub fn memchr(a: *const c_void, c: c_int, n: size_t) -> *mut c_void;

    pub fn strlen(s: *const c_char) -> size_t;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: size_t) -> c_int;
    pub fn strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strcat(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strrchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char;

    pub fn snprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn vsnprintf(s: *mut c_char, n: size_t, fmt: *const c_char, ap: *mut c_void) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn putchar(c: c_int) -> c_int;
    pub fn fputs(s: *const c_char, f: *mut c_void) -> c_int;
    pub fn fputc(c: c_int, f: *mut c_void) -> c_int;
    pub fn fflush(f: *mut c_void) -> c_int;
    pub fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;

    pub fn abort() -> !;
    pub fn exit(code: c_int) -> !;
    pub fn qsort(
        base: *mut c_void,
        n: size_t,
        w: size_t,
        cmp: Option<unsafe extern "C" fn(*const c_void, *const c_void) -> c_int>,
    );

    pub fn time(t: *mut time_t) -> time_t;
    pub fn mktime(t: *mut tm) -> time_t;
    pub fn gmtime(t: *const time_t) -> *mut tm;
    pub fn localtime(t: *const time_t) -> *mut tm;
    pub fn gettimeofday(tv: *mut timeval, tz: *mut c_void) -> c_int;

    pub fn pow(x: c_double, y: c_double) -> c_double;
    pub fn sqrt(x: c_double) -> c_double;
    pub fn exp(x: c_double) -> c_double;
    pub fn log(x: c_double) -> c_double;
    pub fn sin(x: c_double) -> c_double;
    pub fn cos(x: c_double) -> c_double;
    pub fn tan(x: c_double) -> c_double;
    pub fn asin(x: c_double) -> c_double;
    pub fn acos(x: c_double) -> c_double;
    pub fn atan(x: c_double) -> c_double;
    pub fn atan2(y: c_double, x: c_double) -> c_double;
    pub fn fabs(x: c_double) -> c_double;
    pub fn floor(x: c_double) -> c_double;
    pub fn ceil(x: c_double) -> c_double;
    pub fn fmod(x: c_double, y: c_double) -> c_double;
    pub fn ldexp(x: c_double, e: c_int) -> c_double;
    pub fn frexp(x: c_double, e: *mut c_int) -> c_double;
    pub fn strtod(s: *const c_char, e: *mut *mut c_char) -> c_double;

    pub fn longjmp(env: *mut c_void, val: c_int) -> !;

    pub static mut stdout: *mut c_void;
    pub static mut stderr: *mut c_void;
}

/// C `isnan()`
#[inline]
pub fn isnan(x: f64) -> bool {
    x.is_nan()
}

/// C `isinf()`
#[inline]
pub fn isinf(x: f64) -> bool {
    x.is_infinite()
}

/// C `isfinite()`
#[inline]
pub fn isfinite(x: f64) -> bool {
    x.is_finite()
}

/// C `signbit()`
#[inline]
pub fn signbit(x: f64) -> bool {
    x.is_sign_negative()
}

pub const INFINITY: f64 = f64::INFINITY;
pub const NAN: f64 = f64::NAN;
pub const DBL_MAX: f64 = f64::MAX;
pub const DBL_MIN: f64 = f64::MIN_POSITIVE;
pub const DBL_EPSILON: f64 = f64::EPSILON;
pub const INT_MAX: c_int = c_int::MAX;
pub const INT_MIN: c_int = c_int::MIN;
pub const UINT_MAX: c_uint = c_uint::MAX;
pub const CHAR_BIT: c_int = 8;
