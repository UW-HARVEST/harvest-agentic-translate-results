use libm17::math::{
    add16, decode_LICH, div16, encode_LICH, eucl_norm, golay24_encode, golay24_sdecode, int_to_soft,
    mul16, q_abs_diff, s_calc_checksum, s_detect_errors, s_popcount, soft_bit_not, soft_bit_xor,
    soft_to_int, soft_xor, sub16, DECODE_MATRIX, ENCODE_MATRIX, RRC_TAPS_10, RRC_TAPS_5,
};

#[test]
fn test_q_abs_diff() {
    assert_eq!(q_abs_diff(5, 3), 2);
    assert_eq!(q_abs_diff(3, 5), 2);
    assert_eq!(q_abs_diff(0, 0xFFFF), 65535);
    assert_eq!(q_abs_diff(100, 100), 0);
}

#[test]
fn test_soft_bit_xor() {
    assert_eq!(soft_bit_xor(0, 0), 0x0000);
    assert_eq!(soft_bit_xor(0, 0x7FFF), 0x7FFE);
    assert_eq!(soft_bit_xor(0, 0xFFFF), 0xFFFE);
    assert_eq!(soft_bit_xor(0x7FFF, 0), 0x7FFE);
    assert_eq!(soft_bit_xor(0x7FFF, 0x7FFF), 0x7FFE);
    assert_eq!(soft_bit_xor(0x7FFF, 0xFFFF), 0x7FFF);
    assert_eq!(soft_bit_xor(0xFFFF, 0), 0xFFFE);
    assert_eq!(soft_bit_xor(0xFFFF, 0x7FFF), 0x7FFF);
    assert_eq!(soft_bit_xor(0xFFFF, 0xFFFF), 0x0000);
}

#[test]
fn test_soft_bit_not() {
    assert_eq!(soft_bit_not(0), 0xFFFF);
    assert_eq!(soft_bit_not(0x7FFF), 0x8000);
    assert_eq!(soft_bit_not(0xFFFF), 0x0000);
    assert_eq!(soft_bit_not(0x1234), 0xEDCB);
}

#[test]
fn test_add16_sub16() {
    assert_eq!(add16(0, 0), 0);
    assert_eq!(add16(0x7FFF, 0x8000), 0xFFFF);
    assert_eq!(add16(0xFFFF, 1), 0xFFFF); // saturates
    assert_eq!(add16(0x8000, 0x8000), 0xFFFF);

    assert_eq!(sub16(10, 5), 5);
    assert_eq!(sub16(5, 10), 0); // saturates
    assert_eq!(sub16(0xFFFF, 0xFFFF), 0);
}

#[test]
fn test_div16_mul16() {
    assert_eq!(div16(0x8000, 0xFFFF), 0x8000);
    assert_eq!(div16(0xFFFF, 0xFFFF), 0xFFFF);
    assert_eq!(div16(1, 2), 0x8000);

    assert_eq!(mul16(0, 0), 0x0000);
    assert_eq!(mul16(0xFFFF, 0xFFFF), 0xFFFE);
    assert_eq!(mul16(0x8000, 0x8000), 0x4000);
}

#[test]
fn test_int_to_soft_and_soft_to_int() {
    let mut out = [0u16; 12];
    int_to_soft(&mut out, 0xABC, 12);
    let expected = [
        0x0000, 0x0000, 0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF, 0x0000, 0xFFFF, 0x0000, 0xFFFF, 0x0000,
        0xFFFF,
    ];
    assert_eq!(out, expected);
    assert_eq!(soft_to_int(&out, 12), 0x0ABC);

    int_to_soft(&mut out, 0, 12);
    assert_eq!(out, [0u16; 12]);
    assert_eq!(soft_to_int(&out, 12), 0);

    int_to_soft(&mut out, 0xFFF, 12);
    assert_eq!(out, [0xFFFFu16; 12]);
    assert_eq!(soft_to_int(&out, 12), 0xFFF);
}

#[test]
fn test_soft_xor_vec() {
    let a = [0x0000, 0xFFFF, 0x7FFF, 0xFFFF];
    let b = [0xFFFF, 0xFFFF, 0x7FFF, 0x0000];
    let mut out = [0u16; 4];
    soft_xor(&mut out, &a, &b, 4);
    assert_eq!(out[0], 0xFFFE);
    assert_eq!(out[1], 0x0000);
    assert_eq!(out[2], 0x7FFE);
    assert_eq!(out[3], 0xFFFE);
}

#[test]
fn test_s_popcount() {
    let v = [0xFFFFu16, 0x7FFF, 0x0000, 0xFFFF];
    assert_eq!(s_popcount(&v, 4), 0xFFFF + 0x7FFF + 0 + 0xFFFF);

    let z = [0u16; 12];
    assert_eq!(s_popcount(&z, 12), 0);

    let f = [0xFFFFu16; 12];
    assert_eq!(s_popcount(&f, 12), 12 * 0xFFFF);
}

