use libutf::libutf_utf::*;

// ----------------- ascii_validate -----------------

#[test]
fn test_ascii_validate_abc() {
    let s: [u8; 3] = [0x41, 0x42, 0x43];
    assert_eq!(ascii_validate(&s), true);
}

#[test]
fn test_ascii_validate_high_bit() {
    let s: [u8; 1] = [0x80];
    assert_eq!(ascii_validate(&s), false);
}

#[test]
fn test_ascii_validate_zeros20() {
    let s = [0u8; 20];
    assert_eq!(ascii_validate(&s), true);
}

#[test]
fn test_ascii_validate_high_bit_after_block() {
    let mut s = [0u8; 20];
    s[17] = 0x80;
    // First 16 bytes pass fast path (zeros), then byte at idx 17 fails
    assert_eq!(ascii_validate(&s), false);
}

#[test]
fn test_ascii_validate_high_bit_in_block() {
    let mut s = [0u8; 20];
    s[5] = 0xff;
    assert_eq!(ascii_validate(&s), false);
}

#[test]
fn test_ascii_validate_empty() {
    let s: [u8; 0] = [];
    assert_eq!(ascii_validate(&s), true);
}

// ----------------- utf8_validate -----------------

#[test]
fn test_utf8_validate_hello() {
    let s = b"Hello, World!";
    assert_eq!(utf8_validate(s), true);
}

#[test]
fn test_utf8_validate_2byte() {
    let s: &[u8] = b"\xc3\xa9";
    assert_eq!(utf8_validate(s), true);
}

#[test]
fn test_utf8_validate_3byte_euro() {
    let s: &[u8] = b"\xe2\x82\xac";
    assert_eq!(utf8_validate(s), true);
}

#[test]
fn test_utf8_validate_4byte_emoji() {
    let s: &[u8] = b"\xf0\x9f\x98\x80";
    assert_eq!(utf8_validate(s), true);
}

