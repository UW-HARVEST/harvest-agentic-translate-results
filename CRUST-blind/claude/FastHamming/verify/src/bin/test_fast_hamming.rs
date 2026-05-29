use FastHamming::fast_hamming::{hecc_decode, hecc_encode, log2uint8};

// ===== log2uint8 tests =====
//
// Ground-truth values produced by running the C log2uint8 test program in
// c_src/test/log2uint8.c. The C function is `31 - __builtin_clz(v)` which
// equals floor(log2(v)) for v >= 1.

#[test]
fn test_log2uint8_basic_values() {
    assert_eq!(log2uint8(1), 0);
    assert_eq!(log2uint8(2), 1);
    assert_eq!(log2uint8(3), 1);
    assert_eq!(log2uint8(4), 2);
    assert_eq!(log2uint8(7), 2);
    assert_eq!(log2uint8(8), 3);
    assert_eq!(log2uint8(16), 4);
    assert_eq!(log2uint8(32), 5);
    assert_eq!(log2uint8(64), 6);
    assert_eq!(log2uint8(128), 7);
    assert_eq!(log2uint8(255), 7);
}

#[test]
fn test_log2uint8_all_nonzero() {
    // For v in 1..=255, floor(log2(v)) is the index of the highest set bit.
    // Compute the expected value the same way the C builtin does conceptually.
    for v in 1u16..=255u16 {
        let mut expected: u8 = 0;
        let mut tmp = v;
        while tmp > 1 {
            tmp >>= 1;
            expected += 1;
        }
        assert_eq!(log2uint8(v as u8), expected, "log2uint8({v}) wrong");
    }
}

#[test]
fn test_log2uint8_powers_of_two() {
    // Each power of 2 from 1 (=2^0) up to 128 (=2^7).
    let pairs: [(u8, u8); 8] = [
        (1, 0),
        (2, 1),
        (4, 2),
        (8, 3),
        (16, 4),
        (32, 5),
        (64, 6),
        (128, 7),
    ];
    for (v, exp) in pairs {
        assert_eq!(log2uint8(v), exp);
    }
}

#[test]
fn test_log2uint8_one_below_powers_of_two() {
    // floor(log2(2^k - 1)) = k - 1, for k >= 1.
    let pairs: [(u8, u8); 7] = [
        (3, 1),    // 2^2 - 1
        (7, 2),    // 2^3 - 1
        (15, 3),   // 2^4 - 1
        (31, 4),   // 2^5 - 1
        (63, 5),   // 2^6 - 1
        (127, 6),  // 2^7 - 1
        (255, 7),  // 2^8 - 1
    ];
    for (v, exp) in pairs {
        assert_eq!(log2uint8(v), exp);
    }
}

// ===== hecc_encode tests =====
//
// Ground-truth values produced by running the original C hecc_encode against
// the same inputs (see oracle program built from c_src).

#[test]
fn test_encode_full_block_data8() {
    // From c_src/test/coding.c - the canonical 15-byte test vector.
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let expected: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
        0xd9,
    ];
    let out = hecc_encode(&data8);
    assert_eq!(out.len(), 16);
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn test_encode_empty_input() {
    let out = hecc_encode(&[]);
    assert_eq!(out.len(), 0);
    assert!(out.is_empty());
}

#[test]
fn test_encode_one_byte_zero() {
    // C oracle: encode([0x00]) -> [0x00, 0x00], outsize=2
    let out = hecc_encode(&[0x00]);
    assert_eq!(out, vec![0x00, 0x00]);
}

#[test]
fn test_encode_one_byte_ff() {
    // C oracle: encode([0xff]) -> [0xff, 0x03], outsize=2
    let out = hecc_encode(&[0xff]);
    assert_eq!(out, vec![0xff, 0x03]);
}

#[test]
fn test_encode_one_byte_55() {
    // C oracle: encode([0x55]) -> [0x55, 0x87], outsize=2
    let out = hecc_encode(&[0x55]);
    assert_eq!(out, vec![0x55, 0x87]);
}

#[test]
fn test_encode_one_byte_aa() {
    // C oracle: encode([0xaa]) -> [0xaa, 0x84], outsize=2
    let out = hecc_encode(&[0xaa]);
    assert_eq!(out, vec![0xaa, 0x84]);
}

#[test]
fn test_encode_two_bytes() {
    // C oracle: encode([0xde, 0xad]) -> [0xde, 0xad, 0x1f]
    let out = hecc_encode(&[0xde, 0xad]);
    assert_eq!(out, vec![0xde, 0xad, 0x1f]);
}

#[test]
fn test_encode_seven_bytes() {
    // C oracle: encode(0..6) -> [0,1,2,3,4,5,6, 0xad]
    let input: [u8; 7] = [0, 1, 2, 3, 4, 5, 6];
    let out = hecc_encode(&input);
    assert_eq!(out, vec![0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0xad]);
}

#[test]
fn test_encode_fourteen_bytes() {
    // C oracle: encode(0..13) -> [0..13, 0x49], outsize=15
    let input: [u8; 14] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13];
    let out = hecc_encode(&input);
    let expected: Vec<u8> = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x49,
    ];
    assert_eq!(out, expected);
}

