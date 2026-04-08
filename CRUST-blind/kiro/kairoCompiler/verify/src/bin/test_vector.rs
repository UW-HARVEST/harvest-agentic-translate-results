use kairoCompiler::vector::*;

#[test]
fn test_vector_empty_on_create() {
    let v = vector_create(4);
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
}

#[test]
fn test_vector_push_and_count() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_push(&mut v, &200i32.to_ne_bytes());
    assert_eq!(vector_count(&v), 3);
    assert!(!vector_empty(&v));
}

#[test]
fn test_vector_at() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_push(&mut v, &200i32.to_ne_bytes());

    let b0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(i32::from_ne_bytes(b0.try_into().unwrap()), 42);
    let b1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(i32::from_ne_bytes(b1.try_into().unwrap()), 100);
    let b2 = vector_at(&mut v, 2).unwrap();
    assert_eq!(i32::from_ne_bytes(b2.try_into().unwrap()), 200);
}

#[test]
fn test_vector_back() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_push(&mut v, &200i32.to_ne_bytes());
    let b = vector_back(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 200);
}

#[test]
fn test_vector_pop() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_push(&mut v, &200i32.to_ne_bytes());
    vector_pop(&mut v);
    assert_eq!(vector_count(&v), 2);
    let b = vector_back(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 100);
}

#[test]
fn test_vector_peek_increments() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_set_peek_pointer(&mut v, 0);

    let b0 = vector_peek(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b0.try_into().unwrap()), 42);
    let b1 = vector_peek(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b1.try_into().unwrap()), 100);
}

#[test]
fn test_vector_peek_no_increment() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_set_peek_pointer(&mut v, 0);

    let b0 = vector_peek_no_increment(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b0.try_into().unwrap()), 42);
    let b1 = vector_peek_no_increment(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b1.try_into().unwrap()), 42);
}

#[test]
fn test_vector_current_index() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    assert_eq!(vector_current_index(&v), 2);
}

#[test]
fn test_vector_element_size() {
    let v = vector_create(4);
    assert_eq!(vector_element_size(&v), 4);
}

#[test]
fn test_vector_save_restore() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_save(&mut v);
    vector_push(&mut v, &999i32.to_ne_bytes());
    assert_eq!(vector_count(&v), 3);
    vector_restore(&mut v);
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_clone() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    let mut v2 = vector_clone(&v);
    assert_eq!(vector_count(&v2), 2);
    let b = vector_at(&mut v2, 0).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 42);
}

#[test]
fn test_vector_clear() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    vector_clear(&mut v);
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
}

#[test]
fn test_vector_back_or_null_empty() {
    let mut v = vector_create(4);
    assert!(vector_back_or_null(&mut v).is_none());
}

#[test]
fn test_vector_push_at() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_ne_bytes());
    vector_push(&mut v, &30i32.to_ne_bytes());
    vector_push_at(&mut v, 1, &20i32.to_ne_bytes());
    assert_eq!(vector_count(&v), 3);
    let b0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(i32::from_ne_bytes(b0.try_into().unwrap()), 10);
    let b1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(i32::from_ne_bytes(b1.try_into().unwrap()), 20);
    let b2 = vector_at(&mut v, 2).unwrap();
    assert_eq!(i32::from_ne_bytes(b2.try_into().unwrap()), 30);
}

#[test]
fn test_vector_pop_at() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_ne_bytes());
    vector_push(&mut v, &20i32.to_ne_bytes());
    vector_push(&mut v, &30i32.to_ne_bytes());
    vector_pop_at(&mut v, 1);
    assert_eq!(vector_count(&v), 2);
    let b0 = vector_at(&mut v, 0).unwrap();
    assert_eq!(i32::from_ne_bytes(b0.try_into().unwrap()), 10);
    let b1 = vector_at(&mut v, 1).unwrap();
    assert_eq!(i32::from_ne_bytes(b1.try_into().unwrap()), 30);
}

#[test]
fn test_vector_peek_back() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_ne_bytes());
    vector_push(&mut v, &20i32.to_ne_bytes());
    vector_set_peek_pointer(&mut v, 1);
    vector_peek_back(&mut v);
    assert_eq!(v.pindex, 0);
}

#[test]
fn test_vector_set_peek_pointer_end() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_ne_bytes());
    vector_push(&mut v, &20i32.to_ne_bytes());
    vector_set_peek_pointer_end(&mut v);
    // rindex=2, so pindex = 2-1 = 1
    assert_eq!(v.pindex, 1);
}

#[test]
fn test_vector_flags() {
    let mut v = vector_create(4);
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    assert_eq!(v.flags, 1);
    vector_unset_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    assert_eq!(v.flags, 0);
}

#[test]
fn test_vector_save_purge() {
    let mut v = vector_create(4);
    vector_push(&mut v, &1i32.to_ne_bytes());
    vector_save(&mut v);
    vector_push(&mut v, &2i32.to_ne_bytes());
    vector_save_purge(&mut v);
    // save_purge just pops the save, doesn't restore state
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_peek_at() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_push(&mut v, &100i32.to_ne_bytes());
    let b = vector_peek_at(&mut v, 0).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 42);
    // Out of bounds returns None
    assert!(vector_peek_at(&mut v, 5).is_none());
}

#[test]
fn test_vector_peek_returns_none_past_end() {
    let mut v = vector_create(4);
    vector_push(&mut v, &42i32.to_ne_bytes());
    vector_set_peek_pointer(&mut v, 0);
    let _ = vector_peek(&mut v); // consumes index 0
    assert!(vector_peek(&mut v).is_none()); // past end
}

#[test]
fn test_vector_peek_decrement_flag() {
    let mut v = vector_create(4);
    vector_push(&mut v, &10i32.to_ne_bytes());
    vector_push(&mut v, &20i32.to_ne_bytes());
    vector_push(&mut v, &30i32.to_ne_bytes());
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    vector_set_peek_pointer_end(&mut v);
    // pindex = rindex-1 = 2
    let b = vector_peek(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 30);
    // After peek with decrement, pindex should be 1
    let b = vector_peek(&mut v).unwrap();
    assert_eq!(i32::from_ne_bytes(b.try_into().unwrap()), 20);
}

fn main() {}
