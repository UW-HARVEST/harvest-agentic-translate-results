//! Translation of jslex.c

use crate::*;

const EOF: c_int = -1;

/* ------------------------------------------------------------- errors ---- */

/// Non-variadic core of the `static void jsY_error(js_State *, const char *, ...)`
/// helper of jslex.c: takes the already formatted message.
unsafe fn jsY_error_str(J: *mut js_State, msgbuf: *const c_char) -> ! {
    let mut buf: [c_char; 512] = [0; 512];

    snprintf(
        buf.as_mut_ptr(),
        256,
        cs!("%s:%d: "),
        (*J).filename,
        (*J).lexline,
    );
    strcat(buf.as_mut_ptr(), msgbuf);

    crate::jserror::js_newsyntaxerror(J, buf.as_ptr());
    js_throw(J)
}

macro_rules! jsY_error {
    ($J:expr, $fmt:expr) => {{
        let mut __msgbuf: [c_char; 256] = [0; 256];
        snprintf(__msgbuf.as_mut_ptr(), 256, cs!($fmt));
        jsY_error_str($J, __msgbuf.as_ptr())
    }};
    ($J:expr, $fmt:expr $(, $a:expr)+) => {{
        let mut __msgbuf: [c_char; 256] = [0; 256];
        snprintf(__msgbuf.as_mut_ptr(), 256, cs!($fmt) $(, $a)+);
        jsY_error_str($J, __msgbuf.as_ptr())
    }};
}

/* #define jsY_accept(J, x) (J->lexchar == x ? (jsY_next(J), 1) : 0) */
macro_rules! jsY_accept {
    ($J:expr, $x:expr) => {
        if (*$J).lexchar == $x as c_int {
            jsY_next($J);
            1
        } else {
            0
        }
    };
}

/* #define jsY_expect(J, x) if (!jsY_accept(J, x)) jsY_error(J, "expected '%c'", x) */
macro_rules! jsY_expect {
    ($J:expr, $x:expr) => {
        if jsY_accept!($J, $x) == 0 {
            jsY_error!($J, "expected '%c'", $x as c_int);
        }
    };
}

/* ------------------------------------------------------- token strings ---- */

