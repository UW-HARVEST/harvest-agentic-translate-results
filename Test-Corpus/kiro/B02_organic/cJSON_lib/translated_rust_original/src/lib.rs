#![allow(
    non_camel_case_types,
    non_snake_case,
    non_upper_case_globals,
    clippy::missing_safety_doc,
    unused_assignments,
    unused_mut
)]

use std::ffi::{c_char, c_double, c_float, c_int, c_uchar, c_uint, c_void};
use std::ptr;

// cJSON Types
const CJSON_INVALID: c_int = 0;
const CJSON_FALSE: c_int = 1 << 0;
const CJSON_TRUE: c_int = 1 << 1;
const CJSON_NULL: c_int = 1 << 2;
const CJSON_NUMBER: c_int = 1 << 3;
const CJSON_STRING: c_int = 1 << 4;
const CJSON_ARRAY: c_int = 1 << 5;
const CJSON_OBJECT: c_int = 1 << 6;
const CJSON_RAW: c_int = 1 << 7;
const CJSON_IS_REFERENCE: c_int = 256;
const CJSON_STRING_IS_CONST: c_int = 512;
const CJSON_NESTING_LIMIT: usize = 1000;
const CJSON_CIRCULAR_LIMIT: usize = 10000;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn tolower(c: c_int) -> c_int;
    fn fabs(x: c_double) -> c_double;
    fn printf(format: *const c_char, ...) -> c_int;
    fn exit(status: c_int) -> !;
}

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

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

type cJSON_bool = c_int;

#[derive(Clone, Copy)]
struct internal_hooks {
    allocate: unsafe extern "C" fn(usize) -> *mut c_void,
    deallocate: unsafe extern "C" fn(*mut c_void),
    reallocate: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
}

#[repr(C)]
struct error {
    json: *const c_uchar,
    position: usize,
}

struct parse_buffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: internal_hooks,
}

struct printbuffer {
    buffer: *mut c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: internal_hooks,
}

static mut global_error: error = error {
    json: ptr::null(),
    position: 0,
};

static mut global_hooks: internal_hooks = internal_hooks {
    allocate: malloc,
    deallocate: free,
    reallocate: Some(realloc),
};

// Record struct for test driver
#[repr(C)]
pub struct record {
    pub precision: *const c_char,
    pub lat: c_double,
    pub lon: c_double,
    pub address: *const c_char,
    pub city: *const c_char,
    pub state: *const c_char,
    pub zip: *const c_char,
    pub country: *const c_char,
}

// Helper macros as inline functions
#[inline]
unsafe fn can_read(buffer: *const parse_buffer, size: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + size) <= (*buffer).length
}

#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

#[inline]
unsafe fn cannot_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !can_access_at_index(buffer, index)
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

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
    let copy = ((*hooks).allocate)(length) as *mut c_uchar;
    if copy.is_null() {
        return ptr::null_mut();
    }
    memcpy(copy as *mut c_void, string as *const c_void, length);
    copy
}

unsafe fn get_decimal_point() -> c_uchar {
    b'.'
}

unsafe fn cJSON_New_Item(hooks: *const internal_hooks) -> *mut cJSON {
    let node = ((*hooks).allocate)(std::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        memset(node as *mut c_void, 0, std::mem::size_of::<cJSON>());
    }
    node
}

