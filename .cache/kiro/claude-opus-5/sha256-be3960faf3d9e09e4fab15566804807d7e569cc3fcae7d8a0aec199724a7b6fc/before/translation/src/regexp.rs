//! Translation of src/regexp.c
//!
//! Self-contained regular expression engine (does not use js_State).
//! Exports js_regcomp, js_regcompx, js_regexec, js_regfree, js_regfreex
//! (macro-renamed forms of regcomp/regcompx/regexec/regfree/regfreex).

use crate::jsi::*;
use crate::utf::{jsU_chartorune, jsU_isalpharune, jsU_toupperrune};

use core::panic::AssertUnwindSafe;

/* #define emit regemit  -- static fn named `emit` below */
/* #define next regnext  -- static fn named `next` below */
/* #define accept regaccept -- static fn named `accept` below */

/* #define nelem(a) (int)(sizeof (a) / sizeof (a)[0]) : use nelem! macro */

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

/* C EOF */
const EOF: c_int = -1;

/* ------------------------------------------------------------------ */
/* Structs                                                            */
/* ------------------------------------------------------------------ */

#[repr(C)]
pub struct Reclass {
    pub end: *mut Rune,
    pub spans: [Rune; REG_MAXSPAN],
}

#[repr(C)]
pub struct Reprog {
    pub start: *mut Reinst,
    pub end: *mut Reinst,
    pub cclass: *mut Reclass,
    pub flags: c_int,
    pub nsub: c_int,
}

#[repr(C)]
pub struct Renode {
    pub ty: c_uchar, /* `type` in C */
    pub ng: c_uchar,
    pub m: c_uchar,
    pub n: c_uchar,
    pub c: Rune,
    pub cc: c_int,
    pub x: *mut Renode,
    pub y: *mut Renode,
}

#[repr(C)]
pub struct Reinst {
    pub opcode: c_uchar,
    pub n: c_uchar,
    pub c: Rune,
    pub cc: *mut Reclass,
    pub x: *mut Reinst,
    pub y: *mut Reinst,
}

#[repr(C)]
pub struct cstate {
    pub prog: *mut Reprog,
    pub pstart: *mut Renode,
    pub pend: *mut Renode,

    pub source: *const c_char,
    pub ncclass: c_int,
    pub nsub: c_int,
    pub sub: [*mut Renode; REG_MAXSUB],

    pub lookahead: c_int,
    pub yychar: Rune,
    pub yycc: *mut Reclass,
    pub yymin: c_int,
    pub yymax: c_int,

    pub error: *const c_char,
    /* jmp_buf kaboom -- replaced by unwinding */

    pub cclass: [Reclass; REG_MAXCLASS],
}

/* Marker payload thrown by die() and caught in regcompx. */
struct ReDie;

/* ------------------------------------------------------------------ */
/* die (was longjmp to g->kaboom)                                     */
/* ------------------------------------------------------------------ */

unsafe fn die(g: *mut cstate, message: *const c_char) -> ! {
    unsafe {
        (*g).error = message;
        std::panic::panic_any(ReDie)
    }
}

unsafe fn canon(c: Rune) -> c_int {
    unsafe {
        let u: Rune = jsU_toupperrune(c);
        if c >= 128 && u < 128 {
            return c;
        }
        u
    }
}

/* Scan */

const L_CHAR: c_int = 256;
const L_CCLASS: c_int = 257; /* character class */
const L_NCCLASS: c_int = 258; /* negative character class */
const L_NC: c_int = 259; /* "(?:" no capture */
const L_PLA: c_int = 260; /* "(?=" positive lookahead */
const L_NLA: c_int = 261; /* "(?!" negative lookahead */
const L_WORD: c_int = 262; /* "\b" word boundary */
const L_NWORD: c_int = 263; /* "\B" non-word boundary */
const L_REF: c_int = 264; /* "\1" back-reference */
const L_COUNT: c_int = 265; /* {M,N} */

unsafe fn hex(g: *mut cstate, c: c_int) -> c_int {
    unsafe {
        if c >= '0' as c_int && c <= '9' as c_int {
            return c - '0' as c_int;
        }
        if c >= 'a' as c_int && c <= 'f' as c_int {
            return c - 'a' as c_int + 0xA;
        }
        if c >= 'A' as c_int && c <= 'F' as c_int {
            return c - 'A' as c_int + 0xA;
        }
        die(g, c"invalid escape sequence".as_ptr());
    }
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    unsafe {
        if c >= '0' as c_int && c <= '9' as c_int {
            return c - '0' as c_int;
        }
        die(g, c"invalid quantifier".as_ptr());
    }
}

const ESCAPES: &core::ffi::CStr = c"BbDdSsWw^$\\.*+?()[]{}|-0123456789";

