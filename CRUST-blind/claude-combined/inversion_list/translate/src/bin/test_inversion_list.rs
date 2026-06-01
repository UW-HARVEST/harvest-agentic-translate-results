#![allow(unused_imports)]
use inversion_list::inversion_list::{
    InversionList, InversionListCoupleIterator, InversionListIterator,
};

// ----- new() and basic struct fields -----
// Mirrors c_src/tests/inversion-list/test-create-destroy-2.c
#[test]
fn test_new_basic() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0];
    let set = InversionList::new(20, &values).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    // size in C is 6 (3 couples flattened), each element matches couples[i]
    assert_eq!(set.intervals[0], (0u32, 4u32));
    assert_eq!(set.intervals[1], (5u32, 6u32));
    assert_eq!(set.intervals[2], (7u32, 10u32));
}

// Mirrors test-create-destroy-3.c (duplicates are deduped)
#[test]
fn test_new_with_duplicates() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &values).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0u32, 4u32));
    assert_eq!(set.intervals[1], (5u32, 6u32));
    assert_eq!(set.intervals[2], (7u32, 10u32));
}

// Mirrors test-create-destroy-4.c (out-of-range yields error)
#[test]
fn test_new_out_of_range_returns_err() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0, 2];
    let r = InversionList::new(5, &values);
    assert!(r.is_err());
    match r {
        Err(inversion_list::inversion_list::InversionListError::ValueOutOfRange(v, c)) => {
            assert_eq!(v, 9);
            assert_eq!(c, 5);
        }
        _ => panic!("expected ValueOutOfRange"),
    }
}

// Empty input → empty set
#[test]
fn test_new_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 0);
    assert_eq!(set.intervals.len(), 0);
}

// Single element set
#[test]
fn test_new_single() {
    let set = InversionList::new(20, &[5]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 1);
    assert_eq!(set.intervals.len(), 1);
    assert_eq!(set.intervals[0], (5u32, 6u32));
}

// Full set [0..capacity)
#[test]
fn test_new_full() {
    let set = InversionList::new(5, &[0, 1, 2, 3, 4]).unwrap();
    assert_eq!(set.capacity(), 5);
    assert_eq!(set.support(), 5);
    assert_eq!(set.intervals.len(), 1);
    assert_eq!(set.intervals[0], (0u32, 5u32));
}

// ----- capacity() and support() -----
// Mirrors test-getters.c
#[test]
fn test_capacity_and_support() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &values).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
}

// ----- contains() -----
// Mirrors test-member.c. Set is {0,1,2,3,5,7,8,9}
#[test]
fn test_contains() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &values).unwrap();
    assert!(!set.contains(0));
    assert!(!set.contains(4));
    assert!(!set.contains(10));
    assert!(!set.contains(11));
    for v in [1u32, 2, 3, 5, 7, 8, 9, 2] {
        assert!(set.contains(v));
    }
    // Spot-check whole range
    let mem: Vec<bool> = (0..20).map(|v| set.contains(v)).collect();
    let expected: Vec<bool> = (0..20)
        .map(|v| matches!(v, 1 | 2 | 3 | 5 | 7 | 8 | 9))
        .collect();
    assert_eq!(mem, expected);
}

// ----- clone_list() -----
// Mirrors test-clone.c
#[test]
fn test_clone_list() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &values).unwrap();
    let clone = set.clone_list();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 8);
    assert_eq!(clone.intervals.len(), 3);
    assert_eq!(clone.intervals[0], (0u32, 4u32));
    assert_eq!(clone.intervals[1], (5u32, 6u32));
    assert_eq!(clone.intervals[2], (7u32, 10u32));
}

// ----- complement() -----
// Mirrors test-complement.c, all four cases.
#[test]
fn test_complement_starts_at_zero_open_top() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &values).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4u32, 5u32));
    assert_eq!(c.intervals[1], (6u32, 7u32));
    assert_eq!(c.intervals[2], (10u32, 20u32));
}

#[test]
fn test_complement_open_bottom_open_top() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &values).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 13);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0u32, 1u32));
    assert_eq!(c.intervals[1], (4u32, 5u32));
    assert_eq!(c.intervals[2], (6u32, 7u32));
    assert_eq!(c.intervals[3], (10u32, 20u32));
}

#[test]
fn test_complement_open_bottom_closed_top() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2, 19];
    let set = InversionList::new(20, &values).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0u32, 1u32));
    assert_eq!(c.intervals[1], (4u32, 5u32));
    assert_eq!(c.intervals[2], (6u32, 7u32));
    assert_eq!(c.intervals[3], (10u32, 19u32));
}

#[test]
fn test_complement_closed_both_sides() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2, 19, 0];
    let set = InversionList::new(20, &values).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 11);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4u32, 5u32));
    assert_eq!(c.intervals[1], (6u32, 7u32));
    assert_eq!(c.intervals[2], (10u32, 19u32));
}

// ----- to_str() / Display -----
// Mirrors test-to-string.c
#[test]
fn test_to_str() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &values).unwrap();
    assert_eq!(set.to_str(), "[1, 2, 3, 5, 7, 8, 9]");
}

