//! C-ABI shim for `libdriver.so`.
//!
//! Exports exactly the symbols that the C translation unit `c_src/src/main.c`
//! exports when compiled as a shared object (`nm -D`):
//!
//! | symbol                         | kind |
//! |--------------------------------|------|
//! | `array`                        | B    |
//! | `main`                         | T    |
//! | `perform_expensive_operations` | T    |
//!
//! `array` and `perform_expensive_operations` live in `program.rs` (shared with
//! the `driver` binary); `main` is defined here because a `#[no_mangle] main`
//! would collide with the binary's own entry point.
//!
//! The `harness_*` symbols are additional (not present in C) hooks used by the
//! differential test-suite to reach code that the C program only exposes behind
//! its ~5-minute compute loop. They call the very same code the program uses.

mod program;
mod rng;
mod strtoul;

use std::ffi::CStr;
use std::os::raw::{c_char, c_int, c_uint};

/// Reads `argv[i]` as a byte string; `None` for a NULL pointer.
///
/// # Safety
/// `argv` must be a valid array with at least `i + 1` elements.
unsafe fn argv_at<'a>(argv: *const *const c_char, i: usize) -> Option<&'a [u8]> {
    if argv.is_null() {
        return None;
    }
    let p = *argv.add(i);
    if p.is_null() {
        None
    } else {
        Some(CStr::from_ptr(p).to_bytes())
    }
}

/// `int main(int argc, char *argv[])`
///
/// Mirrors the C control flow exactly, including *which* `argv` slots are read:
/// C only dereferences `argv[0]` when `argc != 2`, and only `argv[1]` when
/// `argc == 2`.
///
/// # Safety
/// `argv` must point to an array of at least `max(argc, 1)` NUL-terminated
/// strings (or NULL pointers), exactly as a C runtime would supply.
///
/// `cfg(not(test))`: when this file is compiled as a test harness
/// (`cargo test --all-targets`) rustc generates its own entry point, and two
/// `main` symbols cannot coexist. The cdylib build is unaffected.
#[cfg(not(test))]
#[no_mangle]
pub unsafe extern "C" fn main(argc: c_int, argv: *const *const c_char) -> c_int {
    if argc != 2 {
        program::usage(argv_at(argv, 0))
    } else {
        // A NULL argv[1] faults inside C's strtoul; mirror it as "".
        program::run_with_seed_arg(argv_at(argv, 1).unwrap_or(b""))
    }
}

// ---------------------------------------------------------------------------
// Test hooks (not part of the C surface).
// ---------------------------------------------------------------------------

static mut HARNESS_RNG: Option<rng::GlibcRand> = None;

/// Seeds the port of glibc `srand()` that the program uses.
#[no_mangle]
pub extern "C" fn harness_srand(seed: c_uint) {
    unsafe {
        HARNESS_RNG = Some(rng::GlibcRand::new(seed));
    }
}

/// One draw from the port of glibc `rand()` that the program uses.
#[no_mangle]
pub extern "C" fn harness_rand() -> c_int {
    unsafe {
        match *std::ptr::addr_of_mut!(HARNESS_RNG) {
            Some(ref mut rng) => rng.next_i32(),
            None => 0,
        }
    }
}

/// The program's seed-validation decision for `arg`, without running the
/// ~5-minute compute loop.
///
/// Returns 0 and stores the resulting `unsigned int seed` in `out_seed` when the
/// C program would accept the argument; returns 1 (and leaves `out_seed`
/// untouched) when the C program would print `Invalid seed: '...'`.
///
/// # Safety
/// `arg` must be a NUL-terminated string; `out_seed` must be writable or NULL.
#[no_mangle]
pub unsafe extern "C" fn harness_parse_seed(arg: *const c_char, out_seed: *mut c_uint) -> c_int {
    if arg.is_null() {
        return 1;
    }
    match program::parse_seed(CStr::from_ptr(arg).to_bytes()) {
        Some(seed) => {
            if !out_seed.is_null() {
                *out_seed = seed;
            }
            0
        }
        None => 1,
    }
}

/// `ARRAY_SIZE` / `ITERATIONS` as compiled into this library, so the test-suite
/// can assert they match the C `#define`s.
#[no_mangle]
pub extern "C" fn harness_array_size() -> usize {
    program::ARRAY_SIZE
}

#[no_mangle]
pub extern "C" fn harness_iterations() -> usize {
    program::ITERATIONS
}