// ============ Public API functions ============

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    (global_error.json.add(global_error.position)) as *const c_char
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
    static mut VERSION: [c_char; 15] = [0; 15];
    sprintf(
        VERSION.as_mut_ptr(),
        b"%i.%i.%i\0".as_ptr() as *const c_char,
        1 as c_int,
        7 as c_int,
        19 as c_int,
    );
    VERSION.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        global_hooks.allocate = malloc;
        global_hooks.deallocate = free;
        global_hooks.reallocate = Some(realloc);
        return;
    }
    global_hooks.allocate = malloc;
    if let Some(malloc_fn) = (*hooks).malloc_fn {
        global_hooks.allocate = malloc_fn;
    }
    global_hooks.deallocate = free;
    if let Some(free_fn) = (*hooks).free_fn {
        global_hooks.deallocate = free_fn;
    }
    global_hooks.reallocate = None;
    if global_hooks.allocate as usize == malloc as usize
        && global_hooks.deallocate as usize == free as usize
    {
        global_hooks.reallocate = Some(realloc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let mut next: *mut cJSON;
    while !item.is_null() {
        next = (*item).next;
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & CJSON_IS_REFERENCE) == 0 && !(*item).valuestring.is_null() {
            (global_hooks.deallocate)((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
            (global_hooks.deallocate)((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        (global_hooks.deallocate)(item as *mut c_void);
        item = next;
    }
}

// ============ parse_number ============
unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    let mut i: usize = 0;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = 0;

    while can_access_at_index(input_buffer, i) {
        let ch = *buffer_at_offset(input_buffer).add(i);
        match ch {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = 1;
            }
            _ => break,
        }
        i += 1;
    }

    let number_c_string =
        ((*input_buffer).hooks.allocate)(number_string_length + 1) as *mut c_uchar;
    if number_c_string.is_null() {
        return 0;
    }
    memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = 0;

    let decimal_point = get_decimal_point();
    if has_decimal_point != 0 {
        for j in 0..number_string_length {
            if *number_c_string.add(j) == b'.' {
                *number_c_string.add(j) = decimal_point;
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = strtod(
        number_c_string as *const c_char,
        &mut after_end,
    );
    if number_c_string as *mut c_char == after_end {
        ((*input_buffer).hooks.deallocate)(number_c_string as *mut c_void);
        return 0;
    }

    (*item).valuedouble = number;
    if number >= c_int::MAX as c_double {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = number as c_int;
    }
    (*item).type_ = CJSON_NUMBER;

    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    ((*input_buffer).hooks.deallocate)(number_c_string as *mut c_void);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= c_int::MAX as c_double {
        (*object).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*object).valueint = c_int::MIN;
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
        || ((*object).type_ & CJSON_STRING) == 0
        || ((*object).type_ & CJSON_IS_REFERENCE) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    let v1_len = strlen(valuestring);
    let v2_len = strlen((*object).valuestring);
    if v1_len <= v2_len {
        if !((valuestring as usize + v1_len) < (*object).valuestring as usize
            || ((*object).valuestring as usize + v2_len) < valuestring as usize)
        {
            return ptr::null_mut();
        }
        strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cJSON_strdup(valuestring as *const c_uchar, &global_hooks) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

// ============ printbuffer helpers ============
unsafe fn ensure(p: *mut printbuffer, needed: usize) -> *mut c_uchar {
    if p.is_null() || (*p).buffer.is_null() {
        return ptr::null_mut();
    }
    if (*p).length > 0 && (*p).offset >= (*p).length {
        return ptr::null_mut();
    }
    if needed > c_int::MAX as usize {
        return ptr::null_mut();
    }
    let needed = needed + (*p).offset + 1;
    if needed <= (*p).length {
        return (*p).buffer.add((*p).offset);
    }
    if (*p).noalloc != 0 {
        return ptr::null_mut();
    }
    let newsize: usize;
    if needed > (c_int::MAX as usize / 2) {
        if needed <= c_int::MAX as usize {
            newsize = c_int::MAX as usize;
        } else {
            return ptr::null_mut();
        }
    } else {
        newsize = needed * 2;
    }

    if let Some(reallocate) = (*p).hooks.reallocate {
        let newbuffer = reallocate((*p).buffer as *mut c_void, newsize) as *mut c_uchar;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate)((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        (*p).length = newsize;
        (*p).buffer = newbuffer;
    } else {
        let newbuffer = ((*p).hooks.allocate)(newsize) as *mut c_uchar;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate)((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        memcpy(
            newbuffer as *mut c_void,
            (*p).buffer as *const c_void,
            (*p).offset + 1,
        );
        ((*p).hooks.deallocate)((*p).buffer as *mut c_void);
        (*p).length = newsize;
        (*p).buffer = newbuffer;
    }
    (*p).buffer.add((*p).offset)
}

unsafe fn update_offset(buffer: *mut printbuffer) {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    let buffer_pointer = (*buffer).buffer.add((*buffer).offset);
    (*buffer).offset += strlen(buffer_pointer as *const c_char);
}

unsafe fn compare_double(a: c_double, b: c_double) -> cJSON_bool {
    let max_val = if fabs(a) > fabs(b) { fabs(a) } else { fabs(b) };
    if fabs(a - b) <= max_val * f64::EPSILON {
        1
    } else {
        0
    }
}

// ============ print_number ============
unsafe fn print_number(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let d = (*item).valuedouble;
    let mut number_buffer = [0u8; 26];
    let decimal_point = get_decimal_point();
    let length: c_int;

    if d.is_nan() || d.is_infinite() {
        length = sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"null\0".as_ptr() as *const c_char,
        );
    } else if d == (*item).valueint as c_double {
        length = sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"%d\0".as_ptr() as *const c_char,
            (*item).valueint,
        );
    } else {
        length = sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"%1.15g\0".as_ptr() as *const c_char,
            d,
        );
        let mut test: c_double = 0.0;
        if sscanf(
            number_buffer.as_ptr() as *const c_char,
            b"%lg\0".as_ptr() as *const c_char,
            &mut test as *mut c_double,
        ) != 1
            || compare_double(test, d) == 0
        {
            let _ = sprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                b"%1.17g\0".as_ptr() as *const c_char,
                d,
            );
            // reassign length
        }
    }

    // recalculate length after potential rewrite
    let length = strlen(number_buffer.as_ptr() as *const c_char) as c_int;

    if length < 0 || length > (number_buffer.len() as c_int - 1) {
        return 0;
    }

    let output_pointer = ensure(output_buffer, length as usize + 1);
    if output_pointer.is_null() {
        return 0;
    }

    for i in 0..length as usize {
        if number_buffer[i] == decimal_point {
            *output_pointer.add(i) = b'.';
        } else {
            *output_pointer.add(i) = number_buffer[i];
        }
    }
    *output_pointer.add(length as usize) = 0;
    (*output_buffer).offset += length as usize;
    1
}

// ============ parse_hex4 ============
unsafe fn parse_hex4(input: *const c_uchar) -> c_uint {
    let mut h: c_uint = 0;
    for i in 0..4usize {
        let ch = *input.add(i);
        if ch >= b'0' && ch <= b'9' {
            h += (ch - b'0') as c_uint;
        } else if ch >= b'A' && ch <= b'F' {
            h += (10 + ch - b'A') as c_uint;
        } else if ch >= b'a' && ch <= b'f' {
            h += (10 + ch - b'a') as c_uint;
        } else {
            return 0;
        }
        if i < 3 {
            h <<= 4;
        }
    }
    h
}

// ============ utf16_literal_to_utf8 ============
unsafe fn utf16_literal_to_utf8(
    input_pointer: *const c_uchar,
    input_end: *const c_uchar,
    output_pointer: *mut *mut c_uchar,
) -> c_uchar {
    let first_sequence = input_pointer;
    if (input_end as usize - first_sequence as usize) < 6 {
        return 0;
    }
    let first_code = parse_hex4(first_sequence.add(2));
    if first_code >= 0xDC00 && first_code <= 0xDFFF {
        return 0;
    }

    let mut codepoint: u64;
    let sequence_length: c_uchar;

    if first_code >= 0xD800 && first_code <= 0xDBFF {
        let second_sequence = first_sequence.add(6);
        sequence_length = 12;
        if (input_end as usize - second_sequence as usize) < 6 {
            return 0;
        }
        if *second_sequence != b'\\' || *second_sequence.add(1) != b'u' {
            return 0;
        }
        let second_code = parse_hex4(second_sequence.add(2));
        if second_code < 0xDC00 || second_code > 0xDFFF {
            return 0;
        }
        codepoint = 0x10000 + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);
    } else {
        sequence_length = 6;
        codepoint = first_code as u64;
    }

    let utf8_length: c_uchar;
    let mut first_byte_mark: c_uchar = 0;

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

    let mut utf8_position = utf8_length - 1;
    while utf8_position > 0 {
        *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as c_uchar;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *(*output_pointer).add(0) = ((codepoint | first_byte_mark as u64) & 0xFF) as c_uchar;
    } else {
        *(*output_pointer).add(0) = (codepoint & 0x7F) as c_uchar;
    }
    *output_pointer = (*output_pointer).add(utf8_length as usize);
    sequence_length
}

// ============ parse_string ============
unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut input_pointer = buffer_at_offset(input_buffer).add(1);
    let mut input_end = buffer_at_offset(input_buffer).add(1);
    let mut output_pointer: *mut c_uchar;
    let mut output: *mut c_uchar = ptr::null_mut();

    if *buffer_at_offset(input_buffer) != b'\"' {
        // goto fail
        if !input_pointer.is_null() {
            (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
        }
        return 0;
    }

    // calculate approximate size
    let mut skipped_bytes: usize = 0;
    while (input_end as usize - (*input_buffer).content as usize) < (*input_buffer).length
        && *input_end != b'\"'
    {
        if *input_end == b'\\' {
            if (input_end.add(1) as usize - (*input_buffer).content as usize)
                >= (*input_buffer).length
            {
                // goto fail
                if !input_pointer.is_null() {
                    (*input_buffer).offset =
                        input_pointer as usize - (*input_buffer).content as usize;
                }
                return 0;
            }
            skipped_bytes += 1;
            input_end = input_end.add(1);
        }
        input_end = input_end.add(1);
    }
    if (input_end as usize - (*input_buffer).content as usize) >= (*input_buffer).length
        || *input_end != b'\"'
    {
        if !input_pointer.is_null() {
            (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
        }
        return 0;
    }

    let allocation_length =
        (input_end as usize - buffer_at_offset(input_buffer) as usize) - skipped_bytes;
    output = ((*input_buffer).hooks.allocate)(allocation_length + 1) as *mut c_uchar;
    if output.is_null() {
        if !input_pointer.is_null() {
            (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
        }
        return 0;
    }

    output_pointer = output;
    while (input_pointer as usize) < (input_end as usize) {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        } else {
            let mut sequence_length: c_uchar = 2;
            if (input_end as usize - input_pointer as usize) < 1 {
                // goto fail
                if !output.is_null() {
                    ((*input_buffer).hooks.deallocate)(output as *mut c_void);
                }
                if !input_pointer.is_null() {
                    (*input_buffer).offset =
                        input_pointer as usize - (*input_buffer).content as usize;
                }
                return 0;
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
                b'u' => {
                    sequence_length =
                        utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                    if sequence_length == 0 {
                        if !output.is_null() {
                            ((*input_buffer).hooks.deallocate)(output as *mut c_void);
                        }
                        if !input_pointer.is_null() {
                            (*input_buffer).offset =
                                input_pointer as usize - (*input_buffer).content as usize;
                        }
                        return 0;
                    }
                }
                _ => {
                    if !output.is_null() {
                        ((*input_buffer).hooks.deallocate)(output as *mut c_void);
                    }
                    if !input_pointer.is_null() {
                        (*input_buffer).offset =
                            input_pointer as usize - (*input_buffer).content as usize;
                    }
                    return 0;
                }
            }
            input_pointer = input_pointer.add(sequence_length as usize);
        }
    }

    *output_pointer = 0;
    (*item).type_ = CJSON_STRING;
    (*item).valuestring = output as *mut c_char;
    (*input_buffer).offset = input_end as usize - (*input_buffer).content as usize;
    (*input_buffer).offset += 1;
    1
}

// ============ print_string_ptr ============
unsafe fn print_string_ptr(
    input: *const c_uchar,
    output_buffer: *mut printbuffer,
) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    if input.is_null() {
        let output = ensure(output_buffer, 3);
        if output.is_null() {
            return 0;
        }
        strcpy(output as *mut c_char, b"\"\"\0".as_ptr() as *const c_char);
        return 1;
    }

    let mut escape_characters: usize = 0;
    let mut input_pointer = input;
    while *input_pointer != 0 {
        match *input_pointer {
            b'\"' | b'\\' | b'\x08' | b'\x0C' | b'\n' | b'\r' | b'\t' => {
                escape_characters += 1;
            }
            _ => {
                if *input_pointer < 32 {
                    escape_characters += 5;
                }
            }
        }
        input_pointer = input_pointer.add(1);
    }
    let output_length = (input_pointer as usize - input as usize) + escape_characters;

    let output = ensure(output_buffer, output_length + 3);
    if output.is_null() {
        return 0;
    }

    if escape_characters == 0 {
        *output = b'\"';
        memcpy(
            output.add(1) as *mut c_void,
            input as *const c_void,
            output_length,
        );
        *output.add(output_length + 1) = b'\"';
        *output.add(output_length + 2) = 0;
        return 1;
    }

    *output = b'\"';
    let mut output_ptr = output.add(1);
    input_pointer = input;
    while *input_pointer != 0 {
        if *input_pointer > 31 && *input_pointer != b'\"' && *input_pointer != b'\\' {
            *output_ptr = *input_pointer;
        } else {
            *output_ptr = b'\\';
            output_ptr = output_ptr.add(1);
            match *input_pointer {
                b'\\' => *output_ptr = b'\\',
                b'\"' => *output_ptr = b'\"',
                b'\x08' => *output_ptr = b'b',
                b'\x0C' => *output_ptr = b'f',
                b'\n' => *output_ptr = b'n',
                b'\r' => *output_ptr = b'r',
                b'\t' => *output_ptr = b't',
                _ => {
                    sprintf(
                        output_ptr as *mut c_char,
                        b"u%04x\0".as_ptr() as *const c_char,
                        *input_pointer as c_int,
                    );
                    output_ptr = output_ptr.add(4);
                }
            }
        }
        input_pointer = input_pointer.add(1);
        output_ptr = output_ptr.add(1);
    }
    *output.add(output_length + 1) = b'\"';
    *output.add(output_length + 2) = 0;
    1
}

unsafe fn print_string(item: *const cJSON, p: *mut printbuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const c_uchar, p)
}

// ============ buffer_skip_whitespace, skip_utf8_bom ============
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

// ============ cJSON_ParseWithOpts ============
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
            allocate: malloc,
            deallocate: free,
            reallocate: None,
        },
    };

    global_error.json = ptr::null();
    global_error.position = 0;

    if value.is_null() || buffer_length == 0 {
        // goto fail
        if !value.is_null() {
            let mut local_error = error {
                json: value as *const c_uchar,
                position: 0,
            };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end =
                    local_error.json.add(local_error.position) as *const c_char;
            }
            global_error = local_error;
        }
        return ptr::null_mut();
    }

    buffer.content = value as *const c_uchar;
    buffer.length = buffer_length;
    buffer.offset = 0;
    buffer.hooks = global_hooks;

    let item = cJSON_New_Item(&global_hooks);
    if item.is_null() {
        if !value.is_null() {
            let mut local_error = error {
                json: value as *const c_uchar,
                position: 0,
            };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end =
                    local_error.json.add(local_error.position) as *const c_char;
            }
            global_error = local_error;
        }
        return ptr::null_mut();
    }

    if parse_value(item, buffer_skip_whitespace(skip_utf8_bom(&mut buffer))) == 0 {
        cJSON_Delete(item);
        if !value.is_null() {
            let mut local_error = error {
                json: value as *const c_uchar,
                position: 0,
            };
            if buffer.offset < buffer.length {
                local_error.position = buffer.offset;
            } else if buffer.length > 0 {
                local_error.position = buffer.length - 1;
            }
            if !return_parse_end.is_null() {
                *return_parse_end =
                    local_error.json.add(local_error.position) as *const c_char;
            }
            global_error = local_error;
        }
        return ptr::null_mut();
    }

    if require_null_terminated != 0 {
        buffer_skip_whitespace(&mut buffer);
        if buffer.offset >= buffer.length || *buffer.content.add(buffer.offset) != 0 {
            cJSON_Delete(item);
            if !value.is_null() {
                let mut local_error = error {
                    json: value as *const c_uchar,
                    position: 0,
                };
                if buffer.offset < buffer.length {
                    local_error.position = buffer.offset;
                } else if buffer.length > 0 {
                    local_error.position = buffer.length - 1;
                }
                if !return_parse_end.is_null() {
                    *return_parse_end =
                        local_error.json.add(local_error.position) as *const c_char;
                }
                global_error = local_error;
            }
            return ptr::null_mut();
        }
    }
    if !return_parse_end.is_null() {
        *return_parse_end = buffer.content.add(buffer.offset) as *const c_char;
    }
    item
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

