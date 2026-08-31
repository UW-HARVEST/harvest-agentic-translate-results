#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]
use crate::common::*;
use crate::jsbuiltin::{jsB_propf, jsB_propn};
use crate::jsintern::js_putc;
use crate::jsrun::{
    js_defglobal, js_endtry, js_free, js_gettop, js_isundefined, js_pushnumber,
    js_pushobject, js_pushstring, js_throw, js_tointeger, js_tonumber, js_toobject,
};
use crate::jsvalue::{jsV_numbertostring, js_newcconstructor, js_newnumber};
use crate::types::*;
use crate::{js_rangeerror, js_typeerror};
use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn atoi(s: *const c_char) -> c_int;
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
}

unsafe extern "C-unwind" fn jsB_new_Number(J: *mut js_State) {
    unsafe {
        js_newnumber(J, if js_gettop(J) > 1 { js_tonumber(J, 1) } else { 0.0 });
    }
}

unsafe extern "C-unwind" fn jsB_Number(J: *mut js_State) {
    unsafe {
        js_pushnumber(J, if js_gettop(J) > 1 { js_tonumber(J, 1) } else { 0.0 });
    }
}

unsafe extern "C-unwind" fn Np_valueOf(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        if (*self_).type_ != JS_CNUMBER {
            js_typeerror!(J, c"not a number");
        }
        js_pushnumber(J, (*self_).u.number);
    }
}

unsafe extern "C-unwind" fn Np_toString(J: *mut js_State) {
    unsafe {
        let mut buf = [0 as c_char; 100];
        let self_ = js_toobject(J, 0);
        let radix = if js_isundefined(J, 1) != 0 { 10 } else { js_tointeger(J, 1) };
        let x: f64;
        if (*self_).type_ != JS_CNUMBER {
            js_typeerror!(J, c"not a number");
        }
        x = (*self_).u.number;
        if radix == 10 {
            js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
            return;
        }
        if radix < 2 || radix > 36 {
            js_rangeerror!(J, c"invalid radix");
        }

        /* lame number to string conversion for any radix from 2 to 36 */
        {
            static digits: &[u8; 37] = b"0123456789abcdefghijklmnopqrstuvwxyz\0";
            let mut number: f64 = x;
            let sign: c_int = (x < 0.0) as c_int;
            let mut sb: *mut js_Buffer = std::ptr::null_mut();
            let mut u: u64;
            let limit: u64 = (1u64) << 52;

            let mut ndigits: c_int;
            let mut exp: c_int;
            let mut point: c_int;

            if number == 0.0 {
                js_pushstring(J, c"0".as_ptr());
                return;
            }
            if isnan(number) {
                js_pushstring(J, c"NaN".as_ptr());
                return;
            }
            if isinf(number) {
                js_pushstring(J, if sign != 0 { c"-Infinity".as_ptr() } else { c"Infinity".as_ptr() });
                return;
            }

            if sign != 0 {
                number = -number;
            }

            /* fit as many digits as we want in an int */
            exp = 0;
            while number * pow(radix as f64, exp as f64) > limit as f64 {
                exp -= 1;
            }
            while number * pow(radix as f64, (exp + 1) as f64) < limit as f64 {
                exp += 1;
            }
            u = (number * pow(radix as f64, exp as f64) + 0.5) as u64;

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

            if js_try(J, || {
                if sign != 0 {
                    js_putc(J, &raw mut sb, b'-' as c_int);
                }

                if point <= 0 {
                    js_putc(J, &raw mut sb, b'0' as c_int);
                    js_putc(J, &raw mut sb, b'.' as c_int);
                    while {
                        let old = point;
                        point += 1;
                        old
                    } < 0
                    {
                        js_putc(J, &raw mut sb, b'0' as c_int);
                    }
                    while {
                        ndigits -= 1;
                        ndigits + 1
                    } > 0
                    {
                        js_putc(J, &raw mut sb, buf[ndigits as usize] as c_int);
                    }
                } else {
                    while {
                        ndigits -= 1;
                        ndigits + 1
                    } > 0
                    {
                        js_putc(J, &raw mut sb, buf[ndigits as usize] as c_int);
                        point -= 1;
                        if point == 0 && ndigits > 0 {
                            js_putc(J, &raw mut sb, b'.' as c_int);
                        }
                    }
                    while {
                        let old = point;
                        point -= 1;
                        old
                    } > 0
                    {
                        js_putc(J, &raw mut sb, b'0' as c_int);
                    }
                }

                js_putc(J, &raw mut sb, 0);
                js_pushstring(J, (&raw mut (*sb).s) as *const c_char);

                js_endtry(J);
                js_free(J, sb as *mut c_void);
            })
            .is_err()
            {
                js_free(J, sb as *mut c_void);
                js_throw(J);
            }
        }
    }
}

