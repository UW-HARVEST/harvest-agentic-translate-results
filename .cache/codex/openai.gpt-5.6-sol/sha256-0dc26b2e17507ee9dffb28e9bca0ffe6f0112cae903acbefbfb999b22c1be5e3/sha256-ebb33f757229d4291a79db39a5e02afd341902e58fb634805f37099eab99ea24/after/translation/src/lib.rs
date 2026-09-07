use std::ffi::{c_char, c_int, c_uchar, c_void};
use std::ptr;

type CJsonBool = c_int;

const CJSON_FALSE: CJsonBool = 0;
const CJSON_TRUE: CJsonBool = 1;
const CJSON_NUMBER: c_int = 1 << 3;

#[repr(C)]
pub struct ParseBuffer {
    pub content: *const c_uchar,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[repr(C)]
pub struct CJson {
    pub r#type: c_int,
    pub valueint: c_int,
    pub valuedouble: f64,
}

unsafe extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn free(pointer: *mut c_void);
    fn strtod(string: *const c_char, end_pointer: *mut *mut c_char) -> f64;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut CJson,
    input_buffer: *mut ParseBuffer,
) -> CJsonBool {
    if input_buffer.is_null() || unsafe { (*input_buffer).content.is_null() } {
        return CJSON_FALSE;
    }

    let mut number_string_length = 0usize;
    let mut has_decimal_point = false;

    while unsafe {
        (*input_buffer).offset.wrapping_add(number_string_length) < (*input_buffer).length
    } {
        let byte = unsafe {
            *(*input_buffer)
                .content
                .add((*input_buffer).offset)
                .add(number_string_length)
        };

        match byte {
            b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {
                number_string_length = number_string_length.wrapping_add(1);
            }
            b'.' => {
                number_string_length = number_string_length.wrapping_add(1);
                has_decimal_point = true;
            }
            _ => break,
        }
    }

    let number_c_string = unsafe { malloc(number_string_length.wrapping_add(1)) }.cast::<c_uchar>();
    if number_c_string.is_null() {
        return CJSON_FALSE;
    }

    unsafe {
        ptr::copy_nonoverlapping(
            (*input_buffer).content.add((*input_buffer).offset),
            number_c_string,
            number_string_length,
        );
        *number_c_string.add(number_string_length) = 0;
    }

    if has_decimal_point {
        for i in 0..number_string_length {
            if unsafe { *number_c_string.add(i) } == b'.' {
                unsafe {
                    *number_c_string.add(i) = b'.';
                }
            }
        }
    }

    let mut after_end: *mut c_char = ptr::null_mut();
    let number = unsafe {
        strtod(
            number_c_string.cast::<c_char>(),
            ptr::from_mut(&mut after_end),
        )
    };

    if number_c_string.cast::<c_char>() == after_end {
        unsafe {
            free(number_c_string.cast::<c_void>());
        }
        return CJSON_FALSE;
    }

    unsafe {
        (*item).valuedouble = number;
        (*item).valueint = if number >= c_int::MAX as f64 {
            c_int::MAX
        } else if number <= c_int::MIN as f64 {
            c_int::MIN
        } else {
            number as c_int
        };
        (*item).r#type = CJSON_NUMBER;

        let consumed = after_end.offset_from(number_c_string.cast::<c_char>()) as usize;
        (*input_buffer).offset = (*input_buffer).offset.wrapping_add(consumed);

        free(number_c_string.cast::<c_void>());
    }

    CJSON_TRUE
}
