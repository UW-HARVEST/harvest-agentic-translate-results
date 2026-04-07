use libc::{c_int, c_double, size_t, strtod, malloc, free, memcpy};
use std::ptr;

type CJsonBool = c_int;

const CJSON_NUMBER: c_int = 1 << 3;
const INT_MIN: c_int = c_int::MIN;
const INT_MAX: c_int = c_int::MAX;

#[repr(C)]
pub struct ParseBuffer {
    pub content: *const u8,
    pub length: size_t,
    pub offset: size_t,
    pub depth: size_t,
}

#[repr(C)]
pub struct CJson {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *const CJson,
    input_buffer: *const ParseBuffer,
) -> CJsonBool {
    let number: c_double;
    let mut after_end: *mut u8 = ptr::null_mut();
    let decimal_point: u8 = b'.';
    let mut number_string_length: size_t = 0;
    let mut has_decimal_point: CJsonBool = 0;

    if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {
        return 0;
    }

    let buf = unsafe { &*input_buffer };

    // count number string length
    let mut i: size_t = 0;
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

    let number_c_string = unsafe { malloc(number_string_length + 1) as *mut u8 };
    if number_c_string.is_null() {
        return 0;
    }

    unsafe {
        memcpy(
            number_c_string as *mut libc::c_void,
            buf.content.add(buf.offset) as *const libc::c_void,
            number_string_length,
        );
        *number_c_string.add(number_string_length) = 0;
    }

    if has_decimal_point != 0 {
        for i in 0..number_string_length {
            unsafe {
                if *number_c_string.add(i) == b'.' {
                    *number_c_string.add(i) = decimal_point;
                }
            }
        }
    }

    number = unsafe {
        strtod(
            number_c_string as *const libc::c_char,
            &mut after_end as *mut *mut u8 as *mut *mut libc::c_char,
        )
    };

    if number_c_string as *const u8 == after_end as *const u8 {
        unsafe { free(number_c_string as *mut libc::c_void) };
        return 0;
    }

    let item = unsafe { &mut *(item as *mut CJson) };
    item.valuedouble = number;

    if number >= INT_MAX as c_double {
        item.valueint = INT_MAX;
    } else if number <= INT_MIN as c_double {
        item.valueint = INT_MIN;
    } else {
        item.valueint = number as c_int;
    }

    item.type_ = CJSON_NUMBER;

    unsafe {
        (*input_buffer.cast_mut()).offset +=
            (after_end as usize) - (number_c_string as usize);
        free(number_c_string as *mut libc::c_void);
    }

    1
}
