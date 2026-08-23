//! Translation of `c_src/src/jslex.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsi::*;
use crate::jsrun::*;
use core::ptr::{null, null_mut};

use crate::jsdtoa::js_strtod;
use crate::utf::{chartorune, isalpharune, runelen, runetochar};

/* `EOF` from <stdio.h> */
const EOF: c_int = -1;

/// `JS_NORETURN static void jsY_error(js_State *J, const char *fmt, ...)`
macro_rules! jsY_error {
    ($J:expr, $fmt:expr $(, $a:expr)*) => {{
        let mut msgbuf = [0 as c_char; 256];
        let mut buf = [0 as c_char; 512];
        snprintf(msgbuf.as_mut_ptr(), 256, $fmt $(, $a)*);
        snprintf(buf.as_mut_ptr(), 256, c"%s:%d: ".as_ptr(), (*$J).filename, (*$J).lexline);
        strcat(buf.as_mut_ptr(), msgbuf.as_ptr());
        crate::jserror::js_newsyntaxerror($J, buf.as_ptr());
        crate::jsrun::js_throw($J)
    }};
}

/* ------------------------------------------------------------ tokenstring */

static TOKENSTRING: [Option<&core::ffi::CStr>; 313] = [
    Some(c"(end-of-file)"), Some(c"'\\x01'"), Some(c"'\\x02'"), Some(c"'\\x03'"), Some(c"'\\x04'"), Some(c"'\\x05'"), Some(c"'\\x06'"), Some(c"'\\x07'"),
    Some(c"'\\x08'"), Some(c"'\\x09'"), Some(c"'\\x0A'"), Some(c"'\\x0B'"), Some(c"'\\x0C'"), Some(c"'\\x0D'"), Some(c"'\\x0E'"), Some(c"'\\x0F'"),
    Some(c"'\\x10'"), Some(c"'\\x11'"), Some(c"'\\x12'"), Some(c"'\\x13'"), Some(c"'\\x14'"), Some(c"'\\x15'"), Some(c"'\\x16'"), Some(c"'\\x17'"),
    Some(c"'\\x18'"), Some(c"'\\x19'"), Some(c"'\\x1A'"), Some(c"'\\x1B'"), Some(c"'\\x1C'"), Some(c"'\\x1D'"), Some(c"'\\x1E'"), Some(c"'\\x1F'"),
    Some(c"' '"), Some(c"'!'"), Some(c"'\"'"), Some(c"'#'"), Some(c"'$'"), Some(c"'%'"), Some(c"'&'"), Some(c"'\\''"),
    Some(c"'('"), Some(c"')'"), Some(c"'*'"), Some(c"'+'"), Some(c"','"), Some(c"'-'"), Some(c"'.'"), Some(c"'/'"),
    Some(c"'0'"), Some(c"'1'"), Some(c"'2'"), Some(c"'3'"), Some(c"'4'"), Some(c"'5'"), Some(c"'6'"), Some(c"'7'"),
    Some(c"'8'"), Some(c"'9'"), Some(c"':'"), Some(c"';'"), Some(c"'<'"), Some(c"'='"), Some(c"'>'"), Some(c"'?'"),
    Some(c"'@'"), Some(c"'A'"), Some(c"'B'"), Some(c"'C'"), Some(c"'D'"), Some(c"'E'"), Some(c"'F'"), Some(c"'G'"),
    Some(c"'H'"), Some(c"'I'"), Some(c"'J'"), Some(c"'K'"), Some(c"'L'"), Some(c"'M'"), Some(c"'N'"), Some(c"'O'"),
    Some(c"'P'"), Some(c"'Q'"), Some(c"'R'"), Some(c"'S'"), Some(c"'T'"), Some(c"'U'"), Some(c"'V'"), Some(c"'W'"),
    Some(c"'X'"), Some(c"'Y'"), Some(c"'Z'"), Some(c"'['"), Some(c"'\'"), Some(c"']'"), Some(c"'^'"), Some(c"'_'"),
    Some(c"'`'"), Some(c"'a'"), Some(c"'b'"), Some(c"'c'"), Some(c"'d'"), Some(c"'e'"), Some(c"'f'"), Some(c"'g'"),
    Some(c"'h'"), Some(c"'i'"), Some(c"'j'"), Some(c"'k'"), Some(c"'l'"), Some(c"'m'"), Some(c"'n'"), Some(c"'o'"),
    Some(c"'p'"), Some(c"'q'"), Some(c"'r'"), Some(c"'s'"), Some(c"'t'"), Some(c"'u'"), Some(c"'v'"), Some(c"'w'"),
    Some(c"'x'"), Some(c"'y'"), Some(c"'z'"), Some(c"'{'"), Some(c"'|'"), Some(c"'}'"), Some(c"'~'"), Some(c"'\\x7F'"),
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    None, None, None, None, None, None, None, None,
    Some(c"(identifier)"), Some(c"(number)"), Some(c"(string)"), Some(c"(regexp)"), Some(c"'<='"), Some(c"'>='"), Some(c"'=='"), Some(c"'!='"),
    Some(c"'==='"), Some(c"'!=='"), Some(c"'<<'"), Some(c"'>>'"), Some(c"'>>>'"), Some(c"'&&'"), Some(c"'||'"), Some(c"'+='"),
    Some(c"'-='"), Some(c"'*='"), Some(c"'/='"), Some(c"'%='"), Some(c"'<<='"), Some(c"'>>='"), Some(c"'>>>='"), Some(c"'&='"),
    Some(c"'|='"), Some(c"'^='"), Some(c"'++'"), Some(c"'--'"), Some(c"'break'"), Some(c"'case'"), Some(c"'catch'"), Some(c"'continue'"),
    Some(c"'debugger'"), Some(c"'default'"), Some(c"'delete'"), Some(c"'do'"), Some(c"'else'"), Some(c"'false'"), Some(c"'finally'"), Some(c"'for'"),
    Some(c"'function'"), Some(c"'if'"), Some(c"'in'"), Some(c"'instanceof'"), Some(c"'new'"), Some(c"'null'"), Some(c"'return'"), Some(c"'switch'"),
    Some(c"'this'"), Some(c"'throw'"), Some(c"'true'"), Some(c"'try'"), Some(c"'typeof'"), Some(c"'var'"), Some(c"'void'"), Some(c"'while'"),
    Some(c"'with'"),
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_tokenstring(token: c_int) -> *const c_char {
    if token >= 0 && token < TOKENSTRING.len() as c_int {
        if let Some(s) = TOKENSTRING[token as usize] {
            return s.as_ptr();
        }
    }
    c"<unknown>".as_ptr()
}

/* --------------------------------------------------------------- keywords */

const NKEYWORDS: usize = 29;

static KEYWORDS: [&core::ffi::CStr; NKEYWORDS] = [
    c"break", c"case", c"catch", c"continue", c"debugger", c"default", c"delete",
    c"do", c"else", c"false", c"finally", c"for", c"function", c"if", c"in",
    c"instanceof", c"new", c"null", c"return", c"switch", c"this", c"throw",
    c"true", c"try", c"typeof", c"var", c"void", c"while", c"with",
];

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_findword(
    s: *const c_char,
    list: *const *const c_char,
    num: c_int,
) -> c_int {
    let mut l: c_int = 0;
    let mut r: c_int = num - 1;
    while l <= r {
        let m: c_int = (l + r) >> 1;
        let c: c_int = strcmp(s, *list.offset(m as isize));
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

unsafe fn jsY_findkeyword(J: *mut js_State, s: *const c_char) -> c_int {
    let keywords: [*const c_char; NKEYWORDS] = core::array::from_fn(|i| KEYWORDS[i].as_ptr());
    let i = jsY_findword(s, keywords.as_ptr(), NKEYWORDS as c_int);
    if i >= 0 {
        (*J).text = keywords[i as usize];
        return TK_BREAK + i; /* first keyword + i */
    }
    (*J).text = s;
    TK_IDENTIFIER
}

/* ------------------------------------------------------------ char classes */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_iswhite(c: c_int) -> c_int {
    (c == 0x9 || c == 0xB || c == 0xC || c == 0x20 || c == 0xA0 || c == 0xFEFF) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

/* `#define isalpha(c) ((c >= 'a' && c <= 'z') || (c >= 'A' && c <= 'Z'))` */
#[inline]
fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}

/* `#define isdigit(c) (c >= '0' && c <= '9')` */
#[inline]
fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}

/* `#define ishex(c) ((c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F'))` */
#[inline]
fn ishex(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'f' as c_int) || (c >= 'A' as c_int && c <= 'F' as c_int)
}

unsafe fn jsY_isidentifierstart(c: c_int) -> c_int {
    (isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isidentifierpart(c: c_int) -> c_int {
    (isdigit(c) || isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0)
        as c_int
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

/* ------------------------------------------------------------- next / peek */

unsafe fn jsY_next(J: *mut js_State) {
    let mut c: Rune = 0;
    if *(*J).source == 0 {
        (*J).lexchar = EOF;
        return;
    }
    (*J).source = (*J).source.offset(chartorune(&mut c, (*J).source) as isize);
    /* consume CR LF as one unit */
    if c == '\r' as c_int && *(*J).source == '\n' as c_char {
        (*J).source = (*J).source.offset(1);
    }
    if jsY_isnewline(c) != 0 {
        (*J).line += 1;
        c = '\n' as c_int;
    }
    (*J).lexchar = c;
}

/// `#define jsY_accept(J, x) (J->lexchar == x ? (jsY_next(J), 1) : 0)`
macro_rules! jsY_accept {
    ($J:expr, $x:expr) => {
        (if (*$J).lexchar == ($x as c_int) {
            jsY_next($J);
            1
        } else {
            0
        })
    };
}

/// `#define jsY_expect(J, x) if (!jsY_accept(J, x)) jsY_error(J, "expected '%c'", x)`
macro_rules! jsY_expect {
    ($J:expr, $x:expr) => {
        if jsY_accept!($J, $x) == 0 {
            jsY_error!($J, c"expected '%c'".as_ptr(), $x as c_int)
        }
    };
}

unsafe fn jsY_unescape(J: *mut js_State) {
    if jsY_accept!(J, '\\') != 0 {
        'error: {
            if jsY_accept!(J, 'u') != 0 {
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
        }
        /* error: */
        jsY_error!(J, c"unexpected escape sequence".as_ptr());
    }
}

/* ------------------------------------------------------------- text buffer */

unsafe fn textinit(J: *mut js_State) {
    if (*J).lexbuf.text.is_null() {
        (*J).lexbuf.cap = 4096;
        (*J).lexbuf.text = js_malloc(J, (*J).lexbuf.cap) as *mut c_char;
    }
    (*J).lexbuf.len = 0;
}

unsafe fn textpush(J: *mut js_State, c: Rune) {
    let n: c_int;
    let newcap: c_int;
    if c == EOF {
        n = 1;
    } else {
        n = runelen(c);
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
            runetochar((*J).lexbuf.text.offset((*J).lexbuf.len as isize), &cc);
    }
}

unsafe fn textend(J: *mut js_State) -> *mut c_char {
    textpush(J, EOF);
    (*J).lexbuf.text
}

/* ---------------------------------------------------------------- comments */

unsafe fn lexlinecomment(J: *mut js_State) {
    while (*J).lexchar != EOF && (*J).lexchar != '\n' as c_int {
        jsY_next(J);
    }
}

unsafe fn lexcomment(J: *mut js_State) -> c_int {
    /* already consumed initial '/' '*' sequence */
    while (*J).lexchar != EOF {
        if jsY_accept!(J, '*') != 0 {
            while (*J).lexchar == '*' as c_int {
                jsY_next(J);
            }
            if jsY_accept!(J, '/') != 0 {
                return 0;
            }
        } else {
            jsY_next(J);
        }
    }
    -1
}

/* ----------------------------------------------------------------- numbers */

unsafe fn lexhex(J: *mut js_State) -> f64 {
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

unsafe fn lexnumber(J: *mut js_State) -> c_int {
    let s: *const c_char = (*J).source.offset(-1);

    if jsY_accept!(J, '0') != 0 {
        if jsY_accept!(J, 'x') != 0 || jsY_accept!(J, 'X') != 0 {
            (*J).number = lexhex(J);
            return TK_NUMBER;
        }
        if jsY_isdec((*J).lexchar) != 0 {
            jsY_error!(J, c"number with leading zero".as_ptr());
        }
        if jsY_accept!(J, '.') != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    } else if jsY_accept!(J, '.') != 0 {
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
        if jsY_accept!(J, '.') != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    }

    if jsY_accept!(J, 'e') != 0 || jsY_accept!(J, 'E') != 0 {
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

    (*J).number = js_strtod(s, null_mut());
    TK_NUMBER
}

/* ----------------------------------------------------------------- strings */

unsafe fn lexescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    /* already consumed '\' */

    if jsY_accept!(J, '\n') != 0 {
        return 0;
    }

    let c = (*J).lexchar;
    if c == EOF {
        jsY_error!(J, c"unterminated escape sequence".as_ptr());
    } else if c == 'u' as c_int {
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
    } else if c == 'x' as c_int {
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
    } else if c == '0' as c_int {
        textpush(J, 0);
        jsY_next(J);
    } else if c == '\\' as c_int {
        textpush(J, '\\' as c_int);
        jsY_next(J);
    } else if c == '\'' as c_int {
        textpush(J, '\'' as c_int);
        jsY_next(J);
    } else if c == '"' as c_int {
        textpush(J, '"' as c_int);
        jsY_next(J);
    } else if c == 'b' as c_int {
        textpush(J, 8); /* '\b' */
        jsY_next(J);
    } else if c == 'f' as c_int {
        textpush(J, 12); /* '\f' */
        jsY_next(J);
    } else if c == 'n' as c_int {
        textpush(J, 10); /* '\n' */
        jsY_next(J);
    } else if c == 'r' as c_int {
        textpush(J, 13); /* '\r' */
        jsY_next(J);
    } else if c == 't' as c_int {
        textpush(J, 9); /* '\t' */
        jsY_next(J);
    } else if c == 'v' as c_int {
        textpush(J, 11); /* '\v' */
        jsY_next(J);
    } else {
        /* default */
        textpush(J, (*J).lexchar);
        jsY_next(J);
    }
    0
}

unsafe fn lexstring(J: *mut js_State) -> c_int {
    let s: *const c_char;

    let q: c_int = (*J).lexchar;
    jsY_next(J);

    textinit(J);

    while (*J).lexchar != q {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
            jsY_error!(J, c"string not terminated".as_ptr());
        }
        if jsY_accept!(J, '\\') != 0 {
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

/* ----------------------------------------------------------------- regexps */

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
        return 0;
    }
    1
}

unsafe fn lexregexp(J: *mut js_State) -> c_int {
    let s: *const c_char;
    let mut g: c_int;
    let mut m: c_int;
    let mut i: c_int;
    let mut flags: c_int;
    let mut inclass: c_int = 0;

    /* already consumed initial '/' */

    textinit(J);

    /* regexp body */
    while (*J).lexchar != '/' as c_int || inclass != 0 {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
            jsY_error!(J, c"regular expression not terminated".as_ptr());
        } else if jsY_accept!(J, '\\') != 0 {
            if jsY_accept!(J, '/') != 0 {
                textpush(J, '/' as c_int);
            } else {
                textpush(J, '\\' as c_int);
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
    jsY_expect!(J, '/');

    s = textend(J);

    /* regexp flags */
    m = 0;
    i = m;
    g = i;

    while jsY_isidentifierpart((*J).lexchar) != 0 {
        if jsY_accept!(J, 'g') != 0 {
            g += 1;
        } else if jsY_accept!(J, 'i') != 0 {
            i += 1;
        } else if jsY_accept!(J, 'm') != 0 {
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

/* simple "return [no Line Terminator here] ..." contexts */
unsafe fn isnlthcontext(last: c_int) -> c_int {
    if last == TK_BREAK || last == TK_CONTINUE || last == TK_RETURN || last == TK_THROW {
        return 1;
    }
    0
}

/* ------------------------------------------------------------- main lexer */

unsafe fn jsY_lexx(J: *mut js_State) -> c_int {
    (*J).newline = 0;

    loop {
        (*J).lexline = (*J).line; /* save location of beginning of token */

        while jsY_iswhite((*J).lexchar) != 0 {
            jsY_next(J);
        }

        if jsY_accept!(J, '\n') != 0 {
            (*J).newline = 1;
            if isnlthcontext((*J).lasttoken) != 0 {
                return ';' as c_int;
            }
            continue;
        }

        if jsY_accept!(J, '/') != 0 {
            if jsY_accept!(J, '/') != 0 {
                lexlinecomment(J);
                continue;
            } else if jsY_accept!(J, '*') != 0 {
                if lexcomment(J) != 0 {
                    jsY_error!(J, c"multi-line comment not terminated".as_ptr());
                }
                continue;
            } else if isregexpcontext((*J).lasttoken) != 0 {
                return lexregexp(J);
            } else if jsY_accept!(J, '=') != 0 {
                return TK_DIV_ASS;
            } else {
                return '/' as c_int;
            }
        }

        if (*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int {
            return lexnumber(J);
        }

        /* switch (J->lexchar) */
        let c = (*J).lexchar;
        if c == '(' as c_int {
            jsY_next(J);
            return '(' as c_int;
        } else if c == ')' as c_int {
            jsY_next(J);
            return ')' as c_int;
        } else if c == ',' as c_int {
            jsY_next(J);
            return ',' as c_int;
        } else if c == ':' as c_int {
            jsY_next(J);
            return ':' as c_int;
        } else if c == ';' as c_int {
            jsY_next(J);
            return ';' as c_int;
        } else if c == '?' as c_int {
            jsY_next(J);
            return '?' as c_int;
        } else if c == '[' as c_int {
            jsY_next(J);
            return '[' as c_int;
        } else if c == ']' as c_int {
            jsY_next(J);
            return ']' as c_int;
        } else if c == '{' as c_int {
            jsY_next(J);
            return '{' as c_int;
        } else if c == '}' as c_int {
            jsY_next(J);
            return '}' as c_int;
        } else if c == '~' as c_int {
            jsY_next(J);
            return '~' as c_int;
        } else if c == '\'' as c_int || c == '"' as c_int {
            return lexstring(J);
        } else if c == '.' as c_int {
            return lexnumber(J);
        } else if c == '<' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '<') != 0 {
                if jsY_accept!(J, '=') != 0 {
                    return TK_SHL_ASS;
                }
                return TK_SHL;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_LE;
            }
            return '<' as c_int;
        } else if c == '>' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '>') != 0 {
                if jsY_accept!(J, '>') != 0 {
                    if jsY_accept!(J, '=') != 0 {
                        return TK_USHR_ASS;
                    }
                    return TK_USHR;
                }
                if jsY_accept!(J, '=') != 0 {
                    return TK_SHR_ASS;
                }
                return TK_SHR;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_GE;
            }
            return '>' as c_int;
        } else if c == '=' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                if jsY_accept!(J, '=') != 0 {
                    return TK_STRICTEQ;
                }
                return TK_EQ;
            }
            return '=' as c_int;
        } else if c == '!' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                if jsY_accept!(J, '=') != 0 {
                    return TK_STRICTNE;
                }
                return TK_NE;
            }
            return '!' as c_int;
        } else if c == '+' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '+') != 0 {
                return TK_INC;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_ADD_ASS;
            }
            return '+' as c_int;
        } else if c == '-' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '-') != 0 {
                return TK_DEC;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_SUB_ASS;
            }
            return '-' as c_int;
        } else if c == '*' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                return TK_MUL_ASS;
            }
            return '*' as c_int;
        } else if c == '%' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                return TK_MOD_ASS;
            }
            return '%' as c_int;
        } else if c == '&' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '&') != 0 {
                return TK_AND;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_AND_ASS;
            }
            return '&' as c_int;
        } else if c == '|' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '|') != 0 {
                return TK_OR;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_OR_ASS;
            }
            return '|' as c_int;
        } else if c == '^' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                return TK_XOR_ASS;
            }
            return '^' as c_int;
        } else if c == EOF {
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
            jsY_error!(
                J,
                c"unexpected character: '%c'".as_ptr(),
                (*J).lexchar
            );
        }
        jsY_error!(
            J,
            c"unexpected character: \\u%04X".as_ptr(),
            (*J).lexchar
        );
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_initlex(
    J: *mut js_State,
    filename: *const c_char,
    source: *const c_char,
) {
    (*J).filename = filename;
    (*J).source = source;
    (*J).line = 1;
    (*J).lasttoken = 0;
    jsY_next(J); /* load first lookahead character */
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_lex(J: *mut js_State) -> c_int {
    (*J).lasttoken = jsY_lexx(J);
    (*J).lasttoken
}

/* -------------------------------------------------------------- JSON lexer */

unsafe fn lexjsonnumber(J: *mut js_State) -> c_int {
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

    if jsY_accept!(J, '.') != 0 {
        if isdigit((*J).lexchar) {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, c"missing digits after decimal point".as_ptr());
        }
    }

    if jsY_accept!(J, 'e') != 0 || jsY_accept!(J, 'E') != 0 {
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

    (*J).number = js_strtod(s, null_mut());
    TK_NUMBER
}

unsafe fn lexjsonescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    /* already consumed '\' */

    let c = (*J).lexchar;
    if c == 'u' as c_int {
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
    } else if c == '"' as c_int {
        textpush(J, '"' as c_int);
        jsY_next(J);
    } else if c == '\\' as c_int {
        textpush(J, '\\' as c_int);
        jsY_next(J);
    } else if c == '/' as c_int {
        textpush(J, '/' as c_int);
        jsY_next(J);
    } else if c == 'b' as c_int {
        textpush(J, 8); /* '\b' */
        jsY_next(J);
    } else if c == 'f' as c_int {
        textpush(J, 12); /* '\f' */
        jsY_next(J);
    } else if c == 'n' as c_int {
        textpush(J, 10); /* '\n' */
        jsY_next(J);
    } else if c == 'r' as c_int {
        textpush(J, 13); /* '\r' */
        jsY_next(J);
    } else if c == 't' as c_int {
        textpush(J, 9); /* '\t' */
        jsY_next(J);
    } else {
        /* default */
        jsY_error!(J, c"invalid escape sequence".as_ptr());
    }
    0
}

