//! Translation of cJSON 1.7.19 (c_src/cJSON.c) to Rust.
//!
//! Faithful transliteration: same allocation strategy (libc malloc/free so that
//! callers may `free()` returned buffers), same order of checks, same bugs.
#![allow(non_camel_case_types, non_snake_case, non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_float, c_int, c_void};
use core::ptr::{null, null_mut};

mod driver;

// ---------------------------------------------------------------------------
// libc
// ---------------------------------------------------------------------------

unsafe extern "C" {
    pub(crate) fn malloc(size: usize) -> *mut c_void;
    pub(crate) fn free(ptr: *mut c_void);
    pub(crate) fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    pub(crate) fn strlen(s: *const c_char) -> usize;
    pub(crate) fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    pub(crate) fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    pub(crate) fn strcmp(a: *const c_char, b: *const c_char) -> c_int;
    pub(crate) fn strncmp(a: *const c_char, b: *const c_char, n: usize) -> c_int;
    pub(crate) fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    pub(crate) fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> c_double;
    pub(crate) fn snprintf(s: *mut c_char, n: usize, fmt: *const c_char, ...) -> c_int;
    pub(crate) fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    pub(crate) fn printf(fmt: *const c_char, ...) -> c_int;
    pub(crate) fn exit(status: c_int) -> !;
    pub(crate) fn tolower(c: c_int) -> c_int;
}

// ---------------------------------------------------------------------------
// Public types / constants
// ---------------------------------------------------------------------------

pub type cJSON_bool = c_int;

pub const cJSON_Invalid: c_int = 0;
pub const cJSON_False: c_int = 1 << 0;
pub const cJSON_True: c_int = 1 << 1;
pub const cJSON_NULL: c_int = 1 << 2;
pub const cJSON_Number: c_int = 1 << 3;
pub const cJSON_String: c_int = 1 << 4;
pub const cJSON_Array: c_int = 1 << 5;
pub const cJSON_Object: c_int = 1 << 6;
pub const cJSON_Raw: c_int = 1 << 7;

pub const cJSON_IsReference: c_int = 256;
pub const cJSON_StringIsConst: c_int = 512;

pub const CJSON_VERSION_MAJOR: c_int = 1;
pub const CJSON_VERSION_MINOR: c_int = 7;
pub const CJSON_VERSION_PATCH: c_int = 19;

pub const CJSON_NESTING_LIMIT: usize = 1000;
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

/// `INT_MAX` promoted to double, as in the C comparisons.
const INT_MAX_D: c_double = 2147483647.0;
/// `(double)INT_MIN`
const INT_MIN_D: c_double = -2147483648.0;
/// `INT_MAX` converted to `size_t`
const INT_MAX_USIZE: usize = 2147483647;

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub type_: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

pub type MallocFn = unsafe extern "C" fn(usize) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void);
pub type ReallocFn = unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void;

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<MallocFn>,
    pub free_fn: Option<FreeFn>,
}

#[derive(Clone, Copy)]
#[repr(C)]
pub(crate) struct InternalHooks {
    pub allocate: Option<MallocFn>,
    pub deallocate: Option<FreeFn>,
    pub reallocate: Option<ReallocFn>,
}

impl InternalHooks {
    pub(crate) const NULL: InternalHooks = InternalHooks {
        allocate: None,
        deallocate: None,
        reallocate: None,
    };
}

/// `hooks->allocate(size)`
#[inline]
pub(crate) unsafe fn h_alloc(hooks: *const InternalHooks, size: usize) -> *mut c_void {
    unsafe { ((*hooks).allocate.unwrap())(size) }
}

/// `hooks->deallocate(ptr)`
#[inline]
pub(crate) unsafe fn h_free(hooks: *const InternalHooks, ptr: *mut c_void) {
    unsafe { ((*hooks).deallocate.unwrap())(ptr) }
}

// ---------------------------------------------------------------------------
// global error / hooks
// ---------------------------------------------------------------------------

#[repr(C)]
pub(crate) struct ErrorState {
    pub json: *const u8,
    pub position: usize,
}

pub(crate) static mut GLOBAL_ERROR: ErrorState = ErrorState {
    json: null(),
    position: 0,
};

pub(crate) static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: Some(malloc as MallocFn),
    deallocate: Some(free as FreeFn),
    reallocate: Some(realloc as ReallocFn),
};

#[inline]
pub(crate) fn global_hooks() -> *mut InternalHooks {
    &raw mut GLOBAL_HOOKS
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    unsafe { GLOBAL_ERROR.json.wrapping_add(GLOBAL_ERROR.position) as *const c_char }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    unsafe {
        if cJSON_IsString(item) == 0 {
            return null_mut();
        }
        (*item).valuestring
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    unsafe {
        if cJSON_IsNumber(item) == 0 {
            return c_double::NAN;
        }
        (*item).valuedouble
    }
}

static mut VERSION: [c_char; 15] = [0; 15];

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    unsafe {
        let p = (&raw mut VERSION) as *mut c_char;
        snprintf(
            p,
            15,
            c"%i.%i.%i".as_ptr(),
            CJSON_VERSION_MAJOR,
            CJSON_VERSION_MINOR,
            CJSON_VERSION_PATCH,
        );
        p
    }
}

/// Case insensitive string comparison, doesn't consider two NULL pointers equal though.
pub(crate) unsafe fn case_insensitive_strcmp(mut string1: *const u8, mut string2: *const u8) -> c_int {
    unsafe {
        if string1.is_null() || string2.is_null() {
            return 1;
        }
        if string1 == string2 {
            return 0;
        }
        while tolower(*string1 as c_int) == tolower(*string2 as c_int) {
            if *string1 == b'\0' {
                return 0;
            }
            string1 = string1.add(1);
            string2 = string2.add(1);
        }
        tolower(*string1 as c_int) - tolower(*string2 as c_int)
    }
}

pub(crate) unsafe fn cJSON_strdup(string: *const u8, hooks: *const InternalHooks) -> *mut u8 {
    unsafe {
        if string.is_null() {
            return null_mut();
        }
        let length = strlen(string as *const c_char) + 1;
        let copy = h_alloc(hooks, length) as *mut u8;
        if copy.is_null() {
            return null_mut();
        }
        memcpy(copy as *mut c_void, string as *const c_void, length);
        copy
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    unsafe {
        if hooks.is_null() {
            /* Reset hooks */
            GLOBAL_HOOKS.allocate = Some(malloc as MallocFn);
            GLOBAL_HOOKS.deallocate = Some(free as FreeFn);
            GLOBAL_HOOKS.reallocate = Some(realloc as ReallocFn);
            return;
        }

        GLOBAL_HOOKS.allocate = Some(malloc as MallocFn);
        if (*hooks).malloc_fn.is_some() {
            GLOBAL_HOOKS.allocate = (*hooks).malloc_fn;
        }

        GLOBAL_HOOKS.deallocate = Some(free as FreeFn);
        if (*hooks).free_fn.is_some() {
            GLOBAL_HOOKS.deallocate = (*hooks).free_fn;
        }

        /* use realloc only if both free and malloc are used */
        GLOBAL_HOOKS.reallocate = None;
        if GLOBAL_HOOKS.allocate == Some(malloc as MallocFn)
            && GLOBAL_HOOKS.deallocate == Some(free as FreeFn)
        {
            GLOBAL_HOOKS.reallocate = Some(realloc as ReallocFn);
        }
    }
}

/// Internal constructor.
pub(crate) unsafe fn cJSON_New_Item(hooks: *const InternalHooks) -> *mut cJSON {
    unsafe {
        let node = h_alloc(hooks, size_of::<cJSON>()) as *mut cJSON;
        if !node.is_null() {
            memset(node as *mut c_void, b'\0' as c_int, size_of::<cJSON>());
        }
        node
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    unsafe {
        while !item.is_null() {
            let next = (*item).next;
            if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).child.is_null() {
                cJSON_Delete((*item).child);
            }
            if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
                h_free(global_hooks(), (*item).valuestring as *mut c_void);
                (*item).valuestring = null_mut();
            }
            if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
                h_free(global_hooks(), (*item).string as *mut c_void);
                (*item).string = null_mut();
            }
            h_free(global_hooks(), item as *mut c_void);
            item = next;
        }
    }
}

/// get the decimal point character of the current locale (ENABLE_LOCALES is off)
#[inline]
pub(crate) fn get_decimal_point() -> u8 {
    b'.'
}

// ---------------------------------------------------------------------------
// parse_buffer
// ---------------------------------------------------------------------------

#[repr(C)]
pub(crate) struct ParseBuffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    /// How deeply nested (in arrays/objects) is the input at the current offset.
    pub depth: usize,
    pub hooks: InternalHooks,
}

/// check if the given size is left to read in a given parse buffer (starting with 1)
#[inline]
pub(crate) unsafe fn can_read(buffer: *const ParseBuffer, size: usize) -> bool {
    unsafe { !buffer.is_null() && ((*buffer).offset.wrapping_add(size) <= (*buffer).length) }
}

/// check if the buffer can be accessed at the given index (starting with 0)
#[inline]
pub(crate) unsafe fn can_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    unsafe { !buffer.is_null() && ((*buffer).offset.wrapping_add(index) < (*buffer).length) }
}

#[inline]
pub(crate) unsafe fn cannot_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    unsafe { !can_access_at_index(buffer, index) }
}

/// get a pointer to the buffer at the position
#[inline]
pub(crate) unsafe fn buffer_at_offset(buffer: *const ParseBuffer) -> *const u8 {
    unsafe { (*buffer).content.wrapping_add((*buffer).offset) }
}

