use libutf::libutf_string::{Utf8String, Utf8StringView};

// Helper: build a Utf8String from a literal.
fn make(text: &[u8]) -> Utf8String<'static> {
    let mut s = Utf8String::new();
    s.append_literal(text).unwrap();
    s
}

fn bytes<'a>(s: &'a Utf8String<'a>) -> &'a [u8] {
    &s.data[..s.len]
}

// ----------------- new / init -----------------

#[test]
fn test_new_init_is_empty() {
    let s = Utf8String::new();
    assert_eq!(s.len, 0);
    assert_eq!(s.cap, 0);
    assert_eq!(s.is_empty(), true);
    assert_eq!(s.data.len(), 0);
}

#[test]
fn test_init_resets() {
    let mut s = Utf8String::new();
    s.append_literal(b"abc").unwrap();
    s.init();
    assert_eq!(s.len, 0);
    assert_eq!(s.cap, 0);
    assert!(s.is_empty());
}

#[test]
fn test_clear() {
    let mut s = make(b"hello");
    s.clear();
    assert_eq!(s.len, 0);
    assert!(s.is_empty());
    s.destroy();
}

// ----------------- reserve / shrink_to_fit -----------------

#[test]
fn test_reserve_zero_to_5() {
    let mut s = Utf8String::new();
    assert_eq!(s.reserve(5), Ok(()));
    // round_up_pow2(5) = 8
    assert_eq!(s.cap, 8);
    assert_eq!(s.len, 0);
    s.destroy();
}

#[test]
fn test_reserve_grow_to_100() {
    let mut s = Utf8String::new();
    s.reserve(100).unwrap();
    // round_up_pow2(100) = 128
    assert_eq!(s.cap, 128);
    s.destroy();
}

#[test]
fn test_reserve_within_cap_no_op() {
    let mut s = Utf8String::new();
    s.reserve(64).unwrap();
    let cap_before = s.cap;
    assert_eq!(cap_before, 64);
    s.reserve(32).unwrap();
    assert_eq!(s.cap, cap_before);
    s.destroy();
}

#[test]
fn test_reserve_preserves_data() {
    let mut s = make(b"hello");
    let cap_before = s.cap;
    s.reserve(cap_before * 4).unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(bytes(&s), b"hello");
    s.destroy();
}

#[test]
fn test_shrink_to_fit_basic() {
    let mut s = Utf8String::new();
    s.reserve(128).unwrap();
    s.append_literal(b"hi").unwrap();
    assert_eq!(s.cap, 128);
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 2);
    assert_eq!(bytes(&s), b"hi");
    s.destroy();
}

#[test]
fn test_shrink_to_fit_empty() {
    let mut s = Utf8String::new();
    s.reserve(128).unwrap();
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 0);
    assert_eq!(s.len, 0);
}

#[test]
fn test_shrink_to_fit_no_alloc() {
    // When cap == 0, shrink_to_fit is a no-op.
    let mut s = Utf8String::new();
    s.shrink_to_fit().unwrap();
    assert_eq!(s.cap, 0);
}

// ----------------- append -----------------

#[test]
fn test_append_literal_single() {
    let mut s = Utf8String::new();
    s.append_literal(b"hello").unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(bytes(&s), b"hello");
    s.destroy();
}

#[test]
fn test_append_literal_multi() {
    let mut s = Utf8String::new();
    s.append_literal(b"hello").unwrap();
    s.append_literal(b" world").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(bytes(&s), b"hello world");
    s.destroy();
}

#[test]
fn test_append_character() {
    let mut s = Utf8String::new();
    for c in b"abcde" {
        s.append_character(*c).unwrap();
    }
    assert_eq!(s.len, 5);
    assert_eq!(bytes(&s), b"abcde");
    s.destroy();
}

#[test]
fn test_append_string() {
    let mut a = make(b"foo");
    let b = make(b"bar");
    a.append(&b).unwrap();
    assert_eq!(a.len, 6);
    assert_eq!(bytes(&a), b"foobar");
    a.destroy();
}

