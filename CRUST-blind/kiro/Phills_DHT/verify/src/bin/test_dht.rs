use Phills_DHT::dht::dht::HASHTABLE;

#[test]
fn uninitialise_test() {
    let ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(!ht.dht_is_initialised());
}

#[test]
fn initialised_test() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    ht.dht_init_table(0, 10, false);
    assert!(ht.dht_is_initialised());
}

#[test]
fn get_size_test_1() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 10, false));
    assert_eq!(ht.dht_get_size(), 10);
}

#[test]
fn get_size_test_2() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 254, false));
    assert_eq!(ht.dht_get_size(), 254);
}

#[test]
fn get_size_test_3() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1, 5, false));
    assert_eq!(ht.dht_get_size(), 4);
}

#[test]
fn get_size_test_4() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(6, 11, false));
    assert_eq!(ht.dht_get_size(), 5);
}

#[test]
fn get_size_test_5() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1065, 1109, false));
    assert_eq!(ht.dht_get_size(), 44);
}

#[test]
fn check_bound_test_1() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1065, 1109, false));
    assert_eq!(ht.dht_get_lower_bound(), 1065);
}

#[test]
fn check_bound_test_2() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(62, 1109, false));
    assert_eq!(ht.dht_get_lower_bound(), 62);
}

#[test]
fn check_bound_test_3() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 1109, false));
    assert_eq!(ht.dht_get_lower_bound(), 0);
}

#[test]
fn check_bound_test_4() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1065, 1109, false));
    assert_eq!(ht.dht_get_upper_bound(), 1109);
}

#[test]
fn check_bound_test_5() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 2, false));
    assert_eq!(ht.dht_get_upper_bound(), 2);
}

#[test]
fn read_write_1() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(14).is_none());
}

#[test]
fn read_write_2() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(0).is_none());
}

#[test]
fn read_write_3() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(1).is_none());
}

#[test]
fn read_write_4() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(2).is_none());
}

#[test]
fn read_write_5() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1, 20, false));
    assert!(ht.dht_read(3).is_none());
}

#[test]
fn read_write_6() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(7).is_none());
}

#[test]
fn read_write_7() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(16).is_none());
}

#[test]
fn read_write_8() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(17).is_none());
}

#[test]
fn read_write_9() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    assert!(ht.dht_read(18).is_none());
}

#[test]
fn read_write_10() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(19, 65);
    assert_eq!(*ht.dht_read(19), Some(65));
}

#[test]
fn read_write_11() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(19, 72);
    assert_eq!(*ht.dht_read(19), Some(72));
}

#[test]
fn read_write_12() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(19, 65);
    assert_eq!(*ht.dht_read(19), Some(65));
}

#[test]
fn write_remap_read_1() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(13, 65);
    assert!(ht.dht_init_table(9, 14, true));
    assert_eq!(*ht.dht_read(13), Some(65));
}

#[test]
fn write_remap_read_2() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(1, 6, false));
    ht.dht_write(5, 101);
    assert!(ht.dht_init_table(2, 14, true));
    assert_eq!(*ht.dht_read(5), Some(101));
}

#[test]
fn write_remap_read_3() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(101, 1600, false));
    ht.dht_write(171, 1);
    assert!(ht.dht_init_table(100, 172, true));
    assert_eq!(*ht.dht_read(171), Some(1));
}

#[test]
fn write_remap_read_4() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(0, 65);
    assert!(ht.dht_init_table(0, 4, true));
    assert_eq!(*ht.dht_read(0), Some(65));
}

#[test]
fn write_remap_read_5() {
    let mut ht: HASHTABLE<i32> = HASHTABLE::dht_init();
    assert!(ht.dht_init_table(0, 20, false));
    ht.dht_write(0, 65);
    assert!(ht.dht_init_table(0, 4, false));
    assert!(ht.dht_read(0).is_none());
}

fn main() {}
