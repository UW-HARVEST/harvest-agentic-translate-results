//! Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//!
//! Every test loads BOTH the C `.so` and the Rust `.so` via `libloading` and
//! compares the returned `float` as raw bits.

mod common;

use common::*;

/// Rows that sweep a whole (n, mantissa) sub-space exhaustively.
fn sweep(label: &str, ns: impl IntoIterator<Item = u16>, mants: &[u16]) -> usize {
    let mut count = 0;
    for n in ns {
        for &m in mants {
            let h = h_from(n, m);
            let bits = assert_same(label, h);
            // Cross-check against the oracle parsed out of the C source, so a
            // *shared* mistake in both libraries could not slip through.
            assert_eq!(
                bits,
                oracle_bits(h),
                "[{label}] half2float(0x{h:04X}) disagrees with the C-source oracle"
            );
            count += 1;
        }
    }
    count
}

const RANDOM_PER_ROW: usize = 4096;

// ---------------------------------------------------------------------- C1
#[test]
fn c1_positive_zero_index_zero() {
    // n = 0, mantissa = 0, offset 0x0000 -> m__mantissa[0], exponent 0.
    let bits = assert_same("C1", 0x0000);
    assert_eq!(bits, 0x0000_0000, "C1 must be +0.0");
    assert_eq!(bits, oracle_bits(0x0000));
}

// ---------------------------------------------------------------------- C2
#[test]
fn c2_positive_subnormal_source_offset0() {
    // n = 0, mantissa 1..=1023 -> index 1..=1023, exponent 0.
    let mants: Vec<u16> = (1..=1023).collect();
    let n = sweep("C2", 0..=0, &mants);
    assert_eq!(n, 1023);

    let mut rng = Rng::new(SEED ^ 2);
    for _ in 0..RANDOM_PER_ROW {
        let m = rng.range(1, 1023) as u16;
        assert_same("C2/rand", h_from(0, m));
    }
}

// ---------------------------------------------------------------------- C3
#[test]
fn c3_negative_zero_index_zero() {
    // n = 32 -> offset 0x0000, exponent 0x80000000.
    let bits = assert_same("C3", 0x8000);
    assert_eq!(bits, 0x8000_0000, "C3 must be -0.0");
    // Differs from +0.0 only in the sign bit -- and is NOT equal to it as f32.
    assert_eq!(bits ^ 0x8000_0000, oracle_bits(0x0000));
}

// ---------------------------------------------------------------------- C4
#[test]
fn c4_negative_subnormal_source_offset0() {
    let mants: Vec<u16> = (1..=1023).collect();
    let n = sweep("C4", 32..=32, &mants);
    assert_eq!(n, 1023);

    let mut rng = Rng::new(SEED ^ 4);
    for _ in 0..RANDOM_PER_ROW {
        let m = rng.range(1, 1023) as u16;
        assert_same("C4/rand", h_from(32, m));
    }
}

// ---------------------------------------------------------------------- C5
#[test]
fn c5_positive_normals_mantissa_zero_index_1024() {
    // offset 0x0400 + mantissa 0 -> the region boundary index 1024.
    let n = sweep("C5", 1..=30, &[0]);
    assert_eq!(n, 30);
}

// ---------------------------------------------------------------------- C6
#[test]
fn c6_positive_normals_random_mantissa() {
    let mut rng = Rng::new(SEED ^ 6);
    for _ in 0..RANDOM_PER_ROW * 4 {
        let n = rng.range(1, 30) as u16;
        let m = rng.range(1, 1022) as u16;
        let h = h_from(n, m);
        assert_eq!(assert_same("C6", h), oracle_bits(h));
    }
}

// ---------------------------------------------------------------------- C7
#[test]
fn c7_positive_normals_mantissa_max_index_2047() {
    let n = sweep("C7", 1..=30, &[1023]);
    assert_eq!(n, 30);
}

// ---------------------------------------------------------------------- C8
#[test]
fn c8_positive_infinity_special_exponent_row_31() {
    let bits = assert_same("C8", 0x7C00);
    assert_eq!(bits, 0x7F80_0000, "C8 must be +inf");
    assert!(f32::from_bits(bits).is_infinite() && f32::from_bits(bits) > 0.0);
}

// ---------------------------------------------------------------------- C9
#[test]
fn c9_positive_nan_payloads_row_31() {
    let mants: Vec<u16> = (1..=1023).collect();
    let cnt = sweep("C9", 31..=31, &mants);
    assert_eq!(cnt, 1023);
    // Every one of them must be a NaN whose payload survived the union pun.
    for m in 1..=1023u16 {
        let h = h_from(31, m);
        let bits = assert_same("C9/class", h);
        assert!(
            f32::from_bits(bits).is_nan(),
            "0x{h:04X} -> 0x{bits:08X} should be NaN"
        );
        assert_ne!(bits & 0x007F_FFFF, 0, "NaN payload of 0x{h:04X} was lost");
        assert_eq!(bits >> 31, 0, "0x{h:04X} must be a positive NaN");
    }
}

