use utf8::utf8::{
    as_utf8_string, free_owned_utf8_string, is_utf8_char_boundary, make_utf8_char_iter,
    make_utf8_string, make_utf8_string_lossy, next_utf8_char, nth_utf8_char, slice_utf8_string,
    unicode_code_point, utf8_char_count, validate_utf8, validate_utf8_char, OwnedUtf8String,
    Utf8Char, Utf8CharIter, Utf8String,
};

// english characters are 1 byte each
// russian  2 bytes each
// japanese 3 bytes each
// 🚩 and 😁 is 4 bytes each

const FULL_STR: &[u8] = "Hello Здравствуйте こんにちは 🚩😁".as_bytes();
// 5 + 1 + 12*2 + 1 + 5*3 + 1 + 2*4 = 54
const FULL_STR_LEN: usize = 5 + 1 + 12 * 2 + 1 + 5 * 3 + 1 + 2 * 4;

// ===================== validate_utf8 =====================

#[test]
fn test_validate_utf8_ok() {
    let v = validate_utf8(FULL_STR);
    assert!(v.valid);
    assert_eq!(v.valid_upto, FULL_STR_LEN);
}

#[test]
fn test_validate_utf8_empty() {
    let v = validate_utf8(b"");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 0);
}

#[test]
fn test_validate_utf8_boundary_ok() {
    // last 1b -> 0(1111111)
    let v = validate_utf8(b"\x7F");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 1);

    // first 2b -> 110(00010) 10(000000)
    let v = validate_utf8(b"\xC2\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // last 2b -> 110(11111) 10(111111)
    let v = validate_utf8(b"\xDF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 2);

    // first 3b -> 1110(0000) 10(100000) 10(000000)
    let v = validate_utf8(b"\xE0\xA0\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // last 3b -> 1110(1111) 10(111111) 10(111111)
    let v = validate_utf8(b"\xEF\xBF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 3);

    // first 4b -> 11110(000) 10(010000) 10(000000) 10(000000)
    let v = validate_utf8(b"\xF0\x90\x80\x80");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 4);

    // last 4b (per C definition, allows >0x10FFFF range)
    // -> 11110(111) 10(111111) 10(111111) 10(111111)
    let v = validate_utf8(b"\xF7\xBF\xBF\xBF");
    assert!(v.valid);
    assert_eq!(v.valid_upto, 4);
}

#[test]
fn test_surrogate_rejection() {
    let v = validate_utf8(b"\xED\xA0\x80");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    let v = validate_utf8(b"\xED\xAC\x80");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    let v = validate_utf8(b"\xED\xA0\x8C");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);

    let v = validate_utf8(b"\xED\xBF\xBF");
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 0);
}

#[test]
fn test_validate_utf8_err() {
    // "Hello Здравствуйте\xC0\xC0 こんにちは 🚩😁"
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice("Hello Здравствуйте".as_bytes());
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.extend_from_slice(" こんにちは 🚩😁".as_bytes());
    let v = validate_utf8(&bytes);
    assert!(!v.valid);
    assert_eq!(v.valid_upto, 5 + 1 + 12 * 2);
}

#[test]
fn test_validate_utf8_overlong_encoding_err() {
    // Each is invalid overlong encoding
    let cases: &[&[u8]] = &[
        b"\xC1\x88",         // overlong 2b for 'H'
        b"\xE0\x81\x88",     // overlong 3b for 'H'
        b"\xF0\x80\x81\x88", // overlong 4b for 'H'
        b"\xE0\x90\xB4",     // overlong 3b for 'д'
        b"\xF0\x80\x90\xB4", // overlong 4b for 'д'
        b"\xF0\x83\x81\x93", // overlong 4b for 'こ'
        b"\xC1\xBF",         // last 1b overlong
        b"\xE0\x9F\xBF",     // last 2b overlong
        b"\xF0\x8F\xBF\xBF", // last 3b overlong
    ];
    for c in cases {
        let v = validate_utf8(c);
        assert!(!v.valid, "expected invalid for {:?}", c);
        assert_eq!(v.valid_upto, 0, "expected valid_upto=0 for {:?}", c);
    }
}

// ===================== validate_utf8_char =====================

