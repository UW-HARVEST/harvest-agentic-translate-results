//! Phase B — valid-path differential tests. One test per CONFIGS.md row.
//!
//! Both libraries are driven exclusively through their `.so` exports.

mod common;
use common::{assert_same, capture_stdout, libs, Rng, N, SEED};

// ---------------------------------------------------------------- helpers ----

fn f(bits: u32) -> f32 {
    f32::from_bits(bits)
}

/// Random binary32 with the given sign and a normal exponent (1..=254).
fn rand_normal(rng: &mut Rng, sign: u32) -> f32 {
    let exp = 1 + rng.below(254); // 1..=254 -> normal
    let mant = rng.next_u32() & 0x007f_ffff;
    f((sign << 31) | (exp << 23) | mant)
}

/// Random subnormal: exponent field 0, mantissa non-zero.
fn rand_subnormal(rng: &mut Rng, sign: u32) -> f32 {
    let mut mant = rng.next_u32() & 0x007f_ffff;
    if mant == 0 {
        mant = 1;
    }
    f((sign << 31) | mant)
}

/// Random quiet NaN: exp all ones, mantissa MSB set.
fn rand_qnan(rng: &mut Rng) -> f32 {
    let sign = rng.next_u32() >> 31;
    let mant = 0x0040_0000 | (rng.next_u32() & 0x003f_ffff);
    f((sign << 31) | (0xff << 23) | mant)
}

/// Random signalling NaN: exp all ones, mantissa MSB clear, mantissa non-zero.
fn rand_snan(rng: &mut Rng) -> f32 {
    let sign = rng.next_u32() >> 31;
    let mut mant = rng.next_u32() & 0x003f_ffff;
    if mant == 0 {
        mant = 1;
    }
    f((sign << 31) | (0xff << 23) | mant)
}

// ------------------------------------------------------- rows 1 & 2: zeros ---

#[test]
fn row01_positive_zero() {
    assert_same("row01 +0.0", &[0.0f32]);
    // and repeated, to be sure it is not order-dependent
    assert_same("row01 +0.0 x16", &[0.0f32; 16]);
}

#[test]
fn row02_negative_zero() {
    assert_same("row02 -0.0", &[-0.0f32]);
    assert_same("row02 both zeros", &[0.0f32, -0.0f32, 0.0f32, -0.0f32]);
}

// ------------------------------------------------ rows 3 & 4: small ints -----

#[test]
fn row03_small_positive_integers() {
    let v: Vec<f32> = (1..=1024).map(|i| i as f32).collect();
    assert_same("row03 1..=1024", &v);
}

#[test]
fn row04_small_negative_integers() {
    let v: Vec<f32> = (1..=1024).map(|i| -(i as f32)).collect();
    assert_same("row04 -1..=-1024", &v);
}

// --------------------------------------------- rows 5 & 6: random normals ----

#[test]
fn row05_random_normal_positive() {
    let mut rng = Rng::new(SEED);
    let v: Vec<f32> = (0..N).map(|_| rand_normal(&mut rng, 0)).collect();
    assert_same("row05 random +normal", &v);
}

#[test]
fn row06_random_normal_negative() {
    let mut rng = Rng::new(SEED ^ 0x11);
    let v: Vec<f32> = (0..N).map(|_| rand_normal(&mut rng, 1)).collect();
    assert_same("row06 random -normal", &v);
}

// ------------------------------------------ rows 7 & 8: random subnormals ----

#[test]
fn row07_random_subnormal_positive() {
    let mut rng = Rng::new(SEED ^ 0x22);
    let v: Vec<f32> = (0..N).map(|_| rand_subnormal(&mut rng, 0)).collect();
    assert_same("row07 random +subnormal", &v);
}

#[test]
fn row08_random_subnormal_negative() {
    let mut rng = Rng::new(SEED ^ 0x33);
    let v: Vec<f32> = (0..N).map(|_| rand_subnormal(&mut rng, 1)).collect();
    assert_same("row08 random -subnormal", &v);
}

// ------------------------------------------------------ row 9: infinities ----

