// Phase C -- error-path / boundary differential tests.
//
// One test per row of ERRORS.md. `void driver(double)` is a total function with
// no rejection path (see ERRORS.md for the mechanical grep proving this), so for
// each row the assertion is that C and Rust agree on the SAME non-rejection:
// byte-identical output, exactly one line, no crash, no trap. Where ERRORS.md
// records the precise expected rendering, that exact string is asserted too, so
// a test cannot pass by both sides being equally wrong.

use crate::common::*;

/// Run one bit pattern through both implementations and return the shared output,
/// failing if they disagree or if either produced something other than one line.
pub fn shared_output(row: &str, bits: u64) -> String {
    assert_same(row, &[bits]);
    let i = impls();
    let c = capture_stdout(|| unsafe { (i.c)(f64::from_bits(bits)) });
    let r = capture_stdout(|| unsafe { (i.rust)(f64::from_bits(bits)) });
    assert_eq!(c, r, "[{row}] bits 0x{bits:016x}: outputs differ");
    let s = String::from_utf8(c).expect("output is UTF-8");
    assert!(
        s.ends_with('\n') && s.matches('\n').count() == 1,
        "[{row}] expected exactly one newline-terminated line, got {s:?}"
    );
    s
}

// E1 -- null pointer is NOT EXPRESSIBLE (no pointer parameter in the ABI).
// Nearest analogue: the all-zero-bits argument.
pub fn err_e1_all_zero_bits_no_pointer_to_be_null() {
    let s = shared_output("E1", 0x0000_0000_0000_0000);
    assert_eq!(s, "0 0x0p+0 0.0000\n", "E1: unexpected rendering of +0.0");
}

// E2 -- zero length is NOT EXPRESSIBLE (no length parameter).
// Nearest analogue: -0.0, the boundary that separates %llx from %.4f rendering.
pub fn err_e2_negative_zero() {
    let s = shared_output("E2", 0x8000_0000_0000_0000);
    assert_eq!(
        s, "8000000000000000 -0x0p+0 -0.0000\n",
        "E2: sign of negative zero not preserved"
    );
}

// E3 -- oversized length is NOT EXPRESSIBLE (no length parameter).
// Nearest analogue: +/-DBL_MAX, the longest output the API can produce.
pub fn err_e3_dbl_max_longest_output() {
    for (bits, neg) in [(f64::MAX.to_bits(), false), (f64::MIN.to_bits(), true)] {
        let s = shared_output("E3", bits);
        assert!(
            s.len() > 300,
            "E3: expected the long %.4f expansion, got {} bytes",
            s.len()
        );
        assert!(s.ends_with(".0000\n"), "E3: unexpected tail: {:?}", &s[s.len() - 10..]);
        // no truncation: the %.4f field must carry the full integer expansion
        let fixed = s.rsplit(' ').next().unwrap();
        let int_digits = fixed.split('.').next().unwrap().trim_start_matches('-').len();
        assert_eq!(int_digits, 309, "E3: %.4f integer part was truncated");
        assert_eq!(s.starts_with('-'), false, "E3: %llx field never carries a sign");
        if neg {
            assert!(s.contains(" -0x1."), "E3: negative sign missing from %a");
        }
    }
}

// E4 -- one step past the finite range, high end: +INFINITY.
pub fn err_e4_positive_infinity() {
    let s = shared_output("E4", 0x7FF0_0000_0000_0000);
    assert_eq!(s, "7ff0000000000000 inf inf\n", "E4: +inf rendering");
    // and the finite value one ULP below must still be finite in both
    let s2 = shared_output("E4", f64::MAX.to_bits());
    assert!(!s2.contains("inf"), "E4: DBL_MAX must not render as inf");
}

// E5 -- one step past the finite range, low end: -INFINITY.
pub fn err_e5_negative_infinity() {
    let s = shared_output("E5", 0xFFF0_0000_0000_0000);
    assert_eq!(s, "fff0000000000000 -inf -inf\n", "E5: -inf rendering");
}

// E6 -- positive quiet NaN: a value with no valid numeric interpretation, the
// float analogue of an out-of-range enum crossing the FFI boundary.
pub fn err_e6_quiet_nan_positive() {
    let s = shared_output("E6", 0x7FF8_0000_0000_0000);
    assert_eq!(s, "7ff8000000000000 nan nan\n", "E6: +qNaN rendering");
}

