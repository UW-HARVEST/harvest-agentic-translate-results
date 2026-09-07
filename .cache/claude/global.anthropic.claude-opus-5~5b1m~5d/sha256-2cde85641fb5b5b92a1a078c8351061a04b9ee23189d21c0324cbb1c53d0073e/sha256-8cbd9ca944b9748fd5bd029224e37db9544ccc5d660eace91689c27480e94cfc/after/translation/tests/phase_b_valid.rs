//! Phase B — valid-path differential tests. One test per CONFIGS.md row.
//!
//! Both libraries are driven exclusively through their `.so` exports
//! (`dlsym`), lowest-level entry points first (`printLine`, `printIntLine`),
//! then the mid-level sinks (`bad`, `good`), then the top-level `driver`
//! wrapper, and finally a randomized interleaved composed pipeline.

mod harness;

use harness::*;
use std::ffi::c_int;

// ===========================================================================
// printLine — CONFIGS rows 1..8
// ===========================================================================

#[test]
fn cfg01_print_line_null() {
    diff_print_line("cfg01 printLine(NULL)", None);
}

#[test]
fn cfg02_print_line_empty() {
    diff_print_line("cfg02 printLine(\"\")", Some(b""));
}

#[test]
fn cfg03_print_line_short_ascii() {
    for s in [
        &b"hello"[..],
        b"a",
        b"Calling good()...",
        b"Finished bad()",
        b"ERROR: Array index is negative.",
        b"ERROR: Array index is out-of-bounds",
    ] {
        diff_print_line("cfg03 printLine(short ascii)", Some(s));
    }
}

#[test]
fn cfg04_print_line_percent_specifiers() {
    // `printf("%s\n", line)` must NOT interpret `line` as a format string.
    for s in [
        &b"%s"[..],
        b"%d",
        b"%s %d %n %%",
        b"100%",
        b"%p%p%p%p%p%p%p%p%p%p",
        b"%99999999d",
        b"%.*s",
        b"%%%%%%",
    ] {
        diff_print_line("cfg04 printLine(format specifiers)", Some(s));
    }
}

#[test]
fn cfg05_print_line_embedded_whitespace() {
    for s in [
        &b"line1\nline2"[..],
        b"\n",
        b"\n\n\n",
        b"tab\there",
        b"cr\rhere",
        b"trailing space ",
        b" leading space",
        b"\x0b\x0c vertical/form feed",
    ] {
        diff_print_line("cfg05 printLine(embedded whitespace)", Some(s));
    }
}

#[test]
fn cfg06_print_line_high_bytes() {
    // Non-UTF-8 payloads: the C API is byte-oriented, the Rust side must not
    // assume UTF-8 anywhere.
    let mut all_high: Vec<u8> = (0x80u8..=0xFFu8).collect();
    diff_print_line("cfg06 printLine(0x80..0xFF)", Some(&all_high));
    all_high.reverse();
    diff_print_line("cfg06 printLine(0xFF..0x80)", Some(&all_high));

    // Every single non-NUL byte value, one at a time.
    for b in 1u8..=255u8 {
        diff_print_line("cfg06 printLine(single byte)", Some(&[b]));
    }

    // Truncated / invalid UTF-8 sequences.
    for s in [
        &b"\xC3"[..],
        b"\xE2\x82",
        b"\xF0\x9F\x92",
        b"\xFF\xFE",
        b"caf\xE9",
    ] {
        diff_print_line("cfg06 printLine(invalid utf8)", Some(s));
    }
}

#[test]
fn cfg07_print_line_64kib() {
    // Oversized: larger than the default 4 KiB/64 KiB stdio and pipe buffers.
    let big = vec![b'Z'; 64 * 1024];
    diff_print_line("cfg07 printLine(64 KiB)", Some(&big));

    let mut mixed = Vec::with_capacity(64 * 1024 + 1);
    for i in 0..(64 * 1024 + 1) {
        // never 0 (would terminate the C string early)
        mixed.push(((i % 255) + 1) as u8);
    }
    diff_print_line("cfg07 printLine(64 KiB + 1, mixed bytes)", Some(&mixed));
}

