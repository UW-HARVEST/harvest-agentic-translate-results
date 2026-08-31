#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals, dead_code)]
use crate::common::*;
use crate::types::*;
use crate::utf::{jsU_chartorune, jsU_isalpharune, jsU_toupperrune, Rune};
use std::ffi::{c_char, c_int, c_void};

/* C's EOF */
const EOF: c_int = -1;

/* regexec flags (from regexp.h) */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;

/* If you redefine REG_MAXSUB, you must make sure both the calling
 * code and the regexp.c compilation unit use the same value!
 */
const REG_MAXSUB: usize = 16;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub_sub {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Clone, Copy)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [Resub_sub; REG_MAXSUB],
}

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

/* Alloc callback type: void *(*alloc)(void *ctx, void *p, int n) */
type AllocFn = unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
struct Reclass {
    end: *mut Rune,
    spans: [Rune; REG_MAXSPAN],
}

#[repr(C)]
pub struct Reprog {
    start: *mut Reinst,
    end: *mut Reinst,
    cclass: *mut Reclass,
    flags: c_int,
    nsub: c_int,
}

/* Private marker payload used to implement the C setjmp/longjmp error path
 * via Rust panic-based control flow. */
struct RegKaboom;

#[repr(C)]
struct cstate {
    prog: *mut Reprog,
    pstart: *mut Renode,
    pend: *mut Renode,

    source: *const c_char,
    ncclass: c_int,
    nsub: c_int,
    sub: [*mut Renode; REG_MAXSUB],

    lookahead: c_int,
    yychar: Rune,
    yycc: *mut Reclass,
    yymin: c_int,
    yymax: c_int,

    error: *const c_char,

    cclass: [Reclass; REG_MAXCLASS],
}

unsafe fn die(g: *mut cstate, message: &'static std::ffi::CStr) -> ! {
    unsafe {
        (*g).error = message.as_ptr();
    }
    std::panic::panic_any(RegKaboom);
}

unsafe fn canon(c: Rune) -> Rune {
    let u = jsU_toupperrune(c);
    if c >= 128 && u < 128 {
        return c;
    }
    u
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
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    if c >= 'a' as c_int && c <= 'f' as c_int {
        return c - 'a' as c_int + 0xA;
    }
    if c >= 'A' as c_int && c <= 'F' as c_int {
        return c - 'A' as c_int + 0xA;
    }
    unsafe { die(g, c"invalid escape sequence") };
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    unsafe { die(g, c"invalid quantifier") };
}

const ESCAPES: &std::ffi::CStr = c"BbDdSsWw^$\\.*+?()[]{}|-0123456789";

unsafe fn isunicodeletter(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'z' as c_int)
        || (c >= 'A' as c_int && c <= 'Z' as c_int)
        || jsU_isalpharune(c) != 0) as c_int
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    unsafe {
        if *(*g).source == 0 {
            (*g).yychar = EOF;
            return 0;
        }
        (*g).source = (*g).source.offset(jsU_chartorune(&mut (*g).yychar, (*g).source) as isize);
        if (*g).yychar == '\\' as Rune {
            if *(*g).source == 0 {
                die(g, c"unterminated escape sequence");
            }
            (*g).source = (*g).source.offset(jsU_chartorune(&mut (*g).yychar, (*g).source) as isize);
            match (*g).yychar {
                x if x == 'f' as Rune => {
                    (*g).yychar = '\u{c}' as Rune;
                    return 0;
                }
                x if x == 'n' as Rune => {
                    (*g).yychar = '\n' as Rune;
                    return 0;
                }
                x if x == 'r' as Rune => {
                    (*g).yychar = '\r' as Rune;
                    return 0;
                }
                x if x == 't' as Rune => {
                    (*g).yychar = '\t' as Rune;
                    return 0;
                }
                x if x == 'v' as Rune => {
                    (*g).yychar = '\u{b}' as Rune;
                    return 0;
                }
                x if x == 'c' as Rune => {
                    if *(*g).source.add(0) == 0 {
                        die(g, c"unterminated escape sequence");
                    }
                    (*g).yychar = (*(*g).source as Rune) & 31;
                    (*g).source = (*g).source.add(1);
                    return 0;
                }
                x if x == 'x' as Rune => {
                    if *(*g).source.add(0) == 0 || *(*g).source.add(1) == 0 {
                        die(g, c"unterminated escape sequence");
                    }
                    let c0 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar = hex(g, c0) << 4;
                    let c1 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar += hex(g, c1);
                    if (*g).yychar == 0 {
                        (*g).yychar = '0' as Rune;
                        return 1;
                    }
                    return 1;
                }
                x if x == 'u' as Rune => {
                    if *(*g).source.add(0) == 0
                        || *(*g).source.add(1) == 0
                        || *(*g).source.add(2) == 0
                        || *(*g).source.add(3) == 0
                    {
                        die(g, c"unterminated escape sequence");
                    }
                    let c0 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar = hex(g, c0) << 12;
                    let c1 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar += hex(g, c1) << 8;
                    let c2 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar += hex(g, c2) << 4;
                    let c3 = *(*g).source as c_int;
                    (*g).source = (*g).source.add(1);
                    (*g).yychar += hex(g, c3);
                    if (*g).yychar == 0 {
                        (*g).yychar = '0' as Rune;
                        return 1;
                    }
                    return 1;
                }
                0 => {
                    (*g).yychar = '0' as Rune;
                    return 1;
                }
                _ => {}
            }
            if !strchr(ESCAPES.as_ptr(), (*g).yychar).is_null() {
                return 1;
            }
            if isunicodeletter((*g).yychar) != 0 || (*g).yychar == '_' as Rune {
                /* check identity escape */
                die(g, c"invalid escape character");
            }
            return 0;
        }
        0
    }
}

