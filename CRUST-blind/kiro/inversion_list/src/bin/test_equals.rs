use inversion_list::inversion_list::InversionList;

#[test]
fn test_equals() {
    let a: Vec<u32> = vec![1, 2, 3, 5, 7, 8, 9];
    let b: Vec<u32> = vec![1, 2, 3, 9, 8];
    let c: Vec<u32> = vec![1, 2, 3, 5, 7, 9, 10];

    let set = InversionList::new(20, &a).unwrap();
    let set1 = set.clone_list();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(20, &c).unwrap();

    // test equal
    assert!(set.equal(&set1));
    assert!(set.equal(&set));
    assert!(!set.equal(&set2));
    assert!(!set.equal(&set3));
    assert!(!set2.equal(&set3));

    // test not equal (via PartialEq)
    assert!(set == set1);
    assert!(set == set);
    assert!(set != set2);
    assert!(set != set3);
    assert!(set2 != set3);

    // test less (strict subset)
    assert!(!set.is_strict_subset_of(&set1));
    assert!(!set.is_strict_subset_of(&set));
    assert!(!set.is_strict_subset_of(&set2));
    assert!(!set.is_strict_subset_of(&set3));
    assert!(set2.is_strict_subset_of(&set3));

    // test less-equal (subset)
    assert!(set.is_subset_of(&set1));
    assert!(set.is_subset_of(&set));
    assert!(!set.is_subset_of(&set2));
    assert!(!set.is_subset_of(&set3));
    assert!(set2.is_subset_of(&set3));

    // test greater (strict superset) = other.is_strict_subset_of(self)
    assert!(!set1.is_strict_subset_of(&set));
    assert!(!set.is_strict_subset_of(&set));
    assert!(set2.is_strict_subset_of(&set));
    assert!(!set.is_strict_subset_of(&set3));
    assert!(!set3.is_strict_subset_of(&set2));

    // test greater-equal (superset) = other.is_subset_of(self)
    assert!(set1.is_subset_of(&set));
    assert!(set.is_subset_of(&set));
    assert!(set2.is_subset_of(&set));
    assert!(!set.is_subset_of(&set3));
    assert!(!set3.is_subset_of(&set2));
}

fn main() {}
