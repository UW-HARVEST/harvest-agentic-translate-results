#[allow(unused_imports)]
use libwecan::libwecan::*;

// ============================================================
// EXTRACT MOTOROLA
// ============================================================

#[test]
fn test_extract_motorola_step_1_1_unsigned_one_byte() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    let value = extract(&frame, 7, 8, UNSIGNED, MOTOROLA);
    assert_eq!(value, 255);
}

#[test]
fn test_extract_motorola_step_1_2_signed_one_byte() {
    let mut frame = [0u8; 8];
    frame[1] = 0xFD;
    let value = extract(&frame, 15, 8, SIGNED, MOTOROLA) as i64;
    assert_eq!(value, -3);
}

#[test]
fn test_extract_motorola_step_1_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x0E;
    let value = extract(&frame, 27, 3, UNSIGNED, MOTOROLA);
    assert_eq!(value, 7);
}

#[test]
fn test_extract_motorola_step_1_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x3F;
    let value = extract(&frame, 21, 6, UNSIGNED, MOTOROLA);
    assert_eq!(value, 63);
}

#[test]
fn test_extract_motorola_step_1_5_signed_lsb_start() {
    let mut frame = [0u8; 8];
    frame[4] = 0x0B;
    let value = extract(&frame, 35, 4, SIGNED, MOTOROLA) as i64;
    assert_eq!(value, -5);
}

#[test]
fn test_extract_motorola_step_2_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0xCD;
    frame[7] = 0xAB;
    let value = extract(&frame, 55, 16, UNSIGNED, MOTOROLA);
    assert_eq!(value, 52651);
}

#[test]
fn test_extract_motorola_step_2_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xF7;
    let value = extract(&frame, 39, 16, SIGNED, MOTOROLA) as i64;
    assert_eq!(value, -9);
}

#[test]
fn test_extract_motorola_step_2_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x07;
    frame[4] = 0xFC;
    let value = extract(&frame, 26, 9, UNSIGNED, MOTOROLA);
    assert_eq!(value, 511);
}

#[test]
fn test_extract_motorola_step_2_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x3F;
    frame[4] = 0xFF;
    let value = extract(&frame, 29, 14, UNSIGNED, MOTOROLA);
    assert_eq!(value, 16383);
}

#[test]
fn test_extract_motorola_step_2_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[2] = 0x04;
    frame[3] = 0xEB;
    let value = extract(&frame, 18, 11, SIGNED, MOTOROLA) as i64;
    assert_eq!(value, -789);
}

#[test]
fn test_extract_motorola_step_2_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    for i in 0..7 {
        frame[i] = 0xFF;
    }
    let value = extract(&frame, 7, 56, UNSIGNED, MOTOROLA);
    assert_eq!(value, 72057594037927935);
}

#[test]
fn test_extract_motorola_step_2_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xDC;
    frame[6] = 0x35;
    frame[7] = 0x5E;
    let value = extract(&frame, 39, 32, SIGNED, MOTOROLA) as i64;
    assert_eq!(value, -2345634);
}

// ============================================================
// EXTRACT INTEL
// ============================================================

#[test]
fn test_extract_intel_step_3_1_unsigned_one_byte() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    let value = extract(&frame, 0, 8, UNSIGNED, INTEL);
    assert_eq!(value, 255);
}

#[test]
fn test_extract_intel_step_3_2_signed_one_byte() {
    let mut frame = [0u8; 8];
    frame[5] = 0xDF;
    let value = extract(&frame, 40, 8, SIGNED, INTEL) as i64;
    assert_eq!(value, -33);
}

#[test]
fn test_extract_intel_step_3_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x5E;
    let value = extract(&frame, 17, 7, UNSIGNED, INTEL);
    assert_eq!(value, 47);
}

#[test]
fn test_extract_intel_step_3_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0x76;
    let value = extract(&frame, 48, 7, UNSIGNED, INTEL);
    assert_eq!(value, 118);
}

#[test]
fn test_extract_intel_step_3_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xD3;
    let value = extract(&frame, 32, 8, SIGNED, INTEL) as i64;
    assert_eq!(value, -45);
}

#[test]
fn test_extract_intel_step_4_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0xFA;
    frame[4] = 0xD1;
    let value = extract(&frame, 24, 16, UNSIGNED, INTEL);
    assert_eq!(value, 53754);
}

#[test]
fn test_extract_intel_step_4_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[6] = 0x19;
    frame[7] = 0xFC;
    let value = extract(&frame, 48, 16, SIGNED, INTEL) as i64;
    assert_eq!(value, -999);
}

