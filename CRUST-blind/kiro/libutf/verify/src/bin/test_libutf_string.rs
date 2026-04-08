use libutf::libutf_string::*;

// === Utf8String basic ops ===

#[test]
fn test_new_is_empty() {
    let s = Utf8String::new();
    assert!(s.is_empty());
    assert_eq!(s.len, 0);
}

#[test]
fn test_append_literal() {
    let mut s = Utf8String::new();
    assert!(s.append_literal(b"Hello").is_ok());
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
    assert!(!s.is_empty());
    s.destroy();
}

#[test]
fn test_append_character() {
    let mut s = Utf8String::new();
    assert!(s.append_character(b'A').is_ok());
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
    assert_eq!(&s.data[..s.len], b"Hello");
    s.destroy();
}

#[test]
fn test_clear() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.clear();
    assert!(s.is_empty());
    assert_eq!(s.len, 0);
    s.destroy();
}

#[test]
fn test_reserve_and_shrink() {
    let mut s = Utf8String::new();
    s.reserve(100).unwrap();
    assert!(s.cap >= 100);
    s.append_literal(b"Hi").unwrap();
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 2);
    assert_eq!(&s.data[..s.len], b"Hi");
    s.destroy();
}

#[test]
fn test_shrink_empty() {
    let mut s = Utf8String::new();
    s.reserve(64).unwrap();
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 0);
}

// === append/prepend with Utf8String and Utf8StringView ===

#[test]
fn test_append_string() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let mut other = Utf8String::new();
    other.append_literal(b" World").unwrap();
    s.append(&other).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
    other.destroy();
}

#[test]
fn test_append_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let view = Utf8StringView { data: b" World", len: 6 };
    s.append_view(&view).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

#[test]
fn test_prepend_string() {
    let mut s = Utf8String::new();
    s.append_literal(b"World").unwrap();
    let mut prefix = Utf8String::new();
    prefix.append_literal(b"Hello ").unwrap();
    s.prepend(&prefix).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
    prefix.destroy();
}

#[test]
fn test_prepend_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"World").unwrap();
    let view = Utf8StringView { data: b"Hello ", len: 6 };
    s.prepend_view(&view).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello World");
    s.destroy();
}

// === insert ===

#[test]
fn test_insert_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"ac").unwrap();
    s.insert_character(1, b'b').unwrap();
    assert_eq!(&s.data[..s.len], b"abc");
    s.destroy();
}

#[test]
fn test_insert_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"ad").unwrap();
    s.insert_literal(1, b"bc").unwrap();
    assert_eq!(&s.data[..s.len], b"abcd");
    s.destroy();
}

#[test]
fn test_insert_string() {
    let mut s = Utf8String::new();
    s.append_literal(b"ad").unwrap();
    let mut mid = Utf8String::new();
    mid.append_literal(b"bc").unwrap();
    s.insert(1, &mid).unwrap();
    assert_eq!(&s.data[..s.len], b"abcd");
    s.destroy();
    mid.destroy();
}

#[test]
fn test_insert_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"ad").unwrap();
    let view = Utf8StringView { data: b"bc", len: 2 };
    s.insert_view(1, &view).unwrap();
    assert_eq!(&s.data[..s.len], b"abcd");
    s.destroy();
}

#[test]
fn test_insert_at_start() {
    let mut s = Utf8String::new();
    s.append_literal(b"bc").unwrap();
    s.insert_character(0, b'a').unwrap();
    assert_eq!(&s.data[..s.len], b"abc");
    s.destroy();
}

#[test]
fn test_insert_at_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"ab").unwrap();
    s.insert_character(2, b'c').unwrap();
    assert_eq!(&s.data[..s.len], b"abc");
    s.destroy();
}

#[test]
fn test_insert_past_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"ab").unwrap();
    assert!(s.insert_character(10, b'c').is_err());
    s.destroy();
}

// === replace ===

#[test]
fn test_replace_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    s.replace_literal(5, 1, b"-").unwrap();
    assert_eq!(&s.data[..s.len], b"Hello-World");
    s.destroy();
}

#[test]
fn test_replace_string() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let mut rep = Utf8String::new();
    rep.append_literal(b"Rust").unwrap();
    s.replace(6, 5, &rep).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello Rust");
    s.destroy();
    rep.destroy();
}

#[test]
fn test_replace_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let view = Utf8StringView { data: b"Rust", len: 4 };
    s.replace_view(6, 5, &view).unwrap();
    assert_eq!(&s.data[..s.len], b"Hello Rust");
    s.destroy();
}

