use inversion_list::inversion_list::{InversionList, InversionListIterator};

#[test]
fn test_iterator_basic() {
    // C test-iterator: a={1,2,4,10}, capacity=20
    // iterator yields: 1, 2, 4, 10
    let set = InversionList::new(20, &[1, 2, 4, 10]).unwrap();
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values, vec![1, 2, 4, 10]);
}

#[test]
fn test_iterator_with_zero() {
    // C: set with {0, 5} -> iterator yields 0, 5
    let set = InversionList::new(10, &[0, 5]).unwrap();
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values, vec![0, 5]);
}

#[test]
fn test_iterator_consecutive() {
    let set = InversionList::new(20, &[3, 4, 5, 6]).unwrap();
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values, vec![3, 4, 5, 6]);
}

#[test]
fn test_iterator_member_relationship() {
    // From C test: every value yielded by iterator is a member.
    let set = InversionList::new(20, &[1, 2, 4, 10]).unwrap();
    let it = InversionListIterator::new(&set);
    for v in it {
        assert!(set.contains(v), "Iterator yielded {} which is not a member", v);
    }
}

#[test]
fn test_iterator_empty_set() {
    let set = InversionList::new(20, &[]).unwrap();
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values, Vec::<u32>::new());
}

#[test]
fn test_iterator_single_value() {
    let set = InversionList::new(20, &[7]).unwrap();
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values, vec![7]);
}

#[test]
fn test_iterator_all_members_yielded() {
    // The iterator yields exactly support() values.
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 9]).unwrap();
    assert_eq!(set.support(), 6);
    let it = InversionListIterator::new(&set);
    let values: Vec<u32> = it.collect();
    assert_eq!(values.len(), 6);
    assert_eq!(values, vec![1, 2, 3, 5, 7, 9]);
}

fn main() {}
