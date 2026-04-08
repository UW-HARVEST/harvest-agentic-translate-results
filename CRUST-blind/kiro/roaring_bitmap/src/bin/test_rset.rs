use roaring_bitmap::rset::RSet;

fn new_with_items(items: &[u16]) -> RSet {
    let mut set = RSet::new();
    for &item in items {
        assert!(set.add(item));
    }
    set
}

// --- new ---

#[test]
fn test_new() {
    let set = RSet::new();
    assert_eq!(set.cardinality(), 0);
    assert_eq!(set.length(), std::mem::size_of::<u16>() * 2); // 4 bytes
}

// --- truncate ---

#[test]
fn test_truncate() {
    let mut set = new_with_items(&[1, 2, 3, 4, 5]);
    assert_eq!(set.cardinality(), 5);
    assert!(set.truncate());
    assert_eq!(set.cardinality(), 0);
}

// --- fill ---

#[test]
fn test_fill() {
    let mut set = RSet::new();
    assert!(set.fill());
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), std::mem::size_of::<u16>()); // 2 bytes: just cardinality word
}

// --- add & contains (array mode) ---

#[test]
fn test_add_and_contains_basic() {
    let mut set = RSet::new();
    assert!(!set.contains(42));
    assert!(set.add(42));
    assert!(set.contains(42));
    assert_eq!(set.cardinality(), 1);
}

#[test]
fn test_add_idempotent() {
    let mut set = RSet::new();
    assert!(set.add(100));
    assert!(set.add(100)); // duplicate
    assert_eq!(set.cardinality(), 1);
}

#[test]
fn test_add_boundary_items() {
    let mut set = RSet::new();
    assert!(set.add(0));
    assert!(set.add(0xFFFF));
    assert!(set.contains(0));
    assert!(set.contains(0xFFFF));
    assert_eq!(set.cardinality(), 2);
}

#[test]
fn test_add_to_full_set() {
    let mut set = RSet::new();
    set.fill();
    assert!(set.add(42)); // should be no-op, returns true
    assert_eq!(set.cardinality(), 65536);
}

// --- contains on empty/full ---

#[test]
fn test_contains_empty() {
    let set = RSet::new();
    assert!(!set.contains(0));
    assert!(!set.contains(0xFFFF));
}

#[test]
fn test_contains_full() {
    let mut set = RSet::new();
    set.fill();
    assert!(set.contains(0));
    assert!(set.contains(0xFFFF));
    assert!(set.contains(32768));
}

// --- buffer resizing ---

#[test]
fn test_buffer_resizing() {
    let mut set = RSet::new();
    for i in 0..1000u16 {
        assert!(set.add(i));
    }
    assert_eq!(set.cardinality(), 1000);
}

// --- array to bitset conversion at LOW_CUTOFF (4096) ---

#[test]
fn test_array_to_bitset() {
    let mut set = RSet::new();
    // Add 32768 even numbers: 0, 2, 4, ..., 65534
    // This crosses the LOW_CUTOFF boundary and enters bitset mode
    for i in 0..32768u16 {
        assert!(set.add(i * 2));
    }
    assert_eq!(set.cardinality(), 32768);
}

// --- bitset to inverted array conversion at HIGH_CUTOFF (61440) ---

#[test]
fn test_bitset_to_inverted_array() {
    let mut set = RSet::new();
    for i in 0..=61440u16 {
        assert!(set.add(i));
    }
    assert_eq!(set.cardinality(), 61441);
}

// --- fill ascending ---

#[test]
fn test_fill_ascending() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in 0..65536u32 {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16)); // idempotent
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), std::mem::size_of::<u16>());
}

// --- fill descending ---

#[test]
fn test_fill_descending() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in (0..65536u32).rev() {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16)); // idempotent
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), std::mem::size_of::<u16>());
}

// --- fill optimal (first half ascending, second half descending) ---

#[test]
fn test_fill_optimal() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in 0..32768u32 {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16));
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    for i in (32768..65536u32).rev() {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16));
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), std::mem::size_of::<u16>());
}

// --- contains across all modes ---

#[test]
fn test_contains_all_modes() {
    let mut set = RSet::new();
    for i in 0..65536u32 {
        assert!(!set.contains(i as u16));
        assert!(set.add(i as u16));
        assert!(set.contains(i as u16));
    }
}

// --- equals ---

#[test]
fn test_equals_basic() {
    let set = new_with_items(&[1000, 2000, 3000]);
    let mut comparison = RSet::new();

    assert!(!set.equals(&comparison));
    comparison.add(1000);
    assert!(!set.equals(&comparison));
    comparison.add(2000);
    assert!(!set.equals(&comparison));
    comparison.add(3000);
    assert!(set.equals(&comparison));
}

#[test]
fn test_equals_different_items() {
    let a = new_with_items(&[1000, 2000, 3000]);
    let b = new_with_items(&[1000, 2000, 3001]);
    assert!(!a.equals(&b));
}

#[test]
fn test_equals_same_items() {
    let a = new_with_items(&[1000, 2000, 3000]);
    let b = new_with_items(&[1000, 2000, 3000]);
    assert!(a.equals(&b));
}

#[test]
fn test_equals_empty() {
    let a = RSet::new();
    let b = RSet::new();
    assert!(a.equals(&b));
}

#[test]
fn test_equals_full() {
    let mut a = RSet::new();
    let mut b = RSet::new();
    a.fill();
    b.fill();
    assert!(a.equals(&b));
}

// --- import / export ---

#[test]
fn test_import_export() {
    let set = new_with_items(&[1, 2, 3]);
    let exported = set.export();
    assert_eq!(set.length(), 4 * std::mem::size_of::<u16>()); // cardinality(3) -> 1+3 u16s

    let copy = RSet::import(&exported, set.length());
    assert!(set.equals(&copy));
}

