use crate::internal::*;
use std::ffi::{c_char, c_double, c_int, c_uchar};
use std::ptr;

#[derive(Clone, Copy)]
struct PrintBuffer {
    buffer: *mut c_uchar,
    length: usize,
    offset: usize,
    depth: usize,
    noalloc: cJSON_bool,
    format: cJSON_bool,
    hooks: InternalHooks,
}

unsafe fn ensure(buffer: *mut PrintBuffer, additional: usize) -> *mut c_uchar {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return ptr::null_mut();
    }
    if (*buffer).length > 0 && (*buffer).offset >= (*buffer).length {
        return ptr::null_mut();
    }
    if additional > c_int::MAX as usize {
        return ptr::null_mut();
    }
    let needed = additional.wrapping_add((*buffer).offset).wrapping_add(1);
    if needed <= (*buffer).length {
        return (*buffer).buffer.add((*buffer).offset);
    }
    if (*buffer).noalloc != 0 {
        return ptr::null_mut();
    }

    let new_size = if needed > c_int::MAX as usize / 2 {
        if needed <= c_int::MAX as usize {
            c_int::MAX as usize
        } else {
            return ptr::null_mut();
        }
    } else {
        needed * 2
    };

    let new_buffer = if (*buffer).hooks.reallocate.is_some() {
        reallocate(&(*buffer).hooks, (*buffer).buffer.cast(), new_size) as *mut c_uchar
    } else {
        let allocated = allocate(&(*buffer).hooks, new_size) as *mut c_uchar;
        if !allocated.is_null() {
            memcpy(
                allocated.cast(),
                (*buffer).buffer.cast(),
                (*buffer).offset + 1,
            );
            deallocate(&(*buffer).hooks, (*buffer).buffer.cast());
        }
        allocated
    };

    if new_buffer.is_null() {
        deallocate(&(*buffer).hooks, (*buffer).buffer.cast());
        (*buffer).length = 0;
        (*buffer).buffer = ptr::null_mut();
        return ptr::null_mut();
    }
    (*buffer).length = new_size;
    (*buffer).buffer = new_buffer;
    new_buffer.add((*buffer).offset)
}

unsafe fn update_offset(buffer: *mut PrintBuffer) {
    if buffer.is_null() || (*buffer).buffer.is_null() {
        return;
    }
    (*buffer).offset += strlen((*buffer).buffer.add((*buffer).offset).cast());
}

fn compare_double(left: c_double, right: c_double) -> bool {
    let maximum = left.abs().max(right.abs());
    (left - right).abs() <= maximum * c_double::EPSILON
}

unsafe fn print_number(item: *const cJSON, output: *mut PrintBuffer) -> cJSON_bool {
    if output.is_null() {
        return 0;
    }
    let number = (*item).valuedouble;
    let mut number_buffer = [0u8; 26];
    let length = if number.is_nan() || number.is_infinite() {
        sprintf(number_buffer.as_mut_ptr().cast(), c"null".as_ptr())
    } else if number == (*item).valueint as c_double {
        sprintf(
            number_buffer.as_mut_ptr().cast(),
            c"%d".as_ptr(),
            (*item).valueint,
        )
    } else {
        let mut length = sprintf(
            number_buffer.as_mut_ptr().cast(),
            c"%1.15g".as_ptr(),
            number,
        );
        let mut test = 0.0;
        if sscanf(
            number_buffer.as_ptr().cast(),
            c"%lg".as_ptr(),
            &mut test as *mut c_double,
        ) != 1
            || !compare_double(test, number)
        {
            length = sprintf(
                number_buffer.as_mut_ptr().cast(),
                c"%1.17g".as_ptr(),
                number,
            );
        }
        length
    };

    if length < 0 || length > number_buffer.len() as c_int - 1 {
        return 0;
    }
    let destination = ensure(output, length as usize + 1);
    if destination.is_null() {
        return 0;
    }
    for index in 0..length as usize {
        *destination.add(index) = number_buffer[index];
    }
    *destination.add(length as usize) = 0;
    (*output).offset += length as usize;
    1
}