#[test]
fn test_validate_utf8_char_single_byte() {
    let v = validate_utf8_char(b"H", 0);
    assert!(v.valid);
    assert_eq!(v.next_offset, 1);
}

#[test]
fn test_validate_utf8_char_two_byte() {
    // "д" = 0xD0 0xB4
    let v = validate_utf8_char(b"\xD0\xB4", 0);
    assert!(v.valid);
    assert_eq!(v.next_offset, 2);
}

#[test]
fn test_validate_utf8_char_three_byte() {
    // "こ" = 0xE3 0x81 0x93
    let v = validate_utf8_char(b"\xE3\x81\x93", 0);
    assert!(v.valid);
    assert_eq!(v.next_offset, 3);
}

#[test]
fn test_validate_utf8_char_four_byte() {
    // "😁" = 0xF0 0x9F 0x98 0x81
    let v = validate_utf8_char(b"\xF0\x9F\x98\x81", 0);
    assert!(v.valid);
    assert_eq!(v.next_offset, 4);
}

#[test]
fn test_validate_utf8_char_overlong_two_byte() {
    let v = validate_utf8_char(b"\xC1\x88", 0);
    assert!(!v.valid);
    assert_eq!(v.next_offset, 0);
}

#[test]
fn test_validate_utf8_char_overlong_three_byte() {
    let v = validate_utf8_char(b"\xE0\x81\x88", 0);
    assert!(!v.valid);
    assert_eq!(v.next_offset, 0);
}

#[test]
fn test_validate_utf8_char_overlong_four_byte() {
    let v = validate_utf8_char(b"\xF0\x80\x81\x88", 0);
    assert!(!v.valid);
    assert_eq!(v.next_offset, 0);
}

#[test]
fn test_validate_utf8_char_surrogate() {
    let v = validate_utf8_char(b"\xED\xA0\x80", 0);
    assert!(!v.valid);
    assert_eq!(v.next_offset, 0);
}

#[test]
fn test_validate_utf8_char_at_offset() {
    // "Hд" = 0x48 0xD0 0xB4
    let v = validate_utf8_char(b"H\xD0\xB4", 1);
    assert!(v.valid);
    assert_eq!(v.next_offset, 3);
}

#[test]
fn test_validate_utf8_char_invalid_byte() {
    // 0xC0 not a valid start of 2-byte sequence (overlong)
    let v = validate_utf8_char(b"\xC0\xC0", 0);
    assert!(!v.valid);
    assert_eq!(v.next_offset, 0);
}

// ===================== make_utf8_string =====================

#[test]
fn test_make_utf8_string_ok() {
    let ustr = make_utf8_string(FULL_STR);
    assert_eq!(ustr.byte_len, FULL_STR_LEN);
    assert_eq!(ustr.str.as_bytes(), FULL_STR);
}

#[test]
fn test_make_utf8_string_err() {
    let mut bytes: Vec<u8> = Vec::new();
    bytes.extend_from_slice("Hello Здравствуйте".as_bytes());
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.extend_from_slice(" こんにちは 🚩😁".as_bytes());
    let ustr = make_utf8_string(&bytes);
    // C returns NULL str + byte_len 0; Rust returns empty string + byte_len 0
    assert_eq!(ustr.byte_len, 0);
    assert!(ustr.str.is_empty());
}

#[test]
fn test_make_utf8_string_empty() {
    let ustr = make_utf8_string(b"");
    assert_eq!(ustr.byte_len, 0);
    assert!(ustr.str.is_empty());
}

// ===================== make_utf8_string_lossy =====================

#[test]
fn test_make_utf8_string_lossy_ok() {
    let owned = make_utf8_string_lossy(FULL_STR);
    assert_eq!(owned.byte_len, FULL_STR_LEN);
    assert_eq!(owned.str.as_bytes(), FULL_STR);
}

