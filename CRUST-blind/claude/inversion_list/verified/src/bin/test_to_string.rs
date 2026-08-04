use inversion_list::inversion_list::InversionList;

#[test]
fn test_to_string_basic() {
    // C test-to-string: a={1,2,3,5,7,8,9,2}, capacity=20
    // result: "[1, 2, 3, 5, 7, 8, 9]"
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    assert_eq!(set.to_str(), "[1, 2, 3, 5, 7, 8, 9]");
}

#[test]
fn test_to_string_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    assert_eq!(set.to_str(), "[]");
}

#[test]
fn test_to_string_single() {
    let set = InversionList::new(20, &[5]).unwrap();
    assert_eq!(set.to_str(), "[5]");
}

#[test]
fn test_to_string_with_zero() {
    let set = InversionList::new(20, &[0, 5]).unwrap();
    assert_eq!(set.to_str(), "[0, 5]");
}

#[test]
fn test_display_trait() {
    let set = InversionList::new(20, &[1, 2, 3]).unwrap();
    assert_eq!(format!("{}", set), "[1, 2, 3]");
}

fn main() {}