#[test]
fn test_append_view() {
    let mut a = make(b"foo");
    let backing = b"barbaz";
    let v = Utf8StringView { data: backing, len: 6 };
    a.append_view(&v).unwrap();
    assert_eq!(a.len, 9);
    assert_eq!(bytes(&a), b"foobarbaz");
    a.destroy();
}

// ----------------- prepend -----------------

#[test]
fn test_prepend_literal() {
    let mut s = make(b"world");
    s.prepend_literal(b"hello ").unwrap();
    assert_eq!(s.len, 11);
    assert_eq!(bytes(&s), b"hello world");
    s.destroy();
}

#[test]
fn test_prepend_character() {
    let mut s = make(b"ello");
    s.prepend_character(b'h').unwrap();
    assert_eq!(s.len, 5);
    assert_eq!(bytes(&s), b"hello");
    s.destroy();
}

#[test]
fn test_prepend_string() {
    let mut a = make(b"world");
    let b = make(b"hello ");
    a.prepend(&b).unwrap();
    assert_eq!(bytes(&a), b"hello world");
    a.destroy();
}

#[test]
fn test_prepend_view() {
    let mut a = make(b"world");
    let backing = b"hello ";
    let v = Utf8StringView { data: backing, len: 6 };
    a.prepend_view(&v).unwrap();
    assert_eq!(bytes(&a), b"hello world");
    a.destroy();
}

// ----------------- insert -----------------

#[test]
fn test_insert_literal() {
    let mut s = make(b"AABB");
    s.insert_literal(2, b"XX").unwrap();
    assert_eq!(s.len, 6);
    assert_eq!(bytes(&s), b"AAXXBB");
    s.destroy();
}

#[test]
fn test_insert_character_middle() {
    let mut s = make(b"ab");
    s.insert_character(1, b'X').unwrap();
    assert_eq!(s.len, 3);
    assert_eq!(bytes(&s), b"aXb");
    s.destroy();
}

#[test]
fn test_insert_character_pos_too_large_returns_err() {
    let mut s = make(b"ab");
    let res = s.insert_character(100, b'Y');
    assert_eq!(res, Err(()));
    // String unchanged
    assert_eq!(bytes(&s), b"ab");
    s.destroy();
}

#[test]
fn test_insert_at_end() {
    let mut s = make(b"abc");
    s.insert_character(3, b'D').unwrap();
    assert_eq!(bytes(&s), b"abcD");
    s.destroy();
}

#[test]
fn test_insert_at_start() {
    let mut s = make(b"abc");
    s.insert_character(0, b'Z').unwrap();
    assert_eq!(bytes(&s), b"Zabc");
    s.destroy();
}

#[test]
fn test_insert_string() {
    let mut a = make(b"ABCD");
    let b = make(b"XX");
    a.insert(2, &b).unwrap();
    assert_eq!(bytes(&a), b"ABXXCD");
    a.destroy();
}

#[test]
fn test_insert_view() {
    let mut a = make(b"ABCD");
    let backing = b"XX";
    let v = Utf8StringView { data: backing, len: 2 };
    a.insert_view(2, &v).unwrap();
    assert_eq!(bytes(&a), b"ABXXCD");
    a.destroy();
}

// ----------------- replace -----------------

#[test]
fn test_replace_literal_basic() {
    let mut s = make(b"Hello, world");
    // Replace bytes [5..11) (", worl") of length 6 with " there"
    s.replace_literal(5, 6, b" there").unwrap();
    assert_eq!(bytes(&s), b"Hello thered");
    assert_eq!(s.len, 12);
    s.destroy();
}

#[test]
fn test_replace_literal_clamp_len() {
    let mut s = make(b"abc");
    // pos=1, len=10 (clamped to 2). Replacement "XXX" (len 3).
    // result = a + XXX = "aXXX"
    s.replace_literal(1, 10, b"XXX").unwrap();
    assert_eq!(bytes(&s), b"aXXX");
    assert_eq!(s.len, 4);
    s.destroy();
}

#[test]
fn test_replace_string() {
    let mut a = make(b"Hello, world");
    let r = make(b" there");
    a.replace(5, 6, &r).unwrap();
    assert_eq!(bytes(&a), b"Hello thered");
    a.destroy();
}