// ============ print ============
unsafe fn print(item: *const cJSON, format: cJSON_bool, hooks: *const internal_hooks) -> *mut c_uchar {
    let default_buffer_size: usize = 256;
    let mut buffer = printbuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: *hooks,
    };

    buffer.buffer = ((*hooks).allocate)(default_buffer_size) as *mut c_uchar;
    buffer.length = default_buffer_size;
    if buffer.buffer.is_null() {
        return ptr::null_mut();
    }

    if print_value(item, &mut buffer) == 0 {
        ((*hooks).deallocate)(buffer.buffer as *mut c_void);
        return ptr::null_mut();
    }
    update_offset(&mut buffer);

    let printed: *mut c_uchar;
    if let Some(reallocate) = (*hooks).reallocate {
        printed = reallocate(buffer.buffer as *mut c_void, buffer.offset + 1) as *mut c_uchar;
        if printed.is_null() {
            ((*hooks).deallocate)(buffer.buffer as *mut c_void);
            return ptr::null_mut();
        }
    } else {
        printed = ((*hooks).allocate)(buffer.offset + 1) as *mut c_uchar;
        if printed.is_null() {
            ((*hooks).deallocate)(buffer.buffer as *mut c_void);
            return ptr::null_mut();
        }
        let copy_len = if buffer.length < buffer.offset + 1 {
            buffer.length
        } else {
            buffer.offset + 1
        };
        memcpy(
            printed as *mut c_void,
            buffer.buffer as *const c_void,
            copy_len,
        );
        *printed.add(buffer.offset) = 0;
        ((*hooks).deallocate)(buffer.buffer as *mut c_void);
    }
    printed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print(item, 1, &global_hooks) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print(item, 0, &global_hooks) as *mut c_char
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
        format: fmt,
        hooks: global_hooks,
    };
    p.buffer = (global_hooks.allocate)(prebuffer as usize) as *mut c_uchar;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }
    p.length = prebuffer as usize;

    if print_value(item, &mut p) == 0 {
        (global_hooks.deallocate)(p.buffer as *mut c_void);
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
        return 0;
    }
    let mut p = printbuffer {
        buffer: buffer as *mut c_uchar,
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: global_hooks,
    };
    print_value(item as *const cJSON, &mut p)
}

