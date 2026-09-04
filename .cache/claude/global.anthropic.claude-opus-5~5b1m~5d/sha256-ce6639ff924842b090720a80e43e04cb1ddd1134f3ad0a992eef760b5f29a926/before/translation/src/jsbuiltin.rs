//! Translation of jsbuiltin.c

use crate::*;

unsafe fn jsB_globalf(J: *mut js_State, name: *const c_char, cfun: js_CFunction, n: c_int) {
    js_newcfunction(J, cfun, name, n);
    js_defglobal(J, name, JS_DONTENUM);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_propf(
    J: *mut js_State,
    name: *const c_char,
    cfun: js_CFunction,
    n: c_int,
) {
    let mut pname: *const c_char = strrchr(name, '.' as c_int);
    pname = if !pname.is_null() { pname.add(1) } else { name };
    js_newcfunction(J, cfun, name, n);
    js_defproperty(J, -2, pname, JS_DONTENUM);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_propn(J: *mut js_State, name: *const c_char, number: f64) {
    js_pushnumber(J, number);
    js_defproperty(J, -2, name, JS_READONLY | JS_DONTENUM | JS_DONTCONF);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_props(J: *mut js_State, name: *const c_char, string: *const c_char) {
    js_pushliteral(J, string);
    js_defproperty(J, -2, name, JS_DONTENUM);
}

unsafe extern "C" fn jsB_parseInt(J: *mut js_State) {
    let mut s: *const c_char = js_tostring(J, 1);
    let mut radix: c_int = if js_isdefined(J, 2) != 0 {
        js_tointeger(J, 2)
    } else {
        0
    };
    let mut sign: f64 = 1.0;
    let n: f64;
    let mut e: *mut c_char = null_mut();

    while jsY_iswhite(*s as c_int) != 0 || jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if *s == '-' as c_char {
        s = s.add(1);
        sign = -1.0;
    } else if *s == '+' as c_char {
        s = s.add(1);
    }
    if radix == 0 {
        radix = 10;
        if *s.add(0) == '0' as c_char
            && (*s.add(1) == 'x' as c_char || *s.add(1) == 'X' as c_char)
        {
            s = s.add(2);
            radix = 16;
        }
    } else if radix < 2 || radix > 36 {
        js_pushnumber(J, NAN);
        return;
    }
    n = js_strtol(s, &mut e, radix);
    if s == e as *const c_char {
        js_pushnumber(J, NAN);
    } else {
        js_pushnumber(J, n * sign);
    }
}

unsafe extern "C" fn jsB_parseFloat(J: *mut js_State) {
    let mut s: *const c_char = js_tostring(J, 1);
    let mut e: *mut c_char = null_mut();
    let n: f64;

    while jsY_iswhite(*s as c_int) != 0 || jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if strncmp(s, cs!("Infinity"), 8) == 0 {
        js_pushnumber(J, INFINITY);
    } else if strncmp(s, cs!("+Infinity"), 9) == 0 {
        js_pushnumber(J, INFINITY);
    } else if strncmp(s, cs!("-Infinity"), 9) == 0 {
        js_pushnumber(J, -INFINITY);
    } else {
        n = js_stringtofloat(s, &mut e);
        if e as *const c_char == s {
            js_pushnumber(J, NAN);
        } else {
            js_pushnumber(J, n);
        }
    }
}

unsafe extern "C" fn jsB_isNaN(J: *mut js_State) {
    let n: f64 = js_tonumber(J, 1);
    js_pushboolean(J, isnan(n) as c_int);
}

unsafe extern "C" fn jsB_isFinite(J: *mut js_State) {
    let n: f64 = js_tonumber(J, 1);
    js_pushboolean(J, isfinite(n) as c_int);
}

unsafe fn Encode(J: *mut js_State, str_: *const c_char, unescaped: *const c_char) {
    /* NOTE: volatile to silence GCC warning about longjmp clobbering a variable */
    let mut str: *const c_char = str_;
    let mut sb: *mut js_Buffer = null_mut();

    let HEX: *const c_char = cs!("0123456789ABCDEF");

    if js_try!(J) != 0 {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }

    while *vol!(str) != 0 {
        let c: c_int = *vol!(str) as u8 as c_int;
        setvol!(str, vol!(str).add(1));
        if !strchr(unescaped, c).is_null() {
            js_putc(J, addr_of_mut!(sb), c);
        } else {
            js_putc(J, addr_of_mut!(sb), '%' as c_int);
            js_putc(
                J,
                addr_of_mut!(sb),
                *HEX.offset(((c >> 4) & 0xf) as isize) as c_int,
            );
            js_putc(J, addr_of_mut!(sb), *HEX.offset((c & 0xf) as isize) as c_int);
        }
    }
    js_putc(J, addr_of_mut!(sb), 0);

    js_pushstring(
        J,
        if !sb.is_null() {
            addr_of_mut!((*sb).s) as *const c_char
        } else {
            cs!("")
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

unsafe fn Decode(J: *mut js_State, str_: *const c_char, reserved: *const c_char) {
    /* NOTE: volatile to silence GCC warning about longjmp clobbering a variable */
    let mut str: *const c_char = str_;
    let mut sb: *mut js_Buffer = null_mut();
    let mut a: c_int;
    let mut b: c_int;

    if js_try!(J) != 0 {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }

    while *vol!(str) != 0 {
        let mut c: c_int = *vol!(str) as u8 as c_int;
        setvol!(str, vol!(str).add(1));
        if c != '%' as c_int {
            js_putc(J, addr_of_mut!(sb), c);
        } else {
            if *vol!(str).offset(0) == 0 || *vol!(str).offset(1) == 0 {
                js_urierror!(J, "truncated escape sequence");
            }
            a = *vol!(str) as c_int;
            setvol!(str, vol!(str).add(1));
            b = *vol!(str) as c_int;
            setvol!(str, vol!(str).add(1));
            if jsY_ishex(a) == 0 || jsY_ishex(b) == 0 {
                js_urierror!(J, "invalid escape sequence");
            }
            c = jsY_tohex(a) << 4 | jsY_tohex(b);
            if strchr(reserved, c).is_null() {
                js_putc(J, addr_of_mut!(sb), c);
            } else {
                js_putc(J, addr_of_mut!(sb), '%' as c_int);
                js_putc(J, addr_of_mut!(sb), a);
                js_putc(J, addr_of_mut!(sb), b);
            }
        }
    }
    js_putc(J, addr_of_mut!(sb), 0);

    js_pushstring(
        J,
        if !sb.is_null() {
            addr_of_mut!((*sb).s) as *const c_char
        } else {
            cs!("")
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

/* URIRESERVED ";/?:@&=+$,"
 * URIALPHA "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ"
 * URIDIGIT "0123456789"
 * URIMARK "-_.!~*'()"
 * URIUNESCAPED URIALPHA URIDIGIT URIMARK
 */

unsafe extern "C" fn jsB_decodeURI(J: *mut js_State) {
    /* URIRESERVED "#" */
    Decode(J, js_tostring(J, 1), cs!(";/?:@&=+$,#"));
}

unsafe extern "C" fn jsB_decodeURIComponent(J: *mut js_State) {
    Decode(J, js_tostring(J, 1), cs!(""));
}

unsafe extern "C" fn jsB_encodeURI(J: *mut js_State) {
    /* URIUNESCAPED URIRESERVED "#" */
    Encode(
        J,
        js_tostring(J, 1),
        cs!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.!~*'();/?:@&=+$,#"),
    );
}

unsafe extern "C" fn jsB_encodeURIComponent(J: *mut js_State) {
    /* URIUNESCAPED */
    Encode(
        J,
        js_tostring(J, 1),
        cs!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.!~*'()"),
    );
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_init(J: *mut js_State) {
    /* Create the prototype objects here, before the constructors */
    (*J).Object_prototype = jsV_newobject(J, JS_COBJECT, null_mut());
    (*J).Array_prototype = jsV_newobject(J, JS_CARRAY, (*J).Object_prototype);
    (*J).Function_prototype = jsV_newobject(J, JS_CCFUNCTION, (*J).Object_prototype);
    (*J).Boolean_prototype = jsV_newobject(J, JS_CBOOLEAN, (*J).Object_prototype);
    (*J).Number_prototype = jsV_newobject(J, JS_CNUMBER, (*J).Object_prototype);
    (*J).String_prototype = jsV_newobject(J, JS_CSTRING, (*J).Object_prototype);
    (*J).Date_prototype = jsV_newobject(J, JS_CDATE, (*J).Object_prototype);

    (*J).RegExp_prototype = jsV_newobject(J, JS_CREGEXP, (*J).Object_prototype);
    (*(*J).RegExp_prototype).u.r.prog =
        js_regcompx((*J).alloc, (*J).actx, cs!("(?:)"), 0, null_mut()) as *mut c_void;
    (*(*J).RegExp_prototype).u.r.source = js_strdup(J, cs!("(?:)"));

    /* All the native error types */
    (*J).Error_prototype = jsV_newobject(J, JS_CERROR, (*J).Object_prototype);
    (*J).EvalError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).RangeError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).ReferenceError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).SyntaxError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).TypeError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).URIError_prototype = jsV_newobject(J, JS_CERROR, (*J).Error_prototype);

    /* Create the constructors and fill out the prototype objects */
    jsB_initobject(J);
    jsB_initarray(J);
    jsB_initfunction(J);
    jsB_initboolean(J);
    jsB_initnumber(J);
    jsB_initstring(J);
    jsB_initregexp(J);
    jsB_initdate(J);
    jsB_initerror(J);
    jsB_initmath(J);
    jsB_initjson(J);

    /* Initialize the global object */
    js_pushnumber(J, NAN);
    js_defglobal(J, cs!("NaN"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    js_pushnumber(J, INFINITY);
    js_defglobal(J, cs!("Infinity"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    js_pushundefined(J);
    js_defglobal(J, cs!("undefined"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    jsB_globalf(J, cs!("parseInt"), Some(jsB_parseInt), 1);
    jsB_globalf(J, cs!("parseFloat"), Some(jsB_parseFloat), 1);
    jsB_globalf(J, cs!("isNaN"), Some(jsB_isNaN), 1);
    jsB_globalf(J, cs!("isFinite"), Some(jsB_isFinite), 1);

    jsB_globalf(J, cs!("decodeURI"), Some(jsB_decodeURI), 1);
    jsB_globalf(
        J,
        cs!("decodeURIComponent"),
        Some(jsB_decodeURIComponent),
        1,
    );
    jsB_globalf(J, cs!("encodeURI"), Some(jsB_encodeURI), 1);
    jsB_globalf(
        J,
        cs!("encodeURIComponent"),
        Some(jsB_encodeURIComponent),
        1,
    );
}
