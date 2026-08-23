//! Translated from jsnumber.c — the Number object and its prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe extern "C-unwind" fn jsB_new_Number(J: *mut js_State) {
    crate::jsvalue::js_newnumber(J, if js_gettop(J) > 1 { js_tonumber(J, 1) } else { 0.0 });
}

unsafe extern "C-unwind" fn jsB_Number(J: *mut js_State) {
    js_pushnumber(J, if js_gettop(J) > 1 { js_tonumber(J, 1) } else { 0.0 });
}

unsafe extern "C-unwind" fn Np_valueOf(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CNUMBER {
        crate::jserror::js_typeerror(J, cstr!("not a number"));
    }
    js_pushnumber(J, (*self_).u.number);
}

unsafe extern "C-unwind" fn Np_toString(J: *mut js_State) {
    let mut buf: [c_char; 100] = [0; 100];
    let self_ = js_toobject(J, 0);
    let radix = if js_isundefined(J, 1) != 0 { 10 } else { js_tointeger(J, 1) };
    let mut x: f64 = 0.0;
    if (*self_).type_ != JS_CNUMBER {
        crate::jserror::js_typeerror(J, cstr!("not a number"));
    }
    x = (*self_).u.number;
    if radix == 10 {
        js_pushstring(J, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), x));
        return;
    }
    if radix < 2 || radix > 36 {
        crate::jserror::js_rangeerror(J, cstr!("invalid radix"));
    }

    /* lame number to string conversion for any radix from 2 to 36 */
    {
        static digits: &[u8; 37] = b"0123456789abcdefghijklmnopqrstuvwxyz\0";
        let mut number = x;
        let sign = (x < 0.0) as c_int;
        let mut sb: *mut js_Buffer = std::ptr::null_mut();
        let mut u: u64;
        let limit: u64 = 1u64 << 52;

        let mut ndigits: c_int;
        let mut exp: c_int;
        let mut point: c_int;

        if number == 0.0 {
            js_pushstring(J, cstr!("0"));
            return;
        }
        if number.is_nan() {
            js_pushstring(J, cstr!("NaN"));
            return;
        }
        if number.is_infinite() {
            js_pushstring(J, if sign != 0 { cstr!("-Infinity") } else { cstr!("Infinity") });
            return;
        }

        if sign != 0 {
            number = -number;
        }

        /* fit as many digits as we want in an int */
        exp = 0;
        while number * crate::cutil::pow(radix as f64, exp as f64) > limit as f64 {
            exp -= 1;
        }
        while number * crate::cutil::pow(radix as f64, (exp + 1) as f64) < limit as f64 {
            exp += 1;
        }
        // jsnumber.c:65 `u = number * pow(radix, exp) + 0.5;` — implicit double -> `uint64_t`.
        u = crate::cutil::d2ul(number * crate::cutil::pow(radix as f64, exp as f64) + 0.5);

        /* trim trailing zeros */
        while u > 0 && (u % radix as u64) == 0 {
            u /= radix as u64;
            exp -= 1;
        }

        /* serialize digits */
        ndigits = 0;
        while u > 0 {
            buf[ndigits as usize] = digits[(u % radix as u64) as usize] as c_char;
            ndigits += 1;
            u /= radix as u64;
        }
        point = ndigits - exp;

        let sb_ptr = std::ptr::addr_of_mut!(sb);
        let caught = protect(J, || {
            if sign != 0 {
                crate::jsintern::js_putc(J, sb_ptr, '-' as c_int);
            }

            if point <= 0 {
                crate::jsintern::js_putc(J, sb_ptr, '0' as c_int);
                crate::jsintern::js_putc(J, sb_ptr, '.' as c_int);
                while {
                    let old = point;
                    point += 1;
                    old < 0
                } {
                    crate::jsintern::js_putc(J, sb_ptr, '0' as c_int);
                }
                while {
                    ndigits -= 1;
                    ndigits + 1 > 0
                } {
                    crate::jsintern::js_putc(J, sb_ptr, buf[ndigits as usize] as c_int);
                }
            } else {
                while {
                    ndigits -= 1;
                    ndigits + 1 > 0
                } {
                    crate::jsintern::js_putc(J, sb_ptr, buf[ndigits as usize] as c_int);
                    point -= 1;
                    if point == 0 && ndigits > 0 {
                        crate::jsintern::js_putc(J, sb_ptr, '.' as c_int);
                    }
                }
                while {
                    let old = point;
                    point -= 1;
                    old > 0
                } {
                    crate::jsintern::js_putc(J, sb_ptr, '0' as c_int);
                }
            }

            crate::jsintern::js_putc(J, sb_ptr, 0);
            js_pushstring(J, (*sb).s.as_ptr());
        });
        if caught {
            js_free(J, sb as *mut _);
            js_throw(J);
        }
        js_endtry(J);
        js_free(J, sb as *mut _);
    }
}

