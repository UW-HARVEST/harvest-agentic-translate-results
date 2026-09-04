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

/// The class bitmap of a single entry of glibc's `__ctype_b` table for the
/// `"C"` locale.
///
/// glibc's table is indexed from `-128` to `255`.  In the `"C"` locale every
/// slot below `0` (i.e. the region reachable through a *negative* `char`) is
/// completely empty, which means all of the `is*()` predicates yield `0` for
/// negative characters.  Only the 7-bit ASCII range carries classification
/// bits.
const fn class_of(idx: c_int) -> c_int {
    if idx < 0 || idx > 127 {
        // Negative `char` values (and, defensively, anything past ASCII) have
        // an all-zero class entry in the "C" locale.
        return 0;
    }

    let c = idx as u8;
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

/// `__ctype_b`-equivalent lookup covering the indices `-128 ..= 255`, i.e. every
/// value a `char` (or an `unsigned char`) can take.
const CTYPE_B: [c_int; 384] = {
    let mut table = [0; 384];
    let mut i = 0usize;
    while i < 384 {
        // Slot 0 corresponds to index -128, slot 128 to index 0.
        table[i] = class_of(i as c_int - 128);
        i += 1;
    }
    table
};

#[inline]
fn class(c: c_int) -> c_int {
    // `get` rather than `[]` so that an index outside glibc's `-128 ..= 255`
    // window can never turn into an out-of-bounds read, no matter what the
    // optimiser is able to prove about the caller.
    match CTYPE_B.get((c as i64 + 128) as usize) {
        Some(&bits) => bits,
        None => 0,
    }
}

// The twelve classification helpers.  Each one returns the *masked bit*, exactly
// like glibc's `__isctype` macro does.
#[inline]
pub fn isalnum(c: c_int) -> c_int {
    class(c) & IS_ALNUM
}
#[inline]
pub fn isalpha(c: c_int) -> c_int {
    class(c) & IS_ALPHA
}
#[inline]
pub fn islower(c: c_int) -> c_int {
    class(c) & IS_LOWER
}
#[inline]
pub fn isupper(c: c_int) -> c_int {
    class(c) & IS_UPPER
}
#[inline]
pub fn isdigit(c: c_int) -> c_int {
    class(c) & IS_DIGIT
}
#[inline]
pub fn isxdigit(c: c_int) -> c_int {
    class(c) & IS_XDIGIT
}
#[inline]
pub fn iscntrl(c: c_int) -> c_int {
    class(c) & IS_CNTRL
}
#[inline]
pub fn isgraph(c: c_int) -> c_int {
    class(c) & IS_GRAPH
}
#[inline]
pub fn isspace(c: c_int) -> c_int {
    class(c) & IS_SPACE
}
#[inline]
pub fn isblank(c: c_int) -> c_int {
    class(c) & IS_BLANK
}
#[inline]
pub fn isprint(c: c_int) -> c_int {
    class(c) & IS_PRINT
}
#[inline]
pub fn ispunct(c: c_int) -> c_int {
    class(c) & IS_PUNCT
}

/// glibc's `__ctype_tolower` table for the `"C"` locale, again spanning
/// `-128 ..= 255`.  Negative slots map onto themselves (so the value stays
/// negative, which `printf("%c")` then narrows to the original byte).
const CTYPE_TOLOWER: [c_int; 384] = {
    let mut table = [0; 384];
    let mut i = 0usize;
    while i < 384 {
        let idx = i as c_int - 128;
        table[i] = if idx >= b'A' as c_int && idx <= b'Z' as c_int {
            idx + 32
        } else {
            idx
        };
        i += 1;
    }
    table
};

/// glibc's `__ctype_toupper` table for the `"C"` locale.
const CTYPE_TOUPPER: [c_int; 384] = {
    let mut table = [0; 384];
    let mut i = 0usize;
    while i < 384 {
        let idx = i as c_int - 128;
        table[i] = if idx >= b'a' as c_int && idx <= b'z' as c_int {
            idx - 32
        } else {
            idx
        };
        i += 1;
    }
    table
};

#[inline]
pub fn tolower(c: c_int) -> c_int {
    match CTYPE_TOLOWER.get((c as i64 + 128) as usize) {
        Some(&v) => v,
        None => c,
    }
}

#[inline]
pub fn toupper(c: c_int) -> c_int {
    match CTYPE_TOUPPER.get((c as i64 + 128) as usize) {
        Some(&v) => v,
        None => c,
    }
}
