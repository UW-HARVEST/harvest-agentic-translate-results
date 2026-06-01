use impcheck::vec::IntVec;

#[test]
fn test_init_empty() {
    let mut v = IntVec::vec_init(0);
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 0);
    v.vec_free();
}

#[test]
fn test_init_size() {
    let mut v = IntVec::vec_init(8);
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 8);
    v.vec_free();
}

#[test]
fn test_push_simple() {
    let mut v = IntVec::vec_init(4);
    v.vec_push(10);
    v.vec_push(20);
    v.vec_push(30);
    assert_eq!(v.size, 3);
    assert_eq!(v.capacity, 4);
    assert_eq!(v.get(0), 10);
    assert_eq!(v.get(1), 20);
    assert_eq!(v.get(2), 30);
    v.vec_free();
}

#[test]
fn test_push_grows() {
    let mut v = IntVec::vec_init(2);
    v.vec_push(1);
    v.vec_push(2);
    assert_eq!(v.capacity, 2);
    assert_eq!(v.size, 2);
    // C: new_cap = 2*1.3 = 2; if new_cap < 3: new_cap = 3
    v.vec_push(3);
    assert_eq!(v.capacity, 3);
    assert_eq!(v.size, 3);
    assert_eq!(v.get(0), 1);
    assert_eq!(v.get(1), 2);
    assert_eq!(v.get(2), 3);
    v.vec_free();
}

#[test]
fn test_push_grows_from_zero() {
    let mut v = IntVec::vec_init(0);
    v.vec_push(7);
    // 0*1.3 = 0; 0 < 1, so new_cap = 1
    assert_eq!(v.capacity, 1);
    assert_eq!(v.size, 1);
    assert_eq!(v.get(0), 7);

    v.vec_push(8);
    // 1*1.3 = 1; 1 < 2 so new_cap = 2
    assert_eq!(v.capacity, 2);
    assert_eq!(v.size, 2);
    assert_eq!(v.get(1), 8);
    v.vec_free();
}

#[test]
fn test_push_grows_large() {
    let mut v = IntVec::vec_init(10);
    for i in 0..10 {
        v.vec_push(i);
    }
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 10);
    v.vec_push(99);
    // 10*1.3 = 13
    assert_eq!(v.capacity, 13);
    assert_eq!(v.size, 11);
    assert_eq!(v.get(10), 99);
    v.vec_free();
}

#[test]
fn test_clear() {
    let mut v = IntVec::vec_init(8);
    v.vec_push(1);
    v.vec_push(2);
    assert_eq!(v.size, 2);
    v.vec_clear();
    // C: vec_clear -> reserve(0): new_cap (0) > capacity (8) is false, no realloc;
    //    if (vec->size > new_cap) -> shrink size to 0.
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 8);
    v.vec_free();
}

#[test]
fn test_reserve_grows() {
    let mut v = IntVec::vec_init(4);
    v.vec_push(11);
    v.vec_push(22);
    v.vec_reserve(16);
    assert_eq!(v.capacity, 16);
    assert_eq!(v.size, 2);
    assert_eq!(v.get(0), 11);
    assert_eq!(v.get(1), 22);
    v.vec_free();
}

#[test]
fn test_reserve_no_grow() {
    let mut v = IntVec::vec_init(8);
    v.vec_push(1);
    v.vec_push(2);
    v.vec_push(3);
    // Reserve smaller than capacity: capacity unchanged but size shrinks if > new_cap
    v.vec_reserve(2);
    assert_eq!(v.capacity, 8);
    assert_eq!(v.size, 2);
    v.vec_free();
}

#[test]
fn test_free() {
    let mut v = IntVec::vec_init(8);
    v.vec_push(5);
    v.vec_free();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 0);
}

fn main() {}
