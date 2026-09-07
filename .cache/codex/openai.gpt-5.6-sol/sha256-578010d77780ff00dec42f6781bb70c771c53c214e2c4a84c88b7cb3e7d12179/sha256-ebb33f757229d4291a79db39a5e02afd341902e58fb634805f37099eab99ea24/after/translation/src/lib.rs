use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(mut size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    if size == 0 {
        size = unsafe { strlen(src) } as c_int;
    }

    let allocation_size = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4) as usize;
    let out = unsafe { calloc(1, allocation_size) }.cast::<c_char>();
    if out.is_null() {
        return std::ptr::null_mut();
    }

    let mut input_offset: c_int = 0;
    let mut output_offset: usize = 0;

    while input_offset < size {
        let index = input_offset as usize;
        let b1 = unsafe { *src.add(index) } as u8;
        let b2 = if input_offset + 1 < size {
            (unsafe { *src.add(index + 1) }) as u8
        } else {
            0
        };
        let b3 = if input_offset + 2 < size {
            (unsafe { *src.add(index + 2) }) as u8
        } else {
            0
        };

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x03) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0x0f) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        unsafe {
            *out.add(output_offset) = encode(b4);
            *out.add(output_offset + 1) = encode(b5);
            *out.add(output_offset + 2) = if input_offset + 1 < size {
                encode(b6)
            } else {
                b'=' as c_char
            };
            *out.add(output_offset + 3) = if input_offset + 2 < size {
                encode(b7)
            } else {
                b'=' as c_char
            };
        }

        input_offset += 3;
        output_offset += 4;
    }

    out
}
