// Rust translation of cJSON.c and test.c
// Produces byte-identical output for the same inputs.
#![allow(non_camel_case_types)]
#![allow(non_snake_case)]
#![allow(non_upper_case_globals)]
#![allow(dead_code)]
#![allow(clippy::missing_safety_doc)]

use std::ffi::c_void;
use std::os::raw::{c_char, c_double, c_float, c_int, c_uchar, c_uint, c_ulong};
use std::ptr;

// ----- cJSON Types -----
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

pub type cJSON_bool = c_int;

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

// ----- libc bindings -----
extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn realloc(ptr: *mut c_void, size: usize) -> *mut c_void;
    fn memcpy(dest: *mut c_void, src: *const c_void, n: usize) -> *mut c_void;
    fn memset(s: *mut c_void, c: c_int, n: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
    fn strcmp(s1: *const c_char, s2: *const c_char) -> c_int;
    fn strncmp(s1: *const c_char, s2: *const c_char, n: usize) -> c_int;
    fn strcpy(dest: *mut c_char, src: *const c_char) -> *mut c_char;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
    fn tolower(c: c_int) -> c_int;
    fn sprintf(s: *mut c_char, format: *const c_char, ...) -> c_int;
    fn sscanf(s: *const c_char, format: *const c_char, ...) -> c_int;
    fn printf(format: *const c_char, ...) -> c_int;
    fn fabs(x: c_double) -> c_double;
    fn exit(status: c_int) -> !;
}

const INT_MAX: c_int = c_int::MAX;
const INT_MIN: c_int = c_int::MIN;
const DBL_EPSILON: c_double = f64::EPSILON;
const EXIT_FAILURE: c_int = 1;

// isnan/isinf for f64
fn isnan_d(d: c_double) -> bool {
    d != d
}
fn isinf_d(d: c_double) -> bool {
    let diff = d - d;
    isnan_d(diff) && !isnan_d(d)
}

// ----- Global state -----
#[repr(C)]
struct ErrorState {
    json: *const c_uchar,
    position: usize,
}

static mut GLOBAL_ERROR: ErrorState = ErrorState {
    json: ptr::null(),
    position: 0,
};

#[repr(C)]
#[derive(Copy, Clone)]
struct InternalHooks {
    allocate: Option<unsafe extern "C" fn(usize) -> *mut c_void>,
    deallocate: Option<unsafe extern "C" fn(*mut c_void)>,
    reallocate: Option<unsafe extern "C" fn(*mut c_void, usize) -> *mut c_void>,
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

static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: Some(internal_malloc),
    deallocate: Some(internal_free),
    reallocate: Some(internal_realloc),
};

// ----- cJSON_GetErrorPtr -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    let ge = &raw const GLOBAL_ERROR;
    let json = (*ge).json;
    let pos = (*ge).position;
    // Match C semantics: pointer arithmetic on possibly-null base.
    (json as usize + pos) as *const c_char
}

// ----- cJSON_GetStringValue -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetStringValue(item: *const cJSON) -> *mut c_char {
    if cJSON_IsString(item) == 0 {
        return ptr::null_mut();
    }
    (*item).valuestring
}

// ----- cJSON_GetNumberValue -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetNumberValue(item: *const cJSON) -> c_double {
    if cJSON_IsNumber(item) == 0 {
        return f64::NAN;
    }
    (*item).valuedouble
}

// ----- cJSON_Version -----
static mut VERSION_BUFFER: [c_char; 15] = [0; 15];
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Version() -> *const c_char {
    let fmt = b"%i.%i.%i\0".as_ptr() as *const c_char;
    let vb = &raw mut VERSION_BUFFER;
    sprintf(vb as *mut c_char, fmt, CJSON_VERSION_MAJOR, CJSON_VERSION_MINOR, CJSON_VERSION_PATCH);
    vb as *const c_char
}

// ----- case_insensitive_strcmp -----
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

// ----- cJSON_strdup -----
unsafe fn cJSON_strdup(string: *const c_uchar, hooks: *const InternalHooks) -> *mut c_uchar {
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

// ----- cJSON_InitHooks -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    let gh = &raw mut GLOBAL_HOOKS;
    if hooks.is_null() {
        (*gh).allocate = Some(internal_malloc);
        (*gh).deallocate = Some(internal_free);
        (*gh).reallocate = Some(internal_realloc);
        return;
    }
    (*gh).allocate = Some(internal_malloc);
    if let Some(_) = (*hooks).malloc_fn {
        (*gh).allocate = std::mem::transmute::<
            Option<unsafe extern "C" fn(usize) -> *mut c_void>,
            Option<unsafe extern "C" fn(usize) -> *mut c_void>,
        >((*hooks).malloc_fn);
    }
    (*gh).deallocate = Some(internal_free);
    if let Some(_) = (*hooks).free_fn {
        (*gh).deallocate = (*hooks).free_fn;
    }
    (*gh).reallocate = None;
    let alloc_is_malloc = matches!((*gh).allocate, Some(f) if f as usize == internal_malloc as usize);
    let free_is_free = matches!((*gh).deallocate, Some(f) if f as usize == internal_free as usize);
    if alloc_is_malloc && free_is_free {
        (*gh).reallocate = Some(internal_realloc);
    }
}

// ----- cJSON_New_Item -----
unsafe fn cJSON_New_Item(hooks: *const InternalHooks) -> *mut cJSON {
    let node = ((*hooks).allocate.unwrap())(std::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        memset(node as *mut c_void, 0, std::mem::size_of::<cJSON>());
    }
    node
}