#[test]
fn test_make_utf8_string_lossy_invalid_sequence() {
    // "\xC0He\xC0llo Здр\xC0авствуйте\xC0\xC0 こんに\xC0\xC0\xC0\xC0ちは 🚩\xC0😁\xC0"
    let mut bytes: Vec<u8> = Vec::new();
    bytes.push(0xC0);
    bytes.extend_from_slice(b"He");
    bytes.push(0xC0);
    bytes.extend_from_slice("llo Здр".as_bytes());
    bytes.push(0xC0);
    bytes.extend_from_slice("авствуйте".as_bytes());
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.extend_from_slice(" こんに".as_bytes());
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.push(0xC0);
    bytes.extend_from_slice("ちは 🚩".as_bytes());
    bytes.push(0xC0);
    bytes.extend_from_slice("😁".as_bytes());
    bytes.push(0xC0);

    let expected = "\u{FFFD}He\u{FFFD}llo Здр\u{FFFD}авствуйте\u{FFFD}\u{FFFD} こんに\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}ちは 🚩\u{FFFD}😁\u{FFFD}";

    let owned = make_utf8_string_lossy(&bytes);
    assert_eq!(owned.byte_len, expected.len());
    assert_eq!(owned.str, expected);
}

#[test]
fn test_make_utf8_string_lossy_completely_invalid() {
    let bytes: &[u8] = &[0xC0, 0xC0, 0xC0, 0xC0];
    let expected = "\u{FFFD}\u{FFFD}\u{FFFD}\u{FFFD}";
    let owned = make_utf8_string_lossy(bytes);
    assert_eq!(owned.byte_len, expected.len());
    assert_eq!(owned.str, expected);
}

#[test]
fn test_make_utf8_string_lossy_empty() {
    let owned = make_utf8_string_lossy(b"");
    assert_eq!(owned.byte_len, 0);
    assert_eq!(owned.str, "");
}

// ===================== as_utf8_string =====================

#[test]
fn test_as_utf8_string() {
    let owned = OwnedUtf8String {
        str: "hello".to_string(),
        byte_len: 5,
    };
    let view = as_utf8_string(&owned);
    assert_eq!(view.str, "hello");
    assert_eq!(view.byte_len, 5);
}

#[test]
fn test_as_utf8_string_empty() {
    let owned = OwnedUtf8String {
        str: String::new(),
        byte_len: 0,
    };
    let view = as_utf8_string(&owned);
    assert_eq!(view.str, "");
    assert_eq!(view.byte_len, 0);
}

// ===================== free_owned_utf8_string =====================

#[test]
fn test_free_owned_utf8_string() {
    let mut owned = OwnedUtf8String {
        str: "hello".to_string(),
        byte_len: 5,
    };
    free_owned_utf8_string(&mut owned);
    assert_eq!(owned.str, "");
    assert_eq!(owned.byte_len, 0);
}

#[test]
fn test_free_owned_utf8_string_already_empty() {
    let mut owned = OwnedUtf8String {
        str: String::new(),
        byte_len: 0,
    };
    free_owned_utf8_string(&mut owned);
    assert_eq!(owned.str, "");
    assert_eq!(owned.byte_len, 0);
}

// ===================== slice_utf8_string =====================

#[test]
fn test_make_utf8_string_slice_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let slice = slice_utf8_string(ustr, 6, 24);
    assert_eq!(slice.byte_len, 12 * 2);
    assert_eq!(slice.str, "Здравствуйте");
}

#[test]
fn test_make_utf8_string_slice_start_out_of_bounds_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let slice = slice_utf8_string(ustr, 1000, 1);
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

#[test]
fn test_make_utf8_string_slice_end_out_of_bounds_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let slice = slice_utf8_string(ustr, 6, 1000);
    assert_eq!(slice.byte_len, 12 * 2 + 1 + 5 * 3 + 1 + 2 * 4);
    assert_eq!(slice.str, "Здравствуйте こんにちは 🚩😁");
}

