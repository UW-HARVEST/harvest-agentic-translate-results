use inversion_list::inversion_list::{InversionList, InversionListError};

#[test]
fn test_create_basic_with_zero() {
    // C test-create-destroy-2: a={1,2,3,5,7,8,9,0}, capacity=20
    // C result: capacity=20, support=8, size=6, couples=[0,4,5,6,7,10]
    // Rust intervals = [(0,4), (5,6), (7,10)]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0, 4));
    assert_eq!(set.intervals[1], (5, 6));
    assert_eq!(set.intervals[2], (7, 10));
}

#[test]
fn test_create_with_duplicate() {
    // C test-create-destroy-3: a={1,2,3,5,7,8,9,0,2}, capacity=20
    // duplicates collapsed: still support=8, size=6, couples=[0,4,5,6,7,10]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0, 4));
    assert_eq!(set.intervals[1], (5, 6));
    assert_eq!(set.intervals[2], (7, 10));
}

#[test]
fn test_create_value_out_of_range() {
    // C test-create-destroy-4: a={...9...}, capacity=5
    // C result: NULL with errno=EINVAL
    // Rust result: Err(ValueOutOfRange(9, 5))
    let result = InversionList::new(5, &[1, 2, 3, 5, 7, 8, 9, 0, 2]);
    assert!(result.is_err());
    match result {
        Err(InversionListError::ValueOutOfRange(v, c)) => {
            assert_eq!(v, 9);
            assert_eq!(c, 5);
        }
        _ => panic!("Expected ValueOutOfRange"),
    }
}

#[test]
fn test_create_value_equal_capacity_is_invalid() {
    // C: if buffer[count-1] >= capacity, error. So value == capacity is invalid.
    let result = InversionList::new(10, &[5, 9, 10]);
    assert!(result.is_err());
}

#[test]
fn test_create_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 0);
    assert_eq!(set.intervals.len(), 0);
}

#[test]
fn test_create_single_value() {
    let set = InversionList::new(20, &[5]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 1);
    assert_eq!(set.intervals.len(), 1);
    assert_eq!(set.intervals[0], (5, 6));
}

#[test]
fn test_create_at_capacity_minus_one() {
    let set = InversionList::new(20, &[19]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 1);
    assert_eq!(set.intervals.len(), 1);
    assert_eq!(set.intervals[0], (19, 20));
}

fn main() {}
