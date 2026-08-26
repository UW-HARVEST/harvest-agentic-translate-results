// Translation of c_src/src/lib.c to Rust.
//
// The only public function in the C library is `str_put(int num)`.
//
// Tracing the C function:
//
//     void str_put(int num)
//     {
//       struct { char *key; int value; } *strmap = NULL, s;
//       stbds_string_arena sa = { 0 };
//       int i,j;
//
//       for (i=0; i < num; ++i)
//         stralloc(&sa, strkey(i));   // no output
//       strreset(&sa);                // no output
//
//       {
//         s.key = "a", s.value = num;
//         shputs(strmap, s);          // inserts a single entry, no output
//         // assertions, no output
//
//         for (int z=0; z < shlen(strmap); ++z)
//             printf("%s %d\n", strmap[z], strmap[z].value);
//             // shlen(strmap) is 1, so one iteration with z = 0.
//             // strmap[0] is passed by value: a 16-byte struct {char*,int}.
//             // On x86_64 SysV ABI, the struct occupies the next two
//             // integer arg registers: the `key` pointer goes in rsi
//             // and `value` (with padding) in rdx. The third arg
//             // `strmap[z].value` goes in rcx but is never read because
//             // the format string only consumes %s and %d.
//             // Effective output: "a <num>\n"
//
//         shfree(strmap);
//       }
//     }
//
// The only observable side effect is exactly one printf call producing
// "a <num>\n" on stdout. To preserve byte-identical output (including
// stdout buffering semantics), we call libc::printf directly with the
// same format string.

use std::ffi::c_int;

extern "C" {
    fn printf(fmt: *const u8, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    // Reproduce the single observable printf("%s %d\n", "a", num).
    // The string literals are NUL-terminated for printf.
    let fmt = b"%s %d\n\0".as_ptr();
    let key = b"a\0".as_ptr();
    unsafe {
        printf(fmt, key, num);
    }
}
