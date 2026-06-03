// Rust translation of c_src/src/lib.c
//
// The original C code embeds a copy of the stb_ds hash-map / dynamic array
// library and exposes a single public function `helxo(char letter)`.
// `helxo` builds a small string-keyed hash map and prints its contents.
//
// Because the only exported symbol is `helxo`, this Rust translation
// re-implements the externally observable behaviour using a small
// insertion-ordered map (a `Vec` of key/value pairs) so that updates to an
// existing key keep the original insertion position, mirroring the behaviour
// of `shput` in stb_ds.

use std::os::raw::c_char;

/// Insertion-ordered string -> char map with `shput`-like semantics.
struct OrderedStrMap {
    entries: Vec<(String, c_char)>,
}

impl OrderedStrMap {
    fn new() -> Self {
        OrderedStrMap {
            entries: Vec::new(),
        }
    }

    /// Insert or update `key` with `value`, preserving the original insertion
    /// position when the key already exists.
    fn put(&mut self, key: &str, value: c_char) {
        if let Some(slot) = self.entries.iter_mut().find(|(k, _)| k == key) {
            slot.1 = value;
        } else {
            self.entries.push((key.to_owned(), value));
        }
    }

    fn len(&self) -> usize {
        self.entries.len()
    }

    fn get(&self, index: usize) -> (&str, c_char) {
        let (k, v) = &self.entries[index];
        (k.as_str(), *v)
    }
}

/// Translation of the C function `void helxo(char letter)` from
/// `c_src/src/lib.c`.
///
/// Builds a small string-keyed hash map, then overwrites the value associated
/// with `"jen"` using the supplied `letter`, and finally prints each
/// `"<key> <value>"` pair to stdout in insertion order.
#[no_mangle]
pub extern "C" fn helxo(letter: c_char) {
    let mut hash = OrderedStrMap::new();
    let name = "jen";

    hash.put("bob", b'h' as c_char);
    hash.put("sally", b'e' as c_char);
    hash.put("fred", b'l' as c_char);
    hash.put("jen", b'x' as c_char);
    hash.put("doug", b'o' as c_char);

    hash.put(name, letter);

    for z in 0..hash.len() {
        let (key, value) = hash.get(z);
        // The original C code prints "%s %c\n" — the key followed by the
        // associated char value.
        println!("{} {}", key, value as u8 as char);
    }

    // `shfree(hash)` in the original source releases the map's storage; in
    // Rust this happens automatically when `hash` is dropped at the end of
    // this function.
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn put_preserves_insertion_order_and_updates_in_place() {
        let mut m = OrderedStrMap::new();
        m.put("bob", b'h' as c_char);
        m.put("sally", b'e' as c_char);
        m.put("fred", b'l' as c_char);
        m.put("jen", b'x' as c_char);
        m.put("doug", b'o' as c_char);

        // Updating an existing key should not change its index.
        m.put("jen", b'Z' as c_char);

        assert_eq!(m.len(), 5);
        assert_eq!(m.get(0).0, "bob");
        assert_eq!(m.get(1).0, "sally");
        assert_eq!(m.get(2).0, "fred");
        assert_eq!(m.get(3), ("jen", b'Z' as c_char));
        assert_eq!(m.get(4).0, "doug");
    }

    #[test]
    fn helxo_runs_without_panicking() {
        helxo(b'!' as c_char);
    }
}
