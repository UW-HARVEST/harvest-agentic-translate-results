use aes128_SIMD::base64::{base64_encode, g_mult_sse_byte, columns_sse};
use aes128_SIMD::aes::NB;

#[test]
fn test_base64_encode_hello() {
    assert_eq!(base64_encode(b"Hello", 5), "SGVsbG8=");
}

#[test]
fn test_base64_encode_1byte() {
    assert_eq!(base64_encode(b"a", 1), "YQ==");
}

#[test]
fn test_base64_encode_2bytes() {
    assert_eq!(base64_encode(b"ab", 2), "YWI=");
}

#[test]
fn test_base64_encode_3bytes() {
    assert_eq!(base64_encode(b"abc", 3), "YWJj");
}

#[test]
fn test_base64_encode_empty() {
    assert_eq!(base64_encode(b"", 0), "");
}

#[test]
fn test_base64_encode_lorem() {
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor ...";
    assert_eq!(
        base64_encode(s, s.len()),
        "TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGFtZXQsIGNvbnNlY3RldHVyIGFkaXBpc2NpbmcgZWxpdC4gU2VkIGRvIGVpdXNtb2QgdGVtcG9yIC4uLg=="
    );
}

#[test]
fn test_g_mult_sse_byte() {
    assert_eq!(g_mult_sse_byte(0x02, 0x87), 0x15);
    assert_eq!(g_mult_sse_byte(0x03, 0x6E), 0xB2);
}

#[test]
fn test_columns_sse() {
    let mut state: [[u8; NB]; 4] = [
        [0xDB, 0x13, 0x53, 0x45],
        [0xF2, 0x0A, 0x22, 0x5C],
        [0x01, 0x01, 0x01, 0x01],
        [0xC6, 0xC6, 0xC6, 0xC6],
    ];
    columns_sse(&mut state);
    assert_eq!(state, [
        [0x67, 0xFF, 0x07, 0xA9],
        [0xE1, 0xC2, 0xD2, 0x38],
        [0x7A, 0x4A, 0x22, 0x4A],
        [0x12, 0xA9, 0x41, 0x05],
    ]);
}

fn main() {}
