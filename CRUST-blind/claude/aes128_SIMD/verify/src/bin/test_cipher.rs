use aes128_SIMD::aes::{NB, NK, NR};
use aes128_SIMD::cipher::{cipher, inv_cipher};
use aes128_SIMD::keys::expansion;

#[test]
fn test_cipher_fips_vector() {
    // FIPS-197 key & plaintext, but expected ciphertext is taken from this
    // implementation's C oracle — the C `Expansion` does not produce a
    // standard AES-128 key schedule, so the ciphertext is implementation-
    // specific.
    let key: [u8; 4 * NK] = [
        0x2B, 0x7E, 0x15, 0x16, 0x28, 0xAE, 0xD2, 0xA6,
        0xAB, 0xF7, 0x15, 0x88, 0x09, 0xCF, 0x4F, 0x3C,
    ];
    let pt: [u8; 4 * NB] = [
        0x32, 0x43, 0xf6, 0xa8, 0x88, 0x5a, 0x30, 0x8d,
        0x31, 0x31, 0x98, 0xa2, 0xe0, 0x37, 0x07, 0x34,
    ];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    expansion(&key, &mut w);

    let mut ct = [0u8; 4 * NB];
    cipher(&pt, &mut ct, &w);
    let expected_ct: [u8; 16] = [
        0x67, 0xA5, 0x0A, 0x50, 0xAA, 0xE4, 0xF8, 0x41,
        0x09, 0x4A, 0xB3, 0xDE, 0x14, 0xFE, 0x79, 0xD7,
    ];
    assert_eq!(ct, expected_ct);

    let mut decrypted = [0u8; 4 * NB];
    inv_cipher(&ct, &mut decrypted, &w);
    assert_eq!(decrypted, pt);
}

#[test]
fn test_cipher_zero_key_zero_plaintext() {
    let key = [0u8; 4 * NK];
    let pt = [0u8; 4 * NB];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    expansion(&key, &mut w);

    let mut ct = [0u8; 4 * NB];
    cipher(&pt, &mut ct, &w);
    let expected_ct: [u8; 16] = [
        0x45, 0xD4, 0xDF, 0x3D, 0x15, 0xAE, 0xE3, 0x42,
        0x56, 0xF9, 0xB0, 0x43, 0xD2, 0x29, 0xAC, 0x69,
    ];
    assert_eq!(ct, expected_ct);

    let mut decrypted = [0u8; 4 * NB];
    inv_cipher(&ct, &mut decrypted, &w);
    assert_eq!(decrypted, pt);
}

#[test]
fn test_cipher_inv_cipher_roundtrip_random() {
    let key: [u8; 4 * NK] = [
        0xDE, 0xAD, 0xBE, 0xEF, 0xCA, 0xFE, 0xBA, 0xBE,
        0x12, 0x34, 0x56, 0x78, 0x9A, 0xBC, 0xDE, 0xF0,
    ];
    let pt: [u8; 4 * NB] = [
        0x00, 0x11, 0x22, 0x33, 0x44, 0x55, 0x66, 0x77,
        0x88, 0x99, 0xAA, 0xBB, 0xCC, 0xDD, 0xEE, 0xFF,
    ];
    let mut w = [0u8; 4 * NB * (NR + 1)];
    expansion(&key, &mut w);

    let mut ct = [0u8; 4 * NB];
    cipher(&pt, &mut ct, &w);

    // ct must differ from pt for a non-trivial cipher.
    assert_ne!(ct, pt);

    let mut decrypted = [0u8; 4 * NB];
    inv_cipher(&ct, &mut decrypted, &w);
    assert_eq!(decrypted, pt);
}

fn main() {}