#[test]
fn test_utf8_validate_lone_continuation() {
    let s: &[u8] = b"\x80";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_overlong_null() {
    let s: &[u8] = b"\xc0\x80";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_surrogate() {
    let s: &[u8] = b"\xed\xa0\x80";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_above_10ffff() {
    let s: &[u8] = b"\xf4\x90\x80\x80";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_invalid_lead_byte() {
    let s: &[u8] = b"\xfe";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_truncated_2() {
    let s: &[u8] = b"\xc3";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_truncated_3() {
    let s: &[u8] = b"\xe2\x82";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_truncated_4() {
    let s: &[u8] = b"\xf0\x9f\x98";
    assert_eq!(utf8_validate(s), false);
}

#[test]
fn test_utf8_validate_empty() {
    let s: &[u8] = &[];
    assert_eq!(utf8_validate(s), true);
}

// ----------------- utf8_length_from_utf16le -----------------

#[test]
fn test_utf8_length_from_utf16le_ascii() {
    let data: [u16; 2] = [0x41, 0x42];
    assert_eq!(utf8_length_from_utf16le(&data), 2);
}

#[test]
fn test_utf8_length_from_utf16le_2byte() {
    let data: [u16; 2] = [0xe9, 0xe8];
    assert_eq!(utf8_length_from_utf16le(&data), 4);
}

#[test]
fn test_utf8_length_from_utf16le_3byte() {
    let data: [u16; 1] = [0x20ac];
    assert_eq!(utf8_length_from_utf16le(&data), 3);
}

#[test]
fn test_utf8_length_from_utf16le_surrogate_pair() {
    let data: [u16; 2] = [0xd83d, 0xde00];
    // each surrogate counts as 2 bytes -> total 4
    assert_eq!(utf8_length_from_utf16le(&data), 4);
}

#[test]
fn test_utf8_length_from_utf16le_lone_surrogate() {
    let data: [u16; 1] = [0xd800];
    assert_eq!(utf8_length_from_utf16le(&data), 2);
}

#[test]
fn test_utf8_length_from_utf16le_d7ff() {
    let data: [u16; 1] = [0xd7ff];
    assert_eq!(utf8_length_from_utf16le(&data), 3);
}

#[test]
fn test_utf8_length_from_utf16le_e000() {
    let data: [u16; 1] = [0xe000];
    assert_eq!(utf8_length_from_utf16le(&data), 3);
}

#[test]
fn test_utf8_length_from_utf16le_7f() {
    let data: [u16; 1] = [0x7f];
    assert_eq!(utf8_length_from_utf16le(&data), 1);
}

#[test]
fn test_utf8_length_from_utf16le_7ff() {
    let data: [u16; 1] = [0x7ff];
    assert_eq!(utf8_length_from_utf16le(&data), 2);
}

#[test]
fn test_utf8_length_from_utf16le_800() {
    let data: [u16; 1] = [0x800];
    assert_eq!(utf8_length_from_utf16le(&data), 3);
}

#[test]
fn test_utf8_length_from_utf16le_empty() {
    let data: [u16; 0] = [];
    assert_eq!(utf8_length_from_utf16le(&data), 0);
}

// ----------------- utf8_length_from_utf32 -----------------

#[test]
fn test_utf8_length_from_utf32_ascii() {
    let data: [u32; 1] = [0x41];
    assert_eq!(utf8_length_from_utf32(&data), 1);
}

#[test]
fn test_utf8_length_from_utf32_80() {
    let data: [u32; 1] = [0x80];
    assert_eq!(utf8_length_from_utf32(&data), 2);
}

#[test]
fn test_utf8_length_from_utf32_800() {
    let data: [u32; 1] = [0x800];
    assert_eq!(utf8_length_from_utf32(&data), 3);
}

#[test]
fn test_utf8_length_from_utf32_10000() {
    let data: [u32; 1] = [0x10000];
    assert_eq!(utf8_length_from_utf32(&data), 4);
}

#[test]
fn test_utf8_length_from_utf32_mix() {
    let data: [u32; 4] = [0x7f, 0x7ff, 0xffff, 0x10ffff];
    // 1 + 2 + 3 + 4 = 10
    assert_eq!(utf8_length_from_utf32(&data), 10);
}

#[test]
fn test_utf8_length_from_utf32_empty() {
    let data: [u32; 0] = [];
    assert_eq!(utf8_length_from_utf32(&data), 0);
}

// ----------------- utf8_length_from_latin1 -----------------

#[test]
fn test_utf8_length_from_latin1_ascii() {
    let data = [0x41u8, 0x42, 0x43];
    assert_eq!(utf8_length_from_latin1(&data), 3);
}

#[test]
fn test_utf8_length_from_latin1_high() {
    let data = [0x80u8, 0xff];
    assert_eq!(utf8_length_from_latin1(&data), 4);
}

#[test]
fn test_utf8_length_from_latin1_mix() {
    let data = [0x7fu8, 0x80];
    assert_eq!(utf8_length_from_latin1(&data), 3);
}

#[test]
fn test_utf8_length_from_latin1_empty() {
    let data: [u8; 0] = [];
    assert_eq!(utf8_length_from_latin1(&data), 0);
}

// ----------------- utf8_convert_to_utf16le -----------------

#[test]
fn test_utf8_convert_to_utf16le_hello() {
    let data = b"Hello";
    let mut buf = [0u16; 8];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], &[0x0048, 0x0065, 0x006c, 0x006c, 0x006f]);
}

#[test]
fn test_utf8_convert_to_utf16le_2byte() {
    let data: &[u8] = b"\xc3\xa9";
    let mut buf = [0u16; 4];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0x00e9]);
}

#[test]
fn test_utf8_convert_to_utf16le_3byte_euro() {
    let data: &[u8] = b"\xe2\x82\xac";
    let mut buf = [0u16; 4];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0x20ac]);
}

#[test]
fn test_utf8_convert_to_utf16le_4byte_emoji() {
    let data: &[u8] = b"\xf0\x9f\x98\x80";
    let mut buf = [0u16; 4];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], &[0xd83d, 0xde00]);
}

