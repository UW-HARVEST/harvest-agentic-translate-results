use aes128_SIMD::aes::NB;
use aes128_SIMD::base64;
use aes128_SIMD::cipher_utils;
use std::arch::x86_64::{__m128i, _mm_loadu_si128, _mm_storeu_si128};

#[test]
fn test_base64_basic() {
    assert_eq!(base64::base64_encode(b"Hi", 2), "SGk=");
    assert_eq!(base64::base64_encode(b"Hi!", 3), "SGkh");
    assert_eq!(base64::base64_encode(b"Hello", 5), "SGVsbG8=");
    assert_eq!(base64::base64_encode(b"", 0), "");
}

#[test]
fn test_base64_lorem() {
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor ...";
    let encoded = base64::base64_encode(s, s.len());
    assert_eq!(
        encoded,
        "TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGFtZXQsIGNvbnNlY3RldHVyIGFkaXBpc2NpbmcgZWxpdC4gU2VkIGRvIGVpdXNtb2QgdGVtcG9yIC4uLg=="
    );
}

#[test]
fn test_base64_one_byte() {
    // single byte 'M' should encode as "TQ=="
    assert_eq!(base64::base64_encode(b"M", 1), "TQ==");
    // single byte 0xff
    let inp = [0xffu8];
    assert_eq!(base64::base64_encode(&inp, 1), "/w==");
}

#[test]
fn test_base64_full_alphabet_byte() {
    // Bytes that produce all base64 alphabet characters
    let bytes = [0x00u8, 0x10, 0x83, 0x10, 0x51, 0x87, 0x20, 0x92, 0x8B, 0x30, 0xD3, 0x8F, 0x41, 0x14, 0x93, 0x51, 0x55, 0x97, 0x61, 0x96, 0x9B, 0x71, 0xD7, 0x9F, 0x82, 0x18, 0xA3, 0x92, 0x59, 0xA7, 0xA2, 0x9A, 0xAB, 0xB2, 0xDB, 0xAF, 0xC3, 0x1C, 0xB3, 0xD3, 0x5D, 0xB7, 0xE3, 0x9E, 0xBB, 0xF3, 0xDF, 0xBF];
    let encoded = base64::base64_encode(&bytes, bytes.len());
    assert_eq!(
        encoded,
        "ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/"
    );
}

#[test]
fn test_g_mult_sse_byte() {
    // g_mult_sse_byte should match g_mult
    assert_eq!(base64::g_mult_sse_byte(0x57, 0x83), 0xC1);
    assert_eq!(base64::g_mult_sse_byte(0x53, 0xCA), 0x01);
    assert_eq!(base64::g_mult_sse_byte(0x02, 0x80), 0x1B);
    assert_eq!(base64::g_mult_sse_byte(0x00, 0xff), 0x00);
    assert_eq!(base64::g_mult_sse_byte(0x01, 0x42), 0x42);

    // And it must agree with cipher_utils::g_mult exhaustively for a few values
    for a in 0..=255u8 {
        assert_eq!(base64::g_mult_sse_byte(a, 0x03), cipher_utils::g_mult(a, 0x03));
    }
}

#[test]
fn test_g_mult_sse_lanewise() {
    let a_bytes: [u8; 16] = [
        0x57, 0x53, 0x02, 0x03, 0x0e, 0x0b, 0x0d, 0x09, 0x01, 0x80, 0xff, 0x42, 0x10, 0x55, 0xaa,
        0x77,
    ];
    let b_bytes: [u8; 16] = [
        0x83, 0xCA, 0x80, 0xff, 0x53, 0x10, 0xab, 0xff, 0x42, 0x02, 0x03, 0x01, 0x10, 0x10, 0x10,
        0x10,
    ];
    let a: __m128i =
        unsafe { _mm_loadu_si128(a_bytes.as_ptr() as *const __m128i) };
    let b: __m128i =
        unsafe { _mm_loadu_si128(b_bytes.as_ptr() as *const __m128i) };
    let r = base64::g_mult_sse(a, b);
    let mut out = [0u8; 16];
    unsafe {
        _mm_storeu_si128(out.as_mut_ptr() as *mut __m128i, r);
    }
    let expected: [u8; 16] = [
        cipher_utils::g_mult(0x57, 0x83),
        cipher_utils::g_mult(0x53, 0xCA),
        cipher_utils::g_mult(0x02, 0x80),
        cipher_utils::g_mult(0x03, 0xff),
        cipher_utils::g_mult(0x0e, 0x53),
        cipher_utils::g_mult(0x0b, 0x10),
        cipher_utils::g_mult(0x0d, 0xab),
        cipher_utils::g_mult(0x09, 0xff),
        cipher_utils::g_mult(0x01, 0x42),
        cipher_utils::g_mult(0x80, 0x02),
        cipher_utils::g_mult(0xff, 0x03),
        cipher_utils::g_mult(0x42, 0x01),
        cipher_utils::g_mult(0x10, 0x10),
        cipher_utils::g_mult(0x55, 0x10),
        cipher_utils::g_mult(0xaa, 0x10),
        cipher_utils::g_mult(0x77, 0x10),
    ];
    assert_eq!(out, expected);
}

#[test]
fn test_columns_sse_matches_columns() {
    // columns_sse should produce the same result as the standard MixColumns.
    let mut state_a: [[u8; NB]; 4] = [
        [0xd4, 0xe0, 0xb8, 0x1e],
        [0xbf, 0xb4, 0x41, 0x27],
        [0x5d, 0x52, 0x11, 0x98],
        [0x30, 0xae, 0xf1, 0xe5],
    ];
    base64::columns_sse(&mut state_a);
    assert_eq!(state_a[0], [0x04, 0xe0, 0x48, 0x28]);
    assert_eq!(state_a[1], [0x66, 0xcb, 0xf8, 0x06]);
    assert_eq!(state_a[2], [0x81, 0x19, 0xd3, 0x26]);
    assert_eq!(state_a[3], [0xe5, 0x9a, 0x7a, 0x4c]);
}

fn main() {}
