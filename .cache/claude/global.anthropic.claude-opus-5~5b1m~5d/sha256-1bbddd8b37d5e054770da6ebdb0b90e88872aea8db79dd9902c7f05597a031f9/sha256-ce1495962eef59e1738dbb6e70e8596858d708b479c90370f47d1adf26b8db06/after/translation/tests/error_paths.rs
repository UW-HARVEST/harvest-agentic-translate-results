//! Phase C — error-path differential tests, one per `ERRORS.md` row.
//!
//! `md5_digest` returns `void` and performs **no** validation, so its only
//! observable rejection behaviour is a hardware fault. Rows that fault are
//! therefore verified by re-executing this same test binary in a child process
//! and comparing the **exact termination signal** produced by the C `.so` and
//! by the Rust `.so` — not merely "both failed somehow".
//!
//! The failure mode this guards against is the *opposite* of the C's: a port
//! that adds a null check and returns quietly, or that panics/aborts (SIGABRT)
//! where the C segfaults (SIGSEGV), would be caught here.

mod common;

use common::*;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

// ---------------------------------------------------------------------------
// Child-process crash harness.
// ---------------------------------------------------------------------------

const CASE_ENV: &str = "DIFF_CRASH_CASE";
const LIB_ENV: &str = "DIFF_CRASH_LIB";

/// How a call terminated: either a fatal signal, or a normal exit code.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    Signal(i32),
    Exit(i32),
}

fn describe(o: Outcome) -> String {
    match o {
        Outcome::Signal(11) => "SIGSEGV(11)".to_string(),
        Outcome::Signal(6) => "SIGABRT(6)".to_string(),
        Outcome::Signal(7) => "SIGBUS(7)".to_string(),
        Outcome::Signal(s) => format!("signal({s})"),
        Outcome::Exit(c) => format!("exit({c})"),
    }
}

/// Re-run this test binary, executing only `crash_worker`, with the given case
/// and implementation selected via the environment.
fn run_case(which: Impl, case: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");
    let status = Command::new(exe)
        .args(["--exact", "crash_worker", "--test-threads", "1"])
        .env(CASE_ENV, case)
        .env(
            LIB_ENV,
            match which {
                Impl::C => "c",
                Impl::Rust => "rust",
            },
        )
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");
    match status.signal() {
        Some(s) => Outcome::Signal(s),
        None => Outcome::Exit(status.code().unwrap_or(-1)),
    }
}

/// Assert both implementations terminate the same way, and (when given) that it
/// is the specific expected outcome.
fn assert_same_outcome(case: &str, expected: Option<Outcome>) {
    let c = run_case(Impl::C, case);
    let r = run_case(Impl::Rust, case);
    assert_eq!(
        c,
        r,
        "case '{case}': C terminated with {} but Rust terminated with {}",
        describe(c),
        describe(r)
    );
    if let Some(exp) = expected {
        assert_eq!(
            c,
            exp,
            "case '{case}': expected {} from both, observed {}",
            describe(exp),
            describe(c)
        );
    }
}

/// The child-side worker. In a normal (parent) test run `DIFF_CRASH_CASE` is
/// unset and this is a no-op that simply passes.
#[test]
fn crash_worker() {
    let Ok(case) = std::env::var(CASE_ENV) else {
        return;
    };
    let which = match std::env::var(LIB_ENV).unwrap_or_default().as_str() {
        "c" => Impl::C,
        "rust" => Impl::Rust,
        other => panic!("bad {LIB_ENV}: {other}"),
    };
    let libs = Libs::load();
    let f = libs.digest(which);
    let m = Md5 {
        a: 0x0403_0201,
        b: 0x0807_0605,
        c: 0x0C0B_0A09,
        d: 0x100F_0E0D,
    };
    let mut out = [0u8; 16];

    match case.as_str() {
        // E1: m == NULL
        "null_m" => unsafe { f(std::ptr::null(), out.as_mut_ptr()) },
        // E2: out == NULL
        "null_out" => unsafe { f(&m as *const Md5, std::ptr::null_mut()) },
        // E3: both NULL
        "both_null" => unsafe { f(std::ptr::null(), std::ptr::null_mut()) },
        // E4: out points into a read-only mapping
        "readonly_out" => unsafe {
            let p = mmap(
                std::ptr::null_mut(),
                PAGE,
                PROT_READ,
                MAP_PRIVATE | MAP_ANONYMOUS,
                -1,
                0,
            );
            assert!(p as isize != -1);
            f(&m as *const Md5, p as *mut u8)
        },
        // E5: only 15 writable bytes before a PROT_NONE guard -> out[15] faults
        "out_15" => unsafe {
            let g = GuardedPage::new();
            f(&m as *const Md5, g.end_minus(15))
        },
        // E6: only 15 readable source bytes before the guard -> reading d faults
        "m_15" => unsafe {
            let g = GuardedPage::new();
            g.write_at_end(15, &[0xEE; 15]);
            f(g.end_minus(15) as *const Md5, out.as_mut_ptr())
        },
        // E9 / E10: non-null but never-mapped addresses
        "wild_m" => unsafe { f(1usize as *const Md5, out.as_mut_ptr()) },
        "wild_out" => unsafe { f(&m as *const Md5, 1usize as *mut u8) },
        other => panic!("unknown case {other}"),
    }

    // Reached only if the call did NOT fault. Keep `out` alive so the compiler
    // cannot elide the buffer, then exit(0) to report "no fault".
    std::hint::black_box(&out);
}

