use libutf::libutf_string::*;
use libutf::libutf_utf::Utf8;

#[test]
fn test_init_and_empty() {
    let s = Utf8String::new();
    assert_eq!(s.len, 0);
    assert_eq!(s.is_empty(), true);
}

#[test]
fn test_append_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"Hello");
}

#[test]
fn test_append_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello").unwrap();
    s.append_character(b'!').unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"Hello!");
}

#[test]
fn test_prepend_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello!").unwrap();
    s.prepend_literal(b">>").unwrap();
    assert_eq!(s.len, 8);
    assert_eq!(&s.data[..s.len], b">>Hello!");
}

#[test]
fn test_insert_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b">>Hello!").unwrap();
    s.insert_literal(2, b"-").unwrap();
    assert_eq!(s.len, 9);
    assert_eq!(&s.data[..s.len], b">>-Hello!");
}

#[test]
fn test_replace_string() {
    let mut s = Utf8String::new();
    s.append_literal(b">>-Hello!").unwrap();
    let mut r = Utf8String::new();
    r.append_literal(b"REP").unwrap();
    s.replace(0, 2, &r).unwrap();
    assert_eq!(s.len, 10);
    assert_eq!(&s.data[..s.len], b"REP-Hello!");
}

#[test]
fn test_erase() {
    let mut s = Utf8String::new();
    s.append_literal(b"REP-Hello!").unwrap();
    s.erase(0, 3).unwrap();
    assert_eq!(s.len, 7);
    assert_eq!(&s.data[..s.len], b"-Hello!");
}

#[test]
fn test_compare() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    let mut c = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    b.append_literal(b"abd").unwrap();
    c.append_literal(b"ab").unwrap();
    assert_eq!(a.compare(&b), -1);
    assert_eq!(a.compare(&a), 0);
    assert_eq!(a.compare(&c), 1);
    assert_eq!(c.compare(&a), -1);
    assert_eq!(a.compare_literal(b"abd"), -1);
}

#[test]
fn test_substring() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello, World!").unwrap();
    let v = s.substring(7, 12);
    assert_eq!(v.len, 5);
    assert_eq!(v.data, b"World");

    let v = s.substring(0, usize::MAX);
    assert_eq!(v.len, 13);
    assert_eq!(v.data, b"Hello, World!");

    let v = s.substring(100, 100);
    assert_eq!(v.len, 0);
}

#[test]
fn test_index_of_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello, World!").unwrap();
    assert_eq!(s.index_of_character(0, b'l'), Some(2));
    assert_eq!(s.index_of_character(4, b'l'), Some(10));
    assert_eq!(s.index_of_character(0, b'z'), None);
}

#[test]
fn test_last_index_of_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello, World!").unwrap();
    assert_eq!(s.last_index_of_character(usize::MAX, b'l'), Some(10));
    assert_eq!(s.last_index_of_character(5, b'l'), Some(3));
    assert_eq!(s.last_index_of_character(usize::MAX, b'z'), None);
}

#[test]
fn test_concat() {
    let mut a = Utf8String::new();
    let mut b = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    b.append_literal(b"abd").unwrap();
    let cat = a.concat(&b).unwrap();
    assert_eq!(cat.len, 6);
    assert_eq!(&cat.data[..cat.len], b"abcabd");
}

#[test]
fn test_concat_character() {
    let mut a = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    let cat = a.concat_character(b'!').unwrap();
    assert_eq!(cat.len, 4);
    assert_eq!(&cat.data[..cat.len], b"abc!");
}

#[test]
fn test_concat_literal() {
    let mut a = Utf8String::new();
    a.append_literal(b"abc").unwrap();
    let cat = a.concat_literal(b"XY").unwrap();
    assert_eq!(cat.len, 5);
    assert_eq!(&cat.data[..cat.len], b"abcXY");
}

#[test]
fn test_grow_past_8() {
    let mut s = Utf8String::new();
    s.append_literal(b"01234567890").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(&s.data[..s.len], b"01234567890");
}

#[test]
fn test_replace_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcdefg").unwrap();
    let view = Utf8StringView { data: b"XX", len: 2 };
    s.replace_view(2, 3, &view).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"abXXfg");
}

#[test]
fn test_replace_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcdefg").unwrap();
    s.replace_character(2, 3, b'Z').unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"abZfg");
}

#[test]
fn test_replace_literal() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcdefg").unwrap();
    s.replace_literal(2, 3, b"YYY").unwrap();
    assert_eq!(s.len, 7);
    assert_eq!(&s.data[..s.len], b"abYYYfg");
}

#[test]
fn test_clear_and_isempty() {
    let mut s = Utf8String::new();
    s.append_literal(b"hi").unwrap();
    assert!(!s.is_empty());
    s.clear();
    assert_eq!(s.len, 0);
    assert!(s.is_empty());
}