static tokenstring: [Option<&str>; 313] = [
    Some("(end-of-file)\0"),
    Some("'\\x01'\0"),
    Some("'\\x02'\0"),
    Some("'\\x03'\0"),
    Some("'\\x04'\0"),
    Some("'\\x05'\0"),
    Some("'\\x06'\0"),
    Some("'\\x07'\0"),
    Some("'\\x08'\0"),
    Some("'\\x09'\0"),
    Some("'\\x0A'\0"),
    Some("'\\x0B'\0"),
    Some("'\\x0C'\0"),
    Some("'\\x0D'\0"),
    Some("'\\x0E'\0"),
    Some("'\\x0F'\0"),
    Some("'\\x10'\0"),
    Some("'\\x11'\0"),
    Some("'\\x12'\0"),
    Some("'\\x13'\0"),
    Some("'\\x14'\0"),
    Some("'\\x15'\0"),
    Some("'\\x16'\0"),
    Some("'\\x17'\0"),
    Some("'\\x18'\0"),
    Some("'\\x19'\0"),
    Some("'\\x1A'\0"),
    Some("'\\x1B'\0"),
    Some("'\\x1C'\0"),
    Some("'\\x1D'\0"),
    Some("'\\x1E'\0"),
    Some("'\\x1F'\0"),
    Some("' '\0"),
    Some("'!'\0"),
    Some("'\"'\0"),
    Some("'#'\0"),
    Some("'$'\0"),
    Some("'%'\0"),
    Some("'&'\0"),
    Some("'\\''\0"),
    Some("'('\0"),
    Some("')'\0"),
    Some("'*'\0"),
    Some("'+'\0"),
    Some("','\0"),
    Some("'-'\0"),
    Some("'.'\0"),
    Some("'/'\0"),
    Some("'0'\0"),
    Some("'1'\0"),
    Some("'2'\0"),
    Some("'3'\0"),
    Some("'4'\0"),
    Some("'5'\0"),
    Some("'6'\0"),
    Some("'7'\0"),
    Some("'8'\0"),
    Some("'9'\0"),
    Some("':'\0"),
    Some("';'\0"),
    Some("'<'\0"),
    Some("'='\0"),
    Some("'>'\0"),
    Some("'?'\0"),
    Some("'@'\0"),
    Some("'A'\0"),
    Some("'B'\0"),
    Some("'C'\0"),
    Some("'D'\0"),
    Some("'E'\0"),
    Some("'F'\0"),
    Some("'G'\0"),
    Some("'H'\0"),
    Some("'I'\0"),
    Some("'J'\0"),
    Some("'K'\0"),
    Some("'L'\0"),
    Some("'M'\0"),
    Some("'N'\0"),
    Some("'O'\0"),
    Some("'P'\0"),
    Some("'Q'\0"),
    Some("'R'\0"),
    Some("'S'\0"),
    Some("'T'\0"),
    Some("'U'\0"),
    Some("'V'\0"),
    Some("'W'\0"),
    Some("'X'\0"),
    Some("'Y'\0"),
    Some("'Z'\0"),
    Some("'['\0"),
    /* the C source writes "'\'" here, which the compiler folds to "''" */
    Some("''\0"),
    Some("']'\0"),
    Some("'^'\0"),
    Some("'_'\0"),
    Some("'`'\0"),
    Some("'a'\0"),
    Some("'b'\0"),
    Some("'c'\0"),
    Some("'d'\0"),
    Some("'e'\0"),
    Some("'f'\0"),
    Some("'g'\0"),
    Some("'h'\0"),
    Some("'i'\0"),
    Some("'j'\0"),
    Some("'k'\0"),
    Some("'l'\0"),
    Some("'m'\0"),
    Some("'n'\0"),
    Some("'o'\0"),
    Some("'p'\0"),
    Some("'q'\0"),
    Some("'r'\0"),
    Some("'s'\0"),
    Some("'t'\0"),
    Some("'u'\0"),
    Some("'v'\0"),
    Some("'w'\0"),
    Some("'x'\0"),
    Some("'y'\0"),
    Some("'z'\0"),
    Some("'{'\0"),
    Some("'|'\0"),
    Some("'}'\0"),
    Some("'~'\0"),
    Some("'\\x7F'\0"),
    /* 128 NULL entries (0x80 .. 0xFF) */
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
    Some("(identifier)\0"),
    Some("(number)\0"),
    Some("(string)\0"),
    Some("(regexp)\0"),
    Some("'<='\0"),
    Some("'>='\0"),
    Some("'=='\0"),
    Some("'!='\0"),
    Some("'==='\0"),
    Some("'!=='\0"),
    Some("'<<'\0"),
    Some("'>>'\0"),
    Some("'>>>'\0"),
    Some("'&&'\0"),
    Some("'||'\0"),
    Some("'+='\0"),
    Some("'-='\0"),
    Some("'*='\0"),
    Some("'/='\0"),
    Some("'%='\0"),
    Some("'<<='\0"),
    Some("'>>='\0"),
    Some("'>>>='\0"),
    Some("'&='\0"),
    Some("'|='\0"),
    Some("'^='\0"),
    Some("'++'\0"),
    Some("'--'\0"),
    Some("'break'\0"),
    Some("'case'\0"),
    Some("'catch'\0"),
    Some("'continue'\0"),
    Some("'debugger'\0"),
    Some("'default'\0"),
    Some("'delete'\0"),
    Some("'do'\0"),
    Some("'else'\0"),
    Some("'false'\0"),
    Some("'finally'\0"),
    Some("'for'\0"),
    Some("'function'\0"),
    Some("'if'\0"),
    Some("'in'\0"),
    Some("'instanceof'\0"),
    Some("'new'\0"),
    Some("'null'\0"),
    Some("'return'\0"),
    Some("'switch'\0"),
    Some("'this'\0"),
    Some("'throw'\0"),
    Some("'true'\0"),
    Some("'try'\0"),
    Some("'typeof'\0"),
    Some("'var'\0"),
    Some("'void'\0"),
    Some("'while'\0"),
    Some("'with'\0"),
];

