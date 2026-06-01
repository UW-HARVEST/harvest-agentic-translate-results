use Graph_recogniser::openhash::OpenHashTable;

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
fn test_openhash_initial_state() {
    let table = OpenHashTable::new(2);
    let t = table.read().unwrap();
    assert_eq!(t.cur_size, 0);
    assert_eq!(t.max_size, 2);
    assert_eq!(t.arr.len(), 2);
    for entry in t.arr.iter() {
        assert_eq!(entry.key, None);
        assert_eq!(entry.data, None);
    }
}

#[test]
fn test_openhash_single_insert_and_find() {
    let table = OpenHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        t.insert("nikola", "yolov");
    }
    let t = table.read().unwrap();
    assert_eq!(t.find("nikola"), Some("yolov"));
}

#[test]
fn test_openhash_full_set() {
    let table = OpenHashTable::new(2);
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
fn test_openhash_permutation_lookups() {
    // Mirror the C unit-test permutation lookups.
    let permut: &[usize] = &[
        10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9,
    ];
    let table = OpenHashTable::new(2);
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
fn test_openhash_size_grows_after_inserts() {
    let table = OpenHashTable::new(2);
    {
        let mut t = table.write().unwrap();
        for (k, v) in TEST_STRS {
            t.insert(k, v);
        }
    }
    let t = table.read().unwrap();
    assert_eq!(t.cur_size, TEST_STRS.len());
    assert!(t.max_size >= TEST_STRS.len());
}

#[test]
fn test_openhash_free() {
    let table = OpenHashTable::new(4);
    {
        let mut t = table.write().unwrap();
        t.insert("a", "alpha");
        t.insert("b", "beta");
    }
    {
        let mut t = table.write().unwrap();
        t.free_open_hash_table();
        assert_eq!(t.cur_size, 0);
        assert_eq!(t.max_size, 0);
        assert_eq!(t.arr.len(), 0);
    }
}

#[test]
fn test_openhash_pair_main_example() {
    // Mirror the C main.c example with size 2.
    let pairs: &[(&'static str, &'static str)] = &[
        ("stefan", "manov"),
        ("hristo", "tenchev"),
        ("dimitar", "kajabachev"),
        ("georgi", "popov"),
        ("stanislav", "ivanov"),
        ("nikola", "yolov"),
        ("andrei", "radev"),
    ];
    let table = OpenHashTable::new(2);
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
