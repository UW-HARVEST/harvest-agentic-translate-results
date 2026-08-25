#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_uchar, c_void};
use std::ptr;

pub type cJSON_bool = c_int;

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
    fn free(pointer: *mut c_void);
    fn strtod(input: *const c_char, after_end: *mut *mut c_char) -> c_double;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    if input_buffer.is_null() {
        return 0;
    }

    let content = unsafe { (*input_buffer).content };
    if content.is_null() {
        return 0;
    }

    let mut number_string_length = 0usize;
    let mut has_decimal_point = false;
    let mut i = 0usize;

    while unsafe { (*input_buffer).offset.wrapping_add(i) < (*input_buffer).length } {
        let offset = unsafe { (*input_buffer).offset };
        let byte = unsafe { *content.wrapping_add(offset).wrapping_add(i) };
        match byte {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {
                number_string_length = number_string_length.wrapping_add(1);
            }
            b'.' => {
                number_string_length = number_string_length.wrapping_add(1);
                has_decimal_point = true;
            }
            _ => break,
        }
        i = i.wrapping_add(1);
    }

    let number_c_string = unsafe { malloc(number_string_length.wrapping_add(1)) }.cast::<c_uchar>();
    if number_c_string.is_null() {
        return 0;
    }

    let offset = unsafe { (*input_buffer).offset };
    unsafe {
        ptr::copy_nonoverlapping(
            content.wrapping_add(offset),
            number_c_string,
            number_string_length,
        );
        *number_c_string.wrapping_add(number_string_length) = 0;
    }

    if has_decimal_point {
        for index in 0..number_string_length {
            if unsafe { *number_c_string.wrapping_add(index) } == b'.' {
                unsafe {
                    *number_c_string.wrapping_add(index) = b'.';
                }
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = unsafe {
        strtod(
            number_c_string.cast::<c_char>(),
            ptr::addr_of_mut!(after_end),
        )
    };
    if number_c_string.cast::<c_char>() == after_end {
        unsafe {
            free(number_c_string.cast::<c_void>());
        }
        return 0;
    }

    unsafe {
        (*item).valuedouble = number;
        (*item).valueint = if number >= c_int::MAX as c_double {
            c_int::MAX
        } else if number <= c_int::MIN as c_double {
            c_int::MIN
        } else {
            number as c_int
        };
        (*item).r#type = CJSON_NUMBER;

        let consumed = after_end.offset_from(number_c_string.cast::<c_char>()) as usize;
        (*input_buffer).offset = (*input_buffer).offset.wrapping_add(consumed);
        free(number_c_string.cast::<c_void>());
    }

    1
}
