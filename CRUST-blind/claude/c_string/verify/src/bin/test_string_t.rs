#![allow(unused_imports, dead_code)]

use c_string::string_t::{
    new_string, new_string_from_bytes, string_bytes, string_concat, string_copy, string_endswith,
    string_eq, string_find, string_join_arr, string_len, string_split, string_split_by,
    string_startswith, string_strip, string_substr, string_t_is_space_char, StringT,
    STRING_T_INDEXES_BUFFER_SIZE, STRING_T_SPACE_CHARS_ARR,
};

// ---------- Helpers ----------

fn make(b: &str) -> StringT {
    new_string_from_bytes(b)
}

// ---------- Constants ----------

#[test]
fn test_constants() {
    assert_eq!(STRING_T_INDEXES_BUFFER_SIZE, 512);
    assert_eq!(STRING_T_SPACE_CHARS_ARR, " \t\n\r");
}

// ---------- new_string ----------

#[test]
fn test_new_string_sizes() {
    let sizes = [0usize, 1, 2, 3, 12, 100];
    for &s in sizes.iter() {
        let str_o = new_string(s);
        assert_eq!(str_o.size, s);
        assert_eq!(str_o.bytes.len(), s);
        // Should be zero-initialized
        for b in &str_o.bytes {
            assert_eq!(*b, 0);
        }
    }
}

// ---------- new_string_from_bytes ----------

#[test]
fn test_new_string_from_bytes_basic() {
    let cases = ["", "test", "some another test"];
    let lens: [usize; 3] = [0, 4, 17];
    for (i, &b) in cases.iter().enumerate() {
        let s = new_string_from_bytes(b);
        assert_eq!(s.size, lens[i]);
        assert_eq!(s.bytes.len(), lens[i]);
        assert_eq!(&s.bytes[..], b.as_bytes());
    }
}

// ---------- string_len ----------

#[test]
fn test_string_len_basic() {
    let cases = ["", "test", "some another test"];
    let lens: [usize; 3] = [0, 4, 17];
    for (i, &b) in cases.iter().enumerate() {
        let s = new_string_from_bytes(b);
        assert_eq!(string_len(&s), lens[i]);
    }
}

// ---------- string_bytes ----------

#[test]
fn test_string_bytes_basic() {
    let cases = ["", "test", "some another test"];
    for &b in cases.iter() {
        let s = new_string_from_bytes(b);
        assert_eq!(string_bytes(&s), b);
    }
}

// ---------- string_eq ----------

#[test]
fn test_string_eq_equal_strings() {
    let cases = ["", "\n\n", "test", "some another test"];
    for &b in cases.iter() {
        let l = make(b);
        let r = make(b);
        assert!(string_eq(&l, &r));
    }
}

#[test]
fn test_string_eq_different_length() {
    let l = make("abc");
    let r = make("abcd");
    assert!(!string_eq(&l, &r));
}

#[test]
fn test_string_eq_same_length_different_bytes() {
    let l = make("abc");
    let r = make("abd");
    assert!(!string_eq(&l, &r));
}

#[test]
fn test_string_eq_both_empty() {
    let l = make("");
    let r = make("");
    assert!(string_eq(&l, &r));
}

// ---------- string_copy ----------

#[test]
fn test_string_copy_basic() {
    let cases = ["", "test", "some another test"];
    for &b in cases.iter() {
        let s = make(b);
        let c = string_copy(&s);
        assert_eq!(c.size, s.size);
        assert!(string_eq(&s, &c));
        assert_eq!(&c.bytes[..], b.as_bytes());
    }
}

// ---------- string_concat ----------

#[test]
fn test_string_concat_basic() {
    let first = ["", "first", "some another"];
    let second = ["", "second", " test"];
    let result = ["", "firstsecond", "some another test"];
    for i in 0..3 {
        let f = make(first[i]);
        let s = make(second[i]);
        let r = string_concat(&f, &s);
        let expected = make(result[i]);
        assert_eq!(r.size, expected.size);
        assert!(string_eq(&r, &expected));
        assert_eq!(&r.bytes[..r.size], result[i].as_bytes());
    }
}

#[test]
fn test_string_concat_with_empty_lhs() {
    let f = make("");
    let s = make("hello");
    let r = string_concat(&f, &s);
    assert_eq!(r.size, 5);
    assert_eq!(&r.bytes[..], b"hello");
}

