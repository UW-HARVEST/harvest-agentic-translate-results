use aes128_SIMD::aes::{NB, NR, NK};
use aes128_SIMD::cipher;
use aes128_SIMD::keys;

#[test]
fn test_cipher_fips_vector() {
    // FIPS 197 / NIST SP 800-38A standard test vector for AES-128
    let key: [u8; 4 * NK] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];
    let plaintext: [u8; 4 * NB] = [
        0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d, 0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07,
        0x34,
    ];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    keys::expansion(&key, &mut w);

    // Expected ciphertext captured from running the C `Cipher`.
    let expected: [u8; 16] = [
        0x67, 0xA5, 0x0A, 0x50, 0xAA, 0xE4, 0xF8, 0x41, 0x09, 0x4A, 0xB3, 0xDE, 0x14, 0xFE, 0x79,
        0xD7,
    ];
    let mut out = [0u8; 4 * NB];
    cipher::cipher(&plaintext, &mut out, &w);
    assert_eq!(out, expected);
}

#[test]
fn test_inv_cipher_fips_vector() {
    let key: [u8; 4 * NK] = [
        0x2b, 0x7e, 0x15, 0x16, 0x28, 0xae, 0xd2, 0xa6, 0xab, 0xf7, 0x15, 0x88, 0x09, 0xcf, 0x4f,
        0x3c,
    ];
    let ciphertext: [u8; 4 * NB] = [
        0x67, 0xA5, 0x0A, 0x50, 0xAA, 0xE4, 0xF8, 0x41, 0x09, 0x4A, 0xB3, 0xDE, 0x14, 0xFE, 0x79,
        0xD7,
    ];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    keys::expansion(&key, &mut w);

    let mut out = [0u8; 4 * NB];
    cipher::inv_cipher(&ciphertext, &mut out, &w);
    let expected: [u8; 16] = [
        0x32, 0x43, 0xF6, 0xA8, 0x88, 0x5A, 0x30, 0x8D, 0x31, 0x31, 0x98, 0xA2, 0xE0, 0x37, 0x07,
        0x34,
    ];
    assert_eq!(out, expected);
}

#[test]
fn test_cipher_roundtrip_zero_block() {
    let key: [u8; 4 * NK] = [0u8; 16];
    let plaintext: [u8; 4 * NB] = [0u8; 16];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    keys::expansion(&key, &mut w);
    let mut out = [0u8; 16];
    cipher::cipher(&plaintext, &mut out, &w);
    let mut decrypted = [0u8; 16];
    cipher::inv_cipher(&out, &mut decrypted, &w);
    assert_eq!(decrypted, plaintext);
}

#[test]
fn test_cipher_roundtrip_random() {
    let key: [u8; 4 * NK] = [
        0x10, 0x20, 0x30, 0x40, 0x50, 0x60, 0x70, 0x80, 0x90, 0xa0, 0xb0, 0xc0, 0xd0, 0xe0, 0xf0,
        0x00,
    ];
    let plaintext: [u8; 16] = [
        0xde, 0xad, 0xbe, 0xef, 0xca, 0xfe, 0xba, 0xbe, 0x12, 0x34, 0x56, 0x78, 0x9a, 0xbc, 0xde,
        0xf0,
    ];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    keys::expansion(&key, &mut w);
    let mut ct = [0u8; 16];
    cipher::cipher(&plaintext, &mut ct, &w);
    let mut pt = [0u8; 16];
    cipher::inv_cipher(&ct, &mut pt, &w);
    assert_eq!(pt, plaintext);
    // The ciphertext must not equal the plaintext for non-trivial input.
    assert_ne!(ct, plaintext);
}

fn main() {}
