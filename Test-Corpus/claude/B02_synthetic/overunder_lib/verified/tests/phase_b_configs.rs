// Phase B -- valid-path differential tests, one test per row of CONFIGS.md.
//
// Every function is reached through `dlopen`/`dlsym` on the two shared objects;
// nothing in this file calls the Rust crate directly.

mod common;
use common::*;

/// nextafter(x, +/-inf) without depending on unstable/newer std APIs.
fn ulp_step(x: f64, up: bool) -> f64 {
    if x.is_nan() {
        return x;
    }
    if x == 0.0 {
        return if up { f64::from_bits(1) } else { -f64::from_bits(1) };
    }
    let bits = x.to_bits();
    let newbits = if (x > 0.0) == up { bits + 1 } else { bits - 1 };
    f64::from_bits(newbits)
}

// ===========================================================================
// safe_double_to_int  (rows C1 - C8)
// ===========================================================================

#[test]
fn cfg_c1_signed_zero() {
    for d in [0.0f64, -0.0f64] {
        diff_safe_double_to_int(d, "C1");
    }
}

#[test]
fn cfg_c2_subnormal_and_fractional() {
    let mut vals = vec![
        f64::from_bits(1),  // smallest positive subnormal
        -f64::from_bits(1), // smallest negative subnormal
        f64::MIN_POSITIVE,
        -f64::MIN_POSITIVE,
        f64::MIN_POSITIVE / 2.0,
        1e-300,
        -1e-300,
        0.5,
        -0.5,
        0.9999999999,
        -0.9999999999,
        1.0 - f64::EPSILON,
        -(1.0 - f64::EPSILON),
    ];
    let mut rng = Rng::for_test("C2");
    for _ in 0..2000 {
        let v = rng.range_f64(-1.0, 1.0);
        vals.push(v);
        vals.push(v * 1e-100);
    }
    for d in vals {
        diff_safe_double_to_int(d, "C2");
    }
}

#[test]
fn cfg_c3_positive_inrange_random() {
    let mut rng = Rng::for_test("C3");
    for _ in 0..5000 {
        // exponentially spread magnitudes so every scale in (0, INT_MAX) is hit
        let scale = rng.range_f64(0.0, 31.0);
        let d = 2f64.powf(scale) * rng.range_f64(0.5, 1.0);
        let d = d.min(2147483647.0);
        diff_safe_double_to_int(d, "C3");
        // plus a plain uniform draw with a guaranteed fractional part
        let u = rng.range_f64(0.0, 2147483647.0) + 0.5;
        diff_safe_double_to_int(u.min(2147483647.0), "C3-uniform");
    }
}

#[test]
fn cfg_c4_negative_inrange_random() {
    let mut rng = Rng::for_test("C4");
    for _ in 0..5000 {
        let scale = rng.range_f64(0.0, 31.0);
        let d = -(2f64.powf(scale) * rng.range_f64(0.5, 1.0));
        let d = d.max(-2147483648.0);
        diff_safe_double_to_int(d, "C4");
        let u = -rng.range_f64(0.0, 2147483647.0) - 0.5;
        diff_safe_double_to_int(u.max(-2147483648.0), "C4-uniform");
    }
}

#[test]
fn cfg_c5_exact_integral_random() {
    let mut rng = Rng::for_test("C5");
    for v in i32_corners() {
        diff_safe_double_to_int(v as f64, "C5-corner");
    }
    for _ in 0..5000 {
        let v = rng.next_i32();
        diff_safe_double_to_int(v as f64, "C5");
    }
}

#[test]
fn cfg_c6_just_outside_range() {
    let mut vals = vec![
        2147483648.0,
        -2147483649.0,
        2147483647.5,
        -2147483648.5,
        1e15,
        -1e15,
        1e300,
        -1e300,
        f64::MAX,
        f64::MIN,
    ];
    let mut rng = Rng::for_test("C6");
    for _ in 0..3000 {
        vals.push(rng.range_f64(2147483647.0, 1.0995e12)); // up to 2^40
        vals.push(rng.range_f64(-1.0995e12, -2147483648.0));
    }
    for d in vals {
        diff_safe_double_to_int(d, "C6");
    }
}