/* Customized ToString() on a number */
unsafe fn numtostr(J: *mut js_State, fmt: *const c_char, w: c_int, n: f64) {
    unsafe {
        /* buf needs to fit printf("%.20f", 1e20) */
        let mut buf = [0 as c_char; 50];
        snprintf(buf.as_mut_ptr(), 50, fmt, w, n);
        let e = strchr(buf.as_ptr(), b'e' as c_int);
        if !e.is_null() {
            let exp = atoi(e.offset(1));
            let off = e.offset_from(buf.as_ptr());
            snprintf(e, (50 - off) as usize, c"e%+d".as_ptr(), exp);
        }
        js_pushstring(J, buf.as_ptr());
    }
}

unsafe extern "C-unwind" fn Np_toFixed(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        let width = js_tointeger(J, 1);
        let mut buf = [0 as c_char; 32];
        let x: f64;
        if (*self_).type_ != JS_CNUMBER {
            js_typeerror!(J, c"not a number");
        }
        if width < 0 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        if width > 20 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        x = (*self_).u.number;
        if isnan(x) || isinf(x) || x <= -1e21 || x >= 1e21 {
            js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
        } else {
            numtostr(J, c"%.*f".as_ptr(), width, x);
        }
    }
}

unsafe extern "C-unwind" fn Np_toExponential(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        let width = js_tointeger(J, 1);
        let mut buf = [0 as c_char; 32];
        let x: f64;
        if (*self_).type_ != JS_CNUMBER {
            js_typeerror!(J, c"not a number");
        }
        if width < 0 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        if width > 20 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        x = (*self_).u.number;
        if isnan(x) || isinf(x) {
            js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
        } else {
            numtostr(J, c"%.*e".as_ptr(), width, x);
        }
    }
}

unsafe extern "C-unwind" fn Np_toPrecision(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        let width = js_tointeger(J, 1);
        let mut buf = [0 as c_char; 32];
        let x: f64;
        if (*self_).type_ != JS_CNUMBER {
            js_typeerror!(J, c"not a number");
        }
        if width < 1 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        if width > 21 {
            js_rangeerror!(J, c"precision %d out of range", width);
        }
        x = (*self_).u.number;
        if isnan(x) || isinf(x) {
            js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
        } else {
            numtostr(J, c"%.*g".as_ptr(), width, x);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initnumber(J: *mut js_State) {
    unsafe {
        (*(*J).Number_prototype).u.number = 0.0;

        js_pushobject(J, (*J).Number_prototype);
        {
            jsB_propf(J, c"Number.prototype.valueOf".as_ptr(), Some(Np_valueOf), 0);
            jsB_propf(J, c"Number.prototype.toString".as_ptr(), Some(Np_toString), 1);
            jsB_propf(J, c"Number.prototype.toLocaleString".as_ptr(), Some(Np_toString), 0);
            jsB_propf(J, c"Number.prototype.toFixed".as_ptr(), Some(Np_toFixed), 1);
            jsB_propf(J, c"Number.prototype.toExponential".as_ptr(), Some(Np_toExponential), 1);
            jsB_propf(J, c"Number.prototype.toPrecision".as_ptr(), Some(Np_toPrecision), 1);
        }
        js_newcconstructor(J, Some(jsB_Number), Some(jsB_new_Number), c"Number".as_ptr(), 0); /* 1 */
        {
            jsB_propn(J, c"MAX_VALUE".as_ptr(), 1.7976931348623157e+308);
            jsB_propn(J, c"MIN_VALUE".as_ptr(), 5e-324);
            jsB_propn(J, c"NaN".as_ptr(), NAN);
            jsB_propn(J, c"NEGATIVE_INFINITY".as_ptr(), -INFINITY);
            jsB_propn(J, c"POSITIVE_INFINITY".as_ptr(), INFINITY);
        }
        js_defglobal(J, c"Number".as_ptr(), JS_DONTENUM);
    }
}
