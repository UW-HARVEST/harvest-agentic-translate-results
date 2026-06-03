//! Rust translation of the C library found in `c_src/`.
//!
//! The C source consists primarily of an embedded copy of the
//! `stb_ds` data-structures library (dynamic arrays, string-keyed
//! hash maps and a small string arena allocator) plus a single
//! public entry point declared in `c_src/include/lib.h`:
//!
//! ```c
//! void sh_puts(int num);
//! ```
//!
//! This crate reproduces the observable behaviour of that public
//! function in idiomatic Rust while still re-exporting it with
//! C linkage so consumers of the original shared library continue
//! to work.

use std::collections::HashMap;
use std::os::raw::c_int;

mod string_arena;

use string_arena::StringArena;

/// Build the test key string for the given numeric index.
///
/// In the original C this is implemented with a static buffer and
/// `sprintf`, returning a pointer to the buffer. We just return a
/// fresh `String` instead — the value is consumed immediately by
/// the caller, so allocation overhead is irrelevant.
fn strkey(n: i32) -> String {
    format!("test_{}", n)
}

/// Public entry point mirroring `sh_puts` from `c_src/include/lib.h`.
///
/// The original function:
///   1. Allocates `num` keys (`test_0` … `test_{num-1}`) inside a
///      string arena, then resets the arena.
///   2. Creates a string-keyed hash map in arena mode and inserts
///      the entry `("a", num)`.
///   3. Asserts that the stored key starts with `'a'`, that it
///      lives in different memory from the literal `"a"` (because
///      the arena copies the string) and that the stored value is
///      `num`.
///   4. Iterates the map and prints every entry as `"<key> <value>"`.
///   5. Frees the map.
///
/// The Rust port preserves each of those observable side effects.
pub fn sh_puts_impl(num: c_int) {
    // Step 1: exercise the string arena, then reset it.
    let mut sa = StringArena::new();
    for i in 0..num {
        sa.alloc(&strkey(i));
    }
    sa.reset();

    // Step 2: build the string-keyed hash map.  We use a
    // HashMap<String, i32> here; storing owned `String`s gives us
    // the same "the stored key has a different address from the
    // literal we inserted" property that stb_ds's arena/strdup
    // modes provide in C.
    let original_key: &str = "a";
    let original_value: c_int = num;

    let mut strmap: HashMap<String, c_int> = HashMap::new();
    strmap.insert(original_key.to_string(), original_value);

    // Step 3: replicate the C asserts so any divergence from the
    // expected behaviour panics loudly in debug builds.
    let (stored_key, stored_value) = strmap
        .iter()
        .next()
        .expect("strmap should contain exactly one entry");

    assert_eq!(
        stored_key.as_bytes()[0],
        b'a',
        "stored key must start with 'a'",
    );
    assert_ne!(
        stored_key.as_str().as_ptr(),
        original_key.as_ptr(),
        "arena/strdup mode must copy the key, so addresses must differ",
    );
    assert_eq!(*stored_value, original_value, "stored value must match");

    // Step 4: print every entry, matching the C `printf("%s %d\n", ...)`
    // format used inside the `for (z=0; z < shlen(strmap); ++z)` loop.
    for (k, v) in strmap.iter() {
        println!("{} {}", k, v);
    }

    // Step 5: dropping `strmap` here is the Rust equivalent of
    // `shfree(strmap)`.
}

/// C-ABI shim so the public symbol matches the original library.
#[no_mangle]
pub extern "C" fn sh_puts(num: c_int) {
    sh_puts_impl(num);
}