/* the operator/keyword entries must line up with the token numbers */
const _: () = assert!(tokenstring.len() == 313);

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_tokenstring(token: c_int) -> *const c_char {
    if token >= 0 && token < tokenstring.len() as c_int {
        if let Some(s) = tokenstring[token as usize] {
            return s.as_ptr() as *const c_char;
        }
    }
    cs!("<unknown>")
}

static keywords: [&str; 29] = [
    "break\0",
    "case\0",
    "catch\0",
    "continue\0",
    "debugger\0",
    "default\0",
    "delete\0",
    "do\0",
    "else\0",
    "false\0",
    "finally\0",
    "for\0",
    "function\0",
    "if\0",
    "in\0",
    "instanceof\0",
    "new\0",
    "null\0",
    "return\0",
    "switch\0",
    "this\0",
    "throw\0",
    "true\0",
    "try\0",
    "typeof\0",
    "var\0",
    "void\0",
    "while\0",
    "with\0",
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_findword(
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
    let mut list: [*const c_char; 29] = [null(); 29];
    let mut k: usize = 0;
    while k < 29 {
        list[k] = keywords[k].as_ptr() as *const c_char;
        k += 1;
    }
    let i = jsY_findword(s, list.as_ptr(), 29);
    if i >= 0 {
        (*J).text = list[i as usize];
        return TK_BREAK + i; /* first keyword + i */
    }
    (*J).text = s;
    TK_IDENTIFIER
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_iswhite(c: c_int) -> c_int {
    (c == 0x9 || c == 0xB || c == 0xC || c == 0x20 || c == 0xA0 || c == 0xFEFF) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

#[inline]
fn isalpha(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)) as c_int
}

#[inline]
fn isdigit(c: c_int) -> c_int {
    (c >= '0' as c_int && c <= '9' as c_int) as c_int
}

#[inline]
fn ishex(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'f' as c_int) || (c >= 'A' as c_int && c <= 'F' as c_int)) as c_int
}

