use std::ffi::{c_char, c_int, CStr};

extern "C" {
    fn printf(fmt: *const c_char, ...) -> c_int;
}

/// A simple ordered string→char map that mimics stb_ds shput/shlen iteration behavior:
/// - New keys are appended in insertion order.
/// - Putting an existing key updates its value in place.
/// - Iteration is over entries in insertion order.
struct ShMap {
    entries: Vec<(&'static [u8], c_char)>,
}

impl ShMap {
    fn new() -> Self {
        Self { entries: Vec::new() }
    }

    fn put(&mut self, key: &'static [u8], value: c_char) {
        for e in self.entries.iter_mut() {
            // Compare as C strings (up to and not including the NUL)
            if e.0 == key
                || CStr::from_bytes_with_nul(e.0).ok()
                    == CStr::from_bytes_with_nul(key).ok()
            {
                e.1 = value;
                return;
            }
        }
        self.entries.push((key, value));
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash = ShMap::new();
    hash.put(b"bob\0", b'h' as c_char);
    hash.put(b"sally\0", b'e' as c_char);
    hash.put(b"fred\0", b'l' as c_char);
    hash.put(b"jen\0", b'x' as c_char);
    hash.put(b"doug\0", b'o' as c_char);
    hash.put(b"jen\0", letter);

    let fmt = b"%s %c\n\0";
    for e in &hash.entries {
        unsafe {
            printf(
                fmt.as_ptr() as *const c_char,
                e.0.as_ptr() as *const c_char,
                e.1 as c_int,
            );
        }
    }
}
