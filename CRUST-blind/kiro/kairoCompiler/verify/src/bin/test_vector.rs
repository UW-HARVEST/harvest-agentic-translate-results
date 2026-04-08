use kairoCompiler::vector::*;

#[test]
fn test_vector_create_empty() {
    let v = vector_create(std::mem::size_of::<i32>());
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
    assert_eq!(vector_element_size(&v), 4);
    assert_eq!(vector_current_index(&v), 0);
}

#[test]
fn test_vector_push_and_count() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    assert_eq!(vector_count(&v), 3);
    assert!(!vector_empty(&v));
    assert_eq!(vector_current_index(&v), 3);
}

#[test]
fn test_vector_at() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 0).unwrap().try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 1).unwrap().try_into().unwrap()), 20);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 2).unwrap().try_into().unwrap()), 30);
}

#[test]
fn test_vector_back() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    let back = vector_back(&mut v).unwrap();
    assert_eq!(i32::from_le_bytes(back.try_into().unwrap()), 30);
}

#[test]
fn test_vector_back_or_null() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    assert!(vector_back_or_null(&mut v).is_none());
    vector_push(&mut v, &10i32.to_le_bytes());
    let back = vector_back_or_null(&mut v).unwrap();
    assert_eq!(i32::from_le_bytes(back.try_into().unwrap()), 10);
}

#[test]
fn test_vector_peek_sequence() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    vector_set_peek_pointer(&mut v, 0);
    assert_eq!(i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap()), 20);
    assert_eq!(i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap()), 30);
    assert!(vector_peek(&mut v).is_none());
}

#[test]
fn test_vector_peek_no_increment() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_set_peek_pointer(&mut v, 1);
    let p1 = i32::from_le_bytes(vector_peek_no_increment(&mut v).unwrap().try_into().unwrap());
    assert_eq!(p1, 20);
    let p2 = i32::from_le_bytes(vector_peek_no_increment(&mut v).unwrap().try_into().unwrap());
    assert_eq!(p2, 20);
}

#[test]
fn test_vector_peek_at() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    assert_eq!(i32::from_le_bytes(vector_peek_at(&mut v, 0).unwrap().try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(vector_peek_at(&mut v, 2).unwrap().try_into().unwrap()), 30);
    assert!(vector_peek_at(&mut v, 3).is_none());
}

#[test]
fn test_vector_pop() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push(&mut v, &30i32.to_le_bytes());
    vector_pop(&mut v);
    assert_eq!(vector_count(&v), 2);
    assert_eq!(i32::from_le_bytes(vector_back(&mut v).unwrap().try_into().unwrap()), 20);
}

#[test]
fn test_vector_push_at() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_push_at(&mut v, 0, &99i32.to_le_bytes());
    assert_eq!(vector_count(&v), 3);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 0).unwrap().try_into().unwrap()), 99);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 1).unwrap().try_into().unwrap()), 10);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 2).unwrap().try_into().unwrap()), 20);
}

#[test]
fn test_vector_pop_at() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &99i32.to_le_bytes());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_pop_at(&mut v, 0);
    assert_eq!(vector_count(&v), 2);
    assert_eq!(i32::from_le_bytes(vector_at(&mut v, 0).unwrap().try_into().unwrap()), 10);
}

#[test]
fn test_vector_save_restore() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_save(&mut v);
    vector_push(&mut v, &999i32.to_le_bytes());
    assert_eq!(vector_count(&v), 3);
    vector_restore(&mut v);
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_clone() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    let mut vc = vector_clone(&v);
    assert_eq!(vector_count(&vc), 2);
    assert_eq!(i32::from_le_bytes(vector_at(&mut vc, 0).unwrap().try_into().unwrap()), 10);
}

#[test]
fn test_vector_clear() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &10i32.to_le_bytes());
    vector_push(&mut v, &20i32.to_le_bytes());
    vector_clear(&mut v);
    assert_eq!(vector_count(&v), 0);
    assert!(vector_empty(&v));
}

#[test]
fn test_vector_set_peek_pointer_end() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &1i32.to_le_bytes());
    vector_push(&mut v, &2i32.to_le_bytes());
    vector_push(&mut v, &3i32.to_le_bytes());
    vector_set_peek_pointer_end(&mut v);
    let val = i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap());
    assert_eq!(val, 3);
}

#[test]
fn test_vector_peek_decrement_flag() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &1i32.to_le_bytes());
    vector_push(&mut v, &2i32.to_le_bytes());
    vector_push(&mut v, &3i32.to_le_bytes());
    vector_set_peek_pointer_end(&mut v);
    vector_set_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
    assert_eq!(i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap()), 3);
    assert_eq!(i32::from_le_bytes(vector_peek(&mut v).unwrap().try_into().unwrap()), 2);
    vector_unset_flag(&mut v, VECTOR_FLAG_PEEK_DECREMENT);
}

#[test]
fn test_vector_insert() {
    let mut vsrc = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut vsrc, &100i32.to_le_bytes());
    vector_push(&mut vsrc, &200i32.to_le_bytes());
    let mut vdst = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut vdst, &1i32.to_le_bytes());
    vector_push(&mut vdst, &2i32.to_le_bytes());
    let res = vector_insert(&mut vdst, &vsrc, 1);
    assert_eq!(res, 0);
    assert_eq!(vector_count(&vdst), 4);
    assert_eq!(i32::from_le_bytes(vector_at(&mut vdst, 0).unwrap().try_into().unwrap()), 1);
    assert_eq!(i32::from_le_bytes(vector_at(&mut vdst, 1).unwrap().try_into().unwrap()), 100);
    assert_eq!(i32::from_le_bytes(vector_at(&mut vdst, 2).unwrap().try_into().unwrap()), 200);
    assert_eq!(i32::from_le_bytes(vector_at(&mut vdst, 3).unwrap().try_into().unwrap()), 2);
}

#[test]
fn test_vector_insert_mismatched_esize() {
    let vsrc = vector_create(std::mem::size_of::<i32>());
    let mut vdst = vector_create(std::mem::size_of::<i64>());
    let res = vector_insert(&mut vdst, &vsrc, 0);
    assert_eq!(res, -1);
}

#[test]
fn test_vector_save_purge() {
    let mut v = vector_create(std::mem::size_of::<i32>());
    vector_push(&mut v, &1i32.to_le_bytes());
    vector_save(&mut v);
    vector_push(&mut v, &2i32.to_le_bytes());
    vector_save_purge(&mut v);
    // save_purge removes the save but doesn't restore state
    assert_eq!(vector_count(&v), 2);
}

#[test]
fn test_vector_free() {
    let v = vector_create(std::mem::size_of::<i32>());
    vector_free(v);
}

fn main() {}
