use roaring_bitmap::rset::RSet;

// Helper function to construct a set from items, mirroring rset_new_items in tests.c.
#[allow(dead_code)]
fn rset_new_items(items: &[u16]) -> RSet {
    // In C, rset_new_items uses rset_import(NULL, count). With buffer=NULL the
    // import path constructs an empty set. We do the same here.
    let mut set = RSet::import(&[], items.len());
    for &it in items {
        assert!(set.add(it));
    }
    set
}

// ===== test_new =====
#[test]
fn test_new() {
    let set = RSet::new();
    assert_eq!(set.cardinality(), 0);
    // C returns sizeof(uint16_t) * 2 = 4 bytes for an empty set.
    assert_eq!(set.length(), 4);
}

// ===== test_new_items =====
#[test]
fn test_new_items_empty() {
    let set = rset_new_items(&[]);
    assert_eq!(set.cardinality(), 0);
    assert_eq!(set.length(), 4);
}

#[test]
fn test_new_items_three() {
    let set = rset_new_items(&[1000, 2000, 3000]);
    assert_eq!(set.cardinality(), 3);
    // (1 + 3) * 2 = 8 bytes
    assert_eq!(set.length(), 8);
    assert!(set.contains(1000));
    assert!(set.contains(2000));
    assert!(set.contains(3000));
    assert!(!set.contains(0));
    assert!(!set.contains(999));
    assert!(!set.contains(1001));
    assert!(!set.contains(2001));
    assert!(!set.contains(3001));
}

// ===== test_equals =====
#[test]
fn test_equals_basic() {
    let set = rset_new_items(&[1000, 2000, 3000]);
    let mut comparison = RSet::new();

    // Different cardinality (3 vs 0)
    assert!(!set.equals(&comparison));
    // Add items step by step
    assert!(comparison.add(1000));
    assert!(!set.equals(&comparison));
    assert!(comparison.add(2000));
    assert!(!set.equals(&comparison));
    assert!(comparison.add(3000));
    assert!(set.equals(&comparison));

    // Same cardinality, different content
    let other = rset_new_items(&[1000, 2000, 3001]);
    assert!(!set.equals(&other));

    // Equal sets
    let same = rset_new_items(&[1000, 2000, 3000]);
    assert!(set.equals(&same));
}

#[test]
fn test_equals_empty_sets() {
    let a = RSet::new();
    let b = RSet::new();
    assert!(a.equals(&b));
    assert_eq!(a.cardinality(), 0);
    assert_eq!(b.cardinality(), 0);
}

#[test]
fn test_equals_full_sets() {
    let mut a = RSet::new();
    let mut b = RSet::new();
    a.fill();
    b.fill();
    assert!(a.equals(&b));
    assert_eq!(a.cardinality(), 65536);
    assert_eq!(b.cardinality(), 65536);
}

// ===== test_import_export =====
#[test]
fn test_import_export() {
    let set = rset_new_items(&[1, 2, 3]);
    // Length is 4 * sizeof(u16) = 8 bytes.
    assert_eq!(set.length(), 8);
    let exported = set.export();
    assert_eq!(exported.len(), 8);

    // First two bytes is cardinality (3) in little-endian.
    assert_eq!(exported[0], 3);
    assert_eq!(exported[1], 0);
    // Then items 1, 2, 3 in little-endian u16.
    assert_eq!(exported[2], 1);
    assert_eq!(exported[3], 0);
    assert_eq!(exported[4], 2);
    assert_eq!(exported[5], 0);
    assert_eq!(exported[6], 3);
    assert_eq!(exported[7], 0);

    let copy = RSet::import(&exported, set.length());
    assert!(set.equals(&copy));
    assert_eq!(copy.cardinality(), 3);
    assert_eq!(copy.length(), set.length());
}

// ===== test_copy =====
#[test]
fn test_copy_empty() {
    let set = RSet::new();
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(set.cardinality(), copy.cardinality());
    assert_eq!(set.length(), copy.length());
    assert_eq!(copy.cardinality(), 0);
    assert_eq!(copy.length(), 4);
}