/// Parse the input text to generate a number, and populate the result into item.
pub(crate) unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    unsafe {
        let number: c_double;
        let mut after_end: *mut c_char = null_mut();
        let number_c_string: *mut u8;
        let decimal_point = get_decimal_point();
        let mut i: usize;
        let mut number_string_length: usize = 0;
        let mut has_decimal_point = false;

        if input_buffer.is_null() || (*input_buffer).content.is_null() {
            return 0;
        }

        /* copy the number into a temporary buffer and replace '.' with the decimal point
         * of the current locale (for strtod)
         * This also takes care of '\0' not necessarily being available for marking the end
         * of the input */
        i = 0;
        while can_access_at_index(input_buffer, i) {
            match *buffer_at_offset(input_buffer).wrapping_add(i) {
                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+'
                | b'-' | b'e' | b'E' => {
                    number_string_length += 1;
                }
                b'.' => {
                    number_string_length += 1;
                    has_decimal_point = true;
                }
                _ => break,
            }
            i += 1;
        }

        /* malloc for temporary buffer, add 1 for '\0' */
        number_c_string =
            h_alloc(&raw const (*input_buffer).hooks, number_string_length + 1) as *mut u8;
        if number_c_string.is_null() {
            return 0; /* allocation failure */
        }

        memcpy(
            number_c_string as *mut c_void,
            buffer_at_offset(input_buffer) as *const c_void,
            number_string_length,
        );
        *number_c_string.add(number_string_length) = b'\0';

        if has_decimal_point {
            i = 0;
            while i < number_string_length {
                if *number_c_string.add(i) == b'.' {
                    /* replace '.' with the decimal point of the current locale (for strtod) */
                    *number_c_string.add(i) = decimal_point;
                }
                i += 1;
            }
        }

        number = strtod(number_c_string as *const c_char, &mut after_end);
        if number_c_string as *mut c_char == after_end {
            /* free the temporary buffer */
            h_free(&raw const (*input_buffer).hooks, number_c_string as *mut c_void);
            return 0; /* parse_error */
        }

        (*item).valuedouble = number;

        /* use saturation in case of overflow */
        if number >= INT_MAX_D {
            (*item).valueint = c_int::MAX;
        } else if number <= INT_MIN_D {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = number as c_int;
        }

        (*item).type_ = cJSON_Number;

        (*input_buffer).offset = (*input_buffer)
            .offset
            .wrapping_add((after_end as usize).wrapping_sub(number_c_string as usize));
        /* free the temporary buffer */
        h_free(&raw const (*input_buffer).hooks, number_c_string as *mut c_void);
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    unsafe {
        if number >= INT_MAX_D {
            (*object).valueint = c_int::MAX;
        } else if number <= INT_MIN_D {
            (*object).valueint = c_int::MIN;
        } else {
            (*object).valueint = number as c_int;
        }

        (*object).valuedouble = number;
        (*object).valuedouble
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    unsafe {
        let copy: *mut c_char;
        let v1_len: usize;
        let v2_len: usize;
        /* if object's type is not cJSON_String or is cJSON_IsReference,
         * it should not set valuestring */
        if object.is_null()
            || ((*object).type_ & cJSON_String) == 0
            || ((*object).type_ & cJSON_IsReference) != 0
        {
            return null_mut();
        }
        /* return NULL if the object is corrupted or valuestring is NULL */
        if (*object).valuestring.is_null() || valuestring.is_null() {
            return null_mut();
        }

        v1_len = strlen(valuestring);
        v2_len = strlen((*object).valuestring);

        if v1_len <= v2_len {
            /* strcpy does not handle overlapping string:
             * [X1, X2] [Y1, Y2] => X2 < Y1 or Y2 < X1 */
            if !(valuestring.wrapping_add(v1_len) < (*object).valuestring
                || (*object).valuestring.wrapping_add(v2_len) < valuestring as *mut c_char)
            {
                return null_mut();
            }
            strcpy((*object).valuestring, valuestring);
            return (*object).valuestring;
        }
        copy = cJSON_strdup(valuestring as *const u8, global_hooks()) as *mut c_char;
        if copy.is_null() {
            return null_mut();
        }
        if !(*object).valuestring.is_null() {
            cJSON_free((*object).valuestring as *mut c_void);
        }
        (*object).valuestring = copy;

        copy
    }
}

// ---------------------------------------------------------------------------
// printbuffer
// ---------------------------------------------------------------------------

#[repr(C)]
pub(crate) struct PrintBuffer {
    pub buffer: *mut u8,
    pub length: usize,
    pub offset: usize,
    /// current nesting depth (for formatted printing)
    pub depth: usize,
    pub noalloc: cJSON_bool,
    /// is this print a formatted print
    pub format: cJSON_bool,
    pub hooks: InternalHooks,
}

/// realloc printbuffer if necessary to have at least "needed" bytes more
pub(crate) unsafe fn ensure(p: *mut PrintBuffer, mut needed: usize) -> *mut u8 {
    unsafe {
        let mut newbuffer: *mut u8;
        let newsize: usize;

        if p.is_null() || (*p).buffer.is_null() {
            return null_mut();
        }

        if ((*p).length > 0) && ((*p).offset >= (*p).length) {
            /* make sure that offset is valid */
            return null_mut();
        }

        if needed > INT_MAX_USIZE {
            /* sizes bigger than INT_MAX are currently not supported */
            return null_mut();
        }

        needed = needed.wrapping_add((*p).offset).wrapping_add(1);
        if needed <= (*p).length {
            return (*p).buffer.wrapping_add((*p).offset);
        }

        if (*p).noalloc != 0 {
            return null_mut();
        }

        /* calculate new buffer size */
        if needed > (INT_MAX_USIZE / 2) {
            /* overflow of int, use INT_MAX if possible */
            if needed <= INT_MAX_USIZE {
                newsize = INT_MAX_USIZE;
            } else {
                return null_mut();
            }
        } else {
            newsize = needed * 2;
        }

        if let Some(reallocate) = (*p).hooks.reallocate {
            /* reallocate with realloc if available */
            newbuffer = reallocate((*p).buffer as *mut c_void, newsize) as *mut u8;
            if newbuffer.is_null() {
                h_free(&raw const (*p).hooks, (*p).buffer as *mut c_void);
                (*p).length = 0;
                (*p).buffer = null_mut();

                return null_mut();
            }
        } else {
            /* otherwise reallocate manually */
            newbuffer = h_alloc(&raw const (*p).hooks, newsize) as *mut u8;
            if newbuffer.is_null() {
                h_free(&raw const (*p).hooks, (*p).buffer as *mut c_void);
                (*p).length = 0;
                (*p).buffer = null_mut();

                return null_mut();
            }

            memcpy(
                newbuffer as *mut c_void,
                (*p).buffer as *const c_void,
                (*p).offset + 1,
            );
            h_free(&raw const (*p).hooks, (*p).buffer as *mut c_void);
        }
        (*p).length = newsize;
        (*p).buffer = newbuffer;

        newbuffer = newbuffer.wrapping_add((*p).offset);
        newbuffer
    }
}

/// calculate the new length of the string in a printbuffer and update the offset
pub(crate) unsafe fn update_offset(buffer: *mut PrintBuffer) {
    unsafe {
        if buffer.is_null() || (*buffer).buffer.is_null() {
            return;
        }
        let buffer_pointer = (*buffer).buffer.wrapping_add((*buffer).offset);

        (*buffer).offset += strlen(buffer_pointer as *const c_char);
    }
}

/// securely comparison of floating-point variables
pub(crate) fn compare_double(a: c_double, b: c_double) -> cJSON_bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    ((a - b).abs() <= max_val * c_double::EPSILON) as cJSON_bool
}

/// Render the number nicely from the given item into a string.
pub(crate) unsafe fn print_number(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    unsafe {
        let output_pointer: *mut u8;
        let d = (*item).valuedouble;
        let mut length: c_int;
        let mut i: usize = 0;
        /* temporary buffer to print the number into */
        let mut number_buffer = [0u8; 26];
        let decimal_point = get_decimal_point();
        let mut test: c_double = 0.0;

        if output_buffer.is_null() {
            return 0;
        }

        let nb = number_buffer.as_mut_ptr() as *mut c_char;

        /* This checks for NaN and Infinity */
        if d.is_nan() || d.is_infinite() {
            length = snprintf(nb, 26, c"null".as_ptr());
        } else if d == (*item).valueint as c_double {
            length = snprintf(nb, 26, c"%d".as_ptr(), (*item).valueint);
        } else {
            /* Try 15 decimal places of precision to avoid nonsignificant nonzero digits */
            length = snprintf(nb, 26, c"%1.15g".as_ptr(), d);

            /* Check whether the original double can be recovered */
            if (sscanf(nb as *const c_char, c"%lg".as_ptr(), &mut test) != 1)
                || compare_double(test, d) == 0
            {
                /* If not, print with 17 decimal places of precision */
                length = snprintf(nb, 26, c"%1.17g".as_ptr(), d);
            }
        }

        /* sprintf failed or buffer overrun occurred */
        if (length < 0) || (length > (number_buffer.len() - 1) as c_int) {
            return 0;
        }

        /* reserve appropriate space in the output */
        output_pointer = ensure(output_buffer, length as usize + 1);
        if output_pointer.is_null() {
            return 0;
        }

        /* copy the printed number to the output and replace locale
         * dependent decimal point with '.' */
        while i < (length as usize) {
            if number_buffer[i] == decimal_point {
                *output_pointer.add(i) = b'.';
                i += 1;
                continue;
            }

            *output_pointer.add(i) = number_buffer[i];
            i += 1;
        }
        *output_pointer.add(i) = b'\0';

        (*output_buffer).offset += length as usize;

        1
    }
}

