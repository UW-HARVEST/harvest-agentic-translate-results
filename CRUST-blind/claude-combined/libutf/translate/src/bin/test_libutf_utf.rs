use libutf::libutf_utf::*;

#[test]
fn test_ascii_validate() {
    assert_eq!(ascii_validate(b"Hello, World!"), true);
    assert_eq!(ascii_validate(&[0x80]), false);
    assert_eq!(ascii_validate(&[]), true);
    assert_eq!(ascii_validate(b"abcdefghijklmnopqrstuvwxyz0123456789"), true);
    let mut d = [b'a'; 32];
    d[20] = 0x80;
    assert_eq!(ascii_validate(&d), false);
}

#[test]
fn test_utf8_validate() {
    assert_eq!(utf8_validate(b"Hello"), true);
    assert_eq!(utf8_validate(&[0xC2, 0xA9]), true);
    assert_eq!(utf8_validate(&[0xE2, 0x82, 0xAC]), true);
    assert_eq!(utf8_validate(&[0xF0, 0x9F, 0x98, 0x80]), true);
    assert_eq!(utf8_validate(&[0x80]), false);
    assert_eq!(utf8_validate(&[0xC0, 0xAF]), false);
    assert_eq!(utf8_validate(&[0xED, 0xA0, 0x80]), false);
    assert_eq!(utf8_validate(&[0xC2]), false);
    assert_eq!(utf8_validate(&[]), true);
    assert_eq!(utf8_validate(b"abcdefghijklmnopqrstuvwxyz"), true);
}

#[test]
fn test_utf8_length_from_latin1() {
    assert_eq!(utf8_length_from_latin1(b"Hello"), 5);
    assert_eq!(utf8_length_from_latin1(&[0x80, 0xff]), 4);
    assert_eq!(utf8_length_from_latin1(&[]), 0);
}

#[test]
fn test_utf8_length_from_utf16le() {
    let h = [b'H' as u16, b'i' as u16];
    assert_eq!(utf8_length_from_utf16le(&h), 2);
    assert_eq!(utf8_length_from_utf16le(&[0x00A9]), 2);
    assert_eq!(utf8_length_from_utf16le(&[0x20AC]), 3);
    assert_eq!(utf8_length_from_utf16le(&[0xD83D, 0xDE00]), 4);
    assert_eq!(utf8_length_from_utf16le(&[]), 0);
}

#[test]
fn test_utf8_length_from_utf32() {
    assert_eq!(utf8_length_from_utf32(&[0x48, 0x69]), 2);
    assert_eq!(utf8_length_from_utf32(&[0x00A9, 0x20AC, 0x1F600]), 9);
    assert_eq!(utf8_length_from_utf32(&[]), 0);
}

#[test]
fn test_utf16_length_from_utf8() {
    let fb: [u8; 10] = [0x48, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80];
    assert_eq!(utf16_length_from_utf8(&fb), 5);
    assert_eq!(utf16_length_from_utf8(&[]), 0);
}

#[test]
fn test_utf16_length_from_utf32() {
    assert_eq!(utf16_length_from_utf32(&[0x00A9, 0x20AC, 0x1F600]), 4);
    assert_eq!(utf16_length_from_utf32(&[]), 0);
}

#[test]
fn test_utf16_length_from_latin1() {
    assert_eq!(utf16_length_from_latin1(b"Hello"), 5);
    assert_eq!(utf16_length_from_latin1(&[]), 0);
}

#[test]
fn test_utf32_length_from_utf8() {
    let fb: [u8; 10] = [0x48, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80];
    assert_eq!(utf32_length_from_utf8(&fb), 4);
    assert_eq!(utf32_length_from_utf8(&[]), 0);
}

#[test]
fn test_utf32_length_from_utf16le() {
    assert_eq!(utf32_length_from_utf16le(&[0xD83D, 0xDE00]), 1);
    assert_eq!(utf32_length_from_utf16le(&[b'H' as u16, b'i' as u16]), 2);
    assert_eq!(utf32_length_from_utf16le(&[]), 0);
}

