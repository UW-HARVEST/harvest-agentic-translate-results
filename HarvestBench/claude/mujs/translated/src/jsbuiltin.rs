//! Translated from jsbuiltin.c — global object init and helpers.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::*;
use crate::jsvalue::*;
use crate::types::*;
use std::os::raw::{c_char, c_int};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe fn jsB_globalf(J: *mut js_State, name: *const c_char, cfun: js_CFunction, n: c_int) {
    js_newcfunction(J, cfun, name, n);
    js_defglobal(J, name, JS_DONTENUM);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_propf(J: *mut js_State, name: *const c_char, cfun: js_CFunction, n: c_int) {
    let pname = strrchr(name, '.' as c_int);
    let pname = if !pname.is_null() { pname.add(1) } else { name };
    js_newcfunction(J, cfun, name, n);
    js_defproperty(J, -2, pname, JS_DONTENUM);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_propn(J: *mut js_State, name: *const c_char, number: f64) {
    js_pushnumber(J, number);
    js_defproperty(J, -2, name, JS_READONLY | JS_DONTENUM | JS_DONTCONF);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_props(J: *mut js_State, name: *const c_char, string: *const c_char) {
    js_pushliteral(J, string);
    js_defproperty(J, -2, name, JS_DONTENUM);
}

unsafe extern "C-unwind" fn jsB_parseInt(J: *mut js_State) {
    let mut s = js_tostring(J, 1);
    let mut radix = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { 0 };
    let mut sign = 1.0f64;
    let n: f64;
    let mut e: *mut c_char = std::ptr::null_mut();

    while crate::jslex::jsY_iswhite(*s as c_int) != 0 || crate::jslex::jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if *s == b'-' as c_char {
        s = s.add(1);
        sign = -1.0;
    } else if *s == b'+' as c_char {
        s = s.add(1);
    }
    if radix == 0 {
        radix = 10;
        if *s.add(0) == b'0' as c_char && (*s.add(1) == b'x' as c_char || *s.add(1) == b'X' as c_char) {
            s = s.add(2);
            radix = 16;
        }
    } else if radix < 2 || radix > 36 {
        js_pushnumber(J, f64::NAN);
        return;
    }
    n = js_strtol(s, &mut e, radix);
    if s == e as *const c_char {
        js_pushnumber(J, f64::NAN);
    } else {
        js_pushnumber(J, n * sign);
    }
}

unsafe extern "C-unwind" fn jsB_parseFloat(J: *mut js_State) {
    let mut s = js_tostring(J, 1);
    let mut e: *mut c_char = std::ptr::null_mut();
    let n: f64;

    while crate::jslex::jsY_iswhite(*s as c_int) != 0 || crate::jslex::jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if strncmp(s, cstr!("Infinity"), 8) == 0 {
        js_pushnumber(J, f64::INFINITY);
    } else if strncmp(s, cstr!("+Infinity"), 9) == 0 {
        js_pushnumber(J, f64::INFINITY);
    } else if strncmp(s, cstr!("-Infinity"), 9) == 0 {
        js_pushnumber(J, f64::NEG_INFINITY);
    } else {
        n = js_stringtofloat(s, &mut e);
        if e as *const c_char == s {
            js_pushnumber(J, f64::NAN);
        } else {
            js_pushnumber(J, n);
        }
    }
}

unsafe extern "C-unwind" fn jsB_isNaN(J: *mut js_State) {
    let n = js_tonumber(J, 1);
    js_pushboolean(J, n.is_nan() as c_int);
}

unsafe extern "C-unwind" fn jsB_isFinite(J: *mut js_State) {
    let n = js_tonumber(J, 1);
    js_pushboolean(J, n.is_finite() as c_int);
}

unsafe fn Encode(J: *mut js_State, str_: *const c_char, unescaped: *const c_char) {
    let str = str_;
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    static HEX: &[u8; 17] = b"0123456789ABCDEF\0";

    let str_ptr = str;
    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let caught = protect(J, || {
        let mut s = str_ptr;
        while *s != 0 {
            let c = *s as u8 as c_int;
            s = s.add(1);
            if !strchr(unescaped, c).is_null() {
                crate::jsintern::js_putc(J, sb_ptr, c);
            } else {
                crate::jsintern::js_putc(J, sb_ptr, '%' as c_int);
                crate::jsintern::js_putc(J, sb_ptr, HEX[((c >> 4) & 0xf) as usize] as c_int);
                crate::jsintern::js_putc(J, sb_ptr, HEX[(c & 0xf) as usize] as c_int);
            }
        }
        crate::jsintern::js_putc(J, sb_ptr, 0);
        js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("") });
    });
    if caught {
        js_free(J, sb as *mut _);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut _);
}

unsafe fn Decode(J: *mut js_State, str_: *const c_char, reserved: *const c_char) {
    let str = str_;
    let mut sb: *mut js_Buffer = std::ptr::null_mut();

    let str_ptr = str;
    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let caught = protect(J, || {
        let mut s = str_ptr;
        while *s != 0 {
            let mut c = *s as u8 as c_int;
            s = s.add(1);
            if c != '%' as c_int {
                crate::jsintern::js_putc(J, sb_ptr, c);
            } else {
                if *s.add(0) == 0 || *s.add(1) == 0 {
                    crate::jserror::js_urierror(J, cstr!("truncated escape sequence"));
                }
                let a = *s as c_int;
                s = s.add(1);
                let b = *s as c_int;
                s = s.add(1);
                if crate::jslex::jsY_ishex(a) == 0 || crate::jslex::jsY_ishex(b) == 0 {
                    crate::jserror::js_urierror(J, cstr!("invalid escape sequence"));
                }
                c = crate::jslex::jsY_tohex(a) << 4 | crate::jslex::jsY_tohex(b);
                if strchr(reserved, c).is_null() {
                    crate::jsintern::js_putc(J, sb_ptr, c);
                } else {
                    crate::jsintern::js_putc(J, sb_ptr, '%' as c_int);
                    crate::jsintern::js_putc(J, sb_ptr, a);
                    crate::jsintern::js_putc(J, sb_ptr, b);
                }
            }
        }
        crate::jsintern::js_putc(J, sb_ptr, 0);
        js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("") });
    });
    if caught {
        js_free(J, sb as *mut _);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut _);
}

