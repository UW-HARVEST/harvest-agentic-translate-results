use std::ffi::{c_char, c_int};
use std::fmt::Write as _;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

static PRINT_FORMAT: &[u8] = b"%s %d\n\0";
static mut BUFFER: [c_char; 256] = [0; 256];

fn write_strkey(buf: &mut [c_char; 256], n: c_int) -> *mut c_char {
    let mut s = String::new();
    let _ = write!(&mut s, "test_{}", n);

    let bytes = s.as_bytes();
    let len = bytes.len().min(buf.len().saturating_sub(1));
    for (dst, src) in buf.iter_mut().zip(bytes.iter().copied()).take(len) {
        *dst = src as c_char;
    }
    buf[len] = 0;
    buf.as_mut_ptr()
}

#[unsafe(no_mangle)]
pub extern "C" fn strkey(n: c_int) -> *mut c_char {
    unsafe { write_strkey(&mut *std::ptr::addr_of_mut!(BUFFER), n) }
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    for _ in 0..2 {
        let mut i = 0;
        while i < num {
            let mut key = [0 as c_char; 256];
            let key_ptr = write_strkey(&mut key, i);
            unsafe {
                printf(
                    PRINT_FORMAT.as_ptr().cast::<c_char>(),
                    key_ptr.cast::<c_char>(),
                    i.wrapping_mul(3),
                );
            }
            i = i.wrapping_add(2);
        }
    }
}