// ---------------------------------------------------------------------------
// E1..E4, E9, E10 — faulting rows. Same signal from both, and specifically
// SIGSEGV (a Rust panic/abort would show up as SIGABRT and fail).
// ---------------------------------------------------------------------------

#[test]
fn err_null_m_segv_both() {
    assert_same_outcome("null_m", Some(Outcome::Signal(11)));
}

#[test]
fn err_null_out_segv_both() {
    assert_same_outcome("null_out", Some(Outcome::Signal(11)));
}

#[test]
fn err_both_null_segv_both() {
    assert_same_outcome("both_null", Some(Outcome::Signal(11)));
}

#[test]
fn err_readonly_out_segv_both() {
    assert_same_outcome("readonly_out", Some(Outcome::Signal(11)));
}

#[test]
fn err_wild_m_segv_both() {
    assert_same_outcome("wild_m", Some(Outcome::Signal(11)));
}

#[test]
fn err_wild_out_segv_both() {
    assert_same_outcome("wild_out", Some(Outcome::Signal(11)));
}

// ---------------------------------------------------------------------------
// E5, E6 — the extent boundary, from both sides: one byte short must fault
// identically, and exactly 16 bytes must NOT fault.
// ---------------------------------------------------------------------------

#[test]
fn err_out_15_bytes_segv_both() {
    assert_same_outcome("out_15", Some(Outcome::Signal(11)));
}

#[test]
fn err_m_15_bytes_segv_both() {
    assert_same_outcome("m_15", Some(Outcome::Signal(11)));
}

#[test]
fn err_out_exactly_16_no_overrun() {
    // Exactly 16 writable bytes before the guard page: must complete cleanly,
    // proving no 17th byte is written.
    let libs = Libs::load();
    let m = Md5 {
        a: 0x1122_3344,
        b: 0x5566_7788,
        c: 0x99AA_BBCC,
        d: 0xDDEE_FF00,
    };
    for which in [Impl::C, Impl::Rust] {
        let g = GuardedPage::new();
        g.fill_end(16, 0x00);
        let f = libs.digest(which);
        unsafe { f(&m as *const Md5, g.end_minus(16)) };
        assert_eq!(
            g.read_end(16),
            md5_to_le_bytes(m).to_vec(),
            "{} wrote wrong bytes at the page-end boundary",
            which.name()
        );
    }
}

#[test]
fn err_m_exactly_16_no_overread() {
    // Exactly 16 readable source bytes before the guard page.
    let libs = Libs::load();
    let m = Md5 {
        a: 0xCAFE_BABE,
        b: 0xDEAD_BEEF,
        c: 0x0BAD_F00D,
        d: 0xFEED_FACE,
    };
    for which in [Impl::C, Impl::Rust] {
        let g = GuardedPage::new();
        g.write_at_end(16, &md5_to_le_bytes(m));
        let mut out = [0u8; 16];
        let f = libs.digest(which);
        unsafe { f(g.end_minus(16) as *const Md5, out.as_mut_ptr()) };
        assert_eq!(
            out,
            md5_to_le_bytes(m),
            "{} read wrong bytes at the page-end boundary",
            which.name()
        );
    }
}

// ---------------------------------------------------------------------------
// E7, E8 — misalignment is NOT an error in C (no fault, correct answer). A port
// that dereferences a `*const tflac_md5` directly aborts here under debug
// assertions, so these rows are load-bearing.
// ---------------------------------------------------------------------------

#[test]
fn err_misaligned_m_no_fault() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x7777);
    for off in 1..=7usize {
        for _ in 0..32 {
            let m = rng.md5();
            let sc = Scenario {
                buf_len: 128,
                m_off: off,
                out_off: 64,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("E7 m misaligned by {off}"));
        }
    }
}

#[test]
fn err_misaligned_out_no_fault() {
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x8888);
    for off in 1..=7usize {
        for _ in 0..32 {
            let m = rng.md5();
            let sc = Scenario {
                buf_len: 128,
                m_off: 0,
                out_off: 64 + off,
                fill: 0xAA,
                src: md5_to_le_bytes(m),
            };
            sc.assert_match(&libs, &format!("E8 out misaligned by {off}"));
        }
    }
}

