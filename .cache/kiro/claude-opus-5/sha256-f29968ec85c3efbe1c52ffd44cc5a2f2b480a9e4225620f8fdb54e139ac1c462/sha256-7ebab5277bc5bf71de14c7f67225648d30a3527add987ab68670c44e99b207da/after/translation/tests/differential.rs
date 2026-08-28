//! Differential tests: every exported function is loaded from BOTH the C
//! shared library and the Rust `cdylib` through `libloading` and compared for
//! identical return values and identical bytes written to stdout.
//!
//! Ordered lowest-level first (`cleanup_resources`, `print_result`) then the
//! higher-level entry point (`cleanup`) which calls into `cleanup_resources`.

mod common;

use common::{c_lib_path, capture_stdout, malloc, rust_lib_path, show, strncmp};
use libloading::{Library, Symbol};
use std::ffi::{c_char, c_int};

type CleanupFn = unsafe extern "C" fn(c_int, c_int, c_int, c_int) -> c_int;
type PrintResultFn = unsafe extern "C" fn(*const c_char, c_int);
type CleanupResourcesFn = unsafe extern "C" fn(*mut c_char);

struct Libs {
    c: Library,
    rust: Library,
}

impl Libs {
    fn load() -> Self {
        let c = unsafe { Library::new(c_lib_path()) }.expect("load C .so");
        let rust = unsafe { Library::new(rust_lib_path()) }.expect("load Rust .so");
        Libs { c, rust }
    }

    fn sym<T>(&self, from_c: bool, name: &[u8]) -> Symbol<'_, T> {
        let lib = if from_c { &self.c } else { &self.rust };
        unsafe { lib.get(name) }.unwrap_or_else(|e| {
            panic!(
                "symbol {} missing from {} .so: {e}",
                String::from_utf8_lossy(name),
                if from_c { "C" } else { "Rust" }
            )
        })
    }
}

// ---------------------------------------------------------------------------
// Level 1: cleanup_resources(char *)
// ---------------------------------------------------------------------------

fn cleanup_resources_matches() {
    let libs = Libs::load();
    let c_fn: Symbol<CleanupResourcesFn> = libs.sym(true, b"cleanup_resources");
    let r_fn: Symbol<CleanupResourcesFn> = libs.sym(false, b"cleanup_resources");

    // NULL is a no-op in both implementations.
    let ((), c_out) = capture_stdout(|| unsafe { c_fn(std::ptr::null_mut()) });
    let ((), r_out) = capture_stdout(|| unsafe { r_fn(std::ptr::null_mut()) });
    assert_eq!(c_out, r_out, "cleanup_resources(NULL) stdout differs");
    assert!(c_out.is_empty(), "expected no output, got {}", show(&c_out));

    // A live allocation must be freed by each side (fresh buffer per call, the
    // process shares one libc allocator with both libraries).
    for size in [1usize, 8, 50, 4096] {
        let p = unsafe { malloc(size) } as *mut c_char;
        assert!(!p.is_null());
        let ((), c_out) = capture_stdout(|| unsafe { c_fn(p) });

        let p = unsafe { malloc(size) } as *mut c_char;
        assert!(!p.is_null());
        let ((), r_out) = capture_stdout(|| unsafe { r_fn(p) });

        assert_eq!(c_out, r_out, "cleanup_resources(ptr) stdout differs");
        assert!(c_out.is_empty());
    }
}

// ---------------------------------------------------------------------------
// Level 1: print_result(const char *, int)
// ---------------------------------------------------------------------------

