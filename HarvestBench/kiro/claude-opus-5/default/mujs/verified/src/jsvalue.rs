//! Translation of src/jsvalue.c
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;

use crate::jsdtoa::{js_fmtexp, js_grisu2, js_strtod};
use crate::jsproperty::jsV_newobject;
use crate::jslex::{jsY_isnewline, jsY_iswhite};
use crate::jsrun::{
    jsV_newmemstring, js_call, js_copy, js_defproperty, js_free, js_getproperty, js_iscallable,
    js_isobject, js_isprimitive, js_isstring, js_malloc, js_pop, js_pushnumber, js_pushobject,
    js_pushstring, js_rot2, js_strdup, js_throw, js_toboolean, js_tonumber, js_toobject,
    js_toprimitive, js_tostring, js_tovalue,
};
use crate::jsstring::js_utflen;

/* #define JSV_ISSTRING(v) (v->t.type==JS_TSHRSTR || v->t.type==JS_TMEMSTR || v->t.type==JS_TLITSTR) */
#[inline(always)]
unsafe fn JSV_ISSTRING(v: *mut js_Value) -> bool {
    unsafe {
        (*v).ty() == JS_TSHRSTR || (*v).ty() == JS_TMEMSTR || (*v).ty() == JS_TLITSTR
    }
}

