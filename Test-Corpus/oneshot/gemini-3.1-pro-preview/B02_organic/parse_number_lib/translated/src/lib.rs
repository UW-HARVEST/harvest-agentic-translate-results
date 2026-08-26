use std::os::raw::{c_char, c_double, c_int, c_uchar};

pub type cJSON_bool = c_int;

pub const CJSON_NUMBER: c_int = 1 << 3;

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
    pub valuedouble: c_double,
}

extern "C" {
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

#[unsafe(no_mangle)]
pub extern "C" fn parse_number(item: *mut cJSON, input_buffer: *mut parse_buffer) -> cJSON_bool {
    unsafe {
        if input_buffer.is_null() || (*input_buffer).content.is_null() {
            return 0;
        }

        let buffer = &mut *input_buffer;
        let mut number_string_length = 0;
        let mut has_decimal_point = false;

        let mut i = 0;
        while buffer.offset + i < buffer.length {
            let c = *buffer.content.add(buffer.offset + i);
            match c {
                b'0'..=b'9' | b'+' | b'-' | b'e' | b'E' => {
                    number_string_length += 1;
                }
                b'.' => {
                    number_string_length += 1;
                    has_decimal_point = true;
                }
                _ => break,
            }
            i += 1;
        }

        let mut number_c_string = Vec::with_capacity(number_string_length + 1);
        let src_ptr = buffer.content.add(buffer.offset);
        for j in 0..number_string_length {
            let mut c = *src_ptr.add(j);
            if has_decimal_point && c == b'.' {
                c = b'.';
            }
            number_c_string.push(c);
        }
        number_c_string.push(0);

        let mut after_end: *mut c_char = std::ptr::null_mut();
        let number = strtod(
            number_c_string.as_ptr() as *const c_char,
            &mut after_end,
        );

        if number_c_string.as_ptr() as *mut c_char == after_end {
            return 0;
        }

        if !item.is_null() {
            let item_ref = &mut *item;
            item_ref.valuedouble = number;

            if number >= c_int::MAX as f64 {
                item_ref.valueint = c_int::MAX;
            } else if number <= c_int::MIN as f64 {
                item_ref.valueint = c_int::MIN;
            } else {
                item_ref.valueint = number as c_int;
            }

            item_ref.type_ = CJSON_NUMBER;
        }

        buffer.offset += (after_end as usize) - (number_c_string.as_ptr() as usize);

        1
    }
}
