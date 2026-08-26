use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn calloc(count: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn malloc(size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

fn decode(c: u8) -> u8 {
    match c {
        b'A'..=b'Z' => c - b'A',
        b'a'..=b'z' => c - b'a' + 26,
        b'0'..=b'9' => c - b'0' + 52,
        b'+' => 62,
        _ => 63,
    }
}

fn is_base64(c: u8) -> bool {
    c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'=')
}

/// Decodes a NUL-terminated base64 string into a newly allocated,
/// NUL-terminated buffer.
///
/// The returned allocation uses the C allocator and must be released with
/// `free`.
///
/// # Safety
///
/// `src` must be null or point to a valid NUL-terminated byte string.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() || unsafe { *src } == 0 {
        return std::ptr::null_mut();
    }

    let source_len = unsafe { strlen(src) };
    let allocation_len = source_len.wrapping_add(1) as i32;

    let dest = unsafe { calloc(1, allocation_len.wrapping_add(13) as usize) }.cast::<c_char>();
    if dest.is_null() {
        return std::ptr::null_mut();
    }

    let buf = unsafe { malloc(allocation_len as usize) }.cast::<u8>();
    if buf.is_null() {
        unsafe { free(dest.cast()) };
        return std::ptr::null_mut();
    }

    let mut source_index = 0usize;
    let mut filtered_len = 0usize;
    loop {
        let byte = unsafe { *src.cast::<u8>().add(source_index) };
        if byte == 0 {
            break;
        }
        if is_base64(byte) {
            unsafe { *buf.add(filtered_len) = byte };
            filtered_len += 1;
        }
        source_index += 1;
    }

    let mut output = dest.cast::<u8>();
    let mut index = 0usize;
    while index < filtered_len {
        let c1 = unsafe { *buf.add(index) };
        let c2 = if index + 1 < filtered_len {
            unsafe { *buf.add(index + 1) }
        } else {
            b'A'
        };
        let c3 = if index + 2 < filtered_len {
            unsafe { *buf.add(index + 2) }
        } else {
            b'A'
        };
        let c4 = if index + 3 < filtered_len {
            unsafe { *buf.add(index + 3) }
        } else {
            b'A'
        };

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        unsafe {
            *output = (b1 << 2) | (b2 >> 4);
            output = output.add(1);

            if c3 != b'=' {
                *output = ((b2 & 0x0f) << 4) | (b3 >> 2);
                output = output.add(1);
            }

            if c4 != b'=' {
                *output = ((b3 & 0x03) << 6) | b4;
                output = output.add(1);
            }
        }

        index += 4;
    }

    unsafe { free(buf.cast()) };
    dest
}
