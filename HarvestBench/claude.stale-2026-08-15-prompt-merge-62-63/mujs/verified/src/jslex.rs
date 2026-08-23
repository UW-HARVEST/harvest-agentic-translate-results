//! Translated from jslex.c — the lexer.
#![allow(non_snake_case, non_upper_case_globals)]

use crate::cutil::*;
use crate::jsrun::{js_malloc, js_realloc};
use crate::types::*;
use crate::utf::*;
use std::os::raw::{c_char, c_int, c_void};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

// jsY_error is variadic and lives in shim.c as jsY_error_shim; the shim's rs_jsY_error
// (defined below) builds "file:line: " + msg then throws a syntaxerror.
#[no_mangle]
pub unsafe extern "C-unwind" fn rs_jsY_error(J: *mut js_State, msg: *const c_char) {
    let mut buf: [c_char; 512] = [0; 512];
    libc::snprintf(buf.as_mut_ptr(), 256, cstr!("%s:%d: "), (*J).filename, (*J).lexline);
    strcat(buf.as_mut_ptr(), msg);
    crate::jserror::js_newsyntaxerror(J, buf.as_ptr());
    crate::jsrun::js_throw(J);
}

// Convenience wrapper matching C's `jsY_error(J, fmt, ...)` for internal callers.
macro_rules! jsY_error {
    ($J:expr, $($arg:tt)*) => {
        crate::jserror::jsY_error_shim($J, $($arg)*)
    };
}

// Runtime token strings (indexed by token value). Matches jslex.c tokenstring[].
unsafe fn tokenstring_at(token: c_int) -> *const c_char {
    TS.with_init();
    if token >= 0 && (token as usize) < TS_LEN {
        let p = TS_TABLE[token as usize];
        if !p.is_null() {
            return p;
        }
    }
    cstr!("<unknown>")
}

