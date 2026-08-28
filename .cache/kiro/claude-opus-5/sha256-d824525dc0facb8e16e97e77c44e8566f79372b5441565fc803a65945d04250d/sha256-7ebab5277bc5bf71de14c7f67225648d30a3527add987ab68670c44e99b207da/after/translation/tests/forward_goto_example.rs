//! Differential tests for the lowest-level function:
//!
//! ```c
//! int forward_goto_example(int x);
//! ```
//!
//! Both implementations are invoked through `dlopen`/`dlsym`, never by calling
//! the Rust crate directly.

mod common;

use common::*;
use std::ffi::c_int;

const NAME: &str = "forward_goto_example";

/// One `#[test]` on purpose: the harness redirects the process-wide fds 1 and 2,
/// so sub-cases must run strictly sequentially.
#[test]
fn matches_c() {
    let c: libloading::Symbol<ForwardGotoExample> = sym(c_lib(), NAME);
    let r: libloading::Symbol<ForwardGotoExample> = sym(rust_lib(), NAME);

    let mut diffs = Diffs::new();

    let mut inputs: Vec<c_int> = vec![
        // the `goto error` path
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1_000_000,
        -12345,
        -2,
        -1,
        // the fall-through path
        0,
        1,
        2,
        3,
        7,
        10,
        99,
        12345,
        1_000_000,
        // values where `x * 2` overflows a 32-bit int
        0x3FFF_FFFF,
        0x4000_0000,
        0x4000_0001,
        i32::MAX - 1,
        i32::MAX,
    ];
    // A deterministic spread of additional values.
    let mut state: u32 = 0x1234_5678;
    for _ in 0..64 {
        state = state.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        inputs.push(state as c_int);
    }

    for x in inputs {
        let got_c = capture(|| unsafe { c(x) });
        let got_r = capture(|| unsafe { r(x) });
        diffs.compare(&format!("x={x}"), &got_c, &got_r);
    }

    diffs.assert_empty();
}
