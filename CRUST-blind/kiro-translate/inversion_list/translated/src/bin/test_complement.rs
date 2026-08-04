use inversion_list::inversion_list::InversionList;

#[test]
fn test_complement_1() {
    // {0,1,2,3,5,7,8,9} cap=20 -> complement: {4,6,10..19}
    let a = [1, 2, 3, 5, 7, 8, 9, 0, 2];
    let set = InversionList::new(20, &a).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4, 5));
    assert_eq!(c.intervals[1], (6, 7));
    assert_eq!(c.intervals[2], (10, 20));
}

#[test]
fn test_complement_2() {
    // {1,2,3,5,7,8,9} cap=20 -> complement: {0,4,6,10..19}
    let a = [1, 2, 3, 5, 7, 8, 9, 2];
    let set = InversionList::new(20, &a).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 13);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0, 1));
    assert_eq!(c.intervals[1], (4, 5));
    assert_eq!(c.intervals[2], (6, 7));
    assert_eq!(c.intervals[3], (10, 20));
}

#[test]
fn test_complement_3() {
    // {1,2,3,5,7,8,9,19} cap=20 -> complement: {0,4,6,10..18}
    let a = [1, 2, 3, 5, 7, 8, 9, 2, 19];
    let set = InversionList::new(20, &a).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 12);
    assert_eq!(c.intervals.len(), 4);
    assert_eq!(c.intervals[0], (0, 1));
    assert_eq!(c.intervals[1], (4, 5));
    assert_eq!(c.intervals[2], (6, 7));
    assert_eq!(c.intervals[3], (10, 19));
}

#[test]
fn test_complement_4() {
    // {0,1,2,3,5,7,8,9,19} cap=20 -> complement: {4,6,10..18}
    let a = [1, 2, 3, 5, 7, 8, 9, 2, 19, 0];
    let set = InversionList::new(20, &a).unwrap();
    let c = set.complement();
    assert_eq!(c.capacity(), 20);
    assert_eq!(c.support(), 11);
    assert_eq!(c.intervals.len(), 3);
    assert_eq!(c.intervals[0], (4, 5));
    assert_eq!(c.intervals[1], (6, 7));
    assert_eq!(c.intervals[2], (10, 19));
}

fn main() {}
