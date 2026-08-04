// Direct Rust translation of cJSON.c using libc and raw pointers to preserve
// C semantics.

use libc::{c_char, c_double, c_int, c_void, size_t};
use std::ffi::CString;
use std::ptr;

// ---- Type tag constants ----
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

pub const CJSON_NESTING_LIMIT: size_t = 1000;
pub const CJSON_CIRCULAR_LIMIT: size_t = 10000;

pub type cJSON_bool = c_int;

// ---- cJSON structure ----
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
    pub malloc_fn: Option<unsafe extern "C" fn(size_t) -> *mut c_void>,
    pub free_fn: Option<unsafe extern "C" fn(*mut c_void)>,
}

// ---- Internal hooks ----
type AllocFn = unsafe extern "C" fn(size_t) -> *mut c_void;
type FreeFn = unsafe extern "C" fn(*mut c_void);
type ReallocFn = unsafe extern "C" fn(*mut c_void, size_t) -> *mut c_void;

#[repr(C)]
#[derive(Clone, Copy)]
pub struct internal_hooks {
    pub allocate: Option<AllocFn>,
    pub deallocate: Option<FreeFn>,
    pub reallocate: Option<ReallocFn>,
}

unsafe extern "C" fn internal_malloc(size: size_t) -> *mut c_void {
    libc::malloc(size)
}
unsafe extern "C" fn internal_free(ptr: *mut c_void) {
    libc::free(ptr)
}
unsafe extern "C" fn internal_realloc(ptr: *mut c_void, size: size_t) -> *mut c_void {
    libc::realloc(ptr, size)
}

static mut global_hooks: internal_hooks = internal_hooks {
    allocate: Some(internal_malloc),
    deallocate: Some(internal_free),
    reallocate: Some(internal_realloc),
};

// ---- Error tracking ----
#[repr(C)]
#[derive(Clone, Copy)]
struct error_t {
    json: *const u8,
    position: size_t,
}

static mut global_error: error_t = error_t {
    json: ptr::null(),
    position: 0,
};

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    (global_error.json as *const c_char).offset(global_error.position as isize)
}

// ---- Version ----
static mut version_buf: [c_char; 15] = [0; 15];

#[no_mangle]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    let s = format!(
        "{}.{}.{}",
        CJSON_VERSION_MAJOR, CJSON_VERSION_MINOR, CJSON_VERSION_PATCH
    );
    let bytes = s.as_bytes();
    for i in 0..bytes.len() {
        version_buf[i] = bytes[i] as c_char;
    }
    version_buf[bytes.len()] = 0;
    version_buf.as_ptr()
}

// ---- Helper: tolower ----
unsafe fn tolower_byte(c: u8) -> u8 {
    libc::tolower(c as c_int) as u8
}

unsafe fn case_insensitive_strcmp(string1: *const u8, string2: *const u8) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }
    if string1 == string2 {
        return 0;
    }
    let mut s1 = string1;
    let mut s2 = string2;
    while tolower_byte(*s1) == tolower_byte(*s2) {
        if *s1 == 0 {
            return 0;
        }
        s1 = s1.add(1);
        s2 = s2.add(1);
    }
    tolower_byte(*s1) as c_int - tolower_byte(*s2) as c_int
}

// ---- strdup ----
unsafe fn cJSON_strdup(string: *const u8, hooks: *const internal_hooks) -> *mut u8 {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = libc::strlen(string as *const c_char) + 1;
    let copy = ((*hooks).allocate.unwrap())(length) as *mut u8;
    if copy.is_null() {
        return ptr::null_mut();
    }
    libc::memcpy(copy as *mut c_void, string as *const c_void, length);
    copy
}

// ---- InitHooks ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        global_hooks.allocate = Some(internal_malloc);
        global_hooks.deallocate = Some(internal_free);
        global_hooks.reallocate = Some(internal_realloc);
        return;
    }
    global_hooks.allocate = Some(internal_malloc);
    if let Some(f) = (*hooks).malloc_fn {
        global_hooks.allocate = Some(f);
    }
    global_hooks.deallocate = Some(internal_free);
    if let Some(f) = (*hooks).free_fn {
        global_hooks.deallocate = Some(f);
    }
    global_hooks.reallocate = None;
    let alloc_is_default = match global_hooks.allocate {
        Some(f) => f as usize == internal_malloc as usize,
        None => false,
    };
    let free_is_default = match global_hooks.deallocate {
        Some(f) => f as usize == internal_free as usize,
        None => false,
    };
    if alloc_is_default && free_is_default {
        global_hooks.reallocate = Some(internal_realloc);
    }
}

// ---- New_Item ----
unsafe fn cJSON_New_Item(hooks: *const internal_hooks) -> *mut cJSON {
    let node = ((*hooks).allocate.unwrap())(std::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        libc::memset(node as *mut c_void, 0, std::mem::size_of::<cJSON>());
    }
    node
}

