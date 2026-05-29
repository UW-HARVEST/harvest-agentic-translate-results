#[allow(unused_imports)]
use Phills_DHT::dht::dht::HASHTABLE;

// === Test: uninitialise_test ===
// Mirrors the C uninitialise_test: a fresh init() should report not initialised.
#[test]
fn uninitialise_test() {
    let table: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(!table.dht_is_initialised());
    assert_eq!(table.lower_bound, 0);
    assert_eq!(table.higher_bound, 0);
    assert!(table.hash_table.is_empty());
}

// === Test: initialised_test ===
// After dht_init_table(0, 10, false), is_initialised should return true.
#[test]
fn initialised_test() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 10, false);
    assert!(ok);
    assert!(table.dht_is_initialised());
}

// === Test: get_size_test_1..5 ===
#[test]
fn get_size_test_1() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 10, false);
    assert!(ok);
    assert_eq!(table.dht_get_size(), 10);
}

#[test]
fn get_size_test_2() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 254, false);
    assert!(ok);
    assert_eq!(table.dht_get_size(), 254);
}

#[test]
fn get_size_test_3() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1, 5, false);
    assert!(ok);
    assert_eq!(table.dht_get_size(), 4);
}

#[test]
fn get_size_test_4() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(6, 11, false);
    assert!(ok);
    assert_eq!(table.dht_get_size(), 5);
}

#[test]
fn get_size_test_5() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1065, 1109, false);
    assert!(ok);
    assert_eq!(table.dht_get_size(), 44);
}

// === Test: check_bound_test_1..5 ===
#[test]
fn check_bound_test_1() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1065, 1109, false);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 1065);
}

#[test]
fn check_bound_test_2() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(62, 1109, false);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 62);
}

#[test]
fn check_bound_test_3() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 1109, false);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 0);
}

#[test]
fn check_bound_test_4() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1065, 1109, false);
    assert!(ok);
    assert_eq!(table.dht_get_upper_bound(), 1109);
}

#[test]
fn check_bound_test_5() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 2, false);
    assert!(ok);
    assert_eq!(table.dht_get_upper_bound(), 2);
}

// === Test: read_write_1..9 (read uninitialised slots) ===
#[test]
fn read_write_1() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(14).is_none());
}

#[test]
fn read_write_2() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(0).is_none());
}

#[test]
fn read_write_3() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(1).is_none());
}

#[test]
fn read_write_4() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(2).is_none());
}

#[test]
fn read_write_5() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1, 20, false);
    assert!(ok);
    assert!(table.dht_read(3).is_none());
}

#[test]
fn read_write_6() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(7).is_none());
}

#[test]
fn read_write_7() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(16).is_none());
}

#[test]
fn read_write_8() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(17).is_none());
}

#[test]
fn read_write_9() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(18).is_none());
}

// === Test: read_write_10..12 (write then read) ===
#[test]
fn read_write_10() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, 65);
    let val = table.dht_read(19);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &65);
}

#[test]
fn read_write_11() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, 72);
    let val = table.dht_read(19);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &72);
}

#[test]
fn read_write_12() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, 65);
    let val = table.dht_read(19);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &65);
}

// === Test: write_remap_read_1..5 (migration semantics) ===
#[test]
fn write_remap_read_1() {
    // Write at 13 in [0,20), then re-init to [9,14) with migrate=true.
    // Position 13 lies in the new range, so it must persist.
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(13, 65);
    let ok = table.dht_init_table(9, 14, true);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 9);
    assert_eq!(table.dht_get_upper_bound(), 14);
    assert_eq!(table.dht_get_size(), 5);
    let val = table.dht_read(13);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &65);
}

#[test]
fn write_remap_read_2() {
    // Write at 5 in [1,6), then re-init to [2,14) with migrate=true.
    // Position 5 lies in [2,14), so the value must persist.
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1, 6, false);
    assert!(ok);
    table.dht_write(5, 101);
    let ok = table.dht_init_table(2, 14, true);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 2);
    assert_eq!(table.dht_get_upper_bound(), 14);
    let val = table.dht_read(5);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &101);
}