// ============ parse_value ============
unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    // null
    if can_read(input_buffer, 4)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"null\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = CJSON_NULL;
        (*input_buffer).offset += 4;
        return 1;
    }
    // false
    if can_read(input_buffer, 5)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"false\0".as_ptr() as *const c_char,
            5,
        ) == 0
    {
        (*item).type_ = CJSON_FALSE;
        (*input_buffer).offset += 5;
        return 1;
    }
    // true
    if can_read(input_buffer, 4)
        && strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"true\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = CJSON_TRUE;
        (*item).valueint = 1;
        (*input_buffer).offset += 4;
        return 1;
    }
    // string
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'\"' {
        return parse_string(item, input_buffer);
    }
    // number
    if can_access_at_index(input_buffer, 0) {
        let ch = *buffer_at_offset(input_buffer);
        if ch == b'-' || (ch >= b'0' && ch <= b'9') {
            return parse_number(item, input_buffer);
        }
    }
    // array
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'[' {
        return parse_array(item, input_buffer);
    }
    // object
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'{' {
        return parse_object(item, input_buffer);
    }
    0
}

// ============ print_value ============
unsafe fn print_value(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    if item.is_null() || output_buffer.is_null() {
        return 0;
    }
    match (*item).type_ & 0xFF {
        x if x == CJSON_NULL => {
            let output = ensure(output_buffer, 5);
            if output.is_null() { return 0; }
            strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            1
        }
        x if x == CJSON_FALSE => {
            let output = ensure(output_buffer, 6);
            if output.is_null() { return 0; }
            strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            1
        }
        x if x == CJSON_TRUE => {
            let output = ensure(output_buffer, 5);
            if output.is_null() { return 0; }
            strcpy(output as *mut c_char, b"true\0".as_ptr() as *const c_char);
            1
        }
        x if x == CJSON_NUMBER => print_number(item, output_buffer),
        x if x == CJSON_RAW => {
            if (*item).valuestring.is_null() { return 0; }
            let raw_length = strlen((*item).valuestring) + 1;
            let output = ensure(output_buffer, raw_length);
            if output.is_null() { return 0; }
            memcpy(output as *mut c_void, (*item).valuestring as *const c_void, raw_length);
            1
        }
        x if x == CJSON_STRING => print_string(item, output_buffer),
        x if x == CJSON_ARRAY => print_array(item, output_buffer),
        x if x == CJSON_OBJECT => print_object(item, output_buffer),
        _ => 0,
    }
}

