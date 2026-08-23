//! Translation of `c_src/src/utf.c`
//!
//! The original file is the Plan 9 / Lucent Technologies UTF-8 library as
//! shipped with MuJS.  `src/utf.h` renames every public function with a
//! `jsU_` prefix via `#define`, so the linker names defined here are the
//! `jsU_*` ones; short-name aliases for the other Rust modules are at the
//! bottom of the file.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::cstd::*;
use crate::jsi::*;
use crate::utfdata::*;
use core::ptr::{null, null_mut};

/*
 * enum from utf.c
 */

const Bit1: c_int = 7;
const Bitx: c_int = 6;
const Bit2: c_int = 5;
const Bit3: c_int = 4;
const Bit4: c_int = 3;
const Bit5: c_int = 2;

const T1: c_int = ((1 << (Bit1 + 1)) - 1) ^ 0xFF; /* 0000 0000 */
const Tx: c_int = ((1 << (Bitx + 1)) - 1) ^ 0xFF; /* 1000 0000 */
const T2: c_int = ((1 << (Bit2 + 1)) - 1) ^ 0xFF; /* 1100 0000 */
const T3: c_int = ((1 << (Bit3 + 1)) - 1) ^ 0xFF; /* 1110 0000 */
const T4: c_int = ((1 << (Bit4 + 1)) - 1) ^ 0xFF; /* 1111 0000 */
const T5: c_int = ((1 << (Bit5 + 1)) - 1) ^ 0xFF; /* 1111 1000 */

const Rune1: c_int = (1 << (Bit1 + 0 * Bitx)) - 1; /* 0000 0000 0000 0000 0111 1111 */
const Rune2: c_int = (1 << (Bit2 + 1 * Bitx)) - 1; /* 0000 0000 0000 0111 1111 1111 */
const Rune3: c_int = (1 << (Bit3 + 2 * Bitx)) - 1; /* 0000 0000 1111 1111 1111 1111 */
const Rune4: c_int = (1 << (Bit4 + 3 * Bitx)) - 1; /* 0001 1111 1111 1111 1111 1111 */

const Maskx: c_int = (1 << Bitx) - 1; /* 0011 1111 */
const Testx: c_int = Maskx ^ 0xFF; /* 1100 0000 */

