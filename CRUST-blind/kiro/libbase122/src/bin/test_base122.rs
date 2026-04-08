use libbase122::base122;

// --- BitReader tests ---

#[test]
fn test_bitreader_one_byte() {
    // Input: 0xFF = 11111111
    let input = [0xFFu8];
    let mut reader = base122::BitReader::new(&input);

    // Read 7 bits: should get 0b1111111 = 127
    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0x7F);

    // Read 7 more: only 1 bit left, should get 0b0000001 = 1
    let (n, v) = reader.read(7);
    assert_eq!(n, 1);
    assert_eq!(v, 1);

    // Read again: 0 bits left
    let (n, v) = reader.read(7);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

#[test]
fn test_bitreader_two_bytes() {
    // Input: 10101010 11111111
    let input = [0xAA, 0xFF];
    let mut reader = base122::BitReader::new(&input);

    // First 7 bits of 10101010: 1010101 = 0x55
    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0x55);

    // Next 7 bits: 0111111 11 -> from bit 7: 0_1111111 -> 0x3F
    let (n, v) = reader.read(7);
    assert_eq!(n, 7);
    assert_eq!(v, 0x3F);

    // Remaining 2 bits: 11 = 3
    let (n, v) = reader.read(7);
    assert_eq!(n, 2);
    assert_eq!(v, 3);
}

#[test]
fn test_bitreader_empty() {
    let input: [u8; 0] = [];
    let mut reader = base122::BitReader::new(&input);
    let (n, v) = reader.read(7);
    assert_eq!(n, 0);
    assert_eq!(v, 0);
}

// --- BitWriter tests ---

#[test]
fn test_bitwriter_one_byte() {
    let mut buf = [0u8; 1];
    {
        let mut writer = base122::BitWriter::new(Some(&mut buf), 1);
        assert!(writer.write(1, 0x0F).is_ok());
        assert!(writer.write(1, 0x0F).is_ok());
        assert!(writer.write(5, 0x0F).is_ok());
        assert!(writer.write(5, 0x0F).is_err());
    }
    assert_eq!(buf, [0xDE]);
}

#[test]
fn test_bitwriter_two_bytes() {
    let mut buf = [0u8; 2];
    {
        let mut writer = base122::BitWriter::new(Some(&mut buf), 2);
        assert!(writer.write(1, 0xFF).is_ok());
        assert!(writer.write(8, 0x0F).is_ok());
        assert!(writer.write(1, 0x0).is_ok());
        assert!(writer.write(1, 0xFF).is_ok());
        assert!(writer.write(5, 0xFF).is_ok());
        assert!(writer.write(1, 0xFF).is_err());
    }
    assert_eq!(buf, [0x87, 0xBF]);
}

#[test]
fn test_bitwriter_count_only() {
    let mut writer = base122::BitWriter::new(None, 0);
    assert!(writer.write(7, 0x55).is_ok());
    assert!(writer.write(7, 0x55).is_ok());
    assert_eq!(writer.cur_bit, 14);
}

// --- Round-trip tests from C test suite ---

#[test]
fn test_roundtrip_one_byte() {
    // data: 11111111, encoded: 01111111 01000000
    let data = vec![0xFF];
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, vec![0x7F, 0x40]);
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_roundtrip_several_bytes() {
    // data: 10101010 10101010 10101010 10101010
    // encoded: 01010101 00101010 01010101 00101010 01010000
    let data = vec![0xAA, 0xAA, 0xAA, 0xAA];
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, vec![0x55, 0x2A, 0x55, 0x2A, 0x50]);
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_roundtrip_illegal_one_byte() {
    // data: 00000000 11111111
    // encoded: 11000010 10111111 01100000
    let data = vec![0x00, 0xFF];
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, vec![0xC2, 0xBF, 0x60]);
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_roundtrip_illegal_last_two_bits() {
    // data: 11111111 11111100 (from "1111111 1111111 00")
    // encoded: 01111111 01111111 11011110 10000000
    let data = vec![0xFF, 0xFC];
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, vec![0x7F, 0x7F, 0xDE, 0x80]);
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_roundtrip_fuzz_crash_1() {
    // data: 00010101
    // encoded: 11000111 10000000
    let data = vec![0x15];
    let encoded = base122::encode(&data).unwrap();
    assert_eq!(encoded, vec![0xC7, 0x80]);
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

// --- Encode all 1s round-trip (from C test) ---

#[test]
fn test_encode_all_ones_roundtrip() {
    for i in 0..=65 {
        let data: Vec<u8> = vec![0xFF; i];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "round-trip failed for length {}", i);
    }
}

// --- Decode error tests from C test suite ---

#[test]
fn test_decode_last_byte_extra_data() {
    // "01111111 01111111 01111111" -> error "Last byte has extra data"
    let input = vec![0x7F, 0x7F, 0x7F];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_second_byte_malformed() {
    // "11011110 11111111" -> error "Second byte of two byte sequence malformed"
    let input = vec![0xDE, 0xFF];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Second byte of two byte sequence malformed"));
}

#[test]
fn test_decode_first_byte_malformed() {
    // "11111111" -> error "First byte of two byte sequence malformed"
    let input = vec![0xFF];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("First byte of two byte sequence malformed"));
}

