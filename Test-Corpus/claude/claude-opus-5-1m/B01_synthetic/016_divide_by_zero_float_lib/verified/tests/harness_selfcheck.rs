//! Sanity checks for the differential harness itself: both `.so`s load, all
//! five exported symbols resolve in both, the fd-1 capture actually captures,
//! and the C library really does emit what `c_src/src/driver.c` says it does
//! (so a harness that silently captured nothing could not make the suite pass).

#[macro_use]
mod common;

use common::*;
use std::ffi::CString;

fn main() {
    common::run(cases![
        both_libraries_load_with_all_five_symbols,
        capture_actually_captures,
        capture_survives_a_panic,
        c_library_emits_the_documented_transcript,
        rust_library_is_not_a_stub,
    ]);
}

fn both_libraries_load_with_all_five_symbols() {
    let l = libs();
    // Resolving the symbols happens in `Impl::load`; reaching here means all
    // five (printLine, printIntLine, bad, good, driver) were found in both.
    assert!(l.c.path.exists(), "C .so missing: {}", l.c.path.display());
    assert!(
        l.rs.path.exists(),
        "Rust .so missing: {}",
        l.rs.path.display()
    );
}

fn capture_actually_captures() {
    let l = libs();
    let msg = CString::new("harness-probe").unwrap();
    let (out, ()) = capture(|| unsafe { (l.c.api.print_line)(msg.as_ptr()) });
    assert_eq!(out, b"harness-probe\n", "C capture gave \"{}\"", esc(&out));

    let (out, ()) = capture(|| unsafe { (l.rs.api.print_line)(msg.as_ptr()) });
    assert_eq!(out, b"harness-probe\n", "Rust capture gave \"{}\"", esc(&out));

    // An empty window must capture nothing (proves we are not re-reading stale
    // bytes from the scratch file, which would make every comparison vacuous).
    let (out, ()) = capture(|| {});
    assert!(out.is_empty(), "expected nothing, got \"{}\"", esc(&out));
}

fn capture_survives_a_panic() {
    let r = std::panic::catch_unwind(|| {
        capture(|| panic!("boom"));
    });
    assert!(r.is_err(), "the panic should have propagated");
    // stdout must still be usable and capturing must still work.
    let l = libs();
    let (out, ()) = capture(|| unsafe { (l.c.api.print_int_line)(1234) });
    assert_eq!(out, b"1234\n", "capture broken after a panic: \"{}\"", esc(&out));
}

/// Golden transcript taken straight from the C source, so that a broken capture
/// cannot make the differential comparisons vacuously succeed.
fn c_library_emits_the_documented_transcript() {
    let l = libs();
    let (out, ()) = capture(|| unsafe { (l.c.api.driver)(2.0, 0.0) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n-2147483648\nFinished bad()\n",
        "unexpected C transcript: \"{}\"",
        esc(&out)
    );

    let (out, ()) = capture(|| unsafe { (l.c.api.driver)(0.0, 4.0) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Calling good()...\n50\nThis would result in a divide by zero\n\
         Finished good()\nCalling bad()...\n25\nFinished bad()\n",
        "unexpected C transcript: \"{}\"",
        esc(&out)
    );
}

/// The Rust `.so` must actually do the work, not merely export the names.
fn rust_library_is_not_a_stub() {
    let l = libs();
    let (out, ()) = capture(|| unsafe { (l.rs.api.driver)(2.0, 0.0) });
    assert_eq!(
        String::from_utf8_lossy(&out),
        "Calling good()...\n50\n50\nFinished good()\nCalling bad()...\n-2147483648\nFinished bad()\n",
        "unexpected Rust transcript: \"{}\"",
        esc(&out)
    );
}
