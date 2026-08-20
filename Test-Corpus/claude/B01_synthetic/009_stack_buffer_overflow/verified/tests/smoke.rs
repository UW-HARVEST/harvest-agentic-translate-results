//! Harness smoke test: proves both comparison channels actually work before the
//! real suites rely on them.

mod common;
use common::{exe, ffi};

#[test]
fn smoke_exe_channel_agrees() {
    exe::assert_same(b"3\n5\n", "smoke exe");
}

#[test]
fn smoke_ffi_channel_agrees() {
    ffi::assert_same(&ffi::Call::PrintIntLine(42), b"", "smoke printIntLine");
    ffi::assert_same(&ffi::Call::PrintLine(Some(b"hello")), b"", "smoke printLine");
    ffi::assert_same(&ffi::Call::Bad, b"4\n", "smoke bad");
    ffi::assert_same(&ffi::Call::Good, b"4\n", "smoke good");
    ffi::assert_same(&ffi::Call::Main { with_args: true }, b"1\n2\n", "smoke main");
}

/// The harness must be able to *observe* a divergence, not just report success.
/// Feeding the two channels deliberately different inputs has to fail the
/// comparison; otherwise the assertions above would be vacuous.
#[test]
fn smoke_harness_can_detect_difference() {
    let (a, _) = exe::both(b"3\n5\n");
    let (b, _) = exe::both(b"3\n6\n");
    assert_ne!(
        a, b,
        "harness is not sensitive to input changes -- comparisons would be vacuous"
    );

    let (c1, _) = ffi::both(&ffi::Call::PrintIntLine(1), b"");
    let (c2, _) = ffi::both(&ffi::Call::PrintIntLine(2), b"");
    assert_ne!(c1, c2, "ffi harness is not capturing stdout");
    assert!(!c1.stdout.is_empty(), "ffi harness captured no stdout at all");
}