#[test]
fn row09_infinities() {
    assert_same(
        "row09 inf",
        &[f32::INFINITY, f32::NEG_INFINITY, f32::INFINITY, f32::NEG_INFINITY],
    );
}

// ---------------------------------------------------- rows 10 & 11: NaNs -----

#[test]
fn row10_random_quiet_nans() {
    let mut rng = Rng::new(SEED ^ 0x44);
    let mut v: Vec<f32> = (0..N).map(|_| rand_qnan(&mut rng)).collect();
    v.push(f(0x7fc0_0000)); // canonical qNaN
    v.push(f(0xffc0_0000)); // negative canonical qNaN
    v.push(f32::NAN);
    assert_same("row10 quiet NaNs", &v);
}

#[test]
fn row11_random_signalling_nans() {
    let mut rng = Rng::new(SEED ^ 0x55);
    let mut v: Vec<f32> = (0..N).map(|_| rand_snan(&mut rng)).collect();
    v.push(f(0x7f80_0001)); // minimal sNaN
    v.push(f(0xff80_0001)); // negative minimal sNaN
    v.push(f(0x7fbf_ffff)); // maximal sNaN
    assert_same("row11 signalling NaNs", &v);
}

// --------------------------------------------- row 12: boundary constants ----

#[test]
fn row12_boundary_constants() {
    let v = vec![
        f32::MIN_POSITIVE,  // 0x00800000
        -f32::MIN_POSITIVE, // 0x80800000
        f32::MAX,           // 0x7f7fffff
        f32::MIN,           // 0xff7fffff
        f32::EPSILON,       // 0x34000000
        -f32::EPSILON,
        f(0x0000_0001), // smallest positive subnormal
        f(0x8000_0001), // smallest negative subnormal
        f(0x007f_ffff), // largest positive subnormal
        f(0x807f_ffff), // largest negative subnormal
        1.0,
        -1.0,
    ];
    assert_same("row12 boundary constants", &v);
}

// ----------------------------------------------- rows 13 & 14: byte shapes ---

#[test]
fn row13_all_bytes_below_0x10_zero_padding() {
    // Every byte < 0x10 so `%02x` must emit a leading zero for each.
    let mut v = vec![
        f(0x0f0e_0100),
        f(0x0000_0001),
        f(0x0102_0304),
        f(0x0000_0100),
        f(0x0001_0000),
        f(0x0100_0000),
    ];
    let mut rng = Rng::new(SEED ^ 0x66);
    for _ in 0..N {
        let b = |r: &mut Rng| r.below(0x10);
        let bits = b(&mut rng) | (b(&mut rng) << 8) | (b(&mut rng) << 16) | (b(&mut rng) << 24);
        v.push(f(bits));
    }
    assert_same("row13 bytes < 0x10", &v);
}

#[test]
fn row14_all_bytes_high_bit_set_promotion() {
    // Every byte >= 0x80: catches any sign-extension bug in the
    // `unsigned char` -> `int` variadic promotion.
    let mut v = vec![f(0xffff_ffff), f(0x8080_8080), f(0xfe80_ff81)];
    let mut rng = Rng::new(SEED ^ 0x77);
    for _ in 0..N {
        let b = |r: &mut Rng| 0x80 + r.below(0x80);
        let bits = b(&mut rng) | (b(&mut rng) << 8) | (b(&mut rng) << 16) | (b(&mut rng) << 24);
        v.push(f(bits));
    }
    assert_same("row14 bytes >= 0x80", &v);
}

// --------------------------------------- row 15: 00/ff permutations (order) ---

#[test]
fn row15_zero_ff_permutations_endianness() {
    let mut v = Vec::new();
    for mask in 0u32..16 {
        let mut bits = 0u32;
        for byte in 0..4 {
            if mask & (1 << byte) != 0 {
                bits |= 0xffu32 << (8 * byte);
            }
        }
        v.push(f(bits));
    }
    assert_eq!(v.len(), 16);
    assert_same("row15 00/ff permutations", &v);
}

// ------------------------------------------------ row 16: single-bit walk ----

