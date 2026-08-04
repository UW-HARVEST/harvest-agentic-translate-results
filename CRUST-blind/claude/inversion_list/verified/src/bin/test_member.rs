use inversion_list::inversion_list::InversionList;

#[test]
fn test_member_basic() {
    // C test-member: a={1,2,3,5,7,8,9,2}, capacity=20
    // contains: 0->no, 4->no, 10->no, 11->no
    // contains: 1,2,3,5,7,8,9 -> yes
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();

    assert_eq!(set.contains(0), false);
    assert_eq!(set.contains(4), false);
    assert_eq!(set.contains(10), false);
    assert_eq!(set.contains(11), false);

    assert_eq!(set.contains(1), true);
    assert_eq!(set.contains(2), true);
    assert_eq!(set.contains(3), true);
    assert_eq!(set.contains(5), true);
    assert_eq!(set.contains(7), true);
    assert_eq!(set.contains(8), true);
    assert_eq!(set.contains(9), true);
}

#[test]
fn test_member_empty_set() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.contains(0), false);
    assert_eq!(set.contains(5), false);
    assert_eq!(set.contains(19), false);
}

#[test]
fn test_member_at_boundary() {
    let set = InversionList::new(20, &[19]).unwrap();
    assert_eq!(set.contains(19), true);
    assert_eq!(set.contains(18), false);
    assert_eq!(set.contains(0), false);
}

fn main() {}
