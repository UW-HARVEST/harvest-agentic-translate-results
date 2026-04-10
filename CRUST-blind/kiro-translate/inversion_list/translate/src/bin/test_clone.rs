use inversion_list::inversion_list::InversionList;

#[test]
fn test_clone() {
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &a).unwrap();
    let clone = set.clone_list();
    assert_eq!(clone.capacity(), 20);
    assert_eq!(clone.support(), 8);
    assert_eq!(clone.intervals.len(), 3);
    assert_eq!(clone.intervals[0], (0, 4));
    assert_eq!(clone.intervals[1], (5, 6));
    assert_eq!(clone.intervals[2], (7, 10));
}

fn main() {}
