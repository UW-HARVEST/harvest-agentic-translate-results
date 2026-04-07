use std::ffi::c_char;
use std::os::raw::c_void;

extern "C" {
    fn malloc(size: usize) -> *mut c_void;
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

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

fn is_base64(c: c_char) -> bool {
    let c = c as u8;
    (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() || unsafe { *src } == 0 {
        return std::ptr::null_mut();
    }

    unsafe {
        let l_init = strlen(src) as usize + 1;

        let dest = calloc(1, l_init + 13) as *mut u8;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        let buf = malloc(l_init) as *mut c_char;
        if buf.is_null() {
            free(dest as *mut c_void);
            return std::ptr::null_mut();
        }

        let mut l: usize = 0;
        let mut k: usize = 0;
        while *src.add(k) != 0 {
            if is_base64(*src.add(k)) {
                *buf.add(l) = *src.add(k);
                l += 1;
            }
            k += 1;
        }

        let mut p = dest;
        k = 0;
        while k < l {
            let c1: c_char;
            let mut c2: c_char = b'A' as c_char;
            let mut c3: c_char = b'A' as c_char;
            let mut c4: c_char = b'A' as c_char;

            c1 = *buf.add(k);
            if k + 1 < l {
                c2 = *buf.add(k + 1);
            }
            if k + 2 < l {
                c3 = *buf.add(k + 2);
            }
            if k + 3 < l {
                c4 = *buf.add(k + 3);
            }

            let b1 = decode(c1);
            let b2 = decode(c2);
            let b3 = decode(c3);
            let b4 = decode(c4);

            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);

            if c3 as u8 != b'=' {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }

            if c4 as u8 != b'=' {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }

            k += 4;
        }

        free(buf as *mut c_void);

        dest as *mut c_char
    }
}