unsafe fn lexcount(g: *mut cstate) -> c_int {
    unsafe {
        (*g).yychar = *(*g).source as Rune;
        (*g).source = (*g).source.add(1);

        (*g).yymin = dec(g, (*g).yychar);
        (*g).yychar = *(*g).source as Rune;
        (*g).source = (*g).source.add(1);
        while (*g).yychar != ',' as Rune && (*g).yychar != '}' as Rune {
            (*g).yymin = (*g).yymin * 10 + dec(g, (*g).yychar);
            (*g).yychar = *(*g).source as Rune;
            (*g).source = (*g).source.add(1);
            if (*g).yymin >= REPINF {
                die(g, c"numeric overflow");
            }
        }

        if (*g).yychar == ',' as Rune {
            (*g).yychar = *(*g).source as Rune;
            (*g).source = (*g).source.add(1);
            if (*g).yychar == '}' as Rune {
                (*g).yymax = REPINF;
            } else {
                (*g).yymax = dec(g, (*g).yychar);
                (*g).yychar = *(*g).source as Rune;
                (*g).source = (*g).source.add(1);
                while (*g).yychar != '}' as Rune {
                    (*g).yymax = (*g).yymax * 10 + dec(g, (*g).yychar);
                    (*g).yychar = *(*g).source as Rune;
                    (*g).source = (*g).source.add(1);
                    if (*g).yymax >= REPINF {
                        die(g, c"numeric overflow");
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
            die(g, c"too many character classes");
        }
        (*g).yycc = (*g).cclass.as_mut_ptr().offset((*g).ncclass as isize);
        (*g).ncclass += 1;
        (*(*g).yycc).end = (*(*g).yycc).spans.as_mut_ptr();
    }
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    unsafe {
        let cc = (*g).yycc;

        if a > b {
            die(g, c"invalid character class range");
        }

        /* extend existing ranges if they overlap */
        let mut p = (*cc).spans.as_mut_ptr();
        while p < (*cc).end {
            /* completely inside old range */
            if a >= *p.add(0) && b <= *p.add(1) {
                return;
            }

            /* completely swallows old range */
            if a < *p.add(0) && b >= *p.add(1) {
                *p.add(0) = a;
                *p.add(1) = b;
                return;
            }

            /* extend at start */
            if b >= *p.add(0) - 1 && b <= *p.add(1) && a < *p.add(0) {
                *p.add(0) = a;
                return;
            }

            /* extend at end */
            if a >= *p.add(0) && a <= *p.add(1) + 1 && b > *p.add(1) {
                *p.add(1) = b;
                return;
            }

            p = p.add(2);
        }

        if (*cc).end.add(2) >= (*cc).spans.as_mut_ptr().add(REG_MAXSPAN) {
            die(g, c"too many character class ranges");
        }
        *(*cc).end = a;
        (*cc).end = (*cc).end.add(1);
        *(*cc).end = b;
        (*cc).end = (*cc).end.add(1);
    }
}

unsafe fn addranges_d(g: *mut cstate) {
    unsafe { addrange(g, '0' as Rune, '9' as Rune) };
}

unsafe fn addranges_D(g: *mut cstate) {
    unsafe {
        addrange(g, 0, '0' as Rune - 1);
        addrange(g, '9' as Rune + 1, 0xFFFF);
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
        addrange(g, '0' as Rune, '9' as Rune);
        addrange(g, 'A' as Rune, 'Z' as Rune);
        addrange(g, '_' as Rune, '_' as Rune);
        addrange(g, 'a' as Rune, 'z' as Rune);
    }
}

unsafe fn addranges_W(g: *mut cstate) {
    unsafe {
        addrange(g, 0, '0' as Rune - 1);
        addrange(g, '9' as Rune + 1, 'A' as Rune - 1);
        addrange(g, 'Z' as Rune + 1, '_' as Rune - 1);
        addrange(g, '_' as Rune + 1, 'a' as Rune - 1);
        addrange(g, 'z' as Rune + 1, 0xFFFF);
    }
}

unsafe fn lexclass(g: *mut cstate) -> c_int {
    unsafe {
        let mut type_ = L_CCLASS;
        let mut quoted: c_int;
        let mut havesave: c_int;
        let mut havedash: c_int;
        let mut save: Rune = 0;

        newcclass(g);

        quoted = nextrune(g);
        if quoted == 0 && (*g).yychar == '^' as Rune {
            type_ = L_NCCLASS;
            quoted = nextrune(g);
        }

        havesave = 0;
        havedash = 0;
        loop {
            if (*g).yychar == EOF {
                die(g, c"unterminated character class");
            }
            if quoted == 0 && (*g).yychar == ']' as Rune {
                break;
            }

            if quoted == 0 && (*g).yychar == '-' as Rune {
                if havesave != 0 {
                    if havedash != 0 {
                        addrange(g, save, '-' as Rune);
                        havesave = 0;
                        havedash = 0;
                    } else {
                        havedash = 1;
                    }
                } else {
                    save = '-' as Rune;
                    havesave = 1;
                }
            } else if quoted != 0 && !strchr(c"DSWdsw".as_ptr(), (*g).yychar).is_null() {
                if havesave != 0 {
                    addrange(g, save, save);
                    if havedash != 0 {
                        addrange(g, '-' as Rune, '-' as Rune);
                    }
                }
                match (*g).yychar {
                    x if x == 'd' as Rune => addranges_d(g),
                    x if x == 's' as Rune => addranges_s(g),
                    x if x == 'w' as Rune => addranges_w(g),
                    x if x == 'D' as Rune => addranges_D(g),
                    x if x == 'S' as Rune => addranges_S(g),
                    x if x == 'W' as Rune => addranges_W(g),
                    _ => {}
                }
                havesave = 0;
                havedash = 0;
            } else {
                if quoted != 0 {
                    if (*g).yychar == 'b' as Rune {
                        (*g).yychar = '\u{8}' as Rune;
                    } else if (*g).yychar == '0' as Rune {
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
                addrange(g, '-' as Rune, '-' as Rune);
            }
        }

        type_
    }
}

unsafe fn lex(g: *mut cstate) -> c_int {
    unsafe {
        let quoted = nextrune(g);
        if quoted != 0 {
            match (*g).yychar {
                x if x == 'b' as Rune => return L_WORD,
                x if x == 'B' as Rune => return L_NWORD,
                x if x == 'd' as Rune => {
                    newcclass(g);
                    addranges_d(g);
                    return L_CCLASS;
                }
                x if x == 's' as Rune => {
                    newcclass(g);
                    addranges_s(g);
                    return L_CCLASS;
                }
                x if x == 'w' as Rune => {
                    newcclass(g);
                    addranges_w(g);
                    return L_CCLASS;
                }
                x if x == 'D' as Rune => {
                    newcclass(g);
                    addranges_d(g);
                    return L_NCCLASS;
                }
                x if x == 'S' as Rune => {
                    newcclass(g);
                    addranges_s(g);
                    return L_NCCLASS;
                }
                x if x == 'W' as Rune => {
                    newcclass(g);
                    addranges_w(g);
                    return L_NCCLASS;
                }
                x if x == '0' as Rune => {
                    (*g).yychar = 0;
                    return L_CHAR;
                }
                _ => {}
            }
            if (*g).yychar >= '0' as Rune && (*g).yychar <= '9' as Rune {
                (*g).yychar -= '0' as Rune;
                if *(*g).source >= '0' as c_char && *(*g).source <= '9' as c_char {
                    (*g).yychar = (*g).yychar * 10 + *(*g).source as Rune - '0' as Rune;
                    (*g).source = (*g).source.add(1);
                }
                return L_REF;
            }
            return L_CHAR;
        }

        match (*g).yychar {
            EOF => return (*g).yychar,
            x if x == '$' as Rune
                || x == ')' as Rune
                || x == '*' as Rune
                || x == '+' as Rune
                || x == '.' as Rune
                || x == '?' as Rune
                || x == '^' as Rune
                || x == '|' as Rune =>
            {
                return (*g).yychar;
            }
            _ => {}
        }

        if (*g).yychar == '{' as Rune {
            return lexcount(g);
        }
        if (*g).yychar == '[' as Rune {
            return lexclass(g);
        }
        if (*g).yychar == '(' as Rune {
            if *(*g).source.add(0) == '?' as c_char {
                if *(*g).source.add(1) == ':' as c_char {
                    (*g).source = (*g).source.add(2);
                    return L_NC;
                }
                if *(*g).source.add(1) == '=' as c_char {
                    (*g).source = (*g).source.add(2);
                    return L_PLA;
                }
                if *(*g).source.add(1) == '!' as c_char {
                    (*g).source = (*g).source.add(2);
                    return L_NLA;
                }
            }
            return '(' as c_int;
        }

        L_CHAR
    }
}

/* Parse */

const P_CAT: u8 = 0;
const P_ALT: u8 = 1;
const P_REP: u8 = 2;
const P_BOL: u8 = 3;
const P_EOL: u8 = 4;
const P_WORD: u8 = 5;
const P_NWORD: u8 = 6;
const P_PAR: u8 = 7;
const P_PLA: u8 = 8;
const P_NLA: u8 = 9;
const P_ANY: u8 = 10;
const P_CHAR: u8 = 11;
const P_CCLASS: u8 = 12;
const P_NCCLASS: u8 = 13;
const P_REF: u8 = 14;

#[repr(C)]
struct Renode {
    type_: u8,
    ng: u8,
    m: u8,
    n: u8,
    c: Rune,
    cc: c_int,
    x: *mut Renode,
    y: *mut Renode,
}

unsafe fn newnode(g: *mut cstate, type_: c_int) -> *mut Renode {
    unsafe {
        let node = (*g).pend;
        (*g).pend = (*g).pend.add(1);
        (*node).type_ = type_ as u8;
        (*node).cc = -1;
        (*node).c = 0;
        (*node).ng = 0;
        (*node).m = 0;
        (*node).n = 0;
        (*node).x = std::ptr::null_mut();
        (*node).y = std::ptr::null_mut();
        node
    }
}

unsafe fn empty(node: *mut Renode) -> c_int {
    unsafe {
        if node.is_null() {
            return 1;
        }
        match (*node).type_ {
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
        let rep = newnode(g, P_REP as c_int);
        if max == REPINF && empty(atom) != 0 {
            die(g, c"infinite loop matching the empty string");
        }
        (*rep).ng = ng as u8;
        (*rep).m = min as u8;
        (*rep).n = max as u8;
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
            (*atom).cc = (*g).yycc.offset_from((*g).cclass.as_ptr()) as c_int;
            next(g);
            return atom;
        }
        if (*g).lookahead == L_NCCLASS {
            atom = newnode(g, P_NCCLASS as c_int);
            (*atom).cc = (*g).yycc.offset_from((*g).cclass.as_ptr()) as c_int;
            next(g);
            return atom;
        }
        if (*g).lookahead == L_REF {
            atom = newnode(g, P_REF as c_int);
            if (*g).yychar == 0
                || (*g).yychar >= (*g).nsub
                || (*g).sub[(*g).yychar as usize].is_null()
            {
                die(g, c"invalid back-reference");
            }
            (*atom).n = (*g).yychar as u8;
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
                die(g, c"too many captures");
            }
            (*atom).n = (*g).nsub as u8;
            (*g).nsub += 1;
            (*atom).x = parsealt(g);
            (*g).sub[(*atom).n as usize] = atom;
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('");
            }
            return atom;
        }
        if accept(g, L_NC) != 0 {
            let atom = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('");
            }
            return atom;
        }
        if accept(g, L_PLA) != 0 {
            atom = newnode(g, P_PLA as c_int);
            (*atom).x = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('");
            }
            return atom;
        }
        if accept(g, L_NLA) != 0 {
            atom = newnode(g, P_NLA as c_int);
            (*atom).x = parsealt(g);
            if accept(g, ')' as c_int) == 0 {
                die(g, c"unmatched '('");
            }
            return atom;
        }
        die(g, c"syntax error");
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
                die(g, c"invalid quantifier");
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
        let cat: *mut Renode;
        let head: *mut Renode;
        let mut tail: *mut *mut Renode;
        if (*g).lookahead != EOF && (*g).lookahead != '|' as c_int && (*g).lookahead != ')' as c_int
        {
            /* Build a right-leaning tree by splicing in new 'cat' at the tail. */
            head = parserep(g);
            let mut headslot = head;
            tail = &mut headslot;
            while (*g).lookahead != EOF
                && (*g).lookahead != '|' as c_int
                && (*g).lookahead != ')' as c_int
            {
                let cat = newnode(g, P_CAT as c_int);
                (*cat).x = *tail;
                (*cat).y = parserep(g);
                *tail = cat;
                tail = &mut (*cat).y;
            }
            let _ = cat;
            return headslot;
        }
        std::ptr::null_mut()
    }
}

unsafe fn parsealt(g: *mut cstate) -> *mut Renode {
    unsafe {
        let mut alt: *mut Renode;
        let x: *mut Renode;
        alt = parsecat(g);
        while accept(g, '|' as c_int) != 0 {
            let x = alt;
            alt = newnode(g, P_ALT as c_int);
            (*alt).x = x;
            (*alt).y = parsecat(g);
        }
        let _ = x;
        alt
    }
}

/* Compile */

const I_END: u8 = 0;
const I_JUMP: u8 = 1;
const I_SPLIT: u8 = 2;
const I_PLA: u8 = 3;
const I_NLA: u8 = 4;
const I_ANYNL: u8 = 5;
const I_ANY: u8 = 6;
const I_CHAR: u8 = 7;
const I_CCLASS: u8 = 8;
const I_NCCLASS: u8 = 9;
const I_REF: u8 = 10;
const I_BOL: u8 = 11;
const I_EOL: u8 = 12;
const I_WORD: u8 = 13;
const I_NWORD: u8 = 14;
const I_LPAR: u8 = 15;
const I_RPAR: u8 = 16;

#[repr(C)]
struct Reinst {
    opcode: u8,
    n: u8,
    c: Rune,
    cc: *mut Reclass,
    x: *mut Reinst,
    y: *mut Reinst,
}

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
            die(g, c"stack overflow");
        }
        match (*node).type_ {
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
                    die(g, c"program too large");
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
        let inst = (*prog).end;
        (*prog).end = (*prog).end.add(1);
        (*inst).opcode = opcode as u8;
        (*inst).n = 0;
        (*inst).c = 0;
        (*inst).cc = std::ptr::null_mut();
        (*inst).x = std::ptr::null_mut();
        (*inst).y = std::ptr::null_mut();
        inst
    }
}

unsafe fn compile(prog: *mut Reprog, node_in: *mut Renode) {
    unsafe {
        let mut inst: *mut Reinst;
        let mut split: *mut Reinst;
        let mut jump: *mut Reinst;
        let mut i: c_int;
        let mut node = node_in;

        loop {
            if node.is_null() {
                return;
            }

            match (*node).type_ {
                P_CAT => {
                    compile(prog, (*node).x);
                    node = (*node).y;
                    continue;
                }

                P_ALT => {
                    split = emit(prog, I_SPLIT as c_int);
                    compile(prog, (*node).x);
                    jump = emit(prog, I_JUMP as c_int);
                    compile(prog, (*node).y);
                    (*split).x = split.add(1);
                    (*split).y = jump.add(1);
                    (*jump).x = (*prog).end;
                }

                P_REP => {
                    inst = std::ptr::null_mut(); /* silence compiler warning. assert(node->m > 0). */
                    i = 0;
                    while i < (*node).m as c_int {
                        inst = (*prog).end;
                        compile(prog, (*node).x);
                        i += 1;
                    }
                    if (*node).m == (*node).n {
                        break;
                    }
                    if ((*node).n as c_int) < REPINF {
                        i = (*node).m as c_int;
                        while i < (*node).n as c_int {
                            split = emit(prog, I_SPLIT as c_int);
                            compile(prog, (*node).x);
                            if (*node).ng != 0 {
                                (*split).y = split.add(1);
                                (*split).x = (*prog).end;
                            } else {
                                (*split).x = split.add(1);
                                (*split).y = (*prog).end;
                            }
                            i += 1;
                        }
                    } else if (*node).m == 0 {
                        split = emit(prog, I_SPLIT as c_int);
                        compile(prog, (*node).x);
                        jump = emit(prog, I_JUMP as c_int);
                        if (*node).ng != 0 {
                            (*split).y = split.add(1);
                            (*split).x = (*prog).end;
                        } else {
                            (*split).x = split.add(1);
                            (*split).y = (*prog).end;
                        }
                        (*jump).x = split;
                    } else {
                        split = emit(prog, I_SPLIT as c_int);
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
                    emit(prog, I_BOL as c_int);
                }
                P_EOL => {
                    emit(prog, I_EOL as c_int);
                }
                P_WORD => {
                    emit(prog, I_WORD as c_int);
                }
                P_NWORD => {
                    emit(prog, I_NWORD as c_int);
                }

                P_PAR => {
                    inst = emit(prog, I_LPAR as c_int);
                    (*inst).n = (*node).n;
                    compile(prog, (*node).x);
                    inst = emit(prog, I_RPAR as c_int);
                    (*inst).n = (*node).n;
                }
                P_PLA => {
                    split = emit(prog, I_PLA as c_int);
                    compile(prog, (*node).x);
                    emit(prog, I_END as c_int);
                    (*split).x = split.add(1);
                    (*split).y = (*prog).end;
                }
                P_NLA => {
                    split = emit(prog, I_NLA as c_int);
                    compile(prog, (*node).x);
                    emit(prog, I_END as c_int);
                    (*split).x = split.add(1);
                    (*split).y = (*prog).end;
                }

                P_ANY => {
                    emit(prog, I_ANY as c_int);
                }
                P_CHAR => {
                    inst = emit(prog, I_CHAR as c_int);
                    (*inst).c = if (*prog).flags & REG_ICASE != 0 {
                        canon((*node).c)
                    } else {
                        (*node).c
                    };
                }
                P_CCLASS => {
                    inst = emit(prog, I_CCLASS as c_int);
                    (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                }
                P_NCCLASS => {
                    inst = emit(prog, I_NCCLASS as c_int);
                    (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
                }
                P_REF => {
                    inst = emit(prog, I_REF as c_int);
                    (*inst).n = (*node).n;
                }
                _ => {}
            }
            break;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcompx(
    alloc: Option<AllocFn>,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    unsafe {
        let alloc = alloc.unwrap();

        /* g is heap-allocated because it is large (contains a REG_MAXCLASS
         * array); the C version puts it on the stack. Its address must stay
         * fixed across the parse. */
        let mut g_box: Box<std::mem::MaybeUninit<cstate>> = Box::new(std::mem::MaybeUninit::uninit());
        let g: *mut cstate = g_box.as_mut_ptr() as *mut cstate;

        (*g).pstart = std::ptr::null_mut();
        (*g).prog = std::ptr::null_mut();

        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let mut node: *mut Renode;
            let split: *mut Reinst;
            let jump: *mut Reinst;
            let mut i: c_int;
            let mut n: c_int;

            (*g).prog = alloc(ctx, std::ptr::null_mut(), std::mem::size_of::<Reprog>() as c_int)
                as *mut Reprog;
            if (*g).prog.is_null() {
                die(g, c"cannot allocate regular expression");
            }
            (*(*g).prog).start = std::ptr::null_mut();
            (*(*g).prog).cclass = std::ptr::null_mut();

            n = (strlen(pattern) * 2) as c_int;
            if n > REG_MAXPROG {
                die(g, c"program too large");
            }
            if n > 0 {
                (*g).pstart = alloc(
                    ctx,
                    std::ptr::null_mut(),
                    (std::mem::size_of::<Renode>() as c_int) * n,
                ) as *mut Renode;
                (*g).pend = (*g).pstart;
                if (*g).pstart.is_null() {
                    die(g, c"cannot allocate regular expression parse list");
                }
            }

            (*g).source = pattern;
            (*g).ncclass = 0;
            (*g).nsub = 1;
            i = 0;
            while i < REG_MAXSUB as c_int {
                (*g).sub[i as usize] = std::ptr::null_mut();
                i += 1;
            }

            (*(*g).prog).flags = cflags;

            next(g);
            node = parsealt(g);
            if (*g).lookahead == ')' as c_int {
                die(g, c"unmatched ')'");
            }
            if (*g).lookahead != EOF {
                die(g, c"syntax error");
            }

            n = 6 + count(g, node, 0);
            if n < 0 || n > REG_MAXPROG {
                die(g, c"program too large");
            }

            (*(*g).prog).nsub = (*g).nsub;
            (*(*g).prog).start =
                alloc(ctx, std::ptr::null_mut(), n * std::mem::size_of::<Reinst>() as c_int)
                    as *mut Reinst;
            (*(*g).prog).end = (*(*g).prog).start;
            if (*(*g).prog).start.is_null() {
                die(g, c"cannot allocate regular expression instruction list");
            }

            if (*g).ncclass > 0 {
                (*(*g).prog).cclass = alloc(
                    ctx,
                    std::ptr::null_mut(),
                    (*g).ncclass * std::mem::size_of::<Reclass>() as c_int,
                ) as *mut Reclass;
                if (*(*g).prog).cclass.is_null() {
                    die(g, c"cannot allocate regular expression character class list");
                }
                memcpy(
                    (*(*g).prog).cclass as *mut c_void,
                    (*g).cclass.as_ptr() as *const c_void,
                    (*g).ncclass as usize * std::mem::size_of::<Reclass>(),
                );
                i = 0;
                while i < (*g).ncclass {
                    let dst = (*(*g).prog).cclass.offset(i as isize);
                    let srcoff =
                        (*g).cclass[i as usize].end.offset_from((*g).cclass[i as usize].spans.as_ptr());
                    (*dst).end = (*dst).spans.as_mut_ptr().offset(srcoff);
                    i += 1;
                }
            }

            split = emit((*g).prog, I_SPLIT as c_int);
            (*split).x = split.add(3);
            (*split).y = split.add(1);
            emit((*g).prog, I_ANYNL as c_int);
            jump = emit((*g).prog, I_JUMP as c_int);
            (*jump).x = split;
            emit((*g).prog, I_LPAR as c_int);
            compile((*g).prog, node);
            emit((*g).prog, I_RPAR as c_int);
            emit((*g).prog, I_END as c_int);

            alloc(ctx, (*g).pstart as *mut c_void, 0);

            if !errorp.is_null() {
                *errorp = std::ptr::null();
            }
        }));

        match result {
            Ok(()) => (*g).prog,
            Err(payload) => {
                if payload.downcast_ref::<RegKaboom>().is_none() {
                    /* Not our marker; re-raise. */
                    std::panic::resume_unwind(payload);
                }
                if !errorp.is_null() {
                    *errorp = (*g).error;
                }
                alloc(ctx, (*g).pstart as *mut c_void, 0);
                if !(*g).prog.is_null() {
                    alloc(ctx, (*(*g).prog).cclass as *mut c_void, 0);
                    alloc(ctx, (*(*g).prog).start as *mut c_void, 0);
                    alloc(ctx, (*g).prog as *mut c_void, 0);
                }
                std::ptr::null_mut()
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfreex(
    alloc: Option<AllocFn>,
    ctx: *mut c_void,
    prog: *mut Reprog,
) {
    unsafe {
        let alloc = alloc.unwrap();
        if !prog.is_null() {
            if !(*prog).cclass.is_null() {
                alloc(ctx, (*prog).cclass as *mut c_void, 0);
            }
            alloc(ctx, (*prog).start as *mut c_void, 0);
            alloc(ctx, prog as *mut c_void, 0);
        }
    }
}

unsafe extern "C-unwind" fn default_alloc(
    _ctx: *mut c_void,
    p: *mut c_void,
    n: c_int,
) -> *mut c_void {
    unsafe {
        if n == 0 {
            free(p);
            return std::ptr::null_mut();
        }
        realloc(p, n as usize)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcomp(
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    unsafe { js_regcompx(Some(default_alloc), std::ptr::null_mut(), pattern, cflags, errorp) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfree(prog: *mut Reprog) {
    unsafe { js_regfreex(Some(default_alloc), std::ptr::null_mut(), prog) }
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
        let mut p = (*cc).spans.as_ptr();
        while p < (*cc).end {
            if *p.add(0) <= c && c <= *p.add(1) {
                return 1;
            }
            p = p.add(2);
        }
        0
    }
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    unsafe {
        let mut p = (*cc).spans.as_ptr();
        while p < (*cc).end {
            let mut r = *p.add(0);
            while r <= *p.add(1) {
                if c == canon(r) {
                    return 1;
                }
                r += 1;
            }
            p = p.add(2);
        }
        0
    }
}

unsafe fn strncmpcanon(a: *const c_char, b: *const c_char, n: c_int) -> c_int {
    unsafe {
        let mut a = a;
        let mut b = b;
        let mut ra: Rune = 0;
        let mut rb: Rune = 0;
        let mut c: c_int;
        let mut n = n;
        while n != 0 {
            n -= 1;
            if *a == 0 {
                return -1;
            }
            if *b == 0 {
                return 1;
            }
            a = a.offset(jsU_chartorune(&mut ra, a) as isize);
            b = b.offset(jsU_chartorune(&mut rb, b) as isize);
            c = canon(ra) - canon(rb);
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
        let mut scratch: Resub;
        let mut result: c_int;
        let mut i: c_int;
        let mut c: Rune = 0;
        let mut pc = pc_in;
        let mut sp = sp_in;

        /* stack overflow */
        if depth > REG_MAXREC {
            return -1;
        }

        loop {
            match (*pc).opcode {
                I_END => {
                    return 0;
                }
                I_JUMP => {
                    pc = (*pc).x;
                }
                I_SPLIT => {
                    scratch = *out;
                    result = match_((*pc).x, sp, bol, flags, &mut scratch, depth + 1);
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
                    result = match_((*pc).x, sp, bol, flags, &mut scratch, depth + 1);
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
                    sp = sp.offset(jsU_chartorune(&mut c, sp) as isize);
                    pc = pc.add(1);
                }
                I_ANY => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&mut c, sp) as isize);
                    if isnewline(c) != 0 {
                        return 1;
                    }
                    pc = pc.add(1);
                }
                I_CHAR => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        c = canon(c);
                    }
                    if c != (*pc).c {
                        return 1;
                    }
                    pc = pc.add(1);
                }
                I_CCLASS => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        if incclasscanon((*pc).cc, canon(c)) == 0 {
                            return 1;
                        }
                    } else if incclass((*pc).cc, c) == 0 {
                        return 1;
                    }
                    pc = pc.add(1);
                }
                I_NCCLASS => {
                    if *sp == 0 {
                        return 1;
                    }
                    sp = sp.offset(jsU_chartorune(&mut c, sp) as isize);
                    if flags & REG_ICASE != 0 {
                        if incclasscanon((*pc).cc, canon(c)) != 0 {
                            return 1;
                        }
                    } else if incclass((*pc).cc, c) != 0 {
                        return 1;
                    }
                    pc = pc.add(1);
                }
                I_REF => {
                    i = (*out).sub[(*pc).n as usize]
                        .ep
                        .offset_from((*out).sub[(*pc).n as usize].sp) as c_int;
                    if flags & REG_ICASE != 0 {
                        if strncmpcanon(sp, (*out).sub[(*pc).n as usize].sp, i) != 0 {
                            return 1;
                        }
                    } else if strncmp(sp, (*out).sub[(*pc).n as usize].sp, i as usize) != 0 {
                        return 1;
                    }
                    if i > 0 {
                        sp = sp.offset(i as isize);
                    }
                    pc = pc.add(1);
                }

                I_BOL => {
                    if sp == bol && (flags & REG_NOTBOL) == 0 {
                        pc = pc.add(1);
                    } else if flags & REG_NEWLINE != 0
                        && sp > bol
                        && isnewline(*sp.offset(-1) as c_int) != 0
                    {
                        pc = pc.add(1);
                    } else {
                        return 1;
                    }
                }
                I_EOL => {
                    if *sp == 0 {
                        pc = pc.add(1);
                    } else if flags & REG_NEWLINE != 0 && isnewline(*sp as c_int) != 0 {
                        pc = pc.add(1);
                    } else {
                        return 1;
                    }
                }
                I_WORD => {
                    i = (sp > bol && iswordchar(*sp.offset(-1) as c_int) != 0) as c_int;
                    i ^= iswordchar(*sp.add(0) as c_int);
                    if i == 0 {
                        return 1;
                    }
                    pc = pc.add(1);
                }
                I_NWORD => {
                    i = (sp > bol && iswordchar(*sp.offset(-1) as c_int) != 0) as c_int;
                    i ^= iswordchar(*sp.add(0) as c_int);
                    if i != 0 {
                        return 1;
                    }
                    pc = pc.add(1);
                }

                I_LPAR => {
                    (*out).sub[(*pc).n as usize].sp = sp;
                    pc = pc.add(1);
                }
                I_RPAR => {
                    (*out).sub[(*pc).n as usize].ep = sp;
                    pc = pc.add(1);
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
        let mut scratch: Resub = std::mem::zeroed();
        let mut i: c_int;

        let sub = if sub.is_null() { &mut scratch as *mut Resub } else { sub };

        (*sub).nsub = (*prog).nsub;
        i = 0;
        while i < REG_MAXSUB as c_int {
            (*sub).sub[i as usize].sp = std::ptr::null();
            (*sub).sub[i as usize].ep = std::ptr::null();
            i += 1;
        }

        match_((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
    }
}
