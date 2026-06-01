use simple_lang::utils;

#[test]
fn test_strndup_basic() {
    let result = utils::strndup("hello world", 5);
    assert_eq!(result, "hello");
}

#[test]
fn test_strndup_full_length() {
    let result = utils::strndup("abc", 3);
    assert_eq!(result, "abc");
}

#[test]
fn test_strndup_n_larger_than_string() {
    let result = utils::strndup("abc", 10);
    assert_eq!(result, "abc");
}

#[test]
fn test_strndup_zero() {
    let result = utils::strndup("hello", 0);
    assert_eq!(result, "");
}

#[test]
fn test_strndup_empty() {
    let result = utils::strndup("", 5);
    assert_eq!(result, "");
}

fn main() {}
