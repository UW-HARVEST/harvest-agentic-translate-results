use inversion_list::inversion_list::InversionList;

#[test]
fn test_intersection_two_sets() {
    // C test-intersection: intersect(set, set2) -> "[1, 2, 3, 5, 7, 9]"
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    let set2 = InversionList::new(20, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let result = set.intersection(&set2);
    assert_eq!(result.to_str(), "[1, 2, 3, 5, 7, 9]");
    assert_eq!(result.support(), 6);
    assert_eq!(result.capacity(), 20);
}

#[test]
fn test_intersection_three_sets_chained() {
    // C test-intersection has capacity=30 vs values up to 23 in set3.
    // Run with full valid capacity to get the actual three-way intersection.
    // C output (run): intersect of {1,2,3,5,7,9}, {1,2,3,5,7,9,10}, {23,12,1} = "[1]"
    let set = InversionList::new(30, &[1, 2, 3, 5, 7, 9]).unwrap();
    let set2 = InversionList::new(30, &[1, 2, 3, 5, 7, 9, 10]).unwrap();
    let set3 = InversionList::new(30, &[23, 12, 1]).unwrap();
    let result = set.intersection(&set2).intersection(&set3);
    assert_eq!(result.to_str(), "[1]");
    assert_eq!(result.support(), 1);
    assert_eq!(result.intervals.len(), 1);
    assert_eq!(result.intervals[0], (1, 2));
}

#[test]
fn test_intersection_disjoint() {
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[5, 6, 7]).unwrap();
    let i = a.intersection(&b);
    assert_eq!(i.to_str(), "[]");
    assert_eq!(i.support(), 0);
    assert_eq!(i.intervals.len(), 0);
}

#[test]
fn test_intersection_subset() {
    let big = InversionList::new(20, &[1, 2, 3, 4, 5]).unwrap();
    let small = InversionList::new(20, &[2, 3]).unwrap();
    let i = big.intersection(&small);
    assert_eq!(i.to_str(), "[2, 3]");
    assert_eq!(i.support(), 2);
}

fn main() {}
