use SimpleXML::simple_vector::Vector;

#[test]
fn test_create() {
    let v: Vector<i32> = Vector::new(8);
    assert_eq!(v.size, 0);
    assert_eq!(v.capacity, 8);
    assert_eq!(v.size(), 0);
}

#[test]
fn test_push_back_basic() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    assert_eq!(v.size(), 2);
    assert_eq!(*v.get_element_at(0).unwrap(), 10);
    assert_eq!(*v.get_element_at(1).unwrap(), 20);
}

#[test]
fn test_push_front_basic() {
    let mut v: Vector<i32> = Vector::new(8);
    let a = 10;
    let b = 20;
    let c = 30;
    v.push_back(a);
    v.push_back(b);
    // [a, b]
    v.push_front(a);
    v.push_front(b);
    v.push_front(c);
    // [c, b, a, a, b]
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), a);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), b);
}

#[test]
fn test_insert_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    let a = 10;
    let b = 20;
    let c = 30;
    // Build [c, b, a, a, b]
    v.push_back(a);
    v.push_back(b);
    v.push_front(a);
    v.push_front(b);
    v.push_front(c);

    v.insert_at_index(c, 2);
    // [c, b, c, a, a, b]
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), c);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), a);
    assert_eq!(*v.get_element_at(5).unwrap(), b);
}

#[test]
fn test_remove_at_index() {
    let mut v: Vector<i32> = Vector::new(8);
    let a = 10;
    let b = 20;
    let c = 30;

    v.push_back(c);
    v.push_back(b);
    v.push_back(c);
    v.push_back(a);
    v.push_back(a);
    v.push_back(b);
    // remove element at index 2
    let removed = v.remove_at_index(2);
    assert_eq!(removed, Some(c));
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), a);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), b);
}

#[test]
fn test_reallocate_many() {
    let mut v: Vector<i32> = Vector::new(8);
    let a = 10;
    let b = 20;
    let c = 30;

    v.push_back(c);
    v.push_back(b);
    v.push_back(a);
    v.push_back(a);
    v.push_back(b);

    for _ in 0..100 {
        v.push_back(a);
    }
    assert_eq!(v.size(), 105);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), a);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), b);
    for i in 5..105 {
        assert_eq!(*v.get_element_at(i).unwrap(), a);
    }

    // top
    assert_eq!(*v.top_back().unwrap(), a);
    assert_eq!(*v.top_front().unwrap(), c);
    assert_eq!(v.size(), 105);
}

#[test]
fn test_pop_back_pop_front() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    assert_eq!(v.pop_back(), Some(3));
    assert_eq!(v.size(), 2);
    assert_eq!(v.pop_front(), Some(1));
    assert_eq!(v.size(), 1);
    assert_eq!(*v.get_element_at(0).unwrap(), 2);
}

#[test]
fn test_pop_empty() {
    let mut v: Vector<i32> = Vector::new(8);
    assert_eq!(v.pop_back(), None);
    assert_eq!(v.pop_front(), None);
}

#[test]
fn test_top_empty() {
    let v: Vector<i32> = Vector::new(8);
    assert!(v.top_back().is_none());
    assert!(v.top_front().is_none());
}

#[test]
fn test_get_out_of_range() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    assert!(v.get_element_at(5).is_none());
}

#[test]
fn test_index_of() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(1);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    assert_eq!(v.index_of(&3), Some(4));
    assert_eq!(v.index_of(&1), Some(0));
    assert_eq!(v.index_of(&99), None);
}

#[test]
fn test_index_of_with_start() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(1);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    assert_eq!(v.index_of_with_start(&1, 0), Some(0));
    assert_eq!(v.index_of_with_start(&1, 1), Some(1));
    assert_eq!(v.index_of_with_start(&1, 2), Some(2));
    assert_eq!(v.index_of_with_start(&1, 3), None);
    assert_eq!(v.index_of_with_start(&3, 0), Some(4));
}

#[test]
fn test_release() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(10);
    v.push_back(20);
    v.release();
    assert_eq!(v.size(), 0);
}

#[test]
fn test_string_vector_large() {
    // Mirror C test_vector2 - 100000 elements with strings
    let mut v: Vector<(String, String)> = Vector::new(8);
    for i in 0..100_000 {
        let tag = format!("tag {}", i);
        let val = format!("value {}", i);
        v.push_back((tag, val));
    }
    assert_eq!(v.size(), 100_000);
    for i in 0..v.size() {
        let (t, val) = v.get_element_at(i).unwrap();
        assert_eq!(t, &format!("tag {}", i));
        assert_eq!(val, &format!("value {}", i));
    }
}

fn main() {}
