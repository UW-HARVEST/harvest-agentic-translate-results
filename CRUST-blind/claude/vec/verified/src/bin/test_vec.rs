#![allow(unused_imports)]

use vec::vec::{
    vec_compact, vec_expand, vec_insert, vec_reserve, vec_reserve_po2, vec_splice, vec_swap,
    vec_swapsplice, VEC_VERSION,
};

#[test]
fn test_vec_version_constant() {
    assert_eq!(VEC_VERSION, "0.2.1");
}

#[test]
fn test_vec_expand_from_empty() {
    // C: vec_expand_ on a fresh vec sets capacity = 1
    let mut v: Vec<i32> = Vec::new();
    let err = vec_expand(&mut v);
    assert_eq!(err, 0);
    assert_eq!(v.len(), 0);
    assert!(v.capacity() >= 1);
}

#[test]
fn test_vec_expand_doubles_capacity() {
    // C: when length+1 > capacity, capacity doubles
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(vec_expand(&mut v), 0);
    let cap1 = v.capacity();
    assert!(cap1 >= 1);
    v.push(42); // length = 1, capacity should be 1 now
    let err = vec_expand(&mut v);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 2);
    v.push(43); // length = 2
    let err = vec_expand(&mut v);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 4);
}

#[test]
fn test_vec_expand_no_growth_when_room_available() {
    // C: when length+1 <= capacity, capacity is unchanged
    let mut v: Vec<i32> = Vec::with_capacity(10);
    let cap_before = v.capacity();
    let err = vec_expand(&mut v);
    assert_eq!(err, 0);
    assert_eq!(v.capacity(), cap_before);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_grows_capacity() {
    // C test: vec_reserve on empty grows to n
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve(&mut v, 100);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 100);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_smaller_than_current_no_change() {
    // C test: vec_reserve with smaller n keeps cap == 100
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve(&mut v, 100);
    assert_eq!(err, 0);
    let cap_after_100 = v.capacity();
    assert!(cap_after_100 >= 100);
    let err = vec_reserve(&mut v, 50);
    assert_eq!(err, 0);
    assert_eq!(v.capacity(), cap_after_100);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_with_existing_data() {
    // C test: push 123, 456, then reserve 200 -> cap == 200
    let mut v: Vec<i32> = Vec::new();
    v.push(123);
    v.push(456);
    let err = vec_reserve(&mut v, 200);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 200);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 123);
    assert_eq!(v[1], 456);

    let err = vec_reserve(&mut v, 300);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 300);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 123);
    assert_eq!(v[1], 456);
}

#[test]
fn test_vec_reserve_po2_zero() {
    // C: vec_reserve_po2_(_, 0) returns 0 without changes
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve_po2(&mut v, 0);
    assert_eq!(err, 0);
    assert_eq!(v.capacity(), 0);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_po2_rounds_up() {
    // C: vec_reserve_po2_(_, 5) -> rounds to 8
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve_po2(&mut v, 5);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 8);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_po2_already_po2() {
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve_po2(&mut v, 1);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 1);
}

#[test]
fn test_vec_reserve_po2_100_rounds_to_128() {
    let mut v: Vec<i32> = Vec::new();
    let err = vec_reserve_po2(&mut v, 100);
    assert_eq!(err, 0);
    assert!(v.capacity() >= 128);
}

#[test]
fn test_vec_reserve_po2_smaller_no_change() {
    // After reserving 100 (rounded to 128), reserving 7 (rounded to 8) shouldn't shrink
    let mut v: Vec<i32> = Vec::new();
    vec_reserve_po2(&mut v, 100);
    let cap_before = v.capacity();
    let err = vec_reserve_po2(&mut v, 7);
    assert_eq!(err, 0);
    assert_eq!(v.capacity(), cap_before);
}

#[test]
fn test_vec_compact_empty() {
    // C: vec_compact_ on empty vec sets data=NULL, capacity=0
    let mut v: Vec<i32> = Vec::new();
    vec_reserve(&mut v, 100);
    assert!(v.capacity() >= 100);
    let err = vec_compact(&mut v);
    assert_eq!(err, 0);
    assert_eq!(v.len(), 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn test_vec_compact_after_truncate() {
    // C test: push 1000, truncate 3, compact -> length == capacity
    let mut v: Vec<i32> = Vec::with_capacity(1000);
    for _ in 0..1000 {
        v.push(0);
    }
    v.truncate(3);
    let err = vec_compact(&mut v);
    assert_eq!(err, 0);
    assert_eq!(v.len(), 3);
    assert_eq!(v.capacity(), 3);
}

#[test]
fn test_vec_compact_idempotent() {
    let mut v: Vec<i32> = Vec::new();
    for _ in 0..1000 {
        v.push(0);
    }
    v.truncate(3);
    assert_eq!(vec_compact(&mut v), 0);
    assert_eq!(vec_compact(&mut v), 0);
    assert_eq!(v.len(), 3);
    assert_eq!(v.capacity(), 3);
}

#[test]
fn test_vec_insert_at_front() {
    // C test: for i in 0..1000, vec_insert(&v, 0, i)
    // After this, v.data[0] == 999, v.data[length-1] == 0
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        let r = vec_insert(&mut v, 0, i);
        assert_eq!(r, 0);
    }
    assert_eq!(v[0], 999);
    assert_eq!(v[v.len() - 1], 0);
    assert_eq!(v.len(), 1000);
}

#[test]
fn test_vec_insert_in_middle() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        vec_insert(&mut v, 0, i);
    }
    let r = vec_insert(&mut v, 10, 123);
    assert_eq!(r, 0);
    assert_eq!(v[10], 123);
    assert_eq!(v.len(), 1001);
}

