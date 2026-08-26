//! Translated from c_src/src/jslex.c
use crate::jsi::*;
use crate::prelude::*;

/* stdio.h */
const EOF: c_int = -1;

/* Character values spelled as constants so that they can be used as `match`
 * patterns (a cast expression is not a pattern in Rust). */
const CH_BANG: c_int = b'!' as c_int;
const CH_DQUOTE: c_int = b'"' as c_int;
const CH_PERCENT: c_int = b'%' as c_int;
const CH_AMP: c_int = b'&' as c_int;
const CH_SQUOTE: c_int = b'\'' as c_int;
const CH_LPAREN: c_int = b'(' as c_int;
const CH_RPAREN: c_int = b')' as c_int;
const CH_STAR: c_int = b'*' as c_int;
const CH_PLUS: c_int = b'+' as c_int;
const CH_COMMA: c_int = b',' as c_int;
const CH_MINUS: c_int = b'-' as c_int;
const CH_DOT: c_int = b'.' as c_int;
const CH_SLASH: c_int = b'/' as c_int;
const CH_ZERO: c_int = b'0' as c_int;
const CH_COLON: c_int = b':' as c_int;
const CH_SEMICOLON: c_int = b';' as c_int;
const CH_LT: c_int = b'<' as c_int;
const CH_ASSIGN: c_int = b'=' as c_int;
const CH_GT: c_int = b'>' as c_int;
const CH_QUESTION: c_int = b'?' as c_int;
const CH_LBRACKET: c_int = b'[' as c_int;
const CH_BACKSLASH: c_int = b'\\' as c_int;
const CH_RBRACKET: c_int = b']' as c_int;
const CH_CARET: c_int = b'^' as c_int;
const CH_LBRACE: c_int = b'{' as c_int;
const CH_PIPE: c_int = b'|' as c_int;
const CH_RBRACE: c_int = b'}' as c_int;
const CH_TILDE: c_int = b'~' as c_int;
const CH_b: c_int = b'b' as c_int;
const CH_f: c_int = b'f' as c_int;
const CH_n: c_int = b'n' as c_int;
const CH_r: c_int = b'r' as c_int;
const CH_t: c_int = b't' as c_int;
const CH_u: c_int = b'u' as c_int;
const CH_v: c_int = b'v' as c_int;
const CH_x: c_int = b'x' as c_int;

/* A `static` table of C string pointers.  Raw pointers are not `Sync`, so the
 * arrays are wrapped in a transparent newtype. */
#[repr(transparent)]
struct StrTable<const N: usize>([*const c_char; N]);
unsafe impl<const N: usize> Sync for StrTable<N> {}

macro_rules! tokstr {
    (0) => {
        null()
    };
    ($s:literal) => {
        $s.as_ptr()
    };
}
macro_rules! strtable {
    ($($e:tt),* $(,)?) => { StrTable([ $(tokstr!($e)),* ]) };
}

/* JS_NORETURN static void jsY_error(js_State *J, const char *fmt, ...) */
unsafe fn jsY_error_str(J: *mut js_State, msgbuf: *const c_char) -> ! {
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
    js_throw(J)
}

macro_rules! jsY_error {
    ($J:expr, $($a:expr),*) => {{
        let mut msgbuf__: [c_char; 256] = [0; 256];
        snprintf(msgbuf__.as_mut_ptr(), 256, $($a),*);
        jsY_error_str($J, msgbuf__.as_ptr())
    }};
}

