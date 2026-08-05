//! Translated from jsstring.c — String constructor and prototype methods.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsintern::{js_putc, js_putm, js_puts};
use crate::jsrun::*;
use crate::regexp::{Reprog, Resub, REG_NOTBOL};
use crate::types::*;
use crate::utf::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

const UTFmax_usize: usize = 4;

unsafe fn js_doregexec(J: *mut js_State, prog: *mut Reprog, string: *const c_char, sub: *mut Resub, eflags: c_int) -> c_int {
    let result = crate::regexp::js_regexec(prog, string, sub, eflags);
    if result < 0 {
        crate::jserror::js_error(J, cstr!("regexec failed"));
    }
    result
}

unsafe fn checkstring(J: *mut js_State, idx: c_int) -> *const c_char {
    if js_iscoercible(J, idx) == 0 {
        crate::jserror::js_typeerror(J, cstr!("string function called on null or undefined"));
    }
    js_tostring(J, idx)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_runeat(J: *mut js_State, s: *const c_char, mut i: c_int) -> c_int {
    let mut rune: Rune = EOF;
    let mut s = s;
    while i >= 0 {
        rune = *(s as *const u8) as Rune;
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
        if i == -2 {
            return 0xd800 + ((rune - 0x10000) >> 10);
        } else {
            return 0xdc00 + ((rune - 0x10000) & 0x3ff);
        }
    }
    rune
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_utflen(s: *const c_char) -> c_int {
    let mut c;
    let mut n;
    let mut rune: Rune = 0;
    let mut s = s;
    n = 0;
    loop {
        c = *(s as *const u8) as c_int;
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

#[no_mangle]
pub unsafe extern "C-unwind" fn js_utfptrtoidx(s: *const c_char, p: *const c_char) -> c_int {
    let mut rune: Rune = 0;
    let mut i = 0;
    let mut s = s;
    while s < p {
        if (*(s as *const u8) as c_int) < Runeself {
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

unsafe extern "C-unwind" fn jsB_new_String(J: *mut js_State) {
    crate::jsvalue::js_newstring(J, if js_gettop(J) > 1 { js_tostring(J, 1) } else { cstr!("") });
}

unsafe extern "C-unwind" fn jsB_String(J: *mut js_State) {
    js_pushstring(J, if js_gettop(J) > 1 { js_tostring(J, 1) } else { cstr!("") });
}

unsafe extern "C-unwind" fn Sp_toString(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        crate::jserror::js_typeerror(J, cstr!("not a string"));
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C-unwind" fn Sp_valueOf(J: *mut js_State) {
    let self_ = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        crate::jserror::js_typeerror(J, cstr!("not a string"));
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C-unwind" fn Sp_charAt(J: *mut js_State) {
    let mut buf: [c_char; UTFmax_usize + 1] = [0; UTFmax_usize + 1];
    let s = checkstring(J, 0);
    let pos = js_tointeger(J, 1);
    let mut rune = js_runeat(J, s, pos);
    if rune >= 0 {
        let n = runetochar(buf.as_mut_ptr(), &mut rune);
        buf[n as usize] = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushliteral(J, cstr!(""));
    }
}

unsafe extern "C-unwind" fn Sp_charCodeAt(J: *mut js_State) {
    let s = checkstring(J, 0);
    let pos = js_tointeger(J, 1);
    let rune = js_runeat(J, s, pos);
    if rune >= 0 {
        js_pushnumber(J, rune as f64);
    } else {
        js_pushnumber(J, f64::NAN);
    }
}

unsafe extern "C-unwind" fn Sp_concat(J: *mut js_State) {
    let mut i = 0;
    let top = js_gettop(J);
    let mut n;
    let mut out: *mut c_char = std::ptr::null_mut();
    let mut s;

    if top == 1 {
        return;
    }

    s = checkstring(J, 0);
    n = 1 + strlen(s) as c_int;

    let out_ptr = std::ptr::addr_of_mut!(out);
    let caught = protect(J, || {
        if n > JS_STRLIMIT {
            crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
        }
        *out_ptr = js_malloc(J, n) as *mut c_char;
        strcpy(*out_ptr, s);

        i = 1;
        while i < top {
            s = js_tostring(J, i);
            n += strlen(s) as c_int;
            if n > JS_STRLIMIT {
                crate::jserror::js_rangeerror(J, cstr!("invalid string length"));
            }
            *out_ptr = js_realloc(J, *out_ptr as *mut c_void, n) as *mut c_char;
            strcat(*out_ptr, s);
            i += 1;
        }

        js_pushstring(J, *out_ptr);
    });
    if caught {
        js_free(J, out as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, out as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_indexOf(J: *mut js_State) {
    let mut haystack = checkstring(J, 0);
    let needle = js_tostring(J, 1);
    let pos = js_tointeger(J, 2);
    let len = strlen(needle) as c_int;
    let mut k = 0;
    let mut rune: Rune = 0;
    while *haystack != 0 {
        if k >= pos && strncmp(haystack, needle, len as usize) == 0 {
            js_pushnumber(J, k as f64);
            return;
        }
        haystack = haystack.add(chartorune(&mut rune, haystack) as usize);
        k += 1;
    }
    js_pushnumber(J, -1.0);
}

unsafe extern "C-unwind" fn Sp_lastIndexOf(J: *mut js_State) {
    let mut haystack = checkstring(J, 0);
    let needle = js_tostring(J, 1);
    let pos = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { strlen(haystack) as c_int };
    let len = strlen(needle) as c_int;
    let mut k = 0;
    let mut last = -1;
    let mut rune: Rune = 0;
    while *haystack != 0 && k <= pos {
        if strncmp(haystack, needle, len as usize) == 0 {
            last = k;
        }
        haystack = haystack.add(chartorune(&mut rune, haystack) as usize);
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
    let mut head;
    let mut tail;
    let mut p: *mut c_char = std::ptr::null_mut();
    let mut i;
    let mut k;
    let head_len: c_int = 0;
    let tail_len: c_int = 0;

    head = s;
    i = 0;
    while i < a {
        head = head.add(chartorune(&mut head_rune, head) as usize);
        if head_rune >= 0x10000 {
            i += 1;
        }
        i += 1;
    }

    tail = head;
    k = i - a;
    while k < n {
        tail = tail.add(chartorune(&mut tail_rune, tail) as usize);
        if tail_rune >= 0x10000 {
            k += 1;
        }
        k += 1;
    }

    if i == a && k == n {
        js_pushlstring(J, head, (tail as isize - head as isize) as c_int);
        return;
    }

    let p_ptr = std::ptr::addr_of_mut!(p);
    let mut head_v = head;
    let mut tail_v = tail;
    let caught = protect(J, || {
        *p_ptr = js_malloc(J, UTFmax + (tail_v as isize - head_v as isize) as c_int) as *mut c_char;

        if i > a {
            let mut hr = 0xdc00 + ((head_rune - 0x10000) & 0x3ff);
            let hl = runetochar(*p_ptr, &mut hr);
            let _ = head_len;
            memcpy((*p_ptr).add(hl as usize), head_v, (tail_v as isize - head_v as isize) as usize);
            js_pushlstring(J, *p_ptr, hl + (tail_v as isize - head_v as isize) as c_int);
        }

        if k > n {
            tail_v = tail_v.offset(-(runelen(tail_rune) as isize));
            memcpy(*p_ptr, head_v, (tail_v as isize - head_v as isize) as usize);
            let mut tr = 0xd800 + ((tail_rune - 0x10000) >> 10);
            let tl = runetochar((*p_ptr).add((tail_v as isize - head_v as isize) as usize), &mut tr);
            let _ = tail_len;
            js_pushlstring(J, *p_ptr, (tail_v as isize - head_v as isize) as c_int + tl);
        }
    });
    let _ = (&mut head_v, &mut tail_v);
    if caught {
        js_free(J, p as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, p as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_slice(J: *mut js_State) {
    let str = checkstring(J, 0);
    let len = js_utflen(str);
    let mut s = js_tointeger(J, 1);
    let mut e = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { len };

    s = if s < 0 { s + len } else { s };
    e = if e < 0 { e + len } else { e };

    s = if s < 0 { 0 } else if s > len { len } else { s };
    e = if e < 0 { 0 } else if e > len { len } else { e };

    if s < e {
        Sp_substring_imp(J, str, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str, e, s - e);
    } else {
        js_pushliteral(J, cstr!(""));
    }
}

unsafe extern "C-unwind" fn Sp_substring(J: *mut js_State) {
    let str = checkstring(J, 0);
    let len = js_utflen(str);
    let mut s = js_tointeger(J, 1);
    let mut e = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { len };

    s = if s < 0 { 0 } else if s > len { len } else { s };
    e = if e < 0 { 0 } else if e > len { len } else { e };

    if s < e {
        Sp_substring_imp(J, str, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str, e, s - e);
    } else {
        js_pushliteral(J, cstr!(""));
    }
}

unsafe extern "C-unwind" fn Sp_toLowerCase(J: *mut js_State) {
    let s0 = checkstring(J, 0);
    let mut s;
    let mut dst: *mut c_char = std::ptr::null_mut();
    let mut d: *mut c_char = std::ptr::null_mut();
    let mut rune: Rune = 0;
    let mut full: *const Rune = std::ptr::null();
    let mut n;

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

    let dst_ptr = std::ptr::addr_of_mut!(dst);
    let caught = protect(J, || {
        *dst_ptr = js_malloc(J, n) as *mut c_char;
        d = *dst_ptr;
        s = s0;
        while *s != 0 {
            s = s.add(chartorune(&mut rune, s) as usize);
            full = tolowerrune_full(rune);
            if !full.is_null() {
                let mut f = full;
                while *f != 0 {
                    d = d.add(runetochar(d, f as *mut Rune) as usize);
                    f = f.add(1);
                }
            } else {
                rune = tolowerrune(rune);
                d = d.add(runetochar(d, &mut rune) as usize);
            }
        }
        *d = 0;
        js_pushstring(J, *dst_ptr);
    });
    if caught {
        js_free(J, dst as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, dst as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_toUpperCase(J: *mut js_State) {
    let s0 = checkstring(J, 0);
    let mut s;
    let mut dst: *mut c_char = std::ptr::null_mut();
    let mut d: *mut c_char = std::ptr::null_mut();
    let mut full: *const Rune = std::ptr::null();
    let mut rune: Rune = 0;
    let mut n;

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

    let dst_ptr = std::ptr::addr_of_mut!(dst);
    let caught = protect(J, || {
        *dst_ptr = js_malloc(J, n) as *mut c_char;
        d = *dst_ptr;
        s = s0;
        while *s != 0 {
            s = s.add(chartorune(&mut rune, s) as usize);
            full = toupperrune_full(rune);
            if !full.is_null() {
                let mut f = full;
                while *f != 0 {
                    d = d.add(runetochar(d, f as *mut Rune) as usize);
                    f = f.add(1);
                }
            } else {
                rune = toupperrune(rune);
                d = d.add(runetochar(d, &mut rune) as usize);
            }
        }
        *d = 0;
        js_pushstring(J, *dst_ptr);
    });
    if caught {
        js_free(J, dst as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, dst as *mut c_void);
}

unsafe fn isbol(re: *mut js_Regexp, text: *const c_char, a: *const c_char) -> c_int {
    (a == text || ((*re).flags as c_int & JS_REGEXP_M != 0 && *a.offset(-1) == '\n' as c_char)) as c_int
}

unsafe fn istrim(c: c_int) -> c_int {
    (c == 0x9 || c == 0xB || c == 0xC || c == 0x20 || c == 0xA0 || c == 0xFEFF || c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

unsafe extern "C-unwind" fn Sp_trim(J: *mut js_State) {
    let mut s;
    let mut e;
    s = checkstring(J, 0);
    while istrim(*s as c_int) != 0 {
        s = s.add(1);
    }
    e = s.add(strlen(s));
    while e > s && istrim(*e.offset(-1) as c_int) != 0 {
        e = e.offset(-1);
    }
    js_pushlstring(J, s, (e as isize - s as isize) as c_int);
}

unsafe extern "C-unwind" fn S_fromCharCode(J: *mut js_State) {
    let mut i = 0;
    let top = js_gettop(J);
    let mut s: *mut c_char = std::ptr::null_mut();
    let mut p: *mut c_char = std::ptr::null_mut();
    let mut c: Rune = 0;

    let s_ptr = std::ptr::addr_of_mut!(s);
    let caught = protect(J, || {
        *s_ptr = js_malloc(J, (top - 1) * UTFmax + 1) as *mut c_char;
        p = *s_ptr;
        i = 1;
        while i < top {
            c = js_touint32(J, i) as Rune;
            p = p.add(runetochar(p, &mut c) as usize);
            i += 1;
        }
        *p = 0;
        js_pushstring(J, *s_ptr);
    });
    if caught {
        js_free(J, s as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, s as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_match(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text;
    let mut len;
    let mut a;
    let mut b;
    let mut c;
    let e;
    let mut m: Resub = std::mem::zeroed();
    let mut rune: Rune = 0;

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        crate::jsregexp::js_newregexp(J, cstr!(""), 0);
    } else {
        crate::jsregexp::js_newregexp(J, js_tostring(J, 1), 0);
    }

    re = js_toregexp(J, -1);
    if (*re).flags as c_int & JS_REGEXP_G == 0 {
        crate::jsregexp::js_RegExp_prototype_exec(J, re, text);
        return;
    }

    (*re).last = 0;

    crate::jsvalue::js_newarray(J);

    len = 0;
    a = text;
    e = text.add(strlen(text));
    while a <= e {
        if js_doregexec(J, (*re).prog as *mut Reprog, a, &mut m, if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL }) != 0 {
            break;
        }

        b = m.sub[0].sp;
        c = m.sub[0].ep;

        js_pushlstring(J, b, (c as isize - b as isize) as c_int);
        js_setindex(J, -2, len);
        len += 1;

        a = c;
        if (c as isize - b as isize) == 0 {
            a = a.add(chartorune(&mut rune, a) as usize);
        }
    }

    if len == 0 {
        js_pop(J, 1);
        js_pushnull(J);
    }
}

unsafe extern "C-unwind" fn Sp_search(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text;
    let mut m: Resub = std::mem::zeroed();

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        crate::jsregexp::js_newregexp(J, cstr!(""), 0);
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

unsafe extern "C-unwind" fn Sp_replace_regexp(J: *mut js_State) {
    let re: *mut js_Regexp;
    let mut source;
    let source0;
    let mut s: *const c_char = std::ptr::null();
    let mut r: *const c_char = std::ptr::null();
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    let mut n: c_int = 0;
    let mut x: c_int = 0;
    let mut m: Resub = std::mem::zeroed();

    source0 = checkstring(J, 0);
    source = source0;
    re = js_toregexp(J, 1);

    if js_doregexec(J, (*re).prog as *mut Reprog, source, &mut m, 0) != 0 {
        js_copy(J, 0);
        return;
    }

    (*re).last = 0;

    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let m_ptr = std::ptr::addr_of_mut!(m);
    let caught = protect(J, || {
        'loop_: loop {
            s = (*m_ptr).sub[0].sp;
            n = ((*m_ptr).sub[0].ep as isize - (*m_ptr).sub[0].sp as isize) as c_int;

            if js_iscallable(J, 2) != 0 {
                js_copy(J, 2);
                js_pushundefined(J);
                x = 0;
                while !(*m_ptr).sub[x as usize].sp.is_null() {
                    js_pushlstring(J, (*m_ptr).sub[x as usize].sp, ((*m_ptr).sub[x as usize].ep as isize - (*m_ptr).sub[x as usize].sp as isize) as c_int);
                    x += 1;
                }
                js_pushnumber(J, (s as isize - source as isize) as f64);
                js_copy(J, 0);
                js_call(J, 2 + x);
                r = js_tostring(J, -1);
                js_putm(J, sb_ptr, source, s);
                js_puts(J, sb_ptr, r);
                js_pop(J, 1);
            } else {
                r = js_tostring(J, 2);
                js_putm(J, sb_ptr, source, s);
                while *r != 0 {
                    if *r == '$' as c_char {
                        r = r.add(1);
                        match *r as u8 as char {
                            '\0' => {
                                r = r.offset(-1);
                                js_putc(J, sb_ptr, '$' as c_int);
                            }
                            '$' => {
                                js_putc(J, sb_ptr, '$' as c_int);
                            }
                            '`' => {
                                js_putm(J, sb_ptr, source0, s);
                            }
                            '\'' => {
                                js_puts(J, sb_ptr, s.add(n as usize));
                            }
                            '&' => {
                                js_putm(J, sb_ptr, s, s.add(n as usize));
                            }
                            '0'..='9' => {
                                x = *r as c_int - '0' as c_int;
                                if *r.add(1) >= '0' as c_char && *r.add(1) <= '9' as c_char {
                                    r = r.add(1);
                                    x = x * 10 + *r as c_int - '0' as c_int;
                                }
                                if x > 0 && x < (*m_ptr).nsub {
                                    js_putm(J, sb_ptr, (*m_ptr).sub[x as usize].sp, (*m_ptr).sub[x as usize].ep);
                                } else {
                                    js_putc(J, sb_ptr, '$' as c_int);
                                    if x > 10 {
                                        js_putc(J, sb_ptr, '0' as c_int + x / 10);
                                        js_putc(J, sb_ptr, '0' as c_int + x % 10);
                                    } else {
                                        js_putc(J, sb_ptr, '0' as c_int + x);
                                    }
                                }
                            }
                            _ => {
                                js_putc(J, sb_ptr, '$' as c_int);
                                js_putc(J, sb_ptr, *r as c_int);
                            }
                        }
                        r = r.add(1);
                    } else {
                        js_putc(J, sb_ptr, *r as c_int);
                        r = r.add(1);
                    }
                }
            }

            if (*re).flags as c_int & JS_REGEXP_G != 0 {
                source = (*m_ptr).sub[0].ep;
                if n == 0 {
                    if *source != 0 {
                        js_putc(J, sb_ptr, *source as c_int);
                        source = source.add(1);
                    } else {
                        break 'loop_;
                    }
                }
                if js_doregexec(J, (*re).prog as *mut Reprog, source, m_ptr, if isbol(re, source0, source) != 0 { 0 } else { REG_NOTBOL }) == 0 {
                    continue 'loop_;
                }
            }
            break 'loop_;
        }

        js_puts(J, sb_ptr, s.add(n as usize));
        js_putc(J, sb_ptr, 0);
        js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("") });
    });
    if caught {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_replace_string(J: *mut js_State) {
    let source;
    let needle;
    let s;
    let mut r: *const c_char = std::ptr::null();
    let mut sb: *mut js_Buffer = std::ptr::null_mut();
    let n;

    source = checkstring(J, 0);
    needle = js_tostring(J, 1);

    s = strstr(source, needle);
    if s.is_null() {
        js_copy(J, 0);
        return;
    }
    n = strlen(needle) as c_int;

    let sb_ptr = std::ptr::addr_of_mut!(sb);
    let caught = protect(J, || {
        if js_iscallable(J, 2) != 0 {
            js_copy(J, 2);
            js_pushundefined(J);
            js_pushlstring(J, s, n);
            js_pushnumber(J, (s as isize - source as isize) as f64);
            js_copy(J, 0);
            js_call(J, 3);
            r = js_tostring(J, -1);
            js_putm(J, sb_ptr, source, s);
            js_puts(J, sb_ptr, r);
            js_puts(J, sb_ptr, s.add(n as usize));
            js_putc(J, sb_ptr, 0);
            js_pop(J, 1);
        } else {
            r = js_tostring(J, 2);
            js_putm(J, sb_ptr, source, s);
            while *r != 0 {
                if *r == '$' as c_char {
                    r = r.add(1);
                    match *r as u8 as char {
                        '\0' => {
                            r = r.offset(-1);
                            js_putc(J, sb_ptr, '$' as c_int);
                        }
                        '$' => js_putc(J, sb_ptr, '$' as c_int),
                        '&' => js_putm(J, sb_ptr, s, s.add(n as usize)),
                        '`' => js_putm(J, sb_ptr, source, s),
                        '\'' => js_puts(J, sb_ptr, s.add(n as usize)),
                        _ => {
                            js_putc(J, sb_ptr, '$' as c_int);
                            js_putc(J, sb_ptr, *r as c_int);
                        }
                    }
                    r = r.add(1);
                } else {
                    js_putc(J, sb_ptr, *r as c_int);
                    r = r.add(1);
                }
            }
            js_puts(J, sb_ptr, s.add(n as usize));
            js_putc(J, sb_ptr, 0);
        }
        js_pushstring(J, if !sb.is_null() { (*sb).s.as_ptr() } else { cstr!("") });
    });
    if caught {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

unsafe extern "C-unwind" fn Sp_replace(J: *mut js_State) {
    if js_isregexp(J, 1) != 0 {
        Sp_replace_regexp(J);
    } else {
        Sp_replace_string(J);
    }
}

unsafe extern "C-unwind" fn Sp_split_regexp(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text;
    let limit;
    let mut len;
    let mut k;
    let mut p;
    let mut a;
    let mut b;
    let mut c;
    let e;
    let mut m: Resub = std::mem::zeroed();
    let mut rune: Rune = 0;

    text = checkstring(J, 0);
    re = js_toregexp(J, 1);
    limit = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { 1 << 30 };

    crate::jsvalue::js_newarray(J);
    len = 0;

    if limit == 0 {
        return;
    }

    e = text.add(strlen(text));

    if e == text {
        if js_doregexec(J, (*re).prog as *mut Reprog, text, &mut m, 0) != 0 {
            js_pushliteral(J, cstr!(""));
            js_setindex(J, -2, 0);
        }
        return;
    }

    a = text;
    p = text;
    while a < e {
        if js_doregexec(J, (*re).prog as *mut Reprog, a, &mut m, if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL }) != 0 {
            break;
        }

        b = m.sub[0].sp;
        c = m.sub[0].ep;

        if b == c && b == p {
            a = a.add(chartorune(&mut rune, a) as usize);
            continue;
        }

        if len == limit {
            return;
        }
        js_pushlstring(J, p, (b as isize - p as isize) as c_int);
        js_setindex(J, -2, len);
        len += 1;

        k = 1;
        while k < m.nsub {
            if len == limit {
                return;
            }
            js_pushlstring(J, m.sub[k as usize].sp, (m.sub[k as usize].ep as isize - m.sub[k as usize].sp as isize) as c_int);
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

unsafe extern "C-unwind" fn Sp_split_string(J: *mut js_State) {
    let mut str = checkstring(J, 0);
    let sep = js_tostring(J, 1);
    let limit = if js_isdefined(J, 2) != 0 { js_tointeger(J, 2) } else { 1 << 30 };
    let mut i;
    let mut n;

    crate::jsvalue::js_newarray(J);

    if limit == 0 {
        return;
    }

    n = strlen(sep) as c_int;

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
            js_pushlstring(J, str, (s as isize - str as isize) as c_int);
            js_setindex(J, -2, i);
            str = s.add(n as usize);
        } else {
            js_pushstring(J, str);
            js_setindex(J, -2, i);
            str = std::ptr::null();
        }
        i += 1;
    }
}

unsafe extern "C-unwind" fn Sp_split(J: *mut js_State) {
    if js_isundefined(J, 1) != 0 {
        crate::jsvalue::js_newarray(J);
        js_pushstring(J, js_tostring(J, 0));
        js_setindex(J, -2, 0);
    } else if js_isregexp(J, 1) != 0 {
        Sp_split_regexp(J);
    } else {
        Sp_split_string(J);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsB_initstring(J: *mut js_State) {
    (*(*J).String_prototype).u.s.shrstr[0] = 0;
    (*(*J).String_prototype).u.s.string = (*(*J).String_prototype).u.s.shrstr.as_mut_ptr();
    (*(*J).String_prototype).u.s.length = 0;

    js_pushobject(J, (*J).String_prototype);
    {
        let pf = crate::jsbuiltin::jsB_propf;
        pf(J, cstr!("String.prototype.toString"), Some(Sp_toString), 0);
        pf(J, cstr!("String.prototype.valueOf"), Some(Sp_valueOf), 0);
        pf(J, cstr!("String.prototype.charAt"), Some(Sp_charAt), 1);
        pf(J, cstr!("String.prototype.charCodeAt"), Some(Sp_charCodeAt), 1);
        pf(J, cstr!("String.prototype.concat"), Some(Sp_concat), 0);
        pf(J, cstr!("String.prototype.indexOf"), Some(Sp_indexOf), 1);
        pf(J, cstr!("String.prototype.lastIndexOf"), Some(Sp_lastIndexOf), 1);
        pf(J, cstr!("String.prototype.localeCompare"), Some(Sp_localeCompare), 1);
        pf(J, cstr!("String.prototype.match"), Some(Sp_match), 1);
        pf(J, cstr!("String.prototype.replace"), Some(Sp_replace), 2);
        pf(J, cstr!("String.prototype.search"), Some(Sp_search), 1);
        pf(J, cstr!("String.prototype.slice"), Some(Sp_slice), 2);
        pf(J, cstr!("String.prototype.split"), Some(Sp_split), 2);
        pf(J, cstr!("String.prototype.substring"), Some(Sp_substring), 2);
        pf(J, cstr!("String.prototype.toLowerCase"), Some(Sp_toLowerCase), 0);
        pf(J, cstr!("String.prototype.toLocaleLowerCase"), Some(Sp_toLowerCase), 0);
        pf(J, cstr!("String.prototype.toUpperCase"), Some(Sp_toUpperCase), 0);
        pf(J, cstr!("String.prototype.toLocaleUpperCase"), Some(Sp_toUpperCase), 0);
        pf(J, cstr!("String.prototype.trim"), Some(Sp_trim), 0);
    }
    crate::jsvalue::js_newcconstructor(J, Some(jsB_String), Some(jsB_new_String), cstr!("String"), 0);
    {
        crate::jsbuiltin::jsB_propf(J, cstr!("String.fromCharCode"), Some(S_fromCharCode), 0);
    }
    js_defglobal(J, cstr!("String"), JS_DONTENUM);
}