// E7 -- negative quiet NaN: the sign bit must survive to produce `-nan`.
pub fn err_e7_quiet_nan_negative() {
    let s = shared_output("E7", 0xFFF8_0000_0000_0000);
    assert_eq!(s, "fff8000000000000 -nan -nan\n", "E7: -qNaN rendering");
}

// E8 -- SIGNALLING NaN. Must not be quieted in transit (which would change the
// %llx bits) and must not raise a trap.
pub fn err_e8_signalling_nan() {
    let s = shared_output("E8", 0x7FF0_0000_0000_0001);
    assert_eq!(
        s, "7ff0000000000001 nan nan\n",
        "E8: signalling NaN was quieted or mis-rendered"
    );
    // the mantissa MSB must still be CLEAR in the printed bits, i.e. still sNaN
    let hex = s.split(' ').next().unwrap();
    let printed = u64::from_str_radix(hex, 16).unwrap();
    assert_eq!(
        printed, 0x7FF0_0000_0000_0001,
        "E8: bit pattern altered in transit (sNaN quieted)"
    );
    assert_eq!(printed & (1u64 << 51), 0, "E8: quiet bit got set");

    // a spread of signalling payloads, both signs
    let mut v = Vec::new();
    let mut rng = Rng::new(SEED ^ 0xE8);
    for _ in 0..1500 {
        let payload = (rng.mantissa() & !(1u64 << 51)) | 1;
        v.push(compose(rng.next_u64() & 1, 0x7FF, payload));
    }
    assert_same("E8(sweep)", &v);
}

// E9 -- NaN payloads must be reproduced verbatim by %llx even though %a collapses
// every NaN to `nan`.
pub fn err_e9_nan_payload_preserved() {
    for bits in [
        0x7FF8_DEAD_BEEF_CAFEu64,
        0xFFF8_DEAD_BEEF_CAFE,
        0x7FFF_FFFF_FFFF_FFFF,
        0xFFFF_FFFF_FFFF_FFFF,
        0x7FF0_0000_0000_0001,
        0x7FF4_8000_0000_0000,
    ] {
        let s = shared_output("E9", bits);
        let hex = s.split(' ').next().unwrap();
        assert_eq!(
            u64::from_str_radix(hex, 16).unwrap(),
            bits,
            "E9: payload bits not preserved for 0x{bits:016x} (got {s:?})"
        );
        let tail: Vec<&str> = s.trim_end().split(' ').collect();
        let want = if bits >> 63 == 1 { "-nan" } else { "nan" };
        assert_eq!(tail[1], want, "E9: %a spelling for 0x{bits:016x}");
        assert_eq!(tail[2], want, "E9: %.4f spelling for 0x{bits:016x}");
    }
}

// E10 -- smallest positive subnormal: %a must switch to the 0x0.…p-1022 form.
pub fn err_e10_smallest_subnormal() {
    let s = shared_output("E10", 0x0000_0000_0000_0001);
    assert_eq!(
        s, "1 0x0.0000000000001p-1022 0.0000\n",
        "E10: smallest subnormal rendering"
    );
}

// E11 -- the subnormal/normal transition: the %a mantissa form changes across it.
pub fn err_e11_subnormal_normal_boundary() {
    let largest_sub = 0x000F_FFFF_FFFF_FFFFu64;
    let smallest_norm = 0x0010_0000_0000_0000u64;

    let sub = shared_output("E11", largest_sub);
    let norm = shared_output("E11", smallest_norm);

    assert!(
        sub.contains("0x0.") && sub.contains("p-1022"),
        "E11: largest subnormal should use the 0x0.…p-1022 form, got {sub:?}"
    );
    assert!(
        norm.contains("0x1p-1022"),
        "E11: smallest normal should use the 0x1p-1022 form, got {norm:?}"
    );

    // both signs, and a few ULPs either side of the boundary
    let mut v = Vec::new();
    for base in [largest_sub, smallest_norm] {
        for d in 0..4u64 {
            v.push(base.wrapping_sub(d));
            v.push(base.wrapping_add(d));
            v.push(base.wrapping_sub(d) | (1u64 << 63));
            v.push(base.wrapping_add(d) | (1u64 << 63));
        }
    }
    assert_same("E11(boundary sweep)", &v);
}