// ---------------------------------------------------------------------------
// strings
// ---------------------------------------------------------------------------

/// parse 4 digit hexadecimal number
pub(crate) unsafe fn parse_hex4(input: *const u8) -> u32 {
    unsafe {
        let mut h: u32 = 0;
        let mut i: usize = 0;

        while i < 4 {
            let c = *input.add(i);
            /* parse digit */
            if c >= b'0' && c <= b'9' {
                h += c as u32 - b'0' as u32;
            } else if c >= b'A' && c <= b'F' {
                h += 10 + c as u32 - b'A' as u32;
            } else if c >= b'a' && c <= b'f' {
                h += 10 + c as u32 - b'a' as u32;
            } else {
                /* invalid */
                return 0;
            }

            if i < 3 {
                /* shift left to make place for the next nibble */
                h <<= 4;
            }
            i += 1;
        }

        h
    }
}

/// converts a UTF-16 literal to UTF-8
/// A literal can be one or two sequences of the form \uXXXX
pub(crate) unsafe fn utf16_literal_to_utf8(
    input_pointer: *const u8,
    input_end: *const u8,
    output_pointer: *mut *mut u8,
) -> u8 {
    unsafe {
        let mut codepoint: u64;
        let first_code: u32;
        let first_sequence: *const u8 = input_pointer;
        let utf8_length: u8;
        let mut utf8_position: u8;
        let sequence_length: u8;
        let mut first_byte_mark: u8 = 0;

        if input_end.offset_from(first_sequence) < 6 {
            /* input ends unexpectedly */
            return 0;
        }

        /* get the first utf16 sequence */
        first_code = parse_hex4(first_sequence.add(2));

        /* check that the code is valid */
        if (0xDC00..=0xDFFF).contains(&first_code) {
            return 0;
        }

        /* UTF16 surrogate pair */
        if (0xD800..=0xDBFF).contains(&first_code) {
            let second_sequence = first_sequence.add(6);
            let second_code: u32;
            sequence_length = 12; /* \uXXXX\uXXXX */

            if input_end.offset_from(second_sequence) < 6 {
                /* input ends unexpectedly */
                return 0;
            }

            if (*second_sequence.add(0) != b'\\') || (*second_sequence.add(1) != b'u') {
                /* missing second half of the surrogate pair */
                return 0;
            }

            /* get the second utf16 sequence */
            second_code = parse_hex4(second_sequence.add(2));
            /* check that the code is valid */
            if (second_code < 0xDC00) || (second_code > 0xDFFF) {
                /* invalid second half of the surrogate pair */
                return 0;
            }

            /* calculate the unicode codepoint from the surrogate pair */
            codepoint =
                0x10000 + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);
        } else {
            sequence_length = 6; /* \uXXXX */
            codepoint = first_code as u64;
        }

        /* encode as UTF-8
         * takes at maximum 4 bytes to encode:
         * 11110xxx 10xxxxxx 10xxxxxx 10xxxxxx */
        if codepoint < 0x80 {
            /* normal ascii, encoding 0xxxxxxx */
            utf8_length = 1;
        } else if codepoint < 0x800 {
            /* two bytes, encoding 110xxxxx 10xxxxxx */
            utf8_length = 2;
            first_byte_mark = 0xC0; /* 11000000 */
        } else if codepoint < 0x10000 {
            /* three bytes, encoding 1110xxxx 10xxxxxx 10xxxxxx */
            utf8_length = 3;
            first_byte_mark = 0xE0; /* 11100000 */
        } else if codepoint <= 0x10FFFF {
            /* four bytes, encoding 1110xxxx 10xxxxxx 10xxxxxx 10xxxxxx */
            utf8_length = 4;
            first_byte_mark = 0xF0; /* 11110000 */
        } else {
            /* invalid unicode codepoint */
            return 0;
        }

        /* encode as utf8 */
        utf8_position = utf8_length - 1;
        while utf8_position > 0 {
            /* 10xxxxxx */
            *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as u8;
            codepoint >>= 6;
            utf8_position -= 1;
        }
        /* encode first byte */
        if utf8_length > 1 {
            *(*output_pointer).add(0) = ((codepoint | first_byte_mark as u64) & 0xFF) as u8;
        } else {
            *(*output_pointer).add(0) = (codepoint & 0x7F) as u8;
        }

        *output_pointer = (*output_pointer).add(utf8_length as usize);

        sequence_length
    }
}

/// Parse the input text into an unescaped cinput, and populate item.
pub(crate) unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    unsafe {
        let mut input_pointer: *const u8 = buffer_at_offset(input_buffer).wrapping_add(1);
        let mut input_end: *const u8 = buffer_at_offset(input_buffer).wrapping_add(1);
        let mut output_pointer: *mut u8;
        let mut output: *mut u8 = null_mut();

        'fail: {
            /* not a string */
            if *buffer_at_offset(input_buffer).add(0) != b'\"' {
                break 'fail;
            }

            {
                /* calculate approximate size of the output (overestimate) */
                let allocation_length: usize;
                let mut skipped_bytes: usize = 0;
                while ((input_end.offset_from((*input_buffer).content) as usize)
                    < (*input_buffer).length)
                    && (*input_end != b'\"')
                {
                    /* is escape sequence */
                    if *input_end.add(0) == b'\\' {
                        if (input_end.add(1).offset_from((*input_buffer).content) as usize)
                            >= (*input_buffer).length
                        {
                            /* prevent buffer overflow when last input character is a backslash */
                            break 'fail;
                        }
                        skipped_bytes += 1;
                        input_end = input_end.add(1);
                    }
                    input_end = input_end.add(1);
                }
                if ((input_end.offset_from((*input_buffer).content) as usize)
                    >= (*input_buffer).length)
                    || (*input_end != b'\"')
                {
                    break 'fail; /* string ended unexpectedly */
                }

                /* This is at most how much we need for the output */
                allocation_length = (input_end.offset_from(buffer_at_offset(input_buffer)) as usize)
                    - skipped_bytes;
                output = h_alloc(&raw const (*input_buffer).hooks, allocation_length + 1) as *mut u8;
                if output.is_null() {
                    break 'fail; /* allocation failure */
                }
            }

            output_pointer = output;
            /* loop through the string literal */
            while input_pointer < input_end {
                if *input_pointer != b'\\' {
                    *output_pointer = *input_pointer;
                    output_pointer = output_pointer.add(1);
                    input_pointer = input_pointer.add(1);
                }
                /* escape sequence */
                else {
                    let mut sequence_length: u8 = 2;
                    if input_end.offset_from(input_pointer) < 1 {
                        break 'fail;
                    }

                    match *input_pointer.add(1) {
                        b'b' => {
                            *output_pointer = b'\x08';
                            output_pointer = output_pointer.add(1);
                        }
                        b'f' => {
                            *output_pointer = b'\x0C';
                            output_pointer = output_pointer.add(1);
                        }
                        b'n' => {
                            *output_pointer = b'\n';
                            output_pointer = output_pointer.add(1);
                        }
                        b'r' => {
                            *output_pointer = b'\r';
                            output_pointer = output_pointer.add(1);
                        }
                        b't' => {
                            *output_pointer = b'\t';
                            output_pointer = output_pointer.add(1);
                        }
                        b'\"' | b'\\' | b'/' => {
                            *output_pointer = *input_pointer.add(1);
                            output_pointer = output_pointer.add(1);
                        }

                        /* UTF-16 literal */
                        b'u' => {
                            sequence_length =
                                utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                            if sequence_length == 0 {
                                /* failed to convert UTF16-literal to UTF-8 */
                                break 'fail;
                            }
                        }

                        _ => break 'fail,
                    }
                    input_pointer = input_pointer.add(sequence_length as usize);
                }
            }

            /* zero terminate the output */
            *output_pointer = b'\0';

            (*item).type_ = cJSON_String;
            (*item).valuestring = output as *mut c_char;

            (*input_buffer).offset = input_end.offset_from((*input_buffer).content) as usize;
            (*input_buffer).offset += 1;

            return 1;
        }

        // fail:
        if !output.is_null() {
            h_free(&raw const (*input_buffer).hooks, output as *mut c_void);
        }

        if !input_pointer.is_null() {
            (*input_buffer).offset = input_pointer.offset_from((*input_buffer).content) as usize;
        }

        0
    }
}

