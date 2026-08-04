use Graph_recogniser::cuckoohash::CuckooHashTable;

#[test]
fn test_create_and_find_single_entry() {
    let t = CuckooHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("foo", "bar");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("foo"), Some("bar"));
}

#[test]
fn test_insert_and_find_multiple_size2() {
    // Mirrors test-cuckoohash.c with initial size 2
    let keys: [&'static str; 12] = [
        "stefan", "hristo", "dimitar", "georgi", "stanislav", "nikola",
        "andrei", "iulen", "iasen", "samuele", "henning", "javier",
    ];
    let vals: [&'static str; 12] = [
        "manov", "tenchev", "kajabachev", "popov", "ivanov", "yolov",
        "radev", "dobrev", "bantchev", "carli", "weiler", "martin",
    ];

    let t = CuckooHashTable::new(2);
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
    // Mirrors permut[] in test-cuckoohash.c
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

    let t = CuckooHashTable::new(2);
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
    for &size in &[2usize, 4, 6, 8, 12, 20] {
        let t = CuckooHashTable::new(size);
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
fn test_main_subset_lookup() {
    // Mirrors c_src/bin/main.c; expected "yolov" for "nikola".
    let t = CuckooHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("stefan", "manov");
        g.insert("hristo", "tenchev");
        g.insert("dimitar", "kajabachev");
        g.insert("georgi", "popov");
        g.insert("stanislav", "ivanov");
        g.insert("nikola", "yolov");
        g.insert("andrei", "radev");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("nikola"), Some("yolov"));
    assert_eq!(g.find("stefan"), Some("manov"));
    assert_eq!(g.find("hristo"), Some("tenchev"));
    assert_eq!(g.find("dimitar"), Some("kajabachev"));
    assert_eq!(g.find("georgi"), Some("popov"));
    assert_eq!(g.find("stanislav"), Some("ivanov"));
    assert_eq!(g.find("andrei"), Some("radev"));
}

#[test]
fn test_insert_triggers_resize() {
    // Initial size 2 => half = 1, load factor 1.0; subsequent inserts must resize.
    let t = CuckooHashTable::new(2);
    {
        let mut g = t.write().unwrap();
        g.insert("a", "1");
        g.insert("b", "2");
        g.insert("c", "3");
        g.insert("d", "4");
        g.insert("e", "5");
    }
    let g = t.read().unwrap();
    assert_eq!(g.find("a"), Some("1"));
    assert_eq!(g.find("b"), Some("2"));
    assert_eq!(g.find("c"), Some("3"));
    assert_eq!(g.find("d"), Some("4"));
    assert_eq!(g.find("e"), Some("5"));
}

fn main() {}
