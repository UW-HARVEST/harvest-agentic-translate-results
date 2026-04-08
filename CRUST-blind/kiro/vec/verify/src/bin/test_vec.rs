use vec::vec::{
    vec_compact, vec_expand, vec_insert, vec_reserve, vec_reserve_po2, vec_splice, vec_swap,
    vec_swapsplice, VEC_VERSION,
};

#[test]
fn test_vec_version() {
    assert_eq!(VEC_VERSION, "0.2.1");
}

// --- vec_insert ---

#[test]
fn test_vec_insert_at_front() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        assert_eq!(vec_insert(&mut v, 0, i), 0);
    }
    assert_eq!(v[0], 999);
    assert_eq!(v[v.len() - 1], 0);
}

#[test]
fn test_vec_insert_mid_and_end() {
    let mut v: Vec<i32> = Vec::new();
    for i in 0..1000 {
        vec_insert(&mut v, 0, i);
    }
    vec_insert(&mut v, 10, 123);
    assert_eq!(v[10], 123);
    assert_eq!(v.len(), 1001);
    let idx = v.len() - 2;
    vec_insert(&mut v, idx, 678);
    assert_eq!(v[999], 678);
    assert_eq!(vec_insert(&mut v, 10, 123), 0);
    let len = v.len();
    vec_insert(&mut v, len, 789);
    assert_eq!(v[v.len() - 1], 789);
}

// --- vec_splice ---

#[test]
fn test_vec_splice() {
    let mut v: Vec<i32> = (0..1000).collect();
    vec_splice(&mut v, 0, 10);
    assert_eq!(v[0], 10);
    vec_splice(&mut v, 10, 10);
    assert_eq!(v[10], 30);
    let tail = v.len() - 50;
    vec_splice(&mut v, tail, 50);
    assert_eq!(v[v.len() - 1], 949);
}

// --- vec_swapsplice ---

#[test]
fn test_vec_swapsplice() {
    let mut v: Vec<i32> = (0..10).collect();
    vec_swapsplice(&mut v, 0, 3);
    assert_eq!((v[0], v[1], v[2]), (7, 8, 9));
    let last = v.len() - 1;
    vec_swapsplice(&mut v, last, 1);
    assert_eq!(v[v.len() - 1], 5);
}

// --- vec_swap ---

#[test]
fn test_vec_swap() {
    let mut v = vec!['a' as i32, 'b' as i32, 'c' as i32];
    vec_swap(&mut v, 0, 2);
    assert_eq!((v[0], v[2]), ('c' as i32, 'a' as i32));
    vec_swap(&mut v, 0, 1);
    assert_eq!((v[0], v[1]), ('b' as i32, 'c' as i32));
    vec_swap(&mut v, 1, 2);
    assert_eq!((v[1], v[2]), ('a' as i32, 'c' as i32));
    vec_swap(&mut v, 1, 1);
    assert_eq!(v[1], 'a' as i32);
}

// --- vec_reserve ---

#[test]
fn test_vec_reserve() {
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(vec_reserve(&mut v, 100), 0);
    assert!(v.capacity() >= 100);
    let cap = v.capacity();
    assert_eq!(vec_reserve(&mut v, 50), 0);
    assert_eq!(v.capacity(), cap); // should not shrink
    let mut v2: Vec<i32> = Vec::new();
    v2.push(123);
    v2.push(456);
    assert_eq!(vec_reserve(&mut v2, 200), 0);
    assert!(v2.capacity() >= 200);
    assert_eq!(vec_reserve(&mut v2, 300), 0);
}

// --- vec_reserve_po2 ---

#[test]
fn test_vec_reserve_po2_zero() {
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(vec_reserve_po2(&mut v, 0), 0);
}

#[test]
fn test_vec_reserve_po2_rounds_up() {
    let mut v: Vec<i32> = Vec::new();
    vec_reserve_po2(&mut v, 5);
    assert!(v.capacity() >= 8); // next power of 2 >= 5
    let mut v2: Vec<i32> = Vec::new();
    vec_reserve_po2(&mut v2, 8);
    assert!(v2.capacity() >= 8);
}

// --- vec_expand ---

#[test]
fn test_vec_expand() {
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(vec_expand(&mut v), 0);
    assert!(v.capacity() >= 1);
}

// --- vec_compact ---

#[test]
fn test_vec_compact() {
    let mut v: Vec<i32> = (0..1000).collect();
    v.truncate(3);
    assert_eq!(vec_compact(&mut v), 0);
    assert_eq!(v.len(), 3);
    // After compact, capacity should equal length (or close to it)
    assert!(v.capacity() <= v.len() + 1);
    assert_eq!(vec_compact(&mut v), 0);
}

#[test]
fn test_vec_compact_empty() {
    let mut v: Vec<i32> = Vec::new();
    assert_eq!(vec_compact(&mut v), 0);
    assert_eq!(v.capacity(), 0);
}

fn main() {}
