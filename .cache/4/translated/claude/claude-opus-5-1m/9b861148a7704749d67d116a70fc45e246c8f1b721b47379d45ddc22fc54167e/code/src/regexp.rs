//! Translation of `c_src/src/regexp.c`
#![allow(non_snake_case)]

use crate::cstd::*;
use crate::jsi::*;
use crate::utf::{chartorune, isalpharune, toupperrune};
use core::ptr::{null, null_mut};

/* The `#define`s at the top of regexp.c rename three static helpers:
 *     #define emit   regemit
 *     #define next   regnext
 *     #define accept regaccept
 * so the C source-level names `emit`/`next`/`accept` are the functions
 * `regemit`/`regnext`/`regaccept`.  */

/// `EOF` from `<stdio.h>`
const EOF: c_int = -1;

const REPINF: c_int = 255;
const REG_MAXPROG: c_int = 32 << 10;
const REG_MAXREC: c_int = 4096;
const REG_MAXSPAN: usize = 64;
const REG_MAXCLASS: usize = 128;

/* regcomp flags */
pub const REG_ICASE: c_int = 1;
pub const REG_NEWLINE: c_int = 2;
/* regexec flags */
pub const REG_NOTBOL: c_int = 4;

/// If you redefine `REG_MAXSUB`, you must make sure both the calling code and
/// this module use the same value!
pub const REG_MAXSUB: usize = 16;

/// The allocator callback used by `js_regcompx` / `js_regfreex`; identical in
/// shape to `js_Alloc`.
pub type ReAlloc =
    Option<unsafe extern "C-unwind" fn(*mut c_void, *mut c_void, c_int) -> *mut c_void>;

/* ----------------------------------------------------------------- types */

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
#[derive(Copy, Clone)]
pub struct ResubSpan {
    pub sp: *const c_char,
    pub ep: *const c_char,
}

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Resub {
    pub nsub: c_int,
    pub sub: [ResubSpan; REG_MAXSUB],
}

/// `struct Renode` -- `type` is spelled `type_` in Rust.
#[repr(C)]
#[derive(Copy, Clone)]
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

#[repr(C)]
#[derive(Copy, Clone)]
pub struct Reinst {
    pub opcode: u8,
    pub n: u8,
    pub c: Rune,
    pub cc: *mut Reclass,
    pub x: *mut Reinst,
    pub y: *mut Reinst,
}

/// `struct cstate`.  The `jmp_buf kaboom` member of the C original is gone:
/// `die()` panics instead of calling `longjmp()`.
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

/* --------------------------------------------------------- die / longjmp */

/// Marker for the panic that emulates the single `longjmp(g->kaboom, 1)` of the
/// C original.  The payload actually thrown is `crate::jsi::JsThrow(RE_DIE)` so
/// that the panic hook installed by `install_panic_hook()` keeps stderr clean.
struct ReDie;

/// Owner id that can never be handed out by `next_try_id()`.
const RE_DIE: u64 = u64::MAX;

unsafe fn die(g: *mut cstate, message: *const c_char) -> ! {
    (*g).error = message;
    std::panic::panic_any(JsThrow(RE_DIE))
}

unsafe fn canon(c: Rune) -> c_int {
    let u: Rune = toupperrune(c);
    if c >= 128 && u < 128 {
        return c;
    }
    u
}

/* ------------------------------------------------------------------ Scan */

const L_CHAR: c_int = 256;
/// character class
const L_CCLASS: c_int = 257;
/// negative character class
const L_NCCLASS: c_int = 258;
/// `"(?:"` no capture
const L_NC: c_int = 259;
/// `"(?="` positive lookahead
const L_PLA: c_int = 260;
/// `"(?!"` negative lookahead
const L_NLA: c_int = 261;
/// `"\b"` word boundary
const L_WORD: c_int = 262;
/// `"\B"` non-word boundary
const L_NWORD: c_int = 263;
/// `"\1"` back-reference
const L_REF: c_int = 264;
/// `{M,N}`
const L_COUNT: c_int = 265;

