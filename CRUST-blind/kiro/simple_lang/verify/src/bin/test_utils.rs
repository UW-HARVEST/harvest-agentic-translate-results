use simple_lang::utils;

#[test]
fn test_strndup_basic() {
    assert_eq!(utils::strndup("hello", 3), "hel");
}

#[test]
fn test_strndup_full() {
    assert_eq!(utils::strndup("hello", 5), "hello");
}

#[test]
fn test_strndup_exceeds_len() {
    assert_eq!(utils::strndup("hi", 10), "hi");
}

#[test]
fn test_strndup_zero() {
    assert_eq!(utils::strndup("hello", 0), "");
}

#[test]
fn test_strndup_empty() {
    assert_eq!(utils::strndup("", 5), "");
}

#[test]
fn test_simple_lang_utils_h() {
    assert_eq!(utils::SIMPLE_LANG_UTILS_H, true);
}

fn main() {}
