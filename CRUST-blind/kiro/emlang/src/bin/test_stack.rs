use emlang::data::{Data, DataType, DataValue};
use emlang::stack::Stack;

#[test]
fn test_new() {
    let s = Stack::new(16, 4);
    assert_eq!(s.size, 0);
    assert_eq!(s.cap, 16);
    assert_eq!(s.popped_size, 0);
    assert_eq!(s.popped_cap, 4);
}

#[test]
fn test_push_pop_int() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(42));
    assert_eq!(s.size, 1);
    let d = s.pop().unwrap();
    assert_eq!(s.size, 0);
    assert!(matches!(d.value, DataValue::Int(42)));
}

#[test]
fn test_push_pop_str() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_str("hello".to_string()));
    assert_eq!(s.size, 1);
    let d = s.pop().unwrap();
    assert_eq!(s.size, 0);
    assert_eq!(d.dtype, DataType::Str);
    // Popping a string should add it to popped list
    assert_eq!(s.popped_size, 1);
}

#[test]
fn test_pop_empty() {
    let mut s = Stack::new(16, 4);
    assert!(s.pop().is_none());
}

#[test]
fn test_push_multiple() {
    let mut s = Stack::new(16, 4);
    for i in 0..5 {
        s.push(Data::new_int(i));
    }
    assert_eq!(s.size, 5);
    // Pop in LIFO order
    for i in (0..5).rev() {
        let d = s.pop().unwrap();
        assert!(matches!(d.value, DataValue::Int(v) if v == i));
    }
}

#[test]
fn test_dup_top() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    assert_eq!(s.dup(0), 0); // dup top element
    assert_eq!(s.size, 3);
    let d = s.pop().unwrap();
    assert!(matches!(d.value, DataValue::Int(20)));
}

#[test]
fn test_dup_offset() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    assert_eq!(s.dup(1), 0); // dup element at offset 1 (the 10)
    assert_eq!(s.size, 3);
    let d = s.pop().unwrap();
    assert!(matches!(d.value, DataValue::Int(10)));
}

#[test]
fn test_dup_invalid() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(10));
    assert_eq!(s.dup(1), -1); // offset too large
}

#[test]
fn test_dup_empty() {
    let mut s = Stack::new(16, 4);
    assert_eq!(s.dup(0), -1);
}

#[test]
fn test_swap_top() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    assert_eq!(s.swap(1), 0);
    // Now top should be 10, bottom should be 20
    let top = s.pop().unwrap();
    let bot = s.pop().unwrap();
    assert!(matches!(top.value, DataValue::Int(10)));
    assert!(matches!(bot.value, DataValue::Int(20)));
}

#[test]
fn test_swap_invalid() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(10));
    assert_eq!(s.swap(1), -1);
}

#[test]
fn test_shrink_to() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_str("a".to_string()));
    s.push(Data::new_int(3));
    assert_eq!(s.size, 3);
    s.shrink_to(1);
    assert_eq!(s.size, 1);
    // The string "a" was shrunk away, should be in popped
    assert_eq!(s.popped_size, 1);
}

#[test]
fn test_shrink_to_same_size() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(1));
    s.shrink_to(1); // no-op
    assert_eq!(s.size, 1);
}

#[test]
fn test_clear() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_str("x".to_string()));
    s.clear();
    assert_eq!(s.size, 0);
    assert_eq!(s.popped_size, 0); // gc clears popped
}

#[test]
fn test_gc() {
    let mut s = Stack::new(16, 4);
    s.push(Data::new_str("a".to_string()));
    s.pop(); // adds to popped
    assert_eq!(s.popped_size, 1);
    s.gc();
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_gc_empty() {
    let mut s = Stack::new(16, 4);
    s.gc(); // no-op when popped_size == 0
    assert_eq!(s.popped_size, 0);
}

fn main() {}
