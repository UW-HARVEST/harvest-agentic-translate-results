// Rust translation of c_src/src/driver.c (MIT Lincoln Laboratory, 2025).
//
// The C library consists of a single public symbol, `driver`, declared in
// include/driver.h as:
//
//     void driver(int x, int y);
//
// Output is produced with C `printf`, so this translation calls the very same
// libc `printf` in order to keep stdout buffering/interleaving behaviour (and
// therefore the emitted bytes) identical to the original library.

use std::ffi::{c_char, c_int};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// `printf("<s>")` where `s` is a NUL-terminated literal with no format
/// specifiers (matching the original calls, which gcc lowers to `puts`).
#[inline]
fn print_lit(s: &[u8]) {
    debug_assert_eq!(*s.last().unwrap(), 0);
    unsafe {
        printf(s.as_ptr() as *const c_char);
    }
}

/// Faithful translation of:
///
/// ```c
/// void driver(int x, int y) {
///     while (x > 0 || y > 0) {
///         printf("loop\n");
///
///         if (x == 1 && y == 4) {
///             goto label2;
///         }
///
/// label1:
///         if (x > 0) {
///             printf("x\n");
///             x--;
///         }
///
/// label2:
///         if (y == 0) {
///             continue;
///         }
///         printf("y\n");
///         y--;
///         if (x < 3) {
///             goto label1;
///         }
///     }
/// }
/// ```
///
/// `label1` is a backwards jump target from inside the same iteration, so the
/// loop body is itself a loop; `label2` skips the `label1` block only on the
/// first pass through an iteration when `x == 1 && y == 4`.
#[unsafe(no_mangle)]
pub extern "C" fn driver(x: c_int, y: c_int) {
    let mut x = x;
    let mut y = y;

    // `while (x > 0 || y > 0)`
    'while_loop: while x > 0 || y > 0 {
        print_lit(b"loop\n\0");

        // `if (x == 1 && y == 4) goto label2;`
        let mut skip_label1 = x == 1 && y == 4;

        // Inner loop implements the backwards `goto label1`.
        loop {
            if !skip_label1 {
                // label1:
                if x > 0 {
                    print_lit(b"x\n\0");
                    x = x.wrapping_sub(1);
                }
            }
            skip_label1 = false;

            // label2:
            if y == 0 {
                continue 'while_loop;
            }
            print_lit(b"y\n\0");
            y = y.wrapping_sub(1);
            if x < 3 {
                // goto label1;
                continue;
            }

            // End of the while body: fall through to the loop condition.
            break;
        }
    }
}
