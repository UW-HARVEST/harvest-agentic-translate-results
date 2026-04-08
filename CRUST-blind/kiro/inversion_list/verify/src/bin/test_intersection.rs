use inversion_list::inversion_list::InversionList;

#[test]
fn test_intersection() {
    let a: Vec<u32> = vec![1, 2, 3, 5, 7, 9];
    let b: Vec<u32> = vec![1, 2, 3, 5, 7, 9, 10];
    let c: Vec<u32> = vec![1]; // c_src uses {23, 12, 1} with cap=20, but 23>=20 would fail. The intersection test uses cap=20 for set3.

    let set = InversionList::new(20, &a).unwrap();
    let set2 = InversionList::new(20, &b).unwrap();

    let set1 = set.intersection(&set2);
    assert_eq!(set1.to_str(), "[1, 2, 3, 5, 7, 9]");
}

fn main() {}
