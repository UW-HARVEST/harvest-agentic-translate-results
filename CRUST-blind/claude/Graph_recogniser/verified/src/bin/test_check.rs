// The `check` module mirrors C's CHECK(x) and DEBUGIN(stm) macros from check.h:
//   CHECK(x): assert(x) when NDEBUG is undefined, no-op otherwise.
//   DEBUGIN(stm): emit `stm` when NDEBUG is undefined, no-op otherwise.
// In Rust these are file-local macros gated on `cfg(debug_assertions)`.
// The module exposes nothing public; we still verify it loads alongside its
// companions and that debug-assert behavior matches the C check semantics.

// Pull the module path through the crate to make sure it is reachable.
#[allow(unused_imports)]
use Graph_recogniser::check as _check_module;

#[test]
fn test_check_module_compiles() {
    // The module is a `pub mod check;` with macros that have no public items,
    // mirroring the header-only `check.h` in C. We simply verify the module
    // path is valid and doesn't introduce any link errors.
    let _: () = ();
}

#[test]
fn test_debug_assert_passes_for_true_condition() {
    // Equivalent to CHECK(x) being a no-op or successful assert when x is true.
    debug_assert!(true);
    debug_assert!(1 + 1 == 2);
}

#[test]
fn test_debug_assert_eq_for_equal_values() {
    debug_assert_eq!(2 + 2, 4);
    debug_assert_eq!("foo", "foo");
}

#[test]
#[cfg(debug_assertions)]
fn test_debug_assert_fires_in_debug_builds() {
    // CHECK(false) panics under NDEBUG=undefined builds.
    let result = std::panic::catch_unwind(|| {
        debug_assert!(false, "should panic in debug");
    });
    assert!(result.is_err(), "debug_assert!(false) must panic in debug builds");
}

fn main() {}
