use utf8::utf8::*;

// "Hello Здравствуйте こんにちは 🚩😁"
// 5 ascii + 1 space + 12*2 russian + 1 space + 5*3 japanese + 1 space + 2*4 emoji = 54 bytes
// 5 + 1 + 12 + 1 + 5 + 1 + 2 = 27 chars
const MIXED: &str = "Hello Здравствуйте こんにちは 🚩😁";
const MIXED_BYTES: usize = 5 + 1 + 12 * 2 + 1 + 5 * 3 + 1 + 2 * 4; // 54

// --- validate_utf8 ---

#[test]
fn test_validate_utf8_ok() {
    let v = validate_utf8(MIXED.as_bytes());
    assert!(v.valid);
    assert_eq!(v.valid_upto, MIXED_BYTES);
}

#[test]
fn test_validate_utf8_empty() {
    let v = validate_utf8(b"");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 0);
}

#[test]
fn test_validate_utf8_boundary_ok() {
    // last 1b
    let v = validate_utf8(b"\x7F");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 1);

    // first 2b
    let v = validate_utf8(b"\xC2\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // last 2b
    let v = validate_utf8(b"\xDF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // first 3b
    let v = validate_utf8(b"\xE0\xA0\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // last 3b
    let v = validate_utf8(b"\xEF\xBF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // first 4b
    let v = validate_utf8(b"\xF0\x90\x80\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 4);

    // last 4b
    let v = validate_utf8(b"\xF7\xBF\xBF\xBF");
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
        let v = validate_utf8(seq);
        assert!(!v.valid);
        assert_eq!(v.valid_upto, 0);
    }
}

#[test]
fn test_validate_utf8_err() {
    // "Hello Здравствуйте" then \xC0\xC0 then more
    let mut bytes = Vec::new();
    bytes.extend_from_slice("Hello Здравствуйте".as_bytes());
    bytes.extend_from_slice(b"\xC0\xC0");
    bytes.extend_from_slice(" こんにちは 🚩😁".as_bytes());
    let v = validate_utf8(&bytes);
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 5 + 1 + 12 * 2);
}

#[test]
fn test_validate_utf8_overlong_encoding_err() {
    // 2-byte overlong for 'H' (U+0048)
    let v = validate_utf8(b"\xC1\x88");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // 3-byte overlong for 'H'
    let v = validate_utf8(b"\xE0\x81\x88");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // 4-byte overlong for 'H'
    let v = validate_utf8(b"\xF0\x80\x81\x88");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // 3-byte overlong for 'д' (U+0434)
    let v = validate_utf8(b"\xE0\x90\xB4");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // 4-byte overlong for 'д'
    let v = validate_utf8(b"\xF0\x80\x90\xB4");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // 4-byte overlong for 'こ' (U+3053)
    let v = validate_utf8(b"\xF0\x83\x81\x93");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    // boundary overlongs
    let v = validate_utf8(b"\xC1\xBF"); // last 1b overlong
    assert!(!v.valid);

    let v = validate_utf8(b"\xE0\x9F\xBF"); // last 2b overlong
    assert!(!v.valid);

    let v = validate_utf8(b"\xF0\x8F\xBF\xBF"); // last 3b overlong
    assert!(!v.valid);
}

// --- make_utf8_string ---

#[test]
fn test_make_utf8_string_ok() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    assert_eq!(ustr.byte_len, MIXED_BYTES);
    assert_eq!(ustr.str, MIXED);
}

#[test]
fn test_make_utf8_string_err() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice("Hello Здравствуйте".as_bytes());
    bytes.extend_from_slice(b"\xC0\xC0");
    bytes.extend_from_slice(" こんにちは 🚩😁".as_bytes());
    let ustr = make_utf8_string(&bytes);
    assert!(ustr.str.is_empty());
    assert_eq!(ustr.byte_len, 0);
}

