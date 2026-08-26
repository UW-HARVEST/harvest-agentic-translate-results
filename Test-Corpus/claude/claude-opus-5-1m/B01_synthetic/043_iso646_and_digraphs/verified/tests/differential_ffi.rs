// Copyright 2025 MIT Lincoln Laboratory
// SPDX-License-Identifier: MIT
//
// FFI-boundary differential tests.
//
// Both the C shared object (built from the unmodified `c_src/src/main.c`) and the
// Rust cdylib (`examples/driver_ffi.rs`) are `dlopen`ed, and their exported
// symbols are invoked through function pointers. No Rust function is ever called
// directly, so the `#[no_mangle] extern "C"` wrappers are part of what is tested.
//
// Covers CONFIGS.md rows 1-6 and ERRORS.md rows 19-22.
//
// Comparing `printf` output means capturing file descriptor 1, which is
// process-wide state: the test harness itself writes progress lines to it from
// other threads. Every test body that captures fd 1 therefore runs in a
// dedicated single-threaded subprocess (`ffi_test!` re-executes the test binary
// with `DRIVER_FFI_ROW` set), which removes the interleaving entirely.

mod common;

use common::{c_so, capture, rust_so, Rng, SEED};
use libloading::{Library, Symbol};
use std::os::raw::c_int;
use std::sync::OnceLock;

type DriverFn = unsafe extern "C" fn(c_int, c_int);
type MainFn = unsafe extern "C" fn() -> c_int;

const CHILD_ENV: &str = "DRIVER_FFI_ROW";

/// Re-execute this test binary so that `row` runs alone, with nothing else
/// touching file descriptor 1.
fn run_isolated(row: &str) {
    let exe = std::env::current_exe().expect("current_exe");
    let out = std::process::Command::new(&exe)
        .args(["--exact", row, "--test-threads=1", "--nocapture"])
        .env(CHILD_ENV, row)
        .output()
        .unwrap_or_else(|e| panic!("re-exec {exe:?} for row {row}: {e}"));
    assert!(
        out.status.success(),
        "isolated FFI row `{row}` failed (status {:?}):\n--- stdout ---\n{}\n--- stderr ---\n{}",
        out.status,
        String::from_utf8_lossy(&out.stdout),
        String::from_utf8_lossy(&out.stderr)
    );
    // Guard against the child silently doing nothing (e.g. a filter typo that
    // matches zero tests would still exit 0).
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("1 passed"),
        "isolated FFI row `{row}` did not actually run:\n{stdout}"
    );
}

/// Declares a test whose body only ever executes inside the isolated child.
macro_rules! ffi_test {
    ($name:ident, $body:block) => {
        #[test]
        fn $name() {
            let row = stringify!($name);
            match std::env::var(CHILD_ENV) {
                Ok(v) if v == row => $body,
                _ => run_isolated(row),
            }
        }
    };
}

struct Libs {
    c: Library,
    rust: Library,
}

fn libs() -> &'static Libs {
    static LIBS: OnceLock<Libs> = OnceLock::new();
    LIBS.get_or_init(|| {
        // SAFETY: both objects are built from sources in this repository by this
        // crate's build script and by Cargo.
        let c =
            unsafe { Library::new(c_so()) }.unwrap_or_else(|e| panic!("dlopen {:?}: {e}", c_so()));
        let rust = unsafe { Library::new(rust_so()) }
            .unwrap_or_else(|e| panic!("dlopen {:?}: {e}", rust_so()));
        Libs { c, rust }
    })
}

fn c_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().c.get(b"driver\0") }.expect("C .so must export `driver`")
}

fn rust_driver() -> Symbol<'static, DriverFn> {
    unsafe { libs().rust.get(b"driver\0") }.expect("Rust .so must export `driver`")
}

/// Call `driver` once per pair inside a single captured batch.
///
/// Batching doubles as a check that repeated calls leave no state behind
/// (CONFIGS row 5).
fn drive_all(f: &Symbol<'static, DriverFn>, pairs: &[(i32, i32)]) -> Vec<u8> {
    capture(None, || {
        for &(x, y) in pairs {
            unsafe { f(x, y) };
        }
    })
}

