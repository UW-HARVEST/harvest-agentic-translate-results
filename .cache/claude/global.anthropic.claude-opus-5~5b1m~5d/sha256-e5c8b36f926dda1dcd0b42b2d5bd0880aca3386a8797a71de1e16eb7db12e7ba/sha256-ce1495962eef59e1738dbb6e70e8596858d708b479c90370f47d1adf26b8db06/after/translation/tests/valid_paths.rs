// Phase B — valid-path differential tests, one test per row of CONFIGS.md.
//
// Every test loads BOTH the C `.so` and the Rust `.so` via libloading and
// compares the bytes their exported `driver` writes to stdout.

mod common;

use common::*;

const SEED: u64 = 0x0BAD_C0DE_1234_5678;

// --- C1 -------------------------------------------------------------------
#[test]
fn c1_positive_zero() {
    assert_each_matches("C1", &[0.0f32]);
    // All four bytes are 0x00: every digit must be zero-padded.
    let out = run_one(Impl::C, 0.0f32);
    assert_eq!(&out[..], b"00000000\n", "C output for +0.0");
    assert_eq!(run_one(Impl::Rust, 0.0f32), out);
}

// --- C2 -------------------------------------------------------------------
#[test]
fn c2_negative_zero() {
    let neg_zero = f32::from_bits(0x8000_0000);
    assert_each_matches("C2", &[neg_zero]);
    let out = run_one(Impl::C, neg_zero);
    assert_eq!(&out[..], b"00000080\n", "C output for -0.0 (LE)");
    assert_eq!(run_one(Impl::Rust, neg_zero), out);
    // and it must be distinguishable from +0.0
    assert_ne!(out, run_one(Impl::C, 0.0f32));
}

// --- C3 -------------------------------------------------------------------
#[test]
fn c3_positive_subnormals() {
    let mut rng = Rng::new(SEED ^ 3);
    let mut xs = vec![
        f32::from_bits(0x0000_0001), // smallest positive subnormal
        f32::from_bits(0x0000_0002),
        f32::from_bits(0x0040_0000),
        f32::from_bits(0x007f_ffff), // largest subnormal
    ];
    for _ in 0..20_000 {
        let mantissa = rng.range(1, 0x7f_ffff);
        xs.push(from_fields(0, 0, mantissa));
    }
    assert_batch_matches("C3", &xs);
    assert_each_matches("C3-corners", &xs[..4]);
}

// --- C4 -------------------------------------------------------------------
#[test]
fn c4_negative_subnormals() {
    let mut rng = Rng::new(SEED ^ 4);
    let mut xs = vec![
        f32::from_bits(0x8000_0001),
        f32::from_bits(0x807f_ffff),
    ];
    for _ in 0..20_000 {
        let mantissa = rng.range(1, 0x7f_ffff);
        xs.push(from_fields(1, 0, mantissa));
    }
    assert_batch_matches("C4", &xs);
    assert_each_matches("C4-corners", &xs[..2]);
}

// --- C5 -------------------------------------------------------------------
#[test]
fn c5_subnormal_normal_boundary() {
    let xs = [
        f32::from_bits(0x007f_fffe),
        f32::from_bits(0x007f_ffff), // largest subnormal
        f32::from_bits(0x0080_0000), // FLT_MIN, smallest normal
        f32::from_bits(0x0080_0001),
        f32::from_bits(0x807f_ffff),
        f32::from_bits(0x8080_0000), // -FLT_MIN
    ];
    assert_each_matches("C5", &xs);
}

// --- C6 -------------------------------------------------------------------
#[test]
fn c6_positive_normals() {
    let mut rng = Rng::new(SEED ^ 6);
    let mut xs = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let exponent = rng.range(0x01, 0xfe);
        let mantissa = rng.below(0x80_0000);
        xs.push(from_fields(0, exponent, mantissa));
    }
    assert_batch_matches("C6", &xs);
}

