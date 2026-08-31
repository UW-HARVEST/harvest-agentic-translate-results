// Translation of c_src/src/utf.c and utf.h
#![allow(non_upper_case_globals, non_snake_case, dead_code)]

use crate::utfdata::*;
use std::ffi::c_char;
use std::ptr;

pub type Rune = i32;

pub const UTFmax: i32 = 4;
pub const Runesync: i32 = 0x80;
pub const Runeself: i32 = 0x80;
pub const Runeerror: i32 = 0xFFFD;
pub const Runemax: i32 = 0x10FFFF;

const Bit1: i32 = 7;
const Bitx: i32 = 6;
const Bit2: i32 = 5;
const Bit3: i32 = 4;
const Bit4: i32 = 3;
const Bit5: i32 = 2;

const T1: i32 = ((1 << (Bit1 + 1)) - 1) ^ 0xFF;
const Tx: i32 = ((1 << (Bitx + 1)) - 1) ^ 0xFF;
const T2: i32 = ((1 << (Bit2 + 1)) - 1) ^ 0xFF;
const T3: i32 = ((1 << (Bit3 + 1)) - 1) ^ 0xFF;
const T4: i32 = ((1 << (Bit4 + 1)) - 1) ^ 0xFF;
const T5: i32 = ((1 << (Bit5 + 1)) - 1) ^ 0xFF;

const Rune1: i32 = (1 << (Bit1 + 0 * Bitx)) - 1;
const Rune2: i32 = (1 << (Bit2 + 1 * Bitx)) - 1;
const Rune3: i32 = (1 << (Bit3 + 2 * Bitx)) - 1;
const Rune4: i32 = (1 << (Bit4 + 3 * Bitx)) - 1;

const Maskx: i32 = (1 << Bitx) - 1;
const Testx: i32 = Maskx ^ 0xFF;

const Bad: i32 = Runeerror;

#[inline]
unsafe fn ub(str: *const c_char, off: isize) -> i32 {
    unsafe { *(str.offset(off) as *const u8) as i32 }
}

/// int chartorune(Rune *rune, const char *str) -> jsU_chartorune
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_chartorune(rune: *mut Rune, str: *const c_char) -> i32 {
    unsafe {
        let c: i32;
        let c1: i32;
        let c2: i32;
        let c3: i32;
        let l: i32;

        /* overlong null character */
        if ub(str, 0) == 0xc0 && ub(str, 1) == 0x80 {
            *rune = 0;
            return 2;
        }

        c = ub(str, 0);
        if c < Tx {
            *rune = c;
            return 1;
        }

        c1 = ub(str, 1) ^ Tx;
        if (c1 & Testx) != 0 {
            *rune = Bad;
            return 1;
        }
        if c < T3 {
            if c < T2 {
                *rune = Bad;
                return 1;
            }
            l = ((c << Bitx) | c1) & Rune2;
            if l <= Rune1 {
                *rune = Bad;
                return 1;
            }
            *rune = l;
            return 2;
        }

        c2 = ub(str, 2) ^ Tx;
        if (c2 & Testx) != 0 {
            *rune = Bad;
            return 1;
        }
        if c < T4 {
            l = ((((c << Bitx) | c1) << Bitx) | c2) & Rune3;
            if l <= Rune2 {
                *rune = Bad;
                return 1;
            }
            *rune = l;
            return 3;
        }

        c3 = ub(str, 3) ^ Tx;
        if (c3 & Testx) != 0 {
            *rune = Bad;
            return 1;
        }
        if c < T5 {
            l = ((((((c << Bitx) | c1) << Bitx) | c2) << Bitx) | c3) & Rune4;
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

        *rune = Bad;
        1
    }
}

/// int runetochar(char *str, const Rune *rune) -> jsU_runetochar
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_runetochar(str: *mut c_char, rune: *const Rune) -> i32 {
    unsafe {
        let mut c: i32 = *rune;

        /* overlong null character */
        if c == 0 {
            *str.offset(0) = 0xc0u8 as c_char;
            *str.offset(1) = 0x80u8 as c_char;
            return 2;
        }

        if c <= Rune1 {
            *str.offset(0) = c as u8 as c_char;
            return 1;
        }

        if c <= Rune2 {
            *str.offset(0) = (T2 | (c >> (1 * Bitx))) as u8 as c_char;
            *str.offset(1) = (Tx | (c & Maskx)) as u8 as c_char;
            return 2;
        }

        if c > Runemax {
            c = Runeerror;
        }
        if c <= Rune3 {
            *str.offset(0) = (T3 | (c >> (2 * Bitx))) as u8 as c_char;
            *str.offset(1) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as u8 as c_char;
            *str.offset(2) = (Tx | (c & Maskx)) as u8 as c_char;
            return 3;
        }

        *str.offset(0) = (T4 | (c >> (3 * Bitx))) as u8 as c_char;
        *str.offset(1) = (Tx | ((c >> (2 * Bitx)) & Maskx)) as u8 as c_char;
        *str.offset(2) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as u8 as c_char;
        *str.offset(3) = (Tx | (c & Maskx)) as u8 as c_char;
        4
    }
}

/// int runelen(int c) -> jsU_runelen
#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_runelen(c: i32) -> i32 {
    unsafe {
        let rune: Rune = c;
        let mut str: [c_char; 10] = [0; 10];
        jsU_runetochar(str.as_mut_ptr(), &rune)
    }
}