/// Compare the C and Rust `driver` exports over `pairs`, reporting the first
/// divergent pair rather than dumping the whole stream.
fn assert_driver_matches(row: &str, pairs: &[(i32, i32)]) {
    let c = drive_all(&c_driver(), pairs);
    let r = drive_all(&rust_driver(), pairs);

    if c != r {
        let cl: Vec<&[u8]> = c.split(|&b| b == b'\n').collect();
        let rl: Vec<&[u8]> = r.split(|&b| b == b'\n').collect();
        for (i, (a, b)) in cl.iter().zip(rl.iter()).enumerate() {
            if a != b {
                let (x, y) = pairs.get(i).copied().unwrap_or((0, 0));
                panic!(
                    "[{row}] driver({x}, {y}) diverged (call #{i}): C={:?} Rust={:?}",
                    String::from_utf8_lossy(a),
                    String::from_utf8_lossy(b)
                );
            }
        }
        panic!(
            "[{row}] driver output lengths differ: C={} bytes, Rust={} bytes",
            c.len(),
            r.len()
        );
    }

    // Sanity: the batch really produced one line per call, so "both sides
    // produced nothing" cannot masquerade as a pass.
    assert!(!c.is_empty(), "[{row}] captured no output at all");
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        pairs.len(),
        "[{row}] expected exactly one newline per driver() call"
    );
}

const BOUNDARY: [i32; 9] = [
    0,
    1,
    -1,
    2,
    -2,
    i32::MIN,
    i32::MAX,
    0x5555_5555u32 as i32,
    0xAAAA_AAAAu32 as i32,
];

// CONFIGS row 1 - full cross product of the boundary value set.
ffi_test!(cfg01_driver_boundary_cross_product, {
    let mut pairs = Vec::new();
    for &x in &BOUNDARY {
        for &y in &BOUNDARY {
            pairs.push((x, y));
        }
    }
    assert_eq!(pairs.len(), 81);
    assert_driver_matches("cfg01", &pairs);
});

// CONFIGS row 2 - randomized sweep of the whole i32 x i32 space.
ffi_test!(cfg02_driver_randomized, {
    let mut rng = Rng::new(SEED);
    let pairs: Vec<(i32, i32)> = (0..4000).map(|_| (rng.i32v(), rng.i32v())).collect();
    assert_driver_matches("cfg02", &pairs);
});

// CONFIGS row 3 - correlated pairs that make the result degenerate.
ffi_test!(cfg03_driver_correlated, {
    let mut rng = Rng::new(SEED ^ 0x3333);
    let mut pairs = Vec::new();
    for _ in 0..400 {
        let x = rng.i32v();
        pairs.push((x, 0)); // x | ~0  == x | -1 == -1
        pairs.push((x, -1)); // x | ~-1 == x | 0  == x
        pairs.push((x, x));
        pairs.push((x, !x));
        pairs.push((x, x.wrapping_neg()));
    }
    assert_driver_matches("cfg03", &pairs);
});

// CONFIGS row 4 - every single-bit x against every single-bit y.
ffi_test!(cfg04_driver_single_bits, {
    let mut pairs = Vec::new();
    for i in 0..32 {
        for j in 0..32 {
            pairs.push((1i32 << i, 1i32 << j));
        }
    }
    assert_eq!(pairs.len(), 1024);
    assert_driver_matches("cfg04", &pairs);
});

// CONFIGS row 5 - many back-to-back calls in one process.
ffi_test!(cfg05_driver_repeated_calls, {
    let mut rng = Rng::new(SEED ^ 0x5555);
    let pairs: Vec<(i32, i32)> = (0..1000).map(|_| (rng.i32v(), rng.i32v())).collect();
    assert_driver_matches("cfg05", &pairs);
});

// ERRORS row 19 - extreme in-range values; `driver` has no rejection path.
ffi_test!(err19_driver_extremes, {
    let pairs = [
        (i32::MIN, i32::MIN),
        (i32::MIN, i32::MAX),
        (i32::MAX, i32::MIN),
        (i32::MAX, i32::MAX),
    ];
    assert_driver_matches("err19", &pairs);

    // Spell out what C computes, so "both sides broken the same way" cannot pass.
    let out = drive_all(&c_driver(), &pairs);
    assert_eq!(
        String::from_utf8_lossy(&out),
        format!(
            "{}\n{}\n{}\n{}\n",
            i32::MIN | !i32::MIN, // -1
            i32::MIN | !i32::MAX, // INT_MIN
            i32::MAX | !i32::MIN, // INT_MAX
            i32::MAX | !i32::MAX, // -1
        )
    );
});

