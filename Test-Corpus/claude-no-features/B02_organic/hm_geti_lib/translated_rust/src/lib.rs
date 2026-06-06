//! Rust translation of c_src/src/lib.c
//!
//! The original C source is an stb_ds-style hash-map implementation plus a
//! single public function `hm_geti(int num)` which exercises the hash map
//! with various assertions.  The function produces no stdout/stderr output;
//! its observable behavior on success is "returns without aborting".
//!
//! Because C `assert()` is a no-op in release builds (with `NDEBUG`) and the
//! function produces no other output, a byte-identical reproduction of its
//! output requires the function to (a) terminate normally and (b) emit no
//! bytes to any standard stream.  We therefore implement the same algorithmic
//! behavior using Rust's `HashMap`, with the same default-value semantics
//! that `hmdefault`/`hmget` provide in stb_ds.
//!
//! Public C-ABI symbol: `hm_geti`.

use std::collections::HashMap;
use std::ffi::c_int;

/// Mirror of `hm_geti(int num)` from the original C source.
///
/// The original function:
///   * Creates an empty stb_ds hashmap (int -> int).
///   * Confirms missing-key behavior before/after `hmdefault`.
///   * Inserts `(i, i*5)` for even `i` in `[0, num)` and verifies lookups.
///   * Overwrites even-indexed entries with `(i, i*3)` and re-verifies.
///   * Deletes every fourth key starting at 2, then re-verifies.
///   * Deletes everything and verifies that all keys return the default.
///   * Frees, re-populates evens, and frees again.
///
/// All `STBDS_ASSERT(...)` invariants in the original are preserved here as
/// `assert!` calls.  In release-mode C builds these compile out (because
/// `NDEBUG` is defined for `assert`), but in Rust they remain — which still
/// yields the same observable output (none) so long as the hash map behaves
/// correctly.  Should an assertion ever fail it would be a Rust panic, which
/// is the closest analogue of a C `abort()` from a failed `assert`.
#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    // The stb_ds default value, set by `hmdefault(intmap, -2)`.
    const DEFAULT_VALUE: c_int = -2;

    // Helper closures to mimic stb_ds semantics.
    //
    // `hmgeti` returns the index of the entry in the underlying packed array,
    // or -1 if the key is not present.  In stb_ds, this is unaffected by
    // `hmdefault` — the default only changes `hmget`'s return value for a
    // missing key.  We don't need actual indices, only the -1-vs-non-(-1)
    // distinction, which `HashMap::contains_key` provides.
    fn hmgeti(map: &HashMap<c_int, c_int>, key: c_int) -> isize {
        if map.contains_key(&key) {
            // Any non-negative value is fine; the original asserts only check
            // for `== -1`.  We use 0 as a sentinel "present" indicator.
            0
        } else {
            -1
        }
    }

    // `hmget` returns the entry's value, or the configured default if missing.
    fn hmget(map: &HashMap<c_int, c_int>, key: c_int, default: c_int) -> c_int {
        match map.get(&key) {
            Some(v) => *v,
            None => default,
        }
    }

    // `hmget_ts` is the thread-safe variant; semantically identical to
    // `hmget` in single-threaded use.  The `temp` out-parameter holds the
    // entry index (or -1 for "not found"), but the original code only reads
    // the return value — never `temp` itself — so we just delegate.
    fn hmget_ts(map: &HashMap<c_int, c_int>, key: c_int, default: c_int, temp: &mut isize) -> c_int {
        match map.get(&key) {
            Some(v) => {
                *temp = 0;
                *v
            }
            None => {
                *temp = -1;
                default
            }
        }
    }

    let mut intmap: HashMap<c_int, c_int> = HashMap::new();
    let mut temp: isize = 0;
    let mut i: c_int;

    i = 1;
    assert!(hmgeti(&intmap, i) == -1);
    // `hmdefault(intmap, -2)` — represented by the constant DEFAULT_VALUE.
    assert!(hmgeti(&intmap, i) == -1);
    assert!(hmget(&intmap, i, DEFAULT_VALUE) == DEFAULT_VALUE);

    // Insert (i, i*5) for even i in [0, num).
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(5));
        i += 2;
    }

    i = 0;
    while i < num {
        if i & 1 != 0 {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == DEFAULT_VALUE);
        } else {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == i.wrapping_mul(5));
        }
        if i & 1 != 0 {
            assert!(hmget_ts(&intmap, i, DEFAULT_VALUE, &mut temp) == DEFAULT_VALUE);
        } else {
            assert!(hmget_ts(&intmap, i, DEFAULT_VALUE, &mut temp) == i.wrapping_mul(5));
        }
        i += 1;
    }

    // Overwrite even-indexed keys with (i, i*3).
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }

    i = 0;
    while i < num {
        if i & 1 != 0 {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == DEFAULT_VALUE);
        } else {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == i.wrapping_mul(3));
        }
        i += 1;
    }

    // Delete every fourth even key starting at 2: 2, 6, 10, ...
    i = 2;
    while i < num {
        intmap.remove(&i);
        i += 4;
    }

    i = 0;
    while i < num {
        if i & 3 != 0 {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == DEFAULT_VALUE);
        } else {
            assert!(hmget(&intmap, i, DEFAULT_VALUE) == i.wrapping_mul(3));
        }
        i += 1;
    }

    // Delete everything.
    i = 0;
    while i < num {
        intmap.remove(&i);
        i += 1;
    }

    i = 0;
    while i < num {
        assert!(hmget(&intmap, i, DEFAULT_VALUE) == DEFAULT_VALUE);
        i += 1;
    }

    // hmfree(intmap); — in C this nulls out the pointer; in Rust we drop &
    // re-create.
    drop(intmap);
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();

    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }
    drop(intmap);

    // Suppress "unused variable" warning for `j` analogue: original C declared
    // `int j;` but never used it.  We mirror by simply not declaring one.
    let _ = temp;
}
