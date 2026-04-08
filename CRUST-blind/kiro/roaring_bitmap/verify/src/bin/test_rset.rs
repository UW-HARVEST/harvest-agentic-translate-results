use roaring_bitmap::rset::RSet;

// Helper: create a set with given items
fn new_items(items: &[u16]) -> RSet {
    let mut s = RSet::new();
    for &i in items { s.add(i); }
    s
}

#[test]
fn test_new() {
    let s = RSet::new();
    assert_eq!(s.cardinality(), 0);
    assert_eq!(s.length(), 4); // sizeof(u16) * 2
}

#[test]
fn test_add_three_items() {
    let mut s = RSet::new();
    assert!(s.add(1000));
    assert!(s.add(2000));
    assert!(s.add(3000));
    assert_eq!(s.cardinality(), 3);
    assert_eq!(s.length(), 8); // sizeof(u16) * (1 + 3)
}

#[test]
fn test_truncate() {
    let mut s = new_items(&[1, 2, 3, 4, 5]);
    assert_eq!(s.cardinality(), 5);
    assert!(s.truncate());
    assert_eq!(s.cardinality(), 0);
    assert_eq!(s.length(), 4);
}

#[test]
fn test_fill() {
    let mut s = RSet::new();
    assert!(s.fill());
    assert_eq!(s.cardinality(), 65536);
    assert_eq!(s.length(), 2); // sizeof(u16) * 1
}

#[test]
fn test_buffer_resizing() {
    let mut s = RSet::new();
    for i in 0..1000u16 { assert!(s.add(i)); }
    assert_eq!(s.cardinality(), 1000);
}

#[test]
fn test_array_to_bitset() {
    // Add even numbers 0,2,4,...,65534 => 32768 items, triggers array->bitset conversion
    let mut s = RSet::new();
    for i in 0..32768u16 { assert!(s.add(i * 2)); }
    assert_eq!(s.cardinality(), 32768);
    // Verify contains for even/odd
    assert!(s.contains(0));
    assert!(s.contains(2));
    assert!(s.contains(65534));
    assert!(!s.contains(1));
    assert!(!s.contains(3));
}

#[test]
fn test_bitset_to_inverted_array() {
    // Add 0..=61440 => 61441 items, triggers bitset->inverted-array conversion
    let mut s = RSet::new();
    for i in 0..=61440u16 { assert!(s.add(i)); }
    assert_eq!(s.cardinality(), 61441);
    // Verify contains
    assert!(s.contains(0));
    assert!(s.contains(61440));
    assert!(!s.contains(61441));
    assert!(!s.contains(65535));
}

#[test]
fn test_import_export_roundtrip() {
    let s = new_items(&[1, 2, 3]);
    assert_eq!(s.cardinality(), 3);
    assert_eq!(s.length(), 8);
    let exported = s.export();
    let copy = RSet::import(&exported, s.length());
    assert!(s.equals(&copy));
    assert_eq!(copy.cardinality(), 3);
}

#[test]
fn test_copy_empty() {
    let s = RSet::new();
    let copy = s.copy();
    assert!(s.equals(&copy));
    assert_eq!(copy.cardinality(), 0);
    assert_eq!(copy.length(), 4);
}

#[test]
fn test_copy_with_items() {
    let s = new_items(&[1, 2, 3, 4, 5]);
    let copy = s.copy();
    assert!(s.equals(&copy));
    assert_eq!(copy.cardinality(), 5);
    assert_eq!(copy.length(), 12);
}

#[test]
fn test_contains_array() {
    let s = new_items(&[10, 20, 30]);
    assert!(s.contains(10));
    assert!(!s.contains(15));
    assert!(s.contains(20));
    assert!(s.contains(30));
    assert!(!s.contains(0));
}

#[test]
fn test_contains_full() {
    let mut s = RSet::new();
    s.fill();
    assert!(s.contains(0));
    assert!(s.contains(100));
    assert!(s.contains(65535));
}

#[test]
fn test_contains_empty() {
    let s = RSet::new();
    assert!(!s.contains(0));
    assert!(!s.contains(100));
}

#[test]
fn test_equals_different() {
    let a = new_items(&[1000, 2000, 3000]);
    let b = RSet::new();
    assert!(!a.equals(&b));
}

#[test]
fn test_equals_same() {
    let a = new_items(&[1000, 2000, 3000]);
    let b = new_items(&[1000, 2000, 3000]);
    assert!(a.equals(&b));
}

#[test]
fn test_equals_different_last_item() {
    let a = new_items(&[1000, 2000, 3000]);
    let b = new_items(&[1000, 2000, 3001]);
    assert!(!a.equals(&b));
}

#[test]
fn test_equals_empty_sets() {
    let a = RSet::new();
    let b = RSet::new();
    assert!(a.equals(&b));
}

#[test]
fn test_equals_full_sets() {
    let mut a = RSet::new();
    a.fill();
    let mut b = RSet::new();
    b.fill();
    assert!(a.equals(&b));
}

#[test]
fn test_invert_large_set() {
    // Invert {4..65535} => should give {0,1,2,3}
    let mut s = RSet::new();
    for i in 4..=65535u16 { s.add(i); }
    let mut inv = RSet::new();
    assert!(s.invert(&mut inv));
    assert_eq!(inv.cardinality(), 4);
    assert!(inv.contains(0));
    assert!(inv.contains(1));
    assert!(inv.contains(2));
    assert!(inv.contains(3));
    assert!(!inv.contains(4));
}

