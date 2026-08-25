use crate::internal::*;
use crate::tree::cJSON_Delete;
use std::ffi::{c_char, c_uchar};
use std::ptr;

#[derive(Clone, Copy)]
struct ParseBuffer {
    content: *const c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    hooks: InternalHooks,
}

#[inline]
fn can_read(buffer: *const ParseBuffer, size: usize) -> bool {
    !buffer.is_null() && unsafe { (*buffer).offset.wrapping_add(size) <= (*buffer).length }
}

#[inline]
fn can_access(buffer: *const ParseBuffer, index: usize) -> bool {
    !buffer.is_null() && unsafe { (*buffer).offset.wrapping_add(index) < (*buffer).length }
}

#[inline]
unsafe fn at(buffer: *const ParseBuffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

unsafe fn parse_number(item: *mut cJSON, input: *mut ParseBuffer) -> cJSON_bool {
    if input.is_null() || (*input).content.is_null() {
        return 0;
    }

    let mut length = 0usize;
    let mut has_decimal = false;
    while can_access(input, length) {
        match *at(input).add(length) {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => length += 1,
            b'.' => {
                length += 1;
                has_decimal = true;
            }
            _ => break,
        }
    }

    let number_string = allocate(&(*input).hooks, length + 1) as *mut c_uchar;
    if number_string.is_null() {
        return 0;
    }
    memcpy(number_string.cast(), at(input).cast(), length);
    *number_string.add(length) = 0;

    if has_decimal {
        for index in 0..length {
            if *number_string.add(index) == b'.' {
                *number_string.add(index) = b'.';
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = strtod(number_string.cast(), &mut after_end);
    if number_string.cast::<c_char>() == after_end {
        deallocate(&(*input).hooks, number_string.cast());
        return 0;
    }

    (*item).valuedouble = number;
    (*item).valueint = clamp_int(number);
    (*item).type_ = CJSON_NUMBER;
    (*input).offset = (*input)
        .offset
        .wrapping_add(after_end.offset_from(number_string.cast()) as usize);
    deallocate(&(*input).hooks, number_string.cast());
    1
}

fn parse_hex4(input: *const c_uchar) -> u32 {
    let mut value = 0u32;
    for index in 0..4 {
        let byte = unsafe { *input.add(index) };
        value += match byte {
            b'0'..=b'9' => (byte - b'0') as u32,
            b'A'..=b'F' => (10 + byte - b'A') as u32,
            b'a'..=b'f' => (10 + byte - b'a') as u32,
            _ => return 0,
        };
        if index < 3 {
            value <<= 4;
        }
    }
    value
}

unsafe fn utf16_literal_to_utf8(
    input: *const c_uchar,
    input_end: *const c_uchar,
    output: &mut *mut c_uchar,
) -> c_uchar {
    if input_end.offset_from(input) < 6 {
        return 0;
    }
    let first = parse_hex4(input.add(2));
    if (0xdc00..=0xdfff).contains(&first) {
        return 0;
    }

    let (mut codepoint, sequence_length) = if (0xd800..=0xdbff).contains(&first) {
        let second_sequence = input.add(6);
        if input_end.offset_from(second_sequence) < 6
            || *second_sequence != b'\\'
            || *second_sequence.add(1) != b'u'
        {
            return 0;
        }
        let second = parse_hex4(second_sequence.add(2));
        if !(0xdc00..=0xdfff).contains(&second) {
            return 0;
        }
        (
            0x10000u64 + ((((first & 0x3ff) << 10) | (second & 0x3ff)) as u64),
            12,
        )
    } else {
        (first as u64, 6)
    };

    let (utf8_length, first_mark) = if codepoint < 0x80 {
        (1u8, 0u8)
    } else if codepoint < 0x800 {
        (2, 0xc0)
    } else if codepoint < 0x10000 {
        (3, 0xe0)
    } else if codepoint <= 0x10ffff {
        (4, 0xf0)
    } else {
        return 0;
    };

    let mut position = utf8_length - 1;
    while position > 0 {
        *(*output).add(position as usize) = ((codepoint | 0x80) & 0xbf) as u8;
        codepoint >>= 6;
        position -= 1;
    }
    **output = if utf8_length > 1 {
        ((codepoint | first_mark as u64) & 0xff) as u8
    } else {
        (codepoint & 0x7f) as u8
    };
    *output = (*output).add(utf8_length as usize);
    sequence_length
}

unsafe fn parse_string(item: *mut cJSON, input: *mut ParseBuffer) -> cJSON_bool {
    let mut input_pointer = at(input).add(1);
    let mut input_end = at(input).add(1);
    if *at(input) != b'"' {
        (*input).offset = input_pointer.offset_from((*input).content) as usize;
        return 0;
    }

    let mut skipped = 0usize;
    while input_end.offset_from((*input).content) < (*input).length as isize && *input_end != b'"' {
        if *input_end == b'\\' {
            if input_end.add(1).offset_from((*input).content) >= (*input).length as isize {
                (*input).offset = input_pointer.offset_from((*input).content) as usize;
                return 0;
            }
            skipped += 1;
            input_end = input_end.add(1);
        }
        input_end = input_end.add(1);
    }
    if input_end.offset_from((*input).content) >= (*input).length as isize || *input_end != b'"' {
        (*input).offset = input_pointer.offset_from((*input).content) as usize;
        return 0;
    }

    let allocation_length = input_end.offset_from(at(input)) as usize - skipped;
    let output = allocate(&(*input).hooks, allocation_length + 1) as *mut c_uchar;
    if output.is_null() {
        (*input).offset = input_pointer.offset_from((*input).content) as usize;
        return 0;
    }

    let mut output_pointer = output;
    while input_pointer < input_end {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        } else {
            let mut sequence_length = 2u8;
            if input_end.offset_from(input_pointer) < 1 {
                deallocate(&(*input).hooks, output.cast());
                (*input).offset = input_pointer.offset_from((*input).content) as usize;
                return 0;
            }
            match *input_pointer.add(1) {
                b'b' => {
                    *output_pointer = 8;
                    output_pointer = output_pointer.add(1);
                }
                b'f' => {
                    *output_pointer = 12;
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
                        deallocate(&(*input).hooks, output.cast());
                        (*input).offset = input_pointer.offset_from((*input).content) as usize;
                        return 0;
                    }
                }
                _ => {
                    deallocate(&(*input).hooks, output.cast());
                    (*input).offset = input_pointer.offset_from((*input).content) as usize;
                    return 0;
                }
            }
            input_pointer = input_pointer.add(sequence_length as usize);
        }
    }

    *output_pointer = 0;
    (*item).type_ = CJSON_STRING;
    (*item).valuestring = output.cast();
    (*input).offset = input_end.offset_from((*input).content) as usize + 1;
    1
}

unsafe fn skip_whitespace(buffer: *mut ParseBuffer) -> *mut ParseBuffer {
    if buffer.is_null() || (*buffer).content.is_null() {
        return ptr::null_mut();
    }
    if !can_access(buffer, 0) {
        return buffer;
    }
    while can_access(buffer, 0) && *at(buffer) <= 32 {
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
    if can_access(buffer, 4) && strncmp(at(buffer).cast(), c"\xEF\xBB\xBF".as_ptr(), 3) == 0 {
        (*buffer).offset += 3;
    }
    buffer
}

unsafe fn parse_value(item: *mut cJSON, input: *mut ParseBuffer) -> cJSON_bool {
    if input.is_null() || (*input).content.is_null() {
        return 0;
    }
    if can_read(input, 4) && strncmp(at(input).cast(), c"null".as_ptr(), 4) == 0 {
        (*item).type_ = CJSON_NULL;
        (*input).offset += 4;
        return 1;
    }
    if can_read(input, 5) && strncmp(at(input).cast(), c"false".as_ptr(), 5) == 0 {
        (*item).type_ = CJSON_FALSE;
        (*input).offset += 5;
        return 1;
    }
    if can_read(input, 4) && strncmp(at(input).cast(), c"true".as_ptr(), 4) == 0 {
        (*item).type_ = CJSON_TRUE;
        (*item).valueint = 1;
        (*input).offset += 4;
        return 1;
    }
    if can_access(input, 0) && *at(input) == b'"' {
        return parse_string(item, input);
    }
    if can_access(input, 0) && (*at(input) == b'-' || (*at(input)).is_ascii_digit()) {
        return parse_number(item, input);
    }
    if can_access(input, 0) && *at(input) == b'[' {
        return parse_array(item, input);
    }
    if can_access(input, 0) && *at(input) == b'{' {
        return parse_object(item, input);
    }
    0
}

unsafe fn parse_array(item: *mut cJSON, input: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current: *mut cJSON = ptr::null_mut();
    if (*input).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input).depth += 1;
    if *at(input) != b'[' {
        return 0;
    }
    (*input).offset += 1;
    skip_whitespace(input);
    if can_access(input, 0) && *at(input) == b']' {
        (*input).depth -= 1;
        (*item).type_ = CJSON_ARRAY;
        (*item).child = head;
        (*input).offset += 1;
        return 1;
    }
    if !can_access(input, 0) {
        (*input).offset -= 1;
        return 0;
    }
    (*input).offset -= 1;

    loop {
        let new_item = new_item(&(*input).hooks);
        if new_item.is_null() {
            if !head.is_null() {
                cJSON_Delete(head);
            }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current = new_item;
        } else {
            (*current).next = new_item;
            (*new_item).prev = current;
            current = new_item;
        }
        (*input).offset += 1;
        skip_whitespace(input);
        if parse_value(current, input) == 0 {
            cJSON_Delete(head);
            return 0;
        }
        skip_whitespace(input);
        if !(can_access(input, 0) && *at(input) == b',') {
            break;
        }
    }
    if !can_access(input, 0) || *at(input) != b']' {
        cJSON_Delete(head);
        return 0;
    }
    (*input).depth -= 1;
    if !head.is_null() {
        (*head).prev = current;
    }
    (*item).type_ = CJSON_ARRAY;
    (*item).child = head;
    (*input).offset += 1;
    1
}

unsafe fn parse_object(item: *mut cJSON, input: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current: *mut cJSON = ptr::null_mut();
    if (*input).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input).depth += 1;
    if !can_access(input, 0) || *at(input) != b'{' {
        return 0;
    }
    (*input).offset += 1;
    skip_whitespace(input);
    if can_access(input, 0) && *at(input) == b'}' {
        (*input).depth -= 1;
        (*item).type_ = CJSON_OBJECT;
        (*item).child = head;
        (*input).offset += 1;
        return 1;
    }
    if !can_access(input, 0) {
        (*input).offset -= 1;
        return 0;
    }
    (*input).offset -= 1;

    loop {
        let new_item = new_item(&(*input).hooks);
        if new_item.is_null() {
            if !head.is_null() {
                cJSON_Delete(head);
            }
            return 0;
        }
        if head.is_null() {
            head = new_item;
            current = new_item;
        } else {
            (*current).next = new_item;
            (*new_item).prev = current;
            current = new_item;
        }
        if !can_access(input, 1) {
            cJSON_Delete(head);
            return 0;
        }
        (*input).offset += 1;
        skip_whitespace(input);
        if parse_string(current, input) == 0 {
            cJSON_Delete(head);
            return 0;
        }
        skip_whitespace(input);
        (*current).string = (*current).valuestring;
        (*current).valuestring = ptr::null_mut();
        if !can_access(input, 0) || *at(input) != b':' {
            cJSON_Delete(head);
            return 0;
        }
        (*input).offset += 1;
        skip_whitespace(input);
        if parse_value(current, input) == 0 {
            cJSON_Delete(head);
            return 0;
        }
        skip_whitespace(input);
        if !(can_access(input, 0) && *at(input) == b',') {
            break;
        }
    }
    if !can_access(input, 0) || *at(input) != b'}' {
        cJSON_Delete(head);
        return 0;
    }
    (*input).depth -= 1;
    if !head.is_null() {
        (*head).prev = current;
    }
    (*item).type_ = CJSON_OBJECT;
    (*item).child = head;
    (*input).offset += 1;
    1
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
    cJSON_ParseWithLengthOpts(
        value,
        strlen(value) + 1,
        return_parse_end,
        require_null_terminated,
    )
}

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
    GLOBAL_ERROR = Error {
        json: ptr::null(),
        position: 0,
    };

    let mut failed = value.is_null() || buffer_length == 0;
    if !failed {
        buffer.content = value.cast();
        buffer.length = buffer_length;
        buffer.hooks = GLOBAL_HOOKS;
        item = new_item(&GLOBAL_HOOKS);
        failed = item.is_null();
    }
    if !failed {
        failed = parse_value(item, skip_whitespace(skip_utf8_bom(&mut buffer))) == 0;
    }
    if !failed && require_null_terminated != 0 {
        skip_whitespace(&mut buffer);
        failed = buffer.offset >= buffer.length || *at(&buffer) != 0;
    }
    if !failed {
        if !return_parse_end.is_null() {
            *return_parse_end = at(&buffer).cast();
        }
        return item;
    }

    if !item.is_null() {
        cJSON_Delete(item);
    }
    if !value.is_null() {
        let position = if buffer.offset < buffer.length {
            buffer.offset
        } else if buffer.length > 0 {
            buffer.length - 1
        } else {
            0
        };
        if !return_parse_end.is_null() {
            *return_parse_end = value.add(position);
        }
        GLOBAL_ERROR = Error {
            json: value.cast(),
            position,
        };
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
