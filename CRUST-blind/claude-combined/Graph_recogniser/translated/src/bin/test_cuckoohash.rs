use Graph_recogniser::cuckoohash::CuckooHashTable;

const TEST_STRS: &[(&'static str, &'static str)] = &[
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

#[test]
fn test_cuckoo_initial_state() {
    let table = CuckooHashTable::new(2);
    let t = table.read().unwrap();
    assert_eq!(t.cur_size, 0);
    assert_eq!(t.cur_marker, 0);
    // initial_size 2 means each array holds 1 entry (initial_size / 2)
    assert_eq!(t.max_size, 1);
    assert_eq!(t.first_arr.len(), 1);
    assert_eq!(t.second_arr.len(), 1);
    for entry in t.first_arr.iter() {
        assert_eq!(entry.key, None);
        assert_eq!(entry.data, None);
        assert_eq!(entry.marker, 0);
    }
    for entry in t.second_arr.iter() {
        assert_eq!(entry.key, None);
        assert_eq!(entry.data, None);
        assert_eq!(entry.marker, 0);
    }
}

#[test]
fn test_cuckoo_single_insert_and_find() {
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        t.insert("nikola", "yolov");
    }
    let t = table.read().unwrap();
    assert_eq!(t.find("nikola"), Some("yolov"));
}

#[test]
fn test_cuckoo_find_missing_returns_none() {
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        t.insert("alice", "wonder");
    }
    let t = table.read().unwrap();
    assert_eq!(t.find("bob"), None);
}

#[test]
fn test_cuckoo_full_set() {
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        for (k, v) in TEST_STRS {
            t.insert(k, v);
        }
    }
    let t = table.read().unwrap();
    for (k, v) in TEST_STRS {
        assert_eq!(t.find(k), Some(*v));
    }
}

#[test]
fn test_cuckoo_permutation_lookups() {
    let permut: &[usize] = &[
        10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9,
    ];
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        for (k, v) in TEST_STRS {
            t.insert(k, v);
        }
    }
    let t = table.read().unwrap();
    for &i in permut {
        let (k, v) = TEST_STRS[i];
        assert_eq!(t.find(k), Some(v));
    }
}

#[test]
fn test_cuckoo_size_grows_after_inserts() {
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        for (k, v) in TEST_STRS {
            t.insert(k, v);
        }
    }
    let t = table.read().unwrap();
    assert_eq!(t.cur_size as usize, TEST_STRS.len());
    assert!(t.max_size >= TEST_STRS.len() as u32);
}

#[test]
fn test_cuckoo_main_example() {
    let pairs: &[(&'static str, &'static str)] = &[
        ("stefan", "manov"),
        ("hristo", "tenchev"),
        ("dimitar", "kajabachev"),
        ("georgi", "popov"),
        ("stanislav", "ivanov"),
        ("nikola", "yolov"),
        ("andrei", "radev"),
    ];
    let table = CuckooHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        for (k, v) in pairs {
            t.insert(k, v);
        }
    }
    let t = table.read().unwrap();
    assert_eq!(t.find("nikola"), Some("yolov"));
    assert_eq!(t.find("stefan"), Some("manov"));
    assert_eq!(t.find("andrei"), Some("radev"));
}

fn main() {}
