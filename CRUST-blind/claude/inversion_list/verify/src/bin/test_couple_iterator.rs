use inversion_list::inversion_list::{InversionList, InversionListCoupleIterator};

#[test]
fn test_couple_iterator_basic() {
    // C test-couple-iterator: a={1,2,3,4,10}, capacity=20
    // set has couples=[1,5,10,11], so couples_iter yields (1,5), (10,11)
    let set = InversionList::new(20, &[1, 2, 3, 4, 10]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let couples: Vec<(u32, u32)> = it.collect();
    assert_eq!(couples, vec![(1, 5), (10, 11)]);
}

#[test]
fn test_couple_iterator_single() {
    let set = InversionList::new(20, &[5]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let couples: Vec<(u32, u32)> = it.collect();
    assert_eq!(couples, vec![(5, 6)]);
}

#[test]
fn test_couple_iterator_multiple() {
    // {1,2,3,5,7,8,9,2} -> couples [1,4,5,6,7,10] -> (1,4), (5,6), (7,10)
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let couples: Vec<(u32, u32)> = it.collect();
    assert_eq!(couples, vec![(1, 4), (5, 6), (7, 10)]);
}

#[test]
fn test_couple_iterator_empty() {
    let set = InversionList::new(20, &[]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let couples: Vec<(u32, u32)> = it.collect();
    assert_eq!(couples, Vec::<(u32, u32)>::new());
}

#[test]
fn test_couple_iterator_matches_intervals() {
    let set = InversionList::new(20, &[1, 2, 5, 6, 10]).unwrap();
    let it = InversionListCoupleIterator::new(&set);
    let couples: Vec<(u32, u32)> = it.collect();
    // Should match intervals exactly
    assert_eq!(couples.len(), set.intervals.len());
    for (i, c) in couples.iter().enumerate() {
        assert_eq!(*c, set.intervals[i]);
    }
}

fn main() {}
