//! Translated from jsvalue.c — value conversions and object/value constructors.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::*;
use crate::types::*;
use std::os::raw::{c_char, c_int, c_uint, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const SHRLEN: c_int = 15; // soffsetof(js_Value, t.type)

#[inline]
unsafe fn JSV_ISSTRING(v: *mut js_Value) -> bool {
    let t = (*v).type_();
    t == JS_TSHRSTR || t == JS_TMEMSTR || t == JS_TLITSTR
}

#[inline]
unsafe fn JSV_TOSTRING(v: *mut js_Value) -> *const c_char {
    match (*v).type_() {
        JS_TSHRSTR => (*v).u.shrstr.as_ptr(),
        JS_TLITSTR => (*v).u.litstr,
        JS_TMEMSTR => (*(*v).u.memstr).p.as_ptr(),
        _ => cstr!(""),
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_strtol(s: *const c_char, p: *mut *mut c_char, base: c_int) -> f64 {
    static TABLE: [u8; 256] = [
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 80, 80, 80, 80, 80, 80,
        80, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 80, 80, 80, 80, 80,
        80, 10, 11, 12, 13, 14, 15, 16, 17, 18, 19, 20, 21, 22, 23, 24,
        25, 26, 27, 28, 29, 30, 31, 32, 33, 34, 35, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
        80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80, 80,
    ];
    let mut x: f64;
    let mut c: u8;
    let mut s = s;
    if base == 10 {
        x = 0.0;
        c = *s as u8;
        s = s.add(1);
        while (c.wrapping_sub(b'0')) < 10 {
            x = x * 10.0 + (c - b'0') as f64;
            c = *s as u8;
            s = s.add(1);
        }
    } else {
        x = 0.0;
        c = *s as u8;
        s = s.add(1);
        while (TABLE[c as usize] as c_int) < base {
            x = x * base as f64 + TABLE[c as usize] as f64;
            c = *s as u8;
            s = s.add(1);
        }
    }
    if !p.is_null() {
        *p = s.offset(-1) as *mut c_char;
    }
    x
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertointeger(n: f64) -> c_int {
    if n == 0.0 {
        return 0;
    }
    if n.is_nan() {
        return 0;
    }
    let n = if n < 0.0 { -((-n).floor()) } else { n.floor() };
    if n < c_int::MIN as f64 {
        return c_int::MIN;
    }
    if n > c_int::MAX as f64 {
        return c_int::MAX;
    }
    // jsvalue.c:45 `return (int)n;` — explicit cast to `int`, but provably in range: NaN
    // returned above, n has been truncated to an integer, and n now satisfies
    // INT_MIN <= n <= INT_MAX (both bounds are exactly representable as doubles), so plain
    // `as` is identical to C.
    n as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertoint32(n: f64) -> c_int {
    let two32 = 4294967296.0;
    let two31 = 2147483648.0;
    if !n.is_finite() || n == 0.0 {
        return 0;
    }
    let mut n = n % two32;
    n = if n >= 0.0 { n.floor() } else { n.ceil() + two32 };
    // jsvalue.c:60/62 `if (n >= two31) return n - two32; else return n;` — implicit
    // double -> `int` (return type `int`), but provably in range: non-finite n returned above,
    // and after the fmod/floor/ceil normalisation n is an integer in [0, 2^32], so the taken
    // branch yields [-2^31, 0) resp. [0, 2^31). Plain `as` is identical to C.
    if n >= two31 {
        (n - two32) as c_int
    } else {
        n as c_int
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertouint32(n: f64) -> c_uint {
    jsV_numbertoint32(n) as c_uint
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertoint16(n: f64) -> i16 {
    jsV_numbertoint32(n) as i16
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertouint16(n: f64) -> u16 {
    jsV_numbertoint32(n) as u16
}

unsafe fn jsV_toString(J: *mut js_State, obj: *mut js_Object) -> c_int {
    js_pushobject(J, obj);
    js_getproperty(J, -1, cstr!("toString"));
    if js_iscallable(J, -1) != 0 {
        js_rot2(J);
        js_call(J, 0);
        if js_isprimitive(J, -1) != 0 {
            return 1;
        }
        js_pop(J, 1);
        return 0;
    }
    js_pop(J, 2);
    0
}

unsafe fn jsV_valueOf(J: *mut js_State, obj: *mut js_Object) -> c_int {
    js_pushobject(J, obj);
    js_getproperty(J, -1, cstr!("valueOf"));
    if js_iscallable(J, -1) != 0 {
        js_rot2(J);
        js_call(J, 0);
        if js_isprimitive(J, -1) != 0 {
            return 1;
        }
        js_pop(J, 1);
        return 0;
    }
    js_pop(J, 2);
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_toprimitive(J: *mut js_State, v: *mut js_Value, mut preferred: c_int) {
    if (*v).type_() != JS_TOBJECT {
        return;
    }
    let obj = (*v).u.object;

    if preferred == JS_HNONE {
        preferred = if (*obj).type_ == JS_CDATE { JS_HSTRING } else { JS_HNUMBER };
    }

    if preferred == JS_HSTRING {
        if jsV_toString(J, obj) != 0 || jsV_valueOf(J, obj) != 0 {
            *v = *js_tovalue(J, -1);
            js_pop(J, 1);
            return;
        }
    } else {
        if jsV_valueOf(J, obj) != 0 || jsV_toString(J, obj) != 0 {
            *v = *js_tovalue(J, -1);
            js_pop(J, 1);
            return;
        }
    }

    if (*J).strict != 0 {
        crate::jserror::js_typeerror(J, cstr!("cannot convert object to primitive"));
    }

    (*v).set_type(JS_TLITSTR);
    (*v).u.litstr = cstr!("[object]");
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_toboolean(J: *mut js_State, v: *mut js_Value) -> c_int {
    match (*v).type_() {
        JS_TSHRSTR => ((*v).u.shrstr[0] != 0) as c_int,
        JS_TUNDEFINED => 0,
        JS_TNULL => 0,
        JS_TBOOLEAN => (*v).u.boolean,
        JS_TNUMBER => ((*v).u.number != 0.0 && !(*v).u.number.is_nan()) as c_int,
        JS_TLITSTR => (*(*v).u.litstr != 0) as c_int,
        JS_TMEMSTR => (*(*(*v).u.memstr).p.as_ptr() != 0) as c_int,
        JS_TOBJECT => 1,
        _ => ((*v).u.shrstr[0] != 0) as c_int,
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_itoa(out: *mut c_char, v: c_int) -> *const c_char {
    let mut buf: [c_char; 32] = [0; 32];
    let mut s = out;
    let a: c_uint;
    let mut i = 0usize;
    if v < 0 {
        a = (v as c_uint).wrapping_neg();
        *s = b'-' as c_char;
        s = s.add(1);
    } else {
        a = v as c_uint;
    }
    let mut a = a;
    while a != 0 {
        buf[i] = ((a % 10) as u8 + b'0') as c_char;
        i += 1;
        a /= 10;
    }
    if i == 0 {
        buf[i] = b'0' as c_char;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        *s = buf[i];
        s = s.add(1);
    }
    *s = 0;
    out
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_stringtofloat(s: *const c_char, ep: *mut *mut c_char) -> f64 {
    let mut end: *mut c_char = std::ptr::null_mut();
    let n: f64;
    let mut e = s;
    let mut isflt = 0;
    if *e == b'+' as c_char || *e == b'-' as c_char {
        e = e.add(1);
    }
    while *e >= b'0' as c_char && *e <= b'9' as c_char {
        e = e.add(1);
    }
    if *e == b'.' as c_char {
        e = e.add(1);
        isflt = 1;
    }
    while *e >= b'0' as c_char && *e <= b'9' as c_char {
        e = e.add(1);
    }
    if *e == b'e' as c_char || *e == b'E' as c_char {
        e = e.add(1);
        if *e == b'+' as c_char || *e == b'-' as c_char {
            e = e.add(1);
        }
        while *e >= b'0' as c_char && *e <= b'9' as c_char {
            e = e.add(1);
        }
        isflt = 1;
    }
    if isflt != 0 {
        n = crate::jsdtoa::js_strtod(s, &mut end);
    } else {
        if *s == b'-' as c_char {
            n = -js_strtol(s.add(1), &mut end, 10);
        } else if *s == b'+' as c_char {
            n = js_strtol(s.add(1), &mut end, 10);
        } else {
            n = js_strtol(s, &mut end, 10);
        }
    }
    if end == e as *mut c_char {
        *ep = e as *mut c_char;
        return n;
    }
    *ep = s as *mut c_char;
    0.0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_stringtonumber(J: *mut js_State, s: *const c_char) -> f64 {
    let mut e: *mut c_char = std::ptr::null_mut();
    let n: f64;
    let mut s = s;
    while crate::jslex::jsY_iswhite(*s as c_int) != 0 || crate::jslex::jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if *s.add(0) == b'0' as c_char
        && (*s.add(1) == b'x' as c_char || *s.add(1) == b'X' as c_char)
        && *s.add(2) != 0
    {
        n = js_strtol(s.add(2), &mut e, 16);
    } else if strncmp(s, cstr!("Infinity"), 8) == 0 {
        n = f64::INFINITY;
        e = s.add(8) as *mut c_char;
    } else if strncmp(s, cstr!("+Infinity"), 9) == 0 {
        n = f64::INFINITY;
        e = s.add(9) as *mut c_char;
    } else if strncmp(s, cstr!("-Infinity"), 9) == 0 {
        n = f64::NEG_INFINITY;
        e = s.add(9) as *mut c_char;
    } else {
        n = js_stringtofloat(s, &mut e);
    }
    while crate::jslex::jsY_iswhite(*e as c_int) != 0 || crate::jslex::jsY_isnewline(*e as c_int) != 0 {
        e = e.add(1);
    }
    if *e != 0 {
        return f64::NAN;
    }
    n
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_tonumber(J: *mut js_State, v: *mut js_Value) -> f64 {
    match (*v).type_() {
        JS_TSHRSTR => jsV_stringtonumber(J, (*v).u.shrstr.as_ptr()),
        JS_TUNDEFINED => f64::NAN,
        JS_TNULL => 0.0,
        JS_TBOOLEAN => (*v).u.boolean as f64,
        JS_TNUMBER => (*v).u.number,
        JS_TLITSTR => jsV_stringtonumber(J, (*v).u.litstr),
        JS_TMEMSTR => jsV_stringtonumber(J, (*(*v).u.memstr).p.as_ptr()),
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HNUMBER);
            jsV_tonumber(J, v)
        }
        _ => jsV_stringtonumber(J, (*v).u.shrstr.as_ptr()),
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_tointeger(J: *mut js_State, v: *mut js_Value) -> f64 {
    jsV_numbertointeger(jsV_tonumber(J, v)) as f64
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_numbertostring(J: *mut js_State, buf: *mut c_char, f: f64) -> *const c_char {
    let mut digits: [c_char; 32] = [0; 32];
    let mut p = buf;
    let mut s = digits.as_mut_ptr();
    let mut exp: c_int = 0;
    let mut ndigits: c_int;
    let mut point: c_int;

    if f == 0.0 {
        return cstr!("0");
    }
    if f.is_nan() {
        return cstr!("NaN");
    }
    if f.is_infinite() {
        return if f < 0.0 { cstr!("-Infinity") } else { cstr!("Infinity") };
    }

    if f >= c_int::MIN as f64 && f <= c_int::MAX as f64 {
        // jsvalue.c:283 `int i = (int)f;` — explicit cast to `int`, provably in range: NaN and
        // infinities returned above and the guard restricts f to [INT_MIN, INT_MAX] (both
        // exactly representable as doubles), so plain `as` is identical to C.
        let i = f as c_int;
        if i as f64 == f {
            return js_itoa(buf, i);
        }
    }

    ndigits = crate::jsdtoa::js_grisu2(f, digits.as_mut_ptr(), &mut exp);
    point = ndigits + exp;

    if f.is_sign_negative() {
        *p = b'-' as c_char;
        p = p.add(1);
    }

    if point < -5 || point > 21 {
        *p = *s;
        p = p.add(1);
        s = s.add(1);
        if ndigits > 1 {
            let mut n = ndigits - 1;
            *p = b'.' as c_char;
            p = p.add(1);
            while n != 0 {
                *p = *s;
                p = p.add(1);
                s = s.add(1);
                n -= 1;
            }
        }
        crate::jsdtoa::js_fmtexp(p, point - 1);
    } else if point <= 0 {
        *p = b'0' as c_char;
        p = p.add(1);
        *p = b'.' as c_char;
        p = p.add(1);
        while point < 0 {
            *p = b'0' as c_char;
            p = p.add(1);
            point += 1;
        }
        while ndigits > 0 {
            *p = *s;
            p = p.add(1);
            s = s.add(1);
            ndigits -= 1;
        }
        *p = 0;
    } else {
        while ndigits > 0 {
            *p = *s;
            p = p.add(1);
            s = s.add(1);
            ndigits -= 1;
            point -= 1;
            if point == 0 && ndigits > 0 {
                *p = b'.' as c_char;
                p = p.add(1);
            }
        }
        while point > 0 {
            *p = b'0' as c_char;
            p = p.add(1);
            point -= 1;
        }
        *p = 0;
    }

    buf
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_tostring(J: *mut js_State, v: *mut js_Value) -> *const c_char {
    let mut buf: [c_char; 32] = [0; 32];
    match (*v).type_() {
        JS_TSHRSTR => (*v).u.shrstr.as_ptr(),
        JS_TUNDEFINED => cstr!("undefined"),
        JS_TNULL => cstr!("null"),
        JS_TBOOLEAN => {
            if (*v).u.boolean != 0 {
                cstr!("true")
            } else {
                cstr!("false")
            }
        }
        JS_TLITSTR => (*v).u.litstr,
        JS_TMEMSTR => (*(*v).u.memstr).p.as_ptr(),
        JS_TNUMBER => {
            let p = jsV_numbertostring(J, buf.as_mut_ptr(), (*v).u.number);
            if p == buf.as_ptr() {
                let n = strlen(p) as c_int;
                if n <= SHRLEN {
                    let mut d = (*v).u.shrstr.as_mut_ptr();
                    let mut pp = p;
                    let mut nn = n;
                    while nn > 0 {
                        *d = *pp;
                        d = d.add(1);
                        pp = pp.add(1);
                        nn -= 1;
                    }
                    *d = 0;
                    (*v).set_type(JS_TSHRSTR);
                    (*v).u.shrstr.as_ptr()
                } else {
                    (*v).u.memstr = jsV_newmemstring(J, p, n);
                    (*v).set_type(JS_TMEMSTR);
                    (*(*v).u.memstr).p.as_ptr()
                }
            } else {
                p
            }
        }
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HSTRING);
            jsV_tostring(J, v)
        }
        _ => (*v).u.shrstr.as_ptr(),
    }
}

/* Objects */
unsafe fn jsV_newboolean(J: *mut js_State, v: c_int) -> *mut js_Object {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CBOOLEAN, (*J).Boolean_prototype);
    (*obj).u.boolean = v;
    obj
}

unsafe fn jsV_newnumber(J: *mut js_State, v: f64) -> *mut js_Object {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CNUMBER, (*J).Number_prototype);
    (*obj).u.number = v;
    obj
}

unsafe fn jsV_newstring(J: *mut js_State, v: *const c_char) -> *mut js_Object {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CSTRING, (*J).String_prototype);
    let n = strlen(v);
    if n < std::mem::size_of::<[c_char; 16]>() {
        (*obj).u.s.string = (*obj).u.s.shrstr.as_mut_ptr();
        memcpy((*obj).u.s.shrstr.as_mut_ptr(), v, n + 1);
    } else {
        (*obj).u.s.string = js_strdup(J, v);
    }
    (*obj).u.s.length = crate::jsstring::js_utflen(v);
    obj
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsV_toobject(J: *mut js_State, v: *mut js_Value) -> *mut js_Object {
    let o: *mut js_Object;
    match (*v).type_() {
        JS_TUNDEFINED => {
            crate::jserror::js_typeerror(J, cstr!("cannot convert undefined to object"));
            unreachable!()
        }
        JS_TNULL => {
            crate::jserror::js_typeerror(J, cstr!("cannot convert null to object"));
            unreachable!()
        }
        JS_TOBJECT => return (*v).u.object,
        JS_TSHRSTR => o = jsV_newstring(J, (*v).u.shrstr.as_ptr()),
        JS_TLITSTR => o = jsV_newstring(J, (*v).u.litstr),
        JS_TMEMSTR => o = jsV_newstring(J, (*(*v).u.memstr).p.as_ptr()),
        JS_TBOOLEAN => o = jsV_newboolean(J, (*v).u.boolean),
        JS_TNUMBER => o = jsV_newnumber(J, (*v).u.number),
        _ => {
            o = jsV_newstring(J, (*v).u.shrstr.as_ptr());
        }
    }
    (*v).set_type(JS_TOBJECT);
    (*v).u.object = o;
    o
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newobjectx(J: *mut js_State) {
    let mut prototype: *mut js_Object = std::ptr::null_mut();
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_COBJECT, prototype));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newobject(J: *mut js_State) {
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_COBJECT, (*J).Object_prototype));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newarguments(J: *mut js_State) {
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_CARGUMENTS, (*J).Object_prototype));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newarray(J: *mut js_State) {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CARRAY, (*J).Array_prototype);
    (*obj).u.a.simple = 1;
    js_pushobject(J, obj);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newboolean(J: *mut js_State, v: c_int) {
    js_pushobject(J, jsV_newboolean(J, v));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newnumber(J: *mut js_State, v: f64) {
    js_pushobject(J, jsV_newnumber(J, v));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newstring(J: *mut js_State, v: *const c_char) {
    js_pushobject(J, jsV_newstring(J, v));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newfunction(J: *mut js_State, fun: *mut js_Function, scope: *mut js_Environment) {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CFUNCTION, (*J).Function_prototype);
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
    {
        js_pushnumber(J, (*fun).numparams as f64);
        js_defproperty(J, -2, cstr!("length"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, cstr!("constructor"), JS_DONTENUM);
        }
        js_defproperty(J, -2, cstr!("prototype"), JS_DONTENUM | JS_DONTCONF);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newscript(J: *mut js_State, fun: *mut js_Function, scope: *mut js_Environment) {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CSCRIPT, std::ptr::null_mut());
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newcfunctionx(
    J: *mut js_State,
    cfun: js_CFunction,
    name: *const c_char,
    length: c_int,
    data: *mut c_void,
    finalize: js_Finalize,
) {
    let obj: *mut js_Object;

    let caught = protect(J, || {
        // body: create object
    });
    // The C uses js_try only to run finalize on throw; nothing in the body
    // throws before js_endtry, so replicate exactly:
    if caught {
        if let Some(f) = finalize {
            f(J, data);
        }
        js_throw(J);
    }
    js_endtry(J);

    obj = crate::jsproperty::jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = None;
    (*obj).u.c.length = length;
    (*obj).u.c.data = data;
    (*obj).u.c.finalize = finalize;

    js_pushobject(J, obj);
    {
        js_pushnumber(J, length as f64);
        js_defproperty(J, -2, cstr!("length"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, cstr!("constructor"), JS_DONTENUM);
        }
        js_defproperty(J, -2, cstr!("prototype"), JS_DONTENUM | JS_DONTCONF);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newcfunction(J: *mut js_State, cfun: js_CFunction, name: *const c_char, length: c_int) {
    js_newcfunctionx(J, cfun, name, length, std::ptr::null_mut(), None);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newcconstructor(
    J: *mut js_State,
    cfun: js_CFunction,
    ccon: js_CFunction,
    name: *const c_char,
    length: c_int,
) {
    let obj = crate::jsproperty::jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = ccon;
    (*obj).u.c.length = length;
    js_pushobject(J, obj);
    {
        js_pushnumber(J, length as f64);
        js_defproperty(J, -2, cstr!("length"), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_rot2(J);
        js_copy(J, -2);
        js_defproperty(J, -2, cstr!("constructor"), JS_DONTENUM);
        js_defproperty(J, -2, cstr!("prototype"), JS_DONTENUM | JS_DONTCONF);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newuserdatax(
    J: *mut js_State,
    tag: *const c_char,
    data: *mut c_void,
    has: js_HasProperty,
    put: js_Put,
    del: js_Delete,
    finalize: js_Finalize,
) {
    let mut prototype: *mut js_Object = std::ptr::null_mut();
    let obj: *mut js_Object;

    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);

    let caught = protect(J, || {});
    if caught {
        if let Some(f) = finalize {
            f(J, data);
        }
        js_throw(J);
    }
    js_endtry(J);

    obj = crate::jsproperty::jsV_newobject(J, JS_CUSERDATA, prototype);
    (*obj).u.user.tag = tag;
    (*obj).u.user.data = data;
    (*obj).u.user.has = has;
    (*obj).u.user.put = put;
    (*obj).u.user.delete = del;
    (*obj).u.user.finalize = finalize;

    js_pushobject(J, obj);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newuserdata(J: *mut js_State, tag: *const c_char, data: *mut c_void, finalize: js_Finalize) {
    js_newuserdatax(J, tag, data, None, None, None, finalize);
}

/* Non-trivial operations on values */
#[no_mangle]
pub unsafe extern "C-unwind" fn js_instanceof(J: *mut js_State) -> c_int {
    let mut V: *mut js_Object;
    let O: *mut js_Object;

    if js_iscallable(J, -1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("instanceof: invalid operand"));
    }

    if js_isobject(J, -2) == 0 {
        return 0;
    }

    js_getproperty(J, -1, cstr!("prototype"));
    if js_isobject(J, -1) == 0 {
        crate::jserror::js_typeerror(J, cstr!("instanceof: 'prototype' property is not an object"));
    }
    O = js_toobject(J, -1);
    js_pop(J, 1);

    V = js_toobject(J, -2);
    while !V.is_null() {
        V = (*V).prototype;
        if O == V {
            return 1;
        }
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_concat(J: *mut js_State) {
    js_toprimitive(J, -2, JS_HNONE);
    js_toprimitive(J, -1, JS_HNONE);

    if js_isstring(J, -2) != 0 || js_isstring(J, -1) != 0 {
        let sa = js_tostring(J, -2);
        let sb = js_tostring(J, -1);
        let mut sab: *mut c_char = std::ptr::null_mut();
        let sab_ptr = std::ptr::addr_of_mut!(sab);
        let caught = protect(J, || {
            *sab_ptr = js_malloc(J, (strlen(sa) + strlen(sb) + 1) as c_int) as *mut c_char;
            strcpy(*sab_ptr, sa);
            strcat(*sab_ptr, sb);
            js_pop(J, 2);
            js_pushstring(J, *sab_ptr);
        });
        if caught {
            js_free(J, sab as *mut c_void);
            js_throw(J);
        }
        js_endtry(J);
        js_free(J, sab as *mut c_void);
    } else {
        let x = js_tonumber(J, -2);
        let y = js_tonumber(J, -1);
        js_pop(J, 2);
        js_pushnumber(J, x + y);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_compare(J: *mut js_State, okay: *mut c_int) -> c_int {
    js_toprimitive(J, -2, JS_HNUMBER);
    js_toprimitive(J, -1, JS_HNUMBER);

    *okay = 1;
    if js_isstring(J, -2) != 0 && js_isstring(J, -1) != 0 {
        strcmp(js_tostring(J, -2), js_tostring(J, -1))
    } else {
        let x = js_tonumber(J, -2);
        let y = js_tonumber(J, -1);
        if x.is_nan() || y.is_nan() {
            *okay = 0;
        }
        if x < y {
            -1
        } else if x > y {
            1
        } else {
            0
        }
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_equal(J: *mut js_State) -> c_int {
    let x = js_tovalue(J, -2);
    let y = js_tovalue(J, -1);

    loop {
        if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
            return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
        }
        if (*x).type_() == (*y).type_() {
            if (*x).type_() == JS_TUNDEFINED {
                return 1;
            }
            if (*x).type_() == JS_TNULL {
                return 1;
            }
            if (*x).type_() == JS_TNUMBER {
                return ((*x).u.number == (*y).u.number) as c_int;
            }
            if (*x).type_() == JS_TBOOLEAN {
                return ((*x).u.boolean == (*y).u.boolean) as c_int;
            }
            if (*x).type_() == JS_TOBJECT {
                return ((*x).u.object == (*y).u.object) as c_int;
            }
            return 0;
        }

        if (*x).type_() == JS_TNULL && (*y).type_() == JS_TUNDEFINED {
            return 1;
        }
        if (*x).type_() == JS_TUNDEFINED && (*y).type_() == JS_TNULL {
            return 1;
        }

        if (*x).type_() == JS_TNUMBER && JSV_ISSTRING(y) {
            return ((*x).u.number == jsV_tonumber(J, y)) as c_int;
        }
        if JSV_ISSTRING(x) && (*y).type_() == JS_TNUMBER {
            return (jsV_tonumber(J, x) == (*y).u.number) as c_int;
        }

        if (*x).type_() == JS_TBOOLEAN {
            (*x).set_type(JS_TNUMBER);
            (*x).u.number = if (*x).u.boolean != 0 { 1.0 } else { 0.0 };
            continue;
        }
        if (*y).type_() == JS_TBOOLEAN {
            (*y).set_type(JS_TNUMBER);
            (*y).u.number = if (*y).u.boolean != 0 { 1.0 } else { 0.0 };
            continue;
        }
        if (JSV_ISSTRING(x) || (*x).type_() == JS_TNUMBER) && (*y).type_() == JS_TOBJECT {
            jsV_toprimitive(J, y, JS_HNONE);
            continue;
        }
        if (*x).type_() == JS_TOBJECT && (JSV_ISSTRING(y) || (*y).type_() == JS_TNUMBER) {
            jsV_toprimitive(J, x, JS_HNONE);
            continue;
        }

        return 0;
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_strictequal(J: *mut js_State) -> c_int {
    let x = js_tovalue(J, -2);
    let y = js_tovalue(J, -1);

    if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
        return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
    }

    if (*x).type_() != (*y).type_() {
        return 0;
    }
    if (*x).type_() == JS_TUNDEFINED {
        return 1;
    }
    if (*x).type_() == JS_TNULL {
        return 1;
    }
    if (*x).type_() == JS_TNUMBER {
        return ((*x).u.number == (*y).u.number) as c_int;
    }
    if (*x).type_() == JS_TBOOLEAN {
        return ((*x).u.boolean == (*y).u.boolean) as c_int;
    }
    if (*x).type_() == JS_TOBJECT {
        return ((*x).u.object == (*y).u.object) as c_int;
    }
    0
}
