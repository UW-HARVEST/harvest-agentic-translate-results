use inversion_list::inversion_list::InversionList;

#[test]
fn test_complement_with_zero() {
    // C test-complement case 1: a={1,2,3,5,7,8,9,0,2}, capacity=20
    // set has couples=[0,4,5,6,7,10]
    // complement: capacity=20, support=12, size=6, couples=[4,5,6,7,10,20]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 0, 2]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 20);
    assert_eq!(comp.support(), 12);
    assert_eq!(comp.intervals.len(), 3);
    assert_eq!(comp.intervals[0], (4, 5));
    assert_eq!(comp.intervals[1], (6, 7));
    assert_eq!(comp.intervals[2], (10, 20));
}

#[test]
fn test_complement_no_zero_no_top() {
    // C test-complement case 2: a={1,2,3,5,7,8,9,2}, capacity=20
    // set: couples=[1,4,5,6,7,10]
    // complement: capacity=20, support=13, size=8, couples=[0,1,4,5,6,7,10,20]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 20);
    assert_eq!(comp.support(), 13);
    assert_eq!(comp.intervals.len(), 4);
    assert_eq!(comp.intervals[0], (0, 1));
    assert_eq!(comp.intervals[1], (4, 5));
    assert_eq!(comp.intervals[2], (6, 7));
    assert_eq!(comp.intervals[3], (10, 20));
}

#[test]
fn test_complement_no_zero_with_top_capacity() {
    // C test-complement case 3: a={1,2,3,5,7,8,9,2,19}, capacity=20
    // set: couples=[1,4,5,6,7,10,19,20]
    // complement: capacity=20, support=12, size=8, couples=[0,1,4,5,6,7,10,19]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2, 19]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 20);
    assert_eq!(comp.support(), 12);
    assert_eq!(comp.intervals.len(), 4);
    assert_eq!(comp.intervals[0], (0, 1));
    assert_eq!(comp.intervals[1], (4, 5));
    assert_eq!(comp.intervals[2], (6, 7));
    assert_eq!(comp.intervals[3], (10, 19));
}

#[test]
fn test_complement_with_zero_with_top_capacity() {
    // C test-complement case 4: a={1,2,3,5,7,8,9,2,19,0}, capacity=20
    // set: couples=[0,4,5,6,7,10,19,20]
    // complement: capacity=20, support=11, size=6, couples=[4,5,6,7,10,19]
    let set = InversionList::new(20, &[1, 2, 3, 5, 7, 8, 9, 2, 19, 0]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 20);
    assert_eq!(comp.support(), 11);
    assert_eq!(comp.intervals.len(), 3);
    assert_eq!(comp.intervals[0], (4, 5));
    assert_eq!(comp.intervals[1], (6, 7));
    assert_eq!(comp.intervals[2], (10, 19));
}

#[test]
fn test_complement_full_set() {
    // C: complement of {0,1,2,3} cap=4 -> empty set
    let set = InversionList::new(4, &[0, 1, 2, 3]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 4);
    assert_eq!(comp.support(), 0);
    assert_eq!(comp.intervals.len(), 0);
}

#[test]
fn test_complement_single_at_top() {
    // C: complement of {19} cap=20 -> all of [0,19), support=19, size=2
    let set = InversionList::new(20, &[19]).unwrap();
    let comp = set.complement();
    assert_eq!(comp.capacity(), 20);
    assert_eq!(comp.support(), 19);
    assert_eq!(comp.intervals.len(), 1);
    assert_eq!(comp.intervals[0], (0, 19));
}

fn main() {}
