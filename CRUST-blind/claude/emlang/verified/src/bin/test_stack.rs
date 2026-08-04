use emlang::data::{Data, DataType, DataValue};
use emlang::stack::{Stack, DEFAULT_POPPED_CAP, DEFAULT_STACK_CAP};

#[test]
fn test_constants() {
    assert_eq!(DEFAULT_POPPED_CAP, 32);
    assert_eq!(DEFAULT_STACK_CAP, 1024);
}

#[test]
fn test_stack_make() {
    // C: stack_new(8, 4) -> {cap=8, size=0, popped_cap=4, popped_size=0}
    let s = Stack::make(8, 4);
    assert_eq!(s.cap, 8);
    assert_eq!(s.size, 0);
    assert_eq!(s.popped_cap, 4);
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_do_push_basic() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(10));
    assert_eq!(s.size, 1);
    s.do_push(Data::new_int(20));
    s.do_push(Data::new_int(30));
    assert_eq!(s.size, 3);
    // Top item is 30
    match &s.buf[s.size - 1].value {
        DataValue::Int(i) => assert_eq!(*i, 30),
        _ => panic!("expected Int"),
    }
    // Stack is [10, 20, 30] in order
    match &s.buf[0].value {
        DataValue::Int(i) => assert_eq!(*i, 10),
        _ => panic!("expected Int"),
    }
    match &s.buf[1].value {
        DataValue::Int(i) => assert_eq!(*i, 20),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_pop_basic() {
    // C reference: r=0, val=30, size_after=2
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(10));
    s.do_push(Data::new_int(20));
    s.do_push(Data::new_int(30));
    let r = s.do_pop();
    assert!(r.is_some());
    let d = r.unwrap();
    assert_eq!(d.dtype, DataType::Int);
    match d.value {
        DataValue::Int(i) => assert_eq!(i, 30),
        _ => panic!("expected Int"),
    }
    assert_eq!(s.size, 2);
}

#[test]
fn test_do_pop_empty() {
    // C: stack_pop on empty -> -1
    let mut s = Stack::make(8, 4);
    assert!(s.do_pop().is_none());
    assert_eq!(s.size, 0);
}

#[test]
fn test_do_pop_str_adds_popped() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_str("hello".to_string()));
    assert_eq!(s.popped_size, 0);
    let _ = s.do_pop();
    assert_eq!(s.popped_size, 1);
}

#[test]
fn test_do_dup_off0() {
    // C: stack_dup off=0 duplicates top
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(10));
    s.do_push(Data::new_int(20));
    let r = s.do_dup(0);
    assert_eq!(r, 0);
    assert_eq!(s.size, 3);
    match &s.buf[2].value {
        DataValue::Int(i) => assert_eq!(*i, 20),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_dup_off1() {
    // C: dup off=1 duplicates 2nd from top
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(10));
    s.do_push(Data::new_int(20));
    let r = s.do_dup(1);
    assert_eq!(r, 0);
    assert_eq!(s.size, 3);
    match &s.buf[2].value {
        DataValue::Int(i) => assert_eq!(*i, 10),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_dup_invalid() {
    // C: stack_dup off too large -> -1
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(10));
    let r = s.do_dup(100);
    assert_eq!(r, -1);
    assert_eq!(s.size, 1);
}

#[test]
fn test_do_dup_empty() {
    let mut s = Stack::make(8, 4);
    let r = s.do_dup(0);
    assert_eq!(r, -1);
}

#[test]
fn test_do_swap_off0() {
    // C: swap off=0 -> swaps top with itself, no change
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    s.do_push(Data::new_int(3));
    let r = s.do_swap(0);
    assert_eq!(r, 0);
    match &s.buf[2].value {
        DataValue::Int(i) => assert_eq!(*i, 3),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_swap_off1() {
    // C: stack [1,2,3], swap off=1 swaps 2 with 3 -> [1,3,2]
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    s.do_push(Data::new_int(3));
    let r = s.do_swap(1);
    assert_eq!(r, 0);
    match &s.buf[0].value {
        DataValue::Int(i) => assert_eq!(*i, 1),
        _ => panic!("expected Int"),
    }
    match &s.buf[1].value {
        DataValue::Int(i) => assert_eq!(*i, 3),
        _ => panic!("expected Int"),
    }
    match &s.buf[2].value {
        DataValue::Int(i) => assert_eq!(*i, 2),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_swap_off2() {
    // stack [1,2,3], swap off=2 -> [3,2,1]
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    s.do_push(Data::new_int(3));
    let r = s.do_swap(2);
    assert_eq!(r, 0);
    match &s.buf[0].value {
        DataValue::Int(i) => assert_eq!(*i, 3),
        _ => panic!("expected Int"),
    }
    match &s.buf[2].value {
        DataValue::Int(i) => assert_eq!(*i, 1),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_swap_invalid() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    let r = s.do_swap(100);
    assert_eq!(r, -1);
}

#[test]
fn test_do_shrink_to() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    s.do_push(Data::new_int(3));
    s.do_shrink_to(1);
    assert_eq!(s.size, 1);
    match &s.buf[0].value {
        DataValue::Int(i) => assert_eq!(*i, 1),
        _ => panic!("expected Int"),
    }
}

#[test]
fn test_do_shrink_to_same_size() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    s.do_shrink_to(2);
    assert_eq!(s.size, 2);
}

#[test]
fn test_do_shrink_to_str_adds_popped() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_str("a".to_string()));
    s.do_push(Data::new_str("b".to_string()));
    s.do_push(Data::new_int(3));
    s.do_shrink_to(0);
    assert_eq!(s.size, 0);
    // Two strings should have been added to popped.
    assert_eq!(s.popped_size, 2);
}

#[test]
fn test_do_clear() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_str("foo".to_string()));
    s.do_clear();
    assert_eq!(s.size, 0);
    // gc was called, which clears popped
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_do_gc_empty() {
    let mut s = Stack::make(8, 4);
    s.do_gc();
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_do_gc_after_pop() {
    let mut s = Stack::make(8, 4);
    s.do_push(Data::new_str("a".to_string()));
    let _ = s.do_pop();
    assert_eq!(s.popped_size, 1);
    s.do_gc();
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_do_push_grows() {
    // C: push beyond cap -> cap doubles
    let mut s = Stack::make(2, 2);
    s.do_push(Data::new_int(1));
    s.do_push(Data::new_int(2));
    assert_eq!(s.size, 2);
    // Triggers cap growth
    s.do_push(Data::new_int(3));
    assert_eq!(s.size, 3);
    assert_eq!(s.cap, 4);
}

fn main() {}