const TS_LEN: usize = 313;
static mut TS_TABLE: [*const c_char; TS_LEN] = [std::ptr::null(); TS_LEN];
struct TsInit;
static TS: TsInit = TsInit;
impl TsInit {
    unsafe fn with_init(&self) {
        if !TS_TABLE[0].is_null() {
            return;
        }
        // index 0
        TS_TABLE[0] = cstr!("(end-of-file)");
        // 1..127 char names
        macro_rules! set { ($i:expr, $s:literal) => { TS_TABLE[$i] = cstr!($s); } }
        set!(1, "'\\x01'"); set!(2, "'\\x02'"); set!(3, "'\\x03'"); set!(4, "'\\x04'");
        set!(5, "'\\x05'"); set!(6, "'\\x06'"); set!(7, "'\\x07'"); set!(8, "'\\x08'");
        set!(9, "'\\x09'"); set!(10, "'\\x0A'"); set!(11, "'\\x0B'"); set!(12, "'\\x0C'");
        set!(13, "'\\x0D'"); set!(14, "'\\x0E'"); set!(15, "'\\x0F'"); set!(16, "'\\x10'");
        set!(17, "'\\x11'"); set!(18, "'\\x12'"); set!(19, "'\\x13'"); set!(20, "'\\x14'");
        set!(21, "'\\x15'"); set!(22, "'\\x16'"); set!(23, "'\\x17'"); set!(24, "'\\x18'");
        set!(25, "'\\x19'"); set!(26, "'\\x1A'"); set!(27, "'\\x1B'"); set!(28, "'\\x1C'");
        set!(29, "'\\x1D'"); set!(30, "'\\x1E'"); set!(31, "'\\x1F'");
        set!(32, "' '"); set!(33, "'!'"); set!(34, "'\"'"); set!(35, "'#'");
        set!(36, "'$'"); set!(37, "'%'"); set!(38, "'&'"); set!(39, "'\\''");
        set!(40, "'('"); set!(41, "')'"); set!(42, "'*'"); set!(43, "'+'");
        set!(44, "','"); set!(45, "'-'"); set!(46, "'.'"); set!(47, "'/'");
        set!(48, "'0'"); set!(49, "'1'"); set!(50, "'2'"); set!(51, "'3'");
        set!(52, "'4'"); set!(53, "'5'"); set!(54, "'6'"); set!(55, "'7'");
        set!(56, "'8'"); set!(57, "'9'"); set!(58, "':'"); set!(59, "';'");
        set!(60, "'<'"); set!(61, "'='"); set!(62, "'>'"); set!(63, "'?'");
        set!(64, "'@'"); set!(65, "'A'"); set!(66, "'B'"); set!(67, "'C'");
        set!(68, "'D'"); set!(69, "'E'"); set!(70, "'F'"); set!(71, "'G'");
        set!(72, "'H'"); set!(73, "'I'"); set!(74, "'J'"); set!(75, "'K'");
        set!(76, "'L'"); set!(77, "'M'"); set!(78, "'N'"); set!(79, "'O'");
        set!(80, "'P'"); set!(81, "'Q'"); set!(82, "'R'"); set!(83, "'S'");
        set!(84, "'T'"); set!(85, "'U'"); set!(86, "'V'"); set!(87, "'W'");
        set!(88, "'X'"); set!(89, "'Y'"); set!(90, "'Z'"); set!(91, "'['");
        // jslex.c writes `"'\'"` for index 92 (backslash). In a C string
        // literal `\'` is simply `'`, so the value is the two characters `''`
        // — NOT `'\''`. Preserve the C's quirk verbatim.
        set!(92, "''"); set!(93, "']'"); set!(94, "'^'"); set!(95, "'_'");
        set!(96, "'`'"); set!(97, "'a'"); set!(98, "'b'"); set!(99, "'c'");
        set!(100, "'d'"); set!(101, "'e'"); set!(102, "'f'"); set!(103, "'g'");
        set!(104, "'h'"); set!(105, "'i'"); set!(106, "'j'"); set!(107, "'k'");
        set!(108, "'l'"); set!(109, "'m'"); set!(110, "'n'"); set!(111, "'o'");
        set!(112, "'p'"); set!(113, "'q'"); set!(114, "'r'"); set!(115, "'s'");
        set!(116, "'t'"); set!(117, "'u'"); set!(118, "'v'"); set!(119, "'w'");
        set!(120, "'x'"); set!(121, "'y'"); set!(122, "'z'"); set!(123, "'{'");
        set!(124, "'|'"); set!(125, "'}'"); set!(126, "'~'"); set!(127, "'\\x7F'");
        // 128..255 remain NULL (0)
        // named tokens
        set!(256, "(identifier)"); set!(257, "(number)"); set!(258, "(string)"); set!(259, "(regexp)");
        set!(260, "'<='"); set!(261, "'>='"); set!(262, "'=='"); set!(263, "'!='");
        set!(264, "'==='"); set!(265, "'!=='"); set!(266, "'<<'"); set!(267, "'>>'");
        set!(268, "'>>>'"); set!(269, "'&&'"); set!(270, "'||'"); set!(271, "'+='");
        set!(272, "'-='"); set!(273, "'*='"); set!(274, "'/='"); set!(275, "'%='");
        set!(276, "'<<='"); set!(277, "'>>='"); set!(278, "'>>>='"); set!(279, "'&='");
        set!(280, "'|='"); set!(281, "'^='"); set!(282, "'++'"); set!(283, "'--'");
        set!(284, "'break'"); set!(285, "'case'"); set!(286, "'catch'"); set!(287, "'continue'");
        set!(288, "'debugger'"); set!(289, "'default'"); set!(290, "'delete'"); set!(291, "'do'");
        set!(292, "'else'"); set!(293, "'false'"); set!(294, "'finally'"); set!(295, "'for'");
        set!(296, "'function'"); set!(297, "'if'"); set!(298, "'in'"); set!(299, "'instanceof'");
        set!(300, "'new'"); set!(301, "'null'"); set!(302, "'return'"); set!(303, "'switch'");
        set!(304, "'this'"); set!(305, "'throw'"); set!(306, "'true'"); set!(307, "'try'");
        set!(308, "'typeof'"); set!(309, "'var'"); set!(310, "'void'"); set!(311, "'while'");
        set!(312, "'with'");
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_tokenstring(token: c_int) -> *const c_char {
    tokenstring_at(token)
}

struct Keywords([*const c_char; 29]);
unsafe impl Sync for Keywords {}
static KEYWORDS_W: Keywords = Keywords([
    b"break\0".as_ptr() as *const c_char,
    b"case\0".as_ptr() as *const c_char,
    b"catch\0".as_ptr() as *const c_char,
    b"continue\0".as_ptr() as *const c_char,
    b"debugger\0".as_ptr() as *const c_char,
    b"default\0".as_ptr() as *const c_char,
    b"delete\0".as_ptr() as *const c_char,
    b"do\0".as_ptr() as *const c_char,
    b"else\0".as_ptr() as *const c_char,
    b"false\0".as_ptr() as *const c_char,
    b"finally\0".as_ptr() as *const c_char,
    b"for\0".as_ptr() as *const c_char,
    b"function\0".as_ptr() as *const c_char,
    b"if\0".as_ptr() as *const c_char,
    b"in\0".as_ptr() as *const c_char,
    b"instanceof\0".as_ptr() as *const c_char,
    b"new\0".as_ptr() as *const c_char,
    b"null\0".as_ptr() as *const c_char,
    b"return\0".as_ptr() as *const c_char,
    b"switch\0".as_ptr() as *const c_char,
    b"this\0".as_ptr() as *const c_char,
    b"throw\0".as_ptr() as *const c_char,
    b"true\0".as_ptr() as *const c_char,
    b"try\0".as_ptr() as *const c_char,
    b"typeof\0".as_ptr() as *const c_char,
    b"var\0".as_ptr() as *const c_char,
    b"void\0".as_ptr() as *const c_char,
    b"while\0".as_ptr() as *const c_char,
    b"with\0".as_ptr() as *const c_char,
]);
#[inline]
unsafe fn KEYWORDS() -> &'static [*const c_char; 29] {
    &KEYWORDS_W.0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_findword(s: *const c_char, list: *const *const c_char, num: c_int) -> c_int {
    let mut l = 0;
    let mut r = num - 1;
    while l <= r {
        let m = (l + r) >> 1;
        let c = strcmp(s, *list.add(m as usize));
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
    let i = jsY_findword(s, KEYWORDS().as_ptr(), KEYWORDS().len() as c_int);
    if i >= 0 {
        (*J).text = KEYWORDS()[i as usize];
        return TK_BREAK + i;
    }
    (*J).text = s;
    TK_IDENTIFIER
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_iswhite(c: c_int) -> c_int {
    (c == 0x9 || c == 0xB || c == 0xC || c == 0x20 || c == 0xA0 || c == 0xFEFF) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

#[inline]
unsafe fn isalpha(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int)
}
#[inline]
unsafe fn isdigit(c: c_int) -> bool {
    c >= '0' as c_int && c <= '9' as c_int
}
#[inline]
unsafe fn ishexc(c: c_int) -> bool {
    (c >= 'a' as c_int && c <= 'f' as c_int) || (c >= 'A' as c_int && c <= 'F' as c_int)
}

unsafe fn jsY_isidentifierstart(c: c_int) -> c_int {
    (isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isidentifierpart(c: c_int) -> c_int {
    (isdigit(c) || isalpha(c) || c == '$' as c_int || c == '_' as c_int || isalpharune(c) != 0) as c_int
}

unsafe fn jsY_isdec(c: c_int) -> c_int {
    isdigit(c) as c_int
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_ishex(c: c_int) -> c_int {
    (isdigit(c) || ishexc(c)) as c_int
}

#[no_mangle]
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
    let mut c: Rune = 0;
    if *(*J).source == 0 {
        (*J).lexchar = EOF;
        return;
    }
    (*J).source = (*J).source.add(chartorune(&mut c, (*J).source) as usize);
    if c == '\r' as Rune && *(*J).source == '\n' as c_char {
        (*J).source = (*J).source.add(1);
    }
    if jsY_isnewline(c) != 0 {
        (*J).line += 1;
        c = '\n' as Rune;
    }
    (*J).lexchar = c;
}

macro_rules! accept_ch {
    ($J:expr, $x:expr) => {
        if (*$J).lexchar == $x {
            jsY_next($J);
            1
        } else {
            0
        }
    };
}

macro_rules! expect_ch {
    ($J:expr, $x:expr) => {
        if accept_ch!($J, $x) == 0 {
            jsY_error!($J, cstr!("expected '%c'"), $x);
        }
    };
}

unsafe fn jsY_unescape(J: *mut js_State) {
    if accept_ch!(J, '\\' as c_int) != 0 {
        if accept_ch!(J, 'u' as c_int) != 0 {
            let mut x: c_int = 0;
            'blk: {
                if jsY_ishex((*J).lexchar) == 0 {
                    break 'blk;
                }
                x |= jsY_tohex((*J).lexchar) << 12;
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 {
                    break 'blk;
                }
                x |= jsY_tohex((*J).lexchar) << 8;
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 {
                    break 'blk;
                }
                x |= jsY_tohex((*J).lexchar) << 4;
                jsY_next(J);
                if jsY_ishex((*J).lexchar) == 0 {
                    break 'blk;
                }
                x |= jsY_tohex((*J).lexchar);
                (*J).lexchar = x;
                return;
            }
        }
        /* The C's `error:` label sits OUTSIDE the `if (jsY_accept(J, 'u'))`
         * block, so a backslash that is not followed by a well-formed \uXXXX
         * escape falls through to it -- including `\q`, where the `u` is never
         * accepted at all. */
        jsY_error!(J, cstr!("unexpected escape sequence"));
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
    let n;
    let newcap;
    if c == EOF {
        n = 1;
    } else {
        n = runelen(c);
    }
    if (*J).lexbuf.len + n > (*J).lexbuf.cap {
        newcap = (*J).lexbuf.cap * 2;
        (*J).lexbuf.text = js_realloc(J, (*J).lexbuf.text as *mut c_void, newcap) as *mut c_char;
        (*J).lexbuf.cap = newcap;
    }
    if c == EOF {
        *(*J).lexbuf.text.add((*J).lexbuf.len as usize) = 0;
        (*J).lexbuf.len += 1;
    } else {
        let mut cc = c;
        (*J).lexbuf.len += runetochar((*J).lexbuf.text.add((*J).lexbuf.len as usize), &mut cc);
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
    while (*J).lexchar != EOF {
        if accept_ch!(J, '*' as c_int) != 0 {
            while (*J).lexchar == '*' as c_int {
                jsY_next(J);
            }
            if accept_ch!(J, '/' as c_int) != 0 {
                return 0;
            }
        } else {
            jsY_next(J);
        }
    }
    -1
}

unsafe fn lexhex(J: *mut js_State) -> f64 {
    let mut n = 0.0;
    if jsY_ishex((*J).lexchar) == 0 {
        jsY_error!(J, cstr!("malformed hexadecimal number"));
    }
    while jsY_ishex((*J).lexchar) != 0 {
        n = n * 16.0 + jsY_tohex((*J).lexchar) as f64;
        jsY_next(J);
    }
    n
}

unsafe fn lexnumber(J: *mut js_State) -> c_int {
    let s = (*J).source.offset(-1);

    if accept_ch!(J, '0' as c_int) != 0 {
        if accept_ch!(J, 'x' as c_int) != 0 || accept_ch!(J, 'X' as c_int) != 0 {
            (*J).number = lexhex(J);
            return TK_NUMBER;
        }
        if jsY_isdec((*J).lexchar) != 0 {
            jsY_error!(J, cstr!("number with leading zero"));
        }
        if accept_ch!(J, '.' as c_int) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    } else if accept_ch!(J, '.' as c_int) != 0 {
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
        if accept_ch!(J, '.' as c_int) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        }
    }

    if accept_ch!(J, 'e' as c_int) != 0 || accept_ch!(J, 'E' as c_int) != 0 {
        if (*J).lexchar == '-' as c_int || (*J).lexchar == '+' as c_int {
            jsY_next(J);
        }
        if jsY_isdec((*J).lexchar) != 0 {
            while jsY_isdec((*J).lexchar) != 0 {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, cstr!("missing exponent"));
        }
    }

    if jsY_isidentifierstart((*J).lexchar) != 0 {
        jsY_error!(J, cstr!("number with letter suffix"));
    }

    (*J).number = crate::jsdtoa::js_strtod(s, std::ptr::null_mut());
    TK_NUMBER
}

unsafe fn lexescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    if accept_ch!(J, '\n' as c_int) != 0 {
        return 0;
    }

    match (*J).lexchar {
        EOF => {
            jsY_error!(J, cstr!("unterminated escape sequence"));
        }
        x2 if x2 == 'u' as c_int => {
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
        }
        x2 if x2 == 'x' as c_int => {
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
        }
        x2 if x2 == '0' as c_int => {
            textpush(J, 0);
            jsY_next(J);
        }
        x2 if x2 == '\\' as c_int => {
            textpush(J, '\\' as Rune);
            jsY_next(J);
        }
        x2 if x2 == '\'' as c_int => {
            textpush(J, '\'' as Rune);
            jsY_next(J);
        }
        x2 if x2 == '"' as c_int => {
            textpush(J, '"' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'b' as c_int => {
            textpush(J, '\u{08}' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'f' as c_int => {
            textpush(J, '\u{0c}' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'n' as c_int => {
            textpush(J, '\n' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'r' as c_int => {
            textpush(J, '\r' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 't' as c_int => {
            textpush(J, '\t' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'v' as c_int => {
            textpush(J, '\u{0b}' as Rune);
            jsY_next(J);
        }
        _ => {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    0
}

unsafe fn lexstring(J: *mut js_State) -> c_int {
    let s;
    let q = (*J).lexchar;
    jsY_next(J);

    textinit(J);

    while (*J).lexchar != q {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
            jsY_error!(J, cstr!("string not terminated"));
        }
        if accept_ch!(J, '\\' as c_int) != 0 {
            if lexescape(J) != 0 {
                jsY_error!(J, cstr!("malformed escape sequence"));
            }
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    expect_ch!(J, q);

    s = textend(J);
    (*J).text = s;
    TK_STRING
}

unsafe fn isregexpcontext(last: c_int) -> c_int {
    match last {
        x if x == ']' as c_int
            || x == ')' as c_int
            || x == '}' as c_int
            || x == TK_IDENTIFIER
            || x == TK_NUMBER
            || x == TK_STRING
            || x == TK_FALSE
            || x == TK_NULL
            || x == TK_THIS
            || x == TK_TRUE =>
        {
            0
        }
        _ => 1,
    }
}

unsafe fn lexregexp(J: *mut js_State) -> c_int {
    let s;
    let mut g;
    let mut m;
    let mut i;
    let mut flags;
    let mut inclass = 0;

    textinit(J);

    while (*J).lexchar != '/' as c_int || inclass != 0 {
        if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
            jsY_error!(J, cstr!("regular expression not terminated"));
        } else if accept_ch!(J, '\\' as c_int) != 0 {
            if accept_ch!(J, '/' as c_int) != 0 {
                textpush(J, '/' as Rune);
            } else {
                textpush(J, '\\' as Rune);
                if (*J).lexchar == EOF || (*J).lexchar == '\n' as c_int {
                    jsY_error!(J, cstr!("regular expression not terminated"));
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
    expect_ch!(J, '/' as c_int);

    s = textend(J);

    g = 0;
    i = 0;
    m = 0;

    while jsY_isidentifierpart((*J).lexchar) != 0 {
        if accept_ch!(J, 'g' as c_int) != 0 {
            g += 1;
        } else if accept_ch!(J, 'i' as c_int) != 0 {
            i += 1;
        } else if accept_ch!(J, 'm' as c_int) != 0 {
            m += 1;
        } else {
            jsY_error!(J, cstr!("illegal flag in regular expression: %c"), (*J).lexchar);
        }
    }

    if g > 1 || i > 1 || m > 1 {
        jsY_error!(J, cstr!("duplicated flag in regular expression"));
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

unsafe fn isnlthcontext(last: c_int) -> c_int {
    match last {
        x if x == TK_BREAK || x == TK_CONTINUE || x == TK_RETURN || x == TK_THROW => 1,
        _ => 0,
    }
}

unsafe fn jsY_lexx(J: *mut js_State) -> c_int {
    (*J).newline = 0;

    loop {
        (*J).lexline = (*J).line;

        while jsY_iswhite((*J).lexchar) != 0 {
            jsY_next(J);
        }

        if accept_ch!(J, '\n' as c_int) != 0 {
            (*J).newline = 1;
            if isnlthcontext((*J).lasttoken) != 0 {
                return ';' as c_int;
            }
            continue;
        }

        if accept_ch!(J, '/' as c_int) != 0 {
            if accept_ch!(J, '/' as c_int) != 0 {
                lexlinecomment(J);
                continue;
            } else if accept_ch!(J, '*' as c_int) != 0 {
                if lexcomment(J) != 0 {
                    jsY_error!(J, cstr!("multi-line comment not terminated"));
                }
                continue;
            } else if isregexpcontext((*J).lasttoken) != 0 {
                return lexregexp(J);
            } else if accept_ch!(J, '=' as c_int) != 0 {
                return TK_DIV_ASS;
            } else {
                return '/' as c_int;
            }
        }

        if (*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int {
            return lexnumber(J);
        }

        match (*J).lexchar {
            x if x == '(' as c_int => { jsY_next(J); return '(' as c_int; }
            x if x == ')' as c_int => { jsY_next(J); return ')' as c_int; }
            x if x == ',' as c_int => { jsY_next(J); return ',' as c_int; }
            x if x == ':' as c_int => { jsY_next(J); return ':' as c_int; }
            x if x == ';' as c_int => { jsY_next(J); return ';' as c_int; }
            x if x == '?' as c_int => { jsY_next(J); return '?' as c_int; }
            x if x == '[' as c_int => { jsY_next(J); return '[' as c_int; }
            x if x == ']' as c_int => { jsY_next(J); return ']' as c_int; }
            x if x == '{' as c_int => { jsY_next(J); return '{' as c_int; }
            x if x == '}' as c_int => { jsY_next(J); return '}' as c_int; }
            x if x == '~' as c_int => { jsY_next(J); return '~' as c_int; }

            x if x == '\'' as c_int || x == '"' as c_int => {
                return lexstring(J);
            }

            x if x == '.' as c_int => {
                return lexnumber(J);
            }

            x if x == '<' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '<' as c_int) != 0 {
                    if accept_ch!(J, '=' as c_int) != 0 {
                        return TK_SHL_ASS;
                    }
                    return TK_SHL;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_LE;
                }
                return '<' as c_int;
            }
            x if x == '>' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '>' as c_int) != 0 {
                    if accept_ch!(J, '>' as c_int) != 0 {
                        if accept_ch!(J, '=' as c_int) != 0 {
                            return TK_USHR_ASS;
                        }
                        return TK_USHR;
                    }
                    if accept_ch!(J, '=' as c_int) != 0 {
                        return TK_SHR_ASS;
                    }
                    return TK_SHR;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_GE;
                }
                return '>' as c_int;
            }
            x if x == '=' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '=' as c_int) != 0 {
                    if accept_ch!(J, '=' as c_int) != 0 {
                        return TK_STRICTEQ;
                    }
                    return TK_EQ;
                }
                return '=' as c_int;
            }
            x if x == '!' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '=' as c_int) != 0 {
                    if accept_ch!(J, '=' as c_int) != 0 {
                        return TK_STRICTNE;
                    }
                    return TK_NE;
                }
                return '!' as c_int;
            }
            x if x == '+' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '+' as c_int) != 0 {
                    return TK_INC;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_ADD_ASS;
                }
                return '+' as c_int;
            }
            x if x == '-' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '-' as c_int) != 0 {
                    return TK_DEC;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_SUB_ASS;
                }
                return '-' as c_int;
            }
            x if x == '*' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_MUL_ASS;
                }
                return '*' as c_int;
            }
            x if x == '%' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_MOD_ASS;
                }
                return '%' as c_int;
            }
            x if x == '&' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '&' as c_int) != 0 {
                    return TK_AND;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_AND_ASS;
                }
                return '&' as c_int;
            }
            x if x == '|' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '|' as c_int) != 0 {
                    return TK_OR;
                }
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_OR_ASS;
                }
                return '|' as c_int;
            }
            x if x == '^' as c_int => {
                jsY_next(J);
                if accept_ch!(J, '=' as c_int) != 0 {
                    return TK_XOR_ASS;
                }
                return '^' as c_int;
            }
            EOF => {
                return 0;
            }
            _ => {}
        }

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
            jsY_error!(J, cstr!("unexpected character: '%c'"), (*J).lexchar);
        }
        jsY_error!(J, cstr!("unexpected character: \\u%04X"), (*J).lexchar);
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_initlex(J: *mut js_State, filename: *const c_char, source: *const c_char) {
    (*J).filename = filename;
    (*J).source = source;
    (*J).line = 1;
    (*J).lasttoken = 0;
    jsY_next(J);
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_lex(J: *mut js_State) -> c_int {
    (*J).lasttoken = jsY_lexx(J);
    (*J).lasttoken
}

unsafe fn lexjsonnumber(J: *mut js_State) -> c_int {
    let s = (*J).source.offset(-1);

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
        jsY_error!(J, cstr!("unexpected non-digit"));
    }

    if accept_ch!(J, '.' as c_int) != 0 {
        if isdigit((*J).lexchar) {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, cstr!("missing digits after decimal point"));
        }
    }

    if accept_ch!(J, 'e' as c_int) != 0 || accept_ch!(J, 'E' as c_int) != 0 {
        if (*J).lexchar == '-' as c_int || (*J).lexchar == '+' as c_int {
            jsY_next(J);
        }
        if isdigit((*J).lexchar) {
            while isdigit((*J).lexchar) {
                jsY_next(J);
            }
        } else {
            jsY_error!(J, cstr!("missing digits after exponent indicator"));
        }
    }

    (*J).number = crate::jsdtoa::js_strtod(s, std::ptr::null_mut());
    TK_NUMBER
}

unsafe fn lexjsonescape(J: *mut js_State) -> c_int {
    let mut x: c_int = 0;

    match (*J).lexchar {
        x2 if x2 == 'u' as c_int => {
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
        }
        x2 if x2 == '"' as c_int => {
            textpush(J, '"' as Rune);
            jsY_next(J);
        }
        x2 if x2 == '\\' as c_int => {
            textpush(J, '\\' as Rune);
            jsY_next(J);
        }
        x2 if x2 == '/' as c_int => {
            textpush(J, '/' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'b' as c_int => {
            textpush(J, '\u{08}' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'f' as c_int => {
            textpush(J, '\u{0c}' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'n' as c_int => {
            textpush(J, '\n' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 'r' as c_int => {
            textpush(J, '\r' as Rune);
            jsY_next(J);
        }
        x2 if x2 == 't' as c_int => {
            textpush(J, '\t' as Rune);
            jsY_next(J);
        }
        _ => {
            jsY_error!(J, cstr!("invalid escape sequence"));
        }
    }
    0
}

unsafe fn lexjsonstring(J: *mut js_State) -> c_int {
    let s;

    textinit(J);

    while (*J).lexchar != '"' as c_int {
        if (*J).lexchar == EOF {
            jsY_error!(J, cstr!("unterminated string"));
        } else if (*J).lexchar < 32 {
            jsY_error!(J, cstr!("invalid control character in string"));
        } else if accept_ch!(J, '\\' as c_int) != 0 {
            lexjsonescape(J);
        } else {
            textpush(J, (*J).lexchar);
            jsY_next(J);
        }
    }
    expect_ch!(J, '"' as c_int);

    s = textend(J);
    (*J).text = s;
    TK_STRING
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsY_lexjson(J: *mut js_State) -> c_int {
    loop {
        (*J).lexline = (*J).line;

        while jsY_iswhite((*J).lexchar) != 0 || (*J).lexchar == '\n' as c_int {
            jsY_next(J);
        }

        if ((*J).lexchar >= '0' as c_int && (*J).lexchar <= '9' as c_int) || (*J).lexchar == '-' as c_int {
            return lexjsonnumber(J);
        }

        match (*J).lexchar {
            x if x == ',' as c_int => { jsY_next(J); return ',' as c_int; }
            x if x == ':' as c_int => { jsY_next(J); return ':' as c_int; }
            x if x == '[' as c_int => { jsY_next(J); return '[' as c_int; }
            x if x == ']' as c_int => { jsY_next(J); return ']' as c_int; }
            x if x == '{' as c_int => { jsY_next(J); return '{' as c_int; }
            x if x == '}' as c_int => { jsY_next(J); return '}' as c_int; }
            x if x == '"' as c_int => {
                jsY_next(J);
                return lexjsonstring(J);
            }
            x if x == 'f' as c_int => {
                jsY_next(J);
                expect_ch!(J, 'a' as c_int);
                expect_ch!(J, 'l' as c_int);
                expect_ch!(J, 's' as c_int);
                expect_ch!(J, 'e' as c_int);
                return TK_FALSE;
            }
            x if x == 'n' as c_int => {
                jsY_next(J);
                expect_ch!(J, 'u' as c_int);
                expect_ch!(J, 'l' as c_int);
                expect_ch!(J, 'l' as c_int);
                return TK_NULL;
            }
            x if x == 't' as c_int => {
                jsY_next(J);
                expect_ch!(J, 'r' as c_int);
                expect_ch!(J, 'u' as c_int);
                expect_ch!(J, 'e' as c_int);
                return TK_TRUE;
            }
            EOF => {
                return 0;
            }
            _ => {}
        }

        if (*J).lexchar >= 0x20 && (*J).lexchar <= 0x7E {
            jsY_error!(J, cstr!("unexpected character: '%c'"), (*J).lexchar);
        }
        jsY_error!(J, cstr!("unexpected character: \\u%04X"), (*J).lexchar);
    }
}
