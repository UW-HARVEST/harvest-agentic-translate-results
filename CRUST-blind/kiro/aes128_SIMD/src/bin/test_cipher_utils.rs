use aes128_SIMD::cipher_utils;

#[test]
fn test_g_mult_basic() {
    assert_eq!(cipher_utils::g_mult(0x02, 0x87), 0x15);
    assert_eq!(cipher_utils::g_mult(0x03, 0x6e), 0xB2);
    assert_eq!(cipher_utils::g_mult(0x0e, 0xdb), 0x6E);
    assert_eq!(cipher_utils::g_mult(0x0b, 0x13), 0xAD);
    assert_eq!(cipher_utils::g_mult(0x0d, 0x53), 0xAA);
    assert_eq!(cipher_utils::g_mult(0x09, 0x45), 0x5B);
}

#[test]
fn test_g_mult_boundary() {
    assert_eq!(cipher_utils::g_mult(0x00, 0xFF), 0x00);
    assert_eq!(cipher_utils::g_mult(0x01, 0x53), 0x53);
    assert_eq!(cipher_utils::g_mult(0xFF, 0xFF), 0x13);
    assert_eq!(cipher_utils::g_mult(0x00, 0x00), 0x00);
    assert_eq!(cipher_utils::g_mult(0x01, 0x01), 0x01);
}

#[test]
fn test_sub() {
    let mut state = [
        [0x00, 0x11, 0x22, 0x33],
        [0x44, 0x55, 0x66, 0x77],
        [0x88, 0x99, 0xAA, 0xBB],
        [0xCC, 0xDD, 0xEE, 0xFF],
    ];
    cipher_utils::sub(&mut state);
    assert_eq!(state, [
        [0x63, 0x82, 0x93, 0xC3],
        [0x1B, 0xFC, 0x33, 0xF5],
        [0xC4, 0xEE, 0xAC, 0xEA],
        [0x4B, 0xC1, 0x28, 0x16],
    ]);
}

#[test]
fn test_inv_sub_reverses_sub() {
    let original = [
        [0x00, 0x11, 0x22, 0x33],
        [0x44, 0x55, 0x66, 0x77],
        [0x88, 0x99, 0xAA, 0xBB],
        [0xCC, 0xDD, 0xEE, 0xFF],
    ];
    let mut state = original;
    cipher_utils::sub(&mut state);
    cipher_utils::inv_sub(&mut state);
    assert_eq!(state, original);
}

#[test]
fn test_sbox_rsbox_inverse() {
    for i in 0..256u16 {
        assert_eq!(cipher_utils::RSBOX[cipher_utils::SBOX[i as usize] as usize], i as u8);
    }
}

fn main() {}
