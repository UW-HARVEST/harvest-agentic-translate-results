use std::ffi::c_char;
use std::os::raw::c_int;

/// Decode a base64 character
fn decode(c: c_char) -> u8 {
    let c = c as i32;
    if c >= b'A' as i32 && c <= b'Z' as i32 {
        return (c - b'A' as i32) as u8;
    }
    if c >= b'a' as i32 && c <= b'z' as i32 {
        return (c - b'a' as i32 + 26) as u8;
    }
    if c >= b'0' as i32 && c <= b'9' as i32 {
        return (c - b'0' as i32 + 52) as u8;
    }
    if c == b'+' as i32 {
        return 62;
    }
    63
}

/// Returns TRUE (1) if 'c' is a valid base64 character, otherwise FALSE (0)
fn is_base64(c: c_char) -> c_int {
    let c = c as i32;
    if (c >= b'A' as i32 && c <= b'Z' as i32)
        || (c >= b'a' as i32 && c <= b'z' as i32)
        || (c >= b'0' as i32 && c <= b'9' as i32)
        || c == b'+' as i32
        || c == b'/' as i32
        || c == b'=' as i32
    {
        return 1;
    }
    0
}

/// Decode the base64 encoded string 'src' into a newly allocated buffer.
/// The dest buffer is NUL terminated.
/// Returns NULL in case of error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    // *src check (i.e., is the first byte zero?)
    if unsafe { *src } == 0 {
        return std::ptr::null_mut();
    }

    // Compute strlen(src) + 1
    let src_len = unsafe { libc::strlen(src) };
    let mut l: usize = src_len + 1;

    // The size of the dest will always be less than the source
    // calloc(sizeof(char), l + 13)
    let dest = unsafe { libc::calloc(1, l + 13) } as *mut c_char;
    if dest.is_null() {
        return std::ptr::null_mut();
    }

    let mut p = dest as *mut u8;

    let buf = unsafe { libc::malloc(l) } as *mut u8;
    if buf.is_null() {
        unsafe { libc::free(dest as *mut libc::c_void) };
        return std::ptr::null_mut();
    }

    // Ignore non base64 chars as per the POSIX standard
    let mut k: usize = 0;
    l = 0;
    loop {
        let ch = unsafe { *src.add(k) };
        if ch == 0 {
            break;
        }
        if is_base64(ch) != 0 {
            unsafe { *buf.add(l) = ch as u8 };
            l += 1;
        }
        k += 1;
    }

    // Now l is the count of valid base64 chars
    let mut k: usize = 0;
    while k < l {
        let mut c2: c_char = b'A' as c_char;
        let mut c3: c_char = b'A' as c_char;
        let mut c4: c_char = b'A' as c_char;

        let c1: c_char = unsafe { *buf.add(k) as c_char };

        if k + 1 < l {
            c2 = unsafe { *buf.add(k + 1) as c_char };
        }

        if k + 2 < l {
            c3 = unsafe { *buf.add(k + 2) as c_char };
        }

        if k + 3 < l {
            c4 = unsafe { *buf.add(k + 3) as c_char };
        }

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        unsafe {
            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);
        }

        if c3 != b'=' as c_char {
            unsafe {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }
        }

        if c4 != b'=' as c_char {
            unsafe {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }
        }

        k += 4;
    }

    unsafe { libc::free(buf as *mut libc::c_void) };

    dest
}
