//! Translated from regexp.c — Russ Cox / MuJS regular expression engine.
#![allow(non_snake_case, non_upper_case_globals, non_camel_case_types)]

use crate::cutil::*;
use crate::types::{Rune, EOF};
use crate::utf::{chartorune, isalpharune, toupperrune};
use std::os::raw::{c_char, c_int, c_void};
use std::panic::{catch_unwind, resume_unwind, AssertUnwindSafe};

macro_rules! cstr {
    ($s:literal) => {
        concat!($s, "\0").as_ptr() as *const c_char
    };
}

pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
pub const REG_NOTBOL: c_int = 4;

pub const REG_MAXSUB: usize = 16;

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

type AllocFn = Option<unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;

#[repr(C)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubSpan; REG_MAXSUB],
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct ResubSpan {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

impl Resub {
    fn new() -> Resub {
        Resub {
            nsub: 0,
            sub: [ResubSpan { sp: std::ptr::null(), ep: std::ptr::null() }; REG_MAXSUB],
        }
    }
}

#[repr(C)]
#[derive(Copy, Clone)]
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

pub struct Kaboom;

unsafe fn die(g: *mut cstate, message: *const c_char) -> ! {
    (*g).error = message;
    std::panic::panic_any(Kaboom);
}

unsafe fn canon(c: Rune) -> Rune {
    let u = toupperrune(c);
    if c >= 128 && u < 128 {
        return c;
    }
    u
}

/* Scan */
const L_CHAR: c_int = 256;
const L_CCLASS: c_int = 257;
const L_NCCLASS: c_int = 258;
const L_NC: c_int = 259;
const L_PLA: c_int = 260;
const L_NLA: c_int = 261;
const L_WORD: c_int = 262;
const L_NWORD: c_int = 263;
const L_REF: c_int = 264;
const L_COUNT: c_int = 265;

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
    die(g, cstr!("invalid escape sequence"));
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    die(g, cstr!("invalid quantifier"));
}

const ESCAPES: *const c_char = b"BbDdSsWw^$\\.*+?()[]{}|-0123456789\0".as_ptr() as *const c_char;

unsafe fn isunicodeletter(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'z' as c_int) || (c >= 'A' as c_int && c <= 'Z' as c_int) || isalpharune(c) != 0) as c_int
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    if *(*g).source == 0 {
        (*g).yychar = EOF;
        return 0;
    }
    (*g).source = (*g).source.add(chartorune(&mut (*g).yychar, (*g).source) as usize);
    if (*g).yychar == '\\' as Rune {
        if *(*g).source == 0 {
            die(g, cstr!("unterminated escape sequence"));
        }
        (*g).source = (*g).source.add(chartorune(&mut (*g).yychar, (*g).source) as usize);
        match (*g).yychar {
            x if x == 'f' as Rune => {
                (*g).yychar = '\u{0c}' as Rune;
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
                (*g).yychar = '\u{0b}' as Rune;
                return 0;
            }
            x if x == 'c' as Rune => {
                if *(*g).source.add(0) == 0 {
                    die(g, cstr!("unterminated escape sequence"));
                }
                (*g).yychar = (*(*g).source as c_int) & 31;
                (*g).source = (*g).source.add(1);
                return 0;
            }
            x if x == 'x' as Rune => {
                if *(*g).source.add(0) == 0 || *(*g).source.add(1) == 0 {
                    die(g, cstr!("unterminated escape sequence"));
                }
                let a = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar = hex(g, a) << 4;
                let b = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar += hex(g, b);
                if (*g).yychar == 0 {
                    (*g).yychar = '0' as Rune;
                    return 1;
                }
                return 1;
            }
            x if x == 'u' as Rune => {
                if *(*g).source.add(0) == 0 || *(*g).source.add(1) == 0 || *(*g).source.add(2) == 0 || *(*g).source.add(3) == 0 {
                    die(g, cstr!("unterminated escape sequence"));
                }
                let a = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar = hex(g, a) << 12;
                let b = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar += hex(g, b) << 8;
                let c = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar += hex(g, c) << 4;
                let d = *(*g).source as c_int;
                (*g).source = (*g).source.add(1);
                (*g).yychar += hex(g, d);
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
        if !strchr(ESCAPES, (*g).yychar).is_null() {
            return 1;
        }
        if isunicodeletter((*g).yychar) != 0 || (*g).yychar == '_' as Rune {
            die(g, cstr!("invalid escape character"));
        }
        return 0;
    }
    0
}

unsafe fn lexcount(g: *mut cstate) -> c_int {
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
            die(g, cstr!("numeric overflow"));
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
                    die(g, cstr!("numeric overflow"));
                }
            }
        }
    } else {
        (*g).yymax = (*g).yymin;
    }

    L_COUNT
}