#[test]
fn row16_single_bit_walk() {
    let v: Vec<f32> = (0..32).map(|k| f(1u32 << k)).collect();
    assert_same("row16 single-bit walk", &v);
    // ...and the complement of each single bit.
    let v: Vec<f32> = (0..32).map(|k| f(!(1u32 << k))).collect();
    assert_same("row16 single-bit-clear walk", &v);
}

// ------------------------------------------- row 17: arbitrary bit patterns ---

#[test]
fn row17_uniform_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 0x88);
    // Several large batches so the total sample is big without one huge capture.
    for batch in 0..8 {
        let v: Vec<f32> = (0..8192).map(|_| f(rng.next_u32())).collect();
        assert_same(&format!("row17 uniform random batch {batch}"), &v);
    }
}

// ------------------------------ row 18: exhaustive high 16 bits sweep --------

#[test]
fn row18_exhaustive_high16_sweep() {
    let mut rng = Rng::new(SEED ^ 0x99);
    // 65536 values: every sign/exponent/high-mantissa combination exactly once,
    // low 16 bits randomized. Split into chunks of 8192.
    for chunk in 0..8u32 {
        let v: Vec<f32> = (0..8192u32)
            .map(|i| {
                let hi = chunk * 8192 + i;
                f((hi << 16) | (rng.next_u32() & 0xffff))
            })
            .collect();
        assert_same(&format!("row18 high16 sweep chunk {chunk}"), &v);
    }
}

// ------------------------------------------------- rows 19-21: call counts ---

#[test]
fn row19_single_call_exact_bytes() {
    let l = libs();
    let c = capture_stdout(|| unsafe { (l.c_driver)(1.0f32) });
    let r = capture_stdout(|| unsafe { (l.rust_driver)(1.0f32) });
    assert_eq!(c, r, "row19: single-call output differs");
    assert_eq!(c.len(), 9, "row19: expected exactly 9 bytes, got {:?}", c);
    assert_eq!(c, b"0000803f\n", "row19: unexpected C output {:?}", String::from_utf8_lossy(&c));
}

#[test]
fn row20_two_calls_concatenate() {
    let l = libs();
    let c = capture_stdout(|| unsafe {
        (l.c_driver)(1.0f32);
        (l.c_driver)(-2.5f32);
    });
    let r = capture_stdout(|| unsafe {
        (l.rust_driver)(1.0f32);
        (l.rust_driver)(-2.5f32);
    });
    assert_eq!(c, r, "row20: two-call output differs");
    assert_eq!(c.len(), 18, "row20: expected 18 bytes, got {:?}", c);
}

#[test]
fn row21_many_calls_stream_buffering() {
    let mut rng = Rng::new(SEED ^ 0xAA);
    let v: Vec<f32> = (0..100_000).map(|_| f(rng.next_u32())).collect();
    assert_same("row21 100k calls", &v);
}

// -------------------------------------------- row 22: interleaved libraries ---

#[test]
fn row22_interleaved_c_and_rust_same_stream() {
    let l = libs();
    let mut rng = Rng::new(SEED ^ 0xBB);
    let v: Vec<f32> = (0..2048).map(|_| f(rng.next_u32())).collect();

    // C, Rust, C, Rust ... onto one shared stdout stream.
    let interleaved = capture_stdout(|| {
        for &x in &v {
            unsafe {
                (l.c_driver)(x);
                (l.rust_driver)(x);
            }
        }
    });
    // Each pair of consecutive 9-byte records must be identical.
    assert_eq!(interleaved.len(), v.len() * 18, "row22: unexpected length");
    for (i, rec) in interleaved.chunks(18).enumerate() {
        assert_eq!(
            &rec[..9],
            &rec[9..],
            "row22: record {i} differs (input 0x{:08x}): C={:?} RUST={:?}",
            v[i].to_bits(),
            String::from_utf8_lossy(&rec[..9]),
            String::from_utf8_lossy(&rec[9..])
        );
    }

    // Rust-first ordering too.
    let interleaved = capture_stdout(|| {
        for &x in &v {
            unsafe {
                (l.rust_driver)(x);
                (l.c_driver)(x);
            }
        }
    });
    for (i, rec) in interleaved.chunks(18).enumerate() {
        assert_eq!(&rec[..9], &rec[9..], "row22 (rust first): record {i} differs");
    }
}