#[test]
fn test_copy_with_items() {
    let set = rset_new_items(&[1, 2, 3, 4, 5]);
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(set.cardinality(), copy.cardinality());
    assert_eq!(set.length(), copy.length());
    assert_eq!(copy.cardinality(), 5);
    assert!(copy.contains(1));
    assert!(copy.contains(5));
}

// ===== test_truncate =====
#[test]
fn test_truncate() {
    let mut set = rset_new_items(&[1, 2, 3, 4, 5]);
    assert_eq!(set.cardinality(), 5);
    assert!(set.truncate());
    assert_eq!(set.cardinality(), 0);
    assert_eq!(set.length(), 4);
    // After truncation, no item should be contained.
    assert!(!set.contains(1));
    assert!(!set.contains(2));
    assert!(!set.contains(3));
}

// ===== test_buffer_resizing =====
#[test]
fn test_buffer_resizing() {
    let mut set = RSet::new();
    for i in 0..1000u16 {
        assert!(set.add(i));
    }
    assert_eq!(set.cardinality(), 1000);
    // Should be array storage; length = (1 + 1000) * 2 = 2002 bytes.
    assert_eq!(set.length(), 2002);
    // Spot-check a few items.
    assert!(set.contains(0));
    assert!(set.contains(500));
    assert!(set.contains(999));
    assert!(!set.contains(1000));
}

// ===== test_array_to_bitset =====
#[test]
fn test_array_to_bitset() {
    let mut set = RSet::new();
    // Add 32768 items (0, 2, 4, ..., 65534), forcing conversion to bitset.
    for i in 0..32768u16 {
        assert!(set.add(i.wrapping_mul(2)));
    }
    assert_eq!(set.cardinality(), 32768);
    // Bitset length: 2 + 4096 * 2 = 8194 bytes.
    assert_eq!(set.length(), 8194);
    // Spot check: every even item is in the set, odd items are not.
    assert!(set.contains(0));
    assert!(set.contains(2));
    assert!(set.contains(65534));
    assert!(!set.contains(1));
    assert!(!set.contains(3));
    assert!(!set.contains(65535));
}

// ===== test_bitset_to_inverted_array =====
#[test]
fn test_bitset_to_inverted_array() {
    let mut set = RSet::new();
    // Add 0..=61440 (61441 items), triggering inverted array storage.
    for i in 0..=61440u32 {
        assert!(set.add(i as u16));
    }
    assert_eq!(set.cardinality(), 61441);
    // Inverted array length: 2 + (65536 - 61441) * 2 = 2 + 4095 * 2 = 8192 bytes.
    assert_eq!(set.length(), 8192);
    // 0..=61440 contained, 61441..=65535 not contained.
    assert!(set.contains(0));
    assert!(set.contains(30000));
    assert!(set.contains(61440));
    assert!(!set.contains(61441));
    assert!(!set.contains(65535));
}

// ===== test_fill_ascending =====
#[test]
fn test_fill_ascending() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in 0u32..65536 {
        assert!(set.add(i as u16));
        // Idempotent
        assert!(set.add(i as u16));
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    // Full set length is sizeof(u16) = 2 bytes.
    assert_eq!(set.length(), 2);
}

// ===== test_fill_descending =====
#[test]
fn test_fill_descending() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in (0i32..=65535).rev() {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16)); // idempotent
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), 2);
}

// ===== test_fill_optimal =====
#[test]
fn test_fill_optimal() {
    let mut set = RSet::new();
    let mut comparison = RSet::new();
    for i in 0u32..32768 {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16));
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    for i in (32768u32..=65535).rev() {
        assert!(set.add(i as u16));
        assert!(set.add(i as u16));
        assert!(comparison.add(i as u16));
        assert!(set.equals(&comparison));
    }
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), 2);
}

// ===== test_contains =====
#[test]
fn test_contains_progression() {
    // Walks through array → bitset → inverted_array → full
    // transitions, checking each addition.
    let mut set = RSet::new();
    for i in 0u32..65536 {
        assert!(!set.contains(i as u16));
        assert!(set.add(i as u16));
        assert!(set.contains(i as u16));
    }
}

