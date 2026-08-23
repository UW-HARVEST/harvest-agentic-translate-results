//! Translated from jsmath.c — the Math object.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_uint};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe fn jsM_round(x: f64) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x.is_infinite() {
        return x;
    }
    if x == 0.0 {
        return x;
    }
    if x > 0.0 && x < 0.5 {
        return 0.0;
    }
    if x < 0.0 && x >= -0.5 {
        /* The C writes `return -0;` — that is the *integer* constant 0 negated,
         * which is still integer 0, and it converts to POSITIVE zero. It is NOT
         * the double -0.0. So Math.round(-0.5) is +0 in MuJS, and `1/x` is
         * +Infinity. Do not "fix" this to -0.0. */
        return 0.0;
    }
    crate::cutil::floor(x + 0.5)
}

unsafe extern "C-unwind" fn Math_abs(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::fabs(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_acos(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::acos(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_asin(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::asin(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_atan(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::atan(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_atan2(J: *mut js_State) {
    let y = js_tonumber(J, 1);
    let x = js_tonumber(J, 2);
    js_pushnumber(J, crate::cutil::atan2(y, x));
}

unsafe extern "C-unwind" fn Math_ceil(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::ceil(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_cos(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::cos(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_exp(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::exp(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_floor(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::floor(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_log(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::log(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_pow(J: *mut js_State) {
    let x = js_tonumber(J, 1);
    let y = js_tonumber(J, 2);
    if !y.is_finite() && crate::cutil::fabs(x) == 1.0 {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, crate::cutil::pow(x, y));
    }
}

unsafe extern "C-unwind" fn Math_random(J: *mut js_State) {
    /* Lehmer generator with a=48271 and m=2^31-1 */
    /* Park & Miller (1988). Random Number Generators: Good ones are hard to find. */
    (*J).seed = (((*J).seed as u64) * 48271 % 0x7fffffff) as c_uint;
    js_pushnumber(J, (*J).seed as f64 / 0x7fffffff as f64);
}

unsafe fn Math_init_random(J: *mut js_State) {
    /* Pick initial seed by scrambling current time with Xorshift. */
    /* Marsaglia (2003). Xorshift RNGs. */
    (*J).seed = (libc::time(std::ptr::null_mut()) as c_uint).wrapping_add(123);
    (*J).seed ^= (*J).seed << 13;
    (*J).seed ^= (*J).seed >> 17;
    (*J).seed ^= (*J).seed << 5;
    (*J).seed %= 0x7fffffff;
}

unsafe extern "C-unwind" fn Math_round(J: *mut js_State) {
    let x = js_tonumber(J, 1);
    js_pushnumber(J, jsM_round(x));
}

unsafe extern "C-unwind" fn Math_sin(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::sin(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_sqrt(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::sqrt(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_tan(J: *mut js_State) {
    js_pushnumber(J, crate::cutil::tan(js_tonumber(J, 1)));
}

unsafe extern "C-unwind" fn Math_max(J: *mut js_State) {
    let n = js_gettop(J);
    let mut x = f64::NEG_INFINITY;
    let mut i = 1;
    while i < n {
        let y = js_tonumber(J, i);
        if y.is_nan() {
            x = y;
            break;
        }
        if x.is_sign_negative() == y.is_sign_negative() {
            x = if x > y { x } else { y };
        } else if x.is_sign_negative() {
            x = y;
        }
        i += 1;
    }
    js_pushnumber(J, x);
}

unsafe extern "C-unwind" fn Math_min(J: *mut js_State) {
    let n = js_gettop(J);
    let mut x = f64::INFINITY;
    let mut i = 1;
    while i < n {
        let y = js_tonumber(J, i);
        if y.is_nan() {
            x = y;
            break;
        }
        if x.is_sign_negative() == y.is_sign_negative() {
            x = if x < y { x } else { y };
        } else if y.is_sign_negative() {
            x = y;
        }
        i += 1;
    }
    js_pushnumber(J, x);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initmath(J: *mut js_State) {
    Math_init_random(J);
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_CMATH, (*J).Object_prototype));
    {
        crate::jsbuiltin::jsB_propn(J, cstr!("E"), 2.7182818284590452354);
        crate::jsbuiltin::jsB_propn(J, cstr!("LN10"), 2.302585092994046);
        crate::jsbuiltin::jsB_propn(J, cstr!("LN2"), 0.6931471805599453);
        crate::jsbuiltin::jsB_propn(J, cstr!("LOG2E"), 1.4426950408889634);
        crate::jsbuiltin::jsB_propn(J, cstr!("LOG10E"), 0.4342944819032518);
        crate::jsbuiltin::jsB_propn(J, cstr!("PI"), 3.1415926535897932);
        crate::jsbuiltin::jsB_propn(J, cstr!("SQRT1_2"), 0.7071067811865476);
        crate::jsbuiltin::jsB_propn(J, cstr!("SQRT2"), 1.4142135623730951);

        crate::jsbuiltin::jsB_propf(J, cstr!("Math.abs"), Some(Math_abs), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.acos"), Some(Math_acos), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.asin"), Some(Math_asin), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.atan"), Some(Math_atan), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.atan2"), Some(Math_atan2), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.ceil"), Some(Math_ceil), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.cos"), Some(Math_cos), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.exp"), Some(Math_exp), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.floor"), Some(Math_floor), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.log"), Some(Math_log), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.max"), Some(Math_max), 0); /* 2 */
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.min"), Some(Math_min), 0); /* 2 */
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.pow"), Some(Math_pow), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.random"), Some(Math_random), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.round"), Some(Math_round), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.sin"), Some(Math_sin), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.sqrt"), Some(Math_sqrt), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Math.tan"), Some(Math_tan), 1);
    }
    js_defglobal(J, cstr!("Math"), JS_DONTENUM);
}
