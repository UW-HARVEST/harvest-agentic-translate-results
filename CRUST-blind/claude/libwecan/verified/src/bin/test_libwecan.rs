use libwecan::libwecan::{
    decode_double, decode_float, decode_int64_t, decode_uint64_t, encode_double, encode_float,
    encode_int64_t, encode_uint64_t, extract, insert, FALSE, INTEL, MOTOROLA, SIGNED, TRUE,
    UNSIGNED,
};

const PRECISION: f64 = 0.00001;

fn cmp_double(d1: f64, d2: f64) -> bool {
    (d1 - PRECISION) < d2 && (d1 + PRECISION) > d2
}

// Mirrors C cmp_float: PRECISION is a double, so the float operands are
// promoted to double before subtraction/comparison.
fn cmp_float(f1: f32, f2: f32) -> bool {
    let f1d = f1 as f64;
    let f2d = f2 as f64;
    (f1d - PRECISION) < f2d && (f1d + PRECISION) > f2d
}

fn frames_equal(expected: &[u8], actual: &[u8]) -> bool {
    if expected.len() != actual.len() {
        return false;
    }
    expected.iter().zip(actual.iter()).all(|(a, b)| a == b)
}

// ---------------------------------------------------------------------
// Constants
// ---------------------------------------------------------------------

#[test]
fn test_constants() {
    assert_eq!(FALSE, 0);
    assert_eq!(TRUE, 1);
    assert_eq!(UNSIGNED, 2);
    assert_eq!(SIGNED, 3);
    assert_eq!(INTEL, 4);
    assert_eq!(MOTOROLA, 5);
}

// ---------------------------------------------------------------------
// EXTRACT MOTOROLA tests (mirroring tests.c step 1.1 - 2.7)
// ---------------------------------------------------------------------

#[test]
fn test_extract_motorola_step_1_1() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    let v = extract(&frame, 7, 8, UNSIGNED, MOTOROLA);
    assert_eq!(v, 255);
}

#[test]
fn test_extract_motorola_step_1_2() {
    let mut frame = [0u8; 8];
    frame[1] = 0xFD;
    let v = extract(&frame, 15, 8, SIGNED, MOTOROLA) as i64;
    assert_eq!(v, -3);
}

#[test]
fn test_extract_motorola_step_1_3() {
    let mut frame = [0u8; 8];
    frame[3] = 0x0E;
    let v = extract(&frame, 27, 3, UNSIGNED, MOTOROLA);
    assert_eq!(v, 7);
}

#[test]
fn test_extract_motorola_step_1_4() {
    let mut frame = [0u8; 8];
    frame[2] = 0x3F;
    let v = extract(&frame, 21, 6, UNSIGNED, MOTOROLA);
    assert_eq!(v, 63);
}

#[test]
fn test_extract_motorola_step_1_5() {
    let mut frame = [0u8; 8];
    frame[4] = 0x0B;
    let v = extract(&frame, 35, 4, SIGNED, MOTOROLA) as i64;
    assert_eq!(v, -5);
}

#[test]
fn test_extract_motorola_step_2_1() {
    let mut frame = [0u8; 8];
    frame[6] = 0xCD;
    frame[7] = 0xAB;
    let v = extract(&frame, 55, 16, UNSIGNED, MOTOROLA);
    assert_eq!(v, 52651);
}

#[test]
fn test_extract_motorola_step_2_2() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xF7;
    let v = extract(&frame, 39, 16, SIGNED, MOTOROLA) as i64;
    assert_eq!(v, -9);
}

#[test]
fn test_extract_motorola_step_2_3() {
    let mut frame = [0u8; 8];
    frame[3] = 0x07;
    frame[4] = 0xFC;
    let v = extract(&frame, 26, 9, UNSIGNED, MOTOROLA);
    assert_eq!(v, 511);
}

#[test]
fn test_extract_motorola_step_2_4() {
    let mut frame = [0u8; 8];
    frame[3] = 0x3F;
    frame[4] = 0xFF;
    let v = extract(&frame, 29, 14, UNSIGNED, MOTOROLA);
    assert_eq!(v, 16383);
}

#[test]
fn test_extract_motorola_step_2_5() {
    let mut frame = [0u8; 8];
    frame[2] = 0x04;
    frame[3] = 0xEB;
    let v = extract(&frame, 18, 11, SIGNED, MOTOROLA) as i64;
    assert_eq!(v, -789);
}

