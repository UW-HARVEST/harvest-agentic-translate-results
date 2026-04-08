use impcheck::vec::IntVec;

#[test]
fn test_vec_new() {
    let v = IntVec::new(4);
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 4);
}

#[test]
fn test_vec_push_and_get() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_push(20);
    v.vec_push(30);
    assert_eq!(v.size, 3);
    assert_eq!(v.get_int(0), 10);
    assert_eq!(v.get_int(1), 20);
    assert_eq!(v.get_int(2), 30);
}

#[test]
fn test_vec_push_grows() {
    let mut v = IntVec::new(2);
    v.vec_push(1);
    v.vec_push(2);
    assert_eq!(v.capacity, 2);
    v.vec_push(3); // should trigger growth
    assert!(v.capacity > 2);
    assert_eq!(v.size, 3);
    assert_eq!(v.get_int(0), 1);
    assert_eq!(v.get_int(1), 2);
    assert_eq!(v.get_int(2), 3);
}

#[test]
fn test_vec_reserve() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_push(20);
    v.vec_reserve(10);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 2);
    assert_eq!(v.get_int(0), 10);
    assert_eq!(v.get_int(1), 20);
}

#[test]
fn test_vec_reserve_no_shrink_capacity() {
    let mut v = IntVec::new(10);
    v.vec_reserve(5); // 5 < 10, should not change capacity
    assert_eq!(v.capacity, 10);
}

#[test]
fn test_vec_clear() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_push(20);
    v.vec_clear();
    assert_eq!(v.size, 0);
}

#[test]
fn test_vec_free() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_free();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 0);
}

#[test]
fn test_vec_as_int_slice() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_push(20);
    v.vec_push(30);
    let slice = v.as_int_slice();
    assert_eq!(slice, &[10, 20, 30]);
}

#[test]
fn test_vec_as_int_slice_empty() {
    let v = IntVec::new(4);
    let slice = v.as_int_slice();
    assert_eq!(slice.len(), 0);
}

#[test]
fn test_vec_as_int_slice_mut() {
    let mut v = IntVec::new(4);
    v.vec_push(10);
    v.vec_push(20);
    {
        let slice = v.as_int_slice_mut();
        slice[0] = 99;
    }
    assert_eq!(v.get_int(0), 99);
    assert_eq!(v.get_int(1), 20);
}

#[test]
fn test_vec_push_zero_capacity() {
    let mut v = IntVec::new(0);
    v.vec_push(42);
    assert_eq!(v.size, 1);
    assert!(v.capacity >= 1);
    assert_eq!(v.get_int(0), 42);
}

fn main() {}
