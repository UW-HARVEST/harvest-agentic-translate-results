use SlothLang::stack::{Stack, ListNode};

#[test]
fn test_new_stack_is_empty() {
    let s = Stack::new();
    assert!(s.is_empty());
    // Top and bottom should both be None for the Rust translation; the C
    // version uses an empty sentinel node, but the Rust version returns true
    // for is_empty when top is None, which matches the C definition.
    assert!(s.top.is_none());
    assert!(s.bottom.is_none());
}

#[test]
fn test_push_then_not_empty() {
    let mut s = Stack::new();
    s.push(42);
    assert!(!s.is_empty());
}

#[test]
fn test_push_pop_single() {
    let mut s = Stack::new();
    s.push(7);
    let v = s.pop();
    assert_eq!(v, Some(7));
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
fn test_pop_empty_returns_none() {
    let mut s = Stack::new();
    assert_eq!(s.pop(), None);
}

#[test]
fn test_peek_at_position_zero() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    assert_eq!(s.peek(0), Some(30));
}

#[test]
fn test_peek_deeper_positions() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    assert_eq!(s.peek(0), Some(30));
    assert_eq!(s.peek(1), Some(20));
    assert_eq!(s.peek(2), Some(10));
}

#[test]
fn test_peek_out_of_range_returns_none() {
    let mut s = Stack::new();
    s.push(1);
    assert_eq!(s.peek(1), None);
    assert_eq!(s.peek(5), None);
}

#[test]
fn test_peek_on_empty_returns_none() {
    let s = Stack::new();
    assert_eq!(s.peek(0), None);
}

#[test]
fn test_push_pop_negative_values() {
    let mut s = Stack::new();
    s.push(-1);
    s.push(-100);
    s.push(i32::MIN);
    assert_eq!(s.pop(), Some(i32::MIN));
    assert_eq!(s.pop(), Some(-100));
    assert_eq!(s.pop(), Some(-1));
}

#[test]
fn test_push_pop_max_value() {
    let mut s = Stack::new();
    s.push(i32::MAX);
    assert_eq!(s.pop(), Some(i32::MAX));
}

#[test]
fn test_listnode_construction() {
    let node = ListNode { data: 5, next: None };
    assert_eq!(node.data, 5);
    assert!(node.next.is_none());
}

#[test]
fn test_listnode_with_next() {
    let inner = Box::new(ListNode { data: 1, next: None });
    let node = ListNode { data: 2, next: Some(inner) };
    assert_eq!(node.data, 2);
    assert_eq!(node.next.as_ref().unwrap().data, 1);
}

#[test]
fn test_push_alternating_pop() {
    let mut s = Stack::new();
    s.push(1);
    s.push(2);
    assert_eq!(s.pop(), Some(2));
    s.push(3);
    s.push(4);
    assert_eq!(s.pop(), Some(4));
    assert_eq!(s.pop(), Some(3));
    assert_eq!(s.pop(), Some(1));
    assert!(s.is_empty());
}

#[test]
fn test_print_does_not_panic() {
    let mut s = Stack::new();
    s.push(1);
    s.push(2);
    s.push(3);
    // Just ensure the print method doesn't panic.
    s.print();
}

#[test]
fn test_print_empty_does_not_panic() {
    let s = Stack::new();
    s.print();
}

fn main() {}
