//! Translation of src/jsstring.c

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(unused)]

use crate::jsi::*;

use crate::jsbuiltin::jsB_propf;
use crate::jsintern::{js_putc, js_putm, js_puts};
use crate::regexp::{Reprog, js_regexec};
use crate::utf::{
    jsU_chartorune, jsU_runelen, jsU_runetochar, jsU_tolowerrune,
    jsU_tolowerrune_full, jsU_toupperrune, jsU_toupperrune_full,
};

use crate::jsrun::{
    js_call, js_copy, js_defglobal, js_free, js_gettop, js_iscallable, js_iscoercible,
    js_isdefined, js_isregexp, js_isundefined, js_malloc, js_pop, js_pushliteral,
    js_pushlstring, js_pushnull, js_pushnumber, js_pushobject, js_pushstring, js_pushundefined,
    js_realloc, js_setindex, js_throw, js_tointeger, js_toobject, js_toregexp, js_tostring,
    js_touint32,
};
use crate::jsvalue::{js_newarray, js_newcconstructor, js_newstring};

/* C EOF */
const EOF: c_int = -1;

/* the utf.h macro renames */
#[inline(always)]
unsafe fn chartorune(rune: *mut Rune, s: *const c_char) -> c_int {
    unsafe { jsU_chartorune(rune, s) }
}
#[inline(always)]
unsafe fn runetochar(s: *mut c_char, rune: *const Rune) -> c_int {
    unsafe { jsU_runetochar(s, rune) }
}
#[inline(always)]
unsafe fn runelen(c: c_int) -> c_int {
    unsafe { jsU_runelen(c) }
}
#[inline(always)]
unsafe fn tolowerrune(c: Rune) -> Rune {
    unsafe { jsU_tolowerrune(c) }
}
#[inline(always)]
unsafe fn toupperrune(c: Rune) -> Rune {
    unsafe { jsU_toupperrune(c) }
}
#[inline(always)]
unsafe fn tolowerrune_full(c: Rune) -> *const Rune {
    unsafe { jsU_tolowerrune_full(c) }
}
#[inline(always)]
unsafe fn toupperrune_full(c: Rune) -> *const Rune {
    unsafe { jsU_toupperrune_full(c) }
}

unsafe fn js_doregexec(
    J: *mut js_State,
    prog: *mut Reprog,
    string: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    unsafe {
        let result = js_regexec(prog, string, sub, eflags);
        if result < 0 {
            js_error!(J, c"regexec failed".as_ptr());
        }
        result
    }
}

