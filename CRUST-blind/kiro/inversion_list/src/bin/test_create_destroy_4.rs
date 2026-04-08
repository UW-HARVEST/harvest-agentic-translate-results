use inversion_list::inversion_list::InversionList;

#[test]
fn test_create_destroy_4() {
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(5, &a);
    assert!(set.is_err());
}

fn main() {}
