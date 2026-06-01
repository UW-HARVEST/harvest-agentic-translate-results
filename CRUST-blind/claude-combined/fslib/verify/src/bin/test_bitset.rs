use fslib::bitset::BitSet;

#[test]
fn test_set_get() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    bs.set(33);
    assert_eq!(bs.get(5), true);
    assert_eq!(bs.get(33), true);
    assert_eq!(bs.get(7), false);
    assert_eq!(bs.words.len(), 3);
}

#[test]
fn test_clear() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    assert_eq!(bs.get(5), true);
    bs.clear(5);
    assert_eq!(bs.get(5), false);
}

#[test]
fn test_clear_all_set_all() {
    let mut bs = BitSet::new(64);
    bs.set_all();
    assert_eq!(bs.get(0), true);
    assert_eq!(bs.get(63), true);
    bs.clear_all();
    assert_eq!(bs.get(0), false);
    assert_eq!(bs.get(63), false);
}

#[test]
fn test_union() {
    let mut a = BitSet::new(64);
    let mut b = BitSet::new(64);
    a.set(1);
    b.set(2);
    a.union(&b);
    assert_eq!(a.get(1), true);
    assert_eq!(a.get(2), true);
    assert_eq!(a.get(3), false);
}

#[test]
fn test_intersect() {
    let mut a = BitSet::new(64);
    let mut b = BitSet::new(64);
    a.set(1);
    a.set(2);
    b.set(2);
    b.set(3);
    a.intersect(&b);
    assert_eq!(a.get(1), false);
    assert_eq!(a.get(2), true);
    assert_eq!(a.get(3), false);
}

#[test]
fn test_toggle_all() {
    let mut a = BitSet::new(32);
    a.set(1);
    let t = a.toggle_all();
    assert_eq!(t.get(0), true);
    assert_eq!(t.get(1), false);
    assert_eq!(t.get(2), true);
}

fn main() {}