fn print_result_matches() {
    let libs = Libs::load();
    let c_fn: Symbol<PrintResultFn> = libs.sym(true, b"print_result");
    let r_fn: Symbol<PrintResultFn> = libs.sym(false, b"print_result");

    let labels: [&std::ffi::CStr; 7] = [
        c"",
        c"result",
        c"Total",
        c"a very long label that exceeds any plausible internal buffer size, \
           padded out with filler text so the formatting path is exercised",
        c"percent %s %d literal",
        c"tab\tand\nnewline",
        c"unicode \xE2\x9C\x93 bytes",
    ];
    let values: [c_int; 9] = [0, 1, -1, 42, -42, 12345, c_int::MAX, c_int::MIN, 100];

    for label in labels {
        for v in values {
            let ((), c_out) = capture_stdout(|| unsafe { c_fn(label.as_ptr(), v) });
            let ((), r_out) = capture_stdout(|| unsafe { r_fn(label.as_ptr(), v) });
            assert_eq!(
                c_out,
                r_out,
                "print_result({:?}, {v}) stdout differs:\n C: {}\n R: {}",
                label,
                show(&c_out),
                show(&r_out)
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Level 2: cleanup(int, int, int, int)
// ---------------------------------------------------------------------------

/// Values chosen to hit every `switch` arm and both fall-through chains
/// (`case 10 -> case 20`, `case 30 -> case 40`) plus the `default` arm,
/// including boundary/near-miss values and signed wraparound.
const CANDIDATES: [c_int; 15] = [
    10,
    20,
    30,
    40,
    0,
    1,
    -1,
    9,
    11,
    29,
    41,
    -30,
    100,
    c_int::MAX,
    c_int::MIN,
];

fn run_all_cleanup(f: &CleanupFn) -> (Vec<c_int>, Vec<u8>) {
    capture_stdout(|| {
        let mut rets = Vec::with_capacity(CANDIDATES.len().pow(4));
        for a in CANDIDATES {
            for b in CANDIDATES {
                for c in CANDIDATES {
                    for d in CANDIDATES {
                        rets.push(unsafe { f(a, b, c, d) });
                    }
                }
            }
        }
        rets
    })
}

fn cleanup_matches_exhaustively() {
    let libs = Libs::load();
    let c_fn: Symbol<CleanupFn> = libs.sym(true, b"cleanup");
    let r_fn: Symbol<CleanupFn> = libs.sym(false, b"cleanup");

    let (c_rets, c_out) = run_all_cleanup(&c_fn);
    let (r_rets, r_out) = run_all_cleanup(&r_fn);

    assert_eq!(c_rets.len(), r_rets.len());
    assert_eq!(
        c_rets.len(),
        CANDIDATES.len().pow(4),
        "expected every 4-tuple of candidate arguments to be exercised"
    );

    // Report the first differing tuple with its actual arguments.
    let n = CANDIDATES.len();
    for (idx, (cv, rv)) in c_rets.iter().zip(r_rets.iter()).enumerate() {
        if cv != rv {
            let d = CANDIDATES[idx % n];
            let c = CANDIDATES[(idx / n) % n];
            let b = CANDIDATES[(idx / (n * n)) % n];
            let a = CANDIDATES[(idx / (n * n * n)) % n];
            panic!("cleanup({a}, {b}, {c}, {d}) returned C={cv} Rust={rv}");
        }
    }

    if c_out != r_out {
        // Locate the first differing byte for a readable message.
        let pos = c_out
            .iter()
            .zip(r_out.iter())
            .position(|(x, y)| x != y)
            .unwrap_or(c_out.len().min(r_out.len()));
        let lo = pos.saturating_sub(80);
        panic!(
            "cleanup stdout differs at byte {pos} (C len {}, Rust len {}):\n C: {}\n R: {}",
            c_out.len(),
            r_out.len(),
            show(&c_out[lo..(lo + 160).min(c_out.len())]),
            show(&r_out[lo..(lo + 160).min(r_out.len())]),
        );
    }
    assert!(!c_out.is_empty(), "expected cleanup to print something");
}

/// The C code validates a string with `strncmp("VALID", "VALID", 5)`, which can
/// never fail, so the `Input string validation failed.` branch is dead in both
/// implementations. Confirm the premise holds in this environment and that
/// neither library ever emits that message.
fn validation_branch_is_unreachable_in_both() {
    assert_eq!(
        unsafe { strncmp(c"VALID".as_ptr(), c"VALID".as_ptr(), 5) },
        0
    );

    let libs = Libs::load();
    let c_fn: Symbol<CleanupFn> = libs.sym(true, b"cleanup");
    let r_fn: Symbol<CleanupFn> = libs.sym(false, b"cleanup");

    let (_, c_out) = capture_stdout(|| unsafe { c_fn(10, 20, 30, 40) });
    let (_, r_out) = capture_stdout(|| unsafe { r_fn(10, 20, 30, 40) });
    for out in [&c_out, &r_out] {
        assert!(
            !String::from_utf8_lossy(out).contains("validation failed"),
            "unexpected validation message: {}",
            show(out)
        );
    }
    assert_eq!(c_out, r_out);
}

/// Regression pin for the exact stringized-macro text: `TO_STRING(numbers)`
/// stringizes the *token*, so the message is literally
/// `Processed numbers: numbers`.
fn cleanup_stringize_output_is_token_text() {
    let libs = Libs::load();
    let c_fn: Symbol<CleanupFn> = libs.sym(true, b"cleanup");
    let r_fn: Symbol<CleanupFn> = libs.sym(false, b"cleanup");

    let (c_ret, c_out) = capture_stdout(|| unsafe { c_fn(1, 2, 3, 4) });
    let (r_ret, r_out) = capture_stdout(|| unsafe { r_fn(1, 2, 3, 4) });

    assert_eq!(c_ret, r_ret);
    assert_eq!(c_out, r_out);
    assert_eq!(c_out, b"Processed numbers: numbers\n".to_vec(), "{}", show(&c_out));
}

/// Interleave calls between the two libraries to catch any hidden per-library
/// mutable state (there should be none).
fn cleanup_interleaved_calls_match() {
    let libs = Libs::load();
    let c_fn: Symbol<CleanupFn> = libs.sym(true, b"cleanup");
    let r_fn: Symbol<CleanupFn> = libs.sym(false, b"cleanup");

    for round in 0..50i32 {
        let (a, b, c, d) = (round, 10 * (round % 5), -round, 30 - round);
        let (c_ret, c_out) = capture_stdout(|| unsafe { c_fn(a, b, c, d) });
        let (r_ret, r_out) = capture_stdout(|| unsafe { r_fn(a, b, c, d) });
        assert_eq!(c_ret, r_ret, "cleanup({a}, {b}, {c}, {d}) return differs");
        assert_eq!(c_out, r_out, "cleanup({a}, {b}, {c}, {d}) stdout differs");
    }
}

// ---------------------------------------------------------------------------
// Single entry point.
//
// All of the checks above redirect process file descriptor 1 to compare the
// bytes the two libraries print. That is inherently process-global, so they
// are driven from one `#[test]` to keep the harness from interleaving its own
// progress output (or another test's) into a capture window. Symbol parity
// lives in `tests/symbols.rs`, i.e. a separate test binary/process.
// ---------------------------------------------------------------------------

#[test]
fn c_and_rust_shared_libraries_agree() {
    // Lowest level first, then upward.
    cleanup_resources_matches();
    print_result_matches();
    validation_branch_is_unreachable_in_both();
    cleanup_stringize_output_is_token_text();
    cleanup_interleaved_calls_match();
    cleanup_matches_exhaustively();
}
