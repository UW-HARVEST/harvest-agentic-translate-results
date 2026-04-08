use libutf::libutf_utf::*;

// === utf8_validate ===

#[test]
fn test_utf8_validate_ascii() {
    assert!(utf8_validate(b"Hello"));
}

#[test]
fn test_utf8_validate_empty() {
    assert!(utf8_validate(&[]));
}

#[test]
fn test_utf8_validate_2byte() {
    assert!(utf8_validate(&[0xC3, 0xA9])); // é
}

#[test]
fn test_utf8_validate_3byte() {
    assert!(utf8_validate(&[0xE2, 0x82, 0xAC])); // €
}

#[test]
fn test_utf8_validate_4byte() {
    assert!(utf8_validate(&[0xF0, 0x9D, 0x84, 0x9E])); // 𝄞
}

#[test]
fn test_utf8_validate_mixed() {
    // A + é + € + 𝄞
    assert!(utf8_validate(&[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E]));
}

#[test]
fn test_utf8_validate_invalid_continuation() {
    assert!(!utf8_validate(&[0x80]));
}

#[test]
fn test_utf8_validate_overlong_2byte() {
    assert!(!utf8_validate(&[0xC0, 0x80]));
}

#[test]
fn test_utf8_validate_truncated_2byte() {
    assert!(!utf8_validate(&[0xC3]));
}

#[test]
fn test_utf8_validate_surrogate() {
    assert!(!utf8_validate(&[0xED, 0xA0, 0x80])); // U+D800
}

#[test]
fn test_utf8_validate_over_10ffff() {
    assert!(!utf8_validate(&[0xF4, 0x90, 0x80, 0x80]));
}

#[test]
fn test_utf8_validate_truncated_3byte() {
    assert!(!utf8_validate(&[0xE2, 0x82]));
}

#[test]
fn test_utf8_validate_truncated_4byte() {
    assert!(!utf8_validate(&[0xF0, 0x9D, 0x84]));
}

#[test]
fn test_utf8_validate_bad_continuation_3byte() {
    assert!(!utf8_validate(&[0xE2, 0x82, 0x00]));
}

#[test]
fn test_utf8_validate_bad_continuation_4byte() {
    assert!(!utf8_validate(&[0xF0, 0x9D, 0x84, 0x00]));
}

// === utf16le_validate ===

#[test]
fn test_utf16le_validate_basic() {
    assert!(utf16le_validate(&[0x0041, 0x0042]));
}

#[test]
fn test_utf16le_validate_empty() {
    assert!(utf16le_validate(&[]));
}

#[test]
fn test_utf16le_validate_surrogate_pair() {
    assert!(utf16le_validate(&[0xD800, 0xDC00]));
}

#[test]
fn test_utf16le_validate_lone_high_surrogate() {
    assert!(!utf16le_validate(&[0xD800]));
}

#[test]
fn test_utf16le_validate_lone_low_surrogate() {
    assert!(!utf16le_validate(&[0xDC00]));
}

#[test]
fn test_utf16le_validate_reversed_surrogates() {
    assert!(!utf16le_validate(&[0xDC00, 0xD800]));
}

#[test]
fn test_utf16le_validate_max_bmp() {
    assert!(utf16le_validate(&[0xFFFF]));
}

#[test]
fn test_utf16le_validate_max_surrogate_pair() {
    assert!(utf16le_validate(&[0xDBFF, 0xDFFF])); // U+10FFFF
}

// === utf32_validate ===

#[test]
fn test_utf32_validate_basic() {
    assert!(utf32_validate(&[0x41, 0x10FFFF]));
}

#[test]
fn test_utf32_validate_empty() {
    assert!(utf32_validate(&[]));
}

#[test]
fn test_utf32_validate_surrogate() {
    assert!(!utf32_validate(&[0xD800]));
}

#[test]
fn test_utf32_validate_over() {
    assert!(!utf32_validate(&[0x110000]));
}

#[test]
fn test_utf32_validate_dfff() {
    assert!(!utf32_validate(&[0xDFFF]));
}

#[test]
fn test_utf32_validate_zero() {
    assert!(utf32_validate(&[0]));
}

// === ascii_validate ===

#[test]
fn test_ascii_validate_valid() {
    assert!(ascii_validate(b"Hello World"));
}

#[test]
fn test_ascii_validate_empty() {
    assert!(ascii_validate(&[]));
}

#[test]
fn test_ascii_validate_invalid() {
    assert!(!ascii_validate(&[0x80]));
}

#[test]
fn test_ascii_validate_boundary() {
    assert!(ascii_validate(&[0x7F]));
}

