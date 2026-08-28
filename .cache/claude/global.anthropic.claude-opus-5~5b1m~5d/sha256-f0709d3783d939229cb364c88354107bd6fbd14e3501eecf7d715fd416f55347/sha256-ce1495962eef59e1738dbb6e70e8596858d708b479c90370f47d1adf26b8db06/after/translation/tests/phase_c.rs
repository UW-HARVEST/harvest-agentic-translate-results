//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`.  The C `helloworld` contains no error
//! returns at all: it discards `printf`'s result and unconditionally returns 0.
//! The contract to verify is therefore that the Rust translation swallows I/O
//! failures in exactly the same way — same return value, same stream error
//! state, same `errno` — rather than propagating or panicking.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// E1 — fd 1 closed: write(1, ...) fails with EBADF
// ---------------------------------------------------------------------------
#[test]
fn e1_fd1_closed_write_fails() {
    let run = diff_error("E1", HostileFd::Closed, |w| {
        vec![unsafe { hello(w)() }]
    });
    assert_eq!(run.ret, vec![0], "C returns 0 even though the write failed");
    assert!(run.failed, "the stdout error indicator must be set");
    assert_eq!(run.errno, EBADF, "expected EBADF from write(1, ...)");
}

// ---------------------------------------------------------------------------
// E2 — fd 1 is a read-only descriptor
// ---------------------------------------------------------------------------
#[test]
fn e2_fd1_read_only_write_fails() {
    let run = diff_error("E2", HostileFd::ReadOnly, |w| {
        vec![unsafe { hello(w)() }]
    });
    assert_eq!(run.ret, vec![0]);
    assert!(run.failed, "writing to an O_RDONLY fd must fail");
    assert_eq!(run.errno, EBADF);
}

// ---------------------------------------------------------------------------
// E3 — fd 1 is a pipe with no reader: EPIPE (SIGPIPE ignored)
// ---------------------------------------------------------------------------
#[test]
fn e3_fd1_broken_pipe() {
    let run = diff_error("E3", HostileFd::BrokenPipe, |w| {
        vec![unsafe { hello(w)() }]
    });
    assert_eq!(run.ret, vec![0]);
    assert!(run.failed, "writing to a broken pipe must fail");
    assert_eq!(run.errno, EPIPE);
}

// ---------------------------------------------------------------------------
// E5 — fd 1 is a directory
// ---------------------------------------------------------------------------
#[test]
fn e5_fd1_is_directory() {
    let run = diff_error("E5", HostileFd::Directory, |w| {
        vec![unsafe { hello(w)() }]
    });
    assert_eq!(run.ret, vec![0]);
    assert!(run.failed, "writing to a directory fd must fail");
    assert!(
        run.errno == EBADF || run.errno == EISDIR,
        "unexpected errno {} for a directory fd",
        run.errno
    );
}

// ---------------------------------------------------------------------------
// E4 — the stdout error indicator is already set before the call
// ---------------------------------------------------------------------------

/// Observable outcome of the "already poisoned stream" scenario.
#[derive(Debug, PartialEq, Eq)]
struct Poisoned {
    ret: c_int,
    ferror_before: bool,
    ferror_after: bool,
    bytes: Vec<u8>,
}

fn run_with_poisoned_stdout(w: Which) -> Poisoned {
    let path = scratch_file("poisoned");
    let ret;
    let ferror_before;
    let ferror_after;
    {
        let swap = StdoutSwap::new();
        swap.unbuffered();
        // Break the stream so its error flag latches...
        swap.close_fd1();
        swap.poison();
        ferror_before = swap.ferror() != 0;
        // ...then give it a perfectly good file again, WITHOUT clearerr.
        let fd = open_rw(&path);
        swap.point_at(fd);

        ret = unsafe { hello(w)() };
        ferror_after = swap.ferror() != 0;

        close(fd);
        drop(swap);
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Poisoned {
        ret,
        ferror_before,
        ferror_after,
        bytes,
    }
}

#[test]
fn e4_error_flag_already_set() {
    let c = run_with_poisoned_stdout(Which::C);
    let r = run_with_poisoned_stdout(Which::Rust);
    assert!(c.ferror_before && r.ferror_before, "setup failed to poison stdout");
    assert_eq!(c.ret, r.ret, "E4: return values differ");
    assert_eq!(c.ret, 0, "E4: C ignores the pre-existing error and returns 0");
    assert_eq!(
        c.ferror_after, r.ferror_after,
        "E4: stream error state differs afterwards"
    );
    assert_eq!(
        show(&c.bytes),
        show(&r.bytes),
        "E4: emitted bytes differ on a poisoned stream"
    );
    assert_eq!(c.bytes, r.bytes);
}

// ---------------------------------------------------------------------------
// E6 — error latching across a good / bad / good sequence of calls
// ---------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Latch {
    rets: Vec<c_int>,
    ferrs: Vec<bool>,
    bytes: Vec<u8>,
}

fn run_latch_sequence(w: Which) -> Latch {
    ignore_sigpipe();
    let path = scratch_file("latch");
    let mut rets = Vec::new();
    let mut ferrs = Vec::new();
    {
        let swap = StdoutSwap::new();
        swap.unbuffered();

        // 1. good sink: the write succeeds
        let fd = open_rw(&path);
        swap.point_at(fd);
        rets.push(unsafe { hello(w)() });
        ferrs.push(swap.ferror() != 0);

        // 2. sink destroyed: the write fails
        swap.close_fd1();
        rets.push(unsafe { hello(w)() });
        ferrs.push(swap.ferror() != 0);

        // 3. good sink again, error flag still latched
        swap.point_at(fd);
        rets.push(unsafe { hello(w)() });
        ferrs.push(swap.ferror() != 0);

        close(fd);
        drop(swap);
    }
    let bytes = std::fs::read(&path).unwrap_or_default();
    let _ = std::fs::remove_file(&path);
    Latch { rets, ferrs, bytes }
}

