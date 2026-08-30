// Rust translation of the glibc `<ctype.h>` behaviour that the C library relies
// upon.
//
// The original C code in `c_src/src/driver.c` calls the standard `is*()`
// classification macros and prints their results with `printf("%d")`.  On glibc
// those macros expand to
//
//     #define isalnum(c) __isctype((c), _ISalnum)
//     #define __isctype(c, type) ((*__ctype_b_loc ())[(int) (c)] & (type))
//
// so the value that reaches `printf` is *not* a normalised `0`/`1`; it is the
// raw bit that glibc stores in its `__ctype_b` table.  Those bits are produced
// by `_ISbit(n)`:
//
//     #define _ISbit(bit) ((bit) < 8 ? ((1 << (bit)) << 8) : ((1 << (bit)) >> 8))
//
// The resulting masks are reproduced below verbatim so that the printed numbers
// are byte-identical to the C library's output.
//
// INDEXING.  glibc's tables span the indices `-128 ..= 255`, and `driver`'s
// parameter is a *signed* `char`, so the reachable index range is `-128 ..= 127`
// — i.e. exactly the 256 bit patterns of one byte.  The tables below are
// therefore keyed directly by that byte (`u8`), which makes every lookup
// structurally in-bounds: no matter what a caller leaves in the argument
// register, the index cannot escape the table.  This is not merely defensive.
// A previous version took a `c_int` and guarded the lookup with
// `if c >= -128 && c < 256`; because the argument arrives as an `i8` (which the
// ABI declares sign-extended) the optimiser is entitled to fold that guard away
// and index with the full-width register, which made the *release* build read
// out of bounds and crash when a caller passed an `int` that does not fit in a
// `char`.  The C library truncates to the low byte in that situation, and
// byte-keyed tables reproduce that truncation exactly, at every optimisation
// level.
//
// For the `-128 ..= 255` range the byte-keyed tables are observationally
// identical to the `-128 ..= 255` ones they replace: in the `"C"` locale both
// the negative half of the table and the `128 ..= 255` half carry no
// classification bits at all.
#![allow(dead_code)]

use core::ffi::c_int;

pub const IS_UPPER: c_int = 0x0100; // _ISbit(0)  ->   256
pub const IS_LOWER: c_int = 0x0200; // _ISbit(1)  ->   512
pub const IS_ALPHA: c_int = 0x0400; // _ISbit(2)  ->  1024
pub const IS_DIGIT: c_int = 0x0800; // _ISbit(3)  ->  2048
pub const IS_XDIGIT: c_int = 0x1000; // _ISbit(4) ->  4096
pub const IS_SPACE: c_int = 0x2000; // _ISbit(5)  ->  8192
pub const IS_PRINT: c_int = 0x4000; // _ISbit(6)  -> 16384
pub const IS_GRAPH: c_int = 0x8000; // _ISbit(7)  -> 32768
pub const IS_BLANK: c_int = 0x0001; // _ISbit(8)  ->     1
pub const IS_CNTRL: c_int = 0x0002; // _ISbit(9)  ->     2
pub const IS_PUNCT: c_int = 0x0004; // _ISbit(10) ->     4
pub const IS_ALNUM: c_int = 0x0008; // _ISbit(11) ->     8

