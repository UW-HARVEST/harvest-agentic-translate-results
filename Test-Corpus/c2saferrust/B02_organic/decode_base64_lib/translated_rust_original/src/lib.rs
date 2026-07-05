


extern "C" {
    fn malloc(__size: size_t) -> *mut ::core::ffi::c_void;
    fn calloc(__nmemb: size_t, __size: size_t) -> *mut ::core::ffi::c_void;
    fn free(__ptr: *mut ::core::ffi::c_void);
    fn strlen(__s: *const ::core::ffi::c_char) -> size_t;
}
pub type size_t = usize;
pub const NULL: *mut ::core::ffi::c_void = ::core::ptr::null_mut::<::core::ffi::c_void>();
pub const TRUE: ::core::ffi::c_int = 1 as ::core::ffi::c_int;
pub const FALSE: ::core::ffi::c_int = 0 as ::core::ffi::c_int;
fn decode(c: i8) -> u8 {
    if c >= b'A' as i8 && c <= b'Z' as i8 {
        (c - b'A' as i8) as u8
    } else if c >= b'a' as i8 && c <= b'z' as i8 {
        (c - b'a' as i8) as u8 + 26
    } else if c >= b'0' as i8 && c <= b'9' as i8 {
        (c - b'0' as i8) as u8 + 52
    } else if c == b'+' as i8 {
        62
    } else {
        63
    }
}

fn is_base64(c: ::core::ffi::c_char) -> ::core::ffi::c_int {
    let c = c as u8;
    if c.is_ascii_alphanumeric() || matches!(c, b'+' | b'/' | b'=') {
        TRUE
    } else {
        FALSE
    }
}

#[no_mangle]
pub fn decode_base64(src: &str) -> Option<String> {
    if src.is_empty() {
        return None;
    }

    let mut buf = Vec::with_capacity(src.len());
    for ch in src.bytes() {
        if unsafe { is_base64(ch as ::core::ffi::c_char) } != 0 {
            buf.push(ch);
        }
    }

    let mut dest = Vec::with_capacity(buf.len().saturating_add(13));
    let mut k = 0;

    while k < buf.len() {
        let c1 = buf[k] as ::core::ffi::c_char;
        let c2 = if k + 1 < buf.len() {
            buf[k + 1] as ::core::ffi::c_char
        } else {
            'A' as ::core::ffi::c_char
        };
        let c3 = if k + 2 < buf.len() {
            buf[k + 2] as ::core::ffi::c_char
        } else {
            'A' as ::core::ffi::c_char
        };
        let c4 = if k + 3 < buf.len() {
            buf[k + 3] as ::core::ffi::c_char
        } else {
            'A' as ::core::ffi::c_char
        };

        let b1 = unsafe { decode(c1) } as u8;
        let b2 = unsafe { decode(c2) } as u8;
        let b3 = unsafe { decode(c3) } as u8;
        let b4 = unsafe { decode(c4) } as u8;

        dest.push((b1 << 2) | (b2 >> 4));

        if c3 as u8 != b'=' {
            dest.push(((b2 & 0x0f) << 4) | (b3 >> 2));
        }
        if c4 as u8 != b'=' {
            dest.push(((b3 & 0x03) << 6) | b4);
        }

        k += 4;
    }

    Some(String::from_utf8_lossy(&dest).into_owned())
}

