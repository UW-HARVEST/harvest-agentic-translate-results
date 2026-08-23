//! Translated from c_src/src/json.c
use crate::jsi::*;
use crate::prelude::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isnumberobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).r#type == JS_CNUMBER) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isstringobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).r#type == JS_CSTRING) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isbooleanobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).r#type == JS_CBOOLEAN) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isdateobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).r#type == JS_CDATE) as c_int
}

unsafe fn jsonnext(J: *mut js_State) {
    (*J).lookahead = jsY_lexjson(J);
}

unsafe fn jsonaccept(J: *mut js_State, t: c_int) -> c_int {
    if (*J).lookahead == t {
        jsonnext(J);
        return 1;
    }
    0
}

unsafe fn jsonexpect(J: *mut js_State, t: c_int) {
    if jsonaccept(J, t) == 0 {
        js_syntaxerror!(
            J,
            c"JSON: unexpected token: %s (expected %s)".as_ptr(),
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(t)
        );
    }
}

unsafe fn jsonvalue(J: *mut js_State) {
    let mut i: c_int;

    match (*J).lookahead {
        TK_STRING => {
            js_pushstring(J, (*J).text);
            jsonnext(J);
        }

        TK_NUMBER => {
            js_pushnumber(J, (*J).number);
            jsonnext(J);
        }

        0x7B /* '{' */ => {
            js_newobject(J);
            jsonnext(J);
            if jsonaccept(J, '}' as c_int) != 0 {
                return;
            }
            loop {
                if (*J).lookahead != TK_STRING {
                    js_syntaxerror!(
                        J,
                        c"JSON: unexpected token: %s (expected string)".as_ptr(),
                        jsY_tokenstring((*J).lookahead)
                    );
                }
                js_pushstring(J, (*J).text);
                jsonnext(J);
                jsonexpect(J, ':' as c_int);
                jsonvalue(J);
                js_setproperty(J, -3, js_tostring(J, -2));
                js_pop(J, 1);
                if jsonaccept(J, ',' as c_int) == 0 {
                    break;
                }
            }
            jsonexpect(J, '}' as c_int);
        }

        0x5B /* '[' */ => {
            js_newarray(J);
            jsonnext(J);
            i = 0;
            if jsonaccept(J, ']' as c_int) != 0 {
                return;
            }
            loop {
                jsonvalue(J);
                js_setindex(J, -2, i);
                i += 1;
                if jsonaccept(J, ',' as c_int) == 0 {
                    break;
                }
            }
            jsonexpect(J, ']' as c_int);
        }

        TK_TRUE => {
            js_pushboolean(J, 1);
            jsonnext(J);
        }

        TK_FALSE => {
            js_pushboolean(J, 0);
            jsonnext(J);
        }

        TK_NULL => {
            js_pushnull(J);
            jsonnext(J);
        }

        _ => {
            js_syntaxerror!(
                J,
                c"JSON: unexpected token: %s".as_ptr(),
                jsY_tokenstring((*J).lookahead)
            );
        }
    }
}

unsafe fn jsonrevive(J: *mut js_State, name: *const c_char) {
    let mut key: *const c_char;
    let mut buf: [c_char; 32] = [0; 32];

    /* revive is in 2 */
    /* holder is in -1 */

    js_getproperty(J, -1, name); /* get value from holder */

    if js_isobject(J, -1) != 0 {
        if js_isarray(J, -1) != 0 {
            let mut i: c_int = 0;
            let n: c_int = js_getlength(J, -1);
            i = 0;
            while i < n {
                jsonrevive(J, js_itoa(buf.as_mut_ptr(), i));
                if js_isundefined(J, -1) != 0 {
                    js_pop(J, 1);
                    js_delproperty(J, -1, buf.as_ptr());
                } else {
                    js_setproperty(J, -2, buf.as_ptr());
                }
                i += 1;
            }
        } else {
            js_pushiterator(J, -1, 1);
            loop {
                key = js_nextiterator(J, -1);
                if key.is_null() {
                    break;
                }
                js_rot2(J);
                jsonrevive(J, key);
                if js_isundefined(J, -1) != 0 {
                    js_pop(J, 1);
                    js_delproperty(J, -1, key);
                } else {
                    js_setproperty(J, -2, key);
                }
                js_rot2(J);
            }
            js_pop(J, 1);
        }
    }

    js_copy(J, 2); /* reviver function */
    js_copy(J, -3); /* holder as this */
    js_pushstring(J, name); /* name */
    js_copy(J, -4); /* value */
    js_call(J, 2);
    js_rot2pop1(J); /* pop old value, leave new value on stack */
}