// ----- cJSON_Delete -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    let gh = &raw const GLOBAL_HOOKS;
    while !item.is_null() {
        let next = (*item).next;
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
            ((*gh).deallocate.unwrap())((*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            ((*gh).deallocate.unwrap())((*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        ((*gh).deallocate.unwrap())(item as *mut c_void);
        item = next;
    }
}

// ----- get_decimal_point -----
fn get_decimal_point() -> c_uchar {
    b'.'
}

// ----- parse_buffer -----
#[repr(C)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: InternalHooks,
}

#[inline]
unsafe fn can_read(buffer: *const ParseBuffer, size: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + size <= (*buffer).length)
}
#[inline]
unsafe fn can_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + index < (*buffer).length)
}
#[inline]
unsafe fn cannot_access_at_index(buffer: *const ParseBuffer, index: usize) -> bool {
    !can_access_at_index(buffer, index)
}
#[inline]
unsafe fn buffer_at_offset(buffer: *const ParseBuffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

// ----- parse_number -----
unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut number: c_double;
    let mut after_end: *mut c_uchar = ptr::null_mut();
    let decimal_point = get_decimal_point();
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = 0;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
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
                has_decimal_point = 1;
            }
            _ => break 'outer,
        }
        i += 1;
    }

    let number_c_string =
        ((*input_buffer).hooks.allocate.unwrap())(number_string_length + 1) as *mut c_uchar;
    if number_c_string.is_null() {
        return 0;
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
        return 0;
    }

    (*item).valuedouble = number;

    if number >= INT_MAX as c_double {
        (*item).valueint = INT_MAX;
    } else if number <= INT_MIN as c_double {
        (*item).valueint = INT_MIN;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = cJSON_Number;
    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    ((*input_buffer).hooks.deallocate.unwrap())(number_c_string as *mut c_void);
    1
}

// ----- cJSON_SetNumberHelper -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= INT_MAX as c_double {
        (*object).valueint = INT_MAX;
    } else if number <= INT_MIN as c_double {
        (*object).valueint = INT_MIN;
    } else {
        (*object).valueint = number as c_int;
    }
    (*object).valuedouble = number;
    number
}

// ----- cJSON_SetValuestring -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetValuestring(object: *mut cJSON, valuestring: *const c_char) -> *mut c_char {
    let gh = &raw const GLOBAL_HOOKS;
    if object.is_null()
        || ((*object).type_ & cJSON_String) == 0
        || ((*object).type_ & cJSON_IsReference) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    let v1_len = strlen(valuestring);
    let v2_len = strlen((*object).valuestring);
    if v1_len <= v2_len {
        let vs_end = valuestring.add(v1_len);
        let obj_end = (*object).valuestring.add(v2_len);
        let no_overlap = vs_end < (*object).valuestring as *const c_char || obj_end < valuestring as *mut c_char;
        if !no_overlap {
            return ptr::null_mut();
        }
        strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cJSON_strdup(valuestring as *const c_uchar, gh) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

// ----- printbuffer -----
#[repr(C)]
struct PrintBuffer {
    buffer: *mut c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: InternalHooks,
}

unsafe fn ensure(p: *mut PrintBuffer, mut needed: usize) -> *mut c_uchar {
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
    let newsize: usize;
    if needed > (INT_MAX as usize / 2) {
        if needed <= INT_MAX as usize {
            newsize = INT_MAX as usize;
        } else {
            return ptr::null_mut();
        }
    } else {
        newsize = needed * 2;
    }
    let newbuffer: *mut c_uchar;
    if let Some(realloc_fn) = (*p).hooks.reallocate {
        newbuffer = realloc_fn((*p).buffer as *mut c_void, newsize) as *mut c_uchar;
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

unsafe fn update_offset(buffer: *mut PrintBuffer) {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    let buffer_pointer = (*buffer).buffer.add((*buffer).offset);
    (*buffer).offset += strlen(buffer_pointer as *const c_char);
}

unsafe fn compare_double(a: c_double, b: c_double) -> cJSON_bool {
    let max_val = if fabs(a) > fabs(b) { fabs(a) } else { fabs(b) };
    if fabs(a - b) <= max_val * DBL_EPSILON {
        1
    } else {
        0
    }
}

// ----- print_number -----
unsafe fn print_number(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let d = (*item).valuedouble;
    let mut length: c_int;
    let mut number_buffer: [c_uchar; 26] = [0; 26];
    let decimal_point = get_decimal_point();
    let mut test: c_double = 0.0;

    if output_buffer.is_null() {
        return 0;
    }

    if isnan_d(d) || isinf_d(d) {
        let fmt = b"null\0".as_ptr() as *const c_char;
        let fmt_str = b"%s\0".as_ptr() as *const c_char;
        length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt_str, fmt);
    } else if d == ((*item).valueint as c_double) {
        let fmt = b"%d\0".as_ptr() as *const c_char;
        length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt, (*item).valueint);
    } else {
        let fmt15 = b"%1.15g\0".as_ptr() as *const c_char;
        length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt15, d);

        let scan_fmt = b"%lg\0".as_ptr() as *const c_char;
        let scan_result = sscanf(
            number_buffer.as_ptr() as *const c_char,
            scan_fmt,
            &mut test as *mut c_double,
        );
        if scan_result != 1 || compare_double(test, d) == 0 {
            let fmt17 = b"%1.17g\0".as_ptr() as *const c_char;
            length = sprintf(number_buffer.as_mut_ptr() as *mut c_char, fmt17, d);
        }
    }

    if length < 0 || length > (number_buffer.len() - 1) as c_int {
        return 0;
    }

    let output_pointer = ensure(output_buffer, length as usize + 1);
    if output_pointer.is_null() {
        return 0;
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
    1
}

// ----- parse_hex4 -----
unsafe fn parse_hex4(input: *const c_uchar) -> c_uint {
    let mut h: c_uint = 0;
    for i in 0..4usize {
        let c = *input.add(i);
        if c >= b'0' && c <= b'9' {
            h += (c as c_uint) - (b'0' as c_uint);
        } else if c >= b'A' && c <= b'F' {
            h += 10 + (c as c_uint) - (b'A' as c_uint);
        } else if c >= b'a' && c <= b'f' {
            h += 10 + (c as c_uint) - (b'a' as c_uint);
        } else {
            return 0;
        }
        if i < 3 {
            h <<= 4;
        }
    }
    h
}

// ----- utf16_literal_to_utf8 -----
unsafe fn utf16_literal_to_utf8(
    input_pointer: *const c_uchar,
    input_end: *const c_uchar,
    output_pointer: *mut *mut c_uchar,
) -> c_uchar {
    let mut codepoint: c_ulong = 0;
    let first_sequence = input_pointer;
    let mut utf8_length: c_uchar = 0;
    let mut utf8_position: c_uchar;
    let sequence_length: c_uchar;
    let mut first_byte_mark: c_uchar = 0;

    if (input_end as isize - first_sequence as isize) < 6 {
        return 0;
    }

    let first_code = parse_hex4(first_sequence.add(2));

    if first_code >= 0xDC00 && first_code <= 0xDFFF {
        return 0;
    }

    if first_code >= 0xD800 && first_code <= 0xDBFF {
        let second_sequence = first_sequence.add(6);
        sequence_length = 12;
        if (input_end as isize - second_sequence as isize) < 6 {
            return 0;
        }
        if *second_sequence != b'\\' || *second_sequence.add(1) != b'u' {
            return 0;
        }
        let second_code = parse_hex4(second_sequence.add(2));
        if second_code < 0xDC00 || second_code > 0xDFFF {
            return 0;
        }
        codepoint = 0x10000
            + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as c_ulong);
    } else {
        sequence_length = 6;
        codepoint = first_code as c_ulong;
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
        *(*output_pointer).add(utf8_position as usize) =
            ((codepoint | 0x80) & 0xBF) as c_uchar;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *(*output_pointer).add(0) = ((codepoint | first_byte_mark as c_ulong) & 0xFF) as c_uchar;
    } else {
        *(*output_pointer).add(0) = (codepoint & 0x7F) as c_uchar;
    }

    *output_pointer = (*output_pointer).add(utf8_length as usize);
    sequence_length
}

