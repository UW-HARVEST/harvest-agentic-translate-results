//! Translation of regexp.c (+ the types of regexp.h)

use crate::*;

extern "C" {
    fn puts(s: *const c_char) -> c_int;
}

/* #define emit regemit / #define next regnext / #define accept regaccept */

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

const EOF: c_int = -1;

#[repr(C)]
#[derive(Clone, Copy)]
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
    pub kaboom: [u64; 25], /* jmp_buf: 200 bytes on x86-64 glibc */

    pub cclass: [Reclass; REG_MAXCLASS],
}

unsafe fn die(g: *mut cstate, message: *const c_char) -> ! {
    (*g).error = message;
    longjmp(addr_of_mut!((*g).kaboom) as *mut c_void, 1)
}

unsafe fn canon(c: Rune) -> c_int {
    let u: Rune = toupperrune(c);
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
    die(g, cs!("invalid escape sequence"))
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    if c >= '0' as c_int && c <= '9' as c_int {
        return c - '0' as c_int;
    }
    die(g, cs!("invalid quantifier"))
}

macro_rules! ESCAPES {
    () => {
        cs!("BbDdSsWw^$\\.*+?()[]{}|-0123456789")
    };
}

unsafe fn isunicodeletter(c: c_int) -> c_int {
    ((c >= 'a' as c_int && c <= 'z' as c_int)
        || (c >= 'A' as c_int && c <= 'Z' as c_int)
        || isalpharune(c) != 0) as c_int
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    if *(*g).source == 0 {
        (*g).yychar = EOF;
        return 0;
    }
    (*g).source = (*g)
        .source
        .offset(chartorune(addr_of_mut!((*g).yychar), (*g).source) as isize);
    if (*g).yychar == '\\' as c_int {
        if *(*g).source == 0 {
            die(g, cs!("unterminated escape sequence"));
        }
        (*g).source = (*g)
            .source
            .offset(chartorune(addr_of_mut!((*g).yychar), (*g).source) as isize);
        if (*g).yychar == 'f' as c_int {
            (*g).yychar = 0xC; /* '\f' */
            return 0;
        }
        if (*g).yychar == 'n' as c_int {
            (*g).yychar = 0xA; /* '\n' */
            return 0;
        }
        if (*g).yychar == 'r' as c_int {
            (*g).yychar = 0xD; /* '\r' */
            return 0;
        }
        if (*g).yychar == 't' as c_int {
            (*g).yychar = 0x9; /* '\t' */
            return 0;
        }
        if (*g).yychar == 'v' as c_int {
            (*g).yychar = 0xB; /* '\v' */
            return 0;
        }
        if (*g).yychar == 'c' as c_int {
            if *(*g).source.offset(0) == 0 {
                die(g, cs!("unterminated escape sequence"));
            }
            let c = *(*g).source as c_int;
            (*g).source = (*g).source.offset(1);
            (*g).yychar = c & 31;
            return 0;
        }
        if (*g).yychar == 'x' as c_int {
            if *(*g).source.offset(0) == 0 || *(*g).source.offset(1) == 0 {
                die(g, cs!("unterminated escape sequence"));
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
        if (*g).yychar == 'u' as c_int {
            if *(*g).source.offset(0) == 0
                || *(*g).source.offset(1) == 0
                || *(*g).source.offset(2) == 0
                || *(*g).source.offset(3) == 0
            {
                die(g, cs!("unterminated escape sequence"));
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
        if (*g).yychar == 0 {
            (*g).yychar = '0' as c_int;
            return 1;
        }
        if !strchr(ESCAPES!(), (*g).yychar).is_null() {
            return 1;
        }
        if isunicodeletter((*g).yychar) != 0 || (*g).yychar == '_' as c_int {
            /* check identity escape */
            die(g, cs!("invalid escape character"));
        }
        return 0;
    }
    0
}

unsafe fn lexcount(g: *mut cstate) -> c_int {
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
            die(g, cs!("numeric overflow"));
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
                    die(g, cs!("numeric overflow"));
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
        die(g, cs!("too many character classes"));
    }
    (*g).yycc = (addr_of_mut!((*g).cclass) as *mut Reclass).offset((*g).ncclass as isize);
    (*g).ncclass += 1;
    (*(*g).yycc).end = addr_of_mut!((*(*g).yycc).spans) as *mut Rune;
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    let cc: *mut Reclass = (*g).yycc;
    let mut p: *mut Rune;

    if a > b {
        die(g, cs!("invalid character class range"));
    }

    /* extend existing ranges if they overlap */
    p = addr_of_mut!((*cc).spans) as *mut Rune;
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

    if (*cc).end.offset(2)
        >= (addr_of_mut!((*cc).spans) as *mut Rune).offset(REG_MAXSPAN as isize)
    {
        die(g, cs!("too many character class ranges"));
    }
    *(*cc).end = a;
    (*cc).end = (*cc).end.offset(1);
    *(*cc).end = b;
    (*cc).end = (*cc).end.offset(1);
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
    let mut type_: c_int = L_CCLASS;
    let mut quoted: c_int;
    let mut havesave: c_int;
    let mut havedash: c_int;
    let mut save: Rune = 0;

    newcclass(g);

    quoted = nextrune(g);
    if quoted == 0 && (*g).yychar == '^' as c_int {
        type_ = L_NCCLASS;
        quoted = nextrune(g);
    }

    havedash = 0;
    havesave = havedash;
    loop {
        if (*g).yychar == EOF {
            die(g, cs!("unterminated character class"));
        }
        if quoted == 0 && (*g).yychar == ']' as c_int {
            break;
        }

        if quoted == 0 && (*g).yychar == '-' as c_int {
            if havesave != 0 {
                if havedash != 0 {
                    addrange(g, save, '-' as Rune);
                    havedash = 0;
                    havesave = havedash;
                } else {
                    havedash = 1;
                }
            } else {
                save = '-' as Rune;
                havesave = 1;
            }
        } else if quoted != 0 && !strchr(cs!("DSWdsw"), (*g).yychar).is_null() {
            if havesave != 0 {
                addrange(g, save, save);
                if havedash != 0 {
                    addrange(g, '-' as Rune, '-' as Rune);
                }
            }
            if (*g).yychar == 'd' as c_int {
                addranges_d(g);
            } else if (*g).yychar == 's' as c_int {
                addranges_s(g);
            } else if (*g).yychar == 'w' as c_int {
                addranges_w(g);
            } else if (*g).yychar == 'D' as c_int {
                addranges_D(g);
            } else if (*g).yychar == 'S' as c_int {
                addranges_S(g);
            } else if (*g).yychar == 'W' as c_int {
                addranges_W(g);
            }
            havedash = 0;
            havesave = havedash;
        } else {
            if quoted != 0 {
                if (*g).yychar == 'b' as c_int {
                    (*g).yychar = 0x8; /* '\b' */
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
            addrange(g, '-' as Rune, '-' as Rune);
        }
    }

    type_
}

unsafe fn lex(g: *mut cstate) -> c_int {
    let quoted: c_int = nextrune(g);
    if quoted != 0 {
        if (*g).yychar == 'b' as c_int {
            return L_WORD;
        }
        if (*g).yychar == 'B' as c_int {
            return L_NWORD;
        }
        if (*g).yychar == 'd' as c_int {
            newcclass(g);
            addranges_d(g);
            return L_CCLASS;
        }
        if (*g).yychar == 's' as c_int {
            newcclass(g);
            addranges_s(g);
            return L_CCLASS;
        }
        if (*g).yychar == 'w' as c_int {
            newcclass(g);
            addranges_w(g);
            return L_CCLASS;
        }
        if (*g).yychar == 'D' as c_int {
            newcclass(g);
            addranges_d(g);
            return L_NCCLASS;
        }
        if (*g).yychar == 'S' as c_int {
            newcclass(g);
            addranges_s(g);
            return L_NCCLASS;
        }
        if (*g).yychar == 'W' as c_int {
            newcclass(g);
            addranges_w(g);
            return L_NCCLASS;
        }
        if (*g).yychar == '0' as c_int {
            (*g).yychar = 0;
            return L_CHAR;
        }
        if (*g).yychar >= '0' as c_int && (*g).yychar <= '9' as c_int {
            (*g).yychar -= '0' as c_int;
            if *(*g).source as c_int >= '0' as c_int && *(*g).source as c_int <= '9' as c_int {
                let c = *(*g).source as c_int;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = (*g).yychar * 10 + c - '0' as c_int;
            }
            return L_REF;
        }
        return L_CHAR;
    }

    if (*g).yychar == EOF
        || (*g).yychar == '$' as c_int
        || (*g).yychar == ')' as c_int
        || (*g).yychar == '*' as c_int
        || (*g).yychar == '+' as c_int
        || (*g).yychar == '.' as c_int
        || (*g).yychar == '?' as c_int
        || (*g).yychar == '^' as c_int
        || (*g).yychar == '|' as c_int
    {
        return (*g).yychar;
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
pub struct Renode {
    pub type_: u8,
    pub ng: u8,
    pub m: u8,
    pub n: u8,
    pub c: Rune,
    pub cc: c_int,
    pub x: *mut Renode,
    pub y: *mut Renode,
}

unsafe fn newnode(g: *mut cstate, type_: c_int) -> *mut Renode {
    let node: *mut Renode = (*g).pend;
    (*g).pend = (*g).pend.offset(1);
    (*node).type_ = type_ as u8;
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
    match (*node).type_ as c_int {
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
        die(g, cs!("infinite loop matching the empty string"));
    }
    (*rep).ng = ng as u8;
    (*rep).m = min as u8;
    (*rep).n = max as u8;
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
        (*atom).cc = (((*g).yycc as isize - (addr_of_mut!((*g).cclass) as *mut Reclass) as isize)
            / core::mem::size_of::<Reclass>() as isize) as c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_NCCLASS {
        atom = newnode(g, P_NCCLASS);
        (*atom).cc = (((*g).yycc as isize - (addr_of_mut!((*g).cclass) as *mut Reclass) as isize)
            / core::mem::size_of::<Reclass>() as isize) as c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_REF {
        atom = newnode(g, P_REF);
        if (*g).yychar == 0
            || (*g).yychar >= (*g).nsub
            || (*g).sub[(*g).yychar as usize].is_null()
        {
            die(g, cs!("invalid back-reference"));
        }
        (*atom).n = (*g).yychar as u8;
        (*atom).x = (*g).sub[(*g).yychar as usize];
        regnext(g);
        return atom;
    }
    if regaccept(g, '.' as c_int) != 0 {
        return newnode(g, P_ANY);
    }
    if regaccept(g, '(' as c_int) != 0 {
        atom = newnode(g, P_PAR);
        if (*g).nsub == REG_MAXSUB as c_int {
            die(g, cs!("too many captures"));
        }
        (*atom).n = (*g).nsub as u8;
        (*g).nsub += 1;
        (*atom).x = parsealt(g);
        (*g).sub[(*atom).n as usize] = atom;
        if regaccept(g, ')' as c_int) == 0 {
            die(g, cs!("unmatched '('"));
        }
        return atom;
    }
    if regaccept(g, L_NC) != 0 {
        let atom = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, cs!("unmatched '('"));
        }
        return atom;
    }
    if regaccept(g, L_PLA) != 0 {
        atom = newnode(g, P_PLA);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, cs!("unmatched '('"));
        }
        return atom;
    }
    if regaccept(g, L_NLA) != 0 {
        atom = newnode(g, P_NLA);
        (*atom).x = parsealt(g);
        if regaccept(g, ')' as c_int) == 0 {
            die(g, cs!("unmatched '('"));
        }
        return atom;
    }
    die(g, cs!("syntax error"))
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
            die(g, cs!("invalid quantifier"));
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
    pub opcode: u8,
    pub n: u8,
    pub c: Rune,
    pub cc: *mut Reclass,
    pub x: *mut Reinst,
    pub y: *mut Reinst,
}

unsafe fn count(g: *mut cstate, node: *mut Renode, depth: c_int) -> c_int {
    let min: c_int;
    let max: c_int;
    let n: c_int;
    let mut depth = depth;
    if node.is_null() {
        return 0;
    }
    depth += 1;
    if depth > REG_MAXREC {
        die(g, cs!("stack overflow"));
    }
    match (*node).type_ as c_int {
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
                die(g, cs!("program too large"));
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
    (*prog).end = (*prog).end.offset(1);
    (*inst).opcode = opcode as u8;
    (*inst).n = 0;
    (*inst).c = 0;
    (*inst).cc = null_mut();
    (*inst).y = null_mut();
    (*inst).x = (*inst).y;
    inst
}

unsafe fn compile(prog: *mut Reprog, node: *mut Renode) {
    let mut inst: *mut Reinst;
    let mut split: *mut Reinst;
    let mut jump: *mut Reinst;
    let mut i: c_int;
    let mut node = node;

    loop {
        /* loop: */
        if node.is_null() {
            return;
        }

        match (*node).type_ as c_int {
            P_CAT => {
                compile(prog, (*node).x);
                node = (*node).y;
                continue; /* goto loop */
            }

            P_ALT => {
                split = regemit(prog, I_SPLIT);
                compile(prog, (*node).x);
                jump = regemit(prog, I_JUMP);
                compile(prog, (*node).y);
                (*split).x = split.offset(1);
                (*split).y = jump.offset(1);
                (*jump).x = (*prog).end;
            }

            P_REP => {
                inst = null_mut(); /* silence compiler warning. assert(node->m > 0). */
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
                        split = regemit(prog, I_SPLIT);
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
                    split = regemit(prog, I_SPLIT);
                    compile(prog, (*node).x);
                    jump = regemit(prog, I_JUMP);
                    if (*node).ng != 0 {
                        (*split).y = split.offset(1);
                        (*split).x = (*prog).end;
                    } else {
                        (*split).x = split.offset(1);
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
                (*split).x = split.offset(1);
                (*split).y = (*prog).end;
            }
            P_NLA => {
                split = regemit(prog, I_NLA);
                compile(prog, (*node).x);
                regemit(prog, I_END);
                (*split).x = split.offset(1);
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
        break;
    }
}

/* #ifdef TEST */

unsafe fn dumpnode(g: *mut cstate, node: *mut Renode) {
    let mut p: *mut Rune;
    let cc: *mut Reclass;
    if node.is_null() {
        printf(cs!("Empty"));
        return;
    }
    match (*node).type_ as c_int {
        P_CAT => {
            printf(cs!("Cat("));
            dumpnode(g, (*node).x);
            printf(cs!(", "));
            dumpnode(g, (*node).y);
            printf(cs!(")"));
        }
        P_ALT => {
            printf(cs!("Alt("));
            dumpnode(g, (*node).x);
            printf(cs!(", "));
            dumpnode(g, (*node).y);
            printf(cs!(")"));
        }
        P_REP => {
            printf(
                if (*node).ng != 0 {
                    cs!("NgRep(%d,%d,")
                } else {
                    cs!("Rep(%d,%d,")
                },
                (*node).m as c_int,
                (*node).n as c_int,
            );
            dumpnode(g, (*node).x);
            printf(cs!(")"));
        }
        P_BOL => {
            printf(cs!("Bol"));
        }
        P_EOL => {
            printf(cs!("Eol"));
        }
        P_WORD => {
            printf(cs!("Word"));
        }
        P_NWORD => {
            printf(cs!("NotWord"));
        }
        P_PAR => {
            printf(cs!("Par(%d,"), (*node).n as c_int);
            dumpnode(g, (*node).x);
            printf(cs!(")"));
        }
        P_PLA => {
            printf(cs!("PLA("));
            dumpnode(g, (*node).x);
            printf(cs!(")"));
        }
        P_NLA => {
            printf(cs!("NLA("));
            dumpnode(g, (*node).x);
            printf(cs!(")"));
        }
        P_ANY => {
            printf(cs!("Any"));
        }
        P_CHAR => {
            printf(cs!("Char(%c)"), (*node).c);
        }
        P_CCLASS => {
            printf(cs!("Class("));
            cc = (addr_of_mut!((*g).cclass) as *mut Reclass).offset((*node).cc as isize);
            p = addr_of_mut!((*cc).spans) as *mut Rune;
            while p < (*cc).end {
                printf(cs!("%02X-%02X,"), *p.offset(0), *p.offset(1));
                p = p.offset(2);
            }
            printf(cs!(")"));
        }
        P_NCCLASS => {
            printf(cs!("NotClass("));
            cc = (addr_of_mut!((*g).cclass) as *mut Reclass).offset((*node).cc as isize);
            p = addr_of_mut!((*cc).spans) as *mut Rune;
            while p < (*cc).end {
                printf(cs!("%02X-%02X,"), *p.offset(0), *p.offset(1));
                p = p.offset(2);
            }
            printf(cs!(")"));
        }
        P_REF => {
            printf(cs!("Ref(%d)"), (*node).n as c_int);
        }
        _ => {}
    }
}

unsafe fn dumpcclass(cc: *mut Reclass) {
    let mut p: *mut Rune;
    p = addr_of_mut!((*cc).spans) as *mut Rune;
    while p < (*cc).end {
        if *p.offset(0) > 32 && *p.offset(0) < 127 {
            printf(cs!(" %c"), *p.offset(0));
        } else {
            printf(cs!(" \\x%02x"), *p.offset(0));
        }
        if *p.offset(1) > 32 && *p.offset(1) < 127 {
            printf(cs!("-%c"), *p.offset(1));
        } else {
            printf(cs!("-\\x%02x"), *p.offset(1));
        }
        p = p.offset(2);
    }
    putchar('\n' as c_int);
}

unsafe fn dumpprog(prog: *mut Reprog) {
    let mut inst: *mut Reinst;
    let mut i: c_int;
    i = 0;
    inst = (*prog).start;
    while inst < (*prog).end {
        printf(cs!("% 5d: "), i);
        match (*inst).opcode as c_int {
            I_END => {
                puts(cs!("end"));
            }
            I_JUMP => {
                printf(
                    cs!("jump %d\n"),
                    (((*inst).x as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                );
            }
            I_SPLIT => {
                printf(
                    cs!("split %d %d\n"),
                    (((*inst).x as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                    (((*inst).y as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                );
            }
            I_PLA => {
                printf(
                    cs!("pla %d %d\n"),
                    (((*inst).x as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                    (((*inst).y as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                );
            }
            I_NLA => {
                printf(
                    cs!("nla %d %d\n"),
                    (((*inst).x as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                    (((*inst).y as isize - (*prog).start as isize)
                        / core::mem::size_of::<Reinst>() as isize) as c_int,
                );
            }
            I_ANY => {
                puts(cs!("any"));
            }
            I_ANYNL => {
                puts(cs!("anynl"));
            }
            I_CHAR => {
                printf(
                    if (*inst).c >= 32 && (*inst).c < 127 {
                        cs!("char '%c'\n")
                    } else {
                        cs!("char U+%04X\n")
                    },
                    (*inst).c,
                );
            }
            I_CCLASS => {
                printf(cs!("cclass"));
                dumpcclass((*inst).cc);
            }
            I_NCCLASS => {
                printf(cs!("ncclass"));
                dumpcclass((*inst).cc);
            }
            I_REF => {
                printf(cs!("ref %d\n"), (*inst).n as c_int);
            }
            I_BOL => {
                puts(cs!("bol"));
            }
            I_EOL => {
                puts(cs!("eol"));
            }
            I_WORD => {
                puts(cs!("word"));
            }
            I_NWORD => {
                puts(cs!("nword"));
            }
            I_LPAR => {
                printf(cs!("lpar %d\n"), (*inst).n as c_int);
            }
            I_RPAR => {
                printf(cs!("rpar %d\n"), (*inst).n as c_int);
            }
            _ => {}
        }
        i += 1;
        inst = inst.offset(1);
    }
}

/* #endif */

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regcompx(
    alloc: js_Alloc,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    let mut g: cstate = core::mem::zeroed();
    let gp: *mut cstate = addr_of_mut!(g);
    let node: *mut Renode;
    let split: *mut Reinst;
    let jump: *mut Reinst;
    let mut i: c_int;
    let mut n: c_int;

    setvol!(g.pstart, null_mut::<Renode>());
    setvol!(g.prog, null_mut::<Reprog>());

    if _setjmp(addr_of_mut!(g.kaboom) as *mut c_void) != 0 {
        if !errorp.is_null() {
            *errorp = vol!(g.error);
        }
        (alloc.unwrap())(ctx, vol!(g.pstart) as *mut c_void, 0);
        let p: *mut Reprog = vol!(g.prog);
        if !p.is_null() {
            (alloc.unwrap())(ctx, (*p).cclass as *mut c_void, 0);
            (alloc.unwrap())(ctx, (*p).start as *mut c_void, 0);
            (alloc.unwrap())(ctx, p as *mut c_void, 0);
        }
        return null_mut();
    }

    setvol!(
        g.prog,
        (alloc.unwrap())(ctx, null_mut(), core::mem::size_of::<Reprog>() as c_int) as *mut Reprog
    );
    if vol!(g.prog).is_null() {
        die(gp, cs!("cannot allocate regular expression"));
    }
    let prog: *mut Reprog = vol!(g.prog);
    (*prog).start = null_mut();
    (*prog).cclass = null_mut();

    n = (strlen(pattern) * 2) as c_int;
    if n > REG_MAXPROG {
        die(gp, cs!("program too large"));
    }
    if n > 0 {
        setvol!(
            g.pstart,
            (alloc.unwrap())(
                ctx,
                null_mut(),
                core::mem::size_of::<Renode>().wrapping_mul(n as usize) as c_int
            ) as *mut Renode
        );
        g.pend = vol!(g.pstart);
        if vol!(g.pstart).is_null() {
            die(gp, cs!("cannot allocate regular expression parse list"));
        }
    }

    g.source = pattern;
    g.ncclass = 0;
    g.nsub = 1;
    i = 0;
    while i < REG_MAXSUB as c_int {
        g.sub[i as usize] = null_mut();
        i += 1;
    }

    (*prog).flags = cflags;

    regnext(gp);
    node = parsealt(gp);
    if g.lookahead == ')' as c_int {
        die(gp, cs!("unmatched ')'"));
    }
    if g.lookahead != EOF {
        die(gp, cs!("syntax error"));
    }

    n = 6 + count(gp, node, 0);
    if n < 0 || n > REG_MAXPROG {
        die(gp, cs!("program too large"));
    }

    (*prog).nsub = g.nsub;
    (*prog).end = (alloc.unwrap())(
        ctx,
        null_mut(),
        (n as usize).wrapping_mul(core::mem::size_of::<Reinst>()) as c_int,
    ) as *mut Reinst;
    (*prog).start = (*prog).end;
    if (*prog).start.is_null() {
        die(
            gp,
            cs!("cannot allocate regular expression instruction list"),
        );
    }

    if g.ncclass > 0 {
        (*prog).cclass = (alloc.unwrap())(
            ctx,
            null_mut(),
            (g.ncclass as usize).wrapping_mul(core::mem::size_of::<Reclass>()) as c_int,
        ) as *mut Reclass;
        if (*prog).cclass.is_null() {
            die(
                gp,
                cs!("cannot allocate regular expression character class list"),
            );
        }
        memcpy(
            (*prog).cclass as *mut c_void,
            addr_of!(g.cclass) as *const c_void,
            (g.ncclass as usize).wrapping_mul(core::mem::size_of::<Reclass>()),
        );
        i = 0;
        while i < g.ncclass {
            let dst: *mut Reclass = (*prog).cclass.offset(i as isize);
            let src: *mut Reclass = (addr_of_mut!(g.cclass) as *mut Reclass).offset(i as isize);
            let len: isize = ((*src).end as isize
                - (addr_of_mut!((*src).spans) as *mut Rune) as isize)
                / core::mem::size_of::<Rune>() as isize;
            (*dst).end = (addr_of_mut!((*dst).spans) as *mut Rune).offset(len);
            i += 1;
        }
    }

    split = regemit(prog, I_SPLIT);
    (*split).x = split.offset(3);
    (*split).y = split.offset(1);
    regemit(prog, I_ANYNL);
    jump = regemit(prog, I_JUMP);
    (*jump).x = split;
    regemit(prog, I_LPAR);
    compile(prog, node);
    regemit(prog, I_RPAR);
    regemit(prog, I_END);

    (alloc.unwrap())(ctx, vol!(g.pstart) as *mut c_void, 0);

    if !errorp.is_null() {
        *errorp = null();
    }
    vol!(g.prog)
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
        if *p.offset(0) <= c && c <= *p.offset(1) {
            return 1;
        }
        p = p.offset(2);
    }
    0
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    let mut p: *mut Rune;
    let mut r: Rune;
    p = addr_of_mut!((*cc).spans) as *mut Rune;
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

unsafe fn strncmpcanon(a: *const c_char, b: *const c_char, n: c_int) -> c_int {
    let mut ra: Rune = 0;
    let mut rb: Rune = 0;
    let mut c: c_int;
    let mut a = a;
    let mut b = b;
    let mut n = n;
    while {
        let t = n;
        n = n.wrapping_sub(1);
        t != 0
    } {
        if *a == 0 {
            return -1;
        }
        if *b == 0 {
            return 1;
        }
        a = a.offset(chartorune(addr_of_mut!(ra), a) as isize);
        b = b.offset(chartorune(addr_of_mut!(rb), b) as isize);
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
    let mut scratch: Resub = core::mem::zeroed();
    let mut result: c_int;
    let mut i: c_int;
    let mut c: Rune = 0;
    let mut pc = pc;
    let mut sp = sp;

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
                result = r#match((*pc).x, sp, bol, flags, addr_of_mut!(scratch), depth + 1);
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
                scratch = *out;
                result = r#match((*pc).x, sp, bol, flags, addr_of_mut!(scratch), depth + 1);
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
                sp = sp.offset(chartorune(addr_of_mut!(c), sp) as isize);
                pc = pc.offset(1);
            }
            I_ANY => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(chartorune(addr_of_mut!(c), sp) as isize);
                if isnewline(c) != 0 {
                    return 1;
                }
                pc = pc.offset(1);
            }
            I_CHAR => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                sp = sp.offset(chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                sp = sp.offset(chartorune(addr_of_mut!(c), sp) as isize);
                if (flags & REG_ICASE) != 0 {
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
                if (flags & REG_ICASE) != 0 {
                    if strncmpcanon(sp, (*out).sub[(*pc).n as usize].sp, i) != 0 {
                        return 1;
                    }
                } else {
                    if strncmp(sp, (*out).sub[(*pc).n as usize].sp, i as usize) != 0 {
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
                } else {
                    let mut matched = false;
                    if (flags & REG_NEWLINE) != 0 {
                        if sp > bol && isnewline(*sp.offset(-1) as c_int) != 0 {
                            pc = pc.offset(1);
                            matched = true;
                        }
                    }
                    if !matched {
                        return 1;
                    }
                }
            }
            I_EOL => {
                if *sp == 0 {
                    pc = pc.offset(1);
                } else {
                    let mut matched = false;
                    if (flags & REG_NEWLINE) != 0 {
                        if isnewline(*sp as c_int) != 0 {
                            pc = pc.offset(1);
                            matched = true;
                        }
                    }
                    if !matched {
                        return 1;
                    }
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn js_regexec(
    prog: *mut Reprog,
    string: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    let sp: *const c_char = string;
    let mut scratch: Resub = core::mem::zeroed();
    let mut i: c_int;
    let mut sub = sub;

    if sub.is_null() {
        sub = addr_of_mut!(scratch);
    }

    (*sub).nsub = (*prog).nsub;
    i = 0;
    while i < REG_MAXSUB as c_int {
        (*sub).sub[i as usize].ep = null();
        (*sub).sub[i as usize].sp = (*sub).sub[i as usize].ep;
        i += 1;
    }

    r#match((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
}