fn ucd_bsearch(c: Rune, t: &[Rune], n: usize, ne: usize) -> *const Rune {
    let mut base: usize = 0;
    let mut n = n;
    while n > 1 {
        let m = n / 2;
        let p = base + m * ne;
        if c >= t[p] {
            base = p;
            n = n - m;
        } else {
            n = m;
        }
    }
    if n != 0 && c >= t[base] {
        return unsafe { t.as_ptr().add(base) };
    }
    ptr::null()
}

/// Rune tolowerrune(Rune c) -> jsU_tolowerrune
#[unsafe(no_mangle)]
pub extern "C" fn jsU_tolowerrune(c: Rune) -> Rune {
    unsafe {
        let p = ucd_bsearch(c, &ucd_tolower2, ucd_tolower2.len() / 3, 3);
        if !p.is_null() && c >= *p && c <= *p.add(1) {
            return c + *p.add(2);
        }
        let p = ucd_bsearch(c, &ucd_tolower1, ucd_tolower1.len() / 2, 2);
        if !p.is_null() && c == *p {
            return c + *p.add(1);
        }
        c
    }
}

/// Rune toupperrune(Rune c) -> jsU_toupperrune
#[unsafe(no_mangle)]
pub extern "C" fn jsU_toupperrune(c: Rune) -> Rune {
    unsafe {
        let p = ucd_bsearch(c, &ucd_toupper2, ucd_toupper2.len() / 3, 3);
        if !p.is_null() && c >= *p && c <= *p.add(1) {
            return c + *p.add(2);
        }
        let p = ucd_bsearch(c, &ucd_toupper1, ucd_toupper1.len() / 2, 2);
        if !p.is_null() && c == *p {
            return c + *p.add(1);
        }
        c
    }
}

/// int islowerrune(Rune c) -> jsU_islowerrune
#[unsafe(no_mangle)]
pub extern "C" fn jsU_islowerrune(c: Rune) -> i32 {
    unsafe {
        let p = ucd_bsearch(c, &ucd_toupper2, ucd_toupper2.len() / 3, 3);
        if !p.is_null() && c >= *p && c <= *p.add(1) {
            return 1;
        }
        let p = ucd_bsearch(c, &ucd_toupper1, ucd_toupper1.len() / 2, 2);
        if !p.is_null() && c == *p {
            return 1;
        }
        0
    }
}

/// int isupperrune(Rune c) -> jsU_isupperrune
#[unsafe(no_mangle)]
pub extern "C" fn jsU_isupperrune(c: Rune) -> i32 {
    unsafe {
        let p = ucd_bsearch(c, &ucd_tolower2, ucd_tolower2.len() / 3, 3);
        if !p.is_null() && c >= *p && c <= *p.add(1) {
            return 1;
        }
        let p = ucd_bsearch(c, &ucd_tolower1, ucd_tolower1.len() / 2, 2);
        if !p.is_null() && c == *p {
            return 1;
        }
        0
    }
}

/// int isalpharune(Rune c) -> jsU_isalpharune
#[unsafe(no_mangle)]
pub extern "C" fn jsU_isalpharune(c: Rune) -> i32 {
    unsafe {
        let p = ucd_bsearch(c, &ucd_alpha2, ucd_alpha2.len() / 2, 2);
        if !p.is_null() && c >= *p && c <= *p.add(1) {
            return 1;
        }
        let p = ucd_bsearch(c, &ucd_alpha1, ucd_alpha1.len(), 1);
        if !p.is_null() && c == *p {
            return 1;
        }
        0
    }
}

/// const Rune *tolowerrune_full(Rune c) -> jsU_tolowerrune_full
#[unsafe(no_mangle)]
pub extern "C" fn jsU_tolowerrune_full(c: Rune) -> *const Rune {
    unsafe {
        let p = ucd_bsearch(c, &ucd_tolower_full, ucd_tolower_full.len() / 4, 4);
        if !p.is_null() && c == *p {
            return p.add(1);
        }
        ptr::null()
    }
}

/// const Rune *toupperrune_full(Rune c) -> jsU_toupperrune_full
#[unsafe(no_mangle)]
pub extern "C" fn jsU_toupperrune_full(c: Rune) -> *const Rune {
    unsafe {
        let p = ucd_bsearch(c, &ucd_toupper_full, ucd_toupper_full.len() / 5, 5);
        if !p.is_null() && c == *p {
            return p.add(1);
        }
        ptr::null()
    }
}
