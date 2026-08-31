//! Translation of `src/strconv.c` (with `DTOA_ENABLED == 1`).

use crate::dtoa::dtoa_r;
use crate::types::*;
use core::ffi::{c_char, c_int, c_void};

fn get_decimal_point() -> c_char {
    let mut buf = [0 as c_char; 3];
    unsafe {
        /* sprintf(buf, "%#.0f", 1.0) -- "1." in the current locale */
        snprintf(buf.as_mut_ptr(), 3, b"%#.0f\0".as_ptr() as *const c_char, 1.0f64);
    }
    buf[1]
}

unsafe fn to_locale(strbuffer: *mut StrbufferT) {
    let point: c_char;

    point = get_decimal_point();
    if point == b'.' as c_char {
        /* No conversion needed */
        return;
    }

    /* strchr(strbuffer->value, '.') */
    let mut pos = (*strbuffer).value;
    while *pos != 0 {
        if *pos == b'.' as c_char {
            *pos = point;
            return;
        }
        pos = pos.add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(strbuffer: *mut StrbufferT, out: *mut f64) -> c_int {
    let value: f64;
    let mut end: *mut c_char = core::ptr::null_mut();

    to_locale(strbuffer);

    set_errno(0);
    value = strtod((*strbuffer).value, &mut end);
    /* assert(end == strbuffer->value + strbuffer->length); */

    if (value == f64::INFINITY || value == f64::NEG_INFINITY) && errno() == ERANGE {
        /* Overflow */
        return -1;
    }

    *out = value;
    0
}

/// `%d` of an `int`, as produced by `sprintf(p, "%d", exp)`.
unsafe fn write_dec_int(p: *mut c_char, v: c_int) -> c_int {
    let mut tmp = [0u8; 16];
    let neg = v < 0;
    let mut uv = if neg { (v as i64).unsigned_abs() } else { v as u64 };
    let mut i = tmp.len();
    if uv == 0 {
        i -= 1;
        tmp[i] = b'0';
    }
    while uv > 0 {
        i -= 1;
        tmp[i] = b'0' + (uv % 10) as u8;
        uv /= 10;
    }
    let mut n: usize = 0;
    if neg {
        *p = b'-' as c_char;
        n += 1;
    }
    for &c in &tmp[i..] {
        *p.add(n) = c as c_char;
        n += 1;
    }
    n as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_dtostr(
    buffer: *mut c_char,
    size: usize,
    value: f64,
    precision: c_int,
) -> c_int {
    /* adapted from `format_float_short()` in CPython's Python/pystrtod.c */
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

    digits_len = digits_end.offset_from(digits.as_mut_ptr()) as c_int;
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

    if ((3 + (vdigits_end - vdigits_start) + (if use_exp != 0 { 5 } else { 0 })) as isize as usize)
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
        memset(
            p as *mut c_void,
            b'0' as c_int,
            (decpt - vdigits_start) as usize,
        );
        p = p.offset((decpt - vdigits_start) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        memset(p as *mut c_void, b'0' as c_int, (0 - decpt) as usize);
        p = p.offset((0 - decpt) as isize);
    } else {
        memset(
            p as *mut c_void,
            b'0' as c_int,
            (0 - vdigits_start) as usize,
        );
        p = p.offset((0 - vdigits_start) as isize);
    }

    /* Digits, with included decimal point */
    if 0 < decpt && decpt <= digits_len {
        memcpy(
            p as *mut c_void,
            digits.as_ptr() as *const c_void,
            decpt as usize,
        );
        p = p.offset(decpt as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        memcpy(
            p as *mut c_void,
            digits.as_ptr().offset(decpt as isize) as *const c_void,
            (digits_len - decpt) as usize,
        );
        p = p.offset((digits_len - decpt) as isize);
    } else {
        memcpy(
            p as *mut c_void,
            digits.as_ptr() as *const c_void,
            digits_len as usize,
        );
        p = p.offset(digits_len as isize);
    }

    /* And zeros on the right */
    if digits_len < decpt {
        memset(
            p as *mut c_void,
            b'0' as c_int,
            (decpt - digits_len) as usize,
        );
        p = p.offset((decpt - digits_len) as isize);
        *p = b'.' as c_char;
        p = p.add(1);
        memset(
            p as *mut c_void,
            b'0' as c_int,
            (vdigits_end - decpt) as usize,
        );
        p = p.offset((vdigits_end - decpt) as isize);
    } else {
        memset(
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
        exp_len = write_dec_int(p, exp);
        p = p.offset(exp_len as isize);
    }
    *p = 0;

    p.offset_from(buffer) as c_int
}
