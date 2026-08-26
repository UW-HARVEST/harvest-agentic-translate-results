//! Translation of `c_src/src/jsnumber.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsbuiltin::{jsB_propf, jsB_propn};
use crate::jsdtoa::{js_fmtexp, js_grisu2};
use crate::jsi::*;
use crate::jsintern::js_putc;
use crate::jsproperty::*;
use crate::jsrun::*;
use crate::jsvalue::*;
use core::ptr::{null, null_mut};

extern "C" {
    /// `<stdlib.h>` -- used by `numtostr` to re-parse the exponent.
    fn atoi(s: *const c_char) -> c_int;
}

unsafe extern "C-unwind" fn jsB_new_Number(J: *mut js_State) {
    js_newnumber(
        J,
        if js_gettop(J) > 1 {
            js_tonumber(J, 1)
        } else {
            0.0
        },
    );
}

unsafe extern "C-unwind" fn jsB_Number(J: *mut js_State) {
    js_pushnumber(
        J,
        if js_gettop(J) > 1 {
            js_tonumber(J, 1)
        } else {
            0.0
        },
    );
}

unsafe extern "C-unwind" fn Np_valueOf(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, c"not a number".as_ptr());
    }
    js_pushnumber(J, (*self_).u.number);
}

unsafe extern "C-unwind" fn Np_toString(J: *mut js_State) {
    let mut buf = [0 as c_char; 100];
    let self_: *mut js_Object = js_toobject(J, 0);
    let radix: c_int = if js_isundefined(J, 1) != 0 {
        10
    } else {
        js_tointeger(J, 1)
    };
    #[allow(unused_assignments)]
    let mut x: f64 = 0.0;
    if (*self_).type_ != JS_CNUMBER {
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
        let digits: *const c_char = c"0123456789abcdefghijklmnopqrstuvwxyz".as_ptr();
        let mut number: f64 = x;
        let sign: c_int = (x < 0.0) as c_int;
        let mut sb: *mut js_Buffer = null_mut();
        let sbp = &mut sb as *mut *mut js_Buffer;
        let mut u: u64;
        let limit: u64 = (1 as u64) << 52;

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
            *buf.as_mut_ptr().offset(ndigits as isize) = *digits.offset((u % radix as u64) as isize);
            ndigits += 1;
            u /= radix as u64;
        }
        point = ndigits - exp;

        let ndigitsp = &mut ndigits as *mut c_int;
        let pointp = &mut point as *mut c_int;
        let bufp = buf.as_mut_ptr();

        if js_do_try(J, || {
            if sign != 0 {
                js_putc(J, sbp, b'-' as c_int);
            }

            if *pointp <= 0 {
                js_putc(J, sbp, b'0' as c_int);
                js_putc(J, sbp, b'.' as c_int);
                /* while (point++ < 0) */
                loop {
                    let old = *pointp;
                    *pointp = old + 1;
                    if !(old < 0) {
                        break;
                    }
                    js_putc(J, sbp, b'0' as c_int);
                }
                /* while (ndigits-- > 0) */
                loop {
                    let old = *ndigitsp;
                    *ndigitsp = old - 1;
                    if !(old > 0) {
                        break;
                    }
                    js_putc(J, sbp, *bufp.offset(*ndigitsp as isize) as c_int);
                }
            } else {
                /* while (ndigits-- > 0) */
                loop {
                    let old = *ndigitsp;
                    *ndigitsp = old - 1;
                    if !(old > 0) {
                        break;
                    }
                    js_putc(J, sbp, *bufp.offset(*ndigitsp as isize) as c_int);
                    *pointp -= 1;
                    if *pointp == 0 && *ndigitsp > 0 {
                        js_putc(J, sbp, b'.' as c_int);
                    }
                }
                /* while (point-- > 0) */
                loop {
                    let old = *pointp;
                    *pointp = old - 1;
                    if !(old > 0) {
                        break;
                    }
                    js_putc(J, sbp, b'0' as c_int);
                }
            }

            js_putc(J, sbp, 0);
            let sbv = *sbp;
            js_pushstring(J, (*sbv).s.as_ptr());

            js_endtry(J);
        })
        .is_none()
        {
            js_free(J, sb as *mut c_void);
            js_throw(J);
        }

        js_free(J, sb as *mut c_void);
    }
}

/* Customized ToString() on a number */
unsafe fn numtostr(J: *mut js_State, fmt: *const c_char, w: c_int, n: f64) {
    /* buf needs to fit printf("%.20f", 1e20) */
    let mut buf = [0 as c_char; 50];
    let e: *mut c_char;
    sprintf(buf.as_mut_ptr(), fmt, w, n);
    e = strchr(buf.as_ptr(), b'e' as c_int);
    if !e.is_null() {
        let exp: c_int = atoi(e.offset(1));
        sprintf(e, c"e%+d".as_ptr(), exp);
    }
    js_pushstring(J, buf.as_ptr());
}

unsafe extern "C-unwind" fn Np_toFixed(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf = [0 as c_char; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
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

unsafe extern "C-unwind" fn Np_toExponential(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf = [0 as c_char; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
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

unsafe extern "C-unwind" fn Np_toPrecision(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf = [0 as c_char; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initnumber(J: *mut js_State) {
    (*(*J).Number_prototype).u.number = 0.0;

    js_pushobject(J, (*J).Number_prototype);
    {
        jsB_propf(
            J,
            c"Number.prototype.valueOf".as_ptr(),
            Some(Np_valueOf),
            0,
        );
        jsB_propf(
            J,
            c"Number.prototype.toString".as_ptr(),
            Some(Np_toString),
            1,
        );
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
