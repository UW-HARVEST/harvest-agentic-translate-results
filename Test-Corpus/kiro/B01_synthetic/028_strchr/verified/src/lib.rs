use std::os::raw::{c_char, c_int};

#[no_mangle]
pub extern "C" fn foo(input: *const c_char, c: c_char) -> c_int {
    if input.is_null() {
        return 0;
    }
    // Replicate C strchr semantics: scan until null terminator
    let mut count: c_int = 0;
    let mut p = input;
    unsafe {
        loop {
            let ch = *p;
            if ch == 0 {
                break;
            }
            if ch == c {
                count += 1;
            }
            p = p.add(1);
        }
    }
    count
}

#[no_mangle]
pub extern "C" fn driver(input: *const c_char) {
    let a = foo(input, b'A' as c_char);
    let x = foo(input, b'x' as c_char);
    print!("A: {}\nx: {}\n", a, x);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> c_int {
    0
}
