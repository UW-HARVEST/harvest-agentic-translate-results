use fslib::bitset::BitSet;

#[test]
fn test_new() {
    let bs = BitSet::new(64);
    assert_eq!(bs.get(0), false);
}

#[test]
fn test_set_get() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    assert_eq!(bs.get(5), true);
    assert_eq!(bs.get(4), false);
}

#[test]
fn test_clear() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    bs.clear(5);
    assert_eq!(bs.get(5), false);
}

#[test]
fn test_set_all() {
    let mut bs = BitSet::new(64);
    bs.set_all();
    assert_eq!(bs.get(0), true);
    assert_eq!(bs.get(31), true);
    assert_eq!(bs.get(63), true);
}

#[test]
fn test_clear_all() {
    let mut bs = BitSet::new(64);
    bs.set_all();
    bs.clear_all();
    assert_eq!(bs.get(0), false);
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
    let mut c = BitSet::new(64);
    let mut d = BitSet::new(64);
    c.set(1);
    c.set(2);
    d.set(2);
    d.set(3);
    c.intersect(&d);
    assert_eq!(c.get(1), false);
    assert_eq!(c.get(2), true);
    assert_eq!(c.get(3), false);
}

#[test]
fn test_toggle_all() {
    let mut e = BitSet::new(64);
    e.set(0);
    let toggled = e.toggle_all();
    assert_eq!(toggled.get(0), false);
    assert_eq!(toggled.get(1), true);
}

fn main() {}