#[test]
fn test_utf8_convert_to_utf16le_mix() {
    let data: &[u8] = b"a \xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80";
    let mut buf = [0u16; 16];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 8);
    assert_eq!(
        &buf[..n],
        &[0x0061, 0x0020, 0x00e9, 0x0020, 0x20ac, 0x0020, 0xd83d, 0xde00]
    );
}

#[test]
fn test_utf8_convert_to_utf16le_17_ascii() {
    let data = b"abcdefghijklmnopq";
    let mut buf = [0u16; 32];
    let n = utf8_convert_to_utf16le(data, &mut buf);
    assert_eq!(n, 17);
    let expected: Vec<u16> = (b'a'..=b'q').map(|b| b as u16).collect();
    assert_eq!(&buf[..n], &expected[..]);
}

// ----------------- utf8_convert_to_utf32 -----------------

#[test]
fn test_utf8_convert_to_utf32_hello() {
    let data = b"Hello";
    let mut buf = [0u32; 8];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], &[0x48, 0x65, 0x6c, 0x6c, 0x6f]);
}

#[test]
fn test_utf8_convert_to_utf32_2byte() {
    let data: &[u8] = b"\xc3\xa9";
    let mut buf = [0u32; 4];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0xe9]);
}

#[test]
fn test_utf8_convert_to_utf32_3byte() {
    let data: &[u8] = b"\xe2\x82\xac";
    let mut buf = [0u32; 4];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0x20ac]);
}

#[test]
fn test_utf8_convert_to_utf32_4byte() {
    let data: &[u8] = b"\xf0\x9f\x98\x80";
    let mut buf = [0u32; 4];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0x1f600]);
}

#[test]
fn test_utf8_convert_to_utf32_invalid() {
    let data: &[u8] = b"\x80";
    let mut buf = [0u32; 4];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 0);
}

#[test]
fn test_utf8_convert_to_utf32_surrogate() {
    let data: &[u8] = b"\xed\xa0\x80";
    let mut buf = [0u32; 4];
    let n = utf8_convert_to_utf32(data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf8_convert_to_latin1 -----------------

#[test]
fn test_utf8_convert_to_latin1_hello() {
    let data = b"Hello";
    let mut buf = [0u8; 16];
    let n = utf8_convert_to_latin1(data, &mut buf);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"Hello");
}

#[test]
fn test_utf8_convert_to_latin1_e9() {
    let data: &[u8] = b"\xc3\xa9";
    let mut buf = [0u8; 4];
    let n = utf8_convert_to_latin1(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0xe9]);
}

#[test]
fn test_utf8_convert_to_latin1_ff() {
    let data: &[u8] = b"\xc3\xbf";
    let mut buf = [0u8; 4];
    let n = utf8_convert_to_latin1(data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0xff]);
}

#[test]
fn test_utf8_convert_to_latin1_out_of_range() {
    let data: &[u8] = b"\xc4\x80";
    let mut buf = [0u8; 4];
    let n = utf8_convert_to_latin1(data, &mut buf);
    assert_eq!(n, 0);
}