#[test]
fn test_encode_fifteen_bytes_sequential() {
    // C oracle: encode(0..14) -> [0..14, 0xb1], outsize=16
    let input: [u8; 15] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14];
    let out = hecc_encode(&input);
    let expected: Vec<u8> = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0xb1,
    ];
    assert_eq!(out, expected);
}

#[test]
fn test_encode_sixteen_bytes_one_block_plus_partial() {
    // C oracle: encode(0..15) -> 18 bytes total
    //   block1: 00..0e b1
    //   block2 (partial 1 byte): 0f 87
    let input: [u8; 16] = [0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15];
    let out = hecc_encode(&input);
    let expected: Vec<u8> = vec![
        0x00, 0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08, 0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e,
        0xb1, 0x0f, 0x87,
    ];
    assert_eq!(out.len(), 18);
    assert_eq!(out, expected);
}

#[test]
fn test_encode_thirty_bytes_two_full_blocks() {
    // 30 bytes = exactly 2 full blocks; outsize = 32.
    // Computed by C oracle: input[i] = i*3 + 7
    let mut input = [0u8; 30];
    for i in 0..30 {
        input[i] = (i as u8).wrapping_mul(3).wrapping_add(7);
    }
    let out = hecc_encode(&input);
    let expected: Vec<u8> = vec![
        0x07, 0x0a, 0x0d, 0x10, 0x13, 0x16, 0x19, 0x1c, 0x1f, 0x22, 0x25, 0x28, 0x2b, 0x2e, 0x31,
        0x24, 0x34, 0x37, 0x3a, 0x3d, 0x40, 0x43, 0x46, 0x49, 0x4c, 0x4f, 0x52, 0x55, 0x58, 0x5b,
        0x5e, 0x3d,
    ];
    assert_eq!(out.len(), 32);
    assert_eq!(out, expected);
}

#[test]
fn test_encode_thirty_two_bytes_two_full_plus_partial() {
    // 32 bytes => 2 full blocks (32 -> 32 bytes) + 2-byte partial (-> 3 bytes) = 35 bytes
    // Computed by C oracle: input[i] = i*5 + 1
    let mut input = [0u8; 32];
    for i in 0..32 {
        input[i] = (i as u8).wrapping_mul(5).wrapping_add(1);
    }
    let out = hecc_encode(&input);
    let expected: Vec<u8> = vec![
        0x01, 0x06, 0x0b, 0x10, 0x15, 0x1a, 0x1f, 0x24, 0x29, 0x2e, 0x33, 0x38, 0x3d, 0x42, 0x47,
        0x5e, 0x4c, 0x51, 0x56, 0x5b, 0x60, 0x65, 0x6a, 0x6f, 0x74, 0x79, 0x7e, 0x83, 0x88, 0x8d,
        0x92, 0x84, 0x97, 0x9c, 0x1c,
    ];
    assert_eq!(out.len(), 35);
    assert_eq!(out, expected);
}

#[test]
fn test_encode_output_length_invariants() {
    // For an n-byte input, output length = n + ceil(n / 15).
    // Verified by running the C encoder for each n.
    for n in 0..=64usize {
        let input: Vec<u8> = (0..n).map(|i| i as u8).collect();
        let out = hecc_encode(&input);
        let expected_len = if n == 0 { 0 } else { n + (n + 14) / 15 };
        assert_eq!(
            out.len(),
            expected_len,
            "length mismatch at n={n}: got {} expected {}",
            out.len(),
            expected_len
        );
    }
}

// ===== hecc_decode tests =====

#[test]
fn test_decode_full_block_no_error() {
    // C oracle: decode of correct full encoding
    let enc: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
        0xd9,
    ];
    let expected: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let out = hecc_decode(&enc).expect("decode should succeed");
    assert_eq!(out.len(), 15);
    assert_eq!(&out[..], &expected[..]);
}

#[test]
fn test_decode_empty_input_returns_some_empty() {
    // C oracle: insize=0 -> ret=true, outsize=0
    let out = hecc_decode(&[]);
    assert!(out.is_some());
    let v = out.unwrap();
    assert_eq!(v.len(), 0);
    assert!(v.is_empty());
}

#[test]
fn test_decode_truncated_one_byte_returns_none() {
    // C oracle: insize=1 (insize % 16 == 1) -> ret=false
    let out = hecc_decode(&[0u8]);
    assert!(out.is_none());
}

#[test]
fn test_decode_truncated_seventeen_bytes_returns_none() {
    // C oracle: insize=17 (insize % 16 == 1) -> ret=false
    let buf = [0u8; 17];
    let out = hecc_decode(&buf);
    assert!(out.is_none());
}

#[test]
fn test_decode_partial_block_one_data_byte() {
    // C oracle:
    //   encode([0xff]) -> [0xff, 0x03]
    //   decode([0xff, 0x03]) -> [0xff]
    let enc = [0xff, 0x03];
    let out = hecc_decode(&enc).expect("decode should succeed");
    assert_eq!(out, vec![0xff]);
}

