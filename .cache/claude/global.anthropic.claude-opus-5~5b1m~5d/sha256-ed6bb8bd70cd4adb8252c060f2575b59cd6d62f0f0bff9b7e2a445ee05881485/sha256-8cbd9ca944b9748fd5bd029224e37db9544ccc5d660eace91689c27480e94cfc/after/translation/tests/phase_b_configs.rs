//! Phase B — valid-path differential tests, one test per `CONFIGS.md` row.
//!
//! Both libraries are exercised only through their `.so` exports.

mod harness;

use harness::*;

/// Row 1 — `x == 0`: all four bytes `0x00`, `%02x` zero-padding at every offset.
fn row01_zero(libs: &Libs) {    assert_same(libs, 0, "row 1");
    // Sanity-anchor the expected C rendering (little-endian, 4 bytes).
    assert_eq!(c_out(libs, 0), b"00000000\n");
}

/// Row 2 — `x == -1` (`0xffffffff`): every byte has its high bit set, so a
/// sign-extending translation would print `ffffffff` per byte.
fn row02_all_ones(libs: &Libs) {    assert_same(libs, -1, "row 2");
    assert_eq!(c_out(libs, -1), b"ffffffff\n");
}

/// Row 3 — `x == INT_MAX`.
fn row03_int_max(libs: &Libs) {    assert_same(libs, i32::MAX, "row 3");
}

/// Row 4 — `x == INT_MIN`: top byte exactly `0x80`, the high-bit boundary.
fn row04_int_min(libs: &Libs) {    assert_same(libs, i32::MIN, "row 4");
}

/// Row 5 — boundary neighbours one step inside/past the extremes and the
/// `0x7fffffff <-> 0x80000000` wrap pair.
fn row05_boundary_neighbours(libs: &Libs) {    let xs = [
        i32::MAX,
        i32::MAX - 1,
        i32::MAX.wrapping_add(1), // == INT_MIN, the wrap
        i32::MIN,
        i32::MIN + 1,
        i32::MIN.wrapping_sub(1), // == INT_MAX, the wrap back
        -2,
        -1,
        0,
        1,
        2,
        0x7fff_ffffu32 as i32,
        0x8000_0000u32 as i32,
        0xffff_ffffu32 as i32,
    ];
    for x in xs {
        assert_same(libs, x, "row 5");
    }
    assert_same_batch(libs, &xs, "row 5 (batched)");
}

/// Row 6 — low-nibble byte (`0x01..=0x0f`) at each of the 4 native offsets:
/// the padding x byte-position cross-product.
fn row06_low_nibble_each_offset(libs: &Libs) {    let mut xs = Vec::new();
    for off in 0..4usize {
        for n in 1u8..=0x0f {
            xs.push(with_byte(0, off, n));
        }
    }
    assert_eq!(xs.len(), 60);
    for &x in &xs {
        assert_same(libs, x, "row 6");
    }
    assert_same_batch(libs, &xs, "row 6 (batched)");
}

/// Row 7 — high-bit byte (`0x80..=0xff`) at each of the 4 native offsets:
/// the integer-promotion x byte-position cross-product.
fn row07_high_bit_each_offset(libs: &Libs) {    let mut xs = Vec::new();
    for off in 0..4usize {
        for v in 0x80u8..=0xff {
            xs.push(with_byte(0, off, v));
            xs.push(with_byte(-1, off, v));
        }
    }
    assert_eq!(xs.len(), 1024);
    assert_same_batch(libs, &xs, "row 7 (batched)");
    // Spot-check a subset through the per-call path as well.
    for &x in xs.iter().step_by(37) {
        assert_same(libs, x, "row 7");
    }
}

