use inversion_list::inversion_list::{
    InversionList, InversionListCoupleIterator, InversionListError, InversionListIterator,
};

// === Creation and basic structure ===

#[test]
fn test_new_basic() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0, 4));
    assert_eq!(set.intervals[1], (5, 6));
    assert_eq!(set.intervals[2], (7, 10));
}

#[test]
fn test_new_with_duplicates() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0, 4));
    assert_eq!(set.intervals[1], (5, 6));
    assert_eq!(set.intervals[2], (7, 10));
}

#[test]
fn test_new_value_out_of_range() {
    let result = InversionList::new(5, &[1, 2, 3, 5, 7, 8, 9, 0, 2]);
    assert!(result.is_err());
}

#[test]
fn test_new_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 0);
    assert_eq!(set.intervals.len(), 0);
}

#[test]
fn test_new_single_element() {
    let set = InversionList::new(20, &[5]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 1);
    assert_eq!(set.intervals.len(), 1);
    assert_eq!(set.intervals[0], (5, 6));
}

// === Getters ===

#[test]
fn test_capacity_and_support() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
}

// === Membership ===

#[test]
fn test_contains() {
    let a = [1, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &a).unwrap();
    assert!(!set.contains(0));
    assert!(!set.contains(4));
    assert!(!set.contains(10));
    assert!(!set.contains(11));
    for &v in &a {
        assert!(set.contains(v));
    }
}

#[test]
fn test_contains_boundary() {
    let set = InversionList::new(20, &[0, 19]).unwrap();
    assert!(set.contains(0));
    assert!(set.contains(19));
    assert!(!set.contains(1));
    assert!(!set.contains(18));
}

// === Clone ===

#[test]
fn test_clone_list() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    let clone = set.clone_list();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 8);
    assert_eq!(clone.intervals.len(), 3);
    assert_eq!(clone.intervals[0], (0, 4));
    assert_eq!(clone.intervals[1], (5, 6));
    assert_eq!(clone.intervals[2], (7, 10));
}

// === Complement ===

#[test]
fn test_complement_starts_at_zero() {
    // {0,1,2,3,5,7,8,9} cap=20 -> complement intervals: (4,5),(6,7),(10,20)
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4, 5));
    assert_eq!(c.intervals[1], (6, 7));
    assert_eq!(c.intervals[2], (10, 20));
}

#[test]
fn test_complement_no_zero() {
    // {1,2,3,5,7,8,9} cap=20 -> complement: (0,1),(4,5),(6,7),(10,20)
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 13);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0, 1));
    assert_eq!(c.intervals[1], (4, 5));
    assert_eq!(c.intervals[2], (6, 7));
    assert_eq!(c.intervals[3], (10, 20));
}

#[test]
fn test_complement_ends_at_capacity() {
    // {1,2,3,5,7,8,9,19} cap=20 -> complement: (0,1),(4,5),(6,7),(10,19)
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2, 19]).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0, 1));
    assert_eq!(c.intervals[1], (4, 5));
    assert_eq!(c.intervals[2], (6, 7));
    assert_eq!(c.intervals[3], (10, 19));
}

#[test]
fn test_complement_starts_zero_ends_capacity() {
    // {0,1,2,3,5,7,8,9,19} cap=20 -> complement: (4,5),(6,7),(10,19)
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2, 19, 0]).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 11);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4, 5));
    assert_eq!(c.intervals[1], (6, 7));
    assert_eq!(c.intervals[2], (10, 19));
}

// === to_string / Display ===

#[test]
fn test_to_str() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    assert_eq!(set.to_str(), "[1, 2, 3, 5, 7, 8, 9]");
}

#[test]
fn test_to_str_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.to_str(), "[]");
}

#[test]
fn test_display_matches_to_str() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    assert_eq!(format!("{}", set), set.to_str());
}

// === Equal / Not-Equal ===

#[test]
fn test_equal() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let set3 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    assert!(set.equal(&set1));
    assert!(set.equal(&set));
    assert!(!set.equal(&set2));
    assert!(!set.equal(&set3));
    assert!(!set2.equal(&set3));
}

#[test]
fn test_equal_ignores_capacity() {
    // C: equal(g_cap20, h_cap30) = 1
    let g = InversionList::new(20, &[1, 2, 3]).unwrap();
    let h = InversionList::new(30, &[1, 2, 3]).unwrap();
    assert!(g.equal(&h));
}

#[test]
fn test_not_equal_via_ne() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();

    assert!(!(set != set1));
    assert!(set != set2);
}

// === Less / Less-Equal / Greater / Greater-Equal (C semantics) ===

#[test]
fn test_is_strict_subset_of() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let a_clone = a.clone_list();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    // C ground truth
    assert!(!a.is_strict_subset_of(&a_clone)); // less(a, a_clone) = 0
    assert!(!a.is_strict_subset_of(&a));       // less(a, a) = 0
    assert!(!a.is_strict_subset_of(&b));       // less(a, b) = 0
    assert!(!a.is_strict_subset_of(&c));       // less(a, c) = 0
    assert!(b.is_strict_subset_of(&c));        // less(b, c) = 1
}

#[test]
fn test_is_strict_subset_disjoint_sets() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let d = InversionList::new(20, &[10, 11, 12]).unwrap();
    let f = InversionList::new(20, &[1]).unwrap();

    // C ground truth: less(d, a) = 1 (d.support < a.support and a contains value in 0..13)
    assert!(d.is_strict_subset_of(&a));
    // C ground truth: less(f, a) = 1
    assert!(f.is_strict_subset_of(&a));
    // C ground truth: less(f, d) = 0 (d doesn't contain any value in 0..2)
    assert!(!f.is_strict_subset_of(&d));
}