#[test]
fn e6_error_latching_across_calls() {
    let c = run_latch_sequence(Which::C);
    let r = run_latch_sequence(Which::Rust);
    assert_eq!(c.rets, r.rets, "E6: return sequences differ");
    assert_eq!(
        c.rets,
        vec![0, 0, 0],
        "E6: every call returns 0 regardless of I/O success"
    );
    assert_eq!(c.ferrs, r.ferrs, "E6: ferror progression differs");
    assert!(!c.ferrs[0], "E6: the first (good) write must not set ferror");
    assert!(c.ferrs[1], "E6: the failing write must set ferror");
    assert_eq!(
        show(&c.bytes),
        show(&r.bytes),
        "E6: emitted bytes differ across the sequence"
    );
    assert_eq!(c.bytes, r.bytes);
}

// ---------------------------------------------------------------------------
// E7 — extra arguments through the unprototyped `int helloworld();`
//
// The zero-parameter analogue of "an enum value with no valid variant crosses
// the FFI boundary": meaningless values arrive in the argument registers (and,
// in the 8-argument form, on the stack) and must be ignored identically.
// ---------------------------------------------------------------------------
#[test]
fn e7_extra_arguments_unprototyped() {
    let extremes: [i32; 8] = [
        0,          // also the NULL a pointer parameter would have been
        -1,         // also (void *)-1
        1,
        i32::MIN,   // one step below any valid range
        i32::MAX,   // one step above any valid range
        0x5555_5555,
        -0x5555_5555,
        0x0000_00ff,
    ];

    // 5-argument form (4 ints in registers + 1 double in xmm0), happy path.
    for &v in &extremes {
        let run = diff("E7/5arg", Sink::File, Buffering::Default, |w, _| unsafe {
            hello_extra_args(w)(v, v, v, v, f64::from(v))
        });
        assert_eq!(run.value, 0, "E7: arg {v} changed the return value");
        assert_eq!(run.bytes, LINE, "E7: arg {v} changed the output");
    }

    // 8-argument form: arguments 7 and 8 are passed on the stack.
    let mut rng = Rng::new(0xE007);
    for _ in 0..32 {
        let a: [i32; 8] = std::array::from_fn(|i| {
            if rng.bool() {
                extremes[i]
            } else {
                rng.i32()
            }
        });
        let run = diff("E7/8arg", Sink::File, Buffering::Default, |w, _| unsafe {
            hello_many_args(w)(a[0], a[1], a[2], a[3], a[4], a[5], a[6], a[7])
        });
        assert_eq!(run.value, 0, "E7: args {a:?}");
        assert_eq!(run.bytes, LINE, "E7: args {a:?}");
    }

    // And the same abuse while stdout is unusable: still 0, still no panic.
    for &v in &extremes {
        let run = diff_error("E7/hostile", HostileFd::Closed, |w| {
            vec![unsafe { hello_extra_args(w)(v, v, v, v, f64::from(v)) }]
        });
        assert_eq!(run.ret, vec![0], "E7 hostile: arg {v}");
        assert!(run.failed);
        assert_eq!(run.errno, EBADF);
    }
}

// ---------------------------------------------------------------------------
// E8 — variadic call signature (sets `al` to the SSE-register count)
// ---------------------------------------------------------------------------
#[test]
fn e8_variadic_call_signature() {
    let mut rng = Rng::new(0xE008);
    for i in 0..32 {
        let a = if i == 0 { 0 } else { rng.i32() };
        let b = if i == 1 { -1 } else { rng.i32() };
        let run = diff("E8", Sink::File, Buffering::Default, |w, _| unsafe {
            hello_variadic(w)(a, b, 3.5f64, c"x".as_ptr())
        });
        assert_eq!(run.value, 0, "E8: variadic call returned non-zero");
        assert_eq!(run.bytes, LINE, "E8: variadic call changed the output");
    }

    // Variadic form against a broken stdout too.
    let run = diff_error("E8/hostile", HostileFd::BrokenPipe, |w| {
        vec![unsafe { hello_variadic(w)(i32::MIN, i32::MAX, 0.0f64) }]
    });
    assert_eq!(run.ret, vec![0]);
    assert!(run.failed);
    assert_eq!(run.errno, EPIPE);
}

// ---------------------------------------------------------------------------
// Generic C-API boundaries, documented as structurally inapplicable
// (see the applicability table in ERRORS.md).
// ---------------------------------------------------------------------------
#[test]
fn generic_boundaries_have_no_applicable_surface() {
    // `int helloworld();` — zero parameters, so there is no pointer to null out,
    // no length to zero or oversize, no range to step past and no enum to
    // corrupt.  This test locks that fact in: if the C header ever grows a
    // parameter, the assertion below becomes the reminder to extend ERRORS.md.
    let header = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/include/hello.h"),
    )
    .expect("read c_src/include/hello.h");

    let decls: Vec<&str> = header
        .lines()
        .map(str::trim)
        .filter(|l| l.ends_with(';') && !l.starts_with("//"))
        .collect();
    assert_eq!(
        decls,
        vec!["int helloworld();"],
        "the public header changed — re-derive SYMBOLS.md / ERRORS.md / CONFIGS.md"
    );

    // The one entry point still behaves on the happy path.
    let run = diff("generic", Sink::File, Buffering::Default, |w, _| unsafe {
        hello(w)()
    });
    assert_eq!(run.value, 0);
    assert_eq!(run.bytes, LINE);
}