#[test]
fn test_replace_view() {
    let mut a = make(b"Hello, world");
    let backing = b" there";
    let v = Utf8StringView { data: backing, len: 6 };
    a.replace_view(5, 6, &v).unwrap();
    assert_eq!(bytes(&a), b"Hello thered");
    a.destroy();
}

#[test]
fn test_replace_character() {
    let mut s = make(b"ABCDE");
    // replace 3 chars starting at pos=1 with 'Z' -> "AZE"
    s.replace_character(1, 3, b'Z').unwrap();
    assert_eq!(bytes(&s), b"AZE");
    assert_eq!(s.len, 3);
    s.destroy();
}

#[test]
fn test_replace_pos_too_large() {
    let mut s = make(b"abc");
    assert_eq!(s.replace_literal(100, 0, b"x"), Err(()));
    assert_eq!(bytes(&s), b"abc");
    s.destroy();
}

// ----------------- erase -----------------

#[test]
fn test_erase_basic() {
    let mut s = make(b"Hello, world");
    // Remove 2 chars at pos 5 (", ") -> "Helloworld"
    s.erase(5, 2).unwrap();
    assert_eq!(bytes(&s), b"Helloworld");
    assert_eq!(s.len, 10);
    s.destroy();
}

#[test]
fn test_erase_clamp() {
    let mut s = make(b"Hello");
    s.erase(2, 100).unwrap();
    assert_eq!(bytes(&s), b"He");
    assert_eq!(s.len, 2);
    s.destroy();
}

#[test]
fn test_erase_pos_too_large() {
    let mut s = make(b"abc");
    assert_eq!(s.erase(100, 0), Err(()));
    assert_eq!(bytes(&s), b"abc");
    s.destroy();
}

// ----------------- concat (immutable) -----------------

#[test]
fn test_concat_strings() {
    let a = make(b"foo");
    let b = make(b"bar");
    let c = a.concat(&b).unwrap();
    assert_eq!(bytes(&c), b"foobar");
    assert_eq!(c.len, 6);
}

#[test]
fn test_concat_view() {
    let a = make(b"foo");
    let backing = b"bar";
    let v = Utf8StringView { data: backing, len: 3 };
    let c = a.concat_view(&v).unwrap();
    assert_eq!(bytes(&c), b"foobar");
}

#[test]
fn test_concat_character() {
    let a = make(b"foo");
    let c = a.concat_character(b'!').unwrap();
    assert_eq!(bytes(&c), b"foo!");
}

#[test]
fn test_concat_literal() {
    let a = make(b"foo");
    let c = a.concat_literal(b"bar").unwrap();
    assert_eq!(bytes(&c), b"foobar");
}

// ----------------- compare -----------------

#[test]
fn test_compare_eq() {
    let a = make(b"abc");
    let b = make(b"abc");
    assert_eq!(a.compare(&b), 0);
}

#[test]
fn test_compare_lt() {
    let a = make(b"abc");
    let b = make(b"abd");
    // strncmp gives -1 in C.
    let r = a.compare(&b);
    assert!(r < 0, "expected negative, got {}", r);
}

#[test]
fn test_compare_gt() {
    let a = make(b"abd");
    let b = make(b"abc");
    let r = a.compare(&b);
    assert!(r > 0, "expected positive, got {}", r);
}

#[test]
fn test_compare_prefix_lt() {
    let a = make(b"abc");
    let b = make(b"abcd");
    assert_eq!(a.compare(&b), -1);
}

#[test]
fn test_compare_prefix_gt() {
    let a = make(b"abcd");
    let b = make(b"abc");
    assert_eq!(a.compare(&b), 1);
}

#[test]
fn test_compare_literal_eq() {
    let a = make(b"abc");
    assert_eq!(a.compare_literal(b"abc"), 0);
}

#[test]
fn test_compare_literal_lt() {
    let a = make(b"abc");
    let r = a.compare_literal(b"abd");
    assert!(r < 0);
}

#[test]
fn test_compare_literal_gt() {
    let a = make(b"abc");
    let r = a.compare_literal(b"abb");
    assert!(r > 0);
}

// ----------------- substring -----------------