#[test]
fn write_remap_read_3() {
    // Write at 171 in [101,1600), then re-init to [100,172) with migrate=true.
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(101, 1600, false);
    assert!(ok);
    table.dht_write(171, 1);
    let ok = table.dht_init_table(100, 172, true);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 100);
    assert_eq!(table.dht_get_upper_bound(), 172);
    let val = table.dht_read(171);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &1);
}

#[test]
fn write_remap_read_4() {
    // Write at 0 in [0,20), then re-init to [0,4) with migrate=true.
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(0, 65);
    let ok = table.dht_init_table(0, 4, true);
    assert!(ok);
    assert_eq!(table.dht_get_lower_bound(), 0);
    assert_eq!(table.dht_get_upper_bound(), 4);
    let val = table.dht_read(0);
    assert!(val.is_some());
    assert_eq!(val.as_ref().unwrap(), &65);
}

#[test]
fn write_remap_read_5() {
    // Write at 0 in [0,20), then re-init to [0,4) WITHOUT migration.
    // The old data must be discarded and the slot must be None.
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(0, 65);
    let ok = table.dht_init_table(0, 4, false);
    assert!(ok);
    assert!(table.dht_read(0).is_none());
}

// === Additional edge cases beyond the original C suite ===

// Migration where some entries fall outside the new range. They must be dropped.
#[test]
fn migration_drops_out_of_range_entries() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(0, 20, false);
    table.dht_write(2, 200);
    table.dht_write(10, 1000);
    table.dht_write(15, 1500);
    // New range [5,12). 2 falls out, 10 stays, 15 falls out.
    table.dht_init_table(5, 12, true);
    assert!(table.dht_read(5).is_none());
    assert_eq!(table.dht_read(10).as_ref().unwrap(), &1000);
    assert!(table.dht_read(11).is_none());
}

// dht_write should overwrite an existing value at the same location.
#[test]
fn write_overwrites_existing() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(0, 5, false);
    table.dht_write(2, 100);
    assert_eq!(table.dht_read(2).as_ref().unwrap(), &100);
    table.dht_write(2, 999);
    assert_eq!(table.dht_read(2).as_ref().unwrap(), &999);
}

// Reading every untouched slot of a freshly initialised table should yield None.
#[test]
fn fresh_table_all_none() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(7, 13, false);
    for loc in 7..13u32 {
        assert!(table.dht_read(loc).is_none(), "slot {} should be None", loc);
    }
}

// Re-initialising with migrate=false discards every existing entry, even ones
// whose absolute position would still fall in the new range.
#[test]
fn no_migrate_drops_all() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(0, 10, false);
    table.dht_write(3, 333);
    table.dht_write(5, 555);
    table.dht_init_table(0, 10, false);
    assert!(table.dht_read(3).is_none());
    assert!(table.dht_read(5).is_none());
}

// Reading the upper-most valid index should work.
#[test]
fn read_upper_index() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(10, 20, false);
    table.dht_write(19, 42);
    assert_eq!(table.dht_read(19).as_ref().unwrap(), &42);
    assert_eq!(table.dht_get_size(), 10);
}

// Migration that grows the table preserves entries in the overlap.
#[test]
fn migration_grows_table_preserves_entries() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    table.dht_init_table(0, 5, false);
    table.dht_write(0, 10);
    table.dht_write(4, 40);
    table.dht_init_table(0, 100, true);
    assert_eq!(table.dht_get_size(), 100);
    assert_eq!(table.dht_read(0).as_ref().unwrap(), &10);
    assert_eq!(table.dht_read(4).as_ref().unwrap(), &40);
    assert!(table.dht_read(50).is_none());
    assert!(table.dht_read(99).is_none());
}

fn main() {}
