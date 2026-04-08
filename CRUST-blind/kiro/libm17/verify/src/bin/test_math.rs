use libm17::math;

#[test]
fn test_golay24_encode() {
    assert_eq!(math::golay24_encode(0x0D78), 14125071);
    assert_eq!(math::golay24_encode(0x0000), 0);
    assert_eq!(math::golay24_encode(0x0001), 6379);
    assert_eq!(math::golay24_encode(0x0FFF), 16777215);
    assert_eq!(math::golay24_encode(0x0ABC), 11256380);
    assert_eq!(math::golay24_encode(0x0800), 8391797);
    assert_eq!(math::golay24_encode(0x0400), 4195899);
    assert_eq!(math::golay24_encode(0x0002), 10558);
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
fn test_q_abs_diff() {
    assert_eq!(math::q_abs_diff(100, 200), 100);
    assert_eq!(math::q_abs_diff(200, 100), 100);
    assert_eq!(math::q_abs_diff(0, 0), 0);
    assert_eq!(math::q_abs_diff(0xFFFF, 0), 65535);
}

#[test]
fn test_eucl_norm() {
    let in1 = [1.0f32, 2.0, 3.0];
    let in2 = [4i8, 5, 6];
    let result = math::eucl_norm(&in1, &in2, 3);
    assert!((result - 5.196152).abs() < 0.001);

    let in1b = [0.0f32, 0.0];
    let in2b = [3i8, 4];
    let result2 = math::eucl_norm(&in1b, &in2b, 2);
    assert!((result2 - 5.0).abs() < 0.001);
}

#[test]
fn test_int_to_soft_and_soft_to_int() {
    let mut out = [0u16; 16];
    math::int_to_soft(&mut out, 0xA5, 8);
    assert_eq!(out[0], 0xFFFF);
    assert_eq!(out[1], 0);
    assert_eq!(out[2], 0xFFFF);
    assert_eq!(out[3], 0);
    assert_eq!(out[4], 0);
    assert_eq!(out[5], 0xFFFF);
    assert_eq!(out[6], 0);
    assert_eq!(out[7], 0xFFFF);
    assert_eq!(math::soft_to_int(&out, 8), 165);

    let mut zeros = [0u16; 16];
    math::int_to_soft(&mut zeros, 0x0000, 16);
    assert_eq!(math::soft_to_int(&zeros, 16), 0);

    let mut ones = [0u16; 16];
    math::int_to_soft(&mut ones, 0xFFFF, 16);
    assert_eq!(math::soft_to_int(&ones, 16), 65535);
}

#[test]
fn test_add16() {
    assert_eq!(math::add16(0x8000, 0x8000), 0xFFFF);
    assert_eq!(math::add16(0xFFFF, 1), 0xFFFF);
    assert_eq!(math::add16(0, 0), 0);
}

#[test]
fn test_sub16() {
    assert_eq!(math::sub16(0x8000, 0x4000), 0x4000);
    assert_eq!(math::sub16(0x4000, 0x8000), 0);
    assert_eq!(math::sub16(0, 0), 0);
}

#[test]
fn test_mul16() {
    assert_eq!(math::mul16(0x8000, 0x8000), 16384);
    assert_eq!(math::mul16(0xFFFF, 0xFFFF), 65534);
    assert_eq!(math::mul16(0, 0xFFFF), 0);
}

#[test]
fn test_div16() {
    assert_eq!(math::div16(0x8000, 0x8000), 0xFFFF);
    assert_eq!(math::div16(0x4000, 0x8000), 0x8000);
    assert_eq!(math::div16(1, 1), 0xFFFF);
    assert_eq!(math::div16(0xFFFF, 1), 0xFFFF);
    assert_eq!(math::div16(1, 0xFFFF), 1);
}

#[test]
fn test_soft_bit_xor() {
    assert_eq!(math::soft_bit_xor(0, 0), 0);
    assert_eq!(math::soft_bit_xor(0xFFFF, 0), 65534);
    assert_eq!(math::soft_bit_xor(0, 0xFFFF), 65534);
    assert_eq!(math::soft_bit_xor(0xFFFF, 0xFFFF), 0);
    assert_eq!(math::soft_bit_xor(0x7FFF, 0x7FFF), 32766);
    assert_eq!(math::soft_bit_xor(0x7FFF, 0), 32766);
    assert_eq!(math::soft_bit_xor(0x7FFF, 0xFFFF), 32767);
}

#[test]
fn test_soft_bit_not() {
    assert_eq!(math::soft_bit_not(0), 0xFFFF);
    assert_eq!(math::soft_bit_not(0xFFFF), 0);
    assert_eq!(math::soft_bit_not(0x7FFF), 0x8000);
}

#[test]
fn test_soft_xor() {
    let a = [0x0000u16, 0xFFFF, 0x7FFF, 0x8000];
    let b = [0xFFFFu16, 0x0000, 0x7FFF, 0x8000];
    let mut out = [0u16; 4];
    math::soft_xor(&mut out, &a, &b, 4);
    assert_eq!(out[0], 65534);
    assert_eq!(out[1], 65534);
    assert_eq!(out[2], 32766);
    assert_eq!(out[3], 32766);
}

#[test]
fn test_s_popcount() {
    let arr = [100u16, 200, 300];
    assert_eq!(math::s_popcount(&arr, 3), 600);
}

#[test]
fn test_encode_lich() {
    let inp: [u8; 6] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
    let outp = math::encode_LICH(&inp);
    assert_eq!(outp, [0xAB, 0xC2, 0x3C, 0xDE, 0xF3, 0xF0, 0x12, 0x30, 0xAC, 0x45, 0x6B, 0x6C]);
}

#[test]
fn test_decode_lich_roundtrip() {
    let inp: [u8; 6] = [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56];
    let encoded = math::encode_LICH(&inp);

    let mut soft_bits = [0u16; 96];
    for i in 0..12 {
        for j in 0..8 {
            soft_bits[i * 8 + j] = if (encoded[i] >> (7 - j)) & 1 != 0 { 0xFFFF } else { 0x0000 };
        }
    }

    let decoded = [0u8; 6];
    math::decode_LICH(&decoded, soft_bits);
    assert_eq!(decoded, [0xAB, 0xCD, 0xEF, 0x12, 0x34, 0x56]);
}

fn main() {}
