//! Edge cases where the C code has undefined behaviour. They are kept in a
//! separate test binary so that a hypothetical abort here cannot mask results
//! from the other suites.
//!
//! The C implementation is the reference, so these inputs are verified to match
//! rather than "fixed":
//!
//!  1. A line that is exactly `Integrity checksum changed for: '` leaves an
//!     empty payload, and `al_data->filename[strlen(...) - 1] = '\0'` then
//!     stores one byte *before* the 1-byte allocation.
//!  2. A `** Alert` line with no trailing newline is only 8 bytes, so
//!     `p = str + ALERT_BEGIN_SZ + 1` points past the terminating NUL into
//!     whatever the stack buffer happened to hold.

mod common;

use common::*;
use std::os::raw::c_int;

fn compare(tag: &str, content: &[u8], flag: c_int) {
    let p = pair();
    let dir = TempDir::new("ub");
    let path = dir.file("alerts.log", content);
    let _g = lock();

    let mut results = Vec::new();
    for imp in [&p.c, &p.rs] {
        unsafe {
            let fp = fopen(&path, b"r");
            *libc::__errno_location() = 0;
            let al = (imp.GetAlertData)(flag, fp);
            let snap = snap_alert(al);
            let stream = snap_stream(fp);
            if !al.is_null() {
                (imp.FreeAlertData)(al);
            }
            libc::fclose(fp);
            results.push((snap, stream));
        }
    }
    assert_eq!(
        results[0], results[1],
        "[{tag} flag={flag:#x}] differs\nC:    {:#?}\nRust: {:#?}",
        results[0], results[1]
    );
}

#[test]
fn integrity_line_with_empty_payload() {
    // filename becomes "" and the store lands one byte before the allocation.
    let content: &[u8] = b"** Alert 1: mail - syscheck,\n\
2025 Aug 21 13:27:04 loc\n\
Integrity checksum changed for: '\n";
    for flag in [0, CRALERT_MAIL_SET, CRALERT_READ_ALL] {
        compare("empty-filename", content, flag);
    }

    // Same thing without the trailing newline on the integrity line.
    let content2: &[u8] = b"** Alert 1: mail - syscheck,\n\
2025 Aug 21 13:27:04 loc\n\
Integrity checksum changed for: '";
    for flag in [0, CRALERT_MAIL_SET] {
        compare("empty-filename-no-nl", content2, flag);
    }
}

#[test]
fn bare_alert_header_without_newline() {
    // First read of the call: the bytes at str+9 are indeterminate in C.
    compare("bare-header-first", b"** Alert", 0);
    // After a longer line has primed the buffer the leftover bytes are the same
    // for both implementations, so this variant is well defined.
    compare(
        "bare-header-primed",
        b"padding line long enough to prime the buffer\n** Alert",
        0,
    );
    compare(
        "bare-header-primed-2",
        b"** Alert 1: mail - g,\n2025 Aug 21 13:27:04 loc\n** Alert",
        0,
    );
}

#[test]
fn short_header_lines() {
    // These are *not* UB: fgets writes the NUL at index >= 9, so str+9 is in
    // range. Kept here next to their UB sibling for contrast.
    compare("header-with-newline", b"** Alert\n", 0);
    compare("header-plus-one", b"** Alertx", 0);
    compare("header-plus-one-nl", b"** Alertx\n", 0);
    compare("header-nine-nl", b"** Alert \n", 0);
}
