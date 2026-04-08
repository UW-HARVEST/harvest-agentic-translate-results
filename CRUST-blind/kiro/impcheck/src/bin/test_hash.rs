use impcheck::hash::{SimpleHashTable, HashTable};

#[test]
fn test_compute_hash_values() {
    // Ground truth from C: compute_hash(key) = (0xcbf29ce484222325 ^ key) * 0x00000100000001B3
    assert_eq!(HashTable::<i32>::compute_hash(0), 12638153115695167455);
    assert_eq!(HashTable::<i32>::compute_hash(1), 12638152016183539244);
    assert_eq!(HashTable::<i32>::compute_hash(42), 12638128926439346813);
    assert_eq!(HashTable::<i32>::compute_hash(1000000), 13448308467733135007);
}

#[test]
fn test_simple_hash_table_init() {
    let ht = SimpleHashTable::new(4);
    assert_eq!(ht.size, 0);
    assert_eq!(ht.capacity, 16);
}

#[test]
fn test_simple_hash_table_insert_and_find() {
    let mut ht = SimpleHashTable::new(4);
    assert!(ht.hash_table_insert(1, Box::new(42i32)));
    assert_eq!(ht.size, 1);
    assert!(ht.hash_table_insert(2, Box::new(99i32)));
    assert_eq!(ht.size, 2);

    let found = ht.hash_table_find(1);
    assert!(found.is_some());
    assert_eq!(*found.unwrap().downcast_ref::<i32>().unwrap(), 42);

    let found2 = ht.hash_table_find(2);
    assert!(found2.is_some());
    assert_eq!(*found2.unwrap().downcast_ref::<i32>().unwrap(), 99);
}

#[test]
fn test_simple_hash_table_insert_key_zero_rejected() {
    let mut ht = SimpleHashTable::new(4);
    assert!(!ht.hash_table_insert(0, Box::new(1i32)));
}

#[test]
fn test_simple_hash_table_insert_duplicate_rejected() {
    let mut ht = SimpleHashTable::new(4);
    assert!(ht.hash_table_insert(1, Box::new(42i32)));
    assert!(!ht.hash_table_insert(1, Box::new(99i32)));
}

#[test]
fn test_simple_hash_table_find_missing() {
    let mut ht = SimpleHashTable::new(4);
    assert!(ht.hash_table_find(99).is_none());
}

#[test]
fn test_simple_hash_table_delete() {
    let mut ht = SimpleHashTable::new(4);
    assert!(ht.hash_table_insert(1, Box::new(42i32)));
    assert!(ht.hash_table_insert(2, Box::new(99i32)));
    assert!(ht.hash_table_delete(1));
    assert_eq!(ht.size, 1);
    assert!(ht.hash_table_find(1).is_none());
    assert!(ht.hash_table_find(2).is_some());
}

#[test]
fn test_simple_hash_table_delete_last_found() {
    let mut ht = SimpleHashTable::new(4);
    assert!(ht.hash_table_insert(40, Box::new(100i32)));
    let _ = ht.hash_table_find(40);
    assert!(ht.hash_table_delete_last_found());
    assert_eq!(ht.size, 0);
    assert!(ht.hash_table_find(40).is_none());
}

#[test]
fn test_simple_hash_table_growth() {
    let mut ht = SimpleHashTable::new(4); // capacity=16, max_size=8
    for i in 1..=8 {
        assert!(ht.hash_table_insert(i, Box::new(i as i32)));
    }
    assert_eq!(ht.size, 8);
    // Inserting 9th should trigger growth
    assert!(ht.hash_table_insert(9, Box::new(9i32)));
    assert_eq!(ht.size, 9);
    assert!(ht.capacity > 16);
    // All elements still findable
    for i in 1..=9 {
        assert!(ht.hash_table_find(i).is_some());
    }
}

#[test]
fn test_simple_hash_table_many_insert_delete() {
    let mut ht = SimpleHashTable::new(7);
    for i in 1..=65u64 {
        assert!(ht.hash_table_insert(i, Box::new(i as i32)));
    }
    assert_eq!(ht.size, 65);
    for i in 1..=65u64 {
        if i == 40 { continue; }
        assert!(ht.hash_table_find(i).is_some());
        assert!(ht.hash_table_delete(i));
        assert!(ht.hash_table_find(i).is_none());
    }
    assert_eq!(ht.size, 1);
    assert!(ht.hash_table_find(40).is_some());
    assert!(ht.hash_table_delete_last_found());
    assert_eq!(ht.size, 0);
}

#[test]
fn test_simple_hash_table_delete_nonexistent() {
    let mut ht = SimpleHashTable::new(4);
    assert!(!ht.hash_table_delete(42));
}

#[test]
fn test_simple_hash_table_big_insert_delete() {
    let mut ht = SimpleHashTable::new(7);
    let nb_elems = 1000u64;
    for i in 1..=nb_elems {
        assert!(ht.hash_table_find(i).is_none());
        assert!(ht.hash_table_insert(i, Box::new(i as i32)));
        assert!(ht.hash_table_find(i).is_some());
    }
    assert_eq!(ht.size, nb_elems);
    for i in 1..=nb_elems {
        assert!(ht.hash_table_find(i).is_some());
        assert!(ht.hash_table_delete_last_found());
        assert!(ht.hash_table_find(i).is_none());
    }
    assert_eq!(ht.size, 0);
}

#[test]
fn test_simple_hash_table_alternating_insert_delete() {
    let mut ht = SimpleHashTable::new(7);
    let block_size = 64;
    let nb_iterations = 10;
    let mut counter = 1u64;
    for _ in 0..nb_iterations {
        for _ in 0..block_size {
            assert!(ht.hash_table_insert(counter, Box::new(counter as i32)));
            counter += 1;
        }
        let mut delcounter = counter - block_size;
        while delcounter < counter {
            assert!(ht.hash_table_find(delcounter).is_some());
            assert!(ht.hash_table_delete_last_found());
            delcounter += 2;
        }
    }
    // Check: odd keys (1-indexed from each block start) deleted, even keys present
    for i in 1..counter {
        let block_start = ((i - 1) / block_size as u64) * block_size as u64 + 1;
        let offset = i - block_start;
        if offset % 2 == 0 {
            // These were deleted (delcounter starts at block_start, steps by 2)
            assert!(ht.hash_table_find(i).is_none());
        } else {
            assert!(ht.hash_table_find(i).is_some());
        }
    }
}

fn main() {}
