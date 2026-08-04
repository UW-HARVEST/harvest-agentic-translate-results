use std::ffi::c_double;

extern "C" {
    fn printf(fmt: *const u8, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: c_double) {
    // Reinterpret f64 bits as u64 (mirrors the C union access).
    let x: u64 = f.to_bits();
    // Use libc printf directly so the output matches the C code's
    // %llx, %a, and %.4f formatting byte-for-byte.
    let fmt = b"%llx %a %.4f\n\0";
    unsafe {
        printf(fmt.as_ptr(), x, f, f);
    }
}
