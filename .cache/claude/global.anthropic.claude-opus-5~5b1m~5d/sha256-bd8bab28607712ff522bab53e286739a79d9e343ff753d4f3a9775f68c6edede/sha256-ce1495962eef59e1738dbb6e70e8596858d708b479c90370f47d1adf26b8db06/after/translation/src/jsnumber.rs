//! Translation of jsnumber.c

use crate::*;

extern "C" {
    fn atoi(s: *const c_char) -> c_int;
}

unsafe extern "C" fn jsB_new_Number(J: *mut js_State) {
    js_newnumber(
        J,
        if js_gettop(J) > 1 {
            js_tonumber(J, 1)
        } else {
            0.0
        },
    );
}

unsafe extern "C" fn jsB_Number(J: *mut js_State) {
    js_pushnumber(
        J,
        if js_gettop(J) > 1 {
            js_tonumber(J, 1)
        } else {
            0.0
        },
    );
}

unsafe extern "C" fn Np_valueOf(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, "not a number");
    }
    js_pushnumber(J, (*self_).u.number);
}

unsafe extern "C" fn Np_toString(J: *mut js_State) {
    let mut buf: [c_char; 100] = [0; 100];
    let self_: *mut js_Object = js_toobject(J, 0);
    let radix: c_int = if js_isundefined(J, 1) != 0 {
        10
    } else {
        js_tointeger(J, 1)
    };
    let mut x: f64 = 0.0;
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, "not a number");
    }
    x = (*self_).u.number;
    if radix == 10 {
        js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
        return;
    }
    if radix < 2 || radix > 36 {
        js_rangeerror!(J, "invalid radix");
    }

    /* lame number to string conversion for any radix from 2 to 36 */
    {
        const digits: &[u8; 36] = b"0123456789abcdefghijklmnopqrstuvwxyz";
        let mut number: f64 = x;
        let sign: c_int = (x < 0.0) as c_int;
        let mut sb: *mut js_Buffer = null_mut();
        let mut u: u64;
        let limit: u64 = (1u64) << 52;

        let mut ndigits: c_int;
        let mut exp: c_int;
        let mut point: c_int;

        if number == 0.0 {
            js_pushstring(J, cs!("0"));
            return;
        }
        if isnan(number) {
            js_pushstring(J, cs!("NaN"));
            return;
        }
        if isinf(number) {
            js_pushstring(
                J,
                if sign != 0 {
                    cs!("-Infinity")
                } else {
                    cs!("Infinity")
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
            buf[ndigits as usize] = digits[(u % radix as u64) as usize] as c_char;
            ndigits += 1;
            u /= radix as u64;
        }
        point = ndigits - exp;

        if js_try!(J) != 0 {
            js_free(J, vol!(sb) as *mut c_void);
            js_throw(J);
        }

        if sign != 0 {
            js_putc(J, addr_of_mut!(sb), '-' as c_int);
        }

        if point <= 0 {
            js_putc(J, addr_of_mut!(sb), '0' as c_int);
            js_putc(J, addr_of_mut!(sb), '.' as c_int);
            loop {
                let t = point;
                point += 1;
                if !(t < 0) {
                    break;
                }
                js_putc(J, addr_of_mut!(sb), '0' as c_int);
            }
            loop {
                let t = ndigits;
                ndigits -= 1;
                if !(t > 0) {
                    break;
                }
                js_putc(J, addr_of_mut!(sb), buf[ndigits as usize] as c_int);
            }
        } else {
            loop {
                let t = ndigits;
                ndigits -= 1;
                if !(t > 0) {
                    break;
                }
                js_putc(J, addr_of_mut!(sb), buf[ndigits as usize] as c_int);
                point -= 1;
                if point == 0 && ndigits > 0 {
                    js_putc(J, addr_of_mut!(sb), '.' as c_int);
                }
            }
            loop {
                let t = point;
                point -= 1;
                if !(t > 0) {
                    break;
                }
                js_putc(J, addr_of_mut!(sb), '0' as c_int);
            }
        }

        js_putc(J, addr_of_mut!(sb), 0);
        js_pushstring(J, addr_of!((*sb).s) as *const c_char);

        js_endtry(J);
        js_free(J, sb as *mut c_void);
    }
}

/* Customized ToString() on a number */
unsafe fn numtostr(J: *mut js_State, fmt: *const c_char, w: c_int, n: f64) {
    /* buf needs to fit printf("%.20f", 1e20) */
    let mut buf: [c_char; 50] = [0; 50];
    let e: *mut c_char;
    sprintf(buf.as_mut_ptr(), fmt, w, n);
    e = strchr(buf.as_ptr(), 'e' as c_int);
    if !e.is_null() {
        let exp: c_int = atoi(e.add(1));
        sprintf(e, cs!("e%+d"), exp);
    }
    js_pushstring(J, buf.as_ptr());
}

unsafe extern "C" fn Np_toFixed(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, "not a number");
    }
    if width < 0 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    if width > 20 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    x = (*self_).u.number;
    if isnan(x) || isinf(x) || x <= -1e21 || x >= 1e21 {
        js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cs!("%.*f"), width, x);
    }
}

unsafe extern "C" fn Np_toExponential(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, "not a number");
    }
    if width < 0 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    if width > 20 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    x = (*self_).u.number;
    if isnan(x) || isinf(x) {
        js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cs!("%.*e"), width, x);
    }
}

unsafe extern "C" fn Np_toPrecision(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    let width: c_int = js_tointeger(J, 1);
    let mut buf: [c_char; 32] = [0; 32];
    let x: f64;
    if (*self_).type_ != JS_CNUMBER {
        js_typeerror!(J, "not a number");
    }
    if width < 1 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    if width > 21 {
        js_rangeerror!(J, "precision %d out of range", width);
    }
    x = (*self_).u.number;
    if isnan(x) || isinf(x) {
        js_pushstring(J, jsV_numbertostring(J, buf.as_mut_ptr(), x));
    } else {
        numtostr(J, cs!("%.*g"), width, x);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initnumber(J: *mut js_State) {
    (*(*J).Number_prototype).u.number = 0.0;

    js_pushobject(J, (*J).Number_prototype);
    {
        jsB_propf(J, cs!("Number.prototype.valueOf"), Some(Np_valueOf), 0);
        jsB_propf(J, cs!("Number.prototype.toString"), Some(Np_toString), 1);
        jsB_propf(J, cs!("Number.prototype.toLocaleString"), Some(Np_toString), 0);
        jsB_propf(J, cs!("Number.prototype.toFixed"), Some(Np_toFixed), 1);
        jsB_propf(
            J,
            cs!("Number.prototype.toExponential"),
            Some(Np_toExponential),
            1,
        );
        jsB_propf(
            J,
            cs!("Number.prototype.toPrecision"),
            Some(Np_toPrecision),
            1,
        );
    }
    js_newcconstructor(
        J,
        Some(jsB_Number),
        Some(jsB_new_Number),
        cs!("Number"),
        0,
    ); /* 1 */
    {
        jsB_propn(J, cs!("MAX_VALUE"), 1.7976931348623157e+308);
        jsB_propn(J, cs!("MIN_VALUE"), 5e-324);
        jsB_propn(J, cs!("NaN"), NAN);
        jsB_propn(J, cs!("NEGATIVE_INFINITY"), -INFINITY);
        jsB_propn(J, cs!("POSITIVE_INFINITY"), INFINITY);
    }
    js_defglobal(J, cs!("Number"), JS_DONTENUM);
}
