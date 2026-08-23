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

/* ------------------------------------------------------------------ */
/*  C floating-point -> integer conversions                            */
/* ------------------------------------------------------------------ */
//
// Rust's `as` cast from a float SATURATES (and maps NaN to 0). C's conversion is
// undefined when the truncated value is out of range, and on x86-64 every
// compiler lowers it to `cvttsd2si`, which yields the "integer indefinite"
// value (INT_MIN / INT64_MIN) instead. MuJS *relies* on that: e.g.
// `MakeDay`'s `im = (int)m; if (im < 0 || im >= 12) return NAN;` only rejects a
// NaN month because `(int)NaN` is INT_MIN, and
// `obj->u.r.last = jsV_tointeger(...)` stores `lastIndex = -1` as 65535.
//
// The helpers below reproduce the exact observed gcc/clang -O2 x86-64 behaviour
// (verified against a C probe over NaN, +-inf, +-1, the int32/uint32/int16
// boundaries, 1e18, 1e19 and 1e300):
//
//   (int)d            -> cvttsd2si 32-bit; out of range => INT_MIN
//   (long)d           -> cvttsd2si 64-bit; out of range => INT64_MIN
//   (unsigned)d       -> low 32 bits of (long)d
//   (short)d          -> low 16 bits of (int)d
//   (unsigned short)d -> low 16 bits of (int)d
//   (char)d           -> low  8 bits of (int)d

/// C `(int)d` on x86-64.
#[inline]
pub fn d2i(d: f64) -> c_int {
    let t = d.trunc();
    if t >= -2147483648.0 && t <= 2147483647.0 {
        t as c_int
    } else {
        c_int::MIN // cvttsd2si "integer indefinite" (covers NaN and +-inf)
    }
}

/// C `(long)d` / `(int64_t)d` on x86-64.
#[inline]
pub fn d2l(d: f64) -> i64 {
    let t = d.trunc();
    if t >= -9223372036854775808.0 && t < 9223372036854775808.0 {
        t as i64
    } else {
        i64::MIN
    }
}

/// C `(unsigned)d` / `(uint32_t)d` on x86-64.
#[inline]
pub fn d2u(d: f64) -> u32 {
    d2l(d) as u32
}

/// C `(short)d` / `(int16_t)d` on x86-64.
#[inline]
pub fn d2s(d: f64) -> i16 {
    d2i(d) as i16
}

/// C `(unsigned short)d` / `(uint16_t)d` on x86-64.
#[inline]
pub fn d2us(d: f64) -> u16 {
    d2i(d) as u16
}

/// C `(char)d` / `(int8_t)d` on x86-64.
#[inline]
pub fn d2c(d: f64) -> c_char {
    d2i(d) as c_char
}

/// C `(size_t)d` / `(unsigned long)d` on x86-64.
#[inline]
pub fn d2ul(d: f64) -> u64 {
    // gcc lowers this as: if d < 2^63 use cvttsd2si, else cvttsd2si(d - 2^63)
    // with the sign bit set back. Reproduce that.
    if d < 9223372036854775808.0 {
        d2l(d) as u64
    } else {
        let t = d - 9223372036854775808.0;
        (d2l(t) as u64) | 0x8000_0000_0000_0000
    }
}

/// Interpret a *const c_char as a Rust &str for pattern matching (assumes valid UTF-8 up to NUL).
pub unsafe fn cstr_bytes<'a>(s: *const c_char) -> &'a [u8] {
    let len = strlen(s);
    std::slice::from_raw_parts(s as *const u8, len)
}
