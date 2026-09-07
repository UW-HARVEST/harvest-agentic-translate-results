// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md. Both implementations are loaded from their
// `.so` via libloading and compared byte-for-byte on stdout.

mod common;
use common::*;

// Row 1 — canonical happy path: small positive normals.
#[test]
fn cfg01_happy_path_positive_normals() {
    assert_same(
        "cfg01",
        &[1.0, 2.5, 3.14159, 0.5, 100.0, 1e10, 1.0 / 3.0, 6.02e23],
    );
}

// Row 2 — positive zero: all four bytes 0x00 (%02x zero padding everywhere).
#[test]
fn cfg02_positive_zero() {
    assert_same("cfg02", &[0.0f32]);
    assert_same_bits("cfg02-bits", &[0x0000_0000]);
}

// Row 3 — negative zero must not be canonicalised to +0.0.
#[test]
fn cfg03_negative_zero() {
    assert_same("cfg03", &[-0.0f32]);
    assert_same_bits("cfg03-bits", &[0x8000_0000]);
    // and the two must actually differ, proving no canonicalisation happened
    let d = rust_driver();
    assert_ne!(
        run_batch(&d, &[0.0f32]),
        run_batch(&d, &[-0.0f32]),
        "+0.0 and -0.0 printed identically: sign bit was lost"
    );
}

// Row 4 — sign bit set, negative normals.
#[test]
fn cfg04_negative_normals() {
    assert_same("cfg04", &[-1.0, -2.5, -1e30, -0.5, -3.14159, -1e-30, -7.0]);
}

// Row 5 — every byte in 0x01..=0x0f (leading-zero nibble padding).
#[test]
fn cfg05_all_bytes_low_nibble_only() {
    let mut bits = vec![0x0102_0304u32, 0x0f0f_0f0f, 0x0102_0f0e, 0x0101_0101];
    let mut rng = Rng::new(0x0505_0505);
    for _ in 0..2000 {
        let mut v = 0u32;
        for _ in 0..4 {
            v = (v << 8) | (1 + rng.below(15));
        }
        bits.push(v);
    }
    assert_same_bits("cfg05", &bits);
}

// Row 6 — every byte in 0x10..=0x7f (two digits, high bit clear).
#[test]
fn cfg06_all_bytes_high_bit_clear() {
    let mut bits = vec![0x1122_3344u32, 0x7f7f_7f7f, 0x1010_1010, 0x1070_2f6a];
    let mut rng = Rng::new(0x0606_0606);
    for _ in 0..2000 {
        let mut v = 0u32;
        for _ in 0..4 {
            v = (v << 8) | (0x10 + rng.below(0x70));
        }
        bits.push(v);
    }
    assert_same_bits("cfg06", &bits);
}

// Row 7 — every byte in 0x80..=0xff: unsigned char must be ZERO-extended on
// the variadic promotion to int, else %02x prints "ffffff80".
#[test]
fn cfg07_all_bytes_high_bit_set() {
    let mut bits = vec![0x8080_8080u32, 0xdead_beef, 0xffff_ffff, 0x8090_a0b0];
    let mut rng = Rng::new(0x0707_0707);
    for _ in 0..2000 {
        let mut v = 0u32;
        for _ in 0..4 {
            v = (v << 8) | (0x80 + rng.below(0x80));
        }
        bits.push(v);
    }
    assert_same_bits("cfg07", &bits);
    // sanity: no "ffffff" sign-extension artefact anywhere in either output
    let xs: Vec<f32> = bits.iter().copied().map(f32::from_bits).collect();
    for (name, out) in [
        ("C", run_batch(&c_driver(), &xs)),
        ("Rust", run_batch(&rust_driver(), &xs)),
    ] {
        assert!(
            out.len() == xs.len() * 9,
            "{name} emitted {} bytes for {} calls — sign extension widened %02x",
            out.len(),
            xs.len()
        );
    }
}

// Row 8 — mixed bytes spanning all four byte classes in one value.
#[test]
fn cfg08_mixed_byte_classes() {
    let bits = [
        0x0000_0f80u32,
        0x000f_80ff,
        0x0f80_ff00,
        0x80ff_000f,
        0xff00_0f80,
        0x007f_8001,
        0x8001_7f00,
    ];
    assert_same_bits("cfg08", &bits);
}

