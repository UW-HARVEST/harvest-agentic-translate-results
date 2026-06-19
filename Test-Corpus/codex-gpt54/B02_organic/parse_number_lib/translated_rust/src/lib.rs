#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_uchar, c_void};

pub type cJSON_bool = c_int;

const TRUE: cJSON_bool = 1;
const FALSE: cJSON_bool = 0;

const INT_MIN_VALUE: c_int = c_int::MIN;
const INT_MAX_VALUE: c_int = c_int::MAX;
const CJSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
pub struct cJSON {
    pub r#type: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && (*buffer).offset.wrapping_add(index) < (*buffer).length
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
    let mut after_end: *mut c_char = std::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: usize = 0;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE;
    }

    while can_access_at_index(input_buffer, i) {
        match *buffer_at_offset(input_buffer).add(i) {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = TRUE;
            }
            _ => break,
        }

        i += 1;
    }

    number_c_string = malloc(number_string_length.wrapping_add(1)) as *mut c_uchar;
    if number_c_string.is_null() {
        return FALSE;
    }

    std::ptr::copy_nonoverlapping(
        buffer_at_offset(input_buffer),
        number_c_string,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point == TRUE {
        i = 0;
        while i < number_string_length {
            if *number_c_string.add(i) == b'.' {
                *number_c_string.add(i) = decimal_point;
            }
            i += 1;
        }
    }

    let number = strtod(number_c_string.cast::<c_char>(), &mut after_end);
    if std::ptr::eq(number_c_string.cast::<c_char>(), after_end) {
        free(number_c_string.cast::<c_void>());
        return FALSE;
    }

    (*item).valuedouble = number;

    if number >= INT_MAX_VALUE as c_double {
        (*item).valueint = INT_MAX_VALUE;
    } else if number <= INT_MIN_VALUE as c_double {
        (*item).valueint = INT_MIN_VALUE;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).r#type = CJSON_NUMBER;
    (*input_buffer).offset += after_end.offset_from(number_c_string.cast::<c_char>()) as usize;

    free(number_c_string.cast::<c_void>());
    TRUE
}
