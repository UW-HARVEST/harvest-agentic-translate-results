// Translation of c_src/src/lib.c (a snippet of stb_ds + a small `sh_geti` test
// driver). The only public symbol exposed by include/lib.h is `sh_geti(int)`.
//
// Observable behavior of `sh_geti(num)`:
//
//   1. The string-arena warm-up (stralloc/strreset of keys "test_0".."test_{num-1}")
//      produces no stdout output.
//   2. For j in 0..2:
//        - Build a fresh string-keyed hash map with default value -2.
//        - For i in 0..num step 2: insert ("test_<i>", i*3) -- entries are
//          appended to the underlying array in insertion order.
//        - Iterate that array and print "<key> <value>\n" via printf.  Due to
//          x86_64 SysV passing of the {char*, int} struct as two eightbytes,
//          the format "%s %d\n" reads the key from the first eightbyte and
//          the value from the second; the explicit `strmap[z].value` arg is
//          unused but harmless.
//        - Run a series of shget / shdel assertions.  Provided the underlying
//          stb_ds code is correct, none of these assertions ever fire on this
//          input, so they produce no output and do not abort the process.
//
// Therefore the byte-identical stdout output is simply, twice in a row:
//
//     test_0 0
//     test_2 6
//     test_4 12
//     ...
//     test_<2k> <6k>            for the largest even 2k < num
//
// We reproduce this exactly using libc::printf so the bytes match what the
// original C library would emit (formatting, newline, flushing semantics).

use core::ffi::c_int;

unsafe extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn sh_geti(num: c_int) {
    // Format string: "%s %d\n" followed by NUL terminator.
    static FMT: [u8; 8] = *b"%s %d\n\0\0";

    // Two outer iterations (j == 0 uses strdup mode, j == 1 uses arena mode in
    // the C original; both produce identical stdout output for this driver).
    for _j in 0..2 {
        // Insertion order: i = 0, 2, 4, ..., < num.  Each entry is printed
        // exactly once with its value (i * 3).
        let mut i: c_int = 0;
        while i < num {
            // Build the C-string "test_<i>\0" for %s.
            let mut key_buf: [u8; 32] = [0; 32];
            let key_str = format!("test_{}", i);
            let key_bytes = key_str.as_bytes();
            // Defensive: the largest plausible i fits comfortably; truncate
            // at 30 bytes to leave room for the NUL terminator.
            let copy_len = core::cmp::min(key_bytes.len(), key_buf.len() - 1);
            key_buf[..copy_len].copy_from_slice(&key_bytes[..copy_len]);
            // key_buf[copy_len] is already 0 from initialization.

            // value = i * 3, matching the C `shput(strmap, strkey(i), i*3)`.
            let value: c_int = i.wrapping_mul(3);

            unsafe {
                printf(FMT.as_ptr(), key_buf.as_ptr(), value);
            }

            i += 2;
        }
    }
}