#[test]
fn test_string_concat_with_empty_rhs() {
    let f = make("hello");
    let s = make("");
    let r = string_concat(&f, &s);
    assert_eq!(r.size, 5);
    assert_eq!(&r.bytes[..], b"hello");
}

// ---------- string_substr ----------

#[test]
fn test_string_substr_basic() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  "];
    let pos: [usize; 4] = [0, 1, 5, 0];
    let len: [usize; 4] = [0, 2, 3, 5];
    let expected = ["", "fv", "str", " some"];

    for i in 0..4 {
        let s = make(bytes[i]);
        let sub = string_substr(&s, pos[i], len[i]);
        let exp = make(expected[i]);
        assert_eq!(sub.size, len[i]);
        assert!(string_eq(&sub, &exp));
        assert_eq!(&sub.bytes[..sub.size], expected[i].as_bytes());
    }
}

// ---------- string_startswith ----------

#[test]
fn test_string_startswith_cases() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  ", "1234"];
    let prefixes = ["", "vfv", "test string", " some", "2"];
    let expected = [true, true, true, true, false];

    for i in 0..5 {
        let s = make(bytes[i]);
        assert_eq!(string_startswith(&s, prefixes[i]), expected[i]);
    }
}

#[test]
fn test_string_startswith_longer_prefix_than_string() {
    let s = make("hi");
    assert!(!string_startswith(&s, "hello"));
}

// ---------- string_endswith ----------

#[test]
fn test_string_endswith_cases() {
    let bytes = ["", "vfv\n\n", "test string", " some another test  ", "1234"];
    let suffixes = ["", "fv\n\n", "test string", "test  ", "2"];
    let expected = [true, true, true, true, false];

    for i in 0..5 {
        let s = make(bytes[i]);
        assert_eq!(string_endswith(&s, suffixes[i]), expected[i]);
    }
}

#[test]
fn test_string_endswith_longer_suffix_than_string() {
    let s = make("hi");
    assert!(!string_endswith(&s, "hello"));
}

// ---------- string_find ----------

#[test]
fn test_string_find_basic() {
    let bytes = [
        "",
        "vfv\n\n",
        "test string",
        "test string",
        " some another test  ",
    ];
    let chars = ["", "\n", "no", "", "another"];
    // C returns int with -1 for not-found. Rust uses Option<usize>.
    let expected: [Option<usize>; 5] = [Some(0), Some(3), None, Some(0), Some(6)];

    for i in 0..5 {
        let s = make(bytes[i]);
        assert_eq!(string_find(&s, chars[i]), expected[i]);
    }
}

#[test]
fn test_string_find_at_end() {
    let s = make("hello world");
    assert_eq!(string_find(&s, "world"), Some(6));
    assert_eq!(string_find(&s, "d"), Some(10));
}

#[test]
fn test_string_find_full_string_match() {
    let s = make("hello");
    assert_eq!(string_find(&s, "hello"), Some(0));
}

#[test]
fn test_string_find_pattern_longer_than_string() {
    let s = make("hello");
    assert_eq!(string_find(&s, "helloa"), None);
}

#[test]
fn test_string_find_empty_pattern_returns_zero() {
    let s = make("hello");
    assert_eq!(string_find(&s, ""), Some(0));
}

// ---------- string_strip ----------

#[test]
fn test_string_strip_basic() {
    let inputs = ["", "vfv\n\n", "  test\t", " some another test  "];
    let stripped = ["", "vfv", "test", "some another test"];
    for i in 0..4 {
        let s = make(inputs[i]);
        let r = string_strip(&s);
        let exp = make(stripped[i]);
        assert_eq!(r.size, exp.size);
        assert!(string_eq(&r, &exp));
        assert_eq!(&r.bytes[..r.size], stripped[i].as_bytes());
    }
}

#[test]
fn test_string_strip_all_whitespace_returns_copy() {
    // Per C behavior: when start_pos >= end_pos, returns string_copy(str)
    let s = make("   ");
    let r = string_strip(&s);
    assert_eq!(r.size, 3);
    assert_eq!(&r.bytes[..], b"   ");
}

#[test]
fn test_string_strip_single_char() {
    // Per C behavior: start_pos=0, end_pos=0, start_pos >= end_pos so returns copy
    let s = make("a");
    let r = string_strip(&s);
    assert_eq!(r.size, 1);
    assert_eq!(&r.bytes[..], b"a");
}