unsafe fn hex(g: *mut cstate, c: c_int) -> c_int {
    if c >= b'0' as c_int && c <= b'9' as c_int {
        return c - b'0' as c_int;
    }
    if c >= b'a' as c_int && c <= b'f' as c_int {
        return c - b'a' as c_int + 0xA;
    }
    if c >= b'A' as c_int && c <= b'F' as c_int {
        return c - b'A' as c_int + 0xA;
    }
    die(g, c"invalid escape sequence".as_ptr());
}

unsafe fn dec(g: *mut cstate, c: c_int) -> c_int {
    if c >= b'0' as c_int && c <= b'9' as c_int {
        return c - b'0' as c_int;
    }
    die(g, c"invalid quantifier".as_ptr());
}

/// `#define ESCAPES "BbDdSsWw^$\\.*+?()[]{}|-0123456789"`
#[inline]
fn ESCAPES() -> *const c_char {
    c"BbDdSsWw^$\\.*+?()[]{}|-0123456789".as_ptr()
}

unsafe fn isunicodeletter(c: c_int) -> c_int {
    ((c >= b'a' as c_int && c <= b'z' as c_int)
        || (c >= b'A' as c_int && c <= b'Z' as c_int)
        || isalpharune(c) != 0) as c_int
}

unsafe fn nextrune(g: *mut cstate) -> c_int {
    if *(*g).source == 0 {
        (*g).yychar = EOF;
        return 0;
    }
    (*g).source = (*g)
        .source
        .offset(chartorune(&mut (*g).yychar as *mut Rune, (*g).source) as isize);
    if (*g).yychar == b'\\' as c_int {
        if *(*g).source == 0 {
            die(g, c"unterminated escape sequence".as_ptr());
        }
        (*g).source = (*g)
            .source
            .offset(chartorune(&mut (*g).yychar as *mut Rune, (*g).source) as isize);

        /* switch (g->yychar) */
        if (*g).yychar == b'f' as c_int {
            (*g).yychar = 0xC; /* '\f' */
            return 0;
        }
        if (*g).yychar == b'n' as c_int {
            (*g).yychar = 0xA; /* '\n' */
            return 0;
        }
        if (*g).yychar == b'r' as c_int {
            (*g).yychar = 0xD; /* '\r' */
            return 0;
        }
        if (*g).yychar == b't' as c_int {
            (*g).yychar = 0x9; /* '\t' */
            return 0;
        }
        if (*g).yychar == b'v' as c_int {
            (*g).yychar = 0xB; /* '\v' */
            return 0;
        }
        if (*g).yychar == b'c' as c_int {
            if *(*g).source.offset(0) == 0 {
                die(g, c"unterminated escape sequence".as_ptr());
            }
            let ch = *(*g).source as c_int;
            (*g).source = (*g).source.offset(1);
            (*g).yychar = ch & 31;
            return 0;
        }
        if (*g).yychar == b'x' as c_int {
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
                (*g).yychar = b'0' as c_int;
                return 1;
            }
            return 1;
        }
        if (*g).yychar == b'u' as c_int {
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
                (*g).yychar = b'0' as c_int;
                return 1;
            }
            return 1;
        }
        if (*g).yychar == 0 {
            (*g).yychar = b'0' as c_int;
            return 1;
        }

        if !strchr(ESCAPES(), (*g).yychar).is_null() {
            return 1;
        }
        if isunicodeletter((*g).yychar) != 0 || (*g).yychar == b'_' as c_int {
            /* check identity escape */
            die(g, c"invalid escape character".as_ptr());
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
    while (*g).yychar != b',' as c_int && (*g).yychar != b'}' as c_int {
        (*g).yymin = (*g).yymin.wrapping_mul(10).wrapping_add(dec(g, (*g).yychar));
        (*g).yychar = *(*g).source as c_int;
        (*g).source = (*g).source.offset(1);
        if (*g).yymin >= REPINF {
            die(g, c"numeric overflow".as_ptr());
        }
    }

    if (*g).yychar == b',' as c_int {
        (*g).yychar = *(*g).source as c_int;
        (*g).source = (*g).source.offset(1);
        if (*g).yychar == b'}' as c_int {
            (*g).yymax = REPINF;
        } else {
            (*g).yymax = dec(g, (*g).yychar);
            (*g).yychar = *(*g).source as c_int;
            (*g).source = (*g).source.offset(1);
            while (*g).yychar != b'}' as c_int {
                (*g).yymax = (*g).yymax.wrapping_mul(10).wrapping_add(dec(g, (*g).yychar));
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

unsafe fn newcclass(g: *mut cstate) {
    if (*g).ncclass as usize >= REG_MAXCLASS {
        die(g, c"too many character classes".as_ptr());
    }
    let i = (*g).ncclass;
    (*g).ncclass += 1;
    (*g).yycc = (*g).cclass.as_mut_ptr().offset(i as isize);
    (*(*g).yycc).end = (*(*g).yycc).spans.as_mut_ptr();
}

unsafe fn addrange(g: *mut cstate, a: Rune, b: Rune) {
    let cc: *mut Reclass = (*g).yycc;
    let mut p: *mut Rune;

    if a > b {
        die(g, c"invalid character class range".as_ptr());
    }

    /* extend existing ranges if they overlap */
    p = (*cc).spans.as_mut_ptr();
    while p < (*cc).end {
        /* completely inside old range */
        if a >= *p && b <= *p.offset(1) {
            return;
        }

        /* completely swallows old range */
        if a < *p && b >= *p.offset(1) {
            *p = a;
            *p.offset(1) = b;
            return;
        }

        /* extend at start */
        if b >= (*p).wrapping_sub(1) && b <= *p.offset(1) && a < *p {
            *p = a;
            return;
        }

        /* extend at end */
        if a >= *p && a <= (*p.offset(1)).wrapping_add(1) && b > *p.offset(1) {
            *p.offset(1) = b;
            return;
        }

        p = p.offset(2);
    }

    if (*cc).end.wrapping_add(2) >= (*cc).spans.as_mut_ptr().wrapping_add(REG_MAXSPAN) {
        die(g, c"too many character class ranges".as_ptr());
    }
    *(*cc).end = a;
    (*cc).end = (*cc).end.offset(1);
    *(*cc).end = b;
    (*cc).end = (*cc).end.offset(1);
}

unsafe fn addranges_d(g: *mut cstate) {
    addrange(g, b'0' as Rune, b'9' as Rune);
}

unsafe fn addranges_D(g: *mut cstate) {
    addrange(g, 0, b'0' as Rune - 1);
    addrange(g, b'9' as Rune + 1, 0xFFFF);
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
    addrange(g, b'0' as Rune, b'9' as Rune);
    addrange(g, b'A' as Rune, b'Z' as Rune);
    addrange(g, b'_' as Rune, b'_' as Rune);
    addrange(g, b'a' as Rune, b'z' as Rune);
}

unsafe fn addranges_W(g: *mut cstate) {
    addrange(g, 0, b'0' as Rune - 1);
    addrange(g, b'9' as Rune + 1, b'A' as Rune - 1);
    addrange(g, b'Z' as Rune + 1, b'_' as Rune - 1);
    addrange(g, b'_' as Rune + 1, b'a' as Rune - 1);
    addrange(g, b'z' as Rune + 1, 0xFFFF);
}

unsafe fn lexclass(g: *mut cstate) -> c_int {
    let mut type_: c_int = L_CCLASS;
    let mut quoted: c_int;
    let mut havesave: c_int;
    let mut havedash: c_int;
    let mut save: Rune = 0;

    newcclass(g);

    quoted = nextrune(g);
    if quoted == 0 && (*g).yychar == b'^' as c_int {
        type_ = L_NCCLASS;
        quoted = nextrune(g);
    }

    havedash = 0;
    havesave = havedash;
    loop {
        if (*g).yychar == EOF {
            die(g, c"unterminated character class".as_ptr());
        }
        if quoted == 0 && (*g).yychar == b']' as c_int {
            break;
        }

        if quoted == 0 && (*g).yychar == b'-' as c_int {
            if havesave != 0 {
                if havedash != 0 {
                    addrange(g, save, b'-' as Rune);
                    havedash = 0;
                    havesave = havedash;
                } else {
                    havedash = 1;
                }
            } else {
                save = b'-' as Rune;
                havesave = 1;
            }
        } else if quoted != 0 && !strchr(c"DSWdsw".as_ptr(), (*g).yychar).is_null() {
            if havesave != 0 {
                addrange(g, save, save);
                if havedash != 0 {
                    addrange(g, b'-' as Rune, b'-' as Rune);
                }
            }
            /* switch (g->yychar) */
            if (*g).yychar == b'd' as c_int {
                addranges_d(g);
            } else if (*g).yychar == b's' as c_int {
                addranges_s(g);
            } else if (*g).yychar == b'w' as c_int {
                addranges_w(g);
            } else if (*g).yychar == b'D' as c_int {
                addranges_D(g);
            } else if (*g).yychar == b'S' as c_int {
                addranges_S(g);
            } else if (*g).yychar == b'W' as c_int {
                addranges_W(g);
            }
            havedash = 0;
            havesave = havedash;
        } else {
            if quoted != 0 {
                if (*g).yychar == b'b' as c_int {
                    (*g).yychar = 0x8; /* '\b' */
                } else if (*g).yychar == b'0' as c_int {
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
            addrange(g, b'-' as Rune, b'-' as Rune);
        }
    }

    type_
}

unsafe fn lex(g: *mut cstate) -> c_int {
    let quoted = nextrune(g);
    if quoted != 0 {
        /* switch (g->yychar) */
        if (*g).yychar == b'b' as c_int {
            return L_WORD;
        }
        if (*g).yychar == b'B' as c_int {
            return L_NWORD;
        }
        if (*g).yychar == b'd' as c_int {
            newcclass(g);
            addranges_d(g);
            return L_CCLASS;
        }
        if (*g).yychar == b's' as c_int {
            newcclass(g);
            addranges_s(g);
            return L_CCLASS;
        }
        if (*g).yychar == b'w' as c_int {
            newcclass(g);
            addranges_w(g);
            return L_CCLASS;
        }
        if (*g).yychar == b'D' as c_int {
            newcclass(g);
            addranges_d(g);
            return L_NCCLASS;
        }
        if (*g).yychar == b'S' as c_int {
            newcclass(g);
            addranges_s(g);
            return L_NCCLASS;
        }
        if (*g).yychar == b'W' as c_int {
            newcclass(g);
            addranges_w(g);
            return L_NCCLASS;
        }
        if (*g).yychar == b'0' as c_int {
            (*g).yychar = 0;
            return L_CHAR;
        }

        if (*g).yychar >= b'0' as c_int && (*g).yychar <= b'9' as c_int {
            (*g).yychar -= b'0' as c_int;
            if *(*g).source >= b'0' as c_char && *(*g).source <= b'9' as c_char {
                let d = *(*g).source as c_int;
                (*g).source = (*g).source.offset(1);
                (*g).yychar = (*g).yychar.wrapping_mul(10).wrapping_add(d) - b'0' as c_int;
            }
            return L_REF;
        }
        return L_CHAR;
    }

    /* switch (g->yychar) */
    if (*g).yychar == EOF
        || (*g).yychar == b'$' as c_int
        || (*g).yychar == b')' as c_int
        || (*g).yychar == b'*' as c_int
        || (*g).yychar == b'+' as c_int
        || (*g).yychar == b'.' as c_int
        || (*g).yychar == b'?' as c_int
        || (*g).yychar == b'^' as c_int
        || (*g).yychar == b'|' as c_int
    {
        return (*g).yychar;
    }

    if (*g).yychar == b'{' as c_int {
        return lexcount(g);
    }
    if (*g).yychar == b'[' as c_int {
        return lexclass(g);
    }
    if (*g).yychar == b'(' as c_int {
        if *(*g).source.offset(0) == b'?' as c_char {
            if *(*g).source.offset(1) == b':' as c_char {
                (*g).source = (*g).source.offset(2);
                return L_NC;
            }
            if *(*g).source.offset(1) == b'=' as c_char {
                (*g).source = (*g).source.offset(2);
                return L_PLA;
            }
            if *(*g).source.offset(1) == b'!' as c_char {
                (*g).source = (*g).source.offset(2);
                return L_NLA;
            }
        }
        return b'(' as c_int;
    }

    L_CHAR
}

/* ----------------------------------------------------------------- Parse */

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
        die(g, c"infinite loop matching the empty string".as_ptr());
    }
    (*rep).ng = ng as u8;
    (*rep).m = min as u8;
    (*rep).n = max as u8;
    (*rep).x = atom;
    rep
}

/// `static void next(struct cstate *g)` -- `#define next regnext`
unsafe fn regnext(g: *mut cstate) {
    (*g).lookahead = lex(g);
}

/// `static int accept(struct cstate *g, int t)` -- `#define accept regaccept`
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
        (*atom).cc =
            ((*g).yycc as *const Reclass).offset_from((*g).cclass.as_ptr()) as c_int;
        regnext(g);
        return atom;
    }
    if (*g).lookahead == L_NCCLASS {
        atom = newnode(g, P_NCCLASS);
        (*atom).cc =
            ((*g).yycc as *const Reclass).offset_from((*g).cclass.as_ptr()) as c_int;
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
        (*atom).n = (*g).yychar as u8;
        (*atom).x = (*g).sub[(*g).yychar as usize];
        regnext(g);
        return atom;
    }
    if regaccept(g, b'.' as c_int) != 0 {
        return newnode(g, P_ANY);
    }
    if regaccept(g, b'(' as c_int) != 0 {
        atom = newnode(g, P_PAR);
        if (*g).nsub as usize == REG_MAXSUB {
            die(g, c"too many captures".as_ptr());
        }
        (*atom).n = (*g).nsub as u8;
        (*g).nsub += 1;
        (*atom).x = parsealt(g);
        (*g).sub[(*atom).n as usize] = atom;
        if regaccept(g, b')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_NC) != 0 {
        atom = parsealt(g);
        if regaccept(g, b')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_PLA) != 0 {
        atom = newnode(g, P_PLA);
        (*atom).x = parsealt(g);
        if regaccept(g, b')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    if regaccept(g, L_NLA) != 0 {
        atom = newnode(g, P_NLA);
        (*atom).x = parsealt(g);
        if regaccept(g, b')' as c_int) == 0 {
            die(g, c"unmatched '('".as_ptr());
        }
        return atom;
    }
    die(g, c"syntax error".as_ptr());
}

unsafe fn parserep(g: *mut cstate) -> *mut Renode {
    let atom: *mut Renode;

    if regaccept(g, b'^' as c_int) != 0 {
        return newnode(g, P_BOL);
    }
    if regaccept(g, b'$' as c_int) != 0 {
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
        let min = (*g).yymin;
        let max = (*g).yymax;
        regnext(g);
        if max < min {
            die(g, c"invalid quantifier".as_ptr());
        }
        let ng = regaccept(g, b'?' as c_int);
        return newrep(g, atom, ng, min, max);
    }
    if regaccept(g, b'*' as c_int) != 0 {
        let ng = regaccept(g, b'?' as c_int);
        return newrep(g, atom, ng, 0, REPINF);
    }
    if regaccept(g, b'+' as c_int) != 0 {
        let ng = regaccept(g, b'?' as c_int);
        return newrep(g, atom, ng, 1, REPINF);
    }
    if regaccept(g, b'?' as c_int) != 0 {
        let ng = regaccept(g, b'?' as c_int);
        return newrep(g, atom, ng, 0, 1);
    }
    atom
}

unsafe fn parsecat(g: *mut cstate) -> *mut Renode {
    let mut cat: *mut Renode;
    let mut head: *mut Renode;
    let mut tail: *mut *mut Renode;
    if (*g).lookahead != EOF && (*g).lookahead != b'|' as c_int && (*g).lookahead != b')' as c_int {
        /* Build a right-leaning tree by splicing in new 'cat' at the tail. */
        head = parserep(g);
        tail = &mut head as *mut *mut Renode;
        while (*g).lookahead != EOF
            && (*g).lookahead != b'|' as c_int
            && (*g).lookahead != b')' as c_int
        {
            cat = newnode(g, P_CAT);
            (*cat).x = *tail;
            (*cat).y = parserep(g);
            *tail = cat;
            tail = &mut (*cat).y as *mut *mut Renode;
        }
        return head;
    }
    null_mut()
}

unsafe fn parsealt(g: *mut cstate) -> *mut Renode {
    let mut alt: *mut Renode;
    let mut x: *mut Renode;
    alt = parsecat(g);
    while regaccept(g, b'|' as c_int) != 0 {
        x = alt;
        alt = newnode(g, P_ALT);
        (*alt).x = x;
        (*alt).y = parsecat(g);
    }
    alt
}

/* --------------------------------------------------------------- Compile */

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
    match (*node).type_ as c_int {
        P_CAT => count(g, (*node).x, depth).wrapping_add(count(g, (*node).y, depth)),
        P_ALT => count(g, (*node).x, depth)
            .wrapping_add(count(g, (*node).y, depth))
            .wrapping_add(2),
        P_REP => {
            min = (*node).m as c_int;
            max = (*node).n as c_int;
            if min == max {
                n = count(g, (*node).x, depth).wrapping_mul(min);
            } else if max < REPINF {
                n = count(g, (*node).x, depth)
                    .wrapping_mul(max)
                    .wrapping_add(max - min);
            } else {
                n = count(g, (*node).x, depth)
                    .wrapping_mul(min.wrapping_add(1))
                    .wrapping_add(2);
            }
            if n < 0 || n > REG_MAXPROG {
                die(g, c"program too large".as_ptr());
            }
            n
        }
        P_PAR => count(g, (*node).x, depth).wrapping_add(2),
        P_PLA => count(g, (*node).x, depth).wrapping_add(2),
        P_NLA => count(g, (*node).x, depth).wrapping_add(2),
        _ => 1,
    }
}

/// `static Reinst *emit(Reprog *prog, int opcode)` -- `#define emit regemit`
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

    /* loop: */
    loop {
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

/* ------------------------------------------------------------ public API */

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcompx(
    alloc: ReAlloc,
    ctx: *mut c_void,
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    crate::jsi::install_panic_hook();

    let mut gbox: Box<cstate> = Box::new(core::mem::zeroed());
    let g: *mut cstate = &mut *gbox;

    (*g).pstart = null_mut();
    (*g).prog = null_mut();

    /* The single `if (setjmp(g.kaboom))` of the C original: `die()` panics with
     * `JsThrow(RE_DIE)`; everything that followed the `setjmp` lives in the
     * closure below.  */
    let r = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| -> *mut Reprog {
        let node: *mut Renode;
        let split: *mut Reinst;
        let jump: *mut Reinst;
        let mut i: c_int;
        let mut n: c_int;

        (*g).prog =
            alloc.unwrap()(ctx, null_mut(), core::mem::size_of::<Reprog>() as c_int) as *mut Reprog;
        if (*g).prog.is_null() {
            die(g, c"cannot allocate regular expression".as_ptr());
        }
        (*(*g).prog).start = null_mut();
        (*(*g).prog).cclass = null_mut();

        n = strlen(pattern).wrapping_mul(2) as c_int;
        if n > REG_MAXPROG {
            die(g, c"program too large".as_ptr());
        }
        if n > 0 {
            (*g).pend = alloc.unwrap()(
                ctx,
                null_mut(),
                (core::mem::size_of::<Renode>().wrapping_mul(n as usize)) as c_int,
            ) as *mut Renode;
            (*g).pstart = (*g).pend;
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
        if (*g).lookahead == b')' as c_int {
            die(g, c"unmatched ')'".as_ptr());
        }
        if (*g).lookahead != EOF {
            die(g, c"syntax error".as_ptr());
        }

        n = count(g, node, 0).wrapping_add(6);
        if n < 0 || n > REG_MAXPROG {
            die(g, c"program too large".as_ptr());
        }

        (*(*g).prog).nsub = (*g).nsub;
        (*(*g).prog).end = alloc.unwrap()(
            ctx,
            null_mut(),
            ((n as usize).wrapping_mul(core::mem::size_of::<Reinst>())) as c_int,
        ) as *mut Reinst;
        (*(*g).prog).start = (*(*g).prog).end;
        if (*(*g).prog).start.is_null() {
            die(
                g,
                c"cannot allocate regular expression instruction list".as_ptr(),
            );
        }

        if (*g).ncclass > 0 {
            (*(*g).prog).cclass = alloc.unwrap()(
                ctx,
                null_mut(),
                (((*g).ncclass as usize).wrapping_mul(core::mem::size_of::<Reclass>())) as c_int,
            ) as *mut Reclass;
            if (*(*g).prog).cclass.is_null() {
                die(
                    g,
                    c"cannot allocate regular expression character class list".as_ptr(),
                );
            }
            memcpy(
                (*(*g).prog).cclass as *mut c_void,
                (*g).cclass.as_ptr() as *const c_void,
                ((*g).ncclass as usize).wrapping_mul(core::mem::size_of::<Reclass>()),
            );
            i = 0;
            while i < (*g).ncclass {
                let src: *mut Reclass = (*g).cclass.as_mut_ptr().offset(i as isize);
                let dst: *mut Reclass = (*(*g).prog).cclass.offset(i as isize);
                let len = (*src).end.offset_from((*src).spans.as_mut_ptr());
                (*dst).end = (*dst).spans.as_mut_ptr().offset(len);
                i += 1;
            }
        }

        split = regemit((*g).prog, I_SPLIT);
        (*split).x = split.offset(3);
        (*split).y = split.offset(1);
        regemit((*g).prog, I_ANYNL);
        jump = regemit((*g).prog, I_JUMP);
        (*jump).x = split;
        regemit((*g).prog, I_LPAR);
        compile((*g).prog, node);
        regemit((*g).prog, I_RPAR);
        regemit((*g).prog, I_END);

        alloc.unwrap()(ctx, (*g).pstart as *mut c_void, 0);

        if !errorp.is_null() {
            *errorp = null();
        }
        (*g).prog
    }));

    match r {
        Ok(prog) => prog,
        Err(p) => {
            let is_die = match p.downcast_ref::<JsThrow>() {
                Some(t) => t.0 == RE_DIE,
                None => false,
            };
            if is_die {
                if !errorp.is_null() {
                    *errorp = (*g).error;
                }
                alloc.unwrap()(ctx, (*g).pstart as *mut c_void, 0);
                if !(*g).prog.is_null() {
                    alloc.unwrap()(ctx, (*(*g).prog).cclass as *mut c_void, 0);
                    alloc.unwrap()(ctx, (*(*g).prog).start as *mut c_void, 0);
                    alloc.unwrap()(ctx, (*g).prog as *mut c_void, 0);
                }
                null_mut()
            } else {
                std::panic::resume_unwind(p)
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfreex(alloc: ReAlloc, ctx: *mut c_void, prog: *mut Reprog) {
    if !prog.is_null() {
        if !(*prog).cclass.is_null() {
            alloc.unwrap()(ctx, (*prog).cclass as *mut c_void, 0);
        }
        alloc.unwrap()(ctx, (*prog).start as *mut c_void, 0);
        alloc.unwrap()(ctx, prog as *mut c_void, 0);
    }
}

unsafe extern "C-unwind" fn default_alloc(
    ctx: *mut c_void,
    p: *mut c_void,
    n: c_int,
) -> *mut c_void {
    if n == 0 {
        free(p);
        return null_mut();
    }
    realloc(p, n as size_t)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regcomp(
    pattern: *const c_char,
    cflags: c_int,
    errorp: *mut *const c_char,
) -> *mut Reprog {
    js_regcompx(Some(default_alloc), null_mut(), pattern, cflags, errorp)
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regfree(prog: *mut Reprog) {
    js_regfreex(Some(default_alloc), null_mut(), prog);
}

/* ----------------------------------------------------------------- Match */

unsafe fn isnewline(c: c_int) -> c_int {
    (c == 0xA || c == 0xD || c == 0x2028 || c == 0x2029) as c_int
}

unsafe fn iswordchar(c: c_int) -> c_int {
    (c == b'_' as c_int
        || (c >= b'a' as c_int && c <= b'z' as c_int)
        || (c >= b'A' as c_int && c <= b'Z' as c_int)
        || (c >= b'0' as c_int && c <= b'9' as c_int)) as c_int
}

unsafe fn incclass(cc: *mut Reclass, c: Rune) -> c_int {
    let mut p: *mut Rune = (*cc).spans.as_mut_ptr();
    while p < (*cc).end {
        if *p <= c && c <= *p.offset(1) {
            return 1;
        }
        p = p.offset(2);
    }
    0
}

unsafe fn incclasscanon(cc: *mut Reclass, c: Rune) -> c_int {
    let mut p: *mut Rune = (*cc).spans.as_mut_ptr();
    let mut r: Rune;
    while p < (*cc).end {
        r = *p;
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
    let mut a = a;
    let mut b = b;
    let mut n = n;
    while n != 0 {
        n = n.wrapping_sub(1);
        if *a == 0 {
            return -1;
        }
        if *b == 0 {
            return 1;
        }
        a = a.offset(chartorune(&mut ra as *mut Rune, a) as isize);
        b = b.offset(chartorune(&mut rb as *mut Rune, b) as isize);
        let c = canon(ra) - canon(rb);
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
                result = r#match((*pc).x, sp, bol, flags, &mut scratch as *mut Resub, depth + 1);
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
                result = r#match((*pc).x, sp, bol, flags, &mut scratch as *mut Resub, depth + 1);
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
                sp = sp.offset(chartorune(&mut c as *mut Rune, sp) as isize);
                pc = pc.offset(1);
            }
            I_ANY => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(chartorune(&mut c as *mut Rune, sp) as isize);
                if isnewline(c) != 0 {
                    return 1;
                }
                pc = pc.offset(1);
            }
            I_CHAR => {
                if *sp == 0 {
                    return 1;
                }
                sp = sp.offset(chartorune(&mut c as *mut Rune, sp) as isize);
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
                sp = sp.offset(chartorune(&mut c as *mut Rune, sp) as isize);
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
                sp = sp.offset(chartorune(&mut c as *mut Rune, sp) as isize);
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
                let k = (*pc).n as usize;
                i = (((*out).sub[k].ep as isize) - ((*out).sub[k].sp as isize)) as c_int;
                if (flags & REG_ICASE) != 0 {
                    if strncmpcanon(sp, (*out).sub[k].sp, i) != 0 {
                        return 1;
                    }
                } else {
                    if strncmp(sp, (*out).sub[k].sp, i as size_t) != 0 {
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
                } else if (flags & REG_NEWLINE) != 0
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
                } else if (flags & REG_NEWLINE) != 0 && isnewline(*sp as c_int) != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn js_regexec(
    prog: *mut Reprog,
    sp: *const c_char,
    sub: *mut Resub,
    eflags: c_int,
) -> c_int {
    let mut scratch: Resub = core::mem::zeroed();
    let mut i: usize;

    let mut sub = sub;
    if sub.is_null() {
        sub = &mut scratch as *mut Resub;
    }

    (*sub).nsub = (*prog).nsub;
    i = 0;
    while i < REG_MAXSUB {
        (*sub).sub[i].ep = null();
        (*sub).sub[i].sp = (*sub).sub[i].ep;
        i += 1;
    }

    r#match((*prog).start, sp, sp, (*prog).flags | eflags, sub, 0)
}

/* Short names, as seen by the C source through the `#define`s in regexp.h. */
pub use self::js_regcomp as regcomp;
pub use self::js_regcompx as regcompx;
pub use self::js_regexec as regexec;
pub use self::js_regfree as regfree;
pub use self::js_regfreex as regfreex;
