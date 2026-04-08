use FastHamming::fast_hamming::{hecc_decode, hecc_encode, log2uint8};

const DATA8: [u8; 15] = [
    0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
];
const DATA8_ENC: [u8; 16] = [
    0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87, 0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
    0xd9,
];

// === log2uint8 ===

#[test]
fn test_log2uint8_powers_of_two() {
    assert_eq!(log2uint8(1), 0);
    assert_eq!(log2uint8(2), 1);
    assert_eq!(log2uint8(4), 2);
    assert_eq!(log2uint8(8), 3);
    assert_eq!(log2uint8(16), 4);
    assert_eq!(log2uint8(32), 5);
    assert_eq!(log2uint8(64), 6);
    assert_eq!(log2uint8(128), 7);
}

#[test]
fn test_log2uint8_non_powers() {
    assert_eq!(log2uint8(3), 1);
    assert_eq!(log2uint8(5), 2);
    assert_eq!(log2uint8(7), 2);
    assert_eq!(log2uint8(15), 3);
    assert_eq!(log2uint8(31), 4);
    assert_eq!(log2uint8(63), 5);
    assert_eq!(log2uint8(127), 6);
    assert_eq!(log2uint8(255), 7);
}

// === hecc_encode ===

#[test]
fn test_encode_full_block() {
    let enc = hecc_encode(&DATA8);
    assert_eq!(enc, DATA8_ENC.to_vec());
}

#[test]
fn test_encode_empty() {
    let enc = hecc_encode(&[]);
    assert!(enc.is_empty());
}

#[test]
fn test_encode_one_byte() {
    let enc = hecc_encode(&[0xab]);
    assert_eq!(enc, vec![0xab, 0x07]);
}

#[test]
fn test_encode_two_bytes() {
    let enc = hecc_encode(&[0x00, 0xff]);
    assert_eq!(enc, vec![0x00, 0xff, 0x1d]);
}

#[test]
fn test_encode_14_bytes() {
    let enc = hecc_encode(&DATA8[..14]);
    assert_eq!(enc.len(), 15);
    assert_eq!(enc[14], 0x24);
    assert_eq!(&enc[..14], &DATA8[..14]);
}

#[test]
fn test_encode_30_bytes() {
    let input: Vec<u8> = (0..30).collect();
    let enc = hecc_encode(&input);
    assert_eq!(enc.len(), 32);
    // First block check byte at index 15
    assert_eq!(enc[15], 0xb1);
    // Second block check byte at index 31
    assert_eq!(enc[31], 0x3e);
}

#[test]
fn test_encode_16_bytes() {
    let input: Vec<u8> = (0u8..16).map(|i| i.wrapping_mul(17)).collect();
    let enc = hecc_encode(&input);
    assert_eq!(enc.len(), 18);
    assert_eq!(enc[15], 0x72);
    assert_eq!(enc[17], 0x03);
}

#[test]
fn test_encode_all_zeros() {
    let enc = hecc_encode(&[0u8; 15]);
    assert_eq!(enc, vec![0u8; 16]);
}

#[test]
fn test_encode_all_ff() {
    let enc = hecc_encode(&[0xffu8; 15]);
    assert_eq!(enc, vec![0xffu8; 16]);
}

// === hecc_decode ===

#[test]
fn test_decode_known_good() {
    let dec = hecc_decode(&DATA8_ENC);
    assert_eq!(dec, Some(DATA8.to_vec()));
}

#[test]
fn test_decode_empty() {
    let dec = hecc_decode(&[]);
    assert_eq!(dec, Some(vec![]));
}

#[test]
fn test_decode_truncated_1() {
    assert_eq!(hecc_decode(&[0x00]), None);
}

#[test]
fn test_decode_truncated_17() {
    assert_eq!(hecc_decode(&[0u8; 17]), None);
}

#[test]
fn test_decode_double_error() {
    let mut bad = DATA8_ENC;
    bad[0] ^= 3; // flip 2 adjacent bits
    assert_eq!(hecc_decode(&bad), None);
}

#[test]
fn test_decode_2_bytes() {
    let dec = hecc_decode(&[0xab, 0x07]);
    assert_eq!(dec, Some(vec![0xab]));
}

// === roundtrip ===

#[test]
fn test_roundtrip_full_block() {
    let enc = hecc_encode(&DATA8);
    let dec = hecc_decode(&enc);
    assert_eq!(dec, Some(DATA8.to_vec()));
}

#[test]
fn test_roundtrip_partial() {
    let input = vec![0x42, 0x13, 0x37];
    let enc = hecc_encode(&input);
    let dec = hecc_decode(&enc);
    assert_eq!(dec, Some(input));
}

#[test]
fn test_roundtrip_multi_block() {
    let input: Vec<u8> = (0..30).collect();
    let enc = hecc_encode(&input);
    let dec = hecc_decode(&enc);
    assert_eq!(dec, Some(input));
}

// === single bit error correction ===

#[test]
fn test_single_bit_error_data() {
    for idx in 0..(15 * 8) {
        let mut tmp = DATA8_ENC;
        tmp[idx / 8] ^= 1 << (idx % 8);
        let dec = hecc_decode(&tmp);
        assert_eq!(dec, Some(DATA8.to_vec()), "failed at data bit {idx}");
    }
}

#[test]
fn test_single_bit_error_code() {
    for idx in 0..7 {
        let mut tmp = DATA8_ENC;
        tmp[15] ^= 1 << idx;
        let dec = hecc_decode(&tmp);
        assert_eq!(dec, Some(DATA8.to_vec()), "failed at code bit {idx}");
    }
}

#[test]
fn test_single_bit_error_parity() {
    let mut tmp = DATA8_ENC;
    tmp[15] ^= 1 << 7;
    let dec = hecc_decode(&tmp);
    assert_eq!(dec, Some(DATA8.to_vec()));
}

fn main() {}