#[test]
fn test_extract_intel_step_4_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xEC;
    frame[1] = 0x34;
    let value = extract(&frame, 2, 12, UNSIGNED, INTEL);
    assert_eq!(value, 3387);
}

#[test]
fn test_extract_intel_step_4_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x75;
    frame[3] = 0x03;
    let value = extract(&frame, 16, 11, UNSIGNED, INTEL);
    assert_eq!(value, 885);
}

#[test]
fn test_extract_intel_step_4_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[5] = 0xF6;
    frame[6] = 0xE5;
    let value = extract(&frame, 40, 16, SIGNED, INTEL) as i64;
    assert_eq!(value, -6666);
}

#[test]
fn test_extract_intel_step_4_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xAB;
    frame[1] = 0xFF;
    frame[2] = 0xAB;
    frame[3] = 0xFF;
    frame[4] = 0xAB;
    frame[5] = 0xFF;
    frame[6] = 0xAB;
    let value = extract(&frame, 0, 56, UNSIGNED, INTEL);
    assert_eq!(value, 48413335211474859);
}

#[test]
fn test_extract_intel_step_4_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[0] = 0x96;
    frame[1] = 0x91;
    frame[2] = 0xE6;
    frame[3] = 0xFF;
    let value = extract(&frame, 0, 32, SIGNED, INTEL) as i64;
    assert_eq!(value, -1666666);
}

// ============================================================
// INSERT MOTOROLA
// ============================================================

#[test]
fn test_insert_motorola_step_5_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, 6u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[3] = 0x06;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_5_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, (-15i64) as u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[3] = 0xF1;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_5_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 6, 63u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[0] = 0xFC;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_5_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 47, 8, 113u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[5] = 0x71;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_5_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 23, 8, (-113i64) as u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[2] = 0x8F;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 55, 16, 30126u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[6] = 0x75;
    expected[7] = 0xAE;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 16, (-59595i64) as u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[4] = 0x17;
    expected[5] = 0x35;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 21, 9, 189u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[2] = 0x17;
    expected[3] = 0xA0;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 34, 11, 1390u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[4] = 0x05;
    expected[5] = 0x6E;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 16, (-24244i64) as u64, MOTOROLA);
    let mut expected = [0u8; 8];
    expected[0] = 0xA1;
    expected[1] = 0x4C;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 56, 48413335211474859u64, MOTOROLA);
    let expected: [u8; 8] = [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0x00];
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_motorola_step_6_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 32, (-489i64) as u64, MOTOROLA);
    let expected: [u8; 8] = [0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFE, 0x17];
    assert_eq!(frame, expected);
}

// ============================================================
// INSERT INTEL
// ============================================================

#[test]
fn test_insert_intel_step_7_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 8, 240u64, INTEL);
    let mut expected = [0u8; 8];
    expected[2] = 0xF0;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_7_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 8, (-202i64) as u64, INTEL);
    let mut expected = [0u8; 8];
    expected[4] = 0x36;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_7_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 29, 3, 7u64, INTEL);
    let mut expected = [0u8; 8];
    expected[3] = 0xE0;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_7_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 56, 5, 23u64, INTEL);
    let mut expected = [0u8; 8];
    expected[7] = 0x17;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_7_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 40, 8, (-199i64) as u64, INTEL);
    let mut expected = [0u8; 8];
    expected[5] = 0x39;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 16, 52077u64, INTEL);
    let mut expected = [0u8; 8];
    expected[2] = 0x6D;
    expected[3] = 0xCB;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 16, (-48666i64) as u64, INTEL);
    let mut expected = [0u8; 8];
    expected[4] = 0xE6;
    expected[5] = 0x41;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 44, 11, 1707u64, INTEL);
    let mut expected = [0u8; 8];
    expected[5] = 0xB0;
    expected[6] = 0x6A;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 8, 10, 1023u64, INTEL);
    let mut expected = [0u8; 8];
    expected[1] = 0xFF;
    expected[2] = 0x03;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 48, 16, (-59821i64) as u64, INTEL);
    let mut expected = [0u8; 8];
    expected[6] = 0x53;
    expected[7] = 0x16;
    assert_eq!(frame, expected);
}

#[test]
fn test_insert_intel_step_8_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 0, 56, 48413335211474859u64, INTEL);
    let expected: [u8; 8] = [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0x00];
    assert_eq!(frame, expected);
}

// step 8.7 in C: insert -1339 with 32 bits at startbit 7 with MOTOROLA endianness
#[test]
fn test_insert_motorola_step_8_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 32, (-1339i64) as u64, MOTOROLA);
    let expected: [u8; 8] = [0xFF, 0xFF, 0xFA, 0xC5, 0x00, 0x00, 0x00, 0x00];
    assert_eq!(frame, expected);
}

