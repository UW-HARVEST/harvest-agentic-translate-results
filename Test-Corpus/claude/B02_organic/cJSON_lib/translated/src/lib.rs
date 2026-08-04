//! cJSON library - Rust translation
//!
//! Byte-for-byte compatible translation of cJSON v1.7.19.
#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(clippy::missing_safety_doc)]
#![allow(clippy::not_unsafe_ptr_arg_deref)]

mod test;

use core::ffi::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
use core::ptr;

// ----- C library function declarations we need (for byte-identical sprintf) -----

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn memcpy(dst: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(dst: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strcpy(dst: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strtod(s: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn sprintf(buf: *mut c_char, fmt: *const c_char, ...) -> c_int;
    fn sscanf(s: *const c_char, fmt: *const c_char, ...) -> c_int;
    fn tolower(c: c_int) -> c_int;
}

// ----- Constants from cJSON.h -----

pub const CJSON_VERSION_MAJOR: c_int = 1;
pub const CJSON_VERSION_MINOR: c_int = 7;
pub const CJSON_VERSION_PATCH: c_int = 19;

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

pub const CJSON_NESTING_LIMIT: usize = 1000;
pub const CJSON_CIRCULAR_LIMIT: usize = 10000;

const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;

// IEEE 754 double DBL_EPSILON
const DBL_EPSILON: f64 = 2.2204460492503131e-16_f64;

// cJSON_bool is C's int
pub type cJSON_bool = c_int;
const TRUE: cJSON_bool = 1;
const FALSE: cJSON_bool = 0;

// ----- Struct definitions -----

#[repr(C)]
pub struct cJSON {
    pub next: *mut cJSON,
    pub prev: *mut cJSON,
    pub child: *mut cJSON,
    pub r#type: c_int,
    pub valuestring: *mut c_char,
    pub valueint: c_int,
    pub valuedouble: c_double,
    pub string: *mut c_char,
}

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(sz: usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(ptr: *mut c_void)>,
}

#[repr(C)]
#[derive(Clone, Copy)]
struct internal_hooks {
    allocate: Option<unsafe extern "C" fn(size: usize) -> *mut c_void>,
    deallocate: Option<unsafe extern "C" fn(pointer: *mut c_void)>,
    reallocate: Option<unsafe extern "C" fn(pointer: *mut c_void, size: usize) -> *mut c_void>,
}

unsafe extern "C" fn internal_malloc(size: usize) -> *mut c_void {
    malloc(size)
}
unsafe extern "C" fn internal_free(ptr: *mut c_void) {
    free(ptr)
}
unsafe extern "C" fn internal_realloc(ptr: *mut c_void, size: usize) -> *mut c_void {
    realloc(ptr, size)
}

// ----- Globals -----

#[repr(C)]
struct ErrorState {
    json: *const c_uchar,
    position: usize,
}

// SAFETY: single-threaded use is the C library's contract.
static mut GLOBAL_ERROR: ErrorState = ErrorState {
    json: ptr::null(),
    position: 0,
};

static mut GLOBAL_HOOKS: internal_hooks = internal_hooks {
    allocate: Some(internal_malloc),
    deallocate: Some(internal_free),
    reallocate: Some(internal_realloc),
};

static mut VERSION_BUF: [c_char; 15] = [0; 15];

// ----- Buffer helpers -----

#[repr(C)]
struct parse_buffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: internal_hooks,
}

#[repr(C)]
struct printbuffer {
    buffer: *mut c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: internal_hooks,
}

#[inline]
unsafe fn can_read(buffer: *const parse_buffer, size: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset.wrapping_add(size) <= (*buffer).length)
}

#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset.wrapping_add(index) < (*buffer).length)
}

#[inline]
unsafe fn cannot_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !can_access_at_index(buffer, index)
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

// ----- Public functions -----

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    GLOBAL_ERROR.json.add(GLOBAL_ERROR.position) as *const c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if cJSON_IsString(item) == 0 {
        return ptr::null_mut();
    }
    (*item).valuestring
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if cJSON_IsNumber(item) == 0 {
        return f64::NAN;
    }
    (*item).valuedouble
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    let fmt = b"%i.%i.%i\0".as_ptr() as *const c_char;
    sprintf(
        VERSION_BUF.as_mut_ptr(),
        fmt,
        CJSON_VERSION_MAJOR,
        CJSON_VERSION_MINOR,
        CJSON_VERSION_PATCH,
    );
    VERSION_BUF.as_ptr()
}

// Case insensitive string comparison
unsafe fn case_insensitive_strcmp(string1: *const c_uchar, string2: *const c_uchar) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }
    if string1 == string2 {
        return 0;
    }

    let mut s1 = string1;
    let mut s2 = string2;
    while tolower(*s1 as c_int) == tolower(*s2 as c_int) {
        if *s1 == 0 {
            return 0;
        }
        s1 = s1.add(1);
        s2 = s2.add(1);
    }

    tolower(*s1 as c_int) - tolower(*s2 as c_int)
}

unsafe fn cJSON_strdup(string: *const c_uchar, hooks: *const internal_hooks) -> *mut c_uchar {
    if string.is_null() {
        return ptr::null_mut();
    }

    let length = strlen(string as *const c_char) + 1;
    let copy = ((*hooks).allocate.unwrap())(length) as *mut c_uchar;
    if copy.is_null() {
        return ptr::null_mut();
    }
    memcpy(copy as *mut c_void, string as *const c_void, length);
    copy
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        GLOBAL_HOOKS.allocate = Some(internal_malloc);
        GLOBAL_HOOKS.deallocate = Some(internal_free);
        GLOBAL_HOOKS.reallocate = Some(internal_realloc);
        return;
    }

    GLOBAL_HOOKS.allocate = Some(internal_malloc);
    if (*hooks).malloc_fn.is_some() {
        GLOBAL_HOOKS.allocate = (*hooks).malloc_fn;
    }

    GLOBAL_HOOKS.deallocate = Some(internal_free);
    if (*hooks).free_fn.is_some() {
        GLOBAL_HOOKS.deallocate = (*hooks).free_fn;
    }

    GLOBAL_HOOKS.reallocate = None;
    let alloc_is_default = match GLOBAL_HOOKS.allocate {
        Some(f) => f as usize == internal_malloc as usize,
        None => false,
    };
    let dealloc_is_default = match GLOBAL_HOOKS.deallocate {
        Some(f) => f as usize == internal_free as usize,
        None => false,
    };
    if alloc_is_default && dealloc_is_default {
        GLOBAL_HOOKS.reallocate = Some(internal_realloc);
    }
}