#[test]
fn test_substring_normal() {
    let s = make(b"Hello, world");
    let v = s.substring(7, 12);
    assert_eq!(v.len, 5);
    assert_eq!(&v.data[..v.len], b"world");
}

#[test]
fn test_substring_start_gt_end() {
    let s = make(b"Hello, world");
    // start > end -> clamp start to end
    let v = s.substring(100, 5);
    assert_eq!(v.len, 0);
}

#[test]
fn test_substring_end_too_large() {
    let s = make(b"Hello, world");
    let v = s.substring(7, 100);
    assert_eq!(v.len, 5);
    assert_eq!(&v.data[..v.len], b"world");
}

#[test]
fn test_substring_max_end() {
    let s = make(b"Hello, world");
    let v = s.substring(7, usize::MAX);
    assert_eq!(v.len, 5);
    assert_eq!(&v.data[..v.len], b"world");
}

#[test]
fn test_substring_copy() {
    let s = make(b"Hello, world");
    let copied = s.substring_copy(7, 12).unwrap();
    assert_eq!(copied.len, 5);
    assert_eq!(bytes(&copied), b"world");
}

#[test]
fn test_substring_copy_clamp() {
    let s = make(b"Hello");
    let copied = s.substring_copy(2, 100).unwrap();
    assert_eq!(copied.len, 3);
    assert_eq!(bytes(&copied), b"llo");
}

// ----------------- index_of_character / last_index_of_character -----------------

#[test]
fn test_index_of_character_first_match() {
    let s = make(b"Hello, world");
    assert_eq!(s.index_of_character(0, b'l'), Some(2));
}

#[test]
fn test_index_of_character_from_offset() {
    let s = make(b"Hello, world");
    assert_eq!(s.index_of_character(4, b'l'), Some(10));
}

#[test]
fn test_index_of_character_not_found() {
    let s = make(b"Hello, world");
    assert_eq!(s.index_of_character(0, b'z'), None);
}

#[test]
fn test_index_of_character_past_end() {
    let s = make(b"Hello, world");
    // pos == len -> no match
    assert_eq!(s.index_of_character(11, b'l'), None);
}

#[test]
fn test_last_index_of_character_max_pos() {
    let s = make(b"Hello, world");
    // last 'l' is at index 10
    assert_eq!(s.last_index_of_character(usize::MAX, b'l'), Some(10));
}

#[test]
fn test_last_index_of_character_specific_pos() {
    let s = make(b"Hello, world");
    // pos=5: scan idx 5,4,3 -> 'l' at idx 3
    assert_eq!(s.last_index_of_character(5, b'l'), Some(3));
}

#[test]
fn test_last_index_of_character_not_found_in_range() {
    let s = make(b"Hello, world");
    // pos=1: idx 1='e', 0='H' -> not found
    assert_eq!(s.last_index_of_character(1, b'l'), None);
}

#[test]
fn test_last_index_of_character_not_found_at_all() {
    let s = make(b"Hello, world");
    assert_eq!(s.last_index_of_character(usize::MAX, b'z'), None);
}

#[test]
fn test_last_index_of_character_pos_out_of_range() {
    let s = make(b"Hello, world");
    // pos >= len -> None
    assert_eq!(s.last_index_of_character(100, b'l'), None);
}

#[test]
fn test_last_index_of_character_empty() {
    let s = Utf8String::new();
    assert_eq!(s.last_index_of_character(usize::MAX, b'a'), None);
}

// ----------------- Utf8StringView -----------------

#[test]
fn test_view_is_empty() {
    let backing: &[u8] = &[];
    let v = Utf8StringView { data: backing, len: 0 };
    assert_eq!(v.is_empty(), true);
    let backing2: &[u8] = b"a";
    let v2 = Utf8StringView { data: backing2, len: 1 };
    assert_eq!(v2.is_empty(), false);
}

#[test]
fn test_view_compare_eq() {
    let v1 = Utf8StringView { data: b"abc", len: 3 };
    let v2 = Utf8StringView { data: b"abc", len: 3 };
    assert_eq!(v1.compare(&v2), 0);
}