unsafe extern "C" fn JSON_parse(J: *mut js_State) {
    let source: *const c_char = js_tostring(J, 1);
    jsY_initlex(J, c"JSON".as_ptr(), source);
    jsonnext(J);

    if js_iscallable(J, 2) != 0 {
        js_newobject(J);
        jsonvalue(J);
        js_defproperty(J, -2, c"".as_ptr(), 0);
        jsonrevive(J, c"".as_ptr());
    } else {
        jsonvalue(J);
    }
}

unsafe fn fmtnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    if isnan(n) {
        js_puts(J, sb, c"null".as_ptr());
    } else if isinf(n) {
        js_puts(J, sb, c"null".as_ptr());
    } else if n == 0.0 {
        js_puts(J, sb, c"0".as_ptr());
    } else {
        let mut buf: [c_char; 40] = [0; 40];
        js_puts(J, sb, jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn fmtstr(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    let HEX: *const c_char = c"0123456789abcdef".as_ptr();
    let mut i: c_int;
    let mut n: c_int;
    let mut c: Rune = 0;
    let mut s: *const c_char = s;
    js_putc(J, sb, '"' as c_int);
    while *s != 0 {
        n = jsU_chartorune(&mut c as *mut Rune, s);
        match c {
            0x22 /* '"' */ => js_puts(J, sb, c"\\\"".as_ptr()),
            0x5C /* '\\' */ => js_puts(J, sb, c"\\\\".as_ptr()),
            0x08 /* '\b' */ => js_puts(J, sb, c"\\b".as_ptr()),
            0x0C /* '\f' */ => js_puts(J, sb, c"\\f".as_ptr()),
            0x0A /* '\n' */ => js_puts(J, sb, c"\\n".as_ptr()),
            0x0D /* '\r' */ => js_puts(J, sb, c"\\r".as_ptr()),
            0x09 /* '\t' */ => js_puts(J, sb, c"\\t".as_ptr()),
            _ => {
                if c < ' ' as c_int || (c >= 0xd800 && c <= 0xdfff) {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'u' as c_int);
                    js_putc(J, sb, *HEX.add((((c >> 12) & 15) as usize)) as c_int);
                    js_putc(J, sb, *HEX.add((((c >> 8) & 15) as usize)) as c_int);
                    js_putc(J, sb, *HEX.add((((c >> 4) & 15) as usize)) as c_int);
                    js_putc(J, sb, *HEX.add(((c & 15) as usize)) as c_int);
                } else if c < 128 {
                    js_putc(J, sb, c);
                } else {
                    i = 0;
                    while i < n {
                        js_putc(J, sb, *s.add(i as usize) as c_int);
                        i += 1;
                    }
                }
            }
        }
        s = s.add(n as usize);
    }
    js_putc(J, sb, '"' as c_int);
}

unsafe fn fmtindent(J: *mut js_State, sb: *mut *mut js_Buffer, gap: *const c_char, level: c_int) {
    let mut level: c_int = level;
    js_putc(J, sb, '\n' as c_int);
    loop {
        let t: c_int = level;
        level -= 1;
        if t == 0 {
            break;
        }
        js_puts(J, sb, gap);
    }
}

unsafe fn filterprop(J: *mut js_State, key: *const c_char) -> c_int {
    let mut i: c_int;
    let n: c_int;
    let mut found: c_int;
    /* replacer/property-list is in stack slot 2 */
    if js_isarray(J, 2) != 0 {
        found = 0;
        n = js_getlength(J, 2);
        i = 0;
        while i < n && found == 0 {
            js_getindex(J, 2, i);
            if js_isstring(J, -1) != 0
                || js_isnumber(J, -1) != 0
                || js_isstringobject(J, -1) != 0
                || js_isnumberobject(J, -1) != 0
            {
                found = (strcmp(key, js_tostring(J, -1)) == 0) as c_int;
            }
            js_pop(J, 1);
            i += 1;
        }
        return found;
    }
    1
}

unsafe fn fmtobject(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    obj: *mut js_Object,
    gap: *const c_char,
    level: c_int,
) {
    let mut key: *const c_char;
    let mut save: c_int;
    let mut i: c_int;
    let mut n: c_int;

    n = js_gettop(J) - 1;
    i = 4;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_typeerror!(J, c"cyclic object value".as_ptr());
            }
        }
        i += 1;
    }

    n = 0;
    js_putc(J, sb, '{' as c_int);
    js_pushiterator(J, -1, 1);
    loop {
        key = js_nextiterator(J, -1);
        if key.is_null() {
            break;
        }
        if filterprop(J, key) != 0 {
            save = (*(*sb)).n;
            if n != 0 {
                js_putc(J, sb, ',' as c_int);
            }
            if !gap.is_null() {
                fmtindent(J, sb, gap, level + 1);
            }
            fmtstr(J, sb, key);
            js_putc(J, sb, ':' as c_int);
            if !gap.is_null() {
                js_putc(J, sb, ' ' as c_int);
            }
            js_rot2(J);
            if fmtvalue(J, sb, key, gap, level + 1) == 0 {
                (*(*sb)).n = save;
            } else {
                n += 1;
            }
            js_rot2(J);
        }
    }
    js_pop(J, 1);
    if !gap.is_null() && n != 0 {
        fmtindent(J, sb, gap, level);
    }
    js_putc(J, sb, '}' as c_int);
}