/// Row 8 — exhaustive sweep: each of the 4 offsets x all 256 byte values, with
/// the remaining bytes held at `0x00` and again at `0xff` (2048 inputs).
fn row08_exhaustive_byte_sweep(libs: &Libs) {    let mut xs = Vec::new();
    for base in [0i32, -1i32] {
        for off in 0..4usize {
            for v in 0..=255u8 {
                xs.push(with_byte(base, off, v));
            }
        }
    }
    assert_eq!(xs.len(), 2048);
    assert_same_batch(libs, &xs, "row 8 (batched)");

    // Additionally exhaust two full 16-bit slices of the input domain: every
    // value of the low half-word, and every value of the high half-word.
    let low: Vec<i32> = (0..=0xffffu32).map(|v| v as i32).collect();
    let high: Vec<i32> = (0..=0xffffu32).map(|v| (v << 16) as i32).collect();
    assert_eq!(low.len(), 65_536);
    assert_same_batch(libs, &low, "row 8 (exhaustive low 16 bits)");
    assert_same_batch(libs, &high, "row 8 (exhaustive high 16 bits)");
}

/// Row 9 — byte-order-sensitive asymmetric patterns. A `to_be_bytes`
/// translation passes rows 1/2 but fails here.
fn row09_byte_order_sensitive(libs: &Libs) {    let xs: Vec<i32> = [
        0x0000_00ffu32,
        0xff00_0000,
        0x0000_ff00,
        0x00ff_0000,
        0x1234_5678,
        0x7856_3412,
        0xdead_beef,
        0xefbe_adde,
        0x0000_0001,
        0x0100_0000,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    for &x in &xs {
        assert_same(libs, x, "row 9");
    }
    assert_same_batch(libs, &xs, "row 9 (batched)");
    // Pin the native (little-endian) rendering the C code produces.
    assert_eq!(c_out(libs, 0x1234_5678), b"78563412\n");
}

/// Row 10 — mixed nibble patterns: every byte differs in both nibbles and in
/// padding class.
fn row10_mixed_nibbles(libs: &Libs) {    let xs: Vec<i32> = [
        0x0f1e_2d3cu32,
        0xa0b1_c2d3,
        0x01f0_e0d0,
        0x10ff_0080,
        0x7f80_0f10,
        0x0102_0408,
        0x8040_2010,
        0xfedc_ba98,
        0x0f0f_0f0f,
        0xf0f0_f0f0,
    ]
    .iter()
    .map(|&v| v as i32)
    .collect();
    for &x in &xs {
        assert_same(libs, x, "row 10");
    }
    assert_same_batch(libs, &xs, "row 10 (batched)");
}

/// Row 11 — randomized over the full `i32` domain, 20 000 inputs, fixed seed.
fn row11_random_full_domain(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0011);
    let xs: Vec<i32> = (0..200_000).map(|_| rng.next_i32()).collect();
    assert_same_batch(libs, &xs, "row 11 (batched)");
    // Also drive a slice one call at a time (separate capture per call).
    for &x in xs.iter().take(200) {
        assert_same(libs, x, "row 11");
    }
}

/// Row 12 — randomized with every byte in `0x00..=0x0f` (padding-only output).
fn row12_random_low_nibble_bytes(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0012);
    let pool: Vec<u8> = (0x00u8..=0x0f).collect();
    let xs: Vec<i32> = (0..2_000).map(|_| rng.i32_from_byte_pool(&pool)).collect();
    assert_same_batch(libs, &xs, "row 12 (batched)");
    for &x in xs.iter().take(100) {
        assert_same(libs, x, "row 12");
    }
}

/// Row 13 — randomized with every byte in `0x80..=0xff` (promotion-only output).
fn row13_random_high_bit_bytes(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0013);
    let pool: Vec<u8> = (0x80u8..=0xff).collect();
    let xs: Vec<i32> = (0..2_000).map(|_| rng.i32_from_byte_pool(&pool)).collect();
    assert_same_batch(libs, &xs, "row 13 (batched)");
    for &x in xs.iter().take(100) {
        assert_same(libs, x, "row 13");
    }
}

