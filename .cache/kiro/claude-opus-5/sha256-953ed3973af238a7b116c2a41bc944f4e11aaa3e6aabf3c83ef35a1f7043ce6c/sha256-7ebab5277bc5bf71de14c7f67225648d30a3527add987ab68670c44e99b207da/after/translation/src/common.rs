// Common runtime support: libc bindings, memory helpers, exception handling.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::types::*;
use std::ffi::{c_char, c_double, c_int, c_long, c_uint, c_ulong, c_void};
use std::panic::{AssertUnwindSafe, catch_unwind, resume_unwind};

#[repr(C)]
pub struct FILE {
    _private: [u8; 0],
}

unsafe extern "C" {
    pub fn strlen(s: *const c_char) -> usize;
    pub fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub fn strcpy(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strcat(d: *mut c_char, s: *const c_char) -> *mut c_char;
    pub fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    pub fn strstr(h: *const c_char, n: *const c_char) -> *mut c_char;
    pub fn memcpy(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memmove(d: *mut c_void, s: *const c_void, n: usize) -> *mut c_void;
    pub fn memset(d: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub fn memcmp(a: *const c_void, b: *const c_void, n: usize) -> c_int;
    pub fn malloc(n: usize) -> *mut c_void;
    pub fn realloc(p: *mut c_void, n: usize) -> *mut c_void;
    pub fn free(p: *mut c_void);
    pub fn abort() -> !;
    pub fn exit(code: c_int) -> !;
    pub fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    pub fn printf(fmt: *const c_char, ...) -> c_int;
    pub fn fprintf(f: *mut FILE, fmt: *const c_char, ...) -> c_int;
    pub fn putchar(c: c_int) -> c_int;
    pub fn fputs(s: *const c_char, f: *mut FILE) -> c_int;
    pub fn fputc(c: c_int, f: *mut FILE) -> c_int;
    pub fn longjmp(env: *mut c_void, val: c_int) -> !;
    pub fn qsort(
        base: *mut c_void,
        n: usize,
        size: usize,
        cmp: Option<unsafe extern "C-unwind" fn(*const c_void, *const c_void) -> c_int>,
    );
    pub static mut stderr: *mut FILE;
    pub static mut stdout: *mut FILE;

    /* math */
    pub fn fmod(x: c_double, y: c_double) -> c_double;
    pub fn floor(x: c_double) -> c_double;
    pub fn ceil(x: c_double) -> c_double;
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

    /* time */
    pub fn time(t: *mut c_long) -> c_long;
    pub fn mktime(tm: *mut Tm) -> c_long;
    pub fn localtime(t: *const c_long) -> *mut Tm;
    pub fn gmtime(t: *const c_long) -> *mut Tm;
}

#[repr(C)]
pub struct Tm {
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

pub const INT_MAX: c_int = c_int::MAX;
pub const INT_MIN: c_int = c_int::MIN;
pub const UINT_MAX: c_uint = c_uint::MAX;
pub const DBL_MAX: f64 = f64::MAX;
pub const DBL_MIN: f64 = f64::MIN_POSITIVE;
pub const DBL_EPSILON: f64 = f64::EPSILON;
pub const INFINITY: f64 = f64::INFINITY;
pub const NAN: f64 = f64::NAN;

#[inline]
pub fn isnan(x: f64) -> bool {
    x.is_nan()
}
#[inline]
pub fn isinf(x: f64) -> bool {
    x.is_infinite()
}
#[inline]
pub fn isfinite(x: f64) -> bool {
    x.is_finite()
}
#[inline]
pub fn signbit(x: f64) -> bool {    x.is_sign_negative()
}

/// C's `(int)someDouble` conversion, reproduced exactly.
///
/// Converting a `double` that is NaN or outside the range of `int` to `int` is
/// undefined behaviour in C, but on x86-64 (the reference platform) `cvttsd2si`
/// deterministically yields the "integer indefinite" value `INT_MIN`. Rust's
/// `as` operator instead saturates and maps NaN to 0, so plain `as c_int` is
/// NOT a faithful translation. Use this helper wherever the C source casts a
/// possibly-NaN / possibly-out-of-range double to int.
#[inline]
pub fn d2i(x: f64) -> c_int {
    if x.is_nan() || x < (c_int::MIN as f64) || x > (c_int::MAX as f64) {
        c_int::MIN
    } else {
        x as c_int
    }
}

/// Same as [`d2i`] but for `long` / 64-bit integer casts.
#[inline]
pub fn d2l(x: f64) -> i64 {
    if x.is_nan() || x < (i64::MIN as f64) || x >= (i64::MAX as f64) {
        i64::MIN
    } else {
        x as i64
    }
}

/* ---------------------------------------------------------------- */
/* Exception handling                                               */
/* ---------------------------------------------------------------- */

/// Payload used for the panic that implements a JavaScript `throw` when the
/// target try-frame was established by Rust code (as opposed to C setjmp).
pub struct JsThrow;

/// Establish an internal try frame and run `body`.
///
/// Mirrors the C idiom:
/// ```c
/// if (js_try(J)) { handler }
/// body
/// js_endtry(J);
/// ```
/// `body` is responsible for calling `js_endtry` on the success path, exactly
/// like the C code does.
#[inline]
pub unsafe fn js_try<R>(J: *mut js_State, body: impl FnOnce() -> R) -> Result<R, ()> {
    unsafe {
        let k = (*J).trytop;
        crate::jsrun::pushtry(J, std::ptr::null_mut(), TRY_INTERNAL);
        match catch_unwind(AssertUnwindSafe(body)) {
            Ok(v) => Ok(v),
            Err(e) => {
                if e.downcast_ref::<JsThrow>().is_some() && (*J).trytop == k {
                    Err(())
                } else {
                    resume_unwind(e)
                }
            }
        }
    }
}

/// Raise a JavaScript exception by unwinding to the innermost internal try frame.
pub fn do_throw() -> ! {
    std::panic::panic_any(JsThrow)
}

/// Silence panic output: JsThrow panics are a control-flow mechanism, and any
/// message would corrupt the program's output.
pub fn install_panic_hook() {
    use std::sync::Once;
    static ONCE: Once = Once::new();
    ONCE.call_once(|| {
        std::panic::set_hook(Box::new(|_| {}));
    });
}

/* ---------------------------------------------------------------- */
/* Error raising helpers (see jserror.rs)                           */
/* ---------------------------------------------------------------- */

#[macro_export]
macro_rules! js_error {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_error($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_error($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_evalerror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_evalerror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_evalerror($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_rangeerror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_rangeerror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_rangeerror($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_referenceerror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_referenceerror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_referenceerror($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_syntaxerror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_syntaxerror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_syntaxerror($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_typeerror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_typeerror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_typeerror($J, __b.as_ptr())
    }};
}

#[macro_export]
macro_rules! js_urierror {
    ($J:expr, $fmt:literal) => { $crate::jserror::rt_urierror($J, $fmt.as_ptr()) };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __b = [0 as std::ffi::c_char; 256];
        $crate::common::snprintf(__b.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        $crate::jserror::rt_urierror($J, __b.as_ptr())
    }};
}

/* ---------------------------------------------------------------- */
/* Misc helpers                                                     */
/* ---------------------------------------------------------------- */

/// `char *` at a flexible array member.
#[inline]
pub unsafe fn cstr_of(p: *mut c_char) -> *const c_char {
    p as *const c_char
}

#[inline]
pub unsafe fn cchar(p: *const c_char, i: isize) -> c_int {
    unsafe { *p.offset(i) as c_int }
}

/// Unsigned char access, as C's `(unsigned char)s[i]`.
#[inline]
pub unsafe fn uchar(p: *const c_char, i: isize) -> c_int {
    unsafe { *(p.offset(i) as *const u8) as c_int }
}

pub type c_ulonglong_alias = c_ulong;