// ---------------------------------------------------------------------------
// E11..E13 — degenerate-but-legal input must not be treated as an error.
// ---------------------------------------------------------------------------

#[test]
fn err_all_zero_is_not_an_error() {
    let libs = Libs::load();
    let m = Md5::default();
    // Pre-fill with a sentinel so "left untouched" is distinguishable from
    // "correctly stored as zero".
    let c = digest16_prefill(&libs, Impl::C, m, 0xA5);
    let r = digest16_prefill(&libs, Impl::Rust, m, 0xA5);
    assert_eq!(c, r, "E11: all-zero input diverged");
    assert_eq!(c, [0u8; 16], "E11: all 16 bytes must be stored");
}

#[test]
fn err_exact_overlap_defined() {
    // E12: out == (tflac_u8 *)m is legal (no `restrict`) and well-defined.
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1212);
    for i in 0..200 {
        let m = rng.md5();
        let sc = Scenario {
            buf_len: 64,
            m_off: 16,
            out_off: 16,
            fill: 0x33,
            src: md5_to_le_bytes(m),
        };
        sc.assert_match(&libs, &format!("E12 iter={i}"));
    }
}

#[test]
fn err_repeat_call_idempotent() {
    // E13: no reset / init function exists, so repeated calls must be pure.
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0x1313);
    for i in 0..200 {
        let m = rng.md5();
        let mut prev: Option<[u8; 16]> = None;
        for _ in 0..4 {
            let c = digest16_prefill(&libs, Impl::C, m, 0x5C);
            let r = digest16_prefill(&libs, Impl::Rust, m, 0x5C);
            assert_eq!(c, r, "E13 iter={i}: diverged on repeat");
            if let Some(p) = prev {
                assert_eq!(p, c, "E13 iter={i}: not idempotent");
            }
            prev = Some(c);
        }
    }
}

// ---------------------------------------------------------------------------
// Generic FFI-boundary boundary conditions.
//
// The API has NO enum or integer parameter (see ERRORS.md), so there is no
// "out-of-range enum variant" to pass. The closest real analogue is garbage in
// the argument registers beyond the two declared parameters: a C callee ignores
// them, and the Rust export must ignore them identically rather than, say,
// mis-reading an argument slot.
// ---------------------------------------------------------------------------

#[test]
fn err_extra_garbage_args_ignored_identically() {
    type WideFn = unsafe extern "C" fn(*const Md5, *mut u8, u64, u64, u32, i32, usize);
    let libs = Libs::load();
    let mut rng = Rng::new(SEED ^ 0xABCD);

    for i in 0..300 {
        let m = rng.md5();
        let g = (
            rng.next_u64(),
            rng.next_u64(),
            rng.next_u32(),
            rng.next_u32() as i32,
            rng.next_u64() as usize,
        );
        let mut got = Vec::new();
        for which in [Impl::C, Impl::Rust] {
            let sym = libs.digest(which);
            // Re-type the same loaded symbol with extra trailing parameters.
            let wide: WideFn = unsafe { std::mem::transmute(*sym) };
            let mut out = [0u8; 16];
            unsafe { wide(&m as *const Md5, out.as_mut_ptr(), g.0, g.1, g.2, g.3, g.4) };
            got.push(out);
        }
        assert_eq!(
            got[0], got[1],
            "extra-args iter={i}: C and Rust diverged with garbage in unused arg registers"
        );
        assert_eq!(got[0], md5_to_le_bytes(m), "extra-args iter={i}: wrong bytes");
    }
}

/// Sanity: a `void`-returning C function leaves the return-value register
/// undefined; neither side may be relied upon to return anything, and calling
/// through a signature that (incorrectly) expects a return value must still not
/// diverge in the *memory* effect, which is the only part of the contract.
#[test]
fn err_void_return_not_observable() {
    type IntRetFn = unsafe extern "C" fn(*const Md5, *mut u8) -> i32;
    let libs = Libs::load();
    let m = Md5 {
        a: 1,
        b: 2,
        c: 3,
        d: 4,
    };
    let mut outs = Vec::new();
    for which in [Impl::C, Impl::Rust] {
        let sym = libs.digest(which);
        let f: IntRetFn = unsafe { std::mem::transmute(*sym) };
        let mut out = [0u8; 16];
        let _ignored = unsafe { f(&m as *const Md5, out.as_mut_ptr()) };
        outs.push(out);
    }
    assert_eq!(outs[0], outs[1], "memory effect diverged");
    assert_eq!(outs[0], md5_to_le_bytes(m));
}
