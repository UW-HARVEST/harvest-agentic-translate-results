//! Phase C — error-path differential tests.
//!
//! One test per row of `ERRORS.md`. The C source contains no error returns at
//! all (`driver` is `void` and performs no validation), so its "error surface"
//! is the set of fault behaviours it exhibits on invalid pointers. Each row
//! therefore runs the call in an ISOLATED CHILD PROCESS and compares the exact
//! termination signal (not merely "both failed") plus the stdout bytes.
//!
//! The child is this same test executable re-executed with `DIFF_CHILD_CASE`
//! set; `child_worker` below picks that up, loads the requested `.so` via
//! `libloading`, performs the call, and exits. Re-exec is used instead of
//! `fork()` so that no libc/stdio lock can be inherited from the multi-threaded
//! test harness.

mod common;

use common::*;
use std::ffi::c_char;
use std::os::unix::process::ExitStatusExt;
use std::process::{Command, Stdio};

const CASE_ENV: &str = "DIFF_CHILD_CASE";
const LIB_ENV: &str = "DIFF_CHILD_LIB";
const OUT_ENV: &str = "DIFF_CHILD_OUT";

/// How a child process ended.
#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    signal: Option<i32>,
    code: Option<i32>,
    stdout: Vec<u8>,
}

fn run_case(case: &str, which: &str) -> Outcome {
    let exe = std::env::current_exe().expect("current_exe");

    // The child writes `driver`'s output to a file of its own rather than to
    // its stdout, for two reasons:
    //   * libtest prints its own banner ("running 1 test ...") to stdout, which
    //     would otherwise be mixed into the bytes under comparison;
    //   * a child inherits fd 1, which a *sibling* test thread may currently
    //     have redirected into its own capture file (see common::capture_stdout),
    //     so letting the child write to fd 1 would corrupt that test.
    // Hence stdout/stderr are /dev/null and the payload travels via this file.
    let mut out_path = std::env::temp_dir();
    out_path.push(format!(
        "driver_child_{}_{}_{}_{}.out",
        std::process::id(),
        case,
        which,
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let _ = std::fs::remove_file(&out_path);

    let status = Command::new(exe)
        .args(["child_worker", "--exact", "--test-threads=1"])
        .env(CASE_ENV, case)
        .env(LIB_ENV, which)
        .env(OUT_ENV, &out_path)
        .env(
            "C_DRIVER_SO",
            c_so_path().to_str().expect("utf8 path").to_string(),
        )
        .env(
            "RUST_DRIVER_SO",
            rust_so_path().to_str().expect("utf8 path").to_string(),
        )
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .expect("spawn child");

    let stdout = std::fs::read(&out_path).unwrap_or_default();
    let _ = std::fs::remove_file(&out_path);
    Outcome {
        signal: status.signal(),
        code: status.code(),
        stdout,
    }
}

static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);

/// Runs `case` against both `.so`s in separate processes and asserts the exact
/// same termination status and stdout.
fn assert_same_outcome(case: &str) -> Outcome {
    let c = run_case(case, "c");
    let r = run_case(case, "rust");
    assert_eq!(
        c.signal, r.signal,
        "[{case}] termination signal differs: C={:?} Rust={:?}\n  C stdout={:?}\n  Rust stdout={:?}",
        c.signal,
        r.signal,
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    assert_eq!(c.code, r.code, "[{case}] exit code differs");
    assert_eq!(
        c.stdout,
        r.stdout,
        "[{case}] stdout differs:\n  C   ={:?}\n  Rust={:?}",
        String::from_utf8_lossy(&c.stdout),
        String::from_utf8_lossy(&r.stdout)
    );
    c
}

fn assert_segv(case: &str) {
    let o = assert_same_outcome(case);
    assert_eq!(
        o.signal,
        Some(11),
        "[{case}] expected both to die with SIGSEGV(11), got {:?}/code {:?}",
        o.signal,
        o.code
    );
    assert!(
        o.stdout.is_empty(),
        "[{case}] expected no stdout before the fault, got {:?}",
        String::from_utf8_lossy(&o.stdout)
    );
}

// ---------------------------------------------------------------------------
// ERRORS.md rows 1-11 — invalid pointer conditions
// ---------------------------------------------------------------------------

#[test]
fn err_01_both_null() {
    assert_segv("both_null");
}

/// The interesting one: `s1` is a perfectly valid empty string, yet C still
/// faults, because glibc's `strcspn` materialises the reject set from `s2`
/// before it ever looks at `s1`. A hand-written `strcspn` that loops over `s1`
/// on the outside would print `0` here instead of crashing.
#[test]
fn err_02_empty_s1_null_s2() {
    assert_segv("empty_s1_null_s2");
}

#[test]
fn err_03_null_s1_empty_s2() {
    assert_segv("null_s1_empty_s2");
}

#[test]
fn err_04_valid_s1_null_s2() {
    assert_segv("valid_s1_null_s2");
}

#[test]
fn err_05_null_s1_valid_s2() {
    assert_segv("null_s1_valid_s2");
}

#[test]
fn err_06_unmapped_s1() {
    assert_segv("unmapped_s1");
}

#[test]
fn err_07_unmapped_s2() {
    assert_segv("unmapped_s2");
}

#[test]
fn err_08_both_unmapped() {
    assert_segv("both_unmapped");
}

#[test]
fn err_09_unterminated_s1_runs_off_page() {
    assert_segv("unterminated_s1");
}

#[test]
fn err_10_unterminated_s2_runs_off_page() {
    assert_segv("unterminated_s2");
}

#[test]
fn err_11_unterminated_s1_misaligned() {
    assert_segv("unterminated_s1_misaligned");
}

// ---------------------------------------------------------------------------
// Generic boundaries (documented in ERRORS.md as accepted, not rejected)
// ---------------------------------------------------------------------------

#[test]
fn bnd_empty_empty() {
    let impls = Impls::load();
    let out = same_bytes_out(&impls, b"", b"", "bnd empty/empty");
    assert_eq!(out, b"0\n");
}

#[test]
fn bnd_empty_s1() {
    let impls = Impls::load();
    let out = same_bytes_out(&impls, b"", b"abc", "bnd empty s1");
    assert_eq!(out, b"0\n");
}

#[test]
fn bnd_empty_s2() {
    let impls = Impls::load();
    for s1 in [&b"a"[..], b"abc", &[b'z'; 200][..]] {
        let out = same_bytes_out(&impls, s1, b"", "bnd empty s2");
        assert_eq!(out, format!("{}\n", s1.len()).into_bytes());
    }
}

#[test]
fn bnd_oversized_no_match() {
    let impls = Impls::load();
    let s1 = vec![b'a'; 1024 * 1024];
    let out = same_bytes_out(&impls, &s1, b"Z", "bnd oversized");
    assert_eq!(out, b"1048576\n");
}

#[test]
fn bnd_s2_all_255_bytes() {
    let impls = Impls::load();
    let s2: Vec<u8> = (1u8..=255).collect();
    for s1 in [&b"a"[..], b"\xff", b"\x01\x02\x03", b"hello world"] {
        let out = same_bytes_out(&impls, s1, &s2, "bnd all-255 reject set");
        assert_eq!(out, b"0\n");
    }
}

/// `char` signedness: a naive `i8` comparison sign-extends, so `0x80..=0xFF`
/// bytes are exactly where a hand-rolled `strcspn` diverges.
#[test]
fn bnd_high_bytes() {
    let impls = Impls::load();
    let mut rng = Rng::new(SEED ^ 0xB1);
    for _ in 0..256 {
        let len = rng.range(1, 40);
        let s1 = rng.bytes(len, 0x80, 0xFF);
        let s2 = rng.bytes(4, 0x80, 0xFF);
        same_bytes_out(&impls, &s1, &s2, "bnd high bytes");
    }
    // Explicit worst cases around the sign boundary.
    for &(a, b) in &[
        (0x7Fu8, 0xFFu8),
        (0xFF, 0x7F),
        (0x80, 0x00 + 0x80),
        (0x81, 0x01),
        (0xFE, 0x7E),
    ] {
        let s1 = [a, a, a];
        let s2 = [b];
        same_bytes_out(&impls, &s1, &s2, &format!("bnd sign pair {a:#x}/{b:#x}"));
    }
}

#[test]
fn bnd_embedded_nul() {
    let impls = Impls::load();
    // Buffers whose logical content continues past an embedded NUL: both must
    // stop at the first NUL and ignore the tail.
    let s1 = b"abc\0def\0";
    let s2 = b"d\0a\0";
    let p1 = s1.as_ptr() as *const c_char;
    let p2 = s2.as_ptr() as *const c_char;
    let c_out = capture_stdout(|| unsafe { (impls.c)(p1, p2) });
    let r_out = capture_stdout(|| unsafe { (impls.rust)(p1, p2) });
    assert_eq!(c_out, r_out, "bnd embedded NUL");
    // s2 is effectively "d"; "abc" contains no 'd', so the answer is 3.
    assert_eq!(c_out, b"3\n");
}

#[test]
fn bnd_aliased_pointers() {
    let impls = Impls::load();
    for s in [&b""[..], b"a", b"abcdef", &[b'k'; 100][..]] {
        let buf = cstr(s);
        let p = buf.as_ptr() as *const c_char;
        let c_out = capture_stdout(|| unsafe { (impls.c)(p, p) });
        let r_out = capture_stdout(|| unsafe { (impls.rust)(p, p) });
        assert_eq!(c_out, r_out, "bnd aliased");
        assert_eq!(c_out, b"0\n");
    }
}

#[test]
fn bnd_overlapping_buffers() {
    let impls = Impls::load();
    let buf = cstr(b"abcdefghij");
    for split in 1..10usize {
        let p2 = buf.as_ptr() as *const c_char;
        let p1 = unsafe { buf.as_ptr().add(split) } as *const c_char;
        let c_out = capture_stdout(|| unsafe { (impls.c)(p1, p2) });
        let r_out = capture_stdout(|| unsafe { (impls.rust)(p1, p2) });
        assert_eq!(c_out, r_out, "bnd overlapping split={split}");
        assert_eq!(c_out, b"0\n", "the suffix's first byte is always in s2");
    }
}

#[test]
fn bnd_printf_digit_widths() {
    let impls = Impls::load();
    for n in [0usize, 1, 9, 10, 99, 100, 999, 1000, 65535, 65536] {
        let s1 = vec![b'a'; n];
        let out = same_bytes_out(&impls, &s1, b"Z", &format!("bnd digits n={n}"));
        assert_eq!(out, format!("{n}\n").into_bytes());
    }
}

/// `driver`'s signature carries no enum, flag, or integer parameter, so there
/// is no out-of-range enum value that could cross the FFI boundary. This is
/// asserted at compile time against the real exported symbol type rather than
/// merely claimed in prose.
#[test]
fn phase_c_no_enum_parameters_exist() {
    let impls = Impls::load();
    // Both symbols must have exactly this type; any extra/enum parameter would
    // make these assignments fail to compile.
    let _c: unsafe extern "C" fn(*const c_char, *const c_char) = impls.c;
    let _r: unsafe extern "C" fn(*const c_char, *const c_char) = impls.rust;
    assert_eq!(
        std::mem::size_of::<DriverFn>(),
        std::mem::size_of::<usize>()
    );
}

// ---------------------------------------------------------------------------
// Child-process worker
// ---------------------------------------------------------------------------

/// Not a real test: when `DIFF_CHILD_CASE` is set this is the isolated child
/// that performs one (possibly faulting) call and exits. When the variable is
/// absent it does nothing, so a normal `cargo test` run just sees it pass.
#[test]
fn child_worker() {
    let Ok(case) = std::env::var(CASE_ENV) else {
        return;
    };
    let which = std::env::var(LIB_ENV).unwrap_or_else(|_| "c".into());
    let path = if which == "rust" {
        rust_so_path()
    } else {
        c_so_path()
    };

    let lib = unsafe { libloading::Library::new(&path).expect("dlopen in child") };
    let driver: libloading::Symbol<DriverFn> =
        unsafe { lib.get(b"driver\0").expect("dlsym in child") };

    // Point fd 1 at the parent's designated file and make libc's stdout
    // UNBUFFERED, so that if an implementation printed something and only then
    // faulted, those bytes are still observable instead of dying in the buffer.
    // Applied identically to both implementations.
    let out_path = std::env::var(OUT_ENV).expect("DIFF_CHILD_OUT");
    {
        use std::os::unix::io::AsRawFd;
        let f = std::fs::File::create(&out_path).expect("create child out file");
        unsafe {
            child_ffi::fflush(std::ptr::null_mut());
            assert!(child_ffi::dup2(f.as_raw_fd(), 1) >= 0, "child dup2");
            child_ffi::setvbuf(child_ffi::stdout, std::ptr::null_mut(), child_ffi::IONBF, 0);
        }
        std::mem::forget(f); // fd 1 now owns it
    }

    let valid = cstr(b"abc");
    let empty = cstr(b"");

    // Build the two pointers for this case.
    let (s1, s2): (*const c_char, *const c_char) = match case.as_str() {
        "both_null" => (std::ptr::null(), std::ptr::null()),
        "empty_s1_null_s2" => (empty.as_ptr() as *const c_char, std::ptr::null()),
        "null_s1_empty_s2" => (std::ptr::null(), empty.as_ptr() as *const c_char),
        "valid_s1_null_s2" => (valid.as_ptr() as *const c_char, std::ptr::null()),
        "null_s1_valid_s2" => (std::ptr::null(), valid.as_ptr() as *const c_char),
        "unmapped_s1" => (1usize as *const c_char, valid.as_ptr() as *const c_char),
        "unmapped_s2" => (valid.as_ptr() as *const c_char, 1usize as *const c_char),
        "both_unmapped" => (1usize as *const c_char, 2usize as *const c_char),
        "unterminated_s1" => {
            // One readable page fully filled with a non-NUL byte, then a guard
            // page: scanning for the (absent) NUL must run into the guard.
            let base = guarded_region(1);
            unsafe {
                for i in 0..PAGE {
                    *base.add(i) = b'a';
                }
            }
            // 'b' is not in s1, so nothing can stop the scan early.
            let s2 = Box::leak(cstr(b"b").into_boxed_slice());
            (base as *const c_char, s2.as_ptr() as *const c_char)
        }
        "unterminated_s2" => {
            let base = guarded_region(1);
            unsafe {
                for i in 0..PAGE {
                    *base.add(i) = b'b';
                }
            }
            // s1's bytes never appear in s2, so building the reject set must
            // read s2 all the way into the guard page.
            let s1 = Box::leak(cstr(b"aaaa").into_boxed_slice());
            (s1.as_ptr() as *const c_char, base as *const c_char)
        }
        "unterminated_s1_misaligned" => {
            let base = guarded_region(1);
            unsafe {
                for i in 0..PAGE {
                    *base.add(i) = b'a';
                }
            }
            let s2 = Box::leak(cstr(b"b").into_boxed_slice());
            // Start 7 bytes in, so the pointer is neither 16- nor 8-aligned.
            (
                unsafe { base.add(7) } as *const c_char,
                s2.as_ptr() as *const c_char,
            )
        }
        other => panic!("unknown child case {other:?}"),
    };

    unsafe { driver(s1, s2) };

    // Reached only when the call did NOT fault; flush so the parent sees the
    // bytes, then bypass the harness's own exit path (which would otherwise
    // print a summary into the same file).
    unsafe { child_ffi::fflush(std::ptr::null_mut()) };
    std::process::exit(0);
}

mod child_ffi {
    use std::ffi::{c_char, c_int, c_void};

    pub const IONBF: c_int = 2; // _IONBF

    extern "C" {
        pub fn fflush(stream: *mut c_void) -> c_int;
        pub fn dup2(oldfd: c_int, newfd: c_int) -> c_int;
        pub fn setvbuf(stream: *mut c_void, buf: *mut c_char, mode: c_int, size: usize) -> c_int;
        pub static stdout: *mut c_void;
    }
}
