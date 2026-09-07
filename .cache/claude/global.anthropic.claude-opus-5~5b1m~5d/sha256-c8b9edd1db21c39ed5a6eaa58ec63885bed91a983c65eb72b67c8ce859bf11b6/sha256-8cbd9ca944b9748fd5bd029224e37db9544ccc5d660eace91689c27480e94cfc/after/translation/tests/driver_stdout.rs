// Phase B - stdout differential tests for `driver`.
//
// `driver` writes to stdout with libc `printf`, so verifying it means comparing
// the raw bytes that land on file descriptor 1. fd 1 is process global, which is
// why this target runs WITHOUT the libtest harness (`harness = false` in
// Cargo.toml): tests execute sequentially and every progress message goes to
// stderr, so nothing but the library's own output can end up in a capture.
//
// Covers CONFIGS.md rows 15-22 and the `driver` half of ERRORS.md rows 12-13.
// Both libraries are loaded via `libloading`; no Rust function is called directly.

mod common;
use common::*;

fn main() {
    let mut failures: Vec<String> = Vec::new();

    run(&mut failures, "cfg_15_driver_empty", cfg_15_driver_empty);
    run(&mut failures, "cfg_16_driver_only_A", cfg_16_driver_only_a);
    run(&mut failures, "cfg_17_driver_only_x", cfg_17_driver_only_x);
    run(&mut failures, "cfg_18_driver_both_needles_interleaved", cfg_18_driver_both);
    run(&mut failures, "cfg_19_driver_neither_needle", cfg_19_driver_neither);
    run(&mut failures, "cfg_20_driver_random_full_byte_range", cfg_20_driver_random);
    run(&mut failures, "cfg_21_driver_long_haystack_multidigit", cfg_21_driver_long);
    run(&mut failures, "cfg_22_cross_library_pipeline_consistency", cfg_22_cross_library);
    run(&mut failures, "err_12_driver_invalid_utf8", err_12_driver_invalid_utf8);
    run(&mut failures, "err_13_driver_format_specifiers", err_13_driver_format_specifiers);
    run(&mut failures, "err_10_driver_repeated_calls_are_stable", err_10_repeated_calls);

    eprintln!();
    if failures.is_empty() {
        eprintln!("driver_stdout: all 11 stdout differential tests passed");
    } else {
        eprintln!("driver_stdout: {} FAILED:", failures.len());
        for f in &failures {
            eprintln!("  - {f}");
        }
        std::process::exit(1);
    }
}

/// Run one test, catching a panic so the remaining tests still execute.
fn run(failures: &mut Vec<String>, name: &str, f: fn()) {
    eprint!("test {name} ... ");
    let res = std::panic::catch_unwind(std::panic::AssertUnwindSafe(f));
    match res {
        Ok(()) => eprintln!("ok"),
        Err(_) => {
            eprintln!("FAILED");
            failures.push(name.to_string());
        }
    }
}

// ---------------------------------------------------------------------------
// Row 15: empty haystack.
// ---------------------------------------------------------------------------
fn cfg_15_driver_empty() {
    let out = diff_driver(b"", "row15 empty");
    assert_eq!(out, b"A: 0\nx: 0\n", "got {:?}", String::from_utf8_lossy(&out));
}