// --- C7 -------------------------------------------------------------------
#[test]
fn c7_negative_normals() {
    let mut rng = Rng::new(SEED ^ 7);
    let mut xs = Vec::with_capacity(50_000);
    for _ in 0..50_000 {
        let exponent = rng.range(0x01, 0xfe);
        let mantissa = rng.below(0x80_0000);
        xs.push(from_fields(1, exponent, mantissa));
    }
    assert_batch_matches("C7", &xs);
}

// --- C8 -------------------------------------------------------------------
#[test]
fn c8_small_exact_integers() {
    let mut rng = Rng::new(SEED ^ 8);
    let mut xs: Vec<f32> = (0..=32).map(|i| i as f32).collect();
    for i in 1..=1000 {
        xs.push(i as f32);
        xs.push(-(i as f32));
    }
    for _ in 0..10_000 {
        let v = rng.range(0, 1_000_000) as i64 - 500_000;
        xs.push(v as f32);
    }
    assert_batch_matches("C8", &xs);
    assert_each_matches("C8-small", &xs[..33]);
}

// --- C9 -------------------------------------------------------------------
#[test]
fn c9_fractions_and_decimals() {
    let mut rng = Rng::new(SEED ^ 9);
    let mut xs = vec![
        0.1f32, 0.5, 1.5, -1.5, 2.25, 3.14159, -3.14159, 1e-10, 1e10, -1e-10, -1e10,
        1.0 / 3.0, 2.0f32.sqrt(), std::f32::consts::E, std::f32::consts::PI,
    ];
    for _ in 0..20_000 {
        let num = rng.next_u32() as f64;
        let den = (rng.range(1, u32::MAX)) as f64;
        let v = (num / den) as f32;
        xs.push(v);
        xs.push(-v);
    }
    assert_batch_matches("C9", &xs);
    assert_each_matches("C9-named", &xs[..15]);
}

// --- C10 ------------------------------------------------------------------
#[test]
fn c10_range_extremes() {
    let xs = [
        f32::MAX,                    // 0x7f7fffff
        -f32::MAX,                   // 0xff7fffff
        f32::MIN_POSITIVE,           // 0x00800000
        -f32::MIN_POSITIVE,          // 0x80800000
        f32::EPSILON,                // 0x34000000
        -f32::EPSILON,
    ];
    assert_each_matches("C10", &xs);
    assert_eq!(f32::MAX.to_bits(), 0x7f7f_ffff);
    assert_eq!(run_one(Impl::C, f32::MAX), b"ffff7f7f\n".to_vec());
    assert_eq!(run_one(Impl::Rust, f32::MAX), b"ffff7f7f\n".to_vec());
}

// --- C11 ------------------------------------------------------------------
#[test]
fn c11_infinities() {
    let xs = [f32::INFINITY, f32::NEG_INFINITY];
    assert_each_matches("C11", &xs);
    assert_eq!(run_one(Impl::C, f32::INFINITY), b"0000807f\n".to_vec());
    assert_eq!(run_one(Impl::Rust, f32::INFINITY), b"0000807f\n".to_vec());
    assert_eq!(run_one(Impl::C, f32::NEG_INFINITY), b"000080ff\n".to_vec());
    assert_eq!(run_one(Impl::Rust, f32::NEG_INFINITY), b"000080ff\n".to_vec());
}

// --- C12 ------------------------------------------------------------------
#[test]
fn c12_nan_payloads() {
    let mut rng = Rng::new(SEED ^ 12);
    let mut xs = vec![
        f32::from_bits(0x7fc0_0000), // canonical qNaN
        f32::from_bits(0xffc0_0000), // negative qNaN
        f32::from_bits(0x7f80_0001), // smallest sNaN
        f32::from_bits(0xff80_0001), // negative sNaN
        f32::from_bits(0x7fff_ffff),
        f32::from_bits(0xffff_ffff),
    ];
    for _ in 0..20_000 {
        let sign = rng.bit();
        let mantissa = rng.range(1, 0x7f_ffff); // non-zero => NaN, not inf
        xs.push(from_fields(sign, 0xff, mantissa));
    }
    assert_batch_matches("C12", &xs);
    assert_each_matches("C12-corners", &xs[..6]);
}

