//! Phase C (continued) — the `ERRORS.md` rows whose C behaviour is a FATAL
//! SIGNAL. They are only observable from another process, so each row runs the
//! identical scenario twice in a child process (once against the C `.so`, once
//! against the Rust `.so`) and the two termination statuses are compared:
//! same signal number, same exit code. "Both failed somehow" is not accepted —
//! the signal numbers must be equal AND must be the SIGSEGV/SIGBUS the C
//! implementation produces.
//!
//! To make the fault deterministic (instead of relying on the heap layout) the
//! scenarios use a page-aligned region of `RW_BYTES` writable bytes followed by
//! a `PROT_NONE` guard, so the runaway `memset`/read faults at a known address.

mod common;

use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

use common::*;

// --------------------------------------------------------------------------
// guarded region
// --------------------------------------------------------------------------

unsafe extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
}

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

const RW_BYTES: usize = 64 * 1024;
const TOTAL_BYTES: usize = 4 * 1024 * 1024;

/// `RW_BYTES` readable+writable bytes followed by an unreadable guard.
fn guarded_region() -> *mut f32 {
    unsafe {
        let p = mmap(
            std::ptr::null_mut(),
            TOTAL_BYTES,
            PROT_NONE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        assert!(p as isize != -1 && !p.is_null(), "mmap failed");
        assert_eq!(
            mprotect(p, RW_BYTES, PROT_READ | PROT_WRITE),
            0,
            "mprotect failed"
        );
        p as *mut f32
    }
}

// --------------------------------------------------------------------------
// child side
// --------------------------------------------------------------------------

const CASE_ENV: &str = "HARVEST_CRASH_CASE";
const IMPL_ENV: &str = "HARVEST_CRASH_IMPL";

#[test]
#[ignore = "sub-process helper, launched by the crash-parity tests"]
fn crash_child() {
    let case = std::env::var(CASE_ENV).expect("case env");
    let which = std::env::var(IMPL_ENV).expect("impl env");
    let (c, r) = load_impls();
    let imp = match which.as_str() {
        "c" => &c,
        "rust" => &r,
        other => panic!("bad impl {other}"),
    };
    let f = imp.normalize;

    match case.as_str() {
        // ERRORS.md row 5: size < 0 with dest != src -> memset of length
        // (size_t)(size * 4) == 0xFFFF_FFFF_FFFF_FFFC
        "neg_size_disjoint" => {
            let dest = guarded_region();
            let src = [1.0f32, 2.0, 3.0, 4.0];
            unsafe { f(dest, src.as_ptr(), -1) };
        }
        // ERRORS.md row 6: size == INT_MIN with dest != src
        "int_min_size_disjoint" => {
            let dest = guarded_region();
            let src = [1.0f32, 2.0, 3.0, 4.0];
            unsafe { f(dest, src.as_ptr(), i32::MIN) };
        }
        // ERRORS.md row 11: src == NULL with size > 0 -> read of *NULL
        "null_src_positive" => {
            let mut dest = [0.0f32; 16];
            unsafe { f(dest.as_mut_ptr(), std::ptr::null(), 16) };
        }
        // ERRORS.md row 12: dest == NULL with size > 0 and sum > 0
        "null_dest_positive" => {
            let src = [1.0f32; 16];
            unsafe { f(std::ptr::null_mut(), src.as_ptr(), 16) };
        }
        // ERRORS.md row 23 upper end: size == INT_MAX reads past the buffer
        "int_max_size_inplace" => {
            let buf = guarded_region();
            unsafe {
                for i in 0..(RW_BYTES / 4) {
                    *buf.add(i) = 1.0;
                }
                f(buf, buf, i32::MAX);
            }
        }
        other => panic!("unknown case {other}"),
    }
    // If we get here the scenario did NOT fault; report it distinguishably.
    println!("case {case} with impl {which} returned normally");
    std::process::exit(7);
}

// --------------------------------------------------------------------------
// parent side
// --------------------------------------------------------------------------

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
}

fn run_child(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let mut child = Command::new(exe)
        .arg("crash_child")
        .arg("--exact")
        .arg("--include-ignored")
        .arg("--test-threads=1")
        .env(CASE_ENV, case)
        .env(IMPL_ENV, which)
        .env("RUST_BACKTRACE", "0")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .expect("spawn child");

    // don't hang forever
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(60);
    loop {
        if let Some(status) = child.try_wait().expect("try_wait") {
            return Outcome {
                signal: status.signal(),
                code: status.code(),
            };
        }
        if std::time::Instant::now() > deadline {
            let _ = child.kill();
            panic!("child for case {case}/{which} did not terminate within 60s");
        }
        std::thread::sleep(std::time::Duration::from_millis(20));
    }
}

/// Both implementations must die with the SAME signal, and it must be a memory
/// fault (SIGSEGV = 11, SIGBUS = 7 on Linux/x86-64).
fn assert_same_fatal_signal(case: &str) {
    let c = run_child(case, "c");
    let r = run_child(case, "rust");
    assert_eq!(
        c, r,
        "case {case}: C terminated as {c:?} but Rust terminated as {r:?}\n\
         (signal 6 = SIGABRT from the Rust side means the `.so` was built with \
          debug-assertions on: rustc's null/alignment/overflow checks turn the \
          C library's fault into a non-unwinding panic. `[profile.dev] \
          debug-assertions = false` in Cargo.toml keeps the two ABI-identical.)"
    );
    let sig = c.signal().unwrap_or_else(|| {
        panic!("case {case}: C did not die from a signal: {c:?} (exit code 7 = returned normally)")
    });
    assert!(
        sig == 11 || sig == 7 || sig == 10,
        "case {case}: unexpected signal {sig}"
    );
    println!("case {case}: both implementations died with signal {sig}");
}

impl Outcome {
    fn signal(&self) -> Option<i32> {
        self.signal
    }
}

/// The C library has no null/alignment/overflow checks, so the Rust `.so` must
/// be built without rustc's debug-assertion runtime checks in order to be
/// behaviourally identical on the undefined-behaviour inputs a C caller can
/// still pass. This guards the Cargo.toml setting that makes that true.
#[test]
fn err_00_so_is_built_without_ub_runtime_checks() {
    assert!(
        !cfg!(debug_assertions),
        "debug assertions are enabled: rustc injects null-pointer / alignment / \
         overflow panics that the C library does not have; set \
         `[profile.dev] debug-assertions = false` (see Cargo.toml)"
    );
}

// ERRORS.md row 5
#[test]
fn err_05_negative_size_disjoint_crashes_identically() {
    assert_same_fatal_signal("neg_size_disjoint");
}

// ERRORS.md row 6
#[test]
fn err_06_int_min_size_disjoint_crashes_identically() {
    assert_same_fatal_signal("int_min_size_disjoint");
}

// ERRORS.md row 11
#[test]
fn err_11_null_src_positive_size_crashes_identically() {
    assert_same_fatal_signal("null_src_positive");
}

// ERRORS.md row 12
#[test]
fn err_12_null_dest_positive_size_crashes_identically() {
    assert_same_fatal_signal("null_dest_positive");
}

// ERRORS.md row 23, upper boundary of the `int size` range
#[test]
fn err_24_int_max_size_reads_out_of_bounds_identically() {
    assert_same_fatal_signal("int_max_size_inplace");
}
