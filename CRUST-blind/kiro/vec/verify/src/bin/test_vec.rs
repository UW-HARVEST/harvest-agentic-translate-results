use vec::vec::{
    vec_compact, vec_expand, vec_insert, vec_reserve, vec_reserve_po2, vec_splice, vec_swap,
    vec_swapsplice, VEC_VERSION,
};

#[test]
fn test_vec_version() {
    assert_eq!(VEC_VERSION, "0.2.1");
}

#[test]
fn test_vec_insert_empty() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_insert(&mut v, 0, 42);
    assert_eq!(r, 0);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0], 42);
}

#[test]
fn test_vec_insert_front_1000() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        vec_insert(&mut v, 0, i);
    }
    assert_eq!(v[0], 999);
    assert_eq!(v[v.len() - 1], 0);
    vec_insert(&mut v, 10, 123);
    assert_eq!(v[10], 123);
    assert_eq!(v.len(), 1001);
}

#[test]
fn test_vec_insert_middle_and_end() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        vec_insert(&mut v, 0, i);
    }
    vec_insert(&mut v, 10, 123);
    let idx = v.len() - 2;
    vec_insert(&mut v, idx, 678);
    assert_eq!(v[999], 678);
    let r = vec_insert(&mut v, 10, 123);
    assert_eq!(r, 0);
    let end = v.len();
    vec_insert(&mut v, end, 789);
    assert_eq!(v[v.len() - 1], 789);
}

#[test]
fn test_vec_splice_front() {
    let mut v: Vec<i32> = (0..1000).collect();
    vec_splice(&mut v, 0, 10);
    assert_eq!(v[0], 10);
    assert_eq!(v.len(), 990);
}

#[test]
fn test_vec_splice_middle() {
    let mut v: Vec<i32> = (0..10).map(|i| i * 10).collect();
    vec_splice(&mut v, 2, 3);
    assert_eq!(v.len(), 7);
    assert_eq!(v[0], 0);
    assert_eq!(v[1], 10);
    assert_eq!(v[2], 50);
    assert_eq!(v[3], 60);
    assert_eq!(v[4], 70);
    assert_eq!(v[5], 80);
    assert_eq!(v[6], 90);
}

#[test]
fn test_vec_splice_chain() {
    let mut v: Vec<i32> = (0..1000).collect();
    vec_splice(&mut v, 0, 10);
    vec_splice(&mut v, 10, 10);
    assert_eq!(v[10], 30);
    let idx = v.len() - 50;
    vec_splice(&mut v, idx, 50);
    assert_eq!(v[v.len() - 1], 949);
}

#[test]
fn test_vec_swapsplice_front() {
    let mut v: Vec<i32> = (0..10).collect();
    vec_swapsplice(&mut v, 0, 3);
    assert_eq!(v.len(), 7);
    assert_eq!(v[0], 7);
    assert_eq!(v[1], 8);
    assert_eq!(v[2], 9);
    assert_eq!(v[3], 3);
    assert_eq!(v[4], 4);
    assert_eq!(v[5], 5);
    assert_eq!(v[6], 6);
}

#[test]
fn test_vec_swapsplice_last() {
    let mut v: Vec<i32> = (0..10).collect();
    vec_swapsplice(&mut v, 0, 3);
    let idx = v.len() - 1;
    vec_swapsplice(&mut v, idx, 1);
    assert_eq!(v[v.len() - 1], 5);
}

#[test]
fn test_vec_reserve_po2_zero() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 0);
    assert_eq!(r, 0);
}

#[test]
fn test_vec_reserve_po2_five() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 5);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 8);
}

#[test]
fn test_vec_reserve_po2_exact_power() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 8);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 8);
}

#[test]
fn test_vec_reserve_po2_one() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 1);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 1);
}

#[test]
fn test_vec_expand() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_expand(&mut v);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 1);
}

#[test]
fn test_vec_reserve() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve(&mut v, 10);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 10);
}

#[test]
fn test_vec_reserve_no_shrink() {
    let mut v: Vec<i32> = Vec::new();
    vec_reserve(&mut v, 10);
    let cap = v.capacity();
    vec_reserve(&mut v, 5);
    assert!(v.capacity() >= cap);
}

#[test]
fn test_vec_reserve_with_data() {
    let mut v: Vec<i32> = Vec::new();
    v.push(123);
    v.push(456);
    let r = vec_reserve(&mut v, 200);
    assert_eq!(r, 0);
    assert!(v.capacity() >= 200);
    assert_eq!(v[0], 123);
    assert_eq!(v[1], 456);
}

#[test]
fn test_vec_compact_empty() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_compact(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn test_vec_compact_with_data() {
    let mut v: Vec<i32> = (0..100).collect();
    v.truncate(5);
    let r = vec_compact(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.len(), 5);
    assert_eq!(v.len(), v.capacity());
}

#[test]
fn test_vec_compact_return() {
    let mut v: Vec<i32> = (0..100).collect();
    v.truncate(3);
    vec_compact(&mut v);
    let r = vec_compact(&mut v);
    assert_eq!(r, 0);
}

#[test]
fn test_vec_swap_different() {
    let mut v = vec![10, 20, 30];
    vec_swap(&mut v, 0, 2);
    assert_eq!(v[0], 30);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 10);
}

#[test]
fn test_vec_swap_same() {
    let mut v = vec![10, 20, 30];
    vec_swap(&mut v, 1, 1);
    assert_eq!(v[0], 10);
    assert_eq!(v[1], 20);
    assert_eq!(v[2], 30);
}

#[test]
fn test_vec_swap_chars() {
    let mut v = vec!['a' as i32, 'b' as i32, 'c' as i32];
    vec_swap(&mut v, 0, 2);
    assert_eq!(v[0], 'c' as i32);
    assert_eq!(v[2], 'a' as i32);
    vec_swap(&mut v, 0, 1);
    assert_eq!(v[0], 'b' as i32);
    assert_eq!(v[1], 'c' as i32);
    vec_swap(&mut v, 1, 2);
    assert_eq!(v[1], 'a' as i32);
    assert_eq!(v[2], 'c' as i32);
}

fn main() {}