const Bad: c_int = Runeerror;

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_chartorune(rune: *mut Rune, str: *const c_char) -> c_int {
    let c: c_int;
    let c1: c_int;
    let c2: c_int;
    let c3: c_int;
    let l: c_int;

    'bad: {
        /* overlong null character */
        if *(str as *const u8) == 0xc0 && *(str.offset(1) as *const u8) == 0x80 {
            *rune = 0;
            return 2;
        }

        /*
         * one character sequence
         *	00000-0007F => T1
         */
        c = *(str as *const u8) as c_int;
        if c < Tx {
            *rune = c;
            return 1;
        }

        /*
         * two character sequence
         *	0080-07FF => T2 Tx
         */
        c1 = *(str.offset(1) as *const u8) as c_int ^ Tx;
        if (c1 & Testx) != 0 {
            break 'bad;
        }
        if c < T3 {
            if c < T2 {
                break 'bad;
            }
            l = ((c << Bitx) | c1) & Rune2;
            if l <= Rune1 {
                break 'bad;
            }
            *rune = l;
            return 2;
        }

        /*
         * three character sequence
         *	0800-FFFF => T3 Tx Tx
         */
        c2 = *(str.offset(2) as *const u8) as c_int ^ Tx;
        if (c2 & Testx) != 0 {
            break 'bad;
        }
        if c < T4 {
            l = ((((c << Bitx) | c1) << Bitx) | c2) & Rune3;
            if l <= Rune2 {
                break 'bad;
            }
            *rune = l;
            return 3;
        }

        /*
         * four character sequence
         *	10000-10FFFF => T4 Tx Tx Tx
         */
        if UTFmax >= 4 {
            c3 = *(str.offset(3) as *const u8) as c_int ^ Tx;
            if (c3 & Testx) != 0 {
                break 'bad;
            }
            if c < T5 {
                l = ((((((c << Bitx) | c1) << Bitx) | c2) << Bitx) | c3) & Rune4;
                if l <= Rune3 {
                    break 'bad;
                }
                if l > Runemax {
                    break 'bad;
                }
                *rune = l;
                return 4;
            }
        }
    }

    /*
     * bad decoding
     */
    /* bad: */
    *rune = Bad;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_runetochar(str: *mut c_char, rune: *const Rune) -> c_int {
    let mut c: c_int = *rune;

    /* overlong null character */
    if c == 0 {
        *str.offset(0) = 0xc0u8 as c_char;
        *str.offset(1) = 0x80u8 as c_char;
        return 2;
    }

    /*
     * one character sequence
     *	00000-0007F => 00-7F
     */
    if c <= Rune1 {
        *str.offset(0) = c as c_char;
        return 1;
    }

    /*
     * two character sequence
     *	00080-007FF => T2 Tx
     */
    if c <= Rune2 {
        *str.offset(0) = (T2 | (c >> (1 * Bitx))) as c_char;
        *str.offset(1) = (Tx | (c & Maskx)) as c_char;
        return 2;
    }

    /*
     * three character sequence
     *	00800-0FFFF => T3 Tx Tx
     */
    if c > Runemax {
        c = Runeerror;
    }
    if c <= Rune3 {
        *str.offset(0) = (T3 | (c >> (2 * Bitx))) as c_char;
        *str.offset(1) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
        *str.offset(2) = (Tx | (c & Maskx)) as c_char;
        return 3;
    }

    /*
     * four character sequence
     *	010000-1FFFFF => T4 Tx Tx Tx
     */
    *str.offset(0) = (T4 | (c >> (3 * Bitx))) as c_char;
    *str.offset(1) = (Tx | ((c >> (2 * Bitx)) & Maskx)) as c_char;
    *str.offset(2) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
    *str.offset(3) = (Tx | (c & Maskx)) as c_char;
    4
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_runelen(c: c_int) -> c_int {
    let rune: Rune;
    let mut str = [0 as c_char; 10];

    rune = c;
    jsU_runetochar(str.as_mut_ptr(), &rune as *const Rune)
}

unsafe fn ucd_bsearch(c: Rune, t: *const Rune, n: c_int, ne: c_int) -> *const Rune {
    let mut t = t;
    let mut n = n;
    let mut p: *const Rune;
    let mut m: c_int;

    while n > 1 {
        m = n / 2;
        p = t.offset((m * ne) as isize);
        if c >= *p.offset(0) {
            t = p;
            n = n - m;
        } else {
            n = m;
        }
    }
    if n != 0 && c >= *t.offset(0) {
        return t;
    }
    null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_tolowerrune(c: Rune) -> Rune {
    let mut p: *const Rune;

    p = ucd_bsearch(
        c,
        ucd_tolower2.as_ptr(),
        (ucd_tolower2.len() as c_int) / 3,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return c.wrapping_add(*p.offset(2));
    }
    p = ucd_bsearch(
        c,
        ucd_tolower1.as_ptr(),
        (ucd_tolower1.len() as c_int) / 2,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return c.wrapping_add(*p.offset(1));
    }
    c
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_toupperrune(c: Rune) -> Rune {
    let mut p: *const Rune;

    p = ucd_bsearch(
        c,
        ucd_toupper2.as_ptr(),
        (ucd_toupper2.len() as c_int) / 3,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return c.wrapping_add(*p.offset(2));
    }
    p = ucd_bsearch(
        c,
        ucd_toupper1.as_ptr(),
        (ucd_toupper1.len() as c_int) / 2,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return c.wrapping_add(*p.offset(1));
    }
    c
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_islowerrune(c: Rune) -> c_int {
    let mut p: *const Rune;

    p = ucd_bsearch(
        c,
        ucd_toupper2.as_ptr(),
        (ucd_toupper2.len() as c_int) / 3,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return 1;
    }
    p = ucd_bsearch(
        c,
        ucd_toupper1.as_ptr(),
        (ucd_toupper1.len() as c_int) / 2,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_isupperrune(c: Rune) -> c_int {
    let mut p: *const Rune;

    p = ucd_bsearch(
        c,
        ucd_tolower2.as_ptr(),
        (ucd_tolower2.len() as c_int) / 3,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return 1;
    }
    p = ucd_bsearch(
        c,
        ucd_tolower1.as_ptr(),
        (ucd_tolower1.len() as c_int) / 2,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_isalpharune(c: Rune) -> c_int {
    let mut p: *const Rune;

    p = ucd_bsearch(c, ucd_alpha2.as_ptr(), (ucd_alpha2.len() as c_int) / 2, 2);
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return 1;
    }
    p = ucd_bsearch(c, ucd_alpha1.as_ptr(), ucd_alpha1.len() as c_int, 1);
    if !p.is_null() && c == *p.offset(0) {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_tolowerrune_full(c: Rune) -> *const Rune {
    let p: *const Rune;
    p = ucd_bsearch(
        c,
        ucd_tolower_full.as_ptr(),
        (ucd_tolower_full.len() as c_int) / 4,
        4,
    );
    if !p.is_null() && c == *p.offset(0) {
        return p.offset(1);
    }
    null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_toupperrune_full(c: Rune) -> *const Rune {
    let p: *const Rune;
    p = ucd_bsearch(
        c,
        ucd_toupper_full.as_ptr(),
        (ucd_toupper_full.len() as c_int) / 5,
        5,
    );
    if !p.is_null() && c == *p.offset(0) {
        return p.offset(1);
    }
    null()
}

/* The short names used internally by the rest of the C sources (utf.h
 * #defines them to the jsU_ prefixed symbols). */
pub use self::jsU_chartorune as chartorune;
pub use self::jsU_runetochar as runetochar;
pub use self::jsU_runelen as runelen;
pub use self::jsU_isalpharune as isalpharune;
pub use self::jsU_islowerrune as islowerrune;
pub use self::jsU_isupperrune as isupperrune;
pub use self::jsU_tolowerrune as tolowerrune;
pub use self::jsU_toupperrune as toupperrune;
pub use self::jsU_tolowerrune_full as tolowerrune_full;
pub use self::jsU_toupperrune_full as toupperrune_full;