unsafe fn checkstring(J: *mut js_State, idx: c_int) -> *const c_char {
    unsafe {
        if js_iscoercible(J, idx) == 0 {
            js_typeerror!(J, c"string function called on null or undefined".as_ptr());
        }
        js_tostring(J, idx)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_runeat(J: *mut js_State, s: *const c_char, i: c_int) -> c_int {
    unsafe {
        let mut s = s;
        let mut i = i;
        let mut rune: Rune = EOF;
        while i >= 0 {
            rune = *(s as *const c_uchar) as c_int;
            if rune < Runeself {
                if rune == 0 {
                    return EOF;
                }
                s = s.add(1);
                i -= 1;
            } else {
                s = s.add(chartorune(&mut rune, s) as usize);
                if rune >= 0x10000 {
                    i -= 2;
                } else {
                    i -= 1;
                }
            }
        }
        if rune >= 0x10000 {
            /* high surrogate */
            if i == -2 {
                return 0xd800 + ((rune - 0x10000) >> 10);
            }
            /* low surrogate */
            else {
                return 0xdc00 + ((rune - 0x10000) & 0x3ff);
            }
        }
        rune
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_utflen(s: *const c_char) -> c_int {
    unsafe {
        let mut s = s;
        let mut c: c_int;
        let mut n: c_int;
        let mut rune: Rune = 0;

        n = 0;
        loop {
            c = *(s as *const c_uchar) as c_int;
            if c < Runeself {
                if c == 0 {
                    return n;
                }
                s = s.add(1);
                n += 1;
            } else {
                s = s.add(chartorune(&mut rune, s) as usize);
                if rune >= 0x10000 {
                    n += 2;
                } else {
                    n += 1;
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_utfptrtoidx(s: *const c_char, p: *const c_char) -> c_int {
    unsafe {
        let mut s = s;
        let mut rune: Rune = 0;
        let mut i: c_int = 0;
        while s < p {
            if (*(s as *const c_uchar) as c_int) < Runeself {
                s = s.add(1);
                i += 1;
            } else {
                s = s.add(chartorune(&mut rune, s) as usize);
                if rune >= 0x10000 {
                    i += 2;
                } else {
                    i += 1;
                }
            }
        }
        i
    }
}

unsafe extern "C-unwind" fn jsB_new_String(J: *mut js_State) {
    unsafe {
        js_newstring(
            J,
            if js_gettop(J) > 1 {
                js_tostring(J, 1)
            } else {
                c"".as_ptr()
            },
        );
    }
}

unsafe extern "C-unwind" fn jsB_String(J: *mut js_State) {
    unsafe {
        js_pushstring(
            J,
            if js_gettop(J) > 1 {
                js_tostring(J, 1)
            } else {
                c"".as_ptr()
            },
        );
    }
}

unsafe extern "C-unwind" fn Sp_toString(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        if (*self_).ty != JS_CSTRING {
            js_typeerror!(J, c"not a string".as_ptr());
        }
        js_pushstring(J, (*self_).u.s.string);
    }
}

unsafe extern "C-unwind" fn Sp_valueOf(J: *mut js_State) {
    unsafe {
        let self_ = js_toobject(J, 0);
        if (*self_).ty != JS_CSTRING {
            js_typeerror!(J, c"not a string".as_ptr());
        }
        js_pushstring(J, (*self_).u.s.string);
    }
}

unsafe extern "C-unwind" fn Sp_charAt(J: *mut js_State) {
    unsafe {
        let mut buf: [c_char; (UTFmax + 1) as usize] = [0; (UTFmax + 1) as usize];
        let s = checkstring(J, 0);
        let pos = js_tointeger(J, 1);
        let mut rune: Rune = js_runeat(J, s, pos);
        if rune >= 0 {
            let k = runetochar(buf.as_mut_ptr(), &rune);
            buf[k as usize] = 0;
            js_pushstring(J, buf.as_ptr());
        } else {
            js_pushliteral(J, c"".as_ptr());
        }
    }
}

unsafe extern "C-unwind" fn Sp_charCodeAt(J: *mut js_State) {
    unsafe {
        let s = checkstring(J, 0);
        let pos = js_tointeger(J, 1);
        let rune: Rune = js_runeat(J, s, pos);
        if rune >= 0 {
            js_pushnumber(J, rune as f64);
        } else {
            js_pushnumber(J, NAN);
        }
    }
}

unsafe extern "C-unwind" fn Sp_concat(J: *mut js_State) {
    unsafe {
        let mut i: c_int = 0;
        let top = js_gettop(J);
        let mut n: c_int;
        let mut out: *mut c_char = core::ptr::null_mut();
        let mut s: *const c_char = core::ptr::null();

        if top == 1 {
            return;
        }

        s = checkstring(J, 0);
        n = 1 + strlen(s) as c_int;

        if crate::except::js_try_run(J, || {
            if n > JS_STRLIMIT {
                js_rangeerror!(J, c"invalid string length".as_ptr());
            }
            out = js_malloc(J, n) as *mut c_char;
            strcpy(out, s);

            i = 1;
            while i < top {
                s = js_tostring(J, i);
                n += strlen(s) as c_int;
                if n > JS_STRLIMIT {
                    js_rangeerror!(J, c"invalid string length".as_ptr());
                }
                out = js_realloc(J, out as *mut c_void, n) as *mut c_char;
                strcat(out, s);
                i += 1;
            }

            js_pushstring(J, out);
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, out as *mut c_void);
            js_throw(J);
        }
        js_free(J, out as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_indexOf(J: *mut js_State) {
    unsafe {
        let mut haystack = checkstring(J, 0);
        let needle = js_tostring(J, 1);
        let pos = js_tointeger(J, 2);
        let len = strlen(needle) as c_int;
        let mut k: c_int = 0;
        let mut rune: Rune = 0;
        while *haystack != 0 {
            if k >= pos && strncmp(haystack, needle, len as size_t) == 0 {
                js_pushnumber(J, k as f64);
                return;
            }
            haystack = haystack.add(chartorune(&mut rune, haystack) as usize);
            k += 1;
        }
        js_pushnumber(J, -1.0);
    }
}

unsafe extern "C-unwind" fn Sp_lastIndexOf(J: *mut js_State) {
    unsafe {
        let mut haystack = checkstring(J, 0);
        let needle = js_tostring(J, 1);
        let pos = if js_isdefined(J, 2) != 0 {
            js_tointeger(J, 2)
        } else {
            strlen(haystack) as c_int
        };
        let len = strlen(needle) as c_int;
        let mut k: c_int = 0;
        let mut last: c_int = -1;
        let mut rune: Rune = 0;
        while *haystack != 0 && k <= pos {
            if strncmp(haystack, needle, len as size_t) == 0 {
                last = k;
            }
            haystack = haystack.add(chartorune(&mut rune, haystack) as usize);
            k += 1;
        }
        js_pushnumber(J, last as f64);
    }
}

unsafe extern "C-unwind" fn Sp_localeCompare(J: *mut js_State) {
    unsafe {
        let a = checkstring(J, 0);
        let b = js_tostring(J, 1);
        js_pushnumber(J, strcmp(a, b) as f64);
    }
}

unsafe fn Sp_substring_imp(J: *mut js_State, s: *const c_char, a: c_int, n: c_int) {
    unsafe {
        let mut head_rune: Rune = 0;
        let mut tail_rune: Rune = 0;
        let mut head: *const c_char;
        let mut tail: *const c_char;
        let mut p: *mut c_char = core::ptr::null_mut();
        let mut i: c_int = 0;
        let mut k: c_int;

        /* find start of substring */
        head = s;
        i = 0;
        while i < a {
            head = head.add(chartorune(&mut head_rune, head) as usize);
            if head_rune >= 0x10000 {
                i += 1;
            }
            i += 1;
        }

        /* find end of substring */
        tail = head;
        k = i - a;
        while k < n {
            tail = tail.add(chartorune(&mut tail_rune, tail) as usize);
            if tail_rune >= 0x10000 {
                k += 1;
            }
            k += 1;
        }

        /* no surrogate pair splits! */
        if i == a && k == n {
            js_pushlstring(J, head, tail.offset_from(head) as c_int);
            return;
        }

        if crate::except::js_try_run(J, || {
            p = js_malloc(J, UTFmax + tail.offset_from(head) as c_int) as *mut c_char;

            /* substring starts with low surrogate (head is just after character) */
            if i > a {
                head_rune = 0xdc00 + ((head_rune - 0x10000) & 0x3ff);
                let head_len = runetochar(p, &head_rune);
                memcpy(
                    p.add(head_len as usize) as *mut c_void,
                    head as *const c_void,
                    tail.offset_from(head) as size_t,
                );
                js_pushlstring(J, p, head_len + tail.offset_from(head) as c_int);
            }

            /* substring ends with high surrogate (tail is just after character) */
            if k > n {
                tail = tail.offset(-(runelen(tail_rune) as isize));
                memcpy(
                    p as *mut c_void,
                    head as *const c_void,
                    tail.offset_from(head) as size_t,
                );
                tail_rune = 0xd800 + ((tail_rune - 0x10000) >> 10);
                let tail_len = runetochar(p.add(tail.offset_from(head) as usize), &tail_rune);
                js_pushlstring(J, p, tail.offset_from(head) as c_int + tail_len);
            }

            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, p as *mut c_void);
            js_throw(J);
        }
        js_free(J, p as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_slice(J: *mut js_State) {
    unsafe {
        let str = checkstring(J, 0);
        let len = js_utflen(str);
        let mut s = js_tointeger(J, 1);
        let mut e = if js_isdefined(J, 2) != 0 {
            js_tointeger(J, 2)
        } else {
            len
        };

        s = if s < 0 { s + len } else { s };
        e = if e < 0 { e + len } else { e };

        s = if s < 0 {
            0
        } else if s > len {
            len
        } else {
            s
        };
        e = if e < 0 {
            0
        } else if e > len {
            len
        } else {
            e
        };

        if s < e {
            Sp_substring_imp(J, str, s, e - s);
        } else if s > e {
            Sp_substring_imp(J, str, e, s - e);
        } else {
            js_pushliteral(J, c"".as_ptr());
        }
    }
}

unsafe extern "C-unwind" fn Sp_substring(J: *mut js_State) {
    unsafe {
        let str = checkstring(J, 0);
        let len = js_utflen(str);
        let mut s = js_tointeger(J, 1);
        let mut e = if js_isdefined(J, 2) != 0 {
            js_tointeger(J, 2)
        } else {
            len
        };

        s = if s < 0 {
            0
        } else if s > len {
            len
        } else {
            s
        };
        e = if e < 0 {
            0
        } else if e > len {
            len
        } else {
            e
        };

        if s < e {
            Sp_substring_imp(J, str, s, e - s);
        } else if s > e {
            Sp_substring_imp(J, str, e, s - e);
        } else {
            js_pushliteral(J, c"".as_ptr());
        }
    }
}

unsafe extern "C-unwind" fn Sp_toLowerCase(J: *mut js_State) {
    unsafe {
        let s0 = checkstring(J, 0);
        let mut s: *const c_char = core::ptr::null();
        let mut dst: *mut c_char = core::ptr::null_mut();
        let mut d: *mut c_char = core::ptr::null_mut();
        let mut rune: Rune = 0;
        let mut full: *const Rune = core::ptr::null();
        let mut n: c_int;

        n = 1;
        s = s0;
        while *s != 0 {
            s = s.add(chartorune(&mut rune, s) as usize);
            full = tolowerrune_full(rune);
            if !full.is_null() {
                let mut f = full;
                while *f != 0 {
                    n += runelen(*f);
                    f = f.add(1);
                }
            } else {
                rune = tolowerrune(rune);
                n += runelen(rune);
            }
        }

        if crate::except::js_try_run(J, || {
            dst = js_malloc(J, n) as *mut c_char;
            d = dst;
            s = s0;
            while *s != 0 {
                s = s.add(chartorune(&mut rune, s) as usize);
                full = tolowerrune_full(rune);
                if !full.is_null() {
                    let mut f = full;
                    while *f != 0 {
                        d = d.add(runetochar(d, f) as usize);
                        f = f.add(1);
                    }
                } else {
                    rune = tolowerrune(rune);
                    d = d.add(runetochar(d, &rune) as usize);
                }
            }
            *d = 0;

            js_pushstring(J, dst);
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, dst as *mut c_void);
            js_throw(J);
        }
        js_free(J, dst as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_toUpperCase(J: *mut js_State) {
    unsafe {
        let s0 = checkstring(J, 0);
        let mut s: *const c_char = core::ptr::null();
        let mut dst: *mut c_char = core::ptr::null_mut();
        let mut d: *mut c_char = core::ptr::null_mut();
        let mut full: *const Rune = core::ptr::null();
        let mut rune: Rune = 0;
        let mut n: c_int;

        n = 1;
        s = s0;
        while *s != 0 {
            s = s.add(chartorune(&mut rune, s) as usize);
            full = toupperrune_full(rune);
            if !full.is_null() {
                let mut f = full;
                while *f != 0 {
                    n += runelen(*f);
                    f = f.add(1);
                }
            } else {
                rune = toupperrune(rune);
                n += runelen(rune);
            }
        }

        if crate::except::js_try_run(J, || {
            dst = js_malloc(J, n) as *mut c_char;
            d = dst;
            s = s0;
            while *s != 0 {
                s = s.add(chartorune(&mut rune, s) as usize);
                full = toupperrune_full(rune);
                if !full.is_null() {
                    let mut f = full;
                    while *f != 0 {
                        d = d.add(runetochar(d, f) as usize);
                        f = f.add(1);
                    }
                } else {
                    rune = toupperrune(rune);
                    d = d.add(runetochar(d, &rune) as usize);
                }
            }
            *d = 0;

            js_pushstring(J, dst);
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, dst as *mut c_void);
            js_throw(J);
        }
        js_free(J, dst as *mut c_void);
    }
}

unsafe fn isbol(re: *mut js_Regexp, text: *const c_char, a: *const c_char) -> c_int {
    unsafe {
        (a == text
            || (((*re).flags as c_int & JS_REGEXP_M) != 0 && *a.offset(-1) as c_int == '\n' as c_int))
            as c_int
    }
}

unsafe fn istrim(c: c_int) -> c_int {
    (c == 0x9
        || c == 0xB
        || c == 0xC
        || c == 0x20
        || c == 0xA0
        || c == 0xFEFF
        || c == 0xA
        || c == 0xD
        || c == 0x2028
        || c == 0x2029) as c_int
}

unsafe extern "C-unwind" fn Sp_trim(J: *mut js_State) {
    unsafe {
        let mut s: *const c_char = core::ptr::null();
        let mut e: *const c_char;
        s = checkstring(J, 0);
        while istrim(*s as c_int) != 0 {
            s = s.add(1);
        }
        e = s.add(strlen(s));
        while e > s && istrim(*e.offset(-1) as c_int) != 0 {
            e = e.offset(-1);
        }
        js_pushlstring(J, s, e.offset_from(s) as c_int);
    }
}

unsafe extern "C-unwind" fn S_fromCharCode(J: *mut js_State) {
    unsafe {
        let mut i: c_int = 0;
        let top = js_gettop(J);
        let mut s: *mut c_char = core::ptr::null_mut();
        let mut p: *mut c_char = core::ptr::null_mut();
        let mut c: Rune = 0;

        if crate::except::js_try_run(J, || {
            s = js_malloc(J, (top - 1) * UTFmax + 1) as *mut c_char;
            p = s;

            i = 1;
            while i < top {
                c = js_touint32(J, i) as Rune;
                p = p.add(runetochar(p, &c) as usize);
                i += 1;
            }
            *p = 0;

            js_pushstring(J, s);
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, s as *mut c_void);
            js_throw(J);
        }
        js_free(J, s as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_match(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let text: *const c_char;
        let mut len: c_int;
        let mut a: *const c_char;
        let b: *const c_char;
        let c: *const c_char;
        let e: *const c_char;
        let mut m: Resub = Resub::new();
        let mut rune: Rune = 0;

        text = checkstring(J, 0);

        if js_isregexp(J, 1) != 0 {
            js_copy(J, 1);
        } else if js_isundefined(J, 1) != 0 {
            crate::jsregexp::js_newregexp(J, c"".as_ptr(), 0);
        } else {
            crate::jsregexp::js_newregexp(J, js_tostring(J, 1), 0);
        }

        re = js_toregexp(J, -1);
        if ((*re).flags as c_int & JS_REGEXP_G) == 0 {
            crate::jsregexp::js_RegExp_prototype_exec(J, re, text);
            return;
        }

        (*re).last = 0;

        js_newarray(J);

        len = 0;
        a = text;
        e = text.add(strlen(text));
        while a <= e {
            if js_doregexec(
                J,
                (*re).prog as *mut Reprog,
                a,
                &mut m,
                if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL },
            ) != 0
            {
                break;
            }

            let b = m.sub[0].sp;
            let c = m.sub[0].ep;

            js_pushlstring(J, b, c.offset_from(b) as c_int);
            js_setindex(J, -2, len);
            len += 1;

            a = c;
            if c.offset_from(b) == 0 {
                a = a.add(chartorune(&mut rune, a) as usize);
            }
        }

        if len == 0 {
            js_pop(J, 1);
            js_pushnull(J);
        }
    }
}

unsafe extern "C-unwind" fn Sp_search(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let text: *const c_char;
        let mut m: Resub = Resub::new();

        text = checkstring(J, 0);

        if js_isregexp(J, 1) != 0 {
            js_copy(J, 1);
        } else if js_isundefined(J, 1) != 0 {
            crate::jsregexp::js_newregexp(J, c"".as_ptr(), 0);
        } else {
            crate::jsregexp::js_newregexp(J, js_tostring(J, 1), 0);
        }

        re = js_toregexp(J, -1);

        if js_doregexec(J, (*re).prog as *mut Reprog, text, &mut m, 0) == 0 {
            js_pushnumber(J, js_utfptrtoidx(text, m.sub[0].sp) as f64);
        } else {
            js_pushnumber(J, -1.0);
        }
    }
}

unsafe extern "C-unwind" fn Sp_replace_regexp(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let mut source: *const c_char;
        let source0: *const c_char;
        let mut s: *const c_char = core::ptr::null();
        let mut r: *const c_char = core::ptr::null();
        let mut sb: *mut js_Buffer = core::ptr::null_mut();
        let mut n: c_int = 0;
        let mut x: c_int = 0;
        let mut m: Resub = Resub::new();

        source = checkstring(J, 0);
        source0 = source;
        re = js_toregexp(J, 1);

        if js_doregexec(J, (*re).prog as *mut Reprog, source, &mut m, 0) != 0 {
            js_copy(J, 0);
            return;
        }

        (*re).last = 0;

        if crate::except::js_try_run(J, || {
            'end: {
                'loop_: loop {
                    s = m.sub[0].sp;
                    n = m.sub[0].ep.offset_from(m.sub[0].sp) as c_int;

                    if js_iscallable(J, 2) != 0 {
                        js_copy(J, 2);
                        js_pushundefined(J);
                        x = 0;
                        while !m.sub[x as usize].sp.is_null() {
                            /* arg 0..x: substring and subexps that matched */
                            js_pushlstring(
                                J,
                                m.sub[x as usize].sp,
                                m.sub[x as usize].ep.offset_from(m.sub[x as usize].sp) as c_int,
                            );
                            x += 1;
                        }
                        js_pushnumber(J, s.offset_from(source) as f64); /* arg x+2: offset within search string */
                        js_copy(J, 0); /* arg x+3: search string */
                        js_call(J, 2 + x);
                        r = js_tostring(J, -1);
                        js_putm(J, &mut sb, source, s);
                        js_puts(J, &mut sb, r);
                        js_pop(J, 1);
                    } else {
                        r = js_tostring(J, 2);
                        js_putm(J, &mut sb, source, s);
                        while *r != 0 {
                            if *r as c_int == '$' as c_int {
                                r = r.add(1);
                                let ch = *r as c_int;
                                if ch == 0 {
                                    r = r.offset(-1); /* end of string; back up */
                                    js_putc(J, &mut sb, '$' as c_int);
                                } else if ch == '$' as c_int {
                                    js_putc(J, &mut sb, '$' as c_int);
                                } else if ch == '`' as c_int {
                                    js_putm(J, &mut sb, source0, s);
                                } else if ch == '\'' as c_int {
                                    js_puts(J, &mut sb, s.add(n as usize));
                                } else if ch == '&' as c_int {
                                    js_putm(J, &mut sb, s, s.add(n as usize));
                                } else if ch >= '0' as c_int && ch <= '9' as c_int {
                                    x = *r as c_int - '0' as c_int;
                                    if *r.add(1) as c_int >= '0' as c_int
                                        && *r.add(1) as c_int <= '9' as c_int
                                    {
                                        r = r.add(1);
                                        x = x * 10 + *r as c_int - '0' as c_int;
                                    }
                                    if x > 0 && x < m.nsub {
                                        js_putm(J, &mut sb, m.sub[x as usize].sp, m.sub[x as usize].ep);
                                    } else {
                                        js_putc(J, &mut sb, '$' as c_int);
                                        if x > 10 {
                                            js_putc(J, &mut sb, '0' as c_int + x / 10);
                                            js_putc(J, &mut sb, '0' as c_int + x % 10);
                                        } else {
                                            js_putc(J, &mut sb, '0' as c_int + x);
                                        }
                                    }
                                } else {
                                    js_putc(J, &mut sb, '$' as c_int);
                                    js_putc(J, &mut sb, *r as c_int);
                                }
                                r = r.add(1);
                            } else {
                                js_putc(J, &mut sb, *r as c_int);
                                r = r.add(1);
                            }
                        }
                    }

                    if ((*re).flags as c_int & JS_REGEXP_G) != 0 {
                        source = m.sub[0].ep;
                        if n == 0 {
                            if *source != 0 {
                                js_putc(J, &mut sb, *source as c_int);
                                source = source.add(1);
                            } else {
                                break 'end;
                            }
                        }
                        if js_doregexec(
                            J,
                            (*re).prog as *mut Reprog,
                            source,
                            &mut m,
                            if isbol(re, source0, source) != 0 { 0 } else { REG_NOTBOL },
                        ) == 0
                        {
                            continue 'loop_;
                        }
                    }
                    break 'loop_;
                }
            }
            /* end: */
            js_puts(J, &mut sb, s.add(n as usize));
            js_putc(J, &mut sb, 0);

            js_pushstring(J, if !sb.is_null() { sbs(sb) } else { c"".as_ptr() });
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, sb as *mut c_void);
            js_throw(J);
        }
        js_free(J, sb as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_replace_string(J: *mut js_State) {
    unsafe {
        let source: *const c_char;
        let needle: *const c_char;
        let s: *const c_char;
        let mut r: *const c_char = core::ptr::null();
        let mut sb: *mut js_Buffer = core::ptr::null_mut();
        let n: c_int;

        source = checkstring(J, 0);
        needle = js_tostring(J, 1);

        s = strstr(source, needle);
        if s.is_null() {
            js_copy(J, 0);
            return;
        }
        n = strlen(needle) as c_int;

        if crate::except::js_try_run(J, || {
            if js_iscallable(J, 2) != 0 {
                js_copy(J, 2);
                js_pushundefined(J);
                js_pushlstring(J, s, n); /* arg 1: substring that matched */
                js_pushnumber(J, s.offset_from(source) as f64); /* arg 2: offset within search string */
                js_copy(J, 0); /* arg 3: search string */
                js_call(J, 3);
                r = js_tostring(J, -1);
                js_putm(J, &mut sb, source, s);
                js_puts(J, &mut sb, r);
                js_puts(J, &mut sb, s.add(n as usize));
                js_putc(J, &mut sb, 0);
                js_pop(J, 1);
            } else {
                r = js_tostring(J, 2);
                js_putm(J, &mut sb, source, s);
                while *r != 0 {
                    if *r as c_int == '$' as c_int {
                        r = r.add(1);
                        let ch = *r as c_int;
                        if ch == 0 {
                            r = r.offset(-1); /* end of string; back up */
                            js_putc(J, &mut sb, '$' as c_int);
                        } else if ch == '$' as c_int {
                            js_putc(J, &mut sb, '$' as c_int);
                        } else if ch == '&' as c_int {
                            js_putm(J, &mut sb, s, s.add(n as usize));
                        } else if ch == '`' as c_int {
                            js_putm(J, &mut sb, source, s);
                        } else if ch == '\'' as c_int {
                            js_puts(J, &mut sb, s.add(n as usize));
                        } else {
                            js_putc(J, &mut sb, '$' as c_int);
                            js_putc(J, &mut sb, *r as c_int);
                        }
                        r = r.add(1);
                    } else {
                        js_putc(J, &mut sb, *r as c_int);
                        r = r.add(1);
                    }
                }
                js_puts(J, &mut sb, s.add(n as usize));
                js_putc(J, &mut sb, 0);
            }

            js_pushstring(J, if !sb.is_null() { sbs(sb) } else { c"".as_ptr() });
            crate::jsrun::js_endtry(J);
        }) {
            js_free(J, sb as *mut c_void);
            js_throw(J);
        }
        js_free(J, sb as *mut c_void);
    }
}

unsafe extern "C-unwind" fn Sp_replace(J: *mut js_State) {
    unsafe {
        if js_isregexp(J, 1) != 0 {
            Sp_replace_regexp(J);
        } else {
            Sp_replace_string(J);
        }
    }
}

unsafe extern "C-unwind" fn Sp_split_regexp(J: *mut js_State) {
    unsafe {
        let re: *mut js_Regexp;
        let text: *const c_char;
        let limit: c_int;
        let mut len: c_int;
        let mut k: c_int;
        let mut p: *const c_char;
        let mut a: *const c_char;
        let b: *const c_char;
        let c: *const c_char;
        let e: *const c_char;
        let mut m: Resub = Resub::new();
        let mut rune: Rune = 0;

        text = checkstring(J, 0);
        re = js_toregexp(J, 1);
        limit = if js_isdefined(J, 2) != 0 {
            js_tointeger(J, 2)
        } else {
            1 << 30
        };

        js_newarray(J);
        len = 0;

        if limit == 0 {
            return;
        }

        e = text.add(strlen(text));

        /* splitting the empty string */
        if e == text {
            if js_doregexec(J, (*re).prog as *mut Reprog, text, &mut m, 0) != 0 {
                js_pushliteral(J, c"".as_ptr());
                js_setindex(J, -2, 0);
            }
            return;
        }

        p = text;
        a = text;
        while a < e {
            if js_doregexec(
                J,
                (*re).prog as *mut Reprog,
                a,
                &mut m,
                if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL },
            ) != 0
            {
                break; /* no match */
            }

            let b = m.sub[0].sp;
            let c = m.sub[0].ep;

            /* empty string at end of last match */
            if b == c && b == p {
                a = a.add(chartorune(&mut rune, a) as usize);
                continue;
            }

            if len == limit {
                return;
            }
            js_pushlstring(J, p, b.offset_from(p) as c_int);
            js_setindex(J, -2, len);
            len += 1;

            k = 1;
            while k < m.nsub {
                if len == limit {
                    return;
                }
                js_pushlstring(
                    J,
                    m.sub[k as usize].sp,
                    m.sub[k as usize].ep.offset_from(m.sub[k as usize].sp) as c_int,
                );
                js_setindex(J, -2, len);
                len += 1;
                k += 1;
            }

            a = c;
            p = c;
        }

        if len == limit {
            return;
        }
        js_pushstring(J, p);
        js_setindex(J, -2, len);
    }
}

unsafe extern "C-unwind" fn Sp_split_string(J: *mut js_State) {
    unsafe {
        let mut str = checkstring(J, 0);
        let sep = js_tostring(J, 1);
        let limit = if js_isdefined(J, 2) != 0 {
            js_tointeger(J, 2)
        } else {
            1 << 30
        };
        let mut i: c_int = 0;
        let mut n: c_int;

        js_newarray(J);

        if limit == 0 {
            return;
        }

        n = strlen(sep) as c_int;

        /* empty string */
        if n == 0 {
            let mut rune: Rune = 0;
            i = 0;
            while *str != 0 && i < limit {
                n = chartorune(&mut rune, str);
                js_pushlstring(J, str, n);
                js_setindex(J, -2, i);
                str = str.add(n as usize);
                i += 1;
            }
            return;
        }

        i = 0;
        while !str.is_null() && i < limit {
            let s = strstr(str, sep);
            if !s.is_null() {
                js_pushlstring(J, str, s.offset_from(str) as c_int);
                js_setindex(J, -2, i);
                str = s.add(n as usize);
            } else {
                js_pushstring(J, str);
                js_setindex(J, -2, i);
                str = core::ptr::null();
            }
            i += 1;
        }
    }
}

unsafe extern "C-unwind" fn Sp_split(J: *mut js_State) {
    unsafe {
        if js_isundefined(J, 1) != 0 {
            js_newarray(J);
            js_pushstring(J, js_tostring(J, 0));
            js_setindex(J, -2, 0);
        } else if js_isregexp(J, 1) != 0 {
            Sp_split_regexp(J);
        } else {
            Sp_split_string(J);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initstring(J: *mut js_State) {
    unsafe {
        (*(*J).String_prototype).u.s.shrstr[0] = 0;
        (*(*J).String_prototype).u.s.string =
            (&raw mut (*(*J).String_prototype).u.s.shrstr) as *mut c_char;
        (*(*J).String_prototype).u.s.length = 0;

        js_pushobject(J, (*J).String_prototype);
        {
            jsB_propf(J, c"String.prototype.toString".as_ptr(), Some(Sp_toString), 0);
            jsB_propf(J, c"String.prototype.valueOf".as_ptr(), Some(Sp_valueOf), 0);
            jsB_propf(J, c"String.prototype.charAt".as_ptr(), Some(Sp_charAt), 1);
            jsB_propf(
                J,
                c"String.prototype.charCodeAt".as_ptr(),
                Some(Sp_charCodeAt),
                1,
            );
            jsB_propf(J, c"String.prototype.concat".as_ptr(), Some(Sp_concat), 0); /* 1 */
            jsB_propf(J, c"String.prototype.indexOf".as_ptr(), Some(Sp_indexOf), 1);
            jsB_propf(
                J,
                c"String.prototype.lastIndexOf".as_ptr(),
                Some(Sp_lastIndexOf),
                1,
            );
            jsB_propf(
                J,
                c"String.prototype.localeCompare".as_ptr(),
                Some(Sp_localeCompare),
                1,
            );
            jsB_propf(J, c"String.prototype.match".as_ptr(), Some(Sp_match), 1);
            jsB_propf(J, c"String.prototype.replace".as_ptr(), Some(Sp_replace), 2);
            jsB_propf(J, c"String.prototype.search".as_ptr(), Some(Sp_search), 1);
            jsB_propf(J, c"String.prototype.slice".as_ptr(), Some(Sp_slice), 2);
            jsB_propf(J, c"String.prototype.split".as_ptr(), Some(Sp_split), 2);
            jsB_propf(
                J,
                c"String.prototype.substring".as_ptr(),
                Some(Sp_substring),
                2,
            );
            jsB_propf(
                J,
                c"String.prototype.toLowerCase".as_ptr(),
                Some(Sp_toLowerCase),
                0,
            );
            jsB_propf(
                J,
                c"String.prototype.toLocaleLowerCase".as_ptr(),
                Some(Sp_toLowerCase),
                0,
            );
            jsB_propf(
                J,
                c"String.prototype.toUpperCase".as_ptr(),
                Some(Sp_toUpperCase),
                0,
            );
            jsB_propf(
                J,
                c"String.prototype.toLocaleUpperCase".as_ptr(),
                Some(Sp_toUpperCase),
                0,
            );

            /* ES5 */
            jsB_propf(J, c"String.prototype.trim".as_ptr(), Some(Sp_trim), 0);
        }
        js_newcconstructor(
            J,
            Some(jsB_String),
            Some(jsB_new_String),
            c"String".as_ptr(),
            0,
        ); /* 1 */
        {
            jsB_propf(J, c"String.fromCharCode".as_ptr(), Some(S_fromCharCode), 0); /* 1 */
        }
        js_defglobal(J, c"String".as_ptr(), JS_DONTENUM);
    }
}