unsafe fn newcclass(g: *mut cstate) {
    if (*g).ncclass >= REG_MAXCLASS as c_int {
        die(g, cstr!("too many character classes"));
    }
    (*g).yycc = (*g).cclass.as_mut_ptr().add((*g).ncclass as usize);
    (*g).ncclass += 1;
    (*(*g).yycc).end = (*(*g).yycc).spans.as_mut_ptr();
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    let cc = (*g).yycc;
    if a > b {
        die(g, cstr!("invalid character class range"));
    }
    let mut p = (*cc).spans.as_mut_ptr();
    while p < (*cc).end {
        if a >= *p.add(0) && b <= *p.add(1) {
            return;
        }
        if a < *p.add(0) && b >= *p.add(1) {
            *p.add(0) = a;
            *p.add(1) = b;
            return;
        }
        if b >= *p.add(0) - 1 && b <= *p.add(1) && a < *p.add(0) {
            *p.add(0) = a;
            return;
        }
        if a >= *p.add(0) && a <= *p.add(1) + 1 && b > *p.add(1) {
            *p.add(1) = b;
            return;
        }
        p = p.add(2);
    }
    if (*cc).end.add(2) >= (*cc).spans.as_mut_ptr().add(REG_MAXSPAN) {
        die(g, cstr!("too many character class ranges"));
    }
    *(*cc).end = a;
    (*cc).end = (*cc).end.add(1);
    *(*cc).end = b;
    (*cc).end = (*cc).end.add(1);
}

unsafe fn addranges_d(g: *mut cstate) {
    addrange(g, '0' as Rune, '9' as Rune);
}
unsafe fn addranges_D(g: *mut cstate) {
    addrange(g, 0, '0' as Rune - 1);
    addrange(g, '9' as Rune + 1, 0xFFFF);
}
unsafe fn addranges_s(g: *mut cstate) {
    addrange(g, 0x9, 0xD);
    addrange(g, 0x20, 0x20);
    addrange(g, 0xA0, 0xA0);
    addrange(g, 0x2028, 0x2029);
    addrange(g, 0xFEFF, 0xFEFF);
}
unsafe fn addranges_S(g: *mut cstate) {
    addrange(g, 0, 0x9 - 1);
    addrange(g, 0xD + 1, 0x20 - 1);
    addrange(g, 0x20 + 1, 0xA0 - 1);
    addrange(g, 0xA0 + 1, 0x2028 - 1);
    addrange(g, 0x2029 + 1, 0xFEFF - 1);
    addrange(g, 0xFEFF + 1, 0xFFFF);
}
unsafe fn addranges_w(g: *mut cstate) {
    addrange(g, '0' as Rune, '9' as Rune);
    addrange(g, 'A' as Rune, 'Z' as Rune);
    addrange(g, '_' as Rune, '_' as Rune);
    addrange(g, 'a' as Rune, 'z' as Rune);
}
unsafe fn addranges_W(g: *mut cstate) {
    addrange(g, 0, '0' as Rune - 1);
    addrange(g, '9' as Rune + 1, 'A' as Rune - 1);
    addrange(g, 'Z' as Rune + 1, '_' as Rune - 1);
    addrange(g, '_' as Rune + 1, 'a' as Rune - 1);
    addrange(g, 'z' as Rune + 1, 0xFFFF);
}

