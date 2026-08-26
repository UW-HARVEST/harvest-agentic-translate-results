// Translation of the C library in c_src/.
//
// The C code defines a single function `helxo(char letter)` that:
//   - Builds a string -> char hashmap with stb_ds (insertion-ordered)
//   - Inserts the keys "bob", "sally", "fred", "jen", "doug" with values
//     'h', 'e', 'l', 'x', 'o'
//   - Re-inserts "jen" via a stack-allocated copy of the name with `letter`
//     as the value (this updates the value at the existing index; insertion
//     order is preserved and the original "jen" key pointer is retained).
//   - Iterates the entries in insertion order and prints each one with
//     printf("%s %c\n", hash[z], hash[z].value).
//
// On x86_64 SysV the first vararg is the struct itself, but the `%s`
// conversion happens to consume the first 8 bytes of the struct (the key
// pointer) from the corresponding register and `%c` consumes the second
// 8-byte register, whose low byte is the `value` char. So the effective
// printout is "<key> <value>\n".
//
// There is no `main` in the original C, but the project is built as an
// executable. A small shim reads a single byte from stdin (mirroring the
// behavior of `scanf("%c", &c)`) and uses that byte as the `letter`.

use std::io::Read;

fn helxo(letter: u8) {
    // Insertion-ordered (key, value) entries that mirror the order
    // produced by stb_ds's shput.
    let mut entries: Vec<(&'static str, u8)> = Vec::new();

    // shput inserts a new key at the next index, or updates the value of
    // an existing entry without changing the key or the position.
    fn shput(entries: &mut Vec<(&'static str, u8)>, key: &'static str, value: u8) {
        if let Some(slot) = entries.iter_mut().find(|(k, _)| *k == key) {
            slot.1 = value;
        } else {
            entries.push((key, value));
        }
    }

    shput(&mut entries, "bob", b'h');
    shput(&mut entries, "sally", b'e');
    shput(&mut entries, "fred", b'l');
    shput(&mut entries, "jen", b'x');
    shput(&mut entries, "doug", b'o');

    // The C code does `shput(hash, name, letter)` where `name` is a stack
    // copy of "jen". stb_ds finds the existing "jen" entry and only
    // updates the value; it does not change the key pointer or the
    // entry's position.
    shput(&mut entries, "jen", letter);

    // Mimic the iteration: `for (int z=0; z < shlen(hash); ++z)
    //    printf("%s %c\n", hash[z], hash[z].value);`
    for (key, value) in &entries {
        // Use raw stdout writes so we always emit the exact bytes,
        // regardless of what `letter` (or any value byte) happens to be.
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        out.write_all(key.as_bytes()).unwrap();
        out.write_all(b" ").unwrap();
        out.write_all(std::slice::from_ref(value)).unwrap();
        out.write_all(b"\n").unwrap();
    }
}

fn main() {
    // Read a single byte from stdin to use as the `letter` argument.
    // This mirrors `scanf("%c", &letter)` in C: it reads the next byte
    // available on stdin without any whitespace skipping.
    let mut buf = [0u8; 1];
    let n = std::io::stdin().read(&mut buf).unwrap_or(0);
    let letter = if n == 0 { 0 } else { buf[0] };

    helxo(letter);
}
