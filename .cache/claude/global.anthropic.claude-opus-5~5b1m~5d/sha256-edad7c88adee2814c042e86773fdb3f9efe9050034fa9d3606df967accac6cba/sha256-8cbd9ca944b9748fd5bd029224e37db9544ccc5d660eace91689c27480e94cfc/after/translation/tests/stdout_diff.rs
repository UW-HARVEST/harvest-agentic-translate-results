//! Byte-for-byte stdout comparison for the one printing entry point,
//! `str_dups`.  This file deliberately contains a SINGLE `#[test]`: capturing
//! output means redirecting the process-wide fd 1, so no other test may run
//! concurrently in the same process (the libtest harness itself writes progress
//! lines to fd 1).

#[path = "common/mod.rs"]
mod common;

use common::*;

#[test]
fn str_dups_stdout_is_byte_identical() {
    let g = libs();
    // CONFIGS.md row 55 (valid `num`) and ERRORS.md rows 51/53 (non-positive
    // and extreme `num`) in one pass.
    let nums: Vec<i32> = vec![
        0, 1, 2, 3, 4, 5, 7, 8, 15, 16, 63, 64, 100, 127, 128, 255, 256, 512, 1000, 2000, 4096,
        20000, -1, -2, -100, -12345, i32::MIN, i32::MIN + 1,
    ];
    // NOTE: a huge positive `num` (e.g. `i32::MAX`) is deliberately excluded:
    // the C `stralloc` loop runs `num` times, so it would take hours in BOTH
    // libraries without exercising any new code path.
    for num in nums {
        let c = capture_stdout(|| unsafe { (g.c.str_dups)(num) });
        let r = capture_stdout(|| unsafe { (g.rust.str_dups)(num) });
        assert_eq!(
            c,
            r,
            "str_dups({num}) stdout mismatch:\n  C   : {:?}\n  RUST: {:?}",
            String::from_utf8_lossy(&c),
            String::from_utf8_lossy(&r)
        );
        assert!(
            !c.is_empty(),
            "str_dups({num}) produced no output at all -- capture is broken"
        );
        eprintln!("str_dups({num}) -> {:?} ({} bytes, identical)", String::from_utf8_lossy(&c), c.len());
    }
}