unsafe fn print_string_ptr(input: *const c_uchar, output: *mut PrintBuffer) -> cJSON_bool {
    if output.is_null() {
        return 0;
    }
    if input.is_null() {
        let destination = ensure(output, 3);
        if destination.is_null() {
            return 0;
        }
        strcpy(destination.cast(), c"\"\"".as_ptr());
        return 1;
    }

    let mut input_pointer = input;
    let mut escapes = 0usize;
    while *input_pointer != 0 {
        match *input_pointer {
            b'"' | b'\\' | 8 | 12 | b'\n' | b'\r' | b'\t' => escapes += 1,
            byte if byte < 32 => escapes += 5,
            _ => {}
        }
        input_pointer = input_pointer.add(1);
    }
    let output_length = input_pointer.offset_from(input) as usize + escapes;
    let destination = ensure(output, output_length + 3);
    if destination.is_null() {
        return 0;
    }

    if escapes == 0 {
        *destination = b'"';
        memcpy(destination.add(1).cast(), input.cast(), output_length);
        *destination.add(output_length + 1) = b'"';
        *destination.add(output_length + 2) = 0;
        return 1;
    }

    *destination = b'"';
    let mut destination_pointer = destination.add(1);
    input_pointer = input;
    while *input_pointer != 0 {
        let byte = *input_pointer;
        if byte > 31 && byte != b'"' && byte != b'\\' {
            *destination_pointer = byte;
        } else {
            *destination_pointer = b'\\';
            destination_pointer = destination_pointer.add(1);
            match byte {
                b'\\' => *destination_pointer = b'\\',
                b'"' => *destination_pointer = b'"',
                8 => *destination_pointer = b'b',
                12 => *destination_pointer = b'f',
                b'\n' => *destination_pointer = b'n',
                b'\r' => *destination_pointer = b'r',
                b'\t' => *destination_pointer = b't',
                _ => {
                    sprintf(destination_pointer.cast(), c"u%04x".as_ptr(), byte as c_int);
                    destination_pointer = destination_pointer.add(4);
                }
            }
        }
        input_pointer = input_pointer.add(1);
        destination_pointer = destination_pointer.add(1);
    }
    *destination.add(output_length + 1) = b'"';
    *destination.add(output_length + 2) = 0;
    1
}

unsafe fn print_array(item: *const cJSON, output: *mut PrintBuffer) -> cJSON_bool {
    if output.is_null() {
        return 0;
    }
    let mut destination = ensure(output, 1);
    if destination.is_null() {
        return 0;
    }
    *destination = b'[';
    (*output).offset += 1;
    (*output).depth += 1;

    let mut current = (*item).child;
    while !current.is_null() {
        if print_value(current, output) == 0 {
            return 0;
        }
        update_offset(output);
        if !(*current).next.is_null() {
            let length = if (*output).format != 0 { 2 } else { 1 };
            destination = ensure(output, length + 1);
            if destination.is_null() {
                return 0;
            }
            *destination = b',';
            destination = destination.add(1);
            if (*output).format != 0 {
                *destination = b' ';
                destination = destination.add(1);
            }
            *destination = 0;
            (*output).offset += length;
        }
        current = (*current).next;
    }

    destination = ensure(output, 2);
    if destination.is_null() {
        return 0;
    }
    *destination = b']';
    *destination.add(1) = 0;
    (*output).depth -= 1;
    1
}

unsafe fn print_object(item: *const cJSON, output: *mut PrintBuffer) -> cJSON_bool {
    if output.is_null() {
        return 0;
    }
    let mut length = if (*output).format != 0 { 2 } else { 1 };
    let mut destination = ensure(output, length + 1);
    if destination.is_null() {
        return 0;
    }
    *destination = b'{';
    destination = destination.add(1);
    (*output).depth += 1;
    if (*output).format != 0 {
        *destination = b'\n';
    }
    (*output).offset += length;

    let mut current = (*item).child;
    while !current.is_null() {
        if (*output).format != 0 {
            destination = ensure(output, (*output).depth);
            if destination.is_null() {
                return 0;
            }
            for index in 0..(*output).depth {
                *destination.add(index) = b'\t';
            }
            (*output).offset += (*output).depth;
        }
        if print_string_ptr((*current).string.cast(), output) == 0 {
            return 0;
        }
        update_offset(output);

        length = if (*output).format != 0 { 2 } else { 1 };
        destination = ensure(output, length);
        if destination.is_null() {
            return 0;
        }
        *destination = b':';
        destination = destination.add(1);
        if (*output).format != 0 {
            *destination = b'\t';
        }
        (*output).offset += length;

        if print_value(current, output) == 0 {
            return 0;
        }
        update_offset(output);

        length = usize::from((*output).format != 0) + usize::from(!(*current).next.is_null());
        destination = ensure(output, length + 1);
        if destination.is_null() {
            return 0;
        }
        if !(*current).next.is_null() {
            *destination = b',';
            destination = destination.add(1);
        }
        if (*output).format != 0 {
            *destination = b'\n';
            destination = destination.add(1);
        }
        *destination = 0;
        (*output).offset += length;
        current = (*current).next;
    }

    destination = ensure(
        output,
        if (*output).format != 0 {
            (*output).depth + 1
        } else {
            2
        },
    );
    if destination.is_null() {
        return 0;
    }
    if (*output).format != 0 {
        for _ in 0..(*output).depth - 1 {
            *destination = b'\t';
            destination = destination.add(1);
        }
    }
    *destination = b'}';
    *destination.add(1) = 0;
    (*output).depth -= 1;
    1
}

