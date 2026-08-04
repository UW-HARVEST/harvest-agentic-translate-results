// Rust translation of c_src/src/lib.c
//
// The original C code is a port of the stb_ds dynamic array / hashmap
// library plus a single public function `str_put`.  Only `str_put` is
// exposed externally, so the translation focuses on reproducing its
// observable behavior using idiomatic Rust where possible.

use std::collections::HashMap;
use std::ffi::CString;

// ---------------------------------------------------------------------------
// String arena
// ---------------------------------------------------------------------------

const STBDS_STRING_ARENA_BLOCKSIZE_MIN: usize = 512;
const STBDS_STRING_ARENA_BLOCKSIZE_MAX: usize = 1 << 20;

/// A block of bytes holding null-terminated strings.
pub struct StringBlock {
    pub next: Option<Box<StringBlock>>,
    pub storage: Vec<u8>,
}

/// String arena, analogous to `stbds_string_arena` in the C source.
pub struct StringArena {
    pub storage: Option<Box<StringBlock>>,
    pub remaining: usize,
    pub block: u8,
    pub mode: u8,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena {
            storage: None,
            remaining: 0,
            block: 0,
            mode: 0,
        }
    }

    /// Allocate a copy of `s` inside the arena and return its bytes.
    /// This mirrors the layout/management of `stbds_stralloc`.
    pub fn stralloc(&mut self, s: &str) -> Vec<u8> {
        let len = s.len() + 1; // +1 for NUL terminator like the C version

        if len > self.remaining {
            let blocksize: usize =
                STBDS_STRING_ARENA_BLOCKSIZE_MIN << ((self.block as usize) >> 1);

            if blocksize < STBDS_STRING_ARENA_BLOCKSIZE_MAX {
                self.block = self.block.saturating_add(1);
            }

            if len > blocksize {
                // Allocate a "huge" block sized to fit `s`.
                let mut storage = vec![0u8; len];
                storage[..s.len()].copy_from_slice(s.as_bytes());
                let new_block = Box::new(StringBlock {
                    next: None,
                    storage,
                });
                if let Some(ref mut head) = self.storage {
                    let mut new_block = new_block;
                    new_block.next = head.next.take();
                    head.next = Some(new_block);
                    return head.next.as_ref().unwrap().storage.clone();
                } else {
                    self.storage = Some(new_block);
                    self.remaining = 0;
                    return self.storage.as_ref().unwrap().storage.clone();
                }
            } else {
                let new_block = Box::new(StringBlock {
                    next: self.storage.take(),
                    storage: vec![0u8; blocksize],
                });
                self.storage = Some(new_block);
                self.remaining = blocksize;
                blocksize // unused, suppress warning below
            };
        }

        debug_assert!(len <= self.remaining);

        // Place the string at the end of the current block.
        let head = self.storage.as_mut().expect("arena has block");
        let block_size = head.storage.len();
        let start = block_size - self.remaining;
        let end = start + len;
        head.storage[start..start + s.len()].copy_from_slice(s.as_bytes());
        head.storage[start + s.len()] = 0;
        self.remaining -= len;

        head.storage[start..end].to_vec()
    }

    /// Drop all blocks and reset state, like `stbds_strreset`.
    pub fn strreset(&mut self) {
        // Iteratively drop the chain to avoid potential deep recursion.
        let mut cur = self.storage.take();
        while let Some(mut block) = cur {
            cur = block.next.take();
        }
        self.remaining = 0;
        self.block = 0;
        self.mode = 0;
    }
}

impl Default for StringArena {
    fn default() -> Self {
        StringArena::new()
    }
}

impl Drop for StringArena {
    fn drop(&mut self) {
        self.strreset();
    }
}

// ---------------------------------------------------------------------------
// String map (analogous to stbds shput/shget hashmap of char* -> int)
// ---------------------------------------------------------------------------

#[derive(Clone, Debug)]
pub struct StrMapEntry {
    pub key: String,
    pub value: i32,
}

pub struct StrMap {
    /// Insertion-ordered storage of entries (mirrors stbds array layout).
    pub entries: Vec<StrMapEntry>,
    /// Index lookup for O(1) access by key.
    pub index: HashMap<String, usize>,
}

impl StrMap {
    pub fn new() -> Self {
        StrMap {
            entries: Vec::new(),
            index: HashMap::new(),
        }
    }

    /// Insert or update an entry, equivalent to `shputs`.
    pub fn putentry(&mut self, entry: StrMapEntry) {
        if let Some(&idx) = self.index.get(&entry.key) {
            self.entries[idx] = entry;
        } else {
            let key = entry.key.clone();
            self.entries.push(entry);
            self.index.insert(key, self.entries.len() - 1);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    pub fn get(&self, idx: usize) -> &StrMapEntry {
        &self.entries[idx]
    }
}

impl Default for StrMap {
    fn default() -> Self {
        StrMap::new()
    }
}

// ---------------------------------------------------------------------------
// Helpers and public API
// ---------------------------------------------------------------------------

/// Equivalent to the C `strkey` helper that returns "test_<n>".
pub fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

/// Public API.  Mirrors `void str_put(int num)` from the C source.
#[no_mangle]
pub extern "C" fn str_put(num: i32) {
    let mut sa = StringArena::new();

    for i in 0..num {
        let _ = sa.stralloc(&strkey(i));
    }
    sa.strreset();

    {
        let mut strmap = StrMap::new();
        let s = StrMapEntry {
            key: "a".to_string(),
            value: num,
        };
        strmap.putentry(s.clone());

        // Assertions corresponding to STBDS_ASSERT in the C source.
        assert_eq!(strmap.entries[0].key.as_bytes()[0], b'a');
        assert_eq!(strmap.entries[0].key, s.key);
        assert_eq!(strmap.entries[0].value, s.value);

        // Mirror the C printf loop.  In the original code the format
        // string `%s` consumed the first pointer-sized field of the
        // struct, which happens to be the key, so we print key + value.
        for z in 0..strmap.len() {
            let entry = strmap.get(z);
            // Use CString purely to keep behavior similar to C strings.
            let _c = CString::new(entry.key.clone()).unwrap();
            println!("{} {}", entry.key, entry.value);
        }

        // strmap is dropped here, equivalent to `shfree`.
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn smoke_str_put() {
        str_put(3);
    }

    #[test]
    fn arena_alloc_and_reset() {
        let mut sa = StringArena::new();
        for i in 0..10 {
            let _ = sa.stralloc(&strkey(i));
        }
        sa.strreset();
        assert_eq!(sa.remaining, 0);
        assert!(sa.storage.is_none());
    }
}
