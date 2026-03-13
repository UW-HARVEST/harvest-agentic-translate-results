use std::ptr;
use crate::types::*;
use crate::helpers::*;
use super::print_core::*;

pub(crate) unsafe fn print_array(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let mut current_element = (*item).child;

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
            let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
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
    *output_pointer.add(1) = 0;
    (*output_buffer).depth -= 1;
    1
}

pub(crate) unsafe fn print_object(item: *const cJSON, output_buffer: *mut PrintBuffer) -> cJSON_bool {
    if output_buffer.is_null() {
        return 0;
    }
    let mut current_item = (*item).child;

    let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
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

        // print key
        if print_string_ptr((*current_item).string as *const u8, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        let length = if (*output_buffer).format != 0 { 2usize } else { 1usize };
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

        // print value
        if print_value(current_item, output_buffer) == 0 {
            return 0;
        }
        update_offset(output_buffer);

        // print comma if not last
        let length = (if (*output_buffer).format != 0 { 1usize } else { 0usize })
            + (if !(*current_item).next.is_null() { 1usize } else { 0usize });
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
    *output_pointer.add(1) = 0;
    (*output_buffer).depth -= 1;
    1
}
