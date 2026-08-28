//! Exercises the write/close error branches of `write_to_file`.
//!
//! The C code returns `errno` *after* running `fprintf(stderr, ...)` and, in the
//! write-failure branch, after `fclose(file)` as well, so the value returned is
//! whatever those calls leave in `errno`.

mod common;

use common::*;

fn probe(label: &str, path: &str, content: &str) {
    let p = pair();
    let _g = fs_lock();
    let name = cstr(path);
    let payload = cstr(content);
    unsafe {
        let crc = (p.c.write_to_file)(name.as_ptr(), payload.as_ptr());
        let rrc = (p.rs.write_to_file)(name.as_ptr(), payload.as_ptr());
        assert_eq!(
            crc, rrc,
            "{label}: write_to_file({path:?}, {} bytes) returned c={crc} rust={rrc}",
            content.len()
        );
    }
}

#[test]
fn dev_full_close_failure() {
    // Small payload: fprintf buffers it, so the ENOSPC surfaces from fclose.
    probe("small", "/dev/full", "data");
    probe("newline", "/dev/full", "1 2\n3 4\n");
}

#[test]
fn dev_full_write_failure() {
    // Large payload: the stdio buffer flushes inside fprintf/fputs, so the
    // write branch is taken and `fclose` runs before `errno` is read.
    probe("64k", "/dev/full", &"x".repeat(64 * 1024));
    probe("1m", "/dev/full", &"y".repeat(1024 * 1024));
}

#[test]
fn dev_null_and_special_targets() {
    probe("devnull", "/dev/null", "anything\n");
    probe("devnull-empty", "/dev/null", "");
}
