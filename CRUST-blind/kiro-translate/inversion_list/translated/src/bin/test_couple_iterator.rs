use inversion_list::inversion_list::{InversionList, InversionListCoupleIterator};

#[test]
fn test_couple_iterator() {
    let a = [1, 2, 3, 4, 10];
    let set = InversionList::new(20, &a).unwrap();
    let iter = InversionListCoupleIterator::new(&set);
    for (i, (inf, sup)) in iter.enumerate() {
        assert_eq!(set.intervals[i].0, inf);
        assert_eq!(set.intervals[i].1, sup);
    }
}

fn main() {}
