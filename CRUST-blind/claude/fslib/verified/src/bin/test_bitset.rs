use fslib::bitset::BitSet;

#[test]
fn test_bitset_size_5() {
    // C: bitset_create(5) -> n_words = 5/32 + 1 = 1
    let bs = BitSet::new(5);
    assert_eq!(bs.words.len(), 1);
}

#[test]
fn test_bitset_size_32() {
    // C: bitset_create(32) -> n_words = 32/32 + 1 = 2
    let bs = BitSet::new(32);
    assert_eq!(bs.words.len(), 2);
}

#[test]
fn test_bitset_size_64() {
    // C: bitset_create(64) -> n_words = 64/32 + 1 = 3
    let bs = BitSet::new(64);
    assert_eq!(bs.words.len(), 3);
}

#[test]
fn test_bitset_size_0() {
    // C: bitset_create(0) -> n_words = 0/32 + 1 = 1
    let bs = BitSet::new(0);
    assert_eq!(bs.words.len(), 1);
}

#[test]
fn test_bitset_set_get() {
    let mut bs = BitSet::new(100);
    assert_eq!(bs.words.len(), 4); // 100/32 + 1 = 4
    bs.set(5);
    bs.set(7);
    bs.set(99);
    assert_eq!(bs.get(5), true);
    assert_eq!(bs.get(7), true);
    assert_eq!(bs.get(8), false);
    assert_eq!(bs.get(99), true);
    assert_eq!(bs.get(0), false);
}

#[test]
fn test_bitset_clear_specific() {
    let mut bs = BitSet::new(100);
    bs.set(5);
    bs.set(99);
    bs.clear(5);
    assert_eq!(bs.get(5), false);
    assert_eq!(bs.get(99), true);
}

#[test]
fn test_bitset_clear_all() {
    let mut bs = BitSet::new(100);
    bs.set(5);
    bs.set(99);
    bs.clear_all();
    assert_eq!(bs.get(5), false);
    assert_eq!(bs.get(99), false);
    for i in 0..100 {
        assert_eq!(bs.get(i), false);
    }
}

#[test]
fn test_bitset_set_all() {
    let mut bs = BitSet::new(100);
    bs.set_all();
    // C sets all bits within the n_words allocated, even bits beyond requested nbits
    assert_eq!(bs.get(0), true);
    assert_eq!(bs.get(50), true);
    assert_eq!(bs.get(99), true);
}

#[test]
fn test_bitset_intersect() {
    let mut a = BitSet::new(100);
    let mut b = BitSet::new(100);
    a.set(5);
    a.set(7);
    a.set(99);
    b.set(5);
    b.set(99);
    a.intersect(&b);
    assert_eq!(a.get(5), true);
    assert_eq!(a.get(7), false);
    assert_eq!(a.get(99), true);
}

#[test]
fn test_bitset_union() {
    let mut a = BitSet::new(50);
    let mut b = BitSet::new(50);
    a.set(5);
    a.set(10);
    b.set(10);
    b.set(20);
    a.union(&b);
    assert_eq!(a.get(5), true);
    assert_eq!(a.get(10), true);
    assert_eq!(a.get(20), true);
    assert_eq!(a.get(0), false);
}

#[test]
fn test_bitset_toggle_all() {
    // C toggles all bits within n_words allocated
    let mut a = BitSet::new(100);
    a.set(5);
    a.set(50);
    let toggled = a.toggle_all();
    assert_eq!(toggled.get(5), false);
    assert_eq!(toggled.get(50), false);
    assert_eq!(toggled.get(0), true);
    assert_eq!(toggled.get(99), true);
}

fn main() {}
