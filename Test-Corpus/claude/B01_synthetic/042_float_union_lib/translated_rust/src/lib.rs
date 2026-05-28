use std::ffi::c_double;

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    let x: u64 = f.to_bits();
    // Use libc::printf to preserve byte-identical output formatting,
    // particularly for %a which can differ between Rust's formatting and C.
    let fmt = b"%llx %a %.4f\n\0";
    unsafe {
        libc::printf(
            fmt.as_ptr() as *const libc::c_char,
            x as libc::c_ulonglong,
            f,
            f,
        );
    }
}
