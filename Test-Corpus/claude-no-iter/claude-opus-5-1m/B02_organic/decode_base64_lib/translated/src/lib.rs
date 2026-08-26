use std::ffi::c_char;
use std::ffi::c_int;

/// Decode a base64 character
fn decode(c: c_char) -> u8 {
    let c = c as u8;
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

/// Returns TRUE if 'c' is a valid base64 character, otherwise FALSE
fn is_base64(c: c_char) -> c_int {
    let c = c as u8;
    if (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
    {
        return 1;
    }
    0
}

/// Compute the length of a NUL-terminated C string
unsafe fn c_strlen(s: *const c_char) -> usize {
    let mut n = 0usize;
    while *s.add(n) != 0 {
        n += 1;
    }
    n
}

/// Decode the base64 encoded string 'src' into a freshly allocated buffer.
/// The returned buffer is NUL terminated.
/// Returns NULL in case of error.
#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if !src.is_null() && *src != 0 {
        let mut l: c_int = (c_strlen(src) + 1) as c_int;

        // The size of the dest will always be less than the source
        let dest = libc::calloc(
            std::mem::size_of::<c_char>(),
            (l + 13) as usize,
        ) as *mut c_char;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        let mut p = dest as *mut u8;

        let buf = libc::malloc(l as usize) as *mut u8;
        if buf.is_null() {
            libc::free(dest as *mut libc::c_void);
            return std::ptr::null_mut();
        }

        // Ignore non base64 chars as per the POSIX standard
        let mut k: c_int = 0;
        l = 0;
        loop {
            let ch = *src.offset(k as isize);
            if ch == 0 {
                break;
            }
            if is_base64(ch) != 0 {
                *buf.offset(l as isize) = ch as u8;
                l += 1;
            }
            k += 1;
        }

        let mut k: c_int = 0;
        while k < l {
            #[allow(unused_assignments)]
            let mut c1: c_char = b'A' as c_char;
            let mut c2: c_char = b'A' as c_char;
            let mut c3: c_char = b'A' as c_char;
            let mut c4: c_char = b'A' as c_char;

            c1 = *buf.offset(k as isize) as c_char;

            if k + 1 < l {
                c2 = *buf.offset((k + 1) as isize) as c_char;
            }

            if k + 2 < l {
                c3 = *buf.offset((k + 2) as isize) as c_char;
            }

            if k + 3 < l {
                c4 = *buf.offset((k + 3) as isize) as c_char;
            }

            let b1 = decode(c1);
            let b2 = decode(c2);
            let b3 = decode(c3);
            let b4 = decode(c4);

            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);

            if c3 != b'=' as c_char {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }

            if c4 != b'=' as c_char {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }

            k += 4;
        }

        libc::free(buf as *mut libc::c_void);

        return dest;
    }
    std::ptr::null_mut()
}
