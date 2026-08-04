use inversion_list::inversion_list::InversionList;

#[test]
fn test_create_destroy_3() {
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &a).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
    assert_eq!(set.intervals.len(), 3);
    assert_eq!(set.intervals[0], (0, 4));
    assert_eq!(set.intervals[1], (5, 6));
    assert_eq!(set.intervals[2], (7, 10));
}

fn main() {}
