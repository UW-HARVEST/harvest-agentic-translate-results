//! Translated from c_src/src/jsvalue.c
use crate::jsi::*;
use crate::prelude::*;

/* ascii -> digit value. max base is 36. */
static js_strtol_table: [c_uchar; 256] = [
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_strtol(
    s: *const c_char,
    p: *mut *mut c_char,
    base: c_int,
) -> f64 {
    let mut s = s;
    let mut x: f64;
    let mut c: c_uchar;
    if base == 10 {
        x = 0.0;
        c = *s as c_uchar;
        s = s.add(1);
        while (0 <= (c as c_int - '0' as c_int)) && ((c as c_int - '0' as c_int) < 10) {
            x = x * 10.0 + (c as c_int - '0' as c_int) as f64;
            c = *s as c_uchar;
            s = s.add(1);
        }
    } else {
        x = 0.0;
        c = *s as c_uchar;
        s = s.add(1);
        while (js_strtol_table[c as usize] as c_int) < base {
            x = x * base as f64 + js_strtol_table[c as usize] as f64;
            c = *s as c_uchar;
            s = s.add(1);
        }
    }
    if !p.is_null() {
        *p = s.sub(1) as *mut c_char;
    }
    x
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertointeger(n: f64) -> c_int {
    let mut n = n;
    if n == 0.0 {
        return 0;
    }
    if isnan(n) {
        return 0;
    }
    n = if n < 0.0 { -floor(-n) } else { floor(n) };
    if n < INT_MIN as f64 {
        return INT_MIN;
    }
    if n > INT_MAX as f64 {
        return INT_MAX;
    }
    n as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertoint32(n: f64) -> c_int {
    let two32: f64 = 4294967296.0;
    let two31: f64 = 2147483648.0;
    let mut n = n;

    if !isfinite(n) || n == 0.0 {
        return 0;
    }

    n = fmod(n, two32);
    n = if n >= 0.0 { floor(n) } else { ceil(n) + two32 };
    if n >= two31 {
        (n - two32) as c_int
    } else {
        n as c_int
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertouint32(n: f64) -> c_uint {
    jsV_numbertoint32(n) as c_uint
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertoint16(n: f64) -> c_short {
    jsV_numbertoint32(n) as c_short
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertouint16(n: f64) -> c_ushort {
    jsV_numbertoint32(n) as c_ushort
}

/* obj.toString() */
unsafe fn jsV_toString(J: *mut js_State, obj: *mut js_Object) -> c_int {
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
}

/* obj.valueOf() */
unsafe fn jsV_valueOf(J: *mut js_State, obj: *mut js_Object) -> c_int {
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
}

/* ToPrimitive() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toprimitive(J: *mut js_State, v: *mut js_Value, preferred: c_int) {
    let obj: *mut js_Object;
    let mut preferred = preferred;

    if (*v).t.r#type != JS_TOBJECT {
        return;
    }

    obj = (*v).u.object;

    if preferred == JS_HNONE {
        preferred = if (*obj).r#type == JS_CDATE {
            JS_HSTRING
        } else {
            JS_HNUMBER
        };
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
        js_typeerror!(J, c"cannot convert object to primitive".as_ptr());
    }

    (*v).t.r#type = JS_TLITSTR;
    (*v).u.litstr = c"[object]".as_ptr();
    return;
}

/* ToBoolean() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toboolean(J: *mut js_State, v: *mut js_Value) -> c_int {
    match (*v).t.r#type {
        JS_TUNDEFINED => 0,
        JS_TNULL => 0,
        JS_TBOOLEAN => (*v).u.boolean,
        JS_TNUMBER => ((*v).u.number != 0.0 && !isnan((*v).u.number)) as c_int,
        JS_TLITSTR => (*(*v).u.litstr != 0) as c_int,
        JS_TMEMSTR => (*js_String_p((*v).u.memstr) != 0) as c_int,
        JS_TOBJECT => 1,
        /* JS_TSHRSTR and default */
        _ => (*js_Value_shrstr(v) != 0) as c_int,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_itoa(out: *mut c_char, v: c_int) -> *const c_char {
    let mut buf: [c_char; 32] = [0; 32];
    let mut s: *mut c_char = out;
    let mut a: c_uint;
    let mut i: c_int = 0;
    if v < 0 {
        a = (v as c_uint).wrapping_neg(); /* cast to avoid -INT_MIN signed overflow UB */
        *s = '-' as c_char;
        s = s.add(1);
    } else {
        a = v as c_uint;
    }
    while a != 0 {
        buf[i as usize] = ((a % 10) as c_char) + '0' as c_char;
        i += 1;
        a /= 10;
    }
    if i == 0 {
        buf[i as usize] = '0' as c_char;
        i += 1;
    }
    while i > 0 {
        i -= 1;
        *s = buf[i as usize];
        s = s.add(1);
    }
    *s = 0;
    out
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_stringtofloat(s: *const c_char, ep: *mut *mut c_char) -> f64 {
    let mut end: *mut c_char = null_mut();
    let n: f64;
    let mut e: *const c_char = s;
    let mut isflt: c_int = 0;
    if *e == '+' as c_char || *e == '-' as c_char {
        e = e.add(1);
    }
    while *e >= '0' as c_char && *e <= '9' as c_char {
        e = e.add(1);
    }
    if *e == '.' as c_char {
        e = e.add(1);
        isflt = 1;
    }
    while *e >= '0' as c_char && *e <= '9' as c_char {
        e = e.add(1);
    }
    if *e == 'e' as c_char || *e == 'E' as c_char {
        e = e.add(1);
        if *e == '+' as c_char || *e == '-' as c_char {
            e = e.add(1);
        }
        while *e >= '0' as c_char && *e <= '9' as c_char {
            e = e.add(1);
        }
        isflt = 1;
    }
    if isflt != 0 {
        n = js_strtod(s, &mut end);
    } else {
        /* js_strtol doesn't parse the sign */
        if *s == '-' as c_char {
            n = -js_strtol(s.add(1), &mut end, 10);
        } else if *s == '+' as c_char {
            n = js_strtol(s.add(1), &mut end, 10);
        } else {
            n = js_strtol(s, &mut end, 10);
        }
    }
    if end as *const c_char == e {
        *ep = e as *mut c_char;
        return n;
    }
    *ep = s as *mut c_char;
    0.0
}

/* ToNumber() on a string */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_stringtonumber(J: *mut js_State, s: *const c_char) -> f64 {
    let mut e: *mut c_char = null_mut();
    let n: f64;
    let mut s = s;
    while jsY_iswhite(*s as c_int) != 0 || jsY_isnewline(*s as c_int) != 0 {
        s = s.add(1);
    }
    if *s.add(0) == '0' as c_char
        && (*s.add(1) == 'x' as c_char || *s.add(1) == 'X' as c_char)
        && *s.add(2) != 0
    {
        n = js_strtol(s.add(2), &mut e, 16);
    } else if strncmp(s, c"Infinity".as_ptr(), 8) == 0 {
        n = INFINITY;
        e = s.add(8) as *mut c_char;
    } else if strncmp(s, c"+Infinity".as_ptr(), 9) == 0 {
        n = INFINITY;
        e = s.add(9) as *mut c_char;
    } else if strncmp(s, c"-Infinity".as_ptr(), 9) == 0 {
        n = -INFINITY;
        e = s.add(9) as *mut c_char;
    } else {
        n = js_stringtofloat(s, &mut e);
    }
    while jsY_iswhite(*e as c_int) != 0 || jsY_isnewline(*e as c_int) != 0 {
        e = e.add(1);
    }
    if *e != 0 {
        return NAN;
    }
    n
}

/* ToNumber() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tonumber(J: *mut js_State, v: *mut js_Value) -> f64 {
    match (*v).t.r#type {
        JS_TUNDEFINED => NAN,
        JS_TNULL => 0.0,
        JS_TBOOLEAN => (*v).u.boolean as f64,
        JS_TNUMBER => (*v).u.number,
        JS_TLITSTR => jsV_stringtonumber(J, (*v).u.litstr),
        JS_TMEMSTR => jsV_stringtonumber(J, js_String_p((*v).u.memstr)),
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HNUMBER);
            jsV_tonumber(J, v)
        }
        /* JS_TSHRSTR and default */
        _ => jsV_stringtonumber(J, js_Value_shrstr(v)),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tointeger(J: *mut js_State, v: *mut js_Value) -> f64 {
    jsV_numbertointeger(jsV_tonumber(J, v)) as f64
}

/* ToString() on a number */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_numbertostring(
    J: *mut js_State,
    buf: *mut c_char,
    f: f64,
) -> *const c_char {
    let mut digits: [c_char; 32] = [0; 32];
    let mut p: *mut c_char = buf;
    let mut s: *const c_char;
    let mut exp: c_int = 0;
    let mut ndigits: c_int;
    let mut point: c_int;

    if f == 0.0 {
        return c"0".as_ptr();
    }
    if isnan(f) {
        return c"NaN".as_ptr();
    }
    if isinf(f) {
        return if f < 0.0 {
            c"-Infinity".as_ptr()
        } else {
            c"Infinity".as_ptr()
        };
    }

    /* Fast case for integers. This only works assuming all integers can be
     * exactly represented by a float. This is true for 32-bit integers and
     * 64-bit floats. */
    if f >= INT_MIN as f64 && f <= INT_MAX as f64 {
        let i: c_int = f as c_int;
        if i as f64 == f {
            return js_itoa(buf, i);
        }
    }

    ndigits = js_grisu2(f, digits.as_mut_ptr(), &mut exp);
    s = digits.as_ptr();
    point = ndigits + exp;

    if signbit(f) {
        *p = '-' as c_char;
        p = p.add(1);
    }

    if point < -5 || point > 21 {
        *p = *s;
        p = p.add(1);
        s = s.add(1);
        if ndigits > 1 {
            let mut n: c_int = ndigits - 1;
            *p = '.' as c_char;
            p = p.add(1);
            while n != 0 {
                n -= 1;
                *p = *s;
                p = p.add(1);
                s = s.add(1);
            }
        }
        js_fmtexp(p, point - 1);
    } else if point <= 0 {
        *p = '0' as c_char;
        p = p.add(1);
        *p = '.' as c_char;
        p = p.add(1);
        while point < 0 {
            point += 1;
            *p = '0' as c_char;
            p = p.add(1);
        }
        point += 1;
        while ndigits > 0 {
            ndigits -= 1;
            *p = *s;
            p = p.add(1);
            s = s.add(1);
        }
        *p = 0;
    } else {
        while ndigits > 0 {
            ndigits -= 1;
            *p = *s;
            p = p.add(1);
            s = s.add(1);
            point -= 1;
            if point == 0 && ndigits > 0 {
                *p = '.' as c_char;
                p = p.add(1);
            }
        }
        while point > 0 {
            point -= 1;
            *p = '0' as c_char;
            p = p.add(1);
        }
        *p = 0;
    }

    buf
}

/* ToString() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_tostring(J: *mut js_State, v: *mut js_Value) -> *const c_char {
    let mut buf: [c_char; 32] = [0; 32];
    let mut p: *const c_char;
    match (*v).t.r#type {
        JS_TUNDEFINED => c"undefined".as_ptr(),
        JS_TNULL => c"null".as_ptr(),
        JS_TBOOLEAN => {
            if (*v).u.boolean != 0 {
                c"true".as_ptr()
            } else {
                c"false".as_ptr()
            }
        }
        JS_TLITSTR => (*v).u.litstr,
        JS_TMEMSTR => js_String_p((*v).u.memstr),
        JS_TNUMBER => {
            p = jsV_numbertostring(J, buf.as_mut_ptr(), (*v).u.number);
            if p == buf.as_ptr() {
                let mut n: c_int = strlen(p) as c_int;
                if n <= SOFFSETOF_JS_VALUE_TYPE {
                    let mut s: *mut c_char = js_Value_shrstr(v);
                    while n != 0 {
                        n -= 1;
                        *s = *p;
                        s = s.add(1);
                        p = p.add(1);
                    }
                    *s = 0;
                    (*v).t.r#type = JS_TSHRSTR;
                    return js_Value_shrstr(v);
                } else {
                    (*v).u.memstr = jsV_newmemstring(J, p, n);
                    (*v).t.r#type = JS_TMEMSTR;
                    return js_String_p((*v).u.memstr);
                }
            }
            p
        }
        JS_TOBJECT => {
            jsV_toprimitive(J, v, JS_HSTRING);
            jsV_tostring(J, v)
        }
        /* JS_TSHRSTR and default */
        _ => js_Value_shrstr(v),
    }
}

/* Objects */

unsafe fn jsV_newboolean(J: *mut js_State, v: c_int) -> *mut js_Object {
    let obj: *mut js_Object = jsV_newobject(J, JS_CBOOLEAN, (*J).Boolean_prototype);
    (*obj).u.boolean = v;
    obj
}

unsafe fn jsV_newnumber(J: *mut js_State, v: f64) -> *mut js_Object {
    let obj: *mut js_Object = jsV_newobject(J, JS_CNUMBER, (*J).Number_prototype);
    (*obj).u.number = v;
    obj
}

unsafe fn jsV_newstring(J: *mut js_State, v: *const c_char) -> *mut js_Object {
    let obj: *mut js_Object = jsV_newobject(J, JS_CSTRING, (*J).String_prototype);
    let n: usize = strlen(v);
    if n < 16 {
        (*obj).u.s.string = js_Object_shrstr(obj);
        memcpy(
            js_Object_shrstr(obj) as *mut c_void,
            v as *const c_void,
            n + 1,
        );
    } else {
        (*obj).u.s.string = js_strdup(J, v);
    }
    (*obj).u.s.length = js_utflen(v);
    obj
}

/* ToObject() on a value */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsV_toobject(J: *mut js_State, v: *mut js_Value) -> *mut js_Object {
    let o: *mut js_Object;
    match (*v).t.r#type {
        JS_TUNDEFINED => js_typeerror!(J, c"cannot convert undefined to object".as_ptr()),
        JS_TNULL => js_typeerror!(J, c"cannot convert null to object".as_ptr()),
        JS_TOBJECT => return (*v).u.object,
        JS_TSHRSTR => o = jsV_newstring(J, js_Value_shrstr(v)),
        JS_TLITSTR => o = jsV_newstring(J, (*v).u.litstr),
        JS_TMEMSTR => o = jsV_newstring(J, js_String_p((*v).u.memstr)),
        JS_TBOOLEAN => o = jsV_newboolean(J, (*v).u.boolean),
        JS_TNUMBER => o = jsV_newnumber(J, (*v).u.number),
        _ => js_typeerror!(J, c"cannot convert undefined to object".as_ptr()),
    }
    (*v).t.r#type = JS_TOBJECT;
    (*v).u.object = o;
    o
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newobjectx(J: *mut js_State) {
    let mut prototype: *mut js_Object = null_mut();
    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, prototype));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newobject(J: *mut js_State) {
    js_pushobject(J, jsV_newobject(J, JS_COBJECT, (*J).Object_prototype));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newarguments(J: *mut js_State) {
    js_pushobject(J, jsV_newobject(J, JS_CARGUMENTS, (*J).Object_prototype));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newarray(J: *mut js_State) {
    let obj: *mut js_Object = jsV_newobject(J, JS_CARRAY, (*J).Array_prototype);
    (*obj).u.a.simple = 1;
    js_pushobject(J, obj);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newboolean(J: *mut js_State, v: c_int) {
    js_pushobject(J, jsV_newboolean(J, v));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newnumber(J: *mut js_State, v: f64) {
    js_pushobject(J, jsV_newnumber(J, v));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newstring(J: *mut js_State, v: *const c_char) {
    js_pushobject(J, jsV_newstring(J, v));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newfunction(
    J: *mut js_State,
    fun: *mut js_Function,
    scope: *mut js_Environment,
) {
    let obj: *mut js_Object = jsV_newobject(J, JS_CFUNCTION, (*J).Function_prototype);
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
    {
        js_pushnumber(J, (*fun).numparams as f64);
        js_defproperty(
            J,
            -2,
            c"length".as_ptr(),
            JS_READONLY | JS_DONTENUM | JS_DONTCONF,
        );
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        }
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newscript(
    J: *mut js_State,
    fun: *mut js_Function,
    scope: *mut js_Environment,
) {
    let obj: *mut js_Object = jsV_newobject(J, JS_CSCRIPT, null_mut());
    (*obj).u.f.function = fun;
    (*obj).u.f.scope = scope;
    js_pushobject(J, obj);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcfunctionx(
    J: *mut js_State,
    cfun: js_CFunction,
    name: *const c_char,
    length: c_int,
    data: *mut c_void,
    finalize: js_Finalize,
) {
    let obj: *mut js_Object;

    if js_try!(J) {
        if let Some(fin) = finalize {
            fin(J, data);
        }
        js_throw(J);
    }

    obj = jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = None;
    (*obj).u.c.length = length;
    (*obj).u.c.data = data;
    (*obj).u.c.finalize = finalize;

    js_endtry(J);

    js_pushobject(J, obj);
    {
        js_pushnumber(J, length as f64);
        js_defproperty(
            J,
            -2,
            c"length".as_ptr(),
            JS_READONLY | JS_DONTENUM | JS_DONTCONF,
        );
        js_newobject(J);
        {
            js_copy(J, -2);
            js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        }
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcfunction(
    J: *mut js_State,
    cfun: js_CFunction,
    name: *const c_char,
    length: c_int,
) {
    js_newcfunctionx(J, cfun, name, length, null_mut(), None);
}

/* prototype -- constructor */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newcconstructor(
    J: *mut js_State,
    cfun: js_CFunction,
    ccon: js_CFunction,
    name: *const c_char,
    length: c_int,
) {
    let obj: *mut js_Object = jsV_newobject(J, JS_CCFUNCTION, (*J).Function_prototype);
    (*obj).u.c.name = name;
    (*obj).u.c.function = cfun;
    (*obj).u.c.constructor = ccon;
    (*obj).u.c.length = length;
    js_pushobject(J, obj); /* proto obj */
    {
        js_pushnumber(J, length as f64);
        js_defproperty(
            J,
            -2,
            c"length".as_ptr(),
            JS_READONLY | JS_DONTENUM | JS_DONTCONF,
        );
        js_rot2(J); /* obj proto */
        js_copy(J, -2); /* obj proto obj */
        js_defproperty(J, -2, c"constructor".as_ptr(), JS_DONTENUM);
        js_defproperty(J, -2, c"prototype".as_ptr(), JS_DONTENUM | JS_DONTCONF);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newuserdatax(
    J: *mut js_State,
    tag: *const c_char,
    data: *mut c_void,
    has: js_HasProperty,
    put: js_Put,
    delete: js_Delete,
    finalize: js_Finalize,
) {
    let mut prototype: *mut js_Object = null_mut();
    let obj: *mut js_Object;

    if js_isobject(J, -1) != 0 {
        prototype = js_toobject(J, -1);
    }
    js_pop(J, 1);

    if js_try!(J) {
        if let Some(fin) = finalize {
            fin(J, data);
        }
        js_throw(J);
    }

    obj = jsV_newobject(J, JS_CUSERDATA, prototype);
    (*obj).u.user.tag = tag;
    (*obj).u.user.data = data;
    (*obj).u.user.has = has;
    (*obj).u.user.put = put;
    (*obj).u.user.delete = delete;
    (*obj).u.user.finalize = finalize;

    js_endtry(J);

    js_pushobject(J, obj);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_newuserdata(
    J: *mut js_State,
    tag: *const c_char,
    data: *mut c_void,
    finalize: js_Finalize,
) {
    js_newuserdatax(J, tag, data, None, None, None, finalize);
}

/* Non-trivial operations on values. These are implemented using the stack. */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_instanceof(J: *mut js_State) -> c_int {
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
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_concat(J: *mut js_State) {
    js_toprimitive(J, -2, JS_HNONE);
    js_toprimitive(J, -1, JS_HNONE);

    if js_isstring(J, -2) != 0 || js_isstring(J, -1) != 0 {
        let sa: *const c_char = js_tostring(J, -2);
        let sb: *const c_char = js_tostring(J, -1);
        let mut sab: *mut c_char = null_mut(); /* volatile */
        /* TODO: create js_String directly */
        if js_try!(J) {
            js_free(J, vread(&sab) as *mut c_void);
            js_throw(J);
        }
        vwrite(
            &mut sab,
            js_malloc(J, (strlen(sa) + strlen(sb) + 1) as c_int) as *mut c_char,
        );
        strcpy(vread(&sab), sa);
        strcat(vread(&sab), sb);
        js_pop(J, 2);
        js_pushstring(J, vread(&sab) as *const c_char);
        js_endtry(J);
        js_free(J, vread(&sab) as *mut c_void);
    } else {
        let x: f64 = js_tonumber(J, -2);
        let y: f64 = js_tonumber(J, -1);
        js_pop(J, 2);
        js_pushnumber(J, x + y);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_compare(J: *mut js_State, okay: *mut c_int) -> c_int {
    js_toprimitive(J, -2, JS_HNUMBER);
    js_toprimitive(J, -1, JS_HNUMBER);

    *okay = 1;
    if js_isstring(J, -2) != 0 && js_isstring(J, -1) != 0 {
        strcmp(js_tostring(J, -2), js_tostring(J, -1))
    } else {
        let x: f64 = js_tonumber(J, -2);
        let y: f64 = js_tonumber(J, -1);
        if isnan(x) || isnan(y) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_equal(J: *mut js_State) -> c_int {
    let x: *mut js_Value = js_tovalue(J, -2);
    let y: *mut js_Value = js_tovalue(J, -1);

    loop {
        /* retry: */
        if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
            return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
        }
        if (*x).t.r#type == (*y).t.r#type {
            if (*x).t.r#type == JS_TUNDEFINED {
                return 1;
            }
            if (*x).t.r#type == JS_TNULL {
                return 1;
            }
            if (*x).t.r#type == JS_TNUMBER {
                return ((*x).u.number == (*y).u.number) as c_int;
            }
            if (*x).t.r#type == JS_TBOOLEAN {
                return ((*x).u.boolean == (*y).u.boolean) as c_int;
            }
            if (*x).t.r#type == JS_TOBJECT {
                return ((*x).u.object == (*y).u.object) as c_int;
            }
            return 0;
        }

        if (*x).t.r#type == JS_TNULL && (*y).t.r#type == JS_TUNDEFINED {
            return 1;
        }
        if (*x).t.r#type == JS_TUNDEFINED && (*y).t.r#type == JS_TNULL {
            return 1;
        }

        if (*x).t.r#type == JS_TNUMBER && JSV_ISSTRING(y) {
            return ((*x).u.number == jsV_tonumber(J, y)) as c_int;
        }
        if JSV_ISSTRING(x) && (*y).t.r#type == JS_TNUMBER {
            return (jsV_tonumber(J, x) == (*y).u.number) as c_int;
        }

        if (*x).t.r#type == JS_TBOOLEAN {
            let b = (*x).u.boolean;
            (*x).t.r#type = JS_TNUMBER;
            (*x).u.number = if b != 0 { 1.0 } else { 0.0 };
            continue;
        }
        if (*y).t.r#type == JS_TBOOLEAN {
            let b = (*y).u.boolean;
            (*y).t.r#type = JS_TNUMBER;
            (*y).u.number = if b != 0 { 1.0 } else { 0.0 };
            continue;
        }
        if (JSV_ISSTRING(x) || (*x).t.r#type == JS_TNUMBER) && (*y).t.r#type == JS_TOBJECT {
            jsV_toprimitive(J, y, JS_HNONE);
            continue;
        }
        if (*x).t.r#type == JS_TOBJECT && (JSV_ISSTRING(y) || (*y).t.r#type == JS_TNUMBER) {
            jsV_toprimitive(J, x, JS_HNONE);
            continue;
        }

        return 0;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_strictequal(J: *mut js_State) -> c_int {
    let x: *mut js_Value = js_tovalue(J, -2);
    let y: *mut js_Value = js_tovalue(J, -1);

    if JSV_ISSTRING(x) && JSV_ISSTRING(y) {
        return (strcmp(JSV_TOSTRING(x), JSV_TOSTRING(y)) == 0) as c_int;
    }

    if (*x).t.r#type != (*y).t.r#type {
        return 0;
    }
    if (*x).t.r#type == JS_TUNDEFINED {
        return 1;
    }
    if (*x).t.r#type == JS_TNULL {
        return 1;
    }
    if (*x).t.r#type == JS_TNUMBER {
        return ((*x).u.number == (*y).u.number) as c_int;
    }
    if (*x).t.r#type == JS_TBOOLEAN {
        return ((*x).u.boolean == (*y).u.boolean) as c_int;
    }
    if (*x).t.r#type == JS_TOBJECT {
        return ((*x).u.object == (*y).u.object) as c_int;
    }
    0
}