// E12 -- %.4f underflow must retain the sign: -1e-300 prints -0.0000, not 0.0000.
pub fn err_e12_tiny_negative_signed_zero() {
    let s = shared_output("E12", (-1e-300f64).to_bits());
    let fixed = s.trim_end().rsplit(' ').next().unwrap();
    assert_eq!(fixed, "-0.0000", "E12: sign lost when %.4f underflows: {s:?}");

    let p = shared_output("E12", 1e-300f64.to_bits());
    let pfixed = p.trim_end().rsplit(' ').next().unwrap();
    assert_eq!(pfixed, "0.0000", "E12: positive underflow should be unsigned");

    // sweep tiny magnitudes of both signs
    let mut rng = Rng::new(SEED ^ 0xE12);
    let mut v = Vec::new();
    for _ in 0..2000 {
        v.push(compose(rng.next_u64() & 1, rng.below(1000), rng.mantissa()));
    }
    assert_same("E12(sweep)", &v);
}

// E13 -- %.4f round-half-to-even ties, decided off the exact binary value.
pub fn err_e13_round_half_even_ties() {
    let mut v = Vec::new();
    for base in [0.00005f64, 0.00015, 0.00025, 0.00035, 2.5e-5, 7.5e-5, 1.00005, 0.000050000000001] {
        for s in [1.0f64, -1.0] {
            v.push((base * s).to_bits());
        }
    }
    // exact binary ties: k/2^n lands exactly halfway at the 4th decimal for
    // some k, which is where round-half-even is actually observable.
    let mut rng = Rng::new(SEED ^ 0xE13);
    for _ in 0..3000 {
        let k = rng.below(1 << 22) as f64;
        for d in [16.0f64, 256.0, 65536.0] {
            v.push((k / d).to_bits());
            v.push((-(k / d)).to_bits());
        }
    }
    assert_same("E13", &v);

    // spot-check that both agree on a genuine tie rather than merely both erring
    let s = shared_output("E13", 0.00005f64.to_bits());
    let fixed = s.trim_end().rsplit(' ').next().unwrap();
    assert!(
        fixed == "0.0000" || fixed == "0.0001",
        "E13: unexpected tie resolution {fixed:?}"
    );
}

// E14 -- no bit pattern is rejected: a large randomized sweep of the raw domain,
// asserting every call yields exactly one line from both implementations.
pub fn err_e14_raw_bit_pattern_sweep() {
    let mut rng = Rng::new(SEED ^ 0xE14);
    let v: Vec<u64> = (0..20_000).map(|_| rng.next_u64()).collect();
    assert_same("E14", &v);

    let i = impls();
    for (name, d) in [("C", i.c), ("Rust", i.rust)] {
        let out = capture_stdout(|| {
            for &b in v.iter().take(4096) {
                unsafe { d(f64::from_bits(b)) };
            }
        });
        let lines = out.iter().filter(|&&c| c == b'\n').count();
        assert_eq!(
            lines, 4096,
            "{name}: every input must be accepted and produce exactly one line"
        );
    }
}

// Extra: the three fields must always be present and space-separated, for every
// value class -- a structural invariant that holds precisely because there is no
// error path that could suppress output.
pub fn err_output_shape_invariant_across_all_classes() {
    let mut rng = Rng::new(SEED ^ 0xF00D);
    let mut v = vec![
        0x0,
        0x8000_0000_0000_0000,
        0x7FF0_0000_0000_0000,
        0xFFF0_0000_0000_0000,
        0x7FF8_0000_0000_0000,
        0x1,
        0x000F_FFFF_FFFF_FFFF,
        0x0010_0000_0000_0000,
        f64::MAX.to_bits(),
        f64::MIN.to_bits(),
    ];
    for _ in 0..2000 {
        v.push(rng.next_u64());
    }
    assert_same("shape", &v);

    let i = impls();
    for &b in v.iter().take(300) {
        let out = capture_stdout(|| unsafe { (i.rust)(f64::from_bits(b)) });
        let s = String::from_utf8_lossy(&out);
        let line = s.trim_end_matches('\n');
        let fields: Vec<&str> = line.split(' ').collect();
        assert_eq!(
            fields.len(),
            3,
            "expected 3 space-separated fields for 0x{b:016x}, got {s:?}"
        );
        assert_eq!(
            u64::from_str_radix(fields[0], 16).unwrap(),
            b,
            "field 0 must be the raw bit pattern of 0x{b:016x}"
        );
    }
}
