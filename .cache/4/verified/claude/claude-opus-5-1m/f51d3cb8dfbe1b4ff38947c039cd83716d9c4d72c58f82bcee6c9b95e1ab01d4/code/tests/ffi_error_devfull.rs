//! ERRORS.md row **E3** at the FFI level — `ENOSPC` from `/dev/full`.
//!
//! The C code discards `printf`'s return value, so a full device must not change
//! the value `main` returns. Isolated in its own test binary for the same reason
//! as `ffi_error_epipe.rs`: the failed write latches sticky state inside each
//! runtime's buffered stdout.

mod common;

use common::*;

#[test]
fn e3_dev_full_ffi() {
    let ret = assert_same_so("fd1=/dev/full (ENOSPC)", |f| {
        let fd = open_fd("/dev/full", O_WRONLY, 0);
        assert!(fd >= 0, "open /dev/full failed");
        let ret = with_fd1(fd, || unsafe { f() });
        unsafe { close(fd) };
        ret
    });

    assert_eq!(
        ret, 0,
        "write errors are discarded by the C code, so main must still return 0"
    );
}