// ============ parse_array ============
unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if *buffer_at_offset(input_buffer) != b'[' {
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset += 1;
    buffer_skip_whitespace(input_buffer);
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b']' {
        // success - empty array
        (*input_buffer).depth -= 1;
        if !head.is_null() { (*head).prev = current_item; }
        (*item).type_ = CJSON_ARRAY;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return 1;
    }

    if cannot_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset -= 1;

    loop {
        let new_item = cJSON_New_Item(&(*input_buffer).hooks);
        if new_item.is_null() {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current_item = new_item;
        } else {
            (*current_item).next = new_item;
            (*new_item).prev = current_item;
            current_item = new_item;
        }

        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_value(current_item, input_buffer) == 0 {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b']' {
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    // success
    (*input_buffer).depth -= 1;
    if !head.is_null() { (*head).prev = current_item; }
    (*item).type_ = CJSON_ARRAY;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}

// ============ print_array ============
unsafe fn print_array(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let mut current_element = (*item).child;

    let output_pointer = ensure(output_buffer, 1);
    if output_pointer.is_null() { return 0; }
    *output_pointer = b'[';
    (*output_buffer).offset += 1;
    (*output_buffer).depth += 1;

    while !current_element.is_null() {
        if print_value(current_element, output_buffer) == 0 { return 0; }
        update_offset(output_buffer);
        if !(*current_element).next.is_null() {
            let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
            let op = ensure(output_buffer, length + 1);
            if op.is_null() { return 0; }
            *op = b',';
            if (*output_buffer).format != 0 {
                *op.add(1) = b' ';
            }
            *op.add(length) = 0;
            (*output_buffer).offset += length;
        }
        current_element = (*current_element).next;
    }

    let op = ensure(output_buffer, 2);
    if op.is_null() { return 0; }
    *op = b']';
    *op.add(1) = 0;
    (*output_buffer).depth -= 1;
    1
}

// ============ parse_object ============
unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'{' {
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset += 1;
    buffer_skip_whitespace(input_buffer);
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'}' {
        // success - empty object
        (*input_buffer).depth -= 1;
        if !head.is_null() { (*head).prev = current_item; }
        (*item).type_ = CJSON_OBJECT;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return 1;
    }

    if cannot_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset -= 1;

    loop {
        let new_item = cJSON_New_Item(&(*input_buffer).hooks);
        if new_item.is_null() {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current_item = new_item;
        } else {
            (*current_item).next = new_item;
            (*new_item).prev = current_item;
            current_item = new_item;
        }

        if cannot_access_at_index(input_buffer, 1) {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }

        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_string(current_item, input_buffer) == 0 {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        (*current_item).string = (*current_item).valuestring;
        (*current_item).valuestring = ptr::null_mut();

        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b':' {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }

        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_value(current_item, input_buffer) == 0 {
            if !head.is_null() { cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'}' {
        if !head.is_null() { cJSON_Delete(head); }
        return 0;
    }

    // success
    (*input_buffer).depth -= 1;
    if !head.is_null() { (*head).prev = current_item; }
    (*item).type_ = CJSON_OBJECT;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}

// ============ print_object ============
unsafe fn print_object(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let mut current_item = (*item).child;

    let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
    let mut output_pointer = ensure(output_buffer, length + 1);
    if output_pointer.is_null() { return 0; }
    *output_pointer = b'{';
    (*output_buffer).depth += 1;
    if (*output_buffer).format != 0 {
        *output_pointer.add(1) = b'\n';
    }
    (*output_buffer).offset += length;

    while !current_item.is_null() {
        if (*output_buffer).format != 0 {
            output_pointer = ensure(output_buffer, (*output_buffer).depth);
            if output_pointer.is_null() { return 0; }
            for i in 0..(*output_buffer).depth {
                *output_pointer.add(i) = b'\t';
            }
            (*output_buffer).offset += (*output_buffer).depth;
        }

        // print key
        if print_string_ptr((*current_item).string as *const c_uchar, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
        output_pointer = ensure(output_buffer, length);
        if output_pointer.is_null() { return 0; }
        *output_pointer = b':';
        if (*output_buffer).format != 0 {
            *output_pointer.add(1) = b'\t';
        }
        (*output_buffer).offset += length;

        // print value
        if print_value(current_item, output_buffer) == 0 { return 0; }
        update_offset(output_buffer);

        // print comma if not last
        let length = (if (*output_buffer).format != 0 { 1usize } else { 0usize })
            + (if !(*current_item).next.is_null() { 1usize } else { 0usize });
        output_pointer = ensure(output_buffer, length + 1);
        if output_pointer.is_null() { return 0; }
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

    let ensure_size = if (*output_buffer).format != 0 {
        (*output_buffer).depth + 1
    } else {
        2
    };
    output_pointer = ensure(output_buffer, ensure_size);
    if output_pointer.is_null() { return 0; }
    if (*output_buffer).format != 0 {
        for i in 0..((*output_buffer).depth - 1) {
            *output_pointer.add(i) = b'\t';
        }
        output_pointer = output_pointer.add((*output_buffer).depth - 1);
    }
    *output_pointer = b'}';
    *output_pointer.add(1) = 0;
    (*output_buffer).depth -= 1;
    1
}

// ============ GetArraySize, GetArrayItem, GetObjectItem ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() { return 0; }
    let mut child = (*array).child;
    let mut size: usize = 0;
    while !child.is_null() {
        size += 1;
        child = (*child).next;
    }
    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: usize) -> *mut cJSON {
    if array.is_null() { return ptr::null_mut(); }
    let mut current_child = (*array).child;
    while !current_child.is_null() && index > 0 {
        index -= 1;
        current_child = (*current_child).next;
    }
    current_child
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 { return ptr::null_mut(); }
    get_array_item(array, index as usize)
}

unsafe fn get_object_item(
    object: *const cJSON,
    name: *const c_char,
    case_sensitive: cJSON_bool,
) -> *mut cJSON {
    if object.is_null() || name.is_null() { return ptr::null_mut(); }
    let mut current_element = (*object).child;
    if case_sensitive != 0 {
        while !current_element.is_null() && !(*current_element).string.is_null()
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
    get_object_item(object, string, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    if cJSON_GetObjectItem(object, string).is_null() { 0 } else { 1 }
}

// ============ suffix_object, create_reference, add_item_to_array ============
unsafe fn suffix_object(prev: *mut cJSON, item: *mut cJSON) {
    (*prev).next = item;
    (*item).prev = prev;
}

unsafe fn create_reference(item: *const cJSON, hooks: *const internal_hooks) -> *mut cJSON {
    if item.is_null() { return ptr::null_mut(); }
    let reference = cJSON_New_Item(hooks);
    if reference.is_null() { return ptr::null_mut(); }
    memcpy(
        reference as *mut c_void,
        item as *const c_void,
        std::mem::size_of::<cJSON>(),
    );
    (*reference).string = ptr::null_mut();
    (*reference).type_ |= CJSON_IS_REFERENCE;
    (*reference).next = ptr::null_mut();
    (*reference).prev = ptr::null_mut();
    reference
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item { return 0; }
    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else {
        if !(*child).prev.is_null() {
            suffix_object((*child).prev, item);
            (*array).child = (*array).child; // keep child
            (*(*array).child).prev = item;
        }
    }
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    add_item_to_array(array, item)
}

unsafe fn add_item_to_object(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
    hooks: *const internal_hooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object as *mut cJSON == item {
        return 0;
    }
    let new_key: *mut c_char;
    let new_type: c_int;
    if constant_key != 0 {
        new_key = string as *mut c_char;
        new_type = (*item).type_ | CJSON_STRING_IS_CONST;
    } else {
        new_key = cJSON_strdup(string as *const c_uchar, hooks) as *mut c_char;
        if new_key.is_null() { return 0; }
        new_type = (*item).type_ & !CJSON_STRING_IS_CONST;
    }
    if ((*item).type_ & CJSON_STRING_IS_CONST) == 0 && !(*item).string.is_null() {
        ((*hooks).deallocate)((*item).string as *mut c_void);
    }
    (*item).string = new_key;
    (*item).type_ = new_type;
    add_item_to_array(object, item)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &global_hooks, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &global_hooks, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() { return 0; }
    add_item_to_array(array, create_reference(item, &global_hooks))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() { return 0; }
    add_item_to_object(
        object,
        string,
        create_reference(item, &global_hooks),
        &global_hooks,
        0,
    )
}

// ============ AddXxxToObject helpers ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let null_item = cJSON_CreateNull();
    if add_item_to_object(object, name, null_item, &global_hooks, 0) != 0 {
        return null_item;
    }
    cJSON_Delete(null_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let true_item = cJSON_CreateTrue();
    if add_item_to_object(object, name, true_item, &global_hooks, 0) != 0 {
        return true_item;
    }
    cJSON_Delete(true_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let false_item = cJSON_CreateFalse();
    if add_item_to_object(object, name, false_item, &global_hooks, 0) != 0 {
        return false_item;
    }
    cJSON_Delete(false_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    let bool_item = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, bool_item, &global_hooks, 0) != 0 {
        return bool_item;
    }
    cJSON_Delete(bool_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    let number_item = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, number_item, &global_hooks, 0) != 0 {
        return number_item;
    }
    cJSON_Delete(number_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    let string_item = cJSON_CreateString(string);
    if add_item_to_object(object, name, string_item, &global_hooks, 0) != 0 {
        return string_item;
    }
    cJSON_Delete(string_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    let raw_item = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, raw_item, &global_hooks, 0) != 0 {
        return raw_item;
    }
    cJSON_Delete(raw_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let object_item = cJSON_CreateObject();
    if add_item_to_object(object, name, object_item, &global_hooks, 0) != 0 {
        return object_item;
    }
    cJSON_Delete(object_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let array = cJSON_CreateArray();
    if add_item_to_object(object, name, array, &global_hooks, 0) != 0 {
        return array;
    }
    cJSON_Delete(array);
    ptr::null_mut()
}

// ============ DetachItem, DeleteItem ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
) -> *mut cJSON {
    if parent.is_null() || item.is_null() || (item != (*parent).child && (*item).prev.is_null()) {
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
pub unsafe extern "C" fn cJSON_DetachItemFromArray(
    array: *mut cJSON,
    which: c_int,
) -> *mut cJSON {
    if which < 0 { return ptr::null_mut(); }
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

// ============ InsertItemInArray, ReplaceItem ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 || newitem.is_null() { return 0; }
    let after_inserted = get_array_item(array, which as usize);
    if after_inserted.is_null() {
        return add_item_to_array(array, newitem);
    }
    if after_inserted != (*array).child && (*after_inserted).prev.is_null() {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
    if parent.is_null() || (*parent).child.is_null() || replacement.is_null() || item.is_null() {
        return 0;
    }
    if replacement == item { return 1; }

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
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 { return 0; }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() { return 0; }
    if ((*replacement).type_ & CJSON_STRING_IS_CONST) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string =
        cJSON_strdup(string as *const c_uchar, &global_hooks) as *mut c_char;
    if (*replacement).string.is_null() { return 0; }
    (*replacement).type_ &= !CJSON_STRING_IS_CONST;
    cJSON_ReplaceItemViaPointer(object, get_object_item(object, string, case_sensitive), replacement)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 1)
}

// ============ Create basic types ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() { (*item).type_ = CJSON_NULL; }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() { (*item).type_ = CJSON_TRUE; }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() { (*item).type_ = CJSON_FALSE; }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = if boolean != 0 { CJSON_TRUE } else { CJSON_FALSE };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_NUMBER;
        (*item).valuedouble = num;
        if num >= c_int::MAX as c_double {
            (*item).valueint = c_int::MAX;
        } else if num <= c_int::MIN as c_double {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_STRING;
        (*item).valuestring =
            cJSON_strdup(string as *const c_uchar, &global_hooks) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_STRING | CJSON_IS_REFERENCE;
        (*item).valuestring = string as *mut c_char;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_OBJECT | CJSON_IS_REFERENCE;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_ARRAY | CJSON_IS_REFERENCE;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() {
        (*item).type_ = CJSON_RAW;
        (*item).valuestring =
            cJSON_strdup(raw as *const c_uchar, &global_hooks) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() { (*item).type_ = CJSON_ARRAY; }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item(&global_hooks);
    if !item.is_null() { (*item).type_ = CJSON_OBJECT; }
    item
}

// ============ Create Arrays ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON;
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const c_float,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON;
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i) as c_double);
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || numbers.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON;
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateNumber(*numbers.add(i));
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || strings.is_null() { return ptr::null_mut(); }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON;
    for i in 0..count as usize {
        if a.is_null() { break; }
        n = cJSON_CreateString(*strings.add(i));
        if n.is_null() { cJSON_Delete(a); return ptr::null_mut(); }
        if i == 0 {
            (*a).child = n;
        } else {
            suffix_object(p, n);
        }
        p = n;
    }
    if !a.is_null() && !(*a).child.is_null() {
        (*(*a).child).prev = p;
    }
    a
}

// ============ Duplicate ============
unsafe fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: usize,
    recurse: cJSON_bool,
) -> *mut cJSON {
    if item.is_null() { return ptr::null_mut(); }
    let newitem = cJSON_New_Item(&global_hooks);
    if newitem.is_null() { return ptr::null_mut(); }

    (*newitem).type_ = (*item).type_ & (!CJSON_IS_REFERENCE);
    (*newitem).valueint = (*item).valueint;
    (*newitem).valuedouble = (*item).valuedouble;
    if !(*item).valuestring.is_null() {
        (*newitem).valuestring =
            cJSON_strdup((*item).valuestring as *const c_uchar, &global_hooks) as *mut c_char;
        if (*newitem).valuestring.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
    }
    if !(*item).string.is_null() {
        if ((*item).type_ & CJSON_STRING_IS_CONST) != 0 {
            (*newitem).string = (*item).string;
        } else {
            (*newitem).string =
                cJSON_strdup((*item).string as *const c_uchar, &global_hooks) as *mut c_char;
        }
        if (*newitem).string.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
    }
    if recurse == 0 { return newitem; }

    let mut child = (*item).child;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();
    while !child.is_null() {
        if depth >= CJSON_CIRCULAR_LIMIT {
            cJSON_Delete(newitem);
            return ptr::null_mut();
        }
        newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
        if newchild.is_null() {
            cJSON_Delete(newitem);
            return ptr::null_mut();
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
    newitem
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

// ============ Minify helpers ============
unsafe fn skip_oneline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2);
    while *(*input) != 0 {
        if *(*input) == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2);
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
        if *(*input) == b'\"' as c_char {
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if *(*input) == b'\\' as c_char && *(*input).add(1) == b'\"' as c_char {
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
    if json.is_null() { return; }
    let mut into = json;
    let mut json = json;
    while *json != 0 {
        match *json as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => {
                json = json.add(1);
            }
            b'/' => {
                if *json.add(1) == b'/' as c_char {
                    skip_oneline_comment(&mut json);
                } else if *json.add(1) == b'*' as c_char {
                    skip_multiline_comment(&mut json);
                } else {
                    json = json.add(1);
                }
            }
            b'\"' => {
                minify_string(&mut json, &mut into);
            }
            _ => {
                *into = *json;
                json = json.add(1);
                into = into.add(1);
            }
        }
    }
    *into = 0;
}

// ============ IsXxx type checks ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_INVALID { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_FALSE { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_TRUE { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & (CJSON_TRUE | CJSON_FALSE)) != 0 { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_NULL { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_NUMBER { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_STRING { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_ARRAY { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_OBJECT { 1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    if item.is_null() { return 0; }
    if ((*item).type_ & 0xFF) == CJSON_RAW { 1 } else { 0 }
}

// ============ Compare ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
        return 0;
    }
    // check if type is valid
    match (*a).type_ & 0xFF {
        x if x == CJSON_FALSE
            || x == CJSON_TRUE
            || x == CJSON_NULL
            || x == CJSON_NUMBER
            || x == CJSON_STRING
            || x == CJSON_RAW
            || x == CJSON_ARRAY
            || x == CJSON_OBJECT => {}
        _ => return 0,
    }
    if a == b { return 1; }

    match (*a).type_ & 0xFF {
        x if x == CJSON_FALSE || x == CJSON_TRUE || x == CJSON_NULL => 1,
        x if x == CJSON_NUMBER => {
            if compare_double((*a).valuedouble, (*b).valuedouble) != 0 { 1 } else { 0 }
        }
        x if x == CJSON_STRING || x == CJSON_RAW => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() { return 0; }
            if strcmp((*a).valuestring, (*b).valuestring) == 0 { 1 } else { 0 }
        }
        x if x == CJSON_ARRAY => {
            let mut a_element = (*a).child;
            let mut b_element = (*b).child;
            while !a_element.is_null() && !b_element.is_null() {
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 { return 0; }
                a_element = (*a_element).next;
                b_element = (*b_element).next;
            }
            if a_element != b_element { return 0; }
            1
        }
        x if x == CJSON_OBJECT => {
            let mut a_element = (*a).child;
            while !a_element.is_null() {
                let b_element =
                    get_object_item(b, (*a_element).string, case_sensitive);
                if b_element.is_null() { return 0; }
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 { return 0; }
                a_element = (*a_element).next;
            }
            let mut b_element = (*b).child;
            while !b_element.is_null() {
                let a_el =
                    get_object_item(a, (*b_element).string, case_sensitive);
                if a_el.is_null() { return 0; }
                if cJSON_Compare(b_element, a_el, case_sensitive) == 0 { return 0; }
                b_element = (*b_element).next;
            }
            1
        }
        _ => 0,
    }
}

// ============ malloc/free wrappers ============
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    (global_hooks.allocate)(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    (global_hooks.deallocate)(object);
}

// ============ test driver ============
unsafe fn print_preallocated(root: *mut cJSON) -> c_int {
    let out = cJSON_Print(root as *const cJSON);
    let len = strlen(out) + 5;
    let buf = malloc(len) as *mut c_char;
    if buf.is_null() {
        printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        exit(1);
    }
    let len_fail = strlen(out);
    let buf_fail = malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        printf(b"Failed to allocate memory.\n\0".as_ptr() as *const c_char);
        exit(1);
    }

    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        printf(b"cJSON_PrintPreallocated failed!\n\0".as_ptr() as *const c_char);
        if strcmp(out, buf) != 0 {
            printf(
                b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0".as_ptr()
                    as *const c_char,
            );
            printf(b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char, out);
            printf(
                b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
                buf,
            );
        }
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    printf(b"%s\n\0".as_ptr() as *const c_char, buf);

    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        printf(
            b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0".as_ptr()
                as *const c_char,
        );
        printf(b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char, out);
        printf(
            b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char,
            buf_fail,
        );
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    free(out as *mut c_void);
    free(buf_fail as *mut c_void);
    free(buf as *mut c_void);
    0
}

unsafe fn create_objects(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) {
    // Video
    let mut root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"name\0".as_ptr() as *const c_char,
        cJSON_CreateString(b"Jack (\"Bee\") Nimble\0".as_ptr() as *const c_char),
    );
    let fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"format\0".as_ptr() as *const c_char, fmt);
    cJSON_AddStringToObject(fmt, b"type\0".as_ptr() as *const c_char, b"rect\0".as_ptr() as *const c_char);
    cJSON_AddNumberToObject(fmt, b"width\0".as_ptr() as *const c_char, 1920.0);
    cJSON_AddNumberToObject(fmt, b"height\0".as_ptr() as *const c_char, 1080.0);
    cJSON_AddFalseToObject(fmt, b"interlace\0".as_ptr() as *const c_char);
    cJSON_AddNumberToObject(fmt, b"frame rate\0".as_ptr() as *const c_char, 24.0);
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);

    // Days of the week
    root = cJSON_CreateStringArray(strings, 7);
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);

    // Matrix
    root = cJSON_CreateArray();
    for i in 0..3 {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray((*numbers.add(i)).as_ptr(), 3),
        );
    }
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);

    // Gallery
    root = cJSON_CreateObject();
    let img = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"Image\0".as_ptr() as *const c_char, img);
    cJSON_AddNumberToObject(img, b"Width\0".as_ptr() as *const c_char, 800.0);
    cJSON_AddNumberToObject(img, b"Height\0".as_ptr() as *const c_char, 600.0);
    cJSON_AddStringToObject(
        img,
        b"Title\0".as_ptr() as *const c_char,
        b"View from 15th Floor\0".as_ptr() as *const c_char,
    );
    let thm = cJSON_CreateObject();
    cJSON_AddItemToObject(img, b"Thumbnail\0".as_ptr() as *const c_char, thm);
    cJSON_AddStringToObject(
        thm,
        b"Url\0".as_ptr() as *const c_char,
        b"http:/*www.example.com/image/481989943\0".as_ptr() as *const c_char,
    );
    cJSON_AddNumberToObject(thm, b"Height\0".as_ptr() as *const c_char, 125.0);
    cJSON_AddStringToObject(thm, b"Width\0".as_ptr() as *const c_char, b"100\0".as_ptr() as *const c_char);
    cJSON_AddItemToObject(
        img,
        b"IDs\0".as_ptr() as *const c_char,
        cJSON_CreateIntArray(ids, 4),
    );
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);

    // Records
    root = cJSON_CreateArray();
    for i in 0..2usize {
        let fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        cJSON_AddStringToObject(fld, b"precision\0".as_ptr() as *const c_char, (*fields.add(i)).precision);
        cJSON_AddNumberToObject(fld, b"Latitude\0".as_ptr() as *const c_char, (*fields.add(i)).lat);
        cJSON_AddNumberToObject(fld, b"Longitude\0".as_ptr() as *const c_char, (*fields.add(i)).lon);
        cJSON_AddStringToObject(fld, b"Address\0".as_ptr() as *const c_char, (*fields.add(i)).address);
        cJSON_AddStringToObject(fld, b"City\0".as_ptr() as *const c_char, (*fields.add(i)).city);
        cJSON_AddStringToObject(fld, b"State\0".as_ptr() as *const c_char, (*fields.add(i)).state);
        cJSON_AddStringToObject(fld, b"Zip\0".as_ptr() as *const c_char, (*fields.add(i)).zip);
        cJSON_AddStringToObject(fld, b"Country\0".as_ptr() as *const c_char, (*fields.add(i)).country);
    }
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);

    // Infinity number
    root = cJSON_CreateObject();
    let zero: f64 = 0.0;
    cJSON_AddNumberToObject(root, b"number\0".as_ptr() as *const c_char, 1.0 / zero);
    if print_preallocated(root) != 0 { cJSON_Delete(root); exit(1); }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) -> c_int {
    printf(
        b"Version: %s\n\0".as_ptr() as *const c_char,
        cJSON_Version(),
    );
    create_objects(strings, numbers, ids, fields);
    0
}
