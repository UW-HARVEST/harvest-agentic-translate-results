use inversion_list::inversion_list::{InversionList, InversionListIterator};

#[test]
fn test_iterator() {
    let a: Vec<u32> = vec![1, 2, 4, 10];
    let set = InversionList::new(20, &a).unwrap();

    let iter = InversionListIterator::new(&set);
    for val in iter {
        assert!(set.contains(val));
    }
}

fn main() {}
