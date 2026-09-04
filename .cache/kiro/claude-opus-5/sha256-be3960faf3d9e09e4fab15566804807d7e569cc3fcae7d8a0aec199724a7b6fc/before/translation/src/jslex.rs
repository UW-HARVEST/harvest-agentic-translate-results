//! Translation of src/jslex.c: the lexer.
#![allow(unused_unsafe)]

use crate::jsi::*;
use crate::utf::{jsU_chartorune, jsU_isalpharune, jsU_runelen, jsU_runetochar};
use crate::jsdtoa::js_strtod;

/* C EOF */
const EOF: c_int = -1;

/* isalpharune from utf.h */
#[inline(always)]
unsafe fn isalpharune(c: c_int) -> c_int {
    unsafe { jsU_isalpharune(c) }
}

/* cross-module functions from jsrun / jsstate (owning modules per CONVENTIONS) */
unsafe extern "C-unwind" {
    fn js_malloc(J: *mut js_State, size: c_int) -> *mut c_void;
    fn js_realloc(J: *mut js_State, ptr: *mut c_void, size: c_int) -> *mut c_void;
    fn js_free(J: *mut js_State, ptr: *mut c_void);
    fn js_newsyntaxerror(J: *mut js_State, message: *const c_char);
    fn js_throw(J: *mut js_State) -> !;
}

/* ------------------------------------------------------------------ */
/* jsY_error: static, JS_NORETURN. The C body is:                      */
/*   char buf[512]; char msgbuf[256];                                  */
/*   vsnprintf(msgbuf, 256, fmt, ap);                                  */
/*   snprintf(buf, 256, "%s:%d: ", J->filename, J->lexline);           */
/*   strcat(buf, msgbuf);                                              */
/*   js_newsyntaxerror(J, buf); js_throw(J);                           */
/* The macro reproduces the msgbuf snprintf; jsY_error_msg the tail.   */
/* ------------------------------------------------------------------ */

unsafe fn jsY_error_msg(J: *mut js_State, msgbuf: *const c_char) -> ! {
    unsafe {
        let mut buf: [c_char; 512] = [0; 512];
        snprintf(
            buf.as_mut_ptr(),
            256,
            c"%s:%d: ".as_ptr(),
            (*J).filename,
            (*J).lexline,
        );
        strcat(buf.as_mut_ptr(), msgbuf);

        js_newsyntaxerror(J, buf.as_ptr());
        js_throw(J);
    }
}

macro_rules! jsY_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut b: [c_char; 256] = [0; 256];
        snprintf(b.as_mut_ptr(), 256, $fmt $(, $a)*);
        jsY_error_msg($J, b.as_ptr())
    }};
}

/* ------------------------------------------------------------------ */
/* tokenstring table (indexed by token). Stored as byte slices with an */
/* explicit trailing NUL; empty slice ("") represents a NULL entry.    */
/* ------------------------------------------------------------------ */

