//! `CONFIGS.md` row 11 — the *first-ever* `driver` call in a fresh process.
//!
//! This lives in its own integration-test binary because `cargo test` gives
//! each test file its own process: only here can we guarantee that the call
//! below is the first one each freshly `dlopen`ed library ever sees, with
//! `static int y` still holding its initialiser `123`.
//!
//! It proves `driver` assigns `y = local_y` *before* the guard reads `y`, so the
//! `123` initialiser is unobservable through the public API — and that the Rust
//! `AtomicI32` initialiser behaves the same way.

mod common;

use common::{assert_same, expected};

#[test]
fn first_call_in_process_ignores_static_initialiser() {
    // With y still == 123 in both libraries, driver(1, 2, 3) must SUCCEED.
    // If the guard read the stale 123, this would print "Error: x == 1 but y != 2".
    let out = assert_same(1, 2, 3);
    let s = String::from_utf8_lossy(&out);
    assert_eq!(
        s,
        format!("{}{}", expected::OK, expected::result_line(0)),
        "first call in the process must not observe the `static int y = 123` initialiser"
    );
    assert!(!s.contains("but y != 2"));
}