#[test]
fn test_utf8_convert_to_latin1_3byte_rejected() {
    let data: &[u8] = b"\xe2\x82\xac";
    let mut buf = [0u8; 4];
    let n = utf8_convert_to_latin1(data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf16le_validate -----------------

#[test]
fn test_utf16le_validate_hello() {
    let data: [u16; 5] = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
    assert_eq!(utf16le_validate(&data), true);
}

#[test]
fn test_utf16le_validate_surrogate_pair() {
    let data: [u16; 2] = [0xd801, 0xdc37];
    assert_eq!(utf16le_validate(&data), true);
}

#[test]
fn test_utf16le_validate_lone_high() {
    let data: [u16; 1] = [0xd801];
    assert_eq!(utf16le_validate(&data), false);
}

#[test]
fn test_utf16le_validate_lone_low() {
    let data: [u16; 1] = [0xdc37];
    assert_eq!(utf16le_validate(&data), false);
}

#[test]
fn test_utf16le_validate_high_high() {
    let data: [u16; 2] = [0xd801, 0xd802];
    assert_eq!(utf16le_validate(&data), false);
}

#[test]
fn test_utf16le_validate_low_low() {
    let data: [u16; 2] = [0xdc37, 0xdc38];
    assert_eq!(utf16le_validate(&data), false);
}

#[test]
fn test_utf16le_validate_low_high() {
    let data: [u16; 2] = [0xdc37, 0xd801];
    assert_eq!(utf16le_validate(&data), false);
}

#[test]
fn test_utf16le_validate_empty() {
    let data: [u16; 0] = [];
    assert_eq!(utf16le_validate(&data), true);
}

// ----------------- utf16_length_from_utf8 -----------------

#[test]
fn test_utf16_length_from_utf8_hello() {
    let data = b"Hello";
    assert_eq!(utf16_length_from_utf8(data), 5);
}

#[test]
fn test_utf16_length_from_utf8_2byte() {
    let data: &[u8] = b"\xc3\xa9";
    assert_eq!(utf16_length_from_utf8(data), 1);
}

#[test]
fn test_utf16_length_from_utf8_3byte() {
    let data: &[u8] = b"\xe2\x82\xac";
    assert_eq!(utf16_length_from_utf8(data), 1);
}

#[test]
fn test_utf16_length_from_utf8_4byte() {
    let data: &[u8] = b"\xf0\x9f\x98\x80";
    assert_eq!(utf16_length_from_utf8(data), 2);
}

#[test]
fn test_utf16_length_from_utf8_mix() {
    let data: &[u8] = b"a \xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80";
    assert_eq!(utf16_length_from_utf8(data), 8);
}

// ----------------- utf16_length_from_utf32 -----------------

#[test]
fn test_utf16_length_from_utf32_ascii() {
    let data: [u32; 2] = [0x41, 0x42];
    assert_eq!(utf16_length_from_utf32(&data), 2);
}

#[test]
fn test_utf16_length_from_utf32_10000() {
    let data: [u32; 1] = [0x10000];
    assert_eq!(utf16_length_from_utf32(&data), 2);
}

#[test]
fn test_utf16_length_from_utf32_ffff() {
    let data: [u32; 1] = [0xffff];
    assert_eq!(utf16_length_from_utf32(&data), 1);
}

#[test]
fn test_utf16_length_from_utf32_10ffff() {
    let data: [u32; 1] = [0x10ffff];
    assert_eq!(utf16_length_from_utf32(&data), 2);
}

// ----------------- utf16_length_from_latin1 -----------------

#[test]
fn test_utf16_length_from_latin1() {
    let data = [1u8, 2, 3];
    assert_eq!(utf16_length_from_latin1(&data), 3);
}

#[test]
fn test_utf16_length_from_latin1_empty() {
    let data: [u8; 0] = [];
    assert_eq!(utf16_length_from_latin1(&data), 0);
}

// ----------------- utf16le_convert_to_utf8 -----------------

#[test]
fn test_utf16le_convert_to_utf8_hello() {
    let data: [u16; 5] = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
    let mut buf = [0u8; 32];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"Hello");
}

#[test]
fn test_utf16le_convert_to_utf8_e9() {
    let data: [u16; 1] = [0xe9];
    let mut buf = [0u8; 8];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], &[0xc3, 0xa9]);
}

#[test]
fn test_utf16le_convert_to_utf8_euro() {
    let data: [u16; 1] = [0x20ac];
    let mut buf = [0u8; 8];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0xe2, 0x82, 0xac]);
}

#[test]
fn test_utf16le_convert_to_utf8_emoji() {
    let data: [u16; 2] = [0xd83d, 0xde00];
    let mut buf = [0u8; 8];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 4);
    assert_eq!(&buf[..n], &[0xf0, 0x9f, 0x98, 0x80]);
}