/* Customized ToString() on a number */
unsafe fn numtostr(J: *mut js_State, fmt: *const c_char, w: c_int, n: f64) {
    /* buf needs to fit printf("%.20f", 1e20) */
    let mut buf: [c_char; 50] = [0; 50];
    let e: *mut c_char;
    libc::sprintf(buf.as_mut_ptr(), fmt, w, n);
    e = libc::strchr(buf.as_ptr(), 'e' as c_int) as *mut c_char;
    if !e.is_null() {
        let exp = libc::atoi(e.add(1));
        libc::sprintf(e, cstr!("e%+d"), exp);
    }
    js_pushstring(J, buf.as_ptr());
}

unsafe extern "C-unwind" fn Np_toFixed(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let width = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        crate::jserror::js_typeerror(J, cstr!("not a number"));
    }
    if width < 0 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    if width > 20 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    x = (*self_).u.number;
    if x.is_nan() || x.is_infinite() || x <= -1e21 || x >= 1e21 {
        js_pushstring(J, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cstr!("%.*f"), width, x);
    }
}

unsafe extern "C-unwind" fn Np_toExponential(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let width = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        crate::jserror::js_typeerror(J, cstr!("not a number"));
    }
    if width < 0 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    if width > 20 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    x = (*self_).u.number;
    if x.is_nan() || x.is_infinite() {
        js_pushstring(J, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cstr!("%.*e"), width, x);
    }
}

unsafe extern "C-unwind" fn Np_toPrecision(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    let width = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        crate::jserror::js_typeerror(J, cstr!("not a number"));
    }
    if width < 1 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    if width > 21 {
        crate::jserror::js_rangeerror(J, cstr!("precision %d out of range"), width);
    }
    x = (*self_).u.number;
    if x.is_nan() || x.is_infinite() {
        js_pushstring(J, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cstr!("%.*g"), width, x);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initnumber(J: *mut js_State) {
    (*(*J).Number_prototype).u.number = 0.0;

    js_pushobject(J, (*J).Number_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.valueOf"), Some(Np_valueOf), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.toString"), Some(Np_toString), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.toLocaleString"), Some(Np_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.toFixed"), Some(Np_toFixed), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.toExponential"), Some(Np_toExponential), 1);
        crate::jsbuiltin::jsB_propf(J, cstr!("Number.prototype.toPrecision"), Some(Np_toPrecision), 1);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_Number), Some(jsB_new_Number), cstr!("Number"), 0); /* 1 */
    {
        crate::jsbuiltin::jsB_propn(J, cstr!("MAX_VALUE"), 1.7976931348623157e+308);
        crate::jsbuiltin::jsB_propn(J, cstr!("MIN_VALUE"), 5e-324);
        crate::jsbuiltin::jsB_propn(J, cstr!("NaN"), f64::NAN);
        crate::jsbuiltin::jsB_propn(J, cstr!("NEGATIVE_INFINITY"), -f64::INFINITY);
        crate::jsbuiltin::jsB_propn(J, cstr!("POSITIVE_INFINITY"), f64::INFINITY);
    }
    js_defglobal(J, cstr!("Number"), JS_DONTENUM);
}