unsafe fn cJSON_New_Item(hooks: *const internal_hooks) -> *mut cJSON {
    let node = ((*hooks).allocate.unwrap())(core::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        memset(node as *mut c_void, 0, core::mem::size_of::<cJSON>());
    }
    node
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let mut next: *mut cJSON;
    while !item.is_null() {
        next = (*item).next;
        if ((*item).r#type & cJSON_IsReference) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).r#type & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
            (GLOBAL_HOOKS.deallocate.unwrap())((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).r#type & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            (GLOBAL_HOOKS.deallocate.unwrap())((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        (GLOBAL_HOOKS.deallocate.unwrap())(item as *mut c_void);
        item = next;
    }
}

#[inline]
fn get_decimal_point() -> c_uchar {
    b'.'
}

unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut number: f64;
    let mut after_end: *mut c_uchar = ptr::null_mut();
    let decimal_point = get_decimal_point();
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE;
    }

    let mut i: usize = 0;
    'outer: while can_access_at_index(input_buffer, i) {
        let c = *buffer_at_offset(input_buffer).add(i);
        match c {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = TRUE;
            }
            _ => break 'outer,
        }
        i += 1;
    }

    let number_c_string =
        ((*input_buffer).hooks.allocate.unwrap())(number_string_length + 1) as *mut c_uchar;
    if number_c_string.is_null() {
        return FALSE;
    }

    memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point != 0 {
        for j in 0..number_string_length {
            if *number_c_string.add(j) == b'.' {
                *number_c_string.add(j) = decimal_point;
            }
        }
    }

    number = strtod(
        number_c_string as *const c_char,
        &mut after_end as *mut *mut c_uchar as *mut *mut c_char,
    );
    if number_c_string == after_end {
        ((*input_buffer).hooks.deallocate.unwrap())(number_c_string as *mut c_void);
        return FALSE;
    }

    (*item).valuedouble = number;

    if number >= INT_MAX as f64 {
        (*item).valueint = INT_MAX;
    } else if number <= INT_MIN as f64 {
        (*item).valueint = INT_MIN;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).r#type = cJSON_Number;

    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    ((*input_buffer).hooks.deallocate.unwrap())(number_c_string as *mut c_void);
    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= INT_MAX as f64 {
        (*object).valueint = INT_MAX;
    } else if number <= INT_MIN as f64 {
        (*object).valueint = INT_MIN;
    } else {
        (*object).valueint = number as c_int;
    }

    (*object).valuedouble = number;
    number
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    if object.is_null()
        || ((*object).r#type & cJSON_String) == 0
        || ((*object).r#type & cJSON_IsReference) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }

    let v1_len = strlen(valuestring);
    let v2_len = strlen((*object).valuestring);

    if v1_len <= v2_len {
        // detect overlap
        let v_end = valuestring.add(v1_len);
        let o_end = (*object).valuestring.add(v2_len);
        let no_overlap =
            (v_end < (*object).valuestring as *const c_char) || ((o_end as *const c_char) < valuestring);
        if !no_overlap {
            return ptr::null_mut();
        }
        strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cJSON_strdup(valuestring as *const c_uchar, &raw const GLOBAL_HOOKS) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

unsafe fn ensure(p: *mut printbuffer, mut needed: usize) -> *mut c_uchar {
    if p.is_null() || (*p).buffer.is_null() {
        return ptr::null_mut();
    }

    if (*p).length > 0 && (*p).offset >= (*p).length {
        return ptr::null_mut();
    }

    if needed > INT_MAX as usize {
        return ptr::null_mut();
    }

    needed += (*p).offset + 1;
    if needed <= (*p).length {
        return (*p).buffer.add((*p).offset);
    }

    if (*p).noalloc != 0 {
        return ptr::null_mut();
    }

    let newsize: usize = if needed > (INT_MAX as usize) / 2 {
        if needed <= INT_MAX as usize {
            INT_MAX as usize
        } else {
            return ptr::null_mut();
        }
    } else {
        needed * 2
    };

    let newbuffer: *mut c_uchar;
    if (*p).hooks.reallocate.is_some() {
        newbuffer = ((*p).hooks.reallocate.unwrap())((*p).buffer as *mut c_void, newsize)
            as *mut c_uchar;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate.unwrap())((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
    } else {
        newbuffer = ((*p).hooks.allocate.unwrap())(newsize) as *mut c_uchar;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate.unwrap())((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        memcpy(
            newbuffer as *mut c_void,
            (*p).buffer as *const c_void,
            (*p).offset + 1,
        );
        ((*p).hooks.deallocate.unwrap())((*p).buffer as *mut c_void);
    }
    (*p).length = newsize;
    (*p).buffer = newbuffer;

    newbuffer.add((*p).offset)
}

unsafe fn update_offset(buffer: *mut printbuffer) {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    let buffer_pointer = (*buffer).buffer.add((*buffer).offset);
    (*buffer).offset += strlen(buffer_pointer as *const c_char);
}

#[inline]
fn compare_double(a: f64, b: f64) -> cJSON_bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    if (a - b).abs() <= max_val * DBL_EPSILON {
        TRUE
    } else {
        FALSE
    }
}

unsafe fn print_number(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let d = (*item).valuedouble;
    let mut length: c_int;
    let mut number_buffer: [c_uchar; 26] = [0; 26];
    let decimal_point = get_decimal_point();
    let mut test: f64 = 0.0;

    if output_buffer.is_null() {
        return FALSE;
    }

    if d.is_nan() || d.is_infinite() {
        let fmt = b"null\0".as_ptr() as *const c_char;
        length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt);
    } else if d == (*item).valueint as f64 {
        let fmt = b"%d\0".as_ptr() as *const c_char;
        length = sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            fmt,
            (*item).valueint,
        );
    } else {
        let fmt = b"%1.15g\0".as_ptr() as *const c_char;
        length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt, d);

        let scan_fmt = b"%lg\0".as_ptr() as *const c_char;
        let scanned = sscanf(
            number_buffer.as_ptr() as *const c_char,
            scan_fmt,
            &mut test as *mut f64,
        );
        if scanned != 1 || compare_double(test, d) == 0 {
            let fmt17 = b"%1.17g\0".as_ptr() as *const c_char;
            length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt17, d);
        }
    }

    if length < 0 || length > (number_buffer.len() as c_int - 1) {
        return FALSE;
    }

    let output_pointer = ensure(output_buffer, (length as usize) + 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    let mut i: usize = 0;
    while i < length as usize {
        if number_buffer[i] == decimal_point {
            *output_pointer.add(i) = b'.';
        } else {
            *output_pointer.add(i) = number_buffer[i];
        }
        i += 1;
    }
    *output_pointer.add(i) = 0;

    (*output_buffer).offset += length as usize;

    TRUE
}

unsafe fn parse_hex4(input: *const c_uchar) -> c_uint {
    let mut h: c_uint = 0;
    for i in 0..4 {
        let c = *input.add(i);
        if (b'0'..=b'9').contains(&c) {
            h += (c - b'0') as c_uint;
        } else if (b'A'..=b'F').contains(&c) {
            h += 10 + (c - b'A') as c_uint;
        } else if (b'a'..=b'f').contains(&c) {
            h += 10 + (c - b'a') as c_uint;
        } else {
            return 0;
        }
        if i < 3 {
            h <<= 4;
        }
    }
    h
}

unsafe fn utf16_literal_to_utf8(
    input_pointer: *const c_uchar,
    input_end: *const c_uchar,
    output_pointer: *mut *mut c_uchar,
) -> c_uchar {
    let mut codepoint: u64 = 0;
    let first_sequence = input_pointer;
    let mut utf8_length: c_uchar = 0;
    let mut sequence_length: c_uchar = 0;
    let mut first_byte_mark: c_uchar = 0;

    if (input_end as isize - first_sequence as isize) < 6 {
        return 0;
    }

    let first_code = parse_hex4(first_sequence.add(2));

    if (0xDC00..=0xDFFF).contains(&first_code) {
        return 0;
    }

    if (0xD800..=0xDBFF).contains(&first_code) {
        let second_sequence = first_sequence.add(6);
        sequence_length = 12;

        if (input_end as isize - second_sequence as isize) < 6 {
            return 0;
        }

        if *second_sequence != b'\\' || *second_sequence.add(1) != b'u' {
            return 0;
        }

        let second_code = parse_hex4(second_sequence.add(2));
        if !(0xDC00..=0xDFFF).contains(&second_code) {
            return 0;
        }

        codepoint =
            0x10000 + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);
    } else {
        sequence_length = 6;
        codepoint = first_code as u64;
    }

    if codepoint < 0x80 {
        utf8_length = 1;
    } else if codepoint < 0x800 {
        utf8_length = 2;
        first_byte_mark = 0xC0;
    } else if codepoint < 0x10000 {
        utf8_length = 3;
        first_byte_mark = 0xE0;
    } else if codepoint <= 0x10FFFF {
        utf8_length = 4;
        first_byte_mark = 0xF0;
    } else {
        return 0;
    }

    let mut utf8_position: c_uchar = utf8_length.wrapping_sub(1);
    while utf8_position > 0 {
        *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as c_uchar;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *(*output_pointer) = ((codepoint | first_byte_mark as u64) & 0xFF) as c_uchar;
    } else {
        *(*output_pointer) = (codepoint & 0x7F) as c_uchar;
    }

    *output_pointer = (*output_pointer).add(utf8_length as usize);

    sequence_length
}

unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut input_pointer: *const c_uchar = buffer_at_offset(input_buffer).add(1);
    let mut input_end: *const c_uchar = buffer_at_offset(input_buffer).add(1);
    let mut output_pointer: *mut c_uchar;
    let mut output: *mut c_uchar = ptr::null_mut();

    if *buffer_at_offset(input_buffer) != b'"' {
        // fail path
        if !output.is_null() {
            ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
        }
        (*input_buffer).offset = (input_pointer as usize) - ((*input_buffer).content as usize);
        return FALSE;
    }

    'block: {
        let mut allocation_length: usize = 0;
        let mut skipped_bytes: usize = 0;
        while ((input_end as usize) - ((*input_buffer).content as usize)) < (*input_buffer).length
            && *input_end != b'"'
        {
            if *input_end == b'\\' {
                if ((input_end as usize) + 1 - ((*input_buffer).content as usize))
                    >= (*input_buffer).length
                {
                    break 'block;
                }
                skipped_bytes += 1;
                input_end = input_end.add(1);
            }
            input_end = input_end.add(1);
        }
        if ((input_end as usize) - ((*input_buffer).content as usize)) >= (*input_buffer).length
            || *input_end != b'"'
        {
            break 'block;
        }

        allocation_length = ((input_end as usize) - (buffer_at_offset(input_buffer) as usize))
            - skipped_bytes;
        output =
            ((*input_buffer).hooks.allocate.unwrap())(allocation_length + 1) as *mut c_uchar;
        if output.is_null() {
            break 'block;
        }

        // ----- Copy string body / handle escapes -----
        output_pointer = output;
        while input_pointer < input_end {
            if *input_pointer != b'\\' {
                *output_pointer = *input_pointer;
                output_pointer = output_pointer.add(1);
                input_pointer = input_pointer.add(1);
            } else {
                let mut sequence_length: c_uchar = 2;
                if (input_end as isize - input_pointer as isize) < 1 {
                    // fail
                    if !output.is_null() {
                        ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
                    }
                    (*input_buffer).offset =
                        (input_pointer as usize) - ((*input_buffer).content as usize);
                    return FALSE;
                }

                let esc = *input_pointer.add(1);
                match esc {
                    b'b' => {
                        *output_pointer = 0x08;
                        output_pointer = output_pointer.add(1);
                    }
                    b'f' => {
                        *output_pointer = 0x0C;
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
                    b'"' | b'\\' | b'/' => {
                        *output_pointer = esc;
                        output_pointer = output_pointer.add(1);
                    }
                    b'u' => {
                        sequence_length =
                            utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                        if sequence_length == 0 {
                            // fail
                            if !output.is_null() {
                                ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
                            }
                            (*input_buffer).offset =
                                (input_pointer as usize) - ((*input_buffer).content as usize);
                            return FALSE;
                        }
                    }
                    _ => {
                        // fail
                        if !output.is_null() {
                            ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
                        }
                        (*input_buffer).offset =
                            (input_pointer as usize) - ((*input_buffer).content as usize);
                        return FALSE;
                    }
                }
                input_pointer = input_pointer.add(sequence_length as usize);
            }
        }

        *output_pointer = 0;

        (*item).r#type = cJSON_String;
        (*item).valuestring = output as *mut c_char;

        (*input_buffer).offset = (input_end as usize) - ((*input_buffer).content as usize);
        (*input_buffer).offset += 1;

        return TRUE;
    }

    // fail
    if !output.is_null() {
        ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
    }
    (*input_buffer).offset = (input_pointer as usize) - ((*input_buffer).content as usize);
    FALSE
}