#[test]
fn test_string_strip_two_chars_no_whitespace() {
    // Per C behavior: with "ab", start_pos=0, end_pos=1, 0 < 1 so substr(0, 2) = "ab"
    let s = make("ab");
    let r = string_strip(&s);
    assert_eq!(r.size, 2);
    assert_eq!(&r.bytes[..], b"ab");
}

#[test]
fn test_string_strip_leading_only() {
    // " a": start_pos=1, end_pos=1, 1 >= 1 so returns copy
    let s = make(" a");
    let r = string_strip(&s);
    assert_eq!(r.size, 2);
    assert_eq!(&r.bytes[..], b" a");
}

#[test]
fn test_string_strip_trailing_only() {
    // "a ": start_pos=0, end_pos=0, 0 >= 0 so returns copy
    let s = make("a ");
    let r = string_strip(&s);
    assert_eq!(r.size, 2);
    assert_eq!(&r.bytes[..], b"a ");
}

#[test]
fn test_string_strip_padded_single_char() {
    // " a ": start_pos=1, end_pos=1, 1 >= 1 so returns copy
    let s = make(" a ");
    let r = string_strip(&s);
    assert_eq!(r.size, 3);
    assert_eq!(&r.bytes[..], b" a ");
}

#[test]
fn test_string_strip_padded_multi_char() {
    // " ab ": start_pos=1, end_pos=2, 1 < 2 so substr(1, 2) = "ab"
    let s = make(" ab ");
    let r = string_strip(&s);
    assert_eq!(r.size, 2);
    assert_eq!(&r.bytes[..], b"ab");
}

// ---------- string_split ----------

#[test]
fn test_string_split_empty() {
    let s = make("");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
}

#[test]
fn test_string_split_single_char() {
    let s = make("1");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"1");
}

#[test]
fn test_string_split_no_whitespace() {
    let s = make("some");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 4);
    assert_eq!(&arr[0].bytes[..], b"some");
}

#[test]
fn test_string_split_with_leading_space() {
    // Per C behavior: " some string 124!" -> ["", "some", "string", "124!"]
    let s = make(" some string 124!");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 4);
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
    assert_eq!(arr[1].size, 4);
    assert_eq!(&arr[1].bytes[..], b"some");
    assert_eq!(arr[2].size, 6);
    assert_eq!(&arr[2].bytes[..], b"string");
    assert_eq!(arr[3].size, 4);
    assert_eq!(&arr[3].bytes[..], b"124!");
}

#[test]
fn test_string_split_complex() {
    let s = make("Some github account: vnkrtv");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 4);
    assert_eq!(arr.len(), 4);
    assert_eq!(&arr[0].bytes[..], b"Some");
    assert_eq!(&arr[1].bytes[..], b"github");
    assert_eq!(&arr[2].bytes[..], b"account:");
    assert_eq!(&arr[3].bytes[..], b"vnkrtv");
    assert_eq!(arr[0].size, 4);
    assert_eq!(arr[1].size, 6);
    assert_eq!(arr[2].size, 8);
    assert_eq!(arr[3].size, 6);
}

#[test]
fn test_string_split_only_whitespace() {
    // Per C behavior: "   " -> [""] (size 1)
    let s = make("   ");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
}

#[test]
fn test_string_split_mixed_whitespace() {
    let s = make("a\tb\nc d");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 4);
    assert_eq!(arr.len(), 4);
    assert_eq!(&arr[0].bytes[..], b"a");
    assert_eq!(&arr[1].bytes[..], b"b");
    assert_eq!(&arr[2].bytes[..], b"c");
    assert_eq!(&arr[3].bytes[..], b"d");
    assert_eq!(arr[0].size, 1);
    assert_eq!(arr[1].size, 1);
    assert_eq!(arr[2].size, 1);
    assert_eq!(arr[3].size, 1);
}

#[test]
fn test_string_split_double_space_pads() {
    // Per C: "  hello  " -> ["", "hello"]  (trailing whitespace fully consumed)
    let s = make("  hello  ");
    let mut sz: usize = 0;
    let arr = string_split(&s, &mut sz);
    assert_eq!(sz, 2);
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
    assert_eq!(arr[1].size, 5);
    assert_eq!(&arr[1].bytes[..], b"hello");
}

// ---------- string_split_by ----------

#[test]
fn test_string_split_by_empty() {
    let s = make("");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "W");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
}

