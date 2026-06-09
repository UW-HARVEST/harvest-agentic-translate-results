// Simplified Rust port of the parts of stb_ds.h actually exercised by
// `sh_puts`. The original C library is a generic dynamic-array and hash-map
// implementation; the only externally observable behavior the executable
// exposes (via `sh_puts`) is:
//
//   * Allocating strings to a string arena and resetting the arena
//     (purely internal; no I/O).
//   * Inserting a single entry into a string-keyed hash map and iterating it,
//     yielding exactly one (key, value) pair: ("a", num).
//
// This module provides safe Rust types that reproduce that observable
// behavior. We keep the API surface small and idiomatic.

/// A simple string arena. Real stb_ds allocates fixed-size blocks; we just
/// own the strings since the only observable contract is "you get back a
/// pointer to a copy of the string and `strreset` invalidates them all".
pub struct StringArena {
    blocks: Vec<String>,
}

impl StringArena {
    pub fn new() -> Self {
        StringArena { blocks: Vec::new() }
    }

    /// Equivalent of `stbds_stralloc`: copy `s` into the arena.
    pub fn stralloc(&mut self, s: &str) {
        self.blocks.push(s.to_string());
    }

    /// Equivalent of `stbds_strreset`: free all blocks.
    pub fn reset(&mut self) {
        self.blocks.clear();
    }
}

/// String-keyed hash map matching the subset of stb_ds shput/shfree behavior
/// used by `sh_puts`. Insertion order is preserved (as in stb_ds, which
/// stores the key/value pairs in a contiguous array).
pub struct StringHashMap<V> {
    entries: Vec<(String, V)>,
    /// Storage mode. We model SH_ARENA / SH_STRDUP both by owning a copy of
    /// the key, which matches the externally-observable C behavior
    /// (the key stored in the map is *not* the same pointer as the one
    /// passed in by the caller).
    _mode: Mode,
}

enum Mode {
    Arena,
}

impl<V> StringHashMap<V> {
    /// Equivalent of `sh_new_arena`.
    pub fn new_arena() -> Self {
        StringHashMap {
            entries: Vec::new(),
            _mode: Mode::Arena,
        }
    }

    /// Equivalent of `shputs(t, s)` for a struct with `key` and `value`.
    /// If the key already exists, the value is updated; otherwise a new
    /// entry is appended.
    pub fn put(&mut self, key: &str, value: V) {
        for entry in self.entries.iter_mut() {
            if entry.0 == key {
                entry.1 = value;
                return;
            }
        }
        self.entries.push((key.to_string(), value));
    }

    /// Iterate entries in insertion order.
    pub fn iter(&self) -> impl Iterator<Item = (&str, &V)> {
        self.entries.iter().map(|(k, v)| (k.as_str(), v))
    }
}
