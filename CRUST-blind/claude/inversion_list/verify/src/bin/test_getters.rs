use inversion_list::inversion_list::InversionList;

#[test]
fn test_capacity_and_support() {
    // C test-getters: a={1,2,3,5,7,8,9,0,2}, capacity=20
    // capacity()=20, support()=8
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    assert_eq!(set.capacity(), 20);
    assert_eq!(set.support(), 8);
}

#[test]
fn test_capacity_empty() {
    let set = InversionList::new(100, &[]).unwrap();
    assert_eq!(set.capacity(), 100);
    assert_eq!(set.support(), 0);
}

#[test]
fn test_support_with_duplicates() {
    // duplicates don't affect support
    let set = InversionList::new(20, &[5, 5, 5, 6]).unwrap();
    assert_eq!(set.support(), 2);
}

fn main() {}
