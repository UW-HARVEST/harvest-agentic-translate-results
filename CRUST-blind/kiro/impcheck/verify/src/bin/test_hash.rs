use impcheck::hash::HTable;

#[test]
fn test_htable_new() {
    let ht = HTable::new(4);
    assert_eq!(ht.size, 0);
    assert_eq!(ht.capacity, 16);
    assert_eq!(ht.max_size, 8);
    assert_eq!(ht.growth_factor, 2.0);
    assert_eq!(ht.last_found_idx, 0);
}

#[test]
fn test_insert_and_find() {
    let mut ht = HTable::new(4);
    let ok = ht.insert(1, Box::new(42i32));
    assert!(ok);
    assert_eq!(ht.size, 1);

    let val = ht.find(1);
    assert!(val.is_some());
    assert_eq!(*val.unwrap().downcast_ref::<i32>().unwrap(), 42);
}

#[test]
fn test_insert_key_zero_fails() {
    let mut ht = HTable::new(4);
    let ok = ht.insert(0, Box::new(1i32));
    assert!(!ok);
    assert_eq!(ht.size, 0);
}

#[test]
fn test_insert_duplicate_fails() {
    let mut ht = HTable::new(4);
    assert!(ht.insert(1, Box::new(10i32)));
    let ok = ht.insert(1, Box::new(20i32));
    assert!(!ok);
    assert_eq!(ht.size, 1);
}

#[test]
fn test_find_nonexistent() {
    let mut ht = HTable::new(4);
    ht.insert(1, Box::new(10i32));
    let val = ht.find(99);
    assert!(val.is_none());
}

#[test]
fn test_delete() {
    let mut ht = HTable::new(4);
    ht.insert(1, Box::new(10i32));
    ht.insert(2, Box::new(20i32));
    assert_eq!(ht.size, 2);

    let ok = ht.delete(1);
    assert!(ok);
    assert_eq!(ht.size, 1);
    assert!(ht.find(1).is_none());
    assert!(ht.find(2).is_some());
}

#[test]
fn test_delete_nonexistent() {
    let mut ht = HTable::new(4);
    ht.insert(1, Box::new(10i32));
    let ok = ht.delete(99);
    assert!(!ok);
    assert_eq!(ht.size, 1);
}

#[test]
fn test_delete_last_found() {
    let mut ht = HTable::new(4);
    ht.insert(10, Box::new(100i32));
    ht.insert(20, Box::new(200i32));

    let val = ht.find(10);
    assert!(val.is_some());
    let ok = ht.delete_last_found();
    assert!(ok);
    assert_eq!(ht.size, 1);
    assert!(ht.find(10).is_none());
    assert!(ht.find(20).is_some());
}

#[test]
fn test_growth() {
    // capacity=4, max_size=2, so inserting 3rd element triggers growth
    let mut ht = HTable::new(2);
    assert_eq!(ht.capacity, 4);
    assert_eq!(ht.max_size, 2);

    assert!(ht.insert(1, Box::new(1i32)));
    assert_eq!(ht.size, 1);
    assert_eq!(ht.capacity, 4);

    assert!(ht.insert(2, Box::new(2i32)));
    assert_eq!(ht.size, 2);
    assert_eq!(ht.capacity, 4);

    assert!(ht.insert(3, Box::new(3i32)));
    assert_eq!(ht.size, 3);
    assert_eq!(ht.capacity, 8);

    // All values still findable after growth
    for i in 1u64..=3 {
        let val = ht.find(i);
        assert!(val.is_some());
        assert_eq!(*val.unwrap().downcast_ref::<i32>().unwrap(), i as i32);
    }
}

#[test]
fn test_many_inserts_and_deletes() {
    let mut ht = HTable::new(7);
    assert_eq!(ht.capacity, 128);

    for i in 1u64..=64 {
        assert!(ht.insert(i, Box::new(i as i32)));
    }
    assert_eq!(ht.size, 64);

    // After inserting 65, growth should have occurred (max_size was 64)
    assert!(ht.insert(65, Box::new(65i32)));
    assert_eq!(ht.size, 65);
    assert_eq!(ht.capacity, 256);

    // Delete all except 40
    for i in 1u64..=65 {
        if i == 40 { continue; }
        assert!(ht.find(i).is_some());
        assert!(ht.delete(i));
        assert!(ht.find(i).is_none());
    }
    assert_eq!(ht.size, 1);
    assert!(ht.find(40).is_some());
    assert!(ht.delete_last_found());
    assert_eq!(ht.size, 0);
    assert!(ht.find(40).is_none());
}

#[test]
fn test_find_mut() {
    let mut ht = HTable::new(4);
    ht.insert(5, Box::new(50i32));
    {
        let val = ht.find_mut(5);
        assert!(val.is_some());
        let boxed = val.unwrap();
        *boxed = Box::new(99i32);
    }
    let val = ht.find(5);
    assert_eq!(*val.unwrap().downcast_ref::<i32>().unwrap(), 99);
}

fn main() {}