#[test]
fn cfg_c7_ulp_ladder_at_boundaries() {
    for base in [2147483647.0f64, -2147483648.0f64, 2147483648.0, -2147483649.0] {
        let mut up = base;
        let mut down = base;
        for _ in 0..8 {
            up = ulp_step(up, true);
            down = ulp_step(down, false);
            diff_safe_double_to_int(up, "C7-up");
            diff_safe_double_to_int(down, "C7-down");
        }
        diff_safe_double_to_int(base, "C7-base");
    }
}

#[test]
fn cfg_c8_arbitrary_bit_patterns() {
    let mut rng = Rng::for_test("C8");
    for _ in 0..20000 {
        diff_safe_double_to_int(rng.next_f64_bits(), "C8");
    }
    // and every special encoding explicitly
    for d in [
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::NAN,
        -f64::NAN,
        f64::from_bits(0x7FF8_0000_0000_0000), // canonical quiet NaN
        f64::from_bits(0xFFF8_0000_0000_0000),
        f64::from_bits(0x7FF0_0000_0000_0001), // signalling NaN
        f64::from_bits(0xFFF0_0000_0000_0001),
        f64::from_bits(0x7FFF_FFFF_FFFF_FFFF), // NaN, all payload bits set
    ] {
        diff_safe_double_to_int(d, "C8-special");
    }
}

// ===========================================================================
// process_with_fallthrough  (rows C9 - C17)
// ===========================================================================

#[test]
fn cfg_c9_c14_each_case_random() {
    let mut rng = Rng::for_test("C9-14");
    // Expected fall-through deltas, straight from the C switch.
    let expected_delta = |code: i32| -> Option<i32> {
        match code {
            5 => Some(150),
            4 => Some(100),
            3 => Some(60),
            2 => Some(30),
            1 => Some(10),
            _ => None,
        }
    };
    for code in 0..=5i32 {
        let mut bases = vec![0, 1, -1, 7, -7, 1000, -1000];
        for _ in 0..1000 {
            bases.push(rng.next_i32());
            bases.push(rng.range_i32(-10_000, 10_000));
        }
        for base in bases {
            let got = diff_process(code, base, &format!("C{}", 9 + code));
            // Cross-check against the C semantics we read out of the source.
            if code == 0 {
                assert_eq!(got, 0, "case 0 must discard base_value (base={base})");
            } else if let Some(delta) = expected_delta(code) {
                assert_eq!(
                    got,
                    base.wrapping_add(delta),
                    "case {code} fall-through delta (base={base})"
                );
            }
        }
    }
}

#[test]
fn cfg_c15_default_arm_random() {
    let mut rng = Rng::for_test("C15");
    let codes = [-1i32, -2, -6, -7, 6, 7, 8, 100, 12345, i32::MIN, i32::MAX];
    for code in codes {
        for _ in 0..500 {
            let base = rng.next_i32();
            let got = diff_process(code, base, "C15");
            assert_eq!(got, -1, "default arm sentinel (code={code} base={base})");
        }
    }
}

#[test]
fn cfg_c16_exhaustive_cross_product() {
    let bases = [
        i32::MIN,
        i32::MIN + 1,
        -1,
        0,
        1,
        i32::MAX - 150,
        i32::MAX - 1,
        i32::MAX,
    ];
    for code in -8..=13i32 {
        for &base in &bases {
            diff_process(code, base, "C16");
        }
    }
}

#[test]
fn cfg_c17_fully_random() {
    let mut rng = Rng::for_test("C17");
    for _ in 0..20000 {
        diff_process(rng.next_i32(), rng.next_i32(), "C17");
    }
    // biased toward the small codes so the non-default arms are hit often
    for _ in 0..20000 {
        diff_process(rng.range_i32(-8, 13), rng.next_i32(), "C17-biased");
    }
}

// ===========================================================================
// copy_data_block  (rows C18 - C22)   and   handle_pointer_operations (C24)
// ===========================================================================

