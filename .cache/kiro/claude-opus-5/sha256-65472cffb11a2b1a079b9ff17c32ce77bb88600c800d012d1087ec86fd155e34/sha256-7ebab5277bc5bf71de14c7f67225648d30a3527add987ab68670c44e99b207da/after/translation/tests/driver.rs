//! Top of the API: `void driver(const char *in)`, the only function declared
//! in driver.h. It has no return value, so its entire observable behaviour is
//! the bytes it prints to stdout; the tests compare those byte-for-byte.
//!
//! Comparing stdout requires redirecting fd 1, which is process-wide. This
//! file therefore contains exactly one `#[test]`, so the test harness never
//! writes to stdout from another thread while a capture is in progress.

mod common;

use common::{Libs, Rng, capture_stdout, cstr};

/// Runs `driver` from both `.so`s on the same input and returns the stdout
/// bytes after asserting the two are identical.
fn check(libs: &Libs, bytes: &[u8]) -> Vec<u8> {
    let (c_driver, rust_driver) = libs.driver();
    let buf = cstr(bytes);

    let out_c = capture_stdout(|| unsafe { c_driver(buf.as_ptr()) });
    let out_rust = capture_stdout(|| unsafe { rust_driver(buf.as_ptr()) });

    assert_eq!(
        out_c,
        out_rust,
        "driver({:?}) stdout differs:\n  C   : {:?}\n  Rust: {:?}",
        String::from_utf8_lossy(bytes),
        String::from_utf8_lossy(&out_c),
        String::from_utf8_lossy(&out_rust)
    );
    out_c
}

fn expected(bytes: &[u8]) -> Vec<u8> {
    let a = bytes.iter().filter(|&&b| b == b'A').count();
    let x = bytes.iter().filter(|&&b| b == b'x').count();
    format!("A: {a}\nx: {x}\n").into_bytes()
}

/// Asserts C and Rust agree, and additionally that both match the independently
/// computed expectation.
fn check_exact(libs: &Libs, bytes: &[u8]) {
    let out = check(libs, bytes);
    assert_eq!(
        out,
        expected(bytes),
        "driver({:?}) printed {:?}",
        String::from_utf8_lossy(bytes),
        String::from_utf8_lossy(&out)
    );
}

#[test]
fn driver_stdout_matches_c() {
    let libs = Libs::load();

    // --- empty input -----------------------------------------------------
    assert_eq!(check(&libs, b""), b"A: 0\nx: 0\n");

    // --- small hand-checked cases ---------------------------------------
    let cases: &[(&[u8], &[u8])] = &[
        (b"A", b"A: 1\nx: 0\n"),
        (b"x", b"A: 0\nx: 1\n"),
        (b"Ax", b"A: 1\nx: 1\n"),
        (b"xA", b"A: 1\nx: 1\n"),
        (b"AAxxx", b"A: 2\nx: 3\n"),
        (b"aXaX", b"A: 0\nx: 0\n"),
        (b"The quick brown fox", b"A: 0\nx: 1\n"),
        (b"AAAAAAAAAA", b"A: 10\nx: 0\n"),
        (b" A x ", b"A: 1\nx: 1\n"),
    ];
    for (input, want) in cases {
        let out = check(&libs, input);
        assert_eq!(
            out,
            *want,
            "driver({:?}) printed {:?}",
            String::from_utf8_lossy(input),
            String::from_utf8_lossy(&out)
        );
    }

    // --- non-ASCII / control bytes --------------------------------------
    let byte_cases: Vec<Vec<u8>> = vec![
        vec![0xff, 0x80, b'A', 0x01, b'x'],
        vec![0x0a, 0x0d, 0x09, b'A'],
        // UTF-8 multibyte sequences, opaque bytes as far as the C code cares.
        "héllo wörld — Ax".as_bytes().to_vec(),
        "日本語 A x".as_bytes().to_vec(),
        // Every non-NUL byte value exactly once.
        (1u16..=255).map(|b| b as u8).collect(),
    ];
    for case in &byte_cases {
        check_exact(&libs, case);
    }

    // --- multi-digit counts, exercising "%d" formatting -----------------
    for n in [9usize, 10, 99, 100, 999, 1000, 12345] {
        let mut buf = vec![b'A'; n];
        buf.extend(std::iter::repeat_n(b'x', n + 1));
        let out = check(&libs, &buf);
        assert_eq!(out, format!("A: {n}\nx: {}\n", n + 1).into_bytes());
    }

    // --- lengths around word/vector boundaries --------------------------
    for len in [1usize, 7, 8, 15, 16, 17, 31, 32, 33, 63, 64, 65, 4096] {
        check_exact(&libs, &vec![b'A'; len]);
        check_exact(&libs, &vec![b'x'; len]);
        let mut mixed: Vec<u8> = vec![b'.'; len];
        mixed[0] = b'A';
        mixed[len - 1] = b'x';
        check_exact(&libs, &mixed);
    }

    // --- randomised ------------------------------------------------------
    let mut rng = Rng::new(0x1357_9bdf_0246_8ace);
    for _ in 0..400 {
        let len = rng.below(300);
        let bytes: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'A',
                1 => b'x',
                2 => b'a' + rng.below(26) as u8,
                _ => rng.nonzero_byte(),
            })
            .collect();
        check_exact(&libs, &bytes);
    }

    // --- repeated calls stay stable -------------------------------------
    let repeat_input = b"AAAxxAx";
    let first = check(&libs, repeat_input);
    for _ in 0..20 {
        assert_eq!(check(&libs, repeat_input), first);
    }
}