#[test]
fn test_string_split_by_single_char_no_match() {
    let s = make("1");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "W");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"1");
}

#[test]
fn test_string_split_by_no_match() {
    let s = make("some");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "W");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 4);
    assert_eq!(&arr[0].bytes[..], b"some");
}

#[test]
fn test_string_split_by_leading_split() {
    // Per C: "WsomeWstringW124!" -> ["", "some", "string", "124!"]
    let s = make("WsomeWstringW124!");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "W");
    assert_eq!(sz, 4);
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
    assert_eq!(arr[1].size, 4);
    assert_eq!(&arr[1].bytes[..], b"some");
    assert_eq!(arr[2].size, 6);
    assert_eq!(&arr[2].bytes[..], b"string");
    assert_eq!(arr[3].size, 4);
    assert_eq!(&arr[3].bytes[..], b"124!");
}

#[test]
fn test_string_split_by_complex() {
    let s = make("SomeWgithubWaccount:Wvnkrtv");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "W");
    assert_eq!(sz, 4);
    assert_eq!(arr.len(), 4);
    assert_eq!(arr[0].size, 4);
    assert_eq!(&arr[0].bytes[..], b"Some");
    assert_eq!(arr[1].size, 6);
    assert_eq!(&arr[1].bytes[..], b"github");
    assert_eq!(arr[2].size, 8);
    assert_eq!(&arr[2].bytes[..], b"account:");
    assert_eq!(arr[3].size, 6);
    assert_eq!(&arr[3].bytes[..], b"vnkrtv");
}

#[test]
fn test_string_split_by_two_segments() {
    // Per C: "aXbcXd" split by "X" - because of upper_bound bug,
    // loop doesn't reach final X. Let's check: size=6, split=1, upper=5.
    // pos=0:'a', pos=1:'X' match, indexes[0]=(0,1), start=2, pos=2
    // pos=2:'b', pos=3:'c', pos=4:'X' match, indexes[1]=(2,4), start=5, pos=5
    // exit loop, pos=5, start_pos=5, no extra push.
    // Result: ["a", "bc"]
    let s = make("aXbcXd");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "X");
    assert_eq!(sz, 2);
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"a");
    assert_eq!(arr[1].size, 2);
    assert_eq!(&arr[1].bytes[..], b"bc");
}

#[test]
fn test_string_split_by_three_segments_with_trailing_data() {
    // Per C: "aXbcXdd" split by "X". size=7, split=1, upper=6.
    // After matching at pos=4, start=5, pos=5
    // pos=5:'d', pos=6 (loop exits because pos<6 false)
    // pos != start (6 != 5), push (5, 6+1=7) => "dd"
    // Result: ["a", "bc", "dd"]
    let s = make("aXbcXdd");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "X");
    assert_eq!(sz, 3);
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"a");
    assert_eq!(arr[1].size, 2);
    assert_eq!(&arr[1].bytes[..], b"bc");
    assert_eq!(arr[2].size, 2);
    assert_eq!(&arr[2].bytes[..], b"dd");
}

#[test]
fn test_string_split_by_double_split_at_end() {
    // Per C: "XX" split by "X". size=2, split=1, upper=1. pos<1: pos=0:'X' match,
    // indexes[0]=(0,0), start=1, pos=1. Loop exits. pos==start, no extra push.
    // Result: [""].
    let s = make("XX");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "X");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 0);
    assert_eq!(&arr[0].bytes[..], b"");
}

#[test]
fn test_string_split_by_axx() {
    // Per C: "aXX" split by "X". size=3, split=1, upper=2.
    // pos=0:'a', pos=1:'X' match, indexes[0]=(0,1), start=2, pos=2.
    // Loop exits (pos<2 false). pos==start, no extra push.
    // Result: ["a"]
    let s = make("aXX");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "X");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"a");
}

#[test]
fn test_string_split_by_trailing_split_not_handled() {
    // Per C: "abcX" split by "X". size=4, split=1, upper=3.
    // No match at pos<3. After loop pos=3, start=0, push (0, 3+1=4).
    // Result: ["abcX"]
    let s = make("abcX");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "X");
    assert_eq!(sz, 1);
    assert_eq!(arr.len(), 1);
    assert_eq!(arr[0].size, 4);
    assert_eq!(&arr[0].bytes[..], b"abcX");
}