#[test]
fn cfg08_print_line_randomized() {
    let mut rng = Rng::new(SEED ^ 0x08);
    for iter in 0..1024 {
        let len = rng.below(256) as usize;
        let mut s = Vec::with_capacity(len);
        for _ in 0..len {
            // uniform over 1..=255, so never an interior NUL
            s.push((rng.below(255) + 1) as u8);
        }
        diff_print_line(&format!("cfg08 printLine(random #{iter}, len {len})"), Some(&s));
    }
}

// ===========================================================================
// printIntLine — CONFIGS rows 9..11
// ===========================================================================

#[test]
fn cfg09_print_int_line_zero() {
    diff_print_int_line("cfg09 printIntLine(0)", 0);
}

#[test]
fn cfg10_print_int_line_boundaries() {
    for v in [
        0,
        1,
        -1,
        9,
        10,
        11,
        -9,
        -10,
        99,
        100,
        -100,
        32767,
        -32768,
        65535,
        i32::MAX,
        i32::MIN,
        i32::MAX - 1,
        i32::MIN + 1,
    ] {
        diff_print_int_line(&format!("cfg10 printIntLine({v})"), v);
    }
}

#[test]
fn cfg11_print_int_line_randomized() {
    let mut rng = Rng::new(SEED ^ 0x11);
    for iter in 0..4096 {
        let v = rng.next_i32();
        diff_print_int_line(&format!("cfg11 printIntLine(random #{iter} = {v})"), v);
    }
}

// ===========================================================================
// bad — CONFIGS rows 12..16
// ===========================================================================

#[test]
fn cfg12_bad_index_zero() {
    diff_bad("cfg12 bad(0)", 0);
}

#[test]
fn cfg13_bad_interior_indices() {
    for v in 1..=8 {
        diff_bad(&format!("cfg13 bad({v})"), v);
    }
}

#[test]
fn cfg14_bad_last_valid_index() {
    diff_bad("cfg14 bad(9)", 9);
}

#[test]
fn cfg15_bad_defined_out_of_bounds() {
    // No upper-bound check exists in C; 10 and 11 land in the frame's own
    // padding / loop-counter slot and stay deterministic. See ERRORS.md rows
    // 7-8 and the disassembly note in harness::BAD_MAX_DEFINED.
    for v in 10..=BAD_MAX_DEFINED {
        diff_bad(&format!("cfg15 bad({v})"), v);
    }
}

#[test]
fn cfg16_bad_negative() {
    for v in [-1, -2, -10, -11, -100, i32::MIN, i32::MIN + 1] {
        diff_bad(&format!("cfg16 bad({v})"), v);
    }
    let mut rng = Rng::new(SEED ^ 0x16);
    for iter in 0..512 {
        let v = rng.range_i32(i32::MIN, -1);
        diff_bad(&format!("cfg16 bad(random negative #{iter} = {v})"), v);
    }
}

#[test]
fn cfg16b_bad_full_defined_domain_randomized() {
    // Random sweep across every *defined* input to bad(): INT_MIN..=11.
    let mut rng = Rng::new(SEED ^ 0x16B);
    for iter in 0..512 {
        let v = if iter % 3 == 0 {
            rng.range_i32(0, BAD_MAX_DEFINED)
        } else {
            rng.range_i32(i32::MIN, BAD_MAX_DEFINED)
        };
        diff_bad(&format!("cfg16b bad(random defined #{iter} = {v})"), v);
    }
}

// ===========================================================================
// good — CONFIGS rows 17..22
// ===========================================================================

#[test]
fn cfg17_good_index_zero() {
    diff_good("cfg17 good(0)", 0);
}

#[test]
fn cfg18_good_interior_indices() {
    for v in 1..=8 {
        diff_good(&format!("cfg18 good({v})"), v);
    }
}