// ---- Delete ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let mut next: *mut cJSON;
    while !item.is_null() {
        next = (*item).next;
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
            (global_hooks.deallocate.unwrap())((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            (global_hooks.deallocate.unwrap())((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        (global_hooks.deallocate.unwrap())(item as *mut c_void);
        item = next;
    }
}

// ---- Decimal point ----
unsafe fn get_decimal_point() -> u8 {
    b'.'
}

// ---- parse_buffer ----
#[repr(C)]
struct parse_buffer {
    content: *const u8,
    length: size_t,
    offset: size_t,
    depth: size_t,
    hooks: internal_hooks,
}

#[inline]
unsafe fn can_read(buffer: *const parse_buffer, size: size_t) -> bool {
    !buffer.is_null() && ((*buffer).offset + size) <= (*buffer).length
}

#[inline]
unsafe fn can_access_at_index(buffer: *const parse_buffer, index: size_t) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

#[inline]
unsafe fn cannot_access_at_index(buffer: *const parse_buffer, index: size_t) -> bool {
    !can_access_at_index(buffer, index)
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const u8 {
    (*buffer).content.add((*buffer).offset)
}

// ---- parse_number ----
unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut number: f64;
    let mut after_end: *mut c_char = ptr::null_mut();
    let number_c_string: *mut u8;
    let decimal_point = get_decimal_point();
    let mut i: size_t = 0;
    let mut number_string_length: size_t = 0;
    let mut has_decimal_point = false;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }

    'loop_end: loop {
        i = 0;
        while can_access_at_index(input_buffer, i) {
            let ch = *buffer_at_offset(input_buffer).add(i);
            match ch {
                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+'
                | b'-' | b'e' | b'E' => {
                    number_string_length += 1;
                }
                b'.' => {
                    number_string_length += 1;
                    has_decimal_point = true;
                }
                _ => {
                    break 'loop_end;
                }
            }
            i += 1;
        }
        break 'loop_end;
    }

    number_c_string =
        ((*input_buffer).hooks.allocate.unwrap())(number_string_length + 1) as *mut u8;
    if number_c_string.is_null() {
        return 0;
    }

    libc::memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point {
        let mut k: size_t = 0;
        while k < number_string_length {
            if *number_c_string.add(k) == b'.' {
                *number_c_string.add(k) = decimal_point;
            }
            k += 1;
        }
    }

    number = libc::strtod(number_c_string as *const c_char, &mut after_end);
    if number_c_string == after_end as *mut u8 {
        ((*input_buffer).hooks.deallocate.unwrap())(number_c_string as *mut c_void);
        return 0;
    }

    (*item).valuedouble = number;

    let int_max = c_int::MAX;
    let int_min = c_int::MIN;
    if number >= int_max as f64 {
        (*item).valueint = int_max;
    } else if number <= int_min as f64 {
        (*item).valueint = int_min;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = cJSON_Number;

    let advanced = (after_end as usize).wrapping_sub(number_c_string as usize);
    (*input_buffer).offset += advanced;
    ((*input_buffer).hooks.deallocate.unwrap())(number_c_string as *mut c_void);
    1
}

// ---- SetNumberHelper ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: f64) -> f64 {
    let int_max = c_int::MAX;
    let int_min = c_int::MIN;
    if number >= int_max as f64 {
        (*object).valueint = int_max;
    } else if number <= int_min as f64 {
        (*object).valueint = int_min;
    } else {
        (*object).valueint = number as c_int;
    }
    (*object).valuedouble = number;
    number
}

// ---- SetValuestring ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_SetValuestring(
    object: *mut cJSON,
    valuestring: *const c_char,
) -> *mut c_char {
    if object.is_null()
        || ((*object).type_ & cJSON_String) == 0
        || ((*object).type_ & cJSON_IsReference) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    let v1_len = libc::strlen(valuestring);
    let v2_len = libc::strlen((*object).valuestring);

    if v1_len <= v2_len {
        // overlap check
        let valuestring_end = valuestring.add(v1_len);
        let object_vs = (*object).valuestring;
        let object_vs_end = object_vs.add(v2_len);
        if !((valuestring_end as *const c_char) < object_vs || (object_vs_end as *const c_char) < valuestring) {
            return ptr::null_mut();
        }
        libc::strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cJSON_strdup(valuestring as *const u8, &raw const global_hooks) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

// ---- printbuffer ----
#[repr(C)]
struct printbuffer {
    buffer: *mut u8,
    length: size_t,
    offset: size_t,
    depth: size_t,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: internal_hooks,
}

unsafe fn ensure(p: *mut printbuffer, mut needed: size_t) -> *mut u8 {
    let newbuffer: *mut u8;
    let newsize: size_t;

    if p.is_null() || (*p).buffer.is_null() {
        return ptr::null_mut();
    }

    if (*p).length > 0 && (*p).offset >= (*p).length {
        return ptr::null_mut();
    }

    if needed > c_int::MAX as size_t {
        return ptr::null_mut();
    }

    needed += (*p).offset + 1;
    if needed <= (*p).length {
        return (*p).buffer.add((*p).offset);
    }

    if (*p).noalloc != 0 {
        return ptr::null_mut();
    }

    if needed > (c_int::MAX as size_t) / 2 {
        if needed <= c_int::MAX as size_t {
            newsize = c_int::MAX as size_t;
        } else {
            return ptr::null_mut();
        }
    } else {
        newsize = needed * 2;
    }

    if (*p).hooks.reallocate.is_some() {
        newbuffer =
            ((*p).hooks.reallocate.unwrap())((*p).buffer as *mut c_void, newsize) as *mut u8;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate.unwrap())((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
    } else {
        newbuffer = ((*p).hooks.allocate.unwrap())(newsize) as *mut u8;
        if newbuffer.is_null() {
            ((*p).hooks.deallocate.unwrap())((*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        libc::memcpy(
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
    (*buffer).offset += libc::strlen(buffer_pointer as *const c_char);
}

unsafe fn compare_double(a: f64, b: f64) -> cJSON_bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    if (a - b).abs() <= max_val * f64::EPSILON {
        1
    } else {
        0
    }
}

// ---- print_number ----
unsafe fn print_number(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let output_pointer: *mut u8;
    let d = (*item).valuedouble;
    let length: c_int;
    let decimal_point = get_decimal_point();
    let mut number_buffer: [u8; 26] = [0; 26];

    if output_buffer.is_null() {
        return 0;
    }

    if d.is_nan() || d.is_infinite() {
        length = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"null\0".as_ptr() as *const c_char,
        );
    } else if d == (*item).valueint as f64 {
        length = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"%d\0".as_ptr() as *const c_char,
            (*item).valueint,
        );
    } else {
        let len1 = libc::snprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            26,
            b"%1.15g\0".as_ptr() as *const c_char,
            d,
        );
        let mut test: f64 = 0.0;
        let scan_result = libc::sscanf(
            number_buffer.as_ptr() as *const c_char,
            b"%lg\0".as_ptr() as *const c_char,
            &mut test as *mut f64,
        );
        if scan_result != 1 || compare_double(test, d) == 0 {
            length = libc::snprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                26,
                b"%1.17g\0".as_ptr() as *const c_char,
                d,
            );
        } else {
            length = len1;
        }
    }

    if length < 0 || length > (number_buffer.len() as c_int - 1) {
        return 0;
    }

    output_pointer = ensure(output_buffer, length as size_t + 1);
    if output_pointer.is_null() {
        return 0;
    }

    let mut i: size_t = 0;
    while i < length as size_t {
        if number_buffer[i] == decimal_point {
            *output_pointer.add(i) = b'.';
        } else {
            *output_pointer.add(i) = number_buffer[i];
        }
        i += 1;
    }
    *output_pointer.add(i) = 0;
    (*output_buffer).offset += length as size_t;
    1
}

// ---- parse_hex4 ----
unsafe fn parse_hex4(input: *const u8) -> u32 {
    let mut h: u32 = 0;
    for i in 0..4usize {
        let c = *input.add(i);
        if c >= b'0' && c <= b'9' {
            h += (c - b'0') as u32;
        } else if c >= b'A' && c <= b'F' {
            h += 10 + (c - b'A') as u32;
        } else if c >= b'a' && c <= b'f' {
            h += 10 + (c - b'a') as u32;
        } else {
            return 0;
        }
        if i < 3 {
            h <<= 4;
        }
    }
    h
}

// ---- utf16_literal_to_utf8 ----
unsafe fn utf16_literal_to_utf8(
    input_pointer: *const u8,
    input_end: *const u8,
    output_pointer: *mut *mut u8,
) -> u8 {
    let mut codepoint: u32;
    let first_code: u32;
    let first_sequence = input_pointer;
    let mut utf8_length: u8;
    let mut utf8_position: u8;
    let mut sequence_length: u8;
    let mut first_byte_mark: u8 = 0;

    if (input_end as isize) - (first_sequence as isize) < 6 {
        return 0;
    }
    first_code = parse_hex4(first_sequence.add(2));
    if first_code >= 0xDC00 && first_code <= 0xDFFF {
        return 0;
    }

    if first_code >= 0xD800 && first_code <= 0xDBFF {
        let second_sequence = first_sequence.add(6);
        let second_code: u32;
        sequence_length = 12;
        if (input_end as isize) - (second_sequence as isize) < 6 {
            return 0;
        }
        if *second_sequence.add(0) != b'\\' || *second_sequence.add(1) != b'u' {
            return 0;
        }
        second_code = parse_hex4(second_sequence.add(2));
        if second_code < 0xDC00 || second_code > 0xDFFF {
            return 0;
        }
        codepoint = 0x10000 + (((first_code & 0x3FF) << 10) | (second_code & 0x3FF));
    } else {
        sequence_length = 6;
        codepoint = first_code;
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

    utf8_position = utf8_length - 1;
    while utf8_position > 0 {
        *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as u8;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *(*output_pointer).add(0) = ((codepoint | first_byte_mark as u32) & 0xFF) as u8;
    } else {
        *(*output_pointer).add(0) = (codepoint & 0x7F) as u8;
    }

    *output_pointer = (*output_pointer).add(utf8_length as usize);
    sequence_length
}

// ---- parse_string ----
unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut input_pointer = buffer_at_offset(input_buffer).add(1);
    let mut input_end = buffer_at_offset(input_buffer).add(1);
    let mut output_pointer: *mut u8;
    let mut output: *mut u8 = ptr::null_mut();

    let result: cJSON_bool;

    'fail: loop {
        if *buffer_at_offset(input_buffer).add(0) != b'"' {
            result = 0;
            break 'fail;
        }

        let mut allocation_length: size_t;
        let mut skipped_bytes: size_t = 0;
        while ((input_end as usize - (*input_buffer).content as usize) < (*input_buffer).length)
            && *input_end != b'"'
        {
            if *input_end == b'\\' {
                if (input_end.add(1) as usize - (*input_buffer).content as usize)
                    >= (*input_buffer).length
                {
                    result = 0;
                    break 'fail;
                }
                skipped_bytes += 1;
                input_end = input_end.add(1);
            }
            input_end = input_end.add(1);
        }
        if (input_end as usize - (*input_buffer).content as usize) >= (*input_buffer).length
            || *input_end != b'"'
        {
            result = 0;
            break 'fail;
        }

        allocation_length =
            (input_end as usize - buffer_at_offset(input_buffer) as usize) - skipped_bytes;
        output = ((*input_buffer).hooks.allocate.unwrap())(allocation_length + 1) as *mut u8;
        if output.is_null() {
            result = 0;
            break 'fail;
        }

        output_pointer = output;
        while input_pointer < input_end {
            if *input_pointer != b'\\' {
                *output_pointer = *input_pointer;
                output_pointer = output_pointer.add(1);
                input_pointer = input_pointer.add(1);
            } else {
                let mut sequence_length: u8 = 2;
                if (input_end as isize - input_pointer as isize) < 1 {
                    result = 0;
                    break 'fail;
                }
                match *input_pointer.add(1) {
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
                        *output_pointer = *input_pointer.add(1);
                        output_pointer = output_pointer.add(1);
                    }
                    b'u' => {
                        sequence_length = utf16_literal_to_utf8(
                            input_pointer,
                            input_end,
                            &mut output_pointer,
                        );
                        if sequence_length == 0 {
                            result = 0;
                            break 'fail;
                        }
                    }
                    _ => {
                        result = 0;
                        break 'fail;
                    }
                }
                input_pointer = input_pointer.add(sequence_length as usize);
            }
        }
        *output_pointer = 0;
        (*item).type_ = cJSON_String;
        (*item).valuestring = output as *mut c_char;
        (*input_buffer).offset = input_end as usize - (*input_buffer).content as usize;
        (*input_buffer).offset += 1;
        let _ = allocation_length;
        return 1;
    }

    if !output.is_null() {
        ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
    }
    if !input_pointer.is_null() {
        (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
    }
    result
}

// ---- print_string_ptr ----
unsafe fn print_string_ptr(input: *const u8, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut input_pointer: *const u8;
    let output: *mut u8;
    let mut output_pointer: *mut u8;
    let output_length: size_t;
    let mut escape_characters: size_t = 0;

    if output_buffer.is_null() {
        return 0;
    }

    if input.is_null() {
        let out = ensure(output_buffer, 3);
        if out.is_null() {
            return 0;
        }
        libc::strcpy(out as *mut c_char, b"\"\"\0".as_ptr() as *const c_char);
        return 1;
    }

    input_pointer = input;
    while *input_pointer != 0 {
        match *input_pointer {
            b'"' | b'\\' | 0x08 | 0x0C | b'\n' | b'\r' | b'\t' => {
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
    output_length = (input_pointer as usize - input as usize) + escape_characters;

    output = ensure(output_buffer, output_length + 3);
    if output.is_null() {
        return 0;
    }

    if escape_characters == 0 {
        *output = b'"';
        libc::memcpy(
            output.add(1) as *mut c_void,
            input as *const c_void,
            output_length,
        );
        *output.add(output_length + 1) = b'"';
        *output.add(output_length + 2) = 0;
        return 1;
    }

    *output = b'"';
    output_pointer = output.add(1);
    input_pointer = input;
    while *input_pointer != 0 {
        let ch = *input_pointer;
        if ch > 31 && ch != b'"' && ch != b'\\' {
            *output_pointer = ch;
        } else {
            *output_pointer = b'\\';
            output_pointer = output_pointer.add(1);
            match ch {
                b'\\' => {
                    *output_pointer = b'\\';
                }
                b'"' => {
                    *output_pointer = b'"';
                }
                0x08 => {
                    *output_pointer = b'b';
                }
                0x0C => {
                    *output_pointer = b'f';
                }
                b'\n' => {
                    *output_pointer = b'n';
                }
                b'\r' => {
                    *output_pointer = b'r';
                }
                b'\t' => {
                    *output_pointer = b't';
                }
                _ => {
                    libc::sprintf(
                        output_pointer as *mut c_char,
                        b"u%04x\0".as_ptr() as *const c_char,
                        ch as c_int,
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
    1
}

unsafe fn print_string(item: *const cJSON, p: *mut printbuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const u8, p)
}

// ---- buffer_skip_whitespace ----
unsafe fn buffer_skip_whitespace(buffer: *mut parse_buffer) -> *mut parse_buffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }
    if cannot_access_at_index(buffer, 0) {
        return buffer;
    }
    while can_access_at_index(buffer, 0) && *buffer_at_offset(buffer).add(0) <= 32 {
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
        && libc::strncmp(
            buffer_at_offset(buffer) as *const c_char,
            b"\xEF\xBB\xBF\0".as_ptr() as *const c_char,
            3,
        ) == 0
    {
        (*buffer).offset += 3;
    }
    buffer
}

// ---- ParseWithOpts ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithOpts(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    if value.is_null() {
        return ptr::null_mut();
    }
    let buffer_length = libc::strlen(value) + 1;
    cJSON_ParseWithLengthOpts(value, buffer_length, return_parse_end, require_null_terminated)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: size_t,
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

    global_error.json = ptr::null();
    global_error.position = 0;

    if value.is_null() || buffer_length == 0 {
        // fail
    } else {
        buffer.content = value as *const u8;
        buffer.length = buffer_length;
        buffer.offset = 0;
        buffer.hooks = global_hooks;

        item = cJSON_New_Item(&raw const global_hooks);
        if !item.is_null() {
            if parse_value(
                item,
                buffer_skip_whitespace(skip_utf8_bom(&mut buffer as *mut parse_buffer)),
            ) != 0
            {
                if require_null_terminated != 0 {
                    buffer_skip_whitespace(&mut buffer as *mut parse_buffer);
                    if buffer.offset >= buffer.length || *buffer_at_offset(&buffer).add(0) != 0 {
                        // fail path
                        cJSON_Delete(item);
                        return parse_fail(value, &buffer, return_parse_end);
                    }
                }
                if !return_parse_end.is_null() {
                    *return_parse_end = buffer_at_offset(&buffer) as *const c_char;
                }
                return item;
            }
        }
    }

    if !item.is_null() {
        cJSON_Delete(item);
    }
    parse_fail(value, &buffer, return_parse_end)
}

unsafe fn parse_fail(
    value: *const c_char,
    buffer: *const parse_buffer,
    return_parse_end: *mut *const c_char,
) -> *mut cJSON {
    if !value.is_null() {
        let mut local_error = error_t {
            json: value as *const u8,
            position: 0,
        };
        if (*buffer).offset < (*buffer).length {
            local_error.position = (*buffer).offset;
        } else if (*buffer).length > 0 {
            local_error.position = (*buffer).length - 1;
        }
        if !return_parse_end.is_null() {
            *return_parse_end =
                (local_error.json as *const c_char).offset(local_error.position as isize);
        }
        global_error = local_error;
    }
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_ParseWithLength(
    value: *const c_char,
    buffer_length: size_t,
) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

#[inline]
fn cjson_min(a: size_t, b: size_t) -> size_t {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn print_internal(
    item: *const cJSON,
    format: cJSON_bool,
    hooks: *const internal_hooks,
) -> *mut u8 {
    let default_buffer_size: size_t = 256;
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
    let mut printed: *mut u8 = ptr::null_mut();

    buffer.buffer = ((*hooks).allocate.unwrap())(default_buffer_size) as *mut u8;
    buffer.length = default_buffer_size;
    buffer.format = format;
    buffer.hooks = *hooks;
    if buffer.buffer.is_null() {
        return ptr::null_mut();
    }

    if print_value(item, &mut buffer as *mut printbuffer) == 0 {
        if !buffer.buffer.is_null() {
            ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
        }
        return ptr::null_mut();
    }
    update_offset(&mut buffer as *mut printbuffer);

    if (*hooks).reallocate.is_some() {
        printed =
            ((*hooks).reallocate.unwrap())(buffer.buffer as *mut c_void, buffer.offset + 1)
                as *mut u8;
        if printed.is_null() {
            if !buffer.buffer.is_null() {
                ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
            }
            return ptr::null_mut();
        }
        buffer.buffer = ptr::null_mut();
    } else {
        printed = ((*hooks).allocate.unwrap())(buffer.offset + 1) as *mut u8;
        if printed.is_null() {
            if !buffer.buffer.is_null() {
                ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
            }
            return ptr::null_mut();
        }
        libc::memcpy(
            printed as *mut c_void,
            buffer.buffer as *const c_void,
            cjson_min(buffer.length, buffer.offset + 1),
        );
        *printed.add(buffer.offset) = 0;
        ((*hooks).deallocate.unwrap())(buffer.buffer as *mut c_void);
        buffer.buffer = ptr::null_mut();
    }

    printed
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print_internal(item, 1, &raw const global_hooks) as *mut c_char
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print_internal(item, 0, &raw const global_hooks) as *mut c_char
}

#[no_mangle]
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
    p.buffer = (global_hooks.allocate.unwrap())(prebuffer as size_t) as *mut u8;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }
    p.length = prebuffer as size_t;
    p.offset = 0;
    p.noalloc = 0;
    p.format = fmt;
    p.hooks = global_hooks;
    if print_value(item, &mut p as *mut printbuffer) == 0 {
        (global_hooks.deallocate.unwrap())(p.buffer as *mut c_void);
        return ptr::null_mut();
    }
    p.buffer as *mut c_char
}

#[no_mangle]
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
        buffer: buffer as *mut u8,
        length: length as size_t,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: global_hooks,
    };
    print_value(item as *const cJSON, &mut p as *mut printbuffer)
}

// ---- parse_value ----
unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    if can_read(input_buffer, 4)
        && libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"null\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = cJSON_NULL;
        (*input_buffer).offset += 4;
        return 1;
    }
    if can_read(input_buffer, 5)
        && libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"false\0".as_ptr() as *const c_char,
            5,
        ) == 0
    {
        (*item).type_ = cJSON_False;
        (*input_buffer).offset += 5;
        return 1;
    }
    if can_read(input_buffer, 4)
        && libc::strncmp(
            buffer_at_offset(input_buffer) as *const c_char,
            b"true\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = cJSON_True;
        (*item).valueint = 1;
        (*input_buffer).offset += 4;
        return 1;
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer).add(0) == b'"' {
        return parse_string(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) {
        let c = *buffer_at_offset(input_buffer).add(0);
        if c == b'-' || (c >= b'0' && c <= b'9') {
            return parse_number(item, input_buffer);
        }
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer).add(0) == b'[' {
        return parse_array(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer).add(0) == b'{' {
        return parse_object(item, input_buffer);
    }
    0
}

// ---- print_value ----
unsafe fn print_value(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let output: *mut u8;
    if item.is_null() || output_buffer.is_null() {
        return 0;
    }
    match (*item).type_ & 0xFF {
        x if x == cJSON_NULL => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return 0;
            }
            libc::strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_False => {
            output = ensure(output_buffer, 6);
            if output.is_null() {
                return 0;
            }
            libc::strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_True => {
            output = ensure(output_buffer, 5);
            if output.is_null() {
                return 0;
            }
            libc::strcpy(output as *mut c_char, b"true\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_Number => print_number(item, output_buffer),
        x if x == cJSON_Raw => {
            if (*item).valuestring.is_null() {
                return 0;
            }
            let raw_length = libc::strlen((*item).valuestring) + 1;
            output = ensure(output_buffer, raw_length);
            if output.is_null() {
                return 0;
            }
            libc::memcpy(
                output as *mut c_void,
                (*item).valuestring as *const c_void,
                raw_length,
            );
            1
        }
        x if x == cJSON_String => print_string(item, output_buffer),
        x if x == cJSON_Array => print_array(item, output_buffer),
        x if x == cJSON_Object => print_object(item, output_buffer),
        _ => 0,
    }
}

// ---- parse_array ----
unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    'fail: loop {
        if *buffer_at_offset(input_buffer).add(0) != b'[' {
            break 'fail;
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer).add(0) == b']' {
            // success
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).type_ = cJSON_Array;
            (*item).child = head;
            (*input_buffer).offset += 1;
            return 1;
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
                break 'fail;
            }
            buffer_skip_whitespace(input_buffer);
            if !(can_access_at_index(input_buffer, 0)
                && *buffer_at_offset(input_buffer).add(0) == b',')
            {
                break;
            }
        }
        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer).add(0) != b']'
        {
            break 'fail;
        }
        (*input_buffer).depth -= 1;
        if !head.is_null() {
            (*head).prev = current_item;
        }
        (*item).type_ = cJSON_Array;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return 1;
    }

    if !head.is_null() {
        cJSON_Delete(head);
    }
    0
}