/// Render the cstring provided to an escaped version that can be printed.
pub(crate) unsafe fn print_string_ptr(
    input: *const u8,
    output_buffer: *mut PrintBuffer,
) -> cJSON_bool {
    unsafe {
        let mut input_pointer: *const u8;
        let output: *mut u8;
        let mut output_pointer: *mut u8;
        let output_length: usize;
        /* numbers of additional characters needed for escaping */
        let mut escape_characters: usize = 0;

        if output_buffer.is_null() {
            return 0;
        }

        /* empty string */
        if input.is_null() {
            let out = ensure(output_buffer, 3);
            if out.is_null() {
                return 0;
            }
            strcpy(out as *mut c_char, c"\"\"".as_ptr());

            return 1;
        }

        /* set "flag" to 1 if something needs to be escaped */
        input_pointer = input;
        while *input_pointer != 0 {
            match *input_pointer {
                b'\"' | b'\\' | b'\x08' | b'\x0C' | b'\n' | b'\r' | b'\t' => {
                    /* one character escape sequence */
                    escape_characters += 1;
                }
                _ => {
                    if *input_pointer < 32 {
                        /* UTF-16 escape sequence uXXXX */
                        escape_characters += 5;
                    }
                }
            }
            input_pointer = input_pointer.add(1);
        }
        output_length = (input_pointer.offset_from(input) as usize) + escape_characters;

        output = ensure(output_buffer, output_length + 3);
        if output.is_null() {
            return 0;
        }

        /* no characters have to be escaped */
        if escape_characters == 0 {
            *output.add(0) = b'\"';
            memcpy(
                output.add(1) as *mut c_void,
                input as *const c_void,
                output_length,
            );
            *output.add(output_length + 1) = b'\"';
            *output.add(output_length + 2) = b'\0';

            return 1;
        }

        *output.add(0) = b'\"';
        output_pointer = output.add(1);
        /* copy the string */
        input_pointer = input;
        while *input_pointer != b'\0' {
            if (*input_pointer > 31) && (*input_pointer != b'\"') && (*input_pointer != b'\\') {
                /* normal character, copy */
                *output_pointer = *input_pointer;
            } else {
                /* character needs to be escaped */
                *output_pointer = b'\\';
                output_pointer = output_pointer.add(1);
                match *input_pointer {
                    b'\\' => *output_pointer = b'\\',
                    b'\"' => *output_pointer = b'\"',
                    b'\x08' => *output_pointer = b'b',
                    b'\x0C' => *output_pointer = b'f',
                    b'\n' => *output_pointer = b'n',
                    b'\r' => *output_pointer = b'r',
                    b'\t' => *output_pointer = b't',
                    _ => {
                        /* escape and print as unicode codepoint */
                        /* sprintf(output_pointer, "u%04x", *input_pointer) */
                        const HEX: &[u8; 16] = b"0123456789abcdef";
                        let v = *input_pointer as u32;
                        *output_pointer.add(0) = b'u';
                        *output_pointer.add(1) = HEX[((v >> 12) & 0xF) as usize];
                        *output_pointer.add(2) = HEX[((v >> 8) & 0xF) as usize];
                        *output_pointer.add(3) = HEX[((v >> 4) & 0xF) as usize];
                        *output_pointer.add(4) = HEX[(v & 0xF) as usize];
                        *output_pointer.add(5) = b'\0';
                        output_pointer = output_pointer.add(4);
                    }
                }
            }
            input_pointer = input_pointer.add(1);
            output_pointer = output_pointer.add(1);
        }
        *output.add(output_length + 1) = b'\"';
        *output.add(output_length + 2) = b'\0';

        1
    }
}

/// Invoke print_string_ptr (which is useful) on an item.
pub(crate) unsafe fn print_string(item: *const cJSON, p: *mut PrintBuffer) -> cJSON_bool {
    unsafe { print_string_ptr((*item).valuestring as *const u8, p) }
}

// ---------------------------------------------------------------------------
// parse / print entry points
// ---------------------------------------------------------------------------

/// Utility to jump whitespace and cr/lf
pub(crate) unsafe fn buffer_skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    unsafe {
        if buffer.is_null() || (*buffer).content.is_null() {
            return null_mut();
        }

        if cannot_access_at_index(buffer, 0) {
            return buffer;
        }

        while can_access_at_index(buffer, 0) && (*buffer_at_offset(buffer).add(0) <= 32) {
            (*buffer).offset += 1;
        }

        if (*buffer).offset == (*buffer).length {
            (*buffer).offset = (*buffer).offset.wrapping_sub(1);
        }

        buffer
    }
}

/// skip the UTF-8 BOM (byte order mark) if it is at the beginning of a buffer
pub(crate) unsafe fn skip_utf8_bom(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    unsafe {
        if buffer.is_null() || (*buffer).content.is_null() || ((*buffer).offset != 0) {
            return null_mut();
        }

        if can_access_at_index(buffer, 4)
            && (strncmp(
                buffer_at_offset(buffer) as *const c_char,
                c"\xEF\xBB\xBF".as_ptr(),
                3,
            ) == 0)
        {
            (*buffer).offset += 3;
        }

        buffer
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        let buffer_length: usize;

        if value.is_null() {
            return null_mut();
        }

        /* Adding null character size due to require_null_terminated. */
        buffer_length = strlen(value) + 1;

        cJSON_ParseWithLengthOpts(value, buffer_length, return_parse_end, require_null_terminated)
    }
}

/// Parse an object - create a new root, and populate.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        let mut buffer = ParseBuffer {
            content: null(),
            length: 0,
            offset: 0,
            depth: 0,
            hooks: InternalHooks::NULL,
        };
        let mut item: *mut cJSON = null_mut();

        /* reset error position */
        GLOBAL_ERROR.json = null();
        GLOBAL_ERROR.position = 0;

        'fail: {
            if value.is_null() || buffer_length == 0 {
                break 'fail;
            }

            buffer.content = value as *const u8;
            buffer.length = buffer_length;
            buffer.offset = 0;
            buffer.hooks = *global_hooks();

            item = cJSON_New_Item(global_hooks());
            if item.is_null() {
                /* memory fail */
                break 'fail;
            }

            if parse_value(item, buffer_skip_whitespace(skip_utf8_bom(&mut buffer))) == 0 {
                /* parse failure. ep is set. */
                break 'fail;
            }

            /* if we require null-terminated JSON without appended garbage,
             * skip and then check for a null terminator */
            if require_null_terminated != 0 {
                buffer_skip_whitespace(&mut buffer);
                if (buffer.offset >= buffer.length) || *buffer_at_offset(&buffer).add(0) != b'\0' {
                    break 'fail;
                }
            }
            if !return_parse_end.is_null() {
                *return_parse_end = buffer_at_offset(&buffer) as *const c_char;
            }

            return item;
        }

        // fail:
        if !item.is_null() {
            cJSON_Delete(item);
        }

        if !value.is_null() {
            let mut local_error = ErrorState {
                json: value as *const u8,
                position: 0,
            };

            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }

            if !return_parse_end.is_null() {
                *return_parse_end =
                    local_error.json.wrapping_add(local_error.position) as *const c_char;
            }

            GLOBAL_ERROR.json = local_error.json;
            GLOBAL_ERROR.position = local_error.position;
        }

        null_mut()
    }
}

/// Default options for cJSON_Parse
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    unsafe { cJSON_ParseWithOpts(value, null_mut(), 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: usize,
) -> *mut cJSON {
    unsafe { cJSON_ParseWithLengthOpts(value, buffer_length, null_mut(), 0) }
}

#[inline]
fn cjson_min(a: usize, b: usize) -> usize {
    if a < b { a } else { b }
}

unsafe fn print_json(
    item: *const cJSON,
    format: cJSON_bool,
    hooks: *const InternalHooks,
) -> *mut u8 {
    unsafe {
        const DEFAULT_BUFFER_SIZE: usize = 256;
        let mut buffer = PrintBuffer {
            buffer: null_mut(),
            length: 0,
            offset: 0,
            depth: 0,
            noalloc: 0,
            format: 0,
            hooks: InternalHooks::NULL,
        };
        let mut printed: *mut u8 = null_mut();
        let p: *mut PrintBuffer = &mut buffer;

        /* create buffer */
        (*p).buffer = h_alloc(hooks, DEFAULT_BUFFER_SIZE) as *mut u8;
        (*p).length = DEFAULT_BUFFER_SIZE;
        (*p).format = format;
        (*p).hooks = *hooks;

        'fail: {
            if (*p).buffer.is_null() {
                break 'fail;
            }

            /* print the value */
            if print_value(item, p) == 0 {
                break 'fail;
            }
            update_offset(p);

            /* check if reallocate is available */
            if let Some(reallocate) = (*hooks).reallocate {
                printed = reallocate((*p).buffer as *mut c_void, (*p).offset + 1) as *mut u8;
                if printed.is_null() {
                    break 'fail;
                }
                (*p).buffer = null_mut();
            } else {
                /* otherwise copy the JSON over to a new buffer */
                printed = h_alloc(hooks, (*p).offset + 1) as *mut u8;
                if printed.is_null() {
                    break 'fail;
                }
                memcpy(
                    printed as *mut c_void,
                    (*p).buffer as *const c_void,
                    cjson_min((*p).length, (*p).offset + 1),
                );
                *printed.add((*p).offset) = b'\0'; /* just to be sure */

                /* free the buffer */
                h_free(hooks, (*p).buffer as *mut c_void);
                (*p).buffer = null_mut();
            }

            return printed;
        }

        // fail:
        if !(*p).buffer.is_null() {
            h_free(hooks, (*p).buffer as *mut c_void);
            (*p).buffer = null_mut();
        }

        if !printed.is_null() {
            h_free(hooks, printed as *mut c_void);
            printed = null_mut();
        }

        printed
    }
}

