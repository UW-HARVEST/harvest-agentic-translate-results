//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C library performs no argument
//! validation at all (`driver` is `printf("%zu\n", strcspn(s1, s2))` and returns
//! `void`), so its entire rejection surface is the set of pointer preconditions
//! `strcspn` imposes. Violating one terminates the process, which means these
//! cases cannot be run in the test process itself.
//!
//! Each row therefore re-executes this same test binary as a child, selecting
//! `child_worker` and telling it which library to load and which invalid case to
//! construct. The parent compares the child's *full* termination status — exit
//! code and terminating signal — plus its stdout, between the C `.so` and the
//! Rust `.so`. Asserting on the signal (not merely "it failed") is what makes
//! this an equality check rather than a both-broke-somehow check.

mod common;

use std::ffi::c_char;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

use common::*;

const ENV_CASE: &str = "DRIVER_DIFFTEST_CASE";
const ENV_LIB: &str = "DRIVER_DIFFTEST_LIB";
const ENV_OUT: &str = "DRIVER_DIFFTEST_OUT";

/// How a child process ended: `(exit code, terminating signal, driver output)`.
///
/// The child's output is collected through a file the child points fd 1 at, not
/// through the inherited pipe: the libtest harness writes its own preamble
/// ("running 1 test") to stdout before `child_worker` starts, and that noise is
/// not part of what the library under test produced.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    code: Option<i32>,
    signal: Option<i32>,
    stdout: Vec<u8>,
}

fn run_child(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let out_path = std::env::temp_dir().join(format!(
        "driver-difftest-child-{}-{case}-{which}.out",
        std::process::id()
    ));
    let _ = std::fs::remove_file(&out_path);
    let status = Command::new(exe)
        .args(["--exact", "child_worker", "--test-threads=1", "-q"])
        .env(ENV_CASE, case)
        .env(ENV_LIB, which)
        .env(ENV_OUT, &out_path)
        // Pass the resolved paths down so the child does not have to re-derive
        // them from a possibly different environment.
        .env("C_DRIVER_SO", c_so_path())
        .env("RUST_DRIVER_SO", rust_so_path())
        .stdin(Stdio::null())
        .stderr(Stdio::null())
        .stdout(Stdio::null())
        .status()
        .expect("spawn child");
    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    Outcome {
        code: status.code(),
        signal: status.signal(),
        stdout,
    }
}

/// Run one `ERRORS.md` row against both libraries and require identical
/// termination behaviour.
fn assert_same_failure(case: &str) {
    let c = run_child(case, "c");
    let rust = run_child(case, "rust");
    assert_eq!(
        c, rust,
        "{case}: C and Rust disagreed on how the invalid input is rejected\n  \
         C   : {c:?}\n  Rust: {rust:?}"
    );
    // Every row in ERRORS.md is expected to be fatal; make that explicit so a
    // future change that turns a fault into a silent success cannot pass by
    // "both agree".
    assert_eq!(
        c.signal,
        Some(11),
        "{case}: expected SIGSEGV from the C library, got {c:?}"
    );
    assert!(
        c.stdout.is_empty(),
        "{case}: expected no output before the fault, got {:?}",
        String::from_utf8_lossy(&c.stdout)
    );
}

// ---------------------------------------------------------------------------
// The child side
// ---------------------------------------------------------------------------

/// When `DRIVER_DIFFTEST_CASE` is set this builds the requested invalid input
/// and calls `driver`, which is expected to terminate the process. Without the
/// variable it is an inert no-op, so a normal `cargo test` run is unaffected.
#[test]
fn child_worker() {
    let Ok(case) = std::env::var(ENV_CASE) else {
        return;
    };
    let which = std::env::var(ENV_LIB).expect("DRIVER_DIFFTEST_LIB");

    // Point fd 1 at the parent's collection file so only the library's own
    // output is recorded, and so the fault loses whatever it had buffered —
    // exactly as it does in the C library.
    {
        use std::io::Write;
        use std::os::fd::AsRawFd;
        // Drain anything libtest already buffered (its "running 1 test" preamble)
        // to the *original* fd 1 before we take it over.
        std::io::stdout().flush().ok();
        flush_all_streams();

        let path = std::env::var(ENV_OUT).expect("DRIVER_DIFFTEST_OUT");
        let f = std::fs::File::create(&path).expect("create child output file");
        // SAFETY: fd 1 is open and `f`'s descriptor is valid.
        let rc = unsafe { redirect_fd1(f.as_raw_fd()) };
        assert!(rc >= 0, "child dup2 onto fd 1 failed");
        std::mem::forget(f);
    }

    let lib = load_one(&which);

    let null = std::ptr::null::<c_char>();

    // Buffers are held in locals for the duration of the call; the process is
    // expected not to return from `call`, and `exit` below covers the case where
    // it does.
    let held: Vec<GuardedBuf>;
    let (s1, s2): (*const c_char, *const c_char) = match case.as_str() {
        // --- s2 invalid, s1 valid -------------------------------------------
        "E1" => {
            held = vec![];
            (c"abc".as_ptr(), null)
        }
        "E2" => {
            held = vec![];
            (c"".as_ptr(), null)
        }
        "E3" => {
            held = vec![GuardedBuf::unmapped()];
            (c"".as_ptr(), held[0].ptr())
        }
        "E4" => {
            held = vec![GuardedBuf::unmapped()];
            (c"abcdef".as_ptr(), held[0].ptr())
        }
        // --- s1 invalid, s2 valid -------------------------------------------
        "E5" => {
            held = vec![];
            (null, c"abc".as_ptr())
        }
        "E6" => {
            held = vec![];
            (null, c"a".as_ptr())
        }
        "E7" => {
            held = vec![];
            (null, c"".as_ptr())
        }
        "E8" => {
            held = vec![];
            (null, null)
        }
        "E9" => {
            held = vec![GuardedBuf::unmapped()];
            (held[0].ptr(), c"abc".as_ptr())
        }
        "E10" => {
            held = vec![GuardedBuf::unmapped()];
            (held[0].ptr(), c"".as_ptr())
        }
        // --- unterminated strings -------------------------------------------
        // s2 has no NUL and runs into the guard page, and s1[0] is already a
        // member of the reject set: an implementation that stopped scanning s2
        // as soon as it found the match would return 0 instead of faulting.
        "E11" => {
            held = vec![GuardedBuf::unterminated(b"ab")];
            (c"aXYZ".as_ptr(), held[0].ptr())
        }
        // s1 has no NUL and none of its bytes are in the reject set, so the
        // scan runs off the end of the readable page.
        "E12" => {
            held = vec![GuardedBuf::unterminated(&[b'X'; 64])];
            (held[0].ptr(), c"a".as_ptr())
        }
        // Same, with the empty reject set (the `strlen` degeneration).
        "E13" => {
            held = vec![GuardedBuf::unterminated(&[b'X'; 64])];
            (held[0].ptr(), c"".as_ptr())
        }
        other => panic!("unknown case {other:?}"),
    };

    // SAFETY: deliberately violating `strcspn`'s preconditions — that is the
    // point of the test. The process is expected to die here; whatever the C
    // library does, the Rust library must do too.
    unsafe { lib.call(s1, s2) };

    // Reached only if the call did *not* fault. Flush so the parent sees any
    // output, then exit with a distinctive code so the parent's comparison
    // reports "no fault" rather than a confusing signal mismatch.
    drop(held);
    flush_all_streams();
    std::process::exit(0);
}

