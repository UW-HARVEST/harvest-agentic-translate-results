// Translation of c_src/src/lib.c to Rust.
// Preserves exact behavior of the original cJSON parse_number function.

#![allow(dead_code)]
#![allow(non_camel_case_types)]
#![allow(non_upper_case_globals)]

pub type cJSON_bool = i32;

pub const CJSON_TRUE: cJSON_bool = 1;
pub const CJSON_FALSE: cJSON_bool = 0;

pub const INT_MIN: i32 = i32::MIN;
pub const INT_MAX: i32 = i32::MAX;

pub const cJSON_Number: i32 = 1 << 3;

pub struct ParseBuffer<'a> {
    pub content: Option<&'a [u8]>,
    pub length: usize,
    pub offset: usize,
    pub depth: usize,
}

#[derive(Default, Debug, Clone, Copy)]
pub struct CJson {
    pub type_: i32,
    pub valueint: i32,
    pub valuedouble: f64,
}

#[inline]
fn can_access_at_index(buffer: &ParseBuffer, index: usize) -> bool {
    // Mirrors: ((buffer != NULL) && (((buffer)->offset + index) < (buffer)->length))
    // Here `buffer` is always non-null in Rust due to the reference.
    buffer.offset + index < buffer.length
}

#[inline]
fn buffer_byte_at(buffer: &ParseBuffer, index: usize) -> u8 {
    let content = buffer.content.expect("content is null");
    content[buffer.offset + index]
}

/// Parse a C-style floating point number from `s` (NUL-terminated bytes), returning
/// (parsed_value, bytes_consumed). If no conversion was performed, bytes_consumed is 0.
///
/// This mirrors the behavior of C's `strtod` for the inputs produced by `parse_number`,
/// which contain only the characters [0-9+-eE.].
fn strtod_like(s: &[u8]) -> (f64, usize) {
    // s is the temporary number buffer constructed in parse_number; it has only
    // characters: 0-9 + - e E .  followed by a single NUL terminator byte.
    // Convert to a Rust &str for parsing. We need to find the longest prefix that
    // forms a valid C floating-point literal, then parse it.
    let len = s.len();

    // Skip a leading sign
    let mut i = 0usize;
    if i < len && (s[i] == b'+' || s[i] == b'-') {
        i += 1;
    }

    // Track digits before/after decimal point and exponent
    let mut saw_digit = false;
    while i < len && s[i].is_ascii_digit() {
        saw_digit = true;
        i += 1;
    }
    if i < len && s[i] == b'.' {
        i += 1;
        while i < len && s[i].is_ascii_digit() {
            saw_digit = true;
            i += 1;
        }
    }
    if !saw_digit {
        return (0.0, 0);
    }
    let mut end = i;

    // Optional exponent
    if i < len && (s[i] == b'e' || s[i] == b'E') {
        let mut j = i + 1;
        if j < len && (s[j] == b'+' || s[j] == b'-') {
            j += 1;
        }
        let exp_digits_start = j;
        while j < len && s[j].is_ascii_digit() {
            j += 1;
        }
        if j > exp_digits_start {
            // Valid exponent
            end = j;
        }
        // else: ignore the exponent (already not committed to `end`)
    }

    let bytes = &s[..end];
    let s_str = std::str::from_utf8(bytes).unwrap_or("");
    let value: f64 = s_str.parse().unwrap_or(0.0);
    (value, end)
}

/// Parse the input text to generate a number, and populate the result into item.
pub fn parse_number(item: &mut CJson, input_buffer: &mut ParseBuffer) -> cJSON_bool {
    // (input_buffer == NULL) is impossible because input_buffer is &mut.
    // (input_buffer->content == NULL) maps to content being None.
    if input_buffer.content.is_none() {
        return CJSON_FALSE;
    }

    let mut number_string_length: usize = 0;
    let mut has_decimal_point: bool = false;

    // copy the number into a temporary buffer
    let mut i: usize = 0;
    'loop_end: loop {
        if !can_access_at_index(input_buffer, i) {
            break 'loop_end;
        }
        let c = buffer_byte_at(input_buffer, i);
        match c {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
            | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = true;
            }
            _ => {
                break 'loop_end;
            }
        }
        i += 1;
    }

    // allocate temp buffer of size number_string_length + 1 (for NUL)
    let mut number_c_string: Vec<u8> = vec![0u8; number_string_length + 1];

    // memcpy from buffer_at_offset
    {
        let content = input_buffer.content.expect("content is null");
        let src = &content[input_buffer.offset..input_buffer.offset + number_string_length];
        number_c_string[..number_string_length].copy_from_slice(src);
    }
    number_c_string[number_string_length] = 0u8; // '\0'

    if has_decimal_point {
        // Replace '.' with the decimal_point of the current locale.
        // The C code uses '.' (default C locale), so this is a no-op effectively.
        for j in 0..number_string_length {
            if number_c_string[j] == b'.' {
                number_c_string[j] = b'.';
            }
        }
    }

    let (number, consumed) = strtod_like(&number_c_string);
    if consumed == 0 {
        // parse_error: number_c_string == after_end
        return CJSON_FALSE;
    }

    item.valuedouble = number;

    // saturation
    if number >= (INT_MAX as f64) {
        item.valueint = INT_MAX;
    } else if number <= (INT_MIN as f64) {
        item.valueint = INT_MIN;
    } else {
        item.valueint = number as i32;
    }

    item.type_ = cJSON_Number;

    input_buffer.offset += consumed;
    CJSON_TRUE
}