unsafe fn isunicodeletter(c: c_int) -> c_int {
    unsafe {
        ((c >= 'a' as c_int && c <= 'z' as c_int)
            || (c >= 'A' as c_int && c <= 'Z' as c_int)
            || jsU_isalpharune(c) != 0) as c_int
    }
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    unsafe {
        if *(*g).source == 0 {
            (*g).yychar = EOF;
            return 0;
        }
        (*g).source = (*g).source.offset(jsU_chartorune(&raw mut (*g).yychar, (*g).source) as isize);
        if (*g).yychar == '\\' as c_int {
            if *(*g).source == 0 {
                die(g, c"unterminated escape sequence".as_ptr());
            }
            (*g).source =
                (*g).source.offset(jsU_chartorune(&raw mut (*g).yychar, (*g).source) as isize);
            match (*g).yychar {
                x if x == 'f' as c_int => {
                    (*g).yychar = '\u{c}' as c_int;
                    return 0;
                }
                x if x == 'n' as c_int => {
                    (*g).yychar = '\n' as c_int;
                    return 0;
                }
                x if x == 'r' as c_int => {
                    (*g).yychar = '\r' as c_int;
                    return 0;
                }
                x if x == 't' as c_int => {
                    (*g).yychar = '\t' as c_int;
                    return 0;
                }
                x if x == 'v' as c_int => {
                    (*g).yychar = '\u{b}' as c_int;
                    return 0;
                }
                x if x == 'c' as c_int => {
                    if *(*g).source.offset(0) == 0 {
                        die(g, c"unterminated escape sequence".as_ptr());
                    }
                    let ch = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar = ch & 31;
                    return 0;
                }
                x if x == 'x' as c_int => {
                    if *(*g).source.offset(0) == 0 || *(*g).source.offset(1) == 0 {
                        die(g, c"unterminated escape sequence".as_ptr());
                    }
                    let c0 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar = hex(g, c0) << 4;
                    let c1 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar += hex(g, c1);
                    if (*g).yychar == 0 {
                        (*g).yychar = '0' as c_int;
                        return 1;
                    }
                    return 1;
                }
                x if x == 'u' as c_int => {
                    if *(*g).source.offset(0) == 0
                        || *(*g).source.offset(1) == 0
                        || *(*g).source.offset(2) == 0
                        || *(*g).source.offset(3) == 0
                    {
                        die(g, c"unterminated escape sequence".as_ptr());
                    }
                    let c0 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar = hex(g, c0) << 12;
                    let c1 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar += hex(g, c1) << 8;
                    let c2 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar += hex(g, c2) << 4;
                    let c3 = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    (*g).yychar += hex(g, c3);
                    if (*g).yychar == 0 {
                        (*g).yychar = '0' as c_int;
                        return 1;
                    }
                    return 1;
                }
                0 => {
                    (*g).yychar = '0' as c_int;
                    return 1;
                }
                _ => {}
            }
            if !strchr(ESCAPES.as_ptr(), (*g).yychar).is_null() {
                return 1;
            }
            if isunicodeletter((*g).yychar) != 0 || (*g).yychar == '_' as c_int {
                /* check identity escape */
                die(g, c"invalid escape character".as_ptr());
            }
            return 0;
        }
        0
    }
}

unsafe fn lexcount(g: *mut cstate) -> c_int {
    unsafe {
        (*g).yychar = *(*g).source as c_int;
        (*g).source = (*g).source.offset(1);

        (*g).yymin = dec(g, (*g).yychar);
        (*g).yychar = *(*g).source as c_int;
        (*g).source = (*g).source.offset(1);
        while (*g).yychar != ',' as c_int && (*g).yychar != '}' as c_int {
            (*g).yymin = (*g).yymin * 10 + dec(g, (*g).yychar);
            (*g).yychar = *(*g).source as c_int;
            (*g).source = (*g).source.offset(1);
            if (*g).yymin >= REPINF {
                die(g, c"numeric overflow".as_ptr());
            }
        }

        if (*g).yychar == ',' as c_int {
            (*g).yychar = *(*g).source as c_int;
            (*g).source = (*g).source.offset(1);
            if (*g).yychar == '}' as c_int {
                (*g).yymax = REPINF;
            } else {
                (*g).yymax = dec(g, (*g).yychar);
                (*g).yychar = *(*g).source as c_int;
                (*g).source = (*g).source.offset(1);
                while (*g).yychar != '}' as c_int {
                    (*g).yymax = (*g).yymax * 10 + dec(g, (*g).yychar);
                    (*g).yychar = *(*g).source as c_int;
                    (*g).source = (*g).source.offset(1);
                    if (*g).yymax >= REPINF {
                        die(g, c"numeric overflow".as_ptr());
                    }
                }
            }
        } else {
            (*g).yymax = (*g).yymin;
        }

        L_COUNT
    }
}

unsafe fn newcclass(g: *mut cstate) {
    unsafe {
        if (*g).ncclass >= REG_MAXCLASS as c_int {
            die(g, c"too many character classes".as_ptr());
        }
        (*g).yycc = (&raw mut (*g).cclass[0]).offset((*g).ncclass as isize);
        (*g).ncclass += 1;
        (*(*g).yycc).end = &raw mut (*(*g).yycc).spans[0];
    }
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    unsafe {
        let cc: *mut Reclass = (*g).yycc;
        let mut p: *mut Rune;

        if a > b {
            die(g, c"invalid character class range".as_ptr());
        }

        /* extend existing ranges if they overlap */
        p = &raw mut (*cc).spans[0];
        while p < (*cc).end {
            /* completely inside old range */
            if a >= *p.offset(0) && b <= *p.offset(1) {
                return;
            }

            /* completely swallows old range */
            if a < *p.offset(0) && b >= *p.offset(1) {
                *p.offset(0) = a;
                *p.offset(1) = b;
                return;
            }

            /* extend at start */
            if b >= *p.offset(0) - 1 && b <= *p.offset(1) && a < *p.offset(0) {
                *p.offset(0) = a;
                return;
            }

            /* extend at end */
            if a >= *p.offset(0) && a <= *p.offset(1) + 1 && b > *p.offset(1) {
                *p.offset(1) = b;
                return;
            }

            p = p.offset(2);
        }

        if (*cc).end.offset(2) >= (&raw mut (*cc).spans[0]).offset(REG_MAXSPAN as isize) {
            die(g, c"too many character class ranges".as_ptr());
        }
        *(*cc).end = a;
        (*cc).end = (*cc).end.offset(1);
        *(*cc).end = b;
        (*cc).end = (*cc).end.offset(1);
    }
}

unsafe fn addranges_d(g: *mut cstate) {
    unsafe {
        addrange(g, '0' as c_int, '9' as c_int);
    }
}

