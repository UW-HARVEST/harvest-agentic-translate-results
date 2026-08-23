//! Translation of strconv.c (DTOA_ENABLED path).
use crate::types::*;
use core::ffi::{c_char, c_double, c_int, c_void};

extern "C" {
    fn strchr(s: *const c_char, c: c_int) -> *mut c_char;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn sprintf(s: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn strncpy(dst: *mut c_char, src: *const c_char, n: usize) -> *mut c_char;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn __errno_location() -> *mut c_int;
    // from dtoa.c
    fn dtoa_r(
        dd: f64,
        mode: c_int,
        ndigits: c_int,
        decpt: *mut c_int,
        sign: *mut c_int,
        rve: *mut *mut c_char,
        buf: *mut c_char,
        blen: usize,
    ) -> *mut c_char;
}

const HUGE_VAL: f64 = f64::INFINITY;
const ERANGE: c_int = 34;

unsafe fn get_decimal_point() -> c_char {
    let mut buf = [0 as c_char; 3];
    sprintf(buf.as_mut_ptr(), b"%#.0f\0".as_ptr() as *const c_char, 1.0f64); // "1." in current locale
    buf[1]
}

unsafe fn to_locale(strbuffer: *mut strbuffer_t) {
    let point = get_decimal_point();
    if point == b'.' as c_char {
        /* No conversion needed */
        return;
    }

    let pos = strchr((*strbuffer).value, b'.' as c_int);
    if !pos.is_null() {
        *pos = point;
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(strbuffer: *mut strbuffer_t, out: *mut f64) -> c_int {
    to_locale(strbuffer);

    *__errno_location() = 0;
    let mut end: *mut c_char = core::ptr::null_mut();
    let value = strtod((*strbuffer).value, &mut end);
    // assert(end == strbuffer->value + strbuffer->length);
    debug_assert!(end == (*strbuffer).value.add((*strbuffer).length));

    if (value == HUGE_VAL || value == -HUGE_VAL) && *__errno_location() == ERANGE {
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

    if dtoa_r(
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

    digits_len = (digits_end as isize - digits.as_ptr() as isize) as c_int;
    if decpt <= -4 || decpt > 16 {
        use_exp = 1;
        exp = decpt - 1;
        decpt = 1;
    }

    vdigits_start = if decpt <= 0 { decpt - 1 } else { 0 };
    vdigits_end = digits_len;
    if use_exp == 0 {
        /* decpt + 1 to add ".0" if value is an integer */
        vdigits_end = if vdigits_end > decpt { vdigits_end } else { decpt + 1 };
    } else {
        vdigits_end = if vdigits_end > decpt { vdigits_end } else { decpt };
    }

    if (3 + (vdigits_end - vdigits_start) + (if use_exp != 0 { 5 } else { 0 })) as usize > size {
        /* buffer is too short */
        return -1;
    }

    let mut p = buffer;
    if sign == 1 {
        *p = b'-' as c_char;
        p = p.add(1);
    }

    /* Zero padding on left of digit string */
    if decpt <= 0 {
        memset(p as *mut c_void, b'0' as c_int, (decpt - vdigits_start) as usize);
        p = p.offset((decpt - vdigits_start) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        memset(p as *mut c_void, b'0' as c_int, (0 - decpt) as usize);
        p = p.offset((0 - decpt) as isize);
    } else {
        memset(p as *mut c_void, b'0' as c_int, (0 - vdigits_start) as usize);
        p = p.offset((0 - vdigits_start) as isize);
    }

    /* Digits, with included decimal point */
    if 0 < decpt && decpt <= digits_len {
        strncpy(p, digits.as_ptr(), (decpt - 0) as usize);
        p = p.offset((decpt - 0) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        strncpy(p, digits.as_ptr().offset(decpt as isize), (digits_len - decpt) as usize);
        p = p.offset((digits_len - decpt) as isize);
    } else {
        strncpy(p, digits.as_ptr(), digits_len as usize);
        p = p.offset(digits_len as isize);
    }

    /* And zeros on the right */
    if digits_len < decpt {
        memset(p as *mut c_void, b'0' as c_int, (decpt - digits_len) as usize);
        p = p.offset((decpt - digits_len) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        memset(p as *mut c_void, b'0' as c_int, (vdigits_end - decpt) as usize);
        p = p.offset((vdigits_end - decpt) as isize);
    } else {
        memset(p as *mut c_void, b'0' as c_int, (vdigits_end - digits_len) as usize);
        p = p.offset((vdigits_end - digits_len) as isize);
    }

    if *p.offset(-1) == b'.' as c_char {
        p = p.offset(-1);
    }

    if use_exp != 0 {
        *p = b'e' as c_char;
        p = p.add(1);
        exp_len = sprintf(p, b"%d\0".as_ptr() as *const c_char, exp);
        p = p.offset(exp_len as isize);
    }
    *p = 0;

    (p as isize - buffer as isize) as c_int
}
