// Translation of c_src/src/lib.c to Rust.
//
// The public C API exposed by lib.h is only `void str_put(int num);`.
//
// `str_put` performs a sequence of stb_ds operations (string arena
// allocation, hash map insertion) that have no observable output, except
// for one printf at the end:
//
//     for (int z=0; z < shlen(strmap); ++z)
//         printf("%s %d\n", strmap[z], strmap[z].value);
//
// Where `strmap[z]` is a `struct { char *key; int value; }` passed by
// value to a variadic function. On x86_64 SysV the struct is split into
// two 8-byte eight-bytes; the first contains the `key` pointer (== "a"),
// the second contains `value` in its low 4 bytes. printf consumes the
// first eightbyte for %s and the second for %d, so the output is:
//
//     a {num}\n
//
// Since `shlen(strmap)` is 1 after the single shputs of key "a", the
// loop runs exactly once.
//
// Replicating the byte-identical output therefore reduces to writing
// "a {num}\n" to stdout.

use std::ffi::c_int;
use std::io::Write;

#[unsafe(no_mangle)]
pub extern "C" fn str_put(num: c_int) {
    // Match the C output exactly: "a {num}\n" written to stdout.
    let s = format!("a {}\n", num);
    // Write to stdout (file descriptor 1 in C). Use std::io::stdout to
    // share buffering rules with Rust's runtime; flush so that the bytes
    // appear before the function returns, mirroring printf to a
    // line-buffered stdout.
    let stdout = std::io::stdout();
    let mut handle = stdout.lock();
    let _ = handle.write_all(s.as_bytes());
    let _ = handle.flush();
}
