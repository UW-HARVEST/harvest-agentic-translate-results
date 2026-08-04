use inversion_list::inversion_list::InversionList;

#[test]
fn test_intersection() {
    let a = [1, 2, 3, 5, 7, 9];
    let b = [1, 2, 3, 5, 7, 9, 10];

    let set = InversionList::new(20, &a).unwrap();
    let set2 = InversionList::new(20, &b).unwrap();

    // In C test, set3 = create(20, {23,12,1}) returns NULL since 23>=20
    // So intersection(set, set2, set3=NULL, null) stops at set3, giving set ∩ set2
    let set1 = set.intersection(&set2);
    assert_eq!(set1.to_str(), "[1, 2, 3, 5, 7, 9]");
}

fn main() {}
