use libwecan::libwecan::*;

const PRECISION: f64 = 0.00001;

fn cmp_double(d1: f64, d2: f64) -> bool {
    (d1 - PRECISION) < d2 && (d1 + PRECISION) > d2
}

fn cmp_float(f1: f32, f2: f32) -> bool {
    let (d1, d2) = (f1 as f64, f2 as f64);
    (d1 - PRECISION) < d2 && (d1 + PRECISION) > d2
}

// ==========================================================================
// EXTRACT MOTOROLA
// ==========================================================================

#[test]
fn test_extract_motorola_1_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    assert_eq!(extract(&frame, 7, 8, UNSIGNED, MOTOROLA), 255);
}

#[test]
fn test_extract_motorola_1_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    frame[1] = 0xFD;
    let val = extract(&frame, 15, 8, SIGNED, MOTOROLA) as i64;
    assert_eq!(val, -3);
}

#[test]
fn test_extract_motorola_1_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x0E;
    assert_eq!(extract(&frame, 27, 3, UNSIGNED, MOTOROLA), 7);
}

#[test]
fn test_extract_motorola_1_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x3F;
    assert_eq!(extract(&frame, 21, 6, UNSIGNED, MOTOROLA), 63);
}

#[test]
fn test_extract_motorola_1_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0x0B;
    let val = extract(&frame, 35, 4, SIGNED, MOTOROLA) as i64;
    assert_eq!(val, -5);
}

#[test]
fn test_extract_motorola_2_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0xCD;
    frame[7] = 0xAB;
    assert_eq!(extract(&frame, 55, 16, UNSIGNED, MOTOROLA), 52651);
}

#[test]
fn test_extract_motorola_2_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xF7;
    let val = extract(&frame, 39, 16, SIGNED, MOTOROLA) as i64;
    assert_eq!(val, -9);
}

#[test]
fn test_extract_motorola_2_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x07;
    frame[4] = 0xFC;
    assert_eq!(extract(&frame, 26, 9, UNSIGNED, MOTOROLA), 511);
}

#[test]
fn test_extract_motorola_2_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x3F;
    frame[4] = 0xFF;
    assert_eq!(extract(&frame, 29, 14, UNSIGNED, MOTOROLA), 16383);
}

#[test]
fn test_extract_motorola_2_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[2] = 0x04;
    frame[3] = 0xEB;
    let val = extract(&frame, 18, 11, SIGNED, MOTOROLA) as i64;
    assert_eq!(val, -789);
}

#[test]
fn test_extract_motorola_2_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    for i in 0..7 { frame[i] = 0xFF; }
    assert_eq!(extract(&frame, 7, 56, UNSIGNED, MOTOROLA), 72057594037927935);
}

#[test]
fn test_extract_motorola_2_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xDC;
    frame[6] = 0x35;
    frame[7] = 0x5E;
    let val = extract(&frame, 39, 32, SIGNED, MOTOROLA) as i64;
    assert_eq!(val, -2345634);
}

// ==========================================================================
// EXTRACT INTEL
// ==========================================================================

#[test]
fn test_extract_intel_3_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    assert_eq!(extract(&frame, 0, 8, UNSIGNED, INTEL), 255);
}

#[test]
fn test_extract_intel_3_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    frame[5] = 0xDF;
    let val = extract(&frame, 40, 8, SIGNED, INTEL) as i64;
    assert_eq!(val, -33);
}

#[test]
fn test_extract_intel_3_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x5E;
    assert_eq!(extract(&frame, 17, 7, UNSIGNED, INTEL), 47);
}

#[test]
fn test_extract_intel_3_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0x76;
    assert_eq!(extract(&frame, 48, 7, UNSIGNED, INTEL), 118);
}

#[test]
fn test_extract_intel_3_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xD3;
    let val = extract(&frame, 32, 8, SIGNED, INTEL) as i64;
    assert_eq!(val, -45);
}

#[test]
fn test_extract_intel_4_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0xFA;
    frame[4] = 0xD1;
    assert_eq!(extract(&frame, 24, 16, UNSIGNED, INTEL), 53754);
}

#[test]
fn test_extract_intel_4_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    frame[6] = 0x19;
    frame[7] = 0xFC;
    let val = extract(&frame, 48, 16, SIGNED, INTEL) as i64;
    assert_eq!(val, -999);
}

#[test]
fn test_extract_intel_4_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xEC;
    frame[1] = 0x34;
    assert_eq!(extract(&frame, 2, 12, UNSIGNED, INTEL), 3387);
}

#[test]
fn test_extract_intel_4_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x75;
    frame[3] = 0x03;
    assert_eq!(extract(&frame, 16, 11, UNSIGNED, INTEL), 885);
}

