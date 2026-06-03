// Translation of c_src/src/lib.c to Rust.
//
// The original C file embeds a portion of the `stb_ds.h` hash-map / dynamic
// array library and exposes a single public function `hm_geti(int num)` that
// exercises the integer-keyed hash map.  The library implementation in C is
// generic across any element type via macros; in Rust we provide an
// equivalent specialized implementation using `HashMap` and a dynamic array
// of (key, value) pairs that mirrors the behavior used by `hm_geti`.

use std::collections::HashMap;

/// A hash map mirroring the subset of `stb_ds` semantics used by `hm_geti`.
///
/// The C library stores entries in a contiguous array (so they can be
/// iterated by index) plus a hash table that maps keys to indices into that
/// array.  The "default" value is what `hmget` returns when a key is missing.
/// `hmgeti` returns the index into the entries array, or `-1` if the key is
/// missing.
pub struct IntMap {
    entries: Vec<(i32, i32)>,
    index: HashMap<i32, usize>,
    default_value: i32,
}

impl IntMap {
    pub fn new() -> Self {
        IntMap {
            entries: Vec::new(),
            index: HashMap::new(),
            default_value: 0,
        }
    }

    /// Equivalent to `hmdefault(map, v)`.
    pub fn set_default(&mut self, v: i32) {
        self.default_value = v;
    }

    /// Equivalent to `hmgeti(map, k)` — returns the index of the entry, or
    /// `-1` if the key is missing.
    pub fn geti(&self, k: i32) -> isize {
        match self.index.get(&k) {
            Some(&i) => i as isize,
            None => -1,
        }
    }

    /// Equivalent to `hmget(map, k)` — returns the value associated with the
    /// key, or the default value if the key is missing.
    pub fn get(&self, k: i32) -> i32 {
        match self.index.get(&k) {
            Some(&i) => self.entries[i].1,
            None => self.default_value,
        }
    }

    /// Equivalent to `hmget_ts(map, k, temp)` — returns the value and writes
    /// the index (or `-1`) to `temp`.
    pub fn get_ts(&self, k: i32, temp: &mut isize) -> i32 {
        match self.index.get(&k) {
            Some(&i) => {
                *temp = i as isize;
                self.entries[i].1
            }
            None => {
                *temp = -1;
                self.default_value
            }
        }
    }

    /// Equivalent to `hmput(map, k, v)`. Inserts or updates an entry.
    pub fn put(&mut self, k: i32, v: i32) {
        if let Some(&i) = self.index.get(&k) {
            self.entries[i].1 = v;
        } else {
            let i = self.entries.len();
            self.entries.push((k, v));
            self.index.insert(k, i);
        }
    }

    /// Equivalent to `hmdel(map, k)`. Removes the entry; mimics the
    /// stb_ds behavior of swapping the last element into the freed slot.
    pub fn del(&mut self, k: i32) -> bool {
        if let Some(idx) = self.index.remove(&k) {
            let last = self.entries.len() - 1;
            if idx != last {
                let moved_key = self.entries[last].0;
                self.entries.swap(idx, last);
                self.entries.pop();
                self.index.insert(moved_key, idx);
            } else {
                self.entries.pop();
            }
            true
        } else {
            false
        }
    }

    /// Equivalent to `hmfree(map)` — empties the map.
    pub fn free(&mut self) {
        self.entries.clear();
        self.index.clear();
        self.default_value = 0;
    }
}

impl Default for IntMap {
    fn default() -> Self {
        Self::new()
    }
}

/// Translation of `void hm_geti(int num)` from `c_src/src/lib.c`.
///
/// Mirrors the assertions in the original: the map starts empty, default is
/// set to `-2`, items are inserted, updated, deleted and re-inserted, with
/// asserts checking the values at each step.
pub fn hm_geti(num: i32) {
    let mut intmap = IntMap::new();
    let mut temp: isize = 0;

    let i = 1;
    assert_eq!(intmap.geti(i), -1);
    intmap.set_default(-2);
    assert_eq!(intmap.geti(i), -1);
    assert_eq!(intmap.get(i), -2);

    let mut i = 0;
    while i < num {
        intmap.put(i, i * 5);
        i += 2;
    }

    let mut i = 0;
    while i < num {
        if i & 1 != 0 {
            assert_eq!(intmap.get(i), -2);
        } else {
            assert_eq!(intmap.get(i), i * 5);
        }
        if i & 1 != 0 {
            assert_eq!(intmap.get_ts(i, &mut temp), -2);
        } else {
            assert_eq!(intmap.get_ts(i, &mut temp), i * 5);
        }
        i += 1;
    }

    let mut i = 0;
    while i < num {
        intmap.put(i, i * 3);
        i += 2;
    }

    let mut i = 0;
    while i < num {
        if i & 1 != 0 {
            assert_eq!(intmap.get(i), -2);
        } else {
            assert_eq!(intmap.get(i), i * 3);
        }
        i += 1;
    }

    let mut i = 2;
    while i < num {
        intmap.del(i);
        i += 4;
    }

    let mut i = 0;
    while i < num {
        if i & 3 != 0 {
            assert_eq!(intmap.get(i), -2);
        } else {
            assert_eq!(intmap.get(i), i * 3);
        }
        i += 1;
    }

    let mut i = 0;
    while i < num {
        intmap.del(i);
        i += 1;
    }

    let mut i = 0;
    while i < num {
        assert_eq!(intmap.get(i), -2);
        i += 1;
    }

    intmap.free();

    let mut i = 0;
    while i < num {
        intmap.put(i, i * 3);
        i += 2;
    }

    intmap.free();
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hm_geti_small() {
        hm_geti(16);
    }

    #[test]
    fn test_hm_geti_large() {
        hm_geti(1000);
    }
}
