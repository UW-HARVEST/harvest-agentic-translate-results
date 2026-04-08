use FastHamming::fast_hamming;

const DATA8: [u8; 15] = [
    0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87,
    0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46,
];

const DATA8_ENC: [u8; 16] = [
    0x0b, 0x28, 0x48, 0x69, 0x5e, 0xc8, 0xeb, 0x87,
    0x7f, 0x28, 0xdf, 0x52, 0x03, 0xb4, 0x46, 0xd9,
];

#[test]
fn test_log2uint8() {
    assert_eq!(fast_hamming::log2uint8(1), 0);
    assert_eq!(fast_hamming::log2uint8(2), 1);
    assert_eq!(fast_hamming::log2uint8(3), 1);
    assert_eq!(fast_hamming::log2uint8(4), 2);
    assert_eq!(fast_hamming::log2uint8(7), 2);
    assert_eq!(fast_hamming::log2uint8(8), 3);
    assert_eq!(fast_hamming::log2uint8(16), 4);
    assert_eq!(fast_hamming::log2uint8(32), 5);
    assert_eq!(fast_hamming::log2uint8(64), 6);
    assert_eq!(fast_hamming::log2uint8(128), 7);
    assert_eq!(fast_hamming::log2uint8(255), 7);
}

#[test]
fn test_encode_full_block() {
    let encoded = fast_hamming::hecc_encode(&DATA8);
    assert_eq!(encoded.len(), 16);
    assert_eq!(encoded[15], 0xd9);
    assert_eq!(encoded.as_slice(), &DATA8_ENC);
}

#[test]
fn test_decode_full_block() {
    let decoded = fast_hamming::hecc_decode(&DATA8_ENC);
    assert!(decoded.is_some());
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), 15);
    assert_eq!(decoded.as_slice(), &DATA8);
}

#[test]
fn test_roundtrip_full_block() {
    let encoded = fast_hamming::hecc_encode(&DATA8);
    assert_eq!(encoded.as_slice(), &DATA8_ENC);
    let decoded = fast_hamming::hecc_decode(&encoded);
    assert!(decoded.is_some());
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), 15);
    assert_eq!(decoded.as_slice(), &DATA8);
}

#[test]
fn test_encode_partial_block() {
    let input: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
    let encoded = fast_hamming::hecc_encode(&input);
    assert_eq!(encoded.len(), 6);
    assert_eq!(encoded.as_slice(), &[0x01, 0x02, 0x03, 0x04, 0x05, 0x23]);
}

#[test]
fn test_encode_empty() {
    let encoded = fast_hamming::hecc_encode(&[]);
    assert_eq!(encoded.len(), 0);
}

#[test]
fn test_encode_two_full_blocks() {
    let data30: Vec<u8> = (1..=30).collect();
    let encoded = fast_hamming::hecc_encode(&data30);
    assert_eq!(encoded.len(), 32);
    let expected: [u8; 32] = [
        0x01, 0x02, 0x03, 0x04, 0x05, 0x06, 0x07, 0x08,
        0x09, 0x0a, 0x0b, 0x0c, 0x0d, 0x0e, 0x0f, 0x14,
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0xb0,
    ];
    assert_eq!(encoded.as_slice(), &expected);
}

#[test]
fn test_encode_mixed_full_and_partial() {
    let data20: Vec<u8> = (0x10..0x24).collect();
    let encoded = fast_hamming::hecc_encode(&data20);
    assert_eq!(encoded.len(), 22);
    let expected: [u8; 22] = [
        0x10, 0x11, 0x12, 0x13, 0x14, 0x15, 0x16, 0x17,
        0x18, 0x19, 0x1a, 0x1b, 0x1c, 0x1d, 0x1e, 0xb0,
        0x1f, 0x20, 0x21, 0x22, 0x23, 0x08,
    ];
    assert_eq!(encoded.as_slice(), &expected);
}

#[test]
fn test_roundtrip_mixed() {
    let data20: Vec<u8> = (0x10..0x24).collect();
    let encoded = fast_hamming::hecc_encode(&data20);
    assert_eq!(encoded.len(), 22);
    let decoded = fast_hamming::hecc_decode(&encoded);
    assert!(decoded.is_some());
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), 20);
    assert_eq!(decoded.as_slice(), data20.as_slice());
}

#[test]
fn test_decode_invalid_length_mod16_eq1() {
    let buf = [0u8; 1];
    assert!(fast_hamming::hecc_decode(&buf).is_none());
}

#[test]
fn test_single_bit_error_correction_data() {
    // Flip each data bit in the encoded block; decoder should correct it
    for idx in 0..(15 * 8) {
        let mut corrupted = DATA8_ENC;
        corrupted[idx >> 3] ^= 1 << (idx % 8);
        let decoded = fast_hamming::hecc_decode(&corrupted);
        assert!(decoded.is_some(), "failed to decode with data bit {} flipped", idx);
        let decoded = decoded.unwrap();
        assert_eq!(decoded.len(), 15);
        assert_eq!(decoded.as_slice(), &DATA8, "mismatch correcting data bit {}", idx);
    }
}

#[test]
fn test_single_bit_error_correction_check_bits() {
    // Flip each of the 7 lower bits of the check byte
    for idx in 0..7 {
        let mut corrupted = DATA8_ENC;
        corrupted[15] ^= 1 << idx;
        let decoded = fast_hamming::hecc_decode(&corrupted);
        assert!(decoded.is_some(), "failed to decode with check bit {} flipped", idx);
        let decoded = decoded.unwrap();
        assert_eq!(decoded.len(), 15);
        assert_eq!(decoded.as_slice(), &DATA8, "mismatch correcting check bit {}", idx);
    }
}

#[test]
fn test_single_bit_error_correction_parity_bit() {
    // Flip the parity bit (bit 7 of check byte)
    let mut corrupted = DATA8_ENC;
    corrupted[15] ^= 1 << 7;
    let decoded = fast_hamming::hecc_decode(&corrupted);
    assert!(decoded.is_some());
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), 15);
    assert_eq!(decoded.as_slice(), &DATA8);
}

#[test]
fn test_double_bit_error_detection() {
    let mut corrupted = DATA8_ENC;
    corrupted[0] ^= 3; // flip two adjacent bits
    assert!(fast_hamming::hecc_decode(&corrupted).is_none());
}

#[test]
fn test_roundtrip_partial_block() {
    let input: [u8; 5] = [0x01, 0x02, 0x03, 0x04, 0x05];
    let encoded = fast_hamming::hecc_encode(&input);
    let decoded = fast_hamming::hecc_decode(&encoded);
    assert!(decoded.is_some());
    let decoded = decoded.unwrap();
    assert_eq!(decoded.len(), 5);
    assert_eq!(decoded.as_slice(), &input);
}

fn main() {}