/// Row 14 — randomized from the padding/promotion boundary byte set.
fn row14_random_boundary_bytes(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0014);
    let pool = [0x00u8, 0x0f, 0x10, 0x7f, 0x80, 0xff];
    let xs: Vec<i32> = (0..2_000).map(|_| rng.i32_from_byte_pool(&pool)).collect();
    assert_same_batch(libs, &xs, "row 14 (batched)");
    for &x in xs.iter().take(100) {
        assert_same(libs, x, "row 14");
    }
}

/// Row 15 — sequenced pipeline: 5 000 randomized calls into one captured
/// stream. Verifies one `"\n"`-terminated record per call, in order, with no
/// state leaking between calls.
fn row15_long_sequence_transcript(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0015);
    let xs: Vec<i32> = (0..5_000).map(|_| rng.next_i32()).collect();

    let c = capture_stdout(|| {
        for &x in &xs {
            unsafe { (libs.c_driver)(x) }
        }
    });
    let r = capture_stdout(|| {
        for &x in &xs {
            unsafe { (libs.rust_driver)(x) }
        }
    });
    assert_eq!(c, r, "row 15: full transcripts must be byte-identical");
    assert_eq!(c.len(), xs.len() * 9, "row 15: 9 bytes per record");
    assert_eq!(
        c.iter().filter(|&&b| b == b'\n').count(),
        xs.len(),
        "row 15: exactly one newline per call"
    );
    assert!(
        c.chunks(9)
            .all(|rec| rec[8] == b'\n' && rec[..8].iter().all(|b| b.is_ascii_hexdigit())),
        "row 15: every record is 8 lowercase hex digits + newline"
    );
    assert!(
        !c.iter().any(|b| b.is_ascii_uppercase()),
        "row 15: %02x emits lowercase hex"
    );
}

/// Row 16 — interleaved C/Rust calls into the *same* stdout stream. Confirms
/// the Rust `.so` writes through libc's `stdout` (shared buffer, same ordering)
/// rather than through an independent, differently buffered stream.
fn row16_interleaved_same_stream(libs: &Libs) {    let mut rng = Rng::new(0x5EED_0016);
    let xs: Vec<i32> = (0..500).map(|_| rng.next_i32()).collect();

    let combined = capture_stdout(|| {
        for &x in &xs {
            unsafe {
                (libs.c_driver)(x);
                (libs.rust_driver)(x);
            }
        }
    });

    let lines: Vec<&[u8]> = combined
        .split(|&b| b == b'\n')
        .filter(|l| !l.is_empty())
        .collect();
    assert_eq!(lines.len(), xs.len() * 2, "row 16: two records per input");
    for (i, pair) in lines.chunks(2).enumerate() {
        assert_eq!(
            pair[0],
            pair[1],
            "row 16: C/Rust records differ for input {} (0x{:08x})",
            xs[i],
            xs[i]
        );
    }
}

/// Aggregate entry point — see `harness::run_rows` for why every row runs
/// inside a single `#[test]` (process-wide stdout redirection must be serial).
#[test]
fn all_rows() {
    run_rows(&[
        ("row01_zero", row01_zero),
        ("row02_all_ones", row02_all_ones),
        ("row03_int_max", row03_int_max),
        ("row04_int_min", row04_int_min),
        ("row05_boundary_neighbours", row05_boundary_neighbours),
        ("row06_low_nibble_each_offset", row06_low_nibble_each_offset),
        ("row07_high_bit_each_offset", row07_high_bit_each_offset),
        ("row08_exhaustive_byte_sweep", row08_exhaustive_byte_sweep),
        ("row09_byte_order_sensitive", row09_byte_order_sensitive),
        ("row10_mixed_nibbles", row10_mixed_nibbles),
        ("row11_random_full_domain", row11_random_full_domain),
        ("row12_random_low_nibble_bytes", row12_random_low_nibble_bytes),
        ("row13_random_high_bit_bytes", row13_random_high_bit_bytes),
        ("row14_random_boundary_bytes", row14_random_boundary_bytes),
        ("row15_long_sequence_transcript", row15_long_sequence_transcript),
        ("row16_interleaved_same_stream", row16_interleaved_same_stream),
    ]);
}
