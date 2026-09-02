//! Phase B — valid-path differential tests.
//!
//! One test per row of `CONFIGS.md`. Every test loads both the C `.so` and the
//! Rust `.so` via `libloading` and compares return value **and** stdout bytes.

mod common;

use common::*;
use std::ffi::c_int;

// ---------------------------------------------------------------------------
// Row 1 — all args in the `default` class, randomised in [-1000, 1000]
// ---------------------------------------------------------------------------

#[test]
fn row01_default_class_small_random() {
    let mut rng = Rng::new(SEED ^ 1);
    for _ in 0..2_000 {
        // Redraw until the value is not a switch label, so this row really
        // exercises only the `default` arm.
        let pick = |rng: &mut Rng| loop {
            let v = rng.range_i32(-1000, 1000);
            if !LABELS.contains(&v) {
                return v;
            }
        };
        let (a, b, c, d) = (
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
            pick(&mut rng),
        );
        assert_cleanup(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 2 — exhaustive cross product of the 5 switch classes over 4 positions
// ---------------------------------------------------------------------------

/// Class 0..=3 -> the literal labels; class 4 -> a randomised `default` value.
fn class_value(class: usize, rng: &mut Rng) -> c_int {
    if class < 4 {
        LABELS[class]
    } else {
        loop {
            let v = rng.next_i32();
            if !LABELS.contains(&v) {
                return v;
            }
        }
    }
}

#[test]
fn row02_exhaustive_class_cross_product() {
    let mut rng = Rng::new(SEED ^ 2);
    let mut cases = 0usize;
    for ca in 0..5 {
        for cb in 0..5 {
            for cc in 0..5 {
                for cd in 0..5 {
                    let a = class_value(ca, &mut rng);
                    let b = class_value(cb, &mut rng);
                    let c = class_value(cc, &mut rng);
                    let d = class_value(cd, &mut rng);
                    assert_cleanup(a, b, c, d);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 625, "cross product must be 5^4");
}

// ---------------------------------------------------------------------------
// Rows 3-6 — exactly one label argument, in each of the 4 positions
// ---------------------------------------------------------------------------

fn one_label_in_each_position(label: c_int, seed_salt: u64) {
    let mut rng = Rng::new(SEED ^ seed_salt);
    for pos in 0..4usize {
        for _ in 0..250 {
            let mut args = [0i32; 4];
            for (i, slot) in args.iter_mut().enumerate() {
                *slot = if i == pos {
                    label
                } else {
                    class_value(4, &mut rng)
                };
            }
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
    }
}

#[test]
fn row03_single_10_fallthrough_into_20() {
    one_label_in_each_position(10, 3);
}

#[test]
fn row04_single_20() {
    one_label_in_each_position(20, 4);
}

#[test]
fn row05_single_30_fallthrough_into_40() {
    one_label_in_each_position(30, 5);
}

#[test]
fn row06_single_40() {
    one_label_in_each_position(40, 6);
}

// ---------------------------------------------------------------------------
// Rows 7-8 — all four arguments the same label
// ---------------------------------------------------------------------------

#[test]
fn row07_all_ten_maximum_fallthrough() {
    assert_cleanup(10, 10, 10, 10);
}

#[test]
fn row08_all_same_label() {
    for l in LABELS {
        assert_cleanup(l, l, l, l);
    }
}

// ---------------------------------------------------------------------------
// Rows 9-11 — extremes and the zero shape
// ---------------------------------------------------------------------------

#[test]
fn row09_int_max_in_each_position() {
    let mut rng = Rng::new(SEED ^ 9);
    for pos in 0..4usize {
        let mut args = [0i32; 4];
        args[pos] = i32::MAX;
        assert_cleanup(args[0], args[1], args[2], args[3]);

        // ...and with the other slots randomised, to force wrap-around sums.
        for _ in 0..250 {
            let mut args = [0i32; 4];
            for (i, slot) in args.iter_mut().enumerate() {
                *slot = if i == pos { i32::MAX } else { rng.next_i32() };
            }
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
    }
    assert_cleanup(i32::MAX, i32::MAX, i32::MAX, i32::MAX);
    assert_cleanup(i32::MAX, 1, 0, 0);
    assert_cleanup(i32::MAX, i32::MAX, 10, 30);
}

#[test]
fn row10_int_min_in_each_position() {
    let mut rng = Rng::new(SEED ^ 10);
    for pos in 0..4usize {
        let mut args = [0i32; 4];
        args[pos] = i32::MIN;
        assert_cleanup(args[0], args[1], args[2], args[3]);

        for _ in 0..250 {
            let mut args = [0i32; 4];
            for (i, slot) in args.iter_mut().enumerate() {
                *slot = if i == pos { i32::MIN } else { rng.next_i32() };
            }
            assert_cleanup(args[0], args[1], args[2], args[3]);
        }
    }
    assert_cleanup(i32::MIN, i32::MIN, i32::MIN, i32::MIN);
    assert_cleanup(i32::MIN, -1, 0, 0);
    assert_cleanup(i32::MIN, i32::MIN, 10, 30);
}

#[test]
fn row11_all_zero() {
    assert_cleanup(0, 0, 0, 0);
}

// ---------------------------------------------------------------------------
// Row 12 — all-negative arguments over the whole negative range
// ---------------------------------------------------------------------------

#[test]
fn row12_all_negative_random() {
    let mut rng = Rng::new(SEED ^ 12);
    for _ in 0..2_000 {
        let a = rng.range_i32(i32::MIN, -1);
        let b = rng.range_i32(i32::MIN, -1);
        let c = rng.range_i32(i32::MIN, -1);
        let d = rng.range_i32(i32::MIN, -1);
        assert_cleanup(a, b, c, d);
    }
}

// ---------------------------------------------------------------------------
// Row 13 — exhaustive off-by-one neighbours of every switch label (8^4)
// ---------------------------------------------------------------------------

#[test]
fn row13_exhaustive_label_neighbours() {
    const NEIGHBOURS: [c_int; 8] = [9, 11, 19, 21, 29, 31, 39, 41];
    let mut cases = 0usize;
    for a in NEIGHBOURS {
        for b in NEIGHBOURS {
            for c in NEIGHBOURS {
                for d in NEIGHBOURS {
                    assert_cleanup(a, b, c, d);
                    cases += 1;
                }
            }
        }
    }
    assert_eq!(cases, 4096, "neighbour cross product must be 8^4");
}

// ---------------------------------------------------------------------------
// Row 14 — full-range i32 fuzzing (also covers the "out-of-range variant"
// class: every int outside {10,20,30,40} takes the `default` arm)
// ---------------------------------------------------------------------------

#[test]
fn row14_full_range_random() {
    let mut rng = Rng::new(SEED ^ 14);
    for _ in 0..20_000 {
        assert_cleanup(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32(),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 15 — biased sampler mixing labels and default values in one call
// ---------------------------------------------------------------------------

#[test]
fn row15_biased_mixed_random() {
    let mut rng = Rng::new(SEED ^ 15);
    for _ in 0..20_000 {
        assert_cleanup(
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
    }
}

// ---------------------------------------------------------------------------
// Row 16 — stdout side effect of `cleanup` is exactly the expected line, and
// neither diagnostic branch is ever reached in either library
// ---------------------------------------------------------------------------

#[test]
fn row16_cleanup_stdout_side_effect() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 16);
    let _g = stdout_guard();

    // `TO_STRING(numbers)` stringizes the macro argument, so the C prints the
    // literal text `numbers`, not the array contents.
    let expected: &[u8] = b"Processed numbers: numbers\n";

    for _ in 0..500 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        let (c_ret, c_out) = capture(|| unsafe { (p.c.cleanup)(a, b, c, d) });
        let (r_ret, r_out) = capture(|| unsafe { (p.rust.cleanup)(a, b, c, d) });

        assert_eq!(c_ret, r_ret, "return mismatch for cleanup({a},{b},{c},{d})");
        assert_eq!(
            c_out, r_out,
            "stdout mismatch for cleanup({a},{b},{c},{d}):\n C   : {:?}\n Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
        assert_eq!(
            c_out, expected,
            "C stdout is not the expected success line: {:?}",
            String::from_utf8_lossy(&c_out)
        );
        // Neither rejection diagnostic is reachable through the FFI boundary.
        for out in [&c_out, &r_out] {
            assert!(!contains(out, b"Input string validation failed."));
            assert!(!contains(out, b"Memory allocation failed."));
        }
    }
}

fn contains(hay: &[u8], needle: &[u8]) -> bool {
    hay.windows(needle.len()).any(|w| w == needle)
}

// ---------------------------------------------------------------------------
// Rows 17-22 — `print_result`, driven directly as a low-level entry point
// ---------------------------------------------------------------------------

#[test]
fn row17_print_result_ascii_label() {
    for r in [0, 1, -1, 42, i32::MAX, i32::MIN] {
        assert_print_result("total", r);
        assert_print_result("Result", r);
        assert_print_result("a", r);
    }
    let mut rng = Rng::new(SEED ^ 17);
    for _ in 0..2_000 {
        assert_print_result("label", rng.next_i32());
    }
}

#[test]
fn row18_print_result_empty_label() {
    let mut rng = Rng::new(SEED ^ 18);
    assert_print_result("", 0);
    for _ in 0..500 {
        assert_print_result("", rng.next_i32());
    }
}

#[test]
fn row19_print_result_oversized_label() {
    let mut rng = Rng::new(SEED ^ 19);
    let big = "x".repeat(64 * 1024);
    assert_print_result(&big, 0);
    for _ in 0..20 {
        assert_print_result(&big, rng.next_i32());
    }
    // A label whose length straddles the 50-byte constant used elsewhere.
    for n in [1usize, 48, 49, 50, 51, 52, 4095, 4096, 4097] {
        assert_print_result(&"z".repeat(n), rng.next_i32());
    }
}

#[test]
fn row20_print_result_percent_label() {
    // `label` is a `%s` *argument*, so no format interpretation must occur.
    for label in ["%d", "%s", "%%", "%n", "100%", "%d %s %n %%", "%1$s", "%.*f"] {
        assert_print_result(label, 7);
        assert_print_result(label, i32::MIN);
    }
}

#[test]
fn row21_print_result_non_utf8_label() {
    // Every non-NUL byte value, in one label, must be echoed verbatim.
    let mut bytes: Vec<u8> = (1u8..=255).collect();
    bytes.push(0);
    assert_print_result_raw(&bytes, 123);

    // High bytes only.
    let mut high: Vec<u8> = (0x80u8..=0xFF).collect();
    high.push(0);
    assert_print_result_raw(&high, -123);

    // Lone continuation bytes / truncated sequences.
    for seq in [
        &[0x80u8, 0][..],
        &[0xC3, 0][..],
        &[0xE2, 0x82, 0][..],
        &[0xF0, 0x9F, 0x92, 0][..],
        &[0xFF, 0xFE, 0][..],
    ] {
        assert_print_result_raw(seq, 1);
    }
}

#[test]
fn row22_print_result_null_label() {
    // glibc's `%s` renders a null pointer as `(null)`; both libraries route
    // through the same glibc, so the observable output must be identical.
    assert_same("print_result(NULL, 0)", |imp| unsafe {
        (imp.print_result)(std::ptr::null(), 0);
    });
    for r in [0, 1, -1, i32::MAX, i32::MIN] {
        assert_same(format!("print_result(NULL, {r})"), |imp| unsafe {
            (imp.print_result)(std::ptr::null(), r);
        });
    }
}

// ---------------------------------------------------------------------------
// Rows 23-25 — `cleanup_resources`, driven directly as a low-level entry point
// ---------------------------------------------------------------------------

#[test]
fn row23_cleanup_resources_null() {
    for _ in 0..100 {
        assert_same("cleanup_resources(NULL)", |imp| unsafe {
            (imp.cleanup_resources)(std::ptr::null_mut());
        });
    }
}

#[test]
fn row24_cleanup_resources_live_malloc() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 24);
    let _g = stdout_guard();
    for _ in 0..2_000 {
        let n = rng.range_usize(1, 4096);
        // A fresh allocation per library: the pointer is consumed (freed), so
        // the two calls must never share one.
        let pc = c_malloc(n);
        assert!(!pc.is_null());
        let (_, c_out) = capture(|| unsafe { (p.c.cleanup_resources)(pc) });

        let pr = c_malloc(n);
        assert!(!pr.is_null());
        let (_, r_out) = capture(|| unsafe { (p.rust.cleanup_resources)(pr) });

        assert_eq!(c_out, r_out, "cleanup_resources(malloc({n})) stdout differs");
        assert!(c_out.is_empty(), "cleanup_resources must print nothing");
    }
}

#[test]
fn row25_cleanup_resources_fifty_byte_payload() {
    let p = pair();
    let _g = stdout_guard();
    // Exactly the size and content `cleanup` itself allocates and formats.
    let payload = b"Processed numbers: numbers\0";
    for _ in 0..200 {
        for imp in [&p.c, &p.rust] {
            let buf = c_malloc(50);
            assert!(!buf.is_null());
            unsafe {
                std::ptr::write_bytes(buf, 0, 50);
                std::ptr::copy_nonoverlapping(
                    payload.as_ptr() as *const std::ffi::c_char,
                    buf,
                    payload.len(),
                );
            }
            let (_, out) = capture(|| unsafe { (imp.cleanup_resources)(buf) });
            assert!(out.is_empty(), "{} printed unexpectedly", imp.name);
        }
    }
}

// ---------------------------------------------------------------------------
// Row 26 — composed pipeline across all three entry points
// ---------------------------------------------------------------------------

#[test]
fn row26_composed_pipeline() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 26);
    let _g = stdout_guard();

    for _ in 0..2_000 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        let n = rng.range_usize(1, 128);
        let labels = ["sum", "", "Processed", "%d"];
        let label = std::ffi::CString::new(labels[rng.range_usize(0, 3)]).unwrap();

        // The whole sequence is captured as one stdout region, so any ordering
        // or buffering difference between the libraries shows up here.
        let run = |imp: &Impl| unsafe {
            let r = (imp.cleanup)(a, b, c, d);
            (imp.print_result)(label.as_ptr(), r);
            let buf = c_malloc(n);
            (imp.cleanup_resources)(buf);
            (imp.cleanup_resources)(std::ptr::null_mut());
            r
        };

        let (c_ret, c_out) = capture(|| run(&p.c));
        let (r_ret, r_out) = capture(|| run(&p.rust));
        assert_eq!(c_ret, r_ret, "pipeline return differs for ({a},{b},{c},{d})");
        assert_eq!(
            c_out,
            r_out,
            "pipeline stdout differs for ({a},{b},{c},{d}):\n C   : {:?}\n Rust: {:?}",
            String::from_utf8_lossy(&c_out),
            String::from_utf8_lossy(&r_out)
        );
    }
}

// ---------------------------------------------------------------------------
// Row 27 — repeated invocation on one handle (state leakage / allocator reuse)
// ---------------------------------------------------------------------------

#[test]
fn row27_repeated_invocation() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 27);
    let _g = stdout_guard();

    let mut args = Vec::with_capacity(1_000);
    for _ in 0..1_000 {
        args.push((
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        ));
    }

    // 1000 back-to-back calls into a single library handle, then the same for
    // the other, comparing the full transcript.
    let run = |imp: &Impl| unsafe {
        let mut rets = Vec::with_capacity(args.len());
        for &(a, b, c, d) in &args {
            rets.push((imp.cleanup)(a, b, c, d));
        }
        rets
    };
    let (c_rets, c_out) = capture(|| run(&p.c));
    let (r_rets, r_out) = capture(|| run(&p.rust));
    assert_eq!(c_rets, r_rets, "return transcript differs over 1000 calls");
    assert_eq!(c_out.len(), r_out.len(), "stdout transcript length differs");
    assert_eq!(c_out, r_out, "stdout transcript differs over 1000 calls");
}

// ---------------------------------------------------------------------------
// Row 28 — interleaved libraries inside one captured region
// ---------------------------------------------------------------------------

#[test]
fn row28_interleaved_libraries() {
    let p = pair();
    let mut rng = Rng::new(SEED ^ 28);
    let _g = stdout_guard();

    for _ in 0..500 {
        let (a, b, c, d) = (
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
            biased_i32(&mut rng),
        );
        // C then Rust in one window: the two halves must be byte-identical.
        let ((rc, rr), out) = capture(|| unsafe {
            let rc = (p.c.cleanup)(a, b, c, d);
            let rr = (p.rust.cleanup)(a, b, c, d);
            (rc, rr)
        });
        assert_eq!(rc, rr, "interleaved returns differ for ({a},{b},{c},{d})");
        assert_eq!(
            out.len() % 2,
            0,
            "interleaved stdout has odd length: {:?}",
            String::from_utf8_lossy(&out)
        );
        let (first, second) = out.split_at(out.len() / 2);
        assert_eq!(
            first,
            second,
            "interleaved stdout halves differ: {:?}",
            String::from_utf8_lossy(&out)
        );
    }
}