unsafe fn fmtarray(J: *mut js_State, sb: *mut *mut js_Buffer, gap: *const c_char, level: c_int) {
    let mut n: c_int;
    let mut i: c_int;
    let mut buf: [c_char; 32] = [0; 32];

    n = js_gettop(J) - 1;
    i = 4;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_typeerror!(J, c"cyclic object value".as_ptr());
            }
        }
        i += 1;
    }

    js_putc(J, sb, '[' as c_int);
    n = js_getlength(J, -1);
    i = 0;
    while i < n {
        if i != 0 {
            js_putc(J, sb, ',' as c_int);
        }
        if !gap.is_null() {
            fmtindent(J, sb, gap, level + 1);
        }
        if fmtvalue(J, sb, js_itoa(buf.as_mut_ptr(), i), gap, level + 1) == 0 {
            js_puts(J, sb, c"null".as_ptr());
        }
        i += 1;
    }
    if !gap.is_null() && n != 0 {
        fmtindent(J, sb, gap, level);
    }
    js_putc(J, sb, ']' as c_int);
}

unsafe fn fmtvalue(
    J: *mut js_State,
    sb: *mut *mut js_Buffer,
    key: *const c_char,
    gap: *const c_char,
    level: c_int,
) -> c_int {
    /* replacer/property-list is in 2 */
    /* holder is in -1 */

    js_getproperty(J, -1, key);

    if js_isobject(J, -1) != 0 {
        if js_hasproperty(J, -1, c"toJSON".as_ptr()) != 0 {
            if js_iscallable(J, -1) != 0 {
                js_copy(J, -2);
                js_pushstring(J, key);
                js_call(J, 1);
                js_rot2pop1(J);
            } else {
                js_pop(J, 1);
            }
        }
    }

    if js_iscallable(J, 2) != 0 {
        js_copy(J, 2); /* replacer function */
        js_copy(J, -3); /* holder as this */
        js_pushstring(J, key); /* name */
        js_copy(J, -4); /* old value */
        js_call(J, 2);
        js_rot2pop1(J); /* pop old value, leave new value on stack */
    }

    if js_isobject(J, -1) != 0 && js_iscallable(J, -1) == 0 {
        let obj: *mut js_Object = js_toobject(J, -1);
        match (*obj).r#type {
            JS_CNUMBER => fmtnum(J, sb, (*obj).u.number),
            JS_CSTRING => fmtstr(J, sb, (*obj).u.s.string),
            JS_CBOOLEAN => js_puts(
                J,
                sb,
                if (*obj).u.boolean != 0 {
                    c"true".as_ptr()
                } else {
                    c"false".as_ptr()
                },
            ),
            JS_CARRAY => fmtarray(J, sb, gap, level),
            _ => fmtobject(J, sb, obj, gap, level),
        }
    } else if js_isboolean(J, -1) != 0 {
        js_puts(
            J,
            sb,
            if js_toboolean(J, -1) != 0 {
                c"true".as_ptr()
            } else {
                c"false".as_ptr()
            },
        );
    } else if js_isnumber(J, -1) != 0 {
        fmtnum(J, sb, js_tonumber(J, -1));
    } else if js_isstring(J, -1) != 0 {
        fmtstr(J, sb, js_tostring(J, -1));
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, c"null".as_ptr());
    } else {
        js_pop(J, 1);
        return 0;
    }

    js_pop(J, 1);
    1
}

