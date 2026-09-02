//! Phase C — error/rejection-path differential tests, one per row of `ERRORS.md`.
//!
//! `tool_basename` has no error return, no NULL check, no length bound and no
//! enum parameter (see `ERRORS.md` for the mechanical derivation). The only
//! rejection-shaped behaviour is the undefined behaviour that follows from
//! violating its implicit precondition, so each row is tested by running the
//! call in an isolated child process and asserting that BOTH libraries die with
//! the SAME fatal signal — not merely that "both failed somehow".

mod common;

use common::libs;
use std::ffi::c_char;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

/// Env var that turns this test binary into a one-shot crash victim.
const VICTIM_VAR: &str = "TOOL_BASENAME_VICTIM";

#[derive(Debug, PartialEq, Eq)]
enum Death {
    Signal(i32),
    Exit(i32),
}

/// Re-exec this test binary as a child that performs `case` and report how it died.
fn run_victim(case: &str) -> Death {
    let exe = std::env::current_exe().expect("current_exe");
    // Hand the child the parent's already-built, immutable library snapshot so it
    // does no build of its own and cannot pick up a different artifact.
    let (_, rust_so) = common::loaded_paths();
    let status = Command::new(exe)
        // Run only the trampoline test; it dispatches on VICTIM_VAR.
        .args([
            "--exact",
            "victim_trampoline",
            "--ignored",
            "--test-threads=1",
        ])
        .env(VICTIM_VAR, case)
        .env("TB_RUST_SO", &rust_so)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn victim");

    match status.signal() {
        Some(sig) => Death::Signal(sig),
        None => Death::Exit(status.code().unwrap_or(-1)),
    }
}

/// Not a real test: the entry point used by `run_victim`. Ignored by default so
/// a normal `cargo test` run never executes it.
#[test]
#[ignore = "internal crash-victim trampoline; driven by run_victim()"]
fn victim_trampoline() {
    let case = match std::env::var(VICTIM_VAR) {
        Ok(c) => c,
        Err(_) => return,
    };
    let l = libs();
    let f = match case.as_str() {
        "null_c" | "unterm_c" => l.c_basename,
        "null_rust" | "unterm_rust" => l.rust_basename,
        other => panic!("unknown victim case {other}"),
    };

    if case.starts_with("null") {
        // ERRORS.md row 1: NULL path, no NULL check in the C.
        let ret = unsafe { f(std::ptr::null_mut()) };
        // Reaching here means the library "handled" NULL instead of faulting.
        // Exit code 42 marks that, so the comparison shows which side differs.
        eprintln!("survived NULL, returned {ret:?}");
        std::process::exit(42);
    } else {
        // ERRORS.md row 2: buffer with no NUL terminator, followed by a guard
        // page, so the unbounded scan must fault deterministically.
        let page = unsafe { libc::sysconf(libc::_SC_PAGESIZE) } as usize;
        let total = page * 2;
        let base = unsafe {
            libc::mmap(
                std::ptr::null_mut(),
                total,
                libc::PROT_READ | libc::PROT_WRITE,
                libc::MAP_PRIVATE | libc::MAP_ANONYMOUS,
                -1,
                0,
            )
        };
        assert_ne!(base, libc::MAP_FAILED, "mmap failed");
        unsafe {
            // First page: all non-NUL, no terminator anywhere.
            std::ptr::write_bytes(base as *mut u8, b'a', page);
            // Second page: unreadable guard.
            assert_eq!(
                libc::mprotect(base.add(page), page, libc::PROT_NONE),
                0,
                "mprotect failed"
            );
        }
        let ret = unsafe { f(base as *mut c_char) };
        eprintln!("survived unterminated buffer, returned {ret:?}");
        std::process::exit(43);
    }
}

// ------------------------------------------------------------- ERRORS row 1
#[test]
fn err_row1_null_pointer_same_fatal_signal() {
    let c = run_victim("null_c");
    let r = run_victim("null_rust");
    assert_eq!(
        c, r,
        "ERRORS row 1 divergence: C died as {c:?} but Rust died as {r:?} on a NULL path"
    );
    assert_eq!(
        c,
        Death::Signal(libc::SIGSEGV),
        "expected SIGSEGV from the unchecked NULL dereference, got {c:?}"
    );
}

// ------------------------------------------------------------- ERRORS row 2
#[test]
fn err_row2_unterminated_buffer_same_fatal_signal() {
    let c = run_victim("unterm_c");
    let r = run_victim("unterm_rust");
    assert_eq!(
        c, r,
        "ERRORS row 2 divergence: C died as {c:?} but Rust died as {r:?} on an unterminated buffer"
    );
    assert_eq!(
        c,
        Death::Signal(libc::SIGSEGV),
        "expected SIGSEGV from the unbounded scan past the buffer, got {c:?}"
    );
}

// ------------------------------- generic FFI boundaries (not distinct C rows)
/// Zero length: accepted, not rejected. Must return the same pointer.
#[test]
fn boundary_zero_length_is_accepted_identically() {
    let l = libs();
    let c = common::call(l.c_basename, b"");
    let r = common::call(l.rust_basename, b"");
    assert_eq!(c, r);
    assert_eq!(c.offset, Some(0), "empty string must return path unchanged");
    assert!(c.result.is_empty());
}

/// One step past the separator byte values — an off-by-one in the comparison
/// would show up here as a divergence.
#[test]
fn boundary_bytes_adjacent_to_separators() {
    for b in [0x2Eu8, 0x2F, 0x30, 0x5B, 0x5C, 0x5D] {
        common::assert_same(&[b], "single byte adjacent to separators");
        common::assert_same(&[b'x', b, b'y'], "interior byte adjacent to separators");
        common::assert_same(&[b, b, b], "run of byte adjacent to separators");
    }
}

/// The API takes no enum/int mode, so there is no invalid-variant value to pass.
/// The nearest analogue is an arbitrary out-of-range *byte* value in the data,
/// including every value the signed `c_char` representation can take.
#[test]
fn boundary_every_byte_value_in_every_position_of_a_short_string() {
    // Exhaustive over all 255 non-NUL byte values x 3 positions in a 3-byte string,
    // with the other two positions cycling through both separators and a plain byte.
    const FILLERS: [u8; 3] = *b"/\\x";
    for b in 1u8..=255 {
        for pos in 0..3usize {
            for &f0 in &FILLERS {
                for &f1 in &FILLERS {
                    let mut s = [f0, f1, f0];
                    s[pos] = b;
                    common::assert_same(&s, "exhaustive byte x position");
                }
            }
        }
    }
}

/// Oversized length: no bound exists, so a very long input is accepted.
#[test]
fn boundary_oversized_length_is_accepted() {
    let mut s = vec![b'z'; 4 * 1024 * 1024];
    s[0] = b'/';
    s[2 * 1024 * 1024] = b'\\';
    common::assert_same(&s, "4 MiB input");
}
