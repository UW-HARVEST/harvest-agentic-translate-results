// Translation of c_src/src/jslex.c
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]

use crate::common::*;
use crate::jsdtoa::js_strtod;
use crate::jserror::js_newsyntaxerror;
use crate::jsrun::{js_free, js_malloc, js_realloc, js_throw};
use crate::lexdata::TOKENSTRING;
use crate::types::*;
use crate::utf::*;
use std::ffi::{c_char, c_int, c_void};
use std::ptr;

pub const EOF: c_int = -1;

/// Raise a syntax error at the current lexer position; `msg` is preformatted.
pub unsafe fn jsY_error_msg(J: *mut js_State, msg: *const c_char) -> ! {
    unsafe {
        let mut buf: [c_char; 512] = [0; 512];
        snprintf(
            buf.as_mut_ptr(),
            256,
            c"%s:%d: ".as_ptr(),
            (*J).filename,
            (*J).lexline,
        );
        strcat(buf.as_mut_ptr(), msg);
        js_newsyntaxerror(J, buf.as_ptr());
        js_throw(J)
    }
}

macro_rules! jsY_error {
    ($J:expr, $fmt:literal) => {
        crate::jslex::jsY_error_msg($J, $fmt.as_ptr())
    };
    ($J:expr, $fmt:literal, $($a:expr),+) => {{
        let mut __m = [0 as c_char; 256];
        snprintf(__m.as_mut_ptr(), 256, $fmt.as_ptr(), $($a),+);
        crate::jslex::jsY_error_msg($J, __m.as_ptr())
    }};
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_tokenstring(token: c_int) -> *const c_char {
    if token >= 0 && token < TOKENSTRING.len() as c_int {
        let e = TOKENSTRING[token as usize];
        if !e.is_empty() {
            return e.as_ptr() as *const c_char;
        }
    }
    c"<unknown>".as_ptr()
}

static KEYWORDS: [&[u8]; 29] = [
    b"break\0",
    b"case\0",
    b"catch\0",
    b"continue\0",
    b"debugger\0",
    b"default\0",
    b"delete\0",
    b"do\0",
    b"else\0",
    b"false\0",
    b"finally\0",
    b"for\0",
    b"function\0",
    b"if\0",
    b"in\0",
    b"instanceof\0",
    b"new\0",
    b"null\0",
    b"return\0",
    b"switch\0",
    b"this\0",
    b"throw\0",
    b"true\0",
    b"try\0",
    b"typeof\0",
    b"var\0",
    b"void\0",
    b"while\0",
    b"with\0",
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_findword(
    s: *const c_char,
    list: *mut *const c_char,
    num: c_int,
) -> c_int {
    unsafe {
        let mut l = 0;
        let mut r = num - 1;
        while l <= r {
            let m = (l + r) >> 1;
            let c = strcmp(s, *list.offset(m as isize));
            if c < 0 {
                r = m - 1;
            } else if c > 0 {
                l = m + 1;
            } else {
                return m;
            }
        }
        -1
    }
}

unsafe fn findword_slice(s: *const c_char, list: &[&[u8]]) -> c_int {
    unsafe {
        let mut l: c_int = 0;
        let mut r: c_int = list.len() as c_int - 1;
        while l <= r {
            let m = (l + r) >> 1;
            let c = strcmp(s, list[m as usize].as_ptr() as *const c_char);
            if c < 0 {
                r = m - 1;
            } else if c > 0 {
                l = m + 1;
            } else {
                return m;
            }
        }
        -1
    }
}

unsafe fn jsY_findkeyword(J: *mut js_State, s: *const c_char) -> c_int {
    unsafe {
        let i = findword_slice(s, &KEYWORDS);
        if i >= 0 {
            (*J).text = KEYWORDS[i as usize].as_ptr() as *const c_char;
            return TK_BREAK + i; /* first keyword + i */
        }
        (*J).text = s;
        TK_IDENTIFIER
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_iswhite(c: c_int) -> c_int {
    (c == 0x9 || c == 0xB || c == 0xC || c == 0x20 || c == 0xA0 || c == 0xFEFF) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

#[inline]
fn isalpha(c: c_int) -> bool {
    (c >= b'a' as c_int && c <= b'z' as c_int) || (c >= b'A' as c_int && c <= b'Z' as c_int)
}
#[inline]
fn isdigit(c: c_int) -> bool {
    c >= b'0' as c_int && c <= b'9' as c_int
}
#[inline]
fn ishex(c: c_int) -> bool {
    (c >= b'a' as c_int && c <= b'f' as c_int) || (c >= b'A' as c_int && c <= b'F' as c_int)
}

fn jsY_isidentifierstart(c: c_int) -> bool {
    isalpha(c) || c == b'$' as c_int || c == b'_' as c_int || jsU_isalpharune(c) != 0
}

fn jsY_isidentifierpart(c: c_int) -> bool {
    isdigit(c)
        || isalpha(c)
        || c == b'$' as c_int
        || c == b'_' as c_int
        || jsU_isalpharune(c) != 0
}

fn jsY_isdec(c: c_int) -> bool {
    isdigit(c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_ishex(c: c_int) -> c_int {
    (isdigit(c) || ishex(c)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_tohex(c: c_int) -> c_int {
    if c >= b'0' as c_int && c <= b'9' as c_int {
        return c - b'0' as c_int;
    }
    if c >= b'a' as c_int && c <= b'f' as c_int {
        return c - b'a' as c_int + 0xA;
    }
    if c >= b'A' as c_int && c <= b'F' as c_int {
        return c - b'A' as c_int + 0xA;
    }
    0
}

unsafe fn jsY_next(J: *mut js_State) {
    unsafe {
        let mut c: Rune = 0;
        if *(*J).source == 0 {
            (*J).lexchar = EOF;
            return;
        }
        (*J).source = (*J).source.offset(jsU_chartorune(&mut c, (*J).source) as isize);
        /* consume CR LF as one unit */
        if c == b'\r' as Rune && *(*J).source == b'\n' as c_char {
            (*J).source = (*J).source.add(1);
        }
        if jsY_isnewline(c) != 0 {
            (*J).line += 1;
            c = b'\n' as Rune;
        }
        (*J).lexchar = c;
    }
}

#[inline]
unsafe fn jsY_accept(J: *mut js_State, x: c_int) -> bool {
    unsafe {
        if (*J).lexchar == x {
            jsY_next(J);
            true
        } else {
            false
        }
    }
}

macro_rules! jsY_expect {
    ($J:expr, $x:expr) => {
        if !jsY_accept($J, $x) {
            jsY_error!($J, c"expected '%c'", $x);
        }
    };
}

unsafe fn jsY_unescape(J: *mut js_State) {
    unsafe {
        if jsY_accept(J, b'\\' as c_int) {
            let mut ok = false;
            if jsY_accept(J, b'u' as c_int) {
                let mut x = 0;
                loop {
                    if jsY_ishex((*J).lexchar) == 0 {
                        break;
                    }
                    x |= jsY_tohex((*J).lexchar) << 12;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break;
                    }
                    x |= jsY_tohex((*J).lexchar) << 8;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break;
                    }
                    x |= jsY_tohex((*J).lexchar) << 4;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break;
                    }
                    x |= jsY_tohex((*J).lexchar);
                    (*J).lexchar = x;
                    ok = true;
                    break;
                }
                if ok {
                    return;
                }
            }
            jsY_error!(J, c"unexpected escape sequence");
        }
    }
}

unsafe fn textinit(J: *mut js_State) {
    unsafe {
        if (*J).lexbuf.text.is_null() {
            (*J).lexbuf.cap = 4096;
            (*J).lexbuf.text = js_malloc(J, (*J).lexbuf.cap) as *mut c_char;
        }
        (*J).lexbuf.len = 0;
    }
}

unsafe fn textpush(J: *mut js_State, c: Rune) {
    unsafe {
        let n;
        if c == EOF {
            n = 1;
        } else {
            n = jsU_runelen(c);
        }
        if (*J).lexbuf.len + n > (*J).lexbuf.cap {
            let newcap = (*J).lexbuf.cap * 2;
            (*J).lexbuf.text =
                js_realloc(J, (*J).lexbuf.text as *mut c_void, newcap) as *mut c_char;
            (*J).lexbuf.cap = newcap;
        }
        if c == EOF {
            *(*J).lexbuf.text.offset((*J).lexbuf.len as isize) = 0;
            (*J).lexbuf.len += 1;
        } else {
            (*J).lexbuf.len +=
                jsU_runetochar((*J).lexbuf.text.offset((*J).lexbuf.len as isize), &c);
        }
    }
}

unsafe fn textend(J: *mut js_State) -> *mut c_char {
    unsafe {
        textpush(J, EOF);
        (*J).lexbuf.text
    }
}

unsafe fn lexlinecomment(J: *mut js_State) {
    unsafe {
        while (*J).lexchar != EOF && (*J).lexchar != b'\n' as c_int {
            jsY_next(J);
        }
    }
}

unsafe fn lexcomment(J: *mut js_State) -> c_int {
    unsafe {
        /* already consumed initial '/' '*' sequence */
        while (*J).lexchar != EOF {
            if jsY_accept(J, b'*' as c_int) {
                while (*J).lexchar == b'*' as c_int {
                    jsY_next(J);
                }
                if jsY_accept(J, b'/' as c_int) {
                    return 0;
                }
            } else {
                jsY_next(J);
            }
        }
        -1
    }
}

unsafe fn lexhex(J: *mut js_State) -> f64 {
    unsafe {
        let mut n: f64 = 0.0;
        if jsY_ishex((*J).lexchar) == 0 {
            jsY_error!(J, c"malformed hexadecimal number");
        }
        while jsY_ishex((*J).lexchar) != 0 {
            n = n * 16.0 + jsY_tohex((*J).lexchar) as f64;
            jsY_next(J);
        }
        n
    }
}

unsafe fn lexnumber(J: *mut js_State) -> c_int {
    unsafe {
        let s = (*J).source.offset(-1);

        if jsY_accept(J, b'0' as c_int) {
            if jsY_accept(J, b'x' as c_int) || jsY_accept(J, b'X' as c_int) {
                (*J).number = lexhex(J);
                return TK_NUMBER;
            }
            if jsY_isdec((*J).lexchar) {
                jsY_error!(J, c"number with leading zero");
            }
            if jsY_accept(J, b'.' as c_int) {
                while jsY_isdec((*J).lexchar) {
                    jsY_next(J);
                }
            }
        } else if jsY_accept(J, b'.' as c_int) {
            if !jsY_isdec((*J).lexchar) {
                return b'.' as c_int;
            }
            while jsY_isdec((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            while jsY_isdec((*J).lexchar) {
                jsY_next(J);
            }
            if jsY_accept(J, b'.' as c_int) {
                while jsY_isdec((*J).lexchar) {
                    jsY_next(J);
                }
            }
        }

        if jsY_accept(J, b'e' as c_int) || jsY_accept(J, b'E' as c_int) {
            if (*J).lexchar == b'-' as c_int || (*J).lexchar == b'+' as c_int {
                jsY_next(J);
            }
            if jsY_isdec((*J).lexchar) {
                while jsY_isdec((*J).lexchar) {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing exponent");
            }
        }

        if jsY_isidentifierstart((*J).lexchar) {
            jsY_error!(J, c"number with letter suffix");
        }

        (*J).number = js_strtod(s, ptr::null_mut());
        TK_NUMBER
    }
}

unsafe fn lexescape(J: *mut js_State) -> c_int {
    unsafe {
        let mut x = 0;

        /* already consumed '\' */

        if jsY_accept(J, b'\n' as c_int) {
            return 0;
        }

        match (*J).lexchar {
            EOF => jsY_error!(J, c"unterminated escape sequence"),
            0x75 /* 'u' */ => {
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 12; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 8; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
                textpush(J, x);
            }
            0x78 /* 'x' */ => {
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
                textpush(J, x);
            }
            0x30 /* '0' */ => { textpush(J, 0); jsY_next(J); }
            0x5C /* '\\' */ => { textpush(J, b'\\' as Rune); jsY_next(J); }
            0x27 /* '\'' */ => { textpush(J, b'\'' as Rune); jsY_next(J); }
            0x22 /* '"' */ => { textpush(J, b'"' as Rune); jsY_next(J); }
            0x62 /* 'b' */ => { textpush(J, 8); jsY_next(J); }
            0x66 /* 'f' */ => { textpush(J, 12); jsY_next(J); }
            0x6E /* 'n' */ => { textpush(J, 10); jsY_next(J); }
            0x72 /* 'r' */ => { textpush(J, 13); jsY_next(J); }
            0x74 /* 't' */ => { textpush(J, 9); jsY_next(J); }
            0x76 /* 'v' */ => { textpush(J, 11); jsY_next(J); }
            _ => { textpush(J, (*J).lexchar); jsY_next(J); }
        }
        0
    }
}

unsafe fn lexstring(J: *mut js_State) -> c_int {
    unsafe {
        let q = (*J).lexchar;
        jsY_next(J);

        textinit(J);

        while (*J).lexchar != q {
            if (*J).lexchar == EOF || (*J).lexchar == b'\n' as c_int {
                jsY_error!(J, c"string not terminated");
            }
            if jsY_accept(J, b'\\' as c_int) {
                if lexescape(J) != 0 {
                    jsY_error!(J, c"malformed escape sequence");
                }
            } else {
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, q);

        let s = textend(J);

        (*J).text = s;
        TK_STRING
    }
}

/* the ugliest language wart ever... */
fn isregexpcontext(last: c_int) -> c_int {
    match last {
        0x5D /* ']' */ | 0x29 /* ')' */ | 0x7D /* '}' */ => 0,
        TK_IDENTIFIER | TK_NUMBER | TK_STRING | TK_FALSE | TK_NULL | TK_THIS | TK_TRUE => 0,
        _ => 1,
    }
}

unsafe fn lexregexp(J: *mut js_State) -> c_int {
    unsafe {
        let mut g: c_int;
        let mut m: c_int;
        let mut i: c_int;
        let mut flags: c_int;
        let mut inclass = 0;

        /* already consumed initial '/' */

        textinit(J);

        /* regexp body */
        while (*J).lexchar != b'/' as c_int || inclass != 0 {
            if (*J).lexchar == EOF || (*J).lexchar == b'\n' as c_int {
                jsY_error!(J, c"regular expression not terminated");
            } else if jsY_accept(J, b'\\' as c_int) {
                if jsY_accept(J, b'/' as c_int) {
                    textpush(J, b'/' as Rune);
                } else {
                    textpush(J, b'\\' as Rune);
                    if (*J).lexchar == EOF || (*J).lexchar == b'\n' as c_int {
                        jsY_error!(J, c"regular expression not terminated");
                    }
                    textpush(J, (*J).lexchar);
                    jsY_next(J);
                }
            } else {
                if (*J).lexchar == b'[' as c_int && inclass == 0 {
                    inclass = 1;
                }
                if (*J).lexchar == b']' as c_int && inclass != 0 {
                    inclass = 0;
                }
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, b'/' as c_int);

        let s = textend(J);

        /* regexp flags */
        g = 0;
        i = 0;
        m = 0;

        while jsY_isidentifierpart((*J).lexchar) {
            if jsY_accept(J, b'g' as c_int) {
                g += 1;
            } else if jsY_accept(J, b'i' as c_int) {
                i += 1;
            } else if jsY_accept(J, b'm' as c_int) {
                m += 1;
            } else {
                jsY_error!(
                    J,
                    c"illegal flag in regular expression: %c",
                    (*J).lexchar
                );
            }
        }

        if g > 1 || i > 1 || m > 1 {
            jsY_error!(J, c"duplicated flag in regular expression");
        }

        (*J).text = s;

        flags = 0;
        if g != 0 {
            flags |= JS_REGEXP_G;
        }
        if i != 0 {
            flags |= JS_REGEXP_I;
        }
        if m != 0 {
            flags |= JS_REGEXP_M;
        }
        (*J).number = flags as f64;
        TK_REGEXP
    }
}

/* simple "return [no Line Terminator here] ..." contexts */
fn isnlthcontext(last: c_int) -> c_int {
    match last {
        TK_BREAK | TK_CONTINUE | TK_RETURN | TK_THROW => 1,
        _ => 0,
    }
}

unsafe fn jsY_lexx(J: *mut js_State) -> c_int {
    unsafe {
        (*J).newline = 0;

        loop {
            (*J).lexline = (*J).line; /* save location of beginning of token */

            while jsY_iswhite((*J).lexchar) != 0 {
                jsY_next(J);
            }

            if jsY_accept(J, b'\n' as c_int) {
                (*J).newline = 1;
                if isnlthcontext((*J).lasttoken) != 0 {
                    return b';' as c_int;
                }
                continue;
            }

            if jsY_accept(J, b'/' as c_int) {
                if jsY_accept(J, b'/' as c_int) {
                    lexlinecomment(J);
                    continue;
                } else if jsY_accept(J, b'*' as c_int) {
                    if lexcomment(J) != 0 {
                        jsY_error!(J, c"multi-line comment not terminated");
                    }
                    continue;
                } else if isregexpcontext((*J).lasttoken) != 0 {
                    return lexregexp(J);
                } else if jsY_accept(J, b'=' as c_int) {
                    return TK_DIV_ASS;
                } else {
                    return b'/' as c_int;
                }
            }

            if (*J).lexchar >= b'0' as c_int && (*J).lexchar <= b'9' as c_int {
                return lexnumber(J);
            }

            match (*J).lexchar {
                0x28 => {
                    jsY_next(J);
                    return 0x28;
                } /* ( */
                0x29 => {
                    jsY_next(J);
                    return 0x29;
                } /* ) */
                0x2C => {
                    jsY_next(J);
                    return 0x2C;
                } /* , */
                0x3A => {
                    jsY_next(J);
                    return 0x3A;
                } /* : */
                0x3B => {
                    jsY_next(J);
                    return 0x3B;
                } /* ; */
                0x3F => {
                    jsY_next(J);
                    return 0x3F;
                } /* ? */
                0x5B => {
                    jsY_next(J);
                    return 0x5B;
                } /* [ */
                0x5D => {
                    jsY_next(J);
                    return 0x5D;
                } /* ] */
                0x7B => {
                    jsY_next(J);
                    return 0x7B;
                } /* { */
                0x7D => {
                    jsY_next(J);
                    return 0x7D;
                } /* } */
                0x7E => {
                    jsY_next(J);
                    return 0x7E;
                } /* ~ */

                0x27 | 0x22 => return lexstring(J), /* ' " */

                0x2E => return lexnumber(J), /* . */

                0x3C => {
                    /* < */
                    jsY_next(J);
                    if jsY_accept(J, b'<' as c_int) {
                        if jsY_accept(J, b'=' as c_int) {
                            return TK_SHL_ASS;
                        }
                        return TK_SHL;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_LE;
                    }
                    return b'<' as c_int;
                }

                0x3E => {
                    /* > */
                    jsY_next(J);
                    if jsY_accept(J, b'>' as c_int) {
                        if jsY_accept(J, b'>' as c_int) {
                            if jsY_accept(J, b'=' as c_int) {
                                return TK_USHR_ASS;
                            }
                            return TK_USHR;
                        }
                        if jsY_accept(J, b'=' as c_int) {
                            return TK_SHR_ASS;
                        }
                        return TK_SHR;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_GE;
                    }
                    return b'>' as c_int;
                }

                0x3D => {
                    /* = */
                    jsY_next(J);
                    if jsY_accept(J, b'=' as c_int) {
                        if jsY_accept(J, b'=' as c_int) {
                            return TK_STRICTEQ;
                        }
                        return TK_EQ;
                    }
                    return b'=' as c_int;
                }

                0x21 => {
                    /* ! */
                    jsY_next(J);
                    if jsY_accept(J, b'=' as c_int) {
                        if jsY_accept(J, b'=' as c_int) {
                            return TK_STRICTNE;
                        }
                        return TK_NE;
                    }
                    return b'!' as c_int;
                }

                0x2B => {
                    /* + */
                    jsY_next(J);
                    if jsY_accept(J, b'+' as c_int) {
                        return TK_INC;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_ADD_ASS;
                    }
                    return b'+' as c_int;
                }

                0x2D => {
                    /* - */
                    jsY_next(J);
                    if jsY_accept(J, b'-' as c_int) {
                        return TK_DEC;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_SUB_ASS;
                    }
                    return b'-' as c_int;
                }

                0x2A => {
                    /* * */
                    jsY_next(J);
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_MUL_ASS;
                    }
                    return b'*' as c_int;
                }

                0x25 => {
                    /* % */
                    jsY_next(J);
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_MOD_ASS;
                    }
                    return b'%' as c_int;
                }

                0x26 => {
                    /* & */
                    jsY_next(J);
                    if jsY_accept(J, b'&' as c_int) {
                        return TK_AND;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_AND_ASS;
                    }
                    return b'&' as c_int;
                }

                0x7C => {
                    /* | */
                    jsY_next(J);
                    if jsY_accept(J, b'|' as c_int) {
                        return TK_OR;
                    }
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_OR_ASS;
                    }
                    return b'|' as c_int;
                }

                0x5E => {
                    /* ^ */
                    jsY_next(J);
                    if jsY_accept(J, b'=' as c_int) {
                        return TK_XOR_ASS;
                    }
                    return b'^' as c_int;
                }

                EOF => return 0, /* EOF */

                _ => {}
            }

            /* Handle \uXXXX escapes in identifiers */
            jsY_unescape(J);
            if jsY_isidentifierstart((*J).lexchar) {
                textinit(J);
                textpush(J, (*J).lexchar);

                jsY_next(J);
                jsY_unescape(J);
                while jsY_isidentifierpart((*J).lexchar) {
                    textpush(J, (*J).lexchar);
                    jsY_next(J);
                    jsY_unescape(J);
                }

                textend(J);

                return jsY_findkeyword(J, (*J).lexbuf.text);
            }

            if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
                jsY_error!(J, c"unexpected character: '%c'", (*J).lexchar);
            }
            jsY_error!(J, c"unexpected character: \\u%04X", (*J).lexchar);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_initlex(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    unsafe {
        (*J).filename = filename;
        (*J).source = source;
        (*J).line = 1;
        (*J).lasttoken = 0;
        jsY_next(J); /* load first lookahead character */
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_lex(J: *mut js_State) -> c_int {
    unsafe {
        (*J).lasttoken = jsY_lexx(J);
        (*J).lasttoken
    }
}

unsafe fn lexjsonnumber(J: *mut js_State) -> c_int {
    unsafe {
        let s = (*J).source.offset(-1);

        if (*J).lexchar == b'-' as c_int {
            jsY_next(J);
        }

        if (*J).lexchar == b'0' as c_int {
            jsY_next(J);
        } else if (*J).lexchar >= b'1' as c_int && (*J).lexchar <= b'9' as c_int {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, c"unexpected non-digit");
        }

        if jsY_accept(J, b'.' as c_int) {
            if isdigit((*J).lexchar) {
                while isdigit((*J).lexchar) {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing digits after decimal point");
            }
        }

        if jsY_accept(J, b'e' as c_int) || jsY_accept(J, b'E' as c_int) {
            if (*J).lexchar == b'-' as c_int || (*J).lexchar == b'+' as c_int {
                jsY_next(J);
            }
            if isdigit((*J).lexchar) {
                while isdigit((*J).lexchar) {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing digits after exponent indicator");
            }
        }

        (*J).number = js_strtod(s, ptr::null_mut());
        TK_NUMBER
    }
}

unsafe fn lexjsonescape(J: *mut js_State) -> c_int {
    unsafe {
        let mut x = 0;

        /* already consumed '\' */

        match (*J).lexchar {
            0x75 /* u */ => {
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 12; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 8; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
                if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
                textpush(J, x);
            }
            0x22 => { textpush(J, b'"' as Rune); jsY_next(J); }
            0x5C => { textpush(J, b'\\' as Rune); jsY_next(J); }
            0x2F => { textpush(J, b'/' as Rune); jsY_next(J); }
            0x62 => { textpush(J, 8); jsY_next(J); }
            0x66 => { textpush(J, 12); jsY_next(J); }
            0x6E => { textpush(J, 10); jsY_next(J); }
            0x72 => { textpush(J, 13); jsY_next(J); }
            0x74 => { textpush(J, 9); jsY_next(J); }
            _ => jsY_error!(J, c"invalid escape sequence"),
        }
        0
    }
}

unsafe fn lexjsonstring(J: *mut js_State) -> c_int {
    unsafe {
        textinit(J);

        while (*J).lexchar != b'"' as c_int {
            if (*J).lexchar == EOF {
                jsY_error!(J, c"unterminated string");
            } else if (*J).lexchar < 32 {
                jsY_error!(J, c"invalid control character in string");
            } else if jsY_accept(J, b'\\' as c_int) {
                lexjsonescape(J);
            } else {
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, b'"' as c_int);

        let s = textend(J);

        (*J).text = s;
        TK_STRING
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_lexjson(J: *mut js_State) -> c_int {
    unsafe {
        loop {
            (*J).lexline = (*J).line; /* save location of beginning of token */

            while jsY_iswhite((*J).lexchar) != 0 || (*J).lexchar == b'\n' as c_int {
                jsY_next(J);
            }

            if ((*J).lexchar >= b'0' as c_int && (*J).lexchar <= b'9' as c_int)
                || (*J).lexchar == b'-' as c_int
            {
                return lexjsonnumber(J);
            }

            match (*J).lexchar {
                0x2C => {
                    jsY_next(J);
                    return 0x2C;
                }
                0x3A => {
                    jsY_next(J);
                    return 0x3A;
                }
                0x5B => {
                    jsY_next(J);
                    return 0x5B;
                }
                0x5D => {
                    jsY_next(J);
                    return 0x5D;
                }
                0x7B => {
                    jsY_next(J);
                    return 0x7B;
                }
                0x7D => {
                    jsY_next(J);
                    return 0x7D;
                }

                0x22 => {
                    jsY_next(J);
                    return lexjsonstring(J);
                }

                0x66 => {
                    /* f */
                    jsY_next(J);
                    jsY_expect!(J, b'a' as c_int);
                    jsY_expect!(J, b'l' as c_int);
                    jsY_expect!(J, b's' as c_int);
                    jsY_expect!(J, b'e' as c_int);
                    return TK_FALSE;
                }

                0x6E => {
                    /* n */
                    jsY_next(J);
                    jsY_expect!(J, b'u' as c_int);
                    jsY_expect!(J, b'l' as c_int);
                    jsY_expect!(J, b'l' as c_int);
                    return TK_NULL;
                }

                0x74 => {
                    /* t */
                    jsY_next(J);
                    jsY_expect!(J, b'r' as c_int);
                    jsY_expect!(J, b'u' as c_int);
                    jsY_expect!(J, b'e' as c_int);
                    return TK_TRUE;
                }

                EOF => return 0, /* EOF */

                _ => {}
            }

            if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
                jsY_error!(J, c"unexpected character: '%c'", (*J).lexchar);
            }
            jsY_error!(J, c"unexpected character: \\u%04X", (*J).lexchar);
        }
    }
}
