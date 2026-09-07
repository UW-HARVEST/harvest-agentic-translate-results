//! Sanity check on the harness itself: the stdout capture must produce
//! non-trivial, input-dependent bytes, and the comparison must be able to fail.
mod common;
use common::*;
use std::sync::Mutex;
static L: Mutex<()> = Mutex::new(());

#[test]
fn harness_capture_is_input_dependent_and_can_fail() {
    let _g = L.lock().unwrap_or_else(|e| e.into_inner());
    let p = Pair::load();
    let (c, r) = unsafe { p.both::<FnDoubleneg>(SYM_DOUBLENEG) };
    let (_, o1) = capture_stdout(|| unsafe { c(1, 2, 3, 4) });
    let (_, o2) = capture_stdout(|| unsafe { c(9, 8, 7, 6) });
    let (_, o1r) = capture_stdout(|| unsafe { r(1, 2, 3, 4) });
    assert!(o1.len() > 500, "capture too short: {} bytes", o1.len());
    assert_ne!(o1, o2, "capture is not input-dependent -- harness is vacuous");
    assert_eq!(o1, o1r, "C vs Rust for (1,2,3,4)");
    // The comparison machinery reports a real difference when there is one.
    assert!(diff_report("nc", &o1, &o2).contains("first difference at byte"));
    eprintln!("capture sizes: {} / {} bytes", o1.len(), o2.len());
}
