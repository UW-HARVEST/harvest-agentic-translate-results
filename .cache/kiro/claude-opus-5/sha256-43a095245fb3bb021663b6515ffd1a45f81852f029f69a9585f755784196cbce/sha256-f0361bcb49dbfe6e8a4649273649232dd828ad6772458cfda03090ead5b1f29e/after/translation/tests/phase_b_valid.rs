// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md. Every call goes through the `.so` exports
// of BOTH the C and the Rust library (see tests/common/mod.rs).

mod common;
use common::*;

const SEED: u64 = 0x5EED_1234_ABCD_0001;

// ---------------------------------------------------------------------------
// Rows 1-5: safe_double_to_int
// ---------------------------------------------------------------------------

#[test]
fn row01_sdti_inrange_positive() {
    let mut r = Rng::new(SEED ^ 1);
    for _ in 0..5000 {
        // whole + fractional parts inside [0, INT_MAX]
        let whole = r.range_i32(0, i32::MAX) as f64;
        let frac = (r.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        assert_sdti_eq("cfg1", whole);
        assert_sdti_eq("cfg1", (whole + frac).min(i32::MAX as f64));
    }
}

#[test]
fn row02_sdti_inrange_negative() {
    let mut r = Rng::new(SEED ^ 2);
    for _ in 0..5000 {
        let whole = r.range_i32(i32::MIN, 0) as f64;
        let frac = (r.next_u64() >> 11) as f64 / (1u64 << 53) as f64;
        assert_sdti_eq("cfg2", whole);
        assert_sdti_eq("cfg2", (whole - frac).max(i32::MIN as f64));
    }
}

#[test]
fn row03_sdti_full_magnitude_sweep() {
    let mut r = Rng::new(SEED ^ 3);
    for _ in 0..20000 {
        assert_sdti_eq("cfg3", r.next_f64_scaled());
    }
}

#[test]
fn row04_sdti_exact_boundaries() {
    let imax = i32::MAX as f64; // 2147483647.0, exactly representable
    let imin = i32::MIN as f64; // -2147483648.0, exactly representable
    let cases: &[f64] = &[
        0.0,
        -0.0,
        1.0,
        -1.0,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        2147483646.0,
        2147483647.0,
        2147483647.5,
        2147483648.0,
        4294967296.0,
        -2147483647.0,
        -2147483648.0,
        -2147483648.5,
        -2147483649.0,
        imax,
        imin,
        f64::from_bits(imax.to_bits() - 1), // nextafter(INT_MAX, 0)
        f64::from_bits(imax.to_bits() + 1), // nextafter(INT_MAX, +inf)
        f64::from_bits(imin.to_bits() - 1), // nextafter(INT_MIN, 0)
        f64::from_bits(imin.to_bits() + 1), // nextafter(INT_MIN, -inf)
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF8_0000_0000_0000), // negative quiet NaN
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // NaN, all payload bits set
        f64::MAX,
        f64::MIN,
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::from_bits(1), // smallest subnormal
        f64::from_bits(0x800F_FFFF_FFFF_FFFF), // negative subnormal
        f64::EPSILON,
        1e15,
        -1e15,
        1e300,
        -1e300,
    ];
    for &d in cases {
        assert_sdti_eq("cfg4", d);
    }
}

#[test]
fn row05_sdti_random_bit_patterns() {
    let mut r = Rng::new(SEED ^ 5);
    for _ in 0..30000 {
        assert_sdti_eq("cfg5", r.next_f64_bits());
    }
    // biased toward the exponent range around INT_MAX/INT_MIN
    for _ in 0..20000 {
        let exp = r.range_i32(1020, 1045) as u64;
        let mant = r.next_u64() & ((1u64 << 52) - 1);
        let sign = (r.next_u64() & 1) << 63;
        assert_sdti_eq("cfg5b", f64::from_bits(sign | (exp << 52) | mant));
    }
}

// ---------------------------------------------------------------------------
// Rows 6-9: process_with_fallthrough
// ---------------------------------------------------------------------------

#[test]
fn row06_pwf_each_case() {
    let mut r = Rng::new(SEED ^ 6);
    for code in 0..=5 {
        for _ in 0..2000 {
            assert_pwf_eq("cfg6", code, r.next_i32());
        }
        for base in [0, 1, -1, i32::MAX, i32::MIN, i32::MAX - 50, i32::MIN + 50] {
            assert_pwf_eq("cfg6", code, base);
        }
    }
}

