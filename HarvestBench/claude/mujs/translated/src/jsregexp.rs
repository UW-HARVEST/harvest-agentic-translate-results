//! Translated from jsregexp.c — RegExp constructor and prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::*;
use crate::regexp::{Reprog, Resub, REG_ICASE, REG_NEWLINE, REG_NOTBOL};
use crate::types::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

unsafe fn escaperegexp(J: *mut js_State, pattern: *const c_char) -> *mut c_char {
    let copy;
    let mut p;
    let mut s;
    let mut n = 0;
    s = pattern;
    while *s != 0 {
        if *s == '/' as c_char {
            n += 1;
        }
        n += 1;
        s = s.add(1);
    }
    copy = js_malloc(J, n + 1) as *mut c_char;
    p = copy;
    s = pattern;
    while *s != 0 {
        if *s == '/' as c_char {
            *p = '\\' as c_char;
            p = p.add(1);
        }
        *p = *s;
        p = p.add(1);
        s = s.add(1);
    }
    *p = 0;
    copy
}

unsafe fn js_newregexpx(J: *mut js_State, pattern: *const c_char, flags: c_int, is_clone: c_int) {
    let mut error: *const c_char = std::ptr::null();
    let obj;
    let prog;
    let mut opts;

    obj = crate::jsproperty::jsV_newobject(J, JS_CREGEXP, (*J).RegExp_prototype);

    opts = 0;
    if flags & JS_REGEXP_I != 0 {
        opts |= REG_ICASE;
    }
    if flags & JS_REGEXP_M != 0 {
        opts |= REG_NEWLINE;
    }

    prog = crate::regexp::js_regcompx((*J).alloc, (*J).actx, pattern, opts, &mut error);
    if prog.is_null() {
        crate::jserror::js_syntaxerror(J, cstr!("regular expression: %s"), error);
    }

    (*obj).u.r.prog = prog as *mut c_void;
    (*obj).u.r.source = if is_clone != 0 { js_strdup(J, pattern) } else { escaperegexp(J, pattern) };
    (*obj).u.r.flags = flags as u16;
    (*obj).u.r.last = 0;
    js_pushobject(J, obj);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_newregexp(J: *mut js_State, pattern: *const c_char, flags: c_int) {
    js_newregexpx(J, pattern, flags, 0);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_RegExp_prototype_exec(J: *mut js_State, re: *mut js_Regexp, text: *const c_char) {
    let mut haystack;
    let result;
    let mut i;
    let mut opts;
    let mut m: Resub = std::mem::zeroed();

    haystack = text;
    opts = 0;
    if (*re).flags as c_int & JS_REGEXP_G != 0 {
        if (*re).last as usize > strlen(haystack) {
            (*re).last = 0;
            js_pushnull(J);
            return;
        }
        if (*re).last > 0 {
            haystack = text.add((*re).last as usize);
            if (*re).flags as c_int & JS_REGEXP_M == 0 || *haystack.offset(-1) != '\n' as c_char {
                opts |= REG_NOTBOL;
            }
        }
    }

    result = crate::regexp::js_regexec((*re).prog as *mut Reprog, haystack, &mut m, opts);
    if result < 0 {
        crate::jserror::js_error(J, cstr!("regexec failed"));
    }
    if result == 0 {
        crate::jsvalue::js_newarray(J);
        js_pushstring(J, text);
        js_setproperty(J, -2, cstr!("input"));
        js_pushnumber(J, crate::jsstring::js_utfptrtoidx(text, m.sub[0].sp) as f64);
        js_setproperty(J, -2, cstr!("index"));
        i = 0;
        while i < m.nsub {
            js_pushlstring(J, m.sub[i as usize].sp, (m.sub[i as usize].ep as isize - m.sub[i as usize].sp as isize) as c_int);
            js_setindex(J, -2, i);
            i += 1;
        }
        if (*re).flags as c_int & JS_REGEXP_G != 0 {
            (*re).last = (m.sub[0].ep as isize - text as isize) as u16;
        }
        return;
    }

    if (*re).flags as c_int & JS_REGEXP_G != 0 {
        (*re).last = 0;
    }

    js_pushnull(J);
}

unsafe extern "C-unwind" fn Rp_test(J: *mut js_State) {
    let re: *mut js_Regexp;
    let mut text;
    let result;
    let mut opts;
    let mut m: Resub = std::mem::zeroed();

    re = js_toregexp(J, 0);
    text = js_tostring(J, 1);

    opts = 0;
    if (*re).flags as c_int & JS_REGEXP_G != 0 {
        if (*re).last as usize > strlen(text) {
            (*re).last = 0;
            js_pushboolean(J, 0);
            return;
        }
        if (*re).last > 0 {
            text = text.add((*re).last as usize);
            if (*re).flags as c_int & JS_REGEXP_M == 0 || *text.offset(-1) != '\n' as c_char {
                opts |= REG_NOTBOL;
            }
        }
    }

    result = crate::regexp::js_regexec((*re).prog as *mut Reprog, text, &mut m, opts);
    if result < 0 {
        crate::jserror::js_error(J, cstr!("regexec failed"));
    }
    if result == 0 {
        if (*re).flags as c_int & JS_REGEXP_G != 0 {
            (*re).last = ((*re).last as isize + (m.sub[0].ep as isize - text as isize)) as u16;
        }
        js_pushboolean(J, 1);
        return;
    }

    if (*re).flags as c_int & JS_REGEXP_G != 0 {
        (*re).last = 0;
    }

    js_pushboolean(J, 0);
}

unsafe extern "C-unwind" fn jsB_new_RegExp(J: *mut js_State) {
    let old: *mut js_Regexp;
    let mut pattern;
    let mut flags;
    let mut is_clone = 0;

    if js_isregexp(J, 1) != 0 {
        if js_isdefined(J, 2) != 0 {
            crate::jserror::js_typeerror(J, cstr!("cannot supply flags when creating one RegExp from another"));
        }
        old = js_toregexp(J, 1);
        pattern = (*old).source as *const c_char;
        flags = (*old).flags as c_int;
        is_clone = 1;
    } else if js_isundefined(J, 1) != 0 {
        pattern = cstr!("(?:)");
        flags = 0;
    } else {
        pattern = js_tostring(J, 1);
        flags = 0;
    }

    if strlen(pattern) == 0 {
        pattern = cstr!("(?:)");
    }

    if js_isdefined(J, 2) != 0 {
        let mut s = js_tostring(J, 2);
        let mut g = 0;
        let mut i = 0;
        let mut m = 0;
        while *s != 0 {
            if *s == 'g' as c_char {
                g += 1;
            } else if *s == 'i' as c_char {
                i += 1;
            } else if *s == 'm' as c_char {
                m += 1;
            } else {
                crate::jserror::js_syntaxerror(J, cstr!("invalid regular expression flag: '%c'"), *s as c_int);
            }
            s = s.add(1);
        }
        if g > 1 {
            crate::jserror::js_syntaxerror(J, cstr!("invalid regular expression flag: 'g'"));
        }
        if i > 1 {
            crate::jserror::js_syntaxerror(J, cstr!("invalid regular expression flag: 'i'"));
        }
        if m > 1 {
            crate::jserror::js_syntaxerror(J, cstr!("invalid regular expression flag: 'm'"));
        }
        if g != 0 {
            flags |= JS_REGEXP_G;
        }
        if i != 0 {
            flags |= JS_REGEXP_I;
        }
        if m != 0 {
            flags |= JS_REGEXP_M;
        }
    }

    js_newregexpx(J, pattern, flags, is_clone);
}

unsafe extern "C-unwind" fn jsB_RegExp(J: *mut js_State) {
    if js_isregexp(J, 1) != 0 {
        return;
    }
    jsB_new_RegExp(J);
}

unsafe extern "C-unwind" fn Rp_toString(J: *mut js_State) {
    let re: *mut js_Regexp;
    let mut out: *mut c_char = std::ptr::null_mut();

    re = js_toregexp(J, 0);

    let out_ptr = std::ptr::addr_of_mut!(out);
    let caught = protect(J, || {
        *out_ptr = js_malloc(J, strlen((*re).source) as c_int + 6) as *mut c_char;
        strcpy(*out_ptr, cstr!("/"));
        strcat(*out_ptr, (*re).source);
        strcat(*out_ptr, cstr!("/"));
        if (*re).flags as c_int & JS_REGEXP_G != 0 {
            strcat(*out_ptr, cstr!("g"));
        }
        if (*re).flags as c_int & JS_REGEXP_I != 0 {
            strcat(*out_ptr, cstr!("i"));
        }
        if (*re).flags as c_int & JS_REGEXP_M != 0 {
            strcat(*out_ptr, cstr!("m"));
        }

        js_pop(J, 0);
        js_pushstring(J, *out_ptr);
    });
    if caught {
        js_free(J, out as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, out as *mut c_void);
}

unsafe extern "C-unwind" fn Rp_exec(J: *mut js_State) {
    js_RegExp_prototype_exec(J, js_toregexp(J, 0), js_tostring(J, 1));
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initregexp(J: *mut js_State) {
    js_pushobject(J, (*J).RegExp_prototype);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("RegExp.prototype.toString"), Some(Rp_toString), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("RegExp.prototype.test"), Some(Rp_test), 0);
        crate::jsbuiltin::jsB_propf(J, cstr!("RegExp.prototype.exec"), Some(Rp_exec), 0);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_RegExp), Some(jsB_new_RegExp), cstr!("RegExp"), 1);
    js_defglobal(J, cstr!("RegExp"), JS_DONTENUM);
}