// ============================================================
// ENCODE/DECODE round-trip tests
// ============================================================

#[test]
fn test_encode_decode_motorola_double_positive() {
    let mut frame = [0u8; 8];
    let dphy: f64 = 66.66666;
    encode_double(&mut frame, dphy, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!((decoded - dphy).abs() < 0.00001);
}

#[test]
fn test_encode_decode_motorola_double_negative() {
    let mut frame = [0u8; 8];
    let dphy: f64 = -50.6164129;
    encode_double(&mut frame, dphy, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!((decoded - dphy).abs() < 0.00001);
}

#[test]
fn test_encode_decode_motorola_uint() {
    let mut frame = [0u8; 8];
    let uphy: u64 = 666666666;
    encode_uint64_t(&mut frame, uphy, 7, 32, MOTOROLA, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 7, 32, MOTOROLA, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_intel_double_positive() {
    let mut frame = [0u8; 8];
    let dphy: f64 = 8.4939123;
    encode_double(&mut frame, dphy, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!((decoded - dphy).abs() < 0.00001);
}

#[test]
fn test_encode_decode_intel_double_negative() {
    let mut frame = [0u8; 8];
    let dphy: f64 = -7.7979897;
    encode_double(&mut frame, dphy, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!((decoded - dphy).abs() < 0.00001);
}

#[test]
fn test_encode_decode_intel_uint() {
    let mut frame = [0u8; 8];
    let uphy: u64 = 999999999;
    encode_uint64_t(&mut frame, uphy, 0, 32, INTEL, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_intel_signed_negative() {
    let mut frame = [0u8; 8];
    let sphy: i64 = -1029384756;
    encode_int64_t(&mut frame, sphy, 0, 32, INTEL, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_motorola_float_negative() {
    let mut frame = [0u8; 8];
    let fphy: f32 = -2938.345666;
    encode_float(&mut frame, fphy, 7, 40, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 7, 40, MOTOROLA, 0.0000001, 0.0);
    assert!((decoded - fphy).abs() < 0.00001);
}

// ============================================================
// FD-frame encode/decode tests
// ============================================================

#[test]
fn test_encode_decode_intel_uint_fdframe() {
    let mut frame = [0u8; 40];
    let uphy: u64 = 999999999;
    encode_uint64_t(&mut frame, uphy, 288, 32, INTEL, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 288, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_motorola_signed_fdframe() {
    let mut frame = [0u8; 56];
    let sphy: i64 = -7777;
    encode_int64_t(&mut frame, sphy, 431, 16, MOTOROLA, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 431, 16, MOTOROLA, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_intel_signed_negative_fdframe() {
    let mut frame = [0u8; 48];
    let sphy: i64 = -1029384756;
    encode_int64_t(&mut frame, sphy, 184, 32, INTEL, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 184, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_motorola_float_fdframe() {
    let mut frame = [0u8; 64];
    let fphy: f32 = 8.49391;
    encode_float(&mut frame, fphy, 383, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 383, 32, MOTOROLA, 0.0000001, 0.0);
    assert!((decoded - fphy).abs() < 0.00001);
}

#[test]
fn test_encode_decode_intel_double_negative_fdframe() {
    let mut frame = [0u8; 24];
    let dphy: f64 = -7.7979897;
    encode_double(&mut frame, dphy, 32, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 32, 32, INTEL, 0.0000001, 0.0);
    assert!((decoded - dphy).abs() < 0.00001);
}

// ============================================================
// Constants test
// ============================================================

#[test]
fn test_constants() {
    assert_eq!(FALSE, 0);
    assert_eq!(TRUE, 1);
    assert_eq!(UNSIGNED, 2);
    assert_eq!(SIGNED, 3);
    assert_eq!(INTEL, 4);
    assert_eq!(MOTOROLA, 5);
}

// ============================================================
// Verify exact encoded frame contents (from C tests step 10.x)
// ============================================================

#[test]
fn test_encode_intel_uint_fdframe_bytes() {
    // From C output: bytes 36..39 should hold uint 999999999 little-endian
    let mut frame = [0u8; 40];
    encode_uint64_t(&mut frame, 999999999, 288, 32, INTEL, 1.0, 0.0);
    // 999999999 = 0x3B9AC9FF
    assert_eq!(frame[36], 0xFF);
    assert_eq!(frame[37], 0xC9);
    assert_eq!(frame[38], 0x9A);
    assert_eq!(frame[39], 0x3B);
}

fn main() {}
