use Graph_recogniser::cuckoohash::CuckooHashTable;

#[test]
fn test_cuckoo_insert_and_find() {
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
fn test_cuckoo_full_test_set() {
    let table = CuckooHashTable::new(2);
    let mut t = table.write().unwrap();

    let test_strs = [
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

    for (k, v) in &test_strs {
        t.insert(k, v);
    }

    let permut = [10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9];
    for &i in &permut {
        assert_eq!(t.find(test_strs[i].0), Some(test_strs[i].1));
    }
}

fn main() {}
