//! Differential test for the **side effects** of the exported functions.
//!
//! `CONFIGS.md` axis 5: three of the six exported functions write to `stdout`
//! (`mdcore.c`):
//!
//! ```c
//! printf("helper.call=%d helper.acc=%d\n", r, acc);   // helper_call
//! printf("helper.ptr=%d\n", r);                        // helper_ptr
//! printf("gen.acc=%d\n", r);                           // use_generated
//! ```
//!
//! The other differential tests compare only the `int` return values, so a
//! divergence in a format string (a missing space, `helper.acc` vs `helper_acc`,
//! a wrong argument order) would go unnoticed at the `.so` level. `driver_cli.rs`
//! compares the *executable*'s stdout, but the executable is built from
//! `src/main.rs`, which recompiles the modules rather than loading the `cdylib`
//! — so this file is what pins the `.so`'s own output.
//!
//! Capturing it requires redirecting the process-wide fd 1, which is why this is
//! a separate integration-test binary (its own process) containing exactly ONE
//! `#[test]`: libtest itself writes its progress text to fd 1, so no other test
//! may be running — nor may libtest have anything buffered — while fd 1 points at
//! the capture file.

mod common;

use std::ffi::{c_int, c_void};
use std::fs;
use std::io::Write;
use std::os::fd::AsRawFd;

use common::{load_pair, Api, Rng, CORNERS, N_SHAPES, OP_NAME, REPEAT, SEED};

extern "C" {
    fn dup(oldfd: c_int) -> c_int;
    fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
    fn close(fd: c_int) -> c_int;
    /// `fflush(NULL)` flushes *all* C streams — required because the C library's
    /// `stdout` is fully buffered when fd 1 is a regular file.
    fn fflush(stream: *mut c_void) -> c_int;
}

/// Run `f` with fd 1 redirected to a fresh file, and return what was written.
///
/// Both buffers in front of fd 1 must be drained *before* the redirect, or their
/// contents would land in the capture file:
///
/// * Rust's `io::stdout()` is a `LineWriter`, and libtest's `"test <name> ... "`
///   progress text has no trailing newline, so it is still buffered when the test
///   body starts.
/// * the C library's `stdout` is *fully* buffered whenever fd 1 is not a terminal
///   (which is the case under `cargo test`), so anything the C `.so` printed
///   earlier — e.g. during `load_pair`'s staleness check — is still pending.
fn capture(tag: &str, f: impl FnOnce()) -> Vec<u8> {
    let _ = std::io::stdout().flush();
    // SAFETY: `fflush(NULL)` flushes all open C streams; no arguments to get
    // wrong and no pointers dereferenced by us.
    unsafe { fflush(std::ptr::null_mut()) };
    let path = common::target_profile_dir().join(format!("stdout_capture_{tag}.txt"));
    let file = fs::File::create(&path).expect("create capture file");

    // SAFETY: plain POSIX fd juggling. `saved` is closed again below, and fd 1
    // is restored before the guard is released, so no other code observes the
    // redirected descriptor.
    unsafe {
        let saved = dup(1);
        assert!(saved >= 0, "dup(1) failed");
        assert!(dup2(file.as_raw_fd(), 1) >= 0, "dup2 failed");

        f();

        // Flush the C library's buffered stdout, then Rust's (the cdylib's std
        // is line-buffered, so its complete lines are already out, but be
        // explicit), before putting fd 1 back.
        fflush(std::ptr::null_mut());
        assert!(dup2(saved, 1) >= 0, "restoring fd 1 failed");
        close(saved);
    }
    drop(file);
    fs::read(&path).expect("read capture file")
}

/// Capture the stdout produced by one call to each printing entry point.
fn printed(api: &Api, tag: &str, a: c_int, b: c_int, n: c_int) -> Vec<u8> {
    capture(tag, || {
        // SAFETY: signatures match the C prototypes in mdmacros.h.
        unsafe {
            (api.helper_call)(a, b);
            (api.helper_ptr)(a, b);
            (api.use_generated)(n);
        }
    })
}

/// The C and Rust `.so`s must write byte-identical text for the same inputs, and
/// that text must have the exact shape of the three `printf` format strings.
///
/// Both checks live in one `#[test]` deliberately: see the module comment.
#[test]
fn so_stdout_matches_byte_for_byte_and_has_the_expected_format() {
    let (c, r) = load_pair();

    check_expected_format(&c, &r);

    let mut inputs: Vec<(c_int, c_int, c_int)> = Vec::new();
    for (i, &a) in CORNERS.iter().enumerate() {
        for (j, &b) in CORNERS.iter().enumerate() {
            inputs.push((a, b, N_SHAPES[(i * CORNERS.len() + j) % N_SHAPES.len()]));
        }
    }
    let mut rng = Rng::new(SEED ^ 0x5000);
    for _ in 0..400 {
        inputs.push((
            rng.next_i32_biased(),
            rng.next_i32_biased(),
            rng.next_i32(),
        ));
    }
    // Every `switch` arm and both of its boundaries, explicitly.
    for n in -1..=8 {
        inputs.push((11, 7, n));
    }

    for (a, b, n) in inputs {
        let cv = printed(&c, "c", a, b, n);
        let rv = printed(&r, "rust", a, b, n);
        assert_eq!(
            String::from_utf8_lossy(&cv),
            String::from_utf8_lossy(&rv),
            ".so stdout differs for (a={a}, b={b}, n={n}) [OP={OP_NAME} REPEAT={REPEAT}]"
        );
        assert_eq!(cv, rv, ".so stdout bytes differ for (a={a}, b={b}, n={n})");
    }
}

/// Pin the exact shape of the three format strings, so that a *mutual* change
/// (both sides edited the same wrong way) still gets caught.
fn check_expected_format(c: &Api, r: &Api) {
    for (api, tag) in [(c, "c_fmt"), (r, "rust_fmt")] {
        let out = String::from_utf8(printed(api, tag, 10, 3, 4)).expect("utf-8");
        let lines: Vec<&str> = out.lines().collect();
        assert_eq!(lines.len(), 3, "{}: expected 3 lines, got {out:?}", api.tag);

        let op = match OP_NAME {
            "mul" => 10i32.wrapping_mul(3),
            "sub" => 10i32.wrapping_sub(3),
            _ => 10i32.wrapping_add(3),
        };
        let init: c_int = if OP_NAME == "mul" { 1 } else { 0 };
        let step = |acc: c_int, i: c_int| match OP_NAME {
            "mul" => acc.wrapping_mul(i.wrapping_add(1)),
            "sub" => acc.wrapping_sub(i),
            _ => acc.wrapping_add(i),
        };
        let mut acc = init;
        for i in 0..REPEAT {
            acc = step(acc, i);
        }
        let mut gen = init;
        for i in 0..4 {
            gen = step(gen, i);
        }

        assert_eq!(lines[0], format!("helper.call={op} helper.acc={acc}"), "{}", api.tag);
        assert_eq!(lines[1], format!("helper.ptr={op}"), "{}", api.tag);
        assert_eq!(lines[2], format!("gen.acc={gen}"), "{}", api.tag);
        assert!(out.ends_with('\n'), "{}: output must end with a newline", api.tag);
    }
}
