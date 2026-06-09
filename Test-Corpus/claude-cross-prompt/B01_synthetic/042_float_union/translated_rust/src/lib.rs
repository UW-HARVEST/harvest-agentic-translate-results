// Translated from c_src/src/main.c
//
// Original C:
//   void driver(double f) {
//       raw_double_t u = {.f = f};
//       printf("%llx %a %.4f\n", u.x, f, f);
//   }
//
// To produce byte-identical output to the C printf for the "%llx %a %.4f\n"
// format string, we delegate to libc's printf via FFI. This guarantees that
// platform-specific formatting for %a (hex float) matches the C version
// exactly.

use std::ffi::c_char;
use std::os::raw::c_int;

#[repr(C)]
union RawDouble {
    x: u64,
    f: f64,
}

unsafe extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn driver(f: f64) {
    // Reproduce the C union initialization: u.f = f, then read u.x
    let u = RawDouble { f };
    // SAFETY: Reading the bit pattern of f as u64 is well-defined.
    let x: u64 = unsafe { u.x };

    // Use libc printf to ensure byte-identical output.
    // Format string: "%llx %a %.4f\n\0"
    let fmt = b"%llx %a %.4f\n\0".as_ptr() as *const c_char;
    unsafe {
        // %llx -> unsigned long long (u64 on most platforms; on Windows
        // long long is also 64 bits, so this is safe across targets we
        // care about). Pass x as c_ulonglong.
        printf(fmt, x as core::ffi::c_ulonglong, f, f);
    }
}