static tokenstring: StrTable<313> = strtable![
    c"(end-of-file)",
    c"'\\x01'", c"'\\x02'", c"'\\x03'", c"'\\x04'", c"'\\x05'", c"'\\x06'", c"'\\x07'",
    c"'\\x08'", c"'\\x09'", c"'\\x0A'", c"'\\x0B'", c"'\\x0C'", c"'\\x0D'", c"'\\x0E'", c"'\\x0F'",
    c"'\\x10'", c"'\\x11'", c"'\\x12'", c"'\\x13'", c"'\\x14'", c"'\\x15'", c"'\\x16'", c"'\\x17'",
    c"'\\x18'", c"'\\x19'", c"'\\x1A'", c"'\\x1B'", c"'\\x1C'", c"'\\x1D'", c"'\\x1E'", c"'\\x1F'",
    c"' '", c"'!'", c"'\"'", c"'#'", c"'$'", c"'%'", c"'&'", c"'\\''",
    c"'('", c"')'", c"'*'", c"'+'", c"','", c"'-'", c"'.'", c"'/'",
    c"'0'", c"'1'", c"'2'", c"'3'", c"'4'", c"'5'", c"'6'", c"'7'",
    c"'8'", c"'9'", c"':'", c"';'", c"'<'", c"'='", c"'>'", c"'?'",
    c"'@'", c"'A'", c"'B'", c"'C'", c"'D'", c"'E'", c"'F'", c"'G'",
    c"'H'", c"'I'", c"'J'", c"'K'", c"'L'", c"'M'", c"'N'", c"'O'",
    c"'P'", c"'Q'", c"'R'", c"'S'", c"'T'", c"'U'", c"'V'", c"'W'",
    c"'X'", c"'Y'", c"'Z'", c"'['", c"''", c"']'", c"'^'", c"'_'",
    c"'`'", c"'a'", c"'b'", c"'c'", c"'d'", c"'e'", c"'f'", c"'g'",
    c"'h'", c"'i'", c"'j'", c"'k'", c"'l'", c"'m'", c"'n'", c"'o'",
    c"'p'", c"'q'", c"'r'", c"'s'", c"'t'", c"'u'", c"'v'", c"'w'",
    c"'x'", c"'y'", c"'z'", c"'{'", c"'|'", c"'}'", c"'~'", c"'\\x7F'",

    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,
    0,0,0,0, 0,0,0,0, 0,0,0,0, 0,0,0,0,

    c"(identifier)", c"(number)", c"(string)", c"(regexp)",

    c"'<='", c"'>='", c"'=='", c"'!='", c"'==='", c"'!=='",
    c"'<<'", c"'>>'", c"'>>>'", c"'&&'", c"'||'",
    c"'+='", c"'-='", c"'*='", c"'/='", c"'%='",
    c"'<<='", c"'>>='", c"'>>>='", c"'&='", c"'|='", c"'^='",
    c"'++'", c"'--'",

    c"'break'", c"'case'", c"'catch'", c"'continue'", c"'debugger'",
    c"'default'", c"'delete'", c"'do'", c"'else'", c"'false'", c"'finally'", c"'for'",
    c"'function'", c"'if'", c"'in'", c"'instanceof'", c"'new'", c"'null'", c"'return'",
    c"'switch'", c"'this'", c"'throw'", c"'true'", c"'try'", c"'typeof'", c"'var'",
    c"'void'", c"'while'", c"'with'",
];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_tokenstring(token: c_int) -> *const c_char {
    if token >= 0 && token < tokenstring.0.len() as c_int {
        if !tokenstring.0[token as usize].is_null() {
            return tokenstring.0[token as usize];
        }
    }
    c"<unknown>".as_ptr()
}

