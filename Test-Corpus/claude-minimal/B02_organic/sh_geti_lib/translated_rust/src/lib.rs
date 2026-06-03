// Rust translation of c_src/src/lib.c
//
// The original C code is a copy of the stb_ds library along with a public
// `sh_geti` test routine that exercises the string-keyed hash map. The
// public ABI of this crate exposes the same `sh_geti` entry point with a
// matching `extern "C"` signature so it remains drop-in compatible.
//
// Internally the implementation is rewritten using safe Rust data
// structures (`HashMap` and `Vec`). The behavior of the original test
// routine is preserved: it builds a string arena, populates a hash map
// with even-indexed keys, queries it (validating the "default" value
// returned for missing keys), deletes selected entries, and verifies the
// resulting state.

use std::collections::HashMap;
use std::os::raw::c_int;

// ---------------------------------------------------------------------------
// String arena: equivalent of `stbds_string_arena` plus `stralloc`/`strreset`.
// The arena owns all strings allocated through it. Reset drops them.
// ---------------------------------------------------------------------------

#[derive(Default)]
struct StringArena {
    storage: Vec<String>,
}

impl StringArena {
    fn new() -> Self {
        StringArena {
            storage: Vec::new(),
        }
    }

    fn alloc(&mut self, s: &str) -> &str {
        self.storage.push(s.to_string());
        // Return a reference to the freshly stored string.
        self.storage.last().unwrap().as_str()
    }

    fn reset(&mut self) {
        self.storage.clear();
    }
}

// ---------------------------------------------------------------------------
// String-keyed hash map with a default value, mirroring the subset of the
// stb_ds API that `sh_geti` exercises (`shdefault`, `shput`, `shget`,
// `shgeti`, `shdel`, `shlen`, `shfree`).
// ---------------------------------------------------------------------------

struct StrMap {
    map: HashMap<String, i32>,
    default_value: i32,
}

impl StrMap {
    fn new() -> Self {
        StrMap {
            map: HashMap::new(),
            default_value: 0,
        }
    }

    fn set_default(&mut self, v: i32) {
        self.default_value = v;
    }

    fn put(&mut self, key: &str, value: i32) {
        self.map.insert(key.to_string(), value);
    }

    fn get(&self, key: &str) -> i32 {
        *self.map.get(key).unwrap_or(&self.default_value)
    }

    fn geti(&self, key: &str) -> isize {
        if self.map.contains_key(key) {
            0
        } else {
            -1
        }
    }

    fn del(&mut self, key: &str) -> bool {
        self.map.remove(key).is_some()
    }

    fn len(&self) -> usize {
        self.map.len()
    }

    fn iter(&self) -> impl Iterator<Item = (&String, &i32)> {
        self.map.iter()
    }
}

// ---------------------------------------------------------------------------
// Helper that mirrors the C `strkey` function. Each call produces a fresh
// owned string of the form "test_<n>".
// ---------------------------------------------------------------------------

fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

// ---------------------------------------------------------------------------
// Public entry point.
//
// `sh_geti` is declared in `c_src/include/lib.h` as `void sh_geti(int num);`
// We expose the same signature to remain ABI-compatible.
// ---------------------------------------------------------------------------

#[no_mangle]
pub extern "C" fn sh_geti(num: c_int) {
    let num: i32 = num;

    // Exercise the string arena, mirroring the original loop. We don't keep
    // the returned pointers (the C code didn't either); the goal is to
    // hammer the arena allocation/reset code path.
    let mut sa = StringArena::new();
    for i in 0..num {
        let _ = sa.alloc(&strkey(i));
    }
    sa.reset();

    for j in 0..2 {
        let mut strmap = StrMap::new();

        // Before any insertion `shgeti` for "foo" must return -1.
        assert_eq!(strmap.geti("foo"), -1);

        // The original code switches between strdup/arena modes here. In
        // our HashMap the keys are already owned `String`s, so the choice
        // is invisible to callers. We still keep the branch to retain
        // structural parity with the C source.
        if j == 0 {
            // sh_new_strdup: nothing extra to do — keys are owned.
        } else {
            // sh_new_arena: nothing extra to do — keys are owned.
        }

        assert_eq!(strmap.geti("foo"), -1);

        // shdefault: any subsequent `get` for a missing key returns -2.
        strmap.set_default(-2);

        // `shgeti` reports presence and is independent of the default.
        assert_eq!(strmap.geti("foo"), -1);

        // Insert the even-indexed keys.
        let mut i = 0;
        while i < num {
            strmap.put(&strkey(i), i * 3);
            i += 2;
        }

        // Replicate the printf loop from the C source. It iterates over
        // the live entries of the map and prints `<key> <value>`.
        let entries: Vec<(String, i32)> = strmap
            .iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (key, value) in &entries {
            println!("{} {}", key, value);
        }
        // Sanity: we expect exactly num/2 + (num%2) entries (the even ones
        // strictly less than `num`).
        let expected_len = ((num + 1) / 2) as usize;
        assert_eq!(strmap.len(), expected_len);

        // Verify: odd indices return the default; even indices return i*3.
        for i in 0..num {
            let v = strmap.get(&strkey(i));
            if i & 1 != 0 {
                assert_eq!(v, -2);
            } else {
                assert_eq!(v, i * 3);
            }
        }

        // Delete entries at i = 2, 6, 10, ... (i.e. i % 4 == 2).
        let mut i = 2;
        while i < num {
            strmap.del(&strkey(i));
            i += 4;
        }

        // After the deletions: keys with i & 3 != 0 yield the default.
        for i in 0..num {
            let v = strmap.get(&strkey(i));
            if i & 3 != 0 {
                assert_eq!(v, -2);
            } else {
                assert_eq!(v, i * 3);
            }
        }

        // Wipe everything.
        for i in 0..num {
            strmap.del(&strkey(i));
        }

        // The map is now empty — every lookup must return the default.
        for i in 0..num {
            let v = strmap.get(&strkey(i));
            assert_eq!(v, -2);
        }

        // shfree: dropping `strmap` at the end of this scope handles it.
        drop(strmap);
    }
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sh_geti_runs_with_small_num() {
        sh_geti(8);
    }

    #[test]
    fn sh_geti_runs_with_zero() {
        sh_geti(0);
    }

    #[test]
    fn string_arena_basic() {
        let mut sa = StringArena::new();
        let _ = sa.alloc("hello");
        let _ = sa.alloc("world");
        sa.reset();
    }
}