#[test]
fn cfg19_good_last_valid_index() {
    diff_good("cfg19 good(9)", 9);
}

#[test]
fn cfg20_good_above_range() {
    for v in [10, 11, 12, 100, 1_000_000, i32::MAX - 1, i32::MAX] {
        diff_good(&format!("cfg20 good({v})"), v);
    }
}

#[test]
fn cfg21_good_negative() {
    for v in [-1, -2, -100, i32::MIN + 1, i32::MIN] {
        diff_good(&format!("cfg21 good({v})"), v);
    }
}

#[test]
fn cfg22_good_randomized_full_domain() {
    // `good` is total: every i32 is a defined input (goodB2G range-checks).
    let mut rng = Rng::new(SEED ^ 0x22);
    for iter in 0..2048 {
        let v = rng.next_i32();
        diff_good(&format!("cfg22 good(random #{iter} = {v})"), v);
    }
    // Bias a second sweep toward the interesting boundary region.
    for iter in 0..2048 {
        let v = rng.range_i32(-20, 20);
        diff_good(&format!("cfg22 good(near-boundary #{iter} = {v})"), v);
    }
}

// ===========================================================================
// driver — CONFIGS rows 23..28
// ===========================================================================

#[test]
fn cfg23_driver_valid_cross_product() {
    for g in 0..=9 {
        for b in 0..=9 {
            diff_driver(&format!("cfg23 driver({g}, {b})"), g, b);
        }
    }
}

#[test]
fn cfg24_driver_valid_good_oob_bad() {
    for g in 0..=9 {
        for b in 10..=BAD_MAX_DEFINED {
            diff_driver(&format!("cfg24 driver({g}, {b})"), g, b);
        }
    }
}

#[test]
fn cfg25_driver_negative_bad_data() {
    for g in [0, 5, 9] {
        for b in [-1, -2, -100, i32::MIN + 1, i32::MIN] {
            diff_driver(&format!("cfg25 driver({g}, {b})"), g, b);
        }
    }
}

#[test]
fn cfg26_driver_invalid_good_data() {
    for g in [-1, -2, i32::MIN, 10, 11, 100, i32::MAX] {
        for b in [0, 7, 9] {
            diff_driver(&format!("cfg26 driver({g}, {b})"), g, b);
        }
    }
}

#[test]
fn cfg27_driver_both_invalid() {
    let goods: [c_int; 5] = [-1, i32::MIN, 10, 100, i32::MAX];
    let bads: [c_int; 5] = [-1, i32::MIN, i32::MIN + 1, 10, 11];
    for g in goods {
        for b in bads {
            diff_driver(&format!("cfg27 driver({g}, {b})"), g, b);
        }
    }
}

#[test]
fn cfg28_driver_randomized() {
    let mut rng = Rng::new(SEED ^ 0x28);
    for iter in 0..1024 {
        let g = rng.next_i32(); // full i32 domain: `good` is total
        let b = rng.range_i32(i32::MIN, BAD_MAX_DEFINED); // defined domain only
        diff_driver(&format!("cfg28 driver(random #{iter} = {g}, {b})"), g, b);
    }
    for iter in 0..1024 {
        let g = rng.range_i32(-15, 15);
        let b = rng.range_i32(-15, BAD_MAX_DEFINED);
        diff_driver(&format!("cfg28 driver(near-boundary #{iter} = {g}, {b})"), g, b);
    }
}

// ===========================================================================
// Composed pipeline — CONFIGS rows 29..30
// ===========================================================================