#[test]
fn test_utf16le_convert_to_utf8_helloworld() {
    let data: [u16; 11] = [0x48, 0x65, 0x6c, 0x6c, 0x6f, 0x20, 0x57, 0x6f, 0x72, 0x6c, 0x64];
    let mut buf = [0u8; 32];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 11);
    assert_eq!(&buf[..n], b"Hello World");
}

#[test]
fn test_utf16le_convert_to_utf8_mix() {
    let data: [u16; 8] = [0x61, 0x20, 0xe9, 0x20, 0x20ac, 0x20, 0xd83d, 0xde00];
    let mut buf = [0u8; 32];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 13);
    assert_eq!(
        &buf[..n],
        &[0x61, 0x20, 0xc3, 0xa9, 0x20, 0xe2, 0x82, 0xac, 0x20, 0xf0, 0x9f, 0x98, 0x80]
    );
}

#[test]
fn test_utf16le_convert_to_utf8_lone_high() {
    let data: [u16; 1] = [0xd800];
    let mut buf = [0u8; 8];
    let n = utf16le_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf16le_convert_to_utf32 -----------------

#[test]
fn test_utf16le_convert_to_utf32_basic() {
    let data: [u16; 3] = [0x48, 0x65, 0x6c];
    let mut buf = [0u32; 8];
    let n = utf16le_convert_to_utf32(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0x48, 0x65, 0x6c]);
}

#[test]
fn test_utf16le_convert_to_utf32_emoji() {
    let data: [u16; 2] = [0xd83d, 0xde00];
    let mut buf = [0u32; 4];
    let n = utf16le_convert_to_utf32(&data, &mut buf);
    assert_eq!(n, 1);
    assert_eq!(&buf[..n], &[0x1f600]);
}

#[test]
fn test_utf16le_convert_to_utf32_lone_high() {
    let data: [u16; 1] = [0xd800];
    let mut buf = [0u32; 4];
    let n = utf16le_convert_to_utf32(&data, &mut buf);
    assert_eq!(n, 0);
}