unsafe fn addranges_D(g: *mut cstate) {
    unsafe {
        addrange(g, 0, '0' as c_int - 1);
        addrange(g, '9' as c_int + 1, 0xFFFF);
    }
}

unsafe fn addranges_s(g: *mut cstate) {
    unsafe {
        addrange(g, 0x9, 0xD);
        addrange(g, 0x20, 0x20);
        addrange(g, 0xA0, 0xA0);
        addrange(g, 0x2028, 0x2029);
        addrange(g, 0xFEFF, 0xFEFF);
    }
}

unsafe fn addranges_S(g: *mut cstate) {
    unsafe {
        addrange(g, 0, 0x9 - 1);
        addrange(g, 0xD + 1, 0x20 - 1);
        addrange(g, 0x20 + 1, 0xA0 - 1);
        addrange(g, 0xA0 + 1, 0x2028 - 1);
        addrange(g, 0x2029 + 1, 0xFEFF - 1);
        addrange(g, 0xFEFF + 1, 0xFFFF);
    }
}

unsafe fn addranges_w(g: *mut cstate) {
    unsafe {
        addrange(g, '0' as c_int, '9' as c_int);
        addrange(g, 'A' as c_int, 'Z' as c_int);
        addrange(g, '_' as c_int, '_' as c_int);
        addrange(g, 'a' as c_int, 'z' as c_int);
    }
}

unsafe fn addranges_W(g: *mut cstate) {
    unsafe {
        addrange(g, 0, '0' as c_int - 1);
        addrange(g, '9' as c_int + 1, 'A' as c_int - 1);
        addrange(g, 'Z' as c_int + 1, '_' as c_int - 1);
        addrange(g, '_' as c_int + 1, 'a' as c_int - 1);
        addrange(g, 'z' as c_int + 1, 0xFFFF);
    }
}

unsafe fn lexclass(g: *mut cstate) -> c_int {
    unsafe {
        let mut ty: c_int = L_CCLASS;
        let mut quoted: c_int;
        let mut havesave: c_int;
        let mut havedash: c_int;
        let mut save: Rune = 0;

        newcclass(g);

        quoted = nextrune(g);
        if quoted == 0 && (*g).yychar == '^' as c_int {
            ty = L_NCCLASS;
            quoted = nextrune(g);
        }

        havesave = 0;
        havedash = 0;
        loop {
            if (*g).yychar == EOF {
                die(g, c"unterminated character class".as_ptr());
            }
            if quoted == 0 && (*g).yychar == ']' as c_int {
                break;
            }

            if quoted == 0 && (*g).yychar == '-' as c_int {
                if havesave != 0 {
                    if havedash != 0 {
                        addrange(g, save, '-' as c_int);
                        havesave = 0;
                        havedash = 0;
                    } else {
                        havedash = 1;
                    }
                } else {
                    save = '-' as c_int;
                    havesave = 1;
                }
            } else if quoted != 0 && !strchr(c"DSWdsw".as_ptr(), (*g).yychar).is_null() {
                if havesave != 0 {
                    addrange(g, save, save);
                    if havedash != 0 {
                        addrange(g, '-' as c_int, '-' as c_int);
                    }
                }
                match (*g).yychar {
                    x if x == 'd' as c_int => addranges_d(g),
                    x if x == 's' as c_int => addranges_s(g),
                    x if x == 'w' as c_int => addranges_w(g),
                    x if x == 'D' as c_int => addranges_D(g),
                    x if x == 'S' as c_int => addranges_S(g),
                    x if x == 'W' as c_int => addranges_W(g),
                    _ => {}
                }
                havesave = 0;
                havedash = 0;
            } else {
                if quoted != 0 {
                    if (*g).yychar == 'b' as c_int {
                        (*g).yychar = '\u{8}' as c_int;
                    } else if (*g).yychar == '0' as c_int {
                        (*g).yychar = 0;
                    }
                    /* else identity escape */
                }
                if havesave != 0 {
                    if havedash != 0 {
                        addrange(g, save, (*g).yychar);
                        havesave = 0;
                        havedash = 0;
                    } else {
                        addrange(g, save, save);
                        save = (*g).yychar;
                    }
                } else {
                    save = (*g).yychar;
                    havesave = 1;
                }
            }

            quoted = nextrune(g);
        }

        if havesave != 0 {
            addrange(g, save, save);
            if havedash != 0 {
                addrange(g, '-' as c_int, '-' as c_int);
            }
        }

        ty
    }
}

unsafe fn lex(g: *mut cstate) -> c_int {
    unsafe {
        let quoted: c_int = nextrune(g);
        if quoted != 0 {
            match (*g).yychar {
                x if x == 'b' as c_int => return L_WORD,
                x if x == 'B' as c_int => return L_NWORD,
                x if x == 'd' as c_int => {
                    newcclass(g);
                    addranges_d(g);
                    return L_CCLASS;
                }
                x if x == 's' as c_int => {
                    newcclass(g);
                    addranges_s(g);
                    return L_CCLASS;
                }
                x if x == 'w' as c_int => {
                    newcclass(g);
                    addranges_w(g);
                    return L_CCLASS;
                }
                x if x == 'D' as c_int => {
                    newcclass(g);
                    addranges_d(g);
                    return L_NCCLASS;
                }
                x if x == 'S' as c_int => {
                    newcclass(g);
                    addranges_s(g);
                    return L_NCCLASS;
                }
                x if x == 'W' as c_int => {
                    newcclass(g);
                    addranges_w(g);
                    return L_NCCLASS;
                }
                x if x == '0' as c_int => {
                    (*g).yychar = 0;
                    return L_CHAR;
                }
                _ => {}
            }
            if (*g).yychar >= '0' as c_int && (*g).yychar <= '9' as c_int {
                (*g).yychar -= '0' as c_int;
                if *(*g).source as c_int >= '0' as c_int && *(*g).source as c_int <= '9' as c_int {
                    (*g).yychar = (*g).yychar * 10 + *(*g).source as c_int - '0' as c_int;
                    (*g).source = (*g).source.offset(1);
                }
                return L_REF;
            }
            return L_CHAR;
        }

        match (*g).yychar {
            EOF => return (*g).yychar,
            x if x == '$' as c_int
                || x == ')' as c_int
                || x == '*' as c_int
                || x == '+' as c_int
                || x == '.' as c_int
                || x == '?' as c_int
                || x == '^' as c_int
                || x == '|' as c_int =>
            {
                return (*g).yychar;
            }
            _ => {}
        }

        if (*g).yychar == '{' as c_int {
            return lexcount(g);
        }
        if (*g).yychar == '[' as c_int {
            return lexclass(g);
        }
        if (*g).yychar == '(' as c_int {
            if *(*g).source.offset(0) as c_int == '?' as c_int {
                if *(*g).source.offset(1) as c_int == ':' as c_int {
                    (*g).source = (*g).source.offset(2);
                    return L_NC;
                }
                if *(*g).source.offset(1) as c_int == '=' as c_int {
                    (*g).source = (*g).source.offset(2);
                    return L_PLA;
                }
                if *(*g).source.offset(1) as c_int == '!' as c_int {
                    (*g).source = (*g).source.offset(2);
                    return L_NLA;
                }
            }
            return '(' as c_int;
        }

        L_CHAR
    }
}