// ----- parse_string -----
unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut input_pointer = buffer_at_offset(input_buffer).add(1);
    let mut input_end = buffer_at_offset(input_buffer).add(1);
    let mut output_pointer: *mut c_uchar = ptr::null_mut();
    let mut output: *mut c_uchar = ptr::null_mut();

    if *buffer_at_offset(input_buffer) != b'"' {
        return parse_string_fail(input_buffer, output, input_pointer);
    }

    {
        let mut allocation_length: usize;
        let mut skipped_bytes: usize = 0;
        while ((input_end as usize) - ((*input_buffer).content as usize)) < (*input_buffer).length
            && *input_end != b'"'
        {
            if *input_end == b'\\' {
                if (input_end.add(1) as usize - (*input_buffer).content as usize)
                    >= (*input_buffer).length
                {
                    return parse_string_fail(input_buffer, output, input_pointer);
                }
                skipped_bytes += 1;
                input_end = input_end.add(1);
            }
            input_end = input_end.add(1);
        }
        if ((input_end as usize) - ((*input_buffer).content as usize)) >= (*input_buffer).length
            || *input_end != b'"'
        {
            return parse_string_fail(input_buffer, output, input_pointer);
        }
        allocation_length =
            (input_end as usize) - (buffer_at_offset(input_buffer) as usize) - skipped_bytes;
        output =
            ((*input_buffer).hooks.allocate.unwrap())(allocation_length + 1) as *mut c_uchar;
        if output.is_null() {
            return parse_string_fail(input_buffer, output, input_pointer);
        }
    }

    output_pointer = output;
    while input_pointer < input_end {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        } else {
            let mut sequence_length: c_uchar = 2;
            if (input_end as isize - input_pointer as isize) < 1 {
                return parse_string_fail(input_buffer, output, input_pointer);
            }
            let escape = *input_pointer.add(1);
            match escape {
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
                    sequence_length =
                        utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                    if sequence_length == 0 {
                        return parse_string_fail(input_buffer, output, input_pointer);
                    }
                }
                _ => return parse_string_fail(input_buffer, output, input_pointer),
            }
            input_pointer = input_pointer.add(sequence_length as usize);
        }
    }

    *output_pointer = 0;
    (*item).type_ = cJSON_String;
    (*item).valuestring = output as *mut c_char;
    (*input_buffer).offset = (input_end as usize) - ((*input_buffer).content as usize);
    (*input_buffer).offset += 1;
    1
}

unsafe fn parse_string_fail(
    input_buffer: *mut ParseBuffer,
    output: *mut c_uchar,
    input_pointer: *const c_uchar,
) -> cJSON_bool {
    if !output.is_null() {
        ((*input_buffer).hooks.deallocate.unwrap())(output as *mut c_void);
    }
    if !input_pointer.is_null() {
        (*input_buffer).offset = (input_pointer as usize) - ((*input_buffer).content as usize);
    }
    0
}

