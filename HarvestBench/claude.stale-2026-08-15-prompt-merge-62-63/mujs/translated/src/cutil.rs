//! Small C-library-style helpers used throughout the port.
#![allow(dead_code)]

use std::os::raw::{c_char, c_int};

#[inline]
pub unsafe fn strlen(s: *const c_char) -> usize {
    libc::strlen(s)
}

#[inline]
pub unsafe fn strcmp(a: *const c_char, b: *const c_char) -> c_int {
    libc::strcmp(a, b)
}

#[inline]
pub unsafe fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int {
    libc::strncmp(a, b, n)
}

#[inline]
pub unsafe fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    libc::strcpy(dst, src)
}

#[inline]
pub unsafe fn strcat(dst: *mut c_char, src: *const c_char) -> *mut c_char {
    libc::strcat(dst, src)
}

#[inline]
pub unsafe fn strchr(s: *const c_char, c: c_int) -> *mut c_char {
    libc::strchr(s, c)
}

#[inline]
pub unsafe fn strrchr(s: *const c_char, c: c_int) -> *mut c_char {
    libc::strrchr(s, c)
}

#[inline]
pub unsafe fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char {
    libc::strstr(h, n)
}

#[inline]
pub unsafe fn memcpy(dst: *mut c_char, src: *const c_char, n: usize) {
    if n > 0 {
        libc::memcpy(dst as *mut _, src as *const _, n);
    }
}

#[inline]
pub unsafe fn memset(dst: *mut c_char, c: c_int, n: usize) {
    libc::memset(dst as *mut _, c, n);
}

/* Math functions (libc on linux-gnu doesn't export these; f64 methods call the
 * same system libm, giving identical results). */
#[inline] pub fn floor(x: f64) -> f64 { x.floor() }
#[inline] pub fn ceil(x: f64) -> f64 { x.ceil() }
#[inline] pub fn fabs(x: f64) -> f64 { x.abs() }
#[inline] pub fn fmod(x: f64, y: f64) -> f64 { x % y }
#[inline] pub fn pow(x: f64, y: f64) -> f64 { x.powf(y) }
#[inline] pub fn sqrt(x: f64) -> f64 { x.sqrt() }
#[inline] pub fn sin(x: f64) -> f64 { x.sin() }
#[inline] pub fn cos(x: f64) -> f64 { x.cos() }
#[inline] pub fn tan(x: f64) -> f64 { x.tan() }
#[inline] pub fn asin(x: f64) -> f64 { x.asin() }
#[inline] pub fn acos(x: f64) -> f64 { x.acos() }
#[inline] pub fn atan(x: f64) -> f64 { x.atan() }
#[inline] pub fn atan2(y: f64, x: f64) -> f64 { y.atan2(x) }
#[inline] pub fn exp(x: f64) -> f64 { x.exp() }
#[inline] pub fn log(x: f64) -> f64 { x.ln() }

/// Interpret a *const c_char as a Rust &str for pattern matching (assumes valid UTF-8 up to NUL).
pub unsafe fn cstr_bytes<'a>(s: *const c_char) -> &'a [u8] {
    let len = strlen(s);
    std::slice::from_raw_parts(s as *const u8, len)
}
