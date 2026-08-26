use libc::{memset, printf, strncpy};
use std::ffi::{c_char, c_int, c_void};

#[unsafe(no_mangle)]
pub unsafe extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        printf(c"%s\n".as_ptr(), line);
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn driver(data: c_int) {
    let mut source = [0 as c_char; 100];
    let mut dest = [0 as c_char; 100];

    memset(
        source.as_mut_ptr().cast::<c_void>(),
        i32::from(b'A'),
        100usize - 1,
    );
    source[100 - 1] = 0;

    if data < 100 {
        strncpy(dest.as_mut_ptr(), source.as_ptr(), data as usize);
        *dest.as_mut_ptr().offset(data as isize) = 0;
    }

    printLine(dest.as_ptr());
}
