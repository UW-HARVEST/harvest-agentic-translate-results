//! Harness self-check: proves both `.so`s load, all four symbols resolve, and
//! the stdout tap really captures the libraries' log bytes.

mod common;

use common::*;

#[test]
fn both_libraries_load_and_export_all_four_symbols() {
    let h = harness();
    eprintln!("C    .so: {}", h.c.path.display());
    eprintln!("Rust .so: {}", h.r.path.display());
    // Resolution already happened in `Impl::load`; a failure would have panicked.
    assert!(h.c.path.exists());
    assert!(h.r.path.exists());
}

#[test]
fn stdout_tap_observes_the_info_log_lines() {
    let mut h = harness();
    let (ret, log) = h.assert_gotomach_logged(args(4, 1, 0, 1000));
    eprintln!("ret={ret} log={log:?}");
    assert!(
        log.contains("[INFO] Starting gotomach function"),
        "tap did not capture the entry log line; got {log:?}"
    );
    assert!(
        log.contains("[INFO] Processing completed successfully"),
        "tap did not capture the exit log line; got {log:?}"
    );
    // 11, 21, 31, 41 -> all < 1000 -> sum 104
    assert_eq!(ret, 11 + 21 + 31 + 41);
}

#[test]
fn stdout_tap_observes_the_error_log_lines() {
    let mut h = harness();
    let (ret, log) = h.assert_gotomach_logged(args(-1, 0, 0, 0));
    assert_eq!(ret, -1);
    assert!(
        log.contains("[ERROR] Invalid iteration count"),
        "got {log:?}"
    );
}

#[test]
fn ops_are_callable_through_the_exports() {
    let mut h = harness();
    assert_eq!(h.assert_op(Op::Process, 5, 0, std::ptr::null_mut()), 15);
    assert_eq!(h.assert_op(Op::Double, 5, 0, std::ptr::null_mut()), 10);
    assert_eq!(h.assert_op(Op::Triple, 5, 0, std::ptr::null_mut()), 15);
}