unsafe fn jsY_isidentifierstart(c: c_int) -> c_int {
    (isalpha(c) != 0 || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isidentifierpart(c: c_int) -> c_int {
    (isdigit(c) != 0
        || isalpha(c) != 0
        || c == '$' as c_int
        || c == '_' as c_int
        || isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isdec(c: c_int) -> c_int {
    isdigit(c)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_ishex(c: c_int) -> c_int {
    (isdigit(c) != 0 || ishex(c) != 0) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_tohex(c: c_int) -> c_int {
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
        jsY_error!(J, "unexpected escape sequence");
    }
}

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
        (*J).lexbuf.len += runetochar((*J).lexbuf.text.offset((*J).lexbuf.len as isize), &cc);
    }
}

unsafe fn textend(J: *mut js_State) -> *mut c_char {
    textpush(J, EOF);
    (*J).lexbuf.text
}

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

unsafe fn lexhex(J: *mut js_State) -> f64 {
    let mut n: f64 = 0.0;
    if jsY_ishex((*J).lexchar) == 0 {
        jsY_error!(J, "malformed hexadecimal number");
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
            jsY_error!(J, "number with leading zero");
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
            jsY_error!(J, "missing exponent");
        }
    }

    if jsY_isidentifierstart((*J).lexchar) != 0 {
        jsY_error!(J, "number with letter suffix");
    }

    (*J).number = js_strtod(s, null_mut());
    TK_NUMBER
}

unsafe fn lexescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    /* already consumed '\' */

    if jsY_accept!(J, '\n') != 0 {
        return 0;
    }

    let lc = (*J).lexchar;
    if lc == EOF {
        jsY_error!(J, "unterminated escape sequence");
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
        textpush(J, '\\' as c_int);
        jsY_next(J);
    } else if lc == '\'' as c_int {
        textpush(J, '\'' as c_int);
        jsY_next(J);
    } else if lc == '"' as c_int {
        textpush(J, '"' as c_int);
        jsY_next(J);
    } else if lc == 'b' as c_int {
        textpush(J, 0x8); /* '\b' */
        jsY_next(J);
    } else if lc == 'f' as c_int {
        textpush(J, 0xC); /* '\f' */
        jsY_next(J);
    } else if lc == 'n' as c_int {
        textpush(J, '\n' as c_int);
        jsY_next(J);
    } else if lc == 'r' as c_int {
        textpush(J, '\r' as c_int);
        jsY_next(J);
    } else if lc == 't' as c_int {
        textpush(J, '\t' as c_int);
        jsY_next(J);
    } else if lc == 'v' as c_int {
        textpush(J, 0xB); /* '\v' */
        jsY_next(J);
    } else {
        textpush(J, (*J).lexchar);
        jsY_next(J);
    }
    0
}

unsafe fn lexstring(J: *mut js_State) -> c_int {
    let s: *const c_char;

    let q = (*J).lexchar;
    jsY_next(J);

    textinit(J);

    while (*J).lexchar != q {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
            jsY_error!(J, "string not terminated");
        }
        if jsY_accept!(J, '\\') != 0 {
            if lexescape(J) != 0 {
                jsY_error!(J, "malformed escape sequence");
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

/* the ugliest language wart ever... */
unsafe fn isregexpcontext(last: c_int) -> c_int {
    match last {
        93 /* ']' */
        | 41 /* ')' */
        | 125 /* '}' */
        | TK_IDENTIFIER
        | TK_NUMBER
        | TK_STRING
        | TK_FALSE
        | TK_NULL
        | TK_THIS
        | TK_TRUE => 0,
        _ => 1,
    }
}

unsafe fn lexregexp(J: *mut js_State) -> c_int {
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
            jsY_error!(J, "regular expression not terminated");
        } else if jsY_accept!(J, '\\') != 0 {
            if jsY_accept!(J, '/') != 0 {
                textpush(J, '/' as c_int);
            } else {
                textpush(J, '\\' as c_int);
                if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
                    jsY_error!(J, "regular expression not terminated");
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
                "illegal flag in regular expression: %c",
                (*J).lexchar
            );
        }
    }

    if g > 1 || i > 1 || m > 1 {
        jsY_error!(J, "duplicated flag in regular expression");
    }

    (*J).text = s;

    let mut f: c_int = 0;
    if g != 0 {
        f |= JS_REGEXP_G;
    }
    if i != 0 {
        f |= JS_REGEXP_I;
    }
    if m != 0 {
        f |= JS_REGEXP_M;
    }
    flags = f;
    (*J).number = flags as f64;
    TK_REGEXP
}

/* simple "return [no Line Terminator here] ..." contexts */
unsafe fn isnlthcontext(last: c_int) -> c_int {
    match last {
        TK_BREAK | TK_CONTINUE | TK_RETURN | TK_THROW => 1,
        _ => 0,
    }
}

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
                    jsY_error!(J, "multi-line comment not terminated");
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
        } else if lc == '>' as c_int {
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
        } else if lc == '=' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                if jsY_accept!(J, '=') != 0 {
                    return TK_STRICTEQ;
                }
                return TK_EQ;
            }
            return '=' as c_int;
        } else if lc == '!' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                if jsY_accept!(J, '=') != 0 {
                    return TK_STRICTNE;
                }
                return TK_NE;
            }
            return '!' as c_int;
        } else if lc == '+' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '+') != 0 {
                return TK_INC;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_ADD_ASS;
            }
            return '+' as c_int;
        } else if lc == '-' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '-') != 0 {
                return TK_DEC;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_SUB_ASS;
            }
            return '-' as c_int;
        } else if lc == '*' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                return TK_MUL_ASS;
            }
            return '*' as c_int;
        } else if lc == '%' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
                return TK_MOD_ASS;
            }
            return '%' as c_int;
        } else if lc == '&' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '&') != 0 {
                return TK_AND;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_AND_ASS;
            }
            return '&' as c_int;
        } else if lc == '|' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '|') != 0 {
                return TK_OR;
            }
            if jsY_accept!(J, '=') != 0 {
                return TK_OR_ASS;
            }
            return '|' as c_int;
        } else if lc == '^' as c_int {
            jsY_next(J);
            if jsY_accept!(J, '=') != 0 {
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
            jsY_error!(J, "unexpected character: '%c'", (*J).lexchar);
        }
        jsY_error!(J, "unexpected character: \\u%04X", (*J).lexchar);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_initlex(
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
pub unsafe extern "C" fn jsY_lex(J: *mut js_State) -> c_int {
    (*J).lasttoken = jsY_lexx(J);
    (*J).lasttoken
}

unsafe fn lexjsonnumber(J: *mut js_State) -> c_int {
    let s: *const c_char = (*J).source.offset(-1);

    if (*J).lexchar == '-' as c_int {
        jsY_next(J);
    }

    if (*J).lexchar == '0' as c_int {
        jsY_next(J);
    } else if (*J).lexchar >= '1' as c_int && (*J).lexchar <= '9' as c_int {
        while isdigit((*J).lexchar) != 0 {
            jsY_next(J);
        }
    } else {
        jsY_error!(J, "unexpected non-digit");
    }

    if jsY_accept!(J, '.') != 0 {
        if isdigit((*J).lexchar) != 0 {
            while isdigit((*J).lexchar) != 0 {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, "missing digits after decimal point");
        }
    }

    if jsY_accept!(J, 'e') != 0 || jsY_accept!(J, 'E') != 0 {
        if (*J).lexchar == '-' as c_int || (*J).lexchar == '+' as c_int {
            jsY_next(J);
        }
        if isdigit((*J).lexchar) != 0 {
            while isdigit((*J).lexchar) != 0 {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, "missing digits after exponent indicator");
        }
    }

    (*J).number = js_strtod(s, null_mut());
    TK_NUMBER
}

unsafe fn lexjsonescape(J: *mut js_State) -> c_int {
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
        textpush(J, '"' as c_int);
        jsY_next(J);
    } else if lc == '\\' as c_int {
        textpush(J, '\\' as c_int);
        jsY_next(J);
    } else if lc == '/' as c_int {
        textpush(J, '/' as c_int);
        jsY_next(J);
    } else if lc == 'b' as c_int {
        textpush(J, 0x8); /* '\b' */
        jsY_next(J);
    } else if lc == 'f' as c_int {
        textpush(J, 0xC); /* '\f' */
        jsY_next(J);
    } else if lc == 'n' as c_int {
        textpush(J, '\n' as c_int);
        jsY_next(J);
    } else if lc == 'r' as c_int {
        textpush(J, '\r' as c_int);
        jsY_next(J);
    } else if lc == 't' as c_int {
        textpush(J, '\t' as c_int);
        jsY_next(J);
    } else {
        jsY_error!(J, "invalid escape sequence");
    }
    0
}

unsafe fn lexjsonstring(J: *mut js_State) -> c_int {
    let s: *const c_char;

    textinit(J);

    while (*J).lexchar != '"' as c_int {
        if (*J).lexchar == EOF {
            jsY_error!(J, "unterminated string");
        } else if (*J).lexchar < 32 {
            jsY_error!(J, "invalid control character in string");
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
pub unsafe extern "C" fn jsY_lexjson(J: *mut js_State) -> c_int {
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
            jsY_expect!(J, 'a');
            jsY_expect!(J, 'l');
            jsY_expect!(J, 's');
            jsY_expect!(J, 'e');
            return TK_FALSE;
        } else if lc == 'n' as c_int {
            jsY_next(J);
            jsY_expect!(J, 'u');
            jsY_expect!(J, 'l');
            jsY_expect!(J, 'l');
            return TK_NULL;
        } else if lc == 't' as c_int {
            jsY_next(J);
            jsY_expect!(J, 'r');
            jsY_expect!(J, 'u');
            jsY_expect!(J, 'e');
            return TK_TRUE;
        } else if lc == EOF {
            return 0; /* EOF */
        }

        if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
            jsY_error!(J, "unexpected character: '%c'", (*J).lexchar);
        }
        jsY_error!(J, "unexpected character: \\u%04X", (*J).lexchar);
    }
}
