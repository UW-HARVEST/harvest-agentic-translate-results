use aes128_SIMD::cipher_utils::{g_mult, sub, inv_sub};

#[test]
fn test_g_mult_known_vectors() {
    // Values verified against the C oracle.
    assert_eq!(g_mult(0x57, 0x13), 0xFE);
    assert_eq!(g_mult(0x02, 0x80), 0x1B);
    assert_eq!(g_mult(0x03, 0xff), 0x1A);
    assert_eq!(g_mult(0x00, 0xff), 0x00);
    assert_eq!(g_mult(0xff, 0x00), 0x00);
    assert_eq!(g_mult(0x01, 0xab), 0xAB);
    assert_eq!(g_mult(0x0e, 0x0b), 0x62);
    assert_eq!(g_mult(0x53, 0xca), 0x01);
    assert_eq!(g_mult(0x10, 0x55), 0x27);
}

#[test]
fn test_g_mult_identity() {
    // Multiplication by 1 returns the other operand
    for v in 0u8..=255 {
        assert_eq!(g_mult(1, v), v);
        assert_eq!(g_mult(v, 1), v);
    }
}

#[test]
fn test_g_mult_zero() {
    for v in 0u8..=255 {
        assert_eq!(g_mult(0, v), 0);
        assert_eq!(g_mult(v, 0), 0);
    }
}

#[test]
fn test_g_mult_commutative() {
    // GF(2^8) multiplication is commutative
    let samples: [u8; 8] = [0x02, 0x03, 0x09, 0x0b, 0x0d, 0x0e, 0x57, 0xff];
    for &a in &samples {
        for &b in &samples {
            assert_eq!(g_mult(a, b), g_mult(b, a));
        }
    }
}

#[test]
fn test_sub() {
    let mut state: [[u8; 4]; 4] = [
        [0x00, 0x01, 0x02, 0x03],
        [0x10, 0x11, 0x12, 0x13],
        [0xfe, 0xff, 0x80, 0x7f],
        [0x55, 0xaa, 0x33, 0xcc],
    ];
    sub(&mut state);
    let expected: [[u8; 4]; 4] = [
        [0x63, 0x7C, 0x77, 0x7B],
        [0xCA, 0x82, 0xC9, 0x7D],
        [0xBB, 0x16, 0xCD, 0xD2],
        [0xFC, 0xAC, 0xC3, 0x4B],
    ];
    assert_eq!(state, expected);
}

#[test]
fn test_inv_sub() {
    let mut state: [[u8; 4]; 4] = [
        [0x63, 0x7c, 0x77, 0x7b],
        [0xca, 0x82, 0xc9, 0x7d],
        [0xb7, 0xfd, 0x93, 0x26],
        [0x52, 0x09, 0x6a, 0xd5],
    ];
    inv_sub(&mut state);
    let expected: [[u8; 4]; 4] = [
        [0x00, 0x01, 0x02, 0x03],
        [0x10, 0x11, 0x12, 0x13],
        [0x20, 0x21, 0x22, 0x23],
        [0x48, 0x40, 0x58, 0xB5],
    ];
    assert_eq!(state, expected);
}

#[test]
fn test_sub_inv_sub_roundtrip() {
    let original: [[u8; 4]; 4] = [
        [0x12, 0x34, 0x56, 0x78],
        [0x9a, 0xbc, 0xde, 0xf0],
        [0x01, 0x23, 0x45, 0x67],
        [0x89, 0xab, 0xcd, 0xef],
    ];
    let mut state = original;
    sub(&mut state);
    inv_sub(&mut state);
    assert_eq!(state, original);
}

fn main() {}
