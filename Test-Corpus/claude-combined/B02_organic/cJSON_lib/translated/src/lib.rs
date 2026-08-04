//! Rust translation of cJSON.
//!
//! Aims to produce byte-identical output to the C version for the same inputs.

#![allow(non_snake_case)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]
#![allow(static_mut_refs)]
#![allow(unused_assignments)]
#![allow(clippy::missing_safety_doc)]

use core::ffi::{c_char, c_double, c_int, c_uchar, c_uint, c_void};
use core::ptr;

// Constants from cJSON.h
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

pub type MallocFn = unsafe extern "C" fn(libc::size_t) -> *mut c_void;
pub type FreeFn = unsafe extern "C" fn(*mut c_void);
pub type ReallocFn = unsafe extern "C" fn(*mut c_void, libc::size_t) -> *mut c_void;

#[repr(C)]
pub struct cJSON_Hooks {
    pub malloc_fn: Option<MallocFn>,
    pub free_fn: Option<FreeFn>,
}

#[repr(C)]
#[derive(Copy, Clone)]
struct InternalHooks {
    allocate: Option<MallocFn>,
    deallocate: Option<FreeFn>,
    reallocate: Option<ReallocFn>,
}

// ---- C library bridges ---------------------------------------------------

unsafe extern "C" fn rs_malloc(sz: libc::size_t) -> *mut c_void {
    libc::malloc(sz)
}
unsafe extern "C" fn rs_free(p: *mut c_void) {
    libc::free(p)
}
unsafe extern "C" fn rs_realloc(p: *mut c_void, sz: libc::size_t) -> *mut c_void {
    libc::realloc(p, sz)
}

// ---- Global state --------------------------------------------------------

static mut GLOBAL_ERROR_JSON: *const c_uchar = ptr::null();
static mut GLOBAL_ERROR_POSITION: usize = 0;

static mut GLOBAL_HOOKS: InternalHooks = InternalHooks {
    allocate: Some(rs_malloc),
    deallocate: Some(rs_free),
    reallocate: Some(rs_realloc),
};

#[inline]
unsafe fn alloc_with(hooks: &InternalHooks, size: usize) -> *mut c_void {
    (hooks.allocate.unwrap())(size)
}
#[inline]
unsafe fn free_with(hooks: &InternalHooks, p: *mut c_void) {
    (hooks.deallocate.unwrap())(p)
}

// ---- Helpers for libc ----------------------------------------------------

/// strlen for *const c_char
#[inline]
unsafe fn c_strlen(s: *const c_char) -> usize {
    libc::strlen(s)
}

/// strlen for *const c_uchar
#[inline]
unsafe fn c_strlen_u(s: *const c_uchar) -> usize {
    libc::strlen(s as *const c_char)
}

/// case-insensitive compare exactly like the C version
unsafe fn case_insensitive_strcmp(string1: *const c_uchar, string2: *const c_uchar) -> c_int {
    if string1.is_null() || string2.is_null() {
        return 1;
    }
    if string1 == string2 {
        return 0;
    }
    let mut s1 = string1;
    let mut s2 = string2;
    loop {
        let c1 = libc::tolower(*s1 as c_int);
        let c2 = libc::tolower(*s2 as c_int);
        if c1 != c2 {
            return c1 - c2;
        }
        if *s1 == 0 {
            return 0;
        }
        s1 = s1.add(1);
        s2 = s2.add(1);
    }
}

unsafe fn cJSON_strdup(string: *const c_uchar, hooks: &InternalHooks) -> *mut c_uchar {
    if string.is_null() {
        return ptr::null_mut();
    }
    let length = c_strlen_u(string) + 1;
    let copy = alloc_with(hooks, length) as *mut c_uchar;
    if copy.is_null() {
        return ptr::null_mut();
    }
    libc::memcpy(copy as *mut c_void, string as *const c_void, length);
    copy
}

unsafe fn cJSON_New_Item(hooks: &InternalHooks) -> *mut cJSON {
    let node = alloc_with(hooks, core::mem::size_of::<cJSON>()) as *mut cJSON;
    if !node.is_null() {
        libc::memset(node as *mut c_void, 0, core::mem::size_of::<cJSON>());
    }
    node
}

