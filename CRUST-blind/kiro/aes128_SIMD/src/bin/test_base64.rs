use aes128_SIMD::base64;

#[test]
fn test_base64_empty() {
    assert_eq!(base64::base64_encode(b"", 0), "");
}

#[test]
fn test_base64_one_byte() {
    assert_eq!(base64::base64_encode(b"f", 1), "Zg==");
}

#[test]
fn test_base64_two_bytes() {
    assert_eq!(base64::base64_encode(b"fo", 2), "Zm8=");
}

#[test]
fn test_base64_three_bytes() {
    assert_eq!(base64::base64_encode(b"foo", 3), "Zm9v");
}

#[test]
fn test_base64_four_bytes() {
    assert_eq!(base64::base64_encode(b"foob", 4), "Zm9vYg==");
}

#[test]
fn test_base64_five_bytes() {
    assert_eq!(base64::base64_encode(b"fooba", 5), "Zm9vYmE=");
}

#[test]
fn test_base64_six_bytes() {
    assert_eq!(base64::base64_encode(b"foobar", 6), "Zm9vYmFy");
}

#[test]
fn test_g_mult_sse_byte() {
    assert_eq!(base64::g_mult_sse_byte(0x02, 0x87), 0x15);
    assert_eq!(base64::g_mult_sse_byte(0x03, 0x6e), 0xB2);
    assert_eq!(base64::g_mult_sse_byte(0x00, 0xFF), 0x00);
}

#[test]
fn test_columns_sse() {
    let mut state = [
        [0xdb, 0x13, 0x53, 0x45],
        [0xf2, 0x0a, 0x22, 0x5c],
        [0x01, 0x01, 0x01, 0x01],
        [0xc6, 0xc6, 0xc6, 0xc6],
    ];
    base64::columns_sse(&mut state);
    assert_eq!(state, [
        [0x67, 0xFF, 0x07, 0xA9],
        [0xE1, 0xC2, 0xD2, 0x38],
        [0x7A, 0x4A, 0x22, 0x4A],
        [0x12, 0xA9, 0x41, 0x05],
    ]);
}

fn main() {}