/* Parse */

const P_CAT: c_uchar = 0;
const P_ALT: c_uchar = 1;
const P_REP: c_uchar = 2;
const P_BOL: c_uchar = 3;
const P_EOL: c_uchar = 4;
const P_WORD: c_uchar = 5;
const P_NWORD: c_uchar = 6;
const P_PAR: c_uchar = 7;
const P_PLA: c_uchar = 8;
const P_NLA: c_uchar = 9;
const P_ANY: c_uchar = 10;
const P_CHAR: c_uchar = 11;
const P_CCLASS: c_uchar = 12;
const P_NCCLASS: c_uchar = 13;
const P_REF: c_uchar = 14;

unsafe fn newnode(g: *mut cstate, ty: c_int) -> *mut Renode {
    unsafe {
        let node: *mut Renode = (*g).pend;
        (*g).pend = (*g).pend.offset(1);
        (*node).ty = ty as c_uchar;
        (*node).cc = -1;
        (*node).c = 0;
        (*node).ng = 0;
        (*node).m = 0;
        (*node).n = 0;
        (*node).x = core::ptr::null_mut();
        (*node).y = core::ptr::null_mut();
        node
    }
}

unsafe fn empty(node: *mut Renode) -> c_int {
    unsafe {
        if node.is_null() {
            return 1;
        }
        match (*node).ty {
            P_CAT => (empty((*node).x) != 0 && empty((*node).y) != 0) as c_int,
            P_ALT => (empty((*node).x) != 0 || empty((*node).y) != 0) as c_int,
            P_REP => (empty((*node).x) != 0 || (*node).m == 0) as c_int,
            P_PAR => empty((*node).x),
            P_REF => empty((*node).x),
            P_ANY | P_CHAR | P_CCLASS | P_NCCLASS => 0,
            _ => 1,
        }
    }
}

unsafe fn newrep(g: *mut cstate, atom: *mut Renode, ng: c_int, min: c_int, max: c_int) -> *mut Renode {
    unsafe {
        let rep: *mut Renode = newnode(g, P_REP as c_int);
        if max == REPINF && empty(atom) != 0 {
            die(g, c"infinite loop matching the empty string".as_ptr());
        }
        (*rep).ng = ng as c_uchar;
        (*rep).m = min as c_uchar;
        (*rep).n = max as c_uchar;
        (*rep).x = atom;
        rep
    }
}

unsafe fn next(g: *mut cstate) {
    unsafe {
        (*g).lookahead = lex(g);
    }
}

unsafe fn accept(g: *mut cstate, t: c_int) -> c_int {
    unsafe {
        if (*g).lookahead == t {
            next(g);
            return 1;
        }
        0
    }
}

unsafe fn parseatom(g: *mut cstate) -> *mut Renode {
    unsafe {
        let atom: *mut Renode;
        if (*g).lookahead == L_CHAR {
            atom = newnode(g, P_CHAR as c_int);
            (*atom).c = (*g).yychar;
            next(g);
            return atom;
        }
        if (*g).lookahead == L_CCLASS {
            atom = newnode(g, P_CCLASS as c_int);
            (*atom).cc = ((*g).yycc.offset_from(&raw mut (*g).cclass[0])) as c_int;
            next(g);
            return atom;
        }
        if (*g).lookahead == L_NCCLASS {
            atom = newnode(g, P_NCCLASS as c_int);
            (*atom).cc = ((*g).yycc.offset_from(&raw mut (*g).cclass[0])) as c_int;
            next(g);
            return atom;
        }
        if (*g).lookahead == L_REF {
            atom = newnode(g, P_REF as c_int);
            if (*g).yychar == 0
                || (*g).yychar >= (*g).nsub
                || (*g).sub[(*g).yychar as usize].is_null()
            {
                die(g, c"invalid back-reference".as_ptr());
            }
            (*atom).n = (*g).yychar as c_uchar;
            (*atom).x = (*g).sub[(*g).yychar as usize];
            next(g);
            return atom;
        }
        if accept(g, '.' as c_int) != 0 {
            return newnode(g, P_ANY as c_int);
        }
        if accept(g, '(' as c_int) != 0 {
            atom = newnode(g, P_PAR as c_int);
            if (*g).nsub == REG_MAXSUB as c_int {
                die(g, c"too many captures".as_ptr());
            }
            (*atom).n = (*g).nsub as c_uchar;
            (*g).nsub += 1;
            (*atom).x = parsealt(g);
            (*g).sub[(*atom).n as usize] = atom;
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('".as_ptr());
            }
            return atom;
        }
        if accept(g, L_NC) != 0 {
            let atom2 = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('".as_ptr());
            }
            return atom2;
        }
        if accept(g, L_PLA) != 0 {
            atom = newnode(g, P_PLA as c_int);
            (*atom).x = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('".as_ptr());
            }
            return atom;
        }
        if accept(g, L_NLA) != 0 {
            atom = newnode(g, P_NLA as c_int);
            (*atom).x = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('".as_ptr());
            }
            return atom;
        }
        die(g, c"syntax error".as_ptr());
    }
}

