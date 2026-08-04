// Translation of c_src/src/lib.c
//
// The public C library exposes only `hm_geti(int num)` (see c_src/include/lib.h).
// The function exercises an stb_ds-style hash map and only validates internal
// invariants via `assert(...)`. There is no I/O.
//
// We faithfully reproduce the externally-observable behaviour: the function
// performs the same logical sequence of map operations, and panics (the Rust
// equivalent of an `assert` failure) only when the C code would have done the
// same. Because none of the C asserts ever fire for any non-negative `num`,
// neither do the corresponding checks here.

use std::collections::HashMap;
use std::ffi::c_int;

// Mirrors the C `intmap` plus its `hmdefault` slot. In stb_ds, `hmdefault`
// stores a fallback value returned by `hmget` when the key is absent; `hmgeti`
// is unaffected and still returns -1 for missing keys.
struct IntMap {
    map: HashMap<c_int, c_int>,
    default_value: c_int,
}

impl IntMap {
    fn new() -> Self {
        IntMap {
            map: HashMap::new(),
            // stb_ds's default for `hmdefault` before it is ever set is 0,
            // matching the zero-initialised value slot at index -1.
            default_value: 0,
        }
    }

    // Returns the (opaque) array index of the key, or -1 if missing.
    // The exact index value is implementation-defined in stb_ds, but the C
    // code only ever compares the result against -1, so any non-negative
    // value when present is observationally equivalent.
    fn geti(&self, key: c_int) -> isize {
        if self.map.contains_key(&key) {
            // Any non-negative integer satisfies the C assertions; use the
            // current size as a stable, deterministic stand-in.
            self.map.len() as isize
        } else {
            -1
        }
    }

    fn get(&self, key: c_int) -> c_int {
        *self.map.get(&key).unwrap_or(&self.default_value)
    }

    fn put(&mut self, key: c_int, value: c_int) {
        self.map.insert(key, value);
    }

    fn set_default(&mut self, value: c_int) {
        self.default_value = value;
    }

    fn del(&mut self, key: c_int) {
        self.map.remove(&key);
    }

    fn free(&mut self) {
        self.map.clear();
        self.default_value = 0;
    }
}

/// Public C entry point: `void hm_geti(int num);`
///
/// The C source defines this name directly — there are no preprocessor
/// renaming macros wrapping it — so the linker symbol is simply `hm_geti`.
#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap = IntMap::new();

    let mut i: c_int;

    i = 1;
    assert!(intmap.geti(i) == -1);
    intmap.set_default(-2);
    assert!(intmap.geti(i) == -1);
    assert!(intmap.get(i) == -2);

    i = 0;
    while i < num {
        intmap.put(i, i.wrapping_mul(5));
        i += 2;
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(intmap.get(i) == -2);
        } else {
            assert!(intmap.get(i) == i.wrapping_mul(5));
        }
        // hmget_ts has identical semantics for lookup; the `temp` out-param
        // is never inspected by the test, so we reuse the same path.
        if (i & 1) != 0 {
            assert!(intmap.get(i) == -2);
        } else {
            assert!(intmap.get(i) == i.wrapping_mul(5));
        }
        i += 1;
    }

    i = 0;
    while i < num {
        intmap.put(i, i.wrapping_mul(3));
        i += 2;
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(intmap.get(i) == -2);
        } else {
            assert!(intmap.get(i) == i.wrapping_mul(3));
        }
        i += 1;
    }

    i = 2;
    while i < num {
        intmap.del(i);
        i += 4;
    }

    i = 0;
    while i < num {
        if (i & 3) != 0 {
            assert!(intmap.get(i) == -2);
        } else {
            assert!(intmap.get(i) == i.wrapping_mul(3));
        }
        i += 1;
    }

    i = 0;
    while i < num {
        intmap.del(i);
        i += 1;
    }

    i = 0;
    while i < num {
        assert!(intmap.get(i) == -2);
        i += 1;
    }

    intmap.free();

    i = 0;
    while i < num {
        intmap.put(i, i.wrapping_mul(3));
        i += 2;
    }
    intmap.free();
}
