use jccc::list::*;

#[test]
fn test_create_list() {
    let l = create_list(10);
    assert_eq!(l.blocksize, 10);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
}

#[test]
fn test_add_and_get_element() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(42i32));
    let elem = lget_element(&mut l, 0).unwrap();
    assert_eq!(*elem.downcast_ref::<i32>().unwrap(), 42);
}

#[test]
fn test_add_multiple_elements() {
    let mut l = create_list(4);
    for i in 0..10 {
        ladd_element(&mut l, Box::new(i as i32));
    }
    for i in 0..10 {
        let elem = lget_element(&mut l, i).unwrap();
        assert_eq!(*elem.downcast_ref::<i32>().unwrap(), i as i32);
    }
}

#[test]
fn test_set_element() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    ladd_element(&mut l, Box::new(2i32));
    lset_element(&mut l, 0, Box::new(99i32));
    let elem = lget_element(&mut l, 0).unwrap();
    assert_eq!(*elem.downcast_ref::<i32>().unwrap(), 99);
}

#[test]
fn test_destroy_list() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    assert_eq!(destroy_list(&mut l), 0);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
}

#[test]
fn test_get_out_of_bounds() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    assert!(lget_element(&mut l, 5).is_none());
}

#[test]
fn test_literate() {
    let mut l = create_list(4);
    for _ in 0..5 {
        ladd_element(&mut l, Box::new(1i32));
    }
    let sum = literate(&mut l, |_| 1);
    assert_eq!(sum, 5);
}

fn main() {}