unsafe fn print_string_ptr(input: *const c_uchar, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut output: *mut c_uchar;
    let mut output_pointer: *mut c_uchar;
    let output_length: usize;
    let mut escape_characters: usize = 0;

    if output_buffer.is_null() {
        return FALSE;
    }

    if input.is_null() {
        output = ensure(output_buffer, 3); // sizeof("\"\"") = 3
        if output.is_null() {
            return FALSE;
        }
        strcpy(output as *mut c_char, b"\"\"\0".as_ptr() as *const c_char);
        return TRUE;
    }

    let mut input_pointer = input;
    while *input_pointer != 0 {
        match *input_pointer {
            b'"' | b'\\' | 0x08 | 0x0C | b'\n' | b'\r' | b'\t' => {
                escape_characters += 1;
            }
            c if c < 32 => {
                escape_characters += 5;
            }
            _ => {}
        }
        input_pointer = input_pointer.add(1);
    }
    output_length = (input_pointer as usize - input as usize) + escape_characters;

    output = ensure(output_buffer, output_length + 3);
    if output.is_null() {
        return FALSE;
    }

    if escape_characters == 0 {
        *output = b'"';
        memcpy(
            output.add(1) as *mut c_void,
            input as *const c_void,
            output_length,
        );
        *output.add(output_length + 1) = b'"';
        *output.add(output_length + 2) = 0;
        return TRUE;
    }

    *output = b'"';
    output_pointer = output.add(1);
    let mut input_pointer = input;
    while *input_pointer != 0 {
        if *input_pointer > 31 && *input_pointer != b'"' && *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
        } else {
            *output_pointer = b'\\';
            output_pointer = output_pointer.add(1);
            match *input_pointer {
                b'\\' => *output_pointer = b'\\',
                b'"' => *output_pointer = b'"',
                0x08 => *output_pointer = b'b',
                0x0C => *output_pointer = b'f',
                b'\n' => *output_pointer = b'n',
                b'\r' => *output_pointer = b'r',
                b'\t' => *output_pointer = b't',
                _ => {
                    let fmt = b"u%04x\0".as_ptr() as *const c_char;
                    sprintf(
                        output_pointer as *mut c_char,
                        fmt,
                        *input_pointer as c_int,
                    );
                    output_pointer = output_pointer.add(4);
                }
            }
        }
        input_pointer = input_pointer.add(1);
        output_pointer = output_pointer.add(1);
    }
    *output.add(output_length + 1) = b'"';
    *output.add(output_length + 2) = 0;

    TRUE
}

unsafe fn print_string(item: *const cJSON, p: *mut printbuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const c_uchar, p)
}

unsafe fn buffer_skip_whitespace(buffer: *mut parse_buffer) -> *mut parse_buffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }

    if cannot_access_at_index(buffer, 0) {
        return buffer;
    }

    while can_access_at_index(buffer, 0) && *buffer_at_offset(buffer) <= 32 {
        (*buffer).offset += 1;
    }

    if (*buffer).offset == (*buffer).length {
        (*buffer).offset -= 1;
    }

    buffer
}

unsafe fn skip_utf8_bom(buffer: *mut parse_buffer) -> *mut parse_buffer {
    if buffer.is_null() || (*buffer).content.is_null() || (*buffer).offset != 0 {
        return ptr::null_mut();
    }

    if can_access_at_index(buffer, 4)
        && strncmp(
            buffer_at_offset(buffer) as *const c_char,
            b"\xEF\xBB\xBF\0".as_ptr() as *const c_char,
            3,
        ) == 0
    {
        (*buffer).offset += 3;
    }

    buffer
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }

    let buffer_length = strlen(value) + 1;
    cJSON_ParseWithLengthOpts(value, buffer_length, return_parse_end, require_null_terminated)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    let mut buffer = parse_buffer {
        content: ptr::null(),
        length: 0,
        offset: 0,
        depth: 0,
        hooks: internal_hooks {
            allocate: None,
            deallocate: None,
            reallocate: None,
        },
    };
    let mut item: *mut cJSON = ptr::null_mut();

    GLOBAL_ERROR.json = ptr::null();
    GLOBAL_ERROR.position = 0;

    'fail: {
        if value.is_null() || buffer_length == 0 {
            break 'fail;
        }

        buffer.content = value as *const c_uchar;
        buffer.length = buffer_length;
        buffer.offset = 0;
        buffer.hooks = GLOBAL_HOOKS;

        item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
        if item.is_null() {
            break 'fail;
        }

        let _ = skip_utf8_bom(&mut buffer);
        let _ = buffer_skip_whitespace(&mut buffer);
        if parse_value(item, &mut buffer) == 0 {
            break 'fail;
        }

        if require_null_terminated != 0 {
            let _ = buffer_skip_whitespace(&mut buffer);
            if buffer.offset >= buffer.length || *buffer_at_offset(&buffer) != 0 {
                break 'fail;
            }
        }
        if !return_parse_end.is_null() {
            *return_parse_end = buffer_at_offset(&buffer) as *const c_char;
        }

        return item;
    }

    if !item.is_null() {
        cJSON_Delete(item);
    }

    if !value.is_null() {
        let mut local_error_json = value as *const c_uchar;
        let mut local_error_position: usize = 0;

        if buffer.offset < buffer.length {
            local_error_position = buffer.offset;
        } else if buffer.length > 0 {
            local_error_position = buffer.length - 1;
        }

        if !return_parse_end.is_null() {
            *return_parse_end = local_error_json.add(local_error_position) as *const c_char;
        }

        GLOBAL_ERROR.json = local_error_json;
        GLOBAL_ERROR.position = local_error_position;
        let _ = &mut local_error_json;
    }

    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: usize,
) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