unsafe extern "C" fn JSON_stringify(J: *mut js_State) {
    let mut sb: *mut js_Buffer = null_mut();
    let mut buf: [c_char; 12] = [0; 12];
    /* NOTE: volatile to silence GCC warning about longjmp clobbering a variable */
    let mut gap: *const c_char = null();
    let s: *const c_char;
    let mut n: c_int;

    vwrite(&mut gap, null());

    if js_isnumber(J, 3) != 0 || js_isnumberobject(J, 3) != 0 {
        n = js_tointeger(J, 3);
        if n < 0 {
            n = 0;
        }
        if n > 10 {
            n = 10;
        }
        memset(buf.as_mut_ptr() as *mut c_void, ' ' as c_int, n as usize);
        buf[n as usize] = 0;
        if n > 0 {
            vwrite(&mut gap, buf.as_ptr());
        }
    } else if js_isstring(J, 3) != 0 || js_isstringobject(J, 3) != 0 {
        s = js_tostring(J, 3);
        n = strlen(s) as c_int;
        if n > 10 {
            n = 10;
        }
        memcpy(buf.as_mut_ptr() as *mut c_void, s as *const c_void, n as usize);
        buf[n as usize] = 0;
        if n > 0 {
            vwrite(&mut gap, buf.as_ptr());
        }
    }

    if js_try!(J) {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }

    js_newobject(J); /* wrapper */
    js_copy(J, 1);
    js_defproperty(J, -2, c"".as_ptr(), 0);
    if fmtvalue(J, &mut sb, c"".as_ptr(), vread(&gap), 0) == 0 {
        js_pushundefined(J);
    } else {
        js_putc(J, &mut sb, 0);
        js_pushstring(
            J,
            if !sb.is_null() {
                js_Buffer_s(sb) as *const c_char
            } else {
                c"".as_ptr()
            },
        );
        js_rot2pop1(J);
    }

    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsB_initjson(J: *mut js_State) {
    js_pushobject(J, jsV_newobject(J, JS_CJSON, (*J).Object_prototype));
    {
        jsB_propf(J, c"JSON.parse".as_ptr(), Some(JSON_parse), 2);
        jsB_propf(J, c"JSON.stringify".as_ptr(), Some(JSON_stringify), 3);
    }
    js_defglobal(J, c"JSON".as_ptr(), JS_DONTENUM);
}
