// Translation of c_src/src/lib.c to Rust.
//
// Public C API: void hm_geti(int num)
//
// The function is a self-test that exercises stb_ds hash map operations.
// It produces no externally visible output (no I/O, no return value); the
// only side effect on incorrect behavior would be assertion failure /
// abort. Byte-identical output is therefore "no output" when the
// assertions all hold.
//
// We preserve the public ABI (extern "C", #[unsafe(no_mangle)], symbol
// name `hm_geti`) and replicate the observable semantics by implementing
// the same sequence of operations on a Rust HashMap, with the same
// default-value semantics that hmdefault provides in stb_ds.

use std::collections::HashMap;
use std::ffi::c_int;

/// Self-test exercising hash-map insert/get/del/default semantics.
///
/// Mirrors the behavior of the C `hm_geti(int num)` from `c_src/src/lib.c`.
/// All assertions in the original C body are preserved.
#[unsafe(no_mangle)]
pub extern "C" fn hm_geti(num: c_int) {
    let mut intmap: HashMap<c_int, c_int> = HashMap::new();
    // `default_set` mirrors stb_ds `hmdefault` — once set, hmget returns
    // this value for any missing key.
    let mut default_value: c_int = 0;
    let mut default_set: bool = false;

    // Local helpers replicating stb_ds macros for a HashMap<i32,i32>.
    fn hmgeti(m: &HashMap<c_int, c_int>, key: c_int) -> isize {
        // stbds_hmgeti: returns -1 if key not present, otherwise the
        // (positive) array index of the entry. We don't expose indices,
        // but the caller only compares against -1 / "present", so 0 vs -1
        // is sufficient.
        if m.contains_key(&key) { 0 } else { -1 }
    }
    fn hmget(
        m: &HashMap<c_int, c_int>,
        key: c_int,
        default_value: c_int,
        default_set: bool,
    ) -> c_int {
        match m.get(&key) {
            Some(&v) => v,
            None => {
                if default_set {
                    default_value
                } else {
                    0
                }
            }
        }
    }

    let mut i: c_int;

    i = 1;
    assert!(hmgeti(&intmap, i) == -1);
    // hmdefault(intmap, -2)
    default_value = -2;
    default_set = true;
    assert!(hmgeti(&intmap, i) == -1);
    assert!(hmget(&intmap, i, default_value, default_set) == -2);

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*5);
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(5));
        i += 2;
    }

    // for (i=0; i < num; i+=1) { ... assert hmget / hmget_ts ... }
    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(hmget(&intmap, i, default_value, default_set) == -2);
        } else {
            assert!(hmget(&intmap, i, default_value, default_set) == i.wrapping_mul(5));
        }
        // hmget_ts has the same observable result as hmget for a non-
        // concurrent caller; we replicate the same check.
        if (i & 1) != 0 {
            assert!(hmget(&intmap, i, default_value, default_set) == -2);
        } else {
            assert!(hmget(&intmap, i, default_value, default_set) == i.wrapping_mul(5));
        }
        i += 1;
    }

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }

    // for (i=0; i < num; i+=1) ... assert hmget == -2 / i*3 ...
    i = 0;
    while i < num {
        if (i & 1) != 0 {
            assert!(hmget(&intmap, i, default_value, default_set) == -2);
        } else {
            assert!(hmget(&intmap, i, default_value, default_set) == i.wrapping_mul(3));
        }
        i += 1;
    }

    // for (i=2; i < num; i+=4) hmdel(intmap, i);
    i = 2;
    while i < num {
        intmap.remove(&i);
        i += 4;
    }

    // for (i=0; i < num; i+=1) ... assert (i & 3) ? -2 : i*3 ...
    i = 0;
    while i < num {
        if (i & 3) != 0 {
            assert!(hmget(&intmap, i, default_value, default_set) == -2);
        } else {
            assert!(hmget(&intmap, i, default_value, default_set) == i.wrapping_mul(3));
        }
        i += 1;
    }

    // for (i=0; i < num; i+=1) hmdel(intmap, i);
    i = 0;
    while i < num {
        intmap.remove(&i);
        i += 1;
    }

    // for (i=0; i < num; i+=1) assert hmget(intmap, i) == -2;
    i = 0;
    while i < num {
        assert!(hmget(&intmap, i, default_value, default_set) == -2);
        i += 1;
    }

    // hmfree(intmap)
    intmap.clear();
    // The C code resets default-state by way of freeing the table; in
    // our Rust port the default is local state that we must reset too.
    default_set = false;
    default_value = 0;

    // for (i=0; i < num; i+=2) hmput(intmap, i, i*3);
    i = 0;
    while i < num {
        intmap.insert(i, i.wrapping_mul(3));
        i += 2;
    }

    // hmfree(intmap)
    intmap.clear();

    // Suppress unused-write warnings for the trailing reset.
    let _ = default_value;
    let _ = default_set;
}
