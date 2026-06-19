use std::ffi::{c_char, c_double, c_int};
use std::ptr;

type CjsonBool = c_int;

const CJSON_FALSE: CjsonBool = 0;
const CJSON_TRUE: CjsonBool = 1;
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
    pub valuedouble: c_double,
}

unsafe extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

unsafe fn can_access_at_index(buffer: *const parse_buffer, index: usize) -> bool {
    if buffer.is_null() {
        return false;
    }

    unsafe { (*buffer).offset.wrapping_add(index) < (*buffer).length }
}

unsafe fn buffer_at_offset(buffer: *const parse_buffer) -> *const u8 {
    unsafe { (*buffer).content.wrapping_add((*buffer).offset) }
}

/* Parse the input text to generate a number, and populate the result into item. */
#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut cJSON,
    input_buffer: *mut parse_buffer,
) -> CjsonBool {
    let number: c_double;
    let mut after_end: *mut c_char = ptr::null_mut();
    let decimal_point = b'.';
    let mut i: usize = 0;
    let mut number_string_length: usize = 0;
    let mut has_decimal_point = false;

    unsafe {
        if input_buffer.is_null() || (*input_buffer).content.is_null() {
            return CJSON_FALSE;
        }

        while can_access_at_index(input_buffer, i) {
            match *buffer_at_offset(input_buffer).add(i) {
                b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9' | b'+'
                | b'-' | b'e' | b'E' => {
                    number_string_length += 1;
                }
                b'.' => {
                    number_string_length += 1;
                    has_decimal_point = true;
                }
                _ => {
                    break;
                }
            }
            i += 1;
        }

        let mut number_c_string = Vec::<u8>::new();
        if number_c_string.try_reserve_exact(number_string_length + 1).is_err() {
            return CJSON_FALSE;
        }

        number_c_string.extend_from_slice(std::slice::from_raw_parts(
            buffer_at_offset(input_buffer),
            number_string_length,
        ));
        number_c_string.push(0);

        if has_decimal_point {
            for byte in number_c_string.iter_mut().take(number_string_length) {
                if *byte == b'.' {
                    *byte = decimal_point;
                }
            }
        }

        let number_c_string_ptr = number_c_string.as_mut_ptr();
        number = strtod(
            number_c_string_ptr.cast::<c_char>(),
            ptr::addr_of_mut!(after_end),
        );
        if number_c_string_ptr.cast::<c_char>() == after_end {
            return CJSON_FALSE;
        }

        (*item).valuedouble = number;

        if number >= c_int::MAX as c_double {
            (*item).valueint = c_int::MAX;
        } else if number <= c_int::MIN as c_double {
            (*item).valueint = c_int::MIN;
        } else {
            (*item).valueint = number as c_int;
        }

        (*item).type_ = CJSON_NUMBER;

        (*input_buffer).offset = (*input_buffer).offset.wrapping_add(
            after_end.offset_from(number_c_string_ptr.cast::<c_char>()) as usize,
        );

        CJSON_TRUE
    }
}
