//! Rust translation of the C library in `c_src/`.
//!
//! The C library consists of a single translation unit (`src/lib.c`) exporting
//! exactly one public symbol: `parse_number` (a cJSON-derived number parser).
//!
//! Behaviour is reproduced exactly, including the missing NULL check on `item`
//! and the reliance on the C `strtod` for value/end-pointer determination.

// C identifiers are kept verbatim so the mapping to `c_src/` is obvious.
#![allow(non_camel_case_types)]

use core::ffi::{c_char, c_int, c_uchar};

/// `typedef int cJSON_bool;`
pub type cJSON_bool = c_int;

/* #define true  ((cJSON_bool)1) */
const CJSON_TRUE: cJSON_bool = 1;
/* #define false ((cJSON_bool)0) */
const CJSON_FALSE: cJSON_bool = 0;

/* #define INT_MIN (-__INT_MAX__ - 1) ; #define INT_MAX __INT_MAX__ */
const C_INT_MAX: c_int = c_int::MAX;
const C_INT_MIN: c_int = c_int::MIN;

/* #define cJSON_Number (1 << 3) */
const CJSON_NUMBER: c_int = 1 << 3;

/// ```c
/// typedef struct {
///     const unsigned char *content;
///     size_t length;
///     size_t offset;
///     size_t depth;
/// } parse_buffer;
/// ```
#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    /// How deeply nested (in arrays/objects) is the input at the current offset.
    pub depth: usize,
}

/// ```c
/// typedef struct {
///     int type;
///     int valueint;
///     double valuedouble;
/// } cJSON;
/// ```
#[repr(C)]
pub struct cJSON {
    /// The type of the item, as above.
    pub type_: c_int,
    /// writing to valueint is DEPRECATED, use cJSON_SetNumberValue instead
    pub valueint: c_int,
    /// The item's number, if type==cJSON_Number
    pub valuedouble: f64,
}

unsafe extern "C" {
    /// The C code depends on the exact semantics of the platform `strtod`
    /// (value rounding as well as the resulting end pointer), so the very same
    /// function is used here.
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> f64;
}

/* ------------------------------------------------------------------------- *
 * Field access through the caller-supplied pointers.
 *
 * The C compiler emits plain loads/stores for `input_buffer->x` and `item->x`,
 * which on the target ABI work for ANY non-mapped-away address — including
 * addresses that are not naturally aligned (nothing in `lib.h` promises
 * alignment, and a C caller can legally pass a packed/offset struct).
 *
 * A plain Rust `(*p).field` access does NOT reproduce that: with
 * `-C debug-assertions` (any `dev`-profile build) rustc emits null-pointer and
 * alignment UB checks that turn a would-be SIGSEGV into a non-unwinding panic
 * (SIGABRT), and turn the C's perfectly successful misaligned access into an
 * abort. Routing every access through `read_unaligned` / `write_unaligned`
 * (which carry neither precondition) keeps the observable behaviour identical to
 * the C in EVERY profile.
 * ------------------------------------------------------------------------- */

#[inline]
unsafe fn pb_content(buffer: *const parse_buffer) -> *const c_uchar {
    unsafe { core::ptr::read_unaligned(&raw const (*buffer).content) }
}

#[inline]
unsafe fn pb_length(buffer: *const parse_buffer) -> usize {
    unsafe { core::ptr::read_unaligned(&raw const (*buffer).length) }
}

#[inline]
unsafe fn pb_offset(buffer: *const parse_buffer) -> usize {
    unsafe { core::ptr::read_unaligned(&raw const (*buffer).offset) }
}

#[inline]
unsafe fn pb_set_offset(buffer: *mut parse_buffer, v: usize) {
    unsafe { core::ptr::write_unaligned(&raw mut (*buffer).offset, v) }
}

/// `#define can_access_at_index(buffer, index) \
///     ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))`
#[inline]
fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }
    // Unsigned wraparound, exactly like C's `size_t` addition.
    let (offset, length) = unsafe { (pb_offset(buffer), pb_length(buffer)) };
    offset.wrapping_add(index) < length
}

/// `#define buffer_at_offset(buffer) ((buffer)->content + (buffer)->offset)`
#[inline]
fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    // `wrapping_add` (not `add`) because the C macro is evaluated even when
    // `offset >= length`, in which case `content + offset` is a pointer that is
    // formed but never dereferenced.
    unsafe { pb_content(buffer).wrapping_add(pb_offset(buffer)) }
}

