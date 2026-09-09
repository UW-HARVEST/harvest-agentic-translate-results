//! Translation of json.c

use crate::*;

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isnumberobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CNUMBER) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isstringobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CSTRING) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isbooleanobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CBOOLEAN) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_isdateobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CDATE) as c_int
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
            "JSON: unexpected token: %s (expected %s)",
            jsY_tokenstring((*J).lookahead),
            jsY_tokenstring(t)
        );
    }
}

unsafe fn jsonvalue(J: *mut js_State) {
    let mut i: c_int;

    let la = (*J).lookahead;
    if la == TK_STRING {
        js_pushstring(J, (*J).text);
        jsonnext(J);
    } else if la == TK_NUMBER {
        js_pushnumber(J, (*J).number);
        jsonnext(J);
    } else if la == '{' as c_int {
        js_newobject(J);
        jsonnext(J);
        if jsonaccept(J, '}' as c_int) != 0 {
            return;
        }
        loop {
            if (*J).lookahead != TK_STRING {
                js_syntaxerror!(
                    J,
                    "JSON: unexpected token: %s (expected string)",
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
    } else if la == '[' as c_int {
        js_newarray(J);
        jsonnext(J);
        i = 0;
        if jsonaccept(J, ']' as c_int) != 0 {
            return;
        }
        loop {
            jsonvalue(J);
            let t = i;
            i += 1;
            js_setindex(J, -2, t);
            if jsonaccept(J, ',' as c_int) == 0 {
                break;
            }
        }
        jsonexpect(J, ']' as c_int);
    } else if la == TK_TRUE {
        js_pushboolean(J, 1);
        jsonnext(J);
    } else if la == TK_FALSE {
        js_pushboolean(J, 0);
        jsonnext(J);
    } else if la == TK_NULL {
        js_pushnull(J);
        jsonnext(J);
    } else {
        js_syntaxerror!(
            J,
            "JSON: unexpected token: %s",
            jsY_tokenstring((*J).lookahead)
        );
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
            let mut i: c_int;
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
    jsY_initlex(J, cs!("JSON"), source);
    jsonnext(J);

    if js_iscallable(J, 2) != 0 {
        js_newobject(J);
        jsonvalue(J);
        js_defproperty(J, -2, cs!(""), 0);
        jsonrevive(J, cs!(""));
    } else {
        jsonvalue(J);
    }
}

unsafe fn fmtnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    if isnan(n) {
        js_puts(J, sb, cs!("null"));
    } else if isinf(n) {
        js_puts(J, sb, cs!("null"));
    } else if n == 0.0 {
        js_puts(J, sb, cs!("0"));
    } else {
        let mut buf: [c_char; 40] = [0; 40];
        js_puts(J, sb, jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn fmtstr(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    let HEX: *const c_char = cs!("0123456789abcdef");
    let mut i: c_int;
    let mut n: c_int;
    let mut c: Rune = 0;
    let mut s = s;
    js_putc(J, sb, '"' as c_int);
    while *s != 0 {
        n = jsU_chartorune(&mut c, s);
        if c == '"' as c_int {
            js_puts(J, sb, cs!("\\\""));
        } else if c == '\\' as c_int {
            js_puts(J, sb, cs!("\\\\"));
        } else if c == 8 {
            /* '\b' */
            js_puts(J, sb, cs!("\\b"));
        } else if c == 12 {
            /* '\f' */
            js_puts(J, sb, cs!("\\f"));
        } else if c == '\n' as c_int {
            js_puts(J, sb, cs!("\\n"));
        } else if c == '\r' as c_int {
            js_puts(J, sb, cs!("\\r"));
        } else if c == '\t' as c_int {
            js_puts(J, sb, cs!("\\t"));
        } else {
            if c < ' ' as c_int || (c >= 0xd800 && c <= 0xdfff) {
                js_putc(J, sb, '\\' as c_int);
                js_putc(J, sb, 'u' as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 12) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 8) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 4) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset((c & 15) as isize) as c_int);
            } else if c < 128 {
                js_putc(J, sb, c);
            } else {
                i = 0;
                while i < n {
                    js_putc(J, sb, *s.offset(i as isize) as c_int);
                    i += 1;
                }
            }
        }
        s = s.offset(n as isize);
    }
    js_putc(J, sb, '"' as c_int);
}

unsafe fn fmtindent(J: *mut js_State, sb: *mut *mut js_Buffer, gap: *const c_char, level: c_int) {
    let mut level = level;
    js_putc(J, sb, '\n' as c_int);
    loop {
        let t = level;
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
                js_typeerror!(J, "cyclic object value");
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
            save = (**sb).n;
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
                (**sb).n = save;
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
                js_typeerror!(J, "cyclic object value");
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
            js_puts(J, sb, cs!("null"));
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
        if js_hasproperty(J, -1, cs!("toJSON")) != 0 {
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
        match (*obj).type_ {
            JS_CNUMBER => fmtnum(J, sb, (*obj).u.number),
            JS_CSTRING => fmtstr(J, sb, (*obj).u.s.string),
            JS_CBOOLEAN => js_puts(
                J,
                sb,
                if (*obj).u.boolean != 0 {
                    cs!("true")
                } else {
                    cs!("false")
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
                cs!("true")
            } else {
                cs!("false")
            },
        );
    } else if js_isnumber(J, -1) != 0 {
        fmtnum(J, sb, js_tonumber(J, -1));
    } else if js_isstring(J, -1) != 0 {
        fmtstr(J, sb, js_tostring(J, -1));
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, cs!("null"));
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

    setvol!(gap, null());

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
            setvol!(gap, buf.as_ptr() as *const c_char);
        }
    } else if js_isstring(J, 3) != 0 || js_isstringobject(J, 3) != 0 {
        s = js_tostring(J, 3);
        n = strlen(s) as c_int;
        if n > 10 {
            n = 10;
        }
        memcpy(
            buf.as_mut_ptr() as *mut c_void,
            s as *const c_void,
            n as usize,
        );
        buf[n as usize] = 0;
        if n > 0 {
            setvol!(gap, buf.as_ptr() as *const c_char);
        }
    }

    if js_try!(J) != 0 {
        js_free(J, vol!(sb) as *mut c_void);
        js_throw(J);
    }

    js_newobject(J); /* wrapper */
    js_copy(J, 1);
    js_defproperty(J, -2, cs!(""), 0);
    if fmtvalue(J, addr_of_mut!(sb), cs!(""), vol!(gap), 0) == 0 {
        js_pushundefined(J);
    } else {
        js_putc(J, addr_of_mut!(sb), 0);
        js_pushstring(
            J,
            if !sb.is_null() {
                addr_of!((*sb).s) as *const c_char
            } else {
                cs!("")
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
        jsB_propf(J, cs!("JSON.parse"), Some(JSON_parse), 2);
        jsB_propf(J, cs!("JSON.stringify"), Some(JSON_stringify), 3);
    }
    js_defglobal(J, cs!("JSON"), JS_DONTENUM);
}
