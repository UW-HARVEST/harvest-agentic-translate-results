use inversion_list::inversion_list::InversionList;

#[test]
fn test_clone_basic() {
    // C test-clone: a={1,2,3,5,7,8,9,0,2}, capacity=20
    // clone: capacity=20, support=8, size=6, couples=[0,4,5,6,7,10]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    let clone = set.clone_list();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 8);
    assert_eq!(clone.intervals.len(), 3);
    assert_eq!(clone.intervals[0], (0, 4));
    assert_eq!(clone.intervals[1], (5, 6));
    assert_eq!(clone.intervals[2], (7, 10));
}

#[test]
fn test_clone_independence() {
    // Clone via Clone trait
    let set = InversionList::new(20, &[5, 7]).unwrap();
    let clone = set.clone();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 2);
    assert!(clone.equal(&set));
}

#[test]
fn test_clone_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    let clone = set.clone_list();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 0);
    assert_eq!(clone.intervals.len(), 0);
}

fn main() {}