#[test]
fn test_make_utf8_string_slice_start_non_boundary_err() {
    let ustr = make_utf8_string(FULL_STR);
    let slice = slice_utf8_string(ustr, 7, 3);
    // C returns NULL, Rust returns empty
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

#[test]
fn test_make_utf8_string_slice_end_non_boundary_err() {
    let ustr = make_utf8_string(FULL_STR);
    let slice = slice_utf8_string(ustr, 6, 3);
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

#[test]
fn test_make_utf8_string_slice_full_range() {
    let ustr = make_utf8_string(b"hello");
    let slice = slice_utf8_string(ustr, 0, 5);
    assert_eq!(slice.byte_len, 5);
    assert_eq!(slice.str, "hello");
}

#[test]
fn test_make_utf8_string_slice_zero_length() {
    let ustr = make_utf8_string(b"hello");
    let slice = slice_utf8_string(ustr, 0, 0);
    assert_eq!(slice.byte_len, 0);
    assert_eq!(slice.str, "");
}

// ===================== make_utf8_char_iter / next_utf8_char =====================

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

    // After exhaustion, returns empty
    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");

    // Keeps returning empty
    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

#[test]
fn test_make_utf8_char_iter_holds_string() {
    let ustr = Utf8String {
        str: "abc".to_string(),
        byte_len: 3,
    };
    let iter = make_utf8_char_iter(ustr);
    assert_eq!(iter.str, "abc");
}

#[test]
fn test_next_utf8_char_empty_iterator() {
    let mut iter = Utf8CharIter {
        str: String::new(),
    };
    let ch = next_utf8_char(&mut iter);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

// ===================== utf8_char_count =====================

#[test]
fn test_utf8_char_count_zero() {
    let ustr = make_utf8_string(b"");
    assert_eq!(utf8_char_count(ustr), 0);
}

#[test]
fn test_utf8_char_count_full() {
    let ustr = make_utf8_string(FULL_STR);
    assert_eq!(utf8_char_count(ustr), 5 + 1 + 12 + 1 + 5 + 1 + 2);
}

#[test]
fn test_utf8_char_count_simple() {
    let ustr = make_utf8_string("Hдこ😁".as_bytes());
    assert_eq!(utf8_char_count(ustr), 4);
}

// ===================== is_utf8_char_boundary =====================

#[test]
fn test_is_utf8_char_boundary() {
    let bytes = "Hдこ😁".as_bytes();
    // H -> 0x48 (1b)
    assert!(is_utf8_char_boundary(&bytes[0..]));
    // д -> 0xD0 0xB4 (2b)
    assert!(is_utf8_char_boundary(&bytes[1..]));
    assert!(!is_utf8_char_boundary(&bytes[2..]));
    // こ -> 0xE3 0x81 0x93 (3b)
    assert!(is_utf8_char_boundary(&bytes[3..]));
    assert!(!is_utf8_char_boundary(&bytes[4..]));
    assert!(!is_utf8_char_boundary(&bytes[5..]));
    // 😁 -> 0xF0 0x9F 0x98 0x81 (4b)
    assert!(is_utf8_char_boundary(&bytes[6..]));
    assert!(!is_utf8_char_boundary(&bytes[7..]));
    assert!(!is_utf8_char_boundary(&bytes[8..]));
    assert!(!is_utf8_char_boundary(&bytes[9..]));
    // End of string (empty slice == '\0' in C, true)
    assert!(is_utf8_char_boundary(&bytes[10..]));
}

#[test]
fn test_is_utf8_char_boundary_empty() {
    assert!(is_utf8_char_boundary(b""));
}

#[test]
fn test_is_utf8_char_boundary_ascii() {
    assert!(is_utf8_char_boundary(b"a"));
    assert!(is_utf8_char_boundary(b"\x00"));
    assert!(is_utf8_char_boundary(b"\x7F"));
}

#[test]
fn test_is_utf8_char_boundary_continuation() {
    // 0x80, 0xBF are continuation bytes — NOT boundaries
    assert!(!is_utf8_char_boundary(&[0x80][..]));
    assert!(!is_utf8_char_boundary(&[0xBF][..]));
}

#[test]
fn test_is_utf8_char_boundary_lead_bytes() {
    // 0xC0..=0xFF are lead bytes — boundaries
    assert!(is_utf8_char_boundary(&[0xC0][..]));
    assert!(is_utf8_char_boundary(&[0xFF][..]));
}

// ===================== nth_utf8_char =====================

#[test]
fn test_nth_utf8_char_valid_index_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let ch = nth_utf8_char(ustr, 20);
    assert_eq!(ch.byte_len, 3);
    assert_eq!(ch.str, "ん");
}

#[test]
fn test_nth_utf8_char_first_index_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let ch = nth_utf8_char(ustr, 0);
    assert_eq!(ch.byte_len, 1);
    assert_eq!(ch.str, "H");
}

#[test]
fn test_nth_utf8_char_last_index_ok() {
    let ustr = make_utf8_string(FULL_STR);
    let ch = nth_utf8_char(ustr, 26);
    assert_eq!(ch.byte_len, 4);
    assert_eq!(ch.str, "😁");
}

#[test]
fn test_nth_utf8_char_invalid_index_err() {
    let ustr = make_utf8_string(FULL_STR);
    let ch = nth_utf8_char(ustr, 100);
    // C returns NULL str, Rust returns empty
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

#[test]
fn test_nth_utf8_char_empty_string_err() {
    let ustr = make_utf8_string(b"");
    let ch = nth_utf8_char(ustr, 0);
    assert_eq!(ch.byte_len, 0);
    assert_eq!(ch.str, "");
}

// ===================== unicode_code_point =====================

#[test]
fn test_unicode_code_point_h() {
    let ch = Utf8Char {
        str: "H".to_string(),
        byte_len: 1,
    };
    assert_eq!(unicode_code_point(ch), 72);
}

#[test]
fn test_unicode_code_point_d() {
    let ch = Utf8Char {
        str: "д".to_string(),
        byte_len: 2,
    };
    assert_eq!(unicode_code_point(ch), 1076);
}

#[test]
fn test_unicode_code_point_ko() {
    let ch = Utf8Char {
        str: "こ".to_string(),
        byte_len: 3,
    };
    assert_eq!(unicode_code_point(ch), 12371);
}

#[test]
fn test_unicode_code_point_emoji() {
    let ch = Utf8Char {
        str: "😁".to_string(),
        byte_len: 4,
    };
    assert_eq!(unicode_code_point(ch), 128513);
}

#[test]
fn test_unicode_code_point_via_iterator() {
    let ustr = make_utf8_string("Hдこ😁".as_bytes());
    let mut iter = make_utf8_char_iter(ustr);
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 72);
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 1076);
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 12371);
    assert_eq!(unicode_code_point(next_utf8_char(&mut iter)), 128513);
}

