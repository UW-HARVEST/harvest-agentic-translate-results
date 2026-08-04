use impcheck::vec::IntVec;

fn make_vec(capacity: u64) -> IntVec<'static> {
    IntVec {
        capacity,
        size: 0,
        data: &[],
    }
}

#[test]
fn test_initial_state() {
    let v = make_vec(8);
    assert_eq!(v.capacity, 8);
    assert_eq!(v.size, 0);
    assert!(v.data.is_empty());
}

#[test]
fn test_vec_free_clears_fields() {
    let mut v = IntVec {
        capacity: 16,
        size: 5,
        data: &[],
    };
    v.vec_free();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 0);
    assert!(v.data.is_empty());
}

#[test]
fn test_vec_clear_size_to_zero() {
    let mut v = IntVec {
        capacity: 16,
        size: 5,
        data: &[],
    };
    v.vec_clear();
    assert_eq!(v.size, 0);
    // capacity unchanged from this clear path (mirrors C: vec_reserve(0) shrinks size only)
    assert_eq!(v.capacity, 16);
}

#[test]
fn test_vec_clear_already_empty() {
    let mut v = IntVec {
        capacity: 16,
        size: 0,
        data: &[],
    };
    v.vec_clear();
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 16);
}

#[test]
fn test_vec_reserve_grows_capacity() {
    // C: when new_cap > capacity, capacity grows.
    let mut v = make_vec(4);
    v.vec_reserve(10);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 0);
}

#[test]
fn test_vec_reserve_no_shrink_capacity() {
    // C: when new_cap <= capacity, capacity is not shrunk.
    let mut v = make_vec(10);
    v.vec_reserve(5);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 0);
}

#[test]
fn test_vec_reserve_shrinks_size() {
    // C: vec_reserve sets size to new_cap if size > new_cap.
    let mut v = IntVec {
        capacity: 10,
        size: 8,
        data: &[],
    };
    v.vec_reserve(3);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 3);
}

#[test]
fn test_vec_reserve_zero_shrinks_size_only() {
    let mut v = IntVec {
        capacity: 10,
        size: 8,
        data: &[],
    };
    v.vec_reserve(0);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 0);
}

#[test]
fn test_vec_push_from_zero_capacity() {
    // C: capacity=0, push: new_cap = (0 * 1.3) = 0, then max(new_cap, 0+1) = 1.
    let mut v = make_vec(0);
    v.vec_push(42);
    assert_eq!(v.size, 1);
    assert_eq!(v.capacity, 1);
}

#[test]
fn test_vec_push_grows_when_full() {
    // C: when size == capacity, new_cap = (capacity * 1.3) -> truncated to int
    // capacity=4 -> new_cap = (int)(4*1.3) = 5; max(5, 4+1)=5
    let mut v = IntVec {
        capacity: 4,
        size: 4,
        data: &[],
    };
    v.vec_push(1);
    assert_eq!(v.capacity, 5);
    assert_eq!(v.size, 5);
}

#[test]
fn test_vec_push_grows_when_full_capacity_10() {
    // capacity=10, size=10 -> new_cap = (int)(10*1.3) = 13
    let mut v = IntVec {
        capacity: 10,
        size: 10,
        data: &[],
    };
    v.vec_push(7);
    assert_eq!(v.capacity, 13);
    assert_eq!(v.size, 11);
}

#[test]
fn test_vec_push_no_grow_when_capacity_remains() {
    // capacity=10, size=3 -> push shouldn't trigger grow
    let mut v = IntVec {
        capacity: 10,
        size: 3,
        data: &[],
    };
    v.vec_push(99);
    assert_eq!(v.capacity, 10);
    assert_eq!(v.size, 4);
}

#[test]
fn test_vec_push_capacity_1_growth() {
    // capacity=1, size=1 -> new_cap = (int)(1*1.3) = 1; max(1, 1+1)=2
    let mut v = IntVec {
        capacity: 1,
        size: 1,
        data: &[],
    };
    v.vec_push(0);
    assert_eq!(v.capacity, 2);
    assert_eq!(v.size, 2);
}

#[test]
fn test_vec_push_capacity_2_growth() {
    // capacity=2, size=2 -> new_cap = (int)(2*1.3) = 2; max(2, 2+1)=3
    let mut v = IntVec {
        capacity: 2,
        size: 2,
        data: &[],
    };
    v.vec_push(0);
    assert_eq!(v.capacity, 3);
    assert_eq!(v.size, 3);
}

#[test]
fn test_vec_push_capacity_3_growth() {
    // capacity=3, size=3 -> new_cap = (int)(3*1.3) = 3; max(3, 3+1)=4
    let mut v = IntVec {
        capacity: 3,
        size: 3,
        data: &[],
    };
    v.vec_push(0);
    assert_eq!(v.capacity, 4);
    assert_eq!(v.size, 4);
}

#[test]
fn test_vec_push_capacity_100_growth() {
    // capacity=100, size=100 -> new_cap = (int)(100*1.3) = 130
    let mut v = IntVec {
        capacity: 100,
        size: 100,
        data: &[],
    };
    v.vec_push(0);
    assert_eq!(v.capacity, 130);
    assert_eq!(v.size, 101);
}

#[test]
fn test_vec_push_many() {
    // Track the C growth pattern across many pushes.
    let mut v = make_vec(0);
    let mut expected_cap: u64 = 0;
    for i in 1..=20u64 {
        if expected_cap == v.size {
            let new_cap = ((expected_cap as f64) * 1.3) as u64;
            expected_cap = if new_cap < expected_cap + 1 {
                expected_cap + 1
            } else {
                new_cap
            };
        }
        v.vec_push(i as i32);
        assert_eq!(v.size, i);
        assert_eq!(v.capacity, expected_cap);
    }
}

fn main() {}