unsafe fn print_value(item: *const cJSON, output: *mut PrintBuffer) -> cJSON_bool {
    if item.is_null() || output.is_null() {
        return 0;
    }
    let literal = |output: *mut PrintBuffer, value: *const c_char, length: usize| unsafe {
        let destination = ensure(output, length);
        if destination.is_null() {
            0
        } else {
            strcpy(destination.cast(), value);
            1
        }
    };
    match (*item).type_ & 0xff {
        CJSON_NULL => literal(output, c"null".as_ptr(), 5),
        CJSON_FALSE => literal(output, c"false".as_ptr(), 6),
        CJSON_TRUE => literal(output, c"true".as_ptr(), 5),
        CJSON_NUMBER => print_number(item, output),
        CJSON_RAW => {
            if (*item).valuestring.is_null() {
                return 0;
            }
            let length = strlen((*item).valuestring) + 1;
            let destination = ensure(output, length);
            if destination.is_null() {
                0
            } else {
                memcpy(destination.cast(), (*item).valuestring.cast(), length);
                1
            }
        }
        CJSON_STRING => print_string_ptr((*item).valuestring.cast(), output),
        CJSON_ARRAY => print_array(item, output),
        CJSON_OBJECT => print_object(item, output),
        _ => 0,
    }
}

unsafe fn print(item: *const cJSON, format: cJSON_bool, hooks: &InternalHooks) -> *mut c_uchar {
    let mut buffer = PrintBuffer {
        buffer: allocate(hooks, 256) as *mut c_uchar,
        length: 256,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: *hooks,
    };
    if buffer.buffer.is_null() || print_value(item, &mut buffer) == 0 {
        if !buffer.buffer.is_null() {
            deallocate(hooks, buffer.buffer.cast());
        }
        return ptr::null_mut();
    }
    update_offset(&mut buffer);

    let printed = if hooks.reallocate.is_some() {
        let result = reallocate(hooks, buffer.buffer.cast(), buffer.offset + 1) as *mut c_uchar;
        if !result.is_null() {
            buffer.buffer = ptr::null_mut();
        }
        result
    } else {
        let result = allocate(hooks, buffer.offset + 1) as *mut c_uchar;
        if !result.is_null() {
            memcpy(
                result.cast(),
                buffer.buffer.cast(),
                buffer.length.min(buffer.offset + 1),
            );
            *result.add(buffer.offset) = 0;
            deallocate(hooks, buffer.buffer.cast());
            buffer.buffer = ptr::null_mut();
        }
        result
    };

    if printed.is_null() {
        if !buffer.buffer.is_null() {
            deallocate(hooks, buffer.buffer.cast());
        }
        return ptr::null_mut();
    }
    printed
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_Print(item: *const cJSON) -> *mut c_char {
    print(item, 1, &GLOBAL_HOOKS).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintUnformatted(item: *const cJSON) -> *mut c_char {
    print(item, 0, &GLOBAL_HOOKS).cast()
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn cJSON_PrintBuffered(
    item: *const cJSON,
    prebuffer: c_int,
    format: cJSON_bool,
) -> *mut c_char {
    if prebuffer < 0 {
        return ptr::null_mut();
    }
    let mut buffer = PrintBuffer {
        buffer: allocate(&GLOBAL_HOOKS, prebuffer as usize) as *mut c_uchar,
        length: prebuffer as usize,
        offset: 0,
        depth: 0,
        noalloc: 0,
        format,
        hooks: GLOBAL_HOOKS,
    };
    if buffer.buffer.is_null() {
        return ptr::null_mut();
    }
    if print_value(item, &mut buffer) == 0 {
        deallocate(&GLOBAL_HOOKS, buffer.buffer.cast());
        return ptr::null_mut();
    }
    buffer.buffer.cast()
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
    let mut print_buffer = PrintBuffer {
        buffer: buffer.cast(),
        length: length as usize,
        offset: 0,
        depth: 0,
        noalloc: 1,
        format,
        hooks: GLOBAL_HOOKS,
    };
    print_value(item, &mut print_buffer)
}
