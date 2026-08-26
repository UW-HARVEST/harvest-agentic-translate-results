//! Translated from c_src/src/regexp.c
use crate::jsi::*;
use crate::prelude::*;

use std::mem::{size_of, MaybeUninit};
use std::ptr::{addr_of, addr_of_mut};

/* #define emit regemit / next regnext / accept regaccept */

const EOF: c_int = -1;

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

/* character constants, spelled out so they can be used as match patterns */
const CHR_0: c_int = '0' as c_int;
const CHR_B: c_int = 'B' as c_int;
const CHR_D: c_int = 'D' as c_int;
const CHR_S: c_int = 'S' as c_int;
const CHR_W: c_int = 'W' as c_int;
const CHR_b: c_int = 'b' as c_int;
const CHR_c: c_int = 'c' as c_int;
const CHR_d: c_int = 'd' as c_int;
const CHR_f: c_int = 'f' as c_int;
const CHR_n: c_int = 'n' as c_int;
const CHR_r: c_int = 'r' as c_int;
const CHR_s: c_int = 's' as c_int;
const CHR_t: c_int = 't' as c_int;
const CHR_u: c_int = 'u' as c_int;
const CHR_v: c_int = 'v' as c_int;
const CHR_w: c_int = 'w' as c_int;
const CHR_x: c_int = 'x' as c_int;
const CHR_DOLLAR: c_int = '$' as c_int;
const CHR_RPAREN: c_int = ')' as c_int;
const CHR_STAR: c_int = '*' as c_int;
const CHR_PLUS: c_int = '+' as c_int;
const CHR_DOT: c_int = '.' as c_int;
const CHR_QUEST: c_int = '?' as c_int;
const CHR_CARET: c_int = '^' as c_int;
const CHR_BAR: c_int = '|' as c_int;

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
    kaboom: jmp_buf,

    cclass: [Reclass; REG_MAXCLASS],
}

unsafe fn die(g: *mut cstate, message: *const c_char) -> ! {
    vwrite(addr_of_mut!((*g).error), message);
    longjmp(addr_of_mut!((*g).kaboom), 1)
}

unsafe fn canon(c: Rune) -> c_int {
    let u: Rune = jsU_toupperrune(c);
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

/* `*g->source++` */
macro_rules! srcnext {
    ($g:expr) => {{
        let c__: c_char = *(*$g).source;
        (*$g).source = (*$g).source.add(1);
        c__
    }};
}

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
    die(g, c"invalid escape sequence".as_ptr())
    /* return 0; */
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    die(g, c"invalid quantifier".as_ptr())
    /* return 0; */
}

const ESCAPES: &std::ffi::CStr = c"BbDdSsWw^$\\.*+?()[]{}|-0123456789";

