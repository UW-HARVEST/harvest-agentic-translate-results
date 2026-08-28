//! Rust translation of the C library in `c_src/`.
//!
//! The C library (`c_src/src/lib.c` + `c_src/include/lib.h`) is a extracted
//! driver around cJSON's `parse_number`. Its complete public ABI, as reported by
//! `nm -D libdriver.so`, is a single exported symbol:
//!
//! ```text
//! parse_number
//! ```
//!
//! The header declares no namespace/renaming macros, so the linker name equals
//! the source-level name.
//!
//! This translation is deliberately literal: allocation is performed through
//! libc `malloc`/`free` and number conversion through libc `strtod` so that
//! behaviour (including allocation-failure handling, locale-dependent parsing,
//! rounding and `endptr` placement) is byte-for-byte identical to the C build.
//! Original bugs/quirks are preserved, notably the missing `item == NULL` check.

#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

use core::ffi::{c_char, c_double, c_int, c_uchar, c_void};

/* ---------------------------------------------------------------------------
 * include/lib.h
 * ------------------------------------------------------------------------- */

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

/// ```c
/// typedef struct
/// {
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
    pub valuedouble: c_double,
}

/* ---------------------------------------------------------------------------
 * libc bindings (stdlib.h / string.h)
 * ------------------------------------------------------------------------- */

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

/* ---------------------------------------------------------------------------
 * src/lib.c helper macros
 * ------------------------------------------------------------------------- */

/// `#define can_access_at_index(buffer, index) \
///      ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))`
#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }
    // C `size_t` arithmetic wraps; mirror that instead of panicking.
    unsafe { (*buffer).offset.wrapping_add(index) < (*buffer).length }
}

/// `#define buffer_at_offset(buffer) ((buffer)->content + (buffer)->offset)`
#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    unsafe { (*buffer).content.wrapping_add((*buffer).offset) }
}

/// Faithful equivalent of the C statement `item->FIELD = VALUE;`.
///
/// The C performs **no** `item != NULL` check (lib.c:92 stores straight through
/// the parameter), and that missing check is a behaviour we must reproduce, not
/// fix. Writing it in Rust as the place expression `(*item).FIELD = VALUE` compiles
/// to the same plain store *only* when UB checks are off; under
/// `-C debug-assertions` (which Cargo's `dev` profile enables by default) rustc
/// additionally emits a null-pointer-dereference check, so a NULL `item` aborts
/// with a Rust panic (SIGABRT + a message on stderr) where the C raises SIGSEGV.
///
/// `addr_of_mut!` computes the field's address *without* dereferencing, and
/// `ptr::write` performs exactly the same plain, non-volatile, non-atomic store
/// the C does — so a NULL `item` faults with SIGSEGV at the identical address in
/// every profile, matching the C build byte for byte.
macro_rules! item_store {
    ($item:expr, $field:ident, $value:expr) => {
        unsafe { core::ptr::write(core::ptr::addr_of_mut!((*$item).$field), $value) }
    };
}

/* ---------------------------------------------------------------------------
 * Public ABI
 * ------------------------------------------------------------------------- */

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
    let number: c_double;
    let mut after_end: *mut c_uchar = core::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = CJSON_FALSE;

    if input_buffer.is_null() || unsafe { (*input_buffer).content }.is_null() {
        return CJSON_FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    while unsafe { can_access_at_index(input_buffer, i) } {
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
        i = i.wrapping_add(1);
    }
    /* loop_end: */

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = unsafe { malloc(number_string_length.wrapping_add(1)) } as *mut c_uchar;
    if number_c_string.is_null() {
        return CJSON_FALSE; /* allocation failure */
    }

    unsafe {
        core::ptr::copy_nonoverlapping(
            buffer_at_offset(input_buffer),
            number_c_string,
            number_string_length,
        );
        *number_c_string.wrapping_add(number_string_length) = b'\0';
    }

    if has_decimal_point != CJSON_FALSE {
        i = 0;
        while i < number_string_length {
            if unsafe { *number_c_string.wrapping_add(i) } == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                unsafe { *number_c_string.wrapping_add(i) = decimal_point };
            }
            i += 1;
        }
    }

    number = unsafe {
        strtod(
            number_c_string as *const c_char,
            &mut after_end as *mut *mut c_uchar as *mut *mut c_char,
        )
    };
    if number_c_string == after_end {
        /* free the temporary buffer */
        unsafe { free(number_c_string as *mut c_void) };
        return CJSON_FALSE; /* parse_error */
    }

    // NOTE: the original C never checks `item != NULL`; dereferencing a NULL
    // `item` faults there and faults here too. Bug preserved intentionally.
    // See `item_store!` for why the stores go through `addr_of_mut!`.
    item_store!(item, valuedouble, number);

    /* use saturation in case of overflow */
    if number >= INT_MAX as c_double {
        item_store!(item, valueint, INT_MAX);
    } else if number <= INT_MIN as c_double {
        item_store!(item, valueint, INT_MIN);
    } else {
        /* C's `(int)` cast truncates toward zero; the branches above guarantee
         * the value is representable, so a plain `as` cast is exact. NaN cannot
         * reach here (the scanner above only admits [0-9+-eE.]), but mirror the
         * x86-64 `cvttsd2si` result for it anyway rather than Rust's 0. */
        let truncated = if number.is_nan() {
            INT_MIN
        } else {
            number as c_int
        };
        item_store!(item, valueint, truncated);
    }

    item_store!(item, type_, cJSON_Number);

    unsafe {
        (*input_buffer).offset = (*input_buffer)
            .offset
            .wrapping_add((after_end as usize).wrapping_sub(number_c_string as usize));
    }
    /* free the temporary buffer */
    unsafe { free(number_c_string as *mut c_void) };
    CJSON_TRUE
}