/* #define JSV_TOSTRING(v) (v->t.type==JS_TSHRSTR ? v->u.shrstr : v->t.type==JS_TLITSTR ? v->u.litstr : v->t.type==JS_TMEMSTR ? v->u.memstr->p : "") */
#[inline(always)]
unsafe fn JSV_TOSTRING(v: *mut js_Value) -> *const c_char {
    unsafe {
        if (*v).ty() == JS_TSHRSTR {
            (&raw const (*v).shrstr) as *const c_char
        } else if (*v).ty() == JS_TLITSTR {
            (*v).litstr
        } else if (*v).ty() == JS_TMEMSTR {
            (&raw mut (*(*v).memstr).p) as *const c_char
        } else {
            c"".as_ptr()
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_strtol(s: *const c_char, p: *mut *mut c_char, base: c_int) -> f64 { unsafe {
    /* ascii -> digit value. max base is 36. */
    static table: [c_uchar; 256] = [
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
    let mut s = s;
    let mut x: f64;
    let mut c: c_uchar;
    if base == 10 {
        x = 0.0;
        c = *s as c_uchar;
        s = s.add(1);
        /* (0 <= c - '0') && (c - '0' < 10) with unsigned char c: equivalently
         * the wrapping u8 subtraction is < 10 exactly when c is in '0'..'9'. */
        while (c.wrapping_sub(b'0')) < 10 {
            x = x * 10.0 + (c.wrapping_sub(b'0')) as f64;
            c = *s as c_uchar;
            s = s.add(1);
        }
    } else {
        x = 0.0;
        c = *s as c_uchar;
        s = s.add(1);
        while (table[c as usize] as c_int) < base {
            x = x * base as f64 + table[c as usize] as f64;
            c = *s as c_uchar;
            s = s.add(1);
        }
    }
    if !p.is_null() {
        *p = (s as *mut c_char).offset(-1);
    }
    x
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertointeger(n: f64) -> c_int { unsafe {
    if n == 0.0 { return 0; }
    if isnan(n) { return 0; }
    let n = if n < 0.0 { -floor(-n) } else { floor(n) };
    if n < INT_MIN as f64 { return INT_MIN; }
    if n > INT_MAX as f64 { return INT_MAX; }
    n as c_int
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertoint32(n: f64) -> c_int { unsafe {
    let two32: f64 = 4294967296.0;
    let two31: f64 = 2147483648.0;

    if !isfinite(n) || n == 0.0 {
        return 0;
    }

    let mut n = fmod(n, two32);
    n = if n >= 0.0 { floor(n) } else { ceil(n) + two32 };
    if n >= two31 {
        d2i(n - two32)
    } else {
        d2i(n)
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertouint32(n: f64) -> c_uint { unsafe {
    jsV_numbertoint32(n) as c_uint
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertoint16(n: f64) -> c_short { unsafe {
    jsV_numbertoint32(n) as c_short
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertouint16(n: f64) -> c_ushort { unsafe {
    jsV_numbertoint32(n) as c_ushort
}}

/* obj.toString() */
unsafe fn jsV_toString(J: *mut js_State, obj: *mut js_Object) -> c_int { unsafe {
    js_pushobject(J, obj);
    js_getproperty(J, -1, c"toString".as_ptr());
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
}}

/* obj.valueOf() */
unsafe fn jsV_valueOf(J: *mut js_State, obj: *mut js_Object) -> c_int { unsafe {
    js_pushobject(J, obj);
    js_getproperty(J, -1, c"valueOf".as_ptr());
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
}}

/* ToPrimitive() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_toprimitive(J: *mut js_State, v: *mut js_Value, preferred: c_int) { unsafe {
    let obj: *mut js_Object;

    if (*v).ty() != JS_TOBJECT {
        return;
    }

    obj = (*v).object;

    let preferred = if preferred == JS_HNONE {
        if (*obj).ty == JS_CDATE { JS_HSTRING } else { JS_HNUMBER }
    } else {
        preferred
    };

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
        js_typeerror!(J, c"cannot convert object to primitive".as_ptr());
    }

    (*v).set_ty(JS_TLITSTR);
    (*v).litstr = c"[object]".as_ptr();
}}

/* ToBoolean() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_toboolean(J: *mut js_State, v: *mut js_Value) -> c_int { unsafe {
    match (*v).ty() {
        JS_TUNDEFINED => 0,
        JS_TNULL => 0,
        JS_TBOOLEAN => (*v).boolean,
        JS_TNUMBER => ((*v).number != 0.0 && !isnan((*v).number)) as c_int,
        JS_TLITSTR => (*(*v).litstr != 0) as c_int,
        JS_TMEMSTR => (*(&raw mut (*(*v).memstr).p as *mut c_char) != 0) as c_int,
        JS_TOBJECT => 1,
        /* default and JS_TSHRSTR */
        _ => (*((&raw const (*v).shrstr) as *const c_char) != 0) as c_int,
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_itoa(out: *mut c_char, v: c_int) -> *const c_char { unsafe {
    let mut buf: [c_char; 32] = [0; 32];
    let mut s = out;
    let mut a: c_uint;
    let mut i: c_int = 0;
    if v < 0 {
        a = (v as c_uint).wrapping_neg(); /* cast to avoid -INT_MIN signed overflow UB */
        *s = b'-' as c_char;
        s = s.add(1);
    } else {
        a = v as c_uint;
    }
    while a != 0 {
        buf[i as usize] = ((a % 10) as c_char) + b'0' as c_char;
        i += 1;
        a /= 10;
    }
    if i == 0 {
        buf[i as usize] = b'0' as c_char;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        *s = buf[i as usize];
        s = s.add(1);
    }
    *s = 0;
    out
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_stringtofloat(s: *const c_char, ep: *mut *mut c_char) -> f64 { unsafe {
    let mut end: *mut c_char = core::ptr::null_mut();
    let n: f64;
    let mut e: *const c_char = s;
    let mut isflt: c_int = 0;
    if *e as c_int == '+' as c_int || *e as c_int == '-' as c_int { e = e.add(1); }
    while *e as c_int >= '0' as c_int && *e as c_int <= '9' as c_int { e = e.add(1); }
    if *e as c_int == '.' as c_int { e = e.add(1); isflt = 1; }
    while *e as c_int >= '0' as c_int && *e as c_int <= '9' as c_int { e = e.add(1); }
    if *e as c_int == 'e' as c_int || *e as c_int == 'E' as c_int {
        e = e.add(1);
        if *e as c_int == '+' as c_int || *e as c_int == '-' as c_int { e = e.add(1); }
        while *e as c_int >= '0' as c_int && *e as c_int <= '9' as c_int { e = e.add(1); }
        isflt = 1;
    }
    if isflt != 0 {
        n = js_strtod(s, &mut end);
    } else {
        /* js_strtol doesn't parse the sign */
        if *s as c_int == '-' as c_int {
            n = -js_strtol(s.add(1), &mut end, 10);
        } else if *s as c_int == '+' as c_int {
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
}}

/* ToNumber() on a string */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_stringtonumber(J: *mut js_State, s: *const c_char) -> f64 { unsafe {
    let mut e: *mut c_char = core::ptr::null_mut();
    let n: f64;
    let mut s = s;
    while jsY_iswhite(*s as c_int) != 0 || jsY_isnewline(*s as c_int) != 0 { s = s.add(1); }
    if *s.add(0) as c_int == '0' as c_int
        && (*s.add(1) as c_int == 'x' as c_int || *s.add(1) as c_int == 'X' as c_int)
        && *s.add(2) as c_int != 0
    {
        n = js_strtol(s.add(2), &mut e, 16);
    } else if strncmp(s, c"Infinity".as_ptr(), 8) == 0 {
        n = INFINITY;
        e = (s as *mut c_char).add(8);
    } else if strncmp(s, c"+Infinity".as_ptr(), 9) == 0 {
        n = INFINITY;
        e = (s as *mut c_char).add(9);
    } else if strncmp(s, c"-Infinity".as_ptr(), 9) == 0 {
        n = -INFINITY;
        e = (s as *mut c_char).add(9);
    } else {
        n = js_stringtofloat(s, &mut e);
    }
    while jsY_iswhite(*e as c_int) != 0 || jsY_isnewline(*e as c_int) != 0 { e = e.add(1); }
    if *e != 0 { return NAN; }
    n
}}

/* ToNumber() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_tonumber(J: *mut js_State, v: *mut js_Value) -> f64 { unsafe {
    match (*v).ty() {
        JS_TUNDEFINED => NAN,
        JS_TNULL => 0.0,
        JS_TBOOLEAN => (*v).boolean as f64,
        JS_TNUMBER => (*v).number,
        JS_TLITSTR => jsV_stringtonumber(J, (*v).litstr),
        JS_TMEMSTR => jsV_stringtonumber(J, (&raw mut (*(*v).memstr).p) as *const c_char),
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HNUMBER);
            jsV_tonumber(J, v)
        }
        /* default and JS_TSHRSTR */
        _ => jsV_stringtonumber(J, (&raw const (*v).shrstr) as *const c_char),
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_tointeger(J: *mut js_State, v: *mut js_Value) -> f64 { unsafe {
    jsV_numbertointeger(jsV_tonumber(J, v)) as f64
}}

/* ToString() on a number */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_numbertostring(J: *mut js_State, buf: *mut c_char, f: f64) -> *const c_char { unsafe {
    let mut digits: [c_char; 32] = [0; 32];
    let mut p: *mut c_char = buf;
    let mut s: *mut c_char = (&raw mut digits) as *mut c_char;
    let mut exp: c_int = 0;
    let mut ndigits: c_int;
    let mut point: c_int;

    if f == 0.0 { return c"0".as_ptr(); }
    if isnan(f) { return c"NaN".as_ptr(); }
    if isinf(f) { return if f < 0.0 { c"-Infinity".as_ptr() } else { c"Infinity".as_ptr() }; }

    /* Fast case for integers. This only works assuming all integers can be
     * exactly represented by a float. This is true for 32-bit integers and
     * 64-bit floats. */
    if f >= INT_MIN as f64 && f <= INT_MAX as f64 {
        let i = f as c_int;
        if i as f64 == f {
            return js_itoa(buf, i);
        }
    }

    ndigits = js_grisu2(f, (&raw mut digits) as *mut c_char, &mut exp);
    point = ndigits + exp;

    if signbit(f) {
        *p = b'-' as c_char;
        p = p.add(1);
    }

    if point < -5 || point > 21 {
        *p = *s; p = p.add(1); s = s.add(1);
        if ndigits > 1 {
            let mut n = ndigits - 1;
            *p = b'.' as c_char; p = p.add(1);
            while n != 0 {
                n -= 1;
                *p = *s; p = p.add(1); s = s.add(1);
            }
        }
        js_fmtexp(p, point - 1);
    }

    else if point <= 0 {
        *p = b'0' as c_char; p = p.add(1);
        *p = b'.' as c_char; p = p.add(1);
        while point < 0 {
            point += 1;
            *p = b'0' as c_char; p = p.add(1);
        }
        while ndigits > 0 {
            ndigits -= 1;
            *p = *s; p = p.add(1); s = s.add(1);
        }
        *p = 0;
    }

    else {
        while ndigits > 0 {
            ndigits -= 1;
            *p = *s; p = p.add(1); s = s.add(1);
            point -= 1;
            if point == 0 && ndigits > 0 {
                *p = b'.' as c_char; p = p.add(1);
            }
        }
        while point > 0 {
            point -= 1;
            *p = b'0' as c_char; p = p.add(1);
        }
        *p = 0;
    }

    buf
}}

/* ToString() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_tostring(J: *mut js_State, v: *mut js_Value) -> *const c_char { unsafe {
    let mut buf: [c_char; 32] = [0; 32];
    let p: *const c_char;
    match (*v).ty() {
        JS_TUNDEFINED => c"undefined".as_ptr(),
        JS_TNULL => c"null".as_ptr(),
        JS_TBOOLEAN => if (*v).boolean != 0 { c"true".as_ptr() } else { c"false".as_ptr() },
        JS_TLITSTR => (*v).litstr,
        JS_TMEMSTR => (&raw mut (*(*v).memstr).p) as *const c_char,
        JS_TNUMBER => {
            p = jsV_numbertostring(J, (&raw mut buf) as *mut c_char, (*v).number);
            if p == (&raw const buf) as *const c_char {
                let mut n = strlen(p) as c_int;
                if n <= JS_VALUE_TYPEOFF {
                    let mut sd = (&raw mut (*v).shrstr) as *mut c_char;
                    let mut ps = p;
                    while n != 0 {
                        n -= 1;
                        *sd = *ps;
                        sd = sd.add(1);
                        ps = ps.add(1);
                    }
                    *sd = 0;
                    (*v).set_ty(JS_TSHRSTR);
                    (&raw const (*v).shrstr) as *const c_char
                } else {
                    (*v).memstr = jsV_newmemstring(J, p, n);
                    (*v).set_ty(JS_TMEMSTR);
                    (&raw mut (*(*v).memstr).p) as *const c_char
                }
            } else {
                p
            }
        }
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HSTRING);
            jsV_tostring(J, v)
        }
        /* default and JS_TSHRSTR */
        _ => (&raw const (*v).shrstr) as *const c_char,
    }
}}

/* Objects */

unsafe fn jsV_newboolean(J: *mut js_State, v: c_int) -> *mut js_Object { unsafe {
    let obj = jsV_newobject(J, JS_CBOOLEAN, (*J).Boolean_prototype);
    (*obj).u.boolean = v;
    obj
}}

unsafe fn jsV_newnumber(J: *mut js_State, v: f64) -> *mut js_Object { unsafe {
    let obj = jsV_newobject(J, JS_CNUMBER, (*J).Number_prototype);
    (*obj).u.number = v;
    obj
}}

unsafe fn jsV_newstring(J: *mut js_State, v: *const c_char) -> *mut js_Object { unsafe {
    let obj = jsV_newobject(J, JS_CSTRING, (*J).String_prototype);
    let n = strlen(v);
    if n < core::mem::size_of_val(&(*obj).u.s.shrstr) {
        (*obj).u.s.string = (&raw mut (*obj).u.s.shrstr) as *mut c_char;
        memcpy(
            (&raw mut (*obj).u.s.shrstr) as *mut c_void,
            v as *const c_void,
            n + 1,
        );
    } else {
        (*obj).u.s.string = js_strdup(J, v);
    }
    (*obj).u.s.length = js_utflen(v);
    obj
}}

/* ToObject() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsV_toobject(J: *mut js_State, v: *mut js_Value) -> *mut js_Object { unsafe {
    let o: *mut js_Object;
    match (*v).ty() {
        JS_TUNDEFINED => js_typeerror!(J, c"cannot convert undefined to object".as_ptr()),
        JS_TNULL => js_typeerror!(J, c"cannot convert null to object".as_ptr()),
        JS_TOBJECT => return (*v).object,
        JS_TSHRSTR => o = jsV_newstring(J, (&raw const (*v).shrstr) as *const c_char),
        JS_TLITSTR => o = jsV_newstring(J, (*v).litstr),
        JS_TMEMSTR => o = jsV_newstring(J, (&raw mut (*(*v).memstr).p) as *const c_char),
        JS_TBOOLEAN => o = jsV_newboolean(J, (*v).boolean),
        JS_TNUMBER => o = jsV_newnumber(J, (*v).number),
        /* default */
        _ => js_typeerror!(J, c"cannot convert undefined to object".as_ptr()),
    }
    (*v).set_ty(JS_TOBJECT);
    (*v).object = o;
    o
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newobjectx(J: *mut js_State) { unsafe {
    let mut prototype: *mut js_Object = core::ptr::null_mut();
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, prototype));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newobject(J: *mut js_State) { unsafe {
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, (*J).Object_prototype));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newarguments(J: *mut js_State) { unsafe {
    js_pushobject(J, jsV_newobject(J, JS_CARGUMENTS, (*J).Object_prototype));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newarray(J: *mut js_State) { unsafe {
    let obj = jsV_newobject(J, JS_CARRAY, (*J).Array_prototype);
    (*obj).u.a.simple = 1;
    js_pushobject(J, obj);
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newboolean(J: *mut js_State, v: c_int) { unsafe {
    js_pushobject(J, jsV_newboolean(J, v));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newnumber(J: *mut js_State, v: f64) { unsafe {
    js_pushobject(J, jsV_newnumber(J, v));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newstring(J: *mut js_State, v: *const c_char) { unsafe {
    js_pushobject(J, jsV_newstring(J, v));
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newfunction(J: *mut js_State, fun: *mut js_Function, scope: *mut js_Environment) { unsafe {
    let obj = jsV_newobject(J, JS_CFUNCTION, (*J).Function_prototype);
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
    {
        js_pushnumber(J, (*fun).numparams as f64);
        js_defproperty(J, -2, c"length".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        }
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newscript(J: *mut js_State, fun: *mut js_Function, scope: *mut js_Environment) { unsafe {
    let obj = jsV_newobject(J, JS_CSCRIPT, core::ptr::null_mut());
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newcfunctionx(
    J: *mut js_State,
    cfun: js_CFunction,
    name: *const c_char,
    length: c_int,
    data: *mut c_void,
    finalize: js_Finalize,
) { unsafe {
    let mut obj: *mut js_Object = core::ptr::null_mut();

    if crate::except::js_try_run(J, || {
        obj = jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
        (*obj).u.c.name = name;
        (*obj).u.c.function = cfun;
        (*obj).u.c.constructor = None;
        (*obj).u.c.length = length;
        (*obj).u.c.data = data;
        (*obj).u.c.finalize = finalize;
        crate::jsrun::js_endtry(J);
    }) {
        if finalize.is_some() {
            (finalize.unwrap())(J, data);
        }
        js_throw(J);
    }

    js_pushobject(J, obj);
    {
        js_pushnumber(J, length as f64);
        js_defproperty(J, -2, c"length".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        }
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newcfunction(J: *mut js_State, cfun: js_CFunction, name: *const c_char, length: c_int) { unsafe {
    js_newcfunctionx(J, cfun, name, length, core::ptr::null_mut(), None);
}}

/* prototype -- constructor */
#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newcconstructor(
    J: *mut js_State,
    cfun: js_CFunction,
    ccon: js_CFunction,
    name: *const c_char,
    length: c_int,
) { unsafe {
    let obj = jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = ccon;
    (*obj).u.c.length = length;
    js_pushobject(J, obj); /* proto obj */
    {
        js_pushnumber(J, length as f64);
        js_defproperty(J, -2, c"length".as_ptr(), JS_READONLY | JS_DONTENUM | JS_DONTCONF);
        js_rot2(J); /* obj proto */
        js_copy(J, -2); /* obj proto obj */
        js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newuserdatax(
    J: *mut js_State,
    tag: *const c_char,
    data: *mut c_void,
    has: js_HasProperty,
    put: js_Put,
    delete: js_Delete,
    finalize: js_Finalize,
) { unsafe {
    let mut prototype: *mut js_Object = core::ptr::null_mut();
    let mut obj: *mut js_Object = core::ptr::null_mut();

    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);

    if crate::except::js_try_run(J, || {
        obj = jsV_newobject(J, JS_CUSERDATA, prototype);
        (*obj).u.user.tag = tag;
        (*obj).u.user.data = data;
        (*obj).u.user.has = has;
        (*obj).u.user.put = put;
        (*obj).u.user.delete = delete;
        (*obj).u.user.finalize = finalize;
        crate::jsrun::js_endtry(J);
    }) {
        if finalize.is_some() {
            (finalize.unwrap())(J, data);
        }
        js_throw(J);
    }

    js_pushobject(J, obj);
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newuserdata(J: *mut js_State, tag: *const c_char, data: *mut c_void, finalize: js_Finalize) { unsafe {
    js_newuserdatax(J, tag, data, None, None, None, finalize);
}}

/* Non-trivial operations on values. These are implemented using the stack. */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_instanceof(J: *mut js_State) -> c_int { unsafe {
    let O: *mut js_Object;
    let mut V: *mut js_Object;

    if js_iscallable(J, -1) == 0 {
        js_typeerror!(J, c"instanceof: invalid operand".as_ptr());
    }

    if js_isobject(J, -2) == 0 {
        return 0;
    }

    js_getproperty(J, -1, c"prototype".as_ptr());
    if js_isobject(J, -1) == 0 {
        js_typeerror!(J, c"instanceof: 'prototype' property is not an object".as_ptr());
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
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_concat(J: *mut js_State) { unsafe {
    js_toprimitive(J, -2, JS_HNONE);
    js_toprimitive(J, -1, JS_HNONE);

    if js_isstring(J, -2) != 0 || js_isstring(J, -1) != 0 {
        let sa = js_tostring(J, -2);
        let sb = js_tostring(J, -1);
        let mut sab: *mut c_char = core::ptr::null_mut();
        /* TODO: create js_String directly */
        if crate::except::js_try_run(J, || {
            sab = js_malloc(J, (strlen(sa) + strlen(sb) + 1) as c_int) as *mut c_char;
            strcpy(sab, sa);
            strcat(sab, sb);
            js_pop(J, 2);
            js_pushstring(J, sab);
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, sab as *mut c_void);
            js_throw(J);
        }
        js_free(J, sab as *mut c_void);
    } else {
        let x = js_tonumber(J, -2);
        let y = js_tonumber(J, -1);
        js_pop(J, 2);
        js_pushnumber(J, x + y);
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_compare(J: *mut js_State, okay: *mut c_int) -> c_int { unsafe {
    js_toprimitive(J, -2, JS_HNUMBER);
    js_toprimitive(J, -1, JS_HNUMBER);

    *okay = 1;
    if js_isstring(J, -2) != 0 && js_isstring(J, -1) != 0 {
        strcmp(js_tostring(J, -2), js_tostring(J, -1))
    } else {
        let x = js_tonumber(J, -2);
        let y = js_tonumber(J, -1);
        if isnan(x) || isnan(y) {
            *okay = 0;
        }
        if x < y { -1 } else if x > y { 1 } else { 0 }
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_equal(J: *mut js_State) -> c_int { unsafe {
    let x = js_tovalue(J, -2);
    let y = js_tovalue(J, -1);

    'retry: loop {
        if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
            return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
        }
        if (*x).ty() == (*y).ty() {
            if (*x).ty() == JS_TUNDEFINED { return 1; }
            if (*x).ty() == JS_TNULL { return 1; }
            if (*x).ty() == JS_TNUMBER { return ((*x).number == (*y).number) as c_int; }
            if (*x).ty() == JS_TBOOLEAN { return ((*x).boolean == (*y).boolean) as c_int; }
            if (*x).ty() == JS_TOBJECT { return ((*x).object == (*y).object) as c_int; }
            return 0;
        }

        if (*x).ty() == JS_TNULL && (*y).ty() == JS_TUNDEFINED { return 1; }
        if (*x).ty() == JS_TUNDEFINED && (*y).ty() == JS_TNULL { return 1; }

        if (*x).ty() == JS_TNUMBER && JSV_ISSTRING(y) {
            return ((*x).number == jsV_tonumber(J, y)) as c_int;
        }
        if JSV_ISSTRING(x) && (*y).ty() == JS_TNUMBER {
            return (jsV_tonumber(J, x) == (*y).number) as c_int;
        }

        if (*x).ty() == JS_TBOOLEAN {
            (*x).set_ty(JS_TNUMBER);
            (*x).number = if (*x).boolean != 0 { 1.0 } else { 0.0 };
            continue 'retry;
        }
        if (*y).ty() == JS_TBOOLEAN {
            (*y).set_ty(JS_TNUMBER);
            (*y).number = if (*y).boolean != 0 { 1.0 } else { 0.0 };
            continue 'retry;
        }
        if (JSV_ISSTRING(x) || (*x).ty() == JS_TNUMBER) && (*y).ty() == JS_TOBJECT {
            jsV_toprimitive(J, y, JS_HNONE);
            continue 'retry;
        }
        if (*x).ty() == JS_TOBJECT && (JSV_ISSTRING(y) || (*y).ty() == JS_TNUMBER) {
            jsV_toprimitive(J, x, JS_HNONE);
            continue 'retry;
        }

        return 0;
    }
}}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_strictequal(J: *mut js_State) -> c_int { unsafe {
    let x = js_tovalue(J, -2);
    let y = js_tovalue(J, -1);

    if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
        return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
    }

    if (*x).ty() != (*y).ty() { return 0; }
    if (*x).ty() == JS_TUNDEFINED { return 1; }
    if (*x).ty() == JS_TNULL { return 1; }
    if (*x).ty() == JS_TNUMBER { return ((*x).number == (*y).number) as c_int; }
    if (*x).ty() == JS_TBOOLEAN { return ((*x).boolean == (*y).boolean) as c_int; }
    if (*x).ty() == JS_TOBJECT { return ((*x).object == (*y).object) as c_int; }
    0
}}
