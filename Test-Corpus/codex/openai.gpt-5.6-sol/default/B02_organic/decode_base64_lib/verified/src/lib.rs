use std::ffi::{c_char, c_int, c_void};
use std::ptr;

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

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if src.is_null() || unsafe { *src } == 0 {
        return ptr::null_mut();
    }

    let mut l = unsafe { strlen(src) }.wrapping_add(1) as c_int;
    let dest = unsafe { calloc(1, l.wrapping_add(13) as usize) }.cast::<c_char>();
    if dest.is_null() {
        return ptr::null_mut();
    }

    let buf = unsafe { malloc(l as usize) }.cast::<u8>();
    if buf.is_null() {
        unsafe { free(dest.cast()) };
        return ptr::null_mut();
    }

    let mut k: c_int = 0;
    l = 0;
    while unsafe { *src.add(k as usize) } != 0 {
        let c = unsafe { *src.add(k as usize) } as u8;
        if is_base64(c) {
            unsafe { *buf.add(l as usize) = c };
            l += 1;
        }
        k += 1;
    }

    let mut p = dest.cast::<u8>();
    k = 0;
    while k < l {
        let c1 = unsafe { *buf.add(k as usize) };
        let c2 = if k + 1 < l {
            unsafe { *buf.add((k + 1) as usize) }
        } else {
            b'A'
        };
        let c3 = if k + 2 < l {
            unsafe { *buf.add((k + 2) as usize) }
        } else {
            b'A'
        };
        let c4 = if k + 3 < l {
            unsafe { *buf.add((k + 3) as usize) }
        } else {
            b'A'
        };

        let b1 = decode(c1);
        let b2 = decode(c2);
        let b3 = decode(c3);
        let b4 = decode(c4);

        unsafe {
            *p = (b1 << 2) | (b2 >> 4);
            p = p.add(1);

            if c3 != b'=' {
                *p = ((b2 & 0xf) << 4) | (b3 >> 2);
                p = p.add(1);
            }

            if c4 != b'=' {
                *p = ((b3 & 0x3) << 6) | b4;
                p = p.add(1);
            }
        }

        k += 4;
    }

    unsafe { free(buf.cast()) };
    dest
}
