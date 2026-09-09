//! Translation of jsrepr.c

use crate::*;

unsafe fn reprnum(J: *mut js_State, sb: *mut *mut js_Buffer, n: f64) {
    let mut buf: [c_char; 40] = [0; 40];
    if n == 0.0 && signbit(n) {
        js_puts(J, sb, cs!("-0"));
    } else {
        js_puts(J, sb, jsV_numbertostring(J, buf.as_mut_ptr(), n));
    }
}

unsafe fn reprstr(J: *mut js_State, sb: *mut *mut js_Buffer, s: *const c_char) {
    let HEX: *const c_char = cs!("0123456789ABCDEF");
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
            if c < ' ' as c_int {
                js_putc(J, sb, '\\' as c_int);
                js_putc(J, sb, 'x' as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 4) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset((c & 15) as isize) as c_int);
            } else if c < 128 {
                js_putc(J, sb, c);
            } else if c < 0x10000 {
                js_putc(J, sb, '\\' as c_int);
                js_putc(J, sb, 'u' as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 12) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 8) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset(((c >> 4) & 15) as isize) as c_int);
                js_putc(J, sb, *HEX.offset((c & 15) as isize) as c_int);
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

#[inline]
unsafe fn repr_isalpha(c: c_char) -> bool {
    (c >= 'a' as c_char && c <= 'z' as c_char) || (c >= 'A' as c_char && c <= 'Z' as c_char)
}

#[inline]
unsafe fn repr_isdigit(c: c_char) -> bool {
    c >= '0' as c_char && c <= '9' as c_char
}

unsafe fn reprident(J: *mut js_State, sb: *mut *mut js_Buffer, name: *const c_char) {
    let mut p: *const c_char = name;
    if repr_isdigit(*p) {
        while repr_isdigit(*p) {
            p = p.add(1);
        }
    } else if repr_isalpha(*p) || *p == '_' as c_char {
        while repr_isdigit(*p) || repr_isalpha(*p) || *p == '_' as c_char {
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
    let mut key: *const c_char;
    let mut i: c_int;
    let mut n: c_int;

    n = js_gettop(J) - 1;
    i = 0;
    while i < n {
        if js_isobject(J, i) != 0 {
            if js_toobject(J, i) == js_toobject(J, -1) {
                js_puts(J, sb, cs!("{}"));
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
        let t = n;
        n += 1;
        if t > 0 {
            js_puts(J, sb, cs!(", "));
        }
        reprident(J, sb, key);
        js_puts(J, sb, cs!(": "));
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
                js_puts(J, sb, cs!("[]"));
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
            js_puts(J, sb, cs!(", "));
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
    js_puts(J, sb, cs!("function "));
    js_puts(J, sb, (*fun).name);
    js_putc(J, sb, '(' as c_int);
    i = 0;
    while i < (*fun).numparams {
        if i > 0 {
            js_puts(J, sb, cs!(", "));
        }
        js_puts(J, sb, *(*fun).vartab.offset(i as isize));
        i += 1;
    }
    js_puts(J, sb, cs!(") { [byte code] }"));
}

unsafe fn reprvalue(J: *mut js_State, sb: *mut *mut js_Buffer) {
    if js_isundefined(J, -1) != 0 {
        js_puts(J, sb, cs!("undefined"));
    } else if js_isnull(J, -1) != 0 {
        js_puts(J, sb, cs!("null"));
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
                js_puts(J, sb, cs!("function "));
                js_puts(J, sb, (*obj).u.c.name);
                js_puts(J, sb, cs!("() { [native code] }"));
            }
            JS_CBOOLEAN => {
                js_puts(J, sb, cs!("(new Boolean("));
                js_puts(
                    J,
                    sb,
                    if (*obj).u.boolean != 0 {
                        cs!("true")
                    } else {
                        cs!("false")
                    },
                );
                js_puts(J, sb, cs!("))"));
            }
            JS_CNUMBER => {
                js_puts(J, sb, cs!("(new Number("));
                reprnum(J, sb, (*obj).u.number);
                js_puts(J, sb, cs!("))"));
            }
            JS_CSTRING => {
                js_puts(J, sb, cs!("(new String("));
                reprstr(J, sb, (*obj).u.s.string);
                js_puts(J, sb, cs!("))"));
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
                let mut buf: [c_char; 40] = [0; 40];
                js_puts(J, sb, cs!("(new Date("));
                js_puts(
                    J,
                    sb,
                    jsV_numbertostring(J, buf.as_mut_ptr(), (*obj).u.number),
                );
                js_puts(J, sb, cs!("))"));
            }
            JS_CERROR => {
                js_puts(J, sb, cs!("(new "));
                js_getproperty(J, -1, cs!("name"));
                js_puts(J, sb, js_tostring(J, -1));
                js_pop(J, 1);
                js_putc(J, sb, '(' as c_int);
                if js_hasproperty(J, -1, cs!("message")) != 0 {
                    reprvalue(J, sb);
                    js_pop(J, 1);
                }
                js_puts(J, sb, cs!("))"));
            }
            JS_CMATH => {
                js_puts(J, sb, cs!("Math"));
            }
            JS_CJSON => {
                js_puts(J, sb, cs!("JSON"));
            }
            JS_CITERATOR => {
                js_puts(J, sb, cs!("[iterator "));
            }
            JS_CUSERDATA => {
                js_puts(J, sb, cs!("[userdata "));
                js_puts(J, sb, (*obj).u.user.tag);
                js_putc(J, sb, ']' as c_int);
            }
            _ => {
                reprobject(J, sb);
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_repr(J: *mut js_State, idx: c_int) {
    let mut sb: *mut js_Buffer = null_mut();
    let savebot: c_int;

    if js_try!(J) != 0 {
        js_free(J, vol!(sb) as *mut c_void);
        js_throw(J);
    }

    js_copy(J, idx);

    savebot = (*J).bot;
    (*J).bot = (*J).top - 1;
    reprvalue(J, addr_of_mut!(sb));
    (*J).bot = savebot;

    js_pop(J, 1);

    js_putc(J, addr_of_mut!(sb), 0);
    js_pushstring(
        J,
        if !sb.is_null() {
            addr_of!((*sb).s) as *const c_char
        } else {
            cs!("undefined")
        },
    );

    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_torepr(J: *mut js_State, idx: c_int) -> *const c_char {
    js_repr(J, idx);
    js_replace(J, if idx < 0 { idx - 1 } else { idx });
    js_tostring(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_tryrepr(
    J: *mut js_State,
    idx: c_int,
    error: *const c_char,
) -> *const c_char {
    let s: *const c_char;
    if js_try!(J) != 0 {
        js_pop(J, 1);
        return error;
    }
    s = js_torepr(J, idx);
    js_endtry(J);
    s
}