unsafe fn print_internal(
    item: *const cJSON,
    format: cJSON_bool,
    hooks: *const internal_hooks,
) -> *mut c_uchar {
    let default_buffer_size: usize = 256;
    let mut buffer = printbuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: internal_hooks {
            allocate: None,
            deallocate: None,
            reallocate: None,
        },
    };
    let mut printed: *mut c_uchar = ptr::null_mut();

    buffer.buffer = ((*hooks).allocate.unwrap())(default_buffer_size) as *mut c_uchar;
    buffer.length = default_buffer_size;
    buffer.format = format;
    buffer.hooks = *hooks;
    if buffer.buffer.is_null() {
        // fail
        return ptr::null_mut();
    }

    'block: {
        if print_value(item, &mut buffer) == 0 {
            break 'block;
        }
        update_offset(&mut buffer);

        if (*hooks).reallocate.is_some() {
            printed = ((*hooks).reallocate.unwrap())(
                buffer.buffer as *mut c_void,
                buffer.offset + 1,
            ) as *mut c_uchar;
            if printed.is_null() {
                break 'block;
            }
            buffer.buffer = ptr::null_mut();
        } else {
            printed = ((*hooks).allocate.unwrap())(buffer.offset + 1) as *mut c_uchar;
            if printed.is_null() {
                break 'block;
            }
            let to_copy = if buffer.length < buffer.offset + 1 {
                buffer.length
            } else {
                buffer.offset + 1
            };
            memcpy(
                printed as *mut c_void,
                buffer.buffer as *const c_void,
                to_copy,
            );
            *printed.add(buffer.offset) = 0;
            ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
            buffer.buffer = ptr::null_mut();
        }

        return printed;
    }

    if !buffer.buffer.is_null() {
        ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
    }
    if !printed.is_null() {
        ((*hooks).deallocate.unwrap())(printed as *mut c_void);
    }
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print_internal(item, TRUE, &raw const GLOBAL_HOOKS) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print_internal(item, FALSE, &raw const GLOBAL_HOOKS) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    fmt: cJSON_bool,
) -> *mut c_char {
    if prebuffer < 0 {
        return ptr::null_mut();
    }

    let mut p = printbuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: internal_hooks {
            allocate: None,
            deallocate: None,
            reallocate: None,
        },
    };

    p.buffer = (GLOBAL_HOOKS.allocate.unwrap())(prebuffer as usize) as *mut c_uchar;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }

    p.length = prebuffer as usize;
    p.offset = 0;
    p.noalloc = FALSE;
    p.format = fmt;
    p.hooks = GLOBAL_HOOKS;

    if print_value(item, &mut p) == 0 {
        (GLOBAL_HOOKS.deallocate.unwrap())(p.buffer as *mut c_void);
        return ptr::null_mut();
    }

    p.buffer as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintPreallocated(
    item: *mut cJSON,
    buffer: *mut c_char,
    length: c_int,
    format: cJSON_bool,
) -> cJSON_bool {
    if length < 0 || buffer.is_null() {
        return FALSE;
    }

    let mut p = printbuffer {
        buffer: buffer as *mut c_uchar,
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: TRUE,
        format,
        hooks: GLOBAL_HOOKS,
    };

    print_value(item, &mut p)
}

unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return FALSE;
    }

    if can_read(input_buffer, 4)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"null\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).r#type = cJSON_NULL;
        (*input_buffer).offset += 4;
        return TRUE;
    }
    if can_read(input_buffer, 5)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"false\0".as_ptr() as *const c_char,
            5,
        ) == 0
    {
        (*item).r#type = cJSON_False;
        (*input_buffer).offset += 5;
        return TRUE;
    }
    if can_read(input_buffer, 4)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"true\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).r#type = cJSON_True;
        (*item).valueint = 1;
        (*input_buffer).offset += 4;
        return TRUE;
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'"' {
        return parse_string(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) {
        let c = *buffer_at_offset(input_buffer);
        if c == b'-' || (b'0'..=b'9').contains(&c) {
            return parse_number(item, input_buffer);
        }
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'[' {
        return parse_array(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'{' {
        return parse_object(item, input_buffer);
    }

    FALSE
}

unsafe fn print_value(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let output: *mut c_uchar;

    if item.is_null() || output_buffer.is_null() {
        return FALSE;
    }

    match (*item).r#type & 0xFF {
        cJSON_NULL => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return FALSE;
            }
            strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            TRUE
        }
        cJSON_False => {
            output = ensure(output_buffer, 6);
            if output.is_null() {
                return FALSE;
            }
            strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            TRUE
        }
        cJSON_True => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return FALSE;
            }
            strcpy(output as *mut c_char, b"true\0".as_ptr() as *const c_char);
            TRUE
        }
        cJSON_Number => print_number(item, output_buffer),
        cJSON_Raw => {
            if (*item).valuestring.is_null() {
                return FALSE;
            }
            let raw_length = strlen((*item).valuestring) + 1;
            output = ensure(output_buffer, raw_length);
            if output.is_null() {
                return FALSE;
            }
            memcpy(
                output as *mut c_void,
                (*item).valuestring as *const c_void,
                raw_length,
            );
            TRUE
        }
        cJSON_String => print_string(item, output_buffer),
        cJSON_Array => print_array(item, output_buffer),
        cJSON_Object => print_object(item, output_buffer),
        _ => FALSE,
    }
}

unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return FALSE;
    }
    (*input_buffer).depth += 1;

    'fail: {
        if *buffer_at_offset(input_buffer) != b'[' {
            break 'fail;
        }

        (*input_buffer).offset += 1;
        let _ = buffer_skip_whitespace(input_buffer);
        if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b']' {
            // empty array - go to success
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).r#type = cJSON_Array;
            (*item).child = head;
            (*input_buffer).offset += 1;
            return TRUE;
        }

        if cannot_access_at_index(input_buffer, 0) {
            (*input_buffer).offset -= 1;
            break 'fail;
        }

        (*input_buffer).offset -= 1;

        loop {
            let new_item = cJSON_New_Item(&raw const (*input_buffer).hooks);
            if new_item.is_null() {
                break 'fail;
            }

            if head.is_null() {
                current_item = new_item;
                head = new_item;
            } else {
                (*current_item).next = new_item;
                (*new_item).prev = current_item;
                current_item = new_item;
            }

            (*input_buffer).offset += 1;
            let _ = buffer_skip_whitespace(input_buffer);
            if parse_value(current_item, input_buffer) == 0 {
                break 'fail;
            }
            let _ = buffer_skip_whitespace(input_buffer);

            if !(can_access_at_index(input_buffer, 0)
                && *buffer_at_offset(input_buffer) == b',')
            {
                break;
            }
        }

        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b']' {
            break 'fail;
        }

        (*input_buffer).depth -= 1;
        if !head.is_null() {
            (*head).prev = current_item;
        }
        (*item).r#type = cJSON_Array;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return TRUE;
    }

    if !head.is_null() {
        cJSON_Delete(head);
    }
    FALSE
}

