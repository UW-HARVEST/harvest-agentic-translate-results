use aes128_SIMD::aes;

#[test]
fn test_constants() {
    assert_eq!(aes::NB, 4);
    assert_eq!(aes::NR, 10);
    assert_eq!(aes::NK, 4);
    assert_eq!(aes::SBOX[0x00], 0x63);
    assert_eq!(aes::SBOX[0x53], 0xed);
    assert_eq!(aes::SBOX[0xff], 0x16);
    assert_eq!(aes::RSBOX[0x63], 0x00);
    assert_eq!(aes::RSBOX[0xed], 0x53);
    assert_eq!(aes::RSBOX[0x16], 0xff);
    assert_eq!(aes::RCON[0], 0x00);
    assert_eq!(aes::RCON[1], 0x01);
    assert_eq!(aes::RCON[10], 0x36);
}

#[test]
fn test_shift() {
    let mut state: [[u8; 4]; 4] = [
        [0x00, 0x01, 0x02, 0x03],
        [0x10, 0x11, 0x12, 0x13],
        [0x20, 0x21, 0x22, 0x23],
        [0x30, 0x31, 0x32, 0x33],
    ];
    aes::shift(&mut state);
    // Row 0 unchanged
    assert_eq!(state[0], [0x00, 0x01, 0x02, 0x03]);
    // Row 1 shifted left by 1
    assert_eq!(state[1], [0x11, 0x12, 0x13, 0x10]);
    // Row 2 shifted left by 2
    assert_eq!(state[2], [0x22, 0x23, 0x20, 0x21]);
    // Row 3 shifted left by 3
    assert_eq!(state[3], [0x33, 0x30, 0x31, 0x32]);
}

#[test]
fn test_inv_shift() {
    let mut state: [[u8; 4]; 4] = [
        [0x00, 0x01, 0x02, 0x03],
        [0x10, 0x11, 0x12, 0x13],
        [0x20, 0x21, 0x22, 0x23],
        [0x30, 0x31, 0x32, 0x33],
    ];
    aes::inv_shift(&mut state);
    assert_eq!(state[0], [0x00, 0x01, 0x02, 0x03]);
    // Row 1 shifted right by 1
    assert_eq!(state[1], [0x13, 0x10, 0x11, 0x12]);
    // Row 2 shifted right by 2
    assert_eq!(state[2], [0x22, 0x23, 0x20, 0x21]);
    // Row 3 shifted right by 3
    assert_eq!(state[3], [0x31, 0x32, 0x33, 0x30]);
}

#[test]
fn test_shift_then_inv_shift_roundtrip() {
    let original: [[u8; 4]; 4] = [
        [0xa1, 0xb2, 0xc3, 0xd4],
        [0xe5, 0xf6, 0x07, 0x18],
        [0x29, 0x3a, 0x4b, 0x5c],
        [0x6d, 0x7e, 0x8f, 0x90],
    ];
    let mut state = original;
    aes::shift(&mut state);
    aes::inv_shift(&mut state);
    assert_eq!(state, original);
}

fn main() {}