#[test]
fn test_replace_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    s.replace_character(1, 1, b'X').unwrap();
    assert_eq!(&s.data[..s.len], b"aXc");
    s.destroy();
}

#[test]
fn test_replace_clamp_len() {
    // When pos+len > string.len, C clamps len
    let mut s = Utf8String::new();
    s.append_literal(b"abcde").unwrap();
    s.replace_literal(3, 100, b"XY").unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"abcXY");
    s.destroy();
}

#[test]
fn test_replace_past_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    assert!(s.replace_literal(10, 1, b"X").is_err());
    s.destroy();
}

// === erase ===

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
fn test_erase_clamp() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcde").unwrap();
    s.erase(3, 100).unwrap();
    assert_eq!(s.len, 3);
    assert_eq!(&s.data[..s.len], b"abc");
    s.destroy();
}

#[test]
fn test_erase_past_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    assert!(s.erase(10, 1).is_err());
    s.destroy();
}

#[test]
fn test_erase_zero_len() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    s.erase(1, 0).unwrap();
    assert_eq!(&s.data[..s.len], b"abc");
    s.destroy();
}

// === compare ===

#[test]
fn test_compare_equal() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b"Hello").unwrap();
    assert_eq!(a.compare(&b), 0);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_less() {
    let mut a = Utf8String::new();
    a.append_literal(b"A").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b"Say Hello").unwrap();
    // "A" < "Say Hello" -> negative
    assert!(a.compare(&b) < 0);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_greater() {
    let mut a = Utf8String::new();
    a.append_literal(b"Say Hello").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b"A").unwrap();
    assert!(a.compare(&b) > 0);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_prefix_shorter() {
    // "Hell" < "Hello" (same prefix, shorter)
    let mut a = Utf8String::new();
    a.append_literal(b"Hell").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b"Hello").unwrap();
    assert_eq!(a.compare(&b), -1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_prefix_longer() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b"Hell").unwrap();
    assert_eq!(a.compare(&b), 1);
    a.destroy();
    b.destroy();
}

#[test]
fn test_compare_literal_eq() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.compare_literal(b"Hello"), 0);
    s.destroy();
}

#[test]
fn test_compare_literal_lt() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert!(s.compare_literal(b"Hellp") < 0);
    s.destroy();
}

#[test]
fn test_compare_literal_gt() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert!(s.compare_literal(b"Helln") > 0);
    s.destroy();
}

// === substring ===

#[test]
fn test_substring() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello World").unwrap();
    let v = s.substring(6, 11);
    assert_eq!(v.len, 5);
    assert_eq!(v.data, b"World");
    s.destroy();
}

#[test]
fn test_substring_clamp_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let v = s.substring(2, 100);
    assert_eq!(v.len, 3);
    assert_eq!(v.data, b"llo");
    s.destroy();
}

#[test]
fn test_substring_max_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let v = s.substring(2, usize::MAX);
    assert_eq!(v.len, 3);
    s.destroy();
}

#[test]
fn test_substring_start_gt_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    let v = s.substring(8, 2);
    assert_eq!(v.len, 0);
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

// === index_of / last_index_of ===

#[test]
fn test_index_of_found() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.index_of_character(0, b'l'), Some(2));
    s.destroy();
}

#[test]
fn test_index_of_not_found() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.index_of_character(0, b'z'), None);
    s.destroy();
}

#[test]
fn test_index_of_with_offset() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.index_of_character(3, b'l'), Some(3));
    s.destroy();
}

#[test]
fn test_last_index_of_max() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.last_index_of_character(usize::MAX, b'l'), Some(3));
    s.destroy();
}

#[test]
fn test_last_index_of_from_pos() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.last_index_of_character(2, b'l'), Some(2));
    s.destroy();
}

#[test]
fn test_last_index_of_not_found() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.last_index_of_character(usize::MAX, b'z'), None);
    s.destroy();
}

#[test]
fn test_last_index_of_empty() {
    let s = Utf8String::new();
    assert_eq!(s.last_index_of_character(usize::MAX, b'a'), None);
}

#[test]
fn test_last_index_of_pos_past_end() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.last_index_of_character(10, b'l'), None);
    s.destroy();
}

// === concat ===

