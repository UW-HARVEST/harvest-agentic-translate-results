//! Phase C (rows 3/6), **isolated binary**: fault parity.
//!
//! Where the C's unchecked subscript leaves every mapped page, its "rejection"
//! is a fatal signal. These tests call through both objects in a forked child so
//! the fault can be observed, and assert the two objects are killed by the SAME
//! signal — not merely that both "failed somehow".
//!
//! Kept out of `errors.rs` so that forking never races with the page-mapping
//! test, and out of `oob_pages.rs` so a child never inherits a synthetic page.

mod support;

use support::{decode, libs};

/// Rows 3/6 — where the subscript leaves every mapped page, the C's "rejection"
/// is a fatal signal. Assert both objects are killed by the **same** signal, not
/// merely that both "failed somehow".
#[test]
fn unmappable_subscript_faults_identically_in_both() {
    use support::{CallOutcome, call_in_child};
    let l = libs();
    let (Some(cv), Some(rv)) = (l.c_table, l.rust_table) else {
        panic!("tables must be locatable");
    };

    let candidates: Vec<i32> = vec![
        i32::MIN,
        i32::MIN + 1,
        -2_000_000_000,
        -1_500_000_000,
        -500_000_000,
        -50_000_000,
        50_000_000,
        500_000_000,
        1_500_000_000,
        2_000_000_000,
        i32::MAX - 1,
        i32::MAX,
    ];

    let mut compared = 0usize;
    let mut faulted = 0usize;
    for x in candidates {
        let d = decode(x);
        let off = (d.idx as isize).wrapping_mul(4);
        if support::is_readable(cv.base.wrapping_add(off as usize), 4)
            || support::is_readable(rv.base.wrapping_add(off as usize), 4)
        {
            continue;
        }
        let c_out = call_in_child(l.c, x);
        let r_out = call_in_child(l.rust, x);
        assert_ne!(c_out, CallOutcome::Unknown, "x={x}: C outcome undetermined");
        assert_ne!(
            r_out,
            CallOutcome::Unknown,
            "x={x}: Rust outcome undetermined"
        );
        assert_eq!(
            c_out, r_out,
            "x={x}: C and Rust must fail identically (decoded {d:?})"
        );
        if matches!(c_out, CallOutcome::Signal(_)) {
            faulted += 1;
        }
        compared += 1;
    }
    assert!(compared > 0, "no unmappable subscript was exercised");
    assert!(
        faulted > 0,
        "expected at least one input to fault in both objects"
    );
    eprintln!(
        "{compared} unmappable-subscript inputs compared, {faulted} faulted identically"
    );
}

/// Sanity check for the fork harness itself: a *valid* input must be reported as
/// a normal return with the right bits, so `Signal` results above are meaningful.
#[test]
fn fork_harness_reports_normal_returns() {
    use support::{CallOutcome, call_in_child};
    let l = libs();
    for x in [-16, 0, 1, 128, 129, 1023, 1024, 8223] {
        let expect = CallOutcome::Returned(unsafe { (l.c)(x) }.to_bits());
        assert_eq!(call_in_child(l.c, x), expect, "C at x={x}");
        assert_eq!(call_in_child(l.rust, x), expect, "Rust at x={x}");
    }
}

