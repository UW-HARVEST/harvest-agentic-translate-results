use inversion_list::inversion_list::InversionList;

#[test]
fn test_union() {
    let a = [1, 2, 3, 5, 7, 9];
    let b = [1, 2, 3, 5, 7, 9, 10];
    let c = [23, 12, 1];

    let set = InversionList::new(20, &a).unwrap();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(30, &c).unwrap();

    let set1 = set.union(&set2).union(&set3);
    assert_eq!(set1.to_str(), "[1, 2, 3, 5, 7, 9, 10, 12, 23]");

    let set1 = set.union(&set2);
    assert_eq!(set1.to_str(), "[1, 2, 3, 5, 7, 9, 10]");
}

fn main() {}