/// Render a cJSON item/entity/structure to text.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    unsafe { print_json(item, 1, global_hooks()) as *mut c_char }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    unsafe { print_json(item, 0, global_hooks()) as *mut c_char }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    fmt: cJSON_bool,
) -> *mut c_char {
    unsafe {
        let mut p = PrintBuffer {
            buffer: null_mut(),
            length: 0,
            offset: 0,
            depth: 0,
            noalloc: 0,
            format: 0,
            hooks: InternalHooks::NULL,
        };

        if prebuffer < 0 {
            return null_mut();
        }

        p.buffer = h_alloc(global_hooks(), prebuffer as usize) as *mut u8;
        if p.buffer.is_null() {
            return null_mut();
        }

        p.length = prebuffer as usize;
        p.offset = 0;
        p.noalloc = 0;
        p.format = fmt;
        p.hooks = *global_hooks();

        if print_value(item, &mut p) == 0 {
            h_free(global_hooks(), p.buffer as *mut c_void);
            p.buffer = null_mut();
            return null_mut();
        }

        p.buffer as *mut c_char
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: cJSON_bool,
) -> cJSON_bool {
    unsafe {
        let mut p = PrintBuffer {
            buffer: null_mut(),
            length: 0,
            offset: 0,
            depth: 0,
            noalloc: 0,
            format: 0,
            hooks: InternalHooks::NULL,
        };

        if (length < 0) || buffer.is_null() {
            return 0;
        }

        p.buffer = buffer as *mut u8;
        p.length = length as usize;
        p.offset = 0;
        p.noalloc = 1;
        p.format = format;
        p.hooks = *global_hooks();

        print_value(item, &mut p)
    }
}

/// Parser core - when encountering text, process appropriately.
pub(crate) unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    unsafe {
        if input_buffer.is_null() || (*input_buffer).content.is_null() {
            return 0; /* no input */
        }

        /* parse the different types of values */
        /* null */
        if can_read(input_buffer, 4)
            && (strncmp(
                buffer_at_offset(input_buffer) as *const c_char,
                c"null".as_ptr(),
                4,
            ) == 0)
        {
            (*item).type_ = cJSON_NULL;
            (*input_buffer).offset += 4;
            return 1;
        }
        /* false */
        if can_read(input_buffer, 5)
            && (strncmp(
                buffer_at_offset(input_buffer) as *const c_char,
                c"false".as_ptr(),
                5,
            ) == 0)
        {
            (*item).type_ = cJSON_False;
            (*input_buffer).offset += 5;
            return 1;
        }
        /* true */
        if can_read(input_buffer, 4)
            && (strncmp(
                buffer_at_offset(input_buffer) as *const c_char,
                c"true".as_ptr(),
                4,
            ) == 0)
        {
            (*item).type_ = cJSON_True;
            (*item).valueint = 1;
            (*input_buffer).offset += 4;
            return 1;
        }
        /* string */
        if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer).add(0) == b'\"')
        {
            return parse_string(item, input_buffer);
        }
        /* number */
        if can_access_at_index(input_buffer, 0)
            && ((*buffer_at_offset(input_buffer).add(0) == b'-')
                || ((*buffer_at_offset(input_buffer).add(0) >= b'0')
                    && (*buffer_at_offset(input_buffer).add(0) <= b'9')))
        {
            return parse_number(item, input_buffer);
        }
        /* array */
        if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer).add(0) == b'[') {
            return parse_array(item, input_buffer);
        }
        /* object */
        if can_access_at_index(input_buffer, 0) && (*buffer_at_offset(input_buffer).add(0) == b'{') {
            return parse_object(item, input_buffer);
        }

        0
    }
}

/// Render a value to text.
pub(crate) unsafe fn print_value(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    unsafe {
        let output: *mut u8;

        if item.is_null() || output_buffer.is_null() {
            return 0;
        }

        match (*item).type_ & 0xFF {
            cJSON_NULL => {
                output = ensure(output_buffer, 5);
                if output.is_null() {
                    return 0;
                }
                strcpy(output as *mut c_char, c"null".as_ptr());
                1
            }

            cJSON_False => {
                output = ensure(output_buffer, 6);
                if output.is_null() {
                    return 0;
                }
                strcpy(output as *mut c_char, c"false".as_ptr());
                1
            }

            cJSON_True => {
                output = ensure(output_buffer, 5);
                if output.is_null() {
                    return 0;
                }
                strcpy(output as *mut c_char, c"true".as_ptr());
                1
            }

            cJSON_Number => print_number(item, output_buffer),

            cJSON_Raw => {
                let raw_length: usize;
                if (*item).valuestring.is_null() {
                    return 0;
                }

                raw_length = strlen((*item).valuestring) + 1;
                output = ensure(output_buffer, raw_length);
                if output.is_null() {
                    return 0;
                }
                memcpy(
                    output as *mut c_void,
                    (*item).valuestring as *const c_void,
                    raw_length,
                );
                1
            }

            cJSON_String => print_string(item, output_buffer),

            cJSON_Array => print_array(item, output_buffer),

            cJSON_Object => print_object(item, output_buffer),

            _ => 0,
        }
    }
}

// ---------------------------------------------------------------------------
// arrays / objects
// ---------------------------------------------------------------------------

/// Build an array from input text.
pub(crate) unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    unsafe {
        let mut head: *mut cJSON = null_mut(); /* head of the linked list */
        let mut current_item: *mut cJSON = null_mut();

        if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
            return 0; /* to deeply nested */
        }
        (*input_buffer).depth += 1;

        'fail: {
            'success: {
                if *buffer_at_offset(input_buffer).add(0) != b'[' {
                    /* not an array */
                    break 'fail;
                }

                (*input_buffer).offset += 1;
                buffer_skip_whitespace(input_buffer);
                if can_access_at_index(input_buffer, 0)
                    && (*buffer_at_offset(input_buffer).add(0) == b']')
                {
                    /* empty array */
                    break 'success;
                }

                /* check if we skipped to the end of the buffer */
                if cannot_access_at_index(input_buffer, 0) {
                    (*input_buffer).offset = (*input_buffer).offset.wrapping_sub(1);
                    break 'fail;
                }

                /* step back to character in front of the first element */
                (*input_buffer).offset = (*input_buffer).offset.wrapping_sub(1);
                /* loop through the comma separated array elements */
                loop {
                    /* allocate next item */
                    let new_item = cJSON_New_Item(&raw const (*input_buffer).hooks);
                    if new_item.is_null() {
                        break 'fail; /* allocation failure */
                    }

                    /* attach next item to list */
                    if head.is_null() {
                        /* start the linked list */
                        head = new_item;
                        current_item = new_item;
                    } else {
                        /* add to the end and advance */
                        (*current_item).next = new_item;
                        (*new_item).prev = current_item;
                        current_item = new_item;
                    }

                    /* parse next value */
                    (*input_buffer).offset += 1;
                    buffer_skip_whitespace(input_buffer);
                    if parse_value(current_item, input_buffer) == 0 {
                        break 'fail; /* failed to parse value */
                    }
                    buffer_skip_whitespace(input_buffer);

                    if !(can_access_at_index(input_buffer, 0)
                        && (*buffer_at_offset(input_buffer).add(0) == b','))
                    {
                        break;
                    }
                }

                if cannot_access_at_index(input_buffer, 0)
                    || *buffer_at_offset(input_buffer).add(0) != b']'
                {
                    break 'fail; /* expected end of array */
                }
            }

            // success:
            (*input_buffer).depth -= 1;

            if !head.is_null() {
                (*head).prev = current_item;
            }

            (*item).type_ = cJSON_Array;
            (*item).child = head;

            (*input_buffer).offset += 1;

            return 1;
        }

        // fail:
        if !head.is_null() {
            cJSON_Delete(head);
        }

        0
    }
}

/// Render an array to text
pub(crate) unsafe fn print_array(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    unsafe {
        let mut output_pointer: *mut u8;
        let mut length: usize;
        let mut current_element = (*item).child;

        if output_buffer.is_null() {
            return 0;
        }

        /* Compose the output array. */
        /* opening square bracket */
        output_pointer = ensure(output_buffer, 1);
        if output_pointer.is_null() {
            return 0;
        }

        *output_pointer = b'[';
        (*output_buffer).offset += 1;
        (*output_buffer).depth += 1;

        while !current_element.is_null() {
            if print_value(current_element, output_buffer) == 0 {
                return 0;
            }
            update_offset(output_buffer);
            if !(*current_element).next.is_null() {
                length = if (*output_buffer).format != 0 { 2 } else { 1 };
                output_pointer = ensure(output_buffer, length + 1);
                if output_pointer.is_null() {
                    return 0;
                }
                *output_pointer = b',';
                output_pointer = output_pointer.add(1);
                if (*output_buffer).format != 0 {
                    *output_pointer = b' ';
                    output_pointer = output_pointer.add(1);
                }
                *output_pointer = b'\0';
                (*output_buffer).offset += length;
            }
            current_element = (*current_element).next;
        }

        output_pointer = ensure(output_buffer, 2);
        if output_pointer.is_null() {
            return 0;
        }
        *output_pointer = b']';
        output_pointer = output_pointer.add(1);
        *output_pointer = b'\0';
        (*output_buffer).depth -= 1;

        1
    }
}