#[test]
fn test_extract_intel_4_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[5] = 0xF6;
    frame[6] = 0xE5;
    let val = extract(&frame, 40, 16, SIGNED, INTEL) as i64;
    assert_eq!(val, -6666);
}

#[test]
fn test_extract_intel_4_6_seven_bytes_unsigned() {
    let frame: [u8; 8] = [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0x00];
    assert_eq!(extract(&frame, 0, 56, UNSIGNED, INTEL), 48413335211474859);
}

#[test]
fn test_extract_intel_4_7_four_bytes_signed() {
    let frame: [u8; 8] = [0x96, 0x91, 0xE6, 0xFF, 0x00, 0x00, 0x00, 0x00];
    let val = extract(&frame, 0, 32, SIGNED, INTEL) as i64;
    assert_eq!(val, -1666666);
}

// ==========================================================================
// INSERT MOTOROLA
// ==========================================================================

#[test]
fn test_insert_motorola_5_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, 6, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x06, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_5_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, -15i64 as u64, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0xF1, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_5_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 6, 63, MOTOROLA);
    assert_eq!(frame, [0xFC, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_5_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 47, 8, 113, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0x71, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_5_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 23, 8, -113i64 as u64, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x8F, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_6_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 55, 16, 30126, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x75, 0xAE]);
}

#[test]
fn test_insert_motorola_6_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 16, -59595i64 as u64, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x17, 0x35, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_6_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 21, 9, 189, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x17, 0xA0, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_6_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 34, 11, 1390, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x05, 0x6E, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_6_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 16, -24244i64 as u64, MOTOROLA);
    assert_eq!(frame, [0xA1, 0x4C, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_motorola_6_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 56, 48413335211474859, MOTOROLA);
    assert_eq!(frame, [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0x00]);
}

#[test]
fn test_insert_motorola_6_7_four_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 32, -489i64 as u64, MOTOROLA);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0xFF, 0xFF, 0xFE, 0x17]);
}

// ==========================================================================
// INSERT INTEL
// ==========================================================================

#[test]
fn test_insert_intel_7_1_one_byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 8, 240, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0xF0, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_7_2_one_byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 8, -202i64 as u64, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x36, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_7_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 29, 3, 7, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0xE0, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_7_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 56, 5, 23, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x17]);
}

#[test]
fn test_insert_intel_7_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 40, 8, -199i64 as u64, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0x39, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_8_1_two_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 16, 52077, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x6D, 0xCB, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_8_2_two_bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 16, -48666i64 as u64, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0xE6, 0x41, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_8_3_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 44, 11, 1707, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0xB0, 0x6A, 0x00]);
}

#[test]
fn test_insert_intel_8_4_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 8, 10, 1023, INTEL);
    assert_eq!(frame, [0x00, 0xFF, 0x03, 0x00, 0x00, 0x00, 0x00, 0x00]);
}

#[test]
fn test_insert_intel_8_5_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 48, 16, -59821i64 as u64, INTEL);
    assert_eq!(frame, [0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x53, 0x16]);
}

#[test]
fn test_insert_intel_8_6_seven_bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 0, 56, 48413335211474859, INTEL);
    assert_eq!(frame, [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0x00]);
}

#[test]
fn test_insert_intel_8_7_four_bytes_motorola() {
    // Note: C test 8.7 actually uses MOTOROLA endianness
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 32, -1339i64 as u64, MOTOROLA);
    assert_eq!(frame, [0xFF, 0xFF, 0xFA, 0xC5, 0x00, 0x00, 0x00, 0x00]);
}

// ==========================================================================
// ENCODE/DECODE ROUNDTRIPS
// ==========================================================================

#[test]
fn test_encode_decode_9_1_motorola_double() {
    let mut frame = [0u8; 8];
    encode_double(&mut frame, 66.66666, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert_eq!(&frame[..4], &[0x27, 0xBC, 0x86, 0x68]);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, 66.66666));
}

#[test]
fn test_encode_decode_9_2_motorola_double_negative() {
    let mut frame = [0u8; 8];
    encode_double(&mut frame, -50.6164129, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert_eq!(&frame[..4], &[0xE1, 0xD4, 0x8C, 0x5F]);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, -50.6164129));
}

#[test]
fn test_encode_decode_9_3_motorola_uint() {
    let mut frame = [0u8; 8];
    encode_uint64_t(&mut frame, 666666666, 7, 32, MOTOROLA, 1.0, 0.0);
    assert_eq!(&frame[..4], &[0x27, 0xBC, 0x86, 0xAA]);
    assert_eq!(decode_uint64_t(&frame, 7, 32, MOTOROLA, 1.0, 0.0), 666666666);
}