#[test]
fn test_view_compare_lt() {
    let v1 = Utf8StringView { data: b"abc", len: 3 };
    let v2 = Utf8StringView { data: b"abd", len: 3 };
    let r = v1.compare(&v2);
    assert!(r < 0);
}

#[test]
fn test_view_compare_prefix() {
    let v1 = Utf8StringView { data: b"abc", len: 3 };
    let v2 = Utf8StringView { data: b"abcd", len: 4 };
    assert_eq!(v1.compare(&v2), -1);
    assert_eq!(v2.compare(&v1), 1);
}

#[test]
fn test_view_compare_literal_eq() {
    let v = Utf8StringView { data: b"abc", len: 3 };
    assert_eq!(v.compare_literal(b"abc"), 0);
}

#[test]
fn test_view_compare_literal_lt() {
    let v = Utf8StringView { data: b"abc", len: 3 };
    let r = v.compare_literal(b"abd");
    assert!(r < 0);
}

#[test]
fn test_view_substring() {
    let v = Utf8StringView { data: b"Hello, world", len: 12 };
    let v2 = v.substring(7, 12);
    assert_eq!(v2.len, 5);
    assert_eq!(&v2.data[..v2.len], b"world");
}

#[test]
fn test_view_substring_clamp() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let v2 = v.substring(2, 100);
    assert_eq!(v2.len, 3);
    assert_eq!(&v2.data[..v2.len], b"llo");
}

#[test]
fn test_view_substring_start_gt_end() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let v2 = v.substring(100, 1);
    assert_eq!(v2.len, 0);
}

#[test]
fn test_view_substring_max_end() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let v2 = v.substring(2, usize::MAX);
    assert_eq!(v2.len, 3);
    assert_eq!(&v2.data[..v2.len], b"llo");
}

#[test]
fn test_view_substring_copy() {
    let v = Utf8StringView { data: b"Hello", len: 5 };
    let copied = v.substring_copy(2, 100).unwrap();
    assert_eq!(copied.len, 3);
    assert_eq!(bytes(&copied), b"llo");
}

#[test]
fn test_view_index_of_character() {
    let v = Utf8StringView { data: b"Hello, world", len: 12 };
    assert_eq!(v.index_of_character(0, b'l'), Some(2));
    assert_eq!(v.index_of_character(4, b'l'), Some(10));
    assert_eq!(v.index_of_character(0, b'z'), None);
}

#[test]
fn test_view_last_index_of_character() {
    let v = Utf8StringView { data: b"Hello, world", len: 12 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'l'), Some(10));
    assert_eq!(v.last_index_of_character(5, b'l'), Some(3));
    assert_eq!(v.last_index_of_character(1, b'l'), None);
    assert_eq!(v.last_index_of_character(100, b'l'), None);
}

#[test]
fn test_view_last_index_of_character_empty() {
    let v = Utf8StringView { data: &[], len: 0 };
    assert_eq!(v.last_index_of_character(usize::MAX, b'a'), None);
}

// ----------------- Some end-to-end / sequence tests -----------------

#[test]
fn test_sequence_build_then_modify() {
    // Build "Hello, world", then erase ", " -> "Helloworld",
    // then prepend "Cool: " -> "Cool: Helloworld",
    // then insert "$" at pos 4 -> "Cool$: Helloworld".
    let mut s = make(b"Hello");
    s.append_literal(b", ").unwrap();
    s.append_literal(b"world").unwrap();
    assert_eq!(bytes(&s), b"Hello, world");
    s.erase(5, 2).unwrap();
    assert_eq!(bytes(&s), b"Helloworld");
    s.prepend_literal(b"Cool: ").unwrap();
    assert_eq!(bytes(&s), b"Cool: Helloworld");
    s.insert_character(4, b'$').unwrap();
    assert_eq!(bytes(&s), b"Cool$: Helloworld");
    s.destroy();
}

#[test]
fn test_destroy_empty_after_init() {
    // Should not panic when destroying an uninitialized-but-init'd string.
    let mut s = Utf8String::new();
    s.destroy();
    assert_eq!(s.len, 0);
    assert_eq!(s.cap, 0);
}

fn main() {}
