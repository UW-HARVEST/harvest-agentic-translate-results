// Translation of c_src/src/lib.c — implements the public C function `helxo`.
//
// The C source pulls in the stb_ds.h string-keyed hashmap and inserts five
// entries, then overwrites the value for "jen" with the function argument,
// then iterates the storage in insertion order printing "<key> <value>\n".
//
// stb_ds preserves insertion order (entries live in a packed array; updating
// an existing key only mutates its value slot), so the output is fully
// deterministic and equivalent to the sequence below — no need to reproduce
// the entire hash table to match the C output byte-for-byte.

#![allow(non_camel_case_types)]

use std::ffi::c_char;
use std::io::{self, Write};

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    // Build the in-order list of (key, value) pairs as the C code would have
    // observed them after the sequence of shput() calls.  The fifth shput
    // uses the local `name` buffer ("jen") which already exists in the table,
    // so it overwrites the value for the existing "jen" entry rather than
    // appending a new one.
    let entries: [(&[u8], c_char); 5] = [
        (b"bob",   b'h' as c_char),
        (b"sally", b'e' as c_char),
        (b"fred",  b'l' as c_char),
        (b"jen",   letter),
        (b"doug",  b'o' as c_char),
    ];

    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (key, value) in entries.iter() {
        // Reproduce printf("%s %c\n", key, value) byte-for-byte.
        let _ = out.write_all(key);
        let _ = out.write_all(b" ");
        // %c writes the single byte of the int promoted argument.
        let byte = (*value as u32 & 0xff) as u8;
        let _ = out.write_all(&[byte]);
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}
