use libwecan::libwecan::*;

const PRECISION: f64 = 0.00001;

fn cmp_float(f1: f32, f2: f32) -> bool {
    ((f1 as f64 - PRECISION) < f2 as f64) && ((f1 as f64 + PRECISION) > f2 as f64)
}

fn cmp_double(d1: f64, d2: f64) -> bool {
    (d1 - PRECISION) < d2 && (d1 + PRECISION) > d2
}

// ==========================================================================
// EXTRACT MOTOROLA
// ==========================================================================

#[test]
fn test_extract_motorola_1byte_unsigned_full() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    assert_eq!(extract(&frame, 7, 8, UNSIGNED, MOTOROLA), 255);
}

#[test]
fn test_extract_motorola_1byte_signed() {
    let mut frame = [0u8; 8];
    frame[1] = 0xFD;
    assert_eq!(extract(&frame, 15, 8, SIGNED, MOTOROLA) as i64, -3);
}

#[test]
fn test_extract_motorola_1byte_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x0E;
    assert_eq!(extract(&frame, 27, 3, UNSIGNED, MOTOROLA), 7);
}

#[test]
fn test_extract_motorola_1byte_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x3F;
    assert_eq!(extract(&frame, 21, 6, UNSIGNED, MOTOROLA), 63);
}

#[test]
fn test_extract_motorola_1byte_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0x0B;
    assert_eq!(extract(&frame, 35, 4, SIGNED, MOTOROLA) as i64, -5);
}

#[test]
fn test_extract_motorola_2bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0xCD;
    frame[7] = 0xAB;
    assert_eq!(extract(&frame, 55, 16, UNSIGNED, MOTOROLA), 52651);
}

#[test]
fn test_extract_motorola_2bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xF7;
    assert_eq!(extract(&frame, 39, 16, SIGNED, MOTOROLA) as i64, -9);
}

#[test]
fn test_extract_motorola_2bytes_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x07;
    frame[4] = 0xFC;
    assert_eq!(extract(&frame, 26, 9, UNSIGNED, MOTOROLA), 511);
}

#[test]
fn test_extract_motorola_2bytes_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0x3F;
    frame[4] = 0xFF;
    assert_eq!(extract(&frame, 29, 14, UNSIGNED, MOTOROLA), 16383);
}

#[test]
fn test_extract_motorola_2bytes_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[2] = 0x04;
    frame[3] = 0xEB;
    assert_eq!(extract(&frame, 18, 11, SIGNED, MOTOROLA) as i64, -789);
}

#[test]
fn test_extract_motorola_7bytes_unsigned() {
    let mut frame = [0xFFu8; 8];
    // 7 bytes all 0xFF
    assert_eq!(extract(&frame, 7, 56, UNSIGNED, MOTOROLA), 72057594037927935);
}

#[test]
fn test_extract_motorola_4bytes_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xDC;
    frame[6] = 0x35;
    frame[7] = 0x5E;
    assert_eq!(extract(&frame, 39, 32, SIGNED, MOTOROLA) as i64, -2345634);
}

// ==========================================================================
// EXTRACT INTEL
// ==========================================================================

#[test]
fn test_extract_intel_1byte_unsigned_full() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    assert_eq!(extract(&frame, 0, 8, UNSIGNED, INTEL), 255);
}

#[test]
fn test_extract_intel_1byte_signed() {
    let mut frame = [0u8; 8];
    frame[5] = 0xDF;
    assert_eq!(extract(&frame, 40, 8, SIGNED, INTEL) as i64, -33);
}

#[test]
fn test_extract_intel_1byte_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x5E;
    assert_eq!(extract(&frame, 17, 7, UNSIGNED, INTEL), 47);
}

#[test]
fn test_extract_intel_1byte_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[6] = 0x76;
    assert_eq!(extract(&frame, 48, 7, UNSIGNED, INTEL), 118);
}

#[test]
fn test_extract_intel_1byte_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[4] = 0xD3;
    assert_eq!(extract(&frame, 32, 8, SIGNED, INTEL) as i64, -45);
}

#[test]
fn test_extract_intel_2bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[3] = 0xFA;
    frame[4] = 0xD1;
    assert_eq!(extract(&frame, 24, 16, UNSIGNED, INTEL), 53754);
}

#[test]
fn test_extract_intel_2bytes_signed() {
    let mut frame = [0u8; 8];
    frame[6] = 0x19;
    frame[7] = 0xFC;
    assert_eq!(extract(&frame, 48, 16, SIGNED, INTEL) as i64, -999);
}

