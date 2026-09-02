// Phase B — valid-path differential tests.
// One test per row of CONFIGS.md. Both libraries are exercised only through
// their `.so` exports.

#[path = "common/mod.rs"]
mod common;

use common::{assert_same, assert_same_interleaved, c_outputs, rust_outputs, Rng};

/// CONFIGS.md row 1
#[test]
fn cfg_01_zero() {
    assert_same("row 1: floors = 0", &[0]);
}

/// CONFIGS.md row 2
#[test]
fn cfg_02_one() {
    assert_same("row 2: floors = 1", &[1]);
}

/// CONFIGS.md row 3
#[test]
fn cfg_03_equal_to_bedrooms() {
    assert_same("row 3: floors = 3", &[3]);
}

/// CONFIGS.md row 4
#[test]
fn cfg_04_all_bits_set() {
    assert_same("row 4: floors = -1", &[-1]);
}

/// CONFIGS.md row 5
#[test]
fn cfg_05_int_max() {
    assert_same("row 5: floors = INT_MAX", &[i32::MAX]);
}

/// CONFIGS.md row 6
#[test]
fn cfg_06_int_min() {
    assert_same("row 6: floors = INT_MIN", &[i32::MIN]);
}

/// CONFIGS.md row 7 — byte-lane placement, isolates byte order / offsets.
#[test]
fn cfg_07_one_hot_bytes() {
    let inputs: Vec<i32> = [0x0000_00ffu32, 0x0000_ff00, 0x00ff_0000, 0xff00_0000]
        .iter()
        .map(|&v| v as i32)
        .collect();
    assert_same("row 7: one-hot bytes", &inputs);
}

/// CONFIGS.md row 8 — every single bit position.
#[test]
fn cfg_08_one_hot_bits() {
    let inputs: Vec<i32> = (0..32).map(|k| (1u32 << k) as i32).collect();
    assert_same("row 8: one-hot bits", &inputs);
}

/// CONFIGS.md row 9 — the `%02x` zero-padding branch over the whole low byte.
#[test]
fn cfg_09_low_byte_sweep() {
    let inputs: Vec<i32> = (0..=0xffi32).collect();
    assert_same("row 9: low-byte sweep 0x00..=0xff", &inputs);
}

/// CONFIGS.md row 10 — carry across each byte lane.
#[test]
fn cfg_10_byte_lane_boundaries() {
    let inputs: Vec<i32> = [
        0x7f, 0x80, 0x81, 0x7fff, 0x8000, 0x8001, 0x7f_ffff, 0x80_0000, 0x80_0001,
    ]
    .to_vec();
    assert_same("row 10: byte-lane boundaries", &inputs);
}

/// CONFIGS.md row 11 — randomized full 32-bit domain, fixed seed.
#[test]
fn cfg_11_random_full_domain() {
    let mut rng = Rng::new(0x5EED_0000_0000_0011);
    let inputs: Vec<i32> = (0..20_000).map(|_| rng.next_i32()).collect();
    assert_same("row 11: 20000 random i32 (seed 0x5EED...0011)", &inputs);
}

/// CONFIGS.md row 12 — randomized small magnitudes, fixed seed.
#[test]
fn cfg_12_random_small_magnitude() {
    let mut rng = Rng::new(0x5EED_0000_0000_0012);
    let inputs: Vec<i32> = (0..4096).map(|_| rng.range_i32(-1024, 1024)).collect();
    assert_same("row 12: 4096 random in -1024..=1024", &inputs);
}

/// CONFIGS.md row 13 — full cross-product of interesting byte values per lane.
#[test]
fn cfg_13_bytewise_cross_product() {
    const LANE: [u32; 8] = [0x00, 0x01, 0x0f, 0x10, 0x7f, 0x80, 0xfe, 0xff];
    let mut inputs = Vec::with_capacity(8 * 8 * 8 * 8);
    for &b3 in &LANE {
        for &b2 in &LANE {
            for &b1 in &LANE {
                for &b0 in &LANE {
                    inputs.push(((b3 << 24) | (b2 << 16) | (b1 << 8) | b0) as i32);
                }
            }
        }
    }
    assert_eq!(inputs.len(), 4096);
    assert_same("row 13: bytewise cross-product", &inputs);
}

/// CONFIGS.md row 14 — repeated / re-visited values, no residual state.
#[test]
fn cfg_14_invocation_sequence() {
    let inputs = [7, 7, -12345, 7, 0, 0, i32::MIN, 7];
    assert_same("row 14: invocation sequence", &inputs);

    // The same value must produce the same record no matter its position.
    let c = c_outputs(&inputs);
    let r = rust_outputs(&inputs);
    for recs in [&c, &r] {
        for i in [1usize, 3, 7] {
            assert_eq!(
                recs[0], recs[i],
                "row 14: driver(7) differed between call 0 and call {i}"
            );
        }
        assert_eq!(recs[4], recs[5], "row 14: driver(0) differed across repeats");
    }
}

/// CONFIGS.md row 15 — C and Rust alternating in one shared stdout stream.
#[test]
fn cfg_15_interleaved_libraries() {
    let mut rng = Rng::new(0x5EED_0000_0000_0015);
    let mut inputs: Vec<i32> = vec![0, 1, -1, i32::MAX, i32::MIN, 3];
    inputs.extend((0..2_000).map(|_| rng.next_i32()));
    assert_same_interleaved("row 15: interleaved C/Rust", &inputs);
}

/// CONFIGS.md row 16 — structural invariants of the record, checked on both.
#[test]
fn cfg_16_structural_invariants() {
    let mut rng = Rng::new(0x5EED_0000_0000_0016);
    let mut inputs: Vec<i32> = vec![0, 1, 3, -1, i32::MAX, i32::MIN];
    inputs.extend((0..2_000).map(|_| rng.next_i32()));

    let c = c_outputs(&inputs);
    let r = rust_outputs(&inputs);

    for (which, recs) in [("C", &c), ("Rust", &r)] {
        for (i, rec) in recs.iter().enumerate() {
            let x = inputs[i];
            // 16 struct bytes * 2 hex chars + '\n'
            assert_eq!(
                rec.len(),
                33,
                "{which}: driver({x}) record length {} != 33",
                rec.len()
            );
            assert_eq!(rec[32], b'\n', "{which}: driver({x}) is not newline-terminated");
            assert!(
                rec[..32]
                    .iter()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(b)),
                "{which}: driver({x}) contains non-lowercase-hex: {:?}",
                String::from_utf8_lossy(rec)
            );
            // `floors` occupies offset 0..4, little-endian.
            let want_floors: String = (x as u32)
                .to_le_bytes()
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect();
            assert_eq!(
                &rec[0..8],
                want_floors.as_bytes(),
                "{which}: driver({x}) floors image wrong"
            );
            // `bedrooms == 3` at offset 4..8 and `bathrooms == 2.0` at 8..16.
            assert_eq!(
                &rec[8..16], b"03000000",
                "{which}: driver({x}) bedrooms image wrong"
            );
            assert_eq!(
                &rec[16..32],
                b"0000000000000040",
                "{which}: driver({x}) bathrooms image wrong"
            );
        }
    }
    assert_same("row 16: structural invariants (differential)", &inputs);
}
