use emlang::data::{Data, DataType, DataValue};
use emlang::stack::{Stack, DEFAULT_POPPED_CAP, DEFAULT_STACK_CAP};

#[test]
fn test_constants() {
    assert_eq!(DEFAULT_STACK_CAP, 1024);
    assert_eq!(DEFAULT_POPPED_CAP, 32);
}

#[test]
fn test_new() {
    let s = Stack::new(8, 4);
    assert_eq!(s.cap, 8);
    assert_eq!(s.popped_cap, 4);
    assert_eq!(s.size, 0);
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_push_pop_int() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    assert_eq!(s.size, 2);
    let v = s.pop().expect("pop should succeed");
    assert_eq!(v.dtype, DataType::Int);
    match v.value {
        DataValue::Int(i) => assert_eq!(i, 20),
        _ => panic!("expected Int"),
    }
    assert_eq!(s.size, 1);
    let v2 = s.pop().expect("pop");
    match v2.value {
        DataValue::Int(i) => assert_eq!(i, 10),
        _ => panic!("expected Int"),
    }
    assert_eq!(s.size, 0);
}

#[test]
fn test_pop_empty() {
    let mut s = Stack::new(8, 4);
    assert!(s.pop().is_none());
}

#[test]
fn test_dup() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.push(Data::new_int(3));
    // Duplicate top of stack (offset 0)
    assert_eq!(s.dup(0), 0);
    assert_eq!(s.size, 4);
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 3);
    } else {
        panic!()
    }
    // Duplicate offset 2 (which is index size-2-1 = 0, so value 1)
    assert_eq!(s.dup(2), 0);
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 1);
    } else {
        panic!()
    }
}

#[test]
fn test_dup_invalid() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    // Out of bounds
    assert_eq!(s.dup(1), -1);
    assert_eq!(s.dup(5), -1);
}

#[test]
fn test_swap() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.push(Data::new_int(3));
    // Swap top with offset 2 -> swaps index 0 and 2
    assert_eq!(s.swap(2), 0);
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 1);
    } else {
        panic!()
    }
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 2);
    } else {
        panic!()
    }
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 3);
    } else {
        panic!()
    }
}

#[test]
fn test_swap_invalid() {
    let mut s = Stack::new(8, 4);
    assert_eq!(s.swap(0), -1);
    s.push(Data::new_int(1));
    assert_eq!(s.swap(1), -1);
}

#[test]
fn test_shrink_to() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.push(Data::new_int(3));
    s.shrink_to(1);
    assert_eq!(s.size, 1);
    let v = s.pop().unwrap();
    if let DataValue::Int(i) = v.value {
        assert_eq!(i, 1);
    } else {
        panic!()
    }
}

#[test]
fn test_shrink_to_same_size() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.shrink_to(2);
    assert_eq!(s.size, 2);
}

#[test]
fn test_clear() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.clear();
    assert_eq!(s.size, 0);
}

#[test]
fn test_string_push_pop() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_str("hello".to_string()));
    let v = s.pop().unwrap();
    assert_eq!(v.dtype, DataType::Str);
    if let DataValue::Str(st) = v.value {
        assert_eq!(st, "hello");
    } else {
        panic!()
    }
}

#[test]
fn test_gc_after_pops() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_str("hello".to_string()));
    s.push(Data::new_str("world".to_string()));
    let _ = s.pop().unwrap();
    let _ = s.pop().unwrap();
    assert!(s.popped_size > 0);
    s.gc();
    assert_eq!(s.popped_size, 0);
}

fn main() {}
