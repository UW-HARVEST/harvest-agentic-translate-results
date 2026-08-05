//! Translated from json.c — JSON.parse / JSON.stringify and type predicates.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsintern::{js_putc, js_puts};
use crate::jsrun::*;
use crate::types::*;
use crate::utf::chartorune;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isnumberobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CNUMBER) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isstringobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CSTRING) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isbooleanobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CBOOLEAN) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_isdateobject(J: *mut js_State, idx: c_int) -> c_int {
    (js_isobject(J, idx) != 0 && (*js_toobject(J, idx)).type_ == JS_CDATE) as c_int
}

unsafe fn jsonnext(J: *mut js_State) {
    (*J).lookahead = crate::jslex::jsY_lexjson(J);
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
        crate::jserror::js_syntaxerror(
            J,
            cstr!("JSON: unexpected token: %s (expected %s)"),
            crate::jslex::jsY_tokenstring((*J).lookahead),
            crate::jslex::jsY_tokenstring(t),
        );
    }
}

unsafe fn jsonvalue(J: *mut js_State) {
    let mut i;

    match (*J).lookahead {
        x if x == TK_STRING => {
            js_pushstring(J, (*J).text);
            jsonnext(J);
        }
        x if x == TK_NUMBER => {
            js_pushnumber(J, (*J).number);
            jsonnext(J);
        }
        x if x == '{' as c_int => {
            crate::jsvalue::js_newobject(J);
            jsonnext(J);
            if jsonaccept(J, '}' as c_int) != 0 {
                return;
            }
            loop {
                if (*J).lookahead != TK_STRING {
                    crate::jserror::js_syntaxerror(J, cstr!("JSON: unexpected token: %s (expected string)"), crate::jslex::jsY_tokenstring((*J).lookahead));
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
        x if x == '[' as c_int => {
            crate::jsvalue::js_newarray(J);
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
        x if x == TK_TRUE => {
            js_pushboolean(J, 1);
            jsonnext(J);
        }
        x if x == TK_FALSE => {
            js_pushboolean(J, 0);
            jsonnext(J);
        }
        x if x == TK_NULL => {
            js_pushnull(J);
            jsonnext(J);
        }
        _ => {
            crate::jserror::js_syntaxerror(J, cstr!("JSON: unexpected token: %s"), crate::jslex::jsY_tokenstring((*J).lookahead));
        }
    }
}

unsafe fn jsonrevive(J: *mut js_State, name: *const c_char) {
    let mut key;
    let mut buf: [c_char; 32] = [0; 32];

    js_getproperty(J, -1, name);

    if js_isobject(J, -1) != 0 {
        if js_isarray(J, -1) != 0 {
            let mut i = 0;
            let n = crate::jsarray::js_getlength(J, -1);
            while i < n {
                jsonrevive(J, crate::jsvalue::js_itoa(buf.as_mut_ptr(), i));
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

    js_copy(J, 2);
    js_copy(J, -3);
    js_pushstring(J, name);
    js_copy(J, -4);
    js_call(J, 2);
    js_rot2pop1(J);
}

unsafe extern "C-unwind" fn JSON_parse(J: *mut js_State) {
    let source = js_tostring(J, 1);
    crate::jslex::jsY_initlex(J, cstr!("JSON"), source);
    jsonnext(J);

    if js_iscallable(J, 2) != 0 {
        crate::jsvalue::js_newobject(J);
        jsonvalue(J);
        js_defproperty(J, -2, cstr!(""), 0);
        jsonrevive(J, cstr!(""));
    } else {
        jsonvalue(J);
    }
}

unsafe fn fmtnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    if n.is_nan() {
        js_puts(J, sb, cstr!("null"));
    } else if n.is_infinite() {
        js_puts(J, sb, cstr!("null"));
    } else if n == 0.0 {
        js_puts(J, sb, cstr!("0"));
    } else {
        let mut buf: [c_char; 40] = [0; 40];
        js_puts(J, sb, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn fmtstr(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    static HEX: &[u8; 17] = b"0123456789abcdef\0";
    let mut i;
    let mut n;
    let mut c: Rune = 0;
    let mut s = s;
    js_putc(J, sb, '"' as c_int);
    while *s != 0 {
        n = chartorune(&mut c, s);
        match c {
            x if x == '"' as Rune => js_puts(J, sb, cstr!("\\\"")),
            x if x == '\\' as Rune => js_puts(J, sb, cstr!("\\\\")),
            x if x == '\u{08}' as Rune => js_puts(J, sb, cstr!("\\b")),
            x if x == '\u{0c}' as Rune => js_puts(J, sb, cstr!("\\f")),
            x if x == '\n' as Rune => js_puts(J, sb, cstr!("\\n")),
            x if x == '\r' as Rune => js_puts(J, sb, cstr!("\\r")),
            x if x == '\t' as Rune => js_puts(J, sb, cstr!("\\t")),
            _ => {
                if c < ' ' as Rune || (c >= 0xd800 && c <= 0xdfff) {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'u' as c_int);
                    js_putc(J, sb, HEX[((c >> 12) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[((c >> 8) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[((c >> 4) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[(c & 15) as usize] as c_int);
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

unsafe fn fmtindent(J: *mut js_State, sb: *mut *mut js_Buffer, gap: *const c_char, mut level: c_int) {
    js_putc(J, sb, '\n' as c_int);
    while level > 0 {
        level -= 1;
        js_puts(J, sb, gap);
    }
}

unsafe fn filterprop(J: *mut js_State, key: *const c_char) -> c_int {
    let mut i;
    let n;
    let mut found;
    if js_isarray(J, 2) != 0 {
        found = 0;
        n = crate::jsarray::js_getlength(J, 2);
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

unsafe fn fmtobject(J: *mut js_State, sb: *mut *mut js_Buffer, _obj: *mut js_Object, gap: *const c_char, level: c_int) {
    let mut key;
    let mut save;
    let mut i;
    let mut n;

    n = js_gettop(J) - 1;
    i = 4;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                crate::jserror::js_typeerror(J, cstr!("cyclic object value"));
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
    let mut n;
    let mut i;
    let mut buf: [c_char; 32] = [0; 32];

    n = js_gettop(J) - 1;
    i = 4;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                crate::jserror::js_typeerror(J, cstr!("cyclic object value"));
            }
        }
        i += 1;
    }

    js_putc(J, sb, '[' as c_int);
    n = crate::jsarray::js_getlength(J, -1);
    i = 0;
    while i < n {
        if i != 0 {
            js_putc(J, sb, ',' as c_int);
        }
        if !gap.is_null() {
            fmtindent(J, sb, gap, level + 1);
        }
        if fmtvalue(J, sb, crate::jsvalue::js_itoa(buf.as_mut_ptr(), i), gap, level + 1) == 0 {
            js_puts(J, sb, cstr!("null"));
        }
        i += 1;
    }
    if !gap.is_null() && n != 0 {
        fmtindent(J, sb, gap, level);
    }
    js_putc(J, sb, ']' as c_int);
}

unsafe fn fmtvalue(J: *mut js_State, sb: *mut *mut js_Buffer, key: *const c_char, gap: *const c_char, level: c_int) -> c_int {
    js_getproperty(J, -1, key);

    if js_isobject(J, -1) != 0 {
        if js_hasproperty(J, -1, cstr!("toJSON")) != 0 {
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
        js_copy(J, 2);
        js_copy(J, -3);
        js_pushstring(J, key);
        js_copy(J, -4);
        js_call(J, 2);
        js_rot2pop1(J);
    }

    if js_isobject(J, -1) != 0 && js_iscallable(J, -1) == 0 {
        let obj = js_toobject(J, -1);
        match (*obj).type_ {
            x if x == JS_CNUMBER => fmtnum(J, sb, (*obj).u.number),
            x if x == JS_CSTRING => fmtstr(J, sb, (*obj).u.s.string),
            x if x == JS_CBOOLEAN => js_puts(J, sb, if (*obj).u.boolean != 0 { cstr!("true") } else { cstr!("false") }),
            x if x == JS_CARRAY => fmtarray(J, sb, gap, level),
            _ => fmtobject(J, sb, obj, gap, level),
        }
    } else if js_isboolean(J, -1) != 0 {
        js_puts(J, sb, if js_toboolean(J, -1) != 0 { cstr!("true") } else { cstr!("false") });
    } else if js_isnumber(J, -1) != 0 {
        fmtnum(J, sb, js_tonumber(J, -1));
    } else if js_isstring(J, -1) != 0 {
        fmtstr(J, sb, js_tostring(J, -1));
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, cstr!("null"));
    } else {
        js_pop(J, 1);
        return 0;
    }

    js_pop(J, 1);
    1
}

unsafe extern "C-unwind" fn JSON_stringify(J: *mut js_State) {
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    let mut buf: [c_char; 12] = [0; 12];
    let mut gap: *const c_char;
    let s;
    let mut n;

    gap = std::ptr::null();

    if js_isnumber(J, 3) != 0 || js_isnumberobject(J, 3) != 0 {
        n = js_tointeger(J, 3);
        if n < 0 {
            n = 0;
        }
        if n > 10 {
            n = 10;
        }
        libc::memset(buf.as_mut_ptr() as *mut c_void, ' ' as c_int, n as usize);
        buf[n as usize] = 0;
        if n > 0 {
            gap = buf.as_ptr();
        }
    } else if js_isstring(J, 3) != 0 || js_isstringobject(J, 3) != 0 {
        s = js_tostring(J, 3);
        n = strlen(s) as c_int;
        if n > 10 {
            n = 10;
        }
        memcpy(buf.as_mut_ptr(), s, n as usize);
        buf[n as usize] = 0;
        if n > 0 {
            gap = buf.as_ptr();
        }
    }

    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let gap_v = gap;
    let caught = protect(J, || {
        crate::jsvalue::js_newobject(J);
        js_copy(J, 1);
        js_defproperty(J, -2, cstr!(""), 0);
        if fmtvalue(J, sb_ptr, cstr!(""), gap_v, 0) == 0 {
            js_pushundefined(J);
        } else {
            js_putc(J, sb_ptr, 0);
            js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("") });
            js_rot2pop1(J);
        }
    });
    if caught {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initjson(J: *mut js_State) {
    js_pushobject(J, crate::jsproperty::jsV_newobject(J, JS_CJSON, (*J).Object_prototype));
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("JSON.parse"), Some(JSON_parse), 2);
        crate::jsbuiltin::jsB_propf(J, cstr!("JSON.stringify"), Some(JSON_stringify), 3);
    }
    js_defglobal(J, cstr!("JSON"), JS_DONTENUM);
}
