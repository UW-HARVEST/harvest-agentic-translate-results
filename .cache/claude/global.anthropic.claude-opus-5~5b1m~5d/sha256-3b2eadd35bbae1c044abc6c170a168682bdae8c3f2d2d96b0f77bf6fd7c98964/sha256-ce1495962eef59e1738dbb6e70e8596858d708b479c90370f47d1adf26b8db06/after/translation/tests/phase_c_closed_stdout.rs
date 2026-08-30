//! Phase C row E11 — file descriptor 1 is closed outright.
//!
//! Flushing then fails with `EBADF`. `driver` ignores the `printf`/`puts` return
//! values, so both implementations must return normally and end up in the same
//! stream error state.
//!
//! Own test binary: closing fd 1 and poisoning `stdout` must not leak into the
//! other test binaries.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn err_e11_closed_stdout_fd() {
    let f = impls();

    let cases: [(c_int, c_int); 5] = [
        (0, 0),
        (0, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (-123_456, 654_321),
    ];

    for &(x, y) in &cases {
        let c_err = run_with_stdout_closed(|| unsafe { (f.c)(x, y) });
        let r_err = run_with_stdout_closed(|| unsafe { (f.rust)(x, y) });
        assert_eq!(
            c_err, r_err,
            "ferror(stdout) differs after driver({x}, {y}) with fd 1 closed: C={c_err} Rust={r_err}"
        );
    }

    // Still healthy and still identical afterwards.
    diff_one_expect(-5, -6, &expected_text(-5, -6));
}
