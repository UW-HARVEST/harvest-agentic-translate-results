use dict::dict::{
    dict_get, dict_has, dict_key, dict_len, dict_new, dict_remove, DictType,
};

/// Tests for string-keyed dictionaries — the most common use case.

fn write_i32(slot: &mut [u8], v: i32) {
    slot[..4].copy_from_slice(&v.to_le_bytes());
}

fn read_i32(slot: &[u8]) -> i32 {
    let mut buf = [0u8; 4];
    buf.copy_from_slice(&slot[..4]);
    i32::from_le_bytes(buf)
}

#[test]
fn test_str_dict_insert_and_lookup() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"Hello").unwrap(), 1);
    write_i32(dict_get(&mut d, b"World").unwrap(), 2);
    write_i32(dict_get(&mut d, b"Foo").unwrap(), 3);

    assert_eq!(dict_len(&d), 3);
    assert_eq!(read_i32(dict_get(&mut d, b"Hello").unwrap()), 1);
    assert_eq!(read_i32(dict_get(&mut d, b"World").unwrap()), 2);
    assert_eq!(read_i32(dict_get(&mut d, b"Foo").unwrap()), 3);
}

#[test]
fn test_str_dict_has_and_remove() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"Hello").unwrap(), 100);
    write_i32(dict_get(&mut d, b"World").unwrap(), 200);

    assert!(dict_has(&d, b"Hello"));
    assert!(dict_has(&d, b"World"));
    assert!(!dict_has(&d, b"Goodbye"));

    let removed = dict_remove(&mut d, b"Hello");
    assert!(removed);
    assert!(!dict_has(&d, b"Hello"));
    assert!(dict_has(&d, b"World"));
    assert_eq!(dict_len(&d), 1);

    let removed = dict_remove(&mut d, b"NoSuchKey");
    assert!(!removed);
}

#[test]
fn test_str_dict_overwrite_via_get() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"key").unwrap(), 10);
    assert_eq!(read_i32(dict_get(&mut d, b"key").unwrap()), 10);
    write_i32(dict_get(&mut d, b"key").unwrap(), 999);
    assert_eq!(read_i32(dict_get(&mut d, b"key").unwrap()), 999);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_str_dict_keys_dump() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"alpha").unwrap(), 1);
    write_i32(dict_get(&mut d, b"beta").unwrap(), 2);
    write_i32(dict_get(&mut d, b"gamma").unwrap(), 3);

    let mut size = 0usize;
    let keys = dict_key(&d, &mut size);
    assert_eq!(size, 3);
    assert!(keys.is_some());
    let buf = keys.unwrap();
    assert!(!buf.is_empty());
    // The buffer should contain the concatenation of all key bytes; check
    // that each known key is present somewhere in the dump (in some order).
    let s = std::str::from_utf8(buf).unwrap();
    assert!(s.contains("alpha"));
    assert!(s.contains("beta"));
    assert!(s.contains("gamma"));
}

#[test]
fn test_str_dict_empty_key_lookup() {
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"").unwrap(), 42);
    assert!(dict_has(&d, b""));
    assert_eq!(read_i32(dict_get(&mut d, b"").unwrap()), 42);
    assert_eq!(dict_len(&d), 1);
}

#[test]
fn test_str_dict_keys_with_same_hash_bucket() {
    // "Bar" and "Baz" both fall into bucket 2 with default mod 8 (verified via C).
    let mut d = dict_new(DictType::Str, 0, 4);
    write_i32(dict_get(&mut d, b"Bar").unwrap(), 1);
    write_i32(dict_get(&mut d, b"Baz").unwrap(), 2);
    assert_eq!(dict_len(&d), 2);
    assert_eq!(read_i32(dict_get(&mut d, b"Bar").unwrap()), 1);
    assert_eq!(read_i32(dict_get(&mut d, b"Baz").unwrap()), 2);

    // remove one and the other should still be retrievable.
    assert!(dict_remove(&mut d, b"Bar"));
    assert!(!dict_has(&d, b"Bar"));
    assert!(dict_has(&d, b"Baz"));
    assert_eq!(read_i32(dict_get(&mut d, b"Baz").unwrap()), 2);
}

#[test]
fn test_str_dict_many_entries() {
    let mut d = dict_new(DictType::Str, 0, 4);
    for i in 0..50 {
        let s = format!("k{}", i);
        write_i32(dict_get(&mut d, s.as_bytes()).unwrap(), i);
    }
    assert_eq!(dict_len(&d), 50);
    for i in 0..50 {
        let s = format!("k{}", i);
        assert_eq!(read_i32(dict_get(&mut d, s.as_bytes()).unwrap()), i);
    }
}

fn main() {}
