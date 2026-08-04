use inversion_list::inversion_list::InversionList;

#[test]
fn test_equal_basic() {
    // C test-equals
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let set3 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    // equal
    assert_eq!(set.equal(&set1), true);
    assert_eq!(set.equal(&set), true);
    assert_eq!(set.equal(&set2), false);
    assert_eq!(set.equal(&set3), false);
    assert_eq!(set2.equal(&set3), false);
}

#[test]
fn test_partial_eq_trait() {
    let set = InversionList::new(20, &[1, 2, 3]).unwrap();
    let set_clone = set.clone_list();
    let other = InversionList::new(20, &[5]).unwrap();
    assert!(set == set_clone);
    assert!(set != other);
}

#[test]
fn test_less_basic() {
    // From C test-equals
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let set3 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    assert_eq!(set.is_strict_subset_of(&set1), false);
    assert_eq!(set.is_strict_subset_of(&set), false);
    assert_eq!(set.is_strict_subset_of(&set2), false);
    assert_eq!(set.is_strict_subset_of(&set3), false);
    assert_eq!(set2.is_strict_subset_of(&set3), true);
}

#[test]
fn test_less_equal_basic() {
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9]).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &[1, 2, 3, 9, 8]).unwrap();
    let set3 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();

    assert_eq!(set.is_subset_of(&set1), true);
    assert_eq!(set.is_subset_of(&set), true);
    assert_eq!(set.is_subset_of(&set2), false);
    assert_eq!(set.is_subset_of(&set3), false);
    assert_eq!(set2.is_subset_of(&set3), true);
}

#[test]
fn test_disjoint_semantics() {
    // C: disjoint(set1,set2) returns !equal(set1,set2)
    // So equal sets are not disjoint, non-equal sets are disjoint.
    let set = InversionList::new(20, &[1, 2, 3]).unwrap();
    let set_clone = set.clone_list();
    let other = InversionList::new(20, &[5]).unwrap();
    assert_eq!(set.is_disjoint(&set_clone), false);
    assert_eq!(set.is_disjoint(&other), true);
}

#[test]
fn test_less_same_support_no_subset() {
    // {1,2,3} and {5,6,7}: same support, less returns 0
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[5, 6, 7]).unwrap();
    assert_eq!(a.is_strict_subset_of(&b), false);
    assert_eq!(b.is_strict_subset_of(&a), false);
}

#[test]
fn test_less_smaller_with_member_in_range() {
    // {3} (support=1, max=4 in couples) vs {0,5} (support=2)
    // C: less returns 1 because member(B, 0) is true
    let a = InversionList::new(20, &[3]).unwrap();
    let b = InversionList::new(20, &[0, 5]).unwrap();
    assert_eq!(a.is_strict_subset_of(&b), true);
}

#[test]
fn test_less_smaller_no_member_in_range() {
    // {3} (support=1, max=4) vs {5,7} (support=2)
    // C: less returns 0 because member(B, 0..3) is all false
    let a = InversionList::new(20, &[3]).unwrap();
    let b = InversionList::new(20, &[5, 7]).unwrap();
    assert_eq!(a.is_strict_subset_of(&b), false);
}

fn main() {}