// === length functions ===

#[test]
fn test_utf8_length_from_utf16le() {
    // [A, é, €, surrogate pair for U+10000]
    assert_eq!(utf8_length_from_utf16le(&[0x41, 0xE9, 0x20AC, 0xD800, 0xDC00]), 10);
}

#[test]
fn test_utf8_length_from_utf16le_empty() {
    assert_eq!(utf8_length_from_utf16le(&[]), 0);
}

#[test]
fn test_utf8_length_from_utf32() {
    assert_eq!(utf8_length_from_utf32(&[0x41, 0xE9, 0x20AC, 0x10000]), 10);
}

#[test]
fn test_utf8_length_from_utf32_empty() {
    assert_eq!(utf8_length_from_utf32(&[]), 0);
}

#[test]
fn test_utf8_length_from_latin1() {
    assert_eq!(utf8_length_from_latin1(&[0x41, 0xE9, 0xFF]), 5);
}

#[test]
fn test_utf8_length_from_latin1_ascii_only() {
    assert_eq!(utf8_length_from_latin1(b"abc"), 3);
}

#[test]
fn test_utf8_length_from_latin1_empty() {
    assert_eq!(utf8_length_from_latin1(&[]), 0);
}

#[test]
fn test_utf16_length_from_utf8() {
    // A(1) + é(2) + €(3) + 𝄞(4) = 10 bytes -> 5 utf16 units
    assert_eq!(utf16_length_from_utf8(&[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E]), 5);
}

#[test]
fn test_utf16_length_from_utf8_empty() {
    assert_eq!(utf16_length_from_utf8(&[]), 0);
}

#[test]
fn test_utf16_length_from_utf32() {
    assert_eq!(utf16_length_from_utf32(&[0x41, 0xE9, 0x20AC, 0x10000]), 5);
}

#[test]
fn test_utf16_length_from_latin1() {
    assert_eq!(utf16_length_from_latin1(&[0x41, 0xE9, 0xFF]), 3);
}

#[test]
fn test_utf32_length_from_utf8() {
    assert_eq!(utf32_length_from_utf8(&[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E]), 4);
}

#[test]
fn test_utf32_length_from_utf16le() {
    // 2 BMP + 1 surrogate pair = 3 codepoints
    assert_eq!(utf32_length_from_utf16le(&[0x0041, 0x0042, 0xD800, 0xDC00]), 3);
}

#[test]
fn test_utf32_length_from_latin1() {
    assert_eq!(utf32_length_from_latin1(&[0x41, 0xE9, 0xFF]), 3);
}

#[test]
fn test_latin1_length_from_utf8() {
    assert_eq!(latin1_length_from_utf8(&[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E]), 4);
}

#[test]
fn test_latin1_length_from_utf16le() {
    assert_eq!(latin1_length_from_utf16le(&[0x41, 0xE9, 0xFF]), 3);
}

#[test]
fn test_latin1_length_from_utf32() {
    assert_eq!(latin1_length_from_utf32(&[0x41, 0xE9, 0xFF]), 3);
}

// === conversion functions ===

#[test]
fn test_utf8_convert_to_utf16le() {
    let input: &[u8] = &[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E];
    let mut out = [0u16; 16];
    let n = utf8_convert_to_utf16le(input, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], &[0x0041, 0x00E9, 0x20AC, 0xD834, 0xDD1E]);
}

#[test]
fn test_utf8_convert_to_utf16le_empty() {
    let mut out = [0u16; 1];
    assert_eq!(utf8_convert_to_utf16le(&[], &mut out), 0);
}

#[test]
fn test_utf8_convert_to_utf32() {
    let input: &[u8] = &[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9D, 0x84, 0x9E];
    let mut out = [0u32; 16];
    let n = utf8_convert_to_utf32(input, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], &[0x41, 0xE9, 0x20AC, 0x1D11E]);
}

#[test]
fn test_utf8_convert_to_utf32_empty() {
    let mut out = [0u32; 1];
    assert_eq!(utf8_convert_to_utf32(&[], &mut out), 0);
}

#[test]
fn test_utf8_convert_to_latin1() {
    let mut out = [0u8; 16];
    let n = utf8_convert_to_latin1(&[0x41, 0xC3, 0xA9], &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..2], &[0x41, 0xE9]);
}

#[test]
fn test_utf8_convert_to_latin1_empty() {
    let mut out = [0u8; 1];
    assert_eq!(utf8_convert_to_latin1(&[], &mut out), 0);
}