#[test]
fn test_contains_in_empty_full() {
    // Empty set
    let mut set = RSet::new();
    assert!(!set.contains(0));
    assert!(!set.contains(65535));
    assert!(!set.contains(12345));

    // Full set
    set.fill();
    assert!(set.contains(0));
    assert!(set.contains(65535));
    assert!(set.contains(12345));
}

// ===== test_invert =====
#[test]
fn test_invert_array_to_inverted_array() {
    // Set has 65532 items (4..65535), result should have 4 items (0,1,2,3)
    let mut set = RSet::new();
    for i in 4u32..65536 {
        assert!(set.add(i as u16));
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    let expected = rset_new_items(&[0, 1, 2, 3]);
    assert!(inverted.equals(&expected));
    assert_eq!(inverted.cardinality(), 4);
    assert!(inverted.contains(0));
    assert!(inverted.contains(1));
    assert!(inverted.contains(2));
    assert!(inverted.contains(3));
    assert!(!inverted.contains(4));
    assert!(!inverted.contains(65535));

    // Double-invert returns original
    let mut inverted_twice = RSet::new();
    assert!(inverted.invert(&mut inverted_twice));
    assert!(set.equals(&inverted_twice));
    assert_eq!(inverted_twice.cardinality(), 65532);
}

#[test]
fn test_invert_empty_to_full() {
    let set = RSet::new();
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 65536);
    // Length of full set is 2 bytes.
    assert_eq!(inverted.length(), 2);
}

#[test]
fn test_invert_full_to_empty() {
    let mut set = RSet::new();
    set.fill();
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 0);
    assert_eq!(inverted.length(), 4);
}

#[test]
fn test_invert_bitset() {
    // 30000 items maps to 35536 items (both in bitset range).
    let mut set = RSet::new();
    for i in 0u32..30000 {
        assert!(set.add(i as u16));
    }
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 35536);

    // Build the expected set: 30000..65535 inclusive.
    let mut expected = RSet::new();
    for i in 30000u32..65536 {
        assert!(expected.add(i as u16));
    }
    assert!(inverted.equals(&expected));
    // Spot check via contains
    assert!(!inverted.contains(0));
    assert!(!inverted.contains(29999));
    assert!(inverted.contains(30000));
    assert!(inverted.contains(65535));

    // Double invert returns original cardinality.
    let mut inverted_twice = RSet::new();
    assert!(inverted.invert(&mut inverted_twice));
    assert_eq!(inverted_twice.cardinality(), 30000);
    assert!(set.equals(&inverted_twice));
}

// ===== test_intersection =====
#[test]
fn test_intersection_empty_with_set() {
    let a = RSet::new();
    let b = rset_new_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);

    // Reverse order: B & empty = empty
    assert!(b.intersection(&a, &mut result));
    assert_eq!(result.cardinality(), 0);
}

#[test]
fn test_intersection_full_with_set() {
    let mut a = RSet::new();
    let b = rset_new_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    a.fill();
    let mut result = RSet::new();

    // Full & B = B
    assert!(a.intersection(&b, &mut result));
    assert!(b.equals(&result));
    assert_eq!(result.cardinality(), 10);

    // B & Full = B
    assert!(b.intersection(&a, &mut result));
    assert!(b.equals(&result));
    assert_eq!(result.cardinality(), 10);
}

#[test]
fn test_intersection_arrays() {
    // a: even numbers from 0..100, b: 0..10
    let b = rset_new_items(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9]);
    let mut a = RSet::new();
    for i in (0u32..100).step_by(2) {
        assert!(a.add(i as u16));
    }
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    let expected = rset_new_items(&[0, 2, 4, 6, 8]);
    assert!(result.equals(&expected));
    assert_eq!(result.cardinality(), 5);
}

#[test]
fn test_intersection_disjoint_arrays() {
    // a: even numbers from 0..100, b: odd numbers from 1..100
    let mut a = RSet::new();
    let mut b = RSet::new();
    for i in (0u32..100).step_by(2) {
        assert!(a.add(i as u16));
    }
    for i in (1u32..100).step_by(2) {
        assert!(b.add(i as u16));
    }
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
    assert_eq!(result.length(), 4);
}

