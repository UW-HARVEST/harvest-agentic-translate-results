use utf8::utf8;

// "Hello Здравствуйте こんにちは 🚩😁"
// 5 + 1 + 12*2 + 1 + 5*3 + 1 + 2*4 = 55 bytes, 27 chars
const MIXED: &[u8] = "Hello Здравствуйте こんにちは 🚩😁".as_bytes();

#[test]
fn test_validate_utf8_valid() {
    let v = utf8::validate_utf8(MIXED);
    assert!(v.valid);
    assert_eq!(v.valid_upto, 55);
}

#[test]
fn test_validate_utf8_empty() {
    let v = utf8::validate_utf8(b"");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 0);
}

#[test]
fn test_validate_utf8_invalid() {
    // "Hello Здравствуйте" = 30 bytes, then \xC0\xC0
    let mut bytes = Vec::from(&b"Hello "[..]);
    bytes.extend_from_slice("Здравствуйте".as_bytes());
    bytes.extend_from_slice(b"\xC0\xC0");
    bytes.extend_from_slice(" ".as_bytes());
    bytes.extend_from_slice("こんにちは".as_bytes());
    let v = utf8::validate_utf8(&bytes);
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 30);
}

#[test]
fn test_validate_utf8_boundary_chars() {
    // last 1-byte
    let v = utf8::validate_utf8(b"\x7F");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 1);

    // first 2-byte
    let v = utf8::validate_utf8(b"\xC2\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // last 2-byte
    let v = utf8::validate_utf8(b"\xDF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // first 3-byte
    let v = utf8::validate_utf8(b"\xE0\xA0\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // last 3-byte
    let v = utf8::validate_utf8(b"\xEF\xBF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // first 4-byte
    let v = utf8::validate_utf8(b"\xF0\x90\x80\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 4);

    // last 4-byte
    let v = utf8::validate_utf8(b"\xF7\xBF\xBF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 4);
}

#[test]
fn test_surrogate_rejection() {
    for seq in &[
        b"\xED\xA0\x80".as_slice(),
        b"\xED\xAC\x80",
        b"\xED\xA0\x8C",
        b"\xED\xBF\xBF",
    ] {
        let v = utf8::validate_utf8(seq);
        assert!(!v.valid);
        assert_eq!(v.valid_upto, 0);
    }
}

#[test]
fn test_overlong_encoding() {
    for seq in &[
        b"\xC1\x88".as_slice(),
        b"\xE0\x81\x88",
        b"\xF0\x80\x81\x88",
        b"\xC1\xBF",
        b"\xE0\x9F\xBF",
        b"\xF0\x8F\xBF\xBF",
    ] {
        let v = utf8::validate_utf8(seq);
        assert!(!v.valid);
        assert_eq!(v.valid_upto, 0);
    }
}

#[test]
fn test_validate_utf8_char() {
    let cv = utf8::validate_utf8_char(b"H", 0);
    assert!(cv.valid);
    assert_eq!(cv.next_offset, 1);

    let cv = utf8::validate_utf8_char("д".as_bytes(), 0);
    assert!(cv.valid);
    assert_eq!(cv.next_offset, 2);

    let cv = utf8::validate_utf8_char("こ".as_bytes(), 0);
    assert!(cv.valid);
    assert_eq!(cv.next_offset, 3);

    let cv = utf8::validate_utf8_char("😁".as_bytes(), 0);
    assert!(cv.valid);
    assert_eq!(cv.next_offset, 4);

    let cv = utf8::validate_utf8_char(b"\xC0", 0);
    assert!(!cv.valid);
    assert_eq!(cv.next_offset, 0);
}

#[test]
fn test_make_utf8_string_valid() {
    let s = utf8::make_utf8_string(b"Hello");
    assert_eq!(s.str, "Hello");
    assert_eq!(s.byte_len, 5);
}

#[test]
fn test_make_utf8_string_invalid() {
    let s = utf8::make_utf8_string(b"\xC0\xC0");
    assert_eq!(s.str, "");
    assert_eq!(s.byte_len, 0);
}

#[test]
fn test_make_utf8_string_mixed() {
    let s = utf8::make_utf8_string(MIXED);
    assert_eq!(s.str, "Hello Здравствуйте こんにちは 🚩😁");
    assert_eq!(s.byte_len, 55);
}

#[test]
fn test_make_utf8_string_lossy_valid() {
    let s = utf8::make_utf8_string_lossy(MIXED);
    assert_eq!(s.str, "Hello Здравствуйте こんにちは 🚩😁");
    assert_eq!(s.byte_len, 55);
}

#[test]
fn test_make_utf8_string_lossy_simple_invalid() {
    // "hello\xC0\xC0 world!" -> "hello\u{FFFD}\u{FFFD} world!" = 18 bytes
    let mut bytes = Vec::from(b"hello" as &[u8]);
    bytes.extend_from_slice(b"\xC0\xC0");
    bytes.extend_from_slice(b" world!");
    let s = utf8::make_utf8_string_lossy(&bytes);
    assert_eq!(s.byte_len, 18);
    assert_eq!(s.str, "hello\u{FFFD}\u{FFFD} world!");
}

#[test]
fn test_make_utf8_string_lossy_all_invalid() {
    let s = utf8::make_utf8_string_lossy(b"\xC0\xC0\xC0\xC0");
    assert_eq!(s.byte_len, 12);
    assert_eq!(s.str, "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}");
}

#[test]
fn test_make_utf8_string_lossy_mixed_invalid() {
    // "\xC0He\xC0llo Здр\xC0авствуйте\xC0\xC0 こんに\xC0\xC0\xC0\xC0ちは 🚩\xC0😁\xC0"
    let mut bytes: Vec<u8> = Vec::new();
    bytes.push(0xC0);
    bytes.extend_from_slice(b"He");
    bytes.push(0xC0);
    bytes.extend_from_slice(b"llo ");
    bytes.extend_from_slice("Здр".as_bytes());
    bytes.push(0xC0);
    bytes.extend_from_slice("авствуйте".as_bytes());
    bytes.extend_from_slice(b"\xC0\xC0 ");
    bytes.extend_from_slice("こんに".as_bytes());
    bytes.extend_from_slice(b"\xC0\xC0\xC0\xC0");
    bytes.extend_from_slice("ちは ".as_bytes());
    bytes.extend_from_slice("🚩".as_bytes());
    bytes.push(0xC0);
    bytes.extend_from_slice("😁".as_bytes());
    bytes.push(0xC0);
    let s = utf8::make_utf8_string_lossy(&bytes);
    assert_eq!(s.byte_len, 88);
}

#[test]
fn test_make_utf8_string_lossy_empty() {
    let s = utf8::make_utf8_string_lossy(b"");
    assert_eq!(s.str, "");
    assert_eq!(s.byte_len, 0);
}

#[test]
fn test_as_utf8_string() {
    let owned = utf8::make_utf8_string_lossy(b"test");
    let s = utf8::as_utf8_string(&owned);
    assert_eq!(s.str, "test");
    assert_eq!(s.byte_len, 4);
}

#[test]
fn test_free_owned_utf8_string() {
    let mut owned = utf8::make_utf8_string_lossy(b"test");
    utf8::free_owned_utf8_string(&mut owned);
    assert_eq!(owned.str, "");
    assert_eq!(owned.byte_len, 0);
}

#[test]
fn test_slice_utf8_string_valid() {
    let s = utf8::make_utf8_string(MIXED);
    let sl = utf8::slice_utf8_string(s, 6, 24);
    assert_eq!(sl.byte_len, 24);
    assert_eq!(sl.str, "Здравствуйте");
}

#[test]
fn test_slice_utf8_string_start_out_of_bounds() {
    let s = utf8::make_utf8_string(MIXED);
    let sl = utf8::slice_utf8_string(s, 1000, 1);
    assert_eq!(sl.byte_len, 0);
    assert_eq!(sl.str, "");
}

#[test]
fn test_slice_utf8_string_end_out_of_bounds() {
    let s = utf8::make_utf8_string(MIXED);
    let sl = utf8::slice_utf8_string(s, 6, 1000);
    assert_eq!(sl.byte_len, 49);
    assert_eq!(sl.str, "Здравствуйте こんにちは 🚩😁");
}

#[test]
fn test_slice_utf8_string_start_non_boundary() {
    let s = utf8::make_utf8_string(MIXED);
    let sl = utf8::slice_utf8_string(s, 7, 3);
    // C returns NULL; Rust returns empty string
    assert_eq!(sl.str, "");
    assert_eq!(sl.byte_len, 0);
}

#[test]
fn test_slice_utf8_string_end_non_boundary() {
    let s = utf8::make_utf8_string(MIXED);
    let sl = utf8::slice_utf8_string(s, 6, 3);
    // C returns NULL; Rust returns empty string
    assert_eq!(sl.str, "");
    assert_eq!(sl.byte_len, 0);
}

#[test]
fn test_utf8_char_iter() {
    let s = utf8::make_utf8_string("Hдこ😁".as_bytes());
    let mut iter = utf8::make_utf8_char_iter(s);

    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 1);
    assert_eq!(ch.str, "H");

    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 2);
    assert_eq!(ch.str, "д");

    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 3);
    assert_eq!(ch.str, "こ");

    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 4);
    assert_eq!(ch.str, "😁");

    // exhausted
    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");

    // stays exhausted
    let ch = utf8::next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

#[test]
fn test_utf8_char_count() {
    let s = utf8::make_utf8_string(MIXED);
    assert_eq!(utf8::utf8_char_count(s), 27);
}

#[test]
fn test_utf8_char_count_empty() {
    let s = utf8::make_utf8_string(b"");
    assert_eq!(utf8::utf8_char_count(s), 0);
}

#[test]
fn test_is_utf8_char_boundary() {
    // "Hдこ😁" bytes: H(1) д(2) こ(3) 😁(4) = 10 bytes + null
    let bytes = "Hдこ😁".as_bytes();
    assert!(utf8::is_utf8_char_boundary(&bytes[0..]));  // H
    assert!(utf8::is_utf8_char_boundary(&bytes[1..]));  // д start
    assert!(!utf8::is_utf8_char_boundary(&bytes[2..])); // д cont
    assert!(utf8::is_utf8_char_boundary(&bytes[3..]));  // こ start
    assert!(!utf8::is_utf8_char_boundary(&bytes[4..])); // こ cont
    assert!(!utf8::is_utf8_char_boundary(&bytes[5..])); // こ cont
    assert!(utf8::is_utf8_char_boundary(&bytes[6..]));  // 😁 start
    assert!(!utf8::is_utf8_char_boundary(&bytes[7..])); // 😁 cont
    assert!(!utf8::is_utf8_char_boundary(&bytes[8..])); // 😁 cont
    assert!(!utf8::is_utf8_char_boundary(&bytes[9..])); // 😁 cont
    assert!(utf8::is_utf8_char_boundary(&bytes[10..])); // empty = boundary
}

#[test]
fn test_nth_utf8_char_first() {
    let s = utf8::make_utf8_string(MIXED);
    let ch = utf8::nth_utf8_char(s, 0);
    assert_eq!(ch.byte_len, 1);
    assert_eq!(ch.str, "H");
}

#[test]
fn test_nth_utf8_char_middle() {
    let s = utf8::make_utf8_string(MIXED);
    let ch = utf8::nth_utf8_char(s, 20);
    assert_eq!(ch.byte_len, 3);
    assert_eq!(ch.str, "ん");
}

#[test]
fn test_nth_utf8_char_last() {
    let s = utf8::make_utf8_string(MIXED);
    let ch = utf8::nth_utf8_char(s, 26);
    assert_eq!(ch.byte_len, 4);
    assert_eq!(ch.str, "😁");
}

#[test]
fn test_nth_utf8_char_out_of_bounds() {
    let s = utf8::make_utf8_string(MIXED);
    let ch = utf8::nth_utf8_char(s, 100);
    // C returns NULL; Rust returns empty string
    assert_eq!(ch.str, "");
    assert_eq!(ch.byte_len, 0);
}

#[test]
fn test_nth_utf8_char_empty_string() {
    let s = utf8::make_utf8_string(b"");
    let ch = utf8::nth_utf8_char(s, 0);
    assert_eq!(ch.str, "");
    assert_eq!(ch.byte_len, 0);
}

#[test]
fn test_unicode_code_point() {
    let s = utf8::make_utf8_string("Hдこ😁".as_bytes());
    let mut iter = utf8::make_utf8_char_iter(s);

    assert_eq!(utf8::unicode_code_point(utf8::next_utf8_char(&mut iter)), 72);
    assert_eq!(utf8::unicode_code_point(utf8::next_utf8_char(&mut iter)), 1076);
    assert_eq!(utf8::unicode_code_point(utf8::next_utf8_char(&mut iter)), 12371);
    assert_eq!(utf8::unicode_code_point(utf8::next_utf8_char(&mut iter)), 128513);
}

fn main() {}