#[test]
fn test_to_str_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.to_str(), "[]");
}

#[test]
fn test_display_format() {
    let values = [1u32, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &values).unwrap();
    assert_eq!(format!("{}", set), "[1, 2, 3, 5, 7, 8, 9]");
}

// ----- equal() / PartialEq -----
// Mirrors test-equals.c
#[test]
fn test_equal_basic() {
    let a = [1u32, 2, 3, 5, 7, 8, 9];
    let b = [1u32, 2, 3, 9, 8];
    let c = [1u32, 2, 3, 5, 7, 9, 10];
    let set = InversionList::new(20, &a).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(20, &c).unwrap();

    assert!(set.equal(&set1));
    assert!(set.equal(&set));
    assert!(!set.equal(&set2));
    assert!(!set.equal(&set3));
    assert!(!set2.equal(&set3));

    // PartialEq mirror
    assert_eq!(set, set1);
    assert_ne!(set, set2);
    assert_ne!(set, set3);
}

// ----- is_strict_subset_of (== inversion_list_less) -----
// Mirrors test-equals.c "test less" assertions
#[test]
fn test_is_strict_subset_of() {
    let a = [1u32, 2, 3, 5, 7, 8, 9];
    let b = [1u32, 2, 3, 9, 8];
    let c = [1u32, 2, 3, 5, 7, 9, 10];
    let set = InversionList::new(20, &a).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(20, &c).unwrap();

    assert!(!set.is_strict_subset_of(&set1));
    assert!(!set.is_strict_subset_of(&set));
    assert!(!set.is_strict_subset_of(&set2));
    assert!(!set.is_strict_subset_of(&set3));
    assert!(set2.is_strict_subset_of(&set3));
}

// ----- is_subset_of (== inversion_list_less_equal) -----
// Mirrors test-equals.c "test less-equal" assertions
#[test]
fn test_is_subset_of() {
    let a = [1u32, 2, 3, 5, 7, 8, 9];
    let b = [1u32, 2, 3, 9, 8];
    let c = [1u32, 2, 3, 5, 7, 9, 10];
    let set = InversionList::new(20, &a).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(20, &c).unwrap();

    assert!(set.is_subset_of(&set1));
    assert!(set.is_subset_of(&set));
    assert!(!set.is_subset_of(&set2));
    assert!(!set.is_subset_of(&set3));
    assert!(set2.is_subset_of(&set3));
}

// ----- is_disjoint (== inversion_list_disjoint, which in C is !equal) -----
#[test]
fn test_is_disjoint() {
    let a = [1u32, 2, 3, 5, 7, 8, 9];
    let b = [1u32, 2, 3, 9, 8];
    let set = InversionList::new(20, &a).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &b).unwrap();

    // C semantics: disjoint == !equal
    assert!(!set.is_disjoint(&set1));
    assert!(!set.is_disjoint(&set));
    assert!(set.is_disjoint(&set2));
}

// ----- union -----
// Mirrors test-union.c
#[test]
fn test_union_pair() {
    let a = [1u32, 2, 3, 5, 7, 9];
    let b = [1u32, 2, 3, 5, 7, 9, 10];
    let s1 = InversionList::new(20, &a).unwrap();
    let s2 = InversionList::new(20, &b).unwrap();
    let u = s1.union(&s2);
    assert_eq!(u.to_str(), "[1, 2, 3, 5, 7, 9, 10]");
    // capacity is max of two
    assert_eq!(u.capacity(), 20);
    assert_eq!(u.support(), 7);
}

#[test]
fn test_union_three_chained() {
    let a = [1u32, 2, 3, 5, 7, 9];
    let b = [1u32, 2, 3, 5, 7, 9, 10];
    let c = [23u32, 12, 1];
    let s1 = InversionList::new(20, &a).unwrap();
    let s2 = InversionList::new(20, &b).unwrap();
    let s3 = InversionList::new(30, &c).unwrap();
    let u = s1.union(&s2).union(&s3);
    assert_eq!(u.to_str(), "[1, 2, 3, 5, 7, 9, 10, 12, 23]");
    assert_eq!(u.capacity(), 30);
    assert_eq!(u.support(), 9);
}

#[test]
fn test_union_disjoint_singles() {
    let s1 = InversionList::new(20, &[1, 5]).unwrap();
    let s2 = InversionList::new(20, &[2, 7]).unwrap();
    let u = s1.union(&s2);
    assert_eq!(u.to_str(), "[1, 2, 5, 7]");
    assert_eq!(u.support(), 4);
    // 1 and 2 merge into one couple, then {5} and {7} are separate.
    assert_eq!(u.intervals.len(), 3);
    assert_eq!(u.intervals[0], (1u32, 3u32));
    assert_eq!(u.intervals[1], (5u32, 6u32));
    assert_eq!(u.intervals[2], (7u32, 8u32));
}

