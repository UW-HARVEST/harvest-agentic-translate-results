//! Translation of jsstring.c

use crate::*;

const EOF: c_int = -1;

unsafe fn js_doregexec(
    J: *mut js_State,
    prog: *mut crate::regexp::Reprog,
    string: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    let result: c_int = crate::regexp::js_regexec(prog, string, sub, eflags);
    if result < 0 {
        js_error!(J, "regexec failed");
    }
    result
}

unsafe fn checkstring(J: *mut js_State, idx: c_int) -> *const c_char {
    if js_iscoercible(J, idx) == 0 {
        js_typeerror!(J, "string function called on null or undefined");
    }
    js_tostring(J, idx)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_runeat(J: *mut js_State, s: *const c_char, i: c_int) -> c_int {
    let mut s = s;
    let mut i = i;
    let mut rune: Rune = EOF;
    while i >= 0 {
        rune = *(s as *const u8) as c_int;
        if rune < Runeself {
            if rune == 0 {
                return EOF;
            }
            s = s.add(1);
            i -= 1;
        } else {
            s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
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
pub unsafe extern "C" fn js_utflen(s: *const c_char) -> c_int {
    let c: c_int;
    let mut n: c_int;
    let mut rune: Rune = 0;
    let mut s = s;

    n = 0;
    loop {
        let c = *(s as *const u8) as c_int;
        if c < Runeself {
            if c == 0 {
                return n;
            }
            s = s.add(1);
            n += 1;
        } else {
            s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
            if rune >= 0x10000 {
                n += 2;
            } else {
                n += 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_utfptrtoidx(s: *const c_char, p: *const c_char) -> c_int {
    let mut rune: Rune = 0;
    let mut i: c_int = 0;
    let mut s = s;
    while s < p {
        if (*(s as *const u8) as c_int) < Runeself {
            s = s.add(1);
            i += 1;
        } else {
            s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
            if rune >= 0x10000 {
                i += 2;
            } else {
                i += 1;
            }
        }
    }
    i
}

unsafe extern "C" fn jsB_new_String(J: *mut js_State) {
    js_newstring(
        J,
        if js_gettop(J) > 1 {
            js_tostring(J, 1)
        } else {
            cs!("")
        },
    );
}

unsafe extern "C" fn jsB_String(J: *mut js_State) {
    js_pushstring(
        J,
        if js_gettop(J) > 1 {
            js_tostring(J, 1)
        } else {
            cs!("")
        },
    );
}

unsafe extern "C" fn Sp_toString(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        js_typeerror!(J, "not a string");
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C" fn Sp_valueOf(J: *mut js_State) {
    let self_: *mut js_Object = js_toobject(J, 0);
    if (*self_).type_ != JS_CSTRING {
        js_typeerror!(J, "not a string");
    }
    js_pushstring(J, (*self_).u.s.string);
}

unsafe extern "C" fn Sp_charAt(J: *mut js_State) {
    let mut buf: [c_char; (UTFmax + 1) as usize] = [0; (UTFmax + 1) as usize];
    let s: *const c_char = checkstring(J, 0);
    let pos: c_int = js_tointeger(J, 1);
    let mut rune: Rune = js_runeat(J, s, pos);
    if rune >= 0 {
        let k = runetochar(buf.as_mut_ptr(), addr_of!(rune));
        buf[k as usize] = 0;
        js_pushstring(J, buf.as_ptr());
    } else {
        js_pushliteral(J, cs!(""));
    }
}

unsafe extern "C" fn Sp_charCodeAt(J: *mut js_State) {
    let s: *const c_char = checkstring(J, 0);
    let pos: c_int = js_tointeger(J, 1);
    let rune: Rune = js_runeat(J, s, pos);
    if rune >= 0 {
        js_pushnumber(J, rune as f64);
    } else {
        js_pushnumber(J, NAN);
    }
}

unsafe extern "C" fn Sp_concat(J: *mut js_State) {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut n: c_int;
    let mut out: *mut c_char = null_mut();
    let mut s: *const c_char;

    if top == 1 {
        return;
    }

    s = checkstring(J, 0);
    n = 1 + strlen(s) as c_int;

    if js_try!(J) != 0 {
        js_free(J, vol!(out) as *mut c_void);
        js_throw(J);
    }

    if n > JS_STRLIMIT as c_int {
        js_rangeerror!(J, "invalid string length");
    }
    setvol!(out, js_malloc(J, n) as *mut c_char);
    strcpy(vol!(out), s);

    i = 1;
    while i < top {
        s = js_tostring(J, i);
        n += strlen(s) as c_int;
        if n > JS_STRLIMIT as c_int {
            js_rangeerror!(J, "invalid string length");
        }
        setvol!(out, js_realloc(J, vol!(out) as *mut c_void, n) as *mut c_char);
        strcat(vol!(out), s);
        i += 1;
    }

    js_pushstring(J, vol!(out) as *const c_char);
    js_endtry(J);
    js_free(J, vol!(out) as *mut c_void);
}

unsafe extern "C" fn Sp_indexOf(J: *mut js_State) {
    let mut haystack: *const c_char = checkstring(J, 0);
    let needle: *const c_char = js_tostring(J, 1);
    let pos: c_int = js_tointeger(J, 2);
    let len: c_int = strlen(needle) as c_int;
    let mut k: c_int = 0;
    let mut rune: Rune = 0;
    while *haystack != 0 {
        if k >= pos && strncmp(haystack, needle, len as usize) == 0 {
            js_pushnumber(J, k as f64);
            return;
        }
        haystack = haystack.offset(chartorune(addr_of_mut!(rune), haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, -1.0);
}

unsafe extern "C" fn Sp_lastIndexOf(J: *mut js_State) {
    let mut haystack: *const c_char = checkstring(J, 0);
    let needle: *const c_char = js_tostring(J, 1);
    let pos: c_int = if js_isdefined(J, 2) != 0 {
        js_tointeger(J, 2)
    } else {
        strlen(haystack) as c_int
    };
    let len: c_int = strlen(needle) as c_int;
    let mut k: c_int = 0;
    let mut last: c_int = -1;
    let mut rune: Rune = 0;
    while *haystack != 0 && k <= pos {
        if strncmp(haystack, needle, len as usize) == 0 {
            last = k;
        }
        haystack = haystack.offset(chartorune(addr_of_mut!(rune), haystack) as isize);
        k += 1;
    }
    js_pushnumber(J, last as f64);
}

unsafe extern "C" fn Sp_localeCompare(J: *mut js_State) {
    let a: *const c_char = checkstring(J, 0);
    let b: *const c_char = js_tostring(J, 1);
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
    let head_len: c_int;
    let tail_len: c_int;

    /* find start of substring */
    head = s;
    i = 0;
    while i < a {
        head = head.offset(chartorune(addr_of_mut!(head_rune), head) as isize);
        if head_rune >= 0x10000 {
            i += 1;
        }
        i += 1;
    }

    /* find end of substring */
    tail = head;
    k = i - a;
    while k < n {
        tail = tail.offset(chartorune(addr_of_mut!(tail_rune), tail) as isize);
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

    if js_try!(J) != 0 {
        js_free(J, p as *mut c_void);
        js_throw(J);
    }

    p = js_malloc(J, UTFmax + tail.offset_from(head) as c_int) as *mut c_char;

    /* substring starts with low surrogate (head is just after character) */
    if i > a {
        head_rune = 0xdc00 + ((head_rune - 0x10000) & 0x3ff);
        head_len = runetochar(p, addr_of!(head_rune));
        memcpy(
            p.offset(head_len as isize) as *mut c_void,
            head as *const c_void,
            tail.offset_from(head) as usize,
        );
        js_pushlstring(J, p, head_len + tail.offset_from(head) as c_int);
    }

    /* substring ends with high surrogate (tail is just after character) */
    if k > n {
        tail = tail.offset(-(runelen(tail_rune) as isize));
        memcpy(
            p as *mut c_void,
            head as *const c_void,
            tail.offset_from(head) as usize,
        );
        tail_rune = 0xd800 + ((tail_rune - 0x10000) >> 10);
        tail_len = runetochar(
            p.offset(tail.offset_from(head) as isize),
            addr_of!(tail_rune),
        );
        js_pushlstring(J, p, tail.offset_from(head) as c_int + tail_len);
    }

    js_endtry(J);
    js_free(J, p as *mut c_void);
}

unsafe extern "C" fn Sp_slice(J: *mut js_State) {
    let str_: *const c_char = checkstring(J, 0);
    let len: c_int = js_utflen(str_);
    let mut s: c_int = js_tointeger(J, 1);
    let mut e: c_int = if js_isdefined(J, 2) != 0 {
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
        Sp_substring_imp(J, str_, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str_, e, s - e);
    } else {
        js_pushliteral(J, cs!(""));
    }
}

unsafe extern "C" fn Sp_substring(J: *mut js_State) {
    let str_: *const c_char = checkstring(J, 0);
    let len: c_int = js_utflen(str_);
    let mut s: c_int = js_tointeger(J, 1);
    let mut e: c_int = if js_isdefined(J, 2) != 0 {
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
        Sp_substring_imp(J, str_, s, e - s);
    } else if s > e {
        Sp_substring_imp(J, str_, e, s - e);
    } else {
        js_pushliteral(J, cs!(""));
    }
}

unsafe extern "C" fn Sp_toLowerCase(J: *mut js_State) {
    let mut s: *const c_char;
    let s0: *const c_char = checkstring(J, 0);
    let mut dst: *mut c_char = null_mut();
    let mut d: *mut c_char;
    let mut rune: Rune = 0;
    let mut full: *const Rune;
    let mut n: c_int;

    n = 1;
    s = s0;
    while *s != 0 {
        s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
        full = tolowerrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += runelen(*full);
                full = full.add(1);
            }
        } else {
            rune = tolowerrune(rune);
            n += runelen(rune);
        }
    }

    if js_try!(J) != 0 {
        js_free(J, vol!(dst) as *mut c_void);
        js_throw(J);
    }

    setvol!(dst, js_malloc(J, n) as *mut c_char);
    d = vol!(dst);
    s = s0;
    while *s != 0 {
        s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
        full = tolowerrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                d = d.offset(runetochar(d, full) as isize);
                full = full.add(1);
            }
        } else {
            rune = tolowerrune(rune);
            d = d.offset(runetochar(d, addr_of!(rune)) as isize);
        }
    }
    *d = 0;

    js_pushstring(J, vol!(dst) as *const c_char);
    js_endtry(J);
    js_free(J, vol!(dst) as *mut c_void);
}

unsafe extern "C" fn Sp_toUpperCase(J: *mut js_State) {
    let mut s: *const c_char;
    let s0: *const c_char = checkstring(J, 0);
    let mut dst: *mut c_char = null_mut();
    let mut d: *mut c_char;
    let mut full: *const Rune;
    let mut rune: Rune = 0;
    let mut n: c_int;

    n = 1;
    s = s0;
    while *s != 0 {
        s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
        full = toupperrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                n += runelen(*full);
                full = full.add(1);
            }
        } else {
            rune = toupperrune(rune);
            n += runelen(rune);
        }
    }

    if js_try!(J) != 0 {
        js_free(J, vol!(dst) as *mut c_void);
        js_throw(J);
    }

    setvol!(dst, js_malloc(J, n) as *mut c_char);
    d = vol!(dst);
    s = s0;
    while *s != 0 {
        s = s.offset(chartorune(addr_of_mut!(rune), s) as isize);
        full = toupperrune_full(rune);
        if !full.is_null() {
            while *full != 0 {
                d = d.offset(runetochar(d, full) as isize);
                full = full.add(1);
            }
        } else {
            rune = toupperrune(rune);
            d = d.offset(runetochar(d, addr_of!(rune)) as isize);
        }
    }
    *d = 0;

    js_pushstring(J, vol!(dst) as *const c_char);
    js_endtry(J);
    js_free(J, vol!(dst) as *mut c_void);
}

unsafe fn isbol(re: *mut js_Regexp, text: *const c_char, a: *const c_char) -> c_int {
    (a == text || (((*re).flags as c_int & JS_REGEXP_M) != 0 && *a.offset(-1) == '\n' as c_char))
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

unsafe extern "C" fn Sp_trim(J: *mut js_State) {
    let mut s: *const c_char;
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

unsafe extern "C" fn S_fromCharCode(J: *mut js_State) {
    let mut i: c_int;
    let top: c_int = js_gettop(J);
    let mut s: *mut c_char = null_mut();
    let mut p: *mut c_char;
    let mut c: Rune;

    if js_try!(J) != 0 {
        js_free(J, vol!(s) as *mut c_void);
        js_throw(J);
    }

    setvol!(s, js_malloc(J, (top - 1) * UTFmax + 1) as *mut c_char);
    p = vol!(s);

    i = 1;
    while i < top {
        c = js_touint32(J, i) as Rune;
        p = p.offset(runetochar(p, addr_of!(c)) as isize);
        i += 1;
    }
    *p = 0;

    js_pushstring(J, vol!(s) as *const c_char);
    js_endtry(J);
    js_free(J, vol!(s) as *mut c_void);
}

unsafe extern "C" fn Sp_match(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text: *const c_char;
    let mut len: c_int;
    let mut a: *const c_char;
    let b: *const c_char;
    let c: *const c_char;
    let e: *const c_char;
    let mut m: Resub = core::mem::zeroed();
    let mut rune: Rune = 0;

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        js_newregexp(J, cs!(""), 0);
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
    e = text.add(strlen(text));
    while a <= e {
        if js_doregexec(
            J,
            (*re).prog as *mut crate::regexp::Reprog,
            a,
            addr_of_mut!(m),
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
            a = a.offset(chartorune(addr_of_mut!(rune), a) as isize);
        }
    }

    if len == 0 {
        js_pop(J, 1);
        js_pushnull(J);
    }
}

unsafe extern "C" fn Sp_search(J: *mut js_State) {
    let re: *mut js_Regexp;
    let text: *const c_char;
    let mut m: Resub = core::mem::zeroed();

    text = checkstring(J, 0);

    if js_isregexp(J, 1) != 0 {
        js_copy(J, 1);
    } else if js_isundefined(J, 1) != 0 {
        js_newregexp(J, cs!(""), 0);
    } else {
        js_newregexp(J, js_tostring(J, 1), 0);
    }

    re = js_toregexp(J, -1);

    if js_doregexec(
        J,
        (*re).prog as *mut crate::regexp::Reprog,
        text,
        addr_of_mut!(m),
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
    let mut source: *const c_char;
    let source0: *const c_char;
    let mut s: *const c_char;
    let mut r: *const c_char;
    let mut sb: *mut js_Buffer = null_mut();
    let mut n: c_int;
    let mut x: c_int;
    let mut m: Resub = core::mem::zeroed();

    source = checkstring(J, 0);
    source0 = source;
    re = js_toregexp(J, 1);

    if js_doregexec(
        J,
        (*re).prog as *mut crate::regexp::Reprog,
        source,
        addr_of_mut!(m),
        0,
    ) != 0
    {
        js_copy(J, 0);
        return;
    }

    (*re).last = 0;

    if js_try!(J) != 0 {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }

    s = null();
    n = 0;

    'end: {
        loop {
            /* loop: */
            s = m.sub[0].sp;
            n = m.sub[0].ep.offset_from(m.sub[0].sp) as c_int;

            if js_iscallable(J, 2) != 0 {
                js_copy(J, 2);
                js_pushundefined(J);
                x = 0;
                /* arg 0..x: substring and subexps that matched */
                while !m.sub[x as usize].sp.is_null() {
                    js_pushlstring(
                        J,
                        m.sub[x as usize].sp,
                        m.sub[x as usize]
                            .ep
                            .offset_from(m.sub[x as usize].sp) as c_int,
                    );
                    x += 1;
                }
                js_pushnumber(J, s.offset_from(source) as f64); /* arg x+2: offset within search string */
                js_copy(J, 0); /* arg x+3: search string */
                js_call(J, 2 + x);
                r = js_tostring(J, -1);
                js_putm(J, addr_of_mut!(sb), source, s);
                js_puts(J, addr_of_mut!(sb), r);
                js_pop(J, 1);
            } else {
                r = js_tostring(J, 2);
                js_putm(J, addr_of_mut!(sb), source, s);
                while *r != 0 {
                    if *r == '$' as c_char {
                        r = r.add(1);
                        let ch: c_int = *r as c_int;
                        if ch == 0 {
                            r = r.offset(-1); /* end of string; back up */
                            /* fallthrough */
                            js_putc(J, addr_of_mut!(sb), '$' as c_int);
                        } else if ch == '$' as c_int {
                            js_putc(J, addr_of_mut!(sb), '$' as c_int);
                        } else if ch == '`' as c_int {
                            js_putm(J, addr_of_mut!(sb), source0, s);
                        } else if ch == '\'' as c_int {
                            js_puts(J, addr_of_mut!(sb), s.offset(n as isize));
                        } else if ch == '&' as c_int {
                            js_putm(J, addr_of_mut!(sb), s, s.offset(n as isize));
                        } else if ch >= '0' as c_int && ch <= '9' as c_int {
                            x = *r as c_int - '0' as c_int;
                            if *r.offset(1) >= '0' as c_char && *r.offset(1) <= '9' as c_char {
                                r = r.add(1);
                                x = x * 10 + *r as c_int - '0' as c_int;
                            }
                            if x > 0 && x < m.nsub {
                                js_putm(
                                    J,
                                    addr_of_mut!(sb),
                                    m.sub[x as usize].sp,
                                    m.sub[x as usize].ep,
                                );
                            } else {
                                js_putc(J, addr_of_mut!(sb), '$' as c_int);
                                if x > 10 {
                                    js_putc(J, addr_of_mut!(sb), '0' as c_int + x / 10);
                                    js_putc(J, addr_of_mut!(sb), '0' as c_int + x % 10);
                                } else {
                                    js_putc(J, addr_of_mut!(sb), '0' as c_int + x);
                                }
                            }
                        } else {
                            js_putc(J, addr_of_mut!(sb), '$' as c_int);
                            js_putc(J, addr_of_mut!(sb), *r as c_int);
                        }
                        r = r.add(1);
                    } else {
                        js_putc(J, addr_of_mut!(sb), *r as c_int);
                        r = r.add(1);
                    }
                }
            }

            if ((*re).flags as c_int & JS_REGEXP_G) != 0 {
                source = m.sub[0].ep;
                if n == 0 {
                    if *source != 0 {
                        js_putc(J, addr_of_mut!(sb), *source as c_int);
                        source = source.add(1);
                    } else {
                        break 'end;
                    }
                }
                if js_doregexec(
                    J,
                    (*re).prog as *mut crate::regexp::Reprog,
                    source,
                    addr_of_mut!(m),
                    if isbol(re, source0, source) != 0 {
                        0
                    } else {
                        REG_NOTBOL
                    },
                ) == 0
                {
                    continue;
                }
            }

            break;
        }
    }

    /* end: */
    js_puts(J, addr_of_mut!(sb), s.offset(n as isize));
    js_putc(J, addr_of_mut!(sb), 0);

    js_pushstring(
        J,
        if !sb.is_null() {
            addr_of!((*sb).s) as *const c_char
        } else {
            cs!("")
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

unsafe fn Sp_replace_string(J: *mut js_State) {
    let source: *const c_char;
    let needle: *const c_char;
    let s: *const c_char;
    let mut r: *const c_char;
    let mut sb: *mut js_Buffer = null_mut();
    let n: c_int;

    source = checkstring(J, 0);
    needle = js_tostring(J, 1);

    s = strstr(source, needle);
    if s.is_null() {
        js_copy(J, 0);
        return;
    }
    n = strlen(needle) as c_int;

    if js_try!(J) != 0 {
        js_free(J, sb as *mut c_void);
        js_throw(J);
    }

    if js_iscallable(J, 2) != 0 {
        js_copy(J, 2);
        js_pushundefined(J);
        js_pushlstring(J, s, n); /* arg 1: substring that matched */
        js_pushnumber(J, s.offset_from(source) as f64); /* arg 2: offset within search string */
        js_copy(J, 0); /* arg 3: search string */
        js_call(J, 3);
        r = js_tostring(J, -1);
        js_putm(J, addr_of_mut!(sb), source, s);
        js_puts(J, addr_of_mut!(sb), r);
        js_puts(J, addr_of_mut!(sb), s.offset(n as isize));
        js_putc(J, addr_of_mut!(sb), 0);
        js_pop(J, 1);
    } else {
        r = js_tostring(J, 2);
        js_putm(J, addr_of_mut!(sb), source, s);
        while *r != 0 {
            if *r == '$' as c_char {
                r = r.add(1);
                let ch: c_int = *r as c_int;
                if ch == 0 {
                    r = r.offset(-1); /* end of string; back up */
                    /* fallthrough */
                    js_putc(J, addr_of_mut!(sb), '$' as c_int);
                } else if ch == '$' as c_int {
                    js_putc(J, addr_of_mut!(sb), '$' as c_int);
                } else if ch == '&' as c_int {
                    js_putm(J, addr_of_mut!(sb), s, s.offset(n as isize));
                } else if ch == '`' as c_int {
                    js_putm(J, addr_of_mut!(sb), source, s);
                } else if ch == '\'' as c_int {
                    js_puts(J, addr_of_mut!(sb), s.offset(n as isize));
                } else {
                    js_putc(J, addr_of_mut!(sb), '$' as c_int);
                    js_putc(J, addr_of_mut!(sb), *r as c_int);
                }
                r = r.add(1);
            } else {
                js_putc(J, addr_of_mut!(sb), *r as c_int);
                r = r.add(1);
            }
        }
        js_puts(J, addr_of_mut!(sb), s.offset(n as isize));
        js_putc(J, addr_of_mut!(sb), 0);
    }

    js_pushstring(
        J,
        if !sb.is_null() {
            addr_of!((*sb).s) as *const c_char
        } else {
            cs!("")
        },
    );
    js_endtry(J);
    js_free(J, sb as *mut c_void);
}

unsafe extern "C" fn Sp_replace(J: *mut js_State) {
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
    let b: *const c_char;
    let c: *const c_char;
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

    e = text.add(strlen(text));

    /* splitting the empty string */
    if e == text {
        if js_doregexec(
            J,
            (*re).prog as *mut crate::regexp::Reprog,
            text,
            addr_of_mut!(m),
            0,
        ) != 0
        {
            js_pushliteral(J, cs!(""));
            js_setindex(J, -2, 0);
        }
        return;
    }

    a = text;
    p = a;
    while a < e {
        if js_doregexec(
            J,
            (*re).prog as *mut crate::regexp::Reprog,
            a,
            addr_of_mut!(m),
            if isbol(re, text, a) != 0 { 0 } else { REG_NOTBOL },
        ) != 0
        {
            break; /* no match */
        }

        let b = m.sub[0].sp;
        let c = m.sub[0].ep;

        /* empty string at end of last match */
        if b == c && b == p {
            a = a.offset(chartorune(addr_of_mut!(rune), a) as isize);
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

unsafe fn Sp_split_string(J: *mut js_State) {
    let mut str_: *const c_char = checkstring(J, 0);
    let sep: *const c_char = js_tostring(J, 1);
    let limit: c_int = if js_isdefined(J, 2) != 0 {
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
        while *str_ != 0 && i < limit {
            n = chartorune(addr_of_mut!(rune), str_);
            js_pushlstring(J, str_, n);
            js_setindex(J, -2, i);
            str_ = str_.offset(n as isize);
            i += 1;
        }
        return;
    }

    i = 0;
    while !str_.is_null() && i < limit {
        let s: *const c_char = strstr(str_, sep);
        if !s.is_null() {
            js_pushlstring(J, str_, s.offset_from(str_) as c_int);
            js_setindex(J, -2, i);
            str_ = s.offset(n as isize);
        } else {
            js_pushstring(J, str_);
            js_setindex(J, -2, i);
            str_ = null();
        }
        i += 1;
    }
}

unsafe extern "C" fn Sp_split(J: *mut js_State) {
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
pub unsafe extern "C" fn jsB_initstring(J: *mut js_State) {
    let sp: *mut js_Object = (*J).String_prototype;
    *(addr_of_mut!((*sp).u.s.shrstr) as *mut c_char) = 0;
    (*sp).u.s.string = addr_of_mut!((*sp).u.s.shrstr) as *mut c_char;
    (*sp).u.s.length = 0;

    js_pushobject(J, (*J).String_prototype);
    {
        jsB_propf(J, cs!("String.prototype.toString"), Some(Sp_toString), 0);
        jsB_propf(J, cs!("String.prototype.valueOf"), Some(Sp_valueOf), 0);
        jsB_propf(J, cs!("String.prototype.charAt"), Some(Sp_charAt), 1);
        jsB_propf(J, cs!("String.prototype.charCodeAt"), Some(Sp_charCodeAt), 1);
        jsB_propf(J, cs!("String.prototype.concat"), Some(Sp_concat), 0); /* 1 */
        jsB_propf(J, cs!("String.prototype.indexOf"), Some(Sp_indexOf), 1);
        jsB_propf(
            J,
            cs!("String.prototype.lastIndexOf"),
            Some(Sp_lastIndexOf),
            1,
        );
        jsB_propf(
            J,
            cs!("String.prototype.localeCompare"),
            Some(Sp_localeCompare),
            1,
        );
        jsB_propf(J, cs!("String.prototype.match"), Some(Sp_match), 1);
        jsB_propf(J, cs!("String.prototype.replace"), Some(Sp_replace), 2);
        jsB_propf(J, cs!("String.prototype.search"), Some(Sp_search), 1);
        jsB_propf(J, cs!("String.prototype.slice"), Some(Sp_slice), 2);
        jsB_propf(J, cs!("String.prototype.split"), Some(Sp_split), 2);
        jsB_propf(J, cs!("String.prototype.substring"), Some(Sp_substring), 2);
        jsB_propf(
            J,
            cs!("String.prototype.toLowerCase"),
            Some(Sp_toLowerCase),
            0,
        );
        jsB_propf(
            J,
            cs!("String.prototype.toLocaleLowerCase"),
            Some(Sp_toLowerCase),
            0,
        );
        jsB_propf(
            J,
            cs!("String.prototype.toUpperCase"),
            Some(Sp_toUpperCase),
            0,
        );
        jsB_propf(
            J,
            cs!("String.prototype.toLocaleUpperCase"),
            Some(Sp_toUpperCase),
            0,
        );

        /* ES5 */
        jsB_propf(J, cs!("String.prototype.trim"), Some(Sp_trim), 0);
    }
    js_newcconstructor(
        J,
        Some(jsB_String),
        Some(jsB_new_String),
        cs!("String"),
        0,
    ); /* 1 */
    {
        jsB_propf(J, cs!("String.fromCharCode"), Some(S_fromCharCode), 0); /* 1 */
    }
    js_defglobal(J, cs!("String"), JS_DONTENUM);
}