/// Emulates the C cast `(int)number` for a `double` that is not guaranteed to
/// be in range (x86-64 `cvttsd2si` yields `INT_MIN` for NaN / out of range).
#[inline]
fn double_to_int_c(number: f64) -> c_int {
    if number.is_nan() {
        return C_INT_MIN;
    }
    if number >= (C_INT_MAX as f64) + 1.0 {
        return C_INT_MIN;
    }
    if number < C_INT_MIN as f64 {
        return C_INT_MIN;
    }
    number as c_int
}

/* ------------------------------------------------------------------------- *
 * Stores into `*item`.
 *
 * The C never NULL-checks `item`, so `parse_number(NULL, buf)` with a parsable
 * number performs a plain store through a null pointer and faults with SIGSEGV.
 * A plain Rust `(*item).field = v` does NOT reproduce that: with
 * `-C debug-assertions` (i.e. any `dev`-profile build) rustc emits a
 * null-pointer UB check that turns the fault into a non-unwinding panic, which
 * aborts with SIGABRT instead. Routing the stores through `write_unaligned`
 * keeps the observable behaviour identical to the C in EVERY profile: the store
 * really is emitted, at that address, unchecked.
 * ------------------------------------------------------------------------- */

#[inline]
unsafe fn store_double(item: *mut cJSON, v: f64) {
    unsafe { core::ptr::write_unaligned(&raw mut (*item).valuedouble, v) }
}

#[inline]
unsafe fn store_valueint(item: *mut cJSON, v: c_int) {
    unsafe { core::ptr::write_unaligned(&raw mut (*item).valueint, v) }
}

#[inline]
unsafe fn store_type(item: *mut cJSON, v: c_int) {
    unsafe { core::ptr::write_unaligned(&raw mut (*item).type_, v) }
}

/// Parse the input text to generate a number, and populate the result into item.
///
/// ```c
/// cJSON_bool parse_number(cJSON * const item, parse_buffer * const input_buffer);
/// ```
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> cJSON_bool {
    let number: f64;
    let after_end: *const c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = CJSON_FALSE;

    if input_buffer.is_null() || unsafe { pb_content(input_buffer).is_null() } {
        return CJSON_FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    while can_access_at_index(input_buffer, i) {
        /* buffer_at_offset(input_buffer)[i] */
        // `wrapping_add`: a caller may supply a `length` larger than the real
        // allocation (the C then reads out of bounds too, and the differential
        // tests exercise `length == SIZE_MAX`), so no in-bounds claim is made.
        let c = unsafe { *buffer_at_offset(input_buffer).wrapping_add(i) };
        match c {
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
    /* loop_end: */

    /* malloc for temporary buffer, add 1 for '\0' */
    let mut number_c_string: Vec<c_uchar> = Vec::with_capacity(number_string_length + 1);
    /* memcpy(number_c_string, buffer_at_offset(input_buffer), number_string_length); */
    if number_string_length != 0 {
        unsafe {
            core::ptr::copy_nonoverlapping(
                buffer_at_offset(input_buffer),
                number_c_string.as_mut_ptr(),
                number_string_length,
            );
        }
    }
    unsafe {
        number_c_string.set_len(number_string_length);
    }
    /* number_c_string[number_string_length] = '\0'; */
    number_c_string.push(b'\0');

    if has_decimal_point != CJSON_FALSE {
        i = 0;
        while i < number_string_length {
            if number_c_string[i] == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                number_c_string[i] = decimal_point;
            }
            i += 1;
        }
    }

    let start: *const c_uchar = number_c_string.as_ptr();
    let mut end_ptr: *mut c_char = core::ptr::null_mut();
    number = unsafe { strtod(start as *const c_char, &mut end_ptr) };
    after_end = end_ptr as *const c_uchar;
    if start == after_end {
        /* free the temporary buffer */
        drop(number_c_string);
        return CJSON_FALSE; /* parse_error */
    }

    unsafe {
        /* item->valuedouble = number; */
        store_double(item, number);

        /* use saturation in case of overflow */
        if number >= C_INT_MAX as f64 {
            /* item->valueint = INT_MAX; */
            store_valueint(item, C_INT_MAX);
        } else if number <= C_INT_MIN as f64 {
            /* item->valueint = INT_MIN; */
            store_valueint(item, C_INT_MIN);
        } else {
            /* item->valueint = (int)number; */
            store_valueint(item, double_to_int_c(number));
        }

        /* item->type = cJSON_Number; */
        store_type(item, CJSON_NUMBER);

        /* input_buffer->offset += (size_t)(after_end - number_c_string); */
        pb_set_offset(
            input_buffer,
            pb_offset(input_buffer).wrapping_add(after_end as usize - start as usize),
        );
    }

    /* free the temporary buffer */
    drop(number_c_string);
    CJSON_TRUE
}