#[test]
fn test_utf32_length_from_latin1() {
    assert_eq!(utf32_length_from_latin1(b"Hello"), 5);
}

#[test]
fn test_latin1_length_from_utf8() {
    let fb: [u8; 10] = [0x48, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80];
    assert_eq!(latin1_length_from_utf8(&fb), 4);
    assert_eq!(latin1_length_from_utf8(&[]), 0);
}

#[test]
fn test_latin1_length_from_utf16le() {
    assert_eq!(latin1_length_from_utf16le(&[b'H' as u16, b'i' as u16]), 2);
    assert_eq!(latin1_length_from_utf16le(&[]), 0);
}

#[test]
fn test_latin1_length_from_utf32() {
    assert_eq!(latin1_length_from_utf32(&[0x48, 0x69]), 2);
    assert_eq!(latin1_length_from_utf32(&[]), 0);
}

#[test]
fn test_utf8_convert_to_utf16le() {
    let input: [u8; 10] = [0x48, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80];
    let mut out = [0u16; 16];
    let n = utf8_convert_to_utf16le(&input, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..n], &[72, 169, 8364, 55357, 56832]);

    // Empty
    let n = utf8_convert_to_utf16le(&[], &mut out);
    assert_eq!(n, 0);

    // 16-byte ascii fast path
    let in16 = b"abcdefghijklmnop";
    let n = utf8_convert_to_utf16le(in16, &mut out);
    assert_eq!(n, 16);
    let expected: [u16; 16] = [97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112];
    assert_eq!(&out[..16], &expected);
}

#[test]
fn test_utf8_convert_to_utf32() {
    let input: [u8; 10] = [0x48, 0xC2, 0xA9, 0xE2, 0x82, 0xAC, 0xF0, 0x9F, 0x98, 0x80];
    let mut out = [0u32; 16];
    let n = utf8_convert_to_utf32(&input, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..n], &[72, 169, 8364, 128512]);

    // Bad
    let n = utf8_convert_to_utf32(&[0xC0, 0xAF], &mut out);
    assert_eq!(n, 0);

    // Empty
    let n = utf8_convert_to_utf32(&[], &mut out);
    assert_eq!(n, 0);

    // 16-byte ASCII fast path
    let n = utf8_convert_to_utf32(b"abcdefghijklmnop", &mut out);
    assert_eq!(n, 16);
    let expected: [u32; 16] = [97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112];
    assert_eq!(&out[..16], &expected);
}

#[test]
fn test_utf8_convert_to_latin1() {
    let input: [u8; 3] = [0x48, 0xC2, 0xA9];
    let mut out = [0u8; 16];
    let n = utf8_convert_to_latin1(&input, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..n], &[72, 169]);

    // Bad
    let n = utf8_convert_to_latin1(&[0xC0, 0xAF], &mut out);
    assert_eq!(n, 0);

    // Out of range
    let n = utf8_convert_to_latin1(&[0xE2, 0x82, 0xAC], &mut out);
    assert_eq!(n, 0);

    // Long fast path
    let n = utf8_convert_to_latin1(b"abcdefghijklmnop", &mut out);
    assert_eq!(n, 16);
    let expected: [u8; 16] = [97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112];
    assert_eq!(&out[..16], &expected);
}

#[test]
fn test_utf16le_validate() {
    assert_eq!(utf16le_validate(&[0x48, 0xD83D, 0xDE00]), true);
    assert_eq!(utf16le_validate(&[0xD83D]), false);
    assert_eq!(utf16le_validate(&[0xDC00]), false);
    assert_eq!(utf16le_validate(&[0xD83D, 0x0048]), false);
    assert_eq!(utf16le_validate(&[]), true);
}

