use std::ffi::{c_char, c_void};

unsafe extern "C" {
    fn calloc(nmemb: usize, size: usize) -> *mut c_void;
    fn malloc(size: usize) -> *mut c_void;
    fn free(ptr: *mut c_void);
    fn strlen(s: *const c_char) -> usize;
}

const TRUE: i32 = 1;
const FALSE: i32 = 0;

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

fn is_base64(c: c_char) -> i32 {
    let c = c as u8;

    if (c >= b'A' && c <= b'Z')
        || (c >= b'a' && c <= b'z')
        || (c >= b'0' && c <= b'9')
        || c == b'+'
        || c == b'/'
        || c == b'='
    {
        return TRUE;
    }
    FALSE
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn decode_base64(src: *const c_char) -> *mut c_char {
    if !src.is_null() && unsafe { *src } != 0 {
        let dest: *mut c_char;
        let mut p: *mut u8;
        let mut k: i32;
        let mut l: i32 = unsafe { strlen(src) as i32 } + 1;
        let buf: *mut u8;

        dest = unsafe { calloc(std::mem::size_of::<c_char>(), (l + 13) as usize) } as *mut c_char;
        if dest.is_null() {
            return std::ptr::null_mut();
        }

        p = dest as *mut u8;

        buf = unsafe { malloc(l as usize) } as *mut u8;
        if buf.is_null() {
            unsafe { free(dest as *mut c_void) };
            return std::ptr::null_mut();
        }

        k = 0;
        l = 0;
        while unsafe { *src.offset(k as isize) } != 0 {
            let c = unsafe { *src.offset(k as isize) };
            if is_base64(c) != 0 {
                unsafe {
                    *buf.offset(l as isize) = c as u8;
                }
                l += 1;
            }
            k += 1;
        }

        k = 0;
        while k < l {
            let c1: c_char;
            let mut c2: c_char = b'A' as c_char;
            let mut c3: c_char = b'A' as c_char;
            let mut c4: c_char = b'A' as c_char;
            let b1: u8;
            let b2: u8;
            let b3: u8;
            let b4: u8;

            c1 = unsafe { *buf.offset(k as isize) as c_char };

            if k + 1 < l {
                c2 = unsafe { *buf.offset((k + 1) as isize) as c_char };
            }

            if k + 2 < l {
                c3 = unsafe { *buf.offset((k + 2) as isize) as c_char };
            }

            if k + 3 < l {
                c4 = unsafe { *buf.offset((k + 3) as isize) as c_char };
            }

            b1 = decode(c1);
            b2 = decode(c2);
            b3 = decode(c3);
            b4 = decode(c4);

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

        unsafe { free(buf as *mut c_void) };

        return dest;
    }
    std::ptr::null_mut()
}
