use std::os::raw::c_int;

const CJSON_NUMBER: c_int = 1 << 3;

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
    pub valuedouble: f64,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> c_int {
    let number: f64;
    let mut after_end: *mut libc::c_char = std::ptr::null_mut();
    let decimal_point: u8 = b'.';
    let mut number_string_length: usize = 0;
    let mut has_decimal_point: c_int = 0;

    if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {
        return 0;
    }

    let buf = unsafe { &*input_buffer };

    // Count number string length and detect decimal point
    let mut i: usize = 0;
    while (buf.offset + i) < buf.length {
        let ch = unsafe { *buf.content.add(buf.offset + i) };
        match ch {
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

    // malloc temporary buffer
    let number_c_string = unsafe { libc::malloc(number_string_length + 1) } as *mut u8;
    if number_c_string.is_null() {
        return 0;
    }

    unsafe {
        std::ptr::copy_nonoverlapping(
            buf.content.add(buf.offset),
            number_c_string,
            number_string_length,
        );
        *number_c_string.add(number_string_length) = 0;
    }

    // Replace '.' with locale decimal point (which is '.' — preserving C behavior)
    if has_decimal_point != 0 {
        for j in 0..number_string_length {
            unsafe {
                if *number_c_string.add(j) == b'.' {
                    *number_c_string.add(j) = decimal_point;
                }
            }
        }
    }

    number = unsafe {
        libc::strtod(
            number_c_string as *const libc::c_char,
            &mut after_end,
        )
    };

    if number_c_string as *mut libc::c_char == after_end {
        unsafe { libc::free(number_c_string as *mut libc::c_void) };
        return 0;
    }

    let item = unsafe { &mut *item };
    item.valuedouble = number;

    if number >= i32::MAX as f64 {
        item.valueint = i32::MAX;
    } else if number <= i32::MIN as f64 {
        item.valueint = i32::MIN;
    } else {
        item.valueint = number as c_int;
    }

    item.type_ = CJSON_NUMBER;

    unsafe {
        (*input_buffer).offset += (after_end as usize) - (number_c_string as usize);
        libc::free(number_c_string as *mut libc::c_void);
    }

    1
}
