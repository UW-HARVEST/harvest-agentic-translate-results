// Translation of c_src/src/lib.c
//
// The original C library only exposes one public function, `hm_geti`, which
// exercises stb_ds's hash-map utilities through a series of asserts. The
// function produces no externally observable output (no stdout, no return
// value, no out-pointer modifications visible to the caller). It only mutates
// internal heap state and validates internal invariants via STBDS_ASSERT.
//
// To reproduce byte-identical externally observable output (i.e. nothing) we
// model the same logical operations using Rust's HashMap. The behavior of the
// asserts is mirrored so that, for any valid `num`, the function completes
// without panicking — exactly as the C version does in release builds.

use std::collections::HashMap;
use std::os::raw::c_int;

// Mirror of the inner intmap's behavior: a key->value mapping with a default
// value returned when the key is absent.
struct IntMap {
    data: HashMap<i32, i32>,
    default: i32,
}

impl IntMap {
    fn new() -> Self {
        IntMap {
            data: HashMap::new(),
            default: 0,
        }
    }

    // hmgeti: returns the index of key in the underlying array, or -1 if
    // missing. The original `hm_geti` only ever compares this to -1, so we
    // simply return -1 when absent and 0 when present (any non-negative value
    // would do for the present case but is never inspected here beyond the
    // comparison to -1).
    fn hmgeti(&self, key: i32) -> isize {
        if self.data.contains_key(&key) {
            0
        } else {
            -1
        }
    }

    // hmdefault: set the value returned by hmget for missing keys.
    fn hmdefault(&mut self, v: i32) {
        self.default = v;
    }

    // hmget: return the value for key, or the default if missing.
    fn hmget(&self, key: i32) -> i32 {
        match self.data.get(&key) {
            Some(v) => *v,
            None => self.default,
        }
    }

    // hmget_ts: thread-safe variant in stb_ds; identical behavior here.
    fn hmget_ts(&self, key: i32) -> i32 {
        self.hmget(key)
    }

    // hmput: insert or replace.
    fn hmput(&mut self, key: i32, value: i32) {
        self.data.insert(key, value);
    }

    // hmdel: remove key.
    fn hmdel(&mut self, key: i32) {
        self.data.remove(&key);
    }

    // hmfree: drop all storage.
    fn hmfree(&mut self) {
        self.data.clear();
        self.default = 0;
    }
}

#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap = IntMap::new();

    let mut i: c_int;

    i = 1;
    assert!(intmap.hmgeti(i) == -1);
    intmap.hmdefault(-2);
    assert!(intmap.hmgeti(i) == -1);
    assert!(intmap.hmget(i) == -2);

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(5));
        i += 2;
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(intmap.hmget(i) == -2);
        } else {
            assert!(intmap.hmget(i) == i.wrapping_mul(5));
        }
        if (i & 1) != 0 {
            assert!(intmap.hmget_ts(i) == -2);
        } else {
            assert!(intmap.hmget_ts(i) == i.wrapping_mul(5));
        }
        i += 1;
    }

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(3));
        i += 2;
    }

    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(intmap.hmget(i) == -2);
        } else {
            assert!(intmap.hmget(i) == i.wrapping_mul(3));
        }
        i += 1;
    }

    i = 2;
    while i < num {
        intmap.hmdel(i);
        i += 4;
    }

    i = 0;
    while i < num {
        if (i & 3) != 0 {
            assert!(intmap.hmget(i) == -2);
        } else {
            assert!(intmap.hmget(i) == i.wrapping_mul(3));
        }
        i += 1;
    }

    i = 0;
    while i < num {
        intmap.hmdel(i);
        i += 1;
    }

    i = 0;
    while i < num {
        assert!(intmap.hmget(i) == -2);
        i += 1;
    }

    intmap.hmfree();

    i = 0;
    while i < num {
        intmap.hmput(i, i.wrapping_mul(3));
        i += 2;
    }

    intmap.hmfree();
}
