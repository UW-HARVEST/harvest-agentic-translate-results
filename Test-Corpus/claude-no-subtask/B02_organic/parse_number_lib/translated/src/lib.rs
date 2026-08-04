#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_double, c_int, c_uchar};

pub type cJSON_bool = c_int;
pub type size_t = usize;

const cJSON_Number: c_int = 1 << 3;

const INT_MAX_C: c_int = c_int::MAX;
const INT_MIN_C: c_int = c_int::MIN;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: size_t,
    pub offset: size_t,
    pub depth: size_t,
}

#[repr(C)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

extern "C" {
    fn malloc(size: size_t) -> *mut c_uchar;
    fn free(ptr: *mut c_uchar);
    fn memcpy(dst: *mut c_uchar, src: *const c_uchar, n: size_t) -> *mut c_uchar;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: size_t) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: c_double;
    let mut after_end: *mut c_uchar = std::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: size_t;
    let mut number_string_length: size_t = 0;
    let mut has_decimal_point: cJSON_bool = 0;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    'loop_end: loop {
        if !can_access_at_index(input_buffer, i) {
            break 'loop_end;
        }
        let ch = *buffer_at_offset(input_buffer).add(i);
        match ch {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
            | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = 1;
            }
            _ => {
                break 'loop_end;
            }
        }
        i += 1;
    }

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = malloc(number_string_length + 1);
    if number_c_string.is_null() {
        return 0; /* allocation failure */
    }

    memcpy(number_c_string, buffer_at_offset(input_buffer), number_string_length);
    *number_c_string.add(number_string_length) = b'\0';

    if has_decimal_point != 0 {
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
        return 0; /* parse_error */
    }

    (*item).valuedouble = number;

    /* use saturation in case of overflow */
    if number >= INT_MAX_C as c_double {
        (*item).valueint = INT_MAX_C;
    } else if number <= INT_MIN_C as c_double {
        (*item).valueint = INT_MIN_C;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = cJSON_Number;

    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    /* free the temporary buffer */
    free(number_c_string);
    1
}
