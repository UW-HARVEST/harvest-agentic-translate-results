use SlothLang::stack::Stack;

#[test]
fn test_new_stack_is_empty() {
    let s = Stack::new();
    assert!(s.is_empty());
}

#[test]
fn test_push_then_not_empty() {
    let mut s = Stack::new();
    s.push(1);
    assert!(!s.is_empty());
}

#[test]
fn test_push_pop_single() {
    let mut s = Stack::new();
    s.push(42);
    let popped = s.pop();
    assert_eq!(popped, Some(42));
    assert!(s.is_empty());
}

#[test]
fn test_push_pop_lifo_order() {
    let mut s = Stack::new();
    s.push(1);
    s.push(2);
    s.push(3);
    assert_eq!(s.pop(), Some(3));
    assert_eq!(s.pop(), Some(2));
    assert_eq!(s.pop(), Some(1));
    assert!(s.is_empty());
}

#[test]
fn test_peek_at_positions() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    // Top is 30, then 20, then 10.
    assert_eq!(s.peek(0), Some(30));
    assert_eq!(s.peek(1), Some(20));
    assert_eq!(s.peek(2), Some(10));
}

#[test]
fn test_peek_does_not_modify() {
    let mut s = Stack::new();
    s.push(5);
    s.push(7);
    let _ = s.peek(0);
    let _ = s.peek(1);
    assert_eq!(s.pop(), Some(7));
    assert_eq!(s.pop(), Some(5));
    assert!(s.is_empty());
}

#[test]
fn test_negative_values() {
    let mut s = Stack::new();
    s.push(-1);
    s.push(-2);
    s.push(0);
    assert_eq!(s.pop(), Some(0));
    assert_eq!(s.pop(), Some(-2));
    assert_eq!(s.pop(), Some(-1));
}

#[test]
fn test_large_values() {
    let mut s = Stack::new();
    s.push(i32::MAX);
    s.push(i32::MIN);
    assert_eq!(s.pop(), Some(i32::MIN));
    assert_eq!(s.pop(), Some(i32::MAX));
    assert!(s.is_empty());
}

#[test]
fn test_print_does_not_panic() {
    let mut s = Stack::new();
    s.push(1);
    s.push(2);
    s.print();
    s.pop();
    s.pop();
    let empty = Stack::new();
    empty.print();
}

fn main() {}
