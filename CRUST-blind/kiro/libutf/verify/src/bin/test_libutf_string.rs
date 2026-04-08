use libutf::libutf_string::*;
use libutf::libutf_utf::Utf8;

// === Utf8String basic operations ===

#[test]
fn test_new_is_empty() {
    let s = Utf8String::new();
    assert!(s.is_empty());
    assert_eq!(s.len, 0);
}

#[test]
fn test_append_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.append_literal(b" World").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_append_character() {
    let mut s = Utf8String::new();
    s.append_character(b'A').unwrap();
    assert_eq!(s.len, 1);
    assert_eq!(s.data[0], b'A');
    s.destroy();
}

#[test]
fn test_prepend_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"World").unwrap();
    s.prepend_literal(b"Hello ").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_prepend_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"ello").unwrap();
    s.prepend_character(b'H').unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
    s.destroy();
}

#[test]
fn test_insert_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Helo").unwrap();
    s.insert_character(2, b'l').unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
    s.destroy();
}

#[test]
fn test_insert_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hd").unwrap();
    s.insert_literal(1, b"ello Worl").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_insert_past_end_fails() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hi").unwrap();
    assert!(s.insert_character(5, b'x').is_err());
    s.destroy();
}

#[test]
fn test_replace_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    s.replace_literal(5, 6, b" Rust").unwrap();
    assert_eq!(s.len, 10);
    assert_eq!(&s.data[..s.len], b"Hello Rust");
    s.destroy();
}

#[test]
fn test_replace_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    s.replace_character(1, 1, b'X').unwrap();
    assert_eq!(s.len, 3);
    assert_eq!(&s.data[..s.len], b"aXc");
    s.destroy();
}

#[test]
fn test_erase() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    s.erase(5, 6).unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
    s.destroy();
}

#[test]
fn test_clear() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hi").unwrap();
    assert!(!s.is_empty());
    s.clear();
    assert!(s.is_empty());
    assert_eq!(s.len, 0);
    s.destroy();
}

// === Substring ===

#[test]
fn test_substring() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let view = s.substring(6, 11);
    assert_eq!(view.len, 5);
    assert_eq!(&view.data[..view.len], b"World");
    s.destroy();
}

#[test]
fn test_substring_usize_max() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let view = s.substring(0, usize::MAX);
    assert_eq!(view.len, 5);
    assert_eq!(&view.data[..view.len], b"Hello");
    s.destroy();
}

#[test]
fn test_substring_start_past_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let view = s.substring(10, 3);
    assert_eq!(view.len, 0);
    s.destroy();
}

#[test]
fn test_substring_copy() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let mut copy = s.substring_copy(6, 11).unwrap();
    assert_eq!(copy.len, 5);
    assert_eq!(&copy.data[..copy.len], b"World");
    copy.destroy();
    s.destroy();
}

// === Compare ===

#[test]
fn test_compare_less() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    b.append_literal(b"abd").unwrap();
    assert_eq!(a.compare(&b), -1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_greater() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abd").unwrap();
    b.append_literal(b"abc").unwrap();
    assert_eq!(a.compare(&b), 1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_equal() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    b.append_literal(b"abc").unwrap();
    assert_eq!(a.compare(&b), 0);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_shorter() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    b.append_literal(b"abcd").unwrap();
    assert_eq!(a.compare(&b), -1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_longer() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abcd").unwrap();
    b.append_literal(b"abc").unwrap();
    assert_eq!(a.compare(&b), 1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    assert_eq!(s.compare_literal(b"abd"), -1);
    assert_eq!(s.compare_literal(b"abc"), 0);
    assert_eq!(s.compare_literal(b"abb"), 1);
    s.destroy();
}

// === Index of ===

#[test]
fn test_index_of_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    assert_eq!(s.index_of_character(0, b'o'), Some(4));
    assert_eq!(s.index_of_character(5, b'o'), Some(7));
    assert_eq!(s.index_of_character(0, b'z'), None);
    s.destroy();
}

#[test]
fn test_last_index_of_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    assert_eq!(s.last_index_of_character(usize::MAX, b'o'), Some(7));
    assert_eq!(s.last_index_of_character(5, b'o'), Some(4));
    assert_eq!(s.last_index_of_character(usize::MAX, b'z'), None);
    s.destroy();
}

// === Concat ===

#[test]
fn test_concat_literal() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let mut result = a.concat_literal(b" World").unwrap();
    assert_eq!(result.len, 11);
    assert_eq!(&result.data[..result.len], b"Hello World");
    result.destroy();
    a.destroy();
}

#[test]
fn test_concat_character() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hi").unwrap();
    let mut result = a.concat_character(b'!').unwrap();
    assert_eq!(result.len, 3);
    assert_eq!(&result.data[..result.len], b"Hi!");
    result.destroy();
    a.destroy();
}

#[test]
fn test_concat() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    b.append_literal(b" World").unwrap();
    let mut result = a.concat(&b).unwrap();
    assert_eq!(result.len, 11);
    assert_eq!(&result.data[..result.len], b"Hello World");
    result.destroy();
    a.destroy();
    b.destroy();
}

// === Append/Prepend with Utf8String ===

