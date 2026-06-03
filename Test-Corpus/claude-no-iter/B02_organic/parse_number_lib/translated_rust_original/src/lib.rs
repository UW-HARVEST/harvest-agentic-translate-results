#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::os::raw::{c_double, c_int, c_uchar};

pub type cJSON_bool = c_int;

const TRUE: cJSON_bool = 1;
const FALSE: cJSON_bool = 0;

const C_JSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

extern "C" {
    fn malloc(size: usize) -> *mut c_uchar;
    fn free(ptr: *mut c_uchar);
    fn memcpy(dest: *mut c_uchar, src: *const c_uchar, n: usize) -> *mut c_uchar;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

#[inline]
fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }
    unsafe { ((*buffer).offset + index) < (*buffer).length }
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

/// Parse the input text to generate a number, and populate the result into item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: c_double;
    let mut after_end: *mut c_uchar = std::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    'outer: while can_access_at_index(input_buffer, i) {
        let ch = *buffer_at_offset(input_buffer).add(i);
        match ch {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
            | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = TRUE;
            }
            _ => {
                break 'outer;
            }
        }
        i += 1;
    }

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = malloc(number_string_length + 1);
    if number_c_string.is_null() {
        return FALSE; /* allocation failure */
    }

    memcpy(number_c_string, buffer_at_offset(input_buffer), number_string_length);
    *number_c_string.add(number_string_length) = b'\0';

    if has_decimal_point != FALSE {
        i = 0;
        while i < number_string_length {
            if *number_c_string.add(i) == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                *number_c_string.add(i) = decimal_point;
            }
            i += 1;
        }
    }

    number = strtod(
        number_c_string as *const c_char,
        &mut after_end as *mut *mut c_uchar as *mut *mut c_char,
    );
    if number_c_string == after_end {
        /* free the temporary buffer */
        free(number_c_string);
        return FALSE; /* parse_error */
    }

    (*item).valuedouble = number;

    /* use saturation in case of overflow */
    if number >= c_int::MAX as c_double {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = C_JSON_NUMBER;

    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    /* free the temporary buffer */
    free(number_c_string);
    TRUE
}
