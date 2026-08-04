use kairoCompiler::vector::{
    vector_create, vector_free, vector_push, vector_at, vector_back,
    vector_back_or_null, vector_count, vector_empty, vector_clear,
    vector_pop, vector_pop_at, vector_peek, vector_peek_no_increment,
    vector_set_peek_pointer, vector_set_peek_pointer_end,
    vector_peek_back, vector_peek_at, vector_set_flag, vector_unset_flag,
    vector_save, vector_restore, vector_save_purge,
    vector_element_size, vector_clone, vector_current_index,
    vector_string, vector_data_ptr, vector_insert,
    vector_push_at, vector_pop_value, vector_pop_at_data_address,
    vector_back_ptr, vector_back_ptr_or_null,
    vector_peek_ptr_at, vector_peek_ptr,
    VECTOR_ELEMENT_INCREMENT, VECTOR_FLAG_PEEK_DECREMENT,
};

#[test]
fn test_vector_create_initial() {
    let v = vector_create(4);
    assert_eq!(v.count, 0);
    assert_eq!(v.pindex, 0);
    assert_eq!(v.rindex, 0);
    assert_eq!(v.mindex, 20);
    assert_eq!(v.esize, 4);
    assert_eq!(v.flags, 0);
    assert_eq!(VECTOR_ELEMENT_INCREMENT, 20);
    assert_eq!(VECTOR_FLAG_PEEK_DECREMENT, 1);
}

#[test]
fn test_vector_push_and_count() {
    let mut v = vector_create(4);
    let bytes = 42i32.to_le_bytes();
    vector_push(&mut v, &bytes);
    assert_eq!(vector_count(&v), 1);
    assert_eq!(v.rindex, 1);

    let bytes2 = 100i32.to_le_bytes();
    vector_push(&mut v, &bytes2);
    assert_eq!(vector_count(&v), 2);
    assert_eq!(v.rindex, 2);
}

#[test]
fn test_vector_at() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    let b2 = 100i32.to_le_bytes();
    let b3 = 200i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_push(&mut v, &b3);
    let s0 = vector_at(&mut v, 0).unwrap();
    let v0 = i32::from_le_bytes([s0[0], s0[1], s0[2], s0[3]]);
    assert_eq!(v0, 42);
    let s1 = vector_at(&mut v, 1).unwrap();
    let v1 = i32::from_le_bytes([s1[0], s1[1], s1[2], s1[3]]);
    assert_eq!(v1, 100);
    let s2 = vector_at(&mut v, 2).unwrap();
    let v2 = i32::from_le_bytes([s2[0], s2[1], s2[2], s2[3]]);
    assert_eq!(v2, 200);
}

#[test]
fn test_vector_back() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    let b2 = 100i32.to_le_bytes();
    let b3 = 200i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_push(&mut v, &b3);
    let back = vector_back(&mut v).unwrap();
    let val = i32::from_le_bytes([back[0], back[1], back[2], back[3]]);
    assert_eq!(val, 200);
}

#[test]
fn test_vector_back_or_null_empty() {
    let mut v = vector_create(4);
    assert!(vector_back_or_null(&mut v).is_none());
}

#[test]
fn test_vector_back_or_null_filled() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    vector_push(&mut v, &b1);
    let back = vector_back_or_null(&mut v).unwrap();
    let val = i32::from_le_bytes([back[0], back[1], back[2], back[3]]);
    assert_eq!(val, 42);
}

#[test]
fn test_vector_empty() {
    let mut v = vector_create(4);
    assert!(vector_empty(&v));
    let b = 1i32.to_le_bytes();
    vector_push(&mut v, &b);
    assert!(!vector_empty(&v));
}

#[test]
fn test_vector_clear() {
    let mut v = vector_create(4);
    let b1 = 1i32.to_le_bytes();
    let b2 = 2i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    assert_eq!(vector_count(&v), 2);
    vector_clear(&mut v);
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
}

#[test]
fn test_vector_pop() {
    let mut v = vector_create(4);
    let b1 = 1i32.to_le_bytes();
    let b2 = 2i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_pop(&mut v);
    assert_eq!(vector_count(&v), 1);
    assert_eq!(v.rindex, 1);
}

#[test]
fn test_vector_pop_at_first() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    let b2 = 100i32.to_le_bytes();
    let b3 = 200i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_push(&mut v, &b3);
    vector_pop_at(&mut v, 0);
    assert_eq!(vector_count(&v), 2);
    let s0 = vector_at(&mut v, 0).unwrap();
    let v0 = i32::from_le_bytes([s0[0], s0[1], s0[2], s0[3]]);
    assert_eq!(v0, 100);
    let s1 = vector_at(&mut v, 1).unwrap();
    let v1 = i32::from_le_bytes([s1[0], s1[1], s1[2], s1[3]]);
    assert_eq!(v1, 200);
}

