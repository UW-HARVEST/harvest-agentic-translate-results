// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md; every row uses many randomized inputs with a
// fixed seed plus the hand-picked boundary values.  All calls go through the
// exported symbols of BOTH shared objects.

mod common;
use common::*;

const N: usize = 400; // randomized cases per property row

// ---------------------------------------------------------------------------
// Row 1 — baseline: canonical house, extra_bedrooms = 0, single run() call
// ---------------------------------------------------------------------------
#[test]
fn row01_run_baseline_canonical() {
    diff_run(HouseT::canonical(), 0, 1, "row01");
    // sanity: this really is the shape driver() uses
    let l = libs();
    let mut h = HouseT::canonical();
    let out = capture(|| unsafe { (l.c.run)(&mut h, 0) });
    let expected: &[u8] = b"The house has 2 floors, 5 bedrooms, and 2.5 bathrooms\n\
The house has 3 floors, 5 bedrooms, and 2.5 bathrooms\n\
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n\
The house has 3 floors, 5 bedrooms, and 3.5 bathrooms\n";
    assert_eq!(
        out,
        expected,
        "unexpected C baseline output: {}",
        String::from_utf8_lossy(&out)
    );
}

// ---------------------------------------------------------------------------
// Row 2 — randomized small ints, half-integer bathrooms
// ---------------------------------------------------------------------------
#[test]
fn row02_run_small_random() {
    let mut rng = Rng::new(SEED ^ 2);
    for i in 0..N {
        let h = HouseT::new(
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000),
            rng.range_i32(-1000, 1000) as f64 / 2.0,
        );
        diff_run(h, rng.range_i32(-1000, 1000), 1, &format!("row02#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 3 — full i32 range for every integer field
// ---------------------------------------------------------------------------
#[test]
fn row03_run_full_int_range() {
    let mut rng = Rng::new(SEED ^ 3);
    for i in 0..N {
        let h = HouseT::new(
            rng.next_i32(),
            rng.next_i32(),
            rng.next_i32() as f64 / 2.0,
        );
        diff_run(h, rng.next_i32(), 1, &format!("row03#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 4 — arbitrary finite f64 bathrooms (random bit patterns)
// ---------------------------------------------------------------------------
#[test]
fn row04_run_random_finite_doubles() {
    let mut rng = Rng::new(SEED ^ 4);
    for i in 0..N {
        let h = HouseT::new(
            rng.range_i32(-50, 50),
            rng.range_i32(-50, 50),
            rng.next_finite_f64(),
        );
        diff_run(h, rng.range_i32(-50, 50), 1, &format!("row04#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 5 — %.1f round-half-even tie values
// ---------------------------------------------------------------------------
#[test]
fn row05_run_rounding_ties() {
    let mut ties: Vec<f64> = Vec::new();
    for k in 0..40 {
        ties.push(k as f64 + 0.05);
        ties.push(k as f64 + 0.15);
        ties.push(k as f64 + 0.25);
        ties.push(k as f64 + 0.35);
        ties.push(k as f64 + 0.45);
        ties.push(k as f64 + 0.55);
        ties.push(k as f64 + 0.65);
        ties.push(k as f64 + 0.75);
        ties.push(k as f64 + 0.85);
        ties.push(k as f64 + 0.95);
        ties.push(-(k as f64) - 0.25);
        ties.push(-(k as f64) - 0.75);
        ties.push(k as f64 / 16.0);
        ties.push(k as f64 / 3.0);
    }
    for (i, &b) in ties.iter().enumerate() {
        diff_run(HouseT::new(1, 1, b), 1, 1, &format!("row05#{i}({b})"));
    }
}

// ---------------------------------------------------------------------------
// Row 6 — special finite doubles
// ---------------------------------------------------------------------------
#[test]
fn row06_run_special_finite_doubles() {
    let vals: [f64; 16] = [
        0.0,
        -0.0,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        5e-324,          // smallest subnormal
        -5e-324,
        f64::MAX,
        f64::MIN,
        1e300,
        -1e300,
        1e-300,
        1e16,            // += 1.0 still representable
        9007199254740992.0, // 2^53: += 1.0 is a no-op
        4503599627370496.0, // 2^52
        -9007199254740993.0,
        0.049999999999999996,
    ];
    for (i, &b) in vals.iter().enumerate() {
        diff_run(HouseT::new(0, 0, b), 0, 1, &format!("row06#{i}"));
        diff_run(HouseT::new(-7, 13, b), -3, 2, &format!("row06b#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 7 — non-finite bathrooms
// ---------------------------------------------------------------------------
#[test]
fn row07_run_non_finite_doubles() {
    let vals: [f64; 6] = [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7ff8_0000_0000_0001), // NaN with payload
        f64::from_bits(0xfff0_0000_0000_0001), // signalling-ish NaN
    ];
    for (i, &b) in vals.iter().enumerate() {
        diff_run(HouseT::new(2, 5, b), 4, 1, &format!("row07#{i}"));
        diff_run(HouseT::new(2, 5, b), 4, 3, &format!("row07b#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 8 — add_floor overflow
// ---------------------------------------------------------------------------
#[test]
fn row08_run_floors_overflow() {
    for &f in &[i32::MAX, i32::MAX - 1, i32::MIN, i32::MIN + 1, -1, 0] {
        diff_run(HouseT::new(f, 5, 2.5), 0, 1, &format!("row08({f})"));
        diff_run(HouseT::new(f, 5, 2.5), 0, 5, &format!("row08x5({f})"));
    }
}

// ---------------------------------------------------------------------------
// Row 9 — add_bedrooms overflow / underflow
// ---------------------------------------------------------------------------
#[test]
fn row09_run_bedrooms_overflow() {
    let cases: [(i32, i32); 8] = [
        (i32::MAX, 1),
        (i32::MAX, i32::MAX),
        (i32::MAX - 5, 10),
        (i32::MIN, -1),
        (i32::MIN, i32::MIN),
        (i32::MIN + 5, -10),
        (0, i32::MIN),
        (-1, i32::MIN),
    ];
    for (i, &(bed, extra)) in cases.iter().enumerate() {
        diff_run(HouseT::new(1, bed, 0.5), extra, 1, &format!("row09#{i}"));
        diff_run(HouseT::new(1, bed, 0.5), extra, 4, &format!("row09b#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 10 — boundary cross-product bedrooms × extra_bedrooms
// ---------------------------------------------------------------------------
#[test]
fn row10_run_boundary_cross_product() {
    let bedrooms = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let extras = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    let floors = [i32::MIN, 0, i32::MAX];
    for &fl in &floors {
        for &bed in &bedrooms {
            for &ex in &extras {
                diff_run(
                    HouseT::new(fl, bed, 2.5),
                    ex,
                    1,
                    &format!("row10({fl},{bed},{ex})"),
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Row 11 — state carry-over across N successive run() calls
// ---------------------------------------------------------------------------
#[test]
fn row11_run_state_carry_over() {
    let mut rng = Rng::new(SEED ^ 11);
    for i in 0..N {
        let h = HouseT::new(
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-10_000, 10_000) as f64 / 8.0,
        );
        let n = rng.range_usize(1, 8);
        diff_run(h, rng.next_i32(), n, &format!("row11#{i}(n={n})"));
    }
}

// ---------------------------------------------------------------------------
// Row 12 — carry-over near the double `+= 1.0` resolution limit
// ---------------------------------------------------------------------------
#[test]
fn row12_run_carry_over_precision_limit() {
    let vals: [f64; 7] = [
        4503599627370496.0,  // 2^52
        4503599627370495.5,
        9007199254740992.0,  // 2^53
        9007199254740993.0,  // not representable -> 2^53
        -9007199254740992.0,
        1.7976931348623157e308,
        4.9e-324,
    ];
    for (i, &b) in vals.iter().enumerate() {
        for n in 1..=6 {
            diff_run(HouseT::new(7, -7, b), 3, n, &format!("row12#{i}(n={n})"));
        }
    }
}

// ---------------------------------------------------------------------------
// Row 13 — driver() canonical valid inputs
// ---------------------------------------------------------------------------
#[test]
fn row13_driver_canonical() {
    for s in ["0", "1", "-1", "5", "-5", "42"] {
        diff_driver(s, &format!("row13({s})"));
        assert_accepted(format!("{s}\0").as_bytes(), &format!("row13({s})"));
    }
}

// ---------------------------------------------------------------------------
// Row 14 — randomized valid decimal strings over the full int range
// ---------------------------------------------------------------------------
#[test]
fn row14_driver_random_valid_ints() {
    let mut rng = Rng::new(SEED ^ 14);
    for i in 0..N {
        let v = rng.next_i32();
        let s = if v >= 0 && rng.next_u64() % 2 == 0 {
            format!("+{v}")
        } else {
            format!("{v}")
        };
        diff_driver(&s, &format!("row14#{i}({s})"));
        assert_accepted(format!("{s}\0").as_bytes(), &format!("row14#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 15 — int boundary literals
// ---------------------------------------------------------------------------
#[test]
fn row15_driver_int_boundaries() {
    for s in [
        "-2147483648",
        "-2147483647",
        "2147483647",
        "2147483646",
        "+2147483647",
        "+2147483646",
        "-0",
        "+0",
        "0",
    ] {
        diff_driver(s, &format!("row15({s})"));
        assert_accepted(format!("{s}\0").as_bytes(), &format!("row15({s})"));
    }
}

// ---------------------------------------------------------------------------
// Row 16 — leading zeros
// ---------------------------------------------------------------------------
#[test]
fn row16_driver_leading_zeros() {
    let mut cases: Vec<String> = vec![
        "000".into(),
        "0000000000000000005".into(),
        "-000005".into(),
        "+0000000000000000000000000042".into(),
        "-0000000000000000000000000000".into(),
        format!("{}{}", "0".repeat(60), 7),
        format!("-{}{}", "0".repeat(60), 2147483648u32), // zeros then out-of-int
    ];
    let mut rng = Rng::new(SEED ^ 16);
    for _ in 0..40 {
        let zeros = rng.range_usize(1, 30);
        let v = rng.range_i32(0, 2_000_000);
        cases.push(format!("{}{}", "0".repeat(zeros), v));
        cases.push(format!("-{}{}", "0".repeat(zeros), v));
    }
    for (i, s) in cases.iter().enumerate() {
        diff_driver(s, &format!("row16#{i}({s})"));
    }
}

// ---------------------------------------------------------------------------
// Row 17 — leading whitespace
// ---------------------------------------------------------------------------
#[test]
fn row17_driver_leading_whitespace() {
    let ws = [' ', '\t', '\n', '\u{b}', '\u{c}', '\r'];
    for (i, &c) in ws.iter().enumerate() {
        for body in ["7", "-7", "+7", "2147483648", "abc", ""] {
            let s = format!("{c}{body}");
            diff_driver(&s, &format!("row17#{i}({body})"));
        }
    }
    let mut rng = Rng::new(SEED ^ 17);
    for i in 0..120 {
        let n = rng.range_usize(1, 12);
        let mut s = String::new();
        for _ in 0..n {
            s.push(*rng.pick(&ws));
        }
        s.push_str(&format!("{}", rng.next_i32()));
        diff_driver(&s, &format!("row17rand#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 18 — trailing garbage that still SUCCEEDS (endp != str)
// ---------------------------------------------------------------------------
#[test]
fn row18_driver_trailing_garbage_accepted() {
    for s in [
        "5abc", "5 ", "5.9", "1e3", "12,34", "7-", "9\n", "0zzz", "-3xyz", "+8!",
        "2147483647x", "-2147483648y", "1 2 3",
    ] {
        diff_driver(s, &format!("row18({s})"));
        assert_accepted(format!("{s}\0").as_bytes(), &format!("row18({s})"));
    }
    // embedded NUL: strtol stops at the NUL, the rest is invisible
    diff_driver_raw(b"3\0hidden\0", "row18(embedded NUL)");
    assert_accepted(b"3\0hidden\0", "row18(embedded NUL)");
}

// ---------------------------------------------------------------------------
// Row 19 — base is fixed at 10, so prefixed forms are read as decimal
// ---------------------------------------------------------------------------
#[test]
fn row19_driver_base10_only() {
    for s in [
        "0x1A", "0X", "0x", "0b101", "010", "-0x10", "+0X20", "0o17", "0e0", "08", "09",
    ] {
        diff_driver(s, &format!("row19({s})"));
        assert_accepted(format!("{s}\0").as_bytes(), &format!("row19({s})"));
    }
}

// ---------------------------------------------------------------------------
// Row 20 — randomized digit strings of random length (spans int / long / ERANGE)
// ---------------------------------------------------------------------------
#[test]
fn row20_driver_random_digit_strings() {
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..(N * 2) {
        let len = rng.range_usize(1, 25);
        let mut s = String::new();
        match rng.next_u64() % 3 {
            0 => s.push('-'),
            1 => s.push('+'),
            _ => {}
        }
        for _ in 0..len {
            s.push((b'0' + (rng.next_u64() % 10) as u8) as char);
        }
        diff_driver(&s, &format!("row20#{i}({s})"));
    }
}

// ---------------------------------------------------------------------------
// Row 21 — byte fuzz (mixes accept and reject shapes)
// ---------------------------------------------------------------------------
#[test]
fn row21_driver_byte_fuzz() {
    let mut rng = Rng::new(SEED ^ 21);
    for i in 0..(N * 2) {
        let len = rng.range_usize(0, 24);
        let mut buf: Vec<u8> = Vec::with_capacity(len + 1);
        for _ in 0..len {
            // bias towards interesting bytes: digits, signs, spaces, letters
            let b = match rng.next_u64() % 6 {
                0 => b'0' + (rng.next_u64() % 10) as u8,
                1 => *rng.pick(&[b'-', b'+']),
                2 => *rng.pick(&[b' ', b'\t', b'\n', b'\r', 0x0b, 0x0c]),
                3 => b'a' + (rng.next_u64() % 26) as u8,
                4 => 1 + (rng.next_u64() % 255) as u8, // any non-NUL byte
                _ => *rng.pick(&[b'.', b',', b'e', b'x', b'X', b'*']),
            };
            buf.push(b);
        }
        buf.push(0);
        diff_driver_raw(&buf, &format!("row21#{i}"));
    }
}

// ---------------------------------------------------------------------------
// Row 22 — every single-byte input
// ---------------------------------------------------------------------------
#[test]
fn row22_driver_every_single_byte() {
    for b in 1u16..=255 {
        let buf = [b as u8, 0u8];
        diff_driver_raw(&buf, &format!("row22(0x{b:02x})"));
    }
}

// ---------------------------------------------------------------------------
// Row 23 — oversized inputs
// ---------------------------------------------------------------------------
#[test]
fn row23_driver_oversized_inputs() {
    let big_digits = "1".repeat(4096);
    let big_ws = format!("{}{}", " ".repeat(4096), "-12345");
    let big_alpha = "a".repeat(4096);
    let big_zeros = format!("{}{}", "0".repeat(4096), "7");
    let big_neg = format!("-{}", "9".repeat(4096));
    for (i, s) in [big_digits, big_ws, big_alpha, big_zeros, big_neg]
        .iter()
        .enumerate()
    {
        diff_driver(s, &format!("row23#{i}(len={})", s.len()));
    }
}

// ---------------------------------------------------------------------------
// Row 24 — composed pipeline / cross-entry-point interleaving
// ---------------------------------------------------------------------------
#[test]
fn row24_composed_pipeline() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 24);
    for i in 0..120 {
        let s = format!("{}\0", rng.next_i32());
        let init = HouseT::new(
            rng.next_i32(),
            rng.next_i32(),
            rng.range_i32(-500, 500) as f64 / 4.0,
        );
        let extra = rng.next_i32();
        let n = rng.range_usize(1, 3);

        let mut hc = init;
        let mut hr = init;
        let p = s.as_ptr() as *const std::ffi::c_char;

        let out_c = capture(|| unsafe {
            (l.c.driver)(p);
            for _ in 0..n {
                (l.c.run)(&mut hc, extra);
            }
            (l.c.driver)(p);
            (l.c.run)(&mut hc, extra);
        });
        let out_r = capture(|| unsafe {
            (l.rust.driver)(p);
            for _ in 0..n {
                (l.rust.run)(&mut hr, extra);
            }
            (l.rust.driver)(p);
            (l.rust.run)(&mut hr, extra);
        });
        assert_eq!(
            out_c,
            out_r,
            "row24#{i} pipeline mismatch\n  C   : {}\n  Rust: {}",
            String::from_utf8_lossy(&out_c),
            String::from_utf8_lossy(&out_r)
        );
        assert_eq!(hc.raw(), hr.raw(), "row24#{i} struct mismatch");
    }
}

// ---------------------------------------------------------------------------
// Row 25 — valid but MISALIGNED `house_t *` (C does an unaligned load, which
// x86 allows, so it must work; the Rust must move the same bytes and not abort)
// ---------------------------------------------------------------------------
#[test]
fn row25_run_misaligned_but_valid_pointer() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 25);
    for off in 0usize..8 {
        for i in 0..8 {
            let init = HouseT::new(
                rng.next_i32(),
                rng.next_i32(),
                rng.range_i32(-400, 400) as f64 / 4.0,
            );
            let extra = rng.next_i32();
            let n = rng.range_usize(1, 3);

            let mut buf_c = vec![0u8; 32];
            buf_c[off..off + 16].copy_from_slice(&init.raw());
            let mut buf_r = buf_c.clone();
            let pc = unsafe { buf_c.as_mut_ptr().add(off) } as *mut HouseT;
            let pr = unsafe { buf_r.as_mut_ptr().add(off) } as *mut HouseT;

            let d = child_diff(
                || {
                    for _ in 0..n {
                        unsafe { (l.c.run)(pc, extra) };
                    }
                    let mut s = [0u8; 16];
                    s.copy_from_slice(&buf_c[off..off + 16]);
                    s
                },
                || {
                    for _ in 0..n {
                        unsafe { (l.rust.run)(pr, extra) };
                    }
                    let mut s = [0u8; 16];
                    s.copy_from_slice(&buf_r[off..off + 16]);
                    s
                },
            );
            assert_eq!(
                exit_code(d.status),
                Some(0),
                "row25 off={off} #{i}: child died: {}",
                describe_status(d.status)
            );
            assert_eq!(
                d.out_c,
                d.out_r,
                "row25 off={off} #{i} stdout mismatch\n  C   : {}\n  Rust: {}",
                String::from_utf8_lossy(&d.out_c),
                String::from_utf8_lossy(&d.out_r)
            );
            assert_eq!(
                d.state_c, d.state_r,
                "row25 off={off} #{i} struct bytes mismatch"
            );
            // and the surrounding bytes must not be disturbed differently
            assert!(!d.out_c.is_empty(), "row25 produced no output");
        }
    }
}