/// A randomized *sequence* of calls replayed against one continuous stdout
/// capture. Per-call tests cannot see ordering / flushing / residual-state
/// bugs in the composed pipeline; this row can.
#[test]
fn cfg29_interleaved_pipeline() {
    #[derive(Clone, Copy, Debug)]
    enum Step {
        Line(usize),
        NullLine,
        Int(c_int),
        Bad(c_int),
        Good(c_int),
        Driver(c_int, c_int),
    }

    let strings: Vec<std::ffi::CString> = [
        &b"step"[..],
        b"",
        b"%s%d",
        b"a longer marker line -----",
        b"\xC3\xA9\xC3\xA8",
    ]
    .iter()
    .map(|s| std::ffi::CString::new(*s).unwrap())
    .collect();

    let mut rng = Rng::new(SEED ^ 0x29);
    let mut plan = Vec::with_capacity(256);
    for _ in 0..256 {
        plan.push(match rng.below(6) {
            0 => Step::Line(rng.below(strings.len() as u64) as usize),
            1 => Step::NullLine,
            2 => Step::Int(rng.next_i32()),
            3 => Step::Bad(rng.range_i32(-4, BAD_MAX_DEFINED)),
            4 => Step::Good(rng.range_i32(-4, 14)),
            _ => Step::Driver(rng.range_i32(-4, 14), rng.range_i32(-4, BAD_MAX_DEFINED)),
        });
    }

    let run = |api: &Api| unsafe {
        for step in &plan {
            match *step {
                Step::Line(i) => (api.print_line)(strings[i].as_ptr()),
                Step::NullLine => (api.print_line)(std::ptr::null()),
                Step::Int(v) => (api.print_int_line)(v),
                Step::Bad(v) => (api.bad)(v),
                Step::Good(v) => (api.good)(v),
                Step::Driver(g, b) => (api.driver)(g, b),
            }
        }
    };

    diff("cfg29 interleaved 256-step pipeline", run);
}

/// Repeated invocation must be idempotent: no residual state between calls, so
/// three back-to-back calls yield exactly three identical blocks, in both libs.
#[test]
fn cfg30_repeated_invocation_no_residual_state() {
    let msg = std::ffi::CString::new("repeat").unwrap();
    for v in [0, 5, 9, 10, 11, -1] {
        let m = msg.clone();
        diff(&format!("cfg30 repeat x3 @ {v}"), move |api| unsafe {
            for _ in 0..3 {
                (api.print_line)(m.as_ptr());
                (api.print_int_line)(v);
                (api.bad)(v);
                (api.good)(v);
                (api.driver)(v, v);
            }
        });
    }

    // Also assert the "3 identical copies" structural property against C, so
    // the row proves idempotence and not merely C/Rust agreement.
    for v in [0, 5, 9] {
        let one = capture(|| unsafe { (c_api().bad)(v) });
        let three = capture(|| unsafe {
            for _ in 0..3 {
                (c_api().bad)(v)
            }
        });
        let mut expect = Vec::new();
        for _ in 0..3 {
            expect.extend_from_slice(&one);
        }
        assert_eq!(three, expect, "C bad({v}) is not idempotent");

        let r_one = capture(|| unsafe { (rust_api().bad)(v) });
        let r_three = capture(|| unsafe {
            for _ in 0..3 {
                (rust_api().bad)(v)
            }
        });
        let mut r_expect = Vec::new();
        for _ in 0..3 {
            r_expect.extend_from_slice(&r_one);
        }
        assert_eq!(r_three, r_expect, "Rust bad({v}) is not idempotent");
        assert_eq!(one, r_one, "C/Rust bad({v}) mismatch");
    }
}

// ===========================================================================
// Sanity: the harness really loaded two distinct .so files.
// ===========================================================================

#[test]
fn cfg00_harness_loads_two_distinct_libraries() {
    let l = libs();
    assert_ne!(l.c_path, l.rust_path);
    assert!(l.c_path.to_string_lossy().contains("c_src"));
    assert!(l.rust_path.to_string_lossy().contains("target"));
    // And the capture mechanism actually observes output.
    let out = capture(|| unsafe { (c_api().print_int_line)(42) });
    assert_eq!(out, b"42\n");
    let out = capture(|| unsafe { (rust_api().print_int_line)(42) });
    assert_eq!(out, b"42\n");
}