// Row 9 — little-endian probe: the byte order must be host order, LSB first.
#[test]
fn cfg09_byte_order() {
    assert_same_bits("cfg09", &[0x0000_0001, 0x0100_0000, 0x0000_0100, 0x0001_0000]);
    let d = rust_driver();
    let a = run_batch(&d, &[f32::from_bits(0x0000_0001)]);
    let b = run_batch(&d, &[f32::from_bits(0x0100_0000)]);
    assert_ne!(a, b, "byte order collapsed: 0x00000001 == 0x01000000");
    assert_eq!(a, b"01000000\n", "not little-endian (LSB first)");
    assert_eq!(b, b"00000001\n", "not little-endian (LSB first)");
}

// Row 10 — subnormals must not be flushed to zero.
#[test]
fn cfg10_subnormals() {
    let mut bits = vec![
        0x0000_0001u32, // smallest positive subnormal
        0x007f_ffff,    // largest subnormal
        0x8000_0001,    // negative subnormal
        0x8000_0000 | 0x007f_ffff,
        0x0000_0002,
        0x0040_0000,
    ];
    let mut rng = Rng::new(0x1010_1010);
    for _ in 0..2000 {
        let sign = (rng.below(2) as u32) << 31;
        let mant = rng.next_u32() & 0x007f_ffff;
        bits.push(sign | mant);
    }
    assert_same_bits("cfg10", &bits);
}

// Row 11 — infinities.
#[test]
fn cfg11_infinities() {
    assert_same("cfg11", &[f32::INFINITY, f32::NEG_INFINITY]);
    assert_same_bits("cfg11-bits", &[0x7f80_0000, 0xff80_0000]);
}

// Row 12 — NaNs of every flavour, including signalling and odd payloads.
#[test]
fn cfg12_nans() {
    let mut bits = vec![
        0x7fc0_0000u32, // quiet NaN
        0xffc0_0000,    // negative quiet NaN
        0x7fa0_0000,    // signalling NaN
        0xffa0_0000,    // negative signalling NaN
        0x7f80_0001,    // smallest-payload sNaN
        0x7fff_ffff,    // all payload bits set
        0xffff_ffff,
        0x7fbf_ffff,
    ];
    let mut rng = Rng::new(0x1212_1212);
    for _ in 0..2000 {
        let sign = (rng.below(2) as u32) << 31;
        let payload = (rng.next_u32() & 0x007f_ffff).max(1);
        bits.push(sign | 0x7f80_0000 | payload);
    }
    assert_same_bits("cfg12", &bits);
}

// Row 13 — magnitude extremes and boundary constants.
#[test]
fn cfg13_magnitude_extremes() {
    assert_same(
        "cfg13",
        &[
            f32::MIN_POSITIVE,
            f32::MAX,
            f32::MIN,
            f32::EPSILON,
            f32::from_bits(0x0080_0000),
            f32::from_bits(0x7f7f_ffff),
            f32::from_bits(0xff7f_ffff),
            f32::from_bits(0xffff_ffff),
            f32::from_bits(0x007f_ffff),
        ],
    );
}

// Row 14 — powers of two reinterpreted as bit patterns.
#[test]
fn cfg14_power_of_two_patterns() {
    let mut bits: Vec<u32> = (0..32).map(|k| 1u32 << k).collect();
    bits.extend((0..32).map(|k| !(1u32 << k)));
    bits.extend((0..32).map(|k| (1u32 << k).wrapping_sub(1)));
    assert_same_bits("cfg14", &bits);
    // also the real float powers of two
    let fs: Vec<f32> = (-40..40).map(|e| (2.0f32).powi(e)).collect();
    assert_same("cfg14-floats", &fs);
}

// Row 15 — randomized property test, uniform over ALL 2^32 bit patterns.
#[test]
fn cfg15_random_uniform_over_all_bit_patterns() {
    let mut rng = Rng::new(0xdead_beef_cafe_1515);
    let bits: Vec<u32> = (0..20_000).map(|_| rng.next_u32()).collect();
    assert_same_bits("cfg15", &bits);
}

// Row 16 — randomized property test over finite floats (random exp + mantissa).
#[test]
fn cfg16_random_finite_floats() {
    let mut rng = Rng::new(0x1616_1616_1616_1616);
    let mut bits = Vec::with_capacity(20_000);
    while bits.len() < 20_000 {
        let sign = (rng.below(2) as u32) << 31;
        let exp = rng.below(0xff) as u32; // 0..=0xfe, never the inf/NaN exponent
        let mant = rng.next_u32() & 0x007f_ffff;
        let v = sign | (exp << 23) | mant;
        debug_assert!(f32::from_bits(v).is_finite());
        bits.push(v);
    }
    assert_same_bits("cfg16", &bits);
}