#[test]
fn test_intersection_full_full() {
    let mut a = RSet::new();
    let mut b = RSet::new();
    a.fill();
    b.fill();
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 65536);
}

#[test]
fn test_intersection_empty_empty() {
    let a = RSet::new();
    let b = RSet::new();
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 0);
}

#[test]
fn test_intersection_bitset_bitset() {
    // a = 0..30000, b = 10000..40000 - both bitsets
    let mut a = RSet::new();
    let mut b = RSet::new();
    for i in 0u32..30000 {
        assert!(a.add(i as u16));
    }
    for i in 10000u32..40000 {
        assert!(b.add(i as u16));
    }
    let mut result = RSet::new();
    assert!(a.intersection(&b, &mut result));
    assert_eq!(result.cardinality(), 20000);
    // Verify membership
    assert!(!result.contains(0));
    assert!(!result.contains(9999));
    assert!(result.contains(10000));
    assert!(result.contains(29999));
    assert!(!result.contains(30000));
}

// ===== Additional tests for fill() and copy() =====
#[test]
fn test_fill_basic() {
    let mut set = RSet::new();
    assert_eq!(set.cardinality(), 0);
    assert!(set.fill());
    assert_eq!(set.cardinality(), 65536);
    assert_eq!(set.length(), 2);
    assert!(set.contains(0));
    assert!(set.contains(32768));
    assert!(set.contains(65535));
}

#[test]
fn test_copy_full() {
    let mut set = RSet::new();
    set.fill();
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(copy.cardinality(), 65536);
    assert_eq!(copy.length(), 2);
}

#[test]
fn test_copy_bitset() {
    let mut set = RSet::new();
    for i in 0u32..30000 {
        assert!(set.add(i as u16));
    }
    let copy = set.copy();
    assert!(set.equals(&copy));
    assert_eq!(copy.cardinality(), 30000);
    assert_eq!(copy.length(), set.length());
    // Spot-check contains
    assert!(copy.contains(0));
    assert!(copy.contains(15000));
    assert!(copy.contains(29999));
    assert!(!copy.contains(30000));
}

// ===== Additional contains coverage =====
#[test]
fn test_contains_array_storage() {
    let set = rset_new_items(&[10, 20, 30, 40, 50]);
    assert!(set.contains(10));
    assert!(set.contains(20));
    assert!(set.contains(30));
    assert!(set.contains(40));
    assert!(set.contains(50));
    assert!(!set.contains(0));
    assert!(!set.contains(15));
    assert!(!set.contains(25));
    assert!(!set.contains(35));
    assert!(!set.contains(60));
    assert!(!set.contains(65535));
}

#[test]
fn test_contains_bitset_storage() {
    // 5000 items in bitset range
    let mut set = RSet::new();
    for i in 0u32..5000 {
        assert!(set.add(i as u16));
    }
    assert_eq!(set.cardinality(), 5000);
    assert!(set.contains(0));
    assert!(set.contains(2500));
    assert!(set.contains(4999));
    assert!(!set.contains(5000));
    assert!(!set.contains(65535));
}

#[test]
fn test_contains_inverted_array_storage() {
    // Items 0..62000 (62000 items), in inverted array range (62000 > 61440).
    let mut set = RSet::new();
    for i in 0u32..62000 {
        assert!(set.add(i as u16));
    }
    assert_eq!(set.cardinality(), 62000);
    assert!(set.contains(0));
    assert!(set.contains(30000));
    assert!(set.contains(61999));
    assert!(!set.contains(62000));
    assert!(!set.contains(65535));
}

// ===== Edge cases for add =====
#[test]
fn test_add_idempotent() {
    let mut set = RSet::new();
    assert!(set.add(42));
    assert_eq!(set.cardinality(), 1);
    assert!(set.add(42));
    assert_eq!(set.cardinality(), 1);
    assert!(set.add(42));
    assert_eq!(set.cardinality(), 1);
    assert!(set.contains(42));
}

