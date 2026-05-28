use std::ffi::{c_char, c_double, c_int, c_uchar, c_void};

type CJsonBool = c_int;
type SizeT = libc::size_t;

const C_JSON_NUMBER: c_int = 1 << 3;

const C_TRUE: CJsonBool = 1;
const C_FALSE: CJsonBool = 0;

#[repr(C)]
pub struct ParseBuffer {
    pub content: *const c_uchar,
    pub length: SizeT,
    pub offset: SizeT,
    pub depth: SizeT,
}

#[repr(C)]
pub struct CJson {
    pub type_: c_int,
    pub valueint: c_int,
    pub valuedouble: c_double,
}

extern "C" {
    fn malloc(size: SizeT) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn memcpy(dest: *mut c_void, src: *const c_void, n: SizeT) -> *mut c_void;
    fn strtod(nptr: *const c_char, endptr: *mut *mut c_char) -> c_double;
}

#[inline]
unsafe fn can_access_at_index(buffer: *const ParseBuffer, index: SizeT) -> bool {
    !buffer.is_null() && ((*buffer).offset + index) < (*buffer).length
}

#[inline]
unsafe fn buffer_at_offset(buffer: *const ParseBuffer) -> *const c_uchar {
    (*buffer).content.add((*buffer).offset)
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn parse_number(
    item: *mut CJson,
    input_buffer: *mut ParseBuffer,
) -> CJsonBool {
    let mut number: c_double = 0.0;
    let mut after_end: *mut c_uchar = std::ptr::null_mut();
    let number_c_string: *mut c_uchar;
    let decimal_point: c_uchar = b'.';
    let mut i: SizeT;
    let mut number_string_length: SizeT = 0;
    let mut has_decimal_point: CJsonBool = C_FALSE;

    if input_buffer.is_null() || (*input_buffer).content.is_null() {
        return C_FALSE;
    }

    /* copy the number into a temporary buffer and replace '.' with the decimal point
     * of the current locale (for strtod)
     * This also takes care of '\0' not necessarily being available for marking the end of the input */
    i = 0;
    'loop_end: loop {
        if !can_access_at_index(input_buffer, i) {
            break 'loop_end;
        }
        let byte = *buffer_at_offset(input_buffer).add(i);
        match byte {
            b'0' | b'1' | b'2' | b'3' | b'4' | b'5' | b'6' | b'7' | b'8' | b'9'
            | b'+' | b'-' | b'e' | b'E' => {
                number_string_length += 1;
            }
            b'.' => {
                number_string_length += 1;
                has_decimal_point = C_TRUE;
            }
            _ => {
                break 'loop_end;
            }
        }
        i += 1;
    }

    /* malloc for temporary buffer, add 1 for '\0' */
    number_c_string = malloc(number_string_length + 1) as *mut c_uchar;
    if number_c_string.is_null() {
        return C_FALSE; /* allocation failure */
    }

    memcpy(
        number_c_string as *mut c_void,
        buffer_at_offset(input_buffer) as *const c_void,
        number_string_length,
    );
    *number_c_string.add(number_string_length) = b'\0';

    if has_decimal_point != C_FALSE {
        i = 0;
        while i < number_string_length {
            if *number_c_string.add(i) == b'.' {
                /* replace '.' with the decimal point of the current locale (for strtod) */
                *number_c_string.add(i) = decimal_point;
            }
            i += 1;
        }
    }

    number = strtod(
        number_c_string as *const c_char,
        &mut after_end as *mut *mut c_uchar as *mut *mut c_char,
    );
    if number_c_string == after_end {
        /* free the temporary buffer */
        free(number_c_string as *mut c_void);
        return C_FALSE; /* parse_error */
    }

    (*item).valuedouble = number;

    /* use saturation in case of overflow */
    if number >= c_int::MAX as c_double {
        (*item).valueint = c_int::MAX;
    } else if number <= c_int::MIN as c_double {
        (*item).valueint = c_int::MIN;
    } else {
        (*item).valueint = number as c_int;
    }

    (*item).type_ = C_JSON_NUMBER;

    (*input_buffer).offset += (after_end as usize - number_c_string as usize) as SizeT;
    /* free the temporary buffer */
    free(number_c_string as *mut c_void);
    C_TRUE
}
