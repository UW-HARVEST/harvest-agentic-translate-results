//! Faithful re-implementation of the glibc `<ctype.h>` lookup tables for the
//! `"C"` locale, as used by the original C program.
//!
//! Two details of the C original are load-bearing and are reproduced here:
//!
//! 1. glibc implements `isalnum()` and friends as macros that mask a table
//!    entry (`(*__ctype_b_loc ())[c] & _ISalnum`) and hand the *raw masked
//!    bits* back to the caller. So `printf("%d", isalpha('a'))` prints `1024`,
//!    not `1`. The mask constants below are exactly glibc's `_ISbit` values.
//!
//! 2. The tables are addressed from `-128` through `255`, because a plain
//!    `char` is signed on this platform. In the `"C"` locale every entry in
//!    the `-128..=-1` and `128..=255` regions is zero, and the case-conversion
//!    tables are the identity there. Passing a negative `char` therefore
//!    classifies as "nothing at all" rather than trapping.

/// `_ISbit(0)` — uppercase.
pub const IS_UPPER: u16 = 0x0100;
/// `_ISbit(1)` — lowercase.
pub const IS_LOWER: u16 = 0x0200;
/// `_ISbit(2)` — alphabetic.
pub const IS_ALPHA: u16 = 0x0400;
/// `_ISbit(3)` — decimal digit.
pub const IS_DIGIT: u16 = 0x0800;
/// `_ISbit(4)` — hexadecimal digit.
pub const IS_XDIGIT: u16 = 0x1000;
/// `_ISbit(5)` — whitespace.
pub const IS_SPACE: u16 = 0x2000;
/// `_ISbit(6)` — printing.
pub const IS_PRINT: u16 = 0x4000;
/// `_ISbit(7)` — graphical.
pub const IS_GRAPH: u16 = 0x8000;
/// `_ISbit(8)` — blank.
pub const IS_BLANK: u16 = 0x0001;
/// `_ISbit(9)` — control.
pub const IS_CNTRL: u16 = 0x0002;
/// `_ISbit(10)` — punctuation.
pub const IS_PUNCT: u16 = 0x0004;
/// `_ISbit(11)` — alphanumeric.
pub const IS_ALNUM: u16 = 0x0008;

/// The tables are indexed from `-128`, so this is added to the character value.
const TABLE_BIAS: i32 = 128;
/// `-128 ..= 255`, the index range glibc's ctype tables cover.
const TABLE_LEN: usize = 384;

/// Classification bits for one ASCII code point in the `"C"` locale.
const fn class_of(c: u8) -> u16 {
    let upper = c >= b'A' && c <= b'Z';
    let lower = c >= b'a' && c <= b'z';
    let digit = c >= b'0' && c <= b'9';
    let xdigit = digit || (c >= b'A' && c <= b'F') || (c >= b'a' && c <= b'f');
    let space = c == b' ' || (c >= 0x09 && c <= 0x0d);
    let blank = c == b' ' || c == b'\t';
    let print = c >= 0x20 && c <= 0x7e;
    let graph = c >= 0x21 && c <= 0x7e;
    let cntrl = c <= 0x1f || c == 0x7f;
    let alpha = upper || lower;
    let alnum = alpha || digit;
    let punct = graph && !alnum;

    let mut m = 0u16;
    if upper {
        m |= IS_UPPER;
    }
    if lower {
        m |= IS_LOWER;
    }
    if alpha {
        m |= IS_ALPHA;
    }
    if digit {
        m |= IS_DIGIT;
    }
    if xdigit {
        m |= IS_XDIGIT;
    }
    if space {
        m |= IS_SPACE;
    }
    if print {
        m |= IS_PRINT;
    }
    if graph {
        m |= IS_GRAPH;
    }
    if blank {
        m |= IS_BLANK;
    }
    if cntrl {
        m |= IS_CNTRL;
    }
    if punct {
        m |= IS_PUNCT;
    }
    if alnum {
        m |= IS_ALNUM;
    }
    m
}

/// glibc's `__ctype_b` table for the `"C"` locale, biased by [`TABLE_BIAS`].
const fn build_class_table() -> [u16; TABLE_LEN] {
    // Everything outside 0..=127 stays zero, exactly as in the "C" locale.
    let mut t = [0u16; TABLE_LEN];
    let mut i = 0usize;
    while i < 128 {
        t[TABLE_BIAS as usize + i] = class_of(i as u8);
        i += 1;
    }
    t
}

/// glibc's `__ctype_tolower` / `__ctype_toupper` tables, biased by
/// [`TABLE_BIAS`]. `to_upper` selects which of the two to build.
const fn build_case_table(to_upper: bool) -> [i32; TABLE_LEN] {
    let mut t = [0i32; TABLE_LEN];
    let mut i = 0usize;
    while i < TABLE_LEN {
        // Recover the signed character value this slot stands for.
        let c = i as i32 - TABLE_BIAS;
        // Identity outside the ASCII letters, including the negative range.
        t[i] = if to_upper {
            if c >= b'a' as i32 && c <= b'z' as i32 {
                c - 32
            } else {
                c
            }
        } else if c >= b'A' as i32 && c <= b'Z' as i32 {
            c + 32
        } else {
            c
        };
        i += 1;
    }
    t
}

static CTYPE_B: [u16; TABLE_LEN] = build_class_table();
static CTYPE_TOLOWER: [i32; TABLE_LEN] = build_case_table(false);
static CTYPE_TOUPPER: [i32; TABLE_LEN] = build_case_table(true);

/// `(*__ctype_b_loc ())[c] & mask` — returns the raw masked bits, like the
/// glibc macros, not a normalised 0/1.
fn is_ctype(c: i32, mask: u16) -> i32 {
    let idx = (c + TABLE_BIAS) as usize;
    (CTYPE_B[idx] & mask) as i32
}

pub fn isalnum(c: i32) -> i32 {
    is_ctype(c, IS_ALNUM)
}

pub fn isalpha(c: i32) -> i32 {
    is_ctype(c, IS_ALPHA)
}

pub fn islower(c: i32) -> i32 {
    is_ctype(c, IS_LOWER)
}

pub fn isupper(c: i32) -> i32 {
    is_ctype(c, IS_UPPER)
}

pub fn isdigit(c: i32) -> i32 {
    is_ctype(c, IS_DIGIT)
}

pub fn isxdigit(c: i32) -> i32 {
    is_ctype(c, IS_XDIGIT)
}

pub fn iscntrl(c: i32) -> i32 {
    is_ctype(c, IS_CNTRL)
}

pub fn isgraph(c: i32) -> i32 {
    is_ctype(c, IS_GRAPH)
}

pub fn isspace(c: i32) -> i32 {
    is_ctype(c, IS_SPACE)
}

pub fn isblank(c: i32) -> i32 {
    is_ctype(c, IS_BLANK)
}

pub fn isprint(c: i32) -> i32 {
    is_ctype(c, IS_PRINT)
}

pub fn ispunct(c: i32) -> i32 {
    is_ctype(c, IS_PUNCT)
}

pub fn tolower(c: i32) -> i32 {
    CTYPE_TOLOWER[(c + TABLE_BIAS) as usize]
}

pub fn toupper(c: i32) -> i32 {
    CTYPE_TOUPPER[(c + TABLE_BIAS) as usize]
}
