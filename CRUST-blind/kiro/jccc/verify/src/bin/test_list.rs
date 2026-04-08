use jccc::list::{create_list, destroy_list, ladd_element, lget_element, lset_element, literate};

#[test]
fn test_create_list() {
    let l = create_list(10);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
    assert_eq!(l.blocksize, 10);
}

#[test]
fn test_add_and_get_element() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(42i32));
    ladd_element(&mut l, Box::new(99i32));
    let e0 = lget_element(&mut l, 0).unwrap();
    assert_eq!(*e0.downcast_ref::<i32>().unwrap(), 42);
    let e1 = lget_element(&mut l, 1).unwrap();
    assert_eq!(*e1.downcast_ref::<i32>().unwrap(), 99);
}

#[test]
fn test_set_element() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    let ret = lset_element(&mut l, 0, Box::new(999i32));
    assert_eq!(ret, 0);
    let e = lget_element(&mut l, 0).unwrap();
    assert_eq!(*e.downcast_ref::<i32>().unwrap(), 999);
}

#[test]
fn test_add_across_blocks() {
    // blocksize=4, add 10 elements to force multiple blocks
    let mut l = create_list(4);
    for i in 0..10 {
        ladd_element(&mut l, Box::new(i as i32));
    }
    for i in 0..10 {
        let e = lget_element(&mut l, i).unwrap();
        assert_eq!(*e.downcast_ref::<i32>().unwrap(), i as i32);
    }
}

#[test]
fn test_destroy_list() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    let ret = destroy_list(&mut l);
    assert_eq!(ret, 0);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
}

#[test]
fn test_literate() {
    let mut l = create_list(5);
    for i in 0..5 {
        ladd_element(&mut l, Box::new(i as i32));
    }
    // Sum function: returns the value
    fn sum_fn(e: &mut Box<dyn std::any::Any>) -> i32 {
        *e.downcast_ref::<i32>().unwrap()
    }
    let total = literate(&mut l, sum_fn);
    assert_eq!(total, 0 + 1 + 2 + 3 + 4);
}

#[test]
fn test_get_out_of_bounds_returns_none() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    assert!(lget_element(&mut l, 1).is_none());
}

fn main() {}