#[test]
fn test_make_utf8_string_empty() {
    let ustr = make_utf8_string(b"");
    assert_eq!(ustr.str, "");
    assert_eq!(ustr.byte_len, 0);
}

// --- make_utf8_string_lossy ---

#[test]
fn test_make_utf8_string_lossy_ok() {
    let owned = make_utf8_string_lossy(MIXED.as_bytes());
    assert_eq!(owned.byte_len, MIXED_BYTES);
    assert_eq!(owned.str, MIXED);
}

#[test]
fn test_make_utf8_string_lossy_invalid_sequence() {
    let mut input = Vec::new();
    // "\xC0He\xC0llo Здр\xC0авствуйте\xC0\xC0 こんに\xC0\xC0\xC0\xC0ちは 🚩\xC0😁\xC0"
    input.push(0xC0u8);
    input.extend_from_slice(b"He");
    input.push(0xC0);
    input.extend_from_slice("llo Здр".as_bytes());
    input.push(0xC0);
    input.extend_from_slice("авствуйте".as_bytes());
    input.extend_from_slice(b"\xC0\xC0");
    input.extend_from_slice(" こんに".as_bytes());
    input.extend_from_slice(b"\xC0\xC0\xC0\xC0");
    input.extend_from_slice("ちは 🚩".as_bytes());
    input.push(0xC0);
    input.extend_from_slice("😁".as_bytes());
    input.push(0xC0);

    let expected = "\u{FFFD}He\u{FFFD}llo Здр\u{FFFD}авствуйте\u{FFFD}\u{FFFD} こんに\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}ちは 🚩\u{FFFD}😁\u{FFFD}";
    let owned = make_utf8_string_lossy(&input);
    assert_eq!(owned.byte_len, expected.len());
    assert_eq!(owned.str, expected);
}

#[test]
fn test_make_utf8_string_lossy_completely_invalid() {
    let owned = make_utf8_string_lossy(b"\xC0\xC0\xC0\xC0");
    let expected = "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
    assert_eq!(owned.byte_len, expected.len());
    assert_eq!(owned.str, expected);
}

#[test]
fn test_make_utf8_string_lossy_empty() {
    let owned = make_utf8_string_lossy(b"");
    assert_eq!(owned.str, "");
    assert_eq!(owned.byte_len, 0);
}

// --- as_utf8_string ---

#[test]
fn test_as_utf8_string() {
    let owned = make_utf8_string_lossy(MIXED.as_bytes());
    let ustr = as_utf8_string(&owned);
    assert_eq!(ustr.str, owned.str);
    assert_eq!(ustr.byte_len, owned.byte_len);
}

// --- free_owned_utf8_string ---

#[test]
fn test_free_owned_utf8_string() {
    let mut owned = make_utf8_string_lossy(MIXED.as_bytes());
    free_owned_utf8_string(&mut owned);
    assert!(owned.str.is_empty());
    assert_eq!(owned.byte_len, 0);
}

// --- slice_utf8_string ---

#[test]
fn test_slice_utf8_string_ok() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 6, 24);
    assert_eq!(slice.byte_len, 12 * 2);
    assert_eq!(slice.str, "Здравствуйте");
}

#[test]
fn test_slice_start_out_of_bounds() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 1000, 1);
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

#[test]
fn test_slice_end_out_of_bounds() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 6, 1000);
    assert_eq!(slice.byte_len, 12 * 2 + 1 + 5 * 3 + 1 + 2 * 4);
    assert_eq!(slice.str, "Здравствуйте こんにちは 🚩😁");
}

#[test]
fn test_slice_start_non_boundary_err() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 7, 3);
    assert!(slice.str.is_empty());
    assert_eq!(slice.byte_len, 0);
}

#[test]
fn test_slice_end_non_boundary_err() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 6, 3);
    assert!(slice.str.is_empty());
    assert_eq!(slice.byte_len, 0);
}

#[test]
fn test_slice_empty_range() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let slice = slice_utf8_string(ustr, 0, 0);
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

