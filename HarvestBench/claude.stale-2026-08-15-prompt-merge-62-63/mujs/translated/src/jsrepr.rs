//! Translated from jsrepr.c — js_repr / js_torepr / js_tryrepr.
#![allow(non_snake_case, non_upper_case_globals)]


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

unsafe fn reprnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    let mut buf: [c_char; 40] = [0; 40];
    if n == 0.0 && n.is_sign_negative() {
        js_puts(J, sb, cstr!("-0"));
    } else {
        js_puts(J, sb, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn reprstr(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    static HEX: &[u8; 17] = b"0123456789ABCDEF\0";
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
                if c < ' ' as Rune {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'x' as c_int);
                    js_putc(J, sb, HEX[((c >> 4) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[(c & 15) as usize] as c_int);
                } else if c < 128 {
                    js_putc(J, sb, c);
                } else if c < 0x10000 {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'u' as c_int);
                    js_putc(J, sb, HEX[((c >> 12) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[((c >> 8) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[((c >> 4) & 15) as usize] as c_int);
                    js_putc(J, sb, HEX[(c & 15) as usize] as c_int);
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

#[inline]
unsafe fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}
#[inline]
unsafe fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}

unsafe fn reprident(J: *mut js_State, sb: *mut *mut js_Buffer, name: *const c_char) {
    let mut p = name;
    if isdigit(*p as c_int) {
        while isdigit(*p as c_int) {
            p = p.add(1);
        }
    } else if isalpha(*p as c_int) || *p == '_' as c_char {
        while isdigit(*p as c_int) || isalpha(*p as c_int) || *p == '_' as c_char {
            p = p.add(1);
        }
    }
    if p > name && *p == 0 {
        js_puts(J, sb, name);
    } else {
        reprstr(J, sb, name);
    }
}

unsafe fn reprobject(J: *mut js_State, sb: *mut *mut js_Buffer) {
    let mut key;
    let mut i;
    let mut n;

    n = js_gettop(J) - 1;
    i = 0;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_puts(J, sb, cstr!("{}"));
                return;
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
        if n > 0 {
            js_puts(J, sb, cstr!(", "));
        }
        n += 1;
        reprident(J, sb, key);
        js_puts(J, sb, cstr!(": "));
        js_getproperty(J, -2, key);
        reprvalue(J, sb);
        js_pop(J, 1);
    }
    js_pop(J, 1);
    js_putc(J, sb, '}' as c_int);
}

unsafe fn reprarray(J: *mut js_State, sb: *mut *mut js_Buffer) {
    let mut n;
    let mut i;

    n = js_gettop(J) - 1;
    i = 0;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_puts(J, sb, cstr!("[]"));
                return;
            }
        }
        i += 1;
    }

    js_putc(J, sb, '[' as c_int);
    n = crate::jsarray::js_getlength(J, -1);
    i = 0;
    while i < n {
        if i > 0 {
            js_puts(J, sb, cstr!(", "));
        }
        if js_hasindex(J, -1, i) != 0 {
            reprvalue(J, sb);
            js_pop(J, 1);
        }
        i += 1;
    }
    js_putc(J, sb, ']' as c_int);
}

unsafe fn reprfun(J: *mut js_State, sb: *mut *mut js_Buffer, fun: *mut js_Function) {
    let mut i;
    js_puts(J, sb, cstr!("function "));
    js_puts(J, sb, (*fun).name);
    js_putc(J, sb, '(' as c_int);
    i = 0;
    while i < (*fun).numparams {
        if i > 0 {
            js_puts(J, sb, cstr!(", "));
        }
        js_puts(J, sb, *(*fun).vartab.add(i as usize));
        i += 1;
    }
    js_puts(J, sb, cstr!(") { [byte code] }"));
}

unsafe fn reprvalue(J: *mut js_State, sb: *mut *mut js_Buffer) {
    if js_isundefined(J, -1) != 0 {
        js_puts(J, sb, cstr!("undefined"));
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, cstr!("null"));
    } else if js_isboolean(J, -1) != 0 {
        js_puts(J, sb, if js_toboolean(J, -1) != 0 { cstr!("true") } else { cstr!("false") });
    } else if js_isnumber(J, -1) != 0 {
        reprnum(J, sb, js_tonumber(J, -1));
    } else if js_isstring(J, -1) != 0 {
        reprstr(J, sb, js_tostring(J, -1));
    } else if js_isobject(J, -1) != 0 {
        let obj = js_toobject(J, -1);
        match (*obj).type_ {
            x if x == JS_CARRAY => {
                reprarray(J, sb);
            }
            x if x == JS_CFUNCTION || x == JS_CSCRIPT => {
                reprfun(J, sb, (*obj).u.f.function);
            }
            x if x == JS_CCFUNCTION => {
                js_puts(J, sb, cstr!("function "));
                js_puts(J, sb, (*obj).u.c.name);
                js_puts(J, sb, cstr!("() { [native code] }"));
            }
            x if x == JS_CBOOLEAN => {
                js_puts(J, sb, cstr!("(new Boolean("));
                js_puts(J, sb, if (*obj).u.boolean != 0 { cstr!("true") } else { cstr!("false") });
                js_puts(J, sb, cstr!("))"));
            }
            x if x == JS_CNUMBER => {
                js_puts(J, sb, cstr!("(new Number("));
                reprnum(J, sb, (*obj).u.number);
                js_puts(J, sb, cstr!("))"));
            }
            x if x == JS_CSTRING => {
                js_puts(J, sb, cstr!("(new String("));
                reprstr(J, sb, (*obj).u.s.string);
                js_puts(J, sb, cstr!("))"));
            }
            x if x == JS_CREGEXP => {
                js_putc(J, sb, '/' as c_int);
                js_puts(J, sb, (*obj).u.r.source);
                js_putc(J, sb, '/' as c_int);
                if (*obj).u.r.flags as c_int & JS_REGEXP_G != 0 {
                    js_putc(J, sb, 'g' as c_int);
                }
                if (*obj).u.r.flags as c_int & JS_REGEXP_I != 0 {
                    js_putc(J, sb, 'i' as c_int);
                }
                if (*obj).u.r.flags as c_int & JS_REGEXP_M != 0 {
                    js_putc(J, sb, 'm' as c_int);
                }
            }
            x if x == JS_CDATE => {
                let mut buf: [c_char; 40] = [0; 40];
                js_puts(J, sb, cstr!("(new Date("));
                js_puts(J, sb, crate::jsvalue::jsV_numbertostring(J, buf.as_mut_ptr(), (*obj).u.number));
                js_puts(J, sb, cstr!("))"));
            }
            x if x == JS_CERROR => {
                js_puts(J, sb, cstr!("(new "));
                js_getproperty(J, -1, cstr!("name"));
                js_puts(J, sb, js_tostring(J, -1));
                js_pop(J, 1);
                js_putc(J, sb, '(' as c_int);
                if js_hasproperty(J, -1, cstr!("message")) != 0 {
                    reprvalue(J, sb);
                    js_pop(J, 1);
                }
                js_puts(J, sb, cstr!("))"));
            }
            x if x == JS_CMATH => {
                js_puts(J, sb, cstr!("Math"));
            }
            x if x == JS_CJSON => {
                js_puts(J, sb, cstr!("JSON"));
            }
            x if x == JS_CITERATOR => {
                js_puts(J, sb, cstr!("[iterator "));
            }
            x if x == JS_CUSERDATA => {
                js_puts(J, sb, cstr!("[userdata "));
                js_puts(J, sb, (*obj).u.user.tag);
                js_putc(J, sb, ']' as c_int);
            }
            _ => {
                reprobject(J, sb);
            }
        }
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_repr(J: *mut js_State, idx: c_int) {
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    let mut savebot = 0;

    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let caught = protect(J, || {
        js_copy(J, idx);

        savebot = (*J).bot;
        (*J).bot = (*J).top - 1;
        reprvalue(J, sb_ptr);
        (*J).bot = savebot;

        js_pop(J, 1);

        js_putc(J, sb_ptr, 0);
        js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("undefined") });
    });
    if caught {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_torepr(J: *mut js_State, idx: c_int) -> *const c_char {
    js_repr(J, idx);
    js_replace(J, if idx < 0 { idx - 1 } else { idx });
    js_tostring(J, idx)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_tryrepr(J: *mut js_State, idx: c_int, error: *const c_char) -> *const c_char {
    let mut s: *const c_char = std::ptr::null();
    let sp = std::ptr::addr_of_mut!(s);
    let caught = protect(J, || {
        *sp = js_torepr(J, idx);
    });
    if caught {
        js_pop(J, 1);
        return error;
    }
    js_endtry(J);
    s
}