unsafe fn lexclass(g: *mut cstate) -> c_int {
    let mut type_ = L_CCLASS;
    let mut quoted;
    let mut havesave;
    let mut havedash;
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
            die(g, cstr!("unterminated character class"));
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
        } else if quoted != 0 && !strchr(cstr!("DSWdsw"), (*g).yychar).is_null() {
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
                    (*g).yychar = '\u{08}' as Rune;
                } else if (*g).yychar == '0' as Rune {
                    (*g).yychar = 0;
                }
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

unsafe fn lex(g: *mut cstate) -> c_int {
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
        x if x == EOF
            || x == '$' as Rune
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

unsafe fn empty(node: *mut Renode) -> c_int {
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

unsafe fn newrep(g: *mut cstate, atom: *mut Renode, ng: c_int, min: c_int, max: c_int) -> *mut Renode {
    let rep = newnode(g, P_REP as c_int);
    if max == REPINF && empty(atom) != 0 {
        die(g, cstr!("infinite loop matching the empty string"));
    }
    (*rep).ng = ng as u8;
    (*rep).m = min as u8;
    (*rep).n = max as u8;
    (*rep).x = atom;
    rep
}

unsafe fn next_(g: *mut cstate) {
    (*g).lookahead = lex(g);
}

unsafe fn accept_(g: *mut cstate, t: c_int) -> c_int {
    if (*g).lookahead == t {
        next_(g);
        return 1;
    }
    0
}

unsafe fn parseatom(g: *mut cstate) -> *mut Renode {
    let atom;
    if (*g).lookahead == L_CHAR {
        let a = newnode(g, P_CHAR as c_int);
        (*a).c = (*g).yychar;
        next_(g);
        return a;
    }
    if (*g).lookahead == L_CCLASS {
        let a = newnode(g, P_CCLASS as c_int);
        (*a).cc = ((*g).yycc as isize - (*g).cclass.as_ptr() as isize) as c_int / std::mem::size_of::<Reclass>() as c_int;
        next_(g);
        return a;
    }
    if (*g).lookahead == L_NCCLASS {
        let a = newnode(g, P_NCCLASS as c_int);
        (*a).cc = ((*g).yycc as isize - (*g).cclass.as_ptr() as isize) as c_int / std::mem::size_of::<Reclass>() as c_int;
        next_(g);
        return a;
    }
    if (*g).lookahead == L_REF {
        let a = newnode(g, P_REF as c_int);
        if (*g).yychar == 0 || (*g).yychar >= (*g).nsub || (*g).sub[(*g).yychar as usize].is_null() {
            die(g, cstr!("invalid back-reference"));
        }
        (*a).n = (*g).yychar as u8;
        (*a).x = (*g).sub[(*g).yychar as usize];
        next_(g);
        return a;
    }
    if accept_(g, '.' as c_int) != 0 {
        return newnode(g, P_ANY as c_int);
    }
    if accept_(g, '(' as c_int) != 0 {
        atom = newnode(g, P_PAR as c_int);
        if (*g).nsub == REG_MAXSUB as c_int {
            die(g, cstr!("too many captures"));
        }
        (*atom).n = (*g).nsub as u8;
        (*g).nsub += 1;
        (*atom).x = parsealt(g);
        (*g).sub[(*atom).n as usize] = atom;
        if accept_(g, ')' as c_int) == 0 {
            die(g, cstr!("unmatched '('"));
        }
        return atom;
    }
    if accept_(g, L_NC) != 0 {
        atom = parsealt(g);
        if accept_(g, ')' as c_int) == 0 {
            die(g, cstr!("unmatched '('"));
        }
        return atom;
    }
    if accept_(g, L_PLA) != 0 {
        atom = newnode(g, P_PLA as c_int);
        (*atom).x = parsealt(g);
        if accept_(g, ')' as c_int) == 0 {
            die(g, cstr!("unmatched '('"));
        }
        return atom;
    }
    if accept_(g, L_NLA) != 0 {
        atom = newnode(g, P_NLA as c_int);
        (*atom).x = parsealt(g);
        if accept_(g, ')' as c_int) == 0 {
            die(g, cstr!("unmatched '('"));
        }
        return atom;
    }
    die(g, cstr!("syntax error"));
}

unsafe fn parserep(g: *mut cstate) -> *mut Renode {
    let atom;

    if accept_(g, '^' as c_int) != 0 {
        return newnode(g, P_BOL as c_int);
    }
    if accept_(g, '$' as c_int) != 0 {
        return newnode(g, P_EOL as c_int);
    }
    if accept_(g, L_WORD) != 0 {
        return newnode(g, P_WORD as c_int);
    }
    if accept_(g, L_NWORD) != 0 {
        return newnode(g, P_NWORD as c_int);
    }

    atom = parseatom(g);
    if (*g).lookahead == L_COUNT {
        let min = (*g).yymin;
        let max = (*g).yymax;
        next_(g);
        if max < min {
            die(g, cstr!("invalid quantifier"));
        }
        return newrep(g, atom, accept_(g, '?' as c_int), min, max);
    }
    if accept_(g, '*' as c_int) != 0 {
        return newrep(g, atom, accept_(g, '?' as c_int), 0, REPINF);
    }
    if accept_(g, '+' as c_int) != 0 {
        return newrep(g, atom, accept_(g, '?' as c_int), 1, REPINF);
    }
    if accept_(g, '?' as c_int) != 0 {
        return newrep(g, atom, accept_(g, '?' as c_int), 0, 1);
    }
    atom
}

unsafe fn parsecat(g: *mut cstate) -> *mut Renode {
    let mut cat;
    let mut head;
    let mut tail;
    if (*g).lookahead != EOF && (*g).lookahead != '|' as c_int && (*g).lookahead != ')' as c_int {
        head = parserep(g);
        tail = std::ptr::addr_of_mut!(head);
        while (*g).lookahead != EOF && (*g).lookahead != '|' as c_int && (*g).lookahead != ')' as c_int {
            cat = newnode(g, P_CAT as c_int);
            (*cat).x = *tail;
            (*cat).y = parserep(g);
            *tail = cat;
            tail = std::ptr::addr_of_mut!((*cat).y);
        }
        return head;
    }
    std::ptr::null_mut()
}

unsafe fn parsealt(g: *mut cstate) -> *mut Renode {
    let mut alt;
    let mut x;
    alt = parsecat(g);
    while accept_(g, '|' as c_int) != 0 {
        x = alt;
        alt = newnode(g, P_ALT as c_int);
        (*alt).x = x;
        (*alt).y = parsecat(g);
    }
    alt
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
pub struct Reinst {
    opcode: u8,
    n: u8,
    c: Rune,
    cc: *mut Reclass,
    x: *mut Reinst,
    y: *mut Reinst,
}

unsafe fn count(g: *mut cstate, node: *mut Renode, depth: c_int) -> c_int {
    let min;
    let max;
    let n;
    if node.is_null() {
        return 0;
    }
    let depth = depth + 1;
    if depth > REG_MAXREC {
        die(g, cstr!("stack overflow"));
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
                die(g, cstr!("program too large"));
            }
            n
        }
        P_PAR => count(g, (*node).x, depth) + 2,
        P_PLA => count(g, (*node).x, depth) + 2,
        P_NLA => count(g, (*node).x, depth) + 2,
        _ => 1,
    }
}

unsafe fn emit(prog: *mut Reprog, opcode: c_int) -> *mut Reinst {
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

unsafe fn compile(prog: *mut Reprog, mut node: *mut Renode) {
    let mut inst;
    let mut split;
    let mut jump;
    let mut i;

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
                inst = std::ptr::null_mut();
                i = 0;
                while i < (*node).m as c_int {
                    inst = (*prog).end;
                    compile(prog, (*node).x);
                    i += 1;
                }
                if (*node).m == (*node).n {
                    // break
                } else if (*node).n < REPINF as u8 {
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
                (*inst).c = if (*prog).flags & REG_ICASE != 0 { canon((*node).c) } else { (*node).c };
            }
            P_CCLASS => {
                inst = emit(prog, I_CCLASS as c_int);
                (*inst).cc = (*prog).cclass.add((*node).cc as usize);
            }
            P_NCCLASS => {
                inst = emit(prog, I_NCCLASS as c_int);
                (*inst).cc = (*prog).cclass.add((*node).cc as usize);
            }
            P_REF => {
                inst = emit(prog, I_REF as c_int);
                (*inst).n = (*node).n;
            }
            _ => {}
        }
        return;
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_regcompx(
    alloc: AllocFn,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    let mut g_box: Box<cstate> = Box::new(std::mem::zeroed());
    let g: *mut cstate = &mut *g_box;

    (*g).pstart = std::ptr::null_mut();
    (*g).prog = std::ptr::null_mut();

    let result = catch_unwind(AssertUnwindSafe(|| -> *mut Reprog {
        (*g).prog = (alloc.unwrap())(ctx, std::ptr::null_mut(), std::mem::size_of::<Reprog>() as c_int) as *mut Reprog;
        if (*g).prog.is_null() {
            die(g, cstr!("cannot allocate regular expression"));
        }
        (*(*g).prog).start = std::ptr::null_mut();
        (*(*g).prog).cclass = std::ptr::null_mut();

        let mut n = (strlen(pattern) * 2) as c_int;
        if n > REG_MAXPROG {
            die(g, cstr!("program too large"));
        }
        if n > 0 {
            (*g).pstart = (alloc.unwrap())(ctx, std::ptr::null_mut(), (std::mem::size_of::<Renode>() as c_int) * n) as *mut Renode;
            (*g).pend = (*g).pstart;
            if (*g).pstart.is_null() {
                die(g, cstr!("cannot allocate regular expression parse list"));
            }
        }

        (*g).source = pattern;
        (*g).ncclass = 0;
        (*g).nsub = 1;
        let mut i = 0;
        while i < REG_MAXSUB {
            (*g).sub[i] = std::ptr::null_mut();
            i += 1;
        }

        (*(*g).prog).flags = cflags;

        next_(g);
        let node = parsealt(g);
        if (*g).lookahead == ')' as c_int {
            die(g, cstr!("unmatched ')'"));
        }
        if (*g).lookahead != EOF {
            die(g, cstr!("syntax error"));
        }

        n = 6 + count(g, node, 0);
        if n < 0 || n > REG_MAXPROG {
            die(g, cstr!("program too large"));
        }

        (*(*g).prog).nsub = (*g).nsub;
        (*(*g).prog).start = (alloc.unwrap())(ctx, std::ptr::null_mut(), n * std::mem::size_of::<Reinst>() as c_int) as *mut Reinst;
        (*(*g).prog).end = (*(*g).prog).start;
        if (*(*g).prog).start.is_null() {
            die(g, cstr!("cannot allocate regular expression instruction list"));
        }

        if (*g).ncclass > 0 {
            (*(*g).prog).cclass = (alloc.unwrap())(ctx, std::ptr::null_mut(), (*g).ncclass * std::mem::size_of::<Reclass>() as c_int) as *mut Reclass;
            if (*(*g).prog).cclass.is_null() {
                die(g, cstr!("cannot allocate regular expression character class list"));
            }
            libc::memcpy(
                (*(*g).prog).cclass as *mut c_void,
                (*g).cclass.as_ptr() as *const c_void,
                (*g).ncclass as usize * std::mem::size_of::<Reclass>(),
            );
            let mut i = 0;
            while i < (*g).ncclass {
                (*(*(*g).prog).cclass.add(i as usize)).end = (*(*(*g).prog).cclass.add(i as usize)).spans.as_mut_ptr()
                    .add(((*g).cclass[i as usize].end as isize - (*g).cclass[i as usize].spans.as_ptr() as isize) as usize / std::mem::size_of::<Rune>());
                i += 1;
            }
        }

        let split = emit((*g).prog, I_SPLIT as c_int);
        (*split).x = split.add(3);
        (*split).y = split.add(1);
        emit((*g).prog, I_ANYNL as c_int);
        let jump = emit((*g).prog, I_JUMP as c_int);
        (*jump).x = split;
        emit((*g).prog, I_LPAR as c_int);
        compile((*g).prog, node);
        emit((*g).prog, I_RPAR as c_int);
        emit((*g).prog, I_END as c_int);

        (alloc.unwrap())(ctx, (*g).pstart as *mut c_void, 0);

        if !errorp.is_null() {
            *errorp = std::ptr::null();
        }
        (*g).prog
    }));

    match result {
        Ok(p) => p,
        Err(payload) => {
            if payload.downcast_ref::<Kaboom>().is_some() {
                if !errorp.is_null() {
                    *errorp = (*g).error;
                }
                (alloc.unwrap())(ctx, (*g).pstart as *mut c_void, 0);
                if !(*g).prog.is_null() {
                    (alloc.unwrap())(ctx, (*(*g).prog).cclass as *mut c_void, 0);
                    (alloc.unwrap())(ctx, (*(*g).prog).start as *mut c_void, 0);
                    (alloc.unwrap())(ctx, (*g).prog as *mut c_void, 0);
                }
                return std::ptr::null_mut();
            }
            resume_unwind(payload);
        }
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_regfreex(alloc: AllocFn, ctx: *mut c_void, prog: *mut Reprog) {
    if !prog.is_null() {
        if !(*prog).cclass.is_null() {
            (alloc.unwrap())(ctx, (*prog).cclass as *mut c_void, 0);
        }
        (alloc.unwrap())(ctx, (*prog).start as *mut c_void, 0);
        (alloc.unwrap())(ctx, prog as *mut c_void, 0);
    }
}

unsafe extern "C-unwind" fn default_alloc(_ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        libc::free(p);
        return std::ptr::null_mut();
    }
    libc::realloc(p, n as usize)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_regcomp(pattern: *const c_char, cflags: c_int, errorp: *mut *const c_char) -> *mut Reprog {
    js_regcompx(Some(default_alloc), std::ptr::null_mut(), pattern, cflags, errorp)
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_regfree(prog: *mut Reprog) {
    js_regfreex(Some(default_alloc), std::ptr::null_mut(), prog);
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
    let mut p = (*cc).spans.as_mut_ptr();
    while p < (*cc).end {
        if *p.add(0) <= c && c <= *p.add(1) {
            return 1;
        }
        p = p.add(2);
    }
    0
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    let mut p = (*cc).spans.as_mut_ptr();
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

unsafe fn strncmpcanon(a: *const c_char, b: *const c_char, mut n: c_int) -> c_int {
    let mut ra: Rune = 0;
    let mut rb: Rune = 0;
    let mut c;
    let mut a = a;
    let mut b = b;
    while n != 0 {
        if *a == 0 {
            return -1;
        }
        if *b == 0 {
            return 1;
        }
        a = a.add(chartorune(&mut ra, a) as usize);
        b = b.add(chartorune(&mut rb, b) as usize);
        c = canon(ra) - canon(rb);
        if c != 0 {
            return c;
        }
        n -= 1;
    }
    0
}

unsafe fn regmatch(mut pc: *mut Reinst, mut sp: *const c_char, bol: *const c_char, flags: c_int, out: *mut Resub, depth: c_int) -> c_int {
    let mut scratch: Resub;
    let mut result;
    let mut i;
    let mut c: Rune = 0;

    if depth > REG_MAXREC {
        return -1;
    }

    loop {
        match (*pc).opcode {
            I_END => return 0,
            I_JUMP => {
                pc = (*pc).x;
            }
            I_SPLIT => {
                scratch = std::ptr::read(out);
                result = regmatch((*pc).x, sp, bol, flags, &mut scratch, depth + 1);
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
                result = regmatch((*pc).x, sp, bol, flags, out, depth + 1);
                if result == -1 {
                    return -1;
                }
                if result == 1 {
                    return 1;
                }
                pc = (*pc).y;
            }
            I_NLA => {
                scratch = std::ptr::read(out);
                result = regmatch((*pc).x, sp, bol, flags, &mut scratch, depth + 1);
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
                sp = sp.add(chartorune(&mut c, sp) as usize);
                pc = pc.add(1);
            }
            I_ANY => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.add(chartorune(&mut c, sp) as usize);
                if isnewline(c) != 0 {
                    return 1;
                }
                pc = pc.add(1);
            }
            I_CHAR => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.add(chartorune(&mut c, sp) as usize);
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
                sp = sp.add(chartorune(&mut c, sp) as usize);
                if flags & REG_ICASE != 0 {
                    if incclasscanon((*pc).cc, canon(c)) == 0 {
                        return 1;
                    }
                } else {
                    if incclass((*pc).cc, c) == 0 {
                        return 1;
                    }
                }
                pc = pc.add(1);
            }
            I_NCCLASS => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.add(chartorune(&mut c, sp) as usize);
                if flags & REG_ICASE != 0 {
                    if incclasscanon((*pc).cc, canon(c)) != 0 {
                        return 1;
                    }
                } else {
                    if incclass((*pc).cc, c) != 0 {
                        return 1;
                    }
                }
                pc = pc.add(1);
            }
            I_REF => {
                i = ((*out).sub[(*pc).n as usize].ep as isize - (*out).sub[(*pc).n as usize].sp as isize) as c_int;
                if flags & REG_ICASE != 0 {
                    if strncmpcanon(sp, (*out).sub[(*pc).n as usize].sp, i) != 0 {
                        return 1;
                    }
                } else {
                    if strncmp(sp, (*out).sub[(*pc).n as usize].sp, i as usize) != 0 {
                        return 1;
                    }
                }
                if i > 0 {
                    sp = sp.add(i as usize);
                }
                pc = pc.add(1);
            }
            I_BOL => {
                if sp == bol && (flags & REG_NOTBOL) == 0 {
                    pc = pc.add(1);
                    continue;
                }
                if flags & REG_NEWLINE != 0 {
                    if sp > bol && isnewline(*sp.offset(-1) as c_int) != 0 {
                        pc = pc.add(1);
                        continue;
                    }
                }
                return 1;
            }
            I_EOL => {
                if *sp == 0 {
                    pc = pc.add(1);
                    continue;
                }
                if flags & REG_NEWLINE != 0 {
                    if isnewline(*sp as c_int) != 0 {
                        pc = pc.add(1);
                        continue;
                    }
                }
                return 1;
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
            _ => return 1,
        }
    }
}

#[no_mangle]
pub unsafe extern "C-unwind" fn js_regexec(prog: *mut Reprog, sp: *const c_char, sub: *mut Resub, eflags: c_int) -> c_int {
    let mut scratch = Resub::new();
    let sub = if sub.is_null() { &mut scratch as *mut Resub } else { sub };

    (*sub).nsub = (*prog).nsub;
    let mut i = 0;
    while i < REG_MAXSUB {
        (*sub).sub[i].sp = std::ptr::null();
        (*sub).sub[i].ep = std::ptr::null();
        i += 1;
    }

    regmatch((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
}