// --- make_utf8_char_iter / next_utf8_char ---

#[test]
fn test_utf8_char_iter() {
    let ustr = make_utf8_string("Hдこ😁".as_bytes());
    let mut iter = make_utf8_char_iter(ustr);

    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 1);
    assert_eq!(ch.str, "H");

    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 2);
    assert_eq!(ch.str, "д");

    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 3);
    assert_eq!(ch.str, "こ");

    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 4);
    assert_eq!(ch.str, "😁");

    // exhausted - keeps returning empty
    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");

    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

#[test]
fn test_utf8_char_iter_empty() {
    let ustr = make_utf8_string(b"");
    let mut iter = make_utf8_char_iter(ustr);
    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

// --- utf8_char_count ---

#[test]
fn test_utf8_char_count_zero() {
    let ustr = make_utf8_string(b"");
    assert_eq!(utf8_char_count(ustr), 0);
}

#[test]
fn test_utf8_char_count_mixed() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    assert_eq!(utf8_char_count(ustr), 5 + 1 + 12 + 1 + 5 + 1 + 2);
}

// --- is_utf8_char_boundary ---

#[test]
fn test_is_utf8_char_boundary() {
    let s = "Hдこ😁";
    let bytes = s.as_bytes();
    // H at 0 - boundary
    assert!(is_utf8_char_boundary(&bytes[0..]));
    // д at 1 - boundary
    assert!(is_utf8_char_boundary(&bytes[1..]));
    // continuation at 2
    assert!(!is_utf8_char_boundary(&bytes[2..]));
    // こ at 3 - boundary
    assert!(is_utf8_char_boundary(&bytes[3..]));
    assert!(!is_utf8_char_boundary(&bytes[4..]));
    assert!(!is_utf8_char_boundary(&bytes[5..]));
    // 😁 at 6 - boundary
    assert!(is_utf8_char_boundary(&bytes[6..]));
    assert!(!is_utf8_char_boundary(&bytes[7..]));
    assert!(!is_utf8_char_boundary(&bytes[8..]));
    assert!(!is_utf8_char_boundary(&bytes[9..]));
    // end - boundary (empty slice)
    assert!(is_utf8_char_boundary(&bytes[10..]));
}

// --- nth_utf8_char ---

#[test]
fn test_nth_utf8_char_valid_index() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let ch = nth_utf8_char(ustr, 20);
    assert_eq!(ch.byte_len, 3);
    assert_eq!(ch.str, "ん");
}

#[test]
fn test_nth_utf8_char_first() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let ch = nth_utf8_char(ustr, 0);
    assert_eq!(ch.byte_len, 1);
    assert_eq!(ch.str, "H");
}

#[test]
fn test_nth_utf8_char_last() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let ch = nth_utf8_char(ustr, 26);
    assert_eq!(ch.byte_len, 4);
    assert_eq!(ch.str, "😁");
}

#[test]
fn test_nth_utf8_char_invalid_index() {
    let ustr = make_utf8_string(MIXED.as_bytes());
    let ch = nth_utf8_char(ustr, 100);
    assert!(ch.str.is_empty());
    assert_eq!(ch.byte_len, 0);
}

#[test]
fn test_nth_utf8_char_empty_string() {
    let ustr = make_utf8_string(b"");
    let ch = nth_utf8_char(ustr, 0);
    assert!(ch.str.is_empty());
    assert_eq!(ch.byte_len, 0);
}

// --- unicode_code_point ---

#[test]
fn test_unicode_code_point() {
    let ustr = make_utf8_string("Hдこ😁".as_bytes());
    let mut iter = make_utf8_char_iter(ustr);

    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 72);     // H
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 1076);   // д
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 12371);  // こ
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 128513); // 😁
}

#[test]
fn test_unicode_code_point_ascii() {
    let ustr = make_utf8_string(b"A");
    let ch = nth_utf8_char(ustr, 0);
    assert_eq!(unicode_code_point(ch), 65);
}

fn main() {}
