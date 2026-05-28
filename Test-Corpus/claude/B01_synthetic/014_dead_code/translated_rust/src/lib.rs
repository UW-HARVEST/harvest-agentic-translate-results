// Rust translation of the C reference. Functions mirror the C semantics
// exactly and are exported with the same external symbol names so the
// resulting cdylib can stand in for the C shared library.

use std::ffi::CStr;
use std::os::raw::c_char;
#[cfg(not(test))]
use std::os::raw::c_int;

/// Mirror of C's `printLine`. Prints the C string followed by a newline if
/// the pointer is non-null. No-op when `line` is null.
#[no_mangle]
pub extern "C" fn printLine(line: *const c_char) {
    if !line.is_null() {
        // SAFETY: caller guarantees `line` is a valid NUL-terminated string.
        let s = unsafe { CStr::from_ptr(line) };
        // The C version uses `printf("%s\n", line)` which writes the bytes
        // verbatim; mirror that by writing bytes to stdout directly so we do
        // not require the input to be valid UTF-8.
        use std::io::Write;
        let stdout = std::io::stdout();
        let mut out = stdout.lock();
        let _ = out.write_all(s.to_bytes());
        let _ = out.write_all(b"\n");
    }
}

#[allow(dead_code)]
fn helper_bad() {
    let msg = b"helperBad()\0";
    printLine(msg.as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn bad() {
    let msg = b"bad()\0";
    printLine(msg.as_ptr() as *const c_char);
}

fn helper_good() {
    let msg = b"helperGood()\0";
    printLine(msg.as_ptr() as *const c_char);
}

#[no_mangle]
pub extern "C" fn good() {
    let msg = b"good()\0";
    printLine(msg.as_ptr() as *const c_char);
    helper_good();
}

/// Mirror of C `main(argc, argv)`. Exposed so the cdylib exports the same
/// symbol set as the C shared library.
///
/// Hidden from `cargo test` builds, where the test harness provides its own
/// `main`. The cdylib build (which is what the integration tests load) is
/// not affected.
#[cfg(not(test))]
#[no_mangle]
pub extern "C" fn main(_argc: c_int, _argv: *const *const c_char) -> c_int {
    let calling_good = b"Calling good()...\0";
    let finished_good = b"Finished good()\0";
    let calling_bad = b"Calling bad()...\0";
    let finished_bad = b"Finished bad()\0";
    printLine(calling_good.as_ptr() as *const c_char);
    good();
    printLine(finished_good.as_ptr() as *const c_char);
    printLine(calling_bad.as_ptr() as *const c_char);
    bad();
    printLine(finished_bad.as_ptr() as *const c_char);
    0
}
