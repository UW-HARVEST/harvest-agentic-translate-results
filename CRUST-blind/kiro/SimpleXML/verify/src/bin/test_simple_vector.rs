use SimpleXML::simple_vector::Vector;

#[test]
fn test_create() {
    let v: Vector<i32> = Vector::new(8);
    assert_eq!(v.size(), 0);
    assert_eq!(v.capacity, 8);
}

#[test]
fn test_push_back_and_get() {
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
    v.push_front(10);
    v.push_front(20);
    v.push_front(30);
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);
}

#[test]
fn test_insert_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_front(10);
    v.push_front(20);
    v.push_front(30);
    // v = [30, 20, 10, 10, 20]
    v.insert_at_index(30, 2);
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
    assert_eq!(*v.get_element_at(2).unwrap(), 30);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 10);
    assert_eq!(*v.get_element_at(5).unwrap(), 20);
}

#[test]
fn test_remove_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    for val in [30, 20, 30, 10, 10, 20] {
        v.push_back(val);
    }
    // v = [30, 20, 30, 10, 10, 20], remove index 2
    let removed = v.remove_at_index(2);
    assert_eq!(removed, Some(30));
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);
}

#[test]
fn test_capacity_growth() {
    let mut v: Vector<i32> = Vector::new(8);
    assert_eq!(v.capacity, 8);
    for i in 0..8 {
        v.push_back(i);
    }
    assert_eq!(v.capacity, 8);
    assert_eq!(v.size(), 8);
    // 9th element triggers capacity doubling
    v.push_back(8);
    assert_eq!(v.capacity, 16);
    assert_eq!(v.size(), 9);
}

#[test]
fn test_large_realloc() {
    let mut v: Vector<i32> = Vector::new(8);
    // Start with [30, 20, 10, 10, 20] then push 100 more
    for val in [30, 20, 10, 10, 20] {
        v.push_back(val);
    }
    for _ in 0..100 {
        v.push_back(10);
    }
    assert_eq!(v.size(), 105);
    assert_eq!(*v.get_element_at(0).unwrap(), 30);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 10);
    assert_eq!(*v.get_element_at(4).unwrap(), 20);
    for i in 5..105 {
        assert_eq!(*v.get_element_at(i).unwrap(), 10);
    }
}

#[test]
fn test_top_back_and_front() {
    let mut v: Vector<i32> = Vector::new(8);
    // empty
    assert_eq!(v.top_back(), None);
    assert_eq!(v.top_front(), None);

    v.push_back(30);
    v.push_back(10);
    assert_eq!(*v.top_front().unwrap(), 30);
    assert_eq!(*v.top_back().unwrap(), 10);
    assert_eq!(v.size(), 2); // top doesn't remove
}

#[test]
fn test_pop_back() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(30);
    let popped = v.pop_back();
    assert_eq!(popped, Some(30));
    assert_eq!(v.size(), 2);
}

#[test]
fn test_pop_front() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(30);
    let popped = v.pop_front();
    assert_eq!(popped, Some(10));
    assert_eq!(v.size(), 2);
}

#[test]
fn test_index_of() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(10);
    v.push_back(30);
    assert_eq!(v.index_of(&10), Some(0));
    assert_eq!(v.index_of(&20), Some(1));
    assert_eq!(v.index_of(&30), Some(3));
    assert_eq!(v.index_of(&99), None); // C returns -1, Rust returns None
}

#[test]
fn test_index_of_with_start() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.push_back(10);
    v.push_back(30);
    assert_eq!(v.index_of_with_start(&10, 1), Some(2));
    assert_eq!(v.index_of_with_start(&10, 3), None);
}

#[test]
fn test_get_out_of_bounds() {
    let v: Vector<i32> = Vector::new(8);
    assert_eq!(v.get_element_at(0), None);
    assert_eq!(v.get_element_at(100), None);
}

#[test]
fn test_pop_empty() {
    let mut v: Vector<i32> = Vector::new(8);
    assert_eq!(v.pop_back(), None);
    assert_eq!(v.pop_front(), None);
}

#[test]
fn test_remove_out_of_bounds() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    assert_eq!(v.remove_at_index(5), None);
    assert_eq!(v.size(), 1);
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
fn test_vector_with_strings() {
    let mut v: Vector<String> = Vector::new(8);
    for i in 0..10 {
        v.push_back(format!("tag {}", i));
    }
    for i in 0..10 {
        assert_eq!(v.get_element_at(i).unwrap(), &format!("tag {}", i));
    }
    assert_eq!(v.size(), 10);
}

fn main() {}
