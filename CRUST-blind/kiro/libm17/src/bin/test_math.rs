use libm17::math;

#[test]
fn test_q_abs_diff() {
    assert_eq!(math::q_abs_diff(100, 200), 100);
    assert_eq!(math::q_abs_diff(200, 100), 100);
    assert_eq!(math::q_abs_diff(0, 0), 0);
    assert_eq!(math::q_abs_diff(0xFFFF, 0), 0xFFFF);
    assert_eq!(math::q_abs_diff(0, 0xFFFF), 0xFFFF);
}

#[test]
fn test_eucl_norm() {
    let f1 = [1.0f32, 2.0, 3.0];
    let i2 = [1i8, 2, 3];
    assert!((math::eucl_norm(&f1, &i2, 3) - 0.0).abs() < 1e-5);
    let f2 = [3.0f32, 0.0];
    let i3 = [0i8, 0];
    assert!((math::eucl_norm(&f2, &i3, 2) - 3.0).abs() < 1e-5);
}

#[test]
fn test_int_to_soft_and_soft_to_int() {
    let mut soft = [0u16; 16];
    math::int_to_soft(&mut soft, 0xA5, 8);
    assert_eq!(soft[0], 0xFFFF); // bit 0 = 1
    assert_eq!(soft[1], 0x0000); // bit 1 = 0
    assert_eq!(soft[2], 0xFFFF); // bit 2 = 1
    assert_eq!(soft[3], 0x0000);
    assert_eq!(soft[4], 0x0000);
    assert_eq!(soft[5], 0xFFFF);
    assert_eq!(soft[6], 0x0000);
    assert_eq!(soft[7], 0xFFFF);
    assert_eq!(math::soft_to_int(&soft, 8), 0xA5);

    math::int_to_soft(&mut soft, 0, 8);
    assert_eq!(math::soft_to_int(&soft, 8), 0);

    math::int_to_soft(&mut soft, 0xFFFF, 16);
    assert_eq!(math::soft_to_int(&soft, 16), 0xFFFF);
}

#[test]
fn test_add16() {
    assert_eq!(math::add16(0x8000, 0x8000), 0xFFFF); // saturated
    assert_eq!(math::add16(0xFFFF, 1), 0xFFFF);       // saturated
    assert_eq!(math::add16(0, 0), 0);
    assert_eq!(math::add16(100, 200), 300);
}

#[test]
fn test_sub16() {
    assert_eq!(math::sub16(0x8000, 0x4000), 0x4000);
    assert_eq!(math::sub16(0x4000, 0x8000), 0); // saturated at 0
    assert_eq!(math::sub16(0, 0), 0);
}

#[test]
fn test_mul16() {
    assert_eq!(math::mul16(0xFFFF, 0xFFFF), 0xFFFE);
    assert_eq!(math::mul16(0x8000, 0x8000), 0x4000);
    assert_eq!(math::mul16(0, 0xFFFF), 0);
}

#[test]
fn test_div16() {
    assert_eq!(math::div16(0x8000, 0xFFFF), 0x8000);
    assert_eq!(math::div16(0xFFFF, 0x8000), 0xFFFF); // saturated
    assert_eq!(math::div16(1, 0xFFFF), 1);
}

#[test]
fn test_soft_bit_xor() {
    assert_eq!(math::soft_bit_xor(0, 0), 0);
    assert_eq!(math::soft_bit_xor(0xFFFF, 0xFFFF), 0);
    assert_eq!(math::soft_bit_xor(0xFFFF, 0), 0xFFFE);
    assert_eq!(math::soft_bit_xor(0, 0xFFFF), 0xFFFE);
    assert_eq!(math::soft_bit_xor(0x7FFF, 0x7FFF), 0x7FFE);
}

#[test]
fn test_soft_bit_not() {
    assert_eq!(math::soft_bit_not(0), 0xFFFF);
    assert_eq!(math::soft_bit_not(0xFFFF), 0);
    assert_eq!(math::soft_bit_not(0x7FFF), 0x8000);
}

#[test]
fn test_soft_xor() {
    let a = [0u16, 0xFFFF, 0x8000];
    let b = [0xFFFFu16, 0xFFFF, 0x8000];
    let mut out = [0u16; 3];
    math::soft_xor(&mut out, &a, &b, 3);
    assert_eq!(out[0], 0xFFFE);
    assert_eq!(out[1], 0x0000);
    assert_eq!(out[2], 0x7FFE);
}

#[test]
fn test_golay24_encode() {
    assert_eq!(math::golay24_encode(0), 0x000000);
    assert_eq!(math::golay24_encode(1), 0x0018EB);
    assert_eq!(math::golay24_encode(0xFFF), 0xFFFFFF);
    assert_eq!(math::golay24_encode(0x0D78), 0x0D7880F);
}

#[test]
fn test_golay24_sdecode_clean() {
    let cw: u32 = 0x0D7880F;
    let mut vector = [0u16; 24];
    for i in 0..24 {
        vector[23 - i] = if (cw >> i) & 1 != 0 { 0xFFFF } else { 0 };
    }
    assert_eq!(math::golay24_sdecode(&vector), 0x0D78);
}

#[test]
fn test_encode_lich() {
    let inp: [u8; 6] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
    let outp = math::encode_LICH(&inp);
    assert_eq!(outp, [0xAB, 0xC2, 0x3C, 0xDE, 0xF3, 0xF0, 0x12, 0x30, 0xAC, 0x45, 0x6B, 0x6C]);
}

#[test]
fn test_s_popcount() {
    assert_eq!(math::s_popcount(&[0xFFFF, 0xFFFF, 0xFFFF], 3), 196605);
    assert_eq!(math::s_popcount(&[0, 0, 0], 3), 0);
    assert_eq!(math::s_popcount(&[0x8000, 0x4000], 2), 0xC000);
}

#[test]
fn test_s_calc_checksum() {
    let val: [u16; 12] = [0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0, 0xFFFF, 0];
    let mut out = [0u16; 12];
    math::s_calc_checksum(&mut out, &val);
    let expected: [u16; 12] = [0xFFFE, 0x0002, 0xFFFD, 0xFFFD, 0x0001, 0x0000, 0x0000, 0x0000, 0xFFFD, 0x0000, 0xFFFE, 0xFFFC];
    assert_eq!(out, expected);
}

fn main() {}
