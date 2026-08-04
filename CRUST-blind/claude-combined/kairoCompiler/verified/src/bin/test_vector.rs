use kairoCompiler::vector::{
    vector_at, vector_back, vector_back_or_null, vector_clear, vector_clone, vector_count,
    vector_create, vector_current_index, vector_data_ptr, vector_element_size, vector_empty,
    vector_free, vector_peek, vector_peek_at, vector_peek_no_increment, vector_peek_pop,
    vector_peek_ptr_at, vector_pop, vector_pop_at, vector_pop_value, vector_push, vector_push_at,
    vector_restore, vector_save, vector_set_flag, vector_set_peek_pointer,
    vector_set_peek_pointer_end, vector_unset_flag, VECTOR_ELEMENT_INCREMENT,
    VECTOR_FLAG_PEEK_DECREMENT,
};

#[test]
fn test_vector_create_initial_state() {
    let v = vector_create(4);
    assert_eq!(v.count, 0);
    assert_eq!(v.rindex, 0);
    assert_eq!(v.pindex, 0);
    assert_eq!(v.mindex, VECTOR_ELEMENT_INCREMENT as i32);
    assert_eq!(v.esize, 4);
    assert!(vector_empty(&v));
    assert_eq!(vector_count(&v), 0);
    assert_eq!(vector_element_size(&v), 4);
}

fn push_i32(v: &mut kairoCompiler::vector::Vector, x: i32) {
    let bytes = x.to_le_bytes();
    vector_push(v, &bytes);
}

fn read_i32(slice: &[u8]) -> i32 {
    let mut a = [0u8; 4];
    a.copy_from_slice(&slice[..4]);
    i32::from_le_bytes(a)
}

#[test]
fn test_vector_push_and_at() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    assert_eq!(vector_count(&v), 3);
    assert!(!vector_empty(&v));

    let s0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(read_i32(s0), 10);
    let s1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(read_i32(s1), 20);
    let s2 = vector_at(&mut v, 2).unwrap();
    assert_eq!(read_i32(s2), 30);
}

#[test]
fn test_vector_back_and_pop() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);

    let b = vector_back(&mut v).unwrap();
    assert_eq!(read_i32(b), 30);

    vector_pop(&mut v);
    assert_eq!(vector_count(&v), 2);
    let b2 = vector_back(&mut v).unwrap();
    assert_eq!(read_i32(b2), 20);
}

#[test]
fn test_vector_peek_increment() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_set_peek_pointer(&mut v, 0);

    let p1 = vector_peek(&mut v).unwrap();
    assert_eq!(read_i32(p1), 10);
    let p2 = vector_peek(&mut v).unwrap();
    assert_eq!(read_i32(p2), 20);
    let p3 = vector_peek(&mut v).unwrap();
    assert_eq!(read_i32(p3), 30);
    let p4 = vector_peek(&mut v);
    assert!(p4.is_none());
}

#[test]
fn test_vector_peek_decrement() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_set_peek_pointer(&mut v, 1);
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);

    let p1 = vector_peek(&mut v).unwrap();
    assert_eq!(read_i32(p1), 20);
    let p2 = vector_peek(&mut v).unwrap();
    assert_eq!(read_i32(p2), 10);
    vector_unset_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
}

#[test]
fn test_vector_peek_no_increment() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    vector_set_peek_pointer(&mut v, 0);
    let p = vector_peek_no_increment(&mut v).unwrap();
    assert_eq!(read_i32(p), 10);
    let p2 = vector_peek_no_increment(&mut v).unwrap();
    assert_eq!(read_i32(p2), 10);
}

#[test]
fn test_vector_clear() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_clear(&mut v);
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
}

#[test]
fn test_vector_pop_at() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_pop_at(&mut v, 1);
    assert_eq!(vector_count(&v), 2);
    let s0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(read_i32(s0), 10);
    let s1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(read_i32(s1), 30);
}

#[test]
fn test_vector_push_at() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 30);
    let bytes = 99i32.to_le_bytes();
    vector_push_at(&mut v, 0, &bytes);
    assert_eq!(vector_count(&v), 3);
    let s0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(read_i32(s0), 99);
    let s1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(read_i32(s1), 10);
    let s2 = vector_at(&mut v, 2).unwrap();
    assert_eq!(read_i32(s2), 30);
}

#[test]
fn test_vector_save_restore() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_save(&mut v);
    vector_clear(&mut v);
    assert_eq!(vector_count(&v), 0);
    vector_restore(&mut v);
    assert_eq!(vector_count(&v), 3);
}

#[test]
fn test_vector_current_index() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    assert_eq!(vector_current_index(&v), 3);
}

#[test]
fn test_vector_back_or_null_empty() {
    let mut v = vector_create(4);
    let p = vector_back_or_null(&mut v);
    assert!(p.is_none());
}

#[test]
fn test_vector_set_peek_pointer_end() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_set_peek_pointer_end(&mut v);
    assert_eq!(v.pindex, 2);
}

#[test]
fn test_vector_clone() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    let mut c = vector_clone(&v);
    assert_eq!(vector_count(&c), 2);
    let s = vector_at(&mut c, 0).unwrap();
    assert_eq!(read_i32(s), 10);
}

#[test]
fn test_vector_peek_ptr_at() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    let s = vector_peek_ptr_at(&mut v, 0).unwrap();
    assert_eq!(read_i32(s), 10);
}

#[test]
fn test_vector_peek_at() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    let s = vector_peek_at(&mut v, 1).unwrap();
    assert_eq!(read_i32(s), 20);
    let none = vector_peek_at(&mut v, 5);
    assert!(none.is_none());
}

#[test]
fn test_vector_data_ptr() {
    let v = vector_create(4);
    let p = vector_data_ptr(&v);
    assert_eq!(p.len(), 4 * VECTOR_ELEMENT_INCREMENT);
}

#[test]
fn test_vector_pop_value() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    let bytes = 20i32.to_le_bytes();
    vector_pop_value(&mut v, &bytes);
    assert_eq!(vector_count(&v), 2);
    let s0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(read_i32(s0), 10);
    let s1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(read_i32(s1), 30);
}

#[test]
fn test_vector_peek_pop() {
    let mut v = vector_create(4);
    push_i32(&mut v, 10);
    push_i32(&mut v, 20);
    push_i32(&mut v, 30);
    vector_set_peek_pointer(&mut v, 1);
    vector_peek_pop(&mut v);
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_free_no_panic() {
    let v = vector_create(4);
    vector_free(v);
}

fn main() {}