#[test]
fn test_decode_missing_second_byte() {
    // "11011110" -> error "Two byte sequence is missing second byte"
    let input = vec![0xDE];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Two byte sequence is missing second byte"));
}

#[test]
fn test_decode_unexpected_extra_data_after_shortened() {
    // "11011110 10111111 01111111" -> error "Got unexpected extra data after shortened two byte sequence"
    let input = vec![0xDE, 0xBF, 0x7F];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Got unexpected extra data after shortened two byte sequence"));
}

#[test]
fn test_decode_unrecognized_illegal_index() {
    // "11011010 10111111" -> error "Got unrecognized illegal index"
    let input = vec![0xDA, 0xBF];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Got unrecognized illegal index"));
}

#[test]
fn test_decode_not_byte_multiple() {
    // "01111111" -> error "Decoded data is not a byte multiple"
    let input = vec![0x7F];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Decoded data is not a byte multiple"));
}

#[test]
fn test_decode_last_byte_has_extra_data_2() {
    // "01111111 01111111" -> error "Last byte has extra data"
    let input = vec![0x7F, 0x7F];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_valid_with_shortened_two_byte() {
    // "01111111 11011110 10000000" -> decoded: 11111110 = 0xFE
    let input = vec![0x7F, 0xDE, 0x80];
    let decoded = base122::decode(&input).unwrap();
    assert_eq!(decoded, vec![0xFE]);
}

#[test]
fn test_decode_shortened_extra_data_error() {
    // "01111111 11011111 10100000" -> error "Last byte has extra data"
    let input = vec![0x7F, 0xDF, 0xA0];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Last byte has extra data"));
}

#[test]
fn test_decode_two_byte_not_byte_multiple() {
    // "11011110 10000000" -> error "Decoded data is not a byte multiple"
    let input = vec![0xDE, 0x80];
    let result = base122::decode(&input);
    assert!(result.is_err());
    assert!(result.unwrap_err().message.contains("Decoded data is not a byte multiple"));
}

#[test]
fn test_decode_valid_null_illegal() {
    // "00000000 11011110 10000000" -> decoded: 00000000 = 0x00
    let input = vec![0x00, 0xDE, 0x80];
    let decoded = base122::decode(&input).unwrap();
    assert_eq!(decoded, vec![0x00]);
}

#[test]
fn test_decode_valid_two_null_bytes() {
    // "00000000 11000010 10000000" -> decoded: 00000000 00000000
    let input = vec![0x00, 0xC2, 0x80];
    let decoded = base122::decode(&input).unwrap();
    assert_eq!(decoded, vec![0x00, 0x00]);
}

#[test]
fn test_decode_valid_complex() {
    // "11001111 10000001 01100000" -> decoded: 01000101 00000111 = [0x45, 0x07]
    // Wait - let me verify: C test says decodedLen = 2, expect = "01000101 00000111"
    // 01000101 = 0x45, 00000111 = 0x07
    let input = vec![0xCF, 0x81, 0x60];
    let decoded = base122::decode(&input).unwrap();
    assert_eq!(decoded, vec![0x45, 0x07]);
}

// --- Empty input ---

#[test]
fn test_encode_empty() {
    let encoded = base122::encode(&[]).unwrap();
    assert!(encoded.is_empty());
}

#[test]
fn test_decode_empty() {
    let decoded = base122::decode(&[]).unwrap();
    assert!(decoded.is_empty());
}

// --- Round-trip with various patterns ---

#[test]
fn test_roundtrip_all_zeros() {
    for len in 1..=8 {
        let data = vec![0x00u8; len];
        let encoded = base122::encode(&data).unwrap();
        let decoded = base122::decode(&encoded).unwrap();
        assert_eq!(decoded, data, "round-trip zeros failed for length {}", len);
    }
}

#[test]
fn test_roundtrip_sequential() {
    let data: Vec<u8> = (0..=255).collect();
    let encoded = base122::encode(&data).unwrap();
    let decoded = base122::decode(&encoded).unwrap();
    assert_eq!(decoded, data);
}

#[test]
fn test_encode_no_illegal_in_output() {
    // Encoded output should never contain illegal bytes (0, 10, 13, 34, 38, 92)
    // as single-byte values (they'd be in two-byte sequences)
    let illegals = [0u8, 10, 13, 34, 38, 92];
    for len in 1..=32 {
        let data = vec![0xFFu8; len];
        let encoded = base122::encode(&data).unwrap();
        for (i, &b) in encoded.iter().enumerate() {
            if b >> 7 == 0 {
                // Single byte - should not be illegal
                assert!(!illegals.contains(&b),
                    "encoded byte {} is illegal value {} for input len {}", i, b, len);
            }
        }
    }
}

fn main() {}