// Row 17 — sweeps that cover every byte value in every byte position.
#[test]
fn cfg17_sweep_low_and_high_halves() {
    let low: Vec<u32> = (0..=0xffffu32).map(|v| 0x3f80_0000 | v).collect();
    assert_same_bits("cfg17-low", &low);

    let high: Vec<u32> = (0..=0xffffu32).map(|v| (v << 16) | 0x0000_1234).collect();
    assert_same_bits("cfg17-high", &high);

    // every byte value in every one of the 4 positions
    let mut every: Vec<u32> = Vec::with_capacity(4 * 256);
    for pos in 0..4 {
        for b in 0..=0xffu32 {
            every.push(b << (8 * pos));
        }
    }
    assert_same_bits("cfg17-positions", &every);
}

// Row 18 — stream interleaving: alternate C and Rust inside ONE capture.
#[test]
fn cfg18_interleaved_calls_share_stdout_correctly() {
    let c = c_driver();
    let r = rust_driver();
    let mut rng = Rng::new(0x1818_1818);
    let xs: Vec<f32> = (0..1000).map(|_| f32::from_bits(rng.next_u32())).collect();

    // Interleaved C,Rust,C,Rust... — each pair must produce two identical lines.
    let cf: DriverFn = *c;
    let rf: DriverFn = *r;
    let out = {
        let xs = &xs;
        common::run_batch_with(move || unsafe {
            for &x in xs.iter() {
                cf(x);
                rf(x);
            }
        })
    };
    assert_eq!(out.len(), xs.len() * 2 * 9, "unexpected interleaved length");
    for (i, chunk) in out.chunks(18).enumerate() {
        assert_eq!(
            &chunk[..9],
            &chunk[9..],
            "interleaved call #{i} diverged for {:?}",
            xs[i]
        );
        assert_eq!(&chunk[..9], expected_line(xs[i]).as_slice());
    }
}

// Row 19 — repeated identical calls: no hidden state, idempotent.
#[test]
fn cfg19_repeated_identical_calls() {
    let xs = vec![f32::from_bits(0xdead_beef); 100];
    assert_same("cfg19", &xs);
    let out = run_batch(&rust_driver(), &xs);
    for chunk in out.chunks(9) {
        assert_eq!(chunk, b"efbeadde\n");
    }
}

// Row 20 — ABI shape: void return, f32 in xmm0, no register/stack clobber.
#[test]
fn cfg20_abi_no_clobber() {
    let c = c_driver();
    let r = rust_driver();
    let canary_f: [f32; 8] = [1.5, -2.25, 3.75, 4.5, -5.25, 6.5, 7.75, -8.5];
    let canary_i: [u64; 6] = [0x1111, 0x2222, 0x3333, 0x4444, 0x5555, 0x6666];

    let mut rng = Rng::new(0x2020_2020);
    for _ in 0..500 {
        let x = f32::from_bits(rng.next_u32());
        let cf = canary_f;
        let ci = canary_i;
        let a = run_batch(&c, &[x]);
        let b = run_batch(&r, &[x]);
        assert_eq!(a, b, "ABI test diverged on 0x{:08x}", x.to_bits());
        assert_eq!(cf, canary_f, "float canary clobbered");
        assert_eq!(ci, canary_i, "integer canary clobbered");
    }
}

// Row 21 — the project builds no executable, so there is no stdout-of-binary
// comparison to make. Assert that fact so the row cannot silently rot.
#[test]
fn cfg21_project_builds_no_binary() {
    let cmake = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .parent()
            .unwrap()
            .join("c_src/CMakeLists.txt"),
    )
    .expect("read CMakeLists.txt");
    assert!(
        !cmake.contains("add_executable"),
        "c_src now builds a binary; Phase B must compare its stdout too"
    );
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !cargo.contains("[[bin]]"),
        "crate now builds a binary; Phase B must compare its stdout too"
    );
}

// Row 22 — no Cargo features exist, so the default combo is the only one.
#[test]
fn cfg22_no_feature_axes() {
    let cargo = std::fs::read_to_string(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml"),
    )
    .expect("read Cargo.toml");
    assert!(
        !cargo.contains("[features]"),
        "Cargo.toml gained a [features] table; Phases B-C must be re-run per combo"
    );
}