// ---------------------------------------------------------------------- C10
#[test]
fn c10_negative_normals_mantissa_zero() {
    let n = sweep("C10", 33..=62, &[0]);
    assert_eq!(n, 30);
}

// ---------------------------------------------------------------------- C11
#[test]
fn c11_negative_normals_random_mantissa() {
    let mut rng = Rng::new(SEED ^ 11);
    for _ in 0..RANDOM_PER_ROW * 4 {
        let n = rng.range(33, 62) as u16;
        let m = rng.range(1, 1022) as u16;
        let h = h_from(n, m);
        assert_eq!(assert_same("C11", h), oracle_bits(h));
    }
}

// ---------------------------------------------------------------------- C12
#[test]
fn c12_negative_normals_mantissa_max() {
    let n = sweep("C12", 33..=62, &[1023]);
    assert_eq!(n, 30);
}

// ---------------------------------------------------------------------- C13
#[test]
fn c13_negative_infinity_special_exponent_row_63() {
    let bits = assert_same("C13", 0xFC00);
    assert_eq!(bits, 0xFF80_0000, "C13 must be -inf");
    assert!(f32::from_bits(bits).is_infinite() && f32::from_bits(bits) < 0.0);
}

// ---------------------------------------------------------------------- C14
#[test]
fn c14_negative_nan_payloads_row_63() {
    let mants: Vec<u16> = (1..=1023).collect();
    let cnt = sweep("C14", 63..=63, &mants);
    assert_eq!(cnt, 1023);
    for m in 1..=1023u16 {
        let h = h_from(63, m);
        let bits = assert_same("C14/class", h);
        assert!(f32::from_bits(bits).is_nan());
        assert_eq!(bits >> 31, 1, "0x{h:04X} must be a negative NaN");
    }
    // The largest input, i.e. the largest u32 sum reached anywhere (axis F).
    assert_eq!(assert_same("C14/max", 0xFFFF), 0xFFFF_E000);
}

// ---------------------------------------------------------------------- C15
#[test]
fn c15_offset_region_aliasing_discriminator() {
    // m__mantissa[512] == m__mantissa[1024] == 0x38000000, so a constant or
    // swapped m__offset would still be right for *some* inputs. Compare the
    // two index regions where their step differs (0x4000 vs 0x2000).
    let t = c_tables();
    assert_eq!(t.mantissa[512], t.mantissa[1024], "aliasing precondition");
    assert_ne!(
        t.mantissa[513], t.mantissa[1025],
        "regions must be distinguishable at index 513/1025"
    );

    // All offset-0 rows (n = 0 and n = 32) with a high mantissa -> index 512..1023.
    let mut cnt = 0;
    for n in [0u16, 32] {
        for m in 512..=1023u16 {
            let h = h_from(n, m);
            assert_eq!(assert_same("C15/offset0", h), oracle_bits(h));
            cnt += 1;
        }
    }
    assert_eq!(cnt, 1024);

    // The mirror-image inputs in the offset-0x400 region.
    for n in [1u16, 31, 33, 63] {
        for m in 512..=1023u16 {
            let h = h_from(n, m);
            assert_eq!(assert_same("C15/offset400", h), oracle_bits(h));
        }
    }
}

// ---------------------------------------------------------------------- C16
#[test]
fn c16_full_row_times_boundary_mantissa_cross_product() {
    let mants = [0u16, 1, 511, 512, 1022, 1023];
    let cnt = sweep("C16", 0..=63, &mants);
    assert_eq!(cnt, 64 * 6);
}

