use std::ffi::c_int;

fn print_hex(p: *const u8, len: c_int) {
    let fmt_byte = b"%02x\0".as_ptr() as *const i8;
    let fmt_nl = b"\n\0".as_ptr() as *const i8;
    for i in 0..len {
        unsafe {
            libc::printf(fmt_byte, *p.offset(i as isize) as c_int);
        }
    }
    unsafe {
        libc::printf(fmt_nl);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let p = &x as *const f32 as *const u8;
    print_hex(p, std::mem::size_of::<f32>() as c_int);
}