#[test]
fn row07_pwf_default_branch() {
    let mut r = Rng::new(SEED ^ 7);
    for _ in 0..5000 {
        let mut code = r.next_i32();
        if (0..=5).contains(&code) {
            code = code.wrapping_sub(1000);
        }
        assert_pwf_eq("cfg7", code, r.next_i32());
    }
    for _ in 0..2000 {
        assert_pwf_eq("cfg7", r.range_i32(6, 1000), r.next_i32());
        assert_pwf_eq("cfg7", r.range_i32(-1000, -1), r.next_i32());
    }
}

#[test]
fn row08_pwf_overflow_accumulation() {
    for code in 1..=5 {
        for delta in 0..=60i32 {
            assert_pwf_eq("cfg8", code, i32::MAX - delta);
            assert_pwf_eq("cfg8", code, i32::MIN + delta);
        }
    }
}

#[test]
fn row09_pwf_case_set_boundaries() {
    for code in [i32::MIN, i32::MIN + 1, -7, -6, -5, -1, 0, 1, 5, 6, 7, i32::MAX - 1, i32::MAX] {
        for base in [0, 1, -1, 42, i32::MAX, i32::MIN] {
            assert_pwf_eq("cfg9", code, base);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 10-11: handle_pointer_operations
// ---------------------------------------------------------------------------

#[test]
fn row10_hpo_random() {
    let mut r = Rng::new(SEED ^ 10);
    for _ in 0..20000 {
        assert_hpo_eq("cfg10", r.next_i32());
    }
}

#[test]
fn row11_hpo_overflow() {
    let half_max = i32::MAX / 2;
    let half_min = i32::MIN / 2;
    for k in -100..=100i32 {
        assert_hpo_eq("cfg11", half_max.wrapping_add(k));
        assert_hpo_eq("cfg11", half_min.wrapping_add(k));
        assert_hpo_eq("cfg11", i32::MAX.wrapping_add(k));
        assert_hpo_eq("cfg11", i32::MIN.wrapping_add(k));
        assert_hpo_eq("cfg11", k);
    }
}

// ---------------------------------------------------------------------------
// Rows 12-14: copy_data_block
// ---------------------------------------------------------------------------

#[test]
fn row12_copy_random_payloads() {
    let mut r = Rng::new(SEED ^ 12);
    for _ in 0..5000 {
        let mut src = [0u8; DATABLOCK_SIZE];
        for chunk in src.chunks_mut(8) {
            let v = r.next_u64().to_le_bytes();
            chunk.copy_from_slice(&v[..chunk.len()]);
        }
        assert_copy_eq("cfg12", &src, 0x00);
    }
}

#[test]
fn row13_copy_special_field_contents() {
    let specials: &[f64] = &[
        0.0,
        -0.0,
        f64::NAN,
        -f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        f64::MIN,
        1.5,
        -3.25,
    ];
    let labels: &[[u8; 20]] = &[
        *b"Source\0\0\0\0\0\0\0\0\0\0\0\0\0\0",
        [0xFF; 20],
        [0x00; 20],
        *b"ab\0cd\0ef\0gh\0ij\0kl\0mn",
        *b"\xC3\xA9\xE2\x82\xAC\xF0\x9F\x92\xA9zzzzzzzzzzz",
        *b"AAAAAAAAAAAAAAAAAAAA",
    ];
    for &v in specials {
        for lab in labels {
            for &id in &[0i32, -1, i32::MAX, i32::MIN, 12345] {
                let mut src = [0u8; DATABLOCK_SIZE];
                src[0..4].copy_from_slice(&id.to_le_bytes());
                src[4..8].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]); // padding
                src[8..16].copy_from_slice(&v.to_bits().to_le_bytes());
                src[16..36].copy_from_slice(&lab[..]);
                src[36..40].copy_from_slice(&[0xC0, 0xFF, 0xEE, 0x11]); // trailing pad
                assert_copy_eq("cfg13", &src, 0x5A);
            }
        }
    }
}

#[test]
fn row14_copy_overwrites_prefilled_dest_including_padding() {
    let mut r = Rng::new(SEED ^ 14);
    for fill in [0x00u8, 0xFF, 0x5A, 0xA5, 0x01] {
        for _ in 0..500 {
            let mut src = [0u8; DATABLOCK_SIZE];
            for chunk in src.chunks_mut(8) {
                let v = r.next_u64().to_le_bytes();
                chunk.copy_from_slice(&v[..chunk.len()]);
            }
            assert_copy_eq("cfg14", &src, fill);
        }
    }
}

// ---------------------------------------------------------------------------
// Rows 15-30: overunder (return value AND byte-exact stdout)
// ---------------------------------------------------------------------------

#[test]
fn row15_overunder_a_mod6_zero() {
    let mut r = Rng::new(SEED ^ 15);
    for _ in 0..200 {
        let a = r.range_i32(0, 300_000_000) / 6 * 6;
        assert_overunder_eq("cfg15", a, r.range_i32(-1000, 1000), r.range_i32(-1000, 1000), r.range_i32(-1000, 1000));
    }
}

#[test]
fn row16_overunder_each_fallthrough_chain() {
    let mut r = Rng::new(SEED ^ 16);
    for m in 1..=5i32 {
        for _ in 0..120 {
            let a = r.range_i32(0, 300_000_000) / 6 * 6 + m;
            assert_overunder_eq(
                "cfg16",
                a,
                r.range_i32(-100_000, 100_000),
                r.range_i32(-100_000, 100_000),
                r.range_i32(-30_000, 30_000),
            );
        }
    }
}

#[test]
fn row17_overunder_negative_a_default_branch() {
    let mut r = Rng::new(SEED ^ 17);
    for m in 1..=5i32 {
        for _ in 0..120 {
            let a = -(r.range_i32(0, 300_000_000) / 6 * 6 + m);
            assert_overunder_eq(
                "cfg17",
                a,
                r.range_i32(-100_000, 100_000),
                r.range_i32(-100_000, 100_000),
                r.range_i32(-30_000, 30_000),
            );
        }
    }
}

#[test]
fn row18_overunder_zeros() {
    assert_overunder_eq("cfg18", 0, 0, 0, 0);
    for v in [1, -1, 7, -7, 1000, -1000, i32::MAX, i32::MIN] {
        assert_overunder_eq("cfg18", 0, v, v, v);
        assert_overunder_eq("cfg18", v, 0, v, v);
        assert_overunder_eq("cfg18", v, v, 0, v);
        assert_overunder_eq("cfg18", v, v, v, 0);
    }
}

#[test]
fn row19_overunder_small_positive() {
    let mut r = Rng::new(SEED ^ 19);
    for _ in 0..600 {
        assert_overunder_eq(
            "cfg19",
            r.range_i32(1, 1000),
            r.range_i32(1, 1000),
            r.range_i32(1, 1000),
            r.range_i32(1, 1000),
        );
    }
}

#[test]
fn row20_overunder_small_negative_and_mixed() {
    let mut r = Rng::new(SEED ^ 20);
    for _ in 0..600 {
        assert_overunder_eq(
            "cfg20",
            r.range_i32(-1000, -1),
            r.range_i32(-1000, -1),
            r.range_i32(-1000, -1),
            r.range_i32(-1000, -1),
        );
    }
    for _ in 0..600 {
        let s = |r: &mut Rng| if r.next_u64() & 1 == 0 { 1 } else { -1 };
        let (s1, s2, s3, s4) = (s(&mut r), s(&mut r), s(&mut r), s(&mut r));
        assert_overunder_eq(
            "cfg20b",
            s1 * r.range_i32(1, 5000),
            s2 * r.range_i32(1, 5000),
            s3 * r.range_i32(1, 5000),
            s4 * r.range_i32(1, 5000),
        );
    }
}

#[test]
fn row21_overunder_sqrt_arg_overflows_positive() {
    let mut r = Rng::new(SEED ^ 21);
    let mut found = 0;
    for _ in 0..8000 {
        let a = r.next_i32();
        let d = r.next_i32();
        let s = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        // overflowed (true product exceeds i32) but wrapped to non-negative
        let true_sum = (d as i64) * (d as i64) + (a as i64) * (a as i64);
        if true_sum > i32::MAX as i64 && s >= 0 {
            assert_overunder_eq("cfg21", a, r.next_i32(), r.next_i32(), d);
            found += 1;
            if found >= 250 {
                break;
            }
        }
    }
    assert!(found > 0, "row21 generated no positive-wrap case");
}

#[test]
fn row22_overunder_sqrt_arg_overflows_negative_nan() {
    let mut r = Rng::new(SEED ^ 22);
    let mut found = 0;
    for _ in 0..8000 {
        let a = r.next_i32();
        let d = r.next_i32();
        let s = d.wrapping_mul(d).wrapping_add(a.wrapping_mul(a));
        if s < 0 {
            assert_overunder_eq("cfg22", a, r.next_i32(), r.next_i32(), d);
            found += 1;
            if found >= 250 {
                break;
            }
        }
    }
    assert!(found > 0, "row22 generated no negative-wrap (NaN sqrt) case");
    // deterministic negative-wrap singletons
    for (a, d) in [(46341, 46341), (i32::MAX, i32::MAX), (65536, 46341), (i32::MIN, 1)] {
        assert_overunder_eq("cfg22b", a, 3, 5, d);
    }
}

#[test]
fn row23_overunder_b_times_2_7_over_intmax() {
    let mut r = Rng::new(SEED ^ 23);
    for _ in 0..200 {
        let b = r.range_i32(800_000_000, i32::MAX);
        assert_overunder_eq("cfg23", r.range_i32(0, 1000), b, r.range_i32(-1000, 1000), r.range_i32(0, 100));
    }
    assert_overunder_eq("cfg23b", 6, i32::MAX, 1, 1);
    assert_overunder_eq("cfg23c", 7, 795_364_240, 1, 1); // just over INT_MAX/2.7
}

#[test]
fn row24_overunder_b_times_2_7_under_intmin() {
    let mut r = Rng::new(SEED ^ 24);
    for _ in 0..200 {
        let b = r.range_i32(i32::MIN, -800_000_000);
        assert_overunder_eq("cfg24", r.range_i32(0, 1000), b, r.range_i32(-1000, 1000), r.range_i32(0, 100));
    }
    assert_overunder_eq("cfg24b", 6, i32::MIN, 1, 1);
    assert_overunder_eq("cfg24c", 8, -795_364_241, 1, 1);
}

#[test]
fn row25_overunder_a_times_1_5_over_intmax() {
    let mut r = Rng::new(SEED ^ 25);
    for _ in 0..200 {
        let a = r.range_i32(1_500_000_000, i32::MAX);
        assert_overunder_eq("cfg25", a, r.range_i32(-1000, 1000), r.range_i32(-1000, 1000), r.range_i32(0, 100));
    }
    // exact clamp boundary of a * 1.5 vs (double)INT_MAX
    for a in [1_431_655_764, 1_431_655_765, 1_431_655_766, i32::MAX, i32::MAX - 1] {
        assert_overunder_eq("cfg25b", a, 2, 3, 4);
    }
    for a in [-1_431_655_764, -1_431_655_765, -1_431_655_766, i32::MIN, i32::MIN + 1] {
        assert_overunder_eq("cfg25c", a, 2, 3, 4);
    }
}

#[test]
fn row26_overunder_c_extremes() {
    let mut r = Rng::new(SEED ^ 26);
    for c in [
        i32::MIN,
        i32::MIN + 1,
        i32::MIN / 2,
        -3,
        -2,
        -1,
        0,
        1,
        2,
        3,
        i32::MAX / 2,
        i32::MAX - 1,
        i32::MAX,
    ] {
        assert_overunder_eq("cfg26", r.range_i32(0, 100), r.range_i32(-100, 100), c, r.range_i32(0, 100));
    }
}

#[test]
fn row27_overunder_a_plus_b_overflow() {
    let mut r = Rng::new(SEED ^ 27);
    for _ in 0..150 {
        assert_overunder_eq(
            "cfg27",
            r.range_i32(i32::MAX - 1000, i32::MAX),
            r.range_i32(i32::MAX - 1000, i32::MAX),
            r.range_i32(-100, 100),
            r.range_i32(-100, 100),
        );
        assert_overunder_eq(
            "cfg27b",
            r.range_i32(i32::MIN, i32::MIN + 1000),
            r.range_i32(i32::MIN, i32::MIN + 1000),
            r.range_i32(-100, 100),
            r.range_i32(-100, 100),
        );
    }
}

#[test]
fn row28_overunder_extreme_sweep() {
    let ex = [i32::MIN, i32::MIN + 1, -1, 0, 1, i32::MAX - 1, i32::MAX];
    for &a in &ex {
        for &b in &ex {
            for &c in &ex {
                for &d in &ex {
                    assert_overunder_eq("cfg28", a, b, c, d);
                }
            }
        }
    }
}

#[test]
fn row29_overunder_full_random() {
    let mut r = Rng::new(SEED ^ 29);
    for _ in 0..2500 {
        assert_overunder_eq("cfg29", r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
    }
}

#[test]
fn row30_overunder_stdout_is_compared_everywhere() {
    // `assert_overunder_eq` compares stdout byte-for-byte on every call, so
    // rows 15-29 already cover this. This row pins the invariant explicitly:
    // the capture must be non-empty and identical, and must contain all 8
    // printf call sites of `overunder`.
    let mut r = Rng::new(SEED ^ 30);
    for _ in 0..300 {
        let (a, b, c, d) = (r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32());
        let ((cr, cout), (rr, rout)) = overunder_both(a, b, c, d);
        assert_eq!(cr, rr, "cfg30 return mismatch for ({a},{b},{c},{d})");
        assert_eq!(cout, rout, "cfg30 stdout mismatch for ({a},{b},{c},{d})");
        assert!(!cout.is_empty(), "cfg30 capture was empty — harness broken");
        let s = String::from_utf8_lossy(&cout);
        for needle in [
            "result_1 = ",
            "result_2 = ",
            "Converted values: ",
            "Switch fall-through result: ",
            "Copied block: id=",
            "Pointer operation result: ",
            "Overflow protected conversion: ",
            "Underflow protected conversion: ",
            "Array copied via memcpy: ",
        ] {
            assert!(s.contains(needle), "cfg30 missing {needle:?} in output:\n{s}");
        }
    }
}

// ---------------------------------------------------------------------------
// Row 31: composed pipeline cross-check
// ---------------------------------------------------------------------------

#[test]
fn row31_composition_matches_lowlevel_exports() {
    // Recompute `overunder`'s return value by driving the low-level exports of
    // EACH library separately, in the same order the C source calls them, and
    // require that it equals that library's own `overunder` result. This
    // catches a divergence in how the pipeline is composed even if every
    // individual wrapper agrees.
    let p = pair();
    let mut r = Rng::new(SEED ^ 31);
    let mut inputs: Vec<(i32, i32, i32, i32)> = vec![
        (0, 0, 0, 0),
        (i32::MAX, i32::MAX, i32::MAX, i32::MAX),
        (i32::MIN, i32::MIN, i32::MIN, i32::MIN),
        (7, -3, 11, -5),
    ];
    for _ in 0..1500 {
        inputs.push((r.next_i32(), r.next_i32(), r.next_i32(), r.next_i32()));
    }

    for (a, b, c, d) in inputs {
        for im in [&p.c, &p.rs] {
            let temp1 = a as f64 * 1.5;
            let temp2 = b as f64 * 2.7;
            let temp3 = c as f64 / 3.3;
            let sq = (d.wrapping_mul(d)).wrapping_add(a.wrapping_mul(a)) as f64;
            let temp4 = sq.sqrt();

            let (conv1, conv2, conv3, conv4) = unsafe {
                (
                    (im.safe_double_to_int)(temp1),
                    (im.safe_double_to_int)(temp2),
                    (im.safe_double_to_int)(temp3),
                    (im.safe_double_to_int)(temp4),
                )
            };
            let switch_result = unsafe { (im.process_with_fallthrough)(a % 6, b) };
            let ptr_result = unsafe { (im.handle_pointer_operations)(c) };

            let mut total = conv1
                .wrapping_add(conv2)
                .wrapping_add(conv3)
                .wrapping_add(conv4)
                .wrapping_add(switch_result)
                .wrapping_add(ptr_result);
            total = total.wrapping_add(a); // dest_block.id == a
            for v in [a, b, c, d, a.wrapping_add(b)] {
                total = total.wrapping_add(v);
            }

            let actual = capture_stdout(|| unsafe { (im.overunder)(a, b, c, d) }).0;
            assert_eq!(
                total, actual,
                "[cfg31/{}] composed({a},{b},{c},{d}) = {total} but overunder returned {actual}",
                im.name
            );
        }
    }
}