// ---------------------------------------------------------------------------
// One test per ERRORS.md row
// ---------------------------------------------------------------------------

#[test]
fn e1_s2_null_s1_nonempty() {
    assert_same_failure("E1");
}

/// The interesting one: an empty `s1` does **not** rescue a NULL `s2`, because
/// glibc's `strcspn` reads the reject set before it looks at `s1` at all.
#[test]
fn e2_s2_null_s1_empty() {
    assert_same_failure("E2");
}

#[test]
fn e3_s2_unmapped_s1_empty() {
    assert_same_failure("E3");
}

#[test]
fn e4_s2_unmapped_s1_nonempty() {
    assert_same_failure("E4");
}

#[test]
fn e5_s1_null_s2_multi() {
    assert_same_failure("E5");
}

#[test]
fn e6_s1_null_s2_single() {
    assert_same_failure("E6");
}

#[test]
fn e7_s1_null_s2_empty() {
    assert_same_failure("E7");
}

#[test]
fn e8_both_null() {
    assert_same_failure("E8");
}

#[test]
fn e9_s1_unmapped_s2_nonempty() {
    assert_same_failure("E9");
}

#[test]
fn e10_s1_unmapped_s2_empty() {
    assert_same_failure("E10");
}

/// The second interesting one: the whole reject set is consumed before `s1` is
/// scanned, so a match at `s1[0]` does not prevent the fault.
#[test]
fn e11_s2_unterminated_match_at_zero() {
    assert_same_failure("E11");
}

#[test]
fn e12_s1_unterminated() {
    assert_same_failure("E12");
}

#[test]
fn e13_s1_unterminated_empty_reject() {
    assert_same_failure("E13");
}

/// Boundary conditions that every C API has, checked even though the C source
/// contains no corresponding explicit check.
///
/// `driver`'s signature is `void driver(const char *, const char *)`: there is no
/// integer length to pass as zero or oversized, and — importantly — **no enum
/// parameter**, so there is no out-of-range enum value that could be smuggled
/// across the FFI boundary. What remains reachable is the byte-value domain and
/// zero length, and neither is an error: all 255 non-NUL bytes and both empty
/// strings are legal inputs. This test asserts they are handled identically and
/// do *not* fault, which is the complement of the rows above.
#[test]
fn generic_boundaries_are_not_errors() {
    let pair = load_pair();
    let mut cap = Capture::begin();

    // Zero length, on either side and both.
    assert_same_bytes(&pair, &mut cap, b"", b"");
    assert_same_bytes(&pair, &mut cap, b"", b"a");
    assert_same_bytes(&pair, &mut cap, b"a", b"");

    // Every single byte value, one step past the ASCII range and at both ends of
    // the domain (0x01, 0x7f, 0x80, 0xff).
    for b in [1u8, 0x7f, 0x80, 0xff] {
        assert_same_bytes(&pair, &mut cap, &[b], &[b]);
        assert_same_bytes(&pair, &mut cap, &[b], b"a");
        assert_same_bytes(&pair, &mut cap, b"a", &[b]);
    }

    // The largest legal reject set (every non-NUL byte) and a maximal-length
    // single-page string, i.e. the practical upper end of the input domain.
    let all = full_alphabet();
    assert_same_bytes(&pair, &mut cap, b"", &all);
    let big = vec![b'a'; 4095];
    assert_same_bytes(&pair, &mut cap, &big, b"Z");
}
