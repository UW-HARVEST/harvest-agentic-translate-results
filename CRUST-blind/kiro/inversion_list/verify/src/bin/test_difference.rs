use inversion_list::inversion_list::InversionList;

#[test]
fn test_difference() {
    let a: Vec<u32> = vec![1, 2, 3];
    let b: Vec<u32> = vec![2];
    let c: Vec<u32> = vec![3, 4];

    let set = InversionList::new(20, &a).unwrap();
    let set2 = InversionList::new(20, &b).unwrap();
    let set3 = InversionList::new(20, &c).unwrap();

    // difference with union of set2 and set3
    let combined = set2.union(&set3);
    let set1 = set.difference(&combined);
    assert_eq!(set1.to_str(), "[1]");

    // difference with just set2
    let set1 = set.difference(&set2);
    assert_eq!(set1.to_str(), "[1, 3]");

    // symmetric difference
    let set1 = set.symmetric_difference(&set3);
    assert_eq!(set1.to_str(), "[1, 2, 4]");
}

fn main() {}
