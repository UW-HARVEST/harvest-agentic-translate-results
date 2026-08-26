//! Phase B (valid paths) at the FFI boundary: both shared libraries are loaded
//! with `libloading` and driven through their exported `main` symbol.
//!
//! Covers CONFIGS.md rows C16–C23 and the ERRORS.md generic row `g1`.
//!
//! Failing-write cases (E2 `/dev/full`, EPIPE) deliberately live in their own
//! test binaries — `ffi_error_epipe.rs` and `ffi_error_devfull.rs` — because a
//! failed write latches sticky state inside each runtime's buffered `stdout`
//! (glibc's error indicator, Rust's `LineWriter` buffer) that would otherwise
//! leak into unrelated tests sharing this process.

mod common;

use common::*;
use std::io::Read;
use std::os::fd::FromRawFd;

/// C22 — both libraries load, and each yields a distinct, non-null `main`.
///
/// Also proves `libloading` resolves `main` against the specific library handle
/// rather than picking up the test harness's own entry point.
#[test]
fn c_and_rust_so_export_only_main() {
    let c = c_main_sym();
    let r = rust_main_sym();
    let (ca, ra) = (c as usize, r as usize);
    assert_ne!(ca, 0, "C main symbol is null");
    assert_ne!(ra, 0, "Rust main symbol is null");
    assert_ne!(
        ca, ra,
        "C and Rust `main` resolved to the same address - the libraries are not \
         being loaded independently"
    );
}

/// C16 — one call, fd 1 redirected to a file. Compare returned `int` and bytes.
#[test]
fn b14_ffi_single_call() {
    let (ret, bytes) = assert_same_so("single .so call, fd1=file", |f| {
        capture_fd1("single", || unsafe { f() })
    });
    assert_eq!(ret, 0, "C returns 0 from main; Rust must too");
    assert_eq!(
        bytes, EXPECTED,
        "expected exactly the C program's bytes, got {:?}",
        String::from_utf8_lossy(&bytes)
    );
}

/// C17 + C18 — many sequential calls, then C/Rust calls interleaved in a
/// randomized order on the *same* fd.
#[test]
fn b_repeated_and_interleaved() {
    // C17: randomized call counts; N calls must emit exactly N identical lines.
    let mut rng = Rng::new();
    for _ in 0..12 {
        let n = rng.range(1, 100);
        let (rets, bytes) = assert_same_so(&format!("{n} sequential .so calls"), |f| {
            capture_fd1("repeat", || {
                (0..n).map(|_| unsafe { f() }).collect::<Vec<_>>()
            })
        });
        assert!(rets.iter().all(|&r| r == 0), "every call must return 0");
        assert_eq!(
            bytes.len(),
            EXPECTED.len() * n,
            "{n} calls should emit {n} lines"
        );
        assert_eq!(bytes, EXPECTED.repeat(n), "concatenation mismatch for n={n}");
    }

    // C18: interleave C and Rust arbitrarily. Each call is flushed immediately so
    // the byte order is well defined across the two runtimes; the point is that a
    // C call and a Rust call are individually indistinguishable in the stream.
    let c = c_main_sym();
    let r = rust_main_sym();
    for iter in 0..8 {
        let n = rng.range(2, 40);
        let order: Vec<bool> = (0..n).map(|_| rng.next_u64() & 1 == 0).collect();
        let (rets, bytes) = capture_fd1("interleave", || {
            order
                .iter()
                .map(|&use_c| {
                    let v = unsafe { if use_c { c() } else { r() } };
                    // Make ordering deterministic across the two runtimes.
                    fflush_all();
                    v
                })
                .collect::<Vec<_>>()
        });
        assert!(
            rets.iter().all(|&v| v == 0),
            "iter {iter}: all interleaved calls must return 0, got {rets:?}"
        );
        assert_eq!(
            bytes,
            EXPECTED.repeat(n),
            "iter {iter}: interleaved C/Rust output diverged (order={order:?})"
        );
    }
}

/// C19 — fd 1 is a pipe rather than a file (a different buffering class).
#[test]
fn b15_ffi_pipe() {
    let (ret, bytes) = assert_same_so("single .so call, fd1=pipe", |f| {
        let (rfd, wfd) = make_pipe();
        let ret = with_fd1(wfd, || unsafe { f() });
        // Drop the last writer so the read side sees EOF.
        unsafe { close(wfd) };
        let mut buf = Vec::new();
        let mut rd = unsafe { std::fs::File::from_raw_fd(rfd) };
        rd.read_to_end(&mut buf).expect("read from pipe");
        (ret, buf)
    });
    assert_eq!(ret, 0);
    assert_eq!(bytes, EXPECTED);
}

/// C20 — fd 1 is `/dev/null`: nothing observable, must still return 0.
#[test]
fn b16_ffi_devnull() {
    let ret = assert_same_so("single .so call, fd1=/dev/null", |f| {
        let fd = open_fd("/dev/null", O_WRONLY, 0);
        assert!(fd >= 0, "open /dev/null failed");
        let ret = with_fd1(fd, || unsafe { f() });
        unsafe { close(fd) };
        ret
    });
    assert_eq!(ret, 0);
}

/// C21 / g1 — `int main(void)` must ignore whatever is in the argument
/// registers. Calling it through a six-argument declaration is the closest
/// analogue to "pass a value with no valid meaning across the FFI boundary"
/// for an API that has no pointer, length, or enum parameters at all.
#[test]
fn g1_junk_arguments_ignored() {
    let mut rng = Rng::new();
    for _ in 0..16 {
        let junk = [
            rng.next_u64(),
            rng.next_u64(),
            u64::MAX,
            0,
            rng.next_u64(),
            rng.next_u64(),
        ];
        let (ret, bytes) = assert_same_so("main() called with junk arg registers", |f| {
            let g: MainFnJunk = unsafe { std::mem::transmute::<MainFn, MainFnJunk>(f) };
            capture_fd1("junk", || unsafe {
                g(junk[0], junk[1], junk[2], junk[3], junk[4], junk[5])
            })
        });
        assert_eq!(ret, 0, "junk args must not change the return value");
        assert_eq!(bytes, EXPECTED, "junk args must not change the output");
    }
}

/// C23 — the executable and the shared library must agree, on both sides.
#[test]
fn b17_exe_so_cross_check() {
    // Executable stdout.
    let exe_bytes = assert_same_exe("exe stdout for cross-check", |exe| {
        let out = std::process::Command::new(exe)
            .output()
            .expect("spawn driver");
        Outcome::from_output(out)
    });
    assert_eq!(exe_bytes.stdout, EXPECTED);
    assert_eq!(exe_bytes.code, Some(0));
    assert!(exe_bytes.stderr.is_empty());

    // Shared-library single call.
    let (ret, so_bytes) = assert_same_so("so stdout for cross-check", |f| {
        capture_fd1("cross", || unsafe { f() })
    });
    assert_eq!(ret, 0);

    assert_eq!(
        exe_bytes.stdout, so_bytes,
        "executable and shared library must produce identical bytes"
    );
}
