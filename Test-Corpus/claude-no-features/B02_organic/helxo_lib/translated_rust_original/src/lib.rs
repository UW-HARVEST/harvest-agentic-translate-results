// Translation of c_src/src/lib.c to Rust.
//
// The C library is essentially the stb_ds.h hashtable implementation plus a
// single public function `helxo(char)` declared in c_src/include/lib.h.
//
// The C `helxo` function:
//   1. Builds a string-keyed hashtable (stb_ds), inserts:
//        "bob"   -> 'h'
//        "sally" -> 'e'
//        "fred"  -> 'l'
//        "jen"   -> 'x'
//        "doug"  -> 'o'
//   2. Then `shput(hash, name, letter)` where `name == "jen"`. Because the
//      key "jen" already exists, this UPDATES the existing entry's value to
//      `letter` (and does NOT change its insertion-order position).
//   3. Iterates in insertion order and prints `"%s %c\n"` per entry.
//
// The printf call `printf("%s %c\n", hash[z], hash[z].value)` passes the
// struct `{char *key; char value;}` by value as a variadic argument. On
// x86-64 SysV the 16-byte struct is passed in two integer registers (key in
// the first, value in the second). printf consumes one register per `%s`/`%c`,
// so it effectively prints `key value\n`.
//
// Therefore for byte-identical output we just emit `"<key> <value>\n"`
// per insertion-order entry.

use std::ffi::c_char;
use std::io::{self, Write};

// Append or update a (key, value) pair, preserving insertion order on update
// — matching stb_ds shput semantics where a duplicate key updates the
// existing slot in-place.
fn shput_ordered(entries: &mut Vec<(&'static str, c_char)>, key: &'static str, value: c_char) {
    if let Some(slot) = entries.iter_mut().find(|(k, _)| *k == key) {
        slot.1 = value;
    } else {
        entries.push((key, value));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash: Vec<(&'static str, c_char)> = Vec::new();

    shput_ordered(&mut hash, "bob",   b'h' as c_char);
    shput_ordered(&mut hash, "sally", b'e' as c_char);
    shput_ordered(&mut hash, "fred",  b'l' as c_char);
    shput_ordered(&mut hash, "jen",   b'x' as c_char);
    shput_ordered(&mut hash, "doug",  b'o' as c_char);

    // `name` is "jen" in the C source, so this updates the "jen" entry.
    shput_ordered(&mut hash, "jen", letter);

    let stdout = io::stdout();
    let mut out = stdout.lock();
    for (key, value) in &hash {
        // Emit `<key> <value>\n` — the same bytes the C printf produces.
        let _ = out.write_all(key.as_bytes());
        let _ = out.write_all(b" ");
        // `value` is a c_char (i8 or u8 depending on platform). Print the
        // raw byte exactly as C's `%c` would.
        let byte = (*value as u8) as u8;
        let _ = out.write_all(&[byte]);
        let _ = out.write_all(b"\n");
    }
    let _ = out.flush();
}