unsafe fn isunicodeletter(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'z' as c_int)
        || (c >= 'A' as c_int && c <= 'Z' as c_int)
        || jsU_isalpharune(c) != 0) as c_int
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    if *(*g).source == 0 {
        (*g).yychar = EOF;
        return 0;
    }
    (*g).source = (*g)
        .source
        .offset(jsU_chartorune(addr_of_mut!((*g).yychar), (*g).source) as isize);
    if (*g).yychar == '\\' as c_int {
        if *(*g).source == 0 {
            die(g, c"unterminated escape sequence".as_ptr());
        }
        (*g).source = (*g)
            .source
            .offset(jsU_chartorune(addr_of_mut!((*g).yychar), (*g).source) as isize);
        match (*g).yychar {
            CHR_f => {
                (*g).yychar = 0x0C; /* '\f' */
                return 0;
            }
            CHR_n => {
                (*g).yychar = 0x0A; /* '\n' */
                return 0;
            }
            CHR_r => {
                (*g).yychar = 0x0D; /* '\r' */
                return 0;
            }
            CHR_t => {
                (*g).yychar = 0x09; /* '\t' */
                return 0;
            }
            CHR_v => {
                (*g).yychar = 0x0B; /* '\v' */
                return 0;
            }
            CHR_c => {
                if *(*g).source.add(0) == 0 {
                    die(g, c"unterminated escape sequence".as_ptr());
                }
                (*g).yychar = (srcnext!(g) as c_int) & 31;
                return 0;
            }
            CHR_x => {
                if *(*g).source.add(0) == 0 || *(*g).source.add(1) == 0 {
                    die(g, c"unterminated escape sequence".as_ptr());
                }
                (*g).yychar = hex(g, srcnext!(g) as c_int) << 4;
                (*g).yychar += hex(g, srcnext!(g) as c_int);
                if (*g).yychar == 0 {
                    (*g).yychar = '0' as c_int;
                    return 1;
                }
                return 1;
            }
            CHR_u => {
                if *(*g).source.add(0) == 0
                    || *(*g).source.add(1) == 0
                    || *(*g).source.add(2) == 0
                    || *(*g).source.add(3) == 0
                {
                    die(g, c"unterminated escape sequence".as_ptr());
                }
                (*g).yychar = hex(g, srcnext!(g) as c_int) << 12;
                (*g).yychar += hex(g, srcnext!(g) as c_int) << 8;
                (*g).yychar += hex(g, srcnext!(g) as c_int) << 4;
                (*g).yychar += hex(g, srcnext!(g) as c_int);
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

unsafe fn lexcount(g: *mut cstate) -> c_int {
    (*g).yychar = srcnext!(g) as c_int;

    (*g).yymin = dec(g, (*g).yychar);
    (*g).yychar = srcnext!(g) as c_int;
    while (*g).yychar != ',' as c_int && (*g).yychar != '}' as c_int {
        (*g).yymin = (*g).yymin * 10 + dec(g, (*g).yychar);
        (*g).yychar = srcnext!(g) as c_int;
        if (*g).yymin >= REPINF {
            die(g, c"numeric overflow".as_ptr());
        }
    }

    if (*g).yychar == ',' as c_int {
        (*g).yychar = srcnext!(g) as c_int;
        if (*g).yychar == '}' as c_int {
            (*g).yymax = REPINF;
        } else {
            (*g).yymax = dec(g, (*g).yychar);
            (*g).yychar = srcnext!(g) as c_int;
            while (*g).yychar != '}' as c_int {
                (*g).yymax = (*g).yymax * 10 + dec(g, (*g).yychar);
                (*g).yychar = srcnext!(g) as c_int;
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

unsafe fn newcclass(g: *mut cstate) {
    if (*g).ncclass as usize >= REG_MAXCLASS {
        die(g, c"too many character classes".as_ptr());
    }
    (*g).yycc = (addr_of_mut!((*g).cclass) as *mut Reclass).offset((*g).ncclass as isize);
    (*g).ncclass += 1;
    (*(*g).yycc).end = addr_of_mut!((*(*g).yycc).spans) as *mut Rune;
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    let cc: *mut Reclass = (*g).yycc;
    let mut p: *mut Rune;

    if a > b {
        die(g, c"invalid character class range".as_ptr());
    }

    /* extend existing ranges if they overlap */
    p = addr_of_mut!((*cc).spans) as *mut Rune;
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

    if (*cc).end.add(2) >= (addr_of_mut!((*cc).spans) as *mut Rune).add(REG_MAXSPAN) {
        die(g, c"too many character class ranges".as_ptr());
    }
    *(*cc).end = a;
    (*cc).end = (*cc).end.add(1);
    *(*cc).end = b;
    (*cc).end = (*cc).end.add(1);
}

unsafe fn addranges_d(g: *mut cstate) {
    addrange(g, '0' as c_int, '9' as c_int);
}

unsafe fn addranges_D(g: *mut cstate) {
    addrange(g, 0, '0' as c_int - 1);
    addrange(g, '9' as c_int + 1, 0xFFFF);
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
    addrange(g, '0' as c_int, '9' as c_int);
    addrange(g, 'A' as c_int, 'Z' as c_int);
    addrange(g, '_' as c_int, '_' as c_int);
    addrange(g, 'a' as c_int, 'z' as c_int);
}

unsafe fn addranges_W(g: *mut cstate) {
    addrange(g, 0, '0' as c_int - 1);
    addrange(g, '9' as c_int + 1, 'A' as c_int - 1);
    addrange(g, 'Z' as c_int + 1, '_' as c_int - 1);
    addrange(g, '_' as c_int + 1, 'a' as c_int - 1);
    addrange(g, 'z' as c_int + 1, 0xFFFF);
}

unsafe fn lexclass(g: *mut cstate) -> c_int {
    let mut r#type: c_int = L_CCLASS;
    let mut quoted: c_int;
    let mut havesave: c_int;
    let mut havedash: c_int;
    let mut save: Rune = 0;

    newcclass(g);

    quoted = nextrune(g);
    if quoted == 0 && (*g).yychar == '^' as c_int {
        r#type = L_NCCLASS;
        quoted = nextrune(g);
    }

    havedash = 0;
    havesave = havedash;
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
                    havedash = 0;
                    havesave = havedash;
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
                CHR_d => {
                    addranges_d(g);
                }
                CHR_s => {
                    addranges_s(g);
                }
                CHR_w => {
                    addranges_w(g);
                }
                CHR_D => {
                    addranges_D(g);
                }
                CHR_S => {
                    addranges_S(g);
                }
                CHR_W => {
                    addranges_W(g);
                }
                _ => {}
            }
            havedash = 0;
            havesave = havedash;
        } else {
            if quoted != 0 {
                if (*g).yychar == 'b' as c_int {
                    (*g).yychar = 0x08; /* '\b' */
                } else if (*g).yychar == '0' as c_int {
                    (*g).yychar = 0;
                }
                /* else identity escape */
            }
            if havesave != 0 {
                if havedash != 0 {
                    addrange(g, save, (*g).yychar);
                    havedash = 0;
                    havesave = havedash;
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

    r#type
}

unsafe fn lex(g: *mut cstate) -> c_int {
    let quoted: c_int = nextrune(g);
    if quoted != 0 {
        match (*g).yychar {
            CHR_b => return L_WORD,
            CHR_B => return L_NWORD,
            CHR_d => {
                newcclass(g);
                addranges_d(g);
                return L_CCLASS;
            }
            CHR_s => {
                newcclass(g);
                addranges_s(g);
                return L_CCLASS;
            }
            CHR_w => {
                newcclass(g);
                addranges_w(g);
                return L_CCLASS;
            }
            CHR_D => {
                newcclass(g);
                addranges_d(g);
                return L_NCCLASS;
            }
            CHR_S => {
                newcclass(g);
                addranges_s(g);
                return L_NCCLASS;
            }
            CHR_W => {
                newcclass(g);
                addranges_w(g);
                return L_NCCLASS;
            }
            CHR_0 => {
                (*g).yychar = 0;
                return L_CHAR;
            }
            _ => {}
        }
        if (*g).yychar >= '0' as c_int && (*g).yychar <= '9' as c_int {
            (*g).yychar -= '0' as c_int;
            if *(*g).source >= '0' as c_char && *(*g).source <= '9' as c_char {
                (*g).yychar = (*g).yychar * 10 + srcnext!(g) as c_int - '0' as c_int;
            }
            return L_REF;
        }
        return L_CHAR;
    }

    match (*g).yychar {
        EOF | CHR_DOLLAR | CHR_RPAREN | CHR_STAR | CHR_PLUS | CHR_DOT | CHR_QUEST | CHR_CARET
        | CHR_BAR => {
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

const P_CAT: c_int = 0;
const P_ALT: c_int = 1;
const P_REP: c_int = 2;
const P_BOL: c_int = 3;
const P_EOL: c_int = 4;
const P_WORD: c_int = 5;
const P_NWORD: c_int = 6;
const P_PAR: c_int = 7;
const P_PLA: c_int = 8;
const P_NLA: c_int = 9;
const P_ANY: c_int = 10;
const P_CHAR: c_int = 11;
const P_CCLASS: c_int = 12;
const P_NCCLASS: c_int = 13;
const P_REF: c_int = 14;

#[repr(C)]
struct Renode {
    r#type: c_uchar,
    ng: c_uchar,
    m: c_uchar,
    n: c_uchar,
    c: Rune,
    cc: c_int,
    x: *mut Renode,
    y: *mut Renode,
}

unsafe fn newnode(g: *mut cstate, r#type: c_int) -> *mut Renode {
    let node: *mut Renode = (*g).pend;
    (*g).pend = (*g).pend.add(1);
    (*node).r#type = r#type as c_uchar;
    (*node).cc = -1;
    (*node).c = 0;
    (*node).ng = 0;
    (*node).m = 0;
    (*node).n = 0;
    (*node).y = null_mut();
    (*node).x = (*node).y;
    node
}

unsafe fn empty(node: *mut Renode) -> c_int {
    if node.is_null() {
        return 1;
    }
    match (*node).r#type as c_int {
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
    let rep: *mut Renode = newnode(g, P_REP);
    if max == REPINF && empty(atom) != 0 {
        die(g, c"infinite loop matching the empty string".as_ptr());
    }
    (*rep).ng = ng as c_uchar;
    (*rep).m = min as c_uchar;
    (*rep).n = max as c_uchar;
    (*rep).x = atom;
    rep
}

unsafe fn regnext(g: *mut cstate) {
    (*g).lookahead = lex(g);
}

unsafe fn regaccept(g: *mut cstate, t: c_int) -> c_int {
    if (*g).lookahead == t {
        regnext(g);
        return 1;
    }
    0
}

unsafe fn parseatom(g: *mut cstate) -> *mut Renode {
    let atom: *mut Renode;
    if (*g).lookahead == L_CHAR {
        atom = newnode(g, P_CHAR);
        (*atom).c = (*g).yychar;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_CCLASS {
        atom = newnode(g, P_CCLASS);
        (*atom).cc = (*g)
            .yycc
            .offset_from(addr_of_mut!((*g).cclass) as *mut Reclass) as c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_NCCLASS {
        atom = newnode(g, P_NCCLASS);
        (*atom).cc = (*g)
            .yycc
            .offset_from(addr_of_mut!((*g).cclass) as *mut Reclass) as c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_REF {
        atom = newnode(g, P_REF);
        if (*g).yychar == 0
            || (*g).yychar >= (*g).nsub
            || (*g).sub[(*g).yychar as usize].is_null()
        {
            die(g, c"invalid back-reference".as_ptr());
        }
        (*atom).n = (*g).yychar as c_uchar;
        (*atom).x = (*g).sub[(*g).yychar as usize];
        regnext(g);
        return atom;
    }
    if regaccept(g, '.' as c_int) != 0 {
        return newnode(g, P_ANY);
    }
    if regaccept(g, '(' as c_int) != 0 {
        atom = newnode(g, P_PAR);
        if (*g).nsub as usize == REG_MAXSUB {
            die(g, c"too many captures".as_ptr());
        }
        (*atom).n = (*g).nsub as c_uchar;
        (*g).nsub += 1;
        (*atom).x = parsealt(g);
        (*g).sub[(*atom).n as usize] = atom;
        if regaccept(g, ')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_NC) != 0 {
        atom = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_PLA) != 0 {
        atom = newnode(g, P_PLA);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_NLA) != 0 {
        atom = newnode(g, P_NLA);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    die(g, c"syntax error".as_ptr())
    /* return NULL; */
}

unsafe fn parserep(g: *mut cstate) -> *mut Renode {
    let atom: *mut Renode;

    if regaccept(g, '^' as c_int) != 0 {
        return newnode(g, P_BOL);
    }
    if regaccept(g, '$' as c_int) != 0 {
        return newnode(g, P_EOL);
    }
    if regaccept(g, L_WORD) != 0 {
        return newnode(g, P_WORD);
    }
    if regaccept(g, L_NWORD) != 0 {
        return newnode(g, P_NWORD);
    }

    atom = parseatom(g);
    if (*g).lookahead == L_COUNT {
        let min: c_int = (*g).yymin;
        let max: c_int = (*g).yymax;
        regnext(g);
        if max < min {
            die(g, c"invalid quantifier".as_ptr());
        }
        return newrep(g, atom, regaccept(g, '?' as c_int), min, max);
    }
    if regaccept(g, '*' as c_int) != 0 {
        return newrep(g, atom, regaccept(g, '?' as c_int), 0, REPINF);
    }
    if regaccept(g, '+' as c_int) != 0 {
        return newrep(g, atom, regaccept(g, '?' as c_int), 1, REPINF);
    }
    if regaccept(g, '?' as c_int) != 0 {
        return newrep(g, atom, regaccept(g, '?' as c_int), 0, 1);
    }
    atom
}

unsafe fn parsecat(g: *mut cstate) -> *mut Renode {
    let mut cat: *mut Renode;
    let mut head: *mut Renode;
    let mut tail: *mut *mut Renode;
    if (*g).lookahead != EOF && (*g).lookahead != '|' as c_int && (*g).lookahead != ')' as c_int {
        /* Build a right-leaning tree by splicing in new 'cat' at the tail. */
        head = parserep(g);
        tail = addr_of_mut!(head);
        while (*g).lookahead != EOF
            && (*g).lookahead != '|' as c_int
            && (*g).lookahead != ')' as c_int
        {
            cat = newnode(g, P_CAT);
            (*cat).x = *tail;
            (*cat).y = parserep(g);
            *tail = cat;
            tail = addr_of_mut!((*cat).y);
        }
        return head;
    }
    null_mut()
}

unsafe fn parsealt(g: *mut cstate) -> *mut Renode {
    let mut alt: *mut Renode;
    let mut x: *mut Renode;
    alt = parsecat(g);
    while regaccept(g, '|' as c_int) != 0 {
        x = alt;
        alt = newnode(g, P_ALT);
        (*alt).x = x;
        (*alt).y = parsecat(g);
    }
    alt
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

#[repr(C)]
pub struct Reinst {
    pub opcode: c_uchar,
    pub n: c_uchar,
    pub c: Rune,
    pub cc: *mut Reclass,
    pub x: *mut Reinst,
    pub y: *mut Reinst,
}

unsafe fn count(g: *mut cstate, node: *mut Renode, depth: c_int) -> c_int {
    let mut depth = depth;
    let min: c_int;
    let max: c_int;
    let n: c_int;
    if node.is_null() {
        return 0;
    }
    depth += 1;
    if depth > REG_MAXREC {
        die(g, c"stack overflow".as_ptr());
    }
    match (*node).r#type as c_int {
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

unsafe fn regemit(prog: *mut Reprog, opcode: c_int) -> *mut Reinst {
    let inst: *mut Reinst = (*prog).end;
    (*prog).end = (*prog).end.add(1);
    (*inst).opcode = opcode as c_uchar;
    (*inst).n = 0;
    (*inst).c = 0;
    (*inst).cc = null_mut();
    (*inst).y = null_mut();
    (*inst).x = (*inst).y;
    inst
}

unsafe fn compile(prog: *mut Reprog, node: *mut Renode) {
    let mut node = node;
    let mut inst: *mut Reinst;
    let mut split: *mut Reinst;
    let mut jump: *mut Reinst;
    let mut i: c_int;

    'loop_: loop {
        /* loop: */
        if node.is_null() {
            return;
        }

        match (*node).r#type as c_int {
            P_CAT => {
                compile(prog, (*node).x);
                node = (*node).y;
                continue 'loop_;
            }

            P_ALT => {
                split = regemit(prog, I_SPLIT);
                compile(prog, (*node).x);
                jump = regemit(prog, I_JUMP);
                compile(prog, (*node).y);
                (*split).x = split.add(1);
                (*split).y = jump.add(1);
                (*jump).x = (*prog).end;
            }

            P_REP => 'rep: {
                inst = null_mut(); /* silence compiler warning. assert(node->m > 0). */
                i = 0;
                while i < (*node).m as c_int {
                    inst = (*prog).end;
                    compile(prog, (*node).x);
                    i += 1;
                }
                if (*node).m == (*node).n {
                    break 'rep;
                }
                if ((*node).n as c_int) < REPINF {
                    i = (*node).m as c_int;
                    while i < (*node).n as c_int {
                        split = regemit(prog, I_SPLIT);
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
                    split = regemit(prog, I_SPLIT);
                    compile(prog, (*node).x);
                    jump = regemit(prog, I_JUMP);
                    if (*node).ng != 0 {
                        (*split).y = split.add(1);
                        (*split).x = (*prog).end;
                    } else {
                        (*split).x = split.add(1);
                        (*split).y = (*prog).end;
                    }
                    (*jump).x = split;
                } else {
                    split = regemit(prog, I_SPLIT);
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
                regemit(prog, I_BOL);
            }
            P_EOL => {
                regemit(prog, I_EOL);
            }
            P_WORD => {
                regemit(prog, I_WORD);
            }
            P_NWORD => {
                regemit(prog, I_NWORD);
            }

            P_PAR => {
                inst = regemit(prog, I_LPAR);
                (*inst).n = (*node).n;
                compile(prog, (*node).x);
                inst = regemit(prog, I_RPAR);
                (*inst).n = (*node).n;
            }
            P_PLA => {
                split = regemit(prog, I_PLA);
                compile(prog, (*node).x);
                regemit(prog, I_END);
                (*split).x = split.add(1);
                (*split).y = (*prog).end;
            }
            P_NLA => {
                split = regemit(prog, I_NLA);
                compile(prog, (*node).x);
                regemit(prog, I_END);
                (*split).x = split.add(1);
                (*split).y = (*prog).end;
            }

            P_ANY => {
                regemit(prog, I_ANY);
            }
            P_CHAR => {
                inst = regemit(prog, I_CHAR);
                (*inst).c = if ((*prog).flags & REG_ICASE) != 0 {
                    canon((*node).c)
                } else {
                    (*node).c
                };
            }
            P_CCLASS => {
                inst = regemit(prog, I_CCLASS);
                (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
            }
            P_NCCLASS => {
                inst = regemit(prog, I_NCCLASS);
                (*inst).cc = (*prog).cclass.offset((*node).cc as isize);
            }
            P_REF => {
                inst = regemit(prog, I_REF);
                (*inst).n = (*node).n;
            }
            _ => {}
        }

        break 'loop_;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regcompx(
    alloc: js_Alloc,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    let mut g_storage: MaybeUninit<cstate> = MaybeUninit::uninit();
    let g: *mut cstate = g_storage.as_mut_ptr();
    let node: *mut Renode;
    let split: *mut Reinst;
    let jump: *mut Reinst;
    let mut i: c_int;
    let mut n: c_int;

    vwrite(addr_of_mut!((*g).pstart), null_mut());
    vwrite(addr_of_mut!((*g).prog), null_mut());

    if _setjmp(addr_of_mut!((*g).kaboom)) != 0 {
        if !errorp.is_null() {
            *errorp = vread(addr_of!((*g).error));
        }
        (alloc.unwrap())(ctx, vread(addr_of!((*g).pstart)) as *mut c_void, 0);
        let prog: *mut Reprog = vread(addr_of!((*g).prog));
        if !prog.is_null() {
            (alloc.unwrap())(ctx, (*prog).cclass as *mut c_void, 0);
            (alloc.unwrap())(ctx, (*prog).start as *mut c_void, 0);
            (alloc.unwrap())(ctx, prog as *mut c_void, 0);
        }
        return null_mut();
    }

    vwrite(
        addr_of_mut!((*g).prog),
        (alloc.unwrap())(ctx, null_mut(), size_of::<Reprog>() as c_int) as *mut Reprog,
    );
    if (*g).prog.is_null() {
        die(g, c"cannot allocate regular expression".as_ptr());
    }
    (*(*g).prog).start = null_mut();
    (*(*g).prog).cclass = null_mut();

    n = (strlen(pattern) * 2) as c_int;
    if n > REG_MAXPROG {
        die(g, c"program too large".as_ptr());
    }
    if n > 0 {
        let pl: *mut Renode = (alloc.unwrap())(
            ctx,
            null_mut(),
            (size_of::<Renode>() * n as usize) as c_int,
        ) as *mut Renode;
        (*g).pend = pl;
        vwrite(addr_of_mut!((*g).pstart), pl);
        if (*g).pstart.is_null() {
            die(g, c"cannot allocate regular expression parse list".as_ptr());
        }
    }

    (*g).source = pattern;
    (*g).ncclass = 0;
    (*g).nsub = 1;
    i = 0;
    while (i as usize) < REG_MAXSUB {
        (*g).sub[i as usize] = null_mut();
        i += 1;
    }

    (*(*g).prog).flags = cflags;

    regnext(g);
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
    let il: *mut Reinst = (alloc.unwrap())(
        ctx,
        null_mut(),
        (n as usize * size_of::<Reinst>()) as c_int,
    ) as *mut Reinst;
    (*(*g).prog).end = il;
    (*(*g).prog).start = il;
    if (*(*g).prog).start.is_null() {
        die(
            g,
            c"cannot allocate regular expression instruction list".as_ptr(),
        );
    }

    if (*g).ncclass > 0 {
        (*(*g).prog).cclass = (alloc.unwrap())(
            ctx,
            null_mut(),
            ((*g).ncclass as usize * size_of::<Reclass>()) as c_int,
        ) as *mut Reclass;
        if (*(*g).prog).cclass.is_null() {
            die(
                g,
                c"cannot allocate regular expression character class list".as_ptr(),
            );
        }
        memcpy(
            (*(*g).prog).cclass as *mut c_void,
            addr_of!((*g).cclass) as *const c_void,
            (*g).ncclass as usize * size_of::<Reclass>(),
        );
        i = 0;
        while i < (*g).ncclass {
            let dst: *mut Reclass = (*(*g).prog).cclass.offset(i as isize);
            let src: *mut Reclass =
                (addr_of_mut!((*g).cclass) as *mut Reclass).offset(i as isize);
            (*dst).end = (addr_of_mut!((*dst).spans) as *mut Rune).offset(
                ((*src).end as *const Rune)
                    .offset_from(addr_of!((*src).spans) as *const Rune),
            );
            i += 1;
        }
    }

    split = regemit((*g).prog, I_SPLIT);
    (*split).x = split.add(3);
    (*split).y = split.add(1);
    regemit((*g).prog, I_ANYNL);
    jump = regemit((*g).prog, I_JUMP);
    (*jump).x = split;
    regemit((*g).prog, I_LPAR);
    compile((*g).prog, node);
    regemit((*g).prog, I_RPAR);
    regemit((*g).prog, I_END);

    (alloc.unwrap())(ctx, vread(addr_of!((*g).pstart)) as *mut c_void, 0);

    if !errorp.is_null() {
        *errorp = null();
    }
    vread(addr_of!((*g).prog))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regfreex(alloc: js_Alloc, ctx: *mut c_void, prog: *mut Reprog) {
    if !prog.is_null() {
        if !(*prog).cclass.is_null() {
            (alloc.unwrap())(ctx, (*prog).cclass as *mut c_void, 0);
        }
        (alloc.unwrap())(ctx, (*prog).start as *mut c_void, 0);
        (alloc.unwrap())(ctx, prog as *mut c_void, 0);
    }
}

unsafe extern "C" fn default_alloc(ctx: *mut c_void, p: *mut c_void, n: c_int) -> *mut c_void {
    if n == 0 {
        free(p);
        return null_mut();
    }
    realloc(p, n as usize)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regcomp(
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    js_regcompx(Some(default_alloc), null_mut(), pattern, cflags, errorp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regfree(prog: *mut Reprog) {
    js_regfreex(Some(default_alloc), null_mut(), prog);
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
    let mut p: *mut Rune;
    p = addr_of_mut!((*cc).spans) as *mut Rune;
    while p < (*cc).end {
        if *p.add(0) <= c && c <= *p.add(1) {
            return 1;
        }
        p = p.add(2);
    }
    0
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    let mut p: *mut Rune;
    let mut r: Rune;
    p = addr_of_mut!((*cc).spans) as *mut Rune;
    while p < (*cc).end {
        r = *p.add(0);
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

unsafe fn strncmpcanon(a: *const c_char, b: *const c_char, n: c_int) -> c_int {
    let mut a = a;
    let mut b = b;
    let mut n = n;
    let mut ra: Rune = 0;
    let mut rb: Rune = 0;
    let mut c: c_int;
    loop {
        let t: c_int = n;
        n -= 1;
        if t == 0 {
            break;
        }
        if *a == 0 {
            return -1;
        }
        if *b == 0 {
            return 1;
        }
        a = a.offset(jsU_chartorune(addr_of_mut!(ra), a) as isize);
        b = b.offset(jsU_chartorune(addr_of_mut!(rb), b) as isize);
        c = canon(ra) - canon(rb);
        if c != 0 {
            return c;
        }
    }
    0
}

unsafe fn r#match(
    pc: *mut Reinst,
    sp: *const c_char,
    bol: *const c_char,
    flags: c_int,
    out: *mut Resub,
    depth: c_int,
) -> c_int {
    let mut pc = pc;
    let mut sp = sp;
    let mut scratch_storage: MaybeUninit<Resub> = MaybeUninit::uninit();
    let scratch: *mut Resub = scratch_storage.as_mut_ptr();
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
                std::ptr::copy_nonoverlapping(out as *const Resub, scratch, 1);
                result = r#match((*pc).x, sp, bol, flags, scratch, depth + 1);
                if result == -1 {
                    return -1;
                }
                if result == 0 {
                    std::ptr::copy_nonoverlapping(scratch as *const Resub, out, 1);
                    return 0;
                }
                pc = (*pc).y;
            }

            I_PLA => {
                result = r#match((*pc).x, sp, bol, flags, out, depth + 1);
                if result == -1 {
                    return -1;
                }
                if result == 1 {
                    return 1;
                }
                pc = (*pc).y;
            }
            I_NLA => {
                std::ptr::copy_nonoverlapping(out as *const Resub, scratch, 1);
                result = r#match((*pc).x, sp, bol, flags, scratch, depth + 1);
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
                sp = sp.offset(jsU_chartorune(addr_of_mut!(c), sp) as isize);
                pc = pc.add(1);
            }
            I_ANY => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(jsU_chartorune(addr_of_mut!(c), sp) as isize);
                if isnewline(c) != 0 {
                    return 1;
                }
                pc = pc.add(1);
            }
            I_CHAR => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(jsU_chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                sp = sp.offset(jsU_chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                sp = sp.offset(jsU_chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                let k: usize = (*pc).n as usize;
                i = ((*out).sub[k].ep as isize - (*out).sub[k].sp as isize) as c_int;
                if (flags & REG_ICASE) != 0 {
                    if strncmpcanon(sp, (*out).sub[k].sp, i) != 0 {
                        return 1;
                    }
                } else {
                    if strncmp(sp, (*out).sub[k].sp, i as usize) != 0 {
                        return 1;
                    }
                }
                if i > 0 {
                    sp = sp.offset(i as isize);
                }
                pc = pc.add(1);
            }

            I_BOL => 'sw: {
                if sp == bol && (flags & REG_NOTBOL) == 0 {
                    pc = pc.add(1);
                    break 'sw;
                }
                if (flags & REG_NEWLINE) != 0 {
                    if sp > bol && isnewline(*sp.offset(-1) as c_int) != 0 {
                        pc = pc.add(1);
                        break 'sw;
                    }
                }
                return 1;
            }
            I_EOL => 'sw: {
                if *sp == 0 {
                    pc = pc.add(1);
                    break 'sw;
                }
                if (flags & REG_NEWLINE) != 0 {
                    if isnewline(*sp as c_int) != 0 {
                        pc = pc.add(1);
                        break 'sw;
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
            _ => {
                return 1;
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regexec(
    prog: *mut Reprog,
    sp: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    let mut sub = sub;
    let mut scratch_storage: MaybeUninit<Resub> = MaybeUninit::uninit();
    let mut i: c_int;

    if sub.is_null() {
        sub = scratch_storage.as_mut_ptr();
    }

    (*sub).nsub = (*prog).nsub;
    i = 0;
    while (i as usize) < REG_MAXSUB {
        (*sub).sub[i as usize].ep = null();
        (*sub).sub[i as usize].sp = (*sub).sub[i as usize].ep;
        i += 1;
    }

    r#match((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
}
