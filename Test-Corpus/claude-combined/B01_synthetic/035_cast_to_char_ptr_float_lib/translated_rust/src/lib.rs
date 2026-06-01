use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

fn print_hex(p: *const u8, len: c_int) {
    let mut i: c_int = 0;
    while i < len {
        unsafe {
            printf(b"%02x\0".as_ptr(), *p.offset(i as isize) as c_int);
        }
        i += 1;
    }
    unsafe {
        printf(b"\n\0".as_ptr());
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let p = &x as *const f32 as *const u8;
    print_hex(p, std::mem::size_of::<f32>() as c_int);
}