#[test]
fn test_extract_intel_2bytes_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xEC;
    frame[1] = 0x34;
    assert_eq!(extract(&frame, 2, 12, UNSIGNED, INTEL), 3387);
}

#[test]
fn test_extract_intel_2bytes_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    frame[2] = 0x75;
    frame[3] = 0x03;
    assert_eq!(extract(&frame, 16, 11, UNSIGNED, INTEL), 885);
}

#[test]
fn test_extract_intel_2bytes_lsb_start_signed() {
    let mut frame = [0u8; 8];
    frame[5] = 0xF6;
    frame[6] = 0xE5;
    assert_eq!(extract(&frame, 40, 16, SIGNED, INTEL) as i64, -6666);
}

#[test]
fn test_extract_intel_7bytes_unsigned() {
    let mut frame = [0u8; 8];
    frame[0] = 0xAB; frame[1] = 0xFF; frame[2] = 0xAB; frame[3] = 0xFF;
    frame[4] = 0xAB; frame[5] = 0xFF; frame[6] = 0xAB;
    assert_eq!(extract(&frame, 0, 56, UNSIGNED, INTEL), 48413335211474859);
}

#[test]
fn test_extract_intel_4bytes_signed() {
    let mut frame = [0u8; 8];
    frame[0] = 0x96; frame[1] = 0x91; frame[2] = 0xE6; frame[3] = 0xFF;
    assert_eq!(extract(&frame, 0, 32, SIGNED, INTEL) as i64, -1666666);
}

// ==========================================================================
// INSERT MOTOROLA
// ==========================================================================

#[test]
fn test_insert_motorola_1byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, 6, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0x06, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_1byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 31, 8, -15i64 as u64, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0xF1, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 6, 63, MOTOROLA);
    assert_eq!(frame, [0xFC, 0, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 47, 8, 113, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0x71, 0, 0]);
}

#[test]
fn test_insert_motorola_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 23, 8, -113i64 as u64, MOTOROLA);
    assert_eq!(frame, [0, 0, 0x8F, 0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_2bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 55, 16, 30126, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0, 0x75, 0xAE]);
}

#[test]
fn test_insert_motorola_2bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 16, -59595i64 as u64, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0, 0x17, 0x35, 0, 0]);
}

#[test]
fn test_insert_motorola_2bytes_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 21, 9, 189, MOTOROLA);
    assert_eq!(frame, [0, 0, 0x17, 0xA0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_2bytes_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 34, 11, 1390, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0, 0x05, 0x6E, 0, 0]);
}

#[test]
fn test_insert_motorola_2bytes_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 16, -24244i64 as u64, MOTOROLA);
    assert_eq!(frame, [0xA1, 0x4C, 0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_motorola_7bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 56, 48413335211474859, MOTOROLA);
    assert_eq!(frame, [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0]);
}

#[test]
fn test_insert_motorola_4bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 39, 32, -489i64 as u64, MOTOROLA);
    assert_eq!(frame, [0, 0, 0, 0, 0xFF, 0xFF, 0xFE, 0x17]);
}

// ==========================================================================
// INSERT INTEL
// ==========================================================================

#[test]
fn test_insert_intel_1byte_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 8, 240, INTEL);
    assert_eq!(frame, [0, 0, 0xF0, 0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_intel_1byte_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 8, -202i64 as u64, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0x36, 0, 0, 0]);
}

#[test]
fn test_insert_intel_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 29, 3, 7, INTEL);
    assert_eq!(frame, [0, 0, 0, 0xE0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_intel_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 56, 5, 23, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0, 0, 0x17]);
}

#[test]
fn test_insert_intel_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 40, 8, -199i64 as u64, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0x39, 0, 0]);
}

#[test]
fn test_insert_intel_2bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 16, 16, 52077, INTEL);
    assert_eq!(frame, [0, 0, 0x6D, 0xCB, 0, 0, 0, 0]);
}

#[test]
fn test_insert_intel_2bytes_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 32, 16, -48666i64 as u64, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0xE6, 0x41, 0, 0]);
}

#[test]
fn test_insert_intel_2bytes_lsb_middle_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 44, 11, 1707, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0xB0, 0x6A, 0]);
}

#[test]
fn test_insert_intel_2bytes_lsb_start_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 8, 10, 1023, INTEL);
    assert_eq!(frame, [0, 0xFF, 0x03, 0, 0, 0, 0, 0]);
}

#[test]
fn test_insert_intel_2bytes_lsb_start_signed() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 48, 16, -59821i64 as u64, INTEL);
    assert_eq!(frame, [0, 0, 0, 0, 0, 0, 0x53, 0x16]);
}

