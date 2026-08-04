#![allow(dead_code, unused_imports)]
use avalanche::avalanche::{avalanche, Matrix};
use std::io::Cursor;

fn simple_hash(key: &[u8], out: &mut [u32]) {
    let mut h: u32 = 0;
    for &b in key.iter() {
        h ^= b as u32;
        h = (h << 1) | (h >> 31);
    }
    out[0] = h;
}

fn identity_hash(key: &[u8], out: &mut [u32]) {
    // pack first up-to-4 bytes into a uint32 (big-endian)
    let mut h: u32 = 0;
    for (i, &b) in key.iter().enumerate().take(4) {
        h |= (b as u32) << (24 - 8 * i);
    }
    out[0] = h;
}

fn fnv_1a(key: &[u8], out: &mut [u32]) {
    let mut hash: u32 = 0x811c9dc5;
    for &b in key.iter() {
        // C: char -> int promotion is sign-extended on signed char platforms.
        // The C code does `hash ^= key[i]` where key[i] is char.
        // On x86_64 gcc, char is signed. For the data files, bytes are likely
        // ASCII so this matches u32.
        hash ^= b as i8 as i32 as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    out[0] = hash;
}

#[test]
fn test_matrix_alloc_fields() {
    let m = Matrix::matrix_alloc(3, 4);
    assert_eq!(m.n_rows, 3);
    assert_eq!(m.n_cols, 4);
    assert_eq!(m.vals.len(), 12);
    // initialized to zero
    for v in m.vals.iter() {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_matrix_alloc_empty() {
    let m = Matrix::matrix_alloc(0, 0);
    assert_eq!(m.n_rows, 0);
    assert_eq!(m.n_cols, 0);
    assert_eq!(m.vals.len(), 0);
}

#[test]
fn test_matrix_get_set_basic() {
    let mut m = Matrix::matrix_alloc(2, 2);
    // From C reference: 1/(r+c+1) Hilbert
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 1, 0.5);
    m.matrix_set(1, 0, 0.5);
    m.matrix_set(1, 1, 1.0 / 3.0);
    assert_eq!(m.matrix_get(0, 0), 1.0);
    assert_eq!(m.matrix_get(0, 1), 0.5);
    assert_eq!(m.matrix_get(1, 0), 0.5);
    assert_eq!(m.matrix_get(1, 1), 1.0 / 3.0);
}

#[test]
fn test_matrix_get_row_major() {
    // Verify that vals layout is row-major: M[r][c] is at vals[r*n_cols + c]
    let mut m = Matrix::matrix_alloc(3, 4);
    for r in 0..3 {
        for c in 0..4 {
            m.matrix_set(r, c, (r * 4 + c) as f64);
        }
    }
    for r in 0..3 {
        for c in 0..4 {
            assert_eq!(m.matrix_get(r, c), (r * 4 + c) as f64);
            assert_eq!(m.vals[r * 4 + c], (r * 4 + c) as f64);
        }
    }
}

#[test]
fn test_matrix_fprintf_hilbert() {
    // From C reference matrix_test:
    //   1.0000  0.5000  0.3333
    //   0.5000  0.3333  0.2500
    //   0.3333  0.2500  0.2000
    let mut m = Matrix::matrix_alloc(3, 3);
    for r in 0..3 {
        for c in 0..3 {
            m.matrix_set(r, c, 1.0 / (r + c + 1) as f64);
        }
    }
    let mut buf = Vec::new();
    m.matrix_fprintf(&mut buf, "%8.4f");
    let out = String::from_utf8(buf).unwrap();
    assert_eq!(
        out,
        "  1.0000  0.5000  0.3333\n  0.5000  0.3333  0.2500\n  0.3333  0.2500  0.2000\n"
    );
}

#[test]
fn test_matrix_fprintf_basic_f() {
    // Reference C output for set [[0..3], [4..7], [8..11]] with "%6.2f":
    //   0.00  1.00  2.00  3.00
    //   4.00  5.00  6.00  7.00
    //   8.00  9.00 10.00 11.00
    let mut m = Matrix::matrix_alloc(3, 4);
    for r in 0..3 {
        for c in 0..4 {
            m.matrix_set(r, c, (r * 4 + c) as f64);
        }
    }
    let mut buf = Vec::new();
    m.matrix_fprintf(&mut buf, "%6.2f");
    let out = String::from_utf8(buf).unwrap();
    assert_eq!(
        out,
        "  0.00  1.00  2.00  3.00\n  4.00  5.00  6.00  7.00\n  8.00  9.00 10.00 11.00\n"
    );
}

#[test]
fn test_matrix_fprintf_g_specifier() {
    // Reference C output for [[1, 0.5, 0.333333], [0.25, 0.2, 0.166667]] with "% 6.4g":
    //      1   0.5 0.3333
    //   0.25   0.2 0.1667
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 1, 0.5);
    m.matrix_set(0, 2, 0.333333);
    m.matrix_set(1, 0, 0.25);
    m.matrix_set(1, 1, 0.2);
    m.matrix_set(1, 2, 0.166667);
    let mut buf = Vec::new();
    m.matrix_fprintf(&mut buf, "% 6.4g");
    let out = String::from_utf8(buf).unwrap();
    assert_eq!(out, "     1   0.5 0.3333\n  0.25   0.2 0.1667\n");
}

#[test]
fn test_matrix_fprintf_no_padding() {
    // C printf "%.3f" on the same data produces e.g. "1.000" "0.500" "0.333" concatenated
    // since there's no width.
    let mut m = Matrix::matrix_alloc(2, 3);
    m.matrix_set(0, 0, 1.0);
    m.matrix_set(0, 1, 0.5);
    m.matrix_set(0, 2, 0.333333);
    m.matrix_set(1, 0, 0.25);
    m.matrix_set(1, 1, 0.2);
    m.matrix_set(1, 2, 0.166667);
    let mut buf = Vec::new();
    m.matrix_fprintf(&mut buf, "%.3f");
    let out = String::from_utf8(buf).unwrap();
    // Reference C: "1.0000.5000.333\n0.2500.2000.167\n"
    assert_eq!(out, "1.0000.5000.333\n0.2500.2000.167\n");
}

#[test]
fn test_avalanche_empty_input() {
    // No input bytes: avalanche should not iterate, all entries remain 0.
    let data: Vec<u8> = Vec::new();
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(8, 32);
    avalanche(simple_hash, &mut cur, 1000, &mut r);
    for v in r.vals.iter() {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_avalanche_partial_input() {
    // Input is shorter than one key: should not iterate.
    // key_size = n_rows / 8 = 32 / 8 = 4. We provide 3 bytes.
    let data: Vec<u8> = vec![1, 2, 3];
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(32, 32);
    avalanche(identity_hash, &mut cur, 1000, &mut r);
    let sum: f64 = r.vals.iter().sum();
    assert_eq!(sum, 0.0);
    // every entry must be 0
    for v in r.vals.iter() {
        assert_eq!(*v, 0.0);
    }
}

#[test]
fn test_avalanche_identity_hash() {
    // identity_hash packs first 4 bytes (big-endian) into output[0].
    // Flipping any input bit i (0..32) flips exactly bit i of the output,
    // so the avalanche matrix must be the identity matrix (1 on diagonal, 0 elsewhere).
    let data: Vec<u8> = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(32, 32);
    avalanche(identity_hash, &mut cur, 1000, &mut r);
    for row in 0..32 {
        for col in 0..32 {
            let expected = if row == col { 1.0 } else { 0.0 };
            assert_eq!(
                r.matrix_get(row, col),
                expected,
                "mismatch at ({},{})",
                row,
                col
            );
        }
    }
}

#[test]
fn test_avalanche_max_iter_limit() {
    // 16 input bytes, key_size = 1, max_iter = 2: reads only 2 bytes (0, 1).
    // simple_hash computes h via xor + rotate. With 1 byte, h = rotl(0 ^ b, 1) = rotl(b, 1).
    // For b = 0: h = 0
    // For b = 1: h = 0x00000002 (b after xor=0x01, rotl1 = 0x02)
    // We flip i-th bit (i in 0..8). For input byte b, flipped value f = b ^ (0x80 >> i).
    // Since simple_hash = rotl(byte, 1), the avalanche bit flipped in output i_bit_after_rot
    // is also a single bit. Result: when input bit i (counting from MSB) flips, output bit
    // (i + 1) mod 32 flips, since rotl shifts left by 1.
    //
    // Wait: input is treated as a byte (8 bits). The rotation is on a 32-bit value.
    // For input byte b (range 0..255), h = rotl((b as u32) ^ b, 1)? No: it's just
    // h = 0; h ^= b; h = rotl(h, 1). So h = rotl(b, 1) where b is in lower 8 bits.
    // After rotl: bit 0 (input MSB of byte) is at hash bit 7+1=... let's reason in terms
    // of actual bit positions of u32:
    // Before rotl, h = b (bits 0-7 contain b's value, with b's bit_0 = h's bit_0).
    // After rotl(h, 1), h's bit_(k+1) = old h's bit_k (mod 32).
    //
    // Now the C code's row index uses MSB-first ordering:
    //   i_byte = 0; i_bit in [0,7]; row = i_byte*8 + i_bit
    //   i_mask = 0x80 >> i_bit  -> i_bit=0 means 0x80 (the high bit of the byte)
    // So row 0 corresponds to flipping byte's bit 7 (MSB), which is u32 bit 7 since b is in bits 0..7.
    // After rotl by 1, that becomes u32 bit 8.
    //
    // Output column ordering also MSB-first:
    //   j_word*32 + j_bit, j_mask = 0x80000000 >> j_bit
    //   j_bit=0 -> mask 0x80000000 -> u32 bit 31 (MSB)
    //   j_bit=k -> u32 bit (31 - k)
    // So col k corresponds to u32 bit (31 - k).
    //
    // After rotl by 1: u32 bit 8 is the bit (col where 31 - col == 8 -> col = 23).
    // So flipping input row 0 flips output col 23.
    //
    // Similarly:
    //   row 1 -> input byte bit 6 -> u32 bit 6 -> after rotl -> bit 7 -> col 24
    //   row 2 -> bit 5 -> 6 -> col 25
    //   row 3 -> bit 4 -> 5 -> col 26
    //   row 4 -> bit 3 -> 4 -> col 27
    //   row 5 -> bit 2 -> 3 -> col 28
    //   row 6 -> bit 1 -> 2 -> col 29
    //   row 7 -> bit 0 -> 1 -> col 30
    //
    // The reference C output confirms: row 0 has 1.0 at col 23, row 1 at col 24, etc.

    let data: Vec<u8> = (0u8..16).collect();
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(8, 32);
    avalanche(simple_hash, &mut cur, 2, &mut r);

    // Verify mapping from C reference output (max_iter=2 with bytes 0,1 produces
    // exactly the same matrix as max_iter=1000 because both have single-flip
    // behavior independent of input value):
    let expected_cols = [23, 24, 25, 26, 27, 28, 29, 30];
    for row in 0..8usize {
        for col in 0..32usize {
            let expected = if col == expected_cols[row] { 1.0 } else { 0.0 };
            assert_eq!(
                r.matrix_get(row, col),
                expected,
                "mismatch at ({},{})",
                row,
                col
            );
        }
    }
}

#[test]
fn test_avalanche_normalization() {
    // After processing 2 keys, the matrix entries should be averages.
    // simple_hash, key_size=1: each flip produces exactly one output bit difference
    // (always at the same column), so matrix entry is exactly 1.0 (count=2, divided
    // by 2 gives 1.0).
    let data: Vec<u8> = vec![0xAA, 0x55];
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(8, 32);
    avalanche(simple_hash, &mut cur, 1000, &mut r);
    // Row 0 col 23 should be 1.0 (both keys flip the same output bit when MSB flipped)
    assert_eq!(r.matrix_get(0, 23), 1.0);
    // sum across each row should be 1.0 (one flip per row)
    for row in 0..8 {
        let mut row_sum = 0.0;
        for col in 0..32 {
            row_sum += r.matrix_get(row, col);
        }
        assert_eq!(row_sum, 1.0, "row {} sum incorrect", row);
    }
}

#[test]
fn test_avalanche_single_key_count() {
    // With only one key, denominator is 1, so entries are exact counts.
    let data: Vec<u8> = vec![0x42];
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(8, 32);
    avalanche(simple_hash, &mut cur, 1000, &mut r);
    // row 0, col 23 should be 1.0
    assert_eq!(r.matrix_get(0, 23), 1.0);
    // total sum equals 8 (one flip per row)
    let total: f64 = r.vals.iter().sum();
    assert_eq!(total, 8.0);
}

#[test]
fn test_avalanche_fnv32_first_row() {
    // From C reference (fnv_32 ./data/fnv_32.txt 16 10), the first row is:
    //  0.5   0.9   0.2   0.4   0.9   0.1   0.2   0.7   0.6   0.9   0.1   0.2   0.3   0.6   0.7  1.0   0.1   0.3   0.9   0.2   0.7   0.9   0.2   0.3  1.0   0     0     0     0     0     0     0
    //
    // We need to read the actual data file to feed input.
    let data = std::fs::read("c_src/data/fnv_32.txt").expect("data file");
    let mut cur = Cursor::new(data);
    let mut r = Matrix::matrix_alloc(16 * 8, 32);  // key_len=16, hash_words=1
    avalanche(fnv_1a, &mut cur, 10, &mut r);

    // Reference values (from running the C binary with the same args)
    // Row 0: rounded to "% 6.4g" precision = at most 4 significant figures
    let row0_expected: Vec<f64> = vec![
        0.5, 0.9, 0.2, 0.4, 0.9, 0.1, 0.2, 0.7, 0.6, 0.9, 0.1, 0.2, 0.3, 0.6, 0.7, 1.0,
        0.1, 0.3, 0.9, 0.2, 0.7, 0.9, 0.2, 0.3, 1.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0,
    ];
    for (col, &expected) in row0_expected.iter().enumerate() {
        let got = r.matrix_get(0, col);
        // Allow small float tolerance because comparison is with 1 decimal print
        assert!(
            (got - expected).abs() < 1e-9,
            "col {}: got {} expected {}",
            col,
            got,
            expected,
        );
    }
}

fn main() {}
