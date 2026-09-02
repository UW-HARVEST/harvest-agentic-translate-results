//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exporting
//! exactly one public symbol: `parse_number`. This crate reproduces that ABI
//! byte-for-byte, including the layout of the public `cJSON` / `parse_buffer`
//! structs declared in `include/lib.h`.
//!
//! Behavioural notes (deliberately preserved, quirks included):
//!  * The character scan accepts only `[0-9+-eE.]`; it stops at the first other
//!    byte, so no validation of the resulting numeric string is performed here.
//!  * `decimal_point` is hard-coded to `'.'` in the C source, so the
//!    "localise the decimal separator" loop is a no-op. It is kept for fidelity.
//!  * Number conversion is delegated to the platform `strtod(3)` so that
//!    rounding, overflow (`HUGE_VAL`), and end-pointer placement are identical
//!    to the C build.
//!  * `item` is dereferenced without a NULL check, exactly as the C does.
//!  * On temporary-allocation failure the function returns `false`.

#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]

use std::ffi::{c_char, c_double, c_int, c_uchar};

/* ------------------------------------------------------------------------- */
/* Public types (must match include/lib.h exactly)                           */
/* ------------------------------------------------------------------------- */

/// `typedef int cJSON_bool;`
pub type cJSON_bool = c_int;

/// `#define true ((cJSON_bool)1)`
const CJSON_TRUE: cJSON_bool = 1;
/// `#define false ((cJSON_bool)0)`
const CJSON_FALSE: cJSON_bool = 0;

/// `#define INT_MAX __INT_MAX__`
const INT_MAX: c_int = 2147483647;
/// `#define INT_MIN (-__INT_MAX__ - 1)`
const INT_MIN: c_int = -2147483647 - 1;

/// `#define cJSON_Number (1 << 3)`
const cJSON_Number: c_int = 1 << 3;

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

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

/* ------------------------------------------------------------------------- */
/* Internal helpers mirroring the C macros                                   */
/* ------------------------------------------------------------------------- */

/// `#define can_access_at_index(buffer, index) \
///     ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))`
///
/// The NULL test is handled by the caller (we hold a reference); the offset
/// addition uses C's wrapping `size_t` arithmetic.
#[inline]
fn can_access_at_index(buffer: &parse_buffer, index: usize) -> bool {
    buffer.offset.wrapping_add(index) < buffer.length
}

/// C's `(int)double` conversion as implemented on this target (truncation
/// toward zero; out-of-range/NaN follows the x86 `cvttsd2si` result of
/// `INT_MIN`). Callers only reach this for in-range finite values.
#[inline]
fn double_to_int_trunc(value: c_double) -> c_int {
    if value.is_nan() {
        return INT_MIN;
    }
    let truncated = value.trunc();
    if truncated >= INT_MAX as c_double + 1.0 || truncated < INT_MIN as c_double {
        return INT_MIN;
    }
    truncated as c_int
}

/* ------------------------------------------------------------------------- */
/* Public API                                                                */
/* ------------------------------------------------------------------------- */

/// Parse the input text to generate a number, and populate the result into item.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: c_double;
    let decimal_point: c_uchar = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = CJSON_FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return CJSON_FALSE;
    }

    let buffer: &mut parse_buffer = &mut *input_buffer;

    /* copy the number into a temporary buffer and replace '.' with the decimal
     * point of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking
     * the end of the input */
    i = 0;
    while can_access_at_index(buffer, i) {
        // buffer_at_offset(buffer)[i]
        let byte: c_uchar = *buffer.content.add(buffer.offset.wrapping_add(i));
        match byte {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = CJSON_TRUE;
            }
            _ => break, /* goto loop_end */
        }
        i += 1;
    }

    /* malloc for temporary buffer, add 1 for '\0' */
    let mut number_c_string: Vec<c_uchar> = Vec::new();
    if number_c_string
        .try_reserve_exact(number_string_length + 1)
        .is_err()
    {
        return CJSON_FALSE; /* allocation failure */
    }

    // memcpy(number_c_string, buffer_at_offset(input_buffer), number_string_length);
    number_c_string.extend_from_slice(std::slice::from_raw_parts(
        buffer.content.add(buffer.offset),
        number_string_length,
    ));
    // number_c_string[number_string_length] = '\0';
    number_c_string.push(0);

    if has_decimal_point != CJSON_FALSE {
        for slot in number_c_string[..number_string_length].iter_mut() {
            if *slot == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                *slot = decimal_point;
            }
        }
    }

    let start: *mut c_char = number_c_string.as_mut_ptr() as *mut c_char;
    let mut after_end: *mut c_char = std::ptr::null_mut();
    number = strtod(start as *const c_char, &mut after_end);
    if start == after_end {
        /* free the temporary buffer */
        drop(number_c_string);
        return CJSON_FALSE; /* parse_error */
    }

    (*item).valuedouble = number;

    /* use saturation in case of overflow */
    if number >= INT_MAX as c_double {
        (*item).valueint = INT_MAX;
    } else if number <= INT_MIN as c_double {
        (*item).valueint = INT_MIN;
    } else {
        (*item).valueint = double_to_int_trunc(number);
    }

    (*item).type_ = cJSON_Number;

    let consumed = (after_end as usize).wrapping_sub(start as usize);
    buffer.offset = buffer.offset.wrapping_add(consumed);
    /* free the temporary buffer */
    drop(number_c_string);
    CJSON_TRUE
}
