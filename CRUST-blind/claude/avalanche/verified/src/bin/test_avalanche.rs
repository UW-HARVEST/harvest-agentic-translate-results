#![allow(dead_code, unused_imports)]
use avalanche::avalanche::{avalanche, Matrix};
use std::io::Cursor;

/// Hash function: identity-like, byte i goes into bits [i*8, i*8+7] of the
/// output (LSB-first within byte). Used because it has perfectly predictable
/// avalanche behavior: flipping any input bit flips exactly one output bit.
fn identity_hash(key: &[u8], hash_value: &mut [u32]) {
    let mut h: u32 = 0;
    let n = key.len().min(4);
    for i in 0..n {
        h ^= (key[i] as u32) << (i * 8);
    }
    hash_value[0] = h;
}

/// Constant hash — always returns same value irrespective of key.
fn const_hash(_key: &[u8], hash_value: &mut [u32]) {
    hash_value[0] = 0xDEAD_BEEF;
}

/// Standard FNV-1a 32-bit hash.
fn fnv_1a(key: &[u8], hash_value: &mut [u32]) {
    let mut hash: u32 = 0x811c_9dc5;
    for &b in key {
        // C: hash ^= key[i]; signed-char promotion to int sign-extends.
        // C uses `char` (signed on most platforms), then XORs into uint32_t with
        // sign extension. The Rust translation also needs to mirror this.
        let signed = b as i8 as i32 as u32;
        hash ^= signed;
        hash = hash.wrapping_mul(0x0100_0193);
    }
    hash_value[0] = hash;
}

#[test]
fn test_avalanche_identity_hash_predictable() {
    // 4-byte keys -> 32 input bits, 1 word -> 32 output bits.
    // For our identity hash, flipping bit at row r=byte_idx*8 + bit_idx
    // (MSB-first within byte) flips exactly the output bit at
    // col = 24 - byte_idx*8 + bit_idx (because the hash places byte i into
    // bits [i*8 .. i*8+7] LSB-first, but the matrix indexes hash bits MSB-first).
    let data: Vec<u8> = vec![
        0x12, 0x34, 0x56, 0x78,
        0xAB, 0xCD, 0xEF, 0x10,
        0xFF, 0x00, 0xFF, 0x00,
        0x55, 0xAA, 0x55, 0xAA,
    ];
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 100, &mut m);

    for r in 0..32usize {
        let expected_col = 24 - (r / 8) * 8 + (r % 8);
        for c in 0..32usize {
            let expected = if c == expected_col { 1.0 } else { 0.0 };
            assert_eq!(
                m.matrix_get(r, c),
                expected,
                "mismatch at r={} c={}", r, c
            );
        }
    }
}

#[test]
fn test_avalanche_const_hash_zero_matrix() {
    // A constant hash never differs, so all matrix entries should be 0.
    let data: Vec<u8> = vec![1, 2, 3, 4, 5];
    let mut m = Matrix::matrix_alloc(8, 32);
    let mut cursor = Cursor::new(data);
    avalanche(const_hash, &mut cursor, 5, &mut m);

    for r in 0..8 {
        for c in 0..32 {
            assert_eq!(m.matrix_get(r, c), 0.0, "non-zero at r={} c={}", r, c);
        }
    }
}

#[test]
fn test_avalanche_max_iter_zero() {
    // When max_iter is 0, no iterations and matrix stays at zero.
    let data: Vec<u8> = vec![0x12; 64];
    let mut m = Matrix::matrix_alloc(8, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 0, &mut m);

    for r in 0..8 {
        for c in 0..32 {
            assert_eq!(m.matrix_get(r, c), 0.0);
        }
    }
}

#[test]
fn test_avalanche_empty_input() {
    // Empty input -> 0 iterations -> matrix stays at zero (no normalization).
    let data: Vec<u8> = vec![];
    let mut m = Matrix::matrix_alloc(8, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 1000, &mut m);
    for r in 0..8 {
        for c in 0..32 {
            assert_eq!(m.matrix_get(r, c), 0.0);
        }
    }
}

#[test]
fn test_avalanche_partial_input() {
    // Input shorter than key_size -> no iterations.
    // 4-byte keys, only 3 bytes of data available.
    let data: Vec<u8> = vec![1, 2, 3];
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 100, &mut m);

    for r in 0..32 {
        for c in 0..32 {
            assert_eq!(m.matrix_get(r, c), 0.0);
        }
    }
}

