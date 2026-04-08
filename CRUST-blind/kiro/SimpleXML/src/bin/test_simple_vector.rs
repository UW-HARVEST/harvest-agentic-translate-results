use SimpleXML::simple_vector::Vector;

#[test]
fn test_create() {
    let v: Vector<i32> = Vector::new(8);
    assert_eq!(v.size(), 0);
    assert_eq!(v.capacity, 8);
}

#[test]
fn test_push_back_and_size() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    assert_eq!(v.size(), 2);
    assert_eq!(*v.get_element_at(0).unwrap(), 10);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
}

#[test]
fn test_push_front() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_front(30);
    v.push_front(40);
    v.push_front(50);
    // v = [50, 40, 30, 10, 20]
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 50);
    assert_eq!(*v.get_element_at(1).unwrap(), 40);
    assert_eq!(*v.get_element_at(2).unwrap(), 30);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);
}

#[test]
fn test_insert_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    // v = [1, 2, 3]
    v.insert_at_index(5, 0);
    // v = [5, 1, 2, 3]
    assert_eq!(*v.get_element_at(0).unwrap(), 5);
    v.insert_at_index(10, 2);
    // v = [5, 1, 10, 2, 3]
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    v.insert_at_index(11, 5);
    // v = [5, 1, 10, 2, 3, 11]
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(5).unwrap(), 11);
}

#[test]
fn test_remove_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(50);
    v.push_back(40);
    v.push_back(30);
    v.push_back(10);
    v.push_back(20);
    // insert at 2 then remove at 2
    v.insert_at_index(99, 2);
    assert_eq!(v.size(), 6);
    let removed = v.remove_at_index(2);
    assert_eq!(removed, Some(99));
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 50);
    assert_eq!(*v.get_element_at(1).unwrap(), 40);
    assert_eq!(*v.get_element_at(2).unwrap(), 30);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);
}

#[test]
fn test_pop_back() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    let popped = v.pop_back();
    assert_eq!(popped, Some(3));
    assert_eq!(v.size(), 2);
}

#[test]
fn test_pop_front() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    let popped = v.pop_front();
    assert_eq!(popped, Some(1));
    assert_eq!(v.size(), 2);
    assert_eq!(*v.get_element_at(0).unwrap(), 2);
}

#[test]
fn test_top_back() {
    let mut v: Vector<i32> = Vector::new(8);
    assert!(v.top_back().is_none());
    v.push_back(10);
    v.push_back(20);
    assert_eq!(*v.top_back().unwrap(), 20);
    assert_eq!(v.size(), 2); // top doesn't remove
}

#[test]
fn test_top_front() {
    let mut v: Vector<i32> = Vector::new(8);
    assert!(v.top_front().is_none());
    v.push_back(10);
    v.push_back(20);
    assert_eq!(*v.top_front().unwrap(), 10);
    assert_eq!(v.size(), 2); // top doesn't remove
}

#[test]
fn test_pop_empty() {
    let mut v: Vector<i32> = Vector::new(8);
    assert_eq!(v.pop_back(), None);
    assert_eq!(v.pop_front(), None);
}

#[test]
fn test_get_element_out_of_range() {
    let v: Vector<i32> = Vector::new(8);
    assert!(v.get_element_at(0).is_none());
    assert!(v.get_element_at(100).is_none());
}

#[test]
fn test_remove_out_of_range() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    assert_eq!(v.remove_at_index(5), None);
    assert_eq!(v.size(), 1);
}

#[test]
fn test_reallocation() {
    let mut v: Vector<i32> = Vector::new(8);
    for i in 0..100 {
        v.push_back(i);
    }
    assert_eq!(v.size(), 100);
    for i in 0..100 {
        assert_eq!(*v.get_element_at(i).unwrap(), i as i32);
    }
    assert!(v.capacity >= 100);
}

#[test]
fn test_index_of_pointer_comparison() {
    // index_of uses pointer comparison (std::ptr::eq), so it compares
    // references to elements stored in the vector, not values
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(10); // same value, different slot

    // Searching with a reference to the actual stored element should find it
    let found = v.index_of(v.get_element_at(0).unwrap());
    assert_eq!(found, Some(0));

    let found = v.index_of(v.get_element_at(1).unwrap());
    assert_eq!(found, Some(1));

    // Searching with an external reference should NOT find it (different pointer)
    let external = 10;
    let not_found = v.index_of(&external);
    assert_eq!(not_found, None);
}

#[test]
fn test_index_of_with_start() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(30);

    // Get pointer to element at index 0, search from start=0
    let elem0 = v.get_element_at(0).unwrap() as *const i32;
    let found = v.index_of_with_start(unsafe { &*elem0 }, 0);
    assert_eq!(found, Some(0));

    // Search from start=1 should not find element at index 0
    let found = v.index_of_with_start(unsafe { &*elem0 }, 1);
    assert_eq!(found, None);
}

#[test]
fn test_release() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.release();
    assert_eq!(v.size(), 0);
}

#[test]
fn test_full_c_test_sequence() {
    // Replicate the C test_vector() sequence
    let mut v: Vector<i32> = Vector::new(8);
    assert_eq!(v.size(), 0);
    assert_eq!(v.capacity, 8);

    v.push_back(10);
    v.push_back(20);
    assert_eq!(v.size(), 2);
    assert_eq!(*v.get_element_at(0).unwrap(), 10);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);

    v.push_front(10);
    v.push_front(20);
    v.push_front(30);
    // v = [30, 20, 10, 10, 20]
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);

    v.insert_at_index(30, 2);
    // v = [30, 20, 30, 10, 10, 20]
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(2).unwrap(), 30);

    v.remove_at_index(2);
    // v = [30, 20, 10, 10, 20]
    assert_eq!(v.size(), 5);

    // Reallocation: push 100 more
    for _ in 0..100 {
        v.push_back(10);
    }
    assert_eq!(v.size(), 105);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);

    // top doesn't change size
    assert_eq!(*v.top_back().unwrap(), 10);
    assert_eq!(*v.top_front().unwrap(), 30);
    assert_eq!(v.size(), 105);
}

fn main() {}
