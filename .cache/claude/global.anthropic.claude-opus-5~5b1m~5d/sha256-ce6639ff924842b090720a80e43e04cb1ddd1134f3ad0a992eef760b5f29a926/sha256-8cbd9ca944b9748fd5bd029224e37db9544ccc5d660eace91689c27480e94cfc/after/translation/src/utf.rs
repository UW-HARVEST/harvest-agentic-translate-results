//! Translation of utf.c

use crate::jsi::*;
use crate::utfdata::*;

pub const UTFmax: c_int = 4;
pub const Runesync: c_int = 0x80;
pub const Runeself: c_int = 0x80;
pub const Runeerror: c_int = 0xFFFD;
pub const Runemax: c_int = 0x10FFFF;

const Bitx: c_int = 6;
const T1: c_int = 0x00;
const Tx: c_int = 0x80;
const T2: c_int = 0xC0;
const T3: c_int = 0xE0;
const T4: c_int = 0xF0;
const T5: c_int = 0xF8;
const Rune1: c_int = 0x7F;
const Rune2: c_int = 0x7FF;
const Rune3: c_int = 0xFFFF;
const Rune4: c_int = 0x1FFFFF;
const Maskx: c_int = 0x3F;
const Testx: c_int = 0xC0;
const Bad: c_int = Runeerror;

#[inline]
unsafe fn uc(s: *const c_char, i: usize) -> c_int {
    *(s.add(i) as *const u8) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_chartorune(rune: *mut Rune, str: *const c_char) -> c_int {
    let c: c_int;
    let c1: c_int;
    let c2: c_int;
    let c3: c_int;
    let l: c_int;

    /* overlong null character */
    if uc(str, 0) == 0xc0 && uc(str, 1) == 0x80 {
        *rune = 0;
        return 2;
    }

    /* one character sequence: 00000-0007F => T1 */
    c = uc(str, 0);
    if c < Tx {
        *rune = c;
        return 1;
    }

    /* two character sequence: 0080-07FF => T2 Tx */
    c1 = uc(str, 1) ^ Tx;
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

    /* three character sequence: 0800-FFFF => T3 Tx Tx */
    c2 = uc(str, 2) ^ Tx;
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

    /* four character sequence: 10000-10FFFF => T4 Tx Tx Tx */
    c3 = uc(str, 3) ^ Tx;
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

    /* bad decoding */
    *rune = Bad;
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_runetochar(str: *mut c_char, rune: *const Rune) -> c_int {
    let mut c: c_int = *rune;

    /* overlong null character */
    if c == 0 {
        *str.add(0) = 0xc0u8 as i8 as c_char;
        *str.add(1) = 0x80u8 as i8 as c_char;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_runelen(c: c_int) -> c_int {
    let rune: Rune = c;
    let mut str: [c_char; 10] = [0; 10];
    jsU_runetochar(str.as_mut_ptr(), &rune)
}

unsafe fn ucd_bsearch(c: Rune, t: *const Rune, n: c_int, ne: c_int) -> *const Rune {
    let mut t = t;
    let mut n = n;
    while n > 1 {
        let m = n / 2;
        let p = t.offset((m * ne) as isize);
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
pub unsafe extern "C" fn jsU_tolowerrune(c: Rune) -> Rune {
    let mut p = ucd_bsearch(
        c,
        ucd_tolower2.as_ptr(),
        (ucd_tolower2.len() / 3) as c_int,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return c + *p.offset(2);
    }
    p = ucd_bsearch(
        c,
        ucd_tolower1.as_ptr(),
        (ucd_tolower1.len() / 2) as c_int,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return c + *p.offset(1);
    }
    c
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_toupperrune(c: Rune) -> Rune {
    let mut p = ucd_bsearch(
        c,
        ucd_toupper2.as_ptr(),
        (ucd_toupper2.len() / 3) as c_int,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return c + *p.offset(2);
    }
    p = ucd_bsearch(
        c,
        ucd_toupper1.as_ptr(),
        (ucd_toupper1.len() / 2) as c_int,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return c + *p.offset(1);
    }
    c
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_islowerrune(c: Rune) -> c_int {
    let mut p = ucd_bsearch(
        c,
        ucd_toupper2.as_ptr(),
        (ucd_toupper2.len() / 3) as c_int,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return 1;
    }
    p = ucd_bsearch(
        c,
        ucd_toupper1.as_ptr(),
        (ucd_toupper1.len() / 2) as c_int,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_isupperrune(c: Rune) -> c_int {
    let mut p = ucd_bsearch(
        c,
        ucd_tolower2.as_ptr(),
        (ucd_tolower2.len() / 3) as c_int,
        3,
    );
    if !p.is_null() && c >= *p.offset(0) && c <= *p.offset(1) {
        return 1;
    }
    p = ucd_bsearch(
        c,
        ucd_tolower1.as_ptr(),
        (ucd_tolower1.len() / 2) as c_int,
        2,
    );
    if !p.is_null() && c == *p.offset(0) {
        return 1;
    }
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_isalpharune(c: Rune) -> c_int {
    let mut p = ucd_bsearch(c, ucd_alpha2.as_ptr(), (ucd_alpha2.len() / 2) as c_int, 2);
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
pub unsafe extern "C" fn jsU_tolowerrune_full(c: Rune) -> *const Rune {
    let p = ucd_bsearch(
        c,
        ucd_tolower_full.as_ptr(),
        (ucd_tolower_full.len() / 4) as c_int,
        4,
    );
    if !p.is_null() && c == *p.offset(0) {
        return p.offset(1);
    }
    null()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsU_toupperrune_full(c: Rune) -> *const Rune {
    let p = ucd_bsearch(
        c,
        ucd_toupper_full.as_ptr(),
        (ucd_toupper_full.len() / 5) as c_int,
        5,
    );
    if !p.is_null() && c == *p.offset(0) {
        return p.offset(1);
    }
    null()
}

/* Convenience aliases matching the C source's #define names. */
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