unsafe fn print_array(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut output_pointer: *mut c_uchar;
    let mut length: usize;
    let mut current_element = (*item).child;

    if output_buffer.is_null() {
        return FALSE;
    }

    output_pointer = ensure(output_buffer, 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    *output_pointer = b'[';
    (*output_buffer).offset += 1;
    (*output_buffer).depth += 1;

    while !current_element.is_null() {
        if print_value(current_element, output_buffer) == 0 {
            return FALSE;
        }
        update_offset(output_buffer);
        if !(*current_element).next.is_null() {
            length = if (*output_buffer).format != 0 { 2 } else { 1 };
            output_pointer = ensure(output_buffer, length + 1);
            if output_pointer.is_null() {
                return FALSE;
            }
            *output_pointer = b',';
            output_pointer = output_pointer.add(1);
            if (*output_buffer).format != 0 {
                *output_pointer = b' ';
                output_pointer = output_pointer.add(1);
            }
            *output_pointer = 0;
            (*output_buffer).offset += length;
        }
        current_element = (*current_element).next;
    }

    output_pointer = ensure(output_buffer, 2);
    if output_pointer.is_null() {
        return FALSE;
    }
    *output_pointer = b']';
    output_pointer = output_pointer.add(1);
    *output_pointer = 0;
    (*output_buffer).depth -= 1;

    TRUE
}

unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return FALSE;
    }
    (*input_buffer).depth += 1;

    'fail: {
        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'{' {
            break 'fail;
        }

        (*input_buffer).offset += 1;
        let _ = buffer_skip_whitespace(input_buffer);
        if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'}' {
            // success - empty object
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).r#type = cJSON_Object;
            (*item).child = head;
            (*input_buffer).offset += 1;
            return TRUE;
        }

        if cannot_access_at_index(input_buffer, 0) {
            (*input_buffer).offset -= 1;
            break 'fail;
        }

        (*input_buffer).offset -= 1;

        loop {
            let new_item = cJSON_New_Item(&raw const (*input_buffer).hooks);
            if new_item.is_null() {
                break 'fail;
            }

            if head.is_null() {
                current_item = new_item;
                head = new_item;
            } else {
                (*current_item).next = new_item;
                (*new_item).prev = current_item;
                current_item = new_item;
            }

            if cannot_access_at_index(input_buffer, 1) {
                break 'fail;
            }

            (*input_buffer).offset += 1;
            let _ = buffer_skip_whitespace(input_buffer);
            if parse_string(current_item, input_buffer) == 0 {
                break 'fail;
            }
            let _ = buffer_skip_whitespace(input_buffer);

            (*current_item).string = (*current_item).valuestring;
            (*current_item).valuestring = ptr::null_mut();

            if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b':' {
                break 'fail;
            }

            (*input_buffer).offset += 1;
            let _ = buffer_skip_whitespace(input_buffer);
            if parse_value(current_item, input_buffer) == 0 {
                break 'fail;
            }
            let _ = buffer_skip_whitespace(input_buffer);

            if !(can_access_at_index(input_buffer, 0)
                && *buffer_at_offset(input_buffer) == b',')
            {
                break;
            }
        }

        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'}' {
            break 'fail;
        }

        (*input_buffer).depth -= 1;
        if !head.is_null() {
            (*head).prev = current_item;
        }
        (*item).r#type = cJSON_Object;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return TRUE;
    }

    if !head.is_null() {
        cJSON_Delete(head);
    }
    FALSE
}

unsafe fn print_object(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut output_pointer: *mut c_uchar;
    let mut length: usize;
    let mut current_item = (*item).child;

    if output_buffer.is_null() {
        return FALSE;
    }

    length = if (*output_buffer).format != 0 { 2 } else { 1 };
    output_pointer = ensure(output_buffer, length + 1);
    if output_pointer.is_null() {
        return FALSE;
    }

    *output_pointer = b'{';
    output_pointer = output_pointer.add(1);
    (*output_buffer).depth += 1;
    if (*output_buffer).format != 0 {
        *output_pointer = b'\n';
        // output_pointer = output_pointer.add(1);  // not used after
    }
    (*output_buffer).offset += length;

    while !current_item.is_null() {
        if (*output_buffer).format != 0 {
            output_pointer = ensure(output_buffer, (*output_buffer).depth);
            if output_pointer.is_null() {
                return FALSE;
            }
            for _ in 0..(*output_buffer).depth {
                *output_pointer = b'\t';
                output_pointer = output_pointer.add(1);
            }
            (*output_buffer).offset += (*output_buffer).depth;
        }

        if print_string_ptr((*current_item).string as *const c_uchar, output_buffer) == 0 {
            return FALSE;
        }
        update_offset(output_buffer);

        length = if (*output_buffer).format != 0 { 2 } else { 1 };
        output_pointer = ensure(output_buffer, length);
        if output_pointer.is_null() {
            return FALSE;
        }
        *output_pointer = b':';
        output_pointer = output_pointer.add(1);
        if (*output_buffer).format != 0 {
            *output_pointer = b'\t';
            // output_pointer = output_pointer.add(1);
        }
        (*output_buffer).offset += length;

        if print_value(current_item, output_buffer) == 0 {
            return FALSE;
        }
        update_offset(output_buffer);

        length = (if (*output_buffer).format != 0 { 1 } else { 0 })
            + (if !(*current_item).next.is_null() { 1 } else { 0 });
        output_pointer = ensure(output_buffer, length + 1);
        if output_pointer.is_null() {
            return FALSE;
        }
        if !(*current_item).next.is_null() {
            *output_pointer = b',';
            output_pointer = output_pointer.add(1);
        }

        if (*output_buffer).format != 0 {
            *output_pointer = b'\n';
            output_pointer = output_pointer.add(1);
        }
        *output_pointer = 0;
        (*output_buffer).offset += length;

        current_item = (*current_item).next;
    }

    let needed = if (*output_buffer).format != 0 {
        (*output_buffer).depth + 1
    } else {
        2
    };
    output_pointer = ensure(output_buffer, needed);
    if output_pointer.is_null() {
        return FALSE;
    }
    if (*output_buffer).format != 0 {
        for _ in 0..((*output_buffer).depth - 1) {
            *output_pointer = b'\t';
            output_pointer = output_pointer.add(1);
        }
    }
    *output_pointer = b'}';
    output_pointer = output_pointer.add(1);
    *output_pointer = 0;
    (*output_buffer).depth -= 1;

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() {
        return 0;
    }

    let mut child = (*array).child;
    let mut size: usize = 0;

    while !child.is_null() {
        size += 1;
        child = (*child).next;
    }

    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    if array.is_null() {
        return ptr::null_mut();
    }

    let mut current_child = (*array).child;
    while !current_child.is_null() && index > 0 {
        index -= 1;
        current_child = (*current_child).next;
    }

    current_child
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }
    get_array_item(array, index as usize)
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if object.is_null() || name.is_null() {
        return ptr::null_mut();
    }

    let mut current_element = (*object).child;
    if case_sensitive != 0 {
        while !current_element.is_null()
            && !(*current_element).string.is_null()
            && strcmp(name, (*current_element).string) != 0
        {
            current_element = (*current_element).next;
        }
    } else {
        while !current_element.is_null()
            && case_insensitive_strcmp(
                name as *const c_uchar,
                (*current_element).string as *const c_uchar,
            ) != 0
        {
            current_element = (*current_element).next;
        }
    }

    if current_element.is_null() || (*current_element).string.is_null() {
        return ptr::null_mut();
    }

    current_element
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    if !cJSON_GetObjectItem(object, string).is_null() {
        1
    } else {
        0
    }
}

unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {
    (*prev).next = item;
    (*item).prev = prev;
}

unsafe fn create_reference(item: *const cJSON, hooks: *const internal_hooks) -> *mut cJSON {
    if item.is_null() {
        return ptr::null_mut();
    }

    let reference = cJSON_New_Item(hooks);
    if reference.is_null() {
        return ptr::null_mut();
    }

    memcpy(
        reference as *mut c_void,
        item as *const c_void,
        core::mem::size_of::<cJSON>(),
    );
    (*reference).string = ptr::null_mut();
    (*reference).r#type |= cJSON_IsReference;
    (*reference).next = ptr::null_mut();
    (*reference).prev = ptr::null_mut();
    reference
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item {
        return FALSE;
    }

    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else if !(*child).prev.is_null() {
        suffix_object((*child).prev, item);
        (*(*array).child).prev = item;
    }

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    add_item_to_array(array, item)
}

unsafe fn cast_away_const(string: *const c_void) -> *mut c_void {
    string as *mut c_void
}

unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    hooks: *const internal_hooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    let new_key: *mut c_char;
    let new_type: c_int;

    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return FALSE;
    }

    if constant_key != 0 {
        new_key = cast_away_const(string as *const c_void) as *mut c_char;
        new_type = (*item).r#type | cJSON_StringIsConst;
    } else {
        new_key = cJSON_strdup(string as *const c_uchar, hooks) as *mut c_char;
        if new_key.is_null() {
            return FALSE;
        }
        new_type = (*item).r#type & !cJSON_StringIsConst;
    }

    if ((*item).r#type & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
        ((*hooks).deallocate.unwrap())((*item).string as *mut c_void);
    }

    (*item).string = new_key;
    (*item).r#type = new_type;

    add_item_to_array(object, item)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &raw const GLOBAL_HOOKS, FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &raw const GLOBAL_HOOKS, TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return FALSE;
    }
    add_item_to_array(array, create_reference(item, &raw const GLOBAL_HOOKS))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() {
        return FALSE;
    }
    add_item_to_object(
        object,
        string,
        create_reference(item, &raw const GLOBAL_HOOKS),
        &raw const GLOBAL_HOOKS,
        FALSE,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return null;
    }
    cJSON_Delete(null);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let item = cJSON_CreateTrue();
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let item = cJSON_CreateFalse();
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    let item = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    let item = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    let item = cJSON_CreateString(string);
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    let item = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let item = cJSON_CreateObject();
    if add_item_to_object(object, name, item, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return item;
    }
    cJSON_Delete(item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let array = cJSON_CreateArray();
    if add_item_to_object(object, name, array, &raw const GLOBAL_HOOKS, FALSE) != 0 {
        return array;
    }
    cJSON_Delete(array);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    if parent.is_null()
        || item.is_null()
        || (item != (*parent).child && (*item).prev.is_null())
    {
        return ptr::null_mut();
    }

    if item != (*parent).child {
        (*(*item).prev).next = (*item).next;
    }
    if !(*item).next.is_null() {
        (*(*item).next).prev = (*item).prev;
    }

    if item == (*parent).child {
        (*parent).child = (*item).next;
    } else if (*item).next.is_null() {
        (*(*parent).child).prev = (*item).prev;
    }

    (*item).prev = ptr::null_mut();
    (*item).next = ptr::null_mut();

    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(array: *mut cJSON, which: c_int) -> *mut cJSON {
    if which < 0 {
        return ptr::null_mut();
    }
    cJSON_DetachItemViaPointer(array, get_array_item(array, which as usize))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItem(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItemCaseSensitive(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return FALSE;
    }

    let after_inserted = get_array_item(array, which as usize);
    if after_inserted.is_null() {
        return add_item_to_array(array, newitem);
    }

    if after_inserted != (*array).child && (*after_inserted).prev.is_null() {
        return FALSE;
    }

    (*newitem).next = after_inserted;
    (*newitem).prev = (*after_inserted).prev;
    (*after_inserted).prev = newitem;
    if after_inserted == (*array).child {
        (*array).child = newitem;
    } else {
        (*(*newitem).prev).next = newitem;
    }
    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    if parent.is_null()
        || (*parent).child.is_null()
        || replacement.is_null()
        || item.is_null()
    {
        return FALSE;
    }

    if replacement == item {
        return TRUE;
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
        if !(*replacement).prev.is_null() {
            (*(*replacement).prev).next = replacement;
        }
        if (*replacement).next.is_null() {
            (*(*parent).child).prev = replacement;
        }
    }

    (*item).next = ptr::null_mut();
    (*item).prev = ptr::null_mut();
    cJSON_Delete(item);

    TRUE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 {
        return FALSE;
    }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() {
        return FALSE;
    }

    if ((*replacement).r#type & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string =
        cJSON_strdup(string as *const c_uchar, &raw const GLOBAL_HOOKS) as *mut c_char;
    if (*replacement).string.is_null() {
        return FALSE;
    }

    (*replacement).r#type &= !cJSON_StringIsConst;

    cJSON_ReplaceItemViaPointer(object, get_object_item(object, string, case_sensitive), replacement)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, FALSE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, TRUE)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_NULL;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_True;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_False;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = if boolean != 0 { cJSON_True } else { cJSON_False };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Number;
        (*item).valuedouble = num;

        if num >= INT_MAX as f64 {
            (*item).valueint = INT_MAX;
        } else if num <= INT_MIN as f64 {
            (*item).valueint = INT_MIN;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_String;
        (*item).valuestring =
            cJSON_strdup(string as *const c_uchar, &raw const GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_String | cJSON_IsReference;
        (*item).valuestring = cast_away_const(string as *const c_void) as *mut c_char;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Object | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Array | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Raw;
        (*item).valuestring =
            cJSON_strdup(raw as *const c_uchar, &raw const GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Array;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).r#type = cJSON_Object;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }

    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a = cJSON_CreateArray();

    let mut i: usize = 0;
    while !a.is_null() && i < (count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const f32,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }

    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a = cJSON_CreateArray();

    let mut i: usize = 0;
    while !a.is_null() && i < (count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }

    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a = cJSON_CreateArray();

    let mut i: usize = 0;
    while !a.is_null() && i < (count as usize) {
        n = cJSON_CreateNumber(*numbers.add(i));
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || strings.is_null() {
        return ptr::null_mut();
    }

    let mut n: *mut cJSON = ptr::null_mut();
    let mut p: *mut cJSON = ptr::null_mut();
    let a = cJSON_CreateArray();

    let mut i: usize = 0;
    while !a.is_null() && i < (count as usize) {
        n = cJSON_CreateString(*strings.add(i));
        if n.is_null() {
            cJSON_Delete(a);
            return ptr::null_mut();
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: usize,
    recurse: cJSON_bool,
) -> *mut cJSON {
    let mut newitem: *mut cJSON = ptr::null_mut();
    let mut child: *mut cJSON;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();

    'fail: {
        if item.is_null() {
            break 'fail;
        }
        newitem = cJSON_New_Item(&raw const GLOBAL_HOOKS);
        if newitem.is_null() {
            break 'fail;
        }

        (*newitem).r#type = (*item).r#type & !cJSON_IsReference;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring =
                cJSON_strdup((*item).valuestring as *const c_uchar, &raw const GLOBAL_HOOKS)
                    as *mut c_char;
            if (*newitem).valuestring.is_null() {
                break 'fail;
            }
        }
        if !(*item).string.is_null() {
            (*newitem).string = if ((*item).r#type & cJSON_StringIsConst) != 0 {
                (*item).string
            } else {
                cJSON_strdup((*item).string as *const c_uchar, &raw const GLOBAL_HOOKS)
                    as *mut c_char
            };
            if (*newitem).string.is_null() {
                break 'fail;
            }
        }
        if recurse == 0 {
            return newitem;
        }
        child = (*item).child;
        while !child.is_null() {
            if depth >= CJSON_CIRCULAR_LIMIT {
                break 'fail;
            }
            newchild = cJSON_Duplicate_rec(child, depth + 1, TRUE);
            if newchild.is_null() {
                break 'fail;
            }
            if !next.is_null() {
                (*next).next = newchild;
                (*newchild).prev = next;
                next = newchild;
            } else {
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

    if !newitem.is_null() {
        cJSON_Delete(newitem);
    }
    ptr::null_mut()
}

unsafe fn skip_oneline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2); // skip "//"

    while *(*input) != 0 {
        if *(*input) == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2); // skip "/*"

    while *(*input) != 0 {
        if *(*input) == b'*' as c_char && *(*input).add(1) == b'/' as c_char {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: *mut *mut c_char, output: *mut *mut c_char) {
    *(*output) = *(*input);
    *input = (*input).add(1);
    *output = (*output).add(1);

    while *(*input) != 0 {
        *(*output) = *(*input);
        if *(*input) == b'"' as c_char {
            *(*output) = b'"' as c_char;
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if *(*input) == b'\\' as c_char && *(*input).add(1) == b'"' as c_char {
            *(*output).add(1) = *(*input).add(1);
            *input = (*input).add(1);
            *output = (*output).add(1);
        }
        *input = (*input).add(1);
        *output = (*output).add(1);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    if json.is_null() {
        return;
    }

    let mut into = json;
    let mut json_p = json;

    while *json_p != 0 {
        match *json_p as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => {
                json_p = json_p.add(1);
            }
            b'/' => {
                if *json_p.add(1) == b'/' as c_char {
                    skip_oneline_comment(&mut json_p);
                } else if *json_p.add(1) == b'*' as c_char {
                    skip_multiline_comment(&mut json_p);
                } else {
                    json_p = json_p.add(1);
                }
            }
            b'"' => {
                minify_string(&mut json_p, &mut into);
            }
            _ => {
                *into = *json_p;
                json_p = json_p.add(1);
                into = into.add(1);
            }
        }
    }

    *into = 0;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_Invalid {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_False {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xff) == cJSON_True {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & (cJSON_True | cJSON_False)) != 0 {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_NULL {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_Number {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_String {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_Array {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_Object {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return FALSE;
    }
    if ((*item).r#type & 0xFF) == cJSON_Raw {
        TRUE
    } else {
        FALSE
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).r#type & 0xFF) != ((*b).r#type & 0xFF) {
        return FALSE;
    }

    match (*a).r#type & 0xFF {
        x if x == cJSON_False
            || x == cJSON_True
            || x == cJSON_NULL
            || x == cJSON_Number
            || x == cJSON_String
            || x == cJSON_Raw
            || x == cJSON_Array
            || x == cJSON_Object => {}
        _ => return FALSE,
    }

    if a == b {
        return TRUE;
    }

    match (*a).r#type & 0xFF {
        x if x == cJSON_False || x == cJSON_True || x == cJSON_NULL => TRUE,
        x if x == cJSON_Number => {
            if compare_double((*a).valuedouble, (*b).valuedouble) != 0 {
                TRUE
            } else {
                FALSE
            }
        }
        x if x == cJSON_String || x == cJSON_Raw => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                return FALSE;
            }
            if strcmp((*a).valuestring, (*b).valuestring) == 0 {
                TRUE
            } else {
                FALSE
            }
        }
        x if x == cJSON_Array => {
            let mut a_element = (*a).child;
            let mut b_element = (*b).child;
            while !a_element.is_null() && !b_element.is_null() {
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return FALSE;
                }
                a_element = (*a_element).next;
                b_element = (*b_element).next;
            }
            if a_element != b_element {
                return FALSE;
            }
            TRUE
        }
        x if x == cJSON_Object => {
            let mut a_element = (*a).child;
            while !a_element.is_null() {
                let b_element = get_object_item(b, (*a_element).string, case_sensitive);
                if b_element.is_null() {
                    return FALSE;
                }
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return FALSE;
                }
                a_element = (*a_element).next;
            }
            let mut b_element = (*b).child;
            while !b_element.is_null() {
                let a_element = get_object_item(a, (*b_element).string, case_sensitive);
                if a_element.is_null() {
                    return FALSE;
                }
                if cJSON_Compare(b_element, a_element, case_sensitive) == 0 {
                    return FALSE;
                }
                b_element = (*b_element).next;
            }
            TRUE
        }
        _ => FALSE,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    (GLOBAL_HOOKS.allocate.unwrap())(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    (GLOBAL_HOOKS.deallocate.unwrap())(object);
    // Note: assignment to local 'object = NULL' in C; intentionally a no-op.
}
