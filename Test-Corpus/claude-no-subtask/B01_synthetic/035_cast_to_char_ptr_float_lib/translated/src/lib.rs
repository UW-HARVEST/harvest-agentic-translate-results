use std::ffi::c_int;

fn print_hex(p: *const u8, len: c_int) {
    unsafe {
        for i in 0..len {
            let byte = *p.offset(i as isize);
            // Use libc printf to match C output exactly
            libc::printf(b"%02x\0".as_ptr() as *const i8, byte as c_int);
        }
        libc::printf(b"\n\0".as_ptr() as *const i8);
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(x: f32) {
    let ptr = (&x as *const f32) as *const u8;
    let len = std::mem::size_of::<f32>() as c_int;
    print_hex(ptr, len);
}