// ---------------------------------------------------------------------- C17
#[test]
fn c17_result_class_sweep() {
    #[derive(Debug, PartialEq)]
    enum Class {
        PosZero,
        NegZero,
        PosInf,
        NegInf,
        PosNan,
        NegNan,
        PosFinite,
        NegFinite,
    }
    fn classify(b: u32) -> Class {
        let neg = b >> 31 == 1;
        let f = f32::from_bits(b);
        if f == 0.0 {
            if neg { Class::NegZero } else { Class::PosZero }
        } else if f.is_nan() {
            if neg { Class::NegNan } else { Class::PosNan }
        } else if f.is_infinite() {
            if neg { Class::NegInf } else { Class::PosInf }
        } else if neg {
            Class::NegFinite
        } else {
            Class::PosFinite
        }
    }

    let cases: &[(u16, Class)] = &[
        (0x0000, Class::PosZero),
        (0x8000, Class::NegZero),
        (0x0001, Class::PosFinite), // smallest positive subnormal source
        (0x8001, Class::NegFinite),
        (0x03FF, Class::PosFinite), // largest offset-0 mantissa
        (0x83FF, Class::NegFinite),
        (0x3C00, Class::PosFinite), // 1.0
        (0xBC00, Class::NegFinite), // -1.0
        (0x7BFF, Class::PosFinite), // largest finite half
        (0xFBFF, Class::NegFinite),
        (0x7C00, Class::PosInf),
        (0xFC00, Class::NegInf),
        (0x7C01, Class::PosNan),
        (0xFC01, Class::NegNan),
        (0x7E00, Class::PosNan),
        (0xFFFF, Class::NegNan),
    ];

    for &(h, ref want) in cases {
        let bits = assert_same("C17", h);
        assert_eq!(bits, oracle_bits(h));
        assert_eq!(
            &classify(bits),
            want,
            "half2float(0x{h:04X}) -> 0x{bits:08X} landed in the wrong result class"
        );
    }

    // Spot-check a few exact values a real consumer would rely on.
    assert_eq!(assert_same("C17", 0x3C00), 0x3F80_0000); // 1.0
    assert_eq!(assert_same("C17", 0x4000), 0x4000_0000); // 2.0
    assert_eq!(assert_same("C17", 0xC000), 0xC000_0000); // -2.0
    assert_eq!(assert_same("C17", 0x0001), 0x3380_0000); // 2^-24
}

// ---------------------------------------------------------------------- C18
#[test]
fn c18_exhaustive_domain() {
    // The complete valid input space of the library: all 65 536 uint16_t values.
    let l = libs();
    let c = l.c_fn();
    let mut mismatches = Vec::new();
    for (name, r) in l.rust_variants() {
        for h in 0..=u16::MAX {
            let cb = unsafe { c(h) }.to_bits();
            let rb = unsafe { r(h) }.to_bits();
            let ob = oracle_bits(h);
            if cb != rb || cb != ob {
                mismatches.push(format!(
                    "0x{h:04X}: C=0x{cb:08X} {name}=0x{rb:08X} oracle=0x{ob:08X}"
                ));
                if mismatches.len() >= 16 {
                    break;
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "exhaustive sweep found {} mismatch(es): {mismatches:?}",
        mismatches.len(),
    );
}

// ---------------------------------------------------------------------- C19
#[test]
fn c19_call_order_independence_and_purity() {
    let l = libs();
    let c = l.c_fn();

    // Reference answers in ascending order.
    let mut expect = vec![0u32; 65536];
    for h in 0..=u16::MAX {
        expect[h as usize] = unsafe { c(h) }.to_bits();
    }

    // Fisher-Yates shuffle with the fixed seed, then replay interleaved 3x.
    let mut order: Vec<u16> = (0..=u16::MAX).collect();
    let mut rng = Rng::new(SEED ^ 19);
    for i in (1..order.len()).rev() {
        let j = rng.below(i as u64 + 1) as usize;
        order.swap(i, j);
    }

    let variants = l.rust_variants();
    for &h in &order {
        for _ in 0..3 {
            let cb = unsafe { c(h) }.to_bits();
            assert_eq!(
                cb, expect[h as usize],
                "C is not pure: half2float(0x{h:04X}) changed to 0x{cb:08X}"
            );
            for (name, r) in &variants {
                let rb = unsafe { r(h) }.to_bits();
                assert_eq!(
                    rb, expect[h as usize],
                    "{name} diverges/is not pure: half2float(0x{h:04X}) = 0x{rb:08X}, want 0x{:08X}",
                    expect[h as usize]
                );
            }
        }
    }
}

// ---------------------------------------------------------------------- C20
#[test]
fn c20_randomized_property_sweep_both_orders() {
    let l = libs();
    let c = l.c_fn();
    let variants = l.rust_variants();
    let mut rng = Rng::new(SEED ^ 20);
    for i in 0..200_000u32 {
        let h = rng.next_u16();
        for (name, r) in &variants {
            let (cb, rb) = if i % 2 == 0 {
                // C first, then Rust.
                let cb = unsafe { c(h) }.to_bits();
                let rb = unsafe { r(h) }.to_bits();
                (cb, rb)
            } else {
                // Rust first, then C.
                let rb = unsafe { r(h) }.to_bits();
                let cb = unsafe { c(h) }.to_bits();
                (cb, rb)
            };
            assert_eq!(
                cb, rb,
                "iteration {i}: half2float(0x{h:04X}) C=0x{cb:08X} {name}=0x{rb:08X}"
            );
            assert_eq!(cb, oracle_bits(h));
        }
    }
}