/// Build an object from the text.
pub(crate) unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    unsafe {
        let mut head: *mut cJSON = null_mut(); /* linked list head */
        let mut current_item: *mut cJSON = null_mut();

        if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
            return 0; /* to deeply nested */
        }
        (*input_buffer).depth += 1;

        'fail: {
            'success: {
                if cannot_access_at_index(input_buffer, 0)
                    || (*buffer_at_offset(input_buffer).add(0) != b'{')
                {
                    break 'fail; /* not an object */
                }

                (*input_buffer).offset += 1;
                buffer_skip_whitespace(input_buffer);
                if can_access_at_index(input_buffer, 0)
                    && (*buffer_at_offset(input_buffer).add(0) == b'}')
                {
                    break 'success; /* empty object */
                }

                /* check if we skipped to the end of the buffer */
                if cannot_access_at_index(input_buffer, 0) {
                    (*input_buffer).offset = (*input_buffer).offset.wrapping_sub(1);
                    break 'fail;
                }

                /* step back to character in front of the first element */
                (*input_buffer).offset = (*input_buffer).offset.wrapping_sub(1);
                /* loop through the comma separated array elements */
                loop {
                    /* allocate next item */
                    let new_item = cJSON_New_Item(&raw const (*input_buffer).hooks);
                    if new_item.is_null() {
                        break 'fail; /* allocation failure */
                    }

                    /* attach next item to list */
                    if head.is_null() {
                        /* start the linked list */
                        head = new_item;
                        current_item = new_item;
                    } else {
                        /* add to the end and advance */
                        (*current_item).next = new_item;
                        (*new_item).prev = current_item;
                        current_item = new_item;
                    }

                    if cannot_access_at_index(input_buffer, 1) {
                        break 'fail; /* nothing comes after the comma */
                    }

                    /* parse the name of the child */
                    (*input_buffer).offset += 1;
                    buffer_skip_whitespace(input_buffer);
                    if parse_string(current_item, input_buffer) == 0 {
                        break 'fail; /* failed to parse name */
                    }
                    buffer_skip_whitespace(input_buffer);

                    /* swap valuestring and string, because we parsed the name */
                    (*current_item).string = (*current_item).valuestring;
                    (*current_item).valuestring = null_mut();

                    if cannot_access_at_index(input_buffer, 0)
                        || (*buffer_at_offset(input_buffer).add(0) != b':')
                    {
                        break 'fail; /* invalid object */
                    }

                    /* parse the value */
                    (*input_buffer).offset += 1;
                    buffer_skip_whitespace(input_buffer);
                    if parse_value(current_item, input_buffer) == 0 {
                        break 'fail; /* failed to parse value */
                    }
                    buffer_skip_whitespace(input_buffer);

                    if !(can_access_at_index(input_buffer, 0)
                        && (*buffer_at_offset(input_buffer).add(0) == b','))
                    {
                        break;
                    }
                }

                if cannot_access_at_index(input_buffer, 0)
                    || (*buffer_at_offset(input_buffer).add(0) != b'}')
                {
                    break 'fail; /* expected end of object */
                }
            }

            // success:
            (*input_buffer).depth -= 1;

            if !head.is_null() {
                (*head).prev = current_item;
            }

            (*item).type_ = cJSON_Object;
            (*item).child = head;

            (*input_buffer).offset += 1;
            return 1;
        }

        // fail:
        if !head.is_null() {
            cJSON_Delete(head);
        }

        0
    }
}

/// Render an object to text.
pub(crate) unsafe fn print_object(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    unsafe {
        let mut output_pointer: *mut u8;
        let mut length: usize;
        let mut current_item = (*item).child;

        if output_buffer.is_null() {
            return 0;
        }

        /* Compose the output: */
        length = if (*output_buffer).format != 0 { 2 } else { 1 }; /* fmt: {\n */
        output_pointer = ensure(output_buffer, length + 1);
        if output_pointer.is_null() {
            return 0;
        }

        *output_pointer = b'{';
        output_pointer = output_pointer.add(1);
        (*output_buffer).depth += 1;
        if (*output_buffer).format != 0 {
            *output_pointer = b'\n';
            output_pointer = output_pointer.add(1);
        }
        (*output_buffer).offset += length;

        while !current_item.is_null() {
            if (*output_buffer).format != 0 {
                let mut i: usize = 0;
                output_pointer = ensure(output_buffer, (*output_buffer).depth);
                if output_pointer.is_null() {
                    return 0;
                }
                while i < (*output_buffer).depth {
                    *output_pointer = b'\t';
                    output_pointer = output_pointer.add(1);
                    i += 1;
                }
                (*output_buffer).offset += (*output_buffer).depth;
            }

            /* print key */
            if print_string_ptr((*current_item).string as *const u8, output_buffer) == 0 {
                return 0;
            }
            update_offset(output_buffer);

            length = if (*output_buffer).format != 0 { 2 } else { 1 };
            output_pointer = ensure(output_buffer, length);
            if output_pointer.is_null() {
                return 0;
            }
            *output_pointer = b':';
            output_pointer = output_pointer.add(1);
            if (*output_buffer).format != 0 {
                *output_pointer = b'\t';
                output_pointer = output_pointer.add(1);
            }
            (*output_buffer).offset += length;

            /* print value */
            if print_value(current_item, output_buffer) == 0 {
                return 0;
            }
            update_offset(output_buffer);

            /* print comma if not last */
            length = (if (*output_buffer).format != 0 { 1 } else { 0 })
                + (if !(*current_item).next.is_null() { 1 } else { 0 });
            output_pointer = ensure(output_buffer, length + 1);
            if output_pointer.is_null() {
                return 0;
            }
            if !(*current_item).next.is_null() {
                *output_pointer = b',';
                output_pointer = output_pointer.add(1);
            }

            if (*output_buffer).format != 0 {
                *output_pointer = b'\n';
                output_pointer = output_pointer.add(1);
            }
            *output_pointer = b'\0';
            (*output_buffer).offset += length;

            current_item = (*current_item).next;
        }

        output_pointer = ensure(
            output_buffer,
            if (*output_buffer).format != 0 {
                (*output_buffer).depth + 1
            } else {
                2
            },
        );
        if output_pointer.is_null() {
            return 0;
        }
        if (*output_buffer).format != 0 {
            let mut i: usize = 0;
            while i < ((*output_buffer).depth - 1) {
                *output_pointer = b'\t';
                output_pointer = output_pointer.add(1);
                i += 1;
            }
        }
        *output_pointer = b'}';
        output_pointer = output_pointer.add(1);
        *output_pointer = b'\0';
        (*output_buffer).depth -= 1;

        1
    }
}

// ---------------------------------------------------------------------------
// Get Array size/item / object item.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    unsafe {
        let mut child: *mut cJSON;
        let mut size: usize = 0;

        if array.is_null() {
            return 0;
        }

        child = (*array).child;

        while !child.is_null() {
            size += 1;
            child = (*child).next;
        }

        /* FIXME: Can overflow here. Cannot be fixed without breaking the API */

        size as c_int
    }
}

pub(crate) unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    unsafe {
        let mut current_child: *mut cJSON;

        if array.is_null() {
            return null_mut();
        }

        current_child = (*array).child;
        while !current_child.is_null() && (index > 0) {
            index -= 1;
            current_child = (*current_child).next;
        }

        current_child
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    unsafe {
        if index < 0 {
            return null_mut();
        }

        get_array_item(array, index as usize)
    }
}

pub(crate) unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        let mut current_element: *mut cJSON;

        if object.is_null() || name.is_null() {
            return null_mut();
        }

        current_element = (*object).child;
        if case_sensitive != 0 {
            while !current_element.is_null()
                && !(*current_element).string.is_null()
                && (strcmp(name, (*current_element).string) != 0)
            {
                current_element = (*current_element).next;
            }
        } else {
            while !current_element.is_null()
                && (case_insensitive_strcmp(
                    name as *const u8,
                    (*current_element).string as *const u8,
                ) != 0)
            {
                current_element = (*current_element).next;
            }
        }

        if current_element.is_null() || (*current_element).string.is_null() {
            return null_mut();
        }

        current_element
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { get_object_item(object, string, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe { get_object_item(object, string, 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    unsafe {
        if !cJSON_GetObjectItem(object, string).is_null() {
            1
        } else {
            0
        }
    }
}

/// Utility for array list handling.
pub(crate) unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {
    unsafe {
        (*prev).next = item;
        (*item).prev = prev;
    }
}

/// Utility for handling references.
pub(crate) unsafe fn create_reference(
    item: *const cJSON,
    hooks: *const InternalHooks,
) -> *mut cJSON {
    unsafe {
        let reference: *mut cJSON;
        if item.is_null() {
            return null_mut();
        }

        reference = cJSON_New_Item(hooks);
        if reference.is_null() {
            return null_mut();
        }

        memcpy(
            reference as *mut c_void,
            item as *const c_void,
            size_of::<cJSON>(),
        );
        (*reference).string = null_mut();
        (*reference).type_ |= cJSON_IsReference;
        (*reference).prev = null_mut();
        (*reference).next = null_mut();
        reference
    }
}

pub(crate) unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    unsafe {
        let child: *mut cJSON;

        if item.is_null() || array.is_null() || (array == item) {
            return 0;
        }

        child = (*array).child;
        /*
         * To find the last item in array quickly, we use prev in array
         */
        if child.is_null() {
            /* list is empty, start new one */
            (*array).child = item;
            (*item).prev = item;
            (*item).next = null_mut();
        } else {
            /* append to the end */
            if !(*child).prev.is_null() {
                suffix_object((*child).prev, item);
                (*(*array).child).prev = item;
            }
        }

        1
    }
}

/// Add item to array/object.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    unsafe { add_item_to_array(array, item) }
}

/// helper function to cast away const
#[inline]
pub(crate) fn cast_away_const(string: *const c_void) -> *mut c_void {
    string as *mut c_void
}

pub(crate) unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    hooks: *const InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    unsafe {
        let new_key: *mut c_char;
        let new_type: c_int;

        if object.is_null() || string.is_null() || item.is_null() || (object == item) {
            return 0;
        }

        if constant_key != 0 {
            new_key = cast_away_const(string as *const c_void) as *mut c_char;
            new_type = (*item).type_ | cJSON_StringIsConst;
        } else {
            new_key = cJSON_strdup(string as *const u8, hooks) as *mut c_char;
            if new_key.is_null() {
                return 0;
            }

            new_type = (*item).type_ & !cJSON_StringIsConst;
        }

        if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            h_free(hooks, (*item).string as *mut c_void);
        }

        (*item).string = new_key;
        (*item).type_ = new_type;

        add_item_to_array(object, item)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    unsafe { add_item_to_object(object, string, item, global_hooks(), 0) }
}

