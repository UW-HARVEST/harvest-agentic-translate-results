use inversion_list::inversion_list::InversionList;

#[test]
fn test_member() {
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

fn main() {}
