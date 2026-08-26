use std::ffi::c_char;
use std::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

fn print_hex(p: *const u8, len: c_int) {
    for i in 0..len {
        unsafe {
            let byte = *p.offset(i as isize);
            printf(b"%02x\0".as_ptr() as *const c_char, byte as c_int);
        }
    }
    unsafe {
        printf(b"\n\0".as_ptr() as *const c_char);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let p = &x as *const f32 as *const u8;
    print_hex(p, std::mem::size_of::<f32>() as c_int);
}
