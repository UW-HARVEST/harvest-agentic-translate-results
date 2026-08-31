// Rust translation of c_src/src/driver.c
//
// Original C:
//     void driver(int x) {
//         auto int y = 2*x;   // `auto` is just the default storage class
//         y += 300;
//         printf("%d\n", y);
//     }
//
// The header (include/driver.h) declares `void driver(int x)` with no
// namespace/renaming macros, so the final linker symbol is plain `driver`.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    // Use the platform C library's printf so the emitted bytes and the stdio
    // buffering behaviour match the original library exactly.
    fn printf(format: *const c_char, ...) -> c_int;
}

/// Computes `2 * x + 300` and prints it with a trailing newline.
///
/// Signed-overflow wrapping is used to mirror what the C compiler actually
/// emits for `2*x` / `y += 300` on two's-complement targets.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int) {
    let y = x.wrapping_mul(2).wrapping_add(300);

    // "%d\n" as a NUL-terminated C string.
    const FMT: &[u8; 4] = b"%d\n\0";
    unsafe {
        printf(FMT.as_ptr() as *const c_char, y);
    }
}