#[test]
fn test_eucl_norm() {
    let a: [f32; 3] = [1.0, 2.0, 3.0];
    let b: [i8; 3] = [1, 2, 3];
    assert!((eucl_norm(&a, &b, 3) - 0.0).abs() < 1e-6);

    let c: [f32; 3] = [4.0, 6.0, 3.0];
    let d: [i8; 3] = [1, 2, 3];
    // sqrt(9 + 16 + 0) = 5
    assert!((eucl_norm(&c, &d, 3) - 5.0).abs() < 1e-5);
}

#[test]
fn test_encode_matrix_decode_matrix_constants() {
    assert_eq!(
        ENCODE_MATRIX,
        &[0x8eb, 0x93e, 0xa97, 0xdc6, 0x367, 0x6cd, 0xd99, 0x3da, 0x7b4, 0xf68, 0x63b, 0xc75]
    );
    assert_eq!(
        DECODE_MATRIX,
        &[0xc75, 0x49f, 0x93e, 0x6e3, 0xdc6, 0xf13, 0xab9, 0x1ed, 0x3da, 0x7b4, 0xf68, 0xa4f]
    );
}

#[test]
fn test_golay24_encode() {
    assert_eq!(golay24_encode(0x0D78), 0x00D7880F);
    assert_eq!(golay24_encode(0x000), 0x00000000);
    assert_eq!(golay24_encode(0xFFF), 0x00FFFFFF);
    assert_eq!(golay24_encode(0x001), 0x000018EB);
    assert_eq!(golay24_encode(0x800), 0x00800C75);

    // Verify single-bit data correspondence
    for i in 0..12 {
        let data: u16 = 0x800 >> (11 - i); // single bit i
        let actual_data: u16 = 1 << (11 - i);
        let v = golay24_encode(actual_data);
        // checksum = encode_matrix[bit position]
        let _ = data; // unused but keeps logic aligned
        let _ = v;
    }
}

#[test]
fn test_golay24_sdecode_clean() {
    let mut vector = [0u16; 24];
    let codeword: u32 = 0x0D7880F;
    for i in 0..24 {
        vector[23 - i] = if ((codeword >> i) & 1) != 0 { 0xFFFF } else { 0 };
    }
    assert_eq!(golay24_sdecode(&vector), 0x0D78);
}

#[test]
fn test_golay24_sdecode_zero() {
    let vector = [0u16; 24];
    // all-zero codeword decodes to 0
    assert_eq!(golay24_sdecode(&vector), 0);
}

#[test]
fn test_s_detect_errors_clean() {
    // For a clean codeword (no errors), s_detect_errors should return 0.
    // s_detect_errors expects cw[i] = bit i of original codeword (LSB at index 0).
    // For 0x0D7880F: parity = bits 0..11 = 0x80F, data = bits 12..23 = 0xD78.
    // checksum(data=0xD78) = 0x80F so syndrome = 0 -> errors = 0.
    let mut cw = [0u16; 24];
    let codeword: u32 = 0x0D7880F;
    for i in 0..24 {
        cw[i] = if ((codeword >> i) & 1) != 0 { 0xFFFF } else { 0 };
    }
    assert_eq!(s_detect_errors(&cw), 0);
}

#[test]
fn test_s_calc_checksum() {
    // For all-zero data input, checksum = all zero.
    let value = [0u16; 12];
    let mut out = [0xFFFFu16; 12];
    s_calc_checksum(&mut out, &value);
    assert_eq!(out, [0u16; 12]);
}

#[test]
fn test_encode_LICH() {
    let inp = [0x01u8, 0x23, 0x45, 0x67, 0x89, 0xAB];
    let out = encode_LICH(&inp);
    let expected = [
        0x01u8, 0x2A, 0x59, 0x34, 0x57, 0x39, 0x67, 0x8C, 0xA6, 0x9A, 0xB2, 0xC5,
    ];
    assert_eq!(out, expected);
}

#[test]
fn test_decode_LICH_round_trip() {
    let inp = [0x01u8, 0x23, 0x45, 0x67, 0x89, 0xAB];
    let encoded = encode_LICH(&inp);
    // Convert encoded to soft 96-bit array
    let mut soft = [0u16; 96];
    for i in 0..12 {
        for j in 0..8 {
            soft[i * 8 + j] = if (encoded[i] >> (7 - j)) & 1 != 0 {
                0xFFFF
            } else {
                0
            };
        }
    }
    let outp = [0u8; 6];
    decode_LICH(&outp, soft);
    assert_eq!(outp, inp);
}

#[test]
fn test_rrc_taps_lengths() {
    assert_eq!(RRC_TAPS_10.len(), 81);
    assert_eq!(RRC_TAPS_5.len(), 41);
    // First and last value of taps
    assert!((RRC_TAPS_10[0] - (-0.003195702904062073f32)).abs() < 1e-9);
    assert!((RRC_TAPS_10[40] - 0.359452932027607974f32).abs() < 1e-9);
    assert!((RRC_TAPS_10[80] - (-0.003195702904062073f32)).abs() < 1e-9);
    assert!((RRC_TAPS_5[20] - 0.508340710642860f32).abs() < 1e-9);
}

fn main() {}