#[test]
fn test_vec_insert_at_end() {
    // C: vec_insert(&v, v.length, val) appends
    let mut v: Vec<i32> = Vec::new();
    v.push(1);
    v.push(2);
    let r = vec_insert(&mut v, 2, 99);
    assert_eq!(r, 0);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 1);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 99);
}

#[test]
fn test_vec_insert_near_end() {
    // C test: vec_insert(&v, v.length - 2, 678) when v[999] == 1 in inserted-from-front list
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        vec_insert(&mut v, 0, i);
    }
    // v.length is 1000 now; insert at 999 (length - 1) puts 678 in slot 999
    vec_insert(&mut v, 10, 123);
    // Now length 1001
    let pos = v.len() - 2;
    let r = vec_insert(&mut v, pos, 678);
    assert_eq!(r, 0);
    assert_eq!(v[999], 678);
}

#[test]
fn test_vec_splice_front() {
    // C test: vec_splice 0, 10 from a 0..1000 array gives v[0] == 10
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        v.push(i);
    }
    vec_splice(&mut v, 0, 10);
    assert_eq!(v.len(), 990);
    assert_eq!(v[0], 10);
}

#[test]
fn test_vec_splice_middle() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        v.push(i);
    }
    vec_splice(&mut v, 0, 10);
    vec_splice(&mut v, 10, 10);
    // After first splice, v[10] = 20, after splicing 10..20, v[10] becomes the next element 30
    assert_eq!(v[10], 30);
}

#[test]
fn test_vec_splice_end() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        v.push(i);
    }
    vec_splice(&mut v, 0, 10);
    vec_splice(&mut v, 10, 10);
    let len_before = v.len();
    vec_splice(&mut v, len_before - 50, 50);
    assert_eq!(v[v.len() - 1], 949);
}

#[test]
fn test_vec_splice_count_zero_noop() {
    let mut v: Vec<i32> = Vec::new();
    v.push(1);
    v.push(2);
    v.push(3);
    vec_splice(&mut v, 1, 0);
    assert_eq!(v.len(), 3);
    assert_eq!(v[0], 1);
    assert_eq!(v[1], 2);
    assert_eq!(v[2], 3);
}

#[test]
fn test_vec_swapsplice_front() {
    // C test: 0..10, swapsplice 0,3 -> [7,8,9,3,4,5,6]
    let mut v: Vec<i32> = Vec::new();
    for i in 0..10 {
        v.push(i);
    }
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
fn test_vec_swapsplice_at_end() {
    // After swapsplice(0,3) from 0..10, vec is [7,8,9,3,4,5,6]
    // Then swapsplice(len-1, 1) removes the last element -> [7,8,9,3,4,5]
    // C test asserts v.data[v.length - 1] == 5
    let mut v: Vec<i32> = Vec::new();
    for i in 0..10 {
        v.push(i);
    }
    vec_swapsplice(&mut v, 0, 3);
    let l = v.len();
    vec_swapsplice(&mut v, l - 1, 1);
    assert_eq!(v.len(), 6);
    assert_eq!(v[v.len() - 1], 5);
}

#[test]
fn test_vec_swapsplice_count_zero_noop() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..5 {
        v.push(i);
    }
    vec_swapsplice(&mut v, 1, 0);
    assert_eq!(v.len(), 5);
    for i in 0..5 {
        assert_eq!(v[i as usize], i);
    }
}

#[test]
fn test_vec_swap_basic() {
    // C test:
    //   push 'a','b','c'
    //   swap 0,2 -> [c,b,a]
    //   swap 0,1 -> [b,c,a]
    //   swap 1,2 -> [b,a,c]
    //   swap 1,1 -> [b,a,c]
    let mut v: Vec<char> = Vec::new();
    v.push('a');
    v.push('b');
    v.push('c');
    vec_swap(&mut v, 0, 2);
    assert_eq!(v[0], 'c');
    assert_eq!(v[1], 'b');
    assert_eq!(v[2], 'a');
    vec_swap(&mut v, 0, 1);
    assert_eq!(v[0], 'b');
    assert_eq!(v[1], 'c');
    assert_eq!(v[2], 'a');
    vec_swap(&mut v, 1, 2);
    assert_eq!(v[0], 'b');
    assert_eq!(v[1], 'a');
    assert_eq!(v[2], 'c');
    vec_swap(&mut v, 1, 1);
    assert_eq!(v[0], 'b');
    assert_eq!(v[1], 'a');
    assert_eq!(v[2], 'c');
}

#[test]
fn test_vec_swap_with_ints() {
    let mut v: Vec<i32> = vec![10, 20, 30, 40, 50];
    vec_swap(&mut v, 0, 4);
    assert_eq!(v, vec![50, 20, 30, 40, 10]);
    vec_swap(&mut v, 1, 3);
    assert_eq!(v, vec![50, 40, 30, 20, 10]);
}

#[test]
fn test_vec_swap_same_index_no_change() {
    let mut v: Vec<i32> = vec![1, 2, 3];
    vec_swap(&mut v, 1, 1);
    assert_eq!(v, vec![1, 2, 3]);
}

fn main() {}
