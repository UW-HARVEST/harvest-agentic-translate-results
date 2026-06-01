use libm17::math::*;

#[test]
fn test_golay24_encode() {
    assert_eq!(golay24_encode(0), 0);
    assert_eq!(golay24_encode(1), 6379);
    assert_eq!(golay24_encode(0xFFF), 16777215);
    assert_eq!(golay24_encode(0x123), 1192108);
    assert_eq!(golay24_encode(0xABC), 11256380);
}

#[test]
fn test_q_abs_diff() {
    assert_eq!(q_abs_diff(10, 3), 7);
    assert_eq!(q_abs_diff(3, 10), 7);
    assert_eq!(q_abs_diff(0, 0), 0);
    assert_eq!(q_abs_diff(65535, 0), 65535);
}

#[test]
fn test_eucl_norm() {
    let a: [f32; 3] = [1.0, 2.0, 3.0];
    let b: [i8; 3] = [0, 0, 0];
    let r = eucl_norm(&a, &b, 3);
    let expected: f32 = (1.0_f32 + 4.0 + 9.0).sqrt();
    assert!((r - expected).abs() < 1e-5);
    // 14^0.5 ~= 3.741657
    assert!((r - 3.741657).abs() < 1e-5);
}

#[test]
fn test_int_to_soft_and_back() {
    let mut soft: [u16; 12] = [0; 12];
    int_to_soft(&mut soft, 0xA5C, 12);
    let expected: [u16; 12] = [
        0x0000, 0x0000, 0xFFFF, 0xFFFF, 0xFFFF, 0x0000, 0xFFFF, 0x0000, 0x0000, 0xFFFF, 0x0000,
        0xFFFF,
    ];
    assert_eq!(soft, expected);
    assert_eq!(soft_to_int(&soft, 12), 0xA5C);
}

#[test]
fn test_soft_to_int() {
    let s: [u16; 16] = [
        0xFFFF, 0, 0xFFFF, 0, 0x8000, 0x7FFF, 0xFFFF, 0, 0, 0xFFFF, 0xFFFF, 0xFFFF, 0, 0, 0xFFFF,
        0xFFFF,
    ];
    assert_eq!(soft_to_int(&s, 16), 52821);
    assert_eq!(soft_to_int(&s, 8), 85);
}

#[test]
fn test_add_sub_div_mul16() {
    assert_eq!(add16(10, 20), 30);
    assert_eq!(add16(0xFFFF, 1), 0xFFFF); // saturation
    assert_eq!(sub16(10, 20), 0);
    assert_eq!(sub16(20, 10), 10);
    assert_eq!(div16(0xFFFF, 2), 0xFFFF); // saturated since (0xFFFF<<16)/2 > 0xFFFF
    assert_eq!(div16(0x1000, 0x10), 0xFFFF);
    assert_eq!(mul16(0xFFFF, 0xFFFF), 65534);
    assert_eq!(mul16(0x8000, 0x8000), 16384);
}

#[test]
fn test_soft_bit_xor() {
    assert_eq!(soft_bit_xor(0, 0), 0);
    assert_eq!(soft_bit_xor(0xFFFF, 0), 65534);
    assert_eq!(soft_bit_xor(0xFFFF, 0xFFFF), 0);
    assert_eq!(soft_bit_xor(0x7FFF, 0x7FFF), 32766);
    assert_eq!(soft_bit_xor(0xFFFF, 0x7FFF), 32767);
}

#[test]
fn test_soft_bit_not() {
    assert_eq!(soft_bit_not(0xFFFF), 0);
    assert_eq!(soft_bit_not(0), 65535);
    assert_eq!(soft_bit_not(0x1234), 60875);
}

#[test]
fn test_soft_xor() {
    let a: [u16; 4] = [0xFFFF, 0, 0xFFFF, 0xFFFF];
    let b: [u16; 4] = [0xFFFF, 0xFFFF, 0, 0xFFFF];
    let mut out: [u16; 4] = [0; 4];
    soft_xor(&mut out, &a, &b, 4);
    assert_eq!(out[0], soft_bit_xor(0xFFFF, 0xFFFF));
    assert_eq!(out[1], soft_bit_xor(0, 0xFFFF));
    assert_eq!(out[2], soft_bit_xor(0xFFFF, 0));
    assert_eq!(out[3], soft_bit_xor(0xFFFF, 0xFFFF));
}

#[test]
fn test_s_popcount() {
    let v: [u16; 4] = [10, 20, 30, 40];
    assert_eq!(s_popcount(&v, 4), 100);
    let v: [u16; 4] = [0xFFFF, 0xFFFF, 0xFFFF, 0xFFFF];
    assert_eq!(s_popcount(&v, 4), 4 * 0xFFFF);
}