#[test]
fn test_decode_all_zeros_full_block() {
    // C oracle: decode 16 zero bytes -> 15 zero bytes
    let enc = [0u8; 16];
    let out = hecc_decode(&enc).expect("decode should succeed");
    assert_eq!(out.len(), 15);
    assert_eq!(out, vec![0u8; 15]);
}

#[test]
fn test_decode_corrects_every_data_bit_error() {
    // For every bit position 0..15*8 in the data area, flipping that bit should
    // be corrected and the decoded output should match the original.
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let enc: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
        0xd9,
    ];
    for idx in 0..(15 * 8) {
        let mut buf = enc;
        buf[idx >> 3] ^= 1u8 << (idx % 8);
        let out = hecc_decode(&buf).expect("single-bit error should be correctable");
        assert_eq!(out.len(), 15, "bit {idx} length wrong");
        assert_eq!(&out[..], &data8[..], "bit {idx} did not correct cleanly");
    }
}

#[test]
fn test_decode_corrects_each_parity_bit_flip() {
    // C test_error_single: flipping bits 0..7 of the parity byte should still
    // decode correctly to the original data.
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let enc: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
        0xd9,
    ];
    for idx in 0..8u8 {
        let mut buf = enc;
        buf[15] ^= 1u8 << idx;
        let out = hecc_decode(&buf).expect("parity-only error should be correctable");
        assert_eq!(out.len(), 15);
        assert_eq!(&out[..], &data8[..], "parity bit {idx} flipped: wrong output");
    }
}

#[test]
fn test_decode_double_error_returns_none() {
    // C test_error_double: flipping two adjacent data bits is uncorrectable.
    let mut buf: [u8; 16] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
        0xd9,
    ];
    buf[0] ^= 3;
    let out = hecc_decode(&buf);
    assert!(out.is_none(), "double bit error must not be correctable");
}

#[test]
fn test_roundtrip_data8() {
    let data8: [u8; 15] = [
        0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    ];
    let enc = hecc_encode(&data8);
    assert_eq!(enc.len(), 16);
    let dec = hecc_decode(&enc).expect("roundtrip decode must succeed");
    assert_eq!(dec.len(), 15);
    assert_eq!(&dec[..], &data8[..]);
}

#[test]
fn test_roundtrip_various_sizes() {
    for n in 0..=64usize {
        let input: Vec<u8> = (0..n).map(|i| (i.wrapping_mul(7) ^ 0x5a) as u8).collect();
        let enc = hecc_encode(&input);
        let dec = hecc_decode(&enc).expect("roundtrip decode must succeed");
        assert_eq!(dec, input, "roundtrip mismatch at n={n}");
    }
}

#[test]
fn test_roundtrip_32_bytes_known_values() {
    // C oracle: encode/decode of 32 bytes (5*i + 1 mod 256)
    let mut input = [0u8; 32];
    for i in 0..32 {
        input[i] = (i as u8).wrapping_mul(5).wrapping_add(1);
    }
    let enc = hecc_encode(&input);
    let expected_enc: Vec<u8> = vec![
        0x01, 0x06, 0x0b, 0x10, 0x15, 0x1a, 0x1f, 0x24, 0x29, 0x2e, 0x33, 0x38, 0x3d, 0x42, 0x47,
        0x5e, 0x4c, 0x51, 0x56, 0x5b, 0x60, 0x65, 0x6a, 0x6f, 0x74, 0x79, 0x7e, 0x83, 0x88, 0x8d,
        0x92, 0x84, 0x97, 0x9c, 0x1c,
    ];
    assert_eq!(enc, expected_enc);

    let dec = hecc_decode(&enc).expect("decode should succeed");
    assert_eq!(dec.len(), 32);
    assert_eq!(&dec[..], &input[..]);
}

#[test]
fn test_decode_corrects_single_bit_error_in_partial_block() {
    // Build a partial-block encoding (1 data byte) and flip every data bit.
    // The decoder must correct the error and reproduce the original data.
    let original: [u8; 1] = [0x55];
    let enc = hecc_encode(&original);
    assert_eq!(enc.len(), 2);

    for bit in 0..8u8 {
        let mut buf = enc.clone();
        buf[0] ^= 1u8 << bit;
        let out = hecc_decode(&buf).expect("partial-block single bit must correct");
        assert_eq!(out.len(), 1);
        assert_eq!(out[0], original[0], "bit {bit} flipped: wrong output");
    }
}

#[test]
fn test_decode_returns_none_when_any_block_is_uncorrectable() {
    // Construct a 32-byte input -> 35-byte encoding (2 full + 1 partial block).
    // Corrupt 2 bits in the first block; whole decode must fail.
    let mut input = [0u8; 32];
    for i in 0..32 {
        input[i] = (i as u8).wrapping_mul(5).wrapping_add(1);
    }
    let mut enc = hecc_encode(&input);
    enc[0] ^= 3; // double bit error in first block
    assert!(hecc_decode(&enc).is_none());
}

fn main() {}