// ERRORS row 20 - the identity (`y == -1`) and absorbing (`y == 0`) cases.
ffi_test!(err20_driver_identity_absorbing, {
    let mut rng = Rng::new(SEED ^ 0x2020);
    let mut pairs = Vec::new();
    for _ in 0..300 {
        let x = rng.i32v();
        pairs.push((x, -1));
        pairs.push((x, 0));
    }
    assert_driver_matches("err20", &pairs);

    let out = drive_all(&c_driver(), &[(12345, -1), (12345, 0)]);
    assert_eq!(String::from_utf8_lossy(&out), "12345\n-1\n");
});

// ERRORS row 21 - the UB-adjacent bitwise edges (`~INT_MIN`, mixed signs).
ffi_test!(err21_driver_bitwise_edges, {
    let pairs = [
        (0, i32::MIN),
        (0, i32::MAX),
        (i32::MIN, 0),
        (i32::MAX, 0),
        (-1, i32::MIN),
        (1, i32::MIN),
        (i32::MIN, 1),
        (i32::MIN, -1),
    ];
    assert_driver_matches("err21", &pairs);

    // ~INT_MIN == INT_MAX, so 0 | ~INT_MIN == INT_MAX.
    let out = drive_all(&c_driver(), &[(0, i32::MIN)]);
    assert_eq!(String::from_utf8_lossy(&out), format!("{}\n", i32::MAX));
});

// ---------------------------------------------------------------------------
// CONFIGS row 6 / ERRORS row 22 - the exported `main` symbol.
//
// `main` reads the process's stdin, and neither the C library's `FILE *stdin`
// nor Rust's `std::io::stdin()` can be reset from the outside, so each symbol
// may only be exercised once per process. One subprocess per input it is.
// ---------------------------------------------------------------------------

const CASE_ENV: &str = "DRIVER_FFI_MAIN_CASE";

fn c_main_sym() -> Symbol<'static, MainFn> {
    unsafe { libs().c.get(b"main\0") }.expect("C .so must export `main`")
}

fn rust_main_sym() -> Symbol<'static, MainFn> {
    unsafe { libs().rust.get(b"main\0") }.expect("Rust .so must export `main`")
}

#[test]
fn err22_ffi_main_symbol() {
    if let Ok(hex) = std::env::var(CASE_ENV) {
        // Child role: one input, one call into each `.so`.
        let input = decode_hex(&hex);

        let c_out = capture(Some(&input), || {
            let rc = unsafe { c_main_sym()() };
            assert_eq!(rc, 0, "C main must return 0");
        });
        let r_out = capture(Some(&input), || {
            let rc = unsafe { rust_main_sym()() };
            assert_eq!(rc, 0, "Rust main must return 0");
        });

        assert_eq!(
            c_out,
            r_out,
            "FFI `main` diverged for stdin {:?}: C={:?} Rust={:?}",
            String::from_utf8_lossy(&input),
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert!(
            c_out.ends_with(b"\n"),
            "FFI `main` produced no terminated line: {c_out:?}"
        );
        return;
    }

    // Parent role: fan out over several inputs, one subprocess each.
    let mut rng = Rng::new(SEED ^ 0x2222);
    let mut cases: Vec<Vec<u8>> = vec![
        b"5 7".to_vec(),
        b"".to_vec(),
        b"   \t\n -12345   987654\n".to_vec(),
        b"abc".to_vec(),
        b"9223372036854775808 -9223372036854775809".to_vec(),
        b"-".to_vec(),
        b"--5 3".to_vec(),
    ];
    for _ in 0..5 {
        cases.push(format!("{} {}", rng.i32v(), rng.i32v()).into_bytes());
    }

    let exe = std::env::current_exe().expect("current_exe");
    for input in &cases {
        let out = std::process::Command::new(&exe)
            .args([
                "--exact",
                "err22_ffi_main_symbol",
                "--test-threads=1",
                "--nocapture",
            ])
            .env(CASE_ENV, encode_hex(input))
            .output()
            .expect("re-exec self");
        assert!(
            out.status.success(),
            "FFI `main` case {:?} failed:\n{}\n{}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        );
        assert!(
            String::from_utf8_lossy(&out.stdout).contains("1 passed"),
            "FFI `main` case {:?} did not run",
            String::from_utf8_lossy(input)
        );
    }
}

fn encode_hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}

fn decode_hex(s: &str) -> Vec<u8> {
    s.as_bytes()
        .chunks(2)
        .map(|p| u8::from_str_radix(std::str::from_utf8(p).unwrap(), 16).unwrap())
        .collect()
}
