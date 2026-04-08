use impcheck::vec::IntVec;

#[test]
fn test_intvec_initial_state() {
    let data = [0u8; 32];
    let v = IntVec {
        capacity: 10,
        size: 0,
        data: &data,
    };
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 0);
}

#[test]
fn test_vec_clear() {
    let data = [0u8; 32];
    let mut v = IntVec {
        capacity: 10,
        size: 5,
        data: &data,
    };
    v.vec_clear();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 10);
}

#[test]
fn test_vec_free() {
    let data = [0u8; 32];
    let mut v = IntVec {
        capacity: 10,
        size: 5,
        data: &data,
    };
    v.vec_free();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 0);
}

#[test]
fn test_vec_reserve_shrink() {
    let data = [0u8; 32];
    let mut v = IntVec {
        capacity: 10,
        size: 8,
        data: &data,
    };
    // Reserve smaller than size should shrink size
    v.vec_reserve(3);
    assert_eq!(v.size, 3);
}

#[test]
fn test_vec_reserve_no_shrink() {
    let data = [0u8; 32];
    let mut v = IntVec {
        capacity: 10,
        size: 3,
        data: &data,
    };
    // Reserve larger than size should not change size
    v.vec_reserve(8);
    assert_eq!(v.size, 3);
}

#[test]
fn test_vec_push_increments_size() {
    let data = [0u8; 32];
    let mut v = IntVec {
        capacity: 10,
        size: 0,
        data: &data,
    };
    v.vec_push(42);
    assert_eq!(v.size, 1);
    v.vec_push(99);
    assert_eq!(v.size, 2);
}

fn main() {}
