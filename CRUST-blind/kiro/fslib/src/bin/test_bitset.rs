use fslib::bitset::BitSet;

#[test]
fn test_new_all_zero() {
    let bs = BitSet::new(64);
    assert!(!bs.get(0));
    assert!(!bs.get(31));
    assert!(!bs.get(63));
}

#[test]
fn test_set_and_get() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    assert!(bs.get(5));
    assert!(!bs.get(6));
    assert!(!bs.get(4));
}

#[test]
fn test_clear() {
    let mut bs = BitSet::new(64);
    bs.set(5);
    bs.clear(5);
    assert!(!bs.get(5));
}

#[test]
fn test_set_all() {
    let mut bs = BitSet::new(64);
    bs.set_all();
    assert!(bs.get(0));
    assert!(bs.get(31));
    assert!(bs.get(32));
}

#[test]
fn test_clear_all() {
    let mut bs = BitSet::new(64);
    bs.set_all();
    bs.clear_all();
    assert!(!bs.get(0));
    assert!(!bs.get(31));
}

#[test]
fn test_toggle_all() {
    let mut bs = BitSet::new(64);
    bs.set(3);
    let toggled = bs.toggle_all();
    // bit 3 was set, after toggle it should be clear
    assert!(!toggled.get(3));
    // bit 4 was clear, after toggle it should be set
    assert!(toggled.get(4));
}

#[test]
fn test_union() {
    let mut a = BitSet::new(64);
    let mut b = BitSet::new(64);
    a.set(1);
    a.set(3);
    b.set(2);
    b.set(3);
    a.union(&b);
    assert!(a.get(1));
    assert!(a.get(2));
    assert!(a.get(3));
    assert!(!a.get(4));
}

#[test]
fn test_intersect() {
    let mut a = BitSet::new(64);
    let mut b = BitSet::new(64);
    a.set(1);
    a.set(3);
    b.set(2);
    b.set(3);
    a.intersect(&b);
    assert!(!a.get(1));
    assert!(!a.get(2));
    assert!(a.get(3));
}

#[test]
fn test_boundary_bit_0() {
    let mut bs = BitSet::new(1);
    bs.set(0);
    assert!(bs.get(0));
    bs.clear(0);
    assert!(!bs.get(0));
}

fn main() {}