#[test]
fn cfg_c18_c20_payload_shapes() {
    // C18: all zeros
    diff_copy_data_block(&[0u8; DATABLOCK_SIZE], 0xAA, 0, "C18-zeros");
    // C19: all 0xFF -> value is a NaN, label has no NUL terminator
    diff_copy_data_block(&[0xFFu8; DATABLOCK_SIZE], 0x00, 0, "C19-ones");
    // C20: fully random 40 bytes, padding included
    let mut rng = Rng::for_test("C20");
    for i in 0..2000 {
        let mut src = [0u8; DATABLOCK_SIZE];
        rng.fill(&mut src);
        // Ensure the padding regions carry non-zero, distinguishable bytes so a
        // field-wise (rather than byte-wise) copy would be detected.
        src[OFF_PAD1..OFF_PAD1 + 4].copy_from_slice(&[0xDE, 0xAD, 0xBE, 0xEF]);
        src[OFF_TAILPAD..OFF_TAILPAD + 4].copy_from_slice(&[0xCA, 0xFE, 0xBA, 0xBE]);
        let sentinel = rng.next_u8();
        diff_copy_data_block(&src, sentinel, 0, &format!("C20-{i}"));
    }
}

#[test]
fn cfg_c21_struct_typed_shapes() {
    let mut rng = Rng::for_test("C21");
    let labels: Vec<Vec<u8>> = vec![
        vec![],                    // empty label
        b"Source".to_vec(),        // the label overunder uses
        vec![b'x'; 19],            // 19 bytes + implicit NUL
        vec![b'y'; 20],            // 20 bytes, no NUL at all
        b"\x00\xffmid-NUL".to_vec(),
    ];
    let values: Vec<f64> = vec![
        0.0,
        -0.0,
        1.0,
        -1.0,
        f64::NAN,
        f64::INFINITY,
        f64::NEG_INFINITY,
        f64::MIN_POSITIVE,
        f64::from_bits(1),
        f64::MAX,
        f64::MIN,
    ];
    for label in &labels {
        for &value in &values {
            for _ in 0..8 {
                let id = rng.next_i32();
                let mut src = [0u8; DATABLOCK_SIZE];
                src[OFF_ID..OFF_ID + 4].copy_from_slice(&id.to_ne_bytes());
                rng.fill(&mut src[OFF_PAD1..OFF_PAD1 + 4]); // garbage padding
                src[OFF_VALUE..OFF_VALUE + 8].copy_from_slice(&value.to_ne_bytes());
                let n = label.len().min(20);
                src[OFF_LABEL..OFF_LABEL + n].copy_from_slice(&label[..n]);
                rng.fill(&mut src[OFF_TAILPAD..OFF_TAILPAD + 4]);
                diff_copy_data_block(&src, rng.next_u8(), 0, "C21");
            }
        }
    }
}

#[test]
fn cfg_c22_arena_offsets() {
    let mut rng = Rng::for_test("C22");
    // Both offsets keep 8-byte alignment, which the DataBlock* ABI requires.
    for dest_off in [0usize, 8, 16, 24, 40] {
        for _ in 0..500 {
            let mut src = [0u8; DATABLOCK_SIZE];
            rng.fill(&mut src);
            diff_copy_data_block(&src, rng.next_u8(), dest_off, &format!("C22-off{dest_off}"));
        }
    }
}

#[test]
fn cfg_c24_hpo_full_range() {
    let mut vals = vec![
        0,
        1,
        -1,
        2,
        -2,
        i32::MAX / 2,
        i32::MAX / 2 + 1,
        i32::MIN / 2,
        i32::MIN / 2 - 1,
        i32::MAX,
        i32::MAX - 1,
        i32::MIN,
        i32::MIN + 1,
        1_073_741_823,
        1_073_741_824,
        -1_073_741_824,
        -1_073_741_825,
    ];
    vals.extend(i32_corners());
    let mut rng = Rng::for_test("C24");
    for _ in 0..2000 {
        vals.push(rng.next_i32());
    }
    for v in vals {
        diff_hpo(v, "C24");
    }
}

// Rows C25 - C40 (every `overunder` row) live in tests/phase_overunder.rs, which
// uses a custom single-threaded harness because they must redirect fd 1.
