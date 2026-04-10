use inversion_list::inversion_list::{InversionList, InversionListError};

#[test]
fn test_create_destroy_4() {
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let result = InversionList::new(5, &a);
    assert!(result.is_err());
}

fn main() {}