// ---- print_array ----
unsafe fn print_array(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut output_pointer: *mut u8;
    let mut length: size_t;
    let mut current_element = (*item).child;

    if output_buffer.is_null() {
        return 0;
    }
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
            *output_pointer = 0;
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
    *output_pointer = 0;
    (*output_buffer).depth -= 1;
    1
}

// ---- parse_object ----
unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    'fail: loop {
        if cannot_access_at_index(input_buffer, 0)
            || *buffer_at_offset(input_buffer).add(0) != b'{'
        {
            break 'fail;
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer).add(0) == b'}' {
            // success
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).type_ = cJSON_Object;
            (*item).child = head;
            (*input_buffer).offset += 1;
            return 1;
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
                head = new_item;
                current_item = new_item;
            } else {
                (*current_item).next = new_item;
                (*new_item).prev = current_item;
                current_item = new_item;
            }

            if cannot_access_at_index(input_buffer, 1) {
                break 'fail;
            }
            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if parse_string(current_item, input_buffer) == 0 {
                break 'fail;
            }
            buffer_skip_whitespace(input_buffer);
            (*current_item).string = (*current_item).valuestring;
            (*current_item).valuestring = ptr::null_mut();
            if cannot_access_at_index(input_buffer, 0)
                || *buffer_at_offset(input_buffer).add(0) != b':'
            {
                break 'fail;
            }
            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if parse_value(current_item, input_buffer) == 0 {
                break 'fail;
            }
            buffer_skip_whitespace(input_buffer);
            if !(can_access_at_index(input_buffer, 0)
                && *buffer_at_offset(input_buffer).add(0) == b',')
            {
                break;
            }
        }
        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer).add(0) != b'}'
        {
            break 'fail;
        }
        (*input_buffer).depth -= 1;
        if !head.is_null() {
            (*head).prev = current_item;
        }
        (*item).type_ = cJSON_Object;
        (*item).child = head;
        (*input_buffer).offset += 1;
        return 1;
    }

    if !head.is_null() {
        cJSON_Delete(head);
    }
    0
}