#[test]
fn test_append_string() {
    let mut s = Utf8String::new();
    let mut other = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    other.append_literal(b" World").unwrap();
    s.append(&other).unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
    other.destroy();
}

#[test]
fn test_prepend_string() {
    let mut s = Utf8String::new();
    let mut other = Utf8String::new();
    s.append_literal(b"World").unwrap();
    other.append_literal(b"Hello ").unwrap();
    s.prepend(&other).unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
    other.destroy();
}

// === Utf8StringView ===

#[test]
fn test_view_is_empty() {
    let view = Utf8StringView { data: &[], len: 0 };
    assert!(view.is_empty());
}

#[test]
fn test_view_compare() {
    let a_data: &[Utf8] = b"abc";
    let b_data: &[Utf8] = b"abd";
    let a = Utf8StringView { data: a_data, len: 3 };
    let b = Utf8StringView { data: b_data, len: 3 };
    assert_eq!(a.compare(&b), -1);
    assert_eq!(b.compare(&a), 1);
    let c = Utf8StringView { data: a_data, len: 3 };
    assert_eq!(a.compare(&c), 0);
}

#[test]
fn test_view_compare_literal() {
    let data: &[Utf8] = b"abc";
    let v = Utf8StringView { data, len: 3 };
    assert_eq!(v.compare_literal(b"abd"), -1);
    assert_eq!(v.compare_literal(b"abc"), 0);
    assert_eq!(v.compare_literal(b"abb"), 1);
}

#[test]
fn test_view_substring() {
    let data: &[Utf8] = b"Hello World";
    let v = Utf8StringView { data, len: 11 };
    let sub = v.substring(6, 11);
    assert_eq!(sub.len, 5);
    assert_eq!(&sub.data[..sub.len], b"World");
}

#[test]
fn test_view_substring_copy() {
    let data: &[Utf8] = b"Hello World";
    let v = Utf8StringView { data, len: 11 };
    let mut copy = v.substring_copy(6, 11).unwrap();
    assert_eq!(copy.len, 5);
    assert_eq!(&copy.data[..copy.len], b"World");
    copy.destroy();
}

#[test]
fn test_view_index_of_character() {
    let data: &[Utf8] = b"Hello World";
    let v = Utf8StringView { data, len: 11 };
    assert_eq!(v.index_of_character(0, b'o'), Some(4));
    assert_eq!(v.index_of_character(5, b'o'), Some(7));
    assert_eq!(v.index_of_character(0, b'z'), None);
}

#[test]
fn test_view_last_index_of_character() {
    let data: &[Utf8] = b"Hello World";
    let v = Utf8StringView { data, len: 11 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'o'), Some(7));
    assert_eq!(v.last_index_of_character(5, b'o'), Some(4));
    assert_eq!(v.last_index_of_character(usize::MAX, b'z'), None);
}

// === Reserve / Shrink ===

#[test]
fn test_reserve_and_shrink() {
    let mut s = Utf8String::new();
    s.reserve(100).unwrap();
    assert!(s.cap >= 100);
    s.append_literal(b"Hi").unwrap();
    assert_eq!(s.len, 2);
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 2);
    assert_eq!(s.len, 2);
    assert_eq!(&s.data[..s.len], b"Hi");
    s.destroy();
}

#[test]
fn test_shrink_empty() {
    let mut s = Utf8String::new();
    s.reserve(16).unwrap();
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 0);
    assert_eq!(s.len, 0);
}

// === Erase edge cases ===

#[test]
fn test_erase_clamps_len() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    // erase from pos=3 with len=100 should clamp to remaining
    s.erase(3, 100).unwrap();
    assert_eq!(s.len, 3);
    assert_eq!(&s.data[..s.len], b"Hel");
    s.destroy();
}

#[test]
fn test_replace_clamps_len() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.replace_literal(3, 100, b"p!").unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Help!");
    s.destroy();
}

// === Append/Prepend/Insert/Replace with view ===

#[test]
fn test_append_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let view_data: &[Utf8] = b" World";
    let view = Utf8StringView { data: view_data, len: 6 };
    s.append_view(&view).unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_prepend_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"World").unwrap();
    let view_data: &[Utf8] = b"Hello ";
    let view = Utf8StringView { data: view_data, len: 6 };
    s.prepend_view(&view).unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_insert_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hd").unwrap();
    let view_data: &[Utf8] = b"ello Worl";
    let view = Utf8StringView { data: view_data, len: 9 };
    s.insert_view(1, &view).unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_replace_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let view_data: &[Utf8] = b" Rust";
    let view = Utf8StringView { data: view_data, len: 5 };
    s.replace_view(5, 6, &view).unwrap();
    assert_eq!(s.len, 10);
    assert_eq!(&s.data[..s.len], b"Hello Rust");
    s.destroy();
}

#[test]
fn test_concat_view() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let view_data: &[Utf8] = b" World";
    let view = Utf8StringView { data: view_data, len: 6 };
    let mut result = a.concat_view(&view).unwrap();
    assert_eq!(result.len, 11);
    assert_eq!(&result.data[..result.len], b"Hello World");
    result.destroy();
    a.destroy();
}

fn main() {}
