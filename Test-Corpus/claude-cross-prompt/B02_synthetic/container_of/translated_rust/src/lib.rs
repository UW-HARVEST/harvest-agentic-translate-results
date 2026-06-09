use std::ffi::{c_char, c_int, CStr};

#[repr(C)]
pub struct test {
    pub a: c_int,
    pub b: c_int,
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_container_of_a(i: *mut c_int) -> *mut test {
    let offset = std::mem::offset_of!(test, a);
    (i as *mut u8).sub(offset) as *mut test
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn find_container_of_b(i: *mut c_int) -> *mut test {
    let offset = std::mem::offset_of!(test, b);
    (i as *mut u8).sub(offset) as *mut test
}

// Replicate C's atoi behavior
fn c_atoi(s: &CStr) -> c_int {
    let bytes = s.to_bytes();
    let mut i = 0usize;
    // Skip leading whitespace (matches isspace)
    while i < bytes.len() {
        match bytes[i] {
            b' ' | b'\t' | b'\n' | b'\x0b' | b'\x0c' | b'\r' => i += 1,
            _ => break,
        }
    }
    let mut sign: i64 = 1;
    if i < bytes.len() {
        match bytes[i] {
            b'+' => i += 1,
            b'-' => {
                sign = -1;
                i += 1;
            }
            _ => {}
        }
    }
    let mut result: i64 = 0;
    while i < bytes.len() && bytes[i].is_ascii_digit() {
        result = result.wrapping_mul(10).wrapping_add((bytes[i] - b'0') as i64);
        i += 1;
    }
    (result.wrapping_mul(sign)) as c_int
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn main(_argc: c_int, argv: *mut *mut c_char) -> c_int {
    let arg1 = CStr::from_ptr(*argv.add(1));
    let arg2 = CStr::from_ptr(*argv.add(2));
    let a = c_atoi(arg1);
    let b = c_atoi(arg2);

    let mut t = test { a: 0, b: 0 };
    // memset(&t, 0, sizeof(t)) — already zeroed above
    t.a = a;
    t.b = b;

    let sum = (*find_container_of_a(&mut t.a)).a + (*find_container_of_b(&mut t.b)).b;
    println!("{}", sum);
    0
}
