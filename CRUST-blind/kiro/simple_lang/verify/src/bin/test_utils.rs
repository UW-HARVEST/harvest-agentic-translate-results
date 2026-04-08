use simple_lang::utils;

#[test]
fn test_strndup_basic() {
    assert_eq!(utils::strndup("hello", 5), "hello");
}

#[test]
fn test_strndup_partial() {
    assert_eq!(utils::strndup("hello", 3), "hel");
}

#[test]
fn test_strndup_zero() {
    assert_eq!(utils::strndup("hello", 0), "");
}

#[test]
fn test_strndup_exceeds_length() {
    assert_eq!(utils::strndup("hi", 10), "hi");
}

#[test]
fn test_strndup_empty_string() {
    assert_eq!(utils::strndup("", 5), "");
}

fn main() {}
