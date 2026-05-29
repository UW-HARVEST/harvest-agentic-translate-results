use jccc::list::{
    create_list, destroy_list, ladd_element, lget_element, literate, lset_element, new_block,
};
use std::any::Any;

#[test]
fn test_create_list_initial_state() {
    let l = create_list(10);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
    assert_eq!(l.blocksize, 10);
}

#[test]
fn test_ladd_and_lget_basic() {
    let mut l = create_list(10);
    for i in 0..5_i32 {
        let r = ladd_element(&mut l, Box::new(i));
        assert_eq!(r, 0);
    }
    for i in 0..5_i32 {
        let elem = lget_element(&mut l, i).expect("expected element");
        let v = elem.downcast_ref::<i32>().expect("expected i32");
        assert_eq!(*v, i);
    }
    destroy_list(&mut l);
}

#[test]
fn test_ladd_across_multiple_blocks() {
    let mut l = create_list(10);
    let total: i32 = 25;
    for i in 0..total {
        ladd_element(&mut l, Box::new(i));
    }
    for i in 0..total {
        let elem = lget_element(&mut l, i).expect("expected element");
        let v = elem.downcast_ref::<i32>().expect("expected i32");
        assert_eq!(*v, i);
    }
    destroy_list(&mut l);
}

#[test]
fn test_lget_out_of_bounds_returns_none() {
    let mut l = create_list(10);
    for i in 0..5_i32 {
        ladd_element(&mut l, Box::new(i));
    }
    // index past the filled portion
    assert!(lget_element(&mut l, 5).is_none());
    assert!(lget_element(&mut l, 100).is_none());
    destroy_list(&mut l);
}

#[test]
fn test_lget_negative_returns_none() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(42_i32));
    assert!(lget_element(&mut l, -1).is_none());
    destroy_list(&mut l);
}

#[test]
fn test_lget_empty_list() {
    let mut l = create_list(10);
    assert!(lget_element(&mut l, 0).is_none());
    destroy_list(&mut l);
}

#[test]
fn test_lset_element_basic() {
    let mut l = create_list(10);
    for i in 0..5_i32 {
        ladd_element(&mut l, Box::new(i));
    }
    let r = lset_element(&mut l, 2, Box::new(999_i32));
    assert_eq!(r, 0);
    let elem = lget_element(&mut l, 2).unwrap();
    let v = elem.downcast_ref::<i32>().unwrap();
    assert_eq!(*v, 999);
    destroy_list(&mut l);
}

#[test]
fn test_lset_element_out_of_bounds() {
    let mut l = create_list(10);
    for i in 0..5_i32 {
        ladd_element(&mut l, Box::new(i));
    }
    // setting an index past `full` should return -1
    let r = lset_element(&mut l, 5, Box::new(123_i32));
    assert_eq!(r, -1);
    let r2 = lset_element(&mut l, 100, Box::new(123_i32));
    assert_eq!(r2, -1);
    destroy_list(&mut l);
}

#[test]
fn test_lset_negative_index() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1_i32));
    let r = lset_element(&mut l, -5, Box::new(10_i32));
    assert_eq!(r, -1);
    destroy_list(&mut l);
}

fn sum_i32(elem: &mut Box<dyn Any>) -> i32 {
    *elem.downcast_ref::<i32>().unwrap()
}

#[test]
fn test_literate_sums_elements() {
    let mut l = create_list(10);
    let total: i32 = 25;
    for i in 0..total {
        ladd_element(&mut l, Box::new(i));
    }
    // Sum should be 0+1+...+24 = 300, matching the C reference output.
    assert_eq!(literate(&mut l, sum_i32), 300);
    destroy_list(&mut l);
}

#[test]
fn test_literate_empty_list() {
    let mut l = create_list(10);
    assert_eq!(literate(&mut l, sum_i32), 0);
    destroy_list(&mut l);
}

#[test]
fn test_destroy_list_clears_state() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1_i32));
    ladd_element(&mut l, Box::new(2_i32));
    let r = destroy_list(&mut l);
    assert_eq!(r, 0);
    assert!(l.head.is_none());
    assert!(l.tail.is_none());
}

#[test]
fn test_new_block_initial_state() {
    let mut l = create_list(7);
    let block = new_block(&mut l);
    assert_eq!(block.size, 7);
    assert_eq!(block.full, 0);
    assert!(block.next.is_none());
    assert_eq!(block.array.len(), 0);
}

fn main() {}