#[test]
fn test_string_split_by_multi_char_separator() {
    // Per C: "aXXbXXcXXd" split by "XX". size=10, split=2, upper=8.
    // pos=0:'a', pos=1:'X', pos=2:'X', pos=3:'b'...
    // Actually let me trace: pos=0: substr(0,2)="aX" != "XX", pos=1.
    // pos=1: substr(1,2)="XX" match, indexes[0]=(0,1)="a", start=3, pos=3.
    // pos=3: substr(3,2)="bX" != , pos=4. pos=4: substr(4,2)="XX" match,
    // indexes[1]=(3,4)="b", start=6, pos=6. pos=6: substr(6,2)="cX" != , pos=7.
    // pos=7: substr(7,2)="XX" match, indexes[2]=(6,7)="c", start=9, pos=9.
    // Wait pos=9 < 8 is false, loop exits. But wait pos=7 advanced to 9 by += split_str.size,
    // then loop check: pos < 8 false. Exit. pos=9, start=9, equal, no push.
    // Hmm actually wait, after push at pos=7 was matched, pos += split.size = 2, so pos=9.
    // Result: ["a", "b", "c"]
    let s = make("aXXbXXcXXd");
    let mut sz: usize = 0;
    let arr = string_split_by(&s, &mut sz, "XX");
    assert_eq!(sz, 3);
    assert_eq!(arr.len(), 3);
    assert_eq!(arr[0].size, 1);
    assert_eq!(&arr[0].bytes[..], b"a");
    assert_eq!(arr[1].size, 1);
    assert_eq!(&arr[1].bytes[..], b"b");
    assert_eq!(arr[2].size, 1);
    assert_eq!(&arr[2].bytes[..], b"c");
}

// ---------- string_join_arr ----------

#[test]
fn test_string_join_arr_single_empty() {
    let arr: Vec<StringT> = vec![make("")];
    let r = string_join_arr(&arr, 1, " ");
    let exp = make("");
    assert_eq!(r.size, 0);
    assert!(string_eq(&r, &exp));
}

#[test]
fn test_string_join_arr_single_value() {
    let arr: Vec<StringT> = vec![make("1")];
    let r = string_join_arr(&arr, 1, " ");
    let exp = make("1");
    assert_eq!(r.size, 1);
    assert!(string_eq(&r, &exp));
    assert_eq!(&r.bytes[..], b"1");
}

#[test]
fn test_string_join_arr_two_with_space() {
    let arr: Vec<StringT> = vec![make("some"), make("string")];
    let r = string_join_arr(&arr, 2, " ");
    let exp = make("some string");
    assert_eq!(r.size, 11);
    assert!(string_eq(&r, &exp));
    assert_eq!(&r.bytes[..], b"some string");
}

#[test]
fn test_string_join_arr_three_with_newline_including_empty_first() {
    let arr: Vec<StringT> = vec![make(""), make("some"), make("string")];
    let r = string_join_arr(&arr, 3, "\n");
    let exp = make("\nsome\nstring");
    assert_eq!(r.size, 12);
    assert!(string_eq(&r, &exp));
    assert_eq!(&r.bytes[..], b"\nsome\nstring");
}

#[test]
fn test_string_join_arr_with_multichar_sep() {
    let arr: Vec<StringT> = vec![make("some"), make("string")];
    let r = string_join_arr(&arr, 2, "SOME");
    let exp = make("someSOMEstring");
    assert_eq!(r.size, 14);
    assert!(string_eq(&r, &exp));
    assert_eq!(&r.bytes[..], b"someSOMEstring");
}

#[test]
fn test_string_join_arr_with_empty_separator() {
    let arr: Vec<StringT> = vec![make("abc"), make("def")];
    let r = string_join_arr(&arr, 2, "");
    assert_eq!(r.size, 6);
    assert_eq!(&r.bytes[..], b"abcdef");
}

// ---------- string_t_is_space_char ----------

#[test]
fn test_is_space_char_true() {
    assert!(string_t_is_space_char(b' '));
    assert!(string_t_is_space_char(b'\t'));
    assert!(string_t_is_space_char(b'\n'));
    assert!(string_t_is_space_char(b'\r'));
}

#[test]
fn test_is_space_char_false() {
    assert!(!string_t_is_space_char(b'a'));
    assert!(!string_t_is_space_char(b'0'));
    assert!(!string_t_is_space_char(b'_'));
    assert!(!string_t_is_space_char(0u8));
    assert!(!string_t_is_space_char(b'.'));
}

fn main() {}
