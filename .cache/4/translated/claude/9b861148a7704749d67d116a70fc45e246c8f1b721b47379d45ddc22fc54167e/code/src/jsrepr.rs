//! Translation of `c_src/src/jsrepr.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsarray::{js_getlength, js_setlength};
use crate::jsi::*;
use crate::jsintern::{js_putc, js_putm, js_puts};
use crate::jsproperty::*;
use crate::jsrun::*;
use crate::jsvalue::*;
use crate::utf::chartorune;
use core::ptr::{null, null_mut};

/* static void reprvalue(js_State *J, js_Buffer **sb); */

unsafe fn reprnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    let mut buf = [0 as c_char; 40];
    if n == 0.0 && signbit(n) {
        js_puts(J, sb, c"-0".as_ptr());
    } else {
        js_puts(J, sb, jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn reprstr(J: *mut js_State, sb: *mut *mut js_Buffer, mut s: *const c_char) {
    /* static const char *HEX = "0123456789ABCDEF"; */
    let HEX: *const c_char = c"0123456789ABCDEF".as_ptr();
    let mut i: c_int;
    let mut n: c_int;
    let mut c: Rune = 0;
    js_putc(J, sb, '"' as c_int);
    while *s != 0 {
        n = chartorune(&mut c, s);
        match c {
            0x22 /* '"' */ => js_puts(J, sb, c"\\\"".as_ptr()),
            0x5c /* '\\' */ => js_puts(J, sb, c"\\\\".as_ptr()),
            0x08 /* '\b' */ => js_puts(J, sb, c"\\b".as_ptr()),
            0x0c /* '\f' */ => js_puts(J, sb, c"\\f".as_ptr()),
            0x0a /* '\n' */ => js_puts(J, sb, c"\\n".as_ptr()),
            0x0d /* '\r' */ => js_puts(J, sb, c"\\r".as_ptr()),
            0x09 /* '\t' */ => js_puts(J, sb, c"\\t".as_ptr()),
            _ => {
                if c < ' ' as Rune {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'x' as c_int);
                    js_putc(J, sb, *HEX.offset((((c >> 4) & 15) as isize)) as c_int);
                    js_putc(J, sb, *HEX.offset(((c & 15) as isize)) as c_int);
                } else if c < 128 {
                    js_putc(J, sb, c);
                } else if c < 0x10000 {
                    js_putc(J, sb, '\\' as c_int);
                    js_putc(J, sb, 'u' as c_int);
                    js_putc(J, sb, *HEX.offset((((c >> 12) & 15) as isize)) as c_int);
                    js_putc(J, sb, *HEX.offset((((c >> 8) & 15) as isize)) as c_int);
                    js_putc(J, sb, *HEX.offset((((c >> 4) & 15) as isize)) as c_int);
                    js_putc(J, sb, *HEX.offset(((c & 15) as isize)) as c_int);
                } else {
                    i = 0;
                    while i < n {
                        js_putc(J, sb, *s.offset(i as isize) as c_int);
                        i += 1;
                    }
                }
            }
        }
        s = s.offset(n as isize);
    }
    js_putc(J, sb, '"' as c_int);
}

/* jsrepr.c defines its own ASCII-only isalpha/isdigit (jsi.h does not
 * include <ctype.h>, so the fallback macros are the ones that get used). */
#[inline]
unsafe fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}

#[inline]
unsafe fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}

unsafe fn reprident(J: *mut js_State, sb: *mut *mut js_Buffer, name: *const c_char) {
    let mut p: *const c_char = name;
    if isdigit(*p as c_int) {
        while isdigit(*p as c_int) {
            p = p.offset(1);
        }
    } else if isalpha(*p as c_int) || *p == '_' as c_char {
        while isdigit(*p as c_int) || isalpha(*p as c_int) || *p == '_' as c_char {
            p = p.offset(1);
        }
    }
    if p > name && *p == 0 {
        js_puts(J, sb, name);
    } else {
        reprstr(J, sb, name);
    }
}

unsafe fn reprobject(J: *mut js_State, sb: *mut *mut js_Buffer) {
    let mut key: *const c_char;
    let mut i: c_int;
    let mut n: c_int;

    n = js_gettop(J) - 1;
    i = 0;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_puts(J, sb, c"{}".as_ptr());
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
        let old = n;
        n += 1;
        if old > 0 {
            js_puts(J, sb, c", ".as_ptr());
        }
        reprident(J, sb, key);
        js_puts(J, sb, c": ".as_ptr());
        js_getproperty(J, -2, key);
        reprvalue(J, sb);
        js_pop(J, 1);
    }
    js_pop(J, 1);
    js_putc(J, sb, '}' as c_int);
}