#[test]
fn test_concat() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let mut b = Utf8String::new();
    b.append_literal(b" World").unwrap();
    let mut c = a.concat(&b).unwrap();
    assert_eq!(c.len, 11);
    assert_eq!(&c.data[..c.len], b"Hello World");
    c.destroy();
    a.destroy();
    b.destroy();
}

#[test]
fn test_concat_view() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let view = Utf8StringView { data: b" World", len: 6 };
    let mut c = a.concat_view(&view).unwrap();
    assert_eq!(&c.data[..c.len], b"Hello World");
    c.destroy();
    a.destroy();
}

#[test]
fn test_concat_character() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hi").unwrap();
    let mut c = a.concat_character(b'!').unwrap();
    assert_eq!(&c.data[..c.len], b"Hi!");
    c.destroy();
    a.destroy();
}

#[test]
fn test_concat_literal() {
    let mut a = Utf8String::new();
    a.append_literal(b"Hello").unwrap();
    let mut c = a.concat_literal(b" World").unwrap();
    assert_eq!(&c.data[..c.len], b"Hello World");
    c.destroy();
    a.destroy();
}

// === Utf8StringView ===

#[test]
fn test_view_is_empty() {
    let v = Utf8StringView { data: b"", len: 0 };
    assert!(v.is_empty());
    let v2 = Utf8StringView { data: b"Hi", len: 2 };
    assert!(!v2.is_empty());
}

#[test]
fn test_view_compare() {
    let a = Utf8StringView { data: b"abc", len: 3 };
    let b = Utf8StringView { data: b"abd", len: 3 };
    assert!(a.compare(&b) < 0);
    assert!(b.compare(&a) > 0);
    assert_eq!(a.compare(&a), 0);
}

#[test]
fn test_view_compare_literal() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    assert_eq!(v.compare_literal(b"Hello"), 0);
    assert!(v.compare_literal(b"Hellp") < 0);
    assert!(v.compare_literal(b"Helln") > 0);
}

#[test]
fn test_view_compare_different_lengths() {
    let a = Utf8StringView { data: b"abc", len: 3 };
    let b = Utf8StringView { data: b"abcd", len: 4 };
    assert_eq!(a.compare(&b), -1);
    assert_eq!(b.compare(&a), 1);
}

#[test]
fn test_view_substring() {
    let v = Utf8StringView { data: b"Hello World", len: 11 };
    let sub = v.substring(6, 11);
    assert_eq!(sub.len, 5);
    assert_eq!(sub.data, b"World");
}

#[test]
fn test_view_substring_clamp() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let sub = v.substring(2, 100);
    assert_eq!(sub.len, 3);
}

#[test]
fn test_view_substring_max() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let sub = v.substring(2, usize::MAX);
    assert_eq!(sub.len, 3);
}

#[test]
fn test_view_substring_start_gt_end() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let sub = v.substring(8, 2);
    assert_eq!(sub.len, 0);
}

#[test]
fn test_view_substring_copy() {
    let v = Utf8StringView { data: b"Hello World", len: 11 };
    let mut copy = v.substring_copy(6, 11).unwrap();
    assert_eq!(copy.len, 5);
    assert_eq!(&copy.data[..copy.len], b"World");
    copy.destroy();
}

#[test]
fn test_view_index_of() {
    let v = Utf8StringView { data: b"Hello World", len: 11 };
    assert_eq!(v.index_of_character(0, b'o'), Some(4));
    assert_eq!(v.index_of_character(5, b'o'), Some(7));
    assert_eq!(v.index_of_character(0, b'z'), None);
}

#[test]
fn test_view_last_index_of() {
    let v = Utf8StringView { data: b"Hello World", len: 11 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'o'), Some(7));
    assert_eq!(v.last_index_of_character(5, b'o'), Some(4));
    assert_eq!(v.last_index_of_character(usize::MAX, b'z'), None);
}

#[test]
fn test_view_last_index_of_empty() {
    let v = Utf8StringView { data: b"", len: 0 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'a'), None);
}

#[test]
fn test_view_last_index_of_past_end() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    assert_eq!(v.last_index_of_character(10, b'l'), None);
}

// === init/destroy lifecycle ===

#[test]
fn test_init_reinit() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.destroy();
    s.init();
    assert!(s.is_empty());
    assert_eq!(s.cap, 0);
}

#[test]
fn test_destroy_empty() {
    let mut s = Utf8String::new();
    s.destroy(); // should not crash
}

// === copy ===

#[test]
fn test_append_empty_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.append_literal(b"").unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
    s.destroy();
}

fn main() {}
