use emlang::utils::strcpy_to_heap;

#[test]
fn test_strcpy_to_heap() {
    let s = strcpy_to_heap("hello");
    assert_eq!(s, "hello");
}

#[test]
fn test_strcpy_to_heap_empty() {
    let s = strcpy_to_heap("");
    assert_eq!(s, "");
}

fn main() {}
