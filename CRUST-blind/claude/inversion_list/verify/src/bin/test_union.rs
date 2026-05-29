use inversion_list::inversion_list::InversionList;

#[test]
fn test_union_two_sets() {
    // C test-union: union(set, set2) -> "[1, 2, 3, 5, 7, 9, 10]"
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    let set2 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let result = set.union(&set2);
    assert_eq!(result.to_str(), "[1, 2, 3, 5, 7, 9, 10]");
    assert_eq!(result.support(), 7);
    assert_eq!(result.capacity(), 20);
}

#[test]
fn test_union_three_sets_chained() {
    // C test-union: union(set, set2, set3) -> "[1, 2, 3, 5, 7, 9, 10, 12, 23]"
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    let set2 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let set3 = InversionList::new(30, &[23, 12, 1]).unwrap();
    let result = set.union(&set2).union(&set3);
    assert_eq!(result.to_str(), "[1, 2, 3, 5, 7, 9, 10, 12, 23]");
    assert_eq!(result.support(), 9);
    // capacity should be max
    assert_eq!(result.capacity(), 30);
}

#[test]
fn test_union_disjoint() {
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[5, 6, 7]).unwrap();
    let u = a.union(&b);
    assert_eq!(u.to_str(), "[1, 2, 3, 5, 6, 7]");
    assert_eq!(u.support(), 6);
    assert_eq!(u.intervals.len(), 2);
    assert_eq!(u.intervals[0], (1, 4));
    assert_eq!(u.intervals[1], (5, 8));
}

#[test]
fn test_union_capacity_max() {
    let a = InversionList::new(20, &[1]).unwrap();
    let b = InversionList::new(50, &[2]).unwrap();
    let u = a.union(&b);
    assert_eq!(u.capacity(), 50);
}

fn main() {}
