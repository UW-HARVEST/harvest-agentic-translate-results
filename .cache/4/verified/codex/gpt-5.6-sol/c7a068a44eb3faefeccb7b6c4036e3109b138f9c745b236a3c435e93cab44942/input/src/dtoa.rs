use crate::private::strbuffer_t;
use std::ffi::{CStr, c_char, c_int, c_void};
use std::ptr;

#[unsafe(no_mangle)]
pub static mut dtoa_divmax: c_int = 2;

fn formatted(value: f64, mode: c_int, digits: c_int) -> String {
    if !value.is_finite() {
        return if value.is_nan() {
            "NaN".to_owned()
        } else {
            "Infinity".to_owned()
        };
    }
    if matches!(mode, 2 | 4 | 6 | 8) {
        let precision = digits.max(1);
        let mut buffer = [0i8; 1024];
        unsafe {
            libc::snprintf(
                buffer.as_mut_ptr(),
                buffer.len(),
                c"%.*g".as_ptr(),
                precision,
                value.abs(),
            );
            CStr::from_ptr(buffer.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    } else if matches!(mode, 3 | 5 | 7 | 9) {
        let precision = digits.max(0);
        let mut buffer = [0i8; 1024];
        unsafe {
            libc::snprintf(
                buffer.as_mut_ptr(),
                buffer.len(),
                c"%.*f".as_ptr(),
                precision,
                value.abs(),
            );
            CStr::from_ptr(buffer.as_ptr())
                .to_string_lossy()
                .into_owned()
        }
    } else {
        value.abs().to_string()
    }
}

fn dtoa_parts(value: f64, mode: c_int, ndigits: c_int) -> (Vec<u8>, c_int, c_int) {
    let sign = value.is_sign_negative() as c_int;
    if matches!(mode, 3 | 5 | 7 | 9) && value != 0.0 && value.abs() < 0.5 * 10f64.powi(-ndigits) {
        return (Vec::new(), -ndigits, sign);
    }
    let text = formatted(value, mode, ndigits);
    if !value.is_finite() {
        return (text.into_bytes(), 9999, sign);
    }
    let (mantissa, exponent) = text
        .find(['e', 'E'])
        .map(|index| {
            (
                &text[..index],
                text[index + 1..].parse::<c_int>().unwrap_or(0),
            )
        })
        .unwrap_or((&text, 0));
    let decimal = mantissa.find('.').unwrap_or(mantissa.len()) as c_int + exponent;
    let mut digits: Vec<u8> = mantissa
        .bytes()
        .filter(|byte| *byte != b'.' && *byte != b'+' && *byte != b'-')
        .collect();
    let leading = digits.iter().take_while(|&&byte| byte == b'0').count();
    let mut decpt = decimal - leading as c_int;
    digits.drain(..leading);
    while digits.len() > 1 && digits.last() == Some(&b'0') {
        digits.pop();
    }
    if digits.is_empty() {
        digits.push(b'0');
        decpt = 1;
    }
    (digits, decpt, sign)
}

unsafe fn allocate_digits(length: usize) -> *mut c_char {
    let base = libc::malloc(length + 1 + std::mem::size_of::<usize>()).cast::<u8>();
    if base.is_null() {
        return ptr::null_mut();
    }
    ptr::write(base.cast::<usize>(), length);
    base.add(std::mem::size_of::<usize>()).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa_r(
    value: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
    buffer: *mut c_char,
    buffer_length: usize,
) -> *mut c_char {
    let (digits, point, negative) = dtoa_parts(value, mode, ndigits);
    if !decpt.is_null() {
        *decpt = point;
    }
    if !sign.is_null() {
        *sign = negative;
    }
    let output = if buffer.is_null() {
        allocate_digits(digits.len())
    } else if buffer_length <= digits.len() {
        ptr::null_mut()
    } else {
        buffer
    };
    if output.is_null() {
        if !rve.is_null() {
            *rve = output;
        }
        return output;
    }
    ptr::copy_nonoverlapping(digits.as_ptr(), output.cast(), digits.len());
    *output.add(digits.len()) = 0;
    if !rve.is_null() {
        *rve = output.add(digits.len());
    }
    output
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn dtoa(
    value: f64,
    mode: c_int,
    ndigits: c_int,
    decpt: *mut c_int,
    sign: *mut c_int,
    rve: *mut *mut c_char,
) -> *mut c_char {
    dtoa_r(value, mode, ndigits, decpt, sign, rve, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn freedtoa(value: *mut c_char) {
    if !value.is_null() {
        libc::free(value.cast::<u8>().sub(std::mem::size_of::<usize>()).cast());
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn strtod__unused(input: *const c_char, end: *mut *mut c_char) -> f64 {
    libc::strtod(input, end)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn gethex(
    input: *mut *const c_char,
    result: *mut c_void,
    _rounding: c_int,
    sign: c_int,
) {
    if input.is_null() || result.is_null() {
        return;
    }
    let mut end = ptr::null_mut();
    let mut value = libc::strtod(*input, &mut end);
    if sign != 0 {
        value = -value;
    }
    ptr::write(result.cast::<f64>(), value);
    *input = end;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn jsonp_strtod(buffer: *mut strbuffer_t, output: *mut f64) -> c_int {
    if buffer.is_null() || output.is_null() {
        return -1;
    }
    *libc::__errno_location() = 0;
    let value = libc::strtod((*buffer).value, ptr::null_mut());
    if value.is_infinite() && *libc::__errno_location() == libc::ERANGE {
        -1
    } else {
        *output = value;
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
    let (digits, mut decpt, sign) =
        dtoa_parts(value, if precision == 0 { 0 } else { 2 }, precision);
    let use_exponent = decpt <= -4 || decpt > 16;
    let exponent = decpt - 1;
    if use_exponent {
        decpt = 1;
    }
    let mut output = String::new();
    if sign != 0 {
        output.push('-');
    }
    if decpt <= 0 {
        output.push('0');
        output.push('.');
        for _ in 0..-decpt {
            output.push('0');
        }
        output.push_str(std::str::from_utf8_unchecked(&digits));
    } else if decpt as usize >= digits.len() {
        output.push_str(std::str::from_utf8_unchecked(&digits));
        for _ in digits.len()..decpt as usize {
            output.push('0');
        }
        if !use_exponent {
            output.push_str(".0");
        }
    } else {
        output.push_str(std::str::from_utf8_unchecked(&digits[..decpt as usize]));
        output.push('.');
        output.push_str(std::str::from_utf8_unchecked(&digits[decpt as usize..]));
    }
    if use_exponent {
        output.push('e');
        output.push_str(&exponent.to_string());
    }
    if output.len() + 1 > size || buffer.is_null() {
        return -1;
    }
    ptr::copy_nonoverlapping(output.as_ptr(), buffer.cast(), output.len());
    *buffer.add(output.len()) = 0;
    output.len() as c_int
}
