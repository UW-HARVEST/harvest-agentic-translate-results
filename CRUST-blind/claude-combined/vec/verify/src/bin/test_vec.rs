#![allow(unused_imports)]
use vec::vec::{
    vec_compact, vec_expand, vec_insert, vec_reserve, vec_reserve_po2, vec_splice, vec_swap,
    vec_swapsplice, VEC_VERSION,
};

// ---- vec_insert ----
#[test]
fn test_vec_insert_basic() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_insert(&mut v, 0, 42);
    assert_eq!(r, 0);
    assert_eq!(v.len(), 1);
    assert_eq!(v[0], 42);
}

#[test]
fn test_vec_insert_repeated_at_zero() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000i32 {
        let r = vec_insert(&mut v, 0, i);
        assert_eq!(r, 0);
    }
    assert_eq!(v.len(), 1000);
    assert_eq!(v[0], 999);
    assert_eq!(v[v.len() - 1], 0);
}

#[test]
fn test_vec_insert_middle_and_end() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000i32 {
        vec_insert(&mut v, 0, i);
    }
    // Now v[0]=999, v[999]=0
    let r1 = vec_insert(&mut v, 10, 123);
    assert_eq!(r1, 0);
    assert_eq!(v[10], 123);
    assert_eq!(v.len(), 1001);

    let len = v.len();
    let r2 = vec_insert(&mut v, len - 2, 678);
    assert_eq!(r2, 0);
    assert_eq!(v[999], 678);

    let r3 = vec_insert(&mut v, 10, 123);
    assert_eq!(r3, 0);

    let len = v.len();
    let r4 = vec_insert(&mut v, len, 789);
    assert_eq!(r4, 0);
    let len = v.len();
    assert_eq!(v[len - 1], 789);
}

#[test]
fn test_vec_insert_at_length_appends() {
    let mut v: Vec<i32> = vec![1, 2, 3];
    let r = vec_insert(&mut v, 3, 99);
    assert_eq!(r, 0);
    assert_eq!(v, vec![1, 2, 3, 99]);
}

#[test]
fn test_vec_insert_out_of_bounds_returns_neg1() {
    let mut v: Vec<i32> = vec![1, 2, 3];
    let r = vec_insert(&mut v, 5, 99);
    assert_eq!(r, -1);
    // v is unchanged
    assert_eq!(v, vec![1, 2, 3]);
}

// ---- vec_splice ----
#[test]
fn test_vec_splice_from_start() {
    let mut v: Vec<i32> = (0..1000i32).collect();
    vec_splice(&mut v, 0, 10);
    assert_eq!(v.len(), 990);
    assert_eq!(v[0], 10);
}

#[test]
fn test_vec_splice_middle() {
    let mut v: Vec<i32> = (0..1000i32).collect();
    vec_splice(&mut v, 0, 10);
    vec_splice(&mut v, 10, 10);
    assert_eq!(v.len(), 980);
    assert_eq!(v[10], 30);
}

#[test]
fn test_vec_splice_tail() {
    let mut v: Vec<i32> = (0..1000i32).collect();
    vec_splice(&mut v, 0, 10);
    vec_splice(&mut v, 10, 10);
    let len = v.len();
    vec_splice(&mut v, len - 50, 50);
    let len = v.len();
    assert_eq!(v[len - 1], 949);
    assert_eq!(len, 930);
}

#[test]
fn test_vec_splice_count_zero_noop() {
    let mut v: Vec<i32> = vec![1, 2, 3, 4, 5];
    vec_splice(&mut v, 1, 0);
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
}

// ---- vec_swapsplice ----
#[test]
fn test_vec_swapsplice_from_start() {
    let mut v: Vec<i32> = (0..10i32).collect();
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
fn test_vec_swapsplice_from_end() {
    let mut v: Vec<i32> = (0..10i32).collect();
    vec_swapsplice(&mut v, 0, 3);
    // v = [7,8,9,3,4,5,6]
    let len = v.len();
    vec_swapsplice(&mut v, len - 1, 1);
    let len = v.len();
    assert_eq!(len, 6);
    assert_eq!(v[len - 1], 5);
    assert_eq!(v, vec![7, 8, 9, 3, 4, 5]);
}

#[test]
fn test_vec_swapsplice_overlap() {
    // start=2, count=5, len=10 → overlap region
    let mut v: Vec<i32> = (0..10i32).collect();
    vec_swapsplice(&mut v, 2, 5);
    assert_eq!(v.len(), 5);
    assert_eq!(v, vec![0, 1, 5, 6, 7]);
}

#[test]
fn test_vec_swapsplice_zero_count_noop() {
    let mut v: Vec<i32> = vec![1, 2, 3, 4, 5];
    vec_swapsplice(&mut v, 1, 0);
    assert_eq!(v, vec![1, 2, 3, 4, 5]);
}

// ---- vec_reserve ----
#[test]
fn test_vec_reserve_grows_capacity() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve(&mut v, 100);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 100);
    assert_eq!(v.len(), 0);
}

