use aes128_SIMD::base64::{base64_encode, columns_sse, g_mult_sse, g_mult_sse_byte};

#[test]
fn test_base64_hello() {
    let s = b"Hello";
    assert_eq!(base64_encode(s, s.len()), "SGVsbG8=");
}

#[test]
fn test_base64_hi() {
    let s = b"Hi";
    assert_eq!(base64_encode(s, s.len()), "SGk=");
}

#[test]
fn test_base64_foobar() {
    let s = b"Foobar";
    assert_eq!(base64_encode(s, s.len()), "Rm9vYmFy");
}

#[test]
fn test_base64_random_bytes() {
    let data: [u8; 8] = [0x00, 0x01, 0x02, 0x03, 0xFC, 0xFD, 0xFE, 0xFF];
    assert_eq!(base64_encode(&data, 8), "AAECA/z9/v8=");
}

#[test]
fn test_base64_one_byte() {
    let s = b"a";
    assert_eq!(base64_encode(s, 1), "YQ==");
}

#[test]
fn test_base64_lorem() {
    let s = b"Lorem ipsum dolor sit amet, consectetur adipiscing elit. Sed do eiusmod tempor ...";
    let expected = "TG9yZW0gaXBzdW0gZG9sb3Igc2l0IGFtZXQsIGNvbnNlY3RldHVyIGFkaXBpc2NpbmcgZWxpdC4gU2VkIGRvIGVpdXNtb2QgdGVtcG9yIC4uLg==";
    assert_eq!(base64_encode(s, s.len()), expected);
}

#[test]
fn test_g_mult_sse_byte() {
    // Should match g_mult exactly.
    assert_eq!(g_mult_sse_byte(0x57, 0x13), 0xFE);
    assert_eq!(g_mult_sse_byte(0x02, 0x80), 0x1B);
    assert_eq!(g_mult_sse_byte(0x03, 0xff), 0x1A);
    assert_eq!(g_mult_sse_byte(0x00, 0xff), 0x00);
    assert_eq!(g_mult_sse_byte(0x01, 0xab), 0xAB);
    assert_eq!(g_mult_sse_byte(0x53, 0xca), 0x01);
}

#[test]
fn test_g_mult_sse() {
    use std::arch::x86_64::{_mm_loadu_si128, _mm_storeu_si128, __m128i};
    let a: [u8; 16] = [
        0x57, 0x02, 0x03, 0x01, 0x53, 0x10, 0x0e, 0x0d,
        0x09, 0x0b, 0x14, 0xff, 0xaa, 0x55, 0xcc, 0x33,
    ];
    let b: [u8; 16] = [
        0x13, 0x80, 0xff, 0xab, 0xca, 0x55, 0x0b, 0x09,
        0x0d, 0x0e, 0x10, 0x01, 0x55, 0xaa, 0x33, 0xcc,
    ];
    let mut expected = [0u8; 16];
    for i in 0..16 {
        expected[i] = g_mult_sse_byte(a[i], b[i]);
    }
    let res: [u8; 16];
    unsafe {
        let av = _mm_loadu_si128(a.as_ptr() as *const __m128i);
        let bv = _mm_loadu_si128(b.as_ptr() as *const __m128i);
        let r = g_mult_sse(av, bv);
        let mut out = [0u8; 16];
        _mm_storeu_si128(out.as_mut_ptr() as *mut __m128i, r);
        res = out;
    }
    assert_eq!(res, expected);
}

#[test]
fn test_columns_sse_matches_columns() {
    // columns_sse must match the standard MixColumns transformation.
    let mut state: [[u8; 4]; 4] = [
        [0xd4, 0xe0, 0xb8, 0x1e],
        [0xbf, 0xb4, 0x41, 0x27],
        [0x5d, 0x52, 0x11, 0x98],
        [0x30, 0xae, 0xf1, 0xe5],
    ];
    columns_sse(&mut state);
    let expected: [[u8; 4]; 4] = [
        [0x04, 0xE0, 0x48, 0x28],
        [0x66, 0xCB, 0xF8, 0x06],
        [0x81, 0x19, 0xD3, 0x26],
        [0xE5, 0x9A, 0x7A, 0x4C],
    ];
    assert_eq!(state, expected);
}

fn main() {}
