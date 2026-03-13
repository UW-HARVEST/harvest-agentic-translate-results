use std::ffi::c_int;
use std::ptr;
use crate::types::*;
use crate::helpers::*;

pub(crate) unsafe fn parse_number(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    let decimal_point = b'.';
    let mut i: usize = 0;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point = false;

    while can_access_at_index(input_buffer, i) {
        match *buffer_at_offset(input_buffer).add(i) {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => number_string_length += 1,
            b'.' => {
                number_string_length += 1;
                has_decimal_point = true;
            }
            _ => break,
        }
        i += 1;
    }

    let number_c_string = ((*input_buffer).hooks.allocate)(number_string_length + 1) as *mut u8;
    if number_c_string.is_null() {
        return 0;
    }
    ptr::copy_nonoverlapping(buffer_at_offset(input_buffer), number_c_string, number_string_length);
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point {
        for j in 0..number_string_length {
            if *number_c_string.add(j) == b'.' {
                *number_c_string.add(j) = decimal_point;
            }
        }
    }

    let mut after_end: *mut i8 = ptr::null_mut();
    let number = libc::strtod(number_c_string as *const i8, &mut after_end);
    if number_c_string as *mut i8 == after_end {
        ((*input_buffer).hooks.deallocate)(number_c_string as *mut _);
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
    (*item).type_ = CJSON_NUMBER;

    (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
    ((*input_buffer).hooks.deallocate)(number_c_string as *mut _);
    1
}

pub(crate) unsafe fn parse_hex4(input: *const u8) -> u32 {
    let mut h: u32 = 0;
    for i in 0..4u32 {
        let c = *input.add(i as usize);
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

pub(crate) unsafe fn utf16_literal_to_utf8(
    input_pointer: *const u8,
    input_end: *const u8,
    output_pointer: *mut *mut u8,
) -> u8 {
    let mut codepoint: u64;
    let first_sequence = input_pointer;
    let mut utf8_length: u8;
    let mut first_byte_mark: u8 = 0;
    let mut sequence_length: u8;

    if (input_end as usize) - (first_sequence as usize) < 6 {
        return 0;
    }

    let first_code = parse_hex4(first_sequence.add(2));
    if first_code >= 0xDC00 && first_code <= 0xDFFF {
        return 0;
    }

    if first_code >= 0xD800 && first_code <= 0xDBFF {
        let second_sequence = first_sequence.add(6);
        sequence_length = 12;
        if (input_end as usize) - (second_sequence as usize) < 6 {
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
        *(*output_pointer).add(utf8_position as usize) = ((codepoint | 0x80) & 0xBF) as u8;
        codepoint >>= 6;
        utf8_position -= 1;
    }
    if utf8_length > 1 {
        *(*output_pointer).add(0) = ((codepoint | first_byte_mark as u64) & 0xFF) as u8;
    } else {
        *(*output_pointer).add(0) = (codepoint & 0x7F) as u8;
    }

    *output_pointer = (*output_pointer).add(utf8_length as usize);
    sequence_length
}

pub(crate) unsafe fn parse_string(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if *buffer_at_offset(input_buffer) != b'\"' {
        // not a string
        if !buffer_at_offset(input_buffer).is_null() {
            (*input_buffer).offset =
                (buffer_at_offset(input_buffer) as usize) - ((*input_buffer).content as usize);
        }
        return 0;
    }

    let mut input_pointer = buffer_at_offset(input_buffer).add(1);
    let mut input_end = buffer_at_offset(input_buffer).add(1);
    let mut skipped_bytes: usize = 0;

    while ((input_end as usize) - ((*input_buffer).content as usize)) < (*input_buffer).length
        && *input_end != b'\"'
    {
        if *input_end == b'\\' {
            if ((input_end.add(1) as usize) - ((*input_buffer).content as usize))
                >= (*input_buffer).length
            {
                // prevent buffer overflow
                (*input_buffer).offset =
                    (input_pointer as usize) - ((*input_buffer).content as usize);
                return 0;
            }
            skipped_bytes += 1;
            input_end = input_end.add(1);
        }
        input_end = input_end.add(1);
    }

    if ((input_end as usize) - ((*input_buffer).content as usize)) >= (*input_buffer).length
        || *input_end != b'\"'
    {
        (*input_buffer).offset = (input_pointer as usize) - ((*input_buffer).content as usize);
        return 0;
    }

    let allocation_length =
        ((input_end as usize) - (buffer_at_offset(input_buffer) as usize)) - skipped_bytes;
    let output = ((*input_buffer).hooks.allocate)(allocation_length + 1) as *mut u8;
    if output.is_null() {
        (*input_buffer).offset = (input_pointer as usize) - ((*input_buffer).content as usize);
        return 0;
    }

    let mut output_pointer = output;
    while (input_pointer as usize) < (input_end as usize) {
        if *input_pointer != b'\\' {
            *output_pointer = *input_pointer;
            output_pointer = output_pointer.add(1);
            input_pointer = input_pointer.add(1);
        } else {
            if (input_end as usize) - (input_pointer as usize) < 1 {
                ((*input_buffer).hooks.deallocate)(output as *mut _);
                (*input_buffer).offset =
                    (input_pointer as usize) - ((*input_buffer).content as usize);
                return 0;
            }
            let mut sequence_length: u8 = 2;
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
                        ((*input_buffer).hooks.deallocate)(output as *mut _);
                        (*input_buffer).offset =
                            (input_pointer as usize) - ((*input_buffer).content as usize);
                        return 0;
                    }
                }
                _ => {
                    ((*input_buffer).hooks.deallocate)(output as *mut _);
                    (*input_buffer).offset =
                        (input_pointer as usize) - ((*input_buffer).content as usize);
                    return 0;
                }
            }
            input_pointer = input_pointer.add(sequence_length as usize);
        }
    }

    *output_pointer = 0;
    (*item).type_ = CJSON_STRING;
    (*item).valuestring = output as *mut i8;
    (*input_buffer).offset = (input_end as usize) - ((*input_buffer).content as usize);
    (*input_buffer).offset += 1;
    1
}

pub(crate) unsafe fn parse_value(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }
    // null
    if can_read(input_buffer, 4)
        && libc::strncmp(buffer_at_offset(input_buffer) as *const i8, b"null\0".as_ptr() as *const i8, 4) == 0
    {
        (*item).type_ = CJSON_NULL;
        (*input_buffer).offset += 4;
        return 1;
    }
    // false
    if can_read(input_buffer, 5)
        && libc::strncmp(buffer_at_offset(input_buffer) as *const i8, b"false\0".as_ptr() as *const i8, 5) == 0
    {
        (*item).type_ = CJSON_FALSE;
        (*input_buffer).offset += 5;
        return 1;
    }
    // true
    if can_read(input_buffer, 4)
        && libc::strncmp(buffer_at_offset(input_buffer) as *const i8, b"true\0".as_ptr() as *const i8, 4) == 0
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
        let c = *buffer_at_offset(input_buffer);
        if c == b'-' || (c >= b'0' && c <= b'9') {
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

pub(crate) unsafe fn parse_array(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if *buffer_at_offset(input_buffer) != b'[' {
        // fail
        if !head.is_null() { crate::api::cJSON_Delete(head); }
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

    if !can_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        if !head.is_null() { crate::api::cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset -= 1;
    loop {
        let new_item = cjson_new_item(&(*input_buffer).hooks);
        if new_item.is_null() {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
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
        buffer_skip_whitespace(input_buffer);
        if parse_value(current_item, input_buffer) == 0 {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if !can_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b']' {
        if !head.is_null() { crate::api::cJSON_Delete(head); }
        return 0;
    }

    // success
    (*input_buffer).depth -= 1;
    if !head.is_null() {
        (*head).prev = current_item;
    }
    (*item).type_ = CJSON_ARRAY;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}

pub(crate) unsafe fn parse_object(item: *mut cJSON, input_buffer: *mut ParseBuffer) -> cJSON_bool {
    let mut head: *mut cJSON = ptr::null_mut();
    let mut current_item: *mut cJSON = ptr::null_mut();

    if (*input_buffer).depth >= CJSON_NESTING_LIMIT {
        return 0;
    }
    (*input_buffer).depth += 1;

    if !can_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'{' {
        if !head.is_null() { crate::api::cJSON_Delete(head); }
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

    if !can_access_at_index(input_buffer, 0) {
        (*input_buffer).offset -= 1;
        if !head.is_null() { crate::api::cJSON_Delete(head); }
        return 0;
    }

    (*input_buffer).offset -= 1;
    loop {
        let new_item = cjson_new_item(&(*input_buffer).hooks);
        if new_item.is_null() {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }
        if head.is_null() {
            current_item = new_item;
            head = new_item;
        } else {
            (*current_item).next = new_item;
            (*new_item).prev = current_item;
            current_item = new_item;
        }

        if !can_access_at_index(input_buffer, 1) {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }

        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_string(current_item, input_buffer) == 0 {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        // swap valuestring and string
        (*current_item).string = (*current_item).valuestring;
        (*current_item).valuestring = ptr::null_mut();

        if !can_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b':' {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }

        (*input_buffer).offset += 1;
        buffer_skip_whitespace(input_buffer);
        if parse_value(current_item, input_buffer) == 0 {
            if !head.is_null() { crate::api::cJSON_Delete(head); }
            return 0;
        }
        buffer_skip_whitespace(input_buffer);

        if !(can_access_at_index(input_buffer, 0) && *buffer_at_offset(input_buffer) == b',') {
            break;
        }
    }

    if !can_access_at_index(input_buffer, 0) || *buffer_at_offset(input_buffer) != b'}' {
        if !head.is_null() { crate::api::cJSON_Delete(head); }
        return 0;
    }

    // success
    (*input_buffer).depth -= 1;
    if !head.is_null() {
        (*head).prev = current_item;
    }
    (*item).type_ = CJSON_OBJECT;
    (*item).child = head;
    (*input_buffer).offset += 1;
    1
}