#[test]
fn test_insert_intel_7bytes_unsigned() {
    let mut frame = [0u8; 8];
    insert(&mut frame, 0, 56, 48413335211474859, INTEL);
    assert_eq!(frame, [0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0xFF, 0xAB, 0]);
}

#[test]
fn test_insert_intel_4bytes_signed_motorola() {
    // Note: C test step 8.7 actually uses MOTOROLA despite being in the INTEL section
    let mut frame = [0u8; 8];
    insert(&mut frame, 7, 32, -1339i64 as u64, MOTOROLA);
    assert_eq!(frame, [0xFF, 0xFF, 0xFA, 0xC5, 0, 0, 0, 0]);
}

// ==========================================================================
// ENCODE/DECODE MOTOROLA double
// ==========================================================================

#[test]
fn test_encode_decode_motorola_double() {
    let mut frame = [0u8; 8];
    let val = 66.66666;
    encode_double(&mut frame, val, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, val));
}

#[test]
fn test_encode_decode_motorola_double_negative() {
    let mut frame = [0u8; 8];
    let val = -50.6164129;
    encode_double(&mut frame, val, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, val));
}

#[test]
fn test_encode_decode_motorola_uint() {
    let mut frame = [0u8; 8];
    let val: u64 = 666666666;
    encode_uint64_t(&mut frame, val, 7, 32, MOTOROLA, 1.0, 0.0);
    assert_eq!(decode_uint64_t(&frame, 7, 32, MOTOROLA, 1.0, 0.0), val);
}

// ==========================================================================
// ENCODE/DECODE INTEL double
// ==========================================================================

#[test]
fn test_encode_decode_intel_double() {
    let mut frame = [0u8; 8];
    let val = 8.4939123;
    encode_double(&mut frame, val, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, val));
}

#[test]
fn test_encode_decode_intel_double_negative() {
    let mut frame = [0u8; 8];
    let val = -7.7979897;
    encode_double(&mut frame, val, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, val));
}

#[test]
fn test_encode_decode_intel_uint() {
    let mut frame = [0u8; 8];
    let val: u64 = 999999999;
    encode_uint64_t(&mut frame, val, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decode_uint64_t(&frame, 0, 32, INTEL, 1.0, 0.0), val);
}

#[test]
fn test_encode_decode_intel_int_negative() {
    let mut frame = [0u8; 8];
    let val: i64 = -1029384756;
    encode_int64_t(&mut frame, val, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decode_int64_t(&frame, 0, 32, INTEL, 1.0, 0.0), val);
}

// ==========================================================================
// ENCODE/DECODE MOTOROLA float negative
// ==========================================================================

#[test]
fn test_encode_decode_motorola_float_negative() {
    let mut frame = [0u8; 8];
    let val: f32 = -2938.345666;
    encode_float(&mut frame, val, 7, 40, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 7, 40, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, val));
}

// ==========================================================================
// FDFRAME tests
// ==========================================================================

#[test]
fn test_fdframe_intel_uint() {
    let mut frame = [0u8; 40];
    let val: u64 = 999999999;
    encode_uint64_t(&mut frame, val, 288, 32, INTEL, 1.0, 0.0);
    assert_eq!(decode_uint64_t(&frame, 288, 32, INTEL, 1.0, 0.0), val);
}

#[test]
fn test_fdframe_motorola_int_signed() {
    let mut frame = [0u8; 56];
    let val: i64 = -7777;
    encode_int64_t(&mut frame, val, 431, 16, MOTOROLA, 1.0, 0.0);
    assert_eq!(decode_int64_t(&frame, 431, 16, MOTOROLA, 1.0, 0.0), val);
}

#[test]
fn test_fdframe_intel_int_negative() {
    let mut frame = [0u8; 48];
    let val: i64 = -1029384756;
    encode_int64_t(&mut frame, val, 184, 32, INTEL, 1.0, 0.0);
    assert_eq!(decode_int64_t(&frame, 184, 32, INTEL, 1.0, 0.0), val);
}

#[test]
fn test_fdframe_motorola_float() {
    let mut frame = [0u8; 64];
    let val: f32 = 8.49391;
    encode_float(&mut frame, val, 383, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 383, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, val));
}

#[test]
fn test_fdframe_intel_double_negative() {
    let mut frame = [0u8; 24];
    let val = -7.7979897;
    encode_double(&mut frame, val, 32, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 32, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, val));
}

fn main() {}