#[test]
fn test_extract_motorola_step_2_6() {
    let mut frame = [0u8; 8];
    for i in 0..7 {
        frame[i] = 0xFF;
    }
    let v = extract(&frame, 7, 56, UNSIGNED, MOTOROLA);
    assert_eq!(v, 72057594037927935);
}

#[test]
fn test_extract_motorola_step_2_7() {
    let mut frame = [0u8; 8];
    frame[4] = 0xFF;
    frame[5] = 0xDC;
    frame[6] = 0x35;
    frame[7] = 0x5E;
    let v = extract(&frame, 39, 32, SIGNED, MOTOROLA) as i64;
    assert_eq!(v, -2345634);
}

// ---------------------------------------------------------------------
// EXTRACT INTEL tests (steps 3.1 - 4.7)
// ---------------------------------------------------------------------

#[test]
fn test_extract_intel_step_3_1() {
    let mut frame = [0u8; 8];
    frame[0] = 0xFF;
    let v = extract(&frame, 0, 8, UNSIGNED, INTEL);
    assert_eq!(v, 255);
}

#[test]
fn test_extract_intel_step_3_2() {
    let mut frame = [0u8; 8];
    frame[5] = 0xDF;
    let v = extract(&frame, 40, 8, SIGNED, INTEL) as i64;
    assert_eq!(v, -33);
}

#[test]
fn test_extract_intel_step_3_3() {
    let mut frame = [0u8; 8];
    frame[2] = 0x5E;
    let v = extract(&frame, 17, 7, UNSIGNED, INTEL);
    assert_eq!(v, 47);
}

#[test]
fn test_extract_intel_step_3_4() {
    let mut frame = [0u8; 8];
    frame[6] = 0x76;
    let v = extract(&frame, 48, 7, UNSIGNED, INTEL);
    assert_eq!(v, 118);
}

#[test]
fn test_extract_intel_step_3_5() {
    let mut frame = [0u8; 8];
    frame[4] = 0xD3;
    let v = extract(&frame, 32, 8, SIGNED, INTEL) as i64;
    assert_eq!(v, -45);
}

#[test]
fn test_extract_intel_step_4_1() {
    let mut frame = [0u8; 8];
    frame[3] = 0xFA;
    frame[4] = 0xD1;
    let v = extract(&frame, 24, 16, UNSIGNED, INTEL);
    assert_eq!(v, 53754);
}

#[test]
fn test_extract_intel_step_4_2() {
    let mut frame = [0u8; 8];
    frame[6] = 0x19;
    frame[7] = 0xFC;
    let v = extract(&frame, 48, 16, SIGNED, INTEL) as i64;
    assert_eq!(v, -999);
}

#[test]
fn test_extract_intel_step_4_3() {
    let mut frame = [0u8; 8];
    frame[0] = 0xEC;
    frame[1] = 0x34;
    let v = extract(&frame, 2, 12, UNSIGNED, INTEL);
    assert_eq!(v, 3387);
}

#[test]
fn test_extract_intel_step_4_4() {
    let mut frame = [0u8; 8];
    frame[2] = 0x75;
    frame[3] = 0x03;
    let v = extract(&frame, 16, 11, UNSIGNED, INTEL);
    assert_eq!(v, 885);
}

#[test]
fn test_extract_intel_step_4_5() {
    let mut frame = [0u8; 8];
    frame[5] = 0xF6;
    frame[6] = 0xE5;
    let v = extract(&frame, 40, 16, SIGNED, INTEL) as i64;
    assert_eq!(v, -6666);
}

#[test]
fn test_extract_intel_step_4_6() {
    let mut frame = [0u8; 8];
    frame[0] = 0xAB;
    frame[1] = 0xFF;
    frame[2] = 0xAB;
    frame[3] = 0xFF;
    frame[4] = 0xAB;
    frame[5] = 0xFF;
    frame[6] = 0xAB;
    let v = extract(&frame, 0, 56, UNSIGNED, INTEL);
    assert_eq!(v, 48413335211474859);
}