// ----- print_string_ptr -----
unsafe fn print_string_ptr(input: *const c_uchar, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    if input.is_null() {
        let output = ensure(output_buffer, 3);
        if output.is_null() {
            return 0;
        }
        let empty = b"\"\"\0";
        strcpy(output as *mut c_char, empty.as_ptr() as *const c_char);
        return 1;
    }

    let mut escape_characters: usize = 0;
    let mut input_pointer = input;
    while *input_pointer != 0 {
        let c = *input_pointer;
        match c {
            b'"' | b'\\' | 0x08 | 0x0C | b'\n' | b'\r' | b'\t' => escape_characters += 1,
            _ => {
                if c < 32 {
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
        *output = b'"';
        memcpy(output.add(1) as *mut c_void, input as *const c_void, output_length);
        *output.add(output_length + 1) = b'"';
        *output.add(output_length + 2) = 0;
        return 1;
    }

    *output = b'"';
    let mut output_pointer = output.add(1);
    input_pointer = input;
    while *input_pointer != 0 {
        let c = *input_pointer;
        if c > 31 && c != b'"' && c != b'\\' {
            *output_pointer = c;
        } else {
            *output_pointer = b'\\';
            output_pointer = output_pointer.add(1);
            match c {
                b'\\' => *output_pointer = b'\\',
                b'"' => *output_pointer = b'"',
                0x08 => *output_pointer = b'b',
                0x0C => *output_pointer = b'f',
                b'\n' => *output_pointer = b'n',
                b'\r' => *output_pointer = b'r',
                b'\t' => *output_pointer = b't',
                _ => {
                    let fmt = b"u%04x\0".as_ptr() as *const c_char;
                    sprintf(output_pointer as *mut c_char, fmt, c as c_int);
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

unsafe fn print_string(item: *const cJSON, p: *mut PrintBuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const c_uchar, p)
}

// ----- buffer_skip_whitespace -----
unsafe fn buffer_skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
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

// ----- skip_utf8_bom -----
unsafe fn skip_utf8_bom(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
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

// ----- cJSON_ParseWithOpts -----
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

// ----- cJSON_ParseWithLengthOpts -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: usize,
    return_parse_end: *mut *const c_char,
    require_null_terminated: cJSON_bool,
) -> *mut cJSON {
    let mut buffer = ParseBuffer {
        content: ptr::null(),
        length: 0,
        offset: 0,
        depth: 0,
        hooks: InternalHooks {
            allocate: None,
            deallocate: None,
            reallocate: None,
        },
    };
    let mut item: *mut cJSON = ptr::null_mut();
    let ge = &raw mut GLOBAL_ERROR;
    let gh = &raw const GLOBAL_HOOKS;

    (*ge).json = ptr::null();
    (*ge).position = 0;

    if value.is_null() || buffer_length == 0 {
        return parse_with_length_opts_fail(value, return_parse_end, &buffer, item);
    }

    buffer.content = value as *const c_uchar;
    buffer.length = buffer_length;
    buffer.offset = 0;
    buffer.hooks = *gh;

    item = cJSON_New_Item(gh);
    if item.is_null() {
        return parse_with_length_opts_fail(value, return_parse_end, &buffer, item);
    }

    if parse_value(
        item,
        buffer_skip_whitespace(skip_utf8_bom(&mut buffer)),
    ) == 0
    {
        return parse_with_length_opts_fail(value, return_parse_end, &buffer, item);
    }

    if require_null_terminated != 0 {
        buffer_skip_whitespace(&mut buffer);
        if buffer.offset >= buffer.length || *buffer_at_offset(&buffer) != 0 {
            return parse_with_length_opts_fail(value, return_parse_end, &buffer, item);
        }
    }
    if !return_parse_end.is_null() {
        *return_parse_end = buffer_at_offset(&buffer) as *const c_char;
    }
    item
}

unsafe fn parse_with_length_opts_fail(
    value: *const c_char,
    return_parse_end: *mut *const c_char,
    buffer: *const ParseBuffer,
    item: *mut cJSON,
) -> *mut cJSON {
    let ge = &raw mut GLOBAL_ERROR;
    if !item.is_null() {
        cJSON_Delete(item);
    }
    if !value.is_null() {
        let local_error_json = value as *const c_uchar;
        let mut local_error_position: usize = 0;

        if (*buffer).offset < (*buffer).length {
            local_error_position = (*buffer).offset;
        } else if (*buffer).length > 0 {
            local_error_position = (*buffer).length - 1;
        }

        if !return_parse_end.is_null() {
            *return_parse_end = local_error_json.add(local_error_position) as *const c_char;
        }
        (*ge).json = local_error_json;
        (*ge).position = local_error_position;
    }
    ptr::null_mut()
}

// ----- cJSON_Parse -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Parse(value: *const c_char) -> *mut cJSON {
    cJSON_ParseWithOpts(value, ptr::null_mut(), 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLength(value: *const c_char, buffer_length: usize) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

fn cjson_min(a: usize, b: usize) -> usize {
    if a < b {
        a
    } else {
        b
    }
}

unsafe fn print_internal(item: *const cJSON, format: cJSON_bool, hooks: *const InternalHooks) -> *mut c_uchar {
    let default_buffer_size: usize = 256;
    let mut buffer = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: InternalHooks {
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
        return print_internal_fail(&mut buffer, printed, hooks);
    }

    if print_value(item, &mut buffer) == 0 {
        return print_internal_fail(&mut buffer, printed, hooks);
    }
    update_offset(&mut buffer);

    if let Some(realloc_fn) = (*hooks).reallocate {
        printed = realloc_fn(buffer.buffer as *mut c_void, buffer.offset + 1) as *mut c_uchar;
        if printed.is_null() {
            return print_internal_fail(&mut buffer, printed, hooks);
        }
        buffer.buffer = ptr::null_mut();
    } else {
        printed = ((*hooks).allocate.unwrap())(buffer.offset + 1) as *mut c_uchar;
        if printed.is_null() {
            return print_internal_fail(&mut buffer, printed, hooks);
        }
        memcpy(
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

unsafe fn print_internal_fail(
    buffer: *mut PrintBuffer,
    mut printed: *mut c_uchar,
    hooks: *const InternalHooks,
) -> *mut c_uchar {
    if !(*buffer).buffer.is_null() {
        ((*hooks).deallocate.unwrap())((*buffer).buffer as *mut c_void);
        (*buffer).buffer = ptr::null_mut();
    }
    if !printed.is_null() {
        ((*hooks).deallocate.unwrap())(printed as *mut c_void);
        printed = ptr::null_mut();
    }
    let _ = printed;
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    let gh = &raw const GLOBAL_HOOKS;
    print_internal(item, 1, gh) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    let gh = &raw const GLOBAL_HOOKS;
    print_internal(item, 0, gh) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(item: *const cJSON, prebuffer: c_int, fmt: cJSON_bool) -> *mut c_char {
    let gh = &raw const GLOBAL_HOOKS;
    let mut p = PrintBuffer {
        buffer: ptr::null_mut(),
        length: 0,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format: 0,
        hooks: InternalHooks {
            allocate: None,
            deallocate: None,
            reallocate: None,
        },
    };
    if prebuffer < 0 {
        return ptr::null_mut();
    }
    p.buffer = ((*gh).allocate.unwrap())(prebuffer as usize) as *mut c_uchar;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }
    p.length = prebuffer as usize;
    p.offset = 0;
    p.noalloc = 0;
    p.format = fmt;
    p.hooks = *gh;

    if print_value(item, &mut p) == 0 {
        ((*gh).deallocate.unwrap())(p.buffer as *mut c_void);
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
    let gh = &raw const GLOBAL_HOOKS;
    if length < 0 || buffer.is_null() {
        return 0;
    }
    let mut p = PrintBuffer {
        buffer: buffer as *mut c_uchar,
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: *gh,
    };
    print_value(item, &mut p)
}

// ----- parse_value -----
unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    if can_read(input_buffer, 4)
        && strncmp(
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
        && strncmp(
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
        && strncmp(
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
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'"' {
        return parse_string(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) {
        let c = *buffer_at_offset(input_buffer);
        if c == b'-' || (c >= b'0' && c <= b'9') {
            return parse_number(item, input_buffer);
        }
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'[' {
        return parse_array(item, input_buffer);
    }
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'{' {
        return parse_object(item, input_buffer);
    }
    0
}

// ----- print_value -----
unsafe fn print_value(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if item.is_null() || output_buffer.is_null() {
        return 0;
    }
    match (*item).type_ & 0xFF {
        x if x == cJSON_NULL => {
            let output = ensure(output_buffer, 5);
            if output.is_null() {
                return 0;
            }
            strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_False => {
            let output = ensure(output_buffer, 6);
            if output.is_null() {
                return 0;
            }
            strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_True => {
            let output = ensure(output_buffer, 5);
            if output.is_null() {
                return 0;
            }
            strcpy(output as *mut c_char, b"true\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_Number => print_number(item, output_buffer),
        x if x == cJSON_Raw => {
            if (*item).valuestring.is_null() {
                return 0;
            }
            let raw_length = strlen((*item).valuestring) + 1;
            let output = ensure(output_buffer, raw_length);
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
        x if x == cJSON_String => print_string(item, output_buffer),
        x if x == cJSON_Array => print_array(item, output_buffer),
        x if x == cJSON_Object => print_object(item, output_buffer),
        _ => 0,
    }
}

// ----- parse_array -----
unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if *buffer_at_offset(input_buffer) != b'[' {
        return parse_array_fail(input_buffer, head);
    }

    (*input_buffer).offset += 1;
    buffer_skip_whitespace(input_buffer);
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b']' {
        return parse_array_success(item, input_buffer, head, current_item);
    }
    if cannot_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        return parse_array_fail(input_buffer, head);
    }
    (*input_buffer).offset -= 1;

    loop {
        let new_item = cJSON_New_Item(&(*input_buffer).hooks);
        if new_item.is_null() {
            return parse_array_fail(input_buffer, head);
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
            return parse_array_fail(input_buffer, head);
        }
        buffer_skip_whitespace(input_buffer);
        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b']' {
        return parse_array_fail(input_buffer, head);
    }
    parse_array_success(item, input_buffer, head, current_item)
}

unsafe fn parse_array_success(
    item: *mut cJSON,
    input_buffer: *mut ParseBuffer,
    head: *mut cJSON,
    current_item: *mut cJSON,
) -> cJSON_bool {
    (*input_buffer).depth -= 1;
    if !head.is_null() {
        (*head).prev = current_item;
    }
    (*item).type_ = cJSON_Array;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}

unsafe fn parse_array_fail(_input_buffer: *mut ParseBuffer, head: *mut cJSON) -> cJSON_bool {
    if !head.is_null() {
        cJSON_Delete(head);
    }
    0
}

// ----- print_array -----
unsafe fn print_array(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut current_element = (*item).child;

    if output_buffer.is_null() {
        return 0;
    }
    let mut output_pointer = ensure(output_buffer, 1);
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
            let length: usize = if (*output_buffer).format != 0 { 2 } else { 1 };
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

// ----- parse_object -----
unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'{' {
        return parse_object_fail(head);
    }
    (*input_buffer).offset += 1;
    buffer_skip_whitespace(input_buffer);
    if can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b'}' {
        return parse_object_success(item, input_buffer, head, current_item);
    }
    if cannot_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        return parse_object_fail(head);
    }
    (*input_buffer).offset -= 1;

    loop {
        let new_item = cJSON_New_Item(&(*input_buffer).hooks);
        if new_item.is_null() {
            return parse_object_fail(head);
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
            return parse_object_fail(head);
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_string(current_item, input_buffer) == 0 {
            return parse_object_fail(head);
        }
        buffer_skip_whitespace(input_buffer);

        (*current_item).string = (*current_item).valuestring;
        (*current_item).valuestring = ptr::null_mut();

        if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b':' {
            return parse_object_fail(head);
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_value(current_item, input_buffer) == 0 {
            return parse_object_fail(head);
        }
        buffer_skip_whitespace(input_buffer);
        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if cannot_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'}' {
        return parse_object_fail(head);
    }
    parse_object_success(item, input_buffer, head, current_item)
}

unsafe fn parse_object_success(
    item: *mut cJSON,
    input_buffer: *mut ParseBuffer,
    head: *mut cJSON,
    current_item: *mut cJSON,
) -> cJSON_bool {
    (*input_buffer).depth -= 1;
    if !head.is_null() {
        (*head).prev = current_item;
    }
    (*item).type_ = cJSON_Object;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}

unsafe fn parse_object_fail(head: *mut cJSON) -> cJSON_bool {
    if !head.is_null() {
        cJSON_Delete(head);
    }
    0
}

// ----- print_object -----
unsafe fn print_object(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut current_item = (*item).child;
    if output_buffer.is_null() {
        return 0;
    }
    let mut length: usize = if (*output_buffer).format != 0 { 2 } else { 1 };
    let mut output_pointer = ensure(output_buffer, length + 1);
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

        if print_string_ptr((*current_item).string as *const c_uchar, output_buffer) == 0 {
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

        let fmt_part: usize = if (*output_buffer).format != 0 { 1 } else { 0 };
        let next_part: usize = if !(*current_item).next.is_null() { 1 } else { 0 };
        length = fmt_part + next_part;
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

// ----- cJSON_GetArraySize -----
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
pub unsafe extern "C" fn cJSON_GetObjectItem(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    get_object_item(object, string, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetObjectItemCaseSensitive(object: *const cJSON, string: *const c_char) -> *mut cJSON {
    get_object_item(object, string, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_HasObjectItem(object: *const cJSON, string: *const c_char) -> cJSON_bool {
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

unsafe fn create_reference(item: *const cJSON, hooks: *const InternalHooks) -> *mut cJSON {
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
    hooks: *const InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }
    let new_key: *mut c_char;
    let new_type: c_int;
    if constant_key != 0 {
        new_key = cast_away_const(string as *const c_void) as *mut c_char;
        new_type = (*item).type_ | cJSON_StringIsConst;
    } else {
        new_key = cJSON_strdup(string as *const c_uchar, hooks) as *mut c_char;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    let gh = &raw const GLOBAL_HOOKS;
    add_item_to_object(object, string, item, gh, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    let gh = &raw const GLOBAL_HOOKS;
    add_item_to_object(object, string, item, gh, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(array: *mut cJSON, item: *mut cJSON) -> cJSON_bool {
    let gh = &raw const GLOBAL_HOOKS;
    if array.is_null() {
        return 0;
    }
    add_item_to_array(array, create_reference(item, gh))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToObject(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    let gh = &raw const GLOBAL_HOOKS;
    if object.is_null() || string.is_null() {
        return 0;
    }
    add_item_to_object(object, string, create_reference(item, gh), gh, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, gh, 0) != 0 {
        return null;
    }
    cJSON_Delete(null);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddTrueToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let true_item = cJSON_CreateTrue();
    if add_item_to_object(object, name, true_item, gh, 0) != 0 {
        return true_item;
    }
    cJSON_Delete(true_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let false_item = cJSON_CreateFalse();
    if add_item_to_object(object, name, false_item, gh, 0) != 0 {
        return false_item;
    }
    cJSON_Delete(false_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(object: *mut cJSON, name: *const c_char, boolean: cJSON_bool) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let bool_item = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, bool_item, gh, 0) != 0 {
        return bool_item;
    }
    cJSON_Delete(bool_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(object: *mut cJSON, name: *const c_char, number: c_double) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let number_item = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, number_item, gh, 0) != 0 {
        return number_item;
    }
    cJSON_Delete(number_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(object: *mut cJSON, name: *const c_char, string: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let string_item = cJSON_CreateString(string);
    if add_item_to_object(object, name, string_item, gh, 0) != 0 {
        return string_item;
    }
    cJSON_Delete(string_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(object: *mut cJSON, name: *const c_char, raw: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let raw_item = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, raw_item, gh, 0) != 0 {
        return raw_item;
    }
    cJSON_Delete(raw_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let object_item = cJSON_CreateObject();
    if add_item_to_object(object, name, object_item, gh, 0) != 0 {
        return object_item;
    }
    cJSON_Delete(object_item);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(object: *mut cJSON, name: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let array = cJSON_CreateArray();
    if add_item_to_object(object, name, array, gh, 0) != 0 {
        return array;
    }
    cJSON_Delete(array);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemViaPointer(parent: *mut cJSON, item: *mut cJSON) -> *mut cJSON {
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
pub unsafe extern "C" fn cJSON_DetachItemFromObject(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItem(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DetachItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) -> *mut cJSON {
    let to_detach = cJSON_GetObjectItemCaseSensitive(object, string);
    cJSON_DetachItemViaPointer(object, to_detach)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObject(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObject(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_DeleteItemFromObjectCaseSensitive(object: *mut cJSON, string: *const c_char) {
    cJSON_Delete(cJSON_DetachItemFromObjectCaseSensitive(object, string));
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InsertItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 || newitem.is_null() {
        return 0;
    }
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
pub unsafe extern "C" fn cJSON_ReplaceItemViaPointer(parent: *mut cJSON, item: *mut cJSON, replacement: *mut cJSON) -> cJSON_bool {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(array: *mut cJSON, which: c_int, newitem: *mut cJSON) -> cJSON_bool {
    if which < 0 {
        return 0;
    }
    cJSON_ReplaceItemViaPointer(array, get_array_item(array, which as usize), newitem)
}

unsafe fn replace_item_in_object(
    object: *mut cJSON,
    string: *const c_char,
    replacement: *mut cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    let gh = &raw const GLOBAL_HOOKS;
    if replacement.is_null() || string.is_null() {
        return 0;
    }
    if ((*replacement).type_ & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string = cJSON_strdup(string as *const c_uchar, gh) as *mut c_char;
    if (*replacement).string.is_null() {
        return 0;
    }
    (*replacement).type_ &= !cJSON_StringIsConst;
    cJSON_ReplaceItemViaPointer(object, get_object_item(object, string, case_sensitive), replacement)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObject(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInObjectCaseSensitive(object: *mut cJSON, string: *const c_char, newitem: *mut cJSON) -> cJSON_bool {
    replace_item_in_object(object, string, newitem, 1)
}

// ----- Create basic types -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_NULL;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_True;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_False;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = if boolean != 0 { cJSON_True } else { cJSON_False };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Number;
        (*item).valuedouble = num;
        if num >= INT_MAX as c_double {
            (*item).valueint = INT_MAX;
        } else if num <= INT_MIN as c_double {
            (*item).valueint = INT_MIN;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_String;
        (*item).valuestring = cJSON_strdup(string as *const c_uchar, gh) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_String | cJSON_IsReference;
        (*item).valuestring = cast_away_const(string as *const c_void) as *mut c_char;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Object | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Array | cJSON_IsReference;
        (*item).child = cast_away_const(child as *const c_void) as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Raw;
        (*item).valuestring = cJSON_strdup(raw as *const c_uchar, gh) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Array;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let item = cJSON_New_Item(gh);
    if !item.is_null() {
        (*item).type_ = cJSON_Object;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateIntArray(numbers: *const c_int, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: usize = 0;
    while !a.is_null() && i < count as usize {
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
pub unsafe extern "C" fn cJSON_CreateFloatArray(numbers: *const c_float, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: usize = 0;
    while !a.is_null() && i < count as usize {
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
pub unsafe extern "C" fn cJSON_CreateDoubleArray(numbers: *const c_double, count: c_int) -> *mut cJSON {
    if count < 0 || numbers.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: usize = 0;
    while !a.is_null() && i < count as usize {
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
pub unsafe extern "C" fn cJSON_CreateStringArray(strings: *const *const c_char, count: c_int) -> *mut cJSON {
    if count < 0 || strings.is_null() {
        return ptr::null_mut();
    }
    let a = cJSON_CreateArray();
    let mut p: *mut cJSON = ptr::null_mut();
    let mut n: *mut cJSON = ptr::null_mut();
    let mut i: usize = 0;
    while !a.is_null() && i < count as usize {
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

// ----- cJSON_Duplicate -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate_rec(item: *const cJSON, depth: usize, recurse: cJSON_bool) -> *mut cJSON {
    let gh = &raw const GLOBAL_HOOKS;
    let mut newitem: *mut cJSON = ptr::null_mut();
    let mut child: *const cJSON;
    let mut next: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();

    if item.is_null() {
        return duplicate_fail(newitem);
    }
    newitem = cJSON_New_Item(gh);
    if newitem.is_null() {
        return duplicate_fail(newitem);
    }
    (*newitem).type_ = (*item).type_ & !cJSON_IsReference;
    (*newitem).valueint = (*item).valueint;
    (*newitem).valuedouble = (*item).valuedouble;
    if !(*item).valuestring.is_null() {
        (*newitem).valuestring =
            cJSON_strdup((*item).valuestring as *const c_uchar, gh) as *mut c_char;
        if (*newitem).valuestring.is_null() {
            return duplicate_fail(newitem);
        }
    }
    if !(*item).string.is_null() {
        (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {
            (*item).string
        } else {
            cJSON_strdup((*item).string as *const c_uchar, gh) as *mut c_char
        };
        if (*newitem).string.is_null() {
            return duplicate_fail(newitem);
        }
    }
    if recurse == 0 {
        return newitem;
    }
    child = (*item).child;
    while !child.is_null() {
        if depth >= CJSON_CIRCULAR_LIMIT {
            return duplicate_fail(newitem);
        }
        newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
        if newchild.is_null() {
            return duplicate_fail(newitem);
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

unsafe fn duplicate_fail(newitem: *mut cJSON) -> *mut cJSON {
    if !newitem.is_null() {
        cJSON_Delete(newitem);
    }
    ptr::null_mut()
}

// ----- cJSON_Minify -----
unsafe fn skip_oneline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2); // "//"
    while *(*input) != 0 {
        if *(*input) == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2); // "/*"
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
    let mut json_p = json;
    let mut into = json;
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

// ----- Type checks -----
#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_IsTrue(item: *const cJSON) -> cJSON_bool {
    if item.is_null() {
        return 0;
    }
    if ((*item).type_ & 0xff) == cJSON_True {
        1
    } else {
        0
    }
}

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

#[unsafe(no_mangle)]
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

// ----- cJSON_Compare -----
#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(a: *const cJSON, b: *const cJSON, case_sensitive: cJSON_bool) -> cJSON_bool {
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
            if strcmp((*a).valuestring, (*b).valuestring) == 0 {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_malloc(size: usize) -> *mut c_void {
    let gh = &raw const GLOBAL_HOOKS;
    ((*gh).allocate.unwrap())(size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    let gh = &raw const GLOBAL_HOOKS;
    ((*gh).deallocate.unwrap())(object);
    // (the C code assigns object = NULL after free, but that's a local-only effect)
}

// ===========================================
// test.c translation
// ===========================================

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

unsafe fn print_preallocated(root: *mut cJSON) -> c_int {
    let out = cJSON_Print(root);

    let len = strlen(out) + 5;
    let buf = malloc(len) as *mut c_char;
    if buf.is_null() {
        let fmt = b"Failed to allocate memory.\n\0".as_ptr() as *const c_char;
        printf(fmt);
        exit(1);
    }

    let len_fail = strlen(out);
    let buf_fail = malloc(len_fail) as *mut c_char;
    if buf_fail.is_null() {
        let fmt = b"Failed to allocate memory.\n\0".as_ptr() as *const c_char;
        printf(fmt);
        exit(1);
    }

    if cJSON_PrintPreallocated(root, buf, len as c_int, 1) == 0 {
        let fmt1 = b"cJSON_PrintPreallocated failed!\n\0".as_ptr() as *const c_char;
        printf(fmt1);
        if strcmp(out, buf) != 0 {
            let fmt2 = b"cJSON_PrintPreallocated not the same as cJSON_Print!\n\0".as_ptr() as *const c_char;
            printf(fmt2);
            let fmt3 = b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char;
            printf(fmt3, out);
            let fmt4 = b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char;
            printf(fmt4, buf);
        }
        free(out as *mut c_void);
        free(buf_fail as *mut c_void);
        free(buf as *mut c_void);
        return -1;
    }

    let fmt_ok = b"%s\n\0".as_ptr() as *const c_char;
    printf(fmt_ok, buf);

    if cJSON_PrintPreallocated(root, buf_fail, len_fail as c_int, 1) != 0 {
        let f1 = b"cJSON_PrintPreallocated failed to show error with insufficient memory!\n\0".as_ptr() as *const c_char;
        printf(f1);
        let f2 = b"cJSON_Print result:\n%s\n\0".as_ptr() as *const c_char;
        printf(f2, out);
        let f3 = b"cJSON_PrintPreallocated result:\n%s\n\0".as_ptr() as *const c_char;
        printf(f3, buf_fail);
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
    let mut root: *mut cJSON;
    let fmt: *mut cJSON;
    let img: *mut cJSON;
    let thm: *mut cJSON;
    let mut fld: *mut cJSON;

    let zero: c_double = std::ptr::read_volatile(&0.0_f64 as *const f64);

    // Video
    root = cJSON_CreateObject();
    cJSON_AddItemToObject(
        root,
        b"name\0".as_ptr() as *const c_char,
        cJSON_CreateString(b"Jack (\"Bee\") Nimble\0".as_ptr() as *const c_char),
    );
    fmt = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"format\0".as_ptr() as *const c_char, fmt);
    cJSON_AddStringToObject(
        fmt,
        b"type\0".as_ptr() as *const c_char,
        b"rect\0".as_ptr() as *const c_char,
    );
    cJSON_AddNumberToObject(fmt, b"width\0".as_ptr() as *const c_char, 1920.0);
    cJSON_AddNumberToObject(fmt, b"height\0".as_ptr() as *const c_char, 1080.0);
    cJSON_AddFalseToObject(fmt, b"interlace\0".as_ptr() as *const c_char);
    cJSON_AddNumberToObject(fmt, b"frame rate\0".as_ptr() as *const c_char, 24.0);

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    // String array
    root = cJSON_CreateStringArray(strings, 7);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    // Matrix
    root = cJSON_CreateArray();
    for i in 0..3 {
        cJSON_AddItemToArray(
            root,
            cJSON_CreateIntArray((*numbers.add(i)).as_ptr(), 3),
        );
    }
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    // Gallery
    root = cJSON_CreateObject();
    img = cJSON_CreateObject();
    cJSON_AddItemToObject(root, b"Image\0".as_ptr() as *const c_char, img);
    cJSON_AddNumberToObject(img, b"Width\0".as_ptr() as *const c_char, 800.0);
    cJSON_AddNumberToObject(img, b"Height\0".as_ptr() as *const c_char, 600.0);
    cJSON_AddStringToObject(
        img,
        b"Title\0".as_ptr() as *const c_char,
        b"View from 15th Floor\0".as_ptr() as *const c_char,
    );
    thm = cJSON_CreateObject();
    cJSON_AddItemToObject(img, b"Thumbnail\0".as_ptr() as *const c_char, thm);
    cJSON_AddStringToObject(
        thm,
        b"Url\0".as_ptr() as *const c_char,
        b"http:/*www.example.com/image/481989943\0".as_ptr() as *const c_char,
    );
    cJSON_AddNumberToObject(thm, b"Height\0".as_ptr() as *const c_char, 125.0);
    cJSON_AddStringToObject(
        thm,
        b"Width\0".as_ptr() as *const c_char,
        b"100\0".as_ptr() as *const c_char,
    );
    cJSON_AddItemToObject(
        img,
        b"IDs\0".as_ptr() as *const c_char,
        cJSON_CreateIntArray(ids, 4),
    );

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    // Records
    root = cJSON_CreateArray();
    for i in 0..2 {
        fld = cJSON_CreateObject();
        cJSON_AddItemToArray(root, fld);
        let f = &*fields.add(i);
        cJSON_AddStringToObject(fld, b"precision\0".as_ptr() as *const c_char, f.precision);
        cJSON_AddNumberToObject(fld, b"Latitude\0".as_ptr() as *const c_char, f.lat);
        cJSON_AddNumberToObject(fld, b"Longitude\0".as_ptr() as *const c_char, f.lon);
        cJSON_AddStringToObject(fld, b"Address\0".as_ptr() as *const c_char, f.address);
        cJSON_AddStringToObject(fld, b"City\0".as_ptr() as *const c_char, f.city);
        cJSON_AddStringToObject(fld, b"State\0".as_ptr() as *const c_char, f.state);
        cJSON_AddStringToObject(fld, b"Zip\0".as_ptr() as *const c_char, f.zip);
        cJSON_AddStringToObject(fld, b"Country\0".as_ptr() as *const c_char, f.country);
    }

    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);

    // 1.0 / zero (Inf)
    root = cJSON_CreateObject();
    cJSON_AddNumberToObject(root, b"number\0".as_ptr() as *const c_char, 1.0 / zero);
    if print_preallocated(root) != 0 {
        cJSON_Delete(root);
        exit(EXIT_FAILURE);
    }
    cJSON_Delete(root);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(
    strings: *const *const c_char,
    numbers: *const [c_int; 3],
    ids: *const c_int,
    fields: *const record,
) -> c_int {
    let fmt = b"Version: %s\n\0".as_ptr() as *const c_char;
    printf(fmt, cJSON_Version());
    create_objects(strings, numbers, ids, fields);
    0
}
