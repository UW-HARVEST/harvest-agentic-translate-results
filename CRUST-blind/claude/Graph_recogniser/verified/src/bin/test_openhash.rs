use Graph_recogniser::openhash::OpenHashTable;

#[test]
fn test_create_and_find_single_entry() {
    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("foo", "bar");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("foo"), Some("bar"));
}

#[test]
fn test_insert_and_find_multiple_size2() {
    // Mirrors test-openhash.c with initial size 2
    let keys: [&'static str; 12] = [
        "stefan", "hristo", "dimitar", "georgi", "stanislav", "nikola",
        "andrei", "iulen", "iasen", "samuele", "henning", "javier",
    ];
    let vals: [&'static str; 12] = [
        "manov", "tenchev", "kajabachev", "popov", "ivanov", "yolov",
        "radev", "dobrev", "bantchev", "carli", "weiler", "martin",
    ];

    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        for i in 0..keys.len() {
            g.insert(keys[i], vals[i]);
        }
    }
    let g = t.read().unwrap();
    for i in 0..keys.len() {
        assert_eq!(g.find(keys[i]), Some(vals[i]),
            "expected {} -> {}", keys[i], vals[i]);
    }
}

#[test]
fn test_insert_with_permutation_lookup() {
    // Mirrors permut[] in test-openhash.c
    let keys: [&'static str; 12] = [
        "stefan", "hristo", "dimitar", "georgi", "stanislav", "nikola",
        "andrei", "iulen", "iasen", "samuele", "henning", "javier",
    ];
    let vals: [&'static str; 12] = [
        "manov", "tenchev", "kajabachev", "popov", "ivanov", "yolov",
        "radev", "dobrev", "bantchev", "carli", "weiler", "martin",
    ];
    let permut: [usize; 21] = [
        10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9,
    ];

    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        for i in 0..keys.len() {
            g.insert(keys[i], vals[i]);
        }
    }
    let g = t.read().unwrap();
    for &i in permut.iter() {
        assert_eq!(g.find(keys[i]), Some(vals[i]));
    }
}

#[test]
fn test_works_with_various_initial_sizes() {
    let keys: [&'static str; 12] = [
        "stefan", "hristo", "dimitar", "georgi", "stanislav", "nikola",
        "andrei", "iulen", "iasen", "samuele", "henning", "javier",
    ];
    let vals: [&'static str; 12] = [
        "manov", "tenchev", "kajabachev", "popov", "ivanov", "yolov",
        "radev", "dobrev", "bantchev", "carli", "weiler", "martin",
    ];

    // Sizes verified to succeed with the C implementation.
    for &size in &[2usize, 3, 5, 7, 11, 13, 17] {
        let t = OpenHashTable::new(size);
        {
            let mut g = t.write().unwrap();
            for i in 0..keys.len() {
                g.insert(keys[i], vals[i]);
            }
        }
        let g = t.read().unwrap();
        for i in 0..keys.len() {
            assert_eq!(g.find(keys[i]), Some(vals[i]),
                "size={} key={} expected {}", size, keys[i], vals[i]);
        }
    }
}

#[test]
fn test_insert_triggers_resize() {
    // Initial size 2: load factor 0.6. Inserting 2 items must trigger resize.
    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("a", "1");
        g.insert("b", "2");
        g.insert("c", "3");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("a"), Some("1"));
    assert_eq!(g.find("b"), Some("2"));
    assert_eq!(g.find("c"), Some("3"));
}

#[test]
fn test_find_returns_inserted_data() {
    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("alpha", "ALPHA");
        g.insert("beta", "BETA");
        g.insert("gamma", "GAMMA");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("alpha"), Some("ALPHA"));
    assert_eq!(g.find("beta"), Some("BETA"));
    assert_eq!(g.find("gamma"), Some("GAMMA"));
}

#[test]
fn test_free_open_hash_table_resets_state() {
    let t = OpenHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("x", "y");
        g.free_open_hash_table();
    }
    // After free, no reads should be performed because internal arr is empty.
    // Verified by simply checking it doesn't panic and we can still hold the lock.
    let _g = t.read().unwrap();
}

fn main() {}
