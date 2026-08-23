//! Translation of c_src/src/strconv.c  (DTOA_ENABLED == 1)
#![allow(dead_code)]

use crate::libc;
use crate::strbuffer::strbuffer_t;
use std::ffi::{c_char, c_int, c_void};

/*
  - This code assumes that the decimal separator is exactly one character.

  - If setlocale() is called by another thread between the call to
    get_decimal_point() and the call to sprintf() or strtod(), the
    result may be wrong.
*/
unsafe fn get_decimal_point() -> c_char {
    let mut buf: [c_char; 3] = [0; 3];
    libc::sprintf(
        buf.as_mut_ptr(),
        b"%#.0f\0".as_ptr() as *const c_char,
        1.0f64,
    ); // "1." in the current locale
    buf[1]
}

unsafe fn to_locale(strbuffer: *mut strbuffer_t) {
    let point: c_char;
    let pos: *mut c_char;

    point = get_decimal_point();
    if point == b'.' as c_char {
        /* No conversion needed */
        return;
    }

    pos = libc::strchr((*strbuffer).value, '.' as c_int);
    if !pos.is_null() {
        *pos = point;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(strbuffer: *mut strbuffer_t, out: *mut f64) -> c_int {
    let value: f64;
    let mut end: *mut c_char = std::ptr::null_mut();

    to_locale(strbuffer);

    libc::set_errno(0);
    value = libc::strtod((*strbuffer).value, &mut end);

    if (value == libc::HUGE_VAL || value == -libc::HUGE_VAL) && libc::errno() == libc::ERANGE {
        /* Overflow */
        return -1;
    }

    *out = value;
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_dtostr(
    buffer: *mut c_char,
    size: usize,
    value: f64,
    precision: c_int,
) -> c_int {
    /* adapted from `format_float_short()` in CPython's pystrtod.c */
    let mut digits: [c_char; 25] = [0; 25];
    let mut digits_end: *mut c_char = std::ptr::null_mut();
    let mode: c_int = if precision == 0 { 0 } else { 2 };
    let mut decpt: c_int = 0;
    let mut sign: c_int = 0;
    let exp_len: c_int;
    let mut exp: c_int = 0;
    let mut use_exp: c_int = 0;
    let digits_len: c_int;
    let vdigits_start: c_int;
    let mut vdigits_end: c_int;
    let mut p: *mut c_char;

    if crate::dtoa_r::dtoa_r(
        value,
        mode,
        precision,
        &mut decpt,
        &mut sign,
        &mut digits_end,
        digits.as_mut_ptr(),
        25,
    )
    .is_null()
    {
        // digits is too short => should not happen
        return -1;
    }

    digits_len = digits_end.offset_from(digits.as_ptr()) as c_int;
    if decpt <= -4 || decpt > 16 {
        use_exp = 1;
        exp = decpt - 1;
        decpt = 1;
    }

    vdigits_start = if decpt <= 0 { decpt - 1 } else { 0 };
    vdigits_end = digits_len;
    if use_exp == 0 {
        /* decpt + 1 to add ".0" if value is an integer */
        vdigits_end = if vdigits_end > decpt {
            vdigits_end
        } else {
            decpt + 1
        };
    } else {
        vdigits_end = if vdigits_end > decpt {
            vdigits_end
        } else {
            decpt
        };
    }

    if
    /* sign, decimal point and trailing 0 byte */
    ((3
                 /* total digit count (including zero padding on both sides) */
                 + (vdigits_end - vdigits_start)
                 /* exponent "e+100", max 3 numerical digits */
                 + (if use_exp != 0 { 5 } else { 0 })) as usize)
        > size
    {
        /* buffer is too short */
        return -1;
    }

    p = buffer;
    if sign == 1 {
        *p = b'-' as c_char;
        p = p.add(1);
    }

    /* note that exactly one of the three 'if' conditions is true,
    so we include exactly one decimal point */
    /* Zero padding on left of digit string */
    if decpt <= 0 {
        libc::memset(
            p as *mut c_void,
            b'0' as c_int,
            (decpt - vdigits_start) as usize,
        );
        p = p.offset((decpt - vdigits_start) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        libc::memset(p as *mut c_void, b'0' as c_int, (0 - decpt) as usize);
        p = p.offset((0 - decpt) as isize);
    } else {
        libc::memset(
            p as *mut c_void,
            b'0' as c_int,
            (0 - vdigits_start) as usize,
        );
        p = p.offset((0 - vdigits_start) as isize);
    }

    /* Digits, with included decimal point */
    if 0 < decpt && decpt <= digits_len {
        libc::strncpy(p, digits.as_ptr(), (decpt - 0) as usize);
        p = p.offset((decpt - 0) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        libc::strncpy(
            p,
            digits.as_ptr().offset(decpt as isize),
            (digits_len - decpt) as usize,
        );
        p = p.offset((digits_len - decpt) as isize);
    } else {
        libc::strncpy(p, digits.as_ptr(), digits_len as usize);
        p = p.offset(digits_len as isize);
    }

    /* And zeros on the right */
    if digits_len < decpt {
        libc::memset(
            p as *mut c_void,
            b'0' as c_int,
            (decpt - digits_len) as usize,
        );
        p = p.offset((decpt - digits_len) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        libc::memset(
            p as *mut c_void,
            b'0' as c_int,
            (vdigits_end - decpt) as usize,
        );
        p = p.offset((vdigits_end - decpt) as isize);
    } else {
        libc::memset(
            p as *mut c_void,
            b'0' as c_int,
            (vdigits_end - digits_len) as usize,
        );
        p = p.offset((vdigits_end - digits_len) as isize);
    }

    if *p.sub(1) == b'.' as c_char {
        p = p.sub(1);
    }

    if use_exp != 0 {
        *p = b'e' as c_char;
        p = p.add(1);
        exp_len = libc::sprintf(p, b"%d\0".as_ptr() as *const c_char, exp);
        p = p.offset(exp_len as isize);
    }
    *p = 0;

    p.offset_from(buffer) as c_int
}