// ----- intersection -----
// Mirrors test-intersection.c
#[test]
fn test_intersection_pair() {
    let a = [1u32, 2, 3, 5, 7, 9];
    let b = [1u32, 2, 3, 5, 7, 9, 10];
    let s1 = InversionList::new(20, &a).unwrap();
    let s2 = InversionList::new(20, &b).unwrap();
    let i = s1.intersection(&s2);
    assert_eq!(i.to_str(), "[1, 2, 3, 5, 7, 9]");
    assert_eq!(i.support(), 6);
    assert_eq!(i.capacity(), 20);
}

#[test]
fn test_intersection_three_chained() {
    // (Mirrors logic of test-intersection.c: chaining with a third
    // set that contains only one common element.)
    let a = [1u32, 2, 3, 5, 7, 9];
    let b = [1u32, 2, 3, 5, 7, 9, 10];
    let c = [23u32, 12, 1];
    let s1 = InversionList::new(20, &a).unwrap();
    let s2 = InversionList::new(20, &b).unwrap();
    let s3 = InversionList::new(30, &c).unwrap();
    let i = s1.intersection(&s2).intersection(&s3);
    assert_eq!(i.to_str(), "[1]");
    assert_eq!(i.support(), 1);
    assert_eq!(i.intervals.len(), 1);
    assert_eq!(i.intervals[0], (1u32, 2u32));
}

#[test]
fn test_intersection_disjoint() {
    let s1 = InversionList::new(20, &[1, 5]).unwrap();
    let s2 = InversionList::new(20, &[2, 7]).unwrap();
    let i = s1.intersection(&s2);
    assert_eq!(i.to_str(), "[]");
    assert_eq!(i.support(), 0);
    assert_eq!(i.intervals.len(), 0);
}

// ----- difference -----
// Mirrors test-difference.c
#[test]
fn test_difference_pair() {
    let s = InversionList::new(20, &[1, 2, 3]).unwrap();
    let s2 = InversionList::new(20, &[2]).unwrap();
    let d = s.difference(&s2);
    assert_eq!(d.to_str(), "[1, 3]");
    assert_eq!(d.support(), 2);
}

#[test]
fn test_difference_chained_three() {
    let s = InversionList::new(20, &[1, 2, 3]).unwrap();
    let s2 = InversionList::new(20, &[2]).unwrap();
    let s3 = InversionList::new(20, &[3, 4]).unwrap();
    let d = s.difference(&s2).difference(&s3);
    assert_eq!(d.to_str(), "[1]");
    assert_eq!(d.support(), 1);
}

// ----- symmetric_difference -----
// Mirrors test-difference.c
#[test]
fn test_symmetric_difference_overlap() {
    let s = InversionList::new(20, &[1, 2, 3]).unwrap();
    let s3 = InversionList::new(20, &[3, 4]).unwrap();
    let sd = s.symmetric_difference(&s3);
    assert_eq!(sd.to_str(), "[1, 2, 4]");
    assert_eq!(sd.support(), 3);
}

// ----- InversionListIterator (iterates values) -----
// Mirrors test-iterator.c — every yielded value must be a member of the set.
#[test]
fn test_iterator_yields_all_members() {
    let values = [1u32, 2, 4, 10];
    let set = InversionList::new(20, &values).unwrap();
    let mut it = InversionListIterator::new(&set);
    let collected: Vec<u32> = (&mut it).collect();
    // Set is {1,2,4,10}
    assert_eq!(collected, vec![1u32, 2, 4, 10]);
    for v in &collected {
        assert!(set.contains(*v));
    }
}

#[test]
fn test_iterator_full_set() {
    // {0,5,6,7,19} cap=20
    let set = InversionList::new(20, &[0, 5, 6, 7, 19]).unwrap();
    let it = InversionListIterator::new(&set);
    let collected: Vec<u32> = it.collect();
    assert_eq!(collected, vec![0u32, 5, 6, 7, 19]);
}

#[test]
fn test_iterator_empty_set() {
    let set = InversionList::new(20, &[]).unwrap();
    let it = InversionListIterator::new(&set);
    let collected: Vec<u32> = it.collect();
    assert_eq!(collected, Vec::<u32>::new());
}

// ----- InversionListCoupleIterator (iterates intervals) -----
// Mirrors test-couple-iterator.c
#[test]
fn test_couple_iterator() {
    // {1,2,3,4,10} → couples (1,5) (10,11)
    let set = InversionList::new(20, &[1, 2, 3, 4, 10]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let collected: Vec<(u32, u32)> = it.collect();
    assert_eq!(collected, vec![(1u32, 5u32), (10u32, 11u32)]);
}

#[test]
fn test_couple_iterator_matches_intervals() {
    let set = InversionList::new(20, &[1, 2, 4, 10]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let collected: Vec<(u32, u32)> = it.collect();
    assert_eq!(collected, set.intervals);
}

#[test]
fn test_couple_iterator_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let collected: Vec<(u32, u32)> = it.collect();
    assert_eq!(collected, Vec::<(u32, u32)>::new());
}

fn main() {}