const URIRESERVED: &str = ";/?:@&=+$,";
const URIALPHA: &str = "abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ";
const URIDIGIT: &str = "0123456789";
const URIMARK: &str = "-_.!~*'()";

unsafe extern "C-unwind" fn jsB_decodeURI(J: *mut js_State) {
    Decode(J, js_tostring(J, 1), cstr!(";/?:@&=+$,#"));
}

unsafe extern "C-unwind" fn jsB_decodeURIComponent(J: *mut js_State) {
    Decode(J, js_tostring(J, 1), cstr!(""));
}

unsafe extern "C-unwind" fn jsB_encodeURI(J: *mut js_State) {
    // URIUNESCAPED URIRESERVED "#"  = URIALPHA URIDIGIT URIMARK + reserved + '#'
    Encode(
        J,
        js_tostring(J, 1),
        cstr!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.!~*'();/?:@&=+$,#"),
    );
}

unsafe extern "C-unwind" fn jsB_encodeURIComponent(J: *mut js_State) {
    // URIUNESCAPED = URIALPHA URIDIGIT URIMARK
    Encode(
        J,
        js_tostring(J, 1),
        cstr!("abcdefghijklmnopqrstuvwxyzABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789-_.!~*'()"),
    );
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_init(J: *mut js_State) {
    (*J).Object_prototype = crate::jsproperty::jsV_newobject(J, JS_COBJECT, std::ptr::null_mut());
    (*J).Array_prototype = crate::jsproperty::jsV_newobject(J, JS_CARRAY, (*J).Object_prototype);
    (*J).Function_prototype = crate::jsproperty::jsV_newobject(J, JS_CCFUNCTION, (*J).Object_prototype);
    (*J).Boolean_prototype = crate::jsproperty::jsV_newobject(J, JS_CBOOLEAN, (*J).Object_prototype);
    (*J).Number_prototype = crate::jsproperty::jsV_newobject(J, JS_CNUMBER, (*J).Object_prototype);
    (*J).String_prototype = crate::jsproperty::jsV_newobject(J, JS_CSTRING, (*J).Object_prototype);
    (*J).Date_prototype = crate::jsproperty::jsV_newobject(J, JS_CDATE, (*J).Object_prototype);

    (*J).RegExp_prototype = crate::jsproperty::jsV_newobject(J, JS_CREGEXP, (*J).Object_prototype);
    (*(*J).RegExp_prototype).u.r.prog = crate::regexp::js_regcompx((*J).alloc, (*J).actx, cstr!("(?:)"), 0, std::ptr::null_mut()) as *mut std::os::raw::c_void;
    (*(*J).RegExp_prototype).u.r.source = js_strdup(J, cstr!("(?:)"));

    (*J).Error_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Object_prototype);
    (*J).EvalError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).RangeError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).ReferenceError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).SyntaxError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).TypeError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);
    (*J).URIError_prototype = crate::jsproperty::jsV_newobject(J, JS_CERROR, (*J).Error_prototype);

    crate::jsobject::jsB_initobject(J);
    crate::jsarray::jsB_initarray(J);
    crate::jsfunction::jsB_initfunction(J);
    crate::jsboolean::jsB_initboolean(J);
    crate::jsnumber::jsB_initnumber(J);
    crate::jsstring::jsB_initstring(J);
    crate::jsregexp::jsB_initregexp(J);
    crate::jsdate::jsB_initdate(J);
    crate::jserror::jsB_initerror(J);
    crate::jsmath::jsB_initmath(J);
    crate::json::jsB_initjson(J);

    js_pushnumber(J, f64::NAN);
    js_defglobal(J, cstr!("NaN"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    js_pushnumber(J, f64::INFINITY);
    js_defglobal(J, cstr!("Infinity"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    js_pushundefined(J);
    js_defglobal(J, cstr!("undefined"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);

    jsB_globalf(J, cstr!("parseInt"), Some(jsB_parseInt), 1);
    jsB_globalf(J, cstr!("parseFloat"), Some(jsB_parseFloat), 1);
    jsB_globalf(J, cstr!("isNaN"), Some(jsB_isNaN), 1);
    jsB_globalf(J, cstr!("isFinite"), Some(jsB_isFinite), 1);

    jsB_globalf(J, cstr!("decodeURI"), Some(jsB_decodeURI), 1);
    jsB_globalf(J, cstr!("decodeURIComponent"), Some(jsB_decodeURIComponent), 1);
    jsB_globalf(J, cstr!("encodeURI"), Some(jsB_encodeURI), 1);
    jsB_globalf(J, cstr!("encodeURIComponent"), Some(jsB_encodeURIComponent), 1);
}
