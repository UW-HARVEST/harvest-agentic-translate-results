//! Translation of `src/strconv.c` (with `DTOA_ENABLED == 1`).

use core::ffi::{c_char, c_double, c_int, c_void};

use crate::ffi;
use crate::strbuffer::strbuffer_t;

/*
  - This code assumes that the decimal separator is exactly one character.

  - If setlocale() is called by another thread between the call to
    get_decimal_point() and the call to sprintf() or strtod(), the result may be
    wrong. setlocale() is not thread-safe and should not be used this way.
    Multi-threaded programs should use uselocale() instead.
*/
unsafe fn get_decimal_point() -> c_char {
    let mut buf = [0 as c_char; 3];
    ffi::sprintf(
        buf.as_mut_ptr(),
        b"%#.0f\0".as_ptr() as *const c_char,
        1.0f64,
    ); // "1." in the current locale
    buf[1]
}

unsafe fn to_locale(strbuffer: *mut strbuffer_t) {
    let point = get_decimal_point();
    if point == b'.' as c_char {
        /* No conversion needed */
        return;
    }

    let pos = ffi::strchr((*strbuffer).value, '.' as c_int);
    if !pos.is_null() {
        *pos = point;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(strbuffer: *mut strbuffer_t, out: *mut c_double) -> c_int {
    let mut end: *mut c_char = core::ptr::null_mut();

    to_locale(strbuffer);

    ffi::set_errno(0);
    let value = ffi::strtod((*strbuffer).value, &mut end);
    debug_assert!(end == (*strbuffer).value.add((*strbuffer).length));

    if (value == ffi::HUGE_VAL || value == -ffi::HUGE_VAL) && ffi::errno() == ffi::ERANGE {
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
    value: c_double,
    precision: c_int,
) -> c_int {
    /* adapted from `format_float_short()` in
     * https://github.com/python/cpython/blob/2cf18a44303b6d84faa8ecffaecc427b53ae121e/Python/pystrtod.c#L969
     */
    let mut digits = [0 as c_char; 25];
    let mut digits_end: *mut c_char = core::ptr::null_mut();
    let mode = if precision == 0 { 0 } else { 2 };
    let mut decpt: c_int = 0;
    let mut sign: c_int = 0;
    let exp_len: c_int;
    let mut exp: c_int = 0;
    let mut use_exp: c_int = 0;
    let digits_len: c_int;
    let vdigits_start: c_int;
    let mut vdigits_end: c_int;
    let mut p: *mut c_char;

    if crate::dtoa::dtoa_r(
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

    digits_len = (digits_end as isize - digits.as_mut_ptr() as isize) as c_int;
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
    (3
                 /* total digit count (including zero padding on both sides) */
                 + (vdigits_end - vdigits_start)
                 /* exponent "e+100", max 3 numerical digits */
                 + (if use_exp != 0 { 5 } else { 0 })) as usize
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
        ffi::memset(p as *mut c_void, b'0' as c_int, (decpt - vdigits_start) as usize);
        p = p.offset((decpt - vdigits_start) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        ffi::memset(p as *mut c_void, b'0' as c_int, (0 - decpt) as usize);
        p = p.offset((0 - decpt) as isize);
    } else {
        ffi::memset(p as *mut c_void, b'0' as c_int, (0 - vdigits_start) as usize);
        p = p.offset((0 - vdigits_start) as isize);
    }

    /* Digits, with included decimal point */
    if 0 < decpt && decpt <= digits_len {
        ffi::strncpy(p, digits.as_ptr(), (decpt - 0) as usize);
        p = p.offset((decpt - 0) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        ffi::strncpy(
            p,
            digits.as_ptr().offset(decpt as isize),
            (digits_len - decpt) as usize,
        );
        p = p.offset((digits_len - decpt) as isize);
    } else {
        ffi::strncpy(p, digits.as_ptr(), digits_len as usize);
        p = p.offset(digits_len as isize);
    }

    /* And zeros on the right */
    if digits_len < decpt {
        ffi::memset(p as *mut c_void, b'0' as c_int, (decpt - digits_len) as usize);
        p = p.offset((decpt - digits_len) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        ffi::memset(p as *mut c_void, b'0' as c_int, (vdigits_end - decpt) as usize);
        p = p.offset((vdigits_end - decpt) as isize);
    } else {
        ffi::memset(
            p as *mut c_void,
            b'0' as c_int,
            (vdigits_end - digits_len) as usize,
        );
        p = p.offset((vdigits_end - digits_len) as isize);
    }

    if *p.offset(-1) == b'.' as c_char {
        p = p.offset(-1);
    }

    if use_exp != 0 {
        *p = b'e' as c_char;
        p = p.add(1);
        exp_len = ffi::sprintf(p, b"%d\0".as_ptr() as *const c_char, exp);
        p = p.offset(exp_len as isize);
    }
    *p = 0;

    (p as isize - buffer as isize) as c_int
}