// ---------------------------------------------------------------------------
// Row 16: only 'A's.
// ---------------------------------------------------------------------------
fn cfg_16_driver_only_a() {
    let mut rng = Rng::new(0x16_41);
    for _ in 0..200 {
        let n = rng.range(1, 200);
        let out = diff_driver(&vec![b'A'; n], "row16 only A");
        assert_eq!(out, format!("A: {n}\nx: 0\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 17: only 'x's.
// ---------------------------------------------------------------------------
fn cfg_17_driver_only_x() {
    let mut rng = Rng::new(0x17_78);
    for _ in 0..200 {
        let n = rng.range(1, 200);
        let out = diff_driver(&vec![b'x'; n], "row17 only x");
        assert_eq!(out, format!("A: 0\nx: {n}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 18: both needles interleaved; the exact bytes pin the order of the two
// printf lines as well as the counts.
// ---------------------------------------------------------------------------
fn cfg_18_driver_both() {
    let mut rng = Rng::new(0x18_00);
    for i in 0..300 {
        let len = rng.range(2, 300);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => b'A',
                1 => b'x',
                _ => b'.',
            })
            .collect();
        let out = diff_driver(&hay, &format!("row18 iter {i}"));
        let a = count_bytes(&hay, b'A');
        let x = count_bytes(&hay, b'x');
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
    // Deterministic extremes: needle only at the very start / very end.
    for hay in [&b"Ax......."[..], &b".......Ax"[..], &b"xA"[..], &b"Ax"[..]] {
        let out = diff_driver(hay, "row18 extremes");
        assert_eq!(out, b"A: 1\nx: 1\n");
    }
}

// ---------------------------------------------------------------------------
// Row 19: neither needle present.
// ---------------------------------------------------------------------------
fn cfg_19_driver_neither() {
    let mut rng = Rng::new(0x19_00);
    let alphabet: Vec<u8> = (1u8..=255).filter(|&b| b != b'A' && b != b'x').collect();
    for i in 0..300 {
        let len = rng.range(0, 256);
        let hay = rng.bytes_from(&alphabet, len);
        let out = diff_driver(&hay, &format!("row19 iter {i}"));
        assert_eq!(out, b"A: 0\nx: 0\n");
    }
}

// ---------------------------------------------------------------------------
// Row 20: fully random full-byte-range haystacks, byte-for-byte stdout.
// ---------------------------------------------------------------------------
fn cfg_20_driver_random() {
    let mut rng = Rng::new(0x20_00);
    for i in 0..200 {
        let len = rng.range(0, 512);
        let hay = rng.nonzero_bytes(len);
        let out = diff_driver(&hay, &format!("row20 iter {i}"));
        let a = count_bytes(&hay, b'A');
        let x = count_bytes(&hay, b'x');
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 21: 64 KiB haystack -> multi-digit printf formatting.
// ---------------------------------------------------------------------------
fn cfg_21_driver_long() {
    let mut rng = Rng::new(0x21_00);
    for _ in 0..5 {
        let len = 65536;
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 | 2 => b'A',
                1 => b'x',
                _ => b'.',
            })
            .collect();
        let out = diff_driver(&hay, "row21 64KiB");
        let a = count_bytes(&hay, b'A');
        let x = count_bytes(&hay, b'x');
        assert!(a > 9999 && x > 999, "expected multi-digit counts: a={a} x={x}");
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// Row 22: composed-pipeline cross-check. Each library's printed numbers are
// checked against the OTHER library's `foo`, so a bug that is self-consistent
// within one library is still caught.
// ---------------------------------------------------------------------------
fn cfg_22_cross_library() {
    let mut rng = Rng::new(0x22_00);
    for i in 0..300 {
        let len = rng.range(0, 256);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(4) {
                0 => b'A',
                1 => b'x',
                _ => rng.nonzero_byte(),
            })
            .collect();
        let s = cstr(&hay);

        let c_out = driver_stdout(c_lib(), &hay);
        let r_out = driver_stdout(rust_lib(), &hay);

        let c_a = unsafe { (c_lib().foo())(s.as_ptr(), b'A' as i8) };
        let c_x = unsafe { (c_lib().foo())(s.as_ptr(), b'x' as i8) };
        let r_a = unsafe { (rust_lib().foo())(s.as_ptr(), b'A' as i8) };
        let r_x = unsafe { (rust_lib().foo())(s.as_ptr(), b'x' as i8) };

        assert_eq!(
            c_out,
            format!("A: {r_a}\nx: {r_x}\n").into_bytes(),
            "row22 iter {i}: C driver stdout disagrees with Rust foo"
        );
        assert_eq!(
            r_out,
            format!("A: {c_a}\nx: {c_x}\n").into_bytes(),
            "row22 iter {i}: Rust driver stdout disagrees with C foo"
        );
        assert_eq!(c_out, r_out, "row22 iter {i}: stdout differs");
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 12 (driver half): invalid UTF-8 is just bytes.
// ---------------------------------------------------------------------------
fn err_12_driver_invalid_utf8() {
    let cases: &[&[u8]] = &[
        b"\x80",
        b"\xbf\xbf\xbf",
        b"\xc3",
        b"\xc3\x28",
        b"\xe2\x82",
        b"\xf0\x9f\x92",
        b"\xf8\xa1\xa1\xa1\xa1",
        b"\xc0\xaf",
        b"\xed\xa0\x80",
        b"A\xffx\xfeA\xfdx",
        b"\xff\xfe\xfd\xfc\xfb\xfa",
    ];
    for hay in cases {
        let out = diff_driver(hay, "err12 driver invalid utf8");
        let a = count_bytes(hay, b'A');
        let x = count_bytes(hay, b'x');
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
    let mut rng = Rng::new(0xE0_12);
    for i in 0..500 {
        let len = rng.range(1, 64);
        let hay: Vec<u8> = (0..len).map(|_| rng.range(0x80, 0xff) as u8).collect();
        diff_driver(&hay, &format!("err12 driver rand {i}"));
    }
    // High-bit bytes mixed with the two needles.
    for i in 0..500 {
        let len = rng.range(1, 64);
        let hay: Vec<u8> = (0..len)
            .map(|_| match rng.below(3) {
                0 => b'A',
                1 => b'x',
                _ => rng.range(0x80, 0xff) as u8,
            })
            .collect();
        let out = diff_driver(&hay, &format!("err12 driver mixed {i}"));
        let a = count_bytes(&hay, b'A');
        let x = count_bytes(&hay, b'x');
        assert_eq!(out, format!("A: {a}\nx: {x}\n").into_bytes());
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 13: `%`-bearing input is data, never a format string.
// ---------------------------------------------------------------------------
fn err_13_driver_format_specifiers() {
    let cases: &[&[u8]] = &[
        b"%s",
        b"%n",
        b"%d %d %d %d %d",
        b"%s%s%s%s%s%s%s%s",
        b"%n%n%n%n",
        b"A%sx%nA",
        b"%%%%",
        b"%1000000d",
        b"%.999999f",
        b"%p %p %p",
        b"A: %d\nx: %d\n",
        b"%hn%hhn%lln",
        b"%*d",
        b"%99999999999999999999d",
    ];
    for hay in cases {
        let out = diff_driver(hay, "err13 format specifiers");
        let a = count_bytes(hay, b'A');
        let x = count_bytes(hay, b'x');
        assert_eq!(
            out,
            format!("A: {a}\nx: {x}\n").into_bytes(),
            "input must not be interpreted as a format string: {:?}",
            Preview(hay)
        );
    }
}

// ---------------------------------------------------------------------------
// ERRORS row 10 support: `driver` holds no state, so repeated calls with the
// same and with alternating inputs must keep producing identical output in both
// libraries (catches a translation that memoised or mutated globals).
// ---------------------------------------------------------------------------
fn err_10_repeated_calls() {
    let a = b"AAxx";
    let b = b"xxxAAAAA";
    for _ in 0..50 {
        assert_eq!(diff_driver(a, "err10 repeat a"), b"A: 2\nx: 2\n");
        assert_eq!(diff_driver(b, "err10 repeat b"), b"A: 5\nx: 3\n");
        assert_eq!(diff_driver(b"", "err10 repeat empty"), b"A: 0\nx: 0\n");
    }
}
