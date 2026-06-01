use emlang::utils;

#[test]
fn test_strcpy_to_heap_simple() {
    let s = utils::strcpy_to_heap("hello");
    assert_eq!(s, "hello");
}

#[test]
fn test_strcpy_to_heap_empty() {
    let s = utils::strcpy_to_heap("");
    assert_eq!(s, "");
}

#[test]
fn test_strcpy_to_heap_special() {
    let s = utils::strcpy_to_heap("Hello, world!\n");
    assert_eq!(s, "Hello, world!\n");
}

fn main() {}