unsafe fn parserep(g: *mut cstate) -> *mut Renode {
    unsafe {
        let atom: *mut Renode;

        if accept(g, '^' as c_int) != 0 {
            return newnode(g, P_BOL as c_int);
        }
        if accept(g, '$' as c_int) != 0 {
            return newnode(g, P_EOL as c_int);
        }
        if accept(g, L_WORD) != 0 {
            return newnode(g, P_WORD as c_int);
        }
        if accept(g, L_NWORD) != 0 {
            return newnode(g, P_NWORD as c_int);
        }

        atom = parseatom(g);
        if (*g).lookahead == L_COUNT {
            let min = (*g).yymin;
            let max = (*g).yymax;
            next(g);
            if max < min {
                die(g, c"invalid quantifier".as_ptr());
            }
            return newrep(g, atom, accept(g, '?' as c_int), min, max);
        }
        if accept(g, '*' as c_int) != 0 {
            return newrep(g, atom, accept(g, '?' as c_int), 0, REPINF);
        }
        if accept(g, '+' as c_int) != 0 {
            return newrep(g, atom, accept(g, '?' as c_int), 1, REPINF);
        }
        if accept(g, '?' as c_int) != 0 {
            return newrep(g, atom, accept(g, '?' as c_int), 0, 1);
        }
        atom
    }
}

unsafe fn parsecat(g: *mut cstate) -> *mut Renode {
    unsafe {
        let mut head: *mut Renode;
        let mut tail: *mut *mut Renode;
        if (*g).lookahead != EOF && (*g).lookahead != '|' as c_int && (*g).lookahead != ')' as c_int
        {
            /* Build a right-leaning tree by splicing in new 'cat' at the tail. */
            head = parserep(g);
            tail = &raw mut head;
            while (*g).lookahead != EOF
                && (*g).lookahead != '|' as c_int
                && (*g).lookahead != ')' as c_int
            {
                let cat = newnode(g, P_CAT as c_int);
                (*cat).x = *tail;
                (*cat).y = parserep(g);
                *tail = cat;
                tail = &raw mut (*cat).y;
            }
            return head;
        }
        core::ptr::null_mut()
    }
}

unsafe fn parsealt(g: *mut cstate) -> *mut Renode {
    unsafe {
        let mut alt: *mut Renode;
        alt = parsecat(g);
        while accept(g, '|' as c_int) != 0 {
            let x = alt;
            alt = newnode(g, P_ALT as c_int);
            (*alt).x = x;
            (*alt).y = parsecat(g);
        }
        alt
    }
}

/* Compile */

const I_END: c_int = 0;
const I_JUMP: c_int = 1;
const I_SPLIT: c_int = 2;
const I_PLA: c_int = 3;
const I_NLA: c_int = 4;
const I_ANYNL: c_int = 5;
const I_ANY: c_int = 6;
const I_CHAR: c_int = 7;
const I_CCLASS: c_int = 8;
const I_NCCLASS: c_int = 9;
const I_REF: c_int = 10;
const I_BOL: c_int = 11;
const I_EOL: c_int = 12;
const I_WORD: c_int = 13;
const I_NWORD: c_int = 14;
const I_LPAR: c_int = 15;
const I_RPAR: c_int = 16;

unsafe fn count(g: *mut cstate, node: *mut Renode, depth: c_int) -> c_int {
    unsafe {
        let min: c_int;
        let max: c_int;
        let n: c_int;
        if node.is_null() {
            return 0;
        }
        let depth = depth + 1;
        if depth > REG_MAXREC {
            die(g, c"stack overflow".as_ptr());
        }
        match (*node).ty {
            P_CAT => count(g, (*node).x, depth) + count(g, (*node).y, depth),
            P_ALT => count(g, (*node).x, depth) + count(g, (*node).y, depth) + 2,
            P_REP => {
                min = (*node).m as c_int;
                max = (*node).n as c_int;
                if min == max {
                    n = count(g, (*node).x, depth) * min;
                } else if max < REPINF {
                    n = count(g, (*node).x, depth) * max + (max - min);
                } else {
                    n = count(g, (*node).x, depth) * (min + 1) + 2;
                }
                if n < 0 || n > REG_MAXPROG {
                    die(g, c"program too large".as_ptr());
                }
                n
            }
            P_PAR => count(g, (*node).x, depth) + 2,
            P_PLA => count(g, (*node).x, depth) + 2,
            P_NLA => count(g, (*node).x, depth) + 2,
            _ => 1,
        }
    }
}

unsafe fn emit(prog: *mut Reprog, opcode: c_int) -> *mut Reinst {
    unsafe {
        let inst: *mut Reinst = (*prog).end;
        (*prog).end = (*prog).end.offset(1);
        (*inst).opcode = opcode as c_uchar;
        (*inst).n = 0;
        (*inst).c = 0;
        (*inst).cc = core::ptr::null_mut();
        (*inst).x = core::ptr::null_mut();
        (*inst).y = core::ptr::null_mut();
        inst
    }
}

