use inversion_list::inversion_list::InversionList;

#[test]
fn test_to_string() {
    let a = [1, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &a).unwrap();
    assert_eq!(set.to_str(), "[1, 2, 3, 5, 7, 8, 9]");
}

fn main() {}
