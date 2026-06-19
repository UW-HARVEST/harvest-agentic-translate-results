use std::ffi::{c_char, c_int, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn strlen(s: *const c_char) -> usize;
}

fn encode(u: u8) -> c_char {
    let c = if u < 26 {
        b'A' + u
    } else if u < 52 {
        b'a' + (u - 26)
    } else if u < 62 {
        b'0' + (u - 52)
    } else if u == 62 {
        b'+'
    } else {
        b'/'
    };

    c as c_char
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn encode_base64(mut size: c_int, src: *const c_char) -> *mut c_char {
    if src.is_null() {
        return std::ptr::null_mut();
    }

    if size == 0 {
        size = unsafe { strlen(src) as c_int };
    }

    let out_size = size.wrapping_mul(4).wrapping_div(3).wrapping_add(4);
    let out = unsafe { calloc(1, out_size as usize) as *mut c_char };
    if out.is_null() {
        return std::ptr::null_mut();
    }

    let mut p = out;
    let mut i: c_int = 0;

    while i < size {
        let mut b2 = 0_u8;
        let mut b3 = 0_u8;

        let b1 = unsafe { *src.add(i as usize) as u8 };

        if i + 1 < size {
            b2 = unsafe { *src.add((i + 1) as usize) as u8 };
        }

        if i + 2 < size {
            b3 = unsafe { *src.add((i + 2) as usize) as u8 };
        }

        let b4 = b1 >> 2;
        let b5 = ((b1 & 0x3) << 4) | (b2 >> 4);
        let b6 = ((b2 & 0xf) << 2) | (b3 >> 6);
        let b7 = b3 & 0x3f;

        unsafe {
            *p = encode(b4);
            p = p.add(1);
            *p = encode(b5);
            p = p.add(1);

            if i + 1 < size {
                *p = encode(b6);
            } else {
                *p = b'=' as c_char;
            }
            p = p.add(1);

            if i + 2 < size {
                *p = encode(b7);
            } else {
                *p = b'=' as c_char;
            }
            p = p.add(1);
        }

        i += 3;
    }

    out
}