// ---- print_object ----
unsafe fn print_object(item: *const cJSON, output_buffer: *mut printbuffer) -> cJSON_bool {
    let mut output_pointer: *mut u8;
    let mut length: size_t;
    let mut current_item = (*item).child;

    if output_buffer.is_null() {
        return 0;
    }
    length = if (*output_buffer).format != 0 { 2 } else { 1 };
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
            output_pointer = ensure(output_buffer, (*output_buffer).depth);
            if output_pointer.is_null() {
                return 0;
            }
            for _ in 0..(*output_buffer).depth {
                *output_pointer = b'\t';
                output_pointer = output_pointer.add(1);
            }
            (*output_buffer).offset += (*output_buffer).depth;
        }

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

        if print_value(current_item, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        let format_part: size_t = if (*output_buffer).format != 0 { 1 } else { 0 };
        let next_part: size_t = if !(*current_item).next.is_null() { 1 } else { 0 };
        length = format_part + next_part;
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
        return 0;
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
    1
}

// ---- Get array size/items ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_GetArraySize(array: *const cJSON) -> c_int {
    if array.is_null() {
        return 0;
    }
    let mut child = (*array).child;
    let mut size: size_t = 0;
    while !child.is_null() {
        size += 1;
        child = (*child).next;
    }
    size as c_int
}

