use inversion_list::inversion_list::InversionList;

#[test]
fn test_difference_two_sets() {
    // C test-difference: diff(set, set2) -> "[1, 3]"
    // a={1,2,3}, b={2}
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[2]).unwrap();
    let diff = a.difference(&b);
    assert_eq!(diff.to_str(), "[1, 3]");
    assert_eq!(diff.support(), 2);
    assert_eq!(diff.intervals.len(), 2);
    assert_eq!(diff.intervals[0], (1, 2));
    assert_eq!(diff.intervals[1], (3, 4));
}

#[test]
fn test_difference_three_sets() {
    // C test-difference: diff(set, set2, set3) -> "[1]"
    // a={1,2,3}, b={2}, c={3,4}
    // The C semantics are weird: temp = union(b, c) = {2,3,4}; result = diff(a, temp)
    // So in Rust: diff(a, union(b,c)) = {1,2,3} - {2,3,4} = {1}
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[2]).unwrap();
    let c = InversionList::new(20, &[3, 4]).unwrap();
    let bc = b.union(&c);
    let diff = a.difference(&bc);
    assert_eq!(diff.to_str(), "[1]");
}

#[test]
fn test_difference_disjoint() {
    // {1,2,3} - {5,6,7} = {1,2,3}
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[5, 6, 7]).unwrap();
    let d = a.difference(&b);
    assert_eq!(d.to_str(), "[1, 2, 3]");
    assert_eq!(d.support(), 3);
}

#[test]
fn test_symmetric_difference_basic() {
    // C: sym_diff(a={1,2,3}, c={3,4}) = "[1, 2, 4]"
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let c = InversionList::new(20, &[3, 4]).unwrap();
    let sd = a.symmetric_difference(&c);
    assert_eq!(sd.to_str(), "[1, 2, 4]");
    assert_eq!(sd.support(), 3);
}

#[test]
fn test_symmetric_difference_a_minus_b_only() {
    // C: sym_diff({1,2,3}, {2}) = "[1, 3]"
    // u = {1,2,3}, i = {2}, sd = u - i = {1, 3}
    let a = InversionList::new(20, &[1, 2, 3]).unwrap();
    let b = InversionList::new(20, &[2]).unwrap();
    let sd = a.symmetric_difference(&b);
    assert_eq!(sd.to_str(), "[1, 3]");
}

fn main() {}
