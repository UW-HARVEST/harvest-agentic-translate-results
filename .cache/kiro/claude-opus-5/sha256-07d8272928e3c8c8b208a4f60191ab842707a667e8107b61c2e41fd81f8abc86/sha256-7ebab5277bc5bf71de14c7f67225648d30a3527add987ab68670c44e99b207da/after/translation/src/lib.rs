//! Rust translation of `c_src/src/lib.c` (cJSON's `parse_number`).
//!
//! The goal is byte-identical behaviour with the original C, so the numeric
//! conversion is delegated to the platform `strtod` (locale-sensitive, handles
//! hex floats, overflow to +/-HUGE_VAL, etc.) and the temporary buffer is
//! allocated with `malloc`/`free` exactly like the C version - including the
//! "allocation failure returns false" path and the pointer comparison used to
//! detect a parse error.

#![allow(non_camel_case_types)]

use std::ffi::{c_char, c_double, c_int, c_uchar, c_void};

/// `typedef int cJSON_bool;`
pub type cJSON_bool = c_int;

const TRUE: cJSON_bool = 1;
const FALSE: cJSON_bool = 0;

/// `#define INT_MAX __INT_MAX__`
const INT_MAX: c_int = c_int::MAX;
/// `#define INT_MIN (-__INT_MAX__ - 1)`
const INT_MIN: c_int = -c_int::MAX - 1;

/// `#define cJSON_Number (1 << 3)`
const CJSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
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
    /// The item's number, if type == cJSON_Number
    pub valuedouble: c_double,
}

unsafe extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
}

/// `#define can_access_at_index(buffer, index)`
///
/// The C macro also guards against a NULL `buffer`; by the time it is used the
/// caller has already returned early for that case, but the check is kept for
/// fidelity.
#[inline]
fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }
    let buffer = unsafe { &*buffer };
    // Matches the C `(offset + index) < length`, wrapping included.
    buffer.offset.wrapping_add(index) < buffer.length
}

/// `#define buffer_at_offset(buffer)`
#[inline]
fn buffer_at_offset(buffer: &parse_buffer) -> *const c_uchar {
    unsafe { buffer.content.add(buffer.offset) }
}

/// Parse the input text to generate a number, and populate the result into item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: c_double;
    let mut after_end: *mut c_char = std::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = FALSE;

    if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {
        return FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    while can_access_at_index(input_buffer, i) {
        let c = unsafe { *buffer_at_offset(&*input_buffer).add(i) };
        match c {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
                number_string_length += 1;
            }

            b'.' => {
                number_string_length += 1;
                has_decimal_point = TRUE;
            }

            _ => break, // goto loop_end
        }
        i += 1;
    }
    // loop_end:

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = unsafe { malloc(number_string_length + 1) } as *mut c_uchar;
    if number_c_string.is_null() {
        return FALSE; /* allocation failure */
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            buffer_at_offset(&*input_buffer),
            number_c_string,
            number_string_length,
        );
        *number_c_string.add(number_string_length) = b'\0';
    }

    let scratch: &mut [c_uchar] =
        unsafe { std::slice::from_raw_parts_mut(number_c_string, number_string_length) };

    if has_decimal_point != FALSE {
        i = 0;
        while i < number_string_length {
            if scratch[i] == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                scratch[i] = decimal_point;
            }
            i += 1;
        }
    }

    number = unsafe { strtod(number_c_string as *const c_char, &mut after_end) };
    if number_c_string as *const c_char == after_end as *const c_char {
        /* free the temporary buffer */
        unsafe { free(number_c_string as *mut c_void) };
        return FALSE; /* parse_error */
    }

    let item_ref = unsafe { &mut *item };

    item_ref.valuedouble = number;

    /* use saturation in case of overflow */
    if number >= INT_MAX as c_double {
        item_ref.valueint = INT_MAX;
    } else if number <= INT_MIN as c_double {
        item_ref.valueint = INT_MIN;
    } else {
        item_ref.valueint = number as c_int;
    }

    item_ref.type_ = CJSON_NUMBER;

    let consumed = (after_end as usize).wrapping_sub(number_c_string as usize);
    unsafe {
        (*input_buffer).offset = (*input_buffer).offset.wrapping_add(consumed);
    }
    /* free the temporary buffer */
    unsafe { free(number_c_string as *mut c_void) };
    TRUE
}
