// Translation of c_src/src/lib.c (cJSON parse_number) to Rust.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]

use std::os::raw::{c_char, c_double, c_int};

pub type cJSON_bool = c_int;

pub const cJSON_true: cJSON_bool = 1;
pub const cJSON_false: cJSON_bool = 0;

pub const cJSON_Number: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    /// How deeply nested (in arrays/objects) is the input at the current offset.
    pub depth: usize,
}

#[repr(C)]
pub struct cJSON {
    /// The type of the item, as above.
    pub type_: c_int,
    /// writing to valueint is DEPRECATED, use cJSON_SetNumberValue instead
    pub valueint: c_int,
    /// The item's number, if type==cJSON_Number
    pub valuedouble: c_double,
}

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn malloc(size: usize) -> *mut std::ffi::c_void;
    fn free(ptr: *mut std::ffi::c_void);
    fn memcpy(
        dest: *mut std::ffi::c_void,
        src: *const std::ffi::c_void,
        n: usize,
    ) -> *mut std::ffi::c_void;
}

/// check if the buffer can be accessed at the given index (starting with 0)
#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

/// get a pointer to the buffer at the position
#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const u8 {
    (*buffer).content.add((*buffer).offset)
}

/// Parse the input text to generate a number, and populate the result into item.
///
/// # Safety
///
/// `item` and `input_buffer` must be valid pointers if they are non-null.
/// `input_buffer->content` must point to at least `input_buffer->length` bytes
/// when it is non-null.
#[no_mangle]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: f64;
    let mut after_end: *mut u8 = std::ptr::null_mut();
    let number_c_string: *mut u8;
    let decimal_point: u8 = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = cJSON_false;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return cJSON_false;
    }

    // copy the number into a temporary buffer and replace '.' with the decimal point
    // of the current locale (for strtod)
    // This also takes care of '\0' not necessarily being available for marking the end of the input
    i = 0;
    'loop_end: loop {
        if !can_access_at_index(input_buffer, i) {
            break 'loop_end;
        }
        let c = *buffer_at_offset(input_buffer).add(i);
        match c {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = cJSON_true;
            }
            _ => {
                break 'loop_end;
            }
        }
        i += 1;
    }

    // malloc for temporary buffer, add 1 for '\0'
    number_c_string = malloc(number_string_length + 1) as *mut u8;
    if number_c_string.is_null() {
        return cJSON_false; // allocation failure
    }

    memcpy(
        number_c_string as *mut std::ffi::c_void,
        buffer_at_offset(input_buffer) as *const std::ffi::c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = b'\0';

    if has_decimal_point != cJSON_false {
        i = 0;
        while i < number_string_length {
            if *number_c_string.add(i) == b'.' {
                // replace '.' with the decimal point of the current locale (for strtod)
                *number_c_string.add(i) = decimal_point;
            }
            i += 1;
        }
    }

    number = strtod(
        number_c_string as *const c_char,
        &mut after_end as *mut *mut u8 as *mut *mut c_char,
    );
    if number_c_string == after_end {
        // free the temporary buffer
        free(number_c_string as *mut std::ffi::c_void);
        return cJSON_false; // parse_error
    }

    (*item).valuedouble = number;

    // use saturation in case of overflow
    if number >= c_int::MAX as f64 {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as f64 {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = cJSON_Number;

    (*input_buffer).offset += after_end as usize - number_c_string as usize;
    // free the temporary buffer
    free(number_c_string as *mut std::ffi::c_void);

    // Suppress unused assignment warning for `number`
    let _ = number;

    cJSON_true
}