#[test]
fn test_utf16le_convert_to_utf32_hi_then_nonlow() {
    let data: [u16; 2] = [0xd800, 0x0041];
    let mut buf = [0u32; 4];
    let n = utf16le_convert_to_utf32(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf16le_convert_to_latin1 -----------------

#[test]
fn test_utf16le_convert_to_latin1_basic() {
    let data: [u16; 3] = [0x41, 0x42, 0xe9];
    let mut buf = [0u8; 8];
    let n = utf16le_convert_to_latin1(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0x41, 0x42, 0xe9]);
}

#[test]
fn test_utf16le_convert_to_latin1_overflow() {
    let data: [u16; 1] = [0x100];
    let mut buf = [0u8; 4];
    let n = utf16le_convert_to_latin1(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf32_validate -----------------

#[test]
fn test_utf32_validate_valid() {
    let data: [u32; 2] = [0x41, 0x10ffff];
    assert_eq!(utf32_validate(&data), true);
}

#[test]
fn test_utf32_validate_above_max() {
    let data: [u32; 1] = [0x110000];
    assert_eq!(utf32_validate(&data), false);
}

#[test]
fn test_utf32_validate_d800() {
    let data: [u32; 1] = [0xd800];
    assert_eq!(utf32_validate(&data), false);
}

#[test]
fn test_utf32_validate_dfff() {
    let data: [u32; 1] = [0xdfff];
    assert_eq!(utf32_validate(&data), false);
}

#[test]
fn test_utf32_validate_e000() {
    let data: [u32; 1] = [0xe000];
    assert_eq!(utf32_validate(&data), true);
}

#[test]
fn test_utf32_validate_empty() {
    let data: [u32; 0] = [];
    assert_eq!(utf32_validate(&data), true);
}

// ----------------- utf32_length_from_utf8 -----------------

#[test]
fn test_utf32_length_from_utf8_hello() {
    let data = b"Hello";
    assert_eq!(utf32_length_from_utf8(data), 5);
}

#[test]
fn test_utf32_length_from_utf8_2byte() {
    let data: &[u8] = b"\xc3\xa9";
    assert_eq!(utf32_length_from_utf8(data), 1);
}

#[test]
fn test_utf32_length_from_utf8_3byte() {
    let data: &[u8] = b"\xe2\x82\xac";
    assert_eq!(utf32_length_from_utf8(data), 1);
}

#[test]
fn test_utf32_length_from_utf8_4byte() {
    let data: &[u8] = b"\xf0\x9f\x98\x80";
    assert_eq!(utf32_length_from_utf8(data), 1);
}

#[test]
fn test_utf32_length_from_utf8_mix() {
    let data: &[u8] = b"a \xc3\xa9 \xe2\x82\xac \xf0\x9f\x98\x80";
    assert_eq!(utf32_length_from_utf8(data), 7);
}

// ----------------- utf32_length_from_utf16le -----------------

#[test]
fn test_utf32_length_from_utf16le_basic() {
    let data: [u16; 2] = [0x41, 0x42];
    assert_eq!(utf32_length_from_utf16le(&data), 2);
}

#[test]
fn test_utf32_length_from_utf16le_emoji() {
    let data: [u16; 2] = [0xd83d, 0xde00];
    assert_eq!(utf32_length_from_utf16le(&data), 1);
}

#[test]
fn test_utf32_length_from_utf16le_mix() {
    let data: [u16; 4] = [0x48, 0x65, 0xd83d, 0xde00];
    assert_eq!(utf32_length_from_utf16le(&data), 3);
}

// ----------------- utf32_length_from_latin1 -----------------

#[test]
fn test_utf32_length_from_latin1() {
    let data = [1u8, 2, 3];
    assert_eq!(utf32_length_from_latin1(&data), 3);
}

// ----------------- utf32_convert_to_utf8 -----------------

#[test]
fn test_utf32_convert_to_utf8_hello() {
    let data: [u32; 5] = [0x48, 0x65, 0x6c, 0x6c, 0x6f];
    let mut buf = [0u8; 16];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 5);
    assert_eq!(&buf[..n], b"Hello");
}

#[test]
fn test_utf32_convert_to_utf8_e9() {
    let data: [u32; 1] = [0xe9];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], &[0xc3, 0xa9]);
}

#[test]
fn test_utf32_convert_to_utf8_euro() {
    let data: [u32; 1] = [0x20ac];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0xe2, 0x82, 0xac]);
}

#[test]
fn test_utf32_convert_to_utf8_emoji() {
    let data: [u32; 1] = [0x1f600];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 4);
    assert_eq!(&buf[..n], &[0xf0, 0x9f, 0x98, 0x80]);
}

#[test]
fn test_utf32_convert_to_utf8_surrogate_rejected() {
    let data: [u32; 1] = [0xd800];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_convert_to_utf8_above_max() {
    let data: [u32; 1] = [0x110000];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf32_convert_to_utf16le -----------------

#[test]
fn test_utf32_convert_to_utf16le_basic() {
    let data: [u32; 3] = [0x48, 0x65, 0x6c];
    let mut buf = [0u16; 8];
    let n = utf32_convert_to_utf16le(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0x0048, 0x0065, 0x006c]);
}

#[test]
fn test_utf32_convert_to_utf16le_emoji() {
    let data: [u32; 1] = [0x1f600];
    let mut buf = [0u16; 4];
    let n = utf32_convert_to_utf16le(&data, &mut buf);
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], &[0xd83d, 0xde00]);
}

#[test]
fn test_utf32_convert_to_utf16le_surrogate_rejected() {
    let data: [u32; 1] = [0xd800];
    let mut buf = [0u16; 4];
    let n = utf32_convert_to_utf16le(&data, &mut buf);
    assert_eq!(n, 0);
}

