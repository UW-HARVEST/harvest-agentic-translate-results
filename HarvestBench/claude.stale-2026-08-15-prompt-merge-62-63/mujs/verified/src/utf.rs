//! Translated from utf.c (Rob Pike / Ken Thompson UTF-8 + Unicode case tables).
#![allow(non_upper_case_globals)]

use crate::types::{Rune, EOF};
use crate::utfdata::*;
use std::os::raw::{c_char, c_int};

pub const UTFmax: c_int = 4;
pub const Runesync: Rune = 0x80;
pub const Runeself: Rune = 0x80;
pub const Runeerror: Rune = 0xFFFD;
pub const Runemax: Rune = 0x10FFFF;

const Bit1: c_int = 7;
const Bitx: c_int = 6;
const Bit2: c_int = 5;
const Bit3: c_int = 4;
const Bit4: c_int = 3;
const Bit5: c_int = 2;

const T1: c_int = (((1 << (Bit1 + 1)) - 1) ^ 0xFF);
const Tx: c_int = (((1 << (Bitx + 1)) - 1) ^ 0xFF);
const T2: c_int = (((1 << (Bit2 + 1)) - 1) ^ 0xFF);
const T3: c_int = (((1 << (Bit3 + 1)) - 1) ^ 0xFF);
const T4: c_int = (((1 << (Bit4 + 1)) - 1) ^ 0xFF);
const T5: c_int = (((1 << (Bit5 + 1)) - 1) ^ 0xFF);

const Rune1: c_int = (1 << (Bit1 + 0 * Bitx)) - 1;
const Rune2: c_int = (1 << (Bit2 + 1 * Bitx)) - 1;
const Rune3: c_int = (1 << (Bit3 + 2 * Bitx)) - 1;
const Rune4: c_int = (1 << (Bit4 + 3 * Bitx)) - 1;

const Maskx: c_int = (1 << Bitx) - 1;
const Testx: c_int = Maskx ^ 0xFF;

