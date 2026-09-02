//! Translation of `src/strconv.c` (DTOA_ENABLED == 1 variant).

use crate::cffi;
use crate::dtoa::dtoa_r;
use crate::strbuffer::strbuffer_t;
use core::ffi::{c_char, c_int};

/*
  - This code assumes that the decimal separator is exactly one
    character.

  - If setlocale() is called by another thread between the call to
    get_decimal_point() and the call to sprintf() or strtod(), the
    result may be wrong. setlocale() is not thread-safe and should
    not be used this way. Multi-threaded programs should use
    uselocale() instead.
*/
unsafe fn get_decimal_point() -> c_char {
    unsafe {
        let mut buf = [0 as c_char; 3];
        cffi::sprintf(buf.as_mut_ptr(), c"%#.0f".as_ptr(), 1.0f64); // "1." in the current locale
        buf[1]
    }
}

unsafe fn to_locale(strbuffer: *mut strbuffer_t) {
    unsafe {
        let point = get_decimal_point();
        if point == b'.' as c_char {
            /* No conversion needed */
            return;
        }

        let pos = cffi::c_strchr((*strbuffer).value, b'.');
        if !pos.is_null() {
            *(pos as *mut c_char) = point;
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(strbuffer: *mut strbuffer_t, out: *mut f64) -> c_int {
    unsafe {
        to_locale(strbuffer);

        cffi::set_errno(0);
        let mut end: *mut c_char = core::ptr::null_mut();
        let value = cffi::strtod((*strbuffer).value, &mut end);
        /* assert(end == strbuffer->value + strbuffer->length); */

        if (value == f64::INFINITY || value == f64::NEG_INFINITY)
            && cffi::errno() == cffi::ERANGE
        {
            /* Overflow */
            return -1;
        }

        *out = value;
        0
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_dtostr(
    buffer: *mut c_char,
    size: usize,
    value: f64,
    precision: c_int,
) -> c_int {
    unsafe {
        /* adapted from `format_float_short()` in CPython's pystrtod.c */
        let mut digits = [0 as c_char; 25];
        let mut digits_end: *mut c_char = core::ptr::null_mut();
        let mode = if precision == 0 { 0 } else { 2 };
        let mut decpt: c_int = 0;
        let mut sign: c_int = 0;
        let mut exp: c_int = 0;
        let mut use_exp: c_int = 0;

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

        let digits_len = digits_end.offset_from(digits.as_ptr()) as c_int;
        if decpt <= -4 || decpt > 16 {
            use_exp = 1;
            exp = decpt - 1;
            decpt = 1;
        }

        let vdigits_start = if decpt <= 0 { decpt - 1 } else { 0 };
        let mut vdigits_end = digits_len;
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

        if (3 + (vdigits_end - vdigits_start) + (if use_exp != 0 { 5 } else { 0 })) as usize > size {
            /* buffer is too short */
            return -1;
        }

        let mut p = buffer;
        if sign == 1 {
            *p = b'-' as c_char;
            p = p.add(1);
        }

        /* note that exactly one of the three 'if' conditions is true,
        so we include exactly one decimal point */
        /* Zero padding on left of digit string */
        if decpt <= 0 {
            let n = (decpt - vdigits_start) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
            *p = b'.' as c_char;
            p = p.add(1);
            let n = (0 - decpt) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
        } else {
            let n = (0 - vdigits_start) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
        }

        /* Digits, with included decimal point */
        if 0 < decpt && decpt <= digits_len {
            let n = decpt as usize;
            core::ptr::copy_nonoverlapping(digits.as_ptr() as *const u8, p as *mut u8, n);
            p = p.add(n);
            *p = b'.' as c_char;
            p = p.add(1);
            let n = (digits_len - decpt) as usize;
            core::ptr::copy_nonoverlapping(
                (digits.as_ptr() as *const u8).add(decpt as usize),
                p as *mut u8,
                n,
            );
            p = p.add(n);
        } else {
            let n = digits_len as usize;
            core::ptr::copy_nonoverlapping(digits.as_ptr() as *const u8, p as *mut u8, n);
            p = p.add(n);
        }

        /* And zeros on the right */
        if digits_len < decpt {
            let n = (decpt - digits_len) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
            *p = b'.' as c_char;
            p = p.add(1);
            let n = (vdigits_end - decpt) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
        } else {
            let n = (vdigits_end - digits_len) as usize;
            core::ptr::write_bytes(p as *mut u8, b'0', n);
            p = p.add(n);
        }

        if *p.sub(1) == b'.' as c_char {
            p = p.sub(1);
        }

        if use_exp != 0 {
            *p = b'e' as c_char;
            p = p.add(1);
            let exp_len = cffi::sprintf(p, c"%d".as_ptr(), exp);
            p = p.add(exp_len as usize);
        }
        *p = 0;

        p.offset_from(buffer) as c_int
    }
}
