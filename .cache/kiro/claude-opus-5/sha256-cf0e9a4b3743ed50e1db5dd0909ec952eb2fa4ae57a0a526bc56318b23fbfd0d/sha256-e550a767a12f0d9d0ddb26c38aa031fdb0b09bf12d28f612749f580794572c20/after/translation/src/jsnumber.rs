//! Translation of src/jsnumber.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;
use core::ptr::null_mut;

use crate::jsrun::{
    js_defglobal, js_free, js_gettop, js_isundefined, js_pushnumber, js_pushobject, js_pushstring,
    js_tointeger, js_tonumber, js_toobject, js_throw,
};
use crate::jsintern::js_putc;
use crate::jsvalue::{js_newcconstructor, js_newnumber, jsV_numbertostring};
use crate::jsbuiltin::{jsB_propf, jsB_propn};

unsafe extern "C" {
    fn atoi(s: *const c_char) -> c_int;
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
        let self_: *mut js_Object = js_toobject(J, 0);
        if (*self_).ty != JS_CNUMBER {
            js_typeerror!(J, c"not a number".as_ptr());
        }
        js_pushnumber(J, (*self_).u.number);
    }
}

unsafe extern "C-unwind" fn Np_toString(J: *mut js_State) {
    unsafe {
        let mut buf: [c_char; 100] = [0; 100];
        let self_: *mut js_Object = js_toobject(J, 0);
        let radix: c_int = if js_isundefined(J, 1) != 0 {
            10
        } else {
            js_tointeger(J, 1)
        };
        let mut x: f64 = 0.0;
        if (*self_).ty != JS_CNUMBER {
            js_typeerror!(J, c"not a number".as_ptr());
        }
        x = (*self_).u.number;
        if radix == 10 {
            js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
            return;
        }
        if radix < 2 || radix > 36 {
            js_rangeerror!(J, c"invalid radix".as_ptr());
        }

        /* lame number to string conversion for any radix from 2 to 36 */
        {
            static digits: [c_char; 37] = {
                let src = b"0123456789abcdefghijklmnopqrstuvwxyz\0";
                let mut a = [0 as c_char; 37];
                let mut i = 0;
                while i < 37 {
                    a[i] = src[i] as c_char;
                    i += 1;
                }
                a
            };
            let mut number: f64 = x;
            let sign: c_int = (x < 0.0) as c_int;
            let mut sb: *mut js_Buffer = null_mut();
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
                js_pushstring(
                    J,
                    if sign != 0 {
                        c"-Infinity".as_ptr()
                    } else {
                        c"Infinity".as_ptr()
                    },
                );
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
                buf[ndigits as usize] = digits[(u % radix as u64) as usize];
                ndigits += 1;
                u /= radix as u64;
            }
            point = ndigits - exp;

            if crate::except::js_try_run(J, || {
                if sign != 0 {
                    js_putc(J, &raw mut sb, '-' as c_int);
                }

                if point <= 0 {
                    js_putc(J, &raw mut sb, '0' as c_int);
                    js_putc(J, &raw mut sb, '.' as c_int);
                    while point < 0 {
                        point += 1;
                        js_putc(J, &raw mut sb, '0' as c_int);
                    }
                    while ndigits > 0 {
                        ndigits -= 1;
                        js_putc(J, &raw mut sb, buf[ndigits as usize] as c_int);
                    }
                } else {
                    while ndigits > 0 {
                        ndigits -= 1;
                        js_putc(J, &raw mut sb, buf[ndigits as usize] as c_int);
                        point -= 1;
                        if point == 0 && ndigits > 0 {
                            js_putc(J, &raw mut sb, '.' as c_int);
                        }
                    }
                    while point > 0 {
                        point -= 1;
                        js_putc(J, &raw mut sb, '0' as c_int);
                    }
                }

                js_putc(J, &raw mut sb, 0);
                js_pushstring(J, sbs(sb) as *const c_char);

                crate::jsrun::js_endtry(J);
            }) {
                js_free(J, sb as *mut c_void);
                js_throw(J);
            }
            js_free(J, sb as *mut c_void);
        }
    }
}

/* Customized ToString() on a number */
unsafe fn numtostr(J: *mut js_State, fmt: *const c_char, w: c_int, n: f64) {
    unsafe {
        /* buf needs to fit printf("%.20f", 1e20) */
        let mut buf: [c_char; 50] = [0; 50];
        let e: *mut c_char;
        sprintf(buf.as_mut_ptr(), fmt, w, n);
        e = strchr(buf.as_ptr(), 'e' as c_int);
        if !e.is_null() {
            let exp: c_int = atoi(e.offset(1));
            sprintf(e, c"e%+d".as_ptr(), exp);
        }
        js_pushstring(J, buf.as_ptr());
    }
}

unsafe extern "C-unwind" fn Np_toFixed(J: *mut js_State) {
    unsafe {
        let self_: *mut js_Object = js_toobject(J, 0);
        let width: c_int = js_tointeger(J, 1);
        let mut buf: [c_char; 32] = [0; 32];
        let x: f64;
        if (*self_).ty != JS_CNUMBER {
            js_typeerror!(J, c"not a number".as_ptr());
        }
        if width < 0 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
        }
        if width > 20 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
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
        let self_: *mut js_Object = js_toobject(J, 0);
        let width: c_int = js_tointeger(J, 1);
        let mut buf: [c_char; 32] = [0; 32];
        let x: f64;
        if (*self_).ty != JS_CNUMBER {
            js_typeerror!(J, c"not a number".as_ptr());
        }
        if width < 0 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
        }
        if width > 20 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
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
        let self_: *mut js_Object = js_toobject(J, 0);
        let width: c_int = js_tointeger(J, 1);
        let mut buf: [c_char; 32] = [0; 32];
        let x: f64;
        if (*self_).ty != JS_CNUMBER {
            js_typeerror!(J, c"not a number".as_ptr());
        }
        if width < 1 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
        }
        if width > 21 {
            js_rangeerror!(J, c"precision %d out of range".as_ptr(), width);
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
            jsB_propf(
                J,
                c"Number.prototype.toLocaleString".as_ptr(),
                Some(Np_toString),
                0,
            );
            jsB_propf(J, c"Number.prototype.toFixed".as_ptr(), Some(Np_toFixed), 1);
            jsB_propf(
                J,
                c"Number.prototype.toExponential".as_ptr(),
                Some(Np_toExponential),
                1,
            );
            jsB_propf(
                J,
                c"Number.prototype.toPrecision".as_ptr(),
                Some(Np_toPrecision),
                1,
            );
        }
        js_newcconstructor(
            J,
            Some(jsB_Number),
            Some(jsB_new_Number),
            c"Number".as_ptr(),
            0,
        ); /* 1 */
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
