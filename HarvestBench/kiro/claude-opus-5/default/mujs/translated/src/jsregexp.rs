// Translation of c_src/src/jsregexp.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsbuiltin::jsB_propf;
use crate::jsproperty::jsV_newobject;
use crate::jsrun::{
    js_defglobal, js_endtry, js_free, js_isdefined, js_isregexp, js_isundefined, js_malloc, js_pop,
    js_pushboolean, js_pushlstring, js_pushnull, js_pushnumber, js_pushobject, js_pushstring,
    js_setindex, js_setproperty, js_strdup, js_throw, js_toregexp, js_tostring,
};
use crate::jsstring::js_utfptrtoidx;
use crate::jsvalue::{js_newarray, js_newcconstructor};
use crate::types::*;
use crate::{js_error, js_syntaxerror, js_typeerror};
use std::ffi::{c_char, c_int, c_void};

// The regexp engine API lives in c_src/src/regexp.c (translated in parallel into
// crate::regexp). That module is still a `todo!()` stub at the time this file was
// written, so the API it will export is declared here in a local
// `unsafe extern "C-unwind"` block matching c_src/src/regexp.h. These are
// declarations (not definitions), so they resolve at link time to the real
// symbols provided by crate::regexp once it is translated. `js_regfreex` is also
// declared here and re-exported (pub) so that crate::jsgc's existing
// `use crate::jsregexp::js_regfreex;` continues to resolve.

/* #define REG_MAXSUB 16 (regexp.h) */
pub const REG_MAXSUB: usize = 16;

/* regcomp flags */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
/* regexec flags */
pub const REG_NOTBOL: c_int = 4;

/* Reprog is opaque; regexp.c owns it. Alias to c_void like the C `Reprog *`
 * pointers flow through as `void *` here. */