unsafe fn compile(prog: *mut Reprog, node_in: *mut Renode) {
    unsafe {
        let mut node = node_in;
        let mut inst: *mut Reinst;
        let mut split: *mut Reinst;
        let mut jump: *mut Reinst;
        let mut i: c_int;

        'loop_: loop {
            if node.is_null() {
                return;
            }

            match (*node).ty {
                P_CAT => {
                    compile(prog, (*node).x);
                    node = (*node).y;
                    continue 'loop_;
                }

                P_ALT => {
                    split = emit(prog, I_SPLIT);
                    compile(prog, (*node).x);
                    jump = emit(prog, I_JUMP);
                    compile(prog, (*node).y);
                    (*split).x = split.offset(1);
                    (*split).y = jump.offset(1);
                    (*jump).x = (*prog).end;
                }

                P_REP => {
                    inst = core::ptr::null_mut(); /* silence compiler warning. assert(node->m > 0). */
                    i = 0;
                    while i < (*node).m as c_int {
                        inst = (*prog).end;
                        compile(prog, (*node).x);
                        i += 1;
                    }
                    if (*node).m == (*node).n {
                        break 'loop_;
                    }
                    if (*node).n < REPINF as c_uchar {
                        i = (*node).m as c_int;
                        while i < (*node).n as c_int {
                            split = emit(prog, I_SPLIT);
                            compile(prog, (*node).x);
                            if (*node).ng != 0 {
                                (*split).y = split.offset(1);
                                (*split).x = (*prog).end;
                            } else {
                                (*split).x = split.offset(1);
                                (*split).y = (*prog).end;
                            }
                            i += 1;
                        }
                    } else if (*node).m == 0 {
                        split = emit(prog, I_SPLIT);
                        compile(prog, (*node).x);
                        jump = emit(prog, I_JUMP);
                        if (*node).ng != 0 {
                            (*split).y = split.offset(1);
                            (*split).x = (*prog).end;
                        } else {
                            (*split).x = split.offset(1);
                            (*split).y = (*prog).end;
                        }
                        (*jump).x = split;
                    } else {
                        split = emit(prog, I_SPLIT);
                        if (*node).ng != 0 {
                            (*split).y = inst;
                            (*split).x = (*prog).end;
                        } else {
                            (*split).x = inst;
                            (*split).y = (*prog).end;
                        }
                    }
                }

                P_BOL => {
                    emit(prog, I_BOL);
                }
                P_EOL => {
                    emit(prog, I_EOL);
                }
                P_WORD => {
                    emit(prog, I_WORD);
                }
                P_NWORD => {
                    emit(prog, I_NWORD);
                }

                P_PAR => {
                    inst = emit(prog, I_LPAR);
                    (*inst).n = (*node).n;
                    compile(prog, (*node).x);
                    inst = emit(prog, I_RPAR);
                    (*inst).n = (*node).n;
                }
                P_PLA => {
                    split = emit(prog, I_PLA);
                    compile(prog, (*node).x);
                    emit(prog, I_END);
                    (*split).x = split.offset(1);
                    (*split).y = (*prog).end;
                }
                P_NLA => {
                    split = emit(prog, I_NLA);
                    compile(prog, (*node).x);
                    emit(prog, I_END);
                    (*split).x = split.offset(1);
                    (*split).y = (*prog).end;
                }

                P_ANY => {
                    emit(prog, I_ANY);
                }
                P_CHAR => {
                    inst = emit(prog, I_CHAR);
                    (*inst).c = if (*prog).flags & REG_ICASE != 0 {
                        canon((*node).c)
                    } else {
                        (*node).c
                    };
                }
                P_CCLASS => {
                    inst = emit(prog, I_CCLASS);
                    (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                }
                P_NCCLASS => {
                    inst = emit(prog, I_NCCLASS);
                    (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                }
                P_REF => {
                    inst = emit(prog, I_REF);
                    (*inst).n = (*node).n;
                }
                _ => {}
            }
            break 'loop_;
        }
    }
}

/* ------------------------------------------------------------------ */
/* regcompx                                                           */
/* ------------------------------------------------------------------ */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcompx(
    alloc: ReAlloc,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    unsafe {
        crate::except::install_panic_hook();
        /* struct cstate g; -- allocate on the heap-sized boxed value to avoid
         * huge stack objects, but keep C semantics of a local. Use a Box. */
        let mut gbox: Box<cstate> = Box::new(core::mem::zeroed());
        let g: *mut cstate = &raw mut *gbox;

        (*g).pstart = core::ptr::null_mut();
        (*g).prog = core::ptr::null_mut();

        /* if (setjmp(g.kaboom)) { ... } : replaced by catch_unwind of the body */
        let body = AssertUnwindSafe(|| -> *mut Reprog {
            regcompx_body(g, alloc, ctx, pattern, cflags, errorp)
        });

        match std::panic::catch_unwind(body) {
            Ok(prog) => prog,
            Err(payload) => {
                if payload.downcast_ref::<ReDie>().is_some() {
                    /* longjmp target */
                    if !errorp.is_null() {
                        *errorp = (*g).error;
                    }
                    (alloc.unwrap())(ctx, (*g).pstart as *mut c_void, 0);
                    if !(*g).prog.is_null() {
                        (alloc.unwrap())(ctx, (*(*g).prog).cclass as *mut c_void, 0);
                        (alloc.unwrap())(ctx, (*(*g).prog).start as *mut c_void, 0);
                        (alloc.unwrap())(ctx, (*g).prog as *mut c_void, 0);
                    }
                    core::ptr::null_mut()
                } else {
                    std::panic::resume_unwind(payload);
                }
            }
        }
    }
}

/* The body of regcompx that runs "after setjmp returns 0". */
unsafe fn regcompx_body(
    g: *mut cstate,
    alloc: ReAlloc,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    unsafe {
        let node: *mut Renode;
        let split: *mut Reinst;
        let jump: *mut Reinst;
        let mut i: c_int;
        let mut n: c_int;

        (*g).prog =
            (alloc.unwrap())(ctx, core::ptr::null_mut(), core::mem::size_of::<Reprog>() as c_int)
                as *mut Reprog;
        if (*g).prog.is_null() {
            die(g, c"cannot allocate regular expression".as_ptr());
        }
        (*(*g).prog).start = core::ptr::null_mut();
        (*(*g).prog).cclass = core::ptr::null_mut();

        n = (strlen(pattern) * 2) as c_int;
        if n > REG_MAXPROG {
            die(g, c"program too large".as_ptr());
        }
        if n > 0 {
            (*g).pstart = (alloc.unwrap())(
                ctx,
                core::ptr::null_mut(),
                (core::mem::size_of::<Renode>() * n as usize) as c_int,
            ) as *mut Renode;
            (*g).pend = (*g).pstart;
            if (*g).pstart.is_null() {
                die(g, c"cannot allocate regular expression parse list".as_ptr());
            }
        }

        (*g).source = pattern;
        (*g).ncclass = 0;
        (*g).nsub = 1;
        i = 0;
        while i < REG_MAXSUB as c_int {
            (*g).sub[i as usize] = core::ptr::null_mut();
            i += 1;
        }

        (*(*g).prog).flags = cflags;

        next(g);
        node = parsealt(g);
        if (*g).lookahead == ')' as c_int {
            die(g, c"unmatched ')'".as_ptr());
        }
        if (*g).lookahead != EOF {
            die(g, c"syntax error".as_ptr());
        }

        n = 6 + count(g, node, 0);
        if n < 0 || n > REG_MAXPROG {
            die(g, c"program too large".as_ptr());
        }

        (*(*g).prog).nsub = (*g).nsub;
        (*(*g).prog).start =
            (alloc.unwrap())(ctx, core::ptr::null_mut(), n * core::mem::size_of::<Reinst>() as c_int)
                as *mut Reinst;
        (*(*g).prog).end = (*(*g).prog).start;
        if (*(*g).prog).start.is_null() {
            die(g, c"cannot allocate regular expression instruction list".as_ptr());
        }

        if (*g).ncclass > 0 {
            (*(*g).prog).cclass = (alloc.unwrap())(
                ctx,
                core::ptr::null_mut(),
                (*g).ncclass * core::mem::size_of::<Reclass>() as c_int,
            ) as *mut Reclass;
            if (*(*g).prog).cclass.is_null() {
                die(g, c"cannot allocate regular expression character class list".as_ptr());
            }
            memcpy(
                (*(*g).prog).cclass as *mut c_void,
                (&raw const (*g).cclass[0]) as *const c_void,
                (*g).ncclass as usize * core::mem::size_of::<Reclass>(),
            );
            i = 0;
            while i < (*g).ncclass {
                (*(*(*g).prog).cclass.offset(i as isize)).end =
                    (&raw mut (*(*(*g).prog).cclass.offset(i as isize)).spans[0]).offset(
                        (*g).cclass[i as usize].end.offset_from(&raw const (*g).cclass[i as usize].spans[0]),
                    );
                i += 1;
            }
        }

        split = emit((*g).prog, I_SPLIT);
        (*split).x = split.offset(3);
        (*split).y = split.offset(1);
        emit((*g).prog, I_ANYNL);
        jump = emit((*g).prog, I_JUMP);
        (*jump).x = split;
        emit((*g).prog, I_LPAR);
        compile((*g).prog, node);
        emit((*g).prog, I_RPAR);
        emit((*g).prog, I_END);

        (alloc.unwrap())(ctx, (*g).pstart as *mut c_void, 0);

        if !errorp.is_null() {
            *errorp = core::ptr::null();
        }
        (*g).prog
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfreex(alloc: ReAlloc, ctx: *mut c_void, prog: *mut Reprog) {
    unsafe {
        if !prog.is_null() {
            if !(*prog).cclass.is_null() {
                (alloc.unwrap())(ctx, (*prog).cclass as *mut c_void, 0);
            }
            (alloc.unwrap())(ctx, (*prog).start as *mut c_void, 0);
            (alloc.unwrap())(ctx, prog as *mut c_void, 0);
        }
    }
}

unsafe extern "C-unwind" fn default_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    unsafe {
        if n == 0 {
            free(p);
            return core::ptr::null_mut();
        }
        realloc(p, n as size_t)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcomp(
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    unsafe { js_regcompx(Some(default_alloc), core::ptr::null_mut(), pattern, cflags, errorp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfree(prog: *mut Reprog) {
    unsafe {
        js_regfreex(Some(default_alloc), core::ptr::null_mut(), prog);
    }
}

/* Match */

unsafe fn isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

unsafe fn iswordchar(c: c_int) -> c_int {
    (c == '_' as c_int
        || (c >= 'a' as c_int && c <= 'z' as c_int)
        || (c >= 'A' as c_int && c <= 'Z' as c_int)
        || (c >= '0' as c_int && c <= '9' as c_int)) as c_int
}

unsafe fn incclass(cc: *mut Reclass, c: Rune) -> c_int {
    unsafe {
        let mut p: *mut Rune = &raw mut (*cc).spans[0];
        while p < (*cc).end {
            if *p.offset(0) <= c && c <= *p.offset(1) {
                return 1;
            }
            p = p.offset(2);
        }
        0
    }
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    unsafe {
        let mut p: *mut Rune = &raw mut (*cc).spans[0];
        let mut r: Rune;
        while p < (*cc).end {
            r = *p.offset(0);
            while r <= *p.offset(1) {
                if c == canon(r) {
                    return 1;
                }
                r += 1;
            }
            p = p.offset(2);
        }
        0
    }
}

unsafe fn strncmpcanon(a_in: *const c_char, b_in: *const c_char, n_in: c_int) -> c_int {
    unsafe {
        let mut a = a_in;
        let mut b = b_in;
        let mut n = n_in;
        let mut ra: Rune = 0;
        let mut rb: Rune = 0;
        while n != 0 {
            n -= 1;
            if *a == 0 {
                return -1;
            }
            if *b == 0 {
                return 1;
            }
            a = a.offset(jsU_chartorune(&raw mut ra, a) as isize);
            b = b.offset(jsU_chartorune(&raw mut rb, b) as isize);
            let c = canon(ra) - canon(rb);
            if c != 0 {
                return c;
            }
        }
        0
    }
}

unsafe fn match_(
    pc_in: *mut Reinst,
    sp_in: *const c_char,
    bol: *const c_char,
    flags: c_int,
    out: *mut Resub,
    depth: c_int,
) -> c_int {
    unsafe {
        let mut pc = pc_in;
        let mut sp = sp_in;
        let mut scratch: Resub = Resub::new();
        let mut result: c_int;
        let mut i: c_int;
        let mut c: Rune = 0;

        /* stack overflow */
        if depth > REG_MAXREC {
            return -1;
        }

        loop {
            match (*pc).opcode as c_int {
                I_END => {
                    return 0;
                }
                I_JUMP => {
                    pc = (*pc).x;
                }
                I_SPLIT => {
                    scratch = *out;
                    result = match_((*pc).x, sp, bol, flags, &raw mut scratch, depth + 1);
                    if result == -1 {
                        return -1;
                    }
                    if result == 0 {
                        *out = scratch;
                        return 0;
                    }
                    pc = (*pc).y;
                }

                I_PLA => {
                    result = match_((*pc).x, sp, bol, flags, out, depth + 1);
                    if result == -1 {
                        return -1;
                    }
                    if result == 1 {
                        return 1;
                    }
                    pc = (*pc).y;
                }
                I_NLA => {
                    scratch = *out;
                    result = match_((*pc).x, sp, bol, flags, &raw mut scratch, depth + 1);
                    if result == -1 {
                        return -1;
                    }
                    if result == 0 {
                        return 1;
                    }
                    pc = (*pc).y;
                }

                I_ANYNL => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                    pc = pc.offset(1);
                }
                I_ANY => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                    if isnewline(c) != 0 {
                        return 1;
                    }
                    pc = pc.offset(1);
                }
                I_CHAR => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        c = canon(c);
                    }
                    if c != (*pc).c {
                        return 1;
                    }
                    pc = pc.offset(1);
                }
                I_CCLASS => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        if incclasscanon((*pc).cc, canon(c)) == 0 {
                            return 1;
                        }
                    } else {
                        if incclass((*pc).cc, c) == 0 {
                            return 1;
                        }
                    }
                    pc = pc.offset(1);
                }
                I_NCCLASS => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&raw mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        if incclasscanon((*pc).cc, canon(c)) != 0 {
                            return 1;
                        }
                    } else {
                        if incclass((*pc).cc, c) != 0 {
                            return 1;
                        }
                    }
                    pc = pc.offset(1);
                }
                I_REF => {
                    i = ((*out).sub[(*pc).n as usize].ep as isize
                        - (*out).sub[(*pc).n as usize].sp as isize) as c_int;
                    if flags & REG_ICASE != 0 {
                        if strncmpcanon(sp, (*out).sub[(*pc).n as usize].sp, i) != 0 {
                            return 1;
                        }
                    } else {
                        if strncmp(sp, (*out).sub[(*pc).n as usize].sp, i as size_t) != 0 {
                            return 1;
                        }
                    }
                    if i > 0 {
                        sp = sp.offset(i as isize);
                    }
                    pc = pc.offset(1);
                }

                I_BOL => {
                    if sp == bol && (flags & REG_NOTBOL) == 0 {
                        pc = pc.offset(1);
                    } else if flags & REG_NEWLINE != 0
                        && sp > bol
                        && isnewline(*sp.offset(-1) as c_int) != 0
                    {
                        pc = pc.offset(1);
                    } else {
                        return 1;
                    }
                }
                I_EOL => {
                    if *sp == 0 {
                        pc = pc.offset(1);
                    } else if flags & REG_NEWLINE != 0 && isnewline(*sp as c_int) != 0 {
                        pc = pc.offset(1);
                    } else {
                        return 1;
                    }
                }
                I_WORD => {
                    i = (sp > bol && iswordchar(*sp.offset(-1) as c_int) != 0) as c_int;
                    i ^= iswordchar(*sp.offset(0) as c_int);
                    if i == 0 {
                        return 1;
                    }
                    pc = pc.offset(1);
                }
                I_NWORD => {
                    i = (sp > bol && iswordchar(*sp.offset(-1) as c_int) != 0) as c_int;
                    i ^= iswordchar(*sp.offset(0) as c_int);
                    if i != 0 {
                        return 1;
                    }
                    pc = pc.offset(1);
                }

                I_LPAR => {
                    (*out).sub[(*pc).n as usize].sp = sp;
                    pc = pc.offset(1);
                }
                I_RPAR => {
                    (*out).sub[(*pc).n as usize].ep = sp;
                    pc = pc.offset(1);
                }
                _ => {
                    return 1;
                }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regexec(
    prog: *mut Reprog,
    sp: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    unsafe {
        let mut scratch: Resub = Resub::new();
        let mut i: c_int;
        let mut sub = sub;

        if sub.is_null() {
            sub = &raw mut scratch;
        }

        (*sub).nsub = (*prog).nsub;
        i = 0;
        while i < REG_MAXSUB as c_int {
            (*sub).sub[i as usize].sp = core::ptr::null();
            (*sub).sub[i as usize].ep = core::ptr::null();
            i += 1;
        }

        match_((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
    }
}