static TOKENSTRING: [&[u8]; 313] = [
    b"(end-of-file)\0",
    b"'\\x01'\0", b"'\\x02'\0", b"'\\x03'\0", b"'\\x04'\0", b"'\\x05'\0", b"'\\x06'\0", b"'\\x07'\0",
    b"'\\x08'\0", b"'\\x09'\0", b"'\\x0A'\0", b"'\\x0B'\0", b"'\\x0C'\0", b"'\\x0D'\0", b"'\\x0E'\0", b"'\\x0F'\0",
    b"'\\x10'\0", b"'\\x11'\0", b"'\\x12'\0", b"'\\x13'\0", b"'\\x14'\0", b"'\\x15'\0", b"'\\x16'\0", b"'\\x17'\0",
    b"'\\x18'\0", b"'\\x19'\0", b"'\\x1A'\0", b"'\\x1B'\0", b"'\\x1C'\0", b"'\\x1D'\0", b"'\\x1E'\0", b"'\\x1F'\0",
    b"' '\0", b"'!'\0", b"'\"'\0", b"'#'\0", b"'$'\0", b"'%'\0", b"'&'\0", b"'\\''\0",
    b"'('\0", b"')'\0", b"'*'\0", b"'+'\0", b"','\0", b"'-'\0", b"'.'\0", b"'/'\0",
    b"'0'\0", b"'1'\0", b"'2'\0", b"'3'\0", b"'4'\0", b"'5'\0", b"'6'\0", b"'7'\0",
    b"'8'\0", b"'9'\0", b"':'\0", b"';'\0", b"'<'\0", b"'='\0", b"'>'\0", b"'?'\0",
    b"'@'\0", b"'A'\0", b"'B'\0", b"'C'\0", b"'D'\0", b"'E'\0", b"'F'\0", b"'G'\0",
    b"'H'\0", b"'I'\0", b"'J'\0", b"'K'\0", b"'L'\0", b"'M'\0", b"'N'\0", b"'O'\0",
    b"'P'\0", b"'Q'\0", b"'R'\0", b"'S'\0", b"'T'\0", b"'U'\0", b"'V'\0", b"'W'\0",
    b"'X'\0", b"'Y'\0", b"'Z'\0", b"'['\0", b"''\0", b"']'\0", b"'^'\0", b"'_'\0",
    /* NOTE: index 92 is "''" (2 chars), reproducing the C source's "'\\'" literal,
       which is an escaped quote, not backslash-quote. Do not "fix". */
    b"'`'\0", b"'a'\0", b"'b'\0", b"'c'\0", b"'d'\0", b"'e'\0", b"'f'\0", b"'g'\0",
    b"'h'\0", b"'i'\0", b"'j'\0", b"'k'\0", b"'l'\0", b"'m'\0", b"'n'\0", b"'o'\0",
    b"'p'\0", b"'q'\0", b"'r'\0", b"'s'\0", b"'t'\0", b"'u'\0", b"'v'\0", b"'w'\0",
    b"'x'\0", b"'y'\0", b"'z'\0", b"'{'\0", b"'|'\0", b"'}'\0", b"'~'\0", b"'\\x7F'\0",

    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",
    b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"", b"",

    b"(identifier)\0", b"(number)\0", b"(string)\0", b"(regexp)\0",

    b"'<='\0", b"'>='\0", b"'=='\0", b"'!='\0", b"'==='\0", b"'!=='\0",
    b"'<<'\0", b"'>>'\0", b"'>>>'\0", b"'&&'\0", b"'||'\0",
    b"'+='\0", b"'-='\0", b"'*='\0", b"'/='\0", b"'%='\0",
    b"'<<='\0", b"'>>='\0", b"'>>>='\0", b"'&='\0", b"'|='\0", b"'^='\0",
    b"'++'\0", b"'--'\0",

    b"'break'\0", b"'case'\0", b"'catch'\0", b"'continue'\0", b"'debugger'\0",
    b"'default'\0", b"'delete'\0", b"'do'\0", b"'else'\0", b"'false'\0", b"'finally'\0", b"'for'\0",
    b"'function'\0", b"'if'\0", b"'in'\0", b"'instanceof'\0", b"'new'\0", b"'null'\0", b"'return'\0",
    b"'switch'\0", b"'this'\0", b"'throw'\0", b"'true'\0", b"'try'\0", b"'typeof'\0", b"'var'\0",
    b"'void'\0", b"'while'\0", b"'with'\0",
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_tokenstring(token: c_int) -> *const c_char {
    unsafe {
        if token >= 0 && token < TOKENSTRING.len() as c_int {
            let e = TOKENSTRING[token as usize];
            if !e.is_empty() {
                return e.as_ptr() as *const c_char;
            }
        }
        c"<unknown>".as_ptr()
    }
}

static keywords: [&[u8]; 29] = [
    b"break\0", b"case\0", b"catch\0", b"continue\0", b"debugger\0", b"default\0", b"delete\0",
    b"do\0", b"else\0", b"false\0", b"finally\0", b"for\0", b"function\0", b"if\0", b"in\0",
    b"instanceof\0", b"new\0", b"null\0", b"return\0", b"switch\0", b"this\0", b"throw\0",
    b"true\0", b"try\0", b"typeof\0", b"var\0", b"void\0", b"while\0", b"with\0",
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_findword(
    s: *const c_char,
    list: *const *const c_char,
    num: c_int,
) -> c_int {
    unsafe {
        let mut l: c_int = 0;
        let mut r: c_int = num - 1;
        while l <= r {
            let m: c_int = (l + r) >> 1;
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

unsafe fn jsY_findkeyword(J: *mut js_State, s: *const c_char) -> c_int {
    unsafe {
        /* build a *const *const c_char view of the keywords table */
        let mut list: [*const c_char; 29] = [core::ptr::null(); 29];
        let mut k = 0;
        while k < 29 {
            list[k] = keywords[k].as_ptr() as *const c_char;
            k += 1;
        }
        let i = jsY_findword(s, list.as_ptr(), 29);
        if i >= 0 {
            (*J).text = keywords[i as usize].as_ptr() as *const c_char;
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

/* isalpha(c) ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z')) */
#[inline(always)]
fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}
/* isdigit(c) (c >= '0' && c <= '9') */
#[inline(always)]
fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}
/* ishex(c) ((c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')) */
#[inline(always)]
fn ishex(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'f' as c_int) || (c >= 'A' as c_int && c <= 'F' as c_int)
}

unsafe fn jsY_isidentifierstart(c: c_int) -> c_int {
    unsafe {
        (isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0) as c_int
    }
}

unsafe fn jsY_isidentifierpart(c: c_int) -> c_int {
    unsafe {
        (isdigit(c) || isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0)
            as c_int
    }
}

unsafe fn jsY_isdec(c: c_int) -> c_int {
    isdigit(c) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_ishex(c: c_int) -> c_int {
    (isdigit(c) || ishex(c)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_tohex(c: c_int) -> c_int {
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    if c >= 'a' as c_int && c <= 'f' as c_int {
        return c - 'a' as c_int + 0xA;
    }
    if c >= 'A' as c_int && c <= 'F' as c_int {
        return c - 'A' as c_int + 0xA;
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
        if c == '\r' as Rune && *(*J).source == '\n' as c_char {
            (*J).source = (*J).source.offset(1);
        }
        if jsY_isnewline(c) != 0 {
            (*J).line += 1;
            c = '\n' as Rune;
        }
        (*J).lexchar = c;
    }
}

/* #define jsY_accept(J, x) (J->lexchar == x ? (jsY_next(J), 1) : 0) */
unsafe fn jsY_accept(J: *mut js_State, x: c_int) -> c_int {
    unsafe {
        if (*J).lexchar == x {
            jsY_next(J);
            1
        } else {
            0
        }
    }
}

/* #define jsY_expect(J, x) if (!jsY_accept(J, x)) jsY_error(J, "expected '%c'", x) */
macro_rules! jsY_expect {
    ($J:expr, $x:expr) => {
        if jsY_accept($J, $x) == 0 {
            jsY_error!($J, c"expected '%c'".as_ptr(), $x);
        }
    };
}

unsafe fn jsY_unescape(J: *mut js_State) {
    unsafe {
        if jsY_accept(J, '\\' as c_int) != 0 {
            'error: {
                if jsY_accept(J, 'u' as c_int) != 0 {
                    let mut x: c_int = 0;
                    if jsY_ishex((*J).lexchar) == 0 {
                        break 'error;
                    }
                    x |= jsY_tohex((*J).lexchar) << 12;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break 'error;
                    }
                    x |= jsY_tohex((*J).lexchar) << 8;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break 'error;
                    }
                    x |= jsY_tohex((*J).lexchar) << 4;
                    jsY_next(J);
                    if jsY_ishex((*J).lexchar) == 0 {
                        break 'error;
                    }
                    x |= jsY_tohex((*J).lexchar);
                    (*J).lexchar = x;
                    return;
                }
                /* fallthrough to error: label below */
            }
            /* error: */
            jsY_error!(J, c"unexpected escape sequence".as_ptr());
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
        let n: c_int;
        let newcap: c_int;
        if c == EOF {
            n = 1;
        } else {
            n = jsU_runelen(c);
        }
        if (*J).lexbuf.len + n > (*J).lexbuf.cap {
            newcap = (*J).lexbuf.cap * 2;
            (*J).lexbuf.text =
                js_realloc(J, (*J).lexbuf.text as *mut c_void, newcap) as *mut c_char;
            (*J).lexbuf.cap = newcap;
        }
        if c == EOF {
            *(*J).lexbuf.text.offset((*J).lexbuf.len as isize) = 0;
            (*J).lexbuf.len += 1;
        } else {
            let cc: Rune = c;
            (*J).lexbuf.len +=
                jsU_runetochar((*J).lexbuf.text.offset((*J).lexbuf.len as isize), &cc);
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
        while (*J).lexchar != EOF && (*J).lexchar != '\n' as c_int {
            jsY_next(J);
        }
    }
}

unsafe fn lexcomment(J: *mut js_State) -> c_int {
    unsafe {
        /* already consumed initial '/' '*' sequence */
        while (*J).lexchar != EOF {
            if jsY_accept(J, '*' as c_int) != 0 {
                while (*J).lexchar == '*' as c_int {
                    jsY_next(J);
                }
                if jsY_accept(J, '/' as c_int) != 0 {
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
            jsY_error!(J, c"malformed hexadecimal number".as_ptr());
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
        let s: *const c_char = (*J).source.offset(-1);

        if jsY_accept(J, '0' as c_int) != 0 {
            if jsY_accept(J, 'x' as c_int) != 0 || jsY_accept(J, 'X' as c_int) != 0 {
                (*J).number = lexhex(J);
                return TK_NUMBER;
            }
            if jsY_isdec((*J).lexchar) != 0 {
                jsY_error!(J, c"number with leading zero".as_ptr());
            }
            if jsY_accept(J, '.' as c_int) != 0 {
                while jsY_isdec((*J).lexchar) != 0 {
                    jsY_next(J);
                }
            }
        } else if jsY_accept(J, '.' as c_int) != 0 {
            if jsY_isdec((*J).lexchar) == 0 {
                return '.' as c_int;
            }
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        } else {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
            if jsY_accept(J, '.' as c_int) != 0 {
                while jsY_isdec((*J).lexchar) != 0 {
                    jsY_next(J);
                }
            }
        }

        if jsY_accept(J, 'e' as c_int) != 0 || jsY_accept(J, 'E' as c_int) != 0 {
            if (*J).lexchar == '-' as c_int || (*J).lexchar == '+' as c_int {
                jsY_next(J);
            }
            if jsY_isdec((*J).lexchar) != 0 {
                while jsY_isdec((*J).lexchar) != 0 {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing exponent".as_ptr());
            }
        }

        if jsY_isidentifierstart((*J).lexchar) != 0 {
            jsY_error!(J, c"number with letter suffix".as_ptr());
        }

        (*J).number = js_strtod(s, core::ptr::null_mut());
        TK_NUMBER
    }
}

unsafe fn lexescape(J: *mut js_State) -> c_int {
    unsafe {
        let mut x: c_int = 0;

        /* already consumed '\' */

        if jsY_accept(J, '\n' as c_int) != 0 {
            return 0;
        }

        /* switch (J->lexchar) with case EOF falling into case 'u' in C */
        let lc = (*J).lexchar;
        if lc == EOF {
            jsY_error!(J, c"unterminated escape sequence".as_ptr());
        } else if lc == 'u' as c_int {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 12;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 8;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 4;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x);
        } else if lc == 'x' as c_int {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 4;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x);
        } else if lc == '0' as c_int {
            textpush(J, 0);
            jsY_next(J);
        } else if lc == '\\' as c_int {
            textpush(J, '\\' as Rune);
            jsY_next(J);
        } else if lc == '\'' as c_int {
            textpush(J, '\'' as Rune);
            jsY_next(J);
        } else if lc == '"' as c_int {
            textpush(J, '"' as Rune);
            jsY_next(J);
        } else if lc == 'b' as c_int {
            textpush(J, 0x08 /* '\b' */);
            jsY_next(J);
        } else if lc == 'f' as c_int {
            textpush(J, 0x0C /* '\f' */);
            jsY_next(J);
        } else if lc == 'n' as c_int {
            textpush(J, '\n' as Rune);
            jsY_next(J);
        } else if lc == 'r' as c_int {
            textpush(J, '\r' as Rune);
            jsY_next(J);
        } else if lc == 't' as c_int {
            textpush(J, '\t' as Rune);
            jsY_next(J);
        } else if lc == 'v' as c_int {
            textpush(J, 0x0B /* '\v' */);
            jsY_next(J);
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
        0
    }
}

unsafe fn lexstring(J: *mut js_State) -> c_int {
    unsafe {
        let s: *const c_char;

        let q = (*J).lexchar;
        jsY_next(J);

        textinit(J);

        while (*J).lexchar != q {
            if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
                jsY_error!(J, c"string not terminated".as_ptr());
            }
            if jsY_accept(J, '\\' as c_int) != 0 {
                if lexescape(J) != 0 {
                    jsY_error!(J, c"malformed escape sequence".as_ptr());
                }
            } else {
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, q);

        s = textend(J);

        (*J).text = s;
        TK_STRING
    }
}

/* the ugliest language wart ever... */
unsafe fn isregexpcontext(last: c_int) -> c_int {
    if last == ']' as c_int
        || last == ')' as c_int
        || last == '}' as c_int
        || last == TK_IDENTIFIER
        || last == TK_NUMBER
        || last == TK_STRING
        || last == TK_FALSE
        || last == TK_NULL
        || last == TK_THIS
        || last == TK_TRUE
    {
        0
    } else {
        1
    }
}

unsafe fn lexregexp(J: *mut js_State) -> c_int {
    unsafe {
        let s: *const c_char;
        let mut g: c_int;
        let mut m: c_int;
        let mut i: c_int;
        let flags: c_int;
        let mut inclass: c_int = 0;

        /* already consumed initial '/' */

        textinit(J);

        /* regexp body */
        while (*J).lexchar != '/' as c_int || inclass != 0 {
            if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
                jsY_error!(J, c"regular expression not terminated".as_ptr());
            } else if jsY_accept(J, '\\' as c_int) != 0 {
                if jsY_accept(J, '/' as c_int) != 0 {
                    textpush(J, '/' as Rune);
                } else {
                    textpush(J, '\\' as Rune);
                    if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
                        jsY_error!(J, c"regular expression not terminated".as_ptr());
                    }
                    textpush(J, (*J).lexchar);
                    jsY_next(J);
                }
            } else {
                if (*J).lexchar == '[' as c_int && inclass == 0 {
                    inclass = 1;
                }
                if (*J).lexchar == ']' as c_int && inclass != 0 {
                    inclass = 0;
                }
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, '/' as c_int);

        s = textend(J);

        /* regexp flags */
        g = 0;
        i = 0;
        m = 0;

        while jsY_isidentifierpart((*J).lexchar) != 0 {
            if jsY_accept(J, 'g' as c_int) != 0 {
                g += 1;
            } else if jsY_accept(J, 'i' as c_int) != 0 {
                i += 1;
            } else if jsY_accept(J, 'm' as c_int) != 0 {
                m += 1;
            } else {
                jsY_error!(
                    J,
                    c"illegal flag in regular expression: %c".as_ptr(),
                    (*J).lexchar
                );
            }
        }

        if g > 1 || i > 1 || m > 1 {
            jsY_error!(J, c"duplicated flag in regular expression".as_ptr());
        }

        (*J).text = s;

        flags = 0
            | if g != 0 { JS_REGEXP_G } else { 0 }
            | if i != 0 { JS_REGEXP_I } else { 0 }
            | if m != 0 { JS_REGEXP_M } else { 0 };
        (*J).number = flags as f64;
        TK_REGEXP
    }
}

/* simple "return [no Line Terminator here] ..." contexts */
unsafe fn isnlthcontext(last: c_int) -> c_int {
    if last == TK_BREAK || last == TK_CONTINUE || last == TK_RETURN || last == TK_THROW {
        1
    } else {
        0
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

            if jsY_accept(J, '\n' as c_int) != 0 {
                (*J).newline = 1;
                if isnlthcontext((*J).lasttoken) != 0 {
                    return ';' as c_int;
                }
                continue;
            }

            if jsY_accept(J, '/' as c_int) != 0 {
                if jsY_accept(J, '/' as c_int) != 0 {
                    lexlinecomment(J);
                    continue;
                } else if jsY_accept(J, '*' as c_int) != 0 {
                    if lexcomment(J) != 0 {
                        jsY_error!(J, c"multi-line comment not terminated".as_ptr());
                    }
                    continue;
                } else if isregexpcontext((*J).lasttoken) != 0 {
                    return lexregexp(J);
                } else if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_DIV_ASS;
                } else {
                    return '/' as c_int;
                }
            }

            if (*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int {
                return lexnumber(J);
            }

            let lc = (*J).lexchar;
            if lc == '(' as c_int {
                jsY_next(J);
                return '(' as c_int;
            } else if lc == ')' as c_int {
                jsY_next(J);
                return ')' as c_int;
            } else if lc == ',' as c_int {
                jsY_next(J);
                return ',' as c_int;
            } else if lc == ':' as c_int {
                jsY_next(J);
                return ':' as c_int;
            } else if lc == ';' as c_int {
                jsY_next(J);
                return ';' as c_int;
            } else if lc == '?' as c_int {
                jsY_next(J);
                return '?' as c_int;
            } else if lc == '[' as c_int {
                jsY_next(J);
                return '[' as c_int;
            } else if lc == ']' as c_int {
                jsY_next(J);
                return ']' as c_int;
            } else if lc == '{' as c_int {
                jsY_next(J);
                return '{' as c_int;
            } else if lc == '}' as c_int {
                jsY_next(J);
                return '}' as c_int;
            } else if lc == '~' as c_int {
                jsY_next(J);
                return '~' as c_int;
            } else if lc == '\'' as c_int || lc == '"' as c_int {
                return lexstring(J);
            } else if lc == '.' as c_int {
                return lexnumber(J);
            } else if lc == '<' as c_int {
                jsY_next(J);
                if jsY_accept(J, '<' as c_int) != 0 {
                    if jsY_accept(J, '=' as c_int) != 0 {
                        return TK_SHL_ASS;
                    }
                    return TK_SHL;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_LE;
                }
                return '<' as c_int;
            } else if lc == '>' as c_int {
                jsY_next(J);
                if jsY_accept(J, '>' as c_int) != 0 {
                    if jsY_accept(J, '>' as c_int) != 0 {
                        if jsY_accept(J, '=' as c_int) != 0 {
                            return TK_USHR_ASS;
                        }
                        return TK_USHR;
                    }
                    if jsY_accept(J, '=' as c_int) != 0 {
                        return TK_SHR_ASS;
                    }
                    return TK_SHR;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_GE;
                }
                return '>' as c_int;
            } else if lc == '=' as c_int {
                jsY_next(J);
                if jsY_accept(J, '=' as c_int) != 0 {
                    if jsY_accept(J, '=' as c_int) != 0 {
                        return TK_STRICTEQ;
                    }
                    return TK_EQ;
                }
                return '=' as c_int;
            } else if lc == '!' as c_int {
                jsY_next(J);
                if jsY_accept(J, '=' as c_int) != 0 {
                    if jsY_accept(J, '=' as c_int) != 0 {
                        return TK_STRICTNE;
                    }
                    return TK_NE;
                }
                return '!' as c_int;
            } else if lc == '+' as c_int {
                jsY_next(J);
                if jsY_accept(J, '+' as c_int) != 0 {
                    return TK_INC;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_ADD_ASS;
                }
                return '+' as c_int;
            } else if lc == '-' as c_int {
                jsY_next(J);
                if jsY_accept(J, '-' as c_int) != 0 {
                    return TK_DEC;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_SUB_ASS;
                }
                return '-' as c_int;
            } else if lc == '*' as c_int {
                jsY_next(J);
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_MUL_ASS;
                }
                return '*' as c_int;
            } else if lc == '%' as c_int {
                jsY_next(J);
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_MOD_ASS;
                }
                return '%' as c_int;
            } else if lc == '&' as c_int {
                jsY_next(J);
                if jsY_accept(J, '&' as c_int) != 0 {
                    return TK_AND;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_AND_ASS;
                }
                return '&' as c_int;
            } else if lc == '|' as c_int {
                jsY_next(J);
                if jsY_accept(J, '|' as c_int) != 0 {
                    return TK_OR;
                }
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_OR_ASS;
                }
                return '|' as c_int;
            } else if lc == '^' as c_int {
                jsY_next(J);
                if jsY_accept(J, '=' as c_int) != 0 {
                    return TK_XOR_ASS;
                }
                return '^' as c_int;
            } else if lc == EOF {
                return 0; /* EOF */
            }

            /* Handle \uXXXX escapes in identifiers */
            jsY_unescape(J);
            if jsY_isidentifierstart((*J).lexchar) != 0 {
                textinit(J);
                textpush(J, (*J).lexchar);

                jsY_next(J);
                jsY_unescape(J);
                while jsY_isidentifierpart((*J).lexchar) != 0 {
                    textpush(J, (*J).lexchar);
                    jsY_next(J);
                    jsY_unescape(J);
                }

                textend(J);

                return jsY_findkeyword(J, (*J).lexbuf.text);
            }

            if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
                jsY_error!(J, c"unexpected character: '%c'".as_ptr(), (*J).lexchar);
            }
            jsY_error!(J, c"unexpected character: \\u%04X".as_ptr(), (*J).lexchar);
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
        let s: *const c_char = (*J).source.offset(-1);

        if (*J).lexchar == '-' as c_int {
            jsY_next(J);
        }

        if (*J).lexchar == '0' as c_int {
            jsY_next(J);
        } else if (*J).lexchar >= '1' as c_int && (*J).lexchar <= '9' as c_int {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, c"unexpected non-digit".as_ptr());
        }

        if jsY_accept(J, '.' as c_int) != 0 {
            if isdigit((*J).lexchar) {
                while isdigit((*J).lexchar) {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing digits after decimal point".as_ptr());
            }
        }

        if jsY_accept(J, 'e' as c_int) != 0 || jsY_accept(J, 'E' as c_int) != 0 {
            if (*J).lexchar == '-' as c_int || (*J).lexchar == '+' as c_int {
                jsY_next(J);
            }
            if isdigit((*J).lexchar) {
                while isdigit((*J).lexchar) {
                    jsY_next(J);
                }
            } else {
                jsY_error!(J, c"missing digits after exponent indicator".as_ptr());
            }
        }

        (*J).number = js_strtod(s, core::ptr::null_mut());
        TK_NUMBER
    }
}

unsafe fn lexjsonescape(J: *mut js_State) -> c_int {
    unsafe {
        let mut x: c_int = 0;

        /* already consumed '\' */

        let lc = (*J).lexchar;
        if lc == 'u' as c_int {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 12;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 8;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar) << 4;
                jsY_next(J);
            }
            if jsY_ishex((*J).lexchar) == 0 {
                return 1;
            } else {
                x |= jsY_tohex((*J).lexchar);
                jsY_next(J);
            }
            textpush(J, x);
        } else if lc == '"' as c_int {
            textpush(J, '"' as Rune);
            jsY_next(J);
        } else if lc == '\\' as c_int {
            textpush(J, '\\' as Rune);
            jsY_next(J);
        } else if lc == '/' as c_int {
            textpush(J, '/' as Rune);
            jsY_next(J);
        } else if lc == 'b' as c_int {
            textpush(J, 0x08 /* '\b' */);
            jsY_next(J);
        } else if lc == 'f' as c_int {
            textpush(J, 0x0C /* '\f' */);
            jsY_next(J);
        } else if lc == 'n' as c_int {
            textpush(J, '\n' as Rune);
            jsY_next(J);
        } else if lc == 'r' as c_int {
            textpush(J, '\r' as Rune);
            jsY_next(J);
        } else if lc == 't' as c_int {
            textpush(J, '\t' as Rune);
            jsY_next(J);
        } else {
            jsY_error!(J, c"invalid escape sequence".as_ptr());
        }
        0
    }
}

unsafe fn lexjsonstring(J: *mut js_State) -> c_int {
    unsafe {
        let s: *const c_char;

        textinit(J);

        while (*J).lexchar != '"' as c_int {
            if (*J).lexchar == EOF {
                jsY_error!(J, c"unterminated string".as_ptr());
            } else if (*J).lexchar < 32 {
                jsY_error!(J, c"invalid control character in string".as_ptr());
            } else if jsY_accept(J, '\\' as c_int) != 0 {
                lexjsonescape(J);
            } else {
                textpush(J, (*J).lexchar);
                jsY_next(J);
            }
        }
        jsY_expect!(J, '"' as c_int);

        s = textend(J);

        (*J).text = s;
        TK_STRING
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_lexjson(J: *mut js_State) -> c_int {
    unsafe {
        loop {
            (*J).lexline = (*J).line; /* save location of beginning of token */

            while jsY_iswhite((*J).lexchar) != 0 || (*J).lexchar == '\n' as c_int {
                jsY_next(J);
            }

            if ((*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int)
                || (*J).lexchar == '-' as c_int
            {
                return lexjsonnumber(J);
            }

            let lc = (*J).lexchar;
            if lc == ',' as c_int {
                jsY_next(J);
                return ',' as c_int;
            } else if lc == ':' as c_int {
                jsY_next(J);
                return ':' as c_int;
            } else if lc == '[' as c_int {
                jsY_next(J);
                return '[' as c_int;
            } else if lc == ']' as c_int {
                jsY_next(J);
                return ']' as c_int;
            } else if lc == '{' as c_int {
                jsY_next(J);
                return '{' as c_int;
            } else if lc == '}' as c_int {
                jsY_next(J);
                return '}' as c_int;
            } else if lc == '"' as c_int {
                jsY_next(J);
                return lexjsonstring(J);
            } else if lc == 'f' as c_int {
                jsY_next(J);
                jsY_expect!(J, 'a' as c_int);
                jsY_expect!(J, 'l' as c_int);
                jsY_expect!(J, 's' as c_int);
                jsY_expect!(J, 'e' as c_int);
                return TK_FALSE;
            } else if lc == 'n' as c_int {
                jsY_next(J);
                jsY_expect!(J, 'u' as c_int);
                jsY_expect!(J, 'l' as c_int);
                jsY_expect!(J, 'l' as c_int);
                return TK_NULL;
            } else if lc == 't' as c_int {
                jsY_next(J);
                jsY_expect!(J, 'r' as c_int);
                jsY_expect!(J, 'u' as c_int);
                jsY_expect!(J, 'e' as c_int);
                return TK_TRUE;
            } else if lc == EOF {
                return 0; /* EOF */
            }

            if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
                jsY_error!(J, c"unexpected character: '%c'".as_ptr(), (*J).lexchar);
            }
            jsY_error!(J, c"unexpected character: \\u%04X".as_ptr(), (*J).lexchar);
        }
    }
}