#[test]
fn test_utf16le_convert_to_utf8() {
    let input: [u16; 5] = [0x48, 0x00A9, 0x20AC, 0xD83D, 0xDE00];
    let mut out = [0u8; 32];
    let n = utf16le_convert_to_utf8(&input, &mut out);
    assert_eq!(n, 10);
    assert_eq!(&out[..n], &[72, 194, 169, 226, 130, 172, 240, 159, 152, 128]);

    // Fast path: 4 ASCII u16
    let fast: [u16; 4] = [b'a' as u16, b'b' as u16, b'c' as u16, b'd' as u16];
    let n = utf16le_convert_to_utf8(&fast, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..n], &[97, 98, 99, 100]);

    // Empty
    let n = utf16le_convert_to_utf8(&[], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_utf16le_convert_to_utf32() {
    let input: [u16; 5] = [0x48, 0x00A9, 0x20AC, 0xD83D, 0xDE00];
    let mut out = [0u32; 16];
    let n = utf16le_convert_to_utf32(&input, &mut out);
    assert_eq!(n, 4);
    assert_eq!(&out[..n], &[72, 169, 8364, 128512]);

    // Empty
    let n = utf16le_convert_to_utf32(&[], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_utf16le_convert_to_latin1() {
    let input: [u16; 2] = [0x48, 0xA9];
    let mut out = [0u8; 16];
    let n = utf16le_convert_to_latin1(&input, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..n], &[72, 169]);

    let n = utf16le_convert_to_latin1(&[0x100], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_validate() {
    assert_eq!(utf32_validate(&[0x48, 0xA9, 0x20AC, 0x1F600]), true);
    assert_eq!(utf32_validate(&[0xD800]), false);
    assert_eq!(utf32_validate(&[0x110000]), false);
    assert_eq!(utf32_validate(&[]), true);
}

#[test]
fn test_utf32_convert_to_utf8() {
    let input: [u32; 4] = [0x48, 0xA9, 0x20AC, 0x1F600];
    let mut out = [0u8; 32];
    let n = utf32_convert_to_utf8(&input, &mut out);
    assert_eq!(n, 10);
    assert_eq!(&out[..n], &[72, 194, 169, 226, 130, 172, 240, 159, 152, 128]);

    // Surrogate
    let n = utf32_convert_to_utf8(&[0xD800], &mut out);
    assert_eq!(n, 0);

    // Out of range
    let n = utf32_convert_to_utf8(&[0x110000], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_convert_to_utf16le() {
    let input: [u32; 4] = [0x48, 0xA9, 0x20AC, 0x1F600];
    let mut out = [0u16; 16];
    let n = utf32_convert_to_utf16le(&input, &mut out);
    assert_eq!(n, 5);
    assert_eq!(&out[..n], &[72, 169, 8364, 55357, 56832]);

    // Surrogate
    let n = utf32_convert_to_utf16le(&[0xD800], &mut out);
    assert_eq!(n, 0);

    // Out of range
    let n = utf32_convert_to_utf16le(&[0x110000], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_convert_to_latin1() {
    let input: [u32; 2] = [0x48, 0xA9];
    let mut out = [0u8; 16];
    let n = utf32_convert_to_latin1(&input, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..n], &[72, 169]);

    let n = utf32_convert_to_latin1(&[0x100], &mut out);
    assert_eq!(n, 0);
}

#[test]
fn test_latin1_convert_to_utf8() {
    let input: [u8; 2] = [0x48, 0xA9];
    let mut out = [0u8; 16];
    let n = latin1_convert_to_utf8(&input, &mut out);
    assert_eq!(n, 3);
    assert_eq!(&out[..n], &[72, 194, 169]);

    // 16-byte ASCII fast path
    let n = latin1_convert_to_utf8(b"abcdefghijklmnop", &mut out);
    assert_eq!(n, 16);
    let expected: [u8; 16] = [97,98,99,100,101,102,103,104,105,106,107,108,109,110,111,112];
    assert_eq!(&out[..16], &expected);
}

#[test]
fn test_latin1_convert_to_utf16le() {
    let input: [u8; 2] = [0x48, 0xA9];
    let mut out = [0u16; 16];
    let n = latin1_convert_to_utf16le(&input, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..n], &[72, 169]);
}

#[test]
fn test_latin1_convert_to_utf32() {
    let input: [u8; 2] = [0x48, 0xA9];
    let mut out = [0u32; 16];
    let n = latin1_convert_to_utf32(&input, &mut out);
    assert_eq!(n, 2);
    assert_eq!(&out[..n], &[72, 169]);
}

fn main() {}
