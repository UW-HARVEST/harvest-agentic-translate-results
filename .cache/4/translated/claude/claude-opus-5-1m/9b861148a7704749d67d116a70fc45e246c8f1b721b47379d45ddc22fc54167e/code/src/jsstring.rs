//! Translation of `c_src/src/jsstring.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsarray::{js_getlength, js_setlength};
use crate::jsbuiltin::jsB_propf;
use crate::jsi::*;
use crate::jsintern::{js_putc, js_putm, js_puts};
use crate::jsproperty::*;
use crate::jsregexp::{js_RegExp_prototype_exec, js_newregexp};
use crate::jsrun::*;
use crate::jsvalue::*;
use crate::regexp::{js_regexec, Reprog, Resub, ResubSpan, REG_MAXSUB, REG_NOTBOL};
use crate::utf::{
    chartorune, runelen, runetochar, tolowerrune, tolowerrune_full, toupperrune, toupperrune_full,
};
use core::ptr::{null, null_mut};

/// `<stdio.h>`'s `EOF`
const EOF: c_int = -1;

/// `m.sub[i]` -- raw-pointer indexing so that an out of range index behaves
/// like C instead of panicking.
#[inline]
unsafe fn m_sub(m: *mut Resub, i: c_int) -> *mut ResubSpan {
    (*m).sub.as_mut_ptr().offset(i as isize)
}

/// `a - b` for `const char *` operands
#[inline]
fn pdiff(a: *const c_char, b: *const c_char) -> c_int {
    (a as isize - b as isize) as c_int
}

unsafe fn js_doregexec(
    J: *mut js_State,
    prog: *mut Reprog,
    string: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    let result = js_regexec(prog, string, sub, eflags);
    if result < 0 {
        js_error!(J, c"regexec failed".as_ptr());
    }
    result
}

