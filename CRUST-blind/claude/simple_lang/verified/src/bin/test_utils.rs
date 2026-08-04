use simple_lang::utils::strndup;

#[test]
fn test_strndup_truncate() {
    let s = strndup("hello world", 5);
    assert_eq!(s, "hello");
}

#[test]
fn test_strndup_full() {
    let s = strndup("hello", 10);
    assert_eq!(s, "hello");
}

#[test]
fn test_strndup_zero() {
    let s = strndup("hello", 0);
    assert_eq!(s, "");
}

#[test]
fn test_strndup_empty() {
    let s = strndup("", 5);
    assert_eq!(s, "");
}

#[test]
fn test_strndup_exact() {
    let s = strndup("abcd", 4);
    assert_eq!(s, "abcd");
}

#[test]
fn test_strndup_one() {
    let s = strndup("xyz", 1);
    assert_eq!(s, "x");
}

fn main() {}
