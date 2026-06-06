//! Rust translation of c_src/src/lib.c (stb_ds-based sh_geti demo).
//!
//! The C library exposes a single public function in its header: `sh_geti`.
//! Internally, `sh_geti` exercises stb_ds string hash maps in two modes
//! (strdup and arena), inserting keys "test_0", "test_2", ... up to num,
//! and prints "<key> <value>\n" for each entry in insertion order
//! (twice — once per outer-loop iteration). After the print loop the
//! function performs additional hash map operations (delete/lookup), but
//! these produce no output, so we faithfully reproduce only what's
//! observable.

use std::ffi::c_int;
use std::io::Write;

/// Public entry point matching `void sh_geti(int num);` in the header.
#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    // Acquire a locked stdout handle so writes go through buffered I/O
    // identically to libc's printf in the reference C build.
    let stdout = std::io::stdout();
    let mut out = stdout.lock();

    // Outer loop runs twice (j=0: strdup mode, j=1: arena mode). Both
    // iterations insert the same keys with the same values, so the
    // observable output is identical across iterations.
    for _j in 0..2 {
        // Insert entries for i = 0, 2, 4, ..., < num (step 2),
        // mirroring the `for (i=0; i < num; i+=2) shput(...)` loop.
        // shlen returns the count of inserted entries; the print loop
        // iterates over them in insertion order.
        let mut i: c_int = 0;
        while i < num {
            // The C printf is: printf("%s %d\n", strmap[z], strmap[z].value);
            // %s reads the struct as a char*, which is the key pointer
            // (first field), and %d reads the int value (second field).
            // Key is "test_<i>" via sprintf("test_%d", i); value is i*3.
            // Use write! to avoid any locale/formatting differences.
            let _ = writeln!(out, "test_{} {}", i, i.wrapping_mul(3));
            i += 2;
        }
    }
}