#[test]
fn test_vector_peek_no_increment() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    let b2 = 100i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    let s = vector_peek_no_increment(&mut v).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 42);
    assert_eq!(v.pindex, 0);
}

#[test]
fn test_vector_peek_increments() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    let b2 = 100i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    let s = vector_peek(&mut v).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 42);
    assert_eq!(v.pindex, 1);
}

#[test]
fn test_vector_set_peek_pointer() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_set_peek_pointer(&mut v, 0);
    assert_eq!(v.pindex, 0);
    vector_set_peek_pointer(&mut v, 5);
    assert_eq!(v.pindex, 5);
}

#[test]
fn test_vector_set_peek_pointer_end() {
    let mut v = vector_create(4);
    let b1 = 1i32.to_le_bytes();
    let b2 = 2i32.to_le_bytes();
    let b3 = 3i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_push(&mut v, &b3);
    vector_set_peek_pointer_end(&mut v);
    assert_eq!(v.pindex, 2);
}

#[test]
fn test_vector_peek_back() {
    let mut v = vector_create(4);
    let b1 = 1i32.to_le_bytes();
    let b2 = 2i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_set_peek_pointer(&mut v, 1);
    vector_peek_back(&mut v);
    assert_eq!(v.pindex, 0);
}

#[test]
fn test_vector_peek_at() {
    let mut v = vector_create(4);
    let b1 = 5i32.to_le_bytes();
    let b2 = 6i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    let s = vector_peek_at(&mut v, 1).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 6);
    // peek at index out of bounds returns None
    assert!(vector_peek_at(&mut v, 5).is_none());
}

#[test]
fn test_vector_set_unset_flag() {
    let mut v = vector_create(4);
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    assert_eq!(v.flags, VECTOR_FLAG_PEEK_DECREMENT);
    vector_unset_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    assert_eq!(v.flags, 0);
}

#[test]
fn test_vector_save_restore() {
    let mut v = vector_create(4);
    let b1 = 1i32.to_le_bytes();
    let b2 = 2i32.to_le_bytes();
    vector_push(&mut v, &b1);
    vector_push(&mut v, &b2);
    vector_save(&mut v);
    let b3 = 3i32.to_le_bytes();
    vector_push(&mut v, &b3);
    assert_eq!(vector_count(&v), 3);
    vector_restore(&mut v);
    assert_eq!(vector_count(&v), 2);
    assert_eq!(v.rindex, 2);
}

#[test]
fn test_vector_save_purge() {
    let mut v = vector_create(4);
    vector_save(&mut v);
    assert_eq!(v.saves.len(), 1);
    vector_save_purge(&mut v);
    assert_eq!(v.saves.len(), 0);
}

#[test]
fn test_vector_element_size() {
    let v = vector_create(8);
    assert_eq!(vector_element_size(&v), 8);
}

#[test]
fn test_vector_clone() {
    let mut v = vector_create(4);
    let b1 = 42i32.to_le_bytes();
    vector_push(&mut v, &b1);
    let cloned = vector_clone(&v);
    assert_eq!(cloned.count, v.count);
    assert_eq!(cloned.rindex, v.rindex);
    assert_eq!(cloned.esize, v.esize);
    // Saves are not cloned
    assert_eq!(cloned.saves.len(), 0);
}

#[test]
fn test_vector_current_index() {
    let mut v = vector_create(4);
    assert_eq!(vector_current_index(&v), 0);
    let b1 = 1i32.to_le_bytes();
    vector_push(&mut v, &b1);
    let b2 = 2i32.to_le_bytes();
    vector_push(&mut v, &b2);
    assert_eq!(vector_current_index(&v), 2);
}

#[test]
fn test_vector_count_basic() {
    let mut v = vector_create(4);
    assert_eq!(vector_count(&v), 0);
    let b1 = 1i32.to_le_bytes();
    vector_push(&mut v, &b1);
    assert_eq!(vector_count(&v), 1);
}

#[test]
fn test_vector_string_with_null_term() {
    let mut v = vector_create(1);
    vector_push(&mut v, &[b'h']);
    vector_push(&mut v, &[b'i']);
    vector_push(&mut v, &[0u8]);
    let s = vector_string(&v);
    assert_eq!(s, Some("hi"));
}

#[test]
fn test_vector_data_ptr() {
    let mut v = vector_create(1);
    vector_push(&mut v, &[b'a']);
    let p = vector_data_ptr(&v);
    assert_eq!(p[0], b'a');
}

