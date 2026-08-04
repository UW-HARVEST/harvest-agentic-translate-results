use inversion_list::inversion_list::InversionList;

#[test]
fn test_getters() {
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &a).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
}

fn main() {}