unsafe fn checkstring(J: *mut js_State, idx: c_int) -> *const c_char {
    if js_iscoercible(J, idx) == 0 {
        js_typeerror!(J, c"string function called on null or undefined".as_ptr());
    }
    js_tostring(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_runeat(
    J: *mut js_State,
    s: *const c_char,
    i: c_int,
) -> c_int {
    let mut s = s;
    let mut i = i;
    let mut rune: Rune = EOF;
    while i >= 0 {
        rune = *(s as *const u8) as Rune;
        if rune < Runeself {
            if rune == 0 {
                return EOF;
            }
            s = s.offset(1);
            i -= 1;
        } else {
            s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_utflen(s: *const c_char) -> c_int {
    let mut s = s;
    let mut c: c_int;
    let mut n: c_int;
    let mut rune: Rune = 0;

    n = 0;
    loop {
        c = *(s as *const u8) as c_int;
        if c < Runeself {
            if c == 0 {
                return n;
            }
            s = s.offset(1);
            n += 1;
        } else {
            s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
            if rune >= 0x10000 {
                n += 2;
            } else {
                n += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_utfptrtoidx(s: *const c_char, p: *const c_char) -> c_int {
    let mut s = s;
    let mut rune: Rune = 0;
    let mut i: c_int = 0;
    while s < p {
        if (*(s as *const u8) as Rune) < Runeself {
            s = s.offset(1);
            i += 1;
        } else {
            s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
            if rune >= 0x10000 {
                i += 2;
            } else {
                i += 1;
            }
        }
    }
    i
}

unsafe extern "C-unwind" fn jsB_new_String(J: *mut js_State) {
    js_newstring(
        J,
        if js_gettop(J) > 1 {
            js_tostring(J, 1)
        } else {
            c"".as_ptr()
        },
    );
}

unsafe extern "C-unwind" fn jsB_String(J: *mut js_State) {
    js_pushstring(
        J,
        if js_gettop(J) > 1 {
            js_tostring(J, 1)
        } else {
            c"".as_ptr()
        },
    );
}

unsafe extern "C-unwind" fn Sp_toString(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        js_typeerror!(J, c"not a string".as_ptr());
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C-unwind" fn Sp_valueOf(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        js_typeerror!(J, c"not a string".as_ptr());
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C-unwind" fn Sp_charAt(J: *mut js_State) {
    let mut buf = [0 as c_char; UTFmax + 1];
    let s = checkstring(J, 0);
    let pos = js_tointeger(J, 1);
    let rune: Rune = js_runeat(J, s, pos);
    if rune >= 0 {
        let n = runetochar(buf.as_mut_ptr(), &rune as *const Rune);
        *buf.as_mut_ptr().offset(n as isize) = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushliteral(J, c"".as_ptr());
    }
}

unsafe extern "C-unwind" fn Sp_charCodeAt(J: *mut js_State) {
    let s = checkstring(J, 0);
    let pos = js_tointeger(J, 1);
    let rune: Rune = js_runeat(J, s, pos);
    if rune >= 0 {
        js_pushnumber(J, rune as f64);
    } else {
        js_pushnumber(J, NAN);
    }
}

unsafe extern "C-unwind" fn Sp_concat(J: *mut js_State) {
    let top = js_gettop(J);
    let n0: c_int;
    let mut out: *mut c_char = null_mut();
    let s0: *const c_char;

    if top == 1 {
        return;
    }

    s0 = checkstring(J, 0);
    n0 = 1 + strlen(s0) as c_int;

    let outp = &mut out as *mut *mut c_char;

    if js_do_try(J, || {
        let mut n = n0;
        let mut s = s0;
        let mut i: c_int;

        if n > JS_STRLIMIT {
            js_rangeerror!(J, c"invalid string length".as_ptr());
        }
        *outp = js_malloc(J, n) as *mut c_char;
        strcpy(*outp, s);

        i = 1;
        while i < top {
            s = js_tostring(J, i);
            n += strlen(s) as c_int;
            if n > JS_STRLIMIT {
                js_rangeerror!(J, c"invalid string length".as_ptr());
            }
            *outp = js_realloc(J, *outp as *mut c_void, n) as *mut c_char;
            strcat(*outp, s);
            i += 1;
        }

        js_pushstring(J, *outp);
        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, out as *mut c_void);
        js_throw(J);
    }
    js_free(J, out as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_indexOf(J: *mut js_State) {
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
        haystack = haystack.offset(chartorune(&mut rune as *mut Rune, haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, -1.0);
}

unsafe extern "C-unwind" fn Sp_lastIndexOf(J: *mut js_State) {
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
        haystack = haystack.offset(chartorune(&mut rune as *mut Rune, haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, last as f64);
}

unsafe extern "C-unwind" fn Sp_localeCompare(J: *mut js_State) {
    let a = checkstring(J, 0);
    let b = js_tostring(J, 1);
    js_pushnumber(J, strcmp(a, b) as f64);
}

unsafe fn Sp_substring_imp(J: *mut js_State, s: *const c_char, a: c_int, n: c_int) {
    let mut head_rune: Rune = 0;
    let mut tail_rune: Rune = 0;
    let mut head: *const c_char;
    let mut tail: *const c_char;
    let mut p: *mut c_char = null_mut();
    let mut i: c_int;
    let mut k: c_int;

    /* find start of substring */
    head = s;
    i = 0;
    while i < a {
        head = head.offset(chartorune(&mut head_rune as *mut Rune, head) as isize);
        if head_rune >= 0x10000 {
            i += 1;
        }
        i += 1;
    }

    /* find end of substring */
    tail = head;
    k = i - a;
    while k < n {
        tail = tail.offset(chartorune(&mut tail_rune as *mut Rune, tail) as isize);
        if tail_rune >= 0x10000 {
            k += 1;
        }
        k += 1;
    }

    /* no surrogate pair splits! */
    if i == a && k == n {
        js_pushlstring(J, head, pdiff(tail, head));
        return;
    }

    let pp = &mut p as *mut *mut c_char;
    let head0 = head;
    let tail0 = tail;
    let head_rune0 = head_rune;
    let tail_rune0 = tail_rune;

    if js_do_try(J, || {
        let head = head0;
        let mut tail = tail0;
        let mut head_rune = head_rune0;
        let mut tail_rune = tail_rune0;
        let head_len: c_int;
        let tail_len: c_int;

        *pp = js_malloc(J, UTFmax as c_int + pdiff(tail, head)) as *mut c_char;

        /* substring starts with low surrogate (head is just after character) */
        if i > a {
            head_rune = 0xdc00 + ((head_rune - 0x10000) & 0x3ff);
            head_len = runetochar(*pp, &head_rune as *const Rune);
            memcpy(
                (*pp).offset(head_len as isize) as *mut c_void,
                head as *const c_void,
                pdiff(tail, head) as size_t,
            );
            js_pushlstring(J, *pp, head_len + pdiff(tail, head));
        }

        /* substring ends with high surrogate (tail is just after character) */
        if k > n {
            tail = tail.offset(-(runelen(tail_rune) as isize));
            memcpy(
                *pp as *mut c_void,
                head as *const c_void,
                pdiff(tail, head) as size_t,
            );
            tail_rune = 0xd800 + ((tail_rune - 0x10000) >> 10);
            tail_len = runetochar(
                (*pp).offset(pdiff(tail, head) as isize),
                &tail_rune as *const Rune,
            );
            js_pushlstring(J, *pp, pdiff(tail, head) + tail_len);
        }

        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, p as *mut c_void);
        js_throw(J);
    }
    js_free(J, p as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_slice(J: *mut js_State) {
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

unsafe extern "C-unwind" fn Sp_substring(J: *mut js_State) {
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

unsafe extern "C-unwind" fn Sp_toLowerCase(J: *mut js_State) {
    let s0 = checkstring(J, 0);
    let mut dst: *mut c_char = null_mut();
    let mut rune: Rune = 0;
    let mut full: *const Rune;
    let mut n: c_int;

    n = 1;
    let mut s = s0;
    while *s != 0 {
        s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
        full = tolowerrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += runelen(*full);
                full = full.offset(1);
            }
        } else {
            rune = tolowerrune(rune);
            n += runelen(rune);
        }
    }

    let n0 = n;
    let dstp = &mut dst as *mut *mut c_char;

    if js_do_try(J, || {
        let mut rune: Rune = 0;
        let mut full: *const Rune;
        let mut d: *mut c_char;

        *dstp = js_malloc(J, n0) as *mut c_char;
        d = *dstp;
        let mut s = s0;
        while *s != 0 {
            s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
            full = tolowerrune_full(rune);
            if !full.is_null() {
                while *full != 0 {
                    d = d.offset(runetochar(d, full) as isize);
                    full = full.offset(1);
                }
            } else {
                rune = tolowerrune(rune);
                d = d.offset(runetochar(d, &rune as *const Rune) as isize);
            }
        }
        *d = 0;

        js_pushstring(J, *dstp);
        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, dst as *mut c_void);
        js_throw(J);
    }
    js_free(J, dst as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_toUpperCase(J: *mut js_State) {
    let s0 = checkstring(J, 0);
    let mut dst: *mut c_char = null_mut();
    let mut full: *const Rune;
    let mut rune: Rune = 0;
    let mut n: c_int;

    n = 1;
    let mut s = s0;
    while *s != 0 {
        s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
        full = toupperrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += runelen(*full);
                full = full.offset(1);
            }
        } else {
            rune = toupperrune(rune);
            n += runelen(rune);
        }
    }

    let n0 = n;
    let dstp = &mut dst as *mut *mut c_char;

    if js_do_try(J, || {
        let mut full: *const Rune;
        let mut rune: Rune = 0;
        let mut d: *mut c_char;

        *dstp = js_malloc(J, n0) as *mut c_char;
        d = *dstp;
        let mut s = s0;
        while *s != 0 {
            s = s.offset(chartorune(&mut rune as *mut Rune, s) as isize);
            full = toupperrune_full(rune);
            if !full.is_null() {
                while *full != 0 {
                    d = d.offset(runetochar(d, full) as isize);
                    full = full.offset(1);
                }
            } else {
                rune = toupperrune(rune);
                d = d.offset(runetochar(d, &rune as *const Rune) as isize);
            }
        }
        *d = 0;

        js_pushstring(J, *dstp);
        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, dst as *mut c_void);
        js_throw(J);
    }
    js_free(J, dst as *mut c_void);
}

unsafe fn isbol(re: *mut js_Regexp, text: *const c_char, a: *const c_char) -> c_int {
    (a == text
        || (((*re).flags as c_int & JS_REGEXP_M) != 0 && *a.offset(-1) == '\n' as c_char))
        as c_int
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
    let mut s: *const c_char;
    let mut e: *const c_char;
    s = checkstring(J, 0);
    while istrim(*s as c_int) != 0 {
        s = s.offset(1);
    }
    e = s.offset(strlen(s) as isize);
    while e > s && istrim(*e.offset(-1) as c_int) != 0 {
        e = e.offset(-1);
    }
    js_pushlstring(J, s, pdiff(e, s));
}

unsafe extern "C-unwind" fn S_fromCharCode(J: *mut js_State) {
    let top = js_gettop(J);
    let mut s: *mut c_char = null_mut();

    let sp = &mut s as *mut *mut c_char;

    if js_do_try(J, || {
        let mut i: c_int;
        let mut p: *mut c_char;
        let mut c: Rune;

        *sp = js_malloc(J, (top - 1) * UTFmax as c_int + 1) as *mut c_char;
        p = *sp;

        i = 1;
        while i < top {
            c = js_touint32(J, i) as Rune;
            p = p.offset(runetochar(p, &c as *const Rune) as isize);
            i += 1;
        }
        *p = 0;

        js_pushstring(J, *sp);
        js_endtry(J);
    })
    .is_none()
    {
        js_free(J, s as *mut c_void);
        js_throw(J);
    }
    js_free(J, s as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_match(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text: *const c_char;
    let mut len: c_int;
    let mut a: *const c_char;
    let mut b: *const c_char;
    let mut c: *const c_char;
    let e: *const c_char;
    let mut m: Resub = core::mem::zeroed();
    let mut rune: Rune = 0;

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        js_newregexp(J, c"".as_ptr(), 0);
    } else {
        js_newregexp(J, js_tostring(J, 1), 0);
    }

    re = js_toregexp(J, -1);
    if ((*re).flags as c_int & JS_REGEXP_G) == 0 {
        js_RegExp_prototype_exec(J, re, text);
        return;
    }

    (*re).last = 0;

    js_newarray(J);

    len = 0;
    a = text;
    e = text.offset(strlen(text) as isize);
    while a <= e {
        if js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            a,
            &mut m as *mut Resub,
            if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL },
        ) != 0
        {
            break;
        }

        b = m.sub[0].sp;
        c = m.sub[0].ep;

        js_pushlstring(J, b, pdiff(c, b));
        let l = len;
        len += 1;
        js_setindex(J, -2, l);

        a = c;
        if pdiff(c, b) == 0 {
            a = a.offset(chartorune(&mut rune as *mut Rune, a) as isize);
        }
    }

    if len == 0 {
        js_pop(J, 1);
        js_pushnull(J);
    }
}

unsafe extern "C-unwind" fn Sp_search(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text: *const c_char;
    let mut m: Resub = core::mem::zeroed();

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        js_newregexp(J, c"".as_ptr(), 0);
    } else {
        js_newregexp(J, js_tostring(J, 1), 0);
    }

    re = js_toregexp(J, -1);

    if js_doregexec(
        J,
        (*re).prog as *mut Reprog,
        text,
        &mut m as *mut Resub,
        0,
    ) == 0
    {
        js_pushnumber(J, js_utfptrtoidx(text, m.sub[0].sp) as f64);
    } else {
        js_pushnumber(J, -1.0);
    }
}

unsafe fn Sp_replace_regexp(J: *mut js_State) {
    let re: *mut js_Regexp;
    let source0: *const c_char;
    let source_init: *const c_char;
    let mut sb: *mut js_Buffer = null_mut();
    let mut m: Resub = core::mem::zeroed();

    source0 = checkstring(J, 0);
    source_init = source0;
    re = js_toregexp(J, 1);

    let mp = &mut m as *mut Resub;

    if js_doregexec(
        J,
        (*re).prog as *mut Reprog,
        source_init,
        mp,
        0,
    ) != 0
    {
        js_copy(J, 0);
        return;
    }

    (*re).last = 0;

    let sbp = &mut sb as *mut *mut js_Buffer;

    if js_do_try(J, || {
        let mut source = source_init;
        let mut s: *const c_char = null();
        let mut r: *const c_char;
        let mut n: c_int = 0;
        let mut x: c_int;

        /* loop: */
        loop {
            s = (*m_sub(mp, 0)).sp;
            n = pdiff((*m_sub(mp, 0)).ep, (*m_sub(mp, 0)).sp);

            if js_iscallable(J, 2) != 0 {
                js_copy(J, 2);
                js_pushundefined(J);
                /* arg 0..x: substring and subexps that matched */
                x = 0;
                while !(*m_sub(mp, x)).sp.is_null() {
                    js_pushlstring(
                        J,
                        (*m_sub(mp, x)).sp,
                        pdiff((*m_sub(mp, x)).ep, (*m_sub(mp, x)).sp),
                    );
                    x += 1;
                }
                js_pushnumber(J, pdiff(s, source) as f64); /* arg x+2: offset within search string */
                js_copy(J, 0); /* arg x+3: search string */
                js_call(J, 2 + x);
                r = js_tostring(J, -1);
                js_putm(J, sbp, source, s);
                js_puts(J, sbp, r);
                js_pop(J, 1);
            } else {
                r = js_tostring(J, 2);
                js_putm(J, sbp, source, s);
                while *r != 0 {
                    if *r == '$' as c_char {
                        r = r.offset(1);
                        let ch = *r;
                        match ch as u8 {
                            0 => {
                                r = r.offset(-1); /* end of string; back up */
                                /* fallthrough */
                                js_putc(J, sbp, '$' as c_int);
                            }
                            b'$' => {
                                js_putc(J, sbp, '$' as c_int);
                            }
                            b'`' => {
                                js_putm(J, sbp, source0, s);
                            }
                            b'\'' => {
                                js_puts(J, sbp, s.offset(n as isize));
                            }
                            b'&' => {
                                js_putm(J, sbp, s, s.offset(n as isize));
                            }
                            b'0'..=b'9' => {
                                x = *r as c_int - '0' as c_int;
                                if *r.offset(1) >= '0' as c_char
                                    && *r.offset(1) <= '9' as c_char
                                {
                                    r = r.offset(1);
                                    x = x * 10 + *r as c_int - '0' as c_int;
                                }
                                if x > 0 && x < (*mp).nsub {
                                    js_putm(J, sbp, (*m_sub(mp, x)).sp, (*m_sub(mp, x)).ep);
                                } else {
                                    js_putc(J, sbp, '$' as c_int);
                                    if x > 10 {
                                        js_putc(J, sbp, '0' as c_int + x / 10);
                                        js_putc(J, sbp, '0' as c_int + x % 10);
                                    } else {
                                        js_putc(J, sbp, '0' as c_int + x);
                                    }
                                }
                            }
                            _ => {
                                js_putc(J, sbp, '$' as c_int);
                                js_putc(J, sbp, *r as c_int);
                            }
                        }
                        r = r.offset(1);
                    } else {
                        let c0 = *r;
                        r = r.offset(1);
                        js_putc(J, sbp, c0 as c_int);
                    }
                }
            }

            if ((*re).flags as c_int & JS_REGEXP_G) != 0 {
                source = (*m_sub(mp, 0)).ep;
                if n == 0 {
                    if *source != 0 {
                        let c0 = *source;
                        source = source.offset(1);
                        js_putc(J, sbp, c0 as c_int);
                    } else {
                        break; /* goto end */
                    }
                }
                if js_doregexec(
                    J,
                    (*re).prog as *mut Reprog,
                    source,
                    mp,
                    if isbol(re, source0, source) != 0 {
                        0
                    } else {
                        REG_NOTBOL
                    },
                ) == 0
                {
                    continue; /* goto loop */
                }
            }
            break;
        }

        /* end: */
        js_puts(J, sbp, s.offset(n as isize));
        js_putc(J, sbp, 0);

        js_pushstring(
            J,
            if !(*sbp).is_null() {
                (*(*sbp)).s.as_ptr()
            } else {
                c"".as_ptr()
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

unsafe fn Sp_replace_string(J: *mut js_State) {
    let source: *const c_char;
    let needle: *const c_char;
    let s: *const c_char;
    let mut sb: *mut js_Buffer = null_mut();
    let n: c_int;

    source = checkstring(J, 0);
    needle = js_tostring(J, 1);

    s = strstr(source, needle) as *const c_char;
    if s.is_null() {
        js_copy(J, 0);
        return;
    }
    n = strlen(needle) as c_int;

    let sbp = &mut sb as *mut *mut js_Buffer;

    if js_do_try(J, || {
        let mut r: *const c_char;

        if js_iscallable(J, 2) != 0 {
            js_copy(J, 2);
            js_pushundefined(J);
            js_pushlstring(J, s, n); /* arg 1: substring that matched */
            js_pushnumber(J, pdiff(s, source) as f64); /* arg 2: offset within search string */
            js_copy(J, 0); /* arg 3: search string */
            js_call(J, 3);
            r = js_tostring(J, -1);
            js_putm(J, sbp, source, s);
            js_puts(J, sbp, r);
            js_puts(J, sbp, s.offset(n as isize));
            js_putc(J, sbp, 0);
            js_pop(J, 1);
        } else {
            r = js_tostring(J, 2);
            js_putm(J, sbp, source, s);
            while *r != 0 {
                if *r == '$' as c_char {
                    r = r.offset(1);
                    let ch = *r;
                    match ch as u8 {
                        0 => {
                            r = r.offset(-1); /* end of string; back up */
                            /* fallthrough */
                            js_putc(J, sbp, '$' as c_int);
                        }
                        b'$' => {
                            js_putc(J, sbp, '$' as c_int);
                        }
                        b'&' => {
                            js_putm(J, sbp, s, s.offset(n as isize));
                        }
                        b'`' => {
                            js_putm(J, sbp, source, s);
                        }
                        b'\'' => {
                            js_puts(J, sbp, s.offset(n as isize));
                        }
                        _ => {
                            js_putc(J, sbp, '$' as c_int);
                            js_putc(J, sbp, *r as c_int);
                        }
                    }
                    r = r.offset(1);
                } else {
                    let c0 = *r;
                    r = r.offset(1);
                    js_putc(J, sbp, c0 as c_int);
                }
            }
            js_puts(J, sbp, s.offset(n as isize));
            js_putc(J, sbp, 0);
        }

        js_pushstring(
            J,
            if !(*sbp).is_null() {
                (*(*sbp)).s.as_ptr()
            } else {
                c"".as_ptr()
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

unsafe extern "C-unwind" fn Sp_replace(J: *mut js_State) {
    if js_isregexp(J, 1) != 0 {
        Sp_replace_regexp(J);
    } else {
        Sp_replace_string(J);
    }
}

unsafe fn Sp_split_regexp(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text: *const c_char;
    let limit: c_int;
    let mut len: c_int;
    let mut k: c_int;
    let mut p: *const c_char;
    let mut a: *const c_char;
    let mut b: *const c_char;
    let mut c: *const c_char;
    let e: *const c_char;
    let mut m: Resub = core::mem::zeroed();
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

    e = text.offset(strlen(text) as isize);

    /* splitting the empty string */
    if e == text {
        if js_doregexec(J, (*re).prog as *mut Reprog, text, &mut m as *mut Resub, 0) != 0 {
            js_pushliteral(J, c"".as_ptr());
            js_setindex(J, -2, 0);
        }
        return;
    }

    a = text;
    p = a;
    while a < e {
        if js_doregexec(
            J,
            (*re).prog as *mut Reprog,
            a,
            &mut m as *mut Resub,
            if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL },
        ) != 0
        {
            break; /* no match */
        }

        b = m.sub[0].sp;
        c = m.sub[0].ep;

        /* empty string at end of last match */
        if b == c && b == p {
            a = a.offset(chartorune(&mut rune as *mut Rune, a) as isize);
            continue;
        }

        if len == limit {
            return;
        }
        js_pushlstring(J, p, pdiff(b, p));
        let l = len;
        len += 1;
        js_setindex(J, -2, l);

        k = 1;
        while k < m.nsub {
            if len == limit {
                return;
            }
            js_pushlstring(
                J,
                (*m_sub(&mut m as *mut Resub, k)).sp,
                pdiff(
                    (*m_sub(&mut m as *mut Resub, k)).ep,
                    (*m_sub(&mut m as *mut Resub, k)).sp,
                ),
            );
            let l = len;
            len += 1;
            js_setindex(J, -2, l);
            k += 1;
        }

        p = c;
        a = p;
    }

    if len == limit {
        return;
    }
    js_pushstring(J, p);
    js_setindex(J, -2, len);
}

unsafe fn Sp_split_string(J: *mut js_State) {
    let mut str = checkstring(J, 0);
    let sep = js_tostring(J, 1);
    let limit = if js_isdefined(J, 2) != 0 {
        js_tointeger(J, 2)
    } else {
        1 << 30
    };
    let mut i: c_int;
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
            n = chartorune(&mut rune as *mut Rune, str);
            js_pushlstring(J, str, n);
            js_setindex(J, -2, i);
            str = str.offset(n as isize);
            i += 1;
        }
        return;
    }

    i = 0;
    while !str.is_null() && i < limit {
        let s = strstr(str, sep) as *const c_char;
        if !s.is_null() {
            js_pushlstring(J, str, pdiff(s, str));
            js_setindex(J, -2, i);
            str = s.offset(n as isize);
        } else {
            js_pushstring(J, str);
            js_setindex(J, -2, i);
            str = null();
        }
        i += 1;
    }
}

unsafe extern "C-unwind" fn Sp_split(J: *mut js_State) {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsB_initstring(J: *mut js_State) {
    (*(*J).String_prototype).u.s.shrstr[0] = 0;
    (*(*J).String_prototype).u.s.string = (*(*J).String_prototype).u.s.shrstr.as_mut_ptr();
    (*(*J).String_prototype).u.s.length = 0;

    js_pushobject(J, (*J).String_prototype);
    {
        jsB_propf(
            J,
            c"String.prototype.toString".as_ptr(),
            Some(Sp_toString),
            0,
        );
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
