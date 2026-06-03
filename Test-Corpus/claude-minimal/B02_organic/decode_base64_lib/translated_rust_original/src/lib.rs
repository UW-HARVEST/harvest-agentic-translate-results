use std::ffi::{c_char, CStr};
use std::ptr;

/// Decode a single base64 character to its 6-bit value.
fn decode(c: u8) -> u8 {
    if c >= b'A' && c <= b'Z' {
        return c - b'A';
    }
    if c >= b'a' && c <= b'z' {
        return c - b'a' + 26;
    }
    if c >= b'0' && c <= b'9' {
        return c - b'0' + 52;
    }
    if c == b'+' {
        return 62;
    }
    63
}

/// Returns true if `c` is a valid base64 character.
fn is_base64(c: u8) -> bool {
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
}

/// Decode the base64 encoded C-string `src` and return a NUL-terminated
/// heap-allocated C-string. Returns NULL on error or if input is empty.
///
/// # Safety
///
/// `src` must either be NULL or a valid pointer to a NUL-terminated C string.
/// The returned pointer (if non-null) is allocated by this library and must be
/// freed with `free_decoded_base64` (or compatible logic) to avoid leaks.
#[no_mangle]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return ptr::null_mut();
    }

    let cstr = CStr::from_ptr(src);
    let bytes = cstr.to_bytes();

    if bytes.is_empty() {
        return ptr::null_mut();
    }

    // Filter out non-base64 characters as per the POSIX standard.
    let buf: Vec<u8> = bytes.iter().copied().filter(|&c| is_base64(c)).collect();

    // Allocate destination buffer. The decoded size is always less than the
    // source. We allocate len + 14 to mirror the original C code's
    // `calloc(sizeof(char), l + 13)` where `l = strlen(src) + 1`.
    let src_len = bytes.len();
    let dest_capacity = src_len + 14;
    let mut dest: Vec<u8> = vec![0u8; dest_capacity];

    let mut out_idx: usize = 0;
    let l = buf.len();
    let mut k: usize = 0;
    while k < l {
        let c1: u8 = buf[k];
        let mut c2: u8 = b'A';
        let mut c3: u8 = b'A';
        let mut c4: u8 = b'A';

        if k + 1 < l {
            c2 = buf[k + 1];
        }

        if k + 2 < l {
            c3 = buf[k + 2];
        }

        if k + 3 < l {
            c4 = buf[k + 3];
        }

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        dest[out_idx] = (b1 << 2) | (b2 >> 4);
        out_idx += 1;

        if c3 != b'=' {
            dest[out_idx] = ((b2 & 0xf) << 4) | (b3 >> 2);
            out_idx += 1;
        }

        if c4 != b'=' {
            dest[out_idx] = ((b3 & 0x3) << 6) | b4;
            out_idx += 1;
        }

        k += 4;
    }

    // Ensure NUL terminator (already zero from initialization, but be explicit).
    if out_idx < dest.len() {
        dest[out_idx] = 0;
    }

    // Hand ownership of the buffer to the caller as a raw pointer. Convert to
    // a boxed slice so capacity matches length, then leak.
    let boxed = dest.into_boxed_slice();
    let raw = Box::into_raw(boxed) as *mut c_char;
    raw
}

/// Free a buffer previously returned by `decode_base64`.
///
/// # Safety
///
/// `ptr` must be a pointer previously returned by `decode_base64`, or NULL.
/// The associated allocation length must be `original_len`, which equals the
/// length of the input plus 14.
#[no_mangle]
pub unsafe extern "C" fn free_decoded_base64(ptr: *mut c_char, original_len: usize) {
    if ptr.is_null() {
        return;
    }
    let slice = std::slice::from_raw_parts_mut(ptr as *mut u8, original_len);
    let _ = Box::from_raw(slice as *mut [u8]);
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::ffi::CString;

    fn decode_to_string(input: &str) -> Option<String> {
        let c_input = CString::new(input).unwrap();
        unsafe {
            let result = decode_base64(c_input.as_ptr());
            if result.is_null() {
                return None;
            }
            let decoded = CStr::from_ptr(result).to_string_lossy().into_owned();
            // Reconstruct allocation length to free.
            let original_len = input.len() + 1 + 14;
            free_decoded_base64(result, original_len);
            Some(decoded)
        }
    }

    #[test]
    fn test_basic() {
        assert_eq!(decode_to_string("SGVsbG8="), Some("Hello".to_string()));
        assert_eq!(decode_to_string("Zm9v"), Some("foo".to_string()));
        assert_eq!(
            decode_to_string("Zm9vYmFy"),
            Some("foobar".to_string())
        );
    }

    #[test]
    fn test_empty() {
        assert_eq!(decode_to_string(""), None);
    }

    #[test]
    fn test_null() {
        unsafe {
            assert!(decode_base64(ptr::null()).is_null());
        }
    }
}