#[test]
fn test_is_subset_of() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let a_clone = a.clone_list();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    assert!(a.is_subset_of(&a_clone));  // less_equal(a, a_clone) = 1
    assert!(a.is_subset_of(&a));        // less_equal(a, a) = 1
    assert!(!a.is_subset_of(&b));       // less_equal(a, b) = 0
    assert!(!a.is_subset_of(&c));       // less_equal(a, c) = 0
    assert!(b.is_subset_of(&c));        // less_equal(b, c) = 1
}

// === Disjoint (C semantics: disjoint = not_equal) ===

#[test]
fn test_is_disjoint() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let a_clone = a.clone_list();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let d = InversionList::new(20, &[10, 11, 12]).unwrap();

    assert!(!a.is_disjoint(&a_clone)); // disjoint(a, a_clone) = 0
    assert!(a.is_disjoint(&b));        // disjoint(a, b) = 1
    assert!(a.is_disjoint(&c));        // disjoint(a, c) = 1
    assert!(a.is_disjoint(&d));        // disjoint(a, d) = 1
    assert!(b.is_disjoint(&c));        // disjoint(b, c) = 1
}

// === Union ===

#[test]
fn test_union() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let c = InversionList::new(30, &[23, 12, 1]).unwrap();

    let u = a.union(&b).union(&c);
    assert_eq!(u.to_str(), "[1, 2, 3, 5, 7, 9, 10, 12, 23]");

    let u2 = a.union(&b);
    assert_eq!(u2.to_str(), "[1, 2, 3, 5, 7, 9, 10]");
    assert_eq!(u2.support(), 7);
}

#[test]
fn test_union_self() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let u = a.union(&a);
    assert!(u.equal(&a));
    assert_eq!(u.support(), 7);
}

// === Intersection ===

#[test]
fn test_intersection() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();

    let i = a.intersection(&b);
    assert_eq!(i.to_str(), "[1, 2, 3, 8, 9]");
    assert_eq!(i.support(), 5);
}

#[test]
fn test_intersection_subset() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    let i = a.intersection(&b);
    assert_eq!(i.to_str(), "[1, 2, 3, 5, 7, 9]");
    assert_eq!(i.support(), 6);
}

#[test]
fn test_intersection_with_partial_overlap() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    let i = a.intersection(&c);
    assert_eq!(i.to_str(), "[1, 2, 3, 5, 7, 9]");
}

// === Difference ===

#[test]
fn test_difference() {
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[2]).unwrap();
    let c = InversionList::new(20, &[3, 4]).unwrap();

    let combined = b.union(&c);
    let d = a.difference(&combined);
    assert_eq!(d.to_str(), "[1]");

    let d2 = a.difference(&b);
    assert_eq!(d2.to_str(), "[1, 3]");
}

#[test]
fn test_difference_larger_sets() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    let d = a.difference(&b);
    assert_eq!(d.to_str(), "[5, 7]");

    let d2 = a.difference(&c);
    assert_eq!(d2.to_str(), "[8]");
}

// === Symmetric Difference ===

#[test]
fn test_symmetric_difference() {
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let c = InversionList::new(20, &[3, 4]).unwrap();

    let sd = a.symmetric_difference(&c);
    assert_eq!(sd.to_str(), "[1, 2, 4]");
}

#[test]
fn test_symmetric_difference_larger() {
    let a = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let c = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    let sd = a.symmetric_difference(&b);
    assert_eq!(sd.to_str(), "[5, 7]");

    let sd2 = a.symmetric_difference(&c);
    assert_eq!(sd2.to_str(), "[8, 10]");
}

// === Iterator ===

#[test]
fn test_iterator_values() {
    let set = InversionList::new(20, &[1, 2, 4, 10]).unwrap();
    let vals: Vec<u32> = InversionListIterator::new(&set).collect();
    assert_eq!(vals, vec![1, 2, 4, 10]);
}

#[test]
fn test_iterator_all_members() {
    let set = InversionList::new(20, &[1, 2, 4, 10]).unwrap();
    for val in InversionListIterator::new(&set) {
        assert!(set.contains(val));
    }
}

#[test]
fn test_iterator_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    let vals: Vec<u32> = InversionListIterator::new(&set).collect();
    assert!(vals.is_empty());
}

// === Couple Iterator ===

#[test]
fn test_couple_iterator_values() {
    let set = InversionList::new(20, &[1, 2, 3, 4, 10]).unwrap();
    let pairs: Vec<(u32, u32)> = InversionListCoupleIterator::new(&set).collect();
    assert_eq!(pairs, vec![(1, 5), (10, 11)]);
}

#[test]
fn test_couple_iterator_matches_intervals() {
    let set = InversionList::new(20, &[1, 2, 3, 4, 10]).unwrap();
    for (i, (inf, sup)) in InversionListCoupleIterator::new(&set).enumerate() {
        assert_eq!(set.intervals[i].0, inf);
        assert_eq!(set.intervals[i].1, sup);
    }
}

#[test]
fn test_couple_iterator_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    let pairs: Vec<(u32, u32)> = InversionListCoupleIterator::new(&set).collect();
    assert!(pairs.is_empty());
}

// === PartialEq / Eq ===

#[test]
fn test_partial_eq() {
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[1, 2, 3]).unwrap();
    let c = InversionList::new(20, &[1, 2, 4]).unwrap();
    assert_eq!(a, b);
    assert_ne!(a, c);
}

fn main() {}