unsafe fn lexjsonstring(J: *mut js_State) -> c_int {
    let s: *const c_char;

    textinit(J);

    while (*J).lexchar != '"' as c_int {
        if (*J).lexchar == EOF {
            jsY_error!(J, c"unterminated string".as_ptr());
        } else if (*J).lexchar < 32 {
            jsY_error!(J, c"invalid control character in string".as_ptr());
        } else if jsY_accept!(J, '\\') != 0 {
            lexjsonescape(J);
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    jsY_expect!(J, '"');

    s = textend(J);

    (*J).text = s;
    TK_STRING
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsY_lexjson(J: *mut js_State) -> c_int {
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

        /* switch (J->lexchar) */
        let c = (*J).lexchar;
        if c == ',' as c_int {
            jsY_next(J);
            return ',' as c_int;
        } else if c == ':' as c_int {
            jsY_next(J);
            return ':' as c_int;
        } else if c == '[' as c_int {
            jsY_next(J);
            return '[' as c_int;
        } else if c == ']' as c_int {
            jsY_next(J);
            return ']' as c_int;
        } else if c == '{' as c_int {
            jsY_next(J);
            return '{' as c_int;
        } else if c == '}' as c_int {
            jsY_next(J);
            return '}' as c_int;
        } else if c == '"' as c_int {
            jsY_next(J);
            return lexjsonstring(J);
        } else if c == 'f' as c_int {
            jsY_next(J);
            jsY_expect!(J, 'a');
            jsY_expect!(J, 'l');
            jsY_expect!(J, 's');
            jsY_expect!(J, 'e');
            return TK_FALSE;
        } else if c == 'n' as c_int {
            jsY_next(J);
            jsY_expect!(J, 'u');
            jsY_expect!(J, 'l');
            jsY_expect!(J, 'l');
            return TK_NULL;
        } else if c == 't' as c_int {
            jsY_next(J);
            jsY_expect!(J, 'r');
            jsY_expect!(J, 'u');
            jsY_expect!(J, 'e');
            return TK_TRUE;
        } else if c == EOF {
            return 0; /* EOF */
        }

        if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
            jsY_error!(
                J,
                c"unexpected character: '%c'".as_ptr(),
                (*J).lexchar
            );
        }
        jsY_error!(
            J,
            c"unexpected character: \\u%04X".as_ptr(),
            (*J).lexchar
        );
    }
}