/// Add an item to an object with constant string as key
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    unsafe { add_item_to_object(object, string, item, global_hooks(), 1) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    unsafe {
        if array.is_null() {
            return 0;
        }

        add_item_to_array(array, create_reference(item, global_hooks()))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    unsafe {
        if object.is_null() || string.is_null() {
            return 0;
        }

        add_item_to_object(
            object,
            string,
            create_reference(item, global_hooks()),
            global_hooks(),
            0,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    unsafe {
        if parent.is_null()
            || item.is_null()
            || (item != (*parent).child && (*item).prev.is_null())
        {
            return null_mut();
        }

        if item != (*parent).child {
            /* not the first element */
            (*(*item).prev).next = (*item).next;
        }
        if !(*item).next.is_null() {
            /* not the last element */
            (*(*item).next).prev = (*item).prev;
        }

        if item == (*parent).child {
            /* first element */
            (*parent).child = (*item).next;
        } else if (*item).next.is_null() {
            /* last element */
            (*(*parent).child).prev = (*item).prev;
        }

        /* make sure the detached item doesn't point anywhere anymore */
        (*item).prev = null_mut();
        (*item).next = null_mut();

        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    unsafe {
        if which < 0 {
            return null_mut();
        }

        cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize))
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromArray(array, which)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe {
        let to_detach = cJSON_GetObjectItem(object, string);

        cJSON_DetachItemViaPointer(object, to_detach)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    unsafe {
        let to_detach = cJSON_GetObjectItemCaseSensitive(object, string);

        cJSON_DetachItemViaPointer(object, to_detach)
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObject(object, string)) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    unsafe { cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string)) }
}

/// Replace array/object items with new ones.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe {
        let after_inserted: *mut cJSON;

        if which < 0 || newitem.is_null() {
            return 0;
        }

        after_inserted = get_array_item(array, which as usize);
        if after_inserted.is_null() {
            return add_item_to_array(array, newitem);
        }

        if after_inserted != (*array).child && (*after_inserted).prev.is_null() {
            /* return false if after_inserted is a corrupted array item */
            return 0;
        }

        (*newitem).next = after_inserted;
        (*newitem).prev = (*after_inserted).prev;
        (*after_inserted).prev = newitem;
        if after_inserted == (*array).child {
            (*array).child = newitem;
        } else {
            (*(*newitem).prev).next = newitem;
        }
        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    unsafe {
        if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
            return 0;
        }

        if replacement == item {
            return 1;
        }

        (*replacement).next = (*item).next;
        (*replacement).prev = (*item).prev;

        if !(*replacement).next.is_null() {
            (*(*replacement).next).prev = replacement;
        }
        if (*parent).child == item {
            if (*(*parent).child).prev == (*parent).child {
                (*replacement).prev = replacement;
            }
            (*parent).child = replacement;
        } else {
            /*
             * To find the last item in array quickly, we use prev in array.
             * We can't modify the last item's next pointer where this item was
             * the parent's child
             */
            if !(*replacement).prev.is_null() {
                (*(*replacement).prev).next = replacement;
            }
            if (*replacement).next.is_null() {
                (*(*parent).child).prev = replacement;
            }
        }

        (*item).next = null_mut();
        (*item).prev = null_mut();
        cJSON_Delete(item);

        1
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe {
        if which < 0 {
            return 0;
        }

        cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
    }
}

pub(crate) unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    unsafe {
        if replacement.is_null() || string.is_null() {
            return 0;
        }

        /* replace the name in the replacement */
        if ((*replacement).type_ & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
            cJSON_free((*replacement).string as *mut c_void);
        }
        (*replacement).string = cJSON_strdup(string as *const u8, global_hooks()) as *mut c_char;
        if (*replacement).string.is_null() {
            return 0;
        }

        (*replacement).type_ &= !cJSON_StringIsConst;

        cJSON_ReplaceItemViaPointer(
            object,
            get_object_item(object, string, case_sensitive),
            replacement,
        )
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe { replace_item_in_object(object, string, newitem, 0) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    unsafe { replace_item_in_object(object, string, newitem, 1) }
}

// ---------------------------------------------------------------------------
// Create basic types:
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_NULL;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_True;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_False;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = if boolean != 0 { cJSON_True } else { cJSON_False };
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Number;
            (*item).valuedouble = num;

            /* use saturation in case of overflow */
            if num >= INT_MAX_D {
                (*item).valueint = c_int::MAX;
            } else if num <= INT_MIN_D {
                (*item).valueint = c_int::MIN;
            } else {
                (*item).valueint = num as c_int;
            }
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_String;
            (*item).valuestring =
                cJSON_strdup(string as *const u8, global_hooks()) as *mut c_char;
            if (*item).valuestring.is_null() {
                cJSON_Delete(item);
                return null_mut();
            }
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_String | cJSON_IsReference;
            (*item).valuestring = cast_away_const(string as *const c_void) as *mut c_char;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Object | cJSON_IsReference;
            (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Array | cJSON_IsReference;
            (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Raw;
            (*item).valuestring = cJSON_strdup(raw as *const u8, global_hooks()) as *mut c_char;
            if (*item).valuestring.is_null() {
                cJSON_Delete(item);
                return null_mut();
            }
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Array;
        }
        item
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    unsafe {
        let item = cJSON_New_Item(global_hooks());
        if !item.is_null() {
            (*item).type_ = cJSON_Object;
        }
        item
    }
}

// ---------------------------------------------------------------------------
// Helper functions for creating and adding items to an object at the same time.
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    unsafe {
        let null_item = cJSON_CreateNull();
        if add_item_to_object(object, name, null_item, global_hooks(), 0) != 0 {
            return null_item;
        }
        cJSON_Delete(null_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    unsafe {
        let true_item = cJSON_CreateTrue();
        if add_item_to_object(object, name, true_item, global_hooks(), 0) != 0 {
            return true_item;
        }
        cJSON_Delete(true_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    unsafe {
        let false_item = cJSON_CreateFalse();
        if add_item_to_object(object, name, false_item, global_hooks(), 0) != 0 {
            return false_item;
        }
        cJSON_Delete(false_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        let bool_item = cJSON_CreateBool(boolean);
        if add_item_to_object(object, name, bool_item, global_hooks(), 0) != 0 {
            return bool_item;
        }
        cJSON_Delete(bool_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    unsafe {
        let number_item = cJSON_CreateNumber(number);
        if add_item_to_object(object, name, number_item, global_hooks(), 0) != 0 {
            return number_item;
        }
        cJSON_Delete(number_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    unsafe {
        let string_item = cJSON_CreateString(string);
        if add_item_to_object(object, name, string_item, global_hooks(), 0) != 0 {
            return string_item;
        }
        cJSON_Delete(string_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    unsafe {
        let raw_item = cJSON_CreateRaw(raw);
        if add_item_to_object(object, name, raw_item, global_hooks(), 0) != 0 {
            return raw_item;
        }
        cJSON_Delete(raw_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    unsafe {
        let object_item = cJSON_CreateObject();
        if add_item_to_object(object, name, object_item, global_hooks(), 0) != 0 {
            return object_item;
        }
        cJSON_Delete(object_item);
        null_mut()
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    unsafe {
        let array = cJSON_CreateArray();
        if add_item_to_object(object, name, array, global_hooks(), 0) != 0 {
            return array;
        }
        cJSON_Delete(array);
        null_mut()
    }
}

// ---------------------------------------------------------------------------
// Create Arrays:
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    unsafe {
        let mut i: usize = 0;
        let mut n: *mut cJSON = null_mut();
        let mut p: *mut cJSON = null_mut();
        let a: *mut cJSON;

        if (count < 0) || numbers.is_null() {
            return null_mut();
        }

        a = cJSON_CreateArray();

        while !a.is_null() && (i < count as usize) {
            n = cJSON_CreateNumber(*numbers.add(i) as c_double);
            if n.is_null() {
                cJSON_Delete(a);
                return null_mut();
            }
            if i == 0 {
                (*a).child = n;
            } else {
                suffix_object(p, n);
            }
            p = n;
            i += 1;
        }

        if !a.is_null() && !(*a).child.is_null() {
            (*(*a).child).prev = n;
        }

        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(numbers: *const c_float, count: c_int) -> *mut cJSON {
    unsafe {
        let mut i: usize = 0;
        let mut n: *mut cJSON = null_mut();
        let mut p: *mut cJSON = null_mut();
        let a: *mut cJSON;

        if (count < 0) || numbers.is_null() {
            return null_mut();
        }

        a = cJSON_CreateArray();

        while !a.is_null() && (i < count as usize) {
            n = cJSON_CreateNumber(*numbers.add(i) as c_double);
            if n.is_null() {
                cJSON_Delete(a);
                return null_mut();
            }
            if i == 0 {
                (*a).child = n;
            } else {
                suffix_object(p, n);
            }
            p = n;
            i += 1;
        }

        if !a.is_null() && !(*a).child.is_null() {
            (*(*a).child).prev = n;
        }

        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    unsafe {
        let mut i: usize = 0;
        let mut n: *mut cJSON = null_mut();
        let mut p: *mut cJSON = null_mut();
        let a: *mut cJSON;

        if (count < 0) || numbers.is_null() {
            return null_mut();
        }

        a = cJSON_CreateArray();

        while !a.is_null() && (i < count as usize) {
            n = cJSON_CreateNumber(*numbers.add(i));
            if n.is_null() {
                cJSON_Delete(a);
                return null_mut();
            }
            if i == 0 {
                (*a).child = n;
            } else {
                suffix_object(p, n);
            }
            p = n;
            i += 1;
        }

        if !a.is_null() && !(*a).child.is_null() {
            (*(*a).child).prev = n;
        }

        a
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    unsafe {
        let mut i: usize = 0;
        let mut n: *mut cJSON = null_mut();
        let mut p: *mut cJSON = null_mut();
        let a: *mut cJSON;

        if (count < 0) || strings.is_null() {
            return null_mut();
        }

        a = cJSON_CreateArray();

        while !a.is_null() && (i < count as usize) {
            n = cJSON_CreateString(*strings.add(i));
            if n.is_null() {
                cJSON_Delete(a);
                return null_mut();
            }
            if i == 0 {
                (*a).child = n;
            } else {
                suffix_object(p, n);
            }
            p = n;
            i += 1;
        }

        if !a.is_null() && !(*a).child.is_null() {
            (*(*a).child).prev = n;
        }

        a
    }
}

// ---------------------------------------------------------------------------
// Duplication
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    unsafe { cJSON_Duplicate_rec(item, 0, recurse) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: usize,
    recurse: cJSON_bool,
) -> *mut cJSON {
    unsafe {
        let mut newitem: *mut cJSON = null_mut();
        let mut child: *mut cJSON;
        let mut next: *mut cJSON = null_mut();
        let mut newchild: *mut cJSON = null_mut();

        'fail: {
            /* Bail on bad ptr */
            if item.is_null() {
                break 'fail;
            }
            /* Create new item */
            newitem = cJSON_New_Item(global_hooks());
            if newitem.is_null() {
                break 'fail;
            }
            /* Copy over all vars */
            (*newitem).type_ = (*item).type_ & !cJSON_IsReference;
            (*newitem).valueint = (*item).valueint;
            (*newitem).valuedouble = (*item).valuedouble;
            if !(*item).valuestring.is_null() {
                (*newitem).valuestring =
                    cJSON_strdup((*item).valuestring as *const u8, global_hooks()) as *mut c_char;
                if (*newitem).valuestring.is_null() {
                    break 'fail;
                }
            }
            if !(*item).string.is_null() {
                (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {
                    (*item).string
                } else {
                    cJSON_strdup((*item).string as *const u8, global_hooks()) as *mut c_char
                };
                if (*newitem).string.is_null() {
                    break 'fail;
                }
            }
            /* If non-recursive, then we're done! */
            if recurse == 0 {
                return newitem;
            }
            /* Walk the ->next chain for the child. */
            child = (*item).child;
            while !child.is_null() {
                if depth >= CJSON_CIRCULAR_LIMIT {
                    break 'fail;
                }
                /* Duplicate (with recurse) each item in the ->next chain */
                newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
                if newchild.is_null() {
                    break 'fail;
                }
                if !next.is_null() {
                    /* If newitem->child already set, then crosswire ->prev and ->next
                     * and move on */
                    (*next).next = newchild;
                    (*newchild).prev = next;
                    next = newchild;
                } else {
                    /* Set newitem->child and move to it */
                    (*newitem).child = newchild;
                    next = newchild;
                }
                child = (*child).next;
            }
            if !newitem.is_null() && !(*newitem).child.is_null() {
                (*(*newitem).child).prev = newchild;
            }

            return newitem;
        }

        // fail:
        if !newitem.is_null() {
            cJSON_Delete(newitem);
        }

        null_mut()
    }
}

// ---------------------------------------------------------------------------
// Minify
// ---------------------------------------------------------------------------

pub(crate) unsafe fn skip_oneline_comment(input: &mut *mut u8) {
    unsafe {
        *input = (*input).add(2); /* static_strlen("//") */

        while *(*input).add(0) != b'\0' {
            if *(*input).add(0) == b'\n' {
                *input = (*input).add(1); /* static_strlen("\n") */
                return;
            }
            *input = (*input).add(1);
        }
    }
}

pub(crate) unsafe fn skip_multiline_comment(input: &mut *mut u8) {
    unsafe {
        // static_strlen of the two-character comment opener
        *input = (*input).add(2);

        while *(*input).add(0) != b'\0' {
            if (*(*input).add(0) == b'*') && (*(*input).add(1) == b'/') {
                *input = (*input).add(2); /* static_strlen("*\/") */
                return;
            }
            *input = (*input).add(1);
        }
    }
}

pub(crate) unsafe fn minify_string(input: &mut *mut u8, output: &mut *mut u8) {
    unsafe {
        *(*output).add(0) = *(*input).add(0);
        *input = (*input).add(1);
        *output = (*output).add(1);

        while *(*input).add(0) != b'\0' {
            *(*output).add(0) = *(*input).add(0);

            if *(*input).add(0) == b'\"' {
                *(*output).add(0) = b'\"';
                *input = (*input).add(1);
                *output = (*output).add(1);
                return;
            } else if (*(*input).add(0) == b'\\') && (*(*input).add(1) == b'\"') {
                *(*output).add(1) = *(*input).add(1);
                *input = (*input).add(1);
                *output = (*output).add(1);
            }

            *input = (*input).add(1);
            *output = (*output).add(1);
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    unsafe {
        let mut json: *mut u8 = json as *mut u8;
        let mut into: *mut u8 = json;

        if json.is_null() {
            return;
        }

        while *json.add(0) != b'\0' {
            match *json.add(0) {
                b' ' | b'\t' | b'\r' | b'\n' => {
                    json = json.add(1);
                }

                b'/' => {
                    if *json.add(1) == b'/' {
                        skip_oneline_comment(&mut json);
                    } else if *json.add(1) == b'*' {
                        skip_multiline_comment(&mut json);
                    } else {
                        json = json.add(1);
                    }
                }

                b'\"' => {
                    minify_string(&mut json, &mut into);
                }

                _ => {
                    *into.add(0) = *json.add(0);
                    json = json.add(1);
                    into = into.add(1);
                }
            }
        }

        /* and null-terminate. */
        *into = b'\0';
    }
}

// ---------------------------------------------------------------------------
// Type checks
// ---------------------------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_Invalid) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_False) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xff) == cJSON_True) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & (cJSON_True | cJSON_False)) != 0) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_NULL) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_Number) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_String) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_Array) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_Object) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    unsafe {
        if item.is_null() {
            return 0;
        }
        (((*item).type_ & 0xFF) == cJSON_Raw) as cJSON_bool
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    unsafe {
        if a.is_null() || b.is_null() || (((*a).type_ & 0xFF) != ((*b).type_ & 0xFF)) {
            return 0;
        }

        /* check if type is valid */
        match (*a).type_ & 0xFF {
            cJSON_False | cJSON_True | cJSON_NULL | cJSON_Number | cJSON_String | cJSON_Raw
            | cJSON_Array | cJSON_Object => {}
            _ => return 0,
        }

        /* identical objects are equal */
        if a == b {
            return 1;
        }

        match (*a).type_ & 0xFF {
            /* in these cases and equal type is enough */
            cJSON_False | cJSON_True | cJSON_NULL => 1,

            cJSON_Number => {
                if compare_double((*a).valuedouble, (*b).valuedouble) != 0 {
                    return 1;
                }
                0
            }

            cJSON_String | cJSON_Raw => {
                if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                    return 0;
                }
                if strcmp((*a).valuestring, (*b).valuestring) == 0 {
                    return 1;
                }
                0
            }

            cJSON_Array => {
                let mut a_element = (*a).child;
                let mut b_element = (*b).child;

                while !a_element.is_null() && !b_element.is_null() {
                    if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                        return 0;
                    }

                    a_element = (*a_element).next;
                    b_element = (*b_element).next;
                }

                /* one of the arrays is longer than the other */
                if a_element != b_element {
                    return 0;
                }

                1
            }

            cJSON_Object => {
                let mut a_element: *mut cJSON;
                let mut b_element: *mut cJSON;

                a_element = if !a.is_null() { (*a).child } else { null_mut() };
                while !a_element.is_null() {
                    /* TODO This has O(n^2) runtime, which is horrible! */
                    b_element = get_object_item(b, (*a_element).string, case_sensitive);
                    if b_element.is_null() {
                        return 0;
                    }

                    if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                        return 0;
                    }
                    a_element = (*a_element).next;
                }

                /* doing this twice, once on a and b to prevent true comparison if a
                 * subset of b */
                b_element = if !b.is_null() { (*b).child } else { null_mut() };
                while !b_element.is_null() {
                    a_element = get_object_item(a, (*b_element).string, case_sensitive);
                    if a_element.is_null() {
                        return 0;
                    }

                    if cJSON_Compare(b_element, a_element, case_sensitive) == 0 {
                        return 0;
                    }
                    b_element = (*b_element).next;
                }

                1
            }

            _ => 0,
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    unsafe { h_alloc(global_hooks(), size) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    unsafe {
        h_free(global_hooks(), object);
    }
}
