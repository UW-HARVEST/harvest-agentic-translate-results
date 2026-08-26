use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

fn encode(value: u8) -> c_char {
    match value {
        0..=25 => (b'A' + value) as c_char,
        26..=51 => (b'a' + value - 26) as c_char,
        52..=61 => (b'0' + value - 52) as c_char,
        62 => b'+' as c_char,
        _ => b'/' as c_char,
    }
}

/// Base64-encodes `size` bytes from `src`.
///
/// The returned allocation is owned by the caller and must be released with
/// the C allocator's `free`.
///
/// # Safety
///
/// `src` must either be null or point to data readable for the length selected
/// by the C API. If `size` is zero, `src` must be NUL-terminated.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(mut size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    if size == 0 {
        size = unsafe { strlen(src) } as c_int;
    }

    let output_size = size.wrapping_mul(4) / 3;
    let output_size = output_size.wrapping_add(4) as usize;
    let output = unsafe { calloc(1, output_size) }.cast::<c_char>();
    if output.is_null() {
        return std::ptr::null_mut();
    }

    let mut input_index: c_int = 0;
    let mut output_index: usize = 0;

    while input_index < size {
        let b1 = (unsafe { *src.add(input_index as usize) }) as u8;
        let b2 = if input_index.wrapping_add(1) < size {
            (unsafe { *src.add(input_index.wrapping_add(1) as usize) }) as u8
        } else {
            0
        };
        let b3 = if input_index.wrapping_add(2) < size {
            (unsafe { *src.add(input_index.wrapping_add(2) as usize) }) as u8
        } else {
            0
        };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x03) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0x0f) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        unsafe {
            *output.add(output_index) = encode(b4);
            *output.add(output_index + 1) = encode(b5);
            *output.add(output_index + 2) = if input_index.wrapping_add(1) < size {
                encode(b6)
            } else {
                b'=' as c_char
            };
            *output.add(output_index + 3) = if input_index.wrapping_add(2) < size {
                encode(b7)
            } else {
                b'=' as c_char
            };
        }

        input_index = input_index.wrapping_add(3);
        output_index += 4;
    }

    output
}
