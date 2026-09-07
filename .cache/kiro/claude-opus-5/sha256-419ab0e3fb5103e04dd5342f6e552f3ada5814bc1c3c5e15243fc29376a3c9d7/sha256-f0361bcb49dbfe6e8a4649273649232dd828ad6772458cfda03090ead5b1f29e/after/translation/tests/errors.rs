//! Phase C — error-path / boundary differential tests, one per `ERRORS.md` row.
//!
//! The C library performs **no** validation (see `ERRORS.md`: zero `return`s,
//! zero `if`s, zero asserts), so its whole rejection surface is the generic
//! C-API boundary: null pointers and out-of-bounds `out` buffers. Those are
//! undefined behavior rather than library errors, so each row is compared on
//! the *observed process outcome* — the exact terminating signal or exit
//! status, obtained by running the call in a forked child — and asserted to be
//! identical for C and Rust. "Both failed somehow" is not accepted.
//!
//! Row C14 from `CONFIGS.md` (16-byte `out` flush against an unmapped guard
//! page) lives here too, since it shares the mmap machinery.

mod common;

use common::{load_pair, Impl, Pair, Rng, TflacMd5, SEED};

// ---------------------------------------------------------------- libc FFI --
// Declared directly; the test binary already links libc, so no extra crate.

const PROT_NONE: i32 = 0;
const PROT_READ: i32 = 1;
const PROT_WRITE: i32 = 2;
const MAP_PRIVATE: i32 = 0x02;
const MAP_ANONYMOUS: i32 = 0x20;

unsafe extern "C" {
    fn mmap(addr: *mut u8, len: usize, prot: i32, flags: i32, fd: i32, off: i64) -> *mut u8;
    fn mprotect(addr: *mut u8, len: usize, prot: i32) -> i32;
    fn munmap(addr: *mut u8, len: usize) -> i32;
    fn fork() -> i32;
    fn waitpid(pid: i32, status: *mut i32, options: i32) -> i32;
    fn _exit(code: i32) -> !;
}

const PAGE: usize = 4096;

/// Outcome of running one call in a child process.
#[derive(Debug, PartialEq, Eq, Clone, Copy)]
enum Outcome {
    Exited(i32),
    Signaled(i32),
}

/// Fork, run `f` in the child, and report how the child terminated.
///
/// The child performs only the FFI call and then `_exit(0)`, so it stays
/// async-signal-safe even though the test harness is multi-threaded.
fn child_outcome<F: FnOnce()>(f: F) -> Outcome {
    // SAFETY: child does minimal work then `_exit`; parent only waits.
    unsafe {
        let pid = fork();
        assert!(pid >= 0, "fork failed");
        if pid == 0 {
            f();
            _exit(0);
        }
        let mut status: i32 = 0;
        let r = waitpid(pid, &mut status as *mut i32, 0);
        assert_eq!(r, pid, "waitpid failed");
        let term_sig = status & 0x7f;
        if term_sig == 0 {
            Outcome::Exited((status >> 8) & 0xff)
        } else {
            Outcome::Signaled(term_sig)
        }
    }
}

/// Assert C and Rust terminate identically for the same invalid call, and that
/// the outcome is the specific one `ERRORS.md` predicts.
fn assert_same_outcome<F>(pair: &Pair, label: &str, expected: Outcome, mk: F)
where
    F: Fn(&Impl),
{
    let oc = child_outcome(|| mk(&pair.c));
    let or = child_outcome(|| mk(&pair.rust));
    assert_eq!(oc, or, "[{label}] C gave {oc:?} but Rust gave {or:?}");
    assert_eq!(oc, expected, "[{label}] expected {expected:?}, both gave {oc:?}");
}

const SIGSEGV: i32 = 11;

// ------------------------------------------------------------------- E1..E3 --

#[test]
fn err_e1_null_m() {
    let pair = load_pair();
    assert_same_outcome(&pair, "E1 m==NULL", Outcome::Signaled(SIGSEGV), |imp| {
        let mut out = [0u8; 16];
        unsafe { imp.call(std::ptr::null(), out.as_mut_ptr()) };
        // Keep the buffer alive past the call.
        std::hint::black_box(&out);
    });
}

#[test]
fn err_e2_null_out() {
    let pair = load_pair();
    let m = TflacMd5::new(0x1122_3344, 0x5566_7788, 0x99AA_BBCC, 0xDDEE_FF00);
    assert_same_outcome(&pair, "E2 out==NULL", Outcome::Signaled(SIGSEGV), move |imp| {
        unsafe { imp.call(&m as *const TflacMd5, std::ptr::null_mut()) };
    });
}

#[test]
fn err_e3_both_null() {
    let pair = load_pair();
    assert_same_outcome(&pair, "E3 both NULL", Outcome::Signaled(SIGSEGV), |imp| {
        unsafe { imp.call(std::ptr::null(), std::ptr::null_mut()) };
    });
}

// --------------------------------------------------------- guard-page setup --

/// Map two pages, then revoke all access to the second one. Returns the base
/// address; the writable region is `base[..PAGE]` and `base[PAGE..]` faults.
fn map_with_guard() -> *mut u8 {
    // SAFETY: plain anonymous mapping.
    unsafe {
        let base = mmap(
            std::ptr::null_mut(),
            2 * PAGE,
            PROT_READ | PROT_WRITE,
            MAP_PRIVATE | MAP_ANONYMOUS,
            -1,
            0,
        );
        assert!(base as isize != -1, "mmap failed");
        assert_eq!(mprotect(base.add(PAGE), PAGE, PROT_NONE), 0, "mprotect failed");
        base
    }
}