#[test]
fn test_import_empty() {
    let set = RSet::import(&[], 8);
    assert_eq!(set.cardinality(), 0);
}

// --- copy ---

#[test]
fn test_copy_empty() {
    let set = RSet::new();
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(set.cardinality(), copy.cardinality());
    assert_eq!(set.length(), copy.length());
}

#[test]
fn test_copy_with_items() {
    let set = new_with_items(&[1, 2, 3, 4, 5]);
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(set.cardinality(), copy.cardinality());
    assert_eq!(set.length(), copy.length());
}

// --- length ---

#[test]
fn test_length_empty() {
    let set = RSet::new();
    // empty: cardinality=0, length_for(0)=sizeof(u16)*1=2, total=2+2=4
    assert_eq!(set.length(), 4);
}

#[test]
fn test_length_with_items() {
    let set = new_with_items(&[1, 2, 3]);
    // cardinality=3, length_for(3)=sizeof(u16)*3=6, total=2+6=8
    assert_eq!(set.length(), 8);
}

#[test]
fn test_length_full() {
    let mut set = RSet::new();
    set.fill();
    // full: cardinality=65536, length_for(65536)=0 (65536 >= HIGH_CUTOFF, 65536-65536=0, c=0 -> c=1? No...)
    // Actually length_for(65536): cardinality=65536 >= HIGH_CUTOFF(61440), so c = 65536-65536 = 0, then c=0 so c=1
    // Wait no: length_for checks cardinality==0 first. But cardinality passed is 65536, not 0.
    // cardinality=65536 >= HIGH_CUTOFF -> c = MAX_CARDINALITY - 65536 = 0. Then... c is 0 but the first check is if cardinality==0 not c==0.
    // Hmm, let me re-read: length_for(65536): cardinality=65536, not 0, so skip first branch.
    // 65536 >= 61440 -> c = 65536 - 65536 = 0. return sizeof(u16)*0 = 0.
    // total = 2 + 0 = 2
    assert_eq!(set.length(), 2);
}

// --- free ---

#[test]
fn test_free() {
    let mut set = new_with_items(&[1, 2, 3]);
    set.free();
    // After free, buffer is cleared
}

// --- invert ---

#[test]
fn test_invert_empty() {
    let set = RSet::new();
    let mut result = RSet::new();
    assert!(set.invert(&mut result));
    // ~empty = full
    assert_eq!(result.cardinality(), 65536);
}

#[test]
fn test_invert_full() {
    let mut set = RSet::new();
    set.fill();
    let mut result = RSet::new();
    assert!(set.invert(&mut result));
    // ~full = empty
    assert_eq!(result.cardinality(), 0);
}

#[test]
fn test_invert_large_set() {
    // Add items 4..65535 (65532 items), invert should give {0,1,2,3}
    let mut set = RSet::new();
    for i in 4..65536u32 {
        assert!(set.add(i as u16));
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    let expected = new_with_items(&[0, 1, 2, 3]);
    assert!(inverted.equals(&expected));
}

#[test]
fn test_invert_twice() {
    let mut set = RSet::new();
    for i in 4..65536u32 {
        assert!(set.add(i as u16));
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    let mut inverted_twice = RSet::new();
    assert!(inverted.invert(&mut inverted_twice));
    assert!(set.equals(&inverted_twice));
}

#[test]
fn test_invert_empty_gives_full_then_back() {
    let mut set = RSet::new();
    set.truncate();
    let mut expected_full = RSet::new();
    for i in 0..65536u32 {
        expected_full.add(i as u16);
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 65536);
    assert!(inverted.equals(&expected_full));

    let mut inverted_twice = RSet::new();
    assert!(inverted.invert(&mut inverted_twice));
    assert_eq!(inverted_twice.cardinality(), 0);
    assert!(set.equals(&inverted_twice));
}

#[test]
fn test_invert_bitset_range() {
    // 30000 items -> bitset mode, invert gives 35536 items
    let mut set = RSet::new();
    for i in 0..30000u16 {
        assert!(set.add(i));
    }
    let mut expected = RSet::new();
    for i in 30000..65536u32 {
        expected.add(i as u16);
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 35536);
    assert!(inverted.equals(&expected));

    let mut inverted_twice = RSet::new();
    assert!(inverted.invert(&mut inverted_twice));
    assert_eq!(inverted_twice.cardinality(), 30000);
    assert!(set.equals(&inverted_twice));
}

// --- intersection ---

#[test]
fn test_intersection_with_empty() {
    let a = RSet::new();
    let b = new_with_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut result = RSet::new();

    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
    assert!(b.intersection(&a, &mut result));
    assert_eq!(result.cardinality(), 0);
}

#[test]
fn test_intersection_with_full() {
    let mut a = RSet::new();
    a.fill();
    let b = new_with_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut result = RSet::new();

    assert!(a.intersection(&b, &mut result));
    assert!(b.equals(&result));
    assert!(b.intersection(&a, &mut result));
    assert!(b.equals(&result));
}

#[test]
fn test_intersection_arrays() {
    let mut a = RSet::new();
    for i in (0..100u16).step_by(2) {
        a.add(i);
    }
    let b = new_with_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut result = RSet::new();

    assert!(a.intersection(&b, &mut result));
    let expected = new_with_items(&[0, 2, 4, 6, 8]);
    assert!(result.equals(&expected));
}

#[test]
fn test_intersection_disjoint() {
    let mut a = RSet::new();
    for i in (0..100u16).step_by(2) {
        a.add(i);
    }
    let mut b = RSet::new();
    for i in (1..100u16).step_by(2) {
        b.add(i);
    }
    let mut result = RSet::new();

    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
}

fn main() {}