#[test]
fn test_vector_insert_basic() {
    let mut a = vector_create(4);
    vector_push(&mut a, &10i32.to_le_bytes());
    vector_push(&mut a, &20i32.to_le_bytes());
    vector_push(&mut a, &30i32.to_le_bytes());

    let mut b = vector_create(4);
    vector_push(&mut b, &100i32.to_le_bytes());
    vector_push(&mut b, &200i32.to_le_bytes());

    let result = vector_insert(&mut a, &b, 1);
    assert_eq!(result, 0);
    assert_eq!(vector_count(&a), 5);
    let s0 = vector_at(&mut a, 0).unwrap();
    assert_eq!(i32::from_le_bytes([s0[0], s0[1], s0[2], s0[3]]), 10);
    let s1 = vector_at(&mut a, 1).unwrap();
    assert_eq!(i32::from_le_bytes([s1[0], s1[1], s1[2], s1[3]]), 100);
    let s2 = vector_at(&mut a, 2).unwrap();
    assert_eq!(i32::from_le_bytes([s2[0], s2[1], s2[2], s2[3]]), 200);
    let s3 = vector_at(&mut a, 3).unwrap();
    assert_eq!(i32::from_le_bytes([s3[0], s3[1], s3[2], s3[3]]), 20);
    let s4 = vector_at(&mut a, 4).unwrap();
    assert_eq!(i32::from_le_bytes([s4[0], s4[1], s4[2], s4[3]]), 30);
}

#[test]
fn test_vector_insert_size_mismatch() {
    let mut a = vector_create(4);
    let b = vector_create(8);
    let result = vector_insert(&mut a, &b, 0);
    assert_eq!(result, -1);
}

#[test]
fn test_vector_push_at() {
    let mut v = vector_create(4);
    vector_push(&mut v, &1i32.to_le_bytes());
    vector_push(&mut v, &2i32.to_le_bytes());
    vector_push(&mut v, &3i32.to_le_bytes());
    vector_push_at(&mut v, 1, &99i32.to_le_bytes());
    assert_eq!(vector_count(&v), 4);
    let s0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(i32::from_le_bytes([s0[0], s0[1], s0[2], s0[3]]), 1);
    let s1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(i32::from_le_bytes([s1[0], s1[1], s1[2], s1[3]]), 99);
    let s2 = vector_at(&mut v, 2).unwrap();
    assert_eq!(i32::from_le_bytes([s2[0], s2[1], s2[2], s2[3]]), 2);
    let s3 = vector_at(&mut v, 3).unwrap();
    assert_eq!(i32::from_le_bytes([s3[0], s3[1], s3[2], s3[3]]), 3);
}

#[test]
fn test_vector_pop_value_not_found() {
    let mut v = vector_create(4);
    vector_push(&mut v, &1i32.to_le_bytes());
    let bad = 99i32.to_le_bytes();
    let r = vector_pop_value(&mut v, &bad);
    assert_eq!(r, -1);
    assert_eq!(vector_count(&v), 1);
}

#[test]
fn test_vector_pop_value_found() {
    let mut v = vector_create(4);
    vector_push(&mut v, &1i32.to_le_bytes());
    vector_push(&mut v, &2i32.to_le_bytes());
    vector_push(&mut v, &3i32.to_le_bytes());
    let target = 2i32.to_le_bytes();
    let r = vector_pop_value(&mut v, &target);
    assert_eq!(r, 1);
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_pop_at_data_address() {
    let mut v = vector_create(4);
    vector_push(&mut v, &1i32.to_le_bytes());
    let bytes = [0u8; 4];
    let addr = bytes.as_ptr();
    let r = vector_pop_at_data_address(&mut v, addr);
    // The Rust implementation returns -1 always (cannot safely implement)
    assert_eq!(r, -1);
}

#[test]
fn test_vector_back_ptr_filled() {
    let mut v = vector_create(4);
    vector_push(&mut v, &7i32.to_le_bytes());
    vector_push(&mut v, &13i32.to_le_bytes());
    let s = vector_back_ptr(&mut v).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 13);
}

#[test]
fn test_vector_back_ptr_or_null_empty() {
    let mut v = vector_create(4);
    assert!(vector_back_ptr_or_null(&mut v).is_none());
}

#[test]
fn test_vector_peek_ptr_at_in_bounds() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    let s = vector_peek_ptr_at(&mut v, 1).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 20);
}

#[test]
fn test_vector_peek_ptr_at_out_of_bounds() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_le_bytes());
    let r = vector_peek_ptr_at(&mut v, -1);
    assert!(r.is_none());
}

#[test]
fn test_vector_peek_ptr() {
    let mut v = vector_create(4);
    vector_push(&mut v, &111i32.to_le_bytes());
    let s = vector_peek_ptr(&mut v).unwrap();
    let val = i32::from_le_bytes([s[0], s[1], s[2], s[3]]);
    assert_eq!(val, 111);
    assert_eq!(v.pindex, 1);
}

#[test]
fn test_vector_peek_decrement_flag() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_set_peek_pointer(&mut v, 1);
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    let _ = vector_peek(&mut v);
    assert_eq!(v.pindex, 0);
}

#[test]
fn test_vector_free() {
    let v = vector_create(4);
    vector_free(v);
}

fn main() {}