#[test]
fn test_view_compare() {
    let v1 = Utf8StringView { data: b"abc", len: 3 };
    let v2 = Utf8StringView { data: b"abd", len: 3 };
    let v3 = Utf8StringView { data: b"ab", len: 2 };
    assert_eq!(v1.compare(&v2), -1);
    assert_eq!(v1.compare(&v1), 0);
    assert_eq!(v1.compare(&v3), 1);
    assert_eq!(v3.compare(&v1), -1);
    assert_eq!(v1.compare_literal(b"abd"), -1);
    assert_eq!(v1.compare_literal(b"abc"), 0);
}

#[test]
fn test_view_substring() {
    let v = Utf8StringView { data: b"Hello, World!", len: 13 };
    let sub = v.substring(7, 12);
    assert_eq!(sub.len, 5);
    assert_eq!(sub.data, b"World");
    let sub = v.substring(0, usize::MAX);
    assert_eq!(sub.len, 13);
    assert_eq!(sub.data, b"Hello, World!");
    let sub = v.substring(100, 100);
    assert_eq!(sub.len, 0);
}

#[test]
fn test_view_substring_copy() {
    let v = Utf8StringView { data: b"Hello, World!", len: 13 };
    let copy = v.substring_copy(7, 12).unwrap();
    assert_eq!(copy.len, 5);
    assert_eq!(&copy.data[..copy.len], b"World");
}

#[test]
fn test_view_index_of_character() {
    let v = Utf8StringView { data: b"Hello, World!", len: 13 };
    assert_eq!(v.index_of_character(0, b'l'), Some(2));
    assert_eq!(v.index_of_character(4, b'l'), Some(10));
    assert_eq!(v.index_of_character(0, b'z'), None);
}

#[test]
fn test_view_last_index_of_character() {
    let v = Utf8StringView { data: b"Hello, World!", len: 13 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'l'), Some(10));
    assert_eq!(v.last_index_of_character(5, b'l'), Some(3));
    assert_eq!(v.last_index_of_character(usize::MAX, b'z'), None);
}

#[test]
fn test_view_isempty() {
    let v = Utf8StringView { data: b"", len: 0 };
    assert!(v.is_empty());
    let v2 = Utf8StringView { data: b"x", len: 1 };
    assert!(!v2.is_empty());
}

#[test]
fn test_substring_copy() {
    let mut s = Utf8String::new();
    s.append_literal(b"Hello, World!").unwrap();
    let copy = s.substring_copy(7, 12).unwrap();
    assert_eq!(copy.len, 5);
    assert_eq!(&copy.data[..copy.len], b"World");
}

#[test]
fn test_append_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"foo").unwrap();
    let v = Utf8StringView { data: b"bar", len: 3 };
    s.append_view(&v).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"foobar");
}

#[test]
fn test_prepend_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"oo").unwrap();
    s.prepend_character(b'f').unwrap();
    assert_eq!(s.len, 3);
    assert_eq!(&s.data[..s.len], b"foo");
}

#[test]
fn test_prepend_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"bar").unwrap();
    let v = Utf8StringView { data: b"foo", len: 3 };
    s.prepend_view(&v).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"foobar");
}

#[test]
fn test_prepend() {
    let mut s = Utf8String::new();
    s.append_literal(b"bar").unwrap();
    let mut o = Utf8String::new();
    o.append_literal(b"foo").unwrap();
    s.prepend(&o).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"foobar");
}

#[test]
fn test_insert_string() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcd").unwrap();
    let mut o = Utf8String::new();
    o.append_literal(b"XX").unwrap();
    s.insert(2, &o).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"abXXcd");
}

#[test]
fn test_insert_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcd").unwrap();
    let v = Utf8StringView { data: b"XX", len: 2 };
    s.insert_view(2, &v).unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(&s.data[..s.len], b"abXXcd");
}

#[test]
fn test_insert_character() {
    let mut s = Utf8String::new();
    s.append_literal(b"abcd").unwrap();
    s.insert_character(2, b'Z').unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(&s.data[..s.len], b"abZcd");
}

#[test]
fn test_insert_pos_too_large_returns_err() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    let res = s.insert_character(5, b'!');
    assert!(res.is_err());
    // Original data unchanged
    assert_eq!(s.len, 3);
    assert_eq!(&s.data[..s.len], b"abc");
}

#[test]
fn test_concat_view() {
    let mut s = Utf8String::new();
    s.append_literal(b"foo").unwrap();
    let v = Utf8StringView { data: b"bar", len: 3 };
    let r = s.concat_view(&v).unwrap();
    assert_eq!(r.len, 6);
    assert_eq!(&r.data[..r.len], b"foobar");
}

#[test]
fn test_append_helper_types() {
    // Just verify Utf8 type alias works as expected
    let c: Utf8 = b'X';
    let mut s = Utf8String::new();
    s.append_character(c).unwrap();
    assert_eq!(s.len, 1);
    assert_eq!(s.data[0], b'X');
}

fn main() {}
