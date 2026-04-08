use libutf::libutf_utf::*;

// === UTF-8 VALIDATE ===

#[test]
fn test_utf8_validate_ascii() {
    assert!(utf8_validate(b"Hello"));
}

#[test]
fn test_utf8_validate_empty() {
    assert!(utf8_validate(&[]));
}

#[test]
fn test_utf8_validate_two_byte() {
    assert!(utf8_validate(&[0xC3, 0xA9]));
}

#[test]
fn test_utf8_validate_three_byte() {
    assert!(utf8_validate(&[0xE4, 0xB8, 0x96]));
}

#[test]
fn test_utf8_validate_four_byte() {
    assert!(utf8_validate(&[0xF0, 0x9F, 0x98, 0x80]));
}

#[test]
fn test_utf8_validate_invalid_continuation() {
    assert!(!utf8_validate(&[0x80]));
}

#[test]
fn test_utf8_validate_overlong() {
    assert!(!utf8_validate(&[0xC0, 0x80]));
}

#[test]
fn test_utf8_validate_surrogate() {
    assert!(!utf8_validate(&[0xED, 0xA0, 0x80]));
}

#[test]
fn test_utf8_validate_truncated_2byte() {
    assert!(!utf8_validate(&[0xC3]));
}

#[test]
fn test_utf8_validate_truncated_3byte() {
    assert!(!utf8_validate(&[0xE4, 0xB8]));
}

#[test]
fn test_utf8_validate_truncated_4byte() {
    assert!(!utf8_validate(&[0xF0, 0x9F, 0x98]));
}

#[test]
fn test_utf8_validate_five_byte_leader() {
    assert!(!utf8_validate(&[0xF8, 0x80, 0x80, 0x80, 0x80]));
}

#[test]
fn test_utf8_validate_above_max() {
    assert!(!utf8_validate(&[0xF4, 0x90, 0x80, 0x80]));
}

// === UTF-16 VALIDATE ===

#[test]
fn test_utf16le_validate_ascii() {
    assert!(utf16le_validate(&[0x0048, 0x0065, 0x006C, 0x006C, 0x006F]));
}

#[test]
fn test_utf16le_validate_empty() {
    assert!(utf16le_validate(&[]));
}

