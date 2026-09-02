//! Translation of src/utf.c
//!
//! The public symbols are renamed by macros in utf.h:
//!   chartorune -> jsU_chartorune, runetochar -> jsU_runetochar, etc.
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use crate::jsi::*;
use crate::utfdata::*;

const Bit1: c_int = 7;
const Bitx: c_int = 6;
const Bit2: c_int = 5;
const Bit3: c_int = 4;
const Bit4: c_int = 3;
const Bit5: c_int = 2;

const T1: c_int = ((1 << (Bit1 + 1)) - 1) ^ 0xFF;
const Tx: c_int = ((1 << (Bitx + 1)) - 1) ^ 0xFF;
const T2: c_int = ((1 << (Bit2 + 1)) - 1) ^ 0xFF;
const T3: c_int = ((1 << (Bit3 + 1)) - 1) ^ 0xFF;
const T4: c_int = ((1 << (Bit4 + 1)) - 1) ^ 0xFF;
const T5: c_int = ((1 << (Bit5 + 1)) - 1) ^ 0xFF;

const Rune1: c_int = (1 << (Bit1 + 0 * Bitx)) - 1;
const Rune2: c_int = (1 << (Bit2 + 1 * Bitx)) - 1;
const Rune3: c_int = (1 << (Bit3 + 2 * Bitx)) - 1;
const Rune4: c_int = (1 << (Bit4 + 3 * Bitx)) - 1;

const Maskx: c_int = (1 << Bitx) - 1;
const Testx: c_int = Maskx ^ 0xFF;

const Bad: c_int = Runeerror;

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_chartorune(rune: *mut Rune, str: *const c_char) -> c_int {
    unsafe {
        let c: c_int;
        let c1: c_int;
        let c2: c_int;
        let c3: c_int;
        let l: c_int;

        /* overlong null character */
        if *(str as *const u8) == 0xc0 && *(str as *const u8).add(1) == 0x80 {
            *rune = 0;
            return 2;
        }

        c = *(str as *const u8) as c_int;
        if c < Tx {
            *rune = c;
            return 1;
        }

        c1 = *(str as *const u8).add(1) as c_int ^ Tx;
        if c1 & Testx != 0 {
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

        c2 = *(str as *const u8).add(2) as c_int ^ Tx;
        if c2 & Testx != 0 {
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

        /* if (UTFmax >= 4) */
        c3 = *(str as *const u8).add(3) as c_int ^ Tx;
        if c3 & Testx != 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_runetochar(str: *mut c_char, rune: *const Rune) -> c_int {
    unsafe {
        let mut c: c_int = *rune;

        /* overlong null character */
        if c == 0 {
            *str.add(0) = 0xc0u8 as c_char;
            *str.add(1) = 0x80u8 as c_char;
            return 2;
        }

        if c <= Rune1 {
            *str.add(0) = c as c_char;
            return 1;
        }

        if c <= Rune2 {
            *str.add(0) = (T2 | (c >> (1 * Bitx))) as c_char;
            *str.add(1) = (Tx | (c & Maskx)) as c_char;
            return 2;
        }

        if c > Runemax {
            c = Runeerror;
        }
        if c <= Rune3 {
            *str.add(0) = (T3 | (c >> (2 * Bitx))) as c_char;
            *str.add(1) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
            *str.add(2) = (Tx | (c & Maskx)) as c_char;
            return 3;
        }

        *str.add(0) = (T4 | (c >> (3 * Bitx))) as c_char;
        *str.add(1) = (Tx | ((c >> (2 * Bitx)) & Maskx)) as c_char;
        *str.add(2) = (Tx | ((c >> (1 * Bitx)) & Maskx)) as c_char;
        *str.add(3) = (Tx | (c & Maskx)) as c_char;
        4
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_runelen(c: c_int) -> c_int {
    unsafe {
        let rune: Rune = c;
        let mut str: [c_char; 10] = [0; 10];
        jsU_runetochar(str.as_mut_ptr(), &rune)
    }
}

unsafe fn ucd_bsearch(c: Rune, t: *const Rune, n: c_int, ne: c_int) -> *const Rune {
    unsafe {
        let mut t = t;
        let mut n = n;
        while n > 1 {
            let m = n / 2;
            let p = t.offset((m * ne) as isize);
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
        core::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_tolowerrune(c: Rune) -> Rune {
    unsafe {
        let mut p = ucd_bsearch(
            c,
            ucd_tolower2.as_ptr(),
            ucd_tolower2.len() as c_int / 3,
            3,
        );
        if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
            return c + *p.add(2);
        }
        p = ucd_bsearch(
            c,
            ucd_tolower1.as_ptr(),
            ucd_tolower1.len() as c_int / 2,
            2,
        );
        if !p.is_null() && c == *p.add(0) {
            return c + *p.add(1);
        }
        c
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_toupperrune(c: Rune) -> Rune {
    unsafe {
        let mut p = ucd_bsearch(
            c,
            ucd_toupper2.as_ptr(),
            ucd_toupper2.len() as c_int / 3,
            3,
        );
        if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
            return c + *p.add(2);
        }
        p = ucd_bsearch(
            c,
            ucd_toupper1.as_ptr(),
            ucd_toupper1.len() as c_int / 2,
            2,
        );
        if !p.is_null() && c == *p.add(0) {
            return c + *p.add(1);
        }
        c
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_islowerrune(c: Rune) -> c_int {
    unsafe {
        let mut p = ucd_bsearch(
            c,
            ucd_toupper2.as_ptr(),
            ucd_toupper2.len() as c_int / 3,
            3,
        );
        if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
            return 1;
        }
        p = ucd_bsearch(
            c,
            ucd_toupper1.as_ptr(),
            ucd_toupper1.len() as c_int / 2,
            2,
        );
        if !p.is_null() && c == *p.add(0) {
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_isupperrune(c: Rune) -> c_int {
    unsafe {
        let mut p = ucd_bsearch(
            c,
            ucd_tolower2.as_ptr(),
            ucd_tolower2.len() as c_int / 3,
            3,
        );
        if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
            return 1;
        }
        p = ucd_bsearch(
            c,
            ucd_tolower1.as_ptr(),
            ucd_tolower1.len() as c_int / 2,
            2,
        );
        if !p.is_null() && c == *p.add(0) {
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_isalpharune(c: Rune) -> c_int {
    unsafe {
        let mut p = ucd_bsearch(c, ucd_alpha2.as_ptr(), ucd_alpha2.len() as c_int / 2, 2);
        if !p.is_null() && c >= *p.add(0) && c <= *p.add(1) {
            return 1;
        }
        p = ucd_bsearch(c, ucd_alpha1.as_ptr(), ucd_alpha1.len() as c_int, 1);
        if !p.is_null() && c == *p.add(0) {
            return 1;
        }
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_tolowerrune_full(c: Rune) -> *const Rune {
    unsafe {
        let p = ucd_bsearch(
            c,
            ucd_tolower_full.as_ptr(),
            ucd_tolower_full.len() as c_int / 4,
            4,
        );
        if !p.is_null() && c == *p.add(0) {
            return p.add(1);
        }
        core::ptr::null()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C-unwind" fn jsU_toupperrune_full(c: Rune) -> *const Rune {
    unsafe {
        let p = ucd_bsearch(
            c,
            ucd_toupper_full.as_ptr(),
            ucd_toupper_full.len() as c_int / 5,
            5,
        );
        if !p.is_null() && c == *p.add(0) {
            return p.add(1);
        }
        core::ptr::null()
    }
}