#[test]
fn test_invert_empty_gives_full() {
    let s = RSet::new();
    let mut inv = RSet::new();
    assert!(s.invert(&mut inv));
    assert_eq!(inv.cardinality(), 65536);
}

#[test]
fn test_invert_full_gives_empty() {
    let mut s = RSet::new();
    s.fill();
    let mut inv = RSet::new();
    assert!(s.invert(&mut inv));
    assert_eq!(inv.cardinality(), 0);
}

#[test]
fn test_invert_bitset_range() {
    // Invert {0..29999} => {30000..65535}, cardinality 35536
    let mut s = RSet::new();
    for i in 0..30000u16 { s.add(i); }
    let mut inv = RSet::new();
    assert!(s.invert(&mut inv));
    assert_eq!(inv.cardinality(), 35536);
    assert!(!inv.contains(0));
    assert!(!inv.contains(29999));
    assert!(inv.contains(30000));
    assert!(inv.contains(65535));
}

#[test]
fn test_double_invert_identity() {
    let s = new_items(&[0, 1, 2, 3]);
    let mut inv = RSet::new();
    assert!(s.invert(&mut inv));
    let mut inv2 = RSet::new();
    assert!(inv.invert(&mut inv2));
    assert!(s.equals(&inv2));
    assert_eq!(inv2.cardinality(), 4);
}

#[test]
fn test_intersection_empty() {
    let a = RSet::new();
    let b = new_items(&[0, 1, 2, 3, 4]);
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
    // Also test b & a
    let mut result2 = RSet::new();
    assert!(b.intersection(&a, &mut result2));
    assert_eq!(result2.cardinality(), 0);
}

#[test]
fn test_intersection_full() {
    let mut a = RSet::new();
    a.fill();
    let b = new_items(&[0, 1, 2, 3, 4]);
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 5);
    assert!(b.equals(&result));
}

#[test]
fn test_intersection_overlap() {
    // a = even numbers 0..98, b = {0..9}
    let mut a = RSet::new();
    for i in (0..100u16).step_by(2) { a.add(i); }
    let b = new_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 5);
    // Expected: {0, 2, 4, 6, 8}
    assert!(result.contains(0));
    assert!(result.contains(2));
    assert!(result.contains(4));
    assert!(result.contains(6));
    assert!(result.contains(8));
    assert!(!result.contains(1));
    assert!(!result.contains(3));
}

#[test]
fn test_intersection_disjoint() {
    let mut a = RSet::new();
    for i in (0..100u16).step_by(2) { a.add(i); }
    let mut b = RSet::new();
    for i in (1..100u16).step_by(2) { b.add(i); }
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
}

#[test]
fn test_length_various() {
    let s = RSet::new();
    assert_eq!(s.length(), 4); // empty

    let mut s = RSet::new();
    s.fill();
    assert_eq!(s.length(), 2); // full

    let s = new_items(&[42]);
    assert_eq!(s.length(), 4); // 1 item

    let s = new_items(&[1, 2, 3, 4, 5]);
    assert_eq!(s.length(), 12); // 5 items
}

#[test]
fn test_add_idempotent() {
    let mut s = RSet::new();
    s.add(42);
    s.add(42);
    assert_eq!(s.cardinality(), 1);
}

#[test]
fn test_add_to_full_set() {
    let mut s = RSet::new();
    s.fill();
    assert!(s.add(42)); // should succeed (no-op)
    assert_eq!(s.cardinality(), 65536);
}

#[test]
fn test_fill_ascending() {
    // Add all items 0..65535 ascending, verify full set
    let mut s = RSet::new();
    for i in 0..=65535u16 {
        assert!(s.add(i));
        assert!(s.add(i)); // idempotent
    }
    assert_eq!(s.cardinality(), 65536);
    assert_eq!(s.length(), 2);
}

#[test]
fn test_fill_descending() {
    let mut s = RSet::new();
    for i in (0..=65535u16).rev() {
        assert!(s.add(i));
        assert!(s.add(i)); // idempotent
    }
    assert_eq!(s.cardinality(), 65536);
    assert_eq!(s.length(), 2);
}

#[test]
fn test_contains_inverted_array() {
    // After bitset->inverted-array, contains should work
    let mut s = RSet::new();
    for i in 0..=61440u16 { s.add(i); }
    assert_eq!(s.cardinality(), 61441);
    assert!(s.contains(0));
    assert!(s.contains(30000));
    assert!(s.contains(61440));
    assert!(!s.contains(61441));
    assert!(!s.contains(65535));
}

#[test]
fn test_export_import_preserves_data() {
    // Test with various set sizes
    let s = new_items(&[100, 200, 300, 400, 500]);
    let exported = s.export();
    let imported = RSet::import(&exported, s.length());
    assert!(s.equals(&imported));
    assert_eq!(imported.cardinality(), 5);
    assert!(imported.contains(100));
    assert!(imported.contains(500));
    assert!(!imported.contains(101));
}

#[test]
fn test_copy_preserves_contains() {
    let s = new_items(&[10, 20, 30]);
    let copy = s.copy();
    assert!(copy.contains(10));
    assert!(copy.contains(20));
    assert!(copy.contains(30));
    assert!(!copy.contains(15));
}

#[test]
fn test_truncate_then_add() {
    let mut s = new_items(&[1, 2, 3]);
    s.truncate();
    assert_eq!(s.cardinality(), 0);
    s.add(42);
    assert_eq!(s.cardinality(), 1);
    assert!(s.contains(42));
}

fn main() {}