pub use crate::regexp::Reprog;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub_sub {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

pub use crate::regexp::{Resub, js_regcompx, js_regexec, js_regfreex};

unsafe fn escaperegexp(J: *mut js_State, pattern: *const c_char) -> *mut c_char {
    unsafe {
        let copy: *mut c_char;
        let mut p: *mut c_char;
        let mut s: *const c_char;
        let mut n: c_int = 0;
        s = pattern;
        while *s != 0 {
            if *s == b'/' as c_char {
                n += 1;
            }
            n += 1;
            s = s.add(1);
        }
        copy = js_malloc(J, n + 1) as *mut c_char;
        p = copy;
        s = pattern;
        while *s != 0 {
            if *s == b'/' as c_char {
                *p = b'\\' as c_char;
                p = p.add(1);
            }
            *p = *s;
            p = p.add(1);
            s = s.add(1);
        }
        *p = 0;
        copy
    }
}

unsafe fn js_newregexpx(
    J: *mut js_State,
    pattern: *const c_char,
    flags: c_int,
    is_clone: c_int,
) {
    unsafe {
        let mut error: *const c_char = std::ptr::null();
        let obj: *mut js_Object;
        let prog: *mut Reprog;
        let mut opts: c_int;

        obj = jsV_newobject(J, JS_CREGEXP, (*J).RegExp_prototype);

        opts = 0;
        if flags & JS_REGEXP_I != 0 {
            opts |= REG_ICASE;
        }
        if flags & JS_REGEXP_M != 0 {
            opts |= REG_NEWLINE;
        }

        prog = js_regcompx((*J).alloc, (*J).actx, pattern, opts, &raw mut error);
        if prog.is_null() {
            js_syntaxerror!(J, c"regular expression: %s", error);
        }

        (*obj).u.r.prog = prog as *mut c_void;
        (*obj).u.r.source = if is_clone != 0 {
            js_strdup(J, pattern)
        } else {
            escaperegexp(J, pattern)
        };
        (*obj).u.r.flags = flags as u16;
        (*obj).u.r.last = 0;
        js_pushobject(J, obj);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_newregexp(J: *mut js_State, pattern: *const c_char, flags: c_int) {
    unsafe {
        js_newregexpx(J, pattern, flags, 0);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_RegExp_prototype_exec(
    J: *mut js_State,
    re: *mut js_Regexp,
    text: *const c_char,
) {
    unsafe {
        let mut haystack: *const c_char;
        let result: c_int;
        let mut i: c_int;
        let mut opts: c_int;
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
                if (*re).flags as c_int & JS_REGEXP_M == 0 || *haystack.offset(-1) != b'\n' as c_char {
                    opts |= REG_NOTBOL;
                }
            }
        }

        result = js_regexec((*re).prog as *mut Reprog, haystack, &raw mut m, opts);
        if result < 0 {
            js_error!(J, c"regexec failed");
        }
        if result == 0 {
            js_newarray(J);
            js_pushstring(J, text);
            js_setproperty(J, -2, c"input".as_ptr());
            js_pushnumber(J, js_utfptrtoidx(text, m.sub[0].sp) as f64);
            js_setproperty(J, -2, c"index".as_ptr());
            i = 0;
            while i < m.nsub {
                js_pushlstring(
                    J,
                    m.sub[i as usize].sp,
                    (m.sub[i as usize].ep as isize - m.sub[i as usize].sp as isize) as c_int,
                );
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
}

unsafe extern "C-unwind" fn Rp_test(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let mut text: *const c_char;
        let result: c_int;
        let mut opts: c_int;
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
                if (*re).flags as c_int & JS_REGEXP_M == 0 || *text.offset(-1) != b'\n' as c_char {
                    opts |= REG_NOTBOL;
                }
            }
        }

        result = js_regexec((*re).prog as *mut Reprog, text, &raw mut m, opts);
        if result < 0 {
            js_error!(J, c"regexec failed");
        }
        if result == 0 {
            if (*re).flags as c_int & JS_REGEXP_G != 0 {
                (*re).last =
                    ((*re).last as isize + (m.sub[0].ep as isize - text as isize)) as u16;
            }
            js_pushboolean(J, 1);
            return;
        }

        if (*re).flags as c_int & JS_REGEXP_G != 0 {
            (*re).last = 0;
        }

        js_pushboolean(J, 0);
    }
}

unsafe extern "C-unwind" fn jsB_new_RegExp(J: *mut js_State) {
    unsafe {
        let old: *mut js_Regexp;
        let mut pattern: *const c_char;
        let mut flags: c_int;
        let mut is_clone: c_int = 0;

        if js_isregexp(J, 1) != 0 {
            if js_isdefined(J, 2) != 0 {
                js_typeerror!(J, c"cannot supply flags when creating one RegExp from another");
            }
            old = js_toregexp(J, 1);
            pattern = (*old).source;
            flags = (*old).flags as c_int;
            is_clone = 1;
        } else if js_isundefined(J, 1) != 0 {
            pattern = c"(?:)".as_ptr();
            flags = 0;
        } else {
            pattern = js_tostring(J, 1);
            flags = 0;
        }

        if strlen(pattern) == 0 {
            pattern = c"(?:)".as_ptr();
        }

        if js_isdefined(J, 2) != 0 {
            let mut s = js_tostring(J, 2);
            let mut g: c_int = 0;
            let mut i: c_int = 0;
            let mut m: c_int = 0;
            while *s != 0 {
                if *s == b'g' as c_char {
                    g += 1;
                } else if *s == b'i' as c_char {
                    i += 1;
                } else if *s == b'm' as c_char {
                    m += 1;
                } else {
                    js_syntaxerror!(J, c"invalid regular expression flag: '%c'", *s as c_int);
                }
                s = s.add(1);
            }
            if g > 1 {
                js_syntaxerror!(J, c"invalid regular expression flag: 'g'");
            }
            if i > 1 {
                js_syntaxerror!(J, c"invalid regular expression flag: 'i'");
            }
            if m > 1 {
                js_syntaxerror!(J, c"invalid regular expression flag: 'm'");
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
}

unsafe extern "C-unwind" fn jsB_RegExp(J: *mut js_State) {
    unsafe {
        if js_isregexp(J, 1) != 0 {
            return;
        }
        jsB_new_RegExp(J);
    }
}

unsafe extern "C-unwind" fn Rp_toString(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let mut out: *mut c_char = std::ptr::null_mut();

        re = js_toregexp(J, 0);

        if js_try(J, || {
            out = js_malloc(J, (strlen((*re).source) + 6) as c_int) as *mut c_char; /* extra space for //gim */
            strcpy(out, c"/".as_ptr());
            strcat(out, (*re).source);
            strcat(out, c"/".as_ptr());
            if (*re).flags as c_int & JS_REGEXP_G != 0 {
                strcat(out, c"g".as_ptr());
            }
            if (*re).flags as c_int & JS_REGEXP_I != 0 {
                strcat(out, c"i".as_ptr());
            }
            if (*re).flags as c_int & JS_REGEXP_M != 0 {
                strcat(out, c"m".as_ptr());
            }

            js_pop(J, 0);
            js_pushstring(J, out);
            js_endtry(J);
            js_free(J, out as *mut c_void);
        })
        .is_err()
        {
            js_free(J, out as *mut c_void);
            js_throw(J);
        }
    }
}

unsafe extern "C-unwind" fn Rp_exec(J: *mut js_State) {
    unsafe {
        js_RegExp_prototype_exec(J, js_toregexp(J, 0), js_tostring(J, 1));
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initregexp(J: *mut js_State) {
    unsafe {
        js_pushobject(J, (*J).RegExp_prototype);
        {
            jsB_propf(J, c"RegExp.prototype.toString".as_ptr(), Some(Rp_toString), 0);
            jsB_propf(J, c"RegExp.prototype.test".as_ptr(), Some(Rp_test), 0);
            jsB_propf(J, c"RegExp.prototype.exec".as_ptr(), Some(Rp_exec), 0);
        }
        js_newcconstructor(J, Some(jsB_RegExp), Some(jsB_new_RegExp), c"RegExp".as_ptr(), 1);
        js_defglobal(J, c"RegExp".as_ptr(), JS_DONTENUM);
    }
}
