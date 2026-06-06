use std::ffi::c_double;

extern "C" {
    fn printf(format: *const u8, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    // raw_double_t union: reinterpret f64 bits as u64
    let x: u64 = f.to_bits();
    // Format string matches the C source exactly: "%llx %a %.4f\n\0"
    let fmt = b"%llx %a %.4f\n\0";
    unsafe {
        printf(fmt.as_ptr(), x, f, f);
    }
}