#[test]
fn test_utf8_convert_to_latin1_out_of_range() {
    // 3-byte sequence is out of latin1 range
    let mut out = [0u8; 16];
    assert_eq!(utf8_convert_to_latin1(&[0xE2, 0x82, 0xAC], &mut out), 0);
}

#[test]
fn test_utf16le_convert_to_utf8() {
    let input: &[u16] = &[0x41, 0xE9, 0x20AC, 0xD800, 0xDC00];
    let mut out = [0u8; 32];
    let n = utf16le_convert_to_utf8(input, &mut out);
    assert_eq!(n, 10);
    assert_eq!(&out[..10], &[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x90, 0x80, 0x80]);
}

#[test]
fn test_utf16le_convert_to_utf8_empty() {
    let mut out = [0u8; 1];
    assert_eq!(utf16le_convert_to_utf8(&[], &mut out), 0);
}

#[test]
fn test_utf16le_convert_to_utf32() {
    let input: &[u16] = &[0x41, 0xE9, 0x20AC, 0xD800, 0xDC00];
    let mut out = [0u32; 16];
    let n = utf16le_convert_to_utf32(input, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..4], &[0x41, 0xE9, 0x20AC, 0x10000]);
}

#[test]
fn test_utf16le_convert_to_latin1() {
    let mut out = [0u8; 16];
    let n = utf16le_convert_to_latin1(&[0x41, 0x42, 0xFF], &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], &[0x41, 0x42, 0xFF]);
}

#[test]
fn test_utf16le_convert_to_latin1_overflow() {
    let mut out = [0u8; 16];
    assert_eq!(utf16le_convert_to_latin1(&[0x100], &mut out), 0);
}

#[test]
fn test_utf32_convert_to_utf8() {
    let input: &[u32] = &[0x41, 0xE9, 0x20AC, 0x10000];
    let mut out = [0u8; 32];
    let n = utf32_convert_to_utf8(input, &mut out);
    assert_eq!(n, 10);
    assert_eq!(&out[..10], &[0x41, 0xC3, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x90, 0x80, 0x80]);
}

#[test]
fn test_utf32_convert_to_utf8_surrogate() {
    let mut out = [0u8; 16];
    assert_eq!(utf32_convert_to_utf8(&[0xD800], &mut out), 0);
}

#[test]
fn test_utf32_convert_to_utf8_over() {
    let mut out = [0u8; 16];
    assert_eq!(utf32_convert_to_utf8(&[0x110000], &mut out), 0);
}

#[test]
fn test_utf32_convert_to_utf16le() {
    let input: &[u32] = &[0x41, 0xE9, 0x20AC, 0x10000];
    let mut out = [0u16; 16];
    let n = utf32_convert_to_utf16le(input, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], &[0x0041, 0x00E9, 0x20AC, 0xD800, 0xDC00]);
}

#[test]
fn test_utf32_convert_to_utf16le_surrogate() {
    let mut out = [0u16; 16];
    assert_eq!(utf32_convert_to_utf16le(&[0xD800], &mut out), 0);
}

#[test]
fn test_utf32_convert_to_utf16le_over() {
    let mut out = [0u16; 16];
    assert_eq!(utf32_convert_to_utf16le(&[0x110000], &mut out), 0);
}

#[test]
fn test_utf32_convert_to_latin1() {
    let mut out = [0u8; 16];
    let n = utf32_convert_to_latin1(&[0x41, 0x42, 0xFF], &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], &[0x41, 0x42, 0xFF]);
}

#[test]
fn test_utf32_convert_to_latin1_overflow() {
    let mut out = [0u8; 16];
    assert_eq!(utf32_convert_to_latin1(&[0x100], &mut out), 0);
}

#[test]
fn test_latin1_convert_to_utf8() {
    let mut out = [0u8; 16];
    let n = latin1_convert_to_utf8(&[0x41, 0xE9, 0xFF], &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..5], &[0x41, 0xC3, 0xA9, 0xC3, 0xBF]);
}

#[test]
fn test_latin1_convert_to_utf8_empty() {
    let mut out = [0u8; 1];
    assert_eq!(latin1_convert_to_utf8(&[], &mut out), 0);
}

#[test]
fn test_latin1_convert_to_utf16le() {
    let mut out = [0u16; 16];
    let n = latin1_convert_to_utf16le(&[0x41, 0xE9, 0xFF], &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], &[0x0041, 0x00E9, 0x00FF]);
}

#[test]
fn test_latin1_convert_to_utf32() {
    let mut out = [0u32; 16];
    let n = latin1_convert_to_utf32(&[0x41, 0xE9, 0xFF], &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..3], &[0x41, 0xE9, 0xFF]);
}

fn main() {}