#[test]
fn test_add_after_full() {
    let mut set = RSet::new();
    set.fill();
    // Adding to a full set should still succeed and remain full.
    assert!(set.add(0));
    assert!(set.add(12345));
    assert!(set.add(65535));
    assert_eq!(set.cardinality(), 65536);
}

#[test]
fn test_add_zero_and_max() {
    let mut set = RSet::new();
    assert!(set.add(0));
    assert!(set.add(65535));
    assert_eq!(set.cardinality(), 2);
    assert!(set.contains(0));
    assert!(set.contains(65535));
    assert!(!set.contains(1));
    assert!(!set.contains(65534));
}

// ===== import edge cases =====
#[test]
fn test_import_with_empty_buffer_and_zero_length() {
    let set = RSet::import(&[], 0);
    assert_eq!(set.cardinality(), 0);
    assert_eq!(set.length(), 4);
}

#[test]
fn test_import_round_trip_bitset() {
    // Build a bitset, export, and re-import.
    let mut set = RSet::new();
    for i in 0u32..5000 {
        assert!(set.add(i as u16));
    }
    let exported = set.export();
    let len = set.length();
    let imported = RSet::import(&exported, len);
    assert!(set.equals(&imported));
    assert_eq!(imported.cardinality(), 5000);
}

#[test]
fn test_import_round_trip_inverted_array() {
    let mut set = RSet::new();
    for i in 0u32..=61440 {
        assert!(set.add(i as u16));
    }
    let exported = set.export();
    let len = set.length();
    let imported = RSet::import(&exported, len);
    assert!(set.equals(&imported));
    assert_eq!(imported.cardinality(), 61441);
}

#[test]
fn test_import_round_trip_full() {
    let mut set = RSet::new();
    set.fill();
    let exported = set.export();
    let len = set.length();
    assert_eq!(len, 2);
    let imported = RSet::import(&exported, len);
    assert_eq!(imported.cardinality(), 65536);
    assert!(set.equals(&imported));
}

// ===== free =====
#[test]
fn test_free() {
    let mut set = rset_new_items(&[1, 2, 3]);
    set.free();
    // After free, cardinality logic depends on state, but the set should be
    // freed and not panic.
}

// ===== invert: array <-> inverted array round trip =====
#[test]
fn test_invert_small_array() {
    let set = rset_new_items(&[10, 20, 30]);
    let mut inverted = RSet::new();
    assert!(set.invert(&mut inverted));
    assert_eq!(inverted.cardinality(), 65533);

    // Verify membership of inverted set
    assert!(!inverted.contains(10));
    assert!(!inverted.contains(20));
    assert!(!inverted.contains(30));
    assert!(inverted.contains(0));
    assert!(inverted.contains(11));
    assert!(inverted.contains(65535));

    // Double invert
    let mut twice = RSet::new();
    assert!(inverted.invert(&mut twice));
    assert!(set.equals(&twice));
    assert_eq!(twice.cardinality(), 3);
}

// ===== length checks for various states =====
#[test]
fn test_length_array() {
    // 5 items → (1+5)*2 = 12 bytes
    let set = rset_new_items(&[1, 2, 3, 4, 5]);
    assert_eq!(set.length(), 12);
}

#[test]
fn test_length_bitset() {
    // Bitset always 2 + 4096*2 = 8194 bytes
    let mut set = RSet::new();
    for i in 0u32..10000 {
        assert!(set.add(i as u16));
    }
    assert_eq!(set.length(), 8194);
}

#[test]
fn test_length_inverted_array() {
    // 62000 items → (65536-62000)+1 = 3537 u16s (1 cardinality + 3536 inverted)
    // length = 2 + 2*(65536-62000) = 2 + 7072 = 7074 bytes
    let mut set = RSet::new();
    for i in 0u32..62000 {
        assert!(set.add(i as u16));
    }
    assert_eq!(set.cardinality(), 62000);
    assert_eq!(set.length(), 7074);
}

fn main() {}