unsafe fn get_array_item(array: *const cJSON, mut index: size_t) -> *mut cJSON {
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetArrayItem(array: *const cJSON, index: c_int) -> *mut cJSON {
    if index < 0 {
        return ptr::null_mut();
    }
    get_array_item(array, index as size_t)
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
            && libc::strcmp(name, (*current_element).string) != 0
        {
            current_element = (*current_element).next;
        }
    } else {
        while !current_element.is_null()
            && case_insensitive_strcmp(
                name as *const u8,
                (*current_element).string as *const u8,
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, 0)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(
    object: *const cJSON,
    string: *const c_char,
) -> *mut cJSON {
    get_object_item(object, string, 1)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_HasObjectItem(
    object: *const cJSON,
    string: *const c_char,
) -> cJSON_bool {
    if cJSON_GetObjectItem(object, string).is_null() {
        0
    } else {
        1
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
    libc::memcpy(
        reference as *mut c_void,
        item as *const c_void,
        std::mem::size_of::<cJSON>(),
    );
    (*reference).string = ptr::null_mut();
    (*reference).type_ |= cJSON_IsReference;
    (*reference).next = ptr::null_mut();
    (*reference).prev = ptr::null_mut();
    reference
}

unsafe fn add_item_to_array(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    if item.is_null() || array.is_null() || array == item {
        return 0;
    }
    let child = (*array).child;
    if child.is_null() {
        (*array).child = item;
        (*item).prev = item;
        (*item).next = ptr::null_mut();
    } else {
        if !(*child).prev.is_null() {
            suffix_object((*child).prev, item);
            (*(*array).child).prev = item;
        }
    }
    1
}

#[no_mangle]
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
    let new_key: *mut c_char;
    let new_type: c_int;
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }
    if constant_key != 0 {
        new_key = string as *mut c_char;
        new_type = (*item).type_ | cJSON_StringIsConst;
    } else {
        new_key = cJSON_strdup(string as *const u8, hooks) as *mut c_char;
        if new_key.is_null() {
            return 0;
        }
        new_type = (*item).type_ & !cJSON_StringIsConst;
    }
    if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
        ((*hooks).deallocate.unwrap())((*item).string as *mut c_void);
    }
    (*item).string = new_key;
    (*item).type_ = new_type;
    add_item_to_array(object, item)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &raw const global_hooks, 0)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &raw const global_hooks, 1)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return 0;
    }
    add_item_to_array(array, create_reference(item, &raw const global_hooks))
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    if object.is_null() || string.is_null() {
        return 0;
    }
    add_item_to_object(
        object,
        string,
        create_reference(item, &raw const global_hooks),
        &raw const global_hooks,
        0,
    )
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, &raw const global_hooks, 0) != 0 {
        return null;
    }
    cJSON_Delete(null);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddTrueToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let t = cJSON_CreateTrue();
    if add_item_to_object(object, name, t, &raw const global_hooks, 0) != 0 {
        return t;
    }
    cJSON_Delete(t);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let f = cJSON_CreateFalse();
    if add_item_to_object(object, name, f, &raw const global_hooks, 0) != 0 {
        return f;
    }
    cJSON_Delete(f);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    let b = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, b, &raw const global_hooks, 0) != 0 {
        return b;
    }
    cJSON_Delete(b);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: f64,
) -> *mut cJSON {
    let n = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, n, &raw const global_hooks, 0) != 0 {
        return n;
    }
    cJSON_Delete(n);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    let s = cJSON_CreateString(string);
    if add_item_to_object(object, name, s, &raw const global_hooks, 0) != 0 {
        return s;
    }
    cJSON_Delete(s);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    let r = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, r, &raw const global_hooks, 0) != 0 {
        return r;
    }
    cJSON_Delete(r);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let o = cJSON_CreateObject();
    if add_item_to_object(object, name, o, &raw const global_hooks, 0) != 0 {
        return o;
    }
    cJSON_Delete(o);
    ptr::null_mut()
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let a = cJSON_CreateArray();
    if add_item_to_object(object, name, a, &raw const global_hooks, 0) != 0 {
        return a;
    }
    cJSON_Delete(a);
    ptr::null_mut()
}

