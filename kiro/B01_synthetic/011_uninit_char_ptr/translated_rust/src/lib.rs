use std::mem::MaybeUninit;

#[no_mangle]
pub extern "C" fn printLine(line: *const u8) {
    if !line.is_null() {
        unsafe {
            let mut len = 0usize;
            while *line.add(len) != 0 {
                len += 1;
            }
            let slice = std::slice::from_raw_parts(line, len);
            let s = std::str::from_utf8_unchecked(slice);
            println!("{}", s);
        }
    }
}

#[no_mangle]
pub extern "C" fn bad() {
    let data: *const u8 = unsafe { MaybeUninit::uninit().assume_init() };
    printLine(data);
}

#[no_mangle]
pub extern "C" fn good() {
    let data: *const u8 = b"string\0".as_ptr();
    printLine(data);
}

#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main() -> i32 {
    let mut x: i32 = 0;
    unsafe {
        libc::scanf(b"%d\0".as_ptr() as *const i8, &mut x as *mut i32);
    }
    if x != 0 {
        good();
    } else {
        bad();
    }
    0
}