// --- C13 ------------------------------------------------------------------
#[test]
fn c13_every_byte_value_in_every_position() {
    // Build inputs so that each of the 4 byte positions takes every value
    // 0x00..=0xFF, driving `%02x` across its whole domain (zero padding, the
    // lowercase a-f digits, and bytes >= 0x80 which would sign-extend if the
    // `char raw[]` copy were promoted as signed).
    let mut xs = Vec::new();
    for pos in 0..4usize {
        for v in 0u32..=0xff {
            let bits = v << (8 * pos as u32);
            xs.push(f32::from_bits(bits));
            xs.push(f32::from_bits(bits | 0x5a5a_5a5a & !(0xffu32 << (8 * pos as u32))));
        }
    }
    assert_batch_matches("C13", &xs);

    // Structural: the union of all produced digits must be exactly [0-9a-f].
    let out = run_batch(Impl::C, &xs);
    let mut seen = std::collections::BTreeSet::new();
    for &b in &out {
        if b != b'\n' {
            seen.insert(b);
        }
    }
    let expected: std::collections::BTreeSet<u8> =
        b"0123456789abcdef".iter().copied().collect();
    assert_eq!(seen, expected, "digit alphabet used by C output");
}

// --- C14 ------------------------------------------------------------------
#[test]
fn c14_uniform_random_bit_patterns() {
    let mut rng = Rng::new(SEED ^ 14);
    // 200_000 uniformly random 32-bit patterns reinterpreted as float, in
    // batches so the shared stdout buffer is exercised too.
    const TOTAL: usize = 200_000;
    const BATCH: usize = 20_000;
    let mut done = 0;
    while done < TOTAL {
        let n = BATCH.min(TOTAL - done);
        let xs: Vec<f32> = (0..n).map(|_| f32::from_bits(rng.next_u32())).collect();
        assert_batch_matches("C14", &xs);
        done += n;
    }
}

// --- C15 ------------------------------------------------------------------
#[test]
fn c15_exhaustive_structured_sweep() {
    // Every exponent x both signs x a set of mantissa corner values.
    let mantissas = [
        0u32,
        1,
        2,
        0x0f,
        0x10,
        0x7f,
        0x80,
        0xff,
        0x5a5a,
        0x40_0000,
        0x7f_fffe,
        0x7f_ffff,
    ];
    let mut xs = Vec::new();
    for sign in 0..2u32 {
        for exponent in 0..=0xffu32 {
            for &m in &mantissas {
                xs.push(from_fields(sign, exponent, m));
            }
        }
    }
    assert_eq!(xs.len(), 2 * 256 * mantissas.len());
    assert_batch_matches("C15", &xs);
}

// --- C16 ------------------------------------------------------------------
#[test]
fn c16_zero_calls_produces_no_output() {
    let c_out = capture(|| {});
    assert!(c_out.is_empty(), "empty capture must be empty, got {c_out:?}");
    let c_out = run_batch(Impl::C, &[]);
    let rust_out = run_batch(Impl::Rust, &[]);
    assert_eq!(c_out, rust_out);
    assert!(c_out.is_empty(), "zero driver calls must emit nothing");
}

// --- C17 ------------------------------------------------------------------
#[test]
fn c17_single_call_shape() {
    for &x in &[0.0f32, 1.0, -1.0, f32::INFINITY, f32::from_bits(0xdead_beef)] {
        let c_out = run_one(Impl::C, x);
        let rust_out = run_one(Impl::Rust, x);
        assert_eq!(c_out, rust_out, "single-call bytes for 0x{:08x}", x.to_bits());
        assert_eq!(
            c_out.len(),
            9,
            "one call must emit exactly 8 hex digits + newline, got {:?}",
            String::from_utf8_lossy(&c_out)
        );
        assert_eq!(*c_out.last().unwrap(), b'\n');
    }
}

