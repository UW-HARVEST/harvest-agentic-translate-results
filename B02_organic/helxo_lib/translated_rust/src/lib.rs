use std::ffi::c_char;

// Minimal stb_ds-compatible string hash map that preserves insertion order
// and matches the C implementation's observable behavior exactly.

struct ShEntry {
    key: *const c_char,
    value: c_char,
}

struct StringHashMap {
    entries: Vec<ShEntry>,
}

impl StringHashMap {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn shput(&mut self, key: *const c_char, value: c_char) {
        // Search for existing key (linear scan like stb_ds iteration order)
        for entry in self.entries.iter_mut() {
            if unsafe { libc::strcmp(entry.key, key) } == 0 {
                entry.value = value;
                return;
            }
        }
        // Not found — append (stb_ds appends new entries at the end)
        self.entries.push(ShEntry { key, value });
    }

    fn shlen(&self) -> usize {
        self.entries.len()
    }
}

impl Drop for StringHashMap {
    fn drop(&mut self) {
        self.entries.clear();
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash = StringHashMap::new();

    let name: [c_char; 4] = [b'j' as c_char, b'e' as c_char, b'n' as c_char, 0];

    hash.shput(b"bob\0".as_ptr() as *const c_char, b'h' as c_char);
    hash.shput(b"sally\0".as_ptr() as *const c_char, b'e' as c_char);
    hash.shput(b"fred\0".as_ptr() as *const c_char, b'l' as c_char);
    hash.shput(b"jen\0".as_ptr() as *const c_char, b'x' as c_char);
    hash.shput(b"doug\0".as_ptr() as *const c_char, b'o' as c_char);

    hash.shput(name.as_ptr(), letter);

    for z in 0..hash.shlen() {
        unsafe {
            libc::printf(
                b"%s %c\n\0".as_ptr() as *const c_char,
                hash.entries[z].key,
                hash.entries[z].value as libc::c_int,
            );
        }
    }
}