#[test]
fn test_utf16le_validate_surrogate_pair() {
    assert!(utf16le_validate(&[0xD83D, 0xDE00]));
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
fn test_utf16le_validate_high_followed_by_non_low() {
    assert!(!utf16le_validate(&[0xD800, 0x0041]));
}

// === UTF-32 VALIDATE ===

#[test]
fn test_utf32_validate_ascii() {
    assert!(utf32_validate(&[0x48, 0x65, 0x6C, 0x6C, 0x6F]));
}

#[test]
fn test_utf32_validate_empty() {
    assert!(utf32_validate(&[]));
}

#[test]
fn test_utf32_validate_emoji() {
    assert!(utf32_validate(&[0x1F600]));
}

#[test]
fn test_utf32_validate_surrogate() {
    assert!(!utf32_validate(&[0xD800]));
}

#[test]
fn test_utf32_validate_above_max() {
    assert!(!utf32_validate(&[0x110000]));
}

// === ASCII VALIDATE ===

#[test]
fn test_ascii_validate_hello() {
    assert!(ascii_validate(b"Hello"));
}

#[test]
fn test_ascii_validate_empty() {
    assert!(ascii_validate(&[]));
}

#[test]
fn test_ascii_validate_non_ascii() {
    assert!(!ascii_validate(&[0x80]));
}

#[test]
fn test_ascii_validate_mixed() {
    assert!(!ascii_validate(&[0x41, 0xFF]));
}

// === LENGTH FUNCTIONS ===

#[test]
fn test_utf8_length_from_utf16le() {
    // [A, é, 世, D83D(high), DE00(low)] -> 10 utf8 bytes
    assert_eq!(utf8_length_from_utf16le(&[0x0041, 0x00E9, 0x4E16, 0xD83D, 0xDE00]), 10);
}

#[test]
fn test_utf8_length_from_utf32() {
    assert_eq!(utf8_length_from_utf32(&[0x41, 0xE9, 0x4E16, 0x1F600]), 10);
}

#[test]
fn test_utf8_length_from_latin1() {
    assert_eq!(utf8_length_from_latin1(&[0x41, 0xE9, 0xFF]), 5);
}

#[test]
fn test_utf16_length_from_utf8() {
    // "Aé世😀" in UTF-8
    assert_eq!(utf16_length_from_utf8(&[0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0x96, 0xF0, 0x9F, 0x98, 0x80]), 5);
}

#[test]
fn test_utf16_length_from_utf32() {
    assert_eq!(utf16_length_from_utf32(&[0x41, 0xE9, 0x4E16, 0x1F600]), 5);
}

#[test]
fn test_utf16_length_from_latin1() {
    assert_eq!(utf16_length_from_latin1(&[0x41, 0xE9, 0xFF]), 3);
}

#[test]
fn test_utf32_length_from_utf8() {
    assert_eq!(utf32_length_from_utf8(&[0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0x96, 0xF0, 0x9F, 0x98, 0x80]), 4);
}

#[test]
fn test_utf32_length_from_utf16le() {
    assert_eq!(utf32_length_from_utf16le(&[0x0041, 0x00E9, 0x4E16, 0xD83D, 0xDE00]), 4);
}

#[test]
fn test_utf32_length_from_latin1() {
    assert_eq!(utf32_length_from_latin1(&[0x41, 0xE9, 0xFF]), 3);
}

#[test]
fn test_latin1_length_from_utf8() {
    assert_eq!(latin1_length_from_utf8(&[0x41, 0xC3, 0xA9]), 2);
}

#[test]
fn test_latin1_length_from_utf16le() {
    assert_eq!(latin1_length_from_utf16le(&[0x41, 0xE9]), 2);
}

#[test]
fn test_latin1_length_from_utf32() {
    assert_eq!(latin1_length_from_utf32(&[0x41, 0xE9]), 2);
}

// === CONVERSION FUNCTIONS ===

#[test]
fn test_utf8_convert_to_utf16le() {
    let data: &[u8] = &[0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0x96, 0xF0, 0x9F, 0x98, 0x80];
    let mut result = [0u16; 16];
    let n = utf8_convert_to_utf16le(data, &mut result);
    assert_eq!(n, 5);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 19990);
    assert_eq!(result[3], 55357);
    assert_eq!(result[4], 56832);
}

#[test]
fn test_utf8_convert_to_utf32() {
    let data: &[u8] = &[0x41, 0xC3, 0xA9, 0xE4, 0xB8, 0x96, 0xF0, 0x9F, 0x98, 0x80];
    let mut result = [0u32; 16];
    let n = utf8_convert_to_utf32(data, &mut result);
    assert_eq!(n, 4);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 19990);
    assert_eq!(result[3], 128512);
}

#[test]
fn test_utf8_convert_to_latin1() {
    let data: &[u8] = &[0x41, 0xC3, 0xA9];
    let mut result = [0u8; 16];
    let n = utf8_convert_to_latin1(data, &mut result);
    assert_eq!(n, 2);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
}

#[test]
fn test_utf8_convert_to_latin1_invalid_3byte() {
    let data: &[u8] = &[0xE4, 0xB8, 0x96];
    let mut result = [0u8; 4];
    let n = utf8_convert_to_latin1(data, &mut result);
    assert_eq!(n, 0);
}