const Bad: Rune = Runeerror;

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_chartorune(rune: *mut Rune, str: *const c_char) -> c_int {
    let s = str;
    let b0 = *(s as *const u8) as c_int;
    let b1 = *(s.add(1) as *const u8) as c_int;

    /* overlong null character */
    if b0 == 0xc0 && b1 == 0x80 {
        *rune = 0;
        return 2;
    }

    /* one character sequence: 00000-0007F => T1 */
    let c = b0;
    if c < Tx {
        *rune = c;
        return 1;
    }

    /* two character sequence: 0080-07FF => T2 Tx */
    let c1 = b1 ^ Tx;
    if (c1 & Testx) != 0 {
        *rune = Bad;
        return 1;
    }
    if c < T3 {
        if c < T2 {
            *rune = Bad;
            return 1;
        }
        let l = ((c << Bitx) | c1) & Rune2;
        if l <= Rune1 {
            *rune = Bad;
            return 1;
        }
        *rune = l;
        return 2;
    }

    /* three character sequence: 0800-FFFF => T3 Tx Tx */
    let c2 = (*(s.add(2) as *const u8) as c_int) ^ Tx;
    if (c2 & Testx) != 0 {
        *rune = Bad;
        return 1;
    }
    if c < T4 {
        let l = ((((c << Bitx) | c1) << Bitx) | c2) & Rune3;
        if l <= Rune2 {
            *rune = Bad;
            return 1;
        }
        *rune = l;
        return 3;
    }

    /* four character sequence: 10000-10FFFF => T4 Tx Tx Tx */
    if UTFmax >= 4 {
        let c3 = (*(s.add(3) as *const u8) as c_int) ^ Tx;
        if (c3 & Testx) != 0 {
            *rune = Bad;
            return 1;
        }
        if c < T5 {
            let l = ((((((c << Bitx) | c1) << Bitx) | c2) << Bitx) | c3) & Rune4;
            if l <= Rune3 {
                *rune = Bad;
                return 1;
            }
            if l > Runemax {
                *rune = Bad;
                return 1;
            }
            *rune = l;
            return 4;
        }
    }

    *rune = Bad;
    1
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_runetochar(str: *mut c_char, rune: *const Rune) -> c_int {
    let mut c = *rune;

    /* overlong null character */
    if c == 0 {
        *str.add(0) = 0xc0u8 as c_char;
        *str.add(1) = 0x80u8 as c_char;
        return 2;
    }

    /* one character sequence: 00000-0007F => 00-7F */
    if c <= Rune1 {
        *str.add(0) = c as c_char;
        return 1;
    }

    /* two character sequence: 00080-007FF => T2 Tx */
    if c <= Rune2 {
        *str.add(0) = (T2 | (c >> (1 * Bitx))) as c_char;
        *str.add(1) = (Tx | (c & Maskx)) as c_char;
        return 2;
    }

    /* three character sequence: 00800-0FFFF => T3 Tx Tx */
    if c > Runemax {
        c = Runeerror;
    }
    if c <= Rune3 {
        *str.add(0) = (T3 | (c >> (2 * Bitx))) as c_char;
        *str.add(1) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
        *str.add(2) = (Tx | (c & Maskx)) as c_char;
        return 3;
    }

    /* four character sequence: 010000-1FFFFF => T4 Tx Tx Tx */
    *str.add(0) = (T4 | (c >> (3 * Bitx))) as c_char;
    *str.add(1) = (Tx | ((c >> (2 * Bitx)) & Maskx)) as c_char;
    *str.add(2) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
    *str.add(3) = (Tx | (c & Maskx)) as c_char;
    4
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_runelen(c: c_int) -> c_int {
    let rune: Rune = c;
    let mut str: [c_char; 10] = [0; 10];
    jsU_runetochar(str.as_mut_ptr(), &rune)
}

unsafe fn ucd_bsearch(c: Rune, t: &[Rune], n: usize, ne: usize) -> *const Rune {
    let base = t.as_ptr();
    let mut t = base;
    let mut n = n;
    while n > 1 {
        let m = n / 2;
        let p = t.add(m * ne);
        if c >= *p.add(0) {
            t = p;
            n = n - m;
        } else {
            n = m;
        }
    }
    if n != 0 && c >= *t.add(0) {
        return t;
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_tolowerrune(c: Rune) -> Rune {
    let p = ucd_bsearch(c, ucd_tolower2, ucd_tolower2.len() / 3, 3);
    if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
        return c + *p.add(2);
    }
    let p = ucd_bsearch(c, ucd_tolower1, ucd_tolower1.len() / 2, 2);
    if !p.is_null() && c == *p.add(0) {
        return c + *p.add(1);
    }
    c
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_toupperrune(c: Rune) -> Rune {
    let p = ucd_bsearch(c, ucd_toupper2, ucd_toupper2.len() / 3, 3);
    if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
        return c + *p.add(2);
    }
    let p = ucd_bsearch(c, ucd_toupper1, ucd_toupper1.len() / 2, 2);
    if !p.is_null() && c == *p.add(0) {
        return c + *p.add(1);
    }
    c
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_islowerrune(c: Rune) -> c_int {
    let p = ucd_bsearch(c, ucd_toupper2, ucd_toupper2.len() / 3, 3);
    if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
        return 1;
    }
    let p = ucd_bsearch(c, ucd_toupper1, ucd_toupper1.len() / 2, 2);
    if !p.is_null() && c == *p.add(0) {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_isupperrune(c: Rune) -> c_int {
    let p = ucd_bsearch(c, ucd_tolower2, ucd_tolower2.len() / 3, 3);
    if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
        return 1;
    }
    let p = ucd_bsearch(c, ucd_tolower1, ucd_tolower1.len() / 2, 2);
    if !p.is_null() && c == *p.add(0) {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_isalpharune(c: Rune) -> c_int {
    let p = ucd_bsearch(c, ucd_alpha2, ucd_alpha2.len() / 2, 2);
    if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
        return 1;
    }
    let p = ucd_bsearch(c, ucd_alpha1, ucd_alpha1.len(), 1);
    if !p.is_null() && c == *p.add(0) {
        return 1;
    }
    0
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_tolowerrune_full(c: Rune) -> *const Rune {
    let p = ucd_bsearch(c, ucd_tolower_full, ucd_tolower_full.len() / 4, 4);
    if !p.is_null() && c == *p.add(0) {
        return p.add(1);
    }
    std::ptr::null()
}

#[no_mangle]
pub unsafe extern "C-unwind" fn jsU_toupperrune_full(c: Rune) -> *const Rune {
    let p = ucd_bsearch(c, ucd_toupper_full, ucd_toupper_full.len() / 5, 5);
    if !p.is_null() && c == *p.add(0) {
        return p.add(1);
    }
    std::ptr::null()
}

/* Convenience aliases matching the C `#define` short names */
pub use jsU_chartorune as chartorune;
pub use jsU_isalpharune as isalpharune;
pub use jsU_islowerrune as islowerrune;
pub use jsU_isupperrune as isupperrune;
pub use jsU_runelen as runelen;
pub use jsU_runetochar as runetochar;
pub use jsU_tolowerrune as tolowerrune;
pub use jsU_tolowerrune_full as tolowerrune_full;
pub use jsU_toupperrune as toupperrune;
pub use jsU_toupperrune_full as toupperrune_full;