#[test]
fn test_vec_reserve_smaller_no_op() {
    let mut v: Vec<i32> = Vec::new();
    vec_reserve(&mut v, 100);
    let r = vec_reserve(&mut v, 50);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 100);
}

#[test]
fn test_vec_reserve_with_data() {
    let mut v: Vec<i32> = Vec::new();
    v.push(123);
    v.push(456);
    let r = vec_reserve(&mut v, 200);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 200);
    assert_eq!(v.len(), 2);
    assert_eq!(v[0], 123);
    assert_eq!(v[1], 456);

    let r2 = vec_reserve(&mut v, 300);
    assert_eq!(r2, 0);
    assert_eq!(v.capacity(), 300);
}

// ---- vec_reserve_po2 ----
#[test]
fn test_vec_reserve_po2_zero_returns_zero() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 0);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 0);
}

#[test]
fn test_vec_reserve_po2_one() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 1);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 1);
}

#[test]
fn test_vec_reserve_po2_rounds_up() {
    let mut v: Vec<i32> = Vec::new();
    let r = vec_reserve_po2(&mut v, 5);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 8);

    let mut v2: Vec<i32> = Vec::new();
    vec_reserve_po2(&mut v2, 100);
    assert_eq!(v2.capacity(), 128);

    let mut v3: Vec<i32> = Vec::new();
    vec_reserve_po2(&mut v3, 16);
    assert_eq!(v3.capacity(), 16);
}

// ---- vec_expand ----
#[test]
fn test_vec_expand_empty_vec() {
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(v.capacity(), 0);
    let r = vec_expand(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 1);
}

#[test]
fn test_vec_expand_doubles_when_full() {
    let mut v: Vec<i32> = Vec::with_capacity(4);
    for i in 0..4i32 {
        v.push(i);
    }
    assert_eq!(v.capacity(), 4);
    let r = vec_expand(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 8);
    assert_eq!(v.len(), 4);
}

#[test]
fn test_vec_expand_no_op_when_room_left() {
    let mut v: Vec<i32> = Vec::with_capacity(10);
    v.push(1);
    let r = vec_expand(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.capacity(), 10);
}

// ---- vec_compact ----
#[test]
fn test_vec_compact_after_truncate() {
    let mut v: Vec<i32> = Vec::with_capacity(1000);
    for i in 0..1000i32 {
        v.push(i);
    }
    v.truncate(3);
    let r = vec_compact(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.len(), v.capacity());
    assert_eq!(v.len(), 3);
}

#[test]
fn test_vec_compact_repeated_call() {
    let mut v: Vec<i32> = vec![1, 2, 3];
    let r1 = vec_compact(&mut v);
    assert_eq!(r1, 0);
    let r2 = vec_compact(&mut v);
    assert_eq!(r2, 0);
}

#[test]
fn test_vec_compact_empty() {
    let mut v: Vec<i32> = Vec::with_capacity(100);
    let r = vec_compact(&mut v);
    assert_eq!(r, 0);
    assert_eq!(v.len(), 0);
    assert_eq!(v.capacity(), 0);
}

// ---- vec_swap ----
#[test]
fn test_vec_swap_basic() {
    let mut v: Vec<u8> = vec![b'a', b'b', b'c'];
    vec_swap(&mut v, 0, 2);
    assert_eq!(v[0], b'c');
    assert_eq!(v[1], b'b');
    assert_eq!(v[2], b'a');

    vec_swap(&mut v, 0, 1);
    assert_eq!(v[0], b'b');
    assert_eq!(v[1], b'c');
    assert_eq!(v[2], b'a');

    vec_swap(&mut v, 1, 2);
    assert_eq!(v[0], b'b');
    assert_eq!(v[1], b'a');
    assert_eq!(v[2], b'c');
}

#[test]
fn test_vec_swap_same_index_noop() {
    let mut v: Vec<u8> = vec![b'a', b'b', b'c'];
    vec_swap(&mut v, 1, 1);
    assert_eq!(v[0], b'a');
    assert_eq!(v[1], b'b');
    assert_eq!(v[2], b'c');
}

// ---- VEC_VERSION constant ----
#[test]
fn test_vec_version_constant() {
    assert_eq!(VEC_VERSION, "0.2.1");
}

fn main() {}
