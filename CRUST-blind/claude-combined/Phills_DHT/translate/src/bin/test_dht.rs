use Phills_DHT::dht::dht::HASHTABLE;

#[test]
fn uninitialise_test() {
    let table: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(!table.dht_is_initialised());
    assert_eq!(table.lower_bound, 0);
    assert_eq!(table.higher_bound, 0);
    assert!(table.hash_table.is_empty());
}

#[test]
fn initialised_test() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 10, false);
    assert!(ok);
    assert!(table.dht_is_initialised());
}

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

#[test]
fn read_write_10() {
    let i: i32 = 65;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, i);
    let read = table.dht_read(19);
    assert_eq!(read, &Some(65));
}

#[test]
fn read_write_11() {
    let i: i32 = 72;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, i);
    let read = table.dht_read(19);
    assert_eq!(read, &Some(72));
}

#[test]
fn read_write_12() {
    let i: i32 = 65;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(19, i);
    let read = table.dht_read(19);
    assert_eq!(read, &Some(65));
}

#[test]
fn write_remap_read_1() {
    let i: i32 = 65;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(13, i);

    let ok = table.dht_init_table(9, 14, true);
    assert!(ok);
    let read = table.dht_read(13);
    assert_eq!(read, &Some(65));
    assert_eq!(table.dht_get_lower_bound(), 9);
    assert_eq!(table.dht_get_upper_bound(), 14);
    assert_eq!(table.dht_get_size(), 5);
}

#[test]
fn write_remap_read_2() {
    let i: i32 = 101;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(1, 6, false);
    assert!(ok);
    table.dht_write(5, i);

    let ok = table.dht_init_table(2, 14, true);
    assert!(ok);
    let read = table.dht_read(5);
    assert_eq!(read, &Some(101));
    assert_eq!(table.dht_get_lower_bound(), 2);
    assert_eq!(table.dht_get_upper_bound(), 14);
    assert_eq!(table.dht_get_size(), 12);
}

#[test]
fn write_remap_read_3() {
    let i: i32 = 1;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(101, 1600, false);
    assert!(ok);
    table.dht_write(171, i);

    let ok = table.dht_init_table(100, 172, true);
    assert!(ok);
    let read = table.dht_read(171);
    assert_eq!(read, &Some(1));
    assert_eq!(table.dht_get_lower_bound(), 100);
    assert_eq!(table.dht_get_upper_bound(), 172);
    assert_eq!(table.dht_get_size(), 72);
}

#[test]
fn write_remap_read_4() {
    let i: i32 = 65;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(0, i);

    let ok = table.dht_init_table(0, 4, true);
    assert!(ok);
    let read = table.dht_read(0);
    assert_eq!(read, &Some(65));
    assert_eq!(table.dht_get_lower_bound(), 0);
    assert_eq!(table.dht_get_upper_bound(), 4);
    assert_eq!(table.dht_get_size(), 4);
}

#[test]
fn write_remap_read_5() {
    let i: i32 = 65;
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    table.dht_write(0, i);

    let ok = table.dht_init_table(0, 4, false);
    assert!(ok);
    let read = table.dht_read(0);
    // migrate=false, so the value should be lost (None)
    assert_eq!(read, &None);
}

#[test]
fn read_write_returns_some_after_write() {
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 20, false);
    assert!(ok);
    assert!(table.dht_read(19).is_none());
    table.dht_write(19, 42);
    let r = table.dht_read(19);
    assert!(r.is_some());
    assert_eq!(r, &Some(42));
}

#[test]
fn write_remap_no_overlap_loses_data() {
    // First fill positions 0..5 with values
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(0, 5, false);
    assert!(ok);
    table.dht_write(0, 10);
    table.dht_write(1, 11);
    table.dht_write(2, 12);

    // Reinit with totally non-overlapping range
    let ok = table.dht_init_table(100, 105, true);
    assert!(ok);
    // The new table should be all None since there's no overlap
    for i in 100..105 {
        assert_eq!(table.dht_read(i), &None);
    }
}

#[test]
fn write_remap_partial_overlap() {
    // Fill positions 5..10 with their position values
    let mut table: HASHTABLE<i32> = HASHTABLE::dht_init();
    let ok = table.dht_init_table(5, 10, false);
    assert!(ok);
    table.dht_write(5, 50);
    table.dht_write(6, 60);
    table.dht_write(7, 70);
    table.dht_write(8, 80);
    table.dht_write(9, 90);

    // Reinit with overlap from 7..12
    let ok = table.dht_init_table(7, 12, true);
    assert!(ok);
    assert_eq!(table.dht_read(7), &Some(70));
    assert_eq!(table.dht_read(8), &Some(80));
    assert_eq!(table.dht_read(9), &Some(90));
    assert_eq!(table.dht_read(10), &None);
    assert_eq!(table.dht_read(11), &None);
}

fn main() {}
