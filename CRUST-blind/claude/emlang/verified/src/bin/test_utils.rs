use emlang::utils::strcpy_to_heap;

#[test]
fn test_strcpy_to_heap_empty() {
    // C: strcpy_to_heap("") -> "" on heap
    let s = strcpy_to_heap("");
    assert_eq!(s, "");
    assert_eq!(s.len(), 0);
}

#[test]
fn test_strcpy_to_heap_simple() {
    // C: strcpy_to_heap("hello") -> "hello"
    let s = strcpy_to_heap("hello");
    assert_eq!(s, "hello");
    assert_eq!(s.len(), 5);
}

#[test]
fn test_strcpy_to_heap_with_special_chars() {
    let s = strcpy_to_heap("a\nb\tc");
    assert_eq!(s, "a\nb\tc");
    assert_eq!(s.len(), 5);
}

#[test]
fn test_strcpy_to_heap_owned_string() {
    // The returned string should be an owned, distinct copy.
    let original = "test";
    let copy = strcpy_to_heap(original);
    assert_eq!(copy, original);
}

fn main() {}