#[test]
#[allow(non_snake_case)]
fn test_encode_LICH() {
    let inp: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    let out = encode_LICH(&inp);
    let expected: [u8; 12] = [0x12, 0x30, 0xAC, 0x45, 0x6B, 0x6C, 0x78, 0x98, 0x10, 0xAB, 0xC2, 0x3C];
    assert_eq!(out, expected);
}

#[test]
fn test_golay24_sdecode() {
    // 0xABC encoded
    let encoded = golay24_encode(0xABC);
    let mut cw: [u16; 24] = [0; 24];
    for i in 0..24 {
        cw[i] = if (encoded >> (23 - i)) & 1 != 0 { 0xFFFF } else { 0x0000 };
    }
    assert_eq!(golay24_sdecode(&cw), 2748);

    // 0x123
    let encoded = golay24_encode(0x123);
    for i in 0..24 {
        cw[i] = if (encoded >> (23 - i)) & 1 != 0 { 0xFFFF } else { 0x0000 };
    }
    assert_eq!(golay24_sdecode(&cw), 291);
}

#[test]
#[allow(non_snake_case)]
fn test_decode_LICH() {
    // encode and decode round-trip via encode_LICH+manual unpack
    let inp: [u8; 6] = [0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC];
    let enc = encode_LICH(&inp);
    // unpack into 96 bit array, then convert each bit to soft 16-bit
    let mut soft: [u16; 96] = [0; 96];
    for i in 0..12 {
        for j in 0..8 {
            let bit = (enc[i] >> (7 - j)) & 1;
            soft[i * 8 + j] = if bit == 1 { 0xFFFF } else { 0x0000 };
        }
    }
    let outp: [u8; 6] = [0; 6];
    decode_LICH(&outp, soft);
    assert_eq!(outp, inp);
}

#[test]
fn test_constants() {
    assert_eq!(ENCODE_MATRIX[0], 0x8eb);
    assert_eq!(ENCODE_MATRIX[11], 0xc75);
    assert_eq!(DECODE_MATRIX[0], 0xc75);
    assert_eq!(DECODE_MATRIX[11], 0xa4f);
    assert_eq!(RRC_TAPS_10.len(), 81);
    assert_eq!(RRC_TAPS_5.len(), 41);
}

#[test]
fn test_s_calc_checksum() {
    // For value of all zeros, checksum is all zeros
    let value: [u16; 12] = [0; 12];
    let mut out: [u16; 12] = [0xAAAA; 12];
    s_calc_checksum(&mut out, &value);
    assert_eq!(out, [0u16; 12]);

    // For value where every entry is 0xFFFF, the checksum equals XOR-of-all-encode-matrix-entries
    // computed bit-by-bit in soft logic. We can't easily predict, but we can verify
    // the function ran and gave the expected values via integer interpretation:
    let value: [u16; 12] = [0xFFFF; 12];
    let mut out: [u16; 12] = [0; 12];
    s_calc_checksum(&mut out, &value);
    // The integer XOR of all encode matrix entries
    let mut x: u16 = 0;
    for i in 0..12 {
        x ^= ENCODE_MATRIX[i];
    }
    assert_eq!(soft_to_int(&out, 12), x);
}

#[test]
fn test_s_detect_errors() {
    // perfect codeword should yield 0 errors
    let cw_int = golay24_encode(0xABC);
    let mut cw: [u16; 24] = [0; 24];
    // codeword bits in C: cw is reversed inside golay24_sdecode as cw[i] = codeword[23-i]
    // s_detect_errors expects parity at [0..12], data at [12..24] in its own ordering.
    // Since golay24_encode outputs (data<<12)|checksum, in big-endian bits of a 24-bit number,
    // bit 23 == top bit of data, bit 0 == top bit of checksum.
    // Building the codeword like decode_LICH does: bit i (MSB-first).
    for i in 0..24 {
        cw[i] = if (cw_int >> (23 - i)) & 1 != 0 { 0xFFFF } else { 0x0000 };
    }
    // After internal reversal in golay24_sdecode, cw becomes lsb-first.
    // s_detect_errors is called with the reversed cw, which is what we should
    // also pass directly.
    let mut cw_rev: [u16; 24] = [0; 24];
    for i in 0..24 {
        cw_rev[i] = cw[23 - i];
    }
    let errors = s_detect_errors(&cw_rev);
    assert_eq!(errors, 0); // no errors detected for valid codeword
}

fn main() {}
