use std::ffi::{c_char, c_int, c_double};
use std::os::raw::c_void;
use std::ptr;

pub type cJSON_bool = c_int;

pub const true_: cJSON_bool = 1;
pub const false_: cJSON_bool = 0;

pub const INT_MIN: c_int = i32::MIN;
pub const INT_MAX: c_int = i32::MAX;

pub const cJSON_Number: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const u8,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && unsafe { (*buffer).offset + index < (*buffer).length }
}

fn buffer_at_offset(buffer: *const parse_buffer) -> *const u8 {
    unsafe { (*buffer).content.add((*buffer).offset) }
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {
        return false_;
    }

    let mut number_string_length: usize = 0;
    let mut has_decimal_point: cJSON_bool = false_;

    let mut i: usize = 0;
    while can_access_at_index(input_buffer, i) {
        let c = unsafe { *buffer_at_offset(input_buffer).add(i) };
        match c {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = true_;
            }
            _ => break,
        }
        i += 1;
    }

    let number_c_string = unsafe {
        libc::malloc(number_string_length + 1) as *mut u8
    };
    if number_c_string.is_null() {
        return false_;
    }

    unsafe {
        ptr::copy_nonoverlapping(buffer_at_offset(input_buffer), number_c_string, number_string_length);
        *number_c_string.add(number_string_length) = 0;
    }

    if has_decimal_point != 0 {
        let decimal_point = b'.';
        for j in 0..number_string_length {
            unsafe {
                if *number_c_string.add(j) == b'.' {
                    *number_c_string.add(j) = decimal_point;
                }
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = unsafe {
        libc::strtod(number_c_string as *const c_char, &mut after_end)
    };

    if number_c_string as *mut c_char == after_end {
        unsafe { libc::free(number_c_string as *mut c_void) };
        return false_;
    }

    unsafe {
        (*item).valuedouble = number;

        if number >= INT_MAX as c_double {
            (*item).valueint = INT_MAX;
        } else if number <= INT_MIN as c_double {
            (*item).valueint = INT_MIN;
        } else {
            (*item).valueint = number as c_int;
        }

        (*item).type_ = cJSON_Number;

        (*input_buffer).offset = (*input_buffer).offset.wrapping_add((after_end as usize).wrapping_sub(number_c_string as usize));

        libc::free(number_c_string as *mut c_void);
    }

    true_
}