#[test]
fn test_avalanche_max_iter_caps_keys() {
    // 12 bytes available, 4-byte key, so up to 3 full keys; max_iter=2 caps to 2.
    let data: Vec<u8> = vec![
        0x01, 0x02, 0x03, 0x04, // key 1
        0x10, 0x20, 0x30, 0x40, // key 2
        0xAB, 0xCD, 0xEF, 0x12, // key 3
    ];
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 2, &mut m);
    // identity hash means matrix is bitwise-1 on diagonal pattern after normalization
    // So for 2 keys, sum of any single position should be 0 or 1.
    for r in 0..32 {
        let expected_col = 24 - (r / 8) * 8 + (r % 8);
        for c in 0..32 {
            let expected = if c == expected_col { 1.0 } else { 0.0 };
            assert_eq!(m.matrix_get(r, c), expected, "r={} c={}", r, c);
        }
    }
}

#[test]
fn test_avalanche_fnv_1a_first_row() {
    // From running the C code on data/fnv_32.txt with key_len=4, max_iter=100:
    // Row 0 expected values:
    let data = std::fs::read(
        "/tmp/harvest-crust-verify-JYOzVc/project/c_src/data/fnv_32.txt"
    ).expect("test data file");
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a, &mut cursor, 100, &mut m);

    let expected_row_0: [f64; 32] = [
        0.78, 0.29, 0.39, 0.75, 0.59, 0.62, 0.75, 0.58,
        0.92, 0.10, 0.68, 0.90, 0.12, 0.30, 0.76, 0.55,
        0.92, 0.26, 0.43, 0.75, 0.39, 0.86, 0.29, 0.52,
        1.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00, 0.00,
    ];
    for c in 0..32 {
        let v = m.matrix_get(0, c);
        // values are k/100, exact representation for many; use small epsilon.
        assert!(
            (v - expected_row_0[c]).abs() < 1e-9,
            "row 0 col {} got {} expected {}",
            c, v, expected_row_0[c]
        );
    }
}

#[test]
fn test_avalanche_fnv_1a_first_row_low_iter() {
    // 10 iterations -> row 0 values are k/10
    let data = std::fs::read(
        "/tmp/harvest-crust-verify-JYOzVc/project/c_src/data/fnv_32.txt"
    ).expect("test data file");
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(fnv_1a, &mut cursor, 10, &mut m);

    // From C run with max_iter=10 (printed via %.17g):
    let expected_row_0: [f64; 32] = [
        0.7, 0.4, 0.6, 0.8, 0.4, 0.6, 0.8, 0.4,
        1.0, 0.1, 0.6, 1.0, 0.0, 0.0, 0.6, 0.6,
        1.0, 0.1, 0.3, 0.7, 0.5, 1.0, 0.1, 0.3,
        1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    for c in 0..32 {
        let v = m.matrix_get(0, c);
        assert!(
            (v - expected_row_0[c]).abs() < 1e-9,
            "row 0 col {} got {} expected {}",
            c, v, expected_row_0[c]
        );
    }
}

#[test]
fn test_avalanche_normalization_divides_by_count() {
    // Use identity hash on 1 key. Resulting matrix entries are all in {0, 1}.
    // For 1 key, normalization is /1, so values stay {0, 1}.
    let data: Vec<u8> = vec![0x12, 0x34, 0x56, 0x78];
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 1, &mut m);

    let mut count_ones = 0;
    let mut count_zero = 0;
    for r in 0..32 {
        for c in 0..32 {
            let v = m.matrix_get(r, c);
            if v == 1.0 {
                count_ones += 1;
            } else if v == 0.0 {
                count_zero += 1;
            } else {
                panic!("unexpected value at r={} c={}: {}", r, c, v);
            }
        }
    }
    assert_eq!(count_ones, 32);
    assert_eq!(count_zero, 32 * 32 - 32);
}

#[test]
fn test_avalanche_8bit_keys() {
    // 1-byte keys, 32-bit hash output -> matrix is 8x32
    // Identity hash: byte 0 placed in low 8 bits of hash
    // Flipping bit r (MSB-first) of byte 0 sets output bit (7-r) (LSB-first)
    // -> in MSB-first hash bit numbering: 31 - (7 - r) = 24 + r
    let data: Vec<u8> = vec![0x55, 0xAA, 0xFF, 0x00, 0x12];
    let mut m = Matrix::matrix_alloc(8, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 100, &mut m);

    for r in 0..8usize {
        let expected_col = 24 + r;
        for c in 0..32usize {
            let expected = if c == expected_col { 1.0 } else { 0.0 };
            assert_eq!(m.matrix_get(r, c), expected, "r={} c={}", r, c);
        }
    }
}

#[test]
fn test_avalanche_dimensions_unchanged() {
    // The avalanche function should not change the matrix dimensions.
    let data: Vec<u8> = vec![0; 64];
    let mut m = Matrix::matrix_alloc(32, 32);
    let mut cursor = Cursor::new(data);
    avalanche(identity_hash, &mut cursor, 5, &mut m);
    assert_eq!(m.n_rows, 32);
    assert_eq!(m.n_cols, 32);
    assert_eq!(m.vals.len(), 32 * 32);
}

fn main() {}
