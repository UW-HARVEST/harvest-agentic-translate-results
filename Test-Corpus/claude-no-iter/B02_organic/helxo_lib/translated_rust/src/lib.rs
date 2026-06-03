// Translation of c_src/src/lib.c to Rust.
//
// The original C code is the public-domain stb_ds.h hashmap embedded
// in a single source file plus a small public function `helxo` that
// exercises a string-keyed hashmap and prints its contents.
//
// The C `helxo` function:
//   1. Creates a string-keyed hashmap with entries:
//      "bob"   -> 'h'
//      "sally" -> 'e'
//      "fred"  -> 'l'
//      "jen"   -> 'x'
//      "doug"  -> 'o'
//   2. Updates "jen" to the caller-supplied `letter` (uses the same key,
//      so the insertion-ordered position is preserved).
//   3. Iterates the hashmap in insertion order and prints each entry
//      using `printf("%s %c\n", hash[z], hash[z].value)`.
//
// Note on the printf call: the C code passes the *struct* `hash[z]`
// (which has layout `{ char *key; char value; }`) directly as a variadic
// argument. On the x86-64 SysV ABI a 16-byte struct passed by value is
// split across two 8-byte argument slots, so `%s` consumes the `key`
// pointer and `%c` consumes the byte from the second slot (which holds
// `value`). The third argument `hash[z].value` is therefore read but
// has no corresponding format specifier and is effectively unused.
//
// The user-observable behaviour is therefore: for each entry in
// insertion order, print "<key> <value>\n". stb_ds preserves insertion
// order and updating an existing key keeps its slot, so the resulting
// byte stream is fully determined by `letter`.
//
// To preserve byte-identical output (including any libc stdout
// buffering behaviour) we call libc's `printf` directly rather than
// going through Rust's stdout machinery.

use std::ffi::{c_char, c_int};

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> c_int;
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn helxo(letter: c_char) {
    // Build the same set of entries the C code inserts, in the same
    // order. stb_ds preserves insertion order; updating "jen" with
    // the new value `letter` does not move it, so the order matches
    // what the C code iterates.
    let entries: [(&[u8], c_char); 5] = [
        (b"bob\0",   b'h' as c_char),
        (b"sally\0", b'e' as c_char),
        (b"fred\0",  b'l' as c_char),
        (b"jen\0",   letter),
        (b"doug\0",  b'o' as c_char),
    ];

    let fmt = b"%s %c\n\0".as_ptr() as *const c_char;
    for (key, value) in entries.iter() {
        // SAFETY: format and key strings are NUL-terminated; printf is
        // a standard libc symbol; the value is promoted to c_int per
        // the C variadic calling convention used with `%c`.
        unsafe {
            printf(fmt, key.as_ptr() as *const c_char, *value as c_int);
        }
    }
}