#[no_mangle]
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromArray(
    array: *mut cJSON,
    which: c_int,
) -> *mut cJSON {
    if which < 0 {
        return ptr::null_mut();
    }
    cJSON_DetachItemViaPointer(array, get_array_item(array, which as size_t))
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromArray(array: *mut cJSON, which: c_int) {
    cJSON_Delete(cJSON_DetachItemFromArray(array, which))
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromObject(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItem(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItemCaseSensitive(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_InsertItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return 0;
    }
    let after_inserted = get_array_item(array, which as size_t);
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(
    parent: *mut cJSON,
    item: *mut cJSON,
    replacement: *mut cJSON,
) -> cJSON_bool {
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
    if which < 0 {
        return 0;
    }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as size_t), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if replacement.is_null() || string.is_null() {
        return 0;
    }
    if ((*replacement).type_ & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string =
        cJSON_strdup(string as *const u8, &raw const global_hooks) as *mut c_char;
    if (*replacement).string.is_null() {
        return 0;
    }
    (*replacement).type_ &= !cJSON_StringIsConst;
    cJSON_ReplaceItemViaPointer(object, get_object_item(object, string, case_sensitive), replacement)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 0)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(
    object: *mut cJSON,
    string: *const c_char,
    newitem: *mut cJSON,
) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 1)
}

// ---- Create constructors ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_NULL;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_True;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_False;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = if boolean != 0 { cJSON_True } else { cJSON_False };
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateNumber(num: f64) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Number;
        (*item).valuedouble = num;
        let int_max = c_int::MAX;
        let int_min = c_int::MIN;
        if num >= int_max as f64 {
            (*item).valueint = int_max;
        } else if num <= int_min as f64 {
            (*item).valueint = int_min;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_String;
        (*item).valuestring =
            cJSON_strdup(string as *const u8, &raw const global_hooks) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_String | cJSON_IsReference;
        (*item).valuestring = string as *mut c_char;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Object | cJSON_IsReference;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Array | cJSON_IsReference;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Raw;
        (*item).valuestring =
            cJSON_strdup(raw as *const u8, &raw const global_hooks) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Array;
    }
    item
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item(&raw const global_hooks);
    if !item.is_null() {
        (*item).type_ = cJSON_Object;
    }
    item
}

// ---- Create arrays ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: size_t = 0;
    while !a.is_null() && i < count as size_t {
        n = cJSON_CreateNumber(*numbers.add(i) as f64);
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateFloatArray(numbers: *const f32, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: size_t = 0;
    while !a.is_null() && i < count as size_t {
        n = cJSON_CreateNumber(*numbers.add(i) as f64);
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateDoubleArray(numbers: *const f64, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: size_t = 0;
    while !a.is_null() && i < count as size_t {
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_CreateStringArray(
    strings: *const *const c_char,
    count: c_int,
) -> *mut cJSON {
    if count < 0 || strings.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: size_t = 0;
    while !a.is_null() && i < count as size_t {
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

// ---- Duplicate ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

unsafe fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: size_t,
    recurse: cJSON_bool,
) -> *mut cJSON {
    let mut newitem: *mut cJSON = ptr::null_mut();
    let mut child: *mut cJSON;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();

    'fail: loop {
        if item.is_null() {
            break 'fail;
        }
        newitem = cJSON_New_Item(&raw const global_hooks);
        if newitem.is_null() {
            break 'fail;
        }
        (*newitem).type_ = (*item).type_ & !cJSON_IsReference;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring =
                cJSON_strdup((*item).valuestring as *const u8, &raw const global_hooks)
                    as *mut c_char;
            if (*newitem).valuestring.is_null() {
                break 'fail;
            }
        }
        if !(*item).string.is_null() {
            (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {
                (*item).string
            } else {
                cJSON_strdup((*item).string as *const u8, &raw const global_hooks) as *mut c_char
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
            newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
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
    *input = (*input).add(2); // "//"
    while *(*input).add(0) as u8 != 0 {
        if *(*input).add(0) as u8 == b'\n' {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2); // "/*"
    while *(*input).add(0) as u8 != 0 {
        if *(*input).add(0) as u8 == b'*' && *(*input).add(1) as u8 == b'/' {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: *mut *mut c_char, output: *mut *mut c_char) {
    *(*output).add(0) = *(*input).add(0);
    *input = (*input).add(1);
    *output = (*output).add(1);
    while *(*input).add(0) as u8 != 0 {
        *(*output).add(0) = *(*input).add(0);
        if *(*input).add(0) as u8 == b'"' {
            *(*output).add(0) = b'"' as c_char;
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if *(*input).add(0) as u8 == b'\\' && *(*input).add(1) as u8 == b'"' {
            *(*output).add(1) = *(*input).add(1);
            *input = (*input).add(1);
            *output = (*output).add(1);
        }
        *input = (*input).add(1);
        *output = (*output).add(1);
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_Minify(json: *mut c_char) {
    if json.is_null() {
        return;
    }
    let mut into = json;
    let mut json_p = json;
    while *json_p as u8 != 0 {
        match *json_p as u8 {
            b' ' | b'\t' | b'\r' | b'\n' => {
                json_p = json_p.add(1);
            }
            b'/' => {
                if *json_p.add(1) as u8 == b'/' {
                    skip_oneline_comment(&mut json_p);
                } else if *json_p.add(1) as u8 == b'*' {
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

// ---- Type checks ----
#[no_mangle]
pub unsafe extern "C" fn cJSON_IsInvalid(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_Invalid {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsFalse(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_False {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_True {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsBool(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & (cJSON_True | cJSON_False)) != 0 {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsNull(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_NULL {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsNumber(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_Number {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsString(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_String {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsArray(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_Array {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsObject(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_Object {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_IsRaw(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xFF) == cJSON_Raw {
        1
    } else {
        0
    }
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if cJSON_IsString(item) == 0 {
        return ptr::null_mut();
    }
    (*item).valuestring
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> f64 {
    if cJSON_IsNumber(item) == 0 {
        return f64::NAN;
    }
    (*item).valuedouble
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
        return 0;
    }
    match (*a).type_ & 0xFF {
        x if x == cJSON_False
            || x == cJSON_True
            || x == cJSON_NULL
            || x == cJSON_Number
            || x == cJSON_String
            || x == cJSON_Raw
            || x == cJSON_Array
            || x == cJSON_Object => {}
        _ => return 0,
    }
    if a == b {
        return 1;
    }
    match (*a).type_ & 0xFF {
        x if x == cJSON_False || x == cJSON_True || x == cJSON_NULL => 1,
        x if x == cJSON_Number => {
            if compare_double((*a).valuedouble, (*b).valuedouble) != 0 {
                1
            } else {
                0
            }
        }
        x if x == cJSON_String || x == cJSON_Raw => {
            if (*a).valuestring.is_null() || (*b).valuestring.is_null() {
                return 0;
            }
            if libc::strcmp((*a).valuestring, (*b).valuestring) == 0 {
                1
            } else {
                0
            }
        }
        x if x == cJSON_Array => {
            let mut a_element = (*a).child;
            let mut b_element = (*b).child;
            while !a_element.is_null() && !b_element.is_null() {
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return 0;
                }
                a_element = (*a_element).next;
                b_element = (*b_element).next;
            }
            if a_element != b_element {
                return 0;
            }
            1
        }
        x if x == cJSON_Object => {
            let mut a_element = (*a).child;
            while !a_element.is_null() {
                let b_element = get_object_item(b, (*a_element).string, case_sensitive);
                if b_element.is_null() {
                    return 0;
                }
                if cJSON_Compare(a_element, b_element, case_sensitive) == 0 {
                    return 0;
                }
                a_element = (*a_element).next;
            }
            let mut b_element = (*b).child;
            while !b_element.is_null() {
                let a_element = get_object_item(a, (*b_element).string, case_sensitive);
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

#[no_mangle]
pub unsafe extern "C" fn cJSON_malloc(size: size_t) -> *mut c_void {
    (global_hooks.allocate.unwrap())(size)
}

#[no_mangle]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    (global_hooks.deallocate.unwrap())(object);
}

// suppress unused import warning
#[allow(unused)]
fn _unused() {
    let _ = CString::new("");
}