#[test]
fn test_utf16le_convert_to_utf8() {
    let data: &[u16] = &[0x0041, 0x00E9, 0x4E16, 0xD83D, 0xDE00];
    let mut result = [0u8; 32];
    let n = utf16le_convert_to_utf8(data, &mut result);
    assert_eq!(n, 10);
    assert_eq!(&result[..10], &[65, 195, 169, 228, 184, 150, 240, 159, 152, 128]);
}

#[test]
fn test_utf16le_convert_to_utf32() {
    let data: &[u16] = &[0x0041, 0x00E9, 0x4E16, 0xD83D, 0xDE00];
    let mut result = [0u32; 16];
    let n = utf16le_convert_to_utf32(data, &mut result);
    assert_eq!(n, 4);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 19990);
    assert_eq!(result[3], 128512);
}

#[test]
fn test_utf16le_convert_to_latin1() {
    let data: &[u16] = &[0x41, 0xE9, 0xFF];
    let mut result = [0u8; 16];
    let n = utf16le_convert_to_latin1(data, &mut result);
    assert_eq!(n, 3);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 255);
}

#[test]
fn test_utf16le_convert_to_latin1_overflow() {
    let data: &[u16] = &[0x0100];
    let mut result = [0u8; 1];
    let n = utf16le_convert_to_latin1(data, &mut result);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_convert_to_utf8() {
    let data: &[u32] = &[0x41, 0xE9, 0x4E16, 0x1F600];
    let mut result = [0u8; 32];
    let n = utf32_convert_to_utf8(data, &mut result);
    assert_eq!(n, 10);
    assert_eq!(&result[..10], &[65, 195, 169, 228, 184, 150, 240, 159, 152, 128]);
}

#[test]
fn test_utf32_convert_to_utf8_surrogate() {
    let mut result = [0u8; 8];
    assert_eq!(utf32_convert_to_utf8(&[0xD800], &mut result), 0);
}

#[test]
fn test_utf32_convert_to_utf16le() {
    let data: &[u32] = &[0x41, 0xE9, 0x4E16, 0x1F600];
    let mut result = [0u16; 16];
    let n = utf32_convert_to_utf16le(data, &mut result);
    assert_eq!(n, 5);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 19990);
    assert_eq!(result[3], 55357);
    assert_eq!(result[4], 56832);
}

#[test]
fn test_utf32_convert_to_utf16le_surrogate() {
    let mut result = [0u16; 4];
    assert_eq!(utf32_convert_to_utf16le(&[0xD800], &mut result), 0);
}

#[test]
fn test_utf32_convert_to_latin1() {
    let data: &[u32] = &[0x41, 0xE9, 0xFF];
    let mut result = [0u8; 16];
    let n = utf32_convert_to_latin1(data, &mut result);
    assert_eq!(n, 3);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 255);
}

#[test]
fn test_utf32_convert_to_latin1_overflow() {
    let mut result = [0u8; 1];
    assert_eq!(utf32_convert_to_latin1(&[0x0100], &mut result), 0);
}

#[test]
fn test_latin1_convert_to_utf8() {
    let data: &[u8] = &[0x41, 0xE9, 0xFF];
    let mut result = [0u8; 16];
    let n = latin1_convert_to_utf8(data, &mut result);
    assert_eq!(n, 5);
    assert_eq!(&result[..5], &[65, 195, 169, 195, 191]);
}

#[test]
fn test_latin1_convert_to_utf16le() {
    let data: &[u8] = &[0x41, 0xE9, 0xFF];
    let mut result = [0u16; 16];
    let n = latin1_convert_to_utf16le(data, &mut result);
    assert_eq!(n, 3);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 255);
}

#[test]
fn test_latin1_convert_to_utf32() {
    let data: &[u8] = &[0x41, 0xE9, 0xFF];
    let mut result = [0u32; 16];
    let n = latin1_convert_to_utf32(data, &mut result);
    assert_eq!(n, 3);
    assert_eq!(result[0], 65);
    assert_eq!(result[1], 233);
    assert_eq!(result[2], 255);
}

fn main() {}
