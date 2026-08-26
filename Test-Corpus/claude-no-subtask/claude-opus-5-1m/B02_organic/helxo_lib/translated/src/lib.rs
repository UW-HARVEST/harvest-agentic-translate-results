// Translation of c_src/src/lib.c -- preserves the exact observable output of
// `helxo(letter)` from the original C implementation.
//
// The C function builds a string-keyed stb_ds hashmap, inserts five fixed
// entries, then overwrites the "jen" entry with the parameter `letter`. It
// then iterates the underlying array and prints each entry. stb_ds stores its
// hashmap in an array (insertion-ordered) and reuses the existing slot when an
// existing key is re-put, so the output order is fixed:
//
//   bob <letter>     (when letter overrides; otherwise bob h)
//   ... etc.
//
// In particular, the original printf is `printf("%s %c\n", hash[z], hash[z].value)`.
// On the System V x86_64 ABI the 16-byte struct is passed in two integer
// registers; %s consumes the first (the key pointer) and %c consumes the
// .value passed as the third variadic arg. The net visible output for any
// given `letter` is therefore:
//
//   bob h
//   sally e
//   fred l
//   jen <letter>
//   doug o
//
// We reproduce that byte-for-byte using libc::printf so that stdout buffering
// matches the C code's behavior.

use std::ffi::c_char;

unsafe extern "C" {
    fn printf(format: *const c_char, ...) -> i32;
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    // Format string: "%s %c\n\0"
    let fmt: &[u8] = b"%s %c\n\0";

    // Static keys, NUL-terminated, in stb_ds insertion order.
    let bob: &[u8] = b"bob\0";
    let sally: &[u8] = b"sally\0";
    let fred: &[u8] = b"fred\0";
    let jen: &[u8] = b"jen\0";
    let doug: &[u8] = b"doug\0";

    // The original code initializes "jen" to 'x', then overrides it with
    // `letter` via shput on the same key, which updates the value in place.
    let h: c_char = b'h' as c_char;
    let e: c_char = b'e' as c_char;
    let l: c_char = b'l' as c_char;
    let x_overridden: c_char = letter;
    let o: c_char = b'o' as c_char;

    unsafe {
        printf(fmt.as_ptr() as *const c_char, bob.as_ptr() as *const c_char, h as i32);
        printf(fmt.as_ptr() as *const c_char, sally.as_ptr() as *const c_char, e as i32);
        printf(fmt.as_ptr() as *const c_char, fred.as_ptr() as *const c_char, l as i32);
        printf(fmt.as_ptr() as *const c_char, jen.as_ptr() as *const c_char, x_overridden as i32);
        printf(fmt.as_ptr() as *const c_char, doug.as_ptr() as *const c_char, o as i32);
    }
}