#[test]
fn test_unicode_code_point_boundaries() {
    // last 1b = 0x7F -> 127
    let ch = Utf8Char {
        str: "\u{7F}".to_string(),
        byte_len: 1,
    };
    assert_eq!(unicode_code_point(ch), 127);

    // first 2b -> U+0080
    let ch = Utf8Char {
        str: "\u{80}".to_string(),
        byte_len: 2,
    };
    assert_eq!(unicode_code_point(ch), 0x80);

    // last 2b -> U+07FF
    let ch = Utf8Char {
        str: "\u{7FF}".to_string(),
        byte_len: 2,
    };
    assert_eq!(unicode_code_point(ch), 0x7FF);

    // first 3b -> U+0800
    let ch = Utf8Char {
        str: "\u{800}".to_string(),
        byte_len: 3,
    };
    assert_eq!(unicode_code_point(ch), 0x800);

    // last 3b -> U+FFFF
    let ch = Utf8Char {
        str: "\u{FFFF}".to_string(),
        byte_len: 3,
    };
    assert_eq!(unicode_code_point(ch), 0xFFFF);

    // first 4b -> U+10000
    let ch = Utf8Char {
        str: "\u{10000}".to_string(),
        byte_len: 4,
    };
    assert_eq!(unicode_code_point(ch), 0x10000);

    // last legal 4b -> U+10FFFF
    let ch = Utf8Char {
        str: "\u{10FFFF}".to_string(),
        byte_len: 4,
    };
    assert_eq!(unicode_code_point(ch), 0x10FFFF);
}

// ===================== integration: lossy round trip =====================

#[test]
fn test_lossy_then_as_utf8_string_then_count() {
    let owned = make_utf8_string_lossy("Hдこ😁".as_bytes());
    let view = as_utf8_string(&owned);
    assert_eq!(view.byte_len, 1 + 2 + 3 + 4);
    let count = utf8_char_count(view);
    assert_eq!(count, 4);
}

fn main() {}
