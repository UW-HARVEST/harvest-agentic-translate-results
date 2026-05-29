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
fn test_push_front() {
    // Mirror C test_vector():
    // After push_back(a), push_back(b), push_front(a), push_front(b), push_front(c)
    // expect: [c, b, a, a, b]
    let mut v: Vector<i32> = Vector::new(8);
    let a: i32 = 10;
    let b: i32 = 20;
    let c: i32 = 30;
    v.push_back(a);
    v.push_back(b);
    v.push_front(a);
    v.push_front(b);
    v.push_front(c);
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), a);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), b);
}

#[test]
fn test_insert_at_index() {
    // Start with [c, b, a, a, b], then insert c at index 2
    // expect: [c, b, c, a, a, b]
    let mut v: Vector<i32> = Vector::new(8);
    let a: i32 = 10;
    let b: i32 = 20;
    let c: i32 = 30;
    v.push_back(c);
    v.push_back(b);
    v.push_back(a);
    v.push_back(a);
    v.push_back(b);

    v.insert_at_index(c, 2);
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(0).unwrap(), c);
    assert_eq!(*v.get_element_at(1).unwrap(), b);
    assert_eq!(*v.get_element_at(2).unwrap(), c);
    assert_eq!(*v.get_element_at(3).unwrap(), a);
    assert_eq!(*v.get_element_at(4).unwrap(), a);
    assert_eq!(*v.get_element_at(5).unwrap(), b);
}

#[test]
fn test_insert_at_index_examples() {
    // Tests the documented behavior:
    //    v = [1, 2, 3]
    //    insert_at_index(5, 0) => [5, 1, 2, 3]
    //    insert_at_index(10, 2) => [5, 1, 10, 2, 3]
    //    insert_at_index(11, 5) => [5, 1, 10, 2, 3, 11]
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);

    v.insert_at_index(5, 0);
    assert_eq!(v.size(), 4);
    assert_eq!(*v.get_element_at(0).unwrap(), 5);
    assert_eq!(*v.get_element_at(1).unwrap(), 1);
    assert_eq!(*v.get_element_at(2).unwrap(), 2);
    assert_eq!(*v.get_element_at(3).unwrap(), 3);

    v.insert_at_index(10, 2);
    assert_eq!(v.size(), 5);
    assert_eq!(*v.get_element_at(0).unwrap(), 5);
    assert_eq!(*v.get_element_at(1).unwrap(), 1);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 2);
    assert_eq!(*v.get_element_at(4).unwrap(), 3);

    v.insert_at_index(11, 5);
    assert_eq!(v.size(), 6);
    assert_eq!(*v.get_element_at(0).unwrap(), 5);
    assert_eq!(*v.get_element_at(1).unwrap(), 1);
    assert_eq!(*v.get_element_at(2).unwrap(), 10);
    assert_eq!(*v.get_element_at(3).unwrap(), 2);
    assert_eq!(*v.get_element_at(4).unwrap(), 3);
    assert_eq!(*v.get_element_at(5).unwrap(), 11);
}

#[test]
fn test_remove_at_index() {
    // From [c, b, c, a, a, b], remove index 2 -> [c, b, a, a, b]
    let mut v: Vector<i32> = Vector::new(8);
    let a: i32 = 10;
    let b: i32 = 20;
    let c: i32 = 30;
    v.push_back(c);
    v.push_back(b);
    v.push_back(c);
    v.push_back(a);
    v.push_back(a);
    v.push_back(b);

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
fn test_remove_at_index_out_of_range() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    let r = v.remove_at_index(5);
    assert_eq!(r, None);
    assert_eq!(v.size(), 1);
}

#[test]
fn test_reallocate() {
    // Push enough to force reallocation. Capacity starts at 8.
    let mut v: Vector<i32> = Vector::new(8);
    let a: i32 = 10;
    let b: i32 = 20;
    let c: i32 = 30;
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
}

#[test]
fn test_top_back_top_front() {
    let mut v: Vector<i32> = Vector::new(8);
    let a: i32 = 10;
    let b: i32 = 20;
    let c: i32 = 30;
    v.push_back(c);
    v.push_back(b);
    v.push_back(a);
    v.push_back(a);
    v.push_back(b);
    for _ in 0..100 {
        v.push_back(a);
    }
    assert_eq!(*v.top_back().unwrap(), a);
    assert_eq!(*v.top_front().unwrap(), c);
    assert_eq!(v.size(), 105);
}

#[test]
fn test_top_back_top_front_empty() {
    let v: Vector<i32> = Vector::new(8);
    assert!(v.top_back().is_none());
    assert!(v.top_front().is_none());
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
    assert_eq!(*v.get_element_at(0).unwrap(), 1);
    assert_eq!(*v.get_element_at(1).unwrap(), 2);

    let popped = v.pop_back();
    assert_eq!(popped, Some(2));
    let popped = v.pop_back();
    assert_eq!(popped, Some(1));
    assert_eq!(v.size(), 0);
    let popped = v.pop_back();
    assert_eq!(popped, None);
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
    assert_eq!(*v.get_element_at(1).unwrap(), 3);

    let popped = v.pop_front();
    assert_eq!(popped, Some(2));
    let popped = v.pop_front();
    assert_eq!(popped, Some(3));
    assert_eq!(v.size(), 0);
    let popped = v.pop_front();
    assert_eq!(popped, None);
}

#[test]
fn test_index_of() {
    // v = [1, 1, 1, 2, 3]
    // index_of(3) => 4
    // index_of(1) => 0
    // index_of_with_start(1, 1) => 1
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(1);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);

    assert_eq!(v.index_of(&3), Some(4));
    assert_eq!(v.index_of(&1), Some(0));
    assert_eq!(v.index_of_with_start(&1, 1), Some(1));
    assert_eq!(v.index_of_with_start(&1, 2), Some(2));
    assert_eq!(v.index_of(&99), None);
}

#[test]
fn test_get_element_at_oob() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(7);
    assert!(v.get_element_at(5).is_none());
    assert_eq!(*v.get_element_at(0).unwrap(), 7);
}

#[test]
fn test_release() {
    let mut v: Vector<i32> = Vector::new(8);
    v.push_back(1);
    v.push_back(2);
    v.push_back(3);
    v.release();
    assert_eq!(v.size(), 0);
}

#[test]
fn test_vector_with_xml_elements() {
    // Mirror test_vector2 from C, smaller scale.
    use SimpleXML::simple_xml::XMLElement;
    let mut v: Vector<XMLElement> = Vector::new(8);
    let n = 1000;
    for i in 0..n {
        let tag = format!("tag {}", i);
        let value = format!("value {}", i);
        let e = XMLElement::new(tag, value);
        v.push_back(e);
    }
    assert_eq!(v.size(), n);
    for i in 0..n {
        let tag = format!("tag {}", i);
        let value = format!("value {}", i);
        let e = v.get_element_at(i).unwrap();
        assert_eq!(e.tag_name, tag);
        assert_eq!(e.value, value);
    }
}

fn main() {}
