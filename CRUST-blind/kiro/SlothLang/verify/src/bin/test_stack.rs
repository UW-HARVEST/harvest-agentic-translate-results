use SlothLang::stack::Stack;

#[test]
fn test_new_stack_is_empty() {
    let s = Stack::new();
    assert!(s.is_empty());
}

#[test]
fn test_push_makes_non_empty() {
    let mut s = Stack::new();
    s.push(10);
    assert!(!s.is_empty());
}

#[test]
fn test_push_pop_single() {
    let mut s = Stack::new();
    s.push(10);
    assert_eq!(s.pop(), Some(10));
    assert!(s.is_empty());
}

#[test]
fn test_push_pop_lifo() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    assert_eq!(s.pop(), Some(30));
    assert_eq!(s.pop(), Some(20));
    assert_eq!(s.pop(), Some(10));
    assert!(s.is_empty());
}

#[test]
fn test_peek() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    assert_eq!(s.peek(0), Some(30));
    assert_eq!(s.peek(1), Some(20));
    assert_eq!(s.peek(2), Some(10));
}

#[test]
fn test_pop_empty_returns_none() {
    let mut s = Stack::new();
    assert_eq!(s.pop(), None);
}

#[test]
fn test_peek_out_of_bounds() {
    let mut s = Stack::new();
    s.push(10);
    assert_eq!(s.peek(1), None);
}

fn main() {}