#[test]
fn test_extract_intel_step_4_7() {
    let mut frame = [0u8; 8];
    frame[0] = 0x96;
    frame[1] = 0x91;
    frame[2] = 0xE6;
    frame[3] = 0xFF;
    let v = extract(&frame, 0, 32, SIGNED, INTEL) as i64;
    assert_eq!(v, -1666666);
}

// ---------------------------------------------------------------------
// INSERT MOTOROLA tests (steps 5.1 - 6.7)
// ---------------------------------------------------------------------

#[test]
fn test_insert_motorola_step_5_1() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[3] = 0x06;
    insert(&mut frame, 31, 8, 6u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_5_2() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[3] = 0xF1;
    insert(&mut frame, 31, 8, (-15i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_5_3() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[0] = 0xFC;
    insert(&mut frame, 7, 6, 63u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_5_4() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[5] = 0x71;
    insert(&mut frame, 47, 8, 113u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_5_5() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[2] = 0x8F;
    insert(&mut frame, 23, 8, (-113i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_1() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[6] = 0x75;
    expected[7] = 0xAE;
    insert(&mut frame, 55, 16, 30126u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_2() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[4] = 0x17;
    expected[5] = 0x35;
    insert(&mut frame, 39, 16, (-59595i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_3() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[2] = 0x17;
    expected[3] = 0xA0;
    insert(&mut frame, 21, 9, 189u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_4() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[4] = 0x05;
    expected[5] = 0x6E;
    insert(&mut frame, 34, 11, 1390u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_5() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[0] = 0xA1;
    expected[1] = 0x4C;
    insert(&mut frame, 7, 16, (-24244i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_6() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[0] = 0xAB;
    expected[1] = 0xFF;
    expected[2] = 0xAB;
    expected[3] = 0xFF;
    expected[4] = 0xAB;
    expected[5] = 0xFF;
    expected[6] = 0xAB;
    insert(&mut frame, 7, 56, 48413335211474859u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_6_7() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[4] = 0xFF;
    expected[5] = 0xFF;
    expected[6] = 0xFE;
    expected[7] = 0x17;
    insert(&mut frame, 39, 32, (-489i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

// ---------------------------------------------------------------------
// INSERT INTEL tests (steps 7.1 - 8.7)
// ---------------------------------------------------------------------

#[test]
fn test_insert_intel_step_7_1() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[2] = 0xF0;
    insert(&mut frame, 16, 8, 240u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_7_2() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[4] = 0x36;
    insert(&mut frame, 32, 8, (-202i64) as u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_7_3() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[3] = 0xE0;
    insert(&mut frame, 29, 3, 7u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_7_4() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[7] = 0x17;
    insert(&mut frame, 56, 5, 23u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_7_5() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[5] = 0x39;
    insert(&mut frame, 40, 8, (-199i64) as u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_1() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[2] = 0x6D;
    expected[3] = 0xCB;
    insert(&mut frame, 16, 16, 52077u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_2() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[4] = 0xE6;
    expected[5] = 0x41;
    insert(&mut frame, 32, 16, (-48666i64) as u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_3() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[5] = 0xB0;
    expected[6] = 0x6A;
    insert(&mut frame, 44, 11, 1707u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_4() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[1] = 0xFF;
    expected[2] = 0x03;
    insert(&mut frame, 8, 10, 1023u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_5() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[6] = 0x53;
    expected[7] = 0x16;
    insert(&mut frame, 48, 16, (-59821i64) as u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_intel_step_8_6() {
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[0] = 0xAB;
    expected[1] = 0xFF;
    expected[2] = 0xAB;
    expected[3] = 0xFF;
    expected[4] = 0xAB;
    expected[5] = 0xFF;
    expected[6] = 0xAB;
    insert(&mut frame, 0, 56, 48413335211474859u64, INTEL);
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_insert_motorola_step_8_7() {
    // Note: the C test calls insert(...MOTOROLA) here despite header saying INTEL;
    // we replicate the C behavior exactly.
    let mut frame = [0u8; 8];
    let mut expected = [0u8; 8];
    expected[0] = 0xFF;
    expected[1] = 0xFF;
    expected[2] = 0xFA;
    expected[3] = 0xC5;
    insert(&mut frame, 7, 32, (-1339i64) as u64, MOTOROLA);
    assert!(frames_equal(&expected, &frame));
}

// ---------------------------------------------------------------------
// ENCODE / DECODE tests
// ---------------------------------------------------------------------

#[test]
fn test_encode_decode_double_motorola_positive_step_9_1() {
    let mut frame = [0u8; 8];
    let dphy: f64 = 66.66666;
    encode_double(&mut frame, dphy, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, dphy));
}

#[test]
fn test_encode_decode_double_motorola_negative_step_9_2() {
    let mut frame = [0u8; 8];
    let dphy: f64 = -50.6164129;
    encode_double(&mut frame, dphy, 7, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 7, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_double(decoded, dphy));
}

#[test]
fn test_encode_decode_uint64_motorola_step_9_3() {
    let mut frame = [0u8; 8];
    let uphy: u64 = 666666666;
    encode_uint64_t(&mut frame, uphy, 7, 32, MOTOROLA, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 7, 32, MOTOROLA, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_double_intel_positive_step_9_4() {
    let mut frame = [0u8; 8];
    let dphy: f64 = 8.4939123;
    encode_double(&mut frame, dphy, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, dphy));
}

#[test]
fn test_encode_decode_double_intel_negative_step_9_5() {
    let mut frame = [0u8; 8];
    let dphy: f64 = -7.7979897;
    encode_double(&mut frame, dphy, 0, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, dphy));
}

#[test]
fn test_encode_decode_uint64_intel_step_9_6() {
    let mut frame = [0u8; 8];
    let uphy: u64 = 999999999;
    encode_uint64_t(&mut frame, uphy, 0, 32, INTEL, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_int64_intel_negative_step_9_7() {
    let mut frame = [0u8; 8];
    let sphy: i64 = -1029384756;
    encode_int64_t(&mut frame, sphy, 0, 32, INTEL, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 0, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_float_motorola_step_9_8() {
    let mut frame = [0u8; 8];
    let fphy: f32 = -2938.345666;
    encode_float(&mut frame, fphy, 7, 40, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 7, 40, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, fphy));
}

#[test]
fn test_encode_decode_uint64_intel_fdframe_step_9_9() {
    let mut frame = [0u8; 40];
    let uphy: u64 = 999999999;
    encode_uint64_t(&mut frame, uphy, 288, 32, INTEL, 1.0, 0.0);
    let decoded = decode_uint64_t(&frame, 288, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, uphy);
}

#[test]
fn test_encode_decode_int64_motorola_fdframe_step_10_0() {
    let mut frame = [0u8; 56];
    let sphy: i64 = -7777;
    encode_int64_t(&mut frame, sphy, 431, 16, MOTOROLA, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 431, 16, MOTOROLA, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_int64_intel_fdframe_step_10_1() {
    let mut frame = [0u8; 48];
    let sphy: i64 = -1029384756;
    encode_int64_t(&mut frame, sphy, 184, 32, INTEL, 1.0, 0.0);
    let decoded = decode_int64_t(&frame, 184, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, sphy);
}

#[test]
fn test_encode_decode_float_motorola_fdframe_step_10_2() {
    let mut frame = [0u8; 64];
    let fphy: f32 = 8.49391;
    encode_float(&mut frame, fphy, 383, 32, MOTOROLA, 0.0000001, 0.0);
    let decoded = decode_float(&frame, 383, 32, MOTOROLA, 0.0000001, 0.0);
    assert!(cmp_float(decoded, fphy));
}

#[test]
fn test_encode_decode_double_intel_fdframe_step_10_3() {
    let mut frame = [0u8; 24];
    let dphy: f64 = -7.7979897;
    encode_double(&mut frame, dphy, 32, 32, INTEL, 0.0000001, 0.0);
    let decoded = decode_double(&frame, 32, 32, INTEL, 0.0000001, 0.0);
    assert!(cmp_double(decoded, dphy));
}

// ---------------------------------------------------------------------
// Additional edge-case tests (using values verified via extra_tests.c)
// ---------------------------------------------------------------------

#[test]
fn test_encode_int64_motorola_extra() {
    // gcc ground truth: encode_int64 motorola -555 sb=23 len=16: 00 00 fd d5 00 00 00 00
    let mut frame = [0u8; 8];
    encode_int64_t(&mut frame, -555, 23, 16, MOTOROLA, 1.0, 0.0);
    let expected = [0x00, 0x00, 0xfd, 0xd5, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_int64_t(&frame, 23, 16, MOTOROLA, 1.0, 0.0);
    assert_eq!(decoded, -555);
}

#[test]
fn test_encode_int64_intel_extra() {
    // gcc: encode_int64 intel -12345 sb=16 len=32: 00 00 c7 cf ff ff 00 00
    let mut frame = [0u8; 8];
    encode_int64_t(&mut frame, -12345, 16, 32, INTEL, 1.0, 0.0);
    let expected = [0x00, 0x00, 0xc7, 0xcf, 0xff, 0xff, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_int64_t(&frame, 16, 32, INTEL, 1.0, 0.0);
    assert_eq!(decoded, -12345);
}

#[test]
fn test_encode_uint64_with_factor() {
    // gcc: encode_uint64 intel 1000 factor=10 sb=0 len=16: 64 00 00 00 00 00 00 00
    let mut frame = [0u8; 8];
    encode_uint64_t(&mut frame, 1000, 0, 16, INTEL, 10.0, 0.0);
    let expected = [0x64, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_uint64_t(&frame, 0, 16, INTEL, 10.0, 0.0);
    assert_eq!(decoded, 1000);
}

#[test]
fn test_encode_float_with_factor() {
    // gcc: encode_float intel 12.5 sb=0 len=16 factor=0.1: 7d 00 00 00 00 00 00 00
    let mut frame = [0u8; 8];
    encode_float(&mut frame, 12.5_f32, 0, 16, INTEL, 0.1, 0.0);
    let expected = [0x7d, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_float(&frame, 0, 16, INTEL, 0.1, 0.0);
    assert!(cmp_float(decoded, 12.5_f32));
}

#[test]
fn test_extract_signed_len1_bit_set() {
    // gcc: extract len=1 signed bit set: -1
    let mut frame = [0u8; 8];
    frame[0] = 0x01;
    let v = extract(&frame, 0, 1, SIGNED, INTEL) as i64;
    assert_eq!(v, -1);
}

#[test]
fn test_extract_signed_min_byte_intel() {
    // gcc: extract intel sb=0 len=8 0x80 signed: -128
    let mut frame = [0u8; 8];
    frame[0] = 0x80;
    let v = extract(&frame, 0, 8, SIGNED, INTEL) as i64;
    assert_eq!(v, -128);
}

#[test]
fn test_insert_value_truncation() {
    // gcc: insert intel sb=0 len=8 val=0x1FF: ff 00 00 00 00 00 00 00
    let mut frame = [0u8; 8];
    insert(&mut frame, 0, 8, 0x1FFu64, INTEL);
    let expected = [0xff, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
}

#[test]
fn test_encode_decode_double_negative_extra() {
    // gcc: encode_double intel -3.14 sb=0 len=32 factor=0.001: bc f3 ff ff 00 00 00 00
    let mut frame = [0u8; 8];
    encode_double(&mut frame, -3.14_f64, 0, 32, INTEL, 0.001, 0.0);
    let expected = [0xbc, 0xf3, 0xff, 0xff, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_double(&frame, 0, 32, INTEL, 0.001, 0.0);
    assert!(cmp_double(decoded, -3.14_f64));
}

#[test]
fn test_encode_int64_min_value_8bit() {
    // gcc: encode_int64 intel -128 sb=0 len=8: 80 00 00 00 00 00 00 00
    let mut frame = [0u8; 8];
    encode_int64_t(&mut frame, -128, 0, 8, INTEL, 1.0, 0.0);
    let expected = [0x80, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_int64_t(&frame, 0, 8, INTEL, 1.0, 0.0);
    assert_eq!(decoded, -128);
}

#[test]
fn test_encode_uint64_with_offset() {
    // gcc: encode_uint64 with offset 50, value 200: 96 00 00 00 00 00 00 00
    // can_value = (200 - 50) / 1 = 150 = 0x96
    let mut frame = [0u8; 8];
    encode_uint64_t(&mut frame, 200, 0, 16, INTEL, 1.0, 50.0);
    let expected = [0x96, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00];
    assert!(frames_equal(&expected, &frame));
    let decoded = decode_uint64_t(&frame, 0, 16, INTEL, 1.0, 50.0);
    assert_eq!(decoded, 200);
}

fn main() {}
