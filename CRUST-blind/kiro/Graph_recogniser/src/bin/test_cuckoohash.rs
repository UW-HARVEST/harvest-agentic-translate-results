use Graph_recogniser::cuckoohash::CuckooHashTable;

#[test]
fn test_cuckoo_hash_basic() {
    let table = CuckooHashTable::new(10);
    let mut t = table.write().unwrap();
    t.insert("hello", "world");
    assert_eq!(t.find("hello"), Some("world"));
}

#[test]
fn test_cuckoo_hash_multiple_inserts() {
    let table = CuckooHashTable::new(100);
    let mut t = table.write().unwrap();
    t.insert("key1", "val1");
    t.insert("key2", "val2");
    t.insert("key3", "val3");
    assert_eq!(t.find("key1"), Some("val1"));
    assert_eq!(t.find("key2"), Some("val2"));
    assert_eq!(t.find("key3"), Some("val3"));
}

#[test]
fn test_cuckoo_hash_resize() {
    // Start with size 2, insert enough to trigger resize
    let table = CuckooHashTable::new(2);
    let mut t = table.write().unwrap();
    t.insert("stefan", "manov");
    t.insert("hristo", "tenchev");
    t.insert("dimitar", "kajabachev");
    t.insert("georgi", "popov");
    t.insert("stanislav", "ivanov");
    t.insert("nikola", "yolov");
    t.insert("andrei", "radev");

    assert_eq!(t.find("stefan"), Some("manov"));
    assert_eq!(t.find("hristo"), Some("tenchev"));
    assert_eq!(t.find("dimitar"), Some("kajabachev"));
    assert_eq!(t.find("georgi"), Some("popov"));
    assert_eq!(t.find("stanislav"), Some("ivanov"));
    assert_eq!(t.find("nikola"), Some("yolov"));
    assert_eq!(t.find("andrei"), Some("radev"));
}

#[test]
fn test_cuckoo_hash_full_test_suite() {
    let test_strs: Vec<(&str, &str)> = vec![
        ("stefan", "manov"),
        ("hristo", "tenchev"),
        ("dimitar", "kajabachev"),
        ("georgi", "popov"),
        ("stanislav", "ivanov"),
        ("nikola", "yolov"),
        ("andrei", "radev"),
        ("iulen", "dobrev"),
        ("iasen", "bantchev"),
        ("samuele", "carli"),
        ("henning", "weiler"),
        ("javier", "martin"),
    ];
    let permut = [10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9];

    let table = CuckooHashTable::new(2);
    let mut t = table.write().unwrap();
    for &(k, v) in &test_strs {
        t.insert(k, v);
    }
    for &p in &permut {
        let (key, expected) = test_strs[p];
        assert_eq!(t.find(key), Some(expected), "Failed for key={}", key);
    }
}

#[test]
fn test_cuckoo_hash_find_missing() {
    // With an empty table, find should look in second_arr and return its data (None)
    let table = CuckooHashTable::new(10);
    let t = table.read().unwrap();
    assert_eq!(t.find("nonexistent"), None);
}

fn main() {}