#[test]
fn test_utf32_convert_to_utf16le_above_max() {
    let data: [u32; 1] = [0x110000];
    let mut buf = [0u16; 4];
    let n = utf32_convert_to_utf16le(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- utf32_convert_to_latin1 -----------------

#[test]
fn test_utf32_convert_to_latin1_basic() {
    let data: [u32; 3] = [0x41, 0x42, 0xe9];
    let mut buf = [0u8; 8];
    let n = utf32_convert_to_latin1(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0x41, 0x42, 0xe9]);
}

#[test]
fn test_utf32_convert_to_latin1_overflow() {
    let data: [u32; 1] = [0x100];
    let mut buf = [0u8; 4];
    let n = utf32_convert_to_latin1(&data, &mut buf);
    assert_eq!(n, 0);
}

// ----------------- latin1_length_from_* -----------------

#[test]
fn test_latin1_length_from_utf8_hello() {
    let data = b"Hello";
    assert_eq!(latin1_length_from_utf8(data), 5);
}

#[test]
fn test_latin1_length_from_utf8_2byte() {
    let data: &[u8] = b"\xc3\xa9";
    assert_eq!(latin1_length_from_utf8(data), 1);
}

#[test]
fn test_latin1_length_from_utf8_3byte() {
    let data: &[u8] = b"\xe2\x82\xac";
    assert_eq!(latin1_length_from_utf8(data), 1);
}

#[test]
fn test_latin1_length_from_utf8_4byte() {
    let data: &[u8] = b"\xf0\x9f\x98\x80";
    assert_eq!(latin1_length_from_utf8(data), 1);
}

#[test]
fn test_latin1_length_from_utf16le() {
    let data: [u16; 3] = [1, 2, 3];
    assert_eq!(latin1_length_from_utf16le(&data), 3);
}

#[test]
fn test_latin1_length_from_utf32() {
    let data: [u32; 3] = [1, 2, 3];
    assert_eq!(latin1_length_from_utf32(&data), 3);
}

// ----------------- latin1_convert_to_utf8 -----------------

#[test]
fn test_latin1_convert_to_utf8_basic() {
    let data: [u8; 4] = [0x41, 0x42, 0xe9, 0xff];
    let mut buf = [0u8; 16];
    let n = latin1_convert_to_utf8(&data, &mut buf);
    assert_eq!(n, 6);
    assert_eq!(&buf[..n], &[0x41, 0x42, 0xc3, 0xa9, 0xc3, 0xbf]);
}

#[test]
fn test_latin1_convert_to_utf8_16ascii_then_high() {
    // 16 ascii bytes (fast path) then 0xa0 (-> 2 bytes), then padding
    let mut data = [0u8; 20];
    for i in 0..16 {
        data[i] = (i + 1) as u8;
    }
    data[16] = 0xa0;
    // remaining bytes are 0 (ascii)
    let mut buf = [0u8; 32];
    let n = latin1_convert_to_utf8(&data, &mut buf);
    // Expected: 16 + 2 + 3 = 21
    assert_eq!(n, 21);
    let mut expected: Vec<u8> = (1..=16).collect();
    expected.extend_from_slice(&[0xc2, 0xa0]);
    expected.extend_from_slice(&[0, 0, 0]);
    assert_eq!(&buf[..n], &expected[..]);
}

// ----------------- latin1_convert_to_utf16le -----------------

#[test]
fn test_latin1_convert_to_utf16le_basic() {
    let data: [u8; 2] = [0x41, 0xe9];
    let mut buf = [0u16; 4];
    let n = latin1_convert_to_utf16le(&data, &mut buf);
    assert_eq!(n, 2);
    assert_eq!(&buf[..n], &[0x0041, 0x00e9]);
}

// ----------------- latin1_convert_to_utf32 -----------------

#[test]
fn test_latin1_convert_to_utf32_basic() {
    let data: [u8; 3] = [0x41, 0xe9, 0xff];
    let mut buf = [0u32; 4];
    let n = latin1_convert_to_utf32(&data, &mut buf);
    assert_eq!(n, 3);
    assert_eq!(&buf[..n], &[0x41, 0xe9, 0xff]);
}

fn main() {}
