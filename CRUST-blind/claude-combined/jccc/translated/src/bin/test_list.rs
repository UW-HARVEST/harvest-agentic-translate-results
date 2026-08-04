use jccc::list::{
    create_list, destroy_list, ladd_element, lget_element, literate, lset_element, new_block,
};

#[test]
fn test_create_list_basic() {
    let l = create_list(10);
    assert_eq!(l.blocksize, 10);
    assert!(l.head.is_none());
}

#[test]
fn test_ladd_and_lget_single() {
    let mut l = create_list(10);
    let r = ladd_element(&mut l, Box::new(42i32));
    assert_eq!(r, 0);
    let got = lget_element(&mut l, 0);
    assert!(got.is_some());
    let val = got.unwrap().downcast_ref::<i32>();
    assert_eq!(val, Some(&42));
}

#[test]
fn test_ladd_many_within_blocksize() {
    let mut l = create_list(10);
    for i in 0..5 {
        let r = ladd_element(&mut l, Box::new(i as i32));
        assert_eq!(r, 0);
    }
    for i in 0..5 {
        let got = lget_element(&mut l, i);
        assert!(got.is_some());
        let v = got.unwrap().downcast_ref::<i32>();
        assert_eq!(v, Some(&(i as i32)));
    }
}

#[test]
fn test_ladd_across_blocks() {
    let mut l = create_list(10);
    for i in 0..100 {
        let r = ladd_element(&mut l, Box::new(i as i32));
        assert_eq!(r, 0);
    }
    for i in 0..100 {
        let got = lget_element(&mut l, i);
        assert!(got.is_some(), "missing at index {}", i);
        let v = got.unwrap().downcast_ref::<i32>();
        assert_eq!(v, Some(&(i as i32)));
    }
}

#[test]
fn test_lget_out_of_bounds() {
    let mut l = create_list(10);
    let r = ladd_element(&mut l, Box::new(1i32));
    assert_eq!(r, 0);
    // Negative index
    let g = lget_element(&mut l, -1);
    assert!(g.is_none());
    // Past end
    let g = lget_element(&mut l, 5);
    assert!(g.is_none());
}

#[test]
fn test_lset_element_basic() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    ladd_element(&mut l, Box::new(2i32));
    ladd_element(&mut l, Box::new(3i32));
    let r = lset_element(&mut l, 1, Box::new(99i32));
    assert_eq!(r, 0);
    let g = lget_element(&mut l, 1);
    assert_eq!(g.unwrap().downcast_ref::<i32>(), Some(&99));
}

#[test]
fn test_lset_element_out_of_bounds() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    let r = lset_element(&mut l, 5, Box::new(99i32));
    assert_eq!(r, -1);
    let r2 = lset_element(&mut l, -1, Box::new(99i32));
    assert_eq!(r2, -1);
}

#[test]
fn test_destroy_list_returns_zero() {
    let mut l = create_list(10);
    ladd_element(&mut l, Box::new(1i32));
    ladd_element(&mut l, Box::new(2i32));
    let r = destroy_list(&mut l);
    assert_eq!(r, 0);
    assert!(l.head.is_none());
}

#[test]
fn test_destroy_empty_list() {
    let mut l = create_list(10);
    let r = destroy_list(&mut l);
    assert_eq!(r, 0);
}

#[test]
fn test_new_block_basic() {
    let mut l = create_list(10);
    let b = new_block(&mut l);
    assert_eq!(b.size, 10);
    assert_eq!(b.full, 0);
    assert!(b.next.is_none());
}

#[test]
fn test_literate_sums_returns() {
    let mut l = create_list(10);
    for i in 0..5 {
        ladd_element(&mut l, Box::new(i as i32));
    }
    fn returns_one(_e: &mut Box<dyn std::any::Any>) -> i32 {
        1
    }
    let total = literate(&mut l, returns_one);
    assert_eq!(total, 5);
}

#[test]
fn test_literate_empty() {
    let mut l = create_list(10);
    fn returns_one(_e: &mut Box<dyn std::any::Any>) -> i32 {
        1
    }
    let total = literate(&mut l, returns_one);
    assert_eq!(total, 0);
}

#[test]
fn test_basic_100_test_replication() {
    // Reproduces the C basic_100_test
    let bs = 10;
    let ts = 100;
    let mut l = create_list(bs);
    for i in 0..ts {
        ladd_element(&mut l, Box::new(i as i32));
    }
    for i in 0..ts {
        let got = lget_element(&mut l, i);
        assert!(got.is_some());
        let v = got.unwrap().downcast_ref::<i32>();
        assert_eq!(v, Some(&(i as i32)));
    }
    destroy_list(&mut l);
}

fn main() {}