unsafe fn reprarray(J: *mut js_State, sb: *mut *mut js_Buffer) {
    let mut n: c_int;
    let mut i: c_int;

    n = js_gettop(J) - 1;
    i = 0;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_puts(J, sb, c"[]".as_ptr());
                return;
            }
        }
        i += 1;
    }

    js_putc(J, sb, '[' as c_int);
    n = js_getlength(J, -1);
    i = 0;
    while i < n {
        if i > 0 {
            js_puts(J, sb, c", ".as_ptr());
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
    let mut i: c_int;
    js_puts(J, sb, c"function ".as_ptr());
    js_puts(J, sb, (*fun).name);
    js_putc(J, sb, '(' as c_int);
    i = 0;
    while i < (*fun).numparams {
        if i > 0 {
            js_puts(J, sb, c", ".as_ptr());
        }
        js_puts(J, sb, *(*fun).vartab.offset(i as isize));
        i += 1;
    }
    js_puts(J, sb, c") { [byte code] }".as_ptr());
}

unsafe fn reprvalue(J: *mut js_State, sb: *mut *mut js_Buffer) {
    if js_isundefined(J, -1) != 0 {
        js_puts(J, sb, c"undefined".as_ptr());
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, c"null".as_ptr());
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
        reprnum(J, sb, js_tonumber(J, -1));
    } else if js_isstring(J, -1) != 0 {
        reprstr(J, sb, js_tostring(J, -1));
    } else if js_isobject(J, -1) != 0 {
        let obj: *mut js_Object = js_toobject(J, -1);
        match (*obj).type_ {
            JS_CARRAY => {
                reprarray(J, sb);
            }
            JS_CFUNCTION | JS_CSCRIPT => {
                reprfun(J, sb, (*obj).u.f.function);
            }
            JS_CCFUNCTION => {
                js_puts(J, sb, c"function ".as_ptr());
                js_puts(J, sb, (*obj).u.c.name);
                js_puts(J, sb, c"() { [native code] }".as_ptr());
            }
            JS_CBOOLEAN => {
                js_puts(J, sb, c"(new Boolean(".as_ptr());
                js_puts(
                    J,
                    sb,
                    if (*obj).u.boolean != 0 {
                        c"true".as_ptr()
                    } else {
                        c"false".as_ptr()
                    },
                );
                js_puts(J, sb, c"))".as_ptr());
            }
            JS_CNUMBER => {
                js_puts(J, sb, c"(new Number(".as_ptr());
                reprnum(J, sb, (*obj).u.number);
                js_puts(J, sb, c"))".as_ptr());
            }
            JS_CSTRING => {
                js_puts(J, sb, c"(new String(".as_ptr());
                reprstr(J, sb, (*obj).u.s.string);
                js_puts(J, sb, c"))".as_ptr());
            }
            JS_CREGEXP => {
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
            JS_CDATE => {
                let mut buf = [0 as c_char; 40];
                js_puts(J, sb, c"(new Date(".as_ptr());
                js_puts(
                    J,
                    sb,
                    jsV_numbertostring(J, buf.as_mut_ptr(), (*obj).u.number),
                );
                js_puts(J, sb, c"))".as_ptr());
            }
            JS_CERROR => {
                js_puts(J, sb, c"(new ".as_ptr());
                js_getproperty(J, -1, c"name".as_ptr());
                js_puts(J, sb, js_tostring(J, -1));
                js_pop(J, 1);
                js_putc(J, sb, '(' as c_int);
                if js_hasproperty(J, -1, c"message".as_ptr()) != 0 {
                    reprvalue(J, sb);
                    js_pop(J, 1);
                }
                js_puts(J, sb, c"))".as_ptr());
            }
            JS_CMATH => {
                js_puts(J, sb, c"Math".as_ptr());
            }
            JS_CJSON => {
                js_puts(J, sb, c"JSON".as_ptr());
            }
            JS_CITERATOR => {
                js_puts(J, sb, c"[iterator ".as_ptr());
            }
            JS_CUSERDATA => {
                js_puts(J, sb, c"[userdata ".as_ptr());
                js_puts(J, sb, (*obj).u.user.tag);
                js_putc(J, sb, ']' as c_int);
            }
            /* default: */
            _ => {
                reprobject(J, sb);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_repr(J: *mut js_State, idx: c_int) {
    let mut sb: *mut js_Buffer = null_mut();
    let sbp = &mut sb as *mut *mut js_Buffer;

    if js_do_try(J, || {
        js_copy(J, idx);

        let savebot = (*J).bot;
        (*J).bot = (*J).top - 1;
        reprvalue(J, sbp);
        (*J).bot = savebot;

        js_pop(J, 1);

        js_putc(J, sbp, 0);
        js_pushstring(
            J,
            if !(*sbp).is_null() {
                (*(*sbp)).s.as_ptr()
            } else {
                c"undefined".as_ptr()
            },
        );

        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }
    js_free(J, sb as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_torepr(J: *mut js_State, idx: c_int) -> *const c_char {
    js_repr(J, idx);
    js_replace(J, if idx < 0 { idx - 1 } else { idx });
    js_tostring(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_tryrepr(
    J: *mut js_State,
    idx: c_int,
    error: *const c_char,
) -> *const c_char {
    match js_do_try(J, || {
        let s = js_torepr(J, idx);
        js_endtry(J);
        s
    }) {
        None => {
            js_pop(J, 1);
            error
        }
        Some(s) => s,
    }
}
