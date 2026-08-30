//! Phase C row E10 — the output sink itself fails.
//!
//! `stdout` is pointed at `/dev/full`, so every `write(2)` fails with `ENOSPC`.
//! `driver` ignores the return values of `printf`/`puts`, so neither
//! implementation may reject, abort, or panic; and both must leave the stream in
//! the same error state.
//!
//! This lives in its own test binary because it deliberately sets the `stdout`
//! error indicator for the whole process.

mod common;

use common::*;
use std::ffi::c_int;

#[test]
fn err_e10_failing_stdout_dev_full() {
    if !std::path::Path::new("/dev/full").exists() {
        eprintln!("skipping: /dev/full unavailable on this system");
        return;
    }
    let f = impls();

    let cases: [(c_int, c_int); 6] = [
        (0, 0),
        (0, -1),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (0x1234_5678, -1),
        (-7, 42),
    ];

    for &(x, y) in &cases {
        let c_err = run_to_dev_full(|| unsafe { (f.c)(x, y) });
        let r_err = run_to_dev_full(|| unsafe { (f.rust)(x, y) });
        assert_eq!(
            c_err, r_err,
            "ferror(stdout) differs after driver({x}, {y}) against /dev/full: C={c_err} Rust={r_err}"
        );
        assert!(c_err, "sanity: writing to /dev/full should have set the error flag");
    }

    // The stream recovers identically for both after clearerr.
    diff_one_expect(3, 4, &expected_text(3, 4));
}
