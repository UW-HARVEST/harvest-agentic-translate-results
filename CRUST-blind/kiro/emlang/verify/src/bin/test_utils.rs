use emlang::utils;

#[test]
fn test_strcpy_to_heap_basic() {
    let result = utils::strcpy_to_heap("hello");
    assert_eq!(result, "hello");
}

#[test]
fn test_strcpy_to_heap_empty() {
    let result = utils::strcpy_to_heap("");
    assert_eq!(result, "");
}

#[test]
fn test_strcpy_to_heap_special_chars() {
    let result = utils::strcpy_to_heap("hello\nworld\t!");
    assert_eq!(result, "hello\nworld\t!");
}

fn main() {}