// --- C18 ------------------------------------------------------------------
#[test]
fn c18_many_sequential_calls_concatenate() {
    let mut rng = Rng::new(SEED ^ 18);
    let xs: Vec<f32> = (0..500).map(|_| f32::from_bits(rng.next_u32())).collect();

    let c_batch = run_batch(Impl::C, &xs);
    let rust_batch = run_batch(Impl::Rust, &xs);
    assert_eq!(c_batch, rust_batch, "N-call output must match");

    // The N-call output must equal the concatenation of the N single-call
    // outputs: no state carries between calls in either implementation.
    let mut c_concat = Vec::new();
    let mut rust_concat = Vec::new();
    for &x in &xs {
        c_concat.extend_from_slice(&run_one(Impl::C, x));
        rust_concat.extend_from_slice(&run_one(Impl::Rust, x));
    }
    assert_eq!(c_batch, c_concat, "C: batched != concatenated singles");
    assert_eq!(rust_batch, rust_concat, "Rust: batched != concatenated singles");
}

// --- C19 ------------------------------------------------------------------
#[test]
fn c19_c_and_rust_alternating_into_one_stream() {
    let mut rng = Rng::new(SEED ^ 19);
    let xs: Vec<f32> = (0..400).map(|_| f32::from_bits(rng.next_u32())).collect();

    let c = driver_fn(Impl::C);
    let r = driver_fn(Impl::Rust);

    // Interleave the two implementations into the same fd-1 stream. If either
    // buffered privately instead of using libc's shared `stdout`, the records
    // would come out reordered relative to this expectation.
    let interleaved = capture(|| {
        for &x in &xs {
            unsafe {
                c(x);
                r(x);
            }
        }
    });

    let mut expected = Vec::new();
    for &x in &xs {
        expected.extend_from_slice(&oracle_one(x));
        expected.extend_from_slice(&oracle_one(x));
    }
    assert_eq!(
        interleaved, expected,
        "interleaved C/Rust output must be strictly in call order"
    );
}

// --- C20 ------------------------------------------------------------------
#[test]
fn c20_fully_buffered_file_destination() {
    // `capture` always redirects fd 1 to a regular file, which glibc treats as
    // fully buffered (4096-byte blocks). Emit far more than one block so any
    // flush-timing difference would show up as truncation or reordering.
    let mut rng = Rng::new(SEED ^ 20);
    let xs: Vec<f32> = (0..5_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    let c_out = run_batch(Impl::C, &xs);
    let rust_out = run_batch(Impl::Rust, &xs);
    assert_eq!(c_out.len(), xs.len() * 9, "no bytes lost across block boundaries");
    assert_eq!(c_out, rust_out);
    assert_eq!(c_out, oracle_batch(&xs));
}

// --- C21 ------------------------------------------------------------------
#[test]
fn c21_structural_invariant_of_every_record() {
    let mut rng = Rng::new(SEED ^ 21);
    let xs: Vec<f32> = (0..20_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    for which in [Impl::C, Impl::Rust] {
        let out = run_batch(which, &xs);
        assert_eq!(out.len(), xs.len() * 9, "{}: record length", which.name());
        for (i, rec) in out.chunks(9).enumerate() {
            assert_eq!(rec[8], b'\n', "{}: record {i} must end with \\n", which.name());
            for &d in &rec[..8] {
                assert!(
                    d.is_ascii_digit() || (b'a'..=b'f').contains(&d),
                    "{}: record {i} contains non-lowercase-hex byte {:#04x}",
                    which.name(),
                    d
                );
            }
        }
    }
}

// --- C22 ------------------------------------------------------------------
#[test]
fn c22_byte_order_matches_native_representation() {
    let mut rng = Rng::new(SEED ^ 22);
    let xs: Vec<f32> = (0..20_000).map(|_| f32::from_bits(rng.next_u32())).collect();
    let c_out = run_batch(Impl::C, &xs);
    let rust_out = run_batch(Impl::Rust, &xs);
    let oracle = oracle_batch(&xs);
    assert_eq!(c_out, oracle, "C must print native (to_ne_bytes) order");
    assert_eq!(rust_out, oracle, "Rust must print native (to_ne_bytes) order");

    // On a little-endian target the first printed byte is the low byte.
    if cfg!(target_endian = "little") {
        let out = run_one(Impl::C, f32::from_bits(0x0403_0201));
        assert_eq!(&out[..], b"01020304\n");
        assert_eq!(run_one(Impl::Rust, f32::from_bits(0x0403_0201)), out);
    }
}