static keywords: StrTable<29> = strtable![
    c"break", c"case", c"catch", c"continue", c"debugger", c"default", c"delete",
    c"do", c"else", c"false", c"finally", c"for", c"function", c"if", c"in",
    c"instanceof", c"new", c"null", c"return", c"switch", c"this", c"throw",
    c"true", c"try", c"typeof", c"var", c"void", c"while", c"with",
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
    let i: c_int = jsY_findword(s, keywords.0.as_ptr(), keywords.0.len() as c_int);
    if i >= 0 {
        (*J).text = keywords.0[i as usize];
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

/* #define ishex(c) ((c >= 'a' && c <= 'f') || (c >= 'A' && c <= 'F')) */
#[inline(always)]
fn ishex(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'f' as c_int) || (c >= 'A' as c_int && c <= 'F' as c_int)
}

unsafe fn jsY_isidentifierstart(c: c_int) -> c_int {
    (isalpha(c) || c == '$' as c_int || c == '_' as c_int || jsU_isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isidentifierpart(c: c_int) -> c_int {
    (isdigit(c) || isalpha(c) || c == '$' as c_int || c == '_' as c_int || jsU_isalpharune(c) != 0)
        as c_int
}

unsafe fn jsY_isdec(c: c_int) -> c_int {
    isdigit(c) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsY_ishex(c: c_int) -> c_int {
    (isdigit(c) || ishex(c)) as c_int
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
    (*J).source = (*J).source.offset(jsU_chartorune(&mut c, (*J).source) as isize);
    /* consume CR LF as one unit */
    if c == '\r' as c_int && *(*J).source as c_int == '\n' as c_int {
        (*J).source = (*J).source.add(1);
    }
    if jsY_isnewline(c) != 0 {
        (*J).line += 1;
        c = '\n' as c_int;
    }
    (*J).lexchar = c;
}

/* #define jsY_accept(J, x) (J->lexchar == x ? (jsY_next(J), 1) : 0) */
macro_rules! jsY_accept {
    ($J:expr, $x:expr) => {
        if (*$J).lexchar == $x {
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
            jsY_error!($J, c"expected '%c'".as_ptr(), $x)
        }
    };
}

unsafe fn jsY_unescape(J: *mut js_State) {
    if jsY_accept!(J, '\\' as c_int) != 0 {
        'error: {
            if jsY_accept!(J, 'u' as c_int) != 0 {
                let mut x: c_int = 0;
                if jsY_ishex((*J).lexchar) == 0 { break 'error; } x |= jsY_tohex((*J).lexchar) << 12; jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { break 'error; } x |= jsY_tohex((*J).lexchar) << 8; jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { break 'error; } x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 { break 'error; } x |= jsY_tohex((*J).lexchar);
                (*J).lexchar = x;
                return;
            }
        }
        /* error: */
        jsY_error!(J, c"unexpected escape sequence".as_ptr());
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
        (*J).lexbuf.len +=
            jsU_runetochar((*J).lexbuf.text.offset((*J).lexbuf.len as isize), &c);
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
        if jsY_accept!(J, '*' as c_int) != 0 {
            while (*J).lexchar == '*' as c_int {
                jsY_next(J);
            }
            if jsY_accept!(J, '/' as c_int) != 0 {
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
        jsY_error!(J, c"malformed hexadecimal number".as_ptr());
    }
    while jsY_ishex((*J).lexchar) != 0 {
        n = n * 16.0 + jsY_tohex((*J).lexchar) as f64;
        jsY_next(J);
    }
    n
}

/* #if 0 -- the old hand-rolled number parser (not compiled) */

#[cfg(any())]
unsafe fn lexinteger(J: *mut js_State) -> f64 {
    let mut n: f64 = 0.0;
    if jsY_isdec((*J).lexchar) == 0 {
        jsY_error!(J, c"malformed number".as_ptr());
    }
    while jsY_isdec((*J).lexchar) != 0 {
        n = n * 10.0 + ((*J).lexchar - '0' as c_int) as f64;
        jsY_next(J);
    }
    n
}

#[cfg(any())]
unsafe fn lexfraction(J: *mut js_State) -> f64 {
    let mut n: f64 = 0.0;
    let mut d: f64 = 1.0;
    while jsY_isdec((*J).lexchar) != 0 {
        n = n * 10.0 + ((*J).lexchar - '0' as c_int) as f64;
        d = d * 10.0;
        jsY_next(J);
    }
    n / d
}

#[cfg(any())]
unsafe fn lexexponent(J: *mut js_State) -> f64 {
    let sign: f64;
    if jsY_accept!(J, 'e' as c_int) != 0 || jsY_accept!(J, 'E' as c_int) != 0 {
        if jsY_accept!(J, '-' as c_int) != 0 {
            sign = -1.0;
        } else if jsY_accept!(J, '+' as c_int) != 0 {
            sign = 1.0;
        } else {
            sign = 1.0;
        }
        return sign * lexinteger(J);
    }
    0.0
}

#[cfg(any())]
unsafe fn lexnumber(J: *mut js_State) -> c_int {
    let mut n: f64;
    let e: f64;

    if jsY_accept!(J, '0' as c_int) != 0 {
        if jsY_accept!(J, 'x' as c_int) != 0 || jsY_accept!(J, 'X' as c_int) != 0 {
            (*J).number = lexhex(J);
            return TK_NUMBER;
        }
        if jsY_isdec((*J).lexchar) != 0 {
            jsY_error!(J, c"number with leading zero".as_ptr());
        }
        n = 0.0;
        if jsY_accept!(J, '.' as c_int) != 0 {
            n += lexfraction(J);
        }
    } else if jsY_accept!(J, '.' as c_int) != 0 {
        if jsY_isdec((*J).lexchar) == 0 {
            return '.' as c_int;
        }
        n = lexfraction(J);
    } else {
        n = lexinteger(J);
        if jsY_accept!(J, '.' as c_int) != 0 {
            n += lexfraction(J);
        }
    }

    e = lexexponent(J);
    if e < 0.0 {
        n /= pow(10.0, -e);
    } else if e > 0.0 {
        n *= pow(10.0, e);
    }

    if jsY_isidentifierstart((*J).lexchar) != 0 {
        jsY_error!(J, c"number with letter suffix".as_ptr());
    }

    (*J).number = n;
    TK_NUMBER
}

/* #else */

unsafe fn lexnumber(J: *mut js_State) -> c_int {
    let s: *const c_char = (*J).source.sub(1);

    if jsY_accept!(J, '0' as c_int) != 0 {
        if jsY_accept!(J, 'x' as c_int) != 0 || jsY_accept!(J, 'X' as c_int) != 0 {
            (*J).number = lexhex(J);
            return TK_NUMBER;
        }
        if jsY_isdec((*J).lexchar) != 0 {
            jsY_error!(J, c"number with leading zero".as_ptr());
        }
        if jsY_accept!(J, '.' as c_int) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    } else if jsY_accept!(J, '.' as c_int) != 0 {
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
        if jsY_accept!(J, '.' as c_int) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    }

    if jsY_accept!(J, 'e' as c_int) != 0 || jsY_accept!(J, 'E' as c_int) != 0 {
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

/* #endif */

unsafe fn lexescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    /* already consumed '\' */

    if jsY_accept!(J, '\n' as c_int) != 0 {
        return 0;
    }

    match (*J).lexchar {
        EOF => {
            jsY_error!(J, c"unterminated escape sequence".as_ptr());
        }
        CH_u => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 12; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 8; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
            textpush(J, x);
        }
        CH_x => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
            textpush(J, x);
        }
        CH_ZERO => { textpush(J, 0); jsY_next(J); }
        CH_BACKSLASH => { textpush(J, '\\' as Rune); jsY_next(J); }
        CH_SQUOTE => { textpush(J, '\'' as Rune); jsY_next(J); }
        CH_DQUOTE => { textpush(J, '"' as Rune); jsY_next(J); }
        CH_b => { textpush(J, 0x08); jsY_next(J); }
        CH_f => { textpush(J, 0x0C); jsY_next(J); }
        CH_n => { textpush(J, '\n' as Rune); jsY_next(J); }
        CH_r => { textpush(J, '\r' as Rune); jsY_next(J); }
        CH_t => { textpush(J, '\t' as Rune); jsY_next(J); }
        CH_v => { textpush(J, 0x0B); jsY_next(J); }
        _ => { textpush(J, (*J).lexchar); jsY_next(J); }
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
        if jsY_accept!(J, '\\' as c_int) != 0 {
            if lexescape(J) != 0 {
                jsY_error!(J, c"malformed escape sequence".as_ptr());
            }
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    jsY_expect!(J, q);

    s = textend(J) as *const c_char;

    (*J).text = s;
    TK_STRING
}

/* the ugliest language wart ever... */
unsafe fn isregexpcontext(last: c_int) -> c_int {
    match last {
        CH_RBRACKET | CH_RPAREN | CH_RBRACE | TK_IDENTIFIER | TK_NUMBER | TK_STRING | TK_FALSE
        | TK_NULL | TK_THIS | TK_TRUE => 0,
        _ => 1,
    }
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
        } else if jsY_accept!(J, '\\' as c_int) != 0 {
            if jsY_accept!(J, '/' as c_int) != 0 {
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

    s = textend(J) as *const c_char;

    /* regexp flags */
    g = 0;
    i = 0;
    m = 0;

    while jsY_isidentifierpart((*J).lexchar) != 0 {
        if jsY_accept!(J, 'g' as c_int) != 0 {
            g += 1;
        } else if jsY_accept!(J, 'i' as c_int) != 0 {
            i += 1;
        } else if jsY_accept!(J, 'm' as c_int) != 0 {
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

        if jsY_accept!(J, '\n' as c_int) != 0 {
            (*J).newline = 1;
            if isnlthcontext((*J).lasttoken) != 0 {
                return ';' as c_int;
            }
            continue;
        }

        if jsY_accept!(J, '/' as c_int) != 0 {
            if jsY_accept!(J, '/' as c_int) != 0 {
                lexlinecomment(J);
                continue;
            } else if jsY_accept!(J, '*' as c_int) != 0 {
                if lexcomment(J) != 0 {
                    jsY_error!(J, c"multi-line comment not terminated".as_ptr());
                }
                continue;
            } else if isregexpcontext((*J).lasttoken) != 0 {
                return lexregexp(J);
            } else if jsY_accept!(J, '=' as c_int) != 0 {
                return TK_DIV_ASS;
            } else {
                return '/' as c_int;
            }
        }

        if (*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int {
            return lexnumber(J);
        }

        match (*J).lexchar {
            CH_LPAREN => { jsY_next(J); return '(' as c_int; }
            CH_RPAREN => { jsY_next(J); return ')' as c_int; }
            CH_COMMA => { jsY_next(J); return ',' as c_int; }
            CH_COLON => { jsY_next(J); return ':' as c_int; }
            CH_SEMICOLON => { jsY_next(J); return ';' as c_int; }
            CH_QUESTION => { jsY_next(J); return '?' as c_int; }
            CH_LBRACKET => { jsY_next(J); return '[' as c_int; }
            CH_RBRACKET => { jsY_next(J); return ']' as c_int; }
            CH_LBRACE => { jsY_next(J); return '{' as c_int; }
            CH_RBRACE => { jsY_next(J); return '}' as c_int; }
            CH_TILDE => { jsY_next(J); return '~' as c_int; }

            CH_SQUOTE | CH_DQUOTE => {
                return lexstring(J);
            }

            CH_DOT => {
                return lexnumber(J);
            }

            CH_LT => {
                jsY_next(J);
                if jsY_accept!(J, '<' as c_int) != 0 {
                    if jsY_accept!(J, '=' as c_int) != 0 {
                        return TK_SHL_ASS;
                    }
                    return TK_SHL;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_LE;
                }
                return '<' as c_int;
            }

            CH_GT => {
                jsY_next(J);
                if jsY_accept!(J, '>' as c_int) != 0 {
                    if jsY_accept!(J, '>' as c_int) != 0 {
                        if jsY_accept!(J, '=' as c_int) != 0 {
                            return TK_USHR_ASS;
                        }
                        return TK_USHR;
                    }
                    if jsY_accept!(J, '=' as c_int) != 0 {
                        return TK_SHR_ASS;
                    }
                    return TK_SHR;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_GE;
                }
                return '>' as c_int;
            }

            CH_ASSIGN => {
                jsY_next(J);
                if jsY_accept!(J, '=' as c_int) != 0 {
                    if jsY_accept!(J, '=' as c_int) != 0 {
                        return TK_STRICTEQ;
                    }
                    return TK_EQ;
                }
                return '=' as c_int;
            }

            CH_BANG => {
                jsY_next(J);
                if jsY_accept!(J, '=' as c_int) != 0 {
                    if jsY_accept!(J, '=' as c_int) != 0 {
                        return TK_STRICTNE;
                    }
                    return TK_NE;
                }
                return '!' as c_int;
            }

            CH_PLUS => {
                jsY_next(J);
                if jsY_accept!(J, '+' as c_int) != 0 {
                    return TK_INC;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_ADD_ASS;
                }
                return '+' as c_int;
            }

            CH_MINUS => {
                jsY_next(J);
                if jsY_accept!(J, '-' as c_int) != 0 {
                    return TK_DEC;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_SUB_ASS;
                }
                return '-' as c_int;
            }

            CH_STAR => {
                jsY_next(J);
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_MUL_ASS;
                }
                return '*' as c_int;
            }

            CH_PERCENT => {
                jsY_next(J);
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_MOD_ASS;
                }
                return '%' as c_int;
            }

            CH_AMP => {
                jsY_next(J);
                if jsY_accept!(J, '&' as c_int) != 0 {
                    return TK_AND;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_AND_ASS;
                }
                return '&' as c_int;
            }

            CH_PIPE => {
                jsY_next(J);
                if jsY_accept!(J, '|' as c_int) != 0 {
                    return TK_OR;
                }
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_OR_ASS;
                }
                return '|' as c_int;
            }

            CH_CARET => {
                jsY_next(J);
                if jsY_accept!(J, '=' as c_int) != 0 {
                    return TK_XOR_ASS;
                }
                return '^' as c_int;
            }

            EOF => {
                return 0; /* EOF */
            }

            _ => {}
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
    let s: *const c_char = (*J).source.sub(1);

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

    if jsY_accept!(J, '.' as c_int) != 0 {
        if isdigit((*J).lexchar) {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, c"missing digits after decimal point".as_ptr());
        }
    }

    if jsY_accept!(J, 'e' as c_int) != 0 || jsY_accept!(J, 'E' as c_int) != 0 {
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

    /* NB: the C switch lists 'default' first; it throws, so no fallthrough. */
    match (*J).lexchar {
        CH_u => {
            jsY_next(J);
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 12; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 8; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar) << 4; jsY_next(J); }
            if jsY_ishex((*J).lexchar) == 0 { return 1; } else { x |= jsY_tohex((*J).lexchar); jsY_next(J); }
            textpush(J, x);
        }
        CH_DQUOTE => { textpush(J, '"' as Rune); jsY_next(J); }
        CH_BACKSLASH => { textpush(J, '\\' as Rune); jsY_next(J); }
        CH_SLASH => { textpush(J, '/' as Rune); jsY_next(J); }
        CH_b => { textpush(J, 0x08); jsY_next(J); }
        CH_f => { textpush(J, 0x0C); jsY_next(J); }
        CH_n => { textpush(J, '\n' as Rune); jsY_next(J); }
        CH_r => { textpush(J, '\r' as Rune); jsY_next(J); }
        CH_t => { textpush(J, '\t' as Rune); jsY_next(J); }
        _ => {
            jsY_error!(J, c"invalid escape sequence".as_ptr());
        }
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
        } else if jsY_accept!(J, '\\' as c_int) != 0 {
            lexjsonescape(J);
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    jsY_expect!(J, '"' as c_int);

    s = textend(J) as *const c_char;

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

        match (*J).lexchar {
            CH_COMMA => { jsY_next(J); return ',' as c_int; }
            CH_COLON => { jsY_next(J); return ':' as c_int; }
            CH_LBRACKET => { jsY_next(J); return '[' as c_int; }
            CH_RBRACKET => { jsY_next(J); return ']' as c_int; }
            CH_LBRACE => { jsY_next(J); return '{' as c_int; }
            CH_RBRACE => { jsY_next(J); return '}' as c_int; }

            CH_DQUOTE => {
                jsY_next(J);
                return lexjsonstring(J);
            }

            CH_f => {
                jsY_next(J);
                jsY_expect!(J, 'a' as c_int);
                jsY_expect!(J, 'l' as c_int);
                jsY_expect!(J, 's' as c_int);
                jsY_expect!(J, 'e' as c_int);
                return TK_FALSE;
            }

            CH_n => {
                jsY_next(J);
                jsY_expect!(J, 'u' as c_int);
                jsY_expect!(J, 'l' as c_int);
                jsY_expect!(J, 'l' as c_int);
                return TK_NULL;
            }

            CH_t => {
                jsY_next(J);
                jsY_expect!(J, 'r' as c_int);
                jsY_expect!(J, 'u' as c_int);
                jsY_expect!(J, 'e' as c_int);
                return TK_TRUE;
            }

            EOF => {
                return 0; /* EOF */
            }

            _ => {}
        }

        if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
            jsY_error!(J, c"unexpected character: '%c'".as_ptr(), (*J).lexchar);
        }
        jsY_error!(J, c"unexpected character: \\u%04X".as_ptr(), (*J).lexchar);
    }
}
