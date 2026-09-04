// CONFIGS.md C18 / ERRORS.md E6 -- the non-terminating path.
//
// For x > 0 with y < 0 the C code loops ~2^31 times (`y == 0` is false for a
// negative y, so `y--` runs forever until signed overflow, which is UB). Such a
// call cannot be run to completion, so instead each implementation is run in a
// forked child whose stdout is a pipe; the first N bytes of output are compared
// and the child is then killed. This verifies the divergent-path PREFIX rather
// than skipping the row.
//
// These tests live in their own test binary and are serialized so that fork()
// is never called while another test thread is inside libc.

mod common;
use common::*;

use std::sync::{Mutex, OnceLock};

fn fork_lock() -> std::sync::MutexGuard<'static, ()> {
    static L: OnceLock<Mutex<()>> = OnceLock::new();
    L.get_or_init(|| Mutex::new(()))
        .lock()
        .unwrap_or_else(|e| e.into_inner())
}

const WANT: usize = 64 * 1024;

#[track_caller]
fn assert_same_prefix(x: i32, y: i32, want: usize) {
    let _g = fork_lock();
    let c = prefix_via_fork(c_driver(), x, y, want);
    let r = prefix_via_fork(rust_driver(), x, y, want);
    assert!(
        c.len() >= want / 2,
        "C produced only {} bytes for driver({x}, {y}); expected an unbounded stream",
        c.len()
    );
    assert_eq!(
        c.len(),
        r.len(),
        "prefix length mismatch for driver({x}, {y})"
    );
    if c != r {
        let at = c.iter().zip(r.iter()).position(|(a, b)| a != b).unwrap();
        panic!(
            "prefix divergence for driver({x}, {y}) at byte {at}\n  C   : {:?}\n  Rust: {:?}",
            String::from_utf8_lossy(&c[at.saturating_sub(40)..(at + 40).min(c.len())]),
            String::from_utf8_lossy(&r[at.saturating_sub(40)..(at + 40).min(r.len())]),
        );
    }
}

// E6 / C18: x > 0, y < 0 -- unbounded output; compare the prefix.
#[test]
fn err_e6_negative_y_prefix() {
    for (x, y) in [(1, -1), (2, -3), (5, -100), (3, i32::MIN + 1)] {
        assert_same_prefix(x, y, WANT);
    }
}

#[test]
fn cfg_c18_infinite_path_prefix() {
    let mut rng = Rng::new(SEED ^ 18);
    for _ in 0..6 {
        let x = rng.range(1, 50);
        let y = rng.range(-5000, -1);
        assert_same_prefix(x, y, WANT);
    }
    // INT_MIN on the y axis with a positive x: the extreme of the same path.
    assert_same_prefix(7, i32::MIN, WANT);
}