// ---- Public API ----------------------------------------------------------

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_GetErrorPtr() -> *const c_char {
    (GLOBAL_ERROR_JSON.add(GLOBAL_ERROR_POSITION)) as *const c_char
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
    libc::sprintf(
        VERSION.as_mut_ptr(),
        b"%i.%i.%i\0".as_ptr() as *const c_char,
        CJSON_VERSION_MAJOR,
        CJSON_VERSION_MINOR,
        CJSON_VERSION_PATCH,
    );
    VERSION.as_ptr()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_InitHooks(hooks: *mut cJSON_Hooks) {
    if hooks.is_null() {
        GLOBAL_HOOKS.allocate = Some(rs_malloc);
        GLOBAL_HOOKS.deallocate = Some(rs_free);
        GLOBAL_HOOKS.reallocate = Some(rs_realloc);
        return;
    }
    GLOBAL_HOOKS.allocate = Some(rs_malloc);
    if let Some(mfn) = (*hooks).malloc_fn {
        GLOBAL_HOOKS.allocate = Some(mfn);
    }
    GLOBAL_HOOKS.deallocate = Some(rs_free);
    if let Some(ffn) = (*hooks).free_fn {
        GLOBAL_HOOKS.deallocate = Some(ffn);
    }
    GLOBAL_HOOKS.reallocate = None;
    let same_malloc = matches!(GLOBAL_HOOKS.allocate, Some(f) if f as usize == rs_malloc as usize);
    let same_free = matches!(GLOBAL_HOOKS.deallocate, Some(f) if f as usize == rs_free as usize);
    if same_malloc && same_free {
        GLOBAL_HOOKS.reallocate = Some(rs_realloc);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Delete(mut item: *mut cJSON) {
    while !item.is_null() {
        let next = (*item).next;
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).child.is_null() {
            cJSON_Delete((*item).child);
        }
        if ((*item).type_ & cJSON_IsReference) == 0 && !(*item).valuestring.is_null() {
            free_with(&GLOBAL_HOOKS, (*item).valuestring as *mut c_void);
            (*item).valuestring = ptr::null_mut();
        }
        if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
            free_with(&GLOBAL_HOOKS, (*item).string as *mut c_void);
            (*item).string = ptr::null_mut();
        }
        free_with(&GLOBAL_HOOKS, item as *mut c_void);
        item = next;
    }
}

// ---- Decimal point ------------------------------------------------------

unsafe fn get_decimal_point() -> c_uchar {
    // ENABLE_LOCALES is set in CMakeLists. Use locale's decimal point.
    let lconv = libc::localeconv();
    if lconv.is_null() {
        return b'.';
    }
    let dp = (*lconv).decimal_point;
    if dp.is_null() {
        return b'.';
    }
    *dp as c_uchar
}

// ---- Parse buffer -------------------------------------------------------

#[derive(Copy, Clone)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: InternalHooks,
}

#[inline]
fn can_read(buffer: &ParseBuffer, size: usize) -> bool {
    buffer.offset.checked_add(size).map_or(false, |x| x <= buffer.length)
}

#[inline]
fn can_access_at_index(buffer: &ParseBuffer, index: usize) -> bool {
    buffer.offset.checked_add(index).map_or(false, |x| x < buffer.length)
}

#[inline]
fn cannot_access_at_index(buffer: &ParseBuffer, index: usize) -> bool {
    !can_access_at_index(buffer, index)
}

#[inline]
unsafe fn buffer_at_offset(buffer: &ParseBuffer) -> *const c_uchar {
    buffer.content.add(buffer.offset)
}

// ---- parse_number ------------------------------------------------------

unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    let decimal_point = get_decimal_point();
    let mut number_string_length: usize = 0;
    let mut has_decimal_point = false;
    let mut i: usize = 0;
    loop {
        if !can_access_at_index(&*input_buffer, i) {
            break;
        }
        let c = *buffer_at_offset(&*input_buffer).add(i);
        match c {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-'
            | b'e' | b'E' => {
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
    let number_c_string =
        alloc_with(&(*input_buffer).hooks, number_string_length + 1) as *mut c_uchar;
    if number_c_string.is_null() {
        return 0;
    }
    libc::memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(&*input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point {
        let mut j: usize = 0;
        while j < number_string_length {
            if *number_c_string.add(j) == b'.' {
                *number_c_string.add(j) = decimal_point;
            }
            j += 1;
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = libc::strtod(number_c_string as *const c_char, &mut after_end);
    if number_c_string as *mut c_char == after_end {
        free_with(&(*input_buffer).hooks, number_c_string as *mut c_void);
        return 0;
    }

    (*item).valuedouble = number;
    if number >= c_int::MAX as f64 {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as f64 {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = number as c_int;
    }
    (*item).type_ = cJSON_Number;
    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    free_with(&(*input_buffer).hooks, number_c_string as *mut c_void);
    1
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_SetNumberHelper(object: *mut cJSON, number: c_double) -> c_double {
    if number >= c_int::MAX as f64 {
        (*object).valueint = c_int::MAX;
    } else if number <= c_int::MIN as f64 {
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
        || ((*object).type_ & cJSON_String) == 0
        || ((*object).type_ & cJSON_IsReference) != 0
    {
        return ptr::null_mut();
    }
    if (*object).valuestring.is_null() || valuestring.is_null() {
        return ptr::null_mut();
    }
    let v1_len = c_strlen(valuestring);
    let v2_len = c_strlen((*object).valuestring);
    if v1_len <= v2_len {
        // overlap check: !( valuestring + v1_len < object->valuestring || object->valuestring + v2_len < valuestring )
        let vs_end = valuestring.add(v1_len);
        let obj_vs = (*object).valuestring;
        let obj_vs_end = obj_vs.add(v2_len);
        let no_overlap = (vs_end as usize) < (obj_vs as usize)
            || (obj_vs_end as usize) < (valuestring as usize);
        if !no_overlap {
            return ptr::null_mut();
        }
        libc::strcpy((*object).valuestring, valuestring);
        return (*object).valuestring;
    }
    let copy = cJSON_strdup(valuestring as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char;
    if copy.is_null() {
        return ptr::null_mut();
    }
    if !(*object).valuestring.is_null() {
        cJSON_free((*object).valuestring as *mut c_void);
    }
    (*object).valuestring = copy;
    copy
}

// ---- Print buffer -------------------------------------------------------

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
    if needed > c_int::MAX as usize {
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
    if needed > (c_int::MAX as usize) / 2 {
        if needed <= c_int::MAX as usize {
            newsize = c_int::MAX as usize;
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
            free_with(&(*p).hooks, (*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
    } else {
        newbuffer = alloc_with(&(*p).hooks, newsize) as *mut c_uchar;
        if newbuffer.is_null() {
            free_with(&(*p).hooks, (*p).buffer as *mut c_void);
            (*p).length = 0;
            (*p).buffer = ptr::null_mut();
            return ptr::null_mut();
        }
        libc::memcpy(
            newbuffer as *mut c_void,
            (*p).buffer as *const c_void,
            (*p).offset + 1,
        );
        free_with(&(*p).hooks, (*p).buffer as *mut c_void);
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
    (*buffer).offset += c_strlen_u(buffer_pointer);
}

#[inline]
fn compare_double(a: f64, b: f64) -> bool {
    let max_val = if a.abs() > b.abs() { a.abs() } else { b.abs() };
    (a - b).abs() <= max_val * f64::EPSILON
}

unsafe fn print_number(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let d = (*item).valuedouble;
    let mut number_buffer: [c_uchar; 26] = [0; 26];
    let decimal_point = get_decimal_point();
    let length: c_int;

    if output_buffer.is_null() {
        return 0;
    }

    if d.is_nan() || d.is_infinite() {
        length = libc::sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"null\0".as_ptr() as *const c_char,
        );
    } else if d == (*item).valueint as f64 {
        length = libc::sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"%d\0".as_ptr() as *const c_char,
            (*item).valueint,
        );
    } else {
        let mut len = libc::sprintf(
            number_buffer.as_mut_ptr() as *mut c_char,
            b"%1.15g\0".as_ptr() as *const c_char,
            d,
        );
        let mut test: f64 = 0.0;
        let scan_ok = libc::sscanf(
            number_buffer.as_ptr() as *const c_char,
            b"%lg\0".as_ptr() as *const c_char,
            &mut test as *mut f64,
        );
        if scan_ok != 1 || !compare_double(test, d) {
            len = libc::sprintf(
                number_buffer.as_mut_ptr() as *mut c_char,
                b"%1.17g\0".as_ptr() as *const c_char,
                d,
            );
        }
        length = len;
    }

    if length < 0 || length > (number_buffer.len() as c_int - 1) {
        return 0;
    }
    let length_us = length as usize;

    let output_pointer = ensure(output_buffer, length_us + 1);
    if output_pointer.is_null() {
        return 0;
    }

    let mut i: usize = 0;
    while i < length_us {
        if number_buffer[i] == decimal_point {
            *output_pointer.add(i) = b'.';
        } else {
            *output_pointer.add(i) = number_buffer[i];
        }
        i += 1;
    }
    *output_pointer.add(i) = 0;
    (*output_buffer).offset += length_us;
    1
}

// ---- parse_string -----------------------------------------------------

unsafe fn parse_hex4(input: *const c_uchar) -> c_uint {
    let mut h: c_uint = 0;
    for i in 0..4usize {
        let c = *input.add(i);
        if c >= b'0' && c <= b'9' {
            h += c as c_uint - b'0' as c_uint;
        } else if c >= b'A' && c <= b'F' {
            h += 10 + c as c_uint - b'A' as c_uint;
        } else if c >= b'a' && c <= b'f' {
            h += 10 + c as c_uint - b'a' as c_uint;
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
    let first_sequence = input_pointer;
    if (input_end as isize) - (first_sequence as isize) < 6 {
        return 0;
    }
    let first_code = parse_hex4(first_sequence.add(2));
    if first_code >= 0xDC00 && first_code <= 0xDFFF {
        return 0;
    }
    let codepoint: u64;
    let sequence_length: c_uchar;
    if first_code >= 0xD800 && first_code <= 0xDBFF {
        let second_sequence = first_sequence.add(6);
        sequence_length = 12;
        if (input_end as isize) - (second_sequence as isize) < 6 {
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
            + ((((first_code & 0x3FF) << 10) | (second_code & 0x3FF)) as u64);
    } else {
        sequence_length = 6;
        codepoint = first_code as u64;
    }

    let utf8_length: c_uchar;
    let first_byte_mark: c_uchar;
    if codepoint < 0x80 {
        utf8_length = 1;
        first_byte_mark = 0;
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

    let mut cp = codepoint;
    let out = *output_pointer;
    let mut utf8_position = (utf8_length as i32) - 1;
    while utf8_position > 0 {
        *out.add(utf8_position as usize) = ((cp | 0x80) & 0xBF) as c_uchar;
        cp >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *out = ((cp | first_byte_mark as u64) & 0xFF) as c_uchar;
    } else {
        *out = (cp & 0x7F) as c_uchar;
    }
    *output_pointer = out.add(utf8_length as usize);
    sequence_length
}

unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut input_pointer = buffer_at_offset(&*input_buffer).add(1);
    let mut input_end = buffer_at_offset(&*input_buffer).add(1);
    let mut output: *mut c_uchar = ptr::null_mut();

    let result = (|| -> Result<(), ()> {
        if *buffer_at_offset(&*input_buffer) != b'\"' {
            return Err(());
        }

        let mut skipped_bytes: usize = 0;
        // walk to find the closing quote
        loop {
            if (input_end as usize - (*input_buffer).content as usize) >= (*input_buffer).length {
                break;
            }
            if *input_end == b'\"' {
                break;
            }
            if *input_end == b'\\' {
                if (input_end.add(1) as usize - (*input_buffer).content as usize)
                    >= (*input_buffer).length
                {
                    return Err(());
                }
                skipped_bytes += 1;
                input_end = input_end.add(1);
            }
            input_end = input_end.add(1);
        }
        if (input_end as usize - (*input_buffer).content as usize) >= (*input_buffer).length
            || *input_end != b'\"'
        {
            return Err(());
        }
        let allocation_length =
            (input_end as usize - buffer_at_offset(&*input_buffer) as usize) - skipped_bytes;
        output = alloc_with(&(*input_buffer).hooks, allocation_length + 1) as *mut c_uchar;
        if output.is_null() {
            return Err(());
        }
        Ok(())
    })();

    if result.is_err() {
        if !output.is_null() {
            free_with(&(*input_buffer).hooks, output as *mut c_void);
        }
        if !input_pointer.is_null() {
            (*input_buffer).offset = input_pointer as usize - (*input_buffer).content as usize;
        }
        return 0;
    }

    let mut output_pointer = output;

    while input_pointer < input_end {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        } else {
            let mut sequence_length: c_uchar = 2;
            if (input_end as isize) - (input_pointer as isize) < 1 {
                if !output.is_null() {
                    free_with(&(*input_buffer).hooks, output as *mut c_void);
                }
                (*input_buffer).offset =
                    input_pointer as usize - (*input_buffer).content as usize;
                return 0;
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
                b'\"' | b'\\' | b'/' => {
                    *output_pointer = *input_pointer.add(1);
                    output_pointer = output_pointer.add(1);
                }
                b'u' => {
                    sequence_length =
                        utf16_literal_to_utf8(input_pointer, input_end, &mut output_pointer);
                    if sequence_length == 0 {
                        if !output.is_null() {
                            free_with(&(*input_buffer).hooks, output as *mut c_void);
                        }
                        (*input_buffer).offset =
                            input_pointer as usize - (*input_buffer).content as usize;
                        return 0;
                    }
                }
                _ => {
                    if !output.is_null() {
                        free_with(&(*input_buffer).hooks, output as *mut c_void);
                    }
                    (*input_buffer).offset =
                        input_pointer as usize - (*input_buffer).content as usize;
                    return 0;
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
    1
}

unsafe fn print_string_ptr(input: *const c_uchar, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    if input.is_null() {
        let output = ensure(output_buffer, 3); // sizeof("\"\"") == 3
        if output.is_null() {
            return 0;
        }
        libc::strcpy(output as *mut c_char, b"\"\"\0".as_ptr() as *const c_char);
        return 1;
    }
    let mut escape_characters: usize = 0;
    let mut p = input;
    while *p != 0 {
        match *p {
            b'\"' | b'\\' | 0x08 | 0x0C | b'\n' | b'\r' | b'\t' => {
                escape_characters += 1;
            }
            _ => {
                if *p < 32 {
                    escape_characters += 5;
                }
            }
        }
        p = p.add(1);
    }
    let output_length = (p as usize - input as usize) + escape_characters;
    let output = ensure(output_buffer, output_length + 3); // sizeof("\"\"") == 3
    if output.is_null() {
        return 0;
    }
    if escape_characters == 0 {
        *output = b'\"';
        libc::memcpy(
            output.add(1) as *mut c_void,
            input as *const c_void,
            output_length,
        );
        *output.add(output_length + 1) = b'\"';
        *output.add(output_length + 2) = 0;
        return 1;
    }
    *output = b'\"';
    let mut output_pointer = output.add(1);
    let mut input_pointer = input;
    while *input_pointer != 0 {
        let c = *input_pointer;
        if c > 31 && c != b'\"' && c != b'\\' {
            *output_pointer = c;
        } else {
            *output_pointer = b'\\';
            output_pointer = output_pointer.add(1);
            match c {
                b'\\' => *output_pointer = b'\\',
                b'\"' => *output_pointer = b'\"',
                0x08 => *output_pointer = b'b',
                0x0C => *output_pointer = b'f',
                b'\n' => *output_pointer = b'n',
                b'\r' => *output_pointer = b'r',
                b'\t' => *output_pointer = b't',
                _ => {
                    libc::sprintf(
                        output_pointer as *mut c_char,
                        b"u%04x\0".as_ptr() as *const c_char,
                        c as c_uint,
                    );
                    output_pointer = output_pointer.add(4);
                }
            }
        }
        input_pointer = input_pointer.add(1);
        output_pointer = output_pointer.add(1);
    }
    *output.add(output_length + 1) = b'\"';
    *output.add(output_length + 2) = 0;
    1
}

unsafe fn print_string(item: *const cJSON, p: *mut PrintBuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const c_uchar, p)
}

unsafe fn buffer_skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }
    if cannot_access_at_index(&*buffer, 0) {
        return buffer;
    }
    while can_access_at_index(&*buffer, 0) && *buffer_at_offset(&*buffer) <= 32 {
        (*buffer).offset += 1;
    }
    if (*buffer).offset == (*buffer).length {
        (*buffer).offset -= 1;
    }
    buffer
}

unsafe fn skip_utf8_bom(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() || (*buffer).offset != 0 {
        return ptr::null_mut();
    }
    if can_access_at_index(&*buffer, 4)
        && libc::strncmp(
            buffer_at_offset(&*buffer) as *const c_char,
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
    let buffer_length = c_strlen(value) + 1;
    cJSON_ParseWithLengthOpts(value, buffer_length, return_parse_end, require_null_terminated)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ParseWithLengthOpts(
    value: *const c_char,
    buffer_length: libc::size_t,
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

    GLOBAL_ERROR_JSON = ptr::null();
    GLOBAL_ERROR_POSITION = 0;

    let mut bail = false;

    if value.is_null() || buffer_length == 0 {
        bail = true;
    }

    if !bail {
        buffer.content = value as *const c_uchar;
        buffer.length = buffer_length;
        buffer.offset = 0;
        buffer.hooks = GLOBAL_HOOKS;

        item = cJSON_New_Item(&GLOBAL_HOOKS);
        if item.is_null() {
            bail = true;
        }
    }

    if !bail {
        let bskipped = buffer_skip_whitespace(skip_utf8_bom(&mut buffer));
        if bskipped.is_null() || parse_value(item, bskipped) == 0 {
            bail = true;
        }
    }

    if !bail && require_null_terminated != 0 {
        buffer_skip_whitespace(&mut buffer);
        if buffer.offset >= buffer.length || *buffer_at_offset(&buffer) != 0 {
            bail = true;
        }
    }

    if !bail {
        if !return_parse_end.is_null() {
            *return_parse_end = buffer_at_offset(&buffer) as *const c_char;
        }
        return item;
    }

    if !item.is_null() {
        cJSON_Delete(item);
    }
    if !value.is_null() {
        let mut local_position: usize = 0;
        if buffer.offset < buffer.length {
            local_position = buffer.offset;
        } else if buffer.length > 0 {
            local_position = buffer.length - 1;
        }
        let local_json = value as *const c_uchar;
        if !return_parse_end.is_null() {
            *return_parse_end = (local_json.add(local_position)) as *const c_char;
        }
        GLOBAL_ERROR_JSON = local_json;
        GLOBAL_ERROR_POSITION = local_position;
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
    buffer_length: libc::size_t,
) -> *mut cJSON {
    cJSON_ParseWithLengthOpts(value, buffer_length, ptr::null_mut(), 0)
}

unsafe fn print_internal(
    item: *const cJSON,
    format: cJSON_bool,
    hooks: &InternalHooks,
) -> *mut c_uchar {
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

    buffer.buffer = alloc_with(hooks, default_buffer_size) as *mut c_uchar;
    buffer.length = default_buffer_size;
    buffer.format = format;
    buffer.hooks = *hooks;
    if buffer.buffer.is_null() {
        return ptr::null_mut();
    }

    let mut fail = false;
    if print_value(item, &mut buffer) == 0 {
        fail = true;
    }

    if !fail {
        update_offset(&mut buffer);

        if let Some(realloc_fn) = hooks.reallocate {
            printed = realloc_fn(buffer.buffer as *mut c_void, buffer.offset + 1) as *mut c_uchar;
            if printed.is_null() {
                fail = true;
            } else {
                buffer.buffer = ptr::null_mut();
            }
        } else {
            printed = alloc_with(hooks, buffer.offset + 1) as *mut c_uchar;
            if printed.is_null() {
                fail = true;
            } else {
                let copy_len = if buffer.length < buffer.offset + 1 {
                    buffer.length
                } else {
                    buffer.offset + 1
                };
                libc::memcpy(
                    printed as *mut c_void,
                    buffer.buffer as *const c_void,
                    copy_len,
                );
                *printed.add(buffer.offset) = 0;
                free_with(hooks, buffer.buffer as *mut c_void);
                buffer.buffer = ptr::null_mut();
            }
        }
    }

    if fail {
        if !buffer.buffer.is_null() {
            free_with(hooks, buffer.buffer as *mut c_void);
            buffer.buffer = ptr::null_mut();
        }
        if !printed.is_null() {
            free_with(hooks, printed as *mut c_void);
        }
        return ptr::null_mut();
    }

    printed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print_internal(item, 1, &GLOBAL_HOOKS) as *mut c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print_internal(item, 0, &GLOBAL_HOOKS) as *mut c_char
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
    p.buffer = alloc_with(&GLOBAL_HOOKS, prebuffer as usize) as *mut c_uchar;
    if p.buffer.is_null() {
        return ptr::null_mut();
    }
    p.length = prebuffer as usize;
    p.offset = 0;
    p.noalloc = 0;
    p.format = fmt;
    p.hooks = GLOBAL_HOOKS;
    if print_value(item, &mut p) == 0 {
        free_with(&GLOBAL_HOOKS, p.buffer as *mut c_void);
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
    let mut p = PrintBuffer {
        buffer: buffer as *mut c_uchar,
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: GLOBAL_HOOKS,
    };
    print_value(item, &mut p)
}

unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    if can_read(&*input_buffer, 4)
        && libc::strncmp(
            buffer_at_offset(&*input_buffer) as *const c_char,
            b"null\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = cJSON_NULL;
        (*input_buffer).offset += 4;
        return 1;
    }
    if can_read(&*input_buffer, 5)
        && libc::strncmp(
            buffer_at_offset(&*input_buffer) as *const c_char,
            b"false\0".as_ptr() as *const c_char,
            5,
        ) == 0
    {
        (*item).type_ = cJSON_False;
        (*input_buffer).offset += 5;
        return 1;
    }
    if can_read(&*input_buffer, 4)
        && libc::strncmp(
            buffer_at_offset(&*input_buffer) as *const c_char,
            b"true\0".as_ptr() as *const c_char,
            4,
        ) == 0
    {
        (*item).type_ = cJSON_True;
        (*item).valueint = 1;
        (*input_buffer).offset += 4;
        return 1;
    }
    if can_access_at_index(&*input_buffer, 0) && *buffer_at_offset(&*input_buffer) == b'\"' {
        return parse_string(item, input_buffer);
    }
    if can_access_at_index(&*input_buffer, 0) {
        let c = *buffer_at_offset(&*input_buffer);
        if c == b'-' || (c >= b'0' && c <= b'9') {
            return parse_number(item, input_buffer);
        }
    }
    if can_access_at_index(&*input_buffer, 0) && *buffer_at_offset(&*input_buffer) == b'[' {
        return parse_array(item, input_buffer);
    }
    if can_access_at_index(&*input_buffer, 0) && *buffer_at_offset(&*input_buffer) == b'{' {
        return parse_object(item, input_buffer);
    }
    0
}

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
            libc::strcpy(output as *mut c_char, b"null\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_False => {
            let output = ensure(output_buffer, 6);
            if output.is_null() {
                return 0;
            }
            libc::strcpy(output as *mut c_char, b"false\0".as_ptr() as *const c_char);
            1
        }
        x if x == cJSON_True => {
            let output = ensure(output_buffer, 5);
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
            let raw_length = c_strlen((*item).valuestring) + 1;
            let output = ensure(output_buffer, raw_length);
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

unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    let bail = (|| -> Result<bool, ()> {
        if *buffer_at_offset(&*input_buffer) != b'[' {
            return Err(());
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if can_access_at_index(&*input_buffer, 0) && *buffer_at_offset(&*input_buffer) == b']' {
            return Ok(true); // success branch
        }
        if cannot_access_at_index(&*input_buffer, 0) {
            (*input_buffer).offset -= 1;
            return Err(());
        }
        (*input_buffer).offset -= 1;
        loop {
            let new_item = cJSON_New_Item(&(*input_buffer).hooks);
            if new_item.is_null() {
                return Err(());
            }
            if head.is_null() {
                head = new_item;
                current_item = head;
            } else {
                (*current_item).next = new_item;
                (*new_item).prev = current_item;
                current_item = new_item;
            }
            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if parse_value(current_item, input_buffer) == 0 {
                return Err(());
            }
            buffer_skip_whitespace(input_buffer);
            if !(can_access_at_index(&*input_buffer, 0)
                && *buffer_at_offset(&*input_buffer) == b',')
            {
                break;
            }
        }
        if cannot_access_at_index(&*input_buffer, 0) || *buffer_at_offset(&*input_buffer) != b']' {
            return Err(());
        }
        Ok(false)
    })();

    match bail {
        Ok(_) => {
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).type_ = cJSON_Array;
            (*item).child = head;
            (*input_buffer).offset += 1;
            1
        }
        Err(_) => {
            if !head.is_null() {
                cJSON_Delete(head);
            }
            0
        }
    }
}

unsafe fn print_array(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    let mut current_element = (*item).child;

    if output_buffer.is_null() {
        return 0;
    }

    let output_pointer = ensure(output_buffer, 1);
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
            let output_pointer = ensure(output_buffer, length + 1);
            if output_pointer.is_null() {
                return 0;
            }
            let mut op = output_pointer;
            *op = b',';
            op = op.add(1);
            if (*output_buffer).format != 0 {
                *op = b' ';
                op = op.add(1);
            }
            *op = 0;
            (*output_buffer).offset += length;
        }
        current_element = (*current_element).next;
    }
    let output_pointer = ensure(output_buffer, 2);
    if output_pointer.is_null() {
        return 0;
    }
    let mut op = output_pointer;
    *op = b']';
    op = op.add(1);
    *op = 0;
    (*output_buffer).depth -= 1;
    1
}

unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    let result = (|| -> Result<bool, ()> {
        if cannot_access_at_index(&*input_buffer, 0) || *buffer_at_offset(&*input_buffer) != b'{' {
            return Err(());
        }
        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if can_access_at_index(&*input_buffer, 0) && *buffer_at_offset(&*input_buffer) == b'}' {
            return Ok(true);
        }
        if cannot_access_at_index(&*input_buffer, 0) {
            (*input_buffer).offset -= 1;
            return Err(());
        }
        (*input_buffer).offset -= 1;
        loop {
            let new_item = cJSON_New_Item(&(*input_buffer).hooks);
            if new_item.is_null() {
                return Err(());
            }
            if head.is_null() {
                head = new_item;
                current_item = head;
            } else {
                (*current_item).next = new_item;
                (*new_item).prev = current_item;
                current_item = new_item;
            }
            if cannot_access_at_index(&*input_buffer, 1) {
                return Err(());
            }
            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if parse_string(current_item, input_buffer) == 0 {
                return Err(());
            }
            buffer_skip_whitespace(input_buffer);
            (*current_item).string = (*current_item).valuestring;
            (*current_item).valuestring = ptr::null_mut();
            if cannot_access_at_index(&*input_buffer, 0)
                || *buffer_at_offset(&*input_buffer) != b':'
            {
                return Err(());
            }
            (*input_buffer).offset += 1;
            buffer_skip_whitespace(input_buffer);
            if parse_value(current_item, input_buffer) == 0 {
                return Err(());
            }
            buffer_skip_whitespace(input_buffer);
            if !(can_access_at_index(&*input_buffer, 0)
                && *buffer_at_offset(&*input_buffer) == b',')
            {
                break;
            }
        }
        if cannot_access_at_index(&*input_buffer, 0) || *buffer_at_offset(&*input_buffer) != b'}' {
            return Err(());
        }
        Ok(false)
    })();

    match result {
        Ok(_) => {
            (*input_buffer).depth -= 1;
            if !head.is_null() {
                (*head).prev = current_item;
            }
            (*item).type_ = cJSON_Object;
            (*item).child = head;
            (*input_buffer).offset += 1;
            1
        }
        Err(_) => {
            if !head.is_null() {
                cJSON_Delete(head);
            }
            0
        }
    }
}

unsafe fn print_object(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let mut current_item = (*item).child;
    let mut length: usize = if (*output_buffer).format != 0 { 2 } else { 1 };
    let output_pointer = ensure(output_buffer, length + 1);
    if output_pointer.is_null() {
        return 0;
    }
    let mut op = output_pointer;
    *op = b'{';
    op = op.add(1);
    (*output_buffer).depth += 1;
    if (*output_buffer).format != 0 {
        *op = b'\n';
        // op no longer needed
    }
    let _ = op;
    (*output_buffer).offset += length;

    while !current_item.is_null() {
        if (*output_buffer).format != 0 {
            let output_pointer = ensure(output_buffer, (*output_buffer).depth);
            if output_pointer.is_null() {
                return 0;
            }
            let mut op = output_pointer;
            for _i in 0..(*output_buffer).depth {
                *op = b'\t';
                op = op.add(1);
            }
            (*output_buffer).offset += (*output_buffer).depth;
        }
        if print_string_ptr((*current_item).string as *const c_uchar, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        length = if (*output_buffer).format != 0 { 2 } else { 1 };
        let output_pointer = ensure(output_buffer, length);
        if output_pointer.is_null() {
            return 0;
        }
        let mut op = output_pointer;
        *op = b':';
        op = op.add(1);
        if (*output_buffer).format != 0 {
            *op = b'\t';
        }
        let _ = op;
        (*output_buffer).offset += length;

        if print_value(current_item, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        let comma = if !(*current_item).next.is_null() { 1 } else { 0 };
        let nl = if (*output_buffer).format != 0 { 1 } else { 0 };
        length = comma + nl;
        let output_pointer = ensure(output_buffer, length + 1);
        if output_pointer.is_null() {
            return 0;
        }
        let mut op = output_pointer;
        if !(*current_item).next.is_null() {
            *op = b',';
            op = op.add(1);
        }
        if (*output_buffer).format != 0 {
            *op = b'\n';
            op = op.add(1);
        }
        *op = 0;
        (*output_buffer).offset += length;

        current_item = (*current_item).next;
    }

    let needed = if (*output_buffer).format != 0 {
        (*output_buffer).depth + 1
    } else {
        2
    };
    let output_pointer = ensure(output_buffer, needed);
    if output_pointer.is_null() {
        return 0;
    }
    let mut op = output_pointer;
    if (*output_buffer).format != 0 {
        for _i in 0..((*output_buffer).depth - 1) {
            *op = b'\t';
            op = op.add(1);
        }
    }
    *op = b'}';
    op = op.add(1);
    *op = 0;
    (*output_buffer).depth -= 1;
    1
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
            && libc::strcmp(name, (*current_element).string) != 0
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

unsafe fn create_reference(item: *const cJSON, hooks: &InternalHooks) -> *mut cJSON {
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
        core::mem::size_of::<cJSON>(),
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
    } else if !(*child).prev.is_null() {
        suffix_object((*child).prev, item);
        (*(*array).child).prev = item;
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
    hooks: &InternalHooks,
    constant_key: cJSON_bool,
) -> cJSON_bool {
    if object.is_null() || string.is_null() || item.is_null() || object == item {
        return 0;
    }
    let new_key: *mut c_char;
    let new_type: c_int;
    if constant_key != 0 {
        new_key = string as *mut c_char;
        new_type = (*item).type_ | cJSON_StringIsConst;
    } else {
        new_key = cJSON_strdup(string as *const c_uchar, hooks) as *mut c_char;
        if new_key.is_null() {
            return 0;
        }
        new_type = (*item).type_ & !cJSON_StringIsConst;
    }
    if ((*item).type_ & cJSON_StringIsConst) == 0 && !(*item).string.is_null() {
        free_with(hooks, (*item).string as *mut c_void);
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
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 0)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemToObjectCS(
    object: *mut cJSON,
    string: *const c_char,
    item: *mut cJSON,
) -> cJSON_bool {
    add_item_to_object(object, string, item, &GLOBAL_HOOKS, 1)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddItemReferenceToArray(
    array: *mut cJSON,
    item: *mut cJSON,
) -> cJSON_bool {
    if array.is_null() {
        return 0;
    }
    add_item_to_array(array, create_reference(item, &GLOBAL_HOOKS))
}

#[unsafe(no_mangle)]
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
        create_reference(item, &GLOBAL_HOOKS),
        &GLOBAL_HOOKS,
        0,
    )
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNullToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let null = cJSON_CreateNull();
    if add_item_to_object(object, name, null, &GLOBAL_HOOKS, 0) != 0 {
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
    let it = cJSON_CreateTrue();
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddFalseToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let it = cJSON_CreateFalse();
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddBoolToObject(
    object: *mut cJSON,
    name: *const c_char,
    boolean: cJSON_bool,
) -> *mut cJSON {
    let it = cJSON_CreateBool(boolean);
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddNumberToObject(
    object: *mut cJSON,
    name: *const c_char,
    number: c_double,
) -> *mut cJSON {
    let it = cJSON_CreateNumber(number);
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddStringToObject(
    object: *mut cJSON,
    name: *const c_char,
    string: *const c_char,
) -> *mut cJSON {
    let it = cJSON_CreateString(string);
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddRawToObject(
    object: *mut cJSON,
    name: *const c_char,
    raw: *const c_char,
) -> *mut cJSON {
    let it = cJSON_CreateRaw(raw);
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddObjectToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let it = cJSON_CreateObject();
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
    ptr::null_mut()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_AddArrayToObject(
    object: *mut cJSON,
    name: *const c_char,
) -> *mut cJSON {
    let it = cJSON_CreateArray();
    if add_item_to_object(object, name, it, &GLOBAL_HOOKS, 0) != 0 {
        return it;
    }
    cJSON_Delete(it);
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_ReplaceItemInArray(
    array: *mut cJSON,
    which: c_int,
    newitem: *mut cJSON,
) -> cJSON_bool {
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
    if replacement.is_null() || string.is_null() {
        return 0;
    }
    if ((*replacement).type_ & cJSON_StringIsConst) == 0 && !(*replacement).string.is_null() {
        cJSON_free((*replacement).string as *mut c_void);
    }
    (*replacement).string = cJSON_strdup(string as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNull() -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_NULL;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateTrue() -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_True;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateFalse() -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_False;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateBool(boolean: cJSON_bool) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = if boolean != 0 { cJSON_True } else { cJSON_False };
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateNumber(num: c_double) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_Number;
        (*item).valuedouble = num;
        if num >= c_int::MAX as f64 {
            (*item).valueint = c_int::MAX;
        } else if num <= c_int::MIN as f64 {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = num as c_int;
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateString(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_String;
        (*item).valuestring =
            cJSON_strdup(string as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateStringReference(string: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_String | cJSON_IsReference;
        (*item).valuestring = string as *mut c_char;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObjectReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_Object | cJSON_IsReference;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArrayReference(child: *const cJSON) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_Array | cJSON_IsReference;
        (*item).child = child as *mut cJSON;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateRaw(raw: *const c_char) -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_Raw;
        (*item).valuestring = cJSON_strdup(raw as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char;
        if (*item).valuestring.is_null() {
            cJSON_Delete(item);
            return ptr::null_mut();
        }
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateArray() -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
    if !item.is_null() {
        (*item).type_ = cJSON_Array;
    }
    item
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_CreateObject() -> *mut cJSON {
    let item = cJSON_New_Item(&GLOBAL_HOOKS);
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
pub unsafe extern "C" fn cJSON_CreateFloatArray(
    numbers: *const f32,
    count: c_int,
) -> *mut cJSON {
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
pub unsafe extern "C" fn cJSON_CreateDoubleArray(
    numbers: *const c_double,
    count: c_int,
) -> *mut cJSON {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Duplicate(item: *const cJSON, recurse: cJSON_bool) -> *mut cJSON {
    cJSON_Duplicate_rec(item, 0, recurse)
}

unsafe fn cJSON_Duplicate_rec(
    item: *const cJSON,
    depth: libc::size_t,
    recurse: cJSON_bool,
) -> *mut cJSON {
    let mut newitem: *mut cJSON = ptr::null_mut();
    let mut newchild: *mut cJSON = ptr::null_mut();

    let result = (|| -> Result<*mut cJSON, ()> {
        if item.is_null() {
            return Err(());
        }
        newitem = cJSON_New_Item(&GLOBAL_HOOKS);
        if newitem.is_null() {
            return Err(());
        }
        (*newitem).type_ = (*item).type_ & !cJSON_IsReference;
        (*newitem).valueint = (*item).valueint;
        (*newitem).valuedouble = (*item).valuedouble;
        if !(*item).valuestring.is_null() {
            (*newitem).valuestring =
                cJSON_strdup((*item).valuestring as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char;
            if (*newitem).valuestring.is_null() {
                return Err(());
            }
        }
        if !(*item).string.is_null() {
            (*newitem).string = if ((*item).type_ & cJSON_StringIsConst) != 0 {
                (*item).string
            } else {
                cJSON_strdup((*item).string as *const c_uchar, &GLOBAL_HOOKS) as *mut c_char
            };
            if (*newitem).string.is_null() {
                return Err(());
            }
        }
        if recurse == 0 {
            return Ok(newitem);
        }
        let mut child = (*item).child;
        let mut next: *mut cJSON = ptr::null_mut();
        while !child.is_null() {
            if depth >= CJSON_CIRCULAR_LIMIT {
                return Err(());
            }
            newchild = cJSON_Duplicate_rec(child, depth + 1, 1);
            if newchild.is_null() {
                return Err(());
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
        Ok(newitem)
    })();
    match result {
        Ok(p) => p,
        Err(_) => {
            if !newitem.is_null() {
                cJSON_Delete(newitem);
            }
            ptr::null_mut()
        }
    }
}

unsafe fn skip_oneline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2);
    while **input != 0 {
        if **input == b'\n' as c_char {
            *input = (*input).add(1);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn skip_multiline_comment(input: *mut *mut c_char) {
    *input = (*input).add(2);
    while **input != 0 {
        if **input == b'*' as c_char && *(*input).add(1) == b'/' as c_char {
            *input = (*input).add(2);
            return;
        }
        *input = (*input).add(1);
    }
}

unsafe fn minify_string(input: *mut *mut c_char, output: *mut *mut c_char) {
    **output = **input;
    *input = (*input).add(1);
    *output = (*output).add(1);
    while **input != 0 {
        **output = **input;
        if **input == b'\"' as c_char {
            **output = b'\"' as c_char;
            *input = (*input).add(1);
            *output = (*output).add(1);
            return;
        } else if **input == b'\\' as c_char && *(*input).add(1) == b'\"' as c_char {
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Compare(
    a: *const cJSON,
    b: *const cJSON,
    case_sensitive: cJSON_bool,
) -> cJSON_bool {
    if a.is_null() || b.is_null() || ((*a).type_ & 0xFF) != ((*b).type_ & 0xFF) {
        return 0;
    }
    let kind = (*a).type_ & 0xFF;
    match kind {
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
    match kind {
        x if x == cJSON_False || x == cJSON_True || x == cJSON_NULL => 1,
        x if x == cJSON_Number => {
            if compare_double((*a).valuedouble, (*b).valuedouble) {
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
                0
            } else {
                1
            }
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
pub unsafe extern "C" fn cJSON_malloc(size: libc::size_t) -> *mut c_void {
    alloc_with(&GLOBAL_HOOKS, size)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_free(object: *mut c_void) {
    free_with(&GLOBAL_HOOKS, object);
}
