use std::ffi::c_int;
use std::ptr;
use crate::types::*;
use crate::helpers::*;

pub(crate) unsafe fn print_number(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let d = (*item).valuedouble;
    let mut number_buffer = [0u8; 26];
    let length: c_int;

    if d.is_nan() || d.is_infinite() {
        length = libc::sprintf(number_buffer.as_mut_ptr() as *mut i8, b"null\0".as_ptr() as *const i8);
    } else if d == (*item).valueint as f64 {
        length = libc::sprintf(
            number_buffer.as_mut_ptr() as *mut i8,
            b"%d\0".as_ptr() as *const i8,
            (*item).valueint,
        );
    } else {
        length = libc::sprintf(
            number_buffer.as_mut_ptr() as *mut i8,
            b"%1.15g\0".as_ptr() as *const i8,
            d,
        );
        let mut test: f64 = 0.0;
        if libc::sscanf(
            number_buffer.as_ptr() as *const i8,
            b"%lg\0".as_ptr() as *const i8,
            &mut test,
        ) != 1
            || compare_double(test, d) == 0
        {
            libc::sprintf(
                number_buffer.as_mut_ptr() as *mut i8,
                b"%1.17g\0".as_ptr() as *const i8,
                d,
            );
        }
        // re-read length
        let _ = length; // suppress warning
    }

    // recalculate length from the buffer
    let length = libc::strlen(number_buffer.as_ptr() as *const i8) as c_int;

    if length < 0 || length > (number_buffer.len() as c_int - 1) {
        return 0;
    }

    let output_pointer = ensure(output_buffer, length as usize + 1);
    if output_pointer.is_null() {
        return 0;
    }

    let decimal_point = b'.';
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

pub(crate) unsafe fn print_string_ptr(input: *const u8, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    if input.is_null() {
        let output = ensure(output_buffer, 3);
        if output.is_null() {
            return 0;
        }
        libc::strcpy(output as *mut i8, b"\"\"\0".as_ptr() as *const i8);
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
    let output_length = (input_pointer as usize) - (input as usize) + escape_characters;

    let output = ensure(output_buffer, output_length + 3);
    if output.is_null() {
        return 0;
    }

    if escape_characters == 0 {
        *output = b'\"';
        ptr::copy_nonoverlapping(input, output.add(1), output_length);
        *output.add(output_length + 1) = b'\"';
        *output.add(output_length + 2) = 0;
        return 1;
    }

    *output = b'\"';
    let mut op = output.add(1);
    input_pointer = input;
    while *input_pointer != 0 {
        if *input_pointer > 31 && *input_pointer != b'\"' && *input_pointer != b'\\' {
            *op = *input_pointer;
        } else {
            *op = b'\\';
            op = op.add(1);
            match *input_pointer {
                b'\\' => *op = b'\\',
                b'\"' => *op = b'\"',
                b'\x08' => *op = b'b',
                b'\x0C' => *op = b'f',
                b'\n' => *op = b'n',
                b'\r' => *op = b'r',
                b'\t' => *op = b't',
                _ => {
                    libc::sprintf(op as *mut i8, b"u%04x\0".as_ptr() as *const i8, *input_pointer as c_int);
                    op = op.add(4);
                }
            }
        }
        input_pointer = input_pointer.add(1);
        op = op.add(1);
    }
    *output.add(output_length + 1) = b'\"';
    *output.add(output_length + 2) = 0;
    1
}

pub(crate) unsafe fn print_string(item: *const cJSON, p: *mut PrintBuffer) -> cJSON_bool {
    print_string_ptr((*item).valuestring as *const u8, p)
}

pub(crate) unsafe fn print_value(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if item.is_null() || output_buffer.is_null() {
        return 0;
    }
    match (*item).type_ & 0xFF {
        CJSON_NULL => {
            let output = ensure(output_buffer, 5);
            if output.is_null() { return 0; }
            libc::strcpy(output as *mut i8, b"null\0".as_ptr() as *const i8);
            1
        }
        CJSON_FALSE => {
            let output = ensure(output_buffer, 6);
            if output.is_null() { return 0; }
            libc::strcpy(output as *mut i8, b"false\0".as_ptr() as *const i8);
            1
        }
        CJSON_TRUE => {
            let output = ensure(output_buffer, 5);
            if output.is_null() { return 0; }
            libc::strcpy(output as *mut i8, b"true\0".as_ptr() as *const i8);
            1
        }
        CJSON_NUMBER => print_number(item, output_buffer),
        CJSON_RAW => {
            if (*item).valuestring.is_null() { return 0; }
            let raw_length = libc::strlen((*item).valuestring) + 1;
            let output = ensure(output_buffer, raw_length);
            if output.is_null() { return 0; }
            ptr::copy_nonoverlapping((*item).valuestring as *const u8, output, raw_length);
            1
        }
        CJSON_STRING => print_string(item, output_buffer),
        CJSON_ARRAY => super::print_compound::print_array(item, output_buffer),
        CJSON_OBJECT => super::print_compound::print_object(item, output_buffer),
        _ => 0,
    }
}
