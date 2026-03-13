use std::ffi::c_int;
use std::os::raw::c_uchar;

const CJSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct parse_buffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
pub struct cJSON {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: f64,
}

unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

extern "C" {
    fn strtod(nptr: *const i8, endptr: *mut *mut i8) -> f64;
    fn malloc(size: usize) -> *mut u8;
    fn free(ptr: *mut u8);
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *const cJSON,
    input_buffer: *const parse_buffer,
) -> c_int {
    let number: f64;
    let mut after_end: *mut u8 = std::ptr::null_mut();
    let number_c_string: *mut u8;
    let decimal_point: u8 = b'.';
    let mut i: usize;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: c_int = 0;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return 0;
    }

    // Count number string length
    i = 0;
    while can_access_at_index(input_buffer, i) {
        match *buffer_at_offset(input_buffer).add(i) {
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

    // malloc for temporary buffer, add 1 for '\0'
    number_c_string = malloc(number_string_length + 1);
    if number_c_string.is_null() {
        return 0;
    }

    std::ptr::copy_nonoverlapping(buffer_at_offset(input_buffer), number_c_string, number_string_length);
    *number_c_string.add(number_string_length) = 0;

    if has_decimal_point != 0 {
        for j in 0..number_string_length {
            if *number_c_string.add(j) == b'.' {
                *number_c_string.add(j) = decimal_point;
            }
        }
    }

    number = strtod(number_c_string as *const i8, &mut after_end as *mut *mut u8 as *mut *mut i8);
    if number_c_string == after_end {
        free(number_c_string);
        return 0;
    }

    let item = &mut *(item as *mut cJSON);
    item.valuedouble = number;

    if number >= i32::MAX as f64 {
        item.valueint = i32::MAX;
    } else if number <= i32::MIN as f64 {
        item.valueint = i32::MIN;
    } else {
        item.valueint = number as c_int;
    }

    item.type_ = CJSON_NUMBER;

    (*(input_buffer as *mut parse_buffer)).offset += after_end as usize - number_c_string as usize;
    free(number_c_string);
    1
}
