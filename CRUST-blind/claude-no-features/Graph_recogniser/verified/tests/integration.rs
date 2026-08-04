use Graph_recogniser::cuckoohash::CuckooHashTable;
use Graph_recogniser::openhash::OpenHashTable;

const TEST_STRS: &[(&str, &str)] = &[
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

const PERMUT: &[usize] = &[
    10, 0, 4, 3, 5, 3, 7, 11, 4, 11, 6, 0, 1, 8, 5, 1, 10, 3, 5, 2, 9,
];

#[test]
fn test_openhash() {
    let tt = OpenHashTable::new(2);
    {
        let mut t = tt.write().unwrap();
        for (k, d) in TEST_STRS {
            t.insert(k, d);
        }
    }
    {
        let t = tt.read().unwrap();
        for &i in PERMUT {
            let (k, d) = TEST_STRS[i];
            assert_eq!(t.find(k), Some(d));
        }
    }
}

#[test]
fn test_cuckoohash() {
    let tt = CuckooHashTable::new(2);
    {
        let mut t = tt.write().unwrap();
        for (k, d) in TEST_STRS {
            t.insert(k, d);
        }
    }
    {
        let t = tt.read().unwrap();
        for &i in PERMUT {
            let (k, d) = TEST_STRS[i];
            assert_eq!(t.find(k), Some(d));
        }
    }
}