// ----------------------------------- row 23: calling convention / no folding ---

#[test]
fn row23_runtime_computed_arguments() {
    // Values the optimiser cannot constant-fold: derived from a runtime source
    // and passed straight into the FFI call.
    let mut rng = Rng::new(SEED ^ 0xCC);
    let seedy = std::hint::black_box(rng.next_u32());
    let mut v = Vec::new();
    let mut acc = seedy;
    for i in 0..N {
        acc = acc.wrapping_mul(1_664_525).wrapping_add(1_013_904_223);
        // mix in arithmetic results (values that arrive in xmm registers)
        let a = std::hint::black_box(f(acc));
        let b = std::hint::black_box(i as f32) * std::hint::black_box(0.1f32);
        v.push(a);
        v.push(b);
        v.push(std::hint::black_box(a + b));
        v.push(std::hint::black_box(a * b));
        v.push(std::hint::black_box(a / b));
        v.push(std::hint::black_box(-a));
    }
    assert_same("row23 runtime-computed args", &v);
}

// ------------------------ row 24: every byte value in every byte position ----

#[test]
fn row24_every_byte_value_in_every_position() {
    let mut v = Vec::with_capacity(4 * 256);
    for pos in 0..4u32 {
        for byte in 0..256u32 {
            // vary the other bytes too so positions are distinguishable
            let base = 0x1122_3344u32 & !(0xffu32 << (8 * pos));
            v.push(f(base | (byte << (8 * pos))));
        }
    }
    assert_eq!(v.len(), 1024);
    assert_same("row24 byte x position matrix", &v);
}

// ------------------------- deep sweeps (opt-in: `cargo test -- --ignored`) ----

/// Strided sweep over the ENTIRE 2^32 input space (prime stride 4093, so the
/// sample walks every region of the exponent/mantissa/sign space).
#[test]
#[ignore = "slow: ~1M FFI calls per library"]
fn deep01_strided_sweep_of_full_32bit_space() {
    const STRIDE: u32 = 4093;
    let mut batch: Vec<f32> = Vec::with_capacity(1 << 18);
    let mut bits: u64 = 0;
    let mut n = 0usize;
    let mut chunk = 0usize;
    while bits < u32::MAX as u64 {
        batch.push(f(bits as u32));
        bits += STRIDE as u64;
        if batch.len() == (1 << 18) {
            assert_same(&format!("deep01 full-space stride chunk {chunk}"), &batch);
            n += batch.len();
            chunk += 1;
            batch.clear();
        }
    }
    if !batch.is_empty() {
        assert_same("deep01 full-space stride tail", &batch);
        n += batch.len();
    }
    assert!(n > 1_000_000, "deep01 only covered {n} values");
    println!("deep01 compared {n} distinct bit patterns across the full 2^32 space");
}

/// Exhaustive over all 2^16 low bits for several fixed high halves (every IEEE
/// class: zero/subnormal, small normal, mid normal, large normal, inf/NaN).
#[test]
#[ignore = "slow: ~0.5M FFI calls per library"]
fn deep02_exhaustive_low16_for_every_class() {
    for hi in [
        0x0000u32, // +zero / +subnormal
        0x8000,    // -zero / -subnormal
        0x0080,    // smallest +normal
        0x3f80,    // around 1.0
        0x7f7f,    // near FLT_MAX
        0x7f80,    // +inf / +sNaN
        0x7fc0,    // +qNaN
        0xff80,    // -inf / -sNaN
    ] {
        let v: Vec<f32> = (0..=0xffffu32).map(|lo| f((hi << 16) | lo)).collect();
        assert_eq!(v.len(), 65536);
        assert_same(&format!("deep02 exhaustive low16, hi=0x{hi:04x}"), &v);
    }
}