fn unmap_guard(base: *mut u8) {
    // SAFETY: `base` came from `map_with_guard`.
    unsafe {
        assert_eq!(munmap(base, 2 * PAGE), 0, "munmap failed");
    }
}

/// Run the call with `out` placed `slack` bytes below the guard page, i.e. the
/// writable tail is exactly `slack` bytes long.
fn guard_case(imp: &Impl, m: &TflacMd5, slack: usize) {
    let base = map_with_guard();
    // SAFETY: out lands inside the writable page; writes beyond `slack` fault.
    unsafe {
        let out = base.add(PAGE - slack);
        imp.call(m as *const TflacMd5, out);
    }
    unmap_guard(base);
}

// ---------------------------------------------------------------------- E4 --

#[test]
fn err_e4_writes_exactly_16_undersized_out() {
    let pair = load_pair();
    let m = TflacMd5::new(0x0403_0201, 0x0807_0605, 0x0C0B_0A09, 0x100F_0E0D);

    // 16 bytes of slack: the full write fits, so the child exits cleanly.
    assert_same_outcome(&pair, "E4 slack=16", Outcome::Exited(0), move |imp| {
        guard_case(imp, &m, 16);
    });

    // Every undersized buffer must fault identically in both: the C writes
    // indices 0..15 unconditionally, with no truncation and no error return.
    for slack in [1usize, 2, 3, 4, 8, 12, 15] {
        assert_same_outcome(
            &pair,
            &format!("E4 slack={slack}"),
            Outcome::Signaled(SIGSEGV),
            move |imp| guard_case(imp, &m, slack),
        );
    }
}

// ---------------------------------------------------------------------- E5 --

#[test]
fn err_e5_no_overwrite_past_16() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 0xE5);
    // Oversized `out`: the tail must be untouched, and by exactly the same
    // amount in both implementations.
    for i in 0..512 {
        let m = rng.next_md5();
        let run = |imp: &Impl| -> Vec<u8> {
            let mut buf = vec![0x3Cu8; 4096];
            unsafe { imp.call(&m as *const TflacMd5, buf.as_mut_ptr()) };
            buf
        };
        let (gc, gr) = (run(&pair.c), run(&pair.rust));
        assert_eq!(gc, gr, "[E5/#{i}] {m:?}");
        assert!(gc[16..].iter().all(|&b| b == 0x3C), "[E5/#{i}] C wrote past 16");
        assert!(gr[16..].iter().all(|&b| b == 0x3C), "[E5/#{i}] Rust wrote past 16");
    }
}

// ------------------------------------------------------- C14 (from CONFIGS) --

#[test]
fn cfg_c14_out_flush_against_guard_page() {
    let pair = load_pair();
    let mut rng = Rng::new(SEED ^ 14);

    // Exactly 16 writable bytes before an unmapped page. Surviving proves
    // neither implementation touches a 17th byte, and the bytes written must
    // match.
    for i in 0..64 {
        let m = rng.next_md5();
        let run = |imp: &Impl| -> [u8; 16] {
            let base = map_with_guard();
            let mut copy = [0u8; 16];
            // SAFETY: `out` is the last 16 writable bytes of the mapping.
            unsafe {
                let out = base.add(PAGE - 16);
                imp.call(&m as *const TflacMd5, out);
                std::ptr::copy_nonoverlapping(out, copy.as_mut_ptr(), 16);
            }
            unmap_guard(base);
            copy
        };
        let (gc, gr) = (run(&pair.c), run(&pair.rust));
        assert_eq!(gc, gr, "[C14/#{i}] {m:?}\n C={gc:02x?}\n R={gr:02x?}");
    }

    // Mirror case: the struct itself flush against the guard page, so any
    // read past 16 bytes would fault.
    for i in 0..64 {
        let m = rng.next_md5();
        let run = |imp: &Impl| -> Outcome {
            child_outcome(|| {
                let base = map_with_guard();
                // SAFETY: struct occupies the final 16 writable bytes.
                unsafe {
                    let mp = base.add(PAGE - 16);
                    std::ptr::copy_nonoverlapping(&m as *const TflacMd5 as *const u8, mp, 16);
                    let mut out = [0u8; 16];
                    imp.call(mp as *const TflacMd5, out.as_mut_ptr());
                    std::hint::black_box(&out);
                }
                unmap_guard(base);
            })
        };
        let (oc, or) = (run(&pair.c), run(&pair.rust));
        assert_eq!(oc, or, "[C14/read#{i}] C={oc:?} Rust={or:?}");
        assert_eq!(oc, Outcome::Exited(0), "[C14/read#{i}] no over-read expected");
    }
}

// ------------------------------------------------------------------- E6/E7 --
// Recorded as derived-and-vacuous in ERRORS.md; asserted here so the claim is
// mechanically checked rather than merely stated.

#[test]
fn err_e6_e7_no_bounded_scalar_and_no_enum_in_api() {
    let root = common::workspace_root().join("c_src");
    let h = std::fs::read_to_string(root.join("include").join("lib.h")).unwrap();
    let c = std::fs::read_to_string(root.join("src").join("lib.c")).unwrap();
    let src = format!("{h}\n{c}");

    // E7: no enum crosses the FFI boundary, so no invalid variant exists.
    assert!(!src.contains("enum"), "an enum appeared in the C API; ERRORS.md E7 needs a real row");

    // E6: no validation construct exists, so no scalar has a valid range whose
    // boundary could be exceeded.
    for forbidden in ["if ", "if(", "switch", "assert", "return ", "return;", "NULL"] {
        assert!(
            !src.contains(forbidden),
            "C source contains `{forbidden}`; the ERRORS.md derivation must be redone"
        );
    }
}