/// The class bitmap of the `__ctype_b` slot reached by the `char` whose bit
/// pattern is `byte`, in the `"C"` locale.
///
/// Bytes `0x80 ..= 0xFF` are *negative* `char`s, so they select the negative
/// half of glibc's table, which is completely empty in the `"C"` locale: every
/// `is*()` predicate yields `0` for them.  Only 7-bit ASCII carries bits.
const fn class_of(byte: u8) -> c_int {
    if byte > 127 {
        // Negative `char` values have an all-zero class entry in the "C" locale.
        return 0;
    }

    let c = byte;
    let mut mask: c_int = 0;

    let is_cntrl = c < 32 || c == 127;
    let is_print = c >= 32 && c <= 126;
    let is_graph = c >= 33 && c <= 126;
    let is_space = matches!(c, b'\t' | b'\n' | 0x0b | 0x0c | b'\r' | b' ');
    let is_blank = matches!(c, b'\t' | b' ');
    let is_digit = c >= b'0' && c <= b'9';
    let is_upper = c >= b'A' && c <= b'Z';
    let is_lower = c >= b'a' && c <= b'z';
    let is_xdigit = is_digit || (c >= b'A' && c <= b'F') || (c >= b'a' && c <= b'f');
    let is_alpha = is_upper || is_lower;
    let is_alnum = is_alpha || is_digit;
    // In the "C" locale punctuation is every printable, non-alphanumeric,
    // non-space character.
    let is_punct = is_graph && !is_alnum;

    if is_upper {
        mask |= IS_UPPER;
    }
    if is_lower {
        mask |= IS_LOWER;
    }
    if is_alpha {
        mask |= IS_ALPHA;
    }
    if is_digit {
        mask |= IS_DIGIT;
    }
    if is_xdigit {
        mask |= IS_XDIGIT;
    }
    if is_space {
        mask |= IS_SPACE;
    }
    if is_print {
        mask |= IS_PRINT;
    }
    if is_graph {
        mask |= IS_GRAPH;
    }
    if is_blank {
        mask |= IS_BLANK;
    }
    if is_cntrl {
        mask |= IS_CNTRL;
    }
    if is_punct {
        mask |= IS_PUNCT;
    }
    if is_alnum {
        mask |= IS_ALNUM;
    }

    mask
}

/// `__ctype_b`-equivalent lookup, keyed by the byte pattern of the `char`.
const CTYPE_B: [c_int; 256] = {
    let mut table = [0; 256];
    let mut i = 0usize;
    while i < 256 {
        table[i] = class_of(i as u8);
        i += 1;
    }
    table
};

#[inline]
fn class(c: u8) -> c_int {
    CTYPE_B[c as usize]
}

// The twelve classification helpers.  Each one returns the *masked bit*, exactly
// like glibc's `__isctype` macro does.
#[inline]
pub fn isalnum(c: u8) -> c_int {
    class(c) & IS_ALNUM
}
#[inline]
pub fn isalpha(c: u8) -> c_int {
    class(c) & IS_ALPHA
}
#[inline]
pub fn islower(c: u8) -> c_int {
    class(c) & IS_LOWER
}
#[inline]
pub fn isupper(c: u8) -> c_int {
    class(c) & IS_UPPER
}
#[inline]
pub fn isdigit(c: u8) -> c_int {
    class(c) & IS_DIGIT
}
#[inline]
pub fn isxdigit(c: u8) -> c_int {
    class(c) & IS_XDIGIT
}
#[inline]
pub fn iscntrl(c: u8) -> c_int {
    class(c) & IS_CNTRL
}
#[inline]
pub fn isgraph(c: u8) -> c_int {
    class(c) & IS_GRAPH
}
#[inline]
pub fn isspace(c: u8) -> c_int {
    class(c) & IS_SPACE
}
#[inline]
pub fn isblank(c: u8) -> c_int {
    class(c) & IS_BLANK
}
#[inline]
pub fn isprint(c: u8) -> c_int {
    class(c) & IS_PRINT
}
#[inline]
pub fn ispunct(c: u8) -> c_int {
    class(c) & IS_PUNCT
}

/// `__ctype_tolower`, keyed by the byte pattern of the `char`.
///
/// Bytes above `0x7F` are negative `char`s and map onto themselves, so the value
/// stays negative — which `printf("%c")` then narrows back to the original byte.
const CTYPE_TOLOWER: [c_int; 256] = {
    let mut table = [0; 256];
    let mut i = 0usize;
    while i < 256 {
        // Reinterpret the byte as the signed `char` the C passes around.
        let ch = i as u8 as i8 as c_int;
        table[i] = if ch >= b'A' as c_int && ch <= b'Z' as c_int {
            ch + 32
        } else {
            ch
        };
        i += 1;
    }
    table
};

/// `__ctype_toupper`, keyed by the byte pattern of the `char`.
const CTYPE_TOUPPER: [c_int; 256] = {
    let mut table = [0; 256];
    let mut i = 0usize;
    while i < 256 {
        let ch = i as u8 as i8 as c_int;
        table[i] = if ch >= b'a' as c_int && ch <= b'z' as c_int {
            ch - 32
        } else {
            ch
        };
        i += 1;
    }
    table
};

#[inline]
pub fn tolower(c: u8) -> c_int {
    CTYPE_TOLOWER[c as usize]
}

#[inline]
pub fn toupper(c: u8) -> c_int {
    CTYPE_TOUPPER[c as usize]
}