#[test]
fn test_encode_decode_9_4_intel_double() {
    let mut frame = [0u8; 8];
    encode_double(&mut frame, 8.4939123, 0, 32, INTEL, 0.0000001, 0.0);
    assert_eq!(&frame[..4], &[0x73, 0x11, 0x10, 0x05]);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, 8.4939123));
}

#[test]
fn test_encode_decode_9_5_intel_double_negative() {
    let mut frame = [0u8; 8];
    encode_double(&mut frame, -7.7979897, 0, 32, INTEL, 0.0000001, 0.0);
    assert_eq!(&frame[..4], &[0x07, 0x1F, 0x5A, 0xFB]);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, -7.7979897));
}

#[test]
fn test_encode_decode_9_6_intel_uint() {
    let mut frame = [0u8; 8];
    encode_uint64_t(&mut frame, 999999999, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(&frame[..4], &[0xFF, 0xC9, 0x9A, 0x3B]);
    assert_eq!(decode_uint64_t(&frame, 0, 32, INTEL, 1.0, 0.0), 999999999);
}

#[test]
fn test_encode_decode_9_7_intel_int_negative() {
    let mut frame = [0u8; 8];
    encode_int64_t(&mut frame, -1029384756, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(&frame[..4], &[0xCC, 0xD5, 0xA4, 0xC2]);
    assert_eq!(decode_int64_t(&frame, 0, 32, INTEL, 1.0, 0.0), -1029384756);
}

#[test]
fn test_encode_decode_9_8_motorola_float_negative() {
    let mut frame = [0u8; 8];
    encode_float(&mut frame, -2938.345666f32, 7, 40, MOTOROLA, 0.0000001, 0.0);
    assert_eq!(&frame[..5], &[0xF9, 0x28, 0x9C, 0x06, 0xF9]);
    let decoded = decode_float(&frame, 7, 40, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, -2938.345666f32));
}

// ==========================================================================
// ENCODE/DECODE FDFRAME ROUNDTRIPS
// ==========================================================================

#[test]
fn test_encode_decode_9_9_intel_uint_fdframe() {
    let mut frame = [0u8; 40];
    encode_uint64_t(&mut frame, 999999999, 288, 32, INTEL, 1.0, 0.0);
    assert_eq!(frame[36], 0xFF);
    assert_eq!(frame[37], 0xC9);
    assert_eq!(frame[38], 0x9A);
    assert_eq!(frame[39], 0x3B);
    assert_eq!(decode_uint64_t(&frame, 288, 32, INTEL, 1.0, 0.0), 999999999);
}

#[test]
fn test_encode_decode_10_0_motorola_int_fdframe() {
    let mut frame = [0u8; 56];
    encode_int64_t(&mut frame, -7777, 431, 16, MOTOROLA, 1.0, 0.0);
    assert_eq!(frame[53], 0xE1);
    assert_eq!(frame[54], 0x9F);
    assert_eq!(decode_int64_t(&frame, 431, 16, MOTOROLA, 1.0, 0.0), -7777);
}

#[test]
fn test_encode_decode_10_1_intel_int_negative_fdframe() {
    let mut frame = [0u8; 48];
    encode_int64_t(&mut frame, -1029384756, 184, 32, INTEL, 1.0, 0.0);
    assert_eq!(frame[23], 0xCC);
    assert_eq!(frame[24], 0xD5);
    assert_eq!(frame[25], 0xA4);
    assert_eq!(frame[26], 0xC2);
    assert_eq!(decode_int64_t(&frame, 184, 32, INTEL, 1.0, 0.0), -1029384756);
}

#[test]
fn test_encode_decode_10_2_motorola_float_fdframe() {
    let mut frame = [0u8; 64];
    encode_float(&mut frame, 8.49391f32, 383, 32, MOTOROLA, 0.0000001, 0.0);
    assert_eq!(frame[47], 0x05);
    assert_eq!(frame[48], 0x10);
    assert_eq!(frame[49], 0x11);
    assert_eq!(frame[50], 0x5A);
    let decoded = decode_float(&frame, 383, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, 8.49391f32));
}

#[test]
fn test_encode_decode_10_3_intel_double_negative_fdframe() {
    let mut frame = [0u8; 24];
    encode_double(&mut frame, -7.7979897, 32, 32, INTEL, 0.0000001, 0.0);
    assert_eq!(frame[4], 0x07);
    assert_eq!(frame[5], 0x1F);
    assert_eq!(frame[6], 0x5A);
    assert_eq!(frame[7], 0xFB);
    let decoded = decode_double(&frame, 32, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, -7.7979897));
}

fn main() {}
