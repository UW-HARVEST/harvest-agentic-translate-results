use SlothLang::stack::Stack;

#[test]
fn test_new_stack_is_empty() {
    let s = Stack::new();
    assert!(s.is_empty());
}

#[test]
fn test_push_makes_non_empty() {
    let mut s = Stack::new();
    s.push(42);
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
fn test_peek_top() {
    let mut s = Stack::new();
    s.push(5);
    s.push(10);
    assert_eq!(s.peek(0), Some(10));
    assert_eq!(s.peek(1), Some(5));
}

#[test]
fn test_peek_out_of_bounds() {
    let s = Stack::new();
    assert_eq!(s.peek(0), None);
}

#[test]
fn test_peek_does_not_remove() {
    let mut s = Stack::new();
    s.push(7);
    assert_eq!(s.peek(0), Some(7));
    assert_eq!(s.peek(0), Some(7));
    assert!(!s.is_empty());
}

#[test]
fn test_push_zero() {
    let mut s = Stack::new();
    s.push(0);
    assert!(!s.is_empty());
    assert_eq!(s.pop(), Some(0));
}

#[test]
fn test_push_negative() {
    let mut s = Stack::new();
    s.push(-1);
    assert_eq!(s.pop(), Some(-1));
}

#[test]
fn test_many_pushes() {
    let mut s = Stack::new();
    for i in 0..100 {
        s.push(i);
    }
    for i in (0..100).rev() {
        assert_eq!(s.pop(), Some(i));
    }
    assert!(s.is_empty());
}

#[test]
fn test_peek_deep() {
    let mut s = Stack::new();
    s.push(10);
    s.push(20);
    s.push(30);
    assert_eq!(s.peek(0), Some(30));
    assert_eq!(s.peek(1), Some(20));
    assert_eq!(s.peek(2), Some(10));
    assert_eq!(s.peek(3), None);
}

#[test]
fn test_interleaved_push_pop() {
    let mut s = Stack::new();
    s.push(1);
    s.push(2);
    assert_eq!(s.pop(), Some(2));
    s.push(3);
    assert_eq!(s.pop(), Some(3));
    assert_eq!(s.pop(), Some(1));
    assert!(s.is_empty());
}

fn main() {}
