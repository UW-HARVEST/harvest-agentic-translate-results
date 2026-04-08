use emlang::data::{Data, DataType, DataValue};
use emlang::stack::Stack;

#[test]
fn test_stack_new() {
    let s = Stack::new(8, 4);
    assert_eq!(s.cap, 8);
    assert_eq!(s.size, 0);
    assert_eq!(s.popped_cap, 4);
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_push_and_size() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    s.push(Data::new_int(30));
    assert_eq!(s.size, 3);
}

#[test]
fn test_pop_lifo() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    s.push(Data::new_int(30));

    let d = s.pop().unwrap();
    assert_eq!(d.dtype, DataType::Int);
    match d.value { DataValue::Int(v) => assert_eq!(v, 30), _ => panic!() }
    assert_eq!(s.size, 2);

    let d = s.pop().unwrap();
    match d.value { DataValue::Int(v) => assert_eq!(v, 20), _ => panic!() }
    assert_eq!(s.size, 1);

    let d = s.pop().unwrap();
    match d.value { DataValue::Int(v) => assert_eq!(v, 10), _ => panic!() }
    assert_eq!(s.size, 0);
}

#[test]
fn test_pop_empty() {
    let mut s = Stack::new(8, 4);
    assert!(s.pop().is_none());
}

#[test]
fn test_dup_top() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(100));
    s.push(Data::new_int(200));
    s.push(Data::new_int(300));
    let ret = s.dup(0);
    assert_eq!(ret, 0);
    assert_eq!(s.size, 4);
    match s.buf[s.size - 1].value { DataValue::Int(v) => assert_eq!(v, 300), _ => panic!() }
}

#[test]
fn test_dup_deep() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(100));
    s.push(Data::new_int(200));
    s.push(Data::new_int(300));
    s.dup(0); // size=4, top=300
    let ret = s.dup(3);
    assert_eq!(ret, 0);
    assert_eq!(s.size, 5);
    match s.buf[s.size - 1].value { DataValue::Int(v) => assert_eq!(v, 100), _ => panic!() }
}

#[test]
fn test_dup_out_of_range() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(100));
    s.push(Data::new_int(200));
    s.push(Data::new_int(300));
    s.dup(0);
    s.dup(3);
    let ret = s.dup(99);
    assert_eq!(ret, -1);
}

#[test]
fn test_swap() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.push(Data::new_int(3));
    let ret = s.swap(2);
    assert_eq!(ret, 0);
    // buf[0] was 1, buf[2] was 3 -> swapped
    match s.buf[0].value { DataValue::Int(v) => assert_eq!(v, 3), _ => panic!() }
    match s.buf[2].value { DataValue::Int(v) => assert_eq!(v, 1), _ => panic!() }
}

#[test]
fn test_swap_out_of_range() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(1));
    s.push(Data::new_int(2));
    s.push(Data::new_int(3));
    let ret = s.swap(99);
    assert_eq!(ret, -1);
}

#[test]
fn test_shrink_to() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    s.push(Data::new_int(30));
    s.push(Data::new_int(40));
    s.shrink_to(2);
    assert_eq!(s.size, 2);
    match s.buf[0].value { DataValue::Int(v) => assert_eq!(v, 10), _ => panic!() }
    match s.buf[1].value { DataValue::Int(v) => assert_eq!(v, 20), _ => panic!() }
}

#[test]
fn test_clear() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_int(10));
    s.push(Data::new_int(20));
    s.clear();
    assert_eq!(s.size, 0);
    assert_eq!(s.popped_size, 0);
}

#[test]
fn test_pop_string_adds_to_popped() {
    let mut s = Stack::new(8, 4);
    s.push(Data::new_str("hello".to_string()));
    s.pop();
    assert_eq!(s.popped_size, 1);
    assert_eq!(s.popped[0].str, "hello");
    assert_eq!(s.popped[0].marked, false);
}

fn main() {}
